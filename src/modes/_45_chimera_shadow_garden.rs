use crate::color::{darken, lerp_color};
use crate::opts::param_f32;
use crate::registry::{AnimKind, Mode, ModeFrame, Param};
use crate::types::{Cell, Grid};
use crossterm::style::Color;
use std::f32::consts::{PI, TAU};

pub(super) struct ChimeraShadowGardenMode;
pub(super) static MODE: ChimeraShadowGardenMode = ChimeraShadowGardenMode;
const TITLE: &str = "domain expansion: chimera shadow garden";
const PARAMS: &[Param] = &[
    param!("GROWTH", "garden growth", 0.0, 1.5, 0.8, 0.1),
    param!("CROWN", "chimera crown span", 0.4, 1.4, 1.0, 0.1),
    param!("TIDE", "pool ripples", 0.0, 1.0, 0.55, 0.05),
    param!("SPORES", "drifting spores", 0.0, 1.0, 0.45, 0.05),
    param!("SWAY", "stem sway", 0.0, 1.0, 0.6, 0.05),
    param!("SPEED", "animation speed", 0.0, 2.0, 0.6, 0.1),
];

impl Mode for ChimeraShadowGardenMode {
    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
    fn name(&self) -> &'static str {
        "chimera-shadow-garden"
    }
    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
    fn help(&self) -> &'static str {
        TITLE
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
        // Resolve positional arguments, live controls, and defaults.
        let k = std::array::from_fn(|i| {
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
        });
        // Pass the borrowed frame and controls to the renderer.
        draw_chimera_shadow_garden(frame, &k);
    }
}

// Stable identities keep plants and spores fixed when another control changes.
#[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
fn noise(seed: u64, id: usize) -> f32 {
    let mut n = seed.wrapping_add((id as u64).wrapping_mul(0x9e3779b97f4a7c15));
    n = (n ^ (n >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    n = (n ^ (n >> 27)).wrapping_mul(0x94d049bb133111eb);
    ((n ^ (n >> 31)) >> 40) as f32 / 16777216.0
}

#[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
fn put(grid: &mut Grid, p: [f32; 2], ch: char, fg: Color) {
    if !(0.0..1.0).contains(&p[0]) || !(0.0..1.0).contains(&p[1]) {
        return;
    }
    let h = grid.len();
    let w = grid.first().map_or(0, Vec::len);
    if let Some(c) = grid
        .get_mut((p[1] * h as f32) as usize)
        .and_then(|row| row.get_mut((p[0] * w as f32) as usize))
    {
        c.ch = ch;
        c.fg = fg;
    }
}

// Sample normalized curves at cell resolution, with a grid-sized work bound.
#[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
fn curve(grid: &mut Grid, a: [f32; 2], b: [f32; 2], c: [f32; 2], fg: Color) {
    let h = grid.len();
    let w = grid.first().map_or(0, Vec::len);
    let length = ((a[0] - b[0]).abs() + (b[0] - c[0]).abs()) * w as f32
        + ((a[1] - b[1]).abs() + (b[1] - c[1]).abs()) * h as f32;
    let steps = (length.ceil() as usize * 2).clamp(1, 4 * (w + h).max(1));
    for j in 0..=steps {
        let u = j as f32 / steps as f32;
        let v = 1.0 - u;
        let dx = ((b[0] - a[0]) * v + (c[0] - b[0]) * u) * w as f32;
        let dy = ((b[1] - a[1]) * v + (c[1] - b[1]) * u) * h as f32;
        let ch = if dx.abs() > dy.abs() * 2.3 {
            '_'
        } else if dy.abs() > dx.abs() * 1.5 {
            '|'
        } else if dx * dy > 0.0 {
            '\\'
        } else {
            '/'
        };
        put(
            grid,
            [
                v * v * a[0] + 2.0 * v * u * b[0] + u * u * c[0],
                v * v * a[1] + 2.0 * v * u * b[1] + u * u * c[1],
            ],
            ch,
            fg,
        );
    }
}

#[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
fn draw_chimera_shadow_garden(frame: &mut ModeFrame<'_>, k: &[f32; 6]) {
    // Initialize depth colors and seed-stable object positions.
    // Evaluate sway, ripples, and drifting spores from time.
    // Draw the arch, pool, silhouette, plants, and title in layer order.
    let (w, h) = (frame.width, frame.height);
    if w == 0 || h == 0 {
        return;
    }
    let t = if frame.time.is_finite() {
        frame.time * k[5]
    } else {
        0.0
    };
    // Compute phases in f64 so finite, extreme f32 inputs remain finite.
    let t = if t.is_finite() {
        t as f64
    } else {
        frame.time as f64 * k[5] as f64
    };
    let phase = |speed: f64, offset: f32| (t * speed + offset as f64).rem_euclid(TAU as f64) as f32;
    let ink = darken(frame.palette[0], 8);
    let far = lerp_color(ink, frame.palette[2], 0.42);
    let middle = lerp_color(ink, frame.palette[1], 0.72);
    let rim = frame.palette[3];
    let light = frame.palette[4];
    let grid = &mut *frame.grid;
    for (y, row) in grid.iter_mut().enumerate() {
        let depth = y as f32 / h as f32;
        row.fill(Cell::with_bg(
            ' ',
            far,
            lerp_color(ink, frame.palette[2], depth * 0.07),
        ));
    }

    // Nested distant vaults leave an open pocket around the central figure.
    for ring in 0..3 {
        let rx = 0.40 + ring as f32 * 0.035;
        let ry = 0.47 + ring as f32 * 0.025;
        for j in 0..=w + h {
            let a = PI + PI * j as f32 / (w + h) as f32;
            let ch = if a.sin().abs() > 0.9 {
                '_'
            } else if a.cos() > 0.0 {
                '\\'
            } else {
                '/'
            };
            put(grid, [0.5 + rx * a.cos(), 0.60 + ry * a.sin()], ch, far);
        }
        for side in [-1.0, 1.0] {
            curve(
                grid,
                [0.5 + side * rx, 0.60],
                [0.5 + side * rx, 0.66],
                [0.5 + side * rx, 0.71],
                far,
            );
        }
    }

    // Perspective ripples and a broken reflection occupy the lower third.
    let waterline = (h as f32 * 0.68) as usize;
    for (y, row) in grid.iter_mut().enumerate().skip(waterline) {
        let depth = (y as f32 / h as f32 - 0.68) / 0.32;
        for (x, cell) in row.iter_mut().enumerate() {
            let nx = (x as f32 / w as f32 - 0.5) / (0.25 + depth * 0.8);
            let wave = (nx * nx * 6.0 + depth * 26.0 - phase(1.5, 0.0)).sin();
            if wave > 1.0 - k[2] * 0.22 {
                cell.ch = if depth > 0.35 { '~' } else { '-' };
                cell.fg = lerp_color(far, rim, depth * 0.3);
            }
            let drift = phase(1.2, depth * 18.0).sin() * k[2] * 0.045;
            if (nx + drift).abs() < 0.05 + 0.11 * (1.0 - depth) && (y % 2 == 0 || wave > 0.5) {
                cell.ch = if x % 3 == 0 { ':' } else { '.' };
                cell.fg = middle;
            }
        }
    }

    // Seeded garden layers: short distant shoots, tall foreground flowers.
    for layer in 0..2 {
        let count = ((8.0 + k[0] * 20.0) as usize).min(w / 2 + 1);
        for i in 0..count {
            let id = 100 + layer * 200 + i * 5;
            let x = 0.02 + noise(frame.seed, id) * 0.96;
            if (x - 0.5).abs() < 0.13 + layer as f32 * 0.06 {
                continue;
            }
            let base = 0.70 + layer as f32 * 0.27;
            let length = (0.05 + noise(frame.seed, id + 1) * 0.17)
                * (0.35 + k[0])
                * (0.7 + layer as f32 * 0.3);
            let bend = phase(0.75, noise(frame.seed, id + 2) * TAU).sin() * k[4] * 0.025;
            let tip = [x + bend, base - length];
            let fg = if layer == 0 { far } else { middle };
            curve(grid, [x, base], [x - bend, base - length * 0.5], tip, fg);
            for side in [-1.0, 1.0] {
                curve(
                    grid,
                    [x, base - length * 0.28],
                    [x + side * 0.035, base - length * 0.55],
                    [x + side * 0.025, base - length * 0.65],
                    fg,
                );
            }
            put(
                grid,
                tip,
                if layer == 0 { '*' } else { '@' },
                if layer == 0 { middle } else { rim },
            );
            if layer == 1 && w >= 60 {
                put(grid, [tip[0] - 1.0 / w as f32, tip[1]], '(', middle);
                put(grid, [tip[0] + 1.0 / w as f32, tip[1]], ')', middle);
            }
        }
    }

    // A hollow crown, paired horn-petals, ribs, and spreading root feet.
    let sway = phase(0.7, 0.0).sin() * k[4] * 0.018;
    let cx = 0.5 + sway;
    for side in [-1.0, 1.0] {
        for rib in 0..4 {
            let r = rib as f32;
            let tip = [cx + side * (0.08 + r * 0.034) * k[1], 0.24 + r * 0.075];
            curve(
                grid,
                [cx + side * 0.016, 0.61 - r * 0.022],
                [cx + side * (0.19 + r * 0.024) * k[1], 0.49 - r * 0.014],
                tip,
                middle,
            );
            curve(
                grid,
                tip,
                [cx + side * 0.06 * k[1], 0.40 + r * 0.027],
                [cx + side * 0.016, 0.61 - r * 0.022],
                rim,
            );
        }
        curve(
            grid,
            [cx + side * 0.027, 0.43],
            [cx + side * 0.14 * k[1], 0.24],
            [cx + side * 0.035 * k[1], 0.19],
            rim,
        );
        curve(
            grid,
            [cx + side * 0.035 * k[1], 0.19],
            [cx + side * 0.18 * k[1], 0.32],
            [cx + side * 0.045, 0.47],
            middle,
        );
        curve(
            grid,
            [cx + side * 0.033, 0.44],
            [cx + side * 0.053, 0.56],
            [0.5 + side * 0.015, 0.66],
            rim,
        );
        for root in 0..3 {
            let r = root as f32;
            curve(
                grid,
                [0.5 + side * 0.015, 0.63],
                [0.5 + side * (0.055 + r * 0.025), 0.70],
                [0.5 + side * (0.085 + r * 0.045), 0.68 + r * 0.025],
                middle,
            );
        }
    }
    // Clear the mask interior so its eyes remain legible over the ribs.
    for y in (h as f32 * 0.39) as usize..((h as f32 * 0.48).ceil() as usize).min(h) {
        for x in ((cx - 0.034) * w as f32) as usize..(((cx + 0.034) * w as f32) as usize).min(w) {
            grid[y][x].ch = ' ';
        }
    }
    put(grid, [cx - 0.024, 0.425], '<', rim);
    put(grid, [cx + 0.024, 0.425], '>', rim);
    put(grid, [cx - 0.012, 0.425], 'o', light);
    put(grid, [cx + 0.012, 0.425], 'o', light);
    put(grid, [cx, 0.465], 'V', rim);
    put(grid, [cx, 0.515], ':', light);
    put(grid, [cx, 0.56], ':', rim);

    // Drifting points only occupy empty cells, preserving silhouette contours.
    let spores = ((k[3] * 110.0) as usize).min(w.saturating_mul(h) / 12);
    for i in 0..spores {
        let id = 1000 + i * 3;
        let x = (noise(frame.seed, id) + phase(0.09, i as f32) / TAU).fract();
        let y = 0.16 + 0.69 * (noise(frame.seed, id + 1) - phase(0.16, 0.0) / TAU).rem_euclid(1.0);
        let cell = &mut grid[(y * h as f32) as usize][(x * w as f32) as usize];
        if cell.ch == ' ' {
            cell.ch = if noise(frame.seed, id + 2) > 0.8 {
                '+'
            } else {
                '.'
            };
            cell.fg = if i % 3 == 0 { rim } else { far };
        }
    }
    if w >= TITLE.len() + 4 && h >= 12 {
        let start = (w - TITLE.len()) / 2;
        let row = &mut grid[h - 1];
        row.fill(Cell::with_bg(' ', far, ink));
        for (i, ch) in TITLE.chars().enumerate() {
            row[start + i] = Cell::with_bg(ch, middle, ink);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{morph::IterateFrameRenderer, render::grid_to_plain};

    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
    fn defaults() -> Vec<f32> {
        PARAMS.iter().map(|p| p.default).collect()
    }

    #[test]
    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
    fn chimera_shadow_garden_static_snapshot() {
        let mut renderer = IterateFrameRenderer::new(MODE.name(), 42, "deep", 80, 28).unwrap();
        insta::assert_snapshot!(
            "chimera_shadow_garden_seed42",
            grid_to_plain(renderer.render(0.0, Some(&defaults())).unwrap()).join("\n")
        );
    }

    #[test]
    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
    fn chimera_shadow_garden_motion_snapshot() {
        let mut renderer = IterateFrameRenderer::new(MODE.name(), 42, "deep", 80, 28).unwrap();
        insta::assert_snapshot!(
            "chimera_shadow_garden_time5",
            grid_to_plain(renderer.render(5.0, Some(&defaults())).unwrap()).join("\n")
        );
    }

    #[test]
    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
    fn chimera_shadow_garden_controls_and_replay() {
        let mut renderer = IterateFrameRenderer::new(MODE.name(), 42, "deep", 100, 36).unwrap();
        let k = defaults();
        let a = renderer.render(0.0, Some(&k)).unwrap().clone();
        let b = renderer.render(5.0, Some(&k)).unwrap().clone();
        assert_ne!(grid_to_plain(&a), grid_to_plain(&b));
        assert_eq!(&a, renderer.render(0.0, Some(&k)).unwrap());
        assert_eq!(&b, renderer.render(5.0, Some(&k)).unwrap());
        for i in 0..k.len() {
            let mut changed = k.clone();
            changed[i] = PARAMS[i].max;
            assert_ne!(
                grid_to_plain(&b),
                grid_to_plain(renderer.render(5.0, Some(&changed)).unwrap()),
                "{}",
                PARAMS[i].key
            );
        }
        let mut stopped = k.clone();
        stopped[5] = 0.0;
        assert_eq!(&a, renderer.render(80.0, Some(&stopped)).unwrap());
        let mut other = IterateFrameRenderer::new(MODE.name(), 43, "deep", 100, 36).unwrap();
        assert_ne!(&a, other.render(0.0, Some(&k)).unwrap());
        let mut themed = IterateFrameRenderer::new(MODE.name(), 42, "moss", 100, 36).unwrap();
        assert_ne!(&a, themed.render(0.0, Some(&k)).unwrap());
    }

    #[test]
    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
    fn chimera_shadow_garden_boundaries_and_cli() {
        use rand::{SeedableRng, rngs::StdRng};
        for (w, h) in [(0, 0), (0, 3), (3, 0), (1, 1), (2, 9), (9, 2), (80, 28)] {
            for k in [
                defaults(),
                PARAMS.iter().map(|p| p.min).collect(),
                PARAMS.iter().map(|p| p.max).collect(),
                vec![f32::NAN; 6],
            ] {
                for t in [0.0, 5.0, -19.0, f32::NAN, f32::INFINITY, f32::MAX] {
                    let mut grid = vec![vec![Cell::blank(); w]; h];
                    let palette = crate::color::named_theme("deep").unwrap();
                    let mut rng = StdRng::seed_from_u64(u64::MAX);
                    let mut frame = ModeFrame {
                        grid: &mut grid,
                        width: w,
                        height: h,
                        seed: u64::MAX,
                        palette: &palette,
                        rng: &mut rng,
                        time: t,
                        args: &[],
                        param_values: Some(&k),
                    };
                    MODE.render(&mut frame);
                    let a = frame.grid.clone();
                    MODE.render(&mut frame);
                    assert_eq!(a, *frame.grid);
                    assert_eq!(grid.len(), h);
                    assert!(grid.iter().all(|r| r.len() == w));
                    assert!(grid.iter().flatten().all(|c| c.ch.is_ascii()));
                }
            }
        }
        let k = defaults();
        let mut renderer = IterateFrameRenderer::new(MODE.name(), 42, "deep", 80, 28).unwrap();
        let expected = renderer.render(0.0, Some(&k)).unwrap().clone();
        let mut grid = vec![vec![Cell::blank(); 80]; 28];
        let palette = crate::color::named_theme("deep").unwrap();
        let mut rng = StdRng::seed_from_u64(42);
        let args: Vec<String> = ["ascii-renderer", "42", MODE.name(), "deep"]
            .into_iter()
            .map(String::from)
            .chain(k.iter().map(f32::to_string))
            .collect();
        MODE.render(&mut ModeFrame {
            grid: &mut grid,
            width: 80,
            height: 28,
            seed: 42,
            palette: &palette,
            rng: &mut rng,
            time: 0.0,
            args: &args,
            param_values: Some(&[0.0; 6]),
        });
        assert_eq!(expected, grid);
    }
}
