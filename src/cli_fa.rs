use crossterm::style::Color;
use rand::RngExt;
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::io::{self, IsTerminal, Read as _};

use crate::automata::*;
use crate::biomes::*;
use crate::color::*;
use crate::content::*;
use crate::fills::*;
use crate::layout::*;
use crate::markdown::*;
use crate::mondrian::*;
use crate::render::*;
use crate::scene::*;
use crate::sprites::*;
use crate::tree_draw::*;
use crate::types::*;
use crate::walker::*;
use crate::avant::*;
use crate::cli::*;
use crate::gridio::*;
use crate::ink::*;
use crate::modes_creatures::*;
use crate::modes_geo::*;
use crate::modes_sky::*;
use crate::modes_tree::*;
use crate::morph::*;
use crate::opts::*;
use crate::pp::*;
use crate::registry::*;
use crate::warps::*;
use crate::automata; use crate::avant; use crate::biomes; use crate::borders; use crate::color; use crate::content; use crate::fills; use crate::layout; use crate::markdown; use crate::mondrian; use crate::render; use crate::scene; use crate::sprites; use crate::tree_draw; use crate::types; use crate::walker;


/// Dispatch arm for mode(s): fullmetal-eyes (moved verbatim from run()).
pub(crate) fn cli_fullmetal_eyes(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        grid = draw_fullmetal_eyes(grid, width, height, seed, palette, rng, t_anim, &args);
    (grid, false)
}

/// Dispatch arm for mode(s): fullmetal-eyes2 (moved verbatim from run()).
pub(crate) fn cli_fullmetal_eyes2(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        grid = draw_fullmetal_eyes2(grid, width, height, seed, palette, rng, t_anim, &args);
    (grid, false)
}

/// Dispatch arm for mode(s): fullmetal-alchemist (moved verbatim from run()).
pub(crate) fn cli_fullmetal_alchemist(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        // fullmetal-alchemist [rings] [glyphs] -- original generative alchemical sealwork
        let ring_count: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(5);
        let ring_count = ring_count.clamp(2, 9);
        let glyph_count: usize = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(64);
        let glyph_count = glyph_count.clamp(0, 180);
        let chord_count: usize = args.get(6).and_then(|s| s.parse().ok()).unwrap_or(28);
        let chord_count = chord_count.clamp(0, 96);

        let bg = darken(palette[0], 8);
        let chalk = lighten(palette[4], 8);
        let gold = lighten(palette[1], 32);
        let ember = lighten(palette[3], 24);
        let shadow = darken(palette[2], 36);

        for y in 0..height {
            for x in 0..width {
                let n = (x * 17 + y * 31 + seed as usize) % 89;
                let ch = match n {
                    0 => '·',
                    1 => '∙',
                    2 => '°',
                    3 => '\'',
                    _ => ' ',
                };
                grid[y][x] = if ch == ' ' {
                    Cell::new(' ', bg)
                } else {
                    Cell::new(ch, shadow)
                };
            }
        }

        let put = |grid: &mut Grid, x: i32, y: i32, ch: char, fg: Color| {
            if x >= 0 && y >= 0 && (x as usize) < width && (y as usize) < height {
                grid[y as usize][x as usize] = Cell::new(ch, fg);
            }
        };
        let stroke_char = |x0: i32, y0: i32, x1: i32, y1: i32| {
            let dx = x1 - x0;
            let dy = y1 - y0;
            if dx.abs() > dy.abs() * 2 {
                '─'
            } else if dy.abs() > dx.abs() * 2 {
                '│'
            } else if dx.signum() == dy.signum() {
                '╲'
            } else {
                '╱'
            }
        };
        let draw_line = |grid: &mut Grid, mut x0: i32, mut y0: i32, x1: i32, y1: i32, fg: Color| {
            let ch = stroke_char(x0, y0, x1, y1);
            let dx = (x1 - x0).abs();
            let sx = if x0 < x1 { 1 } else { -1 };
            let dy = -(y1 - y0).abs();
            let sy = if y0 < y1 { 1 } else { -1 };
            let mut err = dx + dy;
            loop {
                put(grid, x0, y0, ch, fg);
                if x0 == x1 && y0 == y1 {
                    break;
                }
                let e2 = 2 * err;
                if e2 >= dy {
                    err += dy;
                    x0 += sx;
                }
                if e2 <= dx {
                    err += dx;
                    y0 += sy;
                }
            }
        };
        let point_on = |cx: i32, cy: i32, rx: f32, ry: f32, angle: f32| {
            (
                cx + (angle.cos() * rx).round() as i32,
                cy + (angle.sin() * ry).round() as i32,
            )
        };
        let draw_ellipse = |grid: &mut Grid,
                            cx: i32,
                            cy: i32,
                            rx: f32,
                            ry: f32,
                            fg: Color,
                            phase: f32,
                            dotted: bool| {
            let samples = ((rx + ry) * 16.0).max(96.0) as usize;
            let mut prev: Option<(i32, i32)> = None;
            for i in 0..=samples {
                if dotted && i % 5 == 3 {
                    prev = None;
                    continue;
                }
                let a = phase + i as f32 / samples as f32 * std::f32::consts::TAU;
                let p = point_on(cx, cy, rx, ry, a);
                if let Some(q) = prev {
                    let ch = stroke_char(q.0, q.1, p.0, p.1);
                    draw_line(grid, q.0, q.1, p.0, p.1, fg);
                    put(grid, p.0, p.1, ch, fg);
                } else {
                    put(grid, p.0, p.1, '·', fg);
                }
                prev = Some(p);
            }
        };
        let draw_poly = |grid: &mut Grid,
                         cx: i32,
                         cy: i32,
                         rx: f32,
                         ry: f32,
                         sides: usize,
                         phase: f32,
                         skip: usize,
                         fg: Color| {
            let mut pts = Vec::new();
            for i in 0..sides {
                let a = phase + i as f32 / sides as f32 * std::f32::consts::TAU;
                pts.push(point_on(cx, cy, rx, ry, a));
            }
            let skip = skip.max(1).min(sides - 1);
            for i in 0..sides {
                let j = (i + skip) % sides;
                draw_line(grid, pts[i].0, pts[i].1, pts[j].0, pts[j].1, fg);
            }
        };

        let cx = width as i32 / 2;
        let cy = height as i32 / 2;
        let max_rx = (width as f32 / 2.0 - 4.0).max(8.0);
        let max_ry = (height as f32 / 2.0 - 2.0).max(4.0);

        for i in 0..ring_count {
            let t = i as f32 / ring_count.max(1) as f32;
            let rx = max_rx * (1.0 - t * 0.68);
            let ry = max_ry * (1.0 - t * 0.68);
            let fg = if i % 2 == 0 {
                shift_hue(gold, i as f64 * 21.0)
            } else {
                shift_hue(chalk, i as f64 * -18.0)
            };
            draw_ellipse(&mut grid, cx, cy, rx, ry, fg, i as f32 * 0.11, i % 3 == 2);
        }

        let phase = rng.random::<f32>() * std::f32::consts::TAU;
        draw_poly(
            &mut grid,
            cx,
            cy,
            max_rx * 0.84,
            max_ry * 0.84,
            3,
            phase,
            1,
            ember,
        );
        draw_poly(
            &mut grid,
            cx,
            cy,
            max_rx * 0.72,
            max_ry * 0.72,
            6,
            phase + std::f32::consts::PI / 6.0,
            2,
            lighten(chalk, 8),
        );
        draw_poly(
            &mut grid,
            cx,
            cy,
            max_rx * 0.54,
            max_ry * 0.54,
            5 + (seed as usize % 3),
            phase * 0.5,
            2,
            shift_hue(gold, 70.0),
        );

        for _ in 0..chord_count {
            let a = rng.random::<f32>() * std::f32::consts::TAU;
            let b = a + rng.random_range(2.0..5.2) + rng.random_range(-0.35..0.35);
            let r = rng.random_range(0.34..0.96);
            let p1 = point_on(cx, cy, max_rx * r, max_ry * r, a);
            let p2 = point_on(cx, cy, max_rx * r, max_ry * r, b);
            let color = if rng.random::<f32>() < 0.55 {
                darken(chalk, rng.random_range(0..38))
            } else {
                darken(ember, rng.random_range(0..32))
            };
            draw_line(&mut grid, p1.0, p1.1, p2.0, p2.1, color);
        }

        let runes = [
            '△', '▽', '□', '◇', '○', '☉', '☽', '☿', '♄', '♃', '♁', '✶', '✦', '✧', '+', '×', '≡',
            '∴', '∵',
        ];
        for i in 0..glyph_count {
            let lane = match i % 4 {
                0 => 0.96,
                1 => 0.81,
                2 => 0.59,
                _ => rng.random_range(0.30..0.92),
            };
            let a = i as f32 / glyph_count.max(1) as f32 * std::f32::consts::TAU
                + rng.random_range(-0.045..0.045)
                + phase * 0.13;
            let (x, y) = point_on(cx, cy, max_rx * lane, max_ry * lane, a);
            let glyph = runes[rng.random_range(0..runes.len())];
            put(
                &mut grid,
                x,
                y,
                glyph,
                shift_hue(lighten(gold, 8), rng.random_range(-80..=90) as f64),
            );
        }

        let walker_count = (ring_count + glyph_count / 30).clamp(4, 14);
        for w in 0..walker_count {
            let mut angle = phase + w as f32 * 1.37;
            let mut radius = rng.random_range(0.16..0.42);
            let mut prev = point_on(cx, cy, max_rx * radius, max_ry * radius, angle);
            let steps = rng.random_range(8..22);
            for s in 0..steps {
                angle += rng.random_range(-0.55..0.72);
                radius = (radius + rng.random_range(-0.04..0.09)).clamp(0.10, 0.74);
                let next = point_on(cx, cy, max_rx * radius, max_ry * radius, angle);
                let color = if s % 3 == 0 { ember } else { darken(chalk, 18) };
                draw_line(&mut grid, prev.0, prev.1, next.0, next.1, color);
                if rng.random::<f32>() < 0.42 {
                    put(
                        &mut grid,
                        next.0,
                        next.1,
                        runes[rng.random_range(0..runes.len())],
                        gold,
                    );
                }
                prev = next;
            }
        }

        let anchors = [
            (-std::f32::consts::FRAC_PI_2, '△'),
            (0.0, '☉'),
            (std::f32::consts::FRAC_PI_2, '▽'),
            (std::f32::consts::PI, '□'),
        ];
        for &(a, ch) in &anchors {
            let outer = point_on(cx, cy, max_rx + 1.0, max_ry + 1.0, a);
            let inner = point_on(cx, cy, max_rx * 0.88, max_ry * 0.88, a);
            draw_line(&mut grid, inner.0, inner.1, outer.0, outer.1, gold);
            put(&mut grid, outer.0, outer.1, ch, lighten(chalk, 15));
        }

        for dy in -2i32..=2i32 {
            for dx in -4i32..=4i32 {
                let metric = (dx as f32 / 4.0).powi(2) + (dy as f32 / 2.0).powi(2);
                if metric <= 1.0 {
                    let ch = if dx == 0 && dy == 0 {
                        '☉'
                    } else if dx.abs() == dy.abs() {
                        '╳'
                    } else if dy == 0 {
                        '═'
                    } else if dx == 0 {
                        '║'
                    } else {
                        '·'
                    };
                    put(&mut grid, cx + dx, cy + dy, ch, lighten(ember, 18));
                }
            }
        }
    (grid, false)
}

/// Dispatch arm for mode(s): fullmetal-alchemist2 (moved verbatim from run()).
pub(crate) fn cli_fullmetal_alchemist2(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        // fullmetal-alchemist2 [nodes=0] [runes] [fractures] -- node-first ritual geometry
        let node_arg: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(0);
        let node_count = if node_arg == 0 {
            5 + ((seed as usize * 37 + 11) % 7)
        } else {
            node_arg.clamp(5, 14)
        };
        let rune_count: usize = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(72);
        let rune_count = rune_count.clamp(16, 240);
        let fracture_count: usize = args.get(6).and_then(|s| s.parse().ok()).unwrap_or(7);
        let fracture_count = fracture_count.clamp(0, 80);

        let bg = darken(palette[0], 10);
        let chalk = lighten(palette[4], 10);
        let gold = lighten(palette[1], 34);
        let ether = shift_hue(lighten(palette[3], 32), 42.0);
        let blood = shift_hue(lighten(palette[2], 42), -30.0);
        let hush = darken(palette[2], 55);

        for y in 0..height {
            for x in 0..width {
                let n = (x * 23 + y * 41 + seed as usize * 5) % 113;
                let ch = match n {
                    0 => '·',
                    1 => '∙',
                    2 => '°',
                    3 => '\'',
                    4 => '`',
                    _ => ' ',
                };
                grid[y][x] = if ch == ' ' {
                    Cell::new(' ', bg)
                } else {
                    Cell::new(ch, hush)
                };
            }
        }

        let put = |grid: &mut Grid, x: i32, y: i32, ch: char, fg: Color| {
            if x >= 0 && y >= 0 && (x as usize) < width && (y as usize) < height {
                grid[y as usize][x as usize] = Cell::new(ch, fg);
            }
        };
        let blank = |grid: &mut Grid, x: i32, y: i32| {
            if x >= 0 && y >= 0 && (x as usize) < width && (y as usize) < height {
                grid[y as usize][x as usize] = Cell::blank();
            }
        };
        let stroke_char = |x0: i32, y0: i32, x1: i32, y1: i32| {
            let dx = x1 - x0;
            let dy = y1 - y0;
            if dx.abs() > dy.abs() * 2 {
                '─'
            } else if dy.abs() > dx.abs() * 2 {
                '│'
            } else if dx.signum() == dy.signum() {
                '╲'
            } else {
                '╱'
            }
        };
        let draw_line = |grid: &mut Grid, mut x0: i32, mut y0: i32, x1: i32, y1: i32, fg: Color| {
            let dx = (x1 - x0).abs();
            let sx = if x0 < x1 { 1 } else { -1 };
            let dy = -(y1 - y0).abs();
            let sy = if y0 < y1 { 1 } else { -1 };
            let ch = stroke_char(x0, y0, x1, y1);
            let mut err = dx + dy;
            loop {
                put(grid, x0, y0, ch, fg);
                if x0 == x1 && y0 == y1 {
                    break;
                }
                let e2 = 2 * err;
                if e2 >= dy {
                    err += dy;
                    x0 += sx;
                }
                if e2 <= dx {
                    err += dx;
                    y0 += sy;
                }
            }
        };
        let point_on = |cx: i32, cy: i32, rx: f32, ry: f32, angle: f32| {
            (
                cx + (angle.cos() * rx).round() as i32,
                cy + (angle.sin() * ry).round() as i32,
            )
        };
        let draw_arc = |grid: &mut Grid,
                        cx: i32,
                        cy: i32,
                        rx: f32,
                        ry: f32,
                        start: f32,
                        end: f32,
                        fg: Color,
                        puncture: usize| {
            let span = (end - start).abs().max(0.05);
            let samples = ((rx + ry) * span * 3.4).max(12.0) as usize;
            let mut prev: Option<(i32, i32)> = None;
            for i in 0..=samples {
                if puncture > 0 && i % puncture == puncture - 1 {
                    prev = None;
                    continue;
                }
                let a = start + (end - start) * i as f32 / samples as f32;
                let p = point_on(cx, cy, rx, ry, a);
                if let Some(q) = prev {
                    draw_line(grid, q.0, q.1, p.0, p.1, fg);
                } else {
                    put(grid, p.0, p.1, '·', fg);
                }
                prev = Some(p);
            }
        };
        let draw_poly = |grid: &mut Grid, pts: &[(i32, i32)], skip: usize, fg: Color| {
            if pts.len() < 2 {
                return;
            }
            let skip = skip.max(1).min(pts.len() - 1);
            for i in 0..pts.len() {
                let j = (i + skip) % pts.len();
                draw_line(grid, pts[i].0, pts[i].1, pts[j].0, pts[j].1, fg);
            }
        };

        let cx = width as i32 / 2;
        let cy = height as i32 / 2;
        let max_rx = (width as f32 / 2.0 - 3.0).max(10.0);
        let max_ry = (height as f32 / 2.0 - 2.0).max(5.0);
        let phase = -std::f32::consts::FRAC_PI_2 + rng.random_range(-0.08..0.08);

        for y in 0..height {
            for x in 0..width {
                let dx = (x as i32 - cx) as f32 / (max_rx * 0.95);
                let dy = (y as i32 - cy) as f32 / (max_ry * 0.95);
                let metric = dx * dx + dy * dy;
                if metric < 0.91 {
                    blank(&mut grid, x as i32, y as i32);
                } else if metric < 1.08 && (x + y + seed as usize) % 5 == 0 {
                    grid[y][x] = Cell::new('·', darken(chalk, 55));
                }
            }
        }

        for band in 0..3 {
            let rx = max_rx - band as f32 * 3.2;
            let ry = max_ry - band as f32 * 1.25;
            let color = match band {
                0 => ether,
                1 => chalk,
                _ => gold,
            };
            let gap = 0.16 + band as f32 * 0.035;
            for node in 0..node_count {
                let a0 = phase + node as f32 * std::f32::consts::TAU / node_count as f32 + gap;
                let a1 =
                    phase + (node + 1) as f32 * std::f32::consts::TAU / node_count as f32 - gap;
                draw_arc(
                    &mut grid,
                    cx,
                    cy,
                    rx,
                    ry,
                    a0,
                    a1,
                    darken(color, (band * 10) as u8),
                    if band == 1 { 7 } else { 0 },
                );
            }
        }

        let mut nodes = Vec::new();
        let mut inner_nodes = Vec::new();
        let node_glyphs = ['△', '▽', '□', '◇', '☉', '☽', '☿', '♄', '♃', '♁', '✦', '∴'];
        for i in 0..node_count {
            let base = phase + i as f32 * std::f32::consts::TAU / node_count as f32;
            let a = base + rng.random_range(-0.17..0.17);
            let outer_scale = rng.random_range(0.82..0.97);
            let inner_scale = rng.random_range(0.53..0.68);
            let outer = point_on(cx, cy, max_rx * outer_scale, max_ry * outer_scale, a);
            let inner = point_on(cx, cy, max_rx * inner_scale, max_ry * inner_scale, a);
            nodes.push(outer);
            inner_nodes.push(inner);
            draw_line(
                &mut grid,
                inner.0,
                inner.1,
                outer.0,
                outer.1,
                darken(gold, 6),
            );
            draw_line(
                &mut grid,
                outer.0 - 2,
                outer.1,
                outer.0 + 2,
                outer.1,
                lighten(chalk, 8),
            );
            draw_line(
                &mut grid,
                outer.0,
                outer.1 - 1,
                outer.0,
                outer.1 + 1,
                lighten(chalk, 8),
            );
            put(
                &mut grid,
                outer.0,
                outer.1,
                node_glyphs[i % node_glyphs.len()],
                ether,
            );
            put(&mut grid, outer.0 - 3, outer.1, '╴', darken(ether, 20));
            put(&mut grid, outer.0 + 3, outer.1, '╶', darken(ether, 20));
        }

        let mut core = Vec::new();
        let core_count = 5 + (seed as usize % 2);
        for i in 0..core_count {
            let a = phase
                + std::f32::consts::PI / core_count as f32
                + i as f32 * std::f32::consts::TAU / core_count as f32;
            core.push(point_on(cx, cy, max_rx * 0.24, max_ry * 0.24, a));
        }
        draw_poly(&mut grid, &core, 1, darken(chalk, 4));

        let runes = [
            '△', '▽', '□', '◇', '○', '☉', '☽', '☿', '♄', '♃', '♁', '✶', '✦', '✧', '+', '×', '≡',
            '∴', '∵', '⌬', '⊕', '⊗',
        ];
        for i in 0..rune_count {
            let lane = match i % 5 {
                0 => 0.93,
                1 => 0.84,
                2 => 0.76,
                3 => 0.68,
                _ => rng.random_range(0.66..0.92),
            };
            let jitter = rng.random_range(-0.035..0.035);
            let a = phase
                + i as f32 / rune_count as f32 * std::f32::consts::TAU
                + jitter
                + (i % node_count) as f32 * 0.006;
            let p = point_on(cx, cy, max_rx * lane, max_ry * lane, a);
            let color = match i % 4 {
                0 => gold,
                1 => ether,
                2 => chalk,
                _ => blood,
            };
            put(
                &mut grid,
                p.0,
                p.1,
                runes[(i + rng.random_range(0..runes.len())) % runes.len()],
                darken(color, rng.random_range(0..24)),
            );
        }

        for _ in 0..fracture_count {
            let node = rng.random_range(0..node_count);
            let from = nodes[node];
            let target_angle = phase
                + (node as f32 + rng.random_range(1.5..4.5)) * std::f32::consts::TAU
                    / node_count as f32;
            let target_radius = rng.random_range(0.68..0.94);
            let to = point_on(
                cx,
                cy,
                max_rx * target_radius,
                max_ry * target_radius,
                target_angle,
            );
            let mid = (
                ((from.0 + to.0) / 2) + rng.random_range(-5..=5),
                ((from.1 + to.1) / 2) + rng.random_range(-2..=2),
            );
            let color = if rng.random::<f32>() < 0.45 {
                darken(blood, rng.random_range(0..34))
            } else {
                darken(ether, rng.random_range(0..28))
            };
            draw_line(&mut grid, from.0, from.1, mid.0, mid.1, color);
            draw_line(&mut grid, mid.0, mid.1, to.0, to.1, color);
            if rng.random::<f32>() < 0.38 {
                put(&mut grid, mid.0, mid.1, '✦', lighten(color, 18));
            }
        }

        let core_clear_rx = (max_rx * 0.18).round() as i32;
        let core_clear_ry = (max_ry * 0.18).round() as i32;
        for dy in -core_clear_ry..=core_clear_ry {
            for dx in -core_clear_rx..=core_clear_rx {
                let metric = (dx as f32 / core_clear_rx.max(1) as f32).powi(2)
                    + (dy as f32 / core_clear_ry.max(1) as f32).powi(2);
                if metric <= 1.0 {
                    blank(&mut grid, cx + dx, cy + dy);
                }
            }
        }
        draw_arc(
            &mut grid,
            cx,
            cy,
            max_rx * 0.16,
            max_ry * 0.14,
            phase,
            phase + std::f32::consts::TAU,
            lighten(gold, 6),
            0,
        );
        draw_arc(
            &mut grid,
            cx,
            cy,
            max_rx * 0.10,
            max_ry * 0.09,
            phase,
            phase + std::f32::consts::TAU,
            darken(chalk, 6),
            5,
        );

        let vertical = (max_ry * 0.24) as i32;
        let horizontal = (max_rx * 0.10) as i32;
        draw_line(
            &mut grid,
            cx,
            cy - vertical,
            cx,
            cy + vertical,
            lighten(chalk, 10),
        );
        draw_line(
            &mut grid,
            cx - horizontal,
            cy,
            cx + horizontal,
            cy,
            lighten(chalk, 10),
        );
        for dy in -2i32..=2i32 {
            for dx in -5i32..=5i32 {
                let metric = (dx as f32 / 5.0).powi(2) + (dy as f32 / 2.0).powi(2);
                if metric <= 1.0 {
                    let ch = if dx == 0 && dy == 0 {
                        '⊕'
                    } else if dx.abs() == dy.abs() * 2 {
                        '╳'
                    } else if dy == 0 {
                        '═'
                    } else if dx == 0 {
                        '║'
                    } else {
                        '·'
                    };
                    put(&mut grid, cx + dx, cy + dy, ch, lighten(blood, 18));
                }
            }
        }
        for (i, &(nx, ny)) in nodes.iter().enumerate() {
            for dy in -1i32..=1i32 {
                for dx in -2i32..=2i32 {
                    blank(&mut grid, nx + dx, ny + dy);
                }
            }
            put(&mut grid, nx - 2, ny - 1, '╭', lighten(chalk, 8));
            put(&mut grid, nx + 2, ny - 1, '╮', lighten(chalk, 8));
            put(&mut grid, nx - 2, ny + 1, '╰', lighten(chalk, 8));
            put(&mut grid, nx + 2, ny + 1, '╯', lighten(chalk, 8));
            put(&mut grid, nx - 1, ny - 1, '─', lighten(chalk, 8));
            put(&mut grid, nx, ny - 1, '─', lighten(chalk, 8));
            put(&mut grid, nx + 1, ny - 1, '─', lighten(chalk, 8));
            put(&mut grid, nx - 1, ny + 1, '─', lighten(chalk, 8));
            put(&mut grid, nx, ny + 1, '─', lighten(chalk, 8));
            put(&mut grid, nx + 1, ny + 1, '─', lighten(chalk, 8));
            put(&mut grid, nx - 2, ny, '│', lighten(chalk, 8));
            put(&mut grid, nx + 2, ny, '│', lighten(chalk, 8));
            put(&mut grid, nx, ny, node_glyphs[i % node_glyphs.len()], ether);
        }
    (grid, false)
}

/// Dispatch arm for mode(s): fa3, fullmetal-alchemist3 (moved verbatim from run()).
pub(crate) fn cli_fa3_fullmetal_alchemist3(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        // fa3 [paths=0] [rings] [nodes=0] -- ornamented ray paths with inner circles and node stations
        let path_arg: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(0);
        let path_count = if path_arg == 0 {
            8 + ((seed as usize * 19 + 3) % 8)
        } else {
            path_arg.clamp(5, 22)
        };
        let inner_count: usize = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(4);
        let inner_count = inner_count.clamp(2, 7);
        let node_arg: usize = args.get(6).and_then(|s| s.parse().ok()).unwrap_or(0);
        let base_nodes = if node_arg == 0 {
            4 + ((seed as usize * 31 + 7) % 7)
        } else {
            node_arg.clamp(3, 14)
        };

        let bg = darken(palette[0], 12);
        let chalk = lighten(palette[4], 12);
        let gold = lighten(palette[1], 32);
        let ether = shift_hue(lighten(palette[3], 36), 35.0);
        let rose = shift_hue(lighten(palette[2], 42), -38.0);
        let hush = darken(palette[2], 66);

        for y in 0..height {
            for x in 0..width {
                let n = (x * 29 + y * 37 + seed as usize * 11) % 173;
                let ch = match n {
                    0 => '·',
                    1 => '∙',
                    2 if (x + y) % 5 == 0 => '°',
                    _ => ' ',
                };
                grid[y][x] = if ch == ' ' {
                    Cell::new(' ', bg)
                } else {
                    Cell::new(ch, hush)
                };
            }
        }

        let put = |grid: &mut Grid, x: i32, y: i32, ch: char, fg: Color| {
            if x >= 0 && y >= 0 && (x as usize) < width && (y as usize) < height {
                grid[y as usize][x as usize] = Cell::new(ch, fg);
            }
        };
        let blank = |grid: &mut Grid, x: i32, y: i32| {
            if x >= 0 && y >= 0 && (x as usize) < width && (y as usize) < height {
                grid[y as usize][x as usize] = Cell::blank();
            }
        };
        let stroke_char = |x0: i32, y0: i32, x1: i32, y1: i32| {
            let dx = x1 - x0;
            let dy = y1 - y0;
            if dx.abs() > dy.abs() * 2 {
                '─'
            } else if dy.abs() > dx.abs() * 2 {
                '│'
            } else if dx.signum() == dy.signum() {
                '╲'
            } else {
                '╱'
            }
        };
        let draw_line = |grid: &mut Grid, mut x0: i32, mut y0: i32, x1: i32, y1: i32, fg: Color| {
            let ch = stroke_char(x0, y0, x1, y1);
            let dx = (x1 - x0).abs();
            let sx = if x0 < x1 { 1 } else { -1 };
            let dy = -(y1 - y0).abs();
            let sy = if y0 < y1 { 1 } else { -1 };
            let mut err = dx + dy;
            loop {
                put(grid, x0, y0, ch, fg);
                if x0 == x1 && y0 == y1 {
                    break;
                }
                let e2 = 2 * err;
                if e2 >= dy {
                    err += dy;
                    x0 += sx;
                }
                if e2 <= dx {
                    err += dx;
                    y0 += sy;
                }
            }
        };
        let point_on = |cx: i32, cy: i32, rx: f32, ry: f32, angle: f32| {
            (
                cx + (angle.cos() * rx).round() as i32,
                cy + (angle.sin() * ry).round() as i32,
            )
        };
        let draw_arc = |grid: &mut Grid,
                        cx: i32,
                        cy: i32,
                        rx: f32,
                        ry: f32,
                        start: f32,
                        end: f32,
                        fg: Color,
                        gap: usize| {
            let span = (end - start).abs().max(0.05);
            let samples = ((rx + ry) * span * 3.6).max(18.0) as usize;
            let mut prev: Option<(i32, i32)> = None;
            for i in 0..=samples {
                if gap > 0 && i % gap == gap - 1 {
                    prev = None;
                    continue;
                }
                let a = start + (end - start) * i as f32 / samples as f32;
                let p = point_on(cx, cy, rx, ry, a);
                if let Some(q) = prev {
                    draw_line(grid, q.0, q.1, p.0, p.1, fg);
                } else {
                    put(grid, p.0, p.1, '·', fg);
                }
                prev = Some(p);
            }
        };
        let draw_shape = |grid: &mut Grid, x: i32, y: i32, kind: usize, fg: Color| match kind % 7 {
            0 => {
                put(grid, x, y, '◇', fg);
                put(grid, x - 1, y, '╴', darken(fg, 18));
                put(grid, x + 1, y, '╶', darken(fg, 18));
            }
            1 => {
                for dx in -1..=1 {
                    put(grid, x + dx, y - 1, '─', fg);
                    put(grid, x + dx, y + 1, '─', fg);
                }
                put(grid, x - 2, y, '│', fg);
                put(grid, x + 2, y, '│', fg);
                put(grid, x, y, '□', lighten(fg, 10));
            }
            2 => {
                put(grid, x, y - 1, '△', lighten(fg, 8));
                put(grid, x - 1, y, '╱', fg);
                put(grid, x + 1, y, '╲', fg);
                put(grid, x, y + 1, '─', darken(fg, 12));
            }
            3 => {
                put(grid, x, y - 1, '○', fg);
                put(grid, x - 1, y, '◌', darken(fg, 10));
                put(grid, x, y, '☉', lighten(fg, 10));
                put(grid, x + 1, y, '◌', darken(fg, 10));
                put(grid, x, y + 1, '○', fg);
            }
            4 => {
                put(grid, x, y, '⊕', lighten(fg, 12));
                put(grid, x - 1, y, '─', fg);
                put(grid, x + 1, y, '─', fg);
                put(grid, x, y - 1, '│', fg);
                put(grid, x, y + 1, '│', fg);
            }
            5 => {
                put(grid, x, y, '⌬', lighten(fg, 8));
                put(grid, x - 1, y - 1, '╲', fg);
                put(grid, x + 1, y - 1, '╱', fg);
                put(grid, x - 1, y + 1, '╱', fg);
                put(grid, x + 1, y + 1, '╲', fg);
            }
            _ => {
                put(grid, x, y, '✦', lighten(fg, 14));
                put(grid, x - 1, y, '·', fg);
                put(grid, x + 1, y, '·', fg);
                put(grid, x, y - 1, '·', fg);
                put(grid, x, y + 1, '·', fg);
            }
        };

        let cx = width as i32 / 2;
        let cy = height as i32 / 2;
        let max_rx = (width as f32 / 2.0 - 3.0).max(10.0);
        let max_ry = (height as f32 / 2.0 - 2.0).max(5.0);
        let phase = -std::f32::consts::FRAC_PI_2 + rng.random_range(-0.10..0.10);

        for y in 0..height {
            for x in 0..width {
                let dx = (x as i32 - cx) as f32 / (max_rx * 0.96);
                let dy = (y as i32 - cy) as f32 / (max_ry * 0.96);
                let metric = dx * dx + dy * dy;
                if metric < 0.88 {
                    blank(&mut grid, x as i32, y as i32);
                } else if metric < 1.07 && (x + y + seed as usize) % 4 == 0 {
                    grid[y][x] = Cell::new('·', darken(chalk, 55));
                }
            }
        }

        for band in 0..4 {
            let rx = max_rx - band as f32 * 2.6;
            let ry = max_ry - band as f32 * 1.05;
            let fg = match band {
                0 => ether,
                1 => chalk,
                2 => gold,
                _ => darken(rose, 4),
            };
            let gap = 0.08 + band as f32 * 0.035;
            let segments = path_count.max(6);
            for seg in 0..segments {
                let a0 = phase + seg as f32 * std::f32::consts::TAU / segments as f32 + gap;
                let a1 = phase + (seg + 1) as f32 * std::f32::consts::TAU / segments as f32 - gap;
                draw_arc(
                    &mut grid,
                    cx,
                    cy,
                    rx,
                    ry,
                    a0,
                    a1,
                    darken(fg, (band * 7) as u8),
                    if band == 2 { 6 } else { 0 },
                );
            }
        }

        let mut ring_specs = Vec::new();
        for r in 0..inner_count {
            let t = (r + 1) as f32 / (inner_count + 1) as f32;
            let scale = 0.18 + t * 0.60 + rng.random_range(-0.025..0.025);
            let node_count = (base_nodes + r + (seed as usize % 3)).clamp(3, 16);
            let ring_phase = phase + r as f32 * 0.41 + rng.random_range(-0.12..0.12);
            let fg = match r % 4 {
                0 => gold,
                1 => ether,
                2 => chalk,
                _ => rose,
            };
            ring_specs.push((scale, node_count, ring_phase, fg));
            draw_arc(
                &mut grid,
                cx,
                cy,
                max_rx * scale,
                max_ry * scale,
                ring_phase,
                ring_phase + std::f32::consts::TAU,
                darken(fg, 8),
                if r % 2 == 0 { 8 } else { 0 },
            );
            for n in 0..node_count {
                let a = ring_phase + n as f32 * std::f32::consts::TAU / node_count as f32;
                let p = point_on(cx, cy, max_rx * scale, max_ry * scale, a);
                draw_shape(&mut grid, p.0, p.1, (n + r * 3) % 7, fg);
            }
        }

        for path in 0..path_count {
            let base_a = phase
                + path as f32 * std::f32::consts::TAU / path_count as f32
                + rng.random_range(-0.13..0.13);
            let mut prev = point_on(cx, cy, max_rx * 0.08, max_ry * 0.08, base_a);
            let path_color = match path % 4 {
                0 => ether,
                1 => gold,
                2 => chalk,
                _ => rose,
            };
            for (ri, &(scale, _, ring_phase, _)) in ring_specs.iter().enumerate() {
                let a = base_a + (ring_phase - phase) * 0.18 + (ri as f32 * 0.11).sin() * 0.16;
                let p = point_on(cx, cy, max_rx * scale, max_ry * scale, a);
                draw_line(&mut grid, prev.0, prev.1, p.0, p.1, darken(path_color, 10));
                draw_shape(
                    &mut grid,
                    p.0,
                    p.1,
                    path + ri + (seed as usize % 5),
                    shift_hue(lighten(path_color, 6), (ri * 22) as f64),
                );

                let bead_count = 1 + ((path + ri + seed as usize) % 3);
                for bead in 1..=bead_count {
                    let f = bead as f32 / (bead_count + 1) as f32;
                    let bx = (prev.0 as f32 + (p.0 - prev.0) as f32 * f).round() as i32;
                    let by = (prev.1 as f32 + (p.1 - prev.1) as f32 * f).round() as i32;
                    let bead_ch = ['○', '□', '◇', '△', '☉', '⊕'][(path + ri + bead) % 6];
                    put(&mut grid, bx, by, bead_ch, darken(path_color, 4));
                }
                prev = p;
            }
            let outer = point_on(
                cx,
                cy,
                max_rx * 0.96,
                max_ry * 0.96,
                base_a + rng.random_range(-0.05..0.05),
            );
            draw_line(
                &mut grid,
                prev.0,
                prev.1,
                outer.0,
                outer.1,
                darken(path_color, 8),
            );
            draw_shape(
                &mut grid,
                outer.0,
                outer.1,
                path + 3,
                lighten(path_color, 8),
            );
        }

        let core_rx = (max_rx * 0.17).round() as i32;
        let core_ry = (max_ry * 0.17).round() as i32;
        for dy in -core_ry..=core_ry {
            for dx in -core_rx..=core_rx {
                let metric = (dx as f32 / core_rx.max(1) as f32).powi(2)
                    + (dy as f32 / core_ry.max(1) as f32).powi(2);
                if metric <= 1.0 {
                    blank(&mut grid, cx + dx, cy + dy);
                }
            }
        }
        for r in 0..3 {
            let scale = 0.06 + r as f32 * 0.045;
            draw_arc(
                &mut grid,
                cx,
                cy,
                max_rx * scale,
                max_ry * scale,
                phase,
                phase + std::f32::consts::TAU,
                if r == 1 { gold } else { chalk },
                if r == 2 { 5 } else { 0 },
            );
        }
        let core_nodes = 6 + (seed as usize % 3);
        for n in 0..core_nodes {
            let a = phase + n as f32 * std::f32::consts::TAU / core_nodes as f32;
            let p = point_on(cx, cy, max_rx * 0.13, max_ry * 0.13, a);
            draw_shape(
                &mut grid,
                p.0,
                p.1,
                n + 4,
                if n % 2 == 0 { gold } else { ether },
            );
        }
        put(&mut grid, cx, cy, '⊙', lighten(rose, 16));
    (grid, false)
}

/// Dispatch arm for mode(s): fa4, fullmetal-alchemist4 (moved verbatim from run()).
pub(crate) fn cli_fa4_fullmetal_alchemist4(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        // fa4 [paths=0] [rings] [nodes=0] [ornaments] [stations=0] -- airy curved ritual lattice
        let path_arg: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(0);
        let path_count = if path_arg == 0 {
            0
        } else {
            path_arg.clamp(1, 6)
        };
        let ring_count: usize = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(2);
        let ring_count = ring_count.clamp(2, 5);
        let node_arg: usize = args.get(6).and_then(|s| s.parse().ok()).unwrap_or(0);
        let base_nodes = if node_arg == 0 {
            3 + ((seed as usize * 41 + 9) % 3)
        } else {
            node_arg.clamp(3, 8)
        };
        let ornament_step: usize = args.get(7).and_then(|s| s.parse().ok()).unwrap_or(32);
        let ornament_step = ornament_step.clamp(18, 48);
        let station_arg: usize = args.get(8).and_then(|s| s.parse().ok()).unwrap_or(0);
        let station_base = if station_arg == 0 {
            6
        } else {
            station_arg.clamp(4, 16)
        };

        let bg = darken(palette[0], 13);
        let chalk = lighten(palette[4], 14);
        let gold = lighten(palette[1], 34);
        let ether = shift_hue(lighten(palette[3], 38), 32.0);
        let rose = shift_hue(lighten(palette[2], 44), -36.0);
        let verdigris = shift_hue(lighten(palette[1], 28), 92.0);
        let hush = darken(palette[2], 70);

        for y in 0..height {
            for x in 0..width {
                let n = (x * 31 + y * 43 + seed as usize * 13) % 353;
                let ch = match n {
                    0 => '·',
                    1 if (x + y + seed as usize) % 9 == 0 => '°',
                    _ => ' ',
                };
                grid[y][x] = if ch == ' ' {
                    Cell::new(' ', bg)
                } else {
                    Cell::new(ch, hush)
                };
            }
        }

        let put = |grid: &mut Grid, x: i32, y: i32, ch: char, fg: Color| {
            if x >= 0 && y >= 0 && (x as usize) < width && (y as usize) < height {
                grid[y as usize][x as usize] = Cell::new(ch, fg);
            }
        };
        let blank = |grid: &mut Grid, x: i32, y: i32| {
            if x >= 0 && y >= 0 && (x as usize) < width && (y as usize) < height {
                grid[y as usize][x as usize] = Cell::blank();
            }
        };
        let point_on = |cx: i32, cy: i32, rx: f32, ry: f32, angle: f32| {
            (
                cx + (angle.cos() * rx).round() as i32,
                cy + (angle.sin() * ry).round() as i32,
            )
        };
        let curve_char = |prev: (i32, i32), here: (i32, i32), next: (i32, i32)| {
            let dx1 = (here.0 - prev.0).signum();
            let dy1 = (here.1 - prev.1).signum();
            let dx2 = (next.0 - here.0).signum();
            let dy2 = (next.1 - here.1).signum();
            if (dx1, dy1) == (dx2, dy2) {
                if dy1 == 0 {
                    '─'
                } else if dx1 == 0 {
                    '│'
                } else if dx1 == dy1 {
                    '╲'
                } else {
                    '╱'
                }
            } else if dy1 == 0 && dx2 == 0 {
                match (dx1, dy2) {
                    (1, 1) => '╮',
                    (1, -1) => '╯',
                    (-1, 1) => '╭',
                    (-1, -1) => '╰',
                    _ => '╮',
                }
            } else if dx1 == 0 && dy2 == 0 {
                match (dy1, dx2) {
                    (1, 1) => '╰',
                    (1, -1) => '╯',
                    (-1, 1) => '╭',
                    (-1, -1) => '╮',
                    _ => '╰',
                }
            } else if dx1 != dx2 && dy1 != dy2 {
                match (dx1, dy1, dx2, dy2) {
                    (1, 1, 1, -1) | (-1, -1, -1, 1) => '╯',
                    (1, -1, 1, 1) | (-1, 1, -1, -1) => '╮',
                    (1, 1, -1, 1) | (-1, -1, 1, -1) => '╰',
                    (1, -1, -1, -1) | (-1, 1, 1, 1) => '╭',
                    _ => '○',
                }
            } else if dx2 == 0 || dx1 == 0 {
                '│'
            } else if dy2 == 0 || dy1 == 0 {
                '─'
            } else if dx2 == dy2 {
                '╲'
            } else {
                '╱'
            }
        };
        let draw_line = |grid: &mut Grid, mut x0: i32, mut y0: i32, x1: i32, y1: i32, fg: Color| {
            let dx = (x1 - x0).abs();
            let sx = if x0 < x1 { 1 } else { -1 };
            let dy = -(y1 - y0).abs();
            let sy = if y0 < y1 { 1 } else { -1 };
            let ch = if dx > (-dy) * 2 {
                '─'
            } else if -dy > dx * 2 {
                '│'
            } else if (x1 - x0).signum() == (y1 - y0).signum() {
                '╲'
            } else {
                '╱'
            };
            let mut err = dx + dy;
            loop {
                put(grid, x0, y0, ch, fg);
                if x0 == x1 && y0 == y1 {
                    break;
                }
                let e2 = 2 * err;
                if e2 >= dy {
                    err += dy;
                    x0 += sx;
                }
                if e2 <= dx {
                    err += dx;
                    y0 += sy;
                }
            }
        };
        let draw_micro_shape =
            |grid: &mut Grid, x: i32, y: i32, kind: usize, fg: Color| match kind % 10 {
                0 => {
                    put(grid, x, y, '○', lighten(fg, 10));
                    put(grid, x - 1, y, '╴', darken(fg, 12));
                    put(grid, x + 1, y, '╶', darken(fg, 12));
                }
                1 => {
                    put(grid, x, y, '◇', lighten(fg, 12));
                    put(grid, x, y - 1, '╭', fg);
                    put(grid, x, y + 1, '╯', fg);
                }
                2 => {
                    put(grid, x - 1, y - 1, '╭', fg);
                    put(grid, x + 1, y - 1, '╮', fg);
                    put(grid, x - 1, y + 1, '╰', fg);
                    put(grid, x + 1, y + 1, '╯', fg);
                    put(grid, x, y, '⊙', lighten(fg, 10));
                }
                3 => {
                    put(grid, x, y - 1, '△', lighten(fg, 8));
                    put(grid, x - 1, y, '╰', fg);
                    put(grid, x + 1, y, '╯', fg);
                    put(grid, x, y + 1, '╰', darken(fg, 8));
                }
                4 => {
                    put(grid, x, y, '☉', lighten(fg, 14));
                    put(grid, x - 1, y - 1, '╭', darken(fg, 4));
                    put(grid, x + 1, y - 1, '╮', darken(fg, 4));
                    put(grid, x - 1, y + 1, '╰', darken(fg, 4));
                    put(grid, x + 1, y + 1, '╯', darken(fg, 4));
                }
                5 => {
                    put(grid, x, y, '⌬', lighten(fg, 8));
                    put(grid, x - 1, y, '╭', fg);
                    put(grid, x + 1, y, '╮', fg);
                }
                6 => {
                    put(grid, x, y, '□', lighten(fg, 8));
                    put(grid, x - 1, y, '◜', fg);
                    put(grid, x + 1, y, '◝', fg);
                    put(grid, x, y + 1, '╯', darken(fg, 10));
                }
                7 => {
                    put(grid, x, y, '⊕', lighten(fg, 10));
                    put(grid, x - 1, y, '─', fg);
                    put(grid, x + 1, y, '─', fg);
                    put(grid, x, y - 1, '│', fg);
                    put(grid, x, y + 1, '│', fg);
                }
                8 => {
                    put(grid, x, y, '✦', lighten(fg, 16));
                    put(grid, x - 1, y, '◌', fg);
                    put(grid, x + 1, y, '◌', fg);
                }
                _ => {
                    put(grid, x, y, '∴', lighten(fg, 10));
                    put(grid, x - 1, y - 1, '·', fg);
                    put(grid, x + 1, y + 1, '·', fg);
                    put(grid, x + 1, y - 1, '·', darken(fg, 10));
                }
            };
        let draw_curve = |grid: &mut Grid,
                          p0: (i32, i32),
                          p1: (i32, i32),
                          p2: (i32, i32),
                          fg: Color,
                          accent: Color,
                          shape_offset: usize,
                          ornament_step: usize|
         -> Vec<(i32, i32)> {
            let d0 = ((p1.0 - p0.0).abs() + (p1.1 - p0.1).abs()) as f32;
            let d1 = ((p2.0 - p1.0).abs() + (p2.1 - p1.1).abs()) as f32;
            let samples = ((d0 + d1) * 1.7).clamp(18.0, 240.0) as usize;
            let mut pts = Vec::new();
            for i in 0..=samples {
                let t = i as f32 / samples as f32;
                let u = 1.0 - t;
                let x = (u * u * p0.0 as f32 + 2.0 * u * t * p1.0 as f32 + t * t * p2.0 as f32)
                    .round() as i32;
                let y = (u * u * p0.1 as f32 + 2.0 * u * t * p1.1 as f32 + t * t * p2.1 as f32)
                    .round() as i32;
                if pts.last().copied() != Some((x, y)) {
                    pts.push((x, y));
                }
            }
            if pts.len() < 3 {
                draw_line(grid, p0.0, p0.1, p2.0, p2.1, fg);
                return pts;
            }
            for i in 0..pts.len() {
                let (x, y) = pts[i];
                if i == 0 || i + 1 == pts.len() {
                    if shape_offset % 3 != 1 {
                        put(grid, x, y, '○', darken(accent, 8));
                    }
                } else {
                    let ch = curve_char(pts[i - 1], pts[i], pts[i + 1]);
                    put(grid, x, y, ch, fg);
                    if i % ornament_step == shape_offset % ornament_step {
                        draw_micro_shape(grid, x, y, i + shape_offset, accent);
                    }
                }
            }
            pts
        };
        let draw_arc = |grid: &mut Grid,
                        cx: i32,
                        cy: i32,
                        rx: f32,
                        ry: f32,
                        start: f32,
                        end: f32,
                        fg: Color,
                        accent: Color,
                        gap: usize,
                        ornament_step: usize| {
            let span = (end - start).abs().max(0.04);
            let samples = ((rx + ry) * span * 4.2).max(20.0) as usize;
            let mut pts = Vec::new();
            for i in 0..=samples {
                if gap > 0 && i % gap == gap - 1 {
                    if pts.len() > 2 {
                        for p in 1..pts.len() - 1 {
                            let ch = curve_char(pts[p - 1], pts[p], pts[p + 1]);
                            put(grid, pts[p].0, pts[p].1, ch, fg);
                            if p % (ornament_step * 2) == 0 {
                                draw_micro_shape(grid, pts[p].0, pts[p].1, p, accent);
                            }
                        }
                    }
                    pts.clear();
                    continue;
                }
                let a = start + (end - start) * i as f32 / samples as f32;
                let p = point_on(cx, cy, rx, ry, a);
                if pts.last().copied() != Some(p) {
                    pts.push(p);
                }
            }
            if pts.len() > 2 {
                for p in 1..pts.len() - 1 {
                    let ch = curve_char(pts[p - 1], pts[p], pts[p + 1]);
                    put(grid, pts[p].0, pts[p].1, ch, fg);
                    if p % (ornament_step * 2) == 0 {
                        draw_micro_shape(grid, pts[p].0, pts[p].1, p + 3, accent);
                    }
                }
            }
        };
        let draw_box_frame = |grid: &mut Grid, cx: i32, cy: i32, hw: i32, hh: i32, fg: Color| {
            for x in cx - hw + 1..=cx + hw - 1 {
                put(grid, x, cy - hh, '─', fg);
                put(grid, x, cy + hh, '─', fg);
            }
            for y in cy - hh + 1..=cy + hh - 1 {
                put(grid, cx - hw, y, '│', fg);
                put(grid, cx + hw, y, '│', fg);
            }
            put(grid, cx - hw, cy - hh, '╭', fg);
            put(grid, cx + hw, cy - hh, '╮', fg);
            put(grid, cx - hw, cy + hh, '╰', fg);
            put(grid, cx + hw, cy + hh, '╯', fg);
        };
        let draw_diamond_frame =
            |grid: &mut Grid, cx: i32, cy: i32, rx: i32, ry: i32, fg: Color| {
                let top = (cx, cy - ry);
                let right = (cx + rx, cy);
                let bottom = (cx, cy + ry);
                let left = (cx - rx, cy);
                draw_line(grid, top.0, top.1, right.0, right.1, fg);
                draw_line(grid, right.0, right.1, bottom.0, bottom.1, fg);
                draw_line(grid, bottom.0, bottom.1, left.0, left.1, fg);
                draw_line(grid, left.0, left.1, top.0, top.1, fg);
                put(grid, top.0, top.1, '△', lighten(fg, 8));
                put(grid, right.0, right.1, '◇', lighten(fg, 8));
                put(grid, bottom.0, bottom.1, '▽', lighten(fg, 8));
                put(grid, left.0, left.1, '◇', lighten(fg, 8));
            };
        let draw_ring_ticks = |grid: &mut Grid,
                               cx: i32,
                               cy: i32,
                               rx: f32,
                               ry: f32,
                               count: usize,
                               phase: f32,
                               fg: Color| {
            for i in 0..count {
                let a = phase + i as f32 * std::f32::consts::TAU / count as f32;
                let p = point_on(cx, cy, rx, ry, a);
                if a.sin().abs() > a.cos().abs() {
                    put(grid, p.0 - 1, p.1, '─', fg);
                    put(grid, p.0, p.1, '┼', lighten(fg, 8));
                    put(grid, p.0 + 1, p.1, '─', fg);
                } else {
                    put(grid, p.0, p.1 - 1, '│', fg);
                    put(grid, p.0, p.1, '┼', lighten(fg, 8));
                    put(grid, p.0, p.1 + 1, '│', fg);
                }
            }
        };
        let draw_geo_station = |grid: &mut Grid, x: i32, y: i32, kind: usize, fg: Color| {
            for dy in -1i32..=1 {
                for dx in -3i32..=3 {
                    blank(grid, x + dx, y + dy);
                }
            }
            match kind % 8 {
                0 => {
                    put(grid, x, y - 1, '╱', fg);
                    put(grid, x + 1, y - 1, '╲', fg);
                    put(grid, x - 1, y, '◇', lighten(fg, 16));
                    put(grid, x, y, '◆', lighten(fg, 20));
                    put(grid, x + 1, y, '◇', lighten(fg, 16));
                    put(grid, x, y + 1, '╲', fg);
                    put(grid, x + 1, y + 1, '╱', fg);
                }
                1 => {
                    put(grid, x - 2, y - 1, '╭', fg);
                    put(grid, x - 1, y - 1, '─', fg);
                    put(grid, x, y - 1, '□', lighten(fg, 14));
                    put(grid, x + 1, y - 1, '─', fg);
                    put(grid, x + 2, y - 1, '╮', fg);
                    put(grid, x - 2, y, '│', fg);
                    put(grid, x, y, '⊙', lighten(fg, 20));
                    put(grid, x + 2, y, '│', fg);
                    put(grid, x - 2, y + 1, '╰', fg);
                    put(grid, x - 1, y + 1, '─', fg);
                    put(grid, x, y + 1, '□', lighten(fg, 14));
                    put(grid, x + 1, y + 1, '─', fg);
                    put(grid, x + 2, y + 1, '╯', fg);
                }
                2 => {
                    put(grid, x, y - 1, '△', lighten(fg, 18));
                    put(grid, x - 2, y, '╱', fg);
                    put(grid, x - 1, y, '─', darken(fg, 4));
                    put(grid, x, y, '☉', lighten(fg, 18));
                    put(grid, x + 1, y, '─', darken(fg, 4));
                    put(grid, x + 2, y, '╲', fg);
                    put(grid, x - 1, y + 1, '╰', fg);
                    put(grid, x, y + 1, '─', fg);
                    put(grid, x + 1, y + 1, '╯', fg);
                }
                3 => {
                    put(grid, x, y, '⊕', lighten(fg, 20));
                    put(grid, x - 2, y, '╴', fg);
                    put(grid, x - 1, y, '○', lighten(fg, 12));
                    put(grid, x + 1, y, '○', lighten(fg, 12));
                    put(grid, x + 2, y, '╶', fg);
                    put(grid, x, y - 1, '│', fg);
                    put(grid, x, y + 1, '│', fg);
                }
                4 => {
                    put(grid, x, y, '✦', lighten(fg, 22));
                    put(grid, x - 2, y, '◇', fg);
                    put(grid, x + 2, y, '◇', fg);
                    put(grid, x, y - 1, '△', fg);
                    put(grid, x, y + 1, '▽', fg);
                    put(grid, x - 1, y - 1, '╲', darken(fg, 6));
                    put(grid, x + 1, y - 1, '╱', darken(fg, 6));
                }
                5 => {
                    put(grid, x - 2, y, '╭', fg);
                    put(grid, x - 1, y, '─', fg);
                    put(grid, x, y, '⌬', lighten(fg, 18));
                    put(grid, x + 1, y, '─', fg);
                    put(grid, x + 2, y, '╮', fg);
                    put(grid, x - 1, y + 1, '╰', darken(fg, 8));
                    put(grid, x, y + 1, '─', darken(fg, 8));
                    put(grid, x + 1, y + 1, '╯', darken(fg, 8));
                }
                6 => {
                    put(grid, x - 2, y - 1, '╭', fg);
                    put(grid, x, y - 1, '┬', fg);
                    put(grid, x + 2, y - 1, '╮', fg);
                    put(grid, x - 1, y, '□', lighten(fg, 14));
                    put(grid, x, y, '┼', lighten(fg, 20));
                    put(grid, x + 1, y, '□', lighten(fg, 14));
                    put(grid, x - 2, y + 1, '╰', fg);
                    put(grid, x, y + 1, '┴', fg);
                    put(grid, x + 2, y + 1, '╯', fg);
                }
                _ => {
                    put(grid, x, y + 1, '▽', lighten(fg, 16));
                    put(grid, x - 2, y, '╲', fg);
                    put(grid, x - 1, y, '─', fg);
                    put(grid, x, y, '⊙', lighten(fg, 18));
                    put(grid, x + 1, y, '─', fg);
                    put(grid, x + 2, y, '╱', fg);
                    put(grid, x, y - 1, '╭', darken(fg, 8));
                }
            }
        };
        let draw_ring_stations = |grid: &mut Grid,
                                  cx: i32,
                                  cy: i32,
                                  rx: f32,
                                  ry: f32,
                                  count: usize,
                                  phase: f32,
                                  rung: usize,
                                  fg: Color| {
            let mut count = count.clamp(4, 16);
            if count % 2 == 1 {
                count += 1;
            }
            for i in 0..count {
                let sector = (i * 8 / count) % 8;
                let kind = match (rung + sector) % 8 {
                    0 => 2,
                    1 => 3,
                    2 => 1,
                    3 => 0,
                    4 => 7,
                    5 => 4,
                    6 => 6,
                    _ => 5,
                };
                let a = phase + i as f32 * std::f32::consts::TAU / count as f32;
                let wobble = if (i + rung) % 2 == 0 { 1.006 } else { 0.994 };
                let p = point_on(cx, cy, rx * wobble, ry * wobble, a);
                let color = if sector % 2 == 0 {
                    lighten(fg, 8)
                } else {
                    darken(fg, 4)
                };
                draw_geo_station(grid, p.0, p.1, kind + rung * 3 + i, color);
            }
        };

        let cx = width as i32 / 2;
        let cy = height as i32 / 2;
        let max_rx = (width as f32 / 2.0 - 3.0).max(10.0);
        let max_ry = (height as f32 / 2.0 - 2.0).max(5.0);
        let phase = -std::f32::consts::FRAC_PI_2 + rng.random_range(-0.16..0.16);

        for y in 0..height {
            for x in 0..width {
                let dx = (x as i32 - cx) as f32 / (max_rx * 0.98);
                let dy = (y as i32 - cy) as f32 / (max_ry * 0.98);
                let metric = dx * dx + dy * dy;
                if metric < 0.90 {
                    blank(&mut grid, x as i32, y as i32);
                } else if metric < 1.08 && (x + y + seed as usize) % 4 == 0 {
                    grid[y][x] = Cell::new('·', darken(chalk, 55));
                }
            }
        }

        for band in 0..1 {
            let rx = max_rx - band as f32 * 4.8;
            let ry = max_ry - band as f32 * 1.95;
            let fg = match band {
                0 => ether,
                _ => chalk,
            };
            let segments = (ring_count + band * 2 + 2).max(5);
            let gap = 0.16 + band as f32 * 0.045;
            for seg in 0..segments {
                let a0 = phase + seg as f32 * std::f32::consts::TAU / segments as f32 + gap;
                let a1 = phase + (seg + 1) as f32 * std::f32::consts::TAU / segments as f32 - gap;
                draw_arc(
                    &mut grid,
                    cx,
                    cy,
                    rx,
                    ry,
                    a0,
                    a1,
                    darken(fg, (band * 5) as u8),
                    lighten(fg, 12),
                    if band == 1 { 24 } else { 0 },
                    ornament_step + band * 3,
                );
            }
        }

        let mut ring_specs = Vec::new();
        for r in 0..ring_count {
            let t = (r + 1) as f32 / (ring_count + 1) as f32;
            let scale = 0.24 + t * 0.52 + rng.random_range(-0.012..0.016);
            let nodes = (base_nodes + r + (seed as usize % 2)).clamp(4, 10);
            let ring_phase = phase + r as f32 * 0.29 + rng.random_range(-0.13..0.13);
            let fg = match r % 5 {
                0 => gold,
                1 => ether,
                2 => chalk,
                3 => verdigris,
                _ => rose,
            };
            ring_specs.push((scale, nodes, ring_phase, fg));
            draw_arc(
                &mut grid,
                cx,
                cy,
                max_rx * scale,
                max_ry * scale,
                ring_phase,
                ring_phase + std::f32::consts::TAU,
                darken(fg, 7),
                lighten(fg, 12),
                if r % 2 == 0 { 21 } else { 0 },
                ornament_step + r * 3,
            );

            for n in 0..nodes {
                let a = ring_phase + n as f32 * std::f32::consts::TAU / nodes as f32;
                let p = point_on(cx, cy, max_rx * scale, max_ry * scale, a);
                draw_micro_shape(&mut grid, p.0, p.1, n + r * 5, fg);
                if n % 3 == 0 {
                    let inner = point_on(
                        cx,
                        cy,
                        max_rx * (scale - 0.035).max(0.05),
                        max_ry * (scale - 0.035).max(0.04),
                        a + 0.025,
                    );
                    let outer = point_on(
                        cx,
                        cy,
                        max_rx * (scale + 0.035).min(0.98),
                        max_ry * (scale + 0.035).min(0.98),
                        a - 0.025,
                    );
                    let control = point_on(cx, cy, max_rx * scale, max_ry * scale, a + 0.10);
                    draw_curve(
                        &mut grid,
                        inner,
                        control,
                        outer,
                        darken(fg, 8),
                        lighten(fg, 10),
                        n + r,
                        ornament_step + 8,
                    );
                }
            }
        }

        for path in 0..path_count {
            let base_a = phase
                + path as f32 * std::f32::consts::TAU / path_count as f32
                + rng.random_range(-0.10..0.10);
            let path_color = match path % 5 {
                0 => ether,
                1 => gold,
                2 => chalk,
                3 => verdigris,
                _ => rose,
            };
            let mut prev = point_on(cx, cy, max_rx * 0.42, max_ry * 0.36, base_a);
            for (ri, &(scale, _, ring_phase, ring_color)) in ring_specs.iter().enumerate() {
                let wobble =
                    (path as f32 * 0.73 + ri as f32 * 1.11 + seed as f32 * 0.013).sin() * 0.22;
                let a = base_a + (ring_phase - phase) * 0.22 + wobble;
                let target = point_on(cx, cy, max_rx * scale, max_ry * scale, a);
                let mid_scale = (scale + 0.06).min(0.98);
                let bend = if (path + ri) % 2 == 0 { 0.48 } else { -0.48 };
                let control = point_on(
                    cx,
                    cy,
                    max_rx * mid_scale,
                    max_ry * mid_scale,
                    a + bend + rng.random_range(-0.08..0.08),
                );
                draw_curve(
                    &mut grid,
                    prev,
                    control,
                    target,
                    darken(path_color, (ri * 4) as u8),
                    lighten(ring_color, 10),
                    path + ri * 3,
                    ornament_step + 3 + (path + ri) % 5,
                );
                if (path + ri) % 5 == 0 {
                    let hook = point_on(
                        cx,
                        cy,
                        max_rx * (scale + 0.045).min(0.98),
                        max_ry * (scale + 0.045).min(0.98),
                        a + 0.18,
                    );
                    let hook_control = point_on(cx, cy, max_rx * scale, max_ry * scale, a + 0.34);
                    draw_curve(
                        &mut grid,
                        target,
                        hook_control,
                        hook,
                        darken(ring_color, 10),
                        path_color,
                        path + ri + 5,
                        ornament_step + 10,
                    );
                }
                draw_micro_shape(
                    &mut grid,
                    target.0,
                    target.1,
                    path + ri + seed as usize,
                    shift_hue(lighten(path_color, 5), (ri * 18) as f64),
                );
                prev = target;
            }
            let outer_a = base_a + rng.random_range(-0.08..0.08);
            let outer = point_on(cx, cy, max_rx * 0.98, max_ry * 0.98, outer_a);
            let control = point_on(
                cx,
                cy,
                max_rx * 0.86,
                max_ry * 0.86,
                outer_a + rng.random_range(-0.55..0.55),
            );
            draw_curve(
                &mut grid,
                prev,
                control,
                outer,
                darken(path_color, 6),
                lighten(path_color, 12),
                path + 11,
                ornament_step,
            );
            draw_micro_shape(
                &mut grid,
                outer.0,
                outer.1,
                path + 7,
                lighten(path_color, 12),
            );
        }

        let bridge_count = (path_count / 2).min(2);
        for bridge in 0..bridge_count {
            let scale = rng.random_range(0.52..0.86);
            let a0 = phase
                + bridge as f32 * std::f32::consts::TAU / bridge_count as f32
                + rng.random_range(-0.10..0.10);
            let a2 = a0 + rng.random_range(0.30..0.92);
            let p0 = point_on(cx, cy, max_rx * scale, max_ry * scale, a0);
            let p2 = point_on(cx, cy, max_rx * scale, max_ry * scale, a2);
            let ctrl_scale = (scale + rng.random_range(-0.10..0.13)).clamp(0.16, 0.98);
            let p1 = point_on(
                cx,
                cy,
                max_rx * ctrl_scale,
                max_ry * ctrl_scale,
                (a0 + a2) * 0.5 + rng.random_range(-0.45..0.45),
            );
            let color = match bridge % 5 {
                0 => darken(ether, 12),
                1 => darken(gold, 10),
                2 => darken(rose, 8),
                3 => darken(verdigris, 8),
                _ => darken(chalk, 18),
            };
            draw_curve(
                &mut grid,
                p0,
                p1,
                p2,
                color,
                lighten(color, 16),
                bridge,
                ornament_step + 8 + bridge % 7,
            );
        }

        let outer_belts = [
            (0.935_f32, 0.925_f32, darken(ether, 14)),
            (0.960_f32, 0.955_f32, lighten(ether, 5)),
            (0.985_f32, 0.982_f32, darken(chalk, 2)),
        ];
        for (i, &(sx, sy, color)) in outer_belts.iter().enumerate() {
            draw_arc(
                &mut grid,
                cx,
                cy,
                max_rx * sx,
                max_ry * sy,
                phase + i as f32 * 0.035,
                phase + i as f32 * 0.035 + std::f32::consts::TAU,
                color,
                lighten(color, 10),
                0,
                96,
            );
            draw_ring_stations(
                &mut grid,
                cx,
                cy,
                max_rx * sx,
                max_ry * sy,
                station_base + i * 2,
                phase + i as f32 * 0.29,
                i,
                color,
            );
        }
        draw_ring_ticks(
            &mut grid,
            cx,
            cy,
            max_rx * 0.960,
            max_ry * 0.955,
            10,
            phase + std::f32::consts::PI / 10.0,
            darken(gold, 4),
        );

        let core_rx = (max_rx * 0.43).round() as i32;
        let core_ry = (max_ry * 0.37).round() as i32;
        for dy in -core_ry..=core_ry {
            for dx in -core_rx..=core_rx {
                let metric = (dx as f32 / core_rx.max(1) as f32).powi(2)
                    + (dy as f32 / core_ry.max(1) as f32).powi(2);
                if metric <= 1.0 {
                    blank(&mut grid, cx + dx, cy + dy);
                }
            }
        }
        let thick_rings = [(0.34, 0.29, ether)];
        for (i, &(rxs, rys, color)) in thick_rings.iter().enumerate() {
            for belt in [0.0_f32] {
                draw_arc(
                    &mut grid,
                    cx,
                    cy,
                    max_rx * (rxs + belt),
                    max_ry * (rys + belt * 0.8),
                    phase + i as f32 * 0.08,
                    phase + i as f32 * 0.08 + std::f32::consts::TAU,
                    darken(color, 10),
                    lighten(color, 8),
                    0,
                    64,
                );
            }
            draw_ring_stations(
                &mut grid,
                cx,
                cy,
                max_rx * rxs,
                max_ry * rys,
                station_base,
                phase + i as f32 * 0.41 + 0.17,
                i + 10,
                color,
            );
        }
        let seal_nodes = 6;
        let mut outer_nodes = Vec::new();
        let mut inner_nodes = Vec::new();
        for n in 0..seal_nodes {
            let a = phase + n as f32 * std::f32::consts::TAU / seal_nodes as f32;
            let outer = point_on(cx, cy, max_rx * 0.36, max_ry * 0.31, a);
            let inner = point_on(
                cx,
                cy,
                max_rx * 0.18,
                max_ry * 0.16,
                a + std::f32::consts::PI / 6.0,
            );
            outer_nodes.push(outer);
            inner_nodes.push(inner);
        }
        for n in 0..seal_nodes {
            draw_micro_shape(
                &mut grid,
                outer_nodes[n].0,
                outer_nodes[n].1,
                n + 2,
                if n % 2 == 0 { gold } else { ether },
            );
            put(
                &mut grid,
                inner_nodes[n].0,
                inner_nodes[n].1,
                ['△', '□', '◇', '○', '▽', '⊕'][n],
                lighten(chalk, 8),
            );
        }

        for dy in -3i32..=3i32 {
            for dx in -9i32..=9i32 {
                let metric = (dx as f32 / 9.0).powi(2) + (dy as f32 / 3.0).powi(2);
                if metric <= 1.0 {
                    blank(&mut grid, cx + dx, cy + dy);
                }
            }
        }
        draw_diamond_frame(&mut grid, cx, cy, 8, 3, darken(ether, 6));
        draw_box_frame(&mut grid, cx, cy, 5, 2, lighten(gold, 8));
        draw_line(&mut grid, cx - 7, cy, cx + 7, cy, lighten(chalk, 10));
        draw_line(&mut grid, cx, cy - 4, cx, cy + 4, lighten(chalk, 10));
        put(&mut grid, cx - 2, cy - 1, '╭', lighten(gold, 10));
        put(&mut grid, cx + 2, cy - 1, '╮', lighten(gold, 10));
        put(&mut grid, cx - 2, cy + 1, '╰', lighten(gold, 10));
        put(&mut grid, cx + 2, cy + 1, '╯', lighten(gold, 10));
        put(&mut grid, cx - 1, cy - 1, '─', lighten(gold, 10));
        put(&mut grid, cx, cy - 1, '⊛', lighten(rose, 18));
        put(&mut grid, cx + 1, cy - 1, '─', lighten(gold, 10));
        put(&mut grid, cx - 1, cy + 1, '─', lighten(gold, 10));
        put(&mut grid, cx, cy + 1, '☉', lighten(ether, 16));
        put(&mut grid, cx + 1, cy + 1, '─', lighten(gold, 10));
    (grid, false)
}

/// Dispatch arm for mode(s): fa5, fullmetal-alchemist5 (moved verbatim from run()).
pub(crate) fn cli_fa5_fullmetal_alchemist5(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        // fa5 [polys=0] [skew=0] [chords=0] [stations=0] -- inscribed polygon star array
        let poly_arg: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(0);
        let poly_count = if poly_arg == 0 {
            3 + ((seed as usize * 23 + 5) % 4)
        } else {
            poly_arg.clamp(2, 7)
        };
        let skew_arg: usize = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(0);
        let skew_override = if skew_arg == 0 {
            None
        } else {
            Some(skew_arg.clamp(1, 4))
        };
        let chord_arg: usize = args.get(6).and_then(|s| s.parse().ok()).unwrap_or(0);
        let chord_pairs = if chord_arg == 0 {
            2 + (seed as usize % 4)
        } else {
            chord_arg.clamp(0, 8)
        };
        let station_arg: usize = args.get(7).and_then(|s| s.parse().ok()).unwrap_or(0);
        let station_base = if station_arg == 0 {
            6 + (seed as usize % 5)
        } else {
            station_arg.clamp(4, 16)
        };

        let bg = darken(palette[0], 12);
        let chalk = lighten(palette[4], 14);
        let gold = lighten(palette[1], 32);
        let ether = shift_hue(lighten(palette[3], 36), 35.0);
        let rose = shift_hue(lighten(palette[2], 42), -38.0);
        let verdigris = shift_hue(lighten(palette[1], 26), 92.0);
        let hush = darken(palette[2], 66);
        let ring_colors = [chalk, gold, ether, rose, verdigris, chalk, gold];

        for y in 0..height {
            for x in 0..width {
                let n = (x * 31 + y * 43 + seed as usize * 13) % 353;
                let ch = match n {
                    0 => '·',
                    1 if (x + y + seed as usize) % 9 == 0 => '°',
                    _ => ' ',
                };
                grid[y][x] = if ch == ' ' {
                    Cell::new(' ', bg)
                } else {
                    Cell::new(ch, hush)
                };
            }
        }

        let put = |grid: &mut Grid, x: i32, y: i32, ch: char, fg: Color| {
            if x >= 0 && y >= 0 && (x as usize) < width && (y as usize) < height {
                grid[y as usize][x as usize] = Cell::new(ch, fg);
            }
        };
        let blank = |grid: &mut Grid, x: i32, y: i32| {
            if x >= 0 && y >= 0 && (x as usize) < width && (y as usize) < height {
                grid[y as usize][x as usize] = Cell::blank();
            }
        };
        let point_on = |cx: i32, cy: i32, rx: f32, ry: f32, angle: f32| {
            (
                cx + (angle.cos() * rx).round() as i32,
                cy + (angle.sin() * ry).round() as i32,
            )
        };
        let stroke_char = |x0: i32, y0: i32, x1: i32, y1: i32| {
            let dx = x1 - x0;
            let dy = y1 - y0;
            if dx.abs() > dy.abs() * 2 {
                '─'
            } else if dy.abs() > dx.abs() * 2 {
                '│'
            } else if dx.signum() == dy.signum() {
                '╲'
            } else {
                '╱'
            }
        };
        let draw_line = |grid: &mut Grid, mut x0: i32, mut y0: i32, x1: i32, y1: i32, fg: Color| {
            let ch = stroke_char(x0, y0, x1, y1);
            let dx = (x1 - x0).abs();
            let sx = if x0 < x1 { 1 } else { -1 };
            let dy = -(y1 - y0).abs();
            let sy = if y0 < y1 { 1 } else { -1 };
            let mut err = dx + dy;
            loop {
                put(grid, x0, y0, ch, fg);
                if x0 == x1 && y0 == y1 {
                    break;
                }
                let e2 = 2 * err;
                if e2 >= dy {
                    err += dy;
                    x0 += sx;
                }
                if e2 <= dx {
                    err += dx;
                    y0 += sy;
                }
            }
        };
        let curve_char = |prev: (i32, i32), here: (i32, i32), next: (i32, i32)| {
            let dx1 = (here.0 - prev.0).signum();
            let dy1 = (here.1 - prev.1).signum();
            let dx2 = (next.0 - here.0).signum();
            let dy2 = (next.1 - here.1).signum();
            if (dx1, dy1) == (dx2, dy2) {
                if dy1 == 0 {
                    '─'
                } else if dx1 == 0 {
                    '│'
                } else if dx1 == dy1 {
                    '╲'
                } else {
                    '╱'
                }
            } else if dy1 == 0 && dx2 == 0 {
                match (dx1, dy2) {
                    (1, 1) => '╮',
                    (1, -1) => '╯',
                    (-1, 1) => '╭',
                    (-1, -1) => '╰',
                    _ => '╮',
                }
            } else if dx1 == 0 && dy2 == 0 {
                match (dy1, dx2) {
                    (1, 1) => '╰',
                    (1, -1) => '╯',
                    (-1, 1) => '╭',
                    (-1, -1) => '╮',
                    _ => '╰',
                }
            } else if dx1 != dx2 && dy1 != dy2 {
                match (dx1, dy1, dx2, dy2) {
                    (1, 1, 1, -1) | (-1, -1, -1, 1) => '╯',
                    (1, -1, 1, 1) | (-1, 1, -1, -1) => '╮',
                    (1, 1, -1, 1) | (-1, -1, 1, -1) => '╰',
                    (1, -1, -1, -1) | (-1, 1, 1, 1) => '╭',
                    _ => '○',
                }
            } else if dx2 == 0 || dx1 == 0 {
                '│'
            } else if dy2 == 0 || dy1 == 0 {
                '─'
            } else if dx2 == dy2 {
                '╲'
            } else {
                '╱'
            }
        };
        let draw_arc = |grid: &mut Grid,
                        cx: i32,
                        cy: i32,
                        rx: f32,
                        ry: f32,
                        start: f32,
                        end: f32,
                        fg: Color,
                        gap: usize| {
            let span = (end - start).abs().max(0.05);
            let samples = ((rx + ry) * span * 3.8).max(20.0) as usize;
            let mut pts: Vec<(i32, i32)> = Vec::new();
            let mut flush = |pts: &mut Vec<(i32, i32)>| {
                if pts.len() > 2 {
                    for p in 1..pts.len() - 1 {
                        let ch = curve_char(pts[p - 1], pts[p], pts[p + 1]);
                        put(grid, pts[p].0, pts[p].1, ch, fg);
                    }
                }
                pts.clear();
            };
            for i in 0..=samples {
                if gap > 0 && i % gap == gap - 1 {
                    flush(&mut pts);
                    continue;
                }
                let a = start + (end - start) * i as f32 / samples as f32;
                let p = point_on(cx, cy, rx, ry, a);
                if pts.last().copied() != Some(p) {
                    pts.push(p);
                }
            }
            flush(&mut pts);
        };
        let draw_micro_shape = |grid: &mut Grid, x: i32, y: i32, kind: usize, fg: Color| match kind % 10 {
            0 => {
                put(grid, x, y, '○', lighten(fg, 10));
                put(grid, x - 1, y, '╴', darken(fg, 12));
                put(grid, x + 1, y, '╶', darken(fg, 12));
            }
            1 => {
                put(grid, x, y, '◇', lighten(fg, 12));
                put(grid, x, y - 1, '╭', fg);
                put(grid, x, y + 1, '╯', fg);
            }
            2 => {
                put(grid, x - 1, y - 1, '╭', fg);
                put(grid, x + 1, y - 1, '╮', fg);
                put(grid, x - 1, y + 1, '╰', fg);
                put(grid, x + 1, y + 1, '╯', fg);
                put(grid, x, y, '⊙', lighten(fg, 10));
            }
            3 => {
                put(grid, x, y - 1, '△', lighten(fg, 8));
                put(grid, x - 1, y, '╰', fg);
                put(grid, x + 1, y, '╯', fg);
                put(grid, x, y + 1, '╰', darken(fg, 8));
            }
            4 => {
                put(grid, x, y, '☉', lighten(fg, 14));
                put(grid, x - 1, y - 1, '╭', darken(fg, 4));
                put(grid, x + 1, y - 1, '╮', darken(fg, 4));
                put(grid, x - 1, y + 1, '╰', darken(fg, 4));
                put(grid, x + 1, y + 1, '╯', darken(fg, 4));
            }
            5 => {
                put(grid, x, y, '⌬', lighten(fg, 8));
                put(grid, x - 1, y, '╭', fg);
                put(grid, x + 1, y, '╮', fg);
            }
            6 => {
                put(grid, x, y, '□', lighten(fg, 8));
                put(grid, x - 1, y, '◜', fg);
                put(grid, x + 1, y, '◝', fg);
                put(grid, x, y + 1, '╯', darken(fg, 10));
            }
            7 => {
                put(grid, x, y, '⊕', lighten(fg, 10));
                put(grid, x - 1, y, '─', fg);
                put(grid, x + 1, y, '─', fg);
                put(grid, x, y - 1, '│', fg);
                put(grid, x, y + 1, '│', fg);
            }
            8 => {
                put(grid, x, y, '✦', lighten(fg, 16));
                put(grid, x - 1, y, '◌', fg);
                put(grid, x + 1, y, '◌', fg);
            }
            _ => {
                put(grid, x, y, '∴', lighten(fg, 10));
                put(grid, x - 1, y - 1, '·', fg);
                put(grid, x + 1, y + 1, '·', fg);
                put(grid, x + 1, y - 1, '·', darken(fg, 10));
            }
        };
        let draw_ring_ticks = |grid: &mut Grid,
                               cx: i32,
                               cy: i32,
                               rx: f32,
                               ry: f32,
                               count: usize,
                               phase: f32,
                               fg: Color| {
            for i in 0..count {
                let a = phase + i as f32 * std::f32::consts::TAU / count as f32;
                let p = point_on(cx, cy, rx, ry, a);
                if a.sin().abs() > a.cos().abs() {
                    put(grid, p.0 - 1, p.1, '─', fg);
                    put(grid, p.0, p.1, '┼', lighten(fg, 8));
                    put(grid, p.0 + 1, p.1, '─', fg);
                } else {
                    put(grid, p.0, p.1 - 1, '│', fg);
                    put(grid, p.0, p.1, '┼', lighten(fg, 8));
                    put(grid, p.0, p.1 + 1, '│', fg);
                }
            }
        };
        let draw_polygon = |grid: &mut Grid,
                            cx: i32,
                            cy: i32,
                            rx: f32,
                            ry: f32,
                            n: usize,
                            phase: f32,
                            fg: Color|
         -> Vec<(i32, i32)> {
            let n = n.max(3);
            let mut verts = Vec::with_capacity(n);
            for i in 0..n {
                let a = phase + i as f32 * std::f32::consts::TAU / n as f32;
                verts.push(point_on(cx, cy, rx, ry, a));
            }
            for i in 0..n {
                let (ax, ay) = verts[i];
                let (bx, by) = verts[(i + 1) % n];
                draw_line(grid, ax, ay, bx, by, fg);
            }
            verts
        };
        let draw_star = |grid: &mut Grid, verts: &[(i32, i32)], k: usize, fg: Color| {
            let n = verts.len();
            if k == 0 || n < 3 {
                return;
            }
            for i in 0..n {
                let j = (i + k) % n;
                if j != i {
                    let (ax, ay) = verts[i];
                    let (bx, by) = verts[j];
                    draw_line(grid, ax, ay, bx, by, fg);
                }
            }
        };

        let cx = width as i32 / 2;
        let cy = height as i32 / 2;
        let max_rx = (width as f32 / 2.0 - 3.0).max(10.0);
        let max_ry = (height as f32 / 2.0 - 2.0).max(5.0);
        let base_phase = -std::f32::consts::FRAC_PI_2 + rng.random_range(-0.12..0.12);

        for y in 0..height {
            for x in 0..width {
                let dx = (x as i32 - cx) as f32 / (max_rx * 0.96);
                let dy = (y as i32 - cy) as f32 / (max_ry * 0.96);
                let metric = dx * dx + dy * dy;
                if metric < 0.92 {
                    blank(&mut grid, x as i32, y as i32);
                } else if metric < 1.08 && (x + y + seed as usize) % 4 == 0 {
                    grid[y][x] = Cell::new('·', darken(chalk, 55));
                }
            }
        }

        for &(sx, sy, color) in [
            (0.965_f32, 0.955_f32, darken(ether, 12)),
            (0.985_f32, 0.978_f32, lighten(chalk, 4)),
        ]
        .iter()
        {
            draw_arc(
                &mut grid,
                cx,
                cy,
                max_rx * sx,
                max_ry * sy,
                base_phase,
                base_phase + std::f32::consts::TAU,
                color,
                0,
            );
        }
        draw_ring_ticks(
            &mut grid,
            cx,
            cy,
            max_rx * 0.975,
            max_ry * 0.965,
            station_base,
            base_phase,
            darken(gold, 6),
        );

        let scale_lo = 0.30_f32;
        let scale_hi = 0.86_f32;
        let mut ring_specs: Vec<(usize, f32, f32, Color)> = Vec::new();
        for r in 0..poly_count {
            let t = if poly_count > 1 {
                r as f32 / (poly_count - 1) as f32
            } else {
                0.5
            };
            let scale = scale_lo + t * (scale_hi - scale_lo) + rng.random_range(-0.014..0.014);
            let n = 3 + ((seed as usize * (r * 7 + 3) + 11) % 6);
            let phase = base_phase
                + r as f32 * 0.41
                + rng.random_range(-0.20..0.20);
            let color = ring_colors[r % ring_colors.len()];
            ring_specs.push((n, scale, phase, color));
        }

        let mut all_verts: Vec<Vec<(i32, i32)>> = Vec::new();
        for &(n, scale, phase, color) in ring_specs.iter() {
            let rx = max_rx * scale;
            let ry = max_ry * scale;
            let verts = draw_polygon(&mut grid, cx, cy, rx, ry, n, phase, darken(color, 6));
            if n >= 5 {
                let max_k = (n - 1) / 2;
                let k = skew_override
                    .unwrap_or_else(|| 2 + (seed as usize + n) % (max_k - 1).max(1))
                    .min(max_k)
                    .max(2);
                draw_star(&mut grid, &verts, k, lighten(color, 10));
            } else {
                let twin_phase = phase + std::f32::consts::TAU / (2 * n) as f32;
                draw_polygon(&mut grid, cx, cy, rx, ry, n, twin_phase, darken(color, 14));
            }
            for (i, &(vx, vy)) in verts.iter().enumerate() {
                draw_micro_shape(&mut grid, vx, vy, i + n + (seed as usize % 5), color);
            }
            all_verts.push(verts);
        }

        for r in 1..all_verts.len() {
            let outer = &all_verts[r];
            let inner = &all_verts[r - 1];
            if outer.is_empty() || inner.is_empty() {
                continue;
            }
            let chord_color = match r % 5 {
                0 => darken(verdigris, 8),
                1 => darken(rose, 6),
                2 => darken(ether, 10),
                3 => darken(gold, 6),
                _ => darken(chalk, 14),
            };
            let offset = (seed as usize + r * 3) % inner.len();
            let steps = chord_pairs.min(outer.len());
            for c in 0..steps {
                let oi = if steps > 0 {
                    (c * outer.len()) / steps
                } else {
                    c
                };
                let ii = (oi + offset) % inner.len();
                let (ox, oy) = outer[oi];
                let (ix, iy) = inner[ii];
                draw_line(&mut grid, ox, oy, ix, iy, chord_color);
            }
        }

        let core_rx = (max_rx * 0.22).round() as i32;
        let core_ry = (max_ry * 0.22).round() as i32;
        for dy in -core_ry..=core_ry {
            for dx in -core_rx..=core_rx {
                let metric = (dx as f32 / core_rx.max(1) as f32).powi(2)
                    + (dy as f32 / core_ry.max(1) as f32).powi(2);
                if metric <= 1.0 {
                    blank(&mut grid, cx + dx, cy + dy);
                }
            }
        }
        draw_arc(
            &mut grid,
            cx,
            cy,
            max_rx * 0.205,
            max_ry * 0.205,
            base_phase,
            base_phase + std::f32::consts::TAU,
            darken(ether, 8),
            0,
        );
        let core_n = 3 + (seed as usize % 4);
        let core_verts = draw_polygon(
            &mut grid,
            cx,
            cy,
            max_rx * 0.16,
            max_ry * 0.16,
            core_n,
            base_phase,
            lighten(gold, 8),
        );
        if core_n >= 5 {
            draw_star(&mut grid, &core_verts, 2, lighten(ether, 12));
        } else {
            let twin_phase = base_phase + std::f32::consts::TAU / (2 * core_n) as f32;
            draw_polygon(
                &mut grid,
                cx,
                cy,
                max_rx * 0.16,
                max_ry * 0.16,
                core_n,
                twin_phase,
                darken(ether, 10),
            );
        }
        let core_ring_n = 6;
        for i in 0..core_ring_n {
            let a = base_phase + i as f32 * std::f32::consts::TAU / core_ring_n as f32;
            let p = point_on(cx, cy, max_rx * 0.10, max_ry * 0.10, a);
            put(
                &mut grid,
                p.0,
                p.1,
                ['△', '◇', '○', '□', '▽', '⊕'][i],
                lighten(chalk, 8),
            );
        }
        put(&mut grid, cx, cy, '⊙', lighten(rose, 18));
    (grid, false)
}
