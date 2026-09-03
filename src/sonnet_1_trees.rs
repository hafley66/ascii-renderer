//! sonnet-1 tree species: krummholz, fig, colonist, proproot, bottle, stilt.
//! Plus the sonnet-1-trees sample sheet.

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

/// A species writes band+tone, never a literal color; the mode owns the ramp.
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
        if ch == '\0' || x < 0 || y < 0 {
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

/// One painted cell of a frozen scene. `pal` indexes the per-frame color
/// table, `group` selects the sway oscillator, `sway` is the height weight.
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

/// Paint a bake list. `offs` is per-group sway in cells scaled by 65536.
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

/// Slope-to-glyph for a float line segment.
fn glyph_for(dx: f32, dy: f32) -> char {
    let a = dx.abs();
    let b = dy.abs();
    if b > a * 1.9 {
        '│'
    } else if a > b * 2.6 {
        '─'
    } else if dx * dy < 0.0 {
        '╱'
    } else {
        '╲'
    }
}

fn stroke(c: &mut Canvas, x0: f32, y0: f32, x1: f32, y1: f32, band: u8, tone: u8) {
    let dx = x1 - x0;
    let dy = y1 - y0;
    let steps = (dx.abs().max(dy.abs()).ceil() as i32).clamp(1, 512);
    let ch = glyph_for(dx / ASPECT, dy);
    for s in 0..=steps {
        let f = s as f32 / steps as f32;
        c.put((x0 + dx * f).round() as i32, (y0 + dy * f).round() as i32, ch, band, tone);
    }
}

/// A limb spurring off a mostly-straight column: the right corner glyph for
/// the turn is a tee, not a generic line char.
fn branch_glyph(vertical_trunk: bool, side: i32) -> char {
    if vertical_trunk {
        if side >= 0 { '├' } else { '┤' }
    } else if side >= 0 {
        '┬'
    } else {
        '┴'
    }
}

/// Rounded corner for a wobbling column that changes drift direction.
fn wobble_corner(d0: i32, d1: i32) -> char {
    match (d0.signum(), d1.signum()) {
        (1, -1) | (1, 0) => '╮',
        (-1, 1) | (-1, 0) => '╭',
        (0, 1) => '╭',
        (0, -1) => '╮',
        _ => '│',
    }
}

/// Overwrite a column's own cells with corner glyphs wherever its drift
/// direction reverses, so a gnarled trunk reads as bent, not just noisy.
fn mark_wobble(c: &mut Canvas, top_y: i32, xs: &[i32], tone: u8) {
    for i in 1..xs.len().saturating_sub(1) {
        let d0 = xs[i] - xs[i - 1];
        let d1 = xs[i + 1] - xs[i];
        if d0 == 0 && d1 == 0 {
            continue;
        }
        if d0.signum() == d1.signum() {
            continue;
        }
        c.put(xs[i], top_y + i as i32, wobble_corner(d0, d1), BAND_TRUNK, tone);
    }
}

/// Bark glyph for a taper ring: thin rings draw a plain rule, thick rings
/// get a ridged fill that lightens toward the edge.
fn bark_glyph(half: i32, d: i32, n: f32) -> char {
    if half <= 0 {
        if d == 0 { '│' } else { '\0' }
    } else if half == 1 {
        if d == 0 { '┃' } else { '\0' }
    } else if d == 0 {
        if n > 0.82 { '▓' } else { '█' }
    } else if d.abs() == half {
        if n > 0.55 { '▒' } else { '░' }
    } else if n > 0.6 {
        '▓'
    } else {
        '█'
    }
}

/// A tapering trunk column with a caller-chosen radius profile. Fills `xs`
/// with the center x for every row from `top_y` (index 0) to `base_y`.
fn draw_bark_column(
    c: &mut Canvas,
    top_y: i32,
    base_y: i32,
    base_x: f32,
    lean: f32,
    wobble: f32,
    radius: impl Fn(f32) -> f32,
    rng: &mut StdRng,
    xs: &mut Vec<i32>,
) {
    xs.clear();
    let span = (base_y - top_y).max(1) as f32;
    let mut drift = 0.0f32;
    for y in top_y..=base_y {
        let f = (y - top_y) as f32 / span;
        drift += (rng.random::<f32>() - 0.5) * wobble;
        drift = drift.clamp(-span * 0.22, span * 0.22);
        let x = base_x + lean * f * f + drift;
        let xi = x.round() as i32;
        xs.push(xi);
        let half = radius(f).round() as i32;
        let tone = (6.0 + (1.0 - f) * 12.0) as u8;
        for d in -half..=half {
            let n = hashf(xi + d, y, 0x8A21);
            c.put(xi + d, y, bark_glyph(half, d, n), BAND_TRUNK, tone);
        }
        if half >= 2 && rng.random::<f32>() < 0.05 {
            c.put(xi, y, '┼', BAND_TRUNK, tone.saturating_add(6));
        }
    }
}

/// Repaint a column already sampled into `xs`, so it sits over later art.
fn redraw_bark_column(c: &mut Canvas, top_y: i32, xs: &[i32], radius: impl Fn(f32) -> f32, skip: usize) {
    let span = xs.len().max(1) as f32;
    for (k, &xi) in xs.iter().enumerate().skip(skip) {
        let f = k as f32 / span;
        let half = radius(f).round() as i32;
        let tone = (6.0 + (1.0 - f) * 12.0) as u8;
        for d in -half..=half {
            let n = hashf(xi + d, top_y + k as i32, 0x8A21);
            c.put(xi + d, top_y + k as i32, bark_glyph(half, d, n), BAND_TRUNK, tone);
        }
    }
}

/// Ground flare at the base of a trunk: buttress ticks, a root mat, or scatter.
fn flare_roots(c: &mut Canvas, x: i32, y: i32, spread: i32, style: usize, rng: &mut StdRng) {
    let spread = spread.max(2);
    match style % 3 {
        0 => {
            let n = 3 + rng.random_range(0..3u32) as i32;
            c.put(x, y, '┴', BAND_ROOT, 15);
            for k in 0..n {
                let side = if k % 2 == 0 { 1 } else { -1 };
                let len = ((spread as f32) * (0.25 + rng.random::<f32>() * 0.5)) as i32;
                for d in 1..=len {
                    let cap = d == len;
                    let g = if cap && side > 0 { '╯' } else if cap { '╰' } else { '─' };
                    c.put(x + side * d, y - (d / 4).min(1), g, BAND_ROOT, (14 - d).max(3) as u8);
                }
            }
        }
        1 => {
            for d in -spread..=spread {
                if hashf(x + d, y, 0x51C1) < 0.5 {
                    continue;
                }
                let g = if d.abs() * 2 < spread { '▒' } else { '░' };
                c.put(x + d, y, g, BAND_ROOT, 8 + (spread - d.abs()).clamp(0, 8) as u8);
            }
            c.put(x, y, '┴', BAND_ROOT, 16);
        }
        _ => {
            for d in -spread..=spread {
                if hashf(x + d, y, 0x77E3) < 0.6 {
                    continue;
                }
                let g = if (d.abs() & 1) == 0 { '∙' } else { '·' };
                c.put(x + d, y, g, BAND_ROOT, 7);
            }
            c.put(x, y, '┴', BAND_ROOT, 14);
        }
    }
}

/// Seeded two-term radial fringe. Makes every crown lopsided in its own way.
#[derive(Clone, Copy)]
struct Fringe {
    k1: f32,
    k2: f32,
    p1: f32,
    p2: f32,
    a1: f32,
    a2: f32,
}

impl Fringe {
    fn seeded(rng: &mut StdRng) -> Self {
        Fringe {
            k1: 1.0 + rng.random_range(0..3u32) as f32,
            k2: 3.0 + rng.random_range(0..4u32) as f32,
            p1: rng.random::<f32>() * TAU,
            p2: rng.random::<f32>() * TAU,
            a1: 0.14 + rng.random::<f32>() * 0.22,
            a2: 0.05 + rng.random::<f32>() * 0.14,
        }
    }

    #[inline]
    fn at(&self, th: f32) -> f32 {
        (1.0 + self.a1 * (self.k1 * th + self.p1).sin() + self.a2 * (self.k2 * th + self.p2).sin()).max(0.3)
    }
}

/// Canopy ellipse whose density and glyph weight thin from core to fringe;
/// `thin_pow` steepens (>1) or softens (<1) how fast the fringe empties out.
fn leaf_cloud(c: &mut Canvas, cx: f32, cy: f32, rx: f32, ry: f32, band: u8, fruit: f32, thin_pow: f32, rng: &mut StdRng, salt: u32) {
    if rx < 0.5 || ry < 0.5 {
        return;
    }
    let fringe = Fringe::seeded(rng);
    let x0 = (cx - rx * 1.15).floor() as i32;
    let x1 = (cx + rx * 1.15).ceil() as i32;
    let y0 = (cy - ry * 1.15).floor() as i32;
    let y1 = (cy + ry * 1.15).ceil() as i32;
    for y in y0..=y1 {
        for x in x0..=x1 {
            let nx = (x as f32 - cx) / rx;
            let ny = (y as f32 - cy) / ry;
            let rr = (nx * nx + ny * ny).sqrt();
            if rr < 0.001 {
                continue;
            }
            let th = (-ny).atan2(nx);
            let edge = fringe.at(th);
            let d = rr / edge;
            if d > 1.0 {
                continue;
            }
            let n = hashf(x, y, salt);
            let keep = (1.0 - d.powf(thin_pow)).clamp(0.0, 1.0);
            if n > keep {
                continue;
            }
            let g = if d < 0.35 { '▓' } else if d < 0.6 { '▒' } else if d < 0.82 { '∙' } else { '·' };
            let tone = (26.0 - d * 14.0) as u8;
            c.put(x, y, g, band, tone);
            if d > 0.7 && hashf(x, y, salt ^ 0x91) < fruit {
                c.put(x, y, '●', BAND_BLOOM, 27);
            }
        }
    }
}

// ── species ─────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Species {
    Krummholz,
    Fig,
    Colonist,
    PropRoot,
    Bottle,
    Stilt,
}

pub(crate) const SPECIES: [Species; 6] = [
    Species::Krummholz,
    Species::Fig,
    Species::Colonist,
    Species::PropRoot,
    Species::Bottle,
    Species::Stilt,
];

impl Species {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Species::Krummholz => "krummholz",
            Species::Fig => "fig",
            Species::Colonist => "colonist",
            Species::PropRoot => "proproot",
            Species::Bottle => "bottle",
            Species::Stilt => "stilt",
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
    pub wind: f32,
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
        Species::Krummholz => grow_krummholz(c, p, rng),
        Species::Fig => grow_fig(c, p, rng),
        Species::Colonist => grow_colonist(c, p, rng),
        Species::PropRoot => grow_proproot(c, p, rng),
        Species::Bottle => grow_bottle(c, p, rng),
        Species::Stilt => grow_stilt(c, p, rng),
    }
}

// --- 1. krummholz: wind-flagged turtle walk -------------------------
// See briefs/sonnet-1-trees.md for the rule set.

fn grow_krummholz(c: &mut Canvas, p: &Plot, rng: &mut StdRng) {
    let (rx, ry) = p.root();
    let top = p.crown_top();
    let full_h = (ry - top).max(6) as f32;
    let stunt = (0.28 + p.energy * 0.18).clamp(0.26, 0.50);
    let stem_top = ry - (full_h * stunt) as i32;
    let spread = p.spread().max(3) as f32;
    let wind = p.wind.clamp(-1.0, 1.0);

    flare_roots(c, rx, ry, (spread * 0.45) as i32, p.roots, rng);

    let lean = wind * spread * 0.45;
    let radius = move |f: f32| (0.5 + spread * 0.09) * f.powf(0.7) + 0.35;
    let mut xs: Vec<i32> = Vec::new();
    draw_bark_column(c, stem_top, ry, rx as f32, lean, 0.55, radius, rng, &mut xs);
    mark_wobble(c, stem_top, &xs, 16);

    let downwind_side: i32 = if wind >= 0.0 { 1 } else { -1 };
    let n_branch = 4 + (p.branch * 5.0) as i32;
    let salt = rng.random_range(0..9999u32);
    for k in 0..n_branch {
        let t = 0.45 + 0.5 * (k as f32 / n_branch.max(1) as f32);
        let idx = (((1.0 - t) * (xs.len().saturating_sub(1)) as f32).round() as usize).min(xs.len().saturating_sub(1));
        let bx = xs[idx];
        let by = stem_top + idx as i32;
        let flagged = rng.random::<f32>() < 0.5 + wind.abs() * 0.42;
        let side = if flagged { downwind_side } else { -downwind_side };
        c.put(bx, by, branch_glyph(true, side), BAND_TRUNK, 17);
        if flagged {
            let len = spread * (0.42 + rng.random::<f32>() * 0.55) * (1.0 + wind.abs() * 0.5);
            let ex = bx as f32 + side as f32 * len;
            let ey = by as f32 - len * 0.16 - rng.random::<f32>() * 1.5;
            stroke(c, bx as f32, by as f32, ex, ey, BAND_LIMB, 14);
            leaf_cloud(c, ex, ey, len * 0.75, len * 0.46, BAND_LEAF, p.fruit, 0.85, rng, salt + k as u32);
        } else {
            let len = spread * (0.06 + rng.random::<f32>() * 0.10);
            let ex = bx as f32 + side as f32 * len;
            let ey = by as f32 - len * 0.1;
            stroke(c, bx as f32, by as f32, ex, ey, BAND_LIMB, 9);
        }
    }
}

// --- 2. fig: a strangler wrapping a host trunk -----------------------
// See briefs/sonnet-1-trees.md for the rule set.

fn grow_fig(c: &mut Canvas, p: &Plot, rng: &mut StdRng) {
    let (rx, ry) = p.root();
    let top = p.crown_top();
    let spread = p.spread().max(3) as f32;
    let host_top = top + (spread * 0.68) as i32;

    let host_radius = move |f: f32| 0.35 + f * 0.55;
    let mut hxs: Vec<i32> = Vec::new();
    draw_bark_column(c, host_top, ry, rx as f32, 0.0, 0.14, host_radius, rng, &mut hxs);

    let strands = 4 + (p.roots as i32 % 3) + rng.random_range(0..2u32) as i32;
    let thick_stage = (p.energy * 2.2) as i32;
    let steps = (ry - host_top).max(6);
    for _ in 0..strands {
        let phase = rng.random::<f32>() * TAU;
        let amp = spread * (0.30 + rng.random::<f32>() * 0.40);
        let wraps = 0.8 + rng.random::<f32>() * 1.2;
        let mut px = rx as f32;
        let mut py = host_top as f32;
        for s in 1..=steps {
            let f = s as f32 / steps as f32;
            let x = rx as f32 + (phase + f * wraps * TAU).sin() * amp * f;
            let y = host_top as f32 + f * (ry - host_top) as f32;
            let tone = (9.0 + f * 15.0) as u8;
            stroke(c, px, py, x, y, BAND_ROOT, tone);
            if thick_stage >= 1 {
                stroke(c, px + 1.0, py, x + 1.0, y, BAND_ROOT, tone.saturating_sub(2));
            }
            px = x;
            py = y;
        }
        c.put(px.round() as i32, ry, '┴', BAND_ROOT, 15);
    }
    flare_roots(c, rx, ry, spread as i32, 1, rng);
    c.put(rx, host_top, branch_glyph(true, 1), BAND_TRUNK, 18);

    let salt = rng.random_range(0..9999u32);
    let ccx = rx as f32;
    let ccy = (top + host_top) as f32 * 0.5;
    let crx = spread * 1.30;
    let cry = ((host_top - top) as f32 * 0.62).max(2.0);
    leaf_cloud(c, ccx, ccy, crx, cry, BAND_LEAF, p.fruit, 0.9, rng, salt);
    leaf_cloud(c, ccx * 0.85, host_top as f32 - 1.0, crx * 0.22, cry * 0.20, BAND_LEAF, 0.0, 2.2, rng, salt ^ 7);
}

// --- 3. colonist: true space colonization toward a leaf cloud --------
// See briefs/sonnet-1-trees.md for the rule set.

struct SNode {
    x: f32,
    y: f32,
    parent: i32,
    mass: u32,
}

fn grow_colonist(c: &mut Canvas, p: &Plot, rng: &mut StdRng) {
    let (rx, ry) = p.root();
    let top = p.crown_top();
    let spread = p.spread().max(3) as f32;
    let height = (ry - top).max(6) as f32;

    let base_top = ry - (height * 0.16) as i32;
    let base_radius = move |f: f32| 0.3 + spread * 0.08 * f;
    let mut xs: Vec<i32> = Vec::new();
    draw_bark_column(c, base_top, ry, rx as f32, 0.0, 0.20, base_radius, rng, &mut xs);
    flare_roots(c, rx, ry, (spread * 0.4) as i32, p.roots, rng);
    c.put(rx, base_top, branch_glyph(true, 1), BAND_TRUNK, 17);

    let bias = (rng.random::<f32>() - 0.5) * spread * 0.7;
    let cloud_cy = (top as f32 + base_top as f32) * 0.5;
    let cloud_ry = ((base_top - top) as f32 * 0.5).max(2.0);
    let n_att = ((spread * cloud_ry * 2.4 * p.detail) as usize).clamp(50, 420);
    let mut att: Vec<(f32, f32)> = Vec::with_capacity(n_att);
    for _ in 0..n_att {
        let lobe = if rng.random::<f32>() < 0.58 { 1.0 } else { -1.0 };
        let cx = rx as f32 + bias * lobe * 0.55;
        let th = rng.random::<f32>() * TAU;
        let rr = rng.random::<f32>().sqrt();
        att.push((cx + th.cos() * rr * spread, cloud_cy - th.sin() * rr * cloud_ry));
    }

    let step = (cloud_ry * 0.28).clamp(0.8, 2.2);
    let ri = step * 5.2;
    let rk = step * 1.4;
    let node_cap = 420usize;
    let mut nodes: Vec<SNode> = vec![SNode { x: rx as f32, y: base_top as f32, parent: -1, mass: 1 }];

    let bx0 = rx as f32 - spread * 2.4;
    let by0 = top as f32 - cloud_ry * 0.4;
    let bw = (((spread * 4.8) / (ri * ASPECT)).ceil() as usize).clamp(1, 64);
    let bh = ((((base_top as f32 - by0) + cloud_ry) / ri).ceil() as usize).clamp(1, 64);
    let mut buckets: Vec<Vec<u32>> = vec![Vec::new(); bw * bh];
    let bucket_of = |x: f32, y: f32| -> (usize, usize) {
        let ix = (((x - bx0) / (ri * ASPECT)) as isize).clamp(0, bw as isize - 1) as usize;
        let iy = (((y - by0) / ri) as isize).clamp(0, bh as isize - 1) as usize;
        (ix, iy)
    };
    {
        let (ix, iy) = bucket_of(nodes[0].x, nodes[0].y);
        buckets[iy * bw + ix].push(0);
    }

    let mut alive = vec![true; att.len()];
    let mut remaining = att.len();
    let mut pull: Vec<(f32, f32, u32)> = Vec::new();
    let mut iters = 0;

    while remaining > 0 && nodes.len() < node_cap && iters < 70 {
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
            if k == 0 || nodes.len() >= node_cap {
                continue;
            }
            let jx = (rng.random::<f32>() - 0.5) * 0.3;
            let jy = (rng.random::<f32>() - 0.5) * 0.18;
            let mut ux = px / k as f32 + jx;
            let mut uy = py / k as f32 - 0.20 + jy;
            let m = (ux * ux + uy * uy).sqrt().max(0.001);
            ux /= m;
            uy /= m;
            let nx = nodes[i].x + ux * step * ASPECT;
            let ny = nodes[i].y + uy * step;
            let idx = nodes.len() as u32;
            nodes.push(SNode { x: nx, y: ny, parent: i as i32, mass: 1 });
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
    let salt = rng.random_range(0..9999u32);
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
        stroke(c, nodes[par as usize].x, nodes[par as usize].y, nodes[i].x, nodes[i].y, band, tone);
        if band == BAND_TRUNK && hashf(nodes[i].x as i32, nodes[i].y as i32, salt) < 0.20 {
            leaf_cloud(c, nodes[i].x, nodes[i].y, 3.0, 1.8, BAND_LEAF, 0.0, 1.5, rng, salt ^ i as u32);
        }
    }
    for i in 1..nodes.len() {
        if nodes[i].mass > 5 {
            continue;
        }
        let (x, y) = (nodes[i].x.round() as i32, nodes[i].y.round() as i32);
        let n = hashf(x, y, 0x4241 ^ salt);
        let g = if n < 0.34 { '◆' } else if n < 0.68 { '◇' } else { '∙' };
        c.put(x, y, g, BAND_LEAF, (20.0 + n * 10.0) as u8);
        if n < p.fruit {
            c.put(x, y, '●', BAND_BLOOM, 26);
        }
    }
}

// --- 4. proproot: banyan-style aerial prop roots ----------------------
// See briefs/sonnet-1-trees.md for the rule set.

fn grow_proproot(c: &mut Canvas, p: &Plot, rng: &mut StdRng) {
    let (rx, ry) = p.root();
    let top = p.crown_top();
    let spread = p.spread().max(3) as f32;
    let height = (ry - top).max(6) as f32;
    let trunk_top = ry - (height * 0.25) as i32;

    let radius = move |f: f32| 0.5 + spread * 0.12 * f;
    let mut xs: Vec<i32> = Vec::new();
    draw_bark_column(c, trunk_top, ry, rx as f32, 0.0, 0.14, radius, rng, &mut xs);
    flare_roots(c, rx, ry, (spread * 0.5) as i32, p.roots, rng);

    let n_limbs = 3 + (p.branch * 4.0) as i32;
    let mut tips: Vec<(f32, f32, f32)> = Vec::new();
    for k in 0..n_limbs {
        let side: i32 = if k % 2 == 0 { 1 } else { -1 };
        let ang = 0.08 + rng.random::<f32>() * 0.24;
        let len = spread * (0.5 + rng.random::<f32>() * 0.65);
        let ex = rx as f32 + side as f32 * len;
        let ey = trunk_top as f32 - len * ang * 0.5;
        c.put(rx, trunk_top, branch_glyph(true, side), BAND_TRUNK, 16);
        stroke(c, rx as f32, trunk_top as f32, ex, ey, BAND_LIMB, 14);
        for _ in 0..2 {
            let fside: f32 = if rng.random::<f32>() < 0.5 { 1.0 } else { -1.0 };
            let flen = len * 0.5;
            let fx = ex + fside * flen * 0.4;
            let fy = ey - flen * 0.22;
            stroke(c, ex, ey, fx, fy, BAND_LIMB, 13);
            tips.push((fx, fy, flen.max(2.0)));
        }
    }

    let salt = rng.random_range(0..9999u32);
    for (i, &(tx, ty, tl)) in tips.iter().enumerate() {
        leaf_cloud(c, tx, ty - tl * 0.15, tl * 1.25, tl * 0.75, BAND_LEAF, p.fruit, 0.9, rng, salt + i as u32);
    }

    let n_props = (p.roots as i32 + 1 + (p.energy * 2.0) as i32).clamp(1, tips.len().max(1) as i32);
    for i in 0..n_props {
        if tips.is_empty() {
            break;
        }
        let (tx, ty0, tl) = tips[(i as usize) % tips.len()];
        let ty = ty0 + tl * 0.35;
        let drop = ry as f32 - ty;
        if drop < 2.0 {
            continue;
        }
        let touches = drop < height * 0.85;
        let steps = drop.max(2.0) as i32;
        let sway = (rng.random::<f32>() - 0.5) * 2.4;
        let stage = (p.energy * 2.4) as i32;
        let mut px = tx;
        let mut py = ty;
        for s in 1..=steps {
            let f = s as f32 / steps as f32;
            let nx = tx + sway * f * f;
            let ny = ty + drop * f;
            let d0 = (nx - px).round() as i32;
            let g = if d0.abs() >= 1 { wobble_corner(d0, 0) } else { '│' };
            let tone = (9.0 + f * 11.0) as u8;
            c.put(nx.round() as i32, ny.round() as i32, g, BAND_ROOT, tone);
            if touches && stage >= 1 {
                c.put(nx.round() as i32 - 1, ny.round() as i32, '┃', BAND_ROOT, tone);
            }
            px = nx;
            py = ny;
        }
        if touches {
            c.put(px.round() as i32, ry, '┴', BAND_ROOT, 15);
            flare_roots(c, px.round() as i32, ry, (spread * 0.16).max(1.0) as i32, (p.roots + i as usize) % 3, rng);
        } else {
            c.put(px.round() as i32, py.round() as i32, '╷', BAND_ROOT, 10);
        }
    }
    redraw_bark_column(c, trunk_top, &xs, radius, 0);
}

// --- 5. bottle: baobab bottle trunk with a tiny crown -----------------
// See briefs/sonnet-1-trees.md for the rule set.

fn grow_bottle(c: &mut Canvas, p: &Plot, rng: &mut StdRng) {
    let (rx, ry) = p.root();
    let top = p.crown_top();
    let spread = p.spread().max(3) as f32;
    let height = (ry - top).max(6) as f32;
    let crown_base = top + (height * 0.14) as i32;

    let base_r = spread * 0.95;
    let peak = 0.60 + (rng.random::<f32>() - 0.5) * 0.14;
    let width_ = 0.40;
    let radius = move |f: f32| {
        let belly = (1.0 - ((f - peak) / width_).powi(2)).max(0.0).sqrt();
        (base_r * (0.22 + 0.85 * belly)).max(0.55)
    };
    let mut xs: Vec<i32> = Vec::new();
    draw_bark_column(c, crown_base, ry, rx as f32, 0.0, 0.10, radius, rng, &mut xs);

    flare_roots(c, rx, ry, (spread * 0.85) as i32, 0, rng);

    let n_stub = 3 + (p.branch * 3.0) as i32;
    let salt = rng.random_range(0..9999u32);
    for k in 0..n_stub {
        let ang = (k as f32 + 0.5) / n_stub.max(1) as f32 * std::f32::consts::PI - 0.35;
        let len = spread * 0.22 * (0.6 + rng.random::<f32>() * 0.6);
        let side: i32 = if ang.cos() >= 0.0 { 1 } else { -1 };
        let ex = rx as f32 + ang.cos() * len * ASPECT * 0.5;
        let ey = crown_base as f32 - ang.sin().abs() * len * 0.6 - 1.0;
        c.put(rx, crown_base, branch_glyph(true, side), BAND_TRUNK, 16);
        stroke(c, rx as f32, crown_base as f32, ex, ey, BAND_LIMB, 13);
        leaf_cloud(c, ex, ey, len * 0.55, len * 0.38, BAND_LEAF, p.fruit, 1.7, rng, salt + k as u32);
    }
}

// --- 6. stilt: mangrove stilt roots interlocking over water -----------
// See briefs/sonnet-1-trees.md for the rule set.

fn grow_stilt(c: &mut Canvas, p: &Plot, rng: &mut StdRng) {
    let (rx, ry) = p.root();
    let top = p.crown_top();
    let spread = p.spread().max(3) as f32;
    let height = (ry - top).max(6) as f32;
    let stilt_h = (height * (0.22 + rng.random::<f32>() * 0.16)).max(3.0) as i32;
    let water_y = ry - stilt_h;
    let crown_base = (top + (height * (1.0 - p.bare)) as i32).min(water_y - 1).max(top + 1);

    let radius = move |f: f32| 0.4 + spread * 0.10 * f;
    let mut xs: Vec<i32> = Vec::new();
    draw_bark_column(c, crown_base, water_y, rx as f32, 0.0, 0.14, radius, rng, &mut xs);
    c.put(rx, water_y, branch_glyph(true, 1), BAND_TRUNK, 16);

    let legs = 3 + (p.roots as i32 % 3) + 2;
    let salt = rng.random_range(0..9999u32);
    let steps = stilt_h.max(3);
    for k in 0..legs {
        let side0: f32 = if k % 2 == 0 { 1.0 } else { -1.0 };
        let reach = spread * (0.5 + rng.random::<f32>() * 0.55);
        let mut px = rx as f32;
        let mut py = water_y as f32;
        for s in 1..=steps {
            let f = s as f32 / steps as f32;
            let bend = (f * std::f32::consts::PI).sin();
            let x = rx as f32 + side0 * reach * f - side0 * reach * 0.4 * bend;
            let y = water_y as f32 + f * stilt_h as f32;
            stroke(c, px, py, x, y, BAND_ROOT, (9.0 + f * 10.0) as u8);
            px = x;
            py = y;
        }
        c.put(px.round() as i32, ry, '┴', BAND_ROOT, 15);
        if hashf(px.round() as i32, ry, salt) > 0.4 {
            c.put(px.round() as i32, ry - 1, '╷', BAND_ROOT, 12);
        }
    }
    let half_w = (spread * 2.0) as i32;
    for dx in -half_w..=half_w {
        let x = rx + dx;
        let n = hashf(x, ry, salt ^ 0x51);
        if n < 0.55 {
            continue;
        }
        c.put_soft(x, ry, if n > 0.85 { '≈' } else { '~' }, BAND_ROOT, 10);
    }

    let ccy = (top + crown_base) as f32 * 0.5;
    let crx = spread * 1.10;
    let cry = ((crown_base - top) as f32 * 0.5).max(2.0);
    leaf_cloud(c, rx as f32, ccy, crx, cry, BAND_LEAF, p.fruit, 1.3, rng, salt ^ 3);
    redraw_bark_column(c, crown_base, &xs, radius, 0);
}

// ── sonnet-1-trees: the sample sheet ──────────────────────────────────

pub(crate) struct TreesKnobs {
    pub energy: f32,
    pub fruit: f32,
    pub branch: f32,
    pub scrub: f32,
    pub roots: f32,
    pub wind: f32,
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
            fruit: param_f32("FRUIT", 0.16).clamp(0.0, 1.0),
            branch: param_f32("BRANCH", 0.6).clamp(0.05, 1.0),
            scrub: param_f32("SCRUB", 0.56).clamp(0.2, 1.0),
            roots: param_f32("ROOTS", 0.0).clamp(0.0, 3.0),
            wind: param_f32("WIND", 0.35).clamp(-1.0, 1.0),
            sway: param_f32("SWAY", 0.9).clamp(0.0, 4.0),
            speed: param_f32("SPEED", 1.0).clamp(0.0, 3.0),
            detail: param_f32("DETAIL", 1.0).clamp(0.2, 2.0),
            hue: param_f32("HUE", 0.0).clamp(-180.0, 180.0),
            bare: param_f32("BARE", 0.26).clamp(0.08, 0.60),
        }
    }
}

struct SheetBake {
    key: (usize, usize, u64, u32, u32, u32, u32, u32, u32, u32),
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

type SheetKey = (usize, usize, u64, u32, u32, u32, u32, u32, u32, u32);

fn sheet_key(w: usize, h: usize, seed: u64, k: &TreesKnobs) -> SheetKey {
    (w, h, seed, q(k.energy), q(k.fruit), q(k.branch), q(k.scrub), q(k.roots), q(k.wind), q(k.detail) ^ q(k.bare))
}

fn build_sheet_lut(palette: &[Color; 5], k: &TreesKnobs, slots: usize) -> Vec<Color> {
    let mut lut = Vec::with_capacity(slots * PAL_STRIDE);
    for s in 0..slots {
        let col = (s % SPECIES.len()) as f64;
        let hue = k.hue as f64 + col * 12.0 - 30.0;
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

fn bake_sheet(w: usize, h: usize, seed: u64, k: &TreesKnobs, key: SheetKey) -> SheetBake {
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
    let mut prng = StdRng::seed_from_u64(seed ^ 0x50_11E7_u64);

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
                wind: (k.wind + (prng.random::<f32>() - 0.5) * 0.3).clamp(-1.0, 1.0),
            };
            let mut rng = StdRng::seed_from_u64(seed ^ (hash2(i as i32 + 1, row as i32 + 1, 0x51F3) as u64) << 8);
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

pub(crate) fn draw_sonnet_1_trees(
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

    measure_layer("sonnet-1-trees", "clear", || {
        for row in grid.iter_mut().take(h) {
            row.fill(Cell::blank());
        }
    });

    SHEET.with(|store| {
        let mut slot = store.borrow_mut();
        let key = sheet_key(w, h, seed, k);
        if slot.as_ref().map(|b| b.key) != Some(key) {
            *slot = Some(measure_layer("sonnet-1-trees", "bake", || bake_sheet(w, h, seed, k, key)));
        }
        let b = slot.as_ref().unwrap();
        let lut = build_sheet_lut(palette, k, b.slots);

        let mut offs = vec![0i32; b.phase.len()];
        if t > 0.0 {
            for (i, &(om, ph)) in b.phase.iter().enumerate() {
                offs[i] = (k.sway * 65536.0 * (t * k.speed * om + ph).sin()) as i32;
            }
        }

        measure_layer("sonnet-1-trees", "ground", || blit(grid, w, h, &b.chrome, &lut, &offs));
        measure_layer("sonnet-1-trees", "trees", || blit(grid, w, h, &b.trees, &lut, &offs));

        if t > 0.0 && !b.leaves.is_empty() {
            measure_layer("sonnet-1-trees", "flicker", || {
                let n = b.leaves.len();
                let span = (n / 7).max(1);
                let start = ((t * k.speed * 3.0) as usize).wrapping_mul(97) % n;
                for j in 0..span {
                    let cell = b.trees[b.leaves[(start + j) % n] as usize];
                    let dx = (offs[cell.group as usize] * cell.sway as i32) >> 24;
                    let x = cell.x as i32 + dx;
                    let y = cell.y as usize;
                    if x < 0 || x as usize >= w || y >= h {
                        continue;
                    }
                    let base = cell.pal - (cell.pal % TONES as u16);
                    let tone = (cell.pal % TONES as u16 + 5).min(TONES as u16 - 1);
                    grid[y][x as usize] = Cell::new(cell.ch, lut[(base + tone) as usize]);
                }
            });
        }
    });
}

pub(crate) fn cli_sonnet_1_trees(
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
        k.wind = v;
    }
    if let Some(v) = args.get(10).and_then(|s| s.parse().ok()) {
        k.sway = v;
    }
    if let Some(v) = args.get(11).and_then(|s| s.parse().ok()) {
        k.speed = v;
    }
    if let Some(v) = args.get(12).and_then(|s| s.parse().ok()) {
        k.detail = v;
    }
    if let Some(v) = args.get(13).and_then(|s| s.parse().ok()) {
        k.hue = v;
    }
    draw_sonnet_1_trees(&mut grid, width, height, seed, &palette, t_anim, &k);
    (grid, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(w: usize, h: usize, seed: u64, t: f32) -> String {
        let mut g = vec![vec![Cell::blank(); w]; h];
        let p = crate::color::make_palette(seed);
        let k = TreesKnobs::from_env();
        draw_sonnet_1_trees(&mut g, w, h, seed, &p, t, &k);
        g.iter()
            .map(|row| row.iter().map(|c| c.ch).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn snapshot_sonnet_1_trees_static() {
        insta::assert_snapshot!("sonnet_1_trees_80x24_static", run(80, 24, 42, 0.0));
    }

    #[test]
    fn sonnet_1_trees_is_deterministic() {
        assert_eq!(run(80, 24, 42, 0.0), run(80, 24, 42, 0.0));
        assert_ne!(run(80, 24, 42, 0.0), run(80, 24, 9, 0.0));
    }

    #[test]
    fn sonnet_1_trees_animates() {
        assert_ne!(run(80, 24, 42, 0.0), run(80, 24, 42, 4.0));
    }
}
