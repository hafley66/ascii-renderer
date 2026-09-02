//! tree-of-life-3 -- tree-of-life-2 plus eye fruits: algorithmic ellipse eyes at the branch tips that
//! blink on their own clocks, sync-blink, and track a shared gaze target; hollow outlines on the ethereal side.
use crate::color::*;
use crate::opts::param_f32;
use crate::types::*;
use crossterm::style::Color;
use rand::RngExt;
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::cell::RefCell;

#[derive(Clone, Copy, PartialEq)]
pub(crate) struct EyeKnobs {
    pub depth: u32,
    pub spread: f32,
    pub sway: f32,
    pub speed: f32,
    pub motes: usize,
    pub glow: f32,
    pub seam: f32,
    pub roots: f32,
    pub veil: f32,
    pub season: f32,
    pub ring: f32,
    pub tide: f32,
    pub gust: f32,
    pub surge: f32,
    pub flock: usize,
    pub day: f32,
    pub flair: f32,
    pub eyes: usize,
    pub gaze: f32,
    pub blink: f32,
}

impl EyeKnobs {
    pub(crate) fn from_env() -> Self {
        EyeKnobs {
            depth: param_f32("DEPTH", 8.0).round().clamp(4.0, 11.0) as u32,
            spread: param_f32("SPREAD", 0.55).clamp(0.15, 1.2),
            sway: param_f32("SWAY", 2.0).clamp(0.0, 6.0),
            speed: param_f32("SPEED", 1.0).clamp(0.05, 4.0),
            motes: param_f32("MOTES", 40.0).round().clamp(0.0, 300.0) as usize,
            glow: param_f32("GLOW", 0.8).clamp(0.0, 1.0),
            seam: param_f32("SEAM", 0.5).clamp(0.1, 0.9),
            roots: param_f32("ROOTS", 0.28).clamp(0.05, 0.5),
            veil: param_f32("VEIL", 5.0).clamp(0.0, 14.0),
            season: param_f32("SEASON", 0.12).clamp(0.0, 1.0),
            ring: param_f32("RING", 1.0).clamp(0.0, 1.0),
            tide: param_f32("TIDE", 0.7).clamp(0.0, 1.0),
            gust: param_f32("GUST", 0.8).clamp(0.0, 1.0),
            surge: param_f32("SURGE", 1.0).clamp(0.0, 1.0),
            flock: param_f32("FLOCK", 3.0).round().clamp(0.0, 12.0) as usize,
            day: param_f32("DAY", 0.5).clamp(0.0, 2.0),
            flair: param_f32("FLAIR", 1.0).clamp(0.0, 1.0),
            eyes: param_f32("EYES", 12.0).round().clamp(0.0, 80.0) as usize,
            gaze: param_f32("GAZE", 1.0).clamp(0.0, 1.0),
            blink: param_f32("BLINK", 1.0).clamp(0.0, 4.0),
        }
    }

    fn geometry_key(&self) -> (u32, u32, u32, usize, usize) {
        (self.depth, self.spread.to_bits(), self.roots.to_bits(), self.motes, self.eyes)
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

#[derive(Clone, Copy)]
struct TreeCell {
    x: i32,
    y: i32,
    kind: Kind,
    ord: f32,
    phase: f32,
    tex: u8,
    eth: (u8, u8, u8),
    phys: (u8, u8, u8),
    phys_alt: (u8, u8, u8),
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
    y0: f32,
    phase: f32,
    rate: f32,
    amp: f32,
    freq: f32,
    tint: f32,
}

#[derive(Clone, Copy)]
struct Eye {
    x: i32,
    y: i32,
    rx: i32,
    ry: i32,
    phase: f32,
    rate: f32,
    tint: f32,
    lag: f32,
    double: bool,
}

#[derive(Clone, Copy)]
struct RingPt {
    x: i32,
    y: i32,
    ang: f32,
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
    key: (usize, usize, u64, (u32, u32, u32, usize, usize)),
    bg_eth: Grid,
    bg_phys: Grid,
    cells: Vec<TreeCell>,
    stars: Vec<Star>,
    motes: Vec<Mote>,
    fallers: Vec<Mote>,
    ring: Vec<RingPt>,
    tips: Vec<(i32, i32)>,
    eyes: Vec<Eye>,
    sky_rows: Vec<((u8, u8, u8), (u8, u8, u8), (u8, u8, u8))>,
    ground: usize,
    cx: i32,
    eth_rgb: (u8, u8, u8),
    eth_hi: (u8, u8, u8),
    leaf_rgb: (u8, u8, u8),
    leaf_hi: (u8, u8, u8),
    spring: (u8, u8, u8),
    autumn: (u8, u8, u8),
    bark_hi: (u8, u8, u8),
    eth_dark: (u8, u8, u8),
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

/// Veil x for a row: seam column plus two travelling harmonics. t == 0 is a fixed wave.
#[inline]
fn veil_x(y: i32, w: usize, ts: f32, k: &EyeKnobs) -> i32 {
    let tide = k.tide * 0.5 * (ts * 0.11).sin() + k.tide * 0.12 * (ts * 0.37).sin();
    let base = w as f32 * (k.seam + tide).clamp(-0.05, 1.05);
    let fy = y as f32;
    let wave = (fy * 0.33 + ts * 0.45).sin() + 0.45 * (fy * 0.81 - ts * 0.7).sin();
    (base + k.veil * wave).round().clamp(0.0, w as f32) as i32
}

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

fn blank_cell(x: i32, y: i32, kind: Kind, ord: f32, phase: f32, tex: u8) -> TreeCell {
    TreeCell { x, y, kind, ord, phase, tex, eth: (0, 0, 0), phys: (0, 0, 0), phys_alt: (0, 0, 0) }
}

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
                map[py as usize * w + x as usize] = Some(blank_cell(x, py, kind, ord, rng.random::<f32>(), tex_k));
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
            let tex = rng.random_range(0..4u32) as u8;
            map[i] = Some(blank_cell(x, y, Kind::Leaf, ord, rng.random::<f32>(), tex));
        }
    }
}

fn build(w: usize, h: usize, seed: u64, palette: &[Color; 5], k: &EyeKnobs) -> Cached {
    let mut rng = StdRng::seed_from_u64(seed ^ 0x2A7C_11D3_9E37_79B9);
    let cx = (w / 2) as i32;
    let ground = ((h as f32) * (1.0 - k.roots)).round().clamp(4.0, h as f32 - 3.0) as usize;

    let eth_hue = (hue_of(palette[3]) + 150.0).rem_euclid(360.0);
    let eth_rgb = rgb3(hsl_to_rgb(eth_hue, 0.55, 0.66));
    let eth_hi = rgb3(hsl_to_rgb(eth_hue, 0.35, 0.92));
    let eth_dark = rgb3(hsl_to_rgb(eth_hue, 0.45, 0.06));
    let leaf_rgb = rgb3(palette[1]);
    let leaf_hi = rgb3(palette[3]);
    let leaf_hue = hue_of(palette[1]);
    let spring = rgb3(hsl_to_rgb((leaf_hue + 25.0).rem_euclid(360.0), 0.6, 0.72));
    let autumn = rgb3(hsl_to_rgb(28.0, 0.85, 0.5));
    let bark = rgb3(darken(palette[2], 25));
    let bark_hi = rgb3(lighten(palette[2], 20));
    let dirt = rgb3(darken(palette[2], 55));
    let sky = rgb3(palette[0]);

    let mut canopy: Vec<Seg> = Vec::new();
    let base_thick = (w as f32 / 30.0).clamp(1.0, 7.0);
    grow(&mut rng, &mut canopy, 0.0, 0.0, std::f32::consts::FRAC_PI_2, 10.0, 0, k.depth, base_thick, 0.0, k.spread);
    let half_w = (w as f32 * 0.42).max(4.0);
    fit(&mut canopy, cx as f32, ground as f32, half_w, (ground as f32 - 2.5).max(2.0));
    let ord_max = canopy.iter().fold(1.0_f32, |m, s| m.max(s.ord1));

    let mut roots: Vec<Seg> = Vec::new();
    let root_lvl = (k.depth.saturating_sub(3)).max(2);
    grow(&mut rng, &mut roots, 0.0, 0.0, -std::f32::consts::FRAC_PI_2, 8.0, 0, root_lvl, base_thick * 0.55, 0.0, k.spread * 1.2);
    let root_span = (h as f32 - ground as f32 - 1.5).max(1.0);
    fit(&mut roots, cx as f32, ground as f32, half_w * 0.45, root_span);
    let root_ord_max = roots.iter().fold(1.0_f32, |m, s| m.max(s.ord1));

    let mut map: Vec<Option<TreeCell>> = vec![None; w * h];
    raster(&roots, root_lvl, true, root_ord_max, w, h, &mut rng, &mut map);
    raster(&canopy, k.depth, false, ord_max, w, h, &mut rng, &mut map);
    for s in canopy.iter().filter(|s| s.lvl + 1 >= k.depth) {
        if rng.random::<f32>() < 0.25 {
            continue;
        }
        let r = if rng.random::<f32>() < 0.15 { 2 } else { 1 };
        blob(&mut map, w, h, s.x1.round() as i32, s.y1.round() as i32, r, (s.ord1 / ord_max).min(1.0), &mut rng);
    }
    let mut edge_idx: Vec<usize> = Vec::new();
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            let Some(c) = &map[i] else { continue };
            if c.kind != Kind::Leaf {
                continue;
            }
            let open = [(0i32, -1i32), (0, 1), (-1, 0), (1, 0)].iter().any(|(dx, dy)| {
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 || map[ny as usize * w + nx as usize].is_none()
            });
            if open {
                edge_idx.push(i);
            }
        }
    }
    for i in edge_idx {
        if let Some(c) = &mut map[i] {
            c.kind = Kind::LeafEdge;
        }
    }

    let mut cells: Vec<TreeCell> = Vec::with_capacity(map.len() / 8);
    for c in map.into_iter().flatten() {
        let mut c = c;
        let tip = c.ord.clamp(0.0, 1.0);
        c.eth = mix(eth_rgb, eth_hi, tip * 0.5);
        match c.kind {
            Kind::Trunk | Kind::Branch => {
                let light = if c.tex == 5 { 0.55 } else if c.tex == 4 { 0.0 } else { 0.25 };
                c.phys = mix(bark, bark_hi, light + c.ord * 0.2);
                c.phys_alt = bark_hi;
            }
            Kind::Twig => {
                c.phys = mix(bark_hi, leaf_rgb, 0.35);
                c.phys_alt = leaf_rgb;
            }
            Kind::Leaf | Kind::LeafEdge => {
                let shade = c.phase * 0.5 + if c.kind == Kind::LeafEdge { 0.3 } else { 0.0 };
                c.phys = mix(leaf_rgb, leaf_hi, shade);
                c.phys_alt = mix(leaf_hi, (255, 255, 240), 0.3);
            }
            Kind::Root | Kind::RootTip => {
                c.phys = mix(bark, dirt, 0.3 + c.ord * 0.5);
                c.phys_alt = bark;
            }
        }
        cells.push(c);
    }
    cells.sort_by_key(|c| match c.kind {
        Kind::Root => 0,
        Kind::RootTip => 1,
        Kind::Trunk => 2,
        Kind::Branch => 3,
        Kind::Twig => 4,
        Kind::Leaf => 5,
        Kind::LeafEdge => 6,
    });

    let mut bg_eth = vec![vec![Cell::blank(); w]; h];
    let mut bg_phys = vec![vec![Cell::blank(); w]; h];
    let dirt_chars = ['·', '∙', ',', '·', '"', '~', '·'];
    for y in 0..h {
        let fy = y as f32 / h as f32;
        let dist = (y as f32 - ground as f32).abs() / h as f32;
        let eth_c = mix(eth_dark, eth_rgb, (0.05 + (1.0 - dist) * 0.05) * 0.3);
        for x in 0..w {
            bg_eth[y][x] = Cell::new(' ', scale(eth_c, 1.0));
            if y < ground {
                let c = mix(rgb3(darken(palette[0], 70)), sky, fy * 1.3);
                bg_phys[y][x] = Cell::new(' ', scale(c, 1.0));
            } else if y == ground {
                bg_phys[y][x] = Cell::new('─', scale(mix(dirt, bark_hi, 0.25), 1.0));
            } else {
                let depth = (y - ground) as f32 / (h - ground).max(1) as f32;
                let ch = dirt_chars[rng.random_range(0..dirt_chars.len() as u32) as usize];
                let c = mix(dirt, rgb3(darken(palette[0], 40)), depth * 0.6);
                bg_phys[y][x] = Cell::new(ch, scale(c, 1.0));
            }
        }
    }
    let mut stars = Vec::new();
    for y in 0..h {
        for x in 0..w {
            if rng.random::<f32>() < 0.02 {
                stars.push(Star { x, y, phase: rng.random::<f32>() * 6.2832, rate: 0.6 + rng.random::<f32>() * 2.2 });
            }
        }
    }
    let mut motes = Vec::with_capacity(k.motes);
    for _ in 0..k.motes {
        motes.push(Mote {
            x0: 1.0 + rng.random::<f32>() * (w as f32 - 2.0),
            y0: 0.0,
            phase: rng.random::<f32>(),
            rate: 0.25 + rng.random::<f32>() * 0.45,
            amp: 1.0 + rng.random::<f32>() * 3.0,
            freq: 0.6 + rng.random::<f32>() * 1.6,
            tint: rng.random::<f32>(),
        });
    }
    let leaf_cells: Vec<(i32, i32)> = cells
        .iter()
        .filter(|c| matches!(c.kind, Kind::Leaf | Kind::LeafEdge))
        .map(|c| (c.x, c.y))
        .collect();
    let mut fallers = Vec::with_capacity(k.motes);
    for _ in 0..k.motes {
        let (lx, ly) = if leaf_cells.is_empty() {
            ((1.0 + rng.random::<f32>() * (w as f32 - 2.0)) as i32, 2)
        } else {
            leaf_cells[rng.random_range(0..leaf_cells.len() as u32) as usize]
        };
        fallers.push(Mote {
            x0: lx as f32,
            y0: ly as f32,
            phase: rng.random::<f32>(),
            rate: 0.18 + rng.random::<f32>() * 0.3,
            amp: 1.5 + rng.random::<f32>() * 3.5,
            freq: 1.0 + rng.random::<f32>() * 2.5,
            tint: rng.random::<f32>(),
        });
    }
    // canopy tips for the surge burst, thinned to at most 96
    let mut tips: Vec<(i32, i32)> = canopy
        .iter()
        .filter(|s| s.lvl + 1 >= k.depth)
        .map(|s| (s.x1.round() as i32, s.y1.round() as i32))
        .collect();
    if tips.len() > 96 {
        let stride = tips.len() / 96 + 1;
        tips = tips.into_iter().step_by(stride).collect();
    }
    // eye fruits: greedy spacing over shuffled leaf cells, biggest eyes nearest the trunk
    let mut eyes: Vec<Eye> = Vec::with_capacity(k.eyes);
    if k.eyes > 0 && !leaf_cells.is_empty() {
        let mut order: Vec<usize> = (0..leaf_cells.len()).collect();
        for i in (1..order.len()).rev() {
            let j = rng.random_range(0..(i as u32 + 1)) as usize;
            order.swap(i, j);
        }
        let big = (w as f32 / 16.0).clamp(4.0, 8.0);
        for &i in &order {
            if eyes.len() >= k.eyes {
                break;
            }
            let (ex, ey) = leaf_cells[i];
            if ey < 2 || ey + 2 >= ground as i32 {
                continue;
            }
            let near = ((ex - cx).abs() as f32 / half_w).clamp(0.0, 1.0);
            let rx = (big * (1.0 - near * 0.35) * (0.75 + rng.random::<f32>() * 0.4)).round().max(4.0) as i32;
            let ry = if rx >= 6 { 2 } else { 1 };
            if ex - rx - 1 < 0 || ex + rx + 1 >= w as i32 || ey - ry - 1 < 1 {
                continue;
            }
            if eyes.iter().any(|e| (e.x - ex).abs() <= e.rx + rx + 3 && (e.y - ey).abs() <= e.ry + ry + 2) {
                continue;
            }
            eyes.push(Eye {
                x: ex,
                y: ey,
                rx,
                ry,
                phase: rng.random::<f32>(),
                rate: 0.05 + rng.random::<f32>() * 0.09,
                tint: rng.random::<f32>(),
                lag: rng.random::<f32>() * 0.9,
                double: rng.random::<f32>() < 0.3,
            });
        }
    }
    // per-row sky colours: (night, day) for the living half, base for the ethereal half
    let day_sky = mix(sky, (110, 165, 235), 0.65);
    let mut sky_rows: Vec<((u8, u8, u8), (u8, u8, u8), (u8, u8, u8))> = Vec::with_capacity(h);
    for y in 0..h {
        let fy = y as f32 / h as f32;
        let night = mix(rgb3(darken(palette[0], 70)), sky, fy * 1.3);
        let day = mix(mix(day_sky, (200, 220, 245), 0.25), day_sky, fy * 1.2);
        let dist = (y as f32 - ground as f32).abs() / h as f32;
        let eth_c = mix(eth_dark, eth_rgb, (0.05 + (1.0 - dist) * 0.05) * 0.3);
        sky_rows.push((night, day, eth_c));
    }
    // ring of life: ellipse centred mid-grid, tangent to the frame
    let rcx = cx as f32;
    let rcy = h as f32 * 0.5 - 0.5;
    let rx = w as f32 * 0.5 - 1.5;
    let ry = h as f32 * 0.5 - 1.0;
    let n = ((rx + ry) * 4.0) as usize;
    let mut ring: Vec<RingPt> = Vec::with_capacity(n);
    for i in 0..n {
        let ang = i as f32 / n as f32 * 6.2832;
        let x = (rcx + ang.cos() * rx).round() as i32;
        let y = (rcy + ang.sin() * ry).round() as i32;
        if ring.last().map(|p| p.x == x && p.y == y).unwrap_or(false) {
            continue;
        }
        if x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h {
            ring.push(RingPt { x, y, ang });
        }
    }

    Cached {
        key: (w, h, seed, k.geometry_key()),
        bg_eth,
        bg_phys,
        cells,
        stars,
        motes,
        fallers,
        ring,
        tips,
        eyes,
        sky_rows,
        ground,
        cx,
        eth_rgb,
        eth_hi,
        leaf_rgb,
        leaf_hi,
        spring,
        autumn,
        bark_hi,
        eth_dark,
    }
}

const ETH_TRUNK: [char; 3] = ['░', '▒', '▓'];
const ETH_TWIG: [char; 2] = ['·', '∙'];
const ETH_LEAF: [char; 3] = ['°', '○', '◦'];
const BARK: [[char; 2]; 6] = [['│', '║'], ['─', '═'], ['/', '╱'], ['\\', '╲'], ['▌', '║'], ['▐', '║']];
const LEAF: [[char; 2]; 4] = [['♠', '♣'], ['♣', '♠'], ['*', '♣'], ['♠', '*']];
const MOTE: [char; 4] = ['·', '∙', '°', '○'];
const FALL: [char; 4] = [',', '\'', '`', '"'];
const RUNE: [char; 5] = ['·', '◦', '○', '✧', '✦'];
const VINE: [char; 3] = ['~', '·', '♣'];
const SPARK: [char; 4] = ['✦', '✧', '∙', '·'];
const BIRD: [char; 2] = ['v', '~'];
const WISP: [char; 4] = ['○', '°', '∙', '·'];
const SEAM_TEXT: &[u8] = b"ARBOR\xb7VITAE\xb7MEMENTO\xb7MORI\xb7ANIMA\xb7MUNDI\xb7";

const SCLERA: (u8, u8, u8) = (236, 230, 216);
const PUPIL: (u8, u8, u8) = (12, 8, 14);

#[inline]
fn hash2(a: i32, b: i32) -> u32 {
    let mut x = (a as u32).wrapping_mul(0x9E37_79B9) ^ (b as u32).wrapping_mul(0x85EB_CA6B);
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^= x >> 12;
    x
}

/// Eyes clamp shut for a beat when the surge burst fires (phase window fixed per eye).
#[inline]
fn surge_blink(e: &Eye, ts: f32) -> f32 {
    let su = (ts * 0.16).fract();
    let start = 0.72 + e.phase * 0.05;
    if su > start && su < start + 0.06 { ((su - start) / 0.06 * 3.1416).sin() } else { 0.0 }
}

#[inline]
fn smooth(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// Season weights (spring, summer, autumn, winter) from a 0..4 phase, each a tent of width 2.
#[inline]
fn season_w(phase: f32) -> [f32; 4] {
    let mut w = [0.0f32; 4];
    for (i, slot) in w.iter_mut().enumerate() {
        let mut d = (phase - i as f32).abs();
        d = d.min(4.0 - d);
        *slot = (1.0 - d).max(0.0);
    }
    w
}

pub(crate) fn draw_lifetree3(grid: &mut Grid, w: usize, h: usize, seed: u64, palette: &[Color; 5], t: f32, k: &EyeKnobs) {
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
fn frame(grid: &mut Grid, c: &Cached, t: f32, k: &EyeKnobs) {
    let w = c.bg_eth[0].len();
    let h = c.bg_eth.len();
    let ts = t * k.speed;
    let hh = h.min(512);

    // clocks: gust envelope, surge front, heartbeat, day, negative flash
    let gust = smooth(((ts * 0.19).sin() + 0.6 * (ts * 0.53).sin() - 0.55) * 2.2) * k.gust;
    let su = (ts * 0.16).fract();
    let root_front = 1.0 - su * 4.0;
    let canopy_front = (su - 0.22) * 1.9;
    let burst = ((su - 0.74) / 0.26).clamp(0.0, 1.0) * k.surge;
    let burst_on = k.surge > 0.0 && su > 0.74;
    let negative = k.flair > 0.0 && k.surge > 0.0 && su > 0.74 && su < 0.79;
    let beat_p = (ts * 0.55).rem_euclid(3.0);
    let flash = ((1.0 - beat_p * 5.0).max(0.0) + 0.6 * (1.0 - (beat_p - 0.32).abs() * 6.0).max(0.0)) * k.glow;
    let beat = beat_p / 3.0;
    let beat_r = beat * (w as f32 * 0.55);
    let beat2_r = ((beat_p - 0.32).max(0.0) / 3.0) * (w as f32 * 0.55);
    let heart_y = c.ground as f32 - 2.0;
    let sun_a = ts * 0.07 * k.day;
    let day = if k.day > 0.0 { smooth(sun_a.sin() * 1.4 + 0.5) } else { 0.0 };
    let glitch_rows = k.flair * flash;

    // veil per row, then background compose: sky rows are uniform, dirt rows memcpy
    let mut veil: [i32; 512] = [0; 512];
    for y in 0..hh {
        let v = veil_x(y as i32, w, ts, k).clamp(0, w as i32) as usize;
        veil[y] = v as i32;
        let (night, dayc, ethc) = c.sky_rows[y];
        let eth_col = if negative { scale(c.eth_hi, 0.85) } else { scale(ethc, 1.0 + flash * 0.9) };
        let eth_cell = Cell::new(' ', eth_col);
        grid[y][..v].fill(eth_cell);
        if y < c.ground {
            grid[y][v..w].fill(Cell::new(' ', scale(mix(night, dayc, day), 1.0)));
        } else {
            grid[y][v..w].copy_from_slice(&c.bg_phys[y][v..w]);
        }
    }
    let is_eth = |x: i32, y: i32| -> bool { y >= 0 && (y as usize) < hh && x < veil[y as usize] };
    let ink = |bright: (u8, u8, u8), b: f32| -> Color {
        if negative { scale(c.eth_dark, 1.0 + b * 0.5) } else { scale(bright, b) }
    };
    let shear = |y: i32| -> i32 {
        if glitch_rows < 0.05 {
            0
        } else {
            let r = hash2(y, (beat_p * 40.0) as i32) % 9;
            (((r as i32) - 4) as f32 * glitch_rows * 1.4).round() as i32
        }
    };

    // seasons on the living half
    let sw = season_w((ts * k.season).rem_euclid(4.0));
    let season_tint = mix(mix(c.spring, c.leaf_rgb, sw[1] / (sw[0] + sw[1]).max(1e-3)), c.autumn, sw[2]);
    let leaf_keep = (0.6 * sw[0] + 1.0 * sw[1] + 0.8 * sw[2] + 0.12 * sw[3]) - gust * 0.3;
    let season_mix = 0.7 * sw[0] + 0.85 * sw[2];
    let snow = sw[3];

    // sun and moon on the living sky
    if k.day > 0.0 {
        let sx = (c.cx as f32 + sun_a.cos() * w as f32 * 0.42).round() as i32;
        let sy = (c.ground as f32 - 1.0 - sun_a.sin() * (c.ground as f32 - 2.0)).round() as i32;
        let mx = (c.cx as f32 - sun_a.cos() * w as f32 * 0.42).round() as i32;
        let my = (c.ground as f32 - 1.0 + sun_a.sin() * (c.ground as f32 - 2.0)).round() as i32;
        if sy < c.ground as i32 && !is_eth(sx, sy) {
            put(grid, sx, sy, '◉', scale((255, 220, 120), 1.0));
            for (dx, dy) in [(-2, 0), (2, 0), (0, -1), (0, 1)] {
                if grid.get(( sy + dy).max(0) as usize).and_then(|r| r.get((sx + dx).max(0) as usize)).map(|cc| cc.ch == ' ').unwrap_or(false) && !is_eth(sx + dx, sy + dy) {
                    put(grid, sx + dx, sy + dy, '·', scale((255, 200, 90), 0.8));
                }
            }
        }
        if my < c.ground as i32 && !is_eth(mx, my) {
            put(grid, mx, my, '○', scale((220, 225, 240), 0.95));
        }
    }

    for s in &c.stars {
        let v = (ts * s.rate + s.phase).sin();
        let (ch, col) = if is_eth(s.x as i32, s.y as i32) {
            let b = 0.35 + 0.65 * v.max(0.0) + flash * 0.4;
            (if v > 0.75 { '✦' } else { '·' }, ink(c.eth_rgb, b * 0.9))
        } else if s.y < c.ground {
            let b = (0.35 + 0.15 * v) * (1.0 - day);
            if b < 0.12 {
                continue;
            }
            (if v > 0.9 { '✧' } else { '·' }, scale(c.eth_hi, b))
        } else {
            continue;
        };
        put(grid, s.x as i32, s.y as i32, ch, col);
    }

    // shooting stars on the ethereal half: two streaks with short tails
    for i in 0..2 {
        let life = (ts * (0.21 + i as f32 * 0.07) + i as f32 * 0.5).fract();
        if life > 0.35 {
            continue;
        }
        let f = life / 0.35;
        let sx0 = (hash2(i, (ts * 0.21) as i32) % w.max(1) as u32) as f32;
        let sy0 = 0.0;
        for tail in 0..5 {
            let ft = f - tail as f32 * 0.04;
            if ft < 0.0 {
                break;
            }
            let x = (sx0 - ft * w as f32 * 0.4).round() as i32;
            let y = (sy0 + ft * c.ground as f32 * 0.8).round() as i32;
            if is_eth(x, y) {
                put(grid, x, y, if tail == 0 { '✦' } else { '·' }, ink(c.eth_hi, 1.0 - tail as f32 * 0.18));
            }
        }
    }

    // ring of life with a comet on the ethereal arc
    if k.ring > 0.0 && !c.ring.is_empty() {
        let n = c.ring.len() as f32;
        let head = (ts * 0.4).fract() * n;
        for (i, p) in c.ring.iter().enumerate() {
            if is_eth(p.x, p.y) {
                let mut d = (i as f32 - head).rem_euclid(n);
                if d > n * 0.5 {
                    d = n;
                }
                let comet = (1.0 - d / (n * 0.12)).max(0.0);
                let v = (p.ang * 5.0 - ts * 1.9).sin();
                let base = v * 0.5 + 0.5;
                let idx = (((base + comet).min(1.0)) * 4.99) as usize;
                let b = 0.35 + 0.65 * k.glow * base.max(comet);
                put(grid, p.x, p.y, RUNE[idx.min(4)], ink(c.eth_hi, b * k.ring));
            } else {
                let v = (p.ang * 9.0 + ts * 0.3).sin();
                let idx = if v > 0.6 { 2 } else if v > -0.3 { 0 } else { 1 };
                put(grid, p.x, p.y, VINE[idx], scale(c.bark_hi, 0.75 * k.ring + 0.25));
            }
        }
    }

    // ethereal horizon pulse
    let gy = c.ground as i32;
    if gy >= 0 && (gy as usize) < hh {
        for x in 0..veil[gy as usize] {
            let d = (x - c.cx).abs() as f32;
            let p = (d * 0.35 - ts * 2.2).sin();
            let b = 0.3 + 0.7 * k.glow * p.max(0.0) + flash * 0.5;
            put(grid, x, gy, if p > 0.6 { '∙' } else { '·' }, ink(c.eth_hi, b));
        }
    }

    // heartbeat rings on the ethereal half over empty cells
    if k.glow > 0.0 {
        for (r, fade) in [(beat_r, (1.0 - beat).powi(2)), (beat2_r, (1.0 - beat).powi(2) * 0.6)] {
            if r <= 0.0 {
                continue;
            }
            let steps = (r * 3.0) as usize + 8;
            for i in 0..steps {
                let a = i as f32 / steps as f32 * 6.2832;
                let x = (c.cx as f32 + a.cos() * r).round() as i32;
                let y = (heart_y + a.sin() * r * 0.5).round() as i32;
                if !is_eth(x, y) || x < 0 || y < 0 || x as usize >= w {
                    continue;
                }
                if grid[y as usize][x as usize].ch == ' ' {
                    put(grid, x, y, if fade > 0.5 { '∙' } else { '·' }, ink(c.eth_rgb, 0.4 + 0.6 * fade * k.glow));
                }
            }
        }
    }

    // still-water reflection of the canopy under the ethereal horizon
    if k.flair > 0.0 {
        for cell in &c.cells {
            if matches!(cell.kind, Kind::Root | Kind::RootTip | Kind::Twig) {
                continue;
            }
            let ry = 2 * gy - cell.y;
            if ry <= gy || ry >= h as i32 || !is_eth(cell.x, ry) {
                continue;
            }
            let wob = ((ry as f32 * 0.9 + ts * 1.3).sin() * 1.5).round() as i32;
            let x = cell.x + wob;
            if x < 0 || x as usize >= w || grid[ry as usize][x as usize].ch != ' ' {
                continue;
            }
            let depth = (ry - gy) as f32 / (h as i32 - gy).max(1) as f32;
            let ch = if cell.kind == Kind::Trunk { '˙' } else { '·' };
            put(grid, x, ry, ch, ink(c.eth_rgb, (0.55 - depth * 0.4) * k.flair));
        }
    }

    let gf = c.ground as f32;
    let sway_eff = k.sway * (1.0 + 2.5 * gust);
    for cell in &c.cells {
        let hf = ((gf - cell.y as f32) / gf).clamp(0.0, 1.0);
        let rooted = matches!(cell.kind, Kind::Root | Kind::RootTip);
        // surge: how far behind the front this cell sits (0 = at the front)
        let surge = if k.surge <= 0.0 {
            0.0
        } else if rooted {
            let lag = cell.ord - root_front;
            if lag >= 0.0 && lag < 0.5 { (1.0 - lag * 2.0) * k.surge } else { 0.0 }
        } else {
            let lag = canopy_front - cell.ord;
            if lag >= 0.0 && lag < 0.35 { (1.0 - lag / 0.35) * k.surge } else { 0.0 }
        };
        if is_eth(cell.x, cell.y) {
            let dx = if rooted {
                0
            } else {
                let te = ts - 0.6;
                let s = (te * 0.9 + cell.ord * 2.6).sin() + 0.35 * (te * 2.1 + cell.y as f32 * 0.21).sin();
                (sway_eff * 0.5 * hf * hf.sqrt() * s * 0.74).round() as i32
            } + shear(cell.y);
            let p = (cell.ord * 11.0 - ts * 1.7 + cell.phase * 0.6).sin();
            let pulse = p.max(0.0);
            let pulse = pulse * pulse;
            let ddx = cell.x as f32 - c.cx as f32;
            let ddy = (cell.y as f32 - heart_y) * 2.0;
            let d = (ddx * ddx + ddy * ddy).sqrt();
            let beat_hit = (1.0 - ((d - beat_r).abs() / 2.5)).max(0.0).max((1.0 - ((d - beat2_r).abs() / 2.5)).max(0.0) * 0.7) * (1.0 - beat);
            let lit = pulse.max(beat_hit).max(surge);
            let b = 0.42 + 0.58 * k.glow * lit + 0.1 * (1.0 - k.glow) + surge * 0.4;
            let idx = if lit > 0.55 { 2 } else if lit > 0.15 { 1 } else { 0 };
            let ch = match cell.kind {
                Kind::Trunk | Kind::Branch => if surge > 0.6 { '█' } else { ETH_TRUNK[idx] },
                Kind::Root => if surge > 0.6 { '▓' } else { ETH_TRUNK[idx.min(1)] },
                Kind::Twig | Kind::RootTip => ETH_TWIG[(idx > 0) as usize],
                Kind::Leaf => {
                    if burst_on && cell.phase < burst { '✦' } else { continue }
                }
                Kind::LeafEdge => if burst_on && cell.phase < burst * 0.5 { '✧' } else { ETH_LEAF[idx] },
            };
            put(grid, cell.x + dx, cell.y, ch, ink(mix(cell.eth, c.eth_hi, lit * 0.7), b));
        } else {
            let dx = if rooted {
                0
            } else {
                let s = (ts * 0.9 + cell.ord * 2.6).sin() + 0.35 * (ts * 2.1 + cell.y as f32 * 0.21).sin();
                (sway_eff * hf * hf.sqrt() * s * 0.74 + gust * 2.0 * hf).round() as i32
            };
            let daylight = 1.0 + 0.18 * day;
            let (ch, col) = match cell.kind {
                Kind::Leaf | Kind::LeafEdge => {
                    if cell.phase > leaf_keep {
                        if snow > 0.3 && cell.phase < leaf_keep + 0.25 * snow {
                            ('·', scale((235, 240, 255), 0.6 + 0.4 * snow))
                        } else if burst_on && cell.phase < leaf_keep + burst * 0.5 {
                            ('*', scale((255, 235, 200), 0.9))
                        } else {
                            continue;
                        }
                    } else {
                        let r = (ts * (3.1 + gust * 6.0) + cell.phase * 6.2832).sin();
                        let pair = LEAF[(cell.tex & 3) as usize];
                        let ch = if surge > 0.7 && sw[0] > 0.3 { '*' } else if r > 0.62 { pair[1] } else { pair[0] };
                        let light = (0.82 + 0.22 * r.max(0.0) + 0.1 * (ts * 0.4 + cell.x as f32 * 0.05).sin()) * daylight + surge * 0.3;
                        let base = mix(cell.phys, season_tint, season_mix);
                        (ch, scale(mix(base, cell.phys_alt, r.max(0.0) * 0.4 + surge * 0.5), light))
                    }
                }
                Kind::Trunk | Kind::Branch => {
                    let pair = BARK[(cell.tex as usize).min(5)];
                    let ch = if cell.phase > 0.55 { pair[1] } else { pair[0] };
                    let breath = (0.94 + 0.06 * (ts * 0.5 + cell.ord * 3.0).sin()) * daylight;
                    (ch, scale(mix(cell.phys, c.eth_hi, surge * 0.6), breath + surge * 0.3))
                }
                Kind::Twig => {
                    if cell.phase > 0.6 {
                        continue;
                    }
                    ('·', scale(mix(cell.phys, c.eth_hi, surge * 0.6), 0.95 * daylight))
                }
                Kind::Root => {
                    let pair = BARK[(cell.tex as usize).min(5)];
                    (if cell.phase > 0.7 { pair[1] } else { pair[0] }, scale(mix(cell.phys, c.eth_hi, surge * 0.6), 0.9 + surge * 0.3))
                }
                Kind::RootTip => ('·', scale(mix(cell.phys, c.eth_hi, surge * 0.6), 0.85 + surge * 0.3)),
            };
            put(grid, cell.x + dx, cell.y, ch, col);
        }
    }

    // eye fruits: size by season, blink on own clock plus a sync blink, gaze at the lead wisp
    if k.eyes > 0 {
        let size_k = 0.75 * sw[0] + 1.0 * sw[1] + 0.9 * sw[2] + 0.8 * sw[3];
        let asleep = (sw[3] * 1.6 - 0.3).clamp(0.0, 1.0);
        let sync_p = (ts * 0.07).fract();
        let sync_blink = if sync_p < 0.04 { (sync_p / 0.04 * 3.1416).sin() } else { 0.0 };
        let iris_base = mix(c.leaf_hi, c.eth_rgb, 0.35);
        let iris_alt = mix(c.autumn, c.leaf_rgb, 0.4);
        let socket = mix(c.leaf_rgb, (0, 0, 0), 0.8);
        for e in &c.eyes {
            if size_k < 0.25 {
                break;
            }
            let rx = ((e.rx as f32) * size_k).round().max(3.0) as i32;
            let ry = if rx >= 6 { 2 } else { 1 };
            let bp = (ts * e.rate * k.blink + e.phase).fract();
            let mut blink = if bp < 0.07 { (bp / 0.07 * 3.1416).sin() } else { 0.0 };
            if e.double && bp > 0.10 && bp < 0.17 {
                blink = blink.max(((bp - 0.10) / 0.07 * 3.1416).sin());
            }
            let blink = blink.max(sync_blink).max(surge_blink(e, ts)).max(asleep);
            let open = (1.0 - blink).clamp(0.0, 1.0);
            let eth = is_eth(e.x, e.y);
            let tt = ts - e.lag;
            let tx = c.cx as f32 + w as f32 * 0.45 * (tt * 0.17).sin();
            let ty = gf * 0.5 + gf * 0.45 * (tt * 0.29).cos();
            let gmax = (rx - 3).max(0) as f32;
            let gx = ((tx - e.x as f32) / w as f32 * 3.0 * rx as f32 * k.gaze).round().clamp(-gmax, gmax) as i32;
            let gy = ((ty - e.y as f32) / h as f32 * 2.6 * ry as f32 * k.gaze).round().clamp(-(ry - 1).max(0) as f32, (ry - 1).max(0) as f32) as i32;
            let iris = mix(iris_base, iris_alt, e.tint);
            let daylight = 0.75 + 0.25 * day;
            let lid = if eth { c.eth_hi } else { mix(c.bark_hi, (0, 0, 0), 0.2) };
            // socket: a dark ellipse one cell wider than the eye so the eye pops off the leaves
            if !eth {
                let sr = ry + 1;
                for dy in -sr..=sr {
                    let f = dy as f32 / (sr as f32 + 0.3);
                    let hw = ((rx + 1) as f32 * (1.0 - f * f).max(0.0).sqrt()).round() as i32;
                    for dx in -hw..=hw {
                        let x = e.x + dx;
                        let y = e.y + dy;
                        if x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h {
                            grid[y as usize][x as usize] = Cell::with_bg(' ', scale(socket, 1.0), scale(socket, 1.0));
                        }
                    }
                }
            }
            if open < 0.18 {
                for dx in -rx..=rx {
                    let ch = if eth { '·' } else if dx == -rx { '◜' } else if dx == rx { '◝' } else { '─' };
                    let cell = if eth { Cell::new(ch, ink(lid, 0.8)) } else { Cell::with_bg(ch, scale(lid, 1.0), scale(socket, 1.0)) };
                    let x = e.x + dx;
                    if x >= 0 && e.y >= 0 && (x as usize) < w && (e.y as usize) < h {
                        grid[e.y as usize][x as usize] = cell;
                    }
                }
                continue;
            }
            let ry_open = (ry as f32 * open).max(0.5);
            let vis = ry_open.round().max(1.0) as i32;
            let iris_r = (rx as f32 * 0.42).max(1.2);
            for dy in -vis..=vis {
                let fy = dy as f32 / (vis as f32 + 0.45);
                let hw = (rx as f32 * (1.0 - fy * fy).sqrt()).round() as i32;
                let top = dy == -vis;
                let bottom = dy == vis;
                for dx in -hw..=hw {
                    let x = e.x + dx;
                    let y = e.y + dy;
                    if x < 0 || y < 0 || x as usize >= w || y as usize >= h {
                        continue;
                    }
                    let xs = if ry == 2 { 0.55 } else { 0.85 };
                    let d = ((dx - gx) as f32 * xs).powi(2) + ((dy - gy) as f32).powi(2);
                    let in_iris = d < iris_r * iris_r;
                    let pupil = dx == gx && dy == gy;
                    let spec = ry == 2 && dx == gx - 1 && dy == gy - 1 && in_iris;
                    let edge = dx == -hw || dx == hw;
                    let ring = !in_iris && d < (iris_r + 0.8).powi(2);
                    if eth {
                        let b = 0.5 + 0.5 * k.glow * (0.5 + 0.5 * (ts * 1.3 + e.phase * 6.28).sin()) + flash * 0.4;
                        let ch = if top || bottom || edge {
                            '°'
                        } else if pupil {
                            '◉'
                        } else if ring {
                            '○'
                        } else {
                            ' '
                        };
                        if ch == ' ' {
                            grid[y as usize][x as usize] = Cell::new(' ', Color::Reset);
                        } else {
                            put(grid, x, y, ch, ink(c.eth_hi, b));
                        }
                    } else {
                        let lids = ry_open >= ry as f32 * 0.6;
                        let cell = if top && lids {
                            Cell::with_bg('▄', scale(SCLERA, daylight), scale(socket, 1.0))
                        } else if bottom && lids {
                            Cell::with_bg('▀', scale(SCLERA, daylight), scale(socket, 1.0))
                        } else if pupil {
                            Cell::with_bg('●', scale(PUPIL, 1.0), scale(iris, daylight))
                        } else if spec {
                            Cell::with_bg('°', scale((255, 255, 255), 1.0), scale(iris, daylight))
                        } else if in_iris {
                            Cell::with_bg('█', scale(iris, daylight), scale(SCLERA, daylight))
                        } else if edge {
                            Cell::with_bg(if dx < 0 { '(' } else { ')' }, scale(lid, 1.0), scale(SCLERA, daylight))
                        } else {
                            Cell::with_bg(' ', scale(SCLERA, daylight), scale(SCLERA, daylight))
                        };
                        grid[y as usize][x as usize] = cell;
                    }
                }
            }
        }
    }

    // surge burst: sparks fly from every tip on the ethereal half
    if burst_on {
        for (i, &(tx, ty)) in c.tips.iter().enumerate() {
            for j in 0..3 {
                let hsh = hash2(i as i32 * 3 + j, 77);
                let ang = (hsh % 628) as f32 / 100.0;
                let spd = 3.0 + ((hsh >> 8) % 6) as f32;
                let f = burst;
                let x = (tx as f32 + ang.cos() * spd * f).round() as i32;
                let y = (ty as f32 + ang.sin() * spd * f * 0.5 - f * 1.5).round() as i32;
                if !is_eth(x, y) {
                    continue;
                }
                let idx = ((f * 3.99) as usize).min(3);
                put(grid, x, y, SPARK[idx], ink(c.eth_hi, 1.0 - f * 0.6));
            }
        }
    }

    if k.motes > 0 {
        let span = c.ground as f32 + 2.0;
        for m in &c.motes {
            let life = (ts * m.rate * (0.35 + flash * 0.2) + m.phase).fract();
            let y = (c.ground as f32 + 1.0 - life * span).round() as i32;
            let x = (m.x0 + m.amp * (life * m.freq * 6.2832 + m.phase * 6.2832).sin()).round() as i32;
            if !is_eth(x, y) {
                continue;
            }
            let stage = ((life * 4.0) as usize).min(3);
            let b = (life * 3.1416).sin();
            put(grid, x, y, MOTE[[0, 1, 2, 1][stage]], ink(mix(c.eth_rgb, c.eth_hi, m.tint), 0.35 + 0.65 * b));
        }
        // living half: leaves detach from the canopy, gusts turn them into streaks, snow in winter
        let leaf_rate = 0.35 + 0.65 * sw[2] + 0.2 * sw[1] + gust;
        for m in &c.fallers {
            let snowing = snow > 0.2 && m.tint < snow;
            if !snowing && m.tint > leaf_rate {
                continue;
            }
            let rate = if snowing { m.rate * 0.6 } else { m.rate * (1.0 + gust * 2.0) };
            let life = (ts * rate * 0.3 + m.phase).fract();
            let (x, y) = if snowing {
                let fall_span = (c.ground as f32 - 2.0).max(1.0);
                ((m.x0 + m.amp * (life * m.freq * 6.2832).sin()).round() as i32, (2.0 + life * fall_span).round() as i32)
            } else {
                let drop = (c.ground as f32 - m.y0).max(1.0);
                let x = m.x0 + m.amp * (life * m.freq * 6.2832 + m.phase * 6.2832).sin() + (k.sway * 1.5 + gust * 18.0) * life;
                (x.round() as i32, (m.y0 + life * drop * (1.0 - gust * 0.5)).round() as i32)
            };
            if is_eth(x, y) || y >= c.ground as i32 || x < 0 || x as usize >= w {
                continue;
            }
            if snowing {
                put(grid, x, y, if m.phase > 0.5 { '*' } else { '·' }, scale((235, 240, 255), 0.7 + 0.3 * snow));
            } else {
                if k.eyes > 0 && sw[2] > 0.4 && m.tint < 0.12 {
                    let ch = if ((life * 9.0) as usize) & 1 == 0 { '◉' } else { '●' };
                    put(grid, x, y, ch, scale(mix(c.leaf_hi, c.eth_rgb, 0.35), 0.9));
                    continue;
                }
                let ch = if gust > 0.45 { if m.phase > 0.5 { '─' } else { '~' } } else { FALL[((life * m.freq * 12.0) as usize) & 3] };
                let base = mix(mix(c.leaf_rgb, c.leaf_hi, m.tint), c.autumn, sw[2] * 0.8);
                put(grid, x, y, ch, scale(base, 0.75 + 0.25 * (life * 6.28).sin().abs()));
            }
        }
    }

    // flocks over the living sky, wisps through the ethereal half
    for f in 0..k.flock {
        let ph = f as f32 * 2.1;
        let fx = c.cx as f32 + w as f32 * 0.42 * (ts * 0.13 + ph).sin();
        let fy = 2.0 + gf * 0.42 * (0.5 + 0.5 * (ts * 0.21 + ph * 1.7).sin());
        let heading = (ts * 0.13 + ph).cos().signum();
        for b in 0..7 {
            let bx = (fx + heading * (b as f32 - 3.0) * 2.2 + (ts * 0.9 + b as f32).sin()).round() as i32;
            let by = (fy + ((b as f32 - 3.0).abs() * 0.7) + 0.6 * (ts * 1.7 + b as f32 * 1.3).sin()).round() as i32;
            if by >= gy || is_eth(bx, by) {
                continue;
            }
            let flap = (ts * 6.0 + b as f32 * 0.9 + ph).sin() > 0.0;
            put(grid, bx, by, BIRD[flap as usize], scale((30, 30, 40), 1.0 + day * 0.5 + 0.8 * (1.0 - day)));
        }
        // one wisp per flock slot, a 6-glyph trail on a lissajous path
        for tr in 0..6 {
            let tt = ts - tr as f32 * 0.12;
            let wx = (c.cx as f32 + w as f32 * 0.45 * (tt * 0.17 + ph).sin()).round() as i32;
            let wy = (gf * 0.5 + gf * 0.45 * (tt * 0.29 + ph * 0.6).cos()).round() as i32;
            if !is_eth(wx, wy) {
                continue;
            }
            put(grid, wx, wy, WISP[(tr / 2).min(3)], ink(c.eth_hi, 1.0 - tr as f32 * 0.14));
        }
    }

    // the veil itself: a phrase scrolling down the seam over empty cells
    let n = SEAM_TEXT.len() as i32;
    let scroll = (ts * 2.0) as i32;
    for y in 0..hh {
        let x = veil[y] as usize;
        if x >= w {
            continue;
        }
        let cur = grid[y][x].ch;
        if cur != ' ' && cur != '·' && cur != '─' {
            continue;
        }
        let idx = ((y as i32 - scroll).rem_euclid(n)) as usize;
        let byte = SEAM_TEXT[idx];
        let ch = if byte == 0xb7 { '·' } else { byte as char };
        let p = (y as f32 * 0.55 - ts * 2.6).sin();
        let b = 0.35 + 0.45 * k.glow * (p * 0.5 + 0.5) + flash * 0.4;
        put(grid, x as i32, y as i32, if k.flair > 0.0 { ch } else { '┆' }, scale(c.eth_hi, b));
    }
}

pub(crate) fn cli_lifetree3(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
    let _ = (rng, term_w, term_h, mode, theme_name);
    let mut k = EyeKnobs::from_env();
    let f = |i: usize| args.get(i).and_then(|s| s.parse::<f32>().ok());
    if let Some(v) = f(4) {
        k.depth = v.round().clamp(4.0, 11.0) as u32;
    }
    if let Some(v) = f(5) {
        k.sway = v.clamp(0.0, 6.0);
    }
    if let Some(v) = f(6) {
        k.speed = v.clamp(0.05, 4.0);
    }
    if let Some(v) = f(7) {
        k.motes = v.round().clamp(0.0, 300.0) as usize;
    }
    if let Some(v) = f(8) {
        k.seam = v.clamp(0.1, 0.9);
    }
    if let Some(v) = f(9) {
        k.veil = v.clamp(0.0, 14.0);
    }
    if let Some(v) = f(10) {
        k.season = v.clamp(0.0, 1.0);
    }
    if let Some(v) = f(11) {
        k.tide = v.clamp(0.0, 1.0);
    }
    if let Some(v) = f(12) {
        k.gust = v.clamp(0.0, 1.0);
    }
    if let Some(v) = f(13) {
        k.flair = v.clamp(0.0, 1.0);
    }
    if let Some(v) = f(14) {
        k.eyes = v.round().clamp(0.0, 80.0) as usize;
    }
    draw_lifetree3(&mut grid, width, height, seed, &palette, t_anim, &k);
    (grid, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(w: usize, h: usize, seed: u64, t: f32) -> String {
        let mut g = vec![vec![Cell::blank(); w]; h];
        let p = crate::color::named_theme("moss").unwrap();
        let k = EyeKnobs::from_env();
        draw_lifetree3(&mut g, w, h, seed, &p, t, &k);
        g.iter()
            .map(|row| row.iter().map(|c| c.ch).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn snapshot_lifetree3_small() {
        insta::assert_snapshot!("lifetree3_80x24", run(80, 24, 42, 0.0));
    }

    #[test]
    fn snapshot_lifetree3_wide() {
        insta::assert_snapshot!("lifetree3_120x40", run(120, 40, 42, 0.0));
    }

    #[test]
    fn deterministic_and_seed_sensitive() {
        assert_eq!(run(90, 30, 42, 0.0), run(90, 30, 42, 0.0));
        assert_ne!(run(90, 30, 42, 0.0), run(90, 30, 7, 0.0));
    }

    #[test]
    fn t_moves_frame_and_veil() {
        let a = run(90, 30, 42, 0.0);
        let b = run(90, 30, 42, 2.0);
        assert_ne!(a, b);
        let k = EyeKnobs::from_env();
        let v0: Vec<i32> = (0..30).map(|y| veil_x(y, 90, 0.0, &k)).collect();
        let v1: Vec<i32> = (0..30).map(|y| veil_x(y, 90, 2.0, &k)).collect();
        assert_ne!(v0, v1);
        assert!(v0.iter().any(|&x| x != v0[0]), "veil is a wave, never a straight column");
    }

    #[test]
    fn seasons_cycle_leaf_visibility() {
        let mut k = EyeKnobs::from_env();
        k.season = 1.0;
        k.ring = 0.0;
        let count = |t: f32| {
            let mut g = vec![vec![Cell::blank(); 100]; 32];
            let p = crate::color::named_theme("moss").unwrap();
            draw_lifetree3(&mut g, 100, 32, 42, &p, t, &k);
            g.iter().flatten().filter(|c| matches!(c.ch, '♠' | '♣')).count()
        };
        let summer = count(1.0);
        let winter = count(3.0);
        assert!(winter * 3 < summer, "winter {} should shed most of summer {}", winter, summer);
    }

    #[test]
    fn ring_and_both_halves_present() {
        let s = run(100, 32, 42, 0.0);
        assert!(s.contains('░') || s.contains('▒'), "ethereal fill");
        assert!(s.contains('♠') || s.contains('♣'), "living leaves");
        assert!(s.contains('✦') || s.contains('✧') || s.contains('○'), "rune ring");
        assert!(s.contains('~'), "vine ring");
    }

    #[test]
    fn eyes_present_and_blink() {
        let s0 = run(100, 32, 42, 0.0);
        assert!(s0.contains('●') || s0.contains('◉'), "eye pupils");
        let mut k = EyeKnobs::from_env();
        k.blink = 4.0;
        let count = |t: f32| {
            let mut g = vec![vec![Cell::blank(); 100]; 32];
            let p = crate::color::named_theme("moss").unwrap();
            draw_lifetree3(&mut g, 100, 32, 42, &p, t, &k);
            g.iter().flatten().filter(|c| c.ch == '●').count()
        };
        let a: Vec<usize> = (0..40).map(|i| count(i as f32 * 0.05)).collect();
        assert!(a.iter().any(|&n| n != a[0]), "pupil count must change across a blink window: {:?}", a);
    }

    #[test]
    fn no_eyes_knob_removes_eyes() {
        let mut k = EyeKnobs::from_env();
        k.eyes = 0;
        k.day = 0.0;
        let mut g = vec![vec![Cell::blank(); 100]; 32];
        let p = crate::color::named_theme("moss").unwrap();
        draw_lifetree3(&mut g, 100, 32, 42, &p, 0.0, &k);
        assert_eq!(g.iter().flatten().filter(|c| c.ch == '●').count(), 0);
    }

    #[test]
    fn frame_cost_is_flat() {
        let mut g = vec![vec![Cell::blank(); 200]; 60];
        let p = crate::color::named_theme("ember").unwrap();
        let k = EyeKnobs::from_env();
        draw_lifetree3(&mut g, 200, 60, 42, &p, 0.0, &k);
        let start = std::time::Instant::now();
        for i in 1..=200 {
            draw_lifetree3(&mut g, 200, 60, 42, &p, i as f32 * 0.06, &k);
        }
        let per = start.elapsed().as_secs_f64() / 200.0;
        eprintln!("lifetree3 frame 200x60: {:.3}ms", per * 1000.0);
        assert!(per < 0.004, "frame {:.3}ms exceeds 4ms budget at 200x60", per * 1000.0);
    }
}
