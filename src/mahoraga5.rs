//! mahoraga-5 -- Shibuya, choreographed: generic bone rigs with depth order,
//! Sukuna rigged and cutting, contour hatching, sampled Fire Arrow, ghosts, shake.
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
    pub hatch: f32,    // 0..1 -- contour hatching in the mid-tones
    pub haze: f32,     // 0..1 -- atmospheric fade on far blocks
    pub lean: f32,     // -0.3..0.3 -- forward hunch shear
    pub pose_a: u8,    // 0 stand, 1 point, 2 guard, 3 lunge, 4 adapt, 5 swing, 6 flinch
    pub pose_b: u8,    // second keyframe
    pub blend: f32,    // 0..1 -- pose_a to pose_b
    pub noise: f32,    // 0..0.6 rad -- per-seed joint perturbation
    pub jitter: f32,   // 0..0.4 rad -- sway amplitude over t
    pub breath: f32,   // 0..0.2 rad -- spine breathing over t
    pub hold: f32,     // 0.5..6 -- animation units per keyframe
    pub aim: f32,      // 0/1 -- pointing arm IK-aims at Sukuna
    pub shrine: f32,   // 0..1 -- Malevolent Shrine presence behind him
    pub sukuna: f32,   // 0..1 -- Sukuna rig height in figure units (0 hides)
    pub sukpose: u8,   // 0 stance, 1 slash, 2 point, 3 crouch
    pub ash: f32,      // 0..1 -- falling ash density
    pub debris: f32,   // 0..1 -- shards off the cut ends
    pub aura: f32,     // 0..1 -- adaptation rings behind the figure
    pub vignette: f32, // 0..1 -- edge darkening
    pub ghosts: f32,   // 0..3 -- motion afterimages while animating
    pub shake: f32,    // 0..1 -- screen shake on each adaptation
    pub lines: f32,    // 0..1 -- speed lines behind a lunge
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
            grain: param_f32("GRAIN", 0.14).clamp(0.0, 1.0),
            hatch: param_f32("HATCH", 0.6).clamp(0.0, 1.0),
            haze: param_f32("HAZE", 0.6).clamp(0.0, 1.0),
            lean: param_f32("LEAN", 0.08).clamp(-0.3, 0.3),
            pose_a: param_f32("POSEA", 1.0).clamp(0.0, 6.0) as u8,
            pose_b: param_f32("POSEB", 4.0).clamp(0.0, 6.0) as u8,
            blend: param_f32("BLEND", 0.0).clamp(0.0, 1.0),
            noise: param_f32("NOISE", 0.12).clamp(0.0, 0.6),
            jitter: param_f32("JITTER", 0.08).clamp(0.0, 0.4),
            breath: param_f32("BREATH", 0.04).clamp(0.0, 0.2),
            hold: param_f32("HOLD", 2.0).clamp(0.5, 6.0),
            aim: param_f32("AIM", 1.0).clamp(0.0, 1.0),
            shrine: param_f32("SHRINE", 0.7).clamp(0.0, 1.0),
            sukuna: param_f32("SUKUNA", 0.45).clamp(0.0, 1.0),
            sukpose: param_f32("SUKPOSE", 1.0).clamp(0.0, 3.0) as u8,
            ash: param_f32("ASH", 0.25).clamp(0.0, 1.0),
            debris: param_f32("DEBRIS", 0.6).clamp(0.0, 1.0),
            aura: param_f32("AURA", 0.35).clamp(0.0, 1.0),
            vignette: param_f32("VIG", 0.45).clamp(0.0, 1.0),
            ghosts: param_f32("GHOSTS", 2.0).clamp(0.0, 3.0),
            shake: param_f32("SHAKE", 0.6).clamp(0.0, 1.0),
            lines: param_f32("LINES", 0.6).clamp(0.0, 1.0),
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

/// Axis-aligned bound around one primitive, used to skip it per cell and per row.
#[derive(Clone, Copy)]
struct Aabb {
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
}

/// Padding on every bound so rounding never culls a primitive the exact
/// distance test would have counted as a hit.
const SLACK: f32 = 1e-4;

impl Aabb {
    const EMPTY: Aabb = Aabb { x0: f32::MAX, y0: f32::MAX, x1: f32::MIN, y1: f32::MIN };

    fn seg(a: P, b: P, r: f32) -> Aabb {
        let r = r + SLACK;
        Aabb { x0: a.0.min(b.0) - r, y0: a.1.min(b.1) - r, x1: a.0.max(b.0) + r, y1: a.1.max(b.1) + r }
    }

    fn disc(c: P, r: f32) -> Aabb {
        Aabb::seg(c, c, r)
    }

    fn grow(self, o: Aabb) -> Aabb {
        Aabb { x0: self.x0.min(o.x0), y0: self.y0.min(o.y0), x1: self.x1.max(o.x1), y1: self.y1.max(o.y1) }
    }

    /// Lower bound on the distance from p to the enclosed shape; exact box SDF inside.
    fn lb(&self, p: P) -> f32 {
        (self.x0 - p.0).max(p.0 - self.x1).max((self.y0 - p.1).max(p.1 - self.y1))
    }

    fn spans_row(&self, lo: f32, hi: f32, care: f32) -> bool {
        self.y0 - care <= hi && self.y1 + care >= lo
    }
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
    if ax > ay * 5.0 {
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

// ── rigs ────────────────────────────────────────────────────────────

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
    Contour,
    Dark,
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
    z: i8,
}

const fn bone(parent: Option<usize>, anchor: f32, off: P, len: f32, rest: f32, r0: f32, r1: f32, part: Part, z: i8) -> BoneDef {
    BoneDef { parent, anchor, off, len, rest, r0, r1, part, z }
}

const UP: f32 = PI;
const RIGHT: f32 = PI / 2.0;
const LEFT: f32 = -PI / 2.0;
const HIDDEN: i8 = -100;

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
    bone(None, 0.0, (0.0, 0.0), 0.24, UP, 0.095, 0.13, Part::Skin, 0),
    bone(Some(SPINE), 1.0, (0.0, 0.0), 0.06, UP, 0.032, 0.032, Part::Skin, 0),
    bone(Some(NECK), 1.0, (0.0, 0.0), 0.11, UP, 0.056, 0.05, Part::Skin, 1),
    bone(Some(SPINE), 0.9, (0.0, 0.0), 0.2, RIGHT, 0.07, 0.06, Part::Skin, 1),
    bone(Some(CLAV_R), 1.0, (0.0, 0.0), 0.17, 0.4, 0.046, 0.046, Part::Skin, 2),
    bone(Some(UPPER_R), 1.0, (0.0, 0.0), 0.2, 0.15, 0.058, 0.058, Part::Wrap, 3),
    bone(Some(FORE_R), 1.0, (0.0, 0.0), 0.08, 0.15, 0.052, 0.045, Part::Skin, 4),
    bone(Some(HAND_R), 1.0, (0.0, 0.0), 0.1, 0.15, 0.015, 0.012, Part::Skin, 4),
    bone(Some(SPINE), 0.9, (0.0, 0.0), 0.2, LEFT, 0.07, 0.06, Part::Skin, 1),
    bone(Some(CLAV_L), 1.0, (0.0, 0.0), 0.17, -0.4, 0.046, 0.046, Part::Skin, 2),
    bone(Some(UPPER_L), 1.0, (0.0, 0.0), 0.2, -0.15, 0.058, 0.058, Part::Wrap, 3),
    bone(Some(FORE_L), 1.0, (0.0, 0.0), 0.08, -0.15, 0.052, 0.045, Part::Skin, 4),
    bone(Some(HAND_L), 1.0, (0.0, 0.0), 0.1, -0.15, 0.015, 0.012, Part::Skin, 4),
    bone(None, 0.0, (0.06, 0.02), 0.16, 0.08, 0.056, 0.05, Part::Skin, 0),
    bone(Some(THIGH_R), 1.0, (0.0, 0.0), 0.14, 0.0, 0.05, 0.045, Part::Skin, 0),
    bone(Some(SHIN_R), 1.0, (0.0, 0.0), 0.06, RIGHT, 0.042, 0.035, Part::Skin, 0),
    bone(None, 0.0, (-0.06, 0.02), 0.16, -0.08, 0.056, 0.05, Part::Skin, 0),
    bone(Some(THIGH_L), 1.0, (0.0, 0.0), 0.14, 0.0, 0.05, 0.045, Part::Skin, 0),
    bone(Some(SHIN_L), 1.0, (0.0, 0.0), 0.06, LEFT, 0.042, 0.035, Part::Skin, 0),
    bone(None, 0.0, (0.0, -0.02), 0.18, 0.0, 0.13, 0.08, Part::Cloth, 1),
    bone(Some(HEAD), 1.0, (0.0, 0.0), 0.14, UP, 0.0, 0.0, Part::Skin, HIDDEN),
];

pub const POSE_NAMES: [&str; 7] = ["stand", "point", "guard", "lunge", "adapt", "swing", "flinch"];

/// Joint deltas (radians, relative to rest) for each Mahoraga keyframe.
fn keyframe(idx: usize) -> Vec<f32> {
    let mut d = vec![0.0f32; NBONES];
    let mut set = |pairs: &[(usize, f32)]| {
        for &(i, v) in pairs {
            d[i] = v;
        }
    };
    match idx % 7 {
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
        6 => set(&[(SPINE, -0.35), (NECK, -0.5), (UPPER_R, 0.6), (FORE_R, 1.2), (UPPER_L, -1.6), (FORE_L, -1.0), (THIGH_R, 0.2), (THIGH_L, -0.5), (SHIN_L, 0.4)]),
        _ => {}
    }
    d
}

// Sukuna: four arms, two per side, lower pair off the mid spine.
const S_SPINE: usize = 0;
const S_NECK: usize = 1;
const S_HEAD: usize = 2;
const S_CLAV_R: usize = 3;
const S_UPPER_R: usize = 4;
const S_FORE_R: usize = 5;
const S_CLAV_L: usize = 6;
const S_UPPER_L: usize = 7;
const S_FORE_L: usize = 8;
const S_UPPER2_R: usize = 9;
const S_FORE2_R: usize = 10;
const S_UPPER2_L: usize = 11;
const S_FORE2_L: usize = 12;
const S_THIGH_R: usize = 13;
const S_SHIN_R: usize = 14;
const S_THIGH_L: usize = 15;
const S_SHIN_L: usize = 16;
const S_NBONES: usize = 17;

static SUKUNA_BONES: [BoneDef; S_NBONES] = [
    bone(None, 0.0, (0.0, 0.0), 0.32, UP, 0.085, 0.12, Part::Dark, 0),
    bone(Some(S_SPINE), 1.0, (0.0, 0.0), 0.05, UP, 0.03, 0.03, Part::Dark, 0),
    bone(Some(S_NECK), 1.0, (0.0, 0.0), 0.13, UP, 0.07, 0.06, Part::Dark, 1),
    bone(Some(S_SPINE), 0.92, (0.0, 0.0), 0.15, RIGHT, 0.06, 0.05, Part::Dark, 1),
    bone(Some(S_CLAV_R), 1.0, (0.0, 0.0), 0.2, 0.55, 0.045, 0.04, Part::Dark, 2),
    bone(Some(S_UPPER_R), 1.0, (0.0, 0.0), 0.22, 0.2, 0.04, 0.03, Part::Dark, 3),
    bone(Some(S_SPINE), 0.92, (0.0, 0.0), 0.15, LEFT, 0.06, 0.05, Part::Dark, 1),
    bone(Some(S_CLAV_L), 1.0, (0.0, 0.0), 0.2, -0.55, 0.045, 0.04, Part::Dark, 2),
    bone(Some(S_UPPER_L), 1.0, (0.0, 0.0), 0.22, -0.2, 0.04, 0.03, Part::Dark, 3),
    bone(Some(S_SPINE), 0.55, (0.08, 0.0), 0.18, 1.1, 0.04, 0.035, Part::Dark, 2),
    bone(Some(S_UPPER2_R), 1.0, (0.0, 0.0), 0.2, 0.3, 0.035, 0.028, Part::Dark, 3),
    bone(Some(S_SPINE), 0.55, (-0.08, 0.0), 0.18, -1.1, 0.04, 0.035, Part::Dark, 2),
    bone(Some(S_UPPER2_L), 1.0, (0.0, 0.0), 0.2, -0.3, 0.035, 0.028, Part::Dark, 3),
    bone(None, 0.0, (0.05, 0.0), 0.2, 0.15, 0.055, 0.045, Part::Dark, 0),
    bone(Some(S_THIGH_R), 1.0, (0.0, 0.0), 0.2, -0.05, 0.045, 0.04, Part::Dark, 0),
    bone(None, 0.0, (-0.05, 0.0), 0.2, -0.15, 0.055, 0.045, Part::Dark, 0),
    bone(Some(S_THIGH_L), 1.0, (0.0, 0.0), 0.2, 0.05, 0.045, 0.04, Part::Dark, 0),
];

const S_ROOT: P = (0.0, 0.55);

/// Sukuna keyframes: stance, slash (lead arm sweeping), point, crouch.
fn sukuna_keyframe(idx: usize) -> Vec<f32> {
    let mut d = vec![0.0f32; S_NBONES];
    let mut set = |pairs: &[(usize, f32)]| {
        for &(i, v) in pairs {
            d[i] = v;
        }
    };
    match idx % 4 {
        1 => set(&[(S_UPPER_R, 2.2), (S_FORE_R, 0.3), (S_UPPER2_R, 1.4), (S_FORE2_R, -0.4), (S_UPPER_L, -0.8), (S_FORE_L, -1.4), (S_SPINE, 0.25), (S_THIGH_R, 0.5), (S_SHIN_R, -0.4), (S_THIGH_L, -0.3)]),
        2 => set(&[(S_UPPER_R, 1.6), (S_FORE_R, 0.0), (S_UPPER2_L, -0.6), (S_FORE2_L, -0.9), (S_SPINE, 0.1)]),
        3 => set(&[(S_SPINE, 0.5), (S_NECK, -0.3), (S_THIGH_R, 1.1), (S_SHIN_R, -1.4), (S_THIGH_L, -0.9), (S_SHIN_L, 1.2), (S_UPPER_R, 0.9), (S_FORE_R, 1.6), (S_UPPER_L, -0.6), (S_UPPER2_R, 0.8), (S_UPPER2_L, -0.9)]),
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

/// Forward kinematics in local space, then placed: p' = root + scale * (mirror * u, v).
fn solve(bones: &[BoneDef], deltas: &[f32], root: P, scale: f32, mirror: f32) -> Vec<Seg> {
    let mut local: Vec<Seg> = Vec::with_capacity(bones.len());
    for (i, bd) in bones.iter().enumerate() {
        let (start, parent_ang, parent_rest) = match bd.parent {
            Some(pi) => {
                let ps = local[pi];
                let s = (ps.a.0 + (ps.b.0 - ps.a.0) * bd.anchor, ps.a.1 + (ps.b.1 - ps.a.1) * bd.anchor);
                (s, ps.ang, bones[pi].rest)
            }
            None => ((0.0, 0.0), 0.0, 0.0),
        };
        let ang = parent_ang + (bd.rest - parent_rest) + deltas[i];
        let a = (start.0 + bd.off.0, start.1 + bd.off.1);
        let d = dir(ang);
        local.push(Seg { a, b: (a.0 + d.0 * bd.len, a.1 + d.1 * bd.len), ang });
    }
    local
        .into_iter()
        .map(|s| Seg { a: (root.0 + s.a.0 * scale * mirror, root.1 + s.a.1 * scale), b: (root.0 + s.b.0 * scale * mirror, root.1 + s.b.1 * scale), ang: s.ang })
        .collect()
}

/// Two-bone IK on one Mahoraga arm chain so the finger tip reaches toward `target`.
fn aim_arm(deltas: &mut [f32], target: P, chain: [usize; 5], bend_sign: f32) {
    let [clav_i, upper_i, fore_i, hand_i, finger_i] = chain;
    let segs = solve(&BONES, deltas, ROOT, 1.0, 1.0);
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
fn aim_at(deltas: &mut [f32], target: P) {
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

fn bone_normal(p: P, s: &Seg, r0: f32, r1: f32) -> P {
    let e = 0.003;
    let dx = sd_bone((p.0 + e, p.1), s, r0, r1) - sd_bone((p.0 - e, p.1), s, r0, r1);
    let dy = sd_bone((p.0, p.1 + e), s, r0, r1) - sd_bone((p.0, p.1 - e), s, r0, r1);
    let l = len((dx, dy)).max(1e-6);
    (dx / l, dy / l)
}

/// A posed rig: solved segments over a bone table, each with its bound.
struct Body {
    bones: &'static [BoneDef],
    segs: Vec<Seg>,
    boxes: Vec<Aabb>,
}

struct Hit {
    d: f32,
    part: Part,
    bone: usize,
    over: bool,
}

impl Body {
    fn new(bones: &'static [BoneDef], segs: Vec<Seg>) -> Body {
        let boxes = bones.iter().zip(segs.iter()).map(|(bd, s)| Aabb::seg(s.a, s.b, bd.r0.max(bd.r1))).collect();
        Body { bones, segs, boxes }
    }

    /// Bones whose bound can reach a tile of cells, plus their union box.
    fn tile_mask(&self, t: Tile, care: f32, out: &mut Vec<u16>) -> Aabb {
        out.clear();
        let mut span = Aabb::EMPTY;
        for (i, bd) in self.bones.iter().enumerate() {
            if bd.z != HIDDEN && t.holds(&self.boxes[i], care) {
                out.push(i as u16);
                span = span.grow(self.boxes[i]);
            }
        }
        span
    }

    /// Nearest-in-depth bone containing p, else the closest bone surface.
    /// Bones further than `care` are skipped: the caller only reads d below it.
    fn sample(&self, p: P, contour_w: f32, care: f32, rows: RowSlice) -> Hit {
        let mut nearest = Hit { d: f32::MAX, part: Part::Skin, bone: 0, over: false };
        if rows.span.lb(p) >= care {
            return nearest;
        }
        let mut inside: Option<(f32, usize)> = None;
        let mut min_inside_z = i8::MAX;
        let mut inside_count = 0;
        for &bi in rows.mask {
            let i = bi as usize;
            let bd = &self.bones[i];
            if bd.z == HIDDEN || self.boxes[i].lb(p) >= care {
                continue;
            }
            let d = sd_bone(p, &self.segs[i], bd.r0, bd.r1);
            if d < nearest.d {
                nearest = Hit { d, part: bd.part, bone: i, over: false };
            }
            if d < 0.0 {
                inside_count += 1;
                min_inside_z = min_inside_z.min(bd.z);
                let better = match inside {
                    None => true,
                    Some((bd0, i0)) => bd.z > self.bones[i0].z || (bd.z == self.bones[i0].z && d < bd0),
                };
                if better {
                    inside = Some((d, i));
                }
            }
        }
        if let Some((d, i)) = inside {
            let over = inside_count > 1 && self.bones[i].z > min_inside_z && -d < contour_w;
            let part = if over { Part::Contour } else { self.bones[i].part };
            return Hit { d, part, bone: i, over };
        }
        nearest
    }

    fn normal(&self, p: P, bone: usize) -> P {
        let bd = &self.bones[bone];
        bone_normal(p, &self.segs[bone], bd.r0, bd.r1)
    }
}

/// Mahoraga: body rig plus the wheel riding the head locator.
struct Figure {
    lean: f32,
    lit: usize,
    body: Body,
    wheel_c: P,
    spokes: [(P, P); 8],
    wheel_box: Aabb,
}

impl Figure {
    fn new(rot: f32, lean: f32, lit: usize, deltas: &[f32]) -> Self {
        let segs = solve(&BONES, deltas, ROOT, 1.0, 1.0);
        let c = segs[WHEEL].b;
        let mut spokes = [((0.0, 0.0), (0.0, 0.0)); 8];
        for (k, sp) in spokes.iter_mut().enumerate() {
            let a = rot + k as f32 * TAU / 8.0;
            let d = (a.cos(), a.sin());
            *sp = ((c.0 + d.0 * WHEEL_R * 0.25, c.1 + d.1 * WHEEL_R * 0.25), (c.0 + d.0 * WHEEL_R * 1.32, c.1 + d.1 * WHEEL_R * 1.32));
        }
        let wheel_box = Aabb::disc(c, WHEEL_R * 1.32 + 0.02);
        Figure { lean, lit, body: Body::new(&BONES, segs), wheel_c: c, spokes, wheel_box }
    }

    fn warp(&self, p: P) -> P {
        (p.0 - self.lean * (0.62 - p.1), p.1)
    }

    /// Widest sideways offset `warp` can apply inside a row band.
    fn warp_shift(&self, lo: f32, hi: f32) -> f32 {
        (self.lean * (0.62 - lo)).abs().max((self.lean * (0.62 - hi)).abs())
    }

    fn face(&self, p: P, best: &mut Hit) {
        let head = self.body.segs[HEAD];
        let hd = dir(head.ang);
        let side = (hd.1, -hd.0);
        let eye_c = (head.a.0 + hd.0 * 0.055, head.a.1 + hd.1 * 0.055);
        for s in [-1.0f32, 1.0] {
            let c = (eye_c.0 + side.0 * 0.022 * s, eye_c.1 + side.1 * 0.022 * s);
            let d = sd_circle(p, c, 0.011);
            if d < best.d {
                *best = Hit { d, part: Part::Eye, bone: HEAD, over: false };
            }
        }
        let m = (head.a.0 + hd.0 * 0.02, head.a.1 + hd.1 * 0.02);
        let d = sd_seg(p, (m.0 - side.0 * 0.024, m.1 - side.1 * 0.024), (m.0 + side.0 * 0.024, m.1 + side.1 * 0.024), 0.006);
        if d < best.d {
            *best = Hit { d, part: Part::Mouth, bone: HEAD, over: false };
        }
    }

    fn wheel(&self, p: P, care: f32) -> Hit {
        let far = self.wheel_box.lb(p);
        if far >= care {
            return Hit { d: far, part: Part::Ring, bone: WHEEL, over: false };
        }
        let c = self.wheel_c;
        let mut best = Hit { d: (len(sub(p, c)) - WHEEL_R).abs() - 0.014, part: Part::Ring, bone: WHEEL, over: false };
        let hub = sd_circle(p, c, 0.009);
        if hub < best.d {
            best = Hit { d: hub, part: Part::Hub, bone: WHEEL, over: false };
        }
        for (k, &(inner, outer)) in self.spokes.iter().enumerate() {
            let sp = sd_seg(p, inner, outer, 0.011);
            if sp < best.d {
                best = Hit { d: sp, part: Part::Spoke, bone: WHEEL, over: false };
            }
            let h = sd_circle(p, outer, 0.02);
            if h < best.d {
                best = Hit { d: h, part: Part::Handle(k), bone: WHEEL, over: false };
            }
        }
        best
    }

    /// Wheel in front when inside it, else the body (face features win inside the head).
    fn sample(&self, p: P, contour_w: f32, care: f32, rows: RowSlice) -> Hit {
        let p = self.warp(p);
        let w = self.wheel(p, care);
        if w.d < 0.0 {
            return w;
        }
        let mut b = self.body.sample(p, contour_w, care, rows);
        if b.d < 0.0 && b.bone == HEAD {
            self.face(p, &mut b);
        }
        if w.d < b.d { w } else { b }
    }

    fn normal(&self, p: P, bone: usize) -> P {
        self.body.normal(self.warp(p), bone)
    }
}

// ── posing ──────────────────────────────────────────────────────────

const CYCLE: [usize; 7] = [0, 4, 6, 2, 1, 3, 5];

/// Which two Mahoraga keyframes and how far between them, for this frame.
fn keyframe_pair(t: f32, knobs: &ShrineKnobs) -> (usize, usize, f32) {
    if t > 0.0 {
        let seg = t * knobs.speed / knobs.hold.max(0.1);
        let i = seg.floor() as usize;
        (CYCLE[i % 7], CYCLE[(i + 1) % 7], smoothstep(0.5, 1.0, seg.fract()))
    } else {
        (knobs.pose_a as usize, knobs.pose_b as usize, knobs.blend)
    }
}

/// Blend keyframes, add seed noise and sway, then aim the arm when pointing.
fn pose_deltas(seed: u64, t: f32, knobs: &ShrineKnobs, aim: Option<P>) -> Vec<f32> {
    let (ia, ib, w) = keyframe_pair(t, knobs);
    let mut a = keyframe(ia);
    let mut b = keyframe(ib);
    if let Some(target) = aim {
        if ia == 1 {
            aim_at(&mut a, target);
        }
        if ib == 1 {
            aim_at(&mut b, target);
        }
    }
    let mut d = vec![0.0f32; NBONES];
    let mut rng = side_rng(seed, 7);
    for i in 0..NBONES {
        d[i] = a[i] + (b[i] - a[i]) * w;
        let n: f32 = rng.random();
        let freq: f32 = rng.random_range(0.6..1.7);
        let phase: f32 = rng.random_range(0.0..TAU);
        if i != WHEEL {
            d[i] += (n - 0.5) * 2.0 * knobs.noise;
            if t > 0.0 {
                d[i] += (t * freq + phase).sin() * knobs.jitter;
            }
        }
    }
    if t > 0.0 {
        d[SPINE] += (t * 1.3).sin() * knobs.breath;
        d[NECK] -= (t * 1.3).sin() * knobs.breath * 0.6;
    }
    d
}

/// Sukuna's pose: static knob, or slash on every adaptation beat while animating.
fn sukuna_deltas(seed: u64, t: f32, frac: f32, knobs: &ShrineKnobs) -> Vec<f32> {
    let (ia, ib, w) = if t > 0.0 {
        let swing = smoothstep(0.0, 0.25, frac) * (1.0 - smoothstep(0.35, 0.7, frac));
        (0usize, 1usize, swing)
    } else {
        (knobs.sukpose as usize, knobs.sukpose as usize, 0.0)
    };
    let a = sukuna_keyframe(ia);
    let b = sukuna_keyframe(ib);
    let mut rng = side_rng(seed, 8);
    (0..S_NBONES)
        .map(|i| {
            let n: f32 = rng.random();
            let freq: f32 = rng.random_range(0.8..2.0);
            let phase: f32 = rng.random_range(0.0..TAU);
            let mut v = a[i] + (b[i] - a[i]) * w + (n - 0.5) * knobs.noise;
            if t > 0.0 {
                v += (t * freq + phase).sin() * knobs.jitter * 0.6;
            }
            v
        })
        .collect()
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

#[derive(Clone, Copy)]
enum ShrinePrim {
    Slab { c: P, hx: f32, hy: f32 },
    Beam { a: P, b: P, r: f32 },
    Shell { c: P, r: f32, w: f32 },
}

impl ShrinePrim {
    fn sd(&self, p: P) -> f32 {
        match *self {
            ShrinePrim::Slab { c, hx, hy } => sd_box(p, c, hx, hy),
            ShrinePrim::Beam { a, b, r } => sd_seg(p, a, b, r),
            ShrinePrim::Shell { c, r, w } => sd_circle(p, c, r).abs() - w,
        }
    }

    fn bound(&self) -> Aabb {
        match *self {
            ShrinePrim::Slab { c, hx, hy } => Aabb { x0: c.0 - hx - SLACK, y0: c.1 - hy - SLACK, x1: c.0 + hx + SLACK, y1: c.1 + hy + SLACK },
            ShrinePrim::Beam { a, b, r } => Aabb::seg(a, b, r),
            ShrinePrim::Shell { c, r, w } => Aabb::disc(c, r + w),
        }
    }
}

/// Malevolent Shrine: eaves, ridge, skull with horns, pillars. Figure-space,
/// built once per frame so the per-cell path only walks the row's primitives.
struct ShrineGeo {
    prims: Vec<(ShrinePrim, ShrinePart, Aabb)>,
    half_w: f32,
}

fn make_shrine(presence: f32) -> Option<ShrineGeo> {
    if presence <= 0.0 {
        return None;
    }
    let w = 0.75 + presence * 0.5;
    let mut prims: Vec<(ShrinePrim, ShrinePart)> = vec![
        (ShrinePrim::Slab { c: (0.0, -0.02), hx: w * 0.55, hy: 0.012 }, ShrinePart::Roof),
        (ShrinePrim::Slab { c: (0.0, 0.05), hx: w * 0.55, hy: 0.012 }, ShrinePart::Roof),
        (ShrinePrim::Beam { a: (w * 0.5, 0.06), b: (w, 0.0), r: 0.014 }, ShrinePart::Eave),
        (ShrinePrim::Beam { a: (-w * 0.5, 0.06), b: (-w, 0.0), r: 0.014 }, ShrinePart::Eave),
        (ShrinePrim::Shell { c: (0.0, -0.12), r: 0.07, w: 0.012 }, ShrinePart::Skull),
        (ShrinePrim::Beam { a: (0.05, -0.16), b: (0.16, -0.32), r: 0.014 }, ShrinePart::Horn),
        (ShrinePrim::Beam { a: (-0.05, -0.16), b: (-0.16, -0.32), r: 0.014 }, ShrinePart::Horn),
    ];
    for k in 0..5 {
        let x = (k as f32 - 2.0) * w * 0.25;
        prims.push((ShrinePrim::Slab { c: (x, 0.45), hx: 0.012, hy: 0.4 }, ShrinePart::Pillar));
    }
    Some(ShrineGeo { prims: prims.into_iter().map(|(pr, part)| (pr, part, pr.bound())).collect(), half_w: w * 0.55 })
}

fn sd_shrine(geo: &ShrineGeo, p: P, mask: &[u16]) -> Option<ShrinePart> {
    let mut best: Option<(f32, ShrinePart)> = None;
    for &i in mask {
        let (prim, part, bx) = &geo.prims[i as usize];
        if bx.lb(p) >= 0.0 {
            continue;
        }
        let d = prim.sd(p);
        if d < 0.0 && best.map_or(true, |b| d < b.0) {
            best = Some((d, *part));
        }
    }
    if best.is_none() && p.0.abs() < geo.half_w && p.1 > 0.05 && p.1 < 0.85 {
        return Some(ShrinePart::Wall);
    }
    best.map(|b| b.1)
}

// ── the cuts ────────────────────────────────────────────────────────
// ── the cuts ────────────────────────────────────────────────────────

struct Slash {
    q: P,
    d: P,
    n: P,
    half: f32,
    slip: f32,
    bright: f32,
}

/// Cuts leave Sukuna's lead hand and aim into Mahoraga's body; with no hand
/// they fall as the older random field.
fn make_slashes(seed: u64, count: usize, cut_uv: f32, u_span: f32, v_span: (f32, f32), origin: Option<P>) -> Vec<Slash> {
    let mut rng = side_rng(seed, 1);
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let slip = cut_uv * rng.random_range(0.5..1.4) * if rng.random::<bool>() { 1.0 } else { -1.0 };
        let bright: f32 = rng.random();
        if let Some(o) = origin {
            let start = (o.0 + rng.random_range(-0.06..0.06), o.1 + rng.random_range(-0.08..0.04));
            let goal = (rng.random_range(-0.28..0.32), rng.random_range(0.08..0.62));
            let to = sub(goal, start);
            let l = len(to).max(0.1);
            let d = (to.0 / l, to.1 / l);
            let half = rng.random_range(0.75..1.25) * l * 0.5 + 0.1;
            let q = (start.0 + d.0 * half, start.1 + d.1 * half);
            out.push(Slash { q, d, n: (-d.1, d.0), half, slip, bright });
            continue;
        }
        let family: f32 = if i % 3 == 2 { -1.0 } else { 1.0 };
        let a: f32 = family * rng.random_range(0.55..0.95) + rng.random_range(-0.06..0.06);
        let d = (a.cos(), a.sin());
        let qu = rng.random_range(-u_span * 0.7..u_span * 0.7) * rng.random::<f32>().sqrt();
        let qv = rng.random_range(v_span.0 * 0.6..v_span.1 * 0.95);
        let half = rng.random_range(0.35..0.75) * u_span.max(0.8);
        out.push(Slash { q: (qu, qv), d, n: (-d.1, d.0), half, slip, bright });
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

fn sample_city<'a>(city: &'a [Building], rows: &[u16], p: P) -> Option<&'a Building> {
    rows.iter().map(|&i| &city[i as usize]).find(|b| p.0 >= b.u0 && p.0 <= b.u1 && p.1 >= b.top && p.1 <= b.base)
}

// ── the clock ───────────────────────────────────────────────────────

/// t=0 shows `turns` adaptations; t>0 replays the fight on a loop.
fn progress(t: f32, knobs: &ShrineKnobs) -> f32 {
    if t <= 0.0 { knobs.turns } else { (t * knobs.speed).rem_euclid(knobs.fuga as f32 + 3.0) }
}

// ── render ──────────────────────────────────────────────────────────

/// Column tile width for the per-tile primitive masks.
const TILE: usize = 64;

const SKIN_RAMP: [char; 10] = [' ', '.', '·', ':', '-', '=', '+', '*', '%', '@'];
const CLOTH_RAMP: [char; 6] = [' ', '.', '-', '~', '=', '#'];
const DARK_RAMP: [char; 5] = ['@', '#', '%', '*', '+'];

struct Scene<'a> {
    fig: Figure,
    ghosts: Vec<Figure>,
    sukuna: Option<Body>,
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
    u_span: f32,
    t: f32,
    frac: f32,
    lunge: f32,
    fuga: Option<Fuga>,
    shrine: Option<ShrineGeo>,
    contour_w: f32,
    edge_eps: f32,
}

/// Fire Arrow beam with its endpoints and reach bound resolved once per frame.
struct Fuga {
    o: P,
    tip: P,
    reach: f32,
    bx: Aabb,
}

/// Cell band one mask is built for: a row's worth of v, a tile's worth of u.
#[derive(Clone, Copy)]
struct Tile {
    lo: f32,
    hi: f32,
    ulo: f32,
    uhi: f32,
}

impl Tile {
    fn holds(&self, bx: &Aabb, care: f32) -> bool {
        bx.spans_row(self.lo, self.hi, care) && bx.x0 - care <= self.uhi && bx.x1 + care >= self.ulo
    }
}

/// One row's live primitive indices: everything else is out of reach of the row.
#[derive(Clone, Copy)]
struct RowSlice<'a> {
    mask: &'a [u16],
    span: Aabb,
}

struct RowCtx {
    fig: (Vec<u16>, Aabb),
    ghosts: Vec<(Vec<u16>, Aabb)>,
    sukuna: (Vec<u16>, Aabb),
    city: Vec<u16>,
    shrine: Vec<u16>,
}

impl RowCtx {
    fn new(sc: &Scene) -> RowCtx {
        RowCtx {
            fig: (Vec::with_capacity(NBONES), Aabb::EMPTY),
            ghosts: sc.ghosts.iter().map(|_| (Vec::with_capacity(NBONES), Aabb::EMPTY)).collect(),
            sukuna: (Vec::with_capacity(S_NBONES), Aabb::EMPTY),
            city: Vec::with_capacity(sc.city.len()),
            shrine: Vec::with_capacity(sc.shrine.as_ref().map_or(0, |g| g.prims.len())),
        }
    }

    fn rebuild(&mut self, sc: &Scene, t: Tile) {
        self.fig.1 = sc.fig.body.tile_mask(t, sc.edge_eps, &mut self.fig.0);
        for (slot, ghost) in self.ghosts.iter_mut().zip(sc.ghosts.iter()) {
            slot.1 = ghost.body.tile_mask(t, 0.0, &mut slot.0);
        }
        if let Some(body) = &sc.sukuna {
            self.sukuna.1 = body.tile_mask(t, 0.0, &mut self.sukuna.0);
        }
        self.city.clear();
        for (i, b) in sc.city.iter().enumerate() {
            if b.top <= t.hi && b.base >= t.lo && b.u0 <= t.uhi && b.u1 >= t.ulo {
                self.city.push(i as u16);
            }
        }
        self.shrine.clear();
        if let Some(geo) = &sc.shrine {
            for (i, (_, _, bx)) in geo.prims.iter().enumerate() {
                if t.holds(bx, 0.0) {
                    self.shrine.push(i as u16);
                }
            }
        }
    }

    fn slice(pair: &(Vec<u16>, Aabb)) -> RowSlice<'_> {
        RowSlice { mask: &pair.0, span: pair.1 }
    }
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
    fire_core: Color,
    fire_mid: Color,
    fire_out: Color,
}

fn ramp_glyph(ramp: &[char], shade: f32) -> char {
    ramp[((shade.clamp(0.0, 0.999)) * ramp.len() as f32) as usize]
}

/// Skin: density ramp, with cross-contour rings wrapping each bone in the mid-tones.
fn skin_glyph(sc: &Scene, shade: f32, hit: &Hit, pu: P, noise: f32) -> char {
    let k = sc.knobs;
    if k.hatch > 0.0 && shade > 0.28 && shade < 0.7 {
        let seg = &sc.fig.body.segs[hit.bone];
        let p = sc.fig.warp(pu);
        let ba = sub(seg.b, seg.a);
        let along = dot(sub(p, seg.a), ba) / len(ba).max(1e-6);
        let phase = (along * sc.fig_h / 3.0).rem_euclid(1.0);
        if phase < 0.28 && noise + 0.5 < k.hatch + 0.2 {
            return stroke_glyph(ba.1 * 2.0, -ba.0);
        }
    }
    ramp_glyph(&SKIN_RAMP, shade)
}

fn figure_cell(sc: &Scene, ink: &Ink, hit: &Hit, pu: P, noise: f32) -> (char, Color) {
    let k = sc.knobs;
    match hit.part {
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
        Part::Contour => {
            let n = sc.fig.normal(pu, hit.bone);
            (stroke_glyph(-n.1 * 2.0, n.0), darken(ink.bone, 30))
        }
        _ => {
            let n = sc.fig.normal(pu, hit.bone);
            let lit = 0.5 + 0.5 * dot(n, sc.light);
            let rim = smoothstep(0.0, 0.05, -hit.d);
            let shade = (lit * 0.75 + (1.0 - rim) * 0.35 + noise * k.grain).clamp(0.0, 0.999);
            match hit.part {
                Part::Cloth => (ramp_glyph(&CLOTH_RAMP, shade), lerp_color(darken(ink.cloth, 60), ink.cloth, shade)),
                Part::Wrap => {
                    let band = ((pu.1 * sc.fig_h * 0.9) as i32).rem_euclid(3);
                    let ch = if band == 0 { '=' } else if shade > 0.5 { '-' } else { '.' };
                    (ch, lerp_color(darken(ink.wrap, 50), ink.wrap, shade))
                }
                _ => (skin_glyph(sc, shade, hit, pu, noise), lerp_color(ink.bone, ink.pale, shade)),
            }
        }
    }
}

/// Sukuna: a dark cutout with a lit rim and contour lines where limbs cross.
fn sukuna_cell(sc: &Scene, ink: &Ink, body: &Body, rows: RowSlice, pu: P, noise: f32) -> Option<(char, Color)> {
    let rim = 0.45 / sc.fig_h;
    let hit = body.sample(pu, rim * 0.8, 0.0, rows);
    if hit.d >= 0.0 {
        return None;
    }
    let n = body.normal(pu, hit.bone);
    if hit.part == Part::Contour {
        return Some((stroke_glyph(-n.1 * 2.0, n.0), lerp_color(ink.sukuna, ink.cut, 0.35)));
    }
    if hit.d < -rim {
        let _ = noise;
        return Some((' ', darken(ink.sukuna, 60)));
    }
    let lit = 0.5 + 0.5 * dot(n, sc.light);
    Some((stroke_glyph(-n.1 * 2.0, n.0), lerp_color(ink.sukuna, ink.cut, lit * 0.8)))
}

/// Fire Arrow as a tapered beam sampled per cell: core, fringe, flicker halo.
fn fuga_cell(sc: &Scene, ink: &Ink, pu: P, x: usize, y: usize) -> Option<(char, Color)> {
    let f = sc.fuga.as_ref()?;
    if f.bx.lb(pu) >= 0.0 {
        return None;
    }
    let (o, tip, reach) = (f.o, f.tip, f.reach);
    let ba = sub(tip, o);
    let pa = sub(pu, o);
    let h = (dot(pa, ba) / dot(ba, ba).max(1e-6)).clamp(0.0, 1.0);
    let across = len((pa.0 - ba.0 * h, pa.1 - ba.1 * h));
    let taper = 1.0 - h * 0.45;
    let flick = hash01(sc.seed ^ 0xF17E, x as i64 + (sc.t * 9.0) as i64, y as i64);
    let core = 0.022 * taper;
    let mid = 0.05 * taper;
    let out = 0.085 * taper;
    let d_tip = len(sub(pu, tip));
    if reach >= 1.0 && d_tip < 0.2 {
        let ring = (d_tip - 0.13 - (sc.t.max(0.0) * 0.3).fract() * 0.05).abs() < 0.012;
        if ring && flick < 0.7 {
            return Some(('*', ink.fire_out));
        }
        if d_tip < 0.08 && flick < 0.6 {
            return Some((if flick < 0.2 { '+' } else { '*' }, if flick < 0.3 { ink.fire_core } else { ink.fire_mid }));
        }
    }
    if across < core {
        return Some(('=', ink.fire_core));
    }
    if across < mid {
        return Some((if flick < 0.35 { '~' } else { '-' }, ink.fire_mid));
    }
    if across < out && flick < 0.4 - (across - mid) / (out - mid) * 0.3 {
        return Some(('·', ink.fire_out));
    }
    None
}

/// One cell of the scene, front to back. None = leave blank.
fn shade_cell(sc: &Scene, ink: &Ink, row: &RowCtx, x: usize, y: usize, p0: P) -> Option<(char, Color)> {
    let k = sc.knobs;
    let (pu, hit) = displace(p0, &sc.slashes, sc.live, sc.blade);
    if let Some(c) = fuga_cell(sc, ink, pu, x, y) {
        return Some(c);
    }
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

    if let Some(body) = &sc.sukuna {
        if let Some(c) = sukuna_cell(sc, ink, body, RowCtx::slice(&row.sukuna), pu, noise) {
            return Some(c);
        }
    }

    let contour_w = sc.contour_w;
    let fh = sc.fig.sample(pu, contour_w, sc.edge_eps, RowCtx::slice(&row.fig));
    if fh.d < 0.0 {
        return Some(figure_cell(sc, ink, &fh, pu, noise));
    }
    let edge_eps = sc.edge_eps;
    if fh.d < edge_eps && matches!(fh.part, Part::Skin | Part::Cloth | Part::Wrap) {
        let n = sc.fig.normal(pu, fh.bone);
        if n.0.abs() > 0.45 {
            return Some((stroke_glyph(-n.1 * 2.0, n.0), lerp_color(ink.bone, ink.pale, 0.6)));
        }
    }

    for (g, ghost) in sc.ghosts.iter().enumerate() {
        if noise + 0.5 < 0.55 - g as f32 * 0.15 && ghost.sample(pu, contour_w, 0.0, RowCtx::slice(&row.ghosts[g])).d < 0.0 {
            let ch = if g == 0 { ':' } else { '·' };
            return Some((ch, darken(ink.pale, 90 + g as u8 * 30)));
        }
    }

    if sc.lunge > 0.0 && k.lines > 0.0 {
        let rel = sc.fig.warp(pu);
        if rel.0 < -0.3 && rel.1 > 0.2 && rel.1 < 1.0 {
            let row = hash01(sc.seed ^ 0x11E5, 0, y as i64);
            let gap = hash01(sc.seed ^ 0x11E6, x as i64 / 4, y as i64);
            if row < k.lines * sc.lunge * 0.45 && gap < 0.8 {
                return Some(('─', darken(ink.cut, 70)));
            }
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

    let block = sample_city(&sc.city, &row.city, pu);
    if let Some(b) = block {
        if b.z >= 0.3 {
            return block_cell(sc, ink, b, pu, x, y);
        }
    }
    if let Some(geo) = &sc.shrine {
        if let Some(part) = sd_shrine(geo, sc.fig.warp(pu), &row.shrine) {
            if let Some(c) = shrine_cell(sc, ink, part, pu, noise) {
                return Some(c);
            }
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

pub fn draw_mahoraga5(grid: &mut Grid, width: usize, height: usize, seed: u64, palette: &[Color; 5], rng: &mut StdRng, t: f32, knobs: &ShrineKnobs) {
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
    let sukuna_root = (-u_span * 0.62, v_bot - sukuna_s * 0.45 - 0.02);
    let sukuna = if sukuna_s > 0.0 {
        let d = measure_layer("mahoraga-5", "sukuna_pose", || sukuna_deltas(seed, t, frac, knobs));
        Some(Body::new(&SUKUNA_BONES, solve(&SUKUNA_BONES, &d, sukuna_root, sukuna_s, 1.0)))
    } else {
        None
    };
    let hand = sukuna.as_ref().map(|b| b.segs[S_FORE_R].b);
    let aim = if knobs.aim > 0.5 { hand.or(Some((0.62, 0.3))) } else { None };
    let deltas = measure_layer("mahoraga-5", "pose", || pose_deltas(seed, t, knobs, aim));
    let fig = Figure::new(rot, knobs.lean, adaptations, &deltas);
    let wheel_c = fig.wheel_c;
    let (ia, ib, w) = keyframe_pair(t, knobs);
    let lunge = (if ia == 3 { 1.0 - w } else { 0.0 }) + (if ib == 3 { w } else { 0.0 });
    let ghosts: Vec<Figure> = measure_layer("mahoraga-5", "ghosts", || if t > 0.0 && knobs.ghosts >= 1.0 {
        (1..=knobs.ghosts.round() as usize)
            .map(|g| {
                let tg = (t - g as f32 * 0.22).max(0.001);
                Figure::new(rot, knobs.lean, adaptations, &pose_deltas(seed, tg, knobs, aim))
            })
            .collect()
    } else {
        Vec::new()
    });
    let shake = if t > 0.0 && frac < 0.18 && knobs.shake > 0.0 {
        let amp = knobs.shake * (0.18 - frac) / 0.18 * 0.035;
        ((t * 53.0).sin() * amp, (t * 37.0).cos() * amp * 0.5)
    } else {
        (0.0, 0.0)
    };
    let fuga = if reach > 0.0 {
        let o = (-u_span * 0.92, v_top + 0.12);
        let tip = (o.0 + (wheel_c.0 - o.0) * reach, o.1 + (wheel_c.1 - o.1) * reach);
        Some(Fuga { o, tip, reach, bx: Aabb::seg(o, tip, 0.2) })
    } else {
        None
    };

    let sc = Scene {
        fig,
        ghosts,
        sukuna,
        slashes: measure_layer("mahoraga-5", "slashes", || make_slashes(seed, knobs.slash.round() as usize, knobs.cut / (2.0 * fig_h), u_span, (v_top, v_bot), hand)),
        live,
        blade: 0.26 / fig_h,
        city: measure_layer("mahoraga-5", "city", || make_city(seed, knobs.density.round() as usize, u_span, v_bot, horizon)),
        light: (knobs.light.to_radians().cos(), knobs.light.to_radians().sin()),
        knobs,
        seed,
        fig_h,
        horizon,
        v_top,
        u_span,
        t,
        frac,
        lunge,
        fuga,
        shrine: make_shrine(knobs.shrine),
        contour_w: 0.35 / fig_h,
        edge_eps: 0.3 / fig_h,
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
        fire_core: hsl_to_rgb(48.0, 1.0, 0.72),
        fire_mid: hsl_to_rgb(28.0, 1.0, 0.55),
        fire_out: hsl_to_rgb(8.0, 0.95, 0.42),
    };

    measure_layer("mahoraga-5", "shade", || {
        let slip_max = sc.slashes.iter().take(sc.live).map(|s| s.slip.abs()).sum::<f32>() + 1e-4;
        let mut row = RowCtx::new(&sc);
        for y in 0..height {
            let v = (y as f32 - top) / fig_h + shake.1;
            let (lo, hi) = (v - slip_max, v + slip_max);
            let reach = slip_max + sc.fig.warp_shift(lo, hi) + 0.005;
            for xt in (0..width).step_by(TILE) {
                let xe = (xt + TILE).min(width);
                let ulo = (xt as f32 - cx) / (2.0 * fig_h) + shake.0 - reach;
                let uhi = ((xe - 1) as f32 - cx) / (2.0 * fig_h) + shake.0 + reach;
                row.rebuild(&sc, Tile { lo, hi, ulo, uhi });
                for x in xt..xe {
                    let p0 = ((x as f32 - cx) / (2.0 * fig_h) + shake.0, v);
                    if let Some((ch, fg)) = shade_cell(&sc, &ink, &row, x, y, p0) {
                        set(grid, x as i32, y as i32, ch, fg);
                    }
                }
            }
        }
    });

    measure_layer("mahoraga-5", "ash", || draw_ash(grid, width, height, seed, t, knobs.ash, lighten(palette[3], 10)));
    measure_layer("mahoraga-5", "debris", || draw_debris(grid, &sc, &ink, cx, top));
    measure_layer("mahoraga-5", "vignette", || vignette(grid, width, height, knobs.vignette));
    if reach >= 1.0 {
        let wx = cx + (wheel_c.0 + knobs.lean * (0.62 - wheel_c.1)) * 2.0 * fig_h;
        let wy = top + fig_h * wheel_c.1;
        set(grid, wx.round() as i32, wy.round() as i32, '◉', hsl_to_rgb(48.0, 1.0, 0.8));
    }
    let _ = sc.u_span;
    let _ = sc.frac;
}

pub fn render_mahoraga5_frame(width: usize, height: usize, seed: u64, palette: &[Color; 5], mut rng: StdRng, t: f32, knobs: &ShrineKnobs) -> Grid {
    let mut grid = vec![vec![Cell::blank(); width]; height];
    draw_mahoraga5(&mut grid, width, height, seed, palette, &mut rng, t, knobs);
    grid
}

pub(crate) fn cli_mahoraga5(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
    // mahoraga-5 [turns] [slash] [cut] [focus] [pose_a] [pose_b] [blend] [sukpose] -- positional overrides win over env/defaults
    let mut knobs = ShrineKnobs::from_env();
    let f = |i: usize| args.get(i).and_then(|v| v.parse::<f32>().ok());
    if let Some(v) = f(4) {
        knobs.turns = v.clamp(0.0, 8.0);
    }
    if let Some(v) = f(5) {
        knobs.slash = v.clamp(0.0, 24.0);
    }
    if let Some(v) = f(6) {
        knobs.cut = v.clamp(0.0, 4.0);
    }
    if let Some(v) = f(7) {
        knobs.focus = v.clamp(0.0, 1.0);
    }
    if let Some(v) = f(8) {
        knobs.pose_a = v.clamp(0.0, 6.0) as u8;
    }
    if let Some(v) = f(9) {
        knobs.pose_b = v.clamp(0.0, 6.0) as u8;
    }
    if let Some(v) = f(10) {
        knobs.blend = v.clamp(0.0, 1.0);
    }
    if let Some(v) = f(11) {
        knobs.sukpose = v.clamp(0.0, 3.0) as u8;
    }
    let _ = (term_w, term_h, mode, theme_name);
    draw_mahoraga5(&mut grid, width, height, seed, &palette, &mut rng, t_anim, &knobs);
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
        let g = render_mahoraga5_frame(w, h, seed, &p, StdRng::seed_from_u64(seed), t, &knobs);
        g.iter()
            .map(|row| row.iter().map(|c| c.ch).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn snapshot_mahoraga5_pointing() {
        insta::assert_snapshot!("mahoraga5_80x24", run(80, 24, 42, 0.0, 7.0, 1, 4, 0.0));
    }

    #[test]
    fn snapshot_mahoraga5_flinch_fuga_tall() {
        insta::assert_snapshot!("mahoraga5_100x40_flinch_fuga", run(100, 40, 42, 0.0, 8.0, 6, 3, 0.3));
    }

    #[test]
    fn snapshot_mahoraga5_animated_frame() {
        insta::assert_snapshot!("mahoraga5_100x40_t3", run(100, 40, 42, 3.0, 7.0, 1, 4, 0.0));
    }

    #[test]
    fn depth_order_keeps_crossing_arm_visible() {
        let segs = solve(&BONES, &keyframe(2), ROOT, 1.0, 1.0);
        let body = Body::new(&BONES, segs);
        let mid = body.segs[FORE_R];
        let p = ((mid.a.0 + mid.b.0) * 0.5, (mid.a.1 + mid.b.1) * 0.5);
        let mut mask = Vec::new();
        let span = body.tile_mask(Tile { lo: p.1, hi: p.1, ulo: p.0, uhi: p.0 }, 1.0, &mut mask);
        let hit = body.sample(p, 0.0, 1.0, RowSlice { mask: &mask, span });
        assert_eq!(hit.bone, FORE_R, "guard forearm should win over the torso at {:?}", p);
    }

    #[test]
    fn sukuna_rig_stands_and_slashes() {
        let stance = solve(&SUKUNA_BONES, &sukuna_keyframe(0), S_ROOT, 1.0, 1.0);
        let slash = solve(&SUKUNA_BONES, &sukuna_keyframe(1), S_ROOT, 1.0, 1.0);
        assert!(stance[S_HEAD].b.1 < 0.1, "head above root: {}", stance[S_HEAD].b.1);
        assert!(slash[S_FORE_R].b.1 < stance[S_FORE_R].b.1 - 0.2, "slash raises the lead hand");
    }

    #[test]
    fn ik_reaches_target_either_side() {
        for (target, finger) in [((0.55, 0.3), FINGER_R), ((-0.5, 0.8), FINGER_L)] {
            let mut d = keyframe(1);
            aim_at(&mut d, target);
            let segs = solve(&BONES, &d, ROOT, 1.0, 1.0);
            let miss = len(sub(segs[finger].b, target));
            assert!(miss < 0.03, "finger tip {:?} misses {:?} by {}", segs[finger].b, target, miss);
        }
    }

    #[test]
    fn deterministic_seed_sensitive_and_t_moves() {
        assert_eq!(run(90, 30, 42, 0.0, 7.0, 1, 4, 0.0), run(90, 30, 42, 0.0, 7.0, 1, 4, 0.0));
        assert_ne!(run(90, 30, 42, 0.0, 7.0, 1, 4, 0.0), run(90, 30, 7, 0.0, 7.0, 1, 4, 0.0));
        assert_ne!(run(90, 30, 42, 1.0, 7.0, 1, 4, 0.0), run(90, 30, 42, 3.0, 7.0, 1, 4, 0.0));
    }

    #[test]
    fn figure_and_arrow_present() {
        let s = run(100, 40, 42, 0.0, 8.0, 6, 3, 0.3);
        assert!(s.contains('◉'), "hub");
        assert!(s.contains('◆'), "handles");
        assert!(s.contains("====="), "fire arrow core");
        assert!(!run(80, 24, 42, 0.0, 3.0, 1, 4, 0.0).contains("======="), "no arrow before fuga");
    }
}
