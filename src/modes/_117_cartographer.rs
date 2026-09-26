use crossterm::style::Color;

use crate::_0_profile::measure_layer;
use crate::opts::param_f32;
use crate::registry::{AnimKind, Mode, ModeFrame, Param};
use crate::types::{Cell, Grid};

pub(super) struct Cartographer;
pub(super) static MODE: Cartographer = Cartographer;

const NAME: &str = "cartographer";
const HELP: &str = "cartographer: an absent island inks itself [pace] [rugged] [rivers] [hachures] [script] [patina]";
const PARAMS: &[Param] = &[
    param!("PACE", "drawing pace", 0.5, 2.0, 1.0, 0.1),
    param!("RUGGED", "coast roughness", 0.2, 1.8, 1.0, 0.1),
    param!("RIVERS", "river count", 1.0, 8.0, 4.0, 1.0),
    param!("HACHURES", "mountain strokes", 0.0, 2.0, 1.0, 0.1),
    param!("SCRIPT", "place names", 0.0, 2.0, 1.0, 0.1),
    param!("PATINA", "paper age", 0.0, 2.0, 1.0, 0.1),
];

#[derive(Clone, Copy)]
struct Knobs {
    pace: f32,
    rugged: f32,
    rivers: usize,
    hachures: f32,
    script: f32,
    patina: f32,
}

impl Mode for Cartographer {
    fn name(&self) -> &'static str { NAME }
    fn help(&self) -> &'static str { HELP }
    fn animation(&self) -> AnimKind { AnimKind::Iterate }
    fn params(&self) -> &'static [Param] { PARAMS }

    fn render(&self, frame: &mut ModeFrame<'_>) {
        let mut values = [0.0_f32; 6];
        for (i, param) in PARAMS.iter().enumerate() {
            values[i] = frame.args.get(i + 4).and_then(|s| s.parse::<f32>().ok())
                .or_else(|| frame.param_values.and_then(|v| v.get(i).copied()))
                .unwrap_or_else(|| param_f32(param.key, param.default))
                .clamp(param.min, param.max);
        }
        let knobs = Knobs {
            pace: values[0], rugged: values[1], rivers: values[2].round() as usize,
            hachures: values[3], script: values[4], patina: values[5],
        };
        draw(frame, knobs);
    }
}

fn hash(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^ (x >> 31)
}

fn ink(grid: &mut Grid, x: i32, y: i32, ch: char, fg: Color) {
    if x >= 0 && y >= 0 {
        if let Some(cell) = grid.get_mut(y as usize).and_then(|row| row.get_mut(x as usize)) {
            cell.ch = ch;
            cell.fg = fg;
        }
    }
}

fn draw(frame: &mut ModeFrame<'_>, knobs: Knobs) {
    let w = frame.width;
    let h = frame.height;
    let seed = frame.seed;
    let age = (frame.time.max(0.0) * knobs.pace).rem_euclid(16.0);
    measure_layer(NAME, "paper", || {
        for y in 0..h.min(frame.grid.len()) {
            for x in 0..w.min(frame.grid[y].len()) {
                let grain = hash(seed ^ (x as u64).wrapping_mul(811) ^ (y as u64).wrapping_mul(10007)) % 9;
                let tint = 216 + grain as u8;
                frame.grid[y][x] = Cell::with_bg(' ', Color::Rgb { r: 74, g: 50, b: 32 }, Color::Rgb { r: tint, g: tint.saturating_sub(11), b: tint.saturating_sub(32) });
            }
        }
    });
    measure_layer(NAME, "border", || {
        if w < 4 || h < 4 { return; }
        let brown = Color::Rgb { r: 104, g: 71, b: 43 };
        for x in 1..w-1 {
            ink(frame.grid, x as i32, 1, '=', brown);
            ink(frame.grid, x as i32, h as i32 - 2, '=', brown);
        }
        for y in 2..h-2 {
            ink(frame.grid, 1, y as i32, '|', brown);
            ink(frame.grid, w as i32 - 2, y as i32, '|', brown);
        }
    });
    let _ = age;
    let _ = knobs;
}
