#![allow(unused)]
use crate::color::*;
use crate::fills::*;
use crate::sprites::*;
use crate::types::*;
use crossterm::style::Color;
use rand::RngExt;
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::f32::consts::TAU;

// ── Shared helpers ──────────────────────────────────────────────────

fn put(grid: &mut Grid, x: i32, y: i32, ch: char, fg: Color) {
    if x >= 0 && y >= 0 && (y as usize) < grid.len() && (x as usize) < grid[0].len() {
        grid[y as usize][x as usize] = Cell::new(ch, fg);
    }
}

/// Deterministic hash of (seed, position, time). Time makes the same cell
/// re-roll when animated, while seed keeps the whole scene reproducible.
fn hash_seed(seed: u64, x: usize, y: usize, t: f32) -> u64 {
    let mut s = seed;
    s = s.wrapping_mul(0x9E37_79B9_7F4A_7C15).wrapping_add(x as u64);
    s = s.wrapping_mul(0x9E37_79B9_7F4A_7C15).wrapping_add(y as u64);
    s = s.wrapping_mul(0x9E37_79B9_7F4A_7C15).wrapping_add(t.to_bits() as u64);
    s
}

/// Seeded RNG for a single element so every feature can vary independently
/// while staying locked to the global seed and current animation time.
fn element_rng(seed: u64, x: usize, y: usize, t: f32) -> StdRng {
    StdRng::seed_from_u64(hash_seed(seed, x, y, t))
}

// ── Avant-garde face sprite ─────────────────────────────────────────

fn draw_avant_face(
    grid: &mut Grid,
    cx: i32,
    cy: i32,
    size: i32,
    style: usize,
    palette: &[Color; 5],
    phase: f32,
    _rng: &mut StdRng,
) {
    let style = style % 8;
    let primary = palette[1];
    let accent = palette[3];
    let dim = darken(primary, 35);
    let bright = lighten(accent, 25);

    let w = (size / 2).max(1);
    let eye_y = cy - 1;
    let mouth_y = cy + (size as f32 * 0.6).round() as i32;

    // eyes: style + phase morph the glyph
    let eye_pool = ['●', '◉', '◆', '⬤', '○', '◐', '◑', '◒'];
    let li = ((style + (phase * 3.0) as usize) % eye_pool.len());
    let ri = ((style + 1 + (phase * 5.0) as usize) % eye_pool.len());
    put(grid, cx - w, eye_y, eye_pool[li], bright);
    put(grid, cx + w, eye_y, eye_pool[ri], bright);

    // nose / median feature
    let nose = match (style + (phase * 2.0) as usize) % 5 {
        0 => '│',
        1 => '▲',
        2 => '◆',
        3 => '╱',
        _ => '╲',
    };
    put(grid, cx, cy, nose, primary);

    // mouth
    let mouth_pool = ['◡', '◠', '─', '▪', '╭'];
    let mi = ((style * 3 + (phase * 4.0) as usize) % mouth_pool.len());
    put(grid, cx, mouth_y, mouth_pool[mi], accent);

    // brows: phase flips asymmetry
    if (style + (phase * 2.0) as usize) % 2 == 0 {
        put(grid, cx - w - 1, eye_y - 1, '╱', dim);
        put(grid, cx + w + 1, eye_y - 1, '╲', dim);
    } else {
        put(grid, cx - w - 1, eye_y - 1, '╲', dim);
        put(grid, cx + w + 1, eye_y - 1, '╱', dim);
    }

    // horizontal rays / whiskers
    let ray_len = size.max(1);
    for i in 1..=ray_len {
        put(grid, cx - w - 1 - i, eye_y, '─', dim);
        put(grid, cx + w + 1 + i, eye_y, '─', dim);
    }

    // vertical stem through the face
    for i in 1..=size {
        put(grid, cx, cy - 1 - i, '│', dim);
        if mouth_y + i < grid.len() as i32 {
            put(grid, cx, mouth_y + i, '│', dim);
        }
    }
}

// ── Rhizome: underground tree network ───────────────────────────────

fn rhizome_branch(
    grid: &mut Grid,
    pen: &TreePen,
    initial_dir: MoveDir,
    depth: u32,
    color: Color,
    seed: u64,
    t: f32,
) {
    if depth == 0 {
        return;
    }
    let w = grid[0].len() as i32;
    let h = grid.len() as i32;

    let mut bpen = pen.fork(lighten(color, (10 * (4 - depth)).min(60) as u8));
    bpen.last_dir = Some(initial_dir);

    let len = element_rng(seed, bpen.x as usize + depth as usize * 17, bpen.y as usize, t)
        .random_range(3..10u32) as i32;
    let mut dir = initial_dir;

    for i in 0..len {
        if bpen.y < 0 || bpen.y >= h || bpen.x < 0 || bpen.x >= w {
            break;
        }
        bpen.step(grid, dir);
        if bpen.y < 0 || bpen.y >= h || bpen.x < 0 || bpen.x >= w {
            break;
        }

        let mut er = element_rng(seed, bpen.x as usize, bpen.y as usize + depth as usize * 7, t);
        if er.random::<f32>() < 0.3 {
            dir = match dir {
                MoveDir::UpRight => MoveDir::UpLeft,
                MoveDir::UpLeft => MoveDir::UpRight,
                _ => MoveDir::Up,
            };
        }
        if i > 0 && i % 3 == 0 && depth > 1 && er.random::<f32>() < 0.25 {
            let fork_dir = if er.random::<bool>() {
                MoveDir::UpRight
            } else {
                MoveDir::UpLeft
            };
            rhizome_branch(grid, &bpen, fork_dir, depth - 1, color, seed, t);
        }
    }
    if bpen.y >= 0 && bpen.y < h && bpen.x >= 0 && bpen.x < w {
        put(grid, bpen.x, bpen.y, '·', lighten(color, 40));
    }
}

pub fn draw_rhizome(
    grid: &mut Grid,
    w: usize,
    h: usize,
    seed: u64,
    palette: &[Color; 5],
    rng: &mut StdRng,
    t: f32,
    count: usize,
    depth: u32,
) {
    fill_truchet(grid, w, h, darken(palette[0], 18), rng);

    let count = if count == 0 { 5 } else { count };
    let depth = if depth == 0 { 3 } else { depth };
    let ground_y = (h as f32 * 0.78) as i32;
    let top_y = (h as f32 * 0.12) as i32;
    let margin = (w as f32 * 0.08) as usize;

    for i in 0..count {
        let param = i as f32 / count.max(1) as f32;
        let base_x = margin + (param * (w - 2 * margin) as f32) as usize;
        let jitter = ((hash_seed(seed, i, 0, t) as f32 / u64::MAX as f32) - 0.5)
            * (w as f32 / (count as f32 + 1.0));
        let root_x = (base_x as f32 + jitter).clamp(margin as f32, (w - margin) as f32) as i32;
        let color = palette[1 + (i % 3)];

        let mut pen = TreePen::new(root_x, ground_y, color);
        let sway = (t * TAU * 0.1 + i as f32 * 1.3).sin() * 0.25;
        let inner_left = margin as i32 + 2;
        let inner_right = w as i32 - margin as i32 - 2;

        let mut steps = 0;
        let max_steps = h;
        while pen.y > top_y
            && pen.y < h as i32
            && pen.x >= 0
            && pen.x < w as i32
            && steps < max_steps
        {
            steps += 1;
            let h_frac = (pen.y - top_y) as f32 / (ground_y - top_y).max(1) as f32;
            let mut er = element_rng(seed, pen.x as usize + i * 73, pen.y as usize, t);
            let r: f32 = er.random();
            let mut dir = if r < 0.65 {
                MoveDir::Up
            } else if r < 0.65 + (0.35 * (0.5 + sway * 0.5)) {
                MoveDir::UpRight
            } else {
                MoveDir::UpLeft
            };
            // keep the rhizome roughly in its lane
            if pen.x <= inner_left && dir == MoveDir::UpLeft {
                dir = MoveDir::Up;
            }
            if pen.x >= inner_right && dir == MoveDir::UpRight {
                dir = MoveDir::Up;
            }
            pen.step(grid, dir);

            if h_frac > 0.25 && er.random::<f32>() < 0.12 * h_frac {
                let branch_dir = if er.random::<bool>() {
                    MoveDir::UpRight
                } else {
                    MoveDir::UpLeft
                };
                rhizome_branch(grid, &pen, branch_dir, depth, color, seed, t);
            }
        }

        // root tendrils curling down from the base
        for (k, &start_dir) in [MoveDir::DownLeft, MoveDir::DownRight].iter().enumerate() {
            let mut rpen = pen.fork(darken(color, 25));
            rpen.last_dir = Some(MoveDir::Down);
            let rlen = element_rng(seed, i * 11 + k, 0, t).random_range(3..8u32) as i32;
            let mut dir = start_dir;
            for _ in 0..rlen {
                if rpen.y >= h as i32 - 1 || rpen.x < 0 || rpen.x >= w as i32 {
                    break;
                }
                rpen.step(grid, dir);
                if element_rng(seed, rpen.x as usize + k * 100, rpen.y as usize, t).random::<f32>()
                    < 0.25
                {
                    dir = match dir {
                        MoveDir::DownLeft => MoveDir::DownRight,
                        MoveDir::DownRight => MoveDir::DownLeft,
                        _ => MoveDir::Down,
                    };
                }
            }
        }
    }
}

// ── Effigy: scattered algorithmic faces ─────────────────────────────

pub fn draw_effigy(
    grid: &mut Grid,
    w: usize,
    h: usize,
    seed: u64,
    palette: &[Color; 5],
    rng: &mut StdRng,
    t: f32,
    count: usize,
) {
    let rect = Rect {
        x: 0,
        y: 0,
        w,
        h,
    };
    fill_noise(grid, &rect, NoiseVariant::Dot, darken(palette[0], 20), palette[0], rng);

    let count = if count == 0 { 6 } else { count };
    let margin = (w as f32 * 0.08) as i32;
    let mut placed: Vec<(i32, i32, i32)> = Vec::new();

    for i in 0..count {
        let mut attempts = 0;
        loop {
            attempts += 1;
            if attempts > 120 {
                break;
            }
            let mut er = element_rng(seed, i * 7 + attempts, 0, t);
            let cx = er.random_range(margin as u32..(w as i32 - margin) as u32) as i32;
            let cy = er.random_range((h as i32 / 5) as u32..(h as i32 * 4 / 5) as u32) as i32;
            let size = er.random_range(2..5u32) as i32;

            let too_close = placed.iter().any(|&(px, py, ps)| {
                ((px - cx).abs() + (py - cy).abs()) < (ps + size + 3)
            });
            if !too_close {
                placed.push((cx, cy, size));
                let style = (i + (t * 4.0) as usize) % 8;
                draw_avant_face(grid, cx, cy, size, style, palette, t + i as f32, rng);
                break;
            }
        }
    }
}

// ── Dendrite: neuron-like branching ─────────────────────────────────

fn dendrite_branch(
    grid: &mut Grid,
    pen: &mut TreePen,
    depth: u32,
    color: Color,
    seed: u64,
    t: f32,
) {
    if depth == 0 {
        return;
    }
    let w = grid[0].len() as i32;
    let hh = grid.len() as i32;
    let len = element_rng(seed, pen.x as usize + depth as usize * 13, pen.y as usize, t)
        .random_range(4..10u32) as i32;
    let drift = (t * TAU * 0.15 + depth as f32).sin() * 0.3;

    let mut er = element_rng(seed, pen.x as usize, pen.y as usize + depth as usize * 5, t);
    let mut dir = MoveDir::Up;

    for step in 0..len {
        if pen.y < 2 || pen.y >= hh - 1 || pen.x < 1 || pen.x >= w - 1 {
            break;
        }
        let r = er.random::<f32>();
        dir = if r < 0.5 + drift {
            MoveDir::Up
        } else if r < 0.75 + drift * 0.25 {
            MoveDir::UpLeft
        } else {
            MoveDir::UpRight
        };
        pen.step(grid, dir);

        if step > 0 && step % 3 == 0 && depth > 1 {
            let mut left = pen.fork(lighten(color, 15));
            dendrite_branch(grid, &mut left, depth - 1, color, seed, t);
            let mut right = pen.fork(lighten(color, 15));
            dendrite_branch(grid, &mut right, depth - 1, color, seed, t);
        }
    }
    if pen.y >= 0 && pen.y < hh && pen.x >= 0 && pen.x < w {
        put(grid, pen.x, pen.y, '●', lighten(color, 40));
    }
}

pub fn draw_dendrite(
    grid: &mut Grid,
    w: usize,
    h: usize,
    seed: u64,
    palette: &[Color; 5],
    rng: &mut StdRng,
    t: f32,
    seeds: usize,
    depth: u32,
) {
    let rect = Rect {
        x: 0,
        y: 0,
        w,
        h,
    };
    fill_noise(grid, &rect, NoiseVariant::Dot, palette[0], darken(palette[0], 10), rng);

    let seeds = if seeds == 0 { 3 } else { seeds };
    let depth = if depth == 0 { 4 } else { depth };
    let ground_y = (h as f32 * 0.78) as i32;
    let margin = (w as f32 * 0.1) as usize;

    for i in 0..seeds {
        let base_x = if seeds == 1 {
            w / 2
        } else {
            margin + i * (w - 2 * margin) / (seeds - 1)
        };
        let jitter = (element_rng(seed, i, 0, t).random::<f32>() - 0.5)
            * (w as f32 / seeds.max(1) as f32);
        let root_x = (base_x as f32 + jitter).clamp(margin as f32, (w - margin) as f32) as i32;
        let color = palette[1 + (i % 3)];
        let mut pen = TreePen::new(root_x, ground_y, color);
        dendrite_branch(grid, &mut pen, depth, color, seed, t);
    }
}

// ── Totem: stacked face poles ───────────────────────────────────────

pub fn draw_totem(
    grid: &mut Grid,
    w: usize,
    h: usize,
    seed: u64,
    palette: &[Color; 5],
    rng: &mut StdRng,
    t: f32,
    poles: usize,
) {
    let rect = Rect {
        x: 0,
        y: 0,
        w,
        h,
    };
    fill_noise(grid, &rect, NoiseVariant::Static, palette[0], darken(palette[0], 10), rng);

    let poles = if poles == 0 { 2 } else { poles };
    let ground_y = (h as f32 * 0.78) as i32;
    let top_y = (h as f32 * 0.15) as i32;
    let margin = (w as f32 * 0.08) as usize;

    for p in 0..poles {
        let fx = (p + 1) as f32 / (poles + 1) as f32;
        let cx = (margin as f32 + fx * (w - 2 * margin) as f32) as i32;

        // central spine
        for y in top_y..=ground_y {
            put(grid, cx, y, '│', darken(palette[2], 30));
        }

        let seg_count = element_rng(seed, p * 3, 0, t).random_range(4..8u32) as i32;
        let seg_h = ((ground_y - top_y) as f32 / seg_count as f32) as i32;

        for s in 0..seg_count {
            let cy = ground_y - s * seg_h - seg_h / 2;
            let size = (seg_h / 2 - 1).max(1);
            let style = (p * 7 + s as usize + (t * 3.0) as usize) % 8;
            let phase = t + s as f32 * 0.5;
            draw_avant_face(grid, cx, cy, size, style, palette, phase, rng);
        }
    }
}

// ── Chimera: special hybrid of trees and faces ──────────────────────

fn connect_line(grid: &mut Grid, x0: i32, y0: i32, x1: i32, y1: i32, color: Color) {
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    let mut x = x0;
    let mut y = y0;

    loop {
        if x >= 0 && y >= 0 && (y as usize) < grid.len() && (x as usize) < grid[0].len() {
            let ch = if dx.abs() > dy.abs() * 2 {
                '─'
            } else if dy.abs() > dx.abs() * 2 {
                '│'
            } else if sx == sy {
                '╲'
            } else {
                '╱'
            };
            grid[y as usize][x as usize] = Cell::new(ch, color);
        }
        if x == x1 && y == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
}

pub fn draw_chimera(
    grid: &mut Grid,
    w: usize,
    h: usize,
    seed: u64,
    palette: &[Color; 5],
    rng: &mut StdRng,
    t: f32,
    density: u32,
    drift: f32,
) {
    fill_truchet(grid, w, h, darken(palette[0], 15), rng);

    let density = density.clamp(10, 100);
    let tree_count = 1 + density / 25;
    let face_count = 1 + density / 20;
    let margin = (w as f32 * 0.08) as usize;
    let drift_amp = drift.max(0.0) * (w as f32 / 80.0).max(0.5);

    // Trees rooted near the bottom, drifting horizontally with time
    let mut tree_tops: Vec<(usize, usize)> = Vec::new();
    for i in 0..tree_count as usize {
        let base_x = if tree_count == 1 {
            w / 2
        } else {
            margin + i * (w - 2 * margin) / (tree_count as usize - 1)
        };
        let sway = ((t * TAU + i as f32 * 0.9).sin() * drift_amp) as i32;
        let rx = (base_x as i32 + sway).clamp(margin as i32, (w - margin) as i32) as usize;
        let root_y = h * 3 / 4;
        let canopy_y = h / 4 + i * 2;
        let spread = (w / 8).max(5);
        let kind = (seed as usize + i) % 19;
        let color = palette[1 + (i % 3)];
        draw_tree(grid, rx, root_y, canopy_y, spread, kind, color, rng);
        tree_tops.push((rx, canopy_y));
    }

    // Faces floating in the upper canopy
    let mut face_centers: Vec<(i32, i32)> = Vec::new();
    for j in 0..face_count as usize {
        let mut er = element_rng(seed, j * 3, 0, t);
        let fx = er.random_range(margin as u32..(w - margin) as u32) as i32;
        let fy = er.random_range((h / 5) as u32..(h * 3 / 5) as u32) as i32;
        let size = er.random_range(2..5u32) as i32;
        let style = (j + (t * 4.0) as usize) % 8;
        draw_avant_face(grid, fx, fy, size, style, palette, t + j as f32, rng);
        face_centers.push((fx, fy));
    }

    // Tendrils lashing selected tree tops to nearby faces
    let vine_color = darken(palette[2], 20);
    for &(tx, ty) in &tree_tops {
        if let Some(&(fx, fy)) = face_centers.iter().min_by_key(|&&(fx, fy)| {
            ((fx - tx as i32).abs() + (fy - ty as i32).abs()) as i32
        }) {
            if (fx - tx as i32).abs() + (fy - ty as i32).abs() < (w as i32 / 3) {
                connect_line(grid, tx as i32, ty as i32, fx, fy, vine_color);
            }
        }
    }
}
