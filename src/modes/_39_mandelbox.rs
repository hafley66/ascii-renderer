use std::ops::{Add, AddAssign, Mul, Neg, Sub};
use std::thread;

use crossterm::style::Color;
use rand::rngs::StdRng;

use crate::_0_profile::measure_layer;
use crate::color::{darken, lerp_color, lighten};
use crate::opts::param_f32;
use crate::registry::{AnimKind, Mode, ModeFrame, Param};
use crate::types::{Cell, Grid};

const TAU: f32 = std::f32::consts::TAU;
const SURFACE_GLYPHS: [char; 12] = ['·', ':', ';', '+', '=', 'x', 'X', '#', '%', '@', '▓', '█'];
const GLOW_GLYPHS: [char; 6] = [' ', '·', '∙', ':', '+', '*'];

pub(super) struct MandelboxMode;

pub(super) static MODE: MandelboxMode = MandelboxMode;

const PARAMS: &[Param] = &[
    param!("FOLDS", "fractal folds", 4.0, 10.0, 6.0, 1.0),
    param!("STEPS", "ray steps", 12.0, 64.0, 36.0, 1.0),
    param!("SCALE", "fold scale", -2.2, -1.2, -1.72, 0.02),
    param!("DETAIL", "surface detail", 0.5, 2.0, 1.0, 0.05),
    param!("ZOOM", "camera zoom", 0.65, 1.5, 1.0, 0.05),
    param!("SPIN", "object spin", -2.0, 2.0, 0.48, 0.05),
    param!("TILT", "camera elevation", -0.45, 0.55, 0.12, 0.02),
    param!("SPEED", "animation clock", 0.05, 3.0, 0.42, 0.05),
    param!("AO", "crease darkness", 0.0, 1.5, 0.9, 0.05),
    param!("GLOW", "fractal corona", 0.0, 1.5, 0.88, 0.05),
    param!("FLOOR", "perspective floor", 0.0, 1.5, 0.72, 0.05),
    param!("STARS", "stellar field", 0.0, 1.5, 0.62, 0.05),
];

impl Mode for MandelboxMode {
    fn name(&self) -> &'static str {
        "mandelbox"
    }

    fn help(&self) -> &'static str {
        "Ray-marched rotating 3D Mandelbox with orbit-trap surface bands, ambient creases, fractal corona, star field, and perspective floor [folds] [steps] [scale] [detail] [zoom] [spin] [tilt] [speed] [ao] [glow] [floor] [stars]"
    }

    fn animation(&self) -> AnimKind {
        AnimKind::Iterate
    }

    fn params(&self) -> &'static [Param] {
        PARAMS
    }

    fn render(&self, frame: &mut ModeFrame<'_>) {
        let params = MandelboxParams::from_inputs(frame.args, frame.param_values);
        draw_mandelbox(
            frame.grid,
            frame.width,
            frame.height,
            frame.seed,
            frame.palette,
            frame.rng,
            frame.time,
            &params,
        );
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct MandelboxParams {
    pub(crate) folds: usize,
    pub(crate) steps: usize,
    pub(crate) scale: f32,
    pub(crate) detail: f32,
    pub(crate) zoom: f32,
    pub(crate) spin: f32,
    pub(crate) tilt: f32,
    pub(crate) speed: f32,
    pub(crate) ao: f32,
    pub(crate) glow: f32,
    pub(crate) floor: f32,
    pub(crate) stars: f32,
}

impl Default for MandelboxParams {
    fn default() -> Self {
        Self {
            folds: 6,
            steps: 36,
            scale: -1.72,
            detail: 1.0,
            zoom: 1.0,
            spin: 0.48,
            tilt: 0.12,
            speed: 0.42,
            ao: 0.9,
            glow: 0.88,
            floor: 0.72,
            stars: 0.62,
        }
    }
}

impl MandelboxParams {
    pub(crate) fn from_inputs(args: &[String], param_values: Option<&[f32]>) -> Self {
        let read = |index: usize, key: &str, default: f32| {
            args.get(index)
                .and_then(|value| value.parse::<f32>().ok())
                .or_else(|| {
                    param_values
                        .and_then(|values| values.get(index.saturating_sub(4)))
                        .copied()
                })
                .unwrap_or_else(|| param_f32(key, default))
        };
        Self {
            folds: read(4, "FOLDS", 6.0).round().clamp(4.0, 10.0) as usize,
            steps: read(5, "STEPS", 36.0).round().clamp(12.0, 64.0) as usize,
            scale: read(6, "SCALE", -1.72).clamp(-2.2, -1.2),
            detail: read(7, "DETAIL", 1.0).clamp(0.5, 2.0),
            zoom: read(8, "ZOOM", 1.0).clamp(0.65, 1.5),
            spin: read(9, "SPIN", 0.48).clamp(-2.0, 2.0),
            tilt: read(10, "TILT", 0.12).clamp(-0.45, 0.55),
            speed: read(11, "SPEED", 0.42).clamp(0.05, 3.0),
            ao: read(12, "AO", 0.9).clamp(0.0, 1.5),
            glow: read(13, "GLOW", 0.88).clamp(0.0, 1.5),
            floor: read(14, "FLOOR", 0.72).clamp(0.0, 1.5),
            stars: read(15, "STARS", 0.62).clamp(0.0, 1.5),
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct V3 {
    x: f32,
    y: f32,
    z: f32,
}

impl V3 {
    #[inline]
    const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    #[inline]
    fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    #[inline]
    fn length(self) -> f32 {
        self.dot(self).sqrt()
    }

    #[inline]
    fn normalized(self) -> Self {
        self * (1.0 / self.length().max(0.000_001))
    }

    #[inline]
    fn cross(self, other: Self) -> Self {
        Self::new(
            self.y * other.z - self.z * other.y,
            self.z * other.x - self.x * other.z,
            self.x * other.y - self.y * other.x,
        )
    }
}

impl Add for V3 {
    type Output = Self;

    #[inline]
    fn add(self, other: Self) -> Self {
        Self::new(self.x + other.x, self.y + other.y, self.z + other.z)
    }
}

impl AddAssign for V3 {
    #[inline]
    fn add_assign(&mut self, other: Self) {
        self.x += other.x;
        self.y += other.y;
        self.z += other.z;
    }
}

impl Sub for V3 {
    type Output = Self;

    #[inline]
    fn sub(self, other: Self) -> Self {
        Self::new(self.x - other.x, self.y - other.y, self.z - other.z)
    }
}

impl Mul<f32> for V3 {
    type Output = Self;

    #[inline]
    fn mul(self, scalar: f32) -> Self {
        Self::new(self.x * scalar, self.y * scalar, self.z * scalar)
    }
}

impl Neg for V3 {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self {
        Self::new(-self.x, -self.y, -self.z)
    }
}

struct Scene {
    folds: usize,
    steps: usize,
    scale: f32,
    epsilon: f32,
    yaw_sin: f32,
    yaw_cos: f32,
    pitch_sin: f32,
    pitch_cos: f32,
}

impl Scene {
    fn new(t: f32, seed: u64, params: &MandelboxParams) -> Self {
        let seed_phase = hash01(seed, 19) * TAU;
        let clock = t * params.speed;
        let (yaw_sin, yaw_cos) = (clock * params.spin + seed_phase * 0.17).sin_cos();
        let (pitch_sin, pitch_cos) =
            (clock * params.spin * 0.37 + 0.28 + seed_phase * 0.07).sin_cos();
        Self {
            folds: params.folds,
            steps: params.steps,
            scale: params.scale,
            epsilon: 0.0022 / params.detail,
            yaw_sin,
            yaw_cos,
            pitch_sin,
            pitch_cos,
        }
    }

    #[inline]
    fn to_object(&self, point: V3) -> V3 {
        let yawed = V3::new(
            point.x * self.yaw_cos - point.z * self.yaw_sin,
            point.y,
            point.x * self.yaw_sin + point.z * self.yaw_cos,
        );
        V3::new(
            yawed.x,
            yawed.y * self.pitch_cos + yawed.z * self.pitch_sin,
            -yawed.y * self.pitch_sin + yawed.z * self.pitch_cos,
        )
    }

    #[inline]
    fn distance_object(&self, origin: V3) -> f32 {
        let mut z = origin;
        let mut derivative = 1.0;

        for _ in 0..self.folds {
            z.x = z.x.clamp(-1.0, 1.0) * 2.0 - z.x;
            z.y = z.y.clamp(-1.0, 1.0) * 2.0 - z.y;
            z.z = z.z.clamp(-1.0, 1.0) * 2.0 - z.z;

            let radius2 = z.dot(z);
            let sphere_scale = if radius2 < 0.25 {
                4.0
            } else if radius2 < 1.0 {
                1.0 / radius2
            } else {
                1.0
            };
            z = z * sphere_scale;
            derivative *= sphere_scale;

            z = z * self.scale + origin;
            derivative = derivative * self.scale.abs() + 1.0;
        }

        z.length() / derivative.abs().max(0.000_001)
    }

    #[inline]
    fn orbit_trap_object(&self, origin: V3) -> f32 {
        let mut z = origin;
        let mut trap: f32 = 8.0;

        for _ in 0..self.folds {
            z.x = z.x.clamp(-1.0, 1.0) * 2.0 - z.x;
            z.y = z.y.clamp(-1.0, 1.0) * 2.0 - z.y;
            z.z = z.z.clamp(-1.0, 1.0) * 2.0 - z.z;

            let radius2 = z.dot(z);
            let sphere_scale = if radius2 < 0.25 {
                4.0
            } else if radius2 < 1.0 {
                1.0 / radius2
            } else {
                1.0
            };
            z = z * sphere_scale;
            trap = trap.min(z.x.abs().min(z.y.abs()).min(z.z.abs()));
            trap = trap.min((radius2 - 0.74).abs());
            z = z * self.scale + origin;
        }

        trap
    }
}

struct Projection {
    columns: Vec<V3>,
    rows: Vec<V3>,
}

impl Projection {
    fn new(width: usize, height: usize, camera: V3, target: V3, zoom: f32) -> Self {
        let forward = (target - camera).normalized();
        let right = forward.cross(V3::new(0.0, 1.0, 0.0)).normalized();
        let up = right.cross(forward).normalized();
        let fov = 0.69 / zoom;
        let aspect = width as f32 / (height.max(1) as f32 * 2.0);
        let columns = (0..width)
            .map(|x| {
                let sx = ((x as f32 + 0.5) / width.max(1) as f32 * 2.0 - 1.0) * aspect;
                right * (sx * fov)
            })
            .collect();
        let rows = (0..height)
            .map(|y| {
                let sy = 1.0 - (y as f32 + 0.5) / height.max(1) as f32 * 2.0;
                forward + up * (sy * fov)
            })
            .collect();
        Self { columns, rows }
    }

    #[inline]
    fn ray(&self, x: usize, y: usize) -> V3 {
        (self.rows[y] + self.columns[x]).normalized()
    }
}

#[derive(Clone, Copy)]
struct MarchHit {
    point: V3,
    ray: V3,
    trap: f32,
    steps: usize,
}

#[derive(Clone, Copy)]
struct MarchResult {
    hit: Option<MarchHit>,
    nearest: f32,
}

#[inline]
fn sphere_interval(origin: V3, ray: V3, radius: f32) -> Option<(f32, f32)> {
    let projection = origin.dot(ray);
    let discriminant = projection * projection - (origin.dot(origin) - radius * radius);
    if discriminant < 0.0 {
        return None;
    }
    let root = discriminant.sqrt();
    let near = -projection - root;
    let far = -projection + root;
    (far > 0.0).then_some((near.max(0.0), far))
}

#[inline]
fn march(scene: &Scene, origin: V3, ray: V3, near: f32, far: f32) -> MarchResult {
    let mut travel = near;
    let mut nearest: f32 = 8.0;
    for step in 0..scene.steps {
        let point = origin + ray * travel;
        let distance = scene.distance_object(point);
        nearest = nearest.min(distance);
        let threshold = scene.epsilon * (1.0 + travel * 0.08);
        if distance < threshold {
            return MarchResult {
                hit: Some(MarchHit {
                    point,
                    ray,
                    trap: scene.orbit_trap_object(point),
                    steps: step,
                }),
                nearest,
            };
        }
        travel += distance.max(threshold * 0.45);
        if travel > far {
            break;
        }
    }
    MarchResult { hit: None, nearest }
}

#[inline]
fn mix64(mut value: u64) -> u64 {
    value ^= value >> 30;
    value = value.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

#[inline]
fn hash01(seed: u64, tag: u64) -> f32 {
    let value = mix64(seed ^ tag.wrapping_mul(0x9E37_79B9_7F4A_7C15));
    ((value >> 40) as f32) * (1.0 / 16_777_215.0)
}

#[inline]
fn screen_hash(seed: u64, x: usize, y: usize) -> f32 {
    hash01(
        seed ^ (x as u64).wrapping_mul(0xD6E8_FEB8_6659_FD93),
        y as u64 ^ 0xA5A3_56E4_9B2F_7A49,
    )
}

#[inline]
fn ramp(glyphs: &[char], value: f32, dither: f32) -> char {
    let index = (value.clamp(0.0, 1.0) * (glyphs.len() - 1) as f32 + dither - 0.5)
        .round()
        .clamp(0.0, (glyphs.len() - 1) as f32) as usize;
    glyphs[index]
}

fn shade_surface(
    normal: V3,
    light: V3,
    hit: MarchHit,
    max_steps: usize,
    palette: &[Color; 5],
    noise: f32,
    clock: f32,
    params: &MandelboxParams,
) -> Cell {
    let view = -hit.ray;
    let diffuse = normal.dot(light).max(0.0);
    let facing = normal.dot(view).abs().clamp(0.0, 1.0);
    let rim = (1.0 - facing).powi(2);
    let half_vector = (light + view).normalized();
    let specular = normal.dot(half_vector).max(0.0).powi(16);
    let crease = (hit.trap * 5.5).clamp(0.0, 1.0);
    let ambient = (1.0 - params.ao * (1.0 - crease) * 0.52).clamp(0.2, 1.0);
    let step_fade = 1.0 - hit.steps as f32 / max_steps.max(1) as f32 * 0.28;
    let band = (hit.trap * 31.0 - clock * 0.7).sin() * 0.5 + 0.5;
    let brightness = ((0.16 + diffuse * 0.72 + rim * 0.34 + specular * 0.58) * ambient * step_fade)
        .clamp(0.0, 1.0)
        .sqrt();
    let color_a = lerp_color(palette[1], palette[2], band);
    let color_b = lerp_color(color_a, palette[3], (rim * 0.75 + specular).clamp(0.0, 1.0));
    let fg = lerp_color(darken(color_b, 52), lighten(color_b, 46), brightness);
    let bg = lerp_color(darken(palette[0], 8), darken(color_a, 62), rim * 0.32);
    let glyph_light = brightness * (0.58 + band * 0.42);
    Cell::with_bg(ramp(&SURFACE_GLYPHS, glyph_light, noise), fg, bg)
}

fn hit_normal(hits: &[Option<MarchHit>], width: usize, height: usize, x: usize, y: usize) -> V3 {
    let hit = hits[y * width + x].expect("surface normal requires a hit");
    let nearby = |other: &MarchHit| {
        let delta = other.point - hit.point;
        delta.dot(delta) < 0.42
    };
    let left = (x > 0)
        .then(|| hits[y * width + x - 1])
        .flatten()
        .filter(nearby);
    let right = (x + 1 < width)
        .then(|| hits[y * width + x + 1])
        .flatten()
        .filter(nearby);
    let up = (y > 0)
        .then(|| hits[(y - 1) * width + x])
        .flatten()
        .filter(nearby);
    let down = (y + 1 < height)
        .then(|| hits[(y + 1) * width + x])
        .flatten()
        .filter(nearby);

    let horizontal = match (left, right) {
        (Some(left), Some(right)) => right.point - left.point,
        (None, Some(right)) => right.point - hit.point,
        (Some(left), None) => hit.point - left.point,
        (None, None) => V3::new(1.0, 0.0, 0.0),
    };
    let vertical = match (up, down) {
        (Some(up), Some(down)) => down.point - up.point,
        (None, Some(down)) => down.point - hit.point,
        (Some(up), None) => hit.point - up.point,
        (None, None) => V3::new(0.0, -1.0, 0.0),
    };
    let crossed = horizontal.cross(vertical);
    let mut normal = if crossed.dot(crossed) > 0.000_001 {
        crossed.normalized()
    } else {
        hit.point.normalized()
    };
    if normal.dot(-hit.ray) < 0.0 {
        normal = -normal;
    }
    normal
}

fn shade_floor(
    origin: V3,
    ray: V3,
    palette: &[Color; 5],
    noise: f32,
    params: &MandelboxParams,
) -> Option<Cell> {
    if params.floor <= 0.0 || ray.y >= -0.000_1 {
        return None;
    }
    let travel = (-2.05 - origin.y) / ray.y;
    if travel <= 0.0 || travel > 16.0 {
        return None;
    }
    let point = origin + ray * travel;
    let width = (0.018 * travel).clamp(0.025, 0.24);
    let x_line = (point.x - point.x.round()).abs() < width;
    let z_line = (point.z - point.z.round()).abs() < width;
    let fade = (1.0 - travel / 18.0).clamp(0.0, 1.0) * params.floor;
    let radial2 = point.x * point.x + point.z * point.z;
    let shadow = 0.72 / (1.0 + radial2 * 0.9);
    let checker = ((point.x.floor() as i32 + point.z.floor() as i32) & 1) == 0;
    let ch = match (x_line, z_line) {
        (true, true) => '┼',
        (true, false) => '│',
        (false, true) => '─',
        (false, false) if noise > 0.985 - fade * 0.018 => '·',
        _ => ' ',
    };
    let line_light = if x_line || z_line { 0.56 } else { 0.12 };
    let cell_light = (line_light + if checker { 0.08 } else { 0.0 }) * fade * (1.0 - shadow);
    let fg = lerp_color(darken(palette[2], 64), palette[1], cell_light);
    let bg = lerp_color(darken(palette[0], 12), darken(palette[2], 66), fade * 0.3);
    Some(Cell::with_bg(ch, fg, bg))
}

fn shade_sky(
    x: usize,
    y: usize,
    seed: u64,
    palette: &[Color; 5],
    noise: f32,
    clock: f32,
    params: &MandelboxParams,
) -> Cell {
    let threshold = 0.994 - params.stars * 0.007;
    let (ch, light) = if noise > threshold {
        let twinkle = (clock * 1.7 + noise * TAU).sin() * 0.5 + 0.5;
        if noise > 0.9992 {
            ('✦', 0.82 + twinkle * 0.18)
        } else if twinkle > 0.65 {
            ('+', 0.58 + twinkle * 0.24)
        } else {
            ('·', 0.42 + twinkle * 0.2)
        }
    } else {
        let dust = screen_hash(seed ^ 0xFACE_FEED, x / 4, y / 2);
        if dust > 0.982 && noise > 0.78 {
            ('·', 0.19)
        } else {
            (' ', 0.0)
        }
    };
    let fg = lerp_color(darken(palette[2], 74), palette[4], light);
    Cell::with_bg(ch, fg, darken(palette[0], 10))
}

struct TraceContext<'a> {
    width: usize,
    seed: u64,
    camera: V3,
    object_camera: V3,
    projection: &'a Projection,
    scene: &'a Scene,
    palette: &'a [Color; 5],
    clock: f32,
    params: &'a MandelboxParams,
}

fn trace_rows(
    context: &TraceContext<'_>,
    grid: &mut [Vec<Cell>],
    hits: &mut [Option<MarchHit>],
    y_offset: usize,
) {
    for (local_y, row) in grid.iter_mut().enumerate() {
        let y = y_offset + local_y;
        let row_width = context.width.min(row.len());
        for x in 0..row_width {
            let ray = context.projection.ray(x, y);
            let object_ray = context.scene.to_object(ray);
            let noise = screen_hash(context.seed, x, y);

            let result = sphere_interval(context.camera, ray, 2.85).map(|(near, far)| {
                march(context.scene, context.object_camera, object_ray, near, far)
            });
            if let Some(hit) = result.and_then(|result| result.hit) {
                hits[local_y * context.width + x] = Some(hit);
                continue;
            }

            let nearest = result.map_or(8.0, |result| result.nearest);
            let glow_base = ((0.24 - nearest) * (1.0 / 0.24)).clamp(0.0, 1.0);
            let glow = glow_base * glow_base * context.params.glow;
            if glow > 0.06 {
                let intensity = glow.clamp(0.0, 1.0);
                let fg = lerp_color(
                    darken(context.palette[1], 72),
                    context.palette[3],
                    intensity,
                );
                let bg = lerp_color(
                    darken(context.palette[0], 10),
                    darken(context.palette[2], 62),
                    intensity * 0.24,
                );
                row[x] = Cell::with_bg(ramp(&GLOW_GLYPHS, intensity, noise), fg, bg);
            } else if let Some(cell) =
                shade_floor(context.camera, ray, context.palette, noise, context.params)
            {
                row[x] = cell;
            } else {
                row[x] = shade_sky(
                    x,
                    y,
                    context.seed,
                    context.palette,
                    noise,
                    context.clock,
                    context.params,
                );
            }
        }
    }
}

pub(crate) fn draw_mandelbox(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    _rng: &mut StdRng,
    t: f32,
    params: &MandelboxParams,
) {
    if width == 0 || height == 0 || grid.is_empty() {
        return;
    }

    // 1. Precompute one camera basis and one inverse object transform from explicit inputs.
    let clock = t * params.speed;
    let seed_phase = hash01(seed, 71) * TAU;
    let orbit = clock * 0.23 + seed_phase * 0.08;
    let elevation = 0.48 + params.tilt + (clock * 0.19).sin() * 0.08;
    let radius = 5.1 / params.zoom.sqrt();
    let camera = V3::new(orbit.sin() * radius, elevation, orbit.cos() * radius);
    let target = V3::new(0.0, -0.12, 0.0);
    let projection = Projection::new(width, height, camera, target, params.zoom);
    let scene = Scene::new(t, seed, params);
    let object_camera = scene.to_object(camera);
    let object_light = scene.to_object(V3::new(-0.48, 0.78, -0.4)).normalized();
    let mut hits = vec![None; width.saturating_mul(height)];
    let active_height = height.min(grid.len());
    let context = TraceContext {
        width,
        seed,
        camera,
        object_camera,
        projection: &projection,
        scene: &scene,
        palette,
        clock,
        params,
    };

    // 2. Generate exactly one ray for each cell in a bounded grid pass.
    measure_layer("mandelbox", "trace", || {
        // 3. Split row ownership when the frame has enough work, then bound each ray by the fractal sphere and STEPS.
        let desired_workers = match width.saturating_mul(active_height) {
            0..=2_999 => 1,
            3_000..=7_999 => 3,
            _ => 8,
        };
        let workers = thread::available_parallelism()
            .map_or(1, |count| count.get())
            .min(desired_workers)
            .min(active_height);
        if workers == 1 {
            trace_rows(
                &context,
                &mut grid[..active_height],
                &mut hits[..active_height * width],
                0,
            );
            return;
        }
        let rows_per_worker = active_height.div_ceil(workers);
        thread::scope(|scope| {
            let mut grid_tail = &mut grid[..active_height];
            let mut hit_tail = &mut hits[..active_height * width];
            let mut y_offset = 0;
            while !grid_tail.is_empty() {
                let rows = rows_per_worker.min(grid_tail.len());
                let (grid_chunk, next_grid) = grid_tail.split_at_mut(rows);
                let (hit_chunk, next_hits) = hit_tail.split_at_mut(rows * width);
                grid_tail = next_grid;
                hit_tail = next_hits;
                let context = &context;
                scope.spawn(move || trace_rows(context, grid_chunk, hit_chunk, y_offset));
                y_offset += rows;
            }
        });
    });

    // 4. Derive coherent normals from adjacent 3D hits, then shade each confirmed surface once.
    measure_layer("mandelbox", "surface_shade", || {
        for y in 0..height.min(grid.len()) {
            let row_width = width.min(grid[y].len());
            for x in 0..row_width {
                let Some(hit) = hits[y * width + x] else {
                    continue;
                };
                let noise = screen_hash(seed, x, y);
                let normal = hit_normal(&hits, width, height, x, y);
                grid[y][x] = shade_surface(
                    normal,
                    object_light,
                    hit,
                    scene.steps,
                    palette,
                    noise,
                    clock,
                    params,
                );
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::make_palette;
    use crate::render::grid_to_plain;
    use rand::SeedableRng;

    fn frame(width: usize, height: usize, seed: u64, t: f32, params: &MandelboxParams) -> Grid {
        let mut grid = vec![vec![Cell::blank(); width]; height];
        let palette = make_palette(seed);
        let mut rng = StdRng::seed_from_u64(seed);
        draw_mandelbox(
            &mut grid, width, height, seed, &palette, &mut rng, t, params,
        );
        grid
    }

    fn plain(grid: &Grid) -> String {
        grid_to_plain(grid).join("\n")
    }

    #[test]
    fn deterministic_seeded_frame_and_visible_motion() {
        let params = MandelboxParams::default();
        let a = frame(96, 34, 42, 1.25, &params);
        let b = frame(96, 34, 42, 1.25, &params);
        let moved = frame(96, 34, 42, 5.75, &params);
        let reseeded = frame(96, 34, 43, 1.25, &params);
        assert_eq!(a, b);
        assert_ne!(a, moved);
        assert_ne!(a, reseeded);
    }

    #[test]
    fn parameter_values_override_and_clamp() {
        let values = [
            99.0, 0.0, -99.0, 99.0, 0.0, 99.0, -99.0, 0.0, 99.0, -1.0, 99.0, -1.0,
        ];
        let params = MandelboxParams::from_inputs(&[], Some(&values));
        insta::assert_debug_snapshot!(params, @r###"
        MandelboxParams {
            folds: 10,
            steps: 12,
            scale: -2.2,
            detail: 2.0,
            zoom: 0.65,
            spin: 2.0,
            tilt: -0.45,
            speed: 0.05,
            ao: 1.5,
            glow: 0.0,
            floor: 1.5,
            stars: 0.0,
        }
        "###);
    }

    #[test]
    fn tiny_grids_and_parameter_extrema_terminate() {
        let maxed = MandelboxParams {
            folds: 10,
            steps: 64,
            scale: -2.2,
            detail: 2.0,
            zoom: 1.5,
            spin: -2.0,
            tilt: 0.55,
            speed: 3.0,
            ao: 1.5,
            glow: 1.5,
            floor: 1.5,
            stars: 1.5,
        };
        for size in [(1usize, 1usize), (2, 1), (3, 5), (9, 2)] {
            let grid = frame(size.0, size.1, 7, 50_000.0, &maxed);
            assert_eq!(grid.len(), size.1);
            assert!(grid.iter().all(|row| row.len() == size.0));
        }
    }

    #[test]
    fn frame_contains_dense_fractal_surface_and_floor() {
        let grid = frame(100, 36, 42, 0.0, &MandelboxParams::default());
        let surface = grid
            .iter()
            .flat_map(|row| row.iter())
            .filter(|cell| SURFACE_GLYPHS.contains(&cell.ch) && cell.ch != '·')
            .count();
        let floor = grid
            .iter()
            .flat_map(|row| row.iter())
            .filter(|cell| matches!(cell.ch, '┼' | '│' | '─'))
            .count();
        assert!(surface >= 120);
        assert!(floor >= 20);
    }

    #[test]
    fn optional_environment_changes_the_composition() {
        let params = MandelboxParams::default();
        let full = frame(90, 32, 77, 2.0, &params);
        let void = frame(
            90,
            32,
            77,
            2.0,
            &MandelboxParams {
                glow: 0.0,
                floor: 0.0,
                stars: 0.0,
                ..params
            },
        );
        assert_ne!(full, void);
    }

    #[test]
    fn snapshot_mandelbox_seed_42() {
        insta::assert_snapshot!(plain(&frame(80, 28, 42, 0.0, &MandelboxParams::default(),)));
    }

    #[test]
    fn snapshot_mandelbox_rotated() {
        insta::assert_snapshot!(plain(
            &frame(80, 28, 42, 5.25, &MandelboxParams::default(),)
        ));
    }

    #[test]
    #[ignore = "release-only performance probe; run with --release --ignored"]
    fn perf_mandelbox_frame() {
        use std::hint::black_box;
        use std::time::Instant;

        let width = 320;
        let height = 100;
        let frames = 240;
        let seed = 42;
        let params = MandelboxParams::default();
        let palette = make_palette(seed);
        let mut grid = vec![vec![Cell::blank(); width]; height];
        let started = Instant::now();
        for index in 0..frames {
            let mut rng = StdRng::seed_from_u64(seed);
            draw_mandelbox(
                &mut grid,
                width,
                height,
                seed,
                &palette,
                &mut rng,
                index as f32 * 0.06,
                &params,
            );
            black_box(&grid);
        }
        let average_ms = started.elapsed().as_secs_f64() * 1_000.0 / frames as f64;
        eprintln!("mandelbox {width}x{height}: {average_ms:.3} ms/frame");
        assert!(
            average_ms < 16.0,
            "{average_ms:.3} ms exceeds 60 fps budget"
        );
    }
}
