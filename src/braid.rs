//! braid -- a vertical plait of colored ribbons tied by a seeded braid word.
//! Crossings scroll downward with t; over strands occlude under strands.
use crate::color::*;
use crate::opts::param_f32;
use crate::types::*;
use crossterm::style::Color;
use rand::RngExt;
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::cell::RefCell;

const MAX_STRANDS: usize = 16;
const WORD_HALF: usize = 2048;

pub(crate) struct BraidKnobs {
    pub strands: f32,
    pub speed: f32,
    pub pitch: f32,
    pub gap: f32,
    pub width: f32,
    pub cross: f32,
    pub sway: f32,
    pub dust: f32,
    pub twist: f32,
    pub fill: f32,
}

impl BraidKnobs {
    pub(crate) fn from_env() -> Self {
        BraidKnobs {
            strands: param_f32("STRANDS", 5.0),
            speed: param_f32("SPEED", 4.0),
            pitch: param_f32("PITCH", 6.0),
            gap: param_f32("GAP", 8.0),
            width: param_f32("WIDTH", 3.0),
            cross: param_f32("CROSS", 0.6),
            sway: param_f32("SWAY", 1.0),
            dust: param_f32("DUST", 0.06),
            twist: param_f32("TWIST", 0.85),
            fill: param_f32("FILL", 0.75),
        }
    }

    fn n(&self) -> usize {
        (self.strands.round() as usize).clamp(2, MAX_STRANDS)
    }
}

struct Cached {
    key: (usize, usize, u64, usize, u32, u32),
    n: usize,
    word: Vec<(u16, u16)>,
    perm: Vec<u8>,
    phase: [f32; MAX_STRANDS],
    color: [Color; MAX_STRANDS],
}

thread_local! {
    static CACHE: RefCell<Option<Cached>> = RefCell::new(None);
}

fn build(w: usize, h: usize, seed: u64, n: usize, twist: f32, fill: f32, palette: &[Color; 5]) -> Cached {
    let mut rng = StdRng::seed_from_u64(seed ^ 0xB2A1D);
    let mut half: Vec<(u16, u16)> = Vec::with_capacity(WORD_HALF);
    for k in 0..WORD_HALF {
        let parity = k % 2;
        let mut lanes: u16 = 0;
        let mut over_left: u16 = 0;
        for lane in (parity..n - 1).step_by(2) {
            if rng.random::<f32>() >= fill {
                continue;
            }
            lanes |= 1 << lane;
            let base_left = lane % 2 == 0;
            let slip = rng.random::<f32>() >= twist;
            if base_left != slip {
                over_left |= 1 << lane;
            }
        }
        half.push((lanes, over_left));
    }
    // second half is the inverse word, so the full cycle returns to identity
    let mut word = half.clone();
    for &(lanes, over_left) in half.iter().rev() {
        word.push((lanes, !over_left & lanes));
    }
    let len = word.len();
    let mut perm: Vec<u8> = Vec::with_capacity(len * n);
    let mut state: Vec<u8> = (0..n as u8).collect();
    for &(lanes, _) in word.iter() {
        perm.extend_from_slice(&state);
        for lane in 0..n - 1 {
            if lanes & (1 << lane) != 0 {
                state.swap(lane, lane + 1);
            }
        }
    }
    let mut phase = [0.0f32; MAX_STRANDS];
    for p in phase.iter_mut() {
        *p = rng.random::<f32>() * std::f32::consts::TAU;
    }
    let base = [palette[1], palette[3], palette[2], palette[4]];
    let mut color = [palette[1]; MAX_STRANDS];
    for (s, c) in color.iter_mut().enumerate() {
        let b = base[s % base.len()];
        let turn = (s / base.len()) as f64 * 37.0;
        *c = shift_hue(b, turn);
    }
    Cached {
        key: (w, h, seed, n, twist.to_bits(), fill.to_bits()),
        n,
        word,
        perm,
        phase,
        color,
    }
}

fn hash2(x: i32, y: i32, seed: u64) -> u32 {
    let mut v = (x as u32).wrapping_mul(0x9E37_79B9) ^ (y as u32).wrapping_mul(0x85EB_CA6B) ^ (seed as u32);
    v ^= v >> 15;
    v = v.wrapping_mul(0x2C1B_3C6D);
    v ^= v >> 12;
    v
}

fn put(grid: &mut Grid, w: usize, h: usize, x: i32, y: i32, ch: char, fg: Color) {
    if x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h {
        grid[y as usize][x as usize] = Cell::new(ch, fg);
    }
}

struct Lane {
    x: f32,
    strand: usize,
    slope: i8,
    over: bool,
    under: bool,
    knot: bool,
}

fn paint_lane(grid: &mut Grid, w: usize, h: usize, y: i32, lane: &Lane, half: i32, c: &Cached) {
    let cx = lane.x.round() as i32;
    let body = c.color[lane.strand];
    let (core, edge, fill) = if lane.under {
        (darken(body, 55), darken(body, 70), darken(body, 60))
    } else if lane.over {
        (lighten(body, 18), body, lighten(body, 8))
    } else {
        (body, darken(body, 25), darken(body, 10))
    };
    let edge_ch = match lane.slope {
        1 => '/',
        -1 => '\\',
        _ => '|',
    };
    if lane.over {
        put(grid, w, h, cx - half - 1, y, ' ', body);
        put(grid, w, h, cx + half + 1, y, ' ', body);
    }
    for dx in -half..=half {
        let x = cx + dx;
        let is_edge = dx == -half || dx == half;
        let ch = if lane.knot && dx == 0 {
            'X'
        } else if lane.knot && is_edge {
            '+'
        } else if is_edge {
            edge_ch
        } else if lane.under {
            if (x + y) % 2 == 0 { ':' } else { '.' }
        } else if half > 1 && dx == 0 {
            if lane.slope == 0 { '|' } else { edge_ch }
        } else {
            '#'
        };
        let fg = if is_edge { edge } else if dx == 0 { core } else { fill };
        put(grid, w, h, x, y, ch, fg);
    }
}

pub(crate) fn draw_braid(grid: &mut Grid, w: usize, h: usize, seed: u64, palette: &[Color; 5], t: f32, k: &BraidKnobs) {
    let n = k.n();
    let twist = k.twist.clamp(0.0, 1.0);
    let fill = k.fill.clamp(0.0, 1.0);
    CACHE.with(|cell| {
        let mut slot = cell.borrow_mut();
        let key = (w, h, seed, n, twist.to_bits(), fill.to_bits());
        let stale = slot.as_ref().map(|c| c.key != key).unwrap_or(true);
        if stale {
            *slot = Some(build(w, h, seed, n, twist, fill, palette));
        }
        let c = slot.as_ref().unwrap();
        render(grid, w, h, seed, palette, t, k, c);
    });
}

fn render(grid: &mut Grid, w: usize, h: usize, seed: u64, palette: &[Color; 5], t: f32, k: &BraidKnobs, c: &Cached) {
    let n = c.n;
    let pitch = k.pitch.max(2.0);
    let speed = k.speed;
    let half = ((k.width.round() as i32 - 1) / 2).clamp(0, 6);
    let width = half * 2 + 1;
    let max_gap = ((w as f32 - width as f32 - 2.0) / n as f32).max(1.0);
    let gap = k.gap.clamp(1.0, max_gap.max(1.0));
    let cross = k.cross.clamp(0.1, 1.0);
    let sway = k.sway.max(0.0);
    let dust = k.dust.clamp(0.0, 1.0);
    let len = c.word.len();
    let cx0 = w as f32 / 2.0 - 0.5;
    let dust_c = darken(palette[2], 30);
    let dust_hi = darken(palette[4], 90);
    let dust_glyph = ['.', '`', '\''];

    for y in 0..h as i32 {
        let u = t * speed - y as f32 + h as f32;
        let kf = (u / pitch).floor();
        let frac = u - kf * pitch;
        let ki = (kf as i64).rem_euclid(len as i64) as usize;
        let (lanes_bits, over_left) = c.word[ki];
        let a = pitch * (1.0 - cross) * 0.5;
        let b = pitch * (1.0 + cross) * 0.5;
        let s = ((frac - a) / (b - a)).clamp(0.0, 1.0);
        let moving = s > 0.0 && s < 1.0;
        let mid_row = ((frac - pitch * 0.5).abs()) < 0.5;
        let uy = (u.floor()) as i32;

        for x in 0..w as i32 {
            let hv = hash2(x, uy, seed);
            let r = (hv & 0xFFFF) as f32 / 65535.0;
            if r < dust {
                let g = dust_glyph[((hv >> 16) % 3) as usize];
                let fg = if (hv >> 20) & 7 == 0 { dust_hi } else { dust_c };
                grid[y as usize][x as usize] = Cell::new(g, fg);
            } else {
                grid[y as usize][x as usize] = Cell::blank();
            }
        }

        let mut lanes: [Lane; MAX_STRANDS] = std::array::from_fn(|_| Lane { x: 0.0, strand: 0, slope: 0, over: false, under: false, knot: false });
        for i in 0..n {
            let strand = c.perm[ki * n + i] as usize;
            let ph = c.phase[strand];
            let wob = sway * (t * 0.9 + ph + y as f32 * 0.12).sin();
            let lx = |lane: usize| cx0 + (lane as f32 - (n as f32 - 1.0) * 0.5) * gap + wob;
            let mut l = Lane { x: lx(i), strand, slope: 0, over: false, under: false, knot: false };
            if lanes_bits & (1 << i) != 0 {
                l.x = lx(i) + (lx(i + 1) - lx(i)) * s;
                l.slope = if moving { 1 } else { 0 };
                let left_over = over_left & (1 << i) != 0;
                l.over = moving && left_over;
                l.under = moving && !left_over;
            } else if i > 0 && lanes_bits & (1 << (i - 1)) != 0 {
                l.x = lx(i) + (lx(i - 1) - lx(i)) * s;
                l.slope = if moving { -1 } else { 0 };
                let left_over = over_left & (1 << (i - 1)) != 0;
                l.over = moving && !left_over;
                l.under = moving && left_over;
            }
            l.knot = l.over && mid_row;
            lanes[i] = l;
        }
        for i in 0..n {
            if !lanes[i].over {
                paint_lane(grid, w, h, y, &lanes[i], half, c);
            }
        }
        for i in 0..n {
            if lanes[i].over {
                paint_lane(grid, w, h, y, &lanes[i], half, c);
            }
        }
    }
}

pub(crate) fn cli_braid(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
    let _ = (rng, term_w, term_h, mode, theme_name);
    let mut k = BraidKnobs::from_env();
    let pos: Vec<f32> = args.iter().skip(4).filter_map(|a| a.parse().ok()).collect();
    let slots: [&mut f32; 10] = [
        &mut k.strands,
        &mut k.speed,
        &mut k.pitch,
        &mut k.gap,
        &mut k.width,
        &mut k.cross,
        &mut k.sway,
        &mut k.dust,
        &mut k.twist,
        &mut k.fill,
    ];
    for (slot, v) in slots.into_iter().zip(pos.iter()) {
        *slot = *v;
    }
    draw_braid(&mut grid, width, height, seed, &palette, t_anim, &k);
    (grid, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(w: usize, h: usize, seed: u64, t: f32) -> String {
        let mut g = vec![vec![Cell::blank(); w]; h];
        let p = crate::color::make_palette(seed);
        let k = BraidKnobs::from_env();
        draw_braid(&mut g, w, h, seed, &p, t, &k);
        g.iter()
            .map(|row| row.iter().map(|c| c.ch).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn snapshot_braid_small() {
        insta::assert_snapshot!("braid_80x24", run(80, 24, 42, 0.0));
    }

    #[test]
    fn snapshot_braid_large() {
        insta::assert_snapshot!("braid_110x36", run(110, 36, 42, 0.0));
    }

    #[test]
    fn deterministic_and_seed_sensitive() {
        assert_eq!(run(90, 30, 42, 0.0), run(90, 30, 42, 0.0));
        assert_ne!(run(90, 30, 42, 0.0), run(90, 30, 7, 0.0));
    }

    #[test]
    fn t_scrolls_the_plait() {
        assert_ne!(run(90, 30, 42, 0.0), run(90, 30, 42, 3.0));
        assert_ne!(run(90, 30, 42, 3.0), run(90, 30, 42, 6.0));
    }

    #[test]
    fn frame_cost() {
        let (w, h) = (200usize, 60usize);
        let mut g = vec![vec![Cell::blank(); w]; h];
        let p = crate::color::make_palette(42);
        let k = BraidKnobs::from_env();
        draw_braid(&mut g, w, h, 42, &p, 0.0, &k);
        let mut worst = 0.0f64;
        let start = std::time::Instant::now();
        for f in 0..200 {
            let t0 = std::time::Instant::now();
            draw_braid(&mut g, w, h, 42, &p, f as f32 * 0.05, &k);
            worst = worst.max(t0.elapsed().as_secs_f64() * 1000.0);
        }
        let avg = start.elapsed().as_secs_f64() * 1000.0 / 200.0;
        eprintln!("braid frame_cost 200x60: avg {:.3} ms, worst {:.3} ms", avg, worst);
        if !cfg!(debug_assertions) {
            assert!(avg < 4.0, "avg frame {:.3} ms", avg);
        }
    }
}
