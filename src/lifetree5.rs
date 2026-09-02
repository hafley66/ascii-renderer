//! tree-of-life-5 -- the tree in the Klein projective disk: straight-chord geodesic branches,
//! Euclidean spin + a rotating diameter seam that sweeps ethereal from living, horocycle rings, motes.
//! Skeleton cached in Klein coords; a frame does zero rng and zero allocation.
use crate::color::*;
use crate::opts::param_f32;
use crate::types::*;
use crossterm::style::Color;
use rand::RngExt;
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::cell::RefCell;

type C = (f32, f32);

#[inline]
fn polar(r: f32, ang: f32) -> C {
    (r * ang.cos(), r * ang.sin())
}

#[inline]
fn norm2(a: C) -> f32 {
    a.0 * a.0 + a.1 * a.1
}

#[inline]
fn rotate(z: C, cosr: f32, sinr: f32) -> C {
    (z.0 * cosr - z.1 * sinr, z.0 * sinr + z.1 * cosr)
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) struct Knobs5 {
    pub depth: u32,
    pub spread: f32,
    pub len: f32,
    pub spin: f32,
    pub seam: f32,
    pub motes: usize,
    pub glow: f32,
    pub rings: f32,
    pub wind: f32,
    pub speed: f32,
}

impl Knobs5 {
    pub(crate) fn from_env() -> Self {
        Knobs5 {
            depth: param_f32("DEPTH", 7.0).round().clamp(4.0, 10.0) as u32,
            spread: param_f32("SPREAD", 0.6).clamp(0.2, 1.3),
            len: param_f32("LEN", 0.8).clamp(0.3, 2.0),
            spin: param_f32("SPIN", 0.1).clamp(-1.0, 1.0),
            seam: param_f32("SEAM", 0.05).clamp(-0.5, 0.5),
            motes: param_f32("MOTES", 50.0).round().clamp(0.0, 300.0) as usize,
            glow: param_f32("GLOW", 0.8).clamp(0.0, 1.0),
            rings: param_f32("RINGS", 0.7).clamp(0.0, 1.0),
            wind: param_f32("WIND", 0.6).clamp(0.0, 1.0),
            speed: param_f32("SPEED", 1.0).clamp(0.05, 4.0),
        }
    }
    fn geometry_key(&self) -> (u32, u32, u32, usize) {
        (self.depth, self.spread.to_bits(), self.len.to_bits(), self.motes)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Trunk,
    Branch,
    Twig,
    Root,
}

#[derive(Clone, Copy)]
struct BSeg {
    p0: C,
    p1: C,
    kind: Kind,
    ord: f32,
    phase: f32,
}

#[derive(Clone, Copy)]
struct BLeaf {
    z: C,
    phase: f32,
    tex: u8,
}

#[derive(Clone, Copy)]
struct Mote {
    ang: f32,
    phase: f32,
    rate: f32,
    tint: f32,
}

#[derive(Clone, Copy)]
struct Ring {
    c: C,
    rad: f32,
    phase: f32,
}

struct Cached {
    key: (usize, usize, u64, (u32, u32, u32, usize)),
    segs: Vec<BSeg>,
    leaves: Vec<BLeaf>,
    rings: Vec<Ring>,
    motes: Vec<Mote>,
    cx: f32,
    cy: f32,
    rx: f32,
    ry: f32,
    eth_rgb: (u8, u8, u8),
    eth_hi: (u8, u8, u8),
    eth_dark: (u8, u8, u8),
    live_dark: (u8, u8, u8),
    bark: (u8, u8, u8),
    bark_hi: (u8, u8, u8),
    leaf_rgb: (u8, u8, u8),
    leaf_hi: (u8, u8, u8),
}

thread_local! {
    static CACHE: RefCell<Option<Cached>> = const { RefCell::new(None) };
}

fn rgb3(c: Color) -> (u8, u8, u8) {
    match c {
        Color::Rgb { r, g, b } => (r, g, b),
        _ => (128, 128, 128),
    }
}

fn hue_of(c: Color) -> f64 {
    let (r, g, b) = rgb3(c);
    let (r, g, b) = (r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    if d < 1e-6 {
        return 0.0;
    }
    let h = if max == r {
        60.0 * (((g - b) / d) % 6.0)
    } else if max == g {
        60.0 * ((b - r) / d + 2.0)
    } else {
        60.0 * ((r - g) / d + 4.0)
    };
    h.rem_euclid(360.0)
}

#[inline]
fn scale(c: (u8, u8, u8), k: f32) -> Color {
    let f = |v: u8| ((v as f32 * k).round().clamp(0.0, 255.0)) as u8;
    Color::Rgb { r: f(c.0), g: f(c.1), b: f(c.2) }
}

#[inline]
fn mix(a: (u8, u8, u8), b: (u8, u8, u8), t: f32) -> (u8, u8, u8) {
    let t = t.clamp(0.0, 1.0);
    let f = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    (f(a.0, b.0), f(a.1, b.1), f(a.2, b.2))
}

#[inline]
fn put(grid: &mut Grid, x: i32, y: i32, ch: char, fg: Color) {
    if x >= 0 && y >= 0 && (y as usize) < grid.len() && (x as usize) < grid[0].len() {
        let c = &mut grid[y as usize][x as usize];
        c.ch = ch;
        c.fg = fg;
    }
}

fn grow(
    rng: &mut StdRng,
    segs: &mut Vec<BSeg>,
    leaves: &mut Vec<BLeaf>,
    pos: C,
    ang: f32,
    len: f32,
    lvl: u32,
    max_lvl: u32,
    spread: f32,
    ord0: f32,
    is_root: bool,
) {
    if lvl > max_lvl || len < 0.05 || norm2(pos) > 0.93 {
        return;
    }
    let end = (pos.0 + len * ang.cos(), pos.1 + len * ang.sin());
    let kind = if is_root {
        Kind::Root
    } else if lvl <= 1 {
        Kind::Trunk
    } else if lvl + 2 >= max_lvl {
        Kind::Twig
    } else {
        Kind::Branch
    };
    segs.push(BSeg { p0: pos, p1: end, kind, ord: ord0, phase: rng.random::<f32>() });
    if lvl == max_lvl && !is_root {
        for _ in 0..4 {
            let a2 = rng.random::<f32>() * 6.2832;
            let r = 0.04 + rng.random::<f32>() * 0.18;
            leaves.push(BLeaf { z: (end.0 + r * a2.cos(), end.1 + r * a2.sin()), phase: rng.random::<f32>(), tex: rng.random_range(0..3u32) as u8 });
        }
        return;
    }
    let n = if lvl == 0 { 2 } else if lvl <= 3 && rng.random::<f32>() < 0.28 { 3 } else { 2 };
    let keep = if lvl >= 5 { 0.7 } else if lvl >= 3 { 0.85 } else { 1.0 };
    for i in 0..n {
        if rng.random::<f32>() > keep {
            continue;
        }
        let side = if n == 2 { if i == 0 { -1.0 } else { 1.0 } } else { i as f32 - 1.0 };
        let sp = spread * (0.6 + rng.random::<f32>() * 0.8);
        let jitter = (rng.random::<f32>() - 0.5) * 0.3;
        let na = ang + side * sp + jitter;
        let nl = len * (0.82 + rng.random::<f32>() * 0.12);
        grow(rng, segs, leaves, end, na, nl, lvl + 1, max_lvl, spread, ord0 + len, is_root);
    }
}

fn build(w: usize, h: usize, seed: u64, palette: &[Color; 5], k: &Knobs5) -> Cached {
    let mut rng = StdRng::seed_from_u64(seed ^ 0x5EED_CAFE_7A11_3E5);
    let rx = ((w as f32) * 0.5 - 1.0).min(((h as f32) * 0.5 - 0.5) * 2.1).max(4.0);
    let ry = rx / 2.1;
    let cx = (w as f32) * 0.5;
    let cy = (h as f32) * 0.5;

    let eth_hue = (hue_of(palette[3]) + 150.0).rem_euclid(360.0);
    let eth_rgb = rgb3(hsl_to_rgb(eth_hue, 0.55, 0.66));
    let eth_hi = rgb3(hsl_to_rgb(eth_hue, 0.35, 0.92));
    let eth_dark = rgb3(hsl_to_rgb(eth_hue, 0.5, 0.09));
    let live_dark = mix(rgb3(palette[0]), rgb3(palette[1]), 0.12);
    let bark = rgb3(darken(palette[2], 20));
    let bark_hi = rgb3(lighten(palette[2], 25));
    let leaf_rgb = rgb3(palette[1]);
    let leaf_hi = rgb3(palette[3]);

    let mut segs = Vec::new();
    let mut leaves = Vec::new();
    let up = std::f32::consts::FRAC_PI_2;
    grow(&mut rng, &mut segs, &mut leaves, (0.0, -0.2), up, k.len, 0, k.depth, k.spread, 0.0, false);
    let root_lvl = k.depth.saturating_sub(3).max(2);
    grow(&mut rng, &mut segs, &mut leaves, (0.0, -0.2), -up, k.len * 0.7, 0, root_lvl, k.spread * 1.3, 0.0, true);
    let ord_max = segs.iter().fold(0.1_f32, |m, s| m.max(s.ord));
    for s in segs.iter_mut() {
        s.ord /= ord_max;
    }

    let mut rings = Vec::new();
    for i in 0..4 {
        let phase = rng.random::<f32>();
        rings.push(Ring { c: (0.0, 0.0), rad: 0.22 + i as f32 * 0.17, phase });
    }

    let mut motes = Vec::with_capacity(k.motes);
    for _ in 0..k.motes {
        motes.push(Mote {
            ang: rng.random::<f32>() * 6.2832,
            phase: rng.random::<f32>(),
            rate: 0.12 + rng.random::<f32>() * 0.25,
            tint: rng.random::<f32>(),
        });
    }

    Cached {
        key: (w, h, seed, k.geometry_key()),
        segs,
        leaves,
        rings,
        motes,
        cx,
        cy,
        rx,
        ry,
        eth_rgb,
        eth_hi,
        eth_dark,
        live_dark,
        bark,
        bark_hi,
        leaf_rgb,
        leaf_hi,
    }
}

const ETH_FILL: [char; 3] = ['░', '▒', '▓'];
const ETH_LEAF: [char; 4] = ['○', '°', '◦', '◉'];
const LIVE_LEAF: [[char; 2]; 3] = [['♠', '♣'], ['♣', '*'], ['♠', '*']];
const MOTE: [char; 4] = ['·', '∙', '°', '○'];

#[inline]
fn slope_glyph(dx: i32, dy: i32, heavy: bool) -> char {
    let adx = dx.abs();
    let ady = dy.abs() * 2;
    if adx * 3 < ady {
        if heavy { '║' } else { '│' }
    } else if ady * 3 < adx {
        if heavy { '═' } else { '─' }
    } else if (dx > 0) == (dy < 0) {
        if heavy { '╱' } else { '/' }
    } else if heavy {
        '╲'
    } else {
        '\\'
    }
}

#[inline]
fn to_screen(c: &Cached, z: C) -> (i32, i32) {
    ((c.cx + z.0 * c.rx).round() as i32, (c.cy - z.1 * c.ry).round() as i32)
}

#[inline]
fn line(x0: i32, y0: i32, x1: i32, y1: i32, mut f: impl FnMut(i32, i32)) {
    let n = (x1 - x0).abs().max((y1 - y0).abs()).max(1);
    for i in 0..=n {
        let t = i as f32 / n as f32;
        f((x0 as f32 + (x1 - x0) as f32 * t).round() as i32, (y0 as f32 + (y1 - y0) as f32 * t).round() as i32);
    }
}

pub(crate) fn draw_lifetree5(grid: &mut Grid, w: usize, h: usize, seed: u64, palette: &[Color; 5], t: f32, k: &Knobs5) {
    if w == 0 || h == 0 {
        return;
    }
    CACHE.with(|slot| {
        let mut slot = slot.borrow_mut();
        let key = (w, h, seed, k.geometry_key());
        if slot.as_ref().map(|c| c.key != key).unwrap_or(true) {
            *slot = Some(build(w, h, seed, palette, k));
        }
        let c = slot.as_ref().unwrap();
        frame(grid, c, t, k);
    });
}

#[inline]
fn frame(grid: &mut Grid, c: &Cached, t: f32, k: &Knobs5) {
    let w = grid[0].len();
    let h = grid.len();
    let ts = t * k.speed;
    let (cosr, sinr) = (ts * k.spin).sin_cos();
    let (cosa, sina) = (ts * k.seam).sin_cos();
    let side_of = |z: C| -> bool {
        let zr = rotate(z, cosa, -sina);
        zr.0 < 0.0
    };
    let beat = (ts * 0.6).rem_euclid(3.0);
    let flash = (1.0 - beat * 4.0).max(0.0) * k.glow;

    for y in 0..h {
        for x in 0..w {
            let z = ((x as f32 + 0.5 - c.cx) / c.rx, (c.cy - y as f32) / c.ry);
            let r2 = norm2(z);
            if r2 >= 1.0 {
                grid[y][x] = Cell::blank();
                continue;
            }
            let eth = side_of(z);
            let fade = 1.0 - r2 * r2;
            let base = if eth { c.eth_dark } else { c.live_dark };
            let col = scale(base, 0.35 + 0.65 * fade + if eth { flash * 0.6 } else { 0.0 });
            grid[y][x] = Cell::with_bg(' ', col, col);
        }
    }

    let rim_n = ((c.rx + c.ry) * 4.0) as usize;
    for i in 0..rim_n {
        let ang = i as f32 / rim_n as f32 * 6.2832;
        let x = (c.cx + ang.cos() * (c.rx - 0.3)).round() as i32;
        let y = (c.cy - ang.sin() * (c.ry - 0.3)).round() as i32;
        let p = (ang * 6.0 - ts * 1.4).sin();
        let ch = if p > 0.7 { '∙' } else { '·' };
        put(grid, x, y, ch, scale(c.eth_hi, 0.3 + 0.3 * p.max(0.0)));
    }

    if k.rings > 0.0 {
        for r in &c.rings {
            let pulse = 0.5 + 0.5 * (ts * 1.2 + r.phase * 6.28).sin();
            let rad = r.rad * (0.9 + 0.15 * pulse);
            let n = 20;
            let mut prev: Option<(i32, i32)> = None;
            for i in 0..=n {
                let a = i as f32 / n as f32 * 6.2832;
                let zo = (r.c.0 + rad * a.cos(), r.c.1 + rad * a.sin());
                if norm2(zo) >= 0.96 {
                    prev = None;
                    continue;
                }
                let z = rotate(zo, cosr, sinr);
                let s = to_screen(c, z);
                if side_of(z) {
                    let col = scale(mix(c.eth_rgb, c.eth_hi, pulse * 0.7), 0.3 * k.rings * (0.5 + 0.5 * pulse));
                    if let Some(p) = prev {
                        line(p.0, p.1, s.0, s.1, |x, y| {
                            if x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h && grid[y as usize][x as usize].ch == ' ' {
                                grid[y as usize][x as usize].ch = '·';
                                grid[y as usize][x as usize].fg = col;
                            }
                        });
                    }
                }
                prev = Some(s);
            }
        }
    }

    for s in &c.segs {
        let mid = ((s.p0.0 + s.p1.0) * 0.5, (s.p0.1 + s.p1.1) * 0.5);
        let eth = side_of(mid);
        let z0 = rotate(s.p0, cosr, sinr);
        let z1 = rotate(s.p1, cosr, sinr);
        let local = 1.0 - norm2(mid);
        let heavy = matches!(s.kind, Kind::Trunk);
        let thick = if heavy && local > 0.5 { 1 } else { 0 };
        let p0 = to_screen(c, z0);
        let p1 = to_screen(c, z1);
        let dx = p1.0 - p0.0;
        let dy = p1.1 - p0.1;
        if eth {
            let p = (s.ord * 9.0 - ts * 1.6 + s.phase).sin().max(0.0);
            let pulse = p * p;
            let idx = if pulse > 0.55 { 2 } else if pulse > 0.15 { 1 } else { 0 };
            let ch = match s.kind {
                Kind::Twig => if idx > 0 { '∙' } else { '·' },
                _ => ETH_FILL[idx],
            };
            let b = (0.4 + 0.6 * k.glow * pulse.max(flash)) * (0.45 + 0.55 * local);
            let col = scale(mix(c.eth_rgb, c.eth_hi, pulse * 0.7), b);
            line(p0.0, p0.1, p1.0, p1.1, |x, y| {
                for k2 in -thick..=thick {
                    put(grid, x + k2, y, ch, col);
                }
            });
        } else {
            let ch = match s.kind {
                Kind::Twig => '·',
                _ => slope_glyph(dx, dy, heavy),
            };
            let base = match s.kind {
                Kind::Root => mix(c.bark, (0, 0, 0), 0.35),
                Kind::Twig => mix(c.bark_hi, c.leaf_rgb, 0.4),
                _ => mix(c.bark, c.bark_hi, s.ord * 0.6),
            };
            let breath = 0.9 + 0.1 * (ts * 0.5 + s.ord * 3.0).sin();
            let col = scale(base, (0.5 + 0.5 * local) * breath);
            line(p0.0, p0.1, p1.0, p1.1, |x, y| {
                for k2 in -thick..=thick {
                    put(grid, x + k2, y, ch, col);
                }
            });
        }
    }

    for l in &c.leaves {
        let z = rotate(l.z, cosr, sinr);
        let local = 1.0 - norm2(l.z);
        if local < 0.03 {
            continue;
        }
        let (mut x, mut y) = to_screen(c, z);
        if side_of(l.z) {
            let p = (ts * 1.3 + l.phase * 6.28).sin();
            let ch = if p > 0.5 { '○' } else { '°' };
            put(grid, x, y, ch, scale(c.eth_hi, (0.45 + 0.4 * p.max(0.0)) * (0.5 + 0.5 * local)));
        } else {
            let sway = k.wind * (ts * 1.8 + l.phase * 6.2832).sin() * 1.2;
            x = (x as f32 + sway).round() as i32;
            let r = (ts * 2.7 + l.phase * 6.2832).sin();
            let pair = LIVE_LEAF[(l.tex & 3) as usize % 3];
            let ch = if r > 0.6 { pair[1] } else { pair[0] };
            let col = scale(mix(c.leaf_rgb, c.leaf_hi, l.phase * 0.5 + r.max(0.0) * 0.3), (0.7 + 0.3 * local) * (0.85 + 0.15 * r));
            put(grid, x, y, ch, col);
        }
    }

    for m in &c.motes {
        let life = (ts * m.rate * 0.4 + m.phase).fract();
        let d = life * 3.0;
        let zo = polar(d, m.ang + 0.4 * (life * 6.28 + m.phase).sin());
        if !side_of(zo) {
            continue;
        }
        let z = rotate(zo, cosr, sinr);
        let local = 1.0 - norm2(z);
        if local < 0.02 {
            continue;
        }
        let (x, y) = to_screen(c, z);
        let stage = ((life * 4.0) as usize).min(3);
        let b = (life * 3.1416).sin() * (0.5 + 0.5 * local);
        put(grid, x, y, MOTE[[0, 1, 2, 1][stage]], scale(mix(c.eth_rgb, c.eth_hi, m.tint), 0.35 + 0.65 * b));
    }

    for i in 0..48 {
        let s = -1.05 + 2.1 * i as f32 / 47.0;
        let zo = rotate((0.0, s), cosa, sina);
        if norm2(zo) >= 0.985 {
            continue;
        }
        let z = rotate(zo, cosr, sinr);
        let (x, y) = to_screen(c, z);
        if x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h && grid[y as usize][x as usize].ch == ' ' {
            let p = (s * 4.0 - ts * 2.5).sin();
            if p > 0.1 {
                put(grid, x, y, '┆', scale(c.eth_hi, 0.35 + 0.5 * k.glow * p));
            }
        }
    }
}

pub(crate) fn cli_lifetree5(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
    let _ = (rng, term_w, term_h, mode, theme_name);
    let mut k = Knobs5::from_env();
    let f = |i: usize| args.get(i).and_then(|s| s.parse::<f32>().ok());
    if let Some(v) = f(4) {
        k.depth = v.round().clamp(4.0, 10.0) as u32;
    }
    if let Some(v) = f(5) {
        k.spread = v.clamp(0.2, 1.3);
    }
    if let Some(v) = f(6) {
        k.len = v.clamp(0.3, 2.0);
    }
    if let Some(v) = f(7) {
        k.spin = v.clamp(-1.0, 1.0);
    }
    if let Some(v) = f(8) {
        k.seam = v.clamp(-0.5, 0.5);
    }
    if let Some(v) = f(9) {
        k.motes = v.round().clamp(0.0, 300.0) as usize;
    }
    draw_lifetree5(&mut grid, width, height, seed, &palette, t_anim, &k);
    (grid, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(w: usize, h: usize, seed: u64, t: f32) -> String {
        let mut g = vec![vec![Cell::blank(); w]; h];
        let p = crate::color::named_theme("moss").unwrap();
        let k = Knobs5::from_env();
        draw_lifetree5(&mut g, w, h, seed, &p, t, &k);
        g.iter()
            .map(|row| row.iter().map(|c| c.ch).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn snapshot_lifetree5_small() {
        insta::assert_snapshot!("lifetree5_80x24", run(80, 24, 42, 0.0));
    }

    #[test]
    fn snapshot_lifetree5_wide() {
        insta::assert_snapshot!("lifetree5_120x40", run(120, 40, 42, 0.0));
    }

    #[test]
    fn deterministic_and_seed_sensitive() {
        assert_eq!(run(90, 30, 42, 0.0), run(90, 30, 42, 0.0));
        assert_ne!(run(90, 30, 42, 0.0), run(90, 30, 7, 0.0));
    }

    #[test]
    fn rotation_moves_tree() {
        assert_ne!(run(90, 30, 42, 0.0), run(90, 30, 42, 3.0));
    }

    #[test]
    fn corners_outside_disk_stay_blank() {
        let s = run(100, 32, 42, 0.0);
        let first = s.lines().next().unwrap();
        assert_eq!(first.chars().next().unwrap(), ' ');
        assert_eq!(first.chars().last().unwrap(), ' ');
    }

    #[test]
    fn both_sides_present() {
        let s = run(100, 32, 42, 0.0);
        assert!(s.contains('░') || s.contains('▒') || s.contains('▓'), "ethereal fill");
        assert!(s.contains('║') || s.contains('│') || s.contains('╱') || s.contains('╲'), "living bark");
    }

    #[test]
    fn rotate_roundtrip() {
        let (cosr, sinr) = 0.7f32.sin_cos();
        let z = (0.5, 0.4);
        let r = rotate(z, cosr, sinr);
        let back = rotate(r, cosr, -sinr);
        assert!((back.0 - z.0).abs() < 1e-4 && (back.1 - z.1).abs() < 1e-4);
    }

    #[test]
    fn frame_cost_is_flat() {
        let mut g = vec![vec![Cell::blank(); 200]; 60];
        let p = crate::color::named_theme("ember").unwrap();
        let k = Knobs5::from_env();
        draw_lifetree5(&mut g, 200, 60, 42, &p, 0.0, &k);
        let start = std::time::Instant::now();
        for i in 1..=200 {
            draw_lifetree5(&mut g, 200, 60, 42, &p, i as f32 * 0.06, &k);
        }
        let per = start.elapsed().as_secs_f64() / 200.0;
        eprintln!("lifetree5 frame 200x60: {:.3}ms", per * 1000.0);
        assert!(per < 0.006, "frame {:.3}ms exceeds 6ms budget at 200x60", per * 1000.0);
    }
}
