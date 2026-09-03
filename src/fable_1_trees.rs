//! fable-1-trees: five tree-thing growth algorithms (space colonization, banyan,
//! mangrove, baobab, coral DLA) and the two-row sample sheet that shows them.
use crate::_0_profile::measure_layer;
use crate::color::*;
use crate::opts::param_f32;
use crate::types::*;
use crossterm::style::Color;
use rand::RngExt;
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::cell::RefCell;
use std::f32::consts::TAU;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Species {
    Colonize,
    Banyan,
    Mangrove,
    Baobab,
    Coral,
}

pub(crate) const SPECIES: [Species; 5] = [
    Species::Colonize,
    Species::Banyan,
    Species::Mangrove,
    Species::Baobab,
    Species::Coral,
];

impl Species {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Species::Colonize => "colonize",
            Species::Banyan => "banyan",
            Species::Mangrove => "mangrove",
            Species::Baobab => "baobab",
            Species::Coral => "coral",
        }
    }
}

/// Color set one tree draws with.
#[derive(Clone, Copy)]
pub(crate) struct Ink {
    pub trunk: Color,
    pub bark: Color,
    pub limb: Color,
    pub twig: Color,
    pub root: Color,
    pub leaf_dark: Color,
    pub leaf: Color,
    pub leaf_lit: Color,
    pub fruit: Color,
    pub fruit_dark: Color,
}

impl Ink {
    pub(crate) fn from_base(trunk: Color, leaf: Color, fruit: Color) -> Ink {
        Ink {
            trunk: lighten(trunk, 12),
            bark: darken(trunk, 14),
            limb: lerp_color(trunk, leaf, 0.45),
            twig: lerp_color(trunk, leaf, 0.7),
            root: darken(trunk, 4),
            leaf_dark: darken(leaf, 30),
            leaf,
            leaf_lit: lighten(leaf, 45),
            fruit,
            fruit_dark: darken(fruit, 40),
        }
    }

    /// Blend every color toward `to` by `f`, for depth falloff.
    pub(crate) fn faded(&self, to: Color, f: f32) -> Ink {
        let m = |c: Color| lerp_color(c, to, f);
        Ink {
            trunk: m(self.trunk),
            bark: m(self.bark),
            limb: m(self.limb),
            twig: m(self.twig),
            root: m(self.root),
            leaf_dark: m(self.leaf_dark),
            leaf: m(self.leaf),
            leaf_lit: m(self.leaf_lit),
            fruit: m(self.fruit),
            fruit_dark: m(self.fruit_dark),
        }
    }
}

/// Growth dials shared by every species.
#[derive(Clone, Copy)]
pub(crate) struct Growth {
    pub fruit: f32,
    pub branch: f32,
    pub leaf: f32,
    pub roots: f32,
}

const SOFT: [char; 6] = [' ', '·', '∙', '~', '░', '╌'];
const LEAFY: [char; 5] = ['▒', '░', '◦', '∙', '•'];

/// Clipped painter: writes only inside both the plot and the grid.
pub(crate) struct Brush<'a> {
    grid: &'a mut Grid,
    pub plot: Rect,
}

impl<'a> Brush<'a> {
    pub(crate) fn new(grid: &'a mut Grid, plot: Rect) -> Brush<'a> {
        Brush { grid, plot }
    }

    fn inside(&self, x: i32, y: i32) -> bool {
        let p = self.plot;
        x >= p.x as i32
            && y >= p.y as i32
            && x < (p.x + p.w) as i32
            && y < (p.y + p.h) as i32
            && (y as usize) < self.grid.len()
            && (x as usize) < self.grid[y as usize].len()
    }

    fn put(&mut self, x: i32, y: i32, ch: char, fg: Color) {
        if self.inside(x, y) {
            self.grid[y as usize][x as usize] = Cell::new(ch, fg);
        }
    }

    fn put_soft(&mut self, x: i32, y: i32, ch: char, fg: Color) {
        if self.inside(x, y) && SOFT.contains(&self.grid[y as usize][x as usize].ch) {
            self.grid[y as usize][x as usize] = Cell::new(ch, fg);
        }
    }

    fn put_leafy(&mut self, x: i32, y: i32, ch: char, fg: Color) {
        if self.inside(x, y) {
            let cur = self.grid[y as usize][x as usize].ch;
            if SOFT.contains(&cur) || LEAFY.contains(&cur) {
                self.grid[y as usize][x as usize] = Cell::new(ch, fg);
            }
        }
    }

    fn root(&self) -> (i32, i32) {
        let p = self.plot;
        (p.x as i32 + p.w as i32 / 2, p.y as i32 + p.h as i32 - 1)
    }
}

fn slope_glyph(dx: i32, dy: i32) -> char {
    if dy == 0 {
        return '─';
    }
    if dx == 0 {
        return '│';
    }
    let r = dx.abs() as f32 / dy.abs() as f32;
    if r < 0.7 {
        '│'
    } else if r > 3.5 {
        '─'
    } else if (dx > 0) == (dy < 0) {
        '╱'
    } else {
        '╲'
    }
}

fn stroke(b: &mut Brush, x0: i32, y0: i32, x1: i32, y1: i32, fg: Color, heavy: bool) {
    let dx = x1 - x0;
    let dy = y1 - y0;
    let n = dx.abs().max(dy.abs());
    if n == 0 {
        return;
    }
    let mut ch = slope_glyph(dx, dy);
    if heavy && ch == '│' {
        ch = '┃';
    }
    for i in 1..=n {
        let f = i as f32 / n as f32;
        let x = x0 + (dx as f32 * f).round() as i32;
        let y = y0 + (dy as f32 * f).round() as i32;
        b.put(x, y, ch, fg);
    }
}

/// Surface roots on the ground row: `╌╌╱┴╲╌`.
fn flare(b: &mut Brush, rx: i32, ry: i32, hw: i32, ink: &Ink, rng: &mut StdRng) {
    b.put(rx, ry, if hw == 0 { '┴' } else { '┃' }, ink.trunk);
    b.put(rx - hw - 1, ry, '╱', ink.root);
    b.put(rx + hw + 1, ry, '╲', ink.root);
    for side in [-1i32, 1] {
        let len = rng.random_range(0..4u32) as i32;
        for k in 1..=len {
            b.put_soft(rx + side * (hw + 1 + k), ry, '╌', ink.root);
        }
    }
}

// ── space colonization ──────────────────────────────────────────────

struct CNode {
    x: f32,
    y: f32,
    parent: i32,
    kids: u16,
    size: u32,
}

fn grow_colonize(b: &mut Brush, energy: f32, ink: &Ink, g: &Growth, rng: &mut StdRng) {
    let (rx, ry) = b.root();
    let ph = b.plot.h as f32;
    let pw = b.plot.w as f32;
    let h = (ph * energy).max(4.0);
    let half = (pw / 2.0 * (0.55 + 0.45 * energy)).max(2.0);
    let trunk_h = ((h * (0.2 + 0.12 * rng.random::<f32>())).max(1.0)) as i32;
    let crown_h = (h - trunk_h as f32).max(2.0);
    let ccy = ry as f32 - trunk_h as f32 - crown_h * 0.5;
    let lean = (rng.random::<f32>() - 0.5) * half * 0.6;
    let ccx = rx as f32 + lean;
    let bias = rng.random::<f32>() * 2.0 - 1.0;
    let n_attr = ((half * crown_h * 0.5 * (0.5 + g.branch)) as usize).clamp(12, 160);
    let mut attr: Vec<(f32, f32)> = Vec::with_capacity(n_attr);
    let mut tries = 0;
    while attr.len() < n_attr && tries < n_attr * 8 {
        tries += 1;
        let u = rng.random::<f32>() * 2.0 - 1.0;
        let v = rng.random::<f32>() * 2.0 - 1.0;
        if u * u + v * v > 1.0 {
            continue;
        }
        if u * bias < -0.25 && rng.random::<f32>() < 0.6 {
            continue;
        }
        attr.push((ccx + u * half, ccy + v * crown_h * 0.5));
    }
    let mut nodes: Vec<CNode> = (0..=trunk_h)
        .map(|i| CNode {
            x: rx as f32,
            y: (ry - i) as f32,
            parent: i - 1,
            kids: if i < trunk_h { 1 } else { 0 },
            size: 0,
        })
        .collect();
    let pw_i = b.plot.w as i32;
    let ph_i = b.plot.h as i32;
    let (ox, oy) = (b.plot.x as i32, b.plot.y as i32);
    let mut occ = vec![false; (pw_i * ph_i) as usize];
    for n in &nodes {
        let (cx, cy) = (n.x.round() as i32 - ox, n.y.round() as i32 - oy);
        if cx >= 0 && cy >= 0 && cx < pw_i && cy < ph_i {
            occ[(cy * pw_i + cx) as usize] = true;
        }
    }
    let infl = 4.0 + 4.0 * g.branch.min(1.5) + crown_h * 0.08;
    let kill = 1.5f32;
    for _ in 0..120 {
        if attr.is_empty() || nodes.len() >= 600 {
            break;
        }
        let mut acc = vec![(0.0f32, 0.0f32, 0u32); nodes.len()];
        let mut keep = Vec::with_capacity(attr.len());
        for &(ax, ay) in &attr {
            let mut best = 0usize;
            let mut bd = f32::MAX;
            for (i, n) in nodes.iter().enumerate() {
                let du = (ax - n.x) * 0.5;
                let dv = ay - n.y;
                let d = du * du + dv * dv;
                if d < bd {
                    bd = d;
                    best = i;
                }
            }
            if bd < kill * kill {
                continue;
            }
            if bd < infl * infl {
                let d = bd.sqrt();
                let n = &nodes[best];
                acc[best].0 += (ax - n.x) * 0.5 / d;
                acc[best].1 += (ay - n.y) / d;
                acc[best].2 += 1;
            }
            keep.push((ax, ay));
        }
        attr = keep;
        let mut added = 0;
        let count = nodes.len();
        for i in 0..count {
            let (au, av, c) = acc[i];
            if c == 0 {
                continue;
            }
            let jitter = (rng.random::<f32>() - 0.5) * 0.5;
            let (mut u, mut v) = (au / c as f32 + jitter, av / c as f32);
            let len = (u * u + v * v).sqrt();
            if len < 1e-3 {
                continue;
            }
            u /= len;
            v /= len;
            let nx = nodes[i].x + u * 2.0;
            let ny = nodes[i].y + v;
            let (cx, cy) = (nx.round() as i32, ny.round() as i32);
            if (cx, cy) == (nodes[i].x.round() as i32, nodes[i].y.round() as i32) {
                continue;
            }
            let (lx, ly) = (cx - ox, cy - oy);
            if lx < 0 || ly < 0 || lx >= pw_i || ly >= ph_i || occ[(ly * pw_i + lx) as usize] {
                continue;
            }
            occ[(ly * pw_i + lx) as usize] = true;
            nodes.push(CNode { x: nx, y: ny, parent: i as i32, kids: 0, size: 0 });
            nodes[i].kids += 1;
            added += 1;
        }
        if added == 0 {
            break;
        }
    }
    for i in (0..nodes.len()).rev() {
        nodes[i].size += 1;
        let p = nodes[i].parent;
        if p >= 0 {
            let s = nodes[i].size;
            nodes[p as usize].size += s;
        }
    }
    let total = nodes[0].size.max(1) as f32;
    for i in 1..nodes.len() {
        let p = nodes[i].parent as usize;
        let f = nodes[i].size as f32 / total;
        let (fg, heavy) = if f > 0.3 {
            (ink.trunk, true)
        } else if f > 0.08 {
            (ink.limb, false)
        } else {
            (ink.twig, false)
        };
        let (x0, y0) = (nodes[p].x.round() as i32, nodes[p].y.round() as i32);
        let (x1, y1) = (nodes[i].x.round() as i32, nodes[i].y.round() as i32);
        stroke(b, x0, y0, x1, y1, fg, heavy);
    }
    for i in trunk_h as usize + 1..nodes.len() {
        if nodes[i].size > 3 || nodes[i].kids == 0 {
            continue;
        }
        let (x, y) = (nodes[i].x.round() as i32, nodes[i].y.round() as i32);
        for (dx, dy) in [(-1, 0), (1, 0), (0, -1)] {
            if rng.random::<f32>() < g.leaf * 0.45 {
                b.put_soft(x + dx, y + dy, if dy == 0 { '▒' } else { '░' }, ink.leaf);
            }
        }
    }
    let tips: Vec<(i32, i32)> = nodes
        .iter()
        .skip(trunk_h as usize + 1)
        .filter(|n| n.kids == 0)
        .map(|n| (n.x.round() as i32, n.y.round() as i32))
        .collect();
    for &(tx, ty) in &tips {
        b.put_soft(tx, ty - 1, '◦', ink.leaf_lit);
        for (dx, dy) in [(-1, 0), (1, 0), (-2, 0), (2, 0), (-1, -1), (1, -1), (0, -2)] {
            if rng.random::<f32>() < g.leaf * 0.6 {
                let ch = if dy == 0 { '▒' } else { '░' };
                let fg = if dy == 0 { ink.leaf } else { ink.leaf_dark };
                b.put_soft(tx + dx, ty + dy, ch, fg);
            }
        }
        if rng.random::<f32>() < g.fruit * 0.5 {
            b.put_soft(tx, ty + 1, '●', ink.fruit);
        }
    }
    flare(b, rx, ry, 0, ink, rng);
}

// ── banyan ──────────────────────────────────────────────────────────

fn grow_banyan(b: &mut Brush, energy: f32, ink: &Ink, g: &Growth, rng: &mut StdRng) {
    let (rx, ry) = b.root();
    let h = (b.plot.h as f32 * energy).max(4.0);
    let half = ((b.plot.w as f32 / 2.0 * (0.6 + 0.4 * energy)).max(2.0)) as i32;
    let trunk_h = ((h * 0.5) as i32).max(2);
    let thick = half >= 4;
    let mut x = rx;
    for i in 0..trunk_h {
        let y = ry - i;
        if i > 0 && i % 3 == 0 && rng.random::<f32>() < 0.35 {
            let s = if rng.random::<bool>() { 1 } else { -1 };
            x += s;
            b.put(x, y, if s > 0 { '╱' } else { '╲' }, ink.trunk);
        } else {
            b.put(x, y, '┃', ink.trunk);
        }
        if thick && i < trunk_h / 2 {
            b.put(x - 1, y, '│', ink.bark);
            b.put(x + 1, y, '│', ink.bark);
        }
    }
    let crown_y = ry - trunk_h;
    let mut cover: Vec<(i32, i32)> = vec![(x, crown_y + 1)];
    let mut roots: Vec<(i32, i32)> = Vec::new();
    let n_limbs = (2.0 + g.branch * 2.5 + rng.random::<f32>() * 1.5) as usize;
    for li in 0..n_limbs {
        let mut side = if li % 2 == 0 { 1 } else { -1 };
        if rng.random::<f32>() < 0.2 {
            side = -side;
        }
        let len = (half as f32 * (0.45 + 0.55 * rng.random::<f32>())) as i32;
        let mut lx = x;
        let mut ly = crown_y + rng.random_range(0..2u32) as i32;
        let rise = 0.12 + 0.25 * rng.random::<f32>();
        let mut since_root = rng.random_range(0..3u32) as i32;
        for s in 0..len {
            lx += side;
            if rng.random::<f32>() < rise {
                ly -= 1;
                b.put(lx, ly, if side > 0 { '╱' } else { '╲' }, ink.limb);
            } else {
                b.put(lx, ly, '─', ink.limb);
            }
            cover.push((lx, ly));
            since_root += 1;
            if s >= 2 && since_root >= 3 && rng.random::<f32>() < g.roots * 0.6 {
                since_root = 0;
                roots.push((lx, ly));
            }
        }
    }
    for (sx, sy) in roots {
        let full = rng.random::<f32>() < 0.65;
        let span = ry - sy - 1;
        let stop = if full {
            ry
        } else {
            sy + 1 + (span as f32 * (0.3 + 0.5 * rng.random::<f32>())) as i32
        };
        let third = ry - (ry - sy) / 3;
        let mut x = sx;
        for y in sy + 1..=stop {
            if y < stop && rng.random::<f32>() < 0.12 {
                let s = if rng.random::<bool>() { 1 } else { -1 };
                x += s;
                b.put_soft(x, y, if s > 0 { '╲' } else { '╱' }, ink.root);
                continue;
            }
            let ch = if full && y >= third { '│' } else { '┊' };
            b.put_soft(x, y, ch, ink.root);
        }
        if full {
            b.put(x, ry, '┴', ink.trunk);
        } else {
            b.put_soft(x, stop, '╷', lighten(ink.root, 20));
        }
    }
    let rows = ((1.0 + energy * 2.0 * g.leaf).round() as i32).clamp(1, 4);
    for &(cx, cy) in &cover {
        for k in 1..=rows {
            let dens = g.leaf * (1.0 - (k - 1) as f32 * 0.28);
            for dx in -k..=k {
                if rng.random::<f32>() < dens {
                    let r = rng.random::<f32>();
                    let (ch, fg) = if r < 0.5 {
                        ('▒', ink.leaf)
                    } else if r < 0.8 {
                        ('░', ink.leaf_dark)
                    } else {
                        ('◦', ink.leaf_lit)
                    };
                    b.put_soft(cx + dx, cy - k, ch, fg);
                }
            }
        }
        for dx in -1..=1 {
            if rng.random::<f32>() < g.leaf * 0.3 {
                b.put_soft(cx + dx, cy + 1, '∙', ink.leaf_dark);
            }
        }
        if rng.random::<f32>() < g.fruit * 0.3 {
            let dx = rng.random_range(0..3u32) as i32 - 1;
            b.put_leafy(cx + dx, cy - 1, '●', ink.fruit);
        }
    }
    let hw = if thick { 1 } else { 0 };
    b.put(rx - hw - 1, ry, '╱', ink.root);
    b.put(rx + hw + 1, ry, '╲', ink.root);
}

// ── mangrove ────────────────────────────────────────────────────────

fn grow_mangrove(b: &mut Brush, energy: f32, ink: &Ink, g: &Growth, rng: &mut StdRng) {
    let (rx, ry) = b.root();
    let h = (b.plot.h as f32 * energy).max(4.0);
    let half = ((b.plot.w as f32 / 2.0 * (0.6 + 0.4 * energy)).max(2.0)) as i32;
    let stilt = ((h * 0.3) as i32).clamp(2, 8);
    let hub_y = ry - stilt;
    let n_roots = (3.0 + g.roots * 4.0 + rng.random::<f32>() * 2.0) as usize;
    let skew = 0.35 + 0.3 * rng.random::<f32>();
    for _ in 0..n_roots {
        let side = if rng.random::<f32>() < skew { 1 } else { -1 };
        let reach = (1.0 + rng.random::<f32>() * half as f32 * 0.95) as i32;
        let mut x = rx;
        let mut y = hub_y + (rng.random_range(0..3u32) as i32).min(stilt - 1);
        let mut flat = rng.random::<f32>() < 0.5;
        while (x - rx).abs() < reach && y < ry {
            x += side;
            if flat {
                b.put_soft(x, y, '─', ink.root);
            } else {
                y += 1;
                b.put_soft(x, y, if side > 0 { '╲' } else { '╱' }, ink.root);
            }
            flat = !flat && rng.random::<f32>() < 0.5;
        }
        while y < ry {
            y += 1;
            b.put_soft(x, y, '│', ink.root);
        }
        b.put(x, ry, '┴', ink.root);
    }
    let crown_base = ry - ((h * 0.5) as i32).max(stilt + 2);
    let lean_at = if hub_y - crown_base > 2 {
        crown_base + 1 + rng.random_range(0..(hub_y - crown_base - 1) as u32) as i32
    } else {
        -1
    };
    let mut x = rx;
    for y in (crown_base..=hub_y).rev() {
        if y == lean_at {
            let s = if rng.random::<bool>() { 1 } else { -1 };
            x += s;
            b.put(x, y, if s > 0 { '╱' } else { '╲' }, ink.trunk);
        } else {
            b.put(x, y, '┃', ink.trunk);
        }
    }
    b.put(rx, ry, '┃', ink.trunk);
    let cry = ((h * 0.22) as i32).max(2);
    let crx = half;
    let ccx = x + ((rng.random::<f32>() - 0.5) * half as f32 * 0.5) as i32;
    let ccy = crown_base + 1 - cry;
    let n_inner = (2.0 + g.branch * 3.0) as usize;
    for _ in 0..n_inner {
        let tx = ccx + ((rng.random::<f32>() * 2.0 - 1.0) * crx as f32 * 0.8) as i32;
        let ty = ccy + ((rng.random::<f32>() * 2.0 - 1.0) * cry as f32 * 0.7) as i32;
        stroke(b, x, crown_base, tx, ty, ink.limb, false);
    }
    for y in ccy - cry..=ccy + cry {
        for x in ccx - crx..=ccx + crx {
            let u = (x - ccx) as f32 / crx as f32;
            let v = (y - ccy) as f32 / cry as f32;
            let nd = u * u + v * v;
            if nd > 1.0 {
                continue;
            }
            if rng.random::<f32>() >= g.leaf * (1.05 - nd * 0.75) {
                continue;
            }
            let (ch, fg) = if nd < 0.4 {
                ('▒', ink.leaf_dark)
            } else if nd < 0.8 {
                ('░', ink.leaf)
            } else if rng.random::<bool>() {
                ('◦', ink.leaf_lit)
            } else {
                ('∙', ink.leaf)
            };
            b.put_soft(x, y, ch, fg);
        }
    }
    for x in ccx - crx + 1..ccx + crx {
        if rng.random::<f32>() < g.fruit * 0.35 {
            let u = (x - ccx) as f32 / crx as f32;
            let yb = ccy + (cry as f32 * (1.0 - u * u).max(0.0).sqrt()) as i32 + 1;
            b.put_soft(x, yb, '╎', ink.fruit_dark);
            b.put_soft(x, yb + 1, '•', ink.fruit);
        }
    }
}

// ── baobab ──────────────────────────────────────────────────────────

fn twig(
    b: &mut Brush,
    x: i32,
    y: i32,
    dir: f32,
    len: i32,
    depth: usize,
    ink: &Ink,
    rng: &mut StdRng,
    tips: &mut Vec<(i32, i32)>,
) {
    let fg = if depth == 0 { ink.limb } else { ink.twig };
    let mut fx = x as f32;
    let mut cx = x;
    let mut cy = y;
    for _ in 0..len {
        fx += dir;
        cy -= 1;
        let nx = fx.round() as i32;
        b.put(nx, cy, slope_glyph(nx - cx, -1), fg);
        cx = nx;
    }
    if depth < 2 && len >= 3 && rng.random::<f32>() < 0.85 {
        let k = if rng.random::<f32>() < 0.6 { 2 } else { 1 };
        for i in 0..k {
            let sgn = if i == 0 { 1.0 } else { -1.0 };
            let nd = dir + sgn * (0.5 + rng.random::<f32>() * 1.0);
            let nl = ((len as f32) * (0.5 + rng.random::<f32>() * 0.25)) as i32;
            twig(b, cx, cy, nd, nl.max(1), depth + 1, ink, rng, tips);
        }
    } else {
        tips.push((cx, cy));
    }
}

fn grow_baobab(b: &mut Brush, energy: f32, ink: &Ink, g: &Growth, rng: &mut StdRng) {
    let (rx, ry) = b.root();
    let h = (b.plot.h as f32 * energy).max(4.0);
    let half = (b.plot.w as f32 / 2.0).max(2.0);
    let trunk_h = ((h * 0.62) as i32).max(3);
    let base_hw = ((half * 0.45).min(h * 0.2) as i32).clamp(1, 7);
    let lean = (rng.random::<f32>() - 0.5) * 3.0;
    let bulge = 0.5 + 0.5 * rng.random::<f32>();
    let mut prev_hw = base_hw;
    let mut top_cx = rx;
    let mut top_hw = 0;
    for i in 0..trunk_h {
        let f = i as f32 / trunk_h as f32;
        let hw = ((base_hw as f32 * (1.0 - bulge * f * f * 0.8)).round() as i32).max(0);
        let cx = rx + (lean * f).round() as i32;
        let y = ry - i;
        if hw == 0 {
            b.put(cx, y, '┃', ink.trunk);
        } else {
            let lch = if hw < prev_hw { '╱' } else { '│' };
            let rch = if hw < prev_hw { '╲' } else { '│' };
            b.put(cx - hw, y, lch, ink.trunk);
            b.put(cx + hw, y, rch, ink.trunk);
            for x in cx - hw + 1..cx + hw {
                let mut ch = if (x + i / 3).rem_euclid(3) == 0 { '▒' } else { '░' };
                if rng.random::<f32>() < 0.06 {
                    ch = '▓';
                }
                b.put(x, y, ch, ink.bark);
            }
        }
        prev_hw = hw;
        top_cx = cx;
        top_hw = hw;
    }
    flare(b, rx, ry, base_hw, ink, rng);
    let top_y = ry - trunk_h;
    let n = (3.0 + g.branch * 4.0 + rng.random::<f32>() * 2.0) as usize;
    let twig_len = ((h * 0.3) as i32).max(2);
    let mut tips: Vec<(i32, i32)> = Vec::new();
    for _ in 0..n {
        let sx = top_cx + rng.random_range(0..(2 * top_hw + 1) as u32) as i32 - top_hw;
        let dir = (rng.random::<f32>() - 0.5) * 3.2;
        let len = ((twig_len as f32) * (0.6 + 0.4 * rng.random::<f32>())) as i32;
        twig(b, sx, top_y + 1, dir, len.max(1), 0, ink, rng, &mut tips);
    }
    for &(tx, ty) in &tips {
        b.put_soft(tx, ty - 1, '╵', ink.twig);
        if rng.random::<f32>() < g.leaf * 0.35 {
            b.put_soft(tx - 1, ty, '∙', ink.leaf);
        }
        if rng.random::<f32>() < g.leaf * 0.35 {
            b.put_soft(tx + 1, ty, '∙', ink.leaf);
        }
        if rng.random::<f32>() < g.fruit * 0.6 {
            b.put_soft(tx + 1, ty + 1, '○', ink.fruit);
        }
    }
}

// ── coral (diffusion-limited aggregation) ───────────────────────────

fn grow_coral(b: &mut Brush, energy: f32, ink: &Ink, g: &Growth, rng: &mut StdRng) {
    let (rx, ry) = b.root();
    let h = (b.plot.h as f32 * energy).max(4.0);
    let half = (b.plot.w as f32 / 2.0 * (0.6 + 0.4 * energy)).max(2.0);
    let pw = b.plot.w as i32;
    let ph = b.plot.h as i32;
    let (ox, oy) = (b.plot.x as i32, b.plot.y as i32);
    let inside = |x: i32, y: i32| x >= ox && y >= oy && x < ox + pw && y < oy + ph;
    let idx = |x: i32, y: i32| ((y - oy) * pw + (x - ox)) as usize;
    let mut occ: Vec<i32> = vec![-1; (pw * ph) as usize];
    let mut cells: Vec<(i32, i32, i32, u16)> = Vec::new();
    let stem = ((h * 0.18) as i32).clamp(1, 5);
    for i in 0..=stem {
        let (x, y) = (rx, ry - i);
        if inside(x, y) {
            occ[idx(x, y)] = cells.len() as i32;
            cells.push((x, y, i - 1, i as u16));
        }
    }
    let ex = half;
    let ey = (h * 0.42).max(1.5);
    let ecx = rx as f32 + (rng.random::<f32>() - 0.5) * half * 0.5;
    let ecy = ry as f32 - stem as f32 - ey * 0.9;
    let n_walk = ((ex * ey * 1.1 * (0.6 + 0.4 * g.leaf)) as usize).clamp(24, 260);
    let stick_one = 0.5 + 0.45 * g.branch.min(1.0);
    let stick_many = 0.1;
    let pull = 0.18;
    let dirs: [(i32, i32, i32); 8] = [
        (0, 1, 4),
        (-1, 0, 2),
        (1, 0, 2),
        (0, -1, 0),
        (-1, 1, 3),
        (1, 1, 3),
        (-1, -1, 1),
        (1, -1, 1),
    ];
    'walkers: for _ in 0..n_walk {
        if cells.len() > 500 {
            break;
        }
        let (mut x, mut y);
        loop {
            let u = rng.random::<f32>() * 2.0 - 1.0;
            let v = rng.random::<f32>() * 2.0 - 1.0;
            if u * u + v * v <= 1.0 {
                x = (ecx + u * ex).round() as i32;
                y = (ecy + v * ey).round() as i32;
                break;
            }
        }
        if !inside(x, y) || occ[idx(x, y)] >= 0 {
            continue;
        }
        for _ in 0..300 {
            let mut n8 = 0;
            let mut n4 = 0;
            let mut best = -1i32;
            let mut best_score = -1;
            for (k, &(dx, dy, score)) in dirs.iter().enumerate() {
                let (nx, ny) = (x + dx, y + dy);
                if !inside(nx, ny) {
                    continue;
                }
                let p = occ[idx(nx, ny)];
                if p >= 0 {
                    n8 += 1;
                    if k < 4 {
                        n4 += 1;
                        if score > best_score {
                            best_score = score;
                            best = p;
                        }
                    }
                }
            }
            if n4 > 0 {
                let p_stick = if n8 == 1 {
                    stick_one
                } else if n8 == 2 {
                    stick_many * 2.0
                } else {
                    stick_many
                };
                if rng.random::<f32>() < p_stick {
                    let g_next = cells[best as usize].3 + 1;
                    occ[idx(x, y)] = cells.len() as i32;
                    cells.push((x, y, best, g_next));
                    continue 'walkers;
                }
            }
            let r = rng.random::<f32>();
            let mut dx = if r < 0.28 {
                -1
            } else if r < 0.56 {
                1
            } else {
                0
            };
            if rng.random::<f32>() < pull {
                dx = if (ecx - x as f32) > 0.0 { 1 } else { -1 };
            }
            let r2 = rng.random::<f32>();
            let dy = if r2 < 0.45 {
                1
            } else if r2 < 0.65 {
                -1
            } else {
                0
            };
            x += dx;
            y += dy;
            if !inside(x, y) || y >= ry {
                continue 'walkers;
            }
        }
    }
    let max_gen = cells.iter().map(|c| c.3).max().unwrap_or(1).max(1) as f32;
    let mut kids = vec![0u8; cells.len()];
    for c in &cells {
        if c.2 >= 0 {
            kids[c.2 as usize] = kids[c.2 as usize].saturating_add(1);
        }
    }
    let ramp = [ink.trunk, ink.limb, ink.leaf, ink.leaf_lit];
    for (i, c) in cells.iter().enumerate() {
        let lvl = ((c.3 as f32 / max_gen) * 3.99) as usize;
        let mut fg = ramp[lvl.min(3)];
        let ch = if i <= stem as usize {
            '┃'
        } else if kids[i] == 0 {
            if rng.random::<f32>() < g.fruit * 0.6 {
                fg = ink.fruit;
                '✶'
            } else if lvl >= 2 {
                '•'
            } else {
                '◦'
            }
        } else {
            let p = &cells[c.2 as usize];
            let (dx, dy) = (c.0 - p.0, c.1 - p.1);
            if dx == 0 {
                '│'
            } else {
                '─'
            }
        };
        b.put(c.0, c.1, ch, fg);
    }
    b.put(rx - 1, ry, '╱', ink.root);
    b.put(rx + 1, ry, '╲', ink.root);
    for dx in [-3, -2, 2, 3] {
        if rng.random::<f32>() < 0.5 {
            b.put_soft(rx + dx, ry, '◦', ink.leaf_dark);
        }
    }
}

/// Grow one species into `grid`, clipped to `plot`; the root sits at the plot's bottom center.
pub(crate) fn grow(sp: Species, grid: &mut Grid, plot: Rect, energy: f32, ink: &Ink, g: &Growth, rng: &mut StdRng) {
    if plot.w < 3 || plot.h < 3 {
        return;
    }
    let mut b = Brush::new(grid, plot);
    let energy = energy.clamp(0.15, 1.0);
    match sp {
        Species::Colonize => grow_colonize(&mut b, energy, ink, g, rng),
        Species::Banyan => grow_banyan(&mut b, energy, ink, g, rng),
        Species::Mangrove => grow_mangrove(&mut b, energy, ink, g, rng),
        Species::Baobab => grow_baobab(&mut b, energy, ink, g, rng),
        Species::Coral => grow_coral(&mut b, energy, ink, g, rng),
    }
}

pub(crate) fn hash01(x: i32, y: i32, k: u32, seed: u64) -> f32 {
    let mut h = (x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (y as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9)
        ^ (k as u64).wrapping_mul(0x94D0_49BB_1331_11EB)
        ^ seed;
    h ^= h >> 31;
    h = h.wrapping_mul(0xD6E8_FEB8_6659_FD93);
    h ^= h >> 32;
    (h & 0xFF_FFFF) as f32 / 16_777_216.0
}

// ── sample sheet ────────────────────────────────────────────────────

pub(crate) struct SheetKnobs {
    pub energy: f32,
    pub fruit: f32,
    pub branch: f32,
    pub leaf: f32,
    pub roots: f32,
    pub scrub: f32,
    pub flicker: f32,
    pub sway: f32,
}

impl SheetKnobs {
    pub(crate) fn from_env() -> Self {
        SheetKnobs {
            energy: param_f32("ENERGY", 0.9),
            fruit: param_f32("FRUIT", 0.25),
            branch: param_f32("BRANCH", 0.7),
            leaf: param_f32("LEAF", 0.9),
            roots: param_f32("ROOTS", 0.7),
            scrub: param_f32("SCRUB", 0.55),
            flicker: param_f32("FLICKER", 1.0),
            sway: param_f32("SWAY", 1.0),
        }
    }
}

/// Row bands of the sheet: (top, height) for the full row and the scrub row.
fn sheet_rows(h: usize) -> [(usize, usize); 2] {
    let cell_h0 = ((h as f32 * 0.58) as usize).max(12);
    let cell_h1 = h.saturating_sub(cell_h0).max(9);
    [(0, cell_h0), (cell_h0, cell_h1)]
}

struct SheetCache {
    key: (usize, usize, u64, [u32; 6], [Color; 5]),
    page: Grid,
}

thread_local! {
    static SHEET: RefCell<Option<SheetCache>> = RefCell::new(None);
}

fn sheet_key(w: usize, h: usize, seed: u64, palette: &[Color; 5], k: &SheetKnobs) -> (usize, usize, u64, [u32; 6], [Color; 5]) {
    let bits = [k.energy.to_bits(), k.fruit.to_bits(), k.branch.to_bits(), k.leaf.to_bits(), k.roots.to_bits(), k.scrub.to_bits()];
    (w, h, seed, bits, *palette)
}

/// Grow the sheet once per (size, seed, knobs, palette); each frame copies it and flickers.
pub(crate) fn draw_fable_1_trees(grid: &mut Grid, w: usize, h: usize, seed: u64, palette: &[Color; 5], t: f32, k: &SheetKnobs) {
    let key = sheet_key(w, h, seed, palette, k);
    SHEET.with(|cell| {
        let mut slot = cell.borrow_mut();
        let stale = slot.as_ref().map(|c| c.key != key).unwrap_or(true);
        if stale {
            let mut page: Grid = vec![vec![Cell::blank(); w]; h];
            grow_sheet(&mut page, w, h, seed, palette, k);
            *slot = Some(SheetCache { key, page });
        }
        let page = &slot.as_ref().unwrap().page;
        measure_layer("fable-1-trees", "sway", || copy_swayed(grid, page, w, h, t, k));
    });
    flicker(grid, w, h, seed, t, k);
}

/// Copy the grown page into the frame, leaning each tree column by a slow wind.
fn copy_swayed(grid: &mut Grid, page: &Grid, w: usize, h: usize, t: f32, k: &SheetKnobs) {
    let cols = SPECIES.len();
    let cell_w = (w / cols).max(1);
    let rows = sheet_rows(h);
    let amp = if t > 0.0 { k.sway.max(0.0) } else { 0.0 };
    let mut shifts = [0i32; 5];
    for (y, (dst, src)) in grid.iter_mut().zip(page.iter()).enumerate().take(h) {
        let n = w.min(dst.len()).min(src.len());
        let (top, rh) = if y < rows[1].0 { rows[0] } else { rows[1] };
        let gy = top + rh - 2;
        let frac = if y < gy { (gy - y) as f32 / rh.max(1) as f32 } else { 0.0 };
        if amp <= 0.0 || frac <= 0.0 {
            dst[..n].copy_from_slice(&src[..n]);
            continue;
        }
        for (i, s) in shifts.iter_mut().enumerate() {
            let wind = (t * 0.35 + i as f32 * 1.3).sin() + 0.4 * (t * 0.9 + i as f32 * 0.7).sin();
            *s = (amp * wind * frac.powf(1.5)).round() as i32;
        }
        for (x, cell) in dst.iter_mut().enumerate().take(n) {
            let i = (x / cell_w).min(cols - 1);
            let sx = x as i32 - shifts[i];
            let lo = (i * cell_w) as i32;
            let hi = if i == cols - 1 { n as i32 } else { lo + cell_w as i32 };
            *cell = if sx >= lo && sx < hi { src[sx as usize] } else { Cell::blank() };
        }
    }
}

fn grow_sheet(grid: &mut Grid, w: usize, h: usize, seed: u64, palette: &[Color; 5], k: &SheetKnobs) {
    let cols = SPECIES.len();
    if w < cols * 4 || h < 6 {
        return;
    }
    let cell_w = w / cols;
    let bands = sheet_rows(h);
    let rows = 2usize;
    let row_y = |row: usize| bands[row].0;
    let row_h = |row: usize| bands[row].1;
    let ground_fg = darken(palette[2], 10);
    measure_layer("fable-1-trees", "ground", || {
        for row in 0..rows {
            let gy = row_y(row) + row_h(row) - 2;
            if gy >= h {
                continue;
            }
            for x in 0..w {
                let u = hash01(x as i32, gy as i32, 3, seed);
                let ch = if u < 0.8 { '·' } else { '∙' };
                grid[gy][x] = Cell::new(ch, ground_fg);
            }
        }
    });
    let mut rng = StdRng::seed_from_u64(seed ^ 0x00FA_B1E5);
    let growth = Growth { fruit: k.fruit, branch: k.branch, leaf: k.leaf, roots: k.roots };
    measure_layer("fable-1-trees", "trees", || {
        for row in 0..rows {
            let energy = if row == 0 { k.energy } else { k.energy * k.scrub };
            for (i, sp) in SPECIES.iter().enumerate() {
                let px = i * cell_w;
                let py = row_y(row);
                let leaf = shift_hue(palette[1], (i as f64 - 2.0) * 16.0 + row as f64 * 8.0);
                let ink = Ink::from_base(palette[2], leaf, palette[3]);
                let plot = Rect { x: px + 1, y: py + 1, w: cell_w.saturating_sub(2), h: row_h(row).saturating_sub(3) };
                grow(*sp, grid, plot, energy, &ink, &growth, &mut rng);
            }
        }
    });
    measure_layer("fable-1-trees", "labels", || {
        let fg = darken(palette[4], 50);
        for row in 0..rows {
            let ly = row_y(row) + row_h(row) - 1;
            if ly >= h {
                continue;
            }
            for (i, sp) in SPECIES.iter().enumerate() {
                let label = sp.label();
                let lx = i * cell_w + cell_w / 2 - label.len() / 2;
                for (j, ch) in label.chars().enumerate() {
                    if lx + j < w {
                        grid[ly][lx + j] = Cell::new(ch, fg);
                    }
                }
            }
        }
    });
}

fn flicker(grid: &mut Grid, w: usize, h: usize, seed: u64, t: f32, k: &SheetKnobs) {
    measure_layer("fable-1-trees", "flicker", || {
        if t <= 0.0 || k.flicker <= 0.0 {
            return;
        }
        let frame = (t * k.flicker).floor() as u32 + 1;
        let phase = (t * k.flicker * TAU / 6.0).sin() * 0.5 + 0.5;
        for y in 0..h {
            for x in 0..w {
                let ch = grid[y][x].ch;
                let alt = match ch {
                    '▒' => '░',
                    '░' => '▒',
                    '◦' => '∙',
                    '∙' => '◦',
                    _ => continue,
                };
                if hash01(x as i32, y as i32, frame, seed) < 0.04 + 0.05 * phase {
                    grid[y][x].ch = alt;
                }
            }
        }
    });
}

pub(crate) fn cli_fable_1_trees(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
    let _ = (rng, term_w, term_h, mode, theme_name);
    let mut k = SheetKnobs::from_env();
    let pos: Vec<f32> = args.iter().skip(4).filter_map(|a| a.parse().ok()).collect();
    let slots: [&mut f32; 8] = [
        &mut k.energy,
        &mut k.fruit,
        &mut k.branch,
        &mut k.leaf,
        &mut k.roots,
        &mut k.scrub,
        &mut k.flicker,
        &mut k.sway,
    ];
    for (slot, v) in slots.into_iter().zip(pos.iter()) {
        *slot = *v;
    }
    draw_fable_1_trees(&mut grid, width, height, seed, &palette, t_anim, &k);
    (grid, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(w: usize, h: usize, seed: u64, t: f32) -> String {
        let mut g = vec![vec![Cell::blank(); w]; h];
        let p = crate::color::make_palette(seed);
        let k = SheetKnobs::from_env();
        draw_fable_1_trees(&mut g, w, h, seed, &p, t, &k);
        g.iter()
            .map(|row| row.iter().map(|c| c.ch).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn snapshot_fable_1_trees_80x24() {
        insta::assert_snapshot!("fable_1_trees_80x24", run(80, 24, 42, 0.0));
    }

    #[test]
    fn deterministic_and_seed_sensitive() {
        assert_eq!(run(110, 36, 42, 0.0), run(110, 36, 42, 0.0));
        assert_ne!(run(110, 36, 42, 0.0), run(110, 36, 7, 0.0));
    }

    #[test]
    fn every_species_draws_a_trunk_and_canopy() {
        for sp in SPECIES {
            let mut g = vec![vec![Cell::blank(); 30]; 20];
            let ink = Ink::from_base(crate::color::rgb(90, 70, 40), crate::color::rgb(60, 140, 60), crate::color::rgb(200, 80, 60));
            let growth = Growth { fruit: 0.3, branch: 0.7, leaf: 0.9, roots: 0.7 };
            let mut rng = StdRng::seed_from_u64(5);
            grow(sp, &mut g, Rect { x: 1, y: 1, w: 28, h: 18 }, 0.9, &ink, &growth, &mut rng);
            let filled = g.iter().flatten().filter(|c| c.ch != ' ').count();
            assert!(filled > 20, "{:?} drew only {} cells", sp, filled);
            let top_half = g.iter().take(10).flatten().filter(|c| c.ch != ' ').count();
            assert!(top_half > 3, "{:?} has no canopy in the top half", sp);
        }
    }

    #[test]
    fn tiny_plots_do_not_panic() {
        for sp in SPECIES {
            let mut g = vec![vec![Cell::blank(); 6]; 5];
            let ink = Ink::from_base(crate::color::rgb(90, 70, 40), crate::color::rgb(60, 140, 60), crate::color::rgb(200, 80, 60));
            let growth = Growth { fruit: 1.0, branch: 1.5, leaf: 1.5, roots: 1.0 };
            let mut rng = StdRng::seed_from_u64(9);
            grow(sp, &mut g, Rect { x: 0, y: 0, w: 6, h: 5 }, 1.0, &ink, &growth, &mut rng);
            grow(sp, &mut g, Rect { x: 0, y: 0, w: 2, h: 2 }, 1.0, &ink, &growth, &mut rng);
        }
    }
}
