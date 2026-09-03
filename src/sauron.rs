//! sauron -- the great eye: a slit-pupil lens wreathed in a fire wall that
//! flickers from fixed per-column streams; the gaze wanders, embers rise.
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

fn arc(grid: &mut Grid, cx: f32, cy: f32, rx: f32, ry: f32, a0: f32, a1: f32, color: Color) {
    let steps = (((a1 - a0).abs() * rx.max(ry)).ceil() as usize).clamp(8, 400);
    let (sx, sy) = ((cx + a0.cos() * rx) as i32, (cy + a0.sin() * ry) as i32);
    let mut pen = TreePen::new(sx, sy, color);
    for i in 1..=steps {
        let a = a0 + (a1 - a0) * i as f32 / steps as f32;
        step_toward(&mut pen, grid, (cx + a.cos() * rx) as i32, (cy + a.sin() * ry) as i32);
    }
}

fn col_rng(seed: u64, x: usize) -> StdRng {
    StdRng::seed_from_u64(seed ^ (x as u64).wrapping_mul(0x9E37_79B9) ^ 0xF1E7)
}

/// Same hue/lightness ramp as before, with the two fmod calls behind
/// range checks; anything outside the fast range falls back to `hsl_to_rgb`.
fn fire_color(heat: f32, hue_off: f64) -> Color {
    let heat = heat as f64;
    let hv = 50.0 - heat * 45.0 + hue_off;
    let l = (0.18 + heat * 0.38).min(0.58);
    let s = (0.85 + heat * 0.15).min(1.0);
    if !(0.0..360.0).contains(&hv) {
        return hsl_to_rgb(hv.rem_euclid(360.0), s, l);
    }
    let u = hv / 60.0;
    let fold = if u < 2.0 {
        u
    } else if u < 4.0 {
        u - 2.0
    } else if u < 6.0 {
        u - 4.0
    } else {
        u % 2.0
    };
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - (fold - 1.0).abs());
    let m = l - c / 2.0;
    let (r1, g1, b1) = match hv as u32 {
        0..=59 => (c, x, 0.0),
        60..=119 => (x, c, 0.0),
        120..=179 => (0.0, c, x),
        180..=239 => (0.0, x, c),
        240..=299 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    Color::Rgb {
        r: ((r1 + m) * 255.0) as u8,
        g: ((g1 + m) * 255.0) as u8,
        b: ((b1 + m) * 255.0) as u8,
    }
}

/// Fire wall: per-column flames with fixed base heights and sin flicker.
/// The columns part around the eye, flanking it with taller fire.
fn draw_fire(grid: &mut Grid, w: usize, h: usize, seed: u64, t: f32, blaze: f32, ex: f32, erx: f32, hue_off: f64) {
    let turb = param_f32("TURB", 1.0).clamp(0.0, 3.0);
    let gh = grid.len();
    let gw = if gh > 0 { grid[0].len() } else { 0 };
    let (rows, cols_n) = (h.min(gh), w.min(gw));
    if rows == 0 || cols_n == 0 {
        return;
    }
    let hh = h as f32;

    // Per-column flame stream, drawn once so the paint pass can run row-major.
    struct FireCol {
        x: usize,
        top: f32,
        fm: f32,
        shim_base: f32,
        speck: bool,
    }
    let mut cols: Vec<FireCol> = Vec::with_capacity(cols_n);
    for x in 0..cols_n {
        let mut rng = col_rng(seed, x);
        let base = rng.random_range(0.35..0.75) + blaze * 0.25;
        let ph1 = rng.random::<f32>() * std::f32::consts::TAU;
        let ph2 = rng.random::<f32>() * std::f32::consts::TAU;
        let jitter = rng.random::<f32>();
        // part the fire around the lens: dip over the eye, flare at its rims
        let dx = (x as f32 + 0.5 - ex) / erx.max(1.0);
        let dip = (1.0 - dx * dx).clamp(0.0, 1.0);
        let parted = 1.0 - dip * 0.55 + (1.0 - ((dx.abs() - 1.0).abs())).clamp(0.0, 1.0) * 0.35;
        let flick = (t * 2.2 * turb + ph1).sin() * 0.10 + (t * 5.1 * turb + ph2).sin() * 0.05;
        let flame_h = (base * parted + flick) * hh;
        if flame_h <= 0.0 {
            continue;
        }
        cols.push(FireCol {
            x,
            top: hh - flame_h,
            fm: flame_h.max(1.0),
            shim_base: t * 7.0 * turb + ph2,
            speck: (jitter + (t * 3.0 + ph1).sin()) % 1.0 > 0.45,
        });
    }

    // The shimmer only moves heat by +-0.08, so the flame core saturates to a
    // fixed glyph and color, and a dark cell without a speck is already blank.
    let core_col = fire_color(1.0, hue_off);

    for y in 0..rows {
        let gy = h as i32 - 1 - y as i32;
        let gyf = gy as f32;
        let yshim = y as f32 * 0.7;
        let row = &mut grid[y];
        for c in cols.iter() {
            let heat = 1.0 - (gyf - c.top) / c.fm;
            if heat <= 0.0 {
                continue;
            }
            if heat >= 1.2 {
                row[c.x] = Cell::new('▓', core_col);
                continue;
            }
            if heat <= 0.13 && !c.speck {
                continue;
            }
            let shimmer = (c.shim_base + yshim).sin() * 0.08;
            let ht = (heat + shimmer).clamp(0.0, 1.0);
            let ch = if ht > 0.72 {
                '▓'
            } else if ht > 0.45 {
                '▒'
            } else if ht > 0.22 {
                '░'
            } else if c.speck {
                '·'
            } else {
                continue;
            };
            row[c.x] = Cell::new(ch, fire_color(ht, hue_off));
        }
    }
}

/// Rising embers and rare flares around the eye.
fn draw_embers(grid: &mut Grid, w: usize, h: usize, seed: u64, t: f32, count: usize, hue_off: f64) {
    let mut rng = StdRng::seed_from_u64(seed ^ 0xE3B0);
    for _ in 0..count {
        let x0 = rng.random::<f32>() * w as f32;
        let ph = rng.random::<f32>();
        let rise = rng.random_range(2.5..7.0);
        let sway = rng.random_range(1.5..4.0);
        let sp = rng.random_range(0.7..1.4);
        let flare = rng.random::<f32>() < 0.15;
        let prog = ((t * rise * sp + ph * 60.0) % (h as f32 + 8.0)) / (h as f32 + 8.0);
        let x = x0 + (t * 0.9 + ph * 6.0).sin() * sway;
        let y = h as f32 - 1.0 - prog * (h as f32 + 6.0);
        let fade = 1.0 - prog;
        if fade < 0.05 {
            continue;
        }
        let ch = if flare && fade > 0.5 {
            '✶'
        } else if fade > 0.6 {
            '•'
        } else {
            '·'
        };
        set(grid, x as i32, y as i32, ch, fire_color((0.55 + fade * 0.4).min(1.0), hue_off));
    }
}

/// The eye: lens lids, iris gradient, slit pupil with wandering gaze.
fn draw_eye(grid: &mut Grid, cx: f32, cy: f32, rx: f32, ry: f32, t: f32, gaze: f32, slit: usize, iris_r: f32, hue_off: f64) {
    let gaze_x = ((t * 0.55).sin() * 0.6 + (t * 1.7).sin() * 0.25 + (t * 0.23).cos() * 0.5).clamp(-1.0, 1.0);
    let px = cx + gaze_x * gaze * rx * 0.5;
    // lids
    arc(grid, cx, cy, rx, ry, std::f32::consts::PI, std::f32::consts::TAU, hsl_to_rgb((12.0 + hue_off).rem_euclid(360.0), 0.9, 0.5));
    arc(grid, cx, cy, rx, ry, 0.0, std::f32::consts::PI, hsl_to_rgb((8.0 + hue_off).rem_euclid(360.0), 0.9, 0.42));
    set(grid, (cx - rx) as i32, cy as i32, '◆', hsl_to_rgb((5.0 + hue_off).rem_euclid(360.0), 0.95, 0.55));
    set(grid, (cx + rx) as i32, cy as i32, '◆', hsl_to_rgb((5.0 + hue_off).rem_euclid(360.0), 0.95, 0.55));
    // iris + lens fill
    let iris_f = iris_r.clamp(0.15, 0.95);
    let pupil = px;
    for y in (cy - ry) as i32..=(cy + ry) as i32 {
        for x in (cx - rx) as i32..=(cx + rx) as i32 {
            let fx = (x as f32 - cx) / rx;
            let fy = (y as f32 - cy) / ry;
            let d = fx * fx + fy * fy;
            if d > 1.0 {
                continue;
            }
            let dxp = (x as f32 - pupil) / (rx * iris_f);
            let dyp = (y as f32 - cy) / (ry * iris_f);
            // vertical slit: tall in y, knife-thin in x
            let sw = (slit as f32 * 0.18).clamp(0.08, 0.34);
            let pd = (dxp * dxp) / (sw * sw) + dyp * dyp * 0.8;
            let flick = (t * 9.0 + (x * 7 + y * 13) as f32 * 0.35).sin() * 0.05;
            let ch = if pd < 1.0 {
                // the slit pupil: near-black core with a live ember center
                if (x as f32 - pupil).abs() < (slit as f32 * 0.6).max(0.9) && (y as f32 - cy).abs() < ry * 0.45 {
                    '◉'
                } else {
                    '┃'
                }
            } else if dxp * dxp + dyp * dyp < 1.0 {
                // iris: heat grows toward the pupil, streaked
                let streak = ((x + y * 2).rem_euclid(5) == 0) as usize;
                if streak == 1 && flick > 0.0 {
                    '╱'
                } else if (x + y) % 7 == 0 {
                    '▒'
                } else {
                    '░'
                }
            } else {
                ' '
            };
            let heat = if pd < 1.0 {
                0.05
            } else {
                (1.2 - (dxp * dxp + dyp * dyp)).clamp(0.0, 1.0) * 0.9 + flick
            };
            let c = if pd < 1.0 {
                hsl_to_rgb((hue_off + 18.0).rem_euclid(360.0), 0.95, if ch == '◉' { 0.5 } else { 0.06 })
            } else {
                fire_color(heat.clamp(0.0, 1.0), hue_off)
            };
            set(grid, x, y, ch, c);
        }
    }
    // radiating glare ticks off the pupil
    for k in 0..10 {
        let a = k as f32 / 10.0 * std::f32::consts::TAU + t * 0.15;
        let gx = pupil + a.cos() * rx * iris_f * 1.15;
        let gy = cy + a.sin() * ry * iris_f * 1.15;
        let ch = if k % 2 == 0 { '╲' } else { '╱' };
        set(grid, gx as i32, gy as i32, ch, fire_color(1.0, hue_off));
    }
}

/// Smoke cap and heat shimmer above everything.
fn draw_smoke(grid: &mut Grid, w: usize, h: usize, seed: u64, t: f32, hue_off: f64) {
    let smoke = hsl_to_rgb((30.0 + hue_off).rem_euclid(360.0), 0.15, 0.10);
    for y in 0..(h / 6).max(1) {
        for x in 0..w {
            let v = (x as f32 * 0.7 + y as f32 * 3.1 + t * 1.4).sin();
            if v > 0.55 - y as f32 * 0.1 {
                set(grid, x as i32, y as i32, if v > 0.8 { '▒' } else { '░' }, smoke);
            }
        }
    }
}

pub fn draw_sauron(grid: &mut Grid, width: usize, height: usize, seed: u64, palette: &[Color; 5], t: f32) {
    let blaze = param_f32("BLAZE", 1.0).clamp(0.0, 2.0);
    let gaze = param_f32("GAZE", 0.8).clamp(0.0, 1.0);
    let slit = param_f32("SLIT", 2.0).clamp(1.0, 5.0) as usize;
    let iris_r = param_f32("IRIS", 0.7).clamp(0.15, 0.95);
    let embers = param_f32("EMBERS", 26.0).clamp(0.0, 90.0) as usize;
    let hue_off = if let Color::Rgb { r, g, .. } = palette[4] {
        ((r as f64 + g as f64 * 0.5) % 20.0) - 10.0
    } else {
        0.0
    };

    let cx = width as f32 / 2.0 - 0.5;
    let cy = height as f32 / 2.0 - 0.5;
    let rx = (width as f32 * 0.30).max(8.0);
    let ry = (height as f32 * 0.13).max(4.0);

    measure_layer("sauron", "fire", || draw_fire(grid, width, height, seed, t, blaze, cx, rx, hue_off));
    measure_layer("sauron", "embers", || draw_embers(grid, width, height, seed, t, embers, hue_off));
    measure_layer("sauron", "eye", || draw_eye(grid, cx, cy, rx, ry, t, gaze, slit, iris_r, hue_off));
    measure_layer("sauron", "smoke", || draw_smoke(grid, width, height, seed, t, hue_off));
}

pub(crate) fn cli_sauron(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
    let _ = (rng, term_w, term_h, args, mode, theme_name);
    draw_sauron(&mut grid, width, height, seed, &palette, t_anim);
    (grid, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(w: usize, h: usize, seed: u64, t: f32) -> String {
        let mut g = vec![vec![Cell::blank(); w]; h];
        let p = crate::color::make_palette(seed);
        draw_sauron(&mut g, w, h, seed, &p, t);
        g.iter()
            .map(|row| row.iter().map(|c| c.ch).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn snapshot_sauron_small() {
        insta::assert_snapshot!("sauron_80x24", run(80, 24, 42, 0.0));
    }

    #[test]
    fn snapshot_sauron_wide() {
        insta::assert_snapshot!("sauron_100x36", run(100, 36, 42, 0.0));
    }

    #[test]
    fn deterministic_and_seed_sensitive() {
        assert_eq!(run(90, 30, 42, 0.0), run(90, 30, 42, 0.0));
        assert_ne!(run(90, 30, 42, 0.0), run(90, 30, 7, 0.0));
    }

    #[test]
    fn t_moves_fire_and_gaze() {
        assert_ne!(run(90, 30, 42, 0.0), run(90, 30, 42, 2.0));
        assert_ne!(run(90, 30, 42, 2.0), run(90, 30, 42, 4.0));
    }

    #[test]
    fn eye_present_over_fire() {
        let s = run(90, 30, 42, 0.0);
        assert!(s.contains('◉'), "the eye needs its ember pupil");
        assert!(s.contains('┃'), "the eye needs its slit pupil");
    }
}
