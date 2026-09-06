//! Astra's Fern Valley preserve. Draft filename is leased by the integrator.
//! Geometry and inline snapshots travel together when the file is renamed.

use crate::opts::param_f32;
use crate::registry::{AnimKind, Mode, ModeFrame, Param};
use crate::types::{Cell, Grid};
use crossterm::style::Color;
use std::f32::consts::TAU;

pub(super) struct JurassicParkMode;
pub(super) static MODE: JurassicParkMode = JurassicParkMode;

const PARAMS: &[Param] = &[
    param!("HERD", "herd density", 0.0, 1.0, 1.0, 0.1),
    param!("SPECIES", "herbivore / predator mix", 0.0, 1.0, 0.5, 0.05),
    param!("SCALE", "dinosaur scale", 0.55, 1.35, 1.0, 0.05),
    param!("FERNS", "vegetation density", 0.0, 1.0, 0.55, 0.05),
    param!("LIGHT", "daylight", 0.0, 1.0, 0.75, 0.05),
    param!("SPEED", "motion speed", 0.0, 3.0, 1.0, 0.1),
    param!("DEPTH", "scene depth", 0.0, 1.0, 0.65, 0.05),
    param!("VARIATION", "seed variation", 0.0, 1.0, 0.5, 0.05),
    param!("RAIN", "rainfall", 0.0, 1.0, 0.15, 0.05),
];

impl Mode for JurassicParkMode {
    fn name(&self) -> &'static str {
        "astra-jurassic-park"
    }

    fn help(&self) -> &'static str {
        "Fern Valley: procedural longnecks, ceratopsians, raptors and tyrannosaurs. [herd] [species] [scale] [ferns] [light] [speed] [depth] [variation] [rain]"
    }

    fn animation(&self) -> AnimKind {
        AnimKind::Iterate
    }

    fn params(&self) -> &'static [Param] {
        PARAMS
    }

    fn render(&self, frame: &mut ModeFrame<'_>) {
        // Resolve borrowed frame inputs; no environment writes or retained state.
        let knobs = std::array::from_fn(|i| {
            let p = &PARAMS[i];
            let value = frame
                .param_values
                .and_then(|values| values.get(i).copied())
                .or_else(|| frame.args.get(i + 4).and_then(|arg| arg.parse().ok()))
                .unwrap_or_else(|| param_f32(p.key, p.default));
            if value.is_finite() {
                value.clamp(p.min, p.max)
            } else {
                p.default
            }
        });
        draw_preserve(frame, &knobs);
    }
}

fn hash(mut n: u64) -> u64 {
    n = (n ^ (n >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    n = (n ^ (n >> 27)).wrapping_mul(0x94d049bb133111eb);
    n ^ (n >> 31)
}

fn random(seed: u64, lane: u64) -> f32 {
    (hash(seed.wrapping_add(lane.wrapping_mul(0x9e3779b97f4a7c15))) >> 40) as f32 / 16_777_216.0
}

fn rgb(color: Color) -> [u8; 3] {
    match color {
        Color::Rgb { r, g, b } => [r, g, b],
        Color::Black => [0, 0, 0],
        Color::DarkGrey => [80, 80, 80],
        Color::Grey => [180, 180, 180],
        Color::Red | Color::DarkRed => [210, 90, 90],
        Color::Green | Color::DarkGreen => [90, 190, 110],
        Color::Blue | Color::DarkBlue => [90, 120, 210],
        Color::Yellow | Color::DarkYellow => [220, 190, 90],
        Color::Magenta | Color::DarkMagenta => [190, 100, 200],
        Color::Cyan | Color::DarkCyan => [80, 190, 200],
        Color::AnsiValue(n) if n >= 232 => [8 + (n - 232) * 10; 3],
        Color::AnsiValue(n) if n >= 16 => {
            let n = n - 16;
            let channel = |v| if v == 0 { 0 } else { 55 + v * 40 };
            [channel(n / 36), channel(n / 6 % 6), channel(n % 6)]
        }
        _ => [225, 225, 210],
    }
}

fn tint(a: Color, b: Color, amount: f32) -> Color {
    let a = rgb(a);
    let b = rgb(b);
    let c: [u8; 3] = std::array::from_fn(|i| {
        (a[i] as f32 + (b[i] as f32 - a[i] as f32) * amount.clamp(0.0, 1.0)) as u8
    });
    Color::Rgb {
        r: c[0],
        g: c[1],
        b: c[2],
    }
}

type Point = [f32; 2];

struct Canvas<'a> {
    grid: &'a mut Grid,
    width: usize,
    height: usize,
}

impl Canvas<'_> {
    fn put(&mut self, p: Point, ch: char, color: Color) {
        let x = p[0].round() as i32;
        let y = p[1].round() as i32;
        if x >= 0 && y >= 0 && (x as usize) < self.width && (y as usize) < self.height {
            let cell = &mut self.grid[y as usize][x as usize];
            cell.ch = ch;
            cell.fg = color;
        }
    }

    fn line(&mut self, a: Point, b: Point, ch: char, color: Color) {
        let dx = b[0] - a[0];
        let dy = b[1] - a[1];
        let steps =
            (dx.abs().max(dy.abs()).ceil() as usize).clamp(1, (self.width + self.height).max(1));
        let ch = if ch != '\0' {
            ch
        } else if dx.abs() > dy.abs() * 2.0 {
            '_'
        } else if dy.abs() > dx.abs() * 2.0 {
            '|'
        } else if dx * dy > 0.0 {
            '\\'
        } else {
            '/'
        };
        for i in 0..=steps {
            let f = i as f32 / steps as f32;
            self.put([a[0] + dx * f, a[1] + dy * f], ch, color);
        }
    }

    fn ellipse(&mut self, center: Point, radius: Point, color: Color, identity: u64) {
        let rx = radius[0].max(0.55);
        let ry = radius[1].max(0.55);
        let x0 = (center[0] - rx).floor().max(0.0) as usize;
        let x1 = ((center[0] + rx).ceil().max(0.0) as usize + 1).min(self.width);
        let y0 = (center[1] - ry).floor().max(0.0) as usize;
        let y1 = ((center[1] + ry).ceil().max(0.0) as usize + 1).min(self.height);
        for y in y0..y1 {
            for x in x0..x1 {
                let dx = (x as f32 - center[0]) / rx;
                let dy = (y as f32 - center[1]) / ry;
                if dx * dx + dy * dy <= 1.0 {
                    let edge = dx * dx + dy * dy > 0.48
                        || (dx - 1.0 / rx).powi(2) + dy * dy > 1.0
                        || (dx + 1.0 / rx).powi(2) + dy * dy > 1.0;
                    let ch = if edge {
                        if dy < -0.5 {
                            '_'
                        } else if dy > 0.5 {
                            '-'
                        } else if dx < 0.0 {
                            '('
                        } else {
                            ')'
                        }
                    } else if random(identity, (x + y * self.width) as u64) < 0.24 {
                        ':'
                    } else {
                        ' '
                    };
                    self.put([x as f32, y as f32], ch, color);
                }
            }
        }
    }

    fn triangle(&mut self, a: Point, b: Point, c: Point, color: Color) {
        let edge = |p: Point, q: Point, r: Point| {
            (q[0] - p[0]) * (r[1] - p[1]) - (q[1] - p[1]) * (r[0] - p[0])
        };
        let area = edge(a, b, c);
        if area.abs() < 0.01 {
            self.line(a, c, '\0', color);
            return;
        }
        let x0 = a[0].min(b[0]).min(c[0]).floor().max(0.0) as usize;
        let x1 = ((a[0].max(b[0]).max(c[0]).ceil().max(0.0) as usize) + 1).min(self.width);
        let y0 = a[1].min(b[1]).min(c[1]).floor().max(0.0) as usize;
        let y1 = ((a[1].max(b[1]).max(c[1]).ceil().max(0.0) as usize) + 1).min(self.height);
        for y in y0..y1 {
            for x in x0..x1 {
                let p = [x as f32, y as f32];
                if edge(a, b, p) / area >= 0.0
                    && edge(b, c, p) / area >= 0.0
                    && edge(c, a, p) / area >= 0.0
                {
                    self.put(p, ':', color);
                }
            }
        }
        self.line(a, c, '\0', color);
        self.line(b, c, '\0', color);
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Animal {
    identity: u64,
    species: usize,
    facing: f32,
    origin: Point,
    scale: Point,
    phase: f32,
    pose: f32,
    color: Color,
}

impl Animal {
    fn point(&self, x: f32, y: f32) -> Point {
        [
            self.origin[0] + self.facing * x * self.scale[0],
            self.origin[1] + y * self.scale[1],
        ]
    }

    fn oval(&self, canvas: &mut Canvas<'_>, center: Point, radius: Point, color: Color) {
        canvas.ellipse(
            self.point(center[0], center[1]),
            [radius[0] * self.scale[0], radius[1] * self.scale[1]],
            color,
            self.identity,
        );
    }

    fn line(&self, canvas: &mut Canvas<'_>, a: Point, b: Point, ch: char, color: Color) {
        canvas.line(self.point(a[0], a[1]), self.point(b[0], b[1]), ch, color);
    }

    fn triangle(&self, canvas: &mut Canvas<'_>, a: Point, b: Point, c: Point, color: Color) {
        canvas.triangle(
            self.point(a[0], a[1]),
            self.point(b[0], b[1]),
            self.point(c[0], c[1]),
            color,
        );
    }
}

// Positive half-cycle swings above the ground. The other half-cycle plants at y=0.
// Opposite phases guarantee at least one planted foot for bipeds and quadrupeds.
fn foot(hip: f32, phase: f32) -> Point {
    [hip + 0.7 * phase.cos(), -0.65 * phase.sin().max(0.0)]
}

fn draw_dinosaur(canvas: &mut Canvas<'_>, animal: &Animal, time: f32) {
    // Evaluate one stable identity's pose and gait; origin.y is its ground plane.
    // Draw shadow, far limbs, tapered tail, body, neck/crest, head, near limbs, eye.
    let a = animal;
    let phase = a.phase + time * (1.8 + random(a.identity, 18));
    let bob = phase.sin().abs() * 0.16;
    let skin = a.color;
    let shade = tint(skin, Color::Black, 0.30);
    let gleam = tint(skin, Color::White, 0.40);
    let (body_y, body_x, body_h) = match a.species {
        0 => (-2.6 - bob, 3.4, 1.35),
        1 => (-2.4 - bob, 3.5, 1.45),
        2 => (-2.6 - bob, 2.5, 0.95),
        _ => (-3.5 - bob, 3.0, 1.6),
    };
    a.line(canvas, [-3.1, 0.4], [3.5, 0.4], '.', shade);
    let leg_count = if a.species < 2 { 4 } else { 2 };
    for leg in 0..leg_count {
        let hip = if a.species < 2 {
            if leg < 2 { -2.1 } else { 2.0 }
        } else {
            -0.8 + leg as f32 * 1.4
        };
        let f = foot(hip, phase + (leg % 2) as f32 * std::f32::consts::PI);
        let knee = [(hip + f[0]) * 0.5 - 0.45, body_y * 0.35];
        a.line(canvas, [hip, body_y + 0.4], knee, '\0', shade);
        a.line(canvas, knee, f, '\0', skin);
        a.line(canvas, f, [f[0] + 0.8, f[1]], '_', gleam);
    }
    let tail_y = body_y - 0.8 - a.pose * 0.7 + (phase * 0.65).sin() * 0.5;
    let tail_length = if a.species == 2 { 7.1 } else { 6.2 };
    a.triangle(
        canvas,
        [-2.4, body_y - 0.5],
        [-2.0, body_y + 0.8],
        [-tail_length, tail_y],
        skin,
    );
    a.oval(canvas, [0.0, body_y], [body_x, body_h], skin);
    let head: Point;
    match a.species {
        0 => {
            // Sauropod: ascending tapered neck with a bend and a small rounded head.
            let bend = [2.6 + a.pose * 0.35, -5.8 - bob];
            head = [4.0 + a.pose * 0.6, -7.0 + a.pose * 0.4 - bob];
            a.triangle(canvas, [1.4, body_y + 0.4], [3.0, body_y], bend, skin);
            a.triangle(
                canvas,
                [bend[0] - 0.7, bend[1] + 1.0],
                [bend[0] + 0.6, bend[1] + 1.0],
                head,
                skin,
            );
            a.line(canvas, bend, head, '\0', skin);
            a.oval(canvas, head, [1.65, 0.9], gleam);
        }
        1 => {
            // Ceratopsian: deep frill, low beaked head, paired brow and nasal horns.
            head = [4.1, body_y + 0.2 + a.pose * 0.25];
            a.oval(canvas, [2.5, body_y - 0.6], [1.2, 1.9], shade);
            for n in 0..3 {
                let x = 1.6 + n as f32 * 0.65;
                a.line(
                    canvas,
                    [x, body_y - 1.4],
                    [x - 0.2, body_y - 2.3],
                    '^',
                    gleam,
                );
            }
            a.oval(canvas, head, [1.9, 1.0], skin);
            a.line(
                canvas,
                [3.5, head[1] - 0.7],
                [4.2, head[1] - 1.8],
                '/',
                gleam,
            );
            a.line(
                canvas,
                [5.0, head[1] - 0.3],
                [5.7, head[1] - 1.2],
                '^',
                gleam,
            );
            canvas.put(
                a.point(6.0, head[1]),
                if a.facing > 0.0 { '>' } else { '<' },
                gleam,
            );
        }
        2 => {
            // Raptor: horizontal tail, S-neck, narrow muzzle and feathered crown.
            head = [3.8, -4.0 + a.pose * 0.4 - bob];
            a.triangle(
                canvas,
                [1.1, body_y],
                [2.4, body_y + 0.3],
                [2.6, head[1]],
                skin,
            );
            a.line(canvas, [2.6, head[1]], head, '\0', gleam);
            a.oval(canvas, head, [1.65, 0.85], skin);
            for n in 0..3 {
                a.line(
                    canvas,
                    [2.8 + n as f32 * 0.6, head[1] - 0.4],
                    [2.2 + n as f32 * 0.7, head[1] - 1.3],
                    '/',
                    gleam,
                );
            }
            a.line(canvas, [1.7, body_y], [3.0, body_y + 0.8], '\0', gleam);
            a.line(canvas, [3.0, body_y + 0.8], [3.6, body_y + 0.4], '/', gleam);
        }
        _ => {
            // Tyrannosaur: upright haunches, thick neck, oversized jaw and tiny arms.
            head = [3.7, -5.3 + a.pose * 0.3 - bob];
            a.triangle(
                canvas,
                [0.8, body_y + 0.5],
                [3.0, body_y],
                [2.8, head[1] - 0.2],
                skin,
            );
            a.oval(canvas, head, [2.35, 1.35], gleam);
            a.line(
                canvas,
                [2.8, head[1] + 0.8],
                [5.1, head[1] + 0.8],
                '_',
                shade,
            );
            a.line(canvas, [2.0, body_y + 0.1], [3.4, body_y + 0.6], '\0', skin);
            canvas.put(a.point(3.5, body_y + 0.5), 'w', gleam);
        }
    }
    canvas.put(a.point(head[0] + 0.35, head[1] - 0.15), 'o', Color::White);
    canvas.put(a.point(head[0] + 1.2, head[1] + 0.35), 'u', gleam);
}

fn herd(width: usize, height: usize, seed: u64, k: &[f32; 9], palette: &[Color; 5]) -> Vec<Animal> {
    let unit = (width as f32 / 80.0).min(height as f32 / 24.0).max(0.5);
    let cols = ((width as f32 / (24.0 * unit)).floor() as usize).clamp(1, 48);
    let rows = ((height as f32 * 0.62 / (7.0 * unit)).floor() as usize).clamp(1, 24);
    let capacity = (cols * rows).min(width.saturating_mul(height));
    let count = (capacity as f32 * k[0]).round() as usize;
    let slot_w = width as f32 / cols as f32;
    let slot_h = height as f32 * 0.58 / rows as f32;
    let variant = seed ^ hash((k[7] * 65535.0).round() as u64 + 71);
    (0..count)
        .map(|i| {
            // Round-robin slots keep density changes from moving surviving identities.
            let slot = i * 5 % capacity;
            // Five may share a divisor with capacity; use the natural order in that case.
            let slot = if capacity % 5 == 0 { i } else { slot };
            let row = slot / cols;
            let identity = hash(variant.wrapping_add(i as u64 * 131));
            let depth = (row as f32 + 0.5) / rows as f32;
            let scale = unit
                * k[2]
                * (0.9 + random(identity, 3) * 0.15)
                * (1.0 - k[6] * (1.0 - depth) * 0.2);
            let balanced = (i + (hash(variant) % 4) as usize) % 4;
            let species = if random(identity, 31) < (k[1] - 0.5).abs() * 2.0 {
                (hash(identity) % 2) as usize + if k[1] > 0.5 { 2 } else { 0 }
            } else {
                balanced
            };
            Animal {
                identity,
                species,
                facing: if random(identity, 2) < 0.5 { -1.0 } else { 1.0 },
                origin: [
                    slot_w * (slot % cols) as f32 + slot_w * (0.44 + random(identity, 4) * 0.12),
                    height as f32 * 0.35 + slot_h * (row + 1) as f32
                        - random(identity, 5) * unit * 0.55,
                ],
                scale: [scale, scale * 0.88],
                phase: random(identity, 6) * TAU,
                pose: random(identity, 7) * 2.0 - 1.0,
                color: tint(
                    palette[(hash(identity) % 5) as usize],
                    Color::Rgb {
                        r: 185,
                        g: 230,
                        b: 152,
                    },
                    0.28 + 0.25 * random(identity, 9),
                ),
            }
        })
        .collect()
}

fn fern(canvas: &mut Canvas<'_>, base: Point, size: f32, phase: f32, color: Color) {
    let sway = phase.sin() * size * 0.32;
    let tip = [base[0] + sway, base[1] - size * 1.6];
    canvas.line(base, tip, '|', color);
    for leaf in 1..=3 {
        let f = leaf as f32 / 4.0;
        let middle = [base[0] + sway * f, base[1] - size * 1.6 * f];
        for sign in [-1.0, 1.0] {
            let end = [middle[0] + sign * size * (1.3 - f), middle[1] - size * 0.5];
            canvas.line(middle, end, '\0', color);
        }
    }
}

fn draw_preserve(frame: &mut ModeFrame<'_>, k: &[f32; 9]) {
    // Initialize the clipped canvas, seed identities, atmosphere and terrain.
    // Evaluate herd gait, water, cloud and fern phases from the explicit clock.
    // Rasterize bounded mountain columns, fence, plants and depth-sorted animals.
    // Finish with sparse rain and corner foliage for foreground occlusion.
    let height = frame.height.min(frame.grid.len());
    let width = frame.width.min(
        frame
            .grid
            .iter()
            .take(height)
            .map(Vec::len)
            .min()
            .unwrap_or(0),
    );
    if width == 0 || height == 0 {
        return;
    }
    let time = if frame.time.is_finite() {
        frame.time.rem_euclid(4096.0) * k[5]
    } else {
        0.0
    };
    let unit = (width as f32 / 80.0).min(height as f32 / 24.0).max(0.5);
    let night = Color::Rgb {
        r: 12,
        g: 24,
        b: 37,
    };
    let sky = tint(
        night,
        Color::Rgb {
            r: 47,
            g: 78,
            b: 85,
        },
        k[4],
    );
    let mist = tint(frame.palette[2], sky, 0.45 + k[6] * 0.28);
    let leaf = tint(
        frame.palette[1],
        Color::Rgb {
            r: 110,
            g: 183,
            b: 114,
        },
        0.55,
    );
    let gold = tint(
        frame.palette[3],
        Color::Rgb {
            r: 240,
            g: 196,
            b: 113,
        },
        0.45,
    );
    let horizon = height as f32 * 0.33;
    let seed = frame.seed ^ hash((k[7] * 65535.0).round() as u64);
    let mut canvas = Canvas {
        grid: frame.grid,
        width,
        height,
    };
    for y in 0..height {
        for x in 0..width {
            let p = y as f32 / height as f32;
            let noise = random(seed, (x + y * width) as u64);
            let ground = y as f32 > horizon;
            let stream = (x as f32 / width as f32 - 0.51 - (p * 8.0).sin() * 0.065).abs();
            let ch = if ground && stream < 0.014 + p * 0.015 {
                if ((x as f32 / unit + time * 2.0 + y as f32 / unit) * 0.7).sin() > 0.35 {
                    '~'
                } else {
                    ' '
                }
            } else if ground && noise < 0.045 {
                ','
            } else if ground && noise > 0.982 {
                '.'
            } else if !ground && k[4] < 0.3 && noise < 0.016 {
                '*'
            } else {
                ' '
            };
            canvas.grid[y][x] =
                Cell::with_bg(ch, mist, if ground { tint(sky, leaf, 0.07) } else { sky });
        }
    }
    // Mountain silhouettes occupy only their clipped skyline-to-horizon interval.
    for x in 0..width {
        let nx = x as f32 / width as f32;
        let ridge = ((nx * 4.0 + random(seed, 8)).fract() - 0.5).abs() * 2.0;
        let top = (height as f32 * (0.12 + ridge * 0.14 + (1.0 - k[6]) * 0.06)) as usize;
        for y in top..(horizon as usize).min(height) {
            let ch = if y == top {
                if ridge < 0.1 {
                    '^'
                } else if (nx * 4.0 + random(seed, 8)).fract() < 0.5 {
                    '/'
                } else {
                    '\\'
                }
            } else if (x + y * 3) % 13 == 0 {
                '.'
            } else {
                ' '
            };
            canvas.put([x as f32, y as f32], ch, mist);
        }
    }
    canvas.ellipse(
        [width as f32 * 0.81, height as f32 * 0.08],
        [2.0 * unit, 0.8 * unit],
        gold,
        seed,
    );
    for i in 0..3 {
        let x = (random(seed, 40 + i) * width as f32 + time * unit * 0.5).rem_euclid(width as f32);
        let y = (0.055 + i as f32 * 0.035) * height as f32;
        canvas.line([x - 2.0 * unit, y], [x + 2.0 * unit, y], '_', mist);
    }
    let fence_y = (height as f32 * 0.35).round();
    canvas.line([0.0, fence_y], [width as f32 - 1.0, fence_y], '-', gold);
    let post_step = (6.0 * unit).round().max(2.0) as usize;
    for x in (0..width).step_by(post_step) {
        canvas.line(
            [x as f32, fence_y - unit],
            [x as f32, fence_y + unit],
            '|',
            gold,
        );
    }
    let plant_count = ((width as f32 / (6.0 * unit)) * k[3]) as usize;
    for i in 0..plant_count.min(width) {
        let identity = hash(seed.wrapping_add(i as u64 * 97));
        let x = random(identity, 1) * width as f32;
        let y = horizon + unit * (1.4 + random(identity, 2) * 1.7);
        fern(
            &mut canvas,
            [x, y],
            unit * (0.7 + random(identity, 3)),
            time + random(identity, 4) * TAU,
            leaf,
        );
    }
    if width >= 36 && height >= 12 {
        let label = "[ FERN VALLEY ]";
        let start = width / 2 - label.len() / 2;
        for (i, ch) in label.chars().enumerate() {
            canvas.put([(start + i) as f32, fence_y], ch, gold);
        }
    }
    let mut animals = herd(width, height, frame.seed, k, frame.palette);
    animals.sort_by(|a, b| a.origin[1].total_cmp(&b.origin[1]));
    for animal in &mut animals {
        let distance = 1.0 - animal.origin[1] / height as f32;
        animal.color = tint(
            animal.color,
            sky,
            (1.0 - k[4]) * 0.38 + distance * k[6] * 0.25,
        );
        // A short in-slot stroll leaves the composition's margins intact.
        animal.origin[0] += (time * 0.55 + animal.phase).sin() * unit * 0.8;
        draw_dinosaur(&mut canvas, animal, time);
    }
    let drops = (width.saturating_mul(height) as f32 * k[8] * 0.022) as usize;
    for i in 0..drops.min(width.saturating_mul(height)) {
        let x = (random(seed, 500 + i as u64 * 2) * width as f32 - time * unit * 2.0)
            .rem_euclid(width as f32);
        let y = (random(seed, 501 + i as u64 * 2) * height as f32 + time * unit * 5.0)
            .rem_euclid(height as f32);
        // Sparse atmospheric marks only touch unoccupied cells.
        if canvas.grid[y as usize][x as usize].ch == ' ' {
            canvas.put([x.floor(), y.floor()], '/', mist);
        }
    }
    for i in 0..(k[3] * 6.0).ceil() as usize {
        let left = i % 2 == 0;
        let x = if left {
            i as f32 * unit * 1.3
        } else {
            width as f32 - 1.0 - i as f32 * unit * 1.3
        };
        fern(
            &mut canvas,
            [x, height as f32 - 1.0],
            unit * (1.5 + random(seed, 90 + i as u64)),
            time * 1.3 + i as f32,
            leaf,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{SeedableRng, rngs::StdRng};

    const PALETTE: [Color; 5] = [
        Color::Rgb {
            r: 232,
            g: 170,
            b: 112,
        },
        Color::Rgb {
            r: 130,
            g: 205,
            b: 145,
        },
        Color::Rgb {
            r: 118,
            g: 191,
            b: 218,
        },
        Color::Rgb {
            r: 229,
            g: 204,
            b: 140,
        },
        Color::Rgb {
            r: 191,
            g: 153,
            b: 205,
        },
    ];

    fn defaults() -> [f32; 9] {
        std::array::from_fn(|i| PARAMS[i].default)
    }

    fn render(width: usize, height: usize, seed: u64, time: f32, knobs: &[f32; 9]) -> Grid {
        let mut grid = vec![vec![Cell::blank(); width]; height];
        let mut rng = StdRng::seed_from_u64(seed);
        MODE.render(&mut ModeFrame {
            grid: &mut grid,
            width,
            height,
            seed,
            palette: &PALETTE,
            rng: &mut rng,
            time,
            args: &[],
            param_values: Some(knobs),
        });
        grid
    }

    fn plain(grid: &Grid) -> String {
        grid.iter()
            .map(|row| {
                row.iter()
                    .map(|cell| cell.ch)
                    .collect::<String>()
                    .trim_end()
                    .to_owned()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn preserve_at_dawn() {
        insta::assert_snapshot!(plain(&render(80, 24, 1701, 0.0, &defaults())), @r"

                                                                                      __
                        _____                                          (  )
                      /^^            _____/^^                 /^^                 /^^
                   ///.  \\\           ///   \\\           ///   \\\           ///   \\\
        \\\     ///.        \\\     ///      .  \\\     ///         \\\     ///     .
           \\\//.            . \\\//      .        \\\//            .  \\\//     .
        |     |     |     |     |     \\//  |     |     |     |     | /   |  \\//     |
        |\//--|-----|-----|-----|-\|\//\/[ FERN VALLEY ]|-----|-----|-----|--_//_\/---|-
        |_/_  |  ,__|     |     |_\_\/__    |   \//     |     | /   |   o)|   __|\//  |
                 uo-\             ,_ _           _                ,    u \\    |  _
        ,          \\          ,             ~   ^^^                     |\
          ,         ||__ ____,        , , _ ~  ___^^/^                   ||____,       ,
                   .||   )_          /     __(    (/o^>                  ||   )___
               .     |  _|                ~   -------u  ,           ,     /|  /    , ,
               ,   ___.___              .     __ |__     .               ______ ,   .
                                             .......                            .
          ,      , ___       ,        ,     ///~                        ,
           ,       uo_)                ,    o/~~                   .       , ^^
          \|//       |_ :)____      ,      u--/\ _______                    _ ^^/^\|/\|/
        |\\///      w_-\ -_      .          ///- -___                  ___(:  (/o\ /////
        __// _         |/                       /_ ~~~      .  /          _------_\//__
        _____   .   ..._...      ,           .__....~             .      .__..__  ______
        |  |      , ,                        .                           ,  ,      |  |
        ");
    }

    #[test]
    fn preserve_in_motion() {
        insta::assert_snapshot!(plain(&render(80, 24, 1701, 2.25, &defaults())), @r"

        ____                    /
                         _____                                         (  )
                      /^^             _____^^                 /^^                 /^^
                   ///.  \\\           ///   \\\           ///   \\\           ///   \\\
        \\\     ///.        \\\     ///      .  \\\     ///         \\\     ///     .
           \\\//.            . \\\//      .        \\\//            .  \\\//     .
        |     |     |     |     |     \//   |     |     |     |     |     |  \\//     |
        |-\/--|-----|-----|-----|\\\\////[ FERN VALLEY ]|-----|-----|-----|--_//_\/---|-
        |_/__ |  , o_     |     |_/_//__    |   _//~    |     |     |  o) |   __|\/_  |
                  u--\            ,_ _          ~_                ,    u\\     |  _
        ,           \\         ,               ~ ^^                     |\
          ,          ||_______        , , \\_ ~__^^)/^                  ||_______      ,
                   . ||   )_                \(   (_/o>                  | |  )_
               .    __/ __|                 ~/-------u  ,           ,    /_ _/     , ,
               ,    .__.._.             .  ~~___ ___     .              __.._.. ,   .
                                           ~~.......                            .
          ,      ,  __       ,        ,     //                          ,
           ,       uo_)                ,   o/    ~  ___           /.       , ^^
         \|/         |_ : )___      ,  /  u--/\ _____ , /             ___    ^^/ ^\|/| /
        \\///        w-\ -_      .          //- -__        /            _(:: (_o^\\/////
        __/ __          /|                     |/           .            |------u_\//___
        ____    .    .___..      ,          ..__...   ~~          .      ___.___  _____
        |  |      , ,                        .       ~~~~                ,  ,      |  |
        ");
    }

    #[test]
    fn deterministic_frames_and_independent_identities() {
        let k = defaults();
        let first = render(80, 24, 1701, 0.0, &k);
        let later = render(80, 24, 1701, 2.25, &k);
        assert_ne!(plain(&first), plain(&later));
        assert_eq!(first, render(80, 24, 1701, 0.0, &k));
        assert_eq!(later, render(80, 24, 1701, 2.25, &k));
        assert_ne!(first, render(80, 24, 1702, 0.0, &k));
        let mut frozen = k;
        frozen[5] = 0.0;
        assert_eq!(
            render(80, 24, 1701, 0.0, &frozen),
            render(80, 24, 1701, 99.0, &frozen)
        );
        let animals = herd(80, 24, 1701, &k, &PALETTE);
        for species in 0..4 {
            assert!(animals.iter().any(|a| a.species == species));
        }
        let mut sparse = k;
        sparse[0] = 0.5;
        assert_eq!(herd(80, 24, 1701, &sparse, &PALETTE), animals[..3]);
        let mut mixed = k;
        mixed[1] = 0.0;
        assert!(
            herd(80, 24, 1701, &mixed, &PALETTE)
                .iter()
                .all(|a| a.species < 2)
        );
        mixed[1] = 1.0;
        assert!(
            herd(80, 24, 1701, &mixed, &PALETTE)
                .iter()
                .all(|a| a.species >= 2)
        );
        for step in 0..64 {
            let phase = step as f32 * TAU / 64.0;
            let feet = [foot(0.0, phase), foot(0.0, phase + std::f32::consts::PI)];
            assert!(feet.iter().all(|f| f[1] <= 0.0));
            assert!(feet.iter().any(|f| f[1].abs() < 0.00001));
        }
    }

    #[test]
    fn controls_clamp_and_change_frames() {
        let k = defaults();
        for i in 0..PARAMS.len() {
            let mut low = k;
            let mut high = k;
            low[i] = PARAMS[i].min;
            high[i] = PARAMS[i].max;
            assert_ne!(
                render(80, 24, 1701, 1.75, &low),
                render(80, 24, 1701, 1.75, &high),
                "{}",
                PARAMS[i].key
            );
            let low_frame = render(32, 12, 11, 0.0, &low);
            let high_frame = render(32, 12, 11, 0.0, &high);
            low[i] -= 1000.0;
            high[i] += 1000.0;
            assert_eq!(low_frame, render(32, 12, 11, 0.0, &low));
            assert_eq!(high_frame, render(32, 12, 11, 0.0, &high));
            for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
                let mut values = k;
                values[i] = invalid;
                assert_eq!(
                    render(32, 12, 11, 0.0, &values),
                    render(32, 12, 11, 0.0, &k)
                );
            }
        }
    }

    #[test]
    fn small_empty_and_large_grids_are_bounded() {
        for (w, h) in [(0, 0), (0, 4), (1, 1), (2, 3), (9, 4), (2000, 1000)] {
            for extreme in [false, true] {
                let k = std::array::from_fn(|i| {
                    if extreme {
                        PARAMS[i].max
                    } else {
                        PARAMS[i].min
                    }
                });
                let grid = render(w, h, u64::MAX, -10.0, &k);
                assert_eq!(grid.len(), h);
                assert!(
                    grid.iter()
                        .all(|row| row.len() == w && row.iter().all(|cell| cell.ch.is_ascii()))
                );
            }
        }
        insta::assert_snapshot!(plain(&render(16, 8, 1701, 0.5, &defaults())), @r"
        __
        \ /_____\ /^\-/^
                     \//
        |--|-//--|--|_/_
        |  | ||o |  |  |
        \\/   u\\   \|//
        _/__   || )/_/_/
        __     ____  __
        ");
    }
}
