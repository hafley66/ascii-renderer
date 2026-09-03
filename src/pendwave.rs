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
    pub link: f32,
    pub arc: f32,
    pub rowdt: f32,
    pub bands: f32,
}

impl PendWaveKnobs {
    pub(crate) fn from_env() -> Self {
        PendWaveKnobs {
            count: param_f32("COUNT", 15.0),
            cycle: param_f32("CYCLE", 120.0),
            base: param_f32("BASE", 20.0),
            swing: param_f32("SWING", 0.5),
            view: param_f32("VIEW", 2.0),
            trail: param_f32("TRAIL", 8.0),
            tail: param_f32("TAIL", 0.1),
            aspect: param_f32("ASPECT", 2.0),
            hue: param_f32("HUE", 18.0),
            link: param_f32("LINK", 0.0),
            arc: param_f32("ARC", 0.0),
            rowdt: param_f32("ROWDT", 0.2),
            bands: param_f32("BANDS", 0.0),
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

fn put_soft(grid: &mut Grid, w: usize, h: usize, x: i32, y: i32, ch: char, fg: Color) {
    if x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h && grid[y as usize][x as usize].ch == ' ' {
        grid[y as usize][x as usize] = Cell::new(ch, fg);
    }
}

fn pill(grid: &mut Grid, w: usize, h: usize, x: i32, y: i32, c: Color) {
    let bg = darken(c, 70);
    let cells = [(-1, '(', darken(c, 10)), (0, '@', lighten(c, 60)), (1, ')', darken(c, 10))];
    for (dx, ch, fg) in cells {
        if x + dx >= 0 && y >= 0 && ((x + dx) as usize) < w && (y as usize) < h {
            grid[y as usize][(x + dx) as usize] = Cell::with_bg(ch, fg, bg);
        }
    }
}

fn link(grid: &mut Grid, w: usize, h: usize, a: (i32, i32), b: (i32, i32), fg: Color) {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let steps = dx.abs().max(dy.abs());
    if steps < 2 {
        return;
    }
    let slope = dx as f32 / (dy as f32).abs().max(0.5);
    let ch = if slope.abs() < 0.7 { '|' } else if slope.abs() > 3.0 { '-' } else if (dx > 0) == (dy > 0) { '\\' } else { '/' };
    for s in 1..steps {
        let x = a.0 + ((dx * s) as f32 / steps as f32).round() as i32;
        let y = a.1 + ((dy * s) as f32 / steps as f32).round() as i32;
        put_soft(grid, w, h, x, y, ch, fg);
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
    } else if k.view < 1.5 {
        top(grid, w, h, palette, t, k, &rig, &colors);
    } else {
        waterfall(grid, w, h, palette, t, k, &rig, &colors);
    }
}

fn front(grid: &mut Grid, w: usize, h: usize, palette: &[Color; 5], t: f32, k: &PendWaveKnobs, rig: &Rig, colors: &[Color]) {
    let n = rig.n;
    let aspect = k.aspect.max(0.25);
    let beam_y = 0;
    let floor_y = h as i32 - 1;
    let lmax = (h as f32 - 3.0).max(1.0);
    let step = w as f32 / (n as f32 + 1.0);
    let trail = (k.trail.round() as i32).clamp(0, 200);
    let tail = k.tail.max(0.001);
    let beam_fg = palette[1];
    let floor_fg = darken(palette[1], 40);
    let shadow_fg = darken(palette[2], 40);
    let arc_fg = darken(palette[2], 70);
    let link_fg = darken(palette[4], 110);

    let pos = |i: usize, th: f32| -> (f32, f32) {
        let px = step * (i as f32 + 1.0);
        let len = lmax * rig.length(i);
        (px + len * th.sin() * aspect, 1.0 + len * th.cos())
    };

    let x_first = (step * 0.5).round() as i32;
    let x_last = (step * (n as f32 + 0.5)).round() as i32;
    for x in x_first..=x_last {
        put(grid, w, h, x, beam_y, '=', beam_fg);
        put(grid, w, h, x, floor_y, '-', floor_fg);
    }

    let mut bobs: [(i32, i32); MAX_COUNT] = [(0, 0); MAX_COUNT];
    for i in 0..n {
        let px = (step * (i as f32 + 1.0)).round() as i32;
        let c = colors[i];
        let (bx, by) = pos(i, rig.angle(i, t));
        let rows = (by - 1.0).max(1.0);
        let slope = (bx - px as f32) / rows;
        let ch = if slope.abs() < 0.4 { '|' } else if slope > 0.0 { '\\' } else { '/' };
        let last = by.floor() as i32;
        for r in 1..=last {
            let frac = (r as f32 - 1.0) / rows;
            let x = (px as f32 + (bx - px as f32) * frac).round() as i32;
            put(grid, w, h, x, r, ch, darken(c, 90 - (frac * 60.0) as u8));
        }
        put(grid, w, h, px, beam_y, '+', lighten(beam_fg, 40));
        bobs[i] = (bx.round() as i32, by.round() as i32);
    }
    if k.arc > 0.5 {
        for i in 0..n {
            for j in 0..=32 {
                let th = -rig.swing + 2.0 * rig.swing * j as f32 / 32.0;
                let (x, y) = pos(i, th);
                put_soft(grid, w, h, x.round() as i32, y.round() as i32, '.', arc_fg);
            }
        }
    }
    for i in 0..n {
        let c = colors[i];
        for j in (1..=trail).rev() {
            let (x, y) = pos(i, rig.angle(i, t - j as f32 * tail));
            let f = j as f32 / trail as f32;
            let ch = if j <= 2 { 'o' } else { '.' };
            put_soft(grid, w, h, x.round() as i32, y.round() as i32, ch, darken(c, 20 + (f * 100.0) as u8));
        }
    }
    if k.link > 1.5 {
        for i in 1..n {
            link(grid, w, h, bobs[i - 1], bobs[i], link_fg);
        }
    }
    for i in 0..n {
        let (bx, by) = bobs[i];
        let wide = 1 + ((1.0 - rig.length(i)) * 3.0).round() as i32;
        for dx in -wide..=wide {
            put(grid, w, h, bx + dx, floor_y, '~', shadow_fg);
        }
        pill(grid, w, h, bx, by, colors[i]);
    }
}

fn top(grid: &mut Grid, w: usize, h: usize, palette: &[Color; 5], t: f32, k: &PendWaveKnobs, rig: &Rig, colors: &[Color]) {
    let n = rig.n;
    let cx = w as f32 * 0.5;
    let reach = (w as f32 * 0.5 - 3.0).max(1.0);
    let norm = if rig.swing > 0.01 { rig.swing.sin() } else { 1.0 };
    let trail = (k.trail.round() as i32).clamp(0, 200);
    let tail = k.tail.max(0.001);
    let guide_fg = darken(palette[2], 60);
    let link_fg = darken(palette[4], 110);
    let lane = h as f32 / n as f32;
    let lane_y = |i: usize| ((i as f32 + 0.5) * lane).floor() as i32;
    let bob_x = |i: usize, tt: f32| (cx + reach * rig.angle(i, tt).sin() / norm).round() as i32;

    for y in 0..h as i32 {
        put(grid, w, h, cx as i32, y, ':', guide_fg);
    }
    if k.arc > 0.5 {
        for i in 0..n {
            let y = lane_y(i);
            let x0 = (cx - reach).round() as i32;
            let x1 = (cx + reach).round() as i32;
            for x in x0..=x1 {
                put_soft(grid, w, h, x, y, '.', darken(guide_fg, 20));
            }
        }
    }
    for i in 0..n {
        let y = lane_y(i);
        let c = colors[i];
        for j in (1..=trail).rev() {
            let x = bob_x(i, t - j as f32 * tail);
            let f = j as f32 / trail as f32;
            let ch = if f < 0.25 { '*' } else if f < 0.5 { '=' } else if f < 0.75 { ':' } else { '.' };
            put(grid, w, h, x, y, ch, darken(c, (f * 110.0) as u8));
        }
    }
    if k.link > 0.5 {
        for i in 1..n {
            link(grid, w, h, (bob_x(i - 1, t), lane_y(i - 1)), (bob_x(i, t), lane_y(i)), link_fg);
        }
    }
    for i in 0..n {
        pill(grid, w, h, bob_x(i, t), lane_y(i), colors[i]);
    }
}

const RAMP: [char; 10] = [' ', '.', ':', '-', '=', '+', '*', '#', '%', '@'];

fn waterfall(grid: &mut Grid, w: usize, h: usize, palette: &[Color; 5], t: f32, k: &PendWaveKnobs, rig: &Rig, colors: &[Color]) {
    let n = rig.n;
    let dt = k.rowdt.max(0.0005);
    let bands = k.bands > 0.5;
    let beam_fg = palette[1];
    for x in 0..w {
        let u = x as f32 / (w as f32 - 1.0).max(1.0);
        let idx = u * (n as f32 - 1.0);
        let idx = if bands { idx.round() } else { idx };
        let beats = rig.base + if rig.flip { n as f32 - 1.0 - idx } else { idx };
        let ci = (idx.round() as usize).min(n - 1);
        let c = colors[ci];
        for y in 1..h {
            let tt = t - (y as f32 - 1.0) * dt;
            let v = (TAU * beats * tt / rig.cycle).cos();
            let level = ((v + 1.0) * 0.5 * (RAMP.len() as f32 - 0.01)).floor() as usize;
            let ch = RAMP[level.min(RAMP.len() - 1)];
            let fg = if v >= 0.0 { lighten(c, (v * 70.0) as u8) } else { darken(c, (-v * 90.0) as u8 + 10) };
            grid[y][x] = Cell::new(ch, fg);
        }
        grid[0][x] = Cell::new('=', beam_fg);
    }
    let step = w as f32 / (n as f32 + 1.0);
    for i in 0..n {
        let x = (step * (i as f32 + 1.0)).round() as i32;
        let v = rig.angle(i, t) / rig.swing.max(0.01);
        let ch = if v > 0.33 { ')' } else if v < -0.33 { '(' } else { '|' };
        put(grid, w, h, x, 0, ch, lighten(colors[i], 50));
    }
}

pub(crate) fn cli_pendwave(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
    let _ = (rng, term_w, term_h, mode, theme_name);
    let mut k = PendWaveKnobs::from_env();
    let pos: Vec<f32> = args.iter().skip(4).filter_map(|a| a.parse().ok()).collect();
    let slots: [&mut f32; 13] = [
        &mut k.count,
        &mut k.cycle,
        &mut k.base,
        &mut k.swing,
        &mut k.view,
        &mut k.trail,
        &mut k.tail,
        &mut k.aspect,
        &mut k.hue,
        &mut k.link,
        &mut k.arc,
        &mut k.rowdt,
        &mut k.bands,
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
    fn snapshot_pendwave_waterfall() {
        insta::assert_snapshot!("pendwave_waterfall_110x36_t7", run(110, 36, 42, 7.0, 2.0));
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
