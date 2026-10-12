//! Tidepool: a rocky shore at low tide. Shallow water shimmers with
//! caustics above a speckled wet sand; kelp sways, stars breathe, foam bobs.
use crate::_0_profile::measure_layer;
use crate::color::{darken, hsl_to_rgb, lerp_color, lighten};
use crate::opts::param_f32;
use crate::registry::{AnimKind, Mode, ModeFrame, Param};
use crate::types::{Cell, Grid};
use crossterm::style::Color;
use rayon::prelude::*;
use std::f32::consts::TAU;

pub(super) struct Tidepool;
pub(super) static MODE: Tidepool = Tidepool;

const NAME: &str = "tidepool";
const KNOBS: usize = 9;
const HELP: &str = "tidepool: rocky shore at low tide, caustics shimmer and kelp sways [tide] [shimmer] [stars] [kelp] [sway] [hue] [foam] [pebbles] [chop]";

const L_SEA: u64 = 0x21;
const L_SAND: u64 = 0x22;
const L_STAR: u64 = 0x23;
const L_KELP: u64 = 0x24;
const L_FOAM: u64 = 0x25;
const L_CRITTER: u64 = 0x26;

const PARALLEL_MIN_CELLS: usize = 20_480;

const PARAMS: &[Param] = &[
    param!("TIDE", "water level", 0.05, 0.95, 0.55, 0.01),
    param!("SHIMMER", "caustic speed", 0.0, 3.0, 1.0, 0.05),
    param!("STARS", "sea stars", 0.0, 8.0, 3.0, 1.0),
    param!("KELP", "kelp density", 0.0, 1.0, 0.55, 0.05),
    param!("SWAY", "kelp sway rad/s", 0.0, 2.0, 0.8, 0.05),
    param!("HUE", "hue shift", 0.0, 180.0, 25.0, 1.0),
    param!("FOAM", "foam along the waterline", 0.0, 1.5, 1.0, 0.05),
    param!("PEBBLES", "pebble scatter", 0.0, 1.0, 0.5, 0.05),
    param!("CHOP", "waterline bob", 0.0, 2.0, 0.6, 0.05),
];

impl Mode for Tidepool {
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
fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// One frame's resolved geometry and palette.
struct Look {
    seed: u64,
    wl: f32,
    time: f32,
    shimmer: f32,
    stars: usize,
    kelp: f32,
    sway: f32,
    hue: f32,
    foam: f32,
    pebbles: f32,
    chop: f32,
    sea_hue: f64,
    sand_hue: f64,
    kelp_hue: f64,
}

impl Look {
    fn new(seed: u64, w: usize, h: usize, time: f32, p: &[f32; KNOBS]) -> Self {
        let wl = (p[0] * h as f32).round().max(2.0);
        let drift = unit(hash(seed, L_SEA, 0, 3)) * 40.0;
        Look {
            seed,
            wl,
            time,
            shimmer: p[1],
            stars: p[2].round() as usize,
            kelp: p[3],
            sway: p[4],
            hue: p[5],
            foam: p[6],
            pebbles: p[7],
            chop: p[8],
            sea_hue: (185.0 + drift as f64 + p[5] as f64 * 0.9).rem_euclid(360.0),
            sand_hue: (32.0 + drift as f64 * 0.2 + p[5] as f64 * 0.25).rem_euclid(360.0),
            kelp_hue: (95.0 + drift as f64 * 0.3 + p[5] as f64 * 0.4).rem_euclid(360.0),
        }
    }

    /// Waterline row for one column: base tide plus a slow bob and seed jitter.
    #[inline]
    fn waterline(&self, x: usize) -> f32 {
        let t = self.time;
        let bob = (x as f32 * 0.35 + t * 0.6).sin() * self.chop * 1.2;
        let jitter = (unit(hash(self.seed, L_FOAM, x as u64, 0)) - 0.5) * 0.9;
        self.wl + bob + jitter
    }

    /// Caustic lattice value in [-3, 3]; drifts with shimmer.
    #[inline]
    fn caustic(&self, x: f32, y: f32) -> f32 {
        let t = self.time * self.shimmer;
        (x * 0.7 + t).sin() + (y * 1.1 - t * 0.8).sin() + ((x + y) * 0.5 + t * 1.3).sin()
    }
}

fn draw(frame: &mut ModeFrame<'_>, p: &[f32; KNOBS]) {
    let (w, h) = (frame.width, frame.height);
    if w == 0 || h == 0 {
        return;
    }
    let look = Look::new(frame.seed, w, h, frame.time, p);
    measure_layer(NAME, "sea", || paint_sea(frame.grid, w, h, &look));
    measure_layer(NAME, "sand", || paint_sand(frame.grid, w, h, &look));
    measure_layer(NAME, "kelp", || paint_kelp(frame.grid, w, h, &look));
    measure_layer(NAME, "stars", || paint_stars(frame.grid, w, h, &look));
    measure_layer(NAME, "critters", || paint_critters(frame.grid, w, h, &look));
    measure_layer(NAME, "foam", || paint_foam(frame.grid, w, h, &look));
}

#[inline(always)]
fn each_row<F>(grid: &mut Grid, w: usize, h: usize, row: F)
where
    F: Fn(usize, &mut Vec<Cell>) + Sync + Send,
{
    let rows = grid.len().min(h);
    let slice = &mut grid[..rows];
    if w * h >= PARALLEL_MIN_CELLS {
        slice.par_iter_mut().enumerate().with_min_len(8).for_each(|(y, r)| row(y, r));
    } else {
        slice.iter_mut().enumerate().for_each(|(y, r)| row(y, r));
    }
}

/// Shallow sea: depth ramp, wave streaks, caustic sparkle.
fn paint_sea(grid: &mut Grid, w: usize, h: usize, look: &Look) {
    let t = look.time;
    each_row(grid, w, h, |y, row| {
        for (x, cell) in row.iter_mut().enumerate().take(w) {
            let wl = look.waterline(x);
            let d = wl - (y as f32 + 0.5);
            if d <= 0.0 {
                continue;
            }
            let norm = (d / wl.max(1.0)).clamp(0.0, 1.0);
            let c = look.caustic(x as f32, y as f32);
            let g = hash(look.seed, L_SEA, x as u64, y as u64);
            let ripple = (x as f32 * 0.6 - t * 1.5 * look.shimmer).sin();
            let ch = if norm < 0.12 {
                ' '
            } else if ripple > 0.82 && d < 1.6 {
                '~'
            } else if c > 2.25 {
                '*'
            } else if c > 1.65 {
                '+'
            } else if norm < 0.3 {
                '.'
            } else if norm < 0.55 {
                ':'
            } else if unit(g) < 0.5 {
                '~'
            } else {
                '-'
            };
            let lit = (0.10 + 0.16 * (1.0 - norm) + 0.06 * c).clamp(0.05, 0.55);
            let fg = hsl_to_rgb(look.sea_hue, 0.75, (lit + 0.18).clamp(0.0, 0.8) as f64);
            let bg = hsl_to_rgb(look.sea_hue, 0.6, lit as f64);
            *cell = Cell::with_bg(ch, fg, bg);
        }
    });
}

/// Wet and dry sand: speckles, pebbles, a darker wet band under the foam.
fn paint_sand(grid: &mut Grid, w: usize, h: usize, look: &Look) {
    each_row(grid, w, h, |y, row| {
        for (x, cell) in row.iter_mut().enumerate().take(w) {
            let wl = look.waterline(x);
            let below = (y as f32 + 0.5) - wl;
            if below <= 0.0 {
                continue;
            }
            let g = hash(look.seed, L_SAND, x as u64, y as u64);
            let u = unit(g);
            let wet = smoothstep((3.2 - below) / 3.2);
            let ch = if u < 0.05 {
                '.'
            } else if u < 0.10 {
                ':'
            } else if u < 0.14 {
                ','
            } else if u < 0.17 && wet < 0.5 {
                '\''
            } else {
                ' '
            };
            let peb = unit(hash(look.seed, L_SAND, x as u64, y as u64 | 1 << 16));
            let (ch, fg) = if peb < look.pebbles * 0.06 {
                ('o', darken(hsl_to_rgb(look.sand_hue, 0.25, 0.35), 15))
            } else if peb > 1.0 - look.pebbles * 0.01 {
                ('%', darken(hsl_to_rgb(look.sand_hue, 0.2, 0.25), 25))
            } else {
                (ch, darken(hsl_to_rgb(look.sand_hue, 0.35, 0.45), 20))
            };
            let bg = darken(hsl_to_rgb(look.sand_hue, 0.4, (0.30 - 0.14 * wet) as f64), 10);
            *cell = Cell::with_bg(ch, fg, bg);
        }
    });
}

/// Kelp strands rooted near the waterline, swaying as they rise into the sea.
fn paint_kelp(grid: &mut Grid, w: usize, h: usize, look: &Look) {
    let count = (look.kelp * (w as f32 / 6.0)).round() as usize;
    for i in 0..count {
        let g = hash(look.seed, L_KELP, i as u64, 0);
        let anchor_x = (unit(g) * (w as f32 - 2.0) + 1.0).floor() as usize;
        let len = (2.0 + unit(hash(g, 1, 7, 0)) * 6.0).round() as usize;
        let phase = unit(hash(g, 2, 11, 0)) * TAU;
        let tint = unit(hash(g, 3, 13, 0)) * 30.0;
        let fg = hsl_to_rgb((look.kelp_hue + tint as f64).rem_euclid(360.0), 0.7, 0.30);
        let dark = darken(hsl_to_rgb((look.kelp_hue + tint as f64).rem_euclid(360.0), 0.7, 0.18), 10);
        let top = (look.waterline(anchor_x) as usize).saturating_sub(len);
        let base = (look.waterline(anchor_x) as usize).min(h.saturating_sub(1));
        for k in 0..base.saturating_sub(top) {
            let y = base - k;
            let frac = k as f32 / len.max(1) as f32;
            let dx = (look.time * look.sway * 2.0 + phase + frac * 2.2).sin() * frac * 1.6;
            let x = (anchor_x as f32 + dx).round().clamp(0.0, (w - 1) as f32) as usize;
            let ch = if k == 0 {
                '%'
            } else if k % 3 == 0 {
                'o'
            } else {
                '|'
            };
            let cell_fg = if k % 3 == 0 { fg } else { dark };
            grid[y][x] = Cell::with_bg(ch, cell_fg, grid[y][x].bg);
        }
    }
}

/// Sea stars on the sand: five arms drawn radially, breathing slowly.
fn paint_stars(grid: &mut Grid, w: usize, h: usize, look: &Look) {
    for i in 0..look.stars {
        let g = hash(look.seed, L_STAR, i as u64, 0);
        let wl = look.waterline(w / 2);
        let cy = (wl + 1.5 + unit(g) * (h as f32 - wl - 3.0).max(0.5)).floor() as usize;
        let cx = (unit(hash(g, 1, 5, 0)) * (w as f32 - 2.0) + 1.0).floor() as usize;
        let phase = unit(hash(g, 2, 9, 0)) * TAU;
        let rot = unit(hash(g, 3, 15, 0)) * TAU;
        let breathe = if look.time > 0.0 {
            0.5 + 0.5 * (look.time * 0.9 + phase).sin()
        } else {
            0.5
        };
        let radius = 1.6 + 0.5 * breathe + unit(hash(g, 4, 21, 0)) * 0.6;
        let hue = (look.sand_hue * 0.2 + 15.0 + unit(hash(g, 5, 27, 0)) as f64 * 40.0).rem_euclid(360.0) as f64;
        let core = hsl_to_rgb(hue, 0.85, 0.62);
        let limb = hsl_to_rgb(hue, 0.8, 0.45);
        let reach = (radius + 1.0).ceil() as usize;
        let y0 = cy.saturating_sub(reach);
        let y1 = (cy + reach + 1).min(h.min(grid.len()));
        let x0 = cx.saturating_sub(reach);
        let x1 = (cx + reach + 1).min(w);
        for y in y0..y1 {
            for x in x0..x1 {
                let dx = x as f32 - cx as f32;
                let dy = (y as f32 - cy as f32) * 2.0;
                let r = (dx * dx + dy * dy).sqrt();
                if r > radius {
                    continue;
                }
                let ang = dy.atan2(dx) + rot;
                let arm = (ang * 5.0 / TAU).rem_euclid(1.0);
                let spoke = (arm - 0.5).abs() * 2.0;
                let ch = if r < radius * 0.35 {
                    'O'
                } else if spoke < 0.18 {
                    '*'
                } else if spoke < 0.4 {
                    '+'
                } else {
                    '.'
                };
                let fg = if r < radius * 0.35 { core } else { limb };
                grid[y][x] = Cell::with_bg(ch, fg, grid[y][x].bg);
            }
        }
    }
}

/// Anemones and shells tucked against the waterline rocks.
fn paint_critters(grid: &mut Grid, w: usize, h: usize, look: &Look) {
    let count = 1 + (unit(hash(look.seed, L_CRITTER, 0, 0)) * 3.0).floor() as usize;
    for i in 0..count {
        let g = hash(look.seed, L_CRITTER, i as u64 + 1, 4);
        let wl = look.waterline(w / 2);
        let cy = (wl + 0.8 + unit(g) * 1.5).floor() as usize;
        let cx = (unit(hash(g, 1, 6, 0)) * (w as f32 - 3.0) + 1.5).floor() as usize;
        let phase = unit(hash(g, 2, 8, 0)) * TAU;
        let pulse = if look.time > 0.0 {
            0.5 + 0.5 * (look.time * 1.8 + phase).sin()
        } else {
            0.5
        };
        let hue = (320.0 + unit(hash(g, 3, 10, 0)) * 60.0).rem_euclid(360.0) as f64;
        let core = hsl_to_rgb(hue, 0.7, 0.55 + 0.15 * pulse as f64);
        let tendril = hsl_to_rgb(hue, 0.6, 0.42);
        let cx_i = cx.min(w.saturating_sub(2));
        let cy_i = cy.min(h.min(grid.len()).saturating_sub(2));
        grid[cy_i][cx_i] = Cell::with_bg('@', core, grid[cy_i][cx_i].bg);
        let arms = 4 + (pulse * 2.0).round() as usize;
        for a in 0..arms {
            let ang = a as f32 / arms as f32 * TAU;
            let x = cx_i as f32 + ang.cos() * 1.4;
            let y = cy_i as f32 + ang.sin() * 1.0;
            if x >= 0.0 && y >= 0.0 && (x as usize) < w && (y as usize) < h.min(grid.len()) {
                let (x, y) = (x as usize, y as usize);
                grid[y][x] = Cell::with_bg('+', tendril, grid[y][x].bg);
            }
        }
    }
}

/// Foam: bright bubbles hugging the waterline, thinning with the foam knob.
fn paint_foam(grid: &mut Grid, w: usize, h: usize, look: &Look) {
    if look.foam <= 0.0 {
        return;
    }
    let t = look.time;
    let white = lighten(hsl_to_rgb(look.sea_hue, 0.2, 0.85), 40);
    each_row(grid, w, h, |y, row| {
        for (x, cell) in row.iter_mut().enumerate().take(w) {
            let wl = look.waterline(x);
            let d = (y as f32 + 0.5) - wl;
            if d < -0.4 || d > 1.6 {
                continue;
            }
            let g = hash(look.seed, L_FOAM, x as u64, y as u64, );
            let u = unit(g ^ (g >> 20));
            let burst = (x as f32 * 0.8 + t * 1.2).sin();
            let density = look.foam * (0.6 + 0.4 * burst);
            let ch = if u < 0.10 * density {
                '*'
            } else if u < 0.30 * density {
                'o'
            } else if u < 0.55 * density && d < 0.8 {
                '~'
            } else if u < 0.75 * density {
                '.'
            } else {
                continue;
            };
            let bg = hsl_to_rgb(look.sea_hue, 0.5, 0.35);
            *cell = Cell::with_bg(ch, white, bg);
        }
    });
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
    fn tidepool_seed42() {
        insta::assert_snapshot!("tidepool_80x24", text(&frame(80, 24, 42, 0.0, &knobs())));
    }

    #[test]
    fn tidepool_seed42_t6() {
        insta::assert_snapshot!("tidepool_80x24_t6", text(&frame(80, 24, 42, 6.0, &knobs())));
    }

    #[test]
    fn deterministic_and_seed_sensitive() {
        let k = knobs();
        assert_eq!(text(&frame(90, 30, 42, 0.0, &k)), text(&frame(90, 30, 42, 0.0, &k)));
        assert_ne!(text(&frame(90, 30, 42, 0.0, &k)), text(&frame(90, 30, 7, 0.0, &k)));
    }

    #[test]
    fn time_moves_the_shore() {
        let k = knobs();
        assert_ne!(text(&frame(90, 30, 42, 0.0, &k)), text(&frame(90, 30, 42, 4.0, &k)));
    }

    #[test]
    fn stars_change_the_beach() {
        let mut k = knobs();
        let a = text(&frame(90, 30, 42, 0.0, &k));
        k[2] = 6.0;
        assert_ne!(a, text(&frame(90, 30, 42, 0.0, &k)));
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
        eprintln!("tidepool frame_cost 200x60: avg {:.3} ms, worst {:.3} ms", avg, worst);
        if !cfg!(debug_assertions) {
            assert!(avg < 6.0, "avg frame {:.3} ms", avg);
        }
    }
}
