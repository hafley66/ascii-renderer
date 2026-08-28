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
use crate::modes_tree::*;
use crate::morph::*;
use crate::opts::*;
use crate::registry::*;
use crate::warps::*;

pub(crate) fn pp_put(grid: &mut Grid, x: i32, y: i32, ch: char, fg: Color) {
    if x >= 0 && y >= 0 && (y as usize) < grid.len() && (x as usize) < grid[0].len() {
        grid[y as usize][x as usize] = Cell::new(ch, fg);
    }
}

pub(crate) fn pp_point_on(cx: i32, cy: i32, rx: f32, ry: f32, a: f32) -> (i32, i32) {
    (
        cx + (a.cos() * rx).round() as i32,
        cy + (a.sin() * ry).round() as i32,
    )
}

pub(crate) fn pp_stroke(dx: i32, dy: i32) -> char {
    if dx.abs() > dy.abs() * 2 {
        '─'
    } else if dy.abs() > dx.abs() * 2 {
        '│'
    } else if dx.signum() == dy.signum() {
        '╲'
    } else {
        '╱'
    }
}

pub(crate) fn pp_line(grid: &mut Grid, mut x0: i32, mut y0: i32, x1: i32, y1: i32, fg: Color) {
    let ch = pp_stroke(x1 - x0, y1 - y0);
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    loop {
        pp_put(grid, x0, y0, ch, fg);
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
}

pub(crate) fn pp_arc(grid: &mut Grid, cx: i32, cy: i32, rx: f32, ry: f32, start: f32, end: f32, fg: Color, gap: usize) {
    let samples = ((rx + ry) * 16.0).max(90.0) as usize;
    let mut prev: Option<(i32, i32)> = None;
    for i in 0..=samples {
        if gap > 0 && i % gap == gap - 1 {
            prev = None;
            continue;
        }
        let a = start + (end - start) * i as f32 / samples as f32;
        let p = pp_point_on(cx, cy, rx, ry, a);
        if let Some(q) = prev {
            pp_line(grid, q.0, q.1, p.0, p.1, fg);
        } else {
            pp_put(grid, p.0, p.1, '·', fg);
        }
        prev = Some(p);
    }
}


pub(crate) fn pp_hash2(x: i32, y: i32, seed: u64) -> f32 {
    let mut h = (x as i64)
        .wrapping_mul(374761393)
        ^ (y as i64).wrapping_mul(668265263)
        ^ (seed as i64).wrapping_mul(2246822519);
    h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    ((h & 0xffff) as f32) / 65535.0
}

pub(crate) fn pp_vnoise(fx: f32, fy: f32, seed: u64) -> f32 {
    let x0 = fx.floor() as i32;
    let y0 = fy.floor() as i32;
    let tx = fx - x0 as f32;
    let ty = fy - y0 as f32;
    let sx = tx * tx * (3.0 - 2.0 * tx);
    let sy = ty * ty * (3.0 - 2.0 * ty);
    let n00 = pp_hash2(x0, y0, seed);
    let n10 = pp_hash2(x0 + 1, y0, seed);
    let n01 = pp_hash2(x0, y0 + 1, seed);
    let n11 = pp_hash2(x0 + 1, y0 + 1, seed);
    let a = n00 + (n10 - n00) * sx;
    let b = n01 + (n11 - n01) * sx;
    a + (b - a) * sy
}

pub(crate) fn pp_fbm(fx: f32, fy: f32, seed: u64) -> f32 {
    let mut v = 0.0;
    let mut amp = 0.5;
    let mut freq = 1.0;
    for o in 0..4u64 {
        v += amp * pp_vnoise(fx * freq, fy * freq, seed.wrapping_add(o * 101));
        amp *= 0.5;
        freq *= 2.0;
    }
    v
}


/// 2-pass chamfer distance transform; vertical cost is doubled for the 2:1 cell
/// aspect so distances are visually round.
pub(crate) fn chamfer(mask: &[Vec<bool>], w: usize, h: usize) -> Vec<Vec<f32>> {
    let big = 1.0e6_f32;
    let (wh, wv, wd) = (1.0_f32, 2.0_f32, 2.236_f32);
    let mut d = vec![vec![big; w]; h];
    for y in 0..h {
        for x in 0..w {
            if mask[y][x] {
                d[y][x] = 0.0;
            }
        }
    }
    for y in 0..h {
        for x in 0..w {
            let mut m = d[y][x];
            if x > 0 {
                m = m.min(d[y][x - 1] + wh);
            }
            if y > 0 {
                m = m.min(d[y - 1][x] + wv);
                if x > 0 {
                    m = m.min(d[y - 1][x - 1] + wd);
                }
                if x + 1 < w {
                    m = m.min(d[y - 1][x + 1] + wd);
                }
            }
            d[y][x] = m;
        }
    }
    for y in (0..h).rev() {
        for x in (0..w).rev() {
            let mut m = d[y][x];
            if x + 1 < w {
                m = m.min(d[y][x + 1] + wh);
            }
            if y + 1 < h {
                m = m.min(d[y + 1][x] + wv);
                if x + 1 < w {
                    m = m.min(d[y + 1][x + 1] + wd);
                }
                if x > 0 {
                    m = m.min(d[y + 1][x - 1] + wd);
                }
            }
            d[y][x] = m;
        }
    }
    d
}


/// Signed distance field: negative inside ink, positive outside, ~0 at the edge.
pub(crate) fn signed_df(g: &Grid, w: usize, h: usize) -> Vec<Vec<f32>> {
    let mut ink = vec![vec![false; w]; h];
    let mut bg = vec![vec![false; w]; h];
    for y in 0..h.min(g.len()) {
        for x in 0..w.min(g[y].len()) {
            let i = morph_is_ink(&g[y][x]);
            ink[y][x] = i;
            bg[y][x] = !i;
        }
    }
    let din = chamfer(&ink, w, h);
    let dbg = chamfer(&bg, w, h);
    let mut s = vec![vec![0.0_f32; w]; h];
    for y in 0..h {
        for x in 0..w {
            s[y][x] = din[y][x] - dbg[y][x];
        }
    }
    s
}


/// Smootherstep easing (6p^5 - 15p^4 + 10p^3): near-zero velocity at both ends,
/// fast through the middle. Gives a pleasant ease-in / ease-out.
pub(crate) fn ease_in_out(p: f32) -> f32 {
    let p = p.clamp(0.0, 1.0);
    p * p * p * (p * (p * 6.0 - 15.0) + 10.0)
}

