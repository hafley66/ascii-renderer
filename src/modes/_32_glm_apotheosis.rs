use crossterm::style::Color;
use rand::rngs::StdRng;
use rand::RngExt;

use crate::_0_profile::measure_layer;
use crate::color::{darken, lerp_color, lighten, shift_hue};
use crate::opts::param_f32;
use crate::pp::{pp_fbm, pp_line};
use crate::registry::{AnimKind, Mode, ModeFrame, Param};
use crate::types::{Cell, Grid};
use super::_33_cosmograph::FbmRow;

const TAU: f32 = std::f32::consts::TAU;

pub(super) struct GlmApotheosisMode;

pub(super) static MODE: GlmApotheosisMode = GlmApotheosisMode;

const PARAMS: &[Param] = &[
    param!("RINGS", "halo rings", 2.0, 9.0, 5.0, 1.0),
    param!("SPOKES", "mandala spokes", 4.0, 24.0, 12.0, 1.0),
    param!("MOTES", "orbiting motes", 0.0, 80.0, 30.0, 2.0),
    param!("WINGS", "wings", 0.0, 2.0, 2.0, 1.0),
    param!("RAYS", "light rays", 0.0, 1.0, 0.7, 0.05),
    param!("GLOW", "glow", 0.0, 1.5, 0.8, 0.05),
    param!("CLOUDS", "cloud banks", 0.0, 4.0, 3.0, 1.0),
    param!("SPARKS", "ascending sparks", 0.0, 60.0, 22.0, 2.0),
    param!("RUNES", "glyph ring", 0.0, 1.0, 1.0, 0.05),
    param!("BOB", "ascension drift", 0.0, 3.0, 1.0, 0.1),
    param!("DENS", "star density", 0.0, 1.0, 0.5, 0.05),
    param!("SPEED", "motion", 0.05, 3.0, 0.8, 0.05),
];

impl Mode for GlmApotheosisMode {
    fn name(&self) -> &'static str {
        "glm-apotheosis"
    }

    fn help(&self) -> &'static str {
        "Ascending figure, colossal rotating halo mandala, rays, motes, clouds [rings] [spokes] [motes] [wings] [rays] [glow] [clouds] [sparks] [runes] [bob] [stars] [speed]"
    }

    fn animation(&self) -> AnimKind {
        AnimKind::Iterate
    }

    fn params(&self) -> &'static [Param] {
        PARAMS
    }

    fn render(&self, frame: &mut ModeFrame<'_>) {
        let params = ApotheosisParams::from_inputs(frame.args, frame.param_values);
        draw_glm_apotheosis(
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
pub(crate) struct ApotheosisParams {
    pub(crate) rings: usize,
    pub(crate) spokes: usize,
    pub(crate) motes: usize,
    pub(crate) wings: usize,
    pub(crate) rays: f32,
    pub(crate) glow: f32,
    pub(crate) clouds: usize,
    pub(crate) sparks: usize,
    pub(crate) runes: f32,
    pub(crate) bob: f32,
    pub(crate) dens: f32,
    pub(crate) speed: f32,
}

impl Default for ApotheosisParams {
    fn default() -> Self {
        Self {
            rings: 5,
            spokes: 12,
            motes: 30,
            wings: 2,
            rays: 0.7,
            glow: 0.8,
            clouds: 3,
            sparks: 22,
            runes: 1.0,
            bob: 1.0,
            dens: 0.5,
            speed: 0.8,
        }
    }
}

impl ApotheosisParams {
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
            rings: read(4, "RINGS", 5.0).round().clamp(2.0, 9.0) as usize,
            spokes: read(5, "SPOKES", 12.0).round().clamp(4.0, 24.0) as usize,
            motes: read(6, "MOTES", 30.0).round().clamp(0.0, 80.0) as usize,
            wings: read(7, "WINGS", 2.0).round().clamp(0.0, 2.0) as usize,
            rays: read(8, "RAYS", 0.7).clamp(0.0, 1.0),
            glow: read(9, "GLOW", 0.8).clamp(0.0, 1.5),
            clouds: read(10, "CLOUDS", 3.0).round().clamp(0.0, 4.0) as usize,
            sparks: read(11, "SPARKS", 22.0).round().clamp(0.0, 60.0) as usize,
            runes: read(12, "RUNES", 1.0).clamp(0.0, 1.0),
            bob: read(13, "BOB", 1.0).clamp(0.0, 3.0),
            dens: read(14, "DENS", 0.5).clamp(0.0, 1.0),
            speed: read(15, "SPEED", 0.8).clamp(0.05, 3.0),
        }
    }
}

fn clampi(v: i32, lo: i32, hi: i32) -> i32 {
    v.max(lo).min(hi.max(lo))
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

fn in_bounds(grid: &Grid, x: i32, y: i32) -> bool {
    x >= 0 && y >= 0 && (y as usize) < grid.len() && (x as usize) < grid[y as usize].len()
}

fn put(grid: &mut Grid, x: i32, y: i32, ch: char, fg: Color) {
    if in_bounds(grid, x, y) {
        let bg = grid[y as usize][x as usize].bg;
        grid[y as usize][x as usize] = Cell::with_bg(ch, fg, bg);
    }
}

fn put_bg(grid: &mut Grid, x: i32, y: i32, ch: char, fg: Color, bg: Color) {
    if in_bounds(grid, x, y) {
        grid[y as usize][x as usize] = Cell::with_bg(ch, fg, bg);
    }
}

#[derive(Clone, Copy)]
struct Plan {
    cx: i32,
    halo_cy: i32,
    halo_rx: f32,
    halo_ry: f32,
    fig_y: i32,
    cloud_top: i32,
    floor_y: i32,
}

fn compute_plan(width: usize, height: usize) -> Plan {
    let w = width as i32;
    let h = height as i32;
    let cx = w / 2;
    let halo_cy = clampi((h as f32 * 0.34).round() as i32, 2, h - 4);
    let span_hi = h.saturating_sub(6).max(1) as f32;
    let span = ((h as f32) * 0.30).clamp(1.0, span_hi);
    let halo_rx = ((w as f32 * 0.30).min(span * 1.9)).clamp(3.0, 30.0);
    let halo_ry = (span * 0.62).clamp(1.6, 12.0);
    let fig_y = clampi(halo_cy + (span * 0.55).round() as i32, halo_cy + 1, h - 4);
    let cloud_top = clampi(
        (h as f32 * 0.76).round() as i32,
        (fig_y + 4).min(h - 2),
        h - 1,
    );
    let floor_y = h - 2;
    Plan {
        cx,
        halo_cy,
        halo_rx,
        halo_ry,
        fig_y,
        cloud_top,
        floor_y,
    }
}

fn draw_sky(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    params: &ApotheosisParams,
) {
    let plan = compute_plan(width, height);
    let zenith = darken(shift_hue(palette[0], 250.0), 30);
    let mid = darken(palette[0], 18);
    let apex_glow = lighten(palette[3], 10);
    let span = height.saturating_sub(1).max(1) as f32;
    let denom_x = (plan.halo_rx * 2.2).max(6.0);
    let denom_y = (plan.halo_ry * 2.4).max(5.0);
    let warm_gain = 0.30 + params.glow * 0.42;
    let mut dx2_col = Vec::with_capacity(width);
    // dx is exactly antisymmetric about plan.cx, so dx2 and the falloff term it
    // feeds repeat bit-for-bit at the mirrored column.
    let mut mirror_col = Vec::with_capacity(width);
    for x in 0..width {
        let dx = (x as f32 - plan.cx as f32) / denom_x;
        dx2_col.push(dx * dx);
        let m = 2 * plan.cx - x as i32;
        mirror_col.push(if m >= 0 && (m as usize) < x { m as usize } else { usize::MAX });
    }
    let mut falloff = vec![0.0f32; width];
    for y in 0..height {
        let f = y as f32 / span;
        let base = lerp_color(zenith, mid, f.powf(0.8));
        let dy = (y as f32 - plan.halo_cy as f32) / denom_y;
        let dy2 = dy * dy;
        let row = &mut grid[y];
        for x in 0..width {
            let mirror = mirror_col[x];
            let fall = if mirror != usize::MAX {
                falloff[mirror]
            } else {
                let d = (dx2_col[x] + dy2).sqrt();
                (1.0 - d.clamp(0.0, 1.0)).powf(2.2)
            };
            falloff[x] = fall;
            let bg = lerp_color(base, apex_glow, (fall * warm_gain).clamp(0.0, 1.0));
            row[x] = Cell::with_bg(' ', bg, bg);
        }
    }
    let star = lighten(palette[4], 8);
    let threshold = 0.86 - params.dens * 0.16;
    let star_seed = seed ^ 0xA0F;
    let mut field_fx = Vec::with_capacity(width);
    for x in 0..width {
        field_fx.push(x as f32 * 0.09);
    }
    for y in 0..height {
        let row_tag = y as u64 * 977;
        let mut field_row = FbmRow::new(y as f32 * 0.15, star_seed);
        for x in 0..width {
            // `pick` is one hash against a fixed cut; only survivors pay for the fbm.
            let pick = hash01(seed, x as u64 * 131 + row_tag);
            if pick <= threshold {
                continue;
            }
            let field = field_row.at(field_fx[x]);
            if field > 0.70 {
                let tw = (t * params.speed * 0.9 + pick * TAU * 2.0).sin();
                let ch = if tw > 0.82 {
                    '✦'
                } else if tw > 0.1 {
                    '∙'
                } else {
                    '·'
                };
                let col = if tw > 0.6 { star } else { darken(star, 32) };
                put(grid, x as i32, y as i32, ch, col);
            }
        }
    }
}

const GLYPHS: [char; 16] = ['ᚠ', 'ᚢ', 'ᚦ', 'ᚨ', 'ᚱ', 'ᚲ', 'ᚷ', 'ᚹ', 'ᚺ', 'ᚾ', 'ᛁ', 'ᛃ', 'ᛇ', 'ᛈ', 'ᛉ', 'ᛋ'];

fn draw_mandala(
    grid: &mut Grid,
    plan: &Plan,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    params: &ApotheosisParams,
) {
    let gold = lighten(palette[3], 20);
    let stone = darken(palette[1], 22);
    let rx = plan.halo_rx;
    let ry = plan.halo_ry;
    for r in (1..=params.rings).rev() {
        let frac = r as f32 / params.rings.max(1) as f32;
        let rrx = rx * (0.30 + 0.70 * frac);
        let rry = ry * (0.30 + 0.70 * frac);
        let dir = if r.rem_euclid(2) == 0 { 1.0 } else { -1.0 };
        let rot = t * params.speed * dir * (0.10 + frac * 0.22);
        let ring_col = if frac > 0.85 { gold } else { stone };
        let samples = (rrx * 6.0).max(16.0) as usize;
        for i in 0..=samples {
            let a = i as f32 / samples as f32 * TAU;
            let x = plan.cx as f32 + (a + rot).cos() * rrx;
            let y = plan.halo_cy as f32 + a.sin() * rry;
            if (i * 7 + r) % 11 == 0 {
                continue;
            }
            put(grid, x.round() as i32, y.round() as i32, '─', ring_col);
        }
        let jewel_count = params.spokes.min(24);
        for s in 0..jewel_count {
            let a = rot + s as f32 * TAU / jewel_count.max(1) as f32;
            let x = plan.cx as f32 + a.cos() * rrx;
            let y = plan.halo_cy as f32 + a.sin() * rry;
            let pulse = (t * params.speed * 1.2 + s as f32 * 0.7 + frac * 3.0).sin();
            let (ch, col) = if pulse > 0.45 {
                ('✦', lighten(gold, (params.glow * 12.0) as u8))
            } else {
                ('◇', darken(gold, 18))
            };
            put(grid, x.round() as i32, y.round() as i32, ch, col);
        }
        if params.runes > 0.35 && r > 1 {
            let n = (params.spokes as f32 * frac).max(3.0).round() as usize;
            for s in 0..n {
                let a = -rot * 1.6 + s as f32 * TAU / n as f32;
                let x = plan.cx as f32 + a.cos() * (rrx + 1.3);
                let y = plan.halo_cy as f32 + a.sin() * (rry + 0.9);
                let g = GLYPHS[((seed as usize) + s * 7 + r * 3) % GLYPHS.len()];
                let col = darken(palette[2], 12 + (r * 6) as u8);
                put(grid, x.round() as i32, y.round() as i32, g, col);
            }
        }
    }
    for s in 0..params.spokes {
        let a = t * params.speed * 0.06 + s as f32 * TAU / params.spokes as f32;
        let x0 = plan.cx as f32 + a.cos() * rx * 0.24;
        let y0 = plan.halo_cy as f32 + a.sin() * ry * 0.24;
        let x1 = plan.cx as f32 + a.cos() * rx * 0.98;
        let y1 = plan.halo_cy as f32 + a.sin() * ry * 0.98;
        pp_line(
            grid,
            x0.round() as i32,
            y0.round() as i32,
            x1.round() as i32,
            y1.round() as i32,
            darken(stone, 8),
        );
    }
    let bp = (t * params.speed * 0.9).sin();
    put(
        grid,
        plan.cx,
        plan.halo_cy,
        '◉',
        if bp > 0.0 { lighten(gold, 16) } else { gold },
    );
}

fn draw_wings(
    grid: &mut Grid,
    plan: &Plan,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    params: &ApotheosisParams,
) {
    if params.wings == 0 {
        return;
    }
    let plume = lighten(palette[3], 14);
    let feathers = 7;
    for side in [-1i32, 1] {
        for f in 0..feathers {
            let h = hash01(seed, 400 + (side + 2) as u64 * 31 + f as u64 * 7);
            let spread = f as f32 / (feathers - 1).max(1) as f32;
            let flap = (t * params.speed * 0.8 + spread * 1.4).sin() * 0.22;
            let ang = -std::f32::consts::FRAC_PI_2
                + side as f32 * (0.55 + spread * 1.05)
                + flap * side as f32;
            let len = plan.halo_rx * (0.55 + spread * 0.42) * (1.0 - spread * 0.18)
                + h * 2.0;
            let x1 = plan.cx as f32 + ang.cos() * len;
            let y1 = (plan.fig_y - 2) as f32 + ang.sin() * len * 0.62;
            let col = if spread > 0.7 {
                lighten(plume, 8)
            } else {
                plume
            };
            pp_line(grid, plan.cx, plan.fig_y - 2, x1.round() as i32, y1.round() as i32, col);
            let tip_x = x1.round() as i32;
            let tip_y = y1.round() as i32;
            put(grid, tip_x, tip_y, if h > 0.6 { '✧' } else { '∙' }, lighten(col, 10));
            let mid_x = (plan.cx as f32 + x1) / 2.0;
            let mid_y = (plan.fig_y - 2) as f32 + y1;
            put(
                grid,
                mid_x.round() as i32,
                mid_y.round() as i32,
                if f % 2 == 0 { '╱' } else { '╲' },
                darken(col, 16),
            );
        }
    }
}

fn draw_figure(
    grid: &mut Grid,
    plan: &Plan,
    palette: &[Color; 5],
    t: f32,
    params: &ApotheosisParams,
) {
    let bob = (t * params.speed * 0.9).sin() * params.bob * 0.9;
    let fy = plan.fig_y + bob.round() as i32;
    let silver = lighten(palette[4], 26);
    let shade = darken(palette[4], 18);
    let head = fy - 4;
    put(grid, plan.cx, head, '☀', lighten(silver, 14));
    put(grid, plan.cx - 1, head + 1, '╭', shade);
    put(grid, plan.cx + 1, head + 1, '╮', shade);
    put(grid, plan.cx, head + 1, '┄', shade);
    for y in head + 2..=fy - 1 {
        let robe = if (y + fy).rem_euclid(2) == 0 { '▓' } else { '▒' };
        put(grid, plan.cx, y, robe, shade);
    }
    for y in fy..=fy + 3 {
        let w = 1 + (y - fy) / 2;
        for dx in -w..=w {
            let ch = if dx == 0 { '│' } else { '░' };
            put(
                grid,
                plan.cx + dx,
                y,
                ch,
                darken(shade, (6 + (y - fy) * 5) as u8),
            );
        }
    }
    let hem = fy + 4;
    for dx in -3..=3 {
        put(
            grid,
            plan.cx + dx,
            hem,
            if dx.abs() == 3 { '·' } else { '∙' },
            darken(shade, 30 + (dx.abs() * 5) as u8),
        );
    }
    let pulse = (t * params.speed * 1.3).sin();
    put(
        grid,
        plan.cx,
        head - 2,
        if pulse > 0.3 { '✧' } else { '✦' },
        lighten(palette[3], (18.0 + params.glow * 14.0) as u8),
    );
}

fn draw_rays(
    grid: &mut Grid,
    plan: &Plan,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    params: &ApotheosisParams,
) {
    if params.rays <= 0.03 {
        return;
    }
    let gold = lighten(shift_hue(palette[3], 350.0), 24);
    let n = (3.0 + params.rays * 9.0).round() as usize;
    let max_d = (plan.halo_ry * 3.4 + 6.0).max(8.0);
    for i in 0..n {
        let fr = if n > 1 {
            i as f32 / (n - 1) as f32 - 0.5
        } else {
            0.0
        };
        let ang = std::f32::consts::FRAC_PI_2 + fr * 2.4 + (t * params.speed * 0.05).sin() * 0.06;
        let steps = (max_d * 2.0) as usize;
        for step in 1..steps {
            let d = step as f32 * 0.5 + plan.halo_ry * 0.7;
            let x = (plan.cx as f32 + ang.cos() * d * 1.7).round() as i32;
            let y = (plan.halo_cy as f32 + ang.sin() * d * 0.9).round() as i32;
            if y < 1 || y >= plan.floor_y {
                continue;
            }
            if hash01(seed, i as u64 * 997 + step as u64) > 0.40 + params.glow * 0.22 {
                continue;
            }
            let glyph = if step % 5 == 0 { '░' } else { '·' };
            put(
                grid,
                x,
                y,
                glyph,
                darken(gold, 10 + (d / max_d * 36.0) as u8),
            );
        }
    }
}

fn draw_motes(
    grid: &mut Grid,
    plan: &Plan,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    params: &ApotheosisParams,
) {
    if params.motes == 0 {
        return;
    }
    let warm = lighten(palette[3], 12);
    for i in 0..params.motes {
        let h1 = hash01(seed, 1000 + i as u64 * 13);
        let h2 = hash01(seed, 2000 + i as u64 * 29);
        let h3 = hash01(seed, 3000 + i as u64 * 41);
        let orbit_r = plan.halo_rx * (0.55 + h1 * 1.35);
        let orbit_rv = plan.halo_ry * (0.55 + h1 * 1.20);
        let a = t * params.speed * (0.25 + h2 * 0.55) * (if h3 > 0.5 { 1.0 } else { -1.0 })
            + h2 * TAU;
        let x = plan.cx as f32 + a.cos() * orbit_r;
        let y = plan.halo_cy as f32 + a.sin() * orbit_rv + (h3 - 0.5) * 3.0;
        let fade = 18 + (h3 * 34.0) as u8;
        let ch = if h3 > 0.85 { '✦' } else if h3 > 0.45 { '∙' } else { '·' };
        put(grid, x.round() as i32, y.round() as i32, ch, darken(warm, fade));
    }
}

fn draw_sparks(
    grid: &mut Grid,
    plan: &Plan,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    params: &ApotheosisParams,
) {
    if params.sparks == 0 {
        return;
    }
    let hot = lighten(palette[3], 28);
    let top_y = 1.0f32;
    let rise = (plan.fig_y as f32 - top_y).max(4.0);
    for i in 0..params.sparks {
        let h1 = hash01(seed, 5000 + i as u64 * 17);
        let h2 = hash01(seed, 6000 + i as u64 * 23);
        let period = 5.0 + h2 * 6.0;
        let prog = ((t * params.speed * 0.35 + h1 * period) % period) / period;
        let x0 = plan.cx as f32 + (h1 - 0.5) * plan.halo_rx * 3.4;
        let x = clampi(
            (x0 + (prog * TAU * 1.4 + h2 * TAU).sin() * 1.8).round() as i32,
            1,
            width as i32 - 2,
        );
        let y = (plan.fig_y as f32 + 4.0) - prog * rise;
        if y < top_y || y > height as f32 - 1.0 {
            continue;
        }
        let (ch, fade) = if prog < 0.25 {
            ('✦', 4u8)
        } else if prog < 0.55 {
            ('*', 16)
        } else if prog < 0.8 {
            ('∙', 30)
        } else {
            ('·', 44)
        };
        put(grid, x, y.round() as i32, ch, darken(hot, fade));
    }
}

fn draw_clouds(
    grid: &mut Grid,
    plan: &Plan,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    params: &ApotheosisParams,
) {
    if params.clouds == 0 {
        return;
    }
    let lit = lighten(palette[2], 14);
    let dark = darken(palette[2], 26);
    for y in plan.cloud_top..height as i32 {
        let f = (y - plan.cloud_top) as f32 / (height as i32 - plan.cloud_top).max(1) as f32;
        for x in 0..width {
            let drift = t * params.speed * (6.0 + f * 10.0);
            let n = pp_fbm(
                x as f32 * 0.16 + drift,
                y as f32 * 0.55 + f * 3.0,
                seed ^ 0xC10,
            );
            let density = n * (0.45 + f * 0.75) * (params.clouds as f32 / 4.0 * 1.35);
            if density > 0.52 {
                let ch = if density > 0.80 {
                    '▓'
                } else if density > 0.66 {
                    '▒'
                } else {
                    '░'
                };
                let col = if density > 0.74 { lit } else { dark };
                put_bg(grid, x as i32, y, ch, darken(col, 8), darken(col, 4));
            }
        }
    }
}

fn draw_frame(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
) {
    if width < 10 || height < 8 {
        return;
    }
    let edge = darken(lighten(palette[1], 18), 14);
    let stud = lighten(palette[3], 12);
    let phase = (t * 1.5 + seed as f32 * 0.011).floor() as usize;
    for x in 1..width as i32 - 1 {
        let pattern = (x as usize + phase) % 8;
        let ch = match pattern {
            0 => '◆',
            4 => '◇',
            _ => '═',
        };
        put(grid, x, 0, ch, if pattern % 4 == 0 { stud } else { edge });
        let pattern = (x as usize + phase + 3) % 8;
        let ch = match pattern {
            0 => '◇',
            4 => '◆',
            _ => '═',
        };
        put(
            grid,
            x,
            height as i32 - 1,
            ch,
            if pattern % 4 == 0 { stud } else { edge },
        );
    }
    for y in 1..height as i32 - 1 {
        let ch = if (y as usize * 2 + phase) % 9 == 0 {
            '○'
        } else {
            '║'
        };
        put(grid, 0, y, ch, edge);
        put(grid, width as i32 - 1, y, ch, edge);
    }
    put(grid, 0, 0, '╔', stud);
    put(grid, width as i32 - 1, 0, '╗', stud);
    put(grid, 0, height as i32 - 1, '╚', stud);
    put(grid, width as i32 - 1, height as i32 - 1, '╝', stud);
}

pub(crate) fn draw_glm_apotheosis(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    rng: &mut StdRng,
    t: f32,
    params: &ApotheosisParams,
) {
    if width == 0 || height == 0 {
        return;
    }
    let t = t + rng.random_range(0.0..TAU) * 0.02;
    let plan = compute_plan(width, height);

    measure_layer("glm-apotheosis", "background", || {
        draw_sky(grid, width, height, seed, palette, t, params);
    });
    measure_layer("glm-apotheosis", "rays", || {
        draw_rays(grid, &plan, seed, palette, t, params);
    });
    measure_layer("glm-apotheosis", "mandala", || {
        draw_mandala(grid, &plan, seed, palette, t, params);
        draw_wings(grid, &plan, seed, palette, t, params);
    });
    measure_layer("glm-apotheosis", "figure", || {
        draw_figure(grid, &plan, palette, t, params);
    });
    measure_layer("glm-apotheosis", "particles", || {
        draw_motes(grid, &plan, seed, palette, t, params);
        draw_sparks(grid, &plan, width, height, seed, palette, t, params);
        draw_clouds(grid, &plan, width, height, seed, palette, t, params);
    });
    measure_layer("glm-apotheosis", "frame", || {
        draw_frame(grid, width, height, seed, palette, t);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::make_palette;
    use crate::render::grid_to_plain;
    use rand::SeedableRng;

    fn frame(width: usize, height: usize, seed: u64, t: f32, params: &ApotheosisParams) -> Grid {
        let mut grid = vec![vec![Cell::blank(); width]; height];
        let palette = make_palette(seed);
        let mut rng = StdRng::seed_from_u64(seed);
        draw_glm_apotheosis(
            &mut grid, width, height, seed, &palette, &mut rng, t, params,
        );
        grid
    }

    fn plain(grid: &Grid) -> String {
        grid_to_plain(grid).join("\n")
    }

    #[test]
    fn deterministic_seeded_frame_and_visible_motion() {
        let params = ApotheosisParams::default();
        let a = plain(&frame(80, 36, 42, 1.25, &params));
        let b = plain(&frame(80, 36, 42, 1.25, &params));
        let c = plain(&frame(80, 36, 42, 3.75, &params));
        let different_seed = plain(&frame(80, 36, 43, 1.25, &params));
        let tuned = ApotheosisParams {
            rings: 9,
            spokes: 24,
            motes: 80,
            wings: 2,
            rays: 1.0,
            glow: 1.5,
            clouds: 4,
            sparks: 60,
            runes: 1.0,
            bob: 3.0,
            dens: 1.0,
            speed: 3.0,
        };
        let different_inputs = plain(&frame(80, 36, 42, 1.25, &tuned));
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_ne!(a, different_seed);
        assert_ne!(a, different_inputs);
        assert_eq!(
            a.lines()
                .map(str::chars)
                .map(Iterator::count)
                .collect::<Vec<_>>(),
            vec![80; 36]
        );
    }

    #[test]
    fn tiny_grid_and_extreme_inputs_terminate() {
        let params = ApotheosisParams {
            rings: 9,
            spokes: 24,
            motes: 80,
            wings: 2,
            rays: 1.0,
            glow: 1.5,
            clouds: 4,
            sparks: 60,
            runes: 1.0,
            bob: 3.0,
            dens: 1.0,
            speed: 3.0,
        };
        for (w, h) in [(12usize, 6usize), (3, 3), (1, 1), (40, 5)] {
            let output = frame(w, h, 7, 8.0, &params);
            assert_eq!(output.len(), h);
            assert_eq!(output.iter().map(Vec::len).collect::<Vec<_>>(), vec![w; h]);
        }
    }

    #[test]
    fn dimensions_shape_the_frame() {
        let params = ApotheosisParams::default();
        for (w, h) in [(60usize, 24usize), (80, 45), (100, 30)] {
            let output = frame(w, h, 11, 0.0, &params);
            assert_eq!(output.len(), h);
            assert_eq!(
                plain(&output)
                    .lines()
                    .map(str::chars)
                    .map(Iterator::count)
                    .collect::<Vec<_>>(),
                vec![w; h]
            );
        }
        let wide = plain(&frame(100, 30, 11, 0.0, &params));
        let narrow = plain(&frame(60, 24, 11, 0.0, &params));
        assert_ne!(wide, narrow);
    }

    #[test]
    fn params_from_args_override_and_clamp() {
        let args: Vec<String> = [
            "ascii-renderer",
            "42",
            "glm-apotheosis",
            "ember",
            "9",
            "24",
            "80",
            "2",
            "1",
            "1.5",
            "4",
            "60",
            "1",
            "3",
            "1",
            "3",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let p = ApotheosisParams::from_args(&args);
        assert_eq!(p.rings, 9);
        assert_eq!(p.spokes, 24);
        assert_eq!(p.motes, 80);
        assert_eq!(p.wings, 2);
        assert!((p.rays - 1.0).abs() < 1e-6);
        assert_eq!(p.clouds, 4);
        assert_eq!(p.sparks, 60);
        let clamped = ApotheosisParams::from_args(&[
            "bin".to_string(),
            "1".to_string(),
            "glm-apotheosis".to_string(),
            "ember".to_string(),
            "999".to_string(),
            "-5".to_string(),
        ]);
        assert_eq!(clamped.rings, 9);
        assert_eq!(clamped.spokes, 4);
        let defaults = ApotheosisParams::from_args(&[]);
        assert_eq!(defaults.rings, 5);
        assert_eq!(defaults.motes, 30);
        assert!((defaults.glow - 0.8).abs() < 1e-6);
    }

    #[test]
    fn snapshot_glm_apotheosis_t0() {
        let params = ApotheosisParams::default();
        insta::assert_snapshot!("glm_apotheosis_t0", plain(&frame(80, 36, 42, 0.0, &params)));
    }

    #[test]
    fn snapshot_glm_apotheosis_in_motion() {
        let params = ApotheosisParams::default();
        insta::assert_snapshot!(
            "glm_apotheosis_t2_75",
            plain(&frame(80, 36, 42, 2.75, &params))
        );
    }
}
