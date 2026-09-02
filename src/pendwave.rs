//! pendulum-wave -- a row of pendulums on one beam, each one beat faster than
//! its neighbor. Front view with strings and shadows, or top view with trails.
use crate::color::*;
use crate::opts::param_f32;
use crate::types::*;
use crossterm::style::Color;
use rand::rngs::StdRng;
use std::f32::consts::TAU;

const MAX_COUNT: usize = 64;

pub(crate) struct PendWaveKnobs {
    pub count: f32,
    pub cycle: f32,
    pub base: f32,
    pub swing: f32,
    pub view: f32,
    pub trail: f32,
    pub tail: f32,
    pub aspect: f32,
    pub hue: f32,
}

impl PendWaveKnobs {
    pub(crate) fn from_env() -> Self {
        PendWaveKnobs {
            count: param_f32("COUNT", 15.0),
            cycle: param_f32("CYCLE", 30.0),
            base: param_f32("BASE", 20.0),
            swing: param_f32("SWING", 0.5),
            view: param_f32("VIEW", 0.0),
            trail: param_f32("TRAIL", 12.0),
            tail: param_f32("TAIL", 0.06),
            aspect: param_f32("ASPECT", 2.0),
            hue: param_f32("HUE", 18.0),
        }
    }

    fn n(&self) -> usize {
        (self.count.round() as usize).clamp(1, MAX_COUNT)
    }
}

fn put(grid: &mut Grid, w: usize, h: usize, x: i32, y: i32, ch: char, fg: Color) {
    if x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h {
        grid[y as usize][x as usize] = Cell::new(ch, fg);
    }
}

struct Rig {
    n: usize,
    cycle: f32,
    base: f32,
    swing: f32,
    flip: bool,
}

impl Rig {
    fn beats(&self, i: usize) -> f32 {
        let idx = if self.flip { self.n - 1 - i } else { i };
        self.base + idx as f32
    }

    fn length(&self, i: usize) -> f32 {
        let r = self.base / self.beats(i);
        r * r
    }

    fn angle(&self, i: usize, t: f32) -> f32 {
        self.swing * (TAU * self.beats(i) * t / self.cycle).cos()
    }
}

pub(crate) fn draw_pendwave(grid: &mut Grid, w: usize, h: usize, seed: u64, palette: &[Color; 5], t: f32, k: &PendWaveKnobs) {
    for row in grid.iter_mut().take(h) {
        for cell in row.iter_mut().take(w) {
            *cell = Cell::blank();
        }
    }
    if w < 8 || h < 5 {
        return;
    }
    let rig = Rig {
        n: k.n(),
        cycle: k.cycle.max(0.1),
        base: k.base.max(1.0),
        swing: k.swing.clamp(0.0, 1.5),
        flip: seed & 1 == 1,
    };
    let bob_base = shift_hue(palette[3], (seed % 360) as f64);
    let colors: Vec<Color> = (0..rig.n).map(|i| shift_hue(bob_base, i as f64 * k.hue as f64)).collect();
    if k.view < 0.5 {
        front(grid, w, h, palette, t, k, &rig, &colors);
    } else {
        top(grid, w, h, palette, t, k, &rig, &colors);
    }
}

fn front(grid: &mut Grid, w: usize, h: usize, palette: &[Color; 5], t: f32, k: &PendWaveKnobs, rig: &Rig, colors: &[Color]) {
    let n = rig.n;
    let aspect = k.aspect.max(0.25);
    let beam_y = 0;
    let floor_y = h as i32 - 1;
    let lmax = (h as f32 - 3.0).max(1.0);
    let step = w as f32 / (n as f32 + 1.0);
    let beam_fg = palette[1];
    let string_fg = darken(palette[2], 20);
    let floor_fg = darken(palette[1], 40);
    let shadow_fg = darken(palette[2], 50);

    let x_first = (step * 0.5).round() as i32;
    let x_last = (step * (n as f32 + 0.5)).round() as i32;
    for x in x_first..=x_last {
        put(grid, w, h, x, beam_y, '=', beam_fg);
        put(grid, w, h, x, floor_y, '-', floor_fg);
    }

    let mut bobs: [(i32, i32); MAX_COUNT] = [(0, 0); MAX_COUNT];
    for i in 0..n {
        let px = (step * (i as f32 + 1.0)).round() as i32;
        let len = lmax * rig.length(i);
        let th = rig.angle(i, t);
        let bx = px as f32 + len * th.sin() * aspect;
        let by = 1.0 + len * th.cos();
        let rows = (by - 1.0).max(1.0);
        let slope = (bx - px as f32) / rows;
        let ch = if slope.abs() < 0.4 { '|' } else if slope > 0.0 { '\\' } else { '/' };
        let last = by.floor() as i32;
        for r in 1..=last {
            let frac = (r as f32 - 1.0) / rows;
            let x = (px as f32 + (bx - px as f32) * frac).round() as i32;
            put(grid, w, h, x, r, ch, string_fg);
        }
        put(grid, w, h, px, beam_y, '+', lighten(beam_fg, 30));
        bobs[i] = (bx.round() as i32, by.round() as i32);
    }
    for i in 0..n {
        let (bx, by) = bobs[i];
        let c = colors[i];
        for dx in -1..=1 {
            put(grid, w, h, bx + dx, floor_y, '~', shadow_fg);
        }
        put(grid, w, h, bx - 1, by, '(', darken(c, 30));
        put(grid, w, h, bx, by, '@', lighten(c, 30));
        put(grid, w, h, bx + 1, by, ')', darken(c, 30));
    }
}

fn top(grid: &mut Grid, w: usize, h: usize, palette: &[Color; 5], t: f32, k: &PendWaveKnobs, rig: &Rig, colors: &[Color]) {
    let n = rig.n;
    let cx = w as f32 * 0.5;
    let reach = (w as f32 * 0.5 - 3.0).max(1.0);
    let norm = if rig.swing > 0.01 { rig.swing.sin() } else { 1.0 };
    let trail = (k.trail.round() as i32).clamp(0, 200);
    let tail = k.tail.max(0.001);
    let guide_fg = darken(palette[2], 50);
    let lane = h as f32 / n as f32;
    for i in 0..n {
        let y = ((i as f32 + 0.5) * lane).floor() as i32;
        put(grid, w, h, cx as i32, y, ':', guide_fg);
        let c = colors[i];
        for j in (1..=trail).rev() {
            let th = rig.angle(i, t - j as f32 * tail);
            let x = (cx + reach * th.sin() / norm).round() as i32;
            let f = j as f32 / trail as f32;
            let ch = if f < 0.34 { '*' } else if f < 0.67 { ':' } else { '.' };
            put(grid, w, h, x, y, ch, darken(c, (f * 110.0) as u8));
        }
        let th = rig.angle(i, t);
        let x = (cx + reach * th.sin() / norm).round() as i32;
        put(grid, w, h, x - 1, y, '(', darken(c, 30));
        put(grid, w, h, x, y, '@', lighten(c, 30));
        put(grid, w, h, x + 1, y, ')', darken(c, 30));
    }
}

pub(crate) fn cli_pendwave(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
    let _ = (rng, term_w, term_h, mode, theme_name);
    let mut k = PendWaveKnobs::from_env();
    let pos: Vec<f32> = args.iter().skip(4).filter_map(|a| a.parse().ok()).collect();
    let slots: [&mut f32; 9] = [
        &mut k.count,
        &mut k.cycle,
        &mut k.base,
        &mut k.swing,
        &mut k.view,
        &mut k.trail,
        &mut k.tail,
        &mut k.aspect,
        &mut k.hue,
    ];
    for (slot, v) in slots.into_iter().zip(pos.iter()) {
        *slot = *v;
    }
    draw_pendwave(&mut grid, width, height, seed, &palette, t_anim, &k);
    (grid, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(w: usize, h: usize, seed: u64, t: f32, view: f32) -> String {
        let mut g = vec![vec![Cell::blank(); w]; h];
        let p = crate::color::make_palette(seed);
        let mut k = PendWaveKnobs::from_env();
        k.view = view;
        draw_pendwave(&mut g, w, h, seed, &p, t, &k);
        g.iter()
            .map(|row| row.iter().map(|c| c.ch).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn snapshot_pendwave_front() {
        insta::assert_snapshot!("pendwave_front_80x24", run(80, 24, 42, 0.0, 0.0));
    }

    #[test]
    fn snapshot_pendwave_top() {
        insta::assert_snapshot!("pendwave_top_80x24", run(80, 24, 42, 0.0, 1.0));
    }

    #[test]
    fn snapshot_pendwave_front_mid_cycle() {
        insta::assert_snapshot!("pendwave_front_110x36_t7", run(110, 36, 42, 7.0, 0.0));
    }

    #[test]
    fn deterministic_and_seed_sensitive() {
        assert_eq!(run(90, 30, 42, 2.0, 0.0), run(90, 30, 42, 2.0, 0.0));
        assert_ne!(run(90, 30, 42, 2.0, 0.0), run(90, 30, 7, 2.0, 0.0));
    }

    #[test]
    fn t_swings_the_bobs() {
        assert_ne!(run(90, 30, 42, 0.0, 0.0), run(90, 30, 42, 1.0, 0.0));
        assert_ne!(run(90, 30, 42, 0.0, 1.0), run(90, 30, 42, 1.0, 1.0));
    }

    #[test]
    fn frame_cost() {
        let (w, h) = (200usize, 60usize);
        let mut g = vec![vec![Cell::blank(); w]; h];
        let p = crate::color::make_palette(42);
        let k = PendWaveKnobs::from_env();
        draw_pendwave(&mut g, w, h, 42, &p, 0.0, &k);
        let mut worst = 0.0f64;
        let start = std::time::Instant::now();
        for f in 0..200 {
            let t0 = std::time::Instant::now();
            draw_pendwave(&mut g, w, h, 42, &p, f as f32 * 0.05, &k);
            worst = worst.max(t0.elapsed().as_secs_f64() * 1000.0);
        }
        let avg = start.elapsed().as_secs_f64() * 1000.0 / 200.0;
        eprintln!("pendulum-wave frame_cost 200x60: avg {:.3} ms, worst {:.3} ms", avg, worst);
        if !cfg!(debug_assertions) {
            assert!(avg < 4.0, "avg frame {:.3} ms", avg);
        }
    }
}
