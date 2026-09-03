//! astrolabe -- a working brass instrument: graduated limb, engraved tympan,
//! precessing rete of stars, sweeping rule. Rolls are t-independent; motion is rotation.
use crate::_0_profile::measure_layer;
use crate::color::*;
use crate::opts::param_f32;
use crate::sprites::{MoveDir, TreePen};
use crate::types::*;
use crossterm::style::Color;
use rand::RngExt;
use rand::SeedableRng;
use rand::rngs::StdRng;

fn set(grid: &mut Grid, x: i32, y: i32, ch: char, fg: Color) {
    if x >= 0 && y >= 0 && (y as usize) < grid.len() && (x as usize) < grid[0].len() {
        grid[y as usize][x as usize] = Cell::new(ch, fg);
    }
}

fn blank_at(grid: &Grid, x: i32, y: i32) -> bool {
    x >= 0
        && y >= 0
        && (y as usize) < grid.len()
        && (x as usize) < grid[0].len()
        && grid[y as usize][x as usize].ch == ' '
}

fn pt(cx: f32, cy: f32, rx: f32, ry: f32, a: f32) -> (i32, i32) {
    ((cx + a.cos() * rx) as i32, (cy + a.sin() * ry) as i32)
}

fn step_toward(pen: &mut TreePen, grid: &mut Grid, tx: i32, ty: i32) {
    while pen.x != tx || pen.y != ty {
        let dx = tx - pen.x;
        let dy = ty - pen.y;
        let dir = match (dx.signum(), dy.signum()) {
            (0, 1) => MoveDir::Down,
            (0, -1) => MoveDir::Up,
            (1, 0) => MoveDir::Right,
            (-1, 0) => MoveDir::Left,
            (1, 1) => MoveDir::DownRight,
            (1, -1) => MoveDir::UpRight,
            (-1, 1) => MoveDir::DownLeft,
            (-1, -1) => MoveDir::UpLeft,
            _ => break,
        };
        pen.step(grid, dir);
    }
}

/// Ellipse arc from angle a0 to a1 (radians, CCW math convention), drawn as
/// connected box-drawing glyphs. Returns the pen at the end point.
fn arc(grid: &mut Grid, cx: f32, cy: f32, rx: f32, ry: f32, a0: f32, a1: f32, color: Color, start: Option<(i32, i32)>) -> TreePen {
    let steps = (((a1 - a0).abs() * rx.max(ry)).ceil() as usize).clamp(8, 400);
    let mut pen = if let Some((sx, sy)) = start {
        let mut p = TreePen::new(sx, sy, color);
        p.last_dir = Some(MoveDir::Right);
        p
    } else {
        let (sx, sy) = pt(cx, cy, rx, ry, a0);
        TreePen::new(sx, sy, color)
    };
    for i in 1..=steps {
        let a = a0 + (a1 - a0) * i as f32 / steps as f32;
        let (tx, ty) = pt(cx, cy, rx, ry, a);
        step_toward(&mut pen, grid, tx, ty);
    }
    pen
}

fn ellipse(grid: &mut Grid, cx: f32, cy: f32, rx: f32, ry: f32, color: Color) {
    let mut pen = arc(grid, cx, cy, rx, ry, -std::f32::consts::PI, std::f32::consts::PI, color, None);
    let (sx, sy) = pt(cx, cy, rx, ry, -std::f32::consts::PI);
    step_toward(&mut pen, grid, sx, sy);
}

// ── Parts ───────────────────────────────────────────────────────────

struct Brass {
    limb: Color,
    limb_hi: Color,
    tympan: Color,
    rete: Color,
    ecliptic: Color,
    star: Color,
    rule: Color,
    ink: Color,
}

fn brass(palette: &[Color; 5], seed: u64) -> Brass {
    let mut rng = StdRng::seed_from_u64(seed ^ 0xA57A);
    let base_hue = if let Color::Rgb { r, g, .. } = palette[3] {
        (r as f64 * 1.7 + g as f64 * 0.9) % 360.0
    } else {
        42.0
    };
    let hue = (base_hue * 0.25 + 40.0 + rng.random_range(-8..8) as f64).rem_euclid(360.0);
    Brass {
        limb: hsl_to_rgb(hue, 0.62, 0.30),
        limb_hi: hsl_to_rgb(hue, 0.70, 0.44),
        tympan: hsl_to_rgb(hue, 0.34, 0.20),
        rete: hsl_to_rgb(hue, 0.72, 0.46),
        ecliptic: hsl_to_rgb((hue + 28.0).rem_euclid(360.0), 0.65, 0.40),
        star: hsl_to_rgb((hue + 14.0).rem_euclid(360.0), 0.55, 0.62),
        rule: hsl_to_rgb((hue - 18.0).rem_euclid(360.0), 0.60, 0.50),
        ink: hsl_to_rgb(hue, 0.30, 0.14),
    }
}

/// Outer limb: double ring, degree ticks, hour numerals, inscription band.
fn draw_limb(grid: &mut Grid, cx: f32, cy: f32, rx: f32, ry: f32, b: &Brass, rng: &mut StdRng) {
    ellipse(grid, cx, cy, rx, ry, b.limb);
    ellipse(grid, cx, cy, rx * 0.94, ry * 0.94, b.limb_hi);
    ellipse(grid, cx, cy, rx * 0.70, ry * 0.70, b.limb);
    // inscription band: dense alternating marks between the outer rings
    let band = ['·', '˙', '⋅', '·'];
    for i in 0..96 {
        let a = i as f32 / 96.0 * std::f32::consts::TAU;
        let (x0, y0) = pt(cx, cy, rx * 0.97, ry * 0.97, a);
        let g = band[rng.random_range(0..band.len() as u32) as usize];
        if blank_at(grid, x0, y0) {
            set(grid, x0, y0, g, darken(b.limb, 10));
        }
    }
    // degree ticks on the inner limb ring: fine + coarse
    for i in 0..72 {
        let a = i as f32 / 72.0 * std::f32::consts::TAU;
        let coarse = i % 6 == 0;
        let (x1, y1) = pt(cx, cy, rx * 0.94, ry * 0.94, a);
        let (x2, y2) = pt(cx, cy, rx * (if coarse { 0.86 } else { 0.90 }), ry * (if coarse { 0.86 } else { 0.90 }), a);
        let ch = if coarse { '┼' } else { '·' };
        let c = if coarse { b.limb_hi } else { b.limb };
        let mut pen = TreePen::new(x1, y1, c);
        step_toward(&mut pen, grid, x2, y2);
        set(grid, x2, y2, ch, c);
    }
    // hour numerals inside the limb
    let hours = ["XII", "I", "II", "III", "IIII", "V", "VI", "VII", "VIII", "IX", "X", "XI"];
    for (i, h) in hours.iter().enumerate() {
        let a = -std::f32::consts::FRAC_PI_2 + i as f32 / 12.0 * std::f32::consts::TAU;
        let (hx, hy) = pt(cx, cy, rx * 0.79, ry * 0.79, a);
        let dir_right = a.cos() >= 0.0;
        let start = if dir_right { hx } else { hx - h.chars().count() as i32 + 1 };
        for (k, ch) in h.chars().enumerate() {
            set(grid, start + k as i32, hy, ch, lighten(b.limb_hi, 10));
        }
    }
}

/// Tympan: stereographic plate -- almucantar rings, azimuth spokes, tropics.
fn draw_tympan(grid: &mut Grid, cx: f32, cy: f32, rx: f32, ry: f32, b: &Brass, rings: usize, spokes: usize, rng: &mut StdRng) {
    let inner_r = rx * 0.70 * 0.92;
    let inner_ry = ry * 0.70 * 0.92;
    // almucantars: concentric altitude rings, engraved fine
    for i in 1..=rings {
        let f = i as f32 / (rings + 1) as f32;
        let c = if i % 2 == 0 { b.tympan } else { darken(b.tympan, 8) };
        ellipse(grid, cx, cy, inner_r * f, inner_ry * f, c);
        // altitude marker at the east point
        let (mx, my) = pt(cx, cy, inner_r * f, inner_ry * f, 0.0);
        set(grid, mx, my, '·', darken(b.tympan, 5));
    }
    // azimuth spokes from center to the inner limb
    for i in 0..spokes {
        let a = i as f32 / spokes as f32 * std::f32::consts::TAU;
        let (tx, ty) = pt(cx, cy, inner_r, inner_ry, a);
        let mut pen = TreePen::new(cx as i32, cy as i32, b.tympan);
        step_toward(&mut pen, grid, tx, ty);
    }
    // horizon line, heavier
    let mut pen = TreePen::new((cx - inner_r) as i32, cy as i32, lighten(b.tympan, 14));
    step_toward(&mut pen, grid, (cx + inner_r) as i32, cy as i32);
    // tropic bands: two dashed ellipses
    for f in [0.36, 0.58] {
        let steps = 64;
        for i in 0..steps {
            if i % 3 == 2 {
                continue;
            }
            let a = i as f32 / steps as f32 * std::f32::consts::TAU;
            let (x, y) = pt(cx, cy, inner_r * f, inner_ry * f, a);
            if blank_at(grid, x, y) {
                set(grid, x, y, '·', darken(b.tympan, 4));
            }
        }
    }
    // zenith mark
    set(grid, cx as i32, (cy - inner_ry * 0.5) as i32, '❉', darken(b.tympan, 0));
    let _ = rng;
}

/// Rete: rotating star cage -- ecliptic band, star pointers, labels. All
/// geometry is a pure rotation by `rot` of seeded placements.
fn draw_rete(grid: &mut Grid, cx: f32, cy: f32, rx: f32, ry: f32, b: &Brass, rot: f32, stars: usize, twinkle: f32, t: f32, ecliptic_amt: f32) {
    let mut srng = StdRng::seed_from_u64((stars as u64).wrapping_mul(0x51A5) ^ 0x9E37);
    let inner_r = rx * 0.66;
    let inner_ry = ry * 0.66;
    let glyphs = ['✦', '✳', '✶', '❉', '◉'];
    for _ in 0..stars {
        let a = srng.random::<f32>() * std::f32::consts::TAU + rot;
        let rr = srng.random::<f32>();
        let f = 0.15 + rr * rr * 0.95; // bias stars outward
        let (x, y) = pt(cx, cy, inner_r * f, inner_ry * f, a);
        let gi = srng.random_range(0..glyphs.len() as u32) as usize;
        let phase = srng.random::<f32>() * std::f32::consts::TAU;
        let tw = (t * 2.2 + phase).sin();
        let g = if tw > 1.0 - twinkle * 0.3 {
            '*'
        } else {
            glyphs[gi]
        };
        let c = if tw > 0.0 { lighten(b.star, 10) } else { b.star };
        if blank_at(grid, x, y) {
            set(grid, x, y, g, c);
        }
        // pointer tick leaning toward the limb
        let (px, py) = pt(cx, cy, inner_r * f * 1.08 + 0.5, inner_ry * f * 1.08 + 0.5, a);
        if blank_at(grid, px, py) && srng.random::<f32>() < 0.7 {
            set(grid, px, py, '·', darken(b.rete, 5));
        }
    }
    // ecliptic: off-center band carrying twelve division ticks
    if ecliptic_amt > 0.05 {
        let ea = rot * 0.6 + 0.7;
        let ecx = cx + ea.cos() * inner_r * 0.22 * ecliptic_amt;
        let ecy = cy + ea.sin() * inner_ry * 0.22 * ecliptic_amt;
        let er = inner_r * 0.42 * ecliptic_amt;
        let ery = inner_ry * 0.42 * ecliptic_amt;
        ellipse(grid, ecx, ecy, er, ery, b.ecliptic);
        for i in 0..12 {
            let a = i as f32 / 12.0 * std::f32::consts::TAU + rot * 0.25;
            let (x1, y1) = pt(ecx, ecy, er, ery, a);
            let (x2, y2) = pt(ecx, ecy, er * 0.9, ery * 0.9, a);
            set(grid, x2, y2, if i % 3 == 0 { '◆' } else { '·' }, b.ecliptic);
            let _ = (x1, y1);
        }
    }
}

/// Rule: a graduated bar pivoting through the center, plus the center pin.
fn draw_rule(grid: &mut Grid, cx: f32, cy: f32, rx: f32, ry: f32, b: &Brass, phi: f32) {
    for sign in [1.0f32, -1.0] {
        let a = phi * sign;
        let (tx, ty) = pt(cx, cy, rx * 0.68, ry * 0.68, a);
        let mut pen = TreePen::new(cx as i32, cy as i32, b.rule);
        let mut k = 0;
        while pen.x != tx || pen.y != ty {
            let dx = tx - pen.x;
            let dy = ty - pen.y;
            let dir = if dx.abs() >= dy.abs() {
                if dx > 0 { MoveDir::Right } else { MoveDir::Left }
            } else if dy > 0 {
                MoveDir::Down
            } else {
                MoveDir::Up
            };
            pen.step(grid, dir);
            k += 1;
            if k % 4 == 0 {
                let nx = pen.x + if dir == MoveDir::Up || dir == MoveDir::Down { 1 } else { 0 };
                let ny = pen.y + if dir == MoveDir::Left || dir == MoveDir::Right { 1 } else { 0 };
                if blank_at(grid, nx, ny) {
                    set(grid, nx, ny, '·', darken(b.rule, 8));
                }
            }
        }
        set(grid, tx, ty, '◆', b.rule);
    }
    set(grid, cx as i32, cy as i32, '◉', lighten(b.rule, 18));
}

/// Throne crest above the limb.
fn draw_throne(grid: &mut Grid, cx: f32, cy: f32, ry: f32, b: &Brass) {
    let ty = (cy - ry - 2.0) as i32;
    if ty < 0 {
        return;
    }
    let tx = cx as i32;
    set(grid, tx, ty, '❖', b.limb_hi);
    set(grid, tx - 1, ty, '╮', b.limb);
    set(grid, tx - 2, ty, '╭', b.limb);
    set(grid, tx + 1, ty, '╭', b.limb);
    set(grid, tx + 2, ty, '╮', b.limb);
    set(grid, tx - 1, ty + 1, '╰', b.limb);
    set(grid, tx + 1, ty + 1, '╯', b.limb);
    set(grid, tx, ty + 1, '│', b.limb);
}

// ── Renderer ────────────────────────────────────────────────────────

pub fn draw_astrolabe(grid: &mut Grid, width: usize, height: usize, seed: u64, palette: &[Color; 5], t: f32) {
    let stars = param_f32("STARS", 42.0).clamp(10.0, 110.0) as usize;
    let rings = param_f32("RINGS", 5.0).clamp(2.0, 9.0) as usize;
    let spokes = param_f32("SPOKES", 8.0).clamp(4.0, 16.0) as usize;
    let rate = param_f32("RATE", 0.08).clamp(0.0, 0.5);
    let rule_rate = param_f32("RULEV", 0.05).clamp(0.0, 0.5);
    let twinkle = param_f32("TWINK", 0.6).clamp(0.0, 1.0);
    let ecliptic_amt = param_f32("ZOD", 1.0).clamp(0.0, 1.0);

    let b = brass(palette, seed);
    let cy = height as f32 / 2.0 - 0.5;
    let cx = width as f32 / 2.0 - 0.5;
    let ry = (height as f32 / 2.0 - 2.5).min(width as f32 / 4.0 - 1.0).max(3.0);
    let rx = ry * 2.0;

    // plate rng: every roll happens unconditionally, so no frame re-rolls
    let mut plate_rng = StdRng::seed_from_u64(seed ^ 0x71AB);
    let small = rx < 12.0;
    let rings = if small { (rings / 2).max(2) } else { rings };
    let spokes = if small { 6 } else { spokes };
    let stars = if small { stars / 2 } else { stars };

    measure_layer("astrolabe", "tympan", || draw_tympan(grid, cx, cy, rx, ry, &b, rings, spokes, &mut plate_rng));
    measure_layer("astrolabe", "rete", || draw_rete(grid, cx, cy, rx, ry, &b, t * rate, stars, twinkle, t, ecliptic_amt));
    measure_layer("astrolabe", "rule", || draw_rule(grid, cx, cy, rx, ry, &b, t * rule_rate + 0.6));
    measure_layer("astrolabe", "limb", || draw_limb(grid, cx, cy, rx, ry, &b, &mut plate_rng));
    measure_layer("astrolabe", "throne", || draw_throne(grid, cx, cy, ry, &b));
}

pub(crate) fn cli_astrolabe(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
    let _ = (rng, term_w, term_h, args, mode, theme_name);
    draw_astrolabe(&mut grid, width, height, seed, &palette, t_anim);
    (grid, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(w: usize, h: usize, seed: u64, t: f32) -> String {
        let mut g = vec![vec![Cell::blank(); w]; h];
        let p = crate::color::make_palette(seed);
        draw_astrolabe(&mut g, w, h, seed, &p, t);
        g.iter()
            .map(|row| row.iter().map(|c| c.ch).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn snapshot_astrolabe_small() {
        insta::assert_snapshot!("astrolabe_80x24", run(80, 24, 42, 0.0));
    }

    #[test]
    fn snapshot_astrolabe_large() {
        insta::assert_snapshot!("astrolabe_110x40", run(110, 40, 42, 0.0));
    }

    #[test]
    fn deterministic_and_seed_sensitive() {
        assert_eq!(run(90, 30, 42, 0.0), run(90, 30, 42, 0.0));
        assert_ne!(run(90, 30, 42, 0.0), run(90, 30, 7, 0.0));
    }

    #[test]
    fn t_moves_the_instrument() {
        assert_ne!(run(90, 30, 42, 0.0), run(90, 30, 42, 6.0));
        assert_ne!(run(90, 30, 42, 6.0), run(90, 30, 42, 12.0));
    }

    #[test]
    fn frames_locally_stable() {
        let a = run(90, 30, 42, 30.0);
        let b = run(90, 30, 42, 30.6);
        let mut changed = 0;
        let ra: Vec<char> = a.chars().collect();
        let rb: Vec<char> = b.chars().collect();
        for (x, y) in ra.iter().zip(rb.iter()) {
            if x != y {
                changed += 1;
            }
        }
        assert!(changed > 0, "motion expected");
        let frac = changed as f64 / ra.len() as f64;
        assert!(frac < 0.10, "{} of {} chars changed in 10 ticks", changed, ra.len());
    }
}
