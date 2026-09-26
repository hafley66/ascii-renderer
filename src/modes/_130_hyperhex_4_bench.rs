//! Ignored release-mode bench for hyperhex-4.
//! Run: CARGO_BUILD_JOBS=4 cargo test --release -- --ignored --nocapture modes::_130_hyperhex_4

use super::*;
use crate::_0_profile::{LayerTotal, layer_capture_begin, layer_capture_end};
use crate::color::make_palette;
use crate::registry::ModeFrame;
use crate::types::{Cell, Grid};
use rand::SeedableRng;
use rand::rngs::StdRng;

const FRAMES: usize = 60;
const ROUNDS: usize = 3;

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

/// FNV-1a over the plain chars of every frame in t = 0..FRAMES, so byte identity
/// of an optimization can be checked across the whole sweep, not just snapshots.
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
        println!("hyperhex-4 {w}x{h} checksum: {:016x}", checksum(w, h));
        let (ms, layers) = bench_size(w, h);
        println!("hyperhex-4 {w}x{h}: {ms:.3} ms/frame");
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
