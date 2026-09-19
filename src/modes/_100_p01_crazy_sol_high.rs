//! Sol Reliquary: a four-dimensional solar torus seen through its own light field.
//!
//! Design before body
//! ------------------
//! Proposed signatures:
//! - `fn draw(frame: &mut ModeFrame<'_>, p: &[f32; KNOBS])`
//! - `fn build_lattice(field: &mut [FieldCell], look: &Look, w: usize, h: usize)`
//! - `fn trace_flux(field: &mut [FieldCell], look: &Look, w: usize, h: usize)`
//! - `fn answer_light(field: &mut [FieldCell], look: &Look, w: usize, h: usize)`
//! - `fn paint(grid: &mut Grid, field: &[FieldCell], look: &Look, w: usize, h: usize)`
//!
//! 1. One Mode instance lives for the process. Each render resolves parameters,
//!    creates a frame-local Look, reuses thread-local scratch, and paints one grid.
//! 2. Persistent storage is scratch capacity only. CLI args, `frame.time`, and
//!    resolved live parameter values are borrowed for one call.
//! 3. Every object identity comes from `(seed, lineage, slot)` through `hash`.
//!    No RNG stream or prior frame influences a frame.
//! 4. The frame clears scratch, deposits the 4D lattice, traces light through the
//!    deposited density, lets light deform a response lattice, then paints.
//! 5. Fixed parametric rings cross under projection. Depth, density, irradiance,
//!    and tangent determine occlusion, intersections, glyphs, and colors.
//! 6. The registry name and six parameter keys are unique. Registration has one
//!    generated path through `src/modes/mod.rs`.
//! 7. Work is bounded by `O(W*H + 17*U + 13*V)`, where `U <= 192`, `V <= 128`.
//!    The completed light tracer adds fixed bounded ray and trail limits.
//! 8. Projection uses terminal-cell coordinates with separate horizontal and
//!    vertical scales. Every deposit checks both axes before indexing.
//! 9. Paint order is void, diffraction atmosphere, far lattice, light, near
//!    lattice, and the preserved umbral core.
//! 10. Time changes projection and phase only. Seeded geometry retains identity.
//! 11. `fold`, `gravity`, `flux`, `spin`, `bloom`, and `aperture` are continuous;
//!     each deforms or illuminates the same torus/light system.
//! 12. Lattice density bends rays; deposited irradiance displaces and brightens
//!     the response lattice, producing a bounded same-frame feedback path.

use crate::_0_profile::measure_layer;
use crate::color::{darken, lerp_color, lighten, shift_hue};
use crate::opts::param_f32;
use crate::registry::{AnimKind, Mode, ModeFrame, Param};
use crate::types::{Cell, Grid};
use crossterm::style::Color;
use std::cell::RefCell;
use std::f32::consts::TAU;

pub(super) struct SolReliquary;
pub(super) static MODE: SolReliquary = SolReliquary;

const NAME: &str = "sol-reliquary";
const KNOBS: usize = 6;
const HELP: &str =
    "sol-reliquary: a light-bent 4D solar torus [fold] [gravity] [flux] [spin] [bloom] [aperture]";

const PARAMS: &[Param] = &[
    param!("SOL_FOLD", "four-dimensional fold", 0.0, 1.0, 0.56, 0.02),
    param!("SOL_GRAVITY", "light curvature", 0.0, 2.0, 0.92, 0.05),
    param!("SOL_FLUX", "energy circulation", 0.0, 2.0, 1.0, 0.05),
    param!("SOL_SPIN", "projection speed", 0.0, 2.0, 0.72, 0.04),
    param!("SOL_BLOOM", "field response", 0.0, 1.5, 0.78, 0.05),
    param!("SOL_APERTURE", "umbra aperture", 0.2, 1.0, 0.62, 0.02),
];

impl Mode for SolReliquary {
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

#[derive(Clone, Copy, Default)]
struct V4 {
    x: f32,
    y: f32,
    z: f32,
    w: f32,
}

#[derive(Clone, Copy, Default)]
struct Projected {
    x: f32,
    y: f32,
    depth: f32,
}

#[derive(Clone, Copy)]
struct FieldCell {
    density: f32,
    light: f32,
    light_peak: f32,
    light_tangent: f32,
    response: f32,
    depth: f32,
    tangent: f32,
    crossings: u8,
}

impl Default for FieldCell {
    fn default() -> Self {
        Self {
            density: 0.0,
            light: 0.0,
            light_peak: 0.0,
            light_tangent: 0.0,
            response: 0.0,
            depth: -10.0,
            tangent: 0.0,
            crossings: 0,
        }
    }
}

thread_local! {
    static SCRATCH: RefCell<Vec<FieldCell>> = const { RefCell::new(Vec::new()) };
}

struct Look {
    seed: u64,
    cx: f32,
    cy: f32,
    sx: f32,
    sy: f32,
    fold: f32,
    gravity: f32,
    flux: f32,
    bloom: f32,
    aperture: f32,
    phase: f32,
    seed_phase: [f32; 4],
    void: Color,
    far: Color,
    body: Color,
    hot: Color,
    white: Color,
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
        let safe_time = if time.is_finite() {
            time.clamp(-1.0e6, 1.0e6)
        } else {
            0.0
        };
        let phase = safe_time * p[3];
        let seed_phase =
            std::array::from_fn(|slot| unit(hash(seed, 0x534f_4c, 0, slot as u64)) * TAU);
        let base = palette[0];
        Self {
            seed,
            cx: (w.saturating_sub(1)) as f32 * 0.5,
            cy: (h.saturating_sub(1)) as f32 * 0.5,
            sx: w as f32 * 0.46,
            sy: h as f32 * 0.64,
            fold: p[0],
            gravity: p[1],
            flux: p[2],
            bloom: p[4],
            aperture: p[5],
            phase,
            seed_phase,
            void: darken(base, 26),
            far: lerp_color(darken(palette[2], 38), palette[2], 0.36),
            body: lerp_color(palette[1], shift_hue(palette[3], -18.0), 0.34),
            hot: lighten(shift_hue(palette[3], 14.0), 24),
            white: lighten(palette[4], 24),
        }
    }
}

#[inline]
fn hash(seed: u64, lineage: u64, index: u64, slot: u64) -> u64 {
    let mut z = seed
        ^ lineage.wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ index.wrapping_mul(0xD1B5_4A32_D192_ED03)
        ^ slot.wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 30;
    z = z.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z ^= z >> 27;
    z = z.wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

#[inline]
fn unit(value: u64) -> f32 {
    (value >> 40) as f32 * (1.0 / 16_777_216.0)
}

#[inline]
fn rotate(a: &mut f32, b: &mut f32, angle: f32) {
    let (s, c) = angle.sin_cos();
    let (old_a, old_b) = (*a, *b);
    *a = old_a * c - old_b * s;
    *b = old_a * s + old_b * c;
}

fn project(u: f32, v: f32, look: &Look) -> Projected {
    let seed_ripple =
        0.035 * (u * 3.0 + look.seed_phase[0]).sin() + 0.022 * (v * 2.0 + look.seed_phase[1]).cos();
    let minor = 0.22 + look.aperture * 0.18 + seed_ripple;
    let major = 0.83 + look.aperture * 0.17;
    let mut p = V4 {
        x: (major + minor * v.cos()) * u.cos(),
        y: (major + minor * v.cos()) * u.sin(),
        z: minor * v.sin(),
        w: minor
            * (0.72 * (v + 2.0 * u + look.seed_phase[2]).sin()
                + 0.28 * (2.0 * v - u + look.seed_phase[3]).cos()),
    };

    let fold = look.fold;
    rotate(&mut p.x, &mut p.w, 0.10 + fold * 0.92 + look.phase * 0.13);
    rotate(
        &mut p.y,
        &mut p.w,
        -0.18 + fold * 0.58 + 0.08 * (look.phase * 0.7).sin(),
    );
    rotate(
        &mut p.z,
        &mut p.w,
        look.phase * 0.09 + look.seed_phase[0] * 0.08,
    );
    let stereo = 1.0 / (1.42 - p.w * (0.24 + fold * 0.24)).max(0.42);
    p.x *= stereo;
    p.y *= stereo;
    p.z *= stereo;

    let tilt = 0.91 + 0.13 * (look.seed_phase[1] + look.phase * 0.11).sin();
    rotate(&mut p.y, &mut p.z, tilt);
    rotate(
        &mut p.x,
        &mut p.z,
        (look.phase * 0.06 + look.seed_phase[3] * 0.025) * (0.25 + fold),
    );
    let perspective = 1.0 / (1.28 - p.z * 0.24).max(0.55);
    Projected {
        x: look.cx + p.x * perspective * look.sx,
        y: look.cy + p.y * perspective * look.sy,
        depth: p.z,
    }
}

fn draw(frame: &mut ModeFrame<'_>, p: &[f32; KNOBS]) {
    let w = frame.width.min(frame.grid.first().map_or(0, Vec::len));
    let h = frame.height.min(frame.grid.len());
    if w == 0 || h == 0 {
        return;
    }
    let look = Look::new(frame.seed, w, h, frame.palette, frame.time, p);
    SCRATCH.with(|slot| {
        let mut scratch = slot.borrow_mut();
        let len = w.saturating_mul(h);
        if scratch.len() < len {
            scratch.resize(len, FieldCell::default());
        }
        let field = &mut scratch[..len];
        field.fill(FieldCell::default());

        // Deposit fixed-topology parametric rings into a shared density/depth field.
        measure_layer(NAME, "lattice", || build_lattice(field, &look, w, h));
        // Photons read lattice density while tracing and write irradiance back.
        measure_layer(NAME, "flux", || trace_flux(field, &look, w, h));
        // Irradiance and its local gradient excite the lattice before final paint.
        measure_layer(NAME, "response", || answer_light(field, &look, w, h));
        measure_layer(NAME, "paint", || paint(frame.grid, field, &look, w, h));
    });
}

#[inline]
fn field_index(x: isize, y: isize, w: usize, h: usize) -> Option<usize> {
    if x < 0 || y < 0 || x as usize >= w || y as usize >= h {
        None
    } else {
        Some(y as usize * w + x as usize)
    }
}

#[inline]
fn density_gradient(field: &[FieldCell], x: isize, y: isize, w: usize, h: usize) -> (f32, f32) {
    let read = |sx: isize, sy: isize| {
        field_index(sx, sy, w, h)
            .map(|i| field[i].density)
            .unwrap_or(0.0)
    };
    (
        read(x + 1, y) - read(x - 1, y),
        read(x, y + 1) - read(x, y - 1),
    )
}

fn trace_flux(field: &mut [FieldCell], look: &Look, w: usize, h: usize) {
    let ray_count = h.clamp(14, 30);
    let trail_steps = ((w + h) / 3).clamp(32, 104);
    let core = 0.045 + look.aperture * 0.055;
    let dt = 0.020 + 0.018 * look.gravity;

    for ray in 0..ray_count {
        let ray_id = ray as u64;
        let jitter = unit(hash(look.seed, 0x5048_4f54, ray_id, 0));
        let orbit = 0.43 + 0.58 * unit(hash(look.seed, 0x5048_4f54, ray_id, 1));
        let rate = 0.13 + 0.22 * unit(hash(look.seed, 0x5048_4f54, ray_id, 2));
        let direction = if hash(look.seed, 0x5048_4f54, ray_id, 3) & 1 == 0 {
            1.0
        } else {
            -1.0
        };
        let angle =
            TAU * (ray as f32 / ray_count as f32 + jitter * 0.08) + look.phase * rate * direction;
        let (s, c) = angle.sin_cos();
        let mut px = c * orbit;
        let mut py = s * orbit * (0.62 + 0.12 * look.fold);
        let circular = (0.28 + look.gravity * 0.19) / orbit.sqrt().max(0.4);
        let mut vx = -s * circular * direction;
        let mut vy = c * circular * direction * (0.68 + 0.15 * look.fold);
        let energy = look.flux * (0.58 + 0.62 * unit(hash(look.seed, 0x5048_4f54, ray_id, 4)));

        for step in 0..trail_steps {
            let sx = look.cx + px * look.sx;
            let sy = look.cy + py * look.sy;
            let ix = sx.round() as isize;
            let iy = sy.round() as isize;
            if ix < -2 || iy < -2 || ix > w as isize + 1 || iy > h as isize + 1 {
                break;
            }

            let (gx, gy) = density_gradient(field, ix, iy, w, h);
            let r2 = px * px + py * py + core;
            let inv = r2.sqrt().recip();
            let gravity = look.gravity * 0.052 * inv * inv * inv;
            let density_turn = 0.010 * look.bloom;
            vx += (-px * gravity - gx * density_turn) * dt;
            vy += (-py * gravity - gy * density_turn) * dt;
            let damping = 1.0 - 0.0012 * dt;
            vx *= damping;
            vy *= damping;
            px += vx * dt;
            py += vy * dt;

            let fade = (1.0 - step as f32 / trail_steps as f32).powf(1.35);
            let pulse = 0.68
                + 0.32
                    * (look.phase * 1.7 - step as f32 * 0.23
                        + jitter * TAU
                        + field_index(ix, iy, w, h)
                            .map(|i| field[i].density * 0.3)
                            .unwrap_or(0.0))
                    .sin();
            let deposit = (energy * fade * pulse.max(0.08)).max(0.0);
            let tangent = vy.atan2(vx * (look.sy / look.sx).max(0.1));
            deposit_light(field, w, h, ix, iy, deposit, tangent, look.bloom);
        }
    }
}

fn deposit_light(
    field: &mut [FieldCell],
    w: usize,
    h: usize,
    x: isize,
    y: isize,
    energy: f32,
    tangent: f32,
    bloom: f32,
) {
    let radius = if bloom > 0.65 { 1 } else { 0 };
    for oy in -radius..=radius {
        for ox in -radius..=radius {
            let Some(index) = field_index(x + ox, y + oy, w, h) else {
                continue;
            };
            let spread = if ox == 0 && oy == 0 {
                1.0
            } else {
                0.045 * bloom
            };
            let amount = energy * spread;
            let cell = &mut field[index];
            cell.light = (cell.light + amount).min(6.0);
            if amount > cell.light_peak {
                cell.light_peak = amount;
                cell.light_tangent = tangent;
            }
        }
    }
}

fn answer_light(field: &mut [FieldCell], look: &Look, w: usize, h: usize) {
    for y in 0..h {
        for x in 0..w {
            let index = y * w + x;
            if field[index].density <= 0.0 {
                continue;
            }
            let read = |sx: isize, sy: isize| {
                field_index(sx, sy, w, h)
                    .map(|i| field[i].light)
                    .unwrap_or(0.0)
            };
            let around = (read(x as isize - 1, y as isize)
                + read(x as isize + 1, y as isize)
                + read(x as isize, y as isize - 1)
                + read(x as isize, y as isize + 1))
                * 0.25;
            let depth_gain = ((field[index].depth + 1.0) * 0.5).clamp(0.16, 1.0);
            field[index].response =
                ((field[index].light * 0.72 + around * 0.48) * look.bloom * depth_gain).min(3.0);
        }
    }
}

fn build_lattice(field: &mut [FieldCell], look: &Look, w: usize, h: usize) {
    let u_steps = (w.saturating_mul(2)).clamp(72, 192);
    let v_steps = (h.saturating_mul(4)).clamp(48, 128);

    for ring in 0..9 {
        let v = TAU * ring as f32 / 9.0 + look.seed_phase[0] * 0.015;
        let mut previous = project(0.0, v, look);
        for step in 1..=u_steps {
            let u = TAU * step as f32 / u_steps as f32;
            let current = project(u, v, look);
            deposit_segment(field, w, h, previous, current, 0.74);
            previous = current;
        }
    }

    for spoke in 0..7 {
        let u = TAU * spoke as f32 / 7.0 + look.seed_phase[1] * 0.022;
        let mut previous = project(u, 0.0, look);
        for step in 1..=v_steps {
            let v = TAU * step as f32 / v_steps as f32;
            let current = project(u, v, look);
            deposit_segment(field, w, h, previous, current, 0.92);
            previous = current;
        }
    }
}

fn deposit_segment(
    field: &mut [FieldCell],
    w: usize,
    h: usize,
    a: Projected,
    b: Projected,
    weight: f32,
) {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let steps = dx.abs().max(dy.abs()).ceil().clamp(1.0, 256.0) as usize;
    let tangent = dy.atan2(dx);
    for step in 0..=steps {
        let q = step as f32 / steps as f32;
        let x = (a.x + dx * q).round() as isize;
        let y = (a.y + dy * q).round() as isize;
        if x < 0 || y < 0 || x as usize >= w || y as usize >= h {
            continue;
        }
        let cell = &mut field[y as usize * w + x as usize];
        let depth = a.depth + (b.depth - a.depth) * q;
        cell.density = (cell.density + weight).min(5.0);
        cell.crossings = cell.crossings.saturating_add(1);
        if depth >= cell.depth {
            cell.depth = depth;
            cell.tangent = tangent;
        }
    }
}

fn direction_glyph(angle: f32, crossing: bool, bright: bool) -> char {
    if crossing {
        return if bright { '*' } else { '+' };
    }
    let dx = angle.cos().abs();
    let dy = angle.sin();
    if dy.abs() < dx * 0.42 {
        if bright { '=' } else { '-' }
    } else if dx < dy.abs() * 0.42 {
        if bright { '!' } else { '|' }
    } else if angle.sin() * angle.cos() >= 0.0 {
        '\\'
    } else {
        '/'
    }
}

fn paint(grid: &mut Grid, field: &[FieldCell], look: &Look, w: usize, h: usize) {
    for (y, row) in grid.iter_mut().take(h).enumerate() {
        for (x, cell) in row.iter_mut().take(w).enumerate() {
            let sample = field[y * w + x];
            *cell = Cell::with_bg(' ', look.far, look.void);
            let ux = (x as f32 - look.cx) / (w as f32 * (0.065 + look.aperture * 0.075));
            let uy = (y as f32 - look.cy) / (h as f32 * (0.055 + look.aperture * 0.065));
            let umbra = ux * ux + uy * uy;
            if umbra < 1.0 {
                continue;
            }
            let light = 1.0 - (-sample.light * 0.34).exp();
            if sample.density <= 0.0 {
                if light > 0.115 {
                    let ch = if light > 0.78 {
                        '*'
                    } else if light > 0.48 {
                        direction_glyph(sample.light_tangent, false, false)
                    } else if light > 0.23 {
                        '~'
                    } else {
                        '.'
                    };
                    let color = lerp_color(look.body, look.hot, light);
                    *cell =
                        Cell::with_bg(ch, if light > 0.82 { look.white } else { color }, look.void);
                    continue;
                }

                let px = (x as f32 - look.cx) / look.sx.max(1.0);
                let py = (y as f32 - look.cy) / look.sy.max(1.0);
                let radius = (px * px + py * py).sqrt();
                let diffraction = (radius * (17.0 + look.gravity * 4.0) - look.phase * 0.28
                    + look.seed_phase[2])
                    .sin();
                let azimuth = (py.atan2(px) * 7.0 + look.seed_phase[3]).cos();
                if radius > 0.32
                    && radius < 1.24
                    && diffraction + azimuth * 0.22 > 1.18
                    && unit(hash(look.seed, 0x4449_4646, x as u64, y as u64)) > 0.84
                {
                    *cell = Cell::with_bg('.', look.far, look.void);
                }
                continue;
            }
            let front = ((sample.depth + 0.8) / 1.6).clamp(0.0, 1.0);
            let mass = (sample.density / 2.5).clamp(0.0, 1.0);
            let lit = (light * look.flux + sample.response * 0.18).clamp(0.0, 1.0);
            let color = lerp_color(
                lerp_color(look.far, look.body, front),
                look.hot,
                (lit * 0.72 + mass * 0.18).clamp(0.0, 1.0),
            );
            let bright = front + lit + mass > 1.35;
            let crossing = sample.density > 2.8;
            let response_mix = (sample.response * 0.32).clamp(0.0, 0.72);
            let tangent =
                sample.tangent * (1.0 - response_mix) + sample.light_tangent * response_mix;
            *cell = Cell::with_bg(
                direction_glyph(tangent, crossing, bright),
                if bright && lit > 0.72 {
                    look.white
                } else {
                    color
                },
                look.void,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    fn defaults() -> [f32; KNOBS] {
        std::array::from_fn(|i| PARAMS[i].default)
    }

    fn frame(w: usize, h: usize, seed: u64, time: f32, p: &[f32; KNOBS]) -> Grid {
        let mut grid = vec![vec![Cell::blank(); w]; h];
        let palette = crate::color::make_palette(seed);
        let mut rng = StdRng::seed_from_u64(seed);
        let args = Vec::new();
        MODE.render(&mut ModeFrame {
            grid: &mut grid,
            width: w,
            height: h,
            seed,
            palette: &palette,
            rng: &mut rng,
            time,
            args: &args,
            param_values: Some(p),
        });
        grid
    }

    fn text(grid: &Grid) -> String {
        crate::render::grid_to_plain(grid).join("\n")
    }

    #[test]
    fn sol_reliquary_foundation_80x24() {
        insta::assert_snapshot!(
            "sol_reliquary_foundation_80x24",
            text(&frame(80, 24, 42, 0.0, &defaults()))
        );
    }

    #[test]
    fn sol_reliquary_motion_middle() {
        insta::assert_snapshot!(
            "sol_reliquary_motion_t2_75",
            text(&frame(80, 24, 42, 2.75, &defaults()))
        );
    }

    #[test]
    fn sol_reliquary_motion_later() {
        insta::assert_snapshot!(
            "sol_reliquary_motion_t7",
            text(&frame(80, 24, 42, 7.0, &defaults()))
        );
    }

    #[test]
    fn sol_reliquary_open_aperture_variant() {
        let mut p = defaults();
        p[0] = 0.24;
        p[1] = 1.45;
        p[5] = 0.94;
        insta::assert_snapshot!(
            "sol_reliquary_open_aperture",
            text(&frame(80, 24, 1701, 2.75, &p))
        );
    }

    #[test]
    fn sol_reliquary_small_clip() {
        insta::assert_snapshot!(
            "sol_reliquary_small_34x10",
            text(&frame(34, 10, 7, 3.5, &defaults()))
        );
    }

    #[test]
    fn sol_reliquary_nearby_fold_049() {
        let mut p = defaults();
        p[0] = 0.49;
        insta::assert_snapshot!("sol_reliquary_fold_049", text(&frame(80, 24, 314, 3.0, &p)));
    }

    #[test]
    fn sol_reliquary_nearby_fold_051() {
        let mut p = defaults();
        p[0] = 0.51;
        insta::assert_snapshot!("sol_reliquary_fold_051", text(&frame(80, 24, 314, 3.0, &p)));
    }

    #[test]
    fn fold_sweep_gallery() {
        let mut panels = Vec::new();
        for fold in [0.15, 0.45, 0.49, 0.51, 0.85] {
            let mut p = defaults();
            p[0] = fold;
            panels.push(format!(
                "fold={fold:.2}\n{}",
                text(&frame(56, 16, 314, 3.0, &p))
            ));
        }
        insta::assert_snapshot!("sol_reliquary_fold_sweep", panels.join("\n\n"));
    }

    #[test]
    fn seed_gallery() {
        let panels = [7, 42, 1701]
            .into_iter()
            .map(|seed| {
                format!(
                    "seed={seed}\n{}",
                    text(&frame(56, 16, seed, 2.75, &defaults()))
                )
            })
            .collect::<Vec<_>>();
        insta::assert_snapshot!("sol_reliquary_seed_gallery", panels.join("\n\n"));
    }

    #[test]
    fn same_inputs_are_cell_exact() {
        assert_eq!(
            frame(80, 24, 1701, 2.5, &defaults()),
            frame(80, 24, 1701, 2.5, &defaults())
        );
    }

    #[test]
    fn seeds_change_the_structure() {
        assert_ne!(
            text(&frame(80, 24, 41, 2.5, &defaults())),
            text(&frame(80, 24, 42, 2.5, &defaults()))
        );
    }

    #[test]
    fn tiny_grids_clip_without_panicking() {
        for (w, h) in [(0, 0), (1, 1), (2, 3), (7, 4), (12, 6)] {
            let grid = frame(w, h, 7, 1.0, &defaults());
            assert_eq!(grid.len(), h);
            assert!(grid.iter().all(|row| row.len() == w));
        }
    }
}
