use crossterm::style::Color;
use rand::RngExt;
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::io::{self, IsTerminal, Read as _};

use crate::automata::*;
use crate::biomes::*;
use crate::color::*;
use crate::content::*;
use crate::fills::*;
use crate::layout::*;
use crate::markdown::*;
use crate::mondrian::*;
use crate::render::*;
use crate::scene::*;
use crate::sprites::*;
use crate::tree_draw::*;
use crate::types::*;
use crate::walker::*;
use crate::avant::*;
use crate::cli::*;
use crate::gridio::*;
use crate::ink::*;
use crate::modes_creatures::*;
use crate::modes_geo::*;
use crate::modes_sky::*;
use crate::modes_tree::*;
use crate::morph::*;
use crate::opts::*;
use crate::pp::*;
use crate::registry::*;
use crate::warps::*;
use crate::automata; use crate::avant; use crate::biomes; use crate::borders; use crate::color; use crate::content; use crate::fills; use crate::layout; use crate::markdown; use crate::mondrian; use crate::render; use crate::scene; use crate::sprites; use crate::tree_draw; use crate::types; use crate::walker;


/// Dispatch arm for mode(s): eyes (moved verbatim from run()).
pub(crate) fn cli_eyes(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        // eyes [density] [mutation] -- maximalist field of varied staring forms
        let density: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(42);
        let density = density.clamp(8, 120);
        let mutation: usize = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(78);
        let mutation = mutation.clamp(0, 140);

        let bg = darken(palette[0], 10);
        let vein = darken(palette[2], 42);
        let lid_base = lighten(palette[1], 18);
        let sclera = lighten(palette[4], 2);
        let iris_base = lighten(palette[3], 28);
        let pupil = darken(palette[0], 4);
        let glare = lighten(palette[4], 25);

        for y in 0..height {
            for x in 0..width {
                let n = (x * 13 + y * 19 + seed as usize * 7) % 67;
                let ch = match n {
                    0 => '·',
                    1 => '∙',
                    2 => '°',
                    3 if mutation > 65 => '╎',
                    _ => ' ',
                };
                grid[y][x] = if ch == ' ' {
                    Cell::new(' ', bg)
                } else {
                    Cell::new(ch, vein)
                };
            }
        }

        let put = |grid: &mut Grid, x: i32, y: i32, ch: char, fg: Color| {
            if x >= 0 && y >= 0 && (x as usize) < width && (y as usize) < height {
                grid[y as usize][x as usize] = Cell::new(ch, fg);
            }
        };
        let line_char = |dx: i32, dy: i32| {
            if dx.abs() > dy.abs() * 2 {
                '─'
            } else if dy.abs() > dx.abs() * 2 {
                '│'
            } else if dx.signum() == dy.signum() {
                '╲'
            } else {
                '╱'
            }
        };
        let draw_line =
            |grid: &mut Grid, mut x0: i32, mut y0: i32, x1: i32, y1: i32, ch: char, fg: Color| {
                let dx = (x1 - x0).abs();
                let sx = if x0 < x1 { 1 } else { -1 };
                let dy = -(y1 - y0).abs();
                let sy = if y0 < y1 { 1 } else { -1 };
                let mut err = dx + dy;
                loop {
                    put(grid, x0, y0, ch, fg);
                    if x0 == x1 && y0 == y1 {
                        break;
                    }
                    let e2 = 2 * err;
                    if e2 >= dy {
                        err += dy;
                        x0 += sx;
                    }
                    if e2 <= dx {
                        err += dx;
                        y0 += sy;
                    }
                }
            };
        let draw_eye = |grid: &mut Grid,
                        cx: i32,
                        cy: i32,
                        rx: i32,
                        ry: i32,
                        style: usize,
                        gaze_x: i32,
                        gaze_y: i32,
                        lid_color: Color,
                        iris_color: Color,
                        rng: &mut StdRng| {
            let rx = rx.max(2);
            let ry = ry.max(1);
            let iris_rx = (rx / 3).max(1);
            let iris_ry = (ry / 2).max(1);
            let pupil_rx = (iris_rx / 2).max(1);
            let pupil_ry = if style % 4 == 0 {
                iris_ry.max(2)
            } else {
                1.max(iris_ry / 2)
            };
            let blink_cut = if style % 9 == 3 { 0.28 } else { 1.0 };

            for dy in -ry - 1..=ry + 1 {
                for dx in -rx - 1..=rx + 1 {
                    let nx = dx as f32 / rx as f32;
                    let ny = dy as f32 / ry as f32;
                    let metric = nx * nx + ny * ny;
                    if metric > 1.28 || ny.abs() > blink_cut {
                        continue;
                    }
                    let x = cx + dx;
                    let y = cy + dy;
                    let edge = (metric - 1.0).abs();
                    if edge < 0.26 || dy.abs() == ry {
                        let ch = if dy < -ry / 3 {
                            if dx < -rx / 2 {
                                '╭'
                            } else if dx > rx / 2 {
                                '╮'
                            } else {
                                '─'
                            }
                        } else if dy > ry / 3 {
                            if dx < -rx / 2 {
                                '╰'
                            } else if dx > rx / 2 {
                                '╯'
                            } else {
                                '─'
                            }
                        } else if dx < 0 {
                            '╱'
                        } else if dx > 0 {
                            '╲'
                        } else {
                            '│'
                        };
                        put(grid, x, y, ch, lid_color);
                        continue;
                    }

                    let idy = dy - gaze_y;
                    let idx = dx - gaze_x;
                    let im = (idx as f32 / iris_rx as f32).powi(2)
                        + (idy as f32 / iris_ry as f32).powi(2);
                    if im <= 1.0 {
                        let pm = (idx as f32 / pupil_rx as f32).powi(2)
                            + (idy as f32 / pupil_ry as f32).powi(2);
                        if pm <= 1.0 {
                            let ch = match style % 8 {
                                0 => '┃',
                                1 => '●',
                                2 => '█',
                                3 => '◆',
                                4 => '◉',
                                5 => '◐',
                                6 => '◍',
                                _ => '◎',
                            };
                            put(grid, x, y, ch, pupil);
                        } else {
                            let ch = match (style
                                + dx.unsigned_abs() as usize
                                + dy.unsigned_abs() as usize)
                                % 7
                            {
                                0 => '◌',
                                1 => '○',
                                2 => '◍',
                                3 => '◐',
                                4 => '◑',
                                5 => '·',
                                _ => '•',
                            };
                            put(grid, x, y, ch, iris_color);
                        }
                    } else {
                        let ch = match (style
                            + dx.unsigned_abs() as usize * 2
                            + dy.unsigned_abs() as usize)
                            % 9
                        {
                            0 => '·',
                            1 => '∙',
                            2 if style % 5 == 0 => '╎',
                            3 if style % 7 == 0 => '◇',
                            _ => ' ',
                        };
                        put(
                            grid,
                            x,
                            y,
                            ch,
                            if ch == ' ' {
                                sclera
                            } else {
                                darken(sclera, 35)
                            },
                        );
                    }
                }
            }

            put(
                grid,
                cx + gaze_x - iris_rx / 2,
                cy + gaze_y - iris_ry,
                '˙',
                glare,
            );
            if style % 3 == 0 {
                put(
                    grid,
                    cx + gaze_x + iris_rx / 2,
                    cy + gaze_y + iris_ry,
                    '·',
                    glare,
                );
            }
            if style % 5 == 0 {
                put(grid, cx - rx, cy, '<', lid_color);
                put(grid, cx + rx, cy, '>', lid_color);
            }

            if style % 2 == 0 {
                let lash_count = (3 + mutation / 28).min(8);
                for i in 0..lash_count {
                    let t = if lash_count <= 1 {
                        0.5
                    } else {
                        i as f32 / (lash_count - 1) as f32
                    };
                    let lx = cx - rx + (t * rx as f32 * 2.0).round() as i32;
                    let ly = cy - ry + ((t - 0.5).abs() * 2.0).round() as i32;
                    let lean = rng.random_range(-2..=2);
                    let len = rng.random_range(1..=3 + (mutation / 50) as i32);
                    let tx = lx + lean;
                    let ty = ly - len;
                    draw_line(
                        grid,
                        lx,
                        ly,
                        tx,
                        ty,
                        line_char(tx - lx, ty - ly),
                        darken(lid_color, 8),
                    );
                }
            }
            if style % 6 == 1 {
                for side in [-1, 1] {
                    for k in 1..=3 {
                        put(
                            grid,
                            cx + side * (rx + k),
                            cy + (k % 2) - 1,
                            '·',
                            darken(iris_color, 20),
                        );
                    }
                }
            }
        };

        let mut row_y = 2i32;
        while row_y < height as i32 {
            let mut x = rng.random_range(-6..=3);
            while x < width as i32 + 6 {
                let rx = rng.random_range(3..=(7 + density / 22) as i32).min(12);
                let ry = rng.random_range(1..=3 + (mutation / 70) as i32).min(5);
                let style = rng.random_range(0..18usize);
                let gaze_x = rng.random_range(-(rx / 4).max(1)..=(rx / 4).max(1));
                let gaze_y = rng.random_range(-(ry / 3).max(0)..=(ry / 3).max(0));
                let lid = shift_hue(lid_base, rng.random_range(-45..=55) as f64);
                let iris = shift_hue(iris_base, rng.random_range(-120..=120) as f64);
                draw_eye(
                    &mut grid, x, row_y, rx, ry, style, gaze_x, gaze_y, lid, iris, &mut rng,
                );
                x += rng
                    .random_range(7i32..=13i32)
                    .saturating_sub(density as i32 / 24);
            }
            row_y += rng.random_range(3..=5);
        }

        let large_count = (3 + mutation / 25).min(8);
        for i in 0..large_count {
            let rx = rng.random_range(7..=(width / 4).max(9) as i32).min(22);
            let ry = rng.random_range(3..=(height / 4).max(4) as i32).min(8);
            let cx = rng.random_range(-(rx / 2)..=(width as i32 + rx / 2));
            let cy = rng.random_range(1..height as i32);
            let style = i + rng.random_range(0..24usize);
            let gaze_x = rng.random_range(-(rx / 3)..=(rx / 3));
            let gaze_y = rng.random_range(-(ry / 3)..=(ry / 3));
            let lid = shift_hue(lighten(lid_base, 14), rng.random_range(-80..=80) as f64);
            let iris = shift_hue(lighten(iris_base, 18), rng.random_range(-160..=160) as f64);
            draw_eye(
                &mut grid, cx, cy, rx, ry, style, gaze_x, gaze_y, lid, iris, &mut rng,
            );
        }

        let sigils = ['◉', '◎', '◌', '◍', '◐', '◑', '●', '•', '˙'];
        for _ in 0..density {
            let x = rng.random_range(0..width) as i32;
            let y = rng.random_range(0..height) as i32;
            if rng.random::<f32>() < mutation as f32 / 170.0 {
                put(
                    &mut grid,
                    x,
                    y,
                    sigils[rng.random_range(0..sigils.len())],
                    shift_hue(iris_base, rng.random_range(-180..=180) as f64),
                );
            }
        }
    (grid, false)
}

/// Dispatch arm for mode(s): eyes2 (moved verbatim from run()).
pub(crate) fn cli_eyes2(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        // eyes2 [count] [pupil-visible] -- anatomical eyes all staring at a focal lure
        let eye_count: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(6);
        let eye_count = eye_count.clamp(3, 20);
        let pupil_visible: usize = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(80);
        let pupil_visible = pupil_visible.clamp(50, 100);

        let bg = darken(palette[0], 12);
        let lid_base = lighten(palette[1], 12);
        let sclera = lighten(palette[4], 4);
        let iris_base = lighten(palette[3], 18);
        let pupil = darken(palette[0], 2);
        let shadow = darken(palette[2], 48);
        let highlight = lighten(palette[4], 26);

        for y in 0..height {
            for x in 0..width {
                let n = (x * 17 + y * 29 + seed as usize * 3) % 91;
                let ch = match n {
                    0 => '·',
                    1 if (x + y) % 3 == 0 => '∙',
                    2 if (x + seed as usize) % 11 == 0 => '°',
                    _ => ' ',
                };
                grid[y][x] = if ch == ' ' {
                    Cell::new(' ', bg)
                } else {
                    Cell::new(ch, shadow)
                };
            }
        }

        let put = |grid: &mut Grid, x: i32, y: i32, ch: char, fg: Color| {
            if x >= 0 && y >= 0 && (x as usize) < width && (y as usize) < height {
                grid[y as usize][x as usize] = Cell::new(ch, fg);
            }
        };
        let draw_line =
            |grid: &mut Grid, mut x0: i32, mut y0: i32, x1: i32, y1: i32, ch: char, fg: Color| {
                let dx = (x1 - x0).abs();
                let sx = if x0 < x1 { 1 } else { -1 };
                let dy = -(y1 - y0).abs();
                let sy = if y0 < y1 { 1 } else { -1 };
                let mut err = dx + dy;
                loop {
                    put(grid, x0, y0, ch, fg);
                    if x0 == x1 && y0 == y1 {
                        break;
                    }
                    let e2 = 2 * err;
                    if e2 >= dy {
                        err += dy;
                        x0 += sx;
                    }
                    if e2 <= dx {
                        err += dx;
                        y0 += sy;
                    }
                }
            };
        let draw_eye = |grid: &mut Grid,
                        cx: i32,
                        cy: i32,
                        rx: i32,
                        ry: i32,
                        open_pct: usize,
                        gaze_x: i32,
                        gaze_y: i32,
                        slant: f32,
                        style: usize,
                        lid_color: Color,
                        iris_color: Color| {
            let rx = rx.max(5);
            let ry = ry.max(2);
            let open = (open_pct as f32 / 100.0).clamp(0.50, 1.0);
            let iris_rx = ((rx as f32 * 0.27).round() as i32).max(2);
            let iris_ry = ((ry as f32 * 0.72).round() as i32).max(2);
            let pupil_rx = ((iris_rx as f32 * 0.42).round() as i32).max(1);
            let pupil_ry = if style % 4 == 0 {
                iris_ry.max(2)
            } else {
                ((iris_ry as f32 * 0.62).round() as i32).max(1)
            };

            for dx in -rx - 2..=rx + 2 {
                let nx = dx as f32 / rx as f32;
                if nx.abs() > 1.06 {
                    continue;
                }
                let curve = (1.0 - nx.abs().powf(1.72)).max(0.0).powf(0.56);
                let top = (-ry as f32 * open * curve - nx * slant).round() as i32;
                let bottom = (ry as f32 * open * 0.84 * curve + nx * slant * 0.38).round() as i32;
                if bottom < top {
                    continue;
                }
                for dy in top..=bottom {
                    let x = cx + dx;
                    let y = cy + dy;
                    let on_top = dy == top;
                    let on_bottom = dy == bottom;
                    if on_top || on_bottom {
                        let edge = dx.abs() as f32 / rx as f32;
                        let ch = if dx <= -rx {
                            if dx < 0 { '<' } else { '>' }
                        } else if dx >= rx {
                            '>'
                        } else if edge < 0.82 {
                            '─'
                        } else if on_top {
                            if dx < 0 { '╱' } else { '╲' }
                        } else if dx < 0 {
                            '╲'
                        } else {
                            '╱'
                        };
                        put(grid, x, y, ch, lid_color);
                        continue;
                    }

                    let idx = dx - gaze_x;
                    let idy = dy - gaze_y;
                    let im = (idx as f32 / iris_rx as f32).powi(2)
                        + (idy as f32 / iris_ry as f32).powi(2);
                    if im <= 1.08 {
                        let pm = (idx as f32 / pupil_rx as f32).powi(2)
                            + (idy as f32 / pupil_ry as f32).powi(2);
                        if pm <= 0.56 {
                            let ch = match style % 6 {
                                0 => '│',
                                1 => '●',
                                2 => '◐',
                                3 => '◑',
                                4 => '◉',
                                _ => '┃',
                            };
                            put(grid, x, y, ch, pupil);
                        } else if im > 0.72 {
                            let ch = if (idx + idy + style as i32) % 3 == 0 {
                                '◌'
                            } else {
                                '○'
                            };
                            put(grid, x, y, ch, darken(iris_color, 8));
                        } else {
                            let ch = match (idx.abs() + idy.abs() + style as i32) % 7 {
                                0 => '╎',
                                1 | 2 | 3 => '·',
                                4 => '∙',
                                _ => '˙',
                            };
                            put(grid, x, y, ch, iris_color);
                        }
                    } else if (dx * 5 + dy * 7 + style as i32) % 37 == 0 {
                        put(grid, x, y, '·', darken(sclera, 45));
                    } else if (dx * 3 + dy * 11 + style as i32) % 11 == 0 {
                        put(grid, x, y, '·', darken(sclera, 24));
                    } else {
                        put(grid, x, y, ' ', sclera);
                    }
                }
            }

            put(
                grid,
                cx + gaze_x - iris_rx / 2,
                cy + gaze_y - iris_ry / 2,
                '˙',
                highlight,
            );
            if style % 5 == 0 {
                let lid_y = cy + gaze_y - iris_ry / 2;
                draw_line(
                    grid,
                    cx - iris_rx,
                    lid_y,
                    cx + iris_rx,
                    lid_y,
                    '─',
                    darken(lid_color, 2),
                );
            }
        };

        let focus_x = (width as i32 / 2
            + rng.random_range(-(width as i32 / 12)..=(width as i32 / 12)))
        .clamp(6, width as i32 - 7);
        let focus_y = ((height as f32 * 0.72).round() as i32
            + rng.random_range(-(height as i32 / 18)..=(height as i32 / 18)))
        .clamp(8, height as i32 - 5);
        let lure_color = shift_hue(lighten(iris_base, 30), 55.0);
        for dy in -3i32..=3i32 {
            for dx in -6i32..=6i32 {
                let metric = (dx as f32 / 6.0).powi(2) + (dy as f32 / 3.0).powi(2);
                if metric <= 1.0 && (dx.abs() + dy.abs()) % 2 == 0 {
                    put(
                        &mut grid,
                        focus_x + dx,
                        focus_y + dy,
                        '·',
                        darken(lure_color, 22),
                    );
                }
            }
        }
        draw_line(
            &mut grid,
            focus_x,
            focus_y - 3,
            focus_x,
            focus_y + 2,
            '│',
            darken(lure_color, 8),
        );
        draw_line(
            &mut grid,
            focus_x - 3,
            focus_y,
            focus_x + 3,
            focus_y,
            '─',
            darken(lure_color, 8),
        );
        put(&mut grid, focus_x, focus_y, '◆', lighten(lure_color, 12));
        put(&mut grid, focus_x, focus_y - 2, '◇', lighten(highlight, 4));
        put(
            &mut grid,
            focus_x - 2,
            focus_y + 2,
            '╲',
            darken(lure_color, 2),
        );
        put(
            &mut grid,
            focus_x + 2,
            focus_y + 2,
            '╱',
            darken(lure_color, 2),
        );

        let gaze_for = |ex: i32, ey: i32, rx: i32, ry: i32| {
            let dx = (focus_x - ex) as f32;
            let dy = (focus_y - ey) as f32;
            let dist = (dx * dx + dy * dy).sqrt().max(1.0);
            let gx = ((dx / dist) * (rx as f32 * 0.24)).round() as i32;
            let gy = ((dy / dist) * (ry as f32 * 0.42)).round() as i32;
            let gx = gx.clamp(-(rx / 4).max(1), (rx / 4).max(1));
            let gy = gy.clamp(-(ry / 2).max(1), (ry / 2).max(1));
            let slant = (dx / dist * 1.35).clamp(-1.2, 1.2);
            (gx, gy, slant)
        };
        let draw_dotted_line =
            |grid: &mut Grid, mut x0: i32, mut y0: i32, x1: i32, y1: i32, fg: Color| {
                let dx = (x1 - x0).abs();
                let sx = if x0 < x1 { 1 } else { -1 };
                let dy = -(y1 - y0).abs();
                let sy = if y0 < y1 { 1 } else { -1 };
                let mut err = dx + dy;
                let mut step = 0usize;
                loop {
                    if step % 4 == 0
                        && x0 >= 0
                        && y0 >= 0
                        && (x0 as usize) < width
                        && (y0 as usize) < height
                        && grid[y0 as usize][x0 as usize].ch == ' '
                    {
                        put(grid, x0, y0, '·', fg);
                    }
                    if x0 == x1 && y0 == y1 {
                        break;
                    }
                    let e2 = 2 * err;
                    if e2 >= dy {
                        err += dy;
                        x0 += sx;
                    }
                    if e2 <= dx {
                        err += dx;
                        y0 += sy;
                    }
                    step += 1;
                }
            };

        let mut eye_specs: Vec<(i32, i32, i32, i32, usize, usize)> = Vec::new();
        for i in 0..eye_count {
            let t = (i as f32 + 0.5) / eye_count as f32;
            let angle =
                std::f32::consts::PI + t * std::f32::consts::PI + rng.random_range(-0.20..0.20);
            let arc_rx = width as f32 * rng.random_range(0.34..0.52);
            let arc_ry = height as f32 * rng.random_range(0.28..0.52);
            let mut ex = (focus_x as f32 + angle.cos() * arc_rx).round() as i32;
            let mut ey = (focus_y as f32 + angle.sin() * arc_ry).round() as i32;
            let mut rx = rng.random_range(7..=13);
            let mut ry = rng.random_range(3..=6);
            if i == eye_count / 2 {
                rx = ((width as f32 * 0.18).round() as i32).clamp(12, 20);
                ry = ((height as f32 * 0.20).round() as i32).clamp(4, 7);
                ex = (width as i32 / 2 + rng.random_range(-4..=4))
                    .clamp(rx + 2, width as i32 - rx - 3);
                ey = (focus_y - (height as i32 / 3).max(6) + rng.random_range(-2..=2))
                    .clamp(ry + 3, height as i32 - ry - 4);
            }
            ex = ex.clamp(-rx / 2, width as i32 + rx / 2);
            ey = ey.clamp(ry + 2, height as i32 - ry - 3);
            let visible =
                (pupil_visible as i32 + rng.random_range(-8..=14)).clamp(50, 100) as usize;
            eye_specs.push((ex, ey, rx, ry, visible, i));
        }

        for &(ex, ey, rx, ry, _, i) in &eye_specs {
            let (gx, gy, _) = gaze_for(ex, ey, rx, ry);
            let iris_x = ex + gx;
            let iris_y = ey + gy;
            draw_dotted_line(
                &mut grid,
                iris_x,
                iris_y,
                focus_x,
                focus_y,
                darken(shift_hue(iris_base, i as f64 * 23.0), 35),
            );
        }
        for &(ex, ey, rx, ry, visible, i) in &eye_specs {
            let (gaze_x, gaze_y, slant) = gaze_for(ex, ey, rx, ry);
            let lid = shift_hue(lid_base, rng.random_range(-34..=42) as f64);
            let iris = shift_hue(iris_base, rng.random_range(-120..=120) as f64);
            draw_eye(
                &mut grid, ex, ey, rx, ry, visible, gaze_x, gaze_y, slant, i, lid, iris,
            );
        }
    (grid, false)
}

/// Dispatch arm for mode(s): metro (moved verbatim from run()).
pub(crate) fn cli_metro(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        // metro [lines] -- transit map: orthogonal routes with rounded bends,
        // stations along each run, interchange rings where routes cross
        let line_count: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(5);
        let line_count = line_count.clamp(2, 9);

        let grid_dot = darken(palette[1], 82);
        for y in (0..height).step_by(3) {
            for x in (0..width).step_by(6) {
                grid[y][x] = Cell::new('·', grid_dot);
            }
        }

        // per-cell line ids by orientation (id = line index + 1); an interchange
        // is where two different lines meet in different orientations, so
        // parallel overlapping runs don't smear into rings and a line never
        // rings against its own bends
        let mut occ_h = vec![vec![0u8; width]; height];
        let mut occ_v = vec![vec![0u8; width]; height];
        macro_rules! rail {
            ($x:expr, $y:expr, $ch:expr, $col:expr, $id:expr, $dirbits:expr) => {{
                let sx = $x;
                let sy = $y;
                if sx >= 0 && sy >= 0 && (sx as usize) < width && (sy as usize) < height {
                    grid[sy as usize][sx as usize] = Cell::new($ch, $col);
                    // first writer keeps the cell: a later corner over an earlier
                    // straight run still registers as two lines meeting
                    if $dirbits & 1 != 0 && occ_h[sy as usize][sx as usize] == 0 {
                        occ_h[sy as usize][sx as usize] = $id;
                    }
                    if $dirbits & 2 != 0 && occ_v[sy as usize][sx as usize] == 0 {
                        occ_v[sy as usize][sx as usize] = $id;
                    }
                }
            }};
        }

        // line 0 is the circle line; the rest alternate between top-to-bottom
        // and left-to-right so the map fills both axes
        for li in 0..line_count {
            let id = (li + 1) as u8;
            let base = [palette[1], palette[2], palette[3]][li % 3];
            let col = shift_hue(lighten(base, 8), li as f64 * 37.0);

            if li == 0 && line_count >= 3 {
                let x0 = rng.random_range(width as i32 / 8..(width as i32 / 3).max(width as i32 / 8 + 1));
                let x1 = rng.random_range(width as i32 * 3 / 5..(width as i32 * 7 / 8).max(width as i32 * 3 / 5 + 1));
                let y0 = rng.random_range(2..(height as i32 / 3).max(3));
                let y1 = rng.random_range(height as i32 * 3 / 5..(height as i32 - 2).max(height as i32 * 3 / 5 + 1));
                let mut until_station: i32 = rng.random_range(3..6);
                for x in x0 + 1..x1 {
                    for yy in [y0, y1] {
                        until_station -= 1;
                        if until_station <= 0 {
                            rail!(x, yy, '○', lighten(col, 30), id, 1);
                            until_station = rng.random_range(5..10);
                        } else {
                            rail!(x, yy, '─', col, id, 1);
                        }
                    }
                }
                for y in y0 + 1..y1 {
                    for xx in [x0, x1] {
                        if rng.random_range(0..7) == 0 {
                            rail!(xx, y, '○', lighten(col, 30), id, 2);
                        } else {
                            rail!(xx, y, '│', col, id, 2);
                        }
                    }
                }
                rail!(x0, y0, '╭', col, id, 3);
                rail!(x1, y0, '╮', col, id, 3);
                rail!(x0, y1, '╰', col, id, 3);
                rail!(x1, y1, '╯', col, id, 3);
                continue;
            }

            if li % 2 == 0 {
                // vertical-major: top to bottom with horizontal jogs
                let mut x: i32 = rng.random_range(3..(width as i32 - 3).max(4));
                let mut y: i32 = 0;
                let mut until_station: i32 = rng.random_range(3..7);
                while y < height as i32 {
                    let run: i32 = rng.random_range(4..9);
                    for _ in 0..run {
                        if y >= height as i32 {
                            break;
                        }
                        until_station -= 1;
                        if until_station <= 0 {
                            rail!(x, y, '○', lighten(col, 30), id, 2);
                            until_station = rng.random_range(4..9);
                        } else {
                            rail!(x, y, '│', col, id, 2);
                        }
                        y += 1;
                    }
                    if y >= height as i32 - 2 {
                        while y < height as i32 {
                            rail!(x, y, '│', col, id, 2);
                            y += 1;
                        }
                        break;
                    }
                    let jog: i32 =
                        rng.random_range(3..9) * if rng.random_range(0..2) == 0 { 1 } else { -1 };
                    let jog = if x + jog < 1 {
                        jog.abs()
                    } else if x + jog > width as i32 - 2 {
                        -jog.abs()
                    } else {
                        jog
                    };
                    let step = jog.signum();
                    rail!(x, y, if step > 0 { '╰' } else { '╯' }, col, id, 3);
                    for _ in 0..jog.abs() - 1 {
                        x += step;
                        rail!(x, y, '─', col, id, 1);
                    }
                    x += step;
                    rail!(x, y, if step > 0 { '╮' } else { '╭' }, col, id, 3);
                    y += 1;
                }
            } else {
                // horizontal-major: left to right with vertical jogs
                let mut y: i32 = rng.random_range(2..(height as i32 - 3).max(3));
                let mut x: i32 = 0;
                let mut until_station: i32 = rng.random_range(4..9);
                while x < width as i32 {
                    let run: i32 = rng.random_range(7..16);
                    for _ in 0..run {
                        if x >= width as i32 {
                            break;
                        }
                        until_station -= 1;
                        if until_station <= 0 {
                            rail!(x, y, '○', lighten(col, 30), id, 1);
                            until_station = rng.random_range(6..13);
                        } else {
                            rail!(x, y, '─', col, id, 1);
                        }
                        x += 1;
                    }
                    if x >= width as i32 - 2 {
                        while x < width as i32 {
                            rail!(x, y, '─', col, id, 1);
                            x += 1;
                        }
                        break;
                    }
                    let jog: i32 =
                        rng.random_range(2..6) * if rng.random_range(0..2) == 0 { 1 } else { -1 };
                    let jog = if y + jog < 1 {
                        jog.abs()
                    } else if y + jog > height as i32 - 2 {
                        -jog.abs()
                    } else {
                        jog
                    };
                    let step = jog.signum();
                    rail!(x, y, if step > 0 { '╮' } else { '╯' }, col, id, 3);
                    for _ in 0..jog.abs() - 1 {
                        y += step;
                        rail!(x, y, '│', col, id, 2);
                    }
                    y += step;
                    rail!(x, y, if step > 0 { '╰' } else { '╭' }, col, id, 3);
                    x += 1;
                }
            }
        }

        for y in 0..height {
            for x in 0..width {
                if occ_h[y][x] > 0 && occ_v[y][x] > 0 && occ_h[y][x] != occ_v[y][x] {
                    grid[y][x] = Cell::new('◉', lighten(palette[4], 15));
                }
            }
        }
    (grid, false)
}

/// Dispatch arm for mode(s): koi (moved verbatim from run()).
pub(crate) fn cli_koi(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        // koi [fish] -- pond seen from above: still water, ripple rings, lily
        // pads, koi gliding with curled tails
        let fish_count: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(7);
        let fish_count = fish_count.clamp(1, 24);

        let pond = Rect {
            x: 0,
            y: 0,
            w: width,
            h: height,
        };
        fill_noise(
            &mut grid,
            &pond,
            NoiseVariant::Dot,
            darken(palette[1], 80),
            darken(palette[2], 86),
            &mut rng,
        );

        // ripple rings, squashed to ellipses
        for _ in 0..(width / 25).max(2) {
            let cx = rng.random_range(0..width) as f32;
            let cy = rng.random_range(0..height) as f32;
            let rings = rng.random_range(1..3usize);
            for ring in 0..rings {
                let r = 1.5 + ring as f32 * 2.0;
                let steps = (r * 9.0) as usize;
                let col = darken(lighten(palette[1], 8), 25 + (ring * 18) as u8);
                for s in 0..steps {
                    let a = s as f32 / steps as f32 * std::f32::consts::TAU;
                    let x = (cx + a.cos() * r * 2.0).round() as i32;
                    let y = (cy + a.sin() * r).round() as i32;
                    if x < 0 || y < 0 || x as usize >= width || y as usize >= height {
                        continue;
                    }
                    let sy = a.sin();
                    let ch = if sy < -0.55 {
                        '‾'
                    } else if sy > 0.55 {
                        '_'
                    } else if a.cos() < 0.0 {
                        '('
                    } else {
                        ')'
                    };
                    grid[y as usize][x as usize] = Cell::new(ch, col);
                }
            }
        }

        // lily pads, some flowering
        for _ in 0..(width / 16).max(3) {
            let px = rng.random_range(2..width.saturating_sub(2).max(3));
            let py = rng.random_range(1..height.saturating_sub(1).max(2));
            let pad = lighten(palette[2], rng.random_range(0..18));
            grid[py][px - 1] = Cell::new('(', darken(pad, 20));
            grid[py][px] = Cell::new('◍', pad);
            grid[py][px + 1] = Cell::new(')', darken(pad, 20));
            if py > 0 && rng.random_range(0..3) == 0 {
                grid[py - 1][px] = Cell::new('✿', lighten(palette[3], 25));
            }
        }

        // koi: bright head, body fading into the water, sine-curled tail
        let body = ['◉', '◎', '○', '∘', '·'];
        for _ in 0..fish_count {
            let dir: i32 = if rng.random_range(0..2) == 0 { 1 } else { -1 };
            let x0 = rng.random_range(6..width.saturating_sub(6).max(7)) as i32;
            let y0 = rng.random_range(1..height.saturating_sub(1).max(2)) as i32;
            let hue = if rng.random_range(0..3) == 0 {
                lighten(palette[4], 10)
            } else {
                lighten(palette[3], 15)
            };
            let phase = rng.random_range(0.0f32..6.28);
            let sway0 = phase.sin();
            for (i, &ch) in body.iter().enumerate() {
                let x = x0 - dir * i as i32;
                let y = y0 + (((i as f32) * 0.7 + phase).sin() - sway0).round() as i32;
                if x < 0 || y < 0 || x as usize >= width || y as usize >= height {
                    continue;
                }
                let col = lerp_color(hue, darken(palette[1], 60), i as f32 / 4.0);
                grid[y as usize][x as usize] = Cell::new(ch, col);
            }
        }
    (grid, false)
}

/// Dispatch arm for mode(s): skyline (moved verbatim from run()).
pub(crate) fn cli_skyline(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        // skyline [lit] -- night city in four depth layers: far slabs, mid
        // backdrop, near towers built from facade archetypes (glass curtain,
        // masonry, ziggurat, banded slab, spire, dome), and foreground hulks
        // cropped by the frame. lit = percent of windows glowing.
        let lit: u32 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(35);
        let lit = lit.clamp(0, 100);

        let horizon = height.saturating_sub(2);

        for _ in 0..(width * height / 40).max(10) {
            let x = rng.random_range(0..width);
            let y = rng.random_range(0..horizon.max(1));
            let ch = match rng.random_range(0..8) {
                0 => '✦',
                1 => '+',
                _ => '·',
            };
            grid[y][x] = Cell::new(ch, darken(palette[4], rng.random_range(30..70)));
        }

        for _ in 0..rng.random_range(2..5usize) {
            let cw = rng.random_range(5..13usize);
            let cx0 = rng.random_range(0..width.saturating_sub(cw).max(1));
            let cy = rng.random_range(1..(height / 3).max(2));
            for i in 0..cw {
                let ch = if i == 0 || i == cw - 1 { '░' } else { '▒' };
                grid[cy][cx0 + i] = Cell::new(ch, darken(palette[4], 62));
                if i > 1 && i < cw - 2 && cy + 1 < height {
                    grid[cy + 1][cx0 + i] = Cell::new('░', darken(palette[4], 70));
                }
            }
        }

        for _ in 0..rng.random_range(1..3usize) {
            let bx = rng.random_range(4..width.saturating_sub(10).max(5));
            let by = rng.random_range(2..(height / 3).max(3));
            for i in 0..rng.random_range(3..6usize) {
                let x = bx + i * 2;
                let y = by + (i % 2);
                if x < width && y < height {
                    grid[y][x] = Cell::new('∨', darken(palette[4], 35));
                }
            }
        }

        let mx = rng.random_range(width / 8..(width / 3).max(width / 8 + 1)) as i32;
        let my = rng.random_range(2..(height / 4).max(3)) as i32;
        for dy in -1..=1i32 {
            for dx in -2..=2i32 {
                let e = (dx as f32 / 2.2).powi(2) + (dy as f32 / 1.2).powi(2);
                if e <= 1.0 {
                    let x = mx + dx;
                    let y = my + dy;
                    if x >= 0 && y >= 0 && (x as usize) < width && (y as usize) < height {
                        let ch = if dx == 1 && dy == 0 { '▒' } else { '▓' };
                        grid[y as usize][x as usize] = Cell::new(ch, lighten(palette[4], 20));
                    }
                }
            }
        }

        // far layer: low distant slabs with gaps of sky
        let far = darken(palette[1], 72);
        let mut x = 0usize;
        while x < width {
            let w = rng.random_range(4..9usize).min(width - x);
            let h = rng.random_range((height / 7).max(2)..(height / 3).max(3));
            let top = horizon.saturating_sub(h);
            for bx in x..x + w {
                grid[top][bx] = Cell::new('▄', far);
                for by in top + 1..horizon {
                    grid[by][bx] = Cell::new('▓', far);
                }
            }
            x += w + rng.random_range(0..6usize);
        }

        // mid layer: continuous backdrop with sparse dim windows
        let mid = darken(palette[1], 55);
        let mid_win = darken(palette[3], 25);
        let mut x = 0usize;
        while x < width {
            let w = rng.random_range(5..11usize).min(width - x);
            let h = rng.random_range((height / 4).max(3)..(height / 2).max(4));
            let top = horizon.saturating_sub(h);
            for bx in x..x + w {
                grid[top][bx] = Cell::new('▄', mid);
                for by in top + 1..horizon {
                    grid[by][bx] = Cell::new('█', mid);
                }
            }
            for by in (top + 2..horizon.saturating_sub(1)).step_by(3) {
                for bx in (x + 1..(x + w).saturating_sub(1)).step_by(3) {
                    if rng.random_range(0..100) < lit / 2 {
                        grid[by][bx] = Cell::new('▪', mid_win);
                    }
                }
            }
            x += w + rng.random_range(2..7usize);
        }

        // near layer: randomly placed towers that overlap into clusters.
        // each rolls a facade archetype, so the variety is in the pattern
        // language of the building, never just its size
        let near = darken(palette[1], 40);
        let win_on = lighten(palette[3], 30);
        let win_off = darken(palette[1], 12);
        let mut street_free = vec![true; width];
        let mut tall_top = horizon;
        let mut tall_x = width / 2;
        // roof gear and beams only land on open sky, never inside another tower
        macro_rules! deco {
            ($x:expr, $y:expr, $ch:expr, $col:expr) => {{
                let dx = $x;
                let dy = $y;
                if !matches!(
                    grid[dy][dx].ch,
                    '█' | '▓' | '▄' | '▀' | '▪' | '▮' | '□' | '║' | '▐' | '▬' | '╥'
                ) {
                    grid[dy][dx] = Cell::new($ch, $col);
                }
            }};
        }
        for _ in 0..(width / 14).max(3) {
            let kind = rng.random_range(0..6usize);
            let w = match kind {
                0 => rng.random_range(6..11usize),
                1 => rng.random_range(7..13usize),
                2 => rng.random_range(9..15usize),
                3 => rng.random_range(8..13usize),
                4 => rng.random_range(3..5usize),
                _ => rng.random_range(7..12usize),
            };
            let w = w.min(width);
            let x = rng.random_range(0..width.saturating_sub(w).max(1));
            for bx in x..x + w {
                street_free[bx] = false;
            }
            let btop = match kind {
                0 => {
                    // glass curtain: whole mullion strips light at once
                    let h = rng.random_range((height / 2).max(5)..(height * 3 / 4).max(6));
                    let top = horizon.saturating_sub(h);
                    for bx in x..x + w {
                        grid[top][bx] = Cell::new('▄', near);
                        for by in top + 1..horizon {
                            let ch = if (bx - x) % 2 == 0 { '█' } else { '▐' };
                            grid[by][bx] = Cell::new(ch, near);
                        }
                    }
                    let mstep = rng.random_range(2..4usize);
                    for bx in (x + 1..(x + w).saturating_sub(1)).step_by(mstep) {
                        let on = rng.random_range(0..100) < lit;
                        let col = if on { win_on } else { darken(palette[1], 22) };
                        for by in top + 1..horizon.saturating_sub(1) {
                            grid[by][bx] = Cell::new('║', col);
                        }
                    }
                    top
                }
                1 => {
                    // masonry: pale cornice, string courses, square windows
                    let h = rng.random_range((height / 3).max(4)..(height / 2).max(5));
                    let top = horizon.saturating_sub(h);
                    for bx in x..x + w {
                        grid[top][bx] = Cell::new('▀', lighten(near, 14));
                        for by in top + 1..horizon {
                            grid[by][bx] = Cell::new('▓', near);
                        }
                    }
                    for by in (top + 2..horizon.saturating_sub(1)).step_by(2) {
                        for bx in (x + 1..(x + w).saturating_sub(1)).step_by(3) {
                            let on = rng.random_range(0..100) < lit;
                            grid[by][bx] = Cell::new('□', if on { win_on } else { win_off });
                        }
                    }
                    top
                }
                2 => {
                    // art-deco ziggurat: tiers stepping in, spire on the crown
                    let mut tx = x;
                    let mut tw = w;
                    let mut bottom = horizon;
                    let mut top = horizon;
                    while tw >= 3 && bottom > 3 {
                        let th = rng.random_range(3..6usize).min(bottom - 1);
                        let t_top = bottom - th;
                        for bx in tx..(tx + tw).min(width) {
                            grid[t_top][bx] = Cell::new('▄', near);
                            for by in t_top + 1..bottom {
                                grid[by][bx] = Cell::new('█', near);
                            }
                        }
                        for by in (t_top + 1..bottom).step_by(2) {
                            for bx in
                                (tx + 1..(tx + tw).min(width).saturating_sub(1)).step_by(2)
                            {
                                let on = rng.random_range(0..100) < lit;
                                grid[by][bx] =
                                    Cell::new('▪', if on { win_on } else { win_off });
                            }
                        }
                        top = t_top;
                        bottom = t_top;
                        tx += 2;
                        tw = tw.saturating_sub(4);
                    }
                    let sx = (x + w / 2).min(width - 1);
                    if top >= 3 {
                        deco!(sx, top - 1, '│', near);
                        deco!(sx, top - 2, '│', near);
                        deco!(sx, top - 3, '✦', lighten(palette[3], 45));
                    }
                    top
                }
                3 => {
                    // banded slab: dark floor stripes, wide lit slots between
                    let h = rng.random_range((height / 3).max(4)..(height * 2 / 3).max(5));
                    let top = horizon.saturating_sub(h);
                    for bx in x..x + w {
                        grid[top][bx] = Cell::new('▄', near);
                        for by in top + 1..horizon {
                            let floor = (by - top) % 3 == 0;
                            let (ch, col) =
                                if floor { ('▄', darken(near, 14)) } else { ('█', near) };
                            grid[by][bx] = Cell::new(ch, col);
                        }
                    }
                    for by in top + 1..horizon.saturating_sub(1) {
                        if (by - top) % 3 != 2 {
                            continue;
                        }
                        for bx in (x + 1..(x + w).saturating_sub(1)).step_by(2) {
                            if rng.random_range(0..100) < lit {
                                grid[by][bx] = Cell::new('▬', win_on);
                            }
                        }
                    }
                    top
                }
                4 => {
                    // needle: thin, very tall, single window column, long mast
                    let h =
                        rng.random_range((height * 3 / 5).max(5)..(height * 5 / 6).max(6));
                    let top = horizon.saturating_sub(h);
                    for bx in x..x + w {
                        grid[top][bx] = Cell::new('▄', near);
                        for by in top + 1..horizon {
                            grid[by][bx] = Cell::new('▓', near);
                        }
                    }
                    let bx = x + w / 2;
                    for by in (top + 2..horizon.saturating_sub(1)).step_by(2) {
                        let on = rng.random_range(0..100) < lit;
                        grid[by][bx] = Cell::new('▪', if on { win_on } else { win_off });
                    }
                    let ah = rng.random_range(3..6usize).min(top);
                    for i in 1..=ah {
                        deco!(bx, top - i, '│', near);
                    }
                    if ah > 0 {
                        deco!(bx, top - ah, '✦', lighten(palette[3], 45));
                    }
                    top
                }
                _ => {
                    // civic dome: squat block, rounded cap, finial
                    let h = rng.random_range((height / 4).max(3)..(height / 2).max(4));
                    let top = horizon.saturating_sub(h);
                    for bx in x..x + w {
                        for by in top..horizon {
                            grid[by][bx] = Cell::new('█', near);
                        }
                    }
                    for by in (top + 1..horizon.saturating_sub(1)).step_by(3) {
                        for bx in (x + 1..(x + w).saturating_sub(1)).step_by(3) {
                            let on = rng.random_range(0..100) < lit;
                            grid[by][bx] = Cell::new('□', if on { win_on } else { win_off });
                        }
                    }
                    if top >= 2 {
                        for bx in x + 1..(x + w).saturating_sub(1) {
                            deco!(bx, top - 1, '▄', lighten(near, 8));
                        }
                        for bx in x + w / 3..(x + w - w / 3).min(width) {
                            deco!(bx, top - 2, '▄', lighten(near, 8));
                        }
                        if top >= 3 {
                            deco!(x + w / 2, top - 3, '+', lighten(palette[3], 35));
                        }
                    }
                    top
                }
            };
            // rooftop water tank on the flat-roofed kinds
            if matches!(kind, 1 | 3) && w >= 7 && btop >= 2 && rng.random_range(0..3) == 0 {
                let wx = x + rng.random_range(1..w - 2);
                deco!(wx, btop - 2, '▄', darken(near, 10));
                deco!(wx + 1, btop - 2, '▄', darken(near, 10));
                deco!(wx, btop - 1, '╥', near);
                deco!(wx + 1, btop - 1, '╥', near);
            }
            if btop < tall_top {
                tall_top = btop;
                tall_x = x + w / 2;
            }
        }

        // searchlight beams off the tallest tower
        if tall_top < horizon && tall_top >= 1 {
            for i in 1..7i32 {
                let y = tall_top as i32 - i;
                if y < 0 {
                    break;
                }
                let f = darken(palette[4], (40 + i * 8).min(90) as u8);
                let lx = tall_x as i32 - i;
                let rx2 = tall_x as i32 + i;
                if lx >= 0 && (lx as usize) < width {
                    deco!(lx as usize, y as usize, '╲', f);
                }
                if rx2 >= 0 && (rx2 as usize) < width {
                    deco!(rx2 as usize, y as usize, '╱', f);
                }
            }
        }

        // parks with a streetlight wherever no near tower landed
        let park = darken(palette[2], 30);
        let mut gx = 0usize;
        while gx < width {
            if street_free[gx] {
                let start = gx;
                while gx < width && street_free[gx] {
                    gx += 1;
                }
                let glen = gx - start;
                if glen >= 4 && horizon >= 3 {
                    for tx in start..gx {
                        match rng.random_range(0..5usize) {
                            0 => grid[horizon - 1][tx] = Cell::new('♣', park),
                            1 => grid[horizon - 1][tx] = Cell::new('♠', darken(park, 12)),
                            2 => grid[horizon - 1][tx] = Cell::new('·', darken(park, 20)),
                            _ => {}
                        }
                    }
                    let lx = start + glen / 2;
                    grid[horizon - 1][lx] = Cell::new('│', darken(palette[4], 50));
                    grid[horizon - 2][lx] = Cell::new('✶', lighten(palette[3], 40));
                }
            } else {
                gx += 1;
            }
        }

        for gx in 0..width {
            if horizon < height {
                grid[horizon][gx] = Cell::new('─', darken(palette[4], 65));
            }
            if horizon + 1 < height && gx % 3 == 0 {
                grid[horizon + 1][gx] = Cell::new('·', darken(palette[1], 70));
            }
        }

        // foreground hulks: this side of the street, near-black, cropped by
        // the bottom of the frame so they read as closest
        let fg = darken(palette[1], 80);
        let fg_win = lighten(palette[3], 40);
        for _ in 0..rng.random_range(1..3usize) {
            let w = rng.random_range(12..22usize).min(width);
            let x = rng.random_range(0..width.saturating_sub(w).max(1));
            let top = rng.random_range((height * 3 / 5).max(2)..(height * 7 / 8).max(3));
            for bx in x..x + w {
                grid[top][bx] = Cell::new('▄', fg);
                for by in top + 1..height {
                    grid[by][bx] = Cell::new('█', fg);
                }
            }
            for by in (top + 2..height.saturating_sub(1)).step_by(3) {
                for bx in (x + 2..(x + w).saturating_sub(2)).step_by(4) {
                    if rng.random_range(0..100) < lit {
                        grid[by][bx] = Cell::new('▮', fg_win);
                        if bx + 1 + 2 < x + w {
                            grid[by][bx + 1] = Cell::new('▮', fg_win);
                        }
                    }
                }
            }
        }
    (grid, false)
}

/// Dispatch arm for mode(s): hive (moved verbatim from run()).
pub(crate) fn cli_hive(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        // hive [fill] -- a comb hanging from the top edge: hex lattice masked
        // to a noise-warped teardrop, ragged bare-wall rim, honey drips off the
        // tip, bees working the boundary. fill = percent of comb cells with honey.
        let fill: u32 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(45);
        let fill_f = fill.clamp(0, 100) as f32 / 100.0;

        let comb = darken(palette[3], 25);
        let honey = lighten(palette[3], 12);
        let ax = width as f32 * rng.random_range(0.38..0.62);
        let rx = width as f32 * rng.random_range(0.22..0.3);
        let ry = height as f32 * rng.random_range(0.75..0.95);
        let mseed = seed.wrapping_add(77);
        // > 0.12 full hex, > 0.0 bare rim wall, otherwise open air
        let mask = |x: f32, y: f32| -> f32 {
            let yn = (y / height.max(1) as f32).clamp(0.0, 1.0);
            let rx_eff = rx * (1.0 - 0.4 * yn);
            let dx = (x - ax) / rx_eff.max(1.0);
            let dy = y / ry.max(1.0);
            1.0 - (dx * dx + dy * dy).sqrt() + (pp_fbm(x / 9.0, y / 5.0, mseed) - 0.5) * 0.7
        };

        for y in 0..height {
            let a_row = y % 2 == 0;
            for x in 0..width {
                let m = mask(x as f32, y as f32);
                if m <= 0.0 {
                    continue;
                }
                let ph = x % 6;
                let ch = if a_row {
                    match ph {
                        0 => '/',
                        3 => '\\',
                        4 | 5 => '_',
                        _ => ' ',
                    }
                } else {
                    match ph {
                        0 => '\\',
                        1 | 2 => '_',
                        3 => '/',
                        _ => ' ',
                    }
                };
                if ch != ' ' {
                    let wall = if m < 0.12 { darken(comb, 18) } else { comb };
                    grid[y][x] = Cell::new(ch, wall);
                } else if m >= 0.12 {
                    // per-hex content, keyed on the hex so both interior cells agree
                    let r = pp_hash2((x / 6) as i32, y as i32, seed);
                    grid[y][x] = if r < fill_f * 0.7 {
                        Cell::new('▒', honey)
                    } else if r < fill_f {
                        Cell::new('▓', darken(honey, 25))
                    } else if r < fill_f + 0.12 && (ph == 1 || ph == 4) {
                        Cell::new('·', lighten(palette[4], 5))
                    } else {
                        Cell::blank()
                    };
                }
            }
        }

        // honey drips off the underside
        for _ in 0..rng.random_range(3..6usize) {
            let dx = rng.random_range(-(rx * 0.6) as i32..=(rx * 0.6) as i32);
            let x = (ax as i32 + dx).clamp(0, width as i32 - 1) as usize;
            let mut bottom = None;
            for y in (0..height).rev() {
                if mask(x as f32, y as f32) > 0.12 {
                    bottom = Some(y);
                    break;
                }
            }
            if let Some(by) = bottom {
                let len = rng.random_range(1..4usize);
                for i in 1..=len {
                    if by + i < height {
                        grid[by + i][x] = Cell::new('│', darken(honey, 25));
                    }
                }
                if by + len + 1 < height {
                    grid[by + len + 1][x] = Cell::new('∙', honey);
                }
            }
        }

        // bees swarm the rim, trails leading out into open air
        let mut placed = 0usize;
        let mut tries = 0usize;
        let want = (width / 8).max(5);
        while placed < want && tries < 600 {
            tries += 1;
            let bx = rng.random_range(0..width);
            let by = rng.random_range(0..height);
            let m = mask(bx as f32, by as f32);
            if m > -0.35 && m < 0.06 {
                let dir: i32 = if (bx as f32) < ax { -1 } else { 1 };
                for i in 1..4i32 {
                    let tx = bx as i32 + dir * i * 2;
                    let ty = by as i32 - (i % 2);
                    if tx >= 0 && ty >= 0 && (tx as usize) < width && (ty as usize) < height {
                        grid[ty as usize][tx as usize] = Cell::new('·', darken(palette[4], 45));
                    }
                }
                grid[by][bx] = Cell::new('ø', lighten(palette[3], 35));
                placed += 1;
            }
        }
    (grid, false)
}

/// Dispatch arm for mode(s): jelly (moved verbatim from run()).
pub(crate) fn cli_jelly(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        // jelly [count] -- deep-sea drift: translucent bells with swaying
        // tentacles, rising bubbles, light shafts from the surface
        let count: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(4);
        let count = count.clamp(1, 12);

        macro_rules! jput {
            ($x:expr, $y:expr, $ch:expr, $col:expr) => {{
                let sx = $x;
                let sy = $y;
                if sx >= 0 && sy >= 0 && (sx as usize) < width && (sy as usize) < height {
                    grid[sy as usize][sx as usize] = Cell::new($ch, $col);
                }
            }};
        }

        // depth-graded plankton
        for y in 0..height {
            let depth = y as f32 / height.max(1) as f32;
            let wc = lerp_color(darken(palette[1], 65), darken(palette[0], 30), depth);
            for x in 0..width {
                if rng.random_range(0..100) < 3 {
                    grid[y][x] = Cell::new('·', wc);
                }
            }
        }

        // broken light shafts slanting down from the surface
        for _ in 0..3 {
            let mut sx = rng.random_range(0..width as i32);
            let depth = rng.random_range(height / 3..(height * 3 / 4).max(height / 3 + 1));
            for y in 0..depth {
                if y % 3 == 2 {
                    sx += 1;
                }
                if rng.random_range(0..5) < 3 {
                    let ch = if y % 3 == 2 { '╲' } else { '│' };
                    let fade = 55 + (y * 30 / depth.max(1)) as u8;
                    jput!(sx, y as i32, ch, darken(palette[4], fade));
                }
            }
        }

        for _ in 0..(width * height / 90).max(6) {
            let x = rng.random_range(0..width);
            let y = rng.random_range(0..height);
            let ch = ['°', 'º', '∘', '·'][rng.random_range(0..4usize)];
            grid[y][x] = Cell::new(ch, darken(palette[4], rng.random_range(25..60)));
        }

        for _ in 0..count {
            let r: i32 = rng.random_range(2..5);
            let cx = rng.random_range(4..(width as i32 - 4).max(5));
            let cy = rng.random_range(2..(height as i32 * 2 / 3).max(3));
            let hue = shift_hue(lighten(palette[2], 18), rng.random_range(-40.0..40.0));

            for dx in -r..=r {
                let ch = if dx == -r {
                    '▗'
                } else if dx == r {
                    '▖'
                } else {
                    '▄'
                };
                jput!(cx + dx, cy - 1, ch, hue);
            }
            for dx in -r - 1..=r + 1 {
                let ch = if dx == -r - 1 {
                    '▐'
                } else if dx == r + 1 {
                    '▌'
                } else if (dx + r).rem_euclid(3) == 0 {
                    '░'
                } else {
                    '▒'
                };
                jput!(cx + dx, cy, ch, hue);
            }
            jput!(cx, cy, '✦', lighten(hue, 25));

            for dx in (-r..=r).step_by(2) {
                let len: i32 = rng.random_range(3..(height as i32 / 3).max(4));
                let phase = rng.random_range(0.0f32..6.3);
                let sway0 = phase.sin();
                let mut prev_off = 0i32;
                for j in 1..=len {
                    let sway = (j as f32 * 0.55 + phase).sin();
                    let off = ((sway - sway0) * 1.6).round() as i32;
                    let slope = off - prev_off;
                    prev_off = off;
                    let ch = if slope > 0 {
                        ')'
                    } else if slope < 0 {
                        '('
                    } else {
                        '|'
                    };
                    let fade = darken(hue, (j * 90 / len.max(1)).min(80) as u8);
                    jput!(cx + dx + off, cy + j, ch, fade);
                }
            }
        }
    (grid, false)
}

/// Dispatch arm for mode(s): jelly2 (moved verbatim from run()).
pub(crate) fn cli_jelly2(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        // jelly2 [count] -- generative jellies. every jelly rolls a species
        // from independent parts: bell shape (shaded dome, moon jelly, box
        // jelly, tall bulb, sideways swimmer), tentacle style (curtain,
        // ribbon, stingers, frill), and an orientation: the bell shears with
        // tilt and the tails lean against it with drift, so no two hang at
        // the same angle. test bed for the parts-generator the aquarium needs.
        let count: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(5);
        let count = count.clamp(1, 14);

        macro_rules! jput {
            ($x:expr, $y:expr, $ch:expr, $col:expr) => {{
                let sx = $x;
                let sy = $y;
                if sx >= 0 && sy >= 0 && (sx as usize) < width && (sy as usize) < height {
                    grid[sy as usize][sx as usize] = Cell::new($ch, $col);
                }
            }};
        }

        // marine snow, dimmer with depth
        for y in 0..height {
            let depth = y as f32 / height.max(1) as f32;
            let wc = lerp_color(darken(palette[1], 60), darken(palette[0], 30), depth);
            for x in 0..width {
                if rng.random_range(0..100) < 3 {
                    let ch = ['·', '˚', '.'][rng.random_range(0..3usize)];
                    grid[y][x] = Cell::new(ch, wc);
                }
            }
        }

        // faint current ribbons
        for _ in 0..2 {
            let ry0 = rng.random_range((height / 5).max(1)..(height * 4 / 5).max(2)) as i32;
            let ph = rng.random_range(0.0f32..6.3);
            let mut x = 0i32;
            while (x as usize) < width {
                let y = ry0 + ((x as f32 * 0.3 + ph).sin() * 1.4).round() as i32;
                if rng.random_range(0..3) < 2 {
                    jput!(x, y, '~', darken(palette[1], 52));
                }
                x += rng.random_range(1..3);
            }
        }

        for _ in 0..(width * height / 110).max(5) {
            let x = rng.random_range(0..width);
            let y = rng.random_range(0..height);
            let ch = ['°', 'º', '∘', '·'][rng.random_range(0..4usize)];
            grid[y][x] = Cell::new(ch, darken(palette[4], rng.random_range(28..60)));
        }

        for _ in 0..count {
            let r: i32 = rng.random_range(2..6);
            let cx = rng.random_range(4..(width as i32 - 4).max(5));
            let cy = rng.random_range(3..(height as i32 * 2 / 3).max(4));
            let base_hue = shift_hue(lighten(palette[2], 18), rng.random_range(-60.0..60.0));
            let hue = darken(base_hue, (cy * 22 / height.max(1) as i32).max(0) as u8);
            let bell = rng.random_range(0..5usize);
            let tilt = rng.random_range(-1.6f32..1.6);
            let ti = tilt.round() as i32;
            // tails trail against the lean
            let drift = -tilt * rng.random_range(0.25..0.6);
            let tstyle = rng.random_range(0..4usize);

            if bell == 4 {
                // sideways swimmer: bell opens along x, tentacles stream behind
                let dir: i32 = if rng.random_range(0..2) == 0 { 1 } else { -1 };
                jput!(cx, cy - 1, if dir > 0 { '▗' } else { '▖' }, darken(hue, 12));
                jput!(cx, cy + 1, if dir > 0 { '▝' } else { '▘' }, darken(hue, 12));
                jput!(cx + dir, cy - 1, if dir > 0 { '\\' } else { '/' }, hue);
                jput!(cx + dir, cy + 1, if dir > 0 { '/' } else { '\\' }, hue);
                jput!(cx, cy, '▒', hue);
                jput!(cx + dir, cy, '▒', hue);
                jput!(cx + 2 * dir, cy, if dir > 0 { ')' } else { '(' }, hue);
                let len: i32 = rng.random_range(5..(width as i32 / 5).max(6));
                for ty in [cy - 1, cy, cy + 1] {
                    let phase = rng.random_range(0.0f32..6.3);
                    let amp = rng.random_range(0.8f32..1.6);
                    let mut prev = 0i32;
                    for j in 1..=len {
                        let sway =
                            ((j as f32 * 0.5 + phase).sin() - phase.sin()) * amp;
                        let off = sway.round() as i32;
                        let slope = (off - prev) * dir;
                        prev = off;
                        let ch = if slope > 0 {
                            '/'
                        } else if slope < 0 {
                            '\\'
                        } else {
                            '~'
                        };
                        let fade = darken(hue, (j * 80 / len.max(1)).min(80) as u8);
                        jput!(cx - dir * (1 + j), ty + off, ch, fade);
                    }
                }
                continue;
            }

            // upright bells: crown row sheared by tilt, body row on cx
            let crown_x = cx + ti;
            match bell {
                0 => {
                    // shaded dome
                    for dx in -r + 1..=r - 1 {
                        let ch = if dx == -r + 1 {
                            '▗'
                        } else if dx == r - 1 {
                            '▖'
                        } else {
                            '▄'
                        };
                        jput!(crown_x + dx, cy - 1, ch, hue);
                    }
                    for dx in -r..=r {
                        let ch = if dx == -r {
                            '▐'
                        } else if dx == r {
                            '▌'
                        } else if (dx + r).rem_euclid(3) == 0 {
                            '░'
                        } else {
                            '▒'
                        };
                        jput!(cx + dx, cy, ch, hue);
                    }
                    jput!(cx, cy, '✦', lighten(hue, 25));
                }
                1 => {
                    // moon jelly: scalloped crown over a clear bell, gonad rings
                    for dx in -r + 1..=r - 1 {
                        jput!(crown_x + dx, cy - 1, '∩', hue);
                    }
                    jput!(cx - r, cy, '(', hue);
                    jput!(cx + r, cy, ')', hue);
                    jput!(cx - 1, cy, '∘', lighten(hue, 20));
                    jput!(cx + 1, cy, '∘', lighten(hue, 20));
                }
                2 => {
                    // box jelly, angular
                    for dx in -r..=r {
                        let ch = if dx == -r {
                            '┌'
                        } else if dx == r {
                            '┐'
                        } else {
                            '─'
                        };
                        jput!(crown_x + dx, cy - 1, ch, hue);
                    }
                    for dx in -r..=r {
                        let ch = if dx.abs() == r { '│' } else { '▒' };
                        jput!(cx + dx, cy, ch, hue);
                    }
                }
                _ => {
                    // tall bulb, two body rows, shear splits across them
                    let midx = cx + ti / 2;
                    for dx in -r + 1..=r - 1 {
                        jput!(crown_x + dx, cy - 2, '▄', hue);
                    }
                    for dx in -r..=r {
                        let ch = if dx == -r {
                            '▐'
                        } else if dx == r {
                            '▌'
                        } else if (dx + r) % 2 == 0 {
                            '▒'
                        } else {
                            '░'
                        };
                        jput!(midx + dx, cy - 1, ch, hue);
                    }
                    for dx in -r + 1..=r - 1 {
                        let ch = if dx == -r + 1 {
                            '('
                        } else if dx == r - 1 {
                            ')'
                        } else {
                            '░'
                        };
                        jput!(cx + dx, cy, ch, hue);
                    }
                    jput!(midx, cy - 1, '✦', lighten(hue, 25));
                }
            }

            let (step, base_len, amp) = match tstyle {
                0 => (2usize, height as i32 / 4, 1.6f32),
                1 => (3, height as i32 / 3, 2.6),
                2 => (2, height as i32 / 4, 0.0),
                _ => (1, 3, 0.9),
            };
            for (k, dx) in (-r + 1..=r - 1).step_by(step).enumerate() {
                let len: i32 = (base_len + rng.random_range(-2..3)).max(2);
                let phase = rng.random_range(0.0f32..6.3);
                let mut prev = 0i32;
                for j in 1..=len {
                    let sway = ((j as f32 * 0.55 + phase).sin() - phase.sin()) * amp;
                    let off = (sway + drift * j as f32).round() as i32;
                    let slope = off - prev;
                    prev = off;
                    if tstyle == 2 && j % 2 == 0 {
                        continue; // gappy stingers
                    }
                    let ch = match tstyle {
                        2 => ['¦', ':', '·'][(j % 3) as usize],
                        1 => {
                            if slope > 0 {
                                ')'
                            } else if slope < 0 {
                                '('
                            } else {
                                '~'
                            }
                        }
                        3 => {
                            if (k + j as usize) % 2 == 0 {
                                '}'
                            } else {
                                '{'
                            }
                        }
                        _ => {
                            if slope > 0 {
                                ')'
                            } else if slope < 0 {
                                '('
                            } else {
                                '|'
                            }
                        }
                    };
                    let fade = darken(hue, (j * 85 / len.max(1)).min(80) as u8);
                    jput!(cx + dx + off, cy + j, ch, fade);
                }
            }
            // frill species still get a long trailing pair at the bell edges
            if tstyle == 3 {
                for dx in [-r + 1, r - 1] {
                    let len: i32 = rng.random_range(
                        (height as i32 / 4).max(3)..(height as i32 / 2).max(4),
                    );
                    let phase = rng.random_range(0.0f32..6.3);
                    let mut prev = 0i32;
                    for j in 1..=len {
                        let sway = ((j as f32 * 0.5 + phase).sin() - phase.sin()) * 1.8;
                        let off = (sway + drift * j as f32).round() as i32;
                        let slope = off - prev;
                        prev = off;
                        let ch = if slope > 0 {
                            ')'
                        } else if slope < 0 {
                            '('
                        } else {
                            '|'
                        };
                        let fade = darken(hue, (j * 85 / len.max(1)).min(80) as u8);
                        jput!(cx + dx + off, cy + j, ch, fade);
                    }
                }
            }
        }

        // small fry drifting in the back
        for _ in 0..count / 2 + 1 {
            let x = rng.random_range(1..(width as i32 - 1).max(2));
            let y = rng.random_range(1..(height as i32 - 2).max(2));
            let dim = darken(palette[2], 42);
            jput!(x, y, '∩', dim);
            let tail = if rng.random_range(0..2) == 0 { '¦' } else { '\'' };
            jput!(x, y + 1, tail, darken(dim, 15));
        }
    (grid, false)
}
