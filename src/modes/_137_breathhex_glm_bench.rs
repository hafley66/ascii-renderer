//! Ignored release-mode bench for breathhex.
//! Run: CARGO_BUILD_JOBS=4 cargo test --release -- --ignored --nocapture modes::_137_breathhex_glm

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
        let t0 = std::time::Instant::now();
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
        let elapsed = t0.elapsed().as_secs_f64() * 1000.0 / FRAMES as f64;
        let layers = layer_capture_end();
        if elapsed < best {
            best = elapsed;
            best_layers = layers;
        }
    }
    (best, best_layers)
}

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
            for cell in row {
                hsh ^= cell.ch as u64;
                hsh = hsh.wrapping_mul(0x100000001b3);
            }
        }
    }
    hsh
}

#[test]
#[ignore]
fn bench() {
    for &(w, h) in &[(200usize, 60usize), (400usize, 120usize)] {
        let (avg_ms, layers) = bench_size(w, h);
        println!("\nbreathhex {}x{}: {:.3} ms/frame", w, h, avg_ms);
        let mut layers = layers;
        layers.sort_by(|a, b| b.total_ns.cmp(&a.total_ns));
        for l in &layers {
            println!("  {:10} {:9.3} ms", l.layer, l.total_ns as f64 / 1_000_000.0 / FRAMES as f64);
        }
        let hsh = checksum(w, h);
        println!("  checksum {:016x}", hsh);
        assert!(
            avg_ms < 6.0,
            "breathhex average frame {avg_ms:.3} ms exceeds 6 ms"
        );
    }
}
