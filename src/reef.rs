//! reef -- coral colonies grown by half-space DLA off the seabed, lit by a
//! caustic water column with sun shafts, rising bubbles and drifting fish.
use crate::_0_profile::measure_layer;
use crate::color::{hsl_to_rgb, lerp_color, lighten};
use crate::opts::param_f32;
use crate::types::*;
use crossterm::style::Color;
use rand::RngExt;
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::cell::RefCell;
use std::f32::consts::TAU;
use std::sync::LazyLock;

/// Branch thickness ramp, fine tips first, trunk last.
const RAMP: [char; 8] = ['.', ':', 'o', 'O', '@', '#', '%', '&'];
const SAND: [char; 5] = ['.', ',', '_', ':', '`'];
const CAUSTIC: [char; 3] = ['-', '~', '='];
const MOTE: [char; 2] = ['.', '`'];
const FISH_RIGHT: [&str; 2] = ["><>", "><(((>"];
const FISH_LEFT: [&str; 2] = ["<><", "<)))><"];
const FISH_HUES: [f64; 5] = [45.0, 200.0, 15.0, 320.0, 60.0];
const COLONY_HUES: [f64; 4] = [350.0, 25.0, 275.0, 175.0];

const L_COLONY: u64 = 0x21;
const L_BUBBLE: u64 = 0x22;
const L_FISH: u64 = 0x23;
const L_SHAFT: u64 = 0x24;
const L_FLOOR: u64 = 0x25;
const L_MOTE: u64 = 0x26;

const WAVE_N: usize = 2048;

/// One period of sine. The caustic pass touches every water cell twice, so the
/// table replaces two libm calls per cell with a multiply, a mask and a load.
static WAVE: LazyLock<[f32; WAVE_N]> =
    LazyLock::new(|| std::array::from_fn(|i| (i as f32 * TAU / WAVE_N as f32).sin()));

#[inline(always)]
#[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
fn wsin(x: f32) -> f32 {
    let i = (x * (WAVE_N as f32 / TAU)) as i32 as usize & (WAVE_N - 1);
    WAVE[i]
}

#[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
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
#[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
fn h01(seed: u64, layer: u64, idx: u64, salt: u64) -> f32 {
    (mix64(seed, layer, idx, salt) & 0xFF_FFFF) as f32 / 16_777_216.0
}

#[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
fn ramp_idx(weight: u32) -> usize {
    match weight {
        1 => 0,
        2 => 1,
        3..=4 => 2,
        5..=9 => 3,
        10..=20 => 4,
        21..=44 => 5,
        45..=96 => 6,
        _ => 7,
    }
}

// ── Knobs ───────────────────────────────────────────────────────────

#[derive(Clone, PartialEq)]
pub struct ReefKnobs {
    pub colony_count: f32,
    pub walker_budget: f32,
    pub stickiness: f32,
    pub water_depth_frac: f32,
    pub bubble_density: f32,
    pub fish_count: f32,
    pub spread: f32,
    pub caustics: f32,
    pub grow: f32,
    pub hue: f32,
    pub speed: f32,
}

impl ReefKnobs {
    #[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
    pub fn from_env() -> Self {
        ReefKnobs {
            colony_count: param_f32("COLONIES", 4.0).clamp(1.0, 12.0),
            walker_budget: param_f32("WALKERS", 1400.0).clamp(40.0, 12000.0),
            stickiness: param_f32("STICK", 0.5).clamp(0.02, 1.0),
            water_depth_frac: param_f32("DEPTH", 0.86).clamp(0.35, 0.98),
            bubble_density: param_f32("BUBBLES", 0.7).clamp(0.0, 3.0),
            fish_count: param_f32("FISH", 7.0).clamp(0.0, 40.0),
            spread: param_f32("SPREAD", 0.86).clamp(0.2, 1.0),
            caustics: param_f32("CAUST", 0.7).clamp(0.0, 1.5),
            grow: param_f32("GROW", 1.0).clamp(0.1, 1.5),
            hue: param_f32("HUE", 0.0).clamp(-180.0, 180.0),
            speed: param_f32("SPEED", 1.0).clamp(0.0, 3.0),
        }
    }
}

// ── The grown bed, cached across animation frames ───────────────────

#[derive(Clone, Copy)]
struct Coral {
    x: i32,
    y: i32,
    hue: f64,
    glyph: usize,
    tip: f32,
    phase: f32,
}

struct Bed {
    key: (u64, usize, usize, ReefKnobs),
    floor: Vec<i32>,
    corals: Vec<Coral>,
    vents: Vec<(i32, i32)>,
}

thread_local! {
    static BED: RefCell<Option<Bed>> = const { RefCell::new(None) };
}

#[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
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
    base: i32,
}

#[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
fn put(
    owner: &mut [i32],
    sites: &mut Vec<(i32, i32)>,
    parents: &mut Vec<usize>,
    w: usize,
    x: i32,
    y: i32,
    parent: usize,
) -> bool {
    let slot = y as usize * w + x as usize;
    if owner[slot] >= 0 {
        return false;
    }
    owner[slot] = sites.len() as i32;
    sites.push((x, y));
    parents.push(parent);
    true
}

/// Grow one colony by DLA inside its own ellipse: a seeded base arc on the
/// seabed, then walkers launched on the rim that stick to kin only.
#[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
fn grow_colony(
    w: usize,
    h: usize,
    owner: &mut [i32],
    sites: &mut Vec<(i32, i32)>,
    parents: &mut Vec<usize>,
    floor: &[i32],
    c: &Colony,
    stick: f32,
    rng: &mut StdRng,
) -> (usize, usize) {
    let start = sites.len();
    if c.cy < 1 || c.cy as usize >= h || c.cx < 1 || c.cx as usize >= w - 1 {
        return (start, start);
    }
    if !put(owner, sites, parents, w, c.cx, c.cy, start) {
        return (start, start);
    }
    let mut left = start;
    let mut right = start;
    for step in 1..=c.base {
        for side in 0..2 {
            let dx = if side == 0 { -step } else { step };
            let x = c.cx + dx;
            if x < 1 || x as usize >= w - 1 {
                continue;
            }
            let y = (floor[x as usize] - 1).min(c.cy + 1).max(1);
            let parent = if side == 0 { left } else { right };
            if put(owner, sites, parents, w, x, y, parent) {
                if side == 0 {
                    left = sites.len() - 1;
                } else {
                    right = sites.len() - 1;
                }
            }
        }
    }
    let ax = c.ax.max(2.0);
    let ay = c.ay.max(2.0);
    let mut rmax = (c.base as f32 / ax).max(0.16);
    let mut grown = sites.len() - start;
    for _ in 0..c.walkers {
        if grown >= c.cells {
            break;
        }
        let rs = (rmax + 0.14).min(1.0);
        let ang = rng.random::<f32>() * (TAU * 0.5);
        let mut px = c.cx as f32 + ang.cos() * rs * ax;
        let mut py = c.cy as f32 - ang.sin() * rs * ay;
        let steps = ((ax + ay) * 9.0) as i32;
        let steps = steps.clamp(80, 1600);
        let mut hit: Option<usize> = None;
        for _ in 0..steps {
            let ix = px.round() as i32;
            let iy = py.round() as i32;
            let mut found = None;
            let mut kin = 0u32;
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    let (nx, ny) = (ix + dx, iy + dy);
                    if nx < 0 || ny < 0 || nx as usize >= w || ny as usize >= h {
                        continue;
                    }
                    let o = owner[ny as usize * w + nx as usize];
                    if o >= start as i32 {
                        found = Some(o as usize);
                        kin += 1;
                    }
                }
            }
            let grip = match kin {
                0 => 0.0,
                1 => stick * stick * 0.5,
                2 => stick,
                _ => 1.0,
            };
            if found.is_some() && rng.random::<f32>() < grip {
                hit = found;
                break;
            }
            let r = rng.random::<f32>();
            let (dx, dy) = if r < 0.33 {
                (-1.0f32, 0.0f32)
            } else if r < 0.66 {
                (1.0, 0.0)
            } else if r < 0.83 {
                (0.0, -1.0)
            } else if r < 0.97 {
                (0.0, 1.0)
            } else {
                let sx = if rng.random::<f32>() < 0.5 { -1.0 } else { 1.0 };
                (sx, if rng.random::<f32>() < 0.5 { -1.0 } else { 1.0 })
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
            if ndx * ndx + ndy * ndy > 1.30 {
                break;
            }
        }
        let Some(parent) = hit else { continue };
        let (ix, iy) = (px.round() as i32, py.round() as i32);
        if ix < 1 || iy < 1 || ix as usize >= w - 1 || iy as usize >= h {
            continue;
        }
        let ndx = (ix - c.cx) as f32 / ax;
        let ndy = (iy - c.cy) as f32 / ay;
        let nr = (ndx * ndx + ndy * ndy).sqrt();
        if nr > 1.0 {
            continue;
        }
        if put(owner, sites, parents, w, ix, iy, parent) {
            grown += 1;
            rmax = rmax.max(nr);
        }
    }
    (start, sites.len())
}

#[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
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
    let mut sites: Vec<(i32, i32)> = Vec::new();
    let mut parents: Vec<usize> = Vec::new();
    let mut vents: Vec<(i32, i32)> = Vec::new();
    for c in 0..n {
        let mut rng = StdRng::seed_from_u64(mix64(seed, L_COLONY, c as u64, 0));
        let jitter = (h01(seed, L_COLONY, c as u64, 3) - 0.5) * slot * 0.5;
        let cx = (left + (c as f32 + 0.5) * slot + jitter).round() as i32;
        let cx = cx.clamp(2, w as i32 - 3);
        let cy = floor[cx as usize] - 1;
        let bulk = (0.66 + h01(seed, L_COLONY, c as u64, 5) * 0.70) * k.grow;
        let tall = 0.46 + h01(seed, L_COLONY, c as u64, 11) * 0.88;
        let room = ((cx - 1).min(w as i32 - 2 - cx)).max(2) as f32;
        let ax = (slot * 0.62 * bulk).clamp(2.5, room.min(slot * 1.05));
        let ay = (ax * tall).clamp(2.0, column * 0.86);
        let cells = ((ax * ay * 1.10).round() as usize).clamp(6, 4200);
        let effort = k.walker_budget / 1400.0;
        let walkers = (cells as f32 * 3.4 * effort) as usize;
        let walkers = walkers.clamp(24, 160_000 / n.max(1)).max(24);
        let colony = Colony {
            cx,
            cy,
            ax,
            ay,
            walkers,
            cells,
            base: ((ax * 0.42) as i32).clamp(1, 24),
        };
        let (a, b) = grow_colony(
            w,
            h,
            &mut owner,
            &mut sites,
            &mut parents,
            &floor,
            &colony,
            k.stickiness,
            &mut rng,
        );
        if b <= a {
            continue;
        }
        vents.push((cx, (cy as f32 - ay * 0.55) as i32));
        let len = b - a;
        let mut weight = vec![1u32; len];
        for i in (1..len).rev() {
            let p = parents[a + i] - a;
            weight[p] += weight[i];
        }
        let mut score = vec![0u32; len];
        let mut touch = vec![0u32; len];
        for i in 0..len {
            let (x, y) = sites[a + i];
            let mut kin = 0u32;
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    let (nx, ny) = (x + dx, y + dy);
                    if nx < 0 || ny < 0 || nx as usize >= w || ny as usize >= h {
                        continue;
                    }
                    if owner[ny as usize * w + nx as usize] >= a as i32 {
                        kin += 1;
                    }
                }
            }
            let bulge = kin.saturating_sub(3);
            score[i] = weight[i] + bulge * bulge * 3;
            touch[i] = kin;
        }
        let root = (weight[0] as f32).ln().max(1.0);
        let hue = COLONY_HUES[c % COLONY_HUES.len()]
            + (c / COLONY_HUES.len()) as f64 * 37.0
            + k.hue as f64
            + (h01(seed, L_COLONY, c as u64, 7) as f64 - 0.5) * 16.0;
        let hue = hue.rem_euclid(360.0);
        let phase = h01(seed, L_COLONY, c as u64, 9) * TAU;
        for i in 0..len {
            let (x, y) = sites[a + i];
            if weight[i] == 1 && touch[i] <= 1 && h01(seed, L_COLONY, (a + i) as u64, 13) < 0.7 {
                continue;
            }
            let rise = ((cy - y) as f32 / ay.max(1.0)).clamp(0.0, 1.0);
            let tip = (1.0 - (weight[i] as f32).ln() / root) * 0.68 + rise * 0.32;
            corals.push(Coral {
                x,
                y,
                hue,
                glyph: ramp_idx(score[i]),
                tip: tip.clamp(0.0, 1.0),
                phase,
            });
        }
    }
    Bed {
        key: (seed, w, h, k.clone()),
        floor,
        corals,
        vents,
    }
}


// ── Painters ────────────────────────────────────────────────────────

/// Water column colors per row: blue-green shallows down to near-black navy,
/// tinted toward the theme background so a palette still reads.
#[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
fn water_rows(h: usize, deep: i32, palette: &[Color; 5]) -> Vec<Color> {
    let span = (deep.max(1)) as f32;
    (0..h)
        .map(|y| {
            let f = (y as f32 / span).clamp(0.0, 1.25);
            let hue = 186.0 + 40.0 * f as f64;
            let sat = 0.52 + 0.24 * f as f64;
            let lum = (0.205 - 0.165 * f as f64).max(0.028);
            lerp_color(hsl_to_rgb(hue.min(238.0), sat, lum), palette[0], 0.30)
        })
        .collect()
}

#[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
fn paint_water(
    grid: &mut Grid,
    w: usize,
    h: usize,
    seed: u64,
    floor: &[i32],
    rows: &[Color],
    palette: &[Color; 5],
    t: f32,
    speed: f32,
) {
    let drift = (t * speed * 0.9) as i32;
    let sand_deep = lerp_color(hsl_to_rgb(34.0, 0.30, 0.20), palette[0], 0.42);
    for y in 0..h.min(grid.len()) {
        let bg = rows[y];
        let mote_fg = lighten(bg, 34);
        let row = &mut grid[y];
        for x in 0..w.min(row.len()) {
            let fl = floor[x.min(floor.len() - 1)];
            if (y as i32) >= fl {
                let d = ((y as i32 - fl) as f32 * 0.22).min(0.7);
                let grain = mix64(seed, L_FLOOR, (y * w + x) as u64, 5);
                let ripple = wsin(x as f32 * 0.24 + y as f32 * 1.35 + fl as f32 * 0.6);
                let sbg = lerp_color(sand_deep, bg, 0.26 + d * 0.42);
                let ch = if ripple > 0.72 {
                    SAND[2]
                } else if grain % 5 == 0 {
                    SAND[(grain >> 8) as usize % SAND.len()]
                } else {
                    ' '
                };
                let fg = lighten(sbg, if ripple > 0.72 { 30 } else { 16 });
                row[x] = Cell::with_bg(ch, fg, sbg);
                continue;
            }
            let mut cell = Cell::with_bg(' ', bg, bg);
            let yy = (y as i32 + drift) as usize;
            if (x ^ yy.wrapping_mul(3)) & 7 == 0 {
                let m = mix64(seed, L_MOTE, (yy * w + x) as u64, 11);
                if m % 23 == 0 {
                    cell = Cell::with_bg(MOTE[(m >> 8) as usize % MOTE.len()], mote_fg, bg);
                }
            }
            row[x] = cell;
        }
    }
}

/// Sun shafts first as a background lift, then the crossing caustic net that
/// sits in the upper reach of the column and thins out with depth.
#[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
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
    let shafts = ((w as f32 / 26.0).round() as usize).clamp(2, 14);
    for s in 0..shafts {
        let base = h01(seed, L_SHAFT, s as u64, 1) * w as f32;
        let slope = (h01(seed, L_SHAFT, s as u64, 2) - 0.5) * 0.9;
        let wide = 2.0 + h01(seed, L_SHAFT, s as u64, 3) * 5.0;
        let sway = wsin(tt * 0.21 + s as f32 * 1.7) * 3.2;
        let gain = (0.35 + h01(seed, L_SHAFT, s as u64, 4) * 0.5) * k.caustics;
        for y in 0..deep.min(h as i32) {
            let fade = 1.0 - y as f32 / deep as f32;
            let lift = gain * fade * fade * 30.0;
            if lift < 1.0 {
                continue;
            }
            let axis = base + sway + y as f32 * slope;
            let lo = (axis - wide * 1.6).floor().max(0.0) as usize;
            let hi = ((axis + wide * 1.6).ceil() as i32).clamp(0, w as i32 - 1) as usize;
            for x in lo..=hi {
                if x >= grid[y as usize].len() {
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
    let band = (deep as f32 * 0.58).max(3.0);
    let glow = lerp_color(hsl_to_rgb(178.0, 0.42, 0.74), palette[4], 0.28);
    for y in 0..(band as usize).min(h) {
        let fade = 1.0 - y as f32 / band;
        let cut = 1.22 - 0.62 * fade * k.caustics.min(1.0);
        let row_a = wsin(y as f32 * 0.41 + tt * 0.85) * 2.4;
        let row_b = wsin(y as f32 * 0.27 - tt * 0.55) * 3.1;
        for x in 0..w.min(grid[y].len()) {
            if grid[y][x].ch != ' ' {
                continue;
            }
            let fx = x as f32;
            let a = wsin(fx * 0.33 + row_a);
            let b = wsin(fx * 0.19 - row_b + tt * 0.3);
            let c = (a + b).abs() * 0.68 + wsin(fx * 0.07 + tt * 0.11) * 0.18;
            if c < cut {
                continue;
            }
            let lvl = (((c - cut) / 0.24) as usize).min(CAUSTIC.len() - 1);
            let bg = grid[y][x].bg;
            let fg = lerp_color(bg, glow, 0.38 + 0.22 * lvl as f32 + 0.20 * fade);
            grid[y][x] = Cell::with_bg(CAUSTIC[lvl], fg, bg);
        }
    }
    let surf = lerp_color(hsl_to_rgb(180.0, 0.38, 0.82), palette[4], 0.35);
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

#[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
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
        let pulse = 0.07 * c.tip * wsin(tt * 1.55 + c.phase + c.tip * 2.6);
        let lum = (0.27 + 0.36 * c.tip + pulse).clamp(0.07, 0.93) as f64;
        let sat = (0.56 + 0.26 * c.tip) as f64;
        let depth = (y as f32 / span).clamp(0.0, 1.0);
        let bg = grid[y][x].bg;
        let fg = lerp_color(hsl_to_rgb(c.hue, sat, lum), bg, 0.10 + 0.30 * depth);
        grid[y][x] = Cell::with_bg(RAMP[c.glyph], fg, bg);
    }
}

#[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
fn paint_bubbles(
    grid: &mut Grid,
    w: usize,
    h: usize,
    seed: u64,
    vents: &[(i32, i32)],
    t: f32,
    k: &ReefKnobs,
) {
    if vents.is_empty() {
        return;
    }
    let hscale = (h as f32 / 24.0).sqrt().clamp(1.0, 7.0);
    let count = (k.bubble_density * w as f32 * 0.42 * hscale) as usize;
    let count = count.min(6000);
    let tt = t * k.speed;
    for i in 0..count {
        let v = vents[i % vents.len()];
        let off = (h01(seed, L_BUBBLE, i as u64, 1) - 0.5) * 5.0;
        let bx = (v.0 as f32 + off).clamp(0.0, w as f32 - 1.0);
        let fl = (v.1.max(1)) as f32;
        let rise = 0.055 + h01(seed, L_BUBBLE, i as u64, 2) * 0.13;
        let ph = h01(seed, L_BUBBLE, i as u64, 3);
        let amp = 0.5 + h01(seed, L_BUBBLE, i as u64, 4) * 2.0;
        let freq = 0.8 + h01(seed, L_BUBBLE, i as u64, 5) * 1.8;
        let big = h01(seed, L_BUBBLE, i as u64, 6);
        let p = (ph + tt * rise).rem_euclid(1.0);
        let y = (fl - 1.0 - p * (fl - 1.0)).round() as i32;
        if y < 0 || y as usize >= h || y as usize >= grid.len() {
            continue;
        }
        let x = (bx + wsin(p * TAU * freq + ph * TAU) * amp).round() as i32;
        if x < 0 || x as usize >= w || x as usize >= grid[y as usize].len() {
            continue;
        }
        let ch = if big > 0.42 { 'o' } else { '.' };
        let bg = grid[y as usize][x as usize].bg;
        let shine = hsl_to_rgb(188.0, 0.34, (0.52 + 0.30 * p) as f64);
        grid[y as usize][x as usize] =
            Cell::with_bg(ch, lerp_color(shine, bg, 0.18 - 0.12 * p), bg);
    }
}

#[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
fn paint_fish(grid: &mut Grid, w: usize, h: usize, seed: u64, floor: &[i32], t: f32, k: &ReefKnobs) {
    let hscale = (h as f32 / 24.0).sqrt().clamp(1.0, 7.0);
    let count = (k.fish_count * (w as f32 / 80.0) * hscale).round() as usize;
    let count = count.min(2000);
    let tt = t * k.speed;
    let deep = *floor.iter().min().unwrap_or(&(h as i32));
    let span = (w + 10) as f32;
    for i in 0..count {
        let lane = h01(seed, L_FISH, i as u64, 1);
        let base_y = 1.0 + lane * (deep as f32 - 3.0).max(1.0);
        let right = h01(seed, L_FISH, i as u64, 2) < 0.5;
        let vel = 1.4 + h01(seed, L_FISH, i as u64, 3) * 4.6;
        let ph = h01(seed, L_FISH, i as u64, 4);
        let sz = if h01(seed, L_FISH, i as u64, 5) < 0.32 { 1 } else { 0 };
        let hue = FISH_HUES[(mix64(seed, L_FISH, i as u64, 6) % FISH_HUES.len() as u64) as usize];
        let dir = if right { 1.0 } else { -1.0 };
        let sx = (ph * span + dir * vel * tt).rem_euclid(span) - 5.0;
        let bob = wsin(tt * 1.9 + ph * TAU) * 1.3;
        let y = (base_y + bob).round() as i32;
        if y < 0 || y as usize >= h || y as usize >= grid.len() {
            continue;
        }
        let fl = floor[(sx.round() as i32).clamp(0, w as i32 - 1) as usize];
        if y >= fl {
            continue;
        }
        let body = if right {
            FISH_RIGHT[sz]
        } else {
            FISH_LEFT[sz]
        };
        let lum = 0.52 + h01(seed, L_FISH, i as u64, 7) as f64 * 0.22;
        let fg = hsl_to_rgb(hue, 0.62, lum);
        for (j, ch) in body.chars().enumerate() {
            let x = sx.round() as i32 + j as i32;
            if x < 0 || x as usize >= w || x as usize >= grid[y as usize].len() {
                continue;
            }
            let bg = grid[y as usize][x as usize].bg;
            let head = j + 2 >= body.chars().count() && right || j <= 1 && !right;
            let tone = if head { lighten(fg, 24) } else { fg };
            grid[y as usize][x as usize] = Cell::with_bg(ch, lerp_color(tone, bg, 0.12), bg);
        }
    }
}

// ── Frame ───────────────────────────────────────────────────────────

#[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
pub fn draw_reef(
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
        let bed = measure_layer("reef", "dla_grow", || build_bed(w, h, seed, k));
        BED.with(|b| *b.borrow_mut() = Some(bed));
    } else {
        measure_layer("reef", "dla_grow", || ());
    }
    BED.with(|cell| {
        let borrow = cell.borrow();
        let bed = borrow.as_ref().expect("reef bed built above");
        let deep = *bed.floor.iter().max().unwrap_or(&(h as i32));
        let rows = water_rows(h, deep, palette);
        measure_layer("reef", "water", || {
            paint_water(
                grid, w, h, seed, &bed.floor, &rows, palette, t, k.speed,
            )
        });
        measure_layer("reef", "caustics", || {
            paint_caustics(grid, w, h, seed, &bed.floor, palette, t, k)
        });
        measure_layer("reef", "coral_paint", || {
            paint_coral(grid, w, h, &bed.corals, deep, t, k.speed)
        });
        measure_layer("reef", "bubbles", || {
            paint_bubbles(grid, w, h, seed, &bed.vents, t, k)
        });
        measure_layer("reef", "fish", || {
            paint_fish(grid, w, h, seed, &bed.floor, t, k)
        });
    });
}

// ── CLI dispatch arm ────────────────────────────────────────────────

#[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
pub(crate) fn cli_reef(
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
    // reef [colonies] [walkers] [stickiness] [depth] [bubbles] [fish]
    let mut k = ReefKnobs::from_env();
    let arg = |i: usize| args.get(i).and_then(|v| v.parse::<f32>().ok());
    if let Some(v) = arg(4) {
        k.colony_count = v.clamp(1.0, 12.0);
    }
    if let Some(v) = arg(5) {
        k.walker_budget = v.clamp(40.0, 12000.0);
    }
    if let Some(v) = arg(6) {
        k.stickiness = v.clamp(0.02, 1.0);
    }
    if let Some(v) = arg(7) {
        k.water_depth_frac = v.clamp(0.35, 0.98);
    }
    if let Some(v) = arg(8) {
        k.bubble_density = v.clamp(0.0, 3.0);
    }
    if let Some(v) = arg(9) {
        k.fish_count = v.clamp(0.0, 40.0);
    }
    let _ = (rng, term_w, term_h, mode, theme_name);
    draw_reef(&mut grid, width, height, seed, &palette, t_anim, &k);
    (grid, false)
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
    fn make(w: usize, h: usize, seed: u64) -> (Grid, [Color; 5]) {
        (
            vec![vec![Cell::blank(); w]; h],
            crate::color::make_palette(seed),
        )
    }

    #[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
    fn plain(grid: &Grid) -> String {
        grid.iter()
            .map(|row| row.iter().map(|c| c.ch).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    #[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
    fn snapshot_reef_standard() {
        let (mut g, p) = make(80, 24, 42);
        let k = ReefKnobs::from_env();
        draw_reef(&mut g, 80, 24, 42, &p, 0.0, &k);
        insta::assert_snapshot!("reef_42", plain(&g));
    }

    #[test]
    #[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
    fn snapshot_reef_tiny_grid() {
        let (mut g, p) = make(46, 14, 7);
        let k = ReefKnobs::from_env();
        draw_reef(&mut g, 46, 14, 7, &p, 0.0, &k);
        insta::assert_snapshot!("reef_tiny_7", plain(&g));
    }

    #[test]
    #[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
    fn deterministic_and_seed_sensitive() {
        let run = |seed: u64| {
            let (mut g, p) = make(80, 24, seed);
            let k = ReefKnobs::from_env();
            draw_reef(&mut g, 80, 24, seed, &p, 0.0, &k);
            plain(&g)
        };
        assert_eq!(run(42), run(42), "same seed -> same reef");
        assert_ne!(run(42), run(7), "new seed -> new reef");
    }

    #[test]
    #[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
    fn t_zero_is_the_static_frame_and_time_moves() {
        let run = |t: f32| {
            let (mut g, p) = make(80, 24, 42);
            let k = ReefKnobs::from_env();
            draw_reef(&mut g, 80, 24, 42, &p, t, &k);
            plain(&g)
        };
        assert_eq!(run(0.0), run(0.0), "t=0 is byte identical across calls");
        assert_ne!(run(0.0), run(3.5), "bubbles, fish and caustics move");
        assert_ne!(run(3.5), run(7.0), "motion keeps going");
    }

    #[test]
    #[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
    fn coral_holds_still_while_the_water_moves() {
        let frame = |t: f32| {
            let (mut g, p) = make(80, 24, 42);
            let mut k = ReefKnobs::from_env();
            k.fish_count = 0.0;
            k.bubble_density = 0.0;
            draw_reef(&mut g, 80, 24, 42, &p, t, &k);
            g
        };
        let a = frame(0.0);
        let b = frame(0.4);
        let trunk = ['@', '#', '%', '&'];
        let mut same = 0usize;
        let mut total = 0usize;
        for y in 0..24 {
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
    #[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
    fn every_glyph_is_one_column_wide() {
        use unicode_width::UnicodeWidthChar;
        let (mut g, p) = make(80, 24, 1701);
        let k = ReefKnobs::from_env();
        draw_reef(&mut g, 80, 24, 1701, &p, 2.0, &k);
        for row in &g {
            for cell in row {
                assert_eq!(cell.ch.width().unwrap_or(0), 1, "wide glyph {:?}", cell.ch);
            }
        }
    }

    #[test]
    #[cfg_attr(feature = "function-trace", tracing::instrument(level = "trace", target = "ascii_renderer::functions", skip_all))]
    fn knobs_change_the_reef() {
        let run = |f: fn(&mut ReefKnobs)| {
            let (mut g, p) = make(80, 24, 42);
            let mut k = ReefKnobs::from_env();
            f(&mut k);
            draw_reef(&mut g, 80, 24, 42, &p, 0.0, &k);
            plain(&g)
        };
        let base = run(|_| {});
        assert_ne!(base, run(|k| k.colony_count = 7.0), "colony count");
        assert_ne!(base, run(|k| k.water_depth_frac = 0.6), "water depth");
        assert_ne!(base, run(|k| k.fish_count = 0.0), "fish count");
    }
}
