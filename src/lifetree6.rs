//! tree-of-life-6 -- hyperbolic tree in the upper half-plane / horocycle projection
//! with SL(2,R) parabolic/hyperbolic flows, {5,4} hyperbolic lattice, and dual ethereal/living halves.
use crate::_0_profile::measure_layer;
use crate::color::*;
use crate::opts::param_f32;
use crate::types::*;
use crossterm::style::Color;
use rand::RngExt;
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::cell::RefCell;

type HPoint = (f32, f32);

#[inline]
fn sl2_act(m: [f32; 4], z: HPoint) -> HPoint {
    let (x, y) = z;
    let denom = (m[2] * x + m[3]) * (m[2] * x + m[3]) + (m[2] * y) * (m[2] * y);
    let d = denom.max(1e-7);
    let num_x = (m[0] * x + m[1]) * (m[2] * x + m[3]) + m[0] * m[2] * y * y;
    let num_y = (m[0] * m[3] - m[1] * m[2]) * y;
    (num_x / d, (num_y / d).max(1e-4))
}

#[inline]
fn h_dist(z1: HPoint, z2: HPoint) -> f32 {
    let dx = z1.0 - z2.0;
    let dy = z1.1 - z2.1;
    let num = dx * dx + dy * dy;
    let denom = 2.0 * z1.1 * z2.1;
    let delta = (num / denom.max(1e-7)).max(0.0);
    (1.0 + delta + ((1.0 + delta) * (1.0 + delta) - 1.0).max(0.0).sqrt()).ln()
}

#[inline]
fn sample_geodesic(z1: HPoint, z2: HPoint, n: usize) -> Vec<HPoint> {
    let mut pts = Vec::with_capacity(n);
    if (z1.0 - z2.0).abs() < 1e-4 {
        for i in 0..n {
            let frac = i as f32 / (n - 1).max(1) as f32;
            let log_y = z1.1.ln() + frac * (z2.1.ln() - z1.1.ln());
            pts.push((z1.0, log_y.exp()));
        }
        return pts;
    }
    let c = (z2.0 * z2.0 + z2.1 * z2.1 - (z1.0 * z1.0 + z1.1 * z1.1)) / (2.0 * (z2.0 - z1.0));
    let r = ((z1.0 - c) * (z1.0 - c) + z1.1 * z1.1).sqrt();
    let ang1 = z1.1.atan2(z1.0 - c);
    let ang2 = z2.1.atan2(z2.0 - c);
    for i in 0..n {
        let frac = i as f32 / (n - 1).max(1) as f32;
        let ang = ang1 + frac * (ang2 - ang1);
        pts.push((c + r * ang.cos(), (r * ang.sin()).max(1e-4)));
    }
    pts
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) struct Knobs6 {
    pub depth: u32,
    pub spread: f32,
    pub zoom: f32,
    pub flow: f32,
    pub warp: f32,
    pub speed: f32,
    pub motes: usize,
    pub glow: f32,
    pub lattice: f32,
    pub seam: f32,
}

impl Knobs6 {
    pub(crate) fn from_env() -> Self {
        Knobs6 {
            depth: param_f32("DEPTH", 7.0).round().clamp(3.0, 10.0) as u32,
            spread: param_f32("SPREAD", 0.72).clamp(0.2, 1.4),
            zoom: param_f32("ZOOM", 0.45).clamp(0.0, 2.0),
            flow: param_f32("FLOW", 0.35).clamp(-1.5, 1.5),
            warp: param_f32("WARP", 0.25).clamp(0.0, 1.0),
            speed: param_f32("SPEED", 1.0).clamp(0.05, 4.0),
            motes: param_f32("MOTES", 50.0).round().clamp(0.0, 300.0) as usize,
            glow: param_f32("GLOW", 0.85).clamp(0.0, 1.0),
            lattice: param_f32("LATTICE", 1.0).clamp(0.0, 1.0),
            seam: param_f32("SEAM", 0.08).clamp(-0.6, 0.6),
        }
    }
    fn geom_key(&self) -> (u32, u32, u32, usize) {
        (self.depth, self.spread.to_bits(), self.lattice.to_bits(), self.motes)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SegmentKind {
    Trunk,
    MainBranch,
    Twig,
    HorocycleRoot,
}

const SEG_SAMPLES: usize = 7;

#[derive(Clone)]
struct Segment6 {
    pts: [HPoint; SEG_SAMPLES],
    kind: SegmentKind,
    depth: u32,
    phase: f32,
    length: f32,
}

#[derive(Clone, Copy)]
struct CanopyNode6 {
    z: HPoint,
    phase: f32,
    glyph_idx: u8,
}

#[derive(Clone, Copy)]
struct Mote6 {
    base_x: f32,
    base_y: f32,
    freq: f32,
    phase: f32,
    shade: f32,
}

#[derive(Clone)]
struct LatticeArc6 {
    pts: [HPoint; 12],
}

struct Cached6 {
    key: (usize, usize, u64, (u32, u32, u32, usize)),
    segments: Vec<Segment6>,
    canopy: Vec<CanopyNode6>,
    lattice: Vec<LatticeArc6>,
    motes: Vec<Mote6>,
    c_x: f32,
    c_y: f32,
    scale_x: f32,
    scale_y: f32,
    col_eth_core: (u8, u8, u8),
    col_eth_glow: (u8, u8, u8),
    col_eth_deep: (u8, u8, u8),
    col_live_dark: (u8, u8, u8),
    col_bark: (u8, u8, u8),
    col_bark_bright: (u8, u8, u8),
    col_leaf: (u8, u8, u8),
    col_leaf_accent: (u8, u8, u8),
}

thread_local! {
    static CACHE: RefCell<Option<Cached6>> = const { RefCell::new(None) };
}

fn rgb_tuple(c: Color) -> (u8, u8, u8) {
    match c {
        Color::Rgb { r, g, b } => (r, g, b),
        _ => (128, 128, 128),
    }
}

fn color_hue(c: Color) -> f64 {
    let (r, g, b) = rgb_tuple(c);
    let (rf, gf, bf) = (r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0);
    let max = rf.max(gf).max(bf);
    let min = rf.min(gf).min(bf);
    let diff = max - min;
    if diff < 1e-6 {
        return 0.0;
    }
    let h = if max == rf {
        60.0 * (((gf - bf) / diff) % 6.0)
    } else if max == gf {
        60.0 * ((bf - rf) / diff + 2.0)
    } else {
        60.0 * ((rf - gf) / diff + 4.0)
    };
    h.rem_euclid(360.0)
}

#[inline]
fn scale_rgb(c: (u8, u8, u8), factor: f32) -> Color {
    let conv = |v: u8| ((v as f32 * factor).round().clamp(0.0, 255.0)) as u8;
    Color::Rgb { r: conv(c.0), g: conv(c.1), b: conv(c.2) }
}

#[inline]
fn blend_rgb(c1: (u8, u8, u8), c2: (u8, u8, u8), ratio: f32) -> (u8, u8, u8) {
    let t = ratio.clamp(0.0, 1.0);
    let calc = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t).round() as u8;
    (calc(c1.0, c2.0), calc(c1.1, c2.1), calc(c1.2, c2.2))
}

#[inline]
fn set_cell(grid: &mut Grid, x: i32, y: i32, ch: char, fg: Color) {
    if x >= 0 && y >= 0 && (y as usize) < grid.len() && (x as usize) < grid[0].len() {
        let cell = &mut grid[y as usize][x as usize];
        cell.ch = ch;
        cell.fg = fg;
    }
}

#[inline]
fn h_step(start: HPoint, heading_rad: f32, dist: f32) -> (HPoint, f32) {
    let cos_t = heading_rad.cos();
    let sin_t = heading_rad.sin();
    if cos_t.abs() < 1e-4 {
        let next_y = if sin_t > 0.0 { start.1 * dist.exp() } else { start.1 * (-dist).exp() };
        return ((start.0, next_y), heading_rad);
    }
    let c = start.0 - start.1 * sin_t / cos_t;
    let r = start.1 / cos_t.abs();
    let ang0 = start.1.atan2(start.0 - c);
    let arc_sgn = if cos_t > 0.0 { -1.0 } else { 1.0 };
    let delta_ang = arc_sgn * 2.0 * (0.5 * dist).tanh();
    let ang_next = (ang0 + delta_ang).clamp(0.02, std::f32::consts::PI - 0.02);
    let end = (c + r * ang_next.cos(), (r * ang_next.sin()).max(1e-4));
    let next_heading = if arc_sgn < 0.0 {
        ang_next + std::f32::consts::FRAC_PI_2
    } else {
        ang_next - std::f32::consts::FRAC_PI_2
    };
    (end, next_heading)
}

fn grow_tree6(
    rng: &mut StdRng,
    segs: &mut Vec<Segment6>,
    canopy: &mut Vec<CanopyNode6>,
    start: HPoint,
    heading: f32,
    h_len: f32,
    depth: u32,
    max_depth: u32,
    spread: f32,
    is_root: bool,
) {
    if depth > max_depth || h_len < 0.06 || start.1 < 0.02 || start.1 > 45.0 {
        return;
    }
    let (end, end_heading) = h_step(start, heading, h_len);
    let raw_pts = sample_geodesic(start, end, SEG_SAMPLES);
    let mut pts_arr = [(0.0f32, 0.0f32); SEG_SAMPLES];
    for (i, p) in raw_pts.into_iter().take(SEG_SAMPLES).enumerate() {
        pts_arr[i] = p;
    }
    let kind = if is_root {
        SegmentKind::HorocycleRoot
    } else if depth <= 1 {
        SegmentKind::Trunk
    } else if depth + 2 >= max_depth {
        SegmentKind::Twig
    } else {
        SegmentKind::MainBranch
    };
    segs.push(Segment6 {
        pts: pts_arr,
        kind,
        depth,
        phase: rng.random::<f32>(),
        length: h_len,
    });
    if depth == max_depth && !is_root {
        for _ in 0..4 {
            let offset_ang = rng.random::<f32>() * 6.28318;
            let offset_r = 0.12 + rng.random::<f32>() * 0.35;
            let leaf_z = (
                end.0 + offset_r * end.1 * offset_ang.cos(),
                (end.1 * (1.0 + offset_r * offset_ang.sin())).max(1e-3),
            );
            canopy.push(CanopyNode6 {
                z: leaf_z,
                phase: rng.random::<f32>(),
                glyph_idx: (rng.random_range(0..4u32)) as u8,
            });
        }
        return;
    }
    let branch_count = if depth == 0 {
        2
    } else if depth <= 2 && rng.random::<f32>() < 0.4 {
        3
    } else {
        2
    };
    for b in 0..branch_count {
        let side = if branch_count == 2 {
            if b == 0 { -1.0 } else { 1.0 }
        } else {
            b as f32 - 1.0
        };
        let branch_spread = spread * (0.65 + rng.random::<f32>() * 0.7);
        let jitter = (rng.random::<f32>() - 0.5) * 0.25;
        let next_head = end_heading + side * branch_spread + jitter;
        let next_len = h_len * (0.78 + rng.random::<f32>() * 0.15);
        grow_tree6(
            rng,
            segs,
            canopy,
            end,
            next_head,
            next_len,
            depth + 1,
            max_depth,
            spread,
            is_root,
        );
    }
}

fn build_cached6(w: usize, h: usize, seed: u64, palette: &[Color; 5], k: &Knobs6) -> Cached6 {
    let mut rng = StdRng::seed_from_u64(seed ^ 0x6A09_E667_BB67_AE85);
    let scale_x = (w as f32 * 0.48).max(5.0);
    let scale_y = (h as f32 * 0.88).max(5.0);
    let c_x = w as f32 * 0.5;
    let c_y = h as f32 * 0.94;

    let eth_h = (color_hue(palette[3]) + 160.0).rem_euclid(360.0);
    let col_eth_core = rgb_tuple(hsl_to_rgb(eth_h, 0.65, 0.70));
    let col_eth_glow = rgb_tuple(hsl_to_rgb(eth_h, 0.40, 0.94));
    let col_eth_deep = rgb_tuple(hsl_to_rgb(eth_h, 0.55, 0.10));
    let col_live_dark = blend_rgb(rgb_tuple(palette[0]), rgb_tuple(palette[1]), 0.14);
    let col_bark = rgb_tuple(darken(palette[2], 18));
    let col_bark_bright = rgb_tuple(lighten(palette[2], 26));
    let col_leaf = rgb_tuple(palette[1]);
    let col_leaf_accent = rgb_tuple(palette[3]);

    let mut segments = Vec::new();
    let mut canopy = Vec::new();
    let root_origin: HPoint = (0.0, 1.0);
    let up_angle = std::f32::consts::FRAC_PI_2;
    grow_tree6(
        &mut rng,
        &mut segments,
        &mut canopy,
        root_origin,
        up_angle,
        0.85,
        0,
        k.depth,
        k.spread,
        false,
    );

    let root_depth = k.depth.saturating_sub(3).max(2);
    for i in 0..4 {
        let x_off = -1.5 + 1.0 * i as f32;
        let mut horo_pts = [(0.0f32, 0.0f32); SEG_SAMPLES];
        for s in 0..SEG_SAMPLES {
            let frac = s as f32 / (SEG_SAMPLES - 1) as f32;
            let x = x_off + frac * 0.8;
            let y = (0.85 - 0.4 * (frac * 3.14159).sin()).max(0.1);
            horo_pts[s] = (x, y);
        }
        segments.push(Segment6 {
            pts: horo_pts,
            kind: SegmentKind::HorocycleRoot,
            depth: 0,
            phase: rng.random::<f32>(),
            length: 0.7,
        });
    }
    grow_tree6(
        &mut rng,
        &mut segments,
        &mut canopy,
        root_origin,
        -up_angle,
        0.55,
        0,
        root_depth,
        k.spread * 1.25,
        true,
    );

    let mut lattice = Vec::new();
    if k.lattice > 0.0 {
        for idx in -5..=5 {
            let cx = idx as f32 * 1.6;
            for r_step in 1..=4 {
                let r = r_step as f32 * 0.9;
                let mut arc = [(0.0f32, 0.0f32); 12];
                for step in 0..12 {
                    let ang = 0.08 + (step as f32 / 11.0) * (std::f32::consts::PI - 0.16);
                    arc[step] = (cx + r * ang.cos(), (r * ang.sin()).max(1e-3));
                }
                lattice.push(LatticeArc6 { pts: arc });
            }
        }
        for idx in -6..=6 {
            let x = idx as f32 * 1.2;
            let mut arc = [(0.0f32, 0.0f32); 12];
            for step in 0..12 {
                let frac = step as f32 / 11.0;
                let y = 0.2 * (2.8f32).powf(frac * 3.0);
                arc[step] = (x, y);
            }
            lattice.push(LatticeArc6 { pts: arc });
        }
    }

    let mut motes = Vec::with_capacity(k.motes);
    for _ in 0..k.motes {
        motes.push(Mote6 {
            base_x: (rng.random::<f32>() - 0.5) * 5.0,
            base_y: 0.2 + rng.random::<f32>() * 3.8,
            freq: 0.15 + rng.random::<f32>() * 0.35,
            phase: rng.random::<f32>(),
            shade: rng.random::<f32>(),
        });
    }

    Cached6 {
        key: (w, h, seed, k.geom_key()),
        segments,
        canopy,
        lattice,
        motes,
        c_x,
        c_y,
        scale_x,
        scale_y,
        col_eth_core,
        col_eth_glow,
        col_eth_deep,
        col_live_dark,
        col_bark,
        col_bark_bright,
        col_leaf,
        col_leaf_accent,
    }
}

const ETH_BLOCKS: [char; 4] = ['░', '▒', '▓', '█'];
const CANOPY_PAIRS: [[char; 2]; 4] = [['♣', '♠'], ['♠', '♣'], ['*', '♣'], ['♠', '*']];
const MOTE_CHARS: [char; 4] = ['·', '∙', '°', '○'];

#[inline]
fn select_branch_char(dx: i32, dy: i32, heavy: bool) -> char {
    let ax = dx.abs();
    let ay = dy.abs() * 2;
    if ax * 3 < ay {
        if heavy { '║' } else { '│' }
    } else if ay * 3 < ax {
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
fn project_to_screen(c: &Cached6, z: HPoint) -> (i32, i32) {
    let x_norm = z.0 / z.1.sqrt();
    let y_norm = (z.1.ln() + 2.5) / 5.5;
    let px = (c.c_x + x_norm * c.scale_x * 0.45).round() as i32;
    let py = (c.c_y - y_norm * c.scale_y).round() as i32;
    (px, py)
}

#[inline]
fn unproject_from_screen(c: &Cached6, px: i32, py: i32) -> HPoint {
    let y_norm = ((c.c_y - py as f32) / c.scale_y).clamp(0.01, 1.1);
    let y = ((y_norm * 5.5) - 2.5).exp();
    let x_norm = (px as f32 - c.c_x) / (c.scale_x * 0.45).max(1.0);
    let x = x_norm * y.sqrt();
    (x, y.max(1e-4))
}

#[inline]
fn raster_line(x0: i32, y0: i32, x1: i32, y1: i32, mut plot: impl FnMut(i32, i32)) {
    let steps = (x1 - x0).abs().max((y1 - y0).abs()).max(1);
    for i in 0..=steps {
        let f = i as f32 / steps as f32;
        let x = (x0 as f32 + (x1 - x0) as f32 * f).round() as i32;
        let y = (y0 as f32 + (y1 - y0) as f32 * f).round() as i32;
        plot(x, y);
    }
}

pub(crate) fn draw_lifetree6(
    grid: &mut Grid,
    w: usize,
    h: usize,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    k: &Knobs6,
) {
    if w == 0 || h == 0 {
        return;
    }
    CACHE.with(|slot| {
        let mut slot = slot.borrow_mut();
        let key = (w, h, seed, k.geom_key());
        if slot.as_ref().map(|c| c.key != key).unwrap_or(true) {
            *slot = Some(build_cached6(w, h, seed, palette, k));
        }
        let c = slot.as_ref().unwrap();
        render_frame6(grid, c, t, k);
    });
}

#[inline]
fn render_frame6(grid: &mut Grid, c: &Cached6, t: f32, k: &Knobs6) {
    let w = grid[0].len();
    let h = grid.len();
    let ts = t * k.speed;

    let dilation = (ts * 0.22 * k.zoom).sin() * 0.4;
    let boost = (dilation * 0.5).exp();
    let horo_flow = (ts * 0.35 * k.flow).sin() * 0.6;
    let sl2_mat = [boost, horo_flow, 0.0, 1.0 / boost];

    let seam_center = (ts * k.seam * 0.8).sin() * 1.5;
    let seam_wave = ts * 0.6;
    let is_ethereal = |z: HPoint| -> bool {
        let curve_x = seam_center + 0.3 * (z.1.ln() * 1.4 + seam_wave).sin();
        z.0 < curve_x
    };

    let beat = (ts * 0.7).rem_euclid(3.0);
    let flash = (1.0 - beat * 4.5).max(0.0) * k.glow;

    measure_layer("tree-of-life-6", "plane", || {
        for y in 0..h {
            for x in 0..w {
                let z_raw = unproject_from_screen(c, x as i32, y as i32);
                let z = sl2_act([1.0 / boost, -horo_flow, 0.0, boost], z_raw);
                let eth = is_ethereal(z);
                let v_ratio = (y as f32 / h as f32).clamp(0.0, 1.0);
                let base_col = if eth { c.col_eth_deep } else { c.col_live_dark };
                let lit = if eth {
                    0.22 + 0.65 * (1.0 - v_ratio) + flash * 0.45
                } else {
                    0.20 + 0.60 * (1.0 - v_ratio)
                };
                let col = scale_rgb(base_col, lit);
                grid[y][x] = Cell::with_bg(' ', col, col);
            }
        }
    });

    measure_layer("tree-of-life-6", "lattice", || {
        if k.lattice > 0.0 {
            for arc in &c.lattice {
                let mut prev: Option<(i32, i32)> = None;
                for &pt in arc.pts.iter() {
                    let z_trans = sl2_act(sl2_mat, pt);
                    let (px, py) = project_to_screen(c, z_trans);
                    if let Some((x0, y0)) = prev {
                        let eth = is_ethereal(pt);
                        let col = if eth {
                            scale_rgb(c.col_eth_core, 0.28 * k.lattice)
                        } else {
                            scale_rgb(c.col_bark, 0.45 * k.lattice)
                        };
                        raster_line(x0, y0, px, py, |lx, ly| {
                            if lx >= 0 && ly >= 0 && (lx as usize) < w && (ly as usize) < h {
                                if grid[ly as usize][lx as usize].ch == ' ' {
                                    grid[ly as usize][lx as usize].ch = '·';
                                    grid[ly as usize][lx as usize].fg = col;
                                }
                            }
                        });
                    }
                    prev = Some((px, py));
                }
            }
        }
    });

    measure_layer("tree-of-life-6", "branches", || {
        for seg in &c.segments {
            let eth = is_ethereal(seg.pts[0]);
            let mut screen_pts = [(0i32, 0i32); SEG_SAMPLES];
            for i in 0..SEG_SAMPLES {
                let z_trans = sl2_act(sl2_mat, seg.pts[i]);
                screen_pts[i] = project_to_screen(c, z_trans);
            }
            let heavy = matches!(seg.kind, SegmentKind::Trunk);
            for i in 0..SEG_SAMPLES - 1 {
                let (x0, y0) = screen_pts[i];
                let (x1, y1) = screen_pts[i + 1];
                if x0 < -10 && x1 < -10 || x0 >= w as i32 + 10 && x1 >= w as i32 + 10 {
                    continue;
                }
                let depth_ratio = 1.0 - (seg.depth as f32 / (k.depth as f32).max(1.0));
                let thick = if heavy && depth_ratio > 0.6 { 1 } else { 0 };

                if eth {
                    let p = (seg.depth as f32 * 1.5 - ts * 1.8 + seg.phase * 6.28).sin().max(0.0);
                    let pulse = p * p;
                    let b_idx = if pulse > 0.6 { 2 } else if pulse > 0.2 { 1 } else { 0 };
                    let ch = match seg.kind {
                        SegmentKind::Twig => if b_idx > 0 { '∙' } else { '·' },
                        _ => ETH_BLOCKS[b_idx],
                    };
                    let intensity = (0.45 + 0.55 * k.glow * pulse.max(flash)) * (0.5 + 0.5 * depth_ratio);
                    let col = scale_rgb(blend_rgb(c.col_eth_core, c.col_eth_glow, pulse * 0.75), intensity);
                    raster_line(x0, y0, x1, y1, |lx, ly| {
                        for off in -thick..=thick {
                            set_cell(grid, lx + off, ly, ch, col);
                        }
                    });
                } else {
                    let ch = match seg.kind {
                        SegmentKind::Twig => '·',
                        _ => select_branch_char(x1 - x0, y1 - y0, heavy),
                    };
                    let base = match seg.kind {
                        SegmentKind::HorocycleRoot => blend_rgb(c.col_bark, (0, 0, 0), 0.4),
                        SegmentKind::Twig => blend_rgb(c.col_bark_bright, c.col_leaf, 0.45),
                        _ => blend_rgb(c.col_bark, c.col_bark_bright, depth_ratio * 0.5),
                    };
                    let sway = 0.88 + 0.12 * (ts * 0.7 + seg.depth as f32 * 0.8).sin();
                    let col = scale_rgb(base, (0.55 + 0.45 * depth_ratio) * sway);
                    raster_line(x0, y0, x1, y1, |lx, ly| {
                        for off in -thick..=thick {
                            set_cell(grid, lx + off, ly, ch, col);
                        }
                    });
                }
            }
        }
    });

    measure_layer("tree-of-life-6", "canopy", || {
        for leaf in &c.canopy {
            let z_trans = sl2_act(sl2_mat, leaf.z);
            let (px, py) = project_to_screen(c, z_trans);
            let eth = is_ethereal(leaf.z);
            if eth {
                let p = (ts * 1.5 + leaf.phase * 6.28).sin();
                let ch = if p > 0.5 { '○' } else { '°' };
                let col = scale_rgb(c.col_eth_glow, 0.5 + 0.45 * p.max(0.0));
                set_cell(grid, px, py, ch, col);
            } else {
                let r = (ts * 2.5 + leaf.phase * 6.28).sin();
                let pair = CANOPY_PAIRS[(leaf.glyph_idx & 3) as usize];
                let ch = if r > 0.55 { pair[1] } else { pair[0] };
                let col = scale_rgb(
                    blend_rgb(c.col_leaf, c.col_leaf_accent, leaf.phase * 0.6 + r.max(0.0) * 0.3),
                    0.8 + 0.2 * r,
                );
                set_cell(grid, px, py, ch, col);
            }
        }
    });

    measure_layer("tree-of-life-6", "motes", || {
        for mote in &c.motes {
            let cycle = (ts * mote.freq * 0.5 + mote.phase).fract();
            let curr_y = mote.base_y * (1.0 + 1.2 * cycle);
            let curr_x = mote.base_x + 0.3 * (cycle * 6.28 + mote.phase).sin();
            let z_pt = (curr_x, curr_y);
            if !is_ethereal(z_pt) {
                continue;
            }
            let z_trans = sl2_act(sl2_mat, z_pt);
            let (px, py) = project_to_screen(c, z_trans);
            let stage = ((cycle * 4.0) as usize).min(3);
            let ch = MOTE_CHARS[[0, 1, 2, 1][stage]];
            let b = (cycle * 3.14159).sin();
            let col = scale_rgb(
                blend_rgb(c.col_eth_core, c.col_eth_glow, mote.shade),
                0.35 + 0.65 * b,
            );
            set_cell(grid, px, py, ch, col);
        }
    });

    measure_layer("tree-of-life-6", "flow", || {
        for step in 0..50 {
            let frac = step as f32 / 49.0;
            let y_val = 0.15 * (4.0f32).powf(frac * 3.2);
            let curve_x = seam_center + 0.3 * (y_val.ln() * 1.4 + seam_wave).sin();
            let z_trans = sl2_act(sl2_mat, (curve_x, y_val));
            let (px, py) = project_to_screen(c, z_trans);
            if px >= 0 && py >= 0 && (px as usize) < w && (py as usize) < h {
                if grid[py as usize][px as usize].ch == ' ' {
                    let pulse = (frac * 10.0 - ts * 2.2).sin();
                    if pulse > 0.1 {
                        set_cell(grid, px, py, '┆', scale_rgb(c.col_eth_glow, 0.4 + 0.5 * k.glow * pulse));
                    }
                }
            }
        }
    });
}

pub(crate) fn cli_lifetree6(
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
    let mut k = Knobs6::from_env();
    let parse_arg = |idx: usize| args.get(idx).and_then(|s| s.parse::<f32>().ok());
    if let Some(v) = parse_arg(4) {
        k.depth = v.round().clamp(3.0, 10.0) as u32;
    }
    if let Some(v) = parse_arg(5) {
        k.zoom = v.clamp(0.0, 2.0);
    }
    if let Some(v) = parse_arg(6) {
        k.flow = v.clamp(-1.5, 1.5);
    }
    if let Some(v) = parse_arg(7) {
        k.spread = v.clamp(0.2, 1.4);
    }
    if let Some(v) = parse_arg(8) {
        k.motes = v.round().clamp(0.0, 300.0) as usize;
    }
    draw_lifetree6(&mut grid, width, height, seed, &palette, t_anim, &k);
    (grid, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_test(w: usize, h: usize, seed: u64, t: f32) -> String {
        let mut g = vec![vec![Cell::blank(); w]; h];
        let p = crate::color::named_theme("moss").unwrap();
        let k = Knobs6::from_env();
        draw_lifetree6(&mut g, w, h, seed, &p, t, &k);
        g.iter()
            .map(|row| row.iter().map(|c| c.ch).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn snapshot_lifetree6_small() {
        insta::assert_snapshot!("lifetree6_80x24", run_test(80, 24, 42, 0.0));
    }

    #[test]
    fn snapshot_lifetree6_wide() {
        insta::assert_snapshot!("lifetree6_120x40", run_test(120, 40, 42, 0.0));
    }

    #[test]
    fn deterministic_and_seed_sensitive() {
        assert_eq!(run_test(90, 30, 42, 0.0), run_test(90, 30, 42, 0.0));
        assert_ne!(run_test(90, 30, 42, 0.0), run_test(90, 30, 7, 0.0));
    }

    #[test]
    fn hyperbolic_flow_moves_tree() {
        assert_ne!(run_test(90, 30, 42, 0.0), run_test(90, 30, 42, 3.0));
    }

    #[test]
    fn both_halves_rendered() {
        let s = run_test(100, 32, 42, 0.0);
        assert!(s.contains('░') || s.contains('▒') || s.contains('▓'), "ethereal half missing");
        assert!(s.contains('║') || s.contains('│') || s.contains('╱') || s.contains('╲'), "living bark missing");
    }

    #[test]
    fn frame_perf_under_6ms() {
        let mut g = vec![vec![Cell::blank(); 200]; 60];
        let p = crate::color::named_theme("ember").unwrap();
        let k = Knobs6::from_env();
        draw_lifetree6(&mut g, 200, 60, 42, &p, 0.0, &k);
        let start = std::time::Instant::now();
        for i in 1..=200 {
            draw_lifetree6(&mut g, 200, 60, 42, &p, i as f32 * 0.06, &k);
        }
        let per = start.elapsed().as_secs_f64() / 200.0;
        eprintln!("lifetree6 frame 200x60: {:.3}ms", per * 1000.0);
        assert!(per < 0.006, "frame {:.3}ms exceeds 6ms budget at 200x60", per * 1000.0);
    }
}
