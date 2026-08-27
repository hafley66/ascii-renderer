#![allow(warnings)]

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
use crate::automata; use crate::avant; use crate::biomes; use crate::borders; use crate::color; use crate::content; use crate::fills; use crate::layout; use crate::markdown; use crate::mondrian; use crate::render; use crate::scene; use crate::sprites; use crate::tree_draw; use crate::types; use crate::walker;
use crate::cli::*;
use crate::gridio::*;
use crate::ink::*;
use crate::modes_creatures::*;
use crate::modes_geo::*;
use crate::modes_tree::*;
use crate::morph::*;
use crate::opts::*;
use crate::pp::*;
use crate::registry::*;
use crate::warps::*;

// --- nebula : fbm cloud field with a shade ramp, palette gradient, scattered
//     stars in the dark voids. ---
pub(crate) fn draw_nebula(grid: &mut Grid, width: usize, height: usize, seed: u64, palette: &[Color; 5], rng: &mut StdRng, t: f32) {
    let ramp = [' ', ' ', '·', '∙', ':', '*', '▒', '▓'];
    // t pans the cloud field; the starfield (rng-placed) stays put behind it.
    for y in 0..height {
        for x in 0..width {
            let fx = x as f32 / 11.0 + t * 0.08;
            let fy = y as f32 / 5.5;
            let n = pp_fbm(fx, fy, seed);
            let t = ((n - 0.25) * 1.7).clamp(0.0, 1.0);
            let idx = (t * (ramp.len() - 1) as f32).round() as usize;
            let body = lerp_color(darken(palette[0], 4), palette[2], t);
            let col = lerp_color(body, palette[3], t * t);
            grid[y][x] = Cell::new(ramp[idx.min(ramp.len() - 1)], col);
        }
    }
    let star_count = (width * height) / 36;
    for _ in 0..star_count {
        let x = rng.random_range(0..width);
        let y = rng.random_range(0..height);
        let n = pp_fbm(x as f32 / 11.0, y as f32 / 5.5, seed);
        if n < 0.42 {
            let ch = match rng.random_range(0..3) {
                0 => '✦',
                1 => '✧',
                _ => '·',
            };
            grid[y][x] = Cell::new(ch, lighten(palette[4], 0));
        }
    }
}


// --- delta : recursive branching river/lightning system fanning down-screen. ---
pub(crate) fn draw_solar_system(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, args: &[String]) -> Grid {
        // solar-system [bodies] -- 3D-ish orbital diagram with planets, cubes, and space hardware
        let body_count: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(8);
        let body_count = body_count.clamp(3, 12);

        macro_rules! set_cell {
            ($x:expr, $y:expr, $ch:expr, $fg:expr) => {{
                let sx = $x;
                let sy = $y;
                if sx >= 0 && sy >= 0 && (sx as usize) < width && (sy as usize) < height {
                    grid[sy as usize][sx as usize] = Cell::new($ch, $fg);
                }
            }};
        }

        let space = Rect {
            x: 0,
            y: 0,
            w: width,
            h: height,
        };
        fill_noise(
            &mut grid,
            &space,
            NoiseVariant::Dot,
            darken(palette[2], 94),
            darken(palette[3], 88),
            &mut rng,
        );

        for _ in 0..(width * height / 36).max(8) {
            let x = rng.random_range(0..width);
            let y = rng.random_range(0..height);
            let ch = ['·', '∙', '°', '*', '✦'][rng.random_range(0..5usize)];
            grid[y][x] = Cell::new(
                ch,
                darken(lighten(palette[4], 10), rng.random_range(15..75)),
            );
        }

        let center_phase = seed as f32 * 0.073 + t_anim * 0.25;
        let center_x_ratio = (0.47 + center_phase.sin() * 0.12).clamp(0.34, 0.62);
        let center_y_ratio = (0.51 + (center_phase * 1.41).cos() * 0.14).clamp(0.34, 0.66);
        let cx = width as f32 * center_x_ratio;
        let cy = height as f32 * center_y_ratio;
        let max_rx = ((cx - 3.0).min(width as f32 - cx - 3.0))
            .max(12.0)
            .min(width as f32 * 0.46);
        let min_rx = (width as f32 * 0.10).max(5.0);
        let max_ry = ((cy - 2.0).min(height as f32 - cy - 2.0))
            .max(4.0)
            .min(height as f32 * 0.38);
        let min_ry = (height as f32 * 0.08).max(2.0).min(max_ry * 0.55);
        let orbit_count = body_count.min(10);
        let sun_rx = (width as f32 / 16.0).clamp(4.0, 8.0);
        let sun_ry = (height as f32 / 8.0).clamp(2.0, 4.0);

        // Perspective orbital plane: rear arcs now complete behind the solar sphere.
        for i in 0..orbit_count {
            let t = i as f32 / orbit_count.max(1) as f32;
            let rx = min_rx + (max_rx - min_rx) * t;
            let ry = min_ry + (max_ry - min_ry) * t;
            let tilt = (i as f32 - orbit_count as f32 * 0.5) * 0.22;
            for s in 0..360 {
                if s % 3 == 1 && i > 6 {
                    continue;
                }
                let a = s as f32 / 360.0 * std::f32::consts::TAU;
                let x = cx + a.cos() * rx + a.sin() * tilt;
                let y = cy + a.sin() * ry;
                if x < 0.0 || y < 0.0 || x >= width as f32 || y >= height as f32 {
                    continue;
                }
                let near = a.sin() > 0.0;
                let ch = if near {
                    if s % 7 == 0 { '═' } else { '─' }
                } else if s % 11 == 0 {
                    '∙'
                } else if s % 4 == 0 {
                    '·'
                } else {
                    ' '
                };
                if ch != ' ' {
                    let color = if near {
                        darken(palette[4], 45)
                    } else {
                        darken(palette[2], 72)
                    };
                    set_cell!(x.round() as i32, y.round() as i32, ch, color);
                }
            }
        }

        // Solar sphere with shaded cells.
        for yy in (cy - sun_ry - 1.0).floor() as i32..=(cy + sun_ry + 1.0).ceil() as i32 {
            for xx in (cx - sun_rx - 1.0).floor() as i32..=(cx + sun_rx + 1.0).ceil() as i32 {
                let dx = (xx as f32 - cx) / sun_rx;
                let dy = (yy as f32 - cy) / sun_ry;
                let d = (dx * dx + dy * dy).sqrt();
                if d > 1.0 {
                    continue;
                }
                let ch = if d < 0.28 {
                    '◉'
                } else if dx < -0.2 || dy > 0.35 {
                    '▒'
                } else if d > 0.78 {
                    '░'
                } else {
                    '●'
                };
                let color = if d < 0.35 {
                    lighten(palette[3], 35)
                } else if dx < -0.2 || dy > 0.35 {
                    darken(palette[3], 25)
                } else {
                    lighten(palette[3], 10)
                };
                set_cell!(xx, yy, ch, color);
            }
        }

        // Planets, moons, and little labels/ticks.
        let planet_glyphs = ['●', '◐', '◑', '◉', '◆', '○'];
        for i in 0..body_count {
            let t = i as f32 / body_count.max(1) as f32;
            let rx = min_rx + (max_rx - min_rx) * (0.12 + t * 0.88);
            let ry = min_ry + (max_ry - min_ry) * (0.12 + t * 0.88);
            let angle = seed as f32 * 0.017 + i as f32 * 1.37 + rng.random::<f32>() * 0.35;
            let px = cx + angle.cos() * rx + angle.sin() * (i as f32 - 3.0) * 0.18;
            let py = cy + angle.sin() * ry;
            let radius = match i % 5 {
                0 => 1,
                1 => 2,
                2 => 1,
                3 => 3,
                _ => 2,
            };
            let color = shift_hue(lighten(palette[1 + i % 3], 12), (i * 37) as f64);

            if radius == 1 {
                set_cell!(
                    px.round() as i32,
                    py.round() as i32,
                    planet_glyphs[i % planet_glyphs.len()],
                    color
                );
            } else {
                for dy in -(radius as i32)..=(radius as i32) {
                    for dx in -(radius as i32 * 2)..=(radius as i32 * 2) {
                        let nx = dx as f32 / (radius as f32 * 2.0);
                        let ny = dy as f32 / radius as f32;
                        if nx * nx + ny * ny > 1.0 {
                            continue;
                        }
                        let shade = nx * -0.7 + ny * 0.45;
                        let ch = if shade > 0.35 {
                            '░'
                        } else if shade < -0.35 {
                            '▓'
                        } else if dx == 0 && dy == 0 {
                            '◉'
                        } else {
                            '●'
                        };
                        let fg = if shade > 0.35 {
                            darken(color, 35)
                        } else if shade < -0.35 {
                            lighten(color, 25)
                        } else {
                            color
                        };
                        set_cell!(px.round() as i32 + dx, py.round() as i32 + dy, ch, fg);
                    }
                }
            }

            if i % 3 == 1 {
                let moon_angle = angle * 1.9 + 0.8;
                let mx = px + moon_angle.cos() * (radius as f32 * 3.2 + 3.0);
                let my = py + moon_angle.sin() * (radius as f32 * 1.3 + 1.4);
                set_cell!(
                    mx.round() as i32,
                    my.round() as i32,
                    '○',
                    lighten(palette[4], 5)
                );
                set_cell!(
                    ((px + mx) * 0.5).round() as i32,
                    ((py + my) * 0.5).round() as i32,
                    '·',
                    darken(palette[4], 45)
                );
            }

            if i % 4 == 2 {
                let lx = px.round() as i32 + radius as i32 * 2 + 2;
                let ly = py.round() as i32;
                for (j, ch) in format!("p{}", i + 1).chars().enumerate() {
                    set_cell!(lx + j as i32, ly, ch, darken(palette[4], 38));
                }
            }
        }

        // Isometric orbital stations: seed-driven boxes riding different orbital lanes.
        let station_count = 2 + (seed as usize % 2);
        for s in 0..station_count {
            let lane = (0.56 + s as f32 * 0.17).min(0.92);
            let station_angle = center_phase * (0.52 + s as f32 * 0.21)
                + s as f32 * std::f32::consts::TAU / station_count as f32
                + 0.85;
            let sx = cx + station_angle.cos() * max_rx * lane;
            let sy = cy + station_angle.sin() * max_ry * (0.66 + s as f32 * 0.08);
            let cube_w = (width as i32 / (10 + s as i32)).clamp(6, 13);
            let cube_h = (height as i32 / (5 + s as i32)).clamp(4, 8);
            let max_cube_x = (width as i32 - cube_w - 7).max(1);
            let max_cube_y = (height as i32 - cube_h - 4).max(2);
            let cube_x = (sx.round() as i32 - cube_w / 2).clamp(1, max_cube_x);
            let cube_y = (sy.round() as i32 - cube_h / 2).clamp(2, max_cube_y);
            let off_x: i32 = if sx >= cx { 4 } else { -4 };
            let off_y: i32 = if s % 2 == 0 { -2 } else { 2 };
            let back_x = cube_x + off_x;
            let back_y = cube_y + off_y;
            let cube_color = shift_hue(lighten(palette[2], 25), s as f64 * 46.0);
            let back_color = darken(cube_color, 18);

            for x in 0..=cube_w {
                set_cell!(cube_x + x, cube_y, '─', cube_color);
                set_cell!(cube_x + x, cube_y + cube_h, '─', cube_color);
                set_cell!(back_x + x, back_y, '─', back_color);
                set_cell!(back_x + x, back_y + cube_h, '─', back_color);
            }
            for y in 0..=cube_h {
                set_cell!(cube_x, cube_y + y, '│', cube_color);
                set_cell!(cube_x + cube_w, cube_y + y, '│', cube_color);
                set_cell!(back_x, back_y + y, '│', back_color);
                set_cell!(back_x + cube_w, back_y + y, '│', back_color);
            }
            for &(x, y, ch) in &[
                (cube_x, cube_y, '┌'),
                (cube_x + cube_w, cube_y, '┐'),
                (cube_x, cube_y + cube_h, '└'),
                (cube_x + cube_w, cube_y + cube_h, '┘'),
                (back_x, back_y, '┌'),
                (back_x + cube_w, back_y, '┐'),
                (back_x, back_y + cube_h, '└'),
                (back_x + cube_w, back_y + cube_h, '┘'),
            ] {
                set_cell!(x, y, ch, lighten(cube_color, 10));
            }

            let connector = if off_x > 0 { '╱' } else { '╲' };
            for k in 1..=off_x.abs() {
                let dx = if off_x > 0 { k } else { -k };
                let dy = off_y * k / off_x.abs();
                set_cell!(cube_x + dx, cube_y + dy, connector, darken(cube_color, 5));
                set_cell!(
                    cube_x + cube_w + dx,
                    cube_y + dy,
                    connector,
                    darken(cube_color, 5)
                );
                set_cell!(
                    cube_x + dx,
                    cube_y + cube_h + dy,
                    connector,
                    darken(cube_color, 5)
                );
                set_cell!(
                    cube_x + cube_w + dx,
                    cube_y + cube_h + dy,
                    connector,
                    darken(cube_color, 5)
                );
            }
            for y in 1..cube_h {
                for x in 1..cube_w {
                    if (x * 2 + y + s as i32) % 4 == 0 {
                        set_cell!(cube_x + x, cube_y + y, '▪', darken(cube_color, 30));
                    }
                }
            }

            let dock_dir: i32 = if sx < cx { 1 } else { -1 };
            let dock_y = cube_y + cube_h / 2;
            for k in 1..=5 {
                set_cell!(
                    cube_x + if dock_dir > 0 { cube_w + k } else { -k },
                    dock_y,
                    '─',
                    darken(cube_color, 20)
                );
            }
            set_cell!(
                cube_x + if dock_dir > 0 { cube_w + 6 } else { -6 },
                dock_y,
                '◇',
                lighten(palette[3], 20)
            );
        }

        // Solar panel squares and a probe mast, also attached to a seed-shifting lane.
        let panel_angle = (center_phase * 0.88 - 0.20).rem_euclid(std::f32::consts::TAU);
        let panel_anchor_x = cx + panel_angle.cos() * max_rx * 0.78;
        let panel_anchor_y = cy + panel_angle.sin() * max_ry * 0.86;
        let panel_x_max = (width as i32 - 32).max(2);
        let panel_y_max = (height as i32 - 7).max(2);
        let panel_x = (panel_anchor_x.round() as i32 - 13).clamp(2, panel_x_max);
        let panel_y = (panel_anchor_y.round() as i32 - 2).clamp(2, panel_y_max);
        for p in 0..3 {
            let x0 = panel_x + p * 9;
            let y0 = panel_y + if p % 2 == 0 { 0 } else { -1 };
            for x in 0..7 {
                set_cell!(x0 + x, y0, '─', lighten(palette[1], 20));
                set_cell!(x0 + x, y0 + 4, '─', lighten(palette[1], 20));
            }
            for y in 0..=4 {
                set_cell!(x0, y0 + y, '│', lighten(palette[1], 20));
                set_cell!(x0 + 7, y0 + y, '│', lighten(palette[1], 20));
            }
            set_cell!(x0, y0, '┌', lighten(palette[1], 35));
            set_cell!(x0 + 7, y0, '┐', lighten(palette[1], 35));
            set_cell!(x0, y0 + 4, '└', lighten(palette[1], 35));
            set_cell!(x0 + 7, y0 + 4, '┘', lighten(palette[1], 35));
            for x in 1..7 {
                for y in 1..4 {
                    if (x + y + p) % 2 == 0 {
                        set_cell!(x0 + x, y0 + y, '□', darken(palette[1], 15));
                    }
                }
            }
        }
        let mast_x = panel_x + 27;
        for y in panel_y - 5..=panel_y + 2 {
            set_cell!(mast_x, y, '│', palette[4]);
        }
        set_cell!(mast_x, panel_y - 6, '◇', lighten(palette[3], 25));
        set_cell!(mast_x - 1, panel_y - 3, '╱', palette[4]);
        set_cell!(mast_x + 1, panel_y - 3, '╲', palette[4]);

        // Perspective rays from the star through the orbital plane.
        for ray in -3..=3 {
            let angle = ray as f32 * 0.18 + 0.9;
            for step in 6..(width / 3).max(8) {
                let x = cx as i32 + (angle.cos() * step as f32 * 1.8).round() as i32;
                let y = cy as i32 + (angle.sin() * step as f32 * 0.55).round() as i32;
                if x >= 0
                    && y >= 0
                    && (x as usize) < width
                    && (y as usize) < height
                    && grid[y as usize][x as usize].ch == ' '
                    && step % 4 == 0
                {
                    set_cell!(x, y, '·', darken(palette[3], 55));
                }
            }
        }
    grid
}

// --- hypercube : multiple seeded 4D cubes projected into terminal space. ---
pub(crate) fn draw_hypercube(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    rng: &mut StdRng,
    t: f32,
    copies: usize,
    speed: f32,
    ghosts: usize,
) {
    use std::f32::consts::TAU;

    if width == 0 || height == 0 {
        return;
    }
    let copies = copies.clamp(1, 5);
    let ghosts = ghosts.min(5);
    let speed = speed.clamp(0.05, 4.0);
    let bg = darken(palette[0], 12);
    let star = darken(palette[4], 62);
    for row in grid.iter_mut() {
        for cell in row.iter_mut() {
            *cell = Cell::new(' ', bg);
        }
    }
    for _ in 0..(width * height / 70).max(3) {
        let x = rng.random_range(0..width);
        let y = rng.random_range(0..height);
        let ch = if rng.random_range(0..5) == 0 { '∙' } else { '·' };
        grid[y][x] = Cell::new(ch, star);
    }

    let phases: Vec<(f32, f32, f32)> = (0..copies)
        .map(|_| {
            (
                rng.random_range(0.0..TAU),
                rng.random_range(0.0..TAU),
                rng.random_range(0.0..TAU),
            )
        })
        .collect();
    let slot_w = width as f32 / copies as f32;
    let edge_colors = [
        lighten(palette[1], 28),
        shift_hue(lighten(palette[2], 30), 42.0),
        shift_hue(lighten(palette[3], 34), -38.0),
        lighten(palette[4], 18),
    ];

    for copy in 0..copies {
        let cx = slot_w * (copy as f32 + 0.5);
        let cy = height as f32 * (0.48 + 0.05 * phases[copy].2.sin());
        let sx = (slot_w * 0.28).clamp(3.0, 17.0);
        let sy = (height as f32 * 0.24).clamp(2.5, 8.0);

        for ghost in (0..=ghosts).rev() {
            let gt = t * speed - ghost as f32 * 0.16;
            let axw = phases[copy].0 + gt * (0.47 + copy as f32 * 0.03);
            let ayz = phases[copy].1 - gt * 0.31;
            let azw = phases[copy].2 + gt * 0.23;
            let axy = phases[copy].1 * 0.35 + gt * 0.17;
            let mut projected = Vec::with_capacity(16);

            for bits in 0..16usize {
                let mut x = if bits & 1 == 0 { -1.0 } else { 1.0 };
                let mut y = if bits & 2 == 0 { -1.0 } else { 1.0 };
                let mut z = if bits & 4 == 0 { -1.0 } else { 1.0 };
                let mut w = if bits & 8 == 0 { -1.0 } else { 1.0 };

                let (c, s) = (axw.cos(), axw.sin());
                (x, w) = (x * c - w * s, x * s + w * c);
                let (c, s) = (ayz.cos(), ayz.sin());
                (y, z) = (y * c - z * s, y * s + z * c);
                let (c, s) = (azw.cos(), azw.sin());
                (z, w) = (z * c - w * s, z * s + w * c);
                let (c, s) = (axy.cos(), axy.sin());
                (x, y) = (x * c - y * s, x * s + y * c);

                let four_d = 1.8 / (2.9 - w * 0.42);
                x *= four_d;
                y *= four_d;
                z *= four_d;
                let three_d = 2.4 / (3.5 - z * 0.34);
                projected.push((
                    (cx + x * three_d * sx).round() as i32,
                    (cy + y * three_d * sy).round() as i32,
                    z,
                ));
            }

            for vertex in 0..16usize {
                for dim in 0..4usize {
                    if vertex & (1 << dim) != 0 {
                        continue;
                    }
                    let other = vertex | (1 << dim);
                    let a = projected[vertex];
                    let b = projected[other];
                    let depth_shade = if (a.2 + b.2) * 0.5 < 0.0 { 20 } else { 0 };
                    let ghost_shade = (ghost * 15 + depth_shade).min(78) as u8;
                    pp_line(
                        grid,
                        a.0,
                        a.1,
                        b.0,
                        b.1,
                        darken(edge_colors[dim], ghost_shade),
                    );
                }
            }

            if ghost == 0 {
                for (i, &(x, y, z)) in projected.iter().enumerate() {
                    let ch = if i == 0 || i == 15 { '◆' } else { '◇' };
                    let color = if z > 0.0 {
                        lighten(palette[4], 26)
                    } else {
                        darken(palette[4], 18)
                    };
                    pp_put(grid, x, y, ch, color);
                }
                pp_put(
                    grid,
                    cx.round() as i32,
                    cy.round() as i32,
                    '⊹',
                    darken(palette[4], 28),
                );
            }
        }
    }
}


// --- flux : seed-stable particles advected through a looping vector field. ---
pub(crate) fn draw_flux(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    rng: &mut StdRng,
    t: f32,
    count: usize,
    trail: usize,
    speed: f32,
) {
    if width == 0 || height == 0 {
        return;
    }
    let count = count.clamp(1, 180);
    let trail = trail.clamp(1, 24);
    let speed = speed.clamp(0.05, 4.0);
    let bg = darken(palette[0], 14);
    for row in grid.iter_mut() {
        for cell in row.iter_mut() {
            *cell = Cell::new(' ', bg);
        }
    }

    // A quiet seeded vector-field lattice makes the flow legible without
    // competing with the bright particle heads.
    for y in (1..height).step_by(4) {
        for x in (2..width).step_by(8) {
            let a = (x as f32 * 0.071 + y as f32 * 0.19 + seed as f32 * 0.013).sin();
            let ch = if a < -0.45 {
                '╲'
            } else if a > 0.45 {
                '╱'
            } else {
                '─'
            };
            grid[y][x] = Cell::new(ch, darken(palette[2], 68));
        }
    }

    for i in 0..count {
        let x0 = rng.random_range(0.0..width as f32);
        let y0 = rng.random_range(0.0..height as f32);
        let phase = rng.random_range(0.0..std::f32::consts::TAU);
        let velocity = rng.random_range(0.72..1.32);
        let curl = rng.random_range(0.65..1.55);
        let base = match i % 4 {
            0 => lighten(palette[1], 24),
            1 => shift_hue(lighten(palette[2], 30), 32.0),
            2 => shift_hue(lighten(palette[3], 34), -36.0),
            _ => lighten(palette[4], 14),
        };
        let position = |time: f32| -> (f32, f32) {
            let flow = time * velocity * 5.2;
            let x = x0
                + flow
                + (y0 * 0.22 + phase + time * 0.63).sin() * 5.5 * curl
                + (phase * 1.7 - time * 0.31).cos() * 1.8;
            let y = y0
                + (x0 * 0.08 - time * 0.71 + phase).sin() * 2.6 * curl
                + (y0 * 0.17 + time * 0.39).cos() * 1.2;
            (x.rem_euclid(width as f32), y.rem_euclid(height as f32))
        };

        for step in (0..=trail).rev() {
            let tau = t * speed - step as f32 * 0.065;
            let p = position(tau);
            let prev = position(tau - 0.025);
            let dx = p.0 - prev.0;
            let dy = p.1 - prev.1;
            let ch = if step == 0 {
                if dx.abs() > dy.abs() {
                    if dx >= 0.0 { '▶' } else { '◀' }
                } else if dy >= 0.0 {
                    '▼'
                } else {
                    '▲'
                }
            } else if step < 3 {
                '•'
            } else if step % 2 == 0 {
                '∙'
            } else {
                '·'
            };
            let shade = if step == 0 {
                0
            } else {
                (10 + step * 66 / trail).min(78) as u8
            };
            pp_put(
                grid,
                p.0.round() as i32,
                p.1.round() as i32,
                ch,
                darken(base, shade),
            );
        }
    }
}


// --- fireworks : phased rockets and ballistic sparks in a seamless loop. ---
pub(crate) fn draw_fireworks(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    rng: &mut StdRng,
    t: f32,
    bursts: usize,
    sparks: usize,
    speed: f32,
) {
    use std::f32::consts::TAU;

    if width == 0 || height == 0 {
        return;
    }
    let bursts = bursts.clamp(1, 14);
    let sparks = sparks.clamp(4, 64);
    let speed = speed.clamp(0.05, 4.0);
    let bg = darken(palette[0], 18);
    for row in grid.iter_mut() {
        for cell in row.iter_mut() {
            *cell = Cell::new(' ', bg);
        }
    }

    for _ in 0..(width * height / 55).max(4) {
        let x = rng.random_range(0..width);
        let y = rng.random_range(0..height.saturating_sub(2).max(1));
        let phase = rng.random_range(0.0..TAU);
        let glow = (t * 0.8 + phase).sin();
        let ch = if glow > 0.72 { '✦' } else if glow > 0.0 { '∙' } else { '·' };
        let col = if glow > 0.72 {
            darken(palette[4], 28)
        } else {
            darken(palette[4], 62)
        };
        grid[y][x] = Cell::new(ch, col);
    }

    // A low, irregular horizon gives the launches a physical origin.
    if height >= 2 {
        for x in 0..width {
            let n = ((x as u64 * 17 + seed * 13) % 11) as usize;
            let y = height - 1 - usize::from(n == 0);
            grid[y][x] = Cell::new(if n == 0 { '▆' } else { '▂' }, darken(palette[1], 64));
        }
    }

    let cycle = 6.2f32;
    let colors = [
        lighten(palette[1], 34),
        shift_hue(lighten(palette[2], 36), 38.0),
        shift_hue(lighten(palette[3], 40), -44.0),
        lighten(palette[4], 22),
    ];
    for burst in 0..bursts {
        let launch_x = rng.random_range(2.0..(width as f32 - 2.0).max(2.1));
        let apex_x = (launch_x + rng.random_range(-7.0..7.0)).clamp(1.0, width as f32 - 2.0);
        let apex_y = rng.random_range(2.0..(height as f32 * 0.48).max(2.1));
        let phase = rng.random_range(0.0..cycle);
        let wind = rng.random_range(-0.34..0.34);
        let gravity = rng.random_range(0.38..0.62);
        let color = colors[burst % colors.len()];
        let spark_specs: Vec<(f32, f32, f32)> = (0..sparks)
            .map(|s| {
                let spoke = s as f32 / sparks as f32 * TAU;
                (
                    spoke + rng.random_range(-0.10..0.10),
                    rng.random_range(3.2..7.4),
                    rng.random_range(0.82..1.18),
                )
            })
            .collect();
        let age = (t * speed + phase).rem_euclid(cycle);

        if age < 1.0 {
            let q = age * age * (3.0 - 2.0 * age);
            let x = launch_x + (apex_x - launch_x) * q;
            let y = (height as f32 - 2.0) + (apex_y - (height as f32 - 2.0)) * q;
            for tail in 1..=5 {
                let ty = y + tail as f32;
                let tx = x - wind * tail as f32 * 0.35;
                pp_put(
                    grid,
                    tx.round() as i32,
                    ty.round() as i32,
                    if tail < 3 { '│' } else { '·' },
                    darken(color, (tail * 12).min(70) as u8),
                );
            }
            pp_put(grid, x.round() as i32, y.round() as i32, '▲', lighten(color, 12));
            continue;
        }

        let explosion_t = age - 1.0;
        if explosion_t < 0.16 {
            pp_put(
                grid,
                apex_x.round() as i32,
                apex_y.round() as i32,
                '✺',
                lighten(color, 18),
            );
        }
        for &(angle, velocity, wobble) in &spark_specs {
            for tail in (0..=3).rev() {
                let et = explosion_t - tail as f32 * 0.11;
                if et < 0.0 {
                    continue;
                }
                let radial = velocity * (1.0 - (-et * 0.55).exp()) / 0.55;
                let x = apex_x + angle.cos() * radial + wind * et * et * 1.8;
                let y = apex_y + angle.sin() * radial * 0.48 * wobble + gravity * et * et;
                let fade = ((explosion_t / 5.2) * 62.0) as usize + tail * 13;
                let ch = if tail == 0 {
                    if angle.cos().abs() > angle.sin().abs() {
                        '━'
                    } else if angle.sin() > 0.0 {
                        '╻'
                    } else {
                        '╹'
                    }
                } else if tail == 1 {
                    '•'
                } else {
                    '·'
                };
                pp_put(
                    grid,
                    x.round() as i32,
                    y.round() as i32,
                    ch,
                    darken(color, fade.min(82) as u8),
                );
            }
        }
    }
}


/// murmuration: starling flocks pouring across a banded dusk sky. Every bird
/// is a pure function of (seed, t): flock centers wander wide slow loops while
/// members ride faster local swirls around them, so the mass knots, splits,
/// and re-forms with no per-frame state. Density picks the glyph -- lone
/// scouts are specks, the packed heart of the flock reads as a solid knot.
pub(crate) fn draw_murmuration(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    rng: &mut StdRng,
    t: f32,
    birds: usize,
    flocks: usize,
    speed: f32,
) {
    use std::f32::consts::TAU;

    if width < 4 || height < 4 {
        return;
    }
    let birds = birds.clamp(8, 600);
    let flocks = flocks.clamp(1, 9);
    let t = t * speed.clamp(0.05, 4.0);

    let zenith = darken(palette[0], 12);
    let horizon = shift_hue(lighten(palette[1], 22), -16.0);
    let span = (height - 1).max(1) as f32;
    let sky_at = |y: usize| {
        let d = y as f32 / span;
        lerp_color(zenith, horizon, d * d * 0.9)
    };
    for y in 0..height {
        let band = sky_at(y);
        for x in 0..width {
            grid[y][x] = Cell::with_bg(' ', band, band);
        }
    }

    // Seeded stars prick through the upper sky; they twinkle but never move.
    for _ in 0..(width * height / 70).max(6) {
        let x = rng.random_range(0..width);
        let y = rng.random_range(0..(height / 2).max(1));
        let phase = rng.random_range(0.0..TAU);
        let tw = (t * 0.9 + phase).sin();
        let ch = if tw > 0.75 {
            '✦'
        } else if tw > 0.0 {
            '∙'
        } else {
            '·'
        };
        grid[y][x] = Cell::with_bg(ch, darken(palette[4], 35), sky_at(y));
    }

    // Flock loops are sized in fractions of the grid so the choreography
    // survives a resize; member swirls are in cells. Flock params are drawn
    // once per flock, then shared by every member.
    struct Flock {
        phase: f32,
        cy0: f32,
        rx: f32,
        ry: f32,
        wx: f32,
        wy: f32,
    }
    let mut specs = Vec::with_capacity(flocks);
    for _ in 0..flocks {
        specs.push(Flock {
            phase: rng.random_range(0.0..TAU),
            cy0: rng.random_range(0.30..0.62),
            rx: rng.random_range(0.16..0.34),
            ry: rng.random_range(0.10..0.20),
            wx: rng.random_range(0.05..0.11),
            wy: rng.random_range(0.04..0.09),
        });
    }

    let mut pos: Vec<(f32, f32)> = Vec::with_capacity(birds);
    for i in 0..birds {
        let f = &specs[i % flocks];
        let cx = width as f32 * (0.5 + f.rx * (t * f.wx + f.phase).cos());
        let cy = height as f32 * (f.cy0 + f.ry * (t * f.wy + f.phase * 1.7).sin());

        let phase = rng.random_range(0.0..TAU);
        let w = rng.random_range(0.5..1.6);
        let rx = rng.random_range(2.0..9.0);
        let ry = rng.random_range(0.8..2.4);
        let a = t * w + phase;
        let x = cx + a.cos() * rx + (a * 2.7).cos() * 1.3;
        let y = cy + a.sin() * ry + (a * 1.9 + phase).sin() * 1.1;
        pos.push((x, y));
    }

    // Density pass: O(n^2) is fine at a few hundred birds.
    for i in 0..birds {
        let (x, y) = pos[i];
        let mut near = 0usize;
        for (j, &(ox, oy)) in pos.iter().enumerate() {
            if i != j && (ox - x).abs() <= 2.5 && (oy - y).abs() <= 1.5 {
                near += 1;
            }
        }
        let ch = if near >= 6 {
            '▓'
        } else if near >= 3 {
            '▒'
        } else if near >= 1 {
            'ˇ'
        } else {
            '·'
        };
        // Silhouettes: pale against the dark zenith, dark against the horizon.
        let d = (y / span).clamp(0.0, 1.0);
        let mut col = lerp_color(lighten(palette[4], 18), darken(palette[4], 46), d);
        if near >= 6 {
            col = darken(col, 20);
        }
        let (xi, yi) = (x.round() as i32, y.round() as i32);
        if xi >= 0 && yi >= 0 && (yi as usize) < height && (xi as usize) < width {
            grid[yi as usize][xi as usize] = Cell::with_bg(ch, col, sky_at(yi as usize));
        }
    }
}


/// lanterns: paper lanterns lifting off dark water. Each lantern rises on its
/// own wrapped cycle with a swaying two-ring halo, a flicker, and a squashed
/// jittered reflection; spawn, drift, and fade are all pure (seed, t).
pub(crate) fn draw_lanterns(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    rng: &mut StdRng,
    t: f32,
    count: usize,
    rise: f32,
    sway: f32,
) {
    use std::f32::consts::TAU;

    if width < 6 || height < 6 {
        return;
    }
    let count = count.clamp(1, 30);
    let rise = rise.clamp(0.1, 4.0);
    let sway = sway.clamp(0.0, 3.0);

    let sky = darken(palette[0], 14);
    for row in grid.iter_mut() {
        for cell in row.iter_mut() {
            *cell = Cell::with_bg(' ', sky, sky);
        }
    }
    let lput = |grid: &mut Grid, x: i32, y: i32, ch: char, col: Color| {
        if x >= 0 && y >= 0 && (y as usize) < height && (x as usize) < width {
            grid[y as usize][x as usize] = Cell::with_bg(ch, col, sky);
        }
    };

    // Sparse high stars with a slow twinkle.
    for _ in 0..(width * height / 90).max(5) {
        let x = rng.random_range(0..width);
        let y = rng.random_range(0..(height * 2 / 3).max(1));
        let phase = rng.random_range(0.0..TAU);
        let ch = if (t * 0.6 + phase).sin() > 0.8 { '✦' } else { '·' };
        lput(grid, x as i32, y as i32, ch, darken(palette[4], 45));
    }

    // Water: horizontal ripple bands, darkening with depth.
    let water_y = (height * 3 / 4).max(1);
    let shallow = darken(palette[1], 48);
    let deep = darken(palette[1], 68);
    for y in water_y..height {
        let d = (y - water_y) as f32 / (height - water_y).max(1) as f32;
        let base = lerp_color(shallow, deep, d);
        for x in 0..width {
            let ripple = (x as f32 * 0.35 + t * 1.2 + y as f32 * 0.9).sin();
            let (ch, col) = if ripple > 0.55 {
                ('≈', lighten(base, 16))
            } else if ripple > 0.10 {
                ('~', base)
            } else {
                (' ', base)
            };
            grid[y][x] = Cell::with_bg(ch, col, sky);
        }
    }

    let cycle = 16.0f32; // seconds for one full ascent
    for i in 0..count {
        let phase = rng.random_range(0.0..cycle);
        let base_x =
            (i as f32 + 0.5) / count as f32 * width as f32 + rng.random_range(-3.0..3.0);
        let sway_amp = rng.random_range(1.5..4.0) * sway;
        let sway_w = rng.random_range(0.4..0.9);
        let sway_ph = rng.random_range(0.0..TAU);
        let flick_w = rng.random_range(5.0..8.0);
        let hue_jit = rng.random_range(-22.0..22.0);

        let age = (t * rise + phase).rem_euclid(cycle);
        let prog = age / cycle;
        // Fade in over the first 8% of the rise, out over the last 12%.
        let vis = (prog / 0.08)
            .clamp(0.0, 1.0)
            .min(((1.0 - prog) / 0.12).clamp(0.0, 1.0));
        let y = water_y as f32 + 1.0 - prog * (water_y as f32 + 3.0);
        let x = base_x + (t * sway_w + sway_ph).sin() * sway_amp;

        let glow = vis * (0.72 + 0.28 * (t * flick_w + sway_ph * 3.0).sin());
        let warm = shift_hue(lighten(palette[1], 28), hue_jit);
        let core = lerp_color(darken(warm, 72), lighten(warm, 32), glow);
        let halo1 = lerp_color(darken(warm, 84), warm, glow * 0.8);
        let halo2 = lerp_color(sky, darken(warm, 55), glow * 0.5);

        let (xi, yi) = (x.round() as i32, y.round() as i32);
        for dy in -2i32..=2 {
            for dx in -2i32..=2 {
                let ring = dx.abs().max(dy.abs());
                if ring == 1 {
                    lput(grid, xi + dx, yi + dy, '∘', halo1);
                } else if ring == 2 && (dx + dy) % 2 == 0 {
                    lput(grid, xi + dx, yi + dy, '·', halo2);
                }
            }
        }
        lput(grid, xi, yi, '◉', core);

        // Reflection: squashed, dimmed, wobbling on the water.
        if y < water_y as f32 {
            let ry = water_y as f32 + ((water_y as f32 - y) * 0.30);
            let rx = x + (t * 2.2 + y).sin() * 1.3;
            let rcol = darken(warm, 58);
            lput(grid, rx.round() as i32, ry.round() as i32, '∘', rcol);
            lput(
                grid,
                (rx + 1.0).round() as i32,
                (ry + 1.0).round() as i32,
                '·',
                darken(rcol, 18),
            );
        }
    }
}


/// tide: surf washing a seeded shore. Sand, speckle, and shells are hashed
/// structure; the waterline is up to four superposed sines of t around a
/// seeded shoreline, and freshly exposed sand stays dark and "wet" by sampling
/// where the front was a beat ago. Pure (seed, t) like every native-T mode.
pub(crate) fn draw_tide(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    rng: &mut StdRng,
    t: f32,
    waves: usize,
    amp: f32,
    speed: f32,
) {
    use std::f32::consts::TAU;

    if width < 6 || height < 6 {
        return;
    }
    let waves = waves.clamp(1, 4);
    let amp = amp.clamp(0.1, 3.0);
    let t = t * speed.clamp(0.05, 4.0);

    // Seeded shoreline and wave set. Wave params are drawn once per front, so
    // the whole seascape replays identically for a given seed.
    let shore_base = height as f32 * 0.42;
    let shore_a1 = rng.random_range(1.0..2.5);
    let shore_a2 = rng.random_range(0.5..1.4);
    let shore_p1 = rng.random_range(0.0..TAU);
    let shore_p2 = rng.random_range(0.0..TAU);
    let shore_at = |x: f32| {
        shore_base + shore_a1 * (x * 0.045 + shore_p1).sin() + shore_a2 * (x * 0.11 + shore_p2).sin()
    };
    let mut wp = [(0.0f32, 0.0f32, 0.0f32); 4];
    for k in 0..waves {
        wp[k] = (
            rng.random_range(1.2..3.2) * amp,
            rng.random_range(0.35..0.80),
            rng.random_range(0.0..TAU),
        );
    }
    let front_at = |x: f32, tt: f32| {
        let mut y = shore_at(x);
        for &(a, w, p) in wp.iter().take(waves) {
            y += a * (tt * w + x * 0.05 + p).sin();
        }
        y
    };

    let sea_deep = darken(palette[1], 74);
    let sea_shallow = darken(palette[1], 58);
    let sea_ink = lighten(palette[1], 10);
    let foam_col = lighten(palette[4], 22);
    let sand_bg = darken(palette[2], 66);
    let sand_wet_bg = darken(palette[2], 78);
    let sand_col = darken(palette[2], 30);
    let shell_col = lighten(palette[3], 18);

    for y in 0..height {
        for x in 0..width {
            let fx = x as f32;
            let dy = y as f32 - front_at(fx, t); // <0 under water, >=0 exposed
            let h = pp_hash2(x as i32, y as i32, seed ^ 0x71DE_71DE);
            let (ch, col, zbg) = if dy < -0.8 {
                // Open water: darker with distance from the front, slow shimmer.
                let depth = (-dy / (height as f32 * 0.35)).clamp(0.0, 1.0);
                let zbg = lerp_color(sea_shallow, sea_deep, depth);
                let shimmer = (fx * 0.30 + t * 1.4 + y as f32 * 0.8).sin();
                if shimmer > 0.45 {
                    ('~', sea_ink, zbg)
                } else if h > 0.90 {
                    ('≈', darken(sea_ink, 30), zbg)
                } else {
                    (' ', zbg, zbg)
                }
            } else if dy < 0.8 {
                // Surf line: bright tumbling foam over a wash-lit bed.
                let tumble = (fx * 0.7 + t * 2.6 + y as f32).sin();
                let zbg = darken(palette[4], 58);
                if tumble > -0.2 {
                    ('≈', foam_col, zbg)
                } else {
                    ('~', darken(foam_col, 18), zbg)
                }
            } else {
                // Sand: wet if the front covered this cell in the last beats.
                let covered_then = front_at(fx, t - 1.1) - y as f32 > -0.8;
                let covered_earlier = front_at(fx, t - 2.2) - y as f32 > -0.8;
                let zbg = if covered_then {
                    sand_wet_bg
                } else if covered_earlier {
                    lerp_color(sand_wet_bg, sand_bg, 0.5)
                } else {
                    sand_bg
                };
                if h > 0.9965 {
                    ('✶', shell_col, zbg)
                } else if h > 0.965 {
                    ('∙', sand_col, zbg)
                } else if h > 0.90 {
                    ('·', darken(sand_col, 18), zbg)
                } else {
                    (' ', zbg, zbg)
                }
            };
            grid[y][x] = Cell::with_bg(ch, col, zbg);
        }
    }
}

pub(crate) fn draw_fireflies(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    rng: &mut StdRng,
    t: f32,
    count: usize,
    glow: f32,
    speed: f32,
) {
    use std::f32::consts::TAU;

    if width < 8 || height < 8 {
        return;
    }
    let count = count.clamp(1, 80);
    let glow_r = glow.clamp(0.2, 3.0) * 2.2;
    let t = t * speed.clamp(0.05, 4.0);

    // Dusk sky: vertical gradient from near-black zenith to a warm horizon.
    let sky_top = darken(palette[0], 78);
    let sky_horizon = darken(palette[0], 42);
    let horizon_y = height as f32 * 0.72;

    // Seeded fly set. Each fly owns a home point, a Lissajous drift, and a blink
    // cycle with its own rate and duty, so the meadow never synchronizes.
    struct Fly {
        bx: f32,
        by: f32,
        ax: f32,
        ay: f32,
        wx: f32,
        wy: f32,
        px: f32,
        py: f32,
        blink: f32,
        phase: f32,
        duty: f32,
    }
    let mut flies: Vec<Fly> = Vec::new();
    for _ in 0..count {
        flies.push(Fly {
            bx: rng.random_range(0.05..0.95) * width as f32,
            by: rng.random_range(0.30..0.70) * height as f32,
            ax: rng.random_range(2.0..9.0),
            ay: rng.random_range(1.5..6.0),
            wx: rng.random_range(0.15..0.5),
            wy: rng.random_range(0.2..0.6),
            px: rng.random_range(0.0..TAU),
            py: rng.random_range(0.0..TAU),
            blink: rng.random_range(0.4..1.1),
            phase: rng.random_range(0.0..TAU),
            duty: rng.random_range(0.25..0.55),
        });
    }

    // Glow field: each lit fly splats a radial falloff into a per-cell buffer;
    // overlapping flies pool additively into brighter patches.
    let mut field = vec![vec![0.0f32; width]; height];
    let mut cores: Vec<(usize, usize, f32)> = Vec::new();
    for f in &flies {
        let x = (f.bx + f.ax * (t * f.wx + f.px).sin()).clamp(1.0, (width - 2) as f32);
        let y = (f.by + f.ay * (t * f.wy + f.py).sin()).clamp(1.0, (height - 3) as f32);
        let s = (t * f.blink + f.phase).sin();
        let on = if s > 0.0 {
            ((s - (1.0 - f.duty)) / f.duty).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let g = on * on;
        if g < 0.02 {
            continue;
        }
        let cx = x as i32;
        let cy = y as i32;
        for dy in -3..=3 {
            for dx in -3..=3 {
                let xx = cx + dx;
                let yy = cy + dy;
                if xx < 0 || yy < 0 || xx >= width as i32 || yy >= height as i32 {
                    continue;
                }
                let d = (dx * dx + dy * dy) as f32;
                let fall = (1.0 - (d.sqrt() / glow_r).min(1.0)).max(0.0);
                field[yy as usize][xx as usize] += g * fall * fall;
            }
        }
        cores.push((cy as usize, cx as usize, g));
    }

    let fly_col = lighten(palette[4], 18);
    let fly_dim = lighten(palette[3], 6);
    let grass_col = darken(palette[2], 52);
    let grass_hi = darken(palette[2], 34);
    let moon_col = lighten(palette[4], 26);
    let star_col = lighten(palette[4], 10);

    // Moon: small seeded disc with a soft halo, upper third of the sky.
    let mx = (rng.random_range(0.15..0.85) * width as f32) as i32;
    let my = (rng.random_range(0.10..0.30) * height as f32) as i32;

    for y in 0..height {
        for x in 0..width {
            let fx = x as f32;
            let fy = y as f32;
            let mut zbg = lerp_color(sky_top, sky_horizon, (fy / horizon_y).clamp(0.0, 1.0));
            if fy > horizon_y {
                zbg = darken(sky_horizon, 14);
            }
            let h = pp_hash2(x as i32, y as i32, seed ^ 0xF1EF_1111);
            let mut ch = ' ';
            let mut col = zbg;

            // Stars: sparse, above the horizon, each twinkling on its own clock.
            if fy < horizon_y - 2.0 && h > 0.9955 {
                let tw = 0.5 + 0.5 * (t * (0.4 + h) + h * TAU).sin();
                if tw > 0.35 {
                    ch = if h > 0.9993 { '✦' } else { '·' };
                    col = lerp_color(darken(star_col, 40), star_col, tw);
                }
            }

            // Moon disc + halo.
            let mdx = fx - mx as f32;
            let mdy = fy - my as f32;
            let md = (mdx * mdx + mdy * mdy).sqrt();
            if md < 2.2 {
                ch = '◉';
                col = moon_col;
            } else if md < 4.5 && ch == ' ' {
                ch = '·';
                col = darken(moon_col, 55);
            }

            // Firefly glow: threshold the pooled field into a char ramp.
            let g = field[y][x];
            if g > 0.06 {
                let step = (g * 4.0).min(3.0) as usize;
                ch = ['·', '∙', '◦', '✦'][step];
                col = if step >= 2 { fly_col } else { fly_dim };
            }

            // Grass silhouette: bottom rows, blades sway on a traveling phase.
            let grass_line = (height - 5) as f32;
            if fy >= grass_line {
                let depth = (fy - grass_line) / 5.0;
                zbg = lerp_color(darken(sky_horizon, 14), darken(palette[2], 70), depth);
                let sway = (fx * 0.35 + t * 1.2).sin() * 0.5 + (fx * 0.13 - t * 0.7).sin() * 0.5;
                let hb = pp_hash2(x as i32, y as i32, seed ^ 0x60A5_5A60);
                let blade = hb + sway * 0.18;
                if blade > 0.78 {
                    ch = if hb > 0.93 { 'ʌ' } else if hb > 0.85 { '|' } else { '/' };
                    col = if hb > 0.90 { grass_hi } else { grass_col };
                } else if blade > 0.62 {
                    ch = '·';
                    col = darken(grass_col, 12);
                } else {
                    ch = ' ';
                    col = zbg;
                }
            }

            grid[y][x] = Cell::with_bg(ch, col, zbg);
        }
    }

    // Fly cores on top: the brightest dot of each lit fly.
    for &(cy, cx, g) in &cores {
        if cy >= height || cx >= width || g < 0.1 {
            continue;
        }
        grid[cy][cx] = Cell::with_bg('✦', lighten(fly_col, 12), grid[cy][cx].bg);
    }
}

pub(crate) fn draw_meteors(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    rng: &mut StdRng,
    t: f32,
    stars: usize,
    rate: f32,
    speed: f32,
) {
    use std::f32::consts::TAU;

    if width < 8 || height < 8 {
        return;
    }
    let stars = stars.clamp(10, 400);
    let rate = rate.clamp(0.2, 4.0);
    let t = t * speed.clamp(0.05, 4.0);

    let sky_top = darken(palette[0], 84);
    let sky_low = darken(palette[0], 62);

    // Starfield: seeded positions, each with its own twinkle clock.
    struct Star {
        x: usize,
        y: usize,
        big: bool,
        ws: f32,
        ph: f32,
    }
    let mut star_list: Vec<Star> = Vec::new();
    for _ in 0..stars {
        star_list.push(Star {
            x: rng.random_range(0..width),
            y: rng.random_range(0..(height * 3 / 5)),
            big: rng.random_bool(0.08),
            ws: rng.random_range(0.3..1.4),
            ph: rng.random_range(0.0..TAU),
        });
    }

    // Mountain silhouettes: two seeded ridgelines, the near one darker.
    let ridge = |amp: f32, base: f32, s1: f32, s2: f32, x: f32| {
        base + amp * (x * 0.021 + s1).sin() + amp * 0.5 * (x * 0.047 + s2).sin()
    };
    let (r1a, r1b, r1p1, r1p2) = (
        rng.random_range(1.5..3.5),
        rng.random_range(0.5..1.6),
        rng.random_range(0.0..TAU),
        rng.random_range(0.0..TAU),
    );
    let (r2a, r2b, r2p1, r2p2) = (
        rng.random_range(2.0..4.5),
        rng.random_range(0.6..2.0),
        rng.random_range(0.0..TAU),
        rng.random_range(0.0..TAU),
    );
    let far_base = height as f32 * 0.78;
    let near_base = height as f32 * 0.88;

    // Meteor schedule: fixed slots, each with its own cycle start, entry point,
    // and fall slope. A slot is live for the first `dur` beats of its cycle, so
    // the sky replays identically for a given seed.
    const SLOTS: usize = 4;
    let cycle = 7.0 / rate;
    let dur = 1.15;
    let mut slots: Vec<(f32, f32, f32, f32, f32)> = Vec::new();
    for i in 0..SLOTS {
        let start = if i == 0 {
            0.0 // the sky always opens mid-streak
        } else {
            rng.random_range(0.0..cycle)
        };
        slots.push((
            start,
            rng.random_range(0.05..0.95) * width as f32,
            rng.random_range(0.05..0.45) * height as f32,
            rng.random_range(0.5..1.1),
            if rng.random_bool(0.5) { 1.0 } else { -1.0 },
        ));
    }

    let star_col = lighten(palette[4], 14);
    let dust_col = darken(palette[4], 46);
    let far_col = darken(palette[2], 68);
    let near_col = darken(palette[2], 78);
    let hot_col = lighten(palette[4], 30);
    let tail_col = lighten(palette[3], 10);

    for y in 0..height {
        for x in 0..width {
            let fx = x as f32;
            let fy = y as f32;
            let mut zbg = lerp_color(sky_top, sky_low, fy / height as f32);

            // Milky way: a diagonal band of dim dust.
            let band_d =
                ((fy / height as f32) - 0.35 - 0.25 * (fx / width as f32)).abs();
            let h = pp_hash2(x as i32, y as i32, seed ^ 0x5E10_5E10);
            let mut ch = ' ';
            let mut col = zbg;
            if band_d < 0.16 && h > 1.0 - 0.05 * (1.0 - band_d / 0.16) {
                ch = '·';
                col = dust_col;
            }

            // Mountains: solid silhouettes, near ridge textured with speckle.
            let far_y = ridge(r1a, far_base, r1p1, r1p2, fx);
            let near_y = ridge(r2a, near_base, r2p1, r2p2, fx);
            if fy >= near_y {
                zbg = darken(near_col, 10);
                col = near_col;
                if h > 0.985 {
                    ch = '·';
                    col = lighten(near_col, 16);
                }
            } else if fy >= far_y {
                zbg = darken(far_col, 8);
                col = far_col;
            }

            grid[y][x] = Cell::with_bg(ch, col, zbg);
        }
    }

    // Stars on top, skipped where a ridge would cover them.
    for s in &star_list {
        if s.y >= height || s.x >= width {
            continue;
        }
        let fy = s.y as f32;
        let fx = s.x as f32;
        if fy >= ridge(r2a, near_base, r2p1, r2p2, fx) {
            continue;
        }
        let tw = 0.5 + 0.5 * (t * s.ws + s.ph).sin();
        let (ch, col) = if s.big {
            ('✦', lerp_color(darken(star_col, 30), star_col, tw))
        } else if tw > 0.25 {
            ('·', lerp_color(darken(star_col, 45), darken(star_col, 10), tw))
        } else {
            continue;
        };
        grid[s.y][s.x] = Cell::with_bg(ch, col, grid[s.y][s.x].bg);
    }

    // Meteors: head + fading tail sampled back along the fall line.
    for &(start, ex, ey, slope, dirx) in &slots {
        let local = (t + start).rem_euclid(cycle);
        if local > dur {
            continue;
        }
        let p = local / dur;
        let fade = 1.0 - p;
        let travel = width as f32 * 0.55;
        let hx = ex + dirx * travel * p;
        let hy = ey + slope * travel * p;
        if hx < -4.0 || hx > (width + 4) as f32 || hy > height as f32 * 0.8 {
            continue;
        }
        let tail_len = (14.0 * fade + 4.0).min(24.0);
        for k in 0..(tail_len as usize) {
            let back = k as f32 / 10.0;
            let txi = (hx - dirx * back) as i32;
            let tyi = (hy - slope * back) as i32;
            if txi < 0 || tyi < 0 || txi >= width as i32 || tyi >= height as i32 {
                continue;
            }
            let f = 1.0 - k as f32 / tail_len;
            if f < 0.05 {
                continue;
            }
            let (ch, col) = if f > 0.85 {
                ('✦', hot_col)
            } else if f > 0.55 {
                ('∙', tail_col)
            } else if f > 0.3 {
                ('·', darken(tail_col, 24))
            } else {
                ('·', darken(tail_col, 44))
            };
            let cur = &mut grid[tyi as usize][txi as usize];
            if cur.ch == ' ' || (cur.ch == '·' && f > 0.5) {
                *cur = Cell::with_bg(ch, col, cur.bg);
            }
        }
        // Head halo: a 3x3 wash around the head, then the hot core.
        let hxi = hx as i32;
        let hyi = hy as i32;
        for dy in -1..=1 {
            for dx in -1..=1 {
                let xx = hxi + dx;
                let yy = hyi + dy;
                if xx < 0 || yy < 0 || xx >= width as i32 || yy >= height as i32 {
                    continue;
                }
                let cur = &mut grid[yy as usize][xx as usize];
                if cur.ch == ' ' {
                    *cur = Cell::with_bg('·', darken(hot_col, 40), cur.bg);
                }
            }
        }
        if hxi >= 0 && hyi >= 0 && hxi < width as i32 && hyi < height as i32 {
            grid[hyi as usize][hxi as usize] =
                Cell::with_bg('✦', lighten(hot_col, 8), grid[hyi as usize][hxi as usize].bg);
        }
    }
}

