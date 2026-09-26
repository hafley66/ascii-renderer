use crate::_0_profile::measure_layer;
use crate::color::lerp_color;
use crate::opts::param_f32;
use crate::pp::{pp_arc, pp_line, pp_put};
use crate::registry::{AnimKind, Mode, ModeFrame, Param};
use crate::types::{Cell, Grid};

pub(super) struct Hyperhex;
pub(super) static MODE: Hyperhex = Hyperhex;

const NAME: &str = "hyperhex";
const HELP: &str = "hyperhex: breathing non-euclidean hex tiling [rings] [breath] [depth] [asym] [curv] [twist]";

const PARAMS: &[Param] = &[
    param!("RINGS", "hex rings", 2.0, 8.0, 5.0, 1.0),
    param!("BREATH", "breath rate", 0.0, 3.0, 0.6, 0.05),
    param!("DEPTH", "breath depth", 0.0, 0.5, 0.22, 0.01),
    param!("ASYM", "asymmetry", 0.0, 1.0, 0.4, 0.01),
    param!("CURV", "curvature", 0.2, 2.5, 1.2, 0.05),
    param!("TWIST", "twist rad/s", 0.0, 2.0, 0.25, 0.01),
];

/// Deterministic per-cell value, independent of the frame RNG stream.
fn splitmix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

fn unit(seed: u64, layer: u64, idx: u64) -> f32 {
    let h = splitmix(seed ^ splitmix(layer.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ idx));
    ((h >> 11) as f64 / (1u64 << 53) as f64) as f32
}

/// Poincare-disk radial map: planar radius `rho` compresses toward the rim.
/// `curv` small approaches the plain linear projection; large crowds the rim.
fn disk_map(px: f32, py: f32, curv: f32, pr: f32, rmax: f32) -> (f32, f32) {
    let rho = (px * px + py * py).sqrt();
    if rho < 1e-6 || pr < 1e-6 {
        return (0.0, 0.0);
    }
    let s = (rho / pr).min(1.0);
    let u = (curv * s).tanh() / curv.tanh();
    (px / rho * u * rmax, py / rho * u * rmax)
}

struct Knobs {
    rings: f32,
    breath: f32,
    depth: f32,
    asym: f32,
    curv: f32,
    twist: f32,
}

fn knob(frame: &ModeFrame<'_>, i: usize, default: f32, lo: f32, hi: f32) -> f32 {
    frame
        .args
        .get(i + 4)
        .and_then(|s| s.parse::<f32>().ok())
        .or_else(|| frame.param_values.and_then(|v| v.get(i).copied()))
        .unwrap_or_else(|| param_f32(PARAMS[i].key, default))
        .clamp(lo, hi)
}

fn knobs(frame: &ModeFrame<'_>) -> Knobs {
    Knobs {
        rings: knob(frame, 0, 5.0, 2.0, 8.0),
        breath: knob(frame, 1, 0.6, 0.0, 3.0),
        depth: knob(frame, 2, 0.22, 0.0, 0.5),
        asym: knob(frame, 3, 0.4, 0.0, 1.0),
        curv: knob(frame, 4, 1.2, 0.2, 2.5),
        twist: knob(frame, 5, 0.25, 0.0, 2.0),
    }
}

struct Hex {
    cx: i32,
    cy: i32,
    verts: [(i32, i32); 6],
    u: f32,
}

fn draw(frame: &mut ModeFrame<'_>, k: &Knobs) {
    let (w, h) = (frame.width, frame.height);
    if w == 0 || h == 0 {
        return;
    }
    let rings = k.rings.round() as i32;
    if rings < 1 {
        return;
    }
    let seed = frame.seed;
    let grid = &mut *frame.grid;
    let pal = frame.palette;
    let t = frame.time;

    // Visual space: x in columns, y in doubled rows. A visual circle is then a
    // 2:1 screen ellipse matching the cell aspect.
    let rmax = (w as f32 * 0.5).min(h as f32) * 0.94;
    let e = rmax / (rings as f32 * 1.75);
    let pr = rmax;
    let cx0 = w as f32 * 0.5;
    let cy0 = h as f32 * 0.5;
    let to_col = |mx: f32| (cx0 + mx).round() as i32;
    let to_row = |my: f32| (cy0 + my * 0.5).round() as i32;

    let mut hexes: Vec<Hex> = Vec::new();

    measure_layer(NAME, "map", || {
        for q in -rings..=rings {
            for r in -rings..=rings {
                if q.abs().max(r.abs()).max((q + r).abs()) > rings {
                    continue;
                }
                let idx = ((q + rings) as u64) * 32 + (r + rings) as u64;
                let phase = std::f32::consts::TAU * unit(seed, 1, idx);
                let breath = 1.0 + k.depth
                    * (std::f32::consts::TAU * k.breath * t + phase).sin();
                let rot = k.twist * t + k.asym * 1.2 * (unit(seed, 2, idx) - 0.5);
                let jx = k.asym * e * 0.35 * (unit(seed, 4, idx) - 0.5);
                let jy = k.asym * e * 0.35 * (unit(seed, 5, idx) - 0.5);
                let px = e * 1.5 * q as f32 + jx;
                let py = e * (3.0f32).sqrt() * (r as f32 + q as f32 * 0.5) + jy;
                let rho = (px * px + py * py).sqrt();
                let s = (rho / pr).min(1.0);
                let uc = s;
                let size = 1.0 - 0.9 * (k.curv / 2.5) * s * s;

                let mut verts = [(0i32, 0i32); 6];
                for vi in 0..6 {
                    let a = rot + std::f32::consts::FRAC_PI_3 * vi as f32;
                    let rv = e * breath * size
                        * (1.0 + k.asym * 0.8 * (unit(seed, 3, idx * 6 + vi as u64) - 0.5));
                    let vpx = px + rv * a.cos();
                    let vpy = py + rv * a.sin();
                    let (mx, my) = disk_map(vpx, vpy, k.curv, pr, rmax);
                    verts[vi] = (to_col(mx), to_row(my));
                }
                let (cmx, cmy) = disk_map(px, py, k.curv, pr, rmax);
                hexes.push(Hex {
                    cx: to_col(cmx),
                    cy: to_row(cmy),
                    verts,
                    u: uc,
                });
            }
        }
    });

    measure_layer(NAME, "ground", || {
        for y in 0..h {
            for x in 0..w {
                let dx = (x as f32 * 2.0 - w as f32) / w as f32;
                let dy = (y as f32 - h as f32 * 0.5) / h as f32;
                let d = (dx * dx + dy * dy).sqrt().min(1.0);
                let bg = lerp_color(pal[0], pal[2], d * 0.32);
                let dust = unit(seed, 10, (y * w + x) as u64) > 0.99;
                let ch = if dust { '.' } else { ' ' };
                let fg = lerp_color(pal[0], pal[1], 0.5);
                grid[y][x] = Cell::with_bg(ch, fg, bg);
            }
        }
    });

    measure_layer(NAME, "cells", || {
        for hx in &hexes {
            let edge = lerp_color(pal[1], pal[3], hx.u);
            for i in 0..6 {
                let (x0, y0) = hx.verts[i];
                let (x1, y1) = hx.verts[(i + 1) % 6];
                pp_line(grid, x0, y0, x1, y1, edge);
            }
        }
    });

    measure_layer(NAME, "centres", || {
        for hx in &hexes {
            if hx.u < 0.99 {
                let ch = if hx.u < 0.4 { 'o' } else { '.' };
                let col = lerp_color(pal[4], pal[2], hx.u);
                pp_put(grid, hx.cx, hx.cy, ch, col);
            }
        }
    });

    measure_layer(NAME, "rim", || {
        pp_arc(
            grid,
            cx0.round() as i32,
            cy0.round() as i32,
            rmax,
            rmax * 0.5,
            0.0,
            std::f32::consts::TAU,
            pal[3],
            0,
        );
    });
}

impl Mode for Hyperhex {
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
        let k = knobs(frame);
        draw(frame, &k);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::make_palette;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    fn render_at(w: usize, h: usize, seed: u64, time: f32) -> String {
        let mut grid: Grid = vec![vec![Cell::blank(); w]; h];
        let palette = make_palette(seed);
        let mut rng = StdRng::seed_from_u64(seed);
        let args: Vec<String> = Vec::new();
        let mut frame = ModeFrame {
            grid: &mut grid,
            width: w,
            height: h,
            seed,
            palette: &palette,
            rng: &mut rng,
            time,
            args: &args,
            param_values: None,
        };
        MODE.render(&mut frame);
        grid.iter()
            .map(|row| row.iter().map(|c| c.ch).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn snapshot_80x24() {
        insta::assert_snapshot!("hyperhex_80x24", render_at(80, 24, 42, 0.0));
    }

    #[test]
    fn snapshot_t6() {
        insta::assert_snapshot!("hyperhex_t6", render_at(80, 24, 42, 6.0));
    }

    #[test]
    fn deterministic() {
        assert_eq!(render_at(80, 24, 42, 0.0), render_at(80, 24, 42, 0.0));
    }

    #[test]
    fn seed_sensitive() {
        assert_ne!(render_at(80, 24, 42, 0.0), render_at(80, 24, 43, 0.0));
    }

    #[test]
    fn time_sensitive() {
        assert_ne!(render_at(80, 24, 42, 0.0), render_at(80, 24, 42, 6.0));
    }

    #[test]
    fn frame_cost() {
        if cfg!(debug_assertions) {
            return;
        }
        let (w, h) = (200usize, 60usize);
        let args: Vec<String> = Vec::new();
        let palette = make_palette(42);
        let mut rng = StdRng::seed_from_u64(42);
        let mut grid: Grid = vec![vec![Cell::blank(); w]; h];
        let start = std::time::Instant::now();
        let iters = 40;
        for i in 0..iters {
            let mut frame = ModeFrame {
                grid: &mut grid,
                width: w,
                height: h,
                seed: 42,
                palette: &palette,
                rng: &mut rng,
                time: i as f32,
                args: &args,
                param_values: None,
            };
            MODE.render(&mut frame);
        }
        let avg_ms = start.elapsed().as_secs_f64() * 1000.0 / iters as f64;
        assert!(
            avg_ms < 6.0,
            "hyperhex average frame {avg_ms:.3} ms exceeds 6 ms"
        );
    }
}

#[cfg(test)]
mod bench {
    //! Ignored release-mode bench. Run:
    //! CARGO_BUILD_JOBS=4 cargo test --release -- --ignored --nocapture modes::_113_hyperhex_
    use super::*;
    use crate::_0_profile::{LayerTotal, layer_capture_begin, layer_capture_end};
    use crate::color::make_palette;
    use crate::registry::ModeFrame;
    use crate::types::{Cell, Grid};
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    const FRAMES: usize = 60;
    const ROUNDS: usize = 8;

    fn bench_size(w: usize, h: usize) -> (f64, Vec<LayerTotal>) {
        let args: Vec<String> = Vec::new();
        let palette = make_palette(42);
        let mut rng = StdRng::seed_from_u64(42);
        let mut grid: Grid = vec![vec![Cell::blank(); w]; h];

        for i in 0..5 {
            let mut frame = ModeFrame {
                grid: &mut grid,
                width: w,
                height: h,
                seed: 42,
                palette: &palette,
                rng: &mut rng,
                time: i as f32,
                args: &args,
                param_values: None,
            };
            MODE.render(&mut frame);
        }

        let mut best = f64::INFINITY;
        let mut best_layers = Vec::new();
        for _ in 0..ROUNDS {
            layer_capture_begin();
            let start = std::time::Instant::now();
            for i in 0..FRAMES {
                let mut frame = ModeFrame {
                    grid: &mut grid,
                    width: w,
                    height: h,
                    seed: 42,
                    palette: &palette,
                    rng: &mut rng,
                    time: i as f32,
                    args: &args,
                    param_values: None,
                };
                MODE.render(&mut frame);
            }
            let ms = start.elapsed().as_secs_f64() * 1000.0 / FRAMES as f64;
            let layers = layer_capture_end();
            if ms < best {
                best = ms;
                best_layers = layers;
            }
        }
        (best, best_layers)
    }

    /// FNV-1a over the plain chars of every frame in t = 0..FRAMES.
    fn checksum(w: usize, h: usize) -> u64 {
        let args: Vec<String> = Vec::new();
        let palette = make_palette(42);
        let mut rng = StdRng::seed_from_u64(42);
        let mut grid: Grid = vec![vec![Cell::blank(); w]; h];
        let mut hsh: u64 = 0xcbf29ce484222325;
        for i in 0..FRAMES {
            let mut frame = ModeFrame {
                grid: &mut grid,
                width: w,
                height: h,
                seed: 42,
                palette: &palette,
                rng: &mut rng,
                time: i as f32,
                args: &args,
                param_values: None,
            };
            MODE.render(&mut frame);
            for row in &grid {
                for c in row {
                    hsh = (hsh ^ c.ch as u64).wrapping_mul(0x100000001b3);
                }
            }
        }
        hsh
    }

    #[test]
    #[ignore]
    fn bench() {
        for &(w, h) in &[(200usize, 60usize), (400usize, 120usize)] {
            println!("{NAME} {w}x{h} checksum: {:016x}", checksum(w, h));
            let (ms, layers) = bench_size(w, h);
            println!("{NAME} {w}x{h}: {ms:.3} ms/frame");
            for l in &layers {
                let per = l.total_ns as f64 / 1e6 / FRAMES as f64;
                println!(
                    "  layer {:<10} {per:.3} ms/frame ({} calls/frame)",
                    l.layer,
                    l.calls / FRAMES as u64
                );
            }
        }
    }
}
