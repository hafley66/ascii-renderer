//! Rosette: a cathedral rose window. Each cell folds into one wedge of an N-fold
//! kaleidoscope; lead lines cut a ring-and-petal field into panes a sun sweeps.
use crate::_0_profile::measure_layer;
use crate::color::{darken, hsl_to_rgb, lerp_color, lighten};
use crate::opts::param_f32;
use crate::registry::{AnimKind, Mode, ModeFrame, Param};
use crate::types::{Cell, Grid};
use crossterm::style::Color;
use rayon::prelude::*;
use std::cell::RefCell;
use std::f32::consts::{PI, TAU};

pub(super) struct Rosette;
pub(super) static MODE: Rosette = Rosette;

const NAME: &str = "rosette";
const KNOBS: usize = 10;
const HELP: &str = "rosette: kaleidoscopic rose window, sun sweeping the panes [fold] [rings] [petals] [lead] [spin] [sun] [warp] [hue] [aspect] [margin]";

/// Glass density ramp, dark pane to full sun.
const GLASS: [char; 10] = [' ', '.', ':', '-', '=', '+', '*', 'o', 'O', '@'];
/// Stone tracery between panes, and the sparkle a lead throws when the sun hits.
const LEAD: [char; 2] = ['#', '%'];
const SPOKE_W: f32 = 0.45;
const RING_W: f32 = 0.4;
const GLINT: char = '+';
/// Wall texture outside the window.
const STONE: [char; 3] = ['.', ':', '\''];

const L_STONE: u64 = 0x11;
const L_PANE: u64 = 0x12;

const PARALLEL_MIN_CELLS: usize = 20_480;

const PARAMS: &[Param] = &[
    param!("FOLD", "kaleidoscope symmetry", 3.0, 24.0, 8.0, 1.0),
    param!("RINGS", "concentric rings", 1.0, 12.0, 4.0, 1.0),
    param!("PETALS", "petals per wedge", 1.0, 8.0, 2.0, 1.0),
    param!("LEAD", "lead line width", 0.03, 0.6, 0.16, 0.01),
    param!("SPIN", "rotation rad/s", -1.0, 1.0, 0.12, 0.02),
    param!("SUN", "sun sweep rad/s", 0.0, 3.0, 0.5, 0.05),
    param!("WARP", "organic warp", 0.0, 1.5, 0.35, 0.05),
    param!("HUE", "hue step per ring", 0.0, 180.0, 47.0, 1.0),
    param!("ASPECT", "cols per row", 0.25, 4.0, 2.0, 0.25),
    param!("MARGIN", "wall margin cells", 0.0, 12.0, 1.0, 1.0),
];

impl Mode for Rosette {
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

/// Value noise on the (ring, petal) lattice so the warp repeats with the fold.
#[inline]
fn noise(seed: u64, u: f32, v: f32) -> f32 {
    let (iu, iv) = (u.floor(), v.floor());
    let (fu, fv) = (smoothstep(u - iu), smoothstep(v - iv));
    let (iu, iv) = (iu as i64 as u64, iv as i64 as u64);
    let s = |du: u64, dv: u64| unit(hash(seed, L_PANE, iu.wrapping_add(du), iv.wrapping_add(dv)));
    let a = s(0, 0) + (s(1, 0) - s(0, 0)) * fu;
    let b = s(0, 1) + (s(1, 1) - s(0, 1)) * fu;
    (a + (b - a) * fv) * 2.0 - 1.0
}

/// One frame's resolved geometry and palette.
struct Look {
    seed: u64,
    cx: f32,
    cy: f32,
    radius: f32,
    aspect: f32,
    fold: f32,
    rings: f32,
    petals: f32,
    lead: f32,
    spin: f32,
    sun: f32,
    warp: f32,
    hue_step: f32,
    base_hue: f32,
    time: f32,
    wall: Color,
    stone: Color,
    lead_fg: Color,
    lead_bg: Color,
    boss: Color,
}

impl Look {
    fn new(seed: u64, w: usize, h: usize, palette: &[Color; 5], time: f32, p: &[f32; KNOBS]) -> Self {
        let aspect = p[8].max(0.25);
        let margin = p[9].round().max(0.0);
        let cx = w as f32 * 0.5;
        let cy = h as f32 * 0.5;
        let radius = ((h as f32 * 0.5 - margin).min(w as f32 * 0.5 / aspect - margin)).max(2.0);
        let base_hue = unit(hash(seed, L_PANE, 0, 7)) * 360.0;
        Look {
            seed,
            cx,
            cy,
            radius,
            aspect,
            fold: p[0].round().max(3.0),
            rings: p[1].round().max(1.0),
            petals: p[2].round().max(1.0),
            lead: p[3],
            spin: p[4],
            sun: p[5],
            warp: p[6],
            hue_step: p[7],
            base_hue,
            time,
            wall: darken(palette[0], 12),
            stone: lerp_color(palette[0], palette[2], 0.3),
            lead_fg: darken(palette[2], 70),
            lead_bg: darken(palette[0], 30),
            boss: lighten(palette[4], 30),
        }
    }
}

/// Polar sample of one cell: field value, ring index, petal index, light.
#[derive(Clone, Copy, Default)]
struct Sample {
    r: f32,
    field: f32,
    edge: f32,
    ring: i32,
    petal: i32,
    light: f32,
    theta: f32,
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
        measure_layer(NAME, "wall", || paint_wall(frame.grid, field, w, h, &look));
        measure_layer(NAME, "glass", || paint_glass(frame.grid, field, w, h, &look));
        measure_layer(NAME, "leads", || paint_leads(frame.grid, field, w, h, &look));
        measure_layer(NAME, "boss", || paint_boss(frame.grid, w, h, &look));
    });
}

/// Fold each cell into one mirrored wedge and evaluate the ring/petal field.
fn sample_field(field: &mut [Sample], w: usize, h: usize, look: &Look) {
    let wedge = TAU / look.fold;
    let spin = look.spin * look.time;
    let sun = look.sun * look.time - PI * 0.75;
    let warp = look.warp;
    let seed = look.seed;
    each_row(field, w, h, |(y, row)| {
        let dy = y as f32 + 0.5 - look.cy;
        for (x, s) in row.iter_mut().enumerate() {
            let dx = (x as f32 + 0.5 - look.cx) / look.aspect;
            let r = (dx * dx + dy * dy).sqrt();
            let theta = dy.atan2(dx);
            let turned = (theta + spin).rem_euclid(TAU);
            let mut a = turned.rem_euclid(wedge);
            if a > wedge * 0.5 {
                a = wedge - a;
            }
            let u = r / look.radius * look.rings;
            let v = a / (wedge * 0.5) * look.petals;
            let n = noise(seed, u * 1.7 + 3.0, v * 1.3 + 5.0) * warp;
            let ring = (TAU * (u + n * 0.25)).sin();
            let petal = (PI * (v + n * 0.35)).cos() * (0.25 + 0.75 * (r / look.radius).min(1.0));
            let f = ring * 0.65 + petal * 0.75;
            let spoke_d = r * a.min(wedge * 0.5 - a).sin();
            let ring_pitch = look.radius / look.rings;
            let ring_d = (r - (r / ring_pitch).round() * ring_pitch).abs();
            let mut edge = (f.abs() / look.lead).min(ring_d / RING_W);
            if r > 2.5 {
                edge = edge.min(spoke_d / SPOKE_W);
            }
            let facing = (theta - sun).cos();
            let light = 0.5 + 0.5 * facing;
            *s = Sample {
                r,
                field: f,
                edge,
                ring: (u + n * 0.25).floor() as i32,
                petal: (v + n * 0.35).floor() as i32,
                light,
                theta,
            };
        }
    });
}

fn paint_wall(grid: &mut Grid, field: &[Sample], w: usize, h: usize, look: &Look) {
    let rim_in = look.radius;
    let rim_out = look.radius + 0.6;
    let rows = grid.len().min(h);
    let slice = &mut grid[..rows];
    let paint = |(y, row): (usize, &mut Vec<Cell>)| {
        for (x, cell) in row.iter_mut().enumerate().take(w) {
            let s = &field[y * w + x];
            if s.r < rim_in {
                continue;
            }
            *cell = if s.r < rim_out {
                Cell::with_bg('@', look.stone, look.lead_bg)
            } else {
                let g = hash(look.seed, L_STONE, x as u64, y as u64);
                let k = unit(g);
                let ch = if k < 0.12 {
                    STONE[(g >> 8) as usize % STONE.len()]
                } else {
                    ' '
                };
                Cell::with_bg(ch, darken(look.stone, 40), look.wall)
            };
        }
    };
    if w * h >= PARALLEL_MIN_CELLS {
        slice.par_iter_mut().enumerate().with_min_len(8).for_each(paint);
    } else {
        slice.iter_mut().enumerate().for_each(paint);
    }
}

/// Pane color: hue climbs by ring, splits by petal, flips warm/cool on the
/// field sign, and the sun decides how much of it shows through.
#[inline]
fn pane_color(look: &Look, s: &Sample) -> (Color, Color) {
    let side = if s.field >= 0.0 { 0.0 } else { 150.0 };
    let jitter = unit(hash(
        look.seed,
        L_PANE,
        s.ring as i64 as u64,
        (s.petal as i64 as u64) << 8 | (s.field >= 0.0) as u64,
    )) * 24.0;
    let hue = (look.base_hue + s.ring as f32 * look.hue_step + s.petal as f32 * 19.0 + side + jitter)
        .rem_euclid(360.0) as f64;
    let depth = s.field.abs().min(1.0) as f64;
    let lit = (0.28 + 0.42 * s.light as f64) * (0.55 + 0.45 * depth);
    let fg = hsl_to_rgb(hue, 0.78, lit.clamp(0.08, 0.72));
    let bg = hsl_to_rgb(hue, 0.6, (lit * 0.38).clamp(0.03, 0.3));
    (fg, bg)
}

fn paint_glass(grid: &mut Grid, field: &[Sample], w: usize, h: usize, look: &Look) {
    let rows = grid.len().min(h);
    let slice = &mut grid[..rows];
    let paint = |(y, row): (usize, &mut Vec<Cell>)| {
        for (x, cell) in row.iter_mut().enumerate().take(w) {
            let s = &field[y * w + x];
            if s.r >= look.radius || s.edge < 1.0 {
                continue;
            }
            let (fg, bg) = pane_color(look, s);
            let grain = unit(hash(look.seed, L_PANE, x as u64, y as u64 | 1 << 32)) * 0.1;
            let level = (s.field.abs().min(1.0) * 0.5 + s.light * 0.42 + grain - 0.05).clamp(0.0, 0.999);
            let ch = GLASS[(level * GLASS.len() as f32) as usize];
            *cell = Cell::with_bg(ch, fg, bg);
        }
    };
    if w * h >= PARALLEL_MIN_CELLS {
        slice.par_iter_mut().enumerate().with_min_len(8).for_each(paint);
    } else {
        slice.iter_mut().enumerate().for_each(paint);
    }
}

fn paint_leads(grid: &mut Grid, field: &[Sample], w: usize, h: usize, look: &Look) {
    let sun = look.sun * look.time - PI * 0.75;
    let rows = grid.len().min(h);
    let slice = &mut grid[..rows];
    let paint = |(y, row): (usize, &mut Vec<Cell>)| {
        for (x, cell) in row.iter_mut().enumerate().take(w) {
            let s = &field[y * w + x];
            if s.r >= look.radius || s.edge >= 1.0 {
                continue;
            }
            let facing = (s.theta - sun).cos();
            let core = s.edge < 0.5;
            let (ch, fg) = if facing > 0.93 && look.time > 0.0 {
                (GLINT, lighten(look.lead_fg, 120))
            } else if core {
                (LEAD[0], look.lead_fg)
            } else {
                (LEAD[1], darken(look.lead_fg, 20))
            };
            *cell = Cell::with_bg(ch, fg, look.lead_bg);
        }
    };
    if w * h >= PARALLEL_MIN_CELLS {
        slice.par_iter_mut().enumerate().with_min_len(8).for_each(paint);
    } else {
        slice.iter_mut().enumerate().for_each(paint);
    }
}

/// The central boss: a small bright disc with a ring of stone around it.
fn paint_boss(grid: &mut Grid, w: usize, h: usize, look: &Look) {
    let pulse = if look.time > 0.0 {
        0.5 + 0.5 * (look.time * 1.7).sin()
    } else {
        0.5
    };
    let core = lighten(look.boss, (pulse * 40.0) as u8);
    let reach_x = (2.0 * look.aspect).ceil() as usize + 1;
    let (x0, x1) = ((look.cx as usize).saturating_sub(reach_x), (look.cx as usize + reach_x).min(w));
    let (y0, y1) = ((look.cy as usize).saturating_sub(3), (look.cy as usize + 3).min(h.min(grid.len())));
    for y in y0..y1 {
        let dy = y as f32 + 0.5 - look.cy;
        for x in x0..x1 {
            let dx = (x as f32 + 0.5 - look.cx) / look.aspect;
            let r = (dx * dx + dy * dy).sqrt();
            if r < 0.9 {
                grid[y][x] = Cell::with_bg('@', core, look.lead_bg);
            } else if r < 1.8 {
                grid[y][x] = Cell::with_bg('%', look.stone, look.lead_bg);
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
    fn rosette_seed42() {
        insta::assert_snapshot!("rosette_80x24", text(&frame(80, 24, 42, 0.0, &knobs())));
    }

    #[test]
    fn rosette_seed42_t6() {
        insta::assert_snapshot!("rosette_80x24_t6", text(&frame(80, 24, 42, 6.0, &knobs())));
    }

    #[test]
    fn deterministic_and_seed_sensitive() {
        let k = knobs();
        assert_eq!(text(&frame(90, 30, 42, 0.0, &k)), text(&frame(90, 30, 42, 0.0, &k)));
        assert_ne!(text(&frame(90, 30, 42, 0.0, &k)), text(&frame(90, 30, 7, 0.0, &k)));
    }

    #[test]
    fn time_turns_the_window() {
        let k = knobs();
        assert_ne!(text(&frame(90, 30, 42, 0.0, &k)), text(&frame(90, 30, 42, 4.0, &k)));
    }

    #[test]
    fn fold_changes_the_figure() {
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
        eprintln!("rosette frame_cost 200x60: avg {:.3} ms, worst {:.3} ms", avg, worst);
        if !cfg!(debug_assertions) {
            assert!(avg < 6.0, "avg frame {:.3} ms", avg);
        }
    }
}
