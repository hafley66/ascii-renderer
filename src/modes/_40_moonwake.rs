use crossterm::style::Color;
use rand::rngs::StdRng;

use crate::_0_profile::measure_layer;
use crate::color::{darken, lerp_color, rgb};
use crate::opts::param_f32;
use crate::registry::{AnimKind, Mode, ModeFrame, Param};
use crate::types::{Cell, Grid};

pub(super) struct MoonwakeMode;
pub(super) static MODE: MoonwakeMode = MoonwakeMode;

const PARAMS: &[Param] = &[
    param!("SWELL", "wave height", 0.6, 1.2, 1.0, 0.05),
    param!("LINES", "engraved contours", 8.0, 32.0, 18.0, 1.0),
    param!("FOAM", "crest and spray", 0.0, 1.5, 1.0, 0.05),
    param!("MOON", "moon radius", 0.0, 1.5, 1.0, 0.05),
    param!("REFLECT", "moonlit water", 0.0, 1.5, 1.0, 0.05),
    param!("BOAT", "sailing lantern", 0.0, 1.0, 1.0, 1.0),
    param!("SPEED", "ocean clock", 0.0, 2.0, 0.65, 0.05),
];

impl Mode for MoonwakeMode {
    fn name(&self) -> &'static str {
        "moonwake"
    }

    fn help(&self) -> &'static str {
        "Moonlit breaking wave, engraved currents, branching foam, drifting spray, and a lantern sailboat [swell] [lines] [foam] [moon] [reflect] [boat] [speed]"
    }

    fn animation(&self) -> AnimKind {
        AnimKind::Iterate
    }

    fn params(&self) -> &'static [Param] {
        PARAMS
    }

    fn render(&self, frame: &mut ModeFrame<'_>) {
        let params = MoonwakeParams::from_inputs(frame.args, frame.param_values);
        draw_moonwake(
            frame.grid,
            frame.width,
            frame.height,
            frame.seed,
            frame.palette,
            frame.rng,
            frame.time,
            &params,
        );
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct MoonwakeParams {
    swell: f32,
    lines: usize,
    foam: f32,
    moon: f32,
    reflect: f32,
    boat: bool,
    speed: f32,
}

impl MoonwakeParams {
    fn from_inputs(args: &[String], values: Option<&[f32]>) -> Self {
        let read = |i: usize| {
            let p = &PARAMS[i];
            let value = args
                .get(i + 4)
                .and_then(|s| s.parse::<f32>().ok())
                .or_else(|| values.and_then(|v| v.get(i)).copied())
                .unwrap_or_else(|| param_f32(p.key, p.default));
            if value.is_finite() {
                value.clamp(p.min, p.max)
            } else {
                p.default
            }
        };
        Self {
            swell: read(0),
            lines: read(1).round() as usize,
            foam: read(2),
            moon: read(3),
            reflect: read(4),
            boat: read(5) >= 0.5,
            speed: read(6),
        }
    }
}

type Point = [f32; 2];
const TAU: f32 = std::f32::consts::TAU;
const DOTS: [[u8; 4]; 2] = [[1, 2, 4, 64], [8, 16, 32, 128]];

fn hash(seed: u64, index: usize) -> f32 {
    let mut n = seed.wrapping_add((index as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15));
    n = (n ^ (n >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    n = (n ^ (n >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    ((n ^ (n >> 31)) >> 40) as f32 / 16_777_216.0
}

fn mix(a: Point, b: Point, t: f32) -> Point {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]
}

fn cubic(points: [Point; 4], t: f32) -> Point {
    let a = mix(points[0], points[1], t);
    let b = mix(points[1], points[2], t);
    let c = mix(points[2], points[3], t);
    mix(mix(a, b, t), mix(b, c, t), t)
}

// Both edges end at the lip. Their interpolation gives continuous engraved
// currents from the foot of the wave into its overhanging spiral.
fn wave(u: f32, v: f32, clock: f32, swell: f32) -> Point {
    const OUTER: [[Point; 4]; 3] = [
        [[-0.12, 1.08], [0.12, 0.98], [0.06, 0.15], [0.40, 0.16]],
        [[0.40, 0.16], [0.58, 0.14], [0.69, 0.28], [0.59, 0.40]],
        [[0.59, 0.40], [0.53, 0.47], [0.48, 0.38], [0.55, 0.34]],
    ];
    const INNER: [[Point; 4]; 3] = [
        [[-0.08, 1.28], [0.50, 1.08], [0.19, 0.52], [0.43, 0.38]],
        [[0.43, 0.38], [0.50, 0.33], [0.59, 0.30], [0.57, 0.40]],
        [[0.57, 0.40], [0.53, 0.43], [0.51, 0.38], [0.55, 0.34]],
    ];
    let segment = (u * 3.0).floor().min(2.0) as usize;
    let local = u * 3.0 - segment as f32;
    let mut p = mix(
        cubic(OUTER[segment], local),
        cubic(INNER[segment], local),
        v,
    );
    let breathe = (clock * 0.7).sin() * 0.013;
    p[1] = 1.03 + (p[1] - 1.03) * (swell + breathe);
    p[0] += (clock * 0.43).sin() * 0.012 * (u * std::f32::consts::PI).sin();
    p[1] += (u * 23.0 - clock * 1.1).sin() * v * (1.0 - v) * 0.018;
    p
}

// Two by four samples per terminal cell preserve thin curved lines. Storage
// lasts for one layer; no simulation history is retained between frames.
struct Strokes {
    width: usize,
    height: usize,
    dots: Vec<u8>,
    colors: Vec<Color>,
    light: Vec<f32>,
}

impl Strokes {
    fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            dots: vec![0; width * height],
            colors: vec![Color::Reset; width * height],
            light: vec![0.0; width * height],
        }
    }

    fn point(&mut self, p: Point, color: Color, light: f32) {
        if !(0.0..1.0).contains(&p[0]) || !(0.0..1.0).contains(&p[1]) {
            return;
        }
        let x = (p[0] * (self.width * 2) as f32) as usize;
        let y = (p[1] * (self.height * 4) as f32) as usize;
        let index = (y / 4) * self.width + x / 2;
        self.dots[index] |= DOTS[x % 2][y % 4];
        if light >= self.light[index] {
            self.light[index] = light;
            self.colors[index] = color;
        }
    }

    fn line(&mut self, a: Point, b: Point, color: Color, light: f32) {
        let dx = (a[0] - b[0]).abs() * (self.width * 2) as f32;
        let dy = (a[1] - b[1]).abs() * (self.height * 4) as f32;
        let steps = (dx.max(dy).ceil() as usize).clamp(1, (self.width + self.height) * 4);
        for step in 0..=steps {
            self.point(mix(a, b, step as f32 / steps as f32), color, light);
        }
    }

    fn paint(&self, grid: &mut Grid) {
        for (y, row) in grid.iter_mut().take(self.height).enumerate() {
            for (x, cell) in row.iter_mut().take(self.width).enumerate() {
                let index = y * self.width + x;
                if self.dots[index] != 0 {
                    cell.ch = char::from_u32(0x2800 + self.dots[index] as u32).unwrap();
                    cell.fg = self.colors[index];
                }
            }
        }
    }
}

fn fill_wave(
    grid: &mut Grid,
    width: usize,
    height: usize,
    polygon: &[Point],
    sea: Color,
    ink: Color,
) {
    let mut crossings = Vec::with_capacity(8);
    for (y, row) in grid.iter_mut().take(height).enumerate() {
        let ny = (y as f32 + 0.5) / height as f32;
        crossings.clear();
        for i in 0..polygon.len() {
            let a = polygon[i];
            let b = polygon[(i + 1) % polygon.len()];
            if (a[1] <= ny && b[1] > ny) || (b[1] <= ny && a[1] > ny) {
                crossings.push(a[0] + (b[0] - a[0]) * (ny - a[1]) / (b[1] - a[1]));
            }
        }
        crossings.sort_by(f32::total_cmp);
        for pair in crossings.chunks_exact(2) {
            let start = (pair[0] * width as f32 - 0.5).ceil().max(0.0) as usize;
            let end = (pair[1] * width as f32 - 0.5).ceil().max(0.0) as usize;
            let row_width = width.min(row.len());
            for cell in row.iter_mut().take(end.min(row_width)).skip(start) {
                *cell = Cell::with_bg(' ', ink, lerp_color(sea, ink, 0.08 + ny * 0.07));
            }
        }
    }
}

fn sky_and_water(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    clock: f32,
    params: &MoonwakeParams,
) {
    let night = darken(palette[0], 5);
    let sea = lerp_color(night, palette[1], 0.12);
    let gold = lerp_color(palette[4], rgb(255, 220, 156), 0.45);
    let radius = (height as f32 * 0.17).min(width as f32 * 0.10) * params.moon;
    let phase = hash(seed, 11) * TAU;
    for (y, row) in grid.iter_mut().take(height).enumerate() {
        let ny = (y as f32 + 0.5) / height as f32;
        for (x, cell) in row.iter_mut().take(width).enumerate() {
            let nx = (x as f32 + 0.5) / width as f32;
            let noise = hash(seed ^ 0xD057, y * width + x);
            let dx = (x as f32 + 0.5 - width as f32 * 0.79) * 0.5;
            let dy = y as f32 + 0.5 - height as f32 * 0.25;
            let distance = dx.hypot(dy);
            let halo = if radius > 0.0 {
                (1.0 - distance / (radius * 2.4)).max(0.0).powi(3)
            } else {
                0.0
            };
            let sky = lerp_color(night, palette[2], ny * 0.08 + halo * 0.24);
            *cell = Cell::with_bg(' ', palette[2], sky);

            if ny < 0.61 {
                if noise > 0.9955 && distance > radius * 1.15 {
                    let twinkle = 0.55 + 0.2 * (clock * 0.6 + noise * 913.0).sin();
                    cell.ch = if noise > 0.9994 { '+' } else { '·' };
                    cell.fg = lerp_color(sky, gold, twinkle);
                }
                if radius > 0.0 && distance < radius + 1.0 {
                    let mut dots = 0;
                    for sx in 0..2 {
                        for sy in 0..4 {
                            let mx =
                                (x as f32 + (sx as f32 + 0.5) * 0.5 - width as f32 * 0.79) * 0.5;
                            let my = y as f32 + (sy as f32 + 0.5) * 0.25 - height as f32 * 0.25;
                            if mx.hypot(my) < radius {
                                dots |= DOTS[sx][sy];
                            }
                        }
                    }
                    if dots != 0 {
                        let mx = dx / radius;
                        let my = dy / radius;
                        let mare = ((mx * 4.0 + phase).sin() * (my * 5.0 - mx * 2.0).cos()
                            + (mx * 11.0 + my * 8.0).sin() * 0.18)
                            * 0.5
                            + 0.5;
                        let light = (0.87 - mare.max(0.0) * 0.25 - mx * 0.08).clamp(0.4, 1.0);
                        cell.ch = if dots == 255 {
                            if mare > 0.64 { '▒' } else { '░' }
                        } else {
                            char::from_u32(0x2800 + dots as u32).unwrap()
                        };
                        cell.fg = lerp_color(gold, palette[2], 0.10 + mare.max(0.0) * 0.12);
                        if dots == 255 {
                            cell.bg = lerp_color(sky, gold, 0.49 + light * 0.20);
                        }
                    }
                }
            } else {
                let depth = (ny - 0.61) / 0.39;
                let ripple = (nx * (100.0 - depth * 55.0) + ny * 87.0 - clock * 1.3 + phase).sin();
                let cross = (nx * 39.0 - ny * 71.0 + clock * 0.7).sin();
                let reflection_x = 0.79 + (ny * 45.0 - clock).sin() * (0.004 + depth * 0.022);
                let beam = (1.0 - (nx - reflection_x).abs() / (0.015 + depth * 0.12)).max(0.0);
                let glint = beam * beam * params.reflect * params.moon.min(1.0);
                cell.bg = lerp_color(sea, palette[1], glint * 0.10);
                if ripple + cross * 0.4 > 0.67 {
                    cell.ch = if glint > 0.35 { '━' } else { '─' };
                    cell.fg = lerp_color(lerp_color(night, palette[1], 0.36), gold, glint * 0.68);
                } else if noise > 0.92 {
                    cell.ch = '·';
                    cell.fg = lerp_color(sea, palette[3], 0.20 + glint * 0.45);
                }
            }

            // A distant headland gives the open water a horizon and scale.
            let mountain = 0.61
                - 0.052 * (-((nx - 0.96) * 14.0).powi(2)).exp()
                - 0.018 * (-((nx - 0.88) * 25.0).powi(2)).exp();
            if nx > 0.82 && ny >= mountain && ny < 0.62 {
                cell.ch = '▄';
                cell.fg = lerp_color(night, palette[2], 0.42);
                cell.bg = sky;
            }
        }
    }
}

fn foam(
    strokes: &mut Strokes,
    seed: u64,
    clock: f32,
    swell: f32,
    amount: f32,
    palette: &[Color; 5],
) {
    if amount <= 0.0 {
        return;
    }
    let pearl = lerp_color(palette[3], palette[4], 0.82);
    let cyan = lerp_color(palette[1], palette[3], 0.7);
    let count = ((strokes.width as f32 * 0.36 * amount) as usize).max(3);
    for i in 0..count {
        let u = 0.09 + 0.84 * (i as f32 + hash(seed, i + 31) * 0.5) / count as f32;
        let root = wave(u, 0.0, clock, swell);
        let next = wave((u + 0.002).min(1.0), 0.0, clock, swell);
        let dx = next[0] - root[0];
        let dy = next[1] - root[1];
        let length = dx.hypot(dy).max(0.00001);
        let normal = [dy / length, -dx / length];
        let tangent = [dx / length, dy / length];
        let reach = (0.008 + hash(seed, i + 211) * 0.043) * amount.sqrt();
        let sway = (clock * 1.3 + i as f32 * 2.7).sin() * 0.004;
        let mut previous = root;
        for step in 1..=12 {
            let s = step as f32 / 12.0;
            let curl = s * s * 0.018 + sway * s;
            let p = [
                root[0] + normal[0] * reach * s + tangent[0] * curl,
                root[1] + normal[1] * reach * s + tangent[1] * curl,
            ];
            strokes.line(previous, p, pearl, 1.0);
            if step == 5 || step == 9 {
                let fork = [
                    p[0] + normal[0] * reach * 0.2 - tangent[0] * 0.008,
                    p[1] + normal[1] * reach * 0.2 - tangent[1] * 0.008,
                ];
                strokes.line(p, fork, pearl, 0.95);
            }
            previous = p;
        }
        // Lace-like loops sit just below each finger of the breaking crest.
        let r = 0.004 + hash(seed, i + 411) * 0.005;
        for step in 0..=20 {
            let angle = step as f32 / 20.0 * TAU;
            strokes.point(
                [root[0] + r * angle.cos(), root[1] + r * 1.4 * angle.sin()],
                pearl,
                0.9,
            );
        }
        let age = (clock * 0.16 + hash(seed, i + 711)).rem_euclid(1.0);
        let launch = wave(u.min(0.79), 0.0, clock, swell);
        let spray = [
            launch[0] + age * (0.04 + hash(seed, i + 911) * 0.09),
            launch[1] - age * 0.16 + age * age * 0.24 - reach,
        ];
        strokes.point(
            spray,
            lerp_color(cyan, pearl, (std::f32::consts::PI * age).sin()),
            0.7,
        );
    }
}

fn boat(grid: &mut Grid, width: usize, height: usize, clock: f32, palette: &[Color; 5]) {
    if width < 24 || height < 10 {
        return;
    }
    let x = 0.79 + (clock * 0.17).sin() * 0.018;
    let y = 0.76 + (clock * 1.1).sin() * 0.008;
    let size = 0.043_f32.min(height as f32 / width as f32 * 0.19);
    let sail = lerp_color(palette[4], rgb(255, 220, 156), 0.30);
    let hull = lerp_color(palette[1], palette[4], 0.43);
    let mut lines = Strokes::new(width, height);
    let mast = [x, y - size * 2.3];
    let foot = [x, y];
    lines.line(mast, foot, sail, 1.0);
    let clew = [x + size * 0.83, y - size * 0.35];
    lines.line(mast, clew, sail, 1.0);
    lines.line(clew, [x + 0.003, y - size * 0.38], sail, 1.0);
    for i in 1..5 {
        let v = i as f32 / 5.0;
        lines.line(
            mix(mast, clew, v),
            [x + 0.003, mast[1] + size * 1.95 * v],
            lerp_color(palette[2], sail, 0.38),
            0.4,
        );
    }
    lines.line(
        [x - size * 0.85, y - size * 0.1],
        [x - size * 0.6, y + size * 0.26],
        hull,
        0.8,
    );
    lines.line(
        [x - size * 0.6, y + size * 0.26],
        [x + size * 0.65, y + size * 0.26],
        hull,
        0.8,
    );
    lines.line(
        [x + size * 0.65, y + size * 0.26],
        [x + size, y - size * 0.1],
        hull,
        0.8,
    );
    lines.line(
        [x - size * 0.85, y - size * 0.1],
        [x + size, y - size * 0.1],
        sail,
        0.9,
    );
    lines.paint(grid);
    let bx = ((x - size * 0.3) * width as f32) as usize;
    let by = ((y - size * 0.2) * height as f32) as usize;
    if let Some(cell) = grid.get_mut(by).and_then(|row| row.get_mut(bx)) {
        cell.ch = '▪';
        cell.fg = rgb(255, 198, 100);
        cell.bg = lerp_color(cell.bg, rgb(180, 87, 24), 0.24);
    }
}

fn draw_moonwake(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    _rng: &mut StdRng,
    t: f32,
    params: &MoonwakeParams,
) {
    // Initialize one bounded drawing surface and stable seed-derived phases.
    let height = height.min(grid.len());
    let width = width.min(grid.iter().take(height).map(Vec::len).min().unwrap_or(0));
    if width == 0 || height == 0 {
        return;
    }
    let clock = if t.is_finite() {
        (t as f64 * params.speed as f64).rem_euclid(4096.0 * std::f64::consts::TAU) as f32
    } else {
        0.0
    };
    let phase = hash(seed, 17) * TAU;
    let night = darken(palette[0], 5);
    let sea = lerp_color(night, palette[1], 0.10);
    let cyan = lerp_color(palette[1], palette[3], 0.85);
    let pearl = lerp_color(palette[3], palette[4], 0.82);

    // Paint the night sky, cratered moon, distant headland, and moving reflections.
    measure_layer("moonwake", "sky_water", || {
        sky_and_water(grid, width, height, seed, palette, clock, params);
    });

    // Fill a curling wave silhouette and engrave nested current lines into it.
    let samples = (width + height) * 2;
    measure_layer("moonwake", "wave", || {
        let polygon: Vec<_> = (0..=samples)
            .map(|i| wave(i as f32 / samples as f32, 0.0, clock, params.swell))
            .chain(
                (0..=samples)
                    .rev()
                    .map(|i| wave(i as f32 / samples as f32, 1.0, clock, params.swell)),
            )
            .collect();
        fill_wave(grid, width, height, &polygon, sea, palette[1]);
        let mut lines = Strokes::new(width, height);
        let count = params.lines.min(height * 2);
        for j in 0..=count {
            let v = j as f32 / count as f32;
            let mut previous = wave(0.0, v, clock, params.swell);
            for i in 1..=samples {
                let u = i as f32 / samples as f32;
                let p = wave(u, v, clock, params.swell);
                let pulse = (u * 37.0 - clock * 1.9 + phase + j as f32 * 0.65).sin();
                let light = if j == 0 {
                    0.95
                } else {
                    (0.26 + (1.0 - v) * 0.25 + pulse * 0.11 + if j % 4 == 0 { 0.22 } else { 0.0 })
                        .clamp(0.0, 0.9)
                };
                let color = if j == 0 {
                    pearl
                } else {
                    lerp_color(palette[1], cyan, light)
                };
                lines.line(previous, p, color, light);
                previous = p;
            }
        }
        // Low foreground swells continue the same current across the bottom edge.
        for j in 0..7 {
            let mut previous = [0.0, 0.0];
            for i in 0..=width * 2 {
                let nx = i as f32 / (width * 2) as f32;
                let ny = 0.91 + j as f32 * 0.026
                    - 0.105
                        * (nx * 8.0 + j as f32 * 0.36 - clock * 0.32 + phase).sin()
                        * (0.25 + nx * 0.75);
                let p = [nx, ny];
                if i > 0 {
                    lines.line(
                        previous,
                        p,
                        lerp_color(palette[1], cyan, 0.30 + j as f32 * 0.045),
                        0.4,
                    );
                }
                previous = p;
            }
        }
        lines.paint(grid);
    });

    // Branch foam from the crest, evaluate airborne spray by age, and add the boat.
    measure_layer("moonwake", "foam_boat", || {
        let mut spray = Strokes::new(width, height);
        foam(&mut spray, seed, clock, params.swell, params.foam, palette);
        spray.paint(grid);
        if params.boat {
            boat(grid, width, height, clock, palette);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::named_theme;
    use crate::render::grid_to_plain;
    use rand::SeedableRng;

    fn defaults() -> Vec<f32> {
        PARAMS.iter().map(|p| p.default).collect()
    }

    fn frame(width: usize, height: usize, seed: u64, t: f32, values: &[f32]) -> Grid {
        let mut grid = vec![vec![Cell::blank(); width]; height];
        let palette = named_theme("deep").unwrap();
        let mut rng = StdRng::seed_from_u64(seed);
        MODE.render(&mut ModeFrame {
            grid: &mut grid,
            width,
            height,
            seed,
            palette: &palette,
            rng: &mut rng,
            time: t,
            args: &[],
            param_values: Some(values),
        });
        grid
    }

    #[test]
    fn snapshot_moonwake_seed_42() {
        insta::assert_snapshot!(grid_to_plain(&frame(80, 24, 42, 0.0, &defaults())).join("\n"));
    }

    #[test]
    fn snapshot_moonwake_in_motion() {
        insta::assert_snapshot!(grid_to_plain(&frame(100, 32, 42, 4.75, &defaults())).join("\n"));
    }

    #[test]
    fn frames_are_reproducible_seed_sensitive_and_animate() {
        let values = defaults();
        let start = frame(96, 32, 42, 1.25, &values);
        let moved = frame(96, 32, 42, 5.75, &values);
        assert_eq!(start, frame(96, 32, 42, 1.25, &values));
        assert_ne!(grid_to_plain(&start), grid_to_plain(&moved));
        assert_ne!(start, frame(96, 32, 77, 1.25, &values));
        let mut stopped = values;
        stopped[6] = 0.0;
        assert_eq!(
            frame(80, 24, 42, 0.0, &stopped),
            frame(80, 24, 42, 50.0, &stopped)
        );
    }

    #[test]
    fn native_playback_matches_direct_frames_and_live_controls() {
        use crate::morph::IterateFrameRenderer;
        let mut player = IterateFrameRenderer::new("moonwake", 42, "deep", 80, 24).unwrap();
        let mut values = defaults();
        for t in [0.0, 4.75, 0.0] {
            assert_eq!(
                player.render(t, Some(&values)).unwrap(),
                &frame(80, 24, 42, t, &values)
            );
        }
        values[0] = 0.7;
        values[3] = 0.0;
        assert_eq!(
            player.render(2.0, Some(&values)).unwrap(),
            &frame(80, 24, 42, 2.0, &values)
        );
    }

    #[test]
    fn controls_change_the_visible_composition() {
        let values = defaults();
        let baseline = grid_to_plain(&frame(100, 36, 42, 2.0, &values));
        for (i, p) in PARAMS.iter().enumerate() {
            let mut changed = values.clone();
            changed[i] = p.min;
            assert_ne!(
                baseline,
                grid_to_plain(&frame(100, 36, 42, 2.0, &changed)),
                "{}",
                p.key
            );
        }
    }

    #[test]
    fn parameter_precedence_limits_and_nonfinite_inputs() {
        let defaults = defaults();
        let params = MoonwakeParams::from_inputs(&[], Some(&defaults));
        let args = [
            "ascii-renderer",
            "42",
            "moonwake",
            "deep",
            "0.7",
            "999",
            "-3",
            "NaN",
            "inf",
        ]
        .map(str::to_string);
        let values = [1.2, 8.0, 1.5, 0.0, 0.0, 0.0, 99.0];
        let parsed = MoonwakeParams::from_inputs(&args, Some(&values));
        assert_eq!(
            parsed,
            MoonwakeParams {
                swell: 0.7,
                lines: 32,
                foam: 0.0,
                moon: params.moon,
                reflect: params.reflect,
                boat: false,
                speed: 2.0,
            }
        );
    }

    #[test]
    fn small_grids_extreme_parameters_and_display_width() {
        use crate::types::display_width;
        let low: Vec<_> = PARAMS.iter().map(|p| p.min).collect();
        let high: Vec<_> = PARAMS.iter().map(|p| p.max).collect();
        for values in [&low, &high] {
            for (width, height) in [
                (0, 0),
                (0, 4),
                (4, 0),
                (1, 1),
                (2, 9),
                (9, 2),
                (17, 7),
                (80, 24),
            ] {
                for t in [-5.0, 50_000.0, f32::MAX, f32::NAN] {
                    let grid = frame(width, height, u64::MAX, t, values);
                    assert_eq!(grid.len(), height);
                    assert!(grid.iter().all(|row| row.len() == width));
                    assert!(
                        grid_to_plain(&grid)
                            .iter()
                            .all(|row| display_width(row) == width)
                    );
                }
            }
        }
    }
}
