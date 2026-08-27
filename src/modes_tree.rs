#![allow(warnings)]

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
use crate::automata; use crate::avant; use crate::biomes; use crate::borders; use crate::color; use crate::content; use crate::fills; use crate::layout; use crate::markdown; use crate::mondrian; use crate::render; use crate::scene; use crate::sprites; use crate::tree_draw; use crate::types; use crate::walker;
use crate::cli::*;
use crate::gridio::*;
use crate::ink::*;
use crate::modes_creatures::*;
use crate::modes_geo::*;
use crate::modes_sky::*;
use crate::morph::*;
use crate::opts::*;
use crate::pp::*;
use crate::registry::*;
use crate::warps::*;

// --- fullmetal-eyes++ : the multi-tier seal cranked. 4 arc bands, 3-4 star
//     polygons, node eyes on EVERY tier vertex, twin rune bands, hero eye. ---
pub(crate) fn draw_fme_pp(grid: &mut Grid, width: usize, height: usize, seed: u64, palette: &[Color; 5], rng: &mut StdRng) {
    use std::f32::consts::{FRAC_PI_2, TAU};
    let bg = darken(palette[0], 12);
    let chalk = lighten(palette[4], 12);
    let gold = lighten(palette[1], 30);
    let iris = lighten(palette[3], 30);
    let lid = lighten(palette[2], 18);
    let pupil = darken(palette[0], 2);
    let sclera = lighten(palette[4], 4);
    let shadow = darken(palette[2], 60);

    for y in 0..height {
        for x in 0..width {
            let n = (x * 19 + y * 43 + seed as usize * 5) % 151;
            let (ch, col) = match n {
                0 => ('·', shadow),
                1 => ('∙', shadow),
                _ => (' ', bg),
            };
            grid[y][x] = Cell::new(ch, col);
        }
    }

    let cx = width as i32 / 2;
    let cy = height as i32 / 2;
    let max_rx = (width as f32 / 2.0 - 4.0).max(12.0);
    let max_ry = (height as f32 / 2.0 - 2.0).max(5.0);
    let phase = seed as f32 * 0.031 - FRAC_PI_2;

    let lure_x = cx + (rng.random_range(-0.35f32..0.35) * max_rx * 0.9) as i32;
    let lure_y = cy + (rng.random_range(-0.30f32..0.30) * max_ry * 0.9) as i32;

    let gaze_for = |ex: i32, ey: i32, rx: i32, ry: i32| -> (i32, i32) {
        let dx = (lure_x - ex) as f32;
        let dy = (lure_y - ey) as f32;
        let d = (dx * dx + dy * dy).sqrt().max(1.0);
        let gx = ((dx / d) * (rx as f32 * 0.34)).round() as i32;
        let gy = ((dy / d) * (ry as f32 * 0.6)).round() as i32;
        (
            gx.clamp(-(rx / 3).max(1), (rx / 3).max(1)),
            gy.clamp(-(ry / 2).max(1), (ry / 2).max(1)),
        )
    };

    let draw_hero_eye = |grid: &mut Grid, ex: i32, ey: i32, rx: i32, ry: i32, gx: i32, gy: i32, iris_c: Color, lid_c: Color| {
        for dx in -rx..=rx {
            let nx = dx as f32 / rx.max(1) as f32;
            let curve = (1.0 - nx.abs().powf(1.6)).max(0.0).powf(0.55);
            let top = (-(ry as f32) * curve).round() as i32;
            let bottom = ((ry as f32) * 0.78 * curve).round() as i32;
            for dy in top..=bottom {
                if dy == top || dy == bottom {
                    let ch = if dx < -rx / 2 {
                        if dy == top { '╱' } else { '╲' }
                    } else if dx > rx / 2 {
                        if dy == top { '╲' } else { '╱' }
                    } else {
                        '─'
                    };
                    pp_put(grid, ex + dx, ey + dy, ch, lid_c);
                } else {
                    let ix = (dx - gx) as f32 / (rx as f32 * 0.42);
                    let iy = (dy - gy) as f32 / ry.max(1) as f32;
                    let im = ix * ix + iy * iy;
                    if im <= 0.16 {
                        pp_put(grid, ex + dx, ey + dy, '◉', pupil);
                    } else if im <= 1.0 {
                        let ang = (iy.atan2(ix) + TAU) % TAU;
                        let fiber = ((ang / (TAU / 16.0)).round() as i32) % 2 == 0;
                        let ch = if im > 0.78 { '○' } else if fiber { '╎' } else { '·' };
                        pp_put(grid, ex + dx, ey + dy, ch, iris_c);
                    } else {
                        pp_put(grid, ex + dx, ey + dy, '·', sclera);
                    }
                }
            }
        }
        pp_put(grid, ex + gx - 1, ey + gy - 1, '˙', chalk);
        pp_put(grid, ex + gx, ey + gy - 1, '˙', chalk);
    };

    let draw_node_eye = |grid: &mut Grid, ncx: i32, ncy: i32, rx: i32, ry: i32, gx: i32, gy: i32, lid_c: Color, iris_c: Color| {
        for dx in -rx..=rx {
            let nx = dx as f32 / rx.max(1) as f32;
            let curve = (1.0 - nx.abs().powf(1.65)).max(0.0).powf(0.55);
            let top = (-(ry as f32) * curve).round() as i32;
            let bottom = ((ry as f32) * 0.75 * curve).round() as i32;
            for dy in top..=bottom {
                if dy == top || dy == bottom {
                    let ch = if dx < -rx / 2 {
                        if dy == top { '╱' } else { '╲' }
                    } else if dx > rx / 2 {
                        if dy == top { '╲' } else { '╱' }
                    } else {
                        '─'
                    };
                    pp_put(grid, ncx + dx, ncy + dy, ch, lid_c);
                } else {
                    let ix = (dx - gx) as f32 / (rx as f32 * 0.5);
                    let iy = (dy - gy) as f32 / ry.max(1) as f32;
                    let im = ix * ix + iy * iy;
                    if im <= 0.5 {
                        pp_put(grid, ncx + dx, ncy + dy, '◉', pupil);
                    } else if im <= 1.0 {
                        pp_put(grid, ncx + dx, ncy + dy, '·', iris_c);
                    }
                }
            }
        }
        pp_put(grid, ncx + gx - 1, ncy + gy - 1, '˙', chalk);
    };

    let runes = [
        '△', '▽', '□', '◇', '☉', '☽', '☿', '♄', '♃', '✦', '∴', '∵', '⊕', '⊗', '✶', '◈',
    ];

    let band_count = 4;
    for i in 0..band_count {
        let t = i as f32 / (band_count - 1) as f32;
        let r = 0.46 + t * 0.46;
        let col = match i % 3 {
            0 => chalk,
            1 => gold,
            _ => iris,
        };
        let gap = if i % 3 == 2 { 9 } else { 0 };
        pp_arc(grid, cx, cy, max_rx * r, max_ry * r, phase, phase + TAU, col, gap);
    }

    let tier_count = 3 + seed as usize % 2;
    let mut tier_verts: Vec<Vec<(i32, i32)>> = Vec::new();
    for ti in 0..tier_count {
        let rad = 0.40 + ti as f32 * 0.15;
        let n = 5 + (seed as usize + ti * 7) % 6;
        let rot = phase + ti as f32 * 0.4 + seed as f32 * 0.01 * ti as f32;
        let mut verts: Vec<(i32, i32)> = Vec::with_capacity(n);
        for i in 0..n {
            let a = rot + i as f32 * TAU / n as f32;
            verts.push(pp_point_on(cx, cy, max_rx * rad, max_ry * rad, a));
        }
        let max_k = ((n - 1) / 2).max(1);
        let k = if max_k >= 2 { 2 + (seed as usize + ti) % (max_k - 1) } else { 1 };
        let col = if ti % 2 == 0 { gold } else { chalk };
        for i in 0..n {
            let (ax, ay) = verts[i];
            let (bx, by) = verts[(i + k.min(max_k)) % n];
            pp_line(grid, ax, ay, bx, by, darken(col, 8));
        }
        for &v in &verts {
            pp_put(grid, v.0, v.1, '◇', lighten(col, 8));
        }
        tier_verts.push(verts);
    }

    for (bi, &r) in [0.92f32, 0.62].iter().enumerate() {
        let ins_n = ((max_rx + max_ry) * r * 0.5).round().clamp(14.0, 40.0) as usize;
        for i in 0..ins_n {
            let a = phase + i as f32 * TAU / ins_n as f32;
            let p = pp_point_on(cx, cy, max_rx * r, max_ry * r, a);
            pp_put(grid, p.0, p.1, runes[(i + seed as usize + bi * 3) % runes.len()], lighten(gold, 8));
        }
    }

    for (ti, verts) in tier_verts.iter().enumerate() {
        let (rx, ry) = if ti + 1 == tier_count { (5, 2) } else { (4, 2) };
        for (i, &(nx, ny)) in verts.iter().enumerate() {
            let (gx, gy) = gaze_for(nx, ny, rx, ry);
            draw_node_eye(grid, nx, ny, rx, ry, gx, gy, darken(lid, 6), shift_hue(iris, (i as f64 * 40.0 + ti as f64 * 17.0) % 360.0));
            pp_put(grid, nx, ny + ry + 1, runes[(i + ti) % runes.len()], lighten(gold, 12));
        }
    }

    let hero_rx = (width as f32 * 0.13) as i32;
    let hero_ry = (height as f32 * 0.15) as i32;
    let (hgx, hgy) = gaze_for(cx, cy, hero_rx, hero_ry);
    draw_hero_eye(grid, cx, cy, hero_rx, hero_ry, hgx, hgy, lighten(iris, 16), lighten(lid, 10));

    pp_arc(grid, lure_x, lure_y, 3.0, 1.5, 0.0, TAU, shift_hue(gold, 40.0), 4);
    pp_put(grid, lure_x, lure_y, '◆', lighten(gold, 20));

    for _ in 0..(tier_count + 2) {
        let a = phase + rng.random::<f32>() * TAU;
        let p1 = pp_point_on(cx, cy, max_rx * rng.random_range(0.22..0.45), max_ry * rng.random_range(0.22..0.45), a);
        let p2 = pp_point_on(cx, cy, max_rx * rng.random_range(0.60..0.90), max_ry * rng.random_range(0.60..0.90), a + rng.random_range(0.3..1.3));
        pp_line(grid, p1.0, p1.1, p2.0, p2.1, darken(iris, 18));
    }
}


// --- trees++ : a lush grounded gallery of tree variants on grassy hillocks,
//     varied spreads + hues, fruit/flower accents, no debug labels. ---
pub(crate) fn draw_trees_pp(grid: &mut Grid, width: usize, height: usize, seed: u64, palette: &[Color; 5], rng: &mut StdRng) {
    let cols = (width / 18).clamp(3, 6);
    let rows = (height / 14).clamp(2, 4);
    let cell_w = width / cols;
    let cell_h = height / rows;
    let grass = darken(palette[2], 30);
    let gc = ['v', 'w', '\u{2c4}', '\u{1d1b}'];

    for row in 0..rows {
        for col in 0..cols {
            let kind = (row * cols + col + seed as usize) % 19;
            let cx = col * cell_w + cell_w / 2;
            let ground_y = (row + 1) * cell_h - 2;
            let canopy_y = row * cell_h + 2;
            let spread = (cell_w / 4).max(3);
            // grass line under the tree
            for x in (col * cell_w)..((col + 1) * cell_w).min(width) {
                let gy = ground_y + 1;
                if gy < height {
                    grid[gy][x] = Cell::new(gc[(x + row) % gc.len()], grass);
                }
            }
            let base = palette[1 + kind % 3];
            let color = shift_hue(base, (kind as f64 * 23.0) % 120.0 - 60.0);
            draw_tree(grid, cx, ground_y, canopy_y, spread, kind, color, rng);
            // accent: fruit hanging in canopy or flower at the base
            if (row + col + seed as usize) % 2 == 0 {
                draw_flower(grid, (cx + spread).min(width.saturating_sub(1)), ground_y.saturating_sub(1), kind % 5, palette[3]);
            } else {
                draw_fruit(grid, cx.saturating_sub(spread / 2), (canopy_y + 3).min(height.saturating_sub(1)), kind % 5, lighten(palette[3], 10));
            }
        }
    }
}


// --- forest++ : layered depth. star sky + disc, far dark pines, mid mix,
//     foreground hero trees, grass band, scattered flowers/fruit. ---
pub(crate) fn draw_forest_pp(grid: &mut Grid, width: usize, height: usize, seed: u64, palette: &[Color; 5], rng: &mut StdRng) {
    let ground_y = height.saturating_sub(4);
    let ground_color = darken(palette[1], 90);
    let tiles = ['╱', '╲'];

    // sky (sparse) + ground (truchet)
    for y in 0..height {
        for x in 0..width {
            if y >= ground_y {
                grid[y][x] = Cell::new(tiles[(x + y) % 2], ground_color);
            } else {
                grid[y][x] = Cell::blank();
            }
        }
    }
    // stars in upper sky
    for _ in 0..(width / 2) {
        let x = rng.random_range(0..width);
        let y = rng.random_range(0..(ground_y / 2).max(1));
        let ch = if rng.random_range(0..4) == 0 { '✦' } else { '·' };
        grid[y][x] = Cell::new(ch, darken(palette[4], 60));
    }
    // sun / moon disc
    let disc_cx = (width / 5 + seed as usize % (width / 2).max(1)) as i32;
    let disc_cy = (2 + seed as usize % 3) as i32;
    let disc_col = lighten(palette[3], 14);
    for dy in -2i32..=2 {
        for dx in -4i32..=4 {
            let m = (dx as f32 / 4.0).powi(2) + (dy as f32 / 2.0).powi(2);
            if m <= 1.0 {
                pp_put(grid, disc_cx + dx, disc_cy + dy, '·', disc_col);
            }
        }
    }
    pp_arc(grid, disc_cx, disc_cy, 4.0, 2.0, 0.0, std::f32::consts::TAU, lighten(disc_col, 10), 0);

    // far hills: small, desaturated, dark pines on the horizon
    let far_color = darken(palette[2], 40);
    let mut x = 2usize;
    while x < width.saturating_sub(2) {
        let h = 4 + (x + seed as usize) % 3;
        draw_pine(grid, x, ground_y.saturating_sub(1), 3, h, far_color);
        x += 4 + (x + seed as usize) % 3;
    }

    // mid trees: medium mixed, slightly darkened
    let mid_color = darken(palette[1], 30);
    let mid_xs = [width / 6, width / 3, width / 2, (width * 2) / 3, (width * 5) / 6];
    for (i, &mx) in mid_xs.iter().enumerate() {
        let canopy = ground_y.saturating_sub(8);
        match (i + seed as usize) % 3 {
            0 => draw_pine(grid, mx, ground_y.saturating_sub(1), 4, 8, mid_color),
            1 => grow_tree(grid, mx, ground_y.saturating_sub(1), canopy, 4, mid_color, rng),
            _ => draw_palm(grid, mx, ground_y.saturating_sub(1), 9, darken(palette[3], 20), rng),
        }
    }

    // foreground hero trees (clear bounding boxes first; willow needs blanks)
    let clear = |grid: &mut Grid, x0: usize, x1: usize, y0: usize, y1: usize| {
        for yy in y0..y1.min(height) {
            for xx in x0..x1.min(width) {
                if yy < ground_y {
                    grid[yy][xx] = Cell::blank();
                }
            }
        }
    };
    let fg_a = width / 6;
    clear(grid, fg_a.saturating_sub(8), fg_a + 8, ground_y.saturating_sub(14), ground_y);
    grow_tree(grid, fg_a, ground_y.saturating_sub(1), ground_y.saturating_sub(13), 6, palette[1], rng);

    let fg_b = width / 2;
    draw_pine(grid, fg_b, ground_y.saturating_sub(1), 5, 12, palette[2]);

    let fg_c = (width * 3) / 4;
    clear(grid, fg_c.saturating_sub(9), fg_c + 9, ground_y.saturating_sub(16), ground_y);
    draw_willow(grid, fg_c, ground_y.saturating_sub(1), ground_y.saturating_sub(14), 7, palette[1]);

    let fg_d = width.saturating_sub(8);
    draw_palm(grid, fg_d, ground_y.saturating_sub(1), 15, palette[3], rng);

    // undergrowth: flowers + fallen fruit
    for _ in 0..(width / 6) {
        let fx = rng.random_range(1..width.saturating_sub(1));
        let fy = ground_y.saturating_sub(1);
        if rng.random_range(0..2) == 0 {
            draw_flower(grid, fx, fy, rng.random_range(0..5), palette[3]);
        } else {
            draw_fruit(grid, fx, ground_y, rng.random_range(0..5), rgb(200, 60, 50));
        }
    }
}


pub(crate) fn draw_fullmetal_eyes(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, args: &[String]) -> Grid {
        // fullmetal-eyes [nodes] [runes] -- alchemical eye seal with watching glyph nodes
        let node_count: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(8);
        let node_count = node_count.clamp(5, 12);
        let rune_count: usize = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(72);
        let rune_count = rune_count.clamp(16, 180);

        let bg = darken(palette[0], 12);
        let chalk = lighten(palette[4], 12);
        let gold = lighten(palette[1], 30);
        let iris = lighten(palette[3], 30);
        let lid = lighten(palette[2], 18);
        let pupil = darken(palette[0], 2);
        let shadow = darken(palette[2], 60);

        for y in 0..height {
            for x in 0..width {
                let n = (x * 19 + y * 43 + seed as usize * 5) % 151;
                let ch = match n {
                    0 => '·',
                    1 => '∙',
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
        let draw_arc = |grid: &mut Grid,
                        cx: i32,
                        cy: i32,
                        rx: f32,
                        ry: f32,
                        start: f32,
                        end: f32,
                        fg: Color,
                        gap: usize| {
            let samples = ((rx + ry) * 16.0).max(90.0) as usize;
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
        let draw_small_eye = |grid: &mut Grid,
                              cx: i32,
                              cy: i32,
                              rx: i32,
                              ry: i32,
                              lid_color: Color,
                              iris_color: Color,
                              style: usize| {
            for dx in -rx..=rx {
                let nx = dx as f32 / rx as f32;
                let curve = (1.0 - nx.abs().powf(1.65)).max(0.0).powf(0.55);
                let top = (-(ry as f32) * curve).round() as i32;
                let bottom = ((ry as f32) * 0.75 * curve).round() as i32;
                for dy in top..=bottom {
                    let x = cx + dx;
                    let y = cy + dy;
                    if dy == top || dy == bottom {
                        let ch = if dx < -rx / 2 {
                            if dy == top { '╱' } else { '╲' }
                        } else if dx > rx / 2 {
                            if dy == top { '╲' } else { '╱' }
                        } else {
                            '─'
                        };
                        put(grid, x, y, ch, lid_color);
                    } else {
                        let im = (dx as f32 / (rx as f32 * 0.32)).powi(2)
                            + (dy as f32 / ry.max(1) as f32).powi(2);
                        if im <= 0.18 {
                            put(grid, x, y, if style % 2 == 0 { '◐' } else { '◑' }, pupil);
                        } else if im <= 1.0 {
                            let ch = if im > 0.74 {
                                '○'
                            } else if (dx.abs() + dy.abs() + style as i32) % 4 == 0 {
                                '╎'
                            } else {
                                '·'
                            };
                            put(grid, x, y, ch, iris_color);
                        }
                    }
                }
            }
            put(grid, cx - 1, cy - 1, '˙', chalk);
        };

        let cx = width as i32 / 2;
        let cy = height as i32 / 2;
        let max_rx = (width as f32 / 2.0 - 4.0).max(12.0);
        let max_ry = (height as f32 / 2.0 - 2.0).max(5.0);
        let phase = seed as f32 * 0.031 - std::f32::consts::FRAC_PI_2 + t_anim * 0.12;

        for i in 0..3 {
            let rx = max_rx * (0.92 - i as f32 * 0.16);
            let ry = max_ry * (0.92 - i as f32 * 0.16);
            draw_arc(
                &mut grid,
                cx,
                cy,
                rx,
                ry,
                phase,
                phase + std::f32::consts::TAU,
                if i == 1 { gold } else { chalk },
                if i == 2 { 7 } else { 0 },
            );
        }

        let mut nodes = Vec::new();
        for i in 0..node_count {
            let a = phase + i as f32 * std::f32::consts::TAU / node_count as f32;
            let outer = point_on(cx, cy, max_rx * 0.82, max_ry * 0.82, a);
            let inner = point_on(cx, cy, max_rx * 0.45, max_ry * 0.45, a);
            nodes.push(outer);
            draw_line(
                &mut grid,
                inner.0,
                inner.1,
                outer.0,
                outer.1,
                darken(gold, 8),
            );
        }
        for i in 0..nodes.len() {
            let j = (i + 2) % nodes.len();
            draw_line(
                &mut grid,
                nodes[i].0,
                nodes[i].1,
                nodes[j].0,
                nodes[j].1,
                darken(chalk, 18),
            );
        }

        let runes = [
            '△', '▽', '□', '◇', '☉', '☽', '☿', '♄', '♃', '✦', '∴', '∵', '⊕', '⊗',
        ];
        for i in 0..rune_count {
            let lane = match i % 4 {
                0 => 0.92,
                1 => 0.74,
                2 => 0.58,
                _ => rng.random_range(0.38..0.88),
            };
            let a = phase
                + i as f32 / rune_count as f32 * std::f32::consts::TAU
                + rng.random_range(-0.035..0.035);
            let p = point_on(cx, cy, max_rx * lane, max_ry * lane, a);
            put(
                &mut grid,
                p.0,
                p.1,
                runes[(i + rng.random_range(0..runes.len())) % runes.len()],
                shift_hue(gold, rng.random_range(-50..=65) as f64),
            );
        }

        draw_small_eye(
            &mut grid,
            cx,
            cy,
            (width as i32 / 5).clamp(12, 20),
            (height as i32 / 5).clamp(4, 7),
            lighten(lid, 10),
            lighten(iris, 16),
            seed as usize,
        );
        for (i, &(nx, ny)) in nodes.iter().enumerate() {
            draw_small_eye(
                &mut grid,
                nx,
                ny,
                5 + (i as i32 % 2),
                2,
                darken(lid, 8),
                shift_hue(iris, i as f64 * 38.0),
                i,
            );
            put(
                &mut grid,
                nx,
                ny + 3,
                runes[i % runes.len()],
                lighten(gold, 12),
            );
        }

        for _ in 0..node_count {
            let a = phase + rng.random::<f32>() * std::f32::consts::TAU;
            let p1 = point_on(
                cx,
                cy,
                max_rx * rng.random_range(0.22..0.45),
                max_ry * rng.random_range(0.22..0.45),
                a,
            );
            let p2 = point_on(
                cx,
                cy,
                max_rx * rng.random_range(0.60..0.90),
                max_ry * rng.random_range(0.60..0.90),
                a + rng.random_range(0.3..1.3),
            );
            draw_line(&mut grid, p1.0, p1.1, p2.0, p2.1, darken(iris, 18));
        }
    grid
}

pub(crate) fn draw_fullmetal_eyes2(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, args: &[String]) -> Grid {
        // fullmetal-eyes2 [tiers=0] [runes=0] -- multi-tier watching seal; every eye tracks a seeded lure
        let tier_arg: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(0);
        let tier_count = if tier_arg == 0 {
            2 + (seed as usize % 2) // 2..3
        } else {
            tier_arg.clamp(1, 4)
        };
        let rune_arg: usize = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(0);
        let rune_count = if rune_arg == 0 {
            60 + (seed as usize % 60) // 60..119
        } else {
            rune_arg.clamp(16, 200)
        };

        let bg = darken(palette[0], 12);
        let chalk = lighten(palette[4], 12);
        let gold = lighten(palette[1], 30);
        let iris = lighten(palette[3], 30);
        let lid = lighten(palette[2], 18);
        let pupil = darken(palette[0], 2);
        let sclera = lighten(palette[4], 4);
        let shadow = darken(palette[2], 60);

        // bg haze
        for y in 0..height {
            for x in 0..width {
                let n = (x * 19 + y * 43 + seed as usize * 5) % 151;
                let ch = match n {
                    0 => '·',
                    1 => '∙',
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
        let draw_arc = |grid: &mut Grid,
                        cx: i32,
                        cy: i32,
                        rx: f32,
                        ry: f32,
                        start: f32,
                        end: f32,
                        fg: Color,
                        gap: usize| {
            let samples = ((rx + ry) * 16.0).max(90.0) as usize;
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

        let cx = width as i32 / 2;
        let cy = height as i32 / 2;
        let max_rx = (width as f32 / 2.0 - 4.0).max(12.0);
        let max_ry = (height as f32 / 2.0 - 2.0).max(5.0);
        let phase = seed as f32 * 0.031 - std::f32::consts::FRAC_PI_2 + t_anim * 0.12;

        // seeded focal lure: the lever that makes every eye gaze a different way per seed
        let lure_x = cx + (rng.random_range(-0.35..0.35) * max_rx * 0.9) as i32;
        let lure_y = cy + (rng.random_range(-0.30..0.30) * max_ry * 0.9) as i32;
        draw_arc(
            &mut grid,
            lure_x,
            lure_y,
            3.0,
            1.5,
            0.0,
            std::f32::consts::TAU,
            shift_hue(gold, 40.0),
            4,
        );
        put(&mut grid, lure_x, lure_y, '◆', lighten(gold, 20));

        // gaze helper: unit vector toward lure, scaled to eye radius
        let gaze_for = |ex: i32, ey: i32, rx: i32, ry: i32| -> (i32, i32) {
            let dx = (lure_x - ex) as f32;
            let dy = (lure_y - ey) as f32;
            let d = (dx * dx + dy * dy).sqrt().max(1.0);
            let mx = rx as f32 * 0.34;
            let my = ry as f32 * 0.6;
            let gx = ((dx / d) * mx).round() as i32;
            let gy = ((dy / d) * my).round() as i32;
            (
                gx.clamp(-(rx / 3).max(1), (rx / 3).max(1)),
                gy.clamp(-(ry / 2).max(1), (ry / 2).max(1)),
            )
        };

        // hero layered eye: almond sclera + lid rims + gaze-shifted radial-fiber iris + ◉ pupil
        let draw_hero_eye = |grid: &mut Grid,
                             ex: i32,
                             ey: i32,
                             rx: i32,
                             ry: i32,
                             gx: i32,
                             gy: i32,
                             iris_c: Color,
                             lid_c: Color| {
            for dx in -rx..=rx {
                let nx = dx as f32 / rx as f32;
                let curve = (1.0 - nx.abs().powf(1.6)).max(0.0).powf(0.55);
                let top = (-(ry as f32) * curve).round() as i32;
                let bottom = ((ry as f32) * 0.78 * curve).round() as i32;
                for dy in top..=bottom {
                    let x = ex + dx;
                    let y = ey + dy;
                    if dy == top || dy == bottom {
                        let ch = if dx < -rx / 2 {
                            if dy == top { '╱' } else { '╲' }
                        } else if dx > rx / 2 {
                            if dy == top { '╲' } else { '╱' }
                        } else {
                            '─'
                        };
                        put(grid, x, y, ch, lid_c);
                    } else {
                        let ix = (dx - gx) as f32 / (rx as f32 * 0.42);
                        let iy = (dy - gy) as f32 / ry.max(1) as f32;
                        let im = ix * ix + iy * iy;
                        if im <= 0.16 {
                            put(grid, x, y, '◉', pupil);
                        } else if im <= 1.0 {
                            let ang =
                                (iy.atan2(ix) + std::f32::consts::TAU) % std::f32::consts::TAU;
                            let fiber = ((ang / (std::f32::consts::TAU / 16.0)).round() as i32) % 2
                                == 0;
                            let ch = if im > 0.78 {
                                '○'
                            } else if fiber {
                                '╎'
                            } else {
                                '·'
                            };
                            put(grid, x, y, ch, iris_c);
                        } else {
                            put(grid, x, y, '·', sclera);
                        }
                    }
                }
            }
            put(grid, ex + gx - 1, ey + gy - 1, '˙', chalk);
            put(grid, ex + gx, ey + gy - 1, '˙', chalk);
        };

        // small gazing node eye
        let draw_node_eye = |grid: &mut Grid,
                             ncx: i32,
                             ncy: i32,
                             rx: i32,
                             ry: i32,
                             gx: i32,
                             gy: i32,
                             lid_c: Color,
                             iris_c: Color| {
            for dx in -rx..=rx {
                let nx = dx as f32 / rx as f32;
                let curve = (1.0 - nx.abs().powf(1.65)).max(0.0).powf(0.55);
                let top = (-(ry as f32) * curve).round() as i32;
                let bottom = ((ry as f32) * 0.75 * curve).round() as i32;
                for dy in top..=bottom {
                    let x = ncx + dx;
                    let y = ncy + dy;
                    if dy == top || dy == bottom {
                        let ch = if dx < -rx / 2 {
                            if dy == top { '╱' } else { '╲' }
                        } else if dx > rx / 2 {
                            if dy == top { '╲' } else { '╱' }
                        } else {
                            '─'
                        };
                        put(grid, x, y, ch, lid_c);
                    } else {
                        let ix = (dx - gx) as f32 / (rx as f32 * 0.5);
                        let iy = (dy - gy) as f32 / ry.max(1) as f32;
                        let im = ix * ix + iy * iy;
                        if im <= 0.5 {
                            put(grid, x, y, '◉', pupil);
                        } else if im <= 1.0 {
                            put(grid, x, y, '·', iris_c);
                        }
                    }
                }
            }
            put(grid, ncx + gx - 1, ncy + gy - 1, '˙', chalk);
        };

        let runes = [
            '△', '▽', '□', '◇', '☉', '☽', '☿', '♄', '♃', '✦', '∴', '∵', '⊕', '⊗', '✶', '◈',
        ];

        // arc bands: 3 concentric, alternating chalk/gold/iris, iris dashed
        let band_count = 3;
        for i in 0..band_count {
            let t = i as f32 / (band_count - 1).max(1) as f32;
            let r = 0.50 + t * 0.40;
            let col = match i % 3 {
                0 => chalk,
                1 => gold,
                _ => iris,
            };
            let gap = if i % 3 == 2 { 9 } else { 0 };
            draw_arc(
                &mut grid,
                cx,
                cy,
                max_rx * r,
                max_ry * r,
                phase,
                phase + std::f32::consts::TAU,
                col,
                gap,
            );
        }

        // multi-tier node web: each tier is a {n/k} star polygon, seeded n/k/rotation
        let mut tier_verts: Vec<Vec<(i32, i32)>> = Vec::new();
        for ti in 0..tier_count {
            let rad = 0.42 + ti as f32 * 0.16;
            let n = 5 + (seed as usize + ti * 7) % 6;
            let rot = phase + ti as f32 * 0.4 + seed as f32 * 0.01 * ti as f32;
            let mut verts: Vec<(i32, i32)> = Vec::with_capacity(n);
            for i in 0..n {
                let a = rot + i as f32 * std::f32::consts::TAU / n as f32;
                verts.push(point_on(cx, cy, max_rx * rad, max_ry * rad, a));
            }
            let max_k = ((n - 1) / 2).max(1);
            let k = if max_k >= 2 {
                2 + (seed as usize + ti) % (max_k - 1)
            } else {
                1
            };
            let col = if ti % 2 == 0 { gold } else { chalk };
            for i in 0..n {
                let (ax, ay) = verts[i];
                let (bx, by) = verts[(i + k.min(max_k)) % n];
                draw_line(&mut grid, ax, ay, bx, by, darken(col, 8));
            }
            for &v in &verts {
                put(&mut grid, v.0, v.1, '◇', lighten(col, 8));
            }
            tier_verts.push(verts);
        }

        // curved rune inscription band running around the outer ring
        let inscribe_r = 0.92;
        let ins_n = (rune_count / 4).clamp(14, 30);
        for i in 0..ins_n {
            let a = phase + i as f32 * std::f32::consts::TAU / ins_n as f32;
            let p = point_on(cx, cy, max_rx * inscribe_r, max_ry * inscribe_r, a);
            put(
                &mut grid,
                p.0,
                p.1,
                runes[(i + seed as usize) % runes.len()],
                lighten(gold, 8),
            );
        }

        // hero eye at center, gazing at lure
        let hero_rx = (width as f32 * 0.12) as i32;
        let hero_ry = (height as f32 * 0.14) as i32;
        let (hgx, hgy) = gaze_for(cx, cy, hero_rx, hero_ry);
        draw_hero_eye(
            &mut grid,
            cx,
            cy,
            hero_rx,
            hero_ry,
            hgx,
            hgy,
            lighten(iris, 16),
            lighten(lid, 10),
        );

        // node eyes at outer tier, each gazing at the lure
        if let Some(outer) = tier_verts.last() {
            for (i, &(nx, ny)) in outer.iter().enumerate() {
                let (ngx, ngy) = gaze_for(nx, ny, 5, 2);
                draw_node_eye(
                    &mut grid,
                    nx,
                    ny,
                    5,
                    2,
                    ngx,
                    ngy,
                    darken(lid, 6),
                    shift_hue(iris, i as f64 * 40.0),
                );
                put(
                    &mut grid,
                    nx,
                    ny + 3,
                    runes[i % runes.len()],
                    lighten(gold, 12),
                );
            }
        }

        // a few seeded crossing chords for energy
        for _ in 0..(tier_count + 1) {
            let a = phase + rng.random::<f32>() * std::f32::consts::TAU;
            let p1 = point_on(
                cx,
                cy,
                max_rx * rng.random_range(0.22..0.45),
                max_ry * rng.random_range(0.22..0.45),
                a,
            );
            let p2 = point_on(
                cx,
                cy,
                max_rx * rng.random_range(0.60..0.90),
                max_ry * rng.random_range(0.60..0.90),
                a + rng.random_range(0.3..1.3),
            );
            draw_line(&mut grid, p1.0, p1.1, p2.0, p2.1, darken(iris, 18));
        }
    grid
}

// --- fa6 : an animated spatial transmutation engine. Seed builds the chambers;
// T rotates the sealwork and moves current through a fixed ritual topology. ---
pub(crate) fn draw_fa6(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    rng: &mut StdRng,
    t: f32,
    chamber_count: usize,
    density: u32,
    speed: f32,
    asymmetry: f32,
) {
    use std::f32::consts::{FRAC_PI_2, TAU};

    if width < 4 || height < 4 {
        return;
    }
    let chamber_count = chamber_count.clamp(3, 18);
    let density = density.min(100);
    let speed = speed.clamp(0.05, 4.0);
    let asymmetry = asymmetry.clamp(0.0, 1.0);
    let bg = darken(palette[0], 16);
    let chalk = lighten(palette[4], 18);
    let gold = lighten(palette[1], 36);
    let ether = shift_hue(lighten(palette[3], 38), 38.0);
    let rose = shift_hue(lighten(palette[2], 42), -42.0);
    let verdigris = shift_hue(lighten(palette[1], 30), 96.0);
    let hush = darken(palette[2], 68);
    let colors = [gold, ether, rose, verdigris, chalk];

    for y in 0..height {
        for x in 0..width {
            let field = pp_hash2(x as i32, y as i32, seed ^ 0xFA60_FA60);
            let ch = if field > 0.994 {
                '°'
            } else if field > 0.982 {
                '·'
            } else {
                ' '
            };
            grid[y][x] = Cell::new(ch, if ch == ' ' { bg } else { hush });
        }
    }

    let core_rx = (width as f32 * 0.235).clamp(10.0, 29.0);
    let core_ry = (height as f32 * 0.34).clamp(4.0, 12.0);
    let cx = (width as i32 / 2
        + rng.random_range(-((width as i32 / 18).max(1))..=(width as i32 / 18).max(1)))
        .clamp(2, width as i32 - 3);
    let cy = (height as i32 / 2 + rng.random_range(-1..=1)).clamp(2, height as i32 - 3);
    let seed_phase = rng.random_range(0.0..TAU);

    #[derive(Clone)]
    struct Chamber {
        x0: usize,
        y0: usize,
        x1: usize,
        y1: usize,
        depth: usize,
    }
    let mut chambers = vec![Chamber {
        x0: 1,
        y0: 1,
        x1: width.saturating_sub(2).max(1),
        y1: height.saturating_sub(2).max(1),
        depth: 0,
    }];

    while chambers.len() < chamber_count {
        let split_idx = chambers
            .iter()
            .enumerate()
            .filter(|(_, c)| c.x1.saturating_sub(c.x0) >= 10 || c.y1.saturating_sub(c.y0) >= 6)
            .max_by_key(|(_, c)| {
                c.x1.saturating_sub(c.x0) * c.y1.saturating_sub(c.y0)
            })
            .map(|(i, _)| i);
        let Some(split_idx) = split_idx else { break };
        let cell = chambers.remove(split_idx);
        let cw = cell.x1.saturating_sub(cell.x0);
        let ch = cell.y1.saturating_sub(cell.y0);
        let can_v = cw >= 10;
        let can_h = ch >= 6;
        let vertical = if can_v && can_h {
            if cw as f32 > ch as f32 * 2.2 {
                true
            } else if ch as f32 > cw as f32 * 0.72 {
                false
            } else {
                rng.random_range(0..2) == 0
            }
        } else {
            can_v
        };
        let next_depth = cell.depth + 1;
        if vertical {
            let lo = cell.x0 + 4;
            let hi = cell.x1.saturating_sub(4);
            if lo >= hi {
                chambers.push(cell);
                break;
            }
            let cut = rng.random_range(lo..=hi);
            chambers.push(Chamber {
                x0: cell.x0,
                y0: cell.y0,
                x1: cut,
                y1: cell.y1,
                depth: next_depth,
            });
            chambers.push(Chamber {
                x0: cut,
                y0: cell.y0,
                x1: cell.x1,
                y1: cell.y1,
                depth: next_depth,
            });
        } else {
            let lo = cell.y0 + 2;
            let hi = cell.y1.saturating_sub(2);
            if lo >= hi {
                chambers.push(cell);
                break;
            }
            let cut = rng.random_range(lo..=hi);
            chambers.push(Chamber {
                x0: cell.x0,
                y0: cell.y0,
                x1: cell.x1,
                y1: cut,
                depth: next_depth,
            });
            chambers.push(Chamber {
                x0: cell.x0,
                y0: cut,
                x1: cell.x1,
                y1: cell.y1,
                depth: next_depth,
            });
        }
    }

    let inside_core = |x: f32, y: f32, margin: f32| -> bool {
        let dx = (x - cx as f32) / (core_rx * margin);
        let dy = (y - cy as f32) / (core_ry * margin);
        dx * dx + dy * dy < 1.0
    };

    // Each recursive chamber is a rational container, but its edges are
    // selectively erased. The missing segments are as important as the frame.
    for (ci, chamber) in chambers.iter().enumerate() {
        let frame_color = darken(colors[ci % colors.len()], 48);
        for x in chamber.x0..=chamber.x1.min(width - 1) {
            if (x + chamber.depth * 3 + seed as usize) % 7 != 0 {
                pp_put(grid, x as i32, chamber.y0 as i32, '┄', frame_color);
            }
            if (x + chamber.depth * 5 + seed as usize) % 6 != 0 {
                pp_put(grid, x as i32, chamber.y1 as i32, '┄', frame_color);
            }
        }
        for y in chamber.y0..=chamber.y1.min(height - 1) {
            if (y + chamber.depth * 3 + seed as usize) % 5 != 0 {
                pp_put(grid, chamber.x0 as i32, y as i32, '┊', frame_color);
            }
            if (y + chamber.depth * 7 + seed as usize) % 6 != 0 {
                pp_put(grid, chamber.x1 as i32, y as i32, '┊', frame_color);
            }
        }
        for &(x, y, ch) in &[
            (chamber.x0, chamber.y0, '╭'),
            (chamber.x1, chamber.y0, '╮'),
            (chamber.x0, chamber.y1, '╰'),
            (chamber.x1, chamber.y1, '╯'),
        ] {
            pp_put(grid, x as i32, y as i32, ch, darken(frame_color, 4));
        }

        let area = chamber.x1.saturating_sub(chamber.x0) * chamber.y1.saturating_sub(chamber.y0);
        let marks = if density == 0 {
            0
        } else {
            ((area * density as usize) / 2600).clamp(1, 14)
        };
        let runes = ['∴', '∵', '△', '▽', '☉', '☿', '⊕', '⌬', '◇', '○', '×', '·'];
        for mark in 0..marks {
            if chamber.x1 <= chamber.x0 + 1 || chamber.y1 <= chamber.y0 + 1 {
                continue;
            }
            let x = rng.random_range(chamber.x0 + 1..chamber.x1) as i32;
            let y = rng.random_range(chamber.y0 + 1..chamber.y1) as i32;
            if !inside_core(x as f32, y as f32, 1.16) {
                pp_put(
                    grid,
                    x,
                    y,
                    runes[(mark + ci * 3 + seed as usize) % runes.len()],
                    darken(colors[(ci + mark) % colors.len()], 42),
                );
            }
        }
        let tag = format!("{:02X}", (seed as usize + ci * 29 + chamber.depth * 11) & 0xff);
        for (j, glyph) in tag.chars().enumerate() {
            pp_put(
                grid,
                chamber.x0 as i32 + 2 + j as i32,
                chamber.y0 as i32,
                glyph,
                darken(chalk, 36),
            );
        }
    }

    // Seeded rupture rays ignore the chamber hierarchy. They are still
    // deterministic and broken by a spatial hash, so they feel torn, not noisy.
    let fracture_count = 2 + (asymmetry * 7.0).round() as usize;
    for fi in 0..fracture_count {
        let angle = seed_phase + rng.random_range(0.0..TAU) + fi as f32 * 0.37;
        let start = pp_point_on(cx, cy, core_rx * 0.92, core_ry * 0.92, angle);
        let ray_len = width.max(height * 2) as f32;
        let end = (
            cx + (angle.cos() * ray_len).round() as i32,
            cy + (angle.sin() * ray_len * 0.52).round() as i32,
        );
        let steps = (end.0 - start.0).abs().max((end.1 - start.1).abs()).max(1);
        for s in 0..=steps {
            let q = s as f32 / steps as f32;
            let x = (start.0 as f32 + (end.0 - start.0) as f32 * q).round() as i32;
            let y = (start.1 as f32 + (end.1 - start.1) as f32 * q).round() as i32;
            if pp_hash2(x, y, seed ^ fi as u64 * 0x9E37) > 0.26 {
                pp_put(
                    grid,
                    x,
                    y,
                    pp_stroke(end.0 - start.0, end.1 - start.1),
                    darken(rose, 34),
                );
            }
        }
        let pulse = (t * speed * 0.22 + fi as f32 / fracture_count as f32).rem_euclid(1.0);
        pp_put(
            grid,
            (start.0 as f32 + (end.0 - start.0) as f32 * pulse).round() as i32,
            (start.1 as f32 + (end.1 - start.1) as f32 * pulse).round() as i32,
            '✦',
            lighten(rose, 12),
        );
    }

    struct Node {
        x: f32,
        y: f32,
        phase: f32,
        radius: f32,
        kind: usize,
        color: Color,
    }
    let mut nodes = Vec::with_capacity(chambers.len());
    for (ci, chamber) in chambers.iter().enumerate() {
        let mut x = (chamber.x0 + chamber.x1) as f32 * 0.5;
        let mut y = (chamber.y0 + chamber.y1) as f32 * 0.5;
        x += rng.random_range(-1.0..1.0) * asymmetry * 3.5;
        y += rng.random_range(-1.0..1.0) * asymmetry * 1.5;
        let mut dx = x - cx as f32;
        let mut dy = y - cy as f32;
        let metric = (dx / (core_rx * 1.28)).powi(2) + (dy / (core_ry * 1.28)).powi(2);
        if metric < 1.0 {
            if metric < 0.01 {
                let a = seed_phase + ci as f32 * TAU / chambers.len().max(1) as f32;
                dx = a.cos() * core_rx;
                dy = a.sin() * core_ry;
            }
            let scale = 1.08 / metric.max(0.02).sqrt();
            x = cx as f32 + dx * scale;
            y = cy as f32 + dy * scale;
        }
        nodes.push(Node {
            x: x.clamp(2.0, width as f32 - 3.0),
            y: y.clamp(2.0, height as f32 - 3.0),
            phase: rng.random_range(0.0..TAU),
            radius: rng.random_range(2.2..4.5),
            kind: (ci * 7 + seed as usize) % 9,
            color: colors[ci % colors.len()],
        });
    }

    let node_pos = |node: &Node| -> (i32, i32) {
        (
            (node.x + (t * speed * 0.43 + node.phase).sin() * asymmetry * 2.2).round() as i32,
            (node.y + (t * speed * 0.31 + node.phase * 1.7).cos() * asymmetry * 0.9).round()
                as i32,
        )
    };
    let positions: Vec<(i32, i32)> = nodes.iter().map(node_pos).collect();

    for (i, node) in nodes.iter().enumerate() {
        let here = positions[i];
        let target = if i > 0 && i % 3 != 0 {
            positions[(i + nodes.len() - 1) % nodes.len()]
        } else {
            (cx, cy)
        };
        pp_line(grid, here.0, here.1, target.0, target.1, darken(node.color, 56));
        let pulse = (t * speed * (0.16 + i as f32 * 0.007) + node.phase / TAU).rem_euclid(1.0);
        pp_put(
            grid,
            (here.0 as f32 + (target.0 - here.0) as f32 * pulse).round() as i32,
            (here.1 as f32 + (target.1 - here.1) as f32 * pulse).round() as i32,
            if i % 2 == 0 { '◆' } else { '◇' },
            lighten(node.color, 14),
        );
    }

    // Carve the central void after drawing the network so every conduit appears
    // to terminate cleanly at the transmutation boundary.
    for y in 0..height {
        for x in 0..width {
            if inside_core(x as f32, y as f32, 1.08) {
                grid[y][x] = Cell::new(' ', bg);
            }
        }
    }

    // Satellite seals sit at chamber centroids. Their orbiting marks are driven
    // by T, but the chamber assignment and iconography remain seed-stable.
    let sigils = ['☉', '☿', '♄', '♃', '△', '▽', '⊕', '⌬', '◉'];
    for (i, node) in nodes.iter().enumerate() {
        let (nx, ny) = positions[i];
        let r = node.radius;
        let phase = node.phase - t * speed * (0.11 + i as f32 * 0.004);
        pp_arc(
            grid,
            nx,
            ny,
            r,
            (r * 0.48).max(1.2),
            phase,
            phase + TAU,
            darken(node.color, 16),
            9,
        );
        let arm = pp_point_on(nx, ny, r + 1.0, (r * 0.48 + 0.8).max(1.5), phase);
        pp_line(grid, nx, ny, arm.0, arm.1, darken(node.color, 8));
        pp_put(grid, nx, ny, sigils[node.kind], lighten(node.color, 18));
        pp_put(grid, arm.0, arm.1, '○', lighten(chalk, 2));
    }

    let phase = seed_phase + t * speed * 0.18;
    pp_arc(
        grid,
        cx,
        cy,
        core_rx,
        core_ry,
        phase,
        phase + TAU,
        lighten(chalk, 2),
        0,
    );
    pp_arc(
        grid,
        cx,
        cy,
        core_rx * 0.89,
        core_ry * 0.87,
        -phase,
        -phase + TAU,
        darken(ether, 4),
        13,
    );

    let polygon = |n: usize, rx: f32, ry: f32, a0: f32| -> Vec<(i32, i32)> {
        (0..n)
            .map(|i| pp_point_on(cx, cy, rx, ry, a0 + i as f32 * TAU / n as f32))
            .collect()
    };
    let outer = polygon(7, core_rx * 0.78, core_ry * 0.76, phase * 1.31 - FRAC_PI_2);
    for i in 0..outer.len() {
        pp_line(
            grid,
            outer[i].0,
            outer[i].1,
            outer[(i + 3) % outer.len()].0,
            outer[(i + 3) % outer.len()].1,
            darken(gold, 4),
        );
    }
    let inner = polygon(5, core_rx * 0.49, core_ry * 0.46, -phase * 1.73 + seed_phase);
    for i in 0..inner.len() {
        pp_line(
            grid,
            inner[i].0,
            inner[i].1,
            inner[(i + 2) % inner.len()].0,
            inner[(i + 2) % inner.len()].1,
            lighten(rose, 4),
        );
    }

    let tick_count = 12 + chamber_count.min(12);
    let ring_runes = ['△', '▽', '□', '◇', '○', '⊕', '∴', '∵'];
    for i in 0..tick_count {
        let a = phase * if i % 2 == 0 { 1.0 } else { -0.7 }
            + i as f32 * TAU / tick_count as f32;
        let p0 = pp_point_on(cx, cy, core_rx * 0.88, core_ry * 0.86, a);
        let p1 = pp_point_on(cx, cy, core_rx * 1.02, core_ry * 1.02, a);
        pp_line(grid, p0.0, p0.1, p1.0, p1.1, darken(gold, 10));
        if i % 3 == 0 {
            pp_put(
                grid,
                p0.0,
                p0.1,
                ring_runes[(i + seed as usize) % ring_runes.len()],
                lighten(colors[i % colors.len()], 10),
            );
        }
    }

    // The impossible balance at the center is intentionally compact: opposing
    // triangles, a material axis, and an eye-like witness glyph.
    let top = (cx, cy - (core_ry * 0.27).round() as i32);
    let left = (cx - (core_rx * 0.17).round() as i32, cy + (core_ry * 0.20).round() as i32);
    let right = (cx + (core_rx * 0.17).round() as i32, left.1);
    pp_line(grid, top.0, top.1, left.0, left.1, lighten(gold, 12));
    pp_line(grid, left.0, left.1, right.0, right.1, lighten(gold, 12));
    pp_line(grid, right.0, right.1, top.0, top.1, lighten(gold, 12));
    pp_line(grid, cx, top.1 - 1, cx, left.1 + 1, lighten(ether, 10));
    pp_put(grid, cx, cy, '◉', lighten(chalk, 24));
    pp_put(grid, cx - 1, cy, '╴', rose);
    pp_put(grid, cx + 1, cy, '╶', rose);
    pp_put(grid, cx, top.1 - 1, '☉', lighten(gold, 16));
    pp_put(grid, cx, left.1 + 1, '▽', lighten(ether, 12));
}


pub(crate) fn draw_delta(grid: &mut Grid, width: usize, height: usize, _seed: u64, palette: &[Color; 5], rng: &mut StdRng, t: f32) {
    use std::f32::consts::FRAC_PI_2;
    let bg = darken(palette[0], 6);
    for y in 0..height {
        for x in 0..width {
            grid[y][x] = Cell::new(' ', bg);
        }
    }
    // Physics tree. Each branch is a torsional spring-damper at its joint: it has
    // a rest angle relative to its parent, an angular deflection `theta`, and an
    // angular velocity `omega`. A turbulent wind force field pushes on each
    // segment; the spring pulls it back to rest; damping bleeds energy. Children
    // hang off the parent's *current* tip and inherit its swayed world angle, so
    // it's a kinematic chain -- trunk motion propagates to twigs, and lighter
    // (shorter) tips have less inertia so they flutter faster than the trunk.
    //
    // Frames are stateless (each render gets a single time `t`), so we re-integrate
    // from a bounded warm-up window each frame: start at rest at t0 = t - WARM and
    // step to t. The artificial rest start's transient decays within WARM, leaving
    // the true forced steady-state at t -- constant cost no matter how large t grows.
    struct Node {
        parent: i32,
        rel: f32,  // rest angle relative to parent (absolute for roots)
        len: f32,
        depth: i32,
        bx: f32,   // root base position (children derive base from parent tip)
        by: f32,
    }
    // Build topology with the exact same rng call order as the static tree, so a
    // t==0 render is byte-identical to the snapshot / plain mode.
    let mut nodes: Vec<Node> = Vec::new();
    struct Pending {
        parent: i32,
        rel: f32,
        len: f32,
        depth: i32,
        bx: f32,
        by: f32,
    }
    let roots = 3;
    let mut stack: Vec<Pending> = Vec::new();
    for r in 0..roots {
        let x = width as f32 * (r as f32 + 1.0) / (roots as f32 + 1.0);
        stack.push(Pending {
            parent: -1,
            rel: FRAC_PI_2 + rng.random_range(-0.25f32..0.25),
            len: height as f32 * 0.30,
            depth: 0,
            bx: x,
            by: 1.0,
        });
    }
    while let Some(p) = stack.pop() {
        if p.depth > 7 || p.len < 2.0 {
            continue;
        }
        let idx = nodes.len() as i32;
        nodes.push(Node {
            parent: p.parent,
            rel: p.rel,
            len: p.len,
            depth: p.depth,
            bx: p.bx,
            by: p.by,
        });
        let children = if p.depth < 2 { 3 } else { 2 };
        for _ in 0..children {
            let da = rng.random_range(-0.65f32..0.65);
            let len = p.len * rng.random_range(0.6f32..0.78);
            stack.push(Pending {
                parent: idx,
                rel: da,
                len,
                depth: p.depth + 1,
                bx: 0.0,
                by: 0.0,
            });
        }
    }
    let n = nodes.len();

    // Per-joint physics constants, tunable from the demo options pane (env knobs;
    // see the "delta" form in MODE_FORMS). Stiffness scales with branch thickness (~len), inertia
    // with mass*len^2 (~len^3 for a uniform rod). Natural frequency then goes as
    // 1/len: the trunk sways slowly, twigs flutter fast.
    let kk = param_f32("K", 4.0); // stiffness coefficient
    let dd = param_f32("D", 0.0055); // inertia density
    let zeta = param_f32("ZETA", 0.18); // damping ratio (underdamped -> lively)
    let wind_amt = param_f32("WIND", 1.0); // gust strength multiplier
    let turb_amt = param_f32("TURB", 1.0); // turbulence multiplier
    let rbow = param_f32("RBOW", 0.0); // 0 = palette colors, 1 = full rainbow gradient
    let stiff = |len: f32| kk * len;
    let inertia = |len: f32| (dd * len * len * len).max(1e-4);
    let damp = |len: f32| 2.0 * zeta * (stiff(len) * inertia(len)).sqrt();

    // Turbulent wind: a rightward gust that swells over time plus spatial chop.
    let wind = |x: f32, y: f32, time: f32| -> (f32, f32) {
        let gust = (0.6 + 0.5 * (time * 0.45).sin() + 0.25 * (time * 1.3 + 1.7).sin()) * wind_amt;
        let turb =
            turb_amt * (0.4 * (x * 0.15 + time * 1.1).sin() + 0.3 * (y * 0.2 - time * 0.9).sin());
        let fx = gust + turb;
        let fy = 0.15 * (x * 0.1 + time * 2.0).sin();
        (fx, fy)
    };

    let mut theta = vec![0.0f32; n];
    let mut omega = vec![0.0f32; n];
    // scratch for the per-step kinematic forward pass
    let mut wang = vec![0.0f32; n]; // world angle
    let mut tipx = vec![0.0f32; n];
    let mut tipy = vec![0.0f32; n];

    let windy = t != 0.0; // t==0 -> rest tree, byte-identical to the static render
    if windy {
        const WARM: f32 = 6.0;
        const DT: f32 = 0.08;
        let t0 = (t - WARM).max(0.0);
        let steps = (((t - t0) / DT).round() as i32).max(1);
        let mut time = t0;
        for _ in 0..steps {
            // forward pass: world angle + tip position from current deflections.
            for i in 0..n {
                let nd = &nodes[i];
                let (bx, by, pang) = if nd.parent < 0 {
                    (nd.bx, nd.by, 0.0)
                } else {
                    let pi = nd.parent as usize;
                    (tipx[pi], tipy[pi], wang[pi])
                };
                let a = pang + nd.rel + theta[i];
                wang[i] = a;
                tipx[i] = bx + a.cos() * nd.len * 1.8;
                tipy[i] = by + a.sin() * nd.len;
            }
            // integrate each joint (semi-implicit Euler).
            for i in 0..n {
                let nd = &nodes[i];
                let (bx, by) = if nd.parent < 0 {
                    (nd.bx, nd.by)
                } else {
                    let pi = nd.parent as usize;
                    (tipx[pi], tipy[pi])
                };
                let mx = (bx + tipx[i]) * 0.5; // segment midpoint (force sample point)
                let my = (by + tipy[i]) * 0.5;
                let (fx, fy) = wind(mx, my, time);
                let a = wang[i];
                // force component perpendicular to the branch -> bending torque.
                let perp = fx * (-a.sin()) + fy * a.cos();
                let exposure = 1.0 + 0.3 * nd.depth as f32; // tips catch more wind
                let torque = perp * nd.len * exposure
                    - stiff(nd.len) * theta[i]
                    - damp(nd.len) * omega[i];
                let alpha = torque / inertia(nd.len);
                omega[i] += alpha * DT;
                theta[i] += omega[i] * DT;
                theta[i] = theta[i].clamp(-0.7, 0.7); // keep branches from folding over
            }
            time += DT;
        }
    }

    // final forward pass + draw.
    for i in 0..n {
        let nd = &nodes[i];
        let (bx, by, pang) = if nd.parent < 0 {
            (nd.bx, nd.by, 0.0)
        } else {
            let pi = nd.parent as usize;
            (tipx[pi], tipy[pi], wang[pi])
        };
        let a = pang + nd.rel + theta[i];
        wang[i] = a;
        let ex = bx + a.cos() * nd.len * 1.8;
        let ey = by + a.sin() * nd.len;
        tipx[i] = ex;
        tipy[i] = ey;
        let mut col = lerp_color(palette[1], palette[3], nd.depth as f32 / 7.0);
        if rbow > 0.0 {
            // hue sweeps with depth (trunk -> tips) plus horizontal position, so the
            // canopy reads as a rainbow gradient. `rbow` blends it over the palette.
            let hue = ((nd.depth as f32 / 7.0) * 280.0 + (ex / width as f32) * 80.0).rem_euclid(360.0);
            let rainbow = hsl_to_rgb(hue as f64, 0.75, 0.55);
            col = lerp_color(col, rainbow, rbow);
        }
        pp_line(grid, bx.round() as i32, by.round() as i32, ex.round() as i32, ey.round() as i32, col);
        pp_put(grid, ex.round() as i32, ey.round() as i32, '◆', lighten(col, 10));
    }
}

