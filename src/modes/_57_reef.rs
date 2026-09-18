//! reef: coral colonies grown off a sand floor in four silhouettes, under a
//! caustic water column with sun shafts, eelgrass, bubbles and schooling fish.
use crate::_0_profile::measure_layer;
use crate::color::{darken, hsl_to_rgb, lerp_color, lighten};
use crate::opts::param_f32;
use crate::registry::{AnimKind, Mode, ModeFrame, Param};
use crate::types::{Cell, Grid};
use crossterm::style::Color;
use rand::RngExt;
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::cell::RefCell;
use std::f32::consts::TAU;
use std::sync::LazyLock;

pub(super) struct Reef;
pub(super) static MODE: Reef = Reef;

const NAME: &str = "reef";
const KNOBS: usize = 11;
const HELP: &str = "reef: DLA coral colonies under a caustic water column, bubbles and fish [colonies] [walkers] [stickiness] [depth] [bubbles] [fish] [spread] [caustics] [grow] [hue] [speed]";

/// Per-silhouette glyph ramp, thin tip first, dense trunk last.
const RAMP_FAN: [char; 6] = ['.', ':', '*', '%', '#', '&'];
const RAMP_STAG: [char; 6] = ['.', ':', 'o', 'O', '@', '#'];
const RAMP_BRAIN: [char; 6] = ['.', '~', '=', 'o', '@', '&'];
const RAMP_TUBE: [char; 6] = ['.', 'o', 'O', '|', '#', '&'];

const SAND: [char; 3] = ['.', ':', '~'];
const CAUSTIC: [char; 3] = ['-', '~', '='];
const MOTE: [char; 2] = ['.', '`'];
const BLADE: [char; 4] = ['|', '/', '\\', ')'];
const ROCK: [char; 3] = ['o', 'O', '&'];
const FISH_R: [&str; 4] = ["><>", "><o>", "><=>", "><((>"];
const FISH_L: [&str; 4] = ["<><", "<o><", "<=><", "<))><"];
const RAY: [&str; 3] = ["  ,-~~~-.  ", "<(  o     >", "  `-~~~-'  "];
const FISH_HUES: [f64; 5] = [45.0, 200.0, 15.0, 320.0, 60.0];
const COLONY_HUES: [f64; 4] = [348.0, 26.0, 288.0, 52.0];

const L_COLONY: u64 = 0x21;
const L_BUBBLE: u64 = 0x22;
const L_FISH: u64 = 0x23;
const L_SHAFT: u64 = 0x24;
const L_FLOOR: u64 = 0x25;
const L_MOTE: u64 = 0x26;
const L_GRASS: u64 = 0x27;
const L_ROCK: u64 = 0x28;
const L_SCHOOL: u64 = 0x29;

const WAVE_N: usize = 2048;

const PARAMS: &[Param] = &[
    param!("COLONIES", "coral colonies", 1.0, 12.0, 4.0, 1.0),
    param!("WALKERS", "dla walker budget", 40.0, 12000.0, 1400.0, 40.0),
    param!("STICK", "stickiness", 0.02, 1.0, 0.5, 0.02),
    param!("DEPTH", "water column depth", 0.35, 0.98, 0.86, 0.02),
    param!("BUBBLES", "bubble density", 0.0, 3.0, 0.7, 0.05),
    param!("FISH", "fish per 80 columns", 0.0, 40.0, 7.0, 1.0),
    param!("SPREAD", "colony spread", 0.2, 1.0, 0.86, 0.02),
    param!("CAUST", "caustics and shafts", 0.0, 1.5, 0.7, 0.05),
    param!("GROW", "colony size", 0.1, 1.5, 1.0, 0.05),
    param!("HUE", "hue rotation deg", -180.0, 180.0, 0.0, 10.0),
    param!("SPEED", "time scale", 0.0, 3.0, 1.0, 0.1),
];

impl Mode for Reef {
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
        let k = ReefKnobs::from_slice(&p);
        draw_reef(
            frame.grid,
            frame.width,
            frame.height,
            frame.seed,
            frame.palette,
            frame.time,
            &k,
        );
    }
}

/// One period of sine. The caustic pass touches every water cell twice, so the
/// table replaces two libm calls per cell with a multiply, a mask and a load.
static WAVE: LazyLock<[f32; WAVE_N]> =
    LazyLock::new(|| std::array::from_fn(|i| (i as f32 * TAU / WAVE_N as f32).sin()));

#[inline(always)]
fn wsin(x: f32) -> f32 {
    let i = (x * (WAVE_N as f32 / TAU)) as i32 as usize & (WAVE_N - 1);
    WAVE[i]
}

fn mix64(seed: u64, layer: u64, idx: u64, salt: u64) -> u64 {
    let mut x = seed
        ^ layer.wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ idx.wrapping_add(1).wrapping_mul(0xBF58_476D_1CE4_E5B9)
        ^ salt.wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^= x >> 30;
    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

/// Side stream for one indexed element: never the main rng, so adding a bubble
/// cannot shift a fish.
fn h01(seed: u64, layer: u64, idx: u64, salt: u64) -> f32 {
    (mix64(seed, layer, idx, salt) & 0xFF_FFFF) as f32 / 16_777_216.0
}

/// Branch age from subtree support count: one glyph step per doubling or so,
/// so trunks land on the dense end of a ramp and tips on the thin end.
fn level_of(weight: u32) -> usize {
    match weight {
        1 => 0,
        2..=3 => 1,
        4..=8 => 2,
        9..=22 => 3,
        23..=64 => 4,
        _ => 5,
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Form {
    Fan,
    Stag,
    Brain,
    Tube,
}

impl Form {
    fn ramp(self) -> &'static [char; 6] {
        match self {
            Form::Fan => &RAMP_FAN,
            Form::Stag => &RAMP_STAG,
            Form::Brain => &RAMP_BRAIN,
            Form::Tube => &RAMP_TUBE,
        }
    }
    /// Column and row stretch applied to the colony's slot, so a stag reads tall
    /// and narrow next to a wide flat fan.
    fn shape(self) -> (f32, f32) {
        match self {
            Form::Fan => (1.18, 0.94),
            Form::Stag => (0.74, 1.34),
            Form::Brain => (0.92, 0.96),
            Form::Tube => (0.82, 1.12),
        }
    }
}

const FORMS: [Form; 4] = [Form::Fan, Form::Stag, Form::Brain, Form::Tube];

#[derive(Clone, PartialEq)]
struct ReefKnobs {
    colony_count: f32,
    walker_budget: f32,
    stickiness: f32,
    water_depth_frac: f32,
    bubble_density: f32,
    fish_count: f32,
    spread: f32,
    caustics: f32,
    grow: f32,
    hue: f32,
    speed: f32,
}

impl ReefKnobs {
    fn from_slice(p: &[f32; KNOBS]) -> Self {
        ReefKnobs {
            colony_count: p[0],
            walker_budget: p[1],
            stickiness: p[2],
            water_depth_frac: p[3],
            bubble_density: p[4],
            fish_count: p[5],
            spread: p[6],
            caustics: p[7],
            grow: p[8],
            hue: p[9],
            speed: p[10],
        }
    }
}

#[derive(Clone, Copy)]
struct Coral {
    x: i32,
    y: i32,
    ch: char,
    hue: f64,
    tip: f32,
    phase: f32,
}

#[derive(Clone, Copy)]
struct Tuft {
    x: i32,
    base: i32,
    tall: i32,
    lean: f32,
    phase: f32,
}

struct Bed {
    key: (u64, usize, usize, ReefKnobs),
    floor: Vec<i32>,
    corals: Vec<Coral>,
    tufts: Vec<Tuft>,
    rocks: Vec<(i32, i32, usize)>,
    vents: Vec<(i32, i32)>,
}

thread_local! {
    static BED: RefCell<Option<Bed>> = const { RefCell::new(None) };
}

fn seabed_profile(w: usize, h: usize, seed: u64, k: &ReefKnobs) -> Vec<i32> {
    let base = (h as f32 * k.water_depth_frac).round();
    let a1 = h01(seed, L_FLOOR, 0, 1) * TAU;
    let a2 = h01(seed, L_FLOOR, 0, 2) * TAU;
    let relief = (h as f32 * 0.045).clamp(1.0, 5.0);
    let lo = (h as f32 * 0.35) as i32;
    let hi = h as i32 - 1;
    (0..w)
        .map(|x| {
            let fx = x as f32;
            let bump = wsin(fx * 0.085 + a1) * 0.62 + wsin(fx * 0.027 + a2) * 0.44;
            (base + bump * relief).round().clamp(lo as f32, hi as f32) as i32
        })
        .collect()
}

struct Colony {
    cx: i32,
    cy: i32,
    ax: f32,
    ay: f32,
    walkers: usize,
    cells: usize,
    form: Form,
}

/// One grown cell: grid position, ramp step and how close to a tip it sits.
#[derive(Clone, Copy)]
struct Sprig {
    x: i32,
    y: i32,
    level: usize,
    tip: f32,
}

fn free(owner: &[i32], w: usize, h: usize, x: i32, y: i32) -> bool {
    x >= 1 && y >= 1 && (x as usize) < w - 1 && (y as usize) < h && owner[y as usize * w + x as usize] < 0
}

fn claim(owner: &mut [i32], w: usize, x: i32, y: i32, id: i32) {
    owner[y as usize * w + x as usize] = id;
}

/// Rim DLA: walkers launch just outside the colony's current reach, and
/// attachment is weighted toward the live tips and away from the buried base.
fn grow_dla(
    w: usize,
    h: usize,
    owner: &mut [i32],
    slot: &mut [i32],
    floor: &[i32],
    c: &Colony,
    stick: f32,
    seed: u64,
    tag: i32,
    rng: &mut StdRng,
) -> Vec<Sprig> {
    let ax = c.ax.max(2.0);
    let ay = c.ay.max(2.0);
    let stag = c.form == Form::Stag;
    let mut cells: Vec<(i32, i32)> = Vec::new();
    let mut parents: Vec<usize> = Vec::new();
    let mut place = |owner: &mut [i32],
                     slot: &mut [i32],
                     cells: &mut Vec<(i32, i32)>,
                     parents: &mut Vec<usize>,
                     x: i32,
                     y: i32,
                     parent: usize| {
        if !free(owner, w, h, x, y) {
            return None;
        }
        let id = cells.len();
        claim(owner, w, x, y, tag);
        slot[y as usize * w + x as usize] = id as i32;
        cells.push((x, y));
        parents.push(parent);
        Some(id)
    };
    let stems = if stag {
        (ax * 0.55).round().clamp(1.0, 3.0) as i32
    } else {
        1
    };
    for st in 0..stems {
        let spread = if stems > 1 {
            (st as f32 / (stems - 1).max(1) as f32 - 0.5) * ax * 1.15
        } else {
            0.0
        };
        let x = (c.cx as f32 + spread).round() as i32;
        let y = (floor[x.clamp(0, w as i32 - 1) as usize] - 1).min(c.cy).max(1);
        let Some(root) = place(owner, slot, &mut cells, &mut parents, x, y, 0) else {
            continue;
        };
        let tall = (ay * if stag { 0.20 } else { 0.16 }).round().max(1.0) as i32;
        let mut up = root;
        for j in 1..=tall {
            match place(owner, slot, &mut cells, &mut parents, x, y - j, up) {
                Some(id) => up = id,
                None => break,
            }
        }
    }
    if !stag {
        let arc = (ax * 0.40).round() as i32;
        for step in 1..=arc {
            for side in 0..2 {
                let x = c.cx + if side == 0 { -step } else { step };
                let y = (floor[x.clamp(0, w as i32 - 1) as usize] - 1).min(c.cy).max(1);
                place(owner, slot, &mut cells, &mut parents, x, y, 0);
            }
        }
    }
    if cells.is_empty() {
        return Vec::new();
    }
    let coord: [f32; 5] = if stag {
        [0.0, 1.0, 0.38, 0.13, 0.04]
    } else {
        [0.0, 1.0, 0.62, 0.26, 0.10]
    };
    let gain = if stag { 0.85 } else { 1.25 };
    let arc_span = if stag { 0.66 } else { 0.92 };
    let steps = ((ax + ay) * 12.0).clamp(120.0, 1400.0) as i32;
    let mut rmax: f32 = if stag { 0.40 } else { 0.24 };

    for _ in 0..c.walkers {
        if cells.len() >= c.cells {
            break;
        }
        let rs = (rmax + 0.20).min(1.0);
        let ang = std::f32::consts::FRAC_PI_2
            + (rng.random::<f32>() - 0.5) * std::f32::consts::PI * arc_span;
        let mut px = c.cx as f32 + ang.cos() * rs * ax;
        let mut py = c.cy as f32 - ang.sin() * rs * ay;
        let mut hit: Option<usize> = None;
        for _ in 0..steps {
            let (ix, iy) = (px.round() as i32, py.round() as i32);
            let mut found = None;
            let mut kin = 0usize;
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    let (nx, ny) = (ix + dx, iy + dy);
                    if nx < 0 || ny < 0 || nx as usize >= w || ny as usize >= h {
                        continue;
                    }
                    let cell = ny as usize * w + nx as usize;
                    if owner[cell] == tag {
                        found = Some(slot[cell] as usize);
                        kin += 1;
                    }
                }
            }
            if let Some(parent) = found {
                let ndx = (ix - c.cx) as f32 / ax;
                let ndy = (iy - c.cy) as f32 / ay;
                let nr = (ndx * ndx + ndy * ndy).sqrt();
                let rise = ((c.cy - iy) as f32 / ay).clamp(0.0, 1.0);
                let radial = (nr / rmax.max(0.30)).min(1.25);
                let tipward = (0.34 + 0.66 * rise) * (0.45 + 0.85 * radial);
                let grip = (stick * coord[kin.min(4)] * tipward * gain).min(1.0);
                if rng.random::<f32>() < grip {
                    hit = Some(parent);
                    break;
                }
            }
            let r = rng.random::<f32>();
            let (dx, dy) = if r < 0.30 {
                (-1.0f32, 0.0f32)
            } else if r < 0.60 {
                (1.0, 0.0)
            } else if r < 0.76 {
                (0.0, -1.0)
            } else {
                (0.0, 1.0)
            };
            let nx = (px + dx).clamp(1.0, w as f32 - 2.0);
            let col = nx.round() as i32;
            let fl = floor[col as usize];
            let ny = (py + dy).clamp(1.0, (fl - 1).max(1) as f32);
            if owner[ny.round() as i32 as usize * w + col as usize] < 0 {
                px = nx;
                py = ny;
            }
            let ndx = (px - c.cx as f32) / ax;
            let ndy = (py - c.cy as f32) / ay;
            if ndx * ndx + ndy * ndy > 5.5 {
                break;
            }
        }
        let Some(parent) = hit else { continue };
        let (ix, iy) = (px.round() as i32, py.round() as i32);
        let ndx = (ix - c.cx) as f32 / ax;
        let ndy = (iy - c.cy) as f32 / ay;
        let nr = (ndx * ndx + ndy * ndy).sqrt();
        let lift = (-ndy).clamp(0.0, 1.0);
        let waist = if stag { 0.24 + 0.70 * lift } else { 0.30 + 0.82 * lift };
        let ragged = 0.74 + 0.34 * h01(seed, L_COLONY, (tag as u64) << 20 | ix as u64, 51);
        if nr > ragged || ndx.abs() > waist {
            continue;
        }
        let (px, py) = cells[parent];
        if place(owner, slot, &mut cells, &mut parents, ix, iy, parent).is_some() {
            if px != ix && py != iy {
                place(owner, slot, &mut cells, &mut parents, ix, py, parent);
            }
            rmax = rmax.max(nr);
        }
    }
    let mut fill: Vec<(i32, i32, usize)> = Vec::new();
    for i in 0..cells.len() {
        let (x, y) = cells[i];
        for (dx, dy) in [(0i32, -1i32), (-1, 0), (1, 0), (0, 1)] {
            let (nx, ny) = (x + dx, y + dy);
            if !free(owner, w, h, nx, ny) || ny >= floor[nx.clamp(0, w as i32 - 1) as usize] {
                continue;
            }
            let mut kin = 0usize;
            for ey in -1i32..=1 {
                for ex in -1i32..=1 {
                    let (ax2, ay2) = (nx + ex, ny + ey);
                    if (ex != 0 || ey != 0)
                        && ax2 >= 0
                        && ay2 >= 0
                        && (ax2 as usize) < w
                        && (ay2 as usize) < h
                        && owner[ay2 as usize * w + ax2 as usize] == tag
                    {
                        kin += 1;
                    }
                }
            }
            if kin >= if stag { 5 } else { 4 } {
                fill.push((nx, ny, i));
            }
        }
    }
    for (x, y, parent) in fill {
        place(owner, slot, &mut cells, &mut parents, x, y, parent);
    }
    let len = cells.len();
    let mut weight = vec![1u32; len];
    for i in (1..len).rev() {
        let p = parents[i];
        if p < i {
            weight[p] += weight[i];
        }
    }
    let root = (weight[0] as f32).ln().max(1.0);
    (0..len)
        .map(|i| {
            let (x, y) = cells[i];
            let rise = ((c.cy - y) as f32 / ay.max(1.0)).clamp(0.0, 1.0);
            let slim = 1.0 - (weight[i] as f32).ln() / root;
            let mut kin = 0f32;
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    let (nx, ny) = (x + dx, y + dy);
                    if (dx != 0 || dy != 0)
                        && nx >= 0
                        && ny >= 0
                        && (nx as usize) < w
                        && (ny as usize) < h
                        && owner[ny as usize * w + nx as usize] == tag
                    {
                        kin += 1.0;
                    }
                }
            }
            let dense = kin / 8.0;
            let trunk = level_of(weight[i]) as f32 / 5.0;
            let level = ((dense * 0.70 + trunk * 0.52) * 5.0).round().clamp(0.0, 5.0) as usize;
            Sprig {
                x,
                y,
                level,
                tip: (slim * 0.62 + rise * 0.38).clamp(0.0, 1.0),
            }
        })
        .collect()
}

/// A brain dome: a solid half ellipse cut by meandering grooves, so the ramp
/// reads as ridges rather than as branch thickness.
fn grow_brain(
    w: usize,
    h: usize,
    owner: &mut [i32],
    floor: &[i32],
    c: &Colony,
    seed: u64,
    idx: usize,
    tag: i32,
) -> Vec<Sprig> {
    let ax = c.ax.max(2.0);
    let ay = c.ay.max(2.0);
    let wob = h01(seed, L_COLONY, idx as u64, 23) * TAU;
    let mut out = Vec::new();
    for dy in 0..=(ay.round() as i32) {
        let y = c.cy - dy;
        if y < 1 {
            break;
        }
        let fy = dy as f32 / ay;
        let half = ax * (1.0 - fy * fy).max(0.0).sqrt();
        let lo = (c.cx as f32 - half).round() as i32;
        let hi = (c.cx as f32 + half).round() as i32;
        for x in lo..=hi {
            if !free(owner, w, h, x, y) || y >= floor[x.clamp(0, w as i32 - 1) as usize] {
                continue;
            }
            claim(owner, w, x, y, tag);
            let fx = (x - c.cx) as f32;
            let groove = wsin(fx * 0.34 + wsin(dy as f32 * 0.55 + wob) * 1.9 + wob);
            let rim = 1.0 - ((fx / ax).abs().max(fy)).clamp(0.0, 1.0);
            let level = if groove.abs() < 0.30 {
                1 + usize::from(groove < 0.0)
            } else if rim > 0.42 {
                5
            } else {
                4
            };
            out.push(Sprig {
                x,
                y,
                level,
                tip: (fy * 0.7 + 0.3 * (1.0 - rim)).clamp(0.0, 1.0),
            });
        }
    }
    out
}

/// A cluster of tube sponges: vertical pipes of mixed height with open mouths.
fn grow_tube(
    w: usize,
    h: usize,
    owner: &mut [i32],
    floor: &[i32],
    c: &Colony,
    seed: u64,
    idx: usize,
    tag: i32,
) -> Vec<Sprig> {
    let ax = c.ax.max(2.0);
    let ay = c.ay.max(2.0);
    let tubes = (ax * 0.75).round().clamp(2.0, 9.0) as usize;
    let mut out = Vec::new();
    for tb in 0..tubes {
        let u = tb as f32 / (tubes - 1).max(1) as f32;
        let wiggle = (h01(seed, L_COLONY, (idx * 16 + tb) as u64, 41) - 0.5) * 0.6;
        let lean = (u - 0.5) * 2.0 + wiggle;
        let x = (c.cx as f32 + lean * ax * 0.92).round() as i32;
        let tall = (ay * (0.42 + h01(seed, L_COLONY, (idx * 16 + tb) as u64, 31) * 0.58)
            * (1.0 - lean.abs() * 0.34))
            .round()
            .max(2.0) as i32;
        let wide = if h01(seed, L_COLONY, (idx * 16 + tb) as u64, 37) < 0.42 { 1 } else { 0 };
        for col in 0..=wide {
            let cx = x + col;
            for j in 0..=tall {
                let y = c.cy - j;
                if !free(owner, w, h, cx, y) || y >= floor[cx.clamp(0, w as i32 - 1) as usize] {
                    continue;
                }
                claim(owner, w, cx, y, tag);
                let f = j as f32 / tall as f32;
                let level = if j == tall {
                    1
                } else if j + 1 == tall {
                    2
                } else if f > 0.3 {
                    3
                } else if f > 0.12 {
                    4
                } else {
                    5
                };
                out.push(Sprig {
                    x: cx,
                    y,
                    level,
                    tip: f.clamp(0.0, 1.0),
                });
            }
        }
    }
    out
}

fn build_bed(w: usize, h: usize, seed: u64, k: &ReefKnobs) -> Bed {
    let floor = seabed_profile(w, h, seed, k);
    let mut corals: Vec<Coral> = Vec::new();
    let n = (k.colony_count * (w as f32 / 80.0))
        .round()
        .clamp(1.0, 110.0) as usize;
    let deep = *floor.iter().max().unwrap_or(&(h as i32));
    let column = (deep as f32 - 1.0).max(3.0);
    let inner = w as f32 * k.spread;
    let left = (w as f32 - inner) * 0.5;
    let slot = inner / n as f32;
    let mut owner: Vec<i32> = vec![-1; w * h];
    let mut slots: Vec<i32> = vec![-1; w * h];
    let turn = (mix64(seed, L_COLONY, 0, 17) % FORMS.len() as u64) as usize;
    for c in 0..n {
        let mut rng = StdRng::seed_from_u64(mix64(seed, L_COLONY, c as u64, 0));
        let form = FORMS[(c + turn) % FORMS.len()];
        let jitter = (h01(seed, L_COLONY, c as u64, 3) - 0.5) * slot * 0.5;
        let cx = (left + (c as f32 + 0.5) * slot + jitter).round() as i32;
        let cx = cx.clamp(2, w as i32 - 3);
        let cy = floor[cx as usize] - 1;
        let bulk = (0.90 + h01(seed, L_COLONY, c as u64, 5) * 0.70) * k.grow;
        let tall = 0.46 + h01(seed, L_COLONY, c as u64, 11) * 0.42;
        let room = ((cx - 1).min(w as i32 - 2 - cx)).max(2) as f32;
        let (sx, sy) = form.shape();
        let available = room.min(slot * 0.58);
        let ax = (slot * 0.40 * bulk * sx).clamp(3.0_f32.min(available), available);
        let ay = (ax * tall * sy).clamp(2.5, column * 0.86);
        let cells = ((ax * ay * 1.20).round() as usize).clamp(6, 4200);
        let effort = k.walker_budget / 1400.0;
        let walkers = (cells as f32 * 14.0 * effort) as usize;
        let walkers = walkers.clamp(24, 160_000 / n.max(1)).max(24);
        let colony = Colony {
            cx,
            cy,
            ax,
            ay,
            walkers,
            cells,
            form,
        };
        let tag = c as i32;
        let sprigs = match form {
            Form::Brain => grow_brain(w, h, &mut owner, &floor, &colony, seed, c, tag),
            Form::Tube => grow_tube(w, h, &mut owner, &floor, &colony, seed, c, tag),
            _ => grow_dla(
                w,
                h,
                &mut owner,
                &mut slots,
                &floor,
                &colony,
                k.stickiness,
                seed,
                tag,
                &mut rng,
            ),
        };
        if sprigs.is_empty() {
            continue;
        }
        let hue = COLONY_HUES[c % COLONY_HUES.len()]
            + (c / COLONY_HUES.len()) as f64 * 37.0
            + k.hue as f64
            + (h01(seed, L_COLONY, c as u64, 7) as f64 - 0.5) * 16.0;
        let hue = hue.rem_euclid(360.0);
        let phase = h01(seed, L_COLONY, c as u64, 9) * TAU;
        let ramp = form.ramp();
        for s in sprigs {
            corals.push(Coral {
                x: s.x,
                y: s.y,
                ch: ramp[s.level.min(5)],
                hue,
                tip: s.tip,
                phase,
            });
        }
    }
    let tufts = grass_tufts(w, h, seed, &floor, &owner);
    let rocks = floor_rocks(w, seed, &floor);
    let vents = floor_vents(w, seed, &floor, k);
    Bed {
        key: (seed, w, h, k.clone()),
        floor,
        corals,
        tufts,
        rocks,
        vents,
    }
}

fn grass_tufts(w: usize, h: usize, seed: u64, floor: &[i32], owner: &[i32]) -> Vec<Tuft> {
    let count = ((w as f32 / 9.0).round() as usize).clamp(2, 60);
    let mut out = Vec::new();
    for i in 0..count {
        let gx = (h01(seed, L_GRASS, i as u64, 1) * (w as f32 - 2.0)) as i32 + 1;
        let base = floor[gx.clamp(0, w as i32 - 1) as usize] - 1;
        if base < 2 {
            continue;
        }
        if owner[(base as usize) * w + gx as usize] >= 0 {
            continue;
        }
        let blades = 2 + (mix64(seed, L_GRASS, i as u64, 2) % 3) as i32;
        for b in 0..blades {
            let bx = gx + b - blades / 2;
            if bx < 1 || bx as usize >= w - 1 {
                continue;
            }
            let idx = (i * 8 + b as usize) as u64;
            let tall = 2 + (h01(seed, L_GRASS, idx, 3) * 4.4) as i32;
            if base - tall < 1 || base as usize >= h {
                continue;
            }
            out.push(Tuft {
                x: bx,
                base,
                tall,
                lean: (h01(seed, L_GRASS, idx, 4) - 0.5) * 1.6,
                phase: h01(seed, L_GRASS, idx, 5) * TAU,
            });
        }
    }
    out
}

fn floor_rocks(w: usize, seed: u64, floor: &[i32]) -> Vec<(i32, i32, usize)> {
    let count = ((w as f32 / 16.0).round() as usize).clamp(2, 30);
    (0..count)
        .filter_map(|i| {
            let x = (h01(seed, L_ROCK, i as u64, 1) * (w as f32 - 2.0)) as i32 + 1;
            let y = floor[x.clamp(0, w as i32 - 1) as usize];
            let kind = (mix64(seed, L_ROCK, i as u64, 2) % ROCK.len() as u64) as usize;
            (y >= 1).then_some((x, y, kind))
        })
        .collect()
}

fn floor_vents(w: usize, seed: u64, floor: &[i32], k: &ReefKnobs) -> Vec<(i32, i32)> {
    if k.bubble_density <= 0.0 {
        return Vec::new();
    }
    let count = 2 + (mix64(seed, L_BUBBLE, 0, 9) % 2) as usize;
    (0..count)
        .map(|i| {
            let f = (i as f32 + 0.5) / count as f32;
            let jitter = (h01(seed, L_BUBBLE, i as u64, 7) - 0.5) * 0.5;
            let x = ((f + jitter / count as f32) * w as f32).clamp(1.0, w as f32 - 2.0) as i32;
            (x, floor[x as usize] - 1)
        })
        .collect()
}

/// Water column colors per row: blue-green near the surface falling to a dark
/// navy at the floor, tinted toward the theme background so a palette reads.
fn water_rows(h: usize, deep: i32, palette: &[Color; 5]) -> Vec<Color> {
    let span = (deep.max(1)) as f32;
    (0..h)
        .map(|y| {
            let f = (y as f32 / span).clamp(0.0, 1.25);
            let hue = 172.0 + 56.0 * f as f64;
            let sat = 0.50 + 0.30 * f as f64;
            let lum = (0.255 - 0.215 * f as f64).max(0.026);
            lerp_color(hsl_to_rgb(hue.min(232.0), sat, lum), palette[0], 0.28)
        })
        .collect()
}

fn paint_water(
    grid: &mut Grid,
    w: usize,
    h: usize,
    seed: u64,
    floor: &[i32],
    rows: &[Color],
    t: f32,
    speed: f32,
) {
    let drift = (t * speed * 0.9) as i32;
    for y in 0..h.min(grid.len()) {
        let bg = rows[y];
        let mote_fg = lighten(bg, 34);
        let row = &mut grid[y];
        for x in 0..w.min(row.len()) {
            let fl = floor[x.min(floor.len() - 1)];
            if (y as i32) >= fl {
                row[x] = Cell::with_bg(' ', bg, bg);
                continue;
            }
            let mut cell = Cell::with_bg(' ', bg, bg);
            let yy = (y as i32 + drift) as usize;
            if (x ^ yy.wrapping_mul(3)) & 7 == 0 {
                let m = mix64(seed, L_MOTE, (yy * w + x) as u64, 11);
                if m % 17 == 0 {
                    cell = Cell::with_bg(MOTE[(m >> 8) as usize % MOTE.len()], mote_fg, bg);
                }
            }
            row[x] = cell;
        }
    }
}

/// Two octaves of warped sine: the argument of one wave is displaced by
/// another, which bends the caustic net into lobes instead of a plaid.
#[inline]
fn warp(x: f32, y: f32, t: f32) -> f32 {
    let a = wsin(x * 0.132 + wsin(y * 0.213 + t * 0.31) * 2.4 + t * 0.17);
    let b = wsin(x * 0.071 - y * 0.114 + wsin(x * 0.037 - t * 0.23) * 1.8);
    a * 0.62 + b * 0.48
}

/// Tilted sun shafts first as a background lift, then the caustic net that is
/// strongest in the upper third and thins out toward the floor.
fn paint_caustics(
    grid: &mut Grid,
    w: usize,
    h: usize,
    seed: u64,
    floor: &[i32],
    palette: &[Color; 5],
    t: f32,
    k: &ReefKnobs,
) {
    if k.caustics <= 0.0 {
        return;
    }
    let tt = t * k.speed;
    let deep = *floor.iter().max().unwrap_or(&(h as i32));
    let shafts = 2 + (mix64(seed, L_SHAFT, 0, 9) % 3) as usize;
    for s in 0..shafts {
        let base = h01(seed, L_SHAFT, s as u64, 1) * w as f32;
        let slope = (h01(seed, L_SHAFT, s as u64, 2) - 0.5) * 1.5;
        let slope = slope + slope.signum() * 0.35;
        let wide = 2.0 + h01(seed, L_SHAFT, s as u64, 3) * 4.0;
        let sway = wsin(tt * 0.21 + s as f32 * 1.7) * 3.2;
        let gain = (0.42 + h01(seed, L_SHAFT, s as u64, 4) * 0.55) * k.caustics;
        for y in 0..deep.min(h as i32) {
            let fade = 1.0 - y as f32 / deep as f32;
            let lift = gain * fade * fade * 34.0;
            if lift < 1.0 {
                continue;
            }
            let axis = base + sway + y as f32 * slope;
            let lo = (axis - wide * 1.6).floor().max(0.0) as usize;
            let hi = ((axis + wide * 1.6).ceil() as i32).clamp(0, w as i32 - 1) as usize;
            for x in lo..=hi {
                if x >= grid[y as usize].len() || y >= floor[x] {
                    break;
                }
                let d = (x as f32 - axis) / wide;
                let a = (1.0 - d * d).max(0.0);
                let amt = (lift * a) as u8;
                if amt == 0 {
                    continue;
                }
                let cell = &mut grid[y as usize][x];
                cell.bg = lighten(cell.bg, amt);
                if cell.ch == ' ' {
                    cell.fg = cell.bg;
                }
            }
        }
    }
    let upper = (deep as f32 / 3.0).max(2.0);
    let glow = lerp_color(hsl_to_rgb(172.0, 0.46, 0.76), palette[4], 0.24);
    let reach = (deep as f32 * 0.66) as usize;
    for y in 0..reach.min(h) {
        let band = (1.0 - y as f32 / upper).max(0.0);
        let strength = (0.12 + 0.88 * band) * k.caustics.min(1.2);
        if strength < 0.2 {
            continue;
        }
        let cut = 1.02 - 0.58 * strength;
        for x in 0..w.min(grid[y].len()) {
            if grid[y][x].ch != ' ' || y as i32 >= floor[x] {
                continue;
            }
            let n = warp(x as f32, y as f32, tt);
            let c = wsin(n * 3.4 + tt * 0.6).abs() * 0.78 + n.abs() * 0.34;
            if c < cut {
                continue;
            }
            let lvl = (((c - cut) / 0.16) as usize).min(CAUSTIC.len() - 1);
            let bg = grid[y][x].bg;
            let fg = lerp_color(bg, glow, 0.40 + 0.20 * lvl as f32 + 0.22 * band);
            grid[y][x] = Cell::with_bg(CAUSTIC[lvl], fg, bg);
        }
    }
    let surf = lerp_color(hsl_to_rgb(176.0, 0.42, 0.84), palette[4], 0.32);
    for y in 0..2usize.min(h) {
        for x in 0..w.min(grid[y].len()) {
            let fx = x as f32;
            let crest = wsin(fx * 0.26 + tt * 0.7) * 0.6 + wsin(fx * 0.11 - tt * 0.4) * 0.4;
            let lit = crest - y as f32 * 0.75;
            if lit < 0.12 {
                continue;
            }
            let lvl = ((lit / 0.32) as usize).min(CAUSTIC.len() - 1);
            let bg = grid[y][x].bg;
            let fg = lerp_color(bg, surf, 0.45 + 0.25 * lvl as f32);
            grid[y][x] = Cell::with_bg(CAUSTIC[lvl], fg, bg);
        }
    }
}

/// Sand, ripples, rocks and eelgrass. The sand band is the top few rows of the
/// floor; below it the bed goes dark and quiet.
fn paint_seabed(
    grid: &mut Grid,
    w: usize,
    h: usize,
    seed: u64,
    bed: &Bed,
    rows: &[Color],
    palette: &[Color; 5],
    t: f32,
    k: &ReefKnobs,
) {
    let sand_hi = lerp_color(hsl_to_rgb(41.0, 0.34, 0.44), palette[0], 0.24);
    let sand_lo = lerp_color(hsl_to_rgb(30.0, 0.26, 0.15), palette[0], 0.40);
    let floor = &bed.floor;
    for x in 0..w {
        let fl = floor[x];
        let deep_row = rows[(fl.max(0) as usize).min(rows.len() - 1)];
        for y in fl.max(0)..h as i32 {
            if y as usize >= grid.len() || x >= grid[y as usize].len() {
                break;
            }
            let d = (y - fl) as f32;
            let sink = (d / 2.4).min(1.0);
            let bg = lerp_color(
                lerp_color(sand_hi, sand_lo, 0.18 + 0.82 * sink),
                deep_row,
                0.20 + 0.28 * sink,
            );
            let ripple = wsin(x as f32 * 0.33 + d * 1.1 + fl as f32 * 0.65)
                + 0.55 * wsin(x as f32 * 0.12 - d * 0.5);
            let g = mix64(seed, L_FLOOR, (y as usize * w + x) as u64, 5);
            let ch = if d < 3.0 {
                if ripple > 1.08 {
                    SAND[2]
                } else if ripple > 0.42 {
                    SAND[1]
                } else if g % 4 == 0 {
                    SAND[0]
                } else {
                    ' '
                }
            } else if g % 11 == 0 {
                SAND[0]
            } else {
                ' '
            };
            let fg = lighten(bg, if ripple > 1.08 { 34 } else { 20 });
            grid[y as usize][x] = Cell::with_bg(ch, fg, bg);
        }
    }
    for &(x, y, kind) in &bed.rocks {
        let stone = lerp_color(hsl_to_rgb(28.0, 0.12, 0.30), palette[0], 0.34);
        for dx in -1i32..=1 {
            let rx = x + dx;
            if rx < 0 || rx as usize >= w {
                continue;
            }
            let ry = if dx == 0 { y - 1 } else { y };
            if ry < 0 || ry as usize >= h || ry as usize >= grid.len() {
                continue;
            }
            if dx != 0 && kind == 0 {
                continue;
            }
            let bg = grid[ry as usize][rx as usize].bg;
            let ch = if dx == 0 { ROCK[kind] } else { ROCK[0] };
            grid[ry as usize][rx as usize] = Cell::with_bg(ch, lerp_color(stone, bg, 0.12), bg);
        }
    }
    let tt = t * k.speed;
    for tuft in &bed.tufts {
        let mut prev = 0i32;
        for j in 1..=tuft.tall {
            let y = tuft.base - j;
            if y < 0 || y as usize >= h || y as usize >= grid.len() {
                break;
            }
            let f = j as f32 / tuft.tall as f32;
            let sway = if t > 0.0 {
                wsin(tt * 0.9 + tuft.phase + f * 1.4) * 1.15
            } else {
                0.0
            };
            let off = ((tuft.lean + sway) * f * 1.7).round() as i32;
            let x = tuft.x + off;
            if x < 0 || x as usize >= w || x as usize >= grid[y as usize].len() {
                break;
            }
            let step = off - prev;
            prev = off;
            let ch = match step {
                0 => BLADE[0],
                1 => BLADE[1],
                -1 => BLADE[2],
                _ => BLADE[3],
            };
            let bg = grid[y as usize][x as usize].bg;
            let blade = hsl_to_rgb(
                126.0 + 26.0 * f as f64,
                0.44,
                (0.24 + 0.24 * f as f64).min(0.62),
            );
            grid[y as usize][x as usize] = Cell::with_bg(ch, lerp_color(blade, bg, 0.14), bg);
        }
    }
}

fn paint_coral(grid: &mut Grid, w: usize, h: usize, corals: &[Coral], deep: i32, t: f32, speed: f32) {
    let tt = t * speed;
    let span = deep.max(1) as f32;
    for c in corals {
        if c.x < 0 || c.y < 0 || c.x as usize >= w || c.y as usize >= h {
            continue;
        }
        let (x, y) = (c.x as usize, c.y as usize);
        if y >= grid.len() || x >= grid[y].len() {
            continue;
        }
        let pulse = 0.06 * c.tip * wsin(tt * 1.55 + c.phase + c.tip * 2.6);
        let lum = (0.24 + 0.44 * c.tip + pulse).clamp(0.07, 0.94) as f64;
        let sat = (0.62 - 0.16 * c.tip) as f64;
        let hue = (c.hue + 18.0 * c.tip as f64).rem_euclid(360.0);
        let depth = (y as f32 / span).clamp(0.0, 1.0);
        let bg = grid[y][x].bg;
        let fg = lerp_color(hsl_to_rgb(hue, sat, lum), bg, 0.08 + 0.26 * depth);
        grid[y][x] = Cell::with_bg(c.ch, fg, darken(bg, 6));
    }
}

/// Bubble columns off the floor vents. Nothing rises in the static frame.
fn paint_bubbles(
    grid: &mut Grid,
    w: usize,
    h: usize,
    seed: u64,
    vents: &[(i32, i32)],
    t: f32,
    k: &ReefKnobs,
) {
    if vents.is_empty() || t <= 0.0 {
        return;
    }
    let hscale = (h as f32 / 24.0).sqrt().clamp(1.0, 7.0);
    let per = ((k.bubble_density * 16.0 * hscale) as usize).clamp(1, 400);
    let tt = t * k.speed;
    for (v, vent) in vents.iter().enumerate() {
        for i in 0..per {
            let idx = (v * 512 + i) as u64;
            let fl = vent.1.max(1) as f32;
            let rise = 0.075 + h01(seed, L_BUBBLE, idx, 2) * 0.16;
            let ph = h01(seed, L_BUBBLE, idx, 3);
            let amp = 0.7 + h01(seed, L_BUBBLE, idx, 4) * 1.9;
            let freq = 0.8 + h01(seed, L_BUBBLE, idx, 5) * 1.6;
            let p = (ph + tt * rise).rem_euclid(1.0);
            let y = (fl - p * (fl - 1.0)).round() as i32;
            if y < 0 || y as usize >= h || y as usize >= grid.len() {
                continue;
            }
            let x = (vent.0 as f32 + wsin(p * TAU * freq + ph * TAU) * amp).round() as i32;
            if x < 0 || x as usize >= w || x as usize >= grid[y as usize].len() {
                continue;
            }
            let ch = if p > 0.34 { 'o' } else { '.' };
            let bg = grid[y as usize][x as usize].bg;
            let shine = hsl_to_rgb(188.0, 0.34, (0.52 + 0.30 * p) as f64);
            grid[y as usize][x as usize] =
                Cell::with_bg(ch, lerp_color(shine, bg, 0.18 - 0.12 * p), bg);
        }
    }
}

fn mirror(ch: char) -> char {
    match ch {
        '(' => ')',
        ')' => '(',
        '<' => '>',
        '>' => '<',
        '/' => '\\',
        '\\' => '/',
        '`' => '\'',
        '\'' => '`',
        c => c,
    }
}

fn stamp(grid: &mut Grid, w: usize, h: usize, x0: i32, y: i32, body: &str, right: bool, fg: Color) {
    if y < 0 || y as usize >= h || y as usize >= grid.len() {
        return;
    }
    let chars: Vec<char> = if right {
        body.chars().collect()
    } else {
        body.chars().rev().map(mirror).collect()
    };
    for (j, ch) in chars.into_iter().enumerate() {
        if ch == ' ' {
            continue;
        }
        let x = x0 + j as i32;
        if x < 0 || x as usize >= w || x as usize >= grid[y as usize].len() {
            continue;
        }
        let bg = grid[y as usize][x as usize].bg;
        grid[y as usize][x as usize] = Cell::with_bg(ch, lerp_color(fg, bg, 0.12), bg);
    }
}

/// Small schools, one heading each, plus a large silhouette that crosses the
/// pane now and then once the clock runs.
fn paint_fish(grid: &mut Grid, w: usize, h: usize, seed: u64, floor: &[i32], t: f32, k: &ReefKnobs) {
    if k.fish_count <= 0.0 {
        return;
    }
    let hscale = (h as f32 / 24.0).sqrt().clamp(1.0, 7.0);
    let total = ((k.fish_count * (w as f32 / 80.0) * hscale).round() as usize).clamp(1, 600);
    let schools = (2 + (mix64(seed, L_SCHOOL, 0, 3) % 3) as usize).min(total);
    let tt = t * k.speed;
    let shallow = *floor.iter().min().unwrap_or(&(h as i32));
    let span = (w + 14) as f32;
    for s in 0..schools {
        let members = (total / schools + usize::from(s < total % schools)).max(1);
        let right = h01(seed, L_SCHOOL, s as u64, 2) < 0.5;
        let lane = 0.12 + h01(seed, L_SCHOOL, s as u64, 1) * 0.76;
        let base_y = 1.0 + lane * (shallow as f32 - 3.0).max(1.0);
        let vel = 1.6 + h01(seed, L_SCHOOL, s as u64, 3) * 3.8;
        let ph = h01(seed, L_SCHOOL, s as u64, 4);
        let shape = (mix64(seed, L_SCHOOL, s as u64, 5) % FISH_R.len() as u64) as usize;
        let hue = FISH_HUES[(mix64(seed, L_SCHOOL, s as u64, 6) % FISH_HUES.len() as u64) as usize];
        let dir = if right { 1.0 } else { -1.0 };
        let head = (ph * span + dir * vel * tt).rem_euclid(span) - 7.0;
        let body = if right { FISH_R[shape] } else { FISH_L[shape] };
        let reach = body.chars().count() as f32 + 1.0;
        for m in 0..members {
            let idx = (s * 64 + m) as u64;
            let rank = m as f32;
            let back = rank * reach * (0.62 + h01(seed, L_FISH, idx, 1) * 0.5);
            let sx = head - dir * back;
            let stagger = (h01(seed, L_FISH, idx, 2) - 0.5) * 3.4;
            let bob = wsin(tt * 1.9 + h01(seed, L_FISH, idx, 3) * TAU) * 1.1;
            let y = (base_y + stagger + bob).round() as i32;
            if y < 1 {
                continue;
            }
            let col = (sx.round() as i32).clamp(0, w as i32 - 1) as usize;
            if y >= floor[col] {
                continue;
            }
            let lum = 0.54 + h01(seed, L_FISH, idx, 4) as f64 * 0.20;
            let fg = lighten(hsl_to_rgb(hue, 0.66, lum), 6);
            stamp(grid, w, h, sx.round() as i32, y, body, right, fg);
        }
    }
    if t <= 0.0 {
        return;
    }
    let cycle = 34.0;
    let pass = (tt / cycle).floor();
    let phase = tt / cycle - pass;
    if phase > 0.52 {
        return;
    }
    let big = (pass as i64 as u64).wrapping_mul(0x9E37_79B9);
    let right = mix64(seed, L_FISH, big, 21) % 2 == 0;
    let dir = if right { 1.0 } else { -1.0 };
    let travel = phase / 0.52;
    let x0 = if right {
        -12.0 + travel * (w as f32 + 24.0)
    } else {
        w as f32 + 12.0 - travel * (w as f32 + 24.0)
    };
    let lane = 0.16 + h01(seed, L_FISH, big, 22) * 0.5;
    let y0 = 2.0 + lane * (shallow as f32 - 5.0).max(1.0)
        + wsin(tt * 0.5 + h01(seed, L_FISH, big, 23) * TAU) * 1.2;
    let hue = 208.0 + h01(seed, L_FISH, big, 24) as f64 * 28.0;
    let fg = hsl_to_rgb(hue, 0.30, 0.66);
    let _ = dir;
    for (r, row) in RAY.iter().enumerate() {
        let y = y0.round() as i32 + r as i32;
        let col = (x0.round() as i32).clamp(0, w as i32 - 1) as usize;
        if y < 1 || y >= floor[col] {
            continue;
        }
        stamp(grid, w, h, x0.round() as i32, y, row, right, fg);
    }
}

fn draw_reef(
    grid: &mut Grid,
    w: usize,
    h: usize,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    k: &ReefKnobs,
) {
    if w < 6 || h < 5 || grid.is_empty() {
        return;
    }
    let key = (seed, w, h, k.clone());
    let fresh = BED.with(|b| b.borrow().as_ref().map(|bed| bed.key != key).unwrap_or(true));
    if fresh {
        let bed = measure_layer(NAME, "dla_grow", || build_bed(w, h, seed, k));
        BED.with(|b| *b.borrow_mut() = Some(bed));
    } else {
        measure_layer(NAME, "dla_grow", || ());
    }
    BED.with(|cell| {
        let borrow = cell.borrow();
        let bed = borrow.as_ref().expect("reef bed built above");
        let deep = *bed.floor.iter().max().unwrap_or(&(h as i32));
        let rows = water_rows(h, deep, palette);
        measure_layer(NAME, "water", || {
            paint_water(grid, w, h, seed, &bed.floor, &rows, t, k.speed)
        });
        measure_layer(NAME, "caustics", || {
            paint_caustics(grid, w, h, seed, &bed.floor, palette, t, k)
        });
        measure_layer(NAME, "seabed", || {
            paint_seabed(grid, w, h, seed, bed, &rows, palette, t, k)
        });
        measure_layer(NAME, "coral_paint", || {
            paint_coral(grid, w, h, &bed.corals, deep, t, k.speed)
        });
        measure_layer(NAME, "bubbles", || {
            paint_bubbles(grid, w, h, seed, &bed.vents, t, k)
        });
        measure_layer(NAME, "fish", || {
            paint_fish(grid, w, h, seed, &bed.floor, t, k)
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::grid_to_plain;

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
    fn reef_seed42() {
        insta::assert_snapshot!("reef_80x24", text(&frame(80, 24, 42, 0.0, &knobs())));
    }

    #[test]
    fn reef_seed42_t6() {
        insta::assert_snapshot!("reef_80x24_t6", text(&frame(80, 24, 42, 6.0, &knobs())));
    }

    #[test]
    fn reef_tiny_grid() {
        insta::assert_snapshot!("reef_tiny_7", text(&frame(46, 14, 7, 0.0, &knobs())));
    }

    #[test]
    fn dense_colonies_fit_a_narrow_spread() {
        let mut values = knobs();
        values[0] = PARAMS[0].max;
        values[6] = PARAMS[6].min;
        let a = frame(80, 24, 42, 0.0, &values);
        assert_eq!(a, frame(80, 24, 42, 0.0, &values));
        assert_eq!(a.len(), 24);
        assert!(a.iter().all(|row| row.len() == 80));
        assert!(a.iter().flatten().any(|cell| cell.ch != ' '));
    }

    #[test]
    fn deterministic_and_seed_sensitive() {
        let k = knobs();
        assert_eq!(text(&frame(80, 24, 42, 0.0, &k)), text(&frame(80, 24, 42, 0.0, &k)));
        assert_ne!(text(&frame(80, 24, 42, 0.0, &k)), text(&frame(80, 24, 7, 0.0, &k)));
    }

    #[test]
    fn t_zero_is_the_static_frame_and_time_moves() {
        let k = knobs();
        assert_eq!(text(&frame(80, 24, 42, 0.0, &k)), text(&frame(80, 24, 42, 0.0, &k)));
        assert_ne!(text(&frame(80, 24, 42, 0.0, &k)), text(&frame(80, 24, 42, 3.5, &k)));
        assert_ne!(text(&frame(80, 24, 42, 3.5, &k)), text(&frame(80, 24, 42, 7.0, &k)));
    }

    #[test]
    fn coral_holds_still_while_the_water_moves() {
        let mut k = knobs();
        k[4] = 0.0;
        k[5] = 0.0;
        let a = frame(80, 24, 42, 0.0, &k);
        let b = frame(80, 24, 42, 0.4, &k);
        let trunk = ['@', '#', '%', '&'];
        let mut same = 0usize;
        let mut total = 0usize;
        for y in 0..18 {
            for x in 0..80 {
                if trunk.contains(&a[y][x].ch) {
                    total += 1;
                    if a[y][x].ch == b[y][x].ch {
                        same += 1;
                    }
                }
            }
        }
        assert!(total > 20, "expected a grown reef, saw {total} branch cells");
        assert_eq!(same, total, "aggregate must not regrow between frames");
    }

    #[test]
    fn four_silhouettes_share_the_bed() {
        let g = frame(120, 30, 42, 0.0, &knobs());
        let seen: Vec<char> = g.iter().flatten().map(|c| c.ch).collect();
        for ramp in [&RAMP_FAN, &RAMP_STAG, &RAMP_BRAIN, &RAMP_TUBE] {
            let hits = ramp[2..].iter().filter(|c| seen.contains(c)).count();
            assert!(hits > 0, "no glyph from ramp {:?} reached the pane", ramp);
        }
    }

    #[test]
    fn the_floor_is_sand_not_underscores() {
        let g = frame(80, 24, 42, 0.0, &knobs());
        assert!(
            g.iter().flatten().all(|c| c.ch != '_'),
            "seabed must not use underscores"
        );
    }

    #[test]
    fn every_glyph_is_one_column_wide() {
        use unicode_width::UnicodeWidthChar;
        let g = frame(80, 24, 1701, 2.0, &knobs());
        for row in &g {
            for cell in row {
                assert_eq!(cell.ch.width().unwrap_or(0), 1, "wide glyph {:?}", cell.ch);
            }
        }
    }

    #[test]
    fn knobs_change_the_reef() {
        let base = text(&frame(80, 24, 42, 0.0, &knobs()));
        let run = |i: usize, v: f32| {
            let mut k = knobs();
            k[i] = v;
            text(&frame(80, 24, 42, 0.0, &k))
        };
        assert_ne!(base, run(0, 7.0), "colony count");
        assert_ne!(base, run(3, 0.6), "water depth");
        assert_ne!(base, run(5, 0.0), "fish count");
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
        eprintln!("reef frame_cost 200x60: avg {:.3} ms, worst {:.3} ms", avg, worst);
        if !cfg!(debug_assertions) {
            assert!(avg < 6.0, "avg frame {:.3} ms", avg);
        }
    }
}
