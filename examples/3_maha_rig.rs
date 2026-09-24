//! Mahoraga rig preview: linked joint matrices, smooth-blended round cones, wire + contour SVG + ASCII.
//! cargo run --release --example 3_maha_rig -- [skin [voxel]|viewer [voxel]|html|probe|all|key 0-4] [yaw] [w] [h]

use glam::{Mat4, Quat, Vec3};

struct Joint {
    name: &'static str,
    parent: Option<usize>,
    offset: Vec3,
    radius: f32,
}

const fn j(name: &'static str, parent: Option<usize>, x: f32, y: f32, z: f32, radius: f32) -> Joint {
    Joint { name, parent, offset: Vec3::new(x, y, z), radius }
}

// Units: feet on y=0, crown near y=11.3, ~9 heads tall (refs), shoulders ~2x the waist.
// Figure faces +z (toward camera), so the character's right side sits at -x (screen left).
const RIG: &[Joint] = &[
    j("pelvis", None, 0.0, 6.35, 0.0, 0.0),
    j("spine", Some(0), 0.0, 1.8, 0.0, 0.7),
    j("chest", Some(1), 0.0, 1.6, 0.1, 0.9),
    j("neck", Some(2), 0.0, 1.3, 0.32, 0.5),
    j("head", Some(3), 0.0, 0.62, 0.2, 0.42),
    j("crown", Some(4), 0.0, 1.05, -0.25, 0.14),
    j("r_shoulder", Some(2), -1.95, 0.25, -0.05, 0.45),
    j("r_elbow", Some(6), 0.0, -2.4, 0.0, 0.4),
    j("r_wrist", Some(7), 0.0, -2.0, 0.0, 0.3),
    j("r_fist", Some(8), 0.0, -0.7, 0.0, 0.38),
    j("l_shoulder", Some(2), 1.95, 0.25, -0.05, 0.45),
    j("l_elbow", Some(10), 0.0, -2.4, 0.0, 0.4),
    j("l_wrist", Some(11), 0.0, -2.0, 0.0, 0.3),
    j("l_fist", Some(12), 0.0, -0.7, 0.0, 0.38),
    j("r_hip", Some(0), -0.6, -0.3, 0.0, 0.7),
    j("r_knee", Some(14), 0.0, -2.82, 0.0, 0.62),
    j("r_ankle", Some(15), 0.0, -2.83, 0.0, 0.34),
    j("r_toe", Some(16), 0.0, -0.3, 1.05, 0.22),
    j("l_hip", Some(0), 0.6, -0.3, 0.0, 0.7),
    j("l_knee", Some(18), 0.0, -2.82, 0.0, 0.62),
    j("l_ankle", Some(19), 0.0, -2.83, 0.0, 0.34),
    j("l_toe", Some(20), 0.0, -0.3, 1.05, 0.22),
    // Four brow wings: an upper pair sweeping up-out, a lower pair nearly level.
    j("r_wing_hi", Some(4), -0.14, 0.2, 0.5, 0.11),
    j("r_wing_hi_mid", Some(22), -0.55, 0.5, -0.25, 0.1),
    j("r_wing_hi_tip", Some(23), -0.7, 0.55, -0.3, 0.06),
    j("l_wing_hi", Some(4), 0.14, 0.2, 0.5, 0.11),
    j("l_wing_hi_mid", Some(25), 0.55, 0.5, -0.25, 0.1),
    j("l_wing_hi_tip", Some(26), 0.7, 0.55, -0.3, 0.06),
    j("r_wing_lo", Some(4), -0.16, 0.08, 0.48, 0.11),
    j("r_wing_lo_mid", Some(28), -0.7, 0.12, -0.3, 0.1),
    j("r_wing_lo_tip", Some(29), -0.85, 0.05, -0.35, 0.06),
    j("l_wing_lo", Some(4), 0.16, 0.08, 0.48, 0.11),
    j("l_wing_lo_mid", Some(31), 0.7, 0.12, -0.3, 0.1),
    j("l_wing_lo_tip", Some(32), 0.85, 0.05, -0.35, 0.06),
];
const CROWN: usize = 5;
// Head-tail spine in the head frame: up off the skull, out behind, drooping past the left shoulder.
const TAIL: [(f32, f32, f32); 8] = [
    (0.0, 0.55, -0.3), (0.05, 0.95, -0.95), (0.12, 1.0, -1.75), (0.22, 0.6, -2.5),
    (0.32, -0.05, -2.95), (0.42, -0.75, -3.05), (0.5, -1.3, -2.85), (0.52, -1.55, -2.5),
];
const PELVIS: usize = 0;
const CHEST: usize = 2;
const HEAD: usize = 4;
const R_SHOULDER: usize = 6;
const L_SHOULDER: usize = 10;
const L_ELBOW: usize = 11;
const R_ELBOW: usize = 7;

// Degrees. pitch > 0 swings the child chain toward camera, spread > 0 away from the body.
#[derive(Clone, Copy, Default)]
struct Rot {
    pitch: f32,
    spread: f32,
    yaw: f32,
}

struct Key {
    name: &'static str,
    bob: f32,
    rots: &'static [(&'static str, Rot)],
}

const fn r(pitch: f32, spread: f32) -> Rot {
    Rot { pitch, spread, yaw: 0.0 }
}

const fn tw(yaw: f32) -> Rot {
    Rot { pitch: 0.0, spread: 0.0, yaw }
}

// Walk toward camera: chest counter-twists against the pelvis, arms swing opposite the legs.
const KEYS: &[Key] = &[
    // Arms hang straight down from shoulders broad enough to clear the lats; elbows soft and a touch forward.
    Key { name: "K0 stand", bob: 0.0, rots: &[
        ("r_shoulder", r(4.0, 3.0)), ("l_shoulder", r(4.0, 3.0)),
        ("r_elbow", r(14.0, 0.0)), ("l_elbow", r(14.0, 0.0)),
        // wrists go slack: hands drop in toward the thigh and curl slightly forward
        ("r_wrist", r(10.0, -14.0)), ("l_wrist", r(10.0, -14.0)),
        ("r_hip", r(0.0, 3.0)), ("l_hip", r(0.0, 3.0)),
    ]},
    Key { name: "K1 R foot plants", bob: -0.25, rots: &[
        ("pelvis", tw(8.0)), ("spine", tw(-8.0)), ("chest", tw(-10.0)),
        ("r_shoulder", r(-30.0, 15.0)), ("l_shoulder", r(32.0, 13.0)),
        ("r_elbow", r(6.0, 0.0)), ("l_elbow", r(35.0, 0.0)),
        ("r_hip", r(24.0, 3.0)), ("r_knee", r(-4.0, 0.0)),
        ("l_hip", r(-18.0, 3.0)), ("l_knee", r(-24.0, 0.0)),
    ]},
    Key { name: "K2 passing", bob: 0.2, rots: &[
        ("pelvis", tw(-2.0)), ("chest", tw(3.0)),
        ("r_shoulder", r(-4.0, 16.0)), ("l_shoulder", r(6.0, 15.0)),
        ("r_elbow", r(12.0, 0.0)), ("l_elbow", r(18.0, 0.0)),
        ("r_hip", r(-4.0, 3.0)), ("r_knee", r(-4.0, 0.0)),
        ("l_hip", r(18.0, 3.0)), ("l_knee", r(-50.0, 0.0)),
    ]},
    Key { name: "K3 L foot plants", bob: -0.25, rots: &[
        ("pelvis", tw(-8.0)), ("spine", tw(8.0)), ("chest", tw(10.0)),
        ("l_shoulder", r(-30.0, 15.0)), ("r_shoulder", r(32.0, 13.0)),
        ("l_elbow", r(6.0, 0.0)), ("r_elbow", r(35.0, 0.0)),
        ("l_hip", r(24.0, 3.0)), ("l_knee", r(-4.0, 0.0)),
        ("r_hip", r(-18.0, 3.0)), ("r_knee", r(-24.0, 0.0)),
    ]},
    Key { name: "K4 passing", bob: 0.2, rots: &[
        ("pelvis", tw(2.0)), ("chest", tw(-3.0)),
        ("l_shoulder", r(-4.0, 16.0)), ("r_shoulder", r(6.0, 15.0)),
        ("l_elbow", r(12.0, 0.0)), ("r_elbow", r(18.0, 0.0)),
        ("l_hip", r(-4.0, 3.0)), ("l_knee", r(-4.0, 0.0)),
        ("r_hip", r(18.0, 3.0)), ("r_knee", r(-50.0, 0.0)),
    ]},
];
// Skinning bind: A-pose (arms 32 deg) so the armpit does not web when the mesh is bound.
const BIND: Key = Key { name: "bind", bob: 0.0, rots: &[
    ("r_shoulder", r(0.0, 32.0)), ("l_shoulder", r(0.0, 32.0)),
    ("r_elbow", r(10.0, 0.0)), ("l_elbow", r(10.0, 0.0)),
    ("r_hip", r(0.0, 3.0)), ("l_hip", r(0.0, 3.0)),
]};
const CYCLE: [usize; 4] = [1, 2, 3, 4];
const STEPS_PER_KEY: usize = 4;

// A resolved pose: one rotation per rig joint, blendable.
struct Frame {
    name: String,
    bob: f32,
    rots: Vec<Rot>,
}

impl Frame {
    fn from_key(k: &Key) -> Frame {
        let rots = RIG
            .iter()
            .map(|j| k.rots.iter().find(|(n, _)| *n == j.name).map(|(_, r)| *r).unwrap_or_default())
            .collect();
        Frame { name: k.name.to_string(), bob: k.bob, rots }
    }

    fn blend(a: &Frame, b: &Frame, t: f32, name: String) -> Frame {
        let e = 0.5 - 0.5 * (t * std::f32::consts::PI).cos();
        let mix = |x: f32, y: f32| x + (y - x) * e;
        let rots = a
            .rots
            .iter()
            .zip(&b.rots)
            .map(|(p, q)| Rot { pitch: mix(p.pitch, q.pitch), spread: mix(p.spread, q.spread), yaw: mix(p.yaw, q.yaw) })
            .collect();
        Frame { name, bob: mix(a.bob, b.bob), rots }
    }
}

fn keyframes() -> Vec<Frame> {
    KEYS.iter().map(Frame::from_key).collect()
}

fn walk_cycle() -> Vec<Frame> {
    let keys = keyframes();
    let mut out = Vec::new();
    for (i, &k) in CYCLE.iter().enumerate() {
        let next = CYCLE[(i + 1) % CYCLE.len()];
        for s in 0..STEPS_PER_KEY {
            let t = s as f32 / STEPS_PER_KEY as f32;
            out.push(Frame::blend(&keys[k], &keys[next], t, format!("walk {}", out.len())));
        }
    }
    out
}

fn local_rotation(name: &str, rot: Rot) -> Quat {
    let side = if name.starts_with("r_") { -1.0 } else { 1.0 };
    Quat::from_rotation_y(rot.yaw.to_radians())
        * Quat::from_rotation_z(side * rot.spread.to_radians())
        * Quat::from_rotation_x(-rot.pitch.to_radians())
}

// Forward kinematics: world = parent_world * T(offset) * R(local).
fn pose(key: &Frame) -> Vec<Mat4> {
    let mut world: Vec<Mat4> = Vec::with_capacity(RIG.len());
    for (joint, &rot) in RIG.iter().zip(&key.rots) {
        let mut offset = joint.offset;
        if joint.parent.is_none() {
            offset.y += key.bob;
        }
        let local = Mat4::from_rotation_translation(local_rotation(joint.name, rot), offset);
        world.push(joint.parent.map_or(local, |p| world[p] * local));
    }
    world
}

#[derive(Clone, Copy)]
struct Shape {
    a: Vec3,
    b: Vec3,
    r1: f32,
    r2: f32,
    kind: Kind,
    core: bool,
    // detail muscle: kept out of the soft core blend, joined with a tight crease instead
    detail: bool,
    // joint whose world matrix carries this shape rigidly (skinning bone)
    bone: usize,
}

fn cone(a: Vec3, b: Vec3, r1: f32, r2: f32, kind: Kind) -> Shape {
    Shape { a, b, r1, r2, kind, core: false, detail: false, bone: 0 }
}

fn core(a: Vec3, b: Vec3, r1: f32, r2: f32) -> Shape {
    Shape { a, b, r1, r2, kind: Kind::Body, core: true, detail: false, bone: 0 }
}

// A surface muscle with a crease at its border (abs, serratus, obliques).
fn detail(a: Vec3, b: Vec3, r1: f32, r2: f32) -> Shape {
    Shape { detail: true, ..core(a, b, r1, r2) }
}

impl Shape {
    fn on(self, bone: usize) -> Shape {
        Shape { bone, ..self }
    }
}

// Torso, neck, traps, delts melt together; limbs and head stay crisp so the arm gap and head read.
const CORE_JOINTS: &[&str] = &["spine", "chest", "neck"];
const TRAP_BUNDLES: usize = 4;

// Bones as capsules, plus muscle masses parented to chest/shoulder/pelvis frames so they ride the pose.
// Blend-shape offsets for the traps; zero is the authored body.
#[derive(Clone, Copy, Default)]
struct Tweak {
    trap_height: f32,
    trap_mass: f32,
    trap_reach: f32,
    trap_slope: f32,
    delt_mass: f32,
    delt_length: f32,
    delt_out: f32,
    delt_cap: f32,
}

fn shapes(world: &[Mat4], pos: &[Vec3], t: Tweak) -> Vec<Shape> {
    let mut out: Vec<Shape> = RIG
        .iter()
        .enumerate()
        .filter_map(|(i, jt)| {
            jt.parent.map(|p| {
                let core = CORE_JOINTS.contains(&jt.name);
                // the skull tapers to a cone at the crown (refs #1, #5); other bones are straight capsules
                let r1 = if jt.name == "crown" { RIG[p].radius } else { jt.radius };
                Shape { core, bone: p, ..cone(pos[p], pos[i], r1, jt.radius, Kind::Body) }
            })
        })
        .collect();
    let at = |m: usize, x: f32, y: f32, z: f32| world[m].transform_point3(Vec3::new(x, y, z));
    // long narrow face (goat-skull read): brow down to a square grin, jaw no wider than the face
    out.push(cone(at(HEAD, 0.0, 0.35, 0.3), at(HEAD, 0.0, -0.2, 0.45), 0.3, 0.27, Kind::Body).on(HEAD));
    for side in [-1.0, 1.0] {
        out.push(cone(at(HEAD, side * 0.2, 0.0, 0.1), at(HEAD, side * 0.13, -0.45, 0.42), 0.17, 0.16, Kind::Body).on(HEAD));
    }
    out.push(cone(at(HEAD, -0.1, -0.47, 0.45), at(HEAD, 0.1, -0.47, 0.45), 0.16, 0.16, Kind::Body).on(HEAD));
    // head-tail: segmented, tapering, arcs up off the back of the skull then back and down past the
    // left shoulder with a curl at the end (refs #2, #6); segments meet in creases so the bands ink
    let tail = TAIL;
    for k in 0..tail.len() - 1 {
        let (a, b) = (tail[k], tail[k + 1]);
        let r = 0.3 - 0.03 * k as f32;
        out.push(detail(at(HEAD, a.0, a.1, a.2), at(HEAD, b.0, b.1, b.2), r, r * 0.88).on(HEAD));
    }
    // cranial ridge: a keel from brow up over the long skull
    out.push(cone(at(HEAD, 0.0, 0.25, 0.42), at(CROWN, 0.0, 0.0, 0.05), 0.2, 0.12, Kind::Body).on(HEAD));
    for side in [-1.0, 1.0] {
        let (shoulder, elbow) = if side < 0.0 { (R_SHOULDER, R_ELBOW) } else { (L_SHOULDER, L_ELBOW) };
        // trapezius as a fan of bundles: spine origins (skull base down to upper back) to clavicle/acromion
        for k in 0..TRAP_BUNDLES {
            let f = k as f32 / (TRAP_BUNDLES - 1) as f32;
            // upper bundle runs skull base to acromion at about 45 degrees; lower ones fan down the back
            let origin = at(CHEST, side * (0.3 - 0.1 * f), 2.05 - 1.9 * f + t.trap_height * (1.0 - f), -0.15 - 0.5 * f);
            let insert = at(CHEST, side * (1.85 - 0.3 * f + t.trap_reach), 0.4 - 0.1 * f - t.trap_slope, -0.1 - 0.3 * f);
            let r = 0.58 - 0.18 * f + t.trap_mass;
            out.push(core(origin, insert, r, r * 0.8).on(CHEST));
        }
        out.push(
            cone(
                at(shoulder, side * (0.04 + t.delt_out), -0.1, 0.03),
                at(shoulder, side * (0.06 + t.delt_out * 0.5), -1.0 - t.delt_length, 0.03),
                0.5 + t.delt_mass,
                0.37 + t.delt_mass * 0.2,
                Kind::Body,
            )
            .on(shoulder),
        );
        // delt cap: a ball on the acromion that swells up and out, not along the arm
        let cap = at(shoulder, side * (0.06 + t.delt_out + t.delt_cap * 0.4), 0.0 + t.delt_cap * 0.5, 0.0);
        out.push(cone(cap, cap + Vec3::Y * 0.01, 0.35 + t.delt_cap * 0.35, 0.35 + t.delt_cap * 0.35, Kind::Body).on(shoulder));
        // pec shelf: a heavy slab from sternum to armpit, underside overhangs the ribs
        out.push(core(at(CHEST, side * 0.25, 0.25, 0.62), at(CHEST, side * 1.45, 0.3, 0.3), 0.55, 0.5).on(CHEST));
        // lats: wings flaring out and back behind him at ~45 deg (seen from above), from the armpit
        // down to a narrow waist (refs: shoulders ~2x waist)
        out.push(core(at(CHEST, side * 1.35, -0.1, -0.7), at(CHEST, side * 0.6, -1.7, -0.25), 0.6, 0.3).on(CHEST));
        out.push(core(at(CHEST, side * 1.45, -0.3, -0.55), at(CHEST, side * 0.9, -1.3, -0.45), 0.42, 0.34).on(CHEST));
        // biceps peak in front, triceps horseshoe behind
        out.push(cone(at(shoulder, 0.0, -0.8, 0.18), at(shoulder, 0.0, -1.7, 0.15), 0.52, 0.4, Kind::Body).on(shoulder));
        out.push(cone(at(shoulder, 0.0, -0.6, -0.16), at(shoulder, 0.0, -1.9, -0.12), 0.46, 0.32, Kind::Body).on(shoulder));
        // forearm: brachioradialis bulge just under the elbow, tapering hard to the wrist
        out.push(cone(at(elbow, side * 0.08, -0.35, 0.06), at(elbow, 0.0, -1.6, 0.0), 0.46, 0.26, Kind::Body).on(elbow));
        // calf: gastrocnemius belly high on the back of the shin
        // foot: broad forefoot and five splayed gripping toes, big toe on the inside (refs #1, #2, #8)
        let ankle = if side < 0.0 { 16 } else { 20 };
        out.push(cone(at(ankle, side * 0.2, -0.3, 0.75), at(ankle, -side * 0.22, -0.3, 0.8), 0.24, 0.26, Kind::Body).on(ankle));
        for k in 0..5 {
            let f = k as f32 / 4.0;
            let x = side * (0.3 - 0.58 * f);
            let splay = side * (0.12 - 0.24 * f);
            let r = if k == 4 { 0.16 } else { 0.12 - 0.01 * (3 - k) as f32 };
            let len = if k == 4 { 0.42 } else { 0.28 + 0.04 * k as f32 };
            out.push(detail(at(ankle, x, -0.36, 1.0), at(ankle, x + splay, -0.44, 1.15 + len), r, r * 0.85).on(ankle));
        }
        let knee = if side < 0.0 { 15 } else { 19 };
        out.push(cone(at(knee, 0.0, -0.45, -0.18), at(knee, 0.0, -1.9, -0.05), 0.62, 0.42, Kind::Body).on(knee));
    }
    // eight-pack: 4 rows x 2, shrinking toward the navel; linea alba and tendinous rows are the creases
    for row in 0..4 {
        let y = -0.6 - row as f32 * 0.5;
        let (z, r) = (0.72 - row as f32 * 0.07, 0.24 - row as f32 * 0.012);
        for side in [-1.0, 1.0] {
            out.push(detail(at(CHEST, side * 0.21, y + 0.08, z), at(CHEST, side * 0.24, y - 0.08, z), r, r).on(CHEST));
        }
    }
    for side in [-1.0, 1.0] {
        // serratus: fingers down the ribs under the armpit, slanting forward-down
        for k in 0..3 {
            let y = -0.45 - k as f32 * 0.35;
            out.push(detail(at(CHEST, side * 1.02, y, 0.05), at(CHEST, side * 0.82, y - 0.25, 0.45), 0.15, 0.12).on(CHEST));
        }
        // back: erector columns either side of the spine groove, scapula blade muscles, teres major to the armpit
        out.push(detail(at(CHEST, side * 0.27, -0.7, -0.78), at(CHEST, side * 0.25, -3.1, -0.6), 0.25, 0.3).on(CHEST));
        out.push(detail(at(CHEST, side * 0.7, 0.15, -0.8), at(CHEST, side * 0.95, -0.5, -0.8), 0.36, 0.3).on(CHEST));
        out.push(detail(at(CHEST, side * 1.0, -0.5, -0.85), at(CHEST, side * 1.65, -0.15, -0.45), 0.28, 0.24).on(CHEST));
        // external oblique: ribs to the hip crest, overhangs the sash
        out.push(detail(at(CHEST, side * 0.78, -1.4, 0.2), at(CHEST, side * 0.72, -3.0, 0.3), 0.3, 0.34).on(CHEST));
    }
    // hakama: cinched at the sash on the hip, flares to the knee
    out.push(cone(at(PELVIS, 0.0, 0.25, 0.0), at(PELVIS, 0.0, -3.3, 0.1), 1.25, 1.8, Kind::Skirt).on(PELVIS));
    out
}

// iq round cone: sphere r1 at a swept to sphere r2 at b.
fn sd_round_cone(p: Vec3, s: &Shape) -> f32 {
    let ba = s.b - s.a;
    let l2 = ba.length_squared().max(1e-6);
    let rr = s.r1 - s.r2;
    let a2 = l2 - rr * rr;
    let il2 = 1.0 / l2;
    let pa = p - s.a;
    let y = pa.dot(ba);
    let z = y - l2;
    let x2 = (pa * l2 - ba * y).length_squared();
    let y2 = y * y * l2;
    let z2 = z * z * l2;
    let k = rr.signum() * rr * rr * x2;
    if z.signum() * a2 * z2 > k {
        return (x2 + z2).sqrt() * il2 - s.r2;
    }
    if y.signum() * a2 * y2 < k {
        return (x2 + y2).sqrt() * il2 - s.r1;
    }
    ((x2 * a2 * il2).sqrt() + y * rr) * il2 - s.r1
}

// iq capped cone: flat caps, for the hakama hem.
fn sd_capped_cone(p: Vec3, s: &Shape) -> f32 {
    let (ra, rb) = (s.r1, s.r2);
    let rba = rb - ra;
    let baba = (s.b - s.a).length_squared();
    let papa = (p - s.a).length_squared();
    let paba = (p - s.a).dot(s.b - s.a) / baba;
    let x = (papa - paba * paba * baba).max(0.0).sqrt();
    let cax = (x - if paba < 0.5 { ra } else { rb }).max(0.0);
    let cay = (paba - 0.5).abs() - 0.5;
    let k = rba * rba + baba;
    let f = ((rba * (x - ra) + paba * baba) / k).clamp(0.0, 1.0);
    let cbx = x - ra - f * rba;
    let cby = paba - f;
    let sign = if cbx < 0.0 && cay < 0.0 { -1.0 } else { 1.0 };
    sign * (cax * cax + cay * cay * baba).min(cbx * cbx + cby * cby * baba).sqrt()
}

fn sd_shape(p: Vec3, s: &Shape) -> f32 {
    if s.kind == Kind::Skirt { sd_capped_cone(p, s) } else { sd_round_cone(p, s) }
}

fn smin(a: f32, b: f32, k: f32) -> f32 {
    let h = (k - (a - b).abs()).max(0.0) / k;
    a.min(b) - h * h * k * 0.25
}

const CORE_BLEND: f32 = 0.4;
const LIMB_BLEND: f32 = 0.3;
const DETAIL_BLEND: f32 = 0.08;

fn scene_sdf(p: Vec3, caps: &[Shape]) -> f32 {
    let (mut body, mut limbs, mut detail) = (f32::INFINITY, f32::INFINITY, f32::INFINITY);
    for c in caps {
        let d = sd_shape(p, c);
        if c.detail {
            detail = detail.min(d)
        } else if c.core {
            body = smin(body, d, CORE_BLEND)
        } else {
            limbs = smin(limbs, d, LIMB_BLEND)
        }
    }
    smin(smin(body, detail, DETAIL_BLEND), limbs, 0.2)
}

struct Camera {
    eye: Vec3,
    fwd: Vec3,
    right: Vec3,
    up: Vec3,
    tan_half: f32,
    aspect: f32,
}

impl Camera {
    // Neck-level eye, straight down the walk line, tilted slightly down at the belly.
    fn new(yaw_deg: f32, aspect: f32) -> Self {
        let yaw = yaw_deg.to_radians();
        let eye = Vec3::new(yaw.sin() * 29.0, 9.8, yaw.cos() * 29.0);
        let target = Vec3::new(0.0, 6.2, 0.0);
        let fwd = (target - eye).normalize();
        let right = fwd.cross(Vec3::Y).normalize();
        let up = right.cross(fwd);
        Camera { eye, fwd, right, up, tan_half: 16.0f32.to_radians().tan(), aspect }
    }

    fn project(&self, p: Vec3, w: usize, h: usize) -> Option<(f32, f32, f32)> {
        let d = p - self.eye;
        let z = d.dot(self.fwd);
        if z <= 0.1 {
            return None;
        }
        let x = d.dot(self.right) / (z * self.tan_half * self.aspect);
        let y = d.dot(self.up) / (z * self.tan_half);
        Some(((x + 1.0) * 0.5 * w as f32, (1.0 - y) * 0.5 * h as f32, z))
    }

    fn ray(&self, col: usize, row: usize, w: usize, h: usize) -> Vec3 {
        let x = ((col as f32 + 0.5) / w as f32) * 2.0 - 1.0;
        let y = 1.0 - ((row as f32 + 0.5) / h as f32) * 2.0;
        self.ray_ndc(x, y)
    }

    fn ray_ndc(&self, x: f32, y: f32) -> Vec3 {
        (self.fwd + self.right * x * self.tan_half * self.aspect + self.up * y * self.tan_half).normalize()
    }
}

fn march(origin: Vec3, dir: Vec3, caps: &[Shape], max_t: f32) -> Option<f32> {
    let mut t = 0.0;
    for _ in 0..80 {
        let d = scene_sdf(origin + dir * t, caps);
        if d < 0.01 {
            return Some(t);
        }
        t += d;
        if t > max_t {
            return None;
        }
    }
    None
}

fn surface_depth(cam: &Camera, caps: &[Shape], w: usize, h: usize) -> Vec<f32> {
    let mut depth = vec![f32::INFINITY; w * h];
    for row in 0..h {
        for col in 0..w {
            let dir = cam.ray(col, row, w, h);
            if let Some(t) = march(cam.eye, dir, caps, 60.0) {
                depth[row * w + col] = t * dir.dot(cam.fwd);
            }
        }
    }
    depth
}

fn occluded(cam: &Camera, p: Vec3, caps: &[Shape]) -> bool {
    let to = p - cam.eye;
    let len = to.length();
    march(cam.eye, to / len, caps, len).is_some_and(|t| t < len - 0.3)
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Kind {
    Body,
    Skirt,
    Wheel,
    Blade,
    Joint,
}

struct Wire {
    pts: Vec<Vec3>,
    kind: Kind,
}

fn polygon(center: Vec3, p: Vec3, q: Vec3, radius: f32, sides: usize, phase: f32) -> Vec<Vec3> {
    (0..=sides)
        .map(|i| {
            let a = (i as f32 + phase) / sides as f32 * std::f32::consts::TAU;
            center + (p * a.cos() + q * a.sin()) * radius
        })
        .collect()
}

// Low-poly tube: stacked polygons, vertical edges, one diagonal per quad (the triangulated mesh look).
fn mesh_tube(out: &mut Vec<Wire>, rings: &[(Vec3, f32)], axis: Vec3, sides: usize, kind: Kind) {
    let (p, q) = axis.any_orthonormal_pair();
    let polys: Vec<Vec<Vec3>> = rings.iter().map(|(c, rad)| polygon(*c, p, q, *rad, sides, 0.0)).collect();
    for poly in &polys {
        out.push(Wire { pts: poly.clone(), kind });
    }
    for pair in polys.windows(2) {
        for k in 0..sides {
            out.push(Wire { pts: vec![pair[0][k], pair[1][k]], kind });
            out.push(Wire { pts: vec![pair[0][k], pair[1][k + 1]], kind });
        }
    }
}

fn wires(world: &[Mat4], pos: &[Vec3], caps: &[Shape], toward_cam: Vec3) -> Vec<Wire> {
    let mut out = Vec::new();
    for c in caps {
        let axis = (c.b - c.a).normalize_or(Vec3::Y);
        let rings = [(c.a, c.r1), (c.a.lerp(c.b, 0.5), (c.r1 + c.r2) * 0.5), (c.b, c.r2)];
        let sides = if c.kind == Kind::Skirt { 8 } else { 6 };
        mesh_tube(&mut out, &rings, axis, sides, c.kind);
    }
    // Wheel: tilted ring of 8 spokes with knob beads, parented to the crown.
    let crown = world[CROWN] * Mat4::from_rotation_translation(Quat::from_rotation_x(-0.25), Vec3::new(0.0, 0.25, 0.0));
    let hub = crown.transform_point3(Vec3::ZERO);
    let (wx, wz) = (crown.transform_vector3(Vec3::X), crown.transform_vector3(Vec3::Z));
    let rim = polygon(hub, wx, wz, 0.8, 16, 0.0);
    out.push(Wire { pts: rim.clone(), kind: Kind::Wheel });
    for k in (0..16).step_by(2) {
        out.push(Wire { pts: vec![hub, rim[k]], kind: Kind::Wheel });
        out.push(Wire { pts: vec![rim[k]], kind: Kind::Joint });
    }
    // Sword of Extermination: flat blade riding the outer edge of the right forearm, running past the fist.
    let fore = world[R_ELBOW];
    let edge = |y: f32, z: f32| fore.transform_point3(Vec3::new(-0.35, y, z));
    let blade = vec![edge(-0.5, 0.12), edge(-2.4, 0.2), edge(-5.3, 0.0), edge(-2.4, -0.2), edge(-0.5, -0.12), edge(-0.5, 0.12)];
    out.push(Wire { pts: blade, kind: Kind::Blade });
    for (i, p) in pos.iter().enumerate() {
        out.push(Wire { pts: vec![*p + toward_cam * RIG[i].radius.max(0.2)], kind: Kind::Joint });
    }
    out
}

fn slope_char(dx: f32, dy: f32) -> char {
    let (ax, ay) = (dx.abs(), dy.abs() * 2.0);
    if ax > ay * 2.0 {
        '-'
    } else if ay > ax * 2.0 {
        '|'
    } else if (dx > 0.0) == (dy > 0.0) {
        '\\'
    } else {
        '/'
    }
}

struct Canvas {
    w: usize,
    h: usize,
    chars: Vec<char>,
    zbuf: Vec<f32>,
    surf: Vec<f32>,
}

impl Canvas {
    fn plot(&mut self, x: f32, y: f32, z: f32, c: char) {
        let (col, row) = (x.floor() as isize, y.floor() as isize);
        if col < 0 || row < 0 || col >= self.w as isize || row >= self.h as isize {
            return;
        }
        let idx = row as usize * self.w + col as usize;
        if z <= self.surf[idx] + 0.35 && z < self.zbuf[idx] {
            self.zbuf[idx] = z;
            self.chars[idx] = c;
        }
    }

    // Walk each projected segment at half-cell steps, glyph from the segment's screen slope.
    fn wire(&mut self, cam: &Camera, wire: &Wire) {
        let proj: Vec<_> = wire.pts.iter().filter_map(|p| cam.project(*p, self.w, self.h)).collect();
        if wire.kind == Kind::Joint {
            if let Some(&(x, y, z)) = proj.first() {
                self.plot(x, y, z, 'o');
            }
            return;
        }
        for seg in proj.windows(2) {
            let ((x0, y0, z0), (x1, y1, z1)) = (seg[0], seg[1]);
            let c = match wire.kind {
                Kind::Skirt => '#',
                Kind::Wheel => '*',
                _ => slope_char(x1 - x0, y1 - y0),
            };
            let n = ((x1 - x0).abs().max((y1 - y0).abs()) * 2.0).ceil().max(1.0) as usize;
            for i in 0..=n {
                let t = i as f32 / n as f32;
                self.plot(x0 + (x1 - x0) * t, y0 + (y1 - y0) * t, z0 + (z1 - z0) * t, c);
            }
        }
    }
}

struct Posed {
    world: Vec<Mat4>,
    pos: Vec<Vec3>,
    caps: Vec<Shape>,
}

fn posed(key: &Frame) -> Posed {
    posed_with(key, Tweak::default())
}

fn posed_with(key: &Frame, tweak: Tweak) -> Posed {
    let world = pose(key);
    let pos: Vec<Vec3> = world.iter().map(|m| m.transform_point3(Vec3::ZERO)).collect();
    let caps = shapes(&world, &pos, tweak);
    Posed { world, pos, caps }
}

fn render(key: &Frame, yaw: f32, w: usize, h: usize) -> Vec<String> {
    let s = posed(key);
    // Terminal cells are about twice as tall as wide.
    let cam = Camera::new(yaw, w as f32 * 0.5 / h as f32);
    let surf = surface_depth(&cam, &s.caps, w, h);
    let mut cv = Canvas { w, h, chars: vec![' '; w * h], zbuf: vec![f32::INFINITY; w * h], surf };
    for wire in wires(&s.world, &s.pos, &s.caps, -cam.fwd) {
        cv.wire(&cam, &wire);
    }
    let mut out = vec![format!("{:<w$}", format!("{} yaw {yaw}", key.name))];
    out.extend(cv.chars.chunks(w).map(|r| r.iter().collect::<String>()));
    out
}

fn svg(key: &Frame, yaw: f32, pw: usize, ph: usize) -> String {
    let s = posed(key);
    let cam = Camera::new(yaw, pw as f32 / ph as f32);
    let kinds = [
        (Kind::Body, "#ffd7b0"),
        (Kind::Skirt, "#8fb8aa"),
        (Kind::Wheel, "#f0b848"),
        (Kind::Blade, "#dfe8f0"),
    ];
    let all = wires(&s.world, &s.pos, &s.caps, -cam.fwd);
    let mut layers = String::new();
    for (kind, color) in kinds {
        let (mut front, mut back) = (String::new(), String::new());
        for wire in all.iter().filter(|w| w.kind == kind) {
            let proj: Vec<_> = wire
                .pts
                .iter()
                .map(|p| cam.project(*p, pw, ph).map(|(x, y, _)| (x, y, occluded(&cam, *p, &s.caps))))
                .collect();
            for pair in proj.windows(2) {
                let (Some((x0, y0, h0)), Some((x1, y1, h1))) = (pair[0], pair[1]) else { continue };
                let seg = format!("M{x0:.1} {y0:.1}L{x1:.1} {y1:.1}");
                if h0 && h1 { back += &seg } else { front += &seg }
            }
        }
        layers += &format!(
            "<path d='{back}' stroke='{color}' stroke-opacity='0.18' stroke-width='0.8' fill='none'/>\
<path d='{front}' stroke='{color}' stroke-width='1.3' fill='none' filter='url(#glow)'/>"
        );
    }
    let mut dots = String::new();
    for wire in all.iter().filter(|w| w.kind == Kind::Joint) {
        if let Some((x, y, _)) = cam.project(wire.pts[0], pw, ph) {
            dots += &format!("<circle cx='{x:.1}' cy='{y:.1}' r='2.6' fill='#fff4e6'/>");
        }
    }
    format!(
        "<svg xmlns='http://www.w3.org/2000/svg' width='{pw}' height='{ph}' style='background:#140b0a'>\
<defs><filter id='glow' x='-20%' y='-20%' width='140%' height='140%'><feGaussianBlur stdDeviation='2.2' result='b'/>\
<feMerge><feMergeNode in='b'/><feMergeNode in='SourceGraphic'/></feMerge></filter></defs>{layers}{dots}</svg>"
    )
}

// Closest approach of the ray to the body (negative on hit) and the hit depth.
fn nearest_shape(p: Vec3, caps: &[Shape]) -> usize {
    caps.iter()
        .enumerate()
        .map(|(i, c)| (i, sd_shape(p, c)))
        .fold((0, f32::INFINITY), |best, cur| if cur.1 < best.1 { cur } else { best })
        .0
}

// (closest approach, negative on hit; hit depth; id of the muscle/bone owning the hit point)
fn ray_field(cam: &Camera, dir: Vec3, caps: &[Shape]) -> (f32, f32, usize) {
    let (mut t, mut closest) = (0.0, f32::INFINITY);
    for _ in 0..90 {
        let d = scene_sdf(cam.eye + dir * t, caps);
        closest = closest.min(d);
        if d < 0.01 {
            return (-0.05, t * dir.dot(cam.fwd), nearest_shape(cam.eye + dir * t, caps));
        }
        t += d;
        if t > 60.0 {
            break;
        }
    }
    (closest, f32::INFINITY, usize::MAX)
}

// Marching squares on the ray field: the silhouette outline only.
fn contour_svg(key: &Frame, yaw: f32, pw: usize, ph: usize, step: f32) -> String {
    let s = posed(key);
    let cam = Camera::new(yaw, pw as f32 / ph as f32);
    let (gw, gh) = ((pw as f32 / step) as usize + 1, (ph as f32 / step) as usize + 1);
    let mut field = vec![(0.0, 0.0, 0); gw * gh];
    for gy in 0..gh {
        for gx in 0..gw {
            let x = gx as f32 * step / pw as f32 * 2.0 - 1.0;
            let y = 1.0 - gy as f32 * step / ph as f32 * 2.0;
            field[gy * gw + gx] = ray_field(&cam, cam.ray_ndc(x, y), &s.caps);
        }
    }
    let f = |x: usize, y: usize| field[y * gw + x];
    let mut outline = String::new();
    let seg = |out: &mut String, (x0, y0): (f32, f32), (x1, y1): (f32, f32)| {
        *out += &format!("M{:.1} {:.1}L{:.1} {:.1}", x0 * step, y0 * step, x1 * step, y1 * step);
    };
    for gy in 0..gh - 1 {
        for gx in 0..gw - 1 {
            let c = [(gx, gy), (gx + 1, gy), (gx + 1, gy + 1), (gx, gy + 1)];
            let v: Vec<f32> = c.iter().map(|&(x, y)| f(x, y).0).collect();
            let mut hits = Vec::new();
            for e in 0..4 {
                let (a, b) = (e, (e + 1) % 4);
                if (v[a] < 0.0) != (v[b] < 0.0) {
                    let t = v[a] / (v[a] - v[b]);
                    let (ax, ay) = (c[a].0 as f32, c[a].1 as f32);
                    let (bx, by) = (c[b].0 as f32, c[b].1 as f32);
                    hits.push((e, (ax + (bx - ax) * t, ay + (by - ay) * t)));
                }
            }
            match hits.len() {
                2 => seg(&mut outline, hits[0].1, hits[1].1),
                4 => {
                    let center = v.iter().sum::<f32>() * 0.25;
                    if (center < 0.0) == (v[0] < 0.0) {
                        seg(&mut outline, hits[0].1, hits[1].1);
                        seg(&mut outline, hits[2].1, hits[3].1);
                    } else {
                        seg(&mut outline, hits[3].1, hits[0].1);
                        seg(&mut outline, hits[1].1, hits[2].1);
                    }
                }
                _ => {}
            }
        }
    }
    format!(
        "<svg xmlns='http://www.w3.org/2000/svg' width='{pw}' height='{ph}' style='background:#f4e4d6'>\
<path d='{outline}' stroke='#3a140e' stroke-width='2.4' stroke-linecap='round' fill='none'/></svg>"
    )
}

fn html(yaw: f32, w: usize, h: usize) -> String {
    let mut body = String::new();
    for key in &keyframes() {
        let ascii = render(key, yaw, w, h).join("\n").replace('&', "&amp;").replace('<', "&lt;");
        body += &format!(
            "<section><h2>{} yaw {yaw}</h2><div class='pair'>{}{}<pre>{ascii}</pre></div></section>",
            key.name,
            svg(key, yaw, 520, 640),
            contour_svg(key, yaw, 520, 640, 2.5)
        );
    }
    format!(
        "<!doctype html><meta charset='utf-8'><title>maha rig</title><style>\
body{{background:#1b1210;color:#f3d9c8;font-family:sans-serif}}\
.pair{{display:flex;gap:24px;align-items:flex-start}}\
pre{{background:#2a1a16;padding:8px;font-size:11px;line-height:1.05;margin:0}}</style>{body}"
    )
}

struct Surface {
    pos: Vec<Vec3>,
    nrm: Vec<Vec3>,
    idx: Vec<u32>,
}

// Blended surface as a triangle mesh (naive surface nets) in world units.
fn surface(caps: &[Shape], voxel: f32) -> Surface {
    use fast_surface_nets::ndshape::{RuntimeShape, Shape as _};
    use fast_surface_nets::{surface_nets, SurfaceNetsBuffer};
    let origin = Vec3::new(-6.0, -1.0, -4.0);
    let dims = ((Vec3::new(12.0, 15.0, 8.0) / voxel).ceil()).as_uvec3().to_array();
    let grid = RuntimeShape::<u32, 3>::new(dims);
    let sdf: Vec<f32> = (0..grid.usize() as u32)
        .map(|i| {
            let [x, y, z] = grid.delinearize(i);
            scene_sdf(origin + Vec3::new(x as f32, y as f32, z as f32) * voxel, caps)
        })
        .collect();
    let mut buf = SurfaceNetsBuffer::default();
    surface_nets(&sdf, &grid, [0; 3], [dims[0] - 1, dims[1] - 1, dims[2] - 1], &mut buf);
    Surface {
        pos: buf.positions.iter().map(|p| origin + Vec3::from_array(*p) * voxel).collect(),
        nrm: buf.normals.iter().map(|n| Vec3::from_array(*n).normalize_or(Vec3::Y)).collect(),
        idx: buf.indices,
    }
}

fn floats(vs: &[Vec3]) -> String {
    let v: Vec<String> = vs.iter().map(|p| format!("{:.3},{:.3},{:.3}", p.x, p.y, p.z)).collect();
    v.join(",")
}

fn ints<T: ToString>(vs: &[T]) -> String {
    vs.iter().map(T::to_string).collect::<Vec<_>>().join(",")
}

// Rigid solid props that are too thin for the voxel field: triangle soups carried by one bone each.
struct Prop {
    kind: &'static str,
    bone: usize,
    tris: Vec<Vec3>,
}

fn quad(t: &mut Vec<Vec3>, a: Vec3, b: Vec3, c: Vec3, d: Vec3) {
    t.extend([a, b, c, a, c, d]);
}

fn frame_of(axis: Vec3) -> (Vec3, Vec3) {
    let u = axis.any_orthonormal_vector();
    (u, axis.cross(u).normalize())
}

fn tube(t: &mut Vec<Vec3>, a: Vec3, b: Vec3, r1: f32, r2: f32, sides: usize) {
    let (u, v) = frame_of((b - a).normalize_or(Vec3::Y));
    let ring = |c: Vec3, r: f32, k: usize| {
        let th = k as f32 / sides as f32 * std::f32::consts::TAU;
        c + (u * th.cos() + v * th.sin()) * r
    };
    for k in 0..sides {
        quad(t, ring(a, r1, k), ring(a, r1, k + 1), ring(b, r2, k + 1), ring(b, r2, k));
    }
}

fn ball(t: &mut Vec<Vec3>, c: Vec3, r: f32) {
    let (rows, cols) = (6, 10);
    let at = |i: usize, k: usize| {
        let (ph, th) = (i as f32 / rows as f32 * std::f32::consts::PI, k as f32 / cols as f32 * std::f32::consts::TAU);
        c + Vec3::new(ph.sin() * th.cos(), ph.cos(), ph.sin() * th.sin()) * r
    };
    for i in 0..rows {
        for k in 0..cols {
            quad(t, at(i, k), at(i + 1, k), at(i + 1, k + 1), at(i, k + 1));
        }
    }
}

// A flight feather: flat pointed leaf from base to tip in the plane with normal n, with a shallow ridge.
fn feather(t: &mut Vec<Vec3>, base: Vec3, tip: Vec3, width: f32, n: Vec3) {
    let along = tip - base;
    let side = along.cross(n).normalize_or(Vec3::X) * width * 0.5;
    let (l, r, ridge) = (base + along * 0.35 + side, base + along * 0.35 - side, base + along * 0.35 + n * width * 0.12);
    t.extend([base, l, ridge, ridge, l, tip, base, ridge, r, ridge, tip, r]);
}

// March from `from` along `dir` to the blended surface; the hit point, lifted `lift` off it.
fn onto_surface(caps: &[Shape], from: Vec3, dir: Vec3, lift: f32) -> Vec3 {
    let mut t = 0.0;
    for _ in 0..64 {
        let d = scene_sdf(from + dir * t, caps);
        if d < 0.005 {
            break;
        }
        t += d;
    }
    from + dir * (t - lift)
}

fn sdf_normal(caps: &[Shape], p: Vec3) -> Vec3 {
    let e = 0.02;
    let f = |d: Vec3| scene_sdf(p + d * e, caps) - scene_sdf(p - d * e, caps);
    Vec3::new(f(Vec3::X), f(Vec3::Y), f(Vec3::Z)).normalize_or(Vec3::Z)
}

// Flat closed loop of `sides` segments around c, lying in the plane with normal n.
fn loop_in(t: &mut Vec<Vec3>, c: Vec3, n: Vec3, radius: f32, thick: f32, sides: usize, phase: f32) {
    let (u, v) = frame_of(n);
    let at = |k: usize| {
        let th = phase + k as f32 / sides as f32 * std::f32::consts::TAU;
        c + (u * th.cos() + v * th.sin()) * radius
    };
    for k in 0..sides {
        tube(t, at(k), at(k + 1), thick, thick, 5);
    }
}

fn props(s: &Posed) -> Vec<Prop> {
    let (world, pos) = (&s.world, &s.pos);
    let mut out = Vec::new();
    // Wings: overlapping flat feathers swept back along the eye-socket tube, on the upper edge of the
    // upper pair and the lower edge of the lower pair, longest at the tip (refs #4, #5).
    for (root, mid, tip, droop) in [(22, 23, 24, -1.0), (25, 26, 27, -1.0), (28, 29, 30, 1.0), (31, 32, 33, 1.0)] {
        let (a, m, b) = (pos[root], pos[mid], pos[tip]);
        let along = (b - a).normalize();
        let back = Vec3::new(0.0, -droop, -0.3);
        let trail = (back - along * along.dot(back)).normalize();
        let n = along.cross(trail).normalize();
        let mut t = Vec::new();
        for k in 0..12 {
            let f = k as f32 / 11.0;
            let base = if f < 0.5 { a.lerp(m, 0.35 + f * 1.3) } else { m.lerp(b, (f - 0.5) * 2.0) };
            let dir = (along * (0.75 + 0.25 * f) + trail * (0.5 - 0.25 * f)).normalize();
            feather(&mut t, base, base + dir * (0.3 + 0.3 * f), 0.28 + 0.06 * f, n);
        }
        out.push(Prop { kind: "wing", bone: mid, tris: t });
    }
    // Dharma wheel: level halo floating over the crown, 8 spokes whose ends carry beads outside the rim.
    let crown = world[CROWN] * Mat4::from_translation(Vec3::new(0.0, 0.45, 0.0));
    let hub = crown.transform_point3(Vec3::ZERO);
    let (radius, mut t) = (0.85, Vec::new());
    let rim = |turn: f32, r: f32| {
        let th = turn * std::f32::consts::TAU;
        hub + Vec3::new(th.cos(), 0.0, th.sin()) * r
    };
    for k in 0..24 {
        tube(&mut t, rim(k as f32 / 24.0, radius), rim((k + 1) as f32 / 24.0, radius), 0.05, 0.05, 6);
    }
    ball(&mut t, hub, 0.14);
    for k in 0..8 {
        tube(&mut t, hub, rim(k as f32 / 8.0, radius * 1.18), 0.035, 0.035, 5);
        ball(&mut t, rim(k as f32 / 8.0, radius * 1.3), 0.13);
    }
    out.push(Prop { kind: "wheel", bone: CROWN, tris: t });
    // Sword of Extermination: thin flat single-edged blade out of the right fist along the forearm.
    let fore = world[R_ELBOW];
    let e = |x: f32, y: f32, z: f32| fore.transform_point3(Vec3::new(x, y, z));
    let mut t = Vec::new();
    let (spine_a, spine_b, edge_a, edge_b, point) =
        (e(-0.1, -1.9, -0.06), e(-0.1, -4.9, -0.03), e(-0.1, -1.9, 0.12), e(-0.1, -4.6, 0.1), e(-0.1, -5.4, 0.0));
    let thick = (fore.transform_vector3(Vec3::X)).normalize() * 0.025;
    for s in [1.0, -1.0] {
        quad(&mut t, spine_a + thick * s, spine_b + thick * s, edge_b, edge_a);
        t.extend([spine_b + thick * s, point, edge_b]);
    }
    quad(&mut t, spine_a + thick, spine_b + thick, spine_b - thick, spine_a - thick);
    out.push(Prop { kind: "blade", bone: R_ELBOW, tris: t });
    // Dark bands between the tail segments (refs #2, #6).
    let mut t = Vec::new();
    for k in 1..TAIL.len() - 1 {
        let h = |i: usize| world[HEAD].transform_point3(Vec3::from(TAIL[i]));
        let (c, dir) = (h(k), (h(k + 1) - h(k - 1)).normalize());
        let r = 0.3 - 0.03 * k as f32 + 0.03;
        tube(&mut t, c - dir * 0.04, c + dir * 0.04, r, r, 10);
    }
    out.push(Prop { kind: "ring", bone: HEAD, tris: t });
    // Necklace across the clavicles: ring and hexagon links, a wide centre plate, dark tassels (refs #4, #10).
    let chest = world[CHEST];
    let toward = chest.transform_vector3(Vec3::Z).normalize();
    let on_chest = |x: f32, y: f32| onto_surface(&s.caps, chest.transform_point3(Vec3::new(x, y, 3.0)), -toward, 0.06);
    let mut t = Vec::new();
    let link = |k: i32| on_chest(k as f32 * 0.3, 0.45 + 0.45 * (k as f32 * 0.3 / 1.2).powi(2));
    for k in -4i32..4 {
        tube(&mut t, link(k), link(k + 1), 0.02, 0.02, 4);
    }
    for k in -4i32..=4 {
        let c = link(k);
        let n = sdf_normal(&s.caps, c);
        match k.abs() {
            0 => {
                loop_in(&mut t, c, n, 0.2, 0.045, 6, 0.0);
                for dx in [-0.05, 0.05] {
                    let top = c + chest.transform_vector3(Vec3::new(dx, -0.18, 0.0));
                    tube(&mut t, top, top + chest.transform_vector3(Vec3::new(dx * 0.5, -0.3, 0.04)), 0.045, 0.02, 4);
                }
            }
            k if k % 2 == 1 => loop_in(&mut t, c, n, 0.08, 0.03, 10, 0.0),
            _ => {
                loop_in(&mut t, c, n, 0.12, 0.035, 6, 0.0);
                let top = c + chest.transform_vector3(Vec3::new(0.0, -0.12, 0.0));
                tube(&mut t, top, top + chest.transform_vector3(Vec3::new(0.0, -0.28, 0.05)), 0.04, 0.025, 4);
            }
        }
    }
    out.push(Prop { kind: "ring", bone: CHEST, tris: t });
    // Sash: light band knotted at the front of the waist, a long tail hanging to about the hem (refs #1, #2).
    let pelvis = world[PELVIS];
    let mut t = Vec::new();
    let band = |th: f32, y: f32| {
        let dir = pelvis.transform_vector3(Vec3::new(th.sin(), 0.0, th.cos())).normalize();
        let axis = pelvis.transform_point3(Vec3::new(0.0, y, 0.0));
        // start inside the arm gap so the probe cannot begin inside a hanging arm
        onto_surface(&s.caps, axis + dir * 1.6, -dir, 0.05)
    };
    for k in 0..32 {
        let (a, b) = (k as f32 / 32.0 * std::f32::consts::TAU, (k + 1) as f32 / 32.0 * std::f32::consts::TAU);
        quad(&mut t, band(a, 0.55), band(b, 0.55), band(b, 0.15), band(a, 0.15));
    }
    let knot = band(0.15, 0.35);
    ball(&mut t, knot + toward * 0.08, 0.22);
    let (mut prev_l, mut prev_r) = (knot + pelvis.transform_vector3(Vec3::new(-0.12, 0.0, 0.1)), knot + pelvis.transform_vector3(Vec3::new(0.14, 0.0, 0.1)));
    for k in 1..=8 {
        let f = k as f32 / 8.0;
        let c = knot + pelvis.transform_vector3(Vec3::new(0.12 * f, -3.0 * f, 0.1 + 0.25 * f * (1.0 - f)));
        let w = 0.2 + 0.07 * f;
        let (l, r) = (c + pelvis.transform_vector3(Vec3::new(-w, 0.0, 0.0)), c + pelvis.transform_vector3(Vec3::new(w, 0.0, 0.0)));
        // the tail drapes over the hakama: never sink into it
        let front = |p: Vec3| {
            let q = onto_surface(&s.caps, p + toward * 1.5, -toward, 0.06);
            if q.dot(toward) > p.dot(toward) { q } else { p }
        };
        let (l, r) = (front(l), front(r));
        quad(&mut t, prev_l, prev_r, r, l);
        (prev_l, prev_r) = (l, r);
    }
    out.push(Prop { kind: "cloth", bone: PELVIS, tris: t });
    // Black rings on both wrists and ankles (refs #1, #2, #9).
    for (bone, r) in [(8, 0.46), (12, 0.46), (16, 0.42), (20, 0.42)] {
        let m = world[bone];
        let mut t = Vec::new();
        let at = |k: usize| m.transform_point3(Vec3::new((k as f32 / 16.0 * std::f32::consts::TAU).cos() * r, 0.1, (k as f32 / 16.0 * std::f32::consts::TAU).sin() * r));
        for k in 0..16 {
            tube(&mut t, at(k), at(k + 1), 0.1, 0.1, 6);
        }
        out.push(Prop { kind: "ring", bone, tris: t });
    }
    // Bandage: overlapping wraps around the right forearm, wrist to just under the elbow (refs #2, #19).
    let mut t = Vec::new();
    for k in 0..9 {
        let y = -0.45 - k as f32 * 0.16;
        // hugs the forearm surface: bulge under the elbow tapering to the wrist
        let r = 0.54 - k as f32 * 0.022;
        let ring = |a: usize, dy: f32| {
            let th = a as f32 / 12.0 * std::f32::consts::TAU;
            fore.transform_point3(Vec3::new(th.cos() * r, y + dy + 0.03 * th.sin(), th.sin() * r))
        };
        for a in 0..12 {
            quad(&mut t, ring(a, 0.1), ring(a + 1, 0.1), ring(a + 1, -0.1), ring(a, -0.1));
        }
    }
    out.push(Prop { kind: "cloth", bone: R_ELBOW, tris: t });
    out
}

fn props_json(s: &Posed) -> String {
    let v: Vec<String> = props(s)
        .iter()
        .map(|p| format!("{{\"kind\":\"{}\",\"bone\":{},\"tri\":[{}]}}", p.kind, p.bone, floats(&p.tris)))
        .collect();
    v.join(",")
}

fn line_json(s: &Posed, with_bone: bool) -> String {
    let mut lines = Vec::new();
    for w in wires(&s.world, &s.pos, &s.caps, Vec3::Z) {
        let (tag, bone) = match w.kind {
            Kind::Wheel => ("wheel", CROWN),
            Kind::Blade => ("blade", R_ELBOW),
            _ => continue,
        };
        let pts: Vec<String> = w.pts.iter().map(|p| format!("[{:.3},{:.3},{:.3}]", p.x, p.y, p.z)).collect();
        let bone = if with_bone { format!(",\"bone\":{bone}") } else { String::new() };
        lines.push(format!("{{\"kind\":\"{tag}\"{bone},\"pts\":[{}]}}", pts.join(",")));
    }
    lines.join(",")
}

fn mesh_json(key: &Frame, voxel: f32, cycle: bool) -> String {
    let s = posed(key);
    // Body and hakama surfaced apart so the legs exist under the (see-through) hakama.
    let (body, skirt): (Vec<Shape>, Vec<Shape>) = s.caps.iter().copied().partition(|c| c.kind != Kind::Skirt);
    let mut m = surface(&body, voxel);
    let body_verts = m.pos.len();
    let cloth = surface(&skirt, voxel);
    m.idx.extend(cloth.idx.iter().map(|i| i + body_verts as u32));
    m.pos.extend(&cloth.pos);
    m.nrm.extend(&cloth.nrm);
    // owning shape per vertex: the viewer inks the seams where ownership changes
    let owner: Vec<usize> =
        m.pos.iter().enumerate().map(|(i, w)| if i < body_verts { nearest_shape(*w, &body) } else { usize::MAX >> 1 }).collect();
    let kind: Vec<u8> = (0..m.pos.len()).map(|i| (i >= body_verts) as u8).collect();
    // crease: concavity of the field (negative Laplacian of the SDF ~ valley between two masses)
    let e = 0.08;
    let crease: Vec<String> = m
        .pos
        .iter()
        .map(|&w| {
            let f = |d: Vec3| scene_sdf(w + d * e, &body);
            let lap = f(Vec3::X) + f(-Vec3::X) + f(Vec3::Y) + f(-Vec3::Y) + f(Vec3::Z) + f(-Vec3::Z) - 6.0 * f(Vec3::ZERO);
            format!("{:.2}", (-lap / (e * e)).max(0.0))
        })
        .collect();
    format!(
        "{{\"name\":\"{}\",\"cycle\":{cycle},\"pos\":[{}],\"nrm\":[{}],\"kind\":[{}],\"owner\":[{}],\"crease\":[{}],\"idx\":[{}],\"lines\":[{}],\"props\":[{}],\"bones\":[{}]}}",
        key.name,
        floats(&m.pos),
        floats(&m.nrm),
        ints(&kind),
        ints(&owner),
        crease.join(","),
        ints(&m.idx),
        line_json(&s, false),
        props_json(&s),
        // joint -> parent segments, for the viewer's ?bones overlay
        RIG.iter()
            .enumerate()
            .filter_map(|(i, j)| j.parent.map(|p| floats(&[s.pos[p], s.pos[i]])))
            .collect::<Vec<_>>()
            .join(",")
    )
}

const SKIN_FALLOFF: f32 = 0.15;
const SKIRT_DRAPE: f32 = 0.6;
const R_HIP: usize = 14;
const L_HIP: usize = 18;

// Up to 4 bones per vertex: each carrying bone scores exp(-(d - dmin) / falloff) from its nearest shape.
// Blend shape as per-vertex deltas on the base topology: slide each vertex along the new field's gradient onto its zero set.
fn morph_deltas(base: &Surface, caps: &[Shape]) -> String {
    let e = 0.01;
    let sdf = |p: Vec3| scene_sdf(p, caps);
    let deltas: Vec<Vec3> = base
        .pos
        .iter()
        .map(|&p0| {
            let mut p = p0;
            for _ in 0..6 {
                let g = Vec3::new(
                    sdf(p + Vec3::X * e) - sdf(p - Vec3::X * e),
                    sdf(p + Vec3::Y * e) - sdf(p - Vec3::Y * e),
                    sdf(p + Vec3::Z * e) - sdf(p - Vec3::Z * e),
                )
                .normalize_or(Vec3::Y);
                p -= g * sdf(p);
            }
            p - p0
        })
        .collect();
    floats(&deltas)
}

fn skin_part(caps: &[Shape], voxel: f32, falloff: f32, drape: bool, morphs: &[(&str, Vec<Shape>)]) -> String {
    let m = surface(caps, voxel);
    let (mut skin_i, mut skin_w) = (Vec::new(), Vec::new());
    for w in &m.pos {
        let mut best = vec![f32::INFINITY; RIG.len()];
        for c in caps {
            best[c.bone] = best[c.bone].min(sd_shape(*w, c));
        }
        // The hakama rides the pelvis but lets the thighs pull it along.
        if drape {
            let waist = (w.y - 4.0).max(0.0) * 0.8;
            best[R_HIP] = (w.x + 0.6).abs() + waist;
            best[L_HIP] = (w.x - 0.6).abs() + waist;
        }
        let dmin = best.iter().cloned().fold(f32::INFINITY, f32::min);
        let mut ranked: Vec<(usize, f32)> =
            best.iter().enumerate().map(|(b, d)| (b, (-(d - dmin) / falloff).exp())).collect();
        ranked.sort_by(|a, b| b.1.total_cmp(&a.1));
        let total: f32 = ranked[..4].iter().map(|r| r.1).sum();
        for (b, wt) in &ranked[..4] {
            skin_i.push(*b as u32);
            skin_w.push(format!("{:.3}", wt / total));
        }
    }
    let morph_json: Vec<String> =
        morphs.iter().map(|(name, c)| format!("\"{name}\":[{}]", morph_deltas(&m, c))).collect();
    format!(
        "{{\"pos\":[{}],\"nrm\":[{}],\"idx\":[{}],\"skinIndex\":[{}],\"skinWeight\":[{}],\"morphs\":{{{}}}}}",
        floats(&m.pos),
        floats(&m.nrm),
        ints(&m.idx),
        ints(&skin_i),
        skin_w.join(","),
        morph_json.join(",")
    )
}

// Bind pose: the body without the hakama (legs stay visible), and the hakama as its own draped mesh.
fn skin_json(voxel: f32) -> String {
    let rest = Frame::from_key(&BIND);
    let s = posed(&rest);
    let body_of = |caps: &[Shape]| caps.iter().filter(|c| c.kind != Kind::Skirt).copied().collect::<Vec<_>>();
    let body = body_of(&s.caps);
    let unit = [
        ("trapHeight", Tweak { trap_height: 0.5, ..Tweak::default() }),
        ("trapMass", Tweak { trap_mass: 0.2, ..Tweak::default() }),
        ("trapReach", Tweak { trap_reach: 0.35, ..Tweak::default() }),
        ("trapSlope", Tweak { trap_slope: 0.35, ..Tweak::default() }),
        ("deltMass", Tweak { delt_mass: 0.2, ..Tweak::default() }),
        ("deltLength", Tweak { delt_length: 0.4, ..Tweak::default() }),
        ("deltOut", Tweak { delt_out: 0.2, ..Tweak::default() }),
        ("deltCap", Tweak { delt_cap: 0.4, ..Tweak::default() }),
    ];
    let morphs: Vec<(&str, Vec<Shape>)> =
        unit.iter().map(|(name, t)| (*name, body_of(&posed_with(&rest, *t).caps))).collect();
    let skirt: Vec<Shape> = s.caps.iter().filter(|c| c.kind == Kind::Skirt).copied().collect();
    let joints: Vec<String> = RIG
        .iter()
        .zip(&rest.rots)
        .map(|(j, r)| {
            format!(
                "{{\"name\":\"{}\",\"parent\":{},\"offset\":[{},{},{}],\"rest\":[{},{},{}]}}",
                j.name,
                j.parent.map_or(-1, |p| p as i32),
                j.offset.x,
                j.offset.y,
                j.offset.z,
                r.pitch,
                r.spread,
                r.yaw
            )
        })
        .collect();
    format!(
        "{{\"joints\":[{}],\"body\":{},\"skirt\":{},\"lines\":[{}],\"props\":[{}]}}",
        joints.join(","),
        skin_part(&body, voxel, SKIN_FALLOFF, false, &morphs),
        skin_part(&skirt, voxel, SKIRT_DRAPE, true, &[]),
        line_json(&s, true),
        props_json(&s)
    )
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let which = args.first().map(String::as_str).unwrap_or("html");
    let yaw: f32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let w: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(64);
    let h: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(50);
    if which == "skin" {
        let voxel: f32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(0.2);
        let page = include_str!("maha_skin.html").replace("__RIG__", &skin_json(voxel));
        let path = "docs/maha_rig/skin.html";
        std::fs::write(path, page).expect("write skin viewer");
        println!("{path}");
        return;
    }
    if which == "viewer" {
        let voxel: f32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(0.2);
        let mut keys: Vec<String> = keyframes().iter().map(|k| mesh_json(k, voxel, false)).collect();
        keys.extend(walk_cycle().iter().map(|k| mesh_json(k, voxel, true)));
        let page = include_str!("maha_viewer.html").replace("__KEYS__", &format!("[{}]", keys.join(",")));
        let path = "docs/maha_rig/viewer.html";
        std::fs::write(path, page).expect("write viewer");
        println!("{path}");
        return;
    }
    if which == "probe" {
        // half-width of the body (arms excluded) at each height, front view
        let rest = Frame::from_key(&KEYS[0]);
        let s = posed(&rest);
        let trunk: Vec<Shape> = s.caps.iter().filter(|c| ![6, 7, 8, 10, 11, 12].contains(&c.bone) && c.kind != Kind::Skirt).copied().collect();
        for i in 0..30 {
            let y = 11.0 - i as f32 * 0.25;
            let mut x = 0.0;
            for k in 0..200 { let xx = k as f32 * 0.02; if (0..40).any(|z| scene_sdf(Vec3::new(xx, y, -2.0 + z as f32 * 0.1), &trunk) < 0.0) { x = xx; } }
            println!("{y:5.2} {x:4.2} {}", "#".repeat((x * 20.0) as usize));
        }
        return;
    }
    if which == "html" {
        let path = "docs/maha_rig/rig.html";
        std::fs::write(path, html(yaw, w, h)).expect("write html");
        println!("{path}");
        return;
    }
    let all = keyframes();
    let keys: Vec<&Frame> = match which.parse::<usize>() {
        Ok(i) => vec![&all[i.min(all.len() - 1)]],
        Err(_) => all.iter().collect(),
    };
    let frames: Vec<Vec<String>> = keys.iter().map(|k| render(k, yaw, w, h)).collect();
    for row in 0..=h {
        let line: Vec<&str> = frames.iter().map(|f| f[row].as_str()).collect();
        println!("{}", line.join(" ").trim_end());
    }
}
