use crate::_0_profile::measure_layer;
use crate::opts::param_f32;
use crate::registry::{AnimKind, Mode, ModeFrame, Param};
use rayon::prelude::*;
use crate::types::{Cell, Grid};

pub(super) struct Nightveil;
pub(super) static MODE: Nightveil = Nightveil;

const NAME: &str = "nightveil";
const KNOBS: usize = 6;
const HELP: &str = "nightveil: polar night, aurora curtains over a ridge [curtains] [drift] [shimmer] [waves] [stars] [motes]";

const PARAMS: &[Param] = &[
    param!("CURTAINS", "curtains", 1.0, 8.0, 4.0, 1.0),
    param!("DRIFT", "drift", 0.0, 3.0, 1.0, 0.05),
    param!("SHIMMER", "shimmer", 0.0, 1.0, 0.55, 0.05),
    param!("WAVES", "waves", 0.0, 3.0, 1.0, 0.1),
    param!("STARS", "stars", 0.0, 1.0, 0.5, 0.05),
    param!("MOTES", "motes", 0.0, 2.0, 1.0, 0.1),
];

const PARALLEL_MIN_CELLS: usize = 20_480;

impl Mode for Nightveil {
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

#[inline(always)]
fn each_row<F>(buf: &mut Grid, w: usize, h: usize, row: F)
where
    F: Fn((usize, &mut Vec<Cell>)) + Sync + Send,
{
    if w == 0 {
        return;
    }
    if w * h >= PARALLEL_MIN_CELLS {
        buf.par_iter_mut().enumerate().with_min_len(8).for_each(row);
    } else {
        buf.iter_mut().enumerate().for_each(row);
    }
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

fn draw(frame: &mut ModeFrame<'_>, p: &[f32; KNOBS]) {
    let w = frame.width;
    let h = frame.height;
    let seed = frame.seed;
    let t = frame.time;
    let grid: &mut Grid = &mut *frame.grid;
    let palette = frame.palette;

    let sky = measure_layer(NAME, "sky", || {
        each_row(grid, w, h, |(y, row)| {
            let fade = 1.0 - (y as f32 / h as f32) * 0.5;
            for cell in row.iter_mut() {
                cell.fg = crate::color::darken(palette[0], (24.0 * fade) as u8);
                cell.bg = crate::color::darken(palette[0], (30.0 * fade) as u8);
            }
        });
    });

    measure_layer(NAME, "stars", || {
        each_row(grid, w, h, |(y, row)| {
            for (x, cell) in row.iter_mut().enumerate() {
                let hs = hash(seed, 7, (y * w + x) as u64, 11);
                if unit(hs) > p[4] * 0.045 {
                    continue;
                }
                let tw = 0.5
                    + 0.5
                        * (t * (1.5 + 2.0 * unit(hash(seed, 7, (y * w + x) as u64, 13)))
                            + 6.2832 * unit(hs))
                            .sin();
                let bright = smoothstep(0.35 + 0.65 * tw);
                cell.ch = if tw > 0.82 { '*' } else { '.' };
                cell.fg = crate::color::lighten(palette[4], (10.0 + 90.0 * bright) as u8);
            }
        });
    });

    let curtains = p[0].round().max(1.0) as usize;
    measure_layer(NAME, "curtains", || {
        each_row(grid, w, h, |(y, row)| {
            let yn = y as f32 / h as f32;
            for (x, cell) in row.iter_mut().enumerate() {
                let mut v = 0.0f32;
                let mut hue_t = 0.0f32;
                for c in 0..curtains {
                    let cs = hash(seed, 3, c as u64, 5);
                    let center = w as f32 * ((c as f32 + 0.5) / curtains as f32)
                        + w as f32 * 0.22 * (unit(cs) - 0.5);
                    let sway = w as f32 * (0.04 + 0.05 * unit(hash(cs, 9, 1, 2)))
                        * (yn * (2.0 + 3.0 * unit(hash(cs, 9, 2, 2)) * p[3])
                            + 0.35 * t * p[1]
                            + unit(cs) * 6.2832)
                            .sin();
                    let dx = x as f32 - (center + sway);
                    let sigma = w as f32 * (0.012 + 0.02 * unit(hash(cs, 9, 3, 2)));
                    let g = (-dx * dx / (2.0 * sigma * sigma)).exp();
                    let vert_top = 0.05 + 0.2 * unit(hash(cs, 9, 4, 2));
                    let vert_bot = 0.62 + 0.3 * unit(hash(cs, 9, 5, 2))
                        + 0.08 * p[2] * (yn * 7.0 + t * (0.6 + p[1]) + unit(cs) * 6.2832).sin();
                    let span = (vert_bot - vert_top).max(0.05);
                    let fall = if yn < vert_top || yn > vert_bot {
                        0.0
                    } else {
                        let e = (yn - vert_top) / span;
                        smoothstep(1.0 - (2.0 * e - 1.0).abs()) * 0.9 + 0.1 * (1.0 - e)
                    };
                    let shimmer = 0.75
                        + 0.25 * p[2]
                            * (yn * 13.0 + x as f32 * 0.35 + t * (1.2 + 2.0 * unit(hash(cs, 9, 6, 2))))
                                .sin();
                    let b = g * fall * shimmer;
                    if b > 0.003 {
                        v += b;
                        hue_t += b * unit(hash(cs, 9, 7, 2));
                    }
                }
                if v <= 0.004 {
                    continue;
                }
                let v = (v / curtains.max(1) as f32 * 2.2).min(1.0);
                cell.ch = if v < 0.15 {
                    '.'
                } else if v < 0.35 {
                    ':'
                } else if v < 0.6 {
                    '~'
                } else if v < 0.85 {
                    '|'
                } else {
                    '#'
                };
                let hue_mix = if hue_t > 0.0 { (hue_t / v).min(1.0) } else { 0.5 };
                let base = crate::color::lerp_color(palette[2], palette[3], hue_mix);
                cell.fg = crate::color::lighten(base, (40.0 + 140.0 * v) as u8);
            }
        });
    });


    measure_layer(NAME, "ridge", || {
        let base_y = (h as f32 * 0.86) as usize;
        let amp = (h as f32 * 0.07).max(1.5);
        for x in 0..w {
            let xn = x as f32 / w as f32;
            let rise = amp
                * (0.6 * (xn * 9.0 + 2.0 * unit(hash(seed, 5, 1, 3)) * 6.2832).sin()
                    + 0.4 * (xn * 21.0 + 4.0 * unit(hash(seed, 5, 2, 3)) * 6.2832).sin());
            let top = base_y.saturating_sub(rise as usize) .min(h.saturating_sub(1));
            for y in top..h {
                let cell = &mut grid[y][x];
                cell.ch = if y == top { '^' } else { ' ' };
                cell.fg = crate::color::darken(palette[1], 34);
                cell.bg = crate::color::darken(palette[1], 40);
            }
            if top > 0 && unit(hash(seed, 5, x as u64, 4)) < 0.35 {
                let cell = &mut grid[top - 1][x];
                cell.ch = if unit(hash(seed, 5, x as u64, 6)) < 0.5 { ',' } else { '\'' };
                cell.fg = crate::color::darken(palette[1], 28);
            }
        }
    });

    measure_layer(NAME, "motes", || {
        let count = ((w * h) as f32 / 900.0 * p[5]).ceil() as usize;
        for m in 0..count {
            let hm = hash(seed, 11, m as u64, 8);
            let speed = 0.4 + 1.6 * unit(hash(hm, 13, 1, 9));
            let x0 = unit(hash(hm, 13, 2, 9)) * w as f32;
            let drift = (unit(hash(hm, 13, 3, 9)) - 0.5) * 2.0;
            let life = h as f32 * 1.3;
            let age = (t * speed * p[1].max(0.2) + unit(hm) * life) % life;
            let y = (h as f32 + 2.0 - age) as isize;
            let x = (x0 + drift * age * 0.35 + w as f32) as usize % w;
            if y < 0 || y as usize >= h {
                continue;
            }
            let fade = (1.0 - age / life).clamp(0.0, 1.0);
            let cell = &mut grid[y as usize][x];
            cell.ch = if fade > 0.6 { '*' } else { '+' };
            cell.fg = crate::color::lighten(palette[3], (40.0 + 110.0 * fade) as u8);
        }
    });

    drop(sky);
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
    fn nightveil_seed42() {
        insta::assert_snapshot!("nightveil_80x24", text(&frame(80, 24, 42, 0.0, &knobs())));
    }

    #[test]
    fn nightveil_seed42_t6() {
        insta::assert_snapshot!("nightveil_80x24_t6", text(&frame(80, 24, 42, 6.0, &knobs())));
    }

    #[test]
    fn nightveil_deterministic() {
        assert_eq!(
            text(&frame(80, 24, 42, 3.0, &knobs())),
            text(&frame(80, 24, 42, 3.0, &knobs()))
        );
    }

    #[test]
    fn nightveil_seed_sensitive() {
        assert_ne!(
            text(&frame(80, 24, 42, 0.0, &knobs())),
            text(&frame(80, 24, 99, 0.0, &knobs()))
        );
    }

    #[test]
    fn nightveil_time_sensitive() {
        assert_ne!(
            text(&frame(80, 24, 42, 0.0, &knobs())),
            text(&frame(80, 24, 42, 5.0, &knobs()))
        );
    }

    #[test]
    fn nightveil_frame_cost() {
        let start = std::time::Instant::now();
        let n = 40;
        for i in 0..n {
            let _ = frame(200, 60, 42, i as f32 * 0.1, &knobs());
        }
        let avg = start.elapsed() / n;
        println!("nightveil 200x60 avg frame: {avg:?}");
        assert!(avg.as_millis() < 6, "frame cost {avg:?} exceeds 6 ms budget");
    }
}
