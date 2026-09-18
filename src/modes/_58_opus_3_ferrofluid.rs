//! opus-3-ferrofluid -- iron filings on a glass plate over drifting bar magnets.
//! @comment-ok: authorship header required by the mode brief
//! Author: Claude Opus 5 (1M context)
//! Written: 2026-09-17

// Contours of the pole set's stream function are the field lines filings lie across.
use crate::_0_profile::measure_layer;
use crate::color::{darken, hsl_to_rgb, lerp_color, lighten};
use crate::opts::param_f32;
use crate::registry::{AnimKind, Mode, ModeFrame, Param};
use crate::types::{Cell, Grid};
use crossterm::style::Color;
use rayon::prelude::*;
use std::cell::RefCell;
use std::f32::consts::{PI, TAU};

pub(super) struct Ferrofluid;
pub(super) static MODE: Ferrofluid = Ferrofluid;

const NAME: &str = "opus-3-ferrofluid";
const KNOBS: usize = 11;
const HELP: &str = "opus-3-ferrofluid: iron filings tracing the field of drifting bar magnets [mags] [lines] [width] [grain] [spin] [drift] [bar] [len] [aspect] [glow] [hue]";

/// A filing lies across its field line, so its glyph is the line's screen slope.
const DIR: [char; 4] = ['-', '\\', '|', '/'];
/// Loose grains between the lines and at the tail of a weak one.
const DUST: [char; 3] = ['.', ',', '\''];
/// Magnet body core and rim.
const BODY: [char; 2] = ['#', '='];
/// Filings heaped at a pole, thin to thick.
const HEAP: [char; 4] = ['*', 'o', 'O', '@'];

const L_MAG: u64 = 0x21;
const L_DUST: u64 = 0x22;
const L_HUE: u64 = 0x23;

/// Field magnitude that counts as a fully lit filing.
const FULL: f32 = 0.42;
const PARALLEL_MIN_CELLS: usize = 20_480;

const PARAMS: &[Param] = &[
    param!("MAGS", "bar magnets", 1.0, 6.0, 3.0, 1.0),
    param!("LINES", "field lines per turn", 4.0, 80.0, 26.0, 1.0),
    param!("WIDTH", "filing line width", 0.1, 2.0, 0.5, 0.05),
    param!("GRAIN", "loose filings", 0.0, 1.0, 0.3, 0.05),
    param!("SPIN", "magnet spin rad/s", 0.0, 1.0, 0.08, 0.02),
    param!("DRIFT", "magnet drift", 0.0, 1.0, 0.12, 0.02),
    param!("BAR", "magnet half width", 0.0, 3.0, 0.9, 0.1),
    param!("LEN", "magnet half length", 2.0, 14.0, 5.0, 0.5),
    param!("ASPECT", "cols per row", 0.25, 4.0, 2.0, 0.25),
    param!("GLOW", "pole heap", 0.0, 2.0, 0.9, 0.1),
    param!("HUE", "pole hue split", 0.0, 180.0, 120.0, 5.0),
];

impl Mode for Ferrofluid {
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

/// One magnetic pole in plate coordinates. `q` is an integer charge so the
/// stream function stays continuous across every branch cut.
#[derive(Clone, Copy)]
struct Pole {
    x: f32,
    y: f32,
    q: f32,
}

/// A bar magnet: its two poles and the body drawn between them.
#[derive(Clone, Copy)]
struct Bar {
    nx: f32,
    ny: f32,
    sx: f32,
    sy: f32,
}

/// Plate coordinates are cell rows by cells columns divided by `aspect`, so a
/// circle in plate space renders round.
struct Look {
    seed: u64,
    aspect: f32,
    lines: f32,
    width: f32,
    grain: f32,
    bar: f32,
    glow: f32,
    poles: Vec<Pole>,
    bars: Vec<Bar>,
    plate: Color,
    grit: Color,
    warm: Color,
    cool: Color,
    iron: Color,
}

impl Look {
    fn new(seed: u64, w: usize, h: usize, palette: &[Color; 5], time: f32, p: &[f32; KNOBS]) -> Self {
        let aspect = p[8].max(0.25);
        let count = p[0].round().max(1.0) as usize;
        let (fw, fh) = (w as f32 / aspect, h as f32);
        let (spin, drift, half_len) = (p[4], p[5], p[7]);
        let mut poles = Vec::with_capacity(count * 2);
        let mut bars = Vec::with_capacity(count);
        for i in 0..count as u64 {
            let swing = TAU * (i as f32 + 0.5) / count as f32 + unit(hash(seed, L_MAG, i, 0)) * 0.9;
            let reach = 0.16 + 0.3 * unit(hash(seed, L_MAG, i, 1));
            let phase = unit(hash(seed, L_MAG, i, 2)) * TAU;
            let rate = unit(hash(seed, L_MAG, i, 3)) * 2.0 - 1.0;
            let half = half_len * (0.6 + 0.6 * unit(hash(seed, L_MAG, i, 4)));
            let q = 1.0 + (hash(seed, L_MAG, i, 5) % 2) as f32;
            let (mut cx, mut cy) = if count == 1 {
                (fw * 0.5, fh * 0.5)
            } else {
                (fw * 0.5 + swing.cos() * fw * reach, fh * 0.5 + swing.sin() * fh * reach)
            };
            cx += (time * drift * rate + phase).sin() * fw * 0.07 * drift;
            cy += (time * drift * rate * 0.8 + phase * 1.7).cos() * fh * 0.07 * drift;
            cx = cx.clamp(fw * 0.12, fw * 0.88);
            cy = cy.clamp(fh * 0.12, fh * 0.88);
            let ang = phase + time * spin * rate;
            let (dx, dy) = (ang.cos() * half, ang.sin() * half);
            poles.push(Pole { x: cx + dx, y: cy + dy, q });
            poles.push(Pole { x: cx - dx, y: cy - dy, q: -q });
            bars.push(Bar { nx: cx + dx, ny: cy + dy, sx: cx - dx, sy: cy - dy });
        }
        let base = unit(hash(seed, L_HUE, 0, 1)) as f64 * 360.0;
        let split = p[10] as f64;
        Look {
            seed,
            aspect,
            lines: p[1].round().max(1.0),
            width: p[2],
            grain: p[3],
            bar: p[6],
            glow: p[9],
            poles,
            bars,
            plate: darken(palette[0], 24),
            grit: darken(palette[2], 52),
            warm: hsl_to_rgb(base, 0.82, 0.58),
            cool: hsl_to_rgb((base + split).rem_euclid(360.0), 0.82, 0.58),
            iron: lighten(palette[2], 18),
        }
    }
}

/// One cell of the resolved field.
#[derive(Clone, Copy, Default)]
struct Sample {
    mag: f32,
    dist: f32,
    slope: u8,
    sign: f32,
    near: f32,
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

fn draw(frame: &mut ModeFrame<'_>, p: &[f32; KNOBS]) {
    let (w, h) = (frame.width, frame.height);
    if w == 0 || h == 0 {
        return;
    }
    let look = Look::new(frame.seed, w, h, frame.palette, frame.time, p);
    SCRATCH.with(|slot| {
        let mut buf = slot.borrow_mut();
        if buf.len() < w * h {
            buf.resize(w * h, Sample::default());
        }
        let field = &mut buf[..w * h];
        measure_layer(NAME, "field", || sample_field(field, w, h, &look));
        measure_layer(NAME, "plate", || paint_plate(frame.grid, field, w, h, &look));
        measure_layer(NAME, "filings", || paint_filings(frame.grid, field, w, h, &look));
        measure_layer(NAME, "magnets", || paint_magnets(frame.grid, w, h, &look));
    });
}

/// Stream function psi = sum q_i * theta_i; its gradient is the field turned a
/// quarter turn, so contour distance is the psi gap over the field magnitude.
fn sample_field(field: &mut [Sample], w: usize, h: usize, look: &Look) {
    let poles = &look.poles[..];
    let lines = look.lines;
    let aspect = look.aspect;
    each_row(field, w, h, |(y, row)| {
        let fy = y as f32 + 0.5;
        for (x, s) in row.iter_mut().enumerate() {
            let fx = (x as f32 + 0.5) / aspect;
            let (mut bx, mut by, mut psi) = (0.0f32, 0.0f32, 0.0f32);
            let (mut warm, mut cool) = (0.0f32, 0.0f32);
            let mut near = f32::MAX;
            for pole in poles {
                let (dx, dy) = (fx - pole.x, fy - pole.y);
                let r2 = (dx * dx + dy * dy).max(0.05);
                bx += pole.q * dx / r2;
                by += pole.q * dy / r2;
                psi += pole.q * dy.atan2(dx);
                let pull = pole.q.abs() / (1.0 + r2.sqrt() * 0.55);
                if pole.q > 0.0 {
                    warm += pull;
                } else {
                    cool += pull;
                }
                near = near.min(r2.sqrt());
            }
            let mag = (bx * bx + by * by).sqrt();
            let step = psi * lines / TAU;
            let gap = (step - step.round()).abs() * TAU / lines;
            let angle = by.atan2(bx * aspect);
            let slope = ((angle / (PI * 0.25)).round() as i32).rem_euclid(4) as u8;
            *s = Sample {
                mag,
                dist: gap / mag.max(1e-5),
                slope,
                sign: (warm - cool) / (warm + cool).max(1e-5),
                near,
            };
        }
    });
}

/// Iron color for a cell: warm toward a north pole, cool toward a south one,
/// darkened toward the plate as the field weakens.
#[inline]
fn iron_color(look: &Look, sign: f32, lit: f32) -> Color {
    let tint = lerp_color(look.cool, look.warm, (sign * 0.5 + 0.5).clamp(0.0, 1.0));
    let ore = lerp_color(look.iron, tint, 0.55);
    lerp_color(look.plate, ore, lit.clamp(0.0, 1.0))
}

/// Bare glass with the grains that never found a line.
fn paint_plate(grid: &mut Grid, field: &[Sample], w: usize, h: usize, look: &Look) {
    let rows = grid.len().min(h);
    let slice = &mut grid[..rows];
    let seed = look.seed;
    let grain = look.grain;
    let paint = |(y, row): (usize, &mut Vec<Cell>)| {
        for (x, cell) in row.iter_mut().enumerate().take(w) {
            let s = &field[y * w + x];
            let strength = (s.mag / FULL).min(1.0);
            let g = hash(seed, L_DUST, x as u64, y as u64);
            let odds = grain * (0.04 + 0.5 * strength);
            *cell = if unit(g) < odds {
                let ch = DUST[(g >> 12) as usize % DUST.len()];
                Cell::with_bg(ch, lerp_color(look.plate, look.grit, 0.4 + 0.6 * strength), look.plate)
            } else {
                Cell::with_bg(' ', look.grit, look.plate)
            };
        }
    };
    if w * h >= PARALLEL_MIN_CELLS {
        slice.par_iter_mut().enumerate().with_min_len(8).for_each(paint);
    } else {
        slice.iter_mut().enumerate().for_each(paint);
    }
}

/// Filings on the contours: the glyph is the line's slope, the brightness is
/// the field, and the last of the strength falls off into single grains.
fn paint_filings(grid: &mut Grid, field: &[Sample], w: usize, h: usize, look: &Look) {
    let rows = grid.len().min(h);
    let slice = &mut grid[..rows];
    let width = look.width;
    let paint = |(y, row): (usize, &mut Vec<Cell>)| {
        for (x, cell) in row.iter_mut().enumerate().take(w) {
            let s = &field[y * w + x];
            if s.dist >= width {
                continue;
            }
            let edge = s.dist / width;
            let strength = (s.mag / FULL).min(1.0);
            let lit = (strength * (1.0 - 0.4 * edge)).clamp(0.0, 1.0);
            let heaped = look.glow > 0.0 && s.near < look.bar + look.glow * 2.2 && strength > 0.7;
            let ch = if heaped {
                let step = ((1.0 - s.near / (look.bar + look.glow * 2.2)) * HEAP.len() as f32) as usize;
                HEAP[step.min(HEAP.len() - 1)]
            } else if lit > 0.16 {
                DIR[s.slope as usize]
            } else {
                DUST[s.slope as usize % DUST.len()]
            };
            let fg = iron_color(look, s.sign, 0.3 + 0.7 * lit);
            *cell = Cell::with_bg(ch, fg, look.plate);
        }
    };
    if w * h >= PARALLEL_MIN_CELLS {
        slice.par_iter_mut().enumerate().with_min_len(8).for_each(paint);
    } else {
        slice.iter_mut().enumerate().for_each(paint);
    }
}

/// Squared distance from a plate point to a segment, plus where along it fell.
#[inline]
fn segment(px: f32, py: f32, ax: f32, ay: f32, bx: f32, by: f32) -> (f32, f32) {
    let (ex, ey) = (bx - ax, by - ay);
    let len2 = (ex * ex + ey * ey).max(1e-5);
    let t = (((px - ax) * ex + (py - ay) * ey) / len2).clamp(0.0, 1.0);
    let (dx, dy) = (px - ax - ex * t, py - ay - ey * t);
    ((dx * dx + dy * dy).sqrt(), t)
}

/// The magnet bodies, drawn over the filings, lettered at each pole.
fn paint_magnets(grid: &mut Grid, w: usize, h: usize, look: &Look) {
    let rows = grid.len().min(h);
    let body = look.bar;
    if body <= 0.0 {
        return;
    }
    let shell = darken(look.iron, 30);
    for bar in &look.bars {
        let pad = body + 1.5;
        let (lo_x, hi_x) = (bar.nx.min(bar.sx) - pad, bar.nx.max(bar.sx) + pad);
        let (lo_y, hi_y) = (bar.ny.min(bar.sy) - pad, bar.ny.max(bar.sy) + pad);
        let x0 = ((lo_x * look.aspect).floor().max(0.0)) as usize;
        let x1 = (((hi_x * look.aspect).ceil()).max(0.0) as usize).min(w);
        let y0 = (lo_y.floor().max(0.0)) as usize;
        let y1 = ((hi_y.ceil()).max(0.0) as usize).min(rows);
        for y in y0..y1 {
            let fy = y as f32 + 0.5;
            for x in x0..x1 {
                let fx = (x as f32 + 0.5) / look.aspect;
                let (d, t) = segment(fx, fy, bar.sx, bar.sy, bar.nx, bar.ny);
                if d >= body {
                    continue;
                }
                let tint = lerp_color(look.cool, look.warm, t);
                let ch = if d < body * 0.55 { BODY[0] } else { BODY[1] };
                grid[y][x] = Cell::with_bg(ch, lerp_color(shell, tint, 0.45), darken(tint, 78));
            }
        }
        let mark = |grid: &mut Grid, px: f32, py: f32, ch: char, tint: Color| {
            let cx = (px * look.aspect).floor();
            let cy = py.floor();
            if cx < 0.0 || cy < 0.0 {
                return;
            }
            let (cx, cy) = (cx as usize, cy as usize);
            if cx < w && cy < rows {
                grid[cy][cx] = Cell::with_bg(ch, lighten(tint, 60), darken(tint, 68));
            }
        };
        mark(grid, bar.nx, bar.ny, 'N', look.warm);
        mark(grid, bar.sx, bar.sy, 'S', look.cool);
    }
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
    fn ferrofluid_seed42() {
        insta::assert_snapshot!("opus_3_ferrofluid_80x24", text(&frame(80, 24, 42, 0.0, &knobs())));
    }

    #[test]
    fn ferrofluid_seed42_t7() {
        insta::assert_snapshot!("opus_3_ferrofluid_80x24_t7", text(&frame(80, 24, 42, 7.0, &knobs())));
    }

    #[test]
    fn deterministic_and_seed_sensitive() {
        let k = knobs();
        assert_eq!(text(&frame(90, 30, 42, 0.0, &k)), text(&frame(90, 30, 42, 0.0, &k)));
        assert_ne!(text(&frame(90, 30, 42, 0.0, &k)), text(&frame(90, 30, 9, 0.0, &k)));
    }

    #[test]
    fn time_moves_the_magnets() {
        let k = knobs();
        assert_ne!(text(&frame(90, 30, 42, 0.0, &k)), text(&frame(90, 30, 42, 5.0, &k)));
    }

    #[test]
    fn line_count_changes_the_figure() {
        let mut k = knobs();
        let a = text(&frame(90, 30, 42, 0.0, &k));
        k[1] = 9.0;
        assert_ne!(a, text(&frame(90, 30, 42, 0.0, &k)));
    }

    #[test]
    fn filings_cover_a_useful_share() {
        let k = knobs();
        let art = text(&frame(90, 30, 42, 0.0, &k));
        let ink = art.chars().filter(|c| !c.is_whitespace()).count();
        let cells = 90 * 30;
        assert!(ink > cells / 12, "too sparse: {ink} of {cells}");
        assert!(ink < cells * 4 / 5, "too dense: {ink} of {cells}");
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
        eprintln!("opus-3-ferrofluid frame_cost 200x60: avg {:.3} ms, worst {:.3} ms", avg, worst);
        if !cfg!(debug_assertions) {
            assert!(avg < 8.0, "avg frame {:.3} ms", avg);
        }
    }
}
