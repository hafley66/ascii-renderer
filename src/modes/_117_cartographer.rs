use crossterm::style::Color;

use crate::_0_profile::measure_layer;
use crate::opts::param_f32;
use crate::registry::{AnimKind, Mode, ModeFrame, Param};
use crate::types::{Cell, Grid};

pub(super) struct Cartographer;
pub(super) static MODE: Cartographer = Cartographer;

const NAME: &str = "cartographer";
const CYCLE: f32 = 24.0;
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

fn write(grid: &mut Grid, x: i32, y: i32, text: &str, fg: Color) {
    for (i, ch) in text.chars().enumerate() {
        ink(grid, x + i as i32, y, ch, fg);
    }
}

fn stroke(grid: &mut Grid, a: (i32, i32), b: (i32, i32), ch: char, fg: Color) {
    let (mut x, mut y) = a;
    let dx = (b.0 - x).abs();
    let dy = -(b.1 - y).abs();
    let sx = if x < b.0 { 1 } else { -1 };
    let sy = if y < b.1 { 1 } else { -1 };
    let mut error = dx + dy;
    loop {
        ink(grid, x, y, ch, fg);
        if (x, y) == b { break; }
        let twice = 2 * error;
        if twice >= dy { error += dy; x += sx; }
        if twice <= dx { error += dx; y += sy; }
    }
}

fn unit(seed: u64) -> f32 {
    (hash(seed) >> 40) as f32 / (1u32 << 24) as f32
}

fn travel(a: (i32, i32), b: (i32, i32), t: f32) -> (i32, i32) {
    let t = t.clamp(0.0, 1.0);
    ((a.0 as f32 + (b.0 - a.0) as f32 * t).round() as i32,
     (a.1 as f32 + (b.1 - a.1) as f32 * t).round() as i32)
}

fn coast_radius(angle: f32, seed: u64, rugged: f32, detail: usize) -> f32 {
    let turn = (angle / std::f32::consts::TAU).rem_euclid(1.0);
    let mut radius = 0.88;
    for octave in 0..=detail.min(3) {
        let count = 8usize << octave;
        let position = turn * count as f32;
        let i = position.floor() as usize;
        let t = position - i as f32;
        let t = t * t * (3.0 - 2.0 * t);
        let key = seed ^ ((octave as u64 + 1) << 48);
        let a = unit(key ^ i as u64);
        let b = unit(key ^ ((i + 1) % count) as u64);
        radius += ((a + (b - a) * t) - 0.5) * [0.38, 0.22, 0.12, 0.07][octave] * rugged;
    }
    radius.clamp(0.53, 1.12)
}

struct Island {
    w: usize,
    h: usize,
    cx: f32,
    cy: f32,
    rx: f32,
    ry: f32,
    land: Vec<bool>,
    height: Vec<f32>,
}

impl Island {
    fn new(w: usize, h: usize, seed: u64, rugged: f32) -> Self {
        let (cx, cy) = (w as f32 * 0.45, h as f32 * 0.51);
        let (rx, ry) = (w as f32 * 0.29, h as f32 * 0.39);
        let mut land = vec![false; w.saturating_mul(h)];
        let mut height = vec![0.0; w.saturating_mul(h)];
        for y in 0..h {
            for x in 0..w {
                let u = (x as f32 - cx) / rx.max(1.0);
                let v = (y as f32 - cy) / ry.max(1.0);
                let radius = (u * u + v * v).sqrt();
                let edge = coast_radius(v.atan2(u), seed, rugged, 3);
                let inside = radius < edge;
                let idx = y * w + x;
                land[idx] = inside;
                if inside {
                    let ridge = ((u * 3.7 + v * 2.1).sin() * 0.06)
                        + ((u * 6.1 - v * 3.3).cos() * 0.03);
                    let grit = (unit(seed ^ (x as u64).wrapping_mul(971) ^ (y as u64).wrapping_mul(2861)) - 0.5) * 0.045;
                    height[idx] = (edge - radius) * 2.0 + ridge + grit;
                }
            }
        }
        Self { w, h, cx, cy, rx, ry, land, height }
    }

    fn point(&self, angle: f32, seed: u64, rugged: f32, detail: usize) -> (i32, i32) {
        let r = coast_radius(angle, seed, rugged, detail);
        ((self.cx + self.rx * r * angle.cos()).round() as i32,
         (self.cy + self.ry * r * angle.sin()).round() as i32)
    }

    fn index(&self, x: i32, y: i32) -> Option<usize> {
        if x >= 0 && y >= 0 && (x as usize) < self.w && (y as usize) < self.h {
            Some(y as usize * self.w + x as usize)
        } else { None }
    }
}

fn river_path(island: &Island, seed: u64, river: usize) -> Vec<(i32, i32)> {
    let angle = std::f32::consts::TAU * unit(seed ^ (river as u64).wrapping_mul(0x41c6_4e6d));
    let mut x = (island.cx + island.rx * 0.12 * angle.cos()).round() as i32;
    let mut y = (island.cy + island.ry * 0.12 * angle.sin()).round() as i32;
    let mut path = vec![(x, y)];
    let limit = (island.w + island.h).min(512);
    for _ in 0..limit {
        let Some(current) = island.index(x, y) else { break; };
        let current_height = island.height[current];
        let mut best: Option<(i32, i32, f32)> = None;
        for (dx, dy) in [(1,0),(-1,0),(0,1),(0,-1),(1,1),(1,-1),(-1,1),(-1,-1)] {
            let (nx, ny) = (x + dx, y + dy);
            let Some(next) = island.index(nx, ny) else { continue; };
            let next_height = if island.land[next] { island.height[next] } else { -100.0 };
            if next_height >= current_height { continue; }
            let drift = ((nx as f32 - island.cx) * angle.cos() / island.rx.max(1.0)
                + (ny as f32 - island.cy) * angle.sin() / island.ry.max(1.0)) * 0.035;
            let score = next_height - drift;
            if best.map_or(true, |(_, _, old)| score < old) { best = Some((nx, ny, score)); }
        }
        let Some((nx, ny, _)) = best else { break; };
        x = nx;
        y = ny;
        path.push((x, y));
        if let Some(next) = island.index(x, y) {
            if !island.land[next] { break; }
        }
    }
    path
}

fn script(seed: u64, word: usize) -> String {
    let stems = ['|', '/', '\\', 'v', '^', 'Y', 'T', 'r'];
    let marks = ['.', ':', '\'', '~', '-', '*', '`', ','];
    let stem_set = std::array::from_fn::<char, 4, _>(|i| {
        stems[(hash(seed ^ 0x731a ^ i as u64) as usize) % stems.len()]
    });
    let mark_set = std::array::from_fn::<char, 3, _>(|i| {
        marks[(hash(seed ^ 0xf204 ^ i as u64) as usize) % marks.len()]
    });
    let mut result = String::new();
    for i in 0..4 {
        let syllable = hash(seed ^ ((word as u64) << 16) ^ i as u64);
        result.push(stem_set[(syllable as usize) % stem_set.len()]);
        result.push(mark_set[((syllable >> 8) as usize) % mark_set.len()]);
    }
    result
}

fn hachure_anchor(island: &Island, seed: u64, index: usize, count: usize) -> (i32, i32) {
    let row = index / 4;
    let column = if row % 2 == 0 { index % 4 } else { 3 - index % 4 };
    let rows = ((count + 3) / 4).max(2);
    let jitter = unit(seed ^ (index as u64).wrapping_mul(0x731b)) - 0.5;
    let x = island.w as f32 * (0.31 + column as f32 * 0.075) + jitter * 1.2;
    let y = island.h as f32 * (0.28 + row as f32 * 0.43 / (rows - 1) as f32);
    (x.round() as i32, y.round() as i32)
}

fn draw(frame: &mut ModeFrame<'_>, knobs: Knobs) {
    let w = frame.width;
    let h = frame.height;
    if w == 0 || h == 0 { return; }
    let cycle = (frame.time.max(0.0) * knobs.pace / CYCLE).floor() as u64;
    let seed = hash(frame.seed ^ cycle.wrapping_mul(0x56a9_f773));
    let age = (frame.time.max(0.0) * knobs.pace).rem_euclid(CYCLE);
    let sepia = Color::Rgb { r: 103, g: 70, b: 42 };
    let dark = Color::Rgb { r: 72, g: 49, b: 32 };
    let blue = Color::Rgb { r: 64, g: 91, b: 101 };
    measure_layer(NAME, "paper", || {
        for y in 0..h.min(frame.grid.len()) {
            for x in 0..w.min(frame.grid[y].len()) {
                let grain = hash(seed ^ (x as u64).wrapping_mul(811) ^ (y as u64).wrapping_mul(10007)) % 9;
                let yellow = ((age - 18.0) / 4.0).clamp(0.0, 1.0) * 22.0 * knobs.patina;
                let tint = 216u8.saturating_add(grain as u8);
                frame.grid[y][x] = Cell::with_bg(' ', dark, Color::Rgb {
                    r: tint, g: tint.saturating_sub(11 + yellow as u8),
                    b: tint.saturating_sub(32 + (yellow * 1.6) as u8),
                });
            }
        }
    });
    let island = measure_layer(NAME, "terrain", || Island::new(w, h, seed, knobs.rugged));
    measure_layer(NAME, "wash", || {
        for y in 2..h.saturating_sub(2).min(frame.grid.len()) {
            for x in 2..w.saturating_sub(2).min(frame.grid[y].len()) {
                let idx = y * w + x;
                if island.land[idx] {
                    let cell = &mut frame.grid[y][x];
                    cell.bg = Color::Rgb { r: 202, g: 183, b: 143 };
                    let grain = hash(seed ^ (idx as u64).wrapping_mul(0xa273));
                    if grain % 13 == 0 { cell.ch = '.'; cell.fg = sepia; }
                } else if hash(seed ^ idx as u64) % 89 == 0 {
                    ink(frame.grid, x as i32, y as i32, '~', blue);
                }
            }
        }
    });
    let coast_pen = measure_layer(NAME, "coast", || {
        let detail = if age < 1.6 { 0 } else if age < 3.2 { 1 } else if age < 4.8 { 2 } else { 3 };
        let count = 192usize;
        let t = (age / 7.0).clamp(0.0, 1.0);
        let eased = t * t * (3.0 - 2.0 * t);
        let shown = (eased * count as f32).round() as usize;
        let mut previous = island.point(-std::f32::consts::FRAC_PI_2, seed, knobs.rugged, detail);
        for i in 1..=shown {
            let angle = -std::f32::consts::FRAC_PI_2 + std::f32::consts::TAU * i as f32 / count as f32;
            let next = island.point(angle, seed, knobs.rugged, detail);
            stroke(frame.grid, previous, next, '#', dark);
            previous = next;
        }
        previous
    });
    let river_pen = measure_layer(NAME, "rivers", || {
        let progress = ((age - 7.0) / 6.0).clamp(0.0, 1.0);
        let mut pen = coast_pen;
        for i in 0..knobs.rivers {
            let path = river_path(&island, seed, i);
            if path.is_empty() { continue; }
            let local = (progress * knobs.rivers as f32 - i as f32).clamp(0.0, 1.0);
            if local <= 0.0 { break; }
            if local < 0.32 {
                pen = travel(pen, path[0], local / 0.32);
                break;
            }
            let shown = (((local - 0.32) / 0.68) * (path.len().saturating_sub(1)) as f32).round() as usize;
            for pair in path.windows(2).take(shown) {
                stroke(frame.grid, pair[0], pair[1], if i == 0 { '=' } else { ':' }, blue);
            }
            pen = path[shown];
            if shown > 0 {
                if shown + 1 == path.len()
                    && island.index(path[shown].0, path[shown].1)
                        .map_or(false, |idx| !island.land[idx]) {
                    let mouth = path[shown];
                    stroke(frame.grid, mouth, (mouth.0 + 2, mouth.1 - 1), '.', blue);
                    stroke(frame.grid, mouth, (mouth.0 + 2, mouth.1 + 1), '.', blue);
                }
            }
            if local < 1.0 { break; }
        }
        pen
    });
    let relief_pen = measure_layer(NAME, "hachures", || {
        let count = ((w / 8).clamp(8, 28) as f32 * knobs.hachures) as usize;
        let progress = ((age - 13.0) / 2.0).clamp(0.0, 1.0);
        let mut pen = river_pen;
        if count == 0 { return pen; }
        let first = hachure_anchor(&island, seed, 0, count);
        if progress < 0.24 { return travel(pen, first, progress / 0.24); }
        let active = ((progress - 0.24) / 0.76 * count as f32).min(count as f32);
        let shown = active.floor() as usize;
        for i in 0..shown {
            let (x, y) = hachure_anchor(&island, seed, i, count);
            let Some(idx) = island.index(x, y) else { continue; };
            if !island.land[idx] || island.height[idx] < 0.28 { continue; }
            let length = if h < 32 { 1 } else { 2 };
            stroke(frame.grid, (x, y), (x + 1, y + length), '/', sepia);
            pen = (x + 1, y + length);
        }
        if shown < count {
            let next = hachure_anchor(&island, seed, shown, count);
            pen = travel(pen, next, active.fract());
        }
        pen
    });
    let script_pen = measure_layer(NAME, "script", || {
        let count = (3.0 * knobs.script).round() as usize;
        let progress = ((age - 15.0) / 2.0).clamp(0.0, 1.0) * count as f32;
        let anchors = [(0.37, 0.36), (0.48, 0.60), (0.30, 0.68), (0.53, 0.43), (0.39, 0.76), (0.50, 0.30)];
        let mut pen = relief_pen;
        for i in 0..count.min(anchors.len()) {
            let x = (w as f32 * anchors[i].0) as i32;
            let y = (h as f32 * anchors[i].1) as i32;
            let name = script(seed, i);
            let local = (progress - i as f32).clamp(0.0, 1.0);
            if local <= 0.0 { break; }
            if local < 0.25 {
                pen = travel(pen, (x, y), local / 0.25);
                break;
            }
            let shown = (((local - 0.25) / 0.75) * name.len() as f32).ceil() as usize;
            for (j, ch) in name.chars().take(shown).enumerate() {
                ink(frame.grid, x + j as i32, y, ch, dark);
            }
            pen = (x + shown.saturating_sub(1) as i32, y);
            if local < 1.0 { break; }
        }
        pen
    });
    measure_layer(NAME, "furniture", || {
        if w < 4 || h < 4 { return; }
        for x in 1..w-1 {
            ink(frame.grid, x as i32, 1, '=', sepia);
            ink(frame.grid, x as i32, h as i32 - 2, '=', sepia);
        }
        for y in 2..h-2 {
            ink(frame.grid, 1, y as i32, '|', sepia);
            ink(frame.grid, w as i32 - 2, y as i32, '|', sepia);
        }
        write(frame.grid, (w as i32 - 25).max(3) / 2, 0, "ATLAS OF ABSENT SHORES", dark);
        let rose = (w as f32 * 0.83) as i32;
        let ry = (h as f32 * 0.31) as i32;
        write(frame.grid, rose - 1, ry - 3, " N ", dark);
        write(frame.grid, rose - 1, ry - 2, " ^ ", dark);
        write(frame.grid, rose - 3, ry - 1, "\\  |  /", dark);
        write(frame.grid, rose - 3, ry, "W--+--E", dark);
        write(frame.grid, rose - 3, ry + 1, "/  |  \\", dark);
        write(frame.grid, rose - 1, ry + 2, " v ", dark);
        write(frame.grid, rose - 1, ry + 3, " S ", dark);
        let sy = h as i32 - 4;
        write(frame.grid, 5, sy, "0====|====|  100 leagues", dark);
        if age > 13.0 {
            let mx = (w as f32 * 0.83) as i32;
            let my = (h as f32 * 0.68) as i32;
            write(frame.grid, mx - 5, my - 1, " /\\_/\\ ", blue);
            write(frame.grid, mx - 5, my, "< o o  >", blue);
            write(frame.grid, mx - 5, my + 1, " \\_v_/~", blue);
            write(frame.grid, mx - 4, my + 2, "~/  \\~", blue);
        }
        if age > 17.0 {
            let x = (w as f32 * 0.68) as i32;
            let y = (h as f32 * 0.82) as i32;
            write(frame.grid, x, y, "MERIDIAN IX", dark);
            stroke(frame.grid, (x, y), (x + 10, y), '-', Color::Rgb { r: 137, g: 61, b: 49 });
            write(frame.grid, x, y + 1, "MERIDIAN XI", dark);
        }
    });
    measure_layer(NAME, "finish", || {
        if age >= 18.0 {
            let cx = w as f32 * 0.18;
            let cy = h as f32 * 0.59;
            let bloom = ((age - 18.0) / 4.0).clamp(0.0, 1.0);
            let rx = w as f32 * 0.14 * bloom;
            let ry = h as f32 * 0.23 * bloom;
            for i in 0..96 {
                let angle = std::f32::consts::TAU * i as f32 / 96.0;
                let x = (cx + rx * angle.cos()) as i32;
                let y = (cy + ry * angle.sin()) as i32;
                if i % 7 != 0 { ink(frame.grid, x, y, '.', Color::Rgb { r: 139, g: 93, b: 54 }); }
            }
        }
        if age >= 22.0 {
            let edge = w.saturating_sub((((age - 22.0) / 2.0) * w as f32) as usize);
            for y in 0..h.min(frame.grid.len()) {
                for x in edge..w.min(frame.grid[y].len()) {
                    frame.grid[y][x] = Cell::with_bg(' ', dark, Color::Rgb { r: 229, g: 217, b: 188 });
                }
            }
        } else if age < 17.0 {
            let pen = if age < 7.0 { coast_pen } else if age < 13.0 { river_pen }
                else if age < 15.0 { relief_pen } else { script_pen };
            ink(frame.grid, pen.0, pen.1, '@', Color::Rgb { r: 255, g: 245, b: 195 });
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    fn render(seed: u64, time: f32, w: usize, h: usize) -> String {
        let mut grid = vec![vec![Cell::blank(); w]; h];
        let palette = crate::color::make_palette(seed);
        let mut rng = StdRng::seed_from_u64(seed);
        MODE.render(&mut ModeFrame {
            grid: &mut grid, width: w, height: h, seed, palette: &palette,
            rng: &mut rng, time, args: &[], param_values: None,
        });
        grid.iter().map(|row| row.iter().map(|cell| cell.ch).collect::<String>())
            .collect::<Vec<_>>().join("\n")
    }

    #[test]
    fn first_ink() {
        insta::assert_snapshot!("cartographer_80x24_t0", render(42, 0.0, 80, 24));
    }

    #[test]
    fn refined_coast() {
        insta::assert_snapshot!("cartographer_80x24_t6", render(42, 6.0, 80, 24));
    }

    #[test]
    fn deterministic_seed_and_time() {
        let first = render(42, 6.0, 80, 24);
        assert_eq!(first, render(42, 6.0, 80, 24));
        assert_ne!(first, render(43, 6.0, 80, 24));
        assert_ne!(first, render(42, 0.0, 80, 24));
    }

    #[cfg(not(debug_assertions))]
    #[test]
    fn release_frame_cost_under_six_ms() {
        use std::time::Instant;
        let _ = render(42, 13.0, 200, 60);
        let started = Instant::now();
        for i in 0..12 {
            let _ = render(42, 13.0 + i as f32 * 0.1, 200, 60);
        }
        let average = started.elapsed().as_secs_f64() / 12.0;
        assert!(average < 0.006, "average frame cost: {average:.6} s");
    }
}
