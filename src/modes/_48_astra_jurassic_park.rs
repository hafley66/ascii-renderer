//! Astra's Fern Valley preserve: deterministic identities, terrain lanes and gaits.
//! Every frame is evaluated independently from its explicit inputs.

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
    param!("GAIT", "stride and bounce", 0.0, 2.0, 1.0, 0.1),
    param!("MIGRATION", "in-slot travel", 0.0, 1.0, 0.5, 0.05),
    param!("FORMATION", "loose / staggered herd", 0.0, 1.0, 0.5, 0.05),
    param!("JUVENILES", "juvenile mix", 0.0, 1.0, 0.3, 0.05),
    param!("ACTIVITY", "browsing and alert poses", 0.0, 2.0, 1.0, 0.1),
    param!(
        "HERD_VARIATION",
        "individual differences",
        0.0,
        1.0,
        0.8,
        0.05
    ),
];

impl Mode for JurassicParkMode {
    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
    fn name(&self) -> &'static str {
        "astra-jurassic-park"
    }

    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
    fn help(&self) -> &'static str {
        "Fern Valley: procedural longnecks, ceratopsians, raptors and tyrannosaurs. [herd] [species] [scale] [ferns] [light] [speed] [depth] [variation] [rain] [gait] [migration] [formation] [juveniles] [activity] [herd_variation]"
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

#[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
fn hash(mut n: u64) -> u64 {
    n = (n ^ (n >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    n = (n ^ (n >> 27)).wrapping_mul(0x94d049bb133111eb);
    n ^ (n >> 31)
}

#[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
fn random(seed: u64, lane: u64) -> f32 {
    (hash(seed.wrapping_add(lane.wrapping_mul(0x9e3779b97f4a7c15))) >> 40) as f32 / 16_777_216.0
}

#[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
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

#[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
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
    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
    fn put(&mut self, p: Point, ch: char, color: Color) {
        let x = p[0].round() as i32;
        let y = p[1].round() as i32;
        if x >= 0 && y >= 0 && (x as usize) < self.width && (y as usize) < self.height {
            let cell = &mut self.grid[y as usize][x as usize];
            cell.ch = ch;
            cell.fg = color;
        }
    }

    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
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

    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
    fn ellipse(&mut self, center: Point, radius: Point, color: Color, identity: u64) {
        let rx = radius[0].max(0.55);
        let ry = radius[1].max(0.7);
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
                    } else if random(
                        identity,
                        ((dx * 8.0 + 8.0) as u64) + 17 * (dy * 4.0 + 4.0) as u64,
                    ) < 0.24
                    {
                        ':'
                    } else {
                        ' '
                    };
                    self.put([x as f32, y as f32], ch, color);
                }
            }
        }
    }

    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
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
    juvenile: bool,
    build: Point,
    tail: f32,
    marking: char,
    cadence: f32,
    stride: f32,
    // A terrain lane is fixed for the identity while its x position moves.
    ground: f32,
    terrain_phase: f32,
    travel: f32,
    color: Color,
}

impl Animal {
    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
    fn ground_at(&self, x: f32) -> f32 {
        self.ground + (x / (self.scale[0] * 9.0) + self.terrain_phase).sin() * self.scale[1] * 0.35
    }

    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
    fn point(&self, x: f32, y: f32) -> Point {
        [
            self.origin[0] + self.facing * x * self.scale[0],
            self.origin[1] + y * self.scale[1],
        ]
    }

    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
    fn oval(&self, canvas: &mut Canvas<'_>, center: Point, radius: Point, color: Color) {
        canvas.ellipse(
            self.point(center[0], center[1]),
            [radius[0] * self.scale[0], radius[1] * self.scale[1]],
            color,
            self.identity,
        );
    }

    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
    fn line(&self, canvas: &mut Canvas<'_>, a: Point, b: Point, ch: char, color: Color) {
        canvas.line(self.point(a[0], a[1]), self.point(b[0], b[1]), ch, color);
    }

    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
    fn triangle(&self, canvas: &mut Canvas<'_>, a: Point, b: Point, c: Point, color: Color) {
        canvas.triangle(
            self.point(a[0], a[1]),
            self.point(b[0], b[1]),
            self.point(c[0], c[1]),
            color,
        );
    }
}

#[derive(Debug, PartialEq)]
struct Gait {
    phase: f32,
    stride: f32,
    lift: f32,
    bob: f32,
    hop: f32,
    pose: f32,
    tail: f32,
}

#[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
fn gait(a: &Animal, time: f32, k: &[f32; 15]) -> Gait {
    // Independent clocks are identity-derived; evaluating t=0 still yields a full pose.
    let phase = a.phase + time * a.cadence;
    let behavior = a.pose * 2.0 + time * (0.55 + random(a.identity, 23) * 0.7);
    let hop = if a.species == 2 {
        behavior.sin().max(0.0).powi(8) * k[13] * k[9] * 0.7
    } else {
        0.0
    };
    Gait {
        phase,
        stride: a.stride * k[9],
        lift: [0.65, 0.55, 1.1, 0.9][a.species] * k[9],
        bob: (phase * 2.0).cos() * [0.22, 0.25, 0.38, 0.42][a.species] * k[9] + hop,
        hop,
        pose: a.pose * 0.3 + behavior.sin() * k[13],
        tail: (phase * 0.7 + a.pose * 3.0).sin() * k[9] * 0.95,
    }
}

#[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
fn foot(a: &Animal, g: &Gait, hip: f32, phase: f32) -> Point {
    // Swing rises off the sampled terrain; the opposite half-cycle remains planted.
    let x = hip + g.stride * phase.cos();
    let world_x = a.point(x, 0.0)[0];
    [
        x,
        (a.ground_at(world_x) - a.origin[1]) / a.scale[1] - g.lift * phase.sin().max(0.0) - g.hop,
    ]
}

#[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
fn place_animal(a: &mut Animal, time: f32, k: &[f32; 15]) {
    // Travel stays inside the reserved slot. Ground is resampled after the x write.
    a.origin[0] += a.facing
        * a.travel
        * k[10]
        * ((time * a.cadence * 0.24 + a.phase).sin() - a.phase.sin())
        * 0.5;
    a.origin[1] = a.ground_at(a.origin[0]);
}

#[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
fn draw_leg(canvas: &mut Canvas<'_>, a: &Animal, g: &Gait, leg: usize, body_y: f32, color: Color) {
    let hip = if a.species < 2 {
        if leg < 2 { -2.1 } else { 2.0 }
    } else {
        -0.8 + leg as f32 * 1.4
    };
    // Quadrupeds alternate diagonal pairs; bipeds alternate left and right.
    let offset = if a.species < 2 {
        [0.0, 1.0, 1.0, 0.0][leg]
    } else {
        leg as f32
    };
    let f = foot(a, g, hip, g.phase + offset * std::f32::consts::PI);
    let knee = [(hip + f[0]) * 0.5 - 0.45, body_y * 0.35];
    let toe_x = f[0] + 0.8;
    let toe_y = f[1]
        + (a.ground_at(a.point(toe_x, 0.0)[0]) - a.ground_at(a.point(f[0], 0.0)[0])) / a.scale[1];
    a.line(canvas, [hip, body_y + 0.4], knee, '\0', color);
    a.line(canvas, knee, f, '\0', color);
    a.line(canvas, f, [toe_x, toe_y], '_', color);
}

#[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
fn draw_dinosaur(canvas: &mut Canvas<'_>, animal: &Animal, time: f32, k: &[f32; 15]) {
    // Evaluate one stable identity's pose and gait; origin.y is its ground plane.
    // Draw shadow, far limbs, tapered tail, body, neck/crest, head, near limbs, eye.
    let a = animal;
    let g = gait(a, time, k);
    let bob = g.bob;
    let skin = a.color;
    let shade = tint(skin, Color::Black, 0.30);
    let gleam = tint(skin, Color::White, 0.40);
    let (body_y, body_x, body_h) = match a.species {
        0 => (-2.6 - bob, 3.4 * a.build[0], 1.35 * a.build[1]),
        1 => (-2.4 - bob, 3.5 * a.build[0], 1.45 * a.build[1]),
        2 => (-2.6 - bob, 2.5 * a.build[0], 0.95 * a.build[1]),
        _ => (-3.5 - bob, 3.0 * a.build[0], 1.6 * a.build[1]),
    };
    for n in -4..=4 {
        let x = a.point(n as f32, 0.0)[0];
        canvas.put([x, a.ground_at(x) + 0.5 * a.scale[1]], '.', shade);
    }
    let leg_count = if a.species < 2 { 4 } else { 2 };
    for leg in (0..leg_count).step_by(2) {
        draw_leg(canvas, a, &g, leg, body_y, shade);
    }
    let tail_y = body_y - 0.8 - a.pose * 0.7 + g.tail;
    let tail_length = if a.species == 2 { 7.1 } else { 6.2 } * a.tail;
    a.triangle(
        canvas,
        [-2.4, body_y - 0.5],
        [-2.0, body_y + 0.8],
        [-tail_length, tail_y],
        skin,
    );
    a.oval(canvas, [0.0, body_y], [body_x, body_h], skin);
    // Body-local markings move with the animal, with fewer marks on small silhouettes.
    for n in 0..if a.scale[0] < 0.75 { 1 } else { 3 } {
        let x = (n as f32 - 1.0) * body_x * 0.48;
        canvas.put(a.point(x, body_y), a.marking, shade);
    }
    let head: Point;
    match a.species {
        0 => {
            // Sauropod: ascending tapered neck with a bend and a small rounded head.
            let bend = [2.6 + g.pose * 0.5, -5.8 + g.pose * 0.55 - bob];
            head = [4.0 + g.pose * 0.7, -7.0 + g.pose * 1.3 - bob];
            a.triangle(canvas, [1.4, body_y + 0.4], [3.0, body_y], bend, skin);
            a.triangle(
                canvas,
                [bend[0] - 0.7, bend[1] + 1.0],
                [bend[0] + 0.6, bend[1] + 1.0],
                head,
                skin,
            );
            a.line(canvas, bend, head, '\0', skin);
            a.oval(
                canvas,
                head,
                [if a.juvenile { 1.95 } else { 1.65 }, 0.9],
                gleam,
            );
        }
        1 => {
            // Ceratopsian: deep frill, low beaked head, paired brow and nasal horns.
            head = [4.1 + g.pose * 0.2, (body_y + 0.2 + g.pose * 0.85).min(-1.1)];
            a.oval(canvas, [2.5, body_y - 0.6], [1.2, 1.9], shade);
            for n in 0..3 {
                let x = 1.6 + n as f32 * 0.65;
                a.line(
                    canvas,
                    [x, body_y - 1.4],
                    [x - 0.2, body_y - if a.juvenile { 1.8 } else { 2.3 }],
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
            head = [3.8 + g.pose * 0.7, -4.0 - g.pose * 0.8 - bob];
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
            head = [3.7 + g.pose * 0.35, -5.3 + g.pose * 0.7 - bob];
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
    for leg in (1..leg_count).step_by(2) {
        draw_leg(canvas, a, &g, leg, body_y, gleam);
    }
    canvas.put(a.point(head[0] + 1.2, head[1] + 0.65), 'u', gleam);
    canvas.put(a.point(head[0] + 0.35, head[1] - 0.15), 'o', Color::White);
}

#[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
fn herd(
    width: usize,
    height: usize,
    seed: u64,
    k: &[f32; 15],
    palette: &[Color; 5],
) -> Vec<Animal> {
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
            let balanced = (i + (hash(variant) % 4) as usize) % 4;
            let species = if random(identity, 31) < (k[1] - 0.5).abs() * 2.0 {
                (hash(identity) % 2) as usize + if k[1] > 0.5 { 2 } else { 0 }
            } else {
                balanced
            };
            let variation = k[14];
            let juvenile = random(identity, 32) < k[12];
            let age_scale = if juvenile {
                0.68 + random(identity, 33) * 0.1
            } else {
                1.0
            };
            let scale = unit
                * k[2]
                * age_scale
                * (1.0 + (random(identity, 3) - 0.5) * 0.4 * variation)
                * (1.0 - k[6] * (1.0 - depth) * 0.3);
            // Reserve the full tail/head envelope and travel room within each slot.
            let scale = scale.min(slot_w / 22.0).min(slot_h / 8.0).max(0.05);
            let stagger = if row % 2 == 0 { -1.0 } else { 1.0 } * k[11] * 0.06;
            let x = slot_w
                * ((slot % cols) as f32
                    + 0.5
                    + stagger
                    + (random(identity, 4) - 0.5) * 0.1 * variation);
            let ground = height as f32 * 0.35 + slot_h * (row + 1) as f32
                - unit * (0.3 + random(identity, 5) * variation * 0.8)
                - k[11] * unit * ((slot % cols) as f32 * 1.5).sin() * 0.6;
            let cadence = match species {
                0 => 1.0 + random(identity, 18) * 0.7,
                1 => 1.4 + random(identity, 18) * 0.9,
                2 => 3.2 + random(identity, 18) * 1.5,
                _ => 1.7 + random(identity, 18) * 0.8,
            } * if juvenile { 1.2 } else { 1.0 };
            Animal {
                identity,
                species,
                facing: if random(identity, 2) < 0.5 * variation {
                    -1.0
                } else {
                    1.0
                },
                origin: [x, ground],
                scale: [
                    scale,
                    scale * (0.88 + (random(identity, 34) - 0.5) * variation * 0.18),
                ],
                phase: random(identity, 6) * TAU,
                pose: random(identity, 7) * 2.0 - 1.0,
                juvenile,
                build: [
                    1.0 + (random(identity, 35) - 0.5) * variation * 0.3,
                    1.0 + (random(identity, 36) - 0.5) * variation * 0.4,
                ],
                tail: 1.0 + (random(identity, 37) - 0.5) * variation * 0.25,
                marking: if random(identity, 38) < variation {
                    [':', '/', '=', '.'][(hash(identity) % 4) as usize]
                } else {
                    ':'
                },
                cadence,
                stride: [1.05, 0.85, 1.5, 1.3][species] * (0.75 + random(identity, 19) * 0.5),
                ground,
                terrain_phase: random(variant, 39) * TAU,
                travel: ((x - slot_w * (slot % cols) as f32)
                    .min(slot_w * ((slot % cols) + 1) as f32 - x)
                    - scale * 8.0
                    - 0.5)
                    .max(0.0),
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

#[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
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

#[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
fn draw_preserve(frame: &mut ModeFrame<'_>, k: &[f32; 15]) {
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
    for animal in &mut animals {
        place_animal(animal, time, k);
    }
    animals.sort_by(|a, b| a.origin[1].total_cmp(&b.origin[1]));
    for animal in &mut animals {
        let distance = 1.0 - animal.origin[1] / height as f32;
        animal.color = tint(
            animal.color,
            sky,
            (1.0 - k[4]) * 0.38 + distance * k[6] * 0.25,
        );
        draw_dinosaur(&mut canvas, animal, time, k);
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

    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
    fn defaults() -> [f32; 15] {
        std::array::from_fn(|i| PARAMS[i].default)
    }

    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
    fn render(width: usize, height: usize, seed: u64, time: f32, knobs: &[f32; 15]) -> Grid {
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

    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
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
    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
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
        |_/_  |  ,  |     |     |_\_\/__    |   \//     |     | /   |  o )|   __|\//  |
                                  ,_ _           _                ,     \\     |  _
        ,         o_           ,             ~  ^^                      |\
          ,       u-\    /   ,        , ,_____  ()/                     ||___  ,       ,
                   ||::\/            /     __/::|-/o^                   |\: |)__
               .    /|/|                  ~ |__ __-u>   ,           ,    /| /|     , ,
               ,   _____.               .  ........      .             _______. ,   .
                                              //                                .
          ,      ,  __       ,        ,     o//~     /                  ,
           ,       _o_)                ,      |\_ __/              .       ,
          \|//       |\==)___       ,         //|:)/  ,                      ^^   \|/\|/
        |\\///      w__\:)       .             ____ ~                 _____  ^^/ \ /////
        __// _          |                    .......~~      .  /        __\ :_//o_\//__
        _____   .   ..___..      ,                  ~             .      .__.._.u ______
        |  |      , ,                        .                           ,  ,      |  |
        ");
    }

    #[test]
    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
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
        |_/__ |  ,  |     |     |_/_//__    |   _//~    |     |     |     |   __|\/_  |
                                  ,_ _         ^^^                ,    _o      |  _
        ,        o_            ,        ___    ~^^/                    u-_
          ,     u-_          ,        , , __(:: (/_^                     \\____,       ,
                  |\ |___                   /- -|(ou                     || :\)__
               .   | /|                     ____\_      ,           ,     /| /|    , ,
               , ._____                 .  ._......      .              _____._.,   .
                                           ~~~                                  .
          ,      ,  ,        ,        ,     ~   ///     /               ,
           ,                           ,       uo)\   //          /.       ,
         \|/      uo_)____          ,  /        ///\:)/ /             ___   ^^^   \|/| /
        \\///       w_|==)__     .               ~~\|      /            _(:::(//^\\/////
        __/ __       __ |                       ..___..     .            |/--\(ou_\//___
        ____    .  ....__..      ,                    ~~          .     .______.  _____
        |  |      , ,                        .       ~~~~                ,  ,      |  |
        ");
    }

    #[test]
    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
    fn preserve_large_herd() {
        insta::assert_snapshot!(plain(&render(120, 36, 1701, 2.25, &defaults())), @r"
                                            /
                                             /
        ______                                                          /                               ____
                                  _______                                                              (   :)
                                                                                                         --
                            //^^^\                   _______^^^\                        //^^^\                        //^^^\
                         /// .    \\\                  ///      \\\                  ///      \\\                  ///  .   \\\
        \\            /// .          \\\            ///          . \\\            ///      .     \\\            ///  .         \
          \\\      /// .            .   \\\      ///          .       \\\      ///      .           \\\      ///  .            .
             \\\/// .            .         \\\///          .            .\\\///      .            .    \\\///  .            .
                 .            .            .         \\//            .            .            .            .   \ |/     .
                                                     \/ //                                                      \\/ /
        |  \//   |        |        |   ,    | \\//\//_\/ _    |    .   |~~~~    |   /    |        |        |   _ \//_\|/     |
        |-_\/_---|--------|--------|--------|-\\/\\//____----[ FERN VALLEY ]----|--------|--------|--------|----_____\///----|--
        |   _    |        |        |        | __/__/_| |    . |      _//_       |     ,  |      . |        |     ,| |_/__    |
        |        |        |        |       ,| . |  | |    , , |     ~~_|        |    .   |        |    o:_ |     ,  | |    , |
        ,     , ,       .           ,    .               ,         ^^^   ,                          , u( )_
                    (o )  ..   ,                   .     ____   ___^^^)/                  ,              \\\               .
               ,     u\_\              ,.                  ___(: : :(:/_^^    .   ,  ,,                   |\_____ ,            .
        ,     .  ,     |\____                                _(/   :|( o^>.                  ,            ||\:: \)____
              .        |\:::|____  ,        .   ,      ,      /|~--/|-- u,                         ,      -| : --\_
                        \|  /|            .             ,    |~\~  |\\                                /   //|  //|      ,
                      ________                              .__.__ ____.         ,             ,         ____.____.          , ,
                      ,....... ,                            ~~~~~.   .           .,   .                           , , ,.
                         .                                  ~~~~                                 ,               ,    ,      .
                                    ,                        ,~      //   ,                        ,                          ,
                                   /                               _o//        _/   .             ,
                    .       ,    ., ,                              u )_\   , __/      ,        /  .   ,
                       ( o:) __    ,, /                /        ,    ~|/\::)_//             ,         ,       .^^     .
         \\|//         (u___:  :))_     , ,                .        ~//-\--//   /               ,    ____      ^^^   , \ |\/| /
        \ \|/ /   ,       w_\|=:=)____    ,            ,             ~~~ \|          /                  _((: ::(:)/ ^ \,\|/\//
        |\\///              -|---            ,                    ,    ____         ,                    (/:   | /:o>, \ / //| /
        __//   __           __ ||       ,   ,              ,     ,  .. ...... .   ,                      |\   |\ - -u__ \// __/
        /___|__          ... .__.. .    , ,                        .         ~~~.   ,                   ..___..___.    __|___|__
        _   _      , ,                               .             ,        ~~~~~                             ,  ,      ._   _
        |   |               /               .                        ,     ~~~~~           ,       ,                     |   |
        ");
    }

    #[test]
    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
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
    }

    #[test]
    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
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
    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
    fn species_motion_and_terrain_contact() {
        let k = defaults();
        for (width, height) in [(80, 24), (320, 96)] {
            let animals = herd(width, height, 1701, &k, &PALETTE);
            for species in 0..4 {
                let animal = animals.iter().find(|a| a.species == species).unwrap();
                // Isolate one dinosaur so weather and vegetation cannot satisfy motion checks.
                let isolated = |time: f32, knobs: &[f32; 15]| {
                    let mut grid = vec![vec![Cell::blank(); width]; height];
                    let mut a = animal.clone();
                    place_animal(&mut a, time, knobs);
                    draw_dinosaur(
                        &mut Canvas {
                            grid: &mut grid,
                            width,
                            height,
                        },
                        &a,
                        time,
                        knobs,
                    );
                    plain(&grid)
                };
                assert_ne!(
                    isolated(0.0, &k),
                    isolated(2.25, &k),
                    "species {species} at {width}x{height}"
                );
                assert_eq!(isolated(2.25, &k), isolated(2.25, &k));
                let mut still = k;
                still[9] = 0.0;
                still[10] = 0.0;
                still[13] = 0.0;
                assert_eq!(isolated(0.0, &still), isolated(2.25, &still));
                // Each movement channel must affect silhouette with the others disabled.
                for control in [9, 10, 13] {
                    let mut moving = still;
                    moving[control] = PARAMS[control].max;
                    let zero = isolated(0.0, &moving);
                    assert!(
                        [0.5, 1.25, 2.25, 4.0]
                            .into_iter()
                            .any(|time| zero != isolated(time, &moving)),
                        "species {species}, {} at {width}x{height}",
                        PARAMS[control].key
                    );
                }
                for step in 0..128 {
                    let time = step as f32 * 0.125;
                    let mut a = animal.clone();
                    place_animal(&mut a, time, &k);
                    assert_eq!(a.origin[1], a.ground_at(a.origin[0]));
                    let g = gait(&a, time, &k);
                    let mut contact = false;
                    for offset in [0.0, std::f32::consts::PI] {
                        let f = foot(&a, &g, 0.0, g.phase + offset);
                        let world = a.point(f[0], f[1]);
                        let clearance = a.ground_at(world[0]) - world[1];
                        assert!(clearance >= -0.0001);
                        contact |= (clearance - g.hop * a.scale[1]).abs() < 0.0001;
                    }
                    // During a raptor hop both feet can clear terrain by the hop height.
                    assert!(contact);
                }
            }
        }
    }

    #[test]
    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
    fn herd_variants_keep_identity_and_bounded_slots() {
        let k = defaults();
        for (w, h) in [(80, 24), (320, 96), (2000, 1000)] {
            let animals = herd(w, h, 1701, &k, &PALETTE);
            for (i, a) in animals.iter().enumerate() {
                for b in &animals[i + 1..] {
                    assert_ne!(a.identity, b.identity);
                    assert_ne!(a.phase, b.phase);
                    assert_ne!(a.cadence, b.cadence);
                    assert_ne!(a.stride, b.stride);
                    assert_ne!(a.build, b.build);
                }
            }
            let mut adult = k;
            adult[12] = 0.0;
            let mut young = k;
            young[12] = 1.0;
            for (a, b) in herd(w, h, 1701, &adult, &PALETTE)
                .iter()
                .zip(herd(w, h, 1701, &young, &PALETTE))
            {
                assert_eq!(a.identity, b.identity);
                assert!(!a.juvenile && b.juvenile);
                assert!(b.scale[0] < a.scale[0]);
                assert!(b.cadence > a.cadence);
            }
            let mut extreme = k;
            extreme[2] = PARAMS[2].max;
            extreme[10] = 1.0;
            extreme[11] = 1.0;
            extreme[14] = 1.0;
            for seed in 0..16 {
                for mut a in herd(w, h, seed, &extreme, &PALETTE) {
                    let initial = a.clone();
                    for time in [0.0, 2.25, 99.0] {
                        a.clone_from(&initial);
                        place_animal(&mut a, time, &extreme);
                        assert!(a.origin[0] - a.scale[0] * 8.0 >= 0.0);
                        assert!(a.origin[0] + a.scale[0] * 8.0 < w as f32);
                        assert!(a.origin[1] >= 0.0 && a.origin[1] < h as f32);
                    }
                }
            }
        }
    }

    #[test]
    #[tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)]
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
        for w in 0..16 {
            for h in 0..12 {
                let max = std::array::from_fn(|i| PARAMS[i].max);
                let grid = render(w, h, u64::MAX, f32::MAX, &max);
                assert_eq!(grid.len(), h);
                assert!(grid.iter().all(|row| row.len() == w));
            }
        }
        for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert_eq!(
                render(32, 12, 1701, invalid, &defaults()),
                render(32, 12, 1701, 0.0, &defaults())
            );
        }
        insta::assert_snapshot!(plain(&render(16, 8, 1701, 0.5, &defaults())), @r"
        __           _
        \ /_____\ /^\-/^
                     \//
        |--|-//--|--|_/_
        |  | ||  |  |  |
        \\/   o     \|//
        _/__  u|\__ _/_/
        __    .___   __
        ");
    }
}
