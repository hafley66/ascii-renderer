//! opus-1 tree species: space colonization, mangrove prop roots, phyllotactic
//! rays, diffusion aggregation, recursive shelves; plus the opus-1-trees sheet.

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

/// Terminal cells are about twice as tall as wide.
pub(crate) const ASPECT: f32 = 2.0;

pub(crate) const BAND_TRUNK: u8 = 0;
pub(crate) const BAND_LIMB: u8 = 1;
pub(crate) const BAND_LEAF: u8 = 2;
pub(crate) const BAND_BLOOM: u8 = 3;
pub(crate) const BAND_ROOT: u8 = 4;
pub(crate) const BAND_AIR: u8 = 5;
pub(crate) const BANDS: usize = 6;
pub(crate) const TONES: usize = 32;
pub(crate) const PAL_STRIDE: usize = BANDS * TONES;

// ── canvas ──────────────────────────────────────────────────────────

/// A species writes band+tone, never a literal color. The mode owns the ramp.
#[derive(Clone, Copy)]
pub(crate) struct TCell {
    pub ch: char,
    pub band: u8,
    pub tone: u8,
}

impl TCell {
    #[inline]
    pub(crate) fn empty() -> Self {
        TCell { ch: '\0', band: 0, tone: 0 }
    }
}

pub(crate) struct Canvas {
    pub w: usize,
    pub h: usize,
    pub cells: Vec<TCell>,
}

impl Canvas {
    pub(crate) fn new(w: usize, h: usize) -> Self {
        let (w, h) = (w.max(1), h.max(1));
        Canvas { w, h, cells: vec![TCell::empty(); w * h] }
    }

    pub(crate) fn reset(&mut self, w: usize, h: usize) {
        let (w, h) = (w.max(1), h.max(1));
        self.w = w;
        self.h = h;
        self.cells.clear();
        self.cells.resize(w * h, TCell::empty());
        for c in self.cells.iter_mut() {
            *c = TCell::empty();
        }
    }

    #[inline]
    pub(crate) fn put(&mut self, x: i32, y: i32, ch: char, band: u8, tone: u8) {
        if x < 0 || y < 0 {
            return;
        }
        let (x, y) = (x as usize, y as usize);
        if x >= self.w || y >= self.h {
            return;
        }
        self.cells[y * self.w + x] = TCell { ch, band, tone: tone.min(31) };
    }

    #[inline]
    pub(crate) fn put_soft(&mut self, x: i32, y: i32, ch: char, band: u8, tone: u8) {
        if x < 0 || y < 0 {
            return;
        }
        let (xu, yu) = (x as usize, y as usize);
        if xu >= self.w || yu >= self.h {
            return;
        }
        let i = yu * self.w + xu;
        if self.cells[i].ch != '\0' {
            return;
        }
        self.cells[i] = TCell { ch, band, tone: tone.min(31) };
    }
}

// ── bake / blit ─────────────────────────────────────────────────────

/// One painted cell of a frozen scene. `pal` indexes the per-frame color table,
/// `group` selects the sway oscillator, `sway` is the 0..255 height weight.
#[derive(Clone, Copy)]
pub(crate) struct BakedCell {
    pub ch: char,
    pub x: u16,
    pub y: u16,
    pub pal: u16,
    pub group: u16,
    pub sway: u8,
}

/// Copy every painted canvas cell into the bake list, translated to grid space.
pub(crate) fn capture(
    canvas: &Canvas,
    ox: i32,
    oy: i32,
    slot: u16,
    group: u16,
    base_local: i32,
    out: &mut Vec<BakedCell>,
) {
    let span = base_local.max(1) as f32;
    let slot_off = slot * PAL_STRIDE as u16;
    for ly in 0..canvas.h {
        let up = ((base_local - ly as i32) as f32 / span).clamp(0.0, 1.0);
        let sway = (255.0 * up * up.sqrt()) as u8;
        let gy = oy + ly as i32;
        if gy < 0 || gy > u16::MAX as i32 {
            continue;
        }
        let row = &canvas.cells[ly * canvas.w..(ly + 1) * canvas.w];
        for (lx, tc) in row.iter().enumerate() {
            if tc.ch == '\0' {
                continue;
            }
            let gx = ox + lx as i32;
            if gx < 0 || gx > u16::MAX as i32 {
                continue;
            }
            out.push(BakedCell {
                ch: tc.ch,
                x: gx as u16,
                y: gy as u16,
                pal: slot_off + tc.band as u16 * TONES as u16 + tc.tone as u16,
                group,
                sway,
            });
        }
    }
}

/// Paint a bake list. `offs` is per-group sway in cells scaled by 65536; the
/// per-cell weight adds another factor of 255, so the product shifts by 24.
pub(crate) fn blit(grid: &mut Grid, w: usize, h: usize, cells: &[BakedCell], lut: &[Color], offs: &[i32]) {
    for c in cells {
        let dx = (offs[c.group as usize] * c.sway as i32) >> 24;
        let x = c.x as i32 + dx;
        let y = c.y as usize;
        if x < 0 || x as usize >= w || y >= h {
            continue;
        }
        grid[y][x as usize] = Cell::new(c.ch, lut[c.pal as usize]);
    }
}

/// Append one 32-step ramp to a color table.
pub(crate) fn ramp(out: &mut Vec<Color>, lo: Color, hi: Color) {
    for t in 0..TONES {
        out.push(lerp_color(lo, hi, t as f32 / (TONES - 1) as f32));
    }
}

// ── shared geometry helpers ─────────────────────────────────────────

#[inline]
pub(crate) fn hash2(x: i32, y: i32, s: u32) -> u32 {
    let mut h = (x as u32).wrapping_mul(0x9E37_79B1)
        ^ (y as u32).wrapping_mul(0x85EB_CA77)
        ^ s.wrapping_mul(0xC2B2_AE3D);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2545_F491);
    h ^ (h >> 13)
}

#[inline]
pub(crate) fn hashf(x: i32, y: i32, s: u32) -> f32 {
    (hash2(x, y, s) & 0xFFFF) as f32 / 65535.0
}

fn glyph_for(dx: f32, dy: f32, heavy: bool) -> char {
    let a = dx.abs();
    let b = dy.abs();
    if b > a * 1.9 {
        if heavy { '┃' } else { '│' }
    } else if a > b * 2.6 {
        '─'
    } else if dx * dy < 0.0 {
        '╱'
    } else {
        '╲'
    }
}

fn stroke(c: &mut Canvas, x0: f32, y0: f32, x1: f32, y1: f32, band: u8, tone: u8, heavy: bool) {
    let dx = x1 - x0;
    let dy = y1 - y0;
    let steps = (dx.abs().max(dy.abs()).ceil() as i32).clamp(1, 512);
    let ch = glyph_for(dx / ASPECT, dy, heavy);
    for s in 0..=steps {
        let f = s as f32 / steps as f32;
        c.put((x0 + dx * f).round() as i32, (y0 + dy * f).round() as i32, ch, band, tone);
    }
}

/// A tapering trunk column. Fills `xs` with the center x for every row from
/// `top_y` (index 0) down to `base_y`.
fn draw_column(
    c: &mut Canvas,
    top_y: i32,
    base_y: i32,
    base_x: f32,
    lean: f32,
    wobble: f32,
    thick: f32,
    rng: &mut StdRng,
    xs: &mut Vec<i32>,
) {
    xs.clear();
    let span = (base_y - top_y).max(1) as f32;
    let mut drift = 0.0f32;
    for y in top_y..=base_y {
        let f = (base_y - y) as f32 / span;
        drift += (rng.random::<f32>() - 0.5) * wobble;
        drift = drift.clamp(-span * 0.25, span * 0.25);
        let x = base_x + lean * f * f + drift;
        let xi = x.round() as i32;
        xs.push(xi);
        let half = ((thick * (1.0 - f * 0.7)).round() as i32).max(0);
        let tone = (6.0 + f * 12.0) as u8;
        for d in -half..=half {
            let ch = if d == 0 && half > 0 { '┃' } else { '│' };
            c.put(xi + d, y, ch, BAND_TRUNK, tone);
        }
        if half > 0 && rng.random::<f32>() < 0.07 {
            c.put(xi, y, '┼', BAND_TRUNK, tone + 6);
        }
    }
}

/// Repaint a column already sampled into `xs`, so it sits over later art.
fn redraw_column(c: &mut Canvas, top_y: i32, xs: &[i32], thick: f32, skip_top: usize) {
    let span = xs.len().max(1) as f32;
    for (k, &xi) in xs.iter().enumerate().skip(skip_top) {
        let f = 1.0 - k as f32 / span;
        let half = ((thick * (1.0 - f * 0.7)).round() as i32).max(0);
        let tone = (6.0 + f * 12.0) as u8;
        for d in -half..=half {
            let ch = if d == 0 && half > 0 { '┃' } else { '│' };
            c.put(xi + d, top_y + k as i32, ch, BAND_TRUNK, tone);
        }
    }
}

/// Buttress flares, knuckle arcs, a mound, or a surface root mat.
pub(crate) fn draw_roots(c: &mut Canvas, x: i32, y: i32, spread: i32, style: usize, rng: &mut StdRng) {
    let spread = spread.max(2);
    match style % 4 {
        0 => {
            let n = 2 + rng.random_range(0..3u32) as i32;
            for k in 0..n {
                let side = if k % 2 == 0 { 1 } else { -1 };
                let len = ((spread as f32) * (0.30 + rng.random::<f32>() * 0.40)) as i32;
                let rise = ((len as f32 / ASPECT).round() as i32).clamp(1, 7);
                stroke(
                    c,
                    x as f32,
                    (y - rise) as f32,
                    (x + side * len) as f32,
                    y as f32,
                    BAND_ROOT,
                    9 + k as u8,
                    false,
                );
                c.put(x + side * len, y, if side < 0 { '╴' } else { '╶' }, BAND_ROOT, 6);
            }
        }
        1 => {
            let l = ((spread as f32) * (0.4 + rng.random::<f32>() * 0.5)) as i32;
            let r = ((spread as f32) * (0.4 + rng.random::<f32>() * 0.5)) as i32;
            c.put(x, y, '┴', BAND_ROOT, 14);
            for d in 1..=l {
                c.put(x - d, y, if d == l { '╰' } else { '─' }, BAND_ROOT, 12 - (d as u8).min(6));
            }
            for d in 1..=r {
                c.put(x + d, y, if d == r { '╯' } else { '─' }, BAND_ROOT, 12 - (d as u8).min(6));
            }
        }
        2 => {
            for dy in 0..2i32 {
                let hw = (spread / 2 + dy).max(1);
                for d in -hw..=hw {
                    let g = if d.abs() * 2 < hw { '▒' } else { '░' };
                    c.put(x + d, y + dy, g, BAND_ROOT, (11 - dy * 3).max(3) as u8);
                }
            }
        }
        _ => {
            for d in -spread..=spread {
                if hashf(x + d, y, 7717) < 0.55 {
                    continue;
                }
                let g = if (d.abs() & 1) == 0 { '∙' } else { '·' };
                c.put(x + d, y, g, BAND_ROOT, 7 + (spread - d.abs()).clamp(0, 8) as u8);
            }
            c.put(x, y, '┴', BAND_ROOT, 15);
        }
    }
}

// ── species ─────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Species {
    Venation,
    Mangrove,
    Phyllotaxis,
    Aggregate,
    Shelf,
}

pub(crate) const SPECIES: [Species; 5] = [
    Species::Venation,
    Species::Mangrove,
    Species::Phyllotaxis,
    Species::Aggregate,
    Species::Shelf,
];

impl Species {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Species::Venation => "venation",
            Species::Mangrove => "mangrove",
            Species::Phyllotaxis => "phyllotaxis",
            Species::Aggregate => "aggregate",
            Species::Shelf => "shelf",
        }
    }

    pub(crate) fn from_index(i: usize) -> Species {
        SPECIES[i % SPECIES.len()]
    }
}

/// Everything a species needs. The rect is in canvas-local coordinates.
pub(crate) struct Plot {
    pub rect: Rect,
    pub energy: f32,
    pub fruit: f32,
    pub branch: f32,
    pub roots: usize,
    pub detail: f32,
    pub bare: f32,
}

impl Plot {
    pub(crate) fn root(&self) -> (i32, i32) {
        (
            self.rect.x as i32 + self.rect.w as i32 / 2,
            self.rect.y as i32 + self.rect.h as i32 - 1,
        )
    }

    pub(crate) fn crown_top(&self) -> i32 {
        let top = self.rect.y as i32;
        let ry = self.root().1;
        ry - ((ry - top) as f32 * self.energy.clamp(0.15, 1.0)) as i32
    }

    pub(crate) fn spread(&self) -> i32 {
        ((self.rect.w as f32 / 2.0 - 1.0) * self.energy.clamp(0.25, 1.0)) as i32
    }
}

pub(crate) fn grow_species(sp: Species, c: &mut Canvas, p: &Plot, rng: &mut StdRng) {
    match sp {
        Species::Venation => grow_venation(c, p, rng),
        Species::Mangrove => grow_mangrove(c, p, rng),
        Species::Phyllotaxis => grow_phyllotaxis(c, p, rng),
        Species::Aggregate => grow_aggregate(c, p, rng),
        Species::Shelf => grow_shelf(c, p, rng),
    }
}

/// Seeded three-term radial lobe. Makes every crown lopsided in its own way.
#[derive(Clone, Copy)]
struct Lobe {
    k1: f32,
    k2: f32,
    p1: f32,
    p2: f32,
    a1: f32,
    a2: f32,
}

impl Lobe {
    fn seeded(rng: &mut StdRng) -> Self {
        Lobe {
            k1: 1.0 + rng.random_range(0..3u32) as f32,
            k2: 3.0 + rng.random_range(0..4u32) as f32,
            p1: rng.random::<f32>() * TAU,
            p2: rng.random::<f32>() * TAU,
            a1: 0.16 + rng.random::<f32>() * 0.20,
            a2: 0.06 + rng.random::<f32>() * 0.12,
        }
    }

    #[inline]
    fn at(&self, th: f32) -> f32 {
        (1.0 + self.a1 * (self.k1 * th + self.p1).sin() + self.a2 * (self.k2 * th + self.p2).sin()).max(0.35)
    }
}

// --- 1. venation: space colonization -------------------------------

struct VNode {
    x: f32,
    y: f32,
    parent: i32,
    mass: u32,
}

fn grow_venation(c: &mut Canvas, p: &Plot, rng: &mut StdRng) {
    let (rx, ry) = p.root();
    let top = p.crown_top();
    let height = (ry - top).max(6) as f32;
    let spread = p.spread().max(3) as f32;
    let lean = (rng.random::<f32>() - 0.5) * spread * 0.5;

    let base_y = ry - (height * (p.bare + rng.random::<f32>() * 0.10)) as i32;
    let mut xs: Vec<i32> = Vec::new();
    let thick = (spread * 0.14).clamp(0.0, 3.0);
    draw_column(c, base_y, ry, rx as f32, lean * 0.4, 0.30, thick, rng, &mut xs);
    draw_roots(c, *xs.last().unwrap_or(&rx), ry, (spread * 0.6) as i32, p.roots, rng);

    let hub_x = xs.first().copied().unwrap_or(rx) as f32;
    let crown_cy = (top as f32 + base_y as f32) * 0.5;
    let crown_ry = ((base_y - top) as f32 * 0.5).max(2.0);
    let crown_rx = spread.max(2.0);
    let ccx = hub_x + lean;
    let lobe = Lobe::seeded(rng);

    let area = crown_rx * crown_ry;
    let count = ((area * 2.8 * p.detail) as usize).clamp(60, 480);
    let mut att: Vec<(f32, f32)> = Vec::with_capacity(count);
    for _ in 0..count {
        let th = rng.random::<f32>() * TAU;
        let rr = rng.random::<f32>().sqrt() * lobe.at(th);
        att.push((ccx + th.cos() * rr * crown_rx, crown_cy - th.sin() * rr * crown_ry));
    }

    let step = (crown_ry * 0.26).clamp(0.85, 2.4);
    let ri = step * 5.5;
    let rk = step * 1.5;
    let node_cap = 640usize;

    let mut nodes: Vec<VNode> = Vec::with_capacity(node_cap);
    let chain = ((base_y - crown_cy as i32).max(1)).min(48);
    for k in 0..=chain {
        nodes.push(VNode {
            x: hub_x + lean * (k as f32 / chain.max(1) as f32),
            y: (base_y - k) as f32,
            parent: if k == 0 { -1 } else { k - 1 },
            mass: 1,
        });
    }

    let bx0 = ccx - crown_rx * 1.6;
    let by0 = top as f32 - crown_ry * 0.4;
    let bw = (((crown_rx * 3.2) / (ri * ASPECT)).ceil() as usize).clamp(1, 64);
    let bh = ((((base_y as f32 - by0) + crown_ry) / ri).ceil() as usize).clamp(1, 64);
    let mut buckets: Vec<Vec<u32>> = vec![Vec::new(); bw * bh];
    let bucket_of = |x: f32, y: f32| -> (usize, usize) {
        let ix = (((x - bx0) / (ri * ASPECT)) as isize).clamp(0, bw as isize - 1) as usize;
        let iy = (((y - by0) / ri) as isize).clamp(0, bh as isize - 1) as usize;
        (ix, iy)
    };
    for (i, n) in nodes.iter().enumerate() {
        let (ix, iy) = bucket_of(n.x, n.y);
        buckets[iy * bw + ix].push(i as u32);
    }

    let mut alive = vec![true; att.len()];
    let mut remaining = att.len();
    let mut pull: Vec<(f32, f32, u32)> = Vec::with_capacity(node_cap);
    let mut iters = 0;

    while remaining > 0 && nodes.len() < node_cap && iters < 90 {
        iters += 1;
        pull.clear();
        pull.resize(nodes.len(), (0.0, 0.0, 0));

        for (ai, a) in att.iter().enumerate() {
            if !alive[ai] {
                continue;
            }
            let (ix, iy) = bucket_of(a.0, a.1);
            let mut best = -1i32;
            let mut bestd = ri * ri;
            for by in iy.saturating_sub(1)..(iy + 2).min(bh) {
                for bxi in ix.saturating_sub(1)..(ix + 2).min(bw) {
                    for &ni in &buckets[by * bw + bxi] {
                        let n = &nodes[ni as usize];
                        let dx = (a.0 - n.x) / ASPECT;
                        let dy = a.1 - n.y;
                        let d = dx * dx + dy * dy;
                        if d < bestd {
                            bestd = d;
                            best = ni as i32;
                        }
                    }
                }
            }
            if best >= 0 {
                let n = &nodes[best as usize];
                let dx = (a.0 - n.x) / ASPECT;
                let dy = a.1 - n.y;
                let m = (dx * dx + dy * dy).sqrt().max(0.001);
                let e = &mut pull[best as usize];
                e.0 += dx / m;
                e.1 += dy / m;
                e.2 += 1;
            }
        }

        let before = nodes.len();
        for i in 0..before {
            let (px, py, k) = pull[i];
            if k == 0 {
                continue;
            }
            if nodes.len() >= node_cap {
                break;
            }
            let jx = (rng.random::<f32>() - 0.5) * 0.35;
            let jy = (rng.random::<f32>() - 0.5) * 0.20;
            let mut ux = px / k as f32 + jx;
            let mut uy = py / k as f32 - 0.22 + jy;
            let m = (ux * ux + uy * uy).sqrt().max(0.001);
            ux /= m;
            uy /= m;
            let nx = nodes[i].x + ux * step * ASPECT;
            let ny = nodes[i].y + uy * step;
            let idx = nodes.len() as u32;
            nodes.push(VNode { x: nx, y: ny, parent: i as i32, mass: 1 });
            let (ix, iy) = bucket_of(nx, ny);
            buckets[iy * bw + ix].push(idx);
        }
        if nodes.len() == before {
            break;
        }

        for (ai, a) in att.iter().enumerate() {
            if !alive[ai] {
                continue;
            }
            let (ix, iy) = bucket_of(a.0, a.1);
            let mut hit = false;
            'kill: for by in iy.saturating_sub(1)..(iy + 2).min(bh) {
                for bxi in ix.saturating_sub(1)..(ix + 2).min(bw) {
                    for &ni in &buckets[by * bw + bxi] {
                        let n = &nodes[ni as usize];
                        let dx = (a.0 - n.x) / ASPECT;
                        let dy = a.1 - n.y;
                        if dx * dx + dy * dy < rk * rk {
                            hit = true;
                            break 'kill;
                        }
                    }
                }
            }
            if hit {
                alive[ai] = false;
                remaining -= 1;
            }
        }
    }

    for i in (1..nodes.len()).rev() {
        let (m, par) = (nodes[i].mass, nodes[i].parent);
        if par >= 0 {
            nodes[par as usize].mass += m;
        }
    }

    let heavy_cut = (nodes.len() as u32 / 8).max(6);
    for i in 1..nodes.len() {
        let par = nodes[i].parent;
        if par < 0 {
            continue;
        }
        let m = nodes[i].mass;
        let (band, tone) = if m > heavy_cut {
            (BAND_TRUNK, 14u8)
        } else if m > 3 {
            (BAND_LIMB, (10 + (12 - m.min(12)) as u8).min(24))
        } else {
            (BAND_LEAF, 18)
        };
        let ax = nodes[par as usize].x;
        let ay = nodes[par as usize].y;
        let bxp = nodes[i].x;
        let byp = nodes[i].y;
        stroke(c, ax, ay, bxp, byp, band, tone, m > heavy_cut * 2);
    }

    for i in 1..nodes.len() {
        if nodes[i].mass > 5 {
            continue;
        }
        let (x, y) = (nodes[i].x.round() as i32, nodes[i].y.round() as i32);
        for dy in -1i32..=1 {
            for dx in -2i32..=2 {
                let far = dx.abs() + dy.abs() * 2;
                let n = hashf(x + dx, y + dy, 8803);
                if n < 0.06 * far as f32 {
                    continue;
                }
                let g = if n > 0.80 {
                    '▒'
                } else if n > 0.45 {
                    '░'
                } else {
                    '∙'
                };
                c.put_soft(x + dx, y + dy, g, BAND_LEAF, (15.0 + n * 11.0) as u8);
            }
        }
    }

    for i in 1..nodes.len() {
        if nodes[i].mass != 1 {
            continue;
        }
        let (x, y) = (nodes[i].x.round() as i32, nodes[i].y.round() as i32);
        let r = hashf(x, y, 4241);
        let g = if r < 0.34 {
            '◆'
        } else if r < 0.68 {
            '◇'
        } else {
            '∙'
        };
        c.put(x, y, g, BAND_LEAF, (22.0 + r * 9.0) as u8);
        if r < p.fruit {
            c.put(x, y, '●', BAND_BLOOM, 24);
        }
    }
}

// --- 2. mangrove: prop-root arches ---------------------------------

fn grow_mangrove(c: &mut Canvas, p: &Plot, rng: &mut StdRng) {
    let (rx, ry) = p.root();
    let top = p.crown_top();
    let height = (ry - top).max(6) as f32;
    let spread = p.spread().max(3) as f32;

    let stilt = (height * (0.20 + rng.random::<f32>() * 0.14)).max(2.0) as i32;
    let base_y = ry - stilt;
    let crown_base = (top + (height * (1.0 - p.bare - rng.random::<f32>() * 0.08).clamp(0.25, 0.86)) as i32)
        .min(base_y - 1)
        .max(top + 1);
    let lean = (rng.random::<f32>() - 0.5) * spread * 0.4;

    let mut xs: Vec<i32> = Vec::new();
    let thick = (spread * 0.11).clamp(0.0, 2.0);
    draw_column(c, crown_base, base_y, rx as f32, lean, 0.24, thick, rng, &mut xs);
    let trunk_x = |y: i32| -> i32 {
        if xs.is_empty() {
            return rx;
        }
        let i = (y - crown_base).clamp(0, xs.len() as i32 - 1) as usize;
        xs.get(i).copied().unwrap_or(rx)
    };

    let props = 3 + (rng.random_range(0..4u32) as f32 * p.energy) as i32;
    for k in 0..props {
        let side: f32 = if k % 2 == 0 { 1.0 } else { -1.0 };
        let reach = spread * (0.35 + rng.random::<f32>() * 0.75) * side;
        let ay = base_y - (rng.random::<f32>() * height * 0.16) as i32;
        let ax = trunk_x(ay) as f32;
        let drop = (ry - ay).max(1) as f32;
        let steps = (drop + reach.abs()).ceil().clamp(2.0, 200.0) as i32;
        let mut px = ax;
        let mut py = ay as f32;
        for s in 1..=steps {
            let f = s as f32 / steps as f32;
            let a = f * std::f32::consts::FRAC_PI_2;
            let nx = ax + reach * a.sin();
            let ny = ay as f32 + drop * (1.0 - a.cos());
            let g = glyph_for(nx - px, ny - py, false);
            c.put(nx.round() as i32, ny.round() as i32, g, BAND_ROOT, 8 + (f * 8.0) as u8);
            px = nx;
            py = ny;
        }
        c.put(px.round() as i32, ry, '┴', BAND_ROOT, 15);
    }
    draw_roots(c, trunk_x(base_y), ry, (spread * 0.4) as i32, p.roots + 3, rng);

    let ccx = trunk_x(crown_base) as f32 + lean * 0.6;
    let ccy = (top + crown_base) as f32 * 0.5;
    let crx = spread * 1.20;
    let cry = ((crown_base - top) as f32 * 0.5).max(1.5);
    let lobe = Lobe::seeded(rng);
    let salt = rng.random_range(0..9999u32);

    let x0 = (ccx - crx).floor() as i32;
    let x1 = (ccx + crx).ceil() as i32;
    for y in top..=crown_base {
        for x in x0..=x1 {
            let nx = (x as f32 - ccx) / crx;
            let ny = (y as f32 - ccy) / cry;
            let rr = (nx * nx + ny * ny).sqrt();
            if rr < 0.001 {
                continue;
            }
            let th = (-ny).atan2(nx);
            let edge = lobe.at(th);
            let d = rr / edge;
            if d > 1.0 {
                continue;
            }
            let n = hashf(x, y, salt);
            if d > 0.72 && n < (d - 0.72) * 3.0 {
                continue;
            }
            let g = if d < 0.36 {
                '▒'
            } else if d < 0.70 {
                '░'
            } else if n < 0.5 {
                '·'
            } else {
                '∙'
            };
            let tone = (26.0 - d * 13.0) as u8;
            c.put(x, y, g, BAND_LEAF, tone);
            if n > 0.94 {
                c.put(x, y, '◆', BAND_LEAF, 29);
            }
        }
    }

    let limbs = 3 + (p.branch * 4.0) as i32;
    for k in 0..limbs {
        let f = (k as f32 + 0.5) / limbs as f32;
        let th = 0.30 + f * 2.55 + (rng.random::<f32>() - 0.5) * 0.25;
        let reach = 0.55 + rng.random::<f32>() * 0.45;
        stroke(
            c,
            trunk_x(crown_base) as f32,
            crown_base as f32,
            ccx + th.cos() * crx * reach,
            ccy - th.sin() * cry * reach * 1.3,
            BAND_LIMB,
            13 + k as u8,
            false,
        );
    }

    let hangers = (crx * p.fruit * 2.0) as i32;
    for _ in 0..hangers {
        let x = (ccx + (rng.random::<f32>() - 0.5) * crx * 1.7).round() as i32;
        let y = crown_base + 1;
        let len = 1 + rng.random_range(0..3u32) as i32;
        for d in 0..len {
            c.put(x, y + d, '│', BAND_BLOOM, 16);
        }
        c.put(x, y + len, '◆', BAND_BLOOM, 27);
    }
}

// --- 3. phyllotaxis: golden-angle ray crown -------------------------

fn grow_phyllotaxis(c: &mut Canvas, p: &Plot, rng: &mut StdRng) {
    let (rx, ry) = p.root();
    let top = p.crown_top();
    let height = (ry - top).max(6) as f32;
    let spread = p.spread().max(3) as f32;

    let cry = (height * ((1.0 - p.bare) * 0.5).clamp(0.12, 0.45)).max(2.0);
    let hub_y = top + cry as i32;
    let bend = (rng.random::<f32>() - 0.5) * spread * 0.55;
    let mut xs: Vec<i32> = Vec::new();
    let thick = (spread * 0.13).clamp(0.0, 3.0);
    draw_column(c, hub_y, ry, rx as f32, bend, 0.18, thick, rng, &mut xs);
    draw_roots(c, *xs.last().unwrap_or(&rx), ry, (spread * 0.65) as i32, p.roots + 1, rng);

    let hub_x = xs.first().copied().unwrap_or(rx) as f32;
    let crx = spread * 1.05;
    let lobe = Lobe::seeded(rng);
    let n = ((crx * cry * 3.4 * p.detail) as usize).clamp(60, 1400);
    let rays = (5.0 + p.branch * 16.0) as usize;
    let ray_stride = (n / rays.max(1)).max(1);
    let spin = rng.random::<f32>() * TAU;

    for i in 0..n {
        let th = spin + i as f32 * 2.399_963_2;
        let rn = ((i as f32 + 0.5) / n as f32).sqrt();
        let edge = lobe.at(th);
        let px = hub_x + th.cos() * rn * crx * edge;
        let py = hub_y as f32 - th.sin() * rn * cry * edge;
        if i % ray_stride == 0 && rn > 0.25 {
            stroke(c, hub_x, hub_y as f32, px, py, BAND_LIMB, (10.0 + rn * 8.0) as u8, rn < 0.55);
        }
        let g = if rn < 0.32 {
            '▒'
        } else if rn < 0.58 {
            '░'
        } else if rn < 0.80 {
            '∙'
        } else if rn < 0.93 {
            '◦'
        } else {
            '◇'
        };
        c.put(px.round() as i32, py.round() as i32, g, BAND_LEAF, (12.0 + rn * 19.0) as u8);
        if rn > 0.72 && hashf(px as i32, py as i32, 991) < p.fruit {
            c.put(px.round() as i32, py.round() as i32, '●', BAND_BLOOM, 26);
        }
    }
    c.put(hub_x.round() as i32, hub_y, '┼', BAND_TRUNK, 18);
}

// --- 4. aggregate: diffusion-limited sticking ----------------------

fn grow_aggregate(c: &mut Canvas, p: &Plot, rng: &mut StdRng) {
    let (rx, ry) = p.root();
    let top = p.crown_top();
    let height = (ry - top).max(6) as f32;
    let spread = p.spread().max(3) as f32;

    let w = p.rect.w.max(3);
    let h = p.rect.h.max(4);
    let ox = p.rect.x as i32;
    let oy = p.rect.y as i32;
    let mut occ = vec![0u16; w * h];
    let mut order: Vec<(i32, i32, u16)> = Vec::new();

    let crown_bottom = ry - (height * p.bare) as i32;
    let trunk_top = (top + crown_bottom) / 2 + (rng.random::<f32>() * height * 0.10) as i32;
    let lean = (rng.random::<f32>() - 0.5) * spread * 0.45;
    let mut xs: Vec<i32> = Vec::new();
    let thick = (spread * 0.10).clamp(0.0, 2.0);
    draw_column(c, trunk_top, ry, rx as f32, lean, 0.28, thick, rng, &mut xs);

    for (k, &tx) in xs.iter().enumerate() {
        let (lx, ly) = (tx - ox, trunk_top + k as i32 - oy);
        if lx >= 0 && ly >= 0 && (lx as usize) < w && (ly as usize) < h {
            occ[ly as usize * w + lx as usize] = 1;
        }
    }

    let ccx = xs.first().copied().unwrap_or(rx) as f32;
    let ccy = (top + crown_bottom) as f32 * 0.5;
    let arx = spread * 1.15;
    let ary = ((crown_bottom - top) as f32 * 0.5).max(2.0);
    let walkers = ((arx * ary * 2.2 * p.detail) as usize).clamp(25, 480);
    let step_cap = 300;
    let drift = 0.22 + p.branch * 0.28;
    let mut stuck: u16 = 1;

    for _ in 0..walkers {
        let a = rng.random::<f32>() * TAU;
        let mut px = ccx + a.cos() * arx * 1.18;
        let mut py = ccy - a.sin() * ary * 1.18;
        for _ in 0..step_cap {
            let dxc = (ccx - px) / ASPECT;
            let dyc = ccy - py;
            let m = (dxc * dxc + dyc * dyc).sqrt().max(0.001);
            px += (rng.random::<f32>() - 0.5) * 2.6 + dxc / m * drift * ASPECT;
            py += (rng.random::<f32>() - 0.5) * 1.3 + dyc / m * drift;
            if py > ry as f32 || py < oy as f32 - 1.0 {
                break;
            }
            if (px - ccx).abs() > arx * 2.0 {
                break;
            }
            let (ix, iy) = (px.round() as i32, py.round() as i32);
            let (lx, ly) = (ix - ox, iy - oy);
            if lx < 1 || ly < 1 || lx as usize >= w - 1 || ly as usize >= h - 1 {
                continue;
            }
            let li = ly as usize * w + lx as usize;
            if occ[li] != 0 {
                continue;
            }
            let mut touch = false;
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    if occ[(ly + dy) as usize * w + (lx + dx) as usize] != 0 {
                        touch = true;
                    }
                }
            }
            if touch {
                stuck = stuck.saturating_add(1);
                occ[li] = stuck;
                order.push((ix, iy, stuck));
                break;
            }
        }
    }

    let last = stuck.max(2) as f32;
    for &(x, y, ord) in &order {
        let (lx, ly) = ((x - ox) as usize, (y - oy) as usize);
        let mut nb = 0;
        for dy in -1i32..=1 {
            for dx in -1i32..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let (nx, ny) = (lx as i32 + dx, ly as i32 + dy);
                if nx >= 0
                    && ny >= 0
                    && (nx as usize) < w
                    && (ny as usize) < h
                    && occ[ny as usize * w + nx as usize] != 0
                {
                    nb += 1;
                }
            }
        }
        let g = match nb {
            0 | 1 => '·',
            2 => '∙',
            3 | 4 => '░',
            5 | 6 => '▒',
            _ => '▓',
        };
        let age = ord as f32 / last;
        let (band, tone) = if age > 0.86 {
            (BAND_BLOOM, 26)
        } else if nb >= 4 {
            (BAND_LIMB, (10.0 + age * 10.0) as u8)
        } else {
            (BAND_LEAF, (12.0 + age * 17.0) as u8)
        };
        c.put(x, y, g, band, tone);
        if age > 0.93 && hashf(x, y, 3323) < p.fruit * 2.0 {
            c.put(x, y, '◇', BAND_BLOOM, 30);
        }
    }

    redraw_column(c, trunk_top, &xs, thick, 0);
    draw_roots(c, *xs.last().unwrap_or(&rx), ry, (spread * 0.55) as i32, p.roots + 2, rng);
}

// --- 5. shelf: recursive subdivision -------------------------------

fn subdivide(
    r: (i32, i32, i32, i32),
    depth: usize,
    max_depth: usize,
    branch: f32,
    out: &mut Vec<(i32, i32, i32, i32)>,
    rng: &mut StdRng,
) {
    let (x, y, w, h) = r;
    let keep = branch * (1.0 - depth as f32 / (max_depth + 1) as f32) + 0.20;
    let roll = rng.random::<f32>();
    if depth >= max_depth || w < 6 || h < 3 || (depth >= 2 && roll > keep) || out.len() > 60 {
        out.push(r);
        return;
    }
    if (w as f32) > (h as f32) * ASPECT {
        let cut = ((w as f32) * (0.32 + rng.random::<f32>() * 0.36)) as i32;
        subdivide((x, y, cut, h), depth + 1, max_depth, branch, out, rng);
        subdivide((x + cut, y, w - cut, h), depth + 1, max_depth, branch, out, rng);
    } else {
        let cut = ((h as f32) * (0.32 + rng.random::<f32>() * 0.36)).max(1.0) as i32;
        subdivide((x, y, w, cut), depth + 1, max_depth, branch, out, rng);
        subdivide((x, y + cut, w, h - cut), depth + 1, max_depth, branch, out, rng);
    }
}

fn grow_shelf(c: &mut Canvas, p: &Plot, rng: &mut StdRng) {
    let (rx, ry) = p.root();
    let top = p.crown_top();
    let height = (ry - top).max(6) as f32;
    let spread = p.spread().max(3) as f32;

    let trunk_top = top + (height * 0.05) as i32;
    let lean = (rng.random::<f32>() - 0.5) * spread * 0.35;
    let mut xs: Vec<i32> = Vec::new();
    let thick = (spread * 0.12).clamp(0.0, 3.0);
    draw_column(c, trunk_top, ry, rx as f32, lean, 0.12, thick, rng, &mut xs);
    let trunk_x = |y: i32| -> i32 {
        let i = (y - trunk_top).clamp(0, xs.len() as i32 - 1) as usize;
        xs.get(i).copied().unwrap_or(rx)
    };
    draw_roots(c, *xs.last().unwrap_or(&rx), ry, (spread * 0.7) as i32, p.roots, rng);

    let cw = (spread * 2.1) as i32;
    let ch = (ry - (height * p.bare) as i32) - top;
    if cw < 4 || ch < 3 {
        return;
    }
    let mut leaves: Vec<(i32, i32, i32, i32)> = Vec::new();
    let depth_cap = (3.0 + p.energy * 3.0) as usize;
    subdivide(
        (rx - cw / 2, top, cw, ch),
        0,
        depth_cap,
        p.branch.clamp(0.05, 0.95),
        &mut leaves,
        rng,
    );
    leaves.sort_by_key(|r| r.1);

    let lobe = Lobe::seeded(rng);
    let ccx = rx as f32 + lean * 0.7;
    let ccy = top as f32 + ch as f32 * 0.55;
    let crx = (cw as f32 * 0.56).max(2.0);
    let cry = (ch as f32 * 0.58).max(2.0);

    let mut scored: Vec<(f32, (i32, i32, i32, i32))> = leaves
        .iter()
        .filter(|r| r.2 >= 3 && r.3 >= 1)
        .map(|&r| {
            let nx = (r.0 + r.2 / 2) as f32 - ccx;
            let ny = (r.1 + r.3 - 1) as f32 - ccy;
            let (nx, ny) = (nx / crx, ny / cry);
            let rr = (nx * nx + ny * ny).sqrt();
            (rr / lobe.at((-ny).atan2(nx)), r)
        })
        .collect();
    scored.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    let keep = scored.iter().filter(|s| s.0 <= 1.0).count().max(3).min(scored.len());
    let mut kept: Vec<(i32, i32, i32, i32)> = scored.into_iter().take(keep).map(|s| s.1).collect();
    kept.sort_by_key(|r| r.1);

    let salt = rng.random_range(0..9999u32);
    for (i, &(bx, by, bw, bh)) in kept.iter().enumerate() {
        let shelf_y = by + bh - 1;
        let cx = bx + bw / 2;
        let tone = (26 - (i % 9) as i32).max(12) as u8;

        let capped = bw > 5;
        for d in 0..bw {
            let x = bx + d;
            let n = hashf(x, shelf_y, salt ^ 0x2B);
            let g = if capped && d == 0 {
                '╰'
            } else if capped && d == bw - 1 {
                '╯'
            } else if n > 0.88 {
                '┬'
            } else if n < 0.10 {
                '╴'
            } else {
                '─'
            };
            c.put(x, shelf_y, g, BAND_LIMB, tone);
        }
        let rows = ((bh - 1).min(3)).max(1);
        for k in 1..=rows {
            let y = shelf_y - k;
            let inset = (k * bw) / (rows * 4 + 2);
            for x in (bx + inset)..(bx + bw - inset) {
                let n = hashf(x, y, salt);
                if n < 0.18 + 0.30 * (k as f32 / rows as f32) {
                    continue;
                }
                let g = if k == 1 {
                    '▒'
                } else if n < 0.6 {
                    '░'
                } else {
                    '∙'
                };
                c.put(x, y, g, BAND_LEAF, (tone as i32 + 2 + k).min(31) as u8);
            }
        }
        if hashf(cx, shelf_y, salt ^ 77) < p.fruit {
            c.put(cx, shelf_y - 1, '●', BAND_BLOOM, 27);
        }

        let tx = trunk_x(shelf_y);
        let (lo, hi) = if cx < tx { (bx + bw - 1, tx) } else { (tx, bx) };
        if hi > lo {
            for x in lo..=hi {
                c.put_soft(x, shelf_y, '─', BAND_LIMB, (tone as i32 - 4).max(6) as u8);
            }
        }
        c.put(tx, shelf_y, if cx < tx { '┤' } else { '├' }, BAND_TRUNK, 20);
    }

    let skip = ((height * 0.28) as usize).min(xs.len().saturating_sub(1));
    redraw_column(c, trunk_top, &xs, thick, skip);
}

// ── opus-1-trees: the sample sheet ──────────────────────────────────

pub(crate) struct TreesKnobs {
    pub energy: f32,
    pub fruit: f32,
    pub branch: f32,
    pub scrub: f32,
    pub roots: f32,
    pub sway: f32,
    pub speed: f32,
    pub detail: f32,
    pub hue: f32,
    pub bare: f32,
}

impl TreesKnobs {
    pub(crate) fn from_env() -> Self {
        TreesKnobs {
            energy: param_f32("ENERGY", 0.88).clamp(0.2, 1.0),
            fruit: param_f32("FRUIT", 0.18).clamp(0.0, 1.0),
            branch: param_f32("BRANCH", 0.62).clamp(0.05, 1.0),
            scrub: param_f32("SCRUB", 0.58).clamp(0.2, 1.0),
            roots: param_f32("ROOTS", 0.0).clamp(0.0, 3.0),
            sway: param_f32("SWAY", 0.9).clamp(0.0, 4.0),
            speed: param_f32("SPEED", 1.0).clamp(0.0, 3.0),
            detail: param_f32("DETAIL", 1.0).clamp(0.2, 2.0),
            hue: param_f32("HUE", 0.0).clamp(-180.0, 180.0),
            bare: param_f32("BARE", 0.26).clamp(0.08, 0.60),
        }
    }
}

struct SheetBake {
    key: (usize, usize, u64, u32, u32, u32, u32, u32, u32),
    chrome: Vec<BakedCell>,
    trees: Vec<BakedCell>,
    leaves: Vec<u32>,
    phase: Vec<(f32, f32)>,
    slots: usize,
}

thread_local! {
    static SHEET: RefCell<Option<SheetBake>> = const { RefCell::new(None) };
}

fn q(v: f32) -> u32 {
    (v * 1000.0) as u32
}

fn sheet_key(w: usize, h: usize, seed: u64, k: &TreesKnobs) -> (usize, usize, u64, u32, u32, u32, u32, u32, u32) {
    (w, h, seed, q(k.energy), q(k.fruit), q(k.branch), q(k.scrub), q(k.roots), q(k.detail) ^ q(k.bare))
}

fn build_sheet_lut(palette: &[Color; 5], k: &TreesKnobs, slots: usize) -> Vec<Color> {
    let mut lut = Vec::with_capacity(slots * PAL_STRIDE);
    for s in 0..slots {
        let col = (s % SPECIES.len()) as f64;
        let hue = k.hue as f64 + col * 13.0 - 26.0;
        let bark = shift_hue(palette[2], hue);
        let limb = shift_hue(palette[1], hue);
        let leaf = shift_hue(palette[1], hue + 14.0);
        let bloom = shift_hue(palette[3], hue);
        ramp(&mut lut, darken(bark, 45), lighten(bark, 20));
        ramp(&mut lut, darken(limb, 45), lighten(limb, 35));
        ramp(&mut lut, darken(leaf, 60), lighten(leaf, 55));
        ramp(&mut lut, darken(bloom, 25), lighten(bloom, 45));
        ramp(&mut lut, darken(bark, 75), darken(bark, 12));
        ramp(&mut lut, darken(palette[4], 110), palette[4]);
    }
    lut
}

fn bake_sheet(
    w: usize,
    h: usize,
    seed: u64,
    k: &TreesKnobs,
    key: (usize, usize, u64, u32, u32, u32, u32, u32, u32),
) -> SheetBake {
    let cols = SPECIES.len();
    let rows = 2usize;
    let cell_w = (w / cols).max(6);
    let tall = ((h as f32 * 0.60) as usize).max(9).min(h.saturating_sub(9).max(9));
    let heights = [tall, h.saturating_sub(tall).max(9)];

    let mut chrome: Vec<BakedCell> = Vec::new();
    let mut trees: Vec<BakedCell> = Vec::new();
    let mut leaves: Vec<u32> = Vec::new();
    let mut phase: Vec<(f32, f32)> = Vec::new();
    let mut canvas = Canvas::new(cell_w, heights[0]);
    let mut prng = StdRng::seed_from_u64(seed ^ 0x0B1_5EED_u64);

    for row in 0..rows {
        let cell_h = heights[row];
        let ground_y = (cell_h as i32 - 4).max(2);
        let label_y = (cell_h as i32 - 1).max(3);
        for (i, sp) in SPECIES.iter().enumerate() {
            let slot = (row * cols + i) as u16;
            let ox = (i * cell_w) as i32;
            let oy = if row == 0 { 0 } else { heights[0] as i32 };

            canvas.reset(cell_w, cell_h);
            for x in 0..cell_w as i32 {
                let n = hashf(x + ox, ground_y, 5150);
                let g = if n < 0.72 { '─' } else if n < 0.9 { '╴' } else { '╶' };
                canvas.put(x, ground_y + 1, g, BAND_ROOT, 4 + (n * 4.0) as u8);
            }
            for x in 0..cell_w as i32 {
                if hashf(x + ox, ground_y + 2, 6161) < 0.90 {
                    continue;
                }
                canvas.put(x, ground_y + 2, '·', BAND_ROOT, 3);
            }
            let label = sp.label();
            let lx = (cell_w as i32 - label.chars().count() as i32) / 2;
            for (j, ch) in label.chars().enumerate() {
                canvas.put(lx + j as i32, label_y, ch, BAND_AIR, if row == 0 { 27 } else { 19 });
            }
            capture(&canvas, ox, oy, slot, slot, ground_y, &mut chrome);

            canvas.reset(cell_w, cell_h);
            let energy = if row == 0 { k.energy } else { k.energy * k.scrub };
            let plot = Plot {
                rect: Rect { x: 1, y: 1, w: cell_w.saturating_sub(2).max(3), h: ground_y as usize },
                energy,
                fruit: k.fruit,
                branch: k.branch,
                roots: (k.roots as usize) + i + row,
                detail: k.detail,
                bare: k.bare,
            };
            let mut rng = StdRng::seed_from_u64(
                seed ^ (hash2(i as i32 + 1, row as i32 + 1, 0x51F3) as u64) << 8,
            );
            grow_species(*sp, &mut canvas, &plot, &mut rng);
            for ly in (ground_y as usize + 2)..cell_h {
                for lx in 0..cell_w {
                    canvas.cells[ly * cell_w + lx] = TCell::empty();
                }
            }
            let start = trees.len();
            capture(&canvas, ox, oy, slot, slot, ground_y, &mut trees);
            for idx in (start..trees.len()).step_by(3) {
                if leaves.len() >= 8000 {
                    break;
                }
                let band = (trees[idx].pal as usize % PAL_STRIDE) / TONES;
                if band == BAND_LEAF as usize {
                    leaves.push(idx as u32);
                }
            }

            let period = 7.0 + prng.random::<f32>() * 9.0;
            phase.push((TAU / period, prng.random::<f32>() * TAU));
        }
    }

    SheetBake { key, chrome, trees, leaves, phase, slots: rows * cols }
}

pub(crate) fn draw_opus_1_trees(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    k: &TreesKnobs,
) {
    let h = height.min(grid.len());
    let w = if h == 0 { 0 } else { width.min(grid[0].len()) };
    if w < 10 || h < 8 {
        return;
    }

    measure_layer("opus-1-trees", "clear", || {
        for row in grid.iter_mut().take(h) {
            row.fill(Cell::blank());
        }
    });

    SHEET.with(|store| {
        let mut slot = store.borrow_mut();
        let key = sheet_key(w, h, seed, k);
        if slot.as_ref().map(|b| b.key) != Some(key) {
            *slot = Some(measure_layer("opus-1-trees", "bake", || bake_sheet(w, h, seed, k, key)));
        }
        let b = slot.as_ref().unwrap();
        let lut = build_sheet_lut(palette, k, b.slots);

        let mut offs = vec![0i32; b.phase.len()];
        if t > 0.0 {
            for (i, &(om, ph)) in b.phase.iter().enumerate() {
                offs[i] = (k.sway * 65536.0 * (t * k.speed * om + ph).sin()) as i32;
            }
        }

        measure_layer("opus-1-trees", "ground", || blit(grid, w, h, &b.chrome, &lut, &offs));
        measure_layer("opus-1-trees", "trees", || blit(grid, w, h, &b.trees, &lut, &offs));

        if t > 0.0 && !b.leaves.is_empty() {
            measure_layer("opus-1-trees", "flicker", || {
                let n = b.leaves.len();
                let span = (n / 7).max(1);
                let start = ((t * k.speed * 3.0) as usize).wrapping_mul(97) % n;
                for j in 0..span {
                    let c = b.trees[b.leaves[(start + j) % n] as usize];
                    let dx = (offs[c.group as usize] * c.sway as i32) >> 24;
                    let x = c.x as i32 + dx;
                    let y = c.y as usize;
                    if x < 0 || x as usize >= w || y >= h {
                        continue;
                    }
                    let base = c.pal - (c.pal % TONES as u16);
                    let tone = (c.pal % TONES as u16 + 5).min(TONES as u16 - 1);
                    grid[y][x as usize] = Cell::new(c.ch, lut[(base + tone) as usize]);
                }
            });
        }
    });
}

pub(crate) fn cli_opus_1_trees(
    mut grid: Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: [Color; 5],
    _rng: StdRng,
    t_anim: f32,
    _term_w: u16,
    _term_h: u16,
    args: &[String],
    _mode: &str,
    _theme_name: &str,
) -> (Grid, bool) {
    let mut k = TreesKnobs::from_env();
    if let Some(v) = args.get(4).and_then(|s| s.parse().ok()) {
        k.energy = v;
    }
    if let Some(v) = args.get(5).and_then(|s| s.parse().ok()) {
        k.fruit = v;
    }
    if let Some(v) = args.get(6).and_then(|s| s.parse().ok()) {
        k.branch = v;
    }
    if let Some(v) = args.get(7).and_then(|s| s.parse().ok()) {
        k.scrub = v;
    }
    if let Some(v) = args.get(8).and_then(|s| s.parse().ok()) {
        k.roots = v;
    }
    if let Some(v) = args.get(9).and_then(|s| s.parse().ok()) {
        k.sway = v;
    }
    if let Some(v) = args.get(10).and_then(|s| s.parse().ok()) {
        k.speed = v;
    }
    if let Some(v) = args.get(11).and_then(|s| s.parse().ok()) {
        k.detail = v;
    }
    if let Some(v) = args.get(12).and_then(|s| s.parse().ok()) {
        k.hue = v;
    }
    draw_opus_1_trees(&mut grid, width, height, seed, &palette, t_anim, &k);
    (grid, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(w: usize, h: usize, seed: u64, t: f32) -> String {
        let mut g = vec![vec![Cell::blank(); w]; h];
        let p = crate::color::make_palette(seed);
        let k = TreesKnobs::from_env();
        draw_opus_1_trees(&mut g, w, h, seed, &p, t, &k);
        g.iter()
            .map(|row| row.iter().map(|c| c.ch).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn snapshot_opus_1_trees_static() {
        insta::assert_snapshot!("opus_1_trees_80x24_static", run(80, 24, 42, 0.0));
    }

    #[test]
    fn opus_1_trees_is_deterministic() {
        assert_eq!(run(80, 24, 42, 0.0), run(80, 24, 42, 0.0));
        assert_ne!(run(80, 24, 42, 0.0), run(80, 24, 9, 0.0));
    }

    #[test]
    fn opus_1_trees_animates() {
        assert_ne!(run(80, 24, 42, 0.0), run(80, 24, 42, 4.0));
    }
}
