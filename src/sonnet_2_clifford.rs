//! sonnet-2-clifford -- a Clifford strange attractor, iterated fresh every frame
//! while its four map constants breathe, so the dust cloud slowly reshapes itself.
use crate::_0_profile::measure_layer;
use crate::color::*;
use crate::opts::param_f32;
use crate::types::*;
use crossterm::style::Color;
use rand::RngExt;
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::cell::RefCell;
use std::f32::consts::TAU;

const TRIALS: u32 = 22;
const TRIAL_ITERS: u32 = 9000;
const BURN_IN: u32 = 500;
const PARAM_RANGE: f32 = 2.2;
const LUT_CAP: u32 = 2048;
const FINE: usize = 48;
const COARSE: usize = 24;
const TARGET_DIM: f32 = 1.4;
const MIN_OCC: usize = FINE * FINE / 25;

pub(crate) struct CliffordKnobs {
    pub speed: f32,
    pub hue: f32,
    pub spread: f32,
    pub drift: f32,
    pub period: f32,
    pub density: f32,
    pub comet: f32,
    pub glow: f32,
    pub scale: f32,
}

impl CliffordKnobs {
    pub(crate) fn from_env() -> Self {
        CliffordKnobs {
            speed: param_f32("SPEED", 1.0),
            hue: param_f32("HUE", 0.0),
            spread: param_f32("SPREAD", 60.0),
            drift: param_f32("DRIFT", 0.5),
            period: param_f32("PERIOD", 36.0),
            density: param_f32("DENSITY", 180_000.0),
            comet: param_f32("COMET", 220.0),
            glow: param_f32("GLOW", 1.0),
            scale: param_f32("SCALE", 1.0),
        }
    }
}

#[derive(Clone, Copy)]
struct Geom {
    a: f32,
    b: f32,
    c: f32,
    d: f32,
    x0: f32,
    y0: f32,
    hue0: f32,
    variant: bool,
}

struct Cached {
    seed: u64,
    geom: Geom,
    counts: Vec<u32>,
}

thread_local! {
    static CACHE: RefCell<Option<Cached>> = RefCell::new(None);
}

fn step(a: f32, b: f32, c: f32, d: f32, x: f32, y: f32) -> (f32, f32) {
    ((a * y).sin() + c * (a * x).cos(), (b * x).sin() + d * (b * y).cos())
}

const QUICK_ITERS: u32 = 2200;
const QUICK_GRID: usize = 24;
const MIN_QUICK_OCC: usize = 30;

struct Quick {
    minx: f32,
    maxx: f32,
    miny: f32,
    maxy: f32,
    x_end: f32,
    y_end: f32,
    occ: usize,
}

/// Retrace a short bbox each frame, then bin a second pass into that bbox:
/// a periodic window fills only a handful of its own bbox cells, chaos fills many.
fn quick_bbox(a: f32, b: f32, c: f32, d: f32, x0: f32, y0: f32) -> Quick {
    let (mut x, mut y) = (x0, y0);
    let (mut minx, mut maxx, mut miny, mut maxy) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
    for i in 0..QUICK_ITERS {
        let (nx, ny) = step(a, b, c, d, x, y);
        x = nx;
        y = ny;
        if i >= BURN_IN {
            minx = minx.min(x);
            maxx = maxx.max(x);
            miny = miny.min(y);
            maxy = maxy.max(y);
        }
    }
    let (x_end, y_end) = (x, y);
    if maxx <= minx || maxy <= miny {
        minx = x - 0.5;
        maxx = x + 0.5;
        miny = y - 0.5;
        maxy = y + 0.5;
    }
    let spanx = (maxx - minx).max(1e-6);
    let spany = (maxy - miny).max(1e-6);
    let mut seen = vec![false; QUICK_GRID * QUICK_GRID];
    let (mut x2, mut y2) = (x0, y0);
    for i in 0..QUICK_ITERS {
        let (nx, ny) = step(a, b, c, d, x2, y2);
        x2 = nx;
        y2 = ny;
        if i >= BURN_IN {
            let u = ((x2 - minx) / spanx).clamp(0.0, 0.999);
            let v = ((y2 - miny) / spany).clamp(0.0, 0.999);
            seen[(v * QUICK_GRID as f32) as usize * QUICK_GRID + (u * QUICK_GRID as f32) as usize] = true;
        }
    }
    let occ = seen.iter().filter(|s| **s).count();
    Quick { minx, maxx, miny, maxy, x_end, y_end, occ }
}

struct Trial {
    minx: f32,
    maxx: f32,
    miny: f32,
    maxy: f32,
    x_end: f32,
    y_end: f32,
    dim: f32,
    occ: usize,
}

/// One clean bbox pass, then a second pass that bins visited cells at two
/// grid scales so the ratio approximates a box-counting fractal dimension.
fn trial(a: f32, b: f32, c: f32, d: f32) -> Option<Trial> {
    let (mut x, mut y) = (0.1f32, 0.1f32);
    let (mut minx, mut maxx, mut miny, mut maxy) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
    for i in 0..TRIAL_ITERS {
        let (nx, ny) = step(a, b, c, d, x, y);
        if !nx.is_finite() || !ny.is_finite() || nx.abs() > 40.0 || ny.abs() > 40.0 {
            return None;
        }
        x = nx;
        y = ny;
        if i >= BURN_IN {
            minx = minx.min(x);
            maxx = maxx.max(x);
            miny = miny.min(y);
            maxy = maxy.max(y);
        }
    }
    if maxx <= minx || maxy <= miny {
        return None;
    }
    let spanx = (maxx - minx).max(1e-4);
    let spany = (maxy - miny).max(1e-4);
    let mut fine = vec![false; FINE * FINE];
    let mut coarse = vec![false; COARSE * COARSE];
    let (mut x2, mut y2) = (0.1f32, 0.1f32);
    for i in 0..TRIAL_ITERS {
        let (nx, ny) = step(a, b, c, d, x2, y2);
        x2 = nx;
        y2 = ny;
        if i >= BURN_IN {
            let u = ((x2 - minx) / spanx).clamp(0.0, 0.999);
            let v = ((y2 - miny) / spany).clamp(0.0, 0.999);
            fine[(v * FINE as f32) as usize * FINE + (u * FINE as f32) as usize] = true;
            coarse[(v * COARSE as f32) as usize * COARSE + (u * COARSE as f32) as usize] = true;
        }
    }
    let nf = fine.iter().filter(|b| **b).count().max(1);
    let nc = coarse.iter().filter(|b| **b).count().max(1);
    let dim = (nf as f32 / nc as f32).ln() / (FINE as f32 / COARSE as f32).ln();
    Some(Trial { minx, maxx, miny, maxy, x_end: x2, y_end: y2, dim, occ: nf })
}

fn seed_hue(seed: u64) -> f32 {
    let h = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    ((h >> 40) % 360) as f32
}

fn build(seed: u64) -> Geom {
    let mut rng = StdRng::seed_from_u64(seed ^ 0xC11F_D00D_u64);
    let mut best: Option<(f32, f32, f32, f32, Trial, f32)> = None;
    for _ in 0..TRIALS {
        let a = rng.random_range(-PARAM_RANGE..PARAM_RANGE);
        let b = rng.random_range(-PARAM_RANGE..PARAM_RANGE);
        let c = rng.random_range(-PARAM_RANGE..PARAM_RANGE);
        let d = rng.random_range(-PARAM_RANGE..PARAM_RANGE);
        let Some(tr) = trial(a, b, c, d) else { continue };
        if tr.occ < MIN_OCC {
            continue;
        }
        let score = -((tr.dim - TARGET_DIM).abs());
        let better = best.as_ref().map(|(_, _, _, _, _, s)| score > *s).unwrap_or(true);
        if better {
            best = Some((a, b, c, d, tr, score));
        }
    }
    let (a, b, c, d, tr) = match best {
        Some((a, b, c, d, tr, _)) => (a, b, c, d, tr),
        None => {
            let (a, b, c, d) = (-1.4, 1.6, 1.0, 0.7);
            let tr = trial(a, b, c, d)
                .unwrap_or(Trial { minx: -1.7, maxx: 1.7, miny: -1.7, maxy: 1.7, x_end: 0.1, y_end: 0.1, dim: 1.0, occ: 0 });
            (a, b, c, d, tr)
        }
    };
    Geom { a, b, c, d, x0: tr.x_end, y0: tr.y_end, hue0: seed_hue(seed), variant: seed & 1 == 1 }
}

fn put(grid: &mut Grid, w: usize, h: usize, x: i32, y: i32, cell: Cell) {
    if x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h {
        grid[y as usize][x as usize] = cell;
    }
}

pub(crate) fn draw_sonnet_2_clifford(
    grid: &mut Grid,
    w: usize,
    h: usize,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    k: &CliffordKnobs,
) {
    CACHE.with(|cell| {
        let mut slot = cell.borrow_mut();
        let stale = slot.as_ref().map(|c| c.seed != seed).unwrap_or(true);
        if stale {
            *slot = Some(Cached { seed, geom: build(seed), counts: Vec::new() });
        }
        let c = slot.as_mut().unwrap();
        render(grid, w, h, palette, t, k, c);
    });
}

fn render(grid: &mut Grid, w: usize, h: usize, palette: &[Color; 5], t: f32, k: &CliffordKnobs, c: &mut Cached) {
    measure_layer("sonnet-2-clifford", "clear", || {
        let vign_steps = 20usize;
        let core = lighten(palette[0], 10);
        let edge = darken(palette[0], 55);
        let ramp: Vec<Color> = (0..vign_steps)
            .map(|i| lerp_color(core, edge, i as f32 / (vign_steps - 1) as f32))
            .collect();
        let cx_f = w as f32 * 0.5;
        let cy_f = h as f32 * 0.5;
        let rx = cx_f.max(1.0);
        let ry = cy_f.max(1.0);
        let col_r2: Vec<f32> = (0..w).map(|x| { let nx = (x as f32 + 0.5 - cx_f) / rx; nx * nx }).collect();
        for y in 0..h {
            let ny = (y as f32 + 0.5 - cy_f) / ry;
            let ry2 = ny * ny;
            let row = &mut grid[y];
            for x in 0..w {
                let r2 = (col_r2[x] + ry2).min(1.0);
                let idx = (r2 * (vign_steps - 1) as f32) as usize;
                row[x] = Cell::with_bg(' ', Color::Reset, ramp[idx]);
            }
        }
    });
    if w < 8 || h < 6 {
        return;
    }

    let geom = c.geom;
    if c.counts.len() != w * h {
        c.counts = vec![0u32; w * h];
    } else {
        c.counts.iter_mut().for_each(|v| *v = 0);
    }

    let g = if t > 0.0 { t * k.speed.max(0.0) } else { 0.0 };
    let period = k.period.max(2.0);
    let phase = TAU * g / period;
    let drift = k.drift.max(0.0);
    let mut a = geom.a + drift * phase.sin();
    let mut b = geom.b + drift * (phase * 1.3).sin();
    let mut c_param = geom.c + drift * (phase * 0.7).sin();
    let mut d = geom.d + drift * (phase * 1.1).sin();

    let cx = w as f32 * 0.5;
    let cy = h as f32 * 0.5;
    let zoom = k.scale.max(0.05);
    let mut q = measure_layer("sonnet-2-clifford", "aim", || quick_bbox(a, b, c_param, d, geom.x0, geom.y0));
    if q.occ < MIN_QUICK_OCC {
        a = geom.a;
        b = geom.b;
        c_param = geom.c;
        d = geom.d;
        q = quick_bbox(a, b, c_param, d, geom.x0, geom.y0);
    }
    let ax0 = (q.minx + q.maxx) * 0.5;
    let ay0 = (q.miny + q.maxy) * 0.5;
    let ex = ((q.maxx - q.minx) * 0.5).max(0.15) * 1.2;
    let ey = ((q.maxy - q.miny) * 0.5).max(0.15) * 1.2;
    let (start_x, start_y) = (q.x_end, q.y_end);
    let sx = zoom * w as f32 * 0.42 / ex;
    let sy = zoom * h as f32 * 0.42 / ey;

    let n = k.density.max(2000.0) as u32;
    let comet_n = (k.comet.round() as u32).clamp(10, 2000);
    let comet_start = n.saturating_sub(comet_n);

    let counts = &mut c.counts;
    let (trail, max_count) = measure_layer("sonnet-2-clifford", "orbit", || {
        let mut x = start_x;
        let mut y = start_y;
        let mut trail: Vec<(f32, f32)> = Vec::with_capacity(comet_n as usize);
        let mut max_count = 1u32;
        for i in 0..n {
            let (nx, ny) = step(a, b, c_param, d, x, y);
            x = nx;
            y = ny;
            let px = cx + (x - ax0) * sx;
            let py = cy + (y - ay0) * sy;
            if px >= 0.0 && py >= 0.0 && (px as usize) < w && (py as usize) < h {
                let idx = py as usize * w + px as usize;
                let v = counts[idx] + 1;
                counts[idx] = v;
                if v > max_count {
                    max_count = v;
                }
            }
            if i >= comet_start {
                trail.push((px, py));
            }
        }
        (trail, max_count)
    });

    let base_hue = (geom.hue0 as f64 + k.hue as f64).rem_euclid(360.0);
    let spread = k.spread.max(0.0) as f64;
    let glow = k.glow.max(0.05);
    let glyphs = ['\u{b7}', '\u{2219}', '\u{2022}', '\u{25cf}', '\u{2b24}'];
    measure_layer("sonnet-2-clifford", "field", || {
        let cap = max_count.min(LUT_CAP);
        let mut lut: Vec<(char, Color)> = Vec::with_capacity(cap as usize + 1);
        lut.push((' ', Color::Reset));
        for cnt in 1..=cap {
            let density_norm = (cnt as f32 / cap as f32).powf(1.0 / glow).clamp(0.0, 1.0);
            let gi = ((density_norm * (glyphs.len() - 1) as f32).round() as usize).min(glyphs.len() - 1);
            let hue = (base_hue + spread * (1.0 - density_norm as f64)).rem_euclid(360.0);
            let light = 0.30 + 0.35 * density_norm as f64;
            lut.push((glyphs[gi], hsl_to_rgb(hue, 0.62, light)));
        }
        for y in 0..h {
            for x in 0..w {
                let v = c.counts[y * w + x];
                if v == 0 {
                    continue;
                }
                let ci = v.min(cap) as usize;
                let (ch, col) = lut[ci];
                let prev_bg = grid[y][x].bg;
                grid[y][x] = Cell::with_bg(ch, col, prev_bg);
            }
        }
    });

    measure_layer("sonnet-2-clifford", "comet", || {
        let accent = lighten(palette[3], 20);
        let len = trail.len();
        if len < 2 {
            return;
        }
        for (i, (px, py)) in trail.iter().enumerate() {
            let age = i as f32 / (len - 1) as f32;
            let fg = lerp_color(darken(accent, 65), lighten(accent, 25), age);
            let ch = if i == len - 1 {
                if geom.variant { '\u{25c6}' } else { '@' }
            } else if age > 0.6 {
                if geom.variant { '\u{2022}' } else { '*' }
            } else if geom.variant {
                '\u{2219}'
            } else {
                '.'
            };
            put(grid, w, h, *px as i32, *py as i32, Cell::with_bg(ch, fg, grid[(*py as usize).min(h - 1)][(*px as usize).min(w - 1)].bg));
        }
    });

    measure_layer("sonnet-2-clifford", "frame", || {
        let half_w = (ex * sx) as i32;
        let half_h = (ey * sy) as i32;
        let x0 = cx as i32 - half_w;
        let x1 = cx as i32 + half_w;
        let y0 = cy as i32 - half_h;
        let y1 = cy as i32 + half_h;
        let rim = darken(palette[2], 15);
        put(grid, w, h, x0, y0, Cell::new('\u{250c}', rim));
        put(grid, w, h, x1, y0, Cell::new('\u{2510}', rim));
        put(grid, w, h, x0, y1, Cell::new('\u{2514}', rim));
        put(grid, w, h, x1, y1, Cell::new('\u{2518}', rim));
        for xx in (x0 + 1)..x1 {
            if (xx - x0) % 6 == 0 {
                put(grid, w, h, xx, y0, Cell::new('\u{2500}', rim));
                put(grid, w, h, xx, y1, Cell::new('\u{2500}', rim));
            }
        }
        for yy in (y0 + 1)..y1 {
            if (yy - y0) % 3 == 0 {
                put(grid, w, h, x0, yy, Cell::new('\u{2502}', rim));
                put(grid, w, h, x1, yy, Cell::new('\u{2502}', rim));
            }
        }
    });
}

pub(crate) fn cli_sonnet_2_clifford(
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
    let mut k = CliffordKnobs::from_env();
    let pos: Vec<f32> = args.iter().skip(4).filter_map(|a| a.parse().ok()).collect();
    let slots: [&mut f32; 9] = [
        &mut k.speed,
        &mut k.hue,
        &mut k.spread,
        &mut k.drift,
        &mut k.period,
        &mut k.density,
        &mut k.comet,
        &mut k.glow,
        &mut k.scale,
    ];
    for (slot, v) in slots.into_iter().zip(pos.iter()) {
        *slot = *v;
    }
    draw_sonnet_2_clifford(&mut grid, width, height, seed, &palette, t_anim, &k);
    (grid, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(w: usize, h: usize, seed: u64, t: f32) -> String {
        let mut g = vec![vec![Cell::blank(); w]; h];
        let p = crate::color::make_palette(seed);
        let k = CliffordKnobs::from_env();
        draw_sonnet_2_clifford(&mut g, w, h, seed, &p, t, &k);
        g.iter()
            .map(|row| row.iter().map(|c| c.ch).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn snapshot_sonnet_2_clifford_static() {
        insta::assert_snapshot!("sonnet_2_clifford_80x24_t0", run(80, 24, 42, 0.0));
    }

    #[test]
    fn snapshot_sonnet_2_clifford_moving() {
        insta::assert_snapshot!("sonnet_2_clifford_80x24_t20", run(80, 24, 42, 20.0));
    }
}
