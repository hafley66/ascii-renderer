//! fable-1-forest: depth-layered forest of the fable-1 species under a slow sky,
//! with hills, ground, and one seeded atmosphere; trees cached as stamps, swayed by t.
use crate::_0_profile::measure_layer;
use crate::color::*;
use crate::fable_1_trees::{grow, hash01, Growth, Ink, Species, SPECIES};
use crate::opts::param_f32;
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
    pub fog: f32,
    pub horizon: f32,
    pub moon: f32,
    pub fruit: f32,
}

impl ForestKnobs {
    pub(crate) fn from_env() -> Self {
        ForestKnobs {
            density: param_f32("DENSITY", 1.0),
            layers: param_f32("LAYERS", 3.0),
            sway: param_f32("SWAY", 1.0),
            speed: param_f32("SPEED", 1.0),
            hue: param_f32("HUE", 0.0),
            atmos: param_f32("ATMOS", 0.0),
            fog: param_f32("FOG", 0.6),
            horizon: param_f32("HORIZON", 0.6),
            moon: param_f32("MOON", 1.0),
            fruit: param_f32("FRUIT", 0.2),
        }
    }
}

struct SCell {
    dx: i16,
    dy: i16,
    ch: char,
    ci: u8,
}

struct Tree {
    root_x: i32,
    root_y: i32,
    height: i32,
    layer: u8,
    cells: Vec<SCell>,
    colors: Vec<Color>,
    phase: f32,
    period_mul: f32,
    amp: f32,
}

struct Mote {
    x: f32,
    y: f32,
    rx: f32,
    ry: f32,
    p1: f32,
    p2: f32,
    speed: f32,
    blink: f32,
    kind: u8,
}

type Key = (usize, usize, u64, [u32; 7]);

struct Scene {
    key: Key,
    ground_y: usize,
    layer_count: usize,
    ridge: Vec<u16>,
    stars: Vec<(u16, u16, u8)>,
    moon: (i32, i32, i32),
    trees: Vec<Tree>,
    motes: Vec<Mote>,
    mist_bands: Vec<(i32, i32)>,
    atmos: u8,
    wind_period: f32,
    light_period: f32,
    sky_night: [Color; 2],
    sky_dusk: [Color; 2],
    ground_far: Color,
    ground_near: Color,
    ground_fg: Color,
    hill: Color,
    fog: Color,
    star: Color,
    moon_fg: Color,
    mote_fg: [Color; 2],
    row_bg: Vec<Color>,
    shifts: Vec<i32>,
    lit: Vec<Color>,
    col_a: Vec<f32>,
    col_b: Vec<f32>,
}

thread_local! {
    static CACHE: RefCell<Option<Scene>> = RefCell::new(None);
}

fn knob_key(k: &ForestKnobs) -> [u32; 7] {
    [
        k.density.to_bits(),
        k.layers.to_bits(),
        k.hue.to_bits(),
        k.atmos.to_bits(),
        k.fog.to_bits(),
        k.horizon.to_bits(),
        k.fruit.to_bits(),
    ]
}

fn color_index(colors: &mut Vec<Color>, c: Color) -> u8 {
    if let Some(i) = colors.iter().position(|&x| x == c) {
        return i as u8;
    }
    if colors.len() < 255 {
        colors.push(c);
        return (colors.len() - 1) as u8;
    }
    0
}

fn build(w: usize, h: usize, seed: u64, palette: &[Color; 5], k: &ForestKnobs) -> Scene {
    let mut rng = StdRng::seed_from_u64(seed ^ 0x00F0_4E57);
    let ground_y = ((h as f32 * k.horizon.clamp(0.3, 0.85)) as usize).clamp(2, h.saturating_sub(2).max(2));
    let hue_shift = k.hue as f64 + (rng.random::<f32>() * 40.0 - 20.0) as f64;
    let leaf_base = shift_hue(palette[1], hue_shift);
    let trunk_base = shift_hue(palette[2], hue_shift * 0.5);
    let accent = shift_hue(palette[3], hue_shift);
    let sky_night = [darken(palette[0], 6), lerp_color(palette[0], palette[2], 0.32)];
    let sky_dusk = [lerp_color(palette[0], palette[1], 0.22), lerp_color(palette[0], accent, 0.5)];
    let fog = lighten(sky_night[1], 14);
    let ground_far = lerp_color(palette[0], palette[2], 0.3);
    let ground_near = lerp_color(palette[0], palette[2], 0.1);
    let ground_fg = lerp_color(palette[2], leaf_base, 0.35);
    let hill = lerp_color(sky_night[1], palette[2], 0.3);
    let star = lighten(palette[4], 10);
    let moon_fg = lighten(palette[4], 20);

    let hill_h = ((ground_y as f32) * 0.2).max(1.0);
    let (f1, f2) = (TAU * (1.5 + rng.random::<f32>()) / w as f32, TAU * (4.0 + 3.0 * rng.random::<f32>()) / w as f32);
    let (p1, p2) = (rng.random::<f32>() * TAU, rng.random::<f32>() * TAU);
    let ridge: Vec<u16> = (0..w)
        .map(|x| {
            let xf = x as f32;
            let v = 0.5 + 0.5 * ((xf * f1 + p1).sin() * 0.65 + (xf * f2 + p2).sin() * 0.35);
            let y = ground_y as f32 - 1.0 - hill_h * (0.25 + 0.75 * v);
            y.max(0.0) as u16
        })
        .collect();
    let n_stars = (w * ground_y / 70).max(4);
    let stars: Vec<(u16, u16, u8)> = (0..n_stars)
        .map(|_| {
            let x = rng.random_range(0..w as u32) as u16;
            let y = rng.random_range(0..(ground_y.max(2) as u32 - 1)) as u16;
            let r = rng.random_range(0..12u32) as u8;
            (x, y, r)
        })
        .collect();
    let moon = (
        rng.random_range((w as u32 / 10)..(w as u32 * 9 / 10).max(w as u32 / 10 + 1)) as i32,
        rng.random_range(1..(ground_y as u32 / 2).max(2)) as i32,
        ((h / 22) as i32).max(1),
    );

    let mut weights = [0f32; 5];
    for wgt in weights.iter_mut() {
        *wgt = 0.25 + rng.random::<f32>();
    }
    weights[4] *= 0.4;
    let boost = rng.random_range(0..5u32) as usize;
    weights[boost] += if boost == 4 { 0.4 } else { 1.0 };
    let wsum: f32 = weights.iter().sum();
    let density_mul = 0.8 + 0.4 * rng.random::<f32>();
    let atmos = {
        let a = k.atmos.round() as i32;
        if (1..=5).contains(&a) {
            a as u8
        } else {
            1 + rng.random_range(0..5u32) as u8
        }
    };
    let wind_period = 30.0 + rng.random::<f32>() * 14.0;
    let layer_count = (k.layers.round() as i32).clamp(1, 6) as usize;
    let mut trees: Vec<Tree> = Vec::new();
    let mut mist_bands: Vec<(i32, i32)> = Vec::new();
    let ground_depth = (h - ground_y) as f32;
    for li in 0..layer_count {
        let lf = if layer_count > 1 { li as f32 / (layer_count - 1) as f32 } else { 1.0 };
        let th_base = (h as f32 * (0.14 + 0.42 * lf)).max(4.0);
        let root_lo = ground_y as f32 + ground_depth * lf * 0.8;
        let jit = ground_depth * 0.08;
        let tw_base = th_base * 1.1;
        let step = (tw_base * (1.1 + 0.6 * lf) / (k.density.max(0.05) * density_mul)).max(3.0);
        mist_bands.push((root_lo as i32 - (ground_depth * 0.22) as i32 - 2, root_lo as i32 + (ground_depth * 0.12) as i32 + 1));
        let fade = (1.0 - lf) * k.fog.clamp(0.0, 1.0);
        let mut x = -(tw_base * 0.3) + rng.random::<f32>() * step;
        while x < w as f32 + tw_base * 0.3 {
            let th = ((th_base * (0.78 + 0.44 * rng.random::<f32>())) as usize).max(4);
            let tw = ((th as f32 * (0.85 + 0.5 * rng.random::<f32>())) as usize).clamp(5, (w * 2).max(5));
            let root_y = ((root_lo + (rng.random::<f32>() * 2.0 - 1.0) * jit) as i32).clamp(ground_y as i32, h as i32 - 1);
            let pick = rng.random::<f32>() * wsum;
            let mut acc = 0.0;
            let mut sp = Species::Colonize;
            for (i, s) in SPECIES.iter().enumerate() {
                acc += weights[i];
                if pick <= acc {
                    sp = *s;
                    break;
                }
            }
            let leaf = shift_hue(leaf_base, (rng.random::<f32>() * 20.0 - 10.0) as f64);
            let ink = Ink::from_base(trunk_base, leaf, accent).faded(fog, fade);
            let growth = Growth { fruit: k.fruit, branch: 0.6 + rng.random::<f32>() * 0.6, leaf: 0.55 + 0.35 * lf + rng.random::<f32>() * 0.3, roots: 0.5 + rng.random::<f32>() * 0.5 };
            let mut local: Grid = vec![vec![Cell::blank(); tw]; th];
            grow(sp, &mut local, Rect { x: 0, y: 0, w: tw, h: th }, 0.82 + 0.18 * rng.random::<f32>(), &ink, &growth, &mut rng);
            let mut cells: Vec<SCell> = Vec::new();
            let mut colors: Vec<Color> = Vec::new();
            for (yy, row) in local.iter().enumerate() {
                for (xx, c) in row.iter().enumerate() {
                    if c.ch == ' ' {
                        continue;
                    }
                    let ci = color_index(&mut colors, c.fg);
                    cells.push(SCell { dx: (xx as i32 - tw as i32 / 2) as i16, dy: (yy as i32 - th as i32 + 1) as i16, ch: c.ch, ci });
                }
            }
            let amp = (0.4 + 0.8 * lf) * (th as f32 / 14.0).clamp(0.5, 6.0);
            trees.push(Tree {
                root_x: x.round() as i32,
                root_y,
                height: th as i32,
                layer: li as u8,
                cells,
                colors,
                phase: rng.random::<f32>() * 0.9 - 0.45,
                period_mul: 0.9 + rng.random::<f32>() * 0.2,
                amp,
            });
            x += step * (0.7 + 0.6 * rng.random::<f32>());
            if rng.random::<f32>() < 0.22 {
                x += step * (0.5 + rng.random::<f32>());
            }
        }
    }
    trees.sort_by(|a, b| a.layer.cmp(&b.layer).then(a.root_y.cmp(&b.root_y)));

    let area = w * h;
    let mut motes: Vec<Mote> = Vec::new();
    match atmos {
        1 => {
            let n = (area / 250).clamp(8, 800);
            for _ in 0..n {
                motes.push(Mote {
                    x: rng.random::<f32>() * w as f32,
                    y: ground_y as f32 * 0.55 + rng.random::<f32>() * (h as f32 * 0.92 - ground_y as f32 * 0.55),
                    rx: 2.0 + rng.random::<f32>() * 6.0,
                    ry: 0.5 + rng.random::<f32>() * 1.5,
                    p1: rng.random::<f32>() * TAU,
                    p2: rng.random::<f32>() * TAU,
                    speed: 18.0 + rng.random::<f32>() * 22.0,
                    blink: 1.5 + rng.random::<f32>() * 2.5,
                    kind: rng.random_range(0..3u32) as u8,
                });
            }
        }
        2 | 5 => {
            let n = if atmos == 2 { (area / 150).clamp(12, 2000) } else { (area / 110).clamp(16, 4000) };
            for _ in 0..n {
                let slow = atmos == 5;
                motes.push(Mote {
                    x: rng.random::<f32>() * w as f32,
                    y: rng.random::<f32>() * h as f32,
                    rx: if slow { 0.5 + rng.random::<f32>() * 1.5 } else { 1.0 + rng.random::<f32>() * 3.0 },
                    ry: 0.0,
                    p1: rng.random::<f32>() * TAU,
                    p2: if slow { 4.0 + rng.random::<f32>() * 4.0 } else { 3.0 + rng.random::<f32>() * 3.0 },
                    speed: if slow { 0.5 + rng.random::<f32>() * 0.7 } else { 0.9 + rng.random::<f32>() * 0.9 },
                    blink: 0.0,
                    kind: rng.random_range(0..4u32) as u8,
                });
            }
        }
        4 => {
            let n = (area / 120).clamp(60, 12000);
            for _ in 0..n {
                motes.push(Mote {
                    x: rng.random::<f32>() * w as f32,
                    y: rng.random::<f32>() * h as f32,
                    rx: 0.3,
                    ry: 0.0,
                    p1: 0.0,
                    p2: 0.0,
                    speed: 6.0 + rng.random::<f32>() * 4.0,
                    blink: 0.0,
                    kind: rng.random_range(0..3u32) as u8,
                });
            }
        }
        _ => {}
    }
    let mote_fg = match atmos {
        1 => [lighten(accent, 60), accent],
        2 => [leaf_base, accent],
        4 => [lerp_color(fog, palette[4], 0.4), fog],
        5 => [palette[4], lerp_color(fog, palette[4], 0.5)],
        _ => [fog, fog],
    };

    Scene {
        key: (w, h, seed, knob_key(k)),
        ground_y,
        layer_count,
        ridge,
        stars,
        moon,
        trees,
        motes,
        mist_bands,
        atmos,
        wind_period,
        light_period: 60.0,
        sky_night,
        sky_dusk,
        ground_far,
        ground_near,
        ground_fg,
        hill,
        fog,
        star,
        moon_fg,
        mote_fg,
        row_bg: vec![palette[0]; h],
        shifts: Vec::new(),
        lit: Vec::new(),
        col_a: vec![0.0; w],
        col_b: vec![0.0; w],
    }
}

pub(crate) fn draw_fable_1_forest(grid: &mut Grid, w: usize, h: usize, seed: u64, palette: &[Color; 5], t: f32, k: &ForestKnobs) {
    if w < 4 || h < 4 {
        return;
    }
    CACHE.with(|cell| {
        let mut slot = cell.borrow_mut();
        let key = (w, h, seed, knob_key(k));
        let stale = slot.as_ref().map(|c| c.key != key).unwrap_or(true);
        if stale {
            *slot = Some(build(w, h, seed, palette, k));
        }
        let s = slot.as_mut().unwrap();
        render(grid, w, h, seed, t, k, s);
    });
}

fn render(grid: &mut Grid, w: usize, h: usize, seed: u64, t: f32, k: &ForestKnobs, s: &mut Scene) {
    let speed = k.speed.max(0.0);
    let light = if t > 0.0 { 0.5 - 0.5 * (TAU * t * speed / s.light_period).cos() } else { 0.0 };
    let gy = s.ground_y;
    let top = lerp_color(s.sky_night[0], s.sky_dusk[0], light);
    let hor = lerp_color(s.sky_night[1], s.sky_dusk[1], light);
    let lift = (light * 22.0) as u8;
    for y in 0..h {
        s.row_bg[y] = if y < gy {
            let f = (y as f32 / gy as f32).powf(1.4);
            lerp_color(top, hor, f)
        } else {
            let f = (y - gy) as f32 / (h - gy).max(1) as f32;
            lighten(lerp_color(s.ground_far, s.ground_near, f), lift)
        };
    }
    let star_vis = (1.0 - light * 1.4).clamp(0.0, 1.0);
    measure_layer("fable-1-forest", "sky", || {
        for y in 0..gy {
            let bg = s.row_bg[y];
            let row = &mut grid[y];
            for cell in row.iter_mut().take(w) {
                *cell = Cell::with_bg(' ', bg, bg);
            }
        }
        if star_vis > 0.05 {
            for &(sx, sy, r) in &s.stars {
                let (x, y) = (sx as usize, sy as usize);
                if x >= w || y >= gy || (y as u16) >= s.ridge[x] {
                    continue;
                }
                let tw = if t > 0.0 { 0.75 + 0.25 * (t * (0.7 + r as f32 * 0.13) + r as f32).sin() } else { 1.0 };
                let ch = if r == 0 { '✦' } else if r < 3 { '+' } else { '·' };
                let fg = lerp_color(s.row_bg[y], s.star, star_vis * tw * if r < 3 { 1.0 } else { 0.7 });
                grid[y][x] = Cell::with_bg(ch, fg, s.row_bg[y]);
            }
        }
        if k.moon > 0.5 {
            let (mx, my, r) = s.moon;
            let drift = if t > 0.0 { (t * speed / s.light_period * w as f32 * 0.15) as i32 } else { 0 };
            let mx = mx + drift;
            let fg = lerp_color(s.moon_fg, hor, light * 0.6);
            for dy in -r..=r {
                for dx in -2 * r..=2 * r {
                    let d = (dx as f32 * 0.5).powi(2) + (dy as f32).powi(2);
                    if d > (r as f32 + 0.5).powi(2) {
                        continue;
                    }
                    let (x, y) = (mx + dx, my + dy);
                    if x < 0 || y < 0 || x as usize >= w || y as usize >= gy {
                        continue;
                    }
                    let ch = if d > (r as f32 - 0.6).powi(2) { '▒' } else { '▓' };
                    grid[y as usize][x as usize] = Cell::with_bg(ch, fg, s.row_bg[y as usize]);
                }
            }
        }
    });
    measure_layer("fable-1-forest", "hills", || {
        let hill = lighten(s.hill, lift);
        let ridge_fg = lerp_color(hill, s.fog, 0.5);
        for x in 0..w {
            let top = s.ridge[x] as usize;
            for y in top..gy {
                let ch = if y == top {
                    '░'
                } else if hash01(x as i32, y as i32, 11, seed) < 0.05 {
                    '·'
                } else {
                    ' '
                };
                grid[y][x] = Cell::with_bg(ch, ridge_fg, hill);
            }
        }
    });
    measure_layer("fable-1-forest", "ground", || {
        let fg_base = lighten(s.ground_fg, lift);
        for y in gy..h {
            let bg = s.row_bg[y];
            let f = (y - gy) as f32 / (h - gy).max(1) as f32;
            let p = 0.06 + 0.22 * f;
            let fg = lerp_color(bg, fg_base, 0.55 + 0.3 * f);
            let row = &mut grid[y];
            for (x, cell) in row.iter_mut().enumerate().take(w) {
                let u = hash01(x as i32, y as i32, 5, seed);
                let ch = if u < p * 0.5 {
                    '·'
                } else if u < p * 0.8 {
                    '∙'
                } else if u < p {
                    '~'
                } else {
                    ' '
                };
                *cell = Cell::with_bg(ch, fg, bg);
            }
        }
    });
    let wind_base = if t > 0.0 { TAU * t * speed / s.wind_period } else { 0.0 };
    let sway = k.sway.max(0.0);
    for li in 0..s.layer_count {
        measure_layer("fable-1-forest", "trees", || {
            let Scene { trees, shifts, lit, .. } = s;
            for tr in trees.iter().filter(|tr| tr.layer as usize == li) {
                lit.clear();
                lit.extend(tr.colors.iter().map(|&c| lighten(c, lift)));
                let wind = if t > 0.0 {
                    (wind_base / tr.period_mul + tr.phase).sin() + 0.35 * (wind_base * 2.7 + tr.phase * 2.0).sin()
                } else {
                    0.0
                };
                let amp = tr.amp * sway * wind;
                shifts.clear();
                for kk in 0..=tr.height as usize {
                    let f = kk as f32 / tr.height.max(1) as f32;
                    shifts.push((amp * f.powf(1.6)).round() as i32);
                }
                for c in &tr.cells {
                    let y = tr.root_y + c.dy as i32;
                    if y < 0 || y as usize >= h {
                        continue;
                    }
                    let x = tr.root_x + c.dx as i32 + shifts[(-c.dy) as usize];
                    if x < 0 || x as usize >= w {
                        continue;
                    }
                    let bg = grid[y as usize][x as usize].bg;
                    grid[y as usize][x as usize] = Cell::with_bg(c.ch, lit[c.ci as usize], bg);
                }
            }
        });
        if s.atmos == 3 {
            measure_layer("fable-1-forest", "mist", || paint_mist(grid, w, h, seed, t, speed, li, s));
        }
    }
    measure_layer("fable-1-forest", "atmos", || paint_motes(grid, w, h, t, speed, s));
}

fn paint_mist(grid: &mut Grid, w: usize, h: usize, seed: u64, t: f32, speed: f32, li: usize, s: &mut Scene) {
    let (y0, y1) = s.mist_bands[li];
    let y0 = y0.max(0) as usize;
    let y1 = (y1.max(0) as usize).min(h - 1);
    if y1 <= y0 {
        return;
    }
    let ph = if t > 0.0 { t * speed } else { 0.0 };
    let f1 = TAU * 3.0 / w as f32;
    let f2 = TAU * 7.3 / w as f32;
    for x in 0..w {
        let xf = x as f32;
        s.col_a[x] = (xf * f1 + ph * TAU / 40.0 + li as f32).sin();
        s.col_b[x] = (xf * f2 - ph * TAU / 27.0 + 1.7 + li as f32 * 0.5).sin();
    }
    let yc = (y0 + y1) as f32 * 0.5;
    let half = (y1 - y0) as f32 * 0.5 + 0.5;
    let fog = lerp_color(s.fog, s.star, 0.12);
    for y in y0..=y1 {
        let d = ((y as f32 - yc) / half).abs();
        let env = 1.0 - d * d;
        let row = &mut grid[y];
        for (x, cell) in row.iter_mut().enumerate().take(w) {
            let v = env * (0.55 + 0.45 * s.col_a[x] * s.col_b[x]) + 0.3 * (hash01(x as i32, y as i32, 21, seed) - 0.5);
            if v > 0.8 {
                cell.ch = '▒';
                cell.fg = fog;
            } else if v > 0.6 {
                cell.ch = '░';
                cell.fg = fog;
            }
        }
    }
}

fn paint_motes(grid: &mut Grid, w: usize, h: usize, t: f32, speed: f32, s: &Scene) {
    let tt = if t > 0.0 { t * speed } else { 0.0 };
    let put = |grid: &mut Grid, x: i32, y: i32, ch: char, fg: Color| {
        if x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h {
            let bg = grid[y as usize][x as usize].bg;
            grid[y as usize][x as usize] = Cell::with_bg(ch, fg, bg);
        }
    };
    match s.atmos {
        1 => {
            for m in &s.motes {
                let a = TAU * tt / m.speed;
                let x = m.x + m.rx * (a + m.p1).sin();
                let y = m.y + m.ry * (a * 1.7 + m.p2).sin();
                let cyc = (tt / m.blink + m.p1 / TAU).fract();
                if cyc < 0.45 {
                    let ch = if cyc < 0.3 { '•' } else { '·' };
                    let fg = if cyc < 0.3 { s.mote_fg[0] } else { s.mote_fg[1] };
                    put(grid, x.round() as i32, y.round() as i32, ch, fg);
                }
            }
        }
        2 | 5 => {
            let glyphs: [char; 4] = if s.atmos == 2 { ['∙', '◦', '~', '·'] } else { ['·', '•', '∙', '·'] };
            let span = h as f32 + 2.0;
            for m in &s.motes {
                let y = (m.y + tt * m.speed).rem_euclid(span) - 1.0;
                let x = m.x + m.rx * (TAU * tt / m.p2 + m.p1).sin() + tt * 0.15;
                let x = x.rem_euclid(w as f32);
                let gi = ((tt / 1.5 + m.p1).floor() as usize + m.kind as usize) % 4;
                let fg = if m.kind % 2 == 0 { s.mote_fg[0] } else { s.mote_fg[1] };
                put(grid, x.round() as i32, y.round() as i32, glyphs[gi], fg);
            }
        }
        4 => {
            let span = h as f32 + 3.0;
            for m in &s.motes {
                let y = (m.y + tt * m.speed).rem_euclid(span) - 2.0;
                let x = (m.x - (y - m.y) * m.rx).rem_euclid(w as f32);
                let (ch, fg) = match m.kind {
                    0 => ('│', s.mote_fg[1]),
                    1 => ('╎', s.mote_fg[1]),
                    _ => ('·', s.mote_fg[0]),
                };
                put(grid, x.round() as i32, y.round() as i32, ch, fg);
                if m.kind < 2 {
                    put(grid, x.round() as i32, y.round() as i32 - 1, '╎', s.mote_fg[1]);
                }
            }
        }
        _ => {}
    }
}

pub(crate) fn cli_fable_1_forest(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
    let _ = (rng, term_w, term_h, mode, theme_name);
    let mut k = ForestKnobs::from_env();
    let pos: Vec<f32> = args.iter().skip(4).filter_map(|a| a.parse().ok()).collect();
    let slots: [&mut f32; 10] = [
        &mut k.density,
        &mut k.layers,
        &mut k.sway,
        &mut k.speed,
        &mut k.hue,
        &mut k.atmos,
        &mut k.fog,
        &mut k.horizon,
        &mut k.moon,
        &mut k.fruit,
    ];
    for (slot, v) in slots.into_iter().zip(pos.iter()) {
        *slot = *v;
    }
    draw_fable_1_forest(&mut grid, width, height, seed, &palette, t_anim, &k);
    (grid, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(w: usize, h: usize, seed: u64, t: f32) -> String {
        let mut g = vec![vec![Cell::blank(); w]; h];
        let p = crate::color::make_palette(seed);
        let k = ForestKnobs::from_env();
        draw_fable_1_forest(&mut g, w, h, seed, &p, t, &k);
        g.iter()
            .map(|row| row.iter().map(|c| c.ch).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn snapshot_fable_1_forest_80x24() {
        insta::assert_snapshot!("fable_1_forest_80x24", run(80, 24, 42, 0.0));
    }

    #[test]
    fn deterministic_and_seed_sensitive() {
        assert_eq!(run(110, 36, 42, 0.0), run(110, 36, 42, 0.0));
        assert_ne!(run(110, 36, 42, 0.0), run(110, 36, 7, 0.0));
    }

    #[test]
    fn t_moves_the_scene() {
        assert_ne!(run(110, 36, 42, 0.0), run(110, 36, 42, 12.0));
    }

    #[test]
    fn every_atmosphere_renders() {
        for a in 1..=5 {
            let mut g = vec![vec![Cell::blank(); 90]; 30];
            let p = crate::color::make_palette(3);
            let mut k = ForestKnobs::from_env();
            k.atmos = a as f32;
            draw_fable_1_forest(&mut g, 90, 30, 3, &p, 7.0, &k);
        }
    }

    #[test]
    fn frame_cost() {
        let (w, h) = (200usize, 60usize);
        let mut g = vec![vec![Cell::blank(); w]; h];
        let p = crate::color::make_palette(42);
        let k = ForestKnobs::from_env();
        draw_fable_1_forest(&mut g, w, h, 42, &p, 0.0, &k);
        let mut worst = 0.0f64;
        let start = std::time::Instant::now();
        for f in 0..200 {
            let t0 = std::time::Instant::now();
            draw_fable_1_forest(&mut g, w, h, 42, &p, f as f32 * 0.05, &k);
            worst = worst.max(t0.elapsed().as_secs_f64() * 1000.0);
        }
        let avg = start.elapsed().as_secs_f64() * 1000.0 / 200.0;
        eprintln!("fable-1-forest frame_cost 200x60: avg {:.3} ms, worst {:.3} ms", avg, worst);
        if !cfg!(debug_assertions) {
            assert!(avg < 4.0, "avg frame {:.3} ms", avg);
        }
    }
}
