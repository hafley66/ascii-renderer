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
use crate::pp::*;
use crate::registry::*;

/// Wind warp: horizontal shear of a single grid, strongest at the top (canopy
/// sways, roots stay put) and oscillating + gusting over `time`. No second frame
/// needed -- this animates one rendered scene "through the wind".
pub(crate) fn warp_wind(src: &Grid, time: f32, amp: f32) -> Grid {
    let h = src.len();
    let w = if h > 0 { src[0].len() } else { 0 };
    let mut g = vec![vec![Cell::blank(); w]; h];
    for y in 0..h {
        // height factor: 1 at the top, 0 at the bottom (squared for a whip feel).
        let hf = if h > 1 { 1.0 - (y as f32 / (h as f32 - 1.0)) } else { 0.0 };
        let gust = 0.5 * (time * 0.37).sin() + 0.5; // 0..1 slow swell
        let sway = amp
            * hf
            * hf
            * (1.0 + gust)
            * ((time + y as f32 * 0.16).sin() + 0.45 * (time * 1.9 + y as f32 * 0.07).sin());
        let dx = sway.round() as i32;
        for x in 0..w {
            let sx = x as i32 - dx;
            if sx >= 0 && (sx as usize) < w {
                g[y][x] = src[y][sx as usize];
            }
        }
    }
    g
}


/// Nearest-cell sample from a source grid (out of bounds -> blank).
pub(crate) fn warp_sample(src: &Grid, sx: f32, sy: f32) -> Cell {
    let xi = sx.round() as i32;
    let yi = sy.round() as i32;
    if xi >= 0 && yi >= 0 && (yi as usize) < src.len() && (xi as usize) < src[0].len() {
        src[yi as usize][xi as usize]
    } else {
        Cell::blank()
    }
}


/// Toroidal drift: scroll the whole grid diagonally over time, wrapping around.
pub(crate) fn warp_drift(src: &Grid, time: f32, amp: f32) -> Grid {
    let h = src.len();
    let w = if h > 0 { src[0].len() } else { 0 };
    let dx = (time * amp).round() as i32;
    let dy = (time * amp * 0.4).round() as i32;
    let mut g = vec![vec![Cell::blank(); w]; h];
    for y in 0..h {
        for x in 0..w {
            let sx = (x as i32 - dx).rem_euclid(w as i32) as usize;
            let sy = (y as i32 - dy).rem_euclid(h as i32) as usize;
            g[y][x] = src[sy][sx];
        }
    }
    g
}


/// Vortex swirl: rotate around the center, faster near the middle, spinning over time.
pub(crate) fn warp_swirl(src: &Grid, time: f32, amp: f32) -> Grid {
    let h = src.len();
    let w = if h > 0 { src[0].len() } else { 0 };
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    let mut g = vec![vec![Cell::blank(); w]; h];
    for y in 0..h {
        for x in 0..w {
            let dx = x as f32 - cx;
            let dy = (y as f32 - cy) * 2.0; // square space
            let r = (dx * dx + dy * dy).sqrt();
            let ang = dy.atan2(dx) - time * amp * 0.3 / (1.0 + r * 0.05);
            let sx = cx + ang.cos() * r;
            let sy = cy + ang.sin() * r / 2.0; // undo aspect
            g[y][x] = warp_sample(src, sx, sy);
        }
    }
    g
}


/// Concentric ripple: radial sine displacement moving outward over time.
pub(crate) fn warp_ripple(src: &Grid, time: f32, amp: f32) -> Grid {
    let h = src.len();
    let w = if h > 0 { src[0].len() } else { 0 };
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    let mut g = vec![vec![Cell::blank(); w]; h];
    for y in 0..h {
        for x in 0..w {
            let dx = x as f32 - cx;
            let dy = (y as f32 - cy) * 2.0;
            let r = (dx * dx + dy * dy).sqrt().max(0.001);
            let off = amp * (r * 0.4 - time * 1.2).sin();
            let nx = dx / r;
            let ny = dy / r;
            g[y][x] = warp_sample(src, x as f32 - nx * off, y as f32 - ny * off / 2.0);
        }
    }
    g
}


/// Breathe: gentle zoom pulse in/out around the center.
pub(crate) fn warp_breathe(src: &Grid, time: f32, amp: f32) -> Grid {
    let h = src.len();
    let w = if h > 0 { src[0].len() } else { 0 };
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    let scale = 1.0 + amp * 0.06 * (time * 0.9).sin();
    let mut g = vec![vec![Cell::blank(); w]; h];
    for y in 0..h {
        for x in 0..w {
            let sx = cx + (x as f32 - cx) / scale;
            let sy = cy + (y as f32 - cy) / scale;
            g[y][x] = warp_sample(src, sx, sy);
        }
    }
    g
}


/// Animated Voronoi: the `stained` tessellation with sites drifting on small
/// orbits over `time`, so the glass cells flow and re-tile continuously. Site
/// base positions/colors are deterministic from `seed`; only the orbit offset
/// moves, so it loops smoothly.
pub(crate) fn voronoi_flow_frame(w: usize, h: usize, seed: u64, time: f32, palette: &[Color; 5]) -> Grid {
    let mut rng = StdRng::seed_from_u64(seed);
    let nseeds = 10 + (seed as usize % 12);
    struct Site {
        bx: f32,
        by: f32,
        ox: f32,
        oy: f32,
        ph: f32,
        sp: f32,
        col: Color,
    }
    let mut sites: Vec<Site> = Vec::with_capacity(nseeds);
    for i in 0..nseeds {
        let bx = rng.random_range(0.0..w.max(1) as f32);
        let by = rng.random_range(0.0..h.max(1) as f32);
        let ox = rng.random_range(3.0..9.0);
        let oy = rng.random_range(1.5..4.5);
        let ph = rng.random_range(0.0..std::f32::consts::TAU);
        let sp = rng.random_range(0.3..0.9) * if rng.random_range(0..2) == 0 { -1.0 } else { 1.0 };
        let base = [palette[1], palette[2], palette[3]][i % 3];
        let col = shift_hue(base, rng.random_range(-90..=90) as f64);
        sites.push(Site { bx, by, ox, oy, ph, sp, col });
    }
    let pos: Vec<(f32, f32, Color)> = sites
        .iter()
        .map(|s| {
            (
                s.bx + (time * s.sp + s.ph).cos() * s.ox,
                s.by + (time * s.sp * 1.3 + s.ph).sin() * s.oy,
                s.col,
            )
        })
        .collect();

    let mut g = vec![vec![Cell::blank(); w]; h];
    let mut id = vec![vec![0usize; w]; h];
    for y in 0..h {
        for x in 0..w {
            let mut best = 0usize;
            let mut bd = f32::MAX;
            for (k, &(sx, sy, _)) in pos.iter().enumerate() {
                let dx = x as f32 - sx;
                let dy = (y as f32 - sy) * 2.0; // cell aspect
                let d = dx * dx + dy * dy;
                if d < bd {
                    bd = d;
                    best = k;
                }
            }
            id[y][x] = best;
            let glass = pos[best].2;
            let ch = if (x + y) % 2 == 0 { '∙' } else { '·' };
            g[y][x] = Cell::new(ch, darken(glass, 8));
        }
    }
    let lead = darken(palette[0], 0);
    for y in 0..h {
        for x in 0..w {
            let here = id[y][x];
            let right = x + 1 < w && id[y][x + 1] != here;
            let down = y + 1 < h && id[y + 1][x] != here;
            if right && down {
                g[y][x] = Cell::new('┼', lead);
            } else if right {
                g[y][x] = Cell::new('│', lead);
            } else if down {
                g[y][x] = Cell::new('─', lead);
            }
        }
    }
    for &(sx, sy, col) in &pos {
        pp_put(&mut g, sx.round() as i32, sy.round() as i32, '◆', lighten(col, 40));
    }
    g
}

