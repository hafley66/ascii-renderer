//! heliotrope -- a light-starved tree reaching for a rotating tesseract sun.
//! @comment-ok: design record required by the mode brief
//! Author: Claude Opus 5 (1M context)
//! Written: 2026-09-18
//!
//! Signatures:
//!   fn stage(seed, w, h, &Knobs) -> Stage             frame geometry in plate units
//!   fn grow(seed, &Stage, &Knobs) -> Plant            space colonization under a polar shadow map
//!   fn hyper(seed, &Stage, &Knobs, t) -> Hyper        4D rotation, nested projection, emission lobes
//!   fn shade(&Stage, &Knobs, &Hyper, &[Pos], &Plant) -> Shade   per-frame polar occlusion around the light
//!   fn draw(frame, &[f32; KNOBS])                     layers: sky, motes, hyper, ground, roots, limbs, tips
//!
//! 1. Lifetime: every frame rebuilds Stage, Hyper, Shade from (seed, size, knobs, t). Plant depends
//!    on no time value; a thread-local memo keyed by (seed, size, structural knobs) skips regrowth.
//! 2. Stored state: Plant nodes (position, parent, light, path distance, tip count, width).
//! 3. Identity: every random draw is splitmix(seed, layer, index, slot). Nodes are identified by
//!    birth index, which is a pure function of the seed and structural knobs. No rng stream is read.
//! 4. Reads and writes per growth round: attractors read nodes (nearest, same kind); influenced
//!    nodes read the polar shadow map and the crowd grid; new nodes write both maps and kill attractors.
//! 5. Rewrite: canopy nodes bend between attractor pull, sun direction and crowd push; the growth
//!    gate and step length scale with transmitted light. Roots follow water attractors and gravity.
//! 6. Uniqueness: a candidate closer than 0.45 step to any node is dropped, so no two nodes coincide.
//! 7. Bounds: attractors <= 2800, nodes <= 6000, rounds <= 160, trunk bootstrap <= 400, nested
//!    tesseracts <= 3, motes <= 2400, polar bins <= 1440 by radial samples <= 480.
//! 8. Plate units: x = column / ASPECT, y = row; every write goes through a bounds-checked put.
//! 9. Paint order: sky and shafts, motes, tesseract (back edges first), ground, roots, limbs
//!    (thick before thin), tips.
//! 10. Animation: topology and identity never depend on t. Time moves the 4D rotation, the light
//!     centroid and its emission lobes, wind sway (a smooth field of position), sap pulses (phase of
//!     path distance) and motes (phase of fixed rays).
//! 11. Continuous knobs: PHOTO, CROWD, SPREAD, DENSITY, SHADE, ROOTS reshape the same growth;
//!     FOLD, SPIN reshape the light source; WIND, SAP move the grown body; HUE, ASPECT restyle it.
//! 12. Feedback: growth deposits occlusion that starves and slows later growth behind it, so the
//!     canopy domes toward the sun; crowd deposits push siblings apart; the grown canopy then
//!     casts the moving shafts that decide where grass and motes light up.

use crate::_0_profile::measure_layer;
use crate::color::{hsl_to_rgb, lerp_color, lighten};
use crate::opts::param_f32;
use crate::registry::{AnimKind, Mode, ModeFrame, Param};
use crate::types::{Cell, Grid};
use crossterm::style::Color;
use rayon::prelude::*;
use std::cell::RefCell;
use std::f32::consts::{PI, TAU};
use std::rc::Rc;

pub(super) struct Heliotrope;
pub(super) static MODE: Heliotrope = Heliotrope;

const NAME: &str = "heliotrope";
const KNOBS: usize = 12;
const HELP: &str = "heliotrope: a tree grown by light competition, reaching for a rotating tesseract sun [photo] [crowd] [spread] [density] [shade] [fold] [spin] [wind] [sap] [roots] [hue] [aspect]";

const PARAMS: &[Param] = &[
    param!("PHOTO", "phototropism", 0.0, 1.0, 0.55, 0.05),
    param!("CROWD", "self avoidance", 0.0, 1.0, 0.45, 0.05),
    param!("SPREAD", "canopy spread", 0.0, 1.0, 0.5, 0.05),
    param!("DENSITY", "attractor density", 0.2, 2.0, 1.0, 0.05),
    param!("SHADE", "leaf opacity", 0.0, 1.0, 0.5, 0.05),
    param!("FOLD", "4D perspective fold", 0.0, 1.0, 0.5, 0.05),
    param!("SPIN", "hypercube spin", 0.0, 1.0, 0.25, 0.05),
    param!("WIND", "wind sway", 0.0, 1.0, 0.3, 0.05),
    param!("SAP", "sap pulse speed", 0.0, 2.0, 0.6, 0.05),
    param!("ROOTS", "root share", 0.0, 1.0, 0.35, 0.05),
    param!("HUE", "sun hue", 0.0, 360.0, 42.0, 5.0),
    param!("ASPECT", "cols per row", 1.0, 3.0, 2.0, 0.1),
];

const L_STAGE: u64 = 0x51;
const L_ATTR: u64 = 0x52;
const L_ROOT: u64 = 0x53;
const L_GATE: u64 = 0x54;
const L_HYPER: u64 = 0x55;
const L_MOTE: u64 = 0x56;
const L_SOIL: u64 = 0x57;
const L_LEAF: u64 = 0x58;

const MAX_ATTR: usize = 2800;
const MAX_NODES: usize = 6000;
const MAX_ROUNDS: usize = 160;
const MAX_TRUNK: usize = 400;
const MAX_LEVELS: usize = 3;
const MAX_MOTES: usize = 2400;
const PARALLEL_MIN_CELLS: usize = 20_480;
const NONE: u32 = u32::MAX;

/// Screen slope glyphs, indexed by the segment angle in eighth turns.
const SLOPE: [char; 4] = ['_', '/', '|', '\\'];
const EDGE: [char; 4] = ['-', '/', '|', '\\'];

impl Mode for Heliotrope {
    fn name(&self) -> &'static str {
        NAME
    }
    fn help(&self) -> &'static str {
        HELP
    }
    fn animation(&self) -> AnimKind {
        AnimKind::Iterate
    }
    fn params(&self) -> &'static [Param] {
        PARAMS
    }
    fn render(&self, frame: &mut ModeFrame<'_>) {
        let p: [f32; KNOBS] = std::array::from_fn(|i| {
            let param = &PARAMS[i];
            let value = frame
                .args
                .get(i + 4)
                .and_then(|v| v.parse::<f32>().ok())
                .or_else(|| frame.param_values.and_then(|v| v.get(i)).copied())
                .unwrap_or_else(|| param_f32(param.key, param.default));
            if value.is_finite() {
                value.clamp(param.min, param.max)
            } else {
                param.default
            }
        });
        draw(frame, &p);
    }
}

/// Splitmix64 over (seed, layer, index, slot).
#[inline]
fn hash(seed: u64, layer: u64, index: u64, slot: u64) -> u64 {
    let mut z = seed
        ^ layer.wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ index.wrapping_mul(0xD1B5_4A32_D192_ED03)
        ^ slot.wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    z ^= z >> 30;
    z = z.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z ^= z >> 27;
    z = z.wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

#[inline]
fn unit(h: u64) -> f32 {
    (h >> 40) as f32 * (1.0 / 16_777_216.0)
}

#[inline]
fn smooth(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[derive(Clone, Copy)]
struct Knobs {
    photo: f32,
    crowd: f32,
    spread: f32,
    density: f32,
    shade: f32,
    fold: f32,
    spin: f32,
    wind: f32,
    sap: f32,
    roots: f32,
    hue: f32,
    aspect: f32,
}

impl Knobs {
    fn new(p: &[f32; KNOBS]) -> Self {
        Knobs {
            photo: p[0],
            crowd: p[1],
            spread: p[2],
            density: p[3],
            shade: p[4],
            fold: p[5],
            spin: p[6],
            wind: p[7],
            sap: p[8],
            roots: p[9],
            hue: p[10],
            aspect: p[11].max(0.5),
        }
    }

    /// Occlusion per unit of deposited wood, shared by growth and the rendered shafts.
    fn opacity(&self) -> f32 {
        0.06 + 0.5 * self.shade
    }
}

/// Frame geometry in plate units.
#[derive(Clone, Copy)]
struct Stage {
    fw: f32,
    fh: f32,
    aspect: f32,
    ground: f32,
    sx: f32,
    sy: f32,
    radius: f32,
    bx: f32,
    step: f32,
}

fn stage(seed: u64, w: usize, h: usize, k: &Knobs) -> Stage {
    let aspect = k.aspect;
    let fw = (w as f32 / aspect).max(1.0);
    let fh = (h as f32).max(1.0);
    let ext = fw.min(fh);
    let ground = fh * 0.8;
    let radius = (ext * 0.2).max(0.6);
    let sx = fw * (0.5 + 0.2 * (unit(hash(seed, L_STAGE, 0, 0)) - 0.5));
    let sy = (radius * 1.35 + 0.4).min(ground * 0.45);
    let bx = (sx + fw * 0.2 * (unit(hash(seed, L_STAGE, 0, 1)) - 0.5)).clamp(fw * 0.2, fw * 0.8);
    let step = 0.9 * (ext / 24.0).sqrt().max(1.0);
    Stage { fw, fh, aspect, ground, sx, sy, radius, bx, step }
}

#[derive(Clone, Copy)]
struct Node {
    x: f32,
    y: f32,
    parent: u32,
    under: bool,
    light: f32,
    dist: f32,
    tips: f32,
    width: f32,
    tip: bool,
}

struct Plant {
    nodes: Vec<Node>,
    /// Draw order: widest limbs first so twigs land on top.
    order: Vec<u32>,
    reach: f32,
}

/// Uniform buckets of node indices for nearest and spacing queries.
struct Buckets {
    cell: f32,
    cols: usize,
    rows: usize,
    slots: Vec<Vec<u32>>,
}

impl Buckets {
    fn new(fw: f32, fh: f32, cell: f32) -> Self {
        let cols = (fw / cell).ceil() as usize + 1;
        let rows = (fh / cell).ceil() as usize + 1;
        Buckets { cell, cols, rows, slots: vec![Vec::new(); cols * rows] }
    }
    fn coord(&self, x: f32, y: f32) -> (usize, usize) {
        let cx = ((x / self.cell).max(0.0) as usize).min(self.cols - 1);
        let cy = ((y / self.cell).max(0.0) as usize).min(self.rows - 1);
        (cx, cy)
    }
    fn insert(&mut self, x: f32, y: f32, id: u32) {
        let (cx, cy) = self.coord(x, y);
        self.slots[cy * self.cols + cx].push(id);
    }
    /// Nearest id accepted by `keep`, searching rings outward until no closer ring can win.
    fn nearest(&self, x: f32, y: f32, keep: impl Fn(u32) -> Option<f32>) -> Option<(f32, u32)> {
        let (cx, cy) = self.coord(x, y);
        let mut best: Option<(f32, u32)> = None;
        let rings = self.cols.max(self.rows);
        for ring in 0..=rings {
            if let Some((d2, _)) = best {
                let reach = (ring as f32 - 1.0).max(0.0) * self.cell;
                if reach * reach > d2 {
                    break;
                }
            }
            let (lo_x, hi_x) = (cx as i64 - ring as i64, cx as i64 + ring as i64);
            let (lo_y, hi_y) = (cy as i64 - ring as i64, cy as i64 + ring as i64);
            for by in lo_y.max(0)..=hi_y.min(self.rows as i64 - 1) {
                for bx in lo_x.max(0)..=hi_x.min(self.cols as i64 - 1) {
                    if by != lo_y && by != hi_y && bx != lo_x && bx != hi_x {
                        continue;
                    }
                    for &id in &self.slots[by as usize * self.cols + bx as usize] {
                        if let Some(d2) = keep(id) {
                            if best.is_none_or(|b| d2 < b.0) {
                                best = Some((d2, id));
                            }
                        }
                    }
                }
            }
        }
        best
    }

    fn near(&self, x: f32, y: f32, mut f: impl FnMut(u32)) {
        let (cx, cy) = self.coord(x, y);
        for by in cy.saturating_sub(1)..=(cy + 1).min(self.rows - 1) {
            for bx in cx.saturating_sub(1)..=(cx + 1).min(self.cols - 1) {
                for &id in &self.slots[by * self.cols + bx] {
                    f(id);
                }
            }
        }
    }
}

/// Wood deposited around the sun, in polar bins; light at a point is the
/// exponential of the wood between it and the sun.
struct Polar {
    cx: f32,
    cy: f32,
    bins: usize,
    unit: f32,
    radial: usize,
    wood: Vec<f32>,
}

impl Polar {
    fn new(cx: f32, cy: f32, bins: usize, unit: f32, reach: f32) -> Self {
        let radial = ((reach / unit).ceil() as usize + 2).min(480);
        Polar { cx, cy, bins, unit, radial, wood: vec![0.0; bins * radial] }
    }
    #[inline]
    fn locate(&self, x: f32, y: f32) -> (usize, usize, f32) {
        let (dx, dy) = (x - self.cx, y - self.cy);
        let r = (dx * dx + dy * dy).sqrt();
        let a = ((dy.atan2(dx) + PI) / TAU * self.bins as f32) as usize % self.bins;
        let ri = ((r / self.unit) as usize).min(self.radial - 1);
        (a, ri, r)
    }
    /// Deposit wood of width `thick` at a point, smeared over the bins it subtends.
    fn deposit(&mut self, x: f32, y: f32, thick: f32, amount: f32) {
        let (a, ri, r) = self.locate(x, y);
        let half = (thick * 0.5).atan2(r.max(0.3));
        let span = ((half / (TAU / self.bins as f32)).round() as usize).min(8);
        let weight = amount / (2 * span + 1) as f32;
        for o in 0..=2 * span {
            let b = (a + self.bins + o - span) % self.bins;
            self.wood[b * self.radial + ri] += weight;
        }
    }
    /// Convert deposits into running totals along each ray.
    fn accumulate(&mut self) {
        for b in 0..self.bins {
            let row = &mut self.wood[b * self.radial..(b + 1) * self.radial];
            let mut run = 0.0;
            for v in row.iter_mut() {
                let here = *v;
                *v = run;
                run += here;
            }
        }
    }
    /// Wood strictly between the sun and radial sample `ri`, before accumulation.
    fn between(&self, a: usize, ri: usize) -> f32 {
        let row = &self.wood[a * self.radial..a * self.radial + ri.saturating_sub(1)];
        row.iter().sum()
    }
}

fn grow(seed: u64, st: &Stage, k: &Knobs) -> Plant {
    // Attractors are fixed seeded samples mapped through continuous envelopes.
    let area = st.fw * st.fh;
    let total = ((k.density * (area / 5.0).clamp(40.0, 1400.0)) as usize).min(MAX_ATTR);
    let under_n = (total as f32 * k.roots * 0.6) as usize;
    let over_n = total - under_n;
    let top = st.sy + st.radius * 1.45;
    let floor = st.ground - (st.ground - top).max(0.0) * 0.38;
    let half = PI * (0.22 + 0.6 * k.spread);
    let skew = (unit(hash(seed, L_STAGE, 1, 0)) - 0.5) * 0.6;
    let (rx, ry) = (st.fw * (0.2 + 0.26 * k.spread), (st.ground - st.sy) * 0.98);
    let rmin = st.radius * 1.45;
    let mut attr: Vec<(f32, f32, bool)> = Vec::with_capacity(total);
    for i in 0..over_n as u64 {
        let u = unit(hash(seed, L_ATTR, i, 0));
        let v = unit(hash(seed, L_ATTR, i, 1));
        let phi = skew + (2.0 * u - 1.0) * half;
        let (s, c) = phi.sin_cos();
        let rmax = 1.0 / ((s / rx).powi(2) + (c / ry).powi(2)).sqrt();
        if rmax <= rmin {
            continue;
        }
        let r = rmin + (rmax - rmin) * v.powf(0.8);
        let (x, y) = (st.sx + s * r, st.sy + c * r);
        if x < 0.5 || x > st.fw - 0.5 || y < 0.4 || y > floor {
            continue;
        }
        attr.push((x, y, false));
    }
    let soil = st.fh - st.ground;
    if soil > 1.5 {
        let reach = st.fw * (0.12 + 0.3 * k.spread);
        for j in 0..under_n as u64 {
            let u = unit(hash(seed, L_ROOT, j, 0));
            let v = unit(hash(seed, L_ROOT, j, 1));
            let y = st.ground + 0.8 + (soil - 1.1) * v.powf(0.7);
            let x = st.bx + (2.0 * u - 1.0) * reach * (1.0 - 0.35 * v);
            if x < 0.3 || x > st.fw - 0.3 || y > st.fh - 0.3 {
                continue;
            }
            attr.push((x, y, true));
        }
    }

    let step = st.step;
    let di = step * 6.0;
    let dk = step * 1.5;
    let reach = {
        let far = |x: f32, y: f32| ((x - st.sx).powi(2) + (y - st.sy).powi(2)).sqrt();
        far(0.0, st.fh).max(far(st.fw, st.fh)).max(far(0.0, 0.0)).max(far(st.fw, 0.0))
    };
    let mut polar = Polar::new(st.sx, st.sy, 96, step, reach);
    let crowd_cell = step;
    let ccols = (st.fw / crowd_cell).ceil() as usize + 2;
    let crows = (st.fh / crowd_cell).ceil() as usize + 2;
    let mut crowd = vec![0.0f32; ccols * crows];
    let mut buckets = Buckets::new(st.fw, st.fh, di);
    let mut nodes: Vec<Node> = Vec::with_capacity(1024);
    let opacity = k.opacity();

    let add = |nodes: &mut Vec<Node>,
               buckets: &mut Buckets,
               polar: &mut Polar,
               crowd: &mut Vec<f32>,
               x: f32,
               y: f32,
               parent: u32,
               under: bool| {
        let id = nodes.len() as u32;
        nodes.push(Node { x, y, parent, under, light: 1.0, dist: 0.0, tips: 0.0, width: 0.0, tip: true });
        buckets.insert(x, y, id);
        if !under {
            polar.deposit(x, y, step, 1.0);
        }
        let cx = ((x / crowd_cell) as usize + 1).min(ccols - 2);
        let cy = ((y / crowd_cell) as usize + 1).min(crows - 2);
        for oy in 0..3 {
            for ox in 0..3 {
                let wgt = if ox == 1 && oy == 1 { 1.0 } else if ox == 1 || oy == 1 { 0.5 } else { 0.25 };
                crowd[(cy + oy - 1) * ccols + cx + ox - 1] += wgt;
            }
        }
    };
    let light_at = |polar: &Polar, x: f32, y: f32| {
        let (a, ri, _) = polar.locate(x, y);
        (-opacity * polar.between(a, ri)).exp()
    };

    add(&mut nodes, &mut buckets, &mut polar, &mut crowd, st.bx, st.ground, NONE, false);
    if soil > 1.5 {
        add(&mut nodes, &mut buckets, &mut polar, &mut crowd, st.bx, st.ground + 0.45, 0, true);
    }

    // Bootstrap: the trunk climbs until some attractor can see it.
    let sees = |x: f32, y: f32, under: bool, attr: &[(f32, f32, bool)]| {
        attr.iter().any(|&(ax, ay, u)| u == under && (ax - x).powi(2) + (ay - y).powi(2) < di * di)
    };
    let lean = (st.sx - st.bx) / (st.ground - st.sy).max(1.0);
    let mut last = 0usize;
    for _ in 0..MAX_TRUNK {
        let n = nodes[last];
        if attr.iter().all(|a| a.2) || n.y < floor + step || (n.y < st.ground - 2.0 * step && sees(n.x, n.y, false, &attr) && n.y < floor + di * 0.5) {
            break;
        }
        let (dx, dy) = (lean * k.photo, -1.0);
        let len = (dx * dx + dy * dy).sqrt();
        let (x, y) = (n.x + dx / len * step, n.y + dy / len * step);
        add(&mut nodes, &mut buckets, &mut polar, &mut crowd, x, y, last as u32, false);
        last = nodes.len() - 1;
    }
    if soil > 1.5 {
        let mut last = 1usize;
        for _ in 0..MAX_TRUNK {
            let n = nodes[last];
            if !attr.iter().any(|a| a.2) || sees(n.x, n.y, true, &attr) || n.y > st.fh - 0.6 {
                break;
            }
            add(&mut nodes, &mut buckets, &mut polar, &mut crowd, n.x, n.y + step * 0.8, last as u32, true);
            last = nodes.len() - 1;
        }
    }

    let mut live = vec![true; attr.len()];
    let mut pull: Vec<(f32, f32, u32)> = Vec::new();
    let min2 = (step * 0.45).powi(2);
    for round in 0..MAX_ROUNDS {
        if nodes.len() >= MAX_NODES {
            break;
        }
        pull.clear();
        pull.resize(nodes.len(), (0.0, 0.0, 0));
        // Each live attractor pulls its nearest node of the same kind.
        for (i, &(ax, ay, under)) in attr.iter().enumerate() {
            if !live[i] {
                continue;
            }
            // Every attractor pulls its nearest node; beyond `di` the pull fades with distance.
            let found = buckets.nearest(ax, ay, |id| {
                let n = &nodes[id as usize];
                (n.under == under).then(|| (n.x - ax).powi(2) + (n.y - ay).powi(2))
            });
            let Some((d2, id)) = found else {
                continue;
            };
            if d2 < dk * dk {
                live[i] = false;
                continue;
            }
            let n = &nodes[id as usize];
            let d = d2.sqrt();
            let weight = (di / d).powi(2).min(1.0);
            let slot = &mut pull[id as usize];
            slot.0 += (ax - n.x) / d * weight;
            slot.1 += (ay - n.y) / d * weight;
            slot.2 += 1;
        }
        let born = nodes.len();
        for id in 0..born {
            let (px, py, count) = pull[id];
            if count == 0 || nodes.len() >= MAX_NODES {
                continue;
            }
            let n = nodes[id];
            let plen = (px * px + py * py).sqrt().max(1e-4);
            let (mut dx, mut dy) = (px / plen, py / plen);
            let cx = ((n.x / crowd_cell) as usize + 1).min(ccols - 2);
            let cy = ((n.y / crowd_cell) as usize + 1).min(crows - 2);
            let gx = crowd[cy * ccols + cx + 1] - crowd[cy * ccols + cx - 1];
            let gy = crowd[(cy + 1) * ccols + cx] - crowd[(cy - 1) * ccols + cx];
            let glen = 1.0 + (gx * gx + gy * gy).sqrt();
            dx -= gx / glen * k.crowd * 1.4;
            dy -= gy / glen * k.crowd * 1.4;
            let len;
            if n.under {
                dy += 0.3 * (1.0 - k.photo * 0.5);
                len = step * 0.85;
            } else {
                let light = light_at(&polar, n.x, n.y);
                let doubt = 0.5 + 0.5 * (wander(seed ^ 0x5eed, n.x / step, n.y / step) * 0.4 + round as f32 * 0.7).sin();
                if light + 0.12 < doubt * k.shade {
                    continue;
                }
                let (tx, ty) = (st.sx - n.x, st.sy - n.y);
                let tl = (tx * tx + ty * ty).sqrt().max(1e-4);
                let bend = k.photo * 0.9 * (0.35 + 0.65 * light);
                dx += tx / tl * bend;
                dy += ty / tl * bend;
                len = step * (0.45 + 0.55 * light);
            }
            // Wander: a smooth seeded angle field bends straight runs into arcs.
            let dl = (dx * dx + dy * dy).sqrt().max(1e-4);
            let swirl = wander(seed, n.x / step, n.y / step);
            dx = dx / dl + swirl.cos() * 0.45;
            dy = dy / dl + swirl.sin() * 0.45;
            let dl = (dx * dx + dy * dy).sqrt().max(1e-4);
            let mut x = (n.x + dx / dl * len).clamp(0.2, st.fw - 0.2);
            let mut y = n.y + dy / dl * len;
            // A limb that meets the sun's disk slides along its rim instead of stalling.
            let (ox, oy) = (x - st.sx, y - st.sy);
            let od = (ox * ox + oy * oy).sqrt();
            let rim = st.radius * 1.12;
            if !n.under && od < rim {
                let od = od.max(1e-3);
                x = st.sx + ox / od * rim;
                y = st.sy + oy / od * rim;
            }
            if n.under {
                if y < st.ground + 0.3 || y > st.fh - 0.2 {
                    continue;
                }
            } else {
                if y > st.ground - 0.3 || y < 0.2 || x < 0.2 || x > st.fw - 0.2 {
                    continue;
                }
            }
            let mut crowded = false;
            buckets.near(x, y, |other| {
                let o = &nodes[other as usize];
                if (o.x - x).powi(2) + (o.y - y).powi(2) < min2 {
                    crowded = true;
                }
            });
            if crowded {
                continue;
            }
            add(&mut nodes, &mut buckets, &mut polar, &mut crowd, x, y, id as u32, n.under);
        }
        if nodes.len() == born {
            break;
        }
    }

    // Pipe model: width follows the tips each limb feeds; light is the final shadow map.
    for i in (0..nodes.len()).rev() {
        let n = nodes[i];
        let own = if n.tips == 0.0 { 1.0 } else { n.tips };
        nodes[i].tips = own;
        if n.parent != NONE {
            let p = n.parent as usize;
            nodes[p].tips += own;
            nodes[p].tip = false;
        }
    }
    let wscale = step / 0.9;
    let mut reach_d = 1.0f32;
    for i in 0..nodes.len() {
        let n = nodes[i];
        let dist = if n.parent == NONE {
            0.0
        } else {
            let p = &nodes[n.parent as usize];
            p.dist + ((n.x - p.x).powi(2) + (n.y - p.y).powi(2)).sqrt()
        };
        nodes[i].dist = dist;
        reach_d = reach_d.max(dist);
        nodes[i].width = (0.26 * n.tips.powf(0.5) * wscale).min(st.fw * 0.08).min(3.2 * wscale);
        nodes[i].light = if n.under { 0.55 } else { light_at(&polar, n.x, n.y) };
    }
    let mut order: Vec<u32> = (0..nodes.len() as u32).collect();
    order.sort_by(|a, b| nodes[*b as usize].width.total_cmp(&nodes[*a as usize].width));
    Plant { nodes, order, reach: reach_d }
}

/// Smooth angle field from three seeded plane waves; nearby points turn alike.
fn wander(seed: u64, x: f32, y: f32) -> f32 {
    let mut a = 0.0;
    for i in 0..3u64 {
        let dir = unit(hash(seed, L_GATE, 9000 + i, 0)) * TAU;
        let freq = 0.08 + 0.12 * unit(hash(seed, L_GATE, 9000 + i, 1));
        let ph = unit(hash(seed, L_GATE, 9000 + i, 2)) * TAU;
        a += (freq * (x * dir.cos() + y * dir.sin()) + ph).sin() * 2.2;
    }
    a
}

thread_local! {
    static MEMO: RefCell<Option<(u64, Rc<Plant>)>> = const { RefCell::new(None) };
}

/// Grow once per (seed, size, structural knobs); animation never changes these.
fn plant(seed: u64, w: usize, h: usize, st: &Stage, k: &Knobs) -> Rc<Plant> {
    let mut key = hash(seed, w as u64, h as u64, 0x6865_6c69);
    for v in [k.photo, k.crowd, k.spread, k.density, k.shade, k.roots, k.aspect] {
        key = hash(key, v.to_bits() as u64, 0, 1);
    }
    MEMO.with(|slot| {
        if let Some((have, p)) = slot.borrow().as_ref() {
            if *have == key {
                return p.clone();
            }
        }
        let p = Rc::new(grow(seed, st, k));
        *slot.borrow_mut() = Some((key, p.clone()));
        p
    })
}

/// Projected nested tesseracts plus the light they cast.
struct Hyper {
    /// Per level: 16 vertices as (column, row, nearness 0..1).
    levels: Vec<[(f32, f32, f32); 16]>,
    lx: f32,
    ly: f32,
    lobes: Vec<(f32, f32)>,
}

fn rotate(v: &mut [f32; 4], i: usize, j: usize, a: f32) {
    let (s, c) = a.sin_cos();
    let (p, q) = (v[i], v[j]);
    v[i] = p * c - q * s;
    v[j] = p * s + q * c;
}

fn hyper(seed: u64, st: &Stage, k: &Knobs, t: f32) -> Hyper {
    let rate: [f32; 3] = std::array::from_fn(|i| 0.5 + unit(hash(seed, L_HYPER, i as u64, 0)));
    let phase: [f32; 3] = std::array::from_fn(|i| unit(hash(seed, L_HYPER, i as u64, 1)) * TAU);
    let d4 = 1.35 + 2.6 * (1.0 - k.fold);
    let norm = 3f32.sqrt() * d4 / (d4 - 1.0);
    let mut levels = Vec::with_capacity(MAX_LEVELS);
    let mut scale = 1.0f32;
    for l in 0..MAX_LEVELS {
        if l > 0 && st.radius * scale < 2.5 {
            break;
        }
        let sign = if l % 2 == 0 { 1.0 } else { -1.3 };
        let spin = t * k.spin * sign * (1.0 + 0.6 * l as f32);
        let mut verts = [(0.0f32, 0.0f32, 0.0f32); 16];
        for (i, out) in verts.iter_mut().enumerate() {
            let mut v: [f32; 4] = std::array::from_fn(|b| if i >> b & 1 == 1 { 1.0 } else { -1.0 });
            rotate(&mut v, 0, 3, spin * rate[0] + phase[0]);
            rotate(&mut v, 1, 2, spin * rate[1] * 0.7 + phase[1]);
            rotate(&mut v, 2, 3, spin * rate[2] * 0.45 + phase[2]);
            rotate(&mut v, 0, 2, 0.55);
            rotate(&mut v, 1, 2, 0.35);
            let k4 = d4 / (d4 - v[3]);
            let (x, y, z) = (v[0] * k4 / norm, v[1] * k4 / norm, v[2] * k4 / norm);
            let k3 = 4.0 / (4.0 - z);
            let px = (st.sx + x * k3 * st.radius * scale) * st.aspect;
            let py = st.sy + y * k3 * st.radius * scale;
            *out = (px, py, ((z + 1.0) * 0.5).clamp(0.0, 1.0) * 0.6 + 0.4 * ((k4 - 0.6) / 2.0).clamp(0.0, 1.0));
        }
        levels.push(verts);
        scale *= 0.4;
    }
    // Light center leans toward the nearest vertices; each vertex throws a lobe.
    let outer = &levels[0];
    let (mut wx, mut wy, mut ws) = (0.0, 0.0, 0.0);
    let mut lobes = Vec::with_capacity(16);
    for &(px, py, near) in outer.iter() {
        let (vx, vy) = (px / st.aspect - st.sx, py - st.sy);
        let wgt = near * near;
        wx += vx * wgt;
        wy += vy * wgt;
        ws += wgt;
        lobes.push((vy.atan2(vx), wgt));
    }
    let ws = ws.max(1e-4);
    Hyper { levels, lx: st.sx + 0.35 * wx / ws, ly: st.sy + 0.35 * wy / ws, lobes }
}

/// Swayed plate position of every node for this frame.
fn sway(plant: &Plant, st: &Stage, k: &Knobs, t: f32) -> Vec<(f32, f32)> {
    let amp = (st.step * 2.5 + st.fw * 0.02) * k.wind;
    let span = (st.ground - (st.sy - st.radius)).max(1.0);
    plant
        .nodes
        .iter()
        .map(|n| {
            if n.under || amp == 0.0 {
                return (n.x, n.y);
            }
            let rise = ((st.ground - n.y) / span).clamp(0.0, 1.0).powf(1.6);
            let gust = (t * 0.9 + n.y * 0.35).sin() * 0.7 + (t * 2.1 + n.x * 0.5 + n.y * 0.2).sin() * 0.3;
            (n.x + amp * rise * gust, n.y + amp * 0.15 * rise * (t * 1.3 + n.x * 0.4).sin())
        })
        .collect()
}

/// Per-frame occlusion of the swayed canopy as seen from the moving light.
struct Shade {
    polar: Polar,
    emit: Vec<f32>,
    opacity: f32,
}

impl Shade {
    #[inline]
    fn at(&self, x: f32, y: f32) -> (f32, f32, f32) {
        let (a, ri, r) = self.polar.locate(x, y);
        let t = (-self.opacity * self.polar.wood[a * self.polar.radial + ri]).exp();
        (t, self.emit[a], r)
    }
}

fn shade(st: &Stage, k: &Knobs, hy: &Hyper, pos: &[(f32, f32)], plant: &Plant) -> Shade {
    let bins = (((st.fw + st.fh) * 4.0) as usize).clamp(180, 1440);
    let unit = (st.fw.min(st.fh) / 240.0).max(0.5);
    let reach = (st.fw.powi(2) + st.fh.powi(2)).sqrt() + st.radius;
    let mut polar = Polar::new(hy.lx, hy.ly, bins, unit, reach);
    for (i, n) in plant.nodes.iter().enumerate() {
        if n.under || n.parent == NONE {
            continue;
        }
        let (x1, y1) = pos[i];
        let (x0, y0) = pos[n.parent as usize];
        let len = ((x1 - x0).powi(2) + (y1 - y0).powi(2)).sqrt();
        let samples = ((len / (unit * 0.7)).ceil() as usize).clamp(1, 64);
        let thick = n.width.max(0.35) + if n.tip { st.step * 0.8 } else { 0.0 };
        for s in 0..samples {
            let f = (s as f32 + 0.5) / samples as f32;
            polar.deposit(x0 + (x1 - x0) * f, y0 + (y1 - y0) * f, thick, thick.min(3.0) * len / samples as f32 / st.step);
        }
    }
    polar.accumulate();
    let emit = (0..bins)
        .map(|b| {
            let a = (b as f32 + 0.5) / bins as f32 * TAU - PI;
            let mut e = 0.0f32;
            for &(la, wgt) in &hy.lobes {
                let d = (a - la + PI).rem_euclid(TAU) - PI;
                e += wgt * (-(d / 0.16).powi(2)).exp();
            }
            0.2 + 0.8 * e.min(1.0)
        })
        .collect();
    Shade { polar, emit, opacity: k.opacity() * 0.8 }
}

struct Look {
    sky_top: Color,
    sky_low: Color,
    glow: Color,
    hot: Color,
    warm: Color,
    far: Color,
    bark: Color,
    bark_lit: Color,
    bloom: Color,
    sap: Color,
    soil: Color,
    root: Color,
    grass: Color,
}

impl Look {
    fn new(k: &Knobs, palette: &[Color; 5]) -> Self {
        let h = k.hue as f64;
        let bark = lerp_color(hsl_to_rgb(26.0, 0.42, 0.4), palette[1], 0.1);
        Look {
            sky_top: hsl_to_rgb((h + 215.0).rem_euclid(360.0), 0.45, 0.04),
            sky_low: hsl_to_rgb((h + 195.0).rem_euclid(360.0), 0.35, 0.12),
            glow: hsl_to_rgb(h, 0.7, 0.3),
            hot: hsl_to_rgb(h, 0.95, 0.8),
            warm: hsl_to_rgb((h + 12.0).rem_euclid(360.0), 0.85, 0.58),
            far: hsl_to_rgb((h + 35.0).rem_euclid(360.0), 0.45, 0.3),
            bark,
            bark_lit: lerp_color(lighten(bark, 40), hsl_to_rgb((h + 12.0).rem_euclid(360.0), 0.8, 0.62), 0.6),
            bloom: lerp_color(hsl_to_rgb((h + 160.0).rem_euclid(360.0), 0.7, 0.62), palette[3], 0.3),
            sap: hsl_to_rgb(h, 0.95, 0.72),
            soil: hsl_to_rgb(26.0, 0.35, 0.06),
            root: hsl_to_rgb(28.0, 0.32, 0.4),
            grass: lerp_color(hsl_to_rgb(95.0, 0.5, 0.4), palette[2], 0.25),
        }
    }
}

#[inline]
fn put(grid: &mut Grid, w: usize, h: usize, x: i64, y: i64, ch: char, fg: Color) {
    if x < 0 || y < 0 || x as usize >= w || y as usize >= h {
        return;
    }
    let cell = &mut grid[y as usize][x as usize];
    cell.ch = ch;
    cell.fg = fg;
}

/// Slope glyph for a screen-space direction.
#[inline]
fn slope(set: &[char; 4], dx: f32, dy: f32) -> char {
    let a = (-dy).atan2(dx).rem_euclid(PI);
    set[((a / (PI * 0.25)).round() as usize) % 4]
}

fn draw(frame: &mut ModeFrame<'_>, p: &[f32; KNOBS]) {
    let (w, h) = (frame.width.min(frame.grid.first().map_or(0, |r| r.len())), frame.height.min(frame.grid.len()));
    if w == 0 || h == 0 {
        return;
    }
    let k = Knobs::new(p);
    let seed = frame.seed;
    let t = frame.time;
    let st = stage(seed, w, h, &k);
    let look = Look::new(&k, frame.palette);
    let plant = measure_layer(NAME, "grow", || plant(seed, w, h, &st, &k));
    let hy = measure_layer(NAME, "hyper", || hyper(seed, &st, &k, t));
    let pos = sway(&plant, &st, &k, t);
    let sh = measure_layer(NAME, "light", || shade(&st, &k, &hy, &pos, &plant));
    let grid = &mut *frame.grid;
    measure_layer(NAME, "sky", || paint_sky(grid, w, h, &st, &hy, &sh, &look));
    measure_layer(NAME, "motes", || paint_motes(grid, w, h, seed, t, &st, &sh, &hy, &look));
    measure_layer(NAME, "tesseract", || paint_hyper(grid, w, h, &hy, &look));
    measure_layer(NAME, "ground", || paint_ground(grid, w, h, seed, &st, &sh, &look));
    measure_layer(NAME, "plant", || paint_plant(grid, w, h, seed, t, &st, &k, &plant, &pos, &look));
}

fn paint_sky(grid: &mut Grid, w: usize, h: usize, st: &Stage, hy: &Hyper, sh: &Shade, look: &Look) {
    let ground = st.ground;
    let paint = |(y, row): (usize, &mut Vec<Cell>)| {
        let fy = y as f32 + 0.5;
        let fall = (fy / ground).clamp(0.0, 1.0);
        let base = lerp_color(look.sky_top, look.sky_low, fall);
        for (x, cell) in row.iter_mut().enumerate().take(w) {
            if fy > ground {
                *cell = Cell::with_bg(' ', look.soil, lerp_color(look.soil, look.sky_top, 0.2 * (1.0 - fall)));
                continue;
            }
            let fx = (x as f32 + 0.5) / st.aspect;
            let (tr, emit, _) = sh.at(fx, fy);
            let r = ((fx - hy.lx).powi(2) + (fy - hy.ly).powi(2)).sqrt();
            let halo = 1.0 / (1.0 + (r / st.radius).powi(2) * 1.6);
            let shaft = tr * emit / (1.0 + r / (st.fh * 0.9));
            let lit = (0.6 * halo + 0.5 * shaft * shaft * (1.0 - halo)).clamp(0.0, 1.0);
            let umbra = lerp_color(base, look.sky_top, (1.0 - tr) * 0.6);
            *cell = Cell::with_bg(' ', look.far, lerp_color(umbra, look.glow, lit));
        }
    };
    let rows = &mut grid[..h];
    if w * h >= PARALLEL_MIN_CELLS {
        rows.par_iter_mut().enumerate().with_min_len(8).for_each(paint);
    } else {
        rows.iter_mut().enumerate().for_each(paint);
    }
}

/// Motes ride fixed rays outward from the light and show only where it reaches.
#[allow(clippy::too_many_arguments)]
fn paint_motes(grid: &mut Grid, w: usize, h: usize, seed: u64, t: f32, st: &Stage, sh: &Shade, hy: &Hyper, look: &Look) {
    let count = ((st.fw * st.fh / 14.0) as usize).clamp(12, MAX_MOTES);
    let reach = (st.fw.powi(2) + st.fh.powi(2)).sqrt() * 0.6;
    for j in 0..count as u64 {
        let a = unit(hash(seed, L_MOTE, j, 0)) * TAU;
        let speed = 0.015 + 0.03 * unit(hash(seed, L_MOTE, j, 1));
        let life = (unit(hash(seed, L_MOTE, j, 2)) + t * speed).fract();
        let r = st.radius * 1.3 + life * reach;
        let (x, y) = (hy.lx + a.cos() * r, hy.ly + a.sin() * r);
        if y >= st.ground - 0.5 {
            continue;
        }
        let (tr, emit, _) = sh.at(x, y);
        let fade = smooth(0.0, 0.15, life) * (1.0 - smooth(0.7, 1.0, life));
        let lum = tr * emit * fade / (1.0 + r / (st.fh * 0.7));
        if lum < 0.3 {
            continue;
        }
        let ch = if lum > 0.55 { '\'' } else { '.' };
        put(grid, w, h, (x * st.aspect) as i64, y as i64, ch, lerp_color(look.far, look.warm, lum));
    }
}

/// Line through cell space with a glyph per step; `f` decides color and may skip.
fn line(from: (f32, f32), to: (f32, f32), mut f: impl FnMut(i64, i64, f32)) {
    let (dx, dy) = (to.0 - from.0, to.1 - from.1);
    let steps = (dx.abs().max(dy.abs()) * 1.5).ceil().clamp(1.0, 4096.0) as usize;
    for s in 0..=steps {
        let u = s as f32 / steps as f32;
        f((from.0 + dx * u).floor() as i64, (from.1 + dy * u).floor() as i64, u);
    }
}

fn paint_hyper(grid: &mut Grid, w: usize, h: usize, hy: &Hyper, look: &Look) {
    let mut edges: Vec<(usize, usize, usize, f32)> = Vec::with_capacity(32 * hy.levels.len());
    for (l, verts) in hy.levels.iter().enumerate() {
        for i in 0..16usize {
            for b in 0..4 {
                let j = i ^ (1 << b);
                if j > i {
                    edges.push((l, i, j, (verts[i].2 + verts[j].2) * 0.5));
                }
            }
        }
    }
    edges.sort_by(|a, b| a.3.total_cmp(&b.3));
    for &(l, i, j, near) in &edges {
        let (a, b) = (hy.levels[l][i], hy.levels[l][j]);
        let ch = slope(&EDGE, b.0 - a.0, b.1 - a.1);
        let heat = (near + 0.25 * l as f32).clamp(0.0, 1.0);
        let fg = if heat > 0.5 { lerp_color(look.warm, look.hot, heat * 2.0 - 1.0) } else { lerp_color(look.far, look.warm, heat * 2.0) };
        let glyph = if near < 0.3 && l == 0 { '.' } else { ch };
        line((a.0, a.1), (b.0, b.1), |x, y, _| put(grid, w, h, x, y, glyph, fg));
    }
    for (l, verts) in hy.levels.iter().enumerate() {
        for &(x, y, near) in verts.iter() {
            let ch = if near > 0.66 { '@' } else if near > 0.33 { 'o' } else { '.' };
            put(grid, w, h, x.floor() as i64, y.floor() as i64, ch, lerp_color(look.warm, look.hot, near + 0.2 * l as f32));
        }
    }
    let (cx, cy) = hy.levels[0].iter().fold((0.0, 0.0), |acc, v| (acc.0 + v.0 / 16.0, acc.1 + v.1 / 16.0));
    put(grid, w, h, cx.floor() as i64, cy.floor() as i64, '*', look.hot);
}

fn paint_ground(grid: &mut Grid, w: usize, h: usize, seed: u64, st: &Stage, sh: &Shade, look: &Look) {
    let gy = st.ground.floor() as i64;
    for x in 0..w {
        let fx = (x as f32 + 0.5) / st.aspect;
        let (tr, emit, r) = sh.at(fx, st.ground - 0.5);
        let lit = (tr * emit / (1.0 + r / (st.fh * 0.6))).clamp(0.0, 1.0);
        let g = unit(hash(seed, L_SOIL, x as u64, 0));
        let (ch, fg) = if g < lit * 0.9 {
            ([',', '\'', '"', 'v'][(g * 97.0) as usize % 4], lerp_color(look.soil, look.grass, 0.35 + 0.65 * lit))
        } else {
            ('_', lerp_color(look.soil, look.root, 0.6))
        };
        put(grid, w, h, x as i64, gy, ch, fg);
        // Strata: wavy bedding planes, dotted where the plane crosses the row.
        let tilt = (unit(hash(seed, L_SOIL, 0, 1)) - 0.5) * 0.08;
        for y in (gy + 1).max(0)..h as i64 {
            let depth = y as f32 + 0.5 - st.ground;
            let plane = (depth * 1.1 + tilt * fx + 0.6 * (fx * 0.21 + (seed % 7) as f32).sin()).rem_euclid(2.4);
            let s = unit(hash(seed, L_SOIL, x as u64, y as u64 + 1));
            if plane < 0.5 && s < 0.4 {
                put(grid, w, h, x as i64, y, if s < 0.12 { ':' } else { '.' }, lerp_color(look.soil, look.root, 0.3 + 0.1 * depth.min(4.0)));
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_plant(grid: &mut Grid, w: usize, h: usize, seed: u64, t: f32, st: &Stage, k: &Knobs, plant: &Plant, pos: &[(f32, f32)], look: &Look) {
    let wave = st.step * 9.0;
    let aspect = st.aspect;
    for &id in &plant.order {
        let n = &plant.nodes[id as usize];
        if n.parent == NONE {
            continue;
        }
        let (x1, y1) = pos[id as usize];
        let (x0, y0) = pos[n.parent as usize];
        let (a, b) = ((x0 * aspect, y0), (x1 * aspect, y1));
        let ch = slope(&SLOPE, b.0 - a.0, b.1 - a.1);
        let flat = ch == '_';
        let signed = if n.under { -n.dist } else { n.dist };
        let phase = (signed / wave + t * k.sap).rem_euclid(1.0);
        let pulse = (-((phase - 0.5) * 7.0).powi(2)).exp() * if n.under { 0.45 } else { 0.35 + 0.65 * n.light };
        let body = if n.under { lerp_color(look.soil, look.root, 0.5 + 0.5 * (1.0 - n.dist / plant.reach)) } else { lerp_color(look.bark, look.bark_lit, n.light.powf(0.45)) };
        let fg = lerp_color(body, look.sap, pulse);
        let hr = n.width * 0.5;
        if hr < 0.45 {
            line(a, b, |x, y, u| {
                let fy = a.1 + (b.1 - a.1) * u;
                let glyph = if flat && fy - fy.floor() < 0.5 { '-' } else { ch };
                put(grid, w, h, x, y, glyph, fg)
            });
            continue;
        }
        let hw = hr * aspect;
        let rim = lerp_color(body, look.soil, 0.45);
        line(a, b, |x, y, u| {
            let (cx, cy) = (a.0 + (b.0 - a.0) * u, a.1 + (b.1 - a.1) * u);
            let _ = (x, y);
            for iy in (cy - hr).floor() as i64..=(cy + hr).floor() as i64 {
                for ix in (cx - hw).floor() as i64..=(cx + hw).floor() as i64 {
                    let ex = (ix as f32 + 0.5 - cx) / hw;
                    let ey = (iy as f32 + 0.5 - cy) / hr.max(0.5);
                    let e = ex * ex + ey * ey;
                    if e > 1.0 {
                        continue;
                    }
                    put(grid, w, h, ix, iy, ch, if e > 0.5 { rim } else { fg });
                }
            }
        });
    }
    // Tips: lit ends open into blooms, shaded ones stay buds.
    for (i, n) in plant.nodes.iter().enumerate() {
        if !n.tip || n.under || n.parent == NONE {
            continue;
        }
        let (x, y) = pos[i];
        let (cx, cy) = ((x * aspect).floor() as i64, y.floor() as i64);
        let signed = n.dist;
        let phase = (signed / wave + t * k.sap).rem_euclid(1.0);
        let flare = (-((phase - 0.5) * 7.0).powi(2)).exp();
        let ch = if n.light > 0.66 { '*' } else if n.light > 0.33 { '+' } else { '.' };
        let fg = lerp_color(lerp_color(look.bark, look.bloom, 0.3 + 0.7 * n.light), look.hot, flare * n.light * 0.6);
        put(grid, w, h, cx, cy, ch, fg);
        let petals = (n.light * 3.0 * (st.step / 0.9)).round() as u64;
        for q in 0..petals.min(6) {
            let hq = hash(seed, L_LEAF, i as u64, q);
            let (ox, oy) = ((unit(hq) * 2.0 - 1.0) * 2.0 * st.step, (unit(hq >> 7) * 2.0 - 1.0) * st.step);
            let (px, py) = (((x + ox / aspect) * aspect).floor() as i64, (y + oy).floor() as i64);
            if px >= 0 && py >= 0 && (px as usize) < w && (py as usize) < h && grid[py as usize][px as usize].ch == ' ' {
                put(grid, w, h, px, py, if hq & 1 == 0 { ',' } else { '`' }, lerp_color(look.bark, look.bloom, 0.5 * n.light));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::grid_to_plain;
    use rand::{SeedableRng, rngs::StdRng};

    fn knobs() -> Vec<f32> {
        PARAMS.iter().map(|p| p.default).collect()
    }

    fn frame(w: usize, h: usize, seed: u64, time: f32, values: &[f32]) -> Grid {
        let mut grid = vec![vec![Cell::blank(); w]; h];
        let palette = crate::color::make_palette(seed);
        let mut rng = StdRng::seed_from_u64(seed);
        MODE.render(&mut ModeFrame {
            grid: &mut grid,
            width: w,
            height: h,
            seed,
            palette: &palette,
            rng: &mut rng,
            time,
            args: &[],
            param_values: Some(values),
        });
        grid
    }

    fn text(grid: &Grid) -> String {
        grid_to_plain(grid).join("\n")
    }

    fn with(pairs: &[(usize, f32)]) -> Vec<f32> {
        let mut k = knobs();
        for &(i, v) in pairs {
            k[i] = v;
        }
        k
    }

    #[test]
    fn heliotrope_80x24() {
        insta::assert_snapshot!("heliotrope_80x24", text(&frame(80, 24, 42, 0.0, &knobs())));
    }

    #[test]
    fn heliotrope_80x24_t6() {
        insta::assert_snapshot!("heliotrope_80x24_t6", text(&frame(80, 24, 42, 6.0, &knobs())));
    }

    #[test]
    fn heliotrope_80x24_t19() {
        insta::assert_snapshot!("heliotrope_80x24_t19", text(&frame(80, 24, 42, 19.0, &knobs())));
    }

    #[test]
    fn heliotrope_wide_fold() {
        let k = with(&[(2, 0.95), (5, 1.0), (4, 0.85)]);
        insta::assert_snapshot!("heliotrope_80x24_spread_fold", text(&frame(80, 24, 7, 3.0, &k)));
    }

    #[test]
    fn heliotrope_small_clip() {
        insta::assert_snapshot!("heliotrope_26x9", text(&frame(26, 9, 42, 2.0, &knobs())));
    }

    #[test]
    fn heliotrope_photo_neighbors() {
        insta::assert_snapshot!("heliotrope_80x24_photo_050", text(&frame(80, 24, 42, 0.0, &with(&[(0, 0.5)]))));
        insta::assert_snapshot!("heliotrope_80x24_photo_060", text(&frame(80, 24, 42, 0.0, &with(&[(0, 0.6)]))));
    }

    #[test]
    fn deterministic_and_seed_sensitive() {
        let k = knobs();
        assert_eq!(text(&frame(90, 30, 42, 3.0, &k)), text(&frame(90, 30, 42, 3.0, &k)));
        assert_ne!(text(&frame(90, 30, 42, 3.0, &k)), text(&frame(90, 30, 9, 3.0, &k)));
    }

    #[test]
    fn time_moves_but_topology_holds() {
        let k = knobs();
        assert_ne!(text(&frame(90, 30, 42, 0.0, &k)), text(&frame(90, 30, 42, 4.0, &k)));
        let st = stage(42, 90, 30, &Knobs::new(&std::array::from_fn(|i| k[i])));
        let a = plant(42, 90, 30, &st, &Knobs::new(&std::array::from_fn(|i| k[i])));
        frame(90, 30, 42, 11.0, &k);
        let b = plant(42, 90, 30, &st, &Knobs::new(&std::array::from_fn(|i| k[i])));
        assert_eq!(a.nodes.len(), b.nodes.len());
    }

    #[test]
    fn growth_stays_bounded_at_extremes() {
        let top: Vec<f32> = PARAMS.iter().map(|p| p.max).collect();
        let low: Vec<f32> = PARAMS.iter().map(|p| p.min).collect();
        for k in [&top, &low] {
            let kn = Knobs::new(&std::array::from_fn(|i| k[i]));
            let st = stage(5, 600, 200, &kn);
            let g = grow(5, &st, &kn);
            assert!(g.nodes.len() <= MAX_NODES);
            for n in &g.nodes {
                assert!(n.x.is_finite() && n.y.is_finite());
            }
        }
    }

    #[test]
    fn tiny_grids_do_not_panic() {
        for (w, h) in [(1, 1), (2, 1), (1, 3), (5, 2), (12, 4), (0, 5), (5, 0)] {
            frame(w, h, 3, 1.5, &knobs());
        }
    }

    #[test]
    fn frame_cost() {
        let (w, h) = (200usize, 60usize);
        let k = knobs();
        let mut worst = 0.0f64;
        let start = std::time::Instant::now();
        for f in 0..60 {
            let t0 = std::time::Instant::now();
            frame(w, h, 42, f as f32 * 0.05, &k);
            worst = worst.max(t0.elapsed().as_secs_f64() * 1000.0);
        }
        let avg = start.elapsed().as_secs_f64() * 1000.0 / 60.0;
        eprintln!("heliotrope frame_cost 200x60: avg {:.3} ms, worst {:.3} ms", avg, worst);
        if !cfg!(debug_assertions) {
            assert!(avg < 12.0, "avg frame {:.3} ms", avg);
        }
    }

    #[test]
    #[ignore]
    fn preview() {
        let w: usize = std::env::var("HW").ok().and_then(|v| v.parse().ok()).unwrap_or(80);
        let h: usize = std::env::var("HH").ok().and_then(|v| v.parse().ok()).unwrap_or(24);
        let seed: u64 = std::env::var("HS").ok().and_then(|v| v.parse().ok()).unwrap_or(42);
        let t: f32 = std::env::var("HT").ok().and_then(|v| v.parse().ok()).unwrap_or(0.0);
        let mut k = knobs();
        if let Ok(spec) = std::env::var("HK") {
            for pair in spec.split(',') {
                if let Some((i, v)) = pair.split_once('=') {
                    k[i.parse::<usize>().unwrap()] = v.parse().unwrap();
                }
            }
        }
        let grid = frame(w, h, seed, t, &k);
        if let Ok(path) = std::env::var("HP") {
            let rgb = |c: Color| match c {
                Color::Rgb { r, g, b } => format!("{r},{g},{b}"),
                _ => "x".to_string(),
            };
            let mut out = format!("{w} {h}\n");
            for row in &grid {
                for c in row {
                    out.push_str(&format!("{} {} {}\n", c.ch as u32, rgb(c.fg), rgb(c.bg)));
                }
            }
            std::fs::write(path, out).unwrap();
        }
        println!("{}", text(&grid));
    }
}
