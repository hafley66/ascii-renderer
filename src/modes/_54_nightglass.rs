//! Nightglass: rain on a window over a sleeping city. One reusable RGBA scratch
//! holds the outside scene; a compose pass turns it into cells, then lens passes
//! refract it through the condensation. No state is kept between frames.
use crate::_0_profile::measure_layer;
use crate::color::{darken, lerp_color, lighten, rgb, shift_hue};
use crate::opts::param_f32;
use crate::registry::{AnimKind, Mode, ModeFrame, Param};
use crate::types::{Cell, Grid};
use crossterm::style::Color;
use rayon::prelude::*;
use std::cell::RefCell;
use std::f32::consts::{PI, TAU};
use std::sync::LazyLock;

pub(super) struct Nightglass;
pub(super) static MODE: Nightglass = Nightglass;

const NAME: &str = "nightglass";
const KNOBS: usize = 12;
const HELP: &str = "nightglass: rain on a window over city bokeh [lights] [glow] [bokeh] [drops] [slide] [rain] [wind] [haze] [glass] [twinkle] [drift] [sweep]";

/// Light is drawn as solid shade, so a lamp reads as a lamp and not as a speck.
const LAMP: [char; 3] = ['▒', '▓', '█'];
/// Rain dashes, thin to solid, and the leaning pair for wind.
const STEEP: [char; 3] = ['╎', '│', '┊'];
const SLANT: [char; 2] = ['/', '\\'];
/// Rim marks on droplets and specular glints on the glass.
const CORE: [char; 2] = ['·', '∙'];
const GLINT: [char; 3] = ['·', '+', '✦'];

const L_LIGHT: u64 = 0x02;
const L_RAIN: u64 = 0x03;
const L_BEAD: u64 = 0x04;
const L_SLIDE: u64 = 0x05;
const L_SPARK: u64 = 0x06;
const L_ROOF: u64 = 0x07;
const L_GLIT: u64 = 0x08;

const K_DISC: u8 = 0;
const K_COLUMN: u8 = 1;
const K_SPARK: u8 = 3;

const WAVE_SIZE: usize = 2048;
const WAVE_MASK: usize = WAVE_SIZE - 1;

/// One period of sine, built once: fog, drift and shimmer phases then cost a
/// multiply, a truncating convert, a mask and a load instead of a libm call.
static WAVE: LazyLock<[f32; WAVE_SIZE]> =
    LazyLock::new(|| std::array::from_fn(|i| (i as f32 * TAU / WAVE_SIZE as f32).sin()));

/// Grids at or above this cell count run their row passes on rayon's pool.
const PARALLEL_MIN_CELLS: usize = 20_480;

#[inline(always)]
fn shade_rows<F>(grid: &mut Grid, w: usize, h: usize, row: F)
where
    F: Fn((usize, &mut Vec<Cell>)) + Sync + Send,
{
    let rows = grid.len().min(h);
    let slice = &mut grid[..rows];
    if w * h >= PARALLEL_MIN_CELLS {
        slice
            .par_iter_mut()
            .enumerate()
            .with_min_len(8)
            .for_each(row);
    } else {
        slice.iter_mut().enumerate().for_each(row);
    }
}

#[inline(always)]
fn fill_rows<F>(field: &mut [[f32; 4]], w: usize, h: usize, row: F)
where
    F: Fn((usize, &mut [[f32; 4]])) + Sync + Send,
{
    if w == 0 {
        return;
    }
    if w * h >= PARALLEL_MIN_CELLS {
        field
            .par_chunks_mut(w)
            .enumerate()
            .with_min_len(8)
            .for_each(row);
    } else {
        field.chunks_mut(w).enumerate().for_each(row);
    }
}

const PARAMS: &[Param] = &[
    param!("LIGHTS", "city lights", 6.0, 90.0, 26.0, 2.0),
    param!("GLOW", "halo gain", 0.0, 1.8, 1.05, 0.05),
    param!("BOKEH", "defocus size", 0.35, 2.0, 1.0, 0.05),
    param!("DROPS", "condensation", 0.0, 2.2, 1.0, 0.1),
    param!("SLIDE", "running drops", 0.0, 2.0, 1.0, 0.1),
    param!("RAIN", "falling rain", 0.0, 2.0, 0.9, 0.1),
    param!("WIND", "rain lean", -1.0, 1.0, 0.3, 0.05),
    param!("HAZE", "window fog", 0.0, 1.0, 0.42, 0.05),
    param!("GLASS", "lens power", 0.0, 1.6, 1.0, 0.05),
    param!("TWINKLE", "glints", 0.0, 1.0, 0.55, 0.05),
    param!("DRIFT", "light drift", 0.0, 2.0, 0.6, 0.05),
    param!("SWEEP", "passing light", 0.0, 1.5, 0.8, 0.05),
];

impl Mode for Nightglass {
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
        // Positional arguments, then native live values, then env defaults; every
        // knob is clamped so a wild roll or a hostile environment still draws.
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

thread_local! {
    static SCRATCH: RefCell<Vec<[f32; 4]>> = const { RefCell::new(Vec::new()) };
}

fn draw(frame: &mut ModeFrame<'_>, p: &[f32; KNOBS]) {
    let (w, h) = (frame.width, frame.height);
    if w == 0 || h == 0 {
        return;
    }
    let look = Look::new(frame.seed, w, h, frame.palette, frame.time, p);
    SCRATCH.with(|slot| {
        let mut buf = slot.borrow_mut();
        if buf.len() < w * h {
            buf.resize(w * h, [0.0; 4]);
        }
        // The scratch keeps its allocation between frames, so a reshaped terminal
        // grows it once and animation never reallocates.
        let field = &mut buf[..w * h];
        measure_layer(NAME, "sky", || fill_sky(field, &look));
        measure_layer(NAME, "city", || splat_city(field, &look));
        measure_layer(NAME, "rain", || fill_rain(field, &look));
        measure_layer(NAME, "compose", || compose(frame.grid, field, &look));
        measure_layer(NAME, "beads", || draw_beads(frame.grid, field, &look));
        measure_layer(NAME, "drops", || draw_drops(frame.grid, field, &look));
        measure_layer(NAME, "sheen", || draw_sheen(frame.grid, &look));
    });
}

/// Splitmix64 over (seed, layer, index, slot): every scene element and every
/// animation phase comes from here, so no rng stream is consumed or ordered.
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
fn span(h: u64, a: f32, b: f32) -> f32 {
    a + (b - a) * unit(h)
}

#[inline]
fn to_lin(c: Color) -> [f32; 3] {
    match c {
        Color::Rgb { r, g, b } => [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0],
        _ => [0.0, 0.0, 0.0],
    }
}

#[inline]
fn tm(x: f32) -> f32 {
    let x = x.max(0.0);
    x / (1.0 + x)
}

#[inline]
fn tm8(x: f32, gain: f32) -> u8 {
    (255.0 * tm(x * gain)).clamp(0.0, 255.0) as u8
}

#[inline]
fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[inline]
fn chan(c: Color, k: usize) -> u8 {
    match c {
        Color::Rgb { r, g, b } => match k {
            0 => r,
            1 => g,
            _ => b,
        },
        _ => 0,
    }
}

#[inline]
fn rgba(r: f32, g: f32, b: f32) -> Color {
    Color::Rgb {
        r: r.clamp(0.0, 255.0) as u8,
        g: g.clamp(0.0, 255.0) as u8,
        b: b.clamp(0.0, 255.0) as u8,
    }
}

/// One placed light: bokeh discs, rain-smeared columns, distant window clusters.
#[derive(Clone, Copy)]
struct Light {
    cx: f32,
    cy: f32,
    rx: f32,
    ry: f32,
    col: [f32; 3],
    bright: f32,
    kind: u8,
}

/// A drop running down the glass: its lens, its track length and its alpha.
#[derive(Clone, Copy)]
struct Drop {
    cx: f32,
    cy: f32,
    rx: f32,
    ry: f32,
    alpha: f32,
    tail: f32,
    salt: u64,
}

/// Everything a frame needs: geometry unit, resolved knobs and derived colors.
struct Look {
    seed: u64,
    w: usize,
    h: usize,
    time: f32,
    /// Characteristic light radius in cells; every scene feature scales with it.
    unit: f32,
    lane_w: f32,
    sky_top: [f32; 3],
    sky_bot: [f32; 3],
    city_glow: [f32; 3],
    haze_warm: [f32; 3],
    sky_glow: [f32; 3],
    haze_cool: [f32; 3],
    fog_col: [f32; 3],
    rain_col: [f32; 3],
    pale: Color,
    pale_lin: [f32; 3],
    warm: Color,
    cool: Color,
    rose: Color,
    hot: Color,
    exposure: f32,
    glow: f32,
    lights: usize,
    bead_count: usize,
    mist: f32,
    drops: usize,
    rain: f32,
    rain_gate: f32,
    rain_speed: f32,
    rain_len: f32,
    rain_lit: f32,
    wind: f32,
    haze: f32,
    glass: f32,
    twinkle: f32,
    drift: f32,
    sweep_x: f32,
    sweep_w: f32,
    sweep_a: f32,
    fog_ph: f32,
    /// Lit districts, resolved once: the sky pass reads them per cell.
    districts: [[f32; 2]; 3],
    /// Roofline tables: heights and roughness per skyline block, per ridge.
    roof: [[f32; 8]; 4],
    /// Halo reach cap: keeps the light pass a bounded share of the frame.
    bloom_cap: f32,
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
        // A wrong clock (NaN or absurd) is not worth a broken frame.
        let time = if time.is_finite() {
            time.clamp(0.0, 1.0e6)
        } else {
            0.0
        };
        let h_w = h as f32;
        let unit_r = (h as f32).powf(0.62) * 0.55 * p[2];
        // Bigger defocus means fewer lights: the pane holds a bounded amount of
        // glow, so the knob moves light around instead of spending more of it.
        let bokeh2 = (p[2] * p[2]).max(0.12);
        // A car passes outside once per cycle and is gone the rest of the time.
        let period = 26.0 / p[11].max(0.06);
        let pass = (time / period).fract() / 0.18;
        let (sweep_x, sweep_a) = if pass < 1.0 {
            (-0.35 + 1.7 * pass, (pass * PI).sin() * p[11] * 0.16)
        } else {
            (0.0, 0.0)
        };
        let warm = lerp_color(rgb(255, 172, 84), palette[3], 0.16);
        let cool = lerp_color(rgb(108, 178, 255), palette[2], 0.20);
        let rose = lerp_color(rgb(255, 104, 172), palette[4], 0.14);
        let hot = lerp_color(rgb(255, 244, 230), palette[4], 0.2);
        // Bokeh count is a knob, but a small terminal cannot hold a crowd.
        let crowd = ((w * h) as f32 / 2600.0).clamp(0.40, 1.0) / bokeh2;
        let lights = (p[0] * crowd).round().clamp(3.0, 220.0) as usize;
        Look {
            seed,
            w,
            h,
            time,
            unit: unit_r,
            lane_w: (unit_r * 0.5).clamp(2.2, 7.0),
            sky_top: to_lin(darken(lerp_color(palette[0], palette[2], 0.16), 30)),
            sky_bot: to_lin(lerp_color(palette[0], palette[2], 0.22)),
            city_glow: to_lin(lerp_color(warm, rgb(255, 150, 74), 0.35)),
            haze_warm: to_lin(lerp_color(warm, palette[3], 0.20)),
            sky_glow: to_lin(lerp_color(palette[2], rgb(126, 116, 205), 0.35)),
            haze_cool: to_lin(lerp_color(cool, palette[2], 0.30)),
            fog_col: to_lin(lerp_color(palette[4], palette[2], 0.35)),
            rain_col: to_lin(lerp_color(palette[4], palette[2], 0.45)),
            pale: lerp_color(palette[4], rgb(214, 230, 255), 0.45),
            pale_lin: to_lin(lerp_color(palette[4], rgb(214, 230, 255), 0.45)),
            warm,
            cool,
            rose,
            hot,
            exposure: 0.82 + 0.42 * p[1],
            glow: p[1],
            lights,
            // Beads thin out with size: a big pane cannot show a thousand lenses,
            // so the rest of the mist is carried by the glitter field instead.
            bead_count: ((w + h * 2) as f32 / 2.6 * p[3]).round().clamp(0.0, 800.0) as usize,
            mist: p[3],
            drops: (((w + h) as f32 / 30.0).clamp(3.0, 10.0) * p[4]).round() as usize,
            rain: p[5],
            rain_gate: (0.05 + 0.30 * p[5]).clamp(0.0, 1.0),
            rain_speed: h as f32 * 0.36 * (0.7 + 0.3 * p[5]),
            rain_len: unit_r * 3.2,
            rain_lit: 0.20 * p[5],
            wind: p[6],
            haze: p[7],
            glass: p[8],
            twinkle: p[9],
            drift: p[10],
            sweep_x,
            sweep_w: (w as f32 * 0.16).max(3.0),
            sweep_a,
            fog_ph: unit(hash(seed, 0xF0, 0, 0)) * TAU,
            districts: std::array::from_fn(|c| {
                let h = hash(seed, L_LIGHT, 900 + c as u64, 17);
                [
                    span(h, 0.10, 0.90) * w as f32,
                    span(h ^ 0x5A5A, 0.26, 0.60) * h_w,
                ]
            }),
            roof: {
                let mut t = [[0.0; 8]; 4];
                for b in 0..8u64 {
                    let far = hash(seed, L_ROOF, b, 1);
                    let near = hash(seed, L_ROOF, 256 + b, 2);
                    t[0][b as usize] = span(far, 0.54, 0.88) * h_w;
                    t[1][b as usize] = span(far ^ 0x1F1F, -0.012, 0.012) * h_w;
                    t[2][b as usize] = span(near, 0.70, 0.99) * h_w;
                    t[3][b as usize] = span(near ^ 0x1F1F, -0.012, 0.012) * h_w;
                }
                t
            },
            // Halo work is capped to a fixed share of the frame, so neither a
            // large terminal nor a big defocus can make the light pass unbounded.
            bloom_cap: ((w * h) as f32 * 1.5 / (4.0 * lights as f32).max(1.0))
                .sqrt()
                .clamp(1.5, 96.0),
        }
    }

    #[inline]
    fn sin_of(&self, x: f32) -> f32 {
        WAVE[(((x * (WAVE_SIZE as f32 / TAU)) as i32) as usize) & WAVE_MASK]
    }

    #[inline]
    fn at(&self, layer: u64, index: u64, slot: u64) -> u64 {
        hash(self.seed, layer, index, slot)
    }

    /// Sky base for one row: a night gradient with the city glowing below. The
    /// glow band carries its own horizontal variation, so districts read as zones.
    fn sky_row(&self, y: usize, zone: f32) -> [f32; 3] {
        let t = if self.h > 1 {
            y as f32 / (self.h - 1) as f32
        } else {
            0.0
        };
        let g = smoothstep(t * 0.9);
        // Squared parabola stands in for a gaussian: cheap and soft enough.
        // Light pollution hugs the horizon; a violet sky glow sits above it.
        let warm_band = (1.0 - ((t - 0.86) / 0.42).powi(2)).max(0.0);
        let warm_band = warm_band * warm_band.sqrt() * 0.52;
        let sky_band = (1.0 - ((t - 0.58) / 0.62).powi(2)).max(0.0);
        let sky_band = sky_band * sky_band * 0.30;
        let mut c = [0.0; 3];
        for k in 0..3 {
            c[k] = self.sky_top[k]
                + (self.sky_bot[k] - self.sky_top[k]) * g * 0.7
                + self.city_glow[k] * warm_band * (0.62 + 0.55 * zone * 0.9)
                + self.sky_glow[k] * sky_band;
        }
        c
    }

    /// Three lit districts lift the sky around them: the atmospheric glow that
    /// sits behind a city at night, and the frame's main colour blocks.
    #[inline]
    fn haze_glow(&self, x: usize, y: usize, cool: bool) -> f32 {
        let xf = x as f32;
        let yf = y as f32 * 0.55;
        let mut sum = 0.0;
        for c in 0..3 {
            if (c == 1) != cool {
                continue;
            }
            let d = &self.districts[c];
            let dx = xf - d[0];
            let dy = yf - d[1] * 0.55;
            sum += 1.0 / (1.0 + (dx * dx + dy * dy) * 0.0042);
        }
        sum * 0.075
    }

    /// A passing light outside: it lights the fog and the falling rain.
    #[inline]
    fn sweep(&self, x: usize) -> f32 {
        if self.sweep_a <= 0.001 {
            return 0.0;
        }
        let u = (x as f32 - self.sweep_x * self.w as f32) / self.sweep_w;
        let f = 1.0 / (1.0 + u * u);
        f * f * self.sweep_a
    }

    /// Window fog: two crossed sines with a slow drift, the cheapest blobby field.
    #[inline]
    fn fog(&self, x: usize, y: usize) -> f32 {
        let a = self.sin_of(
            x as f32 * 0.115 + self.sin_of(y as f32 * 0.075 + self.fog_ph) * 2.4 + self.time * 0.02,
        );
        let b = self.sin_of(x as f32 * 0.031 - y as f32 * 0.048 + self.fog_ph * 1.7);
        (0.5 + 0.32 * a + 0.18 * b).clamp(0.0, 1.0)
    }

    /// Rain as a field: wind-leaned lanes of falling dashes, each with its own
    /// speed, length and head phase. Solved per cell, so there is nothing to store.
    #[inline]
    fn rain_at(&self, x: usize, y: usize) -> f32 {
        let yf = y as f32;
        let u = (x as f32 - self.wind * yf * 1.6) / self.lane_w;
        let lane = u.floor();
        let key = lane as i64 as u64;
        if unit(self.at(L_RAIN, key, 0)) > self.rain_gate {
            return 0.0;
        }
        let len = self.rain_len * span(self.at(L_RAIN, key, 1), 0.55, 1.45);
        let speed = self.rain_speed * span(self.at(L_RAIN, key, 2), 0.7, 1.35);
        let walk = self.h as f32 + len;
        let head =
            (self.time * speed + unit(self.at(L_RAIN, key, 3)) * walk).rem_euclid(walk) - len * 0.5;
        let d = head - yf;
        if d < 0.0 || d > len {
            return 0.0;
        }
        let side = (u - lane) - 0.5 - (unit(self.at(L_RAIN, key, 4)) - 0.5) * 0.8;
        // One cell wide per row, with soft shoulders, so a dash reads as a stroke.
        let across = side.abs() * self.lane_w;
        if across > 0.95 {
            return 0.0;
        }
        let lateral = ((0.95 - across) / 0.4).min(1.0);
        lateral * (0.34 + 0.66 * (1.0 - d / len)) * self.rain
    }

    /// Height of one skyline ridge at column `x`. A far ridge and a near one, both
    /// quantised into blocks, so the pane has depth instead of one straight line.
    #[inline]
    fn ridge(&self, x: usize, seg: f32, row: usize) -> f32 {
        let block = ((x as f32 / (self.w as f32 / seg).max(2.0)) as usize).min(7);
        self.roof[row][block] + self.roof[row + 1][block]
    }

    #[inline]
    fn roof(&self, x: usize, y: usize) -> f32 {
        let yf = y as f32;
        let far_mask = smoothstep((yf - self.ridge(x, 7.0, 0)) / 2.0);
        let near_mask = smoothstep((yf - self.ridge(x, 5.0, 2)) / 2.0);
        (far_mask * 0.55 + near_mask * 0.95).min(1.0)
    }
}

impl Look {
    /// The pane's subjects: two large drops, low and off-centre, the way a camera
    /// catches the nearest beads. Their periods differ, so one stays on the glass.
    fn hero(&self, k: usize) -> (f32, f32, f32) {
        let salt = 900 + k as u64;
        let (lo, hi) = if k == 0 { (0.20, 0.36) } else { (0.56, 0.74) };
        let hx = span(self.at(L_SLIDE, salt, 0), lo, hi);
        let hy = span(self.at(L_SLIDE, salt, 1), 0.32, 0.56);
        let scale = if k == 0 { 0.115 } else { 0.085 };
        let r = self.h as f32 * (scale + 0.04 * unit(self.at(L_SLIDE, salt, 2)));
        (hx * self.w as f32, hy * self.h as f32, r)
    }

    /// True where a running drop has already swept the glass clean this cycle.
    #[inline]
    fn wiped(&self, x: f32, y: f32) -> bool {
        let n = if self.drops > 14 { 14 } else { self.drops };
        for i in 0..n {
            let d = self.drop(i);
            // Only a drop wide enough to matter clears a track worth seeing.
            if d.rx < 1.6 {
                continue;
            }
            if y < d.cy && (x - d.cx).abs() < d.rx * 0.85 {
                return true;
            }
        }
        false
    }

    fn light(&self, i: usize) -> Light {
        let hi = i as u64;
        let depth = unit(self.at(L_LIGHT, hi, 0));
        let kind_roll = unit(self.at(L_LIGHT, hi, 1));
        let kind = if kind_roll < 0.74 {
            K_DISC
        } else if kind_roll < 0.88 {
            K_COLUMN
        } else {
            K_SPARK
        };
        let size = unit(self.at(L_LIGHT, hi, 2));
        let rx = if kind == K_COLUMN {
            self.unit * (0.13 + 0.13 * size)
        } else {
            self.unit * (0.50 + 0.90 * size * size) * (0.55 + 0.75 * depth)
        };
        let hero = i < 3;
        let rx = (if hero { rx * 1.35 } else { rx }).max(0.35);
        let ry = if kind == K_COLUMN {
            rx * (2.6 + 3.2 * unit(self.at(L_LIGHT, hi, 3)))
        } else {
            rx * 0.5
        };
        // Hue: mostly warm street light, some cold signage, a few blown-out points.
        let family = unit(self.at(L_LIGHT, hi, 4));
        let base = if family < 0.52 {
            self.warm
        } else if family < 0.74 {
            self.cool
        } else if family < 0.88 {
            self.rose
        } else {
            self.hot
        };
        let tint = shift_hue(base, span(self.at(L_LIGHT, hi, 5), -24.0, 24.0) as f64);
        // Defocus spreads the same light: a bigger blob is a dimmer one.
        let bright = span(self.at(L_LIGHT, hi, 6), 0.6, 1.5)
            * (0.55 + 0.45 * depth)
            * (1.25 - 0.5 * size)
            * (if hero { 1.2 } else { 1.0 });
        // Slow sway with parallax that grows with depth, plus a pulse; a few
        // lights buzz like failing neon.
        let sway = self.drift * (0.6 + 2.6 * depth) * self.unit * 0.075;
        // Lights gather into districts: a city is not evenly lit, and the
        // clustering is most of the composition.
        let (mut cx, mut cy, spread) = if unit(self.at(L_LIGHT, hi, 13)) < 0.68 {
            let d = self.districts[(unit(self.at(L_LIGHT, hi, 14)) * 2.999) as usize];
            let jx = (unit(self.at(L_LIGHT, hi, 7)) + unit(self.at(L_LIGHT, hi, 15)) - 1.0)
                * self.w as f32
                * 0.30;
            let jy = (unit(self.at(L_LIGHT, hi, 9)) + unit(self.at(L_LIGHT, hi, 16)) - 1.0)
                * self.h as f32
                * 0.22;
            (d[0] + jx, d[1] + jy, 1.0)
        } else {
            (
                (unit(self.at(L_LIGHT, hi, 7)) * 1.2 - 0.1) * self.w as f32,
                self.h as f32 * (0.20 + 0.52 * unit(self.at(L_LIGHT, hi, 9))),
                1.0,
            )
        };
        let _ = spread;
        cx = cx.clamp(-(self.w as f32) * 0.08, self.w as f32 * 1.08)
            + sway
                * self.sin_of(self.time * span(self.at(L_LIGHT, hi, 8), 0.05, 0.22) + depth * TAU);
        cy = cy.clamp(self.h as f32 * 0.10, self.h as f32 * 0.74);
        let pulse = 1.0
            + self.twinkle
                * 0.28
                * self.sin_of(self.time * span(self.at(L_LIGHT, hi, 10), 0.4, 3.1) + family * TAU);
        let buzz = if unit(self.at(L_LIGHT, hi, 11)) > 0.90 && self.twinkle > 0.02 {
            let sq = self.sin_of(self.time * 21.0 + family * 8.0);
            0.86 + 0.14 * if sq > 0.0 { 1.0 } else { -1.0 }
        } else {
            1.0
        };
        Light {
            cx,
            cy,
            rx,
            ry,
            col: to_lin(tint),
            bright: (bright * pulse * buzz).max(0.0),
            kind,
        }
    }

    fn drop(&self, i: usize) -> Drop {
        let hi = i as u64;
        // The first two drops hang over real lights, so the still frame shows the
        // refraction doing something worth looking at.
        let (cx, want, rx) = if i < 2 {
            // The subjects: big beads over the two brightest clusters in the pane.
            let (hx, hy, hr) = self.hero(i);
            (hx, hy, hr)
        } else if i == 2 && self.lights > 6 {
            // Over a cluster of windows: the lens breaks a grid of small lights.
            let l = self.light(6);
            (
                l.cx,
                l.cy,
                self.unit * (0.52 + 0.5 * unit(self.at(L_SLIDE, hi, 0))),
            )
        } else {
            (
                unit(self.at(L_SLIDE, hi, 3)) * self.w as f32,
                self.h as f32 * span(self.at(L_SLIDE, hi, 1), 0.30, 0.85),
                self.unit * (0.42 + 0.55 * unit(self.at(L_SLIDE, hi, 0))),
            )
        };
        let tail = rx * if i < 2 { 2.0 } else { 3.0 };
        let stop = self.h as f32
            * if i < 2 {
                1.02
            } else {
                span(self.at(L_SLIDE, hi, 1), 0.55, 0.95)
            };
        // Age at t = 0: each subject is already sitting where it belongs.
        let phase = if i < 2 {
            ((want + tail) / (stop + tail)).clamp(0.0, 0.96)
        } else {
            unit(self.at(L_SLIDE, hi, 2))
        };
        let period = if i < 2 {
            // Two subjects, different clocks, so the pane is never left without one.
            (108.0 + 54.0 * unit(self.at(L_SLIDE, hi, 4))) * (1.0 + 0.35 * i as f32)
        } else {
            20.0 + 26.0 * unit(self.at(L_SLIDE, hi, 4))
        };
        let age = (self.time / period + phase).fract();
        let cy = -tail + age * (stop + tail);
        // Fade in as it enters and out where it stops, so the cycle never jumps.
        let alpha = smoothstep(age / 0.06).min(smoothstep((1.0 - age) / 0.12));
        Drop {
            cx,
            cy,
            rx,
            ry: rx * 0.62,
            alpha,
            tail,
            salt: hi,
        }
    }
}

fn fill_sky(field: &mut [[f32; 4]], look: &Look) {
    fill_rows(field, look.w, look.h, |(y, row)| {
        for (x, slot) in row.iter_mut().enumerate() {
            let sweep = look.sweep(x) * 1.3;
            // The pane itself carries a sheen, as if the glass caught one lamp
            // from off-frame: a slow diagonal that lifts the frame out of black.
            let sheen = 0.016
                * (1.0
                    - (((x as f32 / look.w as f32) - 0.18).powi(2)
                        + ((y as f32 / look.h as f32) - 0.86).powi(2))
                    .sqrt()
                    .min(1.0));
            // Districts vary both the band and the colour that sits over it.
            let zone = look.haze_glow(x, y, false);
            let cool = look.haze_glow(x, y, true) * 0.6;
            let sky = look.sky_row(y, zone);
            let warm = sweep + zone * 0.5;
            *slot = [
                sky[0] + look.haze_warm[0] * warm + look.haze_cool[0] * cool + sheen * 1.1,
                sky[1] + look.haze_warm[1] * warm + look.haze_cool[1] * cool + sheen,
                sky[2] + look.haze_warm[2] * warm + look.haze_cool[2] * cool + sheen * 0.9,
                sweep * 0.22,
            ];
        }
    });
}

/// One additive light: a halo that reaches past the disc, plus a core profile
/// that gives the disc its rim, its bar or its blown-out point.
#[allow(clippy::too_many_arguments)]
fn splat(
    field: &mut [[f32; 4]],
    look: &Look,
    cx: f32,
    cy: f32,
    rx: f32,
    ry: f32,
    col: [f32; 3],
    bright: f32,
    kind: u8,
) {
    let w = look.w;
    // A point light blooms well past its own disc, a disc stops at its skirt, and
    // a column's box has to cover the whole bar.
    let tiny = (3.2 - rx.min(ry)).clamp(0.0, 1.0);
    let bloom = (rx * 2.8 + 1.0).max((look.unit * 1.6).min(look.bloom_cap) * tiny);
    let reach_x = bloom.max(rx * 2.95);
    let reach_y = bloom.max(ry * 2.95);
    let x0 = (cx - reach_x).floor().max(0.0) as usize;
    let x1 = ((cx + reach_x).ceil()).clamp(0.0, w as f32 - 1.0) as usize;
    let y0 = (cy - reach_y).floor().max(0.0) as usize;
    let y1 = ((cy + reach_y).ceil()).clamp(0.0, look.h as f32 - 1.0) as usize;
    if x1 < x0 || y1 < y0 {
        return;
    }
    let lit_gain = bright * look.glow;
    for y in y0..=y1 {
        let dy = (y as f32 + 0.5 - cy) / ry;
        for x in x0..=x1 {
            let dx = (x as f32 + 0.5 - cx) / rx;
            let d2 = dx * dx + dy * dy;
            if d2 > 8.4 {
                continue;
            }
            let f = &mut field[y * w + x];
            // Light in rain blurs wide: a point light carries a long, soft bloom,
            // while a disc keeps a flat middle and a skirt that stops sooner.
            let r_cells = d2.sqrt() * rx;
            let fade = {
                let t = (r_cells / bloom).min(1.0);
                (1.0 - t) * (1.0 - t)
            };
            let halo = if kind == K_DISC || kind == K_COLUMN {
                let plateau = if kind == K_DISC { 0.5 } else { 1.0 };
                if d2 <= 1.0 {
                    plateau * (1.0 + 0.22 * (1.0 - d2))
                } else {
                    let h = 1.0 / (1.0 + 0.9 * (d2 - 1.0));
                    h * h * plateau
                }
            } else {
                (1.0 / (1.0 + 0.35 * d2)) * fade
            };
            // Broad shapes ring, small ones fill: below a few cells a "ring" is
            // the whole light anyway.
            let core = if kind == K_COLUMN {
                (1.0 - dx.abs() * 0.7).clamp(0.0, 1.0) * (1.0 - dy.abs() * 0.62).clamp(0.0, 1.0)
            } else {
                // A rim about a cell thick, just inside the boundary and hotter
                // than the middle, so a light reads as a circle, not a fuzzy ball.
                let inside = (1.0 - d2.sqrt()) * rx.min(ry);
                let rim = (1.0 - ((inside - 0.55) / 0.78).powi(2)).max(0.0);
                let point = (1.0 - d2 * 1.35).max(0.0);
                rim * (1.0 - tiny) + point * tiny
            };
            let lit = lit_gain * halo;
            let body = core * bright;
            // Light fringes at the rim, the way a real lens splits colour there.
            let fringe = 0.16 * body;
            f[0] += col[0] * (lit + body * 0.95) + col[2] * fringe;
            f[1] += col[1] * (lit + body * 0.95) + col[1] * fringe;
            f[2] += col[2] * (lit + body * 0.95) + col[0] * fringe;
            // Only a bright lamp takes a glyph: a disc's edge is carried by
            // brightness, which keeps a pale street from becoming a wall of tiles.
            let mark = if tiny > 0.5 { body * 1.25 - 0.55 } else { 0.0 };
            if mark > f[3] {
                f[3] = mark;
            }
        }
    }
}

fn splat_city(field: &mut [[f32; 4]], look: &Look) {
    for i in 0..look.lights {
        let l = look.light(i);
        if l.bright <= 0.02 {
            continue;
        }
        splat(field, look, l.cx, l.cy, l.rx, l.ry, l.col, l.bright, l.kind);
    }
    for k in 0..2usize {
        hero_cluster(field, look, k);
    }
    shade_roofs(field, look);
    splat_panes(field, look);
}

/// The cluster one subject drop sits over, and the reason the lens has an image
/// inside it: a neon bar plus lamps are shapes a lens can invert.
fn hero_cluster(field: &mut [[f32; 4]], look: &Look, k: usize) {
    let (hx, hy, hr) = look.hero(k);
    let salt = 700 + k as u64 * 8;
    splat(
        field,
        look,
        hx - hr * 0.10,
        hy - hr * 0.22,
        hr * 0.16,
        hr * 1.15,
        to_lin(if k == 0 { look.rose } else { look.cool }),
        span(look.at(L_LIGHT, salt + 3, 3), 0.22, 0.38),
        K_COLUMN,
    );
    for j in 0..3u64 {
        let a = unit(look.at(L_LIGHT, salt + j, 0)) * TAU;
        let d = hr * (0.20 + 0.45 * unit(look.at(L_LIGHT, salt + j, 1)));
        let r = hr * (0.09 + 0.12 * unit(look.at(L_LIGHT, salt + j, 2)));
        let col = to_lin(if j == 0 {
            look.warm
        } else if j == 1 {
            look.cool
        } else {
            look.hot
        });
        let b = span(look.at(L_LIGHT, salt + j, 3), 0.20, 0.42);
        splat(
            field,
            look,
            hx + a.cos() * d,
            hy + a.sin() * d * 0.55,
            r,
            r * 0.7,
            col,
            b,
            K_DISC,
        );
    }
}

/// Lit windows inside the dark masses: a few panes in a loose grid per building.
/// They are drawn after the silhouettes, so they survive the clipping.
fn splat_panes(field: &mut [[f32; 4]], look: &Look) {
    let blocks = (look.w / 26).clamp(3, 12) as u64;
    for b in 0..blocks {
        let salt = 4_000 + b;
        let bx = span(look.at(L_ROOF, salt, 0), 0.04, 0.96) * look.w as f32;
        let by = span(look.at(L_ROOF, salt, 1), 0.68, 0.95) * look.h as f32;
        let cols = 2 + (unit(look.at(L_ROOF, salt, 2)) * 2.0) as i32;
        let rows = 1 + (unit(look.at(L_ROOF, salt, 3)) * 3.0) as i32;
        let col = to_lin(if unit(look.at(L_ROOF, salt, 4)) < 0.72 {
            look.warm
        } else {
            look.cool
        });
        for r in 0..rows {
            for c in 0..cols {
                let hh = look.at(L_ROOF, salt, 10 + (r * 4 + c) as u64);
                if unit(hh) < 0.34 {
                    continue;
                }
                let cell = span(hh, 0.22, 0.62);
                splat(
                    field,
                    look,
                    bx + c as f32 * 2.1,
                    by + r as f32 * 1.7,
                    0.62,
                    0.5,
                    col,
                    cell,
                    K_SPARK,
                );
            }
        }
    }
}

/// Row-safe silhouette pass: it only ever writes the row it is handed.
fn shade_roofs(field: &mut [[f32; 4]], look: &Look) {
    fill_rows(field, look.w, look.h, |(y, row)| {
        for (x, slot) in row.iter_mut().enumerate() {
            let far = look.ridge(x, 7.0, 0);
            let near = look.ridge(x, 5.0, 2);
            let yf = y as f32;
            let fm = smoothstep((yf - far) / 2.0);
            let nm = smoothstep((yf - near) / 2.0);
            if fm <= 0.001 && nm <= 0.001 {
                continue;
            }
            // The far ridge is a softer, lighter mass; the near one is almost black.
            let keep = (1.0 - 0.50 * fm) * (1.0 - 0.96 * nm);
            for k in 0..3 {
                slot[k] *= keep;
            }
            slot[3] *= keep;
            // Wet parapets catch the city glow along both ridges.
            let edge = ((1.0 - (fm - 0.42).abs() * 3.4).max(0.0)
                + (1.0 - (nm - 0.42).abs() * 3.4).max(0.0))
                * 0.10
                * look.glow;
            let edge = edge * edge;
            slot[0] += look.city_glow[0] * edge;
            slot[1] += look.city_glow[1] * edge;
            slot[2] += look.city_glow[2] * edge;
        }
    });
}

fn fill_rain(field: &mut [[f32; 4]], look: &Look) {
    if look.rain <= 0.002 {
        return;
    }
    fill_rows(field, look.w, look.h, |(y, row)| {
        for (x, slot) in row.iter_mut().enumerate() {
            let r = look.rain_at(x, y);
            if r <= 0.012 {
                continue;
            }
            // A streak is only as visible as the light around it: rain shows in
            // the city glow and almost vanishes against the black sky.
            let local = ((slot[0] + slot[1] + slot[2]) * 0.6).min(1.0);
            let glow = r * look.rain_lit * (1.0 + 2.6 * look.sweep(x));
            slot[0] += look.rain_col[0] * glow;
            slot[1] += look.rain_col[1] * glow;
            slot[2] += look.rain_col[2] * glow;
            // Rain writes the negative half of the channel: a dash is a stroke,
            // not a lamp, and the strongest dash on a cell wins.
            let mark = -r * 1.15 * (0.28 + 0.95 * local);
            if mark < slot[3] {
                slot[3] = mark;
            }
        }
    });
}

/// Glyph for a feature. The sign of the level picks the family: positive is light
/// and fills the cell with shade, negative is a rain dash and draws a stroke.
#[inline]
fn core_glyph(level: f32, lean: f32) -> char {
    if level < -0.14 {
        let stroke = -level;
        if lean.abs() > 0.35 {
            return SLANT[if lean > 0.0 { 1 } else { 0 }];
        }
        return if stroke > 0.70 {
            STEEP[2]
        } else if stroke > 0.42 {
            STEEP[1]
        } else {
            STEEP[0]
        };
    }
    if level < 0.16 {
        return ' ';
    }
    if level < 0.42 {
        return LAMP[0];
    }
    if level < 0.74 {
        return LAMP[1];
    }
    LAMP[2]
}

/// Droplet mist glitter: sub-cell beads are not resolvable, so the mist is a
/// per-cell shimmer that only shows where there is light to catch.
#[inline]
fn glitter(x: usize, y: usize, look: &Look, glow: f32) -> f32 {
    if look.mist <= 0.002 {
        return 0.0;
    }
    let h = hash(look.seed, L_GLIT, (y * look.w + x) as u64, 0);
    let kick = (h & 0xFFFF) as f32 * (1.0 / 65_536.0);
    let amount = (0.10 + 0.9 * kick) * (0.10 + 0.9 * glow) * look.mist;
    let phase = unit(h >> 8) * TAU;
    let rate = 0.5 + 2.2 * unit(h >> 24);
    amount * (0.35 + 0.65 * (0.5 + 0.5 * look.sin_of(look.time * rate + phase)))
}

/// The composite: one field cell plus its position become a drawable cell. Lens
/// passes sample through this, so refraction moves light, fog and glyphs together.
#[inline]
fn comp(f: &[f32; 4], x: usize, y: usize, look: &Look) -> Cell {
    let (mut r, mut g, mut b) = (f[0], f[1], f[2]);
    let level = f[3];
    if look.haze > 0.002 {
        let fog = look.fog(x, y);
        let lift = look.haze * fog * (0.022 + 0.085 * (r + g + b));
        r += lift * look.fog_col[0];
        g += lift * look.fog_col[1];
        b += lift * look.fog_col[2];
    }
    let spread = (r + g + b).min(1.6);
    let mist = glitter(x, y, look, spread);
    if mist > 0.0 {
        // Mist scatters a pale light of its own, weighted by what is behind it.
        r += mist * (0.035 + 0.16 * spread) * look.pale_lin[0];
        g += mist * (0.035 + 0.16 * spread) * look.pale_lin[1];
        b += mist * (0.035 + 0.16 * spread) * look.pale_lin[2];
    }
    let gain = look.exposure;
    let bg = Color::Rgb {
        r: tm8(r, gain),
        g: tm8(g, gain),
        b: tm8(b, gain),
    };
    let fg = lerp_color(
        Color::Rgb {
            r: tm8(r, gain * 2.6),
            g: tm8(g, gain * 2.6),
            b: tm8(b, gain * 2.6),
        },
        look.pale,
        (level.abs() * 0.85).min(0.8),
    );
    Cell::with_bg(core_glyph(level, look.wind), fg, bg)
}

#[inline]
fn sample(field: &[[f32; 4]], look: &Look, x: f32, y: f32) -> Cell {
    let xi = (x.round().max(0.0) as usize).min(look.w.saturating_sub(1));
    let yi = (y.round().max(0.0) as usize).min(look.h.saturating_sub(1));
    comp(&field[yi * look.w + xi], xi, yi, look)
}

fn compose(grid: &mut Grid, field: &[[f32; 4]], look: &Look) {
    let w = look.w;
    shade_rows(grid, w, look.h, |(y, row)| {
        for (x, cell) in row.iter_mut().enumerate() {
            *cell = comp(&field[y * w + x], x, y, look);
        }
    });
}

#[inline]
fn blend(dst: &mut Cell, src: &Cell, t: f32) {
    if t >= 0.995 {
        *dst = *src;
        return;
    }
    dst.fg = lerp_color(dst.fg, src.fg, t);
    dst.bg = lerp_color(dst.bg, src.bg, t);
    if t > 0.45 {
        dst.ch = src.ch;
    }
}

/// Draw one lens: a droplet that refracts the scene behind it. The inside is an
/// inverted, shrunken sample; the rim gathers light and the crown is shaded.
#[allow(clippy::too_many_arguments)]
fn lens(
    grid: &mut Grid,
    field: &[[f32; 4]],
    look: &Look,
    cx: f32,
    cy: f32,
    rx: f32,
    ry: f32,
    alpha: f32,
    power: f32,
    rim: f32,
    glint: bool,
) {
    if alpha <= 0.012 || rx < 0.22 {
        return;
    }
    let w = look.w as f32;
    let h = look.h as f32;
    let x0 = (cx - rx - 0.5).floor().max(0.0) as usize;
    let x1 = ((cx + rx + 0.5).ceil()).clamp(0.0, w - 1.0) as usize;
    let y0 = (cy - ry - 0.5).floor().max(0.0) as usize;
    let y1 = ((cy + ry + 0.5).ceil()).clamp(0.0, h - 1.0) as usize;
    if x1 < x0 || y1 < y0 {
        return;
    }
    let pull = 0.60 * look.glass * power;
    // Each bead catches the light at its own angle, so the mosaic shimmers.
    let spin = (unit(hash(
        look.seed,
        0,
        (cx as i64 as u64) ^ (cy as i64 as u64) << 20,
        1,
    )) - 0.5)
        * 1.5;
    let gather = 0.30 * look.glass * power;
    let sheen = 148.0 * rim * look.glass;
    for y in y0..=y1 {
        let v = (y as f32 + 0.5 - cy) / ry;
        for x in x0..=x1 {
            let u = (x as f32 + 0.5 - cx) / rx;
            let d2 = u * u + v * v;
            if d2 >= 1.0 {
                continue;
            }
            let d = d2.sqrt();
            let a = alpha * ((1.0 - d) * 7.0).min(1.0);
            if a <= 0.01 {
                continue;
            }
            // Inverted sample, bent hardest near the rim: a drop is a fisheye, and
            // the bend is what makes the glass read as glass.
            let warp = pull * (0.45 + 0.75 * d2);
            // A flip alone is invisible in a symmetric scene: the lens also shifts
            // what it shows, which is how a drop betrays itself in a photograph.
            let src = sample(
                field,
                look,
                cx - u * rx * warp + rx * 0.24 * look.glass * power,
                cy - v * ry * warp + ry * 0.12 * look.glass * power,
            );
            // A drop gathers light, and it burns brightest where it focuses it:
            // a caustic in the lower half, under a rim that catches the sky.
            let lift = 1.0 + gather + 0.55 * look.glass * power * (1.0 - d);
            let band = (1.0 - ((d - 0.90) / 0.24).powi(2)).max(0.0);
            let ring =
                band * (0.5 + 0.5 * (v * spin.cos() + u * spin.sin()).clamp(-1.0, 1.0)) * sheen;
            let veil = 9.0 * look.glass * power * (0.55 + 0.45 * (1.0 - d));
            // The upper edge of a drop bends light away: it reads as a dark line.
            let shade = band * (0.35 + 0.5 * (0.5 - 0.5 * v.clamp(-1.0, 1.0))) * 0.8;
            let caustic = (1.0 - ((d - 0.50) / 0.30).powi(2)).max(0.0)
                * (0.20 + 0.80 * v.clamp(0.0, 1.0))
                * 48.0
                * look.glass
                * power;
            let mut cell = src;
            let dim = 1.0 - shade;
            cell.bg = rgba(
                (chan(src.bg, 0) as f32 * lift + ring + caustic + veil * 0.92) * dim,
                (chan(src.bg, 1) as f32 * lift + ring * 0.96 + caustic * 0.97 + veil * 0.96) * dim,
                (chan(src.bg, 2) as f32 * lift + ring * 0.88 + caustic * 0.92 + veil) * dim,
            );
            cell.fg = rgba(
                chan(src.fg, 0).max(chan(src.bg, 0)) as f32 * lift * 1.1
                    + ring * 1.5
                    + caustic * 0.6,
                chan(src.fg, 1).max(chan(src.bg, 1)) as f32 * lift * 1.1
                    + ring * 1.44
                    + caustic * 0.58,
                chan(src.fg, 2).max(chan(src.bg, 2)) as f32 * lift * 1.1
                    + ring * 1.32
                    + caustic * 0.54,
            );
            // A beaded pane stipples: sparse dots keep the glass readable even
            // without colour, and read as sparkle with it.
            let dither = hash(0, 0x4D1, (y * look.w + x) as u64, 3) % 6;
            if band > 0.72 && src.ch == ' ' && dither == 0 {
                cell.ch = if band > 0.88 { CORE[1] } else { CORE[0] };
            }
            blend(&mut grid[y][x], &cell, a);
        }
    }
    if glint && alpha > 0.5 {
        // The specular point where the sky reflects off the drop crown.
        let gx = (cx - rx * 0.36).round();
        let gy = (cy - ry * 0.52).round();
        if gx >= 0.0 && gy >= 0.0 && (gx as usize) < look.w && (gy as usize) < look.h {
            let cell = &mut grid[gy as usize][gx as usize];
            cell.ch = if rx > 2.4 { GLINT[1] } else { GLINT[0] };
            cell.fg = look.pale;
            cell.bg = lighten(cell.bg, 44);
        }
    }
}

fn draw_beads(grid: &mut Grid, field: &[[f32; 4]], look: &Look) {
    // Beads are laid down until their lens area would pass a fixed share of the
    // frame, so the knob is cheap at any size and any defocus.
    let budget = (look.w * look.h) as f32 * 0.11;
    let mut spent = 0.0;
    for i in 0..look.bead_count {
        let hi = i as u64;
        let px = unit(look.at(L_BEAD, hi, 0));
        let py = unit(look.at(L_BEAD, hi, 1));
        let pr = unit(look.at(L_BEAD, hi, 2));
        let pa = unit(look.at(L_BEAD, hi, 3));
        let pv = unit(look.at(L_BEAD, hi, 4));
        // A misted pane beads everywhere, finer where the glass is colder (up).
        let r = look.unit * (0.09 + 0.24 * pr) * (0.55 + 0.75 * py);
        if r < 0.18 {
            continue;
        }
        let bx = px * look.w as f32;
        let by = py * look.h as f32;
        spent += (2.4 * r + 2.0) * (1.5 * r + 2.0);
        if spent > budget {
            break;
        }
        // A bead that has already been wiped by a running drop is gone: the drop
        // leaves a clean track with its own small beads along it.
        if look.wiped(bx, by) {
            continue;
        }
        // Condensation breathes: beads swell, dry out, and come back.
        let period = 7.0 + 16.0 * pv;
        let breath = 0.55 + 0.45 * look.sin_of((look.time / period + pa) * TAU);
        let alpha = (0.28 + 0.64 * pa) * breath;
        lens(
            grid,
            field,
            look,
            bx,
            by,
            r,
            r * 0.62,
            alpha,
            0.7,
            0.75,
            false,
        );
    }
}

fn draw_drops(grid: &mut Grid, field: &[[f32; 4]], look: &Look) {
    for i in 0..look.drops {
        let d = look.drop(i);
        if d.alpha <= 0.012 {
            continue;
        }
        // The wet track above a running drop: the chain of beads it left behind.
        let gap = (d.rx * 1.30).max(1.5);
        let reach = ((d.cy + d.tail) / gap).clamp(0.0, 26.0);
        for j in 1..=(reach as usize) {
            let yj = d.cy - gap * j as f32;
            if yj < -1.0 {
                break;
            }
            // Bead identity follows the pane, not the drop, so a track looks left behind.
            let key = d.salt
                ^ (yj.round() as i64 as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
                ^ (d.cx.round() as i64 as u64);
            let jx = (unit(key) - 0.5) * d.rx * 1.1;
            let jr = d.rx
                * (0.20 + 0.15 * unit(key ^ 0x51ED))
                * (1.0 - 0.45 * j as f32 / reach.max(1.0));
            let ja = d.alpha * 0.85 * (1.0 - j as f32 / reach.max(1.0)).powi(2);
            if jr < 0.2 || ja <= 0.012 {
                continue;
            }
            lens(
                grid,
                field,
                look,
                d.cx + jx,
                yj,
                jr,
                jr * 0.62,
                ja,
                0.55,
                0.6,
                false,
            );
        }
        lens(
            grid, field, look, d.cx, d.cy, d.rx, d.ry, d.alpha, 1.0, 1.2, true,
        );
    }
}

fn draw_sheen(grid: &mut Grid, look: &Look) {
    let n = ((look.w * look.h) as f32 / 1300.0).clamp(4.0, 90.0) as usize;
    for i in 0..n {
        let hi = i as u64;
        let x = (unit(look.at(L_SPARK, hi, 0)) * look.w as f32) as usize;
        let y = (unit(look.at(L_SPARK, hi, 1)) * look.h as f32) as usize;
        let pa = unit(look.at(L_SPARK, hi, 2));
        let pv = unit(look.at(L_SPARK, hi, 3));
        let tw = 0.5 + 0.5 * look.sin_of((look.time / (2.4 + 5.0 * pv) + pa) * TAU);
        let amt = tw * look.twinkle;
        if amt < 0.52 {
            continue;
        }
        let cell = &mut grid[y][x];
        // A glint only shows where the glass is dark enough to catch it.
        let lit = match cell.bg {
            Color::Rgb { r, g, b } => (r as u32 + g as u32 + b as u32) as f32 / 765.0,
            _ => 1.0,
        };
        if lit > 0.34 {
            continue;
        }
        cell.ch = if amt > 0.90 {
            GLINT[2]
        } else if amt > 0.70 {
            GLINT[1]
        } else {
            GLINT[0]
        };
        cell.fg = look.pale;
        cell.bg = lighten(cell.bg, 34);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::morph::IterateFrameRenderer;
    use crate::render::grid_to_plain;
    use rand::{rngs::StdRng, SeedableRng};

    const W: usize = 118;
    const H: usize = 38;

    fn knobs() -> Vec<f32> {
        PARAMS.iter().map(|p| p.default).collect()
    }

    fn frame(seed: u64, time: f32, values: &[f32]) -> Grid {
        let mut grid = vec![vec![Cell::blank(); W]; H];
        let palette = crate::color::make_palette(seed);
        let mut rng = StdRng::seed_from_u64(seed);
        MODE.render(&mut ModeFrame {
            grid: &mut grid,
            width: W,
            height: H,
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

    fn diff(a: &Grid, b: &Grid) -> usize {
        a.iter()
            .flatten()
            .zip(b.iter().flatten())
            .filter(|(x, y)| x.ch != y.ch || x.bg != y.bg)
            .count()
    }

    fn glyphs(grid: &Grid) -> usize {
        grid.iter().flatten().filter(|c| c.ch != ' ').count()
    }

    #[test]
    fn nightglass_snapshots_and_deterministic_motion() {
        let k = knobs();
        let still = frame(42, 0.0, &k);
        let moving = frame(42, 6.0, &k);
        insta::assert_snapshot!("nightglass_seed42", text(&still));
        insta::assert_snapshot!("nightglass_seed42_t6", text(&moving));
        assert_ne!(still, moving);
        assert_eq!(still, frame(42, 0.0, &k));
        assert_ne!(still, frame(43, 0.0, &k));
        // Every knob has to change the pane at some point in the loop: the
        // passing light is a rare event, so this probes a time when it is live.
        let live = frame(42, 2.0, &k);
        for i in 0..KNOBS {
            let param = &PARAMS[i];
            for probe in [param.max, param.min] {
                if probe == param.default {
                    continue;
                }
                let mut changed = k.clone();
                changed[i] = probe;
                assert_ne!(
                    live,
                    frame(42, 2.0, &changed),
                    "{}{probe} did not move the frame",
                    param.key
                );
            }
        }
    }

    /// The drops are the point of the mode: turning the lens off has to visibly
    /// change the pane, and starved knobs have to leave a plainer window.
    #[test]
    fn nightglass_drops_refract_and_starved_knobs_clear_the_pane() {
        let mut k = knobs();
        let lens_on = frame(42, 2.0, &k);
        k[8] = 0.0;
        let lens_off = frame(42, 2.0, &k);
        assert!(diff(&lens_on, &lens_off) > 40);
        let mut bare = knobs();
        bare[3] = 0.0;
        bare[4] = 0.0;
        bare[5] = 0.0;
        bare[9] = 0.0;
        let plain = frame(42, 2.0, &bare);
        assert!(glyphs(&plain) * 2 < glyphs(&frame(42, 2.0, &knobs())));
    }

    #[test]
    fn nightglass_knob_rolls_land_in_distinct_windows() {
        // The demo's randomize mode samples every knob, so a roll must change the
        // picture, not just its shading.
        let frames: Vec<Grid> = (1..=10u64)
            .map(|seed| {
                let values: Vec<f32> = PARAMS
                    .iter()
                    .map(|p| crate::opts::rand_knob(seed, p))
                    .collect();
                frame(seed, 1.5, &values)
            })
            .collect();
        for i in 0..frames.len() {
            for j in 0..i {
                let n = diff(&frames[i], &frames[j]);
                assert!(n > 400, "rolls {i}/{j}: only {n} differing cells");
            }
        }
    }

    #[test]
    fn nightglass_boundaries_and_hostile_inputs() {
        let palette = crate::color::make_palette(42);
        let extremes = [
            PARAMS.iter().map(|p| p.min).collect::<Vec<_>>(),
            PARAMS.iter().map(|p| p.max).collect(),
            vec![f32::NAN; KNOBS],
            vec![f32::INFINITY; KNOBS],
            vec![-1.0e9; KNOBS],
        ];
        for (w, h) in [
            (0, 0),
            (0, 3),
            (3, 0),
            (1, 1),
            (2, 9),
            (9, 2),
            (61, 17),
            (241, 71),
        ] {
            for values in &extremes {
                for time in [
                    0.0,
                    7.0,
                    -100.0,
                    f32::MIN,
                    f32::MAX,
                    f32::NAN,
                    f32::INFINITY,
                ] {
                    let mut grid = vec![vec![Cell::blank(); w]; h];
                    let mut rng = StdRng::seed_from_u64(42);
                    MODE.render(&mut ModeFrame {
                        grid: &mut grid,
                        width: w,
                        height: h,
                        seed: u64::MAX,
                        palette: &palette,
                        rng: &mut rng,
                        time,
                        args: &[],
                        param_values: Some(values),
                    });
                    assert_eq!(grid.len(), h);
                    assert!(grid.iter().all(|r| r.len() == w));
                    assert!(grid
                        .iter()
                        .flatten()
                        .all(|c| crate::types::char_width(c.ch) == 1));
                }
            }
        }
        // Positional arguments reach the same knobs as live values.
        let args: Vec<String> = [
            "ascii-renderer",
            "7",
            "nightglass",
            "moss",
            "12",
            "0.5",
            "1.4",
            "0.0",
            "2.0",
            "1.6",
            "-0.8",
            "0.9",
            "1.3",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let mut grid = vec![vec![Cell::blank(); 24]; 8];
        let mut rng = StdRng::seed_from_u64(7);
        MODE.render(&mut ModeFrame {
            grid: &mut grid,
            width: 24,
            height: 8,
            seed: 7,
            palette: &palette,
            rng: &mut rng,
            time: 0.0,
            args: &args,
            param_values: None,
        });
        let positional = grid.clone();
        let mut reference = vec![vec![Cell::blank(); 24]; 8];
        let mut rng = StdRng::seed_from_u64(7);
        let values: Vec<f32> = args[4..].iter().map(|v| v.parse().unwrap()).collect();
        MODE.render(&mut ModeFrame {
            grid: &mut reference,
            width: 24,
            height: 8,
            seed: 7,
            palette: &palette,
            rng: &mut rng,
            time: 0.0,
            args: &[],
            param_values: Some(&values),
        });
        assert_eq!(positional, reference);
        // A NaN clock is a still frame.
        assert_eq!(frame(42, f32::NAN, &knobs()), frame(42, 0.0, &knobs()));
        let mut renderer = IterateFrameRenderer::new(NAME, 42, "deep", W, H).unwrap();
        assert!(renderer.render(3.0, Some(&knobs())).is_some());
    }
}
