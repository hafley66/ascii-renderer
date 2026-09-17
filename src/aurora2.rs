//! aurora2 -- flow-field curtains of northern light over a pine horizon. Ribbons
//! accumulate additively into intensity and hue fields, then resolve to a ramp.
use crate::_0_profile::measure_layer;
use crate::color::*;
use crate::opts::param_f32;
use crate::pp::{pp_fbm, pp_vnoise};
use crate::types::*;
use crossterm::style::Color;
use rand::RngExt;
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::cell::RefCell;
use std::f32::consts::TAU;

const RAMP: [char; 9] = [' ', '.', ':', '-', '=', '+', '*', '#', '@'];
const REF_COLS: f32 = 80.0;

pub(crate) struct Aurora2Knobs {
    pub ribbons: f32,
    pub width: f32,
    pub drift: f32,
    pub horizon: f32,
    pub stars: f32,
    pub spread: f32,
    pub pines: f32,
    pub fold: f32,
    pub gain: f32,
}

impl Aurora2Knobs {
    #[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
    pub(crate) fn from_env() -> Self {
        Aurora2Knobs {
            ribbons: param_f32("RIBBONS", 5.0),
            width: param_f32("RWIDTH", 7.0),
            drift: param_f32("DRIFT", 1.0),
            horizon: param_f32("HORIZON", 0.72),
            stars: param_f32("STARS", 1.0),
            spread: param_f32("SPREAD", 1.0),
            pines: param_f32("PINES", 1.0),
            fold: param_f32("FOLD", 1.0),
            gain: param_f32("GAIN", 1.0),
        }
    }
}

/// Stream splitter: every roll is keyed to (seed, layer, index) so one curtain
/// or star never shifts its neighbors when a knob moves.
#[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
fn side_rng(seed: u64, layer: u64, index: u64) -> StdRng {
    let mut h = seed ^ layer.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    h ^= index.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h ^= h >> 30;
    h = h.wrapping_mul(0xD6E8_FEB8_6659_FD93);
    h ^= h >> 27;
    StdRng::seed_from_u64(h.wrapping_mul(0x94D0_49BB_1331_11EB))
}

#[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
fn hash01(x: i64, y: i64, k: u64, seed: u64) -> f32 {
    let mut h = (x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (y as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9)
        ^ k.wrapping_mul(0x94D0_49BB_1331_11EB)
        ^ seed;
    h ^= h >> 31;
    h = h.wrapping_mul(0xD6E8_FEB8_6659_FD93);
    h ^= h >> 32;
    (h & 0xFF_FFFF) as f32 / 16_777_216.0
}

/// Hue walk along a curtain: green at the hem, magenta mid, cyan at the crown.
/// `spread` scales every excursion away from the green anchor.
#[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
fn hue_path(u: f32, spread: f32) -> f32 {
    let u = u.rem_euclid(1.0);
    let raw = if u < 0.5 {
        130.0 + (300.0 - 130.0) * (u / 0.5)
    } else {
        300.0 + (190.0 - 300.0) * ((u - 0.5) / 0.5)
    };
    (130.0 + (raw - 130.0) * spread).rem_euclid(360.0)
}

struct Ribbon {
    x: f32,
    sway: f32,
    half: f32,
    phase: f32,
    speed: f32,
    top: f32,
    hem: f32,
    hue0: f32,
    power: f32,
    ray: f32,
}

#[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
fn roll_ribbons(seed: u64, n: usize) -> Vec<Ribbon> {
    let mut out = Vec::with_capacity(n);
    let span = 1.0 / n as f32;
    for i in 0..n {
        let mut r = side_rng(seed, 1, i as u64);
        let top = r.random_range(0.0..0.24f32);
        let hem = (top + r.random_range(0.42..0.78f32)).min(0.97);
        let slot = (i as f32 + 0.5) * span + r.random_range(-0.44..0.44f32) * span;
        out.push(Ribbon {
            x: 0.06 + slot.clamp(0.0, 1.0) * 0.88,
            sway: r.random_range(0.09..0.27f32),
            half: r.random_range(0.55..1.55f32),
            phase: r.random::<f32>() * 47.0,
            speed: r.random_range(0.5..1.7f32),
            top,
            hem,
            hue0: r.random::<f32>() * 0.30,
            power: r.random_range(0.55..1.20f32),
            ray: r.random_range(0.35..0.95f32),
        });
    }
    out
}

struct Field {
    inten: Vec<f32>,
    hue: Vec<f32>,
}

thread_local! {
    static FIELD: RefCell<Field> = RefCell::new(Field { inten: Vec::new(), hue: Vec::new() });
}

#[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
fn accumulate(f: &mut Field, w: usize, sky: usize, seed: u64, t: f32, k: &Aurora2Knobs, ribs: &[Ribbon]) {
    let cells = w * sky;
    f.inten.clear();
    f.hue.clear();
    f.inten.resize(cells, 0.0);
    f.hue.resize(cells, 0.0);
    let scale = (w as f32 / REF_COLS).max(0.35);
    let fold = k.fold.clamp(0.0, 2.0);
    let gain = k.gain.max(0.0);
    for (ri, rib) in ribs.iter().enumerate() {
        let rk = seed ^ (ri as u64).wrapping_mul(0x5EED_1234_ABCD_0001);
        let half = (k.width.max(1.0) * 0.5 * rib.half * scale).max(1.0);
        let reach = half * 3.2;
        let drift = t * k.drift * rib.speed;
        let depth = (rib.hem - rib.top).max(0.06);
        for y in 0..sky {
            let v = (y as f32 + 0.5) / sky as f32;
            let u = (rib.hem - v) / depth;
            if u < -0.12 || u > 1.06 {
                continue;
            }
            let flow = pp_fbm(v * 2.1, rib.phase + drift * 0.13, rk) - 0.5;
            let curl = pp_vnoise(v * 6.3, rib.phase * 2.3 + drift * 0.21, rk ^ 0xA17) - 0.5;
            let spine = (rib.x + (flow * 2.0 + curl * 0.35) * rib.sway) * w as f32;
            let env = if u < 0.0 {
                let s = u / 0.07;
                (-(s * s)).exp()
            } else {
                let rim = -(u / 0.13).powi(2);
                (1.0 - u).max(0.0).powf(0.85) + rim.exp() * 0.8
            };
            if env < 0.02 {
                continue;
            }
            let hue = hue_path(rib.hue0 + u.max(0.0) * 0.82, k.spread);
            let x0 = ((spine - reach).floor() as i64).max(0) as usize;
            let x1 = ((spine + reach).ceil() as i64).min(w as i64 - 1);
            if x1 < x0 as i64 {
                continue;
            }
            let row = y * w;
            for x in x0..=(x1 as usize) {
                let dx = (x as f32 + 0.5 - spine) / half;
                let d2 = dx * dx;
                let body = (-0.95 * d2).exp() + 0.45 * (-6.0 * d2).exp();
                let xn = (x as f32 + 0.5) * REF_COLS / w as f32;
                let fold_n = pp_vnoise(
                    xn * (0.09 + rib.ray * 0.16) + flow * 2.2,
                    v * 0.85 + rib.phase + drift * 0.16,
                    rk ^ 0x5A5,
                );
                let grain = pp_vnoise(
                    xn * 0.62 + flow * 3.4,
                    v * 1.9 + rib.phase * 3.0 + drift * 0.26,
                    rk ^ 0x33F,
                );
                let rn = fold_n * 0.74 + grain * 0.26;
                let ray = 1.0 - fold * 0.52 * (1.0 - rn);
                let a = env * body * ray.max(0.05) * rib.power * gain;
                if a <= 0.004 {
                    continue;
                }
                f.inten[row + x] += a;
                f.hue[row + x] += a * hue;
            }
        }
    }
}

#[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
fn sky_bg(palette: &[Color; 5], v: f32) -> Color {
    let top = lerp_color(rgb(2, 3, 9), palette[0], 0.16);
    let low = lerp_color(rgb(5, 9, 21), palette[0], 0.30);
    lerp_color(top, low, v.clamp(0.0, 1.0).powf(1.6))
}

#[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
fn paint_sky(grid: &mut Grid, w: usize, h: usize, sky: usize, palette: &[Color; 5]) {
    for y in 0..h {
        let v = if sky > 0 {
            (y as f32 / sky as f32).min(1.0)
        } else {
            1.0
        };
        let bg = sky_bg(palette, v);
        for cell in grid[y].iter_mut().take(w) {
            *cell = Cell::with_bg(' ', Color::Reset, bg);
        }
    }
}

#[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
fn paint_stars(grid: &mut Grid, w: usize, sky: usize, seed: u64, t: f32, k: &Aurora2Knobs, palette: &[Color; 5]) {
    if sky == 0 {
        return;
    }
    let want = ((w * sky) as f32 * 0.022 * k.stars.max(0.0)) as usize;
    let n = want.min(6000);
    let cold = rgb(150, 162, 198);
    for i in 0..n {
        let mut r = side_rng(seed, 2, i as u64);
        let x = r.random_range(0..w.max(1));
        let ry = r.random::<f32>().powf(1.45);
        let y = ((ry * sky as f32) as usize).min(sky.saturating_sub(1));
        let base = r.random_range(0.22..1.0f32);
        let phase = r.random::<f32>() * TAU;
        let rate = r.random_range(0.4..2.4f32);
        let tint = r.random_range(-24.0..30.0f32);
        let b = base * (0.60 + 0.40 * (t * rate * k.drift + phase).sin());
        if b < 0.16 {
            continue;
        }
        let ch = if b < 0.32 {
            '·'
        } else if b < 0.52 {
            '.'
        } else if b < 0.78 {
            '+'
        } else {
            '*'
        };
        let warm = lerp_color(cold, hsl_to_rgb((205.0 + tint) as f64, 0.35, 0.86), 0.5);
        let bg = grid[y][x].bg;
        let fg = lerp_color(bg, lerp_color(warm, rgb(255, 255, 255), b * 0.5), 0.30 + b * 0.70);
        let _ = palette;
        grid[y][x] = Cell::with_bg(ch, fg, bg);
    }
}

#[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
fn paint_curtains(grid: &mut Grid, w: usize, sky: usize, f: &Field) {
    for y in 0..sky {
        let row = y * w;
        for x in 0..w {
            let a = f.inten[row + x];
            if a < 0.05 {
                continue;
            }
            let m = (a * 1.15).powf(0.66).min(1.0);
            let li = ((m * 8.0).round() as usize).clamp(1, 8);
            let hue = (f.hue[row + x] / a) as f64;
            let sat = (0.98 - 0.42 * m as f64).clamp(0.30, 1.0);
            let lum = (0.20 + 0.50 * m as f64).clamp(0.10, 0.88);
            let fg = hsl_to_rgb(hue, sat, lum);
            let bg = lerp_color(grid[y][x].bg, hsl_to_rgb(hue, 0.85, 0.12), (m * 0.55).min(0.55));
            grid[y][x] = Cell::with_bg(RAMP[li], fg, bg);
        }
    }
}

#[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
/// Snow does not mirror the sky, it pools the light falling on it. Average each
/// column over the lower sky, then blur sideways so the foreground reads smooth.
#[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
fn column_glow(f: &Field, w: usize, sky: usize) -> (Vec<f32>, Vec<f64>) {
    let mut a = vec![0.0f32; w];
    let mut hue = vec![150.0f64; w];
    if sky == 0 {
        return (a, hue);
    }
    let lo = sky.saturating_sub((sky as f32 * 0.55).ceil() as usize).min(sky - 1);
    let rows = (sky - lo) as f32;
    for x in 0..w {
        let mut sum = 0.0f32;
        let mut hsum = 0.0f32;
        for y in lo..sky {
            let v = f.inten[y * w + x];
            sum += v;
            hsum += f.hue[y * w + x];
        }
        a[x] = sum / rows;
        hue[x] = if sum > 0.001 { (hsum / sum) as f64 } else { 150.0 };
    }
    let span = ((w as f32 / REF_COLS * 3.0).round() as usize).max(2);
    let mut blur = vec![0.0f32; w];
    for x in 0..w {
        let x0 = x.saturating_sub(span);
        let x1 = (x + span).min(w - 1);
        let mut s = 0.0;
        for v in a.iter().take(x1 + 1).skip(x0) {
            s += *v;
        }
        blur[x] = s / (x1 - x0 + 1) as f32;
    }
    (blur, hue)
}

#[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
fn paint_ground(grid: &mut Grid, w: usize, h: usize, sky: usize, seed: u64, f: &Field, palette: &[Color; 5]) {
    if sky >= h {
        return;
    }
    let deep = lerp_color(rgb(3, 4, 8), palette[0], 0.12);
    let ground_rows = (h - sky) as f32;
    let (col_a, col_h) = column_glow(f, w, sky);
    for y in sky..h {
        let d = (y - sky) as f32 / ground_rows.max(1.0);
        let fade = (1.0 - d * 0.82).powf(1.05);
        for x in 0..w {
            let xn = (x as f32 + 0.5) * REF_COLS / w as f32;
            let wob = ((pp_vnoise(xn * 0.42, y as f32 * 0.9, seed ^ 0x7A7) - 0.5) * 3.4).round();
            let sx = (x as i32 + wob as i32).clamp(0, w as i32 - 1) as usize;
            let raw = col_a[sx];
            let hue = col_h[sx];
            let n = hash01(x as i64, y as i64, 91, seed);
            let a = raw * 0.80 * fade * (0.62 + 0.76 * n);
            let lit = lerp_color(deep, hsl_to_rgb(hue, 0.60, 0.16), (a * 1.1).min(0.75));
            let (ch, fg) = if a > 0.06 {
                let m = (a * 1.9).powf(0.7).min(1.0);
                let li = ((m * 8.0).round() as usize).clamp(1, 5);
                (RAMP[li], hsl_to_rgb(hue, 0.70, 0.22 + 0.30 * m as f64))
            } else if n > 0.991 {
                ('.', lerp_color(deep, rgb(120, 140, 175), 0.55))
            } else if n < 0.004 {
                (',', lerp_color(deep, rgb(90, 105, 140), 0.5))
            } else {
                (' ', deep)
            };
            grid[y][x] = Cell::with_bg(ch, fg, lit);
        }
    }
}

#[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
fn pine_glyph(x: i32, y: i32, seed: u64) -> char {
    match (hash01(x as i64, y as i64, 7, seed) * 4.0) as u32 {
        0 => '#',
        1 => '*',
        2 => '%',
        _ => '@',
    }
}

#[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
fn stamp_pine(grid: &mut Grid, w: usize, sky: usize, base: i32, cx: i32, ph: i32, spread: f32, fg: Color, seed: u64) {
    for i in 0..ph {
        let y = base - (ph - 1 - i);
        if y < 0 || y as usize >= sky {
            continue;
        }
        let hw = (i as f32 * spread).floor() as i32;
        for dx in -hw..=hw {
            let x = cx + dx;
            if x < 0 || x as usize >= w {
                continue;
            }
            let ch = if hw == 0 {
                '^'
            } else if dx == -hw {
                '/'
            } else if dx == hw {
                '\\'
            } else {
                pine_glyph(x, y, seed)
            };
            let bg = lerp_color(grid[y as usize][x as usize].bg, rgb(0, 0, 0), 0.82);
            grid[y as usize][x as usize] = Cell::with_bg(ch, fg, bg);
        }
    }
}

#[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
fn paint_pines(grid: &mut Grid, w: usize, h: usize, sky: usize, seed: u64, k: &Aurora2Knobs, palette: &[Color; 5]) {
    if sky == 0 || sky >= h {
        return;
    }
    let dens = k.pines.clamp(0.0, 4.0);
    let tall = ((sky as f32 * 0.38) as i32).clamp(3, 22);
    let far_fg = lerp_color(lerp_color(rgb(7, 10, 18), palette[0], 0.22), rgb(0, 0, 0), 0.30);
    let near_fg = lerp_color(rgb(2, 3, 6), palette[0], 0.05);
    let amp = ((sky as f32 * 0.11).round() as i32).clamp(1, 6);
    let lift = ((sky as f32 * 0.07).round() as i32).max(1);
    let mut crest = Vec::with_capacity(w);
    for x in 0..w {
        let xn = (x as f32 + 0.5) * REF_COLS / w as f32;
        let n = pp_fbm(xn * 0.055, 3.7, seed ^ 0x0D1D_6E5A_1100_4321) - 0.5;
        let top = sky as i32 - 1 - lift - (n * 2.0 * amp as f32).round() as i32;
        crest.push(top.clamp(0, sky as i32 - 1));
    }
    for x in 0..w {
        for y in crest[x]..sky as i32 {
            let n = hash01(x as i64, y as i64, 23, seed);
            let ch = if y == crest[x] {
                if n < 0.74 { '^' } else { '*' }
            } else if n < 0.30 {
                '#'
            } else if n < 0.6 {
                '%'
            } else {
                '@'
            };
            let bg = lerp_color(grid[y as usize][x].bg, rgb(0, 0, 0), 0.86);
            grid[y as usize][x] = Cell::with_bg(ch, far_fg, bg);
        }
    }
    for (rank, fg) in [(0u64, far_fg), (1u64, near_fg)] {
        let step = if rank == 0 { 3.4 } else { 5.6 };
        let n = ((w as f32 / step) * dens) as usize;
        for i in 0..n.min(6000) {
            let mut r = side_rng(seed, 3 + rank, i as u64);
            let cx = r.random_range(-3..(w as i32 + 3));
            let lo = if rank == 0 { 0.28 } else { 0.55 };
            let ph = (((tall as f32) * r.random_range(lo..1.0f32)).round() as i32).max(3);
            let spread = r.random_range(0.42..0.78f32);
            let anchor = crest[cx.clamp(0, w as i32 - 1) as usize];
            let base = if rank == 0 {
                anchor + r.random_range(0..2i32)
            } else {
                (sky as i32 - 1).min(anchor + r.random_range(1..3i32))
            };
            stamp_pine(
                grid,
                w,
                sky,
                base.min(sky as i32 - 1),
                cx,
                ph,
                spread,
                fg,
                seed ^ rank,
            );
        }
    }
}

#[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
pub fn draw_aurora2(
    grid: &mut Grid,
    w: usize,
    h: usize,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    k: &Aurora2Knobs,
) {
    if w == 0 || h == 0 {
        return;
    }
    let sky = ((h as f32 * k.horizon.clamp(0.15, 0.98)).round() as usize).clamp(1, h.saturating_sub(1).max(1));
    measure_layer("aurora2", "sky", || paint_sky(grid, w, h, sky, palette));
    measure_layer("aurora2", "stars", || {
        paint_stars(grid, w, sky, seed, t, k, palette)
    });
    let n = (k.ribbons.round() as i32).clamp(1, 24) as usize;
    let ribs = roll_ribbons(seed, n);
    FIELD.with(|slot| {
        let mut f = slot.borrow_mut();
        measure_layer("aurora2", "field", || {
            accumulate(&mut f, w, sky, seed, t, k, &ribs)
        });
        measure_layer("aurora2", "curtains", || paint_curtains(grid, w, sky, &f));
        measure_layer("aurora2", "ground", || {
            paint_ground(grid, w, h, sky, seed, &f, palette)
        });
    });
    measure_layer("aurora2", "pines", || {
        paint_pines(grid, w, h, sky, seed, k, palette)
    });
}

#[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
pub fn cli_aurora2(
    mut grid: Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: [Color; 5],
    rng: StdRng,
    t_anim: f32,
    term_w: u16,
    term_h: u16,
    args: &[String],
    mode: &str,
    theme_name: &str,
) -> (Grid, bool) {
    let _ = (rng, term_w, term_h, mode, theme_name);
    let mut k = Aurora2Knobs::from_env();
    let pos: Vec<f32> = args.iter().skip(4).filter_map(|a| a.parse().ok()).collect();
    let slots: [&mut f32; 9] = [
        &mut k.ribbons,
        &mut k.width,
        &mut k.drift,
        &mut k.horizon,
        &mut k.stars,
        &mut k.spread,
        &mut k.pines,
        &mut k.fold,
        &mut k.gain,
    ];
    for (slot, v) in slots.into_iter().zip(pos.iter()) {
        *slot = *v;
    }
    draw_aurora2(&mut grid, width, height, seed, &palette, t_anim, &k);
    (grid, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
    fn run(w: usize, h: usize, seed: u64, t: f32) -> String {
        let mut g = vec![vec![Cell::blank(); w]; h];
        let p = crate::color::make_palette(seed);
        let k = Aurora2Knobs::from_env();
        draw_aurora2(&mut g, w, h, seed, &p, t, &k);
        g.iter()
            .map(|row| row.iter().map(|c| c.ch).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    #[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
    fn snapshot_aurora_standard() {
        insta::assert_snapshot!("aurora2_80x24", run(80, 24, 42, 0.0));
    }

    #[test]
    #[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
    fn snapshot_aurora_wide() {
        insta::assert_snapshot!("aurora2_120x36", run(120, 36, 1701, 0.0));
    }

    #[test]
    #[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
    fn deterministic_and_seed_sensitive() {
        assert_eq!(run(90, 30, 42, 0.0), run(90, 30, 42, 0.0));
        assert_ne!(run(90, 30, 42, 0.0), run(90, 30, 7, 0.0));
    }

    #[test]
    #[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
    fn t_drifts_the_curtains() {
        assert_ne!(run(90, 30, 42, 0.0), run(90, 30, 42, 3.0));
        assert_ne!(run(90, 30, 42, 3.0), run(90, 30, 42, 7.0));
    }

    #[test]
    #[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
    fn treeline_stays_still_while_the_sky_moves() {
        let frame = |t: f32| {
            let mut g = vec![vec![Cell::blank(); 90]; 30];
            let p = crate::color::make_palette(42);
            let k = Aurora2Knobs::from_env();
            draw_aurora2(&mut g, 90, 30, 42, &p, t, &k);
            g
        };
        let a = frame(0.0);
        let b = frame(4.0);
        let sky = (30.0f32 * 0.72).round() as usize;
        let row = |g: &Grid, y: usize| g[y].iter().map(|c| c.ch).collect::<String>();
        assert_eq!(row(&a, sky - 1), row(&b, sky - 1), "the ridge crest drifted");
        let mut moved = 0usize;
        for y in 0..sky - 2 {
            if row(&a, y) != row(&b, y) {
                moved += 1;
            }
        }
        assert!(moved > 4, "only {moved} sky rows changed over 4 seconds");
    }

    #[test]
    #[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
    fn tiny_grids_terminate() {
        for (w, h) in [(1usize, 1usize), (6, 2), (12, 3), (3, 20)] {
            let mut g = vec![vec![Cell::blank(); w]; h];
            let p = crate::color::make_palette(5);
            let k = Aurora2Knobs::from_env();
            draw_aurora2(&mut g, w, h, 5, &p, 2.0, &k);
        }
    }

    #[test]
    #[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
    fn every_glyph_is_single_width() {
        use unicode_width::UnicodeWidthChar;
        let mut g = vec![vec![Cell::blank(); 120]; 40];
        let p = crate::color::make_palette(42);
        let k = Aurora2Knobs::from_env();
        for step in 0..12 {
            draw_aurora2(&mut g, 120, 40, 42, &p, step as f32 * 0.7, &k);
            for row in &g {
                for c in row {
                    assert_eq!(c.ch.width().unwrap_or(0), 1, "glyph {:?} is not width 1", c.ch);
                }
            }
        }
    }

    #[test]
    #[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
    fn frame_cost() {
        let (w, h) = (200usize, 60usize);
        let mut g = vec![vec![Cell::blank(); w]; h];
        let p = crate::color::make_palette(42);
        let k = Aurora2Knobs::from_env();
        draw_aurora2(&mut g, w, h, 42, &p, 0.0, &k);
        let start = std::time::Instant::now();
        for f in 0..200 {
            draw_aurora2(&mut g, w, h, 42, &p, f as f32 * 0.05, &k);
        }
        let avg = start.elapsed().as_secs_f64() * 1000.0 / 200.0;
        eprintln!("aurora frame_cost 200x60: avg {:.3} ms", avg);
        if !cfg!(debug_assertions) {
            assert!(avg < 6.0, "avg frame {:.3} ms", avg);
        }
    }
}
