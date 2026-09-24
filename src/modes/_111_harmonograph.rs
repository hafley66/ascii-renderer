/*
AI art prompt sequence, verbatim:

make art! ignore others tests failing it happens, this is an open air debris
garden of me and ai art. do not read the others art it will taint you. i run
this with animations and random knobs and seed hitting right arrow and back (b)
and save(s) so lets keep that in mind. we want beautiful render frames and
beautiful variability across the knob-space. try un peu performance test, and
use the ascii canvas. u will see filenames of other art, try to avoid it the
best u can, good luck! make something beautiful, a scene, a maths, an illusion,
algorithmic form and essence.
*/
use crate::_0_profile::measure_layer;
use crate::registry::{AnimKind, Mode, ModeFrame, Param};
use crate::types::{Cell, Grid};
use crossterm::style::Color;

pub(super) static MODE: Harmonograph = Harmonograph;

pub(super) struct Harmonograph;

static PARAMS: [Param; 6] = [
    Param {
        key: "freq_x",
        label: "Freq X",
        min: 1.0,
        max: 9.0,
        default: 3.0,
        step: 1.0,
        choices: &[],
        randomize: true,
    },
    Param {
        key: "freq_y",
        label: "Freq Y",
        min: 1.0,
        max: 9.0,
        default: 4.0,
        step: 1.0,
        choices: &[],
        randomize: true,
    },
    Param {
        key: "drift",
        label: "Drift",
        min: 0.0,
        max: 0.30,
        default: 0.08,
        step: 0.01,
        choices: &[],
        randomize: true,
    },
    Param {
        key: "wobble",
        label: "Wobble",
        min: 0.0,
        max: 0.6,
        default: 0.2,
        step: 0.01,
        choices: &[],
        randomize: true,
    },
    Param {
        key: "density",
        label: "Density",
        min: 0.35,
        max: 3.0,
        default: 1.25,
        step: 0.05,
        choices: &[],
        randomize: true,
    },
    Param {
        key: "speed",
        label: "Speed",
        min: 0.1,
        max: 2.0,
        default: 0.8,
        step: 0.05,
        choices: &[],
        randomize: true,
    },
];

fn param(frame: &ModeFrame<'_>, index: usize) -> f32 {
    let p = &PARAMS[index];
    frame
        .param_values
        .and_then(|values| values.get(index).copied())
        .unwrap_or_else(|| crate::opts::param_f32(p.key, p.default))
        .clamp(p.min, p.max)
}

/// xy-position of the pen after `steps` samples of the pendulum system.
/// x = sin(fx*t + drift*t) with seed-phase, y likewise; wobble adds a small
/// secondary pendulum so low-frequency combos still dance.
#[allow(clippy::many_single_char_names)]
fn pen(
    t: f32,
    fx: f32,
    fy: f32,
    drift: f32,
    wobble: f32,
    px: f32,
    py: f32,
) -> (f32, f32) {
    let x = (fx * t + px).sin() + wobble * (0.5 * fx * t - px * 1.7).sin();
    let y = (fy * t + py).cos() + wobble * (0.5 * fy * t + py * 2.3).sin();
    let damp = (-drift * t).exp().max(0.05);
    (x * damp, y * damp)
}

struct Brush {
    ch: char,
    color: usize,
}

impl Brush {
    fn pick(steps: usize) -> Self {
        // Head of the trail burns brightest, tail cools through the palette.
        match steps {
            0..=2 => Self { ch: '@', color: 4 },
            3..=8 => Self { ch: 'O', color: 4 },
            9..=20 => Self { ch: 'o', color: 3 },
            21..=44 => Self { ch: '.', color: 2 },
            _ => Self { ch: ',', color: 1 },
        }
    }
}

fn stamp(grid: &mut Grid, x: i32, y: i32, brush: &Brush, palette: &[Color; 5]) {
    if x < 0 || y < 0 {
        return;
    }
    let (ux, uy) = (x as usize, y as usize);
    if uy >= grid.len() || ux >= grid[uy].len() {
        return;
    }
    let cell = &mut grid[uy][ux];
    // Later (brighter) passes overwrite; faint dust fills gaps.
    if matches!(cell.ch, '@' | 'O') && matches!(brush.ch, '.' | ',') {
        return;
    }
    *cell = Cell::new(brush.ch, palette[brush.color]);
}

impl Mode for Harmonograph {
    fn name(&self) -> &'static str {
        "harmonograph"
    }

    fn help(&self) -> &'static str {
        "A damped pendulum draws lissajous embroidery in fading thread"
    }

    fn animation(&self) -> AnimKind {
        AnimKind::Iterate
    }

    fn params(&self) -> &'static [Param] {
        &PARAMS
    }

    fn render(&self, frame: &mut ModeFrame<'_>) {
        let width = frame.width;
        let height = frame.height;
        if width < 4 || height < 2 {
            return;
        }
        for row in frame.grid.iter_mut() {
            for cell in row.iter_mut() {
                *cell = Cell::blank();
            }
        }

        let fx = param(frame, 0);
        let fy = param(frame, 1);
        let drift = param(frame, 2);
        let wobble = param(frame, 3);
        let density = param(frame, 4);
        let speed = param(frame, 5);

        let palette = frame.palette;
        let seed = frame.seed;
        // Per-seed choreography: phase of each pendulum, breathing shift.
        let mut s = seed ^ 0xdead_beef_cafe_babe;
        let mut next = || {
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            s
        };
        let px = (next() % 1000) as f32 / 1000.0 * std::f32::consts::TAU;
        let py = (next() % 1000) as f32 / 1000.0 * std::f32::consts::TAU;
        let breathe = (frame.time * 0.7).sin() * 0.06;
        let cx = width as f32 / 2.0;
        let cy = height as f32 / 2.0;
        // Terminal cells are ~2x taller than wide: fit ellipse of radius 1.
        let axScale = (width as f32 - 2.0) / 2.4;
        let ayScale = (height as f32 - 1.0) / 2.4;
        let steps = (220.0 * density) as usize;
        let t0 = frame.time * speed;

        // Moire ripple rings behind the embroidery, breathing with time.
        let rings = ((f32::min(axScale, ayScale * 2.0)) / 6.0).max(2.0) as i32;
        let ring_phase = frame.time * 0.9 + (seed % 97) as f32 * 0.1;
        let width = frame.width;
        let height = frame.height;
        let grid = &mut *frame.grid;
        measure_layer("harmonograph", "rings", || {
            for ring in 1..=rings {
                let rr = ring as f32 * 5.5 + (ring_phase + ring as f32 * 0.9).sin() * 2.0;
                if rr < 3.0 {
                    continue;
                }
                // Rare rings shimmer.
                let shade = if (ring + (ring_phase * 2.0).floor() as i32) % 4 == 0 {
                    2
                } else {
                    0
                };
                let step = (std::f32::consts::TAU / (rr * 2.5)).max(0.02);
                let mut a = 0.0;
                while a < std::f32::consts::TAU {
                    let gx = cx + a.cos() * rr;
                    let gy = cy + (a.sin() * rr) / 2.0;
                    if gx >= 0.0 && gy >= 0.0 {
                        let (ux, uy) = (gx as usize, gy as usize);
                        if uy < height && ux < width {
                            // Sparse dust ring: only every few angle steps.
                            if ((a * 57.29578) as i32 + ring * 3) % 9 < 2 {
                                grid[uy][ux] = Cell::new('.', palette[shade]);
                            }
                        }
                    }
                    a += step * 3.0;
                }
            }
        });
        measure_layer("harmonograph", "trace", || {
            // Trace the pendulum path, oldest first so newest stamps brightest.
            for i in (0..steps).rev() {
                let t = t0 + i as f32 * 0.075;
                let (nx, ny) = pen(t, fx, fy, drift, wobble + breathe, px, py);
                let gx = cx + nx * axScale;
                let gy = cy + ny * ayScale;
                if gx < 0.0 || gy < 0.0 {
                    continue;
                }
                let (ux, uy) = (gx as i32, gy as i32);
                let brush = Brush::pick(steps - i);
                stamp(grid, ux, uy, &brush, palette);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::ModeRegistry;
    use crossterm::style::Color;
    use rand::{rngs::StdRng, SeedableRng};

    fn grid_to_string(grid: &Grid) -> String {
        grid.iter()
            .map(|row| row.iter().map(|c| c.ch).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn renders_curve() {
        let mut grid: Grid = vec![vec![Cell::blank(); 80]; 24];
        let mut rng = StdRng::seed_from_u64(42);
        let palette = [
            Color::DarkGrey,
            Color::Grey,
            Color::White,
            Color::Yellow,
            Color::Magenta,
        ];
        let mut frame = ModeFrame {
            grid: &mut grid,
            width: 80,
            height: 24,
            seed: 42,
            palette: &palette,
            rng: &mut rng,
            time: 0.0,
            args: &[],
            param_values: None,
        };
        Harmonograph.render(&mut frame);
        let s = grid_to_string(&grid);
        assert!(s.contains('@'), "expected bright pen head");
    }

    #[test]
    fn snapshot_80x24() {
        let mut grid: Grid = vec![vec![Cell::blank(); 80]; 24];
        let mut rng = StdRng::seed_from_u64(7);
        let palette = [
            Color::DarkGrey,
            Color::Grey,
            Color::White,
            Color::Yellow,
            Color::Magenta,
        ];
        let mut frame = ModeFrame {
            grid: &mut grid,
            width: 80,
            height: 24,
            seed: 7,
            rng: &mut rng,
            time: 1.3,
            args: &[],
            param_values: None,
            palette: &palette,
        };
        Harmonograph.render(&mut frame);
        insta::assert_snapshot!("harmonograph_80x24", grid_to_string(&grid));
    }

    #[test]
    fn registers() {
        let mut r = ModeRegistry::default();
        r.add(&MODE);
        assert_eq!(r.get("harmonograph").map(|_| ()), Some(()));
    }
}

#[cfg(test)]
mod probe {
    use super::*;
    use rand::{rngs::StdRng, SeedableRng};

    #[test]
    #[ignore = "frame dump; run with --ignored --nocapture"]
    fn dump_frames() {
        let palette = [
            crossterm::style::Color::DarkGrey,
            crossterm::style::Color::Grey,
            crossterm::style::Color::White,
            crossterm::style::Color::Yellow,
            crossterm::style::Color::Magenta,
        ];
        for t in [0.0f32, 3.0, 7.0, 12.0] {
            let mut grid: Grid = vec![vec![Cell::blank(); 100]; 40];
            let mut rng = StdRng::seed_from_u64(11);
            let mut frame = ModeFrame {
                grid: &mut grid,
                width: 100,
                height: 40,
                seed: 11,
                palette: &palette,
                rng: &mut rng,
                time: t,
                args: &[],
                param_values: None,
            };
            Harmonograph.render(&mut frame);
            eprintln!("=== t={t}");
            for row in grid.iter() {
                let line: String = row.iter().map(|c| c.ch).collect();
                eprintln!("{line}");
            }
        }
    }
}

