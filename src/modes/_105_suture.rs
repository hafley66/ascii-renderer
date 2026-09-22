use crate::_0_profile::measure_layer;
use crate::color::{darken, lerp_color};
use crate::opts::param_f32;
use crate::registry::{AnimKind, Mode, ModeFrame, Param};
use crate::types::{Cell, Grid};
use crossterm::style::Color;

pub(super) struct SutureMode;
pub(super) static MODE: SutureMode = SutureMode;

const PARAMS: &[Param] = &[
    param!("GAP", "fissure width", 2.0, 20.0, 8.0, 1.0),
    param!("BEND", "fissure curvature", 0.0, 1.0, 0.55, 0.05),
    param!("STITCHES", "crossing stitches", 2.0, 12.0, 6.0, 1.0),
    param!("DENSITY", "plate mark density", 0.1, 1.0, 0.7, 0.05),
    param!("SPEED", "plate drift", 0.0, 2.0, 0.4, 0.05),
];

impl Mode for SutureMode {
    fn name(&self) -> &'static str {
        "suture"
    }

    fn help(&self) -> &'static str {
        "Two marked plates held across a shifting fissure [gap] [bend] [stitches] [density] [speed]"
    }

    fn animation(&self) -> AnimKind {
        AnimKind::Iterate
    }

    fn params(&self) -> &'static [Param] {
        PARAMS
    }

    fn render(&self, frame: &mut ModeFrame<'_>) {
        // Resolve positional, native, then persisted or environment controls.
        // Derive a finite time and deterministic seed phase for this frame.
        // Fill the two plates and the empty fissure in one pass.
        // Draw stitch anchors and crossings on selected rows.
        let k: [f32; 5] = std::array::from_fn(|i| {
            let p = &PARAMS[i];
            let value = frame
                .args
                .get(i + 4)
                .and_then(|v| v.parse::<f32>().ok())
                .or_else(|| frame.param_values.and_then(|v| v.get(i)).copied())
                .unwrap_or_else(|| param_f32(p.key, p.default));
            if value.is_finite() {
                value.clamp(p.min, p.max)
            } else {
                p.default
            }
        });
        draw(frame, k);
    }
}

fn seam(y: f32, width: f32, bend: f32, phase: f32, time: f32) -> f32 {
    width
        * (0.51
            + bend
                * (0.07 * (y * 0.31 + phase + time * 0.38).sin()
                    + 0.025 * (y * 0.13 - phase * 0.7 - time * 0.17).sin()))
}

fn mark(grid: &mut Grid, x: isize, y: isize, ch: char, fg: Color, bg: Color) {
    if x >= 0 && y >= 0 {
        if let Some(cell) = grid
            .get_mut(y as usize)
            .and_then(|row| row.get_mut(x as usize))
        {
            *cell = Cell::with_bg(ch, fg, bg);
        }
    }
}

fn draw(frame: &mut ModeFrame<'_>, k: [f32; 5]) {
    let (w, h) = (frame.width, frame.height);
    if w == 0 || h == 0 {
        return;
    }
    let [gap, bend, stitches, density, speed] = k;
    let gap = gap.min((w as f32 * 0.48).max(1.0));
    let time = if frame.time.is_finite() {
        ((frame.time as f64) * speed as f64).clamp(-1_000_000.0, 1_000_000.0) as f32
    } else {
        0.0
    };
    let phase = (frame.seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 40) as f32 / 16_777_216.0
        * std::f32::consts::TAU;
    let drift = (time * 1.8).floor() as i64;
    let offset = ((frame.seed >> 8) % 113) as i64;
    let ink = darken(frame.palette[0], 12);
    let left_bg = lerp_color(ink, frame.palette[1], 0.11);
    let right_bg = lerp_color(ink, frame.palette[2], 0.10);
    let left_fg = lerp_color(frame.palette[1], frame.palette[4], 0.15);
    let right_fg = lerp_color(frame.palette[2], frame.palette[4], 0.18);
    let edge_fg = frame.palette[3];
    let thread = frame.palette[4];

    measure_layer("suture", "plates", || {
        for (y, row) in frame.grid.iter_mut().enumerate() {
            let center = seam(y as f32, w as f32, bend, phase, time);
            let left = center - gap * 0.5;
            let right = center + gap * 0.5;
            let left_stripe = y as i64 * 2 + drift + offset;
            let right_stripe = y as i64 * 3 - drift + offset;
            for (x, cell) in row.iter_mut().enumerate() {
                let xf = x as f32;
                let grain = (x as u64)
                    .wrapping_mul(0xD1B5_4A32_D192_ED03)
                    .wrapping_add((y as u64).wrapping_mul(0x94D0_49BB_1331_11EB))
                    ^ frame.seed;
                let grain = ((grain ^ (grain >> 31)) % 100) as f32;
                *cell = if xf < left - 1.0 {
                    let stripe = (x as i64 + left_stripe).rem_euclid(9);
                    let ch = if stripe < (density * 3.6).ceil() as i64 {
                        '/'
                    } else if grain < density * 34.0 {
                        '.'
                    } else {
                        ' '
                    };
                    Cell::with_bg(ch, left_fg, left_bg)
                } else if xf <= left {
                    Cell::with_bg(if y % 7 == 0 { '+' } else { '|' }, edge_fg, ink)
                } else if xf < right {
                    Cell::with_bg(' ', thread, ink)
                } else if xf <= right + 1.0 {
                    Cell::with_bg(if y % 7 == 0 { '+' } else { '|' }, edge_fg, ink)
                } else {
                    let stripe = (x as i64 / 5 + right_stripe).rem_euclid(11);
                    let ch = if stripe < (density * 3.8).ceil() as i64 {
                        '='
                    } else if grain < density * 28.0 {
                        '.'
                    } else {
                        ' '
                    };
                    Cell::with_bg(ch, right_fg, right_bg)
                };
            }
        }
    });

    measure_layer("suture", "stitches", || {
        let count = stitches.round() as usize;
        for i in 0..count {
            let y = ((i + 1) * h / (count + 1)) as isize;
            let center = seam(y as f32, w as f32, bend, phase, time);
            let x0 = (center - gap * 0.5).round() as isize;
            let x1 = (center + gap * 0.5).round() as isize;
            let span = (x1 - x0).max(1);
            for j in 0..=span {
                let x = x0 + j;
                let ch = if j == 0 || j == span {
                    'o'
                } else if j == span / 2 {
                    'X'
                } else {
                    '-'
                };
                mark(frame.grid, x, y, ch, thread, ink);
            }
            mark(frame.grid, x0, y, 'o', edge_fg, ink);
            mark(frame.grid, x1, y, 'o', edge_fg, ink);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{morph::IterateFrameRenderer, render::grid_to_plain};

    #[test]
    fn suture_fixed_seed_and_motion() {
        let controls: Vec<_> = PARAMS.iter().map(|p| p.default).collect();
        let mut renderer = IterateFrameRenderer::new("suture", 57, "ember", 80, 24).unwrap();
        let first = renderer.render(0.0, Some(&controls)).unwrap().clone();
        let later = renderer.render(6.0, Some(&controls)).unwrap().clone();
        insta::assert_snapshot!("suture_seed57", grid_to_plain(&first).join("\n"));
        insta::assert_snapshot!("suture_seed57_t6", grid_to_plain(&later).join("\n"));
        assert_ne!(first, later);
        assert_eq!(first, *renderer.render(0.0, Some(&controls)).unwrap());
        let mut stopped = controls.clone();
        stopped[4] = 0.0;
        assert_eq!(first, *renderer.render(6.0, Some(&stopped)).unwrap());
        let maximum: Vec<_> = PARAMS.iter().map(|p| p.max).collect();
        assert_ne!(later, *renderer.render(6.0, Some(&maximum)).unwrap());
    }
}
