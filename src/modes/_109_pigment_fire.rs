use crossterm::style::Color;
use rand::RngExt;
use rand::rngs::StdRng;

use crate::_0_profile::measure_layer;
use crate::color::{hsl_to_rgb, lerp_color};
use crate::opts::param_f32;
use crate::registry::{AnimKind, Mode, ModeFrame, Param};
use crate::types::{Cell, Grid};

pub(super) struct PigmentFire;

pub(super) static MODE: PigmentFire = PigmentFire;

const FAMILY_CHOICES: &[&str] = &["spectrum", "ember", "gas", "foxfire", "witchlight", "opal"];
const COLOR_BANDS: usize = 24;
const HEAT_STEPS: usize = 8;

const PARAMS: &[Param] = &[
    Param {
        key: "HEIGHT",
        label: "flame height",
        min: 0.45,
        max: 1.0,
        default: 0.88,
        step: 0.03,
        choices: &[],
        randomize: true,
    },
    Param {
        key: "WIDTH",
        label: "plume width",
        min: 0.18,
        max: 0.85,
        default: 0.62,
        step: 0.03,
        choices: &[],
        randomize: true,
    },
    Param {
        key: "TONGUES",
        label: "flame tongues",
        min: 3.0,
        max: 18.0,
        default: 10.0,
        step: 1.0,
        choices: &[],
        randomize: true,
    },
    Param {
        key: "WIND",
        label: "crosswind",
        min: -1.0,
        max: 1.0,
        default: 0.18,
        step: 0.05,
        choices: &[],
        randomize: true,
    },
    Param {
        key: "SWAY",
        label: "sway",
        min: 0.0,
        max: 1.8,
        default: 0.82,
        step: 0.05,
        choices: &[],
        randomize: true,
    },
    Param {
        key: "SPEED",
        label: "flow speed",
        min: 0.1,
        max: 3.0,
        default: 1.0,
        step: 0.1,
        choices: &[],
        randomize: true,
    },
    Param {
        key: "WASH",
        label: "pigment load",
        min: 0.1,
        max: 1.0,
        default: 0.72,
        step: 0.05,
        choices: &[],
        randomize: true,
    },
    Param {
        key: "BLEED",
        label: "wet edge",
        min: 0.05,
        max: 1.0,
        default: 0.52,
        step: 0.05,
        choices: &[],
        randomize: true,
    },
    Param {
        key: "HEAT",
        label: "core heat",
        min: 0.2,
        max: 1.0,
        default: 0.78,
        step: 0.05,
        choices: &[],
        randomize: true,
    },
    Param {
        key: "FAMILY",
        label: "fire family",
        min: 0.0,
        max: 5.0,
        default: 0.0,
        step: 1.0,
        choices: FAMILY_CHOICES,
        randomize: true,
    },
    Param {
        key: "HUE",
        label: "hue shift",
        min: -180.0,
        max: 180.0,
        default: 0.0,
        step: 5.0,
        choices: &[],
        randomize: true,
    },
    Param {
        key: "GRAIN",
        label: "paper grain",
        min: 0.0,
        max: 1.0,
        default: 0.46,
        step: 0.05,
        choices: &[],
        randomize: true,
    },
    Param {
        key: "SPARKS",
        label: "flying sparks",
        min: 0.0,
        max: 1.0,
        default: 0.45,
        step: 0.05,
        choices: &[],
        randomize: true,
    },
    Param {
        key: "AURA",
        label: "outer glow",
        min: 0.0,
        max: 1.0,
        default: 0.78,
        step: 0.05,
        choices: &[],
        randomize: true,
    },
    Param {
        key: "COALS",
        label: "ember bed",
        min: 0.0,
        max: 1.0,
        default: 0.82,
        step: 0.05,
        choices: &[],
        randomize: true,
    },
    Param {
        key: "FLICKER",
        label: "flicker",
        min: 0.1,
        max: 1.5,
        default: 0.72,
        step: 0.05,
        choices: &[],
        randomize: true,
    },
];

#[derive(Clone, Copy)]
struct FireControls {
    height: f32,
    width: f32,
    tongues: f32,
    wind: f32,
    sway: f32,
    speed: f32,
    wash: f32,
    bleed: f32,
    heat: f32,
    family: f32,
    hue: f32,
    grain: f32,
    sparks: f32,
    aura: f32,
    coals: f32,
    flicker: f32,
}

impl Mode for PigmentFire {
    fn name(&self) -> &'static str {
        "pigment-fire"
    }

    fn help(&self) -> &'static str {
        "A continuously flowing watercolor flame with spectral fire families, wet pigment blooms, coals, and sparks."
    }

    fn animation(&self) -> AnimKind {
        AnimKind::Iterate
    }

    fn params(&self) -> &'static [Param] {
        PARAMS
    }

    fn render(&self, frame: &mut ModeFrame<'_>) {
        let value = |index: usize| {
            let parameter = PARAMS[index];
            frame
                .param_values
                .and_then(|values| values.get(index).copied())
                .unwrap_or_else(|| param_f32(parameter.key, parameter.default))
                .clamp(parameter.min, parameter.max)
        };

        let controls = FireControls {
            height: value(0),
            width: value(1),
            tongues: value(2),
            wind: value(3),
            sway: value(4),
            speed: value(5),
            wash: value(6),
            bleed: value(7),
            heat: value(8),
            family: value(9),
            hue: value(10),
            grain: value(11),
            sparks: value(12),
            aura: value(13),
            coals: value(14),
            flicker: value(15),
        };

        measure_layer(self.name(), "render", || {
            draw_pigment_fire(
                frame.grid,
                frame.width,
                frame.height,
                frame.seed,
                frame.palette,
                frame.rng,
                frame.time,
                controls,
            );
        });
    }
}

// fn draw_pigment_fire(grid, width, height, seed, palette, rng, t, controls)
// Initialize seed-local phase and a deep paper ground.
// Evaluate the flowing plume, wet edge, layered pigment, and grain per cell.
// Stamp the ember bed and a grid-bounded set of rising sparks.
fn draw_pigment_fire(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    rng: &mut StdRng,
    t: f32,
    controls: FireControls,
) {
    if width == 0 || height == 0 || grid.is_empty() {
        return;
    }

    let grid_height = height.min(grid.len());
    let grid_width = grid
        .iter()
        .take(grid_height)
        .map(|row| row.len().min(width))
        .min()
        .unwrap_or(0);
    if grid_width == 0 || grid_height == 0 {
        return;
    }

    let time = if t.is_finite() { t } else { 0.0 };
    let phase = time * controls.speed * controls.flicker;
    let seed_phase = rng.random::<f32>() * std::f32::consts::TAU;
    let paper = lerp_color(Color::Rgb { r: 2, g: 3, b: 12 }, palette[0], 0.22);
    let pigments = pigment_table(controls.family, controls.hue);
    let area = grid_width.saturating_mul(grid_height);

    for y in 0..grid_height {
        let rise = (grid_height.saturating_sub(1 + y) as f32 + 0.5) / grid_height as f32;
        let v = rise / controls.height;
        let clamped_v = v.clamp(0.0, 1.0);
        let taper = (1.0 - clamped_v).sqrt();
        let drift = (v * 7.4 - phase * 1.25 + seed_phase).sin() * controls.sway * 0.105
            + (v * 3.6 + phase * 0.42).sin() * controls.wind * 0.18;
        let envelope =
            controls.width * (0.16 + 0.47 * taper) * (1.0 + (v * 16.0 - phase * 1.7).sin() * 0.055);
        let height_gate = 1.0 - smoothstep(0.78, 1.0, v);

        for x in 0..grid_width {
            let nx = ((x as f32 + 0.5) / grid_width as f32) * 2.0 - 1.0;
            let tendril_wave =
                (nx * controls.tongues * 1.58 + v * 12.5 - phase * 1.65 + seed_phase).sin() * 0.055
                    + (nx * controls.tongues * 0.71 - v * 18.0 + phase * 0.88).sin() * 0.032;
            let flow_ripple = (nx * 25.0 + v * 31.0 - phase * 1.1 + seed_phase).sin() * 0.025
                + (nx * 63.0 - v * 46.0 + phase * 0.53).sin() * 0.012;
            let grain = hash_noise(seed, x, y);
            let cloud = (nx * 10.0 + v * 8.0 - phase * 0.22 + seed_phase * 0.4).sin()
                * (nx * 17.0 - v * 13.0 + phase * 0.17).cos();
            let wash_streak = (nx * 8.4 + v * 13.0 - phase * 0.58 + seed_phase * 1.4).sin()
                * (nx * 5.0 - v * 7.0 + phase * 0.22).cos();
            let boundary = envelope + tendril_wave + flow_ripple + cloud * controls.bleed * 0.022;
            let shape = boundary - (nx - drift).abs();
            let edge_softness = 0.012 + controls.bleed * 0.038;
            let body = smoothstep(-edge_softness, edge_softness, shape) * height_gate;
            let core = smoothstep(envelope * 0.12, envelope * 0.78, shape) * body;
            let halo_span = 0.025 + controls.bleed * 0.135;
            let halo = smoothstep(-halo_span, edge_softness, shape) * (1.0 - body) * controls.aura;
            let mottling =
                (0.91 + (grain - 0.5) * controls.grain * 0.52 + cloud * controls.grain * 0.09)
                    .clamp(0.56, 1.08);
            let pigment_density =
                0.49 + (wash_streak * 0.5 + 0.5) * 0.39 + grain * controls.grain * 0.12;
            let coverage = body * mottling * pigment_density;
            let thermal =
                (core * controls.heat + (1.0 - clamped_v) * 0.12 + grain * 0.09).clamp(0.0, 1.0);
            let band =
                (((nx + 1.0) * 0.5 + phase * 0.012).rem_euclid(1.0) * COLOR_BANDS as f32) as usize;
            let heat = (thermal * (HEAT_STEPS - 1) as f32).round() as usize;
            let pigment = pigments[band * HEAT_STEPS + heat];
            let highlight = quantize_unit(core * controls.heat * 0.36, 6).min(0.48);
            let luminous = lerp_color(
                pigment,
                Color::Rgb {
                    r: 255,
                    g: 246,
                    b: 213,
                },
                highlight,
            );
            let wash_depth = quantize_unit(
                (coverage * (0.17 + controls.wash * 0.42) + halo * 0.29).clamp(0.0, 0.82),
                8,
            );
            let background = lerp_color(paper, pigment, wash_depth);

            let ch = if coverage < 0.12 {
                if halo > 0.34 && grain > 0.78 {
                    '·'
                } else if body > 0.52 && wash_streak < -0.76 && grain < controls.grain * 0.82 {
                    ' '
                } else {
                    ' '
                }
            } else if grain < controls.grain * 0.13 && coverage < 0.9 {
                '·'
            } else if body > 0.52 && wash_streak < -0.76 && grain < controls.grain * 0.82 {
                ' '
            } else if core > 0.72 && grain > 0.84 {
                '*'
            } else if coverage < 0.25 {
                '·'
            } else if coverage < 0.43 {
                '\''
            } else if coverage < 0.61 {
                '░'
            } else if coverage < 0.79 {
                '▒'
            } else if core > 0.76 && grain > 0.92 {
                '✦'
            } else {
                '▓'
            };

            let mut cell = Cell::with_bg(ch, luminous, background);
            let ember_zone = rise < 0.055 + controls.coals * 0.04;
            if ember_zone && nx.abs() < controls.width * 0.96 && grain < controls.coals * 0.47 {
                let coal = Color::Rgb {
                    r: (118.0 + thermal * 120.0) as u8,
                    g: (20.0 + thermal * 88.0) as u8,
                    b: (18.0 + thermal * 24.0) as u8,
                };
                cell = Cell::with_bg(if grain > 0.30 { '*' } else { '_' }, coal, background);
            }
            grid[y][x] = cell;
        }
    }

    let spark_count = ((area as f32 * controls.sparks * 0.012).round() as usize).min(4096);
    for _ in 0..spark_count {
        let seed_x = rng.random::<f32>();
        let seed_y = rng.random::<f32>();
        let seed_glyph = rng.random::<f32>();
        let age = (phase * 0.21 + seed_y).rem_euclid(1.0);
        let spark_y = (grid_height as f32 - 1.0 - age * grid_height as f32 * controls.height * 0.78)
            .round() as isize;
        let spark_x = (grid_width as f32 * 0.5
            + (seed_x - 0.5) * grid_width as f32 * controls.width * 1.8
            + controls.wind * age * grid_width as f32 * 0.12
            + (phase + seed_y * 9.0).sin() * controls.sway * grid_width as f32 * 0.035)
            .round() as isize;
        if spark_x < 0 || spark_y < 0 {
            continue;
        }
        let (spark_x, spark_y) = (spark_x as usize, spark_y as usize);
        if spark_x >= grid_width || spark_y >= grid_height {
            continue;
        }

        let tint_band = (seed_x * COLOR_BANDS as f32) as usize;
        let tint_heat = ((0.62 + seed_glyph * 0.36) * (HEAT_STEPS - 1) as f32).round() as usize;
        let tint = pigments[tint_band * HEAT_STEPS + tint_heat];
        let fg = lerp_color(
            tint,
            Color::Rgb {
                r: 255,
                g: 247,
                b: 220,
            },
            0.38,
        );
        let ch = if seed_glyph > 0.78 {
            '*'
        } else if seed_glyph > 0.38 {
            '+'
        } else {
            '·'
        };
        let bg = grid[spark_y][spark_x].bg;
        grid[spark_y][spark_x] = Cell::with_bg(ch, fg, bg);
    }
}

fn pigment_table(family: f32, hue_shift: f32) -> [Color; COLOR_BANDS * HEAT_STEPS] {
    let mut table = [Color::Rgb { r: 0, g: 0, b: 0 }; COLOR_BANDS * HEAT_STEPS];
    for band in 0..COLOR_BANDS {
        let band_value = band as f32 / COLOR_BANDS as f32;
        for heat in 0..HEAT_STEPS {
            let thermal = heat as f32 / (HEAT_STEPS - 1) as f32;
            table[band * HEAT_STEPS + heat] = fire_pigment(family, band_value, thermal, hue_shift);
        }
    }
    table
}

fn fire_pigment(family: f32, band: f32, thermal: f32, hue_shift: f32) -> Color {
    let family = family.round() as i32;
    let band = band.clamp(0.0, 1.0);
    let thermal = thermal.clamp(0.0, 1.0);
    let (hue, saturation) = match family {
        1 => (8.0 + thermal * 48.0 + band * 7.0, 0.98),
        2 => (188.0 + band * 92.0 - thermal * 14.0, 0.94),
        3 => (101.0 + band * 76.0 - thermal * 8.0, 0.88),
        4 => (265.0 + band * 75.0 + thermal * 8.0, 0.92),
        5 => (350.0 + band * 360.0 + thermal * 12.0, 0.66),
        _ => (350.0 + band * 370.0 + thermal * 12.0, 0.95),
    };
    let lightness = (0.20 + thermal * 0.48).clamp(0.18, 0.74);
    hsl_to_rgb(
        (hue + hue_shift).rem_euclid(360.0) as f64,
        saturation as f64,
        lightness as f64,
    )
}

fn quantize_unit(value: f32, steps: usize) -> f32 {
    let steps = steps as f32;
    (value.clamp(0.0, 1.0) * steps).round() / steps
}

fn smoothstep(edge0: f32, edge1: f32, value: f32) -> f32 {
    let t = ((value - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn hash_noise(seed: u64, x: usize, y: usize) -> f32 {
    let mut value = seed
        ^ (x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (y as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value ^= value >> 30;
    value = value.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^= value >> 31;
    ((value >> 40) as f32) / ((1_u32 << 24) as f32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    fn default_values() -> Vec<f32> {
        PARAMS.iter().map(|parameter| parameter.default).collect()
    }

    fn render_plain(width: usize, height: usize, seed: u64, time: f32, values: &[f32]) -> String {
        let mut grid = vec![vec![Cell::blank(); width]; height];
        let palette = crate::color::make_palette(seed);
        let mut rng = StdRng::seed_from_u64(seed);
        {
            let mut frame = ModeFrame {
                grid: &mut grid,
                width,
                height,
                seed,
                palette: &palette,
                rng: &mut rng,
                time,
                args: &[],
                param_values: Some(values),
            };
            MODE.render(&mut frame);
        }
        grid.iter()
            .map(|row| row.iter().map(|cell| cell.ch).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn pigment_fire_80x30_seed42() {
        insta::assert_snapshot!(
            "pigment_fire_80x30_seed42",
            render_plain(80, 30, 42, 0.0, &default_values())
        );
    }

    #[test]
    fn pigment_fire_80x30_seed42_t5() {
        insta::assert_snapshot!(
            "pigment_fire_80x30_seed42_t5",
            render_plain(80, 30, 42, 5.0, &default_values())
        );
    }

    #[test]
    fn same_inputs_repeat_and_time_changes_the_frame() {
        let values = default_values();
        let first = render_plain(52, 18, 1907, 0.0, &values);
        let repeated = render_plain(52, 18, 1907, 0.0, &values);
        let later = render_plain(52, 18, 1907, 2.75, &values);
        assert_eq!(first, repeated);
        assert_ne!(first, later);
    }

    #[test]
    fn tiny_grid_holds_bounds_at_parameter_extrema() {
        for values in [
            PARAMS
                .iter()
                .map(|parameter| parameter.min)
                .collect::<Vec<_>>(),
            PARAMS
                .iter()
                .map(|parameter| parameter.max)
                .collect::<Vec<_>>(),
        ] {
            let output = render_plain(7, 5, 91, 3.0, &values);
            let rows = output.lines().collect::<Vec<_>>();
            assert_eq!(rows.len(), 5);
            assert!(rows.iter().all(|row| row.chars().count() == 7));
        }
    }
}
