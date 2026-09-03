#![allow(warnings)]

use crossterm::style::Color;
use rand::RngExt;
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::io::{self, IsTerminal, Read as _};

use crate::automata::*;
use crate::biomes::*;
use crate::_0_profile::measure_layer;
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
use crate::modes_geo::*;
use crate::modes_sky::*;
use crate::modes_tree::*;
use crate::morph::*;
use crate::opts::*;
use crate::pp::*;
use crate::registry::*;
use crate::warps::*;

/// Box-drawing glyph for a cell given the in/out step directions, plus axis bits
/// (h = horizontal travel, v = vertical travel) used for crossover detection.
pub(crate) fn snake_seg(din: (i32, i32), dout: (i32, i32)) -> (char, bool, bool) {
    let ch = match (din, dout) {
        ((1, 0), (1, 0)) | ((-1, 0), (-1, 0)) => '─',
        ((0, 1), (0, 1)) | ((0, -1), (0, -1)) => '│',
        ((1, 0), (0, 1)) | ((0, -1), (-1, 0)) => '╮',
        ((1, 0), (0, -1)) | ((0, 1), (-1, 0)) => '╯',
        ((-1, 0), (0, 1)) | ((0, -1), (1, 0)) => '╭',
        ((-1, 0), (0, -1)) | ((0, 1), (1, 0)) => '╰',
        _ => '·',
    };
    let h = din.0 != 0 || dout.0 != 0;
    let v = din.1 != 0 || dout.1 != 0;
    (ch, h, v)
}


/// Build a random-walking, wrap-around (toroidal) closed loop for a snake. The
/// walk meanders -- mostly continuing straight, sometimes turning 90 degrees,
/// never reversing -- with an occasional "hop": it picks a fresh direction and
/// shoots `hop_len` cells in a straight line (rendered as solid blocks). A short
/// Manhattan return leg closes it back to the start so the cycle is seamless (the
/// body window slides forever with no teleport). Cells are already wrapped into
/// [0,w) x [0,h); the per-cell step direction and a per-cell "is this a hop block"
/// flag are returned too, so glyphs (and the wrap seam) render correctly.
pub(crate) fn snake_walk(
    w: i32,
    h: i32,
    rng: &mut StdRng,
    turn_prob: f32,
    hop_chance: f32,
    hop_len: i32,
) -> (Vec<(i32, i32)>, Vec<(i32, i32)>, Vec<bool>) {
    if w < 4 || h < 4 {
        return (Vec::new(), Vec::new(), Vec::new());
    }
    let dirs4 = [(1, 0), (-1, 0), (0, 1), (0, -1)];
    let steps = rng.random_range(40..90);
    let mut moves: Vec<(i32, i32)> = Vec::with_capacity(steps as usize + 32);
    let mut block: Vec<bool> = Vec::with_capacity(steps as usize + 32);
    let mut dir = dirs4[rng.random_range(0..4)];
    let mut cooldown = 0i32; // steps before another hop is allowed (no chaining)
    for _ in 0..steps {
        if cooldown > 0 {
            cooldown -= 1;
        }
        // chance to hop: pick a new (non-reversing) direction and shoot a fast
        // displacement that bends once partway, so it reads as a dash rather than a
        // dead-straight ruler. A cooldown keeps hops from chaining into one long bar.
        if cooldown == 0 && hop_chance > 0.0 && hop_len >= 2 && rng.random::<f32>() < hop_chance {
            let rev = (-dir.0, -dir.1);
            loop {
                let d = dirs4[rng.random_range(0..4)];
                if d != rev {
                    dir = d;
                    break;
                }
            }
            // the hop wanders as it goes (each cell may kink 90 deg), so it reads as
            // a short curvy block-dash rather than a dead-straight ruler.
            for _ in 0..hop_len {
                if rng.random::<f32>() < 0.30 {
                    dir = if rng.random::<bool>() {
                        (dir.1, -dir.0)
                    } else {
                        (-dir.1, dir.0)
                    };
                }
                moves.push(dir);
                block.push(true);
            }
            cooldown = hop_len + rng.random_range(3..8);
            continue;
        }
        // mostly go straight (flowing lines); otherwise turn left/right 90 deg.
        if rng.random::<f32>() < turn_prob {
            dir = if rng.random::<bool>() {
                (dir.1, -dir.0) // turn left
            } else {
                (-dir.1, dir.0) // turn right
            };
        }
        moves.push(dir);
        block.push(false);
    }

    // Close the loop: walk straight back to the start cell (net displacement 0).
    // Random-walk drift is ~sqrt(steps), so these return legs stay short.
    let (mut nx, mut ny) = (0i32, 0i32);
    for &(dx, dy) in &moves {
        nx += dx;
        ny += dy;
    }
    while nx > 0 { moves.push((-1, 0)); block.push(false); nx -= 1; }
    while nx < 0 { moves.push((1, 0)); block.push(false); nx += 1; }
    while ny > 0 { moves.push((0, -1)); block.push(false); ny -= 1; }
    while ny < 0 { moves.push((0, 1)); block.push(false); ny += 1; }

    let start = (rng.random_range(0..w), rng.random_range(0..h));
    let (mut cx, mut cy) = start;
    let mut cells = Vec::with_capacity(moves.len());
    let mut dirs = Vec::with_capacity(moves.len());
    for &(dx, dy) in &moves {
        cx = (cx + dx).rem_euclid(w);
        cy = (cy + dy).rem_euclid(h);
        cells.push((cx, cy));
        dirs.push((dx, dy));
    }
    (cells, dirs, block)
}

pub(crate) fn draw_snakes(
    grid: &mut Grid,
    width: usize,
    height: usize,
    _seed: u64,
    palette: &[Color; 5],
    rng: &mut StdRng,
    t: f32,
    snake_count: usize,
) {
    use std::collections::HashMap;
    // faint board dot grid, same as circuit
    measure_layer("snakes", "clear", || {
        for y in 0..height {
            for x in 0..width {
                if x % 4 == 0 && y % 2 == 0 {
                    grid[y][x] = Cell::new('·', darken(palette[1], 90));
                }
            }
        }
    });

    let w = width as i32;
    let h = height as i32;
    let colors = [palette[1], palette[2], palette[3], lighten(palette[2], 25)];

    // Live knobs (demo panel via ASCII_P_*); fall back to the CLI-arg count.
    let count = param_f32("COUNT", snake_count as f32).round().clamp(1.0, 80.0) as usize;
    let turn_prob = param_f32("TURN", 0.35).clamp(0.0, 0.9);
    let speed_base = param_f32("SPEED", 4.0);
    let body_len = param_f32("LEN", 22.0).round().clamp(4.0, 40.0) as usize;
    let hop_chance = param_f32("HOP", 0.0).clamp(0.0, 1.0);
    let hop_coef = param_f32("HOPC", 0.4).clamp(0.2, 1.5);
    let rbow = param_f32("RBOW", 0.0).clamp(0.0, 1.0);
    // hop displacement = body length * coefficient, in cells.
    let hop_len = ((body_len as f32 * hop_coef).round() as i32).max(2);

    struct Snake {
        cells: Vec<(i32, i32)>,
        dirs: Vec<(i32, i32)>,
        block: Vec<bool>,
        color: Color,
        speed: f32,
        phase: f32,
        body: usize,
    }
    let mut snakes: Vec<Snake> = Vec::new();
    let mut attempts = 0;
    measure_layer("snakes", "spawn", || {
        while snakes.len() < count && attempts < count * 8 {
            attempts += 1;
            let (cells, dirs, block) = snake_walk(w, h, rng, turn_prob, hop_chance, hop_len);
            if cells.len() < 8 {
                continue;
            }
            let n = cells.len();
            let base = colors[snakes.len() % colors.len()];
            // rainbow: each snake gets a hue spread around the wheel, blended over the
            // palette color by `rbow` (0 = palette, 1 = full rainbow).
            let color = if rbow > 0.0 {
                let hue = (snakes.len() as f32 / count.max(1) as f32) * 360.0;
                lerp_color(base, hsl_to_rgb(hue as f64, 0.85, 0.55), rbow)
            } else {
                base
            };
            let speed = (speed_base * (0.8 + rng.random::<f32>() * 0.4)).max(0.5);
            let phase = rng.random_range(0.0..n as f32);
            let body = body_len.clamp(4, 40).min(n.saturating_sub(1));
            snakes.push(Snake { cells, dirs, block, color, speed, phase, body });
        }
    });

    // Pass 1: collect every visible body cell. claim = (ch, color, hbit, vbit, head)
    let mut claims: HashMap<(i32, i32), Vec<(char, Color, bool, bool, bool)>> = HashMap::new();
    measure_layer("snakes", "claims", || {
        for s in &snakes {
            let n = s.cells.len() as i32;
            let head = (t * s.speed + s.phase).rem_euclid(n as f32);
            let head_i = head as i32;
            let bl = s.body as i32;
            for k in 0..bl {
                let idx = (head_i - k).rem_euclid(n);
                let (px, py) = s.cells[idx as usize];
                // dir into this cell, and dir out (into the next) -- from stored steps,
                // not coordinate deltas, so the wrap seam stays a continuous line.
                let din = s.dirs[idx as usize];
                let dout = s.dirs[(idx + 1).rem_euclid(n) as usize];
                let (mut ch, hb, vb) = snake_seg(din, dout);
                // hop cells render as solid blocks instead of thin box-drawing lines.
                if s.block[idx as usize] {
                    ch = '\u{2588}'; // full block
                }
                let fade = 1.0 - k as f32 / bl as f32; // 1 at head -> ~0 at tail
                let amt = (25.0 + 70.0 * fade) as u8;
                claims
                    .entry((px, py))
                    .or_default()
                    .push((ch, lighten(s.color, amt), hb, vb, k == 0));
            }
        }
    });

    // Pass 2: resolve. Single claim -> draw it (head -> pad glyph). Multiple claims
    // spanning both axes -> bright crossover knot. Otherwise (parallel overlap) the
    // head, else the first claim, wins.
    let knot_color = lighten(palette[4], 80);
    measure_layer("snakes", "paint", || {
        for ((px, py), v) in claims {
            if px < 0 || py < 0 || px >= w || py >= h {
                continue;
            }
            let (gx, gy) = (px as usize, py as usize);
            if v.len() == 1 {
                let (ch, col, _, _, head) = v[0];
                grid[gy][gx] = Cell::new(if head { '◉' } else { ch }, col);
            } else {
                let has_h = v.iter().any(|e| e.2);
                let has_v = v.iter().any(|e| e.3);
                if has_h && has_v {
                    grid[gy][gx] = Cell::new('╬', knot_color);
                } else {
                    let e = v.iter().find(|e| e.4).unwrap_or(&v[0]);
                    grid[gy][gx] = Cell::new(if e.4 { '◉' } else { e.0 }, e.1);
                }
            }
        }
    });
}

/// Dispatch arm for mode(s): snakes (moved verbatim from run()).
pub(crate) fn cli_snakes(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        // snakes [count] -- PCB traces that slither around hidden loops; where two
        // cross, a bright crossover knot. Native time T (see draw_snakes).
        let snake_count: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(8);
        let snake_count = snake_count.clamp(1, 80);
        draw_snakes(&mut grid, width, height, seed, &palette, &mut rng, t_anim, snake_count);
    (grid, false)
}

/// Dispatch arm for mode(s): ink (moved verbatim from run()).
pub(crate) fn cli_ink(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        let drops: usize = args
            .get(4)
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| param_f32("DROPS", 5.0) as usize);
        let swirl: f32 = args
            .get(5)
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| param_f32("SWIRL", 1.0));
        let speed: f32 = args
            .get(6)
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| param_f32("SPEED", 1.0));
        draw_ink(
            &mut grid, width, height, seed, &palette, &mut rng, t_anim, drops, swirl, speed,
        );
    (grid, false)
}
