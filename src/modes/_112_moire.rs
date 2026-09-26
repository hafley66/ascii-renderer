use crate::_0_profile::measure_layer;
use crate::color::lerp_color;
use crate::opts::param_f32;
use crate::pp::pp_hash2;
use crate::registry::{AnimKind, Mode, ModeFrame, Param};
use crate::types::{Cell, Grid};

pub(super) struct Moire;
pub(super) static MODE: Moire = Moire;

const NAME: &str = "moire";
const HELP: &str = "moire: concentric-ring moire [sa] [sb] [sep] [angle] [warp] [contrast] [spin]";

const PARAMS: &[Param] = &[
    param!("SA", "ring A spacing", 3.0, 28.0, 9.0, 1.0),
    param!("SB", "ring B spacing", 3.0, 28.0, 13.0, 1.0),
    param!("SEP", "centre separation", 0.0, 90.0, 28.0, 1.0),
    param!("ANG", "axis angle", 0.0, 360.0, 25.0, 1.0),
    param!("WARP", "noise warp", 0.0, 24.0, 6.0, 1.0),
    param!("CONTRAST", "moire contrast", 0.2, 3.0, 1.3, 0.1),
    param!("SPIN", "spin degrees/s", 0.0, 180.0, 40.0, 1.0),
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

/// Proximity to the nearest ring: 1.0 exactly on a ring, 0.0 midway between.
fn ring(dist: f32, spacing: f32) -> f32 {
    1.0 - (std::f32::consts::PI * dist / spacing).sin().abs()
}

const RAMP: &[u8] = b".:-=+*#%@";

fn ramp(m: f32) -> char {
    let i = (m.clamp(0.0, 1.0) * (RAMP.len() as f32 - 1.0)).round() as usize;
    RAMP[i] as char
}

struct Knobs {
    sa: f32,
    sb: f32,
    sep: f32,
    ang: f32,
    warp: f32,
    contrast: f32,
    spin: f32,
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
        sa: knob(frame, 0, 9.0, 3.0, 28.0),
        sb: knob(frame, 1, 13.0, 3.0, 28.0),
        sep: knob(frame, 2, 28.0, 0.0, 90.0),
        ang: knob(frame, 3, 25.0, 0.0, 360.0),
        warp: knob(frame, 4, 6.0, 0.0, 24.0),
        contrast: knob(frame, 5, 1.3, 0.2, 3.0),
        spin: knob(frame, 6, 40.0, 0.0, 180.0),
    }
}

fn draw(frame: &mut ModeFrame<'_>, k: &Knobs) {
    let (w, h) = (frame.width, frame.height);
    if w == 0 || h == 0 {
        return;
    }
    let seed = frame.seed;
    let grid = &mut *frame.grid;
    let pal = frame.palette;

    // Time rotates the axis between the two ring centres.
    let ang = (k.ang + frame.time * k.spin).to_radians();
    let cx = w as f32; // visual space doubles x for the 2:1 cell aspect
    let cy = h as f32 * 0.5;
    let bx = cx + k.sep * ang.cos();
    let by = cy + k.sep * ang.sin();

    let fg_ground = lerp_color(pal[0], pal[1], 0.55);
    let warp_amp = if k.warp > 0.01 { k.warp } else { 0.0 };

    // The warp noise is bilinear over a small integer lattice; hash that lattice
    // once per frame instead of four hashes per screen cell.
    let xn_max = ((w - 1) as f32 * 0.07).floor() as usize + 2;
    let yn_max = ((h - 1) as f32 * 0.14).floor() as usize + 2;
    let mut ntab = vec![0f32; xn_max * yn_max];
    for j in 0..yn_max {
        let base = j * xn_max;
        for i in 0..xn_max {
            ntab[base + i] = pp_hash2(i as i32, j as i32, seed);
        }
    }
    let mut nx0 = vec![0usize; w];
    let mut nsx = vec![0f32; w];
    for x in 0..w {
        let fx = x as f32 * 0.07;
        let x0 = fx.floor() as usize;
        let tx = fx - x0 as f32;
        nx0[x] = x0;
        nsx[x] = tx * tx * (3.0 - 2.0 * tx);
    }
    let mut ny0 = vec![0usize; h];
    let mut nsy = vec![0f32; h];
    for y in 0..h {
        let fy = y as f32 * 0.14;
        let y0 = fy.floor() as usize;
        let ty = fy - y0 as f32;
        ny0[y] = y0;
        nsy[y] = ty * ty * (3.0 - 2.0 * ty);
    }

    // One fused pass: every layer writes its own cell independently, so the
    // final cell state equals the sequential ground/rings/beat/nodes result.
    measure_layer(NAME, "field", || {
        for y in 0..h {
            let vy = y as f32;
            let dyc = vy - cy;
            let dyb = vy - by;
            let gdy = (vy - h as f32 * 0.5) / h as f32;
            let row = y * w;
            let j = ny0[y];
            let sy = nsy[y];
            let base0 = j * xn_max;
            let base1 = (j + 1) * xn_max;
            for x in 0..w {
                let vx = x as f32 * 2.0;
                let warp = if warp_amp > 0.0 {
                    let i = nx0[x];
                    let sx = nsx[x];
                    let n00 = ntab[base0 + i];
                    let n10 = ntab[base0 + i + 1];
                    let n01 = ntab[base1 + i];
                    let n11 = ntab[base1 + i + 1];
                    let a = n00 + (n10 - n00) * sx;
                    let b = n01 + (n11 - n01) * sx;
                    (a + (b - a) * sy - 0.5) * warp_amp
                } else {
                    0.0
                };
                let dxc = vx - cx;
                let dxb = vx - bx;
                let ra = (dxc * dxc + dyc * dyc).sqrt() + warp;
                let rb = (dxb * dxb + dyb * dyb).sqrt() + warp;
                let qa = ring(ra, k.sa);
                let qb = ring(rb, k.sb);
                let m = (qa * qb).powf(k.contrast);

                let dx = (vx - w as f32) / w as f32;
                let t = (dx * dx + gdy * gdy).sqrt().min(1.0);
                let bg = lerp_color(pal[0], pal[2], t * 0.30);
                let dust = unit(seed, 10, (row + x) as u64) > 0.985;
                let ch0 = if dust { '.' } else { ' ' };
                let cell = &mut grid[y][x];
                *cell = Cell::with_bg(ch0, fg_ground, bg);

                if qa > 0.62 {
                    let ch = if qa > 0.9 { ':' } else { '.' };
                    *cell = Cell::new(ch, lerp_color(pal[0], pal[2], qa));
                } else if qb > 0.62 {
                    let ch = if qb > 0.9 { ':' } else { '.' };
                    *cell = Cell::new(ch, lerp_color(pal[0], pal[1], qb));
                }
                if m > 0.30 {
                    *cell = Cell::new(ramp(m), lerp_color(pal[1], pal[3], m));
                }
                if m > 0.86 {
                    *cell = Cell::new('@', pal[4]);
                }
            }
        }
    });
}

impl Mode for Moire {
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

    fn render_at(w: usize, h: usize, seed: u64, time: f32, args: &[&str]) -> String {
        let mut grid: Grid = vec![vec![Cell::blank(); w]; h];
        let palette = make_palette(seed);
        let mut rng = StdRng::seed_from_u64(seed);
        let argv: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        let mut frame = ModeFrame {
            grid: &mut grid,
            width: w,
            height: h,
            seed,
            palette: &palette,
            rng: &mut rng,
            time,
            args: &argv,
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
        insta::assert_snapshot!("moire_80x24", render_at(80, 24, 42, 0.0, &[]));
    }

    #[test]
    fn snapshot_t6_extra() {
        insta::assert_snapshot!("moire_t6", render_at(80, 24, 42, 6.0, &[]));
    }

    #[test]
    fn deterministic() {
        assert_eq!(
            render_at(80, 24, 42, 0.0, &[]),
            render_at(80, 24, 42, 0.0, &[])
        );
    }

    #[test]
    fn seed_sensitive() {
        assert_ne!(
            render_at(80, 24, 42, 0.0, &[]),
            render_at(80, 24, 43, 0.0, &[])
        );
    }

    #[test]
    fn time_sensitive() {
        assert_ne!(
            render_at(80, 24, 42, 0.0, &[]),
            render_at(80, 24, 42, 6.0, &[])
        );
    }

    #[test]
    fn frame_cost() {
        if cfg!(debug_assertions) {
            return;
        }
        let (w, h) = (200usize, 60usize);
        let args = vec![String::new(); 7];
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
        assert!(avg_ms < 6.0, "moire average frame {avg_ms:.3} ms exceeds 6 ms");
    }
}

#[cfg(test)]
mod bench {
    //! Ignored release-mode bench. Run:
    //! CARGO_BUILD_JOBS=4 cargo test --release -- --ignored --nocapture modes::_112_moire_
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
