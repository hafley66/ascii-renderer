//! mahoraga -- Shibuya: Malevolent Shrine slashes fill the field, the
//! eight-handled wheel turns once per adaptation, the Fire Arrow lands.
use crate::color::*;
use crate::opts::param_f32;
use crate::pp::ease_in_out;
use crate::types::*;
use crossterm::style::Color;
use rand::RngExt;
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::f32::consts::{PI, TAU};

pub struct ShrineKnobs {
    pub slash: f32,  // 0..1 -- Dismantle density over the field
    pub radius: f32, // 0.1..0.45 -- wheel radius as a fraction of the short axis
    pub speed: f32,  // 0.1..3 -- adaptations per animation unit
    pub fuga: u8,    // 1..8 -- adaptations survived before the Fire Arrow
    pub haze: f32,   // 0..1 -- domain haze dots
    pub turns: f32,  // 0..8 -- static frame: adaptations already made
}

impl ShrineKnobs {
    pub fn from_env() -> Self {
        ShrineKnobs {
            slash: param_f32("SLASH", 0.6).clamp(0.0, 1.0),
            radius: param_f32("RADIUS", 0.3).clamp(0.1, 0.45),
            speed: param_f32("SPEED", 1.0).clamp(0.1, 3.0),
            fuga: param_f32("FUGA", 8.0).clamp(1.0, 8.0) as u8,
            haze: param_f32("HAZE", 0.5).clamp(0.0, 1.0),
            turns: param_f32("TURNS", 8.0).clamp(0.0, 8.0),
        }
    }
}

fn set(grid: &mut Grid, x: i32, y: i32, ch: char, fg: Color) {
    if x >= 0 && y >= 0 && (y as usize) < grid.len() && (x as usize) < grid[0].len() {
        grid[y as usize][x as usize] = Cell::new(ch, fg);
    }
}

fn side_rng(seed: u64, layer: u64, idx: u64) -> StdRng {
    StdRng::seed_from_u64(seed ^ layer.wrapping_mul(0x9E37_79B9) ^ idx.wrapping_mul(0xC2B2_AE35))
}

/// Cut glyph for a screen-space direction (2:1 cell aspect already applied).
fn cut_glyph(dx: f32, dy: f32) -> char {
    let ax = dx.abs();
    let ay = dy.abs();
    if ax > ay * 2.5 {
        '─'
    } else if ay > ax * 1.5 {
        '│'
    } else if (dx > 0.0) == (dy > 0.0) {
        '\\'
    } else {
        '/'
    }
}

/// Walk a line with the 2:1 aspect and hand every cell to `paint`.
fn line(grid: &mut Grid, x0: f32, y0: f32, x1: f32, y1: f32, mut paint: impl FnMut(&mut Grid, i32, i32, usize, usize)) {
    let dx = x1 - x0;
    let dy = y1 - y0;
    let n = dx.abs().max(dy.abs()).ceil().max(1.0) as usize;
    for i in 0..=n {
        let f = i as f32 / n as f32;
        paint(grid, (x0 + dx * f).round() as i32, (y0 + dy * f).round() as i32, i, n);
    }
}

/// Animation clock: t=0 shows `turns` adaptations; t>0 replays the fight on a loop.
fn progress(t: f32, knobs: &ShrineKnobs) -> f32 {
    if t <= 0.0 {
        knobs.turns
    } else {
        (t * knobs.speed).rem_euclid(knobs.fuga as f32 + 3.0)
    }
}

fn draw_haze(grid: &mut Grid, width: usize, height: usize, rng: &mut StdRng, t: f32, knobs: &ShrineKnobs, ink: Color) {
    let n = (width * height) as f32 * knobs.haze * 0.03;
    for i in 0..n as usize {
        let x = rng.random_range(0..width) as i32;
        let y = rng.random_range(0..height) as i32;
        let g = rng.random_range(0..3u32);
        let drift = if t > 0.0 { (t * 0.6 + i as f32 * 0.37).sin() * 2.0 } else { 0.0 };
        let ch = match g {
            0 => '.',
            1 => ':',
            _ => '·',
        };
        set(grid, x + drift as i32, y, ch, darken(ink, 40));
    }
}

fn draw_slashes(grid: &mut Grid, width: usize, height: usize, seed: u64, rng: &mut StdRng, t: f32, p: f32, knobs: &ShrineKnobs, ink: Color, edge: Color) {
    let count = ((width * height) as f32 / 32.0 * knobs.slash).round() as usize;
    let reveal = if t > 0.0 { 0.35 + 0.65 * (p / knobs.fuga as f32).min(1.0) } else { 1.0 };
    let w = width as f32;
    let h = height as f32;
    for i in 0..count {
        let cx = rng.random_range(0.0..w);
        let cy = rng.random_range(0.0..h);
        let a = rng.random_range(0.0..PI);
        let len = rng.random_range(6.0..(w * 0.45).max(8.0));
        let bright = rng.random::<f32>();
        if (i as f32) >= count as f32 * reveal {
            continue;
        }
        let phase = side_rng(seed, 1, i as u64).random::<f32>();
        if t > 0.0 && (p * 0.7 + phase).rem_euclid(1.0) > 0.85 {
            continue;
        }
        let dx = a.cos() * 2.0;
        let dy = a.sin();
        let ch = cut_glyph(dx, dy);
        let color = if bright > 0.7 { edge } else { ink };
        line(grid, cx - dx * len * 0.5, cy - dy * len * 0.5, cx + dx * len * 0.5, cy + dy * len * 0.5, |g, x, y, k, n| {
            let mid = 1.0 - ((k as f32 / n as f32) - 0.5).abs() * 2.0;
            let fg = if mid > 0.6 { lighten(color, 30) } else { color };
            set(g, x, y, ch, fg);
        });
    }
}

fn draw_wheel(grid: &mut Grid, cx: f32, cy: f32, r: f32, p: f32, t: f32, adaptations: usize, rim: Color, shell: Color) {
    let rx = r * 2.0;
    let ry = r;
    let frac = p.fract();
    let turn = if t > 0.0 && frac < 0.3 { ease_in_out(frac / 0.3) } else { 1.0 };
    let rot = (adaptations as f32 - 1.0 + turn).max(0.0) * (TAU / 8.0);

    for k in 0..adaptations {
        let sr = 1.2 + 0.12 * k as f32;
        let steps = (rx * sr * 1.6) as usize;
        let fg = lerp_color(darken(shell, 50), lighten(shell, 30), k as f32 / 7.0);
        for s in 0..steps {
            let a = (s as f32 + k as f32 * 0.5) / steps as f32 * TAU;
            let x = cx + a.cos() * rx * sr;
            let y = cy + a.sin() * ry * sr;
            let ch = if (s + k) % 4 == 0 { ':' } else { '·' };
            set(grid, x.round() as i32, y.round() as i32, ch, fg);
        }
    }

    let steps = (rx * 5.0) as usize;
    for s in 0..steps {
        let a = s as f32 / steps as f32 * TAU;
        let x = cx + a.cos() * rx;
        let y = cy + a.sin() * ry;
        let ch = cut_glyph(-a.sin() * 2.0, a.cos());
        set(grid, x.round() as i32, y.round() as i32, ch, rim);
    }

    for k in 0..8 {
        let a = rot + k as f32 * TAU / 8.0;
        let dx = a.cos() * 2.0;
        let dy = a.sin();
        let ch = cut_glyph(dx, dy);
        line(grid, cx, cy, cx + a.cos() * rx, cy + a.sin() * ry, |g, x, y, i, n| {
            if i == 0 {
                return;
            }
            if i == n {
                set(g, x, y, '◆', lighten(rim, 40));
            } else {
                set(g, x, y, ch, darken(rim, 20));
            }
        });
    }
}

fn draw_fuga(grid: &mut Grid, width: usize, height: usize, seed: u64, cx: f32, cy: f32, r: f32, t: f32, reach: f32) {
    if reach <= 0.0 {
        return;
    }
    let ox = width as f32 * 0.06;
    let oy = height as f32 * 0.12;
    let far_x = cx + (cx - ox) * 1.4;
    let far_y = cy + (cy - oy) * 1.4;
    let tx = ox + (far_x - ox) * reach;
    let ty = oy + (far_y - oy) * reach;
    let core = hsl_to_rgb(48.0, 1.0, 0.72);
    let mid = hsl_to_rgb(28.0, 1.0, 0.55);
    let outer = hsl_to_rgb(8.0, 0.95, 0.42);
    let shimmer = if t > 0.0 { (t * 9.0) as usize } else { 0 };
    for band in [-1i32, 0, 1] {
        let fg = if band == 0 { core } else { mid };
        line(grid, ox, oy + band as f32, tx, ty + band as f32, |g, x, y, i, _| {
            let ch = if band == 0 {
                '='
            } else if (i + shimmer) % 3 == 0 {
                '~'
            } else {
                '-'
            };
            set(g, x, y, ch, fg);
        });
    }
    for band in [-2i32, 2] {
        line(grid, ox, oy + band as f32, tx, ty + band as f32, |g, x, y, i, _| {
            if (i + shimmer) % 4 == 0 {
                set(g, x, y, '~', outer);
            }
        });
    }

    if reach < 1.0 {
        return;
    }
    let mut sparks = side_rng(seed, 3, 0);
    let n = (r * 12.0) as usize;
    for i in 0..n {
        let a = sparks.random_range(0.0..TAU);
        let d = sparks.random_range(1.2..2.6) * r;
        let spin = if t > 0.0 { t * 0.8 + i as f32 * 0.05 } else { 0.0 };
        let x = cx + (a + spin).cos() * d * 2.0;
        let y = cy + (a + spin).sin() * d;
        let ch = if i % 4 == 0 { '+' } else { '*' };
        let fg = if i % 3 == 0 { core } else { mid };
        set(grid, x.round() as i32, y.round() as i32, ch, fg);
    }
    let burst = (r * 2.0 * 4.0) as usize;
    for s in 0..burst {
        let a = s as f32 / burst as f32 * TAU;
        set(grid, (cx + a.cos() * r * 2.6).round() as i32, (cy + a.sin() * r * 1.3).round() as i32, '*', outer);
    }
}

pub fn draw_mahoraga(grid: &mut Grid, width: usize, height: usize, seed: u64, palette: &[Color; 5], rng: &mut StdRng, t: f32, knobs: &ShrineKnobs) {
    let p = progress(t, knobs);
    let adaptations = (p.floor() as usize).min(8);
    let reach = (p - knobs.fuga as f32 + 1.0).clamp(0.0, 1.0);
    let cx = width as f32 / 2.0 - 0.5;
    let cy = height as f32 / 2.0 - 0.5;
    let r = ((height as f32).min(width as f32 / 2.0) * knobs.radius).max(2.0);

    draw_haze(grid, width, height, rng, t, knobs, palette[3]);
    draw_slashes(grid, width, height, seed, rng, t, p, knobs, darken(palette[1], 10), lighten(palette[2], 20));
    draw_wheel(grid, cx, cy, r, p, t, adaptations, lighten(palette[3], 25), palette[0]);
    draw_fuga(grid, width, height, seed, cx, cy, r, t, reach);
    let hub = if reach >= 1.0 { hsl_to_rgb(48.0, 1.0, 0.8) } else { lighten(palette[4], 40) };
    set(grid, cx.round() as i32, cy.round() as i32, '◉', hub);
}

pub fn render_mahoraga_frame(width: usize, height: usize, seed: u64, palette: &[Color; 5], mut rng: StdRng, t: f32, knobs: &ShrineKnobs) -> Grid {
    let mut grid = vec![vec![Cell::blank(); width]; height];
    draw_mahoraga(&mut grid, width, height, seed, palette, &mut rng, t, knobs);
    grid
}

pub(crate) fn cli_mahoraga(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
    // mahoraga [turns] [slash] -- positional overrides win over env/defaults
    let mut knobs = ShrineKnobs::from_env();
    if let Some(v) = args.get(4).and_then(|v| v.parse::<f32>().ok()) {
        knobs.turns = v.clamp(0.0, 8.0);
    }
    if let Some(v) = args.get(5).and_then(|v| v.parse::<f32>().ok()) {
        knobs.slash = v.clamp(0.0, 1.0);
    }
    let _ = (term_w, term_h, mode, theme_name);
    draw_mahoraga(&mut grid, width, height, seed, &palette, &mut rng, t_anim, &knobs);
    (grid, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(w: usize, h: usize, seed: u64, t: f32, turns: f32) -> String {
        let p = crate::color::make_palette(seed);
        let mut knobs = ShrineKnobs::from_env();
        knobs.turns = turns;
        let g = render_mahoraga_frame(w, h, seed, &p, StdRng::seed_from_u64(seed), t, &knobs);
        g.iter()
            .map(|row| row.iter().map(|c| c.ch).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn snapshot_mahoraga_fuga() {
        insta::assert_snapshot!("mahoraga_80x24", run(80, 24, 42, 0.0, 8.0));
    }

    #[test]
    fn snapshot_mahoraga_three_turns() {
        insta::assert_snapshot!("mahoraga_80x24_t3", run(80, 24, 42, 0.0, 3.0));
    }

    #[test]
    fn deterministic_and_seed_sensitive() {
        assert_eq!(run(90, 30, 42, 0.0, 8.0), run(90, 30, 42, 0.0, 8.0));
        assert_ne!(run(90, 30, 42, 0.0, 8.0), run(90, 30, 7, 0.0, 8.0));
    }

    #[test]
    fn t_turns_the_wheel() {
        assert_ne!(run(90, 30, 42, 1.0, 8.0), run(90, 30, 42, 3.0, 8.0));
    }

    #[test]
    fn wheel_and_arrow_present() {
        let s = run(80, 24, 42, 0.0, 8.0);
        assert!(s.contains('◉'), "hub");
        assert!(s.contains('◆'), "handles");
        assert!(s.contains('='), "fire arrow core");
        assert!(!run(80, 24, 42, 0.0, 3.0).contains('='), "no arrow before fuga");
    }
}
