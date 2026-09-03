use crossterm::style::Color;
use rand::rngs::StdRng;
use rand::RngExt;
use rand::SeedableRng;

use crate::_0_profile::measure_layer;
use crate::color::{darken, lerp_color, lighten, shift_hue};
use crate::opts::param_f32;
use crate::pp::{pp_line, pp_put};
use crate::registry::{AnimKind, Mode, ModeFrame, Param};
use crate::types::{Cell, Grid};
use super::_33_cosmograph::FbmRow;

const TAU: f32 = std::f32::consts::TAU;
const PI: f32 = std::f32::consts::PI;

pub(super) struct GemAetheriumMode;

pub(super) static MODE: GemAetheriumMode = GemAetheriumMode;

const PARAMS: &[Param] = &[
    param!("RINGS", "astrolabe armillary rings", 2.0, 10.0, 6.0, 1.0),
    param!("PLANETS", "orrery celestial bodies", 1.0, 12.0, 7.0, 1.0),
    param!("GEARS", "mechanical epicyclic gears", 0.0, 8.0, 4.0, 1.0),
    param!("ZODIAC", "zodiac constellation points", 4.0, 24.0, 12.0, 1.0),
    param!("SPEED", "celestial mechanics velocity", 0.05, 3.0, 0.75, 0.05),
    param!("TILT", "axial orbital inclination", 0.0, 1.0, 0.45, 0.05),
    param!("NEBULA", "aetherial cosmic dust fog", 0.0, 1.5, 0.7, 0.05),
    param!("RAYS", "mystic chronos ray bursts", 0.0, 1.5, 0.8, 0.05),
    param!("COMETS", "hyperbolic orbital comets", 0.0, 12.0, 4.0, 1.0),
    param!("RUNES", "arcane glyph ring density", 0.0, 1.0, 0.85, 0.05),
    param!("PULSE", "harmonic resonance pulse", 0.0, 2.0, 1.0, 0.1),
    param!("HARMONY", "pythagorean celestial ratios", 1.0, 8.0, 3.0, 1.0),
];

impl Mode for GemAetheriumMode {
    fn name(&self) -> &'static str {
        "gem-aetherium"
    }

    fn help(&self) -> &'static str {
        "Grand Aetherial Orrery, 3D rotating armillary spheres, epicyclic clockwork gears, zodiac astrolabe, orbiting planets & comets [rings] [planets] [gears] [zodiac] [speed] [tilt] [nebula] [rays] [comets] [runes] [pulse] [harmony]"
    }

    fn animation(&self) -> AnimKind {
        AnimKind::Iterate
    }

    fn params(&self) -> &'static [Param] {
        PARAMS
    }

    fn render(&self, frame: &mut ModeFrame<'_>) {
        let params = AetheriumParams::from_inputs(frame.args, frame.param_values);
        draw_gem_aetherium(
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
    pub(crate) fn from_args(args: &[String]) -> Self {
        Self::from_inputs(args, None)
    }

    pub(crate) fn from_inputs(args: &[String], param_values: Option<&[f32]>) -> Self {
        let read = |index: usize, key: &str, default: f32| {
            args.get(index)
                .and_then(|value| value.parse::<f32>().ok())
                .or_else(|| param_values.and_then(|values| values.get(index - 4)).copied())
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

// ----------------------------------------------------------------------------
// Procedural Generation & Rendering
// ----------------------------------------------------------------------------

#[derive(Clone, Copy)]
struct PlanetDef {
    orbit_r: f32,
    speed_mult: f32,
    inclination: f32,
    node_angle: f32,
    phase_offset: f32,
    size: f32,
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
    '♈', '♉', '♊', '♋', '♌', '♍', '♎', '♏', '♐', '♑', '♒', '♓',
    '☉', '☽', '☿', '♀', '♂', '♃', '♄', '♅', '♆', '♇', '✧', '✦',
];

const RUNIC_CHARS: &[char] = &[
    'ᚠ', 'ᚢ', 'ᚦ', 'ᚨ', 'ᚱ', 'ᚲ', 'ᚷ', 'ᚹ', 'ᚺ', 'ᚾ', 'ᛁ', 'ᛃ', 'ᛈ', 'ᛉ', 'ᛋ', 'ᛏ', 'ᛒ', 'ᛖ', 'ᛗ', 'ᛚ', 'ᛜ', 'ᛞ', 'ᛟ',
    'α', 'β', 'γ', 'δ', 'ε', 'ζ', 'η', 'θ', 'ι', 'κ', 'λ', 'μ', 'ν', 'ξ', 'ο', 'π', 'ρ', 'σ', 'τ', 'υ', 'φ', 'χ', 'ψ', 'ω',
];

fn rotate_3d(x: f32, y: f32, z: f32, pitch: f32, yaw: f32, roll: f32) -> (f32, f32, f32) {
    // Yaw (around Y)
    let cy = yaw.cos();
    let sy = yaw.sin();
    let x1 = x * cy + z * sy;
    let y1 = y;
    let z1 = -x * sy + z * cy;

    // Pitch (around X)
    let cp = pitch.cos();
    let sp = pitch.sin();
    let x2 = x1;
    let y2 = y1 * cp - z1 * sp;
    let z2 = y1 * sp + z1 * cp;

    // Roll (around Z)
    let cr = roll.cos();
    let sr = roll.sin();
    let x3 = x2 * cr - y2 * sr;
    let y3 = x2 * sr + y2 * cr;
    let z3 = z2;

    (x3, y3, z3)
}

fn project(x: f32, y: f32, z: f32, cx: f32, cy: f32, aspect: f32, fov: f32) -> Option<(i32, i32, f32)> {
    let dist = fov + z;
    if dist <= 0.1 {
        return None;
    }
    let factor = fov / dist;
    let px = cx + x * factor * aspect;
    let py = cy + y * factor;
    Some((px.round() as i32, py.round() as i32, z))
}

pub(crate) fn draw_gem_aetherium(
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

    let mut z_buf = vec![f32::NEG_INFINITY; width * height];
    let cx = (width as f32) * 0.5;
    let cy = (height as f32) * 0.5;
    let min_dim = (width as f32).min(height as f32 * 2.0);
    let scale = min_dim * 0.44;
    let aspect = 2.05; // terminal character aspect ratio correction (x is stretched)
    let fov = scale * 2.8;

    let anim_t = t * params.speed;
    let pulse_val = 1.0 + (anim_t * 2.0 * params.pulse).sin() * 0.08 * params.pulse;

    // ------------------------------------------------------------------------
    // 1. Cosmic Background Nebula, Starfield & Sacred Radiance
    // ------------------------------------------------------------------------
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

    measure_layer("gem-aetherium", "background", || {
    // Column-invariant terms: the fbm x input and the nx powers never change down
    // a column, so they are tabulated once per frame.
    let nebula_on = params.nebula > 0.05;
    let rays_on = params.rays > 0.05;
    let neb_seed = seed.wrapping_add(999);
    let neb_fx_bias = anim_t * 0.03;
    let neb_fy_bias = anim_t * 0.02;
    let neb_dim = darken(aether_color, 25);
    let ray_spin = anim_t * 0.15;
    let ray_k = params.zodiac as f32 * 0.5;
    let mut nx_col = Vec::with_capacity(width);
    let mut nx2_col = Vec::with_capacity(width);
    let mut neb_fx_col = Vec::with_capacity(width);
    for x in 0..width {
        let nx = (x as f32 - cx) / (width as f32 * 0.5);
        nx_col.push(nx);
        nx2_col.push(nx * nx);
        neb_fx_col.push(nx * 2.2 + neb_fx_bias);
    }
    for y in 0..height {
        let ny = (y as f32 - cy) / (height as f32 * 0.5);
        let ny2 = ny * ny;
        let mut neb_noise = FbmRow::new(ny * 2.2 - neb_fy_bias, neb_seed);
        let row = &mut grid[y];
        z_buf[y * width..y * width + width].fill(-1000.0);
        for x in 0..width {
            let dist_center = (nx2_col[x] + ny2).sqrt();

            let mut cell_ch = ' ';
            let mut cell_fg = bg_color;

            if nebula_on {
                let fbm_val = neb_noise.at(neb_fx_col[x]);
                let neb_intensity = (fbm_val * 0.8 + (1.0 - dist_center * 0.85)).clamp(0.0, 1.0) * params.nebula;
                if neb_intensity > 0.65 {
                    cell_ch = match ((neb_intensity - 0.65) * 8.0) as usize {
                        0 => '░',
                        1 => '▒',
                        2 => '▓',
                        _ => '▒',
                    };
                    cell_fg = lerp_color(bg_color, aether_color, neb_intensity.min(1.0));
                } else if neb_intensity > 0.45 {
                    cell_ch = ' ';
                    cell_fg = neb_dim;
                }
            }

            // The quartic wave term tops out at 1, so attenuation under 0.4 can
            // never clear the gate and the atan2 plus sin pair is skipped.
            if rays_on && dist_center < 1.4 {
                let atten = 1.0 / (dist_center * 1.5 + 0.3);
                if atten * params.rays > 0.4 {
                    let angle = ny.atan2(nx_col[x]) + ray_spin;
                    let ray_wave = (angle * ray_k).sin();
                    if ray_wave > 0.0 {
                        let ray_power = ray_wave.powf(4.0) * atten * params.rays;
                        if ray_power > 0.4 {
                            let r_ch = if ray_power > 1.2 { '│' } else if ray_power > 0.8 { '┆' } else { '┊' };
                            let r_fg = lerp_color(cell_fg, palette[4], (ray_power * 0.4).min(0.9));
                            cell_ch = r_ch;
                            cell_fg = r_fg;
                        }
                    }
                }
            }

            row[x] = Cell::new(cell_ch, cell_fg);
        }
    }
    });

    measure_layer("gem-aetherium", "starfield", || {
    for (sx, sy, b_spd, b_phs, sch) in stars {
        let blink = ((anim_t * b_spd + b_phs).sin() + 1.0) * 0.5;
        let col = lerp_color(star_dim, star_bright, blink);
        if grid[sy][sx].ch == ' ' {
            grid[sy][sx] = Cell::new(sch, col);
        }
    }
    });

    // ------------------------------------------------------------------------
    // Depth Buffer Plotting Helper
    // ------------------------------------------------------------------------
    let mut put_z = |grid: &mut Grid, x: i32, y: i32, z: f32, ch: char, fg: Color| {
        if x >= 0 && y >= 0 && (x as usize) < width && (y as usize) < height {
            let idx = (y as usize) * width + (x as usize);
            if z > z_buf[idx] {
                z_buf[idx] = z;
                grid[y as usize][x as usize] = Cell::new(ch, fg);
            }
        }
    };

    // ------------------------------------------------------------------------
    // 2. Epicyclic Clockwork Gears (Mechanical Foundation)
    // ------------------------------------------------------------------------
    measure_layer("gem-aetherium", "gears", || {
    if params.gears > 0 {
        let mut gear_rng = StdRng::seed_from_u64(seed.wrapping_add(202));
        let num_gears = params.gears;
        let mut gear_defs = Vec::with_capacity(num_gears);

        for g in 0..num_gears {
            let gr = scale * (0.2 + (g as f32) * 0.18);
            let g_teeth = (12 + g * 8) * (params.harmony as usize).max(1);
            let c_dist = if g == 0 { 0.0 } else { scale * (0.35 + (g as f32) * 0.15) };
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

        for gear in &gear_defs {
            let g_center_x = gear.center_r * gear.center_angle.cos();
            let g_center_y = gear.center_r * gear.center_angle.sin() * 0.5; // flat on mechanical plane
            let g_rot = anim_t * gear.speed_ratio;

            let step_count = gear.teeth * 4;
            for s in 0..step_count {
                let theta = (s as f32 / step_count as f32) * TAU;
                let tooth_wave = ((theta + g_rot) * gear.teeth as f32).sin();
                let tooth_r = gear.radius + tooth_wave * (scale * 0.025);
                let gx = g_center_x + tooth_r * (theta + g_rot).cos();
                let gy = g_center_y + tooth_r * (theta + g_rot).sin();
                let gz = -scale * 0.6; // placed behind orrery core

                let (rx, ry, rz) = rotate_3d(gx, gy, gz, params.tilt * 0.6, anim_t * 0.1, 0.0);
                if let Some((px, py, pz)) = project(rx, ry, rz, cx, cy, aspect, fov) {
                    let ch = if tooth_wave > 0.4 { '⚙' } else if s % 2 == 0 { '▪' } else { '▫' };
                    let col = if tooth_wave > 0.6 { gear_highlight } else { gear_color };
                    put_z(grid, px, py, pz, ch, col);
                }
            }

            // Gear Spokes
            for sp in 0..gear.spokes {
                let sp_angle = g_rot + (sp as f32 / gear.spokes as f32) * TAU;
                let sp_steps = 8;
                for step in 1..=sp_steps {
                    let d = (step as f32 / sp_steps as f32) * gear.radius;
                    let gx = g_center_x + d * sp_angle.cos();
                    let gy = g_center_y + d * sp_angle.sin();
                    let gz = -scale * 0.6;
                    let (rx, ry, rz) = rotate_3d(gx, gy, gz, params.tilt * 0.6, anim_t * 0.1, 0.0);
                    if let Some((px, py, pz)) = project(rx, ry, rz, cx, cy, aspect, fov) {
                        put_z(grid, px, py, pz, '─', darken(gear_color, 20));
                    }
                }
            }
        }
    }
    });

    // ------------------------------------------------------------------------
    // 3. Nested 3D Armillary Spheres (Gimbaled Celestial Rings)
    // ------------------------------------------------------------------------
    measure_layer("gem-aetherium", "armillary", || {
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

        for i in 0..num_samples {
            let phi = (i as f32 / num_samples as f32) * TAU;
            // Torus / ring point in ring's local plane (XY)
            let lx = ring_radius * phi.cos();
            let ly = ring_radius * phi.sin();
            let lz = 0.0;

            let (wx, wy, wz) = rotate_3d(lx, ly, lz, cur_pitch, cur_yaw, base_roll);
            if let Some((px, py, pz)) = project(wx, wy, wz, cx, cy, aspect, fov) {
                // Graduated tick marks and runes along the armillary ring
                let is_major_tick = (i % (num_samples / 12).max(1)) == 0;
                let is_minor_tick = (i % (num_samples / 48).max(1)) == 0;

                let (ch, col) = if is_major_tick && params.runes > 0.2 {
                    let rune_idx = (r * 7 + i / (num_samples / 12).max(1)) % RUNIC_CHARS.len();
                    (RUNIC_CHARS[rune_idx], lighten(ring_color, 24))
                } else if is_minor_tick {
                    ('┼', ring_color)
                } else {
                    let stroke_ch = if (phi.sin().abs() > 0.707) { '│' } else { '─' };
                    (stroke_ch, darken(ring_color, 12))
                };

                put_z(grid, px, py, pz, ch, col);
            }
        }
    }
    });

    // ------------------------------------------------------------------------
    // 4. Outer Astrolabe Limb & Zodiac Horizon Ring
    // ------------------------------------------------------------------------
    measure_layer("gem-aetherium", "limb", || {
    let limb_radius = scale * 1.08 * pulse_val;
    let zodiac_count = params.zodiac;
    let zodiac_col = palette[3];
    let zodiac_speed = anim_t * 0.12;

    // Limb outer graduated border
    let limb_samples = (limb_radius * 14.0).clamp(120.0, 360.0) as usize;
    for i in 0..limb_samples {
        let theta = (i as f32 / limb_samples as f32) * TAU;
        let lx = limb_radius * theta.cos();
        let ly = limb_radius * theta.sin() * (0.88 - params.tilt * 0.35); // tilted astrolabe plate
        let lz = -scale * 0.1;

        let (wx, wy, wz) = rotate_3d(lx, ly, lz, params.tilt * 0.5, 0.0, 0.0);
        if let Some((px, py, pz)) = project(wx, wy, wz, cx, cy, aspect, fov) {
            let is_cardinal = (i % (limb_samples / 4).max(1)) == 0;
            let ch = if is_cardinal { '❖' } else if i % 2 == 0 { '═' } else { '─' };
            put_z(grid, px, py, pz, ch, palette[4]);
        }
    }

    // Zodiac Constellation Houses
    for z in 0..zodiac_count {
        let z_angle = zodiac_speed + (z as f32 / zodiac_count as f32) * TAU;
        let zx = limb_radius * 0.95 * z_angle.cos();
        let zy = limb_radius * 0.95 * z_angle.sin() * (0.88 - params.tilt * 0.35);
        let zz = -scale * 0.05;

        let (wx, wy, wz) = rotate_3d(zx, zy, zz, params.tilt * 0.5, 0.0, 0.0);
        if let Some((px, py, pz)) = project(wx, wy, wz, cx, cy, aspect, fov) {
            let symbol = ZODIAC_SYMBOLS[z % ZODIAC_SYMBOLS.len()];
            put_z(grid, px, py, pz, symbol, lighten(zodiac_col, 20));

            // Radiating spoke from zodiac house to center
            let spoke_steps = 14;
            for s in 1..spoke_steps {
                let frac = s as f32 / spoke_steps as f32;
                let sx = zx * (1.0 - frac);
                let sy = zy * (1.0 - frac);
                let sz = zz;
                let (swx, swy, swz) = rotate_3d(sx, sy, sz, params.tilt * 0.5, 0.0, 0.0);
                if let Some((spx, spy, spz)) = project(swx, swy, swz, cx, cy, aspect, fov) {
                    let ch = if s % 3 == 0 { '·' } else { '┄' };
                    put_z(grid, spx, spy, spz, ch, darken(palette[2], 30));
                }
            }
        }
    }
    });

    // ------------------------------------------------------------------------
    // 5. Grand Orrery Planetary Orbits & Moons
    // ------------------------------------------------------------------------
    measure_layer("gem-aetherium", "orrery", || {
    let mut orrery_rng = StdRng::seed_from_u64(seed.wrapping_add(404));
    let num_planets = params.planets;
    let planet_glyphs = ['☉', '☿', '♀', '♁', '♂', '♃', '♄', '♅', '♆', '♇', '✧', '◈'];
    let mut planet_defs = Vec::with_capacity(num_planets);

    for p in 0..num_planets {
        let p_frac = (p + 1) as f32 / (num_planets + 1) as f32;
        let orb_r = scale * (0.22 + p_frac * 0.78);
        // Keplerian harmonic speeds: T^2 = a^3 => speed ~ 1/sqrt(r^3)
        let speed_m = 1.6 / (p_frac * 2.5 + 0.4).sqrt() * if p % 3 == 1 { 1.0 } else { 1.15 };
        let inc = orrery_rng.random_range(0.05..0.6) * params.tilt;
        let node_ang = orrery_rng.random_range(0.0..TAU);
        let phs = orrery_rng.random_range(0.0..TAU);
        let sz = orrery_rng.random_range(0.8..1.8);
        let glyph = planet_glyphs[p % planet_glyphs.len()];
        let has_ring = p == 5 || (p > 2 && orrery_rng.random_bool(0.3));
        let moons = if p > 3 { orrery_rng.random_range(1..=3) } else { 0 };

        planet_defs.push(PlanetDef {
            orbit_r: orb_r,
            speed_mult: speed_m,
            inclination: inc,
            node_angle: node_ang,
            phase_offset: phs,
            size: sz,
            glyph,
            has_ring,
            color_idx: (p % 4) + 1,
            moons,
        });
    }

    for planet in &planet_defs {
        let p_col = palette[planet.color_idx];

        // Draw dotted orbital trace
        let trace_steps = (planet.orbit_r * 8.0).clamp(40.0, 180.0) as usize;
        for ts in 0..trace_steps {
            let trace_ang = (ts as f32 / trace_steps as f32) * TAU;
            let ox = planet.orbit_r * trace_ang.cos();
            let oy = planet.orbit_r * trace_ang.sin();
            let oz = 0.0;
            let (tx, ty, tz) = rotate_3d(ox, oy, oz, planet.inclination, planet.node_angle + anim_t * 0.05, 0.0);
            if let Some((px, py, pz)) = project(tx, ty, tz, cx, cy, aspect, fov) {
                if ts % 3 == 0 {
                    put_z(grid, px, py, pz, '·', darken(p_col, 36));
                }
            }
        }

        // Planet Current Position
        let cur_phi = planet.phase_offset + anim_t * planet.speed_mult;
        let px0 = planet.orbit_r * cur_phi.cos();
        let py0 = planet.orbit_r * cur_phi.sin();
        let pz0 = 0.0;
        let (pw_x, pw_y, pw_z) = rotate_3d(px0, py0, pz0, planet.inclination, planet.node_angle + anim_t * 0.05, 0.0);

        if let Some((px, py, pz)) = project(pw_x, pw_y, pw_z, cx, cy, aspect, fov) {
            // Planet Body
            put_z(grid, px, py, pz + 2.0, planet.glyph, lighten(p_col, 30));

            // Planetary Rings (like Saturn)
            if planet.has_ring {
                for ra in 0..12 {
                    let r_angle = (ra as f32 / 12.0) * TAU;
                    let rx = scale * 0.06 * r_angle.cos();
                    let ry = scale * 0.02 * r_angle.sin();
                    let (rw_x, rw_y, rw_z) = rotate_3d(rx, ry, 0.0, 0.8, anim_t * 0.5, 0.0);
                    if let Some((rpx, rpy, rpz)) = project(pw_x + rw_x, pw_y + rw_y, pw_z + rw_z, cx, cy, aspect, fov) {
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
                if let Some((mpx, mpy, mpz)) = project(pw_x + mw_x, pw_y + mw_y, pw_z + mw_z, cx, cy, aspect, fov) {
                    put_z(grid, mpx, mpy, mpz + 2.5, '∘', lighten(palette[4], 16));
                }
            }
        }
    }
    });

    // ------------------------------------------------------------------------
    // 6. Hyperbolic Orbital Comets with Glowing Ion Tails
    // ------------------------------------------------------------------------
    measure_layer("gem-aetherium", "comets", || {
    if params.comets > 0 {
        let mut comet_rng = StdRng::seed_from_u64(seed.wrapping_add(505));
        let num_comets = params.comets;
        let mut comet_defs = Vec::with_capacity(num_comets);

        for c in 0..num_comets {
            let a = scale * (0.7 + (c as f32) * 0.3);
            let e = comet_rng.random_range(0.72..0.92); // high eccentricity ellipse
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
            // Solve Kepler's equation approx: E ≈ M + e sin(M)
            let ecc_anomaly = mean_anomaly + comet.eccentricity * mean_anomaly.sin();
            let true_anomaly = 2.0 * (((1.0 + comet.eccentricity).sqrt() * (ecc_anomaly * 0.5).sin())
                .atan2((1.0 - comet.eccentricity).sqrt() * (ecc_anomaly * 0.5).cos()));
            let r = comet.semi_major * (1.0 - comet.eccentricity * ecc_anomaly.cos());

            let head_x = r * true_anomaly.cos();
            let head_y = r * true_anomaly.sin();
            let (hw_x, hw_y, hw_z) = rotate_3d(head_x, head_y, 0.0, comet.inclination, comet.node_angle, 0.0);

            // Comet Head
            if let Some((px, py, pz)) = project(hw_x, hw_y, hw_z, cx, cy, aspect, fov) {
                put_z(grid, px, py, pz + 4.0, '✷', palette[4]);

                // Comet Tail (lagging behind orbital path)
                for tail_step in 1..=comet.tail_len {
                    let lag = tail_step as f32 * 0.035;
                    let t_mean = (mean_anomaly - lag + TAU) % TAU;
                    let t_ecc = t_mean + comet.eccentricity * t_mean.sin();
                    let t_true = 2.0 * (((1.0 + comet.eccentricity).sqrt() * (t_ecc * 0.5).sin())
                        .atan2((1.0 - comet.eccentricity).sqrt() * (t_ecc * 0.5).cos()));
                    let tr = comet.semi_major * (1.0 - comet.eccentricity * t_ecc.cos());

                    let tx = tr * t_true.cos();
                    let ty = tr * t_true.sin();
                    let (tw_x, tw_y, tw_z) = rotate_3d(tx, ty, 0.0, comet.inclination, comet.node_angle, 0.0);

                    if let Some((tpx, tpy, tpz)) = project(tw_x, tw_y, tw_z, cx, cy, aspect, fov) {
                        let tail_frac = tail_step as f32 / comet.tail_len as f32;
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
    measure_layer("gem-aetherium", "core", || {
    // Central Radiant Sun / Chronos Singularity
    let core_radius = scale * 0.12 * pulse_val;
    for dy in -3..=3 {
        for dx in -5..=5 {
            let dist_sq = (dx as f32 * 0.5).powi(2) + (dy as f32).powi(2);
            if dist_sq <= core_radius.powi(2) {
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
                    lighten(palette[4], 35)
                } else {
                    palette[3]
                };
                put_z(grid, px, py, 10.0, ch, col);
            }
        }
    }

    // Rotating Astrolabe Sighting Alidade (Double-ended pointer)
    let alidade_len = scale * 1.15 * pulse_val;
    let alidade_angle = anim_t * 0.6;
    let alidade_steps = (alidade_len * 2.0) as usize;

    for s in 0..alidade_steps {
        let dist = -alidade_len + (s as f32 / alidade_steps as f32) * (alidade_len * 2.0);
        let ax = dist * alidade_angle.cos();
        let ay = dist * alidade_angle.sin() * (0.88 - params.tilt * 0.35);
        let az = scale * 0.05;

        let (wx, wy, wz) = rotate_3d(ax, ay, az, params.tilt * 0.5, 0.0, 0.0);
        if let Some((px, py, pz)) = project(wx, wy, wz, cx, cy, aspect, fov) {
            let is_tip = (dist.abs() - alidade_len).abs() < (scale * 0.08);
            let is_sight = (dist.abs() - scale * 0.7).abs() < (scale * 0.05);

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

    fn render_test_grid(width: usize, height: usize, seed: u64, t: f32, params: &AetheriumParams) -> Grid {
        let mut grid = vec![vec![Cell::blank(); width]; height];
        let mut rng = StdRng::seed_from_u64(seed);
        let palette = make_palette(seed);
        draw_gem_aetherium(&mut grid, width, height, seed, &palette, &mut rng, t, params);
        grid
    }

    fn plain(grid: &Grid) -> String {
        grid_to_plain(grid).join("\n")
    }

    #[test]
    fn deterministic_seeded_frame_and_visible_motion() {
        let params = AetheriumParams::default();
        let frame_a = plain(&render_test_grid(80, 24, 42, 0.0, &params));
        let frame_a2 = plain(&render_test_grid(80, 24, 42, 0.0, &params));
        let frame_b = plain(&render_test_grid(80, 24, 42, 1.5, &params));

        assert_eq!(frame_a, frame_a2, "Identical inputs must yield identical frames");
        assert_ne!(frame_a, frame_b, "Time progression must cause visible motion in orrery/astrolabe");
    }

    #[test]
    fn tiny_grid_and_extreme_inputs_terminate() {
        let tiny_params = AetheriumParams {
            rings: 10,
            planets: 12,
            gears: 8,
            zodiac: 24,
            speed: 3.0,
            tilt: 1.0,
            nebula: 1.5,
            rays: 1.5,
            comets: 12,
            runes: 1.0,
            pulse: 2.0,
            harmony: 8.0,
        };
        for (w, h) in [(10usize, 5usize), (2, 2), (1, 1), (30, 6)] {
            let output = render_test_grid(w, h, 999, 3.7, &tiny_params);
            assert_eq!(output.len(), h);
            assert_eq!(output.iter().map(Vec::len).collect::<Vec<_>>(), vec![w; h]);
        }
    }

    #[test]
    fn params_from_args_override_and_clamp() {
        let args = vec![
            "42".to_string(),
            "gem-aetherium".to_string(),
            "cathedral".to_string(),
            "".to_string(),
            "8".to_string(),   // rings
            "10".to_string(),  // planets
            "6".to_string(),   // gears
            "18".to_string(),  // zodiac
            "1.5".to_string(), // speed
            "0.8".to_string(), // tilt
        ];
        let p = AetheriumParams::from_args(&args);
        assert_eq!(p.rings, 8);
        assert_eq!(p.planets, 10);
        assert_eq!(p.gears, 6);
        assert_eq!(p.zodiac, 18);
        assert!((p.speed - 1.5).abs() < 1e-4);
        assert!((p.tilt - 0.8).abs() < 1e-4);
    }

    #[test]
    fn snapshot_gem_aetherium_t0() {
        let params = AetheriumParams::default();
        let out = plain(&render_test_grid(80, 24, 42, 0.0, &params));
        insta::assert_snapshot!("gem_aetherium_t0", out);
    }

    #[test]
    fn snapshot_gem_aetherium_in_motion() {
        let params = AetheriumParams::default();
        let out = plain(&render_test_grid(80, 24, 42, 1.25, &params));
        insta::assert_snapshot!("gem_aetherium_motion", out);
    }
}
