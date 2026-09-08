use crossterm::style::Color;
use rand::RngExt;
use rand::SeedableRng;
use rand::rngs::StdRng;
use rayon::prelude::*;

use super::_33_cosmograph::FBM_OCTAVES;
use crate::_0_profile::measure_layer;
use crate::color::{darken, lerp_color, lighten};
use crate::opts::param_f32;
use crate::pp::pp_hash2;
use crate::registry::{AnimKind, Mode, ModeFrame, Param};
use crate::types::{Cell, Grid};

const TAU: f32 = std::f32::consts::TAU;

struct NebulaColumn {
    x0: [i32; FBM_OCTAVES],
    sx: [f32; FBM_OCTAVES],
    edges: [[f32; 2]; FBM_OCTAVES],
}

/// Frame-owned O(width) scratch. Horizontal noise interpolation is constant
/// throughout a y lattice band, so refresh it only when a scanline crosses an
/// octave's lattice edge. Keep the arithmetic order of FbmRow / pp_fbm.
struct NebulaRows {
    columns: Vec<NebulaColumn>,
    y0: [Option<i32>; FBM_OCTAVES],
}

impl NebulaRows {
    #[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
    fn new(nx: &[[f32; 2]], bias: f32) -> Self {
        let columns = nx
            .iter()
            .map(|&[x, _]| {
                let fx = x * 2.2 + bias;
                let mut column = NebulaColumn {
                    x0: [0; FBM_OCTAVES],
                    sx: [0.0; FBM_OCTAVES],
                    edges: [[0.0; 2]; FBM_OCTAVES],
                };
                let mut freq = 1.0;
                for o in 0..FBM_OCTAVES {
                    let fxo = fx * freq;
                    let ix = fxo.floor() as i32;
                    let tx = fxo - ix as f32;
                    column.x0[o] = ix;
                    column.sx[o] = tx * tx * (3.0 - 2.0 * tx);
                    freq *= 2.0;
                }
                column
            })
            .collect();
        Self {
            columns,
            y0: [None; FBM_OCTAVES],
        }
    }

    #[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
    fn prepare_y(&mut self, fy: f32, seed: u64) -> [f32; FBM_OCTAVES] {
        let mut sy = [0.0; FBM_OCTAVES];
        let mut freq = 1.0;
        for o in 0..FBM_OCTAVES {
            let fyo = fy * freq;
            let iy = fyo.floor() as i32;
            let ty = fyo - iy as f32;
            sy[o] = ty * ty * (3.0 - 2.0 * ty);
            freq *= 2.0;
            if self.y0[o] == Some(iy) {
                continue;
            }
            self.y0[o] = Some(iy);
            let sd = seed.wrapping_add(o as u64 * 101);
            let mut last_x = None;
            let mut c = [0.0; 4];
            for column in &mut self.columns {
                let ix = column.x0[o];
                if last_x != Some(ix) {
                    c = [
                        pp_hash2(ix, iy, sd),
                        pp_hash2(ix + 1, iy, sd),
                        pp_hash2(ix, iy + 1, sd),
                        pp_hash2(ix + 1, iy + 1, sd),
                    ];
                    last_x = Some(ix);
                }
                let sx = column.sx[o];
                column.edges[o] = [c[0] + (c[1] - c[0]) * sx, c[2] + (c[3] - c[2]) * sx];
            }
        }
        sy
    }
}

impl NebulaColumn {
    #[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
    fn at(&self, sy: &[f32; FBM_OCTAVES]) -> f32 {
        let mut v = 0.0;
        let mut amp = 0.5;
        for (o, &sy) in sy.iter().enumerate() {
            let [a, b] = self.edges[o];
            v += amp * (a + (b - a) * sy);
            amp *= 0.5;
        }
        v
    }
}

pub(super) struct GemAetherium2Mode;

pub(super) static MODE: GemAetherium2Mode = GemAetherium2Mode;

const PARAMS: &[Param] = &[
    param!("RINGS", "astrolabe armillary rings", 2.0, 10.0, 6.0, 1.0),
    param!("PLANETS", "orrery celestial bodies", 1.0, 12.0, 7.0, 1.0),
    param!("GEARS", "mechanical epicyclic gears", 0.0, 8.0, 4.0, 1.0),
    param!(
        "ZODIAC",
        "zodiac constellation points",
        4.0,
        24.0,
        12.0,
        1.0
    ),
    param!(
        "SPEED",
        "celestial mechanics velocity",
        0.05,
        3.0,
        0.75,
        0.05
    ),
    param!("TILT", "axial orbital inclination", 0.0, 1.0, 0.45, 0.05),
    param!("NEBULA", "aetherial cosmic dust fog", 0.0, 1.5, 0.7, 0.05),
    param!("RAYS", "mystic chronos ray bursts", 0.0, 1.5, 0.8, 0.05),
    param!("COMETS", "hyperbolic orbital comets", 0.0, 12.0, 4.0, 1.0),
    param!("RUNES", "arcane glyph ring density", 0.0, 1.0, 0.85, 0.05),
    param!("PULSE", "harmonic resonance pulse", 0.0, 2.0, 1.0, 0.1),
    param!(
        "HARMONY",
        "pythagorean celestial ratios",
        1.0,
        8.0,
        3.0,
        1.0
    ),
];

impl Mode for GemAetherium2Mode {
    #[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
    fn name(&self) -> &'static str {
        "gem-aetherium-2"
    }

    #[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
    fn help(&self) -> &'static str {
        "Grand Aetherial Orrery II, optimized 3D rotating armillary spheres, epicyclic clockwork gears, zodiac astrolabe, orbiting planets & comets [rings] [planets] [gears] [zodiac] [speed] [tilt] [nebula] [rays] [comets] [runes] [pulse] [harmony]"
    }

    #[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
    fn animation(&self) -> AnimKind {
        AnimKind::Iterate
    }

    #[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
    fn params(&self) -> &'static [Param] {
        PARAMS
    }

    #[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
    fn render(&self, frame: &mut ModeFrame<'_>) {
        let params = AetheriumParams::from_inputs(frame.args, frame.param_values);
        draw_gem_aetherium_2(
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
pub(crate) struct AetheriumParams {
    pub(crate) rings: usize,
    pub(crate) planets: usize,
    pub(crate) gears: usize,
    pub(crate) zodiac: usize,
    pub(crate) speed: f32,
    pub(crate) tilt: f32,
    pub(crate) nebula: f32,
    pub(crate) rays: f32,
    pub(crate) comets: usize,
    pub(crate) runes: f32,
    pub(crate) pulse: f32,
    pub(crate) harmony: f32,
}

impl Default for AetheriumParams {
    #[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
    fn default() -> Self {
        Self {
            rings: 6,
            planets: 7,
            gears: 4,
            zodiac: 12,
            speed: 0.75,
            tilt: 0.45,
            nebula: 0.7,
            rays: 0.8,
            comets: 4,
            runes: 0.85,
            pulse: 1.0,
            harmony: 3.0,
        }
    }
}

impl AetheriumParams {
    #[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
    pub(crate) fn from_args(args: &[String]) -> Self {
        Self::from_inputs(args, None)
    }

    #[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
    pub(crate) fn from_inputs(args: &[String], param_values: Option<&[f32]>) -> Self {
        let read = |index: usize, key: &str, default: f32| {
            args.get(index)
                .and_then(|value| value.parse::<f32>().ok())
                .or_else(|| {
                    param_values
                        .and_then(|values| values.get(index - 4))
                        .copied()
                })
                .unwrap_or_else(|| param_f32(key, default))
        };
        Self {
            rings: read(4, "RINGS", 6.0).round().clamp(2.0, 10.0) as usize,
            planets: read(5, "PLANETS", 7.0).round().clamp(1.0, 12.0) as usize,
            gears: read(6, "GEARS", 4.0).round().clamp(0.0, 8.0) as usize,
            zodiac: read(7, "ZODIAC", 12.0).round().clamp(4.0, 24.0) as usize,
            speed: read(8, "SPEED", 0.75).clamp(0.05, 3.0),
            tilt: read(9, "TILT", 0.45).clamp(0.0, 1.0),
            nebula: read(10, "NEBULA", 0.7).clamp(0.0, 1.5),
            rays: read(11, "RAYS", 0.8).clamp(0.0, 1.5),
            comets: read(12, "COMETS", 4.0).round().clamp(0.0, 12.0) as usize,
            runes: read(13, "RUNES", 0.85).clamp(0.0, 1.0),
            pulse: read(14, "PULSE", 1.0).clamp(0.0, 2.0),
            harmony: read(15, "HARMONY", 3.0).clamp(1.0, 8.0),
        }
    }
}

#[derive(Clone, Copy)]
struct PlanetDef {
    orbit_r: f32,
    speed_mult: f32,
    inclination: f32,
    node_angle: f32,
    phase_offset: f32,
    glyph: char,
    has_ring: bool,
    color_idx: usize,
    moons: usize,
}

#[derive(Clone, Copy)]
struct CometDef {
    semi_major: f32,
    eccentricity: f32,
    inclination: f32,
    node_angle: f32,
    period: f32,
    phase: f32,
    tail_len: usize,
}

#[derive(Clone, Copy)]
struct GearDef {
    radius: f32,
    teeth: usize,
    center_r: f32,
    center_angle: f32,
    speed_ratio: f32,
    spokes: usize,
}

const ZODIAC_SYMBOLS: &[char] = &[
    '♈', '♉', '♊', '♋', '♌', '♍', '♎', '♏', '♐', '♑', '♒', '♓', '☉', '☽', '☿', '♀',
    '♂', '♃', '♄', '♅', '♆', '♇', '✧', '✦',
];

const RUNIC_CHARS: &[char] = &[
    'ᚠ', 'ᚢ', 'ᚦ', 'ᚨ', 'ᚱ', 'ᚲ', 'ᚷ', 'ᚹ', 'ᚺ', 'ᚾ', 'ᛁ', 'ᛃ', 'ᛈ', 'ᛉ', 'ᛋ', 'ᛏ', 'ᛒ', 'ᛖ', 'ᛗ',
    'ᛚ', 'ᛜ', 'ᛞ', 'ᛟ', 'α', 'β', 'γ', 'δ', 'ε', 'ζ', 'η', 'θ', 'ι', 'κ', 'λ', 'μ', 'ν', 'ξ', 'ο',
    'π', 'ρ', 'σ', 'τ', 'υ', 'φ', 'χ', 'ψ', 'ω',
];

#[inline(always)]
#[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
fn rotate_3d(x: f32, y: f32, z: f32, pitch: f32, yaw: f32, roll: f32) -> (f32, f32, f32) {
    let cy = yaw.cos();
    let sy = yaw.sin();
    let x1 = x * cy + z * sy;
    let y1 = y;
    let z1 = -x * sy + z * cy;

    let cp = pitch.cos();
    let sp = pitch.sin();
    let x2 = x1;
    let y2 = y1 * cp - z1 * sp;
    let z2 = y1 * sp + z1 * cp;

    let cr = roll.cos();
    let sr = roll.sin();
    let x3 = x2 * cr - y2 * sr;
    let y3 = x2 * sr + y2 * cr;
    let z3 = z2;

    (x3, y3, z3)
}

#[inline(always)]
#[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
fn project(
    x: f32,
    y: f32,
    z: f32,
    cx: f32,
    cy: f32,
    aspect: f32,
    fov: f32,
) -> Option<(i32, i32, f32)> {
    let dist = fov + z;
    if dist <= 0.1 {
        return None;
    }
    let factor = fov / dist;
    let px = cx + x * factor * aspect;
    let py = cy + y * factor;
    Some((px.round() as i32, py.round() as i32, z))
}

#[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
pub(crate) fn draw_gem_aetherium_2(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    _rng: &mut StdRng,
    t: f32,
    params: &AetheriumParams,
) {
    if width == 0 || height == 0 {
        return;
    }

    let cx = (width as f32) * 0.5;
    let cy = (height as f32) * 0.5;
    let min_dim = (width as f32).min(height as f32 * 2.0);
    let scale = min_dim * 0.44;
    let aspect = 2.05;
    let fov = scale * 2.8;

    let anim_t = t * params.speed;
    let pulse_val = 1.0 + (anim_t * 2.0 * params.pulse).sin() * 0.08 * params.pulse;

    let bg_color = palette[0];
    let star_dim = darken(palette[1], 30);
    let star_bright = lighten(palette[4], 20);
    let aether_color = lerp_color(palette[0], palette[2], 0.35);

    let mut bg_rng = StdRng::seed_from_u64(seed.wrapping_add(101));
    let star_count = ((width * height) as f32 * 0.05).min(400.0) as usize;
    let mut stars = Vec::with_capacity(star_count);
    for _ in 0..star_count {
        let sx = bg_rng.random_range(0..width);
        let sy = bg_rng.random_range(0..height);
        let blink_speed = bg_rng.random_range(0.8..2.5);
        let blink_phase = bg_rng.random_range(0.0..TAU);
        let ch_type = bg_rng.random_range(0..10);
        let ch = match ch_type {
            0..=5 => '·',
            6..=7 => '˙',
            8 => '✦',
            _ => '✧',
        };
        stars.push((sx, sy, blink_speed, blink_phase, ch));
    }

    let nebula_on = params.nebula > 0.05;
    let rays_on = params.rays > 0.05;
    let neb_seed = seed.wrapping_add(999);
    let neb_fx_bias = anim_t * 0.03;
    let neb_fy_bias = anim_t * 0.02;
    let neb_dim = darken(aether_color, 25);
    let ray_spin = anim_t * 0.15;
    let ray_k = params.zodiac as f32 * 0.5;

    let inv_half_w = 1.0 / (width as f32 * 0.5);
    let inv_half_h = 1.0 / (height as f32 * 0.5);
    measure_layer("gem-aetherium-2", "background", || {
        let nx_col: Vec<_> = if nebula_on || rays_on {
            (0..width)
                .map(|x| {
                    let nx = (x as f32 - cx) * inv_half_w;
                    [nx, nx * nx]
                })
                .collect()
        } else {
            Vec::new()
        };
        measure_layer("gem-aetherium-2", "nebula", || {
            if !nebula_on {
                for row in grid.iter_mut().take(height) {
                    row[..width].fill(Cell::new(' ', bg_color));
                }
                return;
            }
            let mut noise = NebulaRows::new(&nx_col, neb_fx_bias);
            for (y, row) in grid.iter_mut().take(height).enumerate() {
                let ny = (y as f32 - cy) * inv_half_h;
                let ny2 = ny * ny;
                let sy = noise.prepare_y(ny * 2.2 - neb_fy_bias, neb_seed);
                for ((cell, column), &[_, nx2]) in row.iter_mut().zip(&noise.columns).zip(&nx_col) {
                    let dist_center = (nx2 + ny2).sqrt();
                    let fbm_val = column.at(&sy);
                    let neb_intensity = (fbm_val * 0.8 + (1.0 - dist_center * 0.85))
                        .clamp(0.0, 1.0)
                        * params.nebula;
                    *cell = if neb_intensity > 0.65 {
                        let ch = match ((neb_intensity - 0.65) * 8.0) as usize {
                            0 => '░',
                            1 => '▒',
                            2 => '▓',
                            _ => '▒',
                        };
                        Cell::new(
                            ch,
                            lerp_color(bg_color, aether_color, neb_intensity.min(1.0)),
                        )
                    } else if neb_intensity > 0.45 {
                        Cell::new(' ', neb_dim)
                    } else {
                        Cell::new(' ', bg_color)
                    };
                }
            }
        });
        measure_layer("gem-aetherium-2", "rays", || {
            if !rays_on {
                return;
            }
            let shade_row = |(y, row): (usize, &mut Vec<Cell>)| {
                let ny = (y as f32 - cy) * inv_half_h;
                let ny2 = ny * ny;
                for (cell, &[nx, nx2]) in row.iter_mut().zip(&nx_col) {
                    let dist_center = (nx2 + ny2).sqrt();
                    if dist_center < 1.4 {
                        let atten = 1.0 / (dist_center * 1.5 + 0.3);
                        if atten * params.rays > 0.4 {
                            let angle = ny.atan2(nx) + ray_spin;
                            let ray_wave = (angle * ray_k).sin();
                            if ray_wave > 0.0 {
                                let ray_power = ray_wave.powf(4.0) * atten * params.rays;
                                if ray_power > 0.4 {
                                    let ch = if ray_power > 1.2 {
                                        '│'
                                    } else if ray_power > 0.8 {
                                        '┆'
                                    } else {
                                        '┊'
                                    };
                                    let fg =
                                        lerp_color(cell.fg, palette[4], (ray_power * 0.4).min(0.9));
                                    *cell = Cell::new(ch, fg);
                                }
                            }
                        }
                    }
                }
            };
            // Disjoint rows have no RNG or blending dependencies. Keep small
            // previews serial to avoid scheduling overhead; time both paths here
            // on the caller thread so layer capture includes worker completion.
            if width * height >= 65_536 {
                grid[..height]
                    .par_iter_mut()
                    .enumerate()
                    .with_min_len(16)
                    .for_each(shade_row);
            } else {
                grid.iter_mut().take(height).enumerate().for_each(shade_row);
            }
        });
    });

    measure_layer("gem-aetherium-2", "starfield", || {
        for (sx, sy, b_spd, b_phs, sch) in stars {
            let blink = ((anim_t * b_spd + b_phs).sin() + 1.0) * 0.5;
            let col = lerp_color(star_dim, star_bright, blink);
            if grid[sy][sx].ch == ' ' {
                grid[sy][sx] = Cell::new(sch, col);
            }
        }
    });

    // Bounded sparse z-buffer: since foreground geometry covers < 50k pixels even on 2000x1000,
    // we use a sparse map when total grid elements > 16k to avoid 8MB memset/realloc,
    // and flat array for small/standard grids.
    let mut z_buf_dense: Vec<f32>;
    let mut z_buf_sparse: std::collections::HashMap<u32, f32>;
    let use_sparse = width * height > 16384;

    if use_sparse {
        z_buf_dense = Vec::new();
        z_buf_sparse = std::collections::HashMap::with_capacity(4096);
    } else {
        z_buf_dense = vec![-1000.0; width * height];
        z_buf_sparse = std::collections::HashMap::new();
    }

    let mut put_z = |grid: &mut Grid, x: i32, y: i32, z: f32, ch: char, fg: Color| {
        if x >= 0 && y >= 0 && (x as usize) < width && (y as usize) < height {
            let ux = x as usize;
            let uy = y as usize;
            if use_sparse {
                let key = ((uy as u32) << 16) | (ux as u32);
                let entry = z_buf_sparse.entry(key).or_insert(-1000.0);
                if z > *entry {
                    *entry = z;
                    grid[uy][ux] = Cell::new(ch, fg);
                }
            } else {
                let idx = uy * width + ux;
                if z > z_buf_dense[idx] {
                    z_buf_dense[idx] = z;
                    grid[uy][ux] = Cell::new(ch, fg);
                }
            }
        }
    };

    // ------------------------------------------------------------------------
    // 2. Epicyclic Clockwork Gears
    // ------------------------------------------------------------------------
    measure_layer("gem-aetherium-2", "gears", || {
        if params.gears > 0 {
            let mut gear_rng = StdRng::seed_from_u64(seed.wrapping_add(202));
            let num_gears = params.gears;
            let mut gear_defs = Vec::with_capacity(num_gears);

            for g in 0..num_gears {
                let gr = scale * (0.2 + (g as f32) * 0.18);
                let g_teeth = (12 + g * 8) * (params.harmony as usize).max(1);
                let c_dist = if g == 0 {
                    0.0
                } else {
                    scale * (0.35 + (g as f32) * 0.15)
                };
                let c_ang = (g as f32) * TAU / (num_gears as f32) + gear_rng.random_range(0.0..0.5);
                let s_ratio = if g % 2 == 0 { 1.0 } else { -1.33 } * (1.0 + (g as f32) * 0.3);
                let spokes = 4 + (g % 3) * 2;
                gear_defs.push(GearDef {
                    radius: gr,
                    teeth: g_teeth,
                    center_r: c_dist,
                    center_angle: c_ang,
                    speed_ratio: s_ratio,
                    spokes,
                });
            }

            let gear_color = darken(palette[1], 15);
            let gear_highlight = palette[3];
            let spoke_color = darken(gear_color, 20);
            let tilt_pitch = params.tilt * 0.6;
            let yaw_rot = anim_t * 0.1;
            let gz = -scale * 0.6;

            for gear in &gear_defs {
                let g_center_x = gear.center_r * gear.center_angle.cos();
                let g_center_y = gear.center_r * gear.center_angle.sin() * 0.5;
                let g_rot = anim_t * gear.speed_ratio;

                let step_count = gear.teeth * 4;
                let inv_step_count = TAU / step_count as f32;
                let teeth_f = gear.teeth as f32;
                let tooth_amp = scale * 0.025;

                for s in 0..step_count {
                    let theta = s as f32 * inv_step_count;
                    let phi = theta + g_rot;
                    let tooth_wave = (phi * teeth_f).sin();
                    let tooth_r = gear.radius + tooth_wave * tooth_amp;
                    let gx = g_center_x + tooth_r * phi.cos();
                    let gy = g_center_y + tooth_r * phi.sin();

                    let (rx, ry, rz) = rotate_3d(gx, gy, gz, tilt_pitch, yaw_rot, 0.0);
                    if let Some((px, py, pz)) = project(rx, ry, rz, cx, cy, aspect, fov) {
                        let ch = if tooth_wave > 0.4 {
                            '⚙'
                        } else if s % 2 == 0 {
                            '▪'
                        } else {
                            '▫'
                        };
                        let col = if tooth_wave > 0.6 {
                            gear_highlight
                        } else {
                            gear_color
                        };
                        put_z(grid, px, py, pz, ch, col);
                    }
                }

                // Gear Spokes
                let sp_step_angle = TAU / gear.spokes as f32;
                let sp_steps = 8;
                let inv_sp_steps = gear.radius / sp_steps as f32;
                for sp in 0..gear.spokes {
                    let sp_angle = g_rot + sp as f32 * sp_step_angle;
                    let cos_ang = sp_angle.cos();
                    let sin_ang = sp_angle.sin();
                    for step in 1..=sp_steps {
                        let d = step as f32 * inv_sp_steps;
                        let gx = g_center_x + d * cos_ang;
                        let gy = g_center_y + d * sin_ang;
                        let (rx, ry, rz) = rotate_3d(gx, gy, gz, tilt_pitch, yaw_rot, 0.0);
                        if let Some((px, py, pz)) = project(rx, ry, rz, cx, cy, aspect, fov) {
                            put_z(grid, px, py, pz, '─', spoke_color);
                        }
                    }
                }
            }
        }
    });

    // ------------------------------------------------------------------------
    // 3. Nested 3D Armillary Spheres
    // ------------------------------------------------------------------------
    measure_layer("gem-aetherium-2", "armillary", || {
        let num_rings = params.rings;
        let mut arm_rng = StdRng::seed_from_u64(seed.wrapping_add(303));

        for r in 0..num_rings {
            let ring_frac = (r + 1) as f32 / num_rings as f32;
            let ring_radius = scale * (0.35 + ring_frac * 0.65) * pulse_val;
            let ring_speed = (0.3 + (1.0 - ring_frac) * 0.7) * if r % 2 == 0 { 1.0 } else { -0.85 };
            let base_pitch = arm_rng.random_range(0.2..1.2) * params.tilt;
            let base_yaw = arm_rng.random_range(0.0..TAU);
            let base_roll = arm_rng.random_range(-0.5..0.5);

            let cur_yaw = base_yaw + anim_t * ring_speed;
            let cur_pitch = base_pitch + (anim_t * 0.4 + (r as f32)).sin() * 0.25 * params.tilt;

            let num_samples = (ring_radius * 12.0).clamp(60.0, 300.0) as usize;
            let ring_color = match r % 4 {
                0 => palette[3],
                1 => palette[2],
                2 => palette[4],
                _ => palette[1],
            };
            let rune_col = lighten(ring_color, 24);
            let regular_col = darken(ring_color, 12);

            let step_rad = TAU / num_samples as f32;
            let major_mod = (num_samples / 12).max(1);
            let minor_mod = (num_samples / 48).max(1);

            for i in 0..num_samples {
                let phi = i as f32 * step_rad;
                let lx = ring_radius * phi.cos();
                let ly = ring_radius * phi.sin();

                let (wx, wy, wz) = rotate_3d(lx, ly, 0.0, cur_pitch, cur_yaw, base_roll);
                if let Some((px, py, pz)) = project(wx, wy, wz, cx, cy, aspect, fov) {
                    let is_major_tick = (i % major_mod) == 0;
                    let is_minor_tick = (i % minor_mod) == 0;

                    let (ch, col) = if is_major_tick && params.runes > 0.2 {
                        let rune_idx = (r * 7 + i / major_mod) % RUNIC_CHARS.len();
                        (RUNIC_CHARS[rune_idx], rune_col)
                    } else if is_minor_tick {
                        ('┼', ring_color)
                    } else {
                        let stroke_ch = if phi.sin().abs() > 0.707 {
                            '│'
                        } else {
                            '─'
                        };
                        (stroke_ch, regular_col)
                    };

                    put_z(grid, px, py, pz, ch, col);
                }
            }
        }
    });

    // ------------------------------------------------------------------------
    // 4. Outer Astrolabe Limb & Zodiac Horizon Ring
    // ------------------------------------------------------------------------
    measure_layer("gem-aetherium-2", "limb", || {
        let limb_radius = scale * 1.08 * pulse_val;
        let zodiac_count = params.zodiac;
        let zodiac_col = palette[3];
        let zodiac_speed = anim_t * 0.12;
        let tilt_factor = 0.88 - params.tilt * 0.35;
        let tilt_pitch = params.tilt * 0.5;

        // Limb outer graduated border
        let limb_samples = (limb_radius * 14.0).clamp(120.0, 360.0) as usize;
        let l_step = TAU / limb_samples as f32;
        let lz = -scale * 0.1;
        let card_mod = (limb_samples / 4).max(1);
        let border_col = palette[4];

        for i in 0..limb_samples {
            let theta = i as f32 * l_step;
            let lx = limb_radius * theta.cos();
            let ly = limb_radius * theta.sin() * tilt_factor;

            let (wx, wy, wz) = rotate_3d(lx, ly, lz, tilt_pitch, 0.0, 0.0);
            if let Some((px, py, pz)) = project(wx, wy, wz, cx, cy, aspect, fov) {
                let is_cardinal = (i % card_mod) == 0;
                let ch = if is_cardinal {
                    '❖'
                } else if i % 2 == 0 {
                    '═'
                } else {
                    '─'
                };
                put_z(grid, px, py, pz, ch, border_col);
            }
        }

        // Zodiac Constellation Houses & Radiating Spokes
        let spoke_steps = 14;
        let spoke_col = darken(palette[2], 30);
        let zodiac_sym_col = lighten(zodiac_col, 20);
        let zz = -scale * 0.05;

        for z in 0..zodiac_count {
            let z_angle = zodiac_speed + (z as f32 / zodiac_count as f32) * TAU;
            let zx = limb_radius * 0.95 * z_angle.cos();
            let zy = limb_radius * 0.95 * z_angle.sin() * tilt_factor;

            let (wx, wy, wz) = rotate_3d(zx, zy, zz, tilt_pitch, 0.0, 0.0);
            if let Some((px, py, pz)) = project(wx, wy, wz, cx, cy, aspect, fov) {
                let symbol = ZODIAC_SYMBOLS[z % ZODIAC_SYMBOLS.len()];
                put_z(grid, px, py, pz, symbol, zodiac_sym_col);

                for s in 1..spoke_steps {
                    let frac = s as f32 / spoke_steps as f32;
                    let sx = zx * (1.0 - frac);
                    let sy = zy * (1.0 - frac);
                    let (swx, swy, swz) = rotate_3d(sx, sy, zz, tilt_pitch, 0.0, 0.0);
                    if let Some((spx, spy, spz)) = project(swx, swy, swz, cx, cy, aspect, fov) {
                        let ch = if s % 3 == 0 { '·' } else { '┄' };
                        put_z(grid, spx, spy, spz, ch, spoke_col);
                    }
                }
            }
        }
    });

    // ------------------------------------------------------------------------
    // 5. Grand Orrery Planetary Orbits & Moons
    // ------------------------------------------------------------------------
    measure_layer("gem-aetherium-2", "orrery", || {
        let mut orrery_rng = StdRng::seed_from_u64(seed.wrapping_add(404));
        let num_planets = params.planets;
        let planet_glyphs = ['☉', '☿', '♀', '♁', '♂', '♃', '♄', '♅', '♆', '♇', '✧', '◈'];
        let mut planet_defs = Vec::with_capacity(num_planets);

        for p in 0..num_planets {
            let p_frac = (p + 1) as f32 / (num_planets + 1) as f32;
            let orb_r = scale * (0.22 + p_frac * 0.78);
            let speed_m = 1.6 / (p_frac * 2.5 + 0.4).sqrt() * if p % 3 == 1 { 1.0 } else { 1.15 };
            let inc = orrery_rng.random_range(0.05..0.6) * params.tilt;
            let node_ang = orrery_rng.random_range(0.0..TAU);
            let phs = orrery_rng.random_range(0.0..TAU);
            let _sz = orrery_rng.random_range(0.8..1.8);
            let glyph = planet_glyphs[p % planet_glyphs.len()];
            let has_ring = p == 5 || (p > 2 && orrery_rng.random_bool(0.3));
            let moons = if p > 3 {
                orrery_rng.random_range(1..=3)
            } else {
                0
            };

            planet_defs.push(PlanetDef {
                orbit_r: orb_r,
                speed_mult: speed_m,
                inclination: inc,
                node_angle: node_ang,
                phase_offset: phs,
                glyph,
                has_ring,
                color_idx: (p % 4) + 1,
                moons,
            });
        }

        let moon_col = lighten(palette[4], 16);
        let node_advance = anim_t * 0.05;

        for planet in &planet_defs {
            let p_col = palette[planet.color_idx];
            let p_dot_col = darken(p_col, 36);
            let planet_node = planet.node_angle + node_advance;

            let trace_steps = (planet.orbit_r * 8.0).clamp(40.0, 180.0) as usize;
            let t_step = TAU / trace_steps as f32;
            let mut ts = 0;
            while ts < trace_steps {
                let trace_ang = ts as f32 * t_step;
                let ox = planet.orbit_r * trace_ang.cos();
                let oy = planet.orbit_r * trace_ang.sin();
                let (tx, ty, tz) = rotate_3d(ox, oy, 0.0, planet.inclination, planet_node, 0.0);
                if let Some((px, py, pz)) = project(tx, ty, tz, cx, cy, aspect, fov) {
                    put_z(grid, px, py, pz, '·', p_dot_col);
                }
                ts += 3;
            }

            // Planet Current Position
            let cur_phi = planet.phase_offset + anim_t * planet.speed_mult;
            let px0 = planet.orbit_r * cur_phi.cos();
            let py0 = planet.orbit_r * cur_phi.sin();
            let (pw_x, pw_y, pw_z) = rotate_3d(px0, py0, 0.0, planet.inclination, planet_node, 0.0);

            if let Some((px, py, pz)) = project(pw_x, pw_y, pw_z, cx, cy, aspect, fov) {
                // Planet Body
                put_z(grid, px, py, pz + 2.0, planet.glyph, lighten(p_col, 30));

                // Planetary Rings
                if planet.has_ring {
                    let ring_yaw = anim_t * 0.5;
                    let r_amp_x = scale * 0.06;
                    let r_amp_y = scale * 0.02;
                    for ra in 0..12 {
                        let r_angle = (ra as f32 / 12.0) * TAU;
                        let rx = r_amp_x * r_angle.cos();
                        let ry = r_amp_y * r_angle.sin();
                        let (rw_x, rw_y, rw_z) = rotate_3d(rx, ry, 0.0, 0.8, ring_yaw, 0.0);
                        if let Some((rpx, rpy, rpz)) =
                            project(pw_x + rw_x, pw_y + rw_y, pw_z + rw_z, cx, cy, aspect, fov)
                        {
                            put_z(grid, rpx, rpy, rpz + 1.5, '═', p_col);
                        }
                    }
                }

                // Moons
                for m in 0..planet.moons {
                    let m_dist = scale * (0.04 + (m as f32) * 0.025);
                    let m_speed = anim_t * (3.0 + (m as f32) * 2.0);
                    let mx = m_dist * m_speed.cos();
                    let my = m_dist * m_speed.sin();
                    let mz = m_dist * (m_speed * 0.5).sin() * 0.5;
                    let (mw_x, mw_y, mw_z) = rotate_3d(mx, my, mz, 0.3, 0.0, 0.0);
                    if let Some((mpx, mpy, mpz)) =
                        project(pw_x + mw_x, pw_y + mw_y, pw_z + mw_z, cx, cy, aspect, fov)
                    {
                        put_z(grid, mpx, mpy, mpz + 2.5, '∘', moon_col);
                    }
                }
            }
        }
    });

    // ------------------------------------------------------------------------
    // 6. Hyperbolic Orbital Comets with Glowing Ion Tails
    // ------------------------------------------------------------------------
    measure_layer("gem-aetherium-2", "comets", || {
        if params.comets > 0 {
            let mut comet_rng = StdRng::seed_from_u64(seed.wrapping_add(505));
            let num_comets = params.comets;
            let mut comet_defs = Vec::with_capacity(num_comets);

            for c in 0..num_comets {
                let a = scale * (0.7 + (c as f32) * 0.3);
                let e = comet_rng.random_range(0.72..0.92);
                let inc = comet_rng.random_range(0.3..1.4) * params.tilt;
                let node = comet_rng.random_range(0.0..TAU);
                let prd = comet_rng.random_range(0.4..1.2);
                let phs = comet_rng.random_range(0.0..TAU);
                let tlen = comet_rng.random_range(10..22);
                comet_defs.push(CometDef {
                    semi_major: a,
                    eccentricity: e,
                    inclination: inc,
                    node_angle: node,
                    period: prd,
                    phase: phs,
                    tail_len: tlen,
                });
            }

            for comet in &comet_defs {
                let mean_anomaly = (anim_t * comet.period + comet.phase) % TAU;
                let ecc_anomaly = mean_anomaly + comet.eccentricity * mean_anomaly.sin();
                let sqrt_1_plus_e = (1.0 + comet.eccentricity).sqrt();
                let sqrt_1_minus_e = (1.0 - comet.eccentricity).sqrt();
                let half_ecc = ecc_anomaly * 0.5;
                let true_anomaly =
                    2.0 * ((sqrt_1_plus_e * half_ecc.sin()).atan2(sqrt_1_minus_e * half_ecc.cos()));
                let r = comet.semi_major * (1.0 - comet.eccentricity * ecc_anomaly.cos());

                let head_x = r * true_anomaly.cos();
                let head_y = r * true_anomaly.sin();
                let (hw_x, hw_y, hw_z) = rotate_3d(
                    head_x,
                    head_y,
                    0.0,
                    comet.inclination,
                    comet.node_angle,
                    0.0,
                );

                if let Some((px, py, pz)) = project(hw_x, hw_y, hw_z, cx, cy, aspect, fov) {
                    put_z(grid, px, py, pz + 4.0, '✷', palette[4]);

                    // Comet Tail
                    let inv_tlen = 1.0 / comet.tail_len as f32;
                    for tail_step in 1..=comet.tail_len {
                        let lag = tail_step as f32 * 0.035;
                        let t_mean = (mean_anomaly - lag + TAU) % TAU;
                        let t_ecc = t_mean + comet.eccentricity * t_mean.sin();
                        let half_t_ecc = t_ecc * 0.5;
                        let t_true = 2.0
                            * ((sqrt_1_plus_e * half_t_ecc.sin())
                                .atan2(sqrt_1_minus_e * half_t_ecc.cos()));
                        let tr = comet.semi_major * (1.0 - comet.eccentricity * t_ecc.cos());

                        let tx = tr * t_true.cos();
                        let ty = tr * t_true.sin();
                        let (tw_x, tw_y, tw_z) =
                            rotate_3d(tx, ty, 0.0, comet.inclination, comet.node_angle, 0.0);

                        if let Some((tpx, tpy, tpz)) =
                            project(tw_x, tw_y, tw_z, cx, cy, aspect, fov)
                        {
                            let tail_frac = tail_step as f32 * inv_tlen;
                            let ch = if tail_step < 4 {
                                '※'
                            } else if tail_step < 9 {
                                '~'
                            } else {
                                '·'
                            };
                            let col = lerp_color(palette[4], palette[2], tail_frac);
                            put_z(grid, tpx, tpy, tpz + 3.0 - tail_frac, ch, col);
                        }
                    }
                }
            }
        }
    });

    // ------------------------------------------------------------------------
    // 7. Mystic Central Chronos Core & Alidade / Sighting Rule
    // ------------------------------------------------------------------------
    measure_layer("gem-aetherium-2", "core", || {
        let core_radius = scale * 0.12 * pulse_val;
        let core_radius_sq = core_radius * core_radius;
        let light_core_col = lighten(palette[4], 35);
        let mid_core_col = palette[3];

        for dy in -3..=3 {
            for dx in -5..=5 {
                let dist_sq = (dx as f32 * 0.5).powi(2) + (dy as f32).powi(2);
                if dist_sq <= core_radius_sq {
                    let px = (cx + dx as f32).round() as i32;
                    let py = (cy + dy as f32).round() as i32;
                    let ch = if dist_sq < 1.0 {
                        '❂'
                    } else if dist_sq < 3.0 {
                        '☼'
                    } else {
                        '░'
                    };
                    let col = if dist_sq < 1.5 {
                        light_core_col
                    } else {
                        mid_core_col
                    };
                    put_z(grid, px, py, 10.0, ch, col);
                }
            }
        }

        // Rotating Astrolabe Sighting Alidade
        let alidade_len = scale * 1.15 * pulse_val;
        let alidade_angle = anim_t * 0.6;
        let alidade_steps = (alidade_len * 2.0) as usize;
        let cos_alidade = alidade_angle.cos();
        let sin_alidade = alidade_angle.sin() * (0.88 - params.tilt * 0.35);
        let az = scale * 0.05;
        let tilt_pitch = params.tilt * 0.5;
        let tip_thresh = scale * 0.08;
        let sight_thresh = scale * 0.05;
        let inv_alidade_steps = 2.0 * alidade_len / alidade_steps.max(1) as f32;

        for s in 0..alidade_steps {
            let dist = -alidade_len + s as f32 * inv_alidade_steps;
            let ax = dist * cos_alidade;
            let ay = dist * sin_alidade;

            let (wx, wy, wz) = rotate_3d(ax, ay, az, tilt_pitch, 0.0, 0.0);
            if let Some((px, py, pz)) = project(wx, wy, wz, cx, cy, aspect, fov) {
                let abs_dist = dist.abs();
                let is_tip = (abs_dist - alidade_len).abs() < tip_thresh;
                let is_sight = (abs_dist - scale * 0.7).abs() < sight_thresh;

                let (ch, col) = if is_tip {
                    ('▲', palette[4])
                } else if is_sight {
                    ('⌖', palette[3])
                } else {
                    ('─', palette[2])
                };
                put_z(grid, px, py, pz + 6.0, ch, col);
            }
        }
    });
}

// ----------------------------------------------------------------------------
// Tests & Snapshots
// ----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::make_palette;
    use crate::render::grid_to_plain;

    #[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
    fn render_test_grid(
        width: usize,
        height: usize,
        seed: u64,
        t: f32,
        params: &AetheriumParams,
    ) -> Grid {
        let mut grid = vec![vec![Cell::blank(); width]; height];
        let mut rng = StdRng::seed_from_u64(seed);
        let palette = make_palette(seed);
        draw_gem_aetherium_2(
            &mut grid, width, height, seed, &palette, &mut rng, t, params,
        );
        grid
    }

    #[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
    fn plain(grid: &Grid) -> String {
        grid_to_plain(grid).join("\n")
    }

    #[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
    fn max_params() -> AetheriumParams {
        let values: Vec<_> = PARAMS.iter().map(|param| param.max).collect();
        AetheriumParams::from_inputs(&[], Some(&values))
    }

    #[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
    fn fingerprint(grid: &Grid) -> u64 {
        use std::hash::{DefaultHasher, Hash, Hasher};
        let mut hash = DefaultHasher::new();
        for row in grid {
            for cell in row {
                cell.ch.hash(&mut hash);
                cell.fg.hash(&mut hash);
                cell.bg.hash(&mut hash);
            }
        }
        hash.finish()
    }

    #[test]
    #[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
    fn cached_nebula_matches_reference_noise() {
        use crate::pp::pp_fbm;

        for width in [1, 2, 80, 2000] {
            let columns: Vec<_> = (0..width)
                .map(|x| {
                    let nx = (x as f32 - width as f32 * 0.5) / (width as f32 * 0.5);
                    [nx, nx * nx]
                })
                .collect();
            for (seed, bias) in [(0, 0.0), (1701, -0.375), (u64::MAX, 4.25)] {
                let mut noise = NebulaRows::new(&columns, bias);
                // Repeated, adjacent, skipped and reversed lattice bands.
                for fy in [-2.2, -2.199, -1.0, 0.0, 0.0, 0.125, 0.13, 2.2, -2.2] {
                    let sy = noise.prepare_y(fy, seed);
                    for (column, &[nx, _]) in noise.columns.iter().zip(&columns) {
                        assert_eq!(
                            column.at(&sy).to_bits(),
                            pp_fbm(nx * 2.2 + bias, fy, seed).to_bits(),
                            "width={width} seed={seed} bias={bias} nx={nx} fy={fy}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    #[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
    fn parallel_rows_preserve_seeded_frames() {
        let pools: Vec<_> = [1, 4]
            .into_iter()
            .map(|threads| {
                rayon::ThreadPoolBuilder::new()
                    .num_threads(threads)
                    .build()
                    .unwrap()
            })
            .collect();
        for (seed, time, params) in [
            (42, 0.0, AetheriumParams::default()),
            (u64::MAX, -3.7, max_params()),
        ] {
            // Above the parallel threshold, including a partial final row batch.
            let frames: Vec<_> = pools
                .iter()
                .map(|pool| pool.install(|| render_test_grid(257, 257, seed, time, &params)))
                .collect();
            assert_eq!(frames[0], frames[1]);
        }
    }

    #[test]
    #[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
    fn seeded_full_cell_regression() {
        let mut frames = String::new();
        for (name, params) in [
            ("default", AetheriumParams::default()),
            ("maximum", max_params()),
            (
                "no-nebula",
                AetheriumParams {
                    nebula: 0.0,
                    ..Default::default()
                },
            ),
            (
                "no-rays",
                AetheriumParams {
                    rays: 0.0,
                    ..Default::default()
                },
            ),
            (
                "no-background",
                AetheriumParams {
                    nebula: 0.0,
                    rays: 0.0,
                    ..Default::default()
                },
            ),
        ] {
            for (width, height, seed, time) in [
                (1, 1, 999, 3.7),
                (80, 24, 42, 0.0),
                (80, 24, 42, 1.5),
                (320, 100, 1701, -2.25),
            ] {
                use std::fmt::Write;
                let grid = render_test_grid(width, height, seed, time, &params);
                writeln!(
                    frames,
                    "{name} {width}x{height} seed={seed} t={time}: {:016x}",
                    fingerprint(&grid)
                )
                .unwrap();
            }
        }
        insta::assert_snapshot!(frames, @r"
default 1x1 seed=999 t=3.7: 62574f63114ec1b8
default 80x24 seed=42 t=0: f748016a4d0c3209
default 80x24 seed=42 t=1.5: 830d339dfe4a005b
default 320x100 seed=1701 t=-2.25: 1befbae7e2df6158
maximum 1x1 seed=999 t=3.7: 20e372eb7cab1d13
maximum 80x24 seed=42 t=0: 064f8411ba3f250b
maximum 80x24 seed=42 t=1.5: 4646bf27ca9b0e74
maximum 320x100 seed=1701 t=-2.25: 07a21da96c82c19d
no-nebula 1x1 seed=999 t=3.7: 62574f63114ec1b8
no-nebula 80x24 seed=42 t=0: 48d6598b3b4e3e4f
no-nebula 80x24 seed=42 t=1.5: fb67f298463d35be
no-nebula 320x100 seed=1701 t=-2.25: 060efc8cc21bc807
no-rays 1x1 seed=999 t=3.7: 62574f63114ec1b8
no-rays 80x24 seed=42 t=0: bbc8a63952132e14
no-rays 80x24 seed=42 t=1.5: dcf301aa4d71465d
no-rays 320x100 seed=1701 t=-2.25: 148e2fcda46c3103
no-background 1x1 seed=999 t=3.7: 62574f63114ec1b8
no-background 80x24 seed=42 t=0: bea89baf3ddf5da1
no-background 80x24 seed=42 t=1.5: e67a1a47425640b8
no-background 320x100 seed=1701 t=-2.25: 606ed0d7bfc4323b
        ");
    }

    /// Headless layer probe; grid allocation and output encoding are excluded.
    /// Run with: cargo test --release perf_gem_aetherium_2 -- --ignored --nocapture
    #[test]
    #[ignore]
    #[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
    fn perf_gem_aetherium_2() {
        use crate::_0_profile::{layer_capture_begin, layer_capture_end};
        use std::collections::BTreeMap;
        use std::time::Instant;

        let read = |key, default| {
            std::env::var(key)
                .ok()
                .and_then(|value| value.parse::<usize>().ok())
                .filter(|value| *value > 0)
                .unwrap_or(default)
        };
        let width = read("ASCII_PERF_WIDTH", 2000);
        let height = read("ASCII_PERF_HEIGHT", 1000);
        let frames = read("ASCII_PERF_FRAMES", 12);
        let seed = 1701;
        let palette = make_palette(seed);
        let mut grid = vec![vec![Cell::blank(); width]; height];
        for (name, params) in [
            ("default", AetheriumParams::default()),
            ("maximum", max_params()),
        ] {
            let mut times = Vec::with_capacity(frames);
            let mut layers = BTreeMap::<_, u128>::new();
            for frame in 0..=frames {
                let mut rng = StdRng::seed_from_u64(seed);
                layer_capture_begin();
                let started = Instant::now();
                draw_gem_aetherium_2(
                    &mut grid,
                    width,
                    height,
                    seed,
                    &palette,
                    &mut rng,
                    frame as f32 * 0.06,
                    &params,
                );
                let elapsed = started.elapsed().as_nanos();
                let totals = layer_capture_end();
                std::hint::black_box(&grid);
                if frame == 0 {
                    continue;
                }
                times.push(elapsed);
                for layer in totals {
                    *layers.entry(layer.layer).or_default() += layer.total_ns;
                }
            }
            times.sort_unstable();
            eprintln!(
                "{name} {width}x{height} frames={frames} median_ms={:.3} mean_ms={:.3} checksum={:016x}",
                times[times.len() / 2] as f64 / 1e6,
                times.iter().sum::<u128>() as f64 / frames as f64 / 1e6,
                fingerprint(&grid)
            );
            for (layer, ns) in layers {
                eprintln!("  {layer}: {:.3} ms/frame", ns as f64 / frames as f64 / 1e6);
            }
        }
    }

    #[test]
    #[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
    fn deterministic_seeded_frame_and_visible_motion() {
        let params = AetheriumParams::default();
        let frame_a = render_test_grid(80, 24, 42, 0.0, &params);
        let frame_a2 = render_test_grid(80, 24, 42, 0.0, &params);
        let frame_b = render_test_grid(80, 24, 42, 1.5, &params);

        assert_eq!(
            frame_a, frame_a2,
            "Identical inputs must yield identical frames"
        );
        assert_ne!(
            plain(&frame_a),
            plain(&frame_b),
            "Time progression must cause visible motion in orrery/astrolabe"
        );
    }

    #[test]
    #[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
    fn tiny_grid_and_extreme_inputs_terminate() {
        let tiny_params = max_params();
        for (w, h) in [
            (10usize, 5usize),
            (2, 2),
            (1, 1),
            (30, 6),
            (0, 0),
            (0, 5),
            (5, 0),
            (1, 32),
            (32, 1),
        ] {
            let output = render_test_grid(w, h, 999, 3.7, &tiny_params);
            assert_eq!(output.len(), h);
            assert_eq!(output.iter().map(Vec::len).collect::<Vec<_>>(), vec![w; h]);
        }
    }

    #[test]
    #[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
    fn params_from_args_override_and_clamp() {
        let args = vec![
            "42".to_string(),
            "gem-aetherium-2".to_string(),
            "cathedral".to_string(),
            "".to_string(),
            "8".to_string(),
            "10".to_string(),
            "6".to_string(),
            "18".to_string(),
            "1.5".to_string(),
            "0.8".to_string(),
        ];
        let p = AetheriumParams::from_args(&args);
        assert_eq!(p.rings, 8);
        assert_eq!(p.planets, 10);
        assert_eq!(p.gears, 6);
        assert_eq!(p.zodiac, 18);
        assert!((p.speed - 1.5).abs() < 1e-4);
        assert!((p.tilt - 0.8).abs() < 1e-4);
    }
}
