//! Mahoraga rig preview: linked joint matrices, smooth-blended round cones, wire + contour SVG + ASCII.
//! cargo run --example 3_maha_rig -- [html|all|key 0-3] [yaw_deg] [width] [height]

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
    j("pelvis", None, 0.0, 5.8, 0.0, 0.0),
    j("spine", Some(0), 0.0, 1.3, 0.0, 0.95),
    j("chest", Some(1), 0.0, 1.6, 0.1, 1.25),
    j("neck", Some(2), 0.0, 1.0, 0.25, 0.6),
    j("head", Some(3), 0.0, 0.7, 0.45, 0.74),
    j("crown", Some(4), 0.0, 1.0, -0.2, 0.84),
    j("r_shoulder", Some(2), -2.3, 0.2, 0.0, 0.7),
    j("r_elbow", Some(6), 0.0, -2.35, 0.0, 0.55),
    j("r_wrist", Some(7), 0.0, -2.2, 0.0, 0.44),
    j("r_fist", Some(8), 0.0, -0.8, 0.0, 0.48),
    j("l_shoulder", Some(2), 2.3, 0.2, 0.0, 0.7),
    j("l_elbow", Some(10), 0.0, -2.35, 0.0, 0.55),
    j("l_wrist", Some(11), 0.0, -2.2, 0.0, 0.44),
    j("l_fist", Some(12), 0.0, -0.8, 0.0, 0.48),
    j("r_hip", Some(0), -0.8, -0.3, 0.0, 0.85),
    j("r_knee", Some(14), 0.0, -2.6, 0.0, 0.62),
    j("r_ankle", Some(15), 0.0, -2.5, 0.0, 0.45),
    j("r_toe", Some(16), 0.0, -0.3, 0.8, 0.32),
    j("l_hip", Some(0), 0.8, -0.3, 0.0, 0.85),
    j("l_knee", Some(18), 0.0, -2.6, 0.0, 0.62),
    j("l_ankle", Some(19), 0.0, -2.5, 0.0, 0.45),
    j("l_toe", Some(20), 0.0, -0.3, 0.8, 0.32),
    // Four brow wings: an upper pair sweeping up-out, a lower pair nearly level.
    j("r_wing_hi", Some(4), -0.25, 0.4, 0.5, 0.16),
    j("r_wing_hi_mid", Some(22), -1.1, 0.55, -0.45, 0.22),
    j("r_wing_hi_tip", Some(23), -1.2, 0.75, -0.35, 0.07),
    j("l_wing_hi", Some(4), 0.25, 0.4, 0.5, 0.16),
    j("l_wing_hi_mid", Some(25), 1.1, 0.55, -0.45, 0.22),
    j("l_wing_hi_tip", Some(26), 1.2, 0.75, -0.35, 0.07),
    j("r_wing_lo", Some(4), -0.25, 0.1, 0.5, 0.15),
    j("r_wing_lo_mid", Some(28), -1.2, 0.05, -0.4, 0.2),
    j("r_wing_lo_tip", Some(29), -1.3, 0.15, -0.25, 0.07),
    j("l_wing_lo", Some(4), 0.25, 0.1, 0.5, 0.15),
    j("l_wing_lo_mid", Some(31), 1.2, 0.05, -0.4, 0.2),
    j("l_wing_lo_tip", Some(32), 1.3, 0.15, -0.25, 0.07),
];
const CROWN: usize = 5;
const PELVIS: usize = 0;
const CHEST: usize = 2;
const HEAD: usize = 4;
const R_SHOULDER: usize = 6;
const L_SHOULDER: usize = 10;
const L_ELBOW: usize = 11;
const R_ELBOW: usize = 7;
const R_WRIST: usize = 8;

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

const KEYS: &[Key] = &[
    Key { name: "K0 stand", bob: 0.0, rots: &[
        ("r_shoulder", r(0.0, 14.0)), ("l_shoulder", r(0.0, 14.0)),
        ("r_elbow", r(10.0, 0.0)), ("l_elbow", r(10.0, 0.0)),
        ("r_hip", r(0.0, 3.0)), ("l_hip", r(0.0, 3.0)),
    ]},
    Key { name: "K1 R foot plants", bob: -0.2, rots: &[
        ("pelvis", tw(4.0)), ("chest", tw(-5.0)),
        ("r_shoulder", r(-14.0, 14.0)), ("l_shoulder", r(16.0, 13.0)),
        ("r_elbow", r(6.0, 0.0)), ("l_elbow", r(22.0, 0.0)),
        ("r_hip", r(22.0, 3.0)), ("r_knee", r(-4.0, 0.0)),
        ("l_hip", r(-16.0, 3.0)), ("l_knee", r(-22.0, 0.0)),
    ]},
    Key { name: "K2 passing", bob: 0.15, rots: &[
        ("r_shoulder", r(0.0, 14.0)), ("l_shoulder", r(3.0, 14.0)),
        ("r_elbow", r(10.0, 0.0)), ("l_elbow", r(14.0, 0.0)),
        ("r_hip", r(-3.0, 3.0)), ("r_knee", r(-3.0, 0.0)),
        ("l_hip", r(16.0, 3.0)), ("l_knee", r(-45.0, 0.0)),
    ]},
    Key { name: "K3 L foot plants", bob: -0.2, rots: &[
        ("pelvis", tw(-4.0)), ("chest", tw(5.0)),
        ("l_shoulder", r(-14.0, 14.0)), ("r_shoulder", r(16.0, 13.0)),
        ("l_elbow", r(6.0, 0.0)), ("r_elbow", r(22.0, 0.0)),
        ("l_hip", r(22.0, 3.0)), ("l_knee", r(-4.0, 0.0)),
        ("r_hip", r(-16.0, 3.0)), ("r_knee", r(-22.0, 0.0)),
    ]},
];

fn local_rotation(name: &str, rot: Rot) -> Quat {
    let side = if name.starts_with("r_") { -1.0 } else { 1.0 };
    Quat::from_rotation_y(rot.yaw.to_radians())
        * Quat::from_rotation_z(side * rot.spread.to_radians())
        * Quat::from_rotation_x(-rot.pitch.to_radians())
}

// Forward kinematics: world = parent_world * T(offset) * R(local).
fn pose(key: &Key) -> Vec<Mat4> {
    let mut world: Vec<Mat4> = Vec::with_capacity(RIG.len());
    for joint in RIG {
        let rot = key.rots.iter().find(|(n, _)| *n == joint.name).map(|(_, r)| *r).unwrap_or_default();
        let mut offset = joint.offset;
        if joint.parent.is_none() {
            offset.y += key.bob;
        }
        let local = Mat4::from_rotation_translation(local_rotation(joint.name, rot), offset);
        world.push(joint.parent.map_or(local, |p| world[p] * local));
    }
    world
}

struct Shape {
    a: Vec3,
    b: Vec3,
    r1: f32,
    r2: f32,
    kind: Kind,
    core: bool,
}

fn cone(a: Vec3, b: Vec3, r1: f32, r2: f32, kind: Kind) -> Shape {
    Shape { a, b, r1, r2, kind, core: false }
}

fn core(a: Vec3, b: Vec3, r1: f32, r2: f32) -> Shape {
    Shape { a, b, r1, r2, kind: Kind::Body, core: true }
}

// Torso, neck, traps, delts melt together; limbs and head stay crisp so the arm gap and head read.
const CORE_JOINTS: &[&str] = &["spine", "chest", "neck"];

// Bones as capsules, plus muscle masses parented to chest/shoulder/pelvis frames so they ride the pose.
fn shapes(world: &[Mat4], pos: &[Vec3]) -> Vec<Shape> {
    let mut out: Vec<Shape> = RIG
        .iter()
        .enumerate()
        .filter_map(|(i, jt)| {
            jt.parent.map(|p| Shape { core: CORE_JOINTS.contains(&jt.name), ..cone(pos[p], pos[i], jt.radius, jt.radius, Kind::Body) })
        })
        .collect();
    let at = |m: usize, x: f32, y: f32, z: f32| world[m].transform_point3(Vec3::new(x, y, z));
    out.push(cone(at(HEAD, 0.0, -0.1, 0.2), at(HEAD, 0.0, -0.55, 0.35), 0.52, 0.4, Kind::Body));
    for side in [-1.0, 1.0] {
        let (shoulder, elbow) = if side < 0.0 { (R_SHOULDER, R_ELBOW) } else { (L_SHOULDER, L_ELBOW) };
        // traps slope from the neck into the delt; the delt flows down the arm, no cap
        out.push(core(at(CHEST, side * 0.45, 0.95, -0.15), at(CHEST, side * 2.0, 0.3, -0.15), 0.7, 0.7));
        out.push(core(at(shoulder, side * 0.4, -0.2, 0.05), at(shoulder, side * 0.3, -1.3, 0.1), 0.95, 0.68));
        out.push(core(at(CHEST, side * 0.25, 0.3, 0.6), at(CHEST, side * 1.4, 0.35, 0.4), 0.8, 0.7));
        out.push(core(at(CHEST, side * 1.6, -0.1, -0.1), at(CHEST, side * 0.75, -2.0, 0.0), 0.8, 0.6));
        out.push(cone(at(shoulder, 0.0, -0.9, 0.22), at(shoulder, 0.0, -1.9, 0.2), 0.72, 0.56, Kind::Body));
        out.push(cone(at(shoulder, 0.0, -0.7, -0.2), at(shoulder, 0.0, -2.1, -0.15), 0.6, 0.45, Kind::Body));
        out.push(cone(at(elbow, 0.0, -0.4, 0.05), at(elbow, 0.0, -2.2, 0.0), 0.6, 0.42, Kind::Body));
    }
    out.push(cone(at(PELVIS, 0.0, 0.6, 0.0), at(PELVIS, 0.0, -3.3, 0.1), 1.45, 2.05, Kind::Skirt));
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

const CORE_BLEND: f32 = 0.6;
const LIMB_BLEND: f32 = 0.3;

fn scene_sdf(p: Vec3, caps: &[Shape]) -> f32 {
    let (mut body, mut limbs) = (f32::INFINITY, f32::INFINITY);
    for c in caps {
        let d = sd_shape(p, c);
        if c.core { body = smin(body, d, CORE_BLEND) } else { limbs = smin(limbs, d, LIMB_BLEND) }
    }
    smin(body, limbs, 0.35)
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

#[derive(Clone, Copy, PartialEq)]
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
    let crown = world[CROWN] * Mat4::from_rotation_translation(Quat::from_rotation_x(-0.25), Vec3::new(0.0, 0.35, 0.0));
    let hub = crown.transform_point3(Vec3::ZERO);
    let (wx, wz) = (crown.transform_vector3(Vec3::X), crown.transform_vector3(Vec3::Z));
    let rim = polygon(hub, wx, wz, 1.15, 16, 0.0);
    out.push(Wire { pts: rim.clone(), kind: Kind::Wheel });
    for k in (0..16).step_by(2) {
        out.push(Wire { pts: vec![hub, rim[k]], kind: Kind::Wheel });
        out.push(Wire { pts: vec![rim[k]], kind: Kind::Joint });
    }
    // Sword of Extermination: blade continues the right forearm past the knee.
    let fore = (pos[R_WRIST] - pos[R_ELBOW]).normalize();
    let side = fore.cross(toward_cam).normalize_or(Vec3::X) * 0.12;
    let base = pos[R_WRIST] - side * 4.0;
    let tip = base + fore * 4.2;
    out.push(Wire { pts: vec![base - side, tip, base + side, base - side], kind: Kind::Blade });
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

fn posed(key: &Key) -> Posed {
    let world = pose(key);
    let pos: Vec<Vec3> = world.iter().map(|m| m.transform_point3(Vec3::ZERO)).collect();
    let caps = shapes(&world, &pos);
    Posed { world, pos, caps }
}

fn render(key: &Key, yaw: f32, w: usize, h: usize) -> Vec<String> {
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

fn svg(key: &Key, yaw: f32, pw: usize, ph: usize) -> String {
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
fn contour_svg(key: &Key, yaw: f32, pw: usize, ph: usize, step: f32) -> String {
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
    for key in KEYS {
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

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let which = args.first().map(String::as_str).unwrap_or("html");
    let yaw: f32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let w: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(64);
    let h: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(50);
    if which == "html" {
        let path = "docs/maha_rig/rig.html";
        std::fs::write(path, html(yaw, w, h)).expect("write html");
        println!("{path}");
        return;
    }
    let keys: Vec<&Key> = match which.parse::<usize>() {
        Ok(i) => vec![&KEYS[i.min(KEYS.len() - 1)]],
        Err(_) => KEYS.iter().collect(),
    };
    let frames: Vec<Vec<String>> = keys.iter().map(|k| render(k, yaw, w, h)).collect();
    for row in 0..=h {
        let line: Vec<&str> = frames.iter().map(|f| f[row].as_str()).collect();
        println!("{}", line.join(" ").trim_end());
    }
}
