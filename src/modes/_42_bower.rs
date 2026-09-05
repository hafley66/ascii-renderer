use crate::_0_profile::measure_layer;
use crate::color::{darken, lerp_color};
use crate::opts::param_f32;
use crate::opus_1_trees::{Canvas, Plot, Species, grow_species};
use crate::registry::{AnimKind, Mode, ModeFrame, Param};
use crate::types::{Cell, Grid, Rect};
use crossterm::style::Color;
use rand::{SeedableRng, rngs::StdRng};
use std::f32::consts::TAU;

pub(super) struct BowerMode;
pub(super) static MODE: BowerMode = BowerMode;
const PARAMS: &[Param] = &[
    param!("VINES", "curling vine pairs", 3.0, 14.0, 8.0, 1.0),
    param!("FRUIT", "tree jewels", 0.0, 1.0, 0.45, 0.05),
    param!("DETAIL", "tree branching", 0.4, 1.5, 0.85, 0.05),
    param!("SPEED", "lantern and leaf clock", 0.0, 2.0, 0.5, 0.05),
];

// Normalized points keep ornament count independent of render area.
fn put(grid: &mut Grid, p: [f32; 2], ch: char, color: Color) {
    let h = grid.len();
    let w = grid.first().map_or(0, Vec::len);
    if !(0.0..1.0).contains(&p[0]) || !(0.0..1.0).contains(&p[1]) {
        return;
    }
    if let Some(cell) = grid
        .get_mut((p[1] * h as f32) as usize)
        .and_then(|r| r.get_mut((p[0] * w as f32) as usize))
    {
        cell.ch = ch;
        cell.fg = color;
    }
}

fn stroke(grid: &mut Grid, a: [f32; 2], b: [f32; 2], color: Color) {
    let h = grid.len();
    let w = grid.first().map_or(0, Vec::len);
    let dx = (b[0] - a[0]) * w as f32;
    let dy = (b[1] - a[1]) * h as f32;
    let n = (dx.abs().max(dy.abs()).ceil() as usize).clamp(1, (w + h).max(1));
    let ch = if dx.abs() > dy.abs() * 2.2 {
        '─'
    } else if dy.abs() > dx.abs() * 1.5 {
        '│'
    } else if dx * dy > 0.0 {
        '╲'
    } else {
        '╱'
    };
    for i in 0..=n {
        let f = i as f32 / n as f32;
        put(
            grid,
            [a[0] + (b[0] - a[0]) * f, a[1] + (b[1] - a[1]) * f],
            ch,
            color,
        );
    }
}

impl Mode for BowerMode {
    fn name(&self) -> &'static str {
        "bower"
    }
    fn help(&self) -> &'static str {
        "Illuminated garden arch, Opus branching trees, curling vines and hanging lanterns [vines] [fruit] [detail] [speed]"
    }
    fn animation(&self) -> AnimKind {
        AnimKind::Iterate
    }
    fn params(&self) -> &'static [Param] {
        PARAMS
    }
    fn render(&self, frame: &mut ModeFrame<'_>) {
        // Resolve CLI, live values and environment defaults before drawing.
        let read = |i: usize| {
            let p = &PARAMS[i];
            let v = frame
                .args
                .get(i + 4)
                .and_then(|s| s.parse::<f32>().ok())
                .or_else(|| frame.param_values.and_then(|v| v.get(i)).copied())
                .unwrap_or_else(|| param_f32(p.key, p.default));
            if v.is_finite() {
                v.clamp(p.min, p.max)
            } else {
                p.default
            }
        };
        let vines = read(0) as usize;
        let fruit = read(1);
        let detail = read(2);
        let speed = read(3);
        let t = if frame.time.is_finite() {
            frame.time.rem_euclid(3600.0) * speed
        } else {
            0.0
        };
        let (w, h) = (frame.width, frame.height);
        if w == 0 || h == 0 {
            return;
        }
        let ink = darken(frame.palette[0], 12);
        let gold = frame.palette[4];
        let leaf = frame.palette[1];
        let dim = lerp_color(ink, frame.palette[2], 0.42);
        // Row colors avoid per-cell color interpolation and radial functions.
        measure_layer("bower", "vellum", || {
            for (y, row) in frame.grid.iter_mut().enumerate() {
                let bg = lerp_color(ink, frame.palette[0], y as f32 / h as f32 * 0.22);
                row.fill(Cell::with_bg(' ', dim, bg));
            }
        });
        // Reuse Opus venation and phyllotaxis in grid-sized growth canvases.
        measure_layer("bower", "trees", || {
            let cw = (w / 3).max(12);
            let ch = (h * 3 / 5).max(10);
            let mut canvas = Canvas::new(cw, ch);
            let mut lut = Vec::with_capacity(192);
            for color in [
                frame.palette[2],
                gold,
                leaf,
                frame.palette[3],
                frame.palette[2],
                gold,
            ] {
                for tone in 0..32 {
                    lut.push(lerp_color(ink, color, 0.25 + tone as f32 / 42.0));
                }
            }
            for side in 0..2 {
                canvas.reset(cw, ch);
                let mut rng = StdRng::seed_from_u64(
                    frame.seed ^ (side as u64 + 1).wrapping_mul(0x9e3779b97f4a7c15),
                );
                grow_species(
                    if side == 0 {
                        Species::Venation
                    } else {
                        Species::Phyllotaxis
                    },
                    &mut canvas,
                    &Plot {
                        rect: Rect {
                            x: 1,
                            y: 0,
                            w: cw - 2,
                            h: ch,
                        },
                        energy: 0.95,
                        fruit,
                        branch: 0.7,
                        roots: 2,
                        detail,
                        bare: 0.18,
                    },
                    &mut rng,
                );
                let sway = (t * 0.7 + side as f32).sin() * 0.005;
                for y in 0..ch {
                    for x in 0..cw {
                        let c = canvas.cells[y * cw + x];
                        if c.ch == '\0' || c.ch == ' ' {
                            continue;
                        }
                        let rise = 1.0 - y as f32 / ch as f32;
                        let p = [
                            0.10 + side as f32 * 0.48
                                + x as f32 / cw as f32 * 0.32
                                + sway * rise * rise,
                            0.30 + y as f32 / ch as f32 * 0.57,
                        ];
                        put(
                            frame.grid,
                            p,
                            c.ch,
                            lut[c.band as usize * 32 + c.tone as usize],
                        );
                    }
                }
            }
        });
        // Trace nested pointed arches and mirrored acanthus-like spiral stems.
        measure_layer("bower", "tracery", || {
            let steps = (w + h).clamp(80, 6000);
            for ring in 0..3 {
                let inset = ring as f32 * 0.013;
                for side in [-1.0f32, 1.0] {
                    let mut last = [0.5 + side * (0.43 - inset), 0.9];
                    for i in 1..=steps {
                        let u = i as f32 / steps as f32;
                        let p = [
                            0.5 + side * (0.43 - inset) * (1.0 - u * u),
                            0.9 - (0.85 - inset) * u,
                        ];
                        stroke(frame.grid, last, p, if ring == 1 { dim } else { gold });
                        last = p;
                    }
                }
            }
            for side in [-1.0f32, 1.0] {
                for j in 0..vines {
                    let f = (j as f32 + 0.5) / vines as f32;
                    let cy = 0.84 - f * 0.65;
                    let cx = 0.5 + side * (0.40 - 0.26 * f * f);
                    let radius = 0.036 + 0.015 * (f * 3.0).sin();
                    let n = ((w + h) as f32 * radius * 5.0).max(32.0) as usize;
                    let mut last = [cx, cy + radius * 1.7];
                    for i in 0..=n {
                        let u = i as f32 / n as f32;
                        let angle = u * TAU * 1.35;
                        let r = radius * (1.0 - u * 0.88);
                        let p = [cx + side * r * angle.sin(), cy + r * 1.7 * angle.cos()];
                        stroke(frame.grid, last, p, leaf);
                        if i == n / 3 || i == n * 2 / 3 {
                            put(frame.grid, [p[0] + side * 0.008, p[1] - 0.012], '❧', gold);
                        }
                        last = p;
                    }
                    put(frame.grid, last, '◆', frame.palette[3]);
                }
            }
            // Ground terraces and a narrow tiled path converge below the lantern.
            for k in 0..4 {
                let y = 0.9 + k as f32 * 0.022;
                stroke(frame.grid, [0.08, y], [0.92, y], dim);
            }
            for side in [-1.0f32, 1.0] {
                stroke(
                    frame.grid,
                    [0.5 + side * 0.015, 0.71],
                    [0.5 + side * 0.13, 0.99],
                    gold,
                );
            }
        });
        // Analytic pendant motion and seed-stable drifting pollen.
        measure_layer("bower", "lanterns", || {
            for j in 0..3 {
                let cx = 0.5 + (j as f32 - 1.0) * 0.13;
                let top = if j == 1 { 0.12 } else { 0.23 };
                let bottom = if j == 1 { 0.49 } else { 0.38 };
                let x = cx + (t * 0.8 + j as f32).sin() * 0.008;
                stroke(frame.grid, [cx, top], [x, bottom - 0.045], dim);
                let diamond = [
                    [x, bottom - 0.065],
                    [x + 0.024, bottom],
                    [x, bottom + 0.065],
                    [x - 0.024, bottom],
                    [x, bottom - 0.065],
                ];
                for pair in diamond.windows(2) {
                    stroke(frame.grid, pair[0], pair[1], gold);
                }
                put(frame.grid, [x, bottom], '✦', gold);
                put(frame.grid, [x, bottom + 0.08], '◦', frame.palette[3]);
            }
            for i in 0..(w * h / 180).clamp(6, 180) {
                let n = frame
                    .seed
                    .wrapping_add(i as u64 * 7919)
                    .wrapping_mul(0xbf58476d1ce4e5b9);
                let x = 0.15 + ((n >> 32) % 1000) as f32 / 1000.0 * 0.7;
                let y = 0.22 + (((n % 1000) as f32 / 1000.0 + t * 0.025).rem_euclid(1.0)) * 0.64;
                put(frame.grid, [x + (t + i as f32).sin() * 0.008, y], '·', gold);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{morph::IterateFrameRenderer, render::grid_to_plain};
    #[test]
    fn bower_snapshots_and_animation() {
        let mut p = IterateFrameRenderer::new("bower", 42, "moss", 100, 36).unwrap();
        let values: Vec<_> = PARAMS.iter().map(|p| p.default).collect();
        let first = p.render(0.0, Some(&values)).unwrap().clone();
        let moved = p.render(5.0, Some(&values)).unwrap().clone();
        insta::assert_snapshot!("bower_seed_42", grid_to_plain(&first).join("\n"));
        insta::assert_snapshot!("bower_motion", grid_to_plain(&moved).join("\n"));
        assert_ne!(first, moved);
        assert_eq!(&first, p.render(0.0, Some(&values)).unwrap());
        let mut stopped = values;
        stopped[3] = 0.0;
        assert_eq!(&first, p.render(100.0, Some(&stopped)).unwrap());
    }
    #[test]
    fn bower_extremes() {
        for (w, h) in [(1, 1), (9, 2), (2, 9), (80, 24)] {
            let mut p = IterateFrameRenderer::new("bower", u64::MAX, "moss", w, h).unwrap();
            for values in [
                PARAMS.iter().map(|p| p.min).collect::<Vec<_>>(),
                PARAMS.iter().map(|p| p.max).collect(),
                vec![f32::NAN; 4],
            ] {
                for t in [0.0, 4.0, f32::NAN, f32::MAX] {
                    let g = p.render(t, Some(&values)).unwrap();
                    assert_eq!(g.len(), h);
                    assert!(
                        grid_to_plain(g)
                            .iter()
                            .all(|r| crate::types::display_width(r) == w)
                    );
                }
            }
        }
    }
}
