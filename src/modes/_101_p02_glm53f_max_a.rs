//! kolam: India Tamil kolam-rangoli mandala. A pulli dot lattice seeds a
//! continuous knot-trace field whose isolines weave between rings, bow around
//! every dot, and deposit an ink field that later growth feeds on.
use crate::_0_profile::measure_layer;
use crate::color::{darken, lighten, lerp_color, shift_hue};
use crate::opts::param_f32;
use crate::registry::{AnimKind, Mode, ModeFrame, Param};
use crate::types::{Cell, Grid};
use crossterm::style::Color;
use rayon::prelude::*;
use std::cell::RefCell;
use std::f32::consts::{PI, TAU};

pub(super) struct Kolam;
pub(super) static MODE: Kolam = Kolam;

const NAME: &str = "kolam";
const KNOBS: usize = 11;
const HELP: &str = "kolam: India Tamil kolam-rangoli mandala, pulli knot interlace and lotus phyllotaxis [curl] [rings] [petals] [florets] [spread] [persist] [embers] [spin] [tilt] [hue] [glow]";

const MAX_RINGS: usize = 7;
const MAX_LINES: usize = MAX_RINGS + 2;
const PARALLEL_MIN_CELLS: usize = 20_480;
const DOT_CUT: f32 = 2.9;

const L_WARP: u64 = 0x51;
const L_DOT: u64 = 0x52;
const L_FLORA: u64 = 0x53;
const L_ORN: u64 = 0x54;
const GOLDEN: f32 = 2.3999632;

const PARAMS: &[Param] = &[
    param!("CURL", "knot weave amplitude", 0.0, 1.2, 0.5, 0.02),
    param!("RINGS", "pulli dot rings", 2.0, 7.0, 4.0, 1.0),
    param!("PETALS", "lotus petals", 4.0, 36.0, 12.0, 1.0),
    param!("FLORETS", "phyllotaxis fill of the court", 0.0, 1.0, 0.6, 0.02),
    param!("SPREAD", "mandala radius", 0.4, 1.0, 0.92, 0.02),
    param!("PERSIST", "ink reinforcement feedback", 0.0, 1.0, 0.55, 0.02),
    param!("EMBERS", "light riding the ink", 0.0, 1.5, 0.6, 0.05),
    param!("SPIN", "wheel rotation rad/s", -0.3, 0.3, 0.05, 0.01),
    param!("TILT", "cols per row", 1.4, 2.8, 2.0, 0.1),
    param!("HUE", "hue bias deg", -90.0, 90.0, 0.0, 5.0),
    param!("GLOW", "exposure", 0.5, 1.6, 1.0, 0.05),
];

impl Mode for Kolam {
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
fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[inline]
fn glow_mod(c: Color, glow: f32) -> Color {
    if glow >= 1.0 {
        lighten(c, ((glow - 1.0) * 70.0) as u8)
    } else {
        darken(c, ((1.0 - glow) * 70.0) as u8)
    }
}

/// One pulli dot in world units: lines bow around it, never through it.
struct Dot {
    x: f32,
    y: f32,
    amp: f32,
    inv_s2: f32,
}

/// One frame's resolved geometry, lattice, and palette.
struct Look {
    seed: u64,
    cx: f32,
    cy: f32,
    aspect: f32,
    radius: f32,
    pitch: f32,
    n_rings: usize,
    ring_r: [f32; MAX_RINGS],
    m_k: [f32; MAX_RINGS],
    amp: [f32; MAX_RINGS],
    inv_bw2: [f32; MAX_RINGS],
    ph: [f32; MAX_RINGS],
    targets: [f32; MAX_LINES],
    n_lines: usize,
    turn: f32,
    dots: Vec<Dot>,
    row_at: Vec<(u32, u32)>,
    line: Color,
    line_hot: Color,
    line_dim: Color,
    dot: Color,
    dot_hot: Color,
    dot_glyph: char,
    petals: usize,
    florets: usize,
    petal_phase: f32,
    floret_phase: f32,
    r_lo: f32,
    r_hi: f32,
    persist: f32,
    petal_edge: Color,
    petal_spine: Color,
    floret: Color,
    floret_hot: Color,
    orn: Color,
}

impl Look {
    fn new(seed: u64, w: usize, h: usize, palette: &[Color; 5], time: f32, p: &[f32; KNOBS]) -> Self {
        let aspect = p[8].max(0.25);
        let radius = ((h as f32 * 0.46).min(w as f32 * 0.5 / aspect) * p[4] - 0.6).max(1.6);
        let cx = w as f32 * 0.5;
        let cy = h as f32 * 0.5;
        let mut n_rings = p[1].round().clamp(1.0, MAX_RINGS as f32) as usize;
        let court = radius * 0.62;
        while n_rings > 1 && court / (n_rings as f32 + 0.6) < 1.45 {
            n_rings -= 1;
        }
        let pitch = court / (n_rings as f32 + 0.6);
        let curl = p[0];
        let turn = p[7] * time;
        let wobble = 0.10 * curl;
        let mut ring_r = [0.0; MAX_RINGS];
        let mut m_k = [0.0; MAX_RINGS];
        let mut amp = [0.0; MAX_RINGS];
        let mut inv_bw2 = [0.0; MAX_RINGS];
        let mut ph = [0.0; MAX_RINGS];
        let base_lobes = 6.0 + 2.0 * (hash(seed, L_WARP, 0, 1) % 5) as f32;
        for k in 0..MAX_RINGS {
            ring_r[k] = pitch * (k + 1) as f32;
            let hk = hash(seed, L_WARP, k as u64, 2);
            m_k[k] = base_lobes + 2.0 * k as f32;
            let taper = 0.45 + 0.55 * k as f32 / (n_rings.max(2) - 1) as f32;
            amp[k] = curl * pitch * (0.32 + 0.14 * unit(hk)) * taper;
            inv_bw2[k] = 1.0 / (2.0 * (pitch * 0.55) * (pitch * 0.55));
            ph[k] = unit(hk) * TAU + (time * 0.35 + k as f32 * 1.7).sin() * wobble;
        }
        let mut targets = [0.0; MAX_LINES];
        targets[0] = pitch * 0.55;
        for i in 1..n_rings {
            targets[i] = (ring_r[i - 1] + ring_r[i]) * 0.5;
        }
        let n_lines = n_rings;
        // Pulli dots: arc spacing near 2.9 world units, staggered per ring.
        let mut dots = Vec::new();
        for k in 0..n_rings {
            let hk = hash(seed, L_DOT, k as u64, 3);
            let count = ((TAU * ring_r[k] / 2.9).round() as usize).clamp(5, 26);
            let off = unit(hk) * TAU + (k % 2) as f32 * PI / count as f32;
            let hs = hash(seed, L_DOT, k as u64, 4);
            let sigma = 0.9 + 0.5 * unit(hs);
            let damp = (0.4 + 0.3 * unit(hash(seed, L_DOT, k as u64, 5))) * (pitch / 2.4).min(1.15);
            let inv_s2 = 1.0 / (2.0 * sigma * sigma);
            for j in 0..count {
                let a = off + j as f32 * TAU / count as f32;
                let (sa, ca) = a.sin_cos();
                dots.push(Dot {
                    x: ring_r[k] * ca,
                    y: ring_r[k] * sa,
                    amp: damp,
                    inv_s2,
                });
            }
        }
        dots.sort_by(|a, b| a.y.partial_cmp(&b.y).unwrap());
        // Row buckets: dots with |dot.y - row dy| <= DOT_CUT, two-pointer sweep.
        let mut row_at = Vec::with_capacity(h);
        let (mut lo, mut hi) = (0usize, 0usize);
        for y in 0..h {
            let dy = y as f32 + 0.5 - cy;
            while lo < dots.len() && dots[lo].y < dy - DOT_CUT {
                lo += 1;
            }
            if hi < lo {
                hi = lo;
            }
            while hi < dots.len() && dots[hi].y <= dy + DOT_CUT {
                hi += 1;
            }
            row_at.push((lo as u32, hi as u32));
        }
        let bias = shift_hue(palette[2], p[9] as f64);
        let petals = p[2].round().clamp(4.0, 36.0) as usize;
        let florets = (40.0 + 320.0 * p[3].clamp(0.0, 1.0)) as usize;
        let r_lo = radius * 0.66;
        let r_hi = radius * 0.98;
        let persist = p[5];
        let petal_phase = unit(hash(seed, L_FLORA, 0, 6)) * TAU;
        let floret_phase = unit(hash(seed, L_FLORA, 1, 6)) * TAU;
        let lotus = shift_hue(palette[3], p[9] as f64);
        let tint = |c: Color| glow_mod(c, p[10]);
        Look {
            seed,
            cx,
            cy,
            aspect,
            radius,
            pitch,
            n_rings,
            ring_r,
            m_k,
            amp,
            inv_bw2,
            ph,
            targets,
            n_lines,
            turn,
            dots,
            row_at,
            line: tint(bias),
            line_hot: tint(lighten(shift_hue(palette[3], p[9] as f64), 18)),
            line_dim: tint(darken(bias, 26)),
            dot: tint(lerp_color(palette[1], palette[2], 0.45)),
            dot_hot: tint(lighten(palette[4], 14)),
            dot_glyph: if pitch > 2.2 { '∙' } else { '·' },
            petals,
            florets,
            petal_phase,
            floret_phase,
            r_lo,
            r_hi,
            persist,
            petal_edge: tint(lerp_color(lotus, bias, 0.25)),
            petal_spine: tint(lighten(lotus, 22)),
            floret: tint(lerp_color(palette[1], lotus, 0.55)),
            floret_hot: tint(lighten(lotus, 30)),
            orn: tint(darken(lerp_color(palette[4], palette[2], 0.35), 6)),
        }
    }
}

thread_local! {
    static PHI: RefCell<Vec<f32>> = const { RefCell::new(Vec::new()) };
    static INK: RefCell<Vec<f32>> = const { RefCell::new(Vec::new()) };
}

/// The knot scalar field: ring-banded lobes over radius, plus a positive bump
/// around every pulli dot so isolines bow around dots instead of crossing.
#[inline]
fn field_at(look: &Look, dx: f32, dy: f32, dots: &[Dot]) -> f32 {
    let r = (dx * dx + dy * dy).sqrt();
    let th = dy.atan2(dx) + look.turn;
    let mut f = r;
    for k in 0..look.n_rings {
        let dr = r - look.ring_r[k];
        let band = (-dr * dr * look.inv_bw2[k]).exp();
        if band > 0.012 {
            f += look.amp[k] * band * (look.m_k[k] * th + look.ph[k]).sin();
        }
    }
    for d in dots {
        let ex = dx - d.x;
        let ey = dy - d.y;
        let q = ex * ex + ey * ey;
        if q < 9.0 {
            f += d.amp * (-q * d.inv_s2).exp();
        }
    }
    f
}

/// Pass 1: evaluate phi per cell, row parallel.
fn trace_field(phi: &mut [f32], w: usize, h: usize, look: &Look) {
    let (cx, cy, aspect) = (look.cx, look.cy, look.aspect);
    let paint = |(y, row): (usize, &mut [f32])| {
        let dy = y as f32 + 0.5 - cy;
        let (lo, hi) = look.row_at[y];
        let dots = &look.dots[lo as usize..hi as usize];
        for (x, slot) in row.iter_mut().enumerate() {
            let dx = (x as f32 + 0.5 - cx) / aspect;
            *slot = field_at(look, dx, dy, dots);
        }
    };
    if w * h >= PARALLEL_MIN_CELLS {
        phi.par_chunks_mut(w).enumerate().with_min_len(8).for_each(paint);
    } else {
        phi.chunks_mut(w).enumerate().for_each(paint);
    }
}

/// Pass 2: isolines of phi become knot glyphs; dots and line cells deposit ink.
fn paint_knots(grid: &mut Grid, ink: &mut [f32], phi: &[f32], w: usize, h: usize, look: &Look) {
    let rows = grid.len().min(h);
    let slice = &mut grid[..rows];
    let paint = |(y, row, ink_row): (usize, &mut Vec<Cell>, &mut [f32])| {
        let dy = y as f32 + 0.5 - look.cy;
        let (lo, hi) = look.row_at[y];
        let dots = &look.dots[lo as usize..hi as usize];
        let up = y.saturating_sub(1);
        let down = (y + 1).min(h - 1);
        let width_cap = 0.9f32;
        for x in 0..w {
            let dx = (x as f32 + 0.5 - look.cx) / look.aspect;
            let r = (dx * dx + dy * dy).sqrt();
            let f = phi[y * w + x];
            let xp = (x + 1).min(w - 1);
            let xm = x.saturating_sub(1);
            let fx = (phi[y * w + xp] - phi[y * w + xm]) * 0.5;
            let fy = (phi[down * w + x] - phi[up * w + x]) * 0.5;
            let gx = fx * look.aspect;
            let gm = (gx * gx + fy * fy).sqrt();
            let width = (0.42 * (0.75 + gm)).min(width_cap);
            let mut weight = 0.0f32;
            for li in 0..look.n_lines {
                let d = (f - look.targets[li]).abs();
                let wgt = 1.0 - smoothstep(d / width);
                if wgt > weight {
                    weight = wgt;
                }
            }
            weight *= 1.0 - smoothstep((r - look.radius * 0.86) / (look.radius * 0.14));
            let mut drew = false;
            if weight > 0.2 {
                let tx = gx * look.aspect;
                let ty = -fy;
                let (ax, ay) = (tx.abs(), ty.abs());
                let ch = if ay > ax * 2.6 {
                    '│'
                } else if ax > ay * 2.6 {
                    '─'
                } else if tx * ty > 0.0 {
                    '╲'
                } else {
                    '╱'
                };
                let col = if weight > 0.55 {
                    if weight > 0.9 {
                        look.line_hot
                    } else {
                        look.line
                    }
                } else {
                    look.line_dim
                };
                row[x] = Cell::new(ch, col);
                drew = weight > 0.55;
                ink_row[x] = ink_row[x].max(weight);
            }
            if !drew {
                for d in dots {
                    let ex = dx - d.x;
                    let ey = dy - d.y;
                    if ex * ex + ey * ey < 0.2 {
                        row[x] = Cell::new(look.dot_glyph, look.dot_hot);
                        ink_row[x] = ink_row[x].max(0.45);
                        break;
                    }
                }
            }
        }
    };
    if w * h >= PARALLEL_MIN_CELLS {
        slice
            .par_iter_mut()
            .zip(ink.par_chunks_mut(w))
            .enumerate()
            .with_min_len(8)
            .for_each(|(y, (row, ink_row))| paint((y, row, ink_row)));
    } else {
        slice
            .iter_mut()
            .zip(ink.chunks_mut(w))
            .enumerate()
            .for_each(|(y, (row, ink_row))| paint((y, row, ink_row)));
    }
}

/// World (rho, angle) to cell coordinates, with bounds.
#[inline]
fn world_cell(look: &Look, rho: f32, a: f32, w: usize, h: usize) -> Option<(usize, usize)> {
    let x = rho * a.cos() * look.aspect + look.cx - 0.5;
    let y = rho * a.sin() + look.cy - 0.5;
    if x < 0.0 || y < 0.0 {
        return None;
    }
    let (xi, yi) = (x as usize, y as usize);
    if xi >= w || yi >= h {
        return None;
    }
    Some((xi, yi))
}

/// Nearest-cell sample of the shared ink field at a world point.
#[inline]
fn ink_at(ink: &[f32], w: usize, h: usize, look: &Look, wx: f32, wy: f32) -> f32 {
    match world_cell(look, (wx * wx + wy * wy).sqrt(), wy.atan2(wx), w, h) {
        Some((xi, yi)) => ink[yi * w + xi],
        None => 0.0,
    }
}

/// The lotus: rim petals lean toward ink-rich arcs and grow longer on them,
/// and a phyllotaxis court fills the inner disc; both deposit reinforcement.
fn grow_lotus(grid: &mut Grid, ink: &mut [f32], w: usize, h: usize, look: &Look) {
    let persist = look.persist;
    let span = look.r_hi - look.r_lo;
    let slot = TAU * (look.r_lo + look.r_hi) * 0.5 / look.petals as f32;
    let w_max = slot * 0.36;
    for i in 0..look.petals {
        let a0 = look.petal_phase + i as f32 * TAU / look.petals as f32 + look.turn;
        let ink_base = ink_at(ink, w, h, look, look.r_lo * a0.cos(), look.r_lo * a0.sin());
        let probe = 0.30f32;
        let ink_l = ink_at(ink, w, h, look, look.r_lo * (a0 + probe).cos(), look.r_lo * (a0 + probe).sin());
        let ink_r = ink_at(ink, w, h, look, look.r_lo * (a0 - probe).cos(), look.r_lo * (a0 - probe).sin());
        let bend = (ink_l - ink_r) * persist * 0.30;
        let grow = 0.78 + 0.45 * ink_base.min(1.0);
        let steps = ((span * grow) * 2.2).ceil().max(5.0) as usize;
        let spine_col = if ink_base > 0.45 {
            look.petal_spine
        } else {
            look.petal_edge
        };
        for s in 0..=steps {
            let t = s as f32 / steps as f32;
            let rho = look.r_lo + span * grow * t;
            let a = a0 + bend * (t * PI).sin();
            let halfw = w_max * (PI * t).sin().powf(0.75);
            let d_off = (halfw / rho.max(0.6)).min(0.6);
            if s == steps {
                if let Some((xi, yi)) = world_cell(look, rho, a, w, h) {
                    grid[yi][xi] = Cell::new('∧', spine_col);
                    ink[yi * w + xi] += 0.4 * persist * (0.5 + ink_base.min(1.0));
                }
                continue;
            }
            for side in [-1.0f32, 1.0] {
                if let Some((xi, yi)) = world_cell(look, rho, a + d_off * side, w, h) {
                    grid[yi][xi] = Cell::new('⌒', look.petal_edge);
                    ink[yi * w + xi] += 0.3 * persist;
                }
            }
            if s % 3 == 0 {
                if let Some((xi, yi)) = world_cell(look, rho, a, w, h) {
                    grid[yi][xi] = Cell::new('~', spine_col);
                    ink[yi * w + xi] += 0.5 * persist * (0.5 + ink_base.min(1.0));
                }
            }
        }
    }
    // Phyllotaxis court: golden-angle florets stipple in, denser on ink.
    let court_r = look.r_lo * 0.92;
    for i in 0..look.florets {
        let rho = court_r * ((i + 1) as f32 / look.florets as f32).sqrt();
        let a = look.floret_phase + (i + 1) as f32 * GOLDEN + look.turn;
        if let Some((xi, yi)) = world_cell(look, rho, a, w, h) {
            if grid[yi][xi].ch == ' ' {
                let v = ink[yi * w + xi];
                let draw_p = 0.15 + 0.55 * v.min(1.0);
                if unit(hash(look.seed, L_FLORA, i as u64, 9)) < draw_p {
                    let (ch, col) = if v > 0.5 {
                        ('●', look.floret_hot)
                    } else if v > 0.18 {
                        ('∙', look.floret)
                    } else {
                        ('·', look.floret)
                    };
                    grid[yi][xi] = Cell::new(ch, col);
                    ink[yi * w + xi] += 0.28 * persist * (0.5 + v.min(1.0));
                }
            }
        }
    }
}

/// Seed ornament dots the blank cells beside soft strokes: it samples the
/// richest neighbor ink and fires in a mid band, avoiding reinforced cores.
fn paint_ornament(grid: &mut Grid, ink: &[f32], w: usize, h: usize, look: &Look) {
    let rows = grid.len().min(h);
    for y in (0..rows).step_by(2) {
        let row = &mut grid[y];
        let south = (y + 1).min(h - 1);
        for x in (0..w).step_by(2) {
            if row[x].ch != ' ' {
                continue;
            }
            let east = (x + 1).min(w - 1);
            let west = x.saturating_sub(1);
            let v = ink[y * w + east]
                .max(ink[y * w + west])
                .max(ink[south * w + x])
                .max(ink[y.saturating_sub(1) * w + x]);
            if v < 0.2 {
                continue;
            }
            let z = (v - 0.38) / 0.16;
            if unit(hash(look.seed, L_ORN, x as u64, y as u64)) < (-z * z).exp() {
                row[x] = Cell::new('◇', look.orn);
            }
        }
    }
}

fn draw(frame: &mut ModeFrame<'_>, p: &[f32; KNOBS]) {
    let (w, h) = (frame.width, frame.height);
    if w < 2 || h < 2 {
        return;
    }
    let look = measure_layer(NAME, "look", || {
        Look::new(frame.seed, w, h, frame.palette, frame.time, p)
    });
    PHI.with(|cell_| {
        let mut phi = cell_.borrow_mut();
        if phi.len() < w * h {
            phi.resize(w * h, 0.0);
        }
        let phi = &mut phi[..w * h];
        measure_layer(NAME, "field", || trace_field(phi, w, h, &look));
        INK.with(|store| {
            let mut ink = store.borrow_mut();
            if ink.len() < w * h {
                ink.resize(w * h, 0.0);
            }
            let ink = &mut ink[..w * h];
            for v in ink.iter_mut() {
                *v = 0.0;
            }
            measure_layer(NAME, "knots", || paint_knots(frame.grid, ink, phi, w, h, &look));
            measure_layer(NAME, "lotus", || grow_lotus(frame.grid, ink, w, h, &look));
            measure_layer(NAME, "ornament", || paint_ornament(frame.grid, ink, w, h, &look));
        });
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
    fn kolam_80x24() {
        insta::assert_snapshot!("kolam_80x24", text(&frame(80, 24, 42, 0.0, &knobs())));
    }

    #[test]
    fn kolam_clip_40x12() {
        insta::assert_snapshot!("kolam_clip_40x12", text(&frame(40, 12, 42, 0.0, &knobs())));
    }

    #[test]
    fn kolam_lotus_variant() {
        let mut k = knobs();
        k[2] = 30.0;
        k[3] = 0.9;
        k[5] = 0.9;
        insta::assert_snapshot!("kolam_lotus_variant", text(&frame(80, 24, 42, 0.0, &k)));
    }

    #[test]
    fn petals_change_the_rim() {
        let k = knobs();
        let a = text(&frame(90, 30, 42, 0.0, &k));
        let mut wide = k.clone();
        wide[2] = 6.0;
        assert_ne!(a, text(&frame(90, 30, 42, 0.0, &wide)));
    }

    #[test]
    fn deterministic_and_seed_sensitive() {
        let k = knobs();
        assert_eq!(text(&frame(90, 30, 42, 0.0, &k)), text(&frame(90, 30, 42, 0.0, &k)));
        assert_ne!(text(&frame(90, 30, 42, 0.0, &k)), text(&frame(90, 30, 7, 0.0, &k)));
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
        eprintln!("kolam frame_cost 200x60: avg {:.3} ms, worst {:.3} ms", avg, worst);
        if !cfg!(debug_assertions) {
            assert!(avg < 6.0, "avg frame {:.3} ms", avg);
        }
    }
}
