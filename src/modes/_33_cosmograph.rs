use crossterm::style::Color;
use rand::rngs::StdRng;
use rand::RngExt;

use crate::color::{darken, lerp_color, lighten, shift_hue};
use crate::opts::param_f32;
use crate::pp::{pp_arc, pp_fbm, pp_line};
use crate::registry::{AnimKind, Mode, ModeFrame, Param};
use crate::types::{Cell, Grid};

const TAU: f32 = std::f32::consts::TAU;

pub(super) struct CosmographMode;

pub(super) static MODE: CosmographMode = CosmographMode;

const PARAMS: &[Param] = &[
    param!("RINGS", "orrery rings", 2.0, 10.0, 6.0, 1.0),
    param!("PLANETS", "planets", 1.0, 7.0, 5.0, 1.0),
    param!("MOONS", "moons", 0.0, 3.0, 1.0, 1.0),
    param!("ZODIAC", "zodiac band", 0.0, 1.0, 1.0, 0.05),
    param!("SPIRAL", "phyllotaxis", 0.0, 1.0, 0.8, 0.05),
    param!("AURORA", "aurora", 0.0, 3.0, 2.0, 0.1),
    param!("STARS", "star density", 0.0, 1.0, 0.55, 0.05),
    param!("TRAIL", "comet trail", 0.0, 40.0, 18.0, 2.0),
    param!("MOTES", "drift motes", 0.0, 70.0, 26.0, 2.0),
    param!("GLOW", "glow", 0.0, 1.5, 0.8, 0.05),
    param!("TILT", "orbit tilt", 0.2, 1.0, 0.58, 0.02),
    param!("SPEED", "motion", 0.05, 3.0, 0.7, 0.05),
];

impl Mode for CosmographMode {
    fn name(&self) -> &'static str {
        "cosmograph"
    }

    fn help(&self) -> &'static str {
        "Grand orrery: nested counter-rotating rings, planets with moons, zodiac band, aurora, phyllotaxis bloom, comet [rings] [planets] [moons] [zodiac] [spiral] [aurora] [stars] [trail] [motes] [glow] [tilt] [speed]"
    }

    fn animation(&self) -> AnimKind {
        AnimKind::Iterate
    }

    fn params(&self) -> &'static [Param] {
        PARAMS
    }

    fn render(&self, frame: &mut ModeFrame<'_>) {
        let params = CosmographParams::from_inputs(frame.args, frame.param_values);
        draw_cosmograph(
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
pub(crate) struct CosmographParams {
    pub(crate) rings: usize,
    pub(crate) planets: usize,
    pub(crate) moons: usize,
    pub(crate) zodiac: f32,
    pub(crate) spiral: f32,
    pub(crate) aurora: f32,
    pub(crate) stars: f32,
    pub(crate) trail: usize,
    pub(crate) motes: usize,
    pub(crate) glow: f32,
    pub(crate) tilt: f32,
    pub(crate) speed: f32,
}

impl Default for CosmographParams {
    fn default() -> Self {
        Self {
            rings: 6,
            planets: 5,
            moons: 1,
            zodiac: 1.0,
            spiral: 0.8,
            aurora: 2.0,
            stars: 0.55,
            trail: 18,
            motes: 26,
            glow: 0.8,
            tilt: 0.58,
            speed: 0.7,
        }
    }
}

impl CosmographParams {
    pub(crate) fn from_args(args: &[String]) -> Self {
        Self::from_inputs(args, None)
    }

    pub(crate) fn from_inputs(args: &[String], param_values: Option<&[f32]>) -> Self {
        let read = |index: usize, key: &str, default: f32| {
            args.get(index)
                .and_then(|value| value.parse::<f32>().ok())
                .or_else(|| param_values.and_then(|values| values.get(index - 4).copied()) )
                .unwrap_or_else(|| param_f32(key, default))
        };
        Self {
            rings: read(4, "RINGS", 6.0).round().clamp(2.0, 10.0) as usize,
            planets: read(5, "PLANETS", 5.0).round().clamp(1.0, 7.0) as usize,
            moons: read(6, "MOONS", 1.0).round().clamp(0.0, 3.0) as usize,
            zodiac: read(7, "ZODIAC", 1.0).clamp(0.0, 1.0),
            spiral: read(8, "SPIRAL", 0.8).clamp(0.0, 1.0),
            aurora: read(9, "AURORA", 2.0).clamp(0.0, 3.0),
            stars: read(10, "STARS", 0.55).clamp(0.0, 1.0),
            trail: read(11, "TRAIL", 18.0).round().clamp(0.0, 40.0) as usize,
            motes: read(12, "MOTES", 26.0).round().clamp(0.0, 70.0) as usize,
            glow: read(13, "GLOW", 0.8).clamp(0.0, 1.5),
            tilt: read(14, "TILT", 0.58).clamp(0.2, 1.0),
            speed: read(15, "SPEED", 0.7).clamp(0.05, 3.0),
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
    cy: i32,
    max_rx: f32,
    max_ry: f32,
    floor_y: i32,
}

fn compute_plan(width: usize, height: usize, tilt: f32) -> Plan {
    let w = width as i32;
    let h = height as i32;
    let cx = w / 2;
    let cy = clampi((h as f32 * 0.48).round() as i32, 3, (h - 5).max(3));
    let max_rx = ((w as f32 / 2.0) * 0.86).clamp(4.0, 60.0);
    let max_ry = (max_rx * tilt).min((h as f32 / 2.0) * 0.82).max(3.0);
    Plan {
        cx,
        cy,
        max_rx,
        max_ry,
        floor_y: h - 2,
    }
}

fn draw_nebula(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    params: &CosmographParams,
) {
    let plan = compute_plan(width, height, params.tilt);
    let deep = darken(shift_hue(palette[0], 220.0), 34);
    let rim = darken(palette[0], 20);
    let heart = lerp_color(lighten(palette[2], 6), palette[3], 0.35);
    let span_y = height.saturating_sub(1).max(1) as f32;
    let span_x = width.saturating_sub(1).max(1) as f32;
    for y in 0..height {
        for x in 0..width {
            let vfade = y as f32 / span_y;
            let base = lerp_color(deep, rim, vfade.powf(0.85));
            let swirl = pp_fbm(
                x as f32 * 0.055 + (t * params.speed * 0.045).sin() * 2.4,
                y as f32 * 0.085 - t * params.speed * 0.02,
                seed ^ 0x0CB,
            );
            let dx = (x as f32 - plan.cx as f32) / span_x.max(8.0);
            let dy = (y as f32 - plan.cy as f32) / span_y.max(6.0);
            let d = (dx * dx * 0.7 + dy * dy).sqrt();
            let warmth = (1.0 - d.clamp(0.0, 1.0)).powf(2.6)
                * (0.22 + params.glow * 0.40 + swirl * 0.22);
            let bg = lerp_color(base, heart, warmth.clamp(0.0, 1.0));
            grid[y][x] = Cell::with_bg(' ', bg, bg);
        }
    }
}

fn draw_starfield(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    params: &CosmographParams,
) {
    let star = lighten(palette[4], 10);
    let faint = darken(star, 34);
    for y in 0..height {
        for x in 0..width {
            let pick = hash01(seed, x as u64 * 131 + y as u64 * 977 + 5);
            let field = pp_fbm(x as f32 * 0.10, y as f32 * 0.14, seed ^ 0x57A);
            let threshold = 0.875 - params.stars * 0.20;
            if field > 0.68 && pick > threshold {
                let tw = (t * params.speed * 1.1 + pick * TAU * 2.0).sin();
                let ch = if tw > 0.86 {
                    '✦'
                } else if tw > 0.2 {
                    '∙'
                } else {
                    '·'
                };
                let col = if tw > 0.55 { star } else { faint };
                put(grid, x as i32, y as i32, ch, col);
            }
        }
    }
}

fn draw_aurora(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    params: &CosmographParams,
) {
    if params.aurora <= 0.02 {
        return;
    }
    let bands = params.aurora.round().clamp(1.0, 3.0) as usize;
    let band_h = ((height as f32 * 0.30) / bands as f32).max(2.0);
    for b in 0..bands {
        let hue_off = b as f64 * 58.0;
        let ribbon = shift_hue(lighten(palette[2], 6), 150.0 + hue_off);
        let dim = darken(ribbon, 20);
        for x in 0..width {
            let phase = x as f32 * 0.09
                + t * params.speed * (0.5 + b as f32 * 0.22)
                + b as f32 * 2.1;
            let wave = (phase).sin() * band_h * 0.42 + (phase * 0.47).sin() * band_h * 0.3;
            let base_y = 1.0 + b as f32 * band_h * 0.8;
            for k in 0..band_h.round().max(2.0) as i32 {
                let y = (base_y + wave + k as f32 * 0.9).round() as i32;
                if y < 1 || y >= height as i32 - 1 {
                    continue;
                }
                let density = pp_fbm(x as f32 * 0.12, k as f32 * 0.4 + b as f32 * 7.0, seed ^ 0xA0B);
                if density < 0.36 {
                    continue;
                }
                let fade = (k as f32 / band_h).clamp(0.0, 1.0);
                let ch = if fade < 0.2 && density > 0.62 {
                    '▒'
                } else if density > 0.5 {
                    '░'
                } else {
                    '·'
                };
                let col = lerp_color(ribbon, dim, fade);
                put(grid, x as i32, y, ch, col);
            }
        }
    }
}

fn draw_spiral(
    grid: &mut Grid,
    plan: &Plan,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    params: &CosmographParams,
) {
    if params.spiral <= 0.03 {
        return;
    }
    let n = (params.spiral * 90.0).round() as usize;
    let petal = lighten(palette[3], 18);
    let core = lighten(palette[4], 22);
    for i in 0..n {
        let h = hash01(seed, 7000 + i as u64 * 37);
        let a = i as f32 * 2.39996323; // golden angle
        let r_frac = ((i as f32 + 1.0) / n as f32).sqrt() * params.spiral;
        let breathe = 1.0 + (t * params.speed * 0.6 + h * TAU).sin() * 0.06;
        let rx = plan.max_rx * r_frac * breathe * 0.92;
        let ry = plan.max_ry * r_frac * breathe * 0.92;
        let x = plan.cx as f32 + a.cos() * rx;
        let y = plan.cy as f32 + a.sin() * ry;
        let pulse = (t * params.speed * 1.4 + h * TAU).sin();
        let (ch, col) = if pulse > 0.7 {
            ('•', core)
        } else if pulse > 0.0 {
            ('∙', petal)
        } else {
            ('·', darken(petal, 24))
        };
        put(grid, x.round() as i32, y.round() as i32, ch, col);
    }
}

fn draw_orrery(
    grid: &mut Grid,
    plan: &Plan,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    params: &CosmographParams,
) {
    let brass = lighten(shift_hue(palette[3], 20.0), 16);
    let dark_brass = darken(brass, 20);
    for r in (1..=params.rings).rev() {
        let frac = r as f32 / params.rings.max(1) as f32;
        let rx = plan.max_rx * (0.24 + 0.76 * frac);
        let ry = plan.max_ry * (0.24 + 0.76 * frac);
        let dir = if r.rem_euclid(2) == 0 { 1.0 } else { -1.0 };
        let rot = t * params.speed * dir * (0.06 + frac * 0.16);
        let ring_col = if frac > 0.82 { brass } else { dark_brass };
        pp_arc(
            grid,
            plan.cx,
            plan.cy,
            rx,
            ry,
            rot % TAU,
            (rot % TAU) + TAU,
            ring_col,
            if r == 1 { 5 } else { 7 },
        );
        // teeth: clockwork ticks on each band
        let teeth = (6 + r * 5).min(26);
        for s in 0..teeth {
            let a = rot * 0.8 + s as f32 * TAU / teeth as f32;
            let x = plan.cx as f32 + a.cos() * rx;
            let y = plan.cy as f32 + a.sin() * ry;
            let long = (s + r) % 3 == 0;
            put(
                grid,
                x.round() as i32,
                y.round() as i32,
                if long { '┼' } else { '·' },
                if long { brass } else { darken(brass, 14) },
            );
        }
        // epicycle: small eccentric circle riding each band
        let e = hash01(seed, 800 + r as u64 * 91);
        let ea = t * params.speed * (0.3 + e * 0.5) * dir + e * TAU;
        let ecx = plan.cx as f32 + ea.cos() * rx;
        let ecy = plan.cy as f32 + ea.sin() * ry;
        let er = 1.2 + e * 1.6;
        pp_arc(
            grid,
            ecx.round() as i32,
            ecy.round() as i32,
            er,
            er * 0.8,
            0.0,
            TAU,
            darken(brass, 26),
            9,
        );
        put(grid, ecx.round() as i32, ecy.round() as i32, '◌', darken(brass, 30));
    }
    // spokes from core to inner band
    let spokes = params.rings.max(4).min(12);
    for s in 0..spokes {
        let a = -t * params.speed * 0.10 + s as f32 * TAU / spokes as f32;
        let x1 = plan.cx as f32 + a.cos() * plan.max_rx * 0.24;
        let y1 = plan.cy as f32 + a.sin() * plan.max_ry * 0.24;
        pp_line(
            grid,
            plan.cx,
            plan.cy,
            x1.round() as i32,
            y1.round() as i32,
            darken(brass, 10),
        );
    }
    let pulse = (t * params.speed * 1.2).sin();
    put(
        grid,
        plan.cx,
        plan.cy,
        if pulse > 0.0 { '☉' } else { '◉' },
        lighten(brass, (10.0 + params.glow * 14.0) as u8),
    );
}

const ZODIAC: [char; 12] = [
    '♈', '♉', '♊', '♋', '♌', '♍', '♎', '♏', '♐', '♑', '♒', '♓',
];

fn draw_zodiac(
    grid: &mut Grid,
    plan: &Plan,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    params: &CosmographParams,
) {
    if params.zodiac <= 0.03 {
        return;
    }
    let glyph = lighten(palette[1], 12);
    let dim = darken(glyph, 22);
    let rx = plan.max_rx * 1.06 * params.zodiac.clamp(0.0, 1.0);
    let ry = plan.max_ry * 1.06 * params.zodiac.clamp(0.0, 1.0);
    let rot = t * params.speed * 0.045;
    for i in 0..12 {
        let a = rot + i as f32 * TAU / 12.0;
        let x = plan.cx as f32 + a.cos() * rx;
        let y = plan.cy as f32 + a.sin() * ry;
        let tw = (t * params.speed + i as f32 * 0.5).sin();
        let col = if tw > 0.4 { glyph } else { dim };
        put(grid, x.round() as i32, y.round() as i32, ZODIAC[i], col);
        // tick between glyphs
        let am = a + TAU / 24.0;
        let tx = plan.cx as f32 + am.cos() * rx * 0.97;
        let ty = plan.cy as f32 + am.sin() * ry * 0.97;
        put(grid, tx.round() as i32, ty.round() as i32, '¦', dim);
    }
    // dotted band arcs between glyphs
    pp_arc(
        grid,
        plan.cx,
        plan.cy,
        rx,
        ry,
        rot % TAU,
        (rot % TAU) + TAU,
        darken(dim, 12),
        4,
    );
    let _ = seed;
}

const PLANET_GLYPHS: [char; 7] = ['☿', '♀', '⊕', '♂', '♃', '♄', '🜨'];

fn draw_planets(
    grid: &mut Grid,
    plan: &Plan,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    params: &CosmographParams,
) {
    for p in 0..params.planets {
        let band = params.rings.max(2) as f32;
        let slot = (p as f32 + 1.0) / band;
        let rx = plan.max_rx * (0.24 + 0.76 * slot);
        let ry = plan.max_ry * (0.24 + 0.76 * slot);
        let h1 = hash01(seed, 300 + p as u64 * 53);
        let h2 = hash01(seed, 500 + p as u64 * 71);
        let h3 = hash01(seed, 900 + p as u64 * 97);
        // eccentric offset so orbits breathe
        let ecc = (h1 - 0.5) * rx * 0.16;
        let a = t * params.speed * (0.9 - slot * 0.62) * (if h2 > 0.5 { 1.0 } else { -1.0 })
            + h2 * TAU;
        let px = plan.cx as f32 + a.cos() * (rx + ecc);
        let py = plan.cy as f32 + a.sin() * ry + ecc * 0.4;
        let body_col = lerp_color(
            lighten(palette[4], 18),
            shift_hue(palette[2], (h3 * 300.0 - 150.0) as f64),
            h1,
        );
        put(
            grid,
            px.round() as i32,
            py.round() as i32,
            PLANET_GLYPHS[p % PLANET_GLYPHS.len()],
            body_col,
        );
        // ring around the ringed planet
        if h3 > 0.72 {
            pp_arc(
                grid,
                px.round() as i32,
                py.round() as i32,
                2.4,
                0.9,
                0.0,
                TAU,
                darken(body_col, 18),
                6,
            );
        }
        // moonlets
        for m in 0..params.moons {
            let mh = hash01(seed, 4600 + p as u64 * 13 + m as u64 * 101);
            let ma = t * params.speed * (1.6 + mh * 1.4) + mh * TAU;
            let mr = 1.8 + m as f32 * 1.3 + mh;
            let mx = px + ma.cos() * mr;
            let my = py + ma.sin() * mr * 0.7;
            put(
                grid,
                mx.round() as i32,
                my.round() as i32,
                if mh > 0.5 { '·' } else { '∙' },
                lighten(body_col, 10),
            );
        }
        let _ = (width, height);
    }
}

fn draw_comet(
    grid: &mut Grid,
    plan: &Plan,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    params: &CosmographParams,
) {
    if params.trail == 0 {
        return;
    }
    let h1 = hash01(seed, 0xC0D);
    let h2 = hash01(seed, 0xC0E);
    let period = 9.0 + h2 * 7.0;
    let prog = ((t * params.speed * 0.30 + h1 * period) % period) / period;
    // sweeping hyperbolic path across the sky
    let sx = (prog * (width as f32 + 30.0)) - 15.0;
    let sy = 2.0 + (1.0 - (prog * 2.0 - 1.0).abs()) * height as f32 * 0.42
        + (prog * TAU + h1 * 5.0).sin() * 2.0;
    let head_col = lighten(palette[4], 26);
    // trail behind the head
    for k in 1..=params.trail {
        let back = k as f32 / params.trail.max(1) as f32;
        let tx = sx - back * (6.0 + params.trail as f32 * 0.45);
        let ty = sy - back * 2.2 + (prog * TAU * 2.0 + k as f32).sin() * 0.5;
        if tx < 0.0 || ty < 0.0 || ty >= height as f32 - 1.0 {
            continue;
        }
        let fade = (back * 46.0) as u8;
        let ch = if back < 0.15 {
            '•'
        } else if back < 0.5 {
            '∙'
        } else {
            '·'
        };
        put(grid, tx.round() as i32, ty.round() as i32, ch, darken(head_col, fade));
    }
    put(grid, sx.round() as i32, sy.round() as i32, '☄', head_col);
    let _ = plan;
}

fn draw_motes(
    grid: &mut Grid,
    plan: &Plan,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    params: &CosmographParams,
) {
    if params.motes == 0 {
        return;
    }
    let warm = lighten(palette[3], 14);
    for i in 0..params.motes {
        let h1 = hash01(seed, 2000 + i as u64 * 19);
        let h2 = hash01(seed, 2500 + i as u64 * 43);
        let h3 = hash01(seed, 2900 + i as u64 * 67);
        // slow Lissajous drift around the mechanism
        let ax = plan.max_rx * (0.7 + h1 * 0.7);
        let ay = plan.max_ry * (0.7 + h2 * 0.9);
        let x = plan.cx as f32
            + (t * params.speed * (0.12 + h2 * 0.20) + h1 * TAU).sin() * ax;
        let y = plan.cy as f32
            + (t * params.speed * (0.10 + h3 * 0.26) + h2 * TAU).cos() * ay
            + (h3 - 0.5) * 2.0;
        if x < 0.0 || y < 0.0 || x >= width as f32 || y >= height as f32 {
            continue;
        }
        let fade = (14.0 + h3 * 38.0) as u8;
        let ch = if h3 > 0.88 { '✧' } else if h3 > 0.4 { '∙' } else { '·' };
        put(grid, x.round() as i32, y.round() as i32, ch, darken(warm, fade));
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
    if width < 12 || height < 8 {
        return;
    }
    let edge = darken(lighten(palette[1], 16), 10);
    let stud = lighten(palette[3], 14);
    let phase = (t * 1.2 + seed as f32 * 0.013).floor() as usize;
    for x in 1..width as i32 - 1 {
        let pattern = (x as usize * 3 + phase) % 12;
        let (ch, col) = match pattern {
            0 => ('✶', stud),
            6 => ('◆', darken(stud, 8)),
            _ => ('─', edge),
        };
        put(grid, x, 0, ch, col);
        let pattern_b = (x as usize * 3 + phase + 6) % 12;
        let (ch, col) = match pattern_b {
            0 => ('✶', stud),
            6 => ('◆', darken(stud, 8)),
            _ => ('─', edge),
        };
        put(grid, x, height as i32 - 1, ch, col);
    }
    for y in 1..height as i32 - 1 {
        let pattern = (y as usize * 5 + phase) % 11;
        let (ch, col) = if pattern == 0 {
            ('✦', stud)
        } else if pattern == 5 {
            ('│', darken(stud, 6))
        } else {
            ('│', edge)
        };
        put(grid, 0, y, ch, col);
        put(grid, width as i32 - 1, y, ch, col);
    }
    let corner = if (t * 2.0).sin() > 0.0 { '❖' } else { '◈' };
    put(grid, 0, 0, corner, stud);
    put(grid, width as i32 - 1, 0, corner, stud);
    put(grid, 0, height as i32 - 1, corner, stud);
    put(grid, width as i32 - 1, height as i32 - 1, corner, stud);
}

pub(crate) fn draw_cosmograph(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    rng: &mut StdRng,
    t: f32,
    params: &CosmographParams,
) {
    if width == 0 || height == 0 {
        return;
    }
    let t = t + rng.random_range(0.0..TAU) * 0.02;
    let plan = compute_plan(width, height, params.tilt);

    draw_nebula(grid, width, height, seed, palette, t, params);
    draw_starfield(grid, width, height, seed, palette, t, params);
    draw_aurora(grid, width, height, seed, palette, t, params);
    draw_spiral(grid, &plan, seed, palette, t, params);
    draw_orrery(grid, &plan, seed, palette, t, params);
    draw_zodiac(grid, &plan, seed, palette, t, params);
    draw_planets(grid, &plan, width, height, seed, palette, t, params);
    draw_comet(grid, &plan, width, height, seed, palette, t, params);
    draw_motes(grid, &plan, width, height, seed, palette, t, params);
    draw_frame(grid, width, height, seed, palette, t);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::make_palette;
    use crate::render::grid_to_plain;
    use rand::SeedableRng;

    fn frame(width: usize, height: usize, seed: u64, t: f32, params: &CosmographParams) -> Grid {
        let mut grid = vec![vec![Cell::blank(); width]; height];
        let palette = make_palette(seed);
        let mut rng = StdRng::seed_from_u64(seed);
        draw_cosmograph(&mut grid, width, height, seed, &palette, &mut rng, t, params);
        grid
    }

    fn plain(grid: &Grid) -> String {
        grid_to_plain(grid).join("\n")
    }

    #[test]
    fn deterministic_seeded_frame_and_visible_motion() {
        let params = CosmographParams::default();
        let a = plain(&frame(90, 40, 42, 1.25, &params));
        let b = plain(&frame(90, 40, 42, 1.25, &params));
        let c = plain(&frame(90, 40, 42, 3.75, &params));
        let different_seed = plain(&frame(90, 40, 43, 1.25, &params));
        let tuned = CosmographParams {
            rings: 10,
            planets: 7,
            moons: 3,
            zodiac: 0.0,
            spiral: 0.0,
            aurora: 0.0,
            stars: 1.0,
            trail: 40,
            motes: 70,
            glow: 1.5,
            tilt: 1.0,
            speed: 3.0,
        };
        let different_inputs = plain(&frame(90, 40, 42, 1.25, &tuned));
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_ne!(a, different_seed);
        assert_ne!(a, different_inputs);
        assert_eq!(
            a.lines().map(str::chars).map(Iterator::count).collect::<Vec<_>>(),
            vec![90; 40]
        );
    }

    #[test]
    fn tiny_grid_and_extreme_inputs_terminate() {
        let params = CosmographParams {
            rings: 10,
            planets: 7,
            moons: 3,
            zodiac: 1.0,
            spiral: 1.0,
            aurora: 3.0,
            stars: 1.0,
            trail: 40,
            motes: 70,
            glow: 1.5,
            tilt: 1.0,
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
        let params = CosmographParams::default();
        for (w, h) in [(60usize, 24usize), (90, 45), (110, 32)] {
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
        let wide = plain(&frame(110, 32, 11, 0.0, &params));
        let narrow = plain(&frame(60, 24, 11, 0.0, &params));
        assert_ne!(wide, narrow);
    }

    #[test]
    fn params_from_args_override_and_clamp() {
        let args: Vec<String> = [
            "ascii-renderer",
            "42",
            "cosmograph",
            "aurora",
            "10",
            "7",
            "3",
            "0",
            "0",
            "3",
            "1",
            "40",
            "70",
            "1.5",
            "1",
            "3",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let p = CosmographParams::from_args(&args);
        assert_eq!(p.rings, 10);
        assert_eq!(p.planets, 7);
        assert_eq!(p.moons, 3);
        assert!((p.zodiac - 0.0).abs() < 1e-6);
        assert_eq!(p.trail, 40);
        assert_eq!(p.motes, 70);
        let clamped = CosmographParams::from_args(&[
            "bin".to_string(),
            "1".to_string(),
            "cosmograph".to_string(),
            "aurora".to_string(),
            "999".to_string(),
            "-5".to_string(),
        ]);
        assert_eq!(clamped.rings, 10);
        assert_eq!(clamped.planets, 1);
        let defaults = CosmographParams::from_args(&[]);
        assert_eq!(defaults.rings, 6);
        assert_eq!(defaults.motes, 26);
        assert!((defaults.glow - 0.8).abs() < 1e-6);
    }

    #[test]
    fn snapshot_cosmograph_t0() {
        let params = CosmographParams::default();
        insta::assert_snapshot!("cosmograph_t0", plain(&frame(90, 40, 42, 0.0, &params)));
    }

    #[test]
    fn snapshot_cosmograph_in_motion() {
        let params = CosmographParams::default();
        insta::assert_snapshot!(
            "cosmograph_t2_75",
            plain(&frame(90, 40, 42, 2.75, &params))
        );
    }
}
