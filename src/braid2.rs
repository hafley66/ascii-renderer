//! braid-2 -- a horizontal plait of twisted flat ribbons with beads running
//! along each strand. Crossings scroll left with t; beads travel right.
use crate::_0_profile::measure_layer;
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

pub(crate) struct Braid2Knobs {
    pub strands: f32,
    pub speed: f32,
    pub pitch: f32,
    pub gap: f32,
    pub width: f32,
    pub cross: f32,
    pub twist: f32,
    pub pulse: f32,
    pub beads: f32,
    pub trail: f32,
    pub slip: f32,
    pub fill: f32,
}

impl Braid2Knobs {
    pub(crate) fn from_env() -> Self {
        Braid2Knobs {
            strands: param_f32("STRANDS", 5.0),
            speed: param_f32("SPEED", 6.0),
            pitch: param_f32("PITCH", 12.0),
            gap: param_f32("GAP", 4.0),
            width: param_f32("WIDTH", 3.0),
            cross: param_f32("CROSS", 0.5),
            twist: param_f32("TWIST", 28.0),
            pulse: param_f32("PULSE", 10.0),
            beads: param_f32("BEADS", 36.0),
            trail: param_f32("TRAIL", 7.0),
            slip: param_f32("SLIP", 0.15),
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
    bead_off: [f32; MAX_STRANDS],
    color: [Color; MAX_STRANDS],
}

thread_local! {
    static CACHE: RefCell<Option<Cached>> = RefCell::new(None);
}

fn build(w: usize, h: usize, seed: u64, n: usize, slip: f32, fill: f32, palette: &[Color; 5]) -> Cached {
    let mut rng = StdRng::seed_from_u64(seed ^ 0x2B2A1D);
    let mut half: Vec<(u16, u16)> = Vec::with_capacity(WORD_HALF);
    for k in 0..WORD_HALF {
        let parity = k % 2;
        let mut lanes: u16 = 0;
        let mut over_top: u16 = 0;
        for lane in (parity..n - 1).step_by(2) {
            if rng.random::<f32>() >= fill {
                continue;
            }
            lanes |= 1 << lane;
            let base_top = lane % 2 == 0;
            let flip = rng.random::<f32>() < slip;
            if base_top != flip {
                over_top |= 1 << lane;
            }
        }
        half.push((lanes, over_top));
    }
    // second half is the inverse word, so the full cycle returns to identity
    let mut word = half.clone();
    for &(lanes, over_top) in half.iter().rev() {
        word.push((lanes, !over_top & lanes));
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
    let mut bead_off = [0.0f32; MAX_STRANDS];
    for s in 0..MAX_STRANDS {
        phase[s] = rng.random::<f32>() * std::f32::consts::TAU;
        bead_off[s] = rng.random::<f32>();
    }
    let base = [palette[1], palette[3], palette[2], palette[4]];
    let mut color = [palette[1]; MAX_STRANDS];
    for (s, c) in color.iter_mut().enumerate() {
        let b = base[s % base.len()];
        let turn = (s / base.len()) as f64 * 37.0;
        *c = shift_hue(b, turn);
    }
    Cached {
        key: (w, h, seed, n, slip.to_bits(), fill.to_bits()),
        n,
        word,
        perm,
        phase,
        bead_off,
        color,
    }
}

fn put(grid: &mut Grid, w: usize, h: usize, x: i32, y: i32, ch: char, fg: Color) {
    if x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h {
        grid[y as usize][x as usize] = Cell::new(ch, fg);
    }
}

struct Lane {
    y: f32,
    strand: usize,
    slope: i8,
    over: bool,
    under: bool,
    knot: bool,
    half: i32,
    bead: f32,
    trail: f32,
}

fn paint_lane(grid: &mut Grid, w: usize, h: usize, x: i32, l: &Lane, c: &Cached) {
    let cy = l.y.round() as i32;
    let body = c.color[l.strand];
    let glow = (l.bead * 90.0) as u8;
    let (core, rim, fill) = if l.under {
        (darken(body, 55), darken(body, 70), darken(body, 60))
    } else if l.over {
        (lighten(body, 18 + glow), lighten(body, glow), lighten(body, 8 + glow))
    } else {
        (lighten(body, glow), lighten(darken(body, 25), glow), lighten(darken(body, 10), glow))
    };
    let diag = match l.slope {
        1 => '\\',
        -1 => '/',
        _ => '-',
    };
    if l.over {
        put(grid, w, h, x, cy - l.half - 1, ' ', body);
        put(grid, w, h, x, cy + l.half + 1, ' ', body);
    }
    let head = l.bead > 0.0 && (1.0 - l.bead) * l.trail < 1.0;
    let mid_trail = l.bead > 0.5;
    for dy in -l.half..=l.half {
        let y = cy + dy;
        let is_rim = dy == -l.half || dy == l.half;
        let ch = if l.knot && dy == 0 {
            'X'
        } else if l.knot {
            '+'
        } else if head && dy == 0 {
            'o'
        } else if head {
            '*'
        } else if mid_trail && dy == 0 {
            '*'
        } else if l.bead > 0.0 && dy == 0 {
            ':'
        } else if l.half == 0 {
            if l.slope == 0 { '~' } else { diag }
        } else if l.slope != 0 {
            diag
        } else if l.under {
            ':'
        } else if is_rim {
            '-'
        } else if dy == 0 {
            '='
        } else {
            '#'
        };
        let fg = if is_rim && l.half > 0 { rim } else if dy == 0 { core } else { fill };
        put(grid, w, h, x, y, ch, fg);
    }
}

pub(crate) fn draw_braid2(grid: &mut Grid, w: usize, h: usize, seed: u64, palette: &[Color; 5], t: f32, k: &Braid2Knobs) {
    let n = k.n();
    let slip = k.slip.clamp(0.0, 1.0);
    let fill = k.fill.clamp(0.0, 1.0);
    CACHE.with(|cell| {
        let mut slot = cell.borrow_mut();
        let key = (w, h, seed, n, slip.to_bits(), fill.to_bits());
        let stale = slot.as_ref().map(|c| c.key != key).unwrap_or(true);
        if stale {
            *slot = Some(build(w, h, seed, n, slip, fill, palette));
        }
        let c = slot.as_ref().unwrap();
        render(grid, w, h, t, k, c);
    });
}

fn render(grid: &mut Grid, w: usize, h: usize, t: f32, k: &Braid2Knobs, c: &Cached) {
    let n = c.n;
    let pitch = k.pitch.max(2.0);
    let speed = k.speed;
    let half_max = ((k.width.round() as i32 - 1) / 2).clamp(0, 4);
    let width = half_max * 2 + 1;
    let max_gap = ((h as f32 - width as f32 - 2.0) / n as f32).max(1.0);
    let gap = k.gap.clamp(1.0, max_gap);
    let cross = k.cross.clamp(0.1, 1.0);
    let twist = k.twist.max(2.0);
    let pulse = k.pulse;
    let beads = k.beads.max(2.0);
    let trail = k.trail.clamp(0.5, beads);
    let len = c.word.len();
    let cy0 = h as f32 / 2.0 - 0.5;

    measure_layer("braid-2", "clear", || {
        for row in grid.iter_mut().take(h) {
            for cell in row.iter_mut().take(w) {
                *cell = Cell::blank();
            }
        }
    });

    measure_layer("braid-2", "lanes", || {
        for x in 0..w as i32 {
            let u = t * speed + x as f32;
            let kf = (u / pitch).floor();
            let frac = u - kf * pitch;
            let ki = (kf as i64).rem_euclid(len as i64) as usize;
            let (lanes_bits, over_top) = c.word[ki];
            let a = pitch * (1.0 - cross) * 0.5;
            let b = pitch * (1.0 + cross) * 0.5;
            let s = ((frac - a) / (b - a)).clamp(0.0, 1.0);
            let moving = s > 0.0 && s < 1.0;
            let mid_col = (frac - pitch * 0.5).abs() < 0.5;

            let mut lanes: [Lane; MAX_STRANDS] = std::array::from_fn(|_| Lane { y: 0.0, strand: 0, slope: 0, over: false, under: false, knot: false, half: 0, bead: 0.0, trail: 1.0 });
            for i in 0..n {
                let strand = c.perm[ki * n + i] as usize;
                let ly = |lane: usize| cy0 + (lane as f32 - (n as f32 - 1.0) * 0.5) * gap;
                let face = (std::f32::consts::TAU * u / twist + c.phase[strand]).cos().abs();
                let half = (face * (half_max as f32 + 0.49)).floor() as i32;
                let d = (t * pulse + c.bead_off[strand] * beads - u).rem_euclid(beads);
                let bead = if d < trail { 1.0 - d / trail } else { 0.0 };
                let mut l = Lane { y: ly(i), strand, slope: 0, over: false, under: false, knot: false, half: half.min(half_max), bead, trail };
                if lanes_bits & (1 << i) != 0 {
                    l.y = ly(i) + (ly(i + 1) - ly(i)) * s;
                    l.slope = if moving { 1 } else { 0 };
                    let top_over = over_top & (1 << i) != 0;
                    l.over = moving && top_over;
                    l.under = moving && !top_over;
                } else if i > 0 && lanes_bits & (1 << (i - 1)) != 0 {
                    l.y = ly(i) + (ly(i - 1) - ly(i)) * s;
                    l.slope = if moving { -1 } else { 0 };
                    let top_over = over_top & (1 << (i - 1)) != 0;
                    l.over = moving && !top_over;
                    l.under = moving && top_over;
                }
                l.knot = l.over && mid_col;
                lanes[i] = l;
            }
            for i in 0..n {
                if !lanes[i].over {
                    paint_lane(grid, w, h, x, &lanes[i], c);
                }
            }
            for i in 0..n {
                if lanes[i].over {
                    paint_lane(grid, w, h, x, &lanes[i], c);
                }
            }
        }
    });
}

pub(crate) fn cli_braid2(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
    let _ = (rng, term_w, term_h, mode, theme_name);
    let mut k = Braid2Knobs::from_env();
    let pos: Vec<f32> = args.iter().skip(4).filter_map(|a| a.parse().ok()).collect();
    let slots: [&mut f32; 12] = [
        &mut k.strands,
        &mut k.speed,
        &mut k.pitch,
        &mut k.gap,
        &mut k.width,
        &mut k.cross,
        &mut k.twist,
        &mut k.pulse,
        &mut k.beads,
        &mut k.trail,
        &mut k.slip,
        &mut k.fill,
    ];
    for (slot, v) in slots.into_iter().zip(pos.iter()) {
        *slot = *v;
    }
    draw_braid2(&mut grid, width, height, seed, &palette, t_anim, &k);
    (grid, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(w: usize, h: usize, seed: u64, t: f32) -> String {
        let mut g = vec![vec![Cell::blank(); w]; h];
        let p = crate::color::make_palette(seed);
        let k = Braid2Knobs::from_env();
        draw_braid2(&mut g, w, h, seed, &p, t, &k);
        g.iter()
            .map(|row| row.iter().map(|c| c.ch).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn snapshot_braid2_small() {
        insta::assert_snapshot!("braid2_80x24", run(80, 24, 42, 0.0));
    }

    #[test]
    fn snapshot_braid2_large() {
        insta::assert_snapshot!("braid2_110x36", run(110, 36, 42, 0.0));
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
        let k = Braid2Knobs::from_env();
        draw_braid2(&mut g, w, h, 42, &p, 0.0, &k);
        let mut worst = 0.0f64;
        let start = std::time::Instant::now();
        for f in 0..200 {
            let t0 = std::time::Instant::now();
            draw_braid2(&mut g, w, h, 42, &p, f as f32 * 0.05, &k);
            worst = worst.max(t0.elapsed().as_secs_f64() * 1000.0);
        }
        let avg = start.elapsed().as_secs_f64() * 1000.0 / 200.0;
        eprintln!("braid-2 frame_cost 200x60: avg {:.3} ms, worst {:.3} ms", avg, worst);
        if !cfg!(debug_assertions) {
            assert!(avg < 4.0, "avg frame {:.3} ms", avg);
        }
    }
}
