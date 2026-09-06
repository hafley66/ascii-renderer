//! Chaos observatory: paired ODE trajectories and a logistic-map instrument.
//! Complete orbit history is visible at every time; time moves its light and camera.

use crate::color::{lerp_color, shift_hue};
use crate::opts::param_f32;
use crate::registry::{AnimKind, Mode, ModeFrame, Param};
use crate::types::{Cell, Grid};
use crossterm::style::Color;

pub(super) struct ChaosTheoryMode;
pub(super) static MODE: ChaosTheoryMode = ChaosTheoryMode;

const PARAMS: &[Param] = &[
    param!(
        "SYS_FAMILY",
        "sys / Lorenz Rossler Thomas",
        0.0,
        2.0,
        0.0,
        1.0
    ),
    param!("SYS_ORBITS", "sys / orbit pairs", 1.0, 12.0, 2.0, 1.0),
    param!(
        "SYS_STEP",
        "sys / integration dt",
        0.002,
        0.035,
        0.012,
        0.001
    ),
    param!(
        "SYS_TRAIL",
        "sys / trail samples",
        16.0,
        1200.0,
        280.0,
        16.0
    ),
    param!(
        "SYS_DIVERGENCE",
        "sys / initial separation",
        0.00001,
        0.3,
        0.012,
        0.005
    ),
    param!("SYS_COUPLING", "sys / pair coupling", 0.0, 2.0, 0.0, 0.05),
    param!("SYS_DAMPING", "sys / added damping", 0.0, 0.6, 0.025, 0.025),
    param!("SYS_VARIATION", "sys / seed variation", 0.0, 2.0, 0.7, 0.1),
    param!("TIME_SPEED", "time / speed", 0.0, 4.0, 1.0, 0.1),
    param!(
        "TIME_PRECESSION",
        "time / camera precession",
        0.0,
        1.0,
        0.08,
        0.02
    ),
    param!("VIEW_ZOOM", "view / zoom", 0.35, 2.5, 1.0, 0.05),
    param!(
        "VIEW_ROTATION",
        "view / rotation degrees",
        -180.0,
        180.0,
        0.0,
        5.0
    ),
    param!(
        "VIEW_PROJECTION",
        "view / XZ through XY",
        0.0,
        1.0,
        0.12,
        0.05
    ),
    param!(
        "VIEW_DEPTH",
        "view / perspective depth",
        0.0,
        1.0,
        0.55,
        0.05
    ),
    param!(
        "MAP_THRESHOLD",
        "map / bifurcation center r",
        2.8,
        3.95,
        3.57,
        0.01
    ),
    param!("MAP_SPAN", "map / bifurcation r span", 0.1, 1.2, 0.85, 0.05),
    param!("INK_DENSITY", "ink / trail density", 0.05, 1.0, 0.72, 0.05),
    param!("INK_GLYPHS", "ink / fine wire phosphor", 0.0, 2.0, 1.0, 1.0),
    param!("INK_HUE", "ink / hue degrees", -180.0, 180.0, 0.0, 5.0),
    param!("INK_LIGHT", "ink / luminance", 0.1, 1.5, 1.0, 0.05),
    param!("INK_GRAIN", "ink / background grain", 0.0, 1.0, 0.06, 0.05),
    param!("VIEW_GRID", "view / graticule", 0.0, 1.0, 0.55, 0.05),
    param!(
        "VIEW_GHOSTS",
        "view / floor projection",
        0.0,
        1.0,
        0.4,
        0.05
    ),
    param!("INK_HEADS", "ink / moving head size", 0.0, 3.0, 1.0, 1.0),
];

impl Mode for ChaosTheoryMode {
    fn name(&self) -> &'static str {
        "astra-chaos-theory"
    }
    fn help(&self) -> &'static str {
        "Chaos observatory: Lorenz/Rossler/Thomas pairs, logistic bifurcation, log separation. Knobs: sys, time, view, map, ink; positional values follow the knob order."
    }
    fn animation(&self) -> AnimKind {
        AnimKind::Iterate
    }
    fn params(&self) -> &'static [Param] {
        PARAMS
    }
    fn render(&self, frame: &mut ModeFrame<'_>) {
        // Read explicit live values, then CLI slots, then thread-local/env fallback.
        let knobs = std::array::from_fn(|i| {
            let p = &PARAMS[i];
            let v = frame
                .param_values
                .and_then(|v| v.get(i).copied())
                .or_else(|| frame.args.get(i + 4).and_then(|v| v.parse().ok()))
                .unwrap_or_else(|| param_f32(p.key, p.default));
            if v.is_finite() {
                v.clamp(p.min, p.max)
            } else {
                p.default
            }
        });
        draw_chaos(frame, &knobs);
    }
}

fn random(seed: u64, lane: usize) -> f32 {
    let mut n = seed.wrapping_add((lane as u64).wrapping_mul(0x9e3779b97f4a7c15));
    n = (n ^ (n >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    n = (n ^ (n >> 27)).wrapping_mul(0x94d049bb133111eb);
    ((n ^ (n >> 31)) >> 40) as f32 / 16_777_216.0
}

type Point = [f32; 3];

// Each pair advances simultaneously. The extra damping and coupling are explicit
// modifications of the named ODEs; the inset logistic map is a separate system.
fn velocity(p: Point, q: Point, family: usize, damping: f32, coupling: f32) -> Point {
    let [x, y, z] = p;
    let v = match family {
        0 => [10.0 * (y - x), x * (28.0 - z) - y, x * y - 8.0 / 3.0 * z],
        1 => [-y - z, x + 0.2 * y, 0.2 + z * (x - 5.7)],
        _ => [
            y.sin() - 0.208186 * x,
            z.sin() - 0.208186 * y,
            x.sin() - 0.208186 * z,
        ],
    };
    std::array::from_fn(|i| v[i] - damping * p[i] + coupling * (q[i] - p[i]))
}

fn advance(pair: [Point; 2], dt: f32, family: usize, damping: f32, coupling: f32) -> [Point; 2] {
    // Explicit midpoint RK2; all coordinates remain finite within the safety box.
    let midpoint: [Point; 2] = std::array::from_fn(|j| {
        let v = velocity(pair[j], pair[1 - j], family, damping, coupling);
        std::array::from_fn(|i| (pair[j][i] + 0.5 * dt * v[i]).clamp(-80.0, 80.0))
    });
    std::array::from_fn(|j| {
        let v = velocity(midpoint[j], midpoint[1 - j], family, damping, coupling);
        std::array::from_fn(|i| (pair[j][i] + dt * v[i]).clamp(-80.0, 80.0))
    })
}

#[derive(Clone, Copy)]
struct Panel {
    x: usize,
    y: usize,
    w: usize,
    h: usize,
}

struct Canvas<'a> {
    grid: &'a mut Grid,
    width: usize,
    height: usize,
    z: Vec<f32>,
}

impl Canvas<'_> {
    fn line(&mut self, panel: Panel, a: Point, b: Point, ch: char, fg: Color, z: f32) {
        // Clip before rasterizing so offscreen trajectories cannot consume work.
        let dx = b[0] - a[0];
        let dy = b[1] - a[1];
        let mut enter = 0.0_f32;
        let mut leave = 1.0_f32;
        for (p, q) in [
            (-dx, a[0]),
            (dx, panel.w.saturating_sub(1) as f32 - a[0]),
            (-dy, a[1]),
            (dy, panel.h.saturating_sub(1) as f32 - a[1]),
        ] {
            if p == 0.0 {
                if q < 0.0 {
                    return;
                }
            } else if p < 0.0 {
                enter = enter.max(q / p);
            } else {
                leave = leave.min(q / p);
            }
        }
        if enter > leave {
            return;
        }
        let steps = ((dx.abs().max(dy.abs()) * (leave - enter)).ceil() as usize)
            .min(panel.w.max(panel.h))
            .max(1);
        let ch = if ch == '\0' {
            if dx.abs() > dy.abs() * 2.0 {
                '-'
            } else if dy.abs() > dx.abs() * 2.0 {
                '|'
            } else if dx * dy > 0.0 {
                '\\'
            } else {
                '/'
            }
        } else {
            ch
        };
        for i in 0..=steps {
            let t = enter + (leave - enter) * i as f32 / steps as f32;
            self.point(panel, a[0] + dx * t, a[1] + dy * t, ch, fg, z);
        }
    }

    fn put(&mut self, x: usize, y: usize, ch: char, fg: Color, z: f32) {
        if x < self.width && y < self.height {
            let index = y * self.width + x;
            if z >= self.z[index] {
                self.grid[y][x].ch = ch;
                self.grid[y][x].fg = fg;
                self.z[index] = z;
            }
        }
    }

    fn point(&mut self, panel: Panel, x: f32, y: f32, ch: char, fg: Color, z: f32) {
        if x.is_finite()
            && y.is_finite()
            && x >= 0.0
            && y >= 0.0
            && x.round() < panel.w as f32
            && y.round() < panel.h as f32
        {
            self.put(
                panel.x + x.round() as usize,
                panel.y + y.round() as usize,
                ch,
                fg,
                z,
            );
        }
    }

    fn text(&mut self, x: usize, y: usize, text: &str, fg: Color) {
        for (i, ch) in text.chars().take(self.width.saturating_sub(x)).enumerate() {
            self.put(x + i, y, ch, fg, 100.0);
        }
    }

    fn border(&mut self, p: Panel, title: &str, fg: Color) {
        for x in p.x..p.x + p.w {
            self.put(x, p.y, '-', fg, 90.0);
            self.put(x, p.y + p.h - 1, '-', fg, 90.0);
        }
        for y in p.y..p.y + p.h {
            self.put(p.x, y, '|', fg, 90.0);
            self.put(p.x + p.w - 1, y, '|', fg, 90.0);
        }
        for x in [p.x, p.x + p.w - 1] {
            for y in [p.y, p.y + p.h - 1] {
                self.put(x, y, '+', fg, 91.0);
            }
        }
        self.text(
            p.x + 2,
            p.y,
            &title
                .chars()
                .take(p.w.saturating_sub(4))
                .collect::<String>(),
            fg,
        );
    }
}

fn project(p: Point, family: usize, angle: f32, tilt: f32, depth: f32) -> Point {
    let [x, y, z] = match family {
        0 => [p[0] / 22.0, p[1] / 28.0, (p[2] - 25.0) / 26.0],
        1 => [p[0] / 12.0, p[2] / 20.0 - 0.35, p[1] / 12.0],
        _ => [p[0] / 4.5, p[1] / 4.5, p[2] / 4.5],
    };
    let (s, c) = angle.sin_cos();
    let (st, ct) = (tilt * std::f32::consts::FRAC_PI_2).sin_cos();
    let horizontal = x * c - y * s;
    let into = x * s + y * c;
    let vertical = z * ct + into * st;
    let distance = (into * ct - z * st).clamp(-1.5, 1.5);
    let perspective = 1.0 / (1.0 + distance * depth * 0.22);
    [horizontal * perspective, vertical * perspective, distance]
}

fn draw_chaos(frame: &mut ModeFrame<'_>, knobs: &[f32; 24]) {
    // Initialize bounded frame storage, then draw the rear graticule and orbit cage.
    let &[
        family,
        orbits,
        step,
        trail,
        divergence,
        coupling,
        damping,
        variation,
        speed,
        precession,
        zoom,
        rotation,
        projection,
        depth,
        threshold,
        span,
        density,
        glyphs,
        hue,
        light,
        grain,
        graticule,
        ghosts,
        heads,
    ] = knobs;
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
    let area = width.saturating_mul(height);
    let family = family.round() as usize;
    let time = if frame.time.is_finite() {
        frame.time.rem_euclid(65536.0) * speed
    } else {
        0.0
    };
    let palette = frame.palette.map(|c| shift_hue(c, hue as f64));
    let ink = |index: usize, strength: f32| {
        lerp_color(
            palette[0],
            palette[index],
            (strength * light).clamp(0.0, 1.0),
        )
    };
    let mut canvas = Canvas {
        grid: frame.grid,
        width,
        height,
        z: vec![f32::NEG_INFINITY; area],
    };
    for y in 0..height {
        for x in 0..width {
            canvas.grid[y][x] = Cell::with_bg(' ', palette[4], palette[0]);
            if random(frame.seed, y * width + x) < grain * 0.13 {
                canvas.put(x, y, '.', ink(2, 0.22), -10.0);
            }
        }
    }
    let split = width >= 48 && height >= 16;
    let main_width = if split { width * 7 / 10 } else { width };
    let framed = main_width >= 8 && height >= 6;
    let plot = if framed {
        Panel {
            x: 2,
            y: 2,
            w: main_width - 4,
            h: height - 5,
        }
    } else {
        Panel {
            x: 0,
            y: 0,
            w: width,
            h: height,
        }
    };
    if graticule > 0.0 {
        for y in 0..plot.h {
            for x in 0..plot.w {
                let cross = (x == plot.w / 2 && y % 2 == 0) || (y == plot.h / 2 && x % 4 == 0);
                if cross || (x % 8 == 0 && y % 4 == 0) {
                    canvas.point(
                        plot,
                        x as f32,
                        y as f32,
                        if cross { '+' } else { '.' },
                        ink(2, graticule * 0.32),
                        -8.0,
                    );
                }
            }
        }
        for ring in 0..3 {
            let radius = 0.35 + ring as f32 * 0.055;
            let segments = (plot.w + plot.h).clamp(24, 256);
            let mut previous = None;
            for i in 0..=segments {
                let phase = i as f32 / segments as f32 * std::f32::consts::TAU;
                let (s, c) = phase.sin_cos();
                let p = [
                    (plot.w - 1) as f32 * (0.5 + radius * c),
                    (plot.h - 1) as f32 * (0.79 + radius * s * 0.28),
                    0.0,
                ];
                if let Some(a) = previous {
                    canvas.line(
                        plot,
                        a,
                        p,
                        if ring == 1 { '-' } else { '.' },
                        ink(2, graticule * 0.4),
                        -7.0,
                    );
                }
                previous = Some(p);
            }
        }
    }

    // Reconstruct complete paths before evaluating moving light. No work grows
    // with ASCII_T: <=12 pairs, <=2048 samples each, <=6 RK2 substeps/sample.
    // One pair history lives for one orbit; the depth buffer lives for one frame.
    let count = (orbits.round() as usize).min((area / 80).max(1));
    let steps = (area.saturating_mul(12) / count).clamp(1, 2048);
    // Thomas can settle under added damping; retain its approach orbits too.
    let burn = if family == 2 { 0 } else { (steps / 8).min(128) };
    let tail = (trail.round() as usize).min((steps - burn).max(1));
    let travel = steps.saturating_sub(burn + tail);
    let phase = time * 0.37 + random(frame.seed, 9000) * 2.0;
    let head =
        (burn + tail - 1 + ((phase.sin() * 0.5 + 0.5) * travel as f32) as usize).min(steps - 1);
    let start = (head + 1).saturating_sub(tail);
    let angle = rotation.to_radians() + time * precession;
    let glyph_sets = [b"..,:oO@", b".,-~=*#", b".:oxO#@"];
    let glyph_set = glyph_sets[glyphs.round() as usize];
    let mut separation = Vec::new();
    for orbit in 0..count {
        let jitter: Point =
            std::array::from_fn(|i| (random(frame.seed, orbit * 7 + i + 1) - 0.5) * variation);
        let base = match family {
            0 => [
                1.0 + jitter[0] * 5.0,
                1.0 + jitter[1] * 5.0,
                18.0 + jitter[2] * 12.0,
            ],
            1 => [
                3.0 + jitter[0] * 4.0,
                jitter[1] * 4.0,
                0.2 + jitter[2].abs(),
            ],
            _ => {
                let side = if orbit % 2 == 0 { 1.0 } else { -1.0 };
                [
                    side * (2.8 + jitter[0]),
                    side * (-0.6 + jitter[1]),
                    side * (-1.8 + jitter[2]),
                ]
            }
        };
        let mut pair = [base, [base[0] + divergence, base[1], base[2]]];
        let mut history = Vec::with_capacity(steps);
        // Family scaling allows slow Thomas trajectories to fill the instrument.
        let dt = step * [1.0, 2.5, 12.0][family];
        // Small dt refines integration without shortening the visible history.
        let substeps = (0.012 / step).ceil().clamp(1.0, 6.0) as usize;
        for _ in 0..steps {
            for _ in 0..substeps {
                pair = advance(pair, dt, family, damping, coupling);
            }
            history.push(pair);
        }
        let screen = |p| {
            let v = project(p, family, angle, projection, depth);
            [
                (plot.w - 1) as f32 * (0.5 + v[0] * zoom * 0.46),
                (plot.h - 1) as f32 * (0.46 - v[1] * zoom * 0.46),
                v[2],
            ]
        };
        // Rear paths preserve the whole attractor even for a sixteen-sample
        // highlight. Connected strokes and periodic dashes replace point thinning.
        let mut previous = None;
        for (i, pair) in history.iter().enumerate().skip(burn) {
            let projected = pair.map(screen);
            if let Some(prev) = previous {
                let prev: [Point; 2] = prev;
                for j in 0..2 {
                    let v = projected[j];
                    let band = (i / 24 + orbit * 3 + j * 2) % 8;
                    if band as f32 <= 2.0 + density * 5.0 {
                        canvas.line(
                            plot,
                            prev[j],
                            v,
                            if glyphs < 0.5 { ':' } else { '\0' },
                            ink(
                                if j == 0 { 1 } else { 3 },
                                0.28 + density * 0.16 - v[2] * depth * 0.06,
                            ),
                            -1.0 - v[2] * depth,
                        );
                    }
                    if ghosts > 0.0 {
                        let floor = |p: Point| {
                            [
                                p[0],
                                (plot.h - 1) as f32 * (0.83 + p[2] * 0.055 * depth),
                                0.0,
                            ]
                        };
                        canvas.line(
                            plot,
                            floor(prev[j]),
                            floor(v),
                            if ghosts > 0.65 { ':' } else { '.' },
                            ink(2, ghosts * 0.45),
                            -3.0,
                        );
                    }
                }
            }
            previous = Some(projected);
        }
        // Overlay a complete bright window at every t, with depth-ordered pair
        // colors, a luminous crest and moving heads. Never join a history wrap.
        for (i, pair) in history.iter().enumerate().take(head + 1).skip(start) {
            let age = (i - start + 1) as f32 / tail as f32;
            if orbit == 0 {
                let distance = (0..3)
                    .map(|j| (pair[0][j] - pair[1][j]).powi(2))
                    .sum::<f32>()
                    .sqrt();
                // Fixed scale: log10 separation from 1e-6 through 1e2.
                separation.push(((distance.max(1e-6).log10() + 6.0) / 8.0).clamp(0.0, 1.0));
            }
            for (j, &p) in pair.iter().enumerate() {
                let v = screen(p);
                let [x, y, _] = v;
                if (i / 6 + orbit + j) % 10 <= (density * 9.0) as usize {
                    let strength = (age * 0.48 + 0.48 - v[2] * depth * 0.13).clamp(0.1, 1.0);
                    let ch = glyph_set[(strength * 6.0) as usize] as char;
                    canvas.line(
                        plot,
                        screen(history[i.saturating_sub(1).max(start)][j]),
                        v,
                        ch,
                        ink(if j == 0 { 1 } else { 3 }, strength),
                        2.0 + age - v[2] * depth,
                    );
                }
                if i == head && heads >= 1.0 {
                    canvas.point(
                        plot,
                        x,
                        y,
                        if j == 0 { '@' } else { 'o' },
                        ink(if j == 0 { 4 } else { 3 }, 1.0),
                        20.0,
                    );
                    for d in 1..heads.round() as usize {
                        for side in [-1.0, 1.0] {
                            canvas.point(plot, x + d as f32 * side, y, '-', ink(4, 0.7), 19.0);
                            canvas.point(plot, x, y + d as f32 * side, '|', ink(4, 0.7), 19.0);
                        }
                    }
                }
            }
        }
    }

    // Draw measured separation and the independent logistic bifurcation map.
    if split {
        let right = Panel {
            x: main_width,
            y: 0,
            w: width - main_width,
            h: height / 2 + 1,
        };
        let map = Panel {
            x: right.x + 2,
            y: 2,
            w: right.w - 4,
            h: right.h - 4,
        };
        let scan = ((time * 0.21).sin() * 0.5 + 0.5) * (map.w - 1) as f32;
        let map_iterations = area.min(160);
        for x in 0..map.w {
            let r =
                (threshold + span * (x as f32 / (map.w - 1).max(1) as f32 - 0.5)).clamp(0.0, 4.0);
            let mut value = 0.43 + variation * (random(frame.seed, x + 22000) - 0.5) * 0.1;
            for n in 0..map_iterations {
                value = (r * value * (1.0 - value)).clamp(0.0, 1.0);
                if n >= map_iterations / 2 {
                    canvas.point(
                        map,
                        x as f32,
                        (1.0 - value) * (map.h - 1) as f32,
                        if x == scan.round() as usize { '*' } else { ':' },
                        ink(3, 0.8),
                        4.0,
                    );
                }
            }
        }
        canvas.border(right, " LOGISTIC / r ", ink(2, 0.8));
        let range = format!(
            "{:.2} < r < {:.2}",
            (threshold - span / 2.0).max(0.0),
            (threshold + span / 2.0).min(4.0)
        );
        canvas.text(
            map.x,
            right.h - 2,
            &range.chars().take(map.w).collect::<String>(),
            ink(4, 0.65),
        );
        let lower = Panel {
            x: main_width,
            y: right.h - 1,
            w: right.w,
            h: height - right.h + 1,
        };
        let scope = Panel {
            x: lower.x + 2,
            y: lower.y + 2,
            w: lower.w - 4,
            h: lower.h - 4,
        };
        for x in 0..scope.w {
            let index = x * separation.len().saturating_sub(1) / (scope.w - 1).max(1);
            let value = separation.get(index).copied().unwrap_or(0.0);
            let y = (1.0 - value) * (scope.h - 1) as f32;
            canvas.point(
                scope,
                x as f32,
                y,
                if x + 1 == scope.w { '@' } else { '*' },
                ink(1, 0.95),
                6.0,
            );
            canvas.point(
                scope,
                x as f32,
                (scope.h - 1) as f32,
                '.',
                ink(2, 0.35),
                -2.0,
            );
        }
        canvas.border(lower, " LOG10 |A-B| ", ink(2, 0.8));
        canvas.text(
            scope.x,
            height - 2,
            &"-6 ... +2 / time >"
                .chars()
                .take(scope.w)
                .collect::<String>(),
            ink(4, 0.65),
        );
    }
    if framed {
        let name = ["LORENZ", "ROSSLER", "THOMAS"][family];
        canvas.border(
            Panel {
                x: 0,
                y: 0,
                w: main_width,
                h: height,
            },
            &format!(" ASTRA / {name} "),
            ink(2, 0.8),
        );
        canvas.text(2, 1, "PHASE SPACE   A:@ B:o", ink(4, 0.7));
        let status = format!("dt:{step:.3}  pairs:{count}  eps:{divergence:.5}");
        canvas.text(
            2,
            height - 2,
            &status.chars().take(main_width - 4).collect::<String>(),
            ink(4, 0.65),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{SeedableRng, rngs::StdRng};

    fn defaults() -> [f32; 24] {
        std::array::from_fn(|i| PARAMS[i].default)
    }

    fn render(width: usize, height: usize, seed: u64, time: f32, values: &[f32]) -> Grid {
        let mut grid = vec![vec![Cell::blank(); width]; height];
        let mut rng = StdRng::seed_from_u64(seed);
        MODE.render(&mut ModeFrame {
            grid: &mut grid,
            width,
            height,
            seed,
            palette: &crate::color::make_palette(seed),
            rng: &mut rng,
            time,
            args: &[],
            param_values: Some(values),
        });
        grid
    }

    fn plain(grid: &Grid) -> String {
        grid.iter()
            .map(|r| r.iter().map(|c| c.ch).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn chaos_time_zero() {
        insta::assert_snapshot!(plain(&render(80, 24, 1701, 0.0, &defaults())), @r"
+- ASTRA / LORENZ -------------------------------------++- LOGISTIC / r -------+
| PHASE SPACE   A:@ B:o                                ||                      |
| .       .       .       . +     .       .       .    ||               :::::: |
|                                                      ||    :::::::*::::::::: |
|     //-----**             +             --*-         || :::       *::::::::: |
|     ||/-  ---***                      ******         ||           *::::::::: |
| . .||| |************    . +     . -***===-*     .    || ::::::::::*::::::::: |
|    ||| **=~~~~=****=****        ***=***-=|*          ||        :::*::::::::: |
|     || =*|~||/-~~==**=-***+  -*****@*=*=**           ||            ::::::::: |
|     \\ \*=~\\   --~~=***--****=***~***=*/            ||                ::::: |
| . .. \\ ===~\\  .  \~~==****=********/*/.       .    ||         .         :: |
| +   +\\\+=\=~\\-+.  |-~~***==/******/*/ +   +   +    || 3.14 < r < 3.99      |
|        \\ \==~\-------~~~=**=|\o---**                |+- LOG10 |A-B| --------+
|         \\\\\=~~~----o~~*==**=-=--**                 ||                      |
| .       .\\\\\-==~~~~~~**/=\*=@***........      .    ||                 *    |
|     ......-\\--.-==~~~*~/=|\-**-.....-----.......    || * ************** **@ |
|   ........   ------    --=+               .. -- ..   ||  *                   |
|   .............---*****===........................   ||                      |
| .     ................--------................  .    ||                      |
|                   .   ........                       ||                      |
|                           +                          ||                      |
|                                      .               || .................... |
| dt:0.012  pairs:2  eps:0.01200                       || -6 ... +2 / time >   |
+------------------------------------------------------++----------------------+
");
    }

    #[test]
    fn chaos_time_four() {
        insta::assert_snapshot!(plain(&render(80, 24, 1701, 4.0, &defaults())), @r"
+- ASTRA / LORENZ -------------------------------------++- LOGISTIC / r -------+
| PHASE SPACE   A:@ B:o                                ||                      |
| .       .       .       . +     .       .       .    ||               :::*:: |
|                                                      ||    ::::::::::::::*:: |
|          |--              +           *=             || :::       :::::::*:: |
|          \|~~~-                      **              ||           :::::::*:: |
| . .     .\\~\~~~~--     . +     .  **=  .       .    || :::::::::::::::::*:: |
|           \~\\|/-~~~--          ****=                ||        ::::::::::*:: |
|            \~\\****~~~--- +   *****=                 ||            ::::::*:: |
|             \~\\\ ***~~-----****=*                   ||                ::*:: |
| . ..    .    \~\\\  \**~~/****~=**      .       .    ||         .         :: |
| +   +   +   + \~\\\ +\****@*=~=*/   +   +   +   +    || 3.14 < r < 3.99      |
|               \~~\\\---**~**==**                     |+- LOG10 |A-B| --------+
|                 ~~~\\-*o/~*===*                      ||                      |
| .       . ......\\~~\---~**@-*............      .    ||    **       *   *    |
|     ......-----...\~~~~~~*==- .......-----.......    || ***  ******* *** **@ |
|   .. -- ...........\\~~~~*+               .. -- ..   ||                      |
|   .....--...........\---o*................---.....   ||                      |
| .     ................--------................  .    ||                      |
|                   .   ........                       ||                      |
|                           +                          ||                      |
|                                      .               || .................... |
| dt:0.012  pairs:2  eps:0.01200                       || -6 ... +2 / time >   |
+------------------------------------------------------++----------------------+
");
    }

    #[test]
    fn chaos_rossler_first_frame() {
        let mut values = defaults();
        for (i, value) in [
            (0, 1.0),
            (1, 4.0),
            (2, 0.018),
            (3, 160.0),
            (11, 25.0),
            (12, 0.35),
            (17, 0.0),
            (22, 0.75),
        ] {
            values[i] = value;
        }
        insta::assert_snapshot!(plain(&render(80, 24, 42, 0.0, &values)), @r"
+- ASTRA / ROSSLER ------------------------------------++- LOGISTIC / r -------+
| PHASE SPACE   A:@ B:o                                ||                      |
| .       . .     .       . +     .       .       .    ||               :::::: |
|                                                      ||    :::::::*::::::::: |
|                           +                          || :::       *::::::::  |
|                                                      ||           *::::::::: |
| .       .      ..       . +     .       .       .    || ::::::::::*::::::::: |
|    .                    OOOOOOOOOOOOOOO              ||        :::*::::::::: |
|  .                 @@@OOO:OOOO::::::::OOOO           ||            ::::::::: |
|                  ::o:OOOOOOOO:::::::::::OOOoo        ||                ::::: |
| .      ..      ::::@oOO:::::::::::::::::::Oooo  .    ||                   :: |
| +   +   +   + :::::@oO :::  +   + : +:::::oo:oo +    || 3.14 < r < 3.99      |
|              ::::::::  :  +       : ::::::o::oo      |+- LOG10 |A-B| --------+
|              ::::::::  ::        :::::::oo:ooo       ||                      |
| .       . ...::::::::::.:::::::::::::oooooooo   .    ||                      |
|     ......----::oo:::ooo:::::::::oooooooooo......    ||                      |
|   .. -- ..      ooooooooo:::::oooooooo:o  .. -- ..   ||               *      |
|   .....---...:::::::ooooooooooooo::::::::::::::....  || ************** ****@ |
| .     ................--------................  .    ||                      |
|             .         ........                       ||                      |
|                           +                          ||                      |
|                                                      || .................... |
| dt:0.018  pairs:4  eps:0.01200                       || -6 ... +2 / time >   |
+------------------------------------------------------++----------------------+
");
    }

    #[test]
    fn chaos_thomas_first_frame() {
        let mut values = defaults();
        for (i, value) in [
            (0, 2.0),
            (1, 3.0),
            (3, 600.0),
            (6, 0.0),
            (11, -30.0),
            (12, 0.65),
            (16, 0.9),
            (17, 2.0),
        ] {
            values[i] = value;
        }
        insta::assert_snapshot!(plain(&render(80, 24, 7, 0.0, &values)), @r"
+- ASTRA / THOMAS -------------------------------------++- LOGISTIC / r -------+
| PHASE SPACE   A:@ B:o                                ||                      |
| .       .       .       . +     .       .       .  . ||               :::::: |
|                                  -#####              ||    :::::::*::::::::: |
|                           +   OOOO--OOxOOOO          || :::       *::::: ::: |
|                  --          OO      OOO\\###        ||           *::::::::: |
| .       . OOOOOO.  -----. + OO  .      OOO##O#x .    || ::::::::::*::::::::: |
|       OoOOO-----OOOOO  -----||      OOOxO###o#OO     ||        :::*::::::::: |
|      O##           -OOOOOOOOOOOOOOOOO#####-\#@###    ||            ::::::::: |
|      ##            . \OO     #---####-OO /--------   ||                ::::: |
| .   O#  .       .  OO#\O. + |/####-     .     |/.    ||                   :: |
| +  OO#  +   +   OOOOO#OO\   \|  +   +   +   + |/+    || 3.14 < r < 3.99      |
|    ##########OOOOOOOO##OOOOOOOOo             /|      |+- LOG10 |A-B| --------+
|     OO##OOOOOOOOOOO- O#  --O- \\--         --/       ||             .        |
| .    OO##O##OOx.......#|...-----------------    .    ||                      |
|     ...#######xx.....O#       .-----------.......    ||                      |
|.  .. -- .####--x#   O## ...........       .. -- ..   ||                      |
|   .........OO###o@OO##............................   ||.******************   |
| .     ................--------................  .    ||                   *@ |
|                       ........                       ||                   .  |
|                           +                          ||                      |
|                                                      || .................... |
| dt:0.012  pairs:3  eps:0.01200          .            || -6 ... +2 / time >   |
+------------------------------------------------------++----------------------+
");
    }

    #[test]
    fn chaos_short_trail_first_frame() {
        let mut values = defaults();
        for (i, value) in [
            (2, 0.002),
            (3, 16.0),
            (16, 0.12),
            (20, 0.0),
            (21, 0.35),
            (23, 2.0),
        ] {
            values[i] = value;
        }
        insta::assert_snapshot!(plain(&render(80, 24, 1702, 0.0, &values)), @r"
+- ASTRA / LORENZ -------------------------------------++- LOGISTIC / r -------+
| PHASE SPACE   A:@ B:o                                ||                      |
| .       .       .       . +     .       .       .    ||               :::::: |
|                                            --        ||    :::::::*::::::::: |
|       -----               +             ---- |       || :::       *::::::::: |
|      /-/--------                      -----||        ||           *::::::::: |
| .    ||/|/---------     . +     . --------|/    .    || ::::::::::*::::::::: |
|     |||-@-//----------         ---------||//         ||        :::*::::::::: |
|      \||| ||//--------- | +  -------  |////          ||            ::::::::: |
|      \\ \\\\\|-----\\\--o---------   /////           ||                ::::: |
| .     \\\\\\\\\ . -\\\\-|-----//.  //////       .    ||                   :: |
| +   + \\\\\\\\\\-  //--|/=|//// +--/////+   +   +    || 3.14 < r < 3.99      |
|         \\\\\\---------@--o-||\----////              |+- LOG10 |A-B| --------+
|          \\\\\---------|//|||\-----///               ||                      |
| .       . .\\-----------//|\\-----//......      .    ||                      |
|     ......--\------------/| ------...-----.......    || *******************@ |
|   .. .....    -----------/|-----          .. -- ..   ||                      |
|   ................------..\--.....................   ||                      |
| .     ................--------................  .    ||                      |
|                       ........                       ||                      |
|                           +                          ||                      |
|                                                      || .................... |
| dt:0.002  pairs:2  eps:0.01200                       || -6 ... +2 / time >   |
+------------------------------------------------------++----------------------+
");
    }

    #[test]
    fn complete_first_frame_without_background_or_heads() {
        // Test the actual orbit layer: decorations cannot satisfy its coverage.
        let mut values = defaults();
        for i in [20, 21, 22, 23] {
            values[i] = 0.0;
        }
        values[3] = 16.0;
        for family in 0..3 {
            values[0] = family as f32;
            for seed in [0, 1, 7, 42, 1701, 1702, u64::MAX] {
                let grid = render(80, 24, seed, 0.0, &values);
                let cells: Vec<_> = (2..21)
                    .flat_map(|y| (2..54).map(move |x| (x, y)))
                    .filter(|&(x, y)| grid[y][x].ch != ' ')
                    .collect();
                let columns = cells.iter().map(|p| p.0).max().unwrap_or(0)
                    - cells.iter().map(|p| p.0).min().unwrap_or(0);
                let rows = cells.iter().map(|p| p.1).max().unwrap_or(0)
                    - cells.iter().map(|p| p.1).min().unwrap_or(0);
                assert!(
                    cells.len() >= 80 && columns >= 24 && rows >= 8,
                    "family {family}, seed {seed}: {} cells across {columns}x{rows}",
                    cells.len()
                );
            }
        }
    }

    #[test]
    fn large_grid_extreme_controls() {
        let mut values: [f32; 24] = std::array::from_fn(|i| PARAMS[i].max);
        values[2] = PARAMS[2].min;
        for family in 0..3 {
            values[0] = family as f32;
            let grid = render(2000, 1000, 1701, f32::MAX, &values);
            assert_eq!(grid.len(), 1000);
            assert!(
                grid.iter()
                    .all(|row| row.len() == 2000 && row.iter().all(|c| c.ch.is_ascii()))
            );
        }
    }

    #[test]
    fn deterministic_and_time_controls() {
        let values = defaults();
        let first = render(80, 24, 1701, 4.0, &values);
        assert_eq!(first, render(80, 24, 1701, 4.0, &values));
        assert_ne!(first, render(80, 24, 1701, 0.0, &values));
        assert_ne!(first, render(80, 24, 1702, 4.0, &values));
        assert_eq!(
            render(80, 24, 1701, f32::NAN, &values),
            render(80, 24, 1701, 0.0, &values)
        );
        let mut frozen = values;
        frozen[8] = 0.0;
        assert_eq!(
            render(80, 24, 1701, 0.0, &frozen),
            render(80, 24, 1701, 12345.0, &frozen)
        );
    }

    #[test]
    fn every_control_changes_cells() {
        let values = defaults();
        let reference = render(80, 24, 1701, 4.0, &values);
        for (i, p) in PARAMS.iter().enumerate() {
            assert_eq!(PARAMS.iter().filter(|q| q.key == p.key).count(), 1);
            let mut tuned = values;
            tuned[i] = if p.default == p.max { p.min } else { p.max };
            let result = render(80, 24, 1701, 4.0, &tuned);
            // Exclude text and borders: every control must alter the picture.
            assert!(
                (2..21).any(|y| (2..54)
                    .chain(58..78)
                    .any(|x| reference[y][x] != result[y][x])),
                "{} has no visual effect",
                p.key
            );
        }
    }

    #[test]
    fn parameter_clamps_and_nonfinite_defaults() {
        let low: [f32; 24] = std::array::from_fn(|i| PARAMS[i].min);
        let high: [f32; 24] = std::array::from_fn(|i| PARAMS[i].max);
        assert_eq!(
            render(80, 24, 1701, 1.0, &low),
            render(80, 24, 1701, 1.0, &[-f32::MAX; 24])
        );
        assert_eq!(
            render(80, 24, 1701, 1.0, &high),
            render(80, 24, 1701, 1.0, &[f32::MAX; 24])
        );
        for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert_eq!(
                render(80, 24, 1701, 1.0, &defaults()),
                render(80, 24, 1701, 1.0, &[bad; 24])
            );
        }
    }

    #[test]
    fn small_grids_and_extreme_time_terminate() {
        for family in 0..3 {
            for (width, height) in [
                (0, 0),
                (0, 8),
                (8, 0),
                (1, 1),
                (1, 24),
                (80, 1),
                (7, 5),
                (8, 6),
                (48, 16),
            ] {
                let mut values: [f32; 24] = std::array::from_fn(|i| PARAMS[i].max);
                values[0] = family as f32;
                for time in [f32::MAX, -f32::MAX, f32::INFINITY] {
                    let grid = render(width, height, u64::MAX, time, &values);
                    assert_eq!(grid.len(), height);
                    assert!(
                        grid.iter()
                            .all(|row| row.len() == width && row.iter().all(|c| c.ch.is_ascii()))
                    );
                }
            }
        }
    }

    #[test]
    fn paired_integrator_is_finite_and_respects_identity() {
        for family in 0..3 {
            let mut pair = [[1.0, 0.4, 2.0]; 2];
            for _ in 0..2048 {
                pair = advance(pair, 0.035 * [1.0, 2.5, 12.0][family], family, 0.6, 2.0);
                assert_eq!(pair[0], pair[1]);
                assert!(
                    pair.iter()
                        .flatten()
                        .all(|v| v.is_finite() && v.abs() <= 80.0)
                );
            }
        }
    }

    #[test]
    fn frame_inputs_and_parameter_precedence() {
        // Thread-local fallback avoids mutating the process environment in tests.
        let saved = crate::opts::LIVE_PARAMS.with(|p| {
            let saved = p.borrow().clone();
            *p.borrow_mut() = PARAMS.iter().map(|p| (p.key, Some(p.default))).collect();
            saved
        });
        let mut grid = vec![vec![Cell::blank(); 80]; 24];
        let mut rng = StdRng::seed_from_u64(99);
        let palette = crate::color::make_palette(1701);
        let args: Vec<String> = ["ascii-renderer", "1701", "astra-chaos-theory", "bone", "2"]
            .into_iter()
            .map(str::to_owned)
            .collect();
        let mut values = defaults();
        let mut frame = ModeFrame {
            grid: &mut grid,
            width: 80,
            height: 24,
            seed: 1701,
            palette: &palette,
            rng: &mut rng,
            time: 4.0,
            args: &args,
            param_values: None,
        };
        MODE.render(&mut frame);
        values[0] = 2.0;
        assert_eq!(*frame.grid, render(80, 24, 1701, 4.0, &values));
        values[0] = 1.0;
        frame.param_values = Some(&values);
        MODE.render(&mut frame);
        assert_eq!(*frame.grid, render(80, 24, 1701, 4.0, &values));
        frame.args = &[];
        frame.param_values = Some(&[]);
        MODE.render(&mut frame);
        let base = render(80, 24, 1701, 4.0, &defaults());
        assert_eq!(*frame.grid, base);
        let other_palette = crate::color::make_palette(12);
        frame.palette = &other_palette;
        MODE.render(&mut frame);
        assert_ne!(*frame.grid, base);
        assert_eq!(plain(frame.grid), plain(&base));
        // Oversized declared dimensions and ragged storage use their intersection.
        frame.width = usize::MAX;
        frame.height = usize::MAX;
        frame.grid[3].truncate(1);
        MODE.render(&mut frame);
        assert_eq!(frame.grid[3].len(), 1);
        frame.grid[5].clear();
        MODE.render(&mut frame);
        assert!(frame.grid[5].is_empty());
        crate::opts::LIVE_PARAMS.with(|p| *p.borrow_mut() = saved);
    }
}
