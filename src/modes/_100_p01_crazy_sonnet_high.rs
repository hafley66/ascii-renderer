//! synaptic-bloom: dendritic growth coupled to its own density field. Design: `plans/1_synaptic_bloom.md`.
//! Author: Claude Sonnet 5. Written: 2026-09-19.

use crate::_0_profile::measure_layer;
use crate::color::{darken, hsl_to_rgb, lerp_color, lighten};
use crate::opts::param_f32;
use crate::registry::{AnimKind, Mode, ModeFrame, Param};
use crate::types::{Cell, Grid};
use crossterm::style::Color;

pub(super) struct SynapticBloom;
pub(super) static MODE: SynapticBloom = SynapticBloom;

const NAME: &str = "synaptic-bloom";
const KNOBS: usize = 13;
const HELP: &str = "synaptic-bloom: a dendritic ganglion grown toward an attractant field, self-avoiding through its own density [somas] [attract] [perceive] [kill] [step] [repel] [split] [tropism] [depth] [spin] [pulse] [hue] [aspect]";

const DIR: [char; 4] = ['-', '\\', '|', '/'];
const THICK: [char; 4] = ['o', 'O', '#', '\u{2588}'];
const MOTE: [char; 2] = ['.', '\''];
const SOMA_CORE: char = '@';
const SOMA_RING: char = 'o';
const PULSE_CORE: char = '*';
const PULSE_TRAIL: char = 'o';

const L_SOMA: u64 = 0x41;
const L_ATTR: u64 = 0x42;
const L_GROW: u64 = 0x43;
const L_PATH: u64 = 0x44;

const MAX_NODES: usize = 420;
const NODE_REF_AREA: f32 = 80.0 * 24.0;
const MAX_ITERS: usize = 260;
const MISS_LIMIT: u32 = 5;
const MIN_SEGMENT: u32 = 4;
const PULSES_PER_SOMA: usize = 3;
const PULSE_GAP_FRAC: f32 = 0.5;
const BREATH_PERIOD: f32 = 18.0;
const BREATH_DEPTH: f32 = 0.24;

/// 1.0 at `t == 0` (default render fully grown); dips to `1 - BREATH_DEPTH`.
#[inline]
fn breathing_frac(t: f32) -> f32 {
    let base = 1.0 - BREATH_DEPTH * 0.5;
    base + BREATH_DEPTH * 0.5 * (t * std::f32::consts::TAU / BREATH_PERIOD).cos()
}
const FIELD_W: usize = 40;
const FIELD_H: usize = 20;
const SHEAR_GAIN: f32 = 3.2;

const PARAMS: &[Param] = &[
    param!("SOMAS", "ganglion roots", 1.0, 4.0, 2.0, 1.0),
    param!("ATTRACT", "attractant motes", 60.0, 380.0, 220.0, 10.0),
    param!("PERCEIVE", "tip perception radius", 3.0, 14.0, 7.0, 0.5),
    param!("KILL", "attractant kill radius", 0.6, 4.0, 1.6, 0.1),
    param!("STEP", "growth step length", 0.4, 2.0, 0.9, 0.1),
    param!("REPEL", "self-density repulsion", 0.0, 3.0, 1.1, 0.1),
    param!("SPLIT", "branch split sensitivity", 0.3, 2.2, 1.1, 0.1),
    param!("TROPISM", "vertical growth bias", -1.0, 1.0, 0.5, 0.1),
    param!("DEPTH", "z spread / parallax", 0.0, 1.5, 0.7, 0.1),
    param!("SPIN", "camera orbit rad/s", 0.0, 1.0, 0.18, 0.02),
    param!("PULSE", "impulse speed", 2.0, 30.0, 10.0, 1.0),
    param!("HUE", "base hue degrees", 0.0, 360.0, 200.0, 5.0),
    param!("ASPECT", "cols per row", 0.25, 4.0, 2.0, 0.25),
];

impl Mode for SynapticBloom {
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

#[inline]
fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    if e1 <= e0 {
        return if x < e0 { 0.0 } else { 1.0 };
    }
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[derive(Clone, Copy)]
struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

impl Vec3 {
    const ZERO: Vec3 = Vec3 { x: 0.0, y: 0.0, z: 0.0 };

    #[inline]
    fn add(self, o: Vec3) -> Vec3 {
        Vec3 { x: self.x + o.x, y: self.y + o.y, z: self.z + o.z }
    }
    #[inline]
    fn sub(self, o: Vec3) -> Vec3 {
        Vec3 { x: self.x - o.x, y: self.y - o.y, z: self.z - o.z }
    }
    #[inline]
    fn scale(self, s: f32) -> Vec3 {
        Vec3 { x: self.x * s, y: self.y * s, z: self.z * s }
    }
    #[inline]
    fn len(self) -> f32 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }
    #[inline]
    fn dist(self, o: Vec3) -> f32 {
        self.sub(o).len()
    }
    #[inline]
    fn norm(self) -> Vec3 {
        let l = self.len();
        if l < 1e-5 { Vec3 { x: 0.0, y: -1.0, z: 0.0 } } else { self.scale(1.0 / l) }
    }
}

/// One dendrite node. `parent` indexes an earlier node (roots use -1).
/// `dlen` is the arc length from this node's root, used for growth reveal.
struct Node {
    pos: Vec3,
    parent: i32,
    dlen: f32,
    thickness: f32,
}

/// A precomputed root-to-leaf traversal an impulse rides. `cum` is arc length
/// up to each node in `nodes`; `phase` staggers pulses across paths.
struct PulsePath {
    nodes: Vec<usize>,
    cum: Vec<f32>,
    total: f32,
    phase: f32,
}

struct TipState {
    pos: Vec3,
    dir: Vec3,
    node_idx: usize,
    miss: u32,
    since_branch: u32,
}

/// The grown ganglion: nodes, the attractant cloud, the density field the
/// growth wrote into and read back from, and the pulse paths.
struct Ganglion {
    fw: f32,
    fh: f32,
    depth: f32,
    nodes: Vec<Node>,
    max_dlen: f32,
    attractors: Vec<Vec3>,
    consumed: Vec<bool>,
    paths: Vec<PulsePath>,
}

#[inline]
fn field_index(pos: Vec3, fw: f32, fh: f32, depth: f32) -> usize {
    let u = (pos.x / fw.max(1e-3)).clamp(0.0, 0.999_9);
    let v = (pos.y / fh.max(1e-3)).clamp(0.0, 0.999_9);
    let fx = (u * FIELD_W as f32) as usize;
    let fy = (v * FIELD_H as f32) as usize;
    let _ = depth;
    fy.min(FIELD_H - 1) * FIELD_W + fx.min(FIELD_W - 1)
}

fn deposit(field: &mut [f32], pos: Vec3, fw: f32, fh: f32, depth: f32) {
    let idx = field_index(pos, fw, fh, depth);
    let (cx, cy) = (idx % FIELD_W, idx / FIELD_W);
    for (dx, dy, w) in [(0i32, 0i32, 1.0f32), (1, 0, 0.35), (-1, 0, 0.35), (0, 1, 0.35), (0, -1, 0.35)] {
        let nx = cx as i32 + dx;
        let ny = cy as i32 + dy;
        if nx >= 0 && ny >= 0 && (nx as usize) < FIELD_W && (ny as usize) < FIELD_H {
            field[ny as usize * FIELD_W + nx as usize] += w;
        }
    }
}

fn blur_and_decay(field: &mut [f32]) {
    let mut out = field.to_vec();
    for y in 0..FIELD_H {
        for x in 0..FIELD_W {
            let mut sum = 0.0f32;
            let mut n = 0.0f32;
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    let nx = x as i32 + dx;
                    let ny = y as i32 + dy;
                    if nx >= 0 && ny >= 0 && (nx as usize) < FIELD_W && (ny as usize) < FIELD_H {
                        sum += field[ny as usize * FIELD_W + nx as usize];
                        n += 1.0;
                    }
                }
            }
            out[y * FIELD_W + x] = (sum / n.max(1.0)) * 0.985;
        }
    }
    field.copy_from_slice(&out);
}

/// Gradient of the density field at `pos`, pointing toward higher density.
fn field_gradient(field: &[f32], pos: Vec3, fw: f32, fh: f32) -> Vec3 {
    let idx = field_index(pos, fw, fh, 0.0);
    let (cx, cy) = (idx % FIELD_W, idx / FIELD_W);
    let at = |x: i32, y: i32| -> f32 {
        if x < 0 || y < 0 || x as usize >= FIELD_W || y as usize >= FIELD_H {
            0.0
        } else {
            field[y as usize * FIELD_W + x as usize]
        }
    };
    let gx = at(cx as i32 + 1, cy as i32) - at(cx as i32 - 1, cy as i32);
    let gy = at(cx as i32, cy as i32 + 1) - at(cx as i32, cy as i32 - 1);
    Vec3 { x: gx, y: gy, z: 0.0 }
}

impl Ganglion {
    fn build(seed: u64, w: usize, h: usize, p: &[f32; KNOBS]) -> Ganglion {
        let aspect = p[12].max(0.25);
        let (fw, fh) = ((w as f32 / aspect).max(4.0), (h as f32).max(4.0));
        let depth = p[8];
        let somas = p[0].round().max(1.0) as usize;
        let area_ratio = ((w * h) as f32 / NODE_REF_AREA).sqrt().clamp(0.3, 2.5);
        let attract_n = (p[1].round().max(1.0) * area_ratio).clamp(30.0, 900.0) as usize;
        let max_nodes = ((MAX_NODES as f32) * area_ratio).clamp(120.0, 900.0) as usize;
        let perceive = p[2];
        let kill = p[3];
        let step = p[4];
        let repel = p[5];
        let split = p[6];
        let tropism = p[7];

        let mut attractors = Vec::with_capacity(attract_n);
        for i in 0..attract_n as u64 {
            let x = 0.06 * fw + 0.88 * fw * unit(hash(seed, L_ATTR, i, 0));
            let y = 0.06 * fh + 0.88 * fh * unit(hash(seed, L_ATTR, i, 1));
            let z = (unit(hash(seed, L_ATTR, i, 2)) * 2.0 - 1.0) * depth;
            attractors.push(Vec3 { x, y, z });
        }
        let mut consumed = vec![false; attractors.len()];

        let mut nodes: Vec<Node> = Vec::with_capacity(max_nodes);
        let mut tips: Vec<TipState> = Vec::with_capacity(somas * 2);
        for i in 0..somas as u64 {
            let swing = std::f32::consts::TAU * (i as f32 + 0.5) / somas as f32
                + unit(hash(seed, L_SOMA, i, 0)) * 0.6;
            let reach = if somas == 1 { 0.0 } else { 0.22 };
            let cx = fw * 0.5 + swing.cos() * fw * reach;
            let cy = fh * (0.72 - tropism.max(0.0) * 0.18) + swing.sin() * fh * 0.08;
            let cz = (unit(hash(seed, L_SOMA, i, 1)) * 2.0 - 1.0) * depth * 0.3;
            let pos = Vec3 { x: cx, y: cy, z: cz };
            let dir = Vec3 { x: swing.cos() * 0.3, y: -1.0, z: 0.0 }.norm();
            nodes.push(Node { pos, parent: -1, dlen: 0.0, thickness: 0.0 });
            tips.push(TipState { pos, dir, node_idx: i as usize, miss: 0, since_branch: 0 });
        }

        let mut field = vec![0.0f32; FIELD_W * FIELD_H];
        let tropism_vec = Vec3 { x: 0.0, y: -tropism, z: 0.0 };

        let mut iter = 0usize;
        while iter < MAX_ITERS && !tips.is_empty() && nodes.len() < max_nodes {
            iter += 1;
            let mut next_tips: Vec<TipState> = Vec::with_capacity(tips.len() + 2);
            for tip in tips.iter() {
                if nodes.len() >= max_nodes {
                    break;
                }
                let (mut mean, mut mean_w, mut max_angle) = (Vec3::ZERO, 0.0f32, 0.0f32);
                let mut count = 0usize;
                let (mut lmean, mut lw, mut rmean, mut rw) = (Vec3::ZERO, 0.0f32, Vec3::ZERO, 0.0f32);
                for (ai, a) in attractors.iter().enumerate() {
                    if consumed[ai] {
                        continue;
                    }
                    let d = tip.pos.dist(*a);
                    if d > perceive || d < 1e-4 {
                        continue;
                    }
                    let u = a.sub(tip.pos).scale(1.0 / d);
                    let w = 1.0 / (0.3 + d);
                    mean = mean.add(u.scale(w));
                    mean_w += w;
                    count += 1;
                    let side = u.x * tip.dir.y - u.y * tip.dir.x;
                    if side >= 0.0 {
                        lmean = lmean.add(u.scale(w));
                        lw += w;
                    } else {
                        rmean = rmean.add(u.scale(w));
                        rw += w;
                    }
                }
                if count == 0 {
                    if tip.miss >= MISS_LIMIT {
                        continue;
                    }
                    let dir = tip.dir.add(tropism_vec.scale(0.25)).norm();
                    let pos = tip.pos.add(dir.scale(step * 0.7));
                    next_tips.push(TipState {
                        pos,
                        dir,
                        node_idx: tip.node_idx,
                        miss: tip.miss + 1,
                        since_branch: tip.since_branch + 1,
                    });
                    continue;
                }
                let mean_dir = mean.scale(1.0 / mean_w.max(1e-5)).norm();
                for (ai, a) in attractors.iter().enumerate() {
                    if consumed[ai] {
                        continue;
                    }
                    let d = tip.pos.dist(*a);
                    if d > perceive || d < 1e-4 {
                        continue;
                    }
                    let u = a.sub(tip.pos).scale(1.0 / d);
                    let cos_a = (u.x * mean_dir.x + u.y * mean_dir.y + u.z * mean_dir.z).clamp(-1.0, 1.0);
                    max_angle = max_angle.max(cos_a.acos());
                }
                let grad = field_gradient(&field, tip.pos, fw, fh);
                let repulse = grad.scale(repel * 0.6);

                let split_p = smoothstep(split * 1.1, split * 1.9, max_angle);
                let roll = unit(hash(seed, L_GROW, tip.node_idx as u64, iter as u64));
                let ripe = tip.since_branch >= MIN_SEGMENT;
                let balanced = lw.min(rw) > (lw + rw) * 0.3;
                let do_split = ripe && balanced && roll < split_p;

                let mut push_branch = |dir_raw: Vec3, from: &TipState, branched: bool, next_tips: &mut Vec<TipState>| {
                    let mut dir = dir_raw.sub(repulse).add(tropism_vec.scale(0.35));
                    dir = dir.add(from.dir.scale(0.6)).norm();
                    let pos = from.pos.add(dir.scale(step));
                    let pos = Vec3 {
                        x: pos.x.clamp(0.0, fw),
                        y: pos.y.clamp(0.0, fh),
                        z: pos.z.clamp(-depth.max(0.01), depth.max(0.01)),
                    };
                    let subtree_root = from.node_idx;
                    let dlen = nodes[subtree_root].dlen + pos.dist(from.pos);
                    nodes.push(Node { pos, parent: subtree_root as i32, dlen, thickness: 0.0 });
                    let new_idx = nodes.len() - 1;
                    deposit(&mut field, pos, fw, fh, depth);
                    for (aj, aa) in attractors.iter().enumerate() {
                        if !consumed[aj] && pos.dist(*aa) < kill {
                            consumed[aj] = true;
                        }
                    }
                    let since_branch = if branched { 0 } else { from.since_branch + 1 };
                    next_tips.push(TipState { pos, dir, node_idx: new_idx, miss: 0, since_branch });
                };

                if do_split {
                    let ld = lmean.scale(1.0 / lw.max(1e-5)).norm();
                    let rd = rmean.scale(1.0 / rw.max(1e-5)).norm();
                    push_branch(ld, tip, true, &mut next_tips);
                    push_branch(rd, tip, true, &mut next_tips);
                } else {
                    push_branch(mean_dir, tip, false, &mut next_tips);
                }
            }
            tips = next_tips;
            if iter % 8 == 0 {
                blur_and_decay(&mut field);
            }
        }

        let mut has_child = vec![false; nodes.len()];
        for n in &nodes {
            if n.parent >= 0 {
                has_child[n.parent as usize] = true;
            }
        }
        let mut subtree = vec![1u32; nodes.len()];
        for i in (0..nodes.len()).rev() {
            if nodes[i].parent >= 0 {
                let s = subtree[i];
                subtree[nodes[i].parent as usize] += s;
            }
        }
        for i in 0..nodes.len() {
            nodes[i].thickness = ((subtree[i] as f32).sqrt() * 0.5).min(3.2);
        }

        let max_dlen = nodes.iter().fold(0.0f32, |m, n| m.max(n.dlen)).max(1e-3);

        let mut leaves_by_root: Vec<Vec<usize>> = vec![Vec::new(); somas];
        for (i, n) in nodes.iter().enumerate() {
            if has_child[i] || i < somas {
                continue;
            }
            let mut cur = i;
            while nodes[cur].parent >= 0 {
                cur = nodes[cur].parent as usize;
            }
            if cur < somas {
                leaves_by_root[cur].push(i);
            }
        }
        let mut paths = Vec::new();
        for (root, leaves) in leaves_by_root.iter().enumerate() {
            if leaves.is_empty() {
                continue;
            }
            let mut sorted = leaves.clone();
            sorted.sort_by(|a, b| nodes[*b].dlen.partial_cmp(&nodes[*a].dlen).unwrap());
            for pick in 0..PULSES_PER_SOMA.min(sorted.len()) {
                let leaf = sorted[(pick * 7) % sorted.len()];
                let mut chain = vec![leaf];
                let mut cur = leaf;
                while nodes[cur].parent >= 0 {
                    cur = nodes[cur].parent as usize;
                    chain.push(cur);
                }
                chain.reverse();
                let mut cum = vec![0.0f32; chain.len()];
                for i in 1..chain.len() {
                    cum[i] = cum[i - 1] + nodes[chain[i - 1]].pos.dist(nodes[chain[i]].pos);
                }
                let total = *cum.last().unwrap_or(&0.0);
                if total < 1e-3 {
                    continue;
                }
                let phase = unit(hash(seed, L_PATH, root as u64, pick as u64)) * total * (1.0 + PULSE_GAP_FRAC);
                paths.push(PulsePath { nodes: chain, cum, total, phase });
            }
        }

        Ganglion { fw, fh, depth, nodes, max_dlen, attractors, consumed, paths }
    }

    #[inline]
    fn project(&self, pos: Vec3, theta: f32, aspect: f32) -> (f32, f32, f32) {
        let shear = theta.sin() * SHEAR_GAIN;
        let col = (pos.x + pos.z * shear) * aspect;
        let row = pos.y;
        (col, row, pos.z)
    }
}

fn draw(frame: &mut ModeFrame<'_>, p: &[f32; KNOBS]) {
    let (w, h) = (frame.width, frame.height);
    if w == 0 || h == 0 {
        return;
    }
    let ganglion = measure_layer(NAME, "grow", || Ganglion::build(frame.seed, w, h, p));
    let aspect = p[12].max(0.25);
    let theta = frame.time * p[9];
    let growth_frac = breathing_frac(frame.time);
    let frontier = growth_frac * ganglion.max_dlen;
    let hue = p[11] as f64;
    let plate = darken(frame.palette[0], 18);

    let mut depth_buf = vec![f32::MAX; w * h];

    measure_layer(NAME, "field", || paint_field(frame.grid, &ganglion, w, h, aspect, theta, plate));
    measure_layer(NAME, "somas", || {
        paint_somas(frame.grid, &mut depth_buf, &ganglion, w, h, aspect, theta, hue, plate)
    });
    measure_layer(NAME, "edges", || {
        paint_edges(frame.grid, &mut depth_buf, &ganglion, w, h, aspect, theta, frontier, hue, plate)
    });
    measure_layer(NAME, "pulses", || {
        paint_pulses(frame.grid, &depth_buf, &ganglion, w, h, aspect, theta, frame.time, p[10], frontier, hue)
    });
}

fn paint_field(grid: &mut Grid, g: &Ganglion, w: usize, h: usize, aspect: f32, theta: f32, plate: Color) {
    let rows = grid.len().min(h);
    for row in grid[..rows].iter_mut() {
        for cell in row.iter_mut().take(w) {
            *cell = Cell::with_bg(' ', plate, plate);
        }
    }
    for (i, a) in g.attractors.iter().enumerate() {
        if g.consumed[i] {
            continue;
        }
        let (col, row, _) = g.project(*a, theta, aspect);
        let (cx, cy) = (col.round(), row.round());
        if cx < 0.0 || cy < 0.0 {
            continue;
        }
        let (cx, cy) = (cx as usize, cy as usize);
        if cx < w && cy < rows {
            let ch = MOTE[(i & 1) as usize];
            grid[cy][cx] = Cell::with_bg(ch, darken(lighten(plate, 40), 0), plate);
        }
    }
}

fn set_if_nearer(grid: &mut Grid, depth_buf: &mut [f32], w: usize, rows: usize, x: i32, y: i32, z: f32, cell: Cell) {
    if x < 0 || y < 0 {
        return;
    }
    let (x, y) = (x as usize, y as usize);
    if x >= w || y >= rows {
        return;
    }
    let idx = y * w + x;
    if z <= depth_buf[idx] + 0.02 {
        depth_buf[idx] = z.min(depth_buf[idx]);
        grid[y][x] = cell;
    }
}

fn paint_somas(
    grid: &mut Grid,
    depth_buf: &mut [f32],
    g: &Ganglion,
    w: usize,
    h: usize,
    aspect: f32,
    theta: f32,
    hue: f64,
    plate: Color,
) {
    let rows = grid.len().min(h);
    let somas = g.nodes.iter().take_while(|_| true).enumerate().filter(|(_, n)| n.parent < 0);
    for (i, n) in somas {
        let (col, row, z) = g.project(n.pos, theta, aspect);
        let tint = hsl_to_rgb((hue + i as f64 * 25.0).rem_euclid(360.0), 0.75, 0.55);
        let core = lighten(tint, 30);
        let ring = tint;
        for dy in -1i32..=1 {
            for dx in -1i32..=1 {
                let ch = if dx == 0 && dy == 0 { SOMA_CORE } else { SOMA_RING };
                let fg = if dx == 0 && dy == 0 { core } else { ring };
                set_if_nearer(
                    grid,
                    depth_buf,
                    w,
                    rows,
                    col.round() as i32 + dx,
                    row.round() as i32 + dy,
                    z - 0.5,
                    Cell::with_bg(ch, fg, darken(plate, 6)),
                );
            }
        }
    }
}

fn paint_edges(
    grid: &mut Grid,
    depth_buf: &mut [f32],
    g: &Ganglion,
    w: usize,
    h: usize,
    aspect: f32,
    theta: f32,
    frontier: f32,
    hue: f64,
    plate: Color,
) {
    let rows = grid.len().min(h);
    for child in &g.nodes {
        if child.parent < 0 {
            continue;
        }
        let parent = &g.nodes[child.parent as usize];
        let len_a = parent.dlen;
        let len_b = child.dlen;
        let reveal = if len_b > len_a { ((frontier - len_a) / (len_b - len_a)).clamp(0.0, 1.0) } else { 1.0 };
        if reveal <= 0.0 {
            continue;
        }
        let (col_a, row_a, z_a) = g.project(parent.pos, theta, aspect);
        let (col_b, row_b, z_b) = g.project(child.pos, theta, aspect);
        let dist = ((col_b - col_a).powi(2) + (row_b - row_a).powi(2)).sqrt();
        let steps = (dist.round().max(1.0) as usize).min(48);
        let slope = {
            let angle = (row_b - row_a).atan2(col_b - col_a);
            ((angle / (std::f32::consts::PI * 0.25)).round() as i32).rem_euclid(4) as usize
        };
        let thickness = (parent.thickness + child.thickness) * 0.5;
        let depth_norm = (0.5 * (z_a + z_b) / g.depth.max(0.01)).clamp(-1.0, 1.0);
        let tint = hsl_to_rgb((hue as f32 + depth_norm * 40.0).rem_euclid(360.0) as f64, 0.68, (0.42 + 0.12 * (thickness / 3.2)) as f64);
        let fg = if depth_norm > 0.0 { darken(tint, (depth_norm * 40.0) as u8) } else { lighten(tint, (-depth_norm * 25.0) as u8) };
        let last = (steps as f32 * reveal).round().max(0.0) as usize;
        for s in 0..=last.min(steps) {
            let t = s as f32 / steps as f32;
            let col = col_a + (col_b - col_a) * t;
            let row = row_a + (row_b - row_a) * t;
            let z = z_a + (z_b - z_a) * t;
            let ch = if thickness < 1.1 {
                DIR[slope]
            } else {
                let bucket = ((thickness - 1.1) / 2.1 * (THICK.len() as f32 - 1.0)).round() as usize;
                THICK[bucket.min(THICK.len() - 1)]
            };
            set_if_nearer(
                grid,
                depth_buf,
                w,
                rows,
                col.round() as i32,
                row.round() as i32,
                z,
                Cell::with_bg(ch, fg, darken(plate, 2)),
            );
        }
    }
}

fn paint_pulses(
    grid: &mut Grid,
    depth_buf: &[f32],
    g: &Ganglion,
    w: usize,
    h: usize,
    aspect: f32,
    theta: f32,
    time: f32,
    speed: f32,
    frontier: f32,
    hue: f64,
) {
    let rows = grid.len().min(h);
    for path in &g.paths {
        let revealed_len = revealed_length(g, path, frontier);
        if revealed_len < 1e-3 {
            continue;
        }
        let cycle = path.total * (1.0 + PULSE_GAP_FRAC);
        let mut pos_along = (time * speed + path.phase).rem_euclid(cycle);
        if pos_along > revealed_len {
            continue;
        }
        pos_along = pos_along.min(revealed_len);
        let mut seg = 0usize;
        while seg + 1 < path.cum.len() && path.cum[seg + 1] < pos_along {
            seg += 1;
        }
        let seg_end = (seg + 1).min(path.nodes.len() - 1);
        let seg_len = (path.cum[seg_end] - path.cum[seg]).max(1e-4);
        let t = ((pos_along - path.cum[seg]) / seg_len).clamp(0.0, 1.0);
        let a = &g.nodes[path.nodes[seg]].pos;
        let b = &g.nodes[path.nodes[seg_end]].pos;
        let pos = Vec3 { x: a.x + (b.x - a.x) * t, y: a.y + (b.y - a.y) * t, z: a.z + (b.z - a.z) * t };
        let (col, row, z) = g.project(pos, theta, aspect);
        let (cx, cy) = (col.round() as i32, row.round() as i32);
        if cx < 0 || cy < 0 {
            continue;
        }
        let (cx, cy) = (cx as usize, cy as usize);
        if cx >= w || cy >= rows {
            continue;
        }
        let idx = cy * w + cx;
        if z > depth_buf[idx] + 0.4 {
            continue;
        }
        let glow = hsl_to_rgb(hue, 0.85, 0.85);
        grid[cy][cx] = Cell::with_bg(PULSE_CORE, glow, darken(glow, 60));
        for (dx, dy) in [(1i32, 0i32), (-1, 0), (0, 1), (0, -1)] {
            let (nx, ny) = (cx as i32 + dx, cy as i32 + dy);
            if nx >= 0 && ny >= 0 && (nx as usize) < w && (ny as usize) < rows {
                let n_idx = ny as usize * w + nx as usize;
                if z <= depth_buf[n_idx] + 0.4 {
                    let cell = &mut grid[ny as usize][nx as usize];
                    if cell.ch == ' ' {
                        *cell = Cell::with_bg(PULSE_TRAIL, darken(glow, 30), darken(glow, 70));
                    }
                }
            }
        }
    }
}

fn revealed_length(g: &Ganglion, path: &PulsePath, frontier: f32) -> f32 {
    let mut len = 0.0f32;
    for i in 0..path.nodes.len() {
        let dlen = g.nodes[path.nodes[i]].dlen;
        if dlen > frontier {
            if i == 0 {
                return 0.0;
            }
            let dlen_prev = g.nodes[path.nodes[i - 1]].dlen;
            let seg = path.cum[i] - path.cum[i - 1];
            let frac = if dlen > dlen_prev { ((frontier - dlen_prev) / (dlen - dlen_prev)).clamp(0.0, 1.0) } else { 1.0 };
            return path.cum[i - 1] + seg * frac;
        }
        len = path.cum[i];
    }
    len
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

    #[test]
    fn synaptic_bloom_seed42_start() {
        insta::assert_snapshot!("synaptic_bloom_80x24_t0", text(&frame(80, 24, 42, 0.0, &knobs())));
    }

    #[test]
    fn synaptic_bloom_seed42_breath_trough() {
        insta::assert_snapshot!("synaptic_bloom_80x24_t9", text(&frame(80, 24, 42, 9.0, &knobs())));
    }

    #[test]
    fn synaptic_bloom_seed42_later_phase() {
        insta::assert_snapshot!("synaptic_bloom_80x24_t20", text(&frame(80, 24, 42, 20.0, &knobs())));
    }

    #[test]
    fn synaptic_bloom_variant_more_split() {
        let mut k = knobs();
        k[6] = 0.4;
        k[5] = 2.2;
        insta::assert_snapshot!("synaptic_bloom_80x24_variant", text(&frame(80, 24, 42, 20.0, &k)));
    }

    #[test]
    fn synaptic_bloom_small_grid_clips() {
        insta::assert_snapshot!("synaptic_bloom_20x8", text(&frame(20, 8, 42, 5.0, &knobs())));
    }

    #[test]
    fn synaptic_bloom_sweep_perceive_near_a() {
        let mut k = knobs();
        k[2] = 6.5;
        insta::assert_snapshot!("synaptic_bloom_80x24_perceive_6_5", text(&frame(80, 24, 42, 20.0, &k)));
    }

    #[test]
    fn synaptic_bloom_sweep_perceive_near_b() {
        let mut k = knobs();
        k[2] = 7.5;
        insta::assert_snapshot!("synaptic_bloom_80x24_perceive_7_5", text(&frame(80, 24, 42, 20.0, &k)));
    }

    #[test]
    fn deterministic_and_seed_sensitive() {
        let k = knobs();
        assert_eq!(text(&frame(90, 30, 42, 3.0, &k)), text(&frame(90, 30, 42, 3.0, &k)));
        assert_ne!(text(&frame(90, 30, 42, 3.0, &k)), text(&frame(90, 30, 9, 3.0, &k)));
    }

    #[test]
    fn time_zero_is_stable() {
        let k = knobs();
        assert_eq!(text(&frame(90, 30, 42, 0.0, &k)), text(&frame(90, 30, 42, 0.0, &k)));
    }

    #[test]
    fn breathing_frontier_varies_with_time() {
        let k = knobs();
        let full = text(&frame(90, 30, 42, 0.0, &k));
        let trough = text(&frame(90, 30, 42, 9.0, &k));
        assert_ne!(full, trough);
        assert!((breathing_frac(0.0) - 1.0).abs() < 1e-5);
        assert!(breathing_frac(9.0) < breathing_frac(0.0));
    }

    #[test]
    fn nearby_perceive_values_stay_related() {
        let mut a = knobs();
        a[2] = 6.5;
        let mut b = knobs();
        b[2] = 7.5;
        let ta = text(&frame(90, 30, 42, 20.0, &a));
        let tb = text(&frame(90, 30, 42, 20.0, &b));
        assert_ne!(ta, tb);
        let ink_a = ta.chars().filter(|c| !c.is_whitespace()).count() as f32;
        let ink_b = tb.chars().filter(|c| !c.is_whitespace()).count() as f32;
        let ratio = (ink_a - ink_b).abs() / ink_a.max(ink_b).max(1.0);
        assert!(ratio < 0.4, "nearby PERCEIVE values diverged too much: {ink_a} vs {ink_b}");
    }

    #[test]
    fn repel_changes_the_structure() {
        let mut k = knobs();
        let a = text(&frame(90, 30, 42, 20.0, &k));
        k[5] = 0.0;
        assert_ne!(a, text(&frame(90, 30, 42, 20.0, &k)));
    }

    #[test]
    fn small_grid_does_not_panic() {
        let _ = frame(6, 4, 42, 5.0, &knobs());
        let _ = frame(1, 1, 42, 5.0, &knobs());
    }

    #[test]
    fn ink_covers_a_useful_share() {
        let k = knobs();
        let art = text(&frame(90, 30, 42, 20.0, &k));
        let ink = art.chars().filter(|c| !c.is_whitespace()).count();
        let cells = 90 * 30;
        assert!(ink > cells / 20, "too sparse: {ink} of {cells}");
        assert!(ink < cells * 4 / 5, "too dense: {ink} of {cells}");
    }

    #[test]
    fn frame_cost() {
        let (w, h) = (200usize, 60usize);
        let k = knobs();
        let mut worst = 0.0f64;
        let start = std::time::Instant::now();
        for f in 0..60 {
            let t0 = std::time::Instant::now();
            frame(w, h, 42, f as f32 * 0.3, &k);
            worst = worst.max(t0.elapsed().as_secs_f64() * 1000.0);
        }
        let avg = start.elapsed().as_secs_f64() * 1000.0 / 60.0;
        eprintln!("synaptic-bloom frame_cost 200x60: avg {:.3} ms, worst {:.3} ms", avg, worst);
        if !cfg!(debug_assertions) {
            assert!(avg < 8.0, "avg frame {:.3} ms", avg);
        }
    }
}
