//! Mahoraga rig preview: linked joint matrices, smooth-blended round cones, wire + contour SVG + ASCII.
//! cargo run --release --example 3_maha_rig -- [skin [voxel]|viewer [voxel]|html|all|key 0-4] [yaw] [w] [h]

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

// Units: feet on y=0, crown near y=10.8, 7.5 heads tall, shoulders 3 heads wide.
// Figure faces +z (toward camera), so the character's right side sits at -x (screen left).
const RIG: &[Joint] = &[
    j("pelvis", None, 0.0, 6.35, 0.0, 0.0),
    j("spine", Some(0), 0.0, 1.3, 0.0, 0.95),
    j("chest", Some(1), 0.0, 1.6, 0.1, 1.15),
    j("neck", Some(2), 0.0, 1.0, 0.25, 0.42),
    j("head", Some(3), 0.0, 0.5, 0.32, 0.52),
    j("crown", Some(4), 0.0, 0.7, -0.14, 0.59),
    j("r_shoulder", Some(2), -1.25, 0.35, 0.0, 0.45),
    j("r_elbow", Some(6), 0.0, -2.14, 0.0, 0.4),
    j("r_wrist", Some(7), 0.0, -1.68, 0.0, 0.25),
    j("r_fist", Some(8), 0.0, -0.7, 0.0, 0.3),
    j("l_shoulder", Some(2), 1.25, 0.35, 0.0, 0.45),
    j("l_elbow", Some(10), 0.0, -2.14, 0.0, 0.4),
    j("l_wrist", Some(11), 0.0, -1.68, 0.0, 0.25),
    j("l_fist", Some(12), 0.0, -0.7, 0.0, 0.3),
    j("r_hip", Some(0), -0.6, -0.3, 0.0, 0.7),
    j("r_knee", Some(14), 0.0, -2.82, 0.0, 0.62),
    j("r_ankle", Some(15), 0.0, -2.83, 0.0, 0.4),
    j("r_toe", Some(16), 0.0, -0.3, 0.95, 0.32),
    j("l_hip", Some(0), 0.6, -0.3, 0.0, 0.7),
    j("l_knee", Some(18), 0.0, -2.82, 0.0, 0.62),
    j("l_ankle", Some(19), 0.0, -2.83, 0.0, 0.4),
    j("l_toe", Some(20), 0.0, -0.3, 0.95, 0.32),
    // Four brow wings: an upper pair sweeping up-out, a lower pair nearly level.
    j("r_wing_hi", Some(4), -0.21, 0.32, 0.28, 0.18),
    j("r_wing_hi_mid", Some(22), -0.49, 0.49, -0.63, 0.21),
    j("r_wing_hi_tip", Some(23), -0.42, 0.63, -0.77, 0.07),
    j("l_wing_hi", Some(4), 0.21, 0.32, 0.28, 0.18),
    j("l_wing_hi_mid", Some(25), 0.49, 0.49, -0.63, 0.21),
    j("l_wing_hi_tip", Some(26), 0.42, 0.63, -0.77, 0.07),
    j("r_wing_lo", Some(4), -0.21, 0.1, 0.28, 0.17),
    j("r_wing_lo_mid", Some(28), -0.6, 0.07, -0.63, 0.2),
    j("r_wing_lo_tip", Some(29), -0.56, 0.07, -0.84, 0.07),
    j("l_wing_lo", Some(4), 0.21, 0.1, 0.28, 0.17),
    j("l_wing_lo_mid", Some(31), 0.6, 0.07, -0.63, 0.2),
    j("l_wing_lo_tip", Some(32), 0.56, 0.07, -0.84, 0.07),
];
const CROWN: usize = 5;
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
    Key { name: "K0 stand", bob: 0.0, rots: &[
        ("r_shoulder", r(0.0, 32.0)), ("l_shoulder", r(0.0, 32.0)),
        ("r_elbow", r(10.0, 0.0)), ("l_elbow", r(10.0, 0.0)),
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
    // joint whose world matrix carries this shape rigidly (skinning bone)
    bone: usize,
}

fn cone(a: Vec3, b: Vec3, r1: f32, r2: f32, kind: Kind) -> Shape {
    Shape { a, b, r1, r2, kind, core: false, bone: 0 }
}

fn core(a: Vec3, b: Vec3, r1: f32, r2: f32) -> Shape {
    Shape { a, b, r1, r2, kind: Kind::Body, core: true, bone: 0 }
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
                Shape { core, bone: p, ..cone(pos[p], pos[i], jt.radius, jt.radius, Kind::Body) }
            })
        })
        .collect();
    let at = |m: usize, x: f32, y: f32, z: f32| world[m].transform_point3(Vec3::new(x, y, z));
    out.push(cone(at(HEAD, 0.0, -0.07, 0.14), at(HEAD, 0.0, -0.39, 0.25), 0.36, 0.28, Kind::Body).on(HEAD));
    for side in [-1.0, 1.0] {
        let (shoulder, elbow) = if side < 0.0 { (R_SHOULDER, R_ELBOW) } else { (L_SHOULDER, L_ELBOW) };
        // trapezius as a fan of bundles: spine origins (skull base down to upper back) to clavicle/acromion
        for k in 0..TRAP_BUNDLES {
            let f = k as f32 / (TRAP_BUNDLES - 1) as f32;
            // upper bundle runs skull base to acromion at about 45 degrees; lower ones fan down the back
            let origin = at(CHEST, side * 0.28 * (1.0 - f), 1.55 - 1.4 * f + t.trap_height * (1.0 - f), -0.3 - 0.35 * f);
            let insert = at(CHEST, side * (1.2 - 0.2 * f + t.trap_reach), 0.5 - 0.1 * f - t.trap_slope, -0.05 - 0.3 * f);
            let r = 0.34 - 0.1 * f + t.trap_mass;
            out.push(core(origin, insert, r, r * 0.8).on(CHEST));
        }
        out.push(
            cone(
                at(shoulder, side * (0.12 + t.delt_out), -0.1, 0.03),
                at(shoulder, side * (0.06 + t.delt_out * 0.5), -1.0 - t.delt_length, 0.03),
                0.45 + t.delt_mass,
                0.35 + t.delt_mass * 0.2,
                Kind::Body,
            )
            .on(shoulder),
        );
        // delt cap: a ball on the acromion that swells up and out, not along the arm
        let cap = at(shoulder, side * (0.18 + t.delt_out + t.delt_cap * 0.4), 0.0 + t.delt_cap * 0.5, 0.0);
        out.push(cone(cap, cap + Vec3::Y * 0.01, 0.35 + t.delt_cap * 0.35, 0.35 + t.delt_cap * 0.35, Kind::Body).on(shoulder));
        out.push(core(at(CHEST, side * 0.2, 0.3, 0.55), at(CHEST, side * 0.85, 0.35, 0.4), 0.5, 0.45).on(CHEST));
        out.push(core(at(CHEST, side * 0.72, -0.25, -0.3), at(CHEST, side * 0.5, -2.0, -0.05), 0.42, 0.33).on(CHEST));
        out.push(cone(at(shoulder, 0.0, -0.8, 0.15), at(shoulder, 0.0, -1.7, 0.13), 0.43, 0.34, Kind::Body).on(shoulder));
        out.push(cone(at(shoulder, 0.0, -0.6, -0.14), at(shoulder, 0.0, -1.9, -0.1), 0.36, 0.27, Kind::Body).on(shoulder));
        out.push(cone(at(elbow, 0.0, -0.3, 0.03), at(elbow, 0.0, -1.6, 0.0), 0.34, 0.22, Kind::Body).on(elbow));
    }
    out.push(cone(at(PELVIS, 0.0, 0.6, 0.0), at(PELVIS, 0.0, -3.5, 0.1), 1.3, 1.8, Kind::Skirt).on(PELVIS));
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

fn scene_sdf(p: Vec3, caps: &[Shape]) -> f32 {
    let (mut body, mut limbs) = (f32::INFINITY, f32::INFINITY);
    for c in caps {
        let d = sd_shape(p, c);
        if c.core { body = smin(body, d, CORE_BLEND) } else { limbs = smin(limbs, d, LIMB_BLEND) }
    }
    smin(body, limbs, 0.2)
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
    let m = surface(&s.caps, voxel);
    let kind: Vec<u8> = m.pos.iter().map(|w| (s.caps[nearest_shape(*w, &s.caps)].kind == Kind::Skirt) as u8).collect();
    format!(
        "{{\"name\":\"{}\",\"cycle\":{cycle},\"pos\":[{}],\"nrm\":[{}],\"kind\":[{}],\"idx\":[{}],\"lines\":[{}]}}",
        key.name,
        floats(&m.pos),
        floats(&m.nrm),
        ints(&kind),
        ints(&m.idx),
        line_json(&s, false)
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
    let rest = Frame::from_key(&KEYS[0]);
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
        "{{\"joints\":[{}],\"body\":{},\"skirt\":{},\"lines\":[{}]}}",
        joints.join(","),
        skin_part(&body, voxel, SKIN_FALLOFF, false, &morphs),
        skin_part(&skirt, voxel, SKIRT_DRAPE, true, &[]),
        line_json(&s, true)
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
