//! mahoraga-4 -- Shibuya on a rig: 20-bone FK skeleton, keyframe blends,
//! per-seed joint noise, sway over t, IK so the pointing arm finds Sukuna.
use crate::_0_profile::measure_layer;
use crate::color::*;
use crate::opts::param_f32;
use crate::pp::ease_in_out;
use crate::types::*;
use crossterm::style::Color;
use rand::RngExt;
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::f32::consts::{PI, TAU};

pub struct ShrineKnobs {
    pub turns: f32,    // 0..8 -- static frame: adaptations already made
    pub fuga: u8,      // 1..8 -- adaptation that draws the Fire Arrow
    pub slash: f32,    // 0..24 -- Dismantle cut count
    pub cut: f32,      // 0..4 -- slip across each cut, in cells
    pub focus: f32,    // 0..1 -- depth in focus (0 far, 1 near)
    pub blur: f32,     // 0..2 -- how fast focus falls off with depth distance
    pub density: f32,  // 0..60 -- building count
    pub scale: f32,    // 0.3..1 -- figure height as a fraction of the grid
    pub light: f32,    // degrees -- key light direction
    pub grain: f32,    // 0..1 -- dither on the shade ramp
    pub haze: f32,     // 0..1 -- atmospheric fade on far blocks
    pub lean: f32,     // -0.3..0.3 -- forward hunch shear
    pub pose_a: u8,    // 0 stand, 1 point, 2 guard, 3 lunge, 4 adapt, 5 swing
    pub pose_b: u8,    // second keyframe
    pub blend: f32,    // 0..1 -- pose_a to pose_b
    pub noise: f32,    // 0..0.6 rad -- per-seed joint perturbation
    pub jitter: f32,   // 0..0.4 rad -- sway amplitude over t
    pub breath: f32,   // 0..0.2 rad -- spine breathing over t
    pub hold: f32,     // 0.5..6 -- animation units per keyframe
    pub aim: f32,      // 0/1 -- pointing arm IK-aims at Sukuna
    pub shrine: f32,   // 0..1 -- Malevolent Shrine presence behind him
    pub sukuna: f32,   // 0..1 -- foreground Sukuna silhouette size (0 hides)
    pub ash: f32,      // 0..1 -- falling ash density
    pub debris: f32,   // 0..1 -- shards off the cut ends
    pub aura: f32,     // 0..1 -- adaptation rings behind the figure
    pub vignette: f32, // 0..1 -- edge darkening
    pub speed: f32,    // 0.1..3 -- adaptations per animation unit
}

impl ShrineKnobs {
    pub fn from_env() -> Self {
        ShrineKnobs {
            turns: param_f32("TURNS", 7.0).clamp(0.0, 8.0),
            fuga: param_f32("FUGA", 8.0).clamp(1.0, 8.0) as u8,
            slash: param_f32("SLASH", 7.0).clamp(0.0, 24.0),
            cut: param_f32("CUT", 1.5).clamp(0.0, 4.0),
            focus: param_f32("FOCUS", 0.45).clamp(0.0, 1.0),
            blur: param_f32("BLUR", 1.0).clamp(0.0, 2.0),
            density: param_f32("DENS", 22.0).clamp(0.0, 60.0),
            scale: param_f32("SCALE", 0.8).clamp(0.3, 1.0),
            light: param_f32("LIGHT", 215.0),
            grain: param_f32("GRAIN", 0.18).clamp(0.0, 1.0),
            haze: param_f32("HAZE", 0.6).clamp(0.0, 1.0),
            lean: param_f32("LEAN", 0.08).clamp(-0.3, 0.3),
            pose_a: param_f32("POSEA", 1.0).clamp(0.0, 5.0) as u8,
            pose_b: param_f32("POSEB", 4.0).clamp(0.0, 5.0) as u8,
            blend: param_f32("BLEND", 0.0).clamp(0.0, 1.0),
            noise: param_f32("NOISE", 0.12).clamp(0.0, 0.6),
            jitter: param_f32("JITTER", 0.08).clamp(0.0, 0.4),
            breath: param_f32("BREATH", 0.04).clamp(0.0, 0.2),
            hold: param_f32("HOLD", 2.0).clamp(0.5, 6.0),
            aim: param_f32("AIM", 1.0).clamp(0.0, 1.0),
            shrine: param_f32("SHRINE", 0.7).clamp(0.0, 1.0),
            sukuna: param_f32("SUKUNA", 0.42).clamp(0.0, 1.0),
            ash: param_f32("ASH", 0.25).clamp(0.0, 1.0),
            debris: param_f32("DEBRIS", 0.6).clamp(0.0, 1.0),
            aura: param_f32("AURA", 0.35).clamp(0.0, 1.0),
            vignette: param_f32("VIG", 0.45).clamp(0.0, 1.0),
            speed: param_f32("SPEED", 1.0).clamp(0.1, 3.0),
        }
    }
}

// ── small math ──────────────────────────────────────────────────────

type P = (f32, f32);

fn sub(a: P, b: P) -> P {
    (a.0 - b.0, a.1 - b.1)
}
fn dot(a: P, b: P) -> f32 {
    a.0 * b.0 + a.1 * b.1
}
fn len(a: P) -> f32 {
    dot(a, a).sqrt()
}

fn sd_circle(p: P, c: P, r: f32) -> f32 {
    len(sub(p, c)) - r
}

fn sd_seg(p: P, a: P, b: P, r: f32) -> f32 {
    let pa = sub(p, a);
    let ba = sub(b, a);
    let h = (dot(pa, ba) / dot(ba, ba).max(1e-6)).clamp(0.0, 1.0);
    len((pa.0 - ba.0 * h, pa.1 - ba.1 * h)) - r
}

fn sd_ellipse(p: P, c: P, rx: f32, ry: f32) -> f32 {
    let q = ((p.0 - c.0) / rx, (p.1 - c.1) / ry);
    (len(q) - 1.0) * rx.min(ry)
}

fn sd_box(p: P, c: P, hx: f32, hy: f32) -> f32 {
    let dx = (p.0 - c.0).abs() - hx;
    let dy = (p.1 - c.1).abs() - hy;
    len((dx.max(0.0), dy.max(0.0))) + dx.max(dy).min(0.0)
}

fn hash01(seed: u64, x: i64, y: i64) -> f32 {
    let mut h = seed ^ (x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (y as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    h ^= h >> 29;
    h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h ^= h >> 32;
    (h & 0xFFFF) as f32 / 65536.0
}

fn side_rng(seed: u64, layer: u64) -> StdRng {
    StdRng::seed_from_u64(seed ^ layer.wrapping_mul(0x9E37_79B9))
}

fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Glyph for a screen-space direction (2:1 cell aspect already applied).
fn stroke_glyph(dx: f32, dy: f32) -> char {
    let ax = dx.abs();
    let ay = dy.abs();
    if ax > ay * 3.2 {
        '─'
    } else if ay > ax * 1.6 {
        '│'
    } else if (dx > 0.0) == (dy > 0.0) {
        '\\'
    } else {
        '/'
    }
}

fn set(grid: &mut Grid, x: i32, y: i32, ch: char, fg: Color) {
    if x >= 0 && y >= 0 && (y as usize) < grid.len() && (x as usize) < grid[0].len() {
        grid[y as usize][x as usize] = Cell::new(ch, fg);
    }
}

// ── the figure: bone rig ────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
enum Part {
    Skin,
    Cloth,
    Wrap,
    Ring,
    Spoke,
    Handle(usize),
    Hub,
    Eye,
    Mouth,
}

/// One bone: attaches `anchor` of the way along its parent (root when None),
/// plus `off`, then runs `len` at `rest` world angle. Angle 0 = down, +pi/2 = right.
struct BoneDef {
    parent: Option<usize>,
    anchor: f32,
    off: P,
    len: f32,
    rest: f32,
    r0: f32,
    r1: f32,
    part: Part,
    drawn: bool,
}

const fn bone(parent: Option<usize>, anchor: f32, off: P, len: f32, rest: f32, r0: f32, r1: f32, part: Part, drawn: bool) -> BoneDef {
    BoneDef { parent, anchor, off, len, rest, r0, r1, part, drawn }
}

const UP: f32 = PI;
const RIGHT: f32 = PI / 2.0;
const LEFT: f32 = -PI / 2.0;

const SPINE: usize = 0;
const NECK: usize = 1;
const HEAD: usize = 2;
const CLAV_R: usize = 3;
const UPPER_R: usize = 4;
const FORE_R: usize = 5;
const HAND_R: usize = 6;
const FINGER_R: usize = 7;
const CLAV_L: usize = 8;
const UPPER_L: usize = 9;
const FORE_L: usize = 10;
const HAND_L: usize = 11;
const FINGER_L: usize = 12;
const THIGH_R: usize = 13;
const SHIN_R: usize = 14;
const FOOT_R: usize = 15;
const THIGH_L: usize = 16;
const SHIN_L: usize = 17;
const FOOT_L: usize = 18;
const CLOTH: usize = 19;
const WHEEL: usize = 20;
const NBONES: usize = 21;

const ROOT: P = (0.0, 0.66);
const WHEEL_R: f32 = 0.105;

static BONES: [BoneDef; NBONES] = [
    bone(None, 0.0, (0.0, 0.0), 0.24, UP, 0.095, 0.13, Part::Skin, true),
    bone(Some(SPINE), 1.0, (0.0, 0.0), 0.06, UP, 0.032, 0.032, Part::Skin, true),
    bone(Some(NECK), 1.0, (0.0, 0.0), 0.11, UP, 0.056, 0.05, Part::Skin, true),
    bone(Some(SPINE), 0.9, (0.0, 0.0), 0.2, RIGHT, 0.07, 0.06, Part::Skin, true),
    bone(Some(CLAV_R), 1.0, (0.0, 0.0), 0.17, 0.4, 0.046, 0.046, Part::Skin, true),
    bone(Some(UPPER_R), 1.0, (0.0, 0.0), 0.2, 0.15, 0.058, 0.058, Part::Wrap, true),
    bone(Some(FORE_R), 1.0, (0.0, 0.0), 0.08, 0.15, 0.052, 0.045, Part::Skin, true),
    bone(Some(HAND_R), 1.0, (0.0, 0.0), 0.1, 0.15, 0.015, 0.012, Part::Skin, true),
    bone(Some(SPINE), 0.9, (0.0, 0.0), 0.2, LEFT, 0.07, 0.06, Part::Skin, true),
    bone(Some(CLAV_L), 1.0, (0.0, 0.0), 0.17, -0.4, 0.046, 0.046, Part::Skin, true),
    bone(Some(UPPER_L), 1.0, (0.0, 0.0), 0.2, -0.15, 0.058, 0.058, Part::Wrap, true),
    bone(Some(FORE_L), 1.0, (0.0, 0.0), 0.08, -0.15, 0.052, 0.045, Part::Skin, true),
    bone(Some(HAND_L), 1.0, (0.0, 0.0), 0.1, -0.15, 0.015, 0.012, Part::Skin, true),
    bone(None, 0.0, (0.06, 0.02), 0.16, 0.08, 0.056, 0.05, Part::Skin, true),
    bone(Some(THIGH_R), 1.0, (0.0, 0.0), 0.14, 0.0, 0.05, 0.045, Part::Skin, true),
    bone(Some(SHIN_R), 1.0, (0.0, 0.0), 0.06, RIGHT, 0.042, 0.035, Part::Skin, true),
    bone(None, 0.0, (-0.06, 0.02), 0.16, -0.08, 0.056, 0.05, Part::Skin, true),
    bone(Some(THIGH_L), 1.0, (0.0, 0.0), 0.14, 0.0, 0.05, 0.045, Part::Skin, true),
    bone(Some(SHIN_L), 1.0, (0.0, 0.0), 0.06, LEFT, 0.042, 0.035, Part::Skin, true),
    bone(None, 0.0, (0.0, -0.02), 0.18, 0.0, 0.13, 0.08, Part::Cloth, true),
    bone(Some(HEAD), 1.0, (0.0, 0.0), 0.14, UP, 0.0, 0.0, Part::Skin, false),
];

pub const POSE_NAMES: [&str; 6] = ["stand", "point", "guard", "lunge", "adapt", "swing"];

/// Joint deltas (radians, relative to rest) for each keyframe.
fn keyframe(idx: usize) -> [f32; NBONES] {
    let mut d = [0.0f32; NBONES];
    let mut set = |pairs: &[(usize, f32)]| {
        for &(i, v) in pairs {
            d[i] = v;
        }
    };
    match idx % 6 {
        1 => set(&[(UPPER_R, 1.5), (FORE_R, -0.1), (SPINE, 0.06), (NECK, 0.12)]),
        2 => set(&[(UPPER_R, 0.9), (FORE_R, 2.0), (UPPER_L, -0.9), (FORE_L, -2.0), (SPINE, -0.05), (NECK, -0.1)]),
        3 => set(&[
            (SPINE, 0.4),
            (NECK, 0.15),
            (UPPER_R, 2.3),
            (FORE_R, 0.5),
            (UPPER_L, -1.3),
            (FORE_L, -0.3),
            (THIGH_R, 0.85),
            (SHIN_R, -0.5),
            (THIGH_L, -0.45),
            (SHIN_L, 0.2),
            (CLOTH, 0.25),
        ]),
        4 => set(&[
            (SPINE, -0.18),
            (NECK, 0.25),
            (UPPER_R, 0.9),
            (FORE_R, -0.6),
            (UPPER_L, -0.9),
            (FORE_L, 0.6),
            (THIGH_R, 0.35),
            (SHIN_R, -0.3),
            (THIGH_L, -0.35),
            (SHIN_L, 0.3),
        ]),
        5 => set(&[(UPPER_R, 3.0), (FORE_R, 0.7), (UPPER_L, -0.5), (FORE_L, -0.4), (SPINE, -0.2), (NECK, -0.2), (THIGH_L, -0.2)]),
        _ => {}
    }
    d
}

fn dir(a: f32) -> P {
    (a.sin(), a.cos())
}

#[derive(Clone, Copy)]
struct Seg {
    a: P,
    b: P,
    ang: f32,
}

/// Forward kinematics: world segment per bone from the joint deltas.
fn solve(deltas: &[f32; NBONES]) -> [Seg; NBONES] {
    let mut out = [Seg { a: ROOT, b: ROOT, ang: 0.0 }; NBONES];
    for (i, bd) in BONES.iter().enumerate() {
        let (start, parent_ang, parent_rest) = match bd.parent {
            Some(pi) => {
                let ps = out[pi];
                let s = (ps.a.0 + (ps.b.0 - ps.a.0) * bd.anchor, ps.a.1 + (ps.b.1 - ps.a.1) * bd.anchor);
                (s, ps.ang, BONES[pi].rest)
            }
            None => (ROOT, 0.0, 0.0),
        };
        let ang = parent_ang + (bd.rest - parent_rest) + deltas[i];
        let a = (start.0 + bd.off.0, start.1 + bd.off.1);
        let d = dir(ang);
        out[i] = Seg { a, b: (a.0 + d.0 * bd.len, a.1 + d.1 * bd.len), ang };
    }
    out
}

/// Two-bone IK on one arm chain so the finger tip reaches toward `target`.
fn aim_arm(deltas: &mut [f32; NBONES], target: P, chain: [usize; 5], bend_sign: f32) {
    let [clav_i, upper_i, fore_i, hand_i, finger_i] = chain;
    let segs = solve(deltas);
    let s = segs[upper_i].a;
    let l1 = BONES[upper_i].len;
    let l2 = BONES[fore_i].len + BONES[hand_i].len + BONES[finger_i].len;
    let to = sub(target, s);
    let d = len(to).clamp(0.05, l1 + l2 - 0.01);
    let theta = to.0.atan2(to.1);
    let cos_a = ((l1 * l1 + d * d - l2 * l2) / (2.0 * l1 * d)).clamp(-1.0, 1.0);
    let upper = theta - cos_a.acos() * bend_sign;
    let elbow = (s.0 + upper.sin() * l1, s.1 + upper.cos() * l1);
    let rest_e = sub(target, elbow);
    let fore = rest_e.0.atan2(rest_e.1);
    let clav = segs[clav_i].ang;
    deltas[upper_i] = upper - (clav + (BONES[upper_i].rest - BONES[clav_i].rest));
    deltas[fore_i] = fore - (upper + (BONES[fore_i].rest - BONES[upper_i].rest));
    deltas[hand_i] = 0.0;
    deltas[finger_i] = 0.0;
}

/// Point with whichever arm is on the target's side; the other arm hangs.
fn aim_right_arm(deltas: &mut [f32; NBONES], target: P) {
    if target.0 >= 0.0 {
        aim_arm(deltas, target, [CLAV_R, UPPER_R, FORE_R, HAND_R, FINGER_R], 1.0);
    } else {
        deltas[UPPER_R] = 0.0;
        deltas[FORE_R] = 0.0;
        aim_arm(deltas, target, [CLAV_L, UPPER_L, FORE_L, HAND_L, FINGER_L], -1.0);
    }
}

fn sd_bone(p: P, s: &Seg, r0: f32, r1: f32) -> f32 {
    let pa = sub(p, s.a);
    let ba = sub(s.b, s.a);
    let h = (dot(pa, ba) / dot(ba, ba).max(1e-6)).clamp(0.0, 1.0);
    len((pa.0 - ba.0 * h, pa.1 - ba.1 * h)) - (r0 + (r1 - r0) * h)
}

/// Figure space: u right, v down, 1.0 = figure height, square cells.
struct Figure {
    rot: f32,
    lean: f32,
    lit: usize,
    segs: [Seg; NBONES],
    wheel_c: P,
}

impl Figure {
    fn new(rot: f32, lean: f32, lit: usize, deltas: &[f32; NBONES]) -> Self {
        let segs = solve(deltas);
        Figure { rot, lean, lit, segs, wheel_c: segs[WHEEL].b }
    }

    fn warp(&self, p: P) -> P {
        (p.0 - self.lean * (0.62 - p.1), p.1)
    }

    fn body(&self, p: P) -> (f32, Part) {
        let p = self.warp(p);
        let mut best = (f32::MAX, Part::Skin);
        for (i, bd) in BONES.iter().enumerate() {
            if !bd.drawn {
                continue;
            }
            let d = sd_bone(p, &self.segs[i], bd.r0, bd.r1);
            if d < best.0 {
                best = (d, bd.part);
            }
        }
        let head = self.segs[HEAD];
        let hd = dir(head.ang);
        let side = (hd.1, -hd.0);
        let eye_c = (head.a.0 + hd.0 * 0.055, head.a.1 + hd.1 * 0.055);
        for s in [-1.0f32, 1.0] {
            let c = (eye_c.0 + side.0 * 0.022 * s, eye_c.1 + side.1 * 0.022 * s);
            let d = sd_circle(p, c, 0.011);
            if d < best.0 {
                best = (d, Part::Eye);
            }
        }
        let m = (head.a.0 + hd.0 * 0.02, head.a.1 + hd.1 * 0.02);
        let d = sd_seg(p, (m.0 - side.0 * 0.024, m.1 - side.1 * 0.024), (m.0 + side.0 * 0.024, m.1 + side.1 * 0.024), 0.006);
        if d < best.0 {
            best = (d, Part::Mouth);
        }
        best
    }

    fn wheel(&self, p: P) -> (f32, Part) {
        let p = self.warp(p);
        let c = self.wheel_c;
        let mut best = ((len(sub(p, c)) - WHEEL_R).abs() - 0.014, Part::Ring);
        let hub = sd_circle(p, c, 0.009);
        if hub < best.0 {
            best = (hub, Part::Hub);
        }
        for k in 0..8 {
            let a = self.rot + k as f32 * TAU / 8.0;
            let d = (a.cos(), a.sin());
            let inner = (c.0 + d.0 * WHEEL_R * 0.25, c.1 + d.1 * WHEEL_R * 0.25);
            let outer = (c.0 + d.0 * WHEEL_R * 1.32, c.1 + d.1 * WHEEL_R * 1.32);
            let sp = sd_seg(p, inner, outer, 0.011);
            if sp < best.0 {
                best = (sp, Part::Spoke);
            }
            let h = sd_circle(p, outer, 0.02);
            if h < best.0 {
                best = (h, Part::Handle(k));
            }
        }
        best
    }

    fn sample(&self, p: P) -> (f32, Part) {
        let b = self.body(p);
        let w = self.wheel(p);
        if w.0 < b.0 { w } else { b }
    }

    fn normal(&self, p: P) -> P {
        let e = 0.004;
        let dx = self.body((p.0 + e, p.1)).0 - self.body((p.0 - e, p.1)).0;
        let dy = self.body((p.0, p.1 + e)).0 - self.body((p.0, p.1 - e)).0;
        let l = len((dx, dy)).max(1e-6);
        (dx / l, dy / l)
    }
}

// ── posing ──────────────────────────────────────────────────────────

const CYCLE: [usize; 6] = [0, 4, 2, 1, 3, 5];

/// Blend keyframes, add seed noise and sway, then aim the arm when pointing.
fn pose_deltas(seed: u64, t: f32, knobs: &ShrineKnobs, aim: Option<P>) -> [f32; NBONES] {
    let (ia, ib, w) = if t > 0.0 {
        let seg = t * knobs.speed / knobs.hold.max(0.1);
        let i = seg.floor() as usize;
        let f = seg.fract();
        (CYCLE[i % 6], CYCLE[(i + 1) % 6], smoothstep(0.5, 1.0, f))
    } else {
        (knobs.pose_a as usize, knobs.pose_b as usize, knobs.blend)
    };
    let mut a = keyframe(ia);
    let b = keyframe(ib);
    let wa = if ia == 1 { 1.0 - w } else { 0.0 } + if ib == 1 { w } else { 0.0 };
    if let Some(target) = aim {
        if ia == 1 {
            aim_right_arm(&mut a, target);
        }
    }
    let mut b = b;
    if let Some(target) = aim {
        if ib == 1 {
            aim_right_arm(&mut b, target);
        }
    }
    let _ = wa;
    let mut d = [0.0f32; NBONES];
    let mut rng = side_rng(seed, 7);
    for i in 0..NBONES {
        d[i] = a[i] + (b[i] - a[i]) * w;
        if i != WHEEL {
            d[i] += (rng.random::<f32>() - 0.5) * 2.0 * knobs.noise;
        }
        if t > 0.0 && i != WHEEL {
            let freq: f32 = rng.random_range(0.6..1.7);
            let phase: f32 = rng.random_range(0.0..TAU);
            d[i] += (t * freq + phase).sin() * knobs.jitter;
        } else {
            let _ = rng.random_range(0.6..1.7f32);
            let _ = rng.random_range(0.0..TAU);
        }
    }
    if t > 0.0 {
        d[SPINE] += (t * 1.3).sin() * knobs.breath;
        d[NECK] -= (t * 1.3).sin() * knobs.breath * 0.6;
    }
    d
}

// ── Sukuna, foreground ──────────────────────────────────────────────

/// Four-armed silhouette anchored at `c`, `s` = height in figure units.
fn sd_sukuna(p: P, c: P, s: f32) -> f32 {
    let q = ((p.0 - c.0) / s, (p.1 - c.1) / s);
    let mut d = sd_ellipse(q, (0.0, 0.1), 0.09, 0.1);
    d = d.min(sd_seg(q, (0.0, 0.2), (0.0, 0.55), 0.12));
    d = d.min(sd_seg(q, (-0.16, 0.24), (0.16, 0.24), 0.06));
    d = d.min(sd_seg(q, (0.16, 0.24), (0.42, 0.05), 0.045));
    d = d.min(sd_seg(q, (-0.16, 0.24), (-0.4, 0.12), 0.045));
    d = d.min(sd_seg(q, (0.12, 0.34), (0.36, 0.48), 0.04));
    d = d.min(sd_seg(q, (-0.12, 0.34), (-0.34, 0.5), 0.04));
    d = d.min(sd_seg(q, (-0.06, 0.55), (-0.08, 0.95), 0.06));
    d = d.min(sd_seg(q, (0.06, 0.55), (0.1, 0.95), 0.06));
    d * s
}

// ── the shrine ──────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
enum ShrinePart {
    Roof,
    Eave,
    Pillar,
    Skull,
    Horn,
    Wall,
}

/// Malevolent Shrine: eaves, ridge, skull with horns, pillars. Figure-space.
fn sd_shrine(p: P, presence: f32) -> Option<ShrinePart> {
    if presence <= 0.0 {
        return None;
    }
    let w = 0.75 + presence * 0.5;
    let ridge = sd_box(p, (0.0, -0.02), w * 0.55, 0.012);
    let eave_r = sd_seg(p, (w * 0.5, 0.06), (w, 0.0), 0.014);
    let eave_l = sd_seg(p, (-w * 0.5, 0.06), (-w, 0.0), 0.014);
    let mid = sd_box(p, (0.0, 0.05), w * 0.55, 0.012);
    let skull = sd_circle(p, (0.0, -0.12), 0.07).abs() - 0.012;
    let horn_r = sd_seg(p, (0.05, -0.16), (0.16, -0.32), 0.014);
    let horn_l = sd_seg(p, (-0.05, -0.16), (-0.16, -0.32), 0.014);
    let mut best: Option<(f32, ShrinePart)> = None;
    let mut take = |d: f32, part: ShrinePart| {
        if d < 0.0 && best.map_or(true, |b| d < b.0) {
            best = Some((d, part));
        }
    };
    take(ridge, ShrinePart::Roof);
    take(mid, ShrinePart::Roof);
    take(eave_r, ShrinePart::Eave);
    take(eave_l, ShrinePart::Eave);
    take(skull, ShrinePart::Skull);
    take(horn_r, ShrinePart::Horn);
    take(horn_l, ShrinePart::Horn);
    for k in 0..5 {
        let x = (k as f32 - 2.0) * w * 0.25;
        take(sd_box(p, (x, 0.45), 0.012, 0.4), ShrinePart::Pillar);
    }
    if best.is_none() && p.0.abs() < w * 0.55 && p.1 > 0.05 && p.1 < 0.85 {
        return Some(ShrinePart::Wall);
    }
    best.map(|b| b.1)
}

// ── the cuts ────────────────────────────────────────────────────────

struct Slash {
    q: P,
    d: P,
    n: P,
    half: f32,
    slip: f32,
    bright: f32,
}

fn make_slashes(seed: u64, count: usize, cut_uv: f32, u_span: f32, v_span: (f32, f32)) -> Vec<Slash> {
    let mut rng = side_rng(seed, 1);
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let family: f32 = if i % 3 == 2 { -1.0 } else { 1.0 };
        let a: f32 = family * rng.random_range(0.55..0.95) + rng.random_range(-0.06..0.06);
        let d = (a.cos(), a.sin());
        let qu = rng.random_range(-u_span * 0.7..u_span * 0.7) * rng.random::<f32>().sqrt();
        let qv = rng.random_range(v_span.0 * 0.6..v_span.1 * 0.95);
        let half = rng.random_range(0.35..0.75) * u_span.max(0.8);
        let slip = cut_uv * rng.random_range(0.5..1.4) * if rng.random::<bool>() { 1.0 } else { -1.0 };
        out.push(Slash { q: (qu, qv), d, n: (-d.1, d.0), half, slip, bright: rng.random() });
    }
    out
}

/// Slip a sample point across every cut it sits beside; report a cut hit.
fn displace(p: P, slashes: &[Slash], live: usize, blade: f32) -> (P, Option<(usize, f32)>) {
    let mut q = p;
    let mut hit = None;
    for (i, s) in slashes.iter().take(live).enumerate() {
        let rel = sub(p, s.q);
        let along = dot(rel, s.d);
        let across = dot(rel, s.n);
        if along.abs() > s.half {
            continue;
        }
        let fade = smoothstep(0.0, s.half * 0.25, s.half - along.abs());
        if across.abs() < blade * fade.max(0.35) {
            hit = Some((i, 1.0 - along.abs() / s.half));
        } else if across > 0.0 {
            q = (q.0 + s.d.0 * s.slip * fade, q.1 + s.d.1 * s.slip * fade);
        }
    }
    (q, hit)
}

// ── the city ────────────────────────────────────────────────────────

struct Building {
    u0: f32,
    u1: f32,
    top: f32,
    base: f32,
    z: f32,
    id: u64,
}

fn make_city(seed: u64, count: usize, u_span: f32, v_bot: f32, horizon: f32) -> Vec<Building> {
    let mut rng = side_rng(seed, 2);
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let z: f32 = rng.random::<f32>().powf(1.3);
        let wide = rng.random_range(0.06..0.22) * (0.5 + z * 1.6);
        let tall = rng.random_range(0.25..1.4) * (0.5 + z * 1.4);
        let mut u = rng.random_range(-u_span * 1.05..u_span * 1.05);
        let corridor = 0.42 + z * 0.18 + wide * 0.5;
        if z > 0.25 && u.abs() < corridor {
            u = (corridor + rng.random_range(0.0..0.25)) * if u < 0.0 { -1.0 } else { 1.0 };
        }
        let base = horizon + (v_bot - horizon) * z.powf(1.7);
        out.push(Building { u0: u - wide * 0.5, u1: u + wide * 0.5, top: base - tall, base, z, id: i as u64 });
    }
    out.sort_by(|a, b| b.z.partial_cmp(&a.z).unwrap());
    out
}

fn sample_city<'a>(city: &'a [Building], p: P) -> Option<&'a Building> {
    city.iter().find(|b| p.0 >= b.u0 && p.0 <= b.u1 && p.1 >= b.top && p.1 <= b.base)
}

// ── the clock ───────────────────────────────────────────────────────

/// t=0 shows `turns` adaptations; t>0 replays the fight on a loop.
fn progress(t: f32, knobs: &ShrineKnobs) -> f32 {
    if t <= 0.0 { knobs.turns } else { (t * knobs.speed).rem_euclid(knobs.fuga as f32 + 3.0) }
}

// ── render ──────────────────────────────────────────────────────────

const SKIN_RAMP: [char; 10] = [' ', '.', '·', ':', '-', '=', '+', '*', '%', '@'];
const CLOTH_RAMP: [char; 6] = [' ', '.', '-', '~', '=', '#'];
const DARK_RAMP: [char; 5] = ['@', '#', '%', '*', '+'];

struct Scene<'a> {
    fig: Figure,
    slashes: Vec<Slash>,
    live: usize,
    blade: f32,
    city: Vec<Building>,
    light: P,
    knobs: &'a ShrineKnobs,
    seed: u64,
    fig_h: f32,
    horizon: f32,
    v_top: f32,
    sukuna_c: P,
    sukuna_s: f32,
    t: f32,
}

struct Ink {
    pale: Color,
    bone: Color,
    cloth: Color,
    wrap: Color,
    ring: Color,
    cut: Color,
    shrine: Color,
    sukuna: Color,
    block_far: Color,
    block_near: Color,
    ground: Color,
    haze: Color,
}

fn block_cell(sc: &Scene, ink: &Ink, b: &Building, pu: P, x: usize, y: usize) -> Option<(char, Color)> {
    let k = sc.knobs;
    let defocus = ((b.z - k.focus).abs() * k.blur * 2.2).min(1.0);
    let fade = (1.0 - b.z) * k.haze;
    let base = darken(lerp_color(ink.block_far, ink.block_near, b.z), (fade * 110.0) as u8);
    let col = ((pu.0 - b.u0) * 2.0 * sc.fig_h).floor() as i64;
    let row = ((pu.1 - b.top) * sc.fig_h).floor() as i64;
    let width_cols = ((b.u1 - b.u0) * 2.0 * sc.fig_h).floor() as i64;
    if width_cols < 2 {
        return None;
    }
    let on_edge = col == 0 || col == width_cols;
    let on_roof = row == 0;
    let window = col % 3 == 1 && row % 2 == 1 && row > 0;
    let lit = hash01(sc.seed ^ b.id.wrapping_mul(977), col, row) < 0.55 - b.z * 0.2;
    let jitter = hash01(sc.seed ^ 0x51AB, x as i64, y as i64);
    Some(if defocus < 0.45 {
        if on_roof {
            ('─', lighten(base, 30))
        } else if on_edge {
            ('│', lighten(base, 20))
        } else if window {
            if lit { ('=', lighten(base, 70)) } else { ('.', base) }
        } else {
            (' ', base)
        }
    } else if window && lit && jitter < 1.0 - defocus * 0.4 {
        (if defocus > 0.8 { 'O' } else { 'o' }, darken(lighten(base, 60), (defocus * 50.0) as u8))
    } else if (on_edge || on_roof) && jitter > defocus * 0.9 {
        ('·', base)
    } else {
        (' ', base)
    })
}

fn shrine_cell(sc: &Scene, ink: &Ink, part: ShrinePart, pu: P, noise: f32) -> Option<(char, Color)> {
    let k = sc.knobs;
    let dim = darken(ink.shrine, (60.0 * (1.0 - k.shrine)) as u8);
    Some(match part {
        ShrinePart::Roof => ('═', lighten(dim, 20)),
        ShrinePart::Eave => (stroke_glyph(pu.0.signum() * 2.0, -1.0), lighten(dim, 15)),
        ShrinePart::Pillar => ('│', dim),
        ShrinePart::Skull => ('O', lighten(dim, 30)),
        ShrinePart::Horn => (stroke_glyph(pu.0.signum() * 2.0, -2.0), lighten(dim, 30)),
        ShrinePart::Wall => {
            if noise + 0.5 < k.shrine * 0.1 {
                (if noise > 0.3 { ':' } else { '·' }, darken(dim, 30))
            } else {
                return None;
            }
        }
    })
}

fn figure_cell(sc: &Scene, ink: &Ink, d: f32, part: Part, pu: P, noise: f32) -> (char, Color) {
    let k = sc.knobs;
    match part {
        Part::Ring => ('#', ink.ring),
        Part::Hub => ('◉', lighten(ink.ring, 40)),
        Part::Handle(i) => {
            if i < sc.fig.lit { ('◆', lighten(ink.ring, 45)) } else { ('·', darken(ink.ring, 30)) }
        }
        Part::Spoke => {
            let rel = sub(sc.fig.warp(pu), sc.fig.wheel_c);
            (stroke_glyph(rel.0 * 2.0, rel.1), darken(ink.ring, 15))
        }
        Part::Eye => ('◉', lighten(ink.cut, 10)),
        Part::Mouth => ('─', ink.bone),
        _ => {
            let n = sc.fig.normal(pu);
            let lit = 0.5 + 0.5 * dot(n, sc.light);
            let rim = smoothstep(0.0, 0.05, -d);
            let shade = (lit * 0.75 + (1.0 - rim) * 0.35 + noise * k.grain).clamp(0.0, 0.999);
            match part {
                Part::Cloth => {
                    let i = (shade * CLOTH_RAMP.len() as f32) as usize;
                    (CLOTH_RAMP[i], lerp_color(darken(ink.cloth, 60), ink.cloth, shade))
                }
                Part::Wrap => {
                    let band = ((pu.1 * sc.fig_h * 0.9) as i32).rem_euclid(3);
                    let ch = if band == 0 { '=' } else if shade > 0.5 { '-' } else { '.' };
                    (ch, lerp_color(darken(ink.wrap, 50), ink.wrap, shade))
                }
                _ => {
                    let i = (shade * SKIN_RAMP.len() as f32) as usize;
                    (SKIN_RAMP[i], lerp_color(ink.bone, ink.pale, shade))
                }
            }
        }
    }
}

/// One cell of the scene, front to back. None = leave blank.
fn shade_cell(sc: &Scene, ink: &Ink, x: usize, y: usize, p0: P) -> Option<(char, Color)> {
    let k = sc.knobs;
    let (pu, hit) = displace(p0, &sc.slashes, sc.live, sc.blade);
    if let Some((i, along)) = hit {
        let s = &sc.slashes[i];
        let glow = along * (0.5 + 0.5 * s.bright);
        if glow < 0.12 {
            return None;
        }
        let fg = if glow > 0.55 { ink.cut } else { darken(ink.cut, ((1.0 - glow) * 120.0) as u8) };
        return Some((stroke_glyph(s.d.0 * 2.0, s.d.1), fg));
    }
    let noise = hash01(sc.seed, x as i64, y as i64) - 0.5;

    if sc.sukuna_s > 0.0 {
        let d = sd_sukuna(pu, sc.sukuna_c, sc.sukuna_s);
        let rim = 0.45 / sc.fig_h;
        if d < -rim {
            let i = ((noise + 0.5) * (DARK_RAMP.len() as f32 - 0.01)) as usize;
            let ch = if noise + 0.5 < 0.12 { DARK_RAMP[i] } else { ' ' };
            return Some((ch, darken(ink.sukuna, 60)));
        }
        if d < 0.0 {
            let e = 0.003;
            let gx = sd_sukuna((pu.0 + e, pu.1), sc.sukuna_c, sc.sukuna_s) - sd_sukuna((pu.0 - e, pu.1), sc.sukuna_c, sc.sukuna_s);
            let gy = sd_sukuna((pu.0, pu.1 + e), sc.sukuna_c, sc.sukuna_s) - sd_sukuna((pu.0, pu.1 - e), sc.sukuna_c, sc.sukuna_s);
            let lit = 0.5 + 0.5 * dot((gx, gy), sc.light) / len((gx, gy)).max(1e-6);
            let ch = stroke_glyph(-gy * 2.0, gx);
            return Some((ch, lerp_color(ink.sukuna, ink.cut, lit * 0.8)));
        }
    }

    let (d, part) = sc.fig.sample(pu);
    if d < 0.0 {
        return Some(figure_cell(sc, ink, d, part, pu, noise));
    }
    let edge_eps = 0.3 / sc.fig_h;
    if d < edge_eps && matches!(part, Part::Skin | Part::Cloth | Part::Wrap) {
        let n = sc.fig.normal(pu);
        if n.0.abs() > 0.45 {
            return Some((stroke_glyph(-n.1 * 2.0, n.0), lerp_color(ink.bone, ink.pale, 0.6)));
        }
    }

    if k.aura > 0.0 && sc.fig.lit > 0 {
        let rel = sc.fig.warp(pu);
        let rr = len(((rel.0) / 0.62, (rel.1 - 0.55) / 0.52));
        for ring in 0..sc.fig.lit {
            let target = 1.0 + ring as f32 * 0.09;
            if (rr - target).abs() < 0.012 + k.aura * 0.01 {
                let pulse = if sc.t > 0.0 { ((sc.t * 2.0 - ring as f32 * 0.5).sin() * 0.5 + 0.5) * 0.4 } else { 0.0 };
                if noise + 0.5 < k.aura * 0.55 + pulse {
                    let ch = if ring % 2 == 0 { '·' } else { ':' };
                    return Some((ch, darken(ink.ring, (60 - ring as u8 * 6).max(10))));
                }
            }
        }
    }

    let block = sample_city(&sc.city, pu);
    if let Some(b) = block {
        if b.z >= 0.3 {
            return block_cell(sc, ink, b, pu, x, y);
        }
    }
    if let Some(part) = sd_shrine(sc.fig.warp(pu), k.shrine) {
        if let Some(c) = shrine_cell(sc, ink, part, pu, noise) {
            return Some(c);
        }
    }
    if let Some(b) = block {
        return block_cell(sc, ink, b, pu, x, y);
    }

    if pu.1 > sc.horizon {
        let rel = (pu.0, pu.1 - sc.horizon);
        let crater = sd_ellipse(pu, (0.0, 0.995), 0.5, 0.07);
        if crater.abs() < 0.012 {
            return Some((if noise > 0.0 { '~' } else { '-' }, lighten(ink.ground, 25)));
        }
        let ang = rel.0.atan2(rel.1.max(1e-4));
        let ray = ((ang * 9.0).rem_euclid(1.0) - 0.5).abs() < 0.035 && rel.1 > 0.02;
        let rubble = noise + 0.5 < (rel.1 * 1.6).min(0.6) * 0.5;
        if ray {
            return Some((stroke_glyph(ang.sin() * 2.0, ang.cos()), ink.ground));
        }
        if rubble {
            return Some((if noise > 0.2 { ':' } else { '·' }, darken(ink.ground, 25)));
        }
        return None;
    }
    let sky = (pu.1 - sc.v_top) / (sc.horizon - sc.v_top).max(0.1);
    if noise + 0.5 < sky * sky * 0.03 * (0.3 + k.haze) {
        return Some(('·', ink.haze));
    }
    None
}

fn draw_ash(grid: &mut Grid, width: usize, height: usize, seed: u64, t: f32, ash: f32, ink: Color) {
    if ash <= 0.0 {
        return;
    }
    let streams = (width as f32 * ash * 0.28) as usize;
    let mut rng = side_rng(seed, 4);
    for i in 0..streams {
        let x = rng.random_range(0..width) as i32;
        let phase: f32 = rng.random();
        let speed = rng.random_range(0.6..1.6);
        let length = rng.random_range(1..4);
        let head = ((phase + if t > 0.0 { t * speed * 0.35 } else { 0.0 }) * (height as f32 + 6.0)).rem_euclid(height as f32 + 6.0) as i32 - 3;
        for k in 0..length {
            let y = head - k;
            if y < 0 || y >= height as i32 || grid[y as usize][x as usize].ch != ' ' {
                continue;
            }
            let ch = match (k, i % 3) {
                (0, _) => '|',
                (_, 0) => ':',
                _ => '.',
            };
            set(grid, x, y, ch, darken(ink, 20 + k as u8 * 25));
        }
    }
}

fn draw_debris(grid: &mut Grid, sc: &Scene, ink: &Ink, cx: f32, top: f32) {
    let k = sc.knobs;
    if k.debris <= 0.0 {
        return;
    }
    let mut rng = side_rng(sc.seed, 5);
    let glyphs = ['#', '%', '&', '*', '+'];
    for s in sc.slashes.iter().take(sc.live) {
        for end in [-1.0f32, 1.0] {
            let base = (s.q.0 + s.d.0 * s.half * end, s.q.1 + s.d.1 * s.half * end);
            let n = (k.debris * 5.0) as usize + 1;
            for j in 0..n {
                let spread: f32 = rng.random_range(0.0..0.12);
                let side: f32 = rng.random_range(-0.06..0.06);
                let drift = if sc.t > 0.0 { (sc.t * 0.7 + j as f32).sin() * 0.02 } else { 0.0 };
                let pu = (base.0 + s.d.0 * spread * end + s.n.0 * side + drift, base.1 + s.d.1 * spread * end + s.n.1 * side);
                let x = (cx + pu.0 * 2.0 * sc.fig_h).round() as i32;
                let y = (top + pu.1 * sc.fig_h).round() as i32;
                let g = glyphs[rng.random_range(0..glyphs.len())];
                set(grid, x, y, g, darken(ink.cut, rng.random_range(30..110)));
            }
        }
    }
}

fn vignette(grid: &mut Grid, width: usize, height: usize, amount: f32) {
    if amount <= 0.0 {
        return;
    }
    for (y, row) in grid.iter_mut().enumerate() {
        for (x, cell) in row.iter_mut().enumerate() {
            let du = (x as f32 / width as f32 - 0.5) * 2.0;
            let dv = (y as f32 / height as f32 - 0.5) * 2.0;
            let r = (du * du + dv * dv).sqrt();
            let dark = smoothstep(0.55, 1.35, r) * amount;
            if dark > 0.0 {
                cell.fg = darken(cell.fg, (dark * 150.0) as u8);
            }
        }
    }
}

pub fn draw_mahoraga4(grid: &mut Grid, width: usize, height: usize, seed: u64, palette: &[Color; 5], rng: &mut StdRng, t: f32, knobs: &ShrineKnobs) {
    let _ = rng;
    let p = progress(t, knobs);
    let adaptations = (p.floor() as usize).min(8);
    let reach = (p - knobs.fuga as f32 + 1.0).clamp(0.0, 1.0);
    let frac = p.fract();
    let turn = if t > 0.0 && frac < 0.3 { ease_in_out(frac / 0.3) } else { 1.0 };
    let rot = (adaptations as f32 - 1.0 + turn).max(0.0) * (TAU / 8.0) - PI / 2.0;

    let fig_h = (height as f32 * knobs.scale).max(6.0);
    let top = (height as f32 - fig_h) * 0.6;
    let cx = width as f32 / 2.0 - 0.5;
    let u_span = width as f32 / (4.0 * fig_h);
    let v_top = -top / fig_h;
    let v_bot = (height as f32 - top) / fig_h;
    let horizon = 0.965;

    let live = if t > 0.0 {
        (knobs.slash * (0.3 + 0.7 * (p / knobs.fuga as f32).min(1.0))).round() as usize
    } else {
        knobs.slash.round() as usize
    };
    let sukuna_s = if knobs.sukuna > 0.0 && height >= 20 { knobs.sukuna } else { 0.0 };
    let sukuna_c = (-u_span * 0.62, v_bot - sukuna_s * 0.98);
    let aim = if knobs.aim > 0.5 && sukuna_s > 0.0 { Some((sukuna_c.0 + 0.05, sukuna_c.1 + sukuna_s * 0.1)) } else { Some((0.62, 0.3)) };
    let deltas = measure_layer("mahoraga-4", "pose", || pose_deltas(seed, t, knobs, aim));
    let fig = Figure::new(rot, knobs.lean, adaptations, &deltas);
    let wheel_c = fig.wheel_c;
    let sc = Scene {
        fig,
        slashes: measure_layer("mahoraga-4", "slashes", || make_slashes(seed, knobs.slash.round() as usize, knobs.cut / (2.0 * fig_h), u_span, (v_top, v_bot))),
        live,
        blade: 0.26 / fig_h,
        city: measure_layer("mahoraga-4", "city", || make_city(seed, knobs.density.round() as usize, u_span, v_bot, horizon)),
        light: (knobs.light.to_radians().cos(), knobs.light.to_radians().sin()),
        knobs,
        seed,
        fig_h,
        horizon,
        v_top,
        sukuna_c,
        sukuna_s,
        t,
    };
    let pale = lerp_color(lighten(palette[4], 40), rgb(236, 230, 222), 0.55);
    let ink = Ink {
        pale,
        bone: darken(pale, 90),
        cloth: lighten(palette[1], 20),
        wrap: lighten(palette[3], 10),
        ring: lighten(palette[3], 35),
        cut: lerp_color(lighten(palette[2], 40), rgb(250, 245, 235), 0.4),
        shrine: lerp_color(palette[1], palette[2], 0.5),
        sukuna: lerp_color(palette[2], rgb(40, 12, 18), 0.6),
        block_far: palette[0],
        block_near: palette[1],
        ground: darken(palette[1], 40),
        haze: darken(palette[3], 70),
    };

    measure_layer("mahoraga-4", "shade", || {
        for y in 0..height {
            for x in 0..width {
                let p0 = ((x as f32 - cx) / (2.0 * fig_h), (y as f32 - top) / fig_h);
                if let Some((ch, fg)) = shade_cell(&sc, &ink, x, y, p0) {
                    set(grid, x as i32, y as i32, ch, fg);
                }
            }
        }
    });

    measure_layer("mahoraga-4", "ash", || draw_ash(grid, width, height, seed, t, knobs.ash, lighten(palette[3], 10)));
    measure_layer("mahoraga-4", "debris", || draw_debris(grid, &sc, &ink, cx, top));
    measure_layer("mahoraga-4", "vignette", || vignette(grid, width, height, knobs.vignette));
    let wx = cx + (wheel_c.0 + knobs.lean * (0.62 - wheel_c.1)) * 2.0 * fig_h;
    let wy = top + fig_h * wheel_c.1;
    measure_layer("mahoraga-4", "fuga", || draw_fuga(grid, width, height, seed, cx, wy, fig_h * WHEEL_R * 1.3, t, reach));
    if reach >= 1.0 {
        set(grid, wx.round() as i32, wy.round() as i32, '◉', hsl_to_rgb(48.0, 1.0, 0.8));
    }
}

fn draw_fuga(grid: &mut Grid, width: usize, height: usize, seed: u64, cx: f32, cy: f32, r: f32, t: f32, reach: f32) {
    if reach <= 0.0 {
        return;
    }
    let ox = width as f32 * 0.04;
    let oy = height as f32 * 0.1;
    let far_x = cx + (cx - ox) * 1.4;
    let far_y = cy + (cy - oy) * 1.4;
    let tx = ox + (far_x - ox) * reach;
    let ty = oy + (far_y - oy) * reach;
    let core = hsl_to_rgb(48.0, 1.0, 0.72);
    let mid = hsl_to_rgb(28.0, 1.0, 0.55);
    let outer = hsl_to_rgb(8.0, 0.95, 0.42);
    let shimmer = if t > 0.0 { (t * 9.0) as usize } else { 0 };
    let n = (tx - ox).abs().max((ty - oy).abs()).ceil().max(1.0) as usize;
    let bands: &[i32] = if height >= 30 { &[-2, -1, 0, 1, 2] } else { &[-1, 0, 1] };
    for &band in bands {
        for i in 0..=n {
            let f = i as f32 / n as f32;
            let x = (ox + (tx - ox) * f).round() as i32;
            let y = (oy + (ty - oy) * f).round() as i32 + band;
            let (ch, fg) = match band.abs() {
                0 => ('=', core),
                1 => (if (i + shimmer) % 3 == 0 { '~' } else { '-' }, mid),
                _ => {
                    if (i + shimmer) % 4 != 0 {
                        continue;
                    }
                    ('~', outer)
                }
            };
            set(grid, x, y, ch, fg);
        }
    }
    if reach < 1.0 {
        return;
    }
    let mut sparks = side_rng(seed, 3);
    let count = (r * 10.0) as usize;
    for i in 0..count {
        let a = sparks.random_range(0.0..TAU);
        let d = sparks.random_range(1.2..3.0) * r;
        let spin = if t > 0.0 { t * 0.8 + i as f32 * 0.05 } else { 0.0 };
        let x = cx + (a + spin).cos() * d * 2.0;
        let y = cy + (a + spin).sin() * d;
        let ch = if i % 4 == 0 { '+' } else { '*' };
        set(grid, x.round() as i32, y.round() as i32, ch, if i % 3 == 0 { core } else { mid });
    }
}

pub fn render_mahoraga4_frame(width: usize, height: usize, seed: u64, palette: &[Color; 5], mut rng: StdRng, t: f32, knobs: &ShrineKnobs) -> Grid {
    let mut grid = vec![vec![Cell::blank(); width]; height];
    draw_mahoraga4(&mut grid, width, height, seed, palette, &mut rng, t, knobs);
    grid
}

pub(crate) fn cli_mahoraga4(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
    // mahoraga-4 [turns] [slash] [cut] [focus] [pose_a] [pose_b] [blend] -- positional overrides win over env/defaults
    let mut knobs = ShrineKnobs::from_env();
    if let Some(v) = args.get(4).and_then(|v| v.parse::<f32>().ok()) {
        knobs.turns = v.clamp(0.0, 8.0);
    }
    if let Some(v) = args.get(5).and_then(|v| v.parse::<f32>().ok()) {
        knobs.slash = v.clamp(0.0, 24.0);
    }
    if let Some(v) = args.get(6).and_then(|v| v.parse::<f32>().ok()) {
        knobs.cut = v.clamp(0.0, 4.0);
    }
    if let Some(v) = args.get(7).and_then(|v| v.parse::<f32>().ok()) {
        knobs.focus = v.clamp(0.0, 1.0);
    }
    if let Some(v) = args.get(8).and_then(|v| v.parse::<f32>().ok()) {
        knobs.pose_a = v.clamp(0.0, 5.0) as u8;
    }
    if let Some(v) = args.get(9).and_then(|v| v.parse::<f32>().ok()) {
        knobs.pose_b = v.clamp(0.0, 5.0) as u8;
    }
    if let Some(v) = args.get(10).and_then(|v| v.parse::<f32>().ok()) {
        knobs.blend = v.clamp(0.0, 1.0);
    }
    let _ = (term_w, term_h, mode, theme_name);
    draw_mahoraga4(&mut grid, width, height, seed, &palette, &mut rng, t_anim, &knobs);
    (grid, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(w: usize, h: usize, seed: u64, t: f32, turns: f32, pa: u8, pb: u8, blend: f32) -> String {
        let p = crate::color::make_palette(seed);
        let mut knobs = ShrineKnobs::from_env();
        knobs.turns = turns;
        knobs.pose_a = pa;
        knobs.pose_b = pb;
        knobs.blend = blend;
        let g = render_mahoraga4_frame(w, h, seed, &p, StdRng::seed_from_u64(seed), t, &knobs);
        g.iter()
            .map(|row| row.iter().map(|c| c.ch).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn snapshot_mahoraga4_pointing() {
        insta::assert_snapshot!("mahoraga4_80x24", run(80, 24, 42, 0.0, 7.0, 1, 4, 0.0));
    }

    #[test]
    fn snapshot_mahoraga4_lunge_swing_blend_tall() {
        insta::assert_snapshot!("mahoraga4_100x40_blend", run(100, 40, 42, 0.0, 8.0, 3, 5, 0.5));
    }

    #[test]
    fn fk_rest_pose_is_upright() {
        let segs = solve(&[0.0; NBONES]);
        assert!((segs[HEAD].b.1 - 0.25).abs() < 0.02, "head top at {}", segs[HEAD].b.1);
        assert!(segs[CLAV_R].b.0 > 0.15 && segs[CLAV_L].b.0 < -0.15, "shoulders spread");
        assert!(segs[FOOT_R].a.1 > 0.95, "feet on the ground: {}", segs[FOOT_R].a.1);
        assert!((segs[WHEEL].b.1 - 0.11).abs() < 0.02, "wheel above head");
    }

    #[test]
    fn ik_reaches_target_either_side() {
        for (target, finger) in [((0.55, 0.3), FINGER_R), ((-0.5, 0.8), FINGER_L)] {
            let mut d = keyframe(1);
            aim_right_arm(&mut d, target);
            let segs = solve(&d);
            let miss = len(sub(segs[finger].b, target));
            assert!(miss < 0.03, "finger tip {:?} misses {:?} by {}", segs[finger].b, target, miss);
        }
    }

    #[test]
    fn blend_moves_between_keyframes() {
        let a = run(100, 40, 42, 0.0, 7.0, 0, 3, 0.0);
        let mid = run(100, 40, 42, 0.0, 7.0, 0, 3, 0.5);
        let b = run(100, 40, 42, 0.0, 7.0, 0, 3, 1.0);
        assert_ne!(a, mid);
        assert_ne!(mid, b);
        assert_eq!(b, run(100, 40, 42, 0.0, 7.0, 3, 0, 0.0));
    }

    #[test]
    fn deterministic_seed_sensitive_and_t_moves() {
        assert_eq!(run(90, 30, 42, 0.0, 7.0, 1, 4, 0.0), run(90, 30, 42, 0.0, 7.0, 1, 4, 0.0));
        assert_ne!(run(90, 30, 42, 0.0, 7.0, 1, 4, 0.0), run(90, 30, 7, 0.0, 7.0, 1, 4, 0.0));
        assert_ne!(run(90, 30, 42, 1.0, 7.0, 1, 4, 0.0), run(90, 30, 42, 3.0, 7.0, 1, 4, 0.0));
    }

    #[test]
    fn figure_and_arrow_present() {
        let s = run(100, 40, 42, 0.0, 8.0, 3, 5, 0.5);
        assert!(s.contains('◉'), "hub");
        assert!(s.contains('◆'), "handles");
        assert!(s.contains("~--~--~--"), "fire arrow band");
        assert!(!run(80, 24, 42, 0.0, 3.0, 1, 4, 0.0).contains("~--~--~--"), "no arrow before fuga");
    }
}
