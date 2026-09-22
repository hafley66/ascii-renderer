//! Wada: Newton's method on a Mobius-twisted plane. Every basin edge is
//! shared by three territories at once; orbit traps spark where a wandering
//! iterate dives through the origin.
use crate::_0_profile::measure_layer;
use crate::color::{darken, hsl_to_rgb, lighten};
use crate::opts::param_f32;
use crate::registry::{AnimKind, Mode, ModeFrame, Param};
use crate::types::{Cell, Grid};
use rayon::prelude::*;
use std::cell::RefCell;
use std::f32::consts::TAU;

pub(super) struct Wada;
pub(super) static MODE: Wada = Wada;

const NAME: &str = "wada";
const KNOBS: usize = 11;
const HELP: &str = "wada: newton basins on a mobius-twisted plane, every boundary shared three ways [order] [relax] [twist] [warp] [spin] [trap] [vein] [contour] [hue] [grain] [aspect]";

/// Iteration shading ramp: fast convergence sits deep, slow sits on the edge.
const RAMP: [char; 10] = [' ', '.', ':', '-', '=', '+', '*', 'o', 'O', '@'];
/// Shared-boundary seam ink.
const VEIN_CH: char = '#';
/// Sparks an orbit-trap dive leaves behind.
const SPARK: [char; 3] = ['·', '+', '✦'];
/// Cells whose iterate never settled: loose dust.
const DUST: [char; 3] = ['.', '\'', '`'];

const MAXIT: u8 = 32;
const F_TOL2: f32 = 4.0e-6;
const ESC2: f32 = 4096.0;
const STEP_CAP: f32 = 1.8;
const SEAM_MAXIT: u8 = MAXIT / 4;
const CHAOS: u8 = u8::MAX;
const VIEW: f32 = 1.75;
const NUDGE: f32 = 1.0e-4;

const L_BASE: u64 = 0x71;
const L_WARP: u64 = 0x72;
const L_DUST: u64 = 0x73;
const L_TRAP: u64 = 0x74;
const L_MOB: u64 = 0x75;

const PARALLEL_MIN_CELLS: usize = 4096;

const PARAMS: &[Param] = &[
    param!("ORDER", "roots on the ring", 3.0, 12.0, 5.0, 1.0),
    param!("RELAX", "newton step size", 0.4, 1.3, 1.0, 0.05),
    param!("TWIST", "mobius twist", 0.0, 0.9, 0.45, 0.05),
    param!("WARP", "swirl warp", 0.0, 1.5, 0.5, 0.05),
    param!("SPIN", "plane rotation rad/s", -1.0, 1.0, 0.15, 0.02),
    param!("TRAP", "orbit trap density", 0.0, 1.5, 0.7, 0.05),
    param!("VEIN", "boundary vein glow", 0.0, 1.5, 0.8, 0.05),
    param!("CONTOUR", "iteration contour bands", 0.0, 12.0, 6.0, 1.0),
    param!("HUE", "hue step per basin", 0.0, 90.0, 34.0, 1.0),
    param!("GRAIN", "dither grain", 0.0, 1.0, 0.35, 0.05),
    param!("ASPECT", "cols per row", 0.25, 4.0, 2.0, 0.25),
];

impl Mode for Wada {
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

/// One cell's Newton result: which root it fell into, how long it took, and
/// how close its orbit ever came to the origin.
#[derive(Clone, Copy, Default)]
struct Sample {
    basin: u8,
    iters: u8,
    trap: f32,
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
    contour: f32,
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
            contour: p[7],
            hue_step: p[8],
            base_hue,
            grain: p[9],
            swirl_p: unit(hash(seed, L_WARP, 0, 7)) * TAU,
            time,
        }
    }
}

thread_local! {
    static SCRATCH: RefCell<Vec<Sample>> = const { RefCell::new(Vec::new()) };
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
            buf.resize(w * h, Sample::default());
        }
        let field = &mut buf[..w * h];
        measure_layer(NAME, "basins", || solve_basins(field, w, h, &look));
        measure_layer(NAME, "shade", || paint_shade(frame.grid, field, w, h, &look));
        measure_layer(NAME, "veins", || paint_veins(frame.grid, field, w, h, &look));
        measure_layer(NAME, "traps", || paint_traps(frame.grid, field, w, h, &look));
        measure_layer(NAME, "grain", || paint_grain(frame.grid, w, h, &look));
    });
}

/// Warp the cell into the complex plane, then run Newton until it settles on
/// a root or wanders into a cycle the step cannot break.
fn solve_basins(field: &mut [Sample], w: usize, h: usize, look: &Look) {
    let rot = look.spin * look.time;
    let (sr, cr) = rot.sin_cos();
    let mob = look.mob;
    let c = look.cst;
    let relax = look.relax;
    let order = look.order;
    let order_f = look.order_f;
    let seed = look.seed;
    each_row(field, w, h, |(y, row)| {
        for (x, s) in row.iter_mut().enumerate() {
            let mut z = (
                ((x as f32 + 0.5 - look.cx) / look.aspect) / look.radius * VIEW,
                (y as f32 + 0.5 - look.cy) / look.radius * VIEW,
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
            let r2 = z.0 * z.0 + z.1 * z.1;
            let r = r2.sqrt();
            let ang = z.1.atan2(z.0);
            let swirl = look.warp
                * (0.5 * (2.0 * ang + look.swirl_p + 0.9 * look.time).sin()
                    + 0.35 * (3.2 * r * std::f32::consts::PI - 1.1 * look.time).sin());
            let (sn, cs) = (ang + swirl).sin_cos();
            z = (r * cs, r * sn);
            if z.0 * z.0 + z.1 * z.1 < NUDGE * NUDGE {
                let sgn = if hash(seed, L_DUST, (y * w + x) as u64, 1) & 1 == 0 {
                    1.0
                } else {
                    -1.0
                };
                z = (NUDGE * sgn, NUDGE * sgn);
            }
            let mut trap2 = z.0 * z.0 + z.1 * z.1;
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
                    let sgn = if hash(seed, L_DUST, (y * w + x) as u64, it as u64) & 1 == 0 {
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
                // A dive through fp->0 would catapult the orbit off-plane;
                // cap the step so it lands back inside the figure instead.
                let (sx, sy) = if sl2 > STEP_CAP * STEP_CAP {
                    let k = STEP_CAP / sl2.sqrt();
                    (sx * k, sy * k)
                } else {
                    (sx, sy)
                };
                z = (z.0 - sx, z.1 - sy);
                it += 1;
                let m2 = z.0 * z.0 + z.1 * z.1;
                if m2 < trap2 {
                    trap2 = m2;
                }
                if !m2.is_finite() || m2 > ESC2 {
                    break;
                }
            }
            if basin == CHAOS {
                let zp_zn = {
                    let mut zp = z;
                    for _ in 1..order - 1 {
                        zp = cmul(zp, z);
                    }
                    cmul(zp, z)
                };
                let f = (zp_zn.0 - c.0, zp_zn.1 - c.1);
                if f.0 * f.0 + f.1 * f.1 < F_TOL2 * 64.0 {
                    basin = nearest_root(z, look);
                }
            }
            *s = Sample {
                basin,
                iters: it,
                trap: trap2.sqrt(),
            };
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
        // A converged iterate sits far closer to its root than to the next
        // root, so a tiny squared distance ends the scan with the same index
        // the full argmin would return.
        if best < 1.0e-4 {
            break;
        }
    }
    bi as u8
}

/// Basin fill: hue per root, light climbs with dwell time, glyph follows the
/// dwell contour, and unsettled cells fall to dust.
fn paint_shade(grid: &mut Grid, field: &[Sample], w: usize, h: usize, look: &Look) {
    let bands = look.contour + 1.0;
    grid_rows(grid, w, h, |(y, row)| {
        for (x, cell) in row.iter_mut().enumerate().take(w) {
            let s = &field[y * w + x];
            if s.basin == CHAOS {
                let g = hash(look.seed, L_DUST, (y * w + x) as u64, 0);
                // Burn holes keep the frame's hue so high-order runs read as
                // charred weave, not punched-out pixels.
                let bg = hsl_to_rgb(look.base_hue as f64, 0.35, 0.14);
                let fg = hsl_to_rgb(look.base_hue as f64, 0.15, 0.46);
                *cell = Cell::with_bg(DUST[(g % 3) as usize], darken(fg, 18), bg);
                continue;
            }
            let dwell = s.iters as f32 / (MAXIT as f32);
            let band = (s.iters as f32 * bands / (MAXIT as f32 + 1.0)) as u32;
            let jitter = (unit(hash(look.seed, L_DUST, (y * w + x) as u64, 2)) - 0.5) * 0.05 * look.grain;
            let hue = (look.base_hue + s.basin as f32 * look.hue_step + band as f32 * 7.0) % 360.0;
            // Calm interiors hold saturated color; the interleaved web loses
            // saturation instead of brightness, so cell-scale dwell jitter
            // never stabs pale highlights through a field. Glyph density
            // carries the dwell contour, veins and sparks carry the light.
            let calm = 1.0 - dwell;
            let sat = (0.74 * calm + 0.16).min(0.85);
            let bg_l = (0.25 + 0.05 * calm + jitter).clamp(0.15, 0.42);
            let fg_l = (bg_l + 0.04 + 0.14 * calm).clamp(0.18, 0.6);
            let idx = ((s.iters as usize * RAMP.len()) / MAXIT as usize).min(RAMP.len() - 1);
            let fg = hsl_to_rgb(hue as f64, sat as f64, fg_l as f64);
            let bg = hsl_to_rgb(hue as f64, (sat * 0.85) as f64, bg_l as f64);
            *cell = Cell::with_bg(RAMP[idx], fg, bg);
        }
    });
}

/// The seam pass. Striated meet-zones alternate labels every pixel; painting
/// those as veins turns the figure to static. A vein needs a clean
/// single-direction split where both sides settled fast, which only a real
/// boundary between two calm basins produces.
fn paint_veins(grid: &mut Grid, field: &[Sample], w: usize, h: usize, look: &Look) {
    let glow = (0.45 + 0.45 * look.vein.min(1.0)).clamp(0.0, 0.92);
    grid_rows(grid, w, h, |(y, row)| {
        for (x, cell) in row.iter_mut().enumerate().take(w) {
            let s = &field[y * w + x];
            if s.iters > SEAM_MAXIT {
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
                if n.basin != s.basin {
                    ndiff += 1;
                    if n.iters > SEAM_MAXIT {
                        clean = false;
                    }
                }
            }
            if ndiff == 1 && clean {
                let hue = (look.base_hue + s.basin as f32 * look.hue_step) % 360.0;
                let fg = hsl_to_rgb(hue as f64, 0.9, glow as f64);
                let base = *cell;
                *cell = Cell::with_bg(VEIN_CH, fg, base.bg);
            }
        }
    });
}

/// Orbits that plunged through the origin leave a spark wherever they landed.
fn paint_traps(grid: &mut Grid, field: &[Sample], w: usize, h: usize, look: &Look) {
    if look.trap_k <= 0.0 {
        return;
    }
    let reach = 0.08 + 0.16 * look.trap_k;
    let gate = look.trap_k * 0.22;
    grid_rows(grid, w, h, |(y, row)| {
        for (x, cell) in row.iter_mut().enumerate().take(w) {
            let s = &field[y * w + x];
            if s.basin == CHAOS || s.trap >= reach {
                continue;
            }
            let g = hash(look.seed, L_TRAP, (y * w + x) as u64, 0);
            if unit(g) >= gate {
                continue;
            }
            let hue = (look.base_hue + 175.0 + s.basin as f32 * look.hue_step) % 360.0;
            let fg = lighten(hsl_to_rgb(hue as f64, 0.8, 0.66), 12);
            let base = *cell;
            *cell = Cell::with_bg(SPARK[(g % 3) as usize], fg, base.bg);
        }
    });
}

/// Grain last: a radial burn toward the frame edge plus hashed specks in the
/// empty cells, so flat interiors still read as printed matter.
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
    fn wada_seed42() {
        insta::assert_snapshot!("wada_80x24", text(&frame(80, 24, 42, 0.0, &knobs())));
    }

    #[test]
    fn wada_seed42_t6() {
        insta::assert_snapshot!("wada_80x24_t6", text(&frame(80, 24, 42, 6.0, &knobs())));
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
        eprintln!("wada frame_cost 200x60: avg {:.3} ms, worst {:.3} ms", avg, worst);
        if !cfg!(debug_assertions) {
            assert!(avg < 6.0, "avg frame {:.3} ms", avg);
        }
    }

}
