//! Wada v2: the same Mobius-warped Newton plane, but every cell rasterizes as
//! a braille bitmap. Eight subcell samples per cell resolve coastlines at 2x4
//! dot precision instead of one glyph.
use crate::_0_profile::measure_layer;
use crate::color::{darken, hsl_to_rgb, lighten};
use crate::opts::param_f32;
use crate::registry::{AnimKind, Mode, ModeFrame, Param};
use crate::types::{Cell, Grid};
use rayon::prelude::*;
use std::cell::RefCell;
use std::f32::consts::TAU;

pub(super) struct WadaV2;
pub(super) static MODE: WadaV2 = WadaV2;

const NAME: &str = "wada-v2";
const KNOBS: usize = 11;
const HELP: &str = "wada-v2: braille-dot newton basins, eight subcell pixels per cell [order] [relax] [twist] [warp] [spin] [trap] [vein] [dot] [hue] [grain] [aspect]";

const VEIN_CH: char = '#';
const SPARK: [char; 3] = ['·', '+', '✦'];
const DUST: [char; 3] = ['.', '\'', '`'];
const BRAILLE_BASE: u32 = 0x2800;

/// Subcell index (row-major over 2x4) to braille dot bit.
const DOT_BIT: [u8; 8] = [0x01, 0x08, 0x02, 0x10, 0x04, 0x20, 0x40, 0x80];

const MAXIT: u8 = 24;
const F_TOL2: f32 = 1.0e-4;
const ESC2: f32 = 4096.0;
const STEP_CAP: f32 = 1.8;
const SEAM_MAXIT: u8 = MAXIT / 4;
const CHAOS: u8 = u8::MAX;
const VIEW: f32 = 1.75;
const NUDGE: f32 = 1.0e-4;

const L_BASE: u64 = 0x81;
const L_WARP: u64 = 0x82;
const L_DUST: u64 = 0x83;
const L_TRAP: u64 = 0x84;
const L_MOB: u64 = 0x85;

/// Rayon kicks in on subcell work, so a cell row is 8 solves wide.
const PARALLEL_MIN_CELLS: usize = 20_480 / 8;

const PARAMS: &[Param] = &[
    param!("ORDER", "roots on the ring", 3.0, 12.0, 5.0, 1.0),
    param!("RELAX", "newton step size", 0.4, 1.3, 1.0, 0.05),
    param!("TWIST", "mobius twist", 0.0, 0.9, 0.45, 0.05),
    param!("WARP", "swirl warp", 0.0, 1.5, 0.5, 0.05),
    param!("SPIN", "plane rotation rad/s", -1.0, 1.0, 0.15, 0.02),
    param!("TRAP", "orbit trap density", 0.0, 1.5, 0.7, 0.05),
    param!("VEIN", "boundary vein glow", 0.0, 1.5, 0.8, 0.05),
    param!("CONTOUR", "dot fill threshold", 0.0, 12.0, 6.0, 1.0),
    param!("HUE", "hue step per basin", 0.0, 90.0, 34.0, 1.0),
    param!("GRAIN", "dither grain", 0.0, 1.0, 0.35, 0.05),
    param!("ASPECT", "cols per row", 0.25, 4.0, 2.0, 0.25),
];

impl Mode for WadaV2 {
    fn name(&self) -> &'static str {
        NAME
    }
    fn help(&self) -> &'static str {
        HELP
    }
    fn animation(&self) -> AnimKind {
        AnimKind::Iterate
    }
    fn params(&self) -> &'static [Param] {
        PARAMS
    }
    fn render(&self, frame: &mut ModeFrame<'_>) {
        let p: [f32; KNOBS] = std::array::from_fn(|i| {
            let param = &PARAMS[i];
            let value = frame
                .args
                .get(i + 4)
                .and_then(|v| v.parse::<f32>().ok())
                .or_else(|| frame.param_values.and_then(|v| v.get(i)).copied())
                .unwrap_or_else(|| param_f32(param.key, param.default));
            if value.is_finite() {
                value.clamp(param.min, param.max)
            } else {
                param.default
            }
        });
        draw(frame, &p);
    }
}

/// Splitmix64 over (seed, layer, index, slot); no rng stream is consumed.
#[inline]
fn hash(seed: u64, layer: u64, index: u64, slot: u64) -> u64 {
    let mut z = seed
        ^ layer.wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ index.wrapping_mul(0xD1B5_4A32_D192_ED03)
        ^ slot.wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    z ^= z >> 30;
    z = z.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z ^= z >> 27;
    z = z.wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

#[inline]
fn unit(h: u64) -> f32 {
    (h >> 40) as f32 * (1.0 / 16_777_216.0)
}

#[inline]
fn cmul(a: (f32, f32), b: (f32, f32)) -> (f32, f32) {
    (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0)
}

#[inline]
fn cdiv(a: (f32, f32), b: (f32, f32)) -> (f32, f32) {
    let d = b.0 * b.0 + b.1 * b.1;
    ((a.0 * b.0 + a.1 * b.1) / d, (a.1 * b.0 - a.0 * b.1) / d)
}

/// One of the eight subcell samples: which root it fell into and how long.
#[derive(Clone, Copy, Default)]
struct Sub {
    basin: u8,
    iters: u8,
}

/// One cell: eight braille-dot samples plus its cheapest trap dive.
#[derive(Clone, Copy, Default)]
struct CellField {
    subs: [Sub; 8],
    trap: f32,
}

/// The basin that owns most of the cell's dots; ties keep the low index.
#[inline]
fn majority(f: &CellField) -> u8 {
    let mut counts = [0u8; 14];
    for s in &f.subs {
        counts[if s.basin == CHAOS { 13 } else { s.basin as usize }] += 1;
    }
    let mut best = 0usize;
    let mut bn = 0u8;
    for (i, &c) in counts.iter().enumerate() {
        if c > bn {
            bn = c;
            best = i;
        }
    }
    if best == 13 {
        CHAOS
    } else {
        best as u8
    }
}

/// Mean dwell of the cell in iteration units, for seam gates and contours.
#[inline]
fn avg_iters(f: &CellField) -> u16 {
    let sum: u16 = f.subs.iter().map(|s| s.iters as u16).sum();
    sum / 8
}

/// One frame's resolved geometry, roots and palette.
struct Look {
    seed: u64,
    cx: f32,
    cy: f32,
    radius: f32,
    aspect: f32,
    order: usize,
    order_f: f32,
    relax: f32,
    mob: (f32, f32),
    cst: (f32, f32),
    roots: [(f32, f32); 13],
    warp: f32,
    spin: f32,
    trap_k: f32,
    vein: f32,
    dot_t: f32,
    hue_step: f32,
    base_hue: f32,
    grain: f32,
    swirl_p: f32,
    time: f32,
}

impl Look {
    fn new(seed: u64, w: usize, h: usize, time: f32, p: &[f32; KNOBS]) -> Self {
        let aspect = p[10].max(0.25);
        let order = p[0].round().clamp(3.0, 12.0) as usize;
        let cx = w as f32 * 0.5 + (unit(hash(seed, L_BASE, 0, 3)) - 0.5) * w as f32 * 0.12;
        let cy = h as f32 * 0.5 + (unit(hash(seed, L_BASE, 1, 3)) - 0.5) * h as f32 * 0.12;
        let radius = (h as f32 * 0.5).min((w as f32 * 0.5) / aspect).max(2.0);
        let base_hue = unit(hash(seed, L_BASE, 2, 3)) * 360.0;
        let mag = 0.85 + 0.3 * unit(hash(seed, L_BASE, 3, 3));
        let theta = unit(hash(seed, L_BASE, 4, 3)) * TAU + 0.3 * time;
        let (sa, ca) = theta.sin_cos();
        let cst = (mag * ca, mag * sa);
        let mroot = mag.powf(1.0 / order as f32);
        let mut roots = [(0.0f32, 0.0f32); 13];
        for k in 0..order {
            let ak = (theta + TAU * k as f32) / order as f32;
            let (sk, ck) = ak.sin_cos();
            roots[k] = (mroot * ck, mroot * sk);
        }
        let twist = p[2] * 0.6;
        let phase = unit(hash(seed, L_MOB, 0, 5)) * TAU + 0.6 * time;
        let (sm, cm) = phase.sin_cos();
        Look {
            seed,
            cx,
            cy,
            radius,
            aspect,
            order,
            order_f: order as f32,
            relax: p[1],
            mob: (twist * cm, twist * sm),
            cst,
            roots,
            warp: p[3],
            spin: p[4],
            trap_k: p[5],
            vein: p[6],
            dot_t: 0.25 + 0.05 * p[7],
            hue_step: p[8],
            base_hue,
            grain: p[9],
            swirl_p: unit(hash(seed, L_WARP, 0, 7)) * TAU,
            time,
        }
    }
}

thread_local! {
    static SCRATCH: RefCell<Vec<CellField>> = const { RefCell::new(Vec::new()) };
}

#[inline(always)]
fn each_row<T: Send, F>(buf: &mut [T], w: usize, h: usize, row: F)
where
    F: Fn((usize, &mut [T])) + Sync + Send,
{
    if w == 0 {
        return;
    }
    if w * h >= PARALLEL_MIN_CELLS {
        buf.par_chunks_mut(w).enumerate().with_min_len(8).for_each(row);
    } else {
        buf.chunks_mut(w).enumerate().for_each(row);
    }
}

#[inline(always)]
fn grid_rows<F>(grid: &mut Grid, w: usize, h: usize, row: F)
where
    F: Fn((usize, &mut Vec<Cell>)) + Sync + Send,
{
    let rows = grid.len().min(h);
    let slice = &mut grid[..rows];
    if w * h >= PARALLEL_MIN_CELLS {
        slice.par_iter_mut().enumerate().with_min_len(8).for_each(row);
    } else {
        slice.iter_mut().enumerate().for_each(row);
    }
}

fn draw(frame: &mut ModeFrame<'_>, p: &[f32; KNOBS]) {
    let (w, h) = (frame.width, frame.height);
    if w == 0 || h == 0 {
        return;
    }
    let look = Look::new(frame.seed, w, h, frame.time, p);
    SCRATCH.with(|slot| {
        let mut buf = slot.borrow_mut();
        if buf.len() < w * h {
            buf.resize(w * h, CellField::default());
        }
        let field = &mut buf[..w * h];
        measure_layer(NAME, "basins", || solve_basins(field, w, h, &look));
        measure_layer(NAME, "dots", || paint_dots(frame.grid, field, w, h, &look));
        measure_layer(NAME, "veins", || paint_veins(frame.grid, field, w, h, &look));
        measure_layer(NAME, "traps", || paint_traps(frame.grid, field, w, h, &look));
        measure_layer(NAME, "grain", || paint_grain(frame.grid, w, h, &look));
    });
}

/// Warp one subcell sample into the complex plane and run Newton on it.
fn solve_basins(field: &mut [CellField], w: usize, h: usize, look: &Look) {
    let rot = look.spin * look.time;
    let (sr, cr) = rot.sin_cos();
    let mob = look.mob;
    let c = look.cst;
    let relax = look.relax;
    let order = look.order;
    let order_f = look.order_f;
    let seed = look.seed;
    each_row(field, w, h, |(y, row)| {
        for (x, cell) in row.iter_mut().enumerate() {
            // The swirl is a smooth low-frequency field; evaluate it once at
            // the cell center so the eight subcell solves share one rotation
            // and the inner loop stays libm-free.
            let mut zc = (
                ((x as f32 + 0.5 - look.cx) / look.aspect) / look.radius * VIEW,
                (y as f32 + 0.5 - look.cy) / look.radius * VIEW,
            );
            let (zr, zi) = (zc.0 * cr - zc.1 * sr, zc.0 * sr + zc.1 * cr);
            zc = (zr, zi);
            let denc = (
                1.0 - (mob.0 * zc.0 + mob.1 * zc.1),
                mob.1 * zc.0 - mob.0 * zc.1,
            );
            if denc.0 * denc.0 + denc.1 * denc.1 > 1.0e-8 {
                zc = cdiv((zc.0 - mob.0, zc.1 - mob.1), denc);
            }
            let rc = (zc.0 * zc.0 + zc.1 * zc.1).sqrt();
            let angc = zc.1.atan2(zc.0);
            let swirl_c = look.warp
                * (0.5 * (2.0 * angc + look.swirl_p + 0.9 * look.time).sin()
                    + 0.35 * (3.2 * rc * std::f32::consts::PI - 1.1 * look.time).sin());
            let (ws, wc) = swirl_c.sin_cos();
            let mut trap2 = f32::MAX;
            for (k, sub) in cell.subs.iter_mut().enumerate() {
                let col = (k & 1) as f32;
                let rw = (k >> 1) as f32;
                let mut z = (
                    ((x as f32 + 0.25 + 0.5 * col - look.cx) / look.aspect) / look.radius * VIEW,
                    (y as f32 + 0.125 + 0.25 * rw - look.cy) / look.radius * VIEW,
                );
                let (zr, zi) = (z.0 * cr - z.1 * sr, z.0 * sr + z.1 * cr);
                z = (zr, zi);
                // 1 - conj(a) * z for the Mobius map (z - a) / (1 - conj(a) z).
                let den = (
                    1.0 - (mob.0 * z.0 + mob.1 * z.1),
                    mob.1 * z.0 - mob.0 * z.1,
                );
                if den.0 * den.0 + den.1 * den.1 > 1.0e-8 {
                    z = cdiv((z.0 - mob.0, z.1 - mob.1), den);
                }
                z = (z.0 * wc - z.1 * ws, z.0 * ws + z.1 * wc);
                let idx = (y * w + x) as u64;
                if z.0 * z.0 + z.1 * z.1 < NUDGE * NUDGE {
                    let sgn = if hash(seed, L_DUST, idx, 1 + k as u64) & 1 == 0 { 1.0 } else { -1.0 };
                    z = (NUDGE * sgn, NUDGE * sgn);
                }
                let mut t2 = z.0 * z.0 + z.1 * z.1;
                let mut it = 0u8;
                let mut basin = CHAOS;
                while it < MAXIT {
                    let mut zp = z;
                    for _ in 1..order - 1 {
                        zp = cmul(zp, z);
                    }
                    let zn = cmul(zp, z);
                    let f = (zn.0 - c.0, zn.1 - c.1);
                    if f.0 * f.0 + f.1 * f.1 < F_TOL2 {
                        basin = nearest_root(z, look);
                        it += 1;
                        break;
                    }
                    let fp = (zp.0 * order_f, zp.1 * order_f);
                    if fp.0 * fp.0 + fp.1 * fp.1 < 1.0e-12 {
                        let sgn = if hash(seed, L_DUST, idx, 16 + k as u64 + it as u64) & 1 == 0 {
                            1.0
                        } else {
                            -1.0
                        };
                        z = (NUDGE * sgn, -NUDGE * sgn);
                        it += 1;
                        continue;
                    }
                    let step = cdiv(f, fp);
                    let (sx, sy) = (relax * step.0, relax * step.1);
                    let sl2 = sx * sx + sy * sy;
                    // Dives through fp->0 would catapult the orbit off-plane.
                    let (sx, sy) = if sl2 > STEP_CAP * STEP_CAP {
                        let m = STEP_CAP / sl2.sqrt();
                        (sx * m, sy * m)
                    } else {
                        (sx, sy)
                    };
                    z = (z.0 - sx, z.1 - sy);
                    it += 1;
                    let m2 = z.0 * z.0 + z.1 * z.1;
                    if !m2.is_finite() || m2 > ESC2 {
                        break;
                    }
                    if m2 < t2 {
                        t2 = m2;
                    }
                }
                if basin == CHAOS {
                    let mut zp = z;
                    for _ in 1..order - 1 {
                        zp = cmul(zp, z);
                    }
                    let zn = cmul(zp, z);
                    let f = (zn.0 - c.0, zn.1 - c.1);
                    if f.0 * f.0 + f.1 * f.1 < F_TOL2 * 64.0 {
                        basin = nearest_root(z, look);
                    }
                }
                if t2 < trap2 {
                    trap2 = t2;
                }
                *sub = Sub {
                    basin,
                    iters: it,
                };
            }
            cell.trap = trap2.sqrt();
        }
    });
}

#[inline]
fn nearest_root(z: (f32, f32), look: &Look) -> u8 {
    let mut best = f32::MAX;
    let mut bi = 0usize;
    for k in 0..look.order {
        let dx = z.0 - look.roots[k].0;
        let dy = z.1 - look.roots[k].1;
        let d2 = dx * dx + dy * dy;
        if d2 < best {
            best = d2;
            bi = k;
        }
    }
    bi as u8
}

/// Rasterize the cell's eight samples into one braille dot bitmap. Minority
/// and unsettled dots light up, so coastlines dither at subcell precision and
/// calm interiors stay open color.
fn paint_dots(grid: &mut Grid, field: &[CellField], w: usize, h: usize, look: &Look) {
    grid_rows(grid, w, h, |(y, row)| {
        for (x, cell) in row.iter_mut().enumerate().take(w) {
            let f = &field[y * w + x];
            let maj = majority(f);
            let avg = avg_iters(f) as f32;
            let dwell = (avg / MAXIT as f32).min(1.0);
            let calm = 1.0 - dwell;
            let jitter = (unit(hash(look.seed, L_DUST, (y * w + x) as u64, 2)) - 0.5)
                * 0.05
                * look.grain;
            let band = (avg * (look.dot_t * 4.0 + 1.0) / (MAXIT as f32 + 1.0)) as u32;
            let mut bits = 0u8;
            for (k, sub) in f.subs.iter().enumerate() {
                let foreign = sub.basin != maj;
                let slow = sub.iters as f32 >= look.dot_t * MAXIT as f32;
                if foreign || sub.basin == CHAOS || slow {
                    bits |= DOT_BIT[k];
                }
            }
            if maj == CHAOS {
                let bg = hsl_to_rgb(look.base_hue as f64, 0.35, 0.14);
                let fg = hsl_to_rgb(look.base_hue as f64, 0.15, 0.46);
                let g = hash(look.seed, L_DUST, (y * w + x) as u64, 0);
                let ch = if bits == 0 {
                    DUST[(g % 3) as usize]
                } else {
                    char::from_u32(BRAILLE_BASE + bits as u32).unwrap_or(' ')
                };
                *cell = Cell::with_bg(ch, darken(fg, 18), bg);
                continue;
            }
            let hue = (look.base_hue + maj as f32 * look.hue_step + band as f32 * 7.0) % 360.0;
            let sat = (0.74 * calm + 0.16).min(0.85);
            let bg_l = (0.25 + 0.05 * calm + jitter).clamp(0.15, 0.42);
            let fg_l = (bg_l + 0.12 + 0.18 * calm).clamp(0.2, 0.78);
            let bg = hsl_to_rgb(hue as f64, (sat * 0.85) as f64, bg_l as f64);
            let fg = hsl_to_rgb(hue as f64, sat as f64, fg_l as f64);
            let ch = if bits == 0 {
                ' '
            } else {
                char::from_u32(BRAILLE_BASE + bits as u32).unwrap_or(' ')
            };
            *cell = Cell::with_bg(ch, fg, bg);
        }
    });
}

/// Clean single-direction majority splits ink as veins; the dot dither keeps
/// tracing the messy seams underneath, so this only lights calm boundaries.
fn paint_veins(grid: &mut Grid, field: &[CellField], w: usize, h: usize, look: &Look) {
    let glow = (0.45 + 0.45 * look.vein.min(1.0)).clamp(0.0, 0.92);
    grid_rows(grid, w, h, |(y, row)| {
        for (x, cell) in row.iter_mut().enumerate().take(w) {
            let f = &field[y * w + x];
            let maj = majority(f);
            let avg = avg_iters(f) as u8;
            if avg > SEAM_MAXIT {
                continue;
            }
            let mut ndiff = 0u8;
            let mut clean = true;
            for &(ox, oy) in &[(0i32, -1i32), (0, 1), (-1, 0), (1, 0)] {
                let nx = x as i32 + ox;
                let ny = y as i32 + oy;
                if nx < 0 || ny < 0 || nx as usize >= w || ny as usize >= h {
                    continue;
                }
                let n = &field[ny as usize * w + nx as usize];
                if majority(n) != maj {
                    ndiff += 1;
                    if avg_iters(n) as u8 > SEAM_MAXIT {
                        clean = false;
                    }
                }
            }
            if ndiff == 1 && clean {
                let hue = (look.base_hue + maj as f32 * look.hue_step) % 360.0;
                let fg = hsl_to_rgb(hue as f64, 0.9, glow as f64);
                let base = *cell;
                *cell = Cell::with_bg(VEIN_CH, fg, base.bg);
            }
        }
    });
}

/// Orbits that plunged through the origin leave a spark wherever they landed.
fn paint_traps(grid: &mut Grid, field: &[CellField], w: usize, h: usize, look: &Look) {
    if look.trap_k <= 0.0 {
        return;
    }
    let reach = 0.08 + 0.16 * look.trap_k;
    let gate = look.trap_k * 0.22;
    grid_rows(grid, w, h, |(y, row)| {
        for (x, cell) in row.iter_mut().enumerate().take(w) {
            let f = &field[y * w + x];
            if majority(f) == CHAOS || f.trap >= reach {
                continue;
            }
            let g = hash(look.seed, L_TRAP, (y * w + x) as u64, 0);
            if unit(g) >= gate {
                continue;
            }
            let hue = (look.base_hue + 175.0) % 360.0;
            let fg = lighten(hsl_to_rgb(hue as f64, 0.8, 0.66), 12);
            let base = *cell;
            *cell = Cell::with_bg(SPARK[(g % 3) as usize], fg, base.bg);
        }
    });
}

/// Grain last: a radial burn toward the frame edge plus hashed specks in the
/// open cells, so flat interiors still read as printed matter.
fn paint_grain(grid: &mut Grid, w: usize, h: usize, look: &Look) {
    let gx = w as f32 * 0.5;
    let gy = h as f32 * 0.5;
    let burn_k = 6.0 + 14.0 * look.grain;
    let seed = look.seed;
    let grain = look.grain;
    let speck = hsl_to_rgb(((look.base_hue + 175.0) % 360.0) as f64, 0.5, 0.5);
    grid_rows(grid, w, h, |(y, row)| {
        for (x, cell) in row.iter_mut().enumerate().take(w) {
            let nx = (x as f32 + 0.5 - look.cx) / gx;
            let ny = (y as f32 + 0.5 - look.cy) / gy;
            let r2 = nx * nx + ny * ny;
            let burn = (r2 * burn_k) as u8;
            if burn > 0 {
                cell.bg = darken(cell.bg, burn.min(80));
                cell.fg = darken(cell.fg, (burn / 2).min(50));
            }
            if cell.ch == ' ' && grain > 0.0 {
                let g = hash(seed, L_DUST, (y * w + x) as u64, 9);
                if unit(g) < grain * 0.05 {
                    cell.ch = '·';
                    cell.fg = darken(speck, 45);
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::grid_to_plain;
    use rand::{rngs::StdRng, SeedableRng};

    fn knobs() -> Vec<f32> {
        PARAMS.iter().map(|p| p.default).collect()
    }

    fn frame(w: usize, h: usize, seed: u64, time: f32, values: &[f32]) -> Grid {
        let mut grid = vec![vec![Cell::blank(); w]; h];
        let palette = crate::color::make_palette(seed);
        let mut rng = StdRng::seed_from_u64(seed);
        MODE.render(&mut ModeFrame {
            grid: &mut grid,
            width: w,
            height: h,
            seed,
            palette: &palette,
            rng: &mut rng,
            time,
            args: &[],
            param_values: Some(values),
        });
        grid
    }

    fn text(grid: &Grid) -> String {
        grid_to_plain(grid).join("\n")
    }

    #[test]
    fn wada_v2_seed42() {
        insta::assert_snapshot!("wada_v2_80x24", text(&frame(80, 24, 42, 0.0, &knobs())));
    }

    #[test]
    fn wada_v2_seed42_t6() {
        insta::assert_snapshot!("wada_v2_80x24_t6", text(&frame(80, 24, 42, 6.0, &knobs())));
    }

    #[test]
    fn deterministic_and_seed_sensitive() {
        let k = knobs();
        assert_eq!(text(&frame(90, 30, 42, 0.0, &k)), text(&frame(90, 30, 42, 0.0, &k)));
        assert_ne!(text(&frame(90, 30, 42, 0.0, &k)), text(&frame(90, 30, 7, 0.0, &k)));
    }

    #[test]
    fn time_turns_the_plane() {
        let k = knobs();
        assert_ne!(text(&frame(90, 30, 42, 0.0, &k)), text(&frame(90, 30, 42, 4.0, &k)));
    }

    #[test]
    fn order_changes_the_basins() {
        let mut k = knobs();
        let a = text(&frame(90, 30, 42, 0.0, &k));
        k[0] = 7.0;
        assert_ne!(a, text(&frame(90, 30, 42, 0.0, &k)));
    }

    #[test]
    fn frame_cost() {
        let (w, h) = (200usize, 60usize);
        let k = knobs();
        let mut worst = 0.0f64;
        let start = std::time::Instant::now();
        for f in 0..100 {
            let t0 = std::time::Instant::now();
            frame(w, h, 42, f as f32 * 0.05, &k);
            worst = worst.max(t0.elapsed().as_secs_f64() * 1000.0);
        }
        let avg = start.elapsed().as_secs_f64() * 1000.0 / 100.0;
        eprintln!("wada-v2 frame_cost 200x60: avg {:.3} ms, worst {:.3} ms", avg, worst);
        if !cfg!(debug_assertions) {
            assert!(avg < 6.0, "avg frame {:.3} ms", avg);
        }
    }
}
