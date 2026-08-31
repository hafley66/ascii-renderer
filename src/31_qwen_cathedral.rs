use crossterm::style::Color;
use rand::rngs::StdRng;
use rand::RngExt;

use crate::color::{darken, lerp_color, lighten, shift_hue};
use crate::opts::param_f32;
use crate::pp::{pp_fbm, pp_line};
use crate::types::{Cell, Grid};

const TAU: f32 = std::f32::consts::TAU;

#[derive(Clone, Copy, Debug)]
pub(crate) struct CathedralParams {
    pub(crate) bays: usize,
    pub(crate) towers: usize,
    pub(crate) rose: usize,
    pub(crate) candles: usize,
    pub(crate) speed: f32,
    pub(crate) rays: f32,
    pub(crate) smoke: usize,
    pub(crate) depth: usize,
    pub(crate) mosaic: f32,
    pub(crate) glow: f32,
    pub(crate) arch: f32,
    pub(crate) banners: usize,
}

impl Default for CathedralParams {
    fn default() -> Self {
        Self {
            bays: 5,
            towers: 2,
            rose: 12,
            candles: 14,
            speed: 0.8,
            rays: 0.6,
            smoke: 8,
            depth: 3,
            mosaic: 0.55,
            glow: 0.7,
            arch: 0.62,
            banners: 4,
        }
    }
}

impl CathedralParams {
    pub(crate) fn from_args(args: &[String]) -> Self {
        let read = |index: usize, key: &str, default: f32| {
            args.get(index)
                .and_then(|value| value.parse::<f32>().ok())
                .unwrap_or_else(|| param_f32(key, default))
        };
        Self {
            bays: read(4, "NAVES", 5.0).round().clamp(3.0, 9.0) as usize,
            towers: read(5, "TOWERS", 2.0).round().clamp(0.0, 4.0) as usize,
            rose: read(6, "ROSE", 12.0).round().clamp(6.0, 18.0) as usize,
            candles: read(7, "CANDLES", 14.0).round().clamp(0.0, 40.0) as usize,
            speed: read(8, "SPEED", 0.8).clamp(0.05, 3.0),
            rays: read(9, "RAY", 0.6).clamp(0.0, 1.0),
            smoke: read(10, "SMOKE", 8.0).round().clamp(0.0, 24.0) as usize,
            depth: read(11, "DEPTH", 3.0).round().clamp(1.0, 5.0) as usize,
            mosaic: read(12, "MOSAIC", 0.55).clamp(0.0, 1.0),
            glow: read(13, "GLOW", 0.7).clamp(0.0, 1.5),
            arch: read(14, "ARCH", 0.62).clamp(0.0, 1.0),
            banners: read(15, "BANNERS", 4.0).round().clamp(0.0, 8.0) as usize,
        }
    }
}

fn clampi(v: i32, lo: i32, hi: i32) -> i32 {
    v.max(lo).min(hi.max(lo))
}

fn hash01(seed: u64, tag: u64) -> f32 {
    let mut value = seed ^ tag.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    value ^= value >> 30;
    value = value.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^= value >> 31;
    ((value >> 40) as f32) / ((1u64 << 24) - 1) as f32
}

fn in_bounds(grid: &Grid, x: i32, y: i32) -> bool {
    x >= 0 && y >= 0 && (y as usize) < grid.len() && (x as usize) < grid[y as usize].len()
}

fn put(grid: &mut Grid, x: i32, y: i32, ch: char, fg: Color) {
    if in_bounds(grid, x, y) {
        let bg = grid[y as usize][x as usize].bg;
        grid[y as usize][x as usize] = Cell::with_bg(ch, fg, bg);
    }
}

fn put_bg(grid: &mut Grid, x: i32, y: i32, ch: char, fg: Color, bg: Color) {
    if in_bounds(grid, x, y) {
        grid[y as usize][x as usize] = Cell::with_bg(ch, fg, bg);
    }
}

fn line(grid: &mut Grid, a: (i32, i32), b: (i32, i32), color: Color) {
    pp_line(grid, a.0, a.1, b.0, b.1, color);
}

fn quad(p0: (f32, f32), c: (f32, f32), p1: (f32, f32), u: f32) -> (f32, f32) {
    let v = 1.0 - u;
    (
        v * v * p0.0 + 2.0 * v * u * c.0 + u * u * p1.0,
        v * v * p0.1 + 2.0 * v * u * c.1 + u * u * p1.1,
    )
}

fn trace<F>(grid: &mut Grid, samples: usize, color: Color, mut point: F)
where
    F: FnMut(f32) -> (f32, f32),
{
    let mut previous: Option<(i32, i32)> = None;
    for sample in 0..=samples {
        let u = sample as f32 / samples.max(1) as f32;
        let p = point(u);
        let here = (p.0.round() as i32, p.1.round() as i32);
        if let Some(prior) = previous {
            if prior != here {
                line(grid, prior, here, color);
            }
        }
        previous = Some(here);
    }
}

fn draw_arch_half(grid: &mut Grid, spring: (f32, f32), apex: (f32, f32), point: f32, color: Color) {
    let dx = apex.0 - spring.0;
    let rise = spring.1 - apex.1;
    let c = (
        spring.0 + dx * (0.22 + 0.30 * (1.0 - point)),
        spring.1 - rise * (0.62 + 0.28 * point),
    );
    trace(grid, 16, color, |u| quad(spring, c, apex, u));
}

fn pointed_arch(
    grid: &mut Grid,
    lx: f32,
    rx: f32,
    base_y: f32,
    apex_y: f32,
    point: f32,
    color: Color,
) {
    let apex = ((lx + rx) * 0.5, apex_y);
    draw_arch_half(grid, (lx, base_y), apex, point, color);
    draw_arch_half(grid, (rx, base_y), apex, point, color);
}

fn ellipse_point(cx: i32, cy: i32, rx: f32, ry: f32, a: f32) -> (f32, f32) {
    (cx as f32 + a.cos() * rx, cy as f32 + a.sin() * ry)
}

#[derive(Clone, Copy)]
struct Plan {
    roof_y: i32,
    floor_y: i32,
    spring_y: i32,
    arch_h: i32,
    cx: i32,
    rose_cy: i32,
    rose_rx: f32,
    rose_ry: f32,
    portal_apex: i32,
    wall_l: i32,
    wall_r: i32,
}

fn compute_plan(width: usize, height: usize) -> Plan {
    let w = width as i32;
    let h = height as i32;
    let roof_y = clampi((h as f32 * 0.26).round() as i32, 2, h - 6);
    let floor_y = clampi((h as f32 * 0.80).round() as i32, roof_y + 4, h - 2);
    let span = floor_y - roof_y;
    let spring_y = roof_y + span * 15 / 22;
    let arch_h = (span * 5 / 22).clamp(1, 8);
    let rose_cy = roof_y + span * 5 / 22;
    let rose_rx = ((w as f32) * 0.16).min(span as f32 * 0.42).clamp(2.5, 16.0);
    let rose_ry = (span as f32 * 0.17).clamp(1.4, 8.0);
    let portal_apex = clampi(
        (roof_y + span * 12 / 22).max(rose_cy + rose_ry.ceil() as i32 + 2),
        1,
        floor_y - 2,
    );
    Plan {
        roof_y,
        floor_y,
        spring_y,
        arch_h,
        cx: w / 2,
        rose_cy,
        rose_rx,
        rose_ry,
        portal_apex,
        wall_l: 3,
        wall_r: w - 4,
    }
}

fn draw_sky(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    glow: f32,
) {
    let top = darken(shift_hue(palette[0], 348.0), 22);
    let horizon = darken(palette[2], 56);
    let span = height.saturating_sub(1).max(1) as f32;
    for y in 0..height {
        let bg = lerp_color(top, horizon, (y as f32 / span).powf(1.4) * 0.9);
        for x in 0..width {
            grid[y][x] = Cell::with_bg(' ', bg, bg);
        }
    }
    let star = lighten(palette[4], 6);
    for y in 0..height {
        for x in 0..width {
            let pick = hash01(seed, x as u64 * 131 + y as u64 * 977);
            let field = pp_fbm(x as f32 * 0.085, y as f32 * 0.14, seed ^ 0xC47);
            if field > 0.68 && pick > 0.80 - glow * 0.05 {
                let tw = (t * 0.9 + pick * TAU * 2.0).sin();
                let ch = if tw > 0.82 {
                    '✦'
                } else if tw > 0.1 {
                    '∙'
                } else {
                    '·'
                };
                put(
                    grid,
                    x as i32,
                    y as i32,
                    ch,
                    if tw > 0.6 { star } else { darken(star, 30) },
                );
            }
        }
    }
    let moon_x = (width as f32 * 0.80).round() as i32;
    let moon_y = 3i32;
    put(grid, moon_x, moon_y, '☾', lighten(palette[4], 26));
    for (dx, dy) in [(-2, 0), (2, 0), (0, -1), (0, 1), (-1, 1), (1, -1)] {
        put(grid, moon_x + dx, moon_y + dy, '·', darken(palette[4], 34));
    }
}

fn draw_towers(
    grid: &mut Grid,
    plan: &Plan,
    width: usize,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    params: &CathedralParams,
) {
    if params.towers == 0 || width < 20 {
        return;
    }
    let fractions = [0.13f32, 0.87, 0.30, 0.70];
    let stone = darken(palette[1], 44);
    let stone_hi = darken(palette[1], 30);
    let half = clampi(width as i32 / 26, 1, 3);
    for i in 0..params.towers.min(4) {
        let cx = (width as f32 * fractions[i]).round() as i32;
        let top = 2 + (hash01(seed, 300 + i as u64) * 2.0).round() as i32;
        let spire_h = clampi((plan.roof_y - top) / 2, 2, 4);
        let body_top = top + spire_h;
        for y in body_top..=plan.roof_y + 1 {
            for x in cx - half..=cx + half {
                let edge = x == cx - half || x == cx + half;
                put(
                    grid,
                    x,
                    y,
                    if edge { '▓' } else { '█' },
                    if edge { stone } else { darken(stone, 6) },
                );
            }
        }
        let mut y = body_top + 1;
        while y < plan.roof_y {
            let pulse = (t * params.speed * 1.3 + i as f32 * 1.7 + y as f32).sin();
            let lit = pulse > 0.35 - params.glow * 0.2;
            put(
                grid,
                cx,
                y,
                '▪',
                if lit {
                    lighten(palette[3], 22)
                } else {
                    darken(palette[3], 26)
                },
            );
            y += 3;
        }
        for s in 0..spire_h {
            let y = top + s;
            let spread = (s * half) / spire_h.max(1);
            for x in cx - spread..=cx + spread {
                let ch = if x < cx {
                    '╱'
                } else if x > cx {
                    '╲'
                } else {
                    '▲'
                };
                put(grid, x, y, ch, stone_hi);
            }
        }
        put(grid, cx, top - 1, '✚', lighten(palette[3], 18));
        for x in (cx - half..=cx + half).step_by(2) {
            put(grid, x, body_top - 1, '▔', stone_hi);
        }
    }
}

fn draw_wall(
    grid: &mut Grid,
    plan: &Plan,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
) {
    let stone_top = darken(palette[1], 40);
    let stone_bot = darken(shift_hue(palette[2], 14.0), 52);
    let bottom = plan.floor_y.min(height as i32 - 1);
    for y in plan.roof_y..=bottom {
        let f = (y - plan.roof_y) as f32 / (bottom - plan.roof_y).max(1) as f32;
        let bg = lerp_color(stone_top, stone_bot, f * 0.85);
        for x in plan.wall_l..=plan.wall_r {
            put_bg(grid, x, y, ' ', bg, bg);
        }
    }
    for y in plan.roof_y..=bottom {
        for x in plan.wall_l..=plan.wall_r {
            let pick = hash01(seed, x as u64 * 57 + y as u64 * 31);
            if (y - plan.roof_y).rem_euclid(3) == 0 && x.rem_euclid(4) == 1 {
                put(grid, x, y, '·', darken(stone_top, 8));
            } else if pick > 0.94 {
                put(grid, x, y, '░', darken(stone_bot, 4));
            }
        }
    }
    let _ = width;
    for x in plan.wall_l..=plan.wall_r {
        let ch = if (x + plan.roof_y).rem_euclid(6) == 0 {
            '◆'
        } else {
            '▔'
        };
        put(grid, x, plan.roof_y, ch, darken(palette[1], 26));
    }
    let gx = plan.cx;
    for k in 0..3i32 {
        put(
            grid,
            gx - k - 1,
            plan.roof_y - k - 1,
            '╱',
            darken(palette[1], 24),
        );
        put(
            grid,
            gx + k + 1,
            plan.roof_y - k - 1,
            '╲',
            darken(palette[1], 24),
        );
    }
    put(grid, gx, plan.roof_y - 4, '✚', lighten(palette[3], 22));
}

fn draw_buttresses(
    grid: &mut Grid,
    plan: &Plan,
    width: usize,
    height: usize,
    palette: &[Color; 5],
) {
    let stone = darken(palette[1], 38);
    let bottom = (plan.floor_y + 1).min(height as i32 - 2);
    for &(dir, x0, x1) in &[(1i32, 1i32, 2i32), (-1, width as i32 - 3, width as i32 - 2)] {
        for y in plan.roof_y - 1..=bottom {
            put(grid, x0, y, '▓', stone);
            put(grid, x1, y, '█', darken(stone, 8));
        }
        put(grid, x0, plan.roof_y - 2, '▲', stone);
        put(grid, x0, plan.roof_y - 3, '·', darken(palette[3], 12));
        for k in 1..=4i32 {
            let glyph = if dir > 0 { '╲' } else { '╱' };
            put(
                grid,
                x1 + dir * k,
                plan.roof_y - 2 + (k * 3 + 2) / 4,
                glyph,
                stone,
            );
        }
    }
}

fn draw_rose(
    grid: &mut Grid,
    plan: &Plan,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    params: &CathedralParams,
) {
    let cx = plan.cx;
    let cy = plan.rose_cy;
    let (rx, ry) = (plan.rose_rx, plan.rose_ry);
    let rot = hash01(seed, 77) * TAU + t * params.speed * 0.14;
    let stone = darken(palette[1], 24);
    let gold = lighten(palette[3], 22);

    let rxi = rx.ceil() as i32;
    let ryi = ry.ceil() as i32;
    for dy in -ryi..=ryi {
        for dx in -rxi..=rxi {
            let nx = dx as f32 / rx;
            let ny = dy as f32 / ry;
            if nx * nx + ny * ny > 0.62 {
                continue;
            }
            let pick = hash01(seed, (dx + 64) as u64 * 17 + (dy + 64) as u64 * 89);
            if pick > 0.42 {
                let angle = (dy as f32).atan2(dx as f32).to_degrees();
                let pulse = (t * params.speed * 0.8 + pick * TAU).sin();
                let base = shift_hue(
                    palette[1 + (pick * 3.0) as usize % 3],
                    angle as f64 + t as f64 * 8.0,
                );
                let col = if pulse > 0.4 {
                    lighten(base, 8 + (params.glow * 12.0) as u8)
                } else {
                    darken(base, 18)
                };
                put(grid, cx + dx, cy + dy, '░', col);
            }
        }
    }

    let samples = 96;
    for i in 0..=samples {
        let a = i as f32 / samples as f32 * TAU;
        let (x, y) = ellipse_point(cx, cy, rx + 1.1, ry + 0.65, a);
        put(grid, x.round() as i32, y.round() as i32, '▓', stone);
    }
    for s in 0..params.rose {
        let a = s as f32 * TAU / params.rose as f32;
        let (x, y) = ellipse_point(cx, cy, rx + 1.1, ry + 0.65, a);
        put(grid, x.round() as i32, y.round() as i32, '◆', gold);
    }
    for i in 0..=64 {
        let a = i as f32 / 64.0 * TAU;
        let (x, y) = ellipse_point(cx, cy, rx * 0.55, ry * 0.55, a);
        put(grid, x.round() as i32, y.round() as i32, '·', stone);
    }
    for s in 0..params.rose {
        let a = rot + s as f32 * TAU / params.rose as f32;
        let (x0, y0) = ellipse_point(cx, cy, rx * 0.30, ry * 0.30, a);
        let (x1, y1) = ellipse_point(cx, cy, rx * 0.90, ry * 0.90, a);
        line(
            grid,
            (x0.round() as i32, y0.round() as i32),
            (x1.round() as i32, y1.round() as i32),
            darken(stone, 2),
        );
        let pulse = (t * params.speed * 1.1 + s as f32 * 0.9).sin();
        let jewel = if pulse > 0.45 { '✦' } else { '◇' };
        let jcol = if pulse > 0.45 {
            lighten(gold, (params.glow * 16.0) as u8)
        } else {
            darken(gold, 16)
        };
        put(grid, x1.round() as i32, y1.round() as i32, jewel, jcol);
        let am = a + TAU / (params.rose as f32 * 2.0);
        let (mx, my) = ellipse_point(cx, cy, rx * 0.68, ry * 0.68, am);
        put(
            grid,
            mx.round() as i32,
            my.round() as i32,
            '∙',
            darken(gold, 10),
        );
    }
    let bp = (t * params.speed * 0.9).sin();
    put(
        grid,
        cx,
        cy,
        '◉',
        if bp > 0.0 { lighten(gold, 14) } else { gold },
    );
    put(grid, cx - 1, cy, '⊙', darken(gold, 8));
    put(grid, cx + 1, cy, '⊙', darken(gold, 8));
}

fn draw_lancets(
    grid: &mut Grid,
    plan: &Plan,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    params: &CathedralParams,
) {
    let top_y = plan.roof_y + 2;
    let base_y = plan.spring_y - plan.arch_h - 2;
    if base_y - top_y < 2 {
        return;
    }
    let stone = darken(palette[1], 30);
    let per_side = (params.bays / 2).clamp(1, 3);
    let margin = plan.rose_rx.ceil() as i32 + 3;
    for side in [-1i32, 1] {
        let inner = plan.cx + side * margin;
        let outer = if side < 0 {
            plan.wall_l + 2
        } else {
            plan.wall_r - 2
        };
        for k in 1..=per_side as i32 {
            let x = outer + (inner - outer) * k / (per_side + 1) as i32;
            let phase = hash01(seed, 900 + x as u64 * 13 + (side + 2) as u64);
            for y in top_y..=base_y {
                put(grid, x - 1, y, '│', stone);
                put(grid, x + 1, y, '│', stone);
                let pulse = (t * params.speed * 0.9 + phase * TAU + y as f32 * 0.4).sin();
                let gcol = shift_hue(palette[3], phase as f64 * 140.0 + y as f64 * 6.0);
                let gcol = if pulse > 0.25 {
                    lighten(gcol, 20)
                } else {
                    darken(gcol, 16)
                };
                put(grid, x, y, if pulse > 0.25 { '▯' } else { '▮' }, gcol);
            }
            put(grid, x - 1, top_y - 1, '╱', stone);
            put(grid, x + 1, top_y - 1, '╲', stone);
            let point = (params.arch * 2.0).round() as i32;
            for p in 0..point {
                put(
                    grid,
                    x,
                    top_y - 1 - p,
                    if p == point - 1 { '▲' } else { '│' },
                    stone,
                );
            }
        }
    }
}

fn draw_banners(
    grid: &mut Grid,
    plan: &Plan,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    params: &CathedralParams,
) {
    if params.banners == 0 {
        return;
    }
    let rod_y = plan.spring_y - 1;
    let max_len = plan.floor_y - plan.spring_y - 2;
    if max_len < 2 {
        return;
    }
    let len = clampi(max_len, 2, 5);
    let inner_l = plan.wall_l + 3;
    let inner_r = plan.wall_r - 3;
    let span = inner_r - inner_l;
    if span < 4 {
        return;
    }
    let mut xs: Vec<i32> = Vec::new();
    for b in 0..params.bays as i32 {
        let x = inner_l + span * (2 * b + 1) / (2 * params.bays.max(1) as i32);
        if (x - plan.cx).abs() >= 7 {
            xs.push(x);
        }
    }
    if xs.is_empty() {
        return;
    }
    let cloth = ['▨', '▩', '▧', '◆'];
    for i in 0..params.banners.min(xs.len() * 2) {
        let x = xs[i % xs.len()] + if i >= xs.len() { 2 } else { 0 };
        let phase = hash01(seed, 1500 + i as u64 * 37);
        let base = shift_hue(palette[1 + i % 3], (phase * 220.0) as f64);
        for rx in x - 1..=x + 1 {
            put(grid, rx, rod_y, '─', darken(palette[1], 20));
        }
        for k in 1..=len {
            let y = rod_y + k;
            let sway = (t * params.speed * 0.7 + phase * TAU + k as f32 * 0.35).sin();
            let xoff = if k == len {
                sway.round() as i32
            } else {
                (sway * 0.4).round() as i32
            };
            let g = cloth[((phase * 9.0) as usize + k as usize + i) % cloth.len()];
            let col = if k == len { darken(base, 8) } else { base };
            put(grid, x + xoff, y, g, col);
        }
        let sway = (t * params.speed * 0.7 + phase * TAU + (len + 1) as f32 * 0.35).sin();
        put(
            grid,
            x + sway.round() as i32,
            rod_y + len + 1,
            '▽',
            darken(base, 14),
        );
    }
}

fn draw_arcade(grid: &mut Grid, plan: &Plan, palette: &[Color; 5], params: &CathedralParams) {
    let stone = darken(palette[1], 32);
    let inner_l = plan.wall_l + 3;
    let inner_r = plan.wall_r - 3;
    let span = inner_r - inner_l;
    if span < 4 {
        return;
    }
    let bays = params.bays.max(1) as i32;
    let portal_half = clampi((plan.wall_r - plan.wall_l) / 9, 3, 6);
    let mut cols: Vec<i32> = Vec::new();
    for i in 0..=bays {
        cols.push(inner_l + span * i / bays);
    }
    for &x in &cols {
        put(grid, x, plan.spring_y, '▣', stone);
        for y in plan.spring_y + 1..plan.floor_y - 1 {
            put(grid, x, y, '║', darken(stone, 4));
        }
        put(grid, x, plan.floor_y - 1, '■', darken(stone, 10));
    }
    for b in 0..bays as usize {
        let lx = cols[b] as f32;
        let rx = cols[b + 1] as f32;
        let mid = ((lx + rx) * 0.5).round() as i32;
        if (mid - plan.cx).abs() <= portal_half + 1 {
            continue;
        }
        pointed_arch(
            grid,
            lx + 1.0,
            rx - 1.0,
            plan.spring_y as f32,
            (plan.spring_y - plan.arch_h) as f32,
            params.arch,
            stone,
        );
        for d in 1..=params.depth.min(2) as i32 {
            pointed_arch(
                grid,
                lx + 1.0 - d as f32,
                rx - 1.0 + d as f32,
                plan.spring_y as f32,
                (plan.spring_y - plan.arch_h - d) as f32,
                params.arch,
                darken(stone, (d * 6) as u8),
            );
        }
    }
}

fn draw_floor(
    grid: &mut Grid,
    plan: &Plan,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    params: &CathedralParams,
) {
    let top = plan.floor_y;
    let bottom = height as i32 - 2;
    if bottom <= top {
        return;
    }
    let rows = (bottom - top).max(1) as f32;
    let dark1 = darken(palette[2], 58);
    let dark2 = darken(shift_hue(palette[1], 18.0), 64);
    let carpet = darken(palette[2], 34);
    for y in top..=bottom {
        let f = (y - top) as f32 / rows;
        let tile = 1.6 + f * 3.0;
        for x in 1..width as i32 - 1 {
            let u = (x - plan.cx) as f32 / tile;
            let v = (y - top) as f32 / (1.0 + f * 1.5);
            let parity = (u.floor() as i32 + v.floor() as i32).rem_euclid(2);
            let mut bg = if parity == 0 { dark1 } else { dark2 };
            if u.abs() < 1.2 {
                bg = carpet;
            }
            put_bg(grid, x, y, ' ', bg, bg);
            let pick = hash01(seed, x as u64 * 71 + y as u64 * 13);
            if pick < params.mosaic * 0.4 && u.abs() > 1.4 {
                let glyph = ['◇', '◆', '·', '▪'][(pick * 40.0) as usize % 4];
                put(
                    grid,
                    x,
                    y,
                    glyph,
                    darken(palette[3], 20 + (pick * 30.0) as u8),
                );
            } else if u.abs() <= 1.2 && pick < 0.30 {
                put(grid, x, y, '·', darken(carpet, 8));
            }
        }
    }
    let ly = top + 2;
    if ly < bottom && width >= 40 {
        let lrx = clampi(width as i32 / 12, 4, 9) as f32;
        for i in 0..=48 {
            let a = i as f32 / 48.0 * TAU;
            let (px, py) = ellipse_point(plan.cx, ly, lrx, 1.6, a);
            put(
                grid,
                px.round() as i32,
                py.round() as i32,
                '·',
                darken(palette[4], 22),
            );
        }
        put(grid, plan.cx, ly, '◉', darken(palette[4], 12));
    }
}

fn draw_portal(
    grid: &mut Grid,
    plan: &Plan,
    palette: &[Color; 5],
    t: f32,
    params: &CathedralParams,
) {
    let stone = darken(palette[1], 28);
    let half = clampi((plan.wall_r - plan.wall_l) / 9, 3, 6);
    let base = plan.floor_y - 1;
    let apex = plan.portal_apex;
    if base <= apex {
        return;
    }
    let gold = lighten(palette[3], 18);
    let cx = plan.cx;
    let orders = params.depth + 2;
    for d in (0..orders).rev() {
        let di = d as i32;
        pointed_arch(
            grid,
            (cx - half - di) as f32,
            (cx + half + di) as f32,
            base as f32,
            apex as f32,
            params.arch,
            darken(stone, (d * 5) as u8),
        );
    }
    for y in apex + 2..=base {
        put(grid, cx - half, y, '║', stone);
        put(grid, cx + half, y, '║', stone);
    }
    let door_top = apex + (base - apex) * 2 / 5;
    let seam_pulse = (t * params.speed * 1.4).sin();
    for y in door_top..=base - 1 {
        for x in cx - half + 1..=cx + half - 1 {
            let panel = (x + y).rem_euclid(3);
            let ch = if x == cx {
                '│'
            } else if panel == 0 {
                '▮'
            } else {
                '▯'
            };
            let col = if x == cx {
                if seam_pulse > 0.2 {
                    lighten(gold, 10)
                } else {
                    gold
                }
            } else {
                darken(shift_hue(palette[2], 8.0), 44)
            };
            put(grid, x, y, ch, col);
        }
    }
    for y in door_top - 2..door_top {
        for x in cx - half + 2..=cx + half - 2 {
            let pulse = (t * params.speed * 0.8 + (x + y) as f32 * 0.3).sin();
            put(
                grid,
                x,
                y,
                if pulse > 0.3 { '✶' } else { '▒' },
                if pulse > 0.3 {
                    lighten(gold, 6)
                } else {
                    darken(gold, 14)
                },
            );
        }
    }
    for s in 0..2i32 {
        let y = base + 1 + s;
        let w2 = half + 2 + s * 2;
        for x in cx - w2..=cx + w2 {
            put(
                grid,
                x,
                y,
                if s == 0 { '▔' } else { '─' },
                darken(stone, (6 + s * 8) as u8),
            );
        }
    }
}

fn draw_candles(
    grid: &mut Grid,
    plan: &Plan,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    params: &CathedralParams,
) {
    let wax = lighten(palette[4], 42);
    let gold = lighten(palette[3], 26);
    let half = clampi((plan.wall_r - plan.wall_l) / 9, 3, 6);
    for side in [-1i32, 1] {
        let x = plan.cx + side * (half + 2);
        let y = plan.floor_y - 1;
        for k in 0..=2i32 {
            put(grid, x, y - k, '│', wax);
        }
        let flick = (t * params.speed * 3.1 + side as f32).sin();
        put(
            grid,
            x,
            y - 3,
            if flick > 0.0 { '✦' } else { '*' },
            lighten(gold, 8),
        );
    }
    let band = height as i32 - 2 - plan.floor_y;
    if band < 2 {
        return;
    }
    let row_y = plan.floor_y + band * 2 / 3;
    for i in 0..params.candles {
        let h1 = hash01(seed, 6100 + i as u64 * 17);
        let h2 = hash01(seed, 6200 + i as u64 * 29);
        let x = 5 + (h1 * width.saturating_sub(11) as f32).round() as i32;
        let y = row_y + if h2 > 0.72 { 1 } else { 0 };
        if y >= height as i32 - 1 {
            continue;
        }
        let hgt = 1 + if h2 > 0.5 { 1 } else { 0 };
        for k in 0..=hgt {
            put(grid, x, y - k, '│', wax);
        }
        let flame_y = y - hgt - 1;
        let flick = (t * params.speed * (2.2 + h1 * 2.6) + h2 * TAU).sin();
        let (glyph, col) = if flick > 0.62 - params.glow * 0.25 {
            ('✦', lighten(gold, (params.glow * 12.0) as u8))
        } else if flick > 0.05 {
            ('*', gold)
        } else if flick > -0.5 {
            ('∙', darken(palette[3], 6))
        } else {
            ('·', darken(palette[3], 26))
        };
        put(grid, x, flame_y, glyph, col);
        if params.glow > 0.55 && flick > 0.45 {
            put(grid, x - 1, flame_y, '·', darken(gold, 22));
            put(grid, x + 1, flame_y, '·', darken(gold, 22));
        }
    }
}

fn draw_smoke(
    grid: &mut Grid,
    plan: &Plan,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    params: &CathedralParams,
) {
    if params.smoke == 0 {
        return;
    }
    let rise = clampi(plan.spring_y - plan.portal_apex + 3, 3, 8) as f32;
    let y0 = (plan.portal_apex + 2) as f32;
    for i in 0..params.smoke {
        let period = 4.0 + (i % 5) as f32 * 0.9;
        let phase = hash01(seed, 5200 + i as u64 * 7);
        let prog = ((t * params.speed * 0.5 + phase * period) % period) / period;
        let y = y0 - prog * rise;
        let x = plan.cx as f32
            + (prog * TAU * (1.0 + (i % 2) as f32) + phase * TAU).sin() * (0.7 + prog * 2.4)
            + ((i % 3) as f32 - 1.0) * prog * 1.7;
        let (glyph, fade) = if prog < 0.3 {
            ('∙', 8u8)
        } else if prog < 0.62 {
            ('~', 20)
        } else if prog < 0.85 {
            ('∿', 32)
        } else {
            ('·', 46)
        };
        put(
            grid,
            x.round() as i32,
            y.round() as i32,
            glyph,
            darken(palette[4], fade),
        );
    }
}

fn draw_shafts(
    grid: &mut Grid,
    plan: &Plan,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    params: &CathedralParams,
) {
    if params.rays <= 0.03 {
        return;
    }
    let gold = lighten(shift_hue(palette[3], 354.0), 26);
    let n = (2.0 + params.rays * 6.0).round() as usize;
    let max_d = (plan.floor_y - plan.rose_cy + 5).max(4) as f32;
    for i in 0..n {
        let fr = if n > 1 {
            i as f32 / (n - 1) as f32 - 0.5
        } else {
            0.0
        };
        let ang = std::f32::consts::FRAC_PI_2 + fr * 1.8 + (t * params.speed * 0.06).sin() * 0.05;
        let steps = (max_d * 2.0) as usize;
        for step in 2..steps {
            let d = step as f32 * 0.5;
            let x = (plan.cx as f32 + ang.cos() * d * 1.8).round() as i32;
            let y = (plan.rose_cy as f32 + ang.sin() * d * 0.85).round() as i32;
            if y < plan.roof_y - 1 || y > plan.floor_y + 3 {
                continue;
            }
            if d < plan.rose_ry + 1.5 {
                continue;
            }
            if hash01(seed, i as u64 * 997 + step as u64) > 0.42 + params.glow * 0.2 {
                continue;
            }
            let glyph = if step % 5 == 0 { '░' } else { '·' };
            put(
                grid,
                x,
                y,
                glyph,
                darken(gold, 8 + (d / max_d * 34.0) as u8),
            );
        }
    }
}

fn draw_dust(
    grid: &mut Grid,
    plan: &Plan,
    width: usize,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    params: &CathedralParams,
) {
    let n = (params.rays * 26.0 + params.glow * 14.0) as usize;
    let gold = lighten(palette[3], 18);
    let span_x = (plan.wall_r - plan.wall_l - 4).max(1);
    let span_y = (plan.floor_y - plan.roof_y - 4).max(1);
    let _ = width;
    for i in 0..n {
        let h1 = hash01(seed, 8100 + i as u64 * 13);
        let h2 = hash01(seed, 8200 + i as u64 * 23);
        let h3 = hash01(seed, 8300 + i as u64 * 41);
        let x0 = plan.wall_l + 2 + (h1 * span_x as f32).round() as i32;
        let y0 = plan.roof_y + 2 + (h2 * span_y as f32).round() as i32;
        let x = x0 + ((t * params.speed * (0.2 + h3 * 0.4) + h1 * TAU).sin() * 1.6).round() as i32;
        let y = y0 + ((t * params.speed * 0.3 + h2 * TAU).sin() * 0.9).round() as i32;
        if h3 > 0.55 {
            put(grid, x, y, '·', darken(gold, 26 + (h3 * 20.0) as u8));
        }
    }
}

fn draw_frame(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
) {
    if width < 10 || height < 8 {
        return;
    }
    let edge = darken(lighten(palette[1], 20), 12);
    let stud = lighten(palette[3], 12);
    let phase = (t * 1.5 + seed as f32 * 0.011).floor() as usize;
    for x in 1..width as i32 - 1 {
        let pattern = (x as usize + phase) % 8;
        let ch = match pattern {
            0 => '◆',
            4 => '◇',
            _ => '═',
        };
        put(grid, x, 0, ch, if pattern % 4 == 0 { stud } else { edge });
        let pattern = (x as usize + phase + 3) % 8;
        let ch = match pattern {
            0 => '◇',
            4 => '◆',
            _ => '═',
        };
        put(
            grid,
            x,
            height as i32 - 1,
            ch,
            if pattern % 4 == 0 { stud } else { edge },
        );
    }
    for y in 1..height as i32 - 1 {
        let ch = if (y as usize * 2 + phase) % 9 == 0 {
            '○'
        } else {
            '║'
        };
        put(grid, 0, y, ch, edge);
        put(grid, width as i32 - 1, y, ch, edge);
    }
    put(grid, 0, 0, '╔', stud);
    put(grid, width as i32 - 1, 0, '╗', stud);
    put(grid, 0, height as i32 - 1, '╚', stud);
    put(grid, width as i32 - 1, height as i32 - 1, '╝', stud);
}

pub(crate) fn draw_qwen_cathedral(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    rng: &mut StdRng,
    t: f32,
    params: &CathedralParams,
) {
    if width == 0 || height == 0 {
        return;
    }
    let t = t + rng.random_range(0.0..TAU) * 0.02;
    let plan = compute_plan(width, height);

    draw_sky(grid, width, height, seed, palette, t, params.glow);
    draw_towers(grid, &plan, width, seed, palette, t, params);
    draw_wall(grid, &plan, width, height, seed, palette);
    draw_buttresses(grid, &plan, width, height, palette);
    draw_rose(grid, &plan, seed, palette, t, params);
    draw_lancets(grid, &plan, seed, palette, t, params);
    draw_arcade(grid, &plan, palette, params);
    draw_banners(grid, &plan, seed, palette, t, params);
    draw_floor(grid, &plan, width, height, seed, palette, params);
    draw_portal(grid, &plan, palette, t, params);
    draw_candles(grid, &plan, width, height, seed, palette, t, params);
    draw_smoke(grid, &plan, seed, palette, t, params);
    draw_shafts(grid, &plan, seed, palette, t, params);
    draw_dust(grid, &plan, width, seed, palette, t, params);
    draw_frame(grid, width, height, seed, palette, t);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::make_palette;
    use crate::render::grid_to_plain;
    use rand::SeedableRng;

    fn frame(width: usize, height: usize, seed: u64, t: f32, params: &CathedralParams) -> Grid {
        let mut grid = vec![vec![Cell::blank(); width]; height];
        let palette = make_palette(seed);
        let mut rng = StdRng::seed_from_u64(seed);
        draw_qwen_cathedral(
            &mut grid, width, height, seed, &palette, &mut rng, t, params,
        );
        grid
    }

    fn plain(grid: &Grid) -> String {
        grid_to_plain(grid).join("\n")
    }

    #[test]
    fn deterministic_seeded_frame_and_visible_motion() {
        let params = CathedralParams::default();
        let a = plain(&frame(80, 36, 42, 1.25, &params));
        let b = plain(&frame(80, 36, 42, 1.25, &params));
        let c = plain(&frame(80, 36, 42, 3.75, &params));
        let different_seed = plain(&frame(80, 36, 43, 1.25, &params));
        let tuned = CathedralParams {
            bays: 8,
            towers: 4,
            rose: 16,
            candles: 30,
            rays: 1.0,
            smoke: 16,
            depth: 5,
            mosaic: 1.0,
            glow: 1.2,
            arch: 1.0,
            banners: 8,
            ..params
        };
        let different_inputs = plain(&frame(80, 36, 42, 1.25, &tuned));
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_ne!(a, different_seed);
        assert_ne!(a, different_inputs);
        assert_eq!(
            a.lines()
                .map(str::chars)
                .map(Iterator::count)
                .collect::<Vec<_>>(),
            vec![80; 36]
        );
    }

    #[test]
    fn tiny_grid_and_extreme_inputs_terminate() {
        let params = CathedralParams {
            bays: 9,
            towers: 4,
            rose: 18,
            candles: 40,
            speed: 3.0,
            rays: 1.0,
            smoke: 24,
            depth: 5,
            mosaic: 1.0,
            glow: 1.5,
            arch: 1.0,
            banners: 8,
        };
        for (w, h) in [(12usize, 6usize), (3, 3), (1, 1), (40, 5)] {
            let output = frame(w, h, 7, 8.0, &params);
            assert_eq!(output.len(), h);
            assert_eq!(output.iter().map(Vec::len).collect::<Vec<_>>(), vec![w; h]);
        }
    }

    #[test]
    fn dimensions_shape_the_frame() {
        let params = CathedralParams::default();
        for (w, h) in [(60usize, 24usize), (80, 45), (100, 30)] {
            let output = frame(w, h, 11, 0.0, &params);
            assert_eq!(output.len(), h);
            assert_eq!(
                plain(&output)
                    .lines()
                    .map(str::chars)
                    .map(Iterator::count)
                    .collect::<Vec<_>>(),
                vec![w; h]
            );
        }
        let wide = plain(&frame(100, 30, 11, 0.0, &params));
        let narrow = plain(&frame(60, 24, 11, 0.0, &params));
        assert_ne!(wide, narrow);
    }

    #[test]
    fn params_from_args_override_and_clamp() {
        let args: Vec<String> = [
            "ascii-renderer",
            "42",
            "qwen-cathedral",
            "ember",
            "9",
            "4",
            "18",
            "40",
            "3",
            "1",
            "24",
            "5",
            "1",
            "1.5",
            "1",
            "8",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let p = CathedralParams::from_args(&args);
        assert_eq!(p.bays, 9);
        assert_eq!(p.towers, 4);
        assert_eq!(p.rose, 18);
        assert_eq!(p.candles, 40);
        assert!((p.speed - 3.0).abs() < 1e-6);
        assert_eq!(p.depth, 5);
        assert_eq!(p.banners, 8);
        let clamped = CathedralParams::from_args(&[
            "bin".to_string(),
            "1".to_string(),
            "qwen-cathedral".to_string(),
            "ember".to_string(),
            "999".to_string(),
            "-5".to_string(),
        ]);
        assert_eq!(clamped.bays, 9);
        assert_eq!(clamped.towers, 0);
        let defaults = CathedralParams::from_args(&[]);
        assert_eq!(defaults.bays, 5);
        assert_eq!(defaults.depth, 3);
        assert!((defaults.arch - 0.62).abs() < 1e-6);
    }

    #[test]
    fn snapshot_qwen_cathedral_in_motion() {
        let params = CathedralParams::default();
        insta::assert_snapshot!(
            "qwen_cathedral_t2_75",
            plain(&frame(80, 36, 42, 2.75, &params))
        );
    }
}
