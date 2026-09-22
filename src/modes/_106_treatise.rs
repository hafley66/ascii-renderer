//! Treatise: an endless graphic score in the manner of Cardew. A lifeline runs through
//! circles, bent staves, numbers and wedges; what the playhead has sounded crumbles to dust.
use crate::_0_profile::measure_layer;
use crate::color::{hsl_to_rgb, lerp_color, rgb};
use crate::opts::param_f32;
use crate::registry::{AnimKind, Mode, ModeFrame, Param};
use crate::types::{Cell, Grid};
use crossterm::style::Color;
use rayon::prelude::*;
use std::f32::consts::TAU;

pub(super) struct Treatise;
pub(super) static MODE: Treatise = Treatise;

const NAME: &str = "treatise";
const KNOBS: usize = 9;
const HELP: &str = "treatise: endless graphic score, sounded past crumbles to dust [scroll] [span] [staves] [life] [playhead] [paper] [aspect] [drift] [dust]";

const PARAMS: &[Param] = &[
    param!("SCROLL", "score scroll cols/s", 0.0, 30.0, 5.0, 0.5),
    param!("SPAN", "cols per score event", 8.0, 60.0, 20.0, 1.0),
    param!("STAVES", "empty staves below", 0.0, 2.0, 1.0, 1.0),
    param!("LIFE", "lifeline rows", 0.0, 3.0, 1.0, 1.0),
    param!("PLAYHEAD", "playhead position", 0.05, 0.95, 0.38, 0.01),
    param!("PAPER", "night to paper", 0.0, 1.0, 1.0, 0.05),
    param!("ASPECT", "cols per row", 1.0, 3.0, 2.0, 0.25),
    param!("DRIFT", "figure breathing", 0.0, 3.0, 0.7, 0.05),
    param!("DUST", "sounded past decay", 0.0, 1.0, 0.75, 0.05),
];

const L_FIG: u64 = 0x31;
const L_LIFE: u64 = 0x32;
const L_DUST: u64 = 0x33;
const L_STAFF: u64 = 0x34;
const KINDS: u32 = 8;
const STAFF_ROWS: usize = 6;
const PARALLEL_MIN_CELLS: usize = 20_480;

impl Mode for Treatise {
    fn name(&self) -> &'static str {
        NAME
    }
    fn help(&self) -> &'static str {
        HELP
    }
    fn animation(&self) -> AnimKind {
        AnimKind::Iterate
    }
    fn params(&self) -> &'static [Param] {
        PARAMS
    }
    fn render(&self, frame: &mut ModeFrame<'_>) {
        let p: [f32; KNOBS] = std::array::from_fn(|i| {
            let param = &PARAMS[i];
            let value = frame
                .args
                .get(i + 4)
                .and_then(|v| v.parse::<f32>().ok())
                .or_else(|| frame.param_values.and_then(|v| v.get(i)).copied())
                .unwrap_or_else(|| param_f32(param.key, param.default));
            if value.is_finite() {
                value.clamp(param.min, param.max)
            } else {
                param.default
            }
        });
        draw(frame, &p);
    }
}

#[inline]
fn hash(seed: u64, layer: u64, index: u64, slot: u64) -> u64 {
    let mut z = seed
        ^ layer.wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ index.wrapping_mul(0xD1B5_4A32_D192_ED03)
        ^ slot.wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    z ^= z >> 30;
    z = z.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z ^= z >> 27;
    z = z.wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

#[inline]
fn unit(h: u64) -> f32 {
    (h >> 40) as f32 * (1.0 / 16_777_216.0)
}

/// Stroke glyph for a direction in visual units (rows are `aspect` cols tall).
#[inline]
fn slope_char(dx: f32, dy: f32) -> char {
    if dx.abs() >= dy.abs() * 2.4 {
        '-'
    } else if dy.abs() >= dx.abs() * 2.4 {
        '|'
    } else if (dx > 0.0) == (dy > 0.0) {
        '\\'
    } else {
        '/'
    }
}

struct Look {
    seed: u64,
    w: usize,
    h: usize,
    score_h: usize,
    staves: usize,
    span: f32,
    scroll: i64,
    life: usize,
    playhead: usize,
    aspect: f32,
    t: f32,
    dust: f32,
    paper: Color,
    ink: Color,
    accent: Color,
}

impl Look {
    fn new(seed: u64, w: usize, h: usize, time: f32, p: &[f32; KNOBS]) -> Self {
        let mut staves = p[2].round() as usize;
        while staves > 0 && h < staves * STAFF_ROWS + 10 {
            staves -= 1;
        }
        let night = rgb(14, 13, 16);
        let cream = rgb(234, 227, 210);
        let hue = unit(hash(seed, L_FIG, 0, 99)) * 40.0 - 20.0;
        Look {
            seed,
            w,
            h,
            score_h: h - staves * STAFF_ROWS,
            staves,
            span: p[1].round(),
            scroll: (time * p[0]).floor() as i64,
            life: p[3].round() as usize,
            playhead: ((w as f32 * p[4]) as usize).min(w.saturating_sub(1)),
            aspect: p[6],
            t: time * p[7],
            dust: p[8],
            paper: lerp_color(night, cream, p[5]),
            ink: lerp_color(cream, rgb(22, 20, 19), p[5]),
            accent: hsl_to_rgb(((hue + 360.0) % 360.0) as f64, 0.78, 0.46),
        }
    }
}

/// Pen writing world-space columns onto the visible window of the score.
struct Pen<'a> {
    grid: &'a mut Grid,
    w: i64,
    rows: i64,
    scroll: i64,
    paper: Color,
}

impl Pen<'_> {
    #[inline]
    fn put(&mut self, wx: f32, y: f32, ch: char, fg: Color) {
        let x = wx.round() as i64 - self.scroll;
        let y = y.round() as i64;
        if x >= 0 && x < self.w && y >= 0 && y < self.rows {
            self.grid[y as usize][x as usize] = Cell::with_bg(ch, fg, self.paper);
        }
    }

    #[inline]
    fn solid(&mut self, wx: i64, y: i64, ch: char, fg: Color) {
        let x = wx - self.scroll;
        if x >= 0 && x < self.w && y >= 0 && y < self.rows {
            self.grid[y as usize][x as usize] = Cell::with_bg(ch, fg, fg);
        }
    }

    fn line(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, aspect: f32, fg: Color) {
        let (dx, dy) = (x1 - x0, y1 - y0);
        let n = dx.abs().max(dy.abs()).ceil().max(1.0) as i32;
        let ch = slope_char(dx / aspect, dy);
        for i in 0..=n {
            let t = i as f32 / n as f32;
            self.put(x0 + dx * t, y0 + dy * t, ch, fg);
        }
    }

    /// Visible world-column range of `[a, b]`.
    fn cols(&self, a: f32, b: f32) -> std::ops::RangeInclusive<i64> {
        (a.floor() as i64).max(self.scroll)..=(b.ceil() as i64).min(self.scroll + self.w - 1)
    }
}

fn draw(frame: &mut ModeFrame<'_>, p: &[f32; KNOBS]) {
    let (w, h) = (frame.width, frame.height);
    if w == 0 || h == 0 {
        return;
    }
    let lk = Look::new(frame.seed, w, h, frame.time, p);
    let grid = &mut *frame.grid;
    measure_layer(NAME, "paper", || paint_paper(grid, &lk));
    measure_layer(NAME, "staves", || paint_staves(grid, &lk));
    measure_layer(NAME, "lifeline", || paint_lifeline(grid, &lk));
    measure_layer(NAME, "figures", || paint_figures(grid, &lk));
    measure_layer(NAME, "dust", || paint_dust(grid, &lk));
    measure_layer(NAME, "playhead", || paint_playhead(grid, &lk));
}

fn rows_mut<'g>(grid: &'g mut Grid, lk: &Look) -> &'g mut [Vec<Cell>] {
    let h = lk.h.min(grid.len());
    &mut grid[..h]
}

fn paint_paper(grid: &mut Grid, lk: &Look) {
    let blank = Cell::with_bg(' ', lk.ink, lk.paper);
    let w = lk.w;
    let fill = |row: &mut Vec<Cell>| {
        let n = w.min(row.len());
        row[..n].fill(blank)
    };
    let rows = rows_mut(grid, lk);
    if lk.w * lk.h >= PARALLEL_MIN_CELLS {
        rows.par_iter_mut().for_each(fill);
    } else {
        rows.iter_mut().for_each(fill);
    }
}

fn pen<'a>(grid: &'a mut Grid, lk: &Look, rows: usize) -> Pen<'a> {
    Pen { grid, w: lk.w as i64, rows: rows as i64, scroll: lk.scroll, paper: lk.paper }
}

/// Empty staves under the score, the ones Cardew left for the player.
fn paint_staves(grid: &mut Grid, lk: &Look) {
    let bar = (lk.span * 2.0).round().max(4.0) as i64;
    let fg = lerp_color(lk.ink, lk.paper, 0.35);
    let mut pen = pen(grid, lk, lk.h);
    for s in 0..lk.staves {
        let top = (lk.score_h + s * STAFF_ROWS + 1) as i64;
        let shift = (hash(lk.seed, L_STAFF, s as u64, 0) % bar as u64) as i64;
        for wx in pen.cols(lk.scroll as f32, (lk.scroll + lk.w as i64) as f32) {
            let barline = (wx + shift).rem_euclid(bar) == 0;
            for i in 0..5 {
                let ch = if barline && i > 0 { '|' } else { '_' };
                pen.put(wx as f32, (top + i) as f32, ch, fg);
            }
        }
    }
}

fn life_offset(lk: &Look, chunk: i64) -> i64 {
    [-1, 0, 0, 0, 1][(hash(lk.seed, L_LIFE, chunk as u64, 0) % 5) as usize]
}

/// The lifeline: one thick horizontal thread that steps and breaks per event.
fn paint_lifeline(grid: &mut Grid, lk: &Look) {
    if lk.life == 0 {
        return;
    }
    let span = lk.span as i64;
    let mid = (lk.score_h / 2) as i64;
    let fg = lk.ink;
    let mut pen = pen(grid, lk, lk.score_h);
    for wx in pen.cols(lk.scroll as f32, (lk.scroll + lk.w as i64) as f32) {
        let chunk = wx.div_euclid(span);
        let pos = wx.rem_euclid(span);
        let broken = unit(hash(lk.seed, L_LIFE, chunk as u64, 1)) < 0.14;
        if broken && pos * 10 > span * 3 && pos * 10 < span * 6 {
            continue;
        }
        let off = life_offset(lk, chunk);
        let prev = life_offset(lk, chunk - 1);
        let ch = match (pos, off.cmp(&prev)) {
            (0, std::cmp::Ordering::Less) => '/',
            (0, std::cmp::Ordering::Greater) => '\\',
            _ => '=',
        };
        for r in 0..lk.life as i64 {
            pen.put(wx as f32, (mid + off + r) as f32, ch, fg);
        }
    }
}

fn paint_figures(grid: &mut Grid, lk: &Look) {
    let first = ((lk.scroll as f32 - lk.span * 2.5) / lk.span).floor() as i64;
    let last = ((lk.scroll + lk.w as i64) as f32 / lk.span).ceil() as i64;
    let mut pen = pen(grid, lk, lk.score_h);
    for chunk in first..=last {
        figure(&mut pen, lk, chunk, 0);
        if unit(hash(lk.seed, L_FIG, chunk as u64, 1_000)) < 0.45 {
            figure(&mut pen, lk, chunk, 1);
        }
    }
}

fn figure(pen: &mut Pen<'_>, lk: &Look, chunk: i64, slot: u64) {
    let hs = |k: u64| unit(hash(lk.seed, L_FIG + slot * 16, chunk as u64, k));
    let sh = lk.score_h as f32;
    let span = lk.span;
    let asp = lk.aspect;
    let x0 = chunk as f32 * span + hs(1) * span * 0.6;
    if x0 > (lk.scroll + lk.w as i64) as f32 + 1.0 || x0 + span * 2.5 < lk.scroll as f32 {
        return;
    }
    let y0 = 1.0 + hs(2) * (sh - 3.0).max(0.0);
    let fg = if hs(8) < 0.08 { lk.accent } else { lerp_color(lk.ink, lk.paper, hs(7) * 0.3) };
    let breath = (lk.t + hs(9) * TAU).sin();
    match (hs(0) * KINDS as f32) as u32 {
        0 => {
            let r = (1.2 + hs(3) * sh * 0.26) * (1.0 + 0.14 * breath);
            let cx = x0 + r * asp;
            let style = hs(5);
            for y in (y0 - r).floor() as i64..=(y0 + r).ceil() as i64 {
                for wx in pen.cols(cx - r * asp - 1.0, cx + r * asp + 1.0) {
                    let dxv = (wx as f32 - cx) / asp;
                    let dy = y as f32 - y0;
                    let d = (dxv * dxv + dy * dy).sqrt().max(1e-3);
                    let band = 0.5 * (dxv.abs() / asp + dy.abs()) / d;
                    if style < 0.25 && d < r {
                        pen.solid(wx, y, '@', fg);
                    } else if (d - r).abs() < band || (style > 0.7 && (d - r * 0.5).abs() < band) {
                        pen.put(wx as f32, y as f32, slope_char(-dy, dxv), fg);
                    }
                }
            }
        }
        1 => {
            let len = span * (0.9 + hs(3) * 0.9);
            let gap = if sh > 30.0 { 2.0 } else { 1.0 };
            let amp = hs(4) * sh * 0.14;
            let y0 = (1.0 + amp + hs(2) * (sh - 3.0 - 4.0 * gap - 2.0 * amp).max(0.0)).floor();
            let freq = (0.5 + hs(5) * 1.5) * TAU / len;
            let phase = hs(6) * TAU + lk.t;
            for i in 0..5u64 {
                let end = x0 + len * (0.3 + 0.7 * hs(10 + i));
                for wx in pen.cols(x0, end) {
                    let u = freq * (wx as f32 - x0) + phase;
                    let y = y0 + i as f32 * gap + amp * u.sin();
                    pen.put(wx as f32, y, slope_char(1.0 / asp, amp * freq * u.cos()), fg);
                }
                if hs(20 + i) < 0.35 {
                    let u = freq * (end - x0) + phase;
                    pen.put(end, y0 + i as f32 * gap + amp * u.sin(), 'o', fg);
                }
            }
        }
        2 => {
            let n = 2 + (hs(3) * 4.0) as u64;
            let dir = (hs(4) * 3.0).floor() - 1.0;
            let step = 2.0 + hs(5) * 4.0;
            for k in 0..n {
                let d = (hash(lk.seed, L_FIG + slot * 16, chunk as u64, 40 + k) % 10) as u32;
                let ch = char::from_digit(d, 10).unwrap_or('0');
                pen.put(x0 + k as f32 * step, y0 + k as f32 * dir, ch, fg);
            }
        }
        3 => {
            let bw = 2.0 + hs(3) * span * 0.7;
            let bh = (1.0 + hs(4) * sh * 0.22).round();
            let cut = hs(5) < 0.5;
            for y in y0 as i64..(y0 + bh) as i64 {
                for wx in pen.cols(x0, x0 + bw - 1.0) {
                    let diag = ((wx as f32 - x0) / asp - (y as f32 - y0)).abs() < 0.5;
                    if cut && diag {
                        continue;
                    }
                    pen.solid(wx, y, '#', fg);
                }
            }
        }
        4 => {
            let len = span * (0.6 + hs(3));
            let spread = 1.0 + hs(4) * sh * 0.2;
            let (ax, bx) = if hs(5) < 0.5 { (x0, x0 + len) } else { (x0 + len, x0) };
            pen.line(ax, y0, bx, y0 - spread, asp, fg);
            pen.line(ax, y0, bx, y0 + spread, asp, fg);
        }
        5 => {
            let n = 6 + (hs(3) * 22.0) as u64;
            let (rx, ry) = (span * 0.6, 1.0 + sh * 0.15 * hs(4));
            for k in 0..n {
                let g = |s: u64| {
                    let a = unit(hash(lk.seed, L_FIG + slot * 16, chunk as u64, 60 + k * 4 + s));
                    let b = unit(hash(lk.seed, L_FIG + slot * 16, chunk as u64, 62 + k * 4 + s));
                    a + b - 1.0
                };
                let ch = ['.', '.', 'o', '*'][(hash(lk.seed, L_FIG, chunk as u64, 900 + k) % 4) as usize];
                let jig = 0.6 * (lk.t * 1.7 + k as f32).sin();
                pen.put(x0 + rx + g(0) * rx + jig, y0 + g(1) * ry, ch, fg);
            }
        }
        6 => {
            let n = 3 + (hs(3) * 5.0) as u64;
            let step = span * (0.8 + hs(4)) / n as f32;
            let mut prev = (x0, y0);
            for k in 1..=n {
                let y = 1.0 + unit(hash(lk.seed, L_FIG + slot * 16, chunk as u64, 80 + k)) * (sh - 3.0).max(0.0);
                let next = (x0 + k as f32 * step, y);
                pen.line(prev.0, prev.1, next.0, next.1, asp, fg);
                pen.put(prev.0, prev.1, 'o', fg);
                prev = next;
            }
            pen.put(prev.0, prev.1, 'O', fg);
        }
        _ => {
            let len = span * (0.7 + hs(3) * 0.8);
            let m = 3 + (hs(4) * 5.0) as u32;
            let half = 1.0 + hs(5) * sh * 0.3;
            let open = 0.8 + 0.2 * breath;
            for k in 0..m {
                let v = k as f32 / (m - 1) as f32 * 2.0 - 1.0;
                pen.line(x0, y0, x0 + len, y0 + v * half * open, asp, fg);
            }
        }
    }
}

/// Left of the playhead the score has been played: glyphs crumble, ink fades into paper.
fn paint_dust(grid: &mut Grid, lk: &Look) {
    let px = lk.playhead;
    if lk.dust <= 0.0 || px == 0 {
        return;
    }
    let crumble = |(y, row): (usize, &mut Vec<Cell>)| {
        for (x, cell) in row.iter_mut().enumerate().take(px) {
            if cell.ch == ' ' {
                continue;
            }
            let dist = (px - x) as f32 / px as f32;
            let wx = (x as i64 + lk.scroll) as u64;
            let u = unit(hash(lk.seed, L_DUST, wx, y as u64));
            let p = lk.dust * dist;
            if u < p * p * 0.7 {
                *cell = Cell::with_bg(' ', lk.ink, lk.paper);
            } else if u < p {
                cell.ch = if u < p * 0.5 { '.' } else { ':' };
                cell.fg = lerp_color(cell.fg, lk.paper, 0.6 * dist);
                cell.bg = lk.paper;
            } else {
                cell.fg = lerp_color(cell.fg, lk.paper, 0.35 * p);
            }
        }
    };
    let rows = rows_mut(grid, lk);
    if lk.w * lk.h >= PARALLEL_MIN_CELLS {
        rows.par_iter_mut().enumerate().for_each(crumble);
    } else {
        rows.iter_mut().enumerate().for_each(crumble);
    }
}

fn paint_playhead(grid: &mut Grid, lk: &Look) {
    let px = lk.playhead;
    let faint = lerp_color(lk.accent, lk.paper, 0.55);
    for row in rows_mut(grid, lk) {
        let cell = &mut row[px];
        if cell.ch == ' ' {
            *cell = Cell::with_bg('|', faint, lk.paper);
        } else {
            cell.fg = lk.accent;
        }
        for cell in row.iter_mut().take((px + 4).min(lk.w)).skip(px + 1) {
            if cell.ch != ' ' && cell.bg == lk.paper {
                cell.fg = lerp_color(cell.fg, lk.accent, 0.6);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::grid_to_plain;
    use rand::{rngs::StdRng, SeedableRng};

    fn knobs() -> Vec<f32> {
        PARAMS.iter().map(|p| p.default).collect()
    }

    fn frame(w: usize, h: usize, seed: u64, time: f32, values: &[f32]) -> Grid {
        let mut grid = vec![vec![Cell::blank(); w]; h];
        let palette = crate::color::make_palette(seed);
        let mut rng = StdRng::seed_from_u64(seed);
        MODE.render(&mut ModeFrame {
            grid: &mut grid,
            width: w,
            height: h,
            seed,
            palette: &palette,
            rng: &mut rng,
            time,
            args: &[],
            param_values: Some(values),
        });
        grid
    }

    fn text(grid: &Grid) -> String {
        grid_to_plain(grid).join("\n")
    }

    #[test]
    fn treatise_seed42() {
        insta::assert_snapshot!("treatise_80x24", text(&frame(80, 24, 42, 0.0, &knobs())));
    }

    #[test]
    fn treatise_seed42_t6() {
        insta::assert_snapshot!("treatise_80x24_t6", text(&frame(80, 24, 42, 6.0, &knobs())));
    }

    #[test]
    fn deterministic_and_seed_sensitive() {
        let k = knobs();
        assert_eq!(text(&frame(90, 30, 42, 0.0, &k)), text(&frame(90, 30, 42, 0.0, &k)));
        assert_ne!(text(&frame(90, 30, 42, 0.0, &k)), text(&frame(90, 30, 7, 0.0, &k)));
    }

    #[test]
    fn time_scrolls_the_score() {
        let k = knobs();
        assert_ne!(text(&frame(90, 30, 42, 0.0, &k)), text(&frame(90, 30, 42, 4.0, &k)));
    }

    #[test]
    fn tiny_and_extreme_knobs_do_not_panic() {
        let lo: Vec<f32> = PARAMS.iter().map(|p| p.min).collect();
        let hi: Vec<f32> = PARAMS.iter().map(|p| p.max).collect();
        for (w, h) in [(1, 1), (3, 2), (20, 6), (300, 90)] {
            frame(w, h, 3, 2.5, &lo);
            frame(w, h, 3, 2.5, &hi);
        }
    }

    #[test]
    fn frame_cost() {
        let (w, h) = (200usize, 60usize);
        let k = knobs();
        let mut worst = 0.0f64;
        let start = std::time::Instant::now();
        for f in 0..100 {
            let t0 = std::time::Instant::now();
            frame(w, h, 42, f as f32 * 0.05, &k);
            worst = worst.max(t0.elapsed().as_secs_f64() * 1000.0);
        }
        let avg = start.elapsed().as_secs_f64() * 1000.0 / 100.0;
        eprintln!("treatise frame_cost 200x60: avg {:.3} ms, worst {:.3} ms", avg, worst);
        if !cfg!(debug_assertions) {
            assert!(avg < 6.0, "avg frame {:.3} ms", avg);
        }
    }
}
