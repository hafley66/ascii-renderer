use crossterm::style::Color;
use rayon::prelude::*;

use crate::_0_profile::measure_layer;
use crate::opts::param_f32;
use crate::registry::{AnimKind, Mode, ModeFrame, Param};
use crate::types::{Cell, Grid};

pub(super) struct Strata;
pub(super) static MODE: Strata = Strata;

const NAME: &str = "strata";
const HELP: &str = "strata: alien drill core [fold] [fault] [grain] [erosion] [fossils] [speed]";
const PARAMS: &[Param] = &[
    param!("FOLD", "fold", 0.0, 2.0, 1.0, 0.1),
    param!("FAULT", "fault", 0.0, 4.0, 2.0, 0.25),
    param!("GRAIN", "grain", 0.0, 1.0, 0.55, 0.05),
    param!("EROSION", "erosion", 0.0, 3.0, 1.5, 0.25),
    param!("FOSSILS", "fossils", 0.0, 1.0, 0.7, 0.05),
    param!("SPEED", "speed", 0.5, 2.0, 1.0, 0.1),
];

const PAPER: Color = Color::Rgb { r: 221, g: 210, b: 188 };
const DIM: Color = Color::Rgb { r: 117, g: 111, b: 105 };
const VOID: Color = Color::Rgb { r: 17, g: 16, b: 17 };
const IRIDIUM: Color = Color::Rgb { r: 88, g: 239, b: 231 };
const ROCK: [Color; 6] = [
    Color::Rgb { r: 103, g: 88, b: 91 },
    Color::Rgb { r: 134, g: 111, b: 101 },
    Color::Rgb { r: 167, g: 135, b: 103 },
    Color::Rgb { r: 176, g: 160, b: 124 },
    Color::Rgb { r: 151, g: 162, b: 137 },
    Color::Rgb { r: 196, g: 181, b: 148 },
];
const ERAS: [&str; 6] = ["AZH", "VOR", "THREN", "CALYX", "OMBR", "LUMEN"];

struct Plate {
    width: usize,
    height: usize,
    right: usize,
    bottom: usize,
    seed: u64,
    age: f32,
    fold: f32,
    fault: f32,
    grain: f32,
    erosion: f32,
    fossils: f32,
}

fn hash(seed: u64, x: usize, y: usize) -> f32 {
    let mut z = seed ^ (x as u64).wrapping_mul(0x9e3779b97f4a7c15)
        ^ (y as u64).wrapping_mul(0xbf58476d1ce4e5b9);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
    ((z ^ (z >> 31)) >> 40) as f32 / 16_777_215.0
}

fn put(grid: &mut Grid, x: usize, y: usize, ch: char, color: Color) {
    if let Some(cell) = grid.get_mut(y).and_then(|row| row.get_mut(x)) {
        *cell = Cell::new(ch, color);
    }
}

fn write(grid: &mut Grid, x: usize, y: usize, text: &str, color: Color) {
    for (offset, ch) in text.chars().enumerate() {
        put(grid, x + offset, y, ch, color);
    }
}

impl Plate {
    fn span(&self) -> f32 { self.bottom.saturating_sub(2).max(1) as f32 }

    fn fill(&self) -> f32 { 0.54 + 0.46 * self.age }

    fn fault_x(&self, y: usize) -> usize {
        let drift = self.bottom.saturating_sub(y) as f32 * 0.12;
        (self.right as f32 * 0.52 + drift).round() as usize
    }

    fn warped_depth(&self, x: usize, y: usize) -> f32 {
        let xf = x as f32;
        let seed_phase = (self.seed % 997) as f32 * 0.017;
        let fold = ((xf * 0.115 + seed_phase).sin() * 0.78
            + (xf * 0.052 - seed_phase).sin() * 0.55)
            * self.fold * (0.18 + self.age * 1.35);
        let slip = if self.age > 0.36 && x > self.fault_x(y) && y < self.bottom {
            self.fault * ((self.age - 0.36) / 0.64).min(1.0)
        } else { 0.0 };
        self.bottom as f32 - y as f32 - fold - slip
    }

    fn era(&self, x: usize, y: usize) -> Option<usize> {
        let depth = self.warped_depth(x, y);
        if depth < 0.0 || depth > self.span() * self.fill() { return None; }
        Some(((depth / self.span() * 6.0).floor() as usize).min(5))
    }

    fn paint_row(&self, row: &mut [Cell], y: usize) {
        if y < 2 || y > self.bottom { return; }
        for x in 2..=self.right.min(row.len().saturating_sub(1)) {
            let Some(era) = self.era(x, y) else { continue; };
            let depth = self.warped_depth(x, y);
            let near_surface = self.span() * self.fill() - depth < 0.7;
            let n = hash(self.seed ^ ((era as u64) << 32), x, y);
            let ch = if near_surface { '~' } else {
                match era {
                    0 => if n < self.grain * 0.34 { '@' } else { '#' },
                    1 => if n < self.grain * 0.45 { ';' } else { ':' },
                    2 => if n < self.grain * 0.4 { '-' } else { '=' },
                    3 => if n < self.grain * 0.3 { 'O' } else { 'o' },
                    4 => if n < self.grain * 0.27 { '\\' } else { '/' },
                    _ => if n < self.grain * 0.36 { '`' } else { '.' },
                }
            };
            row[x] = Cell::new(ch, ROCK[era]);
        }
    }
}

fn paint_plate(grid: &mut Grid, p: &Plate) {
    for row in grid.iter_mut() {
        for cell in row.iter_mut() { *cell = Cell::new(' ', VOID); }
    }
    write(grid, 2, 0, "VESPER-9  /  CORE 116", PAPER);
    write(grid, 2, 1, "DEEP TIME : FOUR BILLION YEARS", DIM);
    for y in 2..=p.bottom {
        put(grid, 1, y, '|', DIM);
        put(grid, p.right + 1, y, '|', DIM);
    }
    for x in 1..=p.right + 1 {
        put(grid, x, p.bottom + 1, '-', DIM);
    }
    put(grid, 1, p.bottom + 1, '+', DIM);
    put(grid, p.right + 1, p.bottom + 1, '+', DIM);
}

fn paint_events(grid: &mut Grid, p: &Plate) {
    if p.age > 0.62 {
        for x in 2..=p.right {
            let cut = (x as f32 * 0.095 + p.seed as f32 * 0.002).sin() * p.erosion;
            let y = (p.bottom as f32 - p.span() * 0.62 + cut).round() as usize;
            if p.era(x, y).is_some() { put(grid, x, y, '~', PAPER); }
        }
    }
    if p.age > 0.36 {
        for y in 2..p.bottom {
            let x = p.fault_x(y);
            if x <= p.right && p.era(x, y).is_some() {
                put(grid, x, y, '/', VOID);
            }
        }
    }
    if p.age > 0.78 {
        for x in 2..=p.right {
            let y = (p.bottom as f32 - p.span() * 0.77
                + (x as f32 * 0.115).sin() * p.fold * 0.5).round() as usize;
            if p.era(x, y).is_some() { put(grid, x, y, '=', IRIDIUM); }
        }
    }
}

fn paint_fossils(grid: &mut Grid, p: &Plate) {
    if p.fossils < 0.05 { return; }
    let specimens: [(usize, f32, &str); 9] = [
        (1, 0.28, "<:>"),
        (2, 0.67, "/|\\"),
        (3, 0.19, "(o)"), (3, 0.75, "(o)"),
        (4, 0.35, "><((o>"), (4, 0.81, "><((o>"),
        (5, 0.17, "\\|/"), (5, 0.55, "\\|/"), (5, 0.83, "\\|/"),
    ];
    for (era, position, fossil) in specimens {
        if hash(p.seed ^ 0x5f17, era, (position * 100.0) as usize) > p.fossils { continue; }
        let x = 2 + (p.right.saturating_sub(4) as f32 * position).round() as usize;
        let target = (era as f32 + 0.5) / 6.0;
        let y = (p.bottom as f32 - p.span() * target).round() as usize;
        if y > p.bottom || x + fossil.len() > p.right { continue; }
        if p.era(x, y) != Some(era) { continue; }
        write(grid, x, y, fossil, PAPER);
    }
}

fn paint_gutter(grid: &mut Grid, p: &Plate) {
    let x = p.right + 3;
    let current = ((p.age * 6.0).floor() as usize).min(5);
    write(grid, x, 1, "ERA / Ga", DIM);
    for era in (0..6).rev() {
        let y = (p.bottom as f32 - p.span() * (era as f32 + 0.5) / 6.0).round() as usize;
        if y < 2 || y > p.bottom { continue; }
        let marker = if era == current { '>' } else { ':' };
        let color = if era == current { PAPER } else { DIM };
        put(grid, x, y, marker, color);
        write(grid, x + 2, y, ERAS[era], color);
        if p.width >= 80 {
            let ga = (5 - era) as f32 * (4.0 / 6.0);
            write(grid, x + 8, y, &format!("{ga:.1}"), color);
        }
    }
    let elapsed = format!("T+{:04.1} Ga", p.age * 4.0);
    write(grid, x, p.bottom + 1, &elapsed, PAPER);
    if p.age > 0.78 {
        write(grid, x, p.bottom + 2, "* IRIDIUM", IRIDIUM);
    }
}

impl Mode for Strata {
    fn name(&self) -> &'static str { NAME }
    fn help(&self) -> &'static str { HELP }
    fn animation(&self) -> AnimKind { AnimKind::Iterate }
    fn params(&self) -> &'static [Param] { PARAMS }

    fn render(&self, frame: &mut ModeFrame<'_>) {
        let mut values = [0.0; 6];
        for (i, param) in PARAMS.iter().enumerate() {
            values[i] = frame.args.get(i + 4).and_then(|s| s.parse::<f32>().ok())
                .or_else(|| frame.param_values.and_then(|v| v.get(i).copied()))
                .unwrap_or_else(|| param_f32(param.key, param.default))
                .clamp(param.min, param.max);
        }
        let gutter = if frame.width >= 68 { 14 } else { (frame.width / 5).max(8) };
        if frame.width <= gutter + 5 || frame.height < 7 { return; }
        let p = Plate {
            width: frame.width, height: frame.height,
            right: frame.width - gutter - 3,
            bottom: frame.height - 3,
            seed: frame.seed,
            age: (frame.time * values[5]).rem_euclid(60.0) / 60.0,
            fold: values[0], fault: values[1], grain: values[2],
            erosion: values[3], fossils: values[4],
        };
        let grid = &mut *frame.grid;
        measure_layer(NAME, "plate", || paint_plate(grid, &p));
        measure_layer(NAME, "sediment", || {
            if p.width * p.height >= 20_480 {
                grid.par_iter_mut().enumerate().for_each(|(y, row)| p.paint_row(row, y));
            } else {
                for (y, row) in grid.iter_mut().enumerate() { p.paint_row(row, y); }
            }
        });
        measure_layer(NAME, "events", || paint_events(grid, &p));
        measure_layer(NAME, "fossils", || paint_fossils(grid, &p));
        measure_layer(NAME, "gutter", || paint_gutter(grid, &p));
    }
}
