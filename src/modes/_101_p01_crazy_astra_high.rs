//! Pelagium: a luminous, strained membrane suspended around a dark oculus.
//! Signatures, frame lifetime, bounds and feedback: perf/101_pelagium.md.
use crate::_0_profile::measure_layer;
use crate::color::{darken, lerp_color, rgb};
use crate::opts::param_f32;
use crate::registry::{AnimKind, Mode, ModeFrame, Param};
use crate::types::{Cell, Grid};
use crossterm::style::Color;
use std::f32::consts::{PI, TAU};

pub(super) struct Pelagium;
pub(super) static MODE: Pelagium = Pelagium;
const N: usize = 128;
const PARAMS: &[Param] = &[
    param!("APERTURE", "oculus aperture", 0.0, 1.0, 0.5, 0.02),
    param!("TENSION", "membrane tension", 0.0, 1.0, 0.55, 0.02),
    param!("COUPLING", "energy / strain", 0.0, 1.0, 0.7, 0.02),
    param!("FLOW", "current speed", 0.0, 2.0, 0.6, 0.05),
    param!("TRAIL", "filament loading", 0.0, 1.0, 0.65, 0.02),
];

impl Mode for Pelagium {
    fn name(&self) -> &'static str {
        "pelagium"
    }
    fn help(&self) -> &'static str {
        "pelagium: a pleated abyssal crown and energy-bearing filaments [aperture] [tension] [coupling] [flow] [trail]"
    }
    fn animation(&self) -> AnimKind {
        AnimKind::Iterate
    }
    fn params(&self) -> &'static [Param] {
        PARAMS
    }
    fn render(&self, frame: &mut ModeFrame<'_>) {
        // Resolve once; the native player, CLI and persisted knobs share defaults.
        let p = std::array::from_fn(|i| {
            let spec = &PARAMS[i];
            let value = frame
                .args
                .get(i + 4)
                .and_then(|s| s.parse::<f32>().ok())
                .or_else(|| frame.param_values.and_then(|v| v.get(i)).copied())
                .unwrap_or_else(|| param_f32(spec.key, spec.default));
            if value.is_finite() {
                value.clamp(spec.min, spec.max)
            } else {
                spec.default
            }
        });
        draw(frame, &p);
    }
}

fn unit(seed: u64, slot: u64) -> f32 {
    let mut z = seed.wrapping_add(slot.wrapping_mul(0x9e3779b97f4a7c15));
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
    ((z ^ (z >> 31)) >> 40) as f32 / 16_777_216.0
}

struct Mantle {
    energy: [f32; N],
    strain: [f32; N],
    phase: [f32; 4],
    clock: f32,
}

fn solve(seed: u64, time: f32, p: &[f32; 5]) -> Mantle {
    // Fixed identities and analytic forcing. Every frame reconstructs 32 steps.
    let clock = if time.is_finite() { time * p[3] } else { 0.0 };
    let phase = std::array::from_fn(|i| unit(seed, i as u64 + 1) * TAU);
    let source: [f32; N] = std::array::from_fn(|i| {
        let u = i as f32 * TAU / N as f32;
        0.55 + 0.23 * (3.0 * u + phase[0] - clock * 0.7).sin()
            + 0.13 * (5.0 * u + phase[1] + clock * 0.4 + p[0]).cos()
    });
    let mut m = Mantle {
        energy: source,
        strain: [0.0; N],
        phase,
        clock,
    };
    for _ in 0..32 {
        let mut energy = m.energy;
        let mut strain = m.strain;
        for i in 0..N {
            let a = (i + N - 1) % N;
            let b = (i + 1) % N;
            // Edge conductance is symmetric: transport conserves energy before
            // source and filament drain. Strain throttles neighboring currents.
            let flow = [a, b]
                .iter()
                .map(|&j| {
                    (m.energy[j] - m.energy[i]) * 0.18
                        / (1.0 + 50.0 * p[2] * (m.strain[j] - m.strain[i]).abs())
                })
                .sum::<f32>();
            let load = 0.016 * p[4] * (1.0 + 2.0 * m.strain[i]);
            energy[i] += flow + 0.065 * (source[i] - m.energy[i]) - load * m.energy[i];
            let target = p[2] * (m.energy[i] - 0.42) * 0.75;
            strain[i] += 0.16
                * (target - m.strain[i]
                    + p[1] * 1.8 * (m.strain[a] + m.strain[b] - 2.0 * m.strain[i]));
        }
        m.energy = energy;
        m.strain = strain;
    }
    m
}

fn sample(field: &[f32; N], u: f32) -> f32 {
    let index = u.rem_euclid(TAU) * N as f32 / TAU;
    let i = index.floor() as usize % N;
    field[i] + (field[(i + 1) % N] - field[i]) * index.fract()
}

#[derive(Clone, Copy, Default)]
struct Vertex {
    x: f32,
    y: f32,
    z: f32,
    energy: f32,
}

fn surface(m: &Mantle, u: f32, v: f32, p: &[f32; 5]) -> Vertex {
    // One periodic membrane, pleated around the same energetic meridians.
    let energy = sample(&m.energy, u);
    let strain = sample(&m.strain, u);
    let r = 0.88 + strain + 0.06 * (2.0 * u + m.phase[2]).cos();
    let tube = 0.34 - 0.19 * p[0];
    let pleat = (24.0 * u + 1.8 * v.sin() + m.phase[3]).cos();
    let fold = (0.024 + p[2] * 0.048) * pleat * (0.5 + energy);
    let radius = r + (tube + fold) * v.cos();
    Vertex {
        x: radius * u.cos(),
        y: radius * u.sin(),
        z: (tube * (0.75 + 0.65 * p[1]) + fold) * v.sin() + 0.08 * (3.0 * u + m.phase[0]).sin(),
        energy,
    }
}

fn project(v: Vertex, width: usize, height: usize, time: f32) -> Vertex {
    let angle = 0.12 * (time * 0.17).sin();
    let (s, c) = angle.sin_cos();
    let x = c * v.x - s * v.y;
    let y = s * v.x + c * v.y;
    let tilt = 0.58 + 0.12 * (time * 0.13).sin();
    let (s, c) = tilt.sin_cos();
    let scale = (width as f32 / 5.6).min(height as f32 / 3.7);
    Vertex {
        x: width as f32 * 0.5 + x * scale * 2.0,
        y: height as f32 * 0.30 + (y * s - v.z * c) * scale,
        z: y * c + v.z * s,
        energy: v.energy,
    }
}

fn draw(frame: &mut ModeFrame<'_>, p: &[f32; 5]) {
    let (w, h) = (frame.width, frame.height);
    if w == 0 || h == 0 {
        return;
    }
    let mantle = measure_layer("pelagium", "coupled-mantle", || {
        solve(frame.seed, frame.time, p)
    });
    let bg = lerp_color(rgb(2, 7, 13), darken(frame.palette[0], 10), 0.18);
    for row in frame.grid.iter_mut().take(h) {
        for cell in row.iter_mut().take(w) {
            *cell = Cell::with_bg(' ', bg, bg);
        }
    }
    let mut depth = vec![f32::NEG_INFINITY; w * h];
    // Foundation view: bounded projected samples with depth-tested engraving.
    for i in 0..256 {
        for j in 0..64 {
            let v = project(
                surface(&mantle, i as f32 * TAU / 256.0, j as f32 * TAU / 64.0, p),
                w,
                h,
                mantle.clock,
            );
            let (x, y) = (v.x.round() as isize, v.y.round() as isize);
            if x < 0 || y < 0 || x as usize >= w || y as usize >= h {
                continue;
            }
            let index = y as usize * w + x as usize;
            if v.z > depth[index] {
                depth[index] = v.z;
                let light = (v.energy + 0.3 * v.z).clamp(0.0, 1.0);
                let glyph = b".,:;=+*#@"[(light * 8.0) as usize] as char;
                frame.grid[y as usize][x as usize] = Cell::with_bg(
                    glyph,
                    lerp_color(rgb(27, 88, 109), rgb(225, 231, 187), light),
                    bg,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{SeedableRng, rngs::StdRng};

    fn defaults() -> [f32; 5] {
        std::array::from_fn(|i| PARAMS[i].default)
    }
    fn frame(w: usize, h: usize, seed: u64, time: f32, p: &[f32; 5]) -> Grid {
        let mut grid = vec![vec![Cell::blank(); w]; h];
        MODE.render(&mut ModeFrame {
            grid: &mut grid,
            width: w,
            height: h,
            seed,
            palette: &crate::color::named_theme("deep").unwrap(),
            rng: &mut StdRng::seed_from_u64(seed),
            time,
            args: &[],
            param_values: Some(p),
        });
        grid
    }
    fn plain(g: &Grid) -> String {
        crate::render::grid_to_plain(g).join("\n")
    }

    #[test]
    fn pelagium_foundation() {
        insta::assert_snapshot!(
            "pelagium_canonical",
            plain(&frame(80, 24, 42, 0.0, &defaults()))
        );
    }

    #[test]
    fn coupled_state_is_bounded_and_continuous() {
        for seed in [0, 42, u64::MAX] {
            for time in [0.0, 3.0, 20.0] {
                let p = std::array::from_fn(|i| PARAMS[i].max);
                let m = solve(seed, time, &p);
                assert!(
                    m.energy
                        .iter()
                        .all(|e| e.is_finite() && *e >= 0.0 && *e <= 1.0)
                );
                assert!(m.strain.iter().all(|s| s.is_finite() && s.abs() < 0.5));
                let nearby = solve(seed, time + 0.001, &p);
                assert!(
                    m.energy
                        .iter()
                        .zip(nearby.energy)
                        .all(|(a, b)| (a - b).abs() < 0.002)
                );
                let mut unloaded = p;
                unloaded[4] = 0.0;
                assert!(
                    solve(seed, time, &unloaded).energy.iter().sum::<f32>()
                        > m.energy.iter().sum::<f32>()
                );
            }
        }
    }
}
