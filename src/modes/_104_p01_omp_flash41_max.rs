//! heliotrope: a vine grown by 3D space colonization toward a dawn sun, thinned
//! by the shade it casts on itself while a turning camera reveals that depth.
use crate::_0_profile::measure_layer;
use crate::color::{darken, lerp_color, lighten};
use crate::opts::param_f32;
use crate::registry::{AnimKind, Mode, ModeFrame, Param};
use crate::types::{Cell, Grid};
use crossterm::style::Color;
use std::cell::RefCell;
use std::f32::consts::{PI, TAU};

pub(super) struct Heliotrope;
pub(super) static MODE: Heliotrope = Heliotrope;

const NAME: &str = "heliotrope";
const KNOBS: usize = 12;
const HELP: &str = "heliotrope: a sun-chasing vine grown in 3d, thinned by its own shade [lean] [crown] [fork] [crowd] [light] [prune] [leaf] [sap] [sun] [cam] [haze] [aspect]";

/// Wood reads by screen slope: level, backslant, plumb, foreslash.
const WOOD: [char; 4] = ['-', '\\', '|', '/'];
/// A limb carrying the whole stand renders as a solid bar.
const BARK: [char; 4] = ['=', '#', '#', '#'];
/// Growth too thin to read as structure becomes dots and slants.
const TWIG: [char; 4] = ['.', '`', ';', '\''];
/// Leaf ramp, deepest shade first: a canopy speck, then a bud at full sun.
const LEAF: [char; 4] = [',', '*', 'o', 'O'];
const BUD: char = '@';
/// Sap beads climbing the lineage from root to tip.
const SAP: [char; 3] = ['.', 'o', '*'];
const SUN_CORE: char = '@';
const SUN_RIM: char = '%';
const CORONA: [char; 4] = ['+', '*', 'x', ':'];
const RAY: [char; 3] = ['.', ',', '\''];
/// Cast shadow on the soil, thin to deep.
const SHADE: [char; 3] = ['\u{2591}', '\u{2592}', '\u{2593}'];
const SOIL: [char; 3] = ['.', ',', '\''];
const MOTE: [char; 2] = ['.', '`'];

const GOLDEN: f32 = 2.399_963_2;
/// World half extent in x and z, and the height of the density box.
const BX: f32 = 1.45;
const BY: f32 = 1.62;
const FX: usize = 26;
const FY: usize = 22;
/// Lattice spacing of the occlusion march.
const MARCH: f32 = 0.055;
/// How much optical depth counts as full shade.
const SHADE_GAIN: f32 = 3.0;

const L_ATTR: u64 = 0x31;
const L_STEP: u64 = 0x32;
const L_ROLL: u64 = 0x33;
const L_MOTE: u64 = 0x34;
const L_SOIL: u64 = 0x35;
const L_TUFT: u64 = 0x36;
const L_SUN: u64 = 0x37;
const L_CAM: u64 = 0x38;
const L_LEAF: u64 = 0x39;
const L_TIE: u64 = 0x3A;

/// Growth light is refreshed on this stride; between refreshes the tips read the
/// occlusion the last refresh measured.
const LIGHT_EVERY: usize = 6;
/// A limb forks while its tier is under this cap, so the stand keeps a readable
/// trunk-limb-twig hierarchy instead of packing into a bush.
const MAX_TIER: u8 = 4;

const PARAMS: &[Param] = &[
    param!("LEAN", "growth toward the sun", 0.0, 2.0, 0.85, 0.05),
    param!("CROWN", "crown extent", 0.50, 1.40, 1.00, 0.02),
    param!("FORK", "limb division", 0.15, 1.20, 0.55, 0.05),
    param!("CROWD", "occupancy avoidance", 0.0, 1.0, 0.35, 0.05),
    param!("LIGHT", "self shading gain", 0.0, 3.0, 1.30, 0.05),
    param!("PRUNE", "shade thinning", 0.0, 1.0, 0.42, 0.02),
    param!("LEAF", "leaf phyllotaxis", 0.0, 1.50, 0.85, 0.05),
    param!("SAP", "sap flow speed", 0.0, 3.0, 1.00, 0.05),
    param!("SUN", "sun drift rad/s", -1.0, 1.0, 0.10, 0.01),
    param!("CAM", "camera turn rad/s", -1.0, 1.0, 0.14, 0.01),
    param!("HAZE", "depth haze", 0.0, 1.20, 0.55, 0.05),
    param!("ASPECT", "cols per row", 0.50, 3.00, 2.00, 0.05),
];

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
fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Three floats in [-1, 1] keyed to an object identity.
#[inline]
fn jitter(seed: u64, index: u64, layer: u64) -> V3 {
    V3::new(
        unit(hash(seed, layer, index, 0)) * 2.0 - 1.0,
        unit(hash(seed, layer, index, 1)) * 2.0 - 1.0,
        unit(hash(seed, layer, index, 2)) * 2.0 - 1.0,
    )
}

#[derive(Clone, Copy, Default)]
struct V3 {
    x: f32,
    y: f32,
    z: f32,
}

impl V3 {
    #[inline]
    fn new(x: f32, y: f32, z: f32) -> Self {
        V3 { x, y, z }
    }
    #[inline]
    fn add(self, o: V3) -> V3 {
        V3::new(self.x + o.x, self.y + o.y, self.z + o.z)
    }
    #[inline]
    fn sub(self, o: V3) -> V3 {
        V3::new(self.x - o.x, self.y - o.y, self.z - o.z)
    }
    #[inline]
    fn mul(self, s: f32) -> V3 {
        V3::new(self.x * s, self.y * s, self.z * s)
    }
    #[inline]
    fn dot(self, o: V3) -> f32 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }
    #[inline]
    fn cross(self, o: V3) -> V3 {
        V3::new(
            self.y * o.z - self.z * o.y,
            self.z * o.x - self.x * o.z,
            self.x * o.y - self.y * o.x,
        )
    }
    #[inline]
    fn len(self) -> f32 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }
    #[inline]
    fn norm(self) -> V3 {
        let l = self.len();
        if l > 1e-6 {
            self.mul(1.0 / l)
        } else {
            V3::new(0.0, 1.0, 0.0)
        }
    }
}

/// Rodrigues rotation of `v` around a unit `axis`.
fn rotate(v: V3, axis: V3, angle: f32) -> V3 {
    let (s, c) = angle.sin_cos();
    v.mul(c)
        .add(axis.cross(v).mul(s))
        .add(axis.mul(axis.dot(v) * (1.0 - c)))
}

/// Unit vector perpendicular to `d`, rolled by `roll` around `d`.
fn roll_axis(d: V3, roll: f32) -> V3 {
    let up = if d.y.abs() < 0.9 {
        V3::new(0.0, 1.0, 0.0)
    } else {
        V3::new(1.0, 0.0, 0.0)
    };
    let a = d.cross(up).norm();
    let b = d.cross(a).norm();
    a.mul(roll.cos()).add(b.mul(roll.sin())).norm()
}

#[derive(Default)]
struct Field {
    data: Vec<f32>,
    nx: usize,
    ny: usize,
    nz: usize,
}

impl Field {
    fn ensure(&mut self, nx: usize, ny: usize, nz: usize) {
        self.nx = nx;
        self.ny = ny;
        self.nz = nz;
        self.data.resize(nx * ny * nz, 0.0);
    }
    fn clear(&mut self) {
        self.data.iter_mut().for_each(|v| *v = 0.0);
    }
    #[inline]
    fn band(v: f32, extent: f32, n: usize) -> usize {
        (((v + extent) / (2.0 * extent)).clamp(0.0, 0.999_999) * n as f32) as usize
    }
    #[inline]
    fn layer(v: f32, n: usize) -> usize {
        ((v / BY).clamp(0.0, 0.999_999) * n as f32) as usize
    }
    #[inline]
    fn at(&self, p: V3) -> f32 {
        let i = Self::band(p.x, BX, self.nx);
        let j = Self::layer(p.y, self.ny);
        let k = Self::band(p.z, BX, self.nz);
        self.data[(j * self.nz + k) * self.nx + i]
    }
    #[inline]
    fn linear(&self, p: V3) -> usize {
        let i = Self::band(p.x, BX, self.nx);
        let j = Self::layer(p.y, self.ny);
        let k = Self::band(p.z, BX, self.nz);
        (j * self.nz + k) * self.nx + i
    }
    fn splat(&mut self, p: V3, w: f32) {
        let (ci, cj, ck) = (
            Self::band(p.x, BX, self.nx) as isize,
            Self::layer(p.y, self.ny) as isize,
            Self::band(p.z, BX, self.nz) as isize,
        );
        for dj in -1..=1isize {
            for dk in -1..=1isize {
                for di in -1..=1isize {
                    let (i, j, k) = (ci + di, cj + dj, ck + dk);
                    if i < 0
                        || j < 0
                        || k < 0
                        || i as usize >= self.nx
                        || j as usize >= self.ny
                        || k as usize >= self.nz
                    {
                        continue;
                    }
                    let fall = 1.0 - 0.6 * (di.abs() + dj.abs() + dk.abs()) as f32;
                    if fall <= 0.0 {
                        continue;
                    }
                    let idx = (j as usize * self.nz + k as usize) * self.nx + i as usize;
                    self.data[idx] += w * fall;
                }
            }
        }
    }
    /// Optical depth from `from` toward `dir`, walked on a fixed lattice.
    #[inline]
    fn optical(&self, from: V3, dir: V3, steps: usize) -> f32 {
        let mut p = from;
        let mut tau = 0.0;
        for _ in 0..steps {
            p = p.add(dir.mul(MARCH));
            if p.y < 0.0 || p.y > BY || p.x.abs() > BX || p.z.abs() > BX {
                break;
            }
            tau += self.at(p);
        }
        tau * MARCH
    }
    /// Central difference of the density: the occupancy field's own push.
    fn gradient(&self, p: V3, out: &mut V3) {
        let d = 0.07;
        out.x = self.at(p.add(V3::new(d, 0.0, 0.0))) - self.at(p.sub(V3::new(d, 0.0, 0.0)));
        out.y = self.at(p.add(V3::new(0.0, d, 0.0))) - self.at(p.sub(V3::new(0.0, d, 0.0)));
        out.z = self.at(p.add(V3::new(0.0, 0.0, d))) - self.at(p.sub(V3::new(0.0, 0.0, d)));
    }
}

#[derive(Clone, Copy, Default)]
struct Node {
    pos: V3,
    dir: V3,
    parent: u32,
    light: f32,
    sugar: f32,
    thick: f32,
    along: f32,
    since: f32,
    avg: f32,
    leaves: u16,
    tier: u8,
    voxel: u32,
}

#[derive(Default)]
struct Stand {
    nodes: Vec<Node>,
    tips: Vec<u32>,
    next: Vec<u32>,
    attr: Vec<V3>,
    alive: Vec<bool>,
    attr_light: Vec<f32>,
    leaf: Vec<bool>,
    acc: Vec<V3>,
    lit: Vec<f32>,
    cnt: Vec<f32>,
}

struct Budget {
    steps: usize,
    tips: usize,
    nodes: usize,
    attractors: usize,
}

/// Work scales with the square root of the cell count, so a larger terminal
/// resolves the same stand with more limbs, leaves and texture.
fn budget(w: usize, h: usize) -> Budget {
    let scale = ((w * h) as f32 / 1920.0).sqrt().clamp(0.65, 2.2);
    Budget {
        steps: 130,
        tips: (14.0 * scale) as usize,
        nodes: (300.0 * scale) as usize,
        attractors: (70.0 * scale) as usize,
    }
}

#[derive(Clone, Copy)]
struct Knobs {
    lean: f32,
    crown: f32,
    fork: f32,
    crowd: f32,
    light: f32,
    prune: f32,
    leaf: f32,
    sap: f32,
    sun: f32,
    cam: f32,
    haze: f32,
    aspect: f32,
}

impl Knobs {
    fn new(p: &[f32; KNOBS]) -> Self {
        Knobs {
            lean: p[0],
            crown: p[1],
            fork: p[2],
            crowd: p[3],
            light: p[4],
            prune: p[5],
            leaf: p[6],
            sap: p[7],
            sun: p[8],
            cam: p[9],
            haze: p[10],
            aspect: p[11],
        }
    }
}

/// One frame's resolved camera, sun, palette roles and scale.
struct Look {
    k: Knobs,
    seed: u64,
    w: usize,
    h: usize,
    budget: Budget,
    ppu: f32,
    cx: f32,
    cy: f32,
    az: f32,
    tilt: f32,
    aspect: f32,
    sun: V3,
    sun_seed: V3,
    sun_px: f32,
    sun_py: f32,
    sun_r: f32,
    horizon: f32,
    time: f32,
    sky_top: Color,
    sky_low: Color,
    sun_core: Color,
    sun_mid: Color,
    bark_live: Color,
    bark_dead: Color,
    twig: Color,
    leaf_shade: Color,
    leaf_lit: Color,
    sap: Color,
    mote: Color,
    soil: Color,
    soil_far: Color,
    shade: Color,
}

impl Look {
    fn new(
        seed: u64,
        w: usize,
        h: usize,
        palette: &[Color; 5],
        time: f32,
        p: &[f32; KNOBS],
    ) -> Self {
        let k = Knobs::new(p);
        let aspect = k.aspect.max(0.5);
        let ppu = ((h as f32 - 1.0) / 1.35)
            .min(w as f32 / (2.3 * aspect))
            .max(1.0);
        let cx = w as f32 * 0.5;
        let cy = (h as f32 - 3.6).max(0.5);
        let tilt = 0.20;
        let horizon = (cy - 0.20 * ppu).max(1.0);

        let sun_az0 = unit(hash(seed, L_SUN, 0, 0)) * TAU;
        let elev0: f32 = 0.72;
        let sun_seed = V3::new(
            sun_az0.cos() * elev0.cos(),
            elev0.sin(),
            sun_az0.sin() * elev0.cos(),
        );
        let az = sun_az0 + k.sun * time;
        let elev = elev0 + 0.16 * (0.21 * time).sin();
        let sun = V3::new(az.cos() * elev.cos(), elev.sin(), az.sin() * elev.cos());

        let cam_off = 1.15 + 0.7 * (unit(hash(seed, L_CAM, 0, 0)) * 2.0 - 1.0);
        let cam_az = sun_az0 + cam_off + k.cam * time;

        // Sky dome: the disc hangs where the true light direction points, mapped
        // straight onto the frame so a low sun still sits inside the sky band.
        let (cs, sn) = cam_az.sin_cos();
        let sun_xr = sun.x * cs - sun.z * sn;
        let sky_h = horizon.max(1.0);
        let sun_px = cx + sun_xr * w as f32 * 0.40;
        let sun_py = sky_h * 0.10 + (1.0 - sun.y) * sky_h * 0.72;
        let sun_r = (ppu * 0.075).max(0.8);

        let accent = palette[3];
        let text = palette[4];
        Look {
            k,
            seed,
            w,
            h,
            budget: budget(w, h),
            ppu,
            cx,
            cy,
            az: cam_az,
            tilt,
            aspect,
            sun,
            sun_seed,
            sun_px,
            sun_py,
            sun_r,
            horizon,
            time,
            sky_top: darken(palette[0], 30),
            sky_low: lerp_color(palette[0], accent, 0.34),
            sun_core: lighten(accent, 90),
            sun_mid: accent,
            bark_live: lerp_color(palette[0], palette[1], 0.14),
            bark_dead: lerp_color(palette[0], text, 0.16),
            twig: lerp_color(palette[0], palette[1], 0.10),
            leaf_shade: lerp_color(palette[2], palette[0], 0.30),
            leaf_lit: lighten(lerp_color(palette[2], palette[4], 0.75), 10),
            sap: lighten(accent, 60),
            mote: lerp_color(text, palette[0], 0.55),
            soil: darken(palette[0], 8),
            soil_far: lerp_color(palette[0], accent, 0.14),
            shade: darken(palette[0], 20),
        }
    }
}

/// Column, row and camera depth of a world point. Larger depth is farther.
#[inline]
fn project(l: &Look, p: V3) -> (f32, f32, f32) {
    let (s, c) = l.az.sin_cos();
    let xr = p.x * c - p.z * s;
    let zr = p.x * s + p.z * c;
    let yr = p.y * l.tilt.cos() - zr * l.tilt.sin();
    let k = 1.0 / (1.0 + 0.10 * zr).max(0.55);
    (l.cx + xr * k * l.ppu * l.aspect, l.cy - yr * k * l.ppu, zr)
}

#[inline]
fn bg_at(grid: &Grid, x: usize, y: usize) -> Color {
    grid.get(y)
        .and_then(|row| row.get(x))
        .map_or(Color::Reset, |c| c.bg)
}

/// One glyph over whatever background is already there.
#[inline]
fn plot(grid: &mut Grid, x: f32, y: f32, ch: char, fg: Color) {
    if x < 0.0 || y < 0.0 {
        return;
    }
    let (xi, yi) = (x as usize, y as usize);
    if let Some(row) = grid.get_mut(yi) {
        if xi < row.len() {
            let bg = row[xi].bg;
            row[xi] = Cell::with_bg(ch, fg, bg);
        }
    }
}

#[inline]
fn put(grid: &mut Grid, x: i32, y: i32, cell: Cell) {
    if x < 0 || y < 0 {
        return;
    }
    if let Some(row) = grid.get_mut(y as usize) {
        if (x as usize) < row.len() {
            row[x as usize] = cell;
        }
    }
}

#[inline]
fn slope_glyph(ax: f32, ay: f32, bx: f32, by: f32, table: &[char; 4]) -> char {
    let ang = (by - ay).atan2(bx - ax);
    let bucket = (((ang + PI * 0.125) / (PI * 0.25)).floor() as i32).rem_euclid(8);
    table[(bucket & 3) as usize]
}

struct Stamp {
    depth: f32,
    ax: f32,
    ay: f32,
    bx: f32,
    by: f32,
    w: f32,
    light: f32,
    surv: f32,
    phase: f32,
    kind: u8,
}

#[derive(Default)]
struct Scratch {
    field: Field,
    stand: Stand,
    stamps: Vec<Stamp>,
    shadow: Vec<f32>,
    bright: Vec<(u16, u16, Color)>,
}

thread_local! {
    static SCRATCH: RefCell<Scratch> = RefCell::new(Scratch::default());
}

fn draw(frame: &mut ModeFrame<'_>, p: &[f32; KNOBS]) {
    let (w, h) = (frame.width, frame.height);
    if w == 0 || h == 0 {
        return;
    }
    let look = Look::new(frame.seed, w, h, frame.palette, frame.time, p);
    let grid = &mut *frame.grid;
    SCRATCH.with(|slot| {
        let sc = &mut *slot.borrow_mut();
        sc.field.ensure(FX, FY, FX);
        sc.shadow.clear();
        sc.shadow.resize(w * h, 0.0);
        sc.bright.clear();
        measure_layer(NAME, "grow", || {
            let sc = &mut *sc;
            grow(&look, &mut sc.field, &mut sc.stand);
            solve(&look, &mut sc.field, &mut sc.stand);
        });
        measure_layer(NAME, "sky", || {
            let sc = &mut *sc;
            paint_sky(grid, &look, sc);
        });
        measure_layer(NAME, "sun", || paint_sun(grid, &look));
        measure_layer(NAME, "cast", || {
            let sc = &mut *sc;
            build_shadow(&look, &sc.stand, &mut sc.shadow, w);
        });
        measure_layer(NAME, "soil", || {
            let sc = &mut *sc;
            paint_soil(grid, &look, sc);
        });
        measure_layer(NAME, "stand", || {
            let sc = &mut *sc;
            paint_stand(grid, &look, sc);
        });
        measure_layer(NAME, "sap", || {
            let sc = &mut *sc;
            paint_sap(grid, &look, &sc.stand);
        });
        measure_layer(NAME, "bloom", || {
            let sc = &mut *sc;
            paint_bloom(grid, sc);
        });
    });
}

/// Space colonization in 3D. Each step reads the wood laid down so far, through
/// the sun and through the occupancy gradient, so the stand steers out of its
/// own shade and out of its own occupied space.
fn grow(l: &Look, field: &mut Field, st: &mut Stand) {
    let k = &l.k;
    field.clear();
    st.nodes.clear();
    st.tips.clear();
    st.next.clear();
    st.attr.clear();
    st.alive.clear();
    st.attr_light.clear();

    let rad = V3::new(0.48 * k.crown, 0.38 * k.crown, 0.42 * k.crown);
    let center = V3::new(
        l.sun_seed.x * 0.30 * k.lean,
        0.60 + 0.14 * k.crown,
        l.sun_seed.z * 0.30 * k.lean,
    );
    for i in 0..l.budget.attractors as u64 {
        let r = unit(hash(l.seed, L_ATTR, i, 0)).cbrt();
        let cz = 2.0 * unit(hash(l.seed, L_ATTR, i, 1)) - 1.0;
        let sz = (1.0 - cz * cz).max(0.0).sqrt();
        let phi = TAU * unit(hash(l.seed, L_ATTR, i, 2));
        let q = V3::new(
            rad.x * r * sz * phi.cos(),
            rad.y * r * cz,
            rad.z * r * sz * phi.sin(),
        );
        st.attr.push(V3::new(
            center.x + q.x,
            (center.y + q.y).max(0.10),
            center.z + q.z,
        ));
        st.alive.push(true);
        st.attr_light.push(1.0);
    }

    st.nodes.push(Node {
        pos: V3::new(0.0, 0.0, 0.0),
        dir: V3::new(0.0, 1.0, 0.0),
        parent: u32::MAX,
        ..Default::default()
    });
    st.tips.push(0);

    let reach = 0.30 + 0.35 * k.crown;
    let step = 0.05;
    let mut forks: u64 = 0;

    for iter in 0..l.budget.steps {
        if st.tips.is_empty() || st.nodes.len() + 4 >= l.budget.nodes {
            break;
        }
        if iter % LIGHT_EVERY == 0 {
            for a in 0..st.attr.len() {
                if !st.alive[a] {
                    continue;
                }
                let tau = field.optical(st.attr[a], l.sun_seed, 26);
                st.attr_light[a] = 1.0 / (1.0 + k.light * SHADE_GAIN * tau);
            }
        }
        st.next.clear();
        st.acc.clear();
        st.acc.resize(st.tips.len(), V3::default());
        st.lit.clear();
        st.lit.resize(st.tips.len(), 0.0);
        st.cnt.clear();
        st.cnt.resize(st.tips.len(), 0.0);
        // Every live attractor commands the nearest shoot inside its reach, so
        // shoots claim territory instead of all converging on the same crowd.
        for a in 0..st.attr.len() {
            if !st.alive[a] {
                continue;
            }
            let mut best = usize::MAX;
            let mut best_d = reach;
            for (t, &ti) in st.tips.iter().enumerate() {
                let d = st.attr[a].sub(st.nodes[ti as usize].pos).len();
                let d = d * (0.9 + 0.2 * unit(hash(l.seed, L_TIE, a as u64, ti as u64)));
                if d < best_d {
                    best_d = d;
                    best = t;
                }
            }
            if best == usize::MAX {
                continue;
            }
            let ti = st.tips[best] as usize;
            let w = st.attr_light[a] / best_d.max(0.06);
            st.acc[best] = st.acc[best].add(st.attr[a].sub(st.nodes[ti].pos).norm().mul(w));
            st.lit[best] += st.attr_light[a];
            st.cnt[best] += 1.0;
        }
        for t in 0..st.tips.len() {
            let ti = st.tips[t] as usize;
            let pos = st.nodes[ti].pos;
            if st.cnt[t] == 0.0 {
                continue;
            }
            let exposure = st.lit[t] / st.cnt[t];
            let mut want = st.acc[t].norm();
            // Only a shoot that is actually starved of light reaches for it, so
            // well-lit limbs keep the form the attractors gave them.
            let starved = (1.0 - exposure).max(0.0);
            want = want
                .add(l.sun_seed.mul(k.lean * 1.3 * starved * starved))
                .norm();
            let mut g = V3::default();
            field.gradient(pos, &mut g);
            let gmag = g.len();
            if gmag > 1e-5 {
                want = want.sub(g.mul(k.crowd * 1.2 / gmag)).norm();
            }
            want = want.add(jitter(l.seed, ti as u64, L_STEP).mul(0.11)).norm();
            let mut cand = pos.add(want.mul(step));
            // A tip is free to move inside the voxel it already owns; only a
            // different voxel that is much denser than its own trail stops it.
            let margin = 1.0 + 4.0 * (1.0 - k.crowd);
            let baseline = field.at(pos).max(1.0);
            let mut tries = 0;
            while tries < 3
                && field.linear(cand) != st.nodes[ti].voxel as usize
                && field.at(cand) > baseline + margin
            {
                field.gradient(pos, &mut g);
                let gmag = g.len();
                if gmag > 1e-5 {
                    want = want.sub(g.mul(0.55 / gmag)).norm();
                }
                cand = pos.add(want.mul(step));
                tries += 1;
            }
            let blocked = field.linear(cand) != st.nodes[ti].voxel as usize
                && field.at(cand) > baseline + margin;
            if cand.y < 0.0
                || cand.y > BY - 0.06
                || cand.x.abs() > BX - 0.06
                || cand.z.abs() > BX - 0.06
                || blocked
            {
                continue;
            }
            let since = st.nodes[ti].since + step;
            let tier = st.nodes[ti].tier;
            let along = st.nodes[ti].along + step;
            let voxel = field.linear(cand) as u32;
            st.nodes.push(Node {
                pos: cand,
                dir: want,
                parent: ti as u32,
                along,
                since,
                tier,
                voxel,
                ..Default::default()
            });
            let ni = (st.nodes.len() - 1) as u32;
            if voxel != st.nodes[ti].voxel {
                field.splat(cand, 1.0);
            }
            for a in 0..st.attr.len() {
                if st.alive[a] && st.attr[a].sub(cand).len() < 0.14 {
                    st.alive[a] = false;
                }
            }
            let spacing = (0.55 - 0.30 * k.fork) / (1.0 + 0.55 * tier as f32);
            let fork_len = spacing * (1.25 - 0.5 * exposure);
            if since >= fork_len
                && tier < MAX_TIER
                && cand.y > 0.26
                && st.next.len() + 1 < l.budget.tips
                && st.nodes.len() + 4 < l.budget.nodes
            {
                let spread = 0.34 + 0.50 * k.fork;
                let roll = forks as f32 * GOLDEN + unit(hash(l.seed, L_ROLL, ni as u64, 0)) * 0.8;
                forks += 1;
                let axis = roll_axis(want, roll);
                for side in [-1.0f32, 1.0f32] {
                    let slot = if side < 0.0 { 1 } else { 2 };
                    let asym = 0.75 + 0.5 * unit(hash(l.seed, L_ROLL, ni as u64, slot));
                    let cd = rotate(want, axis, spread * side * asym).norm();
                    st.nodes.push(Node {
                        pos: cand.add(cd.mul(step * 0.5)),
                        dir: cd,
                        parent: ni,
                        along: along + step * 0.5,
                        tier: tier.saturating_add(1),
                        ..Default::default()
                    });
                    st.next.push((st.nodes.len() - 1) as u32);
                }
            } else {
                st.next.push(ni);
            }
        }
        std::mem::swap(&mut st.tips, &mut st.next);
    }
}

/// Local light for every node: how much of the sun survives the stand's own wood.
fn light_pass(st: &mut Stand, field: &Field, sun: V3, gain: f32) {
    for n in st.nodes.iter_mut() {
        let tau = field.optical(n.pos, sun, 26);
        n.light = 1.0 / (1.0 + gain * SHADE_GAIN * tau);
    }
}

/// Sugar climbs the lineage: leaves fix it, wood carries it. Thickness follows
/// the carried sugar, so the trunk is as heavy as the light the crown collects.
fn transport(st: &mut Stand) {
    st.leaf.clear();
    st.leaf.resize(st.nodes.len(), true);
    for i in 1..st.nodes.len() {
        st.leaf[st.nodes[i].parent as usize] = false;
    }
    for i in 0..st.nodes.len() {
        let n = &mut st.nodes[i];
        if st.leaf[i] {
            n.leaves = 1;
            n.sugar = n.light;
        } else {
            n.leaves = 0;
            n.sugar = n.light * 0.12;
        }
    }
    for i in (1..st.nodes.len()).rev() {
        let p = st.nodes[i].parent as usize;
        let (s, c) = (st.nodes[i].sugar, st.nodes[i].leaves);
        st.nodes[p].sugar += s;
        st.nodes[p].leaves += c;
    }
    let total = st.nodes[0].sugar.max(1e-3);
    for n in st.nodes.iter_mut() {
        let s = (n.sugar / total).clamp(0.0, 1.0);
        n.thick = 0.22 + 1.15 * s.powf(0.55);
        n.avg = if n.leaves > 0 {
            n.sugar / n.leaves as f32
        } else {
            n.light
        };
    }
}

/// Lay the stand into the field. Optical mode deposits carried mass, so a fat
/// trunk occludes more than a twig of the same length.
fn deposit(st: &Stand, field: &mut Field, optical: bool) {
    field.clear();
    for i in 1..st.nodes.len() {
        field.splat(st.nodes[i].pos, if optical { st.nodes[i].thick } else { 1.0 });
    }
}

/// Two rounds of the coupled loop: light to sugar to thickness to opacity back
/// to light. The second round is what lets the trunk shade the interior it feeds.
fn solve(l: &Look, field: &mut Field, st: &mut Stand) {
    light_pass(st, field, l.sun, l.k.light);
    transport(st);
    deposit(st, field, true);
    light_pass(st, field, l.sun, l.k.light);
    transport(st);
}

fn paint_sky(grid: &mut Grid, l: &Look, sc: &mut Scratch) {
    let rows = grid.len().min(l.h);
    let sun_gx = l.sun_px / l.aspect;
    let reach = (l.sun_r * 5.5).max(3.0);
    for y in 0..rows {
        let v = (y as f32 / l.horizon.max(1.0)).clamp(0.0, 1.0);
        let row = lerp_color(l.sky_top, l.sky_low, v * v * v);
        let band = smoothstep((v - 0.82) / 0.18) * 0.28;
        let row = lerp_color(row, l.sky_low, band);
        let dy = y as f32 - l.sun_py;
        for x in 0..grid[y].len().min(l.w) {
            let dx = x as f32 / l.aspect - sun_gx;
            let d = (dx * dx + dy * dy).sqrt();
            let glow = (0.62 / (1.0 + (d / (2.0 * l.sun_r)).powi(2)) - 0.03).clamp(0.0, 0.6);
            let bg = if d < reach {
                lerp_color(row, l.sun_mid, glow)
            } else {
                row
            };
            grid[y][x] = Cell::with_bg(' ', Color::Reset, bg);
        }
    }
    let motes = ((l.w * l.h) / 500).clamp(8, 36);
    for i in 0..motes as u64 {
        let ux = unit(hash(l.seed, L_MOTE, i, 0));
        let uy = unit(hash(l.seed, L_MOTE, i, 1));
        let uph = unit(hash(l.seed, L_MOTE, i, 2));
        let rise = (l.time * (0.012 + 0.028 * uph) + uy).rem_euclid(1.0);
        let y = (1.0 - rise) * (l.horizon - 1.0);
        let x = ux * l.w as f32 + (l.time * 0.12 + uph * TAU).sin() * 1.8;
        if y < 0.0 || x < 0.0 {
            continue;
        }
        let (xi, yi) = (x as usize, y as usize);
        if yi >= rows || xi >= grid[yi].len().min(l.w) {
            continue;
        }
        let dx = xi as f32 / l.aspect - sun_gx;
        let d = (dx * dx + (yi as f32 - l.sun_py).powi(2)).sqrt();
        if d < l.sun_r * 1.7 {
            continue;
        }
        let tw = 0.3 + 0.7 * (0.5 + 0.5 * (l.time * 0.9 + uph * TAU).sin());
        let bg = grid[yi][xi].bg;
        let fg = lerp_color(bg, l.mote, 0.16 + 0.38 * tw);
        grid[yi][xi] = Cell::with_bg(MOTE[(i & 1) as usize], fg, bg);
    }
    let _ = sc;
}

fn paint_sun(grid: &mut Grid, l: &Look) {
    let r = l.sun_r;
    let rx = (r * 2.8 * l.aspect).ceil() as i32 + 2;
    let ry = (r * 2.8).ceil() as i32 + 2;
    let (cx, cy) = (l.sun_px as i32, l.sun_py as i32);
    for y in (cy - ry).max(0)..=(cy + ry) {
        for x in (cx - rx).max(0)..=(cx + rx) {
            let dx = (x as f32 - l.sun_px) / l.aspect;
            let dy = y as f32 - l.sun_py;
            let d = (dx * dx + dy * dy).sqrt();
            let u = unit(hash(l.seed, L_SUN, x as u64, y as u64));
            let cell = if d < r * 0.72 {
                Some(Cell::with_bg(SUN_CORE, l.sun_core, l.sun_mid))
            } else if d < r * 1.05 {
                Some(Cell::with_bg(SUN_RIM, l.sun_core, l.sun_mid))
            } else if d < r * 1.7 && u < 0.5 {
                Some(Cell::with_bg(
                    CORONA[(u * 4.0) as usize % 4],
                    l.sun_mid,
                    bg_at(grid, x as usize, y as usize),
                ))
            } else if d < r * 2.6 && u < 0.14 {
                Some(Cell::with_bg(
                    ':',
                    darken(l.sun_mid, 40),
                    bg_at(grid, x as usize, y as usize),
                ))
            } else {
                None
            };
            if let Some(c) = cell {
                put(grid, x, y, c);
            }
        }
    }
    let rays = 7;
    for i in 0..rays {
        let a = i as f32 * (TAU / rays as f32) + unit(hash(l.seed, L_SUN, 1, i)) * 0.7;
        let (sa, ca) = a.sin_cos();
        let len = r * (2.6 + 3.4 * unit(hash(l.seed, L_SUN, 2, i)));
        let steps = (len * 1.2).clamp(2.0, 26.0) as usize;
        let tint = unit(hash(l.seed, L_SUN, 3, i));
        for s in 1..steps {
            let f = s as f32 / steps as f32;
            if f < 0.55 && tint < 0.55 {
                continue;
            }
            let x = l.sun_px + ca * len * f * l.aspect;
            let y = l.sun_py + sa * len * f;
            let col = lerp_color(l.sky_low, l.sun_mid, (1.0 - f) * 0.8);
            plot(grid, x, y, RAY[(f * 3.0) as usize % 3], col);
        }
    }
}

/// The stand's own silhouette, thrown along the sun onto the ground plane.
fn build_shadow(l: &Look, st: &Stand, shadow: &mut [f32], w: usize) {
    let sy = l.sun.y.max(0.24);
    let h = shadow.len() / w.max(1);
    for i in 1..st.nodes.len() {
        let n = &st.nodes[i];
        let t = n.pos.y / sy * 0.55;
        let sp = V3::new(n.pos.x - l.sun.x * t, 0.0, n.pos.z - l.sun.z * t);
        let (px, py, _) = project(l, sp);
        let ink = n.thick * (0.35 + 0.65 * n.avg);
        let (cx, cy) = (px as i32, py as i32);
        for dy in -1..=1i32 {
            for dx in -2..=2i32 {
                let (x, y) = (cx + dx, cy + dy);
                if x < 0 || y < 0 || x as usize >= w || y as usize >= h {
                    continue;
                }
                let fall = 1.0 - 0.2 * (dx.abs() / 2 + dy.abs()) as f32;
                let slot = &mut shadow[y as usize * w + x as usize];
                *slot = slot.max(ink * fall);
            }
        }
    }
}

fn paint_soil(grid: &mut Grid, l: &Look, sc: &Scratch) {
    let rows = grid.len().min(l.h);
    let start = (l.horizon.floor().max(0.0) as usize).min(rows);
    let span = (l.h as f32 - l.horizon).max(1.0);
    for y in start..rows {
        let v = ((y as f32 - l.horizon) / span).clamp(0.0, 1.0);
        let base = lerp_color(l.soil_far, l.soil, smoothstep(v * 1.6));
        for x in 0..grid[y].len().min(l.w) {
            let u = unit(hash(l.seed, L_SOIL, x as u64, y as u64));
            let sh = sc.shadow.get(y * l.w + x).copied().unwrap_or(0.0);
            let cell = if sh > 0.42 {
                let level = (((sh - 0.42) / 0.9).clamp(0.0, 0.999) * 3.0) as usize;
                Cell::with_bg(SHADE[level.min(2)], l.shade, base)
            } else if u < 0.028 {
                Cell::with_bg(SOIL[(u * 100.0) as usize % 3], darken(base, 14), base)
            } else {
                Cell::with_bg(' ', darken(base, 14), base)
            };
            grid[y][x] = cell;
        }
    }
    let (bx, by, _) = project(l, V3::new(0.0, 0.0, 0.0));
    for i in 0..6u64 {
        let spread = unit(hash(l.seed, L_TUFT, i, 0)) * 2.0 - 1.0;
        let x = bx + spread * l.ppu * l.aspect * 0.75;
        let len = 1.0 + 3.0 * unit(hash(l.seed, L_TUFT, i, 1));
        let lean = unit(hash(l.seed, L_TUFT, i, 2)) - 0.5;
        let y0 = by + 1.0 + unit(hash(l.seed, L_TUFT, i, 3)) * 1.6;
        let steps = len.ceil() as usize + 1;
        for s in 0..steps {
            let f = s as f32 / steps.max(1) as f32;
            plot(
                grid,
                x + lean * f * 2.0,
                y0 - f * len,
                TWIG[2],
                lerp_color(l.twig, l.soil, 0.35),
            );
        }
    }
}

/// Project every limb and leaf, sort far to near, and ink them in that order so
/// nearer wood occludes farther wood.
fn paint_stand(grid: &mut Grid, l: &Look, sc: &mut Scratch) {
    let comp = 0.10 + 0.55 * l.k.prune;
    sc.stamps.clear();
    for i in 1..sc.stand.nodes.len() {
        let n = &sc.stand.nodes[i];
        let p = &sc.stand.nodes[n.parent as usize];
        let (ax, ay, az) = project(l, p.pos);
        let (bx, by, bz) = project(l, n.pos);
        let surv = smoothstep((n.avg - 0.15) / 0.35);
        sc.stamps.push(Stamp {
            depth: (az + bz) * 0.5,
            ax,
            ay,
            bx,
            by,
            w: n.thick * l.ppu * 0.07,
            light: n.light,
            surv,
            phase: 0.0,
            kind: 0,
        });
    }
    let leaf_reach = 0.05 + 0.03 * l.k.leaf;
    for i in 0..sc.stand.nodes.len() {
        let leafy = sc.stand.leaf[i];
        if !leafy {
            continue;
        }
        let n = &sc.stand.nodes[i];
        let count = (l.k.leaf * 3.0 * n.light.max(0.08)).round() as usize;
        for j in 0..count.min(5) {
            let roll = j as f32 * GOLDEN + unit(hash(l.seed, L_LEAF, i as u64, 0)) * TAU;
            let dir = roll_axis(n.dir, roll);
            let phase = unit(hash(l.seed, L_LEAF, i as u64, 1)) * TAU;
            let flutter = (l.time * 0.9 + phase).sin() * 0.006;
            let tip = n.pos.add(dir.mul(leaf_reach + flutter));
            let (px, py, pz) = project(l, tip);
            sc.stamps.push(Stamp {
                depth: pz - 0.02,
                ax: px,
                ay: py,
                bx: px,
                by: py,
                w: 1.0,
                light: n.light,
                surv: 1.0,
                phase,
                kind: 1,
            });
        }
    }
    sc.stamps.sort_unstable_by(|a, b| {
        b.depth
            .partial_cmp(&a.depth)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    for s in sc.stamps.iter() {
        if s.kind == 0 {
            ink_wood(grid, l, s, &mut sc.bright);
        } else {
            ink_leaf(grid, l, s, comp, &mut sc.bright);
        }
    }
}

fn ink_wood(grid: &mut Grid, l: &Look, s: &Stamp, bright: &mut Vec<(u16, u16, Color)>) {
    let dx = s.bx - s.ax;
    let dy = s.by - s.ay;
    let cells = (dx * dx + dy * dy).sqrt();
    let samples = (cells * 1.4).clamp(1.0, 40.0) as usize;
    let table: &[char; 4] = if s.w > 0.95 {
        &BARK
    } else if s.w > 0.42 {
        &WOOD
    } else {
        &TWIG
    };
    let ch = slope_glyph(s.ax, s.ay, s.bx, s.by, table);
    let far = ((s.depth + BX) / (2.0 * BX)).clamp(0.0, 1.0);
    let haze = l.k.haze * 0.45 * far;
    let base = lerp_color(l.bark_dead, l.bark_live, s.surv);
    let sunn = lerp_color(base, l.sun_mid, 0.30 * smoothstep((s.light - 0.45) / 0.55));
    let col = lerp_color(sunn, l.sky_low, haze);
    let stride = (1.0 / (0.30 + 0.70 * s.surv)).round().max(1.0) as usize;
    for i in (0..=samples).step_by(stride) {
        let f = i as f32 / samples.max(1) as f32;
        let x = s.ax + dx * f;
        let y = s.ay + dy * f;
        plot(grid, x, y, ch, col);
        if s.w > 1.0 {
            plot(grid, x - 1.0, y, ch, darken_frac(col, 0.7));
        }
        if s.w > 1.9 {
            plot(grid, x + 1.0, y, ch, darken_frac(col, 0.7));
        }
    }
    if s.light > 0.72 && s.w > 1.2 && bright.len() < 384 {
        bright.push((s.bx.max(0.0) as u16, s.by.max(0.0) as u16, col));
    }
}

fn ink_leaf(grid: &mut Grid, l: &Look, s: &Stamp, comp: f32, bright: &mut Vec<(u16, u16, Color)>) {
    if s.light < comp * 0.85 {
        return;
    }
    let twinkle = 0.85 + 0.15 * (l.time * 1.3 + s.phase).sin();
    let lit = (s.light * twinkle).clamp(0.0, 1.0);
    let idx = ((lit * lit * 4.0) as usize).min(3);
    let ch = if lit > 0.92 { BUD } else { LEAF[idx] };
    let warm = lerp_color(l.leaf_shade, l.leaf_lit, lit.powf(0.7));
    let sunn = lerp_color(warm, l.sun_mid, 0.42 * smoothstep((lit - 0.45) / 0.55));
    let far = ((s.depth + BX) / (2.0 * BX)).clamp(0.0, 1.0);
    let col = lerp_color(sunn, l.sky_low, l.k.haze * 0.45 * far);
    plot(grid, s.ax, s.ay, ch, col);
    if l.ppu > 26.0 {
        plot(
            grid,
            s.ax + 1.0,
            s.ay,
            LEAF[idx.min(2)],
            darken_frac(col, 0.75),
        );
    }
    if lit > 0.7 && bright.len() < 384 {
        bright.push((s.ax.max(0.0) as u16, s.ay.max(0.0) as u16, col));
    }
}

#[inline]
fn darken_frac(c: Color, f: f32) -> Color {
    match c {
        Color::Rgb { r, g, b } => Color::Rgb {
            r: (r as f32 * f) as u8,
            g: (g as f32 * f) as u8,
            b: (b as f32 * f) as u8,
        },
        other => other,
    }
}

/// Sap beads climbing the lineage, brightest at the phase the run has reached.
fn paint_sap(grid: &mut Grid, l: &Look, st: &Stand) {
    if l.k.sap <= 0.0 || st.nodes.is_empty() {
        return;
    }
    let total = st.nodes[0].sugar.max(1e-3);
    for i in 1..st.nodes.len() {
        let n = &st.nodes[i];
        if n.sugar / total < 0.02 {
            continue;
        }
        let phase = (l.time * l.k.sap * 0.5 - n.along * 1.6).rem_euclid(1.0);
        let bead = (-((phase - 0.5) * (phase - 0.5)) / 0.012).exp();
        if bead < 0.35 {
            continue;
        }
        let (px, py, pz) = project(l, n.pos);
        let far = ((pz + BX) / (2.0 * BX)).clamp(0.0, 1.0);
        let col = lerp_color(
            lerp_color(l.sap, l.bark_live, 0.35),
            l.sky_low,
            l.k.haze * 0.45 * far,
        );
        let ch = SAP[(bead * 3.0).min(2.999) as usize];
        if py < 0.0 || px < 0.0 {
            continue;
        }
        let (yi, xi) = (py as usize, px as usize);
        if yi < grid.len() && xi < grid[yi].len() {
            let bg = grid[yi][xi].bg;
            grid[yi][xi] = Cell::with_bg(ch, col, bg);
        }
    }
}

/// Bright wood, sap and lit leaves bleed a little light into neighbouring cells.
fn paint_bloom(grid: &mut Grid, sc: &mut Scratch) {
    let bright = std::mem::take(&mut sc.bright);
    for &(x, y, col) in bright.iter() {
        for (dx, dy) in [(-1i32, 0i32), (1, 0), (0, -1), (0, 1), (-1, -1), (1, 1)] {
            let (nx, ny) = (x as i32 + dx, y as i32 + dy);
            if nx < 0 || ny < 0 {
                continue;
            }
            let (nx, ny) = (nx as usize, ny as usize);
            if ny >= grid.len() || nx >= grid[ny].len() {
                continue;
            }
            let cell = grid[ny][nx];
            let fg = lerp_color(cell.fg, col, 0.22);
            let bg = lerp_color(cell.bg, col, 0.16);
            grid[ny][nx] = Cell::with_bg(cell.ch, fg, bg);
        }
    }
    sc.bright = bright;
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

    fn diff_ratio(a: &str, b: &str) -> f64 {
        let mut diff = 0usize;
        let mut total = 0usize;
        for (ca, cb) in a.chars().zip(b.chars()) {
            total += 1;
            if ca != cb {
                diff += 1;
            }
        }
        diff as f64 / total.max(1) as f64
    }

    #[test]
    fn heliotrope_seed42() {
        insta::assert_snapshot!("heliotrope_80x24", text(&frame(80, 24, 42, 0.0, &knobs())));
    }

    #[test]
    fn heliotrope_turning_sun_t7() {
        insta::assert_snapshot!("heliotrope_80x24_t7", text(&frame(80, 24, 42, 7.0, &knobs())));
    }

    #[test]
    fn heliotrope_turning_sun_t21() {
        insta::assert_snapshot!(
            "heliotrope_80x24_t21",
            text(&frame(80, 24, 42, 21.0, &knobs()))
        );
    }

    #[test]
    fn heliotrope_lean_high() {
        let mut k = knobs();
        k[0] = 1.80;
        insta::assert_snapshot!("heliotrope_lean_180", text(&frame(80, 24, 42, 0.0, &k)));
    }

    #[test]
    fn heliotrope_small_grid() {
        insta::assert_snapshot!("heliotrope_48x12", text(&frame(48, 12, 42, 0.0, &knobs())));
    }

    #[test]
    fn heliotrope_lean_near_midpoint() {
        let mut k = knobs();
        k[0] = 0.95;
        insta::assert_snapshot!("heliotrope_lean_095", text(&frame(80, 24, 42, 0.0, &k)));
        k[0] = 1.05;
        insta::assert_snapshot!("heliotrope_lean_105", text(&frame(80, 24, 42, 0.0, &k)));
    }

    #[test]
    fn deterministic_and_seed_sensitive() {
        let k = knobs();
        assert_eq!(
            text(&frame(90, 30, 42, 0.0, &k)),
            text(&frame(90, 30, 42, 0.0, &k))
        );
        let reference = text(&frame(90, 30, 42, 0.0, &k));
        let mut distinct = 0;
        for seed in [7u64, 99, 1701] {
            if text(&frame(90, 30, seed, 0.0, &k)) != reference {
                distinct += 1;
            }
        }
        assert!(distinct >= 2, "only {distinct} of three seeds differed");
    }

    #[test]
    fn time_moves_the_light() {
        let k = knobs();
        assert_ne!(
            text(&frame(90, 30, 42, 0.0, &k)),
            text(&frame(90, 30, 42, 6.0, &k))
        );
    }

    #[test]
    fn nearby_lean_is_continuous() {
        let mut k = knobs();
        k[0] = 0.2;
        let far_low = text(&frame(80, 24, 42, 0.0, &k));
        k[0] = 1.8;
        let far_high = text(&frame(80, 24, 42, 0.0, &k));
        k[0] = 0.95;
        let mid_low = text(&frame(80, 24, 42, 0.0, &k));
        k[0] = 1.05;
        let mid_high = text(&frame(80, 24, 42, 0.0, &k));
        let near = diff_ratio(&mid_low, &mid_high);
        let far = diff_ratio(&far_low, &far_high);
        assert!(near < 0.2, "nearby lean values differ by {near}");
        assert!(far > near * 2.0, "lean sweep is flat: {near} vs {far}");
    }

    #[test]
    fn extreme_parameters_terminate() {
        let lo: Vec<f32> = PARAMS.iter().map(|p| p.min).collect();
        let hi: Vec<f32> = PARAMS.iter().map(|p| p.max).collect();
        for values in [&lo, &hi] {
            let g = frame(24, 8, 3, 0.0, values);
            assert_eq!(g.len(), 8);
            assert!(g.iter().all(|row| row.len() == 24));
        }
        let g = frame(2, 2, 1, 4.0, &knobs());
        assert_eq!(g.len(), 2);
    }

    #[test]
    fn frame_cost() {
        let (w, h) = (200usize, 60usize);
        let k = knobs();
        let mut worst = 0.0f64;
        let start = std::time::Instant::now();
        for f in 0..60 {
            let t0 = std::time::Instant::now();
            frame(w, h, 42, f as f32 * 0.25, &k);
            worst = worst.max(t0.elapsed().as_secs_f64() * 1000.0);
        }
        let avg = start.elapsed().as_secs_f64() * 1000.0 / 60.0;
        eprintln!(
            "heliotrope frame_cost 200x60: avg {:.3} ms, worst {:.3} ms",
            avg, worst
        );
        if !cfg!(debug_assertions) {
            assert!(avg < 6.0, "avg frame {:.3} ms", avg);
        }
    }
}
