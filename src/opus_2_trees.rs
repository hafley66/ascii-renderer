//! opus-2-trees: five growth algorithms and the sample sheet that shows them.
//! Mangrove arches, space colonization, banyan pillars, bracket shelves, coral aggregation.

use crate::_0_profile::measure_layer;
use crate::color::{darken, lerp_color, lighten};
use crate::opts::param_f32;
use crate::types::{Cell, Grid, Rect};
use crossterm::style::Color;
use rand::RngExt;
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::cell::RefCell;

pub(crate) const SLOT_TRUNK: u8 = 0;
pub(crate) const SLOT_BARK: u8 = 1;
pub(crate) const SLOT_BRANCH: u8 = 2;
pub(crate) const SLOT_TIP: u8 = 3;
pub(crate) const SLOT_FRUIT: u8 = 4;
pub(crate) const SLOT_DIM: u8 = 5;
pub(crate) const SLOTS: usize = 6;

/// Six drawing colors: trunk, bark, branch, tip, fruit, dim.
pub(crate) type Ink = [Color; SLOTS];

/// Sentinel ink whose colors survive baking as slot indices.
pub(crate) fn slot_ink() -> Ink {
    [
        Color::AnsiValue(0),
        Color::AnsiValue(1),
        Color::AnsiValue(2),
        Color::AnsiValue(3),
        Color::AnsiValue(4),
        Color::AnsiValue(5),
    ]
}

pub(crate) fn ink_from(base: Color, fruit: Color) -> Ink {
    [
        darken(base, 58),
        darken(base, 30),
        base,
        lighten(base, 42),
        fruit,
        darken(base, 78),
    ]
}

#[derive(Clone, Copy)]
pub(crate) struct GrowOpts {
    pub(crate) fruit: f32,
    pub(crate) branch: f32,
    pub(crate) gnarl: f32,
    pub(crate) roots: f32,
}

impl Default for GrowOpts {
    fn default() -> Self {
        GrowOpts {
            fruit: 0.25,
            branch: 0.7,
            gnarl: 0.35,
            roots: 0.6,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Species {
    Mangrove,
    Colony,
    Banyan,
    Bracket,
    Coral,
}

pub(crate) const SPECIES: [Species; 5] = [
    Species::Mangrove,
    Species::Colony,
    Species::Banyan,
    Species::Bracket,
    Species::Coral,
];

impl Species {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Species::Mangrove => "MANGROVE",
            Species::Colony => "COLONY",
            Species::Banyan => "BANYAN",
            Species::Bracket => "BRACKET",
            Species::Coral => "CORAL",
        }
    }

    pub(crate) fn from_index(i: usize) -> Species {
        SPECIES[i % SPECIES.len()]
    }
}

pub(crate) fn grow_species(
    sp: Species,
    g: &mut Grid,
    plot: Rect,
    energy: f32,
    ink: &Ink,
    o: &GrowOpts,
    rng: &mut StdRng,
) {
    match sp {
        Species::Mangrove => grow_mangrove(g, plot, energy, ink, o, rng),
        Species::Colony => grow_colony(g, plot, energy, ink, o, rng),
        Species::Banyan => grow_banyan(g, plot, energy, ink, o, rng),
        Species::Bracket => grow_bracket(g, plot, energy, ink, o, rng),
        Species::Coral => grow_coral(g, plot, energy, ink, o, rng),
    }
}

// ── grid primitives ─────────────────────────────────────────────────

fn put(g: &mut Grid, x: i32, y: i32, ch: char, c: Color) {
    if x < 0 || y < 0 {
        return;
    }
    let (ux, uy) = (x as usize, y as usize);
    if uy >= g.len() {
        return;
    }
    let row = &mut g[uy];
    if ux >= row.len() {
        return;
    }
    row[ux] = Cell::new(ch, c);
}

fn put_soft(g: &mut Grid, x: i32, y: i32, ch: char, c: Color) {
    if x < 0 || y < 0 {
        return;
    }
    let (ux, uy) = (x as usize, y as usize);
    if uy >= g.len() {
        return;
    }
    let row = &mut g[uy];
    if ux >= row.len() {
        return;
    }
    if row[ux].ch == ' ' {
        row[ux] = Cell::new(ch, c);
    }
}

fn seg_glyph(dx: i32, dy: i32) -> char {
    if dy == 0 {
        return '─';
    }
    if dx == 0 {
        return '│';
    }
    if (dx > 0) == (dy > 0) { '╲' } else { '╱' }
}

fn stroke(g: &mut Grid, x0: i32, y0: i32, x1: i32, y1: i32, c: Color, soft: bool) {
    let dx = x1 - x0;
    let dy = y1 - y0;
    let n = dx.abs().max(dy.abs()).max(1);
    let (mut px, mut py) = (x0, y0);
    for i in 1..=n {
        let x = x0 + dx * i / n;
        let y = y0 + dy * i / n;
        let ch = seg_glyph(x - px, y - py);
        if soft {
            put_soft(g, x, y, ch, c);
        } else {
            put(g, x, y, ch, c);
        }
        px = x;
        py = y;
    }
}

fn rf(rng: &mut StdRng) -> f32 {
    rng.random::<f32>()
}

fn ri(rng: &mut StdRng, n: u32) -> u32 {
    if n <= 1 { 0 } else { rng.random_range(0..n) }
}

pub(crate) fn hash2(a: u64, b: u64) -> u64 {
    let mut x = a
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add(b.wrapping_mul(0xC2B2_AE3D_27D4_EB4F))
        .wrapping_add(0x1656_67B1);
    x ^= x >> 29;
    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 32;
    x
}

/// Coherent-edged leaf mass. The per-column edge table keeps the silhouette
/// ragged instead of speckled.
fn leaf_blob(g: &mut Grid, cx: i32, cy: i32, rx: i32, ry: i32, ink: &Ink, rng: &mut StdRng) {
    let rx = rx.max(1);
    let ry = ry.max(1);
    let (rxf, ryf) = (rx as f32, ry as f32);
    let span = (rx * 2 + 1) as usize;
    let mut edge = Vec::with_capacity(span);
    let mut e = 0.9;
    for _ in 0..span {
        e = (e + (rf(rng) - 0.5) * 0.5).clamp(0.45, 1.15);
        edge.push(e);
    }
    for dy in -ry..=ry {
        let fy = dy as f32 / ryf;
        let fy = if dy > 0 { fy * 1.35 } else { fy };
        for dx in -rx..=rx {
            let fx = dx as f32 / rxf;
            let d = fx * fx + fy * fy;
            if d > edge[(dx + rx) as usize] {
                continue;
            }
            let ch = if d < 0.24 {
                '▓'
            } else if d < 0.55 {
                '▒'
            } else if d < 0.82 {
                '░'
            } else {
                '·'
            };
            let slot = if d < 0.42 { SLOT_BRANCH } else { SLOT_TIP };
            put_soft(g, cx + dx, cy + dy, ch, ink[slot as usize]);
        }
    }
}

fn hang_fruit(g: &mut Grid, x: i32, y: i32, ink: &Ink) {
    put(g, x, y, '╷', ink[SLOT_BARK as usize]);
    put(g, x, y + 1, '●', ink[SLOT_FRUIT as usize]);
}

// ── species 1: mangrove ─────────────────────────────────────────────
// Prop roots are power-curve arcs from a raised collar down to splayed feet.

fn grow_mangrove(
    g: &mut Grid,
    plot: Rect,
    energy: f32,
    ink: &Ink,
    o: &GrowOpts,
    rng: &mut StdRng,
) {
    let root_y = plot.y as i32 + plot.h as i32 - 1;
    let cx = plot.x as i32 + plot.w as i32 / 2;
    let hw = (plot.w as i32 / 2).max(2);
    let energy = energy.clamp(0.12, 1.0);
    let hgt = (((plot.h as f32) - 1.0) * energy).max(3.0) as i32;
    let top = root_y - hgt;
    let collar = (root_y - (hgt as f32 * 0.40) as i32).clamp(top + 1, root_y - 1);
    let legs = (2 + (o.roots * 4.0) as i32).clamp(2, 6);

    for side in [-1i32, 1] {
        for k in 0..legs {
            let frac = (k as f32 + 1.0) / legs as f32;
            let reach = ((hw as f32 * (0.25 + 0.75 * frac) * (0.7 + 0.6 * rf(rng))) as i32).max(2);
            let bend = 1.4 + 1.3 * rf(rng) + o.gnarl;
            let drop = (root_y - collar).max(1);
            let steps = (reach + drop).max(3);
            let (mut px, mut py) = (cx, collar);
            for s in 1..=steps {
                let u = s as f32 / steps as f32;
                let x = cx + side * (reach as f32 * u.powf(0.55)) as i32;
                let y = collar + (drop as f32 * u.powf(bend)) as i32;
                if x == px && y == py {
                    continue;
                }
                let ch = seg_glyph(x - px, y - py);
                let slot = if u < 0.5 { SLOT_BARK } else { SLOT_TRUNK };
                put(g, x, y, ch, ink[slot as usize]);
                px = x;
                py = y;
            }
            put(g, px, py, '╷', ink[SLOT_DIM as usize]);
        }
    }

    let collar_w = ((hw as f32) * 0.35) as i32;
    for dx in -collar_w..=collar_w {
        put(g, cx + dx, collar, if dx == 0 { '┴' } else { '─' }, ink[SLOT_BARK as usize]);
    }

    let spikes = ((hw as f32) * o.roots * 0.9) as i32;
    for _ in 0..spikes {
        let x = cx + (ri(rng, (hw * 3) as u32 + 1) as i32) - hw * 3 / 2;
        let tall = 1 + ri(rng, 2) as i32;
        for j in 0..tall {
            put_soft(g, x, root_y - j, '╵', ink[SLOT_DIM as usize]);
        }
    }

    let mut tx = cx;
    for y in (top..=collar).rev() {
        let frac = (collar - y) as f32 / (collar - top).max(1) as f32;
        if rf(rng) < o.gnarl * 0.28 {
            tx += if rf(rng) < 0.5 { -1 } else { 1 };
        }
        tx = tx.clamp(cx - hw / 3, cx + hw / 3);
        let ch = if frac < 0.4 { '┃' } else { '│' };
        let slot = if frac < 0.4 { SLOT_TRUNK } else { SLOT_BARK };
        put(g, tx, y, ch, ink[slot as usize]);
        if frac < 0.5 && rf(rng) < 0.18 {
            put_soft(g, tx + 1, y, '╎', ink[SLOT_DIM as usize]);
        }
    }

    let boughs = (2 + (o.branch * 4.0) as i32).clamp(2, 6);
    let mut side = if ri(rng, 2) == 0 { -1 } else { 1 };
    let span = (collar - top).max(2) as f32;
    for b in 0..boughs {
        let bf = b as f32 / boughs as f32;
        let y = top + (span * (0.06 + 0.62 * bf)) as i32 + ri(rng, 2) as i32;
        let len = ((hw as f32) * (0.35 + 0.6 * rf(rng)) * (0.4 + o.branch)) as i32;
        let len = len.max(2);
        let end_x = tx + side * len;
        let end_y = (y - len / 3).max(top - 1);
        put(g, tx, y, if side < 0 { '┤' } else { '├' }, ink[SLOT_BARK as usize]);
        stroke(g, tx, y, end_x, end_y, ink[SLOT_BRANCH as usize], false);
        let rx = ((len as f32) * 0.75).max(2.0) as i32;
        let ry = ((hgt as f32) * 0.13).clamp(1.0, rx as f32 * 0.7) as i32;
        let blob_y = (end_y - ry / 2).max(top + ry.max(1));
        leaf_blob(g, end_x, blob_y, rx, ry.max(1), ink, rng);
        if rf(rng) < o.fruit {
            hang_fruit(g, end_x + side, end_y + ry / 2 + 1, ink);
        }
        side = -side;
    }
    let crown_rx = ((hw as f32) * 0.72).max(2.0) as i32;
    let crown_ry = ((hgt as f32) * 0.18).max(1.0) as i32;
    leaf_blob(g, tx, top + crown_ry * 2 / 3, crown_rx, crown_ry, ink, rng);
}

// ── species 2: colony ───────────────────────────────────────────────
// Space colonization: attractors pull the nearest frontier node, then die.

struct ColonyNode {
    x: f32,
    y: f32,
    parent: i32,
    weight: u32,
}

fn grow_colony(g: &mut Grid, plot: Rect, energy: f32, ink: &Ink, o: &GrowOpts, rng: &mut StdRng) {
    let root_y = plot.y as i32 + plot.h as i32 - 1;
    let cx = plot.x as i32 + plot.w as i32 / 2;
    let energy = energy.clamp(0.12, 1.0);
    let hgt = (((plot.h as f32) - 1.0) * energy).max(4.0);
    let top = root_y as f32 - hgt;
    let rx = ((plot.w as f32) * 0.46).max(2.5);
    let ry = (hgt * 0.42).max(2.0);
    let crown_cy = top + ry * 0.95;
    let lean = (rf(rng) - 0.5) * 2.0 * rx * 0.25 * (0.4 + o.gnarl);

    let want = ((rx * ry) / 1.35) as usize;
    let n_att = want.clamp(16, 240);
    let squash = 0.55 + 0.5 * rf(rng);
    let squash_left = ri(rng, 2) == 0;
    let mut att: Vec<(f32, f32)> = Vec::with_capacity(n_att);
    let mut guard = 0usize;
    while att.len() < n_att && guard < n_att * 8 {
        guard += 1;
        let u = rf(rng) * 2.0 - 1.0;
        let v = rf(rng) * 2.0 - 1.0;
        if u * u + v * v > 1.0 {
            continue;
        }
        let side_scale = if (u < 0.0) == squash_left { squash } else { 1.0 };
        att.push((
            cx as f32 + lean + u * rx * side_scale,
            crown_cy + v * ry * 0.92,
        ));
    }
    if att.is_empty() {
        return;
    }

    let step = (1.0 + hgt * 0.045).clamp(1.0, 3.2);
    let infl = (rx.max(ry) * 0.95).clamp(4.0, 26.0);
    let kill = (step * 1.15).clamp(1.2, 5.0);
    let node_cap = 420usize;

    let mut nodes: Vec<ColonyNode> = vec![ColonyNode {
        x: cx as f32,
        y: root_y as f32,
        parent: -1,
        weight: 0,
    }];
    let mut head = 0usize;
    let mut guard = 0;
    loop {
        guard += 1;
        if guard > 400 {
            break;
        }
        let (hx, hy) = (nodes[head].x, nodes[head].y);
        let near = att
            .iter()
            .map(|a| {
                let dx = (a.0 - hx) * 0.5;
                let dy = a.1 - hy;
                dx * dx + dy * dy
            })
            .fold(f32::MAX, f32::min);
        if near <= infl * infl || hy <= top + ry * 0.4 {
            break;
        }
        let drift = lean * 0.02 + (rf(rng) - 0.5) * o.gnarl * 0.9;
        nodes.push(ColonyNode {
            x: hx + drift,
            y: hy - step,
            parent: head as i32,
            weight: 0,
        });
        head = nodes.len() - 1;
    }

    let _ = head;
    for _ in 0..34 {
        if att.is_empty() || nodes.len() >= node_cap {
            break;
        }
        let live = nodes.len();
        let mut pull: Vec<(f32, f32, u32)> = vec![(0.0, 0.0, 0); live];
        for a in att.iter() {
            let mut best = usize::MAX;
            let mut best_d = infl * infl;
            for (i, n) in nodes.iter().enumerate() {
                let dx = (a.0 - n.x) * 0.5;
                let dy = a.1 - n.y;
                let d = dx * dx + dy * dy;
                if d < best_d {
                    best_d = d;
                    best = i;
                }
            }
            if best != usize::MAX {
                let n = &nodes[best];
                let dx = a.0 - n.x;
                let dy = a.1 - n.y;
                let len = (dx * dx + dy * dy).sqrt().max(0.001);
                pull[best].0 += dx / len;
                pull[best].1 += dy / len;
                pull[best].2 += 1;
            }
        }

        let mut next: Vec<usize> = Vec::new();
        for i in 0..live {
            if pull[i].2 == 0 {
                continue;
            }
            let len = (pull[i].0 * pull[i].0 + pull[i].1 * pull[i].1).sqrt().max(0.001);
            let jx = (rf(rng) - 0.5) * 0.55;
            let jy = (rf(rng) - 0.5) * 0.3;
            let nx = nodes[i].x + (pull[i].0 / len + jx) * step * 1.8;
            let ny = nodes[i].y + (pull[i].1 / len + jy) * step;
            nodes.push(ColonyNode {
                x: nx,
                y: ny,
                parent: i as i32,
                weight: 0,
            });
            next.push(nodes.len() - 1);
            if nodes.len() >= node_cap {
                break;
            }
        }
        if next.is_empty() {
            break;
        }
        att.retain(|a| {
            !next.iter().any(|&ni| {
                let dx = (a.0 - nodes[ni].x) * 0.5;
                let dy = a.1 - nodes[ni].y;
                dx * dx + dy * dy < kill * kill
            })
        });
    }

    for i in (1..nodes.len()).rev() {
        let w = nodes[i].weight + 1;
        let p = nodes[i].parent;
        if p >= 0 {
            nodes[p as usize].weight += w;
        }
    }

    for i in 1..nodes.len() {
        let p = nodes[i].parent as usize;
        let w = nodes[i].weight;
        let slot = if w > 24 {
            SLOT_TRUNK
        } else if w > 6 {
            SLOT_BARK
        } else {
            SLOT_BRANCH
        };
        let (x0, y0) = (nodes[p].x.round() as i32, nodes[p].y.round() as i32);
        let (x1, y1) = (nodes[i].x.round() as i32, nodes[i].y.round() as i32);
        stroke(g, x0, y0, x1, y1, ink[slot as usize], false);
        if w > 24 && (y0 - y1).abs() > 0 {
            put(g, x0, y0, '┃', ink[SLOT_TRUNK as usize]);
        }
    }

    let cw = (plot.w + 6).max(4);
    let ch_h = (plot.h + 2).max(4);
    let cox = plot.x as i32 - 3;
    let coy = plot.y as i32 - 1;
    let mut mass = vec![0u8; cw * ch_h];
    for n in nodes.iter().skip(1) {
        if n.weight > 5 {
            continue;
        }
        let lx = n.x.round() as i32 - cox;
        let ly = n.y.round() as i32 - coy;
        for dy in -2i32..=2 {
            for dx in -3i32..=3 {
                let (ax, ay) = (lx + dx, ly + dy);
                if ax < 0 || ay < 0 || ax >= cw as i32 || ay >= ch_h as i32 {
                    continue;
                }
                let w = 6i32 - dx.abs() - 2 * dy.abs();
                if w <= 0 {
                    continue;
                }
                let cell = &mut mass[ay as usize * cw + ax as usize];
                *cell = cell.saturating_add(w as u8);
            }
        }
    }
    for ly in 0..ch_h {
        for lx in 0..cw {
            let m = mass[ly * cw + lx];
            if m < 2 {
                continue;
            }
            let ch = if m >= 12 {
                '▓'
            } else if m >= 7 {
                '▒'
            } else if m >= 4 {
                '░'
            } else {
                '·'
            };
            let slot = if m >= 8 { SLOT_BRANCH } else { SLOT_TIP };
            put_soft(g, cox + lx as i32, coy + ly as i32, ch, ink[slot as usize]);
        }
    }
    for n in nodes.iter().skip(1) {
        if n.weight > 1 || rf(rng) >= o.fruit * 0.35 {
            continue;
        }
        put(g, n.x.round() as i32, n.y.round() as i32, '●', ink[SLOT_FRUIT as usize]);
    }

    let flare = ((plot.w as f32) * 0.16).clamp(1.0, 6.0) as i32;
    for k in 1..=flare {
        let h = (flare - k) / 2;
        put_soft(g, cx - k, root_y - h, '╱', ink[SLOT_TRUNK as usize]);
        put_soft(g, cx + k, root_y - h, '╲', ink[SLOT_TRUNK as usize]);
    }
}

// ── species 3: banyan ───────────────────────────────────────────────
// Horizontal boughs drop aerial roots; roots that touch soil become pillars.

fn grow_banyan(g: &mut Grid, plot: Rect, energy: f32, ink: &Ink, o: &GrowOpts, rng: &mut StdRng) {
    let root_y = plot.y as i32 + plot.h as i32 - 1;
    let cx = plot.x as i32 + plot.w as i32 / 2;
    let hw = (plot.w as i32 / 2).max(2);
    let energy = energy.clamp(0.12, 1.0);
    let hgt = (((plot.h as f32) - 1.0) * energy).max(4.0) as i32;
    let top = root_y - hgt;
    let crotch = root_y - (hgt as f32 * 0.42) as i32;
    let tw = (plot.w as i32 / 14).clamp(0, 2);

    let mut tx = cx;
    for y in (crotch..=root_y).rev() {
        if rf(rng) < o.gnarl * 0.2 {
            tx += if rf(rng) < 0.5 { -1 } else { 1 };
        }
        tx = tx.clamp(cx - hw / 4, cx + hw / 4);
        for k in -tw..=tw {
            let ch = if k.abs() == tw { '│' } else { '┃' };
            let slot = if k.abs() == tw { SLOT_BARK } else { SLOT_TRUNK };
            put(g, tx + k, y, ch, ink[slot as usize]);
        }
        if rf(rng) < 0.22 {
            put(g, tx + tw, y, '╿', ink[SLOT_DIM as usize]);
        }
    }
    for k in 1..=(tw + 2) {
        put_soft(g, tx - tw - k, root_y, '╱', ink[SLOT_TRUNK as usize]);
        put_soft(g, tx + tw + k, root_y, '╲', ink[SLOT_TRUNK as usize]);
    }

    let boughs = (2 + (o.branch * 4.0) as i32).clamp(2, 6);
    let mut side = if ri(rng, 2) == 0 { -1 } else { 1 };
    let mut pillars: Vec<i32> = Vec::new();
    for b in 0..boughs {
        let y = crotch - (hgt as f32 * 0.06 * b as f32) as i32 - ri(rng, 2) as i32;
        if y <= top {
            break;
        }
        let len = ((hw as f32) * (0.45 + 0.55 * rf(rng)) * (0.45 + o.branch * 0.8)) as i32;
        let len = len.max(3);
        put(g, tx, y, if side < 0 { '┤' } else { '├' }, ink[SLOT_BARK as usize]);
        let mut bx = tx;
        let mut by = y;
        for s in 1..=len {
            bx += side;
            if s % 3 == 0 && by > top + 1 {
                by -= 1;
                put(g, bx, by, if side < 0 { '╱' } else { '╲' }, ink[SLOT_BRANCH as usize]);
            } else {
                put(g, bx, by, '─', ink[SLOT_BRANCH as usize]);
            }
            if s > 1 && rf(rng) < o.roots * 0.16 {
                let mut rxp = bx;
                let mut ry = by + 1;
                let mut reached = false;
                let drop = (root_y - by).max(1) as f32;
                while ry < root_y {
                    if rf(rng) < 0.16 {
                        rxp += if rf(rng) < 0.5 { -1 } else { 1 };
                    }
                    put_soft(g, rxp, ry, '│', ink[SLOT_BARK as usize]);
                    ry += 1;
                    let done = (ry - by) as f32 / drop;
                    if done > 0.45 && rf(rng) < 0.13 {
                        break;
                    }
                    if ry >= root_y {
                        reached = true;
                        break;
                    }
                }
                if reached {
                    pillars.push(rxp);
                    for py in (by + 1)..=root_y {
                        put(g, rxp, py, '┃', ink[SLOT_TRUNK as usize]);
                    }
                    put_soft(g, rxp - 1, root_y, '╱', ink[SLOT_DIM as usize]);
                    put_soft(g, rxp + 1, root_y, '╲', ink[SLOT_DIM as usize]);
                } else {
                    put(g, rxp, ry - 1, '╵', ink[SLOT_TIP as usize]);
                }
            }
        }
        put(g, bx + side, by, if side < 0 { '╴' } else { '╶' }, ink[SLOT_BRANCH as usize]);
        if rf(rng) < o.fruit {
            hang_fruit(g, bx, by + 1, ink);
        }
        side = -side;
    }

    let crown_h = (top.max(plot.y as i32)..crotch).len() as i32;
    let crown_h = crown_h.max(2);
    let mut wobble = 0.0f32;
    for j in 0..crown_h {
        let y = crotch - j;
        if y < plot.y as i32 {
            break;
        }
        let f = j as f32 / crown_h as f32;
        wobble = (wobble + (rf(rng) - 0.5) * 0.5).clamp(-0.6, 0.6);
        let prof = (1.05 - (f - 0.32).abs() * 0.95 + wobble * 0.2).clamp(0.08, 1.0);
        let half = ((hw as f32) * prof * (0.7 + o.branch * 0.45)) as i32;
        for dx in -half..=half {
            let e = dx.abs() as f32 / half.max(1) as f32;
            let ch = if e < 0.45 {
                '▓'
            } else if e < 0.75 {
                '▒'
            } else if e < 0.92 {
                '░'
            } else {
                '·'
            };
            let slot = if e < 0.5 { SLOT_BRANCH } else { SLOT_TIP };
            put_soft(g, tx + dx, y, ch, ink[slot as usize]);
        }
    }
    for p in pillars {
        put_soft(g, p, root_y - 1, '┃', ink[SLOT_TRUNK as usize]);
    }
}

// ── species 4: bracket ──────────────────────────────────────────────
// A tapering stipe with radial shelves; every shelf gets its own rim profile.

fn grow_bracket(g: &mut Grid, plot: Rect, energy: f32, ink: &Ink, o: &GrowOpts, rng: &mut StdRng) {
    let root_y = plot.y as i32 + plot.h as i32 - 1;
    let cx = plot.x as i32 + plot.w as i32 / 2;
    let hw = (plot.w as i32 / 2).max(2);
    let energy = energy.clamp(0.12, 1.0);
    let hgt = (((plot.h as f32) - 1.0) * energy).max(4.0) as i32;
    let top = root_y - hgt;

    let mat = ((hw as f32) * o.roots * 1.2) as i32;
    for k in -mat..=mat {
        if rf(rng) < 0.55 {
            let ch = if k.abs() < mat / 2 { '╌' } else { '·' };
            put_soft(g, cx + k, root_y, ch, ink[SLOT_DIM as usize]);
        }
    }

    let mut sx = cx;
    for y in (top..=root_y).rev() {
        let f = (root_y - y) as f32 / hgt as f32;
        if rf(rng) < o.gnarl * 0.25 {
            sx += if rf(rng) < 0.5 { -1 } else { 1 };
        }
        sx = sx.clamp(cx - hw / 3, cx + hw / 3);
        let half = (((1.0 - f) * (hw as f32) * 0.16) as i32).clamp(0, 2);
        for k in -half..=half {
            let ch = if k == 0 { '▓' } else { '▒' };
            let slot = if k == 0 { SLOT_TRUNK } else { SLOT_BARK };
            put(g, sx + k, y, ch, ink[slot as usize]);
        }
    }

    let gap = ((hgt as f32 * 0.12) as i32).max(2);
    let mut side = if ri(rng, 2) == 0 { -1i32 } else { 1 };
    let mut y = root_y - gap;
    let mut shelf = 0;
    while y > top + 1 {
        let f = (root_y - y) as f32 / hgt as f32;
        let r = ((hw as f32) * (0.9 - 0.5 * f) * (0.55 + 0.55 * rf(rng)) * (0.4 + o.branch * 0.9))
            as i32;
        let r = r.clamp(2, hw.max(2));
        let both = rf(rng) < 0.25;
        let thick = 1.2 + (hgt as f32) * 0.045;
        let droop = (thick * 1.4).clamp(0.0, (hgt as f32) * 0.1);
        for s in if both { vec![-1i32, 1] } else { vec![side] } {
            let mut lip = 1.0f32;
            for dx in 1..=r {
                lip = (lip + (rf(rng) - 0.5) * 0.35).clamp(0.6, 1.35);
                let u = dx as f32 / r as f32;
                let th = (((1.0 - u * u).max(0.0).sqrt() * lip * thick) as i32).max(1);
                let sag = (u * u * droop) as i32;
                let x = sx + s * dx;
                let top_y = y + sag;
                for kk in 0..th {
                    let ch = if kk == 0 {
                        '─'
                    } else if u < 0.6 {
                        '▓'
                    } else {
                        '▒'
                    };
                    let slot = if kk == 0 { SLOT_TIP } else { SLOT_BRANCH };
                    put(g, x, top_y + kk, ch, ink[slot as usize]);
                }
                put_soft(g, x, top_y + th, '◡', ink[SLOT_BARK as usize]);
                if u > 0.75 && rf(rng) < o.fruit * 0.6 {
                    put_soft(g, x, top_y + th + 1, '·', ink[SLOT_FRUIT as usize]);
                }
            }
            let tipx = sx + s * (r + 1);
            put_soft(g, tipx, y + droop as i32, if s < 0 { '╴' } else { '╶' }, ink[SLOT_TIP as usize]);
        }
        side = -side;
        shelf += 1;
        y -= gap + ri(rng, 2) as i32;
    }

    let cap_r = ((hw as f32) * (0.5 + 0.35 * rf(rng))).max(2.0) as i32;
    let cap_h = ((hgt as f32 * 0.06) as i32).clamp(1, 3);
    for dx in -cap_r..=cap_r {
        let u = dx.abs() as f32 / cap_r as f32;
        let th = ((1.0 - u * u).max(0.0).sqrt() * cap_h as f32) as i32;
        for k in 0..=th {
            let ch = if k == th { '▒' } else { '▓' };
            put(g, sx + dx, top - k, ch, ink[SLOT_BRANCH as usize]);
        }
        put_soft(g, sx + dx, top + 1, '◡', ink[SLOT_TIP as usize]);
    }
    let _ = shelf;
}

// ── species 5: coral ────────────────────────────────────────────────
// Diffusion-limited aggregation: walkers drift down and stick on contact.

fn grow_coral(g: &mut Grid, plot: Rect, energy: f32, ink: &Ink, o: &GrowOpts, rng: &mut StdRng) {
    let root_y = plot.y as i32 + plot.h as i32 - 1;
    let cx = plot.x as i32 + plot.w as i32 / 2;
    let energy = energy.clamp(0.12, 1.0);
    let hgt = ((((plot.h as f32) - 1.0) * energy).max(4.0)) as i32;
    let w = (plot.w).max(3);
    let h = (hgt as usize + 1).max(4);
    let ox = plot.x as i32;
    let oy = root_y - hgt;
    let mut occ = vec![false; w * h];
    let lc = (cx - ox).clamp(1, w as i32 - 2);

    let stalk = ((hgt as f32) * (0.16 + 0.12 * rf(rng))).max(2.0) as i32;
    for j in 0..=stalk {
        let ly = (h as i32 - 1 - j).max(0);
        occ[ly as usize * w + lc as usize] = true;
    }

    let area = (w * h) as f32;
    let want = (area * 0.5 * (0.45 + o.branch)) as usize;
    let particles = want.clamp(60, 4200);
    let max_steps = (h * 3 + 60).min(320);
    let rx = ((w as f32) * 0.46).max(2.0);
    let ry = ((h as f32) * 0.62).max(2.0);
    let base_y = h as f32 - 1.0;
    let head_y = base_y - stalk as f32 - ry * 0.55;
    let inside = |x: f32, y: f32| -> bool {
        let u = (x - lc as f32) / rx;
        let v = (y - head_y) / ry;
        u * u + v * v <= 1.0
    };
    let max_steps = (h * 3 + 60).min(360);

    for _ in 0..particles {
        let ang = rf(rng) * std::f32::consts::TAU;
        let mut px = (lc as f32 + rx * ang.cos() * 0.96) as i32;
        let mut py = (head_y + ry * ang.sin() * 0.96) as i32;
        px = px.clamp(0, w as i32 - 1);
        py = py.clamp(0, h as i32 - 1);
        let mut stuck = false;
        for _ in 0..max_steps {
            let mut touch = false;
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    let nx = px + dx;
                    let ny = py + dy;
                    if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                        continue;
                    }
                    if occ[ny as usize * w + nx as usize] {
                        touch = true;
                    }
                }
            }
            if touch {
                stuck = true;
                break;
            }
            if rf(rng) < 0.45 {
                px += if (px as f32) > lc as f32 { -1 } else { 1 };
                py += if (py as f32) > head_y { -1 } else { 1 };
            } else {
                px += ri(rng, 3) as i32 - 1;
                py += ri(rng, 3) as i32 - 1;
            }
            if px < 0 || py < 0 || px >= w as i32 || py >= h as i32 {
                break;
            }
            if !inside(px as f32, py as f32) && py < (base_y - stalk as f32) as i32 {
                break;
            }
        }
        if stuck && inside(px as f32, py as f32) {
            occ[py as usize * w + px as usize] = true;
        }
    }

    for _ in 0..1 {
        let snapshot = occ.clone();
        for ly in 1..h - 1 {
            for lx in 1..w - 1 {
                if snapshot[ly * w + lx] {
                    continue;
                }
                let mut n = 0;
                for dy in -1i32..=1 {
                    for dx in -1i32..=1 {
                        if dx == 0 && dy == 0 {
                            continue;
                        }
                        let nx = (lx as i32 + dx) as usize;
                        let ny = (ly as i32 + dy) as usize;
                        if snapshot[ny * w + nx] {
                            n += 1;
                        }
                    }
                }
                if n >= 5 {
                    occ[ly * w + lx] = true;
                }
            }
        }
    }

    for ly in 0..h {
        for lx in 0..w {
            if !occ[ly * w + lx] {
                continue;
            }
            let mut n = 0;
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    let nx = lx as i32 + dx;
                    let ny = ly as i32 + dy;
                    if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                        continue;
                    }
                    if occ[ny as usize * w + nx as usize] {
                        n += 1;
                    }
                }
            }
            let f = ly as f32 / h as f32;
            let occ_at = |dx: i32, dy: i32| -> bool {
                let nx = lx as i32 + dx;
                let ny = ly as i32 + dy;
                nx >= 0 && ny >= 0 && nx < w as i32 && ny < h as i32 && occ[ny as usize * w + nx as usize]
            };
            let vert = occ_at(0, -1) || occ_at(0, 1);
            let horiz = occ_at(-1, 0) || occ_at(1, 0);
            let ch = if n >= 5 {
                '▓'
            } else if n == 4 {
                '▒'
            } else if vert && horiz {
                '┼'
            } else if vert {
                '│'
            } else if horiz {
                '─'
            } else if occ_at(-1, -1) || occ_at(1, 1) {
                '╲'
            } else if occ_at(1, -1) || occ_at(-1, 1) {
                '╱'
            } else {
                '·'
            };
            let slot = if f > 0.82 {
                SLOT_TRUNK
            } else if n >= 4 {
                SLOT_BRANCH
            } else if f < 0.35 {
                SLOT_TIP
            } else {
                SLOT_BARK
            };
            put(g, ox + lx as i32, oy + ly as i32, ch, ink[slot as usize]);
            if n <= 1 && f < 0.6 && rf(rng) < o.fruit * 0.25 {
                put(g, ox + lx as i32, oy + ly as i32, '●', ink[SLOT_FRUIT as usize]);
            }
        }
    }

    let flare = ((w as f32) * 0.12).clamp(1.0, 5.0) as i32;
    for k in 1..=flare {
        put_soft(g, ox + lc - k, root_y - (flare - k) / 2, '╱', ink[SLOT_TRUNK as usize]);
        put_soft(g, ox + lc + k, root_y - (flare - k) / 2, '╲', ink[SLOT_TRUNK as usize]);
    }
    for j in 0..stalk.min(3) {
        put_soft(g, ox + lc, root_y - j, '┃', ink[SLOT_TRUNK as usize]);
    }
}

// ── baked sprites ───────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub(crate) struct StampCell {
    pub(crate) x: u16,
    pub(crate) y: u16,
    pub(crate) ch: char,
    pub(crate) slot: u8,
    pub(crate) grp: u8,
}

#[derive(Clone, Default)]
pub(crate) struct Stamp {
    pub(crate) w: u16,
    pub(crate) h: u16,
    pub(crate) cells: Vec<StampCell>,
}

/// Grow into a scratch grid, then keep only the painted cells with their slots.
pub(crate) fn bake(w: usize, h: usize, grp: u8, f: impl FnOnce(&mut Grid)) -> Stamp {
    let mut scratch: Grid = vec![vec![Cell::blank(); w.max(1)]; h.max(1)];
    f(&mut scratch);
    let mut cells = Vec::new();
    for (y, row) in scratch.iter().enumerate() {
        for (x, c) in row.iter().enumerate() {
            if c.ch == ' ' {
                continue;
            }
            let slot = match c.fg {
                Color::AnsiValue(v) => v.min((SLOTS - 1) as u8),
                _ => SLOT_BRANCH,
            };
            cells.push(StampCell {
                x: x.min(u16::MAX as usize) as u16,
                y: y.min(u16::MAX as usize) as u16,
                ch: c.ch,
                slot,
                grp,
            });
        }
    }
    Stamp {
        w: w.min(u16::MAX as usize) as u16,
        h: h.min(u16::MAX as usize) as u16,
        cells,
    }
}

// ── sample sheet mode ───────────────────────────────────────────────

pub(crate) struct Opus2TreesKnobs {
    pub(crate) energy: f32,
    pub(crate) fruit: f32,
    pub(crate) branch: f32,
    pub(crate) gnarl: f32,
    pub(crate) roots: f32,
    pub(crate) sway: f32,
    pub(crate) flick: f32,
    pub(crate) hue: f32,
    pub(crate) scrub: f32,
}

impl Opus2TreesKnobs {
    pub(crate) fn from_env() -> Self {
        Opus2TreesKnobs {
            energy: param_f32("ENERGY", 0.92).clamp(0.1, 1.0),
            fruit: param_f32("FRUIT", 0.3).clamp(0.0, 1.0),
            branch: param_f32("BRANCH", 0.7).clamp(0.0, 1.0),
            gnarl: param_f32("GNARL", 0.35).clamp(0.0, 1.0),
            roots: param_f32("ROOTS", 0.6).clamp(0.0, 1.0),
            sway: param_f32("SWAY", 1.0).clamp(0.0, 4.0),
            flick: param_f32("FLICK", 1.0).clamp(0.0, 6.0),
            hue: param_f32("HUE", 0.0).clamp(-180.0, 180.0),
            scrub: param_f32("SCRUB", 0.72).clamp(0.1, 1.0),
        }
    }

    fn bits(&self) -> [u32; 7] {
        [
            self.energy.to_bits(),
            self.fruit.to_bits(),
            self.branch.to_bits(),
            self.gnarl.to_bits(),
            self.roots.to_bits(),
            self.hue.to_bits(),
            self.scrub.to_bits(),
        ]
    }
}

type SheetKey = (usize, usize, u64, [u32; 7]);

thread_local! {
    static SHEET: RefCell<Option<(SheetKey, Stamp, usize)>> = const { RefCell::new(None) };
}

fn sheet_inks(palette: &[Color; 5], k: &Opus2TreesKnobs) -> Vec<Ink> {
    let mut out = Vec::with_capacity(12);
    for i in 0..SPECIES.len() {
        let f = i as f32 / (SPECIES.len() - 1).max(1) as f32;
        let base = lerp_color(palette[1], palette[3], f);
        let base = crate::color::shift_hue(base, (k.hue + f * 18.0) as f64);
        for row in 0..2 {
            let b = if row == 0 { base } else { darken(base, 18) };
            out.push(ink_from(b, palette[2]));
        }
    }
    out.push([
        darken(palette[4], 120),
        darken(palette[4], 100),
        darken(palette[4], 80),
        darken(palette[4], 60),
        palette[2],
        darken(palette[4], 140),
    ]);
    out
}

fn build_sheet(width: usize, height: usize, seed: u64, k: &Opus2TreesKnobs) -> (Stamp, usize) {
    let cols = SPECIES.len();
    let cell_w = (width / cols).max(4);
    let rows = if height >= 26 { 2 } else { 1 };
    let split = if rows == 2 {
        ((height as f32) * 0.6) as usize
    } else {
        height
    };
    let bands = [(0usize, split), (split, height)];
    let furniture = (cols * 2) as u8;
    let ink = slot_ink();
    let mut page = Stamp {
        w: width as u16,
        h: height as u16,
        cells: Vec::new(),
    };

    for row in 0..rows {
        let (py, band_end) = bands[row];
        for (col, sp) in SPECIES.iter().enumerate() {
            let px = col * cell_w;
            let label_y = band_end.min(height).saturating_sub(1);
            let ground_y = label_y.saturating_sub(1);
            if ground_y <= py + 2 {
                continue;
            }
            let plot_w = cell_w.saturating_sub(2).max(3);
            let plot = Rect {
                x: 1,
                y: 1,
                w: plot_w,
                h: ground_y - py,
            };
            let energy = if row == 0 {
                k.energy
            } else {
                k.energy * k.scrub
            };
            let o = GrowOpts {
                fruit: k.fruit,
                branch: k.branch,
                gnarl: k.gnarl,
                roots: k.roots,
            };
            let grp = (col * 2 + row) as u8;
            let mut rng = StdRng::seed_from_u64(hash2(seed, grp as u64 + 17));
            let tile = bake(cell_w, ground_y - py + 1, grp, |g| {
                grow_species(*sp, g, plot, energy, &ink, &o, &mut rng);
            });
            for c in tile.cells {
                let gx = px + c.x as usize;
                let gy = py + c.y as usize;
                if gx >= width || gy >= height {
                    continue;
                }
                page.cells.push(StampCell {
                    x: gx as u16,
                    y: gy as u16,
                    ch: c.ch,
                    slot: c.slot,
                    grp,
                });
            }

            let mut frng = StdRng::seed_from_u64(hash2(seed, 900 + grp as u64));
            for x in px..(px + cell_w).min(width) {
                let ch = if ri(&mut frng, 5) == 0 { '╌' } else { '─' };
                page.cells.push(StampCell {
                    x: x as u16,
                    y: ground_y as u16,
                    ch,
                    slot: SLOT_DIM,
                    grp: furniture,
                });
            }
            let label = sp.label();
            let lx = px + cell_w / 2 - (label.len() / 2).min(cell_w / 2);
            for (j, ch) in label.chars().enumerate() {
                if lx + j >= width || label_y >= height {
                    break;
                }
                page.cells.push(StampCell {
                    x: (lx + j) as u16,
                    y: label_y as u16,
                    ch,
                    slot: SLOT_TIP,
                    grp: furniture,
                });
            }
        }
    }
    let ground_ref = split.max(1);
    (page, ground_ref)
}

fn blit_sheet(
    grid: &mut Grid,
    width: usize,
    height: usize,
    page: &Stamp,
    band: usize,
    inks: &[Ink],
    t: f32,
    k: &Opus2TreesKnobs,
    seed: u64,
) {
    let groups = inks.len();
    let animate = t > 0.0;
    let band = band.max(1);
    let mut shift = vec![0i16; groups.max(1) * (page.h as usize + 1)];
    if animate && k.sway > 0.0 {
        for g in 0..groups {
            let ph = (hash2(seed, g as u64 * 31 + 5) % 1000) as f32 / 1000.0;
            let amp = k.sway * (0.6 + 0.5 * ph);
            let w = 0.42 + 0.16 * ph;
            let s = (t * w + ph * 6.283).sin();
            for y in 0..=(page.h as usize) {
                let local = (y % band) as f32 / band as f32;
                let up = (1.0 - local).clamp(0.0, 1.0);
                shift[g * (page.h as usize + 1) + y] = (amp * s * up * up).round() as i16;
            }
        }
    }
    let flick_step = if animate && k.flick > 0.0 {
        (t * k.flick) as u64
    } else {
        0
    };
    let twinkle = ['·', '∘', '◦', '✦'];
    for c in &page.cells {
        let gi = (c.grp as usize).min(groups - 1);
        let mut x = c.x as i32;
        if animate && k.sway > 0.0 && (c.grp as usize) < groups - 1 {
            x += shift[gi * (page.h as usize + 1) + c.y as usize] as i32;
        }
        if x < 0 || x >= width as i32 || c.y as usize >= height {
            continue;
        }
        let mut ch = c.ch;
        if flick_step > 0 && c.slot == SLOT_TIP && ch != ' ' {
            let hh = hash2(
                (c.x as u64) << 20 | c.y as u64,
                flick_step + seed,
            );
            if hh % 11 == 0 {
                ch = twinkle[(hh >> 8) as usize % twinkle.len()];
            }
        }
        put(grid, x, c.y as i32, ch, inks[gi][c.slot as usize]);
    }
}

pub(crate) fn draw_opus_2_trees(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    k: &Opus2TreesKnobs,
) {
    if width < 4 || height < 4 {
        return;
    }
    measure_layer("opus-2-trees", "backdrop", || {
        for (y, row) in grid.iter_mut().take(height).enumerate() {
            let f = y as f32 / height.max(1) as f32;
            let bg = lerp_color(darken(palette[0], 8), lighten(palette[0], 10), f);
            for cell in row.iter_mut().take(width) {
                *cell = Cell::with_bg(' ', palette[4], bg);
            }
        }
    });

    measure_layer("opus-2-trees", "grow", || {
        let key: SheetKey = (width, height, seed, k.bits());
        SHEET.with(|s| {
            let mut slot = s.borrow_mut();
            let fresh = matches!(slot.as_ref(), Some((old, _, _)) if *old == key);
            if !fresh {
                let built = build_sheet(width, height, seed, k);
                *slot = Some((key, built.0, built.1));
            }
        });
    });

    let inks = measure_layer("opus-2-trees", "inks", || sheet_inks(palette, k));

    measure_layer("opus-2-trees", "blit", || {
        SHEET.with(|s| {
            let slot = s.borrow();
            if let Some((_, page, band)) = slot.as_ref() {
                blit_sheet(grid, width, height, page, *band, &inks, t, k, seed);
            }
        });
    });
}

pub(crate) fn cli_opus_2_trees(
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
    let mut k = Opus2TreesKnobs::from_env();
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
        k.gnarl = v;
    }
    if let Some(v) = args.get(8).and_then(|s| s.parse().ok()) {
        k.roots = v;
    }
    if let Some(v) = args.get(9).and_then(|s| s.parse().ok()) {
        k.sway = v;
    }
    if let Some(v) = args.get(10).and_then(|s| s.parse().ok()) {
        k.flick = v;
    }
    if let Some(v) = args.get(11).and_then(|s| s.parse().ok()) {
        k.hue = v;
    }
    if let Some(v) = args.get(12).and_then(|s| s.parse().ok()) {
        k.scrub = v;
    }
    draw_opus_2_trees(&mut grid, width, height, seed, &palette, t_anim, &k);
    (grid, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(w: usize, h: usize, seed: u64, t: f32) -> String {
        let mut g = vec![vec![Cell::blank(); w]; h];
        let p = crate::color::make_palette(seed);
        let k = Opus2TreesKnobs::from_env();
        draw_opus_2_trees(&mut g, w, h, seed, &p, t, &k);
        g.iter()
            .map(|row| row.iter().map(|c| c.ch).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn snapshot_opus_2_trees_static() {
        insta::assert_snapshot!("opus_2_trees_80x24_static", run(80, 24, 42, 0.0));
    }

    #[test]
    fn deterministic_and_seed_sensitive() {
        assert_eq!(run(80, 24, 42, 0.0), run(80, 24, 42, 0.0));
        assert_ne!(run(80, 24, 42, 0.0), run(80, 24, 9, 0.0));
    }

    #[test]
    fn time_moves_the_sheet() {
        assert_ne!(run(80, 24, 42, 0.0), run(80, 24, 42, 4.0));
    }
}
