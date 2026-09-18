use crate::_0_profile::measure_layer;
use crate::color::{darken, lighten};
use crate::opts::param_f32;
use crate::registry::{AnimKind, Mode, ModeFrame, Param};
use crate::types::{Cell, Grid};
use crossterm::style::Color;
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};
use std::cell::RefCell;
use std::f32::consts::TAU;

pub(super) struct GothicTrace;
pub(super) static MODE: GothicTrace = GothicTrace;

const PARAMS: &[Param] = &[
    param!("GT_SCENE", "0 lancet / 1 rosette", 0.0, 1.0, 0.0, 1.0),
    param!("GT_DEPTH", "lancet generations", 1.0, 4.0, 3.0, 1.0),
    param!("GT_SLENDER", "window width / height", 0.4, 1.0, 0.82, 0.02),
    param!("GT_ASYM", "seeded branch asymmetry", 0.0, 0.3, 0.14, 0.01),
    param!("GT_LOBES", "guilloche lobes", 3.0, 17.0, 8.0, 1.0),
    param!("GT_WIND", "guilloche winding", 1.0, 7.0, 3.0, 1.0),
    param!("GT_BULGE", "guilloche petal depth", 0.08, 0.48, 0.3, 0.01),
    param!("GT_STRANDS", "interlaced strands", 1.0, 4.0, 2.0, 1.0),
    param!("GT_SPEED", "time multiplier", 0.0, 3.0, 1.0, 0.1),
    param!("GT_DRAW", "draw duration", 0.5, 8.0, 3.0, 0.25),
    param!("GT_STAGGER", "stroke delay", 0.0, 0.15, 0.04, 0.01),
    param!("GT_TRAIL", "tracing tail fraction", 0.02, 0.5, 0.12, 0.02),
    param!("GT_GHOST", "guide visibility", 0.0, 1.0, 0.18, 0.05),
];

#[derive(Clone, Copy, Debug, PartialEq)]
struct Point {
    x: f32,
    y: f32,
}
struct Stroke {
    points: Vec<Point>,
    distance: Vec<f32>,
    length: f32,
    depth: u8,
    ink: usize,
    delay: f32,
}
#[derive(Clone, Copy)]
struct Knobs {
    scene: usize,
    depth: usize,
    slender: f32,
    asymmetry: f32,
    lobes: usize,
    winding: usize,
    bulge: f32,
    strands: usize,
    speed: f32,
    draw: f32,
    stagger: f32,
    trail: f32,
    ghost: f32,
}
#[derive(Clone, Copy, PartialEq, Eq)]
struct GeometryKey {
    width: usize,
    height: usize,
    seed: u64,
    knobs: [u32; 8],
}
struct Geometry {
    key: GeometryKey,
    strokes: Vec<Stroke>,
}

thread_local! { static GEOMETRY: RefCell<Option<Geometry>> = const { RefCell::new(None) }; }

impl Mode for GothicTrace {
    fn name(&self) -> &'static str {
        "gothic-trace"
    }
    fn help(&self) -> &'static str {
        "gothic-trace: animated SVG-path-inspired lancets or rosettes [scene] [depth] [slender] [asymmetry] [lobes] [winding] [bulge] [strands] [speed] [draw] [stagger] [trail] [ghost]"
    }
    fn animation(&self) -> AnimKind {
        AnimKind::Iterate
    }
    fn params(&self) -> &'static [Param] {
        PARAMS
    }
    fn render(&self, frame: &mut ModeFrame<'_>) {
        if frame.width == 0 || frame.height == 0 {
            return;
        }
        let values: Vec<f32> = PARAMS
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let v = frame
                    .args
                    .get(i + 4)
                    .and_then(|x| x.parse().ok())
                    .or_else(|| frame.param_values.and_then(|x| x.get(i)).copied())
                    .unwrap_or_else(|| param_f32(p.key, p.default));
                if v.is_finite() {
                    v.clamp(p.min, p.max)
                } else {
                    p.default
                }
            })
            .collect();
        let k = Knobs {
            scene: values[0].round() as usize,
            depth: values[1].round() as usize,
            slender: values[2],
            asymmetry: values[3],
            lobes: values[4].round() as usize,
            winding: values[5].round() as usize,
            bulge: values[6],
            strands: values[7].round() as usize,
            speed: values[8],
            draw: values[9],
            stagger: values[10],
            trail: values[11],
            ghost: values[12],
        };
        let key = GeometryKey {
            width: frame.width,
            height: frame.height,
            seed: frame.seed,
            knobs: [
                k.scene as f32,
                k.depth as f32,
                k.slender,
                k.asymmetry,
                k.lobes as f32,
                k.winding as f32,
                k.bulge,
                k.strands as f32,
            ]
            .map(f32::to_bits),
        };
        GEOMETRY.with(|cache| {
            let mut cache = cache.borrow_mut();
            if cache.as_ref().is_none_or(|g| g.key != key) {
                *cache = Some(Geometry {
                    key,
                    strokes: build_geometry(frame.width, frame.height, frame.seed, &k),
                });
            }
            for row in frame.grid.iter_mut() {
                row.fill(Cell::blank());
            }
            let strokes = &cache.as_ref().unwrap().strokes;
            measure_layer("gothic-trace", "trace", || {
                draw_trace(frame.grid, strokes, frame.palette, frame.time, &k);
            });
        });
    }
}

fn add_stroke(out: &mut Vec<Stroke>, mut points: Vec<Point>, depth: u8, ink: usize, delay: f32) {
    points.dedup_by(|a, b| a == b);
    if points.len() < 2 || points.iter().any(|p| !p.x.is_finite() || !p.y.is_finite()) {
        return;
    }
    let mut distance = vec![0.0];
    for pair in points.windows(2) {
        distance
            .push(distance.last().unwrap() + (pair[1].x - pair[0].x).hypot(pair[1].y - pair[0].y));
    }
    let length = *distance.last().unwrap();
    if length > 1e-6 {
        out.push(Stroke {
            points,
            distance,
            length,
            depth,
            ink: ink % 5,
            delay,
        });
    }
}

fn flatten_cubic(out: &mut Vec<Point>, p: [Point; 4], depth: u8) {
    let d = |q: Point| {
        let a = p[0];
        let b = p[3];
        let dx = b.x - a.x;
        let dy = b.y - a.y;
        if dx.abs() + dy.abs() < 1e-6 {
            (q.x - a.x).hypot(q.y - a.y)
        } else {
            ((q.x - a.x) * dy - (q.y - a.y) * dx).abs() / (dx * dx + dy * dy).sqrt()
        }
    };
    if depth >= 8 || (d(p[1]) < 0.08 && d(p[2]) < 0.08) {
        out.push(p[3]);
        return;
    }
    let m = |a: Point, b: Point| Point {
        x: (a.x + b.x) * 0.5,
        y: (a.y + b.y) * 0.5,
    };
    let ab = m(p[0], p[1]);
    let bc = m(p[1], p[2]);
    let cd = m(p[2], p[3]);
    let abc = m(ab, bc);
    let bcd = m(bc, cd);
    let mid = m(abc, bcd);
    flatten_cubic(out, [p[0], ab, abc, mid], depth + 1);
    flatten_cubic(out, [mid, bcd, cd, p[3]], depth + 1);
}

fn lancet(cx: f32, base: f32, width: f32, height: f32) -> Vec<Point> {
    let spring = base - 0.55 * height;
    let rise = 0.45 * height;
    let top = spring - rise;
    let l = cx - width / 2.;
    let r = cx + width / 2.;
    let mut out = vec![Point { x: l, y: base }, Point { x: l, y: spring }];
    flatten_cubic(
        &mut out,
        [
            Point { x: l, y: spring },
            Point {
                x: l,
                y: spring - 0.55 * rise,
            },
            Point {
                x: cx - 0.22 * width,
                y: top + 0.16 * rise,
            },
            Point { x: cx, y: top },
        ],
        0,
    );
    flatten_cubic(
        &mut out,
        [
            Point { x: cx, y: top },
            Point {
                x: cx + 0.22 * width,
                y: top + 0.16 * rise,
            },
            Point {
                x: r,
                y: spring - 0.55 * rise,
            },
            Point { x: r, y: spring },
        ],
        0,
    );
    out.push(Point { x: r, y: base });
    out
}

fn guilloche(
    center: Point,
    radius: f32,
    mut lobes: usize,
    mut winding: usize,
    bulge: f32,
    phase: f32,
    rotation: f32,
) -> Vec<Point> {
    while gcd(lobes, winding) != 1 {
        winding += 1;
    }
    let count = (winding + lobes) * 18;
    let dt = TAU / count as f32;
    let scale = radius / 1.045;
    let p = |t: f32| {
        let mut x = 0.;
        let mut y = 0.;
        for (f, a, o) in [
            (winding as f32, 1. - bulge, 0.),
            ((winding as i32 - lobes as i32) as f32, bulge, phase),
            ((winding + lobes) as f32, 0.045, -phase),
        ] {
            x += a * (f * t + o + rotation).cos();
            y += a * (f * t + o + rotation).sin();
        }
        Point {
            x: center.x + scale * x,
            y: center.y + scale * y,
        }
    };
    let v = |t: f32| {
        let mut x = 0.;
        let mut y = 0.;
        for (f, a, o) in [
            (winding as f32, 1. - bulge, 0.),
            ((winding as i32 - lobes as i32) as f32, bulge, phase),
            ((winding + lobes) as f32, 0.045, -phase),
        ] {
            x -= a * f * (f * t + o + rotation).sin();
            y += a * f * (f * t + o + rotation).cos();
        }
        Point {
            x: scale * x,
            y: scale * y,
        }
    };
    let mut out = vec![p(0.)];
    for i in 0..count {
        let a = i as f32 * dt;
        let b = (i + 1) as f32 * dt;
        flatten_cubic(
            &mut out,
            [
                p(a),
                Point {
                    x: p(a).x + v(a).x * dt / 3.,
                    y: p(a).y + v(a).y * dt / 3.,
                },
                Point {
                    x: p(b).x - v(b).x * dt / 3.,
                    y: p(b).y - v(b).y * dt / 3.,
                },
                p(b),
            ],
            0,
        );
    }
    if let Some(first) = out.first().copied() {
        *out.last_mut().unwrap() = first;
    }
    out
}

fn gcd(mut a: usize, mut b: usize) -> usize {
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a
}

fn branch(
    out: &mut Vec<Stroke>,
    rng: &mut StdRng,
    cx: f32,
    base: f32,
    w: f32,
    h: f32,
    level: usize,
    k: &Knobs,
) {
    add_stroke(out, lancet(cx, base, w, h), level as u8, level, 0.0);
    if level + 1 >= k.depth || w < 4. || h < 6. {
        return;
    }
    let gap = w * 0.05;
    let share = 0.5 + (rng.random::<f32>() - 0.5) * k.asymmetry;
    let left = (w - gap) * share;
    let right = w - gap - left;
    let child_h = h * 0.64;
    branch(
        out,
        rng,
        cx - w / 2. + left / 2.,
        base,
        left,
        child_h,
        level + 1,
        k,
    );
    branch(
        out,
        rng,
        cx + w / 2. - right / 2.,
        base,
        right,
        child_h,
        level + 1,
        k,
    );
    let r = (0.14 * w).min(h * (1. - 0.64) * 0.37);
    let y = base - child_h - 1.12 * r;
    if r >= 1.25 {
        add_stroke(
            out,
            (0..32)
                .map(|i| Point {
                    x: cx + 1.12 * r * (i as f32 * TAU / 31.).cos(),
                    y: y + 1.12 * r * (i as f32 * TAU / 31.).sin(),
                })
                .collect(),
            level as u8,
            level,
            0.,
        );
        add_stroke(
            out,
            guilloche(
                Point { x: cx, y },
                r,
                k.lobes,
                k.winding,
                k.bulge,
                rng.random::<f32>() * TAU,
                0.,
            ),
            level as u8,
            level + 1,
            0.,
        );
    }
}

fn build_geometry(width: usize, height: usize, seed: u64, k: &Knobs) -> Vec<Stroke> {
    let w = (width.saturating_sub(1) as f32) / 2.;
    let h = height.saturating_sub(1) as f32;
    if w < 4. || h < 4. {
        return vec![];
    }
    let mut out = Vec::new();
    let p = 1.;
    let mut rng = StdRng::seed_from_u64(seed ^ 0x4754_4652_414D);
    let n = (w.min(h) * 0.16 * (0.30 + 0.15 * rng.random::<f32>())).max(0.3);
    for &(sx, sy, delay) in &[
        (1., 1., 0.),
        (-1., 1., 0.06),
        (1., -1., 0.06),
        (-1., -1., 0.12),
    ] {
        let pts = vec![
            Point {
                x: p + n * sx,
                y: p,
            },
            Point {
                x: p + 1.2 * n * sx,
                y: p,
            },
            Point {
                x: p + n * sx,
                y: p + n * sy,
            },
            Point {
                x: p,
                y: p + 1.2 * n * sy,
            },
        ];
        add_stroke(&mut out, pts, 0, 0, delay);
    }
    add_stroke(
        &mut out,
        vec![Point { x: p + n, y: p }, Point { x: w - p - n, y: p }],
        0,
        0,
        0.03,
    );
    add_stroke(
        &mut out,
        vec![
            Point { x: p + n, y: h - p },
            Point {
                x: w - p - n,
                y: h - p,
            },
        ],
        0,
        0,
        0.03,
    );
    let cx = w / 2.;
    let base = h / 2. + 0.4 * w.min(h);
    let wh = 0.76 * w.min(h);
    let ww = (wh * k.slender).min(w - 4.);
    if k.scene == 0 {
        add_stroke(
            &mut out,
            lancet(cx, base, ww + wh * 0.018 * 4., wh + wh * 0.024),
            0,
            0,
            0.,
        );
        branch(
            &mut out,
            &mut StdRng::seed_from_u64(seed ^ 0x4754_4152_4348),
            cx,
            base,
            ww,
            wh,
            0,
            k,
        );
    } else {
        let r = 0.36 * w.min(h);
        let mut rr = r;
        let mut rg = StdRng::seed_from_u64(seed ^ 0x4754_524F_5345);
        for band in 0..2 {
            if rr < 1.5 {
                break;
            }
            for thread in 0..k.strands.max(1) {
                let off = if k.strands > 1 {
                    (thread as f32 / (k.strands - 1) as f32 - 0.5) * 0.22 * TAU
                } else {
                    0.
                };
                add_stroke(
                    &mut out,
                    guilloche(
                        Point { x: cx, y: h / 2. },
                        rr,
                        k.lobes,
                        k.winding,
                        k.bulge,
                        rg.random::<f32>() * TAU + off,
                        band as f32 * 0.226,
                    ),
                    band as u8,
                    band + thread,
                    0.0,
                );
            }
            rr *= 0.46;
        }
    }
    out
}

fn draw_ease(x: f32) -> f32 {
    let x = x.clamp(0., 1.);
    if x == 0. || x == 1. {
        return x;
    }
    let mut lo: f32 = 0.;
    let mut hi: f32 = 1.;
    for _ in 0..16 {
        let u = (lo + hi) / 2.;
        let bx = 3. * (1. - u).powi(2) * u * 0.2 + 3. * (1. - u) * u.powi(2) * 0.2 + u.powi(3);
        if bx < x { lo = u } else { hi = u }
    }
    let u = (lo + hi) / 2.;
    3. * (1. - u).powi(2) * u * 0.7 + 3. * (1. - u) * u.powi(2) + u.powi(3)
}
fn stroke_window(time: f32, index: usize, extra: f32, k: &Knobs) -> (f32, f32, bool) {
    let d = k.draw;
    let cycle = d + 1. + d + 0.5;
    let t = (time.max(0.) * k.speed).rem_euclid(cycle);
    let delay = (index as f32 * k.stagger).min(0.8) + extra;
    if t < d + 1. {
        let e = draw_ease(((t - delay) / d).clamp(0., 1.));
        return (0., e, t > delay && t < delay + d);
    }
    if t < d + 2. {
        let head = (t - d - 2.) / d;
        return (head - k.trail, head, true);
    }
    (0., 0., false)
}
fn stroke_point(s: &Stroke, f: f32) -> Point {
    let target = f.clamp(0., 1.) * s.length;
    let mut lo = 0;
    let mut hi = s.distance.len() - 1;
    while lo + 1 < hi {
        let m = (lo + hi) / 2;
        if s.distance[m] < target {
            lo = m
        } else {
            hi = m
        }
    }
    let a = s.distance[lo];
    let b = s.distance[hi];
    let q = if b - a < 1e-6 {
        0.
    } else {
        (target - a) / (b - a)
    };
    Point {
        x: s.points[lo].x + (s.points[hi].x - s.points[lo].x) * q,
        y: s.points[lo].y + (s.points[hi].y - s.points[lo].y) * q,
    }
}
fn paint_stroke(grid: &mut Grid, s: &Stroke, start: f32, end: f32, color: Color, ghost: bool) {
    if end <= start {
        return;
    }
    let a = stroke_point(s, start);
    let b = stroke_point(s, end);
    let steps = ((b.x - a.x).abs().max((b.y - a.y).abs()) * 2.)
        .ceil()
        .max(1.) as usize;
    for i in 0..=steps {
        let q = i as f32 / steps as f32;
        let x = (a.x + (b.x - a.x) * q).round() as isize * 2;
        let y = (a.y + (b.y - a.y) * q).round() as isize;
        if y < 0 || x < 0 || y as usize >= grid.len() || x as usize >= grid[y as usize].len() {
            continue;
        }
        let cell = &mut grid[y as usize][x as usize];
        if ghost {
            if cell.ch == ' ' {
                *cell = Cell::new('.', color);
            }
        } else {
            let ch = if i == 0 {
                '*'
            } else if (b.y - a.y).abs() <= 0.4142 * (b.x - a.x).abs() {
                '-'
            } else if (b.y - a.y).abs() >= 2.4142 * (b.x - a.x).abs() {
                '|'
            } else if (b.x - a.x) * (b.y - a.y) >= 0.0 {
                '\\'
            } else {
                '/'
            };
            cell.ch = if cell.ch != ' ' && cell.ch != ch {
                '+'
            } else {
                ch
            };
            cell.fg = color;
        }
    }
}
fn draw_trace(grid: &mut Grid, strokes: &[Stroke], palette: &[Color; 5], time: f32, k: &Knobs) {
    if grid.len() < 4 || grid[0].len() < 4 {
        if grid.len() == 1 && !grid[0].is_empty() {
            grid[0][0] = Cell::new('*', lighten(palette[0], 45));
        }
        return;
    }
    for s in strokes {
        if k.ghost > 0. {
            paint_stroke(
                grid,
                s,
                0.,
                1.,
                darken(palette[s.ink], (220. * (1. - k.ghost)) as u8),
                true,
            );
        }
    }
    for (i, s) in strokes.iter().enumerate() {
        let (a, b, active) = stroke_window(time, i, s.delay, k);
        if active {
            if a < 0. {
                paint_stroke(
                    grid,
                    s,
                    1. + a,
                    1.,
                    darken(palette[s.ink], (s.depth as u8 * 18).min(80)),
                    false,
                );
                paint_stroke(
                    grid,
                    s,
                    0.,
                    b,
                    darken(palette[s.ink], (s.depth as u8 * 18).min(80)),
                    false,
                )
            } else {
                paint_stroke(
                    grid,
                    s,
                    a,
                    b,
                    darken(palette[s.ink], (s.depth as u8 * 18).min(80)),
                    false,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn frame(seed: u64, time: f32, scene: f32, w: usize, h: usize) -> String {
        let mut grid = vec![vec![Cell::blank(); w]; h];
        let palette = crate::color::make_palette(seed);
        let mut rng = StdRng::seed_from_u64(seed);
        let args = Vec::new();
        let values = [
            scene, 3., 0.82, 0.14, 8., 3., 0.3, 2., 1., 3., 0.04, 0.12, 0.18,
        ];
        MODE.render(&mut ModeFrame {
            grid: &mut grid,
            width: w,
            height: h,
            seed,
            palette: &palette,
            rng: &mut rng,
            time,
            args: &args,
            param_values: Some(&values),
        });
        crate::render::grid_to_plain(&grid).join("\n")
    }

    #[test]
    fn gothic_trace_fixed_seed_views() {
        insta::assert_snapshot!("gothic_trace_lancet_seed42_t0", frame(42, 0., 0., 80, 24));
        insta::assert_snapshot!(
            "gothic_trace_lancet_seed42_t4_5",
            frame(42, 4.5, 0., 80, 24)
        );
        insta::assert_snapshot!(
            "gothic_trace_rosette_seed42_t4_5",
            frame(42, 4.5, 1., 80, 24)
        );
    }

    #[test]
    fn gothic_trace_tiny_gallery() {
        assert_eq!(frame(42, 0., 0., 1, 1), "*");
        assert!(frame(42, 0., 0., 2, 3).len() > 0);
        assert!(!frame(42, 0., 0., 7, 4).is_empty());
    }

    #[test]
    fn easing_has_exact_endpoints() {
        assert_eq!(draw_ease(0.), 0.);
        assert_eq!(draw_ease(1.), 1.);
    }
    #[test]
    fn stroke_distance_interpolates() {
        let mut s = Vec::new();
        add_stroke(
            &mut s,
            vec![
                Point { x: 0., y: 0. },
                Point { x: 1., y: 0. },
                Point { x: 1., y: 3. },
            ],
            0,
            0,
            0.,
        );
        assert!((stroke_point(&s[0], 0.5).y - 1.).abs() < 1e-5);
    }
}
