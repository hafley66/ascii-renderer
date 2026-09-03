use crossterm::style::Color;
use rand::rngs::StdRng;

use crate::_0_profile::measure_layer;
use crate::color::{darken, lerp_color, lighten, shift_hue};
use crate::opts::param_f32;
use crate::pp::{pp_hash2, pp_stroke};
use crate::registry::{AnimKind, Mode, ModeFrame, Param};
use crate::types::{Cell, Grid};
use super::_33_cosmograph::FbmRow;

const PI: f32 = std::f32::consts::PI;
const TAU: f32 = std::f32::consts::TAU;

pub(super) struct AetherforgeMode;

pub(super) static MODE: AetherforgeMode = AetherforgeMode;

const PARAMS: &[Param] = &[
    param!("HALOS", "toothed forge halos", 2.0, 12.0, 7.0, 1.0),
    param!("TEETH", "mechanism teeth", 6.0, 48.0, 24.0, 2.0),
    param!("PETALS", "spindle symmetry", 3.0, 24.0, 11.0, 1.0),
    param!("CRUCIBLES", "orbital crucibles", 1.0, 16.0, 8.0, 1.0),
    param!("SPARKS", "star-metal sparks", 0.0, 220.0, 96.0, 4.0),
    param!("RIVERS", "magnetic flux rivers", 1.0, 14.0, 7.0, 1.0),
    param!("SPEED", "forge velocity", 0.05, 3.0, 0.68, 0.05),
    param!("ECC", "orbital eccentricity", 0.0, 1.5, 0.72, 0.05),
    param!("FLUX", "field distortion", 0.0, 1.5, 0.88, 0.05),
    param!("TRAIL", "spark memory", 1.0, 24.0, 13.0, 1.0),
    param!("DEPTH", "nested spindle depth", 1.0, 9.0, 6.0, 1.0),
    param!("BLOOM", "spectral bloom", 0.0, 1.5, 0.92, 0.05),
    param!("GATES", "architectural gates", 0.0, 6.0, 4.0, 1.0),
    param!("PULSE", "reactor pulse", 0.0, 2.0, 1.0, 0.1),
];

impl Mode for AetherforgeMode {
    fn name(&self) -> &'static str {
        "aetherforge"
    }

    fn help(&self) -> &'static str {
        "Kinetic star-metal foundry: toothed halo mechanisms, magnetic rivers, nested rose spindle, orbital crucibles, spark memory, mirrored forge gates [halos] [teeth] [petals] [crucibles] [sparks] [rivers] [speed] [ecc] [flux] [trail] [depth] [bloom] [gates] [pulse]"
    }

    fn animation(&self) -> AnimKind {
        AnimKind::Iterate
    }

    fn params(&self) -> &'static [Param] {
        PARAMS
    }

    fn render(&self, frame: &mut ModeFrame<'_>) {
        let params = AetherforgeParams::from_inputs(frame.args, frame.param_values);
        draw_aetherforge(
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
pub(crate) struct AetherforgeParams {
    pub(crate) halos: usize,
    pub(crate) teeth: usize,
    pub(crate) petals: usize,
    pub(crate) crucibles: usize,
    pub(crate) sparks: usize,
    pub(crate) rivers: usize,
    pub(crate) speed: f32,
    pub(crate) eccentricity: f32,
    pub(crate) flux: f32,
    pub(crate) trail: usize,
    pub(crate) depth: usize,
    pub(crate) bloom: f32,
    pub(crate) gates: usize,
    pub(crate) pulse: f32,
}

impl Default for AetherforgeParams {
    fn default() -> Self {
        Self {
            halos: 7,
            teeth: 24,
            petals: 11,
            crucibles: 8,
            sparks: 96,
            rivers: 7,
            speed: 0.68,
            eccentricity: 0.72,
            flux: 0.88,
            trail: 13,
            depth: 6,
            bloom: 0.92,
            gates: 4,
            pulse: 1.0,
        }
    }
}

impl AetherforgeParams {
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
            halos: read(4, "HALOS", 7.0).round().clamp(2.0, 12.0) as usize,
            teeth: read(5, "TEETH", 24.0).round().clamp(6.0, 48.0) as usize,
            petals: read(6, "PETALS", 11.0).round().clamp(3.0, 24.0) as usize,
            crucibles: read(7, "CRUCIBLES", 8.0).round().clamp(1.0, 16.0) as usize,
            sparks: read(8, "SPARKS", 96.0).round().clamp(0.0, 220.0) as usize,
            rivers: read(9, "RIVERS", 7.0).round().clamp(1.0, 14.0) as usize,
            speed: read(10, "SPEED", 0.68).clamp(0.05, 3.0),
            eccentricity: read(11, "ECC", 0.72).clamp(0.0, 1.5),
            flux: read(12, "FLUX", 0.88).clamp(0.0, 1.5),
            trail: read(13, "TRAIL", 13.0).round().clamp(1.0, 24.0) as usize,
            depth: read(14, "DEPTH", 6.0).round().clamp(1.0, 9.0) as usize,
            bloom: read(15, "BLOOM", 0.92).clamp(0.0, 1.5),
            gates: read(16, "GATES", 4.0).round().clamp(0.0, 6.0) as usize,
            pulse: read(17, "PULSE", 1.0).clamp(0.0, 2.0),
        }
    }
}

fn hash01(seed: u64, tag: u64) -> f32 {
    let mut value = seed ^ tag.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    value ^= value >> 30;
    value = value.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^= value >> 31;
    ((value >> 40) as f32) / ((1u64 << 24) - 1) as f32
}

fn put(grid: &mut Grid, x: i32, y: i32, ch: char, fg: Color) {
    if x < 0 || y < 0 {
        return;
    }
    let (x, y) = (x as usize, y as usize);
    if let Some(row) = grid.get_mut(y)
        && let Some(cell) = row.get_mut(x)
    {
        let bg = cell.bg;
        *cell = Cell::with_bg(ch, fg, bg);
    }
}

fn put_bg(grid: &mut Grid, x: usize, y: usize, bg: Color) {
    if let Some(row) = grid.get_mut(y)
        && let Some(cell) = row.get_mut(x)
    {
        cell.bg = bg;
    }
}

fn line(grid: &mut Grid, mut x0: i32, mut y0: i32, x1: i32, y1: i32, fg: Color) {
    let ch = pp_stroke(x1 - x0, y1 - y0);
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut error = dx + dy;
    loop {
        put(grid, x0, y0, ch, fg);
        if x0 == x1 && y0 == y1 {
            break;
        }
        let twice = error * 2;
        if twice >= dy {
            error += dy;
            x0 += sx;
        }
        if twice <= dx {
            error += dx;
            y0 += sy;
        }
    }
}

fn stroke_curve(
    grid: &mut Grid,
    samples: usize,
    color: Color,
    mut point: impl FnMut(f32) -> (f32, f32),
) {
    let mut previous = None;
    for sample in 0..=samples.max(1) {
        let u = sample as f32 / samples.max(1) as f32;
        let current_f = point(u);
        let current = (current_f.0.round() as i32, current_f.1.round() as i32);
        if let Some((px, py)) = previous
            && current != (px, py)
        {
            line(grid, px, py, current.0, current.1, color);
        }
        previous = Some(current);
    }
}

fn polygon(grid: &mut Grid, points: &[(f32, f32)], color: Color) {
    if points.len() < 2 {
        return;
    }
    for index in 0..points.len() {
        let from = points[index];
        let to = points[(index + 1) % points.len()];
        line(
            grid,
            from.0.round() as i32,
            from.1.round() as i32,
            to.0.round() as i32,
            to.1.round() as i32,
            color,
        );
    }
}

fn ellipse_point(cx: f32, cy: f32, rx: f32, ry: f32, angle: f32) -> (f32, f32) {
    (cx + angle.cos() * rx, cy + angle.sin() * ry)
}

fn draw_star_metal_field(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    params: &AetherforgeParams,
) {
    if width == 0 || height == 0 {
        return;
    }
    let inv_w = 1.0 / width.max(1) as f32;
    let inv_h = 1.0 / height.max(1) as f32;
    let threshold = 0.94 - params.bloom * 0.075;
    let hot_threshold = threshold + 0.075;
    let bloom_on = params.bloom > 0.25;
    let bloom_bg = darken(palette[0], (20.0 - params.bloom * 8.0) as u8);
    let res_t_a = t * params.speed * 0.16;
    let res_t_b = t * 0.11;
    let noise_seed = seed + 101;
    let noise_fx_bias = t * 0.025;
    let noise_fy_bias = t * 0.018;
    let mut noise_fx = Vec::with_capacity(width);
    let mut res_ax = Vec::with_capacity(width);
    let mut res_bx = Vec::with_capacity(width);
    for x in 0..width {
        let nx = x as f32 * inv_w;
        noise_fx.push(nx * 5.0 + noise_fx_bias);
        res_ax.push(nx * 13.0);
        res_bx.push(nx * 3.0);
    }
    for y in 0..height {
        let ny = y as f32 * inv_h;
        let res_ay = ny * 9.0;
        let res_by = ny * 17.0;
        let mut noise_row = FbmRow::new(ny * 7.0 - noise_fy_bias, noise_seed);
        for x in 0..width {
            let noise = noise_row.at(noise_fx[x]);
            let noise_term = noise * 0.72;
            // Resonance is bounded by 1, so a cell that misses the gate at
            // resonance 1 never needs its sin/cos pair evaluated.
            if noise_term + 0.28 > threshold {
                let resonance = (((res_ax[x] - res_ay) + res_t_a).sin()
                    * ((res_by + res_bx[x]) - res_t_b).cos())
                .abs();
                if noise_term + resonance * 0.28 > threshold {
                    let hot = noise > hot_threshold;
                    let glyph = if hot {
                        '✦'
                    } else if resonance > 0.76 {
                        '∙'
                    } else {
                        '·'
                    };
                    let color = if hot {
                        lighten(palette[3], 30)
                    } else {
                        darken(lerp_color(palette[1], palette[2], resonance), 55)
                    };
                    put(grid, x as i32, y as i32, glyph, color);
                }
            }
            if bloom_on && noise > 0.89 {
                put_bg(grid, x, y, bloom_bg);
            }
        }
    }
}

fn draw_flux_rivers(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    params: &AetherforgeParams,
) {
    let w = width.saturating_sub(1) as f32;
    let h = height.saturating_sub(1) as f32;
    let samples = (width * 2).clamp(24, 720);
    for river in 0..params.rivers {
        let fraction = (river + 1) as f32 / (params.rivers + 1) as f32;
        let phase = hash01(seed, 200 + river as u64) * TAU;
        let amplitude =
            h * (0.028 + params.flux * 0.017) * (0.7 + hash01(seed, 300 + river as u64));
        let frequency = 1.0 + (river % 4) as f32 * 0.5;
        let base_y = fraction * h;
        let color = darken(
            lerp_color(palette[1], palette[2], fraction),
            (48.0 - params.bloom * 18.0).max(0.0) as u8,
        );
        stroke_curve(grid, samples, color, |u| {
            let x = u * w;
            let lens = (u * PI).sin().powi(2);
            let first =
                (u * TAU * frequency + phase + t * params.speed * (0.22 + fraction * 0.19)).sin();
            let second = (u * TAU * (frequency + 1.5) - phase * 0.7 - t * 0.31).cos();
            (
                x,
                base_y + amplitude * lens * (first * 0.72 + second * 0.28),
            )
        });
    }
    for river in 0..params.rivers.min(8) {
        let fraction = (river + 1) as f32 / (params.rivers.min(8) + 1) as f32;
        let phase = hash01(seed, 400 + river as u64) * TAU;
        let base_x = fraction * w;
        let color = darken(shift_hue(palette[2], fraction as f64 * 80.0), 60);
        stroke_curve(grid, (height * 2).clamp(16, 360), color, |u| {
            let y = u * h;
            let bend = (u * PI).sin() * w * 0.018 * params.flux;
            (
                base_x + bend * (u * TAU * 2.0 + phase - t * params.speed * 0.24).sin(),
                y,
            )
        });
    }
}

fn draw_forge_gates(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    params: &AetherforgeParams,
) {
    if params.gates == 0 {
        return;
    }
    let cx = width as f32 * 0.5;
    let cy = height as f32 * 0.5;
    let span_x = width as f32 * 0.47;
    let top = 1.0;
    let bottom = height.saturating_sub(2) as f32;
    for gate in 0..params.gates {
        let inset = gate as f32 * (width as f32 * 0.018 + 1.0);
        let left = (cx - span_x + inset).max(1.0);
        let right = (cx + span_x - inset).min(width.saturating_sub(2) as f32);
        let cap_y = top + gate as f32 * 0.75;
        let foot_y = bottom - gate as f32 * 0.45;
        let color = darken(
            lerp_color(
                palette[1],
                palette[3],
                gate as f32 / params.gates.max(1) as f32,
            ),
            (gate * 6) as u8,
        );
        line(
            grid,
            left as i32,
            cap_y as i32,
            left as i32,
            foot_y as i32,
            color,
        );
        line(
            grid,
            right as i32,
            cap_y as i32,
            right as i32,
            foot_y as i32,
            color,
        );
        let arch_phase = t * params.speed * 0.18 + hash01(seed, 500 + gate as u64) * TAU;
        stroke_curve(grid, (width + height).clamp(24, 420), color, |u| {
            let angle = PI + u * PI;
            let rx = (right - left) * 0.5;
            let rise = height as f32 * (0.12 + gate as f32 * 0.012);
            (
                cx + angle.cos() * rx,
                cy + angle.sin() * rise + arch_phase.sin() * 0.3,
            )
        });
        let braces = 3 + gate;
        for brace in 0..braces {
            let v = brace as f32 / braces.max(1) as f32;
            let y = cap_y + (foot_y - cap_y) * v;
            let tooth = if (brace + gate) % 2 == 0 {
                '◆'
            } else {
                '◇'
            };
            put(grid, left as i32, y as i32, tooth, lighten(color, 18));
            put(grid, right as i32, y as i32, tooth, lighten(color, 18));
            let reach = (width as f32 * (0.015 + gate as f32 * 0.002)).max(2.0);
            line(
                grid,
                left as i32,
                y as i32,
                (left + reach) as i32,
                (y + 1.0) as i32,
                color,
            );
            line(
                grid,
                right as i32,
                y as i32,
                (right - reach) as i32,
                (y + 1.0) as i32,
                color,
            );
        }
    }
}

fn draw_halo_mechanisms(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    params: &AetherforgeParams,
) {
    let cx = width as f32 * 0.5;
    let cy = height as f32 * 0.5;
    let max_rx = (width as f32 * 0.39).min(height as f32 * 1.55).max(2.0);
    let max_ry = (height as f32 * 0.39).min(width as f32 * 0.22).max(1.0);
    let samples = ((width + height) * 4).clamp(64, 1100);
    for halo in (0..params.halos).rev() {
        let fraction = (halo + 1) as f32 / params.halos as f32;
        let rx = max_rx * (0.24 + fraction * 0.76);
        let ry = max_ry * (0.24 + fraction * 0.76);
        let direction = if halo % 2 == 0 { 1.0 } else { -1.0 };
        let phase = direction * t * params.speed * (0.12 + halo as f32 * 0.035)
            + hash01(seed, 700 + halo as u64) * TAU;
        let tooth_count = (params.teeth as f32 * (0.55 + fraction * 0.7))
            .round()
            .max(6.0);
        let color = lerp_color(
            darken(palette[1], (halo * 4) as u8),
            palette[3],
            (1.0 - fraction) * params.bloom.min(1.0),
        );
        stroke_curve(grid, samples, color, |u| {
            let a = u * TAU;
            let tooth = (a * tooth_count + phase * 3.0).cos().abs().powf(9.0);
            let corrugation = 1.0 + tooth * (0.018 + 0.015 * params.eccentricity);
            let wobble = 1.0 + (a * (halo + 2) as f32 - phase).sin() * 0.012 * params.flux;
            ellipse_point(
                cx,
                cy,
                rx * corrugation * wobble,
                ry * corrugation / wobble,
                a + phase,
            )
        });
        let node_count = (params.teeth / 3).clamp(4, 16);
        for node in 0..node_count {
            let a = node as f32 / node_count as f32 * TAU + phase;
            let p = ellipse_point(cx, cy, rx, ry, a);
            let glyph = match (node + halo) % 4 {
                0 => '◈',
                1 => '○',
                2 => '✧',
                _ => '¤',
            };
            put(
                grid,
                p.0.round() as i32,
                p.1.round() as i32,
                glyph,
                lighten(color, 22),
            );
            if halo % 2 == 0 && node % 2 == 0 {
                let inner = ellipse_point(cx, cy, rx * 0.93, ry * 0.93, a + 0.05 * direction);
                line(
                    grid,
                    p.0.round() as i32,
                    p.1.round() as i32,
                    inner.0.round() as i32,
                    inner.1.round() as i32,
                    darken(color, 18),
                );
            }
        }
    }
}

fn draw_reactor_spindle(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    params: &AetherforgeParams,
) {
    let cx = width as f32 * 0.5;
    let cy = height as f32 * 0.5;
    let base_rx = (width as f32 * 0.20).min(height as f32 * 0.82).max(2.0);
    let base_ry = (height as f32 * 0.205).min(width as f32 * 0.11).max(1.0);
    let beat =
        1.0 + (t * params.speed * 1.7 + hash01(seed, 900) * TAU).sin() * 0.035 * params.pulse;
    for depth in (0..params.depth).rev() {
        let fraction = (depth + 1) as f32 / params.depth as f32;
        let rx = base_rx * fraction * beat;
        let ry = base_ry * fraction * beat;
        let direction = if depth % 2 == 0 { 1.0 } else { -1.0 };
        let phase = direction * t * params.speed * (0.18 + depth as f32 * 0.05)
            + hash01(seed, 920 + depth as u64) * TAU;
        let petals = params.petals + depth % 3;
        let color = lerp_color(palette[2], lighten(palette[3], 35), 1.0 - fraction * 0.72);
        stroke_curve(grid, ((width + height) * 3).clamp(80, 800), color, |u| {
            let a = u * TAU;
            let rose = (a * petals as f32 * 0.5 + phase).cos().abs();
            let radius = 0.28 + rose.powf(1.25) * 0.72;
            let torsion = (a * 3.0 - phase * 1.4).sin() * params.flux * 0.035;
            ellipse_point(cx, cy, rx * radius, ry * radius, a + torsion)
        });
    }

    let core_radius = (base_ry * 0.32 * beat).max(1.4);
    let facets = params.petals.clamp(5, 16);
    let phase = -t * params.speed * 0.7;
    let mut outer = Vec::with_capacity(facets);
    let mut inner = Vec::with_capacity(facets);
    for facet in 0..facets {
        let a = facet as f32 / facets as f32 * TAU + phase;
        outer.push(ellipse_point(cx, cy, core_radius * 2.1, core_radius, a));
        inner.push(ellipse_point(
            cx,
            cy,
            core_radius * 1.15,
            core_radius * 0.55,
            -a * 1.7,
        ));
    }
    polygon(grid, &outer, lighten(palette[3], 45));
    polygon(grid, &inner, palette[4]);
    for index in 0..facets {
        line(
            grid,
            outer[index].0.round() as i32,
            outer[index].1.round() as i32,
            inner[(index * 3) % facets].0.round() as i32,
            inner[(index * 3) % facets].1.round() as i32,
            lerp_color(palette[1], palette[3], index as f32 / facets as f32),
        );
    }
    put(
        grid,
        cx.round() as i32,
        cy.round() as i32,
        '✺',
        lighten(palette[4], 20),
    );
}

fn draw_crucibles(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    params: &AetherforgeParams,
) {
    let cx = width as f32 * 0.5;
    let cy = height as f32 * 0.5;
    let rx = (width as f32 * 0.32).min(height as f32 * 1.3).max(2.0);
    let ry = (height as f32 * 0.32).min(width as f32 * 0.18).max(1.0);
    for crucible in 0..params.crucibles {
        let identity = hash01(seed, 1100 + crucible as u64);
        let direction = if crucible % 2 == 0 { 1.0 } else { -1.0 };
        let speed = params.speed * (0.16 + identity * 0.22);
        let angle = crucible as f32 / params.crucibles as f32 * TAU
            + identity * 0.8
            + t * speed * direction;
        let eccentric = 1.0 + params.eccentricity * 0.16 * (angle * 2.0 + identity * TAU).sin();
        let point = ellipse_point(cx, cy, rx * eccentric, ry / eccentric.max(0.2), angle);
        let color = shift_hue(palette[3], identity as f64 * 100.0 - 50.0);

        let chain_from = ellipse_point(cx, cy, rx * 0.82, ry * 0.82, angle - direction * 0.035);
        line(
            grid,
            chain_from.0.round() as i32,
            chain_from.1.round() as i32,
            point.0.round() as i32,
            point.1.round() as i32,
            darken(color, 35),
        );

        let size = 1.0 + (identity * 2.2).floor();
        let vessel = [
            (point.0, point.1 - size),
            (point.0 + size * 1.6, point.1),
            (point.0 + size * 0.6, point.1 + size),
            (point.0 - size * 0.6, point.1 + size),
            (point.0 - size * 1.6, point.1),
        ];
        polygon(grid, &vessel, lighten(color, 20));
        put(
            grid,
            point.0.round() as i32,
            point.1.round() as i32,
            '♨',
            palette[4],
        );
        let drip_phase = (t * (0.9 + identity) + identity * 8.0).rem_euclid(1.0);
        for drip in 0..3 {
            let drop_y = point.1 + size + drip_phase * (2.0 + drip as f32 * 1.4);
            put(
                grid,
                (point.0 + (drip as f32 - 1.0) * 0.8).round() as i32,
                drop_y.round() as i32,
                if drip == 1 { '♦' } else { '·' },
                color,
            );
        }
    }
}

fn draw_star_sparks(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    params: &AetherforgeParams,
) {
    let cx = width as f32 * 0.5;
    let cy = height as f32 * 0.5;
    let max_rx = (width as f32 * 0.43).min(height as f32 * 1.75).max(2.0);
    let max_ry = (height as f32 * 0.42).min(width as f32 * 0.24).max(1.0);
    for spark in 0..params.sparks {
        let identity = hash01(seed, 2000 + spark as u64);
        let birth = hash01(seed, 2400 + spark as u64);
        let lifetime = 1.7 + hash01(seed, 2800 + spark as u64) * 3.6;
        let clock =
            (t * params.speed * (0.65 + identity * 0.8) + birth * lifetime).rem_euclid(lifetime);
        let age = clock / lifetime;
        let base_angle = identity * TAU * 3.0 + spark as f32 * 2.399_963;
        let spin = TAU * (0.35 + hash01(seed, 3200 + spark as u64) * 1.1);
        for trail in 0..params.trail {
            let lag = trail as f32 / params.trail.max(1) as f32 * 0.22;
            let trail_age = (age - lag).rem_euclid(1.0);
            let expansion = trail_age.powf(0.72);
            let curl = base_angle + spin * trail_age + params.flux * (trail_age * PI).sin();
            let gravity = trail_age * trail_age * height as f32 * 0.08;
            let x = cx + curl.cos() * max_rx * expansion * (0.35 + identity * 0.65);
            let y =
                cy + curl.sin() * max_ry * expansion - gravity + trail_age * height as f32 * 0.04;
            let head = trail == 0;
            let glyph = if head {
                match spark % 4 {
                    0 => '✦',
                    1 => '♦',
                    2 => '∗',
                    _ => '•',
                }
            } else if trail < params.trail / 3 {
                '·'
            } else {
                '˙'
            };
            let fade = 1.0 - trail as f32 / params.trail.max(1) as f32;
            let color = lerp_color(darken(palette[1], 60), lighten(palette[3], 35), fade);
            put(grid, x.round() as i32, y.round() as i32, glyph, color);
        }
    }
}

fn draw_runic_border(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    params: &AetherforgeParams,
) {
    if width == 0 || height == 0 {
        return;
    }
    let top = 0;
    let bottom = height.saturating_sub(1) as i32;
    let right = width.saturating_sub(1) as i32;
    let offset = (t * params.speed * 8.0).floor() as i32;
    let runes = ['╬', '◊', '═', '✧', '╪', '◇', '═', '¤'];
    for x in 0..width {
        let index = (x as i32 + offset).rem_euclid(runes.len() as i32) as usize;
        let color = if index % 2 == 0 {
            palette[1]
        } else {
            palette[3]
        };
        put(grid, x as i32, top, runes[index], darken(color, 10));
        put(
            grid,
            x as i32,
            bottom,
            runes[(runes.len() - 1 - index + (seed as usize % runes.len())) % runes.len()],
            darken(shift_hue(color, 35.0), 10),
        );
    }
    for y in 1..height.saturating_sub(1) {
        let index = (y as i32 * 2 - offset).rem_euclid(runes.len() as i32) as usize;
        put(
            grid,
            0,
            y as i32,
            if index % 2 == 0 { '║' } else { '◆' },
            palette[2],
        );
        put(
            grid,
            right,
            y as i32,
            if index % 2 == 0 { '║' } else { '◇' },
            palette[2],
        );
    }
    put(grid, 0, top, '╔', lighten(palette[3], 20));
    put(grid, right, top, '╗', lighten(palette[3], 20));
    put(grid, 0, bottom, '╚', lighten(palette[3], 20));
    put(grid, right, bottom, '╝', lighten(palette[3], 20));
}

pub(crate) fn draw_aetherforge(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    _rng: &mut StdRng,
    t: f32,
    params: &AetherforgeParams,
) {
    if width == 0 || height == 0 || grid.is_empty() {
        return;
    }
    // 1. Initialize sparse star-metal and background bloom from explicit frame inputs.
    measure_layer("aetherforge", "star_metal", || {
        draw_star_metal_field(grid, width, height, seed, palette, t, params)
    });
    // 2. Evaluate bounded horizontal and vertical magnetic rivers behind the mechanism.
    measure_layer("aetherforge", "flux_rivers", || {
        draw_flux_rivers(grid, width, height, seed, palette, t, params)
    });
    // 3. Reconstruct every ballistic spiral spark and its fading trail from time.
    measure_layer("aetherforge", "star_sparks", || {
        draw_star_sparks(grid, width, height, seed, palette, t, params)
    });
    // 4. Raise mirrored architectural gates around the active foundry volume.
    measure_layer("aetherforge", "forge_gates", || {
        draw_forge_gates(grid, width, height, seed, palette, t, params)
    });
    // 5. Rotate corrugated halo mechanisms with stable seeded phases and nodes.
    measure_layer("aetherforge", "halo_mechanisms", || {
        draw_halo_mechanisms(grid, width, height, seed, palette, t, params)
    });
    // 6. Draw the nested many-petaled spindle and faceted central reactor.
    measure_layer("aetherforge", "reactor_spindle", || {
        draw_reactor_spindle(grid, width, height, seed, palette, t, params)
    });
    // 7. Orbit crucibles analytically, including chains, molten contents, and drips.
    measure_layer("aetherforge", "crucibles", || {
        draw_crucibles(grid, width, height, seed, palette, t, params)
    });
    // 8. Seal the composition with a moving runic machine border.
    measure_layer("aetherforge", "runic_border", || {
        draw_runic_border(grid, width, height, seed, palette, t, params)
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::make_palette;
    use crate::render::grid_to_plain;
    use rand::SeedableRng;

    fn frame(width: usize, height: usize, seed: u64, t: f32, params: &AetherforgeParams) -> Grid {
        let mut grid = vec![vec![Cell::blank(); width]; height];
        let palette = make_palette(seed);
        let mut rng = StdRng::seed_from_u64(seed);
        draw_aetherforge(
            &mut grid, width, height, seed, &palette, &mut rng, t, params,
        );
        grid
    }

    fn plain(grid: &Grid) -> String {
        grid_to_plain(grid).join("\n")
    }

    #[test]
    fn deterministic_seeded_frame_and_visible_motion() {
        let params = AetherforgeParams::default();
        let a = plain(&frame(100, 34, 42, 1.25, &params));
        let b = plain(&frame(100, 34, 42, 1.25, &params));
        let moved = plain(&frame(100, 34, 42, 3.75, &params));
        let reseeded = plain(&frame(100, 34, 43, 1.25, &params));
        let varied = plain(&frame(
            100,
            34,
            42,
            1.25,
            &AetherforgeParams {
                halos: 10,
                petals: 17,
                crucibles: 13,
                rivers: 11,
                ..params
            },
        ));
        assert_eq!(a, b);
        assert_ne!(a, moved);
        assert_ne!(a, reseeded);
        assert_ne!(a, varied);
    }

    #[test]
    fn parameter_values_override_and_clamp() {
        let values = [
            99.0, 0.0, 99.0, 99.0, 999.0, 99.0, 0.0, 99.0, 99.0, 0.0, 99.0, 99.0, 99.0, 99.0,
        ];
        let params = AetherforgeParams::from_inputs(&[], Some(&values));
        insta::assert_debug_snapshot!(params, @r###"
        AetherforgeParams {
            halos: 12,
            teeth: 6,
            petals: 24,
            crucibles: 16,
            sparks: 220,
            rivers: 14,
            speed: 0.05,
            eccentricity: 1.5,
            flux: 1.5,
            trail: 1,
            depth: 9,
            bloom: 1.5,
            gates: 6,
            pulse: 2.0,
        }
        "###);
    }

    #[test]
    fn tiny_grids_and_parameter_extrema_terminate() {
        let maxed = AetherforgeParams {
            halos: 12,
            teeth: 48,
            petals: 24,
            crucibles: 16,
            sparks: 220,
            rivers: 14,
            speed: 3.0,
            eccentricity: 1.5,
            flux: 1.5,
            trail: 24,
            depth: 9,
            bloom: 1.5,
            gates: 6,
            pulse: 2.0,
        };
        for size in [(1usize, 1usize), (2, 1), (3, 5), (9, 2)] {
            let grid = frame(size.0, size.1, 7, 50_000.0, &maxed);
            assert_eq!(grid.len(), size.1);
            assert!(grid.iter().all(|row| row.len() == size.0));
        }
    }

    #[test]
    fn disabled_optional_layers_change_the_composition() {
        let base = AetherforgeParams::default();
        let full = plain(&frame(90, 30, 77, 2.4, &base));
        let sparse = plain(&frame(
            90,
            30,
            77,
            2.4,
            &AetherforgeParams {
                sparks: 0,
                gates: 0,
                bloom: 0.0,
                pulse: 0.0,
                ..base
            },
        ));
        assert_ne!(full, sparse);
    }

    #[test]
    fn snapshot_aetherforge_t0() {
        insta::assert_snapshot!(plain(&frame(
            104,
            36,
            42,
            0.0,
            &AetherforgeParams::default()
        )));
    }

    #[test]
    fn snapshot_aetherforge_in_motion() {
        insta::assert_snapshot!(plain(&frame(
            104,
            36,
            42,
            2.75,
            &AetherforgeParams::default()
        )));
    }
}
