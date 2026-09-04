use crossterm::style::Color;
use rand::rngs::StdRng;

use crate::_0_profile::measure_layer;
use crate::color::{darken, lerp_color, lighten};
use crate::opts::param_f32;
use crate::registry::{AnimKind, Mode, ModeFrame, Param};
use crate::types::{Cell, Grid};

const TAU: f32 = std::f32::consts::TAU;

pub(super) struct ThunderheadMode;

pub(super) static MODE: ThunderheadMode = ThunderheadMode;

const PARAMS: &[Param] = &[
    param!("SCALE", "storm span", 0.55, 1.35, 1.0, 0.05),
    param!("TURB", "cloud turbulence", 0.0, 1.5, 0.9, 0.05),
    param!("SHEAR", "mesocyclone shear", -1.0, 1.0, 0.55, 0.05),
    param!("RAIN", "rain curtain", 0.0, 1.5, 0.78, 0.05),
    param!("BOLTS", "ground channels", 1.0, 5.0, 2.0, 1.0),
    param!("FORKS", "channel forks", 0.0, 6.0, 3.0, 1.0),
    param!("FLASH", "lightning flash", 0.0, 1.5, 1.0, 0.05),
    param!("WIND", "surface wind", -1.5, 1.5, 0.35, 0.05),
    param!("SPEED", "storm clock", 0.05, 3.0, 0.65, 0.05),
    param!("GLOW", "electric bloom", 0.0, 1.5, 0.9, 0.05),
    param!("REFLECT", "ground reflection", 0.0, 1.5, 0.82, 0.05),
];

impl Mode for ThunderheadMode {
    fn name(&self) -> &'static str {
        "thunderhead"
    }

    fn help(&self) -> &'static str {
        "Rotating supercell, sculpted anvil, deterministic forked lightning, rain curtains, and storm-ground reflection [scale] [turb] [shear] [rain] [bolts] [forks] [flash] [wind] [speed] [glow] [reflect]"
    }

    fn animation(&self) -> AnimKind {
        AnimKind::Iterate
    }

    fn params(&self) -> &'static [Param] {
        PARAMS
    }

    fn render(&self, frame: &mut ModeFrame<'_>) {
        let params = ThunderheadParams::from_inputs(frame.args, frame.param_values);
        draw_thunderhead(
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
pub(crate) struct ThunderheadParams {
    pub(crate) scale: f32,
    pub(crate) turbulence: f32,
    pub(crate) shear: f32,
    pub(crate) rain: f32,
    pub(crate) bolts: usize,
    pub(crate) forks: usize,
    pub(crate) flash: f32,
    pub(crate) wind: f32,
    pub(crate) speed: f32,
    pub(crate) glow: f32,
    pub(crate) reflect: f32,
}

impl Default for ThunderheadParams {
    fn default() -> Self {
        Self {
            scale: 1.0,
            turbulence: 0.9,
            shear: 0.55,
            rain: 0.78,
            bolts: 2,
            forks: 3,
            flash: 1.0,
            wind: 0.35,
            speed: 0.65,
            glow: 0.9,
            reflect: 0.82,
        }
    }
}

impl ThunderheadParams {
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
            scale: read(4, "SCALE", 1.0).clamp(0.55, 1.35),
            turbulence: read(5, "TURB", 0.9).clamp(0.0, 1.5),
            shear: read(6, "SHEAR", 0.55).clamp(-1.0, 1.0),
            rain: read(7, "RAIN", 0.78).clamp(0.0, 1.5),
            bolts: read(8, "BOLTS", 2.0).round().clamp(1.0, 5.0) as usize,
            forks: read(9, "FORKS", 3.0).round().clamp(0.0, 6.0) as usize,
            flash: read(10, "FLASH", 1.0).clamp(0.0, 1.5),
            wind: read(11, "WIND", 0.35).clamp(-1.5, 1.5),
            speed: read(12, "SPEED", 0.65).clamp(0.05, 3.0),
            glow: read(13, "GLOW", 0.9).clamp(0.0, 1.5),
            reflect: read(14, "REFLECT", 0.82).clamp(0.0, 1.5),
        }
    }
}

#[derive(Clone, Copy)]
struct ColumnPhase {
    nx: f32,
    a_sin: f32,
    a_cos: f32,
    b_sin: f32,
    b_cos: f32,
    water_sin: f32,
    water_cos: f32,
}

#[derive(Clone, Copy)]
struct RowPhase {
    ny: f32,
    a_sin: f32,
    a_cos: f32,
    b_sin: f32,
    b_cos: f32,
    water_sin: f32,
    water_cos: f32,
}

struct StormField {
    columns: Vec<ColumnPhase>,
    rows: Vec<RowPhase>,
    ground_y: usize,
}

impl StormField {
    fn new(width: usize, height: usize, t: f32, speed: f32) -> Self {
        let inv_width = 1.0 / width.max(1) as f32;
        let inv_height = 1.0 / height.max(1) as f32;
        let columns = (0..width)
            .map(|x| {
                let nx = (x as f32 + 0.5) * inv_width * 2.0 - 1.0;
                let (a_sin, a_cos) = (nx * 15.7 + t * speed * 0.31).sin_cos();
                let (b_sin, b_cos) = (nx * 7.3 - t * speed * 0.47).sin_cos();
                let (water_sin, water_cos) = (nx * 18.0 + t * speed * 1.4).sin_cos();
                ColumnPhase {
                    nx,
                    a_sin,
                    a_cos,
                    b_sin,
                    b_cos,
                    water_sin,
                    water_cos,
                }
            })
            .collect();
        let rows = (0..height)
            .map(|y| {
                let ny = (y as f32 + 0.5) * inv_height;
                let (a_sin, a_cos) = (ny * 19.3).sin_cos();
                let (b_sin, b_cos) = (-ny * 28.1).sin_cos();
                let (water_sin, water_cos) = (ny * 11.0).sin_cos();
                RowPhase {
                    ny,
                    a_sin,
                    a_cos,
                    b_sin,
                    b_cos,
                    water_sin,
                    water_cos,
                }
            })
            .collect();
        Self {
            columns,
            rows,
            ground_y: ((height as f32 * 0.79).round() as usize).min(height.saturating_sub(1)),
        }
    }

    #[inline]
    fn waves(&self, x: usize, y: usize) -> (f32, f32, f32) {
        let column = self.columns[x];
        let row = self.rows[y];
        (
            column.a_sin * row.a_cos + column.a_cos * row.a_sin,
            column.b_sin * row.b_cos + column.b_cos * row.b_sin,
            column.water_sin * row.water_cos + column.water_cos * row.water_sin,
        )
    }
}

#[derive(Clone, Copy)]
struct StrikeState {
    id: i64,
    intensity: f32,
}

fn strike_state(t: f32, params: &ThunderheadParams) -> StrikeState {
    let period = 3.4;
    let clock = t * params.speed;
    let id = (clock / period).floor() as i64;
    let phase = clock.rem_euclid(period) / period;
    let primary = if phase < 0.075 {
        1.0 - phase / 0.075
    } else {
        0.0
    };
    let echo = if (0.115..0.16).contains(&phase) {
        0.56 * (1.0 - (phase - 0.115) / 0.045)
    } else {
        0.0
    };
    StrikeState {
        id,
        intensity: primary.max(echo) * params.flash,
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
fn ellipse_field(x: f32, y: f32, cx: f32, cy: f32, rx: f32, ry: f32) -> f32 {
    let dx = (x - cx) / rx.max(0.001);
    let dy = (y - cy) / ry.max(0.001);
    1.0 - dx * dx - dy * dy
}

fn draw_storm_sky(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    strike: StrikeState,
    field: &StormField,
) {
    let sky_top = darken(palette[0], 28);
    let sky_horizon = darken(palette[2], 52);
    let ground_bg = darken(palette[0], 36);
    let ground_fg = darken(palette[2], 28);
    let flash = strike.intensity.clamp(0.0, 1.0);

    for y in 0..height.min(grid.len()) {
        let ny = field.rows[y].ny;
        let row_width = width.min(grid[y].len());
        let sky_mix = (ny / 0.79).clamp(0.0, 1.0);
        let sky = lerp_color(sky_top, sky_horizon, sky_mix * 0.72);
        let lit_sky = lerp_color(sky, darken(palette[4], 72), flash * (0.28 + ny * 0.3));
        for x in 0..row_width {
            if y < field.ground_y {
                let grain = hash2(seed ^ 0x5A71_5A71, x as i32, y as i32);
                let ch = if grain > 0.998 { '·' } else { ' ' };
                grid[y][x] = Cell::with_bg(ch, darken(palette[3], 58), lit_sky);
            } else {
                let (_, _, water) = field.waves(x, y);
                let ripple = water + hash2(seed ^ 0xA73E_A73E, x as i32, y as i32) * 0.42;
                let ch = if y == field.ground_y {
                    if ripple > 0.15 { '═' } else { '─' }
                } else if ripple > 0.82 {
                    '〜'
                } else if ripple > 0.28 {
                    '~'
                } else if ripple < -0.72 {
                    '_'
                } else {
                    ' '
                };
                let bg = lerp_color(ground_bg, darken(palette[2], 62), (water + 1.0) * 0.08);
                let fg = lerp_color(ground_fg, palette[3], flash * 0.38);
                grid[y][x] = Cell::with_bg(ch, fg, bg);
            }
        }
    }
}

fn put_preserving_bg(grid: &mut Grid, x: i32, y: i32, ch: char, fg: Color) {
    if x < 0 || y < 0 {
        return;
    }
    if let Some(row) = grid.get_mut(y as usize)
        && let Some(cell) = row.get_mut(x as usize)
    {
        let bg = cell.bg;
        *cell = Cell::with_bg(ch, fg, bg);
    }
}

fn draw_rain(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    params: &ThunderheadParams,
    field: &StormField,
) {
    if params.rain <= 0.0 || field.ground_y == 0 {
        return;
    }
    let area_count = (width.saturating_mul(height) as f32 * params.rain * 0.019) as usize;
    let rain_count = area_count.min((width + height).saturating_mul(6));
    let rain_color = darken(palette[2], 25);
    let rain_head = lerp_color(palette[2], palette[3], 0.46);
    let slant = if params.wind > 0.22 {
        '╲'
    } else if params.wind < -0.22 {
        '╱'
    } else {
        '│'
    };

    for drop in 0..rain_count {
        let identity = hash01(seed ^ 0xA11D_0001, drop as u64);
        let depth = hash01(seed ^ 0xA11D_0002, drop as u64);
        let fall = t * params.speed * (7.0 + depth * 17.0);
        let y = (identity * field.ground_y as f32 + fall).rem_euclid(field.ground_y as f32);
        let base_x = hash01(seed ^ 0xA11D_0003, drop as u64) * width.max(1) as f32;
        let x =
            (base_x + fall * params.wind * (0.22 + depth * 0.18)).rem_euclid(width.max(1) as f32);
        let xi = x.round() as i32;
        let yi = y.round() as i32;
        put_preserving_bg(
            grid,
            xi,
            yi,
            if depth > 0.78 { slant } else { '·' },
            if depth > 0.78 { rain_head } else { rain_color },
        );
        if depth > 0.9 && yi + 1 < field.ground_y as i32 {
            put_preserving_bg(grid, xi, yi + 1, slant, rain_color);
        }
    }
}

fn cloud_glyph(level: usize, shelf: bool) -> char {
    if shelf && level > 1 {
        return match level {
            2 => '≋',
            3 => '≈',
            _ => '≣',
        };
    }
    match level {
        0 => '·',
        1 => '░',
        2 => '▒',
        3 => '▓',
        _ => '█',
    }
}

fn draw_supercell(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    params: &ThunderheadParams,
    strike: StrikeState,
    field: &StormField,
) {
    let center = (hash01(seed, 41) - 0.5) * 0.13;
    let scale = params.scale;
    let flash = strike.intensity.clamp(0.0, 1.0);
    let fg_dark = darken(palette[2], 70);
    let fg_mid = darken(palette[2], 25);
    let fg_light = lerp_color(palette[2], palette[4], 0.46 + flash * 0.34);
    let fg_ramp = [
        fg_dark,
        darken(fg_mid, 18),
        fg_mid,
        lerp_color(fg_mid, fg_light, 0.48),
        fg_light,
    ];
    let bg_dark = darken(palette[0], 35);
    let bg_mid = darken(palette[2], 64);
    let bg_light = lerp_color(darken(palette[2], 42), palette[4], flash * 0.26);
    let phase = t * params.speed * (1.1 + params.shear.abs() * 0.32);

    for y in 0..field.ground_y.min(grid.len()) {
        let ny = field.rows[y].ny;
        if !(0.055..=0.735).contains(&ny) {
            continue;
        }
        let row_width = width.min(grid[y].len());
        for x in 0..row_width {
            let nx = field.columns[x].nx;
            let shear_shift = params.shear * (ny - 0.39) * 0.24;
            let anvil = ellipse_field(
                nx,
                ny,
                center + shear_shift * 0.22,
                0.225,
                0.73 * scale,
                0.135 * scale,
            );
            let tower = ellipse_field(
                nx,
                ny,
                center - shear_shift * 0.45,
                0.385,
                0.31 * scale,
                0.275 * scale,
            );
            let overshoot = ellipse_field(
                nx,
                ny,
                center - params.shear * 0.075,
                0.155,
                0.21 * scale,
                0.115 * scale,
            );
            let base = ellipse_field(
                nx,
                ny,
                center + shear_shift * 0.74,
                0.59,
                0.54 * scale,
                0.13 * scale,
            );
            let shape = anvil.max(tower).max(overshoot).max(base);
            let (wave_a, wave_b, _) = field.waves(x, y);
            let grain = hash2(seed ^ 0xC10D_C10D, x as i32, y as i32);
            let edge_noise =
                params.turbulence * (wave_a * 0.105 + wave_b * 0.072 + (grain - 0.5) * 0.19);
            let density = shape + edge_noise;
            if density <= -0.015 {
                continue;
            }

            let updraft = ((nx - center - shear_shift) * 7.0 - phase).sin();
            let carved = (density * 0.82
                + wave_a * params.turbulence * 0.14
                + updraft * params.shear.abs() * 0.08
                + flash * 0.18)
                .clamp(0.0, 1.0);
            let level = (carved * 4.85).floor().clamp(0.0, 4.0) as usize;
            let shelf_phase = (density * 6.0 + wave_b * 0.45 + phase * 0.42).fract().abs();
            let shelf = ny > 0.49 && shelf_phase < 0.18;
            let ch = cloud_glyph(level, shelf);
            let rim = (1.0 - density.abs() * 3.4).clamp(0.0, 1.0);
            let foreground = lerp_color(fg_ramp[level], fg_light, rim * flash * 0.62);
            let background = lerp_color(bg_dark, bg_mid, carved * 0.72);
            let background = lerp_color(background, bg_light, flash * rim * params.glow * 0.42);
            grid[y][x] = Cell::with_bg(ch, foreground, background);
        }
    }
}

fn lightning_glyph(dx: i32) -> char {
    match dx.cmp(&0) {
        std::cmp::Ordering::Less => '╱',
        std::cmp::Ordering::Equal => '│',
        std::cmp::Ordering::Greater => '╲',
    }
}

fn put_lightning(
    grid: &mut Grid,
    x: i32,
    y: i32,
    ch: char,
    color: Color,
    glow_color: Color,
    glow: f32,
) {
    if x < 0 || y < 0 {
        return;
    }
    for neighbor_x in (x - 1)..=(x + 1) {
        if neighbor_x < 0 {
            continue;
        }
        if let Some(cell) = grid
            .get_mut(y as usize)
            .and_then(|row| row.get_mut(neighbor_x as usize))
        {
            cell.bg = lerp_color(cell.bg, glow_color, glow.clamp(0.0, 1.0) * 0.46);
            if neighbor_x != x && cell.ch == ' ' && glow > 0.45 {
                cell.ch = '·';
                cell.fg = darken(glow_color, 38);
            }
        }
    }
    put_preserving_bg(grid, x, y, ch, color);
}

fn draw_lightning(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    params: &ThunderheadParams,
    strike: StrikeState,
    field: &StormField,
) -> Vec<(i32, i32)> {
    if strike.intensity <= 0.0 || width == 0 || height == 0 {
        return Vec::new();
    }
    let strike_seed = seed ^ (strike.id as u64).wrapping_mul(0xD1B5_4A32_D192_ED03);
    let channel_color = lighten(
        lerp_color(palette[3], palette[4], strike.intensity.clamp(0.0, 1.0)),
        24,
    );
    let glow_color = lighten(palette[3], 12);
    let mut all_points = Vec::with_capacity(params.bolts * height * (params.forks + 1));
    let min_x = if width > 2 { 1 } else { 0 };
    let max_x = width.saturating_sub(1).saturating_sub(min_x) as i32;

    for bolt in 0..params.bolts {
        let start_y = ((height as f32 * (0.31 + hash01(strike_seed, 100 + bolt as u64) * 0.16))
            .round() as usize)
            .min(field.ground_y);
        let spread = (hash01(strike_seed, 200 + bolt as u64) - 0.5) * width as f32 * 0.42;
        let mut x = (width as f32 * 0.5 + spread).round() as i32;
        x = x.clamp(min_x as i32, max_x.max(min_x as i32));
        let mut main = Vec::with_capacity(field.ground_y.saturating_sub(start_y) + 1);

        for y in start_y..=field.ground_y {
            let previous_x = x;
            let jitter = hash2(strike_seed ^ (bolt as u64 * 701), x, y as i32);
            let jitter_step = if jitter < 0.27 {
                -1
            } else if jitter > 0.73 {
                1
            } else {
                0
            };
            let wind_step = if (y + bolt) % 5 == 0 {
                params.wind.signum() as i32
            } else {
                0
            };
            x = (x + jitter_step + wind_step).clamp(min_x as i32, max_x.max(min_x as i32));
            let point = (x, y as i32);
            main.push(point);
            all_points.push(point);
            put_lightning(
                grid,
                x,
                y as i32,
                if y == field.ground_y {
                    '┷'
                } else {
                    lightning_glyph(x - previous_x)
                },
                channel_color,
                glow_color,
                strike.intensity * params.glow,
            );
        }

        for fork in 0..params.forks {
            if main.len() < 3 {
                break;
            }
            let pick = hash01(strike_seed ^ (bolt as u64 * 977), 900 + fork as u64);
            let start_index = ((main.len() - 2) as f32 * (0.16 + pick * 0.58)) as usize;
            let (mut fork_x, mut fork_y) = main[start_index.min(main.len() - 1)];
            let direction = if hash01(strike_seed, 1200 + (bolt * 8 + fork) as u64) < 0.5 {
                -1
            } else {
                1
            };
            let max_length = (height / 5).clamp(2, 10);
            let length = (2.0
                + hash01(strike_seed, 1500 + (bolt * 8 + fork) as u64) * max_length as f32)
                .round() as usize;
            for step in 0..length {
                fork_y += 1;
                if fork_y > field.ground_y as i32 {
                    break;
                }
                let fork_jitter = hash01(
                    strike_seed ^ (fork as u64 * 1301),
                    1800 + (bolt * 64 + step) as u64,
                );
                if step % 2 == 0 || fork_jitter > 0.62 {
                    fork_x += direction;
                }
                if fork_x < 0 || fork_x >= width as i32 {
                    break;
                }
                let point = (fork_x, fork_y);
                all_points.push(point);
                put_lightning(
                    grid,
                    fork_x,
                    fork_y,
                    lightning_glyph(direction),
                    lerp_color(palette[3], channel_color, 0.62),
                    glow_color,
                    strike.intensity * params.glow * 0.62,
                );
            }
        }
    }

    all_points
}

fn draw_ground_reflection(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    params: &ThunderheadParams,
    strike: StrikeState,
    field: &StormField,
    channels: &[(i32, i32)],
) {
    if params.reflect <= 0.0 || channels.is_empty() || field.ground_y + 1 >= height {
        return;
    }
    let reflection_color = lerp_color(darken(palette[2], 15), palette[3], 0.64);
    let ground = field.ground_y as f32;

    for (index, &(x, y)) in channels.iter().enumerate() {
        let distance = ground - y as f32;
        if distance < 0.0 {
            continue;
        }
        let reflected_y = ground + 1.0 + distance * 0.43;
        if reflected_y >= height as f32 {
            continue;
        }
        let fade = (1.0 - (reflected_y - ground) / (height as f32 - ground).max(1.0))
            .clamp(0.0, 1.0)
            * params.reflect
            * strike.intensity;
        if hash01(seed ^ strike.id as u64, 2300 + index as u64) > fade.clamp(0.0, 0.94) {
            continue;
        }
        let wobble = (distance * 1.7 + t * params.speed * 3.0 + index as f32 * 0.37).sin();
        let reflected_x =
            x + (wobble * (1.0 + distance * 0.08) * params.wind.abs().max(0.3)) as i32;
        if reflected_x < 0 || reflected_x >= width as i32 {
            continue;
        }
        put_preserving_bg(
            grid,
            reflected_x,
            reflected_y.round() as i32,
            if fade > 0.62 { '┊' } else { '·' },
            lerp_color(darken(reflection_color, 38), reflection_color, fade),
        );
    }
}

pub(crate) fn draw_thunderhead(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    _rng: &mut StdRng,
    t: f32,
    params: &ThunderheadParams,
) {
    if width == 0 || height == 0 || grid.is_empty() {
        return;
    }

    // 1. Precompute column and row phases, plus the deterministic strike clock.
    let field = StormField::new(width, height, t, params.speed);
    let strike = strike_state(t, params);
    // 2. Fill the sky gradient and rain-swept reflective ground in one bounded pass.
    measure_layer("thunderhead", "storm_sky", || {
        draw_storm_sky(grid, width, height, seed, palette, strike, &field)
    });
    // 3. Reconstruct analytic rain positions from seed identities and the frame time.
    measure_layer("thunderhead", "rain_curtain", || {
        draw_rain(grid, width, height, seed, palette, t, params, &field)
    });
    // 4. Evaluate the anvil, updraft, overshoot, and rotating shelf cloud volumes.
    measure_layer("thunderhead", "supercell", || {
        draw_supercell(
            grid, width, height, seed, palette, t, params, strike, &field,
        )
    });
    // 5. Derive bounded channel topology from seed and strike index, then rasterize forks.
    let channels = measure_layer("thunderhead", "lightning", || {
        draw_lightning(grid, width, height, seed, palette, params, strike, &field)
    });
    // 6. Mirror visible channel samples into the ground with time-driven wave breakup.
    measure_layer("thunderhead", "ground_reflection", || {
        draw_ground_reflection(
            grid, width, height, seed, palette, t, params, strike, &field, &channels,
        )
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::make_palette;
    use crate::render::grid_to_plain;
    use rand::SeedableRng;

    fn frame(width: usize, height: usize, seed: u64, t: f32, params: &ThunderheadParams) -> Grid {
        let mut grid = vec![vec![Cell::blank(); width]; height];
        let palette = make_palette(seed);
        let mut rng = StdRng::seed_from_u64(seed);
        draw_thunderhead(
            &mut grid, width, height, seed, &palette, &mut rng, t, params,
        );
        grid
    }

    fn plain(grid: &Grid) -> String {
        grid_to_plain(grid).join("\n")
    }

    #[test]
    fn deterministic_seeded_frame_and_visible_motion() {
        let params = ThunderheadParams::default();
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
            99.0, -1.0, -99.0, 99.0, 99.0, 99.0, -1.0, 99.0, 0.0, 99.0, -1.0,
        ];
        let params = ThunderheadParams::from_inputs(&[], Some(&values));
        insta::assert_debug_snapshot!(params, @r###"
        ThunderheadParams {
            scale: 1.35,
            turbulence: 0.0,
            shear: -1.0,
            rain: 1.5,
            bolts: 5,
            forks: 6,
            flash: 0.0,
            wind: 1.5,
            speed: 0.05,
            glow: 1.5,
            reflect: 0.0,
        }
        "###);
    }

    #[test]
    fn tiny_grids_and_parameter_extrema_terminate() {
        let maxed = ThunderheadParams {
            scale: 1.35,
            turbulence: 1.5,
            shear: -1.0,
            rain: 1.5,
            bolts: 5,
            forks: 6,
            flash: 1.5,
            wind: 1.5,
            speed: 3.0,
            glow: 1.5,
            reflect: 1.5,
        };
        for size in [(1usize, 1usize), (2, 1), (3, 5), (9, 2)] {
            let grid = frame(size.0, size.1, 7, 50_000.0, &maxed);
            assert_eq!(grid.len(), size.1);
            assert!(grid.iter().all(|row| row.len() == size.0));
        }
    }

    #[test]
    fn strike_frame_contains_channels_and_reflection() {
        let grid = frame(100, 36, 42, 0.0, &ThunderheadParams::default());
        let ground_y = (36.0f32 * 0.79).round() as usize;
        let channel = |cell: &&Cell| matches!(cell.ch, '│' | '╱' | '╲' | '┷');
        let reflection = |cell: &&Cell| matches!(cell.ch, '┊' | '·');
        let sky_channels = grid[..=ground_y]
            .iter()
            .flat_map(|row| row.iter())
            .filter(channel)
            .count();
        let ground_reflections = grid[ground_y + 1..]
            .iter()
            .flat_map(|row| row.iter())
            .filter(reflection)
            .count();
        assert!(sky_channels >= 12);
        assert!(ground_reflections >= 1);
    }

    #[test]
    fn optional_layers_change_the_composition() {
        let params = ThunderheadParams::default();
        let full = frame(90, 32, 77, 0.0, &params);
        let dry = frame(
            90,
            32,
            77,
            0.0,
            &ThunderheadParams {
                rain: 0.0,
                flash: 0.0,
                glow: 0.0,
                reflect: 0.0,
                ..params
            },
        );
        assert_ne!(full, dry);
    }

    #[test]
    fn snapshot_thunderhead_strike() {
        insta::assert_snapshot!(plain(&frame(
            80,
            28,
            42,
            0.0,
            &ThunderheadParams::default(),
        )));
    }

    #[test]
    fn snapshot_thunderhead_between_strikes() {
        insta::assert_snapshot!(plain(&frame(
            80,
            28,
            42,
            2.75,
            &ThunderheadParams::default(),
        )));
    }

    #[test]
    #[ignore = "release-only performance probe; run with --release --ignored"]
    fn perf_thunderhead_frame() {
        use std::hint::black_box;
        use std::time::Instant;

        let width = 320;
        let height = 100;
        let frames = 600;
        let seed = 42;
        let params = ThunderheadParams::default();
        let palette = make_palette(seed);
        let mut grid = vec![vec![Cell::blank(); width]; height];
        let started = Instant::now();
        for index in 0..frames {
            let mut rng = StdRng::seed_from_u64(seed);
            draw_thunderhead(
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
        eprintln!("thunderhead {width}x{height}: {average_ms:.3} ms/frame");
        assert!(
            average_ms < 16.0,
            "{average_ms:.3} ms exceeds 60 fps budget"
        );
    }
}
