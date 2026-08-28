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
use crate::modes_creatures::*;
use crate::modes_geo::*;
use crate::modes_sky::*;
use crate::modes_tree::*;
use crate::morph::*;
use crate::opts::*;
use crate::pp::*;
use crate::registry::*;
use crate::warps::*;

/// Approximate ink density of a glyph, for the `field` strategy.
pub(crate) fn ink_weight(ch: char) -> f32 {
    match ch {
        ' ' => 0.0,
        '·' | '˙' | '\'' | '°' | '`' => 0.15,
        '∙' | ':' | '.' | ',' => 0.28,
        '-' | '─' | '╌' | '│' | '╎' | '|' | '╵' | '╷' => 0.42,
        '+' | '=' | '◦' | '○' | '╱' | '╲' | '╭' | '╮' | '╰' | '╯' => 0.55,
        '*' | '◇' | '△' | '▽' | '□' | '◌' | '✦' | '✧' => 0.68,
        '#' | '◆' | '●' | '◉' | '▪' | '▫' | '◐' | '◑' => 0.82,
        '%' | '▒' | '▓' | '█' | '@' | '❀' | '❁' | '✺' => 0.95,
        _ => 0.6,
    }
}

pub(crate) struct Ink {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) ch: char,
    pub(crate) fg: Color,
}

pub(crate) fn ink_points(g: &Grid) -> Vec<Ink> {
    let mut v = Vec::new();
    for (y, row) in g.iter().enumerate() {
        for (x, c) in row.iter().enumerate() {
            if morph_is_ink(c) {
                v.push(Ink {
                    x: x as f32,
                    y: y as f32,
                    ch: c.ch,
                    fg: rgb_of(c.fg),
                });
            }
        }
    }
    v
}


pub(crate) fn draw_ink(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    rng: &mut StdRng,
    t: f32,
    drops: usize,
    swirl: f32,
    speed: f32,
) {
    use std::f32::consts::TAU;

    if width < 8 || height < 8 {
        return;
    }
    let drops = drops.clamp(1, 9);
    let swirl = swirl.clamp(0.0, 3.0);
    let t = t * speed.clamp(0.05, 4.0);

    // Still water: near-black with a faint vertical light shaft.
    let water_deep = darken(palette[0], 82);
    let water_lit = darken(palette[0], 66);

    // Seeded drop set. Each drop blooms on its own long cycle: the stain swells,
    // tendrils sharpen as it spreads, then the whole thing fades before it
    // re-drops. Phase offsets stagger the drops so the pool never empties.
    struct Drop {
        cx: f32,
        cy: f32,
        rmax: f32,
        col: Color,
        period: f32,
        phase: f32,
        p1: f32,
        p2: f32,
        p3: f32,
        spin: f32,
    }
    let mut drop_list: Vec<Drop> = Vec::new();
    for i in 0..drops {
        drop_list.push(Drop {
            cx: rng.random_range(0.15..0.85) * width as f32,
            cy: rng.random_range(0.18..0.82) * height as f32,
            rmax: rng.random_range(0.16..0.34) * (width as f32).min(height as f32),
            col: [palette[1], palette[2], palette[3]][i % 3],
            period: rng.random_range(9.0..16.0),
            phase: rng.random_range(0.0..1.0),
            p1: rng.random_range(0.0..TAU),
            p2: rng.random_range(0.0..TAU),
            p3: rng.random_range(0.0..TAU),
            spin: if rng.random_bool(0.5) { 1.0 } else { -1.0 },
        });
    }

    for y in 0..height {
        for x in 0..width {
            let fx = x as f32;
            let fy = y as f32;
            let shaft = (fx * 0.02 + 0.5).sin();
            let zbg = lerp_color(water_deep, water_lit, (0.5 + 0.5 * shaft) * 0.35);

            let mut best_d = 0.0f32;
            let mut best_col = zbg;
            for d in &drop_list {
                // Cycle position 0..1; the stain is always present but breathes,
                // peaking mid-cycle, so the pool is never empty.
                let u = (t / d.period + d.phase).fract();
                let env = 0.45 + 0.55 * (0.5 + 0.5 * (u * TAU).sin());
                let r = d.rmax * (0.15 + 0.85 * u);
                let dx = fx - d.cx;
                let dy = (fy - d.cy) * 1.6; // anisotropy: ink spreads wider than deep
                let rad = (dx * dx + dy * dy).sqrt();
                if rad > r * 1.6 {
                    continue;
                }
                // Differential swirl: inner angles rotate faster than outer, so
                // tendrils curl into spirals as the drop ages.
                let ang0 = dy.atan2(dx);
                let ang = ang0 + d.spin * swirl * t * 0.12 * (1.0 / (rad / d.rmax + 0.35));
                let n = 0.62
                    + 0.20 * (ang * 3.0 + d.p1 + t * 0.25 * d.spin).sin()
                    + 0.13 * (ang * 5.0 - d.p2 - t * 0.18 * d.spin).sin()
                    + 0.07 * (ang * 9.0 + d.p3 + t * 0.31 * d.spin).sin();
                let reff = r * n;
                if rad < reff {
                    let dens = (1.0 - rad / reff).min(1.0) * env;
                    if dens > best_d {
                        best_d = dens;
                        best_col = d.col;
                    }
                }
            }

            let (ch, col) = if best_d > 0.92 {
                ('█', lighten(best_col, 16))
            } else if best_d > 0.72 {
                ('▓', lighten(best_col, 8))
            } else if best_d > 0.5 {
                ('░', best_col)
            } else if best_d > 0.3 {
                ('∙', darken(best_col, 14))
            } else if best_d > 0.14 {
                ('·', darken(best_col, 30))
            } else if best_d > 0.04 {
                ('·', darken(best_col, 48))
            } else {
                (' ', zbg)
            };
            grid[y][x] = Cell::with_bg(ch, col, zbg);
        }
    }
}
