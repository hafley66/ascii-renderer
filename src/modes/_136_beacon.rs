//! Beacon: a night coast. A striped lighthouse stands on rocks above a slow
//! swell; twin beams sweep stars and sea while a moon glade and a buoy hold.
use crate::_0_profile::measure_layer;
use crate::color::{darken, lerp_color, lighten, shift_hue};
use crate::opts::param_f32;
use crate::registry::{AnimKind, Mode, ModeFrame, Param};
use crate::types::{Cell, Grid};
use crossterm::style::Color;
use std::f32::consts::{PI, TAU};

pub(super) struct Beacon;
pub(super) static MODE: Beacon = Beacon;

const NAME: &str = "beacon";
const KNOBS: usize = 10;
const HELP: &str = "beacon: night coast, lighthouse twin beams sweeping stars and sea [spin] [swell] [tide] [stars] [moon] [beam] [glow] [tower] [hue] [flick]";

/// Flat water up to a crest.
const SEA_GLYPHS: [char; 5] = [' ', '.', '-', '~', '^'];
/// Shoreline rubble on the point.
const ROCK_GLYPHS: [char; 5] = ['^', '%', '/', '\\', '.'];

// Layer tags for the splitmix hash; no rng stream is consumed anywhere.
const L_STAR: u64 = 0x21;
const L_PHASE: u64 = 0x22;
const L_MOON: u64 = 0x23;
const L_GLOW: u64 = 0x24;
const L_SPARK: u64 = 0x25;
const L_SEA: u64 = 0x26;
const L_GLIM: u64 = 0x27;
const L_MGLOW: u64 = 0x28;
const L_WALL: u64 = 0x29;
const L_ROCK: u64 = 0x2A;
const L_LAMP: u64 = 0x2B;
const L_BUOY: u64 = 0x2C;
const L_MOONPOS: u64 = 0x2D;
const L_BEAM0: u64 = 0x2E;

const PARAMS: &[Param] = &[
    param!("SPIN", "beam rotation rad/s", -1.5, 1.5, 0.45, 0.05),
    param!("SWELL", "sea swell height", 0.0, 2.0, 1.0, 0.1),
    param!("TIDE", "horizon height", 0.4, 0.85, 0.62, 0.02),
    param!("STARS", "star density", 0.0, 0.5, 0.14, 0.01),
    param!("MOON", "moon radius cells", 0.0, 8.0, 3.0, 1.0),
    param!("BEAM", "beam half width rad", 0.05, 0.7, 0.22, 0.01),
    param!("GLOW", "beam brightness", 0.0, 2.0, 1.0, 0.05),
    param!("TOWER", "tower height fraction", 0.35, 0.95, 0.8, 0.05),
    param!("HUE", "hue shift degrees", 0.0, 360.0, 0.0, 15.0),
    param!("FLICK", "lamp flicker depth", 0.0, 1.0, 0.25, 0.05),
];

impl Mode for Beacon {
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

/// Splitmix64 over (seed, layer, index, slot); no rng stream is consumed.
#[inline]
fn hash(seed: u64, layer: u64, index: u64, slot: u64) -> u64 {
    let mut z = seed
        ^ layer.wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ index.wrapping_mul(0xD1B5_4A32_D192_ED03)
        ^ slot.wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    z ^= z >> 30;
    z = z.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z ^= z >> 27;
    z = z.wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

#[inline]
fn unit(h: u64) -> f32 {
    (h >> 40) as f32 * (1.0 / 16_777_216.0)
}

#[inline]
fn wrap_pi(a: f32) -> f32 {
    (a + PI).rem_euclid(TAU) - PI
}

/// One frame's resolved geometry and palette.
struct Look {
    seed: u64,
    time: f32,
    hz: usize,
    tx: i32,
    tower_top: usize,
    tower_h: usize,
    wb: i32,
    lamp_x: f32,
    lamp_y: f32,
    beam0: f32,
    moon_x: f32,
    moon_y: f32,
    moon_r: f32,
    moon_glow: f32,
    buoy_x: usize,
    buoy_base: i32,
    buoy_phase: f32,
    spin: f32,
    swell: f32,
    stars: f32,
    beam_w: f32,
    glow: f32,
    flick: f32,
    sky_top: Color,
    sky_low: Color,
    sea_dark: Color,
    sea_mid: Color,
    sea_lit: Color,
    sea_wave: Color,
    star_dim: Color,
    star_bright: Color,
    moon_c: Color,
    moon_dim: Color,
    beam_c: Color,
    beam_dim: Color,
    lamp_c: Color,
    tower_dark: Color,
    tower_light: Color,
    tower_edge: Color,
    rock_fg: Color,
    rock_bg: Color,
    buoy_lit: Color,
    buoy_dim: Color,
}

impl Look {
    fn new(seed: u64, w: usize, h: usize, palette: &[Color; 5], time: f32, p: &[f32; KNOBS]) -> Self {
        let hue = p[8] as f64;
        let sh = |c: Color| shift_hue(c, hue);
        let hz = ((p[2] * h as f32).round() as usize).clamp(1, h.max(2) - 1);
        // The tower needs three rows above the horizon to read as a lantern.
        let tower_h = if hz >= 5 {
            ((p[7] * hz as f32).round() as usize).clamp(3, hz - 2)
        } else {
            0
        };
        let tower_top = hz - tower_h;
        let tx = ((w as f32) * 0.72).round() as i32;
        let wb = (((tower_h as f32) * 0.30).round() as i32).clamp(1, 6);
        let lamp_x = tx as f32 + 0.5;
        let lamp_y = tower_top as f32 + 1.5;
        let beam0 = unit(hash(seed, L_BEAM0, 0, 0)) * TAU;
        let moon_x = w as f32 * (0.12 + 0.34 * unit(hash(seed, L_MOONPOS, 0, 0)));
        let moon_y = hz as f32 * (0.22 + 0.45 * unit(hash(seed, L_MOONPOS, 0, 1)));
        let moon_r = p[4];
        let moon_glow = if moon_r > 0.5 { (moon_r / 4.0).min(1.0) } else { 0.0 };
        let buoy_x = (w as f32 * (0.10 + 0.45 * unit(hash(seed, L_BUOY, 0, 0)))).round() as usize;
        let buoy_base = (hz as i32
            + 1
            + (unit(hash(seed, L_BUOY, 0, 1)) * 3.0).floor() as i32)
        .clamp(hz as i32 + 1, (h as i32 - 1).max(hz as i32 + 1));
        let buoy_phase = unit(hash(seed, L_BUOY, 0, 2)) * TAU;
        Look {
            seed,
            time,
            hz,
            tx,
            tower_top,
            tower_h,
            wb,
            lamp_x,
            lamp_y,
            beam0,
            moon_x,
            moon_y,
            moon_r,
            moon_glow,
            buoy_x,
            buoy_base,
            buoy_phase,
            spin: p[0],
            swell: p[1],
            stars: p[3],
            beam_w: p[5],
            glow: p[6],
            flick: p[9],
            sky_top: sh(darken(palette[0], 22)),
            sky_low: sh(lerp_color(palette[0], palette[2], 0.30)),
            sea_dark: sh(darken(palette[2], 45)),
            sea_mid: sh(darken(palette[2], 22)),
            sea_lit: sh(lighten(palette[2], 30)),
            sea_wave: sh(lighten(palette[2], 15)),
            star_dim: sh(darken(palette[4], 45)),
            star_bright: sh(lighten(palette[4], 35)),
            moon_c: sh(lighten(palette[4], 30)),
            moon_dim: sh(darken(palette[4], 70)),
            beam_c: sh(lighten(palette[3], 40)),
            beam_dim: sh(darken(palette[3], 35)),
            lamp_c: sh(lighten(palette[4], 35)),
            tower_dark: sh(darken(palette[0], 28)),
            tower_light: sh(darken(palette[1], 5)),
            tower_edge: sh(lighten(palette[4], 10)),
            rock_fg: sh(lighten(palette[0], 55)),
            rock_bg: sh(darken(palette[0], 35)),
            buoy_lit: sh(lighten(palette[3], 30)),
            buoy_dim: sh(darken(palette[2], 15)),
        }
    }
}

fn put(grid: &mut Grid, w: usize, h: usize, x: i32, y: i32, ch: char, fg: Color, bg: Color) {
    if x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h {
        grid[y as usize][x as usize] = Cell::with_bg(ch, fg, bg);
    }
}

fn draw(frame: &mut ModeFrame<'_>, p: &[f32; KNOBS]) {
    let (w, h) = (frame.width, frame.height);
    if w == 0 || h == 0 {
        return;
    }
    let look = Look::new(frame.seed, w, h, frame.palette, frame.time, p);
    measure_layer(NAME, "sky", || paint_sky(frame.grid, w, h, &look));
    measure_layer(NAME, "sea", || paint_sea(frame.grid, w, h, &look));
    measure_layer(NAME, "tower", || paint_tower(frame.grid, w, h, &look));
    measure_layer(NAME, "rocks", || paint_rocks(frame.grid, w, h, &look));
    measure_layer(NAME, "lamp", || paint_lamp(frame.grid, w, h, &look));
    measure_layer(NAME, "buoy", || paint_buoy(frame.grid, w, h, &look));
}

/// Twin beam intensity at a cell, aspect corrected so wedges stay round.
/// One unit at the lantern, gaussian in angle, inverse square in range.
fn beam_at(x: f32, y: f32, look: &Look) -> f32 {
    let dx = (x + 0.5 - look.lamp_x) / 2.0;
    let dy = y + 0.5 - look.lamp_y;
    let r = (dx * dx + dy * dy).sqrt();
    if r < 1.2 {
        return 1.0;
    }
    let theta = dy.atan2(dx);
    let a = look.beam0 + look.spin * look.time;
    let mut best = 0.0f32;
    for off in [0.0, PI] {
        let d = wrap_pi(theta - (a + off));
        best = best.max((-d * d / (look.beam_w * look.beam_w)).exp());
    }
    best * look.glow / (1.0 + 0.008 * r * r)
}

/// Sky rows: gradient, twinkling stars, moon disc and halo, beam warmth.
fn paint_sky(grid: &mut Grid, w: usize, h: usize, look: &Look) {
    for y in 0..look.hz.min(h) {
        let grad = if look.hz > 1 { y as f32 / (look.hz - 1) as f32 } else { 0.0 };
        let base = lerp_color(look.sky_top, look.sky_low, grad);
        let row = &mut grid[y];
        for (x, cell) in row.iter_mut().enumerate().take(w) {
            let mut ch = ' ';
            let mut fg = base;
            let mut is_star = false;
            let mag = unit(hash(look.seed, L_STAR, x as u64, y as u64));
            if mag < look.stars {
                is_star = true;
                let rel = mag / look.stars.max(0.001);
                let phase = unit(hash(look.seed, L_PHASE, x as u64, y as u64)) * TAU;
                let tw = 0.5 + 0.5 * (look.time * 1.7 + phase).sin();
                ch = if rel < 0.45 { '.' } else { '+' };
                if rel > 0.85 && tw > 0.55 {
                    ch = '*';
                }
                fg = lerp_color(look.star_dim, look.star_bright, 0.25 + 0.75 * tw);
            }
            if look.moon_r > 0.5 {
                let dx = (x as f32 + 0.5 - look.moon_x) / 2.0;
                let dy = y as f32 + 0.5 - look.moon_y;
                let d = (dx * dx + dy * dy).sqrt();
                if d < look.moon_r - 0.8 {
                    let crater = unit(hash(look.seed, L_MOON, x as u64, y as u64)) < 0.2;
                    ch = if crater { '.' } else { 'o' };
                    fg = if crater { look.moon_dim } else { look.moon_c };
                } else if d < look.moon_r + 0.2 {
                    ch = 'O';
                    fg = look.moon_c;
                } else if d < look.moon_r + 2.2
                    && unit(hash(look.seed, L_GLOW, x as u64, y as u64)) < 0.30
                {
                    ch = '.';
                    fg = lerp_color(look.moon_dim, look.moon_c, 0.5);
                }
            }
            let b = beam_at(x as f32, y as f32, look).min(1.0);
            let mut bg = base;
            if b > 0.08 {
                bg = lerp_color(base, look.beam_dim, (b * 0.45).min(0.85));
            }
            if is_star {
                if b > 0.55 {
                    ch = '*';
                }
                if b > 0.08 {
                    fg = lerp_color(fg, look.beam_c, (b * 0.6).min(0.8));
                }
            } else if ch == ' ' && b > 0.25 {
                // the wedge core speckles into a visible ray of light
                let spark = unit(hash(look.seed, L_SPARK, x as u64, y as u64));
                ch = if b > 0.70 {
                    if spark < 0.75 { '*' } else { '+' }
                } else if b > 0.45 {
                    if spark < 0.80 { '+' } else { '.' }
                } else if spark < 0.70 {
                    '.'
                } else {
                    ' '
                };
                if ch != ' ' {
                    fg = lerp_color(look.beam_dim, look.beam_c, ((b - 0.25) * 1.6).min(1.0));
                }
            }
            *cell = Cell::with_bg(ch, fg, bg);
        }
    }
}

/// Sea rows: swell with row lagged phases, beam glitter, moon glade.
fn paint_sea(grid: &mut Grid, w: usize, h: usize, look: &Look) {
    if look.hz >= h {
        return;
    }
    let span = (h - look.hz).max(1) as f32;
    for y in look.hz..h {
        let depth = (y - look.hz) as f32 / span;
        let row_phase = unit(hash(look.seed, L_SEA, y as u64, 0)) * TAU;
        let jitter = 0.85 + 0.30 * unit(hash(look.seed, L_SEA, y as u64, 1));
        let k = (0.62 - 0.34 * depth) * jitter;
        let base = lerp_color(look.sea_dark, look.sea_mid, depth * 0.8);
        let amp = look.swell * (0.30 + 0.70 * depth);
        let row = &mut grid[y];
        for (x, cell) in row.iter_mut().enumerate().take(w) {
            let shimmer = (x as f32 * 0.11 + look.time * 0.5 + y as f32 * 1.7).sin();
            let phase = x as f32 * k - look.time * 1.5 + row_phase + 0.4 * shimmer;
            let crest = phase.sin() * amp;
            let idx = if crest > 0.60 {
                4
            } else if crest > 0.20 {
                3
            } else if crest > -0.20 {
                2
            } else if crest > -0.60 {
                1
            } else {
                0
            };
            let mut ch = SEA_GLYPHS[idx];
            let mut fg = lerp_color(look.sea_wave, look.sea_lit, (crest * 0.5 + 0.5).clamp(0.0, 1.0));
            let mut bg = base;
            let b = beam_at(x as f32, y as f32, look).min(1.0);
            if b > 0.02 {
                bg = lerp_color(bg, look.beam_dim, (b * 0.50).min(0.85));
            }
            let mut lit = false;
            if b > 0.30 && unit(hash(look.seed, L_GLIM, x as u64, y as u64)) < (b - 0.30) * 1.1 {
                ch = '*';
                fg = look.beam_c;
                lit = true;
            }
            if !lit && look.moon_glow > 0.0 {
                // moonlight widens into a glade toward the viewer
                let half = 1.0 + depth * (w as f32 * 0.16) * look.moon_glow;
                let mg = (1.0 - ((x as f32 + 0.5 - look.moon_x).abs() / half).min(1.0))
                    .powi(2)
                    * look.moon_glow;
                if mg > 0.40 && unit(hash(look.seed, L_MGLOW, x as u64, y as u64)) < (mg - 0.40) * 0.7
                {
                    ch = '.';
                    fg = lerp_color(look.moon_dim, look.moon_c, 0.6);
                } else if mg > 0.05 {
                    bg = lerp_color(bg, look.sea_lit, (mg * 0.30).min(0.5));
                }
            }
            *cell = Cell::with_bg(ch, fg, bg);
        }
    }
}

/// Banded tapered shaft, gallery rail, and lantern room. Bounded to the
/// tower's own rows and columns; the beam never shows through the masonry.
fn paint_tower(grid: &mut Grid, w: usize, h: usize, look: &Look) {
    let hz = look.hz.min(h);
    if look.tower_h == 0 || hz == 0 {
        return;
    }
    let top = look.tower_top;
    let tx = look.tx;
    // roof cap
    for dx in -1..=1 {
        put(grid, w, h, tx + dx, top as i32, '_', look.tower_edge, look.tower_dark);
    }
    // lantern room with lamp glass
    let ly = top + 1;
    put(grid, w, h, tx - 1, ly as i32, '[', look.tower_edge, look.tower_dark);
    put(grid, w, h, tx, ly as i32, '@', look.lamp_c, look.tower_dark);
    put(grid, w, h, tx + 1, ly as i32, ']', look.tower_edge, look.tower_dark);
    // gallery rail
    let gy = top + 2;
    for (dx, ch) in [(-2, '/'), (-1, '='), (0, '='), (1, '='), (2, '\\')] {
        put(grid, w, h, tx + dx, gy as i32, ch, look.tower_edge, look.tower_dark);
    }
    // tapered shaft with painted bands
    let shaft_top = top + 3;
    for y in shaft_top..hz {
        let denom = (hz - shaft_top).max(1) as f32;
        let frac = (y - shaft_top) as f32 / denom;
        let hw = (1.0 + (look.wb - 1) as f32 * frac).round() as i32;
        let band = ((y - top) / 2) % 2 == 0;
        let bg = if band { look.tower_light } else { look.tower_dark };
        for dx in -hw..=hw {
            let ch = if dx == -hw || dx == hw {
                '|'
            } else if unit(hash(look.seed, L_WALL, (tx + dx) as u64, y as u64)) < 0.10 {
                '.'
            } else {
                ' '
            };
            put(grid, w, h, tx + dx, y as i32, ch, look.tower_edge, bg);
        }
    }
}

/// Rubble mound around the base, widest at the waterline. Bounded to the
/// point; sparse at the crest so sky shows between the stones.
fn paint_rocks(grid: &mut Grid, w: usize, h: usize, look: &Look) {
    let hz = look.hz.min(h);
    if hz == 0 {
        return;
    }
    let tx = look.tx;
    for rel in 0..3i32 {
        let y = hz as i32 - 1 + rel;
        if y < 0 {
            continue;
        }
        let y = y as usize;
        let halfw = (look.wb as f32 * (0.55 + 0.75 * rel as f32 / 2.0)).round() as i32 + 2;
        let lo = (tx - halfw).max(0);
        let hi = (tx + halfw).min(w as i32 - 1);
        let density = 0.55 + 0.20 * rel as f32;
        for x in lo..=hi {
            let g = unit(hash(look.seed, L_ROCK, x as u64, y as u64));
            if g > density {
                continue;
            }
            let idx = if g < 0.30 {
                0
            } else if g < 0.45 {
                1
            } else if g < 0.58 {
                2
            } else if g < 0.71 {
                3
            } else {
                4
            };
            let ch = ROCK_GLYPHS[idx];
            put(grid, w, h, x, y as i32, ch, look.rock_fg, look.rock_bg);
        }
    }
}

/// One cell: the lamp flickers inside its lantern room.
fn paint_lamp(grid: &mut Grid, w: usize, h: usize, look: &Look) {
    if look.tower_h == 0 {
        return;
    }
    let ly = look.tower_top + 1;
    if ly >= h || look.tx < 0 || look.tx >= w as i32 {
        return;
    }
    let ph = unit(hash(look.seed, L_LAMP, 0, 0)) * TAU;
    let f = 1.0 - look.flick * 0.5 * (0.5 + 0.5 * (look.time * 7.3 + ph).sin());
    let ch = if f > 0.55 { '@' } else { 'o' };
    grid[ly][look.tx as usize] =
        Cell::with_bg(ch, lerp_color(look.beam_dim, look.lamp_c, f), look.tower_dark);
}

/// A mooring buoy bobbing in the swell with a slow blink and its wake.
/// Bounded to two cells.
fn paint_buoy(grid: &mut Grid, w: usize, h: usize, look: &Look) {
    if look.hz + 1 >= h || w < 3 {
        return;
    }
    let bx = look.buoy_x;
    if bx == 0 || bx >= w - 1 {
        return;
    }
    let bob = (look.time * 0.9 + look.buoy_phase).sin() * 1.2;
    let y = (look.buoy_base as f32 + bob).round() as i32;
    let y = y.clamp(look.hz as i32 + 1, h as i32 - 1) as usize;
    let blink = (look.time * 0.45 + look.buoy_phase) % 4.0 < 0.7;
    let (ch, fg) = if blink { ('*', look.buoy_lit) } else { ('o', look.buoy_dim) };
    grid[y][bx] = Cell::new(ch, fg);
    if y + 1 < h {
        grid[y + 1][bx] = Cell::new('~', look.buoy_dim);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::grid_to_plain;
    use rand::{rngs::StdRng, SeedableRng};

    fn knobs() -> Vec<f32> {
        PARAMS.iter().map(|p| p.default).collect()
    }

    fn frame(w: usize, h: usize, seed: u64, time: f32, values: &[f32]) -> Grid {
        let mut grid = vec![vec![Cell::blank(); w]; h];
        let palette = crate::color::make_palette(seed);
        let mut rng = StdRng::seed_from_u64(seed);
        MODE.render(&mut ModeFrame {
            grid: &mut grid,
            width: w,
            height: h,
            seed,
            palette: &palette,
            rng: &mut rng,
            time,
            args: &[],
            param_values: Some(values),
        });
        grid
    }

    fn text(grid: &Grid) -> String {
        grid_to_plain(grid).join("\n")
    }

    #[test]
    fn beacon_seed42() {
        insta::assert_snapshot!("beacon_80x24", text(&frame(80, 24, 42, 0.0, &knobs())));
    }

    #[test]
    fn beacon_seed42_t6() {
        insta::assert_snapshot!("beacon_80x24_t6", text(&frame(80, 24, 42, 6.0, &knobs())));
    }

    #[test]
    fn deterministic_and_seed_sensitive() {
        let k = knobs();
        assert_eq!(text(&frame(90, 30, 42, 0.0, &k)), text(&frame(90, 30, 42, 0.0, &k)));
        assert_ne!(text(&frame(90, 30, 42, 0.0, &k)), text(&frame(90, 30, 7, 0.0, &k)));
    }

    #[test]
    fn time_moves_the_scene() {
        let k = knobs();
        assert_ne!(text(&frame(90, 30, 42, 0.0, &k)), text(&frame(90, 30, 42, 4.0, &k)));
    }

    #[test]
    fn knobs_change_the_scene() {
        let mut k = knobs();
        let a = text(&frame(90, 30, 42, 0.0, &k));
        k[2] = 0.45;
        assert_ne!(a, text(&frame(90, 30, 42, 0.0, &k)));
        k[2] = 0.62;
        k[3] = 0.4;
        assert_ne!(a, text(&frame(90, 30, 42, 0.0, &k)));
    }

    #[test]
    fn small_dims_and_extrema_stay_bounded() {
        let lo: Vec<f32> = PARAMS.iter().map(|p| p.min).collect();
        let hi: Vec<f32> = PARAMS.iter().map(|p| p.max).collect();
        for k in [&lo, &hi] {
            for (w, h) in [(12usize, 6usize), (40, 10), (80, 24), (200, 60)] {
                let g = frame(w, h, 42, 3.0, k);
                assert_eq!(g.len(), h);
                assert!(g.iter().all(|row| row.len() == w));
            }
        }
        let g = frame(9, 2, 7, 1.0, &knobs());
        assert_eq!(g.len(), 2);
        let g = frame(2, 1, 7, 1.0, &knobs());
        assert_eq!(g.len(), 1);
    }

    #[test]
    fn frame_cost() {
        let (w, h) = (200usize, 60usize);
        let k = knobs();
        let mut worst = 0.0f64;
        let start = std::time::Instant::now();
        for f in 0..100 {
            let t0 = std::time::Instant::now();
            frame(w, h, 42, f as f32 * 0.05, &k);
            worst = worst.max(t0.elapsed().as_secs_f64() * 1000.0);
        }
        let avg = start.elapsed().as_secs_f64() * 1000.0 / 100.0;
        eprintln!("beacon frame_cost 200x60: avg {:.3} ms, worst {:.3} ms", avg, worst);
        if !cfg!(debug_assertions) {
            assert!(avg < 6.0, "avg frame {:.3} ms", avg);
        }
    }
}
