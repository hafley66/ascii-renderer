//! chladni -- sand on a center-driven square plate, the figure stepping through
//! seeded resonances. Node lines hold sand; antinodes hop loose grains.
use crate::_0_profile::measure_layer;
use crate::color::*;
use crate::opts::param_f32;
use crate::types::*;
use crossterm::style::Color;
use rand::RngExt;
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::cell::RefCell;
use std::f32::consts::PI;

const SEQ: usize = 64;

pub(crate) struct ChladniKnobs {
    pub dwell: f32,
    pub glide: f32,
    pub order: f32,
    pub sand: f32,
    pub shake: f32,
    pub flicker: f32,
    pub margin: f32,
    pub label: f32,
    pub aspect: f32,
}

impl ChladniKnobs {
    pub(crate) fn from_env() -> Self {
        ChladniKnobs {
            dwell: param_f32("DWELL", 3.0),
            glide: param_f32("GLIDE", 2.0),
            order: param_f32("ORDER", 7.0),
            sand: param_f32("SAND", 0.02),
            shake: param_f32("SHAKE", 0.3),
            flicker: param_f32("FLICKER", 8.0),
            margin: param_f32("MARGIN", 1.0),
            label: param_f32("LABEL", 1.0),
            aspect: param_f32("ASPECT", 2.0),
        }
    }

    fn order_n(&self) -> u32 {
        (self.order.round() as u32).clamp(2, 24)
    }
}

#[derive(Clone, Copy)]
struct Figure {
    n: f32,
    m: f32,
    sign: f32,
}

struct Cached {
    key: (u64, u32),
    seq: Vec<Figure>,
    col_n: Vec<f32>,
    col_m: Vec<f32>,
    row_n: Vec<f32>,
    row_m: Vec<f32>,
}

thread_local! {
    static CACHE: RefCell<Option<Cached>> = RefCell::new(None);
}

fn build(seed: u64, order: u32) -> Cached {
    let mut rng = StdRng::seed_from_u64(seed ^ 0xC41AD71);
    let mut seq: Vec<Figure> = Vec::with_capacity(SEQ);
    let mut last = (0u32, 0u32);
    for _ in 0..SEQ {
        let mut pair = last;
        for _ in 0..16 {
            let n = rng.random_range(1..order);
            let m = rng.random_range(n + 1..=order);
            pair = (n, m);
            if pair != last {
                break;
            }
        }
        last = pair;
        let sign = if rng.random::<f32>() < 0.5 { -1.0 } else { 1.0 };
        seq.push(Figure { n: pair.0 as f32, m: pair.1 as f32, sign });
    }
    Cached { key: (seed, order), seq, col_n: Vec::new(), col_m: Vec::new(), row_n: Vec::new(), row_m: Vec::new() }
}

fn hash(x: u32, y: u32, k: u32, seed: u64) -> f32 {
    let mut h = (x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (y as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9)
        ^ (k as u64).wrapping_mul(0x94D0_49BB_1331_11EB)
        ^ seed;
    h ^= h >> 31;
    h = h.wrapping_mul(0xD6E8_FEB8_6659_FD93);
    h ^= h >> 32;
    (h & 0xFF_FFFF) as f32 / 16_777_216.0
}

fn smooth(s: f32) -> f32 {
    let s = s.clamp(0.0, 1.0);
    s * s * (3.0 - 2.0 * s)
}

struct Drive {
    fig: Figure,
    from: Figure,
    to: Figure,
    glide: f32,
}

fn drive_at(t: f32, dwell: f32, glide: f32, seq: &[Figure]) -> Drive {
    let period = dwell + glide;
    let i = (t / period).floor().max(0.0) as usize;
    let tau = t - i as f32 * period;
    let from = seq[i % seq.len()];
    let to = seq[(i + 1) % seq.len()];
    let s = if glide > 0.0 { smooth((tau - dwell) / glide) } else { 0.0 };
    let fig = Figure {
        n: from.n + (to.n - from.n) * s,
        m: from.m + (to.m - from.m) * s,
        sign: from.sign + (to.sign - from.sign) * s,
    };
    Drive { fig, from, to, glide: s }
}

fn field(fig: &Figure, px: f32, py: f32) -> f32 {
    let a = (fig.n * PI * px).cos() * (fig.m * PI * py).cos();
    let b = (fig.m * PI * px).cos() * (fig.n * PI * py).cos();
    a + fig.sign * b
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

pub(crate) fn draw_chladni(grid: &mut Grid, w: usize, h: usize, seed: u64, palette: &[Color; 5], t: f32, k: &ChladniKnobs) {
    let order = k.order_n();
    CACHE.with(|cell| {
        let mut slot = cell.borrow_mut();
        let key = (seed, order);
        let stale = slot.as_ref().map(|c| c.key != key).unwrap_or(true);
        if stale {
            *slot = Some(build(seed, order));
        }
        let c = slot.as_mut().unwrap();
        render(grid, w, h, seed, palette, t, k, c);
    });
}

fn render(grid: &mut Grid, w: usize, h: usize, seed: u64, palette: &[Color; 5], t: f32, k: &ChladniKnobs, c: &mut Cached) {
    measure_layer("chladni", "clear", || {
        for row in grid.iter_mut().take(h) {
            for cell in row.iter_mut().take(w) {
                *cell = Cell::blank();
            }
        }
    });
    if w < 6 || h < 4 {
        return;
    }
    let dwell = k.dwell.max(0.0);
    let glide = k.glide.max(0.0);
    let aspect = k.aspect.max(0.25);
    let mg = (k.margin.round() as i32).clamp(1, 12) as usize;
    let avail_w = w.saturating_sub(2 * mg).max(4) as f32;
    let avail_h = h.saturating_sub(2 * mg).max(2) as f32;
    let mut plate_h = avail_h;
    let mut plate_w = (plate_h * aspect).round();
    if plate_w > avail_w {
        plate_w = avail_w;
        plate_h = (plate_w / aspect).round().max(2.0);
    }
    let pw = plate_w as i32;
    let ph = plate_h as i32;
    let x0 = ((w as i32 - pw) / 2).max(1);
    let y0 = ((h as i32 - ph) / 2).max(1);

    let d = drive_at(t, dwell, glide, &c.seq);
    let storm = 1.0 + 3.0 * d.glide * (1.0 - d.glide) * 4.0;
    let eps = (k.sand.max(0.002) * PI * (d.fig.n + d.fig.m) * 0.5).max(1e-3);
    let frame = (t * k.flicker.max(0.0)).floor() as u32;
    let shake = k.shake.clamp(0.0, 1.0);

    let plate_bg = lighten(palette[0], 6);
    let rim = palette[2];
    let core_fg = lighten(palette[4], 20);
    let slope_fg = palette[3];
    let foot_fg = darken(palette[3], 50);
    let hop_fg = darken(palette[1], 20);

    // separable field: f = cn[x]*rm[y] + sign*cm[x]*rn[y]; sand bands by |f| thresholds
    c.col_n.clear();
    c.col_m.clear();
    c.row_n.clear();
    c.row_m.clear();
    for xx in 0..pw {
        let px = (xx as f32 + 0.5) / plate_w * 2.0 - 1.0;
        c.col_n.push((d.fig.n * PI * px).cos());
        c.col_m.push((d.fig.m * PI * px).cos());
    }
    for yy in 0..ph {
        let py = (yy as f32 + 0.5) / plate_h * 2.0 - 1.0;
        c.row_n.push((d.fig.n * PI * py).cos());
        c.row_m.push((d.fig.m * PI * py).cos());
    }
    let core_lim = eps * (-(0.8f32).ln()).sqrt();
    let slope_lim = eps * (-(0.5f32).ln()).sqrt();
    let foot_lim = eps * (-(0.22f32).ln()).sqrt();
    let hop_scale = shake * storm * 0.5 * 0.25;
    let sign = d.fig.sign;
    let (col_n, col_m, row_n, row_m) = (&c.col_n, &c.col_m, &c.row_n, &c.row_m);
    measure_layer("chladni", "field", || {
        for yy in 0..ph {
            let (rn, rm) = (row_n[yy as usize], row_m[yy as usize]);
            let y = y0 + yy;
            if y < 0 || y as usize >= h {
                continue;
            }
            let row = &mut grid[y as usize];
            for xx in 0..pw {
                let x = x0 + xx;
                if x < 0 || x as usize >= w {
                    continue;
                }
                let f = col_n[xx as usize] * rm + sign * col_m[xx as usize] * rn;
                let af = f.abs();
                let u = hash(x as u32, y as u32, 0, seed);
                let cell = if af < core_lim {
                    Cell::with_bg(if u < 0.65 { '#' } else { '%' }, core_fg, plate_bg)
                } else if af < slope_lim {
                    Cell::with_bg(if u < 0.5 { '+' } else { '=' }, slope_fg, plate_bg)
                } else if af < foot_lim {
                    Cell::with_bg(':', foot_fg, plate_bg)
                } else {
                    let amp = af.min(2.0);
                    let v = hash(x as u32, y as u32, frame + 1, seed);
                    if v < hop_scale * amp * amp {
                        let g = hash(x as u32, y as u32, frame + 7, seed);
                        let ch = if g < 0.45 { '.' } else if g < 0.8 { '`' } else { '~' };
                        Cell::with_bg(ch, hop_fg, plate_bg)
                    } else {
                        Cell::with_bg(' ', hop_fg, plate_bg)
                    }
                };
                row[x as usize] = cell;
            }
        }
    });

    measure_layer("chladni", "rim", || {
        for xx in -1..=pw {
            let ch = if xx == -1 || xx == pw { '+' } else { '-' };
            put(grid, w, h, x0 + xx, y0 - 1, Cell::new(ch, rim));
            put(grid, w, h, x0 + xx, y0 + ph, Cell::new(ch, rim));
        }
        for yy in 0..ph {
            put(grid, w, h, x0 - 1, y0 + yy, Cell::new('|', rim));
            put(grid, w, h, x0 + pw, y0 + yy, Cell::new('|', rim));
        }
    });

    measure_layer("chladni", "drive", || {
        let cx = x0 + pw / 2;
        let cy = y0 + ph / 2;
        let pulse = 0.5 + 0.5 * (t * 4.0 * PI).sin();
        let drive_fg = lighten(palette[3], (pulse * 80.0) as u8);
        put(grid, w, h, cx, cy, Cell::with_bg('@', drive_fg, plate_bg));
        let ring = if pulse > 0.5 { 'o' } else { '*' };
        for (dx, dy) in [(-2, 0), (2, 0), (0, -1), (0, 1)] {
            put(grid, w, h, cx + dx, cy + dy, Cell::with_bg(ring, darken(drive_fg, 40), plate_bg));
        }
    });

    measure_layer("chladni", "label", || {
        if k.label > 0.5 {
            let mut text = format!("{}:{}", d.from.n as i32, d.from.m as i32);
            if d.glide > 0.0 {
                text.push_str(&format!(">{}:{}", d.to.n as i32, d.to.m as i32));
            }
            put_text(grid, w, h, x0 + 1, y0 + ph - 1, &text, palette[4], plate_bg);
        }
    });
}

pub(crate) fn cli_chladni(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
    let _ = (rng, term_w, term_h, mode, theme_name);
    let mut k = ChladniKnobs::from_env();
    let pos: Vec<f32> = args.iter().skip(4).filter_map(|a| a.parse().ok()).collect();
    let slots: [&mut f32; 9] = [
        &mut k.dwell,
        &mut k.glide,
        &mut k.order,
        &mut k.sand,
        &mut k.shake,
        &mut k.flicker,
        &mut k.margin,
        &mut k.label,
        &mut k.aspect,
    ];
    for (slot, v) in slots.into_iter().zip(pos.iter()) {
        *slot = *v;
    }
    draw_chladni(&mut grid, width, height, seed, &palette, t_anim, &k);
    (grid, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(w: usize, h: usize, seed: u64, t: f32) -> String {
        let mut g = vec![vec![Cell::blank(); w]; h];
        let p = crate::color::make_palette(seed);
        let k = ChladniKnobs::from_env();
        draw_chladni(&mut g, w, h, seed, &p, t, &k);
        g.iter()
            .map(|row| row.iter().map(|c| c.ch).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn snapshot_chladni_small() {
        insta::assert_snapshot!("chladni_80x24", run(80, 24, 42, 0.0));
    }

    #[test]
    fn snapshot_chladni_large() {
        insta::assert_snapshot!("chladni_110x36", run(110, 36, 42, 0.0));
    }

    #[test]
    fn order_two_terminates() {
        let mut g = vec![vec![Cell::blank(); 40]; 12];
        let p = crate::color::make_palette(3);
        let mut k = ChladniKnobs::from_env();
        k.order = 2.0;
        draw_chladni(&mut g, 40, 12, 3, &p, 5.0, &k);
    }

    #[test]
    fn deterministic_and_seed_sensitive() {
        assert_eq!(run(90, 30, 42, 0.0), run(90, 30, 42, 0.0));
        assert_ne!(run(90, 30, 42, 0.0), run(90, 30, 7, 0.0));
    }

    #[test]
    fn t_steps_the_figure() {
        assert_ne!(run(90, 30, 42, 0.0), run(90, 30, 42, 4.0));
        assert_ne!(run(90, 30, 42, 4.0), run(90, 30, 42, 9.0));
    }

    #[test]
    fn frame_cost() {
        let (w, h) = (200usize, 60usize);
        let mut g = vec![vec![Cell::blank(); w]; h];
        let p = crate::color::make_palette(42);
        let k = ChladniKnobs::from_env();
        draw_chladni(&mut g, w, h, 42, &p, 0.0, &k);
        let mut worst = 0.0f64;
        let start = std::time::Instant::now();
        for f in 0..200 {
            let t0 = std::time::Instant::now();
            draw_chladni(&mut g, w, h, 42, &p, f as f32 * 0.05, &k);
            worst = worst.max(t0.elapsed().as_secs_f64() * 1000.0);
        }
        let avg = start.elapsed().as_secs_f64() * 1000.0 / 200.0;
        eprintln!("chladni frame_cost 200x60: avg {:.3} ms, worst {:.3} ms", avg, worst);
        if !cfg!(debug_assertions) {
            assert!(avg < 4.0, "avg frame {:.3} ms", avg);
        }
    }
}
