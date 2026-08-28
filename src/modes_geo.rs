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
use crate::modes_sky::*;
use crate::modes_tree::*;
use crate::morph::*;
use crate::opts::*;
use crate::pp::*;
use crate::registry::*;
use crate::warps::*;

// --- eyes++ : an argus field. Hero eye + two orbital rings of gazing eyes,
//     dense rays, halo arcs, every eye tracking a seeded lure. ---
pub(crate) fn draw_eyes_pp(grid: &mut Grid, width: usize, height: usize, seed: u64, palette: &[Color; 5], rng: &mut StdRng) {
    use std::f32::consts::TAU;
    let bg = darken(palette[0], 14);
    let chalk = lighten(palette[4], 14);
    let gold = lighten(palette[1], 30);
    let iris_outer = shift_hue(lighten(palette[3], 26), 18.0);
    let iris_inner = lighten(palette[2], 20);
    let lid_color = lighten(palette[1], 18);
    let pupil = darken(palette[0], 2);
    let sclera = lighten(palette[4], 6);
    let ray_color = lighten(palette[4], 18);
    let hush = darken(palette[2], 60);

    for y in 0..height {
        for x in 0..width {
            let n = (x * 17 + y * 29 + seed as usize * 3) % 97;
            let (ch, col) = match n {
                0 => ('·', hush),
                1 if (x + y) % 3 == 0 => ('∙', hush),
                _ => (' ', bg),
            };
            grid[y][x] = Cell::new(ch, col);
        }
    }

    let cx = width as i32 / 2;
    let cy = height as i32 / 2;
    let max_rx = (width as f32 / 2.0 - 1.0).max(10.0);
    let max_ry = (height as f32 / 2.0 - 1.0).max(5.0);
    let phase = rng.random_range(0.0f32..TAU);

    let lure_x = (cx + rng.random_range(-(width as i32 / 6)..=(width as i32 / 6))).clamp(8, width as i32 - 8);
    let lure_y = (cy + rng.random_range(-(height as i32 / 5)..=(height as i32 / 5))).clamp(6, height as i32 - 4);
    let lure_col = shift_hue(lighten(iris_inner, 26), rng.random_range(-40..=40) as f64);

    let gaze_for = |ex: i32, ey: i32, rx: i32, ry: i32| -> (i32, i32) {
        let dx = (lure_x - ex) as f32;
        let dy = (lure_y - ey) as f32;
        let d = (dx * dx + dy * dy).sqrt().max(1.0);
        let gx = ((dx / d) * (rx as f32 * 0.30)).round() as i32;
        let gy = ((dy / d) * (ry as f32 * 0.55)).round() as i32;
        (
            gx.clamp(-(rx / 3).max(1), (rx / 3).max(1)),
            gy.clamp(-(ry / 2).max(1), (ry / 2).max(1)),
        )
    };

    let ray_count = 28 + (seed as usize % 16);
    for i in 0..ray_count {
        let a = phase + i as f32 * TAU / ray_count as f32;
        let long = i % 2 == 0;
        let r0 = if long { 7.0 } else { 5.0 };
        let r1 = if long { 0.99 } else { 0.62 };
        let p0 = pp_point_on(cx, cy, r0, r0 * 0.5, a);
        let p1 = pp_point_on(cx, cy, max_rx * r1, max_ry * r1, a);
        pp_line(grid, p0.0, p0.1, p1.0, p1.1, darken(ray_color, if long { 8 } else { 30 }));
        if long {
            pp_put(grid, p1.0, p1.1, '◇', darken(ray_color, 18));
        }
    }

    for r in [0.42f32, 0.55, 0.70, 0.86] {
        pp_arc(grid, cx, cy, max_rx * r, max_ry * r, 0.0, TAU, darken(gold, 24 + (r * 20.0) as u8), 0);
    }

    let draw_eye = |grid: &mut Grid, ex: i32, ey: i32, erx: i32, ery: i32, gx: i32, gy: i32, io: Color, ii: Color, lid: Color, fibers: usize| {
        let iris_rx = (erx as f32 * 0.44).round().max(1.0) as i32;
        let iris_ry = (ery as f32 * 0.92).round().max(1.0) as i32;
        let pupil_rx = (iris_rx as f32 * 0.42).round().max(1.0) as i32;
        let pupil_ry = (iris_ry as f32 * 0.62).round().max(1.0) as i32;
        let icx = ex + gx;
        let icy = ey + gy;
        for dx in -erx - 1..=erx + 1 {
            let nx = dx as f32 / erx.max(1) as f32;
            if nx.abs() > 1.04 {
                continue;
            }
            let curve = (1.0 - nx.abs().powf(1.8)).max(0.0).powf(0.6);
            let top = (-ery as f32 * curve).round() as i32;
            let bot = (ery as f32 * curve).round() as i32;
            for dy in top..=bot {
                let idx = dx - gx;
                let idy = dy - gy;
                let im = (idx as f32 / iris_rx as f32).powi(2) + (idy as f32 / iris_ry as f32).powi(2);
                if im <= 1.0 {
                    let pm = (idx as f32 / pupil_rx as f32).powi(2) + (idy as f32 / pupil_ry as f32).powi(2);
                    if pm <= 1.0 {
                        pp_put(grid, ex + dx, ey + dy, '●', pupil);
                    } else {
                        pp_put(grid, ex + dx, ey + dy, '·', darken(ii, 14));
                    }
                } else {
                    pp_put(grid, ex + dx, ey + dy, ' ', sclera);
                }
            }
        }
        for i in 0..fibers {
            let a = i as f32 * TAU / fibers as f32;
            let p0 = pp_point_on(icx, icy, pupil_rx as f32 * 1.1, pupil_ry as f32 * 1.1, a);
            let p1 = pp_point_on(icx, icy, iris_rx as f32 * 0.96, iris_ry as f32 * 0.96, a);
            let col = if i % 2 == 0 { io } else { darken(ii, 6) };
            pp_line(grid, p0.0, p0.1, p1.0, p1.1, col);
        }
        pp_arc(grid, icx, icy, pupil_rx as f32 * 1.1, pupil_ry as f32 * 1.1, 0.0, TAU, pupil, 0);
        pp_put(grid, icx, icy, '◉', pupil);
        pp_put(grid, icx - iris_rx / 2, icy - iris_ry / 2, '˙', chalk);
        for dx in -erx - 1..=erx + 1 {
            let nx = dx as f32 / erx.max(1) as f32;
            if nx.abs() > 1.04 {
                continue;
            }
            let curve = (1.0 - nx.abs().powf(1.8)).max(0.0).powf(0.6);
            let top = (-ery as f32 * curve).round() as i32;
            let bot = (ery as f32 * curve).round() as i32;
            let cht = if dx < -erx / 2 { '╭' } else if dx > erx / 2 { '╮' } else { '─' };
            let chb = if dx < -erx / 2 { '╰' } else if dx > erx / 2 { '╯' } else { '─' };
            pp_put(grid, ex + dx, ey + top, cht, lighten(lid, 10));
            pp_put(grid, ex + dx, ey + bot, chb, darken(lid, 4));
        }
        pp_put(grid, ex - erx, ey, '<', lighten(lid, 6));
        pp_put(grid, ex + erx, ey, '>', lighten(lid, 6));
    };

    for ring in 0..2 {
        let count = if ring == 0 { 6 + seed as usize % 3 } else { 9 + seed as usize % 4 };
        let dist = if ring == 0 { 0.42 } else { 0.74 };
        for i in 0..count {
            let a = phase + i as f32 * TAU / count as f32 + ring as f32 * 0.3;
            let ex = (cx as f32 + a.cos() * max_rx * dist).round() as i32;
            let ey = (cy as f32 + a.sin() * max_ry * dist).round() as i32;
            if ex < 6 || ey < 3 || ex >= width as i32 - 6 || ey >= height as i32 - 3 {
                continue;
            }
            let (rx, ry) = if ring == 0 { (8, 4) } else { (5, 2) };
            let (gx, gy) = gaze_for(ex, ey, rx, ry);
            let io = shift_hue(iris_outer, (i as f64 * 47.0) % 160.0 - 80.0);
            let ii = shift_hue(iris_inner, (i as f64 * 33.0) % 120.0 - 60.0);
            let lid = shift_hue(lid_color, (i as f64 * 20.0) % 80.0 - 40.0);
            draw_eye(grid, ex, ey, rx, ry, gx, gy, io, ii, lid, 10 + i % 4);
        }
    }

    let hero_rx = ((width as f32 * 0.21).round() as i32).clamp(10, 24);
    let hero_ry = ((height as f32 * 0.17).round() as i32).clamp(3, 8);
    let hero_y = cy - (height as i32 / 14).max(1);
    let (hgx, hgy) = gaze_for(cx, hero_y, hero_rx, hero_ry);
    draw_eye(grid, cx, hero_y, hero_rx, hero_ry, hgx, hgy, iris_outer, iris_inner, lid_color, 20 + seed as usize % 8);
    pp_line(grid, cx - (hero_rx - 1), hero_y - hero_ry - 1, cx, hero_y - hero_ry - 3, darken(lid_color, 6));
    pp_line(grid, cx, hero_y - hero_ry - 3, cx + (hero_rx - 1), hero_y - hero_ry - 1, darken(lid_color, 6));

    pp_put(grid, lure_x, lure_y, '◆', lighten(lure_col, 12));
}


// --- circuit : PCB traces with Manhattan routing and pads. Static topology is
// fully determined by `seed` (so a snapshot at any t shows the same board); the
// time `t` only drives a bright "current" pulse that flows along every trace,
// looping continuously. A native iterate mode -- smooth motion, no seed morph.
pub(crate) fn draw_circuit(
    grid: &mut Grid,
    width: usize,
    height: usize,
    _seed: u64,
    palette: &[Color; 5],
    rng: &mut StdRng,
    t: f32,
    trace_count: usize,
) {
    // board: faint dot grid
    for y in 0..height {
        for x in 0..width {
            if x % 4 == 0 && y % 2 == 0 {
                grid[y][x] = Cell::new('·', darken(palette[1], 90));
            }
        }
    }

    let free = |g: &Grid, x: i32, y: i32| -> bool {
        let c = g[y as usize][x as usize].ch;
        c == ' ' || c == '·'
    };

    let trace_colors = [palette[1], palette[2], palette[3]];
    // Keep each trace's polyline + color so the pulse pass can re-walk it.
    let mut traces: Vec<(Vec<(i32, i32)>, Color)> = Vec::new();
    let mut placed = 0;
    let mut attempts = 0;
    while placed < trace_count && attempts < trace_count * 8 {
        attempts += 1;
        let mut x = rng.random_range(2..width as i32 - 2);
        let mut y = rng.random_range(1..height as i32 - 1);
        if !free(grid, x, y) {
            continue;
        }

        let mut pts: Vec<(i32, i32)> = vec![(x, y)];
        let dirs = [(1i32, 0i32), (-1, 0), (0, 1), (0, -1)];
        let mut dir = dirs[rng.random_range(0..4)];
        let segs = rng.random_range(2..5);
        'seg: for _ in 0..segs {
            let len = rng.random_range(4..13);
            for _ in 0..len {
                let nx = x + dir.0;
                let ny = y + dir.1;
                if nx < 1 || ny < 1 || nx >= width as i32 - 1 || ny >= height as i32 - 1 {
                    break 'seg;
                }
                if !free(grid, nx, ny) || pts.contains(&(nx, ny)) {
                    break 'seg;
                }
                x = nx;
                y = ny;
                pts.push((x, y));
            }
            dir = if dir.0 != 0 {
                if rng.random::<f32>() < 0.5 {
                    (0, 1)
                } else {
                    (0, -1)
                }
            } else {
                if rng.random::<f32>() < 0.5 {
                    (1, 0)
                } else {
                    (-1, 0)
                }
            };
        }
        if pts.len() < 4 {
            continue;
        }

        let color = trace_colors[placed % trace_colors.len()];
        let pad_color = lighten(color, 30);
        for i in 0..pts.len() {
            let (px, py) = pts[i];
            let ch = if i == 0 || i == pts.len() - 1 {
                '◉'
            } else {
                let din = (pts[i].0 - pts[i - 1].0, pts[i].1 - pts[i - 1].1);
                let dout = (pts[i + 1].0 - pts[i].0, pts[i + 1].1 - pts[i].1);
                match (din, dout) {
                    ((1, 0), (1, 0)) | ((-1, 0), (-1, 0)) => '─',
                    ((0, 1), (0, 1)) | ((0, -1), (0, -1)) => '│',
                    ((1, 0), (0, 1)) | ((0, -1), (-1, 0)) => '╮',
                    ((1, 0), (0, -1)) | ((0, 1), (-1, 0)) => '╯',
                    ((-1, 0), (0, 1)) | ((0, -1), (1, 0)) => '╭',
                    ((-1, 0), (0, -1)) | ((0, 1), (1, 0)) => '╰',
                    _ => '·',
                }
            };
            let c = if i == 0 || i == pts.len() - 1 {
                pad_color
            } else {
                color
            };
            grid[py as usize][px as usize] = Cell::new(ch, c);
        }
        traces.push((pts, color));
        placed += 1;
    }

    // Current pulse: a bright comet head with a fading tail walks each trace,
    // its head index = t * SPEED + per-trace phase, wrapped over the path length.
    // Cells per t-unit (clock advances ~0.06/frame, so ~0.18 cells/frame).
    const SPEED: f32 = 3.0;
    const TAIL: i32 = 5;
    for (ti, (pts, color)) in traces.iter().enumerate() {
        let len = pts.len() as i32;
        if len < 2 {
            continue;
        }
        // Distinct phase + slight speed spread so traces don't pulse in lockstep.
        let phase = ti as f32 * 0.73;
        let speed = SPEED * (1.0 + ((ti % 5) as f32) * 0.12);
        let head = (t * speed + phase).rem_euclid(len as f32);
        let head_i = head as i32;
        for k in 0..=TAIL {
            let idx = (head_i - k).rem_euclid(len);
            let (px, py) = pts[idx as usize];
            let cur = grid[py as usize][px as usize];
            // brightness fades along the tail; head is brightest
            let fade = 1.0 - k as f32 / (TAIL as f32 + 1.0);
            let amt = (90.0 * fade) as u8;
            grid[py as usize][px as usize] = Cell::new(cur.ch, lighten(*color, amt));
        }
    }
}

// --- snakes : circuit traces that slither. Each snake rides a hidden, fixed
// Manhattan loop (fully determined by `seed`); only a body-window of length L is
// drawn, and the window slides along the loop by `t`, so the trace crawls forever
// with no teleport. Two snakes meeting at perpendicular cells form a bright
// crossover knot. Native iterate mode -- smooth, deterministic in (seed, t).


// --- phyllotaxis : golden-angle sunflower spiral; glyph scales with radius,
//     color ramps outward through the palette. ---
pub(crate) fn draw_phyllotaxis(grid: &mut Grid, width: usize, height: usize, seed: u64, palette: &[Color; 5], rng: &mut StdRng, t: f32) {
    let bg = darken(palette[0], 6);
    for y in 0..height {
        for x in 0..width {
            grid[y][x] = Cell::new(' ', bg);
        }
    }
    let cx = width as f32 / 2.0;
    let cy = height as f32 / 2.0;
    let golden = std::f32::consts::PI * (3.0 - 5.0f32.sqrt());
    let n = 520 + (seed as usize % 280);
    let sx = width as f32 * 0.47 / (n as f32).sqrt();
    let sy = height as f32 * 0.47 / (n as f32).sqrt();
    // t rotates the whole spiral (the florets wheel around the center).
    let rot = (seed as f32 % 360.0).to_radians() + rng.random_range(0.0f32..0.5) + t * 0.15;
    let glyphs = ['·', '∙', '•', '◦', '○', '◌', '✦', '◆', '❀', '✺'];
    for i in 0..n {
        let a = i as f32 * golden + rot;
        let rr = (i as f32).sqrt();
        let x = (cx + a.cos() * sx * rr).round() as i32;
        let y = (cy + a.sin() * sy * rr).round() as i32;
        let t = i as f32 / n as f32;
        let mid = lerp_color(palette[3], palette[1], (t * 2.0).min(1.0));
        let col = lerp_color(mid, palette[2], (t * 2.0 - 1.0).max(0.0));
        let gi = ((1.0 - t) * (glyphs.len() - 1) as f32).round() as usize;
        pp_put(grid, x, y, glyphs[gi.min(glyphs.len() - 1)], col);
    }
    pp_put(grid, cx as i32, cy as i32, '❁', lighten(palette[3], 20));
}


// --- moire : two radial sine gratings interfering; shade ramp + color blend. ---
pub(crate) fn draw_moire(grid: &mut Grid, width: usize, height: usize, _seed: u64, palette: &[Color; 5], rng: &mut StdRng, t: f32) {
    let ramp = [' ', '·', ':', '-', '=', '+', '*', '#', '%', '@'];
    // t drifts the two centers in a slow orbit so the interference fringes flow.
    // Offsets use (cos-1, sin) so they're exactly zero at t=0 (snapshot identity).
    let ax = width as f32 * rng.random_range(0.2..0.4) + ((t * 0.7).cos() - 1.0) * width as f32 * 0.05;
    let ay = height as f32 * rng.random_range(0.3..0.6) + (t * 0.7).sin() * height as f32 * 0.05;
    let bx = width as f32 * rng.random_range(0.6..0.8) - ((t * 0.6).cos() - 1.0) * width as f32 * 0.05;
    let by = height as f32 * rng.random_range(0.4..0.7) - (t * 0.6).sin() * height as f32 * 0.05;
    let f1 = rng.random_range(0.5f32..0.95);
    let f2 = rng.random_range(0.5f32..0.95);
    for y in 0..height {
        for x in 0..width {
            let dx1 = x as f32 - ax;
            let dy1 = (y as f32 - ay) * 2.0;
            let dx2 = x as f32 - bx;
            let dy2 = (y as f32 - by) * 2.0;
            let d1 = (dx1 * dx1 + dy1 * dy1).sqrt();
            let d2 = (dx2 * dx2 + dy2 * dy2).sqrt();
            let v = (d1 * f1 * 0.5).sin() + (d2 * f2 * 0.5).sin();
            let t = (v + 2.0) / 4.0;
            let idx = (t * (ramp.len() - 1) as f32).round() as usize;
            let col = lerp_color(palette[1], palette[3], t);
            grid[y][x] = Cell::new(ramp[idx.min(ramp.len() - 1)], col);
        }
    }
}

pub(crate) fn draw_eyes3(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, args: &[String]) -> Grid {
        // eyes3 [rays=0] [satellites=0] -- radiant all-seeing eye in a stepped pyramid
        let ray_arg: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(0);
        let ray_count = if ray_arg == 0 {
            18 + (seed as usize % 14)
        } else {
            ray_arg.clamp(8, 48)
        };
        let sat_arg: usize = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(0);
        let sat_count = if sat_arg == 0 {
            2 + (seed as usize % 3)
        } else {
            sat_arg.clamp(0, 6)
        };

        let bg = darken(palette[0], 14);
        let chalk = lighten(palette[4], 14);
        let gold = lighten(palette[1], 32);
        let iris_outer = shift_hue(lighten(palette[3], 26), 18.0);
        let iris_inner = lighten(palette[2], 20);
        let lid_color = lighten(palette[1], 18);
        let pupil = darken(palette[0], 2);
        let sclera = lighten(palette[4], 6);
        let ray_color = lighten(palette[4], 18);
        let hush = darken(palette[2], 60);
        let base_c = darken(gold, 8);

        for y in 0..height {
            for x in 0..width {
                let n = (x * 17 + y * 29 + seed as usize * 3) % 97;
                let ch = match n {
                    0 => '·',
                    1 if (x + y) % 3 == 0 => '∙',
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
        let draw_line = |grid: &mut Grid,
                         mut x0: i32,
                         mut y0: i32,
                         x1: i32,
                         y1: i32,
                         fg: Color| {
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
                        fg: Color| {
            let span = (end - start).abs().max(0.05);
            let samples = ((rx + ry) * span * 3.8).max(18.0) as usize;
            let mut pts: Vec<(i32, i32)> = Vec::new();
            for i in 0..=samples {
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
                }
            }
        };
        let draw_small_eye = |grid: &mut Grid,
                              ex: i32,
                              ey: i32,
                              rx: i32,
                              ry: i32,
                              iris_color: Color,
                              lid: Color| {
            let rx = rx.max(3);
            let ry = ry.max(1);
            let iris_rx = (rx / 3).max(1);
            let iris_ry = (ry / 2).max(1);
            for dy in -ry - 1..=ry + 1 {
                for dx in -rx - 1..=rx + 1 {
                    let nx = dx as f32 / rx as f32;
                    let ny = dy as f32 / ry as f32;
                    let metric = nx * nx + ny * ny;
                    if metric > 1.30 {
                        continue;
                    }
                    let edge = (metric - 1.0).abs();
                    let x = ex + dx;
                    let y = ey + dy;
                    if edge < 0.24 {
                        let ch = if dy < -ry / 3 {
                            if dx < -rx / 2 { '╭' } else if dx > rx / 2 { '╮' } else { '─' }
                        } else if dy > ry / 3 {
                            if dx < -rx / 2 { '╰' } else if dx > rx / 2 { '╯' } else { '─' }
                        } else if dx < 0 {
                            '╱'
                        } else if dx > 0 {
                            '╲'
                        } else {
                            '│'
                        };
                        put(grid, x, y, ch, lid);
                        continue;
                    }
                    let im = (dx as f32 / iris_rx as f32).powi(2)
                        + (dy as f32 / iris_ry as f32).powi(2);
                    if im <= 1.0 {
                        put(grid, x, y, '●', pupil);
                    } else {
                        put(grid, x, y, ' ', sclera);
                    }
                }
            }
            put(grid, ex - iris_rx / 2, ey - iris_ry / 2, '˙', chalk);
        };

        let cx = width as i32 / 2;
        let cy = height as i32 / 2;
        let max_rx = (width as f32 / 2.0 - 1.0).max(10.0);
        let max_ry = (height as f32 / 2.0 - 1.0).max(5.0);
        let phase = rng.random_range(0.0..std::f32::consts::TAU) + t_anim * 0.15;

        // radiating light rays from center (alternating long/short)
        for i in 0..ray_count {
            let a = phase + i as f32 * std::f32::consts::TAU / ray_count as f32;
            let long = i % 2 == 0;
            let r0 = if long { 7.0 } else { 5.0 };
            let r1 = if long { 0.98 } else { 0.66 };
            let p0 = point_on(cx, cy, r0, r0 * 0.5, a);
            let p1 = point_on(cx, cy, max_rx * r1, max_ry * r1, a);
            draw_line(
                &mut grid,
                p0.0,
                p0.1,
                p1.0,
                p1.1,
                darken(ray_color, if long { 8 } else { 28 }),
            );
            if long {
                put(&mut grid, p1.0, p1.1, '◇', darken(ray_color, 18));
            }
        }

        // faint halo arcs behind the eye
        for r in [0.46_f32, 0.58, 0.72] {
            draw_arc(
                &mut grid,
                cx,
                cy,
                max_rx * r,
                max_ry * r,
                0.0,
                std::f32::consts::TAU,
                darken(gold, 28 + (r * 18.0) as u8),
            );
        }

        // focal lure (seeded) -- eyes track it for per-seed variation
        let lure_x = (cx + rng.random_range(-(width as i32 / 6)..=(width as i32 / 6)))
            .clamp(8, width as i32 - 8);
        let lure_y = (cy + rng.random_range(-(height as i32 / 5)..=(height as i32 / 5)))
            .clamp(6, height as i32 - 4);
        let lure_col = shift_hue(lighten(iris_inner, 26), rng.random_range(-40..=40) as f64);
        for dy in -2i32..=2 {
            for dx in -4i32..=4 {
                let m = (dx as f32 / 4.0).powi(2) + (dy as f32 / 2.0).powi(2);
                if m <= 1.0 && (dx.abs() + dy.abs()) % 2 == 0 {
                    put(&mut grid, lure_x + dx, lure_y + dy, '·', darken(lure_col, 24));
                }
            }
        }

        // gaze helper: iris offset toward lure, clamped to eye size
        let gaze_for = |ex: i32, ey: i32, rx: i32, ry: i32| -> (i32, i32) {
            let dx = (lure_x - ex) as f32;
            let dy = (lure_y - ey) as f32;
            let d = (dx * dx + dy * dy).sqrt().max(1.0);
            let gx = ((dx / d) * (rx as f32 * 0.30)).round() as i32;
            let gy = ((dy / d) * (ry as f32 * 0.55)).round() as i32;
            (
                gx.clamp(-(rx / 3).max(1), (rx / 3).max(1)),
                gy.clamp(-(ry / 2).max(1), (ry / 2).max(1)),
            )
        };
        // equilateral triangle, drawn via 3 line edges (rotatable -> "angled")
        let draw_equilateral = |grid: &mut Grid, tcx: i32, tcy: i32, r: f32, rot: f32, fg: Color| {
            let mut verts = [(0i32, 0i32); 3];
            for i in 0..3usize {
                let a = rot + i as f32 * std::f32::consts::TAU / 3.0;
                verts[i] = point_on(tcx, tcy, r, r * 0.5, a);
            }
            for i in 0..3usize {
                let (ax, ay) = verts[i];
                let (bx, by) = verts[(i + 1) % 3];
                draw_line(grid, ax, ay, bx, by, fg);
            }
        };
        // layered almond eye with radial-fiber iris, gaze-shifted toward lure
        let draw_layered_eye = |grid: &mut Grid,
                                ex: i32,
                                ey: i32,
                                erx: i32,
                                ery: i32,
                                gx: i32,
                                gy: i32,
                                io: Color,
                                ii: Color,
                                lid: Color,
                                fibers: usize| {
            let iris_rx = (erx as f32 * 0.44).round() as i32;
            let iris_ry = (ery as f32 * 0.92).round() as i32;
            let pupil_rx = (iris_rx as f32 * 0.42).round() as i32;
            let pupil_ry = (iris_ry as f32 * 0.62).round() as i32;
            let icx = ex + gx;
            let icy = ey + gy;
            for dx in -erx - 1..=erx + 1 {
                let nx = dx as f32 / erx as f32;
                if nx.abs() > 1.04 {
                    continue;
                }
                let curve = (1.0 - nx.abs().powf(1.8)).max(0.0).powf(0.6);
                let top = (-ery as f32 * curve).round() as i32;
                let bot = (ery as f32 * curve).round() as i32;
                for dy in top..=bot {
                    let x = ex + dx;
                    let y = ey + dy;
                    let idx = dx - gx;
                    let idy = dy - gy;
                    let im = (idx as f32 / iris_rx as f32).powi(2)
                        + (idy as f32 / iris_ry as f32).powi(2);
                    if im <= 1.0 {
                        let pm = (idx as f32 / pupil_rx as f32).powi(2)
                            + (idy as f32 / pupil_ry as f32).powi(2);
                        if pm <= 1.0 {
                            put(grid, x, y, '●', pupil);
                        } else {
                            put(grid, x, y, '·', darken(ii, 14));
                        }
                    } else {
                        put(grid, x, y, ' ', sclera);
                    }
                }
            }
            for i in 0..fibers {
                let a = i as f32 * std::f32::consts::TAU / fibers as f32;
                let p0 = point_on(icx, icy, pupil_rx as f32 * 1.1, pupil_ry as f32 * 1.1, a);
                let p1 = point_on(icx, icy, iris_rx as f32 * 0.96, iris_ry as f32 * 0.96, a);
                let col = if i % 2 == 0 { io } else { darken(ii, 6) };
                draw_line(grid, p0.0, p0.1, p1.0, p1.1, col);
            }
            draw_arc(
                grid,
                icx,
                icy,
                pupil_rx as f32 * 1.1,
                pupil_ry as f32 * 1.1,
                0.0,
                std::f32::consts::TAU,
                pupil,
            );
            put(grid, icx, icy, '◉', pupil);
            put(grid, icx - iris_rx / 2, icy - iris_ry / 2, '˙', chalk);
            for dx in -erx - 1..=erx + 1 {
                let nx = dx as f32 / erx as f32;
                if nx.abs() > 1.04 {
                    continue;
                }
                let curve = (1.0 - nx.abs().powf(1.8)).max(0.0).powf(0.6);
                let top = (-ery as f32 * curve).round() as i32;
                let bot = (ery as f32 * curve).round() as i32;
                let cht = if dx < -erx / 2 { '╭' } else if dx > erx / 2 { '╮' } else { '─' };
                let chb = if dx < -erx / 2 { '╰' } else if dx > erx / 2 { '╯' } else { '─' };
                put(grid, ex + dx, ey + top, cht, lighten(lid, 10));
                put(grid, ex + dx, ey + bot, chb, darken(lid, 4));
            }
            put(grid, ex - erx, ey, '<', lighten(lid, 6));
            put(grid, ex + erx, ey, '>', lighten(lid, 6));
        };

        // TRIANGLE CLUSTER -- angled, numerous, overlapping (seeded rotation/count)
        let tri_count = 4 + (seed as usize % 4);
        for t in 0..tri_count {
            let r = max_rx * rng.random_range(0.55..0.95);
            let rot = phase * 0.4
                + t as f32 * std::f32::consts::TAU / 3.0
                + rng.random_range(-0.35..0.35);
            let col = match t % 4 {
                0 => darken(gold, 4),
                1 => chalk,
                2 => darken(gold, 18),
                _ => darken(ray_color, 8),
            };
            draw_equilateral(&mut grid, cx, cy, r, rot, col);
            for i in 0..3usize {
                let a = rot + i as f32 * std::f32::consts::TAU / 3.0;
                let v = point_on(cx, cy, r, r * 0.5, a);
                let g = if (v.0 + v.1 + t as i32) % 2 == 0 { '△' } else { '◆' };
                put(&mut grid, v.0, v.1, g, lighten(col, 8));
            }
        }

        // LAYERED EYE CLUSTER -- secondary eyes behind, gazing at the lure
        let extra = 2 + (seed as usize % 3);
        for i in 0..extra {
            let ang = phase
                + i as f32 * std::f32::consts::TAU / extra as f32
                + rng.random_range(-0.4..0.4);
            let dist = rng.random_range(0.30..0.62);
            let ex = (cx as f32 + ang.cos() * max_rx * dist * 0.8).round() as i32;
            let ey = (cy as f32 + ang.sin() * max_ry * dist * 0.9).round() as i32;
            if ex < 6 || ey < 3 || ex >= width as i32 - 6 || ey >= height as i32 - 3 {
                continue;
            }
            let rx = rng.random_range(7..=12);
            let ry = rng.random_range(3..=5);
            let (gx, gy) = gaze_for(ex, ey, rx, ry);
            let io = shift_hue(iris_outer, rng.random_range(-80..=80) as f64);
            let ii = shift_hue(iris_inner, rng.random_range(-60..=60) as f64);
            let lid = shift_hue(lid_color, rng.random_range(-40..=40) as f64);
            let fibers = 12 + (i + seed as usize) % 5;
            draw_layered_eye(&mut grid, ex, ey, rx, ry, gx, gy, io, ii, lid, fibers);
        }

        // HERO eye on top (biggest, fullest detail), gazing at the lure
        let hero_rx = ((width as f32 * 0.205).round() as i32).clamp(10, 22);
        let hero_ry = ((height as f32 * 0.16).round() as i32).clamp(3, 7);
        let hero_x = cx;
        let hero_y = cy - (height as i32 / 14).max(1);
        let (hgx, hgy) = gaze_for(hero_x, hero_y, hero_rx, hero_ry);
        draw_layered_eye(
            &mut grid,
            hero_x,
            hero_y,
            hero_rx,
            hero_ry,
            hgx,
            hgy,
            iris_outer,
            iris_inner,
            lid_color,
            18 + (seed as usize % 8),
        );
        // hero brow chevron
        let brow_y = hero_y - hero_ry - 2;
        let brow_half = hero_rx - 1;
        draw_line(
            &mut grid,
            hero_x - brow_half,
            brow_y + 1,
            hero_x,
            brow_y - 1,
            darken(lid_color, 6),
        );
        draw_line(
            &mut grid,
            hero_x,
            brow_y - 1,
            hero_x + brow_half,
            brow_y + 1,
            darken(lid_color, 6),
        );

        // tiny corner satellites, staring at the viewer (gaze 0,0)
        let corners = [
            (cx - max_rx as i32 + 3, cy - max_ry as i32 + 2),
            (cx + max_rx as i32 - 3, cy - max_ry as i32 + 2),
            (cx - max_rx as i32 + 4, cy + max_ry as i32 - 2),
            (cx + max_rx as i32 - 4, cy + max_ry as i32 - 2),
            (cx, cy + max_ry as i32 - 1),
        ];
        for i in 0..sat_count.min(corners.len()) {
            let (sx, sy) = corners[i];
            if sx < 3 || sy < 2 || sx >= width as i32 - 3 || sy >= height as i32 - 2 {
                continue;
            }
            let srx = rng.random_range(3..=5);
            let sry = rng.random_range(2..=3);
            let iris = shift_hue(iris_outer, rng.random_range(-60..=60) as f64);
            let lid = shift_hue(lid_color, rng.random_range(-40..=40) as f64);
            draw_small_eye(&mut grid, sx, sy, srx, sry, iris, lid);
        }

        // lure core on top of everything so the focal point always reads
        put(&mut grid, lure_x, lure_y, '◆', lighten(lure_col, 12));
    grid
}

pub(crate) fn draw_spiro(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, args: &[String]) -> Grid {
        // spiro [curves=0] [density=0] -- layered hypotrochoid / harmonograph curves
        let curve_arg: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(0);
        let curve_count = if curve_arg == 0 {
            2 + (seed as usize % 3)
        } else {
            curve_arg.clamp(1, 6)
        };
        let density_arg: usize = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(0);
        let density = if density_arg == 0 {
            1400 + (seed as usize % 800)
        } else {
            density_arg.clamp(400, 6000)
        };

        let bg = darken(palette[0], 14);
        let chalk = lighten(palette[4], 12);
        let gold = lighten(palette[1], 30);
        let cyan = shift_hue(lighten(palette[3], 34), 35.0);
        let magenta = shift_hue(lighten(palette[2], 40), -42.0);
        let lime = shift_hue(lighten(palette[1], 30), 90.0);
        let curve_colors = [chalk, gold, cyan, magenta, lime, chalk];

        for y in 0..height {
            for x in 0..width {
                grid[y][x] = Cell::new(' ', bg);
            }
        }
        for _ in 0..(width * height / 90) {
            let x = rng.random_range(0..width);
            let y = rng.random_range(0..height);
            grid[y][x] = Cell::new('·', darken(chalk, 60));
        }

        let put = |grid: &mut Grid, x: i32, y: i32, ch: char, fg: Color| {
            if x >= 0 && y >= 0 && (x as usize) < width && (y as usize) < height {
                grid[y as usize][x as usize] = Cell::new(ch, fg);
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

        let cx = width as f32 / 2.0;
        let cy = height as f32 / 2.0;
        let scale = ((width as f32 / 2.0) - 2.0).min(height as f32 - 2.0).max(8.0);

        for ci in 0..curve_count {
            let r_big = scale * rng.random_range(0.55..0.95);
            let k = rng.random_range(2..9) as f32;
            let r_small = r_big / k;
            let d = r_small * rng.random_range(0.5..1.8);
            let rot = rng.random_range(0.0..std::f32::consts::TAU) + t_anim * (0.1 + ci as f32 * 0.03);
            let turns = (k as i32 + 1) * 2;
            let samples = density;
            let color = curve_colors[ci % curve_colors.len()];
            let accent = lighten(color, 14);
            let cosr = rot.cos();
            let sinr = rot.sin();

            let mut pts: Vec<(i32, i32)> = Vec::with_capacity(samples);
            for i in 0..=samples {
                let t = i as f32 / samples as f32 * std::f32::consts::TAU * turns as f32;
                let xg = (r_big - r_small) * t.cos()
                    + d * ((r_big - r_small) / r_small * t).cos();
                let yg = (r_big - r_small) * t.sin()
                    - d * ((r_big - r_small) / r_small * t).sin();
                let xr = xg * cosr - yg * sinr;
                let yr = xg * sinr + yg * cosr;
                let px = (cx + xr).round() as i32;
                let py = (cy + yr * 0.5).round() as i32;
                if pts.last().copied() != Some((px, py)) {
                    pts.push((px, py));
                }
            }
            for i in 1..pts.len().saturating_sub(1) {
                let ch = curve_char(pts[i - 1], pts[i], pts[i + 1]);
                let col = if (i + ci * 5) % 11 == 0 {
                    accent
                } else {
                    color
                };
                put(&mut grid, pts[i].0, pts[i].1, ch, col);
            }
        }
        put(&mut grid, cx.round() as i32, cy.round() as i32, '⊙', lighten(chalk, 10));
    grid
}

pub(crate) fn draw_spiro_tile(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, args: &[String]) -> Grid {
        // spiro-tile [cols=0] [rows=0] [vary=0] -- tessellated grid of small spiro motifs
        let col_arg: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(0);
        let cols = if col_arg == 0 {
            4 + (seed as usize % 3)
        } else {
            col_arg.clamp(2, 10)
        };
        let row_arg: usize = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(0);
        let rows = if row_arg == 0 {
            2 + (seed as usize % 3)
        } else {
            row_arg.clamp(2, 8)
        };
        let vary_arg: usize = args.get(6).and_then(|s| s.parse().ok()).unwrap_or(0);
        let vary = vary_arg != 0 || (seed as usize % 4 == 0);

        let bg = darken(palette[0], 13);
        let chalk = lighten(palette[4], 12);
        let gold = lighten(palette[1], 28);
        let cyan = shift_hue(lighten(palette[3], 32), 35.0);
        let rose = shift_hue(lighten(palette[2], 38), -40.0);
        let tile_colors = [chalk, gold, cyan, rose, chalk];
        let border_color = darken(chalk, 50);

        let base_k = (3 + seed as usize % 5) as f32;
        let base_dp = 0.72 + (seed as f32 * 0.1).fract() * 0.6;
        let turns_base = (base_k as i32 + 1) * 2;

        for y in 0..height {
            for x in 0..width {
                grid[y][x] = Cell::new(' ', bg);
            }
        }

        let put = |grid: &mut Grid, x: i32, y: i32, ch: char, fg: Color| {
            if x >= 0 && y >= 0 && (x as usize) < width && (y as usize) < height {
                grid[y as usize][x as usize] = Cell::new(ch, fg);
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
        let draw_box = |grid: &mut Grid, x0: i32, y0: i32, x1: i32, y1: i32, fg: Color| {
            for x in x0 + 1..x1 {
                put(grid, x, y0, '─', fg);
                put(grid, x, y1, '─', fg);
            }
            for y in y0 + 1..y1 {
                put(grid, x0, y, '│', fg);
                put(grid, x1, y, '│', fg);
            }
            put(grid, x0, y0, '╭', fg);
            put(grid, x1, y0, '╮', fg);
            put(grid, x0, y1, '╰', fg);
            put(grid, x1, y1, '╯', fg);
        };

        let tile_w = width / cols;
        let tile_h = height / rows;

        for ry in 0..rows {
            for rx in 0..cols {
                let x0 = (rx * tile_w) as i32;
                let y0 = (ry * tile_h) as i32;
                let x1 = (((rx + 1) * tile_w).min(width - 1)) as i32;
                let y1 = (((ry + 1) * tile_h).min(height - 1)) as i32;
                draw_box(&mut grid, x0, y0, x1, y1, border_color);

                let tx = ((rx * tile_w + tile_w / 2) as f32).min(width as f32 - 1.5);
                let ty = ((ry * tile_h + tile_h / 2) as f32).min(height as f32 - 1.0);
                let hw = (tile_w as f32 / 2.0 - 1.5).max(2.0);
                let hh = (tile_h as f32 / 2.0 - 1.0).max(1.5);
                let scale = hw.min(hh * 2.0).max(2.0);

                let (k, dp, rot, flip) = if vary {
                    let k = (base_k + ((rx as i32 - ry as i32) as f32 * 0.16)).max(2.0);
                    let dp = base_dp + (rx as f32 * 0.05).sin() * 0.30 + (ry as f32 * 0.07).cos() * 0.20;
                    let rot = (rx as f32 + ry as f32 * 1.3) * 0.42 + t_anim * 0.12;
                    let flip = (rx + ry) % 2 == 0;
                    (k, dp, rot, flip)
                } else {
                    (base_k, base_dp, t_anim * 0.12, (rx + ry) % 2 == 0)
                };

                let color = tile_colors[(rx + ry * 2) % tile_colors.len()];
                let accent = shift_hue(color, 55.0);

                for ci in 0..2usize {
                    let r_big = scale * (0.88 - ci as f32 * 0.18);
                    let r_small = r_big / k;
                    let d = r_small * dp * if ci == 0 { 1.0 } else { 1.45 };
                    let cosr = rot.cos();
                    let sinr = rot.sin();
                    let turns = turns_base + ci as i32;
                    let samples = 420;
                    let mut pts: Vec<(i32, i32)> = Vec::new();
                    for i in 0..=samples {
                        let t = i as f32 / samples as f32 * std::f32::consts::TAU * turns as f32;
                        let xg = (r_big - r_small) * t.cos()
                            + d * ((r_big - r_small) / r_small * t).cos();
                        let yg = (r_big - r_small) * t.sin()
                            - d * ((r_big - r_small) / r_small * t).sin();
                        let (xg, yg) = if flip { (xg, -yg) } else { (xg, yg) };
                        let xr = xg * cosr - yg * sinr;
                        let yr = xg * sinr + yg * cosr;
                        let px = (tx + xr).round() as i32;
                        let py = (ty + yr * 0.5).round() as i32;
                        if pts.last().copied() != Some((px, py)) {
                            pts.push((px, py));
                        }
                    }
                    let col = if ci == 0 { color } else { darken(accent, 4) };
                    for i in 1..pts.len().saturating_sub(1) {
                        let ch = curve_char(pts[i - 1], pts[i], pts[i + 1]);
                        put(&mut grid, pts[i].0, pts[i].1, ch, col);
                    }
                }
                put(
                    &mut grid,
                    tx.round() as i32,
                    ty.round() as i32,
                    '·',
                    darken(color, 26),
                );
            }
        }
    grid
}

pub(crate) fn draw_weave(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, args: &[String]) -> Grid {
        // weave [horiz=0] [vert=0] -- interlaced wavy warp/weft strands
        let h_arg: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(0);
        let h_count = if h_arg == 0 {
            3 + (seed as usize % 3)
        } else {
            h_arg.clamp(2, 8)
        };
        let v_arg: usize = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(0);
        let v_count = if v_arg == 0 {
            3 + (seed as usize % 4)
        } else {
            v_arg.clamp(2, 8)
        };

        let bg = darken(palette[0], 10);
        let chalk = lighten(palette[4], 12);
        let gold = lighten(palette[1], 28);
        let cyan = shift_hue(lighten(palette[3], 32), 35.0);
        let rose = shift_hue(lighten(palette[2], 38), -40.0);
        let lime = shift_hue(lighten(palette[1], 26), 90.0);
        let h_colors = [chalk, gold, cyan, rose, lime, chalk, gold, cyan];
        let v_colors = [gold, rose, lime, chalk, cyan, gold, rose, lime];

        for y in 0..height {
            for x in 0..width {
                grid[y][x] = Cell::new(' ', bg);
            }
        }

        let put = |grid: &mut Grid, x: i32, y: i32, ch: char, fg: Color| {
            if x >= 0 && y >= 0 && (x as usize) < width && (y as usize) < height {
                grid[y as usize][x as usize] = Cell::new(ch, fg);
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
            } else if dx2 == 0 || dx1 == 0 {
                '│'
            } else if dy2 == 0 || dy1 == 0 {
                '─'
            } else {
                '┼'
            }
        };

        let mut h_spec: Vec<(f32, f32, f32, f32, Color)> = Vec::new();
        let mut v_spec: Vec<(f32, f32, f32, f32, Color)> = Vec::new();
        for i in 0..h_count {
            let base = (i as f32 + 0.5) * (height as f32 / h_count as f32);
            let amp = (height as f32 / (h_count as f32 + 1.0)) * rng.random_range(0.35..0.7);
            let freq = rng.random_range(0.06..0.16);
            let phase = rng.random_range(0.0..std::f32::consts::TAU) + t_anim * 0.6; // warp strands drift
            h_spec.push((base, amp, freq, phase, h_colors[i % h_colors.len()]));
        }
        for i in 0..v_count {
            let base = (i as f32 + 0.5) * (width as f32 / v_count as f32);
            let amp = (width as f32 / (v_count as f32 + 1.0)) * rng.random_range(0.30..0.62);
            let freq = rng.random_range(0.06..0.16);
            let phase = rng.random_range(0.0..std::f32::consts::TAU) - t_anim * 0.6; // weft strands counter-drift
            v_spec.push((base, amp, freq, phase, v_colors[i % v_colors.len()]));
        }

        let y_h = |i: usize, x: i32| -> i32 {
            let (base, amp, freq, phase, _) = h_spec[i];
            (base + amp * (freq * x as f32 + phase).sin()).round() as i32
        };
        let x_v = |i: usize, y: i32| -> i32 {
            let (base, amp, freq, phase, _) = v_spec[i];
            (base + amp * (freq * y as f32 + phase).sin()).round() as i32
        };

        // occupancy: which strand ids pass through a cell
        let mut occ_h: Vec<Vec<Vec<u8>>> = vec![vec![Vec::new(); width]; height];
        let mut occ_v: Vec<Vec<Vec<u8>>> = vec![vec![Vec::new(); width]; height];
        for i in 0..h_count {
            for x in 0..width {
                let y = y_h(i, x as i32);
                if y >= 0 && (y as usize) < height {
                    occ_h[y as usize][x].push(i as u8);
                }
            }
        }
        for i in 0..v_count {
            for y in 0..height {
                let x = x_v(i, y as i32);
                if x >= 0 && (x as usize) < width {
                    occ_v[y][x as usize].push(i as u8);
                }
            }
        }

        // draw horizontal strands
        for i in 0..h_count {
            let color = h_spec[i].4;
            let mut prev = (0, y_h(i, 0));
            let mut here = prev;
            for x in 0..width {
                let next = ((x + 1) as i32, y_h(i, (x + 1) as i32));
                if x > 0 {
                    prev = ((x - 1) as i32, y_h(i, (x - 1) as i32));
                }
                here = (x as i32, y_h(i, x as i32));
                let y = here.1;
                if y < 0 || (y as usize) >= height {
                    continue;
                }
                // over/under: skip if a vertical dominates here
                let vs = &occ_v[y as usize][x];
                let mut under = false;
                if !vs.is_empty() {
                    let v0 = vs[0] as usize;
                    if (i + v0) % 2 != 0 {
                        under = true;
                    }
                }
                if !under {
                    let ch = if x == 0 || x == width - 1 {
                        '─'
                    } else {
                        curve_char(prev, here, next)
                    };
                    put(&mut grid, here.0, here.1, ch, color);
                }
            }
        }
        // draw vertical strands
        for i in 0..v_count {
            let color = v_spec[i].4;
            let mut prev = (x_v(i, 0), 0);
            let mut here = prev;
            for y in 0..height {
                let next = (x_v(i, (y + 1) as i32), (y + 1) as i32);
                if y > 0 {
                    prev = (x_v(i, (y - 1) as i32), (y - 1) as i32);
                }
                here = (x_v(i, y as i32), y as i32);
                let x = here.0;
                if x < 0 || (x as usize) >= width {
                    continue;
                }
                let hs = &occ_h[y][x as usize];
                let mut under = false;
                if !hs.is_empty() {
                    let h0 = hs[0] as usize;
                    if (h0 + i) % 2 == 0 {
                        under = true;
                    }
                }
                if !under {
                    let ch = if y == 0 || y == height - 1 {
                        '│'
                    } else {
                        curve_char(prev, here, next)
                    };
                    put(&mut grid, here.0, here.1, ch, color);
                }
            }
        }
    grid
}

pub(crate) fn draw_gears(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, args: &[String]) -> Grid {
        // gears [count=0] [teeth=0] -- interlocking clockwork mechanism
        let count_arg: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(0);
        let gear_count = if count_arg == 0 {
            2 + (seed as usize % 3)
        } else {
            count_arg.clamp(2, 4)
        };
        let teeth_arg: usize = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(0);
        let base_teeth = if teeth_arg == 0 {
            8 + (seed as usize % 9)
        } else {
            teeth_arg.clamp(6, 24)
        };

        let bg = darken(palette[0], 12);
        let chalk = lighten(palette[4], 14);
        let brass = lighten(palette[1], 30);
        let steel = lighten(palette[3], 28);
        let copper = shift_hue(lighten(palette[2], 36), -22.0);
        let patina = shift_hue(lighten(palette[1], 24), 92.0);
        let gear_colors = [chalk, brass, steel, copper, patina, brass];
        let hush = darken(palette[2], 66);

        for y in 0..height {
            for x in 0..width {
                let n = (x * 17 + y * 23 + seed as usize * 7) % 149;
                let ch = if n == 0 { '·' } else { ' ' };
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
        let draw_line = |grid: &mut Grid,
                         mut x0: i32,
                         mut y0: i32,
                         x1: i32,
                         y1: i32,
                         fg: Color| {
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
                        fg: Color| {
            let span = (end - start).abs().max(0.05);
            let samples = ((rx + ry) * span * 3.8).max(18.0) as usize;
            let mut pts: Vec<(i32, i32)> = Vec::new();
            for i in 0..=samples {
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
                }
            }
        };
        let draw_gear = |grid: &mut Grid,
                         cx: i32,
                         cy: i32,
                         r: i32,
                         teeth: usize,
                         phase: f32,
                         fg: Color,
                         accent: Color| {
            let rf = r as f32;
            let rx = rf;
            let ry = rf * 0.5;
            // teeth: radial bars from rim to tip
            for i in 0..teeth {
                let a = phase + i as f32 * std::f32::consts::TAU / teeth as f32;
                let inner = point_on(cx, cy, rx * 0.92, ry * 0.92, a);
                let outer = point_on(cx, cy, rx + 1.4, ry + 0.7, a);
                draw_line(grid, inner.0, inner.1, outer.0, outer.1, accent);
                let tip = point_on(cx, cy, rx + 1.4, ry + 0.7, a);
                put(grid, tip.0, tip.1, '◆', lighten(accent, 10));
            }
            // rim
            draw_arc(grid, cx, cy, rx, ry, 0.0, std::f32::consts::TAU, fg);
            draw_arc(grid, cx, cy, rx * 0.82, ry * 0.82, 0.0, std::f32::consts::TAU, darken(fg, 8));
            // spokes
            let spokes = (teeth / 2).clamp(3, 6);
            for s in 0..spokes {
                let a = phase + s as f32 * std::f32::consts::TAU / spokes as f32;
                let p_in = point_on(cx, cy, rx * 0.30, ry * 0.30, a);
                let p_out = point_on(cx, cy, rx * 0.80, ry * 0.80, a);
                draw_line(grid, p_in.0, p_in.1, p_out.0, p_out.1, darken(fg, 6));
                put(grid, p_out.0, p_out.1, '○', darken(accent, 6));
            }
            // hub
            draw_arc(grid, cx, cy, rx * 0.30, ry * 0.30, 0.0, std::f32::consts::TAU, accent);
            put(grid, cx, cy, '⊙', lighten(accent, 14));
        };

        // layout: place gears, each tangent to an existing one
        let cx0 = (width as f32 / 2.0).round() as i32;
        let cy0 = (height as f32 / 2.0).round() as i32;
        let r0 = (width.min(height * 2) as f32 / 7.0).round() as i32;
        let mut placed: Vec<(i32, i32, i32, usize, f32, f32, Color)> = Vec::new();
        placed.push((
            cx0,
            cy0,
            r0,
            base_teeth,
            rng.random_range(0.0..std::f32::consts::TAU),
            1.0,
            gear_colors[0],
        ));
        let mut attempts = 0;
        while placed.len() < gear_count && attempts < 60 {
            attempts += 1;
            let anchor = placed[rng.random_range(0..placed.len())];
            let (ax, ay, ar, _, _, _, _) = anchor;
            let new_r = ((ar as f32) * rng.random_range(0.6..1.1))
                .clamp(4.0, (width.min(height * 2) as f32 / 5.0));
            let new_ri = new_r.round() as i32;
            let ang = rng.random_range(0.0..std::f32::consts::TAU);
            let dist = (ar + new_ri) as f32;
            let nx = ax + (dist * ang.cos()).round() as i32;
            let ny = ay + (dist * ang.sin() * 0.5).round() as i32;
            let margin = new_ri + 2;
            if nx < margin
                || ny < margin / 2
                || nx >= width as i32 - margin
                || ny >= height as i32 - margin / 2
            {
                continue;
            }
            // reject overlap
            let mut overlap = false;
            for (ox, oy, or_, _, _, _, _) in &placed {
                let dx = nx - ox;
                let dy = (ny - oy) * 2;
                let d = ((dx * dx + dy * dy) as f32).sqrt();
                if d < (new_ri + or_) as f32 * 0.92 {
                    overlap = true;
                    break;
                }
            }
            if overlap {
                continue;
            }
            let teeth = (base_teeth as f32 * new_r / r0 as f32)
                .round()
                .max(6.0) as usize;
            // mesh tooth phase: half-tooth offset relative to anchor
            let anchor_teeth = anchor.3;
            let anchor_phase = anchor.4;
            let anchor_speed = anchor.5;
            let mesh_angle = ang + std::f32::consts::PI;
            let off = std::f32::consts::TAU / anchor_teeth as f32 / 2.0;
            let phase = mesh_angle
                + off
                - (std::f32::consts::TAU / teeth as f32)
                * ((mesh_angle - anchor_phase) / (std::f32::consts::TAU / teeth as f32)).round();
            let speed = -anchor_speed * anchor_teeth as f32 / teeth as f32;
            let color = gear_colors[placed.len() % gear_colors.len()];
            placed.push((nx, ny, new_ri, teeth, phase, speed, color));
        }

        for &(_, _, _, _, _, _, color) in &placed {
            // shadow lines: none; gears drawn next
        }
        for &(gx, gy, gr, gteeth, gphase, gspeed, color) in &placed {
            draw_gear(
                &mut grid,
                gx,
                gy,
                gr,
                gteeth,
                gphase + gspeed * t_anim * 0.15,
                darken(color, 6),
                lighten(color, 12),
            );
        }
    grid
}

pub(crate) fn draw_kaleido(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, args: &[String]) -> Grid {
        // kaleido [folds=0] [strokes=0] [mirror=0] -- N-fold symmetric mandala
        let fold_arg: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(0);
        let folds = if fold_arg == 0 {
            [6, 8, 12, 5, 7][(seed as usize) % 5]
        } else {
            fold_arg.clamp(3, 16)
        };
        let stroke_arg: usize = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(0);
        let stroke_count = if stroke_arg == 0 {
            8 + (seed as usize % 7)
        } else {
            stroke_arg.clamp(3, 24)
        };
        let mirror_arg: usize = args.get(6).and_then(|s| s.parse().ok()).unwrap_or(0);
        let mirror = mirror_arg != 0 || (seed as usize % 3 == 0);

        let bg = darken(palette[0], 14);
        let chalk = lighten(palette[4], 12);
        let gold = lighten(palette[1], 30);
        let cyan = shift_hue(lighten(palette[3], 34), 35.0);
        let magenta = shift_hue(lighten(palette[2], 40), -42.0);
        let lime = shift_hue(lighten(palette[1], 28), 90.0);
        let violet = shift_hue(lighten(palette[3], 30), 150.0);
        let stroke_colors = [chalk, gold, cyan, magenta, lime, violet, chalk, gold];

        for y in 0..height {
            for x in 0..width {
                grid[y][x] = Cell::new(' ', bg);
            }
        }
        for _ in 0..(width * height / 120) {
            let x = rng.random_range(0..width);
            let y = rng.random_range(0..height);
            grid[y][x] = Cell::new('·', darken(chalk, 62));
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
        let draw_line = |grid: &mut Grid,
                         mut x0: i32,
                         mut y0: i32,
                         x1: i32,
                         y1: i32,
                         fg: Color| {
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

        let cx = width as f32 / 2.0;
        let cy = height as f32 / 2.0;
        let max_r = (width.min(height * 2) as f32 / 2.0 - 2.0).max(8.0);
        let wedge = std::f32::consts::TAU / folds as f32;

        // generate strokes within wedge [0, wedge] as (kind, p1, p2, r, a0, a1, color_idx, ch)
        enum SK {
            Seg((f32, f32), (f32, f32)),
            Arc((f32, f32), f32, f32, f32),
            Dot((f32, f32)),
        }
        let polar = |rad: f32, ang: f32| -> (f32, f32) {
            (rad * ang.cos(), rad * ang.sin() * 0.5)
        };
        let mut strokes: Vec<(SK, usize, char)> = Vec::new();
        for s in 0..stroke_count {
            let color_idx = s % stroke_colors.len();
            let kind = seed as usize + s;
            match kind % 4 {
                0 => {
                    let r1 = rng.random_range(0.15..0.95) * max_r;
                    let r2 = rng.random_range(0.15..0.95) * max_r;
                    let a1 = rng.random_range(0.0..wedge);
                    let a2 = rng.random_range(0.0..wedge);
                    strokes.push((SK::Seg(polar(r1, a1), polar(r2, a2)), color_idx, '─'));
                }
                1 => {
                    let rc = rng.random_range(0.2..0.85) * max_r;
                    let ac = rng.random_range(0.05..wedge - 0.05);
                    let ar = rng.random_range(0.06..0.22) * max_r;
                    let a0 = rng.random_range(0.0..std::f32::consts::TAU);
                    let a1 = a0 + rng.random_range(0.6..2.4);
                    let center = polar(rc, ac);
                    strokes.push((SK::Arc(center, ar, a0, a1), color_idx, '○'));
                }
                2 => {
                    let r1 = rng.random_range(0.2..0.9) * max_r;
                    let a1 = rng.random_range(0.0..wedge);
                    let glyphs = ['◇', '△', '▽', '○', '✦', '⊕', '⊙', '⌬', '□'];
                    strokes.push((
                        SK::Dot(polar(r1, a1)),
                        color_idx,
                        glyphs[(s + seed as usize) % glyphs.len()],
                    ));
                }
                _ => {
                    // short chord cluster: two segments sharing an endpoint
                    let r0 = rng.random_range(0.3..0.9) * max_r;
                    let a0 = rng.random_range(0.0..wedge);
                    let pivot = polar(r0, a0);
                    let r1 = rng.random_range(0.15..0.95) * max_r;
                    let a1 = rng.random_range(0.0..wedge);
                    let r2 = rng.random_range(0.15..0.95) * max_r;
                    let a2 = rng.random_range(0.0..wedge);
                    strokes.push((SK::Seg(pivot, polar(r1, a1)), color_idx, '─'));
                    strokes.push((SK::Seg(pivot, polar(r2, a2)), color_idx, '─'));
                }
            }
        }

        let rotate = |p: (f32, f32), ang: f32| -> (f32, f32) {
            (p.0 * ang.cos() - p.1 * ang.sin(), p.0 * ang.sin() + p.1 * ang.cos())
        };
        let _ = rotate;

        let to_screen = |lp: (f32, f32), ox: f32, oy: f32| -> (i32, i32) {
            ((ox + lp.0).round() as i32, (oy + lp.1).round() as i32)
        };

        let mut pass = |sign: f32| {
            for k in 0..folds {
                let ang = k as f32 * wedge + t_anim * 0.1; // T spins the mandala
                let cosr = ang.cos();
                let sinr = ang.sin();
                let rot = |p: (f32, f32)| -> (f32, f32) {
                    // p.y already aspect-compressed (×0.5); rotate in that space
                    (p.0 * cosr - p.1 * sinr, p.0 * sinr + p.1 * cosr)
                };
                let rot_m = |p: (f32, f32)| -> (f32, f32) {
                    let q = (p.0, sign * p.1);
                    (q.0 * cosr - q.1 * sinr, q.0 * sinr + q.1 * cosr)
                };
                for (sk, cidx, glyph) in &strokes {
                    let color = stroke_colors[*cidx % stroke_colors.len()];
                    match sk {
                        SK::Seg(a, b) => {
                            let (a2, b2) = if sign != 0.0 {
                                (rot_m(*a), rot_m(*b))
                            } else {
                                (rot(*a), rot(*b))
                            };
                            let pa = to_screen(a2, cx, cy);
                            let pb = to_screen(b2, cx, cy);
                            draw_line(&mut grid, pa.0, pa.1, pb.0, pb.1, color);
                        }
                        SK::Arc(center, r, a0, a1) => {
                            let c2 = if sign != 0.0 { rot_m(*center) } else { rot(*center) };
                            let cs = to_screen(c2, cx, cy);
                            let samples = ((*r + *r) * (*a1 - *a0).abs() * 3.8).max(12.0) as usize;
                            let mut pts: Vec<(i32, i32)> = Vec::new();
                            for i in 0..=samples {
                                let a = *a0 + (*a1 - *a0) * i as f32 / samples as f32;
                                let lp = (*r * a.cos(), *r * a.sin() * 0.5);
                                let p = to_screen(lp, cs.0 as f32, cs.1 as f32);
                                if pts.last().copied() != Some(p) {
                                    pts.push(p);
                                }
                            }
                            for i in 1..pts.len().saturating_sub(1) {
                                let ch = curve_char(pts[i - 1], pts[i], pts[i + 1]);
                                put(&mut grid, pts[i].0, pts[i].1, ch, color);
                            }
                        }
                        SK::Dot(p) => {
                            let p2 = if sign != 0.0 { rot_m(*p) } else { rot(*p) };
                            let ps = to_screen(p2, cx, cy);
                            put(&mut grid, ps.0, ps.1, *glyph, lighten(color, 12));
                        }
                    }
                }
            }
        };
        pass(0.0);
        if mirror {
            pass(1.0);
        }
        put(&mut grid, cx.round() as i32, cy.round() as i32, '⊙', lighten(chalk, 12));
    grid
}

pub(crate) fn draw_contour(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, args: &[String]) -> Grid {
        // contour [levels=0] [scale=0] -- topographic iso-lines over procedural heightmap
        let level_arg: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(0);
        let level_count = if level_arg == 0 {
            6 + (seed as usize % 5)
        } else {
            level_arg.clamp(3, 14)
        };
        let scale_arg: usize = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(0);
        let hscale = if scale_arg == 0 {
            0.16 + (seed as f32 * 0.013).fract() * 0.10
        } else {
            (scale_arg as f32 / 100.0).clamp(0.05, 0.5)
        };

        let bg = darken(palette[0], 8);
        let deep_c = darken(palette[1], 30);
        let mid_c = lighten(palette[3], 10);
        let high_c = lighten(palette[4], 16);
        let snow = lighten(palette[4], 30);
        let hush = darken(palette[2], 60);

        for y in 0..height {
            for x in 0..width {
                grid[y][x] = Cell::new(' ', bg);
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
        let draw_line = |grid: &mut Grid,
                         mut x0: i32,
                         mut y0: i32,
                         x1: i32,
                         y1: i32,
                         fg: Color| {
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

        // heightmap with sines + random gaussian bumps
        let bump_n = 1 + (seed as usize % 3);
        let mut bumps: Vec<(f32, f32, f32, f32)> = Vec::new();
        for _ in 0..bump_n {
            let bx = rng.random_range(0.1..0.9) * width as f32;
            let by = rng.random_range(0.1..0.9) * height as f32;
            let br = rng.random_range(3.0..9.0);
            let ba = rng.random_range(0.5..1.4) * if rng.random_range(0..2) == 0 { -1.0 } else { 1.0 };
            bumps.push((bx, by, br, ba));
        }
        let hfield = |xf: f32, yf: f32| -> f32 {
            let mut v = (hscale * xf + t_anim * 0.3).sin() * (hscale * 1.1 * yf).cos();
            v += 0.6 * (hscale * 1.8 * xf + 0.4 - t_anim * 0.2).sin() * (hscale * 1.3 * yf - 0.2).sin();
            v += 0.4 * (hscale * 0.7 * xf - 0.1).cos() * (hscale * 2.1 * yf + 0.5 + t_anim * 0.25).cos();
            for &(bx, by, br, ba) in &bumps {
                let dx = xf - bx;
                let dy = yf - by;
                v += ba * (-(dx * dx + dy * dy) / (br * br)).exp();
            }
            v
        };

        let cols = width + 1;
        let rows = height + 1;
        let mut h = vec![vec![0.0f32; cols]; rows];
        let mut hmin = f32::INFINITY;
        let mut hmax = f32::NEG_INFINITY;
        for yy in 0..rows {
            for xx in 0..cols {
                let v = hfield(xx as f32, yy as f32);
                h[yy][xx] = v;
                if v < hmin {
                    hmin = v;
                }
                if v > hmax {
                    hmax = v;
                }
            }
        }

        let level_color = |frac: f32| -> Color {
            if frac < 0.33 {
                let t = frac / 0.33;
                lerp_color(deep_c, mid_c, t)
            } else if frac < 0.7 {
                let t = (frac - 0.33) / 0.37;
                lerp_color(mid_c, high_c, t)
            } else {
                let t = (frac - 0.7) / 0.3;
                lerp_color(high_c, snow, t)
            }
        };

        for li in 0..level_count {
            let frac = (li as f32 + 0.5) / level_count as f32;
            let level = hmin + (hmax - hmin) * frac;
            let color = level_color(frac);
            let major = li % 3 == 0;
            let line_color = if major { lighten(color, 8) } else { darken(color, 10) };

            for yy in 0..height {
                for xx in 0..width {
                    let c00 = h[yy][xx];
                    let c10 = h[yy][xx + 1];
                    let c11 = h[yy + 1][xx + 1];
                    let c01 = h[yy + 1][xx];
                    let mut code = 0u8;
                    if c00 > level {
                        code |= 1;
                    }
                    if c10 > level {
                        code |= 2;
                    }
                    if c11 > level {
                        code |= 4;
                    }
                    if c01 > level {
                        code |= 8;
                    }
                    if code == 0 || code == 15 {
                        continue;
                    }
                    let xf = xx as f32;
                    let yf = yy as f32;
                    let edge_pt = |e: u8| -> (f32, f32) {
                        match e {
                            1 => {
                                // bottom edge: (xf,yf)-(xf+1,yf)
                                let t = (level - c00) / (c10 - c00);
                                (xf + t, yf)
                            }
                            2 => {
                                // right edge: (xf+1,yf)-(xf+1,yf+1)
                                let t = (level - c10) / (c11 - c10);
                                (xf + 1.0, yf + t)
                            }
                            4 => {
                                // top edge: (xf,yf+1)-(xf+1,yf+1)
                                let t = (level - c01) / (c11 - c01);
                                (xf + t, yf + 1.0)
                            }
                            8 => {
                                // left edge: (xf,yf)-(xf,yf+1)
                                let t = (level - c00) / (c01 - c00);
                                (xf, yf + t)
                            }
                            _ => (xf, yf),
                        }
                    };
                    let pairs: &[(u8, u8)] = match code {
                        1 | 14 => &[(8, 1)],
                        2 | 13 => &[(1, 2)],
                        3 | 12 => &[(8, 2)],
                        4 | 11 => &[(2, 4)],
                        5 => &[(8, 4), (1, 2)],
                        6 | 9 => &[(1, 4)],
                        7 | 8 => &[(8, 4)],
                        10 => &[(8, 1), (2, 4)],
                        _ => &[],
                    };
                    for &(ea, eb) in pairs {
                        let pa = edge_pt(ea);
                        let pb = edge_pt(eb);
                        let ax = pa.0.round() as i32;
                        let ay = pa.1.round() as i32;
                        let bx = pb.0.round() as i32;
                        let by = pb.1.round() as i32;
                        if major {
                            draw_line(&mut grid, ax, ay, bx, by, line_color);
                        } else {
                            // minor contour: sparse char along the segment midpoint
                            let mx = ((pa.0 + pb.0) * 0.5).round() as i32;
                            let my = ((pa.1 + pb.1) * 0.5).round() as i32;
                            let glyph = if (xx + yy) % 3 == 0 { '·' } else { '∙' };
                            put(&mut grid, mx, my, glyph, line_color);
                        }
                    }
                }
            }
        }
        // sparse summit markers
        let _ = hush;
    grid
}


// --- stained : Voronoi glass cells with dark leading + jewel seeds. ---
pub(crate) fn draw_stained(grid: &mut Grid, width: usize, height: usize, seed: u64, palette: &[Color; 5], rng: &mut StdRng) {
    let nseeds = 10 + seed as usize % 12;
    let mut sites: Vec<(i32, i32, Color)> = Vec::new();
    for i in 0..nseeds {
        let x = rng.random_range(0..width) as i32;
        let y = rng.random_range(0..height) as i32;
        let base = [palette[1], palette[2], palette[3]][i % 3];
        let col = shift_hue(base, rng.random_range(-90..=90) as f64);
        sites.push((x, y, col));
    }
    let mut id = vec![vec![0usize; width]; height];
    for y in 0..height {
        for x in 0..width {
            let mut best = 0usize;
            let mut bd = i64::MAX;
            for (k, &(sx, sy, _)) in sites.iter().enumerate() {
                let dx = (x as i32 - sx) as i64;
                let dy = ((y as i32 - sy) * 2) as i64;
                let d = dx * dx + dy * dy;
                if d < bd {
                    bd = d;
                    best = k;
                }
            }
            id[y][x] = best;
            let glass = sites[best].2;
            let ch = if (x + y) % 2 == 0 { '∙' } else { '·' };
            grid[y][x] = Cell::new(ch, darken(glass, 8));
        }
    }
    let lead = darken(palette[0], 0);
    for y in 0..height {
        for x in 0..width {
            let here = id[y][x];
            let right = x + 1 < width && id[y][x + 1] != here;
            let down = y + 1 < height && id[y + 1][x] != here;
            if right && down {
                grid[y][x] = Cell::new('┼', lead);
            } else if right {
                grid[y][x] = Cell::new('│', lead);
            } else if down {
                grid[y][x] = Cell::new('─', lead);
            }
        }
    }
    for &(sx, sy, col) in &sites {
        pp_put(grid, sx, sy, '◆', lighten(col, 40));
    }
}

// ============================================================================
// Grid morphing. Tween two finished grids (any modes/seeds) at the pixel layer.
// Four strategies: dissolve, field, transport (glyphs travel), sdf (shapes melt).
// emit_grid + ASCII_GRID_DUMP let the morph driver capture frames by re-running
// the binary, so it works for every mode with no per-mode rewrite.
// ============================================================================

/// Dispatch arm for mode(s): circuit (moved verbatim from run()).
pub(crate) fn cli_circuit(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        // circuit [traces] -- PCB traces with pads, Manhattan routing.
        // Native time T: current pulses flow along each trace (see draw_circuit).
        let trace_count: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(14);
        let trace_count = trace_count.clamp(1, 60);
        draw_circuit(&mut grid, width, height, seed, &palette, &mut rng, t_anim, trace_count);
    (grid, false)
}

/// Dispatch arm for mode(s): eyes3 (moved verbatim from run()).
pub(crate) fn cli_eyes3(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        grid = draw_eyes3(grid, width, height, seed, palette, rng, t_anim, &args);
    (grid, false)
}

/// Dispatch arm for mode(s): spiro (moved verbatim from run()).
pub(crate) fn cli_spiro(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        grid = draw_spiro(grid, width, height, seed, palette, rng, t_anim, &args);
    (grid, false)
}

/// Dispatch arm for mode(s): spiro-tile (moved verbatim from run()).
pub(crate) fn cli_spiro_tile(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        grid = draw_spiro_tile(grid, width, height, seed, palette, rng, t_anim, &args);
    (grid, false)
}

/// Dispatch arm for mode(s): weave (moved verbatim from run()).
pub(crate) fn cli_weave(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        grid = draw_weave(grid, width, height, seed, palette, rng, t_anim, &args);
    (grid, false)
}

/// Dispatch arm for mode(s): gears (moved verbatim from run()).
pub(crate) fn cli_gears(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        grid = draw_gears(grid, width, height, seed, palette, rng, t_anim, &args);
    (grid, false)
}

/// Dispatch arm for mode(s): kaleido (moved verbatim from run()).
pub(crate) fn cli_kaleido(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        grid = draw_kaleido(grid, width, height, seed, palette, rng, t_anim, &args);
    (grid, false)
}

/// Dispatch arm for mode(s): contour (moved verbatim from run()).
pub(crate) fn cli_contour(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        grid = draw_contour(grid, width, height, seed, palette, rng, t_anim, &args);
    (grid, false)
}


/// Dispatch arm for mode(s): phyllotaxis (moved verbatim from run()).
pub(crate) fn cli_phyllotaxis(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        draw_phyllotaxis(&mut grid, width, height, seed, &palette, &mut rng, t_anim);
    (grid, false)
}

/// Dispatch arm for mode(s): moire (moved verbatim from run()).
pub(crate) fn cli_moire(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        draw_moire(&mut grid, width, height, seed, &palette, &mut rng, t_anim);
    (grid, false)
}

/// Dispatch arm for mode(s): stained (moved verbatim from run()).
pub(crate) fn cli_stained(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        draw_stained(&mut grid, width, height, seed, &palette, &mut rng);
    (grid, false)
}
