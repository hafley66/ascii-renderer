//! Prismata: one kaleidoscopic interference field read through five engraving
//! dialects and seven structural forms. Per frame: a field pass plus four paint
//! passes, all O(W*H), over one reusable thread-local (field, channel) scratch.
use crate::_0_profile::measure_layer;
use crate::color::{darken, lerp_color, lighten, shift_hue};
use crate::opts::param_f32;
use crate::registry::{AnimKind, Mode, ModeFrame, Param};
use crate::types::{Cell, Grid};
use crossterm::style::Color;
use rayon::prelude::*;
use std::cell::RefCell;
use std::f32::consts::{FRAC_PI_2, TAU};
use std::sync::OnceLock;

pub(super) struct Prismata;
pub(super) static MODE: Prismata = Prismata;

const NAME: &str = "prismata";
const KNOBS: usize = 12;
const HELP: &str = "prismata: kaleidoscopic field engraving [form 0 rings 1 plasma 2 spiral 3 lattice 4 cells 5 moire 6 ripple] [dialect 0 engrave 1 stipple 2 mosaic 3 hatch 4 weave] [glyphs 0 density 1 blocks 2 marks] [symmetry] [freq] [warp] [density] [contrast] [chroma] [glow] [flow] [grain]";

const DENSITY: &[char] = &[' ', '.', ':', '-', '=', '+', '*', '#', '%', '@'];
const BLOCKS: &[char] = &[' ', '\u{2591}', '\u{2592}', '\u{2593}', '\u{2588}'];
const MARKS: &[char] = &[' ', '\'', '.', ',', ':', ';', '+', 'o', '*', '#'];
const LINES: [[char; 5]; 3] = [
    ['-', '|', '/', '\\', '+'],
    ['=', '#', '/', '\\', '@'],
    [':', ';', '/', '\\', '*'],
];
const DOTS: [[char; 6]; 3] = [
    [' ', '.', ':', ':', '+', '*'],
    [' ', '.', '\u{2591}', '\u{2592}', '\u{2593}', '\u{2588}'],
    [' ', '.', ',', ';', 'o', 'x'],
];
const DUST: &[char] = &['.', '\'', '`', ',', '*'];

const WAVE_SIZE: usize = 4096;
const WAVE_MASK: usize = WAVE_SIZE - 1;
static WAVE: OnceLock<[f32; WAVE_SIZE]> = OnceLock::new();

/// One period of sine, built once. Per-cell phases then cost a multiply, a
/// truncating convert, a mask and a load instead of a libm argument reduction.
fn wave_table() -> &'static [f32; WAVE_SIZE] {
    WAVE.get_or_init(|| std::array::from_fn(|i| (i as f32 * TAU / WAVE_SIZE as f32).sin()))
}

/// Grids at or above this cell count shade rows on rayon's pool; the value is this
/// mode's measured break-even, not the 65,536 that cheaper bodies use.
const PARALLEL_MIN_CELLS: usize = 16_384;

/// Runs `row` over the first `h` grid rows, across the pool above the gate.
#[inline(always)]
fn shade_rows<F>(grid: &mut Grid, w: usize, h: usize, row: F)
where
    F: Fn((usize, &mut Vec<Cell>)) + Sync + Send,
{
    let rows = grid.len().min(h);
    let slice = &mut grid[..rows];
    if w * h >= PARALLEL_MIN_CELLS {
        slice
            .par_iter_mut()
            .enumerate()
            .with_min_len(16)
            .for_each(row);
    } else {
        slice.iter_mut().enumerate().for_each(row);
    }
}

/// The field pass counterpart of [`shade_rows`], over the scratch's `w`-wide rows.
#[inline(always)]
fn fill_rows<F>(field: &mut [[f32; 2]], w: usize, h: usize, row: F)
where
    F: Fn((usize, &mut [[f32; 2]])) + Sync + Send,
{
    if w == 0 {
        return;
    }
    if w * h >= PARALLEL_MIN_CELLS {
        field
            .par_chunks_mut(w)
            .enumerate()
            .with_min_len(16)
            .for_each(row);
    } else {
        field.chunks_mut(w).enumerate().for_each(row);
    }
}

const PARAMS: &[Param] = &[
    param!("FORM", "form (7 structures)", 0.0, 6.0, 0.0, 1.0),
    param!("DIALECT", "dialect (5 inks)", 0.0, 4.0, 0.0, 1.0),
    param!("GLYPHS", "glyph set (3)", 0.0, 2.0, 0.0, 1.0),
    param!("SYMMETRY", "kaleidoscope sectors", 1.0, 12.0, 6.0, 1.0),
    param!("FREQ", "field frequency", 1.0, 12.0, 4.0, 0.25),
    param!("WARP", "domain warp", 0.0, 1.0, 0.35, 0.05),
    param!("DENSITY", "ink coverage", 0.05, 1.0, 0.6, 0.05),
    param!("CONTRAST", "edge sharpening", 0.0, 1.0, 0.4, 0.05),
    param!("CHROMA", "mono to spectral", 0.0, 1.0, 0.6, 0.05),
    param!("GLOW", "ground bloom", 0.0, 1.0, 0.45, 0.05),
    param!("FLOW", "drift rate", 0.0, 2.0, 0.5, 0.05),
    param!("GRAIN", "dither grain", 0.0, 1.0, 0.25, 0.05),
];

impl Mode for Prismata {
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
        // Resolve positional arguments, native live values, then env defaults.
        // Clamp every knob so a wild roll or a hostile environment stays drawable.
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

thread_local! {
    static SCRATCH: RefCell<Vec<[f32; 2]>> = const { RefCell::new(Vec::new()) };
}

fn draw(frame: &mut ModeFrame<'_>, p: &[f32; KNOBS]) {
    let (w, h) = (frame.width, frame.height);
    if w == 0 || h == 0 {
        return;
    }
    let look = Look::new(frame.seed, w, h, frame.palette, frame.time, p);
    SCRATCH.with(|slot| {
        let mut buf = slot.borrow_mut();
        if buf.len() < w * h {
            buf.resize(w * h, [0.0, 0.0]);
        }
        let field = &mut buf[..w * h];
        // The scratch keeps the grid allocation untouched between frames, so a
        // reshaped terminal grows it once and animation never reallocates.
        measure_layer(NAME, "field", || fill_field(field, w, h, &look));
        measure_layer(NAME, "ground", || paint_ground(frame.grid, field, w, h, &look));
        measure_layer(NAME, "ink", || paint_ink(frame.grid, field, w, h, &look));
        measure_layer(NAME, "detail", || paint_detail(frame.grid, field, w, h, &look));
        measure_layer(NAME, "dust", || paint_dust(frame.grid, w, h, &look));
    });
}

/// Everything a frame needs: geometry, resolved knobs, per-frame colors and ramps.
struct Look {
    seed: u64,
    cx: f32,
    cy: f32,
    scale: f32,
    form: usize,
    dialect: usize,
    symmetry: usize,
    fold: f32,
    inv_fold: f32,
    half: f32,
    rot: f32,
    crot: f32,
    srot: f32,
    freq: f32,
    warp: f32,
    warp_freq: f32,
    flow: f32,
    contrast: f32,
    density: f32,
    bias: f32,
    grain: f32,
    ph: [f32; 6],
    moire_c: f32,
    moire_s: f32,
    src1: [f32; 2],
    src2: [f32; 2],
    wave: &'static [f32; WAVE_SIZE],
    wave_scale: f32,
    ramp: &'static [char],
    dots: &'static [char],
    line: [char; 5],
    bands: f32,
    strand: f32,
    ground_gain: f32,
    deep: Color,
    accent: Color,
    fg_lut: [[Color; 32]; 8],
    bg_lut: [[Color; 32]; 8],
    block_bg: [Color; 8],
}

impl Look {
    fn new(seed: u64, w: usize, h: usize, palette: &[Color; 5], time: f32, p: &[f32; KNOBS]) -> Self {
        let form = p[0] as usize;
        let dialect = p[1] as usize;
        let glyphs = (p[2] as usize).min(2);
        let symmetry = (p[3] as usize).clamp(1, 12);
        let freq = p[4];
        let time = if time.is_finite() {
            time.clamp(-1.0e6, 1.0e6)
        } else {
            0.0
        };
        let flow = time * p[10];
        let phase = |k: u64| unit(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ k) * TAU;
        let rot = phase(0x7F) + flow * 0.13;
        let delta = 0.05 + unit(seed ^ 0xA5A5) * 0.1;
        let radius = 0.45 + 0.3 * (flow * 0.5).sin();
        let deep = darken(palette[0], 42);
        let chroma = p[8];
        let mut fg_lut = [[deep; 32]; 8];
        let mut bg_lut = [[deep; 32]; 8];
        let mut block_bg = [deep; 8];
        let ink = darken(palette[0], 24);
        for b in 0..8 {
            let x = b as f32 / 7.0;
            let base = if x < 0.5 {
                lerp_color(palette[1], palette[2], x * 2.0)
            } else {
                lerp_color(palette[2], palette[3], (x - 0.5) * 2.0)
            };
            let spin = shift_hue(base, (b as f64 - 3.5) * 9.0 * chroma as f64);
            let hue = lerp_color(palette[4], spin, chroma);
            let lo = darken(hue, 46);
            let hi = lighten(hue, 34);
            let mid = darken(hue, 34);
            for l in 0..32 {
                let t = l as f32 / 31.0;
                fg_lut[b][l] = lerp_color(lo, hi, t);
                bg_lut[b][l] = lerp_color(ink, mid, t * 0.85);
            }
            block_bg[b] = lerp_color(ink, mid, 0.55);
        }
        Look {
            seed,
            cx: (w as f32 - 1.0) * 0.5,
            cy: (h as f32 - 1.0) * 0.5,
            scale: (w as f32 * 0.5).min(h as f32).max(1.0),
            form,
            dialect,
            symmetry,
            fold: TAU / symmetry as f32,
            inv_fold: symmetry as f32 / TAU,
            half: TAU / (symmetry as f32 * 2.0),
            rot,
            crot: rot.cos(),
            srot: rot.sin(),
            freq,
            warp: p[5] * 0.2,
            warp_freq: 1.5 + freq * 0.35,
            flow,
            contrast: p[7],
            density: p[6],
            bias: (p[6] - 0.6) * 0.9,
            grain: p[11],
            ph: [
                phase(0x11),
                phase(0x23),
                phase(0x37),
                phase(0x49),
                phase(0x5B),
                phase(0x6D),
            ],
            moire_c: delta.cos(),
            moire_s: delta.sin(),
            src1: [0.0, 0.0],
            src2: [radius * rot.cos(), radius * rot.sin()],
            wave: wave_table(),
            wave_scale: WAVE_SIZE as f32 / TAU,
            ramp: match glyphs {
                1 => BLOCKS,
                2 => MARKS,
                _ => DENSITY,
            },
            dots: &DOTS[glyphs],
            line: LINES[glyphs],
            bands: 3.0 + (freq * 0.9).round(),
            strand: 2.0 + 2.0 * (freq * 0.3).round(),
            ground_gain: p[9] * match dialect {
                1 => 0.4,
                4 => 1.0,
                _ => 0.85,
            },
            deep,
            accent: lighten(palette[4], 40),
            fg_lut,
            bg_lut,
            block_bg,
        }
    }
}

impl Look {
    #[inline]
    fn sin(&self, x: f32) -> f32 {
        self.wave[self.sin_index(x)]
    }
    #[inline]
    fn cos(&self, x: f32) -> f32 {
        self.sin(x + FRAC_PI_2)
    }
    #[inline]
    fn sin_index(&self, x: f32) -> usize {
        ((x * self.wave_scale) as i32 as usize) & WAVE_MASK
    }
}

fn fill_field(field: &mut [[f32; 2]], w: usize, h: usize, look: &Look) {
    let freq = look.freq;
    let (cr, sr) = (look.crot, look.srot);
    let [p0, p1, p2, p3, p4, p5] = look.ph;
    let flow = look.flow;
    let warp = look.warp;
    let wf = look.warp_freq;
    let sym = look.symmetry;
    let (fold, half) = (look.fold, look.half);
    let seed = look.seed;
    let contrast = look.contrast;
    let spiral = look.form == 2;
    fill_rows(field, w, h, |(y, row): (usize, &mut [[f32; 2]])| {
        let py0 = (y as f32 + 0.5 - look.cy) * 2.0 / look.scale;
        for (x, slot) in row.iter_mut().enumerate() {
            let mut px = (x as f32 + 0.5 - look.cx) / look.scale;
            let mut py = py0;
            if warp > 0.0 {
                px += warp * look.sin(py * wf + p3 + flow * 0.51);
                py += warp * look.sin(px * wf + p4 - flow * 0.37);
            }
            let r = (px * px + py * py).sqrt();
            let (rx, ry, theta) = if sym > 1 {
                let raw = py.atan2(px);
                let wrapped = raw - (raw * look.inv_fold).floor() * fold;
                let ang = (wrapped - half).abs() + look.rot;
                (r * look.cos(ang), r * look.sin(ang), ang)
            } else if spiral {
                (
                    px * cr - py * sr,
                    px * sr + py * cr,
                    py.atan2(px) + look.rot,
                )
            } else {
                (px * cr - py * sr, px * sr + py * cr, 0.0)
            };
            let (s, q) = form_value(look, rx, ry, r, theta, flow, seed);
            // Sum and product forms spread differently; the gain keeps every form
            // in a comparable 0..1 range before the contrast shaping runs.
            let gain = match look.form {
                6 => 1.6,
                1 | 5 => 1.25,
                _ => 1.0,
            };
            let f01 = (0.5 + 0.5 * s * gain).clamp(0.0, 1.0);
            let e = ((f01 - 0.5) * (1.0 + 1.4 * contrast) + 0.5).clamp(0.0, 1.0);
            let ss = e * e * (3.0 - 2.0 * e);
            // DENSITY shifts the whole field, so ink coverage moves in every
            // dialect, not only the ones that threshold the field.
            let ink = (e + (ss - e) * contrast + look.bias).clamp(0.0, 1.0);
            *slot = [ink, (0.5 + 0.5 * q).clamp(0.0, 1.0)];
        }
    });
}

#[inline]
fn form_value(
    look: &Look,
    rx: f32,
    ry: f32,
    r: f32,
    theta: f32,
    flow: f32,
    seed: u64,
) -> (f32, f32) {
    // Radial and directional forms share one wavenumber so FREQ means the same
    // texture pitch everywhere; the cell form keeps its own lattice pitch.
    let [p0, p1, p2, _, _, p5] = look.ph;
    let k = look.freq * 1.9;
    match look.form {
        0 => {
            // The folded angle bends each sector's ring phase, so the sector
            // count is visible even though the rings themselves are radial.
            let rr = r * k + theta * 1.6 + p2 - flow * 0.19;
            (look.sin(rr), look.sin(rx * k * 0.42 + p0 + p5))
        }
        1 => {
            let c = look.cos(ry * k * 0.83 + p1);
            (
                look.sin(rx * k + p0 + flow * 0.31) * c,
                look.sin(r * k * 0.6 + p2 - flow * 0.19),
            )
        }
        2 => {
            let rr = r * k + p2;
            (
                look.sin(rr + theta * 4.0 - flow * 0.55),
                look.sin(theta * 3.0 + rr * 0.22),
            )
        }
        3 => {
            let lu = rx * k * 0.72 + p0;
            let lv = ry * k * 0.72 + p2;
            (
                (look.sin(lu).abs() + look.sin(lv).abs() - 1.0).clamp(-1.0, 1.0),
                look.sin(lu + lv + p5),
            )
        }
        4 => {
            let lu = rx * look.freq * 0.95 + p0;
            let lv = ry * look.freq * 0.95 + p1;
            let gu = lu.floor() as i32;
            let gv = lv.floor() as i32;
            // Ordering by squared distance is exact and saves seven square roots.
            let (mut n1, mut n2) = (1.0e9f32, 1.0e9f32);
            for dj in -1..=1 {
                for di in -1..=1 {
                    let (gx, gy) = (gu + di, gv + dj);
                    let hh = hash3(gx, gy, seed);
                    let jx = gx as f32 + 0.15 + (hh & 0xFFFF) as f32 * 1.0681e-5;
                    let jy = gy as f32 + 0.15 + ((hh >> 24) & 0xFFFF) as f32 * 1.0681e-5;
                    let d = (lu - jx) * (lu - jx) + (lv - jy) * (lv - jy);
                    // Branchless two nearest: the compare chain mispredicts on a
                    // per-cell basis, and min/max keeps nine candidates exact.
                    n2 = n2.min(d.max(n1));
                    n1 = n1.min(d);
                }
            }
            let edge = ((n2.sqrt() - n1.sqrt()) * 1.6).clamp(0.0, 1.0);
            (edge * 2.0 - 1.0, (n1.sqrt() * 2.0 - 1.0).clamp(-1.0, 1.0))
        }
        5 => {
            let m2 = (rx * look.moire_c + ry * look.moire_s) * k + look.ph[3];
            (
                look.sin(rx * k + p0 + flow * 0.31) * look.sin(m2),
                look.sin(ry * k * 0.5 + p5),
            )
        }
        6 => {
            let (a1, b1) = (rx - look.src1[0], ry - look.src1[1]);
            let (a2, b2) = (rx - look.src2[0], ry - look.src2[1]);
            let d1 = (a1 * a1 + b1 * b1).sqrt();
            let d2 = (a2 * a2 + b2 * b2).sqrt();
            (
                (look.sin(d1 * k - flow * 1.6) + look.sin(d2 * k - flow * 1.3)) * 0.5,
                look.sin(d1 * k * 0.8 - flow * 0.9),
            )
        }
        _ => (0.0, 0.0),
    }
}

fn paint_ground(grid: &mut Grid, field: &[[f32; 2]], w: usize, h: usize, look: &Look) {
    let gain = look.ground_gain * 31.0;
    shade_rows(grid, w, h, |(y, row): (usize, &mut Vec<Cell>)| {
        let base = y * w;
        for (x, cell) in row.iter_mut().enumerate().take(w) {
            let [f, q] = field[base + x];
            let band = ((q * 7.0) as usize).min(7);
            cell.ch = ' ';
            cell.fg = Color::Reset;
            cell.bg = look.bg_lut[band][((f * gain) as usize).min(31)];
        }
    });
}

fn paint_ink(grid: &mut Grid, field: &[[f32; 2]], w: usize, h: usize, look: &Look) {
    match look.dialect {
        1 => ink_stipple(grid, field, w, h, look),
        2 => ink_mosaic(grid, field, w, h, look),
        3 => ink_hatch(grid, field, w, h, look),
        4 => ink_weave(grid, field, w, h, look),
        _ => ink_engrave(grid, field, w, h, look),
    }
}

fn ink_engrave(grid: &mut Grid, field: &[[f32; 2]], w: usize, h: usize, look: &Look) {
    shade_rows(grid, w, h, |(y, row): (usize, &mut Vec<Cell>)| {
        let base = y * w;
        for (x, cell) in row.iter_mut().enumerate().take(w) {
            let [f, q] = field[base + x];
            cell.ch = ramp_char(look.ramp, f);
            cell.fg = look.fg_lut[((q * 7.0) as usize).min(7)][((f * 31.0) as usize).min(31)];
        }
    });
}

fn ink_stipple(grid: &mut Grid, field: &[[f32; 2]], w: usize, h: usize, look: &Look) {
    let dens = look.density.max(0.05);
    let threshold = 1.0 - dens;
    let seed = look.seed;
    shade_rows(grid, w, h, |(y, row): (usize, &mut Vec<Cell>)| {
        let base = y * w;
        for (x, cell) in row.iter_mut().enumerate().take(w) {
            let [f, q] = field[base + x];
            let cover = ((f - threshold) / dens).clamp(0.0, 1.0);
            if cover > 0.0 && hash01(x, y, seed) < cover * 0.62 {
                cell.ch = ramp_char(look.dots, 0.2 + 0.8 * f);
                cell.fg = look.fg_lut[((q * 7.0) as usize).min(7)][((f * 31.0) as usize).min(31)];
            }
        }
    });
}

fn ink_mosaic(grid: &mut Grid, field: &[[f32; 2]], w: usize, h: usize, look: &Look) {
    let top = look.ramp.len() - 1;
    shade_rows(grid, w, h, |(y, row): (usize, &mut Vec<Cell>)| {
        let base = y * w;
        for (x, cell) in row.iter_mut().enumerate().take(w) {
            let [f, q] = field[base + x];
            let band = ((q * 7.0) as usize).min(7);
            // Squaring the level keeps mid tones from posterizing into solid ink.
            cell.ch = look.ramp[((f * f * top as f32) as usize).min(top)];
            cell.fg = look.fg_lut[band][((0.35 + 0.65 * f) * 31.0) as usize];
            cell.bg = look.block_bg[band];
        }
    });
}

fn ink_hatch(grid: &mut Grid, field: &[[f32; 2]], w: usize, h: usize, look: &Look) {
    let threshold = 1.0 - look.density * 0.95;
    shade_rows(grid, w, h, |(y, row): (usize, &mut Vec<Cell>)| {
        let base = y * w;
        for (x, cell) in row.iter_mut().enumerate().take(w) {
            let [f, q] = field[base + x];
            if f <= threshold {
                continue;
            }
            let (dx, dy) = gradient(field, w, h, x, y);
            cell.ch = dir_char(look.line, dx, dy);
            cell.fg = look.fg_lut[((q * 7.0) as usize).min(7)][((0.4 + 0.6 * f) * 31.0) as usize];
        }
    });
}

fn ink_weave(grid: &mut Grid, field: &[[f32; 2]], w: usize, h: usize, look: &Look) {
    let strand = look.strand;
    shade_rows(grid, w, h, |(y, row): (usize, &mut Vec<Cell>)| {
        let base = y * w;
        for (x, cell) in row.iter_mut().enumerate().take(w) {
            let [f, q] = field[base + x];
            let a = (f * strand).fract();
            let b = (q * strand).fract();
            cell.ch = if a < 0.5 && b < 0.5 {
                look.line[4]
            } else if a < 0.5 {
                look.line[0]
            } else if b < 0.5 {
                look.line[1]
            } else {
                ' '
            };
            cell.fg = look.fg_lut[((a * 7.0) as usize).min(7)][((0.4 + 0.6 * q) * 31.0) as usize];
        }
    });
}

fn paint_detail(grid: &mut Grid, field: &[[f32; 2]], w: usize, h: usize, look: &Look) {
    match look.dialect {
        1 => detail_spark(grid, field, w, h, look),
        2 => detail_mortar(grid, field, w, h, look),
        3 => detail_cross(grid, field, w, h, look),
        4 => detail_strand(grid, field, w, h, look),
        _ => detail_contour(grid, field, w, h, look),
    }
}

fn detail_contour(grid: &mut Grid, field: &[[f32; 2]], w: usize, h: usize, look: &Look) {
    let edge = 0.09 - 0.05 * look.contrast;
    let bands = look.bands;
    shade_rows(grid, w, h, |(y, row): (usize, &mut Vec<Cell>)| {
        let base = y * w;
        for (x, cell) in row.iter_mut().enumerate().take(w) {
            let scaled = field[base + x][0] * bands;
            let frac = scaled - scaled.floor();
            if frac < edge || frac > 1.0 - edge {
                let (dx, dy) = gradient(field, w, h, x, y);
                cell.ch = dir_char(look.line, dx, dy);
                cell.fg = look.accent;
                cell.bg = look.deep;
            }
        }
    });
}

fn detail_spark(grid: &mut Grid, field: &[[f32; 2]], w: usize, h: usize, look: &Look) {
    let seed = look.seed ^ 0x51A7_0001;
    shade_rows(grid, w, h, |(y, row): (usize, &mut Vec<Cell>)| {
        let base = y * w;
        for (x, cell) in row.iter_mut().enumerate().take(w) {
            let f = field[base + x][0];
            if f < 0.34 && hash01(x, y, seed) < (0.34 - f) * 0.48 {
                cell.ch = ramp_char(look.dots, 0.2 + 1.6 * f);
                cell.fg = look.bg_lut[3][((f * 31.0) as usize).min(31)];
            }
        }
    });
}

fn detail_mortar(grid: &mut Grid, field: &[[f32; 2]], w: usize, h: usize, look: &Look) {
    shade_rows(grid, w, h, |(y, row): (usize, &mut Vec<Cell>)| {
        let base = y * w;
        for (x, cell) in row.iter_mut().enumerate().take(w) {
            let level = (field[base + x][0] * 9.0) as usize;
            let right = if x + 1 < w {
                (field[base + x + 1][0] * 9.0) as usize
            } else {
                level
            };
            let down = if y + 1 < h {
                (field[(y + 1) * w + x][0] * 9.0) as usize
            } else {
                level
            };
            if right != level || down != level {
                cell.bg = look.deep;
            }
        }
    });
}

fn detail_cross(grid: &mut Grid, field: &[[f32; 2]], w: usize, h: usize, look: &Look) {
    let threshold = 1.0 - look.density * 0.95;
    let hi = threshold + (1.0 - threshold) * 0.62;
    shade_rows(grid, w, h, |(y, row): (usize, &mut Vec<Cell>)| {
        let base = y * w;
        for (x, cell) in row.iter_mut().enumerate().take(w) {
            let [f, q] = field[base + x];
            if f > hi {
                let (dx, dy) = gradient(field, w, h, x, y);
                cell.ch = perp_char(look.line, dx, dy);
                cell.fg = look.fg_lut[((q * 7.0) as usize).min(7)]
                    [((0.5 + 0.5 * f) * 31.0).min(31.0) as usize];
            }
        }
    });
}

fn detail_strand(grid: &mut Grid, field: &[[f32; 2]], w: usize, h: usize, look: &Look) {
    let strand = look.strand;
    shade_rows(grid, w, h, |(y, row): (usize, &mut Vec<Cell>)| {
        let base = y * w;
        for (x, cell) in row.iter_mut().enumerate().take(w) {
            let [f, q] = field[base + x];
            let a = (f * strand).fract();
            let b = (q * strand).fract();
            if b < 0.5 {
                let level = ((f * 31.0) as usize).min(31);
                cell.fg = if a < 0.5 {
                    look.accent
                } else {
                    look.bg_lut[((a * 7.0) as usize).min(7)][level]
                };
            }
        }
    });
}

fn paint_dust(grid: &mut Grid, w: usize, h: usize, look: &Look) {
    let grain = look.grain;
    if grain <= 0.0 {
        return;
    }
    let seed = look.seed ^ 0xD057_1100;
    let rate = grain * 0.05;
    shade_rows(grid, w, h, |(y, row): (usize, &mut Vec<Cell>)| {
        for (x, cell) in row.iter_mut().enumerate().take(w) {
            if hash01(x, y, seed) < rate {
                cell.ch = DUST[((hash01(x, y, seed ^ 0x1234) * 5.0) as usize).min(4)];
                cell.fg = look.accent;
                cell.bg = look.deep;
            }
        }
    });
}

#[inline]
fn ramp_char(ramp: &[char], level: f32) -> char {
    let top = ramp.len() - 1;
    ramp[((level.clamp(0.0, 1.0) * top as f32) as usize).min(top)]
}

#[inline]
fn dir_char(line: [char; 5], dx: f32, dy: f32) -> char {
    let (ax, ay) = (dx.abs(), dy.abs());
    if ax > ay * 1.7 {
        line[0]
    } else if ay > ax * 1.7 {
        line[1]
    } else if dx * dy > 0.0 {
        line[2]
    } else {
        line[3]
    }
}

/// The cross stroke for a gradient direction, so dense regions become
/// cross-hatch instead of a flat fill.
#[inline]
fn perp_char(line: [char; 5], dx: f32, dy: f32) -> char {
    let (ax, ay) = (dx.abs(), dy.abs());
    if ax > ay * 1.2 {
        line[1]
    } else if ay > ax * 1.2 {
        line[0]
    } else if dx * dy > 0.0 {
        line[3]
    } else {
        line[2]
    }
}

#[inline]
fn gradient(field: &[[f32; 2]], w: usize, h: usize, x: usize, y: usize) -> (f32, f32) {
    let row = y * w;
    let left = field[row + x.saturating_sub(1)][0];
    let right = field[row + (x + 1).min(w - 1)][0];
    let up = field[y.saturating_sub(1) * w + x][0];
    let down = field[(y + 1).min(h - 1) * w + x][0];
    (right - left, down - up)
}

#[inline]
fn unit(mut n: u64) -> f32 {
    n = (n ^ (n >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    n = (n ^ (n >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    ((n ^ (n >> 31)) >> 40) as f32 / 16_777_216.0
}

#[inline]
fn hash3(x: i32, y: i32, seed: u64) -> u64 {
    let mut h = (x as i64 as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (y as i64 as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F)
        ^ seed.wrapping_mul(0x1656_67B1_9E37_79F9);
    h ^= h >> 29;
    h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h ^ (h >> 32)
}

#[inline]
fn hash01(x: usize, y: usize, seed: u64) -> f32 {
    (hash3(x as i32, y as i32, seed ^ 0x5EED_1234_ABCD_0001) >> 40) as f32 / 16_777_216.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::morph::IterateFrameRenderer;
    use crate::render::grid_to_plain;
    use rand::{rngs::StdRng, SeedableRng};

    const W: usize = 110;
    const H: usize = 36;

    fn knobs(form: usize, dialect: usize) -> Vec<f32> {
        let mut k: Vec<f32> = PARAMS.iter().map(|p| p.default).collect();
        k[0] = form as f32;
        k[1] = dialect as f32;
        k
    }

    fn frame(seed: u64, time: f32, values: &[f32]) -> Grid {
        let mut grid = vec![vec![Cell::blank(); W]; H];
        let palette = crate::color::make_palette(seed);
        let mut rng = StdRng::seed_from_u64(seed);
        MODE.render(&mut ModeFrame {
            grid: &mut grid,
            width: W,
            height: H,
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

    fn diff(a: &Grid, b: &Grid) -> usize {
        a.iter()
            .flatten()
            .zip(b.iter().flatten())
            .filter(|(x, y)| x.ch != y.ch)
            .count()
    }

    #[test]
    fn prismata_snapshots_and_deterministic_motion() {
        let k = knobs(0, 0);
        let still = frame(42, 0.0, &k);
        let moving = frame(42, 5.0, &k);
        insta::assert_snapshot!("prismata_seed42", text(&still));
        insta::assert_snapshot!("prismata_seed42_t5", text(&moving));
        assert_ne!(still, moving);
        assert_eq!(still, frame(42, 0.0, &k));
        assert_ne!(still, frame(43, 0.0, &k));
        for i in 0..KNOBS {
            let param = &PARAMS[i];
            for probe in [param.max, param.min] {
                if probe == param.default {
                    continue;
                }
                let mut changed = k.clone();
                changed[i] = probe;
                assert_ne!(moving, frame(42, 5.0, &changed), "{}{probe}", param.key);
            }
        }
    }

    #[test]
    fn prismata_forms_and_dialects_are_distinct() {
        let frames: Vec<Grid> = (0..7)
            .flat_map(|f| (0..5).map(move |d| (f, d)))
            .map(|(f, d)| frame(42, 1.5, &knobs(f, d)))
            .collect();
        for i in 0..frames.len() {
            for j in 0..i {
                let n = diff(&frames[i], &frames[j]);
                assert!(
                    n > 700,
                    "combo {}/{}: only {n} differing glyphs",
                    i,
                    j
                );
            }
        }
        let forms = (0..7)
            .map(|f| format!("form {f}\n{}", text(&frame(42, 1.5, &knobs(f, 0)))))
            .collect::<Vec<_>>()
            .join("\n\n");
        insta::assert_snapshot!("prismata_form_gallery", forms);
        let dialects = (0..5)
            .map(|d| format!("dialect {d}\n{}", text(&frame(42, 1.5, &knobs(3, d)))))
            .collect::<Vec<_>>()
            .join("\n\n");
        insta::assert_snapshot!("prismata_dialect_gallery", dialects);
    }

    #[test]
    fn prismata_random_knob_rolls_land_in_distinct_worlds() {
        // The demo's randomize mode samples every knob with `rand_knob`, so a roll
        // must change the picture, not just its shading.
        let frames: Vec<Grid> = (1..=12u64)
            .map(|seed| {
                let values: Vec<f32> = PARAMS.iter().map(|p| crate::opts::rand_knob(seed, p)).collect();
                frame(seed, 1.5, &values)
            })
            .collect();
        for i in 0..frames.len() {
            for j in 0..i {
                let n = diff(&frames[i], &frames[j]);
                assert!(n > 700, "rolls {i}/{j}: only {n} differing glyphs");
            }
        }
    }

    #[test]
    fn prismata_boundaries_and_hostile_inputs() {
        let palette = crate::color::make_palette(42);
        let extremes = [
            PARAMS.iter().map(|p| p.min).collect::<Vec<_>>(),
            PARAMS.iter().map(|p| p.max).collect(),
            vec![f32::NAN; KNOBS],
            vec![f32::INFINITY; KNOBS],
            vec![-1.0e9; KNOBS],
        ];
        for (w, h) in [(0, 0), (0, 3), (3, 0), (1, 1), (2, 9), (9, 2), (61, 17)] {
            for values in &extremes {
                for time in [0.0, 7.0, -100.0, f32::MIN, f32::MAX, f32::NAN, f32::INFINITY] {
                    let mut grid = vec![vec![Cell::blank(); w]; h];
                    let mut rng = StdRng::seed_from_u64(42);
                    MODE.render(&mut ModeFrame {
                        grid: &mut grid,
                        width: w,
                        height: h,
                        seed: u64::MAX,
                        palette: &palette,
                        rng: &mut rng,
                        time,
                        args: &[],
                        param_values: Some(values),
                    });
                    assert_eq!(grid.len(), h);
                    assert!(grid.iter().all(|r| r.len() == w));
                    assert!(
                        grid.iter()
                            .flatten()
                            .all(|c| crate::types::char_width(c.ch) == 1)
                    );
                }
            }
        }
        // Positional arguments reach the same knobs, and a NaN clock is a still frame.
        let mut grid = vec![vec![Cell::blank(); 24]; 8];
        let mut rng = StdRng::seed_from_u64(7);
        let args: Vec<String> = ["ascii-renderer", "7", "prismata", "moss", "4", "4", "2", "3"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        MODE.render(&mut ModeFrame {
            grid: &mut grid,
            width: 24,
            height: 8,
            seed: 7,
            palette: &palette,
            rng: &mut rng,
            time: 0.0,
            args: &args,
            param_values: None,
        });
        let positional = grid.clone();
        let k = knobs(4, 4);
        let mut reference = vec![vec![Cell::blank(); 24]; 8];
        let mut rng = StdRng::seed_from_u64(7);
        let mut values = k.clone();
        values[2] = 2.0;
        values[3] = 3.0;
        MODE.render(&mut ModeFrame {
            grid: &mut reference,
            width: 24,
            height: 8,
            seed: 7,
            palette: &palette,
            rng: &mut rng,
            time: 0.0,
            args: &[],
            param_values: Some(&values),
        });
        assert_eq!(positional, reference);
        let still = frame(42, f32::NAN, &knobs(0, 0));
        assert_eq!(still, frame(42, 0.0, &knobs(0, 0)));
        let mut renderer = IterateFrameRenderer::new(NAME, 42, "deep", W, H).unwrap();
        assert!(renderer.render(3.0, Some(&knobs(6, 4))).is_some());
    }
}
