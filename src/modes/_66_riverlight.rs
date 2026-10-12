//! Riverlight: paper lanterns drifting down a night river. Each one bobs on
//! the current with a warm halo and a shimmering reflection below.
use crate::_0_profile::measure_layer;
use crate::color::{darken, hsl_to_rgb, lighten};
use crate::opts::param_f32;
use crate::registry::{AnimKind, Mode, ModeFrame, Param};
use crate::types::{Cell, Grid};
use crossterm::style::Color;
use rayon::prelude::*;
use std::f32::consts::{PI, TAU};

pub(super) struct Riverlight;
pub(super) static MODE: Riverlight = Riverlight;

const NAME: &str = "riverlight";
const KNOBS: usize = 9;
const HELP: &str = "lanterns: paper lanterns drifting down a night river [count] [speed] [glow] [wobble] [hue] [spread] [reflect] [stars] [size]";

/// River surface marks and the bank vignette.
const RIVER: [char; 3] = ['~', '-', '.'];
/// Star field glyphs, dim to bright.
const STARS: [char; 3] = ['.', '+', '*'];
/// Lantern body, rim to hot core.
const BODY: [char; 3] = ['o', 'O', '@'];
/// Reflection shimmer.
const SHIMMER: [char; 4] = [':', '.', ' ', '\''];

const L_WATER: u64 = 0x21;
const L_STAR: u64 = 0x22;
const L_LANTERN: u64 = 0x23;

const PARALLEL_MIN_CELLS: usize = 20_480;

const PARAMS: &[Param] = &[
    param!("COUNT", "lanterns on the river", 1.0, 40.0, 14.0, 1.0),
    param!("SPEED", "drift cells/s", 0.0, 3.0, 0.6, 0.05),
    param!("GLOW", "halo reach", 0.0, 2.0, 0.8, 0.05),
    param!("WOBBLE", "bob amplitude rows", 0.0, 2.0, 0.8, 0.05),
    param!("HUE", "base hue deg", 0.0, 360.0, 32.0, 1.0),
    param!("SPREAD", "hue spread deg", 0.0, 180.0, 55.0, 1.0),
    param!("REFLECT", "reflection length", 0.0, 1.5, 0.9, 0.05),
    param!("STARS", "star density", 0.0, 1.0, 0.5, 0.05),
    param!("SIZE", "lantern scale", 0.5, 2.5, 1.0, 0.05),
];

impl Mode for Riverlight {
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

/// One lantern's resolved lane, geometry, and color.
struct Light {
    x: f32,
    y: f32,
    r: i32,
    a: i32,
    hue: f64,
    pulse: f32,
    glow: f32,
    refl: i32,
    phase: f32,
}

fn resolve_lights(seed: u64, w: usize, h: usize, t: f32, p: &[f32; KNOBS]) -> Vec<Light> {
    let count = p[0].round().max(1.0) as u64;
    (0..count)
        .map(|i| {
            let h0 = hash(seed, L_LANTERN, i, 0);
            let speed = p[1] * (0.6 + 0.8 * unit(h0 ^ 1));
            let lane = 0.1 + 0.8 * unit(h0 ^ 2);
            let x0 = unit(h0 ^ 3);
            let span = w as f32 + 10.0;
            let x = (x0 + t * speed / w.max(1) as f32).fract() * span - 5.0;
            let phase = unit(h0 ^ 4) * TAU;
            let y = lane * h as f32
                + p[3] * (0.5 + unit(h0 ^ 5)) * (t * (0.5 + 0.7 * unit(h0 ^ 6)) + phase).sin();
            let s = p[8] * (0.65 + 0.7 * unit(h0 ^ 7));
            let r = (1.7 * s).round().max(1.0) as i32;
            let a = (r as f32 * 2.2).round() as i32;
            let hue = (p[4] + p[5] * (unit(h0 ^ 8) - 0.5) * 2.0).rem_euclid(360.0);
            let pulse = 0.82 + 0.18 * (t * (1.2 + 0.6 * unit(h0 ^ 9)) + phase).sin();
            Light {
                x,
                y,
                r,
                a,
                hue: hue as f64,
                pulse,
                glow: p[2],
                refl: ((p[6] * 7.0 * s) as i32).max(0),
                phase,
            }
        })
        .collect()
}

fn draw(frame: &mut ModeFrame<'_>, p: &[f32; KNOBS]) {
    let (w, h) = (frame.width, frame.height);
    if w == 0 || h == 0 {
        return;
    }
    let seed = frame.seed;
    let t = frame.time;
    let lights = resolve_lights(seed, w, h, t, p);
    measure_layer(NAME, "water", || paint_water(frame.grid, w, h, seed, t, p));
    measure_layer(NAME, "stars", || paint_stars(frame.grid, w, h, seed, t, p));
    measure_layer(NAME, "reflections", || {
        paint_reflections(frame.grid, w, h, seed, t, &lights, p)
    });
    measure_layer(NAME, "glow", || paint_glow(frame.grid, w, h, &lights));
    measure_layer(NAME, "bodies", || paint_bodies(frame.grid, w, h, &lights));
}

fn each_row<F>(grid: &mut Grid, w: usize, row: F)
where
    F: Fn((usize, &mut Vec<Cell>)) + Sync + Send,
{
    let rows = grid.len();
    if rows * w >= PARALLEL_MIN_CELLS {
        grid[..rows]
            .par_iter_mut()
            .enumerate()
            .with_min_len(8)
            .for_each(row);
    } else {
        grid[..rows].iter_mut().enumerate().for_each(row);
    }
}

/// Dark water with faint current streaks drifting with the clock.
fn paint_water(grid: &mut Grid, w: usize, h: usize, seed: u64, t: f32, p: &[f32; KNOBS]) {
    let flow = 0.12 * (0.5 + p[1]);
    let paint = |(y, r): (usize, &mut Vec<Cell>)| {
        let depth = y as f32 / h.max(1) as f32;
        let bank = (1.0 - (2.0 * depth - 1.0).abs()).powf(0.5) * 0.5 + 0.5;
        for (x, cell) in r.iter_mut().enumerate().take(w) {
            let n = unit(hash(
                seed,
                L_WATER,
                ((x as f32 - t * flow * w.max(1) as f32 * 0.3) as i64) as u64,
                y as u64,
            ));
            let l = (0.045 + 0.05 * bank) as f64;
            let fg = hsl_to_rgb(210.0, 0.35, (l + 0.16 * n.clamp(0.0, 1.0) as f64).clamp(0.02, 0.3));
            let bg = hsl_to_rgb(215.0, 0.5, l * 0.7);
            let ch = if n > 0.95 {
                RIVER[1]
            } else if n > 0.88 {
                RIVER[0]
            } else if n > 0.7 {
                RIVER[2]
            } else {
                ' '
            };
            *cell = Cell::with_bg(ch, fg, bg);
        }
    };
    each_row(grid, w, paint);
}

/// Sparse twinkling stars over the upper water, bounded to the top band.
fn paint_stars(grid: &mut Grid, w: usize, h: usize, seed: u64, t: f32, p: &[f32; KNOBS]) {
    let band = (h as f32 * 0.45) as usize;
    let density = p[7] * 0.035;
    for y in 0..band.min(grid.len()) {
        for x in 0..w {
            let hsh = hash(seed, L_STAR, x as u64, y as u64);
            if unit(hsh) >= density {
                continue;
            }
            let tw = 0.5 + 0.5 * (t * (1.5 + 2.0 * unit(hsh ^ 1)) + unit(hsh ^ 2) * TAU).sin();
            if tw < 0.25 {
                continue;
            }
            let lit = (0.25 + 0.5 * tw) as f64;
            let fg = hsl_to_rgb(45.0, 0.25, lit);
            let ch = STARS[((tw * 2.9) as usize).min(2)];
            grid[y][x] = Cell::new(ch, fg);
        }
    }
}

/// Shimmering mirrored flame trails below each lantern.
fn paint_reflections(
    grid: &mut Grid,
    w: usize,
    h: usize,
    seed: u64,
    t: f32,
    lights: &[Light],
    p: &[f32; KNOBS],
) {
    if p[6] <= 0.0 {
        return;
    }
    for (li, l) in lights.iter().enumerate() {
        let start = l.y as i32 + l.r + 1;
        for k in 0..l.refl {
            let y = start + k;
            if y < 0 || y >= h as i32 {
                break;
            }
            let y = y as usize;
            let fade = 1.0 - (k + 1) as f32 / (l.refl + 1) as f32;
            let sway =
                ((t * 1.3 + l.phase + k as f32 * 0.45).sin() * k as f32 * 0.35).round() as i32;
            let flick = (t * 3.0) as u64;
            let hsh = hash(seed, L_LANTERN, li as u64 | 1 << 40, k as u64 + flick);
            if unit(hsh) > 0.75 + 0.2 * fade {
                continue;
            }
            let ch = SHIMMER[(unit(hsh ^ 2) * 3.99) as usize];
            if ch == ' ' {
                continue;
            }
            let lit = (0.45 * fade * l.pulse) as f64;
            let fg = hsl_to_rgb(l.hue, 0.7, lit.max(0.03));
            let x = l.x as i32 + sway;
            if x >= 0 && x < w as i32 {
                grid[y][x as usize] = Cell::new(ch, fg);
            }
        }
    }
}

/// Soft radial halo lightening whatever is already painted.
fn paint_glow(grid: &mut Grid, w: usize, h: usize, lights: &[Light]) {
    for l in lights {
        if l.glow <= 0.0 {
            continue;
        }
        let gr = (2.0 + 4.0 * l.glow) * l.r as f32;
        let ga = gr * 2.2;
        let y0 = (l.y - gr).floor().max(0.0) as usize;
        let y1 = ((l.y + gr).ceil() as usize + 1).min(h);
        let x0 = ((l.x - ga).floor().max(0.0) as usize).min(w.saturating_sub(1));
        let x1 = ((l.x + ga).ceil() as usize + 1).min(w);
        for y in y0..y1.min(grid.len()) {
            for x in x0..x1 {
                let dx = x as f32 - l.x;
                let dy = y as f32 - l.y;
                let d = ((dx / ga).powi(2) + (dy / gr).powi(2)).sqrt();
                let fall = ((1.0 - d).max(0.0) * l.glow * l.pulse * 0.55) as u8;
                if fall == 0 {
                    continue;
                }
                let cell = &mut grid[y][x];
                cell.fg = lighten(cell.fg, fall);
                cell.bg = lighten(cell.bg, fall / 2);
            }
        }
    }
}

/// Paper bodies: rim, hot core, flame dot.
fn paint_bodies(grid: &mut Grid, w: usize, h: usize, lights: &[Light]) {
    for l in lights {
        let cx = l.x.round() as i32;
        let cy = l.y.round() as i32;
        for dy in -l.r..=l.r {
            let y = cy + dy;
            if y < 0 || y >= h as i32 {
                continue;
            }
            let y = y as usize;
            for dx in -l.a..=l.a {
                let x = cx + dx;
                if x < 0 || x >= w as i32 {
                    continue;
                }
                let e = (dx as f32 / l.a as f32).powi(2)
                    + (dy as f32 / l.r as f32).powi(2);
                if e > 1.0 {
                    continue;
                }
                let level = if e > 0.6 {
                    0
                } else if e > 0.25 {
                    1
                } else {
                    2
                };
                let lit = (0.62 - 0.3 * e) * l.pulse;
                let fg = hsl_to_rgb(l.hue, 0.9, (lit as f64).clamp(0.25, 0.8));
                grid[y][x as usize] = Cell::new(BODY[level], fg);
            }
        }
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
    fn riverlight_seed42() {
        insta::assert_snapshot!("riverlight_80x24", text(&frame(80, 24, 42, 0.0, &knobs())));
    }

    #[test]
    fn riverlight_seed42_t6() {
        insta::assert_snapshot!("riverlight_80x24_t6", text(&frame(80, 24, 42, 6.0, &knobs())));
    }

    #[test]
    fn deterministic_and_seed_sensitive() {
        let k = knobs();
        assert_eq!(text(&frame(90, 30, 42, 0.0, &k)), text(&frame(90, 30, 42, 0.0, &k)));
        assert_ne!(text(&frame(90, 30, 42, 0.0, &k)), text(&frame(90, 30, 7, 0.0, &k)));
    }

    #[test]
    fn time_drifts_the_river() {
        let k = knobs();
        assert_ne!(text(&frame(90, 30, 42, 0.0, &k)), text(&frame(90, 30, 42, 4.0, &k)));
    }

    #[test]
    fn count_changes_the_flotilla() {
        let mut k = knobs();
        let a = text(&frame(90, 30, 42, 0.0, &k));
        k[0] = 5.0;
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
        eprintln!("lanterns frame_cost 200x60: avg {:.3} ms, worst {:.3} ms", avg, worst);
        if !cfg!(debug_assertions) {
            assert!(avg < 6.0, "avg frame {:.3} ms", avg);
        }
    }
}
