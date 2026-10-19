use crate::_0_profile::measure_layer;
use crate::color::hsl_to_rgb;
use crate::opts::param_f32;
use crate::registry::{AnimKind, Mode, ModeFrame, Param};
use crate::types::{Cell, Grid};
use crossterm::style::Color;
use rayon::prelude::*;
use std::f64::consts::TAU;

pub(super) struct Breathhex;
pub(super) static MODE: Breathhex = Breathhex;

const NAME: &str = "breathhex-q27";
const HELP: &str = "breathhex-q27: breathing hex tiling of the upper half plane, no rim \
                    [pan] [rise] [wave] [spiral] [pulse] [rate] [spread] [size] [jitter]";

const SQ3: f64 = 1.7320508075688772;
const INV_SQ3_2: f64 = 0.8660254037844386;

const PARAMS: &[Param] = &[
    param!("PAN", "axis drift", 0.0, 0.6, 0.30, 0.02),
    param!("RISE", "rim drift", 0.0, 0.3, 0.10, 0.01),
    param!("WAVE", "wave frequency", 0.0, 4.0, 2.2, 0.1),
    param!("SPIRAL", "phase twist", -4.0, 4.0, 1.5, 0.25),
    param!("PULSE", "breath depth", 0.0, 0.6, 0.32, 0.04),
    param!("RATE", "breath rate", 0.0, 1.0, 0.3, 0.05),
    param!("SPREAD", "hyperbolic depth", 0.8, 4.0, 2.2, 0.1),
    param!("SIZE", "hex scale", 0.08, 0.5, 0.26, 0.01),
    param!("JITTER", "phase chaos", 0.0, 2.0, 0.8, 0.1),
];

impl Mode for Breathhex {
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
        let t = frame.time as f64;
        let seed = frame.seed;
        let base_hue = rgb_hue(frame.palette[1]) + (seed % 97) as f64;
        let pan = knob(frame, 0) as f64 * t * 0.06;
        let rise = knob(frame, 1) as f64 * t * 0.02;
        let wave = knob(frame, 2) as f64;
        let spiral = knob(frame, 3) as f64;
        let pulse = knob(frame, 4) as f64;
        let rate = knob(frame, 5) as f64 * TAU;
        let spread = knob(frame, 6) as f64;
        let size = knob(frame, 7) as f64;
        let jitter = knob(frame, 8) as f64;
        draw(
            frame.grid, frame.width, frame.height, seed, t, base_hue, pan, rise, wave, spiral,
            pulse, rate, spread, size, jitter,
        );
    }
}

/// Screen is an affine window over the upper half plane z = u + iv. Each cell
/// maps to strip coords (xi, eta) = (ln|z|, arg z); the regular hex lattice in
/// that strip is the tiling. Hexes shrink toward the off-screen origin.
fn draw(
    grid: &mut Grid,
    w: usize,
    h: usize,
    seed: u64,
    t: f64,
    base_hue: f64,
    pan: f64,
    rise: f64,
    wave: f64,
    spiral: f64,
    pulse: f64,
    rate: f64,
    spread: f64,
    size: f64,
    jitter: f64,
) {
    let rr = size / INV_SQ3_2;
    let u0 = 0.8;
    let v0 = 0.6;
    let e = spread.exp();
    let u1 = u0 * e;
    let v1 = v0 * e;
    let su = (u1 - u0) / w as f64;
    let sv = (v1 - v0) / h as f64;
    let uc = u0 + (w as f64 * 0.5 + 0.5) * su;
    let vc = v0 + (h as f64 * 0.5 + 0.5) * sv;
    let xic = 0.5 * (uc * uc + vc * vc).ln() + pan;
    let etac = vc.atan2(uc) + rise;

    let rows: Vec<(Vec<u64>, Vec<f32>, Vec<f32>)> = measure_layer(NAME, "field", || {
        (0..h).into_par_iter().map(|y| {
            let v = v0 + (y as f64 + 0.5) * sv;
            let mut hex = Vec::with_capacity(w);
            let mut bri = Vec::with_capacity(w);
            let mut hue = Vec::with_capacity(w);
            let (mut li, mut lj, mut cb, mut ch) = (i64::MIN, i64::MIN, 0f32, 0f32);
            for x in 0..w {
                let u = u0 + (x as f64 + 0.5) * su;
                let xi = 0.5 * (u * u + v * v).ln();
                let eta = v.atan2(u);
                let xd = xi + pan;
                let yd = eta + rise;
                let (i, j) = hex_at(xd, yd, rr);
                let key = (i.wrapping_add(1_000_000) as u64) << 32
                    | (j.wrapping_add(1_000_000) as u64 & 0xFFFF_FFFF);
                let (bn, hv) = if i == li && j == lj {
                    (cb, ch)
                } else {
                    li = i;
                    lj = j;
                    let cx = rr * (SQ3 * i as f64 + INV_SQ3_2 * j as f64);
                    let cy = 1.5 * rr * j as f64;
                    let dx = cx - xic;
                    let dy = cy - etac;
                    let dist = (dx * dx + dy * dy).sqrt();
                    let ang = dy.atan2(dx);
                    let (ja, jb) = hash2(seed, i, j);
                    let ph = wave * dist + spiral * ang + ja * TAU * jitter;
                    let b = 1.0 + pulse * (rate * t + ph).sin();
                    let bn = if pulse > 1e-4 {
                        ((b - (1.0 - pulse)) / (2.0 * pulse)).clamp(0.0, 1.0)
                    } else {
                        0.5
                    };
                    let hue = (base_hue + 55.0 * ang + 30.0 * (jb - 0.5)) % 360.0;
                    cb = bn as f32;
                    ch = if hue < 0.0 { hue + 360.0 } else { hue } as f32;
                    (cb, ch)
                };
                let cx = rr * (SQ3 * i as f64 + INV_SQ3_2 * j as f64);
                let cy = 1.5 * rr * j as f64;
                let ddx = xd - cx;
                let ddy = yd - cy;
                let dc = (ddx * ddx + ddy * ddy).sqrt();
                let eff = size * (0.75 + 0.30 * bn as f64);
                hex.push(key | (if dc <= eff { 1u64 << 63 } else { 0 }));
                bri.push(bn);
                hue.push(hv);
            }
            (hex, bri, hue)
        }).collect()
    });

    measure_layer(NAME, "paint", || {
        grid.par_iter_mut().enumerate().for_each(|(y, row)| {
            let (_hex, bri, hue) = &rows[y];
            for x in 0..w {
                let bn = bri[x] as f64;
                let h = hue[x] as f64;
                let cell = if _hex[x] >> 63 == 1 {
                    Cell::new('\u{2591}', hsl_to_rgb(h, 0.55, 0.16 + 0.26 * bn))
                } else {
                    Cell::new('\u{00B7}', hsl_to_rgb(h, 0.35, 0.10 + 0.12 * bn))
                };
                row[x] = cell;
            }
        });
    });

    measure_layer(NAME, "bound", || {
        grid.par_iter_mut().enumerate().for_each(|(y, row)| {
            let (hex, bri, hue) = &rows[y];
            let below = if y + 1 < h { Some(&rows[y + 1].0) } else { None };
            for x in 0..w {
                let left = x > 0 && hex[x] != hex[x - 1];
                let down = below.is_some_and(|h| hex[x] != h[x]);
                if left || down {
                    row[x] = Cell::new(
                        '\u{2588}',
                        hsl_to_rgb(hue[x] as f64, 0.6, 0.40 + 0.16 * bri[x] as f64),
                    );
                }
            }
        });
    });
}

/// Pointy-top hex lattice in strip space, circumradius rr, axial (q, r).
fn hex_at(x: f64, y: f64, rr: f64) -> (i64, i64) {
    let q = (SQ3 / 3.0 * x - y / 3.0) / rr;
    let r = (2.0 / 3.0 * y) / rr;
    let s = -q - r;
    let mut qi = q.round();
    let mut ri = r.round();
    let mut si = s.round();
    let dq = (q - qi).abs();
    let dr = (r - ri).abs();
    let ds = (s - si).abs();
    if dq > dr && dq > ds {
        qi = -si - ri;
    } else if dr > ds {
        ri = -qi - si;
    } else {
        si = -qi - ri;
    }
    (qi as i64, ri as i64)
}

/// Splitmix64 pair of uniform [0,1) values from (seed, i, j).
fn hash2(seed: u64, i: i64, j: i64) -> (f64, f64) {
    let ii = i as u64;
    let jj = j as u64;
    let mut z = seed
        .wrapping_add(ii.wrapping_mul(0x9E37_79B9_7F4A_7C15))
        .wrapping_add(jj.wrapping_mul(0xC2B2_AE3D_27D4_EB4F));
    let a = mix(&mut z);
    let b = mix(&mut z);
    (a, b)
}

fn mix(z: &mut u64) -> f64 {
    *z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut h = *z;
    h = (h ^ (h >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h = (h ^ (h >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    h ^= h >> 31;
    (h >> 11) as f64 / (1u64 << 53) as f64
}

/// Knob resolution: positional arg, live knob, env, then default; clamp range.
fn knob(frame: &ModeFrame<'_>, idx: usize) -> f32 {
    let p = PARAMS[idx];
    let v = frame
        .args
        .get(idx + 4)
        .and_then(|s| s.parse::<f32>().ok())
        .or(frame.param_values.map(|pv| pv[idx]))
        .unwrap_or_else(|| param_f32(p.key, p.default));
    v.clamp(p.min, p.max)
}

/// Approximate hue in degrees for an Rgb color; fixed fallback otherwise.
fn rgb_hue(c: Color) -> f64 {
    let (r, g, b) = match c {
        Color::Rgb { r, g, b } => (r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0),
        _ => return 200.0,
    };
    let mx = r.max(g).max(b);
    let mn = r.min(g).min(b);
    let d = mx - mn;
    if d < 1e-6 {
        return 200.0;
    }
    let h = if mx == r {
        ((g - b) / d) % 6.0
    } else if mx == g {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    h * 60.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn render(w: usize, h: usize, seed: u64, time: f32) -> Grid {
        let mut grid: Grid = vec![vec![Cell::blank(); w]; h];
        let mut rng = StdRng::seed_from_u64(seed);
        let palette = color::make_palette(seed);
        MODE.render(&mut ModeFrame {
            grid: &mut grid,
            width: w,
            height: h,
            seed,
            palette: &palette,
            rng: &mut rng,
            time,
            args: &[],
            param_values: None,
        });
        grid
    }

    fn grid_to_string(grid: &Grid) -> String {
        grid.iter()
            .map(|row| row.iter().map(|c| c.ch).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn snapshot_80x24() {
        let text = grid_to_string(&render(80, 24, 42, 0.0));
        insta::assert_snapshot!("breathhex_80x24", text);
    }

    #[test]
    fn snapshot_t6() {
        let text = grid_to_string(&render(80, 24, 42, 6.0));
        insta::assert_snapshot!("breathhex_80x24_t6", text);
    }

    #[test]
    fn deterministic() {
        assert_eq!(
            grid_to_string(&render(80, 24, 42, 0.0)),
            grid_to_string(&render(80, 24, 42, 0.0))
        );
    }

    #[test]
    fn seed_sensitive() {
        assert_ne!(
            grid_to_string(&render(80, 24, 42, 0.0)),
            grid_to_string(&render(80, 24, 43, 0.0))
        );
    }

    #[test]
    fn time_sensitive() {
        assert_ne!(
            grid_to_string(&render(80, 24, 42, 0.0)),
            grid_to_string(&render(80, 24, 42, 2.0))
        );
    }

    #[test]
    #[ignore]
    fn frame_cost_release() {
        let (w, h) = (200, 60);
        let mut grid: Grid = vec![vec![Cell::blank(); w]; h];
        let mut rng = StdRng::seed_from_u64(42);
        let palette = color::make_palette(42);
        let frames = 30;
        let start = std::time::Instant::now();
        for k in 0..frames {
            MODE.render(&mut ModeFrame {
                grid: &mut grid,
                width: w,
                height: h,
                seed: 42,
                palette: &palette,
                rng: &mut rng,
                time: (k as f32) * 0.07,
                args: &[],
                param_values: None,
            });
        }
        let avg_ms = start.elapsed().as_secs_f64() / frames as f64 * 1000.0;
        println!("breathhex 200x60 frame cost: {avg_ms:.2} ms");
        assert!(avg_ms < 6.0, "frame cost {avg_ms:.2} ms exceeds 6 ms budget");
    }
}
