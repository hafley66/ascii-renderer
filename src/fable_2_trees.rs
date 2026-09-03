//! fable-2-trees: five growth algorithms of my own plus the sample sheet mode.
//! Species draw straight into a Grid; sprites cache them for sway and flicker.

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
use std::f32::consts::{PI, TAU};

// ---------------------------------------------------------------- species

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Species {
    Colonizer,
    Banyan,
    Mangrove,
    Coral,
    Sunburst,
}

pub(crate) const SPECIES: [Species; 5] = [
    Species::Colonizer,
    Species::Banyan,
    Species::Mangrove,
    Species::Coral,
    Species::Sunburst,
];

impl Species {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Species::Colonizer => "Colonizer",
            Species::Banyan => "Banyan",
            Species::Mangrove => "Mangrove",
            Species::Coral => "Coral",
            Species::Sunburst => "Sunburst",
        }
    }
}

/// Seven colors a species draws with.
#[derive(Clone, Copy)]
pub(crate) struct Ink {
    pub trunk: Color,
    pub bark: Color,
    pub branch: Color,
    pub leaf: Color,
    pub leaf2: Color,
    pub fruit: Color,
    pub root: Color,
}

impl Ink {
    pub(crate) fn from_hue(hue: f64, sat: f64, light: f64) -> Ink {
        let h = |d: f64| (hue + d).rem_euclid(360.0);
        Ink {
            trunk: hsl_to_rgb(h(-25.0), sat * 0.6, light * 0.85),
            bark: hsl_to_rgb(h(-30.0), sat * 0.5, light * 0.62),
            branch: hsl_to_rgb(h(-10.0), sat * 0.7, light),
            leaf: hsl_to_rgb(h(20.0), sat, (light + 0.12).min(0.62)),
            leaf2: hsl_to_rgb(h(5.0), sat * 0.9, light * 0.8),
            fruit: hsl_to_rgb(h(140.0), (sat * 1.3).min(0.9), (light + 0.2).min(0.64)),
            root: hsl_to_rgb(h(-35.0), sat * 0.45, light * 0.6),
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

fn step_glyph(dx: i32, dy: i32, thick: bool) -> char {
    if dx == 0 {
        if thick { '┃' } else { '│' }
    } else if dy == 0 {
        '─'
    } else if (dx > 0) == (dy < 0) {
        '╱'
    } else {
        '╲'
    }
}

/// Draw the cells after (x0, y0) up to and including (x1, y1), glyph per step.
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
        put(grid, x, y, step_glyph(x - px, y - py, thick), c);
        px = x;
        py = y;
    }
}

fn root_of(plot: Rect) -> (i32, i32) {
    (plot.x as i32 + plot.w as i32 / 2, plot.y as i32 + plot.h as i32 - 1)
}

/// Wobbling trunk of `rows` cells rising from (x0, y0); returns the centerline bottom to top.
fn trunk_walk(grid: &mut Grid, x0: i32, y0: i32, rows: i32, lean: f32, wobble: f32, tw: i32, ink: &Ink, rng: &mut StdRng) -> Vec<(i32, i32)> {
    let mut pts = Vec::with_capacity(rows.max(1) as usize);
    let mut xf = x0 as f32;
    let mut x = x0;
    for i in 0..rows.max(1) {
        let y = y0 - i;
        xf += lean + (rf(rng) - 0.5) * wobble;
        let nx = xf.round() as i32;
        let half = (((tw as f32) * (1.0 - i as f32 / rows.max(1) as f32 * 0.6)) as i32 / 2).max(0);
        for d in 1..=half {
            put(grid, nx - d, y, '│', ink.bark);
            put(grid, nx + d, y, '│', ink.bark);
        }
        let ch = if i == 0 || nx == x {
            if half >= 1 { '┃' } else { '│' }
        } else if nx > x {
            '╱'
        } else {
            '╲'
        };
        put(grid, nx, y, ch, ink.trunk);
        x = nx;
        pts.push((nx, y));
    }
    pts
}

/// Elliptical leaf cluster; glyphs ordered core, mid, rim. Never covers structure.
fn leaf_blob(grid: &mut Grid, cx: i32, cy: i32, rx: i32, ry: i32, density: f32, glyphs: [char; 3], ink: &Ink, rng: &mut StdRng) {
    let rx = rx.max(1);
    let ry = ry.max(1);
    for dy in -ry..=ry {
        for dx in -rx..=rx {
            let d = (dx as f32 / (rx as f32 + 0.5)).powi(2) + (dy as f32 / (ry as f32 + 0.5)).powi(2);
            if d > 1.0 {
                continue;
            }
            if rf(rng) < density * (1.0 - d * 0.7) {
                let ch = if d < 0.3 { glyphs[0] } else if d < 0.65 { glyphs[1] } else { glyphs[2] };
                let c = if d < 0.5 { ink.leaf } else { ink.leaf2 };
                put_blank(grid, cx + dx, cy + dy, ch, c);
            }
        }
    }
}

/// Two to four roots fanning down from the root row, plus a flare on the row itself.
pub(crate) fn root_fan(grid: &mut Grid, rx: i32, ry: i32, spread: i32, depth: i32, color: Color, rng: &mut StdRng) {
    put(grid, rx - 1, ry, '╱', color);
    put(grid, rx + 1, ry, '╲', color);
    if depth <= 0 {
        return;
    }
    let n = 2 + rng.random_range(0..3u32) as i32;
    let max_step = (spread / depth.max(1)).clamp(1, 3);
    for k in 0..n {
        let mut side = if k % 2 == 0 { -1 } else { 1 };
        if rf(rng) < 0.25 {
            side = -side;
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
        Species::Colonizer => grow_colonizer(grid, plot, root_depth, energy, ink, k, rng),
        Species::Banyan => grow_banyan(grid, plot, root_depth, energy, ink, k, rng),
        Species::Mangrove => grow_mangrove(grid, plot, root_depth, energy, ink, k, rng),
        Species::Coral => grow_coral(grid, plot, root_depth, energy, ink, k, rng),
        Species::Sunburst => grow_sunburst(grid, plot, root_depth, energy, ink, k, rng),
    }
}

struct Frame {
    rx: i32,
    ry: i32,
    th: f32,
    half: f32,
}

fn frame_of(plot: Rect, energy: f32) -> Frame {
    let (rx, ry) = root_of(plot);
    let e = energy.clamp(0.25, 1.3);
    Frame {
        rx,
        ry,
        th: ((plot.h as f32 - 1.0) * e).max(4.0),
        half: (plot.w as f32 / 2.0 * e).max(2.0),
    }
}

fn leaf_set(th: f32) -> [char; 3] {
    if th < 9.0 { ['•', '∙', '·'] } else { ['●', '•', '∙'] }
}

// ---------------------------------------------------------------- 1. colonizer (space colonization)

struct Node {
    vx: f32,
    vy: f32,
    parent: i32,
}

fn grow_colonizer(grid: &mut Grid, plot: Rect, root_depth: i32, energy: f32, ink: &Ink, k: &GrowKnobs, rng: &mut StdRng) {
    let f = frame_of(plot, energy);
    let trunk_h = ((f.th * (0.26 + 0.12 * rf(rng))) as i32).max(2);
    let tw = ((f.th / 14.0) as i32).max(1);
    let lean = (rf(rng) - 0.5) * 0.3;
    let trunk = trunk_walk(grid, f.rx, f.ry, trunk_h, lean, 0.35, tw, ink, rng);
    let &(tx, ty) = trunk.last().unwrap();
    let crown_h = (f.th - trunk_h as f32).max(3.0);
    let skew = (rf(rng) - 0.5) * 0.8;
    let ex = tx as f32 + skew * f.half * 0.5;
    let ey = ty as f32 - crown_h * 0.5;
    let erx = f.half * (0.8 + 0.2 * rf(rng));
    let ery = crown_h * 0.55;
    let px0 = plot.x as f32;
    let px1 = (plot.x + plot.w) as f32;
    let py0 = plot.y as f32;

    // attractors live in visual units: one column is half a row wide
    let n_att = ((k.detail * 90.0) as usize).clamp(30, 240);
    let mut att: Vec<(f32, f32, bool)> = Vec::with_capacity(n_att);
    let mut tries = 0;
    while att.len() < n_att && tries < n_att * 20 {
        tries += 1;
        let u = rf(rng) * 2.0 - 1.0;
        let v = rf(rng) * 2.0 - 1.0;
        if u * u + v * v > 1.0 {
            continue;
        }
        let x = ex + u * erx;
        let y = ey + v * ery;
        if x < px0 || x >= px1 || y < py0 || y >= ty as f32 {
            continue;
        }
        att.push((x * 0.5, y, true));
    }

    let mut nodes: Vec<Node> = Vec::new();
    let seed_from = trunk.len().saturating_sub((trunk_h as usize / 3).max(1));
    for &(x, y) in trunk.iter().skip(seed_from) {
        nodes.push(Node { vx: x as f32 * 0.5, vy: y as f32, parent: -1 });
    }
    let infl = (crown_h * 0.4).max(3.0);
    let kill = (crown_h * 0.08).max(1.3);
    let step = (crown_h / 22.0).max(1.0);
    for a in att.iter_mut() {
        if nodes.iter().any(|n| (n.vx - a.0).hypot(n.vy - a.1) < kill) {
            a.2 = false;
        }
    }
    let mut acc: Vec<(f32, f32, u32)> = Vec::new();
    for _ in 0..70 {
        acc.clear();
        acc.resize(nodes.len(), (0.0, 0.0, 0));
        let mut any = false;
        for a in att.iter().filter(|a| a.2) {
            let mut best = -1i32;
            let mut bd = infl;
            for (i, n) in nodes.iter().enumerate() {
                let d = (n.vx - a.0).hypot(n.vy - a.1);
                if d < bd {
                    bd = d;
                    best = i as i32;
                }
            }
            if best >= 0 {
                let n = &nodes[best as usize];
                let d = bd.max(0.001);
                let e = &mut acc[best as usize];
                e.0 += (a.0 - n.vx) / d;
                e.1 += (a.1 - n.vy) / d;
                e.2 += 1;
                any = true;
            }
        }
        if !any {
            break;
        }
        let start = nodes.len();
        for i in 0..start {
            if acc[i].2 == 0 {
                continue;
            }
            let dx = acc[i].0 + (rf(rng) - 0.5) * 0.3;
            let dy = acc[i].1 - 0.12 * acc[i].2 as f32 + (rf(rng) - 0.5) * 0.3;
            let l = dx.hypot(dy).max(0.001);
            let nx = nodes[i].vx + dx / l * step;
            let ny = nodes[i].vy + dy / l * step;
            if nx * 2.0 < px0 || nx * 2.0 >= px1 || ny < py0 || ny > f.ry as f32 {
                continue;
            }
            nodes.push(Node { vx: nx, vy: ny, parent: i as i32 });
        }
        if nodes.len() == start {
            break;
        }
        for a in att.iter_mut().filter(|a| a.2) {
            if nodes[start..].iter().any(|n| (n.vx - a.0).hypot(n.vy - a.1) < kill) {
                a.2 = false;
            }
        }
    }

    let mut size = vec![1u32; nodes.len()];
    for i in (0..nodes.len()).rev() {
        let p = nodes[i].parent;
        if p >= 0 {
            size[p as usize] += size[i];
        }
    }
    let maxsize = size.iter().copied().max().unwrap_or(1) as f32;
    let mut depth = vec![0u16; nodes.len()];
    for i in 0..nodes.len() {
        let p = nodes[i].parent;
        if p >= 0 {
            depth[i] = depth[p as usize] + 1;
        }
    }
    let maxd = depth.iter().copied().max().unwrap_or(1).max(1) as f32;
    for i in 0..nodes.len() {
        let p = nodes[i].parent;
        if p < 0 {
            continue;
        }
        let a = &nodes[p as usize];
        let b = &nodes[i];
        let c = lerp_color(ink.branch, ink.leaf2, depth[i] as f32 / maxd * 0.7);
        let thick = size[i] as f32 > maxsize * 0.3;
        seg(grid, (a.vx * 2.0).round() as i32, a.vy.round() as i32, (b.vx * 2.0).round() as i32, b.vy.round() as i32, c, thick);
    }
    let lb = ((crown_h * 0.18) as i32).max(1);
    for i in 0..nodes.len() {
        if size[i] != 1 {
            continue;
        }
        let x = (nodes[i].vx * 2.0).round() as i32;
        let y = nodes[i].vy.round() as i32;
        leaf_blob(grid, x, y, lb * 2, lb, 0.6, ['●', '•', '∙'], ink, rng);
        if rf(rng) < k.fruit {
            put(grid, x, y + 1, '●', ink.fruit);
        }
    }
    for a in &att {
        if !a.2 && rf(rng) < 0.6 {
            put_blank(grid, (a.0 * 2.0).round() as i32, a.1.round() as i32, '·', ink.leaf2);
        }
    }
    root_fan(grid, f.rx, f.ry, (f.half * k.roots) as i32, ((root_depth as f32) * k.roots) as i32, ink.root, rng);
}

// ---------------------------------------------------------------- 2. banyan (limbs + prop roots)

fn grow_limb(grid: &mut Grid, x0: i32, y0: i32, side: i32, len: i32, depth: i32, k: &GrowKnobs, ink: &Ink, rng: &mut StdRng, cells: &mut Vec<(i32, i32)>) {
    let (mut x, mut y) = (x0, y0);
    let rise_p = 0.16 + 0.12 * depth as f32;
    let mut run = 0;
    for i in 0..len {
        x += side;
        if run > 0 && rf(rng) < rise_p {
            y -= 1;
            put(grid, x, y, if side > 0 { '╱' } else { '╲' }, ink.branch);
            run = 0;
        } else {
            put(grid, x, y, '─', ink.branch);
            run += 1;
        }
        cells.push((x, y));
        if depth < 2 && i > len / 3 && rf(rng) < k.branch * 0.10 {
            let sub = (len - i) * 3 / 5;
            if sub >= 3 {
                grow_limb(grid, x, y, side, sub, depth + 1, k, ink, rng, cells);
            }
        }
    }
    put(grid, x + side, y, if side > 0 { '╶' } else { '╴' }, ink.branch);
}

fn grow_banyan(grid: &mut Grid, plot: Rect, root_depth: i32, energy: f32, ink: &Ink, k: &GrowKnobs, rng: &mut StdRng) {
    let f = frame_of(plot, energy);
    let trunk_h = ((f.th * (0.34 + 0.12 * rf(rng))) as i32).max(2);
    let tw = ((f.th / 9.0) as i32).max(1);
    let trunk = trunk_walk(grid, f.rx, f.ry, trunk_h, (rf(rng) - 0.5) * 0.1, 0.15, tw, ink, rng);
    for s in [-1, 1] {
        if rf(rng) < 0.6 {
            let off = tw / 2 + 1;
            let hgt = (trunk_h as f32 * (0.3 + 0.4 * rf(rng))) as i32;
            for i in 0..hgt {
                put_blank(grid, f.rx + s * off, f.ry - i, '│', ink.bark);
            }
        }
    }
    let &(tx, ty) = trunk.last().unwrap();
    let n_limbs = 3 + (rf(rng) * 3.0) as i32;
    let p_left = 0.3 + rf(rng) * 0.4;
    let mut cells: Vec<(i32, i32)> = Vec::new();
    for _ in 0..n_limbs {
        let side = if rf(rng) < p_left { -1 } else { 1 };
        let start = trunk.len() - 1 - (rf(rng) * trunk.len() as f32 * 0.35) as usize;
        let (sx, sy) = trunk[start];
        let len = ((f.half * (0.5 + 0.5 * rf(rng))) as i32).max(2);
        grow_limb(grid, sx + side * (tw / 2), sy, side, len, 0, k, ink, rng, &mut cells);
    }

    // prop roots dropping from the limbs, some reaching the ground
    let gap = ((f.half / 4.0) as i32).max(3);
    let mut dropped: Vec<i32> = Vec::new();
    for (i, &(x, y)) in cells.iter().enumerate() {
        if (x - f.rx).abs() < tw / 2 + 2 || i % gap as usize != 0 || rf(rng) > k.roots * 0.8 {
            continue;
        }
        if dropped.iter().any(|&d| (d - x).abs() < gap - 1) {
            continue;
        }
        dropped.push(x);
        let reach = rf(rng) < 0.6;
        let end_y = if reach { f.ry } else { y + ((f.ry - y) as f32 * (0.3 + 0.5 * rf(rng))) as i32 };
        for yy in (y + 1)..=end_y {
            put_blank(grid, x, yy, '│', ink.root);
        }
        if reach {
            put(grid, x, f.ry, '│', ink.root);
            put(grid, x - 1, f.ry, '╱', ink.root);
            put(grid, x + 1, f.ry, '╲', ink.root);
            if root_depth > 0 {
                put(grid, x, f.ry + 1, '│', darken(ink.root, 20));
            }
        } else {
            put(grid, x, end_y, '╷', lighten(ink.root, 20));
        }
    }

    // canopy dome riding on every limb cell
    let ct = (f.th * 0.18).max(2.0);
    let phase = rf(rng) * TAU;
    let f1 = 0.15 + rf(rng) * 0.15;
    for &(x, y) in &cells {
        let nz = 0.5 + 0.5 * ((x as f32 * f1 + phase).sin() * 0.6 + (x as f32 * 0.37 + phase * 2.0).sin() * 0.4);
        let lh = ((ct * (0.7 + 0.5 * nz)) as i32).max(1);
        for dy in 1..=lh {
            let fr = dy as f32 / lh as f32;
            if rf(rng) < 0.95 * (1.0 - fr).powf(0.4) + 0.15 {
                let ch = if fr < 0.35 { '●' } else if fr < 0.7 { '•' } else if fr < 0.9 { '∙' } else { '·' };
                let c = if fr < 0.5 { ink.leaf } else { ink.leaf2 };
                put_blank(grid, x, y - dy, ch, c);
            }
        }
        if rf(rng) < 0.45 {
            put_blank(grid, x, y + 1, '∙', ink.leaf2);
        }
        if rf(rng) < k.fruit * 0.3 {
            put_blank(grid, x, y + 1, '•', ink.fruit);
        }
    }
    leaf_blob(grid, tx, ty - 1, ((f.half * 0.35) as i32).max(2), ((ct * 0.5) as i32).max(1), 0.7, ['●', '•', '∙'], ink, rng);
    for d in 1..=(tw / 2 + 1) {
        put(grid, f.rx - tw / 2 - d, f.ry, '╱', ink.root);
        put(grid, f.rx + tw / 2 + d, f.ry, '╲', ink.root);
    }
    root_fan(grid, f.rx, f.ry, (f.half * k.roots) as i32, ((root_depth as f32) * k.roots) as i32, ink.root, rng);
}

// ---------------------------------------------------------------- 3. mangrove (stilt roots + turtle branches)

fn turtle_branch(grid: &mut Grid, vx: f32, vy: f32, ang: f32, len: f32, depth: i32, k: &GrowKnobs, ink: &Ink, rng: &mut StdRng, tips: &mut Vec<(i32, i32)>) {
    let (mut x, mut y, mut a) = (vx, vy, ang);
    let (mut px, mut py) = ((vx * 2.0).round() as i32, vy.round() as i32);
    let n = len.max(1.0) as i32;
    for i in 0..n {
        a += (rf(rng) - 0.5) * 0.3;
        x += a.sin();
        y -= a.cos();
        let (cx, cy) = ((x * 2.0).round() as i32, y.round() as i32);
        if cx != px || cy != py {
            seg(grid, px, py, cx, cy, ink.branch, false);
            px = cx;
            py = cy;
        }
        if depth < 2 && i > n / 3 && rf(rng) < k.branch * 0.15 {
            let side = if rf(rng) < 0.5 { -1.0 } else { 1.0 };
            turtle_branch(grid, x, y, a + side * (0.4 + rf(rng) * 0.5), len * 0.55, depth + 1, k, ink, rng, tips);
        }
        if i % 3 == 2 && rf(rng) < 0.35 {
            put_blank(grid, cx + if a > 0.0 { -1 } else { 1 }, cy - 1, '∙', ink.leaf2);
        }
    }
    tips.push((px, py));
}

fn grow_mangrove(grid: &mut Grid, plot: Rect, root_depth: i32, energy: f32, ink: &Ink, k: &GrowKnobs, rng: &mut StdRng) {
    let f = frame_of(plot, energy);
    let stilt_h = ((f.th * 0.26) as i32).max(2);
    let base_y = f.ry - stilt_h;
    let base_x = f.rx;
    let n_roots = 4 + (rf(rng) * 4.0) as i32;
    let bias = if rf(rng) < 0.5 { -1.0 } else { 1.0 };
    for i in 0..n_roots {
        let mut side = if i % 2 == 0 { -1.0 } else { 1.0 };
        if rf(rng) < 0.2 {
            side = bias;
        }
        let reach = f.half * k.roots.max(0.2) * (0.3 + 0.65 * rf(rng)) * side;
        let below = ((rf(rng) * (root_depth as f32 + 0.99)) as i32).clamp(0, root_depth.max(0));
        let steps = stilt_h + below;
        let (mut px, mut py) = (base_x, base_y);
        for s in 1..=steps {
            let u = s as f32 / steps as f32;
            let x = (base_x as f32 + reach * (1.0 - (1.0 - u) * (1.0 - u))).round() as i32;
            let y = base_y + s;
            let c = if y <= f.ry { ink.root } else { darken(ink.root, 25) };
            let dx = x - px;
            if dx.abs() >= 2 {
                let sg = dx.signum();
                for j in 1..dx.abs() {
                    put(grid, px + sg * j, py, '─', c);
                }
                put(grid, x, y, if dx > 0 { '╲' } else { '╱' }, c);
            } else if dx == 0 {
                put(grid, x, y, '│', c);
            } else {
                put(grid, x, y, if dx > 0 { '╲' } else { '╱' }, c);
            }
            px = x;
            py = y;
        }
        put(grid, px + side as i32, py, '·', darken(ink.root, 35));
    }
    let top_y = f.ry - ((f.th * (0.58 + 0.08 * rf(rng))) as i32);
    let rows = (base_y - top_y).max(2);
    let tw = ((f.th / 16.0) as i32).max(1);
    let trunk = trunk_walk(grid, base_x, base_y, rows, (rf(rng) - 0.5) * 0.25, 0.5, tw, ink, rng);
    let n_br = (2.0 + k.branch * 3.0 + rf(rng) * 2.0) as i32;
    let mut tips: Vec<(i32, i32)> = Vec::new();
    for _ in 0..n_br {
        let idx = trunk.len() - 1 - (rf(rng) * trunk.len() as f32 * 0.45) as usize;
        let (sx, sy) = trunk[idx];
        let side = if rf(rng) < 0.5 { -1.0 } else { 1.0 };
        let ang = (25.0 + rf(rng) * 50.0f32).to_radians() * side;
        let len = (f.half * (0.35 + 0.55 * rf(rng))).max(2.0);
        turtle_branch(grid, sx as f32 * 0.5, sy as f32, ang, len, 0, k, ink, rng, &mut tips);
    }
    let &(tx, ty) = trunk.last().unwrap();
    tips.push((tx, ty - 1));
    let r = ((f.th / 9.0) as i32).max(1);
    for &(x, y) in &tips {
        leaf_blob(grid, x, y, r * 2 + 1, (r / 2).max(1), 0.6, ['◆', '◇', '∙'], ink, rng);
        if rf(rng) < k.fruit {
            put_blank(grid, x, y + 1, '╷', ink.fruit);
            put_blank(grid, x, y + 2, '•', ink.fruit);
        }
    }
}

// ---------------------------------------------------------------- 4. coral (diffusion-limited aggregation)

fn count8(occ: &[u8], lw: usize, lh: usize, x: i32, y: i32) -> u32 {
    let mut n = 0;
    for dy in -1..=1 {
        for dx in -1..=1 {
            if dx == 0 && dy == 0 {
                continue;
            }
            let (nx, ny) = (x + dx, y + dy);
            if nx >= 0 && ny >= 0 && (nx as usize) < lw && (ny as usize) < lh && occ[ny as usize * lw + nx as usize] != 0 {
                n += 1;
            }
        }
    }
    n
}

fn grow_coral(grid: &mut Grid, plot: Rect, root_depth: i32, energy: f32, ink: &Ink, k: &GrowKnobs, rng: &mut StdRng) {
    let f = frame_of(plot, energy);
    let trunk_h = ((f.th * 0.3) as i32).max(2);
    let tw = ((f.th / 11.0) as i32).max(1);
    let trunk = trunk_walk(grid, f.rx, f.ry, trunk_h, (rf(rng) - 0.5) * 0.2, 0.25, tw, ink, rng);
    let &(tx, ty) = trunk.last().unwrap();
    let crown_h = (f.th - trunk_h as f32).max(3.0);
    let s = ((crown_h / 30.0).round() as i32).max(1);
    let lh = (crown_h as i32 / s).max(3) as usize;
    let lw = (((f.half * 2.0) as i32 / s).max(5) as usize) | 1;
    // occ: -2 empty, -1 seed, otherwise the linear index of the parent cell
    let mut occ = vec![-2i32; lw * lh];
    let cx0 = (lw / 2) as i32;
    let cy0 = (lh - 1) as i32;
    occ[cy0 as usize * lw + cx0 as usize] = -1;
    let cap = ((k.detail * 150.0) as usize).clamp(20, 400).min(lw * lh * 3 / 10);
    let mut stuck = 1usize;
    let mut maxr = 1.0f32;
    let down = 0.1 + rf(rng) * 0.15;
    let pull = if rf(rng) < 0.5 { -1 } else { 1 };
    let pull_p = rf(rng) * 0.2;
    let mut launched = 0usize;
    let nbrs: [(i32, i32); 8] = [(0, 1), (0, -1), (-1, 0), (1, 0), (-1, 1), (1, 1), (-1, -1), (1, -1)];
    while stuck < cap && launched < cap * 6 {
        launched += 1;
        let ang = rf(rng) * PI;
        let r = maxr + 3.0;
        let mut x = ((cx0 as f32 + 2.0 * r * ang.cos()).round() as i32).clamp(0, lw as i32 - 1);
        let mut y = ((cy0 as f32 - r * ang.sin()).round() as i32).clamp(0, lh as i32 - 1);
        let mut parent = -2i32;
        for _ in 0..500 {
            if occ[y as usize * lw + x as usize] == -2 {
                for (dx, dy) in nbrs {
                    let (nx, ny) = (x + dx, y + dy);
                    if nx >= 0 && ny >= 0 && (nx as usize) < lw && (ny as usize) < lh && occ[ny as usize * lw + nx as usize] != -2 {
                        parent = ny * lw as i32 + nx;
                        break;
                    }
                }
                if parent != -2 {
                    break;
                }
            }
            let u = rf(rng);
            let (dx, dy) = if u < pull_p {
                (pull, 0)
            } else if u < 0.3 + pull_p * 0.5 {
                (-1, 0)
            } else if u < 0.6 {
                (1, 0)
            } else if u < 0.7 + down {
                (0, 1)
            } else {
                (0, -1)
            };
            x += dx;
            y += dy;
            if x < 0 || x >= lw as i32 || y < 0 || y >= lh as i32 {
                break;
            }
        }
        if parent != -2 {
            occ[y as usize * lw + x as usize] = parent;
            stuck += 1;
            let d = ((x - cx0) as f32 * 0.5).hypot((y - cy0) as f32);
            if d > maxr {
                maxr = d;
            }
        }
    }
    let mut has_child = vec![false; lw * lh];
    for &p in &occ {
        if p >= 0 {
            has_child[p as usize] = true;
        }
    }
    let x_origin = tx - cx0 * s;
    let y_origin = ty - cy0 * s - (s - 1);
    let center = |i: i32| (x_origin + (i % lw as i32) * s + s / 2, y_origin + (i / lw as i32) * s + s / 2);
    let maxd = maxr.max(1.0);
    let leaves = leaf_set(f.th);
    for i in 0..(lw * lh) as i32 {
        let p = occ[i as usize];
        if p == -2 {
            continue;
        }
        let (x, y) = center(i);
        let d = (((i % lw as i32) - cx0) as f32 * 0.5).hypot(((i / lw as i32) - cy0) as f32) / maxd;
        let c = lerp_color(ink.branch, ink.leaf2, (d * 1.1).min(1.0));
        if p >= 0 {
            let (px, py) = center(p);
            seg(grid, px, py, x, y, c, s >= 2 && d < 0.4);
        }
        if !has_child[i as usize] {
            if rf(rng) < k.fruit {
                put(grid, x, y, '○', ink.fruit);
            } else {
                put(grid, x, y, leaves[1], ink.leaf);
                leaf_blob(grid, x, y, s + 1, (s / 2).max(1), 0.5, leaves, ink, rng);
            }
        }
    }
    root_fan(grid, f.rx, f.ry, (f.half * k.roots) as i32, ((root_depth as f32) * k.roots) as i32, ink.root, rng);
}

// ---------------------------------------------------------------- 5. sunburst (phyllotactic crown)

fn grow_sunburst(grid: &mut Grid, plot: Rect, root_depth: i32, energy: f32, ink: &Ink, k: &GrowKnobs, rng: &mut StdRng) {
    let f = frame_of(plot, energy);
    let trunk_h = ((f.th * (0.36 + 0.12 * rf(rng))) as i32).max(2);
    let tw = ((f.th / 14.0) as i32).max(1);
    let trunk = trunk_walk(grid, f.rx, f.ry, trunk_h, (rf(rng) - 0.5) * 0.3, 0.3, tw, ink, rng);
    let &(tx, ty) = trunk.last().unwrap();
    let rr = ((f.th - trunk_h as f32) * 0.5).min(f.half * 0.5).max(1.5);
    let cx = tx + ((rf(rng) - 0.5) * rr).round() as i32;
    let cy = ty - rr.round() as i32;
    seg(grid, tx, ty, cx, cy + rr.round() as i32 - 1, ink.trunk, tw >= 2);
    let skew = (rf(rng) - 0.5) * 0.6;
    let skew_ang = rf(rng) * TAU;
    let n_spokes = (3.0 + k.branch * 3.0 + rf(rng) * 2.0) as i32;
    for _ in 0..n_spokes {
        let th = (0.2 + rf(rng) * 2.7) * if rf(rng) < 0.5 { 1.0 } else { -1.0 } + PI * 0.5;
        let r = rr * (1.0 + skew * 0.25 * (th - skew_ang).cos());
        let ex = cx + (2.0 * r * th.cos()).round() as i32;
        let ey = cy - (r * th.sin()).round() as i32;
        seg(grid, cx, cy, ex, ey, ink.branch, false);
        if rf(rng) < k.branch * 0.6 {
            let mx = cx + (1.2 * r * th.cos()).round() as i32;
            let my = cy - (0.6 * r * th.sin()).round() as i32;
            let th2 = th + (rf(rng) - 0.5) * 1.2;
            seg(grid, mx, my, mx + (r * th2.cos()).round() as i32, my - (0.5 * r * th2.sin()).round() as i32, ink.branch, false);
        }
    }
    let n_pts = ((5.0 * rr * rr) as usize).clamp(24, 6000);
    let phase = rf(rng) * TAU;
    for i in 0..n_pts {
        let b = (i as f32 / n_pts as f32).sqrt();
        let th = i as f32 * 2.399963 + phase;
        let r = rr * b * (1.0 + skew * 0.25 * (th - skew_ang).cos());
        let x = cx + (2.0 * r * th.cos()).round() as i32;
        let y = cy - (r * th.sin()).round() as i32;
        let ch = if b < 0.35 { '●' } else if b < 0.65 { '◆' } else if b < 0.85 { '•' } else if b < 0.95 { '∙' } else { '·' };
        let c = lerp_color(ink.leaf2, ink.leaf, b);
        if b > 0.7 && rf(rng) < k.fruit * 0.4 {
            put_blank(grid, x, y, '○', ink.fruit);
        } else {
            put_blank(grid, x, y, ch, c);
        }
    }
    root_fan(grid, f.rx, f.ry, (f.half * k.roots) as i32, ((root_depth as f32) * k.roots) as i32, ink.root, rng);
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
    pub w: usize,
    pub root_row: usize,
    pub rows: Vec<Vec<SpriteCell>>,
    pub spans: Vec<Option<(u16, u16)>>,
}

fn is_leaf_glyph(ch: char) -> bool {
    matches!(ch, '●' | '•' | '∙' | '·' | '◆' | '◇' | '○' | '◦')
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
        other => other,
    }
}

pub(crate) fn sprite_from_grid(scratch: &Grid, root_row: usize, pal: &mut Palette) -> Sprite {
    let w = scratch.first().map(|r| r.len()).unwrap_or(0);
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
    // dense rows occlude what stands behind them; sparse rows (a lone trunk) do not
    let spans = rows
        .iter()
        .map(|row| {
            let (Some(a), Some(b)) = (row.first(), row.last()) else { return None };
            let span = (b.dx - a.dx + 1) as usize;
            if row.len() >= 3 && row.len() * 10 >= span * 3 { Some((a.dx, b.dx)) } else { None }
        })
        .collect();
    Sprite { w, root_row, rows, spans }
}

pub(crate) fn hash3(a: u32, b: u32, c: u32) -> u32 {
    let mut h = a.wrapping_mul(0x9E37_79B9) ^ b.wrapping_mul(0x85EB_CA6B) ^ c.wrapping_mul(0xC2B2_AE35);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    h
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
            let plot_h = (gy - py) as usize;
            let plot_w = cell_w - 2;
            let mut scratch = vec![vec![Cell::blank(); plot_w]; plot_h + rd as usize];
            let mut rng = StdRng::seed_from_u64(key.seed ^ hash3(i as u32 + 1, row as u32 + 1, 0x5EED) as u64);
            let hue = (base_hue + i as f64 * 24.0 + row as f64 * 30.0 - 30.0).rem_euclid(360.0);
            let ink = Ink::from_hue(hue, 0.55, 0.40);
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

pub(crate) fn draw_fable_2_trees(grid: &mut Grid, width: usize, height: usize, seed: u64, palette: &[Color; 5], t: f32, k: &SheetKnobs) {
    let gh = height.min(grid.len());
    let gw = width.min(grid.first().map(|r| r.len()).unwrap_or(0));
    if gw < 4 || gh < 4 {
        return;
    }
    measure_layer("fable-2-trees", "clear", || {
        for row in grid.iter_mut().take(gh) {
            row[..gw].fill(Cell::blank());
        }
    });
    let key = SheetKey { w: gw, h: gh, seed, palette: *palette, energy: k.energy, fruit: k.fruit, branch: k.branch, detail: k.detail, roots: k.roots };
    let hit = SHEET.with(|c| c.borrow().as_ref().map(|s| s.key == key).unwrap_or(false));
    if !hit {
        let sheet = measure_layer("fable-2-trees", "grow", || build_sheet(key.clone(), k));
        SHEET.with(|c| *c.borrow_mut() = Some(sheet));
    }
    SHEET.with(|c| {
        let b = c.borrow();
        let s = b.as_ref().unwrap();
        let lit = &s.pal.colors;
        measure_layer("fable-2-trees", "ground", || {
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
        measure_layer("fable-2-trees", "trees", || {
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

pub(crate) fn cli_fable_2_trees(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], _rng: StdRng, t_anim: f32, _term_w: u16, _term_h: u16, args: &[String], _mode: &str, _theme_name: &str) -> (Grid, bool) {
    let mut k = SheetKnobs::from_env();
    let slots: [&mut f32; 8] = [&mut k.energy, &mut k.fruit, &mut k.branch, &mut k.sway, &mut k.speed, &mut k.flicker, &mut k.detail, &mut k.roots];
    for (i, slot) in slots.into_iter().enumerate() {
        if let Some(v) = args.get(4 + i).and_then(|s| s.parse().ok()) {
            *slot = v;
        }
    }
    draw_fable_2_trees(&mut grid, width, height, seed, &palette, t_anim, &k);
    (grid, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(w: usize, h: usize, seed: u64, t: f32) -> String {
        let mut g = vec![vec![Cell::blank(); w]; h];
        let p = crate::color::make_palette(seed);
        let k = SheetKnobs::from_env();
        draw_fable_2_trees(&mut g, w, h, seed, &p, t, &k);
        g.iter().map(|row| row.iter().map(|c| c.ch).collect::<String>()).collect::<Vec<_>>().join("\n")
    }

    #[test]
    fn snapshot_fable_2_trees_static() {
        insta::assert_snapshot!("fable_2_trees_80x24_static", run(80, 24, 42, 0.0));
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
                assert!(s.chars().filter(|c| *c != ' ').count() > 20, "{:?} drew nothing", sp);
                outs.push(s);
            }
            assert_ne!(outs[0], outs[1], "{:?} identical across seeds", sp);
        }
    }
}
