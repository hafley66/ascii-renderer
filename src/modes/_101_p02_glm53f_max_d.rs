//! Moon gate: a round aperture cut into an ice-ray lattice wall. A split
//! grammar derives the lattice from recursive boundary chords, hui-wen frets
//! frame the field, and a lantern wave travels the straps in derivation order.
use crate::_0_profile::measure_layer;
use crate::color::{darken, lerp_color, lighten};
use crate::opts::param_f32;
use crate::registry::{AnimKind, Mode, ModeFrame, Param};
use crate::types::{Cell, Grid};
use crossterm::style::Color;
use rayon::prelude::*;
use std::cell::RefCell;
use std::f32::consts::TAU;

pub(super) struct Moongate;
pub(super) static MODE: Moongate = Moongate;

const NAME: &str = "moongate";
const KNOBS: usize = 7;
const HELP: &str = "moongate: moon gate in an ice-ray wall, lantern wave along the derivation [gate] [crack] [fret] [root] [glow] [bloom] [drift]";

/// Bare wall flecks, in rising visual weight.
const WALL: [char; 4] = ['.', ':', '\'', ','];
/// Dust motes on the gate floor.
const DUST: char = '·';

/// Direction-class bits painted into the lattice mask.
const C_H: u8 = 1;
const C_V: u8 = 2;
const C_F: u8 = 4;
const C_B: u8 = 8;
/// Cross pairs that read as a full joint rather than a thin one.
const M_HV: u8 = C_H | C_V;
const M_FB: u8 = C_F | C_B;
/// The moon gate ring: a bold bright circle framing the aperture.
const RIM: char = '@';
/// Bloom and cloud hash layers.
const L_BLOOM: u64 = 0x25;
const L_CLOUD: u64 = 0x26;
/// Meihua petal marks, alternating weight around the junction.
const PETAL: [char; 2] = ['*', '·'];
/// Xiangyun band glyphs, curl row to base row.
const CLOUD: [char; 3] = ['⌒', '~', '≈'];

const PARALLEL_MIN_CELLS: usize = 20_480;
/// Terminal cells are about twice as tall as wide.
const ASPECT: f32 = 2.0;
const MARGIN: f32 = 3.0;
/// A clipped chord shorter than this leaves the wall instead of stubble.
const MIN_SEG: f32 = 0.9;
const MAX_SPLITS: usize = 400;

const L_WALL: u64 = 0x21;
const L_ICE: u64 = 0x22;
const L_GATE: u64 = 0x23;
const L_LATTICE: u64 = 0x24;

const PARAMS: &[Param] = &[
    param!("GATE", "moon gate radius", 0.0, 0.9, 0.55, 0.01),
    param!("CRACK", "ice-ray split depth", 1.0, 8.0, 6.0, 1.0),
    param!("FRET", "hui-wen fret density", 0.0, 1.0, 0.5, 0.05),
    param!("ROOT", "crack root bias", 0.0, 1.0, 0.4, 0.05),
    param!("GLOW", "lantern wave speed", 0.0, 2.0, 0.6, 0.05),
    param!("BLOOM", "meihua bloom density", 0.0, 1.0, 0.5, 0.05),
    param!("DRIFT", "cloud drift", 0.0, 2.0, 0.5, 0.05),
];

impl Mode for Moongate {
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

/// Splitmix64 over (seed, layer, index, slot); no rng stream is consumed.
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

/// One frame's resolved geometry and palette.
struct Look {
    seed: u64,
    w: f32,
    h: f32,
    time: f32,
    gate_r: f32,
    crack: u32,
    fret: f32,
    root: f32,
    glow: f32,
    bloom: f32,
    drift: f32,
    u0: f32,
    u1: f32,
    v0: f32,
    v1: f32,
    wall: Color,
    wall_fleck: Color,
    strap_dim: Color,
    strap_lit: Color,
    rim_dim: Color,
    rim_lit: Color,
    gate_bg: Color,
    gate_dust: Color,
    fret_color: Color,
    petal_dim: Color,
    petal_lit: Color,
    cloud_dim: Color,
    cloud_lit: Color,
}

impl Look {
    fn new(seed: u64, w: usize, h: usize, palette: &[Color; 5], time: f32, p: &[f32; KNOBS]) -> Self {
        let (wf, hf) = (w as f32, h as f32);
        let u0 = MARGIN;
        let u1 = (wf / ASPECT - MARGIN).max(u0 + 2.0);
        let v0 = MARGIN;
        let v1 = (hf - MARGIN).max(v0 + 2.0);
        let span = (u1 - u0).min(v1 - v0);
        Look {
            seed,
            w: wf,
            h: hf,
            time,
            gate_r: p[0] * 0.5 * span,
            crack: p[1].round().clamp(1.0, 8.0) as u32,
            fret: p[2],
            root: p[3],
            glow: p[4],
            bloom: p[5],
            drift: p[6],
            u0,
            u1,
            v0,
            v1,
            wall: darken(palette[0], 8),
            wall_fleck: darken(palette[1], 30),
            strap_dim: darken(palette[2], 30),
            strap_lit: lighten(palette[4], 10),
            rim_dim: darken(palette[3], 10),
            rim_lit: lighten(palette[3], 25),
            gate_bg: darken(palette[0], 45),
            gate_dust: darken(palette[1], 45),
            fret_color: lerp_color(palette[1], palette[3], 0.35),
            petal_dim: darken(palette[3], 20),
            petal_lit: lighten(palette[4], 15),
            cloud_dim: darken(palette[2], 35),
            cloud_lit: lighten(palette[2], 8),
        }
    }

    /// Gate centre in lattice space.
    fn cu(&self) -> f32 {
        (self.u0 + self.u1) * 0.5
    }
    fn cv(&self) -> f32 {
        (self.v0 + self.v1) * 0.5
    }
}

/// One derived ice-ray chord with its place in the derivation order.
struct Seg {
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    order: u32,
}

/// Convex polygon in lattice space, vertices in boundary order.
struct Poly {
    x: Vec<f32>,
    y: Vec<f32>,
}

impl Poly {
    fn rect(u0: f32, v0: f32, u1: f32, v1: f32) -> Self {
        Poly {
            x: vec![u0, u1, u1, u0],
            y: vec![v0, v0, v1, v1],
        }
    }
    fn area(&self) -> f32 {
        let n = self.x.len();
        let mut a = 0.0;
        for i in 0..n {
            let j = (i + 1) % n;
            a += self.x[i] * self.y[j] - self.x[j] * self.y[i];
        }
        (a * 0.5).abs()
    }
    fn perimeter(&self) -> f32 {
        let n = self.x.len();
        let mut p = 0.0;
        for i in 0..n {
            let j = (i + 1) % n;
            let dx = self.x[j] - self.x[i];
            let dy = self.y[j] - self.y[i];
            p += (dx * dx + dy * dy).sqrt();
        }
        p
    }
}

#[inline]
fn lerp2(a: (f32, f32), b: (f32, f32), t: f32) -> (f32, f32) {
    (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)
}

/// Stiny's ice-ray grammar: join two boundary points of one convex region with
/// a chord, keep both halves, recurse until the depth or size budget runs out.
fn derive_lattice(segs: &mut Vec<Seg>, look: &Look) {
    segs.clear();
    if look.u1 - look.u0 < 2.0 || look.v1 - look.v0 < 2.0 {
        return;
    }
    let mut stack: Vec<(Poly, u32)> = vec![(
        Poly::rect(look.u0, look.v0, look.u1, look.v1),
        look.crack,
    )];
    let root_corner = (look.u0, look.v1);
    let span_u = look.u1 - look.u0;
    let span_v = look.v1 - look.v0;
    let diag = (span_u * span_u + span_v * span_v).sqrt();
    let mut order: u32 = 0;
    while let Some((poly, depth)) = stack.pop() {
        if depth == 0
            || order >= MAX_SPLITS as u32
            || poly.area() < 1.8
            || poly.perimeter() < 5.0
        {
            continue;
        }
        let Some((p0, p1)) = choose_chord(&poly, look, order, root_corner, diag) else {
            continue;
        };
        let Some((left, right)) = split_poly(&poly, p0, p1) else {
            continue;
        };
        if let Some((a, b)) = clip_out_gate(p0, p1, look) {
            let dx = b.0 - a.0;
            let dy = b.1 - a.1;
            if (dx * dx + dy * dy).sqrt() >= MIN_SEG {
                segs.push(Seg {
                    x0: a.0,
                    y0: a.1,
                    x1: b.0,
                    y1: b.1,
                    order,
                });
                order += 1;
            }
        }
        stack.push((left, depth - 1));
        stack.push((right, depth - 1));
    }
}

/// Split a convex polygon by the chord p0-p1 into its two halves.
fn split_poly(poly: &Poly, p0: (f32, f32), p1: (f32, f32)) -> Option<(Poly, Poly)> {
    let n = poly.x.len();
    let (mut lx, mut ly): (Vec<f32>, Vec<f32>) = (Vec::new(), Vec::new());
    let (mut rx, mut ry): (Vec<f32>, Vec<f32>) = (Vec::new(), Vec::new());
    let ex = p1.0 - p0.0;
    let ey = p1.1 - p0.1;
    for i in 0..n {
        let j = (i + 1) % n;
        let (ax, ay) = (poly.x[i], poly.y[i]);
        let (bx, by) = (poly.x[j], poly.y[j]);
        let sa = ex * (ay - p0.1) - ey * (ax - p0.0);
        let sb = ex * (by - p0.1) - ey * (bx - p0.0);
        if sa >= 0.0 {
            lx.push(ax);
            ly.push(ay);
        }
        if sa <= 0.0 {
            rx.push(ax);
            ry.push(ay);
        }
        if (sa > 0.0 && sb < 0.0) || (sa < 0.0 && sb > 0.0) {
            let ix = ax + (bx - ax) * (sa / (sa - sb));
            let iy = ay + (by - ay) * (sa / (sa - sb));
            lx.push(ix);
            ly.push(iy);
            rx.push(ix);
            ry.push(iy);
        }
    }
    if lx.len() < 3 || rx.len() < 3 {
        return None;
    }
    Some((Poly { x: lx, y: ly }, Poly { x: rx, y: ry }))
}

/// Two boundary points of the polygon, hash-chosen, with ROOT biasing the
/// first point toward the lower left corner so cracks appear to radiate.
fn choose_chord(
    poly: &Poly,
    look: &Look,
    salt: u32,
    root: (f32, f32),
    diag: f32,
) -> Option<((f32, f32), (f32, f32))> {
    let n = poly.x.len();
    let mut edges: Vec<(usize, f32, f32)> = Vec::with_capacity(n);
    let mut total = 0.0;
    for i in 0..n {
        let j = (i + 1) % n;
        let dx = poly.x[j] - poly.x[i];
        let dy = poly.y[j] - poly.y[i];
        let len = (dx * dx + dy * dy).sqrt();
        if len < 1e-4 {
            continue;
        }
        let mx = (poly.x[i] + poly.x[j]) * 0.5 - root.0;
        let my = (poly.y[i] + poly.y[j]) * 0.5 - root.1;
        let near = 1.0 - ((mx * mx + my * my).sqrt() / diag).clamp(0.0, 1.0);
        let weight = len * (1.0 + look.root * 3.0 * near);
        total += weight;
        edges.push((i, len, weight));
    }
    if edges.is_empty() || total <= 0.0 {
        return None;
    }
    let pick = |slot: u64, skip: usize| -> Option<(usize, f32)> {
        let candidates: Vec<(usize, f32, f32)> = edges
            .iter()
            .copied()
            .filter(|(i, _, _)| *i != skip)
            .collect();
        let rest: f32 = candidates.iter().map(|(_, _, w)| *w).sum();
        if rest <= 0.0 {
            return None;
        }
        let mut roll = unit(hash(look.seed, L_ICE, salt as u64, slot)) * rest;
        for &(i, len, weight) in &candidates {
            roll -= weight;
            if roll <= 0.0 {
                return Some((i, len));
            }
        }
        let last = candidates[candidates.len() - 1];
        Some((last.0, last.1))
    };
    let Some((e0, l0)) = pick(1, usize::MAX) else {
        return None;
    };
    let Some((e1, l1)) = pick(2, e0) else {
        return None;
    };
    // the first point hugs the root corner, the second lands anywhere, so
    // chords radiate outward instead of collapsing on the edge starts
    let t0 = unit(hash(look.seed, L_ICE, (salt as u64) << 8 | 0x51, 1))
        .powf(1.0 + look.root * 1.5);
    let p0 = point_on_edge(poly, e0, t0 * l0);
    for attempt in 0..4u64 {
        let t1 = unit(hash(look.seed, L_ICE, (salt as u64) << 8 | 0xad, 2 + attempt));
        let p1 = point_on_edge(poly, e1, t1 * l1);
        let dx = p1.0 - p0.0;
        let dy = p1.1 - p0.1;
        if (dx * dx + dy * dy).sqrt() > 1.4 {
            return Some((p0, p1));
        }
    }
    None
}

/// A point partway along edge i of the polygon.
#[inline]
fn point_on_edge(poly: &Poly, i: usize, dist: f32) -> (f32, f32) {
    let j = (i + 1) % poly.x.len();
    let (ax, ay) = (poly.x[i], poly.y[i]);
    let (bx, by) = (poly.x[j], poly.y[j]);
    let len = ((bx - ax) * (bx - ax) + (by - ay) * (by - ay)).sqrt().max(1e-4);
    lerp2((ax, ay), (bx, by), (dist / len).clamp(0.0, 1.0))
}

/// Trim a chord so no part of it survives inside the moon gate disc.
fn clip_out_gate(a: (f32, f32), b: (f32, f32), look: &Look) -> Option<((f32, f32), (f32, f32))> {
    // clear the rim band too, so chords stop just outside the ring
    let r = look.gate_r + 1.6;
    if look.gate_r < 0.6 {
        return Some((a, b));
    }
    let cu = look.cu();
    let cv = look.cv();
    let dx = b.0 - a.0;
    let dy = b.1 - a.1;
    let fx = a.0 - cu;
    let fy = a.1 - cv;
    let qa = dx * dx + dy * dy;
    if qa < 1e-6 {
        return None;
    }
    let qb = 2.0 * (fx * dx + fy * dy);
    let qc = fx * fx + fy * fy - r * r;
    let ain = qc < 0.0;
    let bin = (b.0 - cu) * (b.0 - cu) + (b.1 - cv) * (b.1 - cv) < r * r;
    if ain && bin {
        return None;
    }
    let disc = qb * qb - 4.0 * qa * qc;
    if !ain && !bin {
        if disc <= 0.0 {
            return Some((a, b));
        }
        let sq = disc.sqrt();
        let t1 = (-qb - sq) / (2.0 * qa);
        let t2 = (-qb + sq) / (2.0 * qa);
        if t1 > 0.0 && t2 < 1.0 {
            // the chord dips through the disc: keep the longer outer piece
            if t1 > 1.0 - t2 {
                Some((a, lerp2(a, b, t1)))
            } else {
                Some((lerp2(a, b, t2), b))
            }
        } else {
            Some((a, b))
        }
    } else {
        // one end inside: keep the outside run from the crossing point
        let sq = disc.max(0.0).sqrt();
        let t_exit = if ain {
            (-qb + sq) / (2.0 * qa)
        } else {
            (-qb - sq) / (2.0 * qa)
        };
        let t = t_exit.clamp(0.0, 1.0);
        if ain {
            Some((lerp2(a, b, t), b))
        } else {
            Some((a, lerp2(a, b, t)))
        }
    }
}

thread_local! {
    static MASK: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    static ORD: RefCell<Vec<f32>> = const { RefCell::new(Vec::new()) };
}

fn draw(frame: &mut ModeFrame<'_>, p: &[f32; KNOBS]) {
    let (w, h) = (frame.width, frame.height);
    if w == 0 || h == 0 {
        return;
    }
    let look = Look::new(frame.seed, w, h, frame.palette, frame.time, p);
    measure_layer(NAME, "wall", || paint_wall(frame.grid, w, h, &look));
    let mut segs: Vec<Seg> = Vec::new();
    measure_layer(NAME, "derive", || derive_lattice(&mut segs, &look));
    MASK.with(|mslot| {
        ORD.with(|oslot| {
            let mut masks = mslot.borrow_mut();
            let mut ords = oslot.borrow_mut();
            if masks.len() < w * h {
                masks.resize(w * h, 0);
            }
            if ords.len() < w * h {
                ords.resize(w * h, 0.0);
            }
            let (masks, ords) = (masks.split_at_mut(w * h).0, ords.split_at_mut(w * h).0);
            masks.fill(0);
            ords.fill(0.0);
            measure_layer(NAME, "lattice", || {
                paint_lattice(frame.grid, &segs, masks, ords, w, h, &look)
            });
            // the ring wins collisions, so it paints after the lattice
            measure_layer(NAME, "gate", || paint_gate(frame.grid, w, h, &look));
            measure_layer(NAME, "fret", || paint_fret(frame.grid, w, h, &look));
            measure_layer(NAME, "bloom", || {
                paint_blooms(frame.grid, masks, ords, w, h, &look)
            });
            measure_layer(NAME, "clouds", || paint_clouds(frame.grid, w, h, &look));
        })
    });
}

/// Bare wall first: dark masonry flecks so everything above reads on space.
fn paint_wall(grid: &mut Grid, w: usize, h: usize, look: &Look) {
    let rows = grid.len().min(h);
    let slice = &mut grid[..rows];
    let paint = |(y, row): (usize, &mut Vec<Cell>)| {
        for (x, cell) in row.iter_mut().enumerate().take(w) {
            let k = unit(hash(look.seed, L_WALL, x as u64, y as u64));
            *cell = if k < 0.085 {
                Cell::new(WALL[(k * 47.0) as usize % WALL.len()], look.wall_fleck)
            } else {
                Cell::blank()
            };
        }
    };
    if w * h >= PARALLEL_MIN_CELLS {
        slice.par_iter_mut().enumerate().for_each(paint);
    } else {
        slice.iter_mut().enumerate().for_each(paint);
    }
}

/// The moon gate: a washed interior with sparse dust, ringed by a bright rim
/// whose sheen turns with the lantern wave.
fn paint_gate(grid: &mut Grid, w: usize, h: usize, look: &Look) {
    if look.gate_r < 1.0 {
        return;
    }
    let cu = look.cu() * ASPECT;
    let cv = look.cv();
    let rows = grid.len().min(h);
    let slice = &mut grid[..rows];
    let paint = |(y, row): (usize, &mut Vec<Cell>)| {
        let v = y as f32 + 0.5 - cv;
        for (x, cell) in row.iter_mut().enumerate().take(w) {
            let u = (x as f32 + 0.5 - cu) / ASPECT;
            let d = (u * u + v * v).sqrt();
            if d < look.gate_r - 0.2 {
                *cell = if unit(hash(look.seed, L_GATE, x as u64, y as u64)) < 0.05 {
                    Cell::new(DUST, look.gate_dust)
                } else {
                    Cell::with_bg(' ', look.gate_bg, look.gate_bg)
                };
            } else if d < look.gate_r + 0.8 {
                let a = v.atan2(u);
                let wave = 0.5 + 0.5 * (a * 6.0 - look.time * look.glow * 1.2).sin();
                let col = lerp_color(look.rim_dim, look.rim_lit, 0.35 + 0.65 * wave);
                *cell = Cell::new(RIM, col);
            }
        }
    };
    if w * h >= PARALLEL_MIN_CELLS {
        slice.par_iter_mut().enumerate().for_each(paint);
    } else {
        slice.iter_mut().enumerate().for_each(paint);
    }
}

/// Direction class of a strap in screen space.
#[inline]
fn classify(dxg: f32, dyg: f32) -> u8 {
    let (adx, ady) = (dxg.abs(), dyg.abs());
    if adx > ady * 2.4 {
        C_H
    } else if ady > adx * 2.4 {
        C_V
    } else if dxg * dyg > 0.0 {
        C_B
    } else {
        C_F
    }
}

/// Joint glyph for a lattice cell touched by one or more strap directions.
#[inline]
fn glyph_for_mask(m: u8) -> char {
    match m {
        C_H => '─',
        C_V => '│',
        C_F => '╱',
        C_B => '╲',
        M_HV | M_FB => '┼',
        _ => '+',
    }
}

/// Raster the derived chords into the mask, then paint each touched cell with
/// a joint glyph and a lantern brightness wave that runs along derivation
/// order, so the light traces how the grammar built the wall.
fn paint_lattice(
    grid: &mut Grid,
    segs: &[Seg],
    masks: &mut [u8],
    ords: &mut [f32],
    w: usize,
    h: usize,
    look: &Look,
) {
    if segs.is_empty() {
        return;
    }
    let order_max = segs.iter().map(|s| s.order).max().unwrap_or(1).max(1) as f32;
    for seg in segs {
        let dxg = (seg.x1 - seg.x0) * ASPECT;
        let dyg = seg.y1 - seg.y0;
        let cls = classify(dxg, dyg);
        let len = (dxg * dxg + dyg * dyg).sqrt();
        let steps = (len * 1.7).ceil().max(1.0) as usize;
        let norm = seg.order as f32 / order_max;
        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let u = seg.x0 + (seg.x1 - seg.x0) * t;
            let v = seg.y0 + (seg.y1 - seg.y0) * t;
            let gx = (u * ASPECT - 0.5).round() as i64;
            let gy = (v - 0.5).round() as i64;
            if gx < 0 || gy < 0 || gx as usize >= w || gy as usize >= h {
                continue;
            }
            let idx = gy as usize * w + gx as usize;
            masks[idx] |= cls;
            if norm > ords[idx] {
                ords[idx] = norm;
            }
        }
    }
    let rows = grid.len().min(h);
    let slice = &mut grid[..rows];
    let paint = |(y, row): (usize, &mut Vec<Cell>)| {
        for (x, cell) in row.iter_mut().enumerate().take(w) {
            let m = masks[y * w + x];
            if m == 0 {
                continue;
            }
            let wave = 0.5
                + 0.5 * (ords[y * w + x] * TAU * 2.0 - look.time * look.glow * 1.8).sin();
            let col = lerp_color(look.strap_dim, look.strap_lit, 0.3 + 0.7 * wave);
            *cell = Cell::new(glyph_for_mask(m), col);
        }
    };
    if w * h >= PARALLEL_MIN_CELLS {
        slice.par_iter_mut().enumerate().for_each(paint);
    } else {
        slice.iter_mut().enumerate().for_each(paint);
    }
}

/// Hui-wen frets: a square-wave meander walked along the top and bottom edges,
/// each glyph computed from the walk's turn direction, never stamped.
fn paint_fret(grid: &mut Grid, w: usize, h: usize, look: &Look) {
    if look.fret <= 0.01 || h < 4 {
        return;
    }
    let period = (4.0 + (1.0 - look.fret) * 8.0).round().max(4.0) as usize;
    for band in 0..2u64 {
        let y_top = if band == 0 { 0 } else { h - 2 };
        let base = (hash(look.seed, L_LATTICE, band, 7) % period as u64) as usize;
        // the fret marches with the lantern, mirrored bands circling opposite ways
        let march = (look.time * look.glow * 1.3).floor() as i64 * if band == 0 { 1 } else { -1 };
        let phase = (base as i64 + march).rem_euclid(period as i64) as usize;
        meander_band(grid, w, y_top, period, phase, look);
    }
}

/// One two-row meander. Row 0 is the inner strand, row 1 the outer strand.
fn meander_band(
    grid: &mut Grid,
    w: usize,
    y_top: usize,
    period: usize,
    phase: usize,
    look: &Look,
) {
    if y_top + 1 >= grid.len() {
        return;
    }
    let mut r = 0usize;
    let mut pts: Vec<(usize, usize)> = Vec::new();
    let mut x = 0usize;
    while x < w {
        pts.push((x, r));
        if (x + phase) % period == 0 && x + 1 < w {
            r = 1 - r;
            pts.push((x, r));
        }
        x += 1;
    }
    for i in 0..pts.len() {
        let (px, pr) = pts[i];
        let indir = if i == 0 {
            (1, 0)
        } else {
            (
                pts[i].0 as isize - pts[i - 1].0 as isize,
                pts[i].1 as isize - pts[i - 1].1 as isize,
            )
        };
        let outdir = if i + 1 == pts.len() {
            (1, 0)
        } else {
            (
                pts[i + 1].0 as isize - pts[i].0 as isize,
                pts[i + 1].1 as isize - pts[i].1 as isize,
            )
        };
        let y = y_top + pr;
        if y < grid.len() && px < grid[y].len() {
            grid[y][px] = Cell::new(join_glyph(indir, outdir), look.fret_color);
        }
    }
}

/// Glyph whose arms join the incoming and outgoing step directions.
fn join_glyph(indir: (isize, isize), outdir: (isize, isize)) -> char {
    let (bx, by) = (-indir.0, -indir.1);
    let (fx, fy) = outdir;
    let left = bx < 0 || fx < 0;
    let right = bx > 0 || fx > 0;
    let up = by < 0 || fy < 0;
    let down = by > 0 || fy > 0;
    match (left, right, up, down) {
        (true, true, false, false) => '─',
        (false, false, true, true) => '│',
        (true, false, false, true) => '┐',
        (true, false, true, false) => '┘',
        (false, true, false, true) => '└',
        (false, true, true, false) => '┌',
        _ => '┼',
    }
}

/// Meihua rosettes bloom at lattice joints: the crossing itself becomes the
/// flower, a bright eye with four dim specks, likelier on younger growth.
fn paint_blooms(
    grid: &mut Grid,
    masks: &[u8],
    ords: &[f32],
    w: usize,
    h: usize,
    look: &Look,
) {
    if look.bloom <= 0.01 {
        return;
    }
    let rows = grid.len().min(h);
    for y in 1..rows.saturating_sub(1) {
        for x in 1..w.saturating_sub(1) {
            let idx = y * w + x;
            if masks[idx].count_ones() < 2 {
                continue;
            }
            let pick = unit(hash(look.seed, L_BLOOM, x as u64, y as u64));
            if pick >= look.bloom * (0.3 + 0.7 * ords[idx]) {
                continue;
            }
            let fk = unit(hash(look.seed, L_BLOOM, (x as u64) << 8 | y as u64, 3));
            let fcol = lerp_color(look.petal_dim, look.petal_lit, 0.45 + 0.55 * fk);
            grid[y][x] = Cell::new('*', fcol);
            for d in 0..4u64 {
                let (dx, dy) = match d {
                    0 => (-1i64, -1i64),
                    1 => (1, -1),
                    2 => (-1, 1),
                    _ => (1, 1),
                };
                let (nx, ny) = (x as i64 + dx, y as i64 + dy);
                if nx < 1 || ny < 1 || nx as usize >= w - 1 || ny as usize >= rows - 1 {
                    continue;
                }
                let cell = &mut grid[ny as usize][nx as usize];
                if cell.ch != ' ' && !WALL.contains(&cell.ch) {
                    continue;
                }
                let pk = unit(hash(
                    look.seed,
                    L_BLOOM,
                    ((x as u64) << 16) | ((y as u64) << 4),
                    4 + d,
                ));
                let col = lerp_color(look.petal_dim, look.petal_lit, 0.25 + 0.5 * pk);
                *cell = Cell::new(if d & 1 == 0 { '·' } else { ',' }, col);
            }
        }
    }
}

/// Xiangyun clouds drift through the aperture on the lantern wind.
/// Three undulating bands and a curl each, clipped to the interior circle.
fn paint_clouds(grid: &mut Grid, w: usize, h: usize, look: &Look) {
    if look.gate_r < 2.5 {
        return;
    }
    let count = ((look.gate_r / 2.6).ceil() as usize).clamp(1, 4);
    let cu = look.cu();
    let cv = look.cv();
    let span = look.gate_r * 2.0;
    let gcx = cu * ASPECT - 0.5;
    let gcy = cv - 0.5;
    let rows = grid.len().min(h);
    for c in 0..count as u64 {
        let hsh = hash(look.seed, L_CLOUD, c, 9);
        let base_x = unit(hsh) * span;
        let base_y = (unit(hsh >> 9) * 1.3 - 0.65) * look.gate_r;
        let speed = 0.9 + 0.7 * unit(hsh >> 18);
        let scale = 0.55 + 0.3 * unit(hsh >> 27);
        let phase = unit(hsh >> 33) * TAU;
        let xc = (base_x + look.time * look.drift * speed) % span - look.gate_r;
        let ccx = gcx + xc * ASPECT;
        let ccy = gcy + base_y;
        let pulse = 0.5 + 0.5 * (look.time * look.glow * 1.2 + c as f32 * 2.1).sin();
        for row in 0..3usize {
            let half = ((2.0 + row as f32 * 1.2) * scale * 2.5).round() as i64;
            let yoff = row as f32;
            let xoff = if row == 1 { -1.5 } else { 0.0 };
            for s in -half..=half {
                let gx = ccx + xoff + s as f32;
                let gy = ccy + yoff + (s as f32 * 0.7 + phase).sin() * 0.4;
                let ix = gx.round() as i64;
                let iy = gy.round() as i64;
                if ix < 0 || iy < 0 || ix as usize >= w || iy as usize >= rows {
                    continue;
                }
                let ux = (ix as f32 + 0.5) / ASPECT - cu;
                let vy = iy as f32 + 0.5 - cv;
                if ux * ux + vy * vy > (look.gate_r - 0.45) * (look.gate_r - 0.45) {
                    continue;
                }
                let (ry, rx) = (iy as usize, ix as usize);
                if ry >= grid.len() || rx >= grid[ry].len() {
                    continue;
                }
                let cell = &mut grid[ry][rx];
                if cell.ch != ' ' {
                    continue;
                }
                let lit = (0.4 + 0.4 * pulse - row as f32 * 0.1).clamp(0.05, 0.95);
                *cell = Cell::new(CLOUD[row], lerp_color(look.cloud_dim, look.cloud_lit, lit));
            }
        }
        let hx = (ccx - 2.0 * scale * 2.5).round() as i64;
        let hy = ccy.round() as i64;
        if hx >= 0
            && hy >= 0
            && (hx as usize) < w
            && (hy as usize) < rows
            && (hx as usize) < grid[hy as usize].len()
        {
            let ux = (hx as f32 + 0.5) / ASPECT - cu;
            let vy = hy as f32 + 0.5 - cv;
            let cell = &mut grid[hy as usize][hx as usize];
            if ux * ux + vy * vy <= (look.gate_r - 0.45) * (look.gate_r - 0.45) && cell.ch == ' ' {
                *cell = Cell::new('o', lerp_color(look.cloud_dim, look.cloud_lit, 0.85));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::grid_to_plain;
    use rand::{rngs::StdRng, SeedableRng};

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

    #[test]
    fn moongate_seed42() {
        insta::assert_snapshot!("moongate_80x24", text(&frame(80, 24, 42, 0.0, &knobs())));
    }

    #[test]
    fn moongate_seed42_t4() {
        insta::assert_snapshot!("moongate_80x24_t4", text(&frame(80, 24, 42, 4.0, &knobs())));
    }

    #[test]
    fn deterministic_and_seed_sensitive() {
        let k = knobs();
        assert_eq!(text(&frame(90, 30, 42, 0.0, &k)), text(&frame(90, 30, 42, 0.0, &k)));
        assert_ne!(text(&frame(90, 30, 42, 0.0, &k)), text(&frame(90, 30, 7, 0.0, &k)));
    }

    #[test]
    fn time_moves_the_lantern() {
        let k = knobs();
        assert_ne!(text(&frame(90, 30, 42, 0.0, &k)), text(&frame(90, 30, 42, 4.0, &k)));
    }

    #[test]
    fn crack_changes_the_figure() {
        let mut k = knobs();
        let a = text(&frame(90, 30, 42, 0.0, &k));
        k[1] = 7.0;
        assert_ne!(a, text(&frame(90, 30, 42, 0.0, &k)));
    }

    #[test]
    fn bloom_changes_the_wall() {
        let mut k = knobs();
        let a = text(&frame(90, 30, 42, 0.0, &k));
        k[5] = 0.0;
        assert_ne!(a, text(&frame(90, 30, 42, 0.0, &k)));
    }

    #[test]
    fn drift_moves_the_clouds() {
        let mut k = knobs();
        let a = text(&frame(90, 30, 42, 4.0, &k));
        k[6] = 0.0;
        assert_ne!(a, text(&frame(90, 30, 42, 4.0, &k)));
    }

    #[test]
    fn moongate_seed42_full_bloom() {
        let mut k = knobs();
        k[5] = 1.0;
        k[6] = 1.5;
        insta::assert_snapshot!("moongate_80x24_bloom", text(&frame(80, 24, 42, 3.0, &k)));
    }

    #[test]
    fn tiny_frame_clips_safely() {
        let g = frame(16, 6, 42, 0.0, &knobs());
        assert_eq!(g.len(), 6);
        assert_eq!(g[0].len(), 16);
    }

    #[test]
    fn frame_cost() {
        let (w, h) = (200usize, 60usize);
        let k = knobs();
        let mut worst = 0.0f64;
        let start = std::time::Instant::now();
        for f in 0..100 {
            let t0 = std::time::Instant::now();
            frame(w, h, 42, f as f32 * 0.05, &k);
            worst = worst.max(t0.elapsed().as_secs_f64() * 1000.0);
        }
        let avg = start.elapsed().as_secs_f64() * 1000.0 / 100.0;
        eprintln!("moongate frame_cost 200x60: avg {:.3} ms, worst {:.3} ms", avg, worst);
        if !cfg!(debug_assertions) {
            assert!(avg < 6.0, "avg frame {:.3} ms", avg);
        }
    }
}
