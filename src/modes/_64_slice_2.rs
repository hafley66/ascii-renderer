//! Port of gothic lib/6_slice, 6a_slicePaths, 6b_slicePose, 7c_sliceVariation.
use crate::color::darken;
use crate::opts::param_f32;
use crate::registry::{AnimKind, Mode, ModeFrame, Param};
use crate::types::{Cell, Grid};
use crossterm::style::Color;
use std::f64::consts::{PI, TAU};
pub(super) struct Slice2;
pub(super) static MODE: Slice2 = Slice2;
pub(super) const PARAMS: &[Param] = &[
    param!(
        "SL2_REVEAL",
        "reveal",
        0.0,
        4.0,
        0.0,
        1.0,
        &["draw", "cut", "slide", "draw+slide", "glow"]
    ),
    param!("SL2_CUT", "cut px", 8.0, 160.0, 48.0, 1.0),
    param!("SL2_ANGLE", "sweep degrees", 0.0, 360.0, 215.0, 1.0),
    param!("SL2_JIT", "flight jitter degrees", 0.0, 180.0, 18.0, 1.0),
    param!("SL2_DIST", "chord flight distance", 0.3, 3.0, 1.2, 0.1),
    param!("SL2_FLIGHT", "flight ms", 60.0, 600.0, 160.0, 10.0),
    param!(
        "SL2_ORDER",
        "order",
        0.0,
        10.0,
        0.0,
        1.0,
        &[
            "sweep",
            "radial",
            "radial-in",
            "path",
            "subpath",
            "golden",
            "golden-angle",
            "vdc",
            "spectral",
            "hilbert",
            "random"
        ]
    ),
    param!(
        "SL2_CURVE",
        "curve",
        0.0,
        18.0,
        0.0,
        1.0,
        &[
            "linear",
            "ease-in",
            "ease-out",
            "ease-in-out",
            "expo",
            "sine",
            "steps",
            "white",
            "smooth-N",
            "fourier",
            "logistic",
            "lorenz",
            "poisson",
            "pink",
            "red",
            "blue",
            "violet",
            "gamma",
            "levy"
        ]
    ),
    param!("SL2_ORD_N", "N beta k", 1.0, 8.0, 2.0, 0.5),
    param!("SL2_GAIN", "progression gain", 0.0, 4.0, 1.5, 0.1),
    param!("SL2_SILENCE", "gap median ceiling", 1.0, 8.0, 2.5, 0.5),
    param!("SL2_SPREAD", "start window ms", 0.0, 3000.0, 800.0, 50.0),
    param!("SL2_BURST", "steps curve bins", 1.0, 16.0, 10.0, 1.0),
    param!(
        "SL2_FEASE",
        "fease",
        0.0,
        6.0,
        0.0,
        1.0,
        &[
            "back",
            "spring",
            "cubic-out",
            "expo-out",
            "elastic",
            "linear",
            "cut"
        ]
    ),
    param!("SL2_STRETCH", "chord stretch", 0.0, 2.5, 0.9, 0.05),
    param!("SL2_OS", "overshoot", 0.0, 1.4, 0.5, 0.05),
    param!("SL2_AILEN", "afterimage length", 0.5, 5.0, 2.0, 0.1),
    param!("SL2_AI", "afterimage on", 0.0, 1.0, 1.0, 1.0),
    param!("SL2_AI_OPACITY", "afterimage opacity", 0.0, 1.0, 0.85, 0.01),
    param!(
        "SL2_AI_FADE",
        "afterimage fade ms",
        20.0,
        2000.0,
        120.0,
        10.0
    ),
    param!("SL2_WEIGHT", "moving width", 0.4, 2.5, 1.0, 0.1),
    param!("SL2_FINAL_WEIGHT", "landed width", 0.4, 2.5, 1.0, 0.1),
    param!("SL2_TIME", "saved cycle position", 0.0, 1.0, 0.0, 0.001),
    param!("SL2_SPEED", "clock speed", 0.1, 2.0, 1.0, 0.1),
    param!("SL2_RUN", "advance clock", 0.0, 1.0, 1.0, 1.0),
    param!(
        "SL2_CORE",
        "core",
        0.0,
        3.0,
        0.0,
        1.0,
        &["original", "iris", "blades", "lattice"]
    ),
    param!("SL2_CORE_SIDES", "core symmetry", 2.0, 9.0, 5.0, 1.0),
    param!(
        "SL2_CORE_SIZE",
        "core radius fraction",
        0.16,
        0.46,
        0.34,
        0.01
    ),
    param!("SL2_CORE_TURN", "core turn degrees", -90.0, 90.0, 18.0, 1.0),
    param!(
        "SL2_CORE_FRAME",
        "core frame",
        0.0,
        2.0,
        0.0,
        1.0,
        &["open", "circle", "polygon"]
    ),
    param!(
        "SL2_SHAPE",
        "shape",
        0.0,
        1.0,
        0.0,
        1.0,
        &["seal", "polyline"]
    ),
    param!("SL2_LOOP", "repeat cycle", 0.0, 1.0, 1.0, 1.0),
];
const REVEAL: usize = 0;
const CUT: usize = 1;
const ANGLE: usize = 2;
const JIT: usize = 3;
const DIST: usize = 4;
const FLIGHT: usize = 5;
const ORDER: usize = 6;
const CURVE: usize = 7;
const ORD_N: usize = 8;
const GAIN: usize = 9;
const SILENCE: usize = 10;
const SPREAD: usize = 11;
const BURST: usize = 12;
const FEASE: usize = 13;
const STRETCH: usize = 14;
const OS: usize = 15;
const AILEN: usize = 16;
const AI: usize = 17;
const AI_OPACITY: usize = 18;
const AI_FADE: usize = 19;
const WEIGHT: usize = 20;
const FINAL_WEIGHT: usize = 21;
const TIME: usize = 22;
const SPEED: usize = 23;
const RUN: usize = 24;
const CORE: usize = 25;
const CORE_SIDES: usize = 26;
const CORE_SIZE: usize = 27;
const CORE_TURN: usize = 28;
const CORE_FRAME: usize = 29;
const SHAPE: usize = 30;
const LOOP: usize = 31;
const DIRECTION: usize = PARAMS.len() + 71;

impl Mode for Slice2 {
    fn name(&self) -> &'static str {
        "slice-2"
    }
    fn help(&self) -> &'static str {
        "Source slice seal: equal arc cuts, ordered reveals, chord flight, blades and afterimages. Select values documented in plans/1_slice_port.md."
    }
    fn animation(&self) -> AnimKind {
        AnimKind::Iterate
    }
    fn params(&self) -> &'static [Param] {
        all_params()
    }
    fn render(&self, f: &mut ModeFrame<'_>) {
        let mut k = values(f, all_params());
        animate_inputs(&mut k, f.time);
        let paths = geometry(f.seed, &k);
        let strokes = compile(paths, f.seed, &k);
        paint(f, &strokes, &k);
    }
}
pub(super) fn values(f: &ModeFrame<'_>, params: &[Param]) -> Vec<f64> {
    params
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let v = f
                .args
                .get(i + 4)
                .and_then(|s| s.parse::<f32>().ok())
                .or_else(|| f.param_values.and_then(|v| v.get(i)).copied())
                .unwrap_or_else(|| param_f32(p.key, p.default));
            if v.is_finite() {
                v.clamp(p.min, p.max) as f64
            } else {
                p.default as f64
            }
        })
        .collect()
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Point(pub f64, pub f64);
impl Point {
    fn mix(self, b: Self, t: f64) -> Self {
        Self(self.0 + (b.0 - self.0) * t, self.1 + (b.1 - self.1) * t)
    }
    fn distance(self, b: Self) -> f64 {
        (self.0 - b.0).hypot(self.1 - b.1)
    }
    pub(super) fn add(self, b: Self) -> Self {
        Self(self.0 + b.0, self.1 + b.1)
    }
}
pub(super) fn polar(r: f64, t: f64) -> Point {
    Point(r * t.cos(), r * t.sin())
}
pub(super) fn closed(mut p: Vec<Point>) -> Vec<Point> {
    if let Some(x) = p.first().copied() {
        p.push(x)
    }
    p
}
pub(super) fn circle(c: Point, r: f64) -> Vec<Point> {
    (0..=128)
        .map(|i| c.add(polar(r, PI + i as f64 * TAU / 128.)))
        .collect()
}
#[derive(Clone)]
pub(super) struct Random(pub(super) u32);
impl Random {
    pub(super) fn next(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x6d2b79f5);
        let mut t = self.0;
        t = (t ^ (t >> 15)).wrapping_mul(t | 1);
        t ^= t.wrapping_add((t ^ (t >> 7)).wrapping_mul(t | 61));
        ((t ^ (t >> 14)) as f64) / 4294967296.
    }
    pub(super) fn pick(&mut self, n: usize) -> usize {
        (self.next() * n as f64).floor() as usize
    }
    fn normal(&mut self) -> f64 {
        for _ in 0..1024 {
            let a = self.next() * 2. - 1.;
            let b = self.next() * 2. - 1.;
            let r = a * a + b * b;
            if r > 0. && r <= 1. {
                return b * (-2. * r.ln() / r).sqrt();
            }
        }
        0.
    }
    fn gamma(&mut self, k: f64) -> f64 {
        let d = k - 1. / 3.;
        let c = 1. / (9. * d).sqrt();
        for _ in 0..1024 {
            let x = self.normal();
            let v = 1. + c * x;
            if v <= 0. {
                continue;
            }
            let v = v * v * v;
            let u = self.next();
            if u < 1. - 0.0331 * x.powi(4) || u.ln() < 0.5 * x * x + d * (1. - v + v.ln()) {
                return d * v;
            }
        }
        k
    }
}
fn gcd(mut a: usize, mut b: usize) -> usize {
    while b != 0 {
        (a, b) = (b, a % b)
    }
    a
}
pub(super) fn star(out: &mut Vec<Vec<Point>>, r: f64, n: usize, step: usize, turn: f64) {
    let g = gcd(n, step);
    for start in 0..g {
        out.push(closed(
            (0..n / g)
                .map(|i| polar(r, turn + ((start + i * step) % n) as f64 * TAU / n as f64))
                .collect(),
        ))
    }
}
pub(super) fn foil(c: Point, r: f64, n: usize) -> Vec<Point> {
    let rho = (r * (PI / n as f64).sin() / (1. + (PI / n as f64).sin()) * 1.15).min(r * 0.97);
    let centers: Vec<_> = (0..n)
        .map(|i| c.add(polar(r - rho, -PI / 2. + i as f64 * TAU / n as f64)))
        .collect();
    let tips: Vec<_> = (0..n)
        .map(|i| {
            let a = centers[i];
            let b = centers[(i + 1) % n];
            let d = a.distance(b);
            let h = (rho * rho - d * d / 4.).max(0.).sqrt();
            let m = a.mix(b, 0.5);
            let v = Point(-(b.1 - a.1) / d * h, (b.0 - a.0) / d * h);
            let p = m.add(v);
            let q = m.add(Point(-v.0, -v.1));
            if p.distance(c) > q.distance(c) { p } else { q }
        })
        .collect();
    let mut out = vec![tips[n - 1]];
    for i in 0..n {
        let a = tips[(i + n - 1) % n];
        let b = tips[i];
        let cc = centers[i];
        let start = (a.1 - cc.1).atan2(a.0 - cc.0);
        let end = (b.1 - cc.1).atan2(b.0 - cc.0);
        let mut span = (end - start).rem_euclid(TAU);
        let apex = (-PI / 2. + i as f64 * TAU / n as f64 - start).rem_euclid(TAU);
        if apex > span {
            span -= TAU
        }
        for j in 1..=32 {
            out.push(cc.add(polar(rho, start + span * j as f64 / 32.)))
        }
    }
    out
}
#[derive(Clone)]
struct Band {
    kind: usize,
    w: f64,
    k: usize,
    step: usize,
    sides: usize,
    lobes: usize,
}
pub(super) fn geometry(seed: u64, k: &[f64]) -> Vec<Vec<Point>> {
    if k[SHAPE].round() == 1. {
        return vec![
            closed(
                (0..5)
                    .map(|i| polar(112. * 0.92, -PI / 2. + i as f64 * TAU * 2. / 5.))
                    .collect(),
            ),
            (0..=24)
                .map(|i| {
                    Point(
                        -112. + 224. * i as f64 / 24.,
                        112. * 0.3 * if i % 2 == 1 { 1. } else { -1. },
                    )
                })
                .collect(),
            (0..=200)
                .map(|i| {
                    polar(
                        112. * 0.12 + 112. * 0.46 * i as f64 / 200.,
                        i as f64 / 200. * TAU * 3.,
                    )
                })
                .collect(),
        ];
    }
    seal_paths(seed, k, 118., 2., true, false)
}
pub(super) fn seal_paths(
    seed: u64,
    k: &[f64],
    r: f64,
    min: f64,
    k_first: bool,
    pupil: bool,
) -> Vec<Vec<Point>> {
    let mut rng = Random(seed as u32);
    let n = 3 + rng.pick(6);
    let ks: Vec<_> = (2..=(n - 1) / 2).filter(|&s| gcd(n, s) == 1).collect();
    let mut mult = if k_first { 2 + rng.pick(3) } else { 0 };
    let make = |kind, w| Band {
        kind,
        w,
        k: 2,
        step: 1,
        sides: n,
        lobes: 3,
    };
    let mut bands = vec![make(0, 0.35)];
    if r >= 28. {
        let mut b = make(if rng.pick(2) == 0 { 1 } else { 2 }, 0.7);
        if !k_first {
            mult = 2 + rng.pick(3)
        }
        b.k = mult;
        bands.push(b);
    }
    for i in 0..(r / 9.).floor() as usize {
        let mut b = make(
            if i % 2 == 0 {
                3 + rng.pick(3)
            } else {
                6 + rng.pick(3)
            },
            1.,
        );
        if i % 2 == 0 {
            b.step = if ks.is_empty() {
                1
            } else {
                ks[rng.pick(ks.len())]
            };
            b.sides = if rng.next() < 0.3 { n * 2 } else { n }
        } else {
            b.lobes = 3 + rng.pick(3);
            b.k = 2 + rng.pick(2)
        }
        bands.push(b)
    }
    if bands.last().unwrap().kind != 3 {
        let mut b = make(3, 1.);
        b.step = ks.first().copied().unwrap_or(1);
        bands.push(b)
    }
    let source_core = 0.16 + rng.next() * 0.08;
    let eye = 3 + rng.pick(4);
    let core = if k[CORE].round() == 0. {
        source_core
    } else {
        k[CORE_SIZE]
    };
    let free = r - r * core - r * 0.03 * bands.len() as f64;
    let total: f64 = bands.iter().map(|b| b.w).sum();
    let mut r0 = r * core;
    let mut out = vec![];
    for mut b in bands {
        let h = free * b.w / total;
        let r1 = r0 + h;
        let rm = (r0 + r1) / 2.;
        if b.kind == 1 {
            b.kind = 2
        } // hero rune font is below 6px, source degrades to ticks
        let rr = (h * 0.46).min(rm * (PI / n as f64).sin() * 0.9);
        if b.kind == 7 && TAU * rr / (b.lobes as f64) < min * 1.5 {
            b.kind = 6
        }
        if b.kind == 6 && rr < min * 0.75 {
            b.kind = 8
        }
        if b.kind == 5 && h < min {
            b.kind = 0
        }
        match b.kind {
            0 => {
                out.push(circle(Point(0., 0.), r1));
                if h >= min && r >= 24. {
                    out.push(circle(Point(0., 0.), r0))
                }
            }
            2 => {
                let count = n * b.k;
                if TAU * rm / (count as f64) < min {
                    out.push(circle(Point(0., 0.), r1))
                } else {
                    for i in 0..count {
                        let a = -PI / 2. + i as f64 * TAU / count as f64;
                        out.push(vec![
                            polar(if i % b.k == 0 { r0 } else { r0 + h * 0.5 }, a),
                            polar(r1, a),
                        ])
                    }
                }
            }
            3 => star(&mut out, r1, b.sides, b.step, -PI / 2.),
            4 => {
                for i in 0..n {
                    out.push(vec![
                        polar(r1, -PI / 2. + i as f64 * TAU / n as f64),
                        polar(r1, -PI / 2. + (i + b.step) as f64 * TAU / n as f64),
                    ])
                }
            }
            5 => {
                for i in 0..n {
                    let a = -PI / 2. + i as f64 * TAU / n as f64;
                    out.push(vec![polar(r0, a), polar(r1, a)])
                }
                out.push(circle(Point(0., 0.), r1))
            }
            6 | 7 => {
                for i in 0..n {
                    let c = polar(rm, -PI / 2. + i as f64 * TAU / n as f64);
                    out.push(if b.kind == 6 {
                        circle(c, rr)
                    } else {
                        foil(c, rr, b.lobes)
                    })
                }
            }
            8 => {
                let count = n * b.k;
                let rr = (h * 0.14).max(0.6);
                if TAU * rm / count as f64 >= rr * 3. {
                    for i in 0..count {
                        out.push(circle(
                            polar(rm, -PI / 2. + i as f64 * TAU / count as f64),
                            rr,
                        ))
                    }
                }
            }
            _ => {}
        }
        r0 = r1 + r * 0.03;
    }
    let radius = r * core * 0.9;
    if k[CORE].round() == 0. {
        out.push(circle(Point(0., 0.), radius));
        let ir = radius * 0.62;
        if TAU * ir / eye as f64 >= min * 1.5 {
            out.push(foil(Point(0., 0.), ir, eye));
        } else if ir >= min {
            out.push(circle(Point(0., 0.), ir));
        }
        if pupil && radius * 0.2 >= 0.6 {
            out.push(circle(Point(0., 0.), radius * 0.2));
        }
        return out;
    }
    let n = k[CORE_SIDES].round() as usize;
    let turn = k[CORE_TURN] * PI / 180.;
    let at = |r: f64, i: f64, p: f64| polar(r, -PI / 2. + i * TAU / n as f64 + p);
    match k[CORE_FRAME].round() as usize {
        1 => out.push(circle(Point(0., 0.), radius)),
        2 => out.push(if n == 2 {
            closed(vec![
                Point(-radius, -radius * 0.2),
                Point(radius, -radius * 0.2),
                Point(radius, radius * 0.2),
                Point(-radius, radius * 0.2),
            ])
        } else {
            closed((0..n).map(|i| at(radius, i as f64, 0.)).collect())
        }),
        _ => {}
    }
    if k[CORE].round() == 1. {
        for i in 0..n {
            let i = i as f64;
            out.push(closed(vec![
                at(radius * 0.92, i, -0.2),
                at(radius * 0.87, i + 0.64, 0.),
                at(radius * 0.25, i + 0.7, turn),
                at(radius * 0.39, i, turn),
            ]));
            out.push(vec![
                at(radius * 0.81, i, 0.),
                at(radius * 0.52, i + 0.26, turn),
                at(radius * 0.4, i + 0.45, turn),
            ])
        }
    } else if k[CORE].round() == 2. {
        for i in 0..n {
            let i = i as f64;
            out.push(closed(vec![
                at(radius * 0.94, i, turn),
                at(radius * 0.28, i + 0.19, 0.),
                at(radius * 0.16, i + 0.54, 0.),
                at(radius * 0.68, i + 0.28, turn),
            ]));
            out.push(vec![
                at(radius * 0.85, i + 0.12, turn),
                at(radius * 0.49, i + 0.38, turn),
                at(radius * 0.32, i + 0.44, 0.),
            ])
        }
    } else {
        let n = n.max(3);
        let outer: Vec<_> = (0..n)
            .map(|i| polar(radius * 0.9, -PI / 2. + i as f64 * TAU / n as f64))
            .collect();
        let inner: Vec<_> = (0..n)
            .map(|i| polar(radius * 0.37, -PI / 2. + i as f64 * TAU / n as f64 + turn))
            .collect();
        out.push(closed(outer.clone()));
        out.push(closed(inner.clone()));
        for i in 0..n {
            let a = outer[i];
            let b = inner[i];
            let c = outer[(i + 1) % n];
            let d = inner[(i + 1) % n];
            out.push(vec![a, b]);
            for t in [0.25, 0.5, 0.75] {
                let l = a.mix(b, t);
                out.push(vec![l, l.mix(c.mix(d, t), 0.28)])
            }
        }
    }
    out
}
#[derive(Debug)]
pub(super) struct Stroke {
    pts: Vec<Point>,
    c: Point,
    diag: f64,
    th: f64,
    d: f64,
    noise: f64,
    sub: usize,
    rev: bool,
    start: f64,
}
fn along(pts: &[Point], q: f64) -> Point {
    let len: f64 = pts.windows(2).map(|p| p[0].distance(p[1])).sum();
    let mut t = q.clamp(0., 1.) * len;
    for pair in pts.windows(2) {
        let d = pair[0].distance(pair[1]);
        if t <= d {
            return pair[0].mix(pair[1], if d > 0. { t / d } else { 0. });
        }
        t -= d
    }
    *pts.last().unwrap()
}
pub(super) fn compile(paths: Vec<Vec<Point>>, seed: u64, k: &[f64]) -> Vec<Stroke> {
    let mut rng = Random(seed as u32 ^ 0x5bf03635);
    let mut out = vec![];
    for (sub, path) in paths.iter().enumerate() {
        let mut distances = vec![0.];
        for p in path.windows(2) {
            distances.push(distances.last().unwrap() + p[0].distance(p[1]));
        }
        let length = *distances.last().unwrap();
        if length <= 0. {
            continue;
        }
        let n = (length / k[CUT].max(6.)).ceil() as usize;
        for i in 0..n {
            let samples = (length / n as f64 / 3.).ceil().clamp(2., 40.) as usize;
            let pts: Vec<_> = (0..=samples)
                .map(|j| {
                    along_table(
                        path,
                        &distances,
                        (i as f64 + j as f64 / samples as f64) / n as f64 * length,
                    )
                })
                .collect();
            let a = pts[0];
            let b = *pts.last().unwrap();
            let chord = a.distance(b);
            let chord = if chord > 0. { chord } else { 12. };
            let th = (b.1 - a.1).atan2(b.0 - a.0)
                + if rng.next() < 0.5 { PI } else { 0. }
                + (rng.next() * 2. - 1.) * k[JIT] * PI / 180.;
            let c = Point(
                pts.iter().map(|p| p.0).sum::<f64>() / pts.len() as f64,
                pts.iter().map(|p| p.1).sum::<f64>() / pts.len() as f64,
            );
            let minx = pts.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
            let maxx = pts.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max);
            let miny = pts.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
            let maxy = pts.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max);
            out.push(Stroke {
                pts,
                c,
                diag: (maxx - minx).hypot(maxy - miny).max(1e-9),
                th,
                d: k[DIST] * chord,
                noise: rng.next(),
                sub,
                rev: rng.next() < 0.5,
                start: 0.,
            });
        }
    }
    schedule(&mut out, seed, k);
    out
}
fn zscore(a: &mut [f64]) {
    let m = a.iter().sum::<f64>() / a.len() as f64;
    let sd = (a.iter().map(|v| (v - m).powi(2)).sum::<f64>() / a.len() as f64).sqrt();
    for x in a {
        *x = (*x - m) / if sd > 0. { sd } else { 1. }
    }
}
fn gaps(kind: usize, n: usize, seed: u64, k: &[f64]) -> Vec<f64> {
    let mut rng = Random(seed as u32 ^ 0x1f123bb5);
    let g = k[GAIN];
    let nn = k[ORD_N];
    let mut a = vec![];
    match kind {
        7 => a = (0..n).map(|_| rng.next() + 0.02).collect(),
        8 => {
            a = (0..n).map(|_| rng.normal()).collect();
            for _ in 0..nn.round() as usize {
                let mut sum = 0.;
                for x in &mut a {
                    sum += *x;
                    *x = sum
                }
                zscore(&mut a)
            }
            for x in &mut a {
                *x = (g * *x).exp()
            }
        }
        9 => {
            let ph: Vec<_> = (0..24).map(|_| rng.next() * TAU).collect();
            a = (0..n)
                .map(|i| {
                    (1..=24)
                        .map(|j| {
                            (TAU * j as f64 * i as f64 / n as f64 + ph[j - 1]).cos()
                                / (j as f64).powf(nn / 2.)
                        })
                        .sum()
                })
                .collect();
            zscore(&mut a);
            for x in &mut a {
                *x = (g * *x).exp()
            }
        }
        10 => {
            let r = 3.57 + 0.43 * (g / 4.).clamp(0., 1.);
            let mut x = 0.1 + rng.next() * 0.8;
            for i in 0..n + 50 {
                x = r * x * (1. - x);
                if i >= 50 {
                    a.push(x + 0.02)
                }
            }
        }
        11 => {
            let (mut x, mut y, mut z) = (1. + rng.next(), 1., 20.);
            let dt = 0.01 * (0.5 + nn / 4.);
            for i in 0..n * 4 + 400 {
                let (dx, dy, dz) = (10. * (y - x), x * (28. - z) - y, x * y - 8. / 3. * z);
                x += dx * dt;
                y += dy * dt;
                z += dz * dt;
                if i >= 400 && i % 4 == 0 {
                    a.push(x)
                }
            }
            zscore(&mut a);
            for x in &mut a {
                *x = (g * *x / 2.).exp()
            }
        }
        12 => a = (0..n).map(|_| -(1. - rng.next()).ln()).collect(),
        13..=16 => {
            let bins = nn.round() as usize + 2;
            let mut states = [vec![], vec![]];
            let mut ids = [0usize; 2];
            for i in 0..n {
                let stream = if kind == 16 { i % 2 } else { 0 };
                if states[stream].is_empty() {
                    states[stream] = (0..bins)
                        .map(|j| {
                            let x = rng.next() * 2. - 1.;
                            if kind == 15 && j % 2 == 0 { -x } else { x }
                        })
                        .collect()
                }
                let tick = ids[stream];
                let j = if kind == 13 {
                    (tick as u32).trailing_zeros() as usize % bins
                } else {
                    tick % bins
                };
                let sign = if tick % 2 == 0 { -1. } else { 1. };
                states[stream][j] = (rng.next() * 2. - 1.) * if kind == 15 { sign } else { 1. };
                a.push(
                    states[stream].iter().sum::<f64>() / bins as f64
                        * if kind == 15 { -sign } else { 1. },
                );
                ids[stream] += 1
            }
            zscore(&mut a);
            for x in &mut a {
                *x = (g * *x).exp()
            }
        }
        17 => a = (0..n).map(|_| rng.gamma(nn)).collect(),
        _ => {
            a = (0..n)
                .map(|_| (1. - rng.next()).powf(-1. / (0.6 + nn / 4.)) - 1. + 0.05)
                .collect()
        }
    }
    let mut sorted = a.clone();
    sorted.sort_by(f64::total_cmp);
    let median = sorted[n / 2];
    let ceiling = if median > 0. { median } else { 1. } * k[SILENCE];
    for x in &mut a {
        *x = x.min(ceiling)
    }
    let total = a.iter().sum::<f64>();
    let mut acc = 0.;
    for x in &mut a {
        acc += *x;
        *x = acc / total
    }
    let first = a[0];
    for x in &mut a {
        *x -= first
    }
    a
}
fn hilbert(mut x: i32, mut y: i32) -> f64 {
    let mut d = 0;
    let mut s = 32;
    while s > 0 {
        let rx = i32::from(x & s != 0);
        let ry = i32::from(y & s != 0);
        d += s * s * ((3 * rx) ^ ry);
        if ry == 0 {
            if rx == 1 {
                x = s - 1 - x;
                y = s - 1 - y
            }
            std::mem::swap(&mut x, &mut y)
        }
        s /= 2
    }
    d as f64
}
fn schedule(st: &mut [Stroke], seed: u64, k: &[f64]) {
    let n = st.len();
    if n == 0 {
        return;
    }
    let mut rng = Random(seed as u32 ^ 0x7f4a7c15);
    let waves: Vec<_> = (0..3)
        .map(|_| {
            let a = rng.next() * TAU;
            let kk = 2. / 240. * (0.5 + rng.next());
            (a.cos() * kk, a.sin() * kk, rng.next() * TAU)
        })
        .collect();
    let mut ang: Vec<_> = (0..n).collect();
    ang.sort_by(|&a, &b| {
        st[a]
            .c
            .1
            .atan2(st[a].c.0)
            .total_cmp(&st[b].c.1.atan2(st[b].c.0))
    });
    let mut ranks = vec![0; n];
    for (r, i) in ang.into_iter().enumerate() {
        ranks[i] = r
    }
    let angle = k[ANGLE] * PI / 180.;
    let phi = (5f64.sqrt() - 1.) / 2.;
    let key = |i: usize| {
        let s = &st[i];
        let r = s.c.0.hypot(s.c.1);
        match k[ORDER].round() as usize {
            0 => -s.c.0 * angle.sin() + s.c.1 * angle.cos() + s.noise * 24.,
            1 => r + s.noise * 8.,
            2 => -r - s.noise * 8.,
            3 => i as f64,
            4 => s.sub as f64 * 1000. + s.noise,
            5 => (i as f64 * phi).fract(),
            6 => (ranks[i] as f64 * phi).fract() + r / 1e6,
            7 => {
                let (mut v, mut ii, mut d) = (0., i, 0.5);
                while ii > 0 {
                    if ii & 1 != 0 {
                        v += d
                    }
                    ii >>= 1;
                    d /= 2.
                }
                v
            }
            8 => waves
                .iter()
                .map(|&(x, y, p)| (x * s.c.0 + y * s.c.1 + p).cos())
                .sum(),
            9 => hilbert(
                ((s.c.0 / 240. + 0.5).clamp(0., 1.) * 63.).floor() as i32,
                ((s.c.1 / 240. + 0.5).clamp(0., 1.) * 63.).floor() as i32,
            ),
            _ => s.noise,
        }
    };
    let mut order: Vec<_> = (0..n).collect();
    order.sort_by(|&a, &b| key(a).total_cmp(&key(b)));
    let curve = k[CURVE].round() as usize;
    let gaps = if curve >= 7 {
        gaps(curve, n, seed, k)
    } else {
        vec![]
    };
    for (rank, i) in order.into_iter().enumerate() {
        let u = if n > 1 {
            rank as f64 / (n - 1) as f64
        } else {
            0.
        };
        let v = match curve {
            0 => u,
            1 => u * u,
            2 => 1. - (1. - u).powi(2),
            3 => {
                if u < 0.5 {
                    2. * u * u
                } else {
                    1. - (-2. * u + 2.).powi(2) / 2.
                }
            }
            4 => {
                if u == 0. {
                    0.
                } else {
                    2f64.powf(10. * u - 10.)
                }
            }
            5 => 1. - (u * PI / 2.).cos(),
            6 => (u * k[BURST] * 0.999).floor() / (k[BURST] - 1.).max(1.),
            _ => gaps[rank],
        };
        st[i].start = 60. + k[SPREAD] * v
    }
}
fn spring(u: f64, os: f64) -> f64 {
    if u == 0. || u >= 1. {
        return u;
    }
    let w = 120f64.sqrt();
    let damping = (16. - os * 10.).max(2.);
    let z = damping / (2. * w);
    let wd = w * (1. - z * z).sqrt();
    let b = z * w / wd;
    let solve = |t: f64| 1. - (-t * z * w).exp() * ((wd * t).cos() + b * (wd * t).sin());
    let (mut t, mut duration, mut rest) = (0., 0., 0);
    for _ in 0..=3000 {
        if rest > 10 {
            break;
        }
        if (1. - solve(t)).abs() < 0.0005 {
            rest += 1
        } else {
            rest = 0
        }
        duration = t;
        t += 0.02
    }
    solve(u * duration)
}
fn ease(u: f64, k: &[f64]) -> f64 {
    match k[FEASE].round() as usize {
        0 => {
            let q = u - 1.;
            1. + (k[OS] + 1.) * q * q * q + k[OS] * q * q
        }
        1 => spring(u, k[OS]),
        2 => 1. - (1. - u).powi(3),
        3 => {
            if u >= 1. {
                1.
            } else {
                1. - 2f64.powf(-10. * u)
            }
        }
        4 => {
            if u >= 1. {
                1.
            } else {
                2f64.powf(-10. * u) * ((u * 10. - 0.75) * TAU / 3.).sin() + 1.
            }
        }
        5 => u,
        _ => {
            if u > 0. {
                1.
            } else {
                0.
            }
        }
    }
}
pub(super) fn paint(f: &mut ModeFrame<'_>, st: &[Stroke], k: &[f64]) {
    if f.width == 0 || f.height == 0 {
        return;
    }
    let duration = (60. + k[SPREAD] + k[FLIGHT] + 350.).round();
    let clock = k[TIME] * duration
        + if k[RUN] >= 0.5 {
            f.time.max(0.) as f64 * 1000. * k[SPEED]
        } else {
            0.
        };
    let t = replay_position(
        clock,
        duration,
        k[RUN] >= 0.5 && k[LOOP] >= 0.5,
        k[DIRECTION].round() as usize,
    );
    let config = motion_config(k);
    let motion = tracks(Some(&config), k, "slice");
    let ms = motion_clock(&config, f.time as f64);
    let scale =
        ((f.width.saturating_sub(2)) as f64 / 2.).min(f.height.saturating_sub(2) as f64) / 244.;
    let center = Point(f.width as f64 / 2., f.height as f64 / 2.);
    for pass in 0..3 {
        for (i, s) in st.iter().enumerate() {
            let mut varied = std::borrow::Cow::Borrowed(k);
            for track in motion.iter().filter(|t| t.stroke) {
                varied.to_mut()[track.index] = track_value(
                    track,
                    k[track.index],
                    ms,
                    stroke_identity(s) ^ fnv(SOURCE_KEYS[track.index]),
                );
            }
            let k = &varied;
            let u = ((t - s.start) / k[FLIGHT]).clamp(0., 1.);
            let age = t - s.start - k[FLIGHT];
            let opacity = (u / 0.25).clamp(0., 1.);
            if opacity <= 0. {
                continue;
            }
            let p = ease(u, k);
            let reveal = k[REVEAL].round() as usize;
            let off = if reveal == 2 || reveal == 3 {
                s.d * if reveal == 3 { 0.35 } else { 1. } * (1. - p)
            } else {
                0.
            };
            let st = if u >= 1. {
                0.
            } else {
                k[STRETCH]
                    * if u < 0.85 {
                        u / 0.85
                    } else {
                        ((u - 0.85) / 0.15 * 4.712).cos() * (1. - (u - 0.85) / 0.15)
                    }
            };
            let c = s.th.cos();
            let sn = s.th.sin();
            let transform = |pt: Point| {
                let q = pt.0 - s.c.0;
                let r = pt.1 - s.c.1;
                Point(
                    pt.0 + st * c * (q * c + r * sn) + off * c,
                    pt.1 + st * sn * (q * c + r * sn) + off * sn,
                )
            };
            let drawn = if reveal == 1 {
                (-0.3 + 1.6 * p.clamp(0., 1.)).clamp(0., 1.)
            } else {
                p.clamp(0., 1.)
            };
            let width = (k[WEIGHT] + (k[FINAL_WEIGHT] - k[WEIGHT]) * (age / 80.).clamp(0., 1.))
                * if (0. ..80.).contains(&age) {
                    1. + 1.5 * (1. - age / 80.)
                } else {
                    1.
                };
            if pass == 0 && k[AI] >= 0.5 && age >= 0. && age < k[AI_FADE] {
                let v = polar(s.diag * k[AILEN], s.th);
                raster(
                    f.grid,
                    s.c.add(Point(-v.0, -v.1)),
                    s.c.add(v),
                    center,
                    scale,
                    f.palette[i % 5],
                    k[AI_OPACITY] * (1. - age / k[AI_FADE]),
                    0.75 * k[WEIGHT],
                );
            }
            if pass == 1 && reveal == 1 && u > 0. && u < 1. {
                let q = if s.rev { 1. - drawn } else { drawn };
                let a = along(&s.pts, q);
                let b = along(&s.pts, (q + 0.001).min(1.));
                let z = if a.distance(b) > 0. {
                    b
                } else {
                    along(&s.pts, (q - 0.001).max(0.))
                };
                let len = a.distance(z).max(1e-9);
                let bw = (s.diag * 0.3).max(3.);
                let v = Point(-(z.1 - a.1) / len * bw, (z.0 - a.0) / len * bw);
                raster(
                    f.grid,
                    a.add(v),
                    a.add(Point(-v.0, -v.1)),
                    center,
                    scale,
                    f.palette[(i + 1) % 5],
                    1.,
                    1.6 * k[WEIGHT],
                );
            }
            if pass == 2 {
                let partial = reveal == 0 || reveal == 1 || reveal == 3;
                let (from, to) = if !partial {
                    (0., 1.)
                } else if s.rev {
                    (1. - drawn, 1.)
                } else {
                    (0., drawn)
                };
                if to <= from {
                    continue;
                }
                let len: f64 = s.pts.windows(2).map(|p| p[0].distance(p[1])).sum();
                let mut at = 0.;
                for pair in s.pts.windows(2) {
                    let d = pair[0].distance(pair[1]);
                    let lo = (from * len - at).max(0.);
                    let hi = (to * len - at).min(d);
                    if hi > lo && d > 0. {
                        raster(
                            f.grid,
                            transform(pair[0].mix(pair[1], lo / d)),
                            transform(pair[0].mix(pair[1], hi / d)),
                            center,
                            scale,
                            f.palette[i % 5],
                            opacity,
                            width,
                        )
                    }
                    at += d
                }
            }
        }
    }
}
pub(super) fn raster(
    grid: &mut Grid,
    a: Point,
    b: Point,
    center: Point,
    scale: f64,
    color: Color,
    opacity: f64,
    width: f64,
) {
    if opacity <= 0. || grid.is_empty() || grid[0].is_empty() {
        return;
    }
    let a = Point(center.0 + a.0 * scale * 2., center.1 + a.1 * scale);
    let b = Point(center.0 + b.0 * scale * 2., center.1 + b.1 * scale);
    let dx = b.0 - a.0;
    let dy = b.1 - a.1;
    // Liang-Barsky clipping bounds raster work to the visible terminal.
    let (mut lo, mut hi) = (0f64, 1f64);
    for (p, q) in [
        (-dx, a.0),
        (dx, grid[0].len().saturating_sub(1) as f64 - a.0),
        (-dy, a.1),
        (dy, grid.len().saturating_sub(1) as f64 - a.1),
    ] {
        if p == 0. {
            if q < 0. {
                return;
            }
        } else {
            let r = q / p;
            if p < 0. {
                lo = lo.max(r)
            } else {
                hi = hi.min(r)
            }
        }
    }
    if lo > hi {
        return;
    }
    let aa = a.mix(b, lo);
    let bb = a.mix(b, hi);
    let steps = (bb.0 - aa.0).abs().max((bb.1 - aa.1).abs()).ceil().max(1.) as usize;
    let ch = if opacity < 0.2 {
        '.'
    } else if width < 0.7 {
        if dy.abs() < 0.4142 * dx.abs() {
            '_'
        } else {
            ':'
        }
    } else if width > 1.7 {
        '#'
    } else if dy.abs() < 0.4142 * dx.abs() {
        '-'
    } else if dy.abs() > 2.4142 * dx.abs() {
        '|'
    } else if dx * dy >= 0. {
        '\\'
    } else {
        '/'
    };
    let ink = darken(color, ((1. - opacity.clamp(0., 1.)) * 230.) as u8);
    let radius = (width * scale * 0.5).floor().clamp(0., 3.) as isize;
    for i in 0..=steps {
        let p = aa.mix(bb, i as f64 / steps as f64);
        let x = p.0.round() as isize;
        let y = p.1.round() as isize;
        for oy in -radius..=radius {
            for ox in -radius * 2..=radius * 2 {
                let xx = x + ox;
                let yy = y + oy;
                if yy >= 0
                    && xx >= 0
                    && (yy as usize) < grid.len()
                    && (xx as usize) < grid[yy as usize].len()
                {
                    let cell = &mut grid[yy as usize][xx as usize];
                    let cross = cell.ch != ' ' && cell.ch != '.' && cell.ch != ch;
                    *cell = Cell::new(if cross && ch != '#' { '+' } else { ch }, ink)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{SeedableRng, rngs::StdRng};
    fn defaults() -> Vec<f64> {
        all_params().iter().map(|p| p.default as f64).collect()
    }
    fn frame(time: f32, k: &[f64], w: usize, h: usize) -> String {
        let mut grid = vec![vec![Cell::blank(); w]; h];
        let palette = crate::color::make_palette(7);
        let mut rng = StdRng::seed_from_u64(7);
        let strokes = compile(geometry(7, k), 7, k);
        paint(
            &mut ModeFrame {
                grid: &mut grid,
                width: w,
                height: h,
                seed: 7,
                palette: &palette,
                rng: &mut rng,
                time,
                args: &[],
                param_values: None,
            },
            &strokes,
            k,
        );
        crate::render::grid_to_plain(&grid).join("\n")
    }
    #[test]
    fn slice_views() {
        let mut k = defaults();
        insta::assert_snapshot!("slice_seed7_t0", frame(0., &k, 100, 36));
        insta::assert_snapshot!("slice_seed7_t040", frame(0.4, &k, 100, 36));
        insta::assert_snapshot!("slice_seed7_t110", frame(1.1, &k, 100, 36));
        k[REVEAL] = 1.;
        k[CORE] = 1.;
        insta::assert_snapshot!("slice_cut_iris_seed7_t060", frame(0.6, &k, 100, 36));
        k[REVEAL] = 3.;
        k[CORE] = 2.;
        insta::assert_snapshot!("slice_slide_blades_seed7_t060", frame(0.6, &k, 100, 36));
    }
    #[test]
    fn slice_determinism_motion_and_extrema() {
        let k = defaults();
        let a = frame(0.4, &k, 80, 24);
        assert_eq!(a, frame(0.4, &k, 80, 24));
        assert_ne!(a, frame(0.8, &k, 80, 24));
        for max in [false, true] {
            let k: Vec<_> = all_params()
                .iter()
                .map(|p| if max { p.max as f64 } else { p.min as f64 })
                .collect();
            for (w, h) in [(0, 0), (1, 1), (2, 3), (80, 24)] {
                let _ = frame(0.4, &k, w, h);
            }
        }
    }
    #[test]
    fn replay_frames_repeat_and_reverse() {
        let mut k = defaults();
        k[SPREAD] = 430.; // One second including flight and landing hold.
        let forward = frame(0.25, &k, 80, 24);
        assert_eq!(forward, frame(1.25, &k, 80, 24));
        k[DIRECTION] = 1.;
        assert_eq!(forward, frame(0.75, &k, 80, 24));
        k[DIRECTION] = 2.;
        assert_eq!(forward, frame(1.75, &k, 80, 24));
    }
    #[test]
    fn slice_orders_curves_and_eases_are_finite() {
        let mut k = defaults();
        for curve in 0..19 {
            for order in 0..11 {
                k[CURVE] = curve as f64;
                k[ORDER] = order as f64;
                let s = compile(geometry(7, &k), 7, &k);
                assert!(!s.is_empty());
                assert!(s.iter().all(|s| s.start.is_finite()
                    && s.start >= 60.
                    && s.start <= 60. + k[SPREAD] + 1e-6));
            }
        }
        for e in 0..7 {
            k[FEASE] = e as f64;
            for i in 0..=100 {
                assert!(ease(i as f64 / 100., &k).is_finite())
            }
        }
    }
    #[test]
    fn slice_schedule_preserves_source_unquantized_linear() {
        let mut k = defaults();
        k[SHAPE] = 1.;
        let a = compile(geometry(7, &k), 7, &k);
        k[BURST] = 1.;
        let b = compile(geometry(7, &k), 7, &k);
        assert_eq!(
            a.iter().map(|s| s.start).collect::<Vec<_>>(),
            b.iter().map(|s| s.start).collect::<Vec<_>>()
        );
        k[CURVE] = 6.;
        let c = compile(geometry(7, &k), 7, &k);
        assert!(c.iter().all(|s| s.start == 60.));
    }
    #[test]
    fn slice_pose_playback_hold() {
        let mut k = defaults();
        k[RUN] = 0.;
        k[TIME] = 0.5;
        assert_eq!(frame(0., &k, 80, 24), frame(900., &k, 80, 24));
        k[TIME] = 1.;
        assert!(frame(0., &k, 80, 24).chars().any(|c| c == '-' || c == '|'));
    }
}

// Source propertyMotion JSON is data, borrowed for a single deterministic frame.
const SOURCE_KEYS: &[&str] = &[
    "reveal",
    "cut",
    "angle",
    "jit",
    "dist",
    "flight",
    "order",
    "curve",
    "ordN",
    "gain",
    "silence",
    "spread",
    "burst",
    "fease",
    "stretch",
    "os",
    "ailen",
    "ai",
    "aiOpacity",
    "aiFade",
    "weight",
    "finalWeight",
    "time",
    "speed",
    "run",
    "core",
    "coreSides",
    "coreSize",
    "coreTurn",
    "coreFrame",
    "shape",
    "loop",
];
fn choices(i: usize) -> &'static [&'static str] {
    match i {
        REVEAL => &["draw", "cut", "slide", "draw+slide", "glow"],
        ORDER => &[
            "sweep",
            "radial",
            "radial-in",
            "path",
            "subpath",
            "golden",
            "golden-angle",
            "vdc",
            "spectral",
            "hilbert",
            "random",
        ],
        CURVE => &[
            "linear",
            "ease-in",
            "ease-out",
            "ease-in-out",
            "expo",
            "sine",
            "steps",
            "white",
            "smooth-N",
            "fourier",
            "logistic",
            "lorenz",
            "poisson",
            "pink",
            "red",
            "blue",
            "violet",
            "gamma",
            "levy",
        ],
        FEASE => &[
            "back",
            "spring",
            "cubic-out",
            "expo-out",
            "elastic",
            "linear",
            "cut",
        ],
        CORE => &["original", "iris", "blades", "lattice"],
        CORE_FRAME => &["open", "circle", "polygon"],
        _ => &[],
    }
}
fn pool(i: usize) -> &'static [&'static str] {
    match i {
        REVEAL => &["draw", "cut", "cut", "slide", "draw+slide", "glow"],
        ORDER => &[
            "sweep",
            "radial",
            "radial-in",
            "path",
            "subpath",
            "golden",
            "golden",
            "golden-angle",
            "vdc",
            "spectral",
            "spectral",
            "hilbert",
        ],
        _ => choices(i),
    }
}
fn numeric(v: &serde_json::Value, default: f64) -> f64 {
    v.as_f64().filter(|v| v.is_finite()).unwrap_or(default)
}
fn string<'a>(v: &'a serde_json::Value, default: &'a str) -> &'a str {
    v.as_str().unwrap_or(default)
}
fn merge_json(base: &serde_json::Value, over: &serde_json::Value) -> serde_json::Value {
    let mut out = base.as_object().cloned().unwrap_or_default();
    if let Some(o) = over.as_object() {
        for (k, v) in o {
            out.insert(k.clone(), v.clone());
        }
    }
    serde_json::Value::Object(out)
}
pub(super) fn fnv(s: &str) -> u32 {
    s.encode_utf16()
        .fold(0x811c9dc5, |h, c| (h ^ c as u32).wrapping_mul(0x01000193))
}
fn replay_position(clock: f64, duration: f64, looping: bool, direction: usize) -> f64 {
    // Reconstruct the cycle directly from elapsed time; no frame history persists.
    let duration = duration.max(1.);
    let clock = clock.max(0.);
    let phase = if looping {
        clock.rem_euclid(duration)
    } else {
        clock.min(duration)
    };
    match direction {
        1 => duration - phase,
        2 if looping && (clock / duration).floor() as u64 % 2 == 1 => duration - phase,
        _ => phase,
    }
}

fn motion_clock(config: &serde_json::Value, seconds: f64) -> f64 {
    let timing = &config["timing"];
    let duration = numeric(&timing["duration"], 4000.);
    let span = duration
        * if timing["direction"] == "alternate" {
            2.
        } else {
            1.
        }
        + numeric(&timing["delay"], 0.);
    let t = &config["transport"];
    let time = numeric(&t["time"], 0.).clamp(0., 1.) * span
        + if t["run"].as_bool().unwrap_or(false) {
            seconds * 1000. * numeric(&t["speed"], 1.)
        } else {
            0.
        };
    replay_position(
        time,
        span,
        timing["loop"].as_bool().unwrap_or(true),
        numeric(&t["direction"], 0.).round() as usize,
    )
}
fn easing_fraction(t: f64, name: &str) -> f64 {
    match name {
        "inOutSine" => -(PI * t).cos() / 2. + 0.5,
        "inOutQuad" => {
            if t < 0.5 {
                2. * t * t
            } else {
                1. - (-2. * t + 2.).powi(2) / 2.
            }
        }
        _ => t,
    }
}
fn variation_time(ms: f64, timing: &serde_json::Value) -> Option<f64> {
    let delay = numeric(&timing["delay"], 0.);
    if ms < delay {
        return None;
    }
    let duration = numeric(&timing["duration"], 4000.).max(1.);
    let mut time = (ms - delay).max(0.);
    if !timing["loop"].as_bool().unwrap_or(true) {
        time = time.min(duration)
    }
    let cycle = (time / duration).floor();
    time = (cycle + easing_fraction(time / duration - cycle, string(&timing["easing"], "linear")))
        * duration;
    match string(&timing["direction"], "normal") {
        "reverse" => time = duration - time,
        "alternate" => {
            let cycle = (time / duration).floor();
            let fraction = time / duration - cycle;
            time = if cycle as i64 % 2 != 0 {
                1. - fraction
            } else {
                fraction
            } * duration
        }
        _ => {}
    }
    Some(time)
}
fn distribution(name: &str, r: &mut Random) -> f64 {
    match name {
        "normal" => (0.5 + r.normal() / 6.).clamp(0., 1.),
        "triangular" => (r.next() + r.next()) / 2.,
        "arcsine" => (1. - (PI * r.next()).cos()) / 2.,
        "exponential" => (-(1. - r.next()).ln() / 4.).min(1.),
        _ => r.next(),
    }
}
fn sample_variation(v: &serde_json::Value, ms: f64, key: u32) -> f64 {
    let seed = numeric(&v["seed"], 7.) as u32;
    let mut rng = Random(seed ^ key);
    let count = numeric(&v["harmonics"], 3.).round().max(1.) as usize;
    let harmonic = 1 + rng.pick(count);
    let phase = rng.next() * numeric(&v["phase"], 1.);
    let ep = rng.next();
    let u = (ms / numeric(&v["period"], 4000.).max(50.)).rem_euclid(1.);
    let envelope = (1. - (TAU * (u + ep)).cos()) / 2.;
    let depth = numeric(&v["depth"], 0.35);
    let mix = (numeric(&v["mix"], 0.25) + (envelope - 0.5) * depth).clamp(0., 1.);
    let mode = string(&v["mode"], "harmonic");
    let knots = (harmonic * 2).max(2);
    let target = |index: usize| {
        let mut r = Random(seed ^ key ^ (index as u32 + 1).wrapping_mul(0x9e3779b1));
        let blend = if mode == "harmonic" {
            mix
        } else {
            (numeric(&v["mix"], 0.25)
                - (TAU * (index as f64 / knots as f64 + ep)).cos() * depth / 2.)
                .clamp(0., 1.)
        };
        distribution(string(&v["distribution"], "normal"), &mut r) * (1. - blend)
            + distribution(string(&v["secondary"], "arcsine"), &mut r) * blend
    };
    let cycles = if mode == "harmonic" { harmonic } else { knots };
    let position = (u * cycles as f64 + phase).rem_euclid(cycles as f64);
    let index = position.floor() as usize;
    let fraction = position - index as f64;
    let value = if mode == "harmonic" {
        let amplitude = (0.25 + 0.75 * target(0)) * (1. - depth * (1. - envelope) * 0.75);
        0.5 + (TAU * position).sin() * amplitude / 2.
    } else if mode == "hold" {
        target(index)
    } else {
        let t = fraction * fraction * (3. - 2. * fraction);
        target(index) * (1. - t) + target((index + 1) % knots) * t
    };
    let lo = numeric(&v["min"], 0.);
    lo + (numeric(&v["max"], 1.) - lo) * value.clamp(0., 1.)
}
fn quantize(i: usize, v: f64) -> f64 {
    let p = &PARAMS[i];
    let step = if i == CUT || i == ANGLE || i == JIT || i == CORE_SIDES || i == CORE_TURN {
        1.
    } else {
        p.step as f64
    };
    (p.min as f64 + ((v - p.min as f64) / step).round() * step).clamp(p.min as f64, p.max as f64)
}
fn field_value(i: usize, v: &serde_json::Value, base: f64) -> f64 {
    if let Some(s) = v.as_str() {
        choices(i)
            .iter()
            .position(|x| *x == s)
            .map(|x| x as f64)
            .unwrap_or(base)
    } else if let Some(b) = v.as_bool() {
        if b { 1. } else { 0. }
    } else {
        numeric(v, base)
    }
}
#[derive(Clone)]
struct Track {
    index: usize,
    timing: serde_json::Value,
    frames: Vec<(f64, serde_json::Value)>,
    variation: Option<serde_json::Value>,
    stroke: bool,
}
fn tracks(config: Option<&serde_json::Value>, k: &[f64], section_name: &str) -> Vec<Track> {
    let Some(config) = config else { return vec![] };
    let section = &config["sections"][section_name];
    let timing = merge_json(&config["timing"], &section["timing"]);
    let Some(fields) = section["fields"].as_object() else {
        return vec![];
    };
    let mut out = vec![];
    for (name, track) in fields {
        let Some(i) = SOURCE_KEYS.iter().position(|x| x == name) else {
            continue;
        };
        if !track["enabled"].as_bool().unwrap_or(false) || track["variationPolicy"] == "none" {
            continue;
        }
        let timing = merge_json(&timing, &track["timing"]);
        let policy = string(
            &track["variationPolicy"],
            if track["variation"].is_object() {
                "cascade"
            } else {
                "none"
            },
        );
        let variation = if policy == "none" {
            None
        } else {
            let p = &PARAMS[i];
            let select = !choices(i).is_empty();
            let (lo, hi, middle) = if select || (i == AI || i == RUN || i == LOOP) {
                (0., 1., 0.5)
            } else {
                (p.min as f64, p.max as f64, k[i])
            };
            let mut v = serde_json::json!({"min":lo.max(middle-(hi-lo)/4.),"max":hi.min(middle+(hi-lo)/4.),"period":numeric(&timing["duration"],4000.)});
            v = merge_json(&v, &config["variation"]);
            if policy == "cascade" {
                v = merge_json(&v, &section["variation"]);
                v = merge_json(&v, &track["variation"])
            }
            Some(v)
        };
        let stroke = variation.as_ref().is_some_and(|v| v["scope"] == "stroke")
            && [
                WEIGHT,
                FINAL_WEIGHT,
                AILEN,
                AI_OPACITY,
                AI_FADE,
                STRETCH,
                OS,
            ]
            .contains(&i);
        let mut frames: Vec<_> = track["frames"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|v| {
                v["at"]
                    .as_f64()
                    .filter(|v| v.is_finite())
                    .map(|at| (at, v["value"].clone()))
            })
            .collect();
        frames.sort_by(|a, b| a.0.total_cmp(&b.0));
        out.push(Track {
            index: i,
            timing,
            frames,
            variation,
            stroke,
        })
    }
    out
}
fn track_value(track: &Track, base: f64, ms: f64, identity: u32) -> f64 {
    let i = track.index;
    if let Some(v) = &track.variation {
        if let Some(time) = variation_time(ms, &track.timing) {
            let value = sample_variation(v, time, identity);
            return if !choices(i).is_empty() {
                let pool = pool(i);
                let choice = pool
                    [((value * pool.len() as f64).floor().max(0.) as usize).min(pool.len() - 1)];
                choices(i).iter().position(|x| *x == choice).unwrap_or(0) as f64
            } else if i == AI || i == RUN || i == LOOP {
                if value >= 0.5 { 1. } else { 0. }
            } else {
                quantize(i, value)
            };
        }
    }
    if track.frames.is_empty() || ms < numeric(&track.timing["delay"], 0.) {
        return base;
    }
    let duration = numeric(&track.timing["duration"], 4000.).max(1.);
    let elapsed = (ms - numeric(&track.timing["delay"], 0.)).max(0.);
    let cycle = (elapsed / duration).floor();
    let looping = track.timing["loop"].as_bool().unwrap_or(true);
    let mut t = if looping {
        elapsed / duration - cycle
    } else {
        (elapsed / duration).min(1.)
    };
    let direction = string(&track.timing["direction"], "normal");
    if direction == "reverse" || (direction == "alternate" && cycle as i64 % 2 != 0 && looping) {
        t = 1. - t
    }
    let mut a = &track.frames[0];
    let mut b = track.frames.last().unwrap();
    if t < a.0 {
        b = a
    } else {
        for f in track.frames.iter().skip(1) {
            if t < f.0 {
                b = f;
                break;
            }
            a = f
        }
    }
    if !choices(i).is_empty() || i == AI || i == RUN || i == LOOP {
        return field_value(i, &a.1, base);
    }
    let fraction = if b.0 > a.0 {
        ((t - a.0) / (b.0 - a.0)).clamp(0., 1.)
    } else {
        0.
    };
    let u = easing_fraction(fraction, string(&track.timing["easing"], "linear"));
    let av = field_value(i, &a.1, base);
    let bv = field_value(i, &b.1, base);
    quantize(i, av + (bv - av) * u)
}

fn stroke_identity(s: &Stroke) -> u32 {
    let mut path = String::new();
    for (i, p) in s.pts.iter().enumerate() {
        path.push_str(&format!(
            "{}{} {}",
            if i == 0 { "M" } else { "L" },
            (p.0 * 100.).round() / 100.,
            (p.1 * 100.).round() / 100.
        ));
    }
    fnv(&format!("{}|{}|{}|{}", path, s.sub + 1, s.c.0, s.c.1))
}

const MOTION_PARAMS: &[Param] = &[
    param!("SL2_M_TIME", "motion saved position", 0., 1., 0., 0.001),
    param!("SL2_M_SPEED", "motion tempo", 0.1, 2., 1., 0.1),
    param!("SL2_M_RUN", "motion clock run", 0., 1., 1., 1.),
    param!(
        "SL2_M1_TARGET",
        "m1 target",
        0.0,
        29.0,
        20.0,
        1.0,
        &[
            "reveal",
            "cut",
            "angle",
            "jit",
            "dist",
            "flight",
            "order",
            "curve",
            "ordN",
            "gain",
            "silence",
            "spread",
            "burst",
            "fease",
            "stretch",
            "os",
            "ailen",
            "ai",
            "aiOpacity",
            "aiFade",
            "weight",
            "finalWeight",
            "time",
            "speed",
            "run",
            "core",
            "coreSides",
            "coreSize",
            "coreTurn",
            "coreFrame"
        ]
    ),
    param!(
        "SL2_M1_SCOPE",
        "m1 scope",
        0.0,
        1.0,
        1.0,
        1.0,
        &["input", "stroke"]
    ),
    param!(
        "SL2_M1_MODE",
        "m1 mode",
        0.0,
        4.0,
        0.0,
        1.0,
        &["off", "keyframes", "harmonic", "drift", "hold"]
    ),
    param!("SL2_M1_LOW", "motion1 low normalized", 0.0, 1.0, 0.25, 0.01),
    param!(
        "SL2_M1_HIGH",
        "motion1 high normalized",
        0.0,
        1.0,
        0.75,
        0.01
    ),
    param!(
        "SL2_M1_PERIOD",
        "motion1 period ms",
        50.0,
        16000.0,
        4000.0,
        50.0
    ),
    param!("SL2_M1_DELAY", "motion1 delay ms", 0.0, 8000.0, 0.0, 50.0),
    param!(
        "SL2_M1_EASING",
        "m1 easing",
        0.0,
        2.0,
        0.0,
        1.0,
        &["linear", "inOutSine", "inOutQuad"]
    ),
    param!(
        "SL2_M1_DIRECTION",
        "m1 direction",
        0.0,
        2.0,
        0.0,
        1.0,
        &["normal", "reverse", "alternate"]
    ),
    param!("SL2_M1_LOOP", "motion1 repeat", 0.0, 1.0, 1.0, 1.0),
    param!(
        "SL2_M1_DIST",
        "m1 dist",
        0.0,
        4.0,
        0.0,
        1.0,
        &["normal", "uniform", "triangular", "arcsine", "exponential"]
    ),
    param!(
        "SL2_M1_SECONDARY",
        "m1 secondary",
        0.0,
        4.0,
        3.0,
        1.0,
        &["normal", "uniform", "triangular", "arcsine", "exponential"]
    ),
    param!(
        "SL2_M1_MIX",
        "motion1 distribution mix",
        0.0,
        1.0,
        0.25,
        0.01
    ),
    param!(
        "SL2_M1_DEPTH",
        "motion1 envelope depth",
        0.0,
        1.0,
        0.35,
        0.01
    ),
    param!("SL2_M1_HARMONICS", "motion1 harmonics", 1.0, 8.0, 3.0, 1.0),
    param!("SL2_M1_PHASE", "motion1 phase spread", 0.0, 1.0, 1.0, 0.01),
    param!(
        "SL2_M1_SEED",
        "motion1 variation seed",
        0.0,
        65535.0,
        7.0,
        1.0
    ),
    param!(
        "SL2_M2_TARGET",
        "m2 target",
        0.0,
        29.0,
        18.0,
        1.0,
        &[
            "reveal",
            "cut",
            "angle",
            "jit",
            "dist",
            "flight",
            "order",
            "curve",
            "ordN",
            "gain",
            "silence",
            "spread",
            "burst",
            "fease",
            "stretch",
            "os",
            "ailen",
            "ai",
            "aiOpacity",
            "aiFade",
            "weight",
            "finalWeight",
            "time",
            "speed",
            "run",
            "core",
            "coreSides",
            "coreSize",
            "coreTurn",
            "coreFrame"
        ]
    ),
    param!(
        "SL2_M2_SCOPE",
        "m2 scope",
        0.0,
        1.0,
        1.0,
        1.0,
        &["input", "stroke"]
    ),
    param!(
        "SL2_M2_MODE",
        "m2 mode",
        0.0,
        4.0,
        0.0,
        1.0,
        &["off", "keyframes", "harmonic", "drift", "hold"]
    ),
    param!("SL2_M2_LOW", "motion2 low normalized", 0.0, 1.0, 0.25, 0.01),
    param!(
        "SL2_M2_HIGH",
        "motion2 high normalized",
        0.0,
        1.0,
        0.75,
        0.01
    ),
    param!(
        "SL2_M2_PERIOD",
        "motion2 period ms",
        50.0,
        16000.0,
        4000.0,
        50.0
    ),
    param!("SL2_M2_DELAY", "motion2 delay ms", 0.0, 8000.0, 0.0, 50.0),
    param!(
        "SL2_M2_EASING",
        "m2 easing",
        0.0,
        2.0,
        0.0,
        1.0,
        &["linear", "inOutSine", "inOutQuad"]
    ),
    param!(
        "SL2_M2_DIRECTION",
        "m2 direction",
        0.0,
        2.0,
        0.0,
        1.0,
        &["normal", "reverse", "alternate"]
    ),
    param!("SL2_M2_LOOP", "motion2 repeat", 0.0, 1.0, 1.0, 1.0),
    param!(
        "SL2_M2_DIST",
        "m2 dist",
        0.0,
        4.0,
        0.0,
        1.0,
        &["normal", "uniform", "triangular", "arcsine", "exponential"]
    ),
    param!(
        "SL2_M2_SECONDARY",
        "m2 secondary",
        0.0,
        4.0,
        3.0,
        1.0,
        &["normal", "uniform", "triangular", "arcsine", "exponential"]
    ),
    param!(
        "SL2_M2_MIX",
        "motion2 distribution mix",
        0.0,
        1.0,
        0.25,
        0.01
    ),
    param!(
        "SL2_M2_DEPTH",
        "motion2 envelope depth",
        0.0,
        1.0,
        0.35,
        0.01
    ),
    param!("SL2_M2_HARMONICS", "motion2 harmonics", 1.0, 8.0, 3.0, 1.0),
    param!("SL2_M2_PHASE", "motion2 phase spread", 0.0, 1.0, 1.0, 0.01),
    param!(
        "SL2_M2_SEED",
        "motion2 variation seed",
        0.0,
        65535.0,
        7.0,
        1.0
    ),
    param!(
        "SL2_M3_TARGET",
        "m3 target",
        0.0,
        29.0,
        19.0,
        1.0,
        &[
            "reveal",
            "cut",
            "angle",
            "jit",
            "dist",
            "flight",
            "order",
            "curve",
            "ordN",
            "gain",
            "silence",
            "spread",
            "burst",
            "fease",
            "stretch",
            "os",
            "ailen",
            "ai",
            "aiOpacity",
            "aiFade",
            "weight",
            "finalWeight",
            "time",
            "speed",
            "run",
            "core",
            "coreSides",
            "coreSize",
            "coreTurn",
            "coreFrame"
        ]
    ),
    param!(
        "SL2_M3_SCOPE",
        "m3 scope",
        0.0,
        1.0,
        1.0,
        1.0,
        &["input", "stroke"]
    ),
    param!(
        "SL2_M3_MODE",
        "m3 mode",
        0.0,
        4.0,
        0.0,
        1.0,
        &["off", "keyframes", "harmonic", "drift", "hold"]
    ),
    param!("SL2_M3_LOW", "motion3 low normalized", 0.0, 1.0, 0.25, 0.01),
    param!(
        "SL2_M3_HIGH",
        "motion3 high normalized",
        0.0,
        1.0,
        0.75,
        0.01
    ),
    param!(
        "SL2_M3_PERIOD",
        "motion3 period ms",
        50.0,
        16000.0,
        4000.0,
        50.0
    ),
    param!("SL2_M3_DELAY", "motion3 delay ms", 0.0, 8000.0, 0.0, 50.0),
    param!(
        "SL2_M3_EASING",
        "m3 easing",
        0.0,
        2.0,
        0.0,
        1.0,
        &["linear", "inOutSine", "inOutQuad"]
    ),
    param!(
        "SL2_M3_DIRECTION",
        "m3 direction",
        0.0,
        2.0,
        0.0,
        1.0,
        &["normal", "reverse", "alternate"]
    ),
    param!("SL2_M3_LOOP", "motion3 repeat", 0.0, 1.0, 1.0, 1.0),
    param!(
        "SL2_M3_DIST",
        "m3 dist",
        0.0,
        4.0,
        0.0,
        1.0,
        &["normal", "uniform", "triangular", "arcsine", "exponential"]
    ),
    param!(
        "SL2_M3_SECONDARY",
        "m3 secondary",
        0.0,
        4.0,
        3.0,
        1.0,
        &["normal", "uniform", "triangular", "arcsine", "exponential"]
    ),
    param!(
        "SL2_M3_MIX",
        "motion3 distribution mix",
        0.0,
        1.0,
        0.25,
        0.01
    ),
    param!(
        "SL2_M3_DEPTH",
        "motion3 envelope depth",
        0.0,
        1.0,
        0.35,
        0.01
    ),
    param!("SL2_M3_HARMONICS", "motion3 harmonics", 1.0, 8.0, 3.0, 1.0),
    param!("SL2_M3_PHASE", "motion3 phase spread", 0.0, 1.0, 1.0, 0.01),
    param!(
        "SL2_M3_SEED",
        "motion3 variation seed",
        0.0,
        65535.0,
        7.0,
        1.0
    ),
    param!(
        "SL2_M4_TARGET",
        "m4 target",
        0.0,
        29.0,
        14.0,
        1.0,
        &[
            "reveal",
            "cut",
            "angle",
            "jit",
            "dist",
            "flight",
            "order",
            "curve",
            "ordN",
            "gain",
            "silence",
            "spread",
            "burst",
            "fease",
            "stretch",
            "os",
            "ailen",
            "ai",
            "aiOpacity",
            "aiFade",
            "weight",
            "finalWeight",
            "time",
            "speed",
            "run",
            "core",
            "coreSides",
            "coreSize",
            "coreTurn",
            "coreFrame"
        ]
    ),
    param!(
        "SL2_M4_SCOPE",
        "m4 scope",
        0.0,
        1.0,
        1.0,
        1.0,
        &["input", "stroke"]
    ),
    param!(
        "SL2_M4_MODE",
        "m4 mode",
        0.0,
        4.0,
        0.0,
        1.0,
        &["off", "keyframes", "harmonic", "drift", "hold"]
    ),
    param!("SL2_M4_LOW", "motion4 low normalized", 0.0, 1.0, 0.25, 0.01),
    param!(
        "SL2_M4_HIGH",
        "motion4 high normalized",
        0.0,
        1.0,
        0.75,
        0.01
    ),
    param!(
        "SL2_M4_PERIOD",
        "motion4 period ms",
        50.0,
        16000.0,
        4000.0,
        50.0
    ),
    param!("SL2_M4_DELAY", "motion4 delay ms", 0.0, 8000.0, 0.0, 50.0),
    param!(
        "SL2_M4_EASING",
        "m4 easing",
        0.0,
        2.0,
        0.0,
        1.0,
        &["linear", "inOutSine", "inOutQuad"]
    ),
    param!(
        "SL2_M4_DIRECTION",
        "m4 direction",
        0.0,
        2.0,
        0.0,
        1.0,
        &["normal", "reverse", "alternate"]
    ),
    param!("SL2_M4_LOOP", "motion4 repeat", 0.0, 1.0, 1.0, 1.0),
    param!(
        "SL2_M4_DIST",
        "m4 dist",
        0.0,
        4.0,
        0.0,
        1.0,
        &["normal", "uniform", "triangular", "arcsine", "exponential"]
    ),
    param!(
        "SL2_M4_SECONDARY",
        "m4 secondary",
        0.0,
        4.0,
        3.0,
        1.0,
        &["normal", "uniform", "triangular", "arcsine", "exponential"]
    ),
    param!(
        "SL2_M4_MIX",
        "motion4 distribution mix",
        0.0,
        1.0,
        0.25,
        0.01
    ),
    param!(
        "SL2_M4_DEPTH",
        "motion4 envelope depth",
        0.0,
        1.0,
        0.35,
        0.01
    ),
    param!("SL2_M4_HARMONICS", "motion4 harmonics", 1.0, 8.0, 3.0, 1.0),
    param!("SL2_M4_PHASE", "motion4 phase spread", 0.0, 1.0, 1.0, 0.01),
    param!(
        "SL2_M4_SEED",
        "motion4 variation seed",
        0.0,
        65535.0,
        7.0,
        1.0
    ),
    param!(
        "SL2_DIRECTION",
        "playback direction",
        0.,
        2.,
        0.,
        1.,
        &["forward", "reverse", "alternate"]
    ),
];
pub(super) fn all_params() -> &'static [Param] {
    static P: std::sync::OnceLock<Vec<Param>> = std::sync::OnceLock::new();
    P.get_or_init(|| {
        PARAMS
            .iter()
            .chain(MOTION_PARAMS)
            .map(|p| {
                let mut p = *p;
                if matches!(
                    p.key,
                    "SL2_TIME"
                        | "SL2_SPEED"
                        | "SL2_RUN"
                        | "SL2_LOOP"
                        | "SL2_DIRECTION"
                        | "SL2_M_TIME"
                        | "SL2_M_SPEED"
                        | "SL2_M_RUN"
                ) {
                    p.randomize = false;
                }
                p
            })
            .collect()
    })
}
fn motion_config(k: &[f64]) -> serde_json::Value {
    let v = |i: usize| {
        k.get(PARAMS.len() + i)
            .copied()
            .unwrap_or(MOTION_PARAMS[i].default as f64)
    };
    let mut fields = serde_json::Map::new();
    let mut span = 4000f64;
    for lane in 0..4 {
        let b = 3 + lane * 17;
        let target = v(b).round() as usize;
        let mode = v(b + 2).round() as usize;
        if mode == 0 {
            continue;
        }
        let p = &PARAMS[target];
        let select = !choices(target).is_empty();
        let is_bool = [AI, RUN, LOOP].contains(&target);
        let (lo, hi) = if select || is_bool {
            (0., 1.)
        } else {
            (p.min as f64, p.max as f64)
        };
        let low = lo + (hi - lo) * v(b + 3);
        let high = lo + (hi - lo) * v(b + 4);
        let period = v(b + 5);
        let delay = v(b + 6);
        let direction = ["normal", "reverse", "alternate"][v(b + 8).round() as usize];
        span = span.max(period * if direction == "alternate" { 2. } else { 1. } + delay);
        let timing = serde_json::json!({"duration":period,"delay":delay,"easing":(["linear","inOutSine","inOutQuad"][v(b+7).round() as usize]),"direction":direction,"loop":v(b+9)>=0.5});
        let field_json = |value: f64| {
            if select {
                let pool = pool(target);
                serde_json::json!(
                    pool[((value * pool.len() as f64).floor().max(0.) as usize)
                        .min(pool.len() - 1)]
                )
            } else if is_bool {
                serde_json::json!(value >= 0.5)
            } else {
                serde_json::json!(value)
            }
        };
        let track = if mode == 1 {
            serde_json::json!({"enabled":true,"timing":timing,"frames":[{"at":0,"value":field_json(low)},{"at":1,"value":field_json(high)}]})
        } else {
            let dist = ["normal", "uniform", "triangular", "arcsine", "exponential"];
            serde_json::json!({"enabled":true,"timing":timing,"variationPolicy":"cascade","variation":{"scope":if v(b+1)>=0.5{"stroke"}else{"input"},"mode":(["harmonic","drift","hold"][mode-2]),"min":low,"max":high,"period":period,"distribution":dist[v(b+10).round() as usize],"secondary":dist[v(b+11).round() as usize],"mix":v(b+12),"depth":v(b+13),"harmonics":v(b+14),"phase":v(b+15),"seed":v(b+16)}})
        };
        fields.insert(SOURCE_KEYS[target].into(), track);
    }
    serde_json::json!({"transport":{"time":v(0),"speed":v(1),"run":v(2)>=0.5,"direction":v(71)},"timing":{"duration":span,"loop":k[LOOP]>=0.5},"sections":{"slice":{"fields":fields}}})
}

pub(super) fn animate_inputs(k: &mut [f64], time: f32) {
    let config = motion_config(k);
    let motion = tracks(Some(&config), k, "slice");
    let ms = motion_clock(&config, time as f64);
    for track in motion.iter().filter(|t| !t.stroke) {
        k[track.index] = track_value(track, k[track.index], ms, fnv(SOURCE_KEYS[track.index]));
    }
}

pub(super) fn animation_params() -> &'static [Param] {
    static P: std::sync::OnceLock<Vec<Param>> = std::sync::OnceLock::new();
    P.get_or_init(|| {
        all_params()
            .iter()
            .enumerate()
            .filter(|(i, _)| !(25..31).contains(i))
            .map(|(_, p)| {
                let mut p = *p;
                if p.key.ends_with("_TARGET") {
                    p.max = 24.;
                    p.choices = &SOURCE_KEYS[..25];
                }
                p
            })
            .collect()
    })
}
pub(super) fn animation_values(v: &[f64]) -> Vec<f64> {
    let mut k: Vec<_> = all_params().iter().map(|p| p.default as f64).collect();
    let mut source = 0;
    for (i, x) in k.iter_mut().enumerate() {
        if (25..31).contains(&i) {
            continue;
        }
        if let Some(value) = v.get(source) {
            *x = *value
        }
        source += 1
    }
    k
}

#[cfg(test)]
mod motion_tests {
    use super::*;
    #[test]
    fn random_exploration_retains_tuned_transport() {
        let spec = crate::registry::ModeSpec {
            animate: AnimKind::Iterate,
            params: all_params(),
        };
        let mut values: Vec<_> = spec.params.iter().map(|p| p.default).collect();
        values[SPEED] = 1.5;
        values[DIRECTION] = 1.;
        let pins = vec![false; values.len() + 1];
        let first = crate::opts::effective_pvals(&spec, &values, 42, true, 0, &pins);
        for roll in 1..64 {
            let rolled = crate::opts::effective_pvals(&spec, &values, 42, true, roll, &pins);
            assert_ne!(rolled, first);
            for (i, p) in spec.params.iter().enumerate().filter(|(_, p)| !p.randomize) {
                assert_eq!(rolled[i], values[i], "{} roll={roll}", p.key);
            }
        }
    }
    #[test]
    fn replay_loops_reverse_and_alternate() {
        assert_eq!(
            [0., 250., 1000., 1250.].map(|t| replay_position(t, 1000., true, 0)),
            [0., 250., 0., 250.]
        );
        assert_eq!(
            [0., 250., 1000., 1250.].map(|t| replay_position(t, 1000., true, 1)),
            [1000., 750., 1000., 750.]
        );
        assert_eq!(
            [0., 250., 1000., 1250., 2000.].map(|t| replay_position(t, 1000., true, 2)),
            [0., 250., 1000., 750., 0.]
        );
        assert_eq!(replay_position(1250., 1000., false, 0), 1000.);
        assert_eq!(replay_position(1250., 1000., false, 1), 0.);
    }
    fn defaults() -> Vec<f64> {
        all_params().iter().map(|p| p.default as f64).collect()
    }
    #[test]
    fn motion_keyframes_interpolate_hold_and_reverse() {
        let mut k = defaults();
        let b = PARAMS.len() + 3;
        k[b] = ANGLE as f64;
        k[b + 2] = 1.;
        k[b + 3] = 0.;
        k[b + 4] = 1.;
        k[b + 5] = 1000.;
        k[b + 9] = 0.;
        let cfg = motion_config(&k);
        let tr = tracks(Some(&cfg), &k, "slice");
        assert_eq!(tr.len(), 1);
        assert_eq!(track_value(&tr[0], 215., 250., 0), 90.);
        assert_eq!(track_value(&tr[0], 215., 1500., 0), 360.);
        k[b + 8] = 1.;
        let tr = tracks(Some(&motion_config(&k)), &k, "slice");
        assert_eq!(track_value(&tr[0], 215., 250., 0), 270.);
    }
    #[test]
    fn motion_variation_modes_repeat_random_access() {
        let base = serde_json::json!({"min":0.4,"max":2.5,"period":1200,"distribution":"normal","secondary":"arcsine","mix":0.25,"depth":0.35,"harmonics":3,"phase":1,"seed":7});
        for mode in ["harmonic", "drift", "hold"] {
            let v = merge_json(&base, &serde_json::json!({"mode":mode}));
            let a = sample_variation(&v, 350., 123);
            assert_eq!(a, sample_variation(&v, 350., 123));
            assert!((a - sample_variation(&v, 1550., 123)).abs() < 1e-12);
            assert!((0.4..=2.5).contains(&a));
        }
    }
    #[test]
    fn motion_independent_stroke_tracks_and_input_rounding() {
        let mut k = defaults();
        for (lane, target) in [WEIGHT, AI_OPACITY, AI_FADE, STRETCH]
            .into_iter()
            .enumerate()
        {
            let b = PARAMS.len() + 3 + lane * 17;
            k[b] = target as f64;
            k[b + 2] = 2.;
        }
        let tr = tracks(Some(&motion_config(&k)), &k, "slice");
        assert_eq!(tr.len(), 4);
        assert!(tr.iter().all(|t| t.stroke));
        let values: Vec<_> = tr
            .iter()
            .map(|t| track_value(t, k[t.index], 733., 17))
            .collect();
        assert!(values.iter().all(|v| v.is_finite()));
        let a = values;
        let b: Vec<_> = tr
            .iter()
            .map(|t| track_value(t, k[t.index], 733., 99))
            .collect();
        assert_ne!(a, b);
    }
    #[test]
    fn motion_delay_and_alternate_boundaries() {
        let t =
            serde_json::json!({"duration":1000,"delay":200,"direction":"alternate","loop":true});
        assert_eq!(variation_time(199., &t), None);
        assert_eq!(variation_time(200., &t), Some(0.));
        assert_eq!(variation_time(1200., &t), Some(1000.));
        assert_eq!(variation_time(1700., &t), Some(500.));
    }
    #[test]
    fn all_live_keys_are_unique_and_defaults_resolve() {
        let mut keys = std::collections::BTreeSet::new();
        for p in all_params() {
            assert!(keys.insert(p.key));
            assert!(p.min <= p.default && p.default <= p.max);
        }
        assert_eq!(all_params().len(), 104);
        assert_eq!(animation_params().len(), 98);
        assert!(
            animation_params()
                .iter()
                .filter(|p| p.key.ends_with("_TARGET"))
                .all(|p| p.max == 24. && p.choices.len() == 25)
        );
        assert_eq!(
            animation_values(
                &animation_params()
                    .iter()
                    .map(|p| p.default as f64)
                    .collect::<Vec<_>>()
            ),
            defaults()
        );
    }
}

fn along_table(path: &[Point], d: &[f64], target: f64) -> Point {
    let hi = d.partition_point(|v| *v < target).min(d.len() - 1).max(1);
    let lo = hi - 1;
    path[lo].mix(
        path[hi],
        if d[hi] > d[lo] {
            ((target - d[lo]) / (d[hi] - d[lo])).clamp(0., 1.)
        } else {
            0.
        },
    )
}
