//! O(width * height) terminal-output stress; fixed 256-entry stack wave table.
//! Frame-owned inputs only. Density/churn/color/background maxima increase output
//! work without adding nested passes or geometry allocations.
use crate::opts::param_f32;
use crate::registry::{AnimKind, Mode, ModeFrame, Param};
use crate::types::Cell;
use crossterm::style::Color;

pub(super) struct TerminalStress;
pub(super) static MODE: TerminalStress = TerminalStress;
const PARAMS: &[Param] = &[
    param!("DENSITY", "occupied cells", 0.0, 1.0, 0.85, 0.05),
    param!("CHURN", "cells changing over time", 0.0, 1.0, 1.0, 0.05),
    param!("COLORS", "indexed color diversity", 1.0, 216.0, 32.0, 1.0),
    param!(
        "BACKGROUND",
        "colored background density",
        0.0,
        1.0,
        0.25,
        0.05
    ),
    param!("FREQUENCY", "spatial frequency", 1.0, 32.0, 5.0, 1.0),
    param!("SPEED", "pattern phases per second", 1.0, 120.0, 30.0, 1.0),
    param!(
        "GLYPHS",
        "ASCII to single-width Unicode",
        0.0,
        1.0,
        0.0,
        1.0
    ),
    param!("PATTERN", "waves / rings / noise", 0.0, 2.0, 0.0, 1.0),
];

impl Mode for TerminalStress {
    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
    fn name(&self) -> &'static str {
        "terminal-stress"
    }
    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
    fn help(&self) -> &'static str {
        "terminal-stress: animated waves, rings and noise with bounded output-stress knobs"
    }
    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
    fn animation(&self) -> AnimKind {
        AnimKind::Iterate
    }
    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
    fn params(&self) -> &'static [Param] {
        PARAMS
    }
    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
    fn render(&self, frame: &mut ModeFrame<'_>) {
        // Resolve declared controls once, then precompute one periodic wave table.
        let values: [f32; 8] = std::array::from_fn(|i| {
            frame
                .param_values
                .and_then(|v| v.get(i))
                .copied()
                .unwrap_or_else(|| param_f32(PARAMS[i].key, PARAMS[i].default))
                .clamp(PARAMS[i].min, PARAMS[i].max)
        });
        let density = (values[0] * 1024.0) as u32;
        let churn = (values[1] * 1024.0) as u32;
        let colors = values[2] as u32;
        let background = (values[3] * 1024.0) as u32;
        let frequency = values[4] as u32;
        let tick = (frame.time.max(0.0) * values[5]).round() as u32;
        let glyphs = if values[6] >= 0.5 {
            ['·', '░', '▒', '▓', '█', '╳', '╬', '⣿']
        } else {
            ['.', ':', '+', 'o', 'x', '%', '#', '@']
        };
        let waves: [i32; 256] = std::array::from_fn(|i| {
            ((i as f32 * std::f32::consts::TAU / 256.0).sin() * 127.0) as i32
        });
        // Evaluate each cell directly from seed, position and time. Every knob
        // preserves one visit per existing grid cell, including simultaneous max.
        for (y, row) in frame.grid.iter_mut().enumerate() {
            for (x, cell) in row.iter_mut().enumerate() {
                let mut hash = (x as u32).wrapping_mul(0x9e3779b9)
                    ^ (y as u32).wrapping_mul(0x85ebca6b)
                    ^ frame.seed as u32;
                hash ^= hash >> 16;
                hash = hash.wrapping_mul(0x7feb352d);
                hash ^= hash >> 15;
                if hash & 1023 >= density {
                    *cell = Cell {
                        ch: ' ',
                        fg: Color::Reset,
                        bg: Color::Reset,
                    };
                    continue;
                }
                let phase = if (hash >> 10) & 1023 < churn { tick } else { 0 };
                let u = (x as u32).wrapping_mul(frequency).wrapping_add(phase);
                let v = (y as u32)
                    .wrapping_mul(frequency * 2)
                    .wrapping_add(frame.seed as u32);
                let value = match values[7] as u32 {
                    0 => ((waves[(u & 255) as usize] + waves[(v & 255) as usize] + 256) / 2) as u32,
                    1 => {
                        let radius = x.abs_diff(frame.width / 2) + y.abs_diff(frame.height / 2) * 2;
                        (waves[((radius as u32).wrapping_mul(frequency).wrapping_add(phase) & 255)
                            as usize]
                            + 128) as u32
                    }
                    _ => hash.wrapping_add(phase.wrapping_mul(0x45d9f3b)) ^ phase.rotate_left(13),
                };
                *cell = Cell {
                    ch: glyphs[(value as usize + phase as usize) % glyphs.len()],
                    fg: Color::AnsiValue((16 + value.wrapping_add(phase) % colors) as u8),
                    bg: if (hash >> 20) & 1023 < background {
                        Color::AnsiValue(
                            (16 + (value / 3).wrapping_add(phase.wrapping_mul(7)) % colors) as u8,
                        )
                    } else {
                        Color::Reset
                    },
                };
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{SeedableRng, rngs::StdRng};
    fn render(width: usize, height: usize, time: f32, max: bool) -> crate::types::Grid {
        let mut grid = vec![vec![Cell::blank(); width]; height];
        let mut rng = StdRng::seed_from_u64(42);
        let values: Vec<_> = PARAMS
            .iter()
            .map(|p| if max { p.max } else { p.default })
            .collect();
        MODE.render(&mut ModeFrame {
            grid: &mut grid,
            width,
            height,
            seed: 42,
            palette: &[Color::White; 5],
            rng: &mut rng,
            time,
            args: &[],
            param_values: Some(&values),
        });
        grid
    }
    #[test]
    fn terminal_stress_snapshots_and_extrema() {
        let text = |grid: &crate::types::Grid| {
            grid.iter()
                .map(|r| r.iter().map(|c| c.ch).collect::<String>())
                .collect::<Vec<_>>()
                .join("\n")
        };
        let first = render(48, 16, 0.0, false);
        let moving = render(48, 16, 0.35, false);
        insta::assert_snapshot!("terminal_stress_seed42", text(&first));
        insta::assert_snapshot!("terminal_stress_seed42_moving", text(&moving));
        assert_ne!(first, moving);
        assert_eq!(moving, render(48, 16, 0.35, false));
        for (w, h) in [(0, 0), (1, 1), (2, 3), (400, 200)] {
            let grid = render(w, h, 0.35, true);
            assert_eq!(grid.len(), h);
            assert!(grid.iter().all(|r| r.len() == w));
            assert!(
                grid.iter()
                    .flatten()
                    .all(|c| crate::types::char_width(c.ch) == 1)
            );
        }
    }
}
