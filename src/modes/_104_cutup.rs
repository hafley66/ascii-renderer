//! Cutup: a woodblock landscape torn into a collage. Fragments of the print shift,
//! mirror and invert over bare paper under a misregistered key impression.
use crate::_0_profile::measure_layer;
use crate::color::{darken, hsl_to_rgb, lighten, shift_hue};
use crate::opts::param_f32;
use crate::registry::{AnimKind, Mode, ModeFrame, Param};
use crate::types::{Cell, Grid};
use crossterm::style::Color;
use rayon::prelude::*;
use std::cell::RefCell;
use std::f32::consts::TAU;

pub(super) struct Cutup;
pub(super) static MODE: Cutup = Cutup;

const NAME: &str = "cutup";
const KNOBS: usize = 11;
const HELP: &str = "cutup: woodblock landscape torn into a cut-up collage [vcuts] [hcuts] [tear] [shift] [flip] [drift] [misreg] [ink] [grain] [hue] [aspect]";

/// Carved ink ramp, bare paper to solid cut.
const RAMP: [char; 10] = [' ', '.', ':', '-', '=', '+', '*', 'o', 'O', '@'];
const RAMP_LEN: usize = RAMP.len();
/// Torn paper fibers along a rip.
const FIBER: [char; 3] = ['.', ':', '\''];
/// The misregistered key impression: solid over ink, ghost over bare paper.
const PRESS: [char; 2] = ['#', '%'];

const B_SKY: u8 = 0;
const B_FAR: u8 = 1;
const B_NEAR: u8 = 2;
const B_WATER: u8 = 3;
const B_SUN: u8 = 4;
const B_KEY: u8 = 5;
const BLOCKS: usize = 6;

const SUN_U: f32 = 0.66;
const SUN_V: f32 = 0.30;
const SUN_ROWS: f32 = 0.19;
const WATER_V: f32 = 0.76;
const RAY_N: f32 = 14.0;
const FIBER_W: f32 = 0.6;

const L_SKY: u64 = 0x21;
const L_RIDGE: u64 = 0x25;
const L_WATER: u64 = 0x29;
const L_CUT: u64 = 0x2D;
const L_FRAG: u64 = 0x31;
const L_PRESS: u64 = 0x3B;
const L_GRAIN: u64 = 0x3F;

const PARALLEL_MIN_CELLS: usize = 20_480;

const PARAMS: &[Param] = &[
    param!("VCUTS", "vertical tears", 0.0, 12.0, 4.0, 1.0),
    param!("HCUTS", "horizontal tears", 0.0, 12.0, 2.0, 1.0),
    param!("TEAR", "tear wobble cells", 0.0, 4.0, 1.25, 0.25),
    param!("SHIFT", "fragment shift cells", 0.0, 24.0, 6.0, 1.0),
    param!("FLIP", "fragment disruption", 0.0, 1.0, 0.45, 0.05),
    param!("DRIFT", "drift over time", 0.0, 1.0, 0.4, 0.05),
    param!("MISREG", "misregistration cells", 0.0, 6.0, 1.75, 0.25),
    param!("INK", "ink coverage", 0.0, 1.0, 0.65, 0.05),
    param!("GRAIN", "paper grain", 0.0, 1.0, 0.35, 0.05),
    param!("HUE", "base hue", 0.0, 360.0, 34.0, 1.0),
    param!("ASPECT", "cols per row", 0.25, 4.0, 2.0, 0.25),
];

impl Mode for Cutup {
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

/// Value noise in one dimension, for ridge crests, gouges and tear wobble.
#[inline]
fn noise1(seed: u64, layer: u64, t: f32) -> f32 {
    let i = t.floor();
    let f = smoothstep(t - i);
    let i = i as i64 as u64;
    let a = unit(hash(seed, layer, i, 0));
    let b = unit(hash(seed, layer, i.wrapping_add(1), 0));
    (a + (b - a) * f) * 2.0 - 1.0
}

/// Triangular peak of width s and height a centered at c.
#[inline]
fn peak(u: f32, c: f32, s: f32, a: f32) -> f32 {
    (1.0 - ((u - c) / s).abs()).max(0.0) * a
}

/// One printed block: hue offset from the base, saturation, and the ink ramp
/// lightness floor and step per level.
struct Tone {
    hue: f64,
    sat: f64,
    lo: f64,
    step: f64,
}

const TONES: [Tone; BLOCKS] = [
    Tone { hue: 200.0, sat: 0.30, lo: 0.26, step: 0.030 },
    Tone { hue: 250.0, sat: 0.42, lo: 0.24, step: 0.034 },
    Tone { hue: 15.0, sat: 0.50, lo: 0.16, step: 0.028 },
    Tone { hue: 185.0, sat: 0.55, lo: 0.22, step: 0.038 },
    Tone { hue: 45.0, sat: 0.85, lo: 0.34, step: 0.050 },
    Tone { hue: 5.0, sat: 0.10, lo: 0.10, step: 0.012 },
];

/// One frame's resolved geometry, knobs and palette.
struct Look {
    seed: u64,
    w: usize,
    h: usize,
    aspect: f32,
    sx: f32,
    sy: f32,
    sun_r: f32,
    water_y: f32,
    vcuts: usize,
    hcuts: usize,
    tear: f32,
    shift: f32,
    flip: f32,
    drift: f32,
    ink_scale: f32,
    grain: f32,
    base_hue: f64,
    paper: Color,
    over_col: Color,
    base_cols: [[Color; RAMP_LEN]; BLOCKS],
    mx: f32,
    my: f32,
    time: f32,
}

impl Look {
    fn new(seed: u64, w: usize, h: usize, palette: &[Color; 5], time: f32, p: &[f32; KNOBS]) -> Self {
        let base_hue = p[9] as f64;
        let mut base_cols = [[Color::Reset; RAMP_LEN]; BLOCKS];
        for (b, row) in base_cols.iter_mut().enumerate() {
            for (l, col) in row.iter_mut().enumerate() {
                let t = &TONES[b];
                let lit = (t.lo + l as f64 * t.step).clamp(0.05, 0.85);
                *col = hsl_to_rgb((base_hue + t.hue).rem_euclid(360.0), t.sat, lit);
            }
        }
        let ang = unit(hash(seed, L_PRESS, 0, 1)) * TAU;
        Look {
            seed,
            w,
            h,
            aspect: p[10].max(0.25),
            sx: w as f32 * SUN_U,
            sy: h as f32 * SUN_V,
            sun_r: (h as f32 * SUN_ROWS).max(1.5),
            water_y: h as f32 * WATER_V,
            vcuts: (p[0].round().max(0.0) as usize).min((w / 4).max(2) - 1),
            hcuts: (p[1].round().max(0.0) as usize).min((h / 3).max(2) - 1),
            tear: p[2],
            shift: p[3],
            flip: p[4],
            drift: p[5],
            ink_scale: 0.5 + 0.76 * p[7],
            grain: p[8],
            base_hue,
            paper: darken(palette[0], 26),
            over_col: hsl_to_rgb((base_hue + 165.0).rem_euclid(360.0), 0.55, 0.5),
            base_cols,
            mx: p[6] * 0.8 * ang.cos(),
            my: p[6] * 0.45 * ang.sin(),
            time,
        }
    }
}

/// One plate cell: carved ink level and the block that printed it.
#[derive(Clone, Copy, Default)]
struct Ink {
    level: u8,
    block: u8,
}

/// One torn fragment of the sheet: where it was cut, where it was pasted, and
/// how its ink mix differs from the plate.
#[derive(Clone, Copy)]
struct Frag {
    i: usize,
    j: usize,
    x0: f32,
    x1: f32,
    y0: f32,
    y1: f32,
    nx0: i32,
    nx1: i32,
    ny0: i32,
    ny1: i32,
    dx: i32,
    dy: i32,
    flip_x: bool,
    flip_y: bool,
    neg: bool,
    dlevel: i32,
    cols: [[Color; RAMP_LEN]; BLOCKS],
    neg_bg: [Color; BLOCKS],
    neg_fg: [Color; BLOCKS],
    osc_amp: f32,
    osc_w: f32,
    osc_ph: f32,
    z: u64,
}

#[derive(Default)]
struct Buffers {
    plate: Vec<Ink>,
    cut_x: Vec<f32>,
    cut_y: Vec<f32>,
    frags: Vec<Frag>,
}

thread_local! {
    static BUFFERS: RefCell<Buffers> = const {
        RefCell::new(Buffers {
            plate: Vec::new(),
            cut_x: Vec::new(),
            cut_y: Vec::new(),
            frags: Vec::new(),
        })
    };
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
fn each_grid_row<F>(grid: &mut Grid, w: usize, h: usize, row: F)
where
    F: Fn((usize, &mut Vec<Cell>)) + Sync + Send,
{
    let rows = grid.len().min(h);
    if w * h >= PARALLEL_MIN_CELLS {
        grid[..rows].par_iter_mut().enumerate().with_min_len(8).for_each(row);
    } else {
        grid[..rows].iter_mut().enumerate().for_each(row);
    }
}

fn draw(frame: &mut ModeFrame<'_>, p: &[f32; KNOBS]) {
    let (w, h) = (frame.width, frame.height);
    if w == 0 || h == 0 {
        return;
    }
    let look = Look::new(frame.seed, w, h, frame.palette, frame.time, p);
    BUFFERS.with(|slot| {
        let mut buf = slot.borrow_mut();
        let Buffers { plate, cut_x, cut_y, frags } = &mut *buf;
        plate.clear();
        plate.resize(w * h, Ink::default());
        measure_layer(NAME, "plate", || sample_plate(plate, w, h, &look));
        measure_layer(NAME, "cuts", || build_cuts(cut_x, cut_y, frags, w, h, &look));
        measure_layer(NAME, "backing", || paint_backing(frame.grid, w, h, &look));
        measure_layer(NAME, "collage", || {
            paint_collage(frame.grid, plate, cut_x, cut_y, frags, w, h, &look)
        });
        measure_layer(NAME, "overprint", || paint_overprint(frame.grid, plate, w, h, &look));
        measure_layer(NAME, "grain", || paint_grain(frame.grid, w, h, &look));
    });
}

/// The plate: sun, two ridges and water, carved as one woodblock landscape.
fn sample_plate(plate: &mut [Ink], w: usize, h: usize, look: &Look) {
    each_row(plate, w, h, |(y, row)| {
        for (x, ink) in row.iter_mut().enumerate() {
            *ink = plate_at(look, x, y, w, h);
        }
    });
    for b in 0..3u64 {
        let bu = unit(hash(look.seed, L_SKY, b, 1));
        let bv = unit(hash(look.seed, L_SKY, b, 2));
        let bx = (w as f32 * (0.12 + 0.26 * b as f32 + (bu - 0.5) * 0.12)) as i32;
        let by = (h as f32 * (0.10 + 0.08 * (b % 2) as f32 + bv * 0.12)) as i32;
        for (ox, oy) in [(-1i32, 0i32), (1, 0), (0, 1)] {
            let px = bx + ox;
            let py = by + oy;
            if px >= 0 && py >= 0 && (px as usize) < w && (py as usize) < h {
                let idx = py as usize * w + px as usize;
                if plate[idx].level == 0 {
                    plate[idx] = Ink { level: 9, block: B_KEY };
                }
            }
        }
    }
}

fn plate_at(look: &Look, x: usize, y: usize, w: usize, h: usize) -> Ink {
    let px = x as f32 + 0.5;
    let py = y as f32 + 0.5;
    let u = px / w as f32;
    let v = py / h as f32;
    let dxs = (px - look.sx) / look.aspect;
    let dys = py - look.sy;
    let r = (dxs * dxs + dys * dys).sqrt();

    if r < look.sun_r {
        let rim = r > look.sun_r - 0.7;
        let level = if rim || r < look.sun_r * 0.3 {
            9u8
        } else {
            (8 - ((r / look.sun_r * 2.5) as i32 % 2)) as u8
        };
        return Ink { level, block: if rim { B_KEY } else { B_SUN } };
    }
    if r > look.sun_r * 1.25 && r < look.sun_r * 1.8 {
        let a = dys.atan2(dxs);
        let ray = (a / TAU * RAY_N).rem_euclid(1.0);
        if ray < 0.18 {
            return Ink { level: 8, block: B_KEY };
        }
    }

    let d1 = noise1(look.seed, L_RIDGE, u * 6.5);
    let d2 = noise1(look.seed, L_RIDGE + 1, u * 11.0 + 31.0);
    let y_far = (0.55 - peak(u, 0.30, 0.24, 0.17) - peak(u, 0.80, 0.16, 0.09) + d1 * 0.03) * h as f32;
    let y_near = (0.71 - peak(u, 0.62, 0.20, 0.13) - peak(u, 0.16, 0.15, 0.07) + d2 * 0.025)
        * h as f32;
    let y_near = y_near.max(y_far + 2.0);
    let y_w = look.water_y;

    if py < y_far {
        for band in 0..2usize {
            let c0 = h as f32 * if band == 0 { 0.09 } else { 0.42 };
            let c1 = h as f32 * if band == 0 { 0.15 } else { 0.47 };
            if py >= c0 && py < c1 {
                let n = noise1(look.seed, L_SKY + 1 + band as u64, u * 9.0 + band as f32 * 37.0);
                if n > 0.35 {
                    return Ink { level: 2 + (n > 0.6) as u8, block: B_SKY };
                }
            }
        }
        return Ink::default();
    }
    if py < y_near {
        let mut level = 4u8;
        if py - y_far < 1.3 {
            level += 1;
        }
        let g = noise1(look.seed, L_RIDGE + 2, u * 13.0 + v * 5.0);
        if g > 0.62 {
            level = 2;
        }
        return Ink { level: level.min(6), block: B_FAR };
    }
    if py < y_w {
        let mut level = 7u8;
        if py - y_near < 1.2 || y_w - py < 1.5 {
            level += 1;
        }
        let g = noise1(look.seed, L_RIDGE + 3, u * 15.0 + v * 7.0);
        if g > 0.6 {
            level = 3;
        }
        return Ink { level: level.min(8), block: B_NEAR };
    }
    if py - y_w < 0.8 {
        let gap = unit(hash(look.seed, L_WATER, x as u64, 0));
        return Ink { level: if gap < 0.18 { 0 } else { 9 }, block: B_KEY };
    }
    let n = noise1(look.seed, L_WATER + 1, u * 7.0 + py * 0.8);
    let dash = unit(hash(look.seed, L_WATER + 2, x as u64, y as u64));
    let depth = ((py - y_w) / (h as f32 - y_w).max(1.0)).clamp(0.0, 1.0);
    let mut level = 2 + ((n + 1.0) * 1.4) as u8;
    level += (depth * 2.2) as u8;
    if (px - look.sx).abs() < (py - y_w) * 0.35 + 1.0 + n * 0.8 {
        level = level.saturating_sub(2);
    }
    if dash < 0.18 {
        level = level.saturating_sub(1);
    }
    Ink { level: level.min(7), block: B_WATER }
}

/// Wobbled cut curves and the fragment table: cut positions, paste offsets,
/// mirroring, negative pieces, ink mix and drift phases.
fn build_cuts(cut_x: &mut Vec<f32>, cut_y: &mut Vec<f32>, frags: &mut Vec<Frag>, w: usize, h: usize, look: &Look) {
    let (vc, hc) = (look.vcuts, look.hcuts);
    let mut xs = Vec::with_capacity(vc + 2);
    xs.push(0.0f32);
    let xstep = w as f32 / (vc + 1) as f32;
    for i in 1..=vc {
        let jit = unit(hash(look.seed, L_CUT, i as u64, 1)) - 0.5;
        xs.push((xstep * i as f32 + jit * xstep * 0.7).clamp(1.0, w as f32 - 1.0));
    }
    xs.push(w as f32);
    let mut ys = Vec::with_capacity(hc + 2);
    ys.push(0.0f32);
    let ystep = h as f32 / (hc + 1) as f32;
    for j in 1..=hc {
        let jit = unit(hash(look.seed, L_CUT, j as u64, 2)) - 0.5;
        ys.push((ystep * j as f32 + jit * ystep * 0.7).clamp(1.0, h as f32 - 1.0));
    }
    ys.push(h as f32);

    cut_x.clear();
    cut_x.resize(h * vc, 0.0);
    for y in 0..h {
        let t = y as f32 + 0.5;
        for i in 1..=vc {
            let wob = noise1(look.seed, L_CUT, t * 0.13 + i as f32 * 7.3)
                + noise1(look.seed, L_CUT + 1, t * 0.31 + i as f32 * 3.1) * 0.4;
            let lo = xs[i - 1] + 1.0;
            let hi = (xs[i + 1] - 1.0).max(lo + 0.5);
            cut_x[y * vc + (i - 1)] = (xs[i] + wob * look.tear).clamp(lo, hi);
        }
    }
    cut_y.clear();
    cut_y.resize(w * hc, 0.0);
    for x in 0..w {
        let t = x as f32 + 0.5;
        for j in 1..=hc {
            let wob = noise1(look.seed, L_CUT + 2, t * 0.15 + j as f32 * 5.7)
                + noise1(look.seed, L_CUT + 3, t * 0.33 + j as f32 * 2.3) * 0.4;
            let lo = ys[j - 1] + 1.0;
            let hi = (ys[j + 1] - 1.0).max(lo + 0.5);
            cut_y[x * hc + (j - 1)] = (ys[j] + wob * look.tear).clamp(lo, hi);
        }
    }

    frags.clear();
    for j in 0..=hc {
        for i in 0..=vc {
            let h0 = hash(look.seed, L_FRAG, i as u64, j as u64);
            let h1 = hash(look.seed, L_FRAG + 1, i as u64, j as u64);
            let h2 = hash(look.seed, L_FRAG + 2, i as u64, j as u64);
            let h3 = hash(look.seed, L_FRAG + 3, i as u64, j as u64);
            let h4 = hash(look.seed, L_FRAG + 4, i as u64, j as u64);
            let h5 = hash(look.seed, L_FRAG + 5, i as u64, j as u64);
            let h6 = hash(look.seed, L_FRAG + 6, i as u64, j as u64);
            let h7 = hash(look.seed, L_FRAG + 7, i as u64, j as u64);
            let (x0, x1) = (xs[i], xs[i + 1]);
            let (y0, y1) = (ys[j], ys[j + 1]);
            let fw = (x1 - x0).max(1.0);
            let fh = (y1 - y0).max(1.0);
            let dx = ((unit(h0) * 2.0 - 1.0) * look.shift.min(fw * 0.75)).round() as i32;
            let dy = ((unit(h1) * 2.0 - 1.0) * (look.shift * 0.45).min(fh * 0.75)).round() as i32;
            let flip_x = unit(h2) < look.flip;
            let flip_y = unit(h3) < look.flip * 0.5;
            let neg = unit(h4) < look.flip * 0.35;
            let dlevel = (unit(h5) * 5.0) as i32 - 2;
            let jit = ((unit(h6) * 2.0 - 1.0) * 28.0 / 12.0).round() * 12.0;
            let mut cols = [[Color::Reset; RAMP_LEN]; BLOCKS];
            for (b, row) in cols.iter_mut().enumerate() {
                for (l, col) in row.iter_mut().enumerate() {
                    *col = if jit == 0.0 {
                        look.base_cols[b][l]
                    } else {
                        shift_hue(look.base_cols[b][l], jit as f64)
                    };
                }
            }
            let mut neg_bg = [Color::Reset; BLOCKS];
            let mut neg_fg = [Color::Reset; BLOCKS];
            for (b, (bg, fg)) in neg_bg.iter_mut().zip(neg_fg.iter_mut()).enumerate() {
                let hue = (look.base_hue + TONES[b].hue + jit as f64).rem_euclid(360.0);
                *bg = hsl_to_rgb(hue, TONES[b].sat * 0.7, 0.18);
                *fg = hsl_to_rgb(hue, TONES[b].sat * 0.35, 0.78);
            }
            frags.push(Frag {
                i,
                j,
                x0,
                x1,
                y0,
                y1,
                nx0: x0.round() as i32,
                nx1: x1.round() as i32,
                ny0: y0.round() as i32,
                ny1: y1.round() as i32,
                dx,
                dy,
                flip_x,
                flip_y,
                neg,
                dlevel,
                cols,
                neg_bg,
                neg_fg,
                osc_amp: look.drift * (1.2 + unit(h7) * 2.2),
                osc_w: 0.35 + unit(hash(look.seed, L_FRAG + 8, i as u64, j as u64)) * 0.9,
                osc_ph: unit(hash(look.seed, L_FRAG + 9, i as u64, j as u64)) * TAU,
                z: hash(look.seed, L_FRAG + 10, i as u64, j as u64),
            });
        }
    }
    frags.sort_by_key(|f| f.z);
}

/// Paper backing: bare sheet with sparse fibers where the print was torn away.
fn paint_backing(grid: &mut Grid, w: usize, h: usize, look: &Look) {
    each_grid_row(grid, w, h, |(y, row)| {
        let bg = darken(look.paper, (y as f32 / h.max(1) as f32 * 8.0) as u8);
        for (x, cell) in row.iter_mut().enumerate().take(w) {
            let g = hash(look.seed, L_GRAIN + 1, x as u64, y as u64);
            let ch = if unit(g) < 0.025 { FIBER[(g >> 8) as usize % FIBER.len()] } else { ' ' };
            *cell = Cell::with_bg(ch, darken(look.paper, 30), bg);
        }
    });
}

/// Paste each fragment back on the sheet in hashed z order: rows are painted
/// independently and each row honors the order, so overlaps stay deterministic.
fn paint_collage(
    grid: &mut Grid,
    plate: &[Ink],
    cut_x: &[f32],
    cut_y: &[f32],
    frags: &[Frag],
    w: usize,
    h: usize,
    look: &Look,
) {
    each_grid_row(grid, w, h, |(y, row)| {
        for frag in frags {
            let (ox, oy) = drift_off(frag, look.time);
            let (dx, dy) = (frag.dx + ox, frag.dy + oy);
            let ry = y as i32 - dy;
            if ry < 0 || ry >= h as i32 {
                continue;
            }
            let ly = ry as f32 + 0.5;
            if ly < frag.y0 || ly >= frag.y1 {
                continue;
            }
            let x_lo = (frag.x0 + dx as f32 - 0.5).ceil().max(0.0) as usize;
            let x_hi = (frag.x1 + dx as f32 - 0.5).ceil().clamp(0.0, w as f32) as usize;
            let cx_lo = if frag.i == 0 { 0.0 } else { cut_x[ry as usize * look.vcuts + frag.i - 1] };
            let cx_hi = if frag.i == look.vcuts {
                w as f32
            } else {
                cut_x[ry as usize * look.vcuts + frag.i]
            };
            for x in x_lo..x_hi {
                let rx = x as i32 - dx;
                if rx < 0 || rx >= w as i32 {
                    continue;
                }
                let lx = rx as f32 + 0.5;
                if lx < cx_lo || lx >= cx_hi {
                    continue;
                }
                let cy_lo = if frag.j == 0 { 0.0 } else { cut_y[rx as usize * look.hcuts + frag.j - 1] };
                let cy_hi = if frag.j == look.hcuts {
                    h as f32
                } else {
                    cut_y[rx as usize * look.hcuts + frag.j]
                };
                if ly < cy_lo || ly >= cy_hi {
                    continue;
                }
                let edge = (lx - cx_lo).min(cx_hi - lx).min(ly - cy_lo).min(cy_hi - ly);
                let mut px = rx;
                if frag.flip_x {
                    px = frag.nx0 + frag.nx1 - 1 - rx;
                }
                let mut py = ry;
                if frag.flip_y {
                    py = frag.ny0 + frag.ny1 - 1 - ry;
                }
                let px = px.clamp(0, w as i32 - 1) as usize;
                let py = py.clamp(0, h as i32 - 1) as usize;
                let g = hash(look.seed, L_FRAG + 11, x as u64, y as u64);
                row[x] = paint_cell(look, frag, &plate[py * w + px], g, edge < FIBER_W);
            }
        }
    });
}

/// Fragment ink: level shifted and scaled by the ink knob, then printed either
/// straight or as a negative with the block color as ground.
fn paint_cell(look: &Look, frag: &Frag, ink: &Ink, g: u64, fiber: bool) -> Cell {
    let b = ink.block as usize;
    if fiber {
        let ch = FIBER[(g >> 8) as usize % FIBER.len()];
        return if frag.neg {
            Cell::with_bg(ch, frag.neg_fg[b], lighten(frag.neg_bg[b], 18))
        } else {
            Cell::with_bg(ch, darken(look.paper, 24), lighten(look.paper, 18))
        };
    }
    let level = (((ink.level as i32 + frag.dlevel).max(0)) as f32 * look.ink_scale)
        .round()
        .clamp(0.0, 9.0) as usize;
    if frag.neg {
        if level == 0 {
            Cell::with_bg(' ', frag.neg_bg[b], frag.neg_bg[b])
        } else {
            let ch = RAMP[(1 + level * 4 / 9).min(5)];
            Cell::with_bg(ch, frag.neg_fg[b], frag.neg_bg[b])
        }
    } else {
        let bg = if level == 0 { look.paper } else { darken(look.paper, 8) };
        Cell::with_bg(RAMP[level], frag.cols[b][level], bg)
    }
}

/// The key block runs again over the reassembled sheet, off register.
fn paint_overprint(grid: &mut Grid, plate: &[Ink], w: usize, h: usize, look: &Look) {
    let slide = look.drift;
    let osc = |rate: f32, ph: f32| slide * ((look.time * rate + ph).sin() - ph.sin());
    let mx = look.mx + osc(0.53, 0.7) * 1.1;
    let my = look.my + osc(0.41, 2.1) * 0.7;
    each_grid_row(grid, w, h, |(y, row)| {
        for (x, cell) in row.iter_mut().enumerate().take(w) {
            let sx = x as f32 + 0.5 + mx;
            let sy = y as f32 + 0.5 + my;
            if sx < 0.0 || sy < 0.0 || sx >= w as f32 || sy >= h as f32 {
                continue;
            }
            let ink = &plate[sy as usize * w + sx as usize];
            if ink.block != B_KEY || ink.level < 7 {
                continue;
            }
            let ch = if rank(cell.ch) >= 3 { PRESS[0] } else { PRESS[1] };
            *cell = Cell::with_bg(ch, look.over_col, darken(cell.bg, 5));
        }
    });
}

/// Paper grain, worn ink, and the press bite along the sheet edge.
fn paint_grain(grid: &mut Grid, w: usize, h: usize, look: &Look) {
    let amount = look.grain;
    each_grid_row(grid, w, h, |(y, row)| {
        for (x, cell) in row.iter_mut().enumerate().take(w) {
            let g = hash(look.seed, L_GRAIN, x as u64, y as u64);
            let u = unit(g);
            if cell.ch == ' ' {
                if u < amount * 0.06 {
                    cell.ch = FIBER[(g >> 8) as usize % FIBER.len()];
                    cell.fg = darken(look.paper, 34);
                }
            } else if u < amount * 0.12 {
                cell.ch = soften(cell.ch);
                cell.fg = lighten(cell.fg, 10);
            }
            if x == 0 || y == 0 || x + 1 == w || y + 1 == h {
                cell.bg = darken(cell.bg, 18);
                if unit(hash(look.seed, L_GRAIN + 1, x as u64, y as u64 | 1 << 40)) < 0.3 {
                    cell.ch = ' ';
                }
            }
        }
    });
}

/// Fragment drift: integer cell oscillation that rests at zero when time is zero.
#[inline]
fn drift_off(frag: &Frag, time: f32) -> (i32, i32) {
    if time == 0.0 || frag.osc_amp <= 0.0 {
        return (0, 0);
    }
    let s = (time * frag.osc_w + frag.osc_ph).sin() - frag.osc_ph.sin();
    ((s * frag.osc_amp).round() as i32, (s * frag.osc_amp * 0.45).round() as i32)
}

#[inline]
fn rank(ch: char) -> u8 {
    match ch {
        ' ' => 0,
        '#' => 5,
        '%' => 3,
        _ => RAMP.iter().position(|&r| r == ch).map(|i| i as u8).unwrap_or(2),
    }
}

/// Wear one step down the ink ramp.
fn soften(ch: char) -> char {
    if ch == '#' {
        return '%';
    }
    match RAMP.iter().position(|&r| r == ch) {
        Some(i) if i > 1 => RAMP[i - 1],
        _ => ch,
    }
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
    fn cutup_seed42() {
        insta::assert_snapshot!("cutup_80x24", text(&frame(80, 24, 42, 0.0, &knobs())));
    }

    #[test]
    fn cutup_seed42_t6() {
        insta::assert_snapshot!("cutup_80x24_t6", text(&frame(80, 24, 42, 6.0, &knobs())));
    }

    #[test]
    fn deterministic_and_seed_sensitive() {
        let k = knobs();
        assert_eq!(text(&frame(90, 30, 42, 0.0, &k)), text(&frame(90, 30, 42, 0.0, &k)));
        assert_ne!(text(&frame(90, 30, 42, 0.0, &k)), text(&frame(90, 30, 7, 0.0, &k)));
    }

    #[test]
    fn time_slides_the_impression() {
        let k = knobs();
        assert_ne!(text(&frame(90, 30, 42, 0.0, &k)), text(&frame(90, 30, 42, 5.0, &k)));
    }

    #[test]
    fn shift_scatters_the_fragments() {
        let mut k = knobs();
        let a = text(&frame(90, 30, 42, 0.0, &k));
        k[3] = 18.0;
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
        eprintln!("cutup frame_cost 200x60: avg {:.3} ms, worst {:.3} ms", avg, worst);
        if !cfg!(debug_assertions) {
            assert!(avg < 6.0, "avg frame {:.3} ms", avg);
        }
    }
}
