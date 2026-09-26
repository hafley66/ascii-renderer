use std::collections::{HashMap, VecDeque};
use std::f64::consts::{PI, TAU};

use crate::_0_profile::measure_layer;
use crate::color::lerp_color;
use crate::opts::param_f32;
use crate::pp::{pp_line, pp_put};
use crate::registry::{AnimKind, Mode, ModeFrame, Param};
use crate::types::{Cell, Grid};
use crossterm::style::Color;

pub(super) struct Hyperhex2;
pub(super) static MODE: Hyperhex2 = Hyperhex2;

const NAME: &str = "hyperhex-2";
const HELP: &str = "hyperhex-2: hyperbolic {6,4} tiling [depth] [skew] [spin] [breath] [swell] [passage] [fill] [wound]";

const PARAMS: &[Param] = &[
    param!("DEPTH", "reflection depth", 2.0, 12.0, 6.0, 1.0),
    param!("SKEW", "asymmetry shift", 0.0, 1.0, 0.4, 0.01),
    param!("SPIN", "spin rad/s", 0.0, 2.0, 0.35, 0.01),
    param!("BREATH", "breath rate", 0.0, 3.0, 0.5, 0.05),
    param!("SWELL", "swell amplitude", 0.0, 0.8, 0.45, 0.01),
    param!("PASSAGE", "passage speed", 0.0, 2.0, 0.4, 0.01),
    param!("FILL", "interior fill", 0.0, 1.0, 0.7, 0.01),
    param!("WOUND", "wound strength", 0.0, 1.0, 0.7, 0.01),
];

/// Deterministic per-cell value, independent of the frame RNG stream.
fn splitmix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

fn unit(seed: u64, layer: u64, idx: u64) -> f64 {
    let h = splitmix(seed ^ splitmix(layer.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ idx));
    ((h >> 11) as f64) / ((1u64 << 53) as f64)
}

type C = (f64, f64);

fn add(a: C, b: C) -> C {
    (a.0 + b.0, a.1 + b.1)
}
fn mul(a: C, b: C) -> C {
    (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0)
}
fn div(a: C, b: C) -> C {
    let d = b.0 * b.0 + b.1 * b.1;
    ((a.0 * b.0 + a.1 * b.1) / d, (a.1 * b.0 - a.0 * b.1) / d)
}
fn conj(a: C) -> C {
    (a.0, -a.1)
}
fn cabs2(a: C) -> f64 {
    a.0 * a.0 + a.1 * a.1
}

/// A disk isometry: z -> (a z + b)/(c z + d), optionally followed by conjugation.
#[derive(Clone, Copy)]
struct Iso {
    a: C,
    b: C,
    c: C,
    d: C,
    anti: bool,
}

fn apply(m: &Iso, z: C) -> C {
    let w = div(add(mul(m.a, z), m.b), add(mul(m.c, z), m.d));
    if m.anti {
        conj(w)
    } else {
        w
    }
}

fn compose(f: &Iso, g: &Iso) -> Iso {
    let fh = if g.anti {
        Iso {
            a: conj(f.a),
            b: conj(f.b),
            c: conj(f.c),
            d: conj(f.d),
            anti: false,
        }
    } else {
        *f
    };
    let a = add(mul(fh.a, g.a), mul(fh.b, g.c));
    let b = add(mul(fh.a, g.b), mul(fh.b, g.d));
    let c = add(mul(fh.c, g.a), mul(fh.d, g.c));
    let d = add(mul(fh.c, g.b), mul(fh.d, g.d));
    Iso {
        a,
        b,
        c,
        d,
        anti: if g.anti { !f.anti } else { f.anti },
    }
}

fn id_iso() -> Iso {
    Iso {
        a: (1.0, 0.0),
        b: (0.0, 0.0),
        c: (0.0, 0.0),
        d: (1.0, 0.0),
        anti: false,
    }
}

fn rot_iso(th: f64) -> Iso {
    Iso {
        a: (th.cos(), th.sin()),
        b: (0.0, 0.0),
        c: (0.0, 0.0),
        d: (th.cos(), -th.sin()),
        anti: false,
    }
}

/// Disk automorphism z -> (z - a)/(1 - conj(a) z): a Mobius translation.
fn trans_iso(a: C) -> Iso {
    Iso {
        a: (1.0, 0.0),
        b: (-a.0, -a.1),
        c: (-a.0, a.1),
        d: (1.0, 0.0),
        anti: false,
    }
}

/// Gaussian pulse for the heartbeat envelope.
fn gauss(u: f64, mu: f64, s: f64) -> f64 {
    let x = (u - mu) / s;
    (-0.5 * x * x).exp()
}

/// Reflection in the geodesic circle centred at `c` (orthogonal to the rim).
fn refl_iso(c: C) -> Iso {
    Iso {
        a: conj(c),
        b: (-1.0, 0.0),
        c: (1.0, 0.0),
        d: (-c.0, -c.1),
        anti: true,
    }
}

/// Circle through two interior points that is orthogonal to the unit circle.
fn circle_through(p: C, q: C) -> Option<(C, f64)> {
    let bp = (cabs2(p) + 1.0) / 2.0;
    let bq = (cabs2(q) + 1.0) / 2.0;
    let det = p.0 * q.1 - p.1 * q.0;
    if det.abs() < 1e-12 {
        return None;
    }
    let x = (bp * q.1 - bq * p.1) / det;
    let y = (p.0 * bq - q.0 * bp) / det;
    let r2 = x * x + y * y - 1.0;
    if r2 <= 1e-12 {
        return None;
    }
    Some(((x, y), r2.sqrt()))
}

/// Fundamental hexagon of the {6,4} tiling: interior angle pi/2, centre angle pi/6.
/// `skew` perturbs the vertex angles so the tiling is not regular.
fn fundamental(skew: f64, seed: u64) -> [C; 6] {
    let r = ((PI / 6.0).cos() / (PI / 6.0).sin() * (PI / 4.0).cos() / (PI / 4.0).sin()).acosh();
    let pat = [0.0, 1.0, -0.9, 1.0, -1.0, 0.7];
    let mut v = [(0.0, 0.0); 6];
    for (k, slot) in v.iter_mut().enumerate() {
        let jitter = unit(seed, 21, k as u64) - 0.5;
        let phi = k as f64 * PI / 3.0 + skew * 0.09 * (pat[k] + 0.25 * jitter);
        *slot = (r * phi.cos(), r * phi.sin());
    }
    v
}

struct Tile {
    m: Iso,
    depth: u32,
    par: u8,
}

fn build_tiling(
    seed: u64,
    depth: u32,
    skew: f64,
    min_edge: f32,
    s: f32,
) -> Vec<Tile> {
    let fund = fundamental(skew, seed);
    let mut tiles: Vec<Tile> = Vec::new();
    let mut seen: HashMap<(i64, i64), ()> = HashMap::new();
    let mut queue: VecDeque<(Iso, u32, u8)> = VecDeque::new();
    queue.push_back((id_iso(), 0, 0));
    seen.insert((0, 0), ());
    while let Some((m, d, par)) = queue.pop_front() {
        tiles.push(Tile { m, depth: d, par });
        if d >= depth {
            continue;
        }
        for k in 0..6 {
            let p = apply(&m, fund[k]);
            let q = apply(&m, fund[(k + 1) % 6]);
            let dx = (p.0 - q.0) as f32 * s;
            let dy = (p.1 - q.1) as f32 * s * 0.5;
            if (dx * dx + dy * dy).sqrt() < min_edge {
                continue;
            }
            let Some((cc, _)) = circle_through(p, q) else {
                continue;
            };
            let nm = compose(&refl_iso(cc), &m);
            let cen = apply(&nm, (0.0, 0.0));
            let key = ((cen.0 * 1e4).round() as i64, (cen.1 * 1e4).round() as i64);
            if seen.contains_key(&key) {
                continue;
            }
            seen.insert(key, ());
            queue.push_back((nm, d + 1, par ^ 1));
        }
    }
    tiles
}

struct Knobs {
    depth: u32,
    skew: f64,
    spin: f64,
    breath: f64,
    swell: f64,
    passage: f64,
    fill: f64,
    wound: f64,
}

fn knob(frame: &ModeFrame<'_>, i: usize, default: f32, lo: f32, hi: f32) -> f32 {
    frame
        .args
        .get(i + 4)
        .and_then(|s| s.parse::<f32>().ok())
        .or_else(|| frame.param_values.and_then(|v| v.get(i).copied()))
        .unwrap_or_else(|| param_f32(PARAMS[i].key, default))
        .clamp(lo, hi)
}

fn knobs(frame: &ModeFrame<'_>) -> Knobs {
    Knobs {
        depth: knob(frame, 0, 6.0, 2.0, 12.0).round() as u32,
        skew: knob(frame, 1, 0.4, 0.0, 1.0) as f64,
        spin: knob(frame, 2, 0.35, 0.0, 2.0) as f64,
        breath: knob(frame, 3, 0.5, 0.0, 3.0) as f64,
        swell: knob(frame, 4, 0.45, 0.0, 0.8) as f64,
        passage: knob(frame, 5, 0.4, 0.0, 2.0) as f64,
        fill: knob(frame, 6, 0.7, 0.0, 1.0) as f64,
        wound: knob(frame, 7, 0.7, 0.0, 1.0) as f64,
    }
}

struct DrawTile {
    uv: [C; 6],
    v: [(i32, i32); 6],
    cx: i32,
    cy: i32,
    edge: f32,
    col: Color,
}

/// Even-odd point-in-polygon on the straight screen frame of a tile.
fn point_in_poly(px: f32, py: f32, v: &[(i32, i32); 6]) -> bool {
    let mut inside = false;
    let mut j = 5;
    for i in 0..6 {
        let xi = v[i].0 as f32;
        let yi = v[i].1 as f32;
        let xj = v[j].0 as f32;
        let yj = v[j].1 as f32;
        if (yi > py) != (yj > py) && px < (xj - xi) * (py - yi) / (yj - yi) + xi {
            inside = !inside;
        }
        j = i;
    }
    inside
}

fn draw_geodesic(grid: &mut Grid, p: C, q: C, cx0: f32, cy0: f32, s: f32, fg: Color) {
    let to_x = |z: C| (cx0 + s * z.0 as f32).round() as i32;
    let to_y = |z: C| (cy0 + s * 0.5 * z.1 as f32).round() as i32;
    let Some((c, r)) = circle_through(p, q) else {
        pp_line(grid, to_x(p), to_y(p), to_x(q), to_y(q), fg);
        return;
    };
    let tp = (p.1 - c.1).atan2(p.0 - c.0);
    let tq = (q.1 - c.1).atan2(q.0 - c.0);
    let mut d = tq - tp;
    while d > PI {
        d -= TAU;
    }
    while d < -PI {
        d += TAU;
    }
    let mid = (c.0 + r * (tp + 0.5 * d).cos(), c.1 + r * (tp + 0.5 * d).sin());
    if cabs2(mid) > 1.0 {
        d += if d > 0.0 { -TAU } else { TAU };
    }
    let chord = (((p.0 - q.0) as f32 * s).powi(2) + ((p.1 - q.1) as f32 * s * 0.5).powi(2)).sqrt();
    let n = ((chord * 1.4).ceil() as i64).clamp(2, 40);
    let mut prev = (to_x(p), to_y(p));
    pp_put(grid, prev.0, prev.1, '.', fg);
    for i in 1..=n {
        let a = tp + d * (i as f64) / (n as f64);
        let zx = c.0 + r * a.cos();
        let zy = c.1 + r * a.sin();
        let cur = (to_x((zx, zy)), to_y((zx, zy)));
        pp_line(grid, prev.0, prev.1, cur.0, cur.1, fg);
        prev = cur;
    }
}

fn draw(frame: &mut ModeFrame<'_>, k: &Knobs) {
    let (w, h) = (frame.width, frame.height);
    if w == 0 || h == 0 {
        return;
    }
    let seed = frame.seed;
    let grid = &mut *frame.grid;
    let pal = frame.palette;
    let t = frame.time;

    let s = (w as f32 * 0.5).min(h as f32) * 0.94;
    let cx0 = w as f32 * 0.5;
    let cy0 = h as f32 * 0.5;
    let fund = fundamental(k.skew, seed);

    let tiles = measure_layer(NAME, "topology", || {
        build_tiling(seed, k.depth, k.skew, 1.1, s)
    });

    // Fixed asymmetric shift, then a Mobius translation on a heartbeat envelope,
    // then spin. All conformal, so the {6,4} tiling stays valid while it breathes.
    let shift_dir = 0.6 + 2.0 * unit(seed, 31, 0);
    let shift = trans_iso((
        k.skew * 0.6 * shift_dir.cos(),
        k.skew * 0.6 * shift_dir.sin(),
    ));
    let period = if k.breath > 0.01 { 1.0 / k.breath } else { 1.0e9 };
    let u = (t as f64 / period).fract();
    let env = gauss(u, 0.10, 0.045) + 0.65 * gauss(u, 0.28, 0.05);
    let amp = (k.swell * env).min(0.85);
    let aa = (
        amp * (k.passage * t as f64).cos(),
        amp * (k.passage * t as f64).sin(),
    );
    let g = compose(&rot_iso(k.spin * t as f64), &compose(&trans_iso(aa), &shift));
    let mut dt: Vec<DrawTile> = Vec::with_capacity(tiles.len());
    measure_layer(NAME, "frame", || {
        for tile in &tiles {
            let m2 = compose(&g, &tile.m);
            let mut uv = [(0.0, 0.0); 6];
            let mut v = [(0, 0); 6];
            for (idx, slot) in uv.iter_mut().enumerate() {
                let z = apply(&m2, fund[idx]);
                *slot = z;
                v[idx] = (
                    (cx0 + s * z.0 as f32).round() as i32,
                    (cy0 + s * 0.5 * z.1 as f32).round() as i32,
                );
            }
            let mut edge = 0.0f32;
            for i in 0..6 {
                let (x0, y0) = v[i];
                let (x1, y1) = v[(i + 1) % 6];
                edge = edge.max((((x0 - x1) as f32).powi(2) + ((y0 - y1) as f32).powi(2)).sqrt());
            }
            if edge < 0.5 {
                continue;
            }
            let dep = tile.depth as f32 / k.depth.max(1) as f32;
            let band = (dep * 0.7 + t * 0.18).rem_euclid(1.0);
            let mut col = lerp_color(pal[1], pal[3], band);
            if tile.par == 1 {
                col = lerp_color(col, pal[2], 0.4);
            }
            let cen = apply(&m2, (0.0, 0.0));
            dt.push(DrawTile {
                uv,
                v,
                cx: (cx0 + s * cen.0 as f32).round() as i32,
                cy: (cy0 + s * 0.5 * cen.1 as f32).round() as i32,
                edge,
                col,
            });
        }
    });

    let large: Vec<usize> = dt
        .iter()
        .enumerate()
        .filter(|(_, tile)| tile.edge >= 2.0)
        .map(|(i, _)| i)
        .collect();
    let cycle = (t as f64 / period).floor() as u64;
    let wound_active = k.wound > 0.05 && !large.is_empty() && u > 0.40 && u < 0.62;
    let wound_idx = if wound_active {
        large[(unit(seed, 41, cycle) * large.len() as f64) as usize % large.len()]
    } else {
        usize::MAX
    };

    measure_layer(NAME, "ground", || {
        for y in 0..h {
            for x in 0..w {
                let dx = (x as f32 * 2.0 - w as f32) / w as f32;
                let dy = (y as f32 - h as f32 * 0.5) / h as f32;
                let d = (dx * dx + dy * dy).sqrt().min(1.0);
                let bg = lerp_color(pal[0], pal[2], d * 0.30);
                let dust = unit(seed, 10, (y * w + x) as u64) > 0.99;
                let ch = if dust { '.' } else { ' ' };
                grid[y][x] = Cell::with_bg(ch, lerp_color(pal[0], pal[1], 0.5), bg);
            }
        }
    });

    measure_layer(NAME, "fill", || {
        const RAMP: &[u8] = b" .:-=+*#%@";
        if k.fill <= 0.05 {
            return;
        }
        for (ti, tile) in dt.iter().enumerate() {
            if tile.edge < 2.0 {
                continue;
            }
            let (mut x0, mut x1) = (i32::MAX, i32::MIN);
            let (mut y0, mut y1) = (i32::MAX, i32::MIN);
            for &(x, y) in tile.v.iter() {
                x0 = x0.min(x);
                x1 = x1.max(x);
                y0 = y0.min(y);
                y1 = y1.max(y);
            }
            x0 = x0.max(0);
            y0 = y0.max(0);
            x1 = x1.min(w as i32 - 1);
            y1 = y1.min(h as i32 - 1);
            let mut rad = 1.0f32;
            for &(x, y) in tile.v.iter() {
                let dx = (x - tile.cx) as f32;
                let dy = (y - tile.cy) as f32;
                rad = rad.max((dx * dx + dy * dy).sqrt());
            }
            let wounded = ti == wound_idx;
            for y in y0..=y1 {
                for x in x0..=x1 {
                    if !point_in_poly(x as f32 + 0.5, y as f32 + 0.5, &tile.v) {
                        continue;
                    }
                    let dx = (x - tile.cx) as f32;
                    let dy = (y - tile.cy) as f32;
                    let dnorm = ((dx * dx + dy * dy).sqrt() / rad).min(1.0);
                    let mut inten = (1.0 - dnorm) * k.fill as f32;
                    if wounded {
                        inten = 1.0 - inten;
                    }
                    if inten < 0.06 {
                        continue;
                    }
                    let gi = ((inten * (RAMP.len() as f32 - 1.0)).round() as usize).min(RAMP.len() - 1);
                    let col = if wounded {
                        pal[4]
                    } else {
                        lerp_color(pal[0], tile.col, 0.35 + 0.65 * inten)
                    };
                    pp_put(grid, x, y, RAMP[gi] as char, col);
                }
            }
        }
    });

    measure_layer(NAME, "edges", || {
        for tile in &dt {
            for i in 0..6 {
                draw_geodesic(grid, tile.uv[i], tile.uv[(i + 1) % 6], cx0, cy0, s, tile.col);
            }
        }
    });
}

impl Mode for Hyperhex2 {
    fn name(&self) -> &'static str {
        NAME
    }
    fn help(&self) -> &'static str {
        HELP
    }
    fn animation(&self) -> AnimKind {
        AnimKind::Iterate
    }
    fn params(&self) -> &'static [Param] {
        PARAMS
    }
    fn render(&self, frame: &mut ModeFrame<'_>) {
        let k = knobs(frame);
        draw(frame, &k);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::make_palette;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    fn render_at(w: usize, h: usize, seed: u64, time: f32) -> String {
        let mut grid: Grid = vec![vec![Cell::blank(); w]; h];
        let palette = make_palette(seed);
        let mut rng = StdRng::seed_from_u64(seed);
        let args: Vec<String> = Vec::new();
        let mut frame = ModeFrame {
            grid: &mut grid,
            width: w,
            height: h,
            seed,
            palette: &palette,
            rng: &mut rng,
            time,
            args: &args,
            param_values: None,
        };
        MODE.render(&mut frame);
        grid.iter()
            .map(|row| row.iter().map(|c| c.ch).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn snapshot_80x24() {
        insta::assert_snapshot!("hyperhex_2_80x24", render_at(80, 24, 42, 0.0));
    }

    #[test]
    fn snapshot_t6() {
        insta::assert_snapshot!("hyperhex_2_t6", render_at(80, 24, 42, 6.0));
    }

    #[test]
    fn deterministic() {
        assert_eq!(render_at(80, 24, 42, 0.0), render_at(80, 24, 42, 0.0));
    }

    #[test]
    fn seed_sensitive() {
        assert_ne!(render_at(80, 24, 42, 0.0), render_at(80, 24, 43, 0.0));
    }

    #[test]
    fn time_sensitive() {
        assert_ne!(render_at(80, 24, 42, 0.0), render_at(80, 24, 42, 6.0));
    }

    #[test]
    fn frame_cost() {
        if cfg!(debug_assertions) {
            return;
        }
        let (w, h) = (200usize, 60usize);
        let args: Vec<String> = Vec::new();
        let palette = make_palette(42);
        let mut rng = StdRng::seed_from_u64(42);
        let mut grid: Grid = vec![vec![Cell::blank(); w]; h];
        let start = std::time::Instant::now();
        let iters = 20;
        for i in 0..iters {
            let mut frame = ModeFrame {
                grid: &mut grid,
                width: w,
                height: h,
                seed: 42,
                palette: &palette,
                rng: &mut rng,
                time: i as f32,
                args: &args,
                param_values: None,
            };
            MODE.render(&mut frame);
        }
        let avg_ms = start.elapsed().as_secs_f64() * 1000.0 / iters as f64;
        assert!(
            avg_ms < 6.0,
            "hyperhex-2 average frame {avg_ms:.3} ms exceeds 6 ms"
        );
    }
}
