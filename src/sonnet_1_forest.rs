//! sonnet-1-forest: a depth-layered stand of the sonnet-1 species with a
//! day cycle and one of three atmospheres that touch the trees themselves.

use crate::_0_profile::measure_layer;
use crate::color::*;
use crate::opts::param_f32;
use crate::sonnet_1_trees::*;
use crate::types::*;
use crossterm::style::Color;
use rand::RngExt;
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::cell::RefCell;
use std::f32::consts::TAU;

const MAX_BAKED: usize = 1_200_000;
const MAX_MOTES: usize = 1400;
const MAX_COLONIST: usize = 46;

pub(crate) struct ForestKnobs {
    pub density: f32,
    pub layers: f32,
    pub sway: f32,
    pub speed: f32,
    pub hue: f32,
    pub atmos: f32,
    pub energy: f32,
    pub fruit: f32,
    pub branch: f32,
    pub detail: f32,
    pub cycle: f32,
    pub horizon: f32,
    pub bare: f32,
    pub wind: f32,
}

impl ForestKnobs {
    pub(crate) fn from_env() -> Self {
        ForestKnobs {
            density: param_f32("DENSITY", 1.0).clamp(0.25, 2.5),
            layers: param_f32("LAYERS", 4.0).clamp(3.0, 6.0),
            sway: param_f32("SWAY", 1.1).clamp(0.0, 4.0),
            speed: param_f32("SPEED", 1.0).clamp(0.0, 3.0),
            hue: param_f32("HUE", 0.0).clamp(-180.0, 180.0),
            atmos: param_f32("ATMOS", 0.0).clamp(0.0, 3.0),
            energy: param_f32("ENERGY", 0.88).clamp(0.3, 1.0),
            fruit: param_f32("FRUIT", 0.12).clamp(0.0, 1.0),
            branch: param_f32("BRANCH", 0.58).clamp(0.05, 1.0),
            detail: param_f32("DETAIL", 1.0).clamp(0.2, 2.0),
            cycle: param_f32("CYCLE", 40.0).clamp(20.0, 60.0),
            horizon: param_f32("HORIZON", 0.40).clamp(0.20, 0.70),
            bare: param_f32("BARE", 0.28).clamp(0.08, 0.60),
            wind: param_f32("WIND", 0.35).clamp(-1.0, 1.0),
        }
    }
}

// ── atmosphere ──────────────────────────────────────────────────────

/// 1 fog banks occluding trunks, 2 fireflies boxed under canopy, 3 leaf fall.
fn atmos_kind(k: &ForestKnobs, seed: u64) -> u8 {
    let a = k.atmos.round() as u8;
    if (1..=3).contains(&a) {
        a
    } else {
        1 + (seed.rotate_left(13) % 3) as u8
    }
}

#[derive(Clone, Copy)]
struct FogBand {
    y0: f32,
    thick: f32,
    x0: f32,
    sp: f32,
    ph: f32,
}

#[derive(Clone, Copy)]
struct Firefly {
    box_i: u32,
    ph: f32,
    sp: f32,
}

// ── bake ────────────────────────────────────────────────────────────

struct ForestBake {
    key: ForestKey,
    sky: Vec<BakedCell>,
    ground: Vec<BakedCell>,
    trees: Vec<BakedCell>,
    trunks: Vec<u32>,
    leaves: Vec<u32>,
    canopy: Vec<(f32, f32, f32, f32)>,
    fog: Vec<FogBand>,
    flies: Vec<Firefly>,
    phase: Vec<(f32, f32, f32)>,
    layers: usize,
    slots: usize,
    horizon: i32,
    kind: u8,
    tint: f64,
}

thread_local! {
    static FOREST: RefCell<Option<ForestBake>> = const { RefCell::new(None) };
}

fn q(v: f32) -> u32 {
    (v * 1000.0) as u32
}

type ForestKey = (usize, usize, u64, u32, u32, u32, u32, u32, u32, u32, u32);

fn forest_key(w: usize, h: usize, seed: u64, k: &ForestKnobs) -> ForestKey {
    (
        w,
        h,
        seed,
        q(k.density),
        q(k.layers),
        q(k.energy),
        q(k.fruit),
        q(k.branch),
        q(k.detail),
        q(k.horizon),
        q(k.atmos) ^ q(k.bare) ^ q(k.wind),
    )
}

/// Seeded species weights, renormalised so every seed draws a different stand.
fn species_mix(rng: &mut StdRng) -> [f32; 6] {
    let mut m = [0.0f32; 6];
    let mut sum = 0.0;
    for v in m.iter_mut() {
        *v = 0.10 + rng.random::<f32>().powf(1.7) * 1.3;
        sum += *v;
    }
    let mut acc = 0.0;
    for v in m.iter_mut() {
        acc += *v / sum;
        *v = acc;
    }
    m
}

fn pick_species(mix: &[f32; 6], r: f32, layer: usize, layers: usize, colonist_left: &mut usize) -> Species {
    let back = layer == 0;
    let front = layers > 1 && layer + 1 == layers;
    let mut i = 0;
    while i < 5 && r > mix[i] {
        i += 1;
    }
    if back && i == 5 {
        i = 0;
    }
    if front && i == 0 {
        i = 3;
    }
    let mut sp = Species::from_index(i);
    if sp == Species::Colonist {
        if *colonist_left == 0 {
            sp = Species::PropRoot;
        } else {
            *colonist_left -= 1;
        }
    }
    sp
}

fn bake_forest(w: usize, h: usize, seed: u64, k: &ForestKnobs, key: ForestKey) -> ForestBake {
    let layers = (k.layers.round() as usize).clamp(3, 6);
    let horizon = ((h as f32 * k.horizon) as i32).clamp(2, h as i32 - 3);
    let air_slot = layers as u16;
    let slots = layers + 1;

    let mut rng = StdRng::seed_from_u64(seed ^ 0x50_17E5);
    let mix = species_mix(&mut rng);
    let kind = atmos_kind(k, seed);
    let tint = (seed.rotate_right(9) % 61) as f64 - 30.0;
    let wind_base = (k.wind + (rng.random::<f32>() - 0.5) * 0.5).clamp(-1.0, 1.0);

    let mut sky: Vec<BakedCell> = Vec::new();
    let mut ground: Vec<BakedCell> = Vec::new();
    let mut trees: Vec<BakedCell> = Vec::new();
    let mut trunks: Vec<u32> = Vec::new();
    let mut leaves: Vec<u32> = Vec::new();
    let mut canopy: Vec<(f32, f32, f32, f32)> = Vec::new();
    let mut phase: Vec<(f32, f32, f32)> = vec![(0.0, 0.0, 0.0)];

    let mut band = Canvas::new(w, horizon.max(1) as usize);
    let star_salt = (seed & 0xFFFF) as u32 + 17;
    for y in 0..horizon {
        let up = 1.0 - y as f32 / horizon.max(1) as f32;
        for x in 0..w as i32 {
            let n = hashf(x, y, star_salt);
            if n > 0.006 + 0.018 * up {
                continue;
            }
            let g = if n < 0.004 { '·' } else if n < 0.010 { '∙' } else { '◦' };
            band.put(x, y, g, BAND_AIR, (12.0 + up * 16.0) as u8);
        }
    }
    let disc_r = ((h as f32 * 0.05).max(2.0)) as i32;
    let disc_x = (w as f32 * (0.14 + (seed % 71) as f32 / 100.0)) as i32;
    let disc_y = (horizon as f32 * (0.20 + (seed / 71 % 37) as f32 / 120.0)) as i32 + disc_r;
    for dy in -disc_r - 1..=disc_r + 1 {
        for dx in -(disc_r * 2) - 2..=(disc_r * 2) + 2 {
            let nx = dx as f32 / (disc_r as f32 * ASPECT);
            let ny = dy as f32 / disc_r as f32;
            let d = (nx * nx + ny * ny).sqrt();
            if d > 1.15 {
                continue;
            }
            let g = if d < 0.6 { '▒' } else if d < 0.92 { '░' } else { '·' };
            band.put(disc_x + dx, disc_y + dy, g, BAND_AIR, (30.0 - d * 14.0) as u8);
        }
    }
    capture(&band, 0, 0, air_slot, 0, 0, &mut sky);

    let mut ground_rows: Vec<i32> = Vec::with_capacity(layers);
    for li in 0..layers {
        let f = (li + 1) as f32 / layers as f32;
        let y = horizon + ((h as i32 - 1 - horizon) as f32 * f.powf(1.35)) as i32;
        ground_rows.push(y.min(h as i32 - 1));
    }
    let mut gcan = Canvas::new(w, (h as i32 - horizon).max(1) as usize);
    for li in 0..layers {
        let top = if li == 0 { horizon } else { ground_rows[li - 1] };
        let bot = ground_rows[li];
        let f = (li + 1) as f32 / layers as f32;
        let dense = (0.05 + f * 0.13) * (0.6 + k.density * 0.5);
        gcan.reset(w, (bot - top + 1).max(1) as usize);
        for ly in 0..=(bot - top) {
            let dy = ly as f32 / (bot - top).max(1) as f32;
            for x in 0..w as i32 {
                let n = hashf(x, top + ly, 0x9A1 + li as u32);
                if n > dense * (0.45 + dy) {
                    continue;
                }
                let g = if n < dense * 0.40 { '∙' } else { '·' };
                gcan.put(x, ly, g, BAND_ROOT, (6.0 + dy * 12.0 + f * 8.0) as u8);
            }
        }
        for x in 0..w as i32 {
            let n = hashf(x, top, 0x5E1 + li as u32);
            if n < 0.55 {
                continue;
            }
            gcan.put(x, 0, if n > 0.88 { '╴' } else { '─' }, BAND_ROOT, (10.0 + f * 12.0) as u8);
        }
        capture(&gcan, 0, top, li as u16, 0, 0, &mut ground);
    }

    let mut canvas = Canvas::new(8, 8);
    let mut colonist_left = MAX_COLONIST;
    'layers: for li in 0..layers {
        let f = if layers > 1 { li as f32 / (layers - 1) as f32 } else { 1.0 };
        let base_y = ground_rows[li];
        let tree_h = ((base_y - horizon) as f32 * 1.15).max(5.0);
        let slot_w = (tree_h * 0.85).max(5.0);
        let overlap = (0.22 * k.density).clamp(0.0, 0.45);
        let step = (slot_w * (1.0 - overlap) * (1.0 + 1.6 * f)).max(3.0);
        let depth_scale = 0.25 + 0.75 * f;

        let mut x = -slot_w * 0.4 + rng.random::<f32>() * step;
        while x < w as f32 + slot_w * 0.4 {
            let cx = x;
            x += step * (0.80 + rng.random::<f32>() * 0.44);
            if trees.len() > MAX_BAKED {
                break 'layers;
            }
            let pw = (slot_w * (0.72 + rng.random::<f32>() * 0.60)) as i32;
            let ph = (tree_h * (0.70 + rng.random::<f32>() * 0.58)) as i32;
            if pw < 5 || ph < 6 {
                continue;
            }
            let px = cx as i32 - pw / 2;
            let py = base_y - ph + 1;
            if px + pw < 0 || px >= w as i32 {
                continue;
            }
            let sp = pick_species(&mix, rng.random::<f32>(), li, layers, &mut colonist_left);
            canvas.reset(pw as usize, (ph + 2) as usize);
            let plot = Plot {
                rect: Rect { x: 0, y: 0, w: pw as usize, h: ph as usize },
                energy: (k.energy * (0.72 + 0.34 * f) * (0.82 + rng.random::<f32>() * 0.32)).clamp(0.25, 1.0),
                fruit: k.fruit * (0.3 + f),
                branch: k.branch,
                roots: rng.random_range(0..4u32) as usize,
                detail: k.detail * (0.5 + 0.6 * f),
                bare: (k.bare * (0.75 + 0.85 * f)).clamp(0.10, 0.58),
                wind: (wind_base + (rng.random::<f32>() - 0.5) * 0.25).clamp(-1.0, 1.0),
            };
            let mut trng = StdRng::seed_from_u64(seed ^ ((li as u64) << 40) ^ ((cx.abs() as u64) << 8) ^ phase.len() as u64);
            grow_species(sp, &mut canvas, &plot, &mut trng);
            let group = phase.len() as u16;
            let start = trees.len();
            capture(&canvas, px, py, li as u16, group, ph - 1, &mut trees);
            for idx in start..trees.len() {
                let band_i = (trees[idx].pal as usize % PAL_STRIDE) / TONES;
                if band_i == BAND_LEAF as usize && leaves.len() < 9000 {
                    leaves.push(idx as u32);
                } else if band_i == BAND_TRUNK as usize && trunks.len() < 9000 && idx % 2 == 0 {
                    trunks.push(idx as u32);
                }
            }
            canopy.push((px as f32 + pw as f32 * 0.5, py as f32 + ph as f32 * 0.35, pw as f32 * 0.5, ph as f32 * 0.3));
            let period = 8.0 + rng.random::<f32>() * 12.0 - f * 2.0;
            phase.push((TAU / period.max(4.0), rng.random::<f32>() * TAU, depth_scale));
        }
    }

    let mut fog: Vec<FogBand> = Vec::new();
    let mut flies: Vec<Firefly> = Vec::new();
    if kind == 1 {
        let n = 3 + (w / 40).clamp(2, 9);
        for i in 0..n {
            fog.push(FogBand {
                y0: horizon as f32 + rng.random::<f32>() * (h as f32 - horizon as f32) * 0.85,
                thick: 1.5 + rng.random::<f32>() * 2.5,
                x0: rng.random::<f32>() * w as f32,
                sp: 0.3 + rng.random::<f32>() * 0.7,
                ph: i as f32 * 1.7,
            });
        }
    } else if kind == 2 && !canopy.is_empty() {
        let n = ((w * h) / 90).min(MAX_MOTES);
        for _ in 0..n {
            flies.push(Firefly {
                box_i: rng.random_range(0..canopy.len() as u32),
                ph: rng.random::<f32>() * TAU,
                sp: 0.4 + rng.random::<f32>() * 1.1,
            });
        }
    }

    ForestBake { key, sky, ground, trees, trunks, leaves, canopy, fog, flies, phase, layers, slots, horizon, kind, tint }
}

// ── color ───────────────────────────────────────────────────────────

/// Four stops around the clock: day, dusk, night, dawn.
fn light_stop(p: f32, palette: &[Color; 5]) -> (Color, Color, f32) {
    let tops = [
        lerp_color(palette[0], palette[2], 0.42),
        palette[0],
        darken(palette[0], 12),
        lerp_color(palette[0], palette[2], 0.22),
    ];
    let hzs = [
        lerp_color(palette[3], palette[4], 0.28),
        lighten(palette[1], 30),
        lerp_color(palette[0], palette[2], 0.34),
        lerp_color(palette[3], palette[1], 0.45),
    ];
    let lights = [1.0f32, 0.62, 0.30, 0.68];
    let s = p * 4.0;
    let i = (s as usize) % 4;
    let j = (i + 1) % 4;
    let f = s - s.floor();
    (
        lerp_color(tops[i], tops[j], f),
        lerp_color(hzs[i], hzs[j], f),
        lights[i] + (lights[j] - lights[i]) * f,
    )
}

fn build_forest_lut(palette: &[Color; 5], k: &ForestKnobs, b: &ForestBake, light: f32, haze: Color) -> Vec<Color> {
    let mut lut = Vec::with_capacity(b.slots * PAL_STRIDE);
    let hue = k.hue as f64 + b.tint;
    let dim = (light * 255.0) as i32;
    let sink = |c: Color| -> Color {
        let d = (200 - dim).clamp(0, 200) as u8;
        darken(c, d)
    };
    for li in 0..b.layers {
        let far = if b.layers > 1 { 1.0 - li as f32 / (b.layers - 1) as f32 } else { 0.0 };
        let fog = far * 0.76;
        let bark = lerp_color(sink(shift_hue(palette[2], hue)), haze, fog);
        let limb = lerp_color(sink(shift_hue(palette[1], hue)), haze, fog);
        let leaf = lerp_color(sink(shift_hue(palette[1], hue + 16.0)), haze, fog);
        let bloom = lerp_color(sink(shift_hue(palette[3], hue)), haze, fog * 0.7);
        let soil = lerp_color(sink(darken(shift_hue(palette[2], hue), 30)), haze, fog);
        ramp(&mut lut, darken(bark, 55), lighten(bark, 18));
        ramp(&mut lut, darken(limb, 50), lighten(limb, 30));
        ramp(&mut lut, darken(leaf, 62), lighten(leaf, 48));
        ramp(&mut lut, darken(bloom, 30), lighten(bloom, 50));
        ramp(&mut lut, darken(soil, 60), lighten(soil, 14));
        ramp(&mut lut, darken(haze, 60), lighten(haze, 20));
    }
    let air = lerp_color(haze, palette[4], 0.35);
    ramp(&mut lut, darken(air, 120), darken(air, 40));
    ramp(&mut lut, darken(air, 90), air);
    ramp(&mut lut, darken(air, 70), lighten(air, 20));
    ramp(&mut lut, darken(palette[3], 40), lighten(palette[3], 40));
    ramp(&mut lut, darken(air, 110), darken(air, 30));
    ramp(&mut lut, darken(air, 130), lighten(air, 40));
    lut
}

// ── frame ───────────────────────────────────────────────────────────

fn paint_fog(grid: &mut Grid, w: usize, h: usize, b: &ForestBake, offs: &[i32], t: f32, speed: f32, col: Color) {
    let ts = t * speed;
    for band in &b.fog {
        let cx = (band.x0 + ts * band.sp * 3.0).rem_euclid(w as f32 + 20.0) - 10.0;
        for dy in 0..(band.thick as i32).max(1) {
            let y = (band.y0 as i32 + dy) as usize;
            if y >= h {
                continue;
            }
            for dx in -14i32..=14 {
                let x = (cx as i32 + dx).rem_euclid(w as i32) as usize;
                let n = hashf(x as i32, y as i32, 0x0F06) + (0.4 * (dx as f32 * 0.3 + band.ph + ts * 0.6).sin());
                if n < 0.65 {
                    continue;
                }
                let g = if n > 0.95 { '≈' } else { '~' };
                grid[y][x] = Cell::new(g, col);
            }
        }
    }
    for &ti in &b.trunks {
        let c = b.trees[ti as usize];
        let dx = (offs[c.group as usize] * c.sway as i32) >> 24;
        let x = c.x as i32 + dx;
        let y = c.y as i32;
        if x < 0 || x as usize >= w || y < 0 || y as usize >= h {
            continue;
        }
        for band in &b.fog {
            let cx = (band.x0 + ts * band.sp * 3.0).rem_euclid(w as f32 + 20.0) - 10.0;
            if (y as f32 - band.y0).abs() > band.thick {
                continue;
            }
            let dxf = (x as f32 - cx).rem_euclid(w as f32);
            let near = dxf.min(w as f32 - dxf);
            if near > 9.0 {
                continue;
            }
            let veil = 1.0 - near / 9.0;
            if hashf(x, y, 0x9CC1) < veil * 0.8 {
                grid[y as usize][x as usize] = Cell::new(if veil > 0.6 { '≈' } else { '~' }, col);
            }
        }
    }
}

fn paint_flies(grid: &mut Grid, w: usize, h: usize, b: &ForestBake, t: f32, speed: f32, warm: Color) {
    let ts = t * speed;
    for fly in &b.flies {
        let (cx, cy, rx, ry) = b.canopy[fly.box_i as usize];
        let x = cx + (ts * fly.sp + fly.ph).sin() * rx * 0.8;
        let y = cy + (ts * fly.sp * 0.7 + fly.ph * 1.6).cos() * ry * 0.8;
        let pulse = 0.5 + 0.5 * (ts * fly.sp * 2.1 + fly.ph).sin();
        let xi = x.round() as i32;
        let yi = y.round() as i32;
        if xi < 0 || xi as usize >= w || yi < 0 || yi as usize >= h {
            continue;
        }
        let g = if pulse > 0.8 { '◦' } else if pulse > 0.5 { '∙' } else { continue };
        grid[yi as usize][xi as usize] = Cell::new(g, warm);
    }
}

fn paint_leaffall(grid: &mut Grid, w: usize, h: usize, b: &ForestBake, offs: &[i32], t: f32, speed: f32, lut: &[Color]) {
    if b.leaves.is_empty() {
        return;
    }
    let n = b.leaves.len();
    let span = (n / 5).max(1);
    let start = ((t * speed * 4.0) as usize).wrapping_mul(131) % n;
    for j in 0..span {
        let idx = b.leaves[(start + j) % n];
        let c = b.trees[idx as usize];
        let dx0 = (offs[c.group as usize] * c.sway as i32) >> 24;
        let x0 = c.x as i32 + dx0;
        let seed_f = hashf(x0, c.y as i32, 0xF411 + j as u32);
        let ts = t * speed * (0.6 + seed_f * 0.8) + seed_f * 37.0;
        let fall = ts % (h as f32 * 0.6 + 6.0);
        let x = x0 as f32 + (ts * 0.7).sin() * 1.6;
        let y = c.y as f32 + fall;
        let xi = x.round() as i32;
        let yi = y.round() as i32;
        if xi < 0 || xi as usize >= w || yi < 0 || yi as usize >= h {
            continue;
        }
        let base = c.pal - (c.pal % TONES as u16);
        let tone = (c.pal % TONES as u16).min(TONES as u16 - 1);
        grid[yi as usize][xi as usize] = Cell::new(if seed_f > 0.5 { '◇' } else { '·' }, lut[(base + tone) as usize]);
    }
}

pub(crate) fn draw_sonnet_1_forest(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    k: &ForestKnobs,
) {
    let h = height.min(grid.len());
    let w = if h == 0 { 0 } else { width.min(grid[0].len()) };
    if w < 10 || h < 8 {
        return;
    }

    measure_layer("sonnet-1-forest", "clear", || {
        for row in grid.iter_mut().take(h) {
            row.fill(Cell::blank());
        }
    });

    FOREST.with(|store| {
        let mut store = store.borrow_mut();
        let key = forest_key(w, h, seed, k);
        if store.as_ref().map(|b| b.key) != Some(key) {
            *store = Some(measure_layer("sonnet-1-forest", "bake", || bake_forest(w, h, seed, k, key)));
        }
        let b = store.as_ref().unwrap();

        let p = ((t * k.speed) / k.cycle).rem_euclid(1.0);
        let (sky_top, haze, light) = light_stop(p, palette);
        let lut = build_forest_lut(palette, k, b, light, haze);

        let mut offs = vec![0i32; b.phase.len()];
        if t > 0.0 {
            for (i, &(om, ph, depth)) in b.phase.iter().enumerate().skip(1) {
                offs[i] = (k.sway * depth * 65536.0 * (t * k.speed * om + ph).sin()) as i32;
            }
        }

        measure_layer("sonnet-1-forest", "sky", || blit(grid, w, h, &b.sky, &lut, &offs));
        measure_layer("sonnet-1-forest", "ground", || blit(grid, w, h, &b.ground, &lut, &offs));
        measure_layer("sonnet-1-forest", "trees", || blit(grid, w, h, &b.trees, &lut, &offs));

        if t > 0.0 {
            measure_layer("sonnet-1-forest", "atmos", || {
                let cool = lerp_color(sky_top, haze, 0.5);
                let warm = lerp_color(haze, palette[4], 0.55);
                match b.kind {
                    1 => paint_fog(grid, w, h, b, &offs, t, k.speed, cool),
                    2 => paint_flies(grid, w, h, b, t, k.speed, warm),
                    _ => paint_leaffall(grid, w, h, b, &offs, t, k.speed, &lut),
                }
            });
        }
    });
}

pub(crate) fn cli_sonnet_1_forest(
    mut grid: Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: [Color; 5],
    _rng: StdRng,
    t_anim: f32,
    _term_w: u16,
    _term_h: u16,
    args: &[String],
    _mode: &str,
    _theme_name: &str,
) -> (Grid, bool) {
    let mut k = ForestKnobs::from_env();
    if let Some(v) = args.get(4).and_then(|s| s.parse().ok()) {
        k.density = v;
    }
    if let Some(v) = args.get(5).and_then(|s| s.parse().ok()) {
        k.layers = v;
    }
    if let Some(v) = args.get(6).and_then(|s| s.parse().ok()) {
        k.sway = v;
    }
    if let Some(v) = args.get(7).and_then(|s| s.parse().ok()) {
        k.speed = v;
    }
    if let Some(v) = args.get(8).and_then(|s| s.parse().ok()) {
        k.hue = v;
    }
    if let Some(v) = args.get(9).and_then(|s| s.parse().ok()) {
        k.atmos = v;
    }
    if let Some(v) = args.get(10).and_then(|s| s.parse().ok()) {
        k.energy = v;
    }
    if let Some(v) = args.get(11).and_then(|s| s.parse().ok()) {
        k.fruit = v;
    }
    if let Some(v) = args.get(12).and_then(|s| s.parse().ok()) {
        k.branch = v;
    }
    if let Some(v) = args.get(13).and_then(|s| s.parse().ok()) {
        k.detail = v;
    }
    if let Some(v) = args.get(14).and_then(|s| s.parse().ok()) {
        k.cycle = v;
    }
    if let Some(v) = args.get(15).and_then(|s| s.parse().ok()) {
        k.horizon = v;
    }
    draw_sonnet_1_forest(&mut grid, width, height, seed, &palette, t_anim, &k);
    (grid, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(w: usize, h: usize, seed: u64, t: f32) -> String {
        let mut g = vec![vec![Cell::blank(); w]; h];
        let p = crate::color::make_palette(seed);
        let k = ForestKnobs::from_env();
        draw_sonnet_1_forest(&mut g, w, h, seed, &p, t, &k);
        g.iter()
            .map(|row| row.iter().map(|c| c.ch).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn snapshot_sonnet_1_forest_static() {
        insta::assert_snapshot!("sonnet_1_forest_80x24_static", run(80, 24, 42, 0.0));
    }

    #[test]
    fn snapshot_sonnet_1_forest_animated() {
        insta::assert_snapshot!("sonnet_1_forest_80x24_t12", run(80, 24, 42, 12.0));
    }

    #[test]
    fn sonnet_1_forest_is_deterministic() {
        assert_eq!(run(80, 24, 42, 0.0), run(80, 24, 42, 0.0));
        assert_ne!(run(80, 24, 42, 0.0), run(80, 24, 11, 0.0));
    }

    #[test]
    fn sonnet_1_forest_animates() {
        assert_ne!(run(80, 24, 42, 0.0), run(80, 24, 42, 6.0));
    }
}
