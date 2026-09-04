use crossterm::style::Color;
use rand::rngs::StdRng;

use crate::_0_profile::measure_layer;
use crate::color::{darken, lerp_color, lighten};
use crate::opts::param_f32;
use crate::registry::{AnimKind, Mode, ModeFrame, Param};
use crate::types::{Cell, Grid};

const TAU: f32 = std::f32::consts::TAU;

pub(super) struct SingularityMode;

pub(super) static MODE: SingularityMode = SingularityMode;

const PARAMS: &[Param] = &[
    param!("MASS", "lensing mass", 0.55, 1.45, 1.0, 0.05),
    param!("SPIN", "frame drag", -1.0, 1.0, 0.82, 0.05),
    param!("TILT", "disk inclination", 0.08, 0.55, 0.22, 0.02),
    param!("DISK", "plasma density", 0.0, 1.5, 1.0, 0.05),
    param!("JET", "polar jet", 0.0, 1.5, 0.68, 0.05),
    param!("STARS", "lensed stars", 0.0, 1.5, 0.78, 0.05),
    param!("SPEED", "orbital speed", 0.05, 3.0, 0.64, 0.05),
    param!("BLOOM", "photon bloom", 0.0, 1.5, 0.9, 0.05),
    param!("GRAIN", "plasma grain", 0.0, 1.5, 0.56, 0.05),
    param!("RINGS", "photon subrings", 1.0, 5.0, 3.0, 1.0),
    param!("ROLL", "disk roll", -0.35, 0.35, -0.06, 0.01),
];

impl Mode for SingularityMode {
    fn name(&self) -> &'static str {
        "singularity"
    }

    fn help(&self) -> &'static str {
        "Gravitationally lensed accretion disk, Kerr shadow, photon subrings, polar jet, and orbiting corona [mass] [spin] [tilt] [disk] [jet] [stars] [speed] [bloom] [grain] [rings] [roll]"
    }

    fn animation(&self) -> AnimKind {
        AnimKind::Iterate
    }

    fn params(&self) -> &'static [Param] {
        PARAMS
    }

    fn render(&self, frame: &mut ModeFrame<'_>) {
        let params = SingularityParams::from_inputs(frame.args, frame.param_values);
        draw_singularity(
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
pub(crate) struct SingularityParams {
    pub(crate) mass: f32,
    pub(crate) spin: f32,
    pub(crate) tilt: f32,
    pub(crate) disk: f32,
    pub(crate) jet: f32,
    pub(crate) stars: f32,
    pub(crate) speed: f32,
    pub(crate) bloom: f32,
    pub(crate) grain: f32,
    pub(crate) rings: usize,
    pub(crate) roll: f32,
}

impl Default for SingularityParams {
    fn default() -> Self {
        Self {
            mass: 1.0,
            spin: 0.82,
            tilt: 0.22,
            disk: 1.0,
            jet: 0.68,
            stars: 0.78,
            speed: 0.64,
            bloom: 0.9,
            grain: 0.56,
            rings: 3,
            roll: -0.06,
        }
    }
}

impl SingularityParams {
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
            mass: read(4, "MASS", 1.0).clamp(0.55, 1.45),
            spin: read(5, "SPIN", 0.82).clamp(-1.0, 1.0),
            tilt: read(6, "TILT", 0.22).clamp(0.08, 0.55),
            disk: read(7, "DISK", 1.0).clamp(0.0, 1.5),
            jet: read(8, "JET", 0.68).clamp(0.0, 1.5),
            stars: read(9, "STARS", 0.78).clamp(0.0, 1.5),
            speed: read(10, "SPEED", 0.64).clamp(0.05, 3.0),
            bloom: read(11, "BLOOM", 0.9).clamp(0.0, 1.5),
            grain: read(12, "GRAIN", 0.56).clamp(0.0, 1.5),
            rings: read(13, "RINGS", 3.0).round().clamp(1.0, 5.0) as usize,
            roll: read(14, "ROLL", -0.06).clamp(-0.35, 0.35),
        }
    }
}

#[derive(Clone, Copy)]
struct AxisSample {
    u: f32,
    v: f32,
}

struct Projection {
    x: Vec<AxisSample>,
    y: Vec<AxisSample>,
}

impl Projection {
    fn new(width: usize, height: usize, roll: f32) -> Self {
        let scale = 1.0 / height.max(1) as f32;
        let cx = width.saturating_sub(1) as f32 * 0.5;
        let cy = height.saturating_sub(1) as f32 * 0.5;
        let (sin_roll, cos_roll) = roll.sin_cos();
        let x = (0..width)
            .map(|column| {
                let px = (column as f32 - cx) * scale * 0.5;
                AxisSample {
                    u: px * cos_roll,
                    v: -px * sin_roll,
                }
            })
            .collect();
        let y = (0..height)
            .map(|row| {
                let py = (row as f32 - cy) * scale;
                AxisSample {
                    u: py * sin_roll,
                    v: py * cos_roll,
                }
            })
            .collect();
        Self { x, y }
    }

    #[inline]
    fn point(&self, x: usize, y: usize) -> (f32, f32) {
        (self.x[x].u + self.y[y].u, self.x[x].v + self.y[y].v)
    }
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
fn hash2(seed: u64, x: i32, y: i32) -> f32 {
    let value = seed
        ^ (x as u64).wrapping_mul(0xD6E8_FEB8_6659_FD93)
        ^ (y as u64).wrapping_mul(0xA5A3_56E4_9B2F_7A49);
    ((mix64(value) >> 40) as f32) * (1.0 / 16_777_215.0)
}

#[inline]
fn lens_source(u: f32, v: f32, einstein2: f32) -> (f32, f32) {
    let factor = 1.0 - einstein2 / (u * u + v * v).max(0.000_1);
    (u * factor, v * factor)
}

fn draw_lensed_background(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    params: &SingularityParams,
    projection: &Projection,
) {
    let einstein = 0.182 * params.mass.sqrt();
    let einstein2 = einstein * einstein;
    let source_scale = height.max(1) as f32;
    let base_bg = darken(palette[0], 18);
    let lens_bg = darken(palette[2], 72);
    let star_gate = 1.0 - params.stars * 0.0075;
    let dust_gate = 1.0 - params.stars * params.grain * 0.021;

    for y in 0..height.min(grid.len()) {
        let row_width = width.min(grid[y].len());
        for x in 0..row_width {
            let (u, v) = projection.point(x, y);
            let r2 = u * u + v * v;
            let r = r2.sqrt();
            let glow =
                ((einstein * 2.15 - r) / (einstein * 2.15)).clamp(0.0, 1.0) * params.bloom * 0.34;
            let bg = lerp_color(base_bg, lens_bg, glow);
            let (source_u, source_v) = lens_source(u, v, einstein2);
            let sx = (source_u * source_scale * 2.0).round() as i32;
            let sy = (source_v * source_scale).round() as i32;
            let star = hash2(seed ^ 0xA11C_E55E, sx, sy);

            if params.stars > 0.0 && star > star_gate {
                let phase = hash2(seed ^ 0x71A7_71A7, sx, sy) * TAU;
                let pulse = 0.5 + 0.5 * (phase + t * (0.7 + star)).sin();
                let ch = if pulse > 0.83 {
                    '✦'
                } else if pulse > 0.48 {
                    '•'
                } else {
                    '·'
                };
                let color = if pulse > 0.72 {
                    lighten(palette[4], 16)
                } else {
                    lerp_color(palette[2], palette[3], pulse)
                };
                grid[y][x] = Cell::with_bg(ch, color, bg);
            } else if params.grain > 0.0
                && star > dust_gate
                && hash2(seed ^ 0xD057_D057, x as i32, y as i32) > 0.72
            {
                grid[y][x] = Cell::with_bg('·', darken(palette[2], 68), bg);
            } else {
                grid[y][x] = Cell::with_bg(' ', base_bg, bg);
            }
        }
    }
}

fn disk_glyph(brightness: f32, grain: f32) -> char {
    match (brightness * 7.0 + grain * 0.75) as usize {
        0 => '·',
        1 => '.',
        2 => ':',
        3 => '-',
        4 => '=',
        5 => '≡',
        6 => '◆',
        _ => '✦',
    }
}

fn draw_relativity_field(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    params: &SingularityParams,
    projection: &Projection,
) {
    let mass_root = params.mass.sqrt();
    let horizon = 0.112 * params.mass;
    let horizon_u = horizon * (1.0 + params.spin.abs() * 0.11);
    let horizon_v = horizon * (1.0 - params.spin.abs() * 0.045);
    let horizon_shift = params.spin * horizon * 0.075;
    let einstein = 0.182 * mass_root;
    let einstein2 = einstein * einstein;
    let disk_inner = 0.105 * mass_root;
    let disk_outer = 0.535 * mass_root;
    let disk_span = (disk_outer - disk_inner).max(0.001);
    let orbit = t
        * params.speed
        * (0.85 + params.spin.abs() * 0.45)
        * if params.spin < 0.0 { -1.0 } else { 1.0 }
        + hash01(seed, 17) * TAU;
    let (orbit_sin, orbit_cos) = orbit.sin_cos();
    let flow_time = t * params.speed * 3.4;
    let jet_limit = 0.58 * mass_root;
    let shadow = Color::Rgb { r: 0, g: 0, b: 0 };

    for y in 0..height.min(grid.len()) {
        let row_width = width.min(grid[y].len());
        for x in 0..row_width {
            let (u, v) = projection.point(x, y);
            let r2 = u * u + v * v;
            let r = r2.sqrt().max(0.000_1);
            let sample = hash2(seed ^ 0xD15C_D15C, x as i32, y as i32);

            if params.jet > 0.0 && v.abs() > horizon_v * 1.05 && v.abs() < jet_limit {
                let reach = ((jet_limit - v.abs()) / jet_limit).clamp(0.0, 1.0);
                let jet_width = 0.006 + (v.abs() - horizon_v).max(0.0) * 0.052 * params.jet;
                let corkscrew = (v * 43.0 - flow_time * 1.7 + hash01(seed, 31) * TAU).sin()
                    * jet_width
                    * params.spin
                    * 0.7;
                let axis_distance = (u - corkscrew).abs();
                if axis_distance < jet_width && sample < 0.3 + reach * 0.48 {
                    let core = 1.0 - axis_distance / jet_width.max(0.000_1);
                    let flicker = 0.5 + 0.5 * (v.abs() * 91.0 + flow_time + sample * TAU).sin();
                    let brightness = (core * 0.62 + reach * 0.2 + flicker * 0.18).clamp(0.0, 1.0);
                    let ch = if brightness > 0.82 {
                        '✦'
                    } else if core > 0.66 {
                        '┃'
                    } else if flicker > 0.5 {
                        '┆'
                    } else {
                        '·'
                    };
                    let fg = lerp_color(darken(palette[2], 28), palette[3], brightness);
                    let bg = lerp_color(grid[y][x].bg, darken(palette[2], 58), brightness * 0.38);
                    grid[y][x] = Cell::with_bg(ch, fg, bg);
                }
            }

            if params.disk > 0.0 {
                let (source_u, source_v) = lens_source(u, v, einstein2);
                let disk_v = source_v / params.tilt;
                let disk_radius = (source_u * source_u + disk_v * disk_v).sqrt();
                if disk_radius >= disk_inner && disk_radius <= disk_outer {
                    let density_hash = hash2(seed ^ 0xACC3_710A, x as i32, y as i32);
                    let radial_heat = 1.0 - (disk_radius - disk_inner) / disk_span;
                    let density = (params.disk * (0.62 + radial_heat * 0.58)).clamp(0.0, 1.0);
                    if density_hash < density {
                        let inv_radius = 1.0 / disk_radius.max(0.000_1);
                        let azimuth_x = source_u * inv_radius;
                        let azimuth_y = disk_v * inv_radius;
                        let approaching = (0.5 + azimuth_x * params.spin * 0.42 + azimuth_y * 0.08)
                            .clamp(0.0, 1.0);
                        let hot_alignment =
                            (azimuth_x * orbit_cos + azimuth_y * orbit_sin).max(0.0);
                        let hot_spot = hot_alignment * hot_alignment;
                        let hot_spot = hot_spot * hot_spot;
                        let wave = 0.5
                            + 0.5
                                * (disk_radius * (74.0 + params.grain * 13.0) - flow_time
                                    + source_u * params.spin * 19.0)
                                    .sin();
                        let texture = (density_hash - 0.5) * params.grain * 0.34;
                        let brightness = (0.12
                            + radial_heat * 0.49
                            + approaching * 0.25
                            + hot_spot * 0.31
                            + wave * params.grain * 0.17
                            + texture)
                            .clamp(0.0, 1.0);
                        let ch = disk_glyph(brightness, density_hash);
                        let warm = lerp_color(palette[1], palette[3], brightness);
                        let fg = if brightness > 0.86 {
                            lighten(lerp_color(warm, palette[4], 0.7), 18)
                        } else if brightness < 0.3 {
                            darken(warm, 34)
                        } else {
                            warm
                        };
                        let bg = lerp_color(
                            grid[y][x].bg,
                            darken(palette[1], 62),
                            brightness * params.bloom * 0.42,
                        );
                        grid[y][x] = Cell::with_bg(ch, fg, bg);
                    }
                }
            }

            let side = (0.5 + (u / r) * params.spin * 0.46).clamp(0.0, 1.0);
            let mut ring_light = 0.0f32;
            for ring in 0..params.rings {
                let fraction = ring as f32 / params.rings.max(1) as f32;
                let target =
                    horizon * (1.32 + ring as f32 * 0.17) * (1.0 + (u / r) * params.spin * 0.055);
                let width = (0.0095 - fraction * 0.0035).max(0.0045) + params.bloom * 0.0015;
                let light =
                    (1.0 - (r - target).abs() / width).clamp(0.0, 1.0) * (1.0 - fraction * 0.18);
                ring_light = ring_light.max(light);
            }
            if ring_light > 0.0 {
                let brightness = (ring_light * (0.52 + side * 0.68) * params.bloom).clamp(0.0, 1.0);
                let ch = if brightness > 0.86 {
                    '✦'
                } else if brightness > 0.56 {
                    '●'
                } else if brightness > 0.28 {
                    '○'
                } else {
                    '·'
                };
                let fg = lerp_color(palette[1], lighten(palette[4], 24), brightness);
                let bg = lerp_color(grid[y][x].bg, darken(palette[3], 66), brightness * 0.5);
                grid[y][x] = Cell::with_bg(ch, fg, bg);
            }

            let shadow_u = (u + horizon_shift) / horizon_u;
            let shadow_v = v / horizon_v;
            let shadow_metric = shadow_u * shadow_u + shadow_v * shadow_v;
            if shadow_metric < 1.0 {
                grid[y][x] = Cell::with_bg(' ', shadow, shadow);
            }
        }
    }
}

fn put(grid: &mut Grid, x: i32, y: i32, ch: char, fg: Color, bg: Color) {
    if x < 0 || y < 0 {
        return;
    }
    if let Some(row) = grid.get_mut(y as usize)
        && let Some(cell) = row.get_mut(x as usize)
    {
        *cell = Cell::with_bg(ch, fg, bg);
    }
}

fn draw_orbiting_corona(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    params: &SingularityParams,
) {
    if params.bloom <= 0.0 {
        return;
    }
    let cx = width.saturating_sub(1) as f32 * 0.5;
    let cy = height.saturating_sub(1) as f32 * 0.5;
    let mass_root = params.mass.sqrt();
    let particles = ((width + height) as f32 * params.bloom * 0.38)
        .round()
        .clamp(0.0, 192.0) as usize;
    let direction = if params.spin < 0.0 { -1.0 } else { 1.0 };

    for particle in 0..particles {
        let identity = hash01(seed, 1000 + particle as u64);
        let orbit = hash01(seed, 1400 + particle as u64);
        let phase = identity * TAU + t * params.speed * direction * (0.22 + orbit * 1.4);
        let radius = (0.14 + orbit * 0.34) * mass_root;
        let precession = (phase * 0.37 + hash01(seed, 1800 + particle as u64) * TAU).sin();
        let x = cx + phase.cos() * radius * height as f32 * 2.0;
        let y = cy
            + phase.sin() * radius * height as f32 * (0.32 + params.tilt * 0.72)
            + precession * params.spin * 0.7;
        let pulse = 0.5 + 0.5 * (t * 1.9 + identity * TAU * 2.0).sin();
        if hash01(seed, 2200 + particle as u64) < 0.34 + pulse * 0.48 {
            let ch = if pulse > 0.86 {
                '✦'
            } else if particle % 5 == 0 {
                '•'
            } else {
                '·'
            };
            let fg = lerp_color(darken(palette[1], 22), lighten(palette[3], 18), pulse);
            let xi = x.round() as i32;
            let yi = y.round() as i32;
            let bg = grid
                .get(yi.max(0) as usize)
                .and_then(|row| row.get(xi.max(0) as usize))
                .map(|cell| cell.bg)
                .unwrap_or(palette[0]);
            put(grid, xi, yi, ch, fg, bg);
        }
    }
}

pub(crate) fn draw_singularity(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    _rng: &mut StdRng,
    t: f32,
    params: &SingularityParams,
) {
    if width == 0 || height == 0 || grid.is_empty() {
        return;
    }

    // 1. Precompute the rolled screen axes so both grid passes reuse additions only.
    let projection = Projection::new(width, height, params.roll);
    // 2. Inverse-lens a sparse deterministic star plane and lay down the radial glow.
    measure_layer("singularity", "lensed_background", || {
        draw_lensed_background(grid, width, height, seed, palette, t, params, &projection)
    });
    // 3. Classify jet, accretion plasma, photon subrings, and shadow in painter order.
    measure_layer("singularity", "relativity_field", || {
        draw_relativity_field(grid, width, height, seed, palette, t, params, &projection)
    });
    // 4. Reconstruct the bounded coronal orbiters directly from seed identity and time.
    measure_layer("singularity", "orbiting_corona", || {
        draw_orbiting_corona(grid, width, height, seed, palette, t, params)
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::make_palette;
    use crate::render::grid_to_plain;
    use rand::SeedableRng;

    fn frame(width: usize, height: usize, seed: u64, t: f32, params: &SingularityParams) -> Grid {
        let mut grid = vec![vec![Cell::blank(); width]; height];
        let palette = make_palette(seed);
        let mut rng = StdRng::seed_from_u64(seed);
        draw_singularity(
            &mut grid, width, height, seed, &palette, &mut rng, t, params,
        );
        grid
    }

    fn plain(grid: &Grid) -> String {
        grid_to_plain(grid).join("\n")
    }

    #[test]
    fn deterministic_seeded_frame_and_visible_motion() {
        let params = SingularityParams::default();
        let a = frame(100, 36, 42, 1.25, &params);
        let b = frame(100, 36, 42, 1.25, &params);
        let moved = frame(100, 36, 42, 3.75, &params);
        let reseeded = frame(100, 36, 43, 1.25, &params);
        assert_eq!(a, b);
        assert_ne!(a, moved);
        assert_ne!(a, reseeded);
    }

    #[test]
    fn parameter_values_override_and_clamp() {
        let values = [
            99.0, -99.0, 99.0, -1.0, 99.0, -1.0, 0.0, 99.0, 99.0, 99.0, -99.0,
        ];
        let params = SingularityParams::from_inputs(&[], Some(&values));
        insta::assert_debug_snapshot!(params, @r###"
        SingularityParams {
            mass: 1.45,
            spin: -1.0,
            tilt: 0.55,
            disk: 0.0,
            jet: 1.5,
            stars: 0.0,
            speed: 0.05,
            bloom: 1.5,
            grain: 1.5,
            rings: 5,
            roll: -0.35,
        }
        "###);
    }

    #[test]
    fn tiny_grids_and_parameter_extrema_terminate() {
        let maxed = SingularityParams {
            mass: 1.45,
            spin: -1.0,
            tilt: 0.55,
            disk: 1.5,
            jet: 1.5,
            stars: 1.5,
            speed: 3.0,
            bloom: 1.5,
            grain: 1.5,
            rings: 5,
            roll: 0.35,
        };
        for size in [(1usize, 1usize), (2, 1), (3, 5), (9, 2)] {
            let grid = frame(size.0, size.1, 7, 50_000.0, &maxed);
            assert_eq!(grid.len(), size.1);
            assert!(grid.iter().all(|row| row.len() == size.0));
        }
    }

    #[test]
    fn optional_layers_change_the_composition() {
        let params = SingularityParams::default();
        let full = frame(90, 32, 77, 2.4, &params);
        let quiet = frame(
            90,
            32,
            77,
            2.4,
            &SingularityParams {
                disk: 0.0,
                jet: 0.0,
                stars: 0.0,
                bloom: 0.0,
                ..params
            },
        );
        assert_ne!(full, quiet);
    }

    #[test]
    fn snapshot_singularity_t0() {
        insta::assert_snapshot!(plain(&frame(
            80,
            28,
            42,
            0.0,
            &SingularityParams::default(),
        )));
    }

    #[test]
    fn snapshot_singularity_in_motion() {
        insta::assert_snapshot!(plain(&frame(
            80,
            28,
            42,
            2.75,
            &SingularityParams::default(),
        )));
    }

    #[test]
    #[ignore = "release-only performance probe; run with --release --ignored"]
    fn perf_singularity_frame() {
        use std::hint::black_box;
        use std::time::Instant;

        let width = 320;
        let height = 100;
        let frames = 600;
        let seed = 42;
        let params = SingularityParams::default();
        let palette = make_palette(seed);
        let mut grid = vec![vec![Cell::blank(); width]; height];
        let started = Instant::now();
        for index in 0..frames {
            let mut rng = StdRng::seed_from_u64(seed);
            draw_singularity(
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
        eprintln!("singularity {width}x{height}: {average_ms:.3} ms/frame");
        assert!(
            average_ms < 16.0,
            "{average_ms:.3} ms exceeds 60 fps budget"
        );
    }
}
