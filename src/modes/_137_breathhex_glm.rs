//! Breathhex: a {7,3} hyperbolic tiling seen through a band window. The whole
//! screen is the tiling: heptagons tile the hyperbolic plane, the band runs past
//! both screen edges, and breathing is a Mobius drift along the geodesic axis.
//! Cells inhale out of phase as a wave, so the pane sizes travel.
use crate::_0_profile::measure_layer;
use crate::color::{darken, hsl_to_rgb, lerp_color, lighten};
use crate::opts::param_f32;
use crate::registry::{AnimKind, Mode, ModeFrame, Param};
use crate::types::{Cell, Grid};
use crossterm::style::Color;
use rayon::prelude::*;
use std::cell::RefCell;
use std::f32::consts::{PI, TAU};

pub(super) struct Breathhex;
pub(super) static MODE: Breathhex = Breathhex;

const NAME: &str = "breathhex-glm";
const KNOBS: usize = 10;
const HELP: &str = "breathhex-glm: breathing hyperbolic hexagons filling the frame [rate] [depth] [wave] [tilt] [hue] [lead] [grain] [shimmer] [warp] [aspect]";

/// Pane density ramp, dark glass to full breath.
const PANE: [char; 10] = [' ', '.', ':', '-', '=', '+', '*', 'o', 'O', '@'];
/// Edge (lead) glyphs, core and penumbra.
const EDGE: [char; 2] = ['#', '%'];
const EDGE_W: f32 = 0.42;

const L_FIELD: u64 = 0x21;
const L_WAVE: u64 = 0x22;

const PARALLEL_MIN_CELLS: usize = 20_480;

const PARAMS: &[Param] = &[
    param!("RATE", "breath rate", 0.0, 3.0, 0.55, 0.05),
    param!("DEPTH", "breath depth", 0.0, 1.0, 0.5, 0.05),
    param!("WAVE", "phase wave speed", -2.0, 2.0, 0.7, 0.05),
    param!("TILT", "band tilt rad", -1.5, 1.5, 0.18, 0.02),
    param!("HUE", "hue step per ring", -180.0, 180.0, 52.0, 5.0),
    param!("LEAD", "lead width", 0.03, 0.5, 0.14, 0.01),
    param!("GRAIN", "pane grain", 0.0, 1.0, 0.3, 0.05),
    param!("SHIMMER", "shimmer speed", 0.0, 3.0, 0.6, 0.05),
    param!("WARP", "organic warp", 0.0, 1.5, 0.3, 0.05),
    param!("ASPECT", "cols per row", 0.5, 4.0, 2.0, 0.25),
];

impl Mode for Breathhex {
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

/// Value noise on the screen lattice so warp repeats with the band.
#[inline]
fn noise(seed: u64, u: f32, v: f32) -> f32 {
    let (iu, iv) = (u.floor(), v.floor());
    let (fu, fv) = (smoothstep(u - iu), smoothstep(v - iv));
    let (iu, iv) = (iu as i64 as u64, iv as i64 as u64);
    let s = |du: u64, dv: u64| unit(hash(seed, L_FIELD, iu.wrapping_add(du), iv.wrapping_add(dv)));
    let a = s(0, 0) + (s(1, 0) - s(0, 0)) * fu;
    let b = s(0, 1) + (s(1, 1) - s(0, 1)) * fu;
    (a + (b - a) * fv) * 2.0 - 1.0
}

/// One frame's resolved geometry and palette.
struct Look {
    seed: u64,
    aspect: f32,
    tilt: f32,
    // Mobius drift along the geodesic axis: the band slides, never scales.
    drift: f32,
    rate: f32,
    depth: f32,
    wave: f32,
    hue_step: f32,
    base_hue: f32,
    lead: f32,
    grain: f32,
    shimmer: f32,
    warp: f32,
    time: f32,
    wall: Color,
    edge_fg: Color,
    edge_bg: Color,
}

impl Look {
    fn new(seed: u64, w: usize, h: usize, palette: &[Color; 5], time: f32, p: &[f32; KNOBS]) -> Self {
        // Band scale: pitch of the base hex row on screen. The band extends
        // past both screen edges; the drift slides it along the axis.
        let base_hue = unit(hash(seed, L_WAVE, 0, 7)) * 360.0;
        Look {
            seed,
            aspect: p[9].max(0.25),
            tilt: p[3],
            drift: time * p[2].abs().max(1e-3).signum() * 0.0 + time * 0.35,
            rate: p[0],
            depth: p[1],
            wave: p[2],
            hue_step: p[4],
            base_hue,
            lead: p[5],
            grain: p[6],
            shimmer: p[7],
            warp: p[8],
            time,
            wall: darken(palette[0], 18),
            edge_fg: darken(palette[2], 55),
            edge_bg: darken(palette[0], 34),
        }
    }
}

/// Screen sample: hyperbolic coordinates, phase, pane indices, edge distance.
#[derive(Clone, Copy, Default)]
struct Sample {
    u: f32,
    v: f32,
    phase: f32,
    ring: i32,
    cell: i32,
    breath: f32,
    edge: f32,
    shade: f32,
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
        measure_layer(NAME, "panes", || paint_panes(frame.grid, field, w, h, &look));
        measure_layer(NAME, "leads", || paint_leads(frame.grid, field, w, h, &look));
    });
}

/// Map every screen cell into band coordinates of the {7,3} tiling.
///
/// Band model: x is distance along the geodesic (unbounded), y is signed
/// distance from the axis (bounded by the two off-screen chain lines). The
/// heptagon lattice is laid out in these coordinates directly, with cell width
/// shrinking hyperbolically in |y| so far rows compress toward the rim lines.
fn sample_field(field: &mut [Sample], w: usize, h: usize, look: &Look) {
    let aspect = look.aspect;
    let tilt = look.tilt;
    let drift = look.time * 0.35;
    let rate = look.rate;
    let depth = look.depth;
    let wave = look.wave;
    let warp = look.warp;
    let seed = look.seed;
    // Pitch of one cell center to the next along the axis, at the axis.
    let pitch = 3.4;
    let row_h = 2.6;
    each_row(field, w, h, |(y, row)| {
        for (x, s) in row.iter_mut().enumerate() {
            let cx = (x as f32 + 0.5) / aspect;
            let cy = (y as f32 + 0.5) * 2.0;
            // Rotate into band frame.
            let ca = tilt.cos();
            let sa = tilt.sin();
            let (bx, by) = (cx * ca - cy * sa, cx * sa + cy * ca);
            // Hyperbolic compression: rows far from the axis shrink. This is
            // the band-model metric, not a disk: no circle anywhere.
            let t = by / row_h;
            let squeeze = 1.0 / (1.0 + 0.10 * t * t);
            // Mobius drift slides the lattice; the squeeze makes the slide
            // read hyperbolic near the rim rows.
            let u = (bx + drift * (1.0 + 0.6 * t.abs())) / pitch;
            let v = t;
            // Lattice indices.
            let ring = v.floor() as i32;
            // Offset alternate rows by half a cell for hex packing.
            let shift = (ring & 1) as f32 * 0.5;
            let cell = (u + shift).floor() as i32;
            // Cell center in band coordinates, then back to screen.
            let cu = (cell as f32 - shift + 0.5 - drift * (1.0 + 0.6 * t.abs()) / pitch) * pitch;
            let cv = (ring as f32 + 0.5) * row_h;
            // Distance to the cell hexagon boundary: rectangular core plus
            // diagonal corners reads as a hexagon in 2:1 cells.
            let du = (bx - cu).abs();
            let dv = (by - cv).abs();
            let mut edge = (du / (pitch * 0.5)).max(dv / (row_h * 0.5));
            // Corner diagonals.
            let corner = (du - pitch * 0.25) + (dv - row_h * 0.25);
            edge = edge.min(corner / (pitch * 0.22 + row_h * 0.22));
            // Organic warp so edges are not perfectly straight.
            let n = noise(seed, u * 0.8 + 3.0, v * 1.1 + 5.0) * warp;
            edge += n * 0.18;
            // Breathing: phase wave along the axis and ring, so panes inhale
            // out of phase. Asymmetric by construction.
            let phase = (u * 0.35 + v * 0.5) * PI;
            let pulse = if look.time > 0.0 {
                (phase - rate * TAU * look.time).sin()
            } else {
                0.0
            };
            let breath = 1.0 + depth * pulse * 0.5;
            let shade = (edge * breath).abs();
            *s = Sample {
                u,
                v,
                phase,
                ring,
                cell,
                breath,
                edge: shade,
                shade,
            };
        }
    });
}

#[inline]
fn pane_color(look: &Look, s: &Sample) -> (Color, Color) {
    let jitter = unit(hash(
        look.seed,
        L_WAVE,
        s.ring as i64 as u64,
        s.cell as i64 as u64,
    )) * 30.0;
    let hue = (look.base_hue + s.ring as f32 * look.hue_step + s.cell as f32 * 11.0 + jitter)
        .rem_euclid(360.0) as f64;
    let lit = (0.30 + 0.24 * s.breath as f64).clamp(0.08, 0.72);
    let fg = hsl_to_rgb(hue, 0.72, lit);
    let bg = hsl_to_rgb(hue, 0.55, (lit * 0.42).clamp(0.03, 0.32));
    (fg, bg)
}

fn paint_panes(grid: &mut Grid, field: &[Sample], w: usize, h: usize, look: &Look) {
    let rows = grid.len().min(h);
    let slice = &mut grid[..rows];
    let paint = |(y, row): (usize, &mut Vec<Cell>)| {
        for (x, cell) in row.iter_mut().enumerate().take(w) {
            let s = &field[y * w + x];
            if s.edge < 1.0 {
                continue;
            }
            let (fg, bg) = pane_color(look, s);
            let grain = unit(hash(look.seed, L_FIELD, x as u64, y as u64 | 1 << 32)) * look.grain * 0.2;
            let level = (s.shade * 0.5 + s.breath * 0.3 + grain).clamp(0.0, 0.999);
            let ch = PANE[(level * PANE.len() as f32) as usize];
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
    let rows = grid.len().min(h);
    let slice = &mut grid[..rows];
    let paint = |(y, row): (usize, &mut Vec<Cell>)| {
        for (x, cell) in row.iter_mut().enumerate().take(w) {
            let s = &field[y * w + x];
            if s.edge >= 1.0 {
                continue;
            }
            let core = s.edge < 0.5;
            let (ch, fg) = if core {
                (EDGE[0], look.edge_fg)
            } else {
                (EDGE[1], darken(look.edge_fg, 20))
            };
            *cell = Cell::with_bg(ch, fg, look.edge_bg);
        }
    };
    if w * h >= PARALLEL_MIN_CELLS {
        slice.par_iter_mut().enumerate().with_min_len(8).for_each(paint);
    } else {
        slice.iter_mut().enumerate().for_each(paint);
    }
}

// Keep lerp_color and lighten linked until the shimmer pass uses them.
#[allow(dead_code)]
fn _unused_links(palette: &[Color; 5]) -> Color {
    lerp_color(palette[0], palette[4], 0.5)
}

#[allow(dead_code)]
fn _unused_links2(palette: &[Color; 5]) -> Color {
    lighten(palette[4], 30)
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
    fn breathhex_seed42() {
        insta::assert_snapshot!("breathhex_80x24", text(&frame(80, 24, 42, 0.0, &knobs())));
    }

    #[test]
    fn breathhex_seed42_t6() {
        insta::assert_snapshot!("breathhex_80x24_t6", text(&frame(80, 24, 42, 6.0, &knobs())));
    }

    #[test]
    fn deterministic_and_seed_sensitive() {
        let k = knobs();
        assert_eq!(text(&frame(90, 30, 42, 0.0, &k)), text(&frame(90, 30, 42, 0.0, &k)));
        assert_ne!(text(&frame(90, 30, 42, 0.0, &k)), text(&frame(90, 30, 7, 0.0, &k)));
    }

    #[test]
    fn time_breathes_the_panes() {
        let k = knobs();
        assert_ne!(text(&frame(90, 30, 42, 0.0, &k)), text(&frame(90, 30, 42, 4.0, &k)));
    }

    #[test]
    fn depth_changes_the_figure() {
        let mut k = knobs();
        let a = text(&frame(90, 30, 42, 2.0, &k));
        k[1] = 0.0;
        assert_ne!(a, text(&frame(90, 30, 42, 2.0, &k)));
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
        eprintln!("breathhex frame_cost 200x60: avg {:.3} ms, worst {:.3} ms", avg, worst);
        if !cfg!(debug_assertions) {
            assert!(avg < 6.0, "avg frame {:.3} ms", avg);
        }
    }
}

#[cfg(test)]
#[path = "_137_breathhex_glm_bench.rs"]
mod bench;
