//! tree-of-life -- one trunk split at a seam: flat ethereal silhouette left, living
//! bark/wind/leaf-fall right. Skeleton + bg cached per (w,h,seed,geometry); frame = memcpy + linear cell pass.
use crate::_0_profile::measure_layer;
use crate::color::*;
use crate::opts::param_f32;
use crate::types::*;
use crossterm::style::Color;
use rand::RngExt;
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::cell::RefCell;

#[derive(Clone, Copy, PartialEq)]
pub(crate) struct LifeKnobs {
    pub depth: u32,
    pub spread: f32,
    pub sway: f32,
    pub speed: f32,
    pub motes: usize,
    pub glow: f32,
    pub seam: f32,
    pub roots: f32,
}

impl LifeKnobs {
    pub(crate) fn from_env() -> Self {
        LifeKnobs {
            depth: param_f32("DEPTH", 8.0).round().clamp(4.0, 11.0) as u32,
            spread: param_f32("SPREAD", 0.55).clamp(0.15, 1.2),
            sway: param_f32("SWAY", 2.0).clamp(0.0, 6.0),
            speed: param_f32("SPEED", 1.0).clamp(0.05, 4.0),
            motes: param_f32("MOTES", 40.0).round().clamp(0.0, 300.0) as usize,
            glow: param_f32("GLOW", 0.8).clamp(0.0, 1.0),
            seam: param_f32("SEAM", 0.5).clamp(0.1, 0.9),
            roots: param_f32("ROOTS", 0.28).clamp(0.05, 0.5),
        }
    }

    fn geometry_key(&self) -> (u32, u32, u32, u32, usize) {
        (
            self.depth,
            self.spread.to_bits(),
            self.seam.to_bits(),
            self.roots.to_bits(),
            self.motes,
        )
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Trunk,
    Branch,
    Twig,
    Leaf,
    LeafEdge,
    Root,
    RootTip,
}

/// One rasterized tree cell. `ord` is distance from the base along the tree,
/// normalized 0..1 (roots run 0..1 downward). `tex` is a slope code for bark.
#[derive(Clone, Copy)]
struct TreeCell {
    x: i32,
    y: i32,
    kind: Kind,
    ord: f32,
    phase: f32,
    tex: u8,
    rgb: (u8, u8, u8),
    alt: (u8, u8, u8),
}

#[derive(Clone, Copy)]
struct Star {
    x: usize,
    y: usize,
    phase: f32,
    rate: f32,
}

#[derive(Clone, Copy)]
struct Mote {
    x0: f32,
    phase: f32,
    rate: f32,
    amp: f32,
    freq: f32,
    tint: f32,
}

struct Seg {
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    lvl: u32,
    thick: f32,
    ord0: f32,
    ord1: f32,
}

struct Cached {
    key: (usize, usize, u64, (u32, u32, u32, u32, usize)),
    bg: Grid,
    cells: Vec<TreeCell>,
    stars: Vec<Star>,
    motes: Vec<Mote>,
    leaves: Vec<Mote>,
    ground: usize,
    divide: i32,
    cx: i32,
    eth_rgb: (u8, u8, u8),
    eth_hi: (u8, u8, u8),
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

/// Recursive branching. `ang` in radians, +y is down; canopy grows with
/// ang near pi/2, roots with ang near -pi/2. Aspect handled at rasterize time.
fn grow(
    rng: &mut StdRng,
    segs: &mut Vec<Seg>,
    x: f32,
    y: f32,
    ang: f32,
    len: f32,
    lvl: u32,
    max_lvl: u32,
    thick: f32,
    ord0: f32,
    spread: f32,
) {
    if lvl > max_lvl || len < 0.6 {
        return;
    }
    let x1 = x + ang.cos() * len;
    let y1 = y - ang.sin() * len;
    segs.push(Seg { x0: x, y0: y, x1, y1, lvl, thick, ord0, ord1: ord0 + len });
    let n = if lvl == 0 {
        2
    } else if lvl <= 3 && rng.random::<f32>() < 0.3 {
        3
    } else {
        2
    };
    let keep = if lvl >= 5 { 0.68 } else if lvl >= 3 { 0.82 } else { 1.0 };
    for i in 0..n {
        if rng.random::<f32>() > keep {
            continue;
        }
        let side = if n == 2 { if i == 0 { -1.0 } else { 1.0 } } else { i as f32 - 1.0 };
        let sp = spread * (0.55 + rng.random::<f32>() * 0.9);
        let jitter = (rng.random::<f32>() - 0.5) * 0.35;
        let na = ang + side * sp + jitter;
        let nl = len * (0.66 + rng.random::<f32>() * 0.14);
        grow(rng, segs, x1, y1, na, nl, lvl + 1, max_lvl, thick * 0.62, ord0 + len, spread);
    }
}

/// Fit a segment cloud built at origin (0,0) into a box anchored at (cx, y_base).
fn fit(segs: &mut [Seg], cx: f32, y_base: f32, half_w: f32, span_h: f32) {
    let mut max_dx = 1.0_f32;
    let mut max_dy = 1.0_f32;
    for s in segs.iter() {
        max_dx = max_dx.max(s.x0.abs()).max(s.x1.abs());
        max_dy = max_dy.max(s.y0.abs()).max(s.y1.abs());
    }
    let kx = half_w / max_dx;
    let ky = span_h / max_dy;
    for s in segs.iter_mut() {
        s.x0 = cx + s.x0 * kx;
        s.x1 = cx + s.x1 * kx;
        s.y0 = y_base + s.y0 * ky;
        s.y1 = y_base + s.y1 * ky;
    }
}

fn slope_code(dx: f32, dy: f32) -> u8 {
    if dx.abs() < 0.25 * dy.abs() {
        0
    } else if dy.abs() < 0.35 * dx.abs() {
        1
    } else if (dx > 0.0) == (dy < 0.0) {
        2
    } else {
        3
    }
}

/// Rasterize segments into a dense (x, y) -> cell map, last write wins.
fn raster(
    segs: &[Seg],
    max_lvl: u32,
    is_root: bool,
    ord_max: f32,
    w: usize,
    h: usize,
    rng: &mut StdRng,
    map: &mut Vec<Option<TreeCell>>,
) {
    for s in segs {
        let dx = s.x1 - s.x0;
        let dy = s.y1 - s.y0;
        let steps = (dx.abs().max(dy.abs()).ceil() as usize).max(1);
        if s.lvl > 2 && dx.abs() < 1.0 && dy.abs() < 1.0 {
            continue;
        }
        let tex = slope_code(dx, dy);
        let kind = if is_root {
            if s.lvl + 1 >= max_lvl { Kind::RootTip } else { Kind::Root }
        } else if s.lvl <= 1 {
            Kind::Trunk
        } else if s.lvl + 2 >= max_lvl {
            Kind::Twig
        } else {
            Kind::Branch
        };
        let half = (s.thick * 0.5).floor() as i32;
        for i in 0..=steps {
            let f = i as f32 / steps as f32;
            let px = (s.x0 + dx * f).round() as i32;
            let py = (s.y0 + dy * f).round() as i32;
            let ord = (s.ord0 + (s.ord1 - s.ord0) * f) / ord_max;
            for k in -half..=half {
                let x = px + k;
                if x < 0 || py < 0 || x >= w as i32 || py >= h as i32 {
                    continue;
                }
                let tex_k = if k == -half && half > 0 { 4 } else if k == half && half > 0 { 5 } else { tex };
                map[py as usize * w + x as usize] = Some(TreeCell {
                    x,
                    y: py,
                    kind,
                    ord,
                    phase: rng.random::<f32>(),
                    tex: tex_k,
                    rgb: (0, 0, 0),
                    alt: (0, 0, 0),
                });
            }
        }
    }
}

fn blob(map: &mut Vec<Option<TreeCell>>, w: usize, h: usize, cx: i32, cy: i32, r: i32, ord: f32, rng: &mut StdRng) {
    for dy in -r..=r {
        for dx in -(r * 2)..=(r * 2) {
            let d = (dx as f32 * 0.5).powi(2) + (dy as f32).powi(2);
            if d > (r as f32 + 0.3).powi(2) {
                continue;
            }
            let x = cx + dx;
            let y = cy + dy;
            if x < 0 || y < 0 || x >= w as i32 || y >= h as i32 {
                continue;
            }
            let i = y as usize * w + x as usize;
            if map[i].is_some() || rng.random::<f32>() > 0.72 {
                continue;
            }
            map[i] = Some(TreeCell {
                x,
                y,
                kind: Kind::Leaf,
                ord,
                phase: rng.random::<f32>(),
                tex: rng.random_range(0..4u32) as u8,
                rgb: (0, 0, 0),
                alt: (0, 0, 0),
            });
        }
    }
}

fn build(w: usize, h: usize, seed: u64, palette: &[Color; 5], k: &LifeKnobs) -> Cached {
    let mut rng = StdRng::seed_from_u64(seed ^ 0x1F3D_5B79_9E37_79B9);
    let cx = (w / 2) as i32;
    let divide = ((w as f32) * k.seam).round() as i32;
    let ground = ((h as f32) * (1.0 - k.roots)).round().clamp(4.0, h as f32 - 3.0) as usize;

    // colors
    let eth_hue = (hue_of(palette[3]) + 150.0).rem_euclid(360.0);
    let eth_rgb = rgb3(hsl_to_rgb(eth_hue, 0.55, 0.66));
    let eth_hi = rgb3(hsl_to_rgb(eth_hue, 0.35, 0.92));
    let eth_dark = rgb3(hsl_to_rgb(eth_hue, 0.45, 0.06));
    let leaf_rgb = rgb3(palette[1]);
    let leaf_hi = rgb3(palette[3]);
    let bark = rgb3(darken(palette[2], 25));
    let bark_hi = rgb3(lighten(palette[2], 20));
    let dirt = rgb3(darken(palette[2], 55));
    let sky = rgb3(palette[0]);

    // canopy
    let mut canopy: Vec<Seg> = Vec::new();
    let base_thick = (w as f32 / 30.0).clamp(1.0, 7.0);
    grow(&mut rng, &mut canopy, 0.0, 0.0, std::f32::consts::FRAC_PI_2, 10.0, 0, k.depth, base_thick, 0.0, k.spread);
    let half_w = (w as f32 * 0.47).max(4.0);
    fit(&mut canopy, cx as f32, ground as f32, half_w, (ground as f32 - 1.5).max(2.0));
    let ord_max = canopy.iter().fold(1.0_f32, |m, s| m.max(s.ord1));

    // roots
    let mut roots: Vec<Seg> = Vec::new();
    let root_lvl = (k.depth.saturating_sub(3)).max(2);
    grow(&mut rng, &mut roots, 0.0, 0.0, -std::f32::consts::FRAC_PI_2, 8.0, 0, root_lvl, base_thick * 0.55, 0.0, k.spread * 1.2);
    let root_span = (h as f32 - ground as f32 - 1.0).max(1.0);
    fit(&mut roots, cx as f32, ground as f32, half_w * 0.45, root_span);
    let root_ord_max = roots.iter().fold(1.0_f32, |m, s| m.max(s.ord1));

    let mut map: Vec<Option<TreeCell>> = vec![None; w * h];
    raster(&roots, root_lvl, true, root_ord_max, w, h, &mut rng, &mut map);
    raster(&canopy, k.depth, false, ord_max, w, h, &mut rng, &mut map);
    // leaf blobs at outer tips
    for s in canopy.iter().filter(|s| s.lvl + 1 >= k.depth) {
        if rng.random::<f32>() < 0.25 {
            continue;
        }
        let r = if rng.random::<f32>() < 0.15 { 2 } else { 1 };
        blob(&mut map, w, h, s.x1.round() as i32, s.y1.round() as i32, r, (s.ord1 / ord_max).min(1.0), &mut rng);
    }
    // leaf edge detection: a leaf with any missing 4-neighbour leaf/branch is an edge
    let mut edge_idx: Vec<usize> = Vec::new();
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            if let Some(c) = &map[i] {
                if c.kind != Kind::Leaf {
                    continue;
                }
                let n = [(0i32, -1i32), (0, 1), (-1, 0), (1, 0)];
                let mut open = false;
                for (dx, dy) in n {
                    let nx = x as i32 + dx;
                    let ny = y as i32 + dy;
                    if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                        open = true;
                        break;
                    }
                    if map[ny as usize * w + nx as usize].is_none() {
                        open = true;
                        break;
                    }
                }
                if open {
                    edge_idx.push(i);
                }
            }
        }
    }
    for i in edge_idx {
        if let Some(c) = &mut map[i] {
            c.kind = Kind::LeafEdge;
        }
    }

    // bake colors per cell. side is fixed by static x.
    let mut cells: Vec<TreeCell> = Vec::with_capacity(map.len() / 8);
    for c in map.into_iter().flatten() {
        let mut c = c;
        let eth = c.x < divide;
        if eth {
            let tip = c.ord.clamp(0.0, 1.0);
            c.rgb = mix(eth_rgb, eth_hi, tip * 0.5);
            c.alt = eth_hi;
        } else {
            match c.kind {
                Kind::Trunk | Kind::Branch => {
                    let light = if c.tex == 5 { 0.55 } else if c.tex == 4 { 0.0 } else { 0.25 };
                    c.rgb = mix(bark, bark_hi, light + c.ord * 0.2);
                    c.alt = bark_hi;
                }
                Kind::Twig => {
                    c.rgb = mix(bark_hi, leaf_rgb, 0.35);
                    c.alt = leaf_rgb;
                }
                Kind::Leaf | Kind::LeafEdge => {
                    let shade = c.phase * 0.5 + if c.kind == Kind::LeafEdge { 0.3 } else { 0.0 };
                    c.rgb = mix(leaf_rgb, leaf_hi, shade);
                    c.alt = mix(leaf_hi, (255, 255, 240), 0.3);
                }
                Kind::Root | Kind::RootTip => {
                    c.rgb = mix(bark, dirt, 0.3 + c.ord * 0.5);
                    c.alt = bark;
                }
            }
        }
        cells.push(c);
    }
    // draw order: roots, trunk, branch, twig, leaf, leaf edge
    cells.sort_by_key(|c| match c.kind {
        Kind::Root => 0,
        Kind::RootTip => 1,
        Kind::Trunk => 2,
        Kind::Branch => 3,
        Kind::Twig => 4,
        Kind::Leaf => 5,
        Kind::LeafEdge => 6,
    });

    // background
    let mut bg = vec![vec![Cell::blank(); w]; h];
    let dirt_chars = ['·', '∙', ',', '·', '"', '~', '·'];
    for y in 0..h {
        let fy = y as f32 / h as f32;
        for x in 0..w {
            let eth = (x as i32) < divide;
            if eth {
                let dist = (y as f32 - ground as f32).abs() / h as f32;
                let l = (0.05 + (1.0 - dist) * 0.05) as f32;
                let c = mix(eth_dark, eth_rgb, l * 0.3);
                bg[y][x] = Cell::with_bg(' ', scale(c, 1.0), Color::Reset);
            } else if y < ground {
                let c = mix(rgb3(darken(palette[0], 70)), sky, fy * 1.3);
                bg[y][x] = Cell::with_bg(' ', scale(c, 1.0), Color::Reset);
            } else if y == ground {
                bg[y][x] = Cell::new('─', scale(mix(dirt, bark_hi, 0.25), 1.0));
            } else {
                let depth = (y - ground) as f32 / (h - ground).max(1) as f32;
                let ch = dirt_chars[rng.random_range(0..dirt_chars.len() as u32) as usize];
                let c = mix(dirt, rgb3(darken(palette[0], 40)), depth * 0.6);
                bg[y][x] = Cell::new(ch, scale(c, 1.0));
            }
        }
    }
    // stars: dense on the ethereal side (also below ground, a mirrored sky),
    // sparse and above the horizon on the physical side
    let mut stars = Vec::new();
    for y in 0..h {
        for x in 0..w {
            let eth = (x as i32) < divide;
            let p = if eth { 0.02 } else if y < ground { 0.012 } else { 0.0 };
            if rng.random::<f32>() < p {
                stars.push(Star {
                    x,
                    y,
                    phase: rng.random::<f32>() * 6.2832,
                    rate: 0.6 + rng.random::<f32>() * 2.2,
                });
            }
        }
    }
    let lo = 1.0_f32;
    let eth_w = (divide.max(1) as f32 - 2.0).max(1.0);
    let mut motes = Vec::with_capacity(k.motes);
    for _ in 0..k.motes {
        motes.push(Mote {
            x0: lo + rng.random::<f32>() * eth_w,
            phase: rng.random::<f32>(),
            rate: 0.25 + rng.random::<f32>() * 0.45,
            amp: 1.0 + rng.random::<f32>() * 3.0,
            freq: 0.6 + rng.random::<f32>() * 1.6,
            tint: rng.random::<f32>(),
        });
    }
    let phys_w = (w as f32 - divide as f32 - 2.0).max(1.0);
    let mut leaves = Vec::with_capacity(k.motes / 2);
    for _ in 0..(k.motes / 2) {
        leaves.push(Mote {
            x0: divide as f32 + 1.0 + rng.random::<f32>() * phys_w,
            phase: rng.random::<f32>(),
            rate: 0.18 + rng.random::<f32>() * 0.3,
            amp: 1.5 + rng.random::<f32>() * 3.5,
            freq: 1.0 + rng.random::<f32>() * 2.5,
            tint: rng.random::<f32>(),
        });
    }

    Cached {
        key: (w, h, seed, k.geometry_key()),
        bg,
        cells,
        stars,
        motes,
        leaves,
        ground,
        divide,
        cx,
        eth_rgb,
        eth_hi,
        leaf_rgb,
        leaf_hi,
    }
}

const ETH_TRUNK: [char; 3] = ['░', '▒', '▓'];
const ETH_TWIG: [char; 2] = ['·', '∙'];
const ETH_LEAF: [char; 3] = ['°', '○', '◦'];
const BARK: [[char; 2]; 6] = [['│', '║'], ['─', '═'], ['/', '╱'], ['\\', '╲'], ['▌', '║'], ['▐', '║']];
const LEAF: [[char; 2]; 4] = [['♠', '♣'], ['♣', '♠'], ['*', '♣'], ['♠', '*']];
const MOTE: [char; 4] = ['·', '∙', '°', '○'];
const FALL: [char; 4] = [',', '\'', '`', '"'];

/// Frame render. t == 0 is the canonical static frame.
pub(crate) fn draw_lifetree(grid: &mut Grid, w: usize, h: usize, seed: u64, palette: &[Color; 5], t: f32, k: &LifeKnobs) {
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
fn frame(grid: &mut Grid, c: &Cached, t: f32, k: &LifeKnobs) {
    let w = c.bg[0].len();
    let h = c.bg.len();
    let ts = t * k.speed;
    measure_layer("tree-of-life", "background", || {
        for y in 0..h {
            grid[y][..w].copy_from_slice(&c.bg[y][..w]);
        }
    });

    // stars: strong twinkle on the ethereal side, slow drift on the physical
    measure_layer("tree-of-life", "stars", || {
        for s in &c.stars {
            let eth = (s.x as i32) < c.divide;
            let v = (ts * s.rate + s.phase).sin();
            let (ch, col) = if eth {
                let b = 0.35 + 0.65 * v.max(0.0);
                (if v > 0.75 { '✦' } else if v > 0.1 { '·' } else { '·' }, scale(c.eth_rgb, b * 0.9))
            } else {
                (if v > 0.9 { '✧' } else { '·' }, scale(c.eth_hi, 0.35 + 0.15 * v))
            };
            put(grid, s.x as i32, s.y as i32, ch, col);
        }
    });

    // ethereal horizon: a pulse travelling outward from the trunk
    measure_layer("tree-of-life", "horizon", || {
        if c.divide > 0 {
            let gy = c.ground as i32;
            for x in 0..c.divide {
                let d = (x - c.cx).abs() as f32;
                let p = (d * 0.35 - ts * 2.2).sin();
                let b = 0.3 + 0.7 * k.glow * p.max(0.0);
                let ch = if p > 0.6 { '∙' } else { '·' };
                put(grid, x, gy, ch, scale(c.eth_hi, b));
            }
        }
    });

    // tree cells
    let gf = c.ground as f32;
    measure_layer("tree-of-life", "tree", || {
        for cell in &c.cells {
            let eth = cell.x < c.divide;
            if eth {
                let p = (cell.ord * 11.0 - ts * 1.7 + cell.phase * 0.6).sin();
                let pulse = p.max(0.0);
                let pulse = pulse * pulse;
                let b = 0.42 + 0.58 * k.glow * pulse + 0.1 * (1.0 - k.glow);
                let idx = if pulse > 0.55 { 2 } else if pulse > 0.15 { 1 } else { 0 };
                let ch = match cell.kind {
                    Kind::Trunk | Kind::Branch => ETH_TRUNK[idx],
                    Kind::Root => ETH_TRUNK[idx.min(1)],
                    Kind::Twig | Kind::RootTip => ETH_TWIG[(idx > 0) as usize],
                    Kind::Leaf => continue,
                    Kind::LeafEdge => ETH_LEAF[idx],
                };
                let col = scale(mix(cell.rgb, cell.alt, pulse * 0.7), b);
                put(grid, cell.x, cell.y, ch, col);
            } else {
                let hf = ((gf - cell.y as f32) / gf).clamp(0.0, 1.0);
                let dx = if matches!(cell.kind, Kind::Root | Kind::RootTip) {
                    0
                } else {
                    let s = (ts * 0.9 + cell.ord * 2.6).sin() + 0.35 * (ts * 2.1 + cell.y as f32 * 0.21).sin();
                    (k.sway * hf * hf.sqrt() * s * 0.74).round() as i32
                };
                let (ch, col) = match cell.kind {
                    Kind::Leaf | Kind::LeafEdge => {
                        let r = (ts * 3.1 + cell.phase * 6.2832).sin();
                        let pair = LEAF[(cell.tex & 3) as usize];
                        let ch = if r > 0.62 { pair[1] } else { pair[0] };
                        let light = 0.82 + 0.22 * r.max(0.0) + 0.1 * (ts * 0.4 + cell.x as f32 * 0.05).sin();
                        (ch, scale(mix(cell.rgb, cell.alt, r.max(0.0) * 0.4), light))
                    }
                    Kind::Trunk | Kind::Branch => {
                        let pair = BARK[(cell.tex as usize).min(5)];
                        let ch = if cell.phase > 0.55 { pair[1] } else { pair[0] };
                        let breath = 0.94 + 0.06 * (ts * 0.5 + cell.ord * 3.0).sin();
                        (ch, scale(cell.rgb, breath))
                    }
                    Kind::Twig => {
                        if cell.phase > 0.6 {
                            continue;
                        }
                        ('·', scale(cell.rgb, 0.95))
                    }
                    Kind::Root => {
                        let pair = BARK[(cell.tex as usize).min(5)];
                        (if cell.phase > 0.7 { pair[1] } else { pair[0] }, scale(cell.rgb, 0.9))
                    }
                    Kind::RootTip => ('·', scale(cell.rgb, 0.85)),
                };
                put(grid, cell.x + dx, cell.y, ch, col);
            }
        }
    });

    // motes rise on the ethereal side
    measure_layer("tree-of-life", "motes", || {
        if k.motes > 0 {
            let span = c.ground as f32 + 2.0;
            for m in &c.motes {
                let life = (ts * m.rate * 0.35 + m.phase).fract();
                let y = (c.ground as f32 + 1.0 - life * span).round() as i32;
                let x = (m.x0 + m.amp * (life * m.freq * 6.2832 + m.phase * 6.2832).sin()).round() as i32;
                if x >= c.divide || y < 0 {
                    continue;
                }
                let stage = ((life * 4.0) as usize).min(3);
                let ch = MOTE[[0, 1, 2, 1][stage]];
                let b = (life * 3.1416).sin();
                let col = scale(mix(c.eth_rgb, c.eth_hi, m.tint), 0.35 + 0.65 * b);
                put(grid, x, y, ch, col);
            }
            // leaves fall on the physical side
            let fall_span = (c.ground as f32 - 2.0).max(1.0);
            for m in &c.leaves {
                let life = (ts * m.rate * 0.3 + m.phase).fract();
                let y = (2.0 + life * fall_span).round() as i32;
                let x = (m.x0 + m.amp * (life * m.freq * 6.2832 + m.phase * 6.2832).sin() + k.sway * life * 1.5).round() as i32;
                if x < c.divide || y >= c.ground as i32 {
                    continue;
                }
                let ch = FALL[((life * m.freq * 12.0) as usize) & 3];
                let col = scale(mix(c.leaf_rgb, c.leaf_hi, m.tint), 0.75 + 0.25 * (life * 6.28).sin().abs());
                put(grid, x, y, ch, col);
            }
        }
    });

    // seam: a faint shimmer column where the two halves meet, skipping tree cells
    measure_layer("tree-of-life", "seam", || {
        if c.divide > 0 && (c.divide as usize) < w {
            let x = c.divide;
            for y in 0..h as i32 {
                let cur = grid[y as usize][x as usize].ch;
                if cur != ' ' && cur != '·' && cur != '─' {
                    continue;
                }
                let p = (y as f32 * 0.55 - ts * 2.6).sin();
                if p > 0.35 {
                    let b = 0.3 + 0.5 * k.glow * p;
                    put(grid, x, y, '┆', scale(c.eth_hi, b));
                }
            }
        }
    });
}

pub(crate) fn cli_lifetree(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
    let _ = (rng, term_w, term_h, mode, theme_name);
    let mut k = LifeKnobs::from_env();
    if let Some(v) = args.get(4).and_then(|s| s.parse::<f32>().ok()) {
        k.depth = v.round().clamp(4.0, 11.0) as u32;
    }
    if let Some(v) = args.get(5).and_then(|s| s.parse::<f32>().ok()) {
        k.sway = v.clamp(0.0, 6.0);
    }
    if let Some(v) = args.get(6).and_then(|s| s.parse::<f32>().ok()) {
        k.speed = v.clamp(0.05, 4.0);
    }
    if let Some(v) = args.get(7).and_then(|s| s.parse::<f32>().ok()) {
        k.motes = v.round().clamp(0.0, 300.0) as usize;
    }
    if let Some(v) = args.get(8).and_then(|s| s.parse::<f32>().ok()) {
        k.seam = v.clamp(0.1, 0.9);
    }
    draw_lifetree(&mut grid, width, height, seed, &palette, t_anim, &k);
    (grid, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(w: usize, h: usize, seed: u64, t: f32) -> String {
        let mut g = vec![vec![Cell::blank(); w]; h];
        let p = crate::color::named_theme("moss").unwrap();
        let k = LifeKnobs::from_env();
        draw_lifetree(&mut g, w, h, seed, &p, t, &k);
        g.iter()
            .map(|row| row.iter().map(|c| c.ch).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn snapshot_lifetree_small() {
        insta::assert_snapshot!("lifetree_80x24", run(80, 24, 42, 0.0));
    }

    #[test]
    fn snapshot_lifetree_wide() {
        insta::assert_snapshot!("lifetree_120x40", run(120, 40, 42, 0.0));
    }

    #[test]
    fn deterministic_and_seed_sensitive() {
        assert_eq!(run(90, 30, 42, 0.0), run(90, 30, 42, 0.0));
        assert_ne!(run(90, 30, 42, 0.0), run(90, 30, 7, 0.0));
    }

    #[test]
    fn t_moves_both_halves() {
        let a = run(90, 30, 42, 0.0);
        let b = run(90, 30, 42, 2.0);
        assert_ne!(a, b);
        let left = |s: &str| s.lines().map(|l| l.chars().take(45).collect::<String>()).collect::<Vec<_>>();
        let right = |s: &str| s.lines().map(|l| l.chars().skip(45).collect::<String>()).collect::<Vec<_>>();
        assert_ne!(left(&a), left(&b));
        assert_ne!(right(&a), right(&b));
    }

    #[test]
    fn halves_have_distinct_glyph_pools() {
        let s = run(90, 30, 42, 0.0);
        let left: String = s.lines().map(|l| l.chars().take(45).collect::<String>()).collect();
        let right: String = s.lines().map(|l| l.chars().skip(45).collect::<String>()).collect();
        assert!(left.contains('░') || left.contains('▒'), "ethereal trunk fill");
        assert!(!left.contains('♠') && !left.contains('♣'), "no solid leaves on the ethereal side");
        assert!(right.contains('♠') || right.contains('♣') || right.contains('*'), "living leaves");
        assert!(right.contains('─'), "ground line");
    }

    #[test]
    fn frame_cost_is_flat() {
        let mut g = vec![vec![Cell::blank(); 200]; 60];
        let p = crate::color::named_theme("ember").unwrap();
        let k = LifeKnobs::from_env();
        draw_lifetree(&mut g, 200, 60, 42, &p, 0.0, &k);
        let start = std::time::Instant::now();
        for i in 1..=200 {
            draw_lifetree(&mut g, 200, 60, 42, &p, i as f32 * 0.06, &k);
        }
        let per = start.elapsed().as_secs_f64() / 200.0;
        eprintln!("lifetree frame 200x60: {:.3}ms", per * 1000.0);
        assert!(per < 0.004, "frame {:.3}ms exceeds 4ms budget at 200x60", per * 1000.0);
    }
}
