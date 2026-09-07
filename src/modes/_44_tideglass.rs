use crate::color::{darken, lerp_color, shift_hue};
use crate::opts::param_f32;
use crate::registry::{AnimKind, Mode, ModeFrame, Param};
use crate::types::{Cell, Grid};
use crossterm::style::Color;
use rand::RngExt;
use std::f32::consts::{PI, TAU};

pub(super) struct TideglassMode;
pub(super) static MODE: TideglassMode = TideglassMode;

const PARAMS: &[Param] = &[
    param!("SCALE", "instrument radius", 0.5, 1.15, 0.92, 0.05),
    param!("RINGS", "nested orbital bands", 1.0, 6.0, 3.0, 1.0),
    param!("TILT", "orbital plane inclination", 0.1, 1.4, 0.7, 0.05),
    param!(
        "TURN",
        "band rotation and handedness",
        -1.0,
        1.0,
        0.35,
        0.05
    ),
    param!("APERTURE", "open iris radius", 0.12, 0.82, 0.38, 0.05),
    param!("TEETH", "graduations and iris facets", 6.0, 36.0, 18.0, 1.0),
    param!("PENDANTS", "hanging light vessels", 0.0, 13.0, 7.0, 1.0),
    param!("SWAY", "pendulum excursion", 0.0, 1.0, 0.35, 0.05),
    param!("TIDE", "waterline height", 0.62, 0.85, 0.73, 0.025),
    param!("WAVES", "reflection fragmentation", 0.0, 1.0, 0.4, 0.05),
    param!("STARS", "distant drifting lights", 0.0, 1.0, 0.3, 0.05),
    param!("GLOW", "lens and water radiance", 0.1, 1.0, 0.65, 0.05),
    param!("HUE", "material hue rotation", -180.0, 180.0, 0.0, 10.0),
    param!("SPEED", "scene clock", 0.0, 2.0, 0.6, 0.05),
];

impl Mode for TideglassMode {
    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
    fn name(&self) -> &'static str {
        "tideglass"
    }
    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
    fn help(&self) -> &'static str {
        "Suspended tidal observatory: orbiting iris, pendulum lights and broken reflections [scale] [rings] [tilt] [turn] [aperture] [teeth] [pendants] [sway] [tide] [waves] [stars] [glow] [hue] [speed]"
    }
    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
    fn animation(&self) -> AnimKind {
        AnimKind::Iterate
    }
    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
    fn params(&self) -> &'static [Param] {
        PARAMS
    }
    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
    fn render(&self, frame: &mut ModeFrame<'_>) {
        let k = std::array::from_fn(|i| {
            let p = &PARAMS[i];
            let value = frame
                .args
                .get(i + 4)
                .and_then(|v| v.parse::<f32>().ok())
                .or_else(|| frame.param_values.and_then(|v| v.get(i)).copied())
                .unwrap_or_else(|| param_f32(p.key, p.default));
            if value.is_finite() {
                value.clamp(p.min, p.max)
            } else {
                p.default
            }
        });
        let t = if frame.time.is_finite() {
            (frame.time as f64 * k[13] as f64).clamp(-1.0e6, 1.0e6) as f32
        } else {
            0.0
        };
        draw_tideglass(frame, &k, t);
    }
}

#[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
fn put(grid: &mut Grid, x: f32, y: f32, cell: Cell) {
    if x >= 0.0 && y >= 0.0 {
        if let Some(target) = grid
            .get_mut(y.round() as usize)
            .and_then(|row| row.get_mut(x.round() as usize))
        {
            *target = cell;
        }
    }
}

#[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
fn line(grid: &mut Grid, a: [f32; 2], b: [f32; 2], color: Color, bg: Color) {
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let glyph = if dx.abs() > dy.abs() * 2.8 {
        '─'
    } else if dy.abs() > dx.abs() {
        '│'
    } else if dx * dy > 0.0 {
        '╲'
    } else {
        '╱'
    };
    let steps = dx.abs().max(dy.abs()).ceil() as usize;
    for i in 0..=steps {
        let f = i as f32 / steps.max(1) as f32;
        put(
            grid,
            a[0] + dx * f,
            a[1] + dy * f,
            Cell::with_bg(glyph, color, bg),
        );
    }
}

#[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
fn draw_tideglass(frame: &mut ModeFrame<'_>, k: &[f32; 14], t: f32) {
    // Resolve stable object identities once, independent of time and draw branches.
    // Draw the sky and suspension, then rear bands, opaque iris, and front bands.
    // Evaluate pendulum positions analytically; reflect completed sky into water.
    // Overlay foreground silhouettes and horizontal foam in bounded grid passes.
    let (w, h) = (frame.width, frame.height);
    if w == 0 || h == 0 {
        return;
    }
    let identity: [f32; 64] = std::array::from_fn(|_| frame.rng.random::<f32>());
    let p = frame.palette.map(|c| shift_hue(c, k[12] as f64));
    let ink = darken(p[0], 8);
    let radius = (w as f32 * 0.40).min(h as f32 * 0.86) * k[0];
    let cx = w as f32 * (0.46 + identity[0] * 0.08);
    let cy = h as f32 * 0.34;
    let water = ((h as f32 * k[8]) as usize).clamp(1, h);
    let phase = identity[1] * TAU;
    let gold = lerp_color(p[3], p[4], 0.3);
    let mist = lerp_color(ink, p[2], 0.18);

    for (y, row) in frame.grid.iter_mut().enumerate() {
        for (x, c) in row.iter_mut().enumerate() {
            let dx = (x as f32 - cx) / radius.max(1.0);
            let dy = (y as f32 - cy) * 2.0 / radius.max(1.0);
            let halo = (-2.2 * (dx * dx + dy * dy)).exp() * k[11];
            let haze = (y as f32 / water as f32).powi(3).min(1.0) * 0.09;
            *c = Cell::with_bg(' ', mist, lerp_color(ink, p[1], halo * 0.32 + haze));
        }
    }
    let stars = ((w * water) as f32 * k[10] * 0.025) as usize;
    for i in 0..stars {
        let a = identity[i % 64];
        let b = identity[(i * 7 + 11) % 64];
        let x = ((a + i as f32 * 0.618034 + t * 0.0015).fract()) * w as f32;
        let y = (b + i as f32 * 0.381966).fract() * water as f32;
        let light = 0.3 + 0.25 * (t * 0.6 + i as f32).sin();
        put(
            frame.grid,
            x,
            y,
            Cell::with_bg(
                if i % 9 == 0 { '+' } else { '·' },
                lerp_color(mist, p[4], light),
                ink,
            ),
        );
    }

    // Two curved load cables retain the composition's large-scale silhouette.
    for side in [-1.0, 1.0] {
        let anchor = [cx + side * radius * 0.8, cy - radius * 0.22];
        let mut prev = [cx + side * radius * 1.1, 0.0];
        for j in 1..=h {
            let f = j as f32 / h as f32;
            let next = [
                cx + side * radius * (1.1 - 0.3 * f + 0.06 * (PI * f).sin()),
                anchor[1] * f,
            ];
            line(frame.grid, prev, next, lerp_color(ink, gold, 0.48), ink);
            prev = next;
        }
        put(
            frame.grid,
            anchor[0],
            anchor[1],
            Cell::with_bg('◆', gold, ink),
        );
    }

    let samples = (radius * 9.0).ceil().max(32.0) as usize;
    // The fixed hoop carries the suspension and hanging vessels as bands turn.
    for j in 0..samples {
        let a = j as f32 * TAU / samples as f32;
        let b = (j + 1) as f32 * TAU / samples as f32;
        line(
            frame.grid,
            [cx + radius * a.cos(), cy + radius * a.sin() * 0.43],
            [cx + radius * b.cos(), cy + radius * b.sin() * 0.43],
            lerp_color(ink, gold, 0.4),
            ink,
        );
    }
    let rings = k[1].round() as usize;
    let teeth = k[5].round() as usize;
    for front in [false, true] {
        for band in 0..rings {
            let r = radius * (1.0 - band as f32 * 0.095);
            let roll = phase + band as f32 * 1.07 + k[3] * (0.7 + t * 0.18);
            let squash = (k[2] + band as f32 * 0.35 + (t * 0.13).sin() * 0.25)
                .cos()
                .abs()
                * 0.65
                + 0.18;
            let project = |a: f32, r: f32| {
                let (u, v) = (a.cos() * r, a.sin() * r * squash);
                [
                    cx + u * roll.cos() - v * roll.sin(),
                    cy + (u * roll.sin() + v * roll.cos()) * 0.5,
                ]
            };
            let base = if band % 2 == 0 { gold } else { p[2] };
            for j in 0..samples {
                let a = j as f32 * TAU / samples as f32;
                if (a.sin() >= 0.0) != front {
                    continue;
                }
                let color = lerp_color(ink, base, if front { 0.7 + 0.3 * a.sin() } else { 0.32 });
                line(
                    frame.grid,
                    project(a, r),
                    project(a + TAU / samples as f32, r),
                    color,
                    ink,
                );
            }
            for j in 0..teeth {
                let a = j as f32 * TAU / teeth as f32 + t * k[3] * 0.22;
                if (a.sin() >= 0.0) != front {
                    continue;
                }
                let color = lerp_color(ink, base, if front { 0.9 } else { 0.4 });
                line(frame.grid, project(a, r), project(a, r * 1.055), color, ink);
                if j % 3 == 0 {
                    let q = project(a, r);
                    put(frame.grid, q[0], q[1], Cell::with_bg('◆', color, ink));
                }
            }
        }
        if front {
            break;
        }

        // Solid faceted lens occludes the rear half of the orbital mechanism.
        let lens = radius * 0.43;
        for (y, row) in frame.grid.iter_mut().enumerate().take(water) {
            for (x, c) in row.iter_mut().enumerate() {
                let dx = (x as f32 - cx) / lens.max(0.1);
                let dy = (y as f32 - cy) * 2.0 / lens.max(0.1);
                let r = dx.hypot(dy);
                if r > 1.07 {
                    continue;
                }
                let a = dy.atan2(dx) + phase + t * 0.12;
                let facet = (a * teeth as f32 * 0.5 + r * 4.0).sin();
                let opening = k[4] * (1.0 + 0.06 * (t * 0.7).sin());
                if r < opening {
                    *c = Cell::with_bg(
                        if dx.abs() < 0.08 { '│' } else { ' ' },
                        lerp_color(ink, p[3], 0.5),
                        ink,
                    );
                } else if r < opening + 0.09 || r > 0.94 {
                    *c = Cell::with_bg(
                        if dy.abs() > dx.abs() { '═' } else { '║' },
                        p[4],
                        lerp_color(ink, gold, k[11] * 0.65),
                    );
                } else {
                    let tone = ((1.0 - r) * 0.6 + facet * 0.15 + 0.35).clamp(0.0, 1.0);
                    let glyphs = ['░', '▒', '▓', '█'];
                    *c = Cell::with_bg(
                        glyphs[(tone * 3.99) as usize],
                        lerp_color(p[1], gold, tone),
                        lerp_color(ink, p[1], k[11] * tone),
                    );
                }
            }
        }
    }

    let pendants = k[6].round() as usize;
    for i in 0..pendants {
        let f = (i as f32 + 0.5) / pendants as f32;
        let a = PI * (0.12 + f * 0.76);
        let anchor = [cx + a.cos() * radius, cy + a.sin() * radius * 0.43];
        let size = (radius * (0.025 + identity[i + 40] * 0.035)).max(0.65);
        let clearance = (water as f32 - size - 1.0 - anchor[1]).max(0.0);
        let length = (h as f32 * (0.07 + identity[i + 4] * 0.13)).min(clearance);
        let swing =
            (t * (0.8 + identity[i + 20] * 0.4) + phase + i as f32 * 0.65).sin() * k[7] * 0.5;
        let bob = [
            anchor[0] + length * swing.sin() * 2.0,
            anchor[1] + length * swing.cos(),
        ];
        line(frame.grid, anchor, bob, lerp_color(ink, gold, 0.45), ink);
        for y in -(size.ceil() as i32)..=size.ceil() as i32 {
            for x in -((size * 2.0).ceil() as i32)..=(size * 2.0).ceil() as i32 {
                let d = x.abs() as f32 * 0.5 + y.abs() as f32;
                if d <= size {
                    put(
                        frame.grid,
                        bob[0] + x as f32,
                        bob[1] + y as f32,
                        Cell::with_bg(
                            if d < size * 0.5 { '◆' } else { '·' },
                            gold,
                            lerp_color(ink, p[3], (1.0 - d / size) * k[11]),
                        ),
                    );
                }
            }
        }
    }

    // Mirror the sky through a depth-dependent horizontal displacement field.
    let (sky, sea) = frame.grid.split_at_mut(water);
    for (depth, row) in sea.iter_mut().enumerate() {
        let d = depth as f32 / (h - water).max(1) as f32;
        let source_y = water.saturating_sub(1 + (depth as f32 * 1.8) as usize);
        for (x, cell) in row.iter_mut().enumerate() {
            let wave = (depth as f32 * 1.7 - t * 1.8 + (x as f32 * 0.08 + t * 0.4).sin()).sin();
            let offset = wave * (1.0 + d * 7.0) * k[9];
            let source_x = (x as f32 + offset).round().clamp(0.0, (w - 1) as f32) as usize;
            let source = sky[source_y][source_x];
            let brightness = (0.68 - d * 0.4) * (0.65 + 0.35 * k[11]);
            let broken = wave > 0.6 - k[9] * 0.5;
            let ch = if source.ch != ' ' && !broken {
                '─'
            } else if wave > 0.96 {
                '·'
            } else {
                ' '
            };
            *cell = Cell::with_bg(
                ch,
                lerp_color(ink, source.fg, brightness),
                lerp_color(ink, source.bg, brightness),
            );
            // Receding banks frame the reflection with dark stepped silhouettes.
            let edge = (x.min(w - 1 - x) as f32) / w as f32;
            let bank = 0.025 + d.powi(2) * 0.13 + 0.015 * (x as f32 * 0.6 + phase).sin();
            if edge < bank {
                *cell = Cell::with_bg(if wave > 0.7 { '─' } else { ' ' }, mist, ink);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{morph::IterateFrameRenderer, render::grid_to_plain};

    #[test]
    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
    fn tideglass_snapshots_and_motion() {
        let k: Vec<_> = PARAMS.iter().map(|p| p.default).collect();
        let mut renderer = IterateFrameRenderer::new("tideglass", 42, "deep", 100, 36).unwrap();
        let a = renderer.render(0.0, Some(&k)).unwrap().clone();
        let b = renderer.render(8.0, Some(&k)).unwrap().clone();
        insta::assert_snapshot!("tideglass_seed42", grid_to_plain(&a).join("\n"));
        insta::assert_snapshot!("tideglass_time8", grid_to_plain(&b).join("\n"));
        assert_ne!(grid_to_plain(&a), grid_to_plain(&b));
        assert_eq!(&a, renderer.render(0.0, Some(&k)).unwrap());
        let mut frozen = k.clone();
        frozen[13] = 0.0;
        assert_eq!(&a, renderer.render(100.0, Some(&frozen)).unwrap());
        for i in 0..k.len() {
            let mut changed = k.clone();
            changed[i] = PARAMS[i].max;
            assert_ne!(
                &b,
                renderer.render(8.0, Some(&changed)).unwrap(),
                "{}",
                PARAMS[i].key
            );
        }
    }

    #[test]
    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
    fn tideglass_extrema_and_rerolls() {
        for (w, h) in [(1, 1), (2, 9), (9, 2), (80, 24)] {
            let mut renderer =
                IterateFrameRenderer::new("tideglass", u64::MAX, "deep", w, h).unwrap();
            for k in [
                PARAMS.iter().map(|p| p.min).collect::<Vec<_>>(),
                PARAMS.iter().map(|p| p.max).collect(),
                vec![f32::NAN; 14],
            ] {
                for t in [0.0, 8.0, -12.0, f32::NAN, f32::INFINITY] {
                    let a = renderer.render(t, Some(&k)).unwrap().clone();
                    assert_eq!(&a, renderer.render(t, Some(&k)).unwrap());
                    assert_eq!(a.len(), h);
                    assert!(a.iter().all(|r| r.len() == w));
                    assert!(
                        a.iter()
                            .flatten()
                            .all(|c| crate::types::char_width(c.ch) == 1)
                    );
                }
            }
        }
        let mut seen = std::collections::HashSet::new();
        for seed in 0..24 {
            let k: Vec<_> = PARAMS
                .iter()
                .map(|p| crate::opts::rand_knob(seed, p))
                .collect();
            let mut renderer =
                IterateFrameRenderer::new("tideglass", seed, "deep", 80, 24).unwrap();
            let a = renderer.render(0.0, Some(&k)).unwrap().clone();
            let b = renderer.render(8.0, Some(&k)).unwrap().clone();
            assert_eq!(&a, renderer.render(0.0, Some(&k)).unwrap());
            if k[13] > 0.0 {
                assert_ne!(grid_to_plain(&a), grid_to_plain(&b));
            }
            assert!(
                seen.insert(grid_to_plain(&a).join("\n")),
                "repeated composition at seed {seed}"
            );
        }
    }
}
