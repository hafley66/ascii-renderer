//! Celestial engraving with bounded animated ink. Independent of Gem 1 and 2.
//! Per frame: O(W*H + R*min(3*(W+H),4096) + 400 + 249) work,
//! one <=4096-entry geometry allocation, no retained simulation or per-cell IO.
use crate::opts::param_f32;
use crate::registry::{AnimKind, Mode, ModeFrame, Param};
use crate::types::{Cell, Grid};
use crossterm::style::Color;
use std::f32::consts::{PI, TAU};

pub(super) struct Aetherium;
pub(super) static MODE: Aetherium = Aetherium;
const PARAMS: &[Param] = &[
    param!("RINGS", "engraved orbital shells", 2.0, 12.0, 7.0, 1.0),
    param!("PLANETS", "orbiting lanterns", 1.0, 12.0, 7.0, 1.0),
    param!("COMETS", "counterpoint comets", 0.0, 8.0, 3.0, 1.0),
    param!("TRAIL", "comet and planet tails", 1.0, 8.0, 5.0, 1.0),
    param!("HARMONIC", "orbital harmonic", 2.0, 9.0, 5.0, 1.0),
    param!("TILT", "armillary inclination", 0.0, 1.0, 0.55, 0.05),
    param!("DUST", "fixed constellation stars", 0.0, 400.0, 160.0, 20.0),
    param!("NEBULA", "engraved cloud bands", 0.0, 1.0, 0.4, 0.05),
    param!("SPEED", "orbital motion", 0.1, 3.0, 0.7, 0.1),
    param!("SPREAD", "celestial field extent", 0.45, 1.0, 0.85, 0.05),
    param!("PETALS", "rosette depth", 0.0, 1.0, 0.35, 0.05),
    param!("TRACERS", "harmonic fireflies", 0.0, 24.0, 12.0, 1.0),
];

impl Mode for Aetherium {
    fn name(&self) -> &'static str {
        "gem-aetherium-3"
    }
    fn help(&self) -> &'static str {
        "gem-aetherium-3: engraved celestial rosettes, orbiting lanterns and bounded comet trails"
    }
    fn animation(&self) -> AnimKind {
        AnimKind::Iterate
    }
    fn params(&self) -> &'static [Param] {
        PARAMS
    }
    fn render(&self, frame: &mut ModeFrame<'_>) {
        // Resolve borrowed frame inputs once; all geometry is seed/knob derived.
        // Engrave stationary nebulae, star charts and harmonic orbital shells.
        // Evaluate moving lanterns and bounded trails analytically at frame.time.
        // No output, logging, environment lookup or allocation inside drawing loops.
        let p: [f32; 12] = std::array::from_fn(|i| {
            let v = frame
                .param_values
                .and_then(|v| v.get(i))
                .copied()
                .unwrap_or_else(|| param_f32(PARAMS[i].key, PARAMS[i].default));
            if v.is_finite() {
                v.clamp(PARAMS[i].min, PARAMS[i].max)
            } else {
                PARAMS[i].default
            }
        });
        draw(
            frame.grid,
            frame.width,
            frame.height,
            frame.seed,
            frame.palette,
            frame.time,
            &p,
        );
    }
}

// The caller owns the grid; scratch geometry lives for this call only. At most
// 12*(8 tail + 5 body) + 8*8 comet + 24 tracer + 5 core = 249 moving writes.
fn draw(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    time: f32,
    p: &[f32; 12],
) {
    if width == 0 || height == 0 {
        return;
    }
    let [
        rings,
        planets,
        comets,
        trail,
        harmonic,
        tilt,
        dust,
        nebula,
        speed,
        spread,
        petals,
        tracers,
    ] = *p;
    let phase = unit(seed ^ 0x519d) * TAU;
    let t = if time.is_finite() {
        time * speed * 0.35
    } else {
        0.0
    };
    let cx = (width - 1) as f32 * 0.5;
    let cy = (height - 1) as f32 * 0.5;
    let sx = cx * spread;
    let sy = cy * spread;
    let faint = crate::color::darken(palette[2], 45);
    let ink = crate::color::darken(palette[1], 60);
    let bright = palette[1];
    let warm = palette[3];
    let white = palette[4];

    // Stationary, continuous bands use arithmetic only at cell scale. Their
    // glyph/color values never change with time, so retained terminal diffs omit them.
    for (y, row) in grid.iter_mut().enumerate() {
        let v = (y as f32 - cy) / cy.max(1.0);
        for (x, cell) in row.iter_mut().enumerate() {
            let u = (x as f32 - cx) / cx.max(1.0);
            let band = ((u * u * 1.7 + v * v * 0.6 + u * v * tilt) * 19.0 + phase) as i32;
            let in_cloud = u * u + v * v < 0.92 && (u * 0.6 + v).abs() < nebula * 0.65;
            let ch = if in_cloud && band.rem_euclid(4) == 0 {
                if (x + y * 2) % 4 == 0 { ':' } else { '.' }
            } else {
                ' '
            };
            *cell = Cell {
                ch,
                fg: if ch == ' ' { Color::Reset } else { faint },
                bg: Color::Reset,
            };
        }
    }
    for i in 0..dust as usize {
        let h = seed.wrapping_add((i as u64).wrapping_mul(0x9e3779b97f4a7c15));
        let x = unit(h ^ 0x613b) * (width - 1) as f32;
        let y = unit(h ^ 0x72c9) * (height - 1) as f32;
        put(
            grid,
            x,
            y,
            if i % 13 == 0 { '+' } else { '.' },
            if i % 13 == 0 { ink } else { faint },
        );
    }

    // One shared unit-circle/rosette table; no trigonometry per ring sample.
    let samples = width
        .saturating_add(height)
        .saturating_mul(3)
        .clamp(32, 4096);
    let circle: Vec<_> = (0..samples)
        .map(|i| {
            let a = i as f32 * TAU / samples as f32;
            let (s, c) = a.sin_cos();
            (s, c, 1.0 + petals * 0.16 * (a * harmonic + phase).cos())
        })
        .collect();
    for ring in 0..rings as usize {
        let radius = 0.22 + 0.75 * (ring + 1) as f32 / rings;
        let turn = phase + ring as f32 * PI / rings;
        let (rs, rc) = turn.sin_cos();
        let flatten = 0.28 + (1.0 - tilt) * 0.52;
        for (i, &(s, c, wave)) in circle.iter().enumerate() {
            let x = radius * wave * (c * rc - s * flatten * rs);
            let y = radius * wave * (c * rs + s * flatten * rc);
            let ch = if i % (samples / 16).max(1) == 0 {
                '+'
            } else if (s * rc + c * flatten * rs).abs() > 0.65 {
                '-'
            } else {
                '.'
            };
            put(
                grid,
                cx + sx * x,
                cy + sy * y,
                ch,
                if ring % 3 == 0 { bright } else { ink },
            );
        }
    }
    // Fixed radial ticks make the outer dial immediately legible at t=0.
    for i in 0..48 {
        let a = i as f32 * TAU / 48.0;
        let (s, c) = a.sin_cos();
        put(
            grid,
            cx + sx * c,
            cy + sy * s,
            if i % 4 == 0 { '+' } else { '.' },
            ink,
        );
        if i % 4 == 0 {
            put(
                grid,
                cx + sx * c * 1.05,
                cy + sy * s * 1.05,
                b"ABCDEFGHIJKL"[i / 4] as char,
                warm,
            );
        }
    }

    // Motion affects at most 249 cells at every declared maximum. Stable trail
    // colors avoid flashing entire color fields. Old marks are restored naturally
    // when the common runtime compares this reconstructed frame to its predecessor.
    for i in 0..planets as usize {
        let turn = phase + i as f32 * 2.399963;
        let radius = 0.26 + 0.65 * (i + 1) as f32 / planets;
        let orbit = |a: f32| {
            let (s, c) = a.sin_cos();
            let (rs, rc) = turn.sin_cos();
            (
                cx + sx * radius * (c * rc - s * 0.55 * rs),
                cy + sy * radius * (c * rs + s * 0.55 * rc),
            )
        };
        let a = turn + t * (1.0 + (i % 3) as f32 / harmonic);
        for k in (1..=trail as usize).rev() {
            let (x, y) = orbit(a - k as f32 * 0.055);
            put(grid, x, y, '.', ink);
        }
        let (x, y) = orbit(a);
        put(grid, x - 1.0, y, '(', bright);
        put(grid, x + 1.0, y, ')', bright);
        put(grid, x, y - 1.0, '.', warm);
        put(grid, x, y + 1.0, '.', warm);
        put(grid, x, y, if i % 3 == 0 { '@' } else { 'o' }, white);
    }
    for i in 0..comets as usize {
        for k in (0..trail as usize).rev() {
            let a = phase + i as f32 * TAU / comets - t * 1.35 + k as f32 * 0.04;
            let (s, c) = a.sin_cos();
            let r = 0.83 + 0.11 * (a * harmonic).sin();
            put(
                grid,
                cx + sx * r * c,
                cy + sy * r * s,
                if k == 0 { '*' } else { '.' },
                if k == 0 { white } else { warm },
            );
        }
    }
    for i in 0..tracers as usize {
        let a = t * 0.65 + phase + i as f32 * TAU / tracers;
        put(
            grid,
            cx + sx * 0.45 * (a * 2.0).sin(),
            cy + sy * 0.45 * (a * 3.0).sin(),
            '+',
            bright,
        );
    }
    let (s, c) = t.sin_cos();
    put(grid, cx, cy, 'O', white);
    put(grid, cx + 2.0 * c, cy + s, '*', warm);
    put(grid, cx - 2.0 * c, cy - s, '*', warm);
    put(grid, cx - 2.0 * s, cy + c, '+', bright);
    put(grid, cx + 2.0 * s, cy - c, '+', bright);
}

fn put(grid: &mut Grid, x: f32, y: f32, ch: char, fg: Color) {
    let (x, y) = (x.round() as isize, y.round() as isize);
    if x >= 0 && y >= 0 {
        if let Some(cell) = grid
            .get_mut(y as usize)
            .and_then(|row| row.get_mut(x as usize))
        {
            *cell = Cell {
                ch,
                fg,
                bg: Color::Reset,
            };
        }
    }
}

fn unit(mut n: u64) -> f32 {
    n = (n ^ (n >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    n = (n ^ (n >> 27)).wrapping_mul(0x94d049bb133111eb);
    ((n ^ (n >> 31)) >> 40) as f32 / 16_777_216.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{SeedableRng, rngs::StdRng};
    fn frame(w: usize, h: usize, t: f32, max: bool) -> Grid {
        let mut grid = vec![vec![Cell::blank(); w]; h];
        let mut rng = StdRng::seed_from_u64(42);
        let values: Vec<_> = PARAMS
            .iter()
            .map(|p| if max { p.max } else { p.default })
            .collect();
        MODE.render(&mut ModeFrame {
            grid: &mut grid,
            width: w,
            height: h,
            seed: 42,
            palette: &crate::color::make_palette(42),
            rng: &mut rng,
            time: t,
            args: &[],
            param_values: Some(&values),
        });
        grid
    }
    fn text(grid: &Grid) -> String {
        grid.iter()
            .map(|r| r.iter().map(|c| c.ch).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }
    #[test]
    fn gem3_snapshots_and_deterministic_motion() {
        let initial = frame(80, 30, 0.0, false);
        let moving = frame(80, 30, 1.5, false);
        insta::assert_snapshot!("gem3_seed42", text(&initial));
        insta::assert_snapshot!("gem3_seed42_moving", text(&moving));
        assert_ne!(initial, moving);
        assert_eq!(moving, frame(80, 30, 1.5, false));
    }
    #[test]
    fn gem3_max_motion_has_constant_cell_budget_and_handles_tiny_grids() {
        for (w, h) in [(0, 0), (1, 1), (2, 3), (162, 61), (366, 199)] {
            let before = frame(w, h, 0.0, true);
            for t in [0.06, 0.5, 17.0, 1000.0] {
                let after = frame(w, h, t, true);
                let changes = before
                    .iter()
                    .flatten()
                    .zip(after.iter().flatten())
                    .filter(|(a, b)| a != b)
                    .count();
                assert!(changes <= 498, "{w}x{h} t={t}: {changes} changed cells");
                assert!(after.iter().flatten().all(|c| c.ch.is_ascii()));
            }
        }
    }

    #[test]
    fn gem3_max_animation_bounds_real_terminal_payload_over_time() {
        for (w, h) in [(162, 61), (366, 199)] {
            let mut encoder = crate::gridio::AnsiFrameEncoder::new();
            let mut output = String::new();
            for i in 0..100 {
                let grid = frame(w, h, i as f32 * 0.06, true);
                encoder.encode(&grid, false, &mut output);
                if i > 0 {
                    assert!(
                        output.len() <= 20_000,
                        "{w}x{h} frame {i}: {} encoded bytes",
                        output.len()
                    );
                }
            }
        }
    }
}
