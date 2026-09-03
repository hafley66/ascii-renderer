//! opus-2-forest: depth-layered stand of opus-2 species under a slow sky.
//! Seed picks the species mix, the density, the palette drift and the weather.

use crate::_0_profile::measure_layer;
use crate::color::{darken, lerp_color, lighten, shift_hue};
use crate::opts::param_f32;
use crate::opus_2_trees::{
    GrowOpts, Ink, SLOTS, SLOT_DIM, SLOT_TIP, Species, Stamp, bake, grow_species, hash2, ink_from,
    slot_ink,
};
use crate::types::{Cell, Grid, Rect};
use crossterm::style::Color;
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::cell::RefCell;

pub(crate) struct Opus2ForestKnobs {
    pub(crate) density: f32,
    pub(crate) layers: f32,
    pub(crate) sway: f32,
    pub(crate) speed: f32,
    pub(crate) hue: f32,
    pub(crate) atmos: f32,
    pub(crate) energy: f32,
    pub(crate) ground: f32,
    pub(crate) haze: f32,
    pub(crate) motes: f32,
    pub(crate) scale: f32,
    pub(crate) cycle: f32,
    pub(crate) fruit: f32,
    pub(crate) branch: f32,
    pub(crate) gnarl: f32,
    pub(crate) mix: f32,
}

impl Opus2ForestKnobs {
    pub(crate) fn from_env() -> Self {
        Opus2ForestKnobs {
            density: param_f32("DENSITY", 0.3).clamp(0.0, 1.0),
            layers: param_f32("LAYERS", 4.0).clamp(2.0, 6.0),
            sway: param_f32("SWAY", 1.2).clamp(0.0, 5.0),
            speed: param_f32("SPEED", 1.0).clamp(0.0, 4.0),
            hue: param_f32("HUE", 0.0).clamp(-180.0, 180.0),
            atmos: param_f32("ATMOS", 0.0).clamp(0.0, 5.0),
            energy: param_f32("ENERGY", 0.86).clamp(0.2, 1.0),
            ground: param_f32("GROUND", 0.5).clamp(0.12, 0.6),
            haze: param_f32("HAZE", 0.7).clamp(0.0, 1.0),
            motes: param_f32("MOTES", 1.0).clamp(0.0, 3.0),
            scale: param_f32("SCALE", 1.0).clamp(0.4, 2.0),
            cycle: param_f32("CYCLE", 44.0).clamp(20.0, 60.0),
            fruit: param_f32("FRUIT", 0.22).clamp(0.0, 1.0),
            branch: param_f32("BRANCH", 0.7).clamp(0.0, 1.0),
            gnarl: param_f32("GNARL", 0.35).clamp(0.0, 1.0),
            mix: param_f32("MIX", 0.5).clamp(0.0, 1.0),
        }
    }

    fn shape_bits(&self) -> [u32; 9] {
        [
            self.density.to_bits(),
            self.layers.to_bits(),
            self.energy.to_bits(),
            self.ground.to_bits(),
            self.scale.to_bits(),
            self.fruit.to_bits(),
            self.branch.to_bits(),
            self.gnarl.to_bits(),
            self.mix.to_bits(),
        ]
    }
}

const VARIANTS: usize = 6;
const TONES: usize = 3;

struct Placed {
    sprite: u16,
    x: i32,
    base_y: i32,
    layer: u8,
    tone: u8,
    flip: bool,
    phase: f32,
}

struct Stand {
    sprites: Vec<Stamp>,
    placed: Vec<Placed>,
    horizon: usize,
    layer_count: usize,
}

type StandKey = (usize, usize, u64, [u32; 9]);

thread_local! {
    static STAND: RefCell<Option<(StandKey, Stand)>> = const { RefCell::new(None) };
}

fn frac_hash(seed: u64, a: u64) -> f32 {
    (hash2(seed, a) % 100_000) as f32 / 100_000.0
}

/// Weighted species mix. MIX at 0 keeps the seed's lopsided draw, at 1 evens it out.
fn species_weights(seed: u64, mix: f32) -> [f32; 5] {
    let mut w = [0.0f32; 5];
    let mut sum = 0.0;
    for (i, slot) in w.iter_mut().enumerate() {
        let raw = 0.12 + frac_hash(seed, 700 + i as u64).powf(1.6);
        *slot = raw;
        sum += raw;
    }
    for slot in w.iter_mut() {
        *slot = (*slot / sum) * (1.0 - mix) + mix * 0.2;
    }
    w
}

fn pick_species(w: &[f32; 5], r: f32) -> usize {
    let mut acc = 0.0;
    for (i, wi) in w.iter().enumerate() {
        acc += *wi;
        if r <= acc {
            return i;
        }
    }
    4
}

fn build_stand(width: usize, height: usize, seed: u64, k: &Opus2ForestKnobs) -> Stand {
    let layer_count = k.layers.round() as usize;
    let horizon = ((height as f32) * (1.0 - k.ground)) as usize;
    let horizon = horizon.clamp(2, height.saturating_sub(3).max(2));
    let base_h = ((height as f32) * 0.55).clamp(6.0, 60.0) * k.scale;
    let ink = slot_ink();
    let weights = species_weights(seed, k.mix);
    let ground_span = (height - horizon).max(2) as f32;

    let mut sprites: Vec<Stamp> = Vec::with_capacity(layer_count * 5 * VARIANTS);
    for li in 0..layer_count {
        let lf = if layer_count > 1 {
            li as f32 / (layer_count - 1) as f32
        } else {
            1.0
        };
        let th = (base_h * (0.22 + 0.78 * lf.powf(1.4))).max(3.0) as usize;
        let tw = ((th as f32) * 1.35).max(5.0) as usize;
        for si in 0..5 {
            for v in 0..VARIANTS {
                let mut rng = StdRng::seed_from_u64(hash2(seed, (li * 97 + si * 13 + v) as u64));
                let energy = k.energy * (0.72 + 0.28 * lf) * (0.82 + 0.3 * frac_hash(seed, (li * 31 + si * 7 + v) as u64));
                let o = GrowOpts {
                    fruit: k.fruit * lf,
                    branch: k.branch,
                    gnarl: k.gnarl,
                    roots: 0.35 + 0.5 * lf,
                };
                let plot = Rect {
                    x: 1,
                    y: 1,
                    w: tw.saturating_sub(2).max(3),
                    h: th.saturating_sub(1).max(3),
                };
                let sp = Species::from_index(si);
                sprites.push(bake(tw, th, 0, |g| {
                    grow_species(sp, g, plot, energy.clamp(0.15, 1.0), &ink, &o, &mut rng);
                }));
            }
        }
    }

    let mut placed: Vec<Placed> = Vec::new();
    let mut prev_trunks: Vec<i32> = Vec::new();
    for li in 0..layer_count {
        let lf = if layer_count > 1 {
            li as f32 / (layer_count - 1) as f32
        } else {
            1.0
        };
        let th = (base_h * (0.22 + 0.78 * lf.powf(1.4))).max(3.0) as usize;
        let tw = ((th as f32) * 1.35).max(5.0) as usize;
        let base_y = horizon as f32 + ground_span * (0.05 + 0.92 * lf.powf(1.35));
        let base_y = (base_y as i32).min(height as i32 - 1);
        let far_gap = 0.82 + 0.55 * lf;
        let step = ((tw as f32) * (1.25 - 0.75 * k.density) * far_gap).max(2.0);
        let mut trunks: Vec<i32> = Vec::new();
        let mut x = -(tw as f32) * 0.6;
        let mut n = 0u64;
        while x < width as f32 + tw as f32 * 0.5 {
            let r1 = frac_hash(seed, (li as u64) * 4096 + n * 3 + 1);
            let r2 = frac_hash(seed, (li as u64) * 4096 + n * 3 + 2);
            let r3 = frac_hash(seed, (li as u64) * 4096 + n * 3 + 3);
            let mut si = pick_species(&weights, r1);
            if si == 3 && lf < 0.7 {
                si = (hash2(seed, n * 5 + 21) % 3) as usize;
            }
            if si == 4 && lf < 0.45 {
                si = (hash2(seed, n * 7 + 33) % 3) as usize;
            }
            let v = (hash2(seed, (li as u64) * 811 + n) % VARIANTS as u64) as usize;
            let mut px = x.round() as i32;
            let trunk = px + tw as i32 / 2;
            if prev_trunks.iter().any(|p| (p - trunk).abs() < 2) {
                px += 2;
            }
            if trunks.iter().any(|p| (p - (px + tw as i32 / 2)).abs() < 2) {
                px += 3;
            }
            trunks.push(px + tw as i32 / 2);
            placed.push(Placed {
                sprite: ((li * 5 + si) * VARIANTS + v) as u16,
                x: px,
                base_y: base_y + (r3 * 2.0) as i32 - 1,
                layer: li as u8,
                tone: (hash2(seed, n + li as u64 * 77) % TONES as u64) as u8,
                flip: hash2(seed, n * 13 + li as u64 * 131) % 2 == 0,
                phase: r2 * 6.283,
            });
            n += 1;
            x += step * (0.72 + 0.55 * r2);
            if n > 4000 {
                break;
            }
        }
        prev_trunks = trunks;
    }

    Stand {
        sprites,
        placed,
        horizon,
        layer_count,
    }
}

// ── sky, light, weather ─────────────────────────────────────────────

struct Sky {
    top: Color,
    horizon: Color,
    light: f32,
    tint: Color,
}

fn sky_at(palette: &[Color; 5], hue: f32, phase: f32) -> Sky {
    let p = |i: usize| shift_hue(palette[i], hue as f64);
    let keys = [
        (darken(p(0), 6), lerp_color(p(0), p(2), 0.45), 0.18, p(2)),
        (lerp_color(p(0), p(2), 0.4), lerp_color(p(3), p(1), 0.45), 0.55, p(3)),
        (lerp_color(p(2), p(4), 0.4), lerp_color(p(4), p(3), 0.55), 1.0, p(4)),
        (lerp_color(p(0), p(1), 0.35), lerp_color(p(1), p(3), 0.5), 0.5, p(1)),
    ];
    let f = phase * keys.len() as f32;
    let i = (f as usize) % keys.len();
    let j = (i + 1) % keys.len();
    let u = f - f.floor();
    Sky {
        top: lerp_color(keys[i].0, keys[j].0, u),
        horizon: lerp_color(keys[i].1, keys[j].1, u),
        light: keys[i].2 + (keys[j].2 - keys[i].2) * u,
        tint: lerp_color(keys[i].3, keys[j].3, u),
    }
}

fn layer_inks(
    palette: &[Color; 5],
    sky: &Sky,
    k: &Opus2ForestKnobs,
    layer_count: usize,
) -> Vec<Ink> {
    let mut out = Vec::with_capacity(layer_count * TONES);
    for li in 0..layer_count {
        let lf = if layer_count > 1 {
            li as f32 / (layer_count - 1) as f32
        } else {
            1.0
        };
        for tone in 0..TONES {
            let t = tone as f32 / (TONES - 1).max(1) as f32;
            let base = lerp_color(palette[1], palette[2], 0.25 + 0.5 * t);
            let base = shift_hue(base, (k.hue + t * 14.0 - 7.0) as f64);
            let lit = lerp_color(darken(base, 45), lighten(base, 25), sky.light);
            let hazed = lerp_color(sky.horizon, lit, 0.25 + 0.75 * lf.powf(0.8) * (1.0 - 0.55 * k.haze) + 0.55 * k.haze * lf);
            out.push(ink_from(hazed, lerp_color(palette[3], sky.tint, 0.35)));
        }
    }
    out
}

fn ridge_profile(x: f32, seed: u64, band: u64) -> f32 {
    let a = frac_hash(seed, 300 + band * 7) * 6.283;
    let b = frac_hash(seed, 301 + band * 7) * 6.283;
    let c = frac_hash(seed, 302 + band * 7) * 6.283;
    let s1 = (x * 0.013 + a).sin();
    let s2 = (x * 0.031 + b).sin() * 0.5;
    let s3 = (x * 0.071 + c).sin() * 0.25;
    ((s1 + s2 + s3) / 1.75).clamp(-1.0, 1.0)
}

fn atmos_kind(seed: u64, k: &Opus2ForestKnobs) -> usize {
    let forced = k.atmos.round() as usize;
    if forced >= 1 {
        return (forced - 1) % 5;
    }
    (hash2(seed, 4242) % 5) as usize
}

// ── the frame ───────────────────────────────────────────────────────

pub(crate) fn draw_opus_2_forest(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    k: &Opus2ForestKnobs,
) {
    if width < 8 || height < 6 {
        return;
    }
    let phase = if t > 0.0 {
        (t * k.speed / k.cycle).rem_euclid(1.0)
    } else {
        0.0
    };
    let sky = sky_at(palette, k.hue, phase);

    measure_layer("opus-2-forest", "grow", || {
        let key: StandKey = (width, height, seed, k.shape_bits());
        STAND.with(|s| {
            let mut slot = s.borrow_mut();
            let fresh = matches!(slot.as_ref(), Some((old, _)) if *old == key);
            if !fresh {
                *slot = Some((key, build_stand(width, height, seed, k)));
            }
        });
    });

    let horizon = STAND.with(|s| s.borrow().as_ref().map(|(_, st)| st.horizon).unwrap_or(2));
    let layer_count = STAND.with(|s| {
        s.borrow()
            .as_ref()
            .map(|(_, st)| st.layer_count)
            .unwrap_or(2)
    });
    let inks = layer_inks(palette, &sky, k, layer_count);

    measure_layer("opus-2-forest", "sky", || {
        paint_sky(grid, width, height, horizon, seed, &sky, t, k);
    });

    measure_layer("opus-2-forest", "ridges", || {
        paint_ridges(grid, width, horizon, seed, &sky, k);
    });

    measure_layer("opus-2-forest", "ground", || {
        paint_ground(grid, width, height, horizon, seed, palette, &sky, k);
    });

    measure_layer("opus-2-forest", "canopy", || {
        STAND.with(|s| {
            let slot = s.borrow();
            if let Some((_, st)) = slot.as_ref() {
                stamp_stand(grid, width, height, st, &inks, t, k);
            }
        });
    });

    measure_layer("opus-2-forest", "atmos", || {
        paint_atmos(grid, width, height, horizon, seed, palette, &sky, t, k);
    });
}

fn paint_sky(
    grid: &mut Grid,
    width: usize,
    height: usize,
    horizon: usize,
    seed: u64,
    sky: &Sky,
    t: f32,
    k: &Opus2ForestKnobs,
) {
    for (y, row) in grid.iter_mut().take(height).enumerate() {
        if y >= horizon {
            break;
        }
        let f = y as f32 / horizon.max(1) as f32;
        let bg = lerp_color(sky.top, sky.horizon, f.powf(1.6));
        let fill = Cell::with_bg(' ', bg, bg);
        for cell in row.iter_mut().take(width) {
            *cell = fill;
        }
    }

    let dark = (0.62 - sky.light).max(0.0) / 0.62;
    if dark > 0.02 {
        let stars = ((width * horizon) / 260) as u64;
        let star_c = lerp_color(sky.horizon, palette_white(), dark);
        for i in 0..stars {
            let h = hash2(seed, 5000 + i);
            let x = (h % width as u64) as i32;
            let y = ((h >> 20) % horizon.max(1) as u64) as i32;
            if (y as f32) > horizon as f32 * 0.82 {
                continue;
            }
            let tw = if t > 0.0 {
                ((t * 0.7 + (h >> 40) as f32 * 0.001).sin() * 0.5 + 0.5) * dark
            } else {
                dark
            };
            if tw < 0.25 {
                continue;
            }
            let ch = if tw > 0.8 { '∘' } else { '·' };
            set_cell(grid, x, y, ch, star_c, width, height);
        }
    }

    if sky.light > 0.42 {
        let wisps = 2 + (horizon / 9).min(4);
        let col = lerp_color(sky.top, palette_white(), 0.3 + 0.3 * sky.light);
        for b in 0..wisps as u64 {
            let h = hash2(seed, 7100 + b);
            let y = ((h % (horizon as u64).max(1)) as f32 * 0.75) as i32 + 1;
            let len = 6 + (h >> 9) as usize % (width / 3).max(6);
            let drift = if t > 0.0 { t * 0.35 * (1.0 + b as f32 * 0.3) } else { 0.0 };
            let x0 = (h >> 21) as f32 % width as f32 + drift;
            for j in 0..len {
                let x = (x0 as i32 + j as i32).rem_euclid(width as i32);
                let e = (j as f32 / len as f32 - 0.5).abs() * 2.0;
                if hash2(h, j as u64) % 5 == 0 {
                    continue;
                }
                let ch = if e < 0.45 { '░' } else { '·' };
                set_cell(grid, x, y, ch, col, width, height);
            }
        }
    }

    if sky.light < 0.62 {
        let mx = if t > 0.0 {
            (0.12 + (t * k.speed / k.cycle) * 0.9).rem_euclid(1.0)
        } else {
            0.12
        };
        let cx = (mx * width as f32) as i32;
        let cy = (horizon as f32 * 0.3) as i32 + 1;
        let r = ((horizon as f32) * 0.09).clamp(2.0, 7.0) as i32;
        let glow = lerp_color(sky.horizon, palette_white(), 0.75 - sky.light * 0.5);
        for dy in -r..=r {
            for dx in -(r * 2)..=(r * 2) {
                let d = (dx * dx) as f32 * 0.25 + (dy * dy) as f32;
                let rr = (r * r) as f32;
                if d > rr {
                    continue;
                }
                let ch = if d < rr * 0.45 { '▓' } else { '▒' };
                set_cell(grid, cx + dx, cy + dy, ch, glow, width, height);
            }
        }
    }
}

fn palette_white() -> Color {
    Color::Rgb {
        r: 235,
        g: 236,
        b: 240,
    }
}

fn set_cell(grid: &mut Grid, x: i32, y: i32, ch: char, c: Color, width: usize, height: usize) {
    if x < 0 || y < 0 || x as usize >= width || y as usize >= height {
        return;
    }
    let (ux, uy) = (x as usize, y as usize);
    if uy >= grid.len() || ux >= grid[uy].len() {
        return;
    }
    let bg = grid[uy][ux].bg;
    grid[uy][ux] = Cell::with_bg(ch, c, bg);
}

fn paint_ridges(
    grid: &mut Grid,
    width: usize,
    horizon: usize,
    seed: u64,
    sky: &Sky,
    k: &Opus2ForestKnobs,
) {
    let bands = 2;
    for b in 0..bands {
        let bf = (b + 1) as f32 / bands as f32;
        let amp = (horizon as f32) * (0.12 + 0.1 * bf);
        let base = horizon as f32 - amp * 0.25;
        let body = lerp_color(
            sky.horizon,
            shift_hue(darken(sky.tint, 90), k.hue as f64),
            0.55 + 0.35 * bf,
        );
        let crest = lerp_color(body, sky.horizon, 0.35);
        for x in 0..width {
            let p = ridge_profile(x as f32, seed, b as u64);
            let top = (base - amp * (0.5 + 0.5 * p)) as i32;
            for y in top..horizon as i32 {
                let ch = if y == top { '▒' } else { '▓' };
                let col = if y == top { crest } else { body };
                set_cell(grid, x as i32, y, ch, col, width, horizon);
            }
        }
    }
}

fn paint_ground(
    grid: &mut Grid,
    width: usize,
    height: usize,
    horizon: usize,
    seed: u64,
    palette: &[Color; 5],
    sky: &Sky,
    k: &Opus2ForestKnobs,
) {
    let span = (height - horizon).max(1) as f32;
    let near = shift_hue(darken(palette[1], 78), k.hue as f64);
    let far = lerp_color(sky.horizon, darken(palette[2], 60), 0.62);
    for y in horizon..height {
        let f = (y - horizon) as f32 / span;
        let bg = lerp_color(
            lerp_color(far, near, f.powf(0.7)),
            darken(palette[0], 0),
            0.35 * (1.0 - sky.light),
        );
        let fill = Cell::with_bg(' ', bg, bg);
        if y < grid.len() {
            for cell in grid[y].iter_mut().take(width) {
                *cell = fill;
            }
        }
        let tex = lerp_color(bg, lighten(near, 30), 0.22 + 0.3 * f);
        let stride = (16.0 - 11.0 * f).max(3.0) as usize;
        let mut x = (hash2(seed, 6000 + y as u64) % stride.max(1) as u64) as usize;
        while x < width {
            let h = hash2(seed, (y as u64) << 16 | x as u64);
            let ch = match h % 8 {
                0 | 1 | 2 => '·',
                3 | 4 => '╌',
                5 if f > 0.55 => '╵',
                6 if f > 0.75 => '│',
                _ => '░',
            };
            set_cell(grid, x as i32, y as i32, ch, tex, width, height);
            x += stride + (h >> 8) as usize % 5;
        }
    }

    let tufts = ((width as f32) * 0.05 * (1.0 + k.motes * 0.3)) as u64;
    let tuft_c = lerp_color(darken(palette[1], 40), lighten(palette[2], 20), 0.4 + 0.3 * sky.light);
    for i in 0..tufts {
        let h = hash2(seed, 6600 + i);
        let x = (h % width as u64) as i32;
        let y = height as i32 - 1 - ((h >> 19) % 3) as i32;
        let blades = 1 + (h >> 33) % 3;
        for b in 0..blades {
            let bx = x + b as i32 - 1;
            let ch = match (h >> (b * 3 + 5)) % 3 {
                0 => '╵',
                1 => '│',
                _ => '╷',
            };
            set_cell(grid, bx, y, ch, tuft_c, width, height);
            if b == 1 {
                set_cell(grid, bx, y - 1, '╵', tuft_c, width, height);
            }
        }
    }
}

fn mirror_glyph(c: char) -> char {
    match c {
        '╱' => '╲',
        '╲' => '╱',
        '├' => '┤',
        '┤' => '├',
        '╴' => '╶',
        '╶' => '╴',
        '╭' => '╮',
        '╮' => '╭',
        '╰' => '╯',
        '╯' => '╰',
        other => other,
    }
}

fn stamp_stand(
    grid: &mut Grid,
    width: usize,
    height: usize,
    st: &Stand,
    inks: &[Ink],
    t: f32,
    k: &Opus2ForestKnobs,
) {
    let animate = t > 0.0 && k.sway > 0.0;
    let gust = if animate {
        0.65 + 0.35 * (t * 0.11).sin()
    } else {
        0.0
    };
    for p in &st.placed {
        let sprite = match st.sprites.get(p.sprite as usize) {
            Some(s) => s,
            None => continue,
        };
        let sh = sprite.h as i32;
        if p.base_y - sh > height as i32 || p.x > width as i32 {
            continue;
        }
        let lf = if st.layer_count > 1 {
            p.layer as f32 / (st.layer_count - 1) as f32
        } else {
            1.0
        };
        let amp = if animate {
            k.sway * gust * (0.35 + 0.85 * lf) * (t * (0.19 + 0.05 * lf) + p.phase).sin()
        } else {
            0.0
        };
        let ink = &inks[(p.layer as usize * TONES + p.tone as usize).min(inks.len() - 1)];
        let far = st.layer_count >= 3 && p.layer == 0;
        let shf = (sh as f32).max(1.0);
        let sw = sprite.w as i32;
        for c in &sprite.cells {
            let up = (sh - 1 - c.y as i32) as f32 / shf;
            let dx = if animate {
                (amp * up * up).round() as i32
            } else {
                0
            };
            let (lx, mut ch) = if p.flip {
                (sw - 1 - c.x as i32, mirror_glyph(c.ch))
            } else {
                (c.x as i32, c.ch)
            };
            if far {
                ch = if c.slot >= 2 { '▒' } else { '▓' };
            }
            let gx = p.x + lx + dx;
            let gy = p.base_y - (sh - 1 - c.y as i32);
            set_cell(grid, gx, gy, ch, ink[c.slot as usize], width, height);
        }
    }
}

fn paint_atmos(
    grid: &mut Grid,
    width: usize,
    height: usize,
    horizon: usize,
    seed: u64,
    palette: &[Color; 5],
    sky: &Sky,
    t: f32,
    k: &Opus2ForestKnobs,
) {
    if k.motes <= 0.0 {
        return;
    }
    let kind = atmos_kind(seed, k);
    let n = (((width * height) as f32 / 220.0) * k.motes).clamp(12.0, 1500.0) as u64;
    let tt = if t > 0.0 { t * k.speed } else { 0.0 };
    match kind {
        0 => {
            let bands = (2.0 + k.motes * 1.5) as i32;
            let col = lerp_color(sky.horizon, palette_white(), 0.4);
            let span = (height - horizon) as f32;
            for b in 0..bands {
                let bf = b as f32 / bands.max(1) as f32;
                let y = (horizon as f32 - 1.0 + span * 0.42 * bf
                    + 1.5 * (tt * 0.05 + b as f32 * 1.7).sin()) as i32;
                let drift = tt * (2.0 + b as f32 * 0.7) * 0.4;
                let mut x = 0i32;
                while x < width as i32 {
                    let h = hash2(seed, (b as u64) * 733 + (x as u64 / 2));
                    let sx = (x + drift as i32).rem_euclid(width as i32);
                    if h % 3 == 0 {
                        set_cell(grid, sx, y, '~', col, width, height);
                    }
                    x += 2 + ((h >> 7) % 5) as i32;
                }
            }
        }
        1 => {
            let col = lerp_color(palette[3], palette_white(), 0.4);
            let band = (height - horizon).max(2) as f32;
            for i in 0..n {
                let h = hash2(seed, 8000 + i);
                let x0 = (h % width as u64) as f32;
                let y0 = horizon as f32 + ((h >> 17) % band as u64) as f32;
                let ph = ((h >> 33) % 1000) as f32 / 159.0;
                let ax = 3.0 + ((h >> 11) % 7) as f32;
                let x = x0 + ax * (tt * 0.19 + ph).sin();
                let y = y0 + 2.0 * (tt * 0.13 + ph * 1.7).cos();
                let tw = (tt * 0.9 + ph * 3.0).sin() * 0.5 + 0.5;
                if tw < 0.42 {
                    continue;
                }
                let ch = if tw > 0.86 {
                    '✦'
                } else if tw > 0.62 {
                    '∘'
                } else {
                    '·'
                };
                set_cell(
                    grid,
                    x as i32,
                    y as i32,
                    ch,
                    lerp_color(col, palette_white(), tw),
                    width,
                    height,
                );
            }
        }
        2 => {
            let col = lerp_color(palette[3], palette[1], 0.4);
            let span = height as f32;
            for i in 0..n {
                let h = hash2(seed, 9000 + i);
                let x0 = (h % width as u64) as f32;
                let ph = ((h >> 21) % 1000) as f32 / 159.0;
                let fall = 1.4 + ((h >> 13) % 5) as f32 * 0.5;
                let y = ((h >> 31) % span.max(1.0) as u64) as f32 + tt * fall;
                let y = y.rem_euclid(span);
                let x = x0 + (4.0 + ((h >> 9) % 5) as f32) * (tt * 0.35 + ph).sin();
                let ch = match (h >> 3) % 4 {
                    0 => '·',
                    1 => '◦',
                    2 => '▪',
                    _ => '◇',
                };
                set_cell(grid, x as i32, y as i32, ch, col, width, height);
            }
        }
        3 => {
            let col = lerp_color(sky.horizon, palette[4], 0.45);
            for i in 0..n {
                let h = hash2(seed, 11000 + i);
                let x0 = (h % width as u64) as f32;
                let speed = 14.0 + ((h >> 12) % 10) as f32;
                let y = (((h >> 25) % height.max(1) as u64) as f32 + tt * speed)
                    .rem_euclid(height as f32);
                let x = x0 + y * 0.35;
                set_cell(grid, x as i32, y as i32, '╱', col, width, height);
                if h % 3 == 0 {
                    set_cell(grid, x as i32 - 1, y as i32 + 1, '╱', darken(col, 30), width, height);
                }
            }
        }
        _ => {
            let flock = (3.0 + k.motes * 3.0) as u64;
            let col = darken(sky.tint, 40);
            for i in 0..flock {
                let h = hash2(seed, 13000 + i);
                let speed = 2.2 + ((h >> 8) % 5) as f32 * 0.6;
                let ph = ((h >> 19) % 1000) as f32 / 159.0;
                let x = ((h % width as u64) as f32 + tt * speed).rem_euclid(width as f32 + 8.0) - 4.0;
                let y = (horizon as f32 * (0.25 + 0.45 * ((h >> 29) % 100) as f32 / 100.0))
                    + 2.0 * (tt * 0.3 + ph).sin();
                let up = (tt * 2.2 + ph).sin() > 0.0;
                let (a, b) = if up { ('╱', '╲') } else { ('╲', '╱') };
                set_cell(grid, x as i32, y as i32, a, col, width, height);
                set_cell(grid, x as i32 + 1, y as i32, b, col, width, height);
            }
        }
    }
    let _ = (SLOT_DIM, SLOT_TIP, SLOTS);
}

pub(crate) fn cli_opus_2_forest(
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
    let mut k = Opus2ForestKnobs::from_env();
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
        k.ground = v;
    }
    if let Some(v) = args.get(12).and_then(|s| s.parse().ok()) {
        k.haze = v;
    }
    if let Some(v) = args.get(13).and_then(|s| s.parse().ok()) {
        k.motes = v;
    }
    if let Some(v) = args.get(14).and_then(|s| s.parse().ok()) {
        k.scale = v;
    }
    if let Some(v) = args.get(15).and_then(|s| s.parse().ok()) {
        k.cycle = v;
    }
    draw_opus_2_forest(&mut grid, width, height, seed, &palette, t_anim, &k);
    (grid, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(w: usize, h: usize, seed: u64, t: f32) -> String {
        let mut g = vec![vec![Cell::blank(); w]; h];
        let p = crate::color::make_palette(seed);
        let k = Opus2ForestKnobs::from_env();
        draw_opus_2_forest(&mut g, w, h, seed, &p, t, &k);
        g.iter()
            .map(|row| row.iter().map(|c| c.ch).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn snapshot_opus_2_forest_static() {
        insta::assert_snapshot!("opus_2_forest_80x24_static", run(80, 24, 42, 0.0));
    }

    #[test]
    fn snapshot_opus_2_forest_animated() {
        insta::assert_snapshot!("opus_2_forest_80x24_animated", run(80, 24, 42, 11.0));
    }

    #[test]
    fn deterministic_and_seed_sensitive() {
        assert_eq!(run(80, 24, 42, 0.0), run(80, 24, 42, 0.0));
        assert_ne!(run(80, 24, 42, 0.0), run(80, 24, 5, 0.0));
    }

    #[test]
    fn time_moves_the_forest() {
        assert_ne!(run(80, 24, 42, 0.0), run(80, 24, 42, 9.0));
    }
}
