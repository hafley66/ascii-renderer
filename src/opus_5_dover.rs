//! opus-5-dover -- the shelf of unread Dover paperbacks, one spread per chapter:
//! Fourier epicycles, a curved surface, a graph waking up, a lambda term
//! reducing, a one-sided band, and a matrix bending the plane.
use crate::_0_profile::measure_layer;
use crate::color::*;
use crate::opts::param_f32;
use crate::types::*;
use crossterm::style::Color;
use rand::RngExt;
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::cell::RefCell;
use std::f32::consts::{PI, TAU};

const CHAPTERS: usize = 6;
const TITLES: [&str; CHAPTERS] = [
    "I. fourier analysis",
    "II. differential geometry",
    "III. graph theory",
    "IV. lambda calculus",
    "V. topology",
    "VI. linear algebra",
];

pub(crate) struct Opus5DoverKnobs {
    pub chapter: f32,
    pub dwell: f32,
    pub speed: f32,
    pub harm: f32,
    pub nodes: f32,
    pub chords: f32,
    pub steps: f32,
    pub twist: f32,
    pub tube: f32,
    pub mesh: f32,
    pub trail: f32,
    pub label: f32,
    pub aspect: f32,
}

impl Opus5DoverKnobs {
    #[cfg_attr(
        feature = "function-trace",
        tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)
    )]
    pub(crate) fn from_env() -> Self {
        Opus5DoverKnobs {
            chapter: param_f32("CHAPTER", -1.0),
            dwell: param_f32("DWELL", 14.0),
            speed: param_f32("SPEED", 1.0),
            harm: param_f32("HARM", 9.0),
            nodes: param_f32("NODES", 22.0),
            chords: param_f32("CHORDS", 0.35),
            steps: param_f32("STEPS", 0.6),
            twist: param_f32("TWIST", 1.0),
            tube: param_f32("TUBE", 0.42),
            mesh: param_f32("MESH", 1.0),
            trail: param_f32("TRAIL", 0.55),
            label: param_f32("LABEL", 1.0),
            aspect: param_f32("ASPECT", 2.0),
        }
    }
}

#[derive(Clone, Copy)]
struct Box2 {
    x0: i32,
    y0: i32,
    w: i32,
    h: i32,
}

impl Box2 {
    #[cfg_attr(
        feature = "function-trace",
        tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)
    )]
    fn cx(&self) -> f32 {
        self.x0 as f32 + self.w as f32 * 0.5
    }

    #[cfg_attr(
        feature = "function-trace",
        tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)
    )]
    fn cy(&self) -> f32 {
        self.y0 as f32 + self.h as f32 * 0.5
    }
}

thread_local! {
    static CLIP: std::cell::Cell<Option<Box2>> = const { std::cell::Cell::new(None) };
}

#[cfg_attr(
    feature = "function-trace",
    tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)
)]
fn inside(x: i32, y: i32) -> bool {
    match CLIP.with(|c| c.get()) {
        Some(b) => x >= b.x0 && y >= b.y0 && x < b.x0 + b.w && y < b.y0 + b.h,
        None => true,
    }
}

#[cfg_attr(
    feature = "function-trace",
    tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)
)]
fn put(grid: &mut Grid, w: usize, h: usize, x: i32, y: i32, cell: Cell) {
    if inside(x, y) && x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h {
        grid[y as usize][x as usize] = cell;
    }
}

#[cfg_attr(
    feature = "function-trace",
    tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)
)]
fn put_ch(grid: &mut Grid, w: usize, h: usize, x: i32, y: i32, ch: char, fg: Color) {
    put(grid, w, h, x, y, Cell::new(ch, fg));
}

#[cfg_attr(
    feature = "function-trace",
    tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)
)]
fn put_soft(grid: &mut Grid, w: usize, h: usize, x: i32, y: i32, ch: char, fg: Color) {
    if inside(x, y) && x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h {
        let slot = &mut grid[y as usize][x as usize];
        if slot.ch == ' ' {
            *slot = Cell::new(ch, fg);
        }
    }
}

#[cfg_attr(
    feature = "function-trace",
    tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)
)]
fn put_text(grid: &mut Grid, w: usize, h: usize, x: i32, y: i32, text: &str, fg: Color) {
    for (i, ch) in text.chars().enumerate() {
        put(grid, w, h, x + i as i32, y, Cell::new(ch, fg));
    }
}

#[cfg_attr(
    feature = "function-trace",
    tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)
)]
fn put_text_bg(
    grid: &mut Grid,
    w: usize,
    h: usize,
    x: i32,
    y: i32,
    text: &str,
    fg: Color,
    bg: Color,
) {
    for (i, ch) in text.chars().enumerate() {
        put(grid, w, h, x + i as i32, y, Cell::with_bg(ch, fg, bg));
    }
}

#[cfg_attr(
    feature = "function-trace",
    tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)
)]
fn bres(x0: i32, y0: i32, x1: i32, y1: i32, mut f: impl FnMut(i32, i32)) {
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let (mut x, mut y) = (x0, y0);
    let mut err = dx + dy;
    let mut guard = dx.max(-dy) + 2;
    loop {
        f(x, y);
        if (x == x1 && y == y1) || guard <= 0 {
            break;
        }
        guard -= 1;
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
}

#[cfg_attr(
    feature = "function-trace",
    tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)
)]
fn slope_glyph(dx: f32, dy: f32) -> char {
    let ax = dx.abs();
    let ay = dy.abs();
    if ay < ax * 0.45 {
        '-'
    } else if ax < ay * 0.45 {
        '|'
    } else if dx * dy < 0.0 {
        '/'
    } else {
        '\\'
    }
}

#[cfg_attr(
    feature = "function-trace",
    tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)
)]
fn phase_of(t: f32, rate: f32) -> f32 {
    if t > 0.0 { t * rate } else { 0.0 }
}

#[cfg_attr(
    feature = "function-trace",
    tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)
)]
fn ramp(mut u: f32) -> f32 {
    u = u.clamp(0.0, 1.0);
    u * u * (3.0 - 2.0 * u)
}

#[cfg_attr(
    feature = "function-trace",
    tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)
)]
fn chapter_at(t: f32, k: &Opus5DoverKnobs) -> (usize, f32) {
    let pick = k.chapter.round();
    if pick >= 0.0 {
        return ((pick as usize) % CHAPTERS, t.max(0.0));
    }
    let dwell = k.dwell.max(1.0);
    let idx = ((t.max(0.0) / dwell).floor() as usize) % CHAPTERS;
    (idx, t.max(0.0) - (t.max(0.0) / dwell).floor() * dwell)
}

#[cfg_attr(
    feature = "function-trace",
    tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)
)]
pub(crate) fn draw_opus_5_dover(
    grid: &mut Grid,
    w: usize,
    h: usize,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    k: &Opus5DoverKnobs,
) {
    let page = darken(palette[0], 62);
    measure_layer("opus-5-dover", "page", || {
        for row in grid.iter_mut().take(h) {
            for cell in row.iter_mut().take(w) {
                *cell = Cell::with_bg(' ', palette[1], page);
            }
        }
    });
    if w < 24 || h < 10 {
        return;
    }
    let labelled = k.label > 0.5;
    let foot = if labelled { 2 } else { 0 };
    let b = Box2 {
        x0: 1,
        y0: 1,
        w: w as i32 - 2,
        h: h as i32 - 2 - foot,
    };
    let (idx, local) = chapter_at(t, k);
    CLIP.with(|c| c.set(Some(b)));
    let caption = match idx {
        0 => ch_fourier(grid, w, h, seed, palette, local, k, b),
        1 => ch_geometry(grid, w, h, seed, palette, local, k, b),
        2 => ch_graph(grid, w, h, seed, palette, local, k, b),
        3 => ch_lambda(grid, w, h, seed, palette, local, k, b),
        4 => ch_topology(grid, w, h, seed, palette, local, k, b),
        _ => ch_linalg(grid, w, h, seed, palette, local, k, b),
    };
    CLIP.with(|c| c.set(None));
    if labelled {
        measure_layer("opus-5-dover", "caption", || {
            let bar = darken(palette[0], 40);
            let y = h as i32 - 1;
            for x in 0..w as i32 {
                put(grid, w, h, x, y, Cell::with_bg(' ', palette[1], bar));
            }
            put_text_bg(grid, w, h, 1, y, TITLES[idx], lighten(palette[4], 40), bar);
            let x = 3 + TITLES[idx].len() as i32;
            put_text_bg(grid, w, h, x, y, &caption, palette[3], bar);
        });
    }
}

struct Epi {
    k: f32,
    amp: f32,
    phase: f32,
}

struct EpiFit {
    epi: Vec<Epi>,
    pts: Vec<(f32, f32)>,
    ox: f32,
    oy: f32,
    ext: f32,
}

thread_local! {
    static EPI: RefCell<Option<((u64, usize), EpiFit)>> = const { RefCell::new(None) };
    static ZBUF: RefCell<Vec<f32>> = const { RefCell::new(Vec::new()) };
}

const EPI_SAMPLES: usize = 480;

#[cfg_attr(
    feature = "function-trace",
    tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)
)]
fn build_epi(seed: u64, n: usize) -> EpiFit {
    let mut rng = StdRng::seed_from_u64(seed ^ 0xF0_1E_2A_57);
    let mut epi = Vec::with_capacity(2 * n);
    for m in 1..=n {
        for sign in [1.0f32, -1.0] {
            let decay = (m as f32).powf(1.4);
            let amp = (0.25 + 0.75 * rng.random::<f32>()) / decay;
            epi.push(Epi {
                k: sign * m as f32,
                amp,
                phase: rng.random::<f32>() * TAU,
            });
        }
    }
    epi.sort_by(|a, b| {
        b.amp
            .partial_cmp(&a.amp)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut pts = Vec::with_capacity(EPI_SAMPLES);
    let (mut lox, mut hix, mut loy, mut hiy) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
    for s in 0..EPI_SAMPLES {
        let th = TAU * s as f32 / EPI_SAMPLES as f32;
        let (mut x, mut y) = (0.0f32, 0.0f32);
        for e in &epi {
            let a = e.k * th + e.phase;
            x += e.amp * a.cos();
            y += e.amp * a.sin();
        }
        lox = lox.min(x);
        hix = hix.max(x);
        loy = loy.min(y);
        hiy = hiy.max(y);
        pts.push((x, y));
    }
    let ext = ((hix - lox).max(hiy - loy) * 0.5).max(1e-3);
    EpiFit {
        epi,
        pts,
        ox: (lox + hix) * 0.5,
        oy: (loy + hiy) * 0.5,
        ext,
    }
}

#[cfg_attr(
    feature = "function-trace",
    tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)
)]
fn ch_fourier(
    grid: &mut Grid,
    w: usize,
    h: usize,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    k: &Opus5DoverKnobs,
    b: Box2,
) -> String {
    let n = (k.harm.round() as usize).clamp(1, 24);
    let aspect = k.aspect.clamp(0.25, 4.0);
    let speed = k.speed.max(0.0);
    let caption = EPI.with(|slot| {
        let mut slot = slot.borrow_mut();
        let stale = slot
            .as_ref()
            .map(|(key, _)| *key != (seed, n))
            .unwrap_or(true);
        if stale {
            *slot = Some(((seed, n), build_epi(seed, n)));
        }
        let fit = &slot.as_ref().unwrap().1;
        let sc = ((b.w as f32 / aspect).min(b.h as f32) * 0.5 - 1.0).max(1.0) / fit.ext;
        let cx = b.cx();
        let cy = b.cy();
        let to_cell = |p: (f32, f32)| -> (f32, f32) {
            (cx + (p.0 - fit.ox) * sc * aspect, cy + (p.1 - fit.oy) * sc)
        };
        let th = phase_of(t, 0.5 * speed) % TAU;
        let head = (th / TAU * EPI_SAMPLES as f32) as usize;
        let ghost = darken(palette[2], 58);
        measure_layer("opus-5-dover", "fourier-ghost", || {
            for p in fit.pts.iter() {
                let (x, y) = to_cell(*p);
                put_soft(grid, w, h, x.round() as i32, y.round() as i32, '.', ghost);
            }
        });
        measure_layer("opus-5-dover", "fourier-trace", || {
            let keep = (k.trail.clamp(0.02, 1.0) * EPI_SAMPLES as f32) as usize;
            for i in 0..head {
                let age = (head - i) as f32 / keep.max(1) as f32;
                if age > 1.0 {
                    continue;
                }
                let (x, y) = to_cell(fit.pts[i]);
                let hot = lerp_color(palette[3], lighten(palette[4], 55), 1.0 - age);
                let ch = if age < 0.25 {
                    '#'
                } else if age < 0.6 {
                    '*'
                } else {
                    '+'
                };
                put(
                    grid,
                    w,
                    h,
                    x.round() as i32,
                    y.round() as i32,
                    Cell::new(ch, hot),
                );
            }
        });
        measure_layer("opus-5-dover", "fourier-arms", || {
            let ring = darken(palette[2], 34);
            let arm = lighten(palette[1], 30);
            let (mut px, mut py) = (0.0f32, 0.0f32);
            for (ai, e) in fit.epi.iter().enumerate() {
                let a = e.k * th + e.phase;
                let nx = px + e.amp * a.cos();
                let ny = py + e.amp * a.sin();
                let (c0x, c0y) = to_cell((px, py));
                let (c1x, c1y) = to_cell((nx, ny));
                let rad = e.amp * sc;
                if ai < 3 && rad * aspect > 1.6 {
                    let steps = ((rad * aspect * 3.0) as usize).clamp(12, 160);
                    for s in 0..steps {
                        let ang = TAU * s as f32 / steps as f32;
                        let rx = c0x + rad * aspect * ang.cos();
                        let ry = c0y + rad * ang.sin();
                        put_soft(grid, w, h, rx.round() as i32, ry.round() as i32, '\'', ring);
                    }
                }
                let g = slope_glyph(c1x - c0x, c1y - c0y);
                bres(
                    c0x.round() as i32,
                    c0y.round() as i32,
                    c1x.round() as i32,
                    c1y.round() as i32,
                    |x, y| put_ch(grid, w, h, x, y, g, arm),
                );
                put_ch(
                    grid,
                    w,
                    h,
                    c0x.round() as i32,
                    c0y.round() as i32,
                    'o',
                    palette[2],
                );
                px = nx;
                py = ny;
            }
            let (tx, ty) = to_cell((px, py));
            put_ch(
                grid,
                w,
                h,
                tx.round() as i32,
                ty.round() as i32,
                '@',
                lighten(palette[4], 70),
            );
        });
        format!("sum c_k exp(i k th), {} arms, th = {:.2}", 2 * n + 1, th)
    });
    caption
}

const SHADE: [char; 8] = ['.', ':', '-', '=', '+', '*', '#', '@'];

#[cfg_attr(
    feature = "function-trace",
    tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)
)]
fn rot3(p: (f32, f32, f32), yaw: f32, pitch: f32) -> (f32, f32, f32) {
    let (sy, cy) = yaw.sin_cos();
    let x = p.0 * cy - p.2 * sy;
    let z = p.0 * sy + p.2 * cy;
    let (sp, cp) = pitch.sin_cos();
    let y = p.1 * cp - z * sp;
    let z = p.1 * sp + z * cp;
    (x, y, z)
}

#[cfg_attr(
    feature = "function-trace",
    tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)
)]
fn project(p: (f32, f32, f32), b: Box2, sc: f32, aspect: f32) -> (f32, f32, f32) {
    let d = 4.2f32;
    let f = d / (d - p.2).max(0.5);
    (
        b.cx() + p.0 * sc * f * aspect,
        b.cy() + p.1 * sc * f,
        p.2 * f,
    )
}

#[cfg_attr(
    feature = "function-trace",
    tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)
)]
fn zplot(
    grid: &mut Grid,
    zb: &mut [f32],
    w: usize,
    h: usize,
    b: Box2,
    sx: f32,
    sy: f32,
    depth: f32,
    cell: Cell,
) {
    let (x, y) = (sx.round() as i32, sy.round() as i32);
    if x < b.x0 || y < b.y0 || x >= b.x0 + b.w || y >= b.y0 + b.h {
        return;
    }
    if x < 0 || y < 0 || x as usize >= w || y as usize >= h {
        return;
    }
    let i = y as usize * w + x as usize;
    if depth > zb[i] {
        zb[i] = depth;
        grid[y as usize][x as usize] = cell;
    }
}

#[cfg_attr(
    feature = "function-trace",
    tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)
)]
fn lambert(n: (f32, f32, f32)) -> f32 {
    let l = (0.42f32, 0.62, 0.66);
    (n.0 * l.0 + n.1 * l.1 + n.2 * l.2).clamp(0.0, 1.0)
}

#[cfg_attr(
    feature = "function-trace",
    tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)
)]
fn shade_char(v: f32) -> char {
    SHADE[((v.clamp(0.0, 1.0) * 7.999) as usize).min(7)]
}

#[cfg_attr(
    feature = "function-trace",
    tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)
)]
fn ch_geometry(
    grid: &mut Grid,
    w: usize,
    h: usize,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    k: &Opus5DoverKnobs,
    b: Box2,
) -> String {
    let aspect = k.aspect.clamp(0.25, 4.0);
    let speed = k.speed.max(0.0);
    let big = 1.0f32;
    let r = k.tube.clamp(0.12, 0.85);
    let yaw = phase_of(t, 0.32 * speed);
    let pitch = 0.55 + 0.22 * phase_of(t, 0.19 * speed).sin();
    let sc = ((b.w as f32 / aspect).min(b.h as f32) * 0.5 - 1.0).max(1.0) / (big + r + 0.25);
    let mesh = k.mesh.clamp(0.25, 3.0);
    let nu = ((22.0 * mesh) as usize).clamp(6, 64);
    let nv = ((13.0 * mesh) as usize).clamp(4, 40);
    let mut rng = StdRng::seed_from_u64(seed ^ 0x5EED_C0FF);
    let p_wind = 2 + (rng.random::<u32>() % 4) as i32;
    let q_wind = 1 + (rng.random::<u32>() % 3) as i32;
    let point = |u: f32, v: f32| -> ((f32, f32, f32), (f32, f32, f32), f32) {
        let (su, cu) = u.sin_cos();
        let (sv, cv) = v.sin_cos();
        let ring = big + r * cv;
        let pos = (ring * cu, r * sv, ring * su);
        let nrm = (cv * cu, sv, cv * su);
        let kg = cv / (r * ring);
        (pos, nrm, kg)
    };
    ZBUF.with(|zb| {
        let mut zb = zb.borrow_mut();
        zb.clear();
        zb.resize(w * h, f32::MIN);
        measure_layer("opus-5-dover", "geometry-mesh", || {
            let fine_v = ((90.0 * mesh) as usize).clamp(24, 200);
            let fine_u = ((150.0 * mesh) as usize).clamp(40, 320);
            for i in 0..nu {
                let u = TAU * i as f32 / nu as f32;
                for j in 0..fine_v {
                    let v = TAU * j as f32 / fine_v as f32;
                    let (pos, nrm, kg) = point(u, v);
                    let rp = rot3(pos, yaw, pitch);
                    let rn = rot3(nrm, yaw, pitch);
                    let (sx, sy, dz) = project(rp, b, sc, aspect);
                    let lit = 0.18 + 0.82 * lambert(rn);
                    let tint = if kg > 0.15 {
                        lerp_color(palette[3], lighten(palette[4], 30), lit)
                    } else if kg < -0.15 {
                        lerp_color(darken(palette[2], 40), palette[2], lit)
                    } else {
                        palette[1]
                    };
                    let cell = Cell::new(shade_char(lit), tint);
                    zplot(grid, &mut zb, w, h, b, sx, sy, dz, cell);
                }
            }
            for j in 0..nv {
                let v = TAU * j as f32 / nv as f32;
                for i in 0..fine_u {
                    let u = TAU * i as f32 / fine_u as f32;
                    let (pos, nrm, kg) = point(u, v);
                    let rp = rot3(pos, yaw, pitch);
                    let rn = rot3(nrm, yaw, pitch);
                    let (sx, sy, dz) = project(rp, b, sc, aspect);
                    let lit = 0.18 + 0.82 * lambert(rn);
                    let tint = if kg > 0.15 {
                        lerp_color(palette[3], lighten(palette[4], 30), lit)
                    } else if kg < -0.15 {
                        lerp_color(darken(palette[2], 40), palette[2], lit)
                    } else {
                        palette[1]
                    };
                    let cell = Cell::new(shade_char(lit * 0.8), tint);
                    zplot(grid, &mut zb, w, h, b, sx, sy, dz, cell);
                }
            }
        });
        measure_layer("opus-5-dover", "geometry-curve", || {
            let samples = 900;
            let bright = lighten(palette[4], 60);
            for s in 0..samples {
                let f = s as f32 / samples as f32;
                let (pos, _, _) = point(TAU * p_wind as f32 * f, TAU * q_wind as f32 * f);
                let rp = rot3(pos, yaw, pitch);
                let (sx, sy, dz) = project(rp, b, sc, aspect);
                zplot(
                    grid,
                    &mut zb,
                    w,
                    h,
                    b,
                    sx,
                    sy,
                    dz + 0.02,
                    Cell::new('o', bright),
                );
            }
        });
        measure_layer("opus-5-dover", "geometry-frame", || {
            let f = (phase_of(t, 0.06 * speed)) % 1.0;
            let u = TAU * p_wind as f32 * f;
            let v = TAU * q_wind as f32 * f;
            let (pos, nrm, _) = point(u, v);
            let rp = rot3(pos, yaw, pitch);
            let (sx, sy, dz) = project(rp, b, sc, aspect);
            zplot(
                grid,
                &mut zb,
                w,
                h,
                b,
                sx,
                sy,
                dz + 0.2,
                Cell::new('@', lighten(palette[4], 80)),
            );
            for step in 1..=3 {
                let e = step as f32 * 0.16;
                let tip = (pos.0 + nrm.0 * e, pos.1 + nrm.1 * e, pos.2 + nrm.2 * e);
                let rt = rot3(tip, yaw, pitch);
                let (tx, ty, tz) = project(rt, b, sc, aspect);
                zplot(
                    grid,
                    &mut zb,
                    w,
                    h,
                    b,
                    tx,
                    ty,
                    tz + 0.2,
                    Cell::new(if step == 3 { '^' } else { '|' }, palette[4]),
                );
            }
        });
    });
    format!(
        "K = cos v / r(R + r cos v), r {:.2}, winds {}:{}",
        r, p_wind, q_wind
    )
}

#[cfg_attr(
    feature = "function-trace",
    tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)
)]
fn ch_topology(
    grid: &mut Grid,
    w: usize,
    h: usize,
    _seed: u64,
    palette: &[Color; 5],
    t: f32,
    k: &Opus5DoverKnobs,
    b: Box2,
) -> String {
    let aspect = k.aspect.clamp(0.25, 4.0);
    let speed = k.speed.max(0.0);
    let half = (k.twist.round() as i32).clamp(1, 6);
    let hw = k.tube.clamp(0.12, 0.85);
    let yaw = phase_of(t, 0.28 * speed);
    let pitch = 0.62 + 0.18 * phase_of(t, 0.13 * speed).sin();
    let sc = ((b.w as f32 / aspect).min(b.h as f32) * 0.5 - 1.0).max(1.0) / (1.0 + hw + 0.2);
    let band = |u: f32, v: f32| -> (f32, f32, f32) {
        let a = half as f32 * u * 0.5;
        let (sa, ca) = a.sin_cos();
        let (su, cu) = u.sin_cos();
        let ring = 1.0 + v * ca;
        (ring * cu, ring * su, v * sa)
    };
    let normal = |u: f32, v: f32| -> (f32, f32, f32) {
        let e = 0.01f32;
        let p0 = band(u, v);
        let pu = band(u + e, v);
        let pv = band(u, v + e);
        let du = (pu.0 - p0.0, pu.1 - p0.1, pu.2 - p0.2);
        let dv = (pv.0 - p0.0, pv.1 - p0.1, pv.2 - p0.2);
        let n = (
            du.1 * dv.2 - du.2 * dv.1,
            du.2 * dv.0 - du.0 * dv.2,
            du.0 * dv.1 - du.1 * dv.0,
        );
        let m = (n.0 * n.0 + n.1 * n.1 + n.2 * n.2).sqrt().max(1e-6);
        (n.0 / m, n.1 / m, n.2 / m)
    };
    let mesh = k.mesh.clamp(0.25, 3.0);
    ZBUF.with(|zb| {
        let mut zb = zb.borrow_mut();
        zb.clear();
        zb.resize(w * h, f32::MIN);
        measure_layer("opus-5-dover", "topology-band", || {
            let nu = ((320.0 * mesh) as usize).clamp(90, 900);
            let nv = ((11.0 * mesh) as usize).clamp(5, 33);
            for i in 0..nu {
                let u = TAU * i as f32 / nu as f32;
                for j in 0..nv {
                    let v = -hw + 2.0 * hw * j as f32 / (nv - 1) as f32;
                    let pos = band(u, v);
                    let nrm = normal(u, v);
                    let rp = rot3(pos, yaw, pitch);
                    let rn = rot3(nrm, yaw, pitch);
                    let facing = rn.2;
                    let lit = 0.2 + 0.8 * lambert(rn);
                    let (sx, sy, dz) = project(rp, b, sc, aspect);
                    let (ch, tint) = if facing >= 0.0 {
                        (
                            shade_char(lit),
                            lerp_color(palette[2], lighten(palette[4], 20), lit),
                        )
                    } else {
                        (
                            if lit > 0.55 { '%' } else { ',' },
                            lerp_color(darken(palette[3], 50), palette[3], lit),
                        )
                    };
                    zplot(grid, &mut zb, w, h, b, sx, sy, dz, Cell::new(ch, tint));
                }
            }
        });
        measure_layer("opus-5-dover", "topology-edge", || {
            let rim = lighten(palette[4], 45);
            let steps = ((520.0 * mesh) as usize).clamp(120, 1200);
            for i in 0..steps {
                let u = TAU * 2.0 * i as f32 / steps as f32;
                let pos = band(u, hw);
                let rp = rot3(pos, yaw, pitch);
                let (sx, sy, dz) = project(rp, b, sc, aspect);
                zplot(
                    grid,
                    &mut zb,
                    w,
                    h,
                    b,
                    sx,
                    sy,
                    dz + 0.05,
                    Cell::new('=', rim),
                );
            }
        });
        measure_layer("opus-5-dover", "topology-walker", || {
            let laps = phase_of(t, 0.05 * speed);
            let u = TAU * laps;
            let nrm = normal(u, 0.0);
            let pos = band(u, 0.0);
            let rp = rot3(pos, yaw, pitch);
            let (sx, sy, dz) = project(rp, b, sc, aspect);
            zplot(
                grid,
                &mut zb,
                w,
                h,
                b,
                sx,
                sy,
                dz + 0.4,
                Cell::new('@', lighten(palette[4], 80)),
            );
            for step in 1..=4 {
                let e = step as f32 * 0.11;
                let tip = (pos.0 + nrm.0 * e, pos.1 + nrm.1 * e, pos.2 + nrm.2 * e);
                let rt = rot3(tip, yaw, pitch);
                let (tx, ty, tz) = project(rt, b, sc, aspect);
                let ch = if step == 4 { 'A' } else { '|' };
                zplot(
                    grid,
                    &mut zb,
                    w,
                    h,
                    b,
                    tx,
                    ty,
                    tz + 0.4,
                    Cell::new(ch, palette[4]),
                );
            }
        });
    });
    let laps = phase_of(t, 0.05 * speed);
    format!(
        "{} half-twists, chi 0, side flip on lap {}",
        half,
        (laps as i32) % 2 + 1
    )
}

struct GraphFit {
    edges: Vec<(usize, usize)>,
    tree: Vec<bool>,
    pos: Vec<(f32, f32)>,
    layer: Vec<usize>,
    deg: Vec<usize>,
    depth: usize,
}

thread_local! {
    static GRAPH: RefCell<Option<((u64, usize, u32), GraphFit)>> = const { RefCell::new(None) };
}

#[cfg_attr(
    feature = "function-trace",
    tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)
)]
fn build_graph(seed: u64, n: usize, chord_pct: u32) -> GraphFit {
    let mut rng = StdRng::seed_from_u64(seed ^ 0x6A_9B_ED_6E);
    let mut edges: Vec<(usize, usize)> = Vec::new();
    let mut tree: Vec<bool> = Vec::new();
    for i in 1..n {
        let parent = (rng.random::<u32>() as usize) % i;
        edges.push((parent, i));
        tree.push(true);
    }
    let extra = (n * chord_pct as usize) / 100;
    for _ in 0..extra {
        let a = (rng.random::<u32>() as usize) % n;
        let b = (rng.random::<u32>() as usize) % n;
        if a != b && !edges.contains(&(a, b)) && !edges.contains(&(b, a)) {
            edges.push((a, b));
            tree.push(false);
        }
    }
    let mut deg = vec![0usize; n];
    for (a, b) in &edges {
        deg[*a] += 1;
        deg[*b] += 1;
    }
    let mut pos: Vec<(f32, f32)> = (0..n)
        .map(|i| {
            let a = TAU * i as f32 / n as f32;
            let jitter = 0.85 + 0.3 * rng.random::<f32>();
            (a.cos() * jitter, a.sin() * jitter)
        })
        .collect();
    let kdist = (4.0 / n as f32).sqrt();
    let mut temp = 0.6f32;
    for _ in 0..260 {
        let mut disp = vec![(0.0f32, 0.0f32); n];
        for i in 0..n {
            for j in (i + 1)..n {
                let dx = pos[i].0 - pos[j].0;
                let dy = pos[i].1 - pos[j].1;
                let d2 = (dx * dx + dy * dy).max(1e-4);
                let d = d2.sqrt();
                let rep = kdist * kdist / d2;
                disp[i].0 += dx / d * rep;
                disp[i].1 += dy / d * rep;
                disp[j].0 -= dx / d * rep;
                disp[j].1 -= dy / d * rep;
            }
        }
        for (a, b) in &edges {
            let dx = pos[*a].0 - pos[*b].0;
            let dy = pos[*a].1 - pos[*b].1;
            let d = (dx * dx + dy * dy).sqrt().max(1e-3);
            let att = d * d / kdist;
            disp[*a].0 -= dx / d * att;
            disp[*a].1 -= dy / d * att;
            disp[*b].0 += dx / d * att;
            disp[*b].1 += dy / d * att;
        }
        for i in 0..n {
            let m = (disp[i].0 * disp[i].0 + disp[i].1 * disp[i].1)
                .sqrt()
                .max(1e-6);
            let step = m.min(temp);
            pos[i].0 += disp[i].0 / m * step;
            pos[i].1 += disp[i].1 / m * step;
        }
        temp *= 0.985;
    }
    let ext = pos
        .iter()
        .map(|p| p.0.abs().max(p.1.abs()))
        .fold(1e-3f32, f32::max);
    for p in pos.iter_mut() {
        p.0 /= ext;
        p.1 /= ext;
    }
    let mut layer = vec![usize::MAX; n];
    let mut queue = vec![0usize];
    layer[0] = 0;
    let mut qi = 0;
    while qi < queue.len() {
        let cur = queue[qi];
        qi += 1;
        for (a, b) in &edges {
            let next = if *a == cur {
                *b
            } else if *b == cur {
                *a
            } else {
                continue;
            };
            if layer[next] == usize::MAX {
                layer[next] = layer[cur] + 1;
                queue.push(next);
            }
        }
    }
    let depth = layer
        .iter()
        .filter(|l| **l != usize::MAX)
        .copied()
        .max()
        .unwrap_or(0);
    for l in layer.iter_mut() {
        if *l == usize::MAX {
            *l = depth + 1;
        }
    }
    GraphFit {
        edges,
        tree,
        pos,
        layer,
        deg,
        depth,
    }
}

#[cfg_attr(
    feature = "function-trace",
    tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)
)]
fn node_glyph(i: usize) -> char {
    let table = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
    table[i % table.len()] as char
}

#[cfg_attr(
    feature = "function-trace",
    tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)
)]
fn ch_graph(
    grid: &mut Grid,
    w: usize,
    h: usize,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    k: &Opus5DoverKnobs,
    b: Box2,
) -> String {
    let n = (k.nodes.round() as usize).clamp(4, 60);
    let chord_pct = (k.chords.clamp(0.0, 2.0) * 100.0) as u32;
    let aspect = k.aspect.clamp(0.25, 4.0);
    let speed = k.speed.max(0.0);
    let caption = GRAPH.with(|slot| {
        let mut slot = slot.borrow_mut();
        let key = (seed, n, chord_pct);
        let stale = slot.as_ref().map(|(kk, _)| *kk != key).unwrap_or(true);
        if stale {
            *slot = Some((key, build_graph(seed, n, chord_pct)));
        }
        let g = &slot.as_ref().unwrap().1;
        let sx = (b.w as f32 * 0.5 - 2.0).max(2.0);
        let sy = (b.h as f32 * 0.5 - 1.0).max(1.0);
        let front = phase_of(t, 0.55 * speed) % (g.depth as f32 + 3.0);
        let cell_of = |i: usize| -> (f32, f32) {
            let sway = if t > 0.0 {
                let ph = i as f32 * 1.7;
                (
                    0.035 * (t * 0.7 + ph).sin(),
                    0.035 * (t * 0.9 + ph * 1.3).cos(),
                )
            } else {
                (0.0, 0.0)
            };
            (
                b.cx() + (g.pos[i].0 + sway.0) * sx,
                b.cy() + (g.pos[i].1 + sway.1) * sy,
            )
        };
        let cold = darken(palette[2], 50);
        let warm = lighten(palette[4], 40);
        measure_layer("opus-5-dover", "graph-edges", || {
            for (e, (a, c)) in g.edges.iter().enumerate() {
                let reach = g.layer[*a].max(g.layer[*c]) as f32;
                let lead = front - reach;
                let (p0, p1) = (cell_of(*a), cell_of(*c));
                let glyph = if g.tree[e] {
                    slope_glyph(p1.0 - p0.0, p1.1 - p0.1)
                } else {
                    ':'
                };
                let tint = if lead < 0.0 {
                    darken(palette[1], 55)
                } else if lead < 1.2 {
                    lerp_color(warm, palette[3], ramp(lead / 1.2))
                } else {
                    cold
                };
                bres(
                    p0.0.round() as i32,
                    p0.1.round() as i32,
                    p1.0.round() as i32,
                    p1.1.round() as i32,
                    |x, y| put_soft(grid, w, h, x, y, glyph, tint),
                );
            }
        });
        measure_layer("opus-5-dover", "graph-nodes", || {
            for i in 0..n {
                let (x, y) = cell_of(i);
                let lead = front - g.layer[i] as f32;
                let hot = if lead < 0.0 {
                    darken(palette[1], 35)
                } else if lead < 1.0 {
                    lighten(palette[4], 70)
                } else {
                    lerp_color(palette[3], palette[2], (g.deg[i] as f32 / 6.0).min(1.0))
                };
                let (xi, yi) = (x.round() as i32, y.round() as i32);
                let ring = if g.deg[i] > 3 { ('[', ']') } else { ('(', ')') };
                put_ch(grid, w, h, xi - 1, yi, ring.0, darken(hot, 30));
                put_ch(grid, w, h, xi + 1, yi, ring.1, darken(hot, 30));
                put_ch(grid, w, h, xi, yi, node_glyph(i), hot);
            }
        });
        let _ = aspect;
        format!(
            "|V| {}, |E| {}, tree + {} chords, front {:.1}/{}",
            n,
            g.edges.len(),
            g.edges.len() + 1 - n,
            front.min(g.depth as f32),
            g.depth
        )
    });
    caption
}

#[cfg_attr(
    feature = "function-trace",
    tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)
)]
fn ch_linalg(
    grid: &mut Grid,
    w: usize,
    h: usize,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    k: &Opus5DoverKnobs,
    b: Box2,
) -> String {
    let aspect = k.aspect.clamp(0.25, 4.0);
    let speed = k.speed.max(0.0);
    let mut rng = StdRng::seed_from_u64(seed ^ 0x11_AA_22_BB);
    let mut m = [0.0f32; 4];
    for slot in m.iter_mut() {
        *slot = -1.7 + 3.4 * rng.random::<f32>();
    }
    if (m[0] * m[3] - m[1] * m[2]).abs() < 0.25 {
        m[0] += 1.1;
        m[3] -= 0.9;
    }
    let s = 0.5 - 0.5 * phase_of(t, 0.45 * speed).cos();
    let a = [
        1.0 + s * (m[0] - 1.0),
        s * m[1],
        s * m[2],
        1.0 + s * (m[3] - 1.0),
    ];
    let span = 2.6f32;
    let sc = ((b.w as f32 / aspect).min(b.h as f32) * 0.5 - 1.0).max(1.0) / span;
    let map = |x: f32, y: f32| -> (f32, f32) {
        let u = a[0] * x + a[1] * y;
        let v = a[2] * x + a[3] * y;
        (b.cx() + u * sc * aspect, b.cy() - v * sc)
    };
    let lattice = darken(palette[2], 52);
    measure_layer("opus-5-dover", "linalg-lattice", || {
        let lines = 5;
        for i in 0..lines {
            let c = -2.0 + 4.0 * i as f32 / (lines - 1) as f32;
            for (p, q) in [(map(c, -2.0), map(c, 2.0)), (map(-2.0, c), map(2.0, c))] {
                let g = slope_glyph(q.0 - p.0, q.1 - p.1);
                bres(
                    p.0.round() as i32,
                    p.1.round() as i32,
                    q.0.round() as i32,
                    q.1.round() as i32,
                    |x, y| put_soft(grid, w, h, x, y, g, lattice),
                );
            }
        }
    });
    measure_layer("opus-5-dover", "linalg-image", || {
        let hot = lighten(palette[4], 45);
        let ghost = darken(palette[2], 45);
        for i in 0..240 {
            let th = TAU * i as f32 / 240.0;
            let p = map_ray(th.cos(), th.sin(), b, sc, aspect);
            put_soft(
                grid,
                w,
                h,
                p.0.round() as i32,
                p.1.round() as i32,
                '.',
                ghost,
            );
        }
        for i in 0..360 {
            let th = TAU * i as f32 / 360.0;
            let p = map(th.cos(), th.sin());
            put(
                grid,
                w,
                h,
                p.0.round() as i32,
                p.1.round() as i32,
                Cell::new('*', hot),
            );
        }
        for (vx, vy, ch, tint) in [
            (1.0f32, 0.0f32, '>', palette[4]),
            (0.0, 1.0, '^', palette[2]),
        ] {
            let o = map(0.0, 0.0);
            let p = map(vx, vy);
            let g = slope_glyph(p.0 - o.0, p.1 - o.1);
            bres(o.0 as i32, o.1 as i32, p.0 as i32, p.1 as i32, |x, y| {
                put_ch(grid, w, h, x, y, g, tint)
            });
            put_ch(
                grid,
                w,
                h,
                p.0.round() as i32,
                p.1.round() as i32,
                ch,
                lighten(tint, 50),
            );
        }
    });
    let tr = a[0] + a[3];
    let det = a[0] * a[3] - a[1] * a[2];
    let disc = tr * tr - 4.0 * det;
    let eig_text;
    if disc >= 0.0 {
        let root = disc.sqrt();
        let l1 = (tr + root) * 0.5;
        let l2 = (tr - root) * 0.5;
        measure_layer("opus-5-dover", "linalg-eigen", || {
            for (l, tint) in [(l1, lighten(palette[3], 30)), (l2, palette[1])] {
                let (ex, ey) = if a[1].abs() > 1e-4 {
                    (a[1], l - a[0])
                } else if a[2].abs() > 1e-4 {
                    (l - a[3], a[2])
                } else {
                    (1.0, 0.0)
                };
                let m = (ex * ex + ey * ey).sqrt().max(1e-6);
                let (ux, uy) = (ex / m * span, ey / m * span);
                let p0 = map_ray(-ux, -uy, b, sc, aspect);
                let p1 = map_ray(ux, uy, b, sc, aspect);
                bres(
                    p0.0 as i32,
                    p0.1 as i32,
                    p1.0 as i32,
                    p1.1 as i32,
                    |x, y| put_soft(grid, w, h, x, y, '=', tint),
                );
            }
        });
        eig_text = format!("l = {:+.2}, {:+.2}", l1, l2);
    } else {
        let re = tr * 0.5;
        let im = (-disc).sqrt() * 0.5;
        eig_text = format!("l = {:+.2} +- {:.2}i (rotation)", re, im);
    }
    measure_layer("opus-5-dover", "linalg-readout", || {
        let bg = darken(palette[0], 46);
        let fg = lighten(palette[4], 30);
        let rows = [
            format!("A = [ {:+.2}  {:+.2} ]", a[0], a[1]),
            format!("    [ {:+.2}  {:+.2} ]", a[2], a[3]),
            format!("det {:+.2}   tr {:+.2}", det, tr),
            eig_text.clone(),
        ];
        for (i, row) in rows.iter().enumerate() {
            put_text_bg(grid, w, h, b.x0, b.y0 + i as i32, row, fg, bg);
        }
    });
    format!("A = I + s(M - I), s {:.2}, det {:+.2}", s, det)
}

#[cfg_attr(
    feature = "function-trace",
    tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)
)]
fn map_ray(x: f32, y: f32, b: Box2, sc: f32, aspect: f32) -> (f32, f32) {
    (b.cx() + x * sc * aspect, b.cy() - y * sc)
}

#[derive(Clone)]
enum Term {
    Var(usize),
    Lam(Box<Term>),
    App(Box<Term>, Box<Term>),
}

#[cfg_attr(
    feature = "function-trace",
    tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)
)]
fn lam(body: Term) -> Term {
    Term::Lam(Box::new(body))
}

#[cfg_attr(
    feature = "function-trace",
    tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)
)]
fn app(f: Term, a: Term) -> Term {
    Term::App(Box::new(f), Box::new(a))
}

#[cfg_attr(
    feature = "function-trace",
    tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)
)]
fn church(n: usize) -> Term {
    let mut body = Term::Var(0);
    for _ in 0..n {
        body = app(Term::Var(1), body);
    }
    lam(lam(body))
}

#[cfg_attr(
    feature = "function-trace",
    tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)
)]
fn shift(t: &Term, d: i32, c: usize) -> Term {
    match t {
        Term::Var(i) => {
            if *i >= c {
                Term::Var((*i as i32 + d).max(0) as usize)
            } else {
                Term::Var(*i)
            }
        }
        Term::Lam(b) => lam(shift(b, d, c + 1)),
        Term::App(f, a) => app(shift(f, d, c), shift(a, d, c)),
    }
}

#[cfg_attr(
    feature = "function-trace",
    tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)
)]
fn subst(t: &Term, j: usize, s: &Term) -> Term {
    match t {
        Term::Var(i) => {
            if *i == j {
                s.clone()
            } else {
                Term::Var(*i)
            }
        }
        Term::Lam(b) => lam(subst(b, j + 1, &shift(s, 1, 0))),
        Term::App(f, a) => app(subst(f, j, s), subst(a, j, s)),
    }
}

#[cfg_attr(
    feature = "function-trace",
    tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)
)]
fn contract(body: &Term, arg: &Term) -> Term {
    shift(&subst(body, 0, &shift(arg, 1, 0)), -1, 0)
}

#[cfg_attr(
    feature = "function-trace",
    tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)
)]
fn size(t: &Term) -> usize {
    match t {
        Term::Var(_) => 1,
        Term::Lam(b) => 1 + size(b),
        Term::App(f, a) => 1 + size(f) + size(a),
    }
}

#[cfg_attr(
    feature = "function-trace",
    tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)
)]
fn find_redex(t: &Term, path: &mut Vec<u8>) -> bool {
    match t {
        Term::Var(_) => false,
        Term::App(f, a) => {
            if matches!(**f, Term::Lam(_)) {
                return true;
            }
            path.push(0);
            if find_redex(f, path) {
                return true;
            }
            path.pop();
            path.push(1);
            if find_redex(a, path) {
                return true;
            }
            path.pop();
            false
        }
        Term::Lam(b) => {
            path.push(2);
            if find_redex(b, path) {
                return true;
            }
            path.pop();
            false
        }
    }
}

#[cfg_attr(
    feature = "function-trace",
    tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)
)]
fn reduce_at(t: &Term, path: &[u8]) -> Term {
    if path.is_empty() {
        if let Term::App(f, a) = t {
            if let Term::Lam(body) = &**f {
                return contract(body, a);
            }
        }
        return t.clone();
    }
    match (t, path[0]) {
        (Term::App(f, a), 0) => app(reduce_at(f, &path[1..]), (**a).clone()),
        (Term::App(f, a), 1) => app((**f).clone(), reduce_at(a, &path[1..])),
        (Term::Lam(b), 2) => lam(reduce_at(b, &path[1..])),
        _ => t.clone(),
    }
}

const BINDERS: [char; 12] = ['x', 'y', 'z', 'f', 'g', 'h', 'u', 'v', 'w', 'p', 'q', 'r'];

#[cfg_attr(
    feature = "function-trace",
    tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)
)]
fn binder_name(level: usize) -> String {
    let base = BINDERS[level % BINDERS.len()];
    if level < BINDERS.len() {
        base.to_string()
    } else {
        format!("{}{}", base, level / BINDERS.len())
    }
}

#[cfg_attr(
    feature = "function-trace",
    tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)
)]
fn show(
    t: &Term,
    depth: usize,
    prec: u8,
    route: Option<&[u8]>,
    out: &mut String,
    mark: &mut Option<(usize, usize)>,
) {
    let here = matches!(route, Some(r) if r.is_empty());
    let start = out.len();
    match t {
        Term::Var(i) => {
            if *i < depth {
                out.push_str(&binder_name(depth - 1 - *i));
            } else {
                out.push_str(&format!("?{}", i - depth));
            }
        }
        Term::Lam(b) => {
            let wrap = prec > 0;
            if wrap {
                out.push('(');
            }
            out.push('\\');
            out.push_str(&binder_name(depth));
            out.push('.');
            let sub = match route {
                Some(r) if !r.is_empty() && r[0] == 2 => Some(&r[1..]),
                _ => None,
            };
            show(b, depth + 1, 0, sub, out, mark);
            if wrap {
                out.push(')');
            }
        }
        Term::App(f, a) => {
            let wrap = prec > 1;
            if wrap {
                out.push('(');
            }
            let lroute = match route {
                Some(r) if !r.is_empty() && r[0] == 0 => Some(&r[1..]),
                _ => None,
            };
            let rroute = match route {
                Some(r) if !r.is_empty() && r[0] == 1 => Some(&r[1..]),
                _ => None,
            };
            show(f, depth, 1, lroute, out, mark);
            out.push(' ');
            show(a, depth, 2, rroute, out, mark);
            if wrap {
                out.push(')');
            }
        }
    }
    if here {
        *mark = Some((start, out.len()));
    }
}

struct LamStep {
    text: String,
    mark: Option<(usize, usize)>,
}

thread_local! {
    static LAMBDA: RefCell<Option<((u64, u32), Vec<LamStep>, &'static str)>> = const { RefCell::new(None) };
}

#[cfg_attr(
    feature = "function-trace",
    tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)
)]
fn seed_term(seed: u64) -> (Term, &'static str) {
    let succ = lam(lam(lam(app(
        Term::Var(1),
        app(app(Term::Var(2), Term::Var(1)), Term::Var(0)),
    ))));
    let plus = lam(lam(lam(lam(app(
        app(Term::Var(3), Term::Var(1)),
        app(app(Term::Var(2), Term::Var(1)), Term::Var(0)),
    )))));
    let mult = lam(lam(lam(app(Term::Var(2), app(Term::Var(1), Term::Var(0))))));
    let s_comb = lam(lam(lam(app(
        app(Term::Var(2), Term::Var(0)),
        app(Term::Var(1), Term::Var(0)),
    ))));
    let k_comb = lam(lam(Term::Var(1)));
    match seed % 5 {
        0 => (app(app(plus, church(2)), church(3)), "plus 2 3"),
        1 => (app(app(mult, church(2)), church(3)), "mult 2 3"),
        2 => (
            app(app(app(s_comb, k_comb.clone()), k_comb), church(2)),
            "S K K 2",
        ),
        3 => (app(app(church(3), succ), church(0)), "3 succ 0"),
        _ => (app(church(2), church(2)), "2 2"),
    }
}

#[cfg_attr(
    feature = "function-trace",
    tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)
)]
fn build_lambda(seed: u64, cap: u32) -> (Vec<LamStep>, &'static str) {
    let (mut term, name) = seed_term(seed);
    let mut steps: Vec<LamStep> = Vec::new();
    for _ in 0..cap {
        let mut path = Vec::new();
        let live = find_redex(&term, &mut path);
        let mut text = String::new();
        let mut mark = None;
        let route: Option<&[u8]> = if live { Some(&path) } else { None };
        show(&term, 0, 0, route, &mut text, &mut mark);
        steps.push(LamStep {
            text,
            mark: if live { mark } else { None },
        });
        if !live || size(&term) > 420 {
            break;
        }
        term = reduce_at(&term, &path);
    }
    (steps, name)
}

#[cfg_attr(
    feature = "function-trace",
    tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)
)]
fn wrap_marked(step: &LamStep, width: usize) -> Vec<(String, Option<(usize, usize)>)> {
    let chars: Vec<char> = step.text.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let end = (i + width).min(chars.len());
        let line: String = chars[i..end].iter().collect();
        let hl = step.mark.and_then(|(s, e)| {
            let s = s.max(i);
            let e = e.min(end);
            if s < e { Some((s - i, e - i)) } else { None }
        });
        out.push((line, hl));
        i = end;
    }
    if out.is_empty() {
        out.push((String::new(), None));
    }
    out
}

#[cfg_attr(
    feature = "function-trace",
    tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)
)]
fn ch_lambda(
    grid: &mut Grid,
    w: usize,
    h: usize,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    k: &Opus5DoverKnobs,
    b: Box2,
) -> String {
    let cap = (k.steps.clamp(0.05, 4.0) * 40.0) as u32;
    let speed = k.speed.max(0.0);
    let caption = LAMBDA.with(|slot| {
        let mut slot = slot.borrow_mut();
        let key = (seed, cap);
        let stale = slot.as_ref().map(|(kk, _, _)| *kk != key).unwrap_or(true);
        if stale {
            let (steps, name) = build_lambda(seed, cap);
            *slot = Some((key, steps, name));
        }
        let entry = slot.as_ref().unwrap();
        let steps = &entry.1;
        let name = entry.2;
        let last = steps.len().saturating_sub(1);
        let hold = 3.0f32;
        let cursor = phase_of(t, 0.75 * speed) % (steps.len() as f32 + hold);
        let cur = (cursor.floor() as usize).min(last);
        let width = (b.w as usize).saturating_sub(6).max(8);
        let mut blocks: Vec<(usize, Vec<(String, Option<(usize, usize)>)>)> = Vec::new();
        let mut budget = (b.h as usize).saturating_sub(2);
        let mut i = cur as i32;
        while i >= 0 && budget > 0 {
            let lines = wrap_marked(&steps[i as usize], width);
            if lines.len() + 1 > budget && !blocks.is_empty() {
                break;
            }
            budget = budget.saturating_sub(lines.len() + 1);
            blocks.push((i as usize, lines));
            i -= 1;
        }
        blocks.reverse();
        measure_layer("opus-5-dover", "lambda-trace", || {
            let mut y = b.y0 + 2;
            for (idx, lines) in blocks.iter() {
                let age = cur - idx;
                let base = if age == 0 {
                    lighten(palette[4], 45)
                } else {
                    darken(palette[1], (18 * age.min(4)) as u8 + 20)
                };
                let hl_bg = darken(palette[3], 30);
                let hl_fg = lighten(palette[0], 80);
                for (li, (line, hl)) in lines.iter().enumerate() {
                    if y >= b.y0 + b.h {
                        break;
                    }
                    let x = b.x0 + 4;
                    if li == 0 {
                        let tag = if age == 0 {
                            format!("{:>2}>", idx)
                        } else {
                            format!("{:>2} ", idx)
                        };
                        put_text(grid, w, h, b.x0, y, &tag, darken(palette[2], 20));
                    }
                    put_text(grid, w, h, x, y, line, base);
                    if age == 0 {
                        if let Some((s, e)) = hl {
                            let seg: String = line.chars().skip(*s).take(e - s).collect();
                            put_text_bg(grid, w, h, x + *s as i32, y, &seg, hl_fg, hl_bg);
                        }
                    }
                    y += 1;
                }
                y += 1;
            }
        });
        measure_layer("opus-5-dover", "lambda-rule", || {
            let rule = "(\\x. M) N  ->  M[x := N]";
            put_text(grid, w, h, b.x0 + 1, b.y0, rule, darken(palette[4], 25));
        });
        let done = cur == last && steps[last].mark.is_none();
        format!(
            "{}, beta {}/{}{}",
            name,
            cur,
            last,
            if done { ", normal form" } else { "" }
        )
    });
    caption
}

#[cfg_attr(
    feature = "function-trace",
    tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)
)]
pub(crate) fn cli_opus_5_dover(
    mut grid: Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: [Color; 5],
    rng: StdRng,
    t_anim: f32,
    term_w: u16,
    term_h: u16,
    args: &[String],
    mode: &str,
    theme_name: &str,
) -> (Grid, bool) {
    let _ = (rng, term_w, term_h, mode, theme_name);
    let mut k = Opus5DoverKnobs::from_env();
    let pos: Vec<f32> = args.iter().skip(4).filter_map(|a| a.parse().ok()).collect();
    let slots: [&mut f32; 13] = [
        &mut k.chapter,
        &mut k.dwell,
        &mut k.speed,
        &mut k.harm,
        &mut k.nodes,
        &mut k.chords,
        &mut k.steps,
        &mut k.twist,
        &mut k.tube,
        &mut k.mesh,
        &mut k.trail,
        &mut k.label,
        &mut k.aspect,
    ];
    for (slot, v) in slots.into_iter().zip(pos.iter()) {
        *slot = *v;
    }
    draw_opus_5_dover(&mut grid, width, height, seed, &palette, t_anim, &k);
    (grid, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg_attr(
        feature = "function-trace",
        tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)
    )]
    fn run(w: usize, h: usize, seed: u64, t: f32, chapter: f32) -> String {
        let mut g = vec![vec![Cell::blank(); w]; h];
        let p = crate::color::make_palette(seed);
        let mut k = Opus5DoverKnobs::from_env();
        k.chapter = chapter;
        draw_opus_5_dover(&mut g, w, h, seed, &p, t, &k);
        g.iter()
            .map(|row| row.iter().map(|c| c.ch).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    #[cfg_attr(
        feature = "function-trace",
        tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)
    )]
    fn snapshot_dover_fourier() {
        insta::assert_snapshot!("opus_5_dover_fourier_80x24", run(80, 24, 42, 0.0, 0.0));
    }

    #[test]
    #[cfg_attr(
        feature = "function-trace",
        tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)
    )]
    fn snapshot_dover_geometry() {
        insta::assert_snapshot!("opus_5_dover_geometry_80x24", run(80, 24, 42, 0.0, 1.0));
    }

    #[test]
    #[cfg_attr(
        feature = "function-trace",
        tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)
    )]
    fn snapshot_dover_graph() {
        insta::assert_snapshot!("opus_5_dover_graph_80x24", run(80, 24, 42, 0.0, 2.0));
    }

    #[test]
    #[cfg_attr(
        feature = "function-trace",
        tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)
    )]
    fn snapshot_dover_lambda() {
        insta::assert_snapshot!("opus_5_dover_lambda_80x24", run(80, 24, 42, 0.0, 3.0));
    }

    #[test]
    #[cfg_attr(
        feature = "function-trace",
        tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)
    )]
    fn snapshot_dover_topology() {
        insta::assert_snapshot!("opus_5_dover_topology_80x24", run(80, 24, 42, 0.0, 4.0));
    }

    #[test]
    #[cfg_attr(
        feature = "function-trace",
        tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)
    )]
    fn snapshot_dover_linalg() {
        insta::assert_snapshot!("opus_5_dover_linalg_110x36", run(110, 36, 42, 0.0, 5.0));
    }

    #[test]
    #[cfg_attr(
        feature = "function-trace",
        tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)
    )]
    fn deterministic_and_seed_sensitive() {
        assert_eq!(run(90, 30, 42, 0.0, -1.0), run(90, 30, 42, 0.0, -1.0));
        assert_ne!(run(90, 30, 42, 0.0, -1.0), run(90, 30, 7, 0.0, -1.0));
    }

    #[test]
    #[cfg_attr(
        feature = "function-trace",
        tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)
    )]
    fn cycle_visits_every_chapter() {
        let k = Opus5DoverKnobs::from_env();
        let dwell = k.dwell.max(1.0);
        for c in 0..CHAPTERS {
            let t = dwell * c as f32 + 0.5;
            assert_eq!(chapter_at(t, &k).0, c);
        }
    }

    #[test]
    #[cfg_attr(
        feature = "function-trace",
        tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)
    )]
    fn every_chapter_animates() {
        for c in 0..CHAPTERS {
            let a = run(90, 30, 42, 0.0, c as f32);
            let b = run(90, 30, 42, 6.0, c as f32);
            assert_ne!(a, b, "chapter {c} static under t");
        }
    }

    #[test]
    #[cfg_attr(
        feature = "function-trace",
        tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)
    )]
    fn church_arithmetic_reaches_normal_form() {
        let (steps, name) = build_lambda(1, 40);
        assert_eq!(name, "mult 2 3");
        let tail = steps.last().unwrap();
        assert!(tail.mark.is_none(), "left a redex: {}", tail.text);
        assert_eq!(tail.text, "\\x.\\y.x (x (x (x (x (x y)))))");
    }

    #[test]
    #[cfg_attr(
        feature = "function-trace",
        tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)
    )]
    fn tiny_grid_is_safe() {
        let mut g = vec![vec![Cell::blank(); 12]; 5];
        let p = crate::color::make_palette(3);
        let k = Opus5DoverKnobs::from_env();
        draw_opus_5_dover(&mut g, 12, 5, 3, &p, 4.0, &k);
    }

    #[test]
    #[cfg_attr(
        feature = "function-trace",
        tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all)
    )]
    fn frame_cost() {
        let (w, h) = (200usize, 60usize);
        let mut g = vec![vec![Cell::blank(); w]; h];
        let p = crate::color::make_palette(42);
        let k = Opus5DoverKnobs::from_env();
        let mut worst = 0.0f64;
        let start = std::time::Instant::now();
        for f in 0..200 {
            let t0 = std::time::Instant::now();
            draw_opus_5_dover(&mut g, w, h, 42, &p, f as f32 * 0.45, &k);
            worst = worst.max(t0.elapsed().as_secs_f64() * 1000.0);
        }
        let avg = start.elapsed().as_secs_f64() * 1000.0 / 200.0;
        eprintln!(
            "opus-5-dover frame_cost 200x60: avg {:.3} ms, worst {:.3} ms",
            avg, worst
        );
        if !cfg!(debug_assertions) {
            assert!(avg < 4.0, "avg frame {:.3} ms", avg);
        }
    }
}
