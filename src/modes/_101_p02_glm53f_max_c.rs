//! Isfahan: a persian portal at dusk. A pointed arch opens out of a dark
//! pishtaq wall, and inside it a girih field of decagram and pentagram
//! cells assembles itself by seeded lattice rules. Straps of gold and jade
//! interlace through filler diamonds, a lantern glow rides the straps, and
//! collapsed cells show their scaffold until growth expands the grammar.
use crate::_0_profile::measure_layer;
use crate::color::{darken, lerp_color, lighten};
use crate::opts::param_f32;
use crate::registry::{AnimKind, Mode, ModeFrame, Param};
use crate::types::{Cell, Grid};
use crossterm::style::Color;
use rayon::prelude::*;
use std::cell::RefCell;
use std::f32::consts::TAU;

pub(super) struct Isfahan;
pub(super) static MODE: Isfahan = Isfahan;

const NAME: &str = "isfahan";
const KNOBS: usize = 4;
const HELP: &str =
    "isfahan: girih strapwork assembles inside a persian portal [growth] [tension] [lumina] [weave]";

// Hash layers.
const L_NIGHT: u64 = 0x21;
const L_FIELD: u64 = 0x22;
const L_STRAP: u64 = 0x23;
const L_KNOT: u64 = 0x24;
const L_PORTAL: u64 = 0x25;

const PARALLEL_MIN_CELLS: usize = 20_480;
const MAX_TILES: usize = 6000;

const PARAMS: &[Param] = &[
    param!("GROWTH", "expanded fraction of the girih grammar", 0.0, 1.0, 0.62, 0.02),
    param!("TENSION", "lattice rotation and arch pointedness", 0.0, 1.0, 0.35, 0.02),
    param!("LUMINA", "lantern glow travelling the straps", 0.0, 1.0, 0.55, 0.02),
    param!("WEAVE", "strapwork density and filler rate", 0.0, 1.0, 0.50, 0.02),
];

// Masonry texture outside the portal.
const STONE: [char; 3] = ['.', ':', '\''];
// Scaffold dot on cells the grammar has not expanded yet.
const SCAFFOLD: char = '\u{00B7}';
// Strap directions, crossings, and high-order knots.
const GLYPH_H: char = '-';
const GLYPH_V: char = '|';
const GLYPH_D1: char = '/';
const GLYPH_D2: char = '\\';
const GLYPH_CROSS: char = '+';
const GLYPH_X: char = 'x';
const GLYPH_NODE: char = '#';
// Focal glyphs: decagram heart, filler eye, impost pucks, apex finial.
const GLYPH_HEART: char = '\u{25C6}';
const GLYPH_EYE: char = '\u{25C7}';
const GLYPH_IMPOST: char = '\u{25CF}';
const GLYPH_FINIAL: char = '\u{25C6}';
const GLYPH_SPARK: char = '\'';

// Direction bits in the raster scratch mask; the tint bit votes gold.
const BIT_H: u8 = 1;
const BIT_V: u8 = 2;
const BIT_D1: u8 = 4;
const BIT_D2: u8 = 8;
const BIT_GOLD: u8 = 0x10;

impl Mode for Isfahan {
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
        let p: [f32; KNOBS] = std::array::from_fn(|i| {
            let param = &PARAMS[i];
            let value = frame
                .args
                .get(i + 4)
                .and_then(|v| v.parse::<f32>().ok())
                .or_else(|| frame.param_values.and_then(|v| v.get(i)).copied())
                .unwrap_or_else(|| param_f32(param.key, param.default));
            if value.is_finite() {
                value.clamp(param.min, param.max)
            } else {
                param.default
            }
        });
        draw(frame, &p);
    }
}

/// Splitmix64 over (seed, layer, index, slot); no rng stream is consumed.
#[inline]
fn hash(seed: u64, layer: u64, index: u64, slot: u64) -> u64 {
    let mut z = seed
        ^ layer.wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ index.wrapping_mul(0xD1B5_4A32_D192_ED03)
        ^ slot.wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    z ^= z >> 30;
    z = z.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z ^= z >> 27;
    z = z.wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

#[inline]
fn unit(h: u64) -> f32 {
    (h >> 40) as f32 * (1.0 / 16_777_216.0)
}

/// Lattice draws: slot picks the stream, ix mixes in so cells decorrelate.
#[inline]
fn field_hash(seed: u64, ix: i64, iy: i64, slot: u64) -> u64 {
    hash(seed, L_FIELD, ix as u64 ^ slot.wrapping_mul(0x2545_F491_4F6C_DD1D), iy as u64)
}

/// Two-centered pointed arch: jambs rise to a spring line, then two offset
/// circles cross at a cusp under the apex. World y grows upward from the floor.
struct Arch {
    jamb: f32,
    e: f32,
    r: f32,
    y_spring: f32,
    y_apex: f32,
}

impl Arch {
    /// `point` in [0, 1]: higher tension slides the centers apart, sharpening
    /// the cusp into a lancet.
    fn new(jamb: f32, y_spring: f32, point: f32) -> Self {
        let e = jamb * (0.22 + 0.20 * point);
        let r = jamb + e;
        let rise = (r * r - e * e).sqrt();
        Arch { jamb, e, r, y_spring, y_apex: y_spring + rise }
    }

    /// Opening height at world x; none beyond the jambs.
    fn profile(&self, x: f32) -> Option<f32> {
        let dx = x.abs();
        if dx > self.jamb {
            return None;
        }
        let inner = (self.r * self.r - (dx + self.e) * (dx + self.e)).max(0.0).sqrt();
        Some(self.y_spring + inner)
    }

    fn contains(&self, x: f32, y: f32) -> bool {
        match self.profile(x) {
            Some(top) => y >= 0.0 && y <= top,
            None => false,
        }
    }
}

/// One girih cell of the lattice.
struct GirihTile {
    cx: f32,
    cy: f32,
    radius: f32,
    variant: u8,
    live: bool,
}

/// One straight ink run in world coordinates.
struct StrapSeg {
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    gold: bool,
}

struct GirihField {
    tiles: Vec<GirihTile>,
    straps: Vec<StrapSeg>,
}

/// Resolved geometry and palette for one frame.
struct Look {
    seed: u64,
    w: usize,
    h: usize,
    aspect: f32,
    cxg: f32,
    world_h: f32,
    arch: Arch,
    lantern: (f32, f32),
    time: f32,
    growth: f32,
    tension: f32,
    lumina: f32,
    weave: f32,
    rot: f32,
    wx_col: Vec<f32>,
    prof: Vec<f32>,
    wall: Color,
    void: Color,
    band: Color,
    stone: Color,
    strap: [Color; 2],
    knot: Color,
    rim: [Color; 2],
    bright: Color,
}

impl Look {
    fn new(seed: u64, w: usize, h: usize, palette: &[Color; 5], time: f32, p: &[f32; KNOBS]) -> Self {
        let aspect = 2.0f32;
        let world_w = w as f32 / aspect;
        let world_h = h as f32;
        let cxg = w as f32 * 0.5;
        // Fit the portal to both axes, then shrink once more if the cusp
        // would leave the frame.
        let mut jamb = (0.80 * (world_w * 0.5).min((world_h - 3.0) * 0.60)).max(2.0);
        let mut y_spring = (jamb * 0.42).max(1.5);
        let mut arch = Arch::new(jamb, y_spring, p[1]);
        let headroom = world_h - 1.2 - y_spring;
        if arch.y_apex > headroom + y_spring {
            let rise = arch.y_apex - y_spring;
            let shrink = (headroom / rise).max(0.25);
            jamb *= shrink;
            y_spring = (jamb * 0.42).max(1.0);
            arch = Arch::new(jamb, y_spring, p[1]);
        }
        let mut wx_col = Vec::with_capacity(w);
        let mut prof = Vec::with_capacity(w);
        for x in 0..w {
            let wx = (x as f32 + 0.5 - cxg) / aspect;
            wx_col.push(wx);
            prof.push(arch.profile(wx).unwrap_or(f32::NAN));
        }
        Look {
            seed,
            w,
            h,
            aspect,
            cxg,
            world_h,
            arch,
            lantern: (0.0, y_spring * 0.55 + 0.5),
            time,
            growth: p[0],
            tension: p[1],
            lumina: p[2],
            weave: p[3],
            rot: (p[1] - 0.5) * 0.9,
            wx_col,
            prof,
            wall: lerp_color(palette[0], palette[2], 0.10),
            void: darken(palette[0], 8),
            band: lerp_color(palette[0], palette[2], 0.28),
            stone: lerp_color(palette[1], palette[2], 0.40),
            strap: [palette[1], palette[3]],
            knot: lighten(palette[3], 30),
            rim: [lighten(palette[2], 25), darken(palette[2], 5)],
            bright: palette[4],
        }
    }

    #[inline]
    fn wx(&self, gx: f32) -> f32 {
        (gx - self.cxg) / self.aspect
    }

    #[inline]
    fn wy(&self, gy: f32) -> f32 {
        self.h as f32 - 0.5 - gy
    }

    #[inline]
    fn gx(&self, wx: f32) -> f32 {
        self.cxg + wx * self.aspect
    }

    #[inline]
    fn gy(&self, wy: f32) -> f32 {
        self.h as f32 - 0.5 - wy
    }
}

thread_local! {
    static MASK: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

#[inline]
fn put(grid: &mut Grid, x: i32, y: i32, cell: Cell) {
    if x < 0 || y < 0 {
        return;
    }
    let (x, y) = (x as usize, y as usize);
    if y < grid.len() && x < grid[y].len() {
        grid[y][x] = cell;
    }
}

fn draw(frame: &mut ModeFrame<'_>, p: &[f32; KNOBS]) {
    let (w, h) = (frame.width, frame.height);
    if w < 4 || h < 4 {
        return;
    }
    let look = Look::new(frame.seed, w, h, frame.palette, frame.time, p);
    let field = measure_layer(NAME, "field", || build_field(&look));
    MASK.with(|slot| {
        let mut buf = slot.borrow_mut();
        if buf.len() < w * h {
            buf.resize(w * h, 0);
        }
        let mask = &mut buf[..w * h];
        measure_layer(NAME, "night", || paint_night(frame.grid, w, h, &look));
        measure_layer(NAME, "scaffold", || paint_scaffold(frame.grid, &look, &field));
        measure_layer(NAME, "straps", || paint_straps(frame.grid, mask, w, h, &look, &field));
        measure_layer(NAME, "knots", || paint_knots(frame.grid, mask, w, h, &look, &field));
        measure_layer(NAME, "portal", || paint_portal(frame.grid, w, h, &look));
        measure_layer(NAME, "crown", || paint_crown(frame.grid, w, h, &look));
    });
}

/// Rectangular decagon lattice under the portal, rotated by tension, with
/// filler diamonds in the cell gaps. Expansion is seeded and gated by growth.
fn build_field(look: &Look) -> GirihField {
    let across = 4.0 + 4.0 * look.weave;
    let rd = (look.arch.jamb / across).max(0.8);
    let sx = rd * 2.60;
    let sy = sx * 0.90;
    let (cos_r, sin_r) = (look.rot.cos(), look.rot.sin());
    let pivot_y = look.world_h * 0.44;
    let span = look.arch.jamb + rd + 1.0;
    let ix_n = (span / sx).ceil() as i64 + 1;
    let iy_n = ((look.arch.y_apex + rd + 1.0) / sy).ceil() as i64 + 1;
    let mut tiles = Vec::new();
    let mut straps = Vec::new();
    let project = |u: f32, v: f32| -> (f32, f32) {
        (u * cos_r - v * sin_r, pivot_y + u * sin_r + v * cos_r)
    };
    for iy in -iy_n..=iy_n {
        for ix in -ix_n..=ix_n {
            let (cx, cy) = project(ix as f32 * sx, iy as f32 * sy);
            if !inside_field(look, cx, cy) {
                continue;
            }
            if tiles.len() >= MAX_TILES {
                break;
            }
            let g0 = field_hash(look.seed, ix, iy, 0);
            let live = unit(g0) < 0.22 + 0.78 * look.growth;
            let variant = if unit(field_hash(look.seed, ix, iy, 1)) < 0.60 {
                0
            } else {
                1
            };
            tiles.push(GirihTile { cx, cy, radius: rd, variant, live });
            if !live {
                continue;
            }
            match variant {
                0 => decagram(look, cx, cy, rd, &mut straps),
                _ => pentagram(look, cx, cy, rd, &mut straps),
            }
            // Filler diamond in the gap toward the upper right; each gap is
            // claimed once, keyed by its lower left cell.
            let (fx, fy) = project((ix as f32 + 0.5) * sx, (iy as f32 + 0.5) * sy);
            if inside_field(look, fx, fy)
                && unit(field_hash(look.seed, ix, iy, 5))
                    < 0.18 + 0.50 * look.weave
            {
                tiles.push(GirihTile { cx: fx, cy: fy, radius: rd * 0.5, variant: 2, live: true });
                filler(fx, fy, rd, &mut straps);
            }
        }
    }
    GirihField { tiles, straps }
}

/// Keep cells inside the opening and clear of the rim and the floor.
fn inside_field(look: &Look, x: f32, y: f32) -> bool {
    match look.arch.profile(x) {
        Some(top) => y > 0.35 && y < top - 0.55,
        None => false,
    }
}

/// Ten chords of the {10/3} star plus five spokes out through edge midpoints.
fn decagram(look: &Look, cx: f32, cy: f32, rd: f32, straps: &mut Vec<StrapSeg>) {
    let vr = rd * 0.92;
    let mut vx = [0.0f32; 10];
    let mut vy = [0.0f32; 10];
    for (k, pair) in vx.iter_mut().zip(vy.iter_mut()).enumerate() {
        let a = look.rot + k as f32 * TAU / 10.0;
        *pair.0 = cx + vr * a.cos();
        *pair.1 = cy + vr * a.sin();
    }
    for k in 0..10 {
        let j = (k + 3) % 10;
        straps.push(StrapSeg { x0: vx[k], y0: vy[k], x1: vx[j], y1: vy[j], gold: true });
    }
    for k in (0..10).step_by(2) {
        let mx = (vx[k] + vx[(k + 1) % 10]) * 0.5;
        let my = (vy[k] + vy[(k + 1) % 10]) * 0.5;
        spoke(cx, cy, mx, my, straps);
    }
}

/// Five pentagram chords plus spokes through the other edge midpoints.
fn pentagram(look: &Look, cx: f32, cy: f32, rd: f32, straps: &mut Vec<StrapSeg>) {
    let vr = rd * 0.98;
    let mut vx = [0.0f32; 5];
    let mut vy = [0.0f32; 5];
    for (k, pair) in vx.iter_mut().zip(vy.iter_mut()).enumerate() {
        let a = look.rot + TAU / 20.0 + k as f32 * TAU / 5.0;
        *pair.0 = cx + vr * a.cos();
        *pair.1 = cy + vr * a.sin();
    }
    for k in 0..5 {
        let j = (k + 2) % 5;
        straps.push(StrapSeg { x0: vx[k], y0: vy[k], x1: vx[j], y1: vy[j], gold: false });
    }
    let mr = rd * 0.92;
    for k in 0..5 {
        let a = look.rot + TAU / 20.0 + (2 * k + 1) as f32 * TAU / 10.0 + TAU / 20.0;
        spoke(cx, cy, cx + mr * a.cos(), cy + mr * a.sin(), straps);
    }
}

/// A matching strap through an edge midpoint: from a third of the way out to
/// just past the midpoint, so neighbor cells chain across the gaps.
fn spoke(cx: f32, cy: f32, mx: f32, my: f32, straps: &mut Vec<StrapSeg>) {
    let (dx, dy) = (mx - cx, my - cy);
    straps.push(StrapSeg {
        x0: cx + dx * 0.34,
        y0: cy + dy * 0.34,
        x1: cx + dx * 1.16,
        y1: cy + dy * 1.16,
        gold: true,
    });
}

/// Small diamond bridging four decagons; second scale of the weave.
fn filler(fx: f32, fy: f32, rd: f32, straps: &mut Vec<StrapSeg>) {
    let hx = rd * 0.34;
    let hy = rd * 0.30;
    let (e, n, w, s) = ((fx + hx, fy), (fx, fy + hy), (fx - hx, fy), (fx, fy - hy));
    for (a, b) in [(&n, &e), (&e, &s), (&s, &w), (&w, &n)] {
        straps.push(StrapSeg { x0: a.0, y0: a.1, x1: b.0, y1: b.1, gold: false });
    }
}

/// Pishtaq wall, paving, and the void of the opening.
fn paint_night(grid: &mut Grid, w: usize, h: usize, look: &Look) {
    let rows = grid.len().min(h);
    let slice = &mut grid[..rows];
    let paint = |(gy, row): (usize, &mut Vec<Cell>)| {
        let wy = look.wy(gy as f32);
        for (gx, cell) in row.iter_mut().enumerate().take(w) {
            let wx = look.wx_col[gx];
            let jamb = look.arch.jamb;
            let in_band_x = wx.abs() > jamb + 1.15 && wx.abs() < jamb + 1.85;
            let in_band_top = wy > look.arch.y_apex + 0.55
                && wy < look.arch.y_apex + 1.25
                && wx.abs() < jamb + 1.85;
            let g = hash(look.seed, L_NIGHT, gx as u64, gy as u64);
            let pilaster = wx.abs() > jamb + 0.62 && wx.abs() < jamb + 1.15;
            if in_band_x || in_band_top {
                *cell = Cell::with_bg(' ', look.stone, look.band);
            } else if look.arch.contains(wx, wy) {
                let bg = if wy < 1.6 { darken(look.void, 4) } else { look.void };
                *cell = Cell::with_bg(' ', bg, bg);
            } else if wy < 1.1 {
                let fg = lerp_color(look.wall, look.stone, 0.25);
                let ch = if (gx + gy) % 3 == 0 && unit(g) < 0.30 { '.' } else { ' ' };
                *cell = Cell::with_bg(ch, fg, lerp_color(look.wall, look.stone, 0.12));
            } else if pilaster {
                *cell = Cell::with_bg(' ', darken(look.stone, 20), darken(look.wall, 5));
            } else {
                let ch = if unit(g) < 0.045 { STONE[(g >> 8) as usize % STONE.len()] } else { ' ' };
                *cell = Cell::with_bg(ch, darken(look.stone, 25), look.wall);
            }
        }
    };
    if w * h >= PARALLEL_MIN_CELLS {
        slice.par_iter_mut().enumerate().with_min_len(8).for_each(paint);
    } else {
        slice.iter_mut().enumerate().for_each(paint);
    }
}

/// Collapsed cells show their vertex scaffold, waiting for growth.
fn paint_scaffold(grid: &mut Grid, look: &Look, field: &GirihField) {
    let fg = darken(look.stone, 10);
    for tile in &field.tiles {
        if tile.live {
            continue;
        }
        match tile.variant {
            2 => {
                let (hx, hy) = (tile.radius * 0.22, tile.radius * 0.20);
                for (dx, dy) in [(hx, 0.0), (0.0, hy), (-hx, 0.0), (0.0, -hy)] {
                    put(
                        grid,
                        look.gx(tile.cx + dx).round() as i32,
                        look.gy(tile.cy + dy).round() as i32,
                        Cell::with_bg(SCAFFOLD, fg, look.void),
                    );
                }
            }
            _ => {
                let vr = tile.radius * 0.92;
                for k in 0..10 {
                    let a = look.rot + k as f32 * TAU / 10.0;
                    let (sx, sy) = (tile.cx + vr * a.cos(), tile.cy + vr * a.sin());
                    if !look.arch.contains(sx, sy) {
                        continue;
                    }
                    put(
                        grid,
                        look.gx(sx).round() as i32,
                        look.gy(sy).round() as i32,
                        Cell::with_bg(SCAFFOLD, fg, look.void),
                    );
                }
            }
        }
    }
}

/// Raster the strap runs, one glyph per direction, votes in the scratch mask.
fn paint_straps(grid: &mut Grid, mask: &mut [u8], w: usize, h: usize, look: &Look, field: &GirihField) {
    let strap = &look.strap;
    for seg in &field.straps {
        let (x0, y0) = (look.gx(seg.x0), look.gy(seg.y0));
        let (x1, y1) = (look.gx(seg.x1), look.gy(seg.y1));
        let (dx, dy) = (x1 - x0, y1 - y0);
        let steps = (dx.abs().max(dy.abs()).ceil() as usize).clamp(1, w + h);
        let bit = dir_bit(dx, dy);
        let mx = (seg.x0 + seg.x1) * 0.5;
        let my = (seg.y0 + seg.y1) * 0.5;
        let glow = strap_glow(look, mx, my);
        let base = if seg.gold { strap[0] } else { strap[1] };
        let fg = lerp_color(base, lighten(base, 70), glow);
        for s in 0..=steps {
            let t = s as f32 / steps as f32;
            let (xi, yi) = ((x0 + dx * t).round() as i32, (y0 + dy * t).round() as i32);
            if xi < 0 || yi < 0 || xi as usize >= w || yi as usize >= h {
                continue;
            }
            if !look.arch.contains(look.wx(xi as f32), look.wy(yi as f32)) {
                continue;
            }
            let idx = yi as usize * w + xi as usize;
            if seg.gold {
                mask[idx] |= BIT_GOLD;
            }
            mask[idx] |= bit;
        }
    }
    for (idx, m) in mask.iter().enumerate() {
        let bits = m & 0x0F;
        if bits == 0 {
            continue;
        }
        let (gx, gy) = ((idx % w) as i32, (idx / w) as i32);
        let fg = if m & BIT_GOLD != 0 { strap[0] } else { strap[1] };
        let ch = strap_glyph(bits);
        put(grid, gx, gy, Cell::with_bg(ch, fg, look.void));
    }
}

#[inline]
fn dir_bit(dx: f32, dy: f32) -> u8 {
    let ang = dy.atan2(dx).abs();
    if ang < 0.30 {
        BIT_H
    } else if ang > std::f32::consts::FRAC_PI_2 - 0.30 {
        BIT_V
    } else if dx * dy > 0.0 {
        BIT_D2
    } else {
        BIT_D1
    }
}

#[inline]
fn strap_glyph(bits: u8) -> char {
    match bits.count_ones() {
        1 => match bits {
            BIT_H => GLYPH_H,
            BIT_V => GLYPH_V,
            BIT_D1 => GLYPH_D1,
            _ => GLYPH_D2,
        },
        2 => match bits {
            b if b == BIT_H | BIT_V => GLYPH_CROSS,
            b if b == BIT_D1 | BIT_D2 => GLYPH_X,
            _ => GLYPH_NODE,
        },
        _ => GLYPH_NODE,
    }
}

/// Lantern phase: a luminance wave rides outward from the focus.
#[inline]
fn strap_glow(look: &Look, wx: f32, wy: f32) -> f32 {
    let (lx, ly) = look.lantern;
    let d = ((wx - lx).powi(2) + (wy - ly).powi(2)).sqrt();
    let wave = 0.5 + 0.5 * (TAU * (look.time * 0.18 - d * 0.16)).sin();
    (0.18 + look.lumina * 0.65 * (0.4 + 0.6 * wave)).clamp(0.0, 1.0)
}

/// Recolor crossings as knots and stamp the focal glyphs.
fn paint_knots(grid: &mut Grid, mask: &[u8], w: usize, h: usize, look: &Look, field: &GirihField) {
    for (idx, m) in mask.iter().enumerate() {
        let bits = m & 0x0F;
        if bits.count_ones() < 2 {
            continue;
        }
        let (gx, gy) = ((idx % w) as i32, (idx / w) as i32);
        let ch = if bits.count_ones() >= 3 { GLYPH_NODE } else { strap_glyph(bits) };
        let glow = strap_glow(look, look.wx(gx as f32), look.wy(gy as f32));
        let fg = lerp_color(look.knot, lighten(look.knot, 60), glow);
        put(grid, gx, gy, Cell::with_bg(ch, fg, look.void));
    }
    for tile in &field.tiles {
        if !tile.live {
            continue;
        }
        let (cx, cy) = (look.gx(tile.cx).round() as i32, look.gy(tile.cy).round() as i32);
        match tile.variant {
            0 => put(grid, cx, cy, Cell::with_bg(GLYPH_HEART, lighten(look.bright, 20), look.void)),
            2 => put(grid, cx, cy, Cell::with_bg(GLYPH_EYE, look.strap[1], look.void)),
            _ => {}
        }
    }
}

/// Jamb stones, arch rim, impost pucks, and the apex finial.
fn paint_portal(grid: &mut Grid, w: usize, h: usize, look: &Look) {
    let rows = grid.len().min(h);
    let slice = &mut grid[..rows];
    let arch = &look.arch;
    let paint = |(gy, row): (usize, &mut Vec<Cell>)| {
        let wy = look.wy(gy as f32);
        for (gx, cell) in row.iter_mut().enumerate().take(w) {
            let wx = look.wx_col[gx];
            let prof = look.prof[gx];
            let jamb = arch.jamb;
            let impost = wx.abs() > jamb - 0.55 && wx.abs() < jamb + 0.62 && (wy - arch.y_spring).abs() < 0.4;
            let finial = wx.abs() < 0.5 && wy > arch.y_apex - 0.15 && wy < arch.y_apex + 1.3;
            if impost {
                *cell = Cell::with_bg(GLYPH_IMPOST, look.bright, look.wall);
            } else if finial {
                let ch = if wy > arch.y_apex + 0.2 { GLYPH_SPARK } else { GLYPH_FINIAL };
                *cell = Cell::with_bg(ch, look.bright, look.wall);
            } else if wx.abs() > jamb + 0.02 && wx.abs() < jamb + 0.62 && wy > -0.1 && wy < arch.y_spring + 0.15 {
                *cell = Cell::with_bg(GLYPH_NODE, look.stone, look.wall);
            } else if prof.is_finite() && wy > prof - 0.62 && wy < prof + 0.02 {
                let (ch, fg) = if wy > prof - 0.30 {
                    ('%', look.rim[0])
                } else {
                    ('#', look.rim[1])
                };
                *cell = Cell::with_bg(ch, fg, look.wall);
            }
        }
    };
    if w * h >= PARALLEL_MIN_CELLS {
        slice.par_iter_mut().enumerate().with_min_len(8).for_each(paint);
    } else {
        slice.iter_mut().enumerate().for_each(paint);
    }
}

/// Muqarnas fan filling the lunette: rings concentric on the apex, seeded
/// radial sectors, alternating tier and void cells for negative space.
fn paint_crown(grid: &mut Grid, w: usize, h: usize, look: &Look) {
    let apex = look.arch.y_apex;
    let ring_w = ((apex - look.arch.y_spring) / 3.5).max(0.7);
    let sectors = 10 + 2 * (unit(hash(look.seed, L_PORTAL, 3, 7)) * 4.0).round() as i32;
    let half = std::f32::consts::FRAC_PI_2;
    let sector_w = std::f32::consts::PI / sectors as f32;
    let tier = ['\u{00B7}', ':', '%', '#'];
    let gx0 = look.gx(-look.arch.jamb - 1.0).floor().max(0.0) as usize;
    let gx1 = (look.gx(look.arch.jamb + 1.0).ceil() as usize + 1).min(w);
    let gy0 = look.gy(apex + 0.5).max(0.0) as usize;
    let rows = grid.len().min(h);
    for gy in gy0..rows {
        for gx in gx0..gx1 {
            let wx = look.wx_col[gx];
            let wy = look.wy(gy as f32);
            let lower = match look.prof[gx] {
                p if p.is_finite() => p + 0.05,
                _ => look.arch.y_spring + 0.15,
            };
            if wy <= lower || wy > apex + 0.5 {
                continue;
            }
            let impost_zone =
                wx.abs() > look.arch.jamb - 0.55 && (wy - look.arch.y_spring).abs() < 0.4;
            let r = (wx * wx + (wy - apex) * (wy - apex)).sqrt();
            let ring = (r / ring_w) as i32;
            if ring == 0 || impost_zone {
                continue;
            }
            let ang = wx.atan2(apex - wy).clamp(-half, half);
            let u = (ang + half) / sector_w;
            let s = (u.floor() as i32).clamp(0, sectors - 1);
            if (ring + s).rem_euclid(2) == 1 {
                continue;
            }
            let fg = match ring {
                1 => darken(look.stone, 18),
                2 => darken(look.stone, 10),
                3 => look.stone,
                _ => lighten(look.stone, 5),
            };
            let ch = if u - u.floor() < 0.22 && ring >= 2 {
                if wx > 0.0 { GLYPH_D1 } else { GLYPH_D2 }
            } else {
                tier[(ring as usize).min(3)]
            };
            grid[gy][gx] = Cell::with_bg(ch, fg, look.wall);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::grid_to_plain;
    use rand::{rngs::StdRng, SeedableRng};

    fn knobs() -> Vec<f32> {
        PARAMS.iter().map(|p| p.default).collect()
    }

    fn frame(w: usize, h: usize, seed: u64, time: f32, values: &[f32]) -> Grid {
        let mut grid = vec![vec![Cell::blank(); w]; h];
        let palette = crate::color::make_palette(seed);
        let mut rng = StdRng::seed_from_u64(seed);
        MODE.render(&mut ModeFrame {
            grid: &mut grid,
            width: w,
            height: h,
            seed,
            palette: &palette,
            rng: &mut rng,
            time,
            args: &[],
            param_values: Some(values),
        });
        grid
    }

    fn text(grid: &Grid) -> String {
        grid_to_plain(grid).join("\n")
    }

    #[test]
    fn isfahan_seed42() {
        insta::assert_snapshot!("isfahan_80x24", text(&frame(80, 24, 42, 0.0, &knobs())));
    }

    #[test]
    fn isfahan_seed42_large() {
        insta::assert_snapshot!("isfahan_120x40", text(&frame(120, 40, 42, 0.0, &knobs())));
    }

    #[test]
    fn isfahan_seed42_clip() {
        insta::assert_snapshot!("isfahan_clip_40x12", text(&frame(40, 12, 42, 0.0, &knobs())));
    }

    #[test]
    fn deterministic_and_seed_sensitive() {
        let k = knobs();
        assert_eq!(text(&frame(90, 30, 42, 0.0, &k)), text(&frame(90, 30, 42, 0.0, &k)));
        assert_ne!(text(&frame(90, 30, 42, 0.0, &k)), text(&frame(90, 30, 7, 0.0, &k)));
    }

    #[test]
    fn weave_changes_the_figure() {
        let mut k = knobs();
        let a = text(&frame(90, 30, 42, 0.0, &k));
        k[3] = 0.1;
        assert_ne!(a, text(&frame(90, 30, 42, 0.0, &k)));
    }

    #[test]
    fn small_dims_terminate() {
        let k = knobs();
        for (w, h) in [(40usize, 12usize), (24usize, 8usize), (17usize, 6usize)] {
            let grid = frame(w, h, 42, 0.0, &k);
            assert_eq!(grid.len(), h);
        }
    }

    #[test]
    fn frame_cost() {
        let (w, h) = (200usize, 60usize);
        let k = knobs();
        let mut worst = 0.0f64;
        let start = std::time::Instant::now();
        for f in 0..100 {
            let t0 = std::time::Instant::now();
            frame(w, h, 42, f as f32 * 0.05, &k);
            worst = worst.max(t0.elapsed().as_secs_f64() * 1000.0);
        }
        let avg = start.elapsed().as_secs_f64() * 1000.0 / 100.0;
        eprintln!("isfahan frame_cost 200x60: avg {:.3} ms, worst {:.3} ms", avg, worst);
        if !cfg!(debug_assertions) {
            assert!(avg < 6.0, "avg frame {:.3} ms", avg);
        }
    }
}
