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
            let u = i as f32 * TAU / N as f32;
            let load = (0.006 + 0.02 * u.sin().max(0.0)) * p[4] * (1.0 + 2.0 * m.strain[i]);
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
    // The membrane is an oblate organism; 1.25 lateral stretch is geometric,
    // in addition to the terminal's two-columns-per-row aspect correction.
    let scale = (width as f32 / 6.6).min(height as f32 / 3.05);
    Vertex {
        x: width as f32 * 0.5 + x * scale * 2.5,
        y: height as f32 * 0.34 + (y * s - v.z * c) * scale,
        z: y * c + v.z * s,
        energy: v.energy,
    }
}

#[derive(Clone, Copy)]
struct FacetPoint {
    point: Vertex,
    u: f32,
    v: f32,
    light: f32,
}

struct Canvas<'a> {
    grid: &'a mut Grid,
    depth: Vec<f32>,
    w: usize,
    h: usize,
    bg: Color,
    cool: [Color; 64],
    warm: [Color; 64],
}

fn direction(dx: f32, dy: f32) -> char {
    if dy.abs() < dx.abs() * 0.24 {
        '-'
    } else if dx.abs() < dy.abs() * 0.65 {
        '|'
    } else if dx * dy > 0.0 {
        '\\'
    } else {
        '/'
    }
}

impl Canvas<'_> {
    fn put(&mut self, x: isize, y: isize, z: f32, ch: char, light: f32, warm: bool) {
        if x < 0 || y < 0 || x as usize >= self.w || y as usize >= self.h {
            return;
        }
        let index = y as usize * self.w + x as usize;
        if z < self.depth[index] {
            return;
        }
        self.depth[index] = z;
        let level = (light.clamp(0.0, 1.0) * 63.0) as usize;
        self.grid[y as usize][x as usize] = Cell::with_bg(
            ch,
            if warm {
                self.warm[level]
            } else {
                self.cool[level]
            },
            self.bg,
        );
    }

    fn triangle(&mut self, tri: [FacetPoint; 3], phase: f32) {
        // Barycentric interpolation visits only the clipped triangle bounds.
        let [a, b, c] = tri;
        let edge = |a: Vertex, b: Vertex, x: f32, y: f32| {
            (b.x - a.x) * (y - a.y) - (b.y - a.y) * (x - a.x)
        };
        let area = edge(a.point, b.point, c.point.x, c.point.y);
        if area.abs() < 0.00001 {
            return;
        }
        let min_x = a.point.x.min(b.point.x).min(c.point.x).floor().max(0.0) as usize;
        let max_x = a
            .point
            .x
            .max(b.point.x)
            .max(c.point.x)
            .ceil()
            .min(self.w as f32) as usize;
        let min_y = a.point.y.min(b.point.y).min(c.point.y).floor().max(0.0) as usize;
        let max_y = a
            .point
            .y
            .max(b.point.y)
            .max(c.point.y)
            .ceil()
            .min(self.h as f32) as usize;
        let rib = direction(c.point.x - a.point.x, c.point.y - a.point.y);
        for y in min_y..max_y {
            for x in min_x..max_x {
                let wa = edge(b.point, c.point, x as f32 + 0.5, y as f32 + 0.5) / area;
                let wb = edge(c.point, a.point, x as f32 + 0.5, y as f32 + 0.5) / area;
                let wc = 1.0 - wa - wb;
                if wa < -0.0001 || wb < -0.0001 || wc < -0.0001 {
                    continue;
                }
                let z = a.point.z * wa + b.point.z * wb + c.point.z * wc;
                if z < self.depth[y * self.w + x] {
                    continue;
                }
                let u = a.u * wa + b.u * wb + c.u * wc;
                let v = a.v * wa + b.v * wb + c.v * wc;
                let energy = a.point.energy * wa + b.point.energy * wb + c.point.energy * wc;
                let base = a.light * wa + b.light * wb + c.light * wc;
                let ridge = (24.0 * u + 1.8 * v.sin() + phase).cos().max(0.0).powi(6);
                let rings = (12.0 * v + 2.0 * (3.0 * u + phase).sin())
                    .cos()
                    .max(0.0)
                    .powi(12);
                let light = (base + ridge * 0.22 + energy * 0.14).clamp(0.0, 1.0);
                let ch = if ridge > 0.48 {
                    rib
                } else if rings > 0.75 && self.w >= 110 {
                    '='
                } else {
                    b"..,:;=+*#%"[(light * 9.0) as usize] as char
                };
                self.put(
                    x as isize,
                    y as isize,
                    z,
                    ch,
                    light,
                    v.sin() > 0.3 && energy > 0.44,
                );
            }
        }
    }

    fn line(&mut self, a: Vertex, b: Vertex, light: f32, warm: bool, bead: bool) {
        let dx = b.x - a.x;
        let dy = b.y - a.y;
        let steps = (dx.abs().max(dy.abs()).ceil() as usize).clamp(1, 4096);
        let ch = if bead { 'o' } else { direction(dx, dy) };
        for i in 0..=steps {
            let f = i as f32 / steps as f32;
            self.put(
                (a.x + f * dx).floor() as isize,
                (a.y + f * dy).floor() as isize,
                a.z + (b.z - a.z) * f + 0.006,
                ch,
                light,
                warm,
            );
        }
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
    let mut canvas = Canvas {
        grid: frame.grid,
        depth: vec![f32::NEG_INFINITY; w * h],
        w,
        h,
        bg,
        cool: std::array::from_fn(|i| {
            let t = i as f32 / 63.0;
            lerp_color(
                lerp_color(rgb(12, 43, 64), rgb(96, 204, 200), t),
                frame.palette[3],
                0.12,
            )
        }),
        warm: std::array::from_fn(|i| {
            lerp_color(rgb(56, 80, 79), rgb(247, 224, 165), i as f32 / 63.0)
        }),
    };
    // Geometry sampling is resolution-adaptive, independent of artistic knobs.
    let nu = (w * 2).clamp(96, 256);
    let nv = h.clamp(24, 64);
    let mut mesh = Vec::with_capacity((nu + 1) * (nv + 1));
    for i in 0..=nu {
        for j in 0..=nv {
            let u = i as f32 * TAU / nu as f32;
            let v = j as f32 * TAU / nv as f32;
            let world = surface(&mantle, u, v, p);
            let normal = [v.cos() * u.cos(), v.cos() * u.sin(), v.sin()];
            let diffuse = (-0.35 * normal[0] + 0.25 * normal[1] + 0.9 * normal[2]).max(0.0);
            let rim = (1.0 - (normal[1] * 0.84 + normal[2] * 0.54).abs())
                .max(0.0)
                .powi(3);
            mesh.push(FacetPoint {
                point: project(world, w, h, mantle.clock),
                u,
                v,
                light: 0.14 + diffuse * 0.53 + rim * 0.23,
            });
        }
    }
    measure_layer("pelagium", "membrane", || {
        for i in 0..nu {
            for j in 0..nv {
                let a = i * (nv + 1) + j;
                let b = a + nv + 1;
                canvas.triangle([mesh[a], mesh[b], mesh[a + 1]], mantle.phase[3]);
                canvas.triangle([mesh[b], mesh[b + 1], mesh[a + 1]], mantle.phase[3]);
            }
        }
        // Two material sutures follow the same deforming surface. Occlusion
        // hides their far arcs; energy brightens the exposed inner lip.
        for v in [0.55, 2.55] {
            let at = |u: f32| project(surface(&mantle, u, v, p), w, h, mantle.clock);
            let mut a = at(0.0);
            for i in 1..=256 {
                let u = i as f32 * TAU / 256.0;
                let b = at(u);
                canvas.line(a, b, 0.4 + b.energy * 0.65, v > 2.0, false);
                a = b;
            }
        }
    });
    // Filaments continue the membrane's fixed material meridians. The energy
    // field that they load also governs their excursion and travelling light.
    measure_layer("pelagium", "filaments", || {
        for i in 0..12 {
            let u = (i as f32 + 0.5) * PI / 12.0;
            let root = surface(&mantle, u, -PI * 0.5, p);
            let e = root.energy;
            let gradient =
                (sample(&mantle.energy, u + 0.12) - sample(&mantle.energy, u - 0.12)) / 0.24;
            let length = (0.48 + 0.90 * p[4]) * (0.6 + e);
            let at = |s: f32| {
                let swirl = u
                    + p[2] * s * (0.35 + 1.6 * gradient)
                    + 0.5 * s * s * (mantle.clock * 0.35 + u * 2.0).sin();
                let radius = (root.x * root.x + root.y * root.y).sqrt() * (1.0 - 0.32 * s)
                    + 0.13 * p[2] * (s * PI).sin() * (7.0 * s - mantle.clock + u * 2.0).sin();
                project(
                    Vertex {
                        x: radius * swirl.cos(),
                        y: radius * swirl.sin(),
                        z: root.z - length * s,
                        energy: e,
                    },
                    w,
                    h,
                    mantle.clock,
                )
            };
            let mut a = at(0.0);
            for j in 1..=128 {
                let s = j as f32 / 128.0;
                let b = at(s);
                let pulse = (s * 18.0 - mantle.clock * 2.3 + u * 3.0)
                    .cos()
                    .max(0.0)
                    .powi(10);
                let light = (0.22 + e * 0.42 + pulse * 0.3) * (1.0 - 0.5 * s);
                canvas.line(a, b, light, pulse > 0.72, pulse > 0.93 && w >= 110);
                a = b;
            }
        }
    });
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
    fn pelagium_canonical() {
        insta::assert_snapshot!(
            "pelagium_canonical",
            plain(&frame(80, 24, 42, 0.0, &defaults()))
        );
    }

    macro_rules! snapshot {
        ($name:ident, $w:expr, $h:expr, $time:expr, $knobs:expr) => {
            #[test]
            fn $name() {
                insta::assert_snapshot!(
                    stringify!($name),
                    plain(&frame($w, $h, 42, $time, &$knobs))
                );
            }
        };
    }
    snapshot!(pelagium_t4, 80, 24, 4.0, defaults());
    snapshot!(pelagium_t11, 80, 24, 11.0, defaults());
    snapshot!(pelagium_open, 80, 24, 4.0, [0.82, 0.2, 0.95, 0.6, 0.4]);
    snapshot!(pelagium_small, 24, 9, 0.0, defaults());
    snapshot!(
        pelagium_aperture049,
        80,
        24,
        0.0,
        [0.49, 0.55, 0.7, 0.6, 0.65]
    );
    snapshot!(
        pelagium_aperture051,
        80,
        24,
        0.0,
        [0.51, 0.55, 0.7, 0.6, 0.65]
    );
    snapshot!(
        pelagium_max,
        80,
        24,
        7.0,
        std::array::from_fn(|i| PARAMS[i].max)
    );

    #[test]
    fn registered_frames_preserve_identity_and_live_controls() {
        let mode = crate::registry::registered_mode("pelagium").unwrap();
        assert_eq!(mode.name(), MODE.name());
        assert!(mode.animation() == AnimKind::Iterate);
        let p = defaults();
        let canonical = frame(80, 24, 42, 4.0, &p);
        for seed in [7, 913] {
            assert_ne!(canonical, frame(80, 24, seed, 4.0, &p));
        }
        assert_ne!(canonical, frame(80, 24, 42, 11.0, &p));
        for i in 0..PARAMS.len() {
            let mut changed = p;
            changed[i] = PARAMS[i].max;
            assert_ne!(
                canonical,
                frame(80, 24, 42, 4.0, &changed),
                "{}",
                PARAMS[i].key
            );
        }
        // Re-entry after seeds, time and knobs change must reproduce every color.
        assert_eq!(canonical, frame(80, 24, 42, 4.0, &p));
        let a = frame(80, 24, 42, 4.0, &[0.49, 0.55, 0.7, 0.6, 0.65]);
        let b = frame(80, 24, 42, 4.0, &[0.51, 0.55, 0.7, 0.6, 0.65]);
        let changed = a
            .iter()
            .flatten()
            .zip(b.iter().flatten())
            .filter(|(a, b)| a.ch != b.ch)
            .count();
        assert!(
            changed > 0 && changed < 80 * 24 / 5,
            "nearby control changed {changed} cells"
        );
    }

    #[test]
    fn clipping_and_invalid_knobs_are_bounded() {
        for (w, h) in [(0, 0), (0, 7), (1, 1), (2, 3), (7, 2), (24, 9)] {
            for p in [
                defaults(),
                std::array::from_fn(|i| PARAMS[i].min),
                std::array::from_fn(|i| PARAMS[i].max),
            ] {
                let result = frame(w, h, 913, 11.0, &p);
                assert_eq!(result.len(), h);
                assert!(
                    result
                        .iter()
                        .all(|row| row.len() == w && row.iter().all(|cell| cell.ch.is_ascii()))
                );
            }
        }
        assert_eq!(
            frame(80, 24, 42, 4.0, &defaults()),
            frame(80, 24, 42, 4.0, &[f32::NAN; 5])
        );
    }

    #[test]
    fn energy_and_strain_have_reciprocal_effects() {
        let mut p = defaults();
        let coupled = solve(42, 4.0, &p);
        p[2] = 0.0;
        let uncoupled = solve(42, 4.0, &p);
        assert_eq!(uncoupled.strain, [0.0; N]);
        assert!(coupled.strain.iter().any(|x| x.abs() > 0.02));
        assert!(
            coupled
                .energy
                .iter()
                .zip(uncoupled.energy)
                .any(|(a, b)| (a - b).abs() > 0.001)
        );
        // Each primary dimension traverses one continuous geometric system.
        for index in 0..5 {
            let mut previous = None;
            for f in [0.0, 0.25, 0.49, 0.5, 0.51, 0.75, 1.0] {
                let mut p = defaults();
                p[index] = PARAMS[index].min + (PARAMS[index].max - PARAMS[index].min) * f;
                let m = solve(42, 4.0, &p);
                let points: Vec<_> = (0..32)
                    .map(|i| surface(&m, i as f32 * TAU / 32.0, 1.2, &p))
                    .collect();
                if let Some((f0, before)) = previous {
                    let before: Vec<Vertex> = before;
                    let movement = points
                        .iter()
                        .zip(before)
                        .map(|(a, b)| (a.x - b.x).abs() + (a.y - b.y).abs() + (a.z - b.z).abs())
                        .sum::<f32>()
                        / 32.0;
                    assert!(
                        movement < 2.0 * (f - f0),
                        "{} moved {movement}",
                        PARAMS[index].key
                    );
                }
                previous = Some((f, points));
            }
        }
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
