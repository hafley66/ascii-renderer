//! sonnet-1-spirograph -- a rolling circle traces a closed hypotrochoid or
//! epitrochoid track forever, pen and generating circles both drawn live.
use crate::_0_profile::measure_layer;
use crate::color::*;
use crate::opts::param_f32;
use crate::types::*;
use crossterm::style::Color;
use rand::RngExt;
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::cell::RefCell;
use std::f32::consts::TAU;

const BASE_CYCLE: f32 = 36.0;
const ECHO_SAMPLES: usize = 1400;
const TRAIL_SAMPLES: usize = 700;
const BLOOM_SETS: [[char; 3]; 3] = [['.', '`', '\''], ['*', ':', '+'], ['~', '"', '.']];

pub(crate) struct Sonnet1SpirographKnobs {
    pub speed: f32,
    pub hue: f32,
    pub depth: f32,
    pub trail: f32,
    pub glow: f32,
    pub arms: f32,
    pub echo: f32,
    pub label: f32,
    pub scale: f32,
    pub margin: f32,
    pub aspect: f32,
}

impl Sonnet1SpirographKnobs {
    pub(crate) fn from_env() -> Self {
        Sonnet1SpirographKnobs {
            speed: param_f32("SPEED", 1.0),
            hue: param_f32("HUE", 0.0),
            depth: param_f32("DEPTH", 0.85),
            trail: param_f32("TRAIL", 0.5),
            glow: param_f32("GLOW", 1.0),
            arms: param_f32("ARMS", 1.0),
            echo: param_f32("ECHO", 0.5),
            label: param_f32("LABEL", 1.0),
            scale: param_f32("SCALE", 0.92),
            margin: param_f32("MARGIN", 1.0),
            aspect: param_f32("ASPECT", 2.0),
        }
    }
}

struct Geom {
    seed: u64,
    r: f32,
    rr: f32,
    inside: bool,
    dist: f32,
    ratio: f32,
    q: u32,
    base_hue: f64,
    bloom: usize,
    dotted_arm: bool,
}

thread_local! {
    static CACHE: RefCell<Option<Geom>> = const { RefCell::new(None) };
}

fn gcd(mut a: u32, mut b: u32) -> u32 {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a.max(1)
}

fn build(seed: u64) -> Geom {
    let mut rng = StdRng::seed_from_u64(seed ^ 0xA5F1_9E2D_5C31_44B7);
    let rr = rng.random_range(6..=13) as f32;
    let r = rng.random_range(1..=5) as f32;
    let inside = rng.random::<f32>() < 0.5;
    let dist = if inside { rr - r } else { rr + r };
    let g = gcd(rr as u32, r as u32);
    let q = (r as u32 / g).max(1);
    let base_hue = (rng.random::<f32>() * 360.0) as f64;
    let bloom = rng.random_range(0..3u32) as usize;
    let dotted_arm = rng.random::<f32>() < 0.5;
    Geom { seed, r, rr, inside, dist, ratio: dist / r, q, base_hue, bloom, dotted_arm }
}

fn pen_point(g: &Geom, d: f32, theta: f32) -> (f32, f32) {
    let inner = g.ratio * theta;
    let base_x = g.dist * theta.cos();
    let base_y = g.dist * theta.sin();
    if g.inside {
        (base_x + d * inner.cos(), base_y - d * inner.sin())
    } else {
        (base_x - d * inner.cos(), base_y - d * inner.sin())
    }
}

fn roll_center(g: &Geom, theta: f32) -> (f32, f32) {
    (g.dist * theta.cos(), g.dist * theta.sin())
}

fn hash(a: u32, b: u32, c: u32, seed: u64) -> f32 {
    let mut h = (a as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (b as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9)
        ^ (c as u64).wrapping_mul(0x94D0_49BB_1331_11EB)
        ^ seed;
    h ^= h >> 31;
    h = h.wrapping_mul(0xD6E8_FEB8_6659_FD93);
    h ^= h >> 32;
    (h & 0xFF_FFFF) as f32 / 16_777_216.0
}

fn put(grid: &mut Grid, w: usize, h: usize, x: i32, y: i32, cell: Cell) {
    if x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h {
        grid[y as usize][x as usize] = cell;
    }
}

fn put_text(grid: &mut Grid, w: usize, h: usize, x: i32, y: i32, text: &str, fg: Color, bg: Color) {
    for (i, ch) in text.chars().enumerate() {
        put(grid, w, h, x + i as i32, y, Cell::with_bg(ch, fg, bg));
    }
}

struct View {
    cx: f32,
    cy: f32,
    unit: f32,
    aspect: f32,
}

impl View {
    fn map(&self, mx: f32, my: f32) -> (i32, i32) {
        ((self.cx + mx * self.unit * self.aspect).round() as i32, (self.cy + my * self.unit).round() as i32)
    }
}

fn draw_ring(grid: &mut Grid, w: usize, h: usize, view: &View, cx: f32, cy: f32, r: f32, col: Color, bg: Color) {
    let n = ((r * view.unit * TAU / 1.1).round() as i32).clamp(24, 240) as usize;
    for i in 0..n {
        let theta = TAU * i as f32 / n as f32;
        let (x, y) = view.map(cx + r * theta.cos(), cy + r * theta.sin());
        put(grid, w, h, x, y, Cell::with_bg('.', col, bg));
    }
}

fn draw_arm(grid: &mut Grid, w: usize, h: usize, view: &View, ax: f32, ay: f32, bx: f32, by: f32, col: Color, bg: Color, dotted: bool) {
    let (x0, y0) = view.map(ax, ay);
    let (x1, y1) = view.map(bx, by);
    let (dx, dy) = (x1 - x0, y1 - y0);
    let steps = dx.abs().max(dy.abs()).max(1);
    // walk the arm in screen space; the glyph reads the local slope so the
    // rod looks straight instead of a staircase of one repeated character
    for s in 0..=steps {
        if dotted && s % 2 == 1 {
            continue;
        }
        let x = x0 + dx * s / steps;
        let y = y0 + dy * s / steps;
        let ch = if dx.abs() > dy.abs() * 2 {
            '-'
        } else if dy.abs() > dx.abs() * 2 {
            '|'
        } else if (dx > 0) == (dy > 0) {
            '\\'
        } else {
            '/'
        };
        put(grid, w, h, x, y, Cell::with_bg(ch, col, bg));
    }
    put(grid, w, h, x1, y1, Cell::with_bg('+', lighten(col, 30), bg));
}

pub(crate) fn draw_sonnet_1_spirograph(grid: &mut Grid, w: usize, h: usize, seed: u64, palette: &[Color; 5], t: f32, k: &Sonnet1SpirographKnobs) {
    CACHE.with(|cell| {
        let mut slot = cell.borrow_mut();
        let stale = slot.as_ref().map(|c| c.seed != seed).unwrap_or(true);
        if stale {
            *slot = Some(build(seed));
        }
        let g = slot.as_ref().unwrap();
        render(grid, w, h, seed, palette, t, k, g);
    });
}

fn render(grid: &mut Grid, w: usize, h: usize, seed: u64, palette: &[Color; 5], t: f32, k: &Sonnet1SpirographKnobs, g: &Geom) {
    let canvas_bg = darken(palette[0], 8);
    measure_layer("sonnet-1-spirograph", "clear", || {
        for row in grid.iter_mut().take(h) {
            for cell in row.iter_mut().take(w) {
                *cell = Cell::with_bg(' ', palette[4], canvas_bg);
            }
        }
    });
    if w < 10 || h < 6 {
        return;
    }

    let depth = k.depth.clamp(0.0, 1.6);
    let d = depth * g.r;
    let extent = (g.dist + d).max(0.5);
    let aspect = k.aspect.clamp(1.0, 3.0);
    let scale = k.scale.clamp(0.1, 1.0);
    let mg = (k.margin.round() as i32).clamp(0, 10);
    let avail_w = (w as i32 - 2 * mg).max(4) as f32;
    let avail_h = (h as i32 - 2 * mg).max(4) as f32;
    let half_h = avail_h * 0.5 * scale;
    let half_w = avail_w * 0.5 * scale;
    let unit = (half_h.min(half_w / aspect) / extent).max(0.05);
    let view = View { cx: w as f32 * 0.5, cy: h as f32 * 0.5, unit, aspect };

    let cycle = (BASE_CYCLE / k.speed.max(0.05)).max(1.0);
    let av = TAU * g.q as f32 / cycle;
    let theta_now = if t > 0.0 { t * av } else { 0.0 };

    let hue = (g.base_hue + k.hue as f64).rem_euclid(360.0);
    let echo_col = hsl_to_rgb(hue, 0.4, 0.16);
    let trail_col = hsl_to_rgb(hue, 0.8, 0.55);
    let mech_col = darken(palette[4], 55);
    let pen_col = lighten(hsl_to_rgb(hue + 20.0, 0.9, 0.62), 20);
    let rim_col = palette[2];

    measure_layer("sonnet-1-spirograph", "echo", || {
        if k.echo > 0.01 {
            let dim = ((1.0 - k.echo.clamp(0.0, 1.0)) * 85.0) as u8;
            let col = darken(echo_col, dim);
            let span = TAU * g.q as f32;
            for i in 0..ECHO_SAMPLES {
                let theta = span * i as f32 / ECHO_SAMPLES as f32;
                let (mx, my) = pen_point(g, d, theta);
                let (x, y) = view.map(mx, my);
                put(grid, w, h, x, y, Cell::with_bg('.', col, canvas_bg));
            }
        }
    });

    measure_layer("sonnet-1-spirograph", "mechanism", || {
        if k.arms > 0.5 {
            draw_ring(grid, w, h, &view, 0.0, 0.0, g.rr, mech_col, canvas_bg);
            let (rcx, rcy) = roll_center(g, theta_now);
            draw_ring(grid, w, h, &view, rcx, rcy, g.r, lighten(mech_col, 20), canvas_bg);
            let (px, py) = pen_point(g, d, theta_now);
            draw_arm(grid, w, h, &view, rcx, rcy, px, py, mech_col, canvas_bg, g.dotted_arm);
        }
    });

    measure_layer("sonnet-1-spirograph", "trail", || {
        let frac = k.trail.clamp(0.05, 1.0);
        let span = frac * TAU * 1.5;
        let count = ((TRAIL_SAMPLES as f32 * frac).round() as usize).max(24);
        for i in 0..count {
            let f = i as f32 / count as f32;
            let theta = theta_now - span * (1.0 - f);
            let (mx, my) = pen_point(g, d, theta);
            let (x, y) = view.map(mx, my);
            let fade = ((1.0 - f) * 100.0) as u8;
            let ch = if f > 0.94 {
                '@'
            } else if f > 0.8 {
                '#'
            } else if f > 0.55 {
                '*'
            } else if f > 0.3 {
                '='
            } else {
                ':'
            };
            put(grid, w, h, x, y, Cell::with_bg(ch, darken(trail_col, fade), canvas_bg));
        }
    });

    measure_layer("sonnet-1-spirograph", "bloom", || {
        let glow = k.glow.clamp(0.0, 3.0);
        if glow > 0.01 {
            let (px, py) = pen_point(g, d, theta_now);
            let (cx, cy) = view.map(px, py);
            put(grid, w, h, cx, cy, Cell::with_bg('@', lighten(pen_col, 40), canvas_bg));
            let n = (16.0 * glow).round() as u32;
            let frame = (t * 6.0).floor() as u32;
            let set = BLOOM_SETS[g.bloom];
            for i in 0..n {
                let u = hash(i, frame, 1, seed);
                let v = hash(i, frame, 2, seed);
                let s = hash(i, frame, 3, seed);
                let ang = u * TAU;
                let rad = 1.0 + v * 3.0 * glow.max(0.3);
                let x = cx + (ang.cos() * rad).round() as i32;
                let y = cy + (ang.sin() * rad * 0.5).round() as i32;
                let ch = set[(s * 3.0) as usize % 3];
                let bright = 90 - (v * 70.0) as u8;
                put(grid, w, h, x, y, Cell::with_bg(ch, darken(pen_col, bright), canvas_bg));
            }
        }
    });

    measure_layer("sonnet-1-spirograph", "rim", || {
        let x0 = mg - 1;
        let x1 = w as i32 - mg;
        let y0 = mg - 1;
        let y1 = h as i32 - mg;
        if x1 > x0 && y1 > y0 {
            for x in x0..=x1 {
                let ch = if x == x0 || x == x1 { '+' } else { '-' };
                put(grid, w, h, x, y0, Cell::with_bg(ch, rim_col, canvas_bg));
                put(grid, w, h, x, y1, Cell::with_bg(ch, rim_col, canvas_bg));
            }
            for y in y0..=y1 {
                put(grid, w, h, x0, y, Cell::with_bg('|', rim_col, canvas_bg));
                put(grid, w, h, x1, y, Cell::with_bg('|', rim_col, canvas_bg));
            }
        }
    });

    measure_layer("sonnet-1-spirograph", "label", || {
        if k.label > 0.5 {
            let side = if g.inside { "in" } else { "out" };
            let text = format!("R{}:r{} {} d{:.2}", g.rr as i32, g.r as i32, side, depth);
            put_text(grid, w, h, mg.max(1), h as i32 - mg.max(1) - 1, &text, palette[4], canvas_bg);
        }
    });
}

pub(crate) fn cli_sonnet_1_spirograph(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
    let _ = (rng, term_w, term_h, mode, theme_name);
    let mut k = Sonnet1SpirographKnobs::from_env();
    let pos: Vec<f32> = args.iter().skip(4).filter_map(|a| a.parse().ok()).collect();
    let slots: [&mut f32; 11] = [
        &mut k.speed,
        &mut k.hue,
        &mut k.depth,
        &mut k.trail,
        &mut k.glow,
        &mut k.arms,
        &mut k.echo,
        &mut k.label,
        &mut k.scale,
        &mut k.margin,
        &mut k.aspect,
    ];
    for (slot, v) in slots.into_iter().zip(pos.iter()) {
        *slot = *v;
    }
    draw_sonnet_1_spirograph(&mut grid, width, height, seed, &palette, t_anim, &k);
    (grid, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(w: usize, h: usize, seed: u64, t: f32) -> String {
        let mut g = vec![vec![Cell::blank(); w]; h];
        let p = crate::color::make_palette(seed);
        let k = Sonnet1SpirographKnobs::from_env();
        draw_sonnet_1_spirograph(&mut g, w, h, seed, &p, t, &k);
        g.iter().map(|row| row.iter().map(|c| c.ch).collect::<String>()).collect::<Vec<_>>().join("\n")
    }

    #[test]
    fn snapshot_sonnet_1_spirograph_static() {
        insta::assert_snapshot!("sonnet_1_spirograph_80x24", run(80, 24, 42, 0.0));
    }

    #[test]
    fn snapshot_sonnet_1_spirograph_moving() {
        insta::assert_snapshot!("sonnet_1_spirograph_110x36_t9", run(110, 36, 42, 9.0));
    }

    #[test]
    fn deterministic_and_seed_sensitive() {
        assert_eq!(run(90, 30, 42, 0.0), run(90, 30, 42, 0.0));
        assert_ne!(run(90, 30, 42, 0.0), run(90, 30, 7, 0.0));
    }

    #[test]
    fn t_zero_is_static_and_t_moves_the_pen() {
        assert_eq!(run(90, 30, 42, 0.0), run(90, 30, 42, 0.0));
        assert_ne!(run(90, 30, 42, 0.0), run(90, 30, 42, 5.0));
        assert_ne!(run(90, 30, 42, 5.0), run(90, 30, 42, 11.0));
    }

    #[test]
    fn frame_cost() {
        let (w, h) = (200usize, 60usize);
        let mut g = vec![vec![Cell::blank(); w]; h];
        let p = crate::color::make_palette(42);
        let k = Sonnet1SpirographKnobs::from_env();
        draw_sonnet_1_spirograph(&mut g, w, h, 42, &p, 0.0, &k);
        let mut worst = 0.0f64;
        let start = std::time::Instant::now();
        for f in 0..200 {
            let t0 = std::time::Instant::now();
            draw_sonnet_1_spirograph(&mut g, w, h, 42, &p, f as f32 * 0.05, &k);
            worst = worst.max(t0.elapsed().as_secs_f64() * 1000.0);
        }
        let avg = start.elapsed().as_secs_f64() * 1000.0 / 200.0;
        eprintln!("sonnet-1-spirograph frame_cost 200x60: avg {:.3} ms, worst {:.3} ms", avg, worst);
        if !cfg!(debug_assertions) {
            assert!(avg < 4.0, "avg frame {:.3} ms", avg);
        }
    }
}
