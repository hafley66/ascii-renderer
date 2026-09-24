//! Streamnet: the two harmonic families of a seeded complex potential, drawn as
//! one orthogonal net. Equipotentials and streamlines cross at right angles; the
//! term phases precess so the whole web slowly morphs.
use crate::_0_profile::measure_layer;
use crate::color::{darken, hsl_to_rgb, lerp_color};
use crate::opts::param_f32;
use crate::registry::{AnimKind, Mode, ModeFrame, Param};
use crate::types::{Cell, Grid};
use crossterm::style::Color;
use rayon::prelude::*;
use std::cell::RefCell;
use std::f32::consts::TAU;

pub(super) struct StreamNet;
pub(super) static MODE: StreamNet = StreamNet;

const NAME: &str = "streamnet";
const KNOBS: usize = 12;
const HELP: &str = "streamnet: equipotentials and streamlines of a morphing complex potential [zoom] [degree] [density] [thick] [warp] [spin] [morph] [hue] [aspect] [depth] [ground] [halo]";

const PARAMS: &[Param] = &[
    param!("ZOOM", "plane scale", 0.8, 2.0, 1.0, 0.05),
    param!("DEGREE", "polynomial degree", 2.0, 6.0, 3.0, 1.0),
    param!("DENSITY", "lines per unit", 1.5, 8.0, 3.5, 0.5),
    param!("THICK", "line weight", 0.05, 0.45, 0.16, 0.01),
    param!("WARP", "coordinate warp", 0.0, 0.6, 0.12, 0.02),
    param!("SPIN", "rotation rad/s", -0.5, 0.5, 0.06, 0.01),
    param!("MORPH", "phase drift rate", 0.0, 1.2, 0.45, 0.05),
    param!("HUE", "base hue", 0.0, 360.0, 198.0, 1.0),
    param!("ASPECT", "cols per row", 0.8, 3.0, 2.0, 0.05),
    param!("DEPTH", "speed response", 0.0, 1.3, 0.7, 0.05),
    param!("GROUND", "field wash", 0.0, 1.0, 0.35, 0.05),
    param!("HALO", "line underglow", 0.0, 0.9, 0.3, 0.05),
];

const L_TERM: u64 = 0x21;
const L_WARP: u64 = 0x22;

const MAXD: usize = 7;
const PARALLEL_MIN_CELLS: usize = 20_480;

impl Mode for StreamNet {
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

/// Splitmix64 over (seed, layer, index, slot); consumes no rng stream.
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
fn smooth(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Bilinear value noise on an integer lattice, output in -1..1.
#[inline]
fn noise(seed: u64, layer: u64, u: f32, v: f32) -> f32 {
    let (iu, iv) = (u.floor(), v.floor());
    let (fu, fv) = (smooth(u - iu), smooth(v - iv));
    let (iu, iv) = (iu as i64 as u64, iv as i64 as u64);
    let s = |du: u64, dv: u64| unit(hash(seed, layer, iu.wrapping_add(du), iv.wrapping_add(dv)));
    let a = s(0, 0) + (s(1, 0) - s(0, 0)) * fu;
    let b = s(0, 1) + (s(1, 1) - s(0, 1)) * fu;
    (a + (b - a) * fv) * 2.0 - 1.0
}

/// One frame's resolved coefficients, geometry, and palette.
struct Look {
    seed: u64,
    cx: f32,
    cy: f32,
    inv: f32,
    aspect: f32,
    degree: usize,
    dens: f32,
    thick: f32,
    warp: f32,
    hue: f32,
    depth: f32,
    ground: f32,
    halo: f32,
    rot_c: f32,
    rot_s: f32,
    cr: [f32; MAXD],
    ci: [f32; MAXD],
    bg: Color,
    wash: Color,
}

impl Look {
    fn new(seed: u64, w: usize, h: usize, palette: &[Color; 5], time: f32, p: &[f32; KNOBS]) -> Self {
        let degree = (p[1].round() as i32).clamp(2, 6) as usize;
        let aspect = p[8].max(0.5);
        let zoom = p[0].max(0.2);
        let half_w = (w as f32 * 0.5) / aspect;
        let half_h = h as f32 * 0.5;
        let radius = (half_w * half_w + half_h * half_h).sqrt().max(1.0);
        let inv = 1.0 / (radius * zoom);
        let theta = p[5] * time;
        let mut cr = [0.0f32; MAXD];
        let mut ci = [0.0f32; MAXD];
        cr[1] = 0.9;
        let mid = (degree as f32 + 1.0) * 0.5;
        for k in 2..=degree {
            let amp = (0.40 + 0.50 * unit(hash(seed, L_TERM, k as u64, 0))) / 1.6f32.powi(k as i32);
            let ph = unit(hash(seed, L_TERM, k as u64, 1)) * TAU;
            let om = (k as f32 - mid) * 0.6;
            let a = ph + om * p[6] * time;
            cr[k] = amp * a.cos();
            ci[k] = amp * a.sin();
        }
        Look {
            seed,
            cx: w as f32 * 0.5,
            cy: h as f32 * 0.5,
            inv,
            aspect,
            degree,
            dens: p[2],
            thick: p[3].max(0.02),
            warp: p[4],
            hue: p[7],
            depth: p[9],
            ground: p[10],
            halo: p[11],
            rot_c: theta.cos(),
            rot_s: theta.sin(),
            cr,
            ci,
            bg: darken(palette[0], 18),
            wash: hsl_to_rgb(((p[7] + 205.0) / 360.0).rem_euclid(1.0) as f64, 0.55, 0.14),
        }
    }
}

/// Per-cell potential samples: both conjugate values and the complex velocity.
#[derive(Clone, Copy, Default)]
struct Sample {
    phi: f32,
    psi: f32,
    re: f32,
    im: f32,
}

thread_local! {
    static SCRATCH: RefCell<Vec<Sample>> = const { RefCell::new(Vec::new()) };
}

#[inline(always)]
fn each_row<T: Send, F>(buf: &mut [T], w: usize, h: usize, row: F)
where
    F: Fn((usize, &mut [T])) + Sync + Send,
{
    if w == 0 {
        return;
    }
    if w * h >= PARALLEL_MIN_CELLS {
        buf.par_chunks_mut(w).enumerate().with_min_len(8).for_each(row);
    } else {
        buf.chunks_mut(w).enumerate().for_each(row);
    }
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
            buf.resize(w * h, Sample::default());
        }
        let field = &mut buf[..w * h];
        measure_layer(NAME, "field", || eval_field(field, w, h, &look));
        measure_layer(NAME, "wash", || paint_wash(frame.grid, field, w, h, &look));
        measure_layer(NAME, "ink", || paint_ink(frame.grid, field, w, h, &look));
    });
}

/// Evaluate the analytic potential and its derivative by Horner per cell.
fn eval_field(field: &mut [Sample], w: usize, h: usize, look: &Look) {
    let degree = look.degree;
    each_row(field, w, h, |(y, row)| {
        let dy = (y as f32 + 0.5 - look.cy) * look.inv;
        for (x, cell) in row.iter_mut().enumerate() {
            let dx = (x as f32 + 0.5 - look.cx) * look.inv / look.aspect;
            let mut zx = dx * look.rot_c - dy * look.rot_s;
            let mut zy = dx * look.rot_s + dy * look.rot_c;
            if look.warp > 0.0 {
                zx += noise(look.seed, L_WARP, dx * 2.6, dy * 2.6) * look.warp * 0.14;
                zy += noise(look.seed, L_WARP, dx * 2.6 + 40.0, dy * 2.6 - 17.0) * look.warp * 0.14;
            }
            let mut ar = 0.0f32;
            let mut ai = 0.0f32;
            let mut gr = 0.0f32;
            let mut gi = 0.0f32;
            for k in (1..=degree).rev() {
                let ck = k as f32;
                let nk = look.cr[k];
                let mk = look.ci[k];
                let nr = ar * zx - ai * zy + nk;
                let ni = ar * zy + ai * zx + mk;
                let gnr = gr * zx - gi * zy + ck * nk;
                let gni = gr * zy + gi * zx + ck * mk;
                ar = nr;
                ai = ni;
                gr = gnr;
                gi = gni;
            }
            *cell = Sample {
                phi: ar * zx - ai * zy,
                psi: ar * zy + ai * zx,
                re: gr,
                im: gi,
            };
        }
    });
}

/// Lay the dark plate: background tinted by local field speed.
fn paint_wash(grid: &mut Grid, field: &[Sample], w: usize, h: usize, look: &Look) {
    let paint = |(y, row): (usize, &mut Vec<Cell>)| {
        let base = y * w;
        for (x, cell) in row.iter_mut().enumerate() {
            let s = field[base + x];
            let spd = (s.re * s.re + s.im * s.im).sqrt();
            let vel = spd / (1.0 + spd);
            *cell = Cell::with_bg(' ', look.bg, lerp_color(look.bg, look.wash, look.ground * vel));
        }
    };
    if w * h >= PARALLEL_MIN_CELLS {
        grid.par_iter_mut().enumerate().for_each(paint);
    } else {
        grid.iter_mut().enumerate().for_each(paint);
    }
}

/// Draw both families as thin lines, oriented by the local complex velocity.
fn paint_ink(grid: &mut Grid, field: &[Sample], w: usize, h: usize, look: &Look) {
    let paint = |(y, row): (usize, &mut Vec<Cell>)| {
        let base = y * w;
        for (x, cell) in row.iter_mut().enumerate() {
            let s = field[base + x];
            let spd = (s.re * s.re + s.im * s.im).sqrt();
            let vel = spd / (1.0 + spd);
            let glow = 0.55 + look.depth * vel;
            let fx = s.phi * look.dens;
            let fy = s.psi * look.dens;
            let dr = (fx - fx.round()).abs();
            let dc = (fy - fy.round()).abs();
            let th = look.thick;
            let rphi = (1.0 - dr / th).max(0.0);
            let rpsi = (1.0 - dc / th).max(0.0);
            let equip = rphi >= rpsi;
            let line = if equip { rphi } else { rpsi };
            if line > 0.04 {
                let slope = if equip {
                    let den = s.im * look.aspect;
                    if den.abs() < 1e-3 {
                        1e3
                    } else {
                        s.re / den
                    }
                } else {
                    let den = s.re * look.aspect;
                    if den.abs() < 1e-3 {
                        1e3
                    } else {
                        -s.im / den
                    }
                };
                let glyph = if rphi > 0.55 && rpsi > 0.55 {
                    '+'
                } else {
                    glyph_for(slope)
                };
                let hue = if equip { look.hue + 36.0 } else { look.hue };
                let light = (0.42 + 0.5 * smooth(line) * glow).min(0.9);
                cell.fg = hsl_to_rgb((hue / 360.0).rem_euclid(1.0) as f64, 0.72, light as f64);
                cell.ch = glyph;
            } else {
                let halo_th = th * 3.0;
                let halo = (1.0 - dr.min(dc) / halo_th).max(0.0);
                if halo > 0.02 && look.halo > 0.0 {
                    let hue = if equip { look.hue + 36.0 } else { look.hue };
                    cell.fg = hsl_to_rgb(
                        (hue / 360.0).rem_euclid(1.0) as f64,
                        0.5,
                        (look.halo * halo * 0.28 * glow) as f64,
                    );
                    cell.ch = if halo > 0.5 { '.' } else { ' ' };
                }
            }
        }
    };
    if w * h >= PARALLEL_MIN_CELLS {
        grid.par_iter_mut().enumerate().for_each(paint);
    } else {
        grid.iter_mut().enumerate().for_each(paint);
    }
}

#[inline]
fn glyph_for(slope: f32) -> char {
    let a = slope.abs();
    if a > 2.4 {
        '|'
    } else if a < 0.45 {
        '-'
    } else if slope > 0.0 {
        '\\'
    } else {
        '/'
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
    fn streamnet_seed42() {
        insta::assert_snapshot!("streamnet_80x24", text(&frame(80, 24, 42, 0.0, &knobs())));
    }

    #[test]
    fn streamnet_seed42_t6() {
        insta::assert_snapshot!("streamnet_80x24_t6", text(&frame(80, 24, 42, 6.0, &knobs())));
    }

    #[test]
    fn deterministic_and_seed_sensitive() {
        let k = knobs();
        assert_eq!(text(&frame(90, 30, 42, 0.0, &k)), text(&frame(90, 30, 42, 0.0, &k)));
        assert_ne!(text(&frame(90, 30, 42, 0.0, &k)), text(&frame(90, 30, 7, 0.0, &k)));
    }

    #[test]
    fn time_morphs_the_net() {
        let k = knobs();
        assert_ne!(text(&frame(90, 30, 42, 0.0, &k)), text(&frame(90, 30, 42, 5.0, &k)));
    }

    #[test]
    fn density_changes_the_net() {
        let mut k = knobs();
        let a = text(&frame(90, 30, 42, 0.0, &k));
        k[2] = 7.0;
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
        eprintln!("streamnet frame_cost 200x60: avg {:.3} ms, worst {:.3} ms", avg, worst);
        if !cfg!(debug_assertions) {
            assert!(avg < 6.0, "avg frame {:.3} ms", avg);
        }
    }
}
