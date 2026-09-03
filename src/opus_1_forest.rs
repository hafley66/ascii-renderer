//! opus-1-forest: depth-layered stand of the opus-1 species over a lit horizon,
//! with drifting weather and a slow day cycle. Geometry bakes once, frames blit.

use crate::_0_profile::measure_layer;
use crate::color::*;
use crate::opts::param_f32;
use crate::opus_1_trees::*;
use crate::types::*;
use crossterm::style::Color;
use rand::RngExt;
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::cell::RefCell;
use std::f32::consts::TAU;

const MAX_BAKED: usize = 1_400_000;
const MAX_MOTES: usize = 2600;

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
}

impl ForestKnobs {
    pub(crate) fn from_env() -> Self {
        ForestKnobs {
            density: param_f32("DENSITY", 1.0).clamp(0.25, 2.5),
            layers: param_f32("LAYERS", 4.0).clamp(3.0, 6.0),
            sway: param_f32("SWAY", 1.1).clamp(0.0, 4.0),
            speed: param_f32("SPEED", 1.0).clamp(0.0, 3.0),
            hue: param_f32("HUE", 0.0).clamp(-180.0, 180.0),
            atmos: param_f32("ATMOS", 0.0).clamp(0.0, 5.0),
            energy: param_f32("ENERGY", 0.9).clamp(0.3, 1.0),
            fruit: param_f32("FRUIT", 0.12).clamp(0.0, 1.0),
            branch: param_f32("BRANCH", 0.6).clamp(0.05, 1.0),
            detail: param_f32("DETAIL", 1.0).clamp(0.2, 2.0),
            cycle: param_f32("CYCLE", 42.0).clamp(20.0, 60.0),
            horizon: param_f32("HORIZON", 0.40).clamp(0.20, 0.70),
            bare: param_f32("BARE", 0.30).clamp(0.08, 0.60),
        }
    }
}

// ── atmosphere ──────────────────────────────────────────────────────

#[derive(Clone, Copy)]
struct Mote {
    x0: f32,
    y0: f32,
    ph: f32,
    sp: f32,
    amp: f32,
    span: f32,
}

/// 1 mist, 2 fireflies, 3 leaf fall, 4 rain, 5 snow.
fn atmos_kind(k: &ForestKnobs, seed: u64) -> u8 {
    let a = k.atmos.round() as u8;
    if a >= 1 && a <= 5 {
        a
    } else {
        1 + (seed.rotate_left(11) % 5) as u8
    }
}

// ── bake ────────────────────────────────────────────────────────────

struct ForestBake {
    key: (usize, usize, u64, u32, u32, u32, u32, u32, u32, u32, u32),
    sky: Vec<BakedCell>,
    ground: Vec<BakedCell>,
    trees: Vec<BakedCell>,
    motes: Vec<Mote>,
    phase: Vec<(f32, f32)>,
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
        q(k.atmos) ^ q(k.bare),
    )
}

/// Seeded species weights, renormalised so every seed draws a different stand.
fn species_mix(rng: &mut StdRng) -> [f32; 5] {
    let mut m = [0.0f32; 5];
    let mut sum = 0.0;
    for v in m.iter_mut() {
        *v = 0.12 + rng.random::<f32>().powf(1.7) * 1.4;
        sum += *v;
    }
    let mut acc = 0.0;
    for v in m.iter_mut() {
        acc += *v / sum;
        *v = acc;
    }
    m
}

fn pick_species(mix: &[f32; 5], r: f32, layer: usize, layers: usize) -> Species {
    let front = layers > 1 && layer + 1 == layers;
    let back = layer == 0;
    let mut i = 0;
    while i < 4 && r > mix[i] {
        i += 1;
    }
    if back && i == 1 {
        i = 2;
    }
    if front && i == 3 {
        i = 1;
    }
    Species::from_index(i)
}

fn bake_forest(w: usize, h: usize, seed: u64, k: &ForestKnobs, key: ForestKey) -> ForestBake {
    let layers = (k.layers.round() as usize).clamp(3, 6);
    let horizon = ((h as f32 * k.horizon) as i32).clamp(2, h as i32 - 3);
    let air_slot = layers as u16;
    let slots = layers + 1;

    let mut rng = StdRng::seed_from_u64(seed ^ 0xF0_1E57);
    let mix = species_mix(&mut rng);
    let kind = atmos_kind(k, seed);
    let tint = (seed.rotate_right(7) % 61) as f64 - 30.0;

    let mut sky: Vec<BakedCell> = Vec::new();
    let mut ground: Vec<BakedCell> = Vec::new();
    let mut trees: Vec<BakedCell> = Vec::new();
    let mut phase: Vec<(f32, f32)> = vec![(0.0, 0.0)];

    // sky: star field thinning toward the horizon, plus haze bands over it
    let mut band = Canvas::new(w, horizon.max(1) as usize);
    let star_salt = (seed & 0xFFFF) as u32 + 13;
    for y in 0..horizon {
        let up = 1.0 - y as f32 / horizon.max(1) as f32;
        for x in 0..w as i32 {
            let n = hashf(x, y, star_salt);
            if n > 0.006 + 0.020 * up {
                continue;
            }
            let g = if n < 0.004 { '·' } else if n < 0.010 { '∙' } else { '◦' };
            band.put(x, y, g, BAND_AIR, (12.0 + up * 16.0) as u8);
        }
    }
    let clouds = 2 + (star_salt % 3) as i32;
    for ci in 0..clouds {
        let cx = (hashf(ci, 3, star_salt) * w as f32) as i32;
        let cy = (horizon as f32 * (0.34 + hashf(ci, 9, star_salt) * 0.50)) as i32;
        let crx = (w as f32 * (0.04 + hashf(ci, 17, star_salt) * 0.05)).max(3.0);
        let cry = (horizon as f32 * 0.06 + 0.9).max(1.2);
        let x0 = (cx as f32 - crx) as i32;
        let x1 = (cx as f32 + crx) as i32;
        for y in (cy - cry as i32 - 1)..=(cy + cry as i32) {
            for x in x0..=x1 {
                let nx = (x - cx) as f32 / crx;
                let ny = (y - cy) as f32 / cry;
                let lift = 0.35 * (nx * 3.0 + ci as f32).sin();
                let d = (nx * nx + (ny + lift) * (ny + lift) * 1.4).sqrt();
                if d > 1.0 {
                    continue;
                }
                let n = hashf(x, y, star_salt ^ 0x7C);
                if n < d * d * 0.85 {
                    continue;
                }
                let g = if d < 0.45 { '▒' } else { '░' };
                band.put(x, y, g, BAND_AIR, (13.0 - d * 7.0) as u8);
            }
        }
    }
    let disc_r = ((h as f32 * 0.055).max(2.0)) as i32;
    let disc_x = (w as f32 * (0.12 + (seed % 71) as f32 / 100.0)) as i32;
    let disc_y = (horizon as f32 * (0.18 + (seed / 71 % 37) as f32 / 120.0)) as i32 + disc_r;
    for dy in -disc_r - 1..=disc_r + 1 {
        for dx in -(disc_r * 2) - 2..=(disc_r * 2) + 2 {
            let nx = dx as f32 / (disc_r as f32 * ASPECT);
            let ny = dy as f32 / disc_r as f32;
            let d = (nx * nx + ny * ny).sqrt();
            if d > 1.18 {
                continue;
            }
            let g = if d < 0.62 {
                '▒'
            } else if d < 0.94 {
                '░'
            } else {
                '·'
            };
            band.put(disc_x + dx, disc_y + dy, g, BAND_AIR, (30.0 - d * 14.0) as u8);
        }
    }
    capture(&band, 0, 0, air_slot, 0, 0, &mut sky);

    // ground: one textured band per depth layer, front bands denser and darker
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
            if n < 0.30 {
                continue;
            }
            gcan.put(x, 0, if n > 0.88 { '╴' } else { '─' }, BAND_ROOT, (10.0 + f * 12.0) as u8);
        }
        let tuft_rows = (bot - top).max(1);
        for x in 0..w as i32 {
            let seedn = hashf(x, li as i32, 0x7C3);
            if seedn > 0.06 + 0.16 * f * k.density {
                continue;
            }
            let hgt = 1 + ((seedn * 40.0) as i32 % (1 + (f * 3.0) as i32));
            let ly = (tuft_rows * (3 + (seedn * 90.0) as i32 % 5) / 8).clamp(1, tuft_rows);
            for d in 0..=hgt {
                let wid = (hgt - d).max(0);
                for dx in -wid..=wid {
                    let n = hashf(x + dx, ly - d, 0x2D9);
                    if n < 0.34 {
                        continue;
                    }
                    let g = if d == 0 { '▒' } else if n > 0.72 { '░' } else { '∙' };
                    gcan.put(x + dx, ly - d, g, BAND_LEAF, (10.0 + f * 10.0 + d as f32 * 3.0) as u8);
                }
            }
        }
        capture(&gcan, 0, top, li as u16, 0, 0, &mut ground);
    }

    // trees: one pass per layer, back to front
    let mut canvas = Canvas::new(8, 8);
    'layers: for li in 0..layers {
        let f = if layers > 1 { li as f32 / (layers - 1) as f32 } else { 1.0 };
        let base_y = ground_rows[li];
        let tree_h = ((base_y - horizon) as f32 * 1.15).max(5.0);
        let slot_w = (tree_h * 0.85).max(5.0);
        let overlap = (0.30 * k.density).clamp(0.0, 0.62);
        let step = (slot_w * (1.0 - overlap) * (1.0 + 1.3 * f)).max(3.0);

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
            let sp = pick_species(&mix, rng.random::<f32>(), li, layers);
            canvas.reset(pw as usize, (ph + 2) as usize);
            let plot = Plot {
                rect: Rect { x: 0, y: 0, w: pw as usize, h: ph as usize },
                energy: (k.energy * (0.72 + 0.34 * f) * (0.82 + rng.random::<f32>() * 0.32)).clamp(0.25, 1.0),
                fruit: k.fruit * (0.3 + f),
                branch: k.branch,
                roots: rng.random_range(0..4u32) as usize,
                detail: k.detail * (0.5 + 0.6 * f),
                bare: (k.bare * (0.75 + 0.85 * f)).clamp(0.10, 0.58),
            };
            let mut trng = StdRng::seed_from_u64(
                seed ^ ((li as u64) << 40) ^ ((cx.abs() as u64) << 8) ^ phase.len() as u64,
            );
            grow_species(sp, &mut canvas, &plot, &mut trng);
            let group = phase.len() as u16;
            capture(&canvas, px, py, li as u16, group, ph - 1, &mut trees);
            let period = 9.0 + rng.random::<f32>() * 13.0 - f * 2.0;
            phase.push((TAU / period.max(4.0), rng.random::<f32>() * TAU));
        }
    }

    // atmosphere seeds
    let mote_count = ((w * h) / 55).min(MAX_MOTES);
    let mut motes: Vec<Mote> = Vec::with_capacity(mote_count);
    for _ in 0..mote_count {
        let y0 = match kind {
            1 => horizon as f32 - rng.random::<f32>() * (horizon as f32 * 0.35) + rng.random::<f32>() * (h as f32 - horizon as f32) * 0.55,
            2 => horizon as f32 + rng.random::<f32>() * (h as f32 - horizon as f32) * 0.95,
            _ => rng.random::<f32>() * h as f32,
        };
        motes.push(Mote {
            x0: rng.random::<f32>() * w as f32,
            y0,
            ph: rng.random::<f32>() * TAU,
            sp: 0.35 + rng.random::<f32>() * 0.9,
            amp: 1.0 + rng.random::<f32>() * 5.0,
            span: h as f32 * (0.5 + rng.random::<f32>() * 0.7),
        });
    }

    ForestBake { key, sky, ground, trees, motes, phase, layers, slots, horizon, kind, tint }
}

// ── color ───────────────────────────────────────────────────────────

/// Four stops around the clock: day, dusk, night, dawn.
fn day_stop(p: f32, palette: &[Color; 5]) -> (Color, Color, f32) {
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
        let far = if b.layers > 1 {
            1.0 - li as f32 / (b.layers - 1) as f32
        } else {
            0.0
        };
        let fog = far * 0.78;
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
    for _ in 0..1 {
        ramp(&mut lut, darken(air, 120), darken(air, 40));
        ramp(&mut lut, darken(air, 90), air);
        ramp(&mut lut, darken(air, 70), lighten(air, 20));
        ramp(&mut lut, darken(palette[3], 40), lighten(palette[3], 40));
        ramp(&mut lut, darken(air, 110), darken(air, 30));
        ramp(&mut lut, darken(air, 130), lighten(air, 40));
    }
    lut
}

// ── frame ───────────────────────────────────────────────────────────

fn paint_atmos(grid: &mut Grid, w: usize, h: usize, b: &ForestBake, t: f32, speed: f32, col: Color, warm: Color) {
    let ts = t * speed;
    for m in &b.motes {
        let (x, y, g, bright) = match b.kind {
            1 => {
                let x = m.x0 + ts * m.sp * 1.6 + (ts * 0.2 + m.ph).sin() * m.amp;
                let y = m.y0 + (ts * 0.11 * m.sp + m.ph).sin() * 1.2;
                let g = if m.amp > 3.5 { '≈' } else { '~' };
                (x, y, g, 0.45)
            }
            2 => {
                let x = m.x0 + (ts * m.sp * 0.5 + m.ph).sin() * m.amp;
                let y = m.y0 + (ts * m.sp * 0.37 + m.ph * 1.7).cos() * m.amp * 0.45;
                let pulse = 0.5 + 0.5 * (ts * m.sp * 1.9 + m.ph).sin();
                let g = if pulse > 0.78 { '◦' } else if pulse > 0.42 { '∙' } else { '·' };
                (x, y, g, pulse)
            }
            3 => {
                let fall = (m.y0 + ts * m.sp * 2.4) % m.span;
                let x = m.x0 + (ts * m.sp * 0.9 + m.ph).sin() * m.amp;
                let g = if m.amp > 4.0 { '◆' } else if m.amp > 2.5 { '◇' } else { '·' };
                (x, fall, g, 0.7)
            }
            4 => {
                let fall = (m.y0 + ts * m.sp * 9.0) % m.span;
                let x = m.x0 + m.amp * 0.2;
                let g = if m.amp > 3.0 { '│' } else { '╵' };
                (x, fall, g, 0.5)
            }
            _ => {
                let fall = (m.y0 + ts * m.sp * 1.7) % m.span;
                let x = m.x0 + (ts * m.sp * 0.55 + m.ph).sin() * m.amp;
                let g = if m.amp > 4.2 { '◦' } else if m.amp > 2.4 { '∙' } else { '·' };
                (x, fall, g, 0.85)
            }
        };
        let xi = (x.round() as i32).rem_euclid(w as i32);
        let yi = y.round() as i32;
        if yi < 0 || yi as usize >= h {
            continue;
        }
        if b.kind == 2 && yi < b.horizon {
            continue;
        }
        let c = lerp_color(col, warm, bright.clamp(0.0, 1.0));
        grid[yi as usize][xi as usize] = Cell::new(g, c);
    }
}

pub(crate) fn draw_opus_1_forest(
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

    measure_layer("opus-1-forest", "clear", || {
        for row in grid.iter_mut().take(h) {
            row.fill(Cell::blank());
        }
    });

    FOREST.with(|store| {
        let mut store = store.borrow_mut();
        let key = forest_key(w, h, seed, k);
        if store.as_ref().map(|b| b.key) != Some(key) {
            *store = Some(measure_layer("opus-1-forest", "bake", || bake_forest(w, h, seed, k, key)));
        }
        let b = store.as_ref().unwrap();

        let p = ((t * k.speed) / k.cycle).rem_euclid(1.0);
        let (sky_top, haze, light) = day_stop(p, palette);
        let lut = build_forest_lut(palette, k, b, light, haze);

        let mut offs = vec![0i32; b.phase.len()];
        if t > 0.0 {
            for (i, &(om, ph)) in b.phase.iter().enumerate().skip(1) {
                offs[i] = (k.sway * 65536.0 * (t * k.speed * om + ph).sin()) as i32;
            }
        }

        measure_layer("opus-1-forest", "sky", || blit(grid, w, h, &b.sky, &lut, &offs));
        measure_layer("opus-1-forest", "ground", || blit(grid, w, h, &b.ground, &lut, &offs));
        measure_layer("opus-1-forest", "trees", || blit(grid, w, h, &b.trees, &lut, &offs));
        measure_layer("opus-1-forest", "atmos", || {
            let cool = lerp_color(sky_top, haze, 0.5);
            let warm = lerp_color(haze, palette[4], 0.55);
            paint_atmos(grid, w, h, b, t, k.speed, cool, warm);
        });
    });
}

pub(crate) fn cli_opus_1_forest(
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
    draw_opus_1_forest(&mut grid, width, height, seed, &palette, t_anim, &k);
    (grid, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(w: usize, h: usize, seed: u64, t: f32) -> String {
        let mut g = vec![vec![Cell::blank(); w]; h];
        let p = crate::color::make_palette(seed);
        let k = ForestKnobs::from_env();
        draw_opus_1_forest(&mut g, w, h, seed, &p, t, &k);
        g.iter()
            .map(|row| row.iter().map(|c| c.ch).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn snapshot_opus_1_forest_static() {
        insta::assert_snapshot!("opus_1_forest_80x24_static", run(80, 24, 42, 0.0));
    }

    #[test]
    fn snapshot_opus_1_forest_animated() {
        insta::assert_snapshot!("opus_1_forest_80x24_t12", run(80, 24, 42, 12.0));
    }

    #[test]
    fn opus_1_forest_is_deterministic() {
        assert_eq!(run(80, 24, 42, 0.0), run(80, 24, 42, 0.0));
        assert_ne!(run(80, 24, 42, 0.0), run(80, 24, 11, 0.0));
    }

    #[test]
    fn opus_1_forest_animates() {
        assert_ne!(run(80, 24, 42, 0.0), run(80, 24, 42, 6.0));
    }
}
