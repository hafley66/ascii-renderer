use std::collections::{HashMap, VecDeque};
use std::f64::consts::{PI, TAU};

use crate::_0_profile::measure_layer;
use crate::color::{hsl_to_rgb, lerp_color};
use crate::opts::param_f32;
use crate::pp::{pp_line, pp_put};
use crate::registry::{AnimKind, Mode, ModeFrame, Param};
use crate::types::{Cell, Grid};
use crossterm::style::Color;

pub(super) struct Hyperhex6;
pub(super) static MODE: Hyperhex6 = Hyperhex6;

const NAME: &str = "hyperhex-6";
const HELP: &str = "hyperhex-6: hyperbolic {6,7} tiling in four acts [depth] [skew] [spin] [breath] [swell] [passage] [fill] [wound] [act] [ghost] [horizon] [twist]";

/// Below this screen edge length a cell is too small to read; skip its fill and arcs.
const EDGE_MIN: f32 = 4.0;

/// Only the largest cells get their geodesic outlines drawn, so the fill lattice
/// stays legible instead of being erased by a thicket of small arcs.
const ARC_MIN: f32 = 6.0;

const PARAMS: &[Param] = &[
    param!("DEPTH", "reflection depth", 2.0, 12.0, 4.0, 1.0),
    param!("SKEW", "asymmetry shift", 0.0, 1.0, 0.55, 0.01),
    param!("SPIN", "spin rad/s", 0.0, 2.0, 0.35, 0.01),
    param!("BREATH", "breath rate", 0.0, 3.0, 0.5, 0.05),
    param!("SWELL", "swell amplitude", 0.0, 0.8, 0.45, 0.01),
    param!("PASSAGE", "passage speed", 0.0, 2.0, 0.4, 0.01),
    param!("FILL", "interior fill", 0.0, 1.0, 0.62, 0.01),
    param!("WOUND", "wound strength", 0.0, 1.0, 0.7, 0.01),
    param!("ACT", "act period s", 8.0, 60.0, 40.0, 1.0),
    param!("GHOST", "ghost dual", 0.0, 1.0, 0.5, 0.01),
    param!("HORIZON", "star horizon", 0.0, 1.0, 0.6, 0.01),
    param!("TWIST", "spiral shear", 0.0, 3.0, 1.1, 0.05),
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

fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
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

/// Fundamental hexagon of the {6,7} tiling: circumradius acosh(cos(pi/7)/sin(pi/6))
/// projected into the disk. `skew` rigidly rotates the seed tile, keeping it regular.
fn fundamental(skew: f64) -> [C; 6] {
    let big_r = ((PI / 7.0).cos() / (PI / 6.0).sin()).acosh();
    let rho = (big_r / 2.0).tanh();
    let mut v = [(0.0, 0.0); 6];
    for (k, slot) in v.iter_mut().enumerate() {
        let phi = k as f64 * PI / 3.0 + skew * 0.35;
        *slot = (rho * phi.cos(), rho * phi.sin());
    }
    v
}

struct Tile {
    m: Iso,
    depth: u32,
    par: u8,
    parent: Option<usize>,
    hops_a: u32,
    hops_b: u32,
}

/// BFS distances in the reflection graph.
fn bfs(adj: &[Vec<usize>], start: usize) -> Vec<u32> {
    let mut d = vec![u32::MAX; adj.len()];
    d[start] = 0;
    let mut q: VecDeque<usize> = VecDeque::new();
    q.push_back(start);
    while let Some(u) = q.pop_front() {
        for &v in &adj[u] {
            if d[v] == u32::MAX {
                d[v] = d[u] + 1;
                q.push_back(v);
            }
        }
    }
    d
}

fn build_tiling(
    depth: u32,
    skew: f64,
    min_edge: f32,
    s: f32,
) -> Vec<Tile> {
    let fund = fundamental(skew);
    let mut tiles: Vec<Tile> = Vec::new();
    let mut seen: HashMap<(i64, i64), ()> = HashMap::new();
    let mut queue: VecDeque<(Iso, u32, u8, Option<usize>)> = VecDeque::new();
    queue.push_back((id_iso(), 0, 0, None));
    seen.insert((0, 0), ());
    while let Some((m, d, par, parent)) = queue.pop_front() {
        let idx = tiles.len();
        tiles.push(Tile {
            m,
            depth: d,
            par,
            parent,
            hops_a: 0,
            hops_b: 0,
        });
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
            queue.push_back((nm, d + 1, par ^ 1, Some(idx)));
        }
    }

    // Undirected adjacency over the reflection tree, then two wave origins on
    // opposite sides of the deepest ring: contagion spreads from both and meets.
    let n = tiles.len();
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
    for i in 0..n {
        if let Some(p) = tiles[i].parent {
            adj[i].push(p);
            adj[p].push(i);
        }
    }
    let maxd = tiles.iter().map(|t| t.depth).max().unwrap_or(0);
    let (mut a, mut b) = (0usize, 0usize);
    let (mut ax, mut bx) = (f64::MAX, f64::MIN);
    for i in 0..n {
        if tiles[i].depth != maxd {
            continue;
        }
        let cen = apply(&tiles[i].m, (0.0, 0.0));
        if cen.0 < ax {
            ax = cen.0;
            a = i;
        }
        if cen.0 > bx {
            bx = cen.0;
            b = i;
        }
    }
    let ha = bfs(&adj, a);
    let hb = bfs(&adj, b);
    for i in 0..n {
        tiles[i].hops_a = ha[i];
        tiles[i].hops_b = hb[i];
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
    act: f64,
    ghost: f64,
    horizon: f64,
    twist: f64,
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
        depth: knob(frame, 0, 4.0, 2.0, 12.0).round() as u32,
        skew: knob(frame, 1, 0.55, 0.0, 1.0) as f64,
        spin: knob(frame, 2, 0.35, 0.0, 2.0) as f64,
        breath: knob(frame, 3, 0.5, 0.0, 3.0) as f64,
        swell: knob(frame, 4, 0.45, 0.0, 0.8) as f64,
        passage: knob(frame, 5, 0.4, 0.0, 2.0) as f64,
        fill: knob(frame, 6, 0.62, 0.0, 1.0) as f64,
        wound: knob(frame, 7, 0.7, 0.0, 1.0) as f64,
        act: knob(frame, 8, 40.0, 8.0, 60.0) as f64,
        ghost: knob(frame, 9, 0.5, 0.0, 1.0) as f64,
        horizon: knob(frame, 10, 0.6, 0.0, 1.0) as f64,
        twist: knob(frame, 11, 1.1, 0.0, 3.0) as f64,
    }
}

/// Four overlapping act windows with partition-of-unity weights.
struct Mood {
    wc: f64,
    wh: f64,
    wf: f64,
    wr: f64,
    amp: f64,
    aa: C,
    u: f64,
    iu: f64,
}

fn act_weight(iu: f64, c: f64) -> f64 {
    let x = 0.5 * (1.0 + (TAU * (iu - c)).cos());
    x * x * x * x
}

fn mood(t: f64, k: &Knobs) -> Mood {
    let period = if k.breath > 0.01 { 1.0 / k.breath } else { 1.0e9 };
    let u = (t / period).fract();
    let env = gauss(u, 0.10, 0.045) + 0.65 * gauss(u, 0.28, 0.05);
    let iu = (t / k.act).rem_euclid(1.0);
    let (mut wc, mut wh, mut wf, mut wr) = (
        act_weight(iu, 0.125),
        act_weight(iu, 0.375),
        act_weight(iu, 0.625),
        act_weight(iu, 0.875),
    );
    let sum = wc + wh + wf + wr;
    if sum > 1e-9 {
        wc /= sum;
        wh /= sum;
        wf /= sum;
        wr /= sum;
    }
    let gain = 0.35 * wc + 1.0 * wh + 0.40 * wf + 0.50 * wr;
    let amp = (k.swell * env * gain).min(0.85);
    let aa = (amp * (k.passage * t).cos(), amp * (k.passage * t).sin());
    Mood {
        wc,
        wh,
        wf,
        wr,
        amp,
        aa,
        u,
        iu,
    }
}

/// Conformal view for a clock time, so the ghost layer can be sampled at a lag.
fn view(m: &Mood, k: &Knobs, shift: &Iso, t: f64) -> Iso {
    compose(
        &rot_iso(k.spin * t),
        &compose(&trans_iso(m.aa), shift),
    )
}

struct DrawTile {
    uv: [C; 6],
    v: [(i32, i32); 6],
    cx: i32,
    cy: i32,
    edge: f32,
    dep: f32,
    col: Color,
    band: f32,
    m: Iso,
    parent_dt: Option<usize>,
    hops_a: u32,
    hops_b: u32,
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

/// Draw the geodesic arc from `p` to `q`, skipping a fraction `gap` at each end
/// so fracture acts leave visible openings between cells.
fn draw_geodesic(grid: &mut Grid, p: C, q: C, cx0: f32, cy0: f32, s: f32, fg: Color, gap: f64) {
    let to_x = |z: C| (cx0 + s * z.0 as f32).round() as i32;
    let to_y = |z: C| (cy0 + s * 0.5 * z.1 as f32).round() as i32;
    let (cc, r) = match circle_through(p, q) {
        Some(v) => v,
        None => {
            pp_line(grid, to_x(p), to_y(p), to_x(q), to_y(q), fg);
            return;
        }
    };
    let tp = (p.1 - cc.1).atan2(p.0 - cc.0);
    let tq = (q.1 - cc.1).atan2(q.0 - cc.0);
    let mut d = tq - tp;
    while d > PI {
        d -= TAU;
    }
    while d < -PI {
        d += TAU;
    }
    let mid = (cc.0 + r * (tp + 0.5 * d).cos(), cc.1 + r * (tp + 0.5 * d).sin());
    if cabs2(mid) > 1.0 {
        d += if d > 0.0 { -TAU } else { TAU };
    }
    let chord = (((p.0 - q.0) as f32 * s).powi(2) + ((p.1 - q.1) as f32 * s * 0.5).powi(2)).sqrt();
    let n = ((chord * 1.4).ceil() as i64).clamp(2, 48);
    let lo = (gap * n as f64).round() as i64;
    let hi = ((1.0 - gap) * n as f64).round() as i64;
    if hi - lo < 1 {
        return;
    }
    let mut prev: Option<(i32, i32)> = None;
    for i in lo..=hi {
        let f = i as f64 / n as f64;
        let a = tp + d * f;
        let cur = (
            to_x((cc.0 + r * a.cos(), cc.1 + r * a.sin())),
            to_y((cc.0 + r * a.cos(), cc.1 + r * a.sin())),
        );
        if let Some(pv) = prev {
            pp_line(grid, pv.0, pv.1, cur.0, cur.1, fg);
        }
        prev = Some(cur);
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
    let t = frame.time as f64;

    let s = (w as f32 * 0.5).min(h as f32) * 0.94;
    let cx0 = w as f32 * 0.5;
    let cy0 = h as f32 * 0.5;
    let fund = fundamental(k.skew);

    let tiles = measure_layer(NAME, "topology", || {
        build_tiling(k.depth, k.skew, 1.1, s)
    });

    let mood_t = mood(t, k);
    // The ghost dual trails the view; it trails further apart during fracture,
    // so the two layers shear into a wider moire.
    let lag = 0.6 + 1.6 * mood_t.wf;
    let mood_g = mood(t - lag, k);

    // Fixed asymmetric shift, then a Mobius translation on a heartbeat envelope,
    // then spin. All conformal, so the {6,4} tiling stays valid while it breathes.
    let shift_dir = 0.6 + 2.0 * unit(seed, 31, 0);
    let shift = trans_iso((
        k.skew * 0.78 * shift_dir.cos(),
        k.skew * 0.78 * shift_dir.sin(),
    ));
    let g = view(&mood_t, k, &shift, t);
    let gg = view(&mood_g, k, &shift, t - lag);

    // Depth story: warm at the centre, cold at the rim, wound in an off-palette hue.
    let warm = hsl_to_rgb(28.0, 0.90, 0.60);
    let cold = hsl_to_rgb(258.0, 0.70, 0.62);
    let wound_col = hsl_to_rgb(((seed as f64) % 360.0 + 190.0) % 360.0, 0.92, 0.58);
    let wound_hot = hsl_to_rgb(((seed as f64) % 360.0 + 190.0) % 360.0, 1.0, 0.72);

    // Act modulation: fracture pushes cells outward and guts the edges;
    // reassembly pulls them back. Heartbeat keeps the strong breathing.
    let drift = (mood_t.wf - 0.5 * mood_t.wr) * 0.30;
    let twist = mood_t.wf * k.twist;
    let gap = mood_t.wf * 0.35;
    let fill_gain = (0.5 * mood_t.wc + 1.0 * mood_t.wh + 0.35 * mood_t.wf + 0.9 * mood_t.wr)
        .clamp(0.0, 1.0) as f32;

    // A bloom flash between the two heartbeat pulses: every cell core ignites.
    let bloom = (gauss(mood_t.u, 0.50, 0.06) * (0.4 + 0.6 * mood_t.wh)) as f32;

    // An eclipse beat: the cell fills starve while the star field swells.
    let ecl = gauss(mood_t.u, 0.78, 0.05);

    // Reassembly births: cells re-ignite centre-first as the act progresses.
    let prog = ((mood_t.iu - 0.75) / 0.25).clamp(0.0, 1.0) as f32;

    // Contagion front from two opposite origins; they interfere when they meet.
    let wound_on = k.wound > 0.05 && mood_t.u > 0.40 && mood_t.u < 0.66;
    let wphase = ((mood_t.u - 0.40) / 0.26).clamp(0.0, 1.0);
    let max_hops = tiles
        .iter()
        .map(|tl| tl.hops_a.max(tl.hops_b))
        .max()
        .unwrap_or(1) as f64;
    let front = wphase * (max_hops + 2.0);

    let mut dt: Vec<DrawTile> = Vec::with_capacity(tiles.len());
    let mut pos = vec![usize::MAX; tiles.len()];
    const BINS: usize = 128;
    let mut density = vec![0.0f32; BINS];

    measure_layer(NAME, "frame", || {
        for (ti, tile) in tiles.iter().enumerate() {
            let base = compose(&g, &tile.m);
            let cen = apply(&base, (0.0, 0.0));
            let rr = cabs2(cen).sqrt();
            let dep = tile.depth as f64 / k.depth.max(1) as f64;
            let m2 = if drift.abs() > 1e-5 || twist.abs() > 1e-5 {
                let dir = if rr > 1e-9 { (cen.0 / rr, cen.1 / rr) } else { (0.0, 0.0) };
                let mut mag = drift * (0.18 + 0.82 * rr);
                if mag > 0.0 && rr + mag > 1.35 {
                    mag = (1.35 - rr).max(0.0);
                }
                if mag < 0.0 && rr + mag < 0.02 {
                    mag = -(rr - 0.02);
                }
                let moved = compose(&trans_iso((dir.0 * mag, dir.1 * mag)), &base);
                compose(&rot_iso(twist * (0.3 + dep)), &moved)
            } else {
                base
            };
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
            let band = (dep * 0.7 + t as f32 * 0.18).rem_euclid(1.0);
            let depth_t = smoothstep(0.0, 1.0, rr as f32);
            let mut col = lerp_color(warm, cold, depth_t);
            col = lerp_color(col, pal[1 + tile.par as usize], 0.35);
            let fade = (1.0 - dep * 0.4).clamp(0.3, 1.0);
            col = lerp_color(pal[0], col, fade);
            let sx = (cx0 + s * cen.0 as f32).round() as i32;
            let sy = (cy0 + s * 0.5 * cen.1 as f32).round() as i32;
            let ang = ((sy as f32 - cy0) as f64).atan2((sx as f32 - cx0) as f64);
            let bin = (((ang + PI) / TAU) * BINS as f64) as usize % BINS;
            density[bin] += 1.0;
            pos[ti] = dt.len();
            let parent_dt = tile.parent.map(|p| pos[p]).filter(|&p| p != usize::MAX);
            dt.push(DrawTile {
                uv,
                v,
                cx: sx,
                cy: sy,
                edge,
                dep,
                col,
                band,
                m: tile.m,
                parent_dt,
                hops_a: tile.hops_a,
                hops_b: tile.hops_b,
            });
        }
    });

    let dmax = density.iter().copied().fold(1.0f32, f32::max);

    measure_layer(NAME, "ground", || {
        let hz = k.horizon as f32;
        let star = (hz + 0.9 * ecl as f32).min(1.6);
        let thr = 0.9996 - 0.0038 * star as f64;
        let dpx = -mood_t.aa.0 as f32 * s * 0.35;
        let dpy = -mood_t.aa.1 as f32 * s * 0.5 * 0.35;
        for y in 0..h {
            for x in 0..w {
                let zx = (x as f32 + 0.5 - cx0) / s;
                let zy = (y as f32 + 0.5 - cy0) / (0.5 * s);
                let d = (zx * zx + zy * zy).sqrt();
                if d > 1.0 {
                    let hx = (x as f32 - dpx).round() as i64;
                    let hy = (y as f32 - dpy).round() as i64;
                    let hv = splitmix(
                        seed ^ (hx as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
                            ^ (hy as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F),
                    );
                    let rv = ((hv >> 11) as f64) / ((1u64 << 53) as f64);
                    let fade = (1.0 - (d - 1.0) / 1.7).clamp(0.12, 1.0);
                    let bg = lerp_color(pal[0], pal[2], 0.10 * fade);
                    if star > 0.02 && rv > thr {
                        let bright = star * fade;
                        let ch = if rv > 0.9992 {
                            '*'
                        } else if rv > 0.9985 {
                            '+'
                        } else {
                            '.'
                        };
                        let fg = lerp_color(pal[0], pal[4], 0.65 * bright + 0.1);
                        grid[y][x] = Cell::with_bg(ch, fg, bg);
                    } else {
                        grid[y][x] = Cell::with_bg(' ', pal[0], bg);
                    }
                } else {
                    let ang = ((y as f32 + 0.5 - cy0) as f64).atan2((x as f32 + 0.5 - cx0) as f64);
                    let bin = (((ang + PI) / TAU) * BINS as f64) as usize % BINS;
                    let dens = density[bin] / dmax;
                    let base = lerp_color(pal[0], cold, d * d * 0.32);
                    let corona = (smoothstep(0.55, 0.98, d) * dens * hz + bloom * 0.35).clamp(0.0, 1.0);
                    let bg = lerp_color(base, warm, corona * 0.7);
                    let dust = unit(seed, 10, (y * w + x) as u64) > 0.992;
                    let ch = if dust { '.' } else { ' ' };
                    grid[y][x] = Cell::with_bg(ch, lerp_color(pal[0], pal[1], 0.5), bg);
                }
            }
        }
    });

    measure_layer(NAME, "ghost", || {
        if k.ghost <= 0.03 {
            return;
        }
        let col = lerp_color(pal[0], pal[2], 0.22 + 0.45 * k.ghost as f32);
        for tile in &dt {
            let Some(pi) = tile.parent_dt else { continue };
            if tile.edge < EDGE_MIN {
                continue;
            }
            let parent = &dt[pi];
            let a = apply(&compose(&gg, &tile.m), (0.0, 0.0));
            let b = apply(&compose(&gg, &parent.m), (0.0, 0.0));
            pp_line(
                grid,
                (cx0 + s * a.0 as f32).round() as i32,
                (cy0 + s * 0.5 * a.1 as f32).round() as i32,
                (cx0 + s * b.0 as f32).round() as i32,
                (cy0 + s * 0.5 * b.1 as f32).round() as i32,
                col,
            );
        }
    });

    measure_layer(NAME, "edges", || {
        for tile in &dt {
            if tile.edge < ARC_MIN {
                continue;
            }
            let ec = lerp_color(tile.col, pal[4], 0.30);
            for i in 0..6 {
                draw_geodesic(grid, tile.uv[i], tile.uv[(i + 1) % 6], cx0, cy0, s, ec, gap);
            }
        }
    });

    measure_layer(NAME, "fill", || {
        const RAMP: &[u8] = b" .:-=+*#%@";
        let fill = (k.fill * fill_gain as f64 * (1.0 - 0.8 * ecl)) as f32;
        if fill <= 0.05 {
            return;
        }
        for tile in dt.iter() {
            if tile.edge < EDGE_MIN {
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
            let fa = (tile.hops_a as f64 - front).abs() < 0.75;
            let fb = (tile.hops_b as f64 - front).abs() < 0.75;
            let wounded = wound_on && (fa || fb);
            let both = fa && fb;
            let alive = (1.0 - tile.dep * (1.0 - prog)).max(0.0);
            for y in y0..=y1 {
                for x in x0..=x1 {
                    if !point_in_poly(x as f32 + 0.5, y as f32 + 0.5, &tile.v) {
                        continue;
                    }
                    let dx = (x - tile.cx) as f32;
                    let dy = (y - tile.cy) as f32;
                    let shape = (1.0 - (dx * dx + dy * dy).sqrt() / rad).clamp(0.0, 1.0);
                    let core = shape * shape;
                    let (energy, col) = if wounded {
                        let ws = (1.0 - shape) * k.wound as f32;
                        if both {
                            (ws.max(shape * 0.9), wound_hot)
                        } else {
                            (ws, wound_col)
                        }
                    } else {
                        let inten = shape * fill * alive;
                        let bright = (0.55 + 0.45 * (tile.band * std::f32::consts::TAU).sin())
                            * (1.0 + 0.9 * bloom);
                        (
                            core,
                            lerp_color(pal[0], tile.col, (0.30 + 0.70 * inten) * (0.6 + 0.4 * bright)),
                        )
                    };
                    let shown = if wounded { energy } else { shape * fill * alive };
                    if shown < 0.05 {
                        continue;
                    }
                    let gi = ((energy * (RAMP.len() as f32 - 1.0)).round() as usize)
                        .min(RAMP.len() - 1);
                    pp_put(grid, x, y, RAMP[gi] as char, col);
                }
            }
        }
    });
}

impl Mode for Hyperhex6 {
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
        insta::assert_snapshot!("hyperhex_6_80x24", render_at(80, 24, 42, 0.0));
    }

    #[test]
    fn snapshot_t6() {
        insta::assert_snapshot!("hyperhex_6_t6", render_at(80, 24, 42, 6.0));
    }

    #[test]
    fn snapshot_wound() {
        insta::assert_snapshot!("hyperhex_6_wound", render_at(80, 24, 42, 1.0));
    }

    #[test]
    fn snapshot_eclipse() {
        insta::assert_snapshot!("hyperhex_6_eclipse", render_at(80, 24, 42, 1.5));
    }

    #[test]
    fn snapshot_reassembly() {
        insta::assert_snapshot!("hyperhex_6_reassembly", render_at(80, 24, 42, 36.0));
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
            "hyperhex-6 average frame {avg_ms:.3} ms exceeds 6 ms"
        );
    }
}
