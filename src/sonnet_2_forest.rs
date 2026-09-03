//! sonnet-2-forest: layered stand of the sonnet-2 species with parallax sway,
//! tree-anchored atmosphere and a two-axis (brightness + warmth) light cycle.

use crate::_0_profile::measure_layer;
use crate::color::*;
use crate::opts::param_f32;
use crate::sonnet_2_trees::*;
use crate::types::*;
use crossterm::style::Color;
use rand::RngExt;
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::cell::RefCell;
use std::f32::consts::TAU;

pub(crate) struct ForestKnobs {
    pub density: f32,
    pub layers: f32,
    pub sway: f32,
    pub speed: f32,
    pub hue: f32,
    pub atmos: f32,
    pub haze: f32,
    pub energy: f32,
    pub ground: f32,
    pub fruit: f32,
    pub light: f32,
}

impl ForestKnobs {
    pub(crate) fn from_env() -> Self {
        ForestKnobs {
            density: param_f32("DENSITY", 1.0).clamp(0.2, 2.0),
            layers: param_f32("LAYERS", 3.0).clamp(1.0, 5.0),
            sway: param_f32("SWAY", 0.6).clamp(0.0, 2.0),
            speed: param_f32("SPEED", 1.0).clamp(0.2, 3.0),
            hue: param_f32("HUE", 0.0).clamp(0.0, 360.0),
            atmos: param_f32("ATMOS", 0.0).clamp(0.0, 5.0),
            haze: param_f32("HAZE", 0.5).clamp(0.0, 1.0),
            energy: param_f32("ENERGY", 0.9).clamp(0.4, 1.2),
            ground: param_f32("GROUND", 0.64).clamp(0.5, 0.9),
            fruit: param_f32("FRUIT", 0.15).clamp(0.0, 1.0),
            light: param_f32("LIGHT", 0.5).clamp(0.0, 1.0),
        }
    }
}

#[derive(PartialEq, Clone)]
struct Key {
    w: usize,
    h: usize,
    seed: u64,
    palette: [Color; 5],
    density: f32,
    layers: f32,
    hue: f32,
    atmos: f32,
    haze: f32,
    energy: f32,
    ground: f32,
    fruit: f32,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Atmos {
    Fogbanks,
    Fireflies,
    Leaves,
    Snow,
    Birds,
}

const ATMOS_KINDS: [Atmos; 5] = [Atmos::Fogbanks, Atmos::Fireflies, Atmos::Leaves, Atmos::Snow, Atmos::Birds];

struct Particle {
    x: f32,
    y: f32,
    phase: f32,
    rate: f32,
    amp: f32,
    var: u8,
}

struct FogBank {
    x0: f32,
    y: f32,
    rx: f32,
    ry: f32,
    speed: f32,
}

struct Tree {
    sprite: Sprite,
    x: i32,
    y: i32,
    lfrac: f32,
    phase: f32,
}

struct Scene {
    key: Key,
    pal: Palette,
    bg_ch: Vec<char>,
    bg_slot: Vec<u16>,
    stars: Vec<(u16, u16, u8)>,
    star_slots: [u16; 2],
    horizon: usize,
    trees: Vec<Tree>,
    canopy_cells: Vec<(f32, f32)>,
    atmos: Atmos,
    parts: Vec<Particle>,
    banks: Vec<FogBank>,
    atm_slots: [u16; 3],
    tufts: Vec<(i32, i32, u16, u8)>,
}

thread_local! {
    static SCENE: RefCell<Option<Scene>> = const { RefCell::new(None) };
}

fn rf(rng: &mut StdRng) -> f32 {
    rng.random::<f32>()
}

fn hue_of_or(c: Color, fallback: f64) -> f64 {
    match c {
        Color::Rgb { r, g, b } => {
            let max = r.max(g).max(b);
            let min = r.min(g).min(b);
            if max - min < 4 {
                return fallback;
            }
            let d = (max - min) as f64;
            let h = if max == r {
                ((g as f64 - b as f64) / d + if g < b { 6.0 } else { 0.0 }) * 60.0
            } else if max == g {
                ((b as f64 - r as f64) / d + 2.0) * 60.0
            } else {
                ((r as f64 - g as f64) / d + 4.0) * 60.0
            };
            h.rem_euclid(360.0)
        }
        _ => fallback,
    }
}

fn is_canopy_glyph(ch: char) -> bool {
    matches!(ch, '●' | '•' | '∙' | '·' | '◆' | '◇' | '○' | '◦' | '▪' | '▫')
}

fn build_scene(key: Key, k: &ForestKnobs) -> Scene {
    let w = key.w;
    let h = key.h;
    let mut rng = StdRng::seed_from_u64(key.seed ^ 0x50EE_2F02);
    let mut pal = Palette::new();
    let hue0 = (palette_hue(&key.palette) + key.hue as f64 + (rf(&mut rng) as f64 - 0.5) * 30.0).rem_euclid(360.0);
    let sky_hue = (hue_of_or(key.palette[0], hue0 + 200.0) + key.hue as f64).rem_euclid(360.0);
    let horizon = ((h as f32 * key.ground) as usize).clamp(2, h - 2);

    let a1 = h as f32 * 0.02;
    let a2 = h as f32 * 0.01;
    let (f1, f2) = (0.9 / w as f32 * TAU * (0.6 + rf(&mut rng)), 2.7 / w as f32 * TAU * (0.6 + rf(&mut rng)));
    let (p1, p2) = (rf(&mut rng) * TAU, rf(&mut rng) * TAU);
    let gl: Vec<usize> = (0..w)
        .map(|x| {
            let v = horizon as f32 + a1 * (x as f32 * f1 + p1).sin() + a2 * (x as f32 * f2 + p2).sin();
            (v.round() as i32).clamp(1, h as i32 - 2) as usize
        })
        .collect();

    let mut bg_ch = vec![' '; w * h];
    let mut bg_slot = vec![0u16; w * h];
    let sky_bands = 48usize;
    let sky_slots: Vec<u16> = (0..sky_bands)
        .map(|b| {
            let f = b as f64 / (sky_bands - 1) as f64;
            pal.intern(hsl_to_rgb(sky_hue, 0.35, 0.04 + 0.13 * f.powf(1.6)))
        })
        .collect();
    let ground_bands = 24usize;
    let ground_hue = (hue0 - 30.0).rem_euclid(360.0);
    let ground_slots: Vec<u16> = (0..ground_bands)
        .map(|b| {
            let f = b as f64 / (ground_bands - 1) as f64;
            pal.intern(hsl_to_rgb(ground_hue, 0.32, 0.22 - 0.13 * f))
        })
        .collect();
    let line_slot = pal.intern(hsl_to_rgb(ground_hue, 0.3, 0.3));
    let ground_glyphs = ['·', '·', ',', '~', '"', '∙', '·', ','];
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            if y < gl[x] {
                let f = y as f32 / horizon.max(1) as f32;
                bg_slot[i] = sky_slots[((f * (sky_bands - 1) as f32) as usize).min(sky_bands - 1)];
            } else if y == gl[x] {
                bg_ch[i] = if hash3(x as u32, 7, key.seed as u32) % 5 == 0 { ' ' } else { '╌' };
                bg_slot[i] = line_slot;
            } else {
                let depth = (y - gl[x]) as f32 / (h - horizon).max(1) as f32;
                let band = ((depth * (ground_bands - 1) as f32) as usize).min(ground_bands - 1);
                bg_slot[i] = ground_slots[band];
                let hv = hash3(x as u32, y as u32, key.seed as u32);
                let keep = if y - gl[x] <= 1 { 40 } else { 14 };
                if hv % 100 < keep {
                    bg_ch[i] = ground_glyphs[((hv >> 8) % ground_glyphs.len() as u32) as usize];
                }
            }
        }
    }
    let star_slots = [pal.intern(hsl_to_rgb(sky_hue, 0.2, 0.55)), pal.intern(hsl_to_rgb(sky_hue, 0.2, 0.3))];
    let mut stars = Vec::new();
    for y in 0..(horizon * 3 / 4) {
        for x in 0..w {
            if hash3(x as u32, y as u32, key.seed as u32 ^ 0x57A5) % 45 == 0 {
                stars.push((x as u16, y as u16, (hash3(x as u32, y as u32, 9) % 250) as u8));
            }
        }
    }

    let dom = rng.random_range(0..SPECIES.len() as u32) as usize;
    let mut sec = rng.random_range(0..SPECIES.len() as u32) as usize;
    if sec == dom {
        sec = (sec + 1) % SPECIES.len();
    }
    let weights: Vec<f32> = (0..SPECIES.len())
        .map(|i| if i == dom { 0.42 } else if i == sec { 0.24 } else { 0.34 / (SPECIES.len() as f32 - 2.0) })
        .collect();
    let pick_species = |rng: &mut StdRng| {
        let mut u = rf(rng);
        for (i, wgt) in weights.iter().enumerate() {
            if u < *wgt {
                return SPECIES[i];
            }
            u -= wgt;
        }
        SPECIES[dom]
    };

    let layers = (key.layers.round() as usize).clamp(1, 5);
    let haze_color = hsl_to_rgb(sky_hue, 0.3, 0.2);
    let gk = GrowKnobs { fruit: key.fruit, branch: 0.85, detail: 1.0, roots: 1.0 };
    let ovl = 0.05 + 0.4 * ((key.density - 0.2) / 1.8).clamp(0.0, 1.0);
    let mut trees: Vec<Tree> = Vec::new();
    let mut canopy_cells: Vec<(f32, f32)> = Vec::new();
    for li in 0..layers {
        let lfrac = if layers > 1 { li as f32 / (layers - 1) as f32 } else { 1.0 };
        let th_max = (h as f32 * key.energy * (0.3 + 0.48 * lfrac)).max(5.0);
        let avg_w = th_max * 1.5;
        let n = (((w as f32 / avg_w) * key.density * 0.75) as usize).clamp(1, 400);
        let mut spans: Vec<(i32, i32)> = Vec::new();
        let mut layer_trees: Vec<Tree> = Vec::new();
        let mut attempts = 0;
        while layer_trees.len() < n && attempts < n * 12 {
            attempts += 1;
            let th = ((th_max * (0.72 + 0.28 * rf(&mut rng))) as usize).max(5);
            let pw = ((th as f32 * (1.2 + 0.6 * rf(&mut rng))) as usize).clamp(5, w.max(5));
            let cx = (rf(&mut rng) * (w as f32 + avg_w * 0.6) - avg_w * 0.3) as i32;
            let x0 = cx - pw as i32 / 2;
            let x1 = x0 + pw as i32;
            let clash = spans.iter().any(|&(a0, a1)| {
                let o = a1.min(x1) - a0.max(x0);
                o > ((a1 - a0).min(pw as i32) as f32 * ovl) as i32
            });
            if clash {
                continue;
            }
            spans.push((x0, x1));
            let gx = cx.clamp(0, w as i32 - 1) as usize;
            let root_y = (gl[gx] as f32 + lfrac * (h - horizon) as f32 * 0.7) as usize;
            let root_y = root_y.min(h - 2);
            let rd = ((th / 8).max(1) as i32).min((h - 1 - root_y) as i32);
            let sp = pick_species(&mut rng);
            let hue = (hue0 + (rf(&mut rng) as f64 - 0.5) * 40.0 + lfrac as f64 * 10.0).rem_euclid(360.0);
            let sat = 0.3 + 0.35 * lfrac as f64;
            let light = 0.18 + 0.22 * lfrac as f64;
            let ink = Ink::from_hue(hue, sat, light).fade(haze_color, key.haze * (1.0 - lfrac) * 0.7);
            let mut scratch = vec![vec![Cell::blank(); pw]; th + rd as usize];
            let mut trng = StdRng::seed_from_u64(key.seed ^ hash3(li as u32, layer_trees.len() as u32, 0x7EE) as u64);
            grow_species(sp, &mut scratch, Rect { x: 0, y: 0, w: pw, h: th }, rd, 0.88 + 0.12 * rf(&mut rng), &ink, &gk, &mut trng);
            let sprite = sprite_from_grid(&scratch, th - 1, &mut pal);
            let phase = cx as f32 / w as f32 * 2.0 + rf(&mut rng) * 0.6;
            let ty = root_y as i32 - (th as i32 - 1);
            if lfrac > 0.4 {
                for (dy, row) in scratch.iter().enumerate() {
                    for (dx, cell) in row.iter().enumerate() {
                        if is_canopy_glyph(cell.ch) {
                            canopy_cells.push(((x0 + dx as i32) as f32, (ty + dy as i32) as f32));
                        }
                    }
                }
            }
            layer_trees.push(Tree { sprite, x: x0, y: ty, lfrac, phase });
        }
        layer_trees.sort_by_key(|t| t.y + t.sprite.root_row as i32);
        trees.extend(layer_trees);
    }

    let tuft_slots = [pal.intern(hsl_to_rgb(hue0 + 10.0, 0.45, 0.26)), pal.intern(hsl_to_rgb(hue0 - 5.0, 0.4, 0.34))];
    let mut tufts = Vec::new();
    let front_drop = ((h - horizon) as f32 * 0.5) as usize;
    for x in 0..w {
        if rf(&mut rng) < 0.18 {
            tufts.push((x as i32, gl[x] as i32 - 1, tuft_slots[0], (rf(&mut rng) * 250.0) as u8));
        }
        if rf(&mut rng) < 0.3 {
            let y = (gl[x] + front_drop).min(h - 1) as i32;
            tufts.push((x as i32, y, tuft_slots[1], (rf(&mut rng) * 250.0) as u8));
        }
    }

    let atmos = if key.atmos >= 1.0 { ATMOS_KINDS[(key.atmos.round() as usize - 1).min(4)] } else { ATMOS_KINDS[rng.random_range(0..5u32) as usize] };
    let accent = key.palette[3];
    let pale = key.palette[4];
    let mut banks = Vec::new();
    let (atm_slots, parts) = match atmos {
        Atmos::Fogbanks => {
            let n = 2 + rng.random_range(0..3u32);
            for _ in 0..n {
                banks.push(FogBank {
                    x0: rf(&mut rng) * w as f32,
                    y: horizon as f32 * (0.5 + rf(&mut rng) * 0.4),
                    rx: w as f32 * (0.12 + rf(&mut rng) * 0.1),
                    ry: (h as f32 * 0.06).max(2.0),
                    speed: 0.15 + rf(&mut rng) * 0.2,
                });
            }
            ([pal.intern(hsl_to_rgb(sky_hue, 0.22, 0.24)), pal.intern(hsl_to_rgb(sky_hue, 0.22, 0.32)), 0], Vec::new())
        }
        Atmos::Fireflies => {
            let n = ((w * h) / 900).clamp(8, 500);
            let ps = (0..n)
                .map(|i| {
                    let (hx, hy) = if !canopy_cells.is_empty() { canopy_cells[i % canopy_cells.len()] } else { (rf(&mut rng) * w as f32, horizon as f32) };
                    Particle { x: hx, y: hy + rf(&mut rng) * 2.0, phase: rf(&mut rng), rate: 0.12 + rf(&mut rng) * 0.25, amp: 1.5 + rf(&mut rng) * 3.0, var: 0 }
                })
                .collect();
            ([pal.intern(darken(accent, 60)), pal.intern(accent), pal.intern(lighten(accent, 40))], ps)
        }
        Atmos::Leaves => {
            let n = ((w * h) / 1500).clamp(6, 400);
            let warm = hsl_to_rgb(hue0 + 40.0, 0.6, 0.45);
            let ps = (0..n)
                .map(|i| {
                    let (sx, sy) = if !canopy_cells.is_empty() { canopy_cells[(i * 7) % canopy_cells.len()] } else { (rf(&mut rng) * w as f32, 0.0) };
                    Particle { x: sx, y: sy, phase: rf(&mut rng) * TAU, rate: 0.5 + rf(&mut rng) * 0.7, amp: 2.5 + rf(&mut rng) * 4.0, var: (rf(&mut rng) * 4.0) as u8 }
                })
                .collect();
            ([pal.intern(darken(warm, 30)), pal.intern(warm), pal.intern(lighten(warm, 30))], ps)
        }
        Atmos::Snow => {
            let n = ((w * h) / 700).clamp(10, 1200);
            let ps = (0..n)
                .map(|_| Particle { x: rf(&mut rng) * w as f32, y: rf(&mut rng) * h as f32, phase: rf(&mut rng) * TAU, rate: 0.8 + rf(&mut rng) * 0.8, amp: 2.0 + rf(&mut rng) * 2.0, var: (rf(&mut rng) * 3.0) as u8 })
                .collect();
            ([pal.intern(darken(pale, 70)), pal.intern(darken(pale, 20)), 0], ps)
        }
        Atmos::Birds => {
            let flocks = 2 + rng.random_range(0..3u32);
            let mut ps = Vec::new();
            for _ in 0..flocks {
                let fx = rf(&mut rng) * w as f32;
                let fy = horizon as f32 * (0.15 + rf(&mut rng) * 0.4);
                let phase = rf(&mut rng) * TAU;
                let rate = 0.7 + rf(&mut rng) * 0.6;
                let n = 5 + rng.random_range(0..5u32) as i32;
                for i in 0..n {
                    let k = (i + 1) / 2;
                    let sgn = if i % 2 == 0 { 1.0 } else { -1.0 };
                    ps.push(Particle { x: fx - k as f32 * 3.0, y: fy + sgn * k as f32 * (if i == 0 { 0.0 } else { 1.0 }), phase, rate, amp: rf(&mut rng) * TAU, var: 0 });
                }
            }
            ([pal.intern(darken(pale, 90)), pal.intern(darken(pale, 50)), 0], ps)
        }
    };

    Scene { key, pal, bg_ch, bg_slot, stars, star_slots, horizon, trees, canopy_cells, atmos, parts, banks, atm_slots, tufts }
}

fn set(grid: &mut Grid, gw: usize, gh: usize, x: i32, y: i32, ch: char, c: Color) {
    if x >= 0 && y >= 0 && (x as usize) < gw && (y as usize) < gh {
        grid[y as usize][x as usize] = Cell::new(ch, c);
    }
}

fn paint_atmos(grid: &mut Grid, gw: usize, gh: usize, s: &Scene, lit: &[Color], t: f32, ts: f32, animating: bool) {
    let c0 = lit[s.atm_slots[0] as usize];
    let c1 = lit[s.atm_slots[1] as usize];
    let c2 = lit[s.atm_slots[2].min(lit.len() as u16 - 1) as usize];
    match s.atmos {
        Atmos::Fogbanks => {
            for b in &s.banks {
                let span = gw as f32 + 3.0 * b.rx;
                let bx = if animating { (b.x0 + ts * b.speed * gw as f32 / 30.0).rem_euclid(span) - b.rx } else { b.x0 };
                for dy in -(b.ry as i32)..=(b.ry as i32) {
                    let row_scale = (1.0 - (dy as f32 / b.ry).powi(2)).max(0.0).sqrt();
                    let hw = b.rx * row_scale;
                    for dx in -(hw as i32)..=(hw as i32) {
                        let d = ((dx as f32 / hw.max(1.0)).powi(2) + (dy as f32 / b.ry).powi(2)).sqrt();
                        if d > 1.0 {
                            continue;
                        }
                        let (ch, c) = if d < 0.5 { ('▒', c1) } else { ('░', c0) };
                        set(grid, gw, gh, (bx + dx as f32).round() as i32, (b.y + dy as f32).round() as i32, ch, c);
                    }
                }
            }
        }
        Atmos::Fireflies => {
            for p in &s.parts {
                let b = if animating { 0.5 + 0.5 * (TAU * (p.rate * ts + p.phase)).sin() } else { 0.5 + 0.5 * (TAU * p.phase).sin() };
                if b < 0.35 {
                    continue;
                }
                let x = p.x + p.amp * (ts * 0.2 + p.phase * 3.0).sin();
                let y = p.y + p.amp * 0.35 * (ts * 0.13 + p.phase * 5.0).sin();
                let (ch, c) = if b < 0.6 { ('·', c0) } else if b < 0.85 { ('•', c1) } else { ('✦', c2) };
                set(grid, gw, gh, x.round() as i32, y.round() as i32, ch, c);
            }
        }
        Atmos::Leaves => {
            let glyphs = [',', '·', '∙', '"'];
            for (i, p) in s.parts.iter().enumerate() {
                let y = (p.y + ts * p.rate).rem_euclid(gh as f32 + 2.0) - 1.0;
                let x = p.x + p.amp * (ts * 0.5 + p.phase).sin();
                let tumble = ((t * 2.0) as u32 + i as u32 + p.var as u32) % 4;
                let c = if i % 3 == 0 { c0 } else if i % 3 == 1 { c1 } else { c2 };
                set(grid, gw, gh, x.round() as i32, y.round() as i32, glyphs[tumble as usize], c);
            }
        }
        Atmos::Snow => {
            let glyphs = ['*', '·', '.'];
            for p in &s.parts {
                let y = (p.y + ts * p.rate).rem_euclid(gh as f32);
                let x = p.x + p.amp * (ts * 0.35 + p.phase).sin();
                let c = if p.var == 0 { c1 } else { c0 };
                set(grid, gw, gh, x.round() as i32, y as i32, glyphs[p.var as usize % 3], c);
            }
        }
        Atmos::Birds => {
            let span = gw as f32 + 30.0;
            for p in &s.parts {
                let x = (p.x + ts * p.rate * gw as f32 / 45.0).rem_euclid(span) - 15.0;
                let y = p.y + 2.0 * (ts * 0.1 + p.phase).sin();
                let flap = ((ts * 1.5 + p.amp) as u32) % 3;
                let ch = ['v', '~', '-'][flap as usize];
                let c = if flap == 1 { c1 } else { c0 };
                set(grid, gw, gh, x.round() as i32, y.round() as i32, ch, c);
            }
        }
    }
}

pub(crate) fn draw_sonnet_2_forest(grid: &mut Grid, width: usize, height: usize, seed: u64, palette: &[Color; 5], t: f32, k: &ForestKnobs) {
    let gh = height.min(grid.len());
    let gw = width.min(grid.first().map(|r| r.len()).unwrap_or(0));
    if gw < 4 || gh < 6 {
        return;
    }
    let key = Key { w: gw, h: gh, seed, palette: *palette, density: k.density, layers: k.layers, hue: k.hue, atmos: k.atmos, haze: k.haze, energy: k.energy, ground: k.ground, fruit: k.fruit };
    let hit = SCENE.with(|c| c.borrow().as_ref().map(|s| s.key == key).unwrap_or(false));
    if !hit {
        let scene = measure_layer("sonnet-2-forest", "build", || build_scene(key.clone(), k));
        SCENE.with(|c| *c.borrow_mut() = Some(scene));
    }
    let animating = t > 0.0;
    let ts = if animating { t * k.speed } else { 0.0 };
    SCENE.with(|c| {
        let b = c.borrow();
        let s = b.as_ref().unwrap();
        let lit: Vec<Color> = measure_layer("sonnet-2-forest", "light", || {
            let cyc = TAU * ts / 46.0;
            let bright = k.light * cyc.sin();
            let warm_target = hsl_to_rgb(28.0, 0.55, 0.55);
            let cool_target = hsl_to_rgb(224.0, 0.4, 0.3);
            let warmth = k.light * 0.35 * cyc.cos();
            s.pal
                .colors
                .iter()
                .map(|&c| {
                    let stage1 = if bright >= 0.0 { lerp_color(c, lighten(c, 40), bright) } else { lerp_color(c, darken(c, 40), -bright) };
                    if warmth >= 0.0 { lerp_color(stage1, warm_target, warmth.min(0.3)) } else { lerp_color(stage1, cool_target, (-warmth).min(0.3)) }
                })
                .collect()
        });
        measure_layer("sonnet-2-forest", "backdrop", || {
            for y in 0..gh {
                let line = &mut grid[y];
                let row_ch = &s.bg_ch[y * gw..(y + 1) * gw];
                let row_slot = &s.bg_slot[y * gw..(y + 1) * gw];
                for x in 0..gw {
                    line[x] = Cell::new(row_ch[x], lit[row_slot[x] as usize]);
                }
            }
        });
        measure_layer("sonnet-2-forest", "sky", || {
            for &(x, y, ph) in &s.stars {
                let tw = if animating { (ts * 0.7 + ph as f32 * 0.1).sin() } else { (ph as f32 * 0.1).sin() };
                let c = if tw > 0.2 { lit[s.star_slots[0] as usize] } else { lit[s.star_slots[1] as usize] };
                grid[y as usize][x as usize] = Cell::new('·', c);
            }
        });
        measure_layer("sonnet-2-forest", "trees", || {
            let tick = (t * 1.5) as u32;
            for tr in &s.trees {
                let amp = k.sway * (0.3 + 0.7 * tr.lfrac) * tr.sprite.root_row as f32 * 0.07;
                let wind = 0.6 * (TAU * ts / 22.0 + tr.phase).sin() + 0.4 * (TAU * ts / 9.5 + tr.phase * 1.7).sin();
                let sway = if animating { amp * wind } else { 0.0 };
                let flicker = if animating { 0.1 + 0.1 * wind.abs() } else { 0.0 };
                blit_sprite(grid, gw, gh, &tr.sprite, tr.x, tr.y, sway, flicker, tick, &lit, Some(Cell::blank()));
            }
        });
        measure_layer("sonnet-2-forest", "undergrowth", || {
            let glyphs = [',', '"', ';', 'w', '∙'];
            for &(x, y, slot, ph) in &s.tufts {
                if y < 0 || y >= gh as i32 || x < 0 || x >= gw as i32 {
                    continue;
                }
                let idx = if animating { (((ts * 1.2 + ph as f32 * 0.05).sin() * 0.5 + 0.5) * 4.99) as usize } else { (ph as usize) % 5 };
                grid[y as usize][x as usize] = Cell::new(glyphs[idx.min(4)], lit[slot as usize]);
            }
        });
        measure_layer("sonnet-2-forest", "atmos", || paint_atmos(grid, gw, gh, s, &lit, t, ts, animating));
    });
}

pub(crate) fn cli_sonnet_2_forest(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], _rng: StdRng, t_anim: f32, _term_w: u16, _term_h: u16, args: &[String], _mode: &str, _theme_name: &str) -> (Grid, bool) {
    let mut k = ForestKnobs::from_env();
    let slots: [&mut f32; 11] = [&mut k.density, &mut k.layers, &mut k.sway, &mut k.speed, &mut k.hue, &mut k.atmos, &mut k.haze, &mut k.energy, &mut k.ground, &mut k.fruit, &mut k.light];
    for (i, slot) in slots.into_iter().enumerate() {
        if let Some(v) = args.get(4 + i).and_then(|s| s.parse().ok()) {
            *slot = v;
        }
    }
    draw_sonnet_2_forest(&mut grid, width, height, seed, &palette, t_anim, &k);
    (grid, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(w: usize, h: usize, seed: u64, t: f32) -> String {
        let mut g = vec![vec![Cell::blank(); w]; h];
        let p = crate::color::make_palette(seed);
        let k = ForestKnobs::from_env();
        draw_sonnet_2_forest(&mut g, w, h, seed, &p, t, &k);
        g.iter().map(|row| row.iter().map(|c| c.ch).collect::<String>()).collect::<Vec<_>>().join("\n")
    }

    #[test]
    fn snapshot_sonnet_2_forest_static() {
        insta::assert_snapshot!("sonnet_2_forest_80x24_static", run(80, 24, 42, 0.0));
    }

    #[test]
    fn deterministic_seed_and_time_sensitive() {
        assert_eq!(run(80, 24, 42, 0.0), run(80, 24, 42, 0.0));
        assert_ne!(run(80, 24, 42, 0.0), run(80, 24, 7, 0.0));
        assert_ne!(run(80, 24, 42, 0.0), run(80, 24, 42, 6.0));
    }

    #[test]
    fn every_atmosphere_renders() {
        for a in 1..=5u32 {
            let mut g = vec![vec![Cell::blank(); 80]; 24];
            let p = crate::color::make_palette(3);
            let mut k = ForestKnobs::from_env();
            k.atmos = a as f32;
            draw_sonnet_2_forest(&mut g, 80, 24, 3, &p, 4.0, &k);
            assert!(g.iter().flatten().any(|c| c.ch != ' '));
        }
    }
}
