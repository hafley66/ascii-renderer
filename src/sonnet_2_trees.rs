//! sonnet-2-trees: six growth algorithms (spire, krummholz, strangler, windrake,
//! bracket, cypress) plus the sample sheet mode.

use crate::_0_profile::measure_layer;
use crate::color::*;
use crate::opts::param_f32;
use crate::types::*;
use crossterm::style::Color;
use rand::RngExt;
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::cell::RefCell;
use std::collections::HashMap;
use std::f32::consts::TAU;

// ---------------------------------------------------------------- species

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Species {
    Spire,
    Krummholz,
    Strangler,
    Windrake,
    Bracket,
    Cypress,
}

pub(crate) const SPECIES: [Species; 6] = [
    Species::Spire,
    Species::Krummholz,
    Species::Strangler,
    Species::Windrake,
    Species::Bracket,
    Species::Cypress,
];

impl Species {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Species::Spire => "Spire",
            Species::Krummholz => "Krummholz",
            Species::Strangler => "Strangler",
            Species::Windrake => "Windrake",
            Species::Bracket => "Bracket",
            Species::Cypress => "Cypress",
        }
    }
}

/// Eight colors a species draws with.
#[derive(Clone, Copy)]
pub(crate) struct Ink {
    pub trunk: Color,
    pub bark: Color,
    pub branch: Color,
    pub leaf: Color,
    pub leaf2: Color,
    pub fruit: Color,
    pub root: Color,
    pub accent: Color,
}

impl Ink {
    pub(crate) fn from_hue(hue: f64, sat: f64, light: f64) -> Ink {
        let h = |d: f64| (hue + d).rem_euclid(360.0);
        Ink {
            trunk: hsl_to_rgb(h(-25.0), sat * 0.6, light * 0.85),
            bark: hsl_to_rgb(h(-32.0), sat * 0.5, light * 0.6),
            branch: hsl_to_rgb(h(-10.0), sat * 0.7, light),
            leaf: hsl_to_rgb(h(20.0), sat, (light + 0.12).min(0.62)),
            leaf2: hsl_to_rgb(h(5.0), sat * 0.9, light * 0.8),
            fruit: hsl_to_rgb(h(150.0), (sat * 1.3).min(0.9), (light + 0.2).min(0.64)),
            root: hsl_to_rgb(h(-35.0), sat * 0.45, light * 0.58),
            accent: hsl_to_rgb(h(80.0), sat * 0.35, (light + 0.18).min(0.7)),
        }
    }

    /// Aerial perspective: pull every color toward `toward` by `k`.
    pub(crate) fn fade(&self, toward: Color, k: f32) -> Ink {
        let f = |c: Color| lerp_color(c, toward, k);
        Ink {
            trunk: f(self.trunk),
            bark: f(self.bark),
            branch: f(self.branch),
            leaf: f(self.leaf),
            leaf2: f(self.leaf2),
            fruit: f(self.fruit),
            root: f(self.root),
            accent: f(self.accent),
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct GrowKnobs {
    pub fruit: f32,
    pub branch: f32,
    pub detail: f32,
    pub roots: f32,
}

// ---------------------------------------------------------------- pen helpers

fn rf(rng: &mut StdRng) -> f32 {
    rng.random::<f32>()
}

fn put(grid: &mut Grid, x: i32, y: i32, ch: char, c: Color) {
    if y >= 0 && x >= 0 && (y as usize) < grid.len() && (x as usize) < grid[y as usize].len() {
        grid[y as usize][x as usize] = Cell::new(ch, c);
    }
}

fn is_blank(grid: &Grid, x: i32, y: i32) -> bool {
    y >= 0
        && x >= 0
        && (y as usize) < grid.len()
        && (x as usize) < grid[y as usize].len()
        && grid[y as usize][x as usize].ch == ' '
}

fn put_blank(grid: &mut Grid, x: i32, y: i32, ch: char, c: Color) {
    if is_blank(grid, x, y) {
        grid[y as usize][x as usize] = Cell::new(ch, c);
    }
}

pub(crate) fn hash3(a: u32, b: u32, c: u32) -> u32 {
    let mut h = a.wrapping_mul(0x9E37_79B9) ^ b.wrapping_mul(0x85EB_CA6B) ^ c.wrapping_mul(0xC2B2_AE35);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    h
}

/// Discretize a heading (radians, 0 = up, clockwise) into one of 8 octants.
fn octant(heading: f32) -> i32 {
    let h = heading.rem_euclid(TAU);
    ((h / (std::f32::consts::PI / 4.0)).round() as i32).rem_euclid(8)
}

const OCT_GLYPH: [char; 8] = ['│', '╱', '─', '╲', '│', '╱', '─', '╲'];

fn dir_glyph(heading: f32) -> char {
    OCT_GLYPH[octant(heading) as usize]
}

/// Corner glyph at a branch split: parent heading `ph`, child heading `chd`.
fn split_glyph(ph: f32, chd: f32) -> char {
    let pv = (ph.sin(), -ph.cos());
    let cv = (chd.sin(), -chd.cos());
    let horiz_p = pv.1.abs() < 0.42;
    let horiz_c = cv.1.abs() < 0.42;
    if horiz_p && !horiz_c {
        if pv.0 >= 0.0 { '├' } else { '┤' }
    } else if !horiz_p && horiz_c {
        let up = pv.1 < 0.0;
        let right = cv.0 >= 0.0;
        match (up, right) {
            (true, true) => '╰',
            (true, false) => '╯',
            (false, true) => '╭',
            (false, false) => '╮',
        }
    } else {
        '┼'
    }
}

/// Straight-line stepper between two points, glyph chosen per-step octant so the
/// stroke reads as trunk/limb regardless of slope.
fn seg(grid: &mut Grid, x0: i32, y0: i32, x1: i32, y1: i32, c: Color, thick: bool) {
    let dx = x1 - x0;
    let dy = y1 - y0;
    let n = dx.abs().max(dy.abs()).max(1);
    let (mut px, mut py) = (x0, y0);
    for i in 1..=n {
        let x = x0 + (dx * i + n / 2).div_euclid(n);
        let y = y0 + (dy * i + n / 2).div_euclid(n);
        if x == px && y == py {
            continue;
        }
        let heading = (px as f32 - x as f32).atan2(py as f32 - y as f32) + std::f32::consts::PI;
        let ch = if thick && dx == 0 { '┃' } else { dir_glyph(heading) };
        put(grid, x, y, ch, c);
        px = x;
        py = y;
    }
}

/// Multi-column tapering trunk: half-width shrinks with height; wide rows get a
/// bark-fill interior, narrow rows are a bare edge stroke.
fn taper_trunk(grid: &mut Grid, x0: i32, y0: i32, rows: i32, base_half: i32, lean: f32, wobble: f32, ink: &Ink, rng: &mut StdRng) -> Vec<(i32, i32)> {
    let mut pts = Vec::with_capacity(rows.max(1) as usize);
    let mut xf = x0 as f32;
    let mut x = x0;
    for i in 0..rows.max(1) {
        let y = y0 - i;
        xf += lean + (rf(rng) - 0.5) * wobble;
        let nx = xf.round() as i32;
        let prog = i as f32 / rows.max(1) as f32;
        let half = ((base_half as f32) * (1.0 - prog * 0.78)).round() as i32;
        for d in 1..=half {
            let edge = d == half;
            let ch = if !edge && half >= 3 { if (nx + d + y) % 5 == 0 { '▓' } else { '▒' } } else { '│' };
            put(grid, nx - d, y, ch, ink.bark);
            put(grid, nx + d, y, ch, ink.bark);
        }
        let ch = if i == 0 || nx == x { if half >= 2 { '┃' } else { '│' } } else if nx > x { '╱' } else { '╲' };
        put(grid, nx, y, ch, ink.trunk);
        x = nx;
        pts.push((nx, y));
    }
    pts
}

/// Elliptical leaf cluster; density thins toward the rim. Never covers structure.
fn leaf_blob(grid: &mut Grid, cx: i32, cy: i32, rx: i32, ry: i32, density: f32, glyphs: [char; 3], ink: &Ink, rng: &mut StdRng) {
    let rx = rx.max(1);
    let ry = ry.max(1);
    for dy in -ry..=ry {
        for dx in -rx..=rx {
            let d = (dx as f32 / (rx as f32 + 0.5)).powi(2) + (dy as f32 / (ry as f32 + 0.5)).powi(2);
            if d > 1.0 {
                continue;
            }
            if rf(rng) < density * (1.0 - d * 0.75) {
                let ch = if d < 0.3 { glyphs[0] } else if d < 0.65 { glyphs[1] } else { glyphs[2] };
                let c = if d < 0.5 { ink.leaf } else { ink.leaf2 };
                put_blank(grid, cx + dx, cy + dy, ch, c);
            }
        }
    }
}

fn root_of(plot: Rect) -> (i32, i32) {
    (plot.x as i32 + plot.w as i32 / 2, plot.y as i32 + plot.h as i32 - 1)
}

struct Frame {
    rx: i32,
    ry: i32,
    th: f32,
    half: f32,
}

fn frame_of(plot: Rect, energy: f32) -> Frame {
    let (rx, ry) = root_of(plot);
    let e = energy.clamp(0.2, 1.3);
    Frame {
        rx,
        ry,
        th: ((plot.h as f32 - 1.0) * e).max(4.0),
        half: (plot.w as f32 / 2.0 * e).max(2.0),
    }
}

/// Two to four roots fanning down from the root row; `bias` skews the fan
/// toward one side (used by wind-driven species). `bias == 0` is symmetric.
fn root_fan(grid: &mut Grid, rx: i32, ry: i32, spread: i32, depth: i32, bias: f32, color: Color, rng: &mut StdRng) {
    put(grid, rx - 1, ry, '╱', color);
    put(grid, rx + 1, ry, '╲', color);
    if depth <= 0 {
        return;
    }
    let n = 2 + rng.random_range(0..3u32) as i32;
    let max_step = (spread / depth.max(1)).clamp(1, 3);
    for k in 0..n {
        let mut side = if k % 2 == 0 { -1 } else { 1 };
        if rf(rng) < 0.25 + bias.abs() * 0.3 {
            side = if bias >= 0.0 { 1 } else { -1 };
        }
        let len = 1 + rng.random_range(0..depth.max(1) as u32) as i32;
        let (mut x, mut y) = (rx + side, ry);
        for s in 0..len {
            y += 1;
            let step = rng.random_range(0..(max_step + 1) as u32) as i32;
            let c = darken(color, (s * 10).min(60) as u8);
            if step == 0 {
                put(grid, x, y, '│', c);
            } else {
                for j in 1..step {
                    put(grid, x + side * j, y, '─', c);
                }
                x += side * step;
                put(grid, x, y, if side > 0 { '╲' } else { '╱' }, c);
            }
        }
        put(grid, x + side, y, '·', darken(color, 40));
    }
}

fn hue_of(c: Color) -> f64 {
    match c {
        Color::Rgb { r, g, b } => {
            let (rf_, gf, bf) = (r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0);
            let max = rf_.max(gf).max(bf);
            let min = rf_.min(gf).min(bf);
            let d = max - min;
            if d < 0.001 {
                return 120.0;
            }
            let h = if (max - rf_).abs() < 0.001 {
                ((gf - bf) / d + if gf < bf { 6.0 } else { 0.0 }) * 60.0
            } else if (max - gf).abs() < 0.001 {
                ((bf - rf_) / d + 2.0) * 60.0
            } else {
                ((rf_ - gf) / d + 4.0) * 60.0
            };
            h.rem_euclid(360.0)
        }
        _ => 120.0,
    }
}

pub(crate) fn palette_hue(palette: &[Color; 5]) -> f64 {
    hue_of(palette[1])
}

// ---------------------------------------------------------------- grow entry

/// Grow one tree. `plot` is the above-ground box whose bottom row is the root row;
/// `root_depth` rows below it are free for roots.
pub(crate) fn grow_species(kind: Species, grid: &mut Grid, plot: Rect, root_depth: i32, energy: f32, ink: &Ink, k: &GrowKnobs, rng: &mut StdRng) {
    if plot.w < 3 || plot.h < 3 {
        return;
    }
    match kind {
        Species::Spire => grow_spire(grid, plot, root_depth, energy, ink, k, rng),
        Species::Krummholz => grow_krummholz(grid, plot, root_depth, energy, ink, k, rng),
        Species::Strangler => grow_strangler(grid, plot, root_depth, energy, ink, k, rng),
        Species::Windrake => grow_windrake(grid, plot, root_depth, energy, ink, k, rng),
        Species::Bracket => grow_bracket(grid, plot, root_depth, energy, ink, k, rng),
        Species::Cypress => grow_cypress(grid, plot, root_depth, energy, ink, k, rng),
    }
}

// ---------------------------------------------------------------- 1. spire (L-system whorled conifer)

/// Rewrite `C` (a live growth tip) into a leader run with a pair of side
/// branches. Terminal `C`s left after the last iteration are needle stamps.
fn spire_expand(iters: u32) -> String {
    let mut s = String::from("C");
    for _ in 0..iters {
        let mut next = String::with_capacity(s.len() * 3);
        for ch in s.chars() {
            match ch {
                'C' => next.push_str("FFF[+C]FFF[-C]FFFC"),
                other => next.push(other),
            }
        }
        s = next;
    }
    s
}

fn needle_stamp(grid: &mut Grid, x: f32, y: f32, heading: f32, ink: &Ink, depth: i32, rng: &mut StdRng) {
    let glyphs = ['▪', '▫', '▲'];
    let n = (4 - depth.min(3)).max(1);
    for i in 0..n {
        let spread = (rf(rng) - 0.5) * 1.6;
        let d = 0.4 + rf(rng) * 0.8;
        let ang = heading + spread;
        let px = x + ang.sin() * d;
        let py = y - ang.cos() * d * 0.6;
        let density = (0.85 - depth as f32 * 0.12).max(0.15);
        if rf(rng) < density {
            let ch = glyphs[i as usize % glyphs.len()];
            let c = if i == 0 { ink.leaf } else { ink.leaf2 };
            put_blank(grid, px.round() as i32, py.round() as i32, ch, c);
        }
    }
}

fn grow_spire(grid: &mut Grid, plot: Rect, root_depth: i32, energy: f32, ink: &Ink, k: &GrowKnobs, rng: &mut StdRng) {
    let f = frame_of(plot, energy);
    let iters = 2 + (f.th > 10.0) as u32 + (k.detail > 1.3) as u32;
    let grammar = spire_expand(iters.min(4));
    let budget = 4000u32;
    let branch_angle = 0.44 + rf(rng) * 0.26;
    let seed_lean = (rf(rng) - 0.5) * 0.12;

    let mut x = f.rx as f32;
    let mut y = f.ry as f32;
    let mut heading: f32 = seed_lean;
    let mut depth: i32 = 0;
    let mut stack: Vec<(f32, f32, f32, i32)> = Vec::new();
    let mut pending_from: Option<f32> = None;
    let mut moves = 0u32;
    let mut leader_moves = 0u32;
    let floor = f.ry as f32;
    let (px0, px1, py0) = (plot.x as f32, (plot.x + plot.w) as f32, plot.y as f32);
    let in_plot = |px: f32, py: f32| px >= px0 && px < px1 && py >= py0 && py <= floor;

    for ch in grammar.chars() {
        match ch {
            'F' => {
                moves += 1;
                if moves > budget {
                    break;
                }
                if depth == 0 {
                    leader_moves += 1;
                }
                let nx = x + heading.sin();
                let ny = y - heading.cos();
                if in_plot(nx, ny) {
                    let thick = depth == 0 && (leader_moves as f32) < f.th * 0.42;
                    let glyph = dir_glyph(heading);
                    let color = if depth == 0 { ink.trunk } else { darken(ink.branch, (depth as u8 * 6).min(50)) };
                    put(grid, nx.round() as i32, ny.round() as i32, glyph, color);
                    if thick {
                        let ix = nx.round() as i32;
                        let iy = ny.round() as i32;
                        put(grid, ix - 1, iy, '│', ink.bark);
                        put(grid, ix + 1, iy, '│', ink.bark);
                        if hash3(ix as u32, iy as u32, moves) % 4 == 0 {
                            put(grid, ix - 1, iy, '▓', darken(ink.bark, 10));
                        }
                    }
                }
                x = nx;
                y = ny;
            }
            '+' => {
                heading += branch_angle * (0.85 + rf(rng) * 0.3);
                if let Some(from) = pending_from.take() {
                    put(grid, x.round() as i32, y.round() as i32, split_glyph(from, heading), ink.branch);
                }
            }
            '-' => {
                heading -= branch_angle * (0.85 + rf(rng) * 0.3);
                if let Some(from) = pending_from.take() {
                    put(grid, x.round() as i32, y.round() as i32, split_glyph(from, heading), ink.branch);
                }
            }
            '[' => {
                stack.push((x, y, heading, depth));
                pending_from = Some(heading);
                depth += 1;
            }
            ']' => {
                if depth >= 1 && rf(rng) < (0.95 / depth as f32).max(0.45) {
                    needle_stamp(grid, x, y, heading, ink, depth, rng);
                    if rf(rng) < k.fruit * 0.3 {
                        put_blank(grid, x.round() as i32, (y + 1.0).round() as i32, '◆', darken(ink.fruit, 10));
                    }
                }
                if let Some((sx, sy, sh, sd)) = stack.pop() {
                    x = sx;
                    y = sy;
                    heading = sh;
                    depth = sd;
                }
            }
            'C' => needle_stamp(grid, x, y, heading, ink, depth.max(1), rng),
            _ => {}
        }
    }
    root_fan(grid, f.rx, f.ry, (f.half * k.roots) as i32, ((root_depth as f32) * k.roots) as i32, 0.0, ink.root, rng);
}

// ---------------------------------------------------------------- 2. krummholz (wind-flagged dwarf, novel)

fn grow_krummholz(grid: &mut Grid, plot: Rect, root_depth: i32, energy: f32, ink: &Ink, k: &GrowKnobs, rng: &mut StdRng) {
    let f = frame_of(plot, (energy * 0.62).max(0.25));
    let wind_dir: f32 = if rf(rng) < 0.5 { -1.0 } else { 1.0 };
    let wind_strength = 0.5 + rf(rng) * 0.4;
    let n_stems = 2 + (rf(rng) * 3.0) as i32;
    let mut cells: Vec<(f32, f32, f32)> = Vec::new();

    for s in 0..n_stems {
        let start_x = f.rx as f32 + (s - n_stems / 2) as f32 * (f.half * 0.4);
        let mut x = start_x.clamp(plot.x as f32 + 1.0, (plot.x + plot.w) as f32 - 2.0);
        let mut y = f.ry as f32;
        let mut heading: f32 = wind_dir * (0.12 + rf(rng) * 0.1);
        let stem_len = ((f.th * (0.55 + 0.4 * rf(rng))) as i32).max(3);
        for i in 0..stem_len {
            heading += wind_dir * 0.1 * wind_strength + (rf(rng) - 0.5) * 0.05;
            if i > stem_len * 8 / 10 {
                heading -= wind_dir * 0.09;
            }
            heading = heading.clamp(-1.3, 1.3);
            let nx = x + heading.sin();
            let ny = y - heading.cos();
            let thick = i < stem_len / 3;
            let glyph = dir_glyph(heading);
            put(grid, nx.round() as i32, ny.round() as i32, glyph, ink.trunk);
            if thick && hash3(nx as u32, ny as u32, s as u32) % 6 == 0 {
                put(grid, nx.round() as i32, ny.round() as i32, '▓', darken(ink.bark, 15));
            }
            x = nx;
            y = ny;
            cells.push((x, y, heading));
        }
    }

    // needle mat compressed leeward: bare on the windward flank, dense downwind.
    for &(x, y, heading) in &cells {
        let wind_align = wind_dir * heading.sin();
        let density = (0.12 + 0.85 * wind_align.max(0.0)) * k.branch.clamp(0.3, 1.5);
        let n = 2 + (density * 5.0) as i32;
        for _ in 0..n {
            if rf(rng) >= density {
                continue;
            }
            let ang = heading + wind_dir * rf(rng) * 0.55;
            let d = 0.6 + rf(rng) * 1.3;
            let lx = x + ang.sin() * d;
            let ly = y - ang.cos() * d * 0.5 - rf(rng) * 0.4;
            let ch = if d < 1.0 { '▪' } else { '∙' };
            put_blank(grid, lx.round() as i32, ly.round() as i32, ch, if wind_align > 0.5 { ink.leaf } else { ink.leaf2 });
        }
        if rf(rng) < k.fruit * 0.15 {
            put_blank(grid, x.round() as i32, (y - 1.0).round() as i32, '▫', ink.fruit);
        }
    }
    root_fan(grid, f.rx, f.ry, (f.half * k.roots) as i32, ((root_depth as f32) * k.roots) as i32, -wind_dir, ink.root, rng);
}

// ---------------------------------------------------------------- 3. strangler (fig lattice on a dead host, novel)

fn grow_strangler(grid: &mut Grid, plot: Rect, root_depth: i32, energy: f32, ink: &Ink, k: &GrowKnobs, rng: &mut StdRng) {
    let f = frame_of(plot, energy);
    let host_h = ((f.th * (0.68 + 0.22 * rf(rng))) as i32).max(3);
    let lean = (rf(rng) - 0.5) * 0.12;
    let mut xf = f.rx as f32;
    let mut host_pts: Vec<(i32, i32)> = Vec::with_capacity(host_h as usize);
    for i in 0..host_h {
        xf += lean;
        let y = f.ry - i;
        put(grid, xf.round() as i32, y, if i == 0 { '┃' } else { '│' }, ink.accent);
        host_pts.push((xf.round() as i32, y));
    }
    let &(hx, hy) = host_pts.last().unwrap();
    put(grid, hx, hy - 1, if rf(rng) < 0.5 { '╱' } else { '╲' }, darken(ink.accent, 20));

    let n_strands = 4 + (k.branch * 4.0) as i32;
    let mut occupied: HashMap<i32, Vec<i32>> = HashMap::new();
    for _ in 0..n_strands {
        let phase = rf(rng) * TAU;
        let freq = 0.3 + rf(rng) * 0.3;
        let amp_top = f.half * (0.45 + 0.35 * rf(rng));
        let mut prev: Option<(i32, i32)> = None;
        for row in 0..=host_h {
            let y = f.ry - row;
            let u = row as f32 / host_h.max(1) as f32;
            let amp = amp_top * u.powf(1.5);
            let base_x = host_pts.get(row as usize).map(|p| p.0).unwrap_or(f.rx);
            let sx = base_x as f32 + amp * (freq * row as f32 * 0.3 + phase).sin();
            let x = sx.round() as i32;
            let fused = occupied.get(&y).map(|v| v.contains(&x)).unwrap_or(false);
            let color = lerp_color(ink.branch, ink.leaf2, (u * 0.8).min(1.0));
            if fused {
                put(grid, x, y, '╳', darken(ink.root, 10));
            } else if let Some((px, py)) = prev {
                seg(grid, px, py, x, y, color, u < 0.3);
            }
            occupied.entry(y).or_default().push(x);
            prev = Some((x, y));
        }
    }

    // base fuses into a root mass over the lower host and ground.
    let mass_rows = ((host_h as f32 * 0.22) as i32).max(1);
    for i in 0..mass_rows {
        let y = f.ry - i;
        let hw = (2 + i / 2).max(1);
        for d in -hw..=hw {
            let ch = if d.abs() == hw { if hw >= 3 { '▓' } else { '▒' } } else { '░' };
            put(grid, f.rx + d, y, ch, darken(ink.root, (i * 6) as u8));
        }
    }

    // canopy fed by the strands, propagules hanging below.
    let cr = (f.half * 0.5).max(2.0) as i32;
    leaf_blob(grid, hx, hy - 1, cr, (cr / 2).max(1), 0.65, ['●', '•', '∙'], ink, rng);
    for _ in 0..(3 + (k.fruit * 5.0) as i32) {
        let dx = (rf(rng) - 0.5) * cr as f32 * 1.6;
        let px = hx + dx as i32;
        if rf(rng) < k.fruit {
            put_blank(grid, px, hy, '╷', ink.fruit);
            put_blank(grid, px, hy + 1, '•', ink.fruit);
        }
    }
    root_fan(grid, f.rx, f.ry, (f.half * k.roots) as i32, ((root_depth as f32) * k.roots) as i32, 0.0, ink.root, rng);
}

// ---------------------------------------------------------------- 4. windrake (radial angle-sweep fan)

fn grow_windrake(grid: &mut Grid, plot: Rect, root_depth: i32, energy: f32, ink: &Ink, k: &GrowKnobs, rng: &mut StdRng) {
    let f = frame_of(plot, energy);
    let trunk_h = ((f.th * (0.3 + 0.1 * rf(rng))) as i32).max(2);
    let tw = ((f.th / 12.0) as i32).max(1);
    let trunk = taper_trunk(grid, f.rx, f.ry, trunk_h, tw, (rf(rng) - 0.5) * 0.45, 0.55, ink, rng);
    let &(tx, ty) = trunk.last().unwrap();

    let wind_side: f32 = if rf(rng) < 0.5 { -1.0 } else { 1.0 };
    let base_angle = wind_side * (0.95 + rf(rng) * 0.5);
    let sweep = 0.7 + rf(rng) * 0.7;
    let n_rays = 5 + (k.branch * 5.0) as i32;
    let crown_r = ((f.th - trunk_h as f32).max(3.0)) * (0.65 + 0.35 * rf(rng));
    let joint = split_glyph(0.0, base_angle);
    put(grid, tx, ty, joint, ink.branch);

    for i in 0..n_rays {
        let frac = if n_rays > 1 { i as f32 / (n_rays - 1) as f32 } else { 0.5 };
        let ang = base_angle - wind_side * sweep * 0.5 + wind_side * sweep * frac + (rf(rng) - 0.5) * 0.1;
        let len = crown_r * (0.55 + 0.45 * rf(rng));
        let ex = tx as f32 + ang.sin() * len;
        let ey = ty as f32 - ang.cos() * len;
        seg(grid, tx, ty, ex.round() as i32, ey.round() as i32, ink.branch, true);

        if rf(rng) < k.branch * 0.5 {
            let fu = 0.55 + rf(rng) * 0.2;
            let mx = tx as f32 + ang.sin() * len * fu;
            let my = ty as f32 - ang.cos() * len * fu;
            let fork_ang = ang + (rf(rng) - 0.5) * 0.7;
            let flen = len * 0.4;
            let fx = mx + fork_ang.sin() * flen;
            let fy = my - fork_ang.cos() * flen;
            seg(grid, mx.round() as i32, my.round() as i32, fx.round() as i32, fy.round() as i32, ink.branch, false);
            if rf(rng) < 0.5 {
                put_blank(grid, fx.round() as i32, (fy - 1.0).round() as i32, '▪', ink.leaf2);
            }
        }

        let steps = (len as i32).max(2);
        for s in 0..steps {
            let d = s as f32;
            let density = (1.0 - d / len).max(0.0).powf(1.3) * 0.8;
            if rf(rng) >= density {
                continue;
            }
            let px = tx as f32 + ang.sin() * d;
            let py = ty as f32 - ang.cos() * d;
            let ch = if density > 0.5 { '▪' } else if density > 0.25 { '▫' } else { '∙' };
            let c = if density > 0.5 { ink.leaf } else { ink.leaf2 };
            put_blank(grid, px.round() as i32, py.round() as i32, ch, c);
        }
        if rf(rng) < k.fruit * 0.2 {
            put_blank(grid, ex.round() as i32, ey.round() as i32, '◆', ink.fruit);
        }
    }
    root_fan(grid, f.rx, f.ry, (f.half * k.roots) as i32, ((root_depth as f32) * k.roots) as i32, -wind_side * 0.5, ink.root, rng);
}

// ---------------------------------------------------------------- 5. bracket (fungal shelf stack on a snag)

fn cap_glyph(dy: i32, hgt: i32) -> char {
    if hgt <= 0 {
        return '▤';
    }
    let f = dy as f32 / hgt as f32;
    if f < 0.4 { '◍' } else if f < 0.8 { '▤' } else { '░' }
}

fn grow_bracket(grid: &mut Grid, plot: Rect, root_depth: i32, energy: f32, ink: &Ink, k: &GrowKnobs, rng: &mut StdRng) {
    let f = frame_of(plot, energy);
    let snag_h = ((f.th * (0.55 + 0.35 * rf(rng))) as i32).max(4);
    let tw = ((f.th / 11.0) as i32).max(1);
    let trunk = taper_trunk(grid, f.rx, f.ry, snag_h, tw, (rf(rng) - 0.5) * 0.1, 0.2, ink, rng);
    let &(tx, ty) = trunk.last().unwrap();
    put(grid, tx, ty - 1, if rf(rng) < 0.5 { '╱' } else { '╲' }, darken(ink.bark, 15));

    let n_brackets = (4.0 + k.detail * 6.0) as i32;
    let usable = (trunk.len() as i32 - 3).max(1);
    let mut used_rows: Vec<i32> = Vec::new();
    for i in 0..n_brackets {
        let mut idx = 1 + (rf(rng) * usable as f32) as i32;
        let mut tries = 0;
        while used_rows.iter().any(|&r| (r - idx).abs() < 2) && tries < 6 {
            idx = 1 + (rf(rng) * usable as f32) as i32;
            tries += 1;
        }
        used_rows.push(idx);
        let (bx, by) = trunk[(idx as usize).min(trunk.len() - 1)];
        let side: i32 = if i % 2 == 0 { 1 } else { -1 };
        let age_frac = 1.0 - idx as f32 / trunk.len().max(1) as f32;
        let w = (2.0 + rf(rng) * 3.0) as i32;
        let w = ((w as f32) * (0.6 + 0.8 * age_frac)).max(1.0) as i32;
        for dx in 0..=w {
            let hgt = (((w - dx) as f32 / w.max(1) as f32) * 2.0).round() as i32;
            for dy in 0..=hgt {
                let ch = cap_glyph(dy, hgt);
                let c = lerp_color(ink.accent, ink.bark, dy as f32 / (hgt.max(1) as f32));
                put(grid, bx + side * (dx + 1), by - dy, ch, c);
            }
        }
        put(grid, bx + side, by, if side > 0 { '├' } else { '┤' }, ink.bark);
        if rf(rng) < k.fruit {
            put_blank(grid, bx + side * (w + 1), by + 1, '✶', ink.fruit);
        }
    }

    for (i, &(x, y)) in trunk.iter().enumerate() {
        let age = 1.0 - i as f32 / trunk.len().max(1) as f32;
        if rf(rng) < 0.15 * age {
            put_blank(grid, x, y, '▒', darken(ink.leaf2, 10));
        }
    }
    // spreading mycelium threads instead of a root flare.
    let n = 3 + (rf(rng) * 3.0) as i32;
    for i in 0..n {
        let side = if i % 2 == 0 { -1 } else { 1 };
        let len = 1 + rng.random_range(0..(root_depth.max(1) as u32 + 2));
        let (mut x, mut y) = (f.rx, f.ry);
        for s in 0..len {
            y += 1;
            x += side * (s as i32 % 2);
            put(grid, x, y, if s % 2 == 0 { '╌' } else { '·' }, darken(ink.root, (s * 8) as u8));
        }
    }
}

// ---------------------------------------------------------------- 6. cypress (buttressed trunk, root knees)

fn grow_cypress(grid: &mut Grid, plot: Rect, root_depth: i32, energy: f32, ink: &Ink, k: &GrowKnobs, rng: &mut StdRng) {
    let f = frame_of(plot, energy);
    let base_flare = f.half * (0.45 + 0.3 * rf(rng));
    let trunk_h = ((f.th * (0.62 + 0.15 * rf(rng))) as i32).max(4);
    for i in 0..trunk_h {
        let y = f.ry - i;
        let prog = i as f32 / trunk_h as f32;
        let hw = (1.0 + base_flare * (1.0 - prog).powf(2.6)).round() as i32;
        for d in 0..=hw {
            let edge = d == hw;
            let ch = if edge {
                '│'
            } else if hw >= 4 && (f.rx + d + y) % 4 == 0 {
                '▓'
            } else if d == 0 {
                if hw >= 2 { '┃' } else { '│' }
            } else {
                '▒'
            };
            let c = if d == 0 { ink.trunk } else { ink.bark };
            put(grid, f.rx - d, y, ch, c);
            put(grid, f.rx + d, y, ch, c);
        }
    }
    let (tx, ty) = (f.rx, f.ry - trunk_h);

    // root knees around the base, outside the buttress footprint.
    let n_knees = 3 + (k.roots * 5.0) as i32;
    for i in 0..n_knees {
        let ang = (i as f32 / n_knees.max(1) as f32) * TAU + rf(rng) * 0.4;
        let r = base_flare * (1.1 + rf(rng) * 0.5);
        let kx = f.rx + (ang.cos() * r * 0.8) as i32;
        let ky = f.ry + (rf(rng) * (root_depth as f32 * 0.5).max(0.5)) as i32;
        let h = 1 + rng.random_range(0..3u32) as i32;
        for dy in 0..h {
            put(grid, kx, ky - dy, if dy == h - 1 { '╷' } else { '│' }, ink.root);
        }
        if rf(rng) < 0.5 {
            put(grid, kx, ky - h, '·', darken(ink.root, 20));
        }
    }

    // flat-topped canopy, density thinning toward the rim.
    let cr = (f.half * (0.8 + 0.3 * rf(rng))).max(2.0) as i32;
    let ch = ((f.th - trunk_h as f32).max(2.0) as i32 / 2).max(1);
    leaf_blob(grid, tx, ty, cr, ch, 0.75, ['▓', '▒', '░'], ink, rng);
    leaf_blob(grid, tx, ty - 1, (cr as f32 * 0.7) as i32, (ch as f32 * 0.6).max(1.0) as i32, 0.5, ['●', '•', '∙'], ink, rng);
    if rf(rng) < k.fruit {
        put_blank(grid, tx, ty - ch - 1, '◆', ink.fruit);
    }
    root_fan(grid, f.rx, f.ry, (f.half * k.roots) as i32, ((root_depth as f32) * k.roots) as i32, 0.0, ink.root, rng);
}

// ---------------------------------------------------------------- sprites

/// Interned color table shared by every sprite in a scene.
pub(crate) struct Palette {
    pub colors: Vec<Color>,
    index: HashMap<Color, u16>,
}

impl Palette {
    pub(crate) fn new() -> Self {
        Palette { colors: Vec::new(), index: HashMap::new() }
    }

    pub(crate) fn intern(&mut self, c: Color) -> u16 {
        if let Some(&i) = self.index.get(&c) {
            return i;
        }
        let i = self.colors.len() as u16;
        self.colors.push(c);
        self.index.insert(c, i);
        i
    }
}

#[derive(Clone, Copy)]
pub(crate) struct SpriteCell {
    pub dx: u16,
    pub ch: char,
    pub slot: u16,
    pub leaf: bool,
}

/// One grown tree as row lists of cells; `root_row` is the row that sits on the ground.
pub(crate) struct Sprite {
    pub root_row: usize,
    pub rows: Vec<Vec<SpriteCell>>,
    pub spans: Vec<Option<(u16, u16)>>,
}

fn is_leaf_glyph(ch: char) -> bool {
    matches!(ch, '●' | '•' | '∙' | '·' | '◆' | '◇' | '○' | '◦' | '▪' | '▫' | '▓' | '▒' | '░')
}

fn leaf_alt(ch: char) -> char {
    match ch {
        '●' => '•',
        '•' => '●',
        '∙' => '·',
        '·' => '∙',
        '◆' => '◇',
        '◇' => '◆',
        '○' => '◦',
        '◦' => '○',
        '▪' => '▫',
        '▫' => '▪',
        '▓' => '▒',
        '▒' => '░',
        '░' => '▒',
        other => other,
    }
}

pub(crate) fn sprite_from_grid(scratch: &Grid, root_row: usize, pal: &mut Palette) -> Sprite {
    let rows: Vec<Vec<SpriteCell>> = scratch
        .iter()
        .map(|row| {
            row.iter()
                .enumerate()
                .filter(|(_, c)| c.ch != ' ')
                .map(|(x, c)| SpriteCell { dx: x as u16, ch: c.ch, slot: pal.intern(c.fg), leaf: is_leaf_glyph(c.ch) })
                .collect()
        })
        .collect();
    let spans = rows
        .iter()
        .map(|row| {
            let (Some(a), Some(b)) = (row.first(), row.last()) else { return None };
            let span = (b.dx - a.dx + 1) as usize;
            if row.len() >= 3 && row.len() * 10 >= span * 3 { Some((a.dx, b.dx)) } else { None }
        })
        .collect();
    Sprite { root_row, rows, spans }
}

/// Paint a sprite with its root row at (x0, y0 + root_row). Rows above the root
/// shear by `sway` scaled with height squared; leaf glyphs flicker by `flicker`.
pub(crate) fn blit_sprite(grid: &mut Grid, gw: usize, gh: usize, sp: &Sprite, x0: i32, y0: i32, sway: f32, flicker: f32, tick: u32, lit: &[Color], mask: Option<Cell>) {
    let rr = sp.root_row.max(1) as f32;
    let fl = (flicker * 300.0) as u32;
    for (dy, row) in sp.rows.iter().enumerate() {
        if row.is_empty() {
            continue;
        }
        let y = y0 + dy as i32;
        if y < 0 || y >= gh as i32 {
            continue;
        }
        let above = (sp.root_row as i32 - dy as i32).max(0) as f32 / rr;
        let base = x0 + (sway * above * above).round() as i32;
        let line = &mut grid[y as usize];
        if let (Some(m), Some((a, b))) = (mask, sp.spans[dy]) {
            let xa = (base + a as i32).clamp(0, gw as i32) as usize;
            let xb = (base + b as i32 + 1).clamp(0, gw as i32) as usize;
            line[xa..xb].fill(m);
        }
        for c in row {
            let x = base + c.dx as i32;
            if x < 0 || x >= gw as i32 {
                continue;
            }
            let mut ch = c.ch;
            if c.leaf && fl > 0 && hash3(c.dx as u32, dy as u32, tick) % 1000 < fl {
                ch = leaf_alt(ch);
            }
            line[x as usize] = Cell::new(ch, lit[c.slot as usize]);
        }
    }
}

// ---------------------------------------------------------------- sample sheet mode

pub(crate) struct SheetKnobs {
    pub energy: f32,
    pub fruit: f32,
    pub branch: f32,
    pub sway: f32,
    pub speed: f32,
    pub flicker: f32,
    pub detail: f32,
    pub roots: f32,
}

impl SheetKnobs {
    pub(crate) fn from_env() -> Self {
        SheetKnobs {
            energy: param_f32("ENERGY", 0.9).clamp(0.3, 1.2),
            fruit: param_f32("FRUIT", 0.25).clamp(0.0, 1.0),
            branch: param_f32("BRANCH", 1.0).clamp(0.3, 1.5),
            sway: param_f32("SWAY", 0.5).clamp(0.0, 2.0),
            speed: param_f32("SPEED", 1.0).clamp(0.2, 3.0),
            flicker: param_f32("FLICKER", 0.5).clamp(0.0, 1.0),
            detail: param_f32("DETAIL", 1.0).clamp(0.4, 2.0),
            roots: param_f32("ROOTS", 1.0).clamp(0.0, 1.0),
        }
    }
}

#[derive(PartialEq, Clone)]
struct SheetKey {
    w: usize,
    h: usize,
    seed: u64,
    palette: [Color; 5],
    energy: f32,
    fruit: f32,
    branch: f32,
    detail: f32,
    roots: f32,
}

struct Placed {
    sprite: Sprite,
    x: i32,
    y: i32,
    phase: f32,
}

struct Sheet {
    key: SheetKey,
    pal: Palette,
    trees: Vec<Placed>,
    labels: Vec<(String, i32, i32, u16)>,
    ground: Vec<(i32, i32, i32, u16)>,
}

thread_local! {
    static SHEET: RefCell<Option<Sheet>> = const { RefCell::new(None) };
}

fn build_sheet(key: SheetKey, k: &SheetKnobs) -> Sheet {
    let gw = key.w;
    let gh = key.h;
    let cols = SPECIES.len();
    let cell_w = (gw / cols).max(4);
    let cell_h = (gh / 2).max(12);
    let base_hue = palette_hue(&key.palette);
    let mut pal = Palette::new();
    let mut trees = Vec::new();
    let mut labels = Vec::new();
    let mut ground = Vec::new();
    let gk = GrowKnobs { fruit: k.fruit, branch: k.branch, detail: k.detail, roots: k.roots };
    for row in 0..2usize {
        let energy = if row == 0 { k.energy } else { k.energy * 0.6 };
        for (i, &sp) in SPECIES.iter().enumerate() {
            let px = (i * cell_w) as i32;
            let py = (row * cell_h) as i32;
            let rd = ((cell_h / 8) as i32).clamp(1, 6);
            let label_y = py + cell_h as i32 - 1;
            let gy = label_y - 1 - rd;
            let plot_h = (gy - py).max(3) as usize;
            let plot_w = cell_w.saturating_sub(2).max(3);
            let mut scratch = vec![vec![Cell::blank(); plot_w]; plot_h + rd as usize];
            let mut rng = StdRng::seed_from_u64(key.seed ^ hash3(i as u32 + 1, row as u32 + 1, 0x50EE) as u64);
            let hue = (base_hue + i as f64 * 26.0 + row as f64 * 30.0 - 30.0).rem_euclid(360.0);
            let ink = Ink::from_hue(hue, 0.55, 0.4);
            let plot = Rect { x: 0, y: 0, w: plot_w, h: plot_h };
            grow_species(sp, &mut scratch, plot, rd, energy, &ink, &gk, &mut rng);
            let sprite = sprite_from_grid(&scratch, plot_h - 1, &mut pal);
            let phase = rf(&mut rng) * TAU;
            trees.push(Placed { sprite, x: px + 1, y: py + 1, phase });
            let dim = pal.intern(darken(ink.bark, 10));
            ground.push((gy, px + 1, px + cell_w as i32 - 2, dim));
            let label = sp.label().to_string();
            let lx = px + cell_w as i32 / 2 - label.len() as i32 / 2;
            let ls = pal.intern(lighten(ink.leaf, 30));
            labels.push((label, lx, label_y, ls));
        }
    }
    Sheet { key, pal, trees, labels, ground }
}

pub(crate) fn draw_sonnet_2_trees(grid: &mut Grid, width: usize, height: usize, seed: u64, palette: &[Color; 5], t: f32, k: &SheetKnobs) {
    let gh = height.min(grid.len());
    let gw = width.min(grid.first().map(|r| r.len()).unwrap_or(0));
    if gw < 4 || gh < 4 {
        return;
    }
    measure_layer("sonnet-2-trees", "clear", || {
        for row in grid.iter_mut().take(gh) {
            row[..gw].fill(Cell::blank());
        }
    });
    let key = SheetKey { w: gw, h: gh, seed, palette: *palette, energy: k.energy, fruit: k.fruit, branch: k.branch, detail: k.detail, roots: k.roots };
    let hit = SHEET.with(|c| c.borrow().as_ref().map(|s| s.key == key).unwrap_or(false));
    if !hit {
        let sheet = measure_layer("sonnet-2-trees", "grow", || build_sheet(key.clone(), k));
        SHEET.with(|c| *c.borrow_mut() = Some(sheet));
    }
    SHEET.with(|c| {
        let b = c.borrow();
        let s = b.as_ref().unwrap();
        let lit = &s.pal.colors;
        measure_layer("sonnet-2-trees", "ground", || {
            for &(y, x0, x1, slot) in &s.ground {
                if y < 0 || y >= gh as i32 {
                    continue;
                }
                let line = &mut grid[y as usize];
                for x in x0.max(0)..=x1.min(gw as i32 - 1) {
                    line[x as usize] = Cell::new('─', lit[slot as usize]);
                }
            }
            for (label, lx, ly, slot) in &s.labels {
                if *ly < 0 || *ly >= gh as i32 {
                    continue;
                }
                for (j, ch) in label.chars().enumerate() {
                    let x = lx + j as i32;
                    if x >= 0 && x < gw as i32 {
                        grid[*ly as usize][x as usize] = Cell::new(ch, lit[*slot as usize]);
                    }
                }
            }
        });
        measure_layer("sonnet-2-trees", "trees", || {
            let animating = t > 0.0;
            let tick = (t * 3.0) as u32;
            for p in &s.trees {
                let sway = if animating {
                    k.sway * p.sprite.root_row as f32 * 0.08 * (TAU * t * k.speed / 24.0 + p.phase).sin()
                } else {
                    0.0
                };
                let flicker = if animating { k.flicker } else { 0.0 };
                blit_sprite(grid, gw, gh, &p.sprite, p.x, p.y, sway, flicker, tick, lit, None);
            }
        });
    });
}

pub(crate) fn cli_sonnet_2_trees(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], _rng: StdRng, t_anim: f32, _term_w: u16, _term_h: u16, args: &[String], _mode: &str, _theme_name: &str) -> (Grid, bool) {
    let mut k = SheetKnobs::from_env();
    let slots: [&mut f32; 8] = [&mut k.energy, &mut k.fruit, &mut k.branch, &mut k.sway, &mut k.speed, &mut k.flicker, &mut k.detail, &mut k.roots];
    for (i, slot) in slots.into_iter().enumerate() {
        if let Some(v) = args.get(4 + i).and_then(|s| s.parse().ok()) {
            *slot = v;
        }
    }
    draw_sonnet_2_trees(&mut grid, width, height, seed, &palette, t_anim, &k);
    (grid, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(w: usize, h: usize, seed: u64, t: f32) -> String {
        let mut g = vec![vec![Cell::blank(); w]; h];
        let p = crate::color::make_palette(seed);
        let k = SheetKnobs::from_env();
        draw_sonnet_2_trees(&mut g, w, h, seed, &p, t, &k);
        g.iter().map(|row| row.iter().map(|c| c.ch).collect::<String>()).collect::<Vec<_>>().join("\n")
    }

    #[test]
    fn snapshot_sonnet_2_trees_static() {
        insta::assert_snapshot!("sonnet_2_trees_80x24_static", run(80, 24, 42, 0.0));
    }

    #[test]
    fn deterministic_and_seed_sensitive() {
        assert_eq!(run(80, 24, 42, 0.0), run(80, 24, 42, 0.0));
        assert_ne!(run(80, 24, 42, 0.0), run(80, 24, 7, 0.0));
    }

    #[test]
    fn every_species_draws_and_differs_by_seed() {
        for &sp in &SPECIES {
            let mut outs = Vec::new();
            for seed in [1u64, 2, 3] {
                let mut g = vec![vec![Cell::blank(); 40]; 24];
                let mut rng = StdRng::seed_from_u64(seed);
                let ink = Ink::from_hue(120.0, 0.5, 0.4);
                let k = GrowKnobs { fruit: 0.2, branch: 1.0, detail: 1.0, roots: 1.0 };
                grow_species(sp, &mut g, Rect { x: 0, y: 0, w: 40, h: 20 }, 3, 0.9, &ink, &k, &mut rng);
                let s: String = g.iter().map(|r| r.iter().map(|c| c.ch).collect::<String>()).collect();
                assert!(s.chars().filter(|c| *c != ' ').count() > 15, "{:?} drew nothing", sp);
                outs.push(s);
            }
            assert_ne!(outs[0], outs[1], "{:?} identical across seeds", sp);
        }
    }
}
