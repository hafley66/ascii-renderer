//! aurora2: folded curtains of northern light over a black pine ridge. Each
//! curtain is a sheet marched in plan view; where it folds, the fold caustics.
use crate::_0_profile::measure_layer;
use crate::color::{hsl_to_rgb, lerp_color, rgb};
use crate::opts::param_f32;
use crate::pp::{pp_fbm, pp_vnoise};
use crate::registry::{AnimKind, Mode, ModeFrame, Param};
use crate::types::{Cell, Grid};
use crossterm::style::Color;
use rand::RngExt;
use rand::SeedableRng;
use rand::rngs::StdRng;
use rayon::prelude::*;
use std::cell::RefCell;
use std::f32::consts::{PI, TAU};

pub(super) struct Aurora2;
pub(super) static MODE: Aurora2 = Aurora2;

const NAME: &str = "aurora2";
const KNOBS: usize = 9;
const HELP: &str = "aurora2: folded light curtains over a black pine ridge, lit snow below [ribbons] [width] [drift] [horizon] [stars] [spread] [pines] [fold] [gain]";

const PARAMS: &[Param] = &[
    param!("RIBBONS", "curtain count", 1.0, 16.0, 5.0, 1.0),
    param!("RWIDTH", "curtain width in columns", 2.0, 30.0, 7.0, 1.0),
    param!("DRIFT", "flow drift speed", 0.0, 4.0, 1.0, 0.1),
    param!("HORIZON", "horizon height fraction", 0.3, 0.95, 0.72, 0.02),
    param!("STARS", "star density", 0.0, 3.0, 1.0, 0.1),
    param!("SPREAD", "hue reach green to violet", 0.0, 2.0, 1.0, 0.05),
    param!("PINES", "treeline density", 0.0, 3.0, 1.0, 0.1),
    param!("FOLD", "ray contrast", 0.0, 2.0, 1.0, 0.05),
    param!("GAIN", "curtain brightness", 0.2, 2.5, 1.0, 0.05),
];

/// Sheet body, smooth to dense. Ray columns swap to the vertical ramp so a
/// streak reads as a streak instead of another blob of hash.
const BODY: [char; 9] = [' ', '.', ':', '-', '=', '+', '*', '#', '@'];
const RAY: [char; 6] = [' ', '.', ':', '|', '!', '#'];
const SNOW: [char; 6] = [' ', '.', ':', '-', '=', '+'];
const PINE: char = '#';
const RIDGE: char = '%';
const REF_COLS: f32 = 80.0;
const PARALLEL_MIN_CELLS: usize = 20_480;

struct Knobs {
    ribbons: f32,
    width: f32,
    drift: f32,
    horizon: f32,
    stars: f32,
    spread: f32,
    pines: f32,
    fold: f32,
    gain: f32,
}

impl Knobs {
    fn from_values(p: &[f32; KNOBS]) -> Self {
        Knobs {
            ribbons: p[0],
            width: p[1],
            drift: p[2],
            horizon: p[3],
            stars: p[4],
            spread: p[5],
            pines: p[6],
            fold: p[7],
            gain: p[8],
        }
    }
}

impl Mode for Aurora2 {
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
        let k = Knobs::from_values(&p);
        draw(frame.grid, frame.width, frame.height, frame.seed, frame.palette, frame.time, &k);
    }
}

/// Stream splitter: every roll is keyed to (seed, layer, index) so one curtain
/// or star never shifts its neighbors when a knob moves.
fn side_rng(seed: u64, layer: u64, index: u64) -> StdRng {
    let mut h = seed ^ layer.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    h ^= index.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h ^= h >> 30;
    h = h.wrapping_mul(0xD6E8_FEB8_6659_FD93);
    h ^= h >> 27;
    StdRng::seed_from_u64(h.wrapping_mul(0x94D0_49BB_1331_11EB))
}

fn hash01(x: i64, y: i64, k: u64, seed: u64) -> f32 {
    let mut h = (x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (y as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9)
        ^ k.wrapping_mul(0x94D0_49BB_1331_11EB)
        ^ seed;
    h ^= h >> 31;
    h = h.wrapping_mul(0xD6E8_FEB8_6659_FD93);
    h ^= h >> 32;
    (h & 0xFF_FFFF) as f32 / 16_777_216.0
}

/// Reinhard shoulder: hems pin near the top of the ramp, crowns keep their
/// gradient, and nothing ever reaches full white.
#[inline]
fn tone(a: f32, gain: f32) -> f32 {
    let x = (a * gain * 2.4).max(0.0);
    x / (1.0 + x)
}

#[inline]
fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

const BANDS: usize = 24;
const LEVELS: usize = 16;

/// Hem green to mid pink to crown violet, mixed in rgb so the walk never
/// detours through yellow the way a hue interpolation does.
fn band_rgb(band: f32, m: f32) -> Color {
    let lum = (0.13 + 0.50 * m as f64).min(0.70);
    let sat = (0.93 - 0.26 * m as f64).max(0.52);
    let green = hsl_to_rgb(133.0, sat, lum);
    let pink = hsl_to_rgb(318.0, sat * 0.93, lum * 0.98);
    let violet = hsl_to_rgb(268.0, sat * 0.88, lum * 0.90);
    if band < 0.45 {
        green
    } else if band < 0.74 {
        lerp_color(green, pink, smoothstep((band - 0.45) / 0.29))
    } else {
        lerp_color(pink, violet, smoothstep((band - 0.74) / 0.26))
    }
}

/// One frame of quantised curtain color: LEVELS brightness steps per BANDS
/// heights, so the per-cell paint is two index lookups and no hsl math.
fn band_table() -> Vec<Color> {
    let mut out = Vec::with_capacity(BANDS * LEVELS);
    for bi in 0..BANDS {
        let band = bi as f32 / (BANDS - 1) as f32;
        for li in 0..LEVELS {
            out.push(band_rgb(band, li as f32 / (LEVELS - 1) as f32));
        }
    }
    out
}

#[inline]
fn band_index(band: f32, m: f32) -> usize {
    let bi = (band.clamp(0.0, 1.0) * (BANDS - 1) as f32).round() as usize;
    let li = (m.clamp(0.0, 1.0) * (LEVELS - 1) as f32).round() as usize;
    bi * LEVELS + li
}

struct Curtain {
    x0: f32,
    span: f32,
    hem: f32,
    depth: f32,
    amp: f32,
    fold: [f32; 3],
    phase: [f32; 3],
    rayf: f32,
    speed: f32,
    power: f32,
    lift: f32,
    breath: f32,
    key: u64,
}

fn roll_curtains(seed: u64, n: usize, width_knob: f32) -> Vec<Curtain> {
    let mut out = Vec::with_capacity(n);
    let slot = 1.0 / n as f32;
    let wide = (width_knob / 7.0).clamp(0.3, 4.0);
    for i in 0..n {
        let mut r = side_rng(seed, 1, i as u64);
        let span = (r.random_range(0.34..0.68f32) * wide).clamp(0.08, 1.2);
        let centre = (i as f32 + 0.5) * slot + r.random_range(-0.30..0.30f32) * slot;
        let hem = r.random_range(0.62..0.95f32);
        out.push(Curtain {
            x0: centre - span * 0.5,
            span,
            hem,
            depth: r.random_range(0.42..0.78f32) * hem,
            amp: r.random_range(0.060..0.145f32),
            fold: [
                r.random_range(0.8..1.6f32),
                r.random_range(2.2..3.4f32),
                r.random_range(5.0..7.5f32),
            ],
            phase: [
                r.random::<f32>() * TAU,
                r.random::<f32>() * TAU,
                r.random::<f32>() * TAU,
            ],
            rayf: r.random_range(22.0..38.0f32),
            speed: r.random_range(0.45..1.35f32),
            power: r.random_range(0.62..1.15f32),
            lift: r.random_range(-0.05..0.05f32),
            breath: r.random::<f32>() * TAU,
            key: seed ^ (i as u64).wrapping_mul(0x5EED_1234_ABCD_0001),
        });
    }
    out
}

/// One screen column of one curtain after the sheet march.
#[derive(Clone, Copy, Default)]
struct Col {
    dens: f32,
    hem: f32,
    depth: f32,
    streak: f32,
}

/// Marching the sheet in plan view and binning it by screen column is what
/// makes a fold bright: two arms of the fold land in the same column and add.
fn march(c: &Curtain, w: usize, sky: usize, t: f32, drift: f32, cols: &mut [Col]) -> (usize, usize) {
    for v in cols.iter_mut() {
        *v = Col::default();
    }
    let reach = (c.span * w as f32).max(2.0);
    let steps = ((reach * 3.0) as usize).clamp(96, 24_000);
    let ph = t * drift * c.speed;
    let amp = c.amp * reach;
    let rayf = c.rayf * (reach / (REF_COLS * 0.5)).max(0.5);
    let (mut lo, mut hi) = (w, 0usize);
    for i in 0..steps {
        let s = i as f32 / (steps - 1).max(1) as f32;
        let a1 = (c.fold[0] * s * TAU + c.phase[0] + ph * 0.29).sin();
        let a2 = (c.fold[1] * s * TAU + c.phase[1] + ph * 0.53).sin();
        let a3 = (c.fold[2] * s * TAU + c.phase[2] + ph * 0.87).sin();
        let z = a1 + 0.55 * a2 + 0.26 * a3;
        let screen = (c.x0 + s * c.span) * w as f32 + z * amp;
        let sag = (c.fold[0] * 0.34 * s * TAU + c.phase[2]).sin();
        let hem = (c.hem + 0.055 * sag + 0.010 * z) * sky as f32;
        let depth = (c.depth * (1.0 - 0.14 * z)).max(0.06) * sky as f32;
        let ray = pp_vnoise(s * rayf, c.phase[0] * 3.1 + ph * 0.09, c.key ^ 0x5A5);
        let glow = pp_fbm(s * 3.4 + 0.5, c.phase[1] + ph * 0.07, c.key ^ 0x9E1);
        let bright = (PI * s).sin().powf(0.45) * (0.30 + 1.5 * glow.powf(1.4));
        let xf = screen - 0.5;
        let base = xf.floor();
        let frac = xf - base;
        let i0 = base as i64;
        for (dx, wg) in [(0i64, 1.0 - frac), (1, frac)] {
            let xx = i0 + dx;
            if xx < 0 || xx >= w as i64 || wg <= 0.0 {
                continue;
            }
            let x = xx as usize;
            let col = &mut cols[x];
            col.dens += wg * bright;
            col.hem += wg * hem;
            col.depth += wg * depth;
            col.streak += wg * ray;
            lo = lo.min(x);
            hi = hi.max(x);
        }
    }
    if lo > hi {
        return (0, 0);
    }
    let norm = reach / steps as f32;
    for col in cols[lo..=hi].iter_mut() {
        let d = col.dens;
        if d <= 1.0e-6 {
            *col = Col::default();
            continue;
        }
        col.hem /= d;
        col.depth /= d;
        col.streak = smoothstep((col.streak / d - 0.58) / 0.17);
        col.dens = d * norm;
    }
    let mut prev = 0.0f32;
    for x in lo..=hi {
        let here = cols[x].dens;
        let next = if x < hi { cols[x + 1].dens } else { 0.0 };
        cols[x].dens = prev * 0.25 + here * 0.5 + next * 0.25;
        prev = here;
    }
    (lo, hi)
}

/// Accumulated light at one sky cell: alpha, height band, streakiness.
#[derive(Clone, Copy, Default)]
struct Lit {
    a: f32,
    band: f32,
    streak: f32,
}

thread_local! {
    static FIELD: RefCell<Vec<Lit>> = const { RefCell::new(Vec::new()) };
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
fn each_grid_row<F>(grid: &mut Grid, w: usize, y0: usize, y1: usize, row: F)
where
    F: Fn((usize, &mut Vec<Cell>)) + Sync + Send,
{
    if y1 <= y0 {
        return;
    }
    let slice = &mut grid[y0..y1];
    if w * (y1 - y0) >= PARALLEL_MIN_CELLS {
        slice.par_iter_mut().enumerate().with_min_len(8).for_each(|(i, r)| row((y0 + i, r)));
    } else {
        slice.iter_mut().enumerate().for_each(|(i, r)| row((y0 + i, r)));
    }
}

/// Every curtain's columns, resolved once, then one row pass over the sky.
fn accumulate(lit: &mut [Lit], w: usize, sky: usize, t: f32, k: &Knobs, curtains: &[Curtain]) {
    for v in lit.iter_mut() {
        *v = Lit::default();
    }
    let fold = k.fold.clamp(0.0, 2.0);
    let spread = k.spread.max(0.0);
    let mut scratch = vec![Col::default(); w];
    let mut plans: Vec<(usize, Vec<Col>, f32, f32)> = Vec::with_capacity(curtains.len());
    for c in curtains {
        let (lo, hi) = march(c, w, sky, t, k.drift, &mut scratch);
        if lo >= hi {
            continue;
        }
        let breath = if t > 0.0 {
            1.0 + 0.22 * (t * (0.45 + c.speed * 0.5) + c.breath).sin()
        } else {
            1.0
        };
        plans.push((lo, scratch[lo..=hi].to_vec(), c.power * breath, c.lift));
    }
    each_row(lit, w, sky, |(y, row)| {
        let yc = y as f32 + 0.5;
        for (lo, cols, power, lift) in plans.iter() {
            for (i, col) in cols.iter().enumerate() {
                if col.dens < 0.02 {
                    continue;
                }
                let u = (col.hem - yc) / col.depth.max(1.0);
                if !(-0.14..1.06).contains(&u) {
                    continue;
                }
                let env = if u < 0.0 {
                    let s = u / 0.055;
                    (-(s * s)).exp()
                } else {
                    let rim = -(u / 0.115).powi(2);
                    rim.exp() * 1.25 + (1.0 - u).powf(1.4) * 0.90
                };
                let mask = smoothstep((u - 0.06) / 0.20);
                let gap = 1.0 - fold * 0.90 * mask * (1.0 - col.streak);
                let cut = gap.max(0.06) * (1.0 + fold * 0.9 * mask * col.streak);
                let a = col.dens.powf(1.5) * env * cut * power;
                if a <= 0.012 {
                    continue;
                }
                let cell = &mut row[lo + i];
                cell.a += a;
                cell.band += a * ((u + lift).max(0.0) * spread).min(1.0);
                cell.streak += a * col.streak * mask;
            }
        }
    });
}

/// Navy where the curtains sit, near black overhead.
fn sky_bg(palette: &[Color; 5], v: f32) -> Color {
    let zenith = lerp_color(rgb(1, 2, 7), palette[0], 0.10);
    let horizon = lerp_color(rgb(7, 13, 34), palette[0], 0.26);
    lerp_color(zenith, horizon, v.clamp(0.0, 1.0).powf(2.0))
}

fn paint_sky(grid: &mut Grid, w: usize, h: usize, sky: usize, palette: &[Color; 5]) {
    let rows = grid.len().min(h);
    each_grid_row(grid, w, 0, rows, |(y, row)| {
        let v = if sky > 0 { (y as f32 / sky as f32).min(1.0) } else { 1.0 };
        let bg = sky_bg(palette, v);
        for cell in row.iter_mut().take(w) {
            *cell = Cell::with_bg(' ', Color::Reset, bg);
        }
    });
}

fn paint_stars(grid: &mut Grid, w: usize, sky: usize, seed: u64, t: f32, k: &Knobs) {
    if sky == 0 {
        return;
    }
    let want = ((w * sky) as f32 * 0.019 * k.stars.max(0.0)) as usize;
    let n = want.min(6000);
    let cold = rgb(150, 162, 198);
    for i in 0..n {
        let mut r = side_rng(seed, 2, i as u64);
        let x = r.random_range(0..w.max(1));
        let ry = r.random::<f32>().powf(1.5);
        let y = ((ry * sky as f32) as usize).min(sky.saturating_sub(1));
        let base = r.random_range(0.20..1.0f32);
        let phase = r.random::<f32>() * TAU;
        let rate = r.random_range(0.5..2.8f32);
        let tint = r.random_range(-24.0..30.0f32);
        let swing = if t > 0.0 { (t * rate + phase).sin() } else { phase.sin() };
        let b = base * (0.58 + 0.42 * swing);
        if b < 0.18 {
            continue;
        }
        let ch = if b < 0.34 {
            '·'
        } else if b < 0.56 {
            '.'
        } else if b < 0.80 {
            '+'
        } else {
            '*'
        };
        let warm = lerp_color(cold, hsl_to_rgb((205.0 + tint) as f64, 0.35, 0.86), 0.5);
        let bg = grid[y][x].bg;
        let fg = lerp_color(bg, lerp_color(warm, rgb(228, 236, 255), b * 0.5), 0.28 + b * 0.66);
        grid[y][x] = Cell::with_bg(ch, fg, bg);
    }
}

/// A thin crescent: the lune between the disc and a shadow disc pushed aside.
fn paint_moon(grid: &mut Grid, w: usize, sky: usize, seed: u64, palette: &[Color; 5]) {
    if sky < 6 || w < 12 {
        return;
    }
    let mut r = side_rng(seed, 9, 0);
    let ry = (sky as f32 * 0.075).clamp(1.1, 2.4);
    let rx = ry * 2.05;
    let mx = r.random_range(0.12..0.88f32) * w as f32;
    let my = r.random_range(0.10..0.34f32) * sky as f32;
    let dir: f32 = if r.random::<f32>() < 0.5 { -1.0 } else { 1.0 };
    let off = dir * rx * r.random_range(0.72..0.95f32);
    let face = lerp_color(rgb(228, 234, 248), palette[4], 0.16);
    let halo = lerp_color(rgb(24, 34, 66), palette[0], 0.25);
    let y0 = ((my - ry * 2.2).floor().max(0.0)) as usize;
    let y1 = ((my + ry * 2.2).ceil() as usize).min(sky.min(grid.len()));
    let x0 = ((mx - rx * 2.2).floor().max(0.0)) as usize;
    let x1 = ((mx + rx * 2.2).ceil() as usize).min(w);
    for y in y0..y1 {
        let dy = (y as f32 + 0.5 - my) / ry;
        for x in x0..x1 {
            let dx = (x as f32 + 0.5 - mx) / rx;
            let d = (dx * dx + dy * dy).sqrt();
            let sdx = (x as f32 + 0.5 - (mx + off)) / rx;
            let ds = (sdx * sdx + dy * dy).sqrt();
            if d <= 1.0 && ds > 1.0 {
                let edge = (1.0 - d).min(ds - 1.0);
                let ch = if edge > 0.22 { '@' } else { '#' };
                let bg = lerp_color(grid[y][x].bg, halo, 0.55);
                grid[y][x] = Cell::with_bg(ch, face, bg);
            } else if d < 1.9 {
                let glow = (1.0 - (d - 1.0) / 0.9).clamp(0.0, 1.0);
                let bg = lerp_color(grid[y][x].bg, halo, glow * 0.45);
                let cell = &mut grid[y][x];
                *cell = Cell::with_bg(cell.ch, cell.fg, bg);
            }
        }
    }
}

fn paint_curtains(grid: &mut Grid, w: usize, sky: usize, lit: &[Lit], k: &Knobs) {
    let gain = k.gain.max(0.05);
    let table = band_table();
    let rows = sky.min(grid.len());
    each_grid_row(grid, w, 0, rows, |(y, row)| {
        let base = y * w;
        for (x, cell) in row.iter_mut().enumerate().take(w) {
            let l = &lit[base + x];
            let m = tone(l.a, gain);
            if m < 0.105 {
                continue;
            }
            let band = l.band / l.a;
            let streak = l.streak / l.a;
            let ch = if streak > 0.46 {
                RAY[((m * 5.0).round() as usize).clamp(1, 5)]
            } else {
                BODY[((m * 8.0).round() as usize).clamp(1, 8)]
            };
            let glow = table[band_index(band, m)];
            let bg = lerp_color(cell.bg, table[band_index(band, 0.08)], (m * 0.62).min(0.58));
            let fg = lerp_color(cell.bg, glow, (0.34 + 0.66 * m).min(1.0));
            *cell = Cell::with_bg(ch, fg, bg);
        }
    });
}

/// Snow does not mirror the sky, it pools the light falling on it. Average each
/// column over the lower sky, then blur sideways so the foreground reads smooth.
fn column_glow(lit: &[Lit], w: usize, sky: usize, gain: f32) -> (Vec<f32>, Vec<f32>) {
    let mut a = vec![0.0f32; w];
    let mut hue = vec![0.0f32; w];
    if sky == 0 {
        return (a, hue);
    }
    let lo = sky.saturating_sub((sky as f32 * 0.55).ceil() as usize).min(sky - 1);
    let rows = (sky - lo) as f32;
    for x in 0..w {
        let mut sum = 0.0f32;
        let mut hsum = 0.0f32;
        for y in lo..sky {
            let l = &lit[y * w + x];
            if l.a <= 0.0 {
                continue;
            }
            let m = tone(l.a, gain);
            sum += m;
            hsum += m * (l.band / l.a);
        }
        a[x] = sum / rows;
        hue[x] = if sum > 0.001 { hsum / sum } else { 0.0 };
    }
    let span = ((w as f32 / REF_COLS * 3.0).round() as usize).max(2);
    let mut blur = vec![0.0f32; w];
    for x in 0..w {
        let x0 = x.saturating_sub(span);
        let x1 = (x + span).min(w - 1);
        let mut s = 0.0;
        for v in a.iter().take(x1 + 1).skip(x0) {
            s += *v;
        }
        blur[x] = s / (x1 - x0 + 1) as f32;
    }
    (blur, hue)
}

fn paint_ground(grid: &mut Grid, w: usize, h: usize, sky: usize, seed: u64, lit: &[Lit], gain: f32, palette: &[Color; 5]) {
    if sky >= h {
        return;
    }
    let deep = lerp_color(rgb(3, 5, 12), palette[0], 0.14);
    let ground_rows = (h - sky) as f32;
    let table = band_table();
    let (col_a, col_h) = column_glow(lit, w, sky, gain);
    let rows = h.min(grid.len());
    each_grid_row(grid, w, sky, rows, |(y, row)| {
        let d = (y - sky) as f32 / ground_rows.max(1.0);
        let fade = (1.0 - d * 0.84).powf(1.15);
        for (x, cell) in row.iter_mut().enumerate().take(w) {
            let xn = (x as f32 + 0.5) * REF_COLS / w as f32;
            let wob = ((pp_vnoise(xn * 0.42, y as f32 * 0.9, seed ^ 0x7A7) - 0.5) * 3.4).round();
            let sx = (x as i32 + wob as i32).clamp(0, w as i32 - 1) as usize;
            let n = hash01(x as i64, y as i64, 91, seed);
            let band = col_h[sx];
            let m = (col_a[sx] * 1.5 * fade * (0.80 + 0.40 * n)).min(1.0);
            let lit_bg = lerp_color(deep, table[band_index(band, 0.12)], (m * 0.95).min(0.72));
            let (ch, fg) = if m > 0.10 {
                let li = ((m * 5.0).round() as usize).clamp(1, 5);
                (SNOW[li], table[band_index(band, m * 0.72)])
            } else if n > 0.994 {
                ('.', lerp_color(deep, rgb(120, 140, 175), 0.5))
            } else {
                (' ', deep)
            };
            *cell = Cell::with_bg(ch, fg, lit_bg);
        }
    });
}

/// The far ridge, then the pines: both silhouettes, one glyph each so the
/// shape carries in plain text and the color stays flat black.
fn paint_ridge(grid: &mut Grid, w: usize, sky: usize, seed: u64, palette: &[Color; 5]) {
    let floor = sky as i32 - 1;
    let lift = (sky as f32 * 0.12).max(1.0);
    let key = seed ^ 0x0D1D_6E5A_1100_4321;
    let ink = lerp_color(rgb(0, 0, 0), rgb(46, 58, 88), 0.55);
    let back = lerp_color(lerp_color(rgb(0, 0, 0), rgb(20, 27, 48), 0.55), palette[0], 0.12);
    for x in 0..w {
        let xn = (x as f32 + 0.5) * REF_COLS / w as f32;
        let n = pp_vnoise(xn * 0.042, 3.7, key) * 0.76 + pp_vnoise(xn * 0.10, 9.1, key ^ 0x31) * 0.24;
        let top = (floor - (lift * (0.42 + 1.10 * n)).round() as i32).clamp(0, floor);
        for y in top..=floor {
            if y < 0 || y as usize >= grid.len() {
                continue;
            }
            grid[y as usize][x] = Cell::with_bg(RIDGE, ink, back);
        }
    }
}

fn paint_pines(grid: &mut Grid, w: usize, sky: usize, seed: u64, k: &Knobs, palette: &[Color; 5]) {
    let floor = sky as i32 - 1;
    if floor < 1 {
        return;
    }
    let dens = k.pines.clamp(0.0, 4.0);
    if dens <= 0.0 {
        return;
    }
    let tall = (sky as f32 * 0.11).clamp(2.0, 12.0);
    let skirt = (sky as f32 * 0.05).max(1.0);
    let mut top: Vec<i32> = (0..w)
        .map(|x| {
            let xn = (x as f32 + 0.5) * REF_COLS / w as f32;
            let n = pp_vnoise(xn * 0.055, 8.3, seed ^ 0x51DE_1A17_0000_0B0E);
            floor - (skirt * (0.8 + 1.3 * n)).round() as i32
        })
        .collect();
    let count = ((w as f32 / 3.0) * dens) as usize;
    for i in 0..count.min(12_000) {
        let mut r = side_rng(seed, 3, i as u64);
        let cx = r.random_range(-4.0..(w as f32 + 4.0));
        let ht = tall * r.random_range(0.40..2.00f32);
        let half = (ht * r.random_range(0.55..1.05f32)).max(1.0);
        let base = floor - r.random_range(0..2i32);
        let x0 = ((cx - half).floor().max(0.0)) as usize;
        let x1 = ((cx + half).ceil() as usize).min(w.saturating_sub(1));
        for (x, slot) in top.iter_mut().enumerate().take(x1 + 1).skip(x0) {
            let dx = ((x as f32 + 0.5 - cx) / half).abs();
            if dx > 1.0 {
                continue;
            }
            let hh = ht * (1.0 - dx.powf(1.3));
            let peak = base - hh.round() as i32;
            if peak < *slot {
                *slot = peak.max(0);
            }
        }
    }
    let ink = lerp_color(rgb(0, 0, 0), palette[0], 0.05);
    let backlit = lerp_color(ink, rgb(96, 124, 158), 0.34);
    for x in 0..w {
        let t = top[x].min(floor);
        let left = top[x.saturating_sub(1)];
        let right = top[(x + 1).min(w - 1)];
        let peak = t < left && t < right;
        for y in t..=floor {
            if y < 0 || y as usize >= grid.len() {
                continue;
            }
            let (ch, fg) = if y == t && peak { ('^', backlit) } else { (PINE, ink) };
            grid[y as usize][x] = Cell::with_bg(ch, fg, ink);
        }
    }
}

fn draw(grid: &mut Grid, w: usize, h: usize, seed: u64, palette: &[Color; 5], t: f32, k: &Knobs) {
    if w == 0 || h == 0 {
        return;
    }
    let sky = ((h as f32 * k.horizon.clamp(0.15, 0.98)).round() as usize).clamp(1, h.saturating_sub(1).max(1));
    measure_layer(NAME, "sky", || {
        paint_sky(grid, w, h, sky, palette);
        paint_stars(grid, w, sky, seed, t, k);
        paint_moon(grid, w, sky, seed, palette);
    });
    let n = (k.ribbons.round() as i32).clamp(1, 24) as usize;
    let curtains = roll_curtains(seed, n, k.width);
    FIELD.with(|slot| {
        let mut field = slot.borrow_mut();
        if field.len() < w * sky {
            field.resize(w * sky, Lit::default());
        }
        let lit = &mut field[..w * sky];
        measure_layer(NAME, "field", || accumulate(lit, w, sky, t, k, &curtains));
        measure_layer(NAME, "curtains", || paint_curtains(grid, w, sky, lit, k));
        let gain = k.gain.max(0.05);
        measure_layer(NAME, "ground", || paint_ground(grid, w, h, sky, seed, lit, gain, palette));
    });
    measure_layer(NAME, "pines", || {
        paint_ridge(grid, w, sky, seed, palette);
        paint_pines(grid, w, sky, seed, k, palette);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::grid_to_plain;
    use rand::{SeedableRng, rngs::StdRng};

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
    fn aurora2_80x24() {
        insta::assert_snapshot!("aurora2_80x24", text(&frame(80, 24, 42, 0.0, &knobs())));
    }

    #[test]
    fn aurora2_120x36() {
        insta::assert_snapshot!("aurora2_120x36", text(&frame(120, 36, 1701, 0.0, &knobs())));
    }

    #[test]
    fn aurora2_80x24_t6() {
        insta::assert_snapshot!("aurora2_80x24_t6", text(&frame(80, 24, 42, 6.0, &knobs())));
    }

    #[test]
    fn deterministic_and_seed_sensitive() {
        let k = knobs();
        assert_eq!(text(&frame(90, 30, 42, 0.0, &k)), text(&frame(90, 30, 42, 0.0, &k)));
        assert_ne!(text(&frame(90, 30, 42, 0.0, &k)), text(&frame(90, 30, 7, 0.0, &k)));
    }

    #[test]
    fn t_drifts_the_curtains() {
        let k = knobs();
        assert_ne!(text(&frame(90, 30, 42, 0.0, &k)), text(&frame(90, 30, 42, 3.0, &k)));
        assert_ne!(text(&frame(90, 30, 42, 3.0, &k)), text(&frame(90, 30, 42, 7.0, &k)));
    }

    #[test]
    fn treeline_stays_still_while_the_sky_moves() {
        let k = knobs();
        let a = frame(90, 30, 42, 0.0, &k);
        let b = frame(90, 30, 42, 4.0, &k);
        let sky = (30.0f32 * 0.72).round() as usize;
        let row = |g: &Grid, y: usize| g[y].iter().map(|c| c.ch).collect::<String>();
        assert_eq!(row(&a, sky - 1), row(&b, sky - 1), "the ridge crest drifted");
        let mut moved = 0usize;
        for y in 0..sky - 2 {
            if row(&a, y) != row(&b, y) {
                moved += 1;
            }
        }
        assert!(moved > 4, "only {moved} sky rows changed over 4 seconds");
    }

    #[test]
    fn hems_sit_below_the_crowns() {
        let k = knobs();
        let g = frame(120, 36, 42, 0.0, &k);
        let sky = (36.0f32 * 0.72).round() as usize;
        let ink = |y: usize| g[y].iter().filter(|c| c.ch != ' ' && c.ch != '·').count();
        let upper: usize = (1..sky / 3).map(ink).sum();
        let lower: usize = (sky / 2..sky.saturating_sub(4)).map(ink).sum();
        assert!(lower > upper, "hem band {lower} is not denser than the crown {upper}");
    }

    #[test]
    fn tiny_grids_terminate() {
        let k = knobs();
        for (w, h) in [(1usize, 1usize), (6, 2), (12, 3), (3, 20)] {
            frame(w, h, 5, 2.0, &k);
        }
    }

    #[test]
    fn every_glyph_is_single_width() {
        use unicode_width::UnicodeWidthChar;
        let k = knobs();
        for step in 0..12 {
            let g = frame(120, 40, 42, step as f32 * 0.7, &k);
            for row in &g {
                for c in row {
                    assert_eq!(c.ch.width().unwrap_or(0), 1, "glyph {:?} is not width 1", c.ch);
                }
            }
        }
    }

    #[test]
    fn frame_cost() {
        let (w, h) = (200usize, 60usize);
        let k = knobs();
        frame(w, h, 42, 0.0, &k);
        let start = std::time::Instant::now();
        for f in 0..200 {
            frame(w, h, 42, f as f32 * 0.05, &k);
        }
        let avg = start.elapsed().as_secs_f64() * 1000.0 / 200.0;
        eprintln!("aurora2 frame_cost 200x60: avg {:.3} ms", avg);
        if !cfg!(debug_assertions) {
            assert!(avg < 6.0, "avg frame {:.3} ms", avg);
        }
    }
}
