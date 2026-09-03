//! haiku-1-torus -- a 3D torus rotating, meridians and parallels drawn
//! with nested glyphs, tilting to reveal topology and depth.

use crate::_0_profile::measure_layer;
use crate::color::*;
use crate::opts::param_f32;
use crate::types::*;
use crossterm::style::Color;
use rand::rngs::StdRng;
use std::f32::consts::PI;

pub(crate) struct TorusKnobs {
    pub speed: f32,
    pub tilt: f32,
    pub scale: f32,
    pub hue: f32,
    pub minor: f32,
    pub density: f32,
}

impl TorusKnobs {
    pub(crate) fn from_env() -> Self {
        TorusKnobs {
            speed: param_f32("SPEED", 1.0),
            tilt: param_f32("TILT", 0.5),
            scale: param_f32("SCALE", 1.0),
            hue: param_f32("HUE", 180.0),
            minor: param_f32("MINOR", 0.35),
            density: param_f32("DENSITY", 1.0),
        }
    }
}

struct Pt3 {
    x: f32,
    y: f32,
    z: f32,
}

fn torus_point(u: f32, v: f32, major: f32, minor_r: f32) -> Pt3 {
    let cu = u.cos();
    let su = u.sin();
    let cv = v.cos();
    let sv = v.sin();
    let r = major + minor_r * cv;
    Pt3 {
        x: r * cu,
        y: minor_r * sv,
        z: r * su,
    }
}

fn rotate_y(p: Pt3, angle: f32) -> Pt3 {
    let ca = angle.cos();
    let sa = angle.sin();
    Pt3 {
        x: p.x * ca - p.z * sa,
        y: p.y,
        z: p.x * sa + p.z * ca,
    }
}

fn rotate_x(p: Pt3, angle: f32) -> Pt3 {
    let ca = angle.cos();
    let sa = angle.sin();
    Pt3 {
        x: p.x,
        y: p.y * ca - p.z * sa,
        z: p.y * sa + p.z * ca,
    }
}

fn project(p: Pt3, w: usize, h: usize) -> (f32, f32) {
    let z_dist = 3.0 + p.z;
    if z_dist < 0.5 {
        return (-999.0, -999.0);
    }
    let scale = 1.0 / (0.3 + 0.7 * z_dist / 4.0);
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    let x = cx + p.x * scale;
    let y = cy - p.y * scale * 0.5;
    (x, y)
}

fn glyph_for_depth(depth: f32, v_norm: f32, density: f32) -> char {
    let r = (depth * 7.0).fract();
    let threshold = 0.3 + 0.7 * (1.0 - depth).max(0.0);
    let threshold = threshold * (0.5 + 0.5 * v_norm) * density;

    if r < threshold * 0.4 {
        '#'
    } else if r < threshold * 0.7 {
        '%'
    } else if r < threshold * 0.85 {
        '='
    } else if r < threshold * 1.0 {
        '+'
    } else if r < 1.0 - (1.0 - threshold) * 0.5 {
        ':'
    } else if r < 1.0 - (1.0 - threshold) * 0.2 {
        '.'
    } else {
        '`'
    }
}

pub(crate) fn draw_haiku_1_torus(grid: &mut Grid, w: usize, h: usize, seed: u64, palette: &[Color; 5], t: f32, k: &TorusKnobs) {
    measure_layer("haiku-1-torus", "clear", || {
        for row in grid.iter_mut().take(h) {
            for cell in row.iter_mut().take(w) {
                *cell = Cell::blank();
            }
        }
    });

    if w < 12 || h < 8 {
        return;
    }

    let major = 1.0 * k.scale.max(0.2);
    let minor_r = major * k.minor.clamp(0.1, 0.6);
    let rot_y = t * k.speed.max(0.0) * PI;
    let tilt_angle = k.tilt.clamp(0.0, PI) * (0.3 + 0.2 * (t * 0.5 * PI).sin());

    let primary_fg = palette[1];
    let accent_fg = palette[3];
    let dim_fg = darken(palette[1], 40);

    measure_layer("haiku-1-torus", "surface", || {
        let meridian_count = ((k.density * 32.0).round() as usize).clamp(8, 64);
        let parallel_count = ((k.density * 16.0).round() as usize).clamp(4, 32);

        let mut depth_map: Vec<Vec<(f32, char, Color)>> = vec![vec![(0.0, ' ', primary_fg); w]; h];

        for m_idx in 0..meridian_count {
            let u = (m_idx as f32 / meridian_count as f32) * 2.0 * PI;

            for p_idx in 0..parallel_count {
                let v = (p_idx as f32 / parallel_count as f32) * 2.0 * PI;
                let v_frac = v / (2.0 * PI);
                let v_norm = ((v_frac - 0.5).abs() * 2.0).clamp(0.0, 1.0);

                let mut p = torus_point(u, v, major, minor_r);
                p = rotate_y(p, rot_y);
                p = rotate_x(p, tilt_angle);

                let depth = (p.z + 3.0) / 6.0;
                if depth < 0.0 || depth > 1.0 {
                    continue;
                }

                let (px, py) = project(p, w, h);
                if px < 0.0 || py < 0.0 || px >= w as f32 || py >= h as f32 {
                    continue;
                }

                let xi = (px + 0.5) as usize;
                let yi = (py + 0.5) as usize;
                if xi >= w || yi >= h {
                    continue;
                }

                let fg = if depth < 0.3 {
                    dim_fg
                } else if depth > 0.65 {
                    accent_fg
                } else {
                    primary_fg
                };

                let ch = glyph_for_depth(depth, v_norm, k.density);

                if depth > depth_map[yi][xi].0 {
                    depth_map[yi][xi] = (depth, ch, fg);
                }
            }
        }

        for y in 0..h {
            for x in 0..w {
                let (_, ch, fg) = depth_map[y][x];
                if ch != ' ' {
                    grid[y][x] = Cell::new(ch, fg);
                }
            }
        }
    });

    measure_layer("haiku-1-torus", "accent_ring", || {
        let u_count = ((k.density * 20.0).round() as usize).clamp(6, 40);
        let v_rim = PI / 4.0;

        for i in 0..u_count {
            let u = (i as f32 / u_count as f32) * 2.0 * PI;

            let mut p = torus_point(u, v_rim, major, minor_r);
            p = rotate_y(p, rot_y);
            p = rotate_x(p, tilt_angle);

            let depth = (p.z + 3.0) / 6.0;
            if depth < 0.1 || depth > 0.95 {
                continue;
            }

            let (px, py) = project(p, w, h);
            if px < 0.0 || py < 0.0 || px >= w as f32 || py >= h as f32 {
                continue;
            }

            let xi = (px + 0.5) as usize;
            let yi = (py + 0.5) as usize;
            if xi >= w || yi >= h {
                continue;
            }

            if grid[yi][xi].ch == ' ' {
                grid[yi][xi] = Cell::new('@', accent_fg);
            }
        }
    });

    measure_layer("haiku-1-torus", "center_pulse", || {
        let pulse = 0.5 + 0.5 * (t * PI).sin();
        let cx = w as f32 / 2.0;
        let cy = h as f32 / 2.0;

        let pulse_r = (pulse * 2.5) as i32;
        for dy in -pulse_r..=pulse_r {
            for dx in -pulse_r..=pulse_r {
                let d2 = dx * dx + dy * dy;
                if d2 <= pulse_r * pulse_r && pulse > 0.2 {
                    let x = ((cx + dx as f32) as usize);
                    let y = ((cy + dy as f32) as usize);
                    if x < w && y < h {
                        let ch = if d2 < 3 { '*' } else { 'o' };
                        grid[y][x] = Cell::new(ch, accent_fg);
                    }
                }
            }
        }
    });

    measure_layer("haiku-1-torus", "frame", || {
        for y in 0..h {
            if grid[y][0].ch == ' ' {
                grid[y][0] = Cell::new('|', dim_fg);
            }
            if grid[y][w - 1].ch == ' ' {
                grid[y][w - 1] = Cell::new('|', dim_fg);
            }
        }
        for x in 0..w {
            if grid[0][x].ch == ' ' {
                grid[0][x] = Cell::new('-', dim_fg);
            }
            if grid[h - 1][x].ch == ' ' {
                grid[h - 1][x] = Cell::new('-', dim_fg);
            }
        }
    });
}

pub(crate) fn cli_haiku_1_torus(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
    let mut knobs = TorusKnobs::from_env();

    if let Some(s) = args.get(4) {
        if let Ok(v) = s.parse::<f32>() {
            knobs.speed = v;
        }
    }
    if let Some(s) = args.get(5) {
        if let Ok(v) = s.parse::<f32>() {
            knobs.tilt = v;
        }
    }
    if let Some(s) = args.get(6) {
        if let Ok(v) = s.parse::<f32>() {
            knobs.scale = v;
        }
    }
    if let Some(s) = args.get(7) {
        if let Ok(v) = s.parse::<f32>() {
            knobs.hue = v;
        }
    }
    if let Some(s) = args.get(8) {
        if let Ok(v) = s.parse::<f32>() {
            knobs.minor = v;
        }
    }
    if let Some(s) = args.get(9) {
        if let Ok(v) = s.parse::<f32>() {
            knobs.density = v;
        }
    }

    draw_haiku_1_torus(&mut grid, width, height, seed, &palette, t_anim, &knobs);
    (grid, false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Grid;

    fn grid_to_string(grid: &Grid) -> String {
        grid.iter()
            .map(|row| row.iter().map(|c| c.ch).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn snapshot_haiku_1_torus_static() {
        let mut grid = vec![vec![Cell::blank(); 80]; 24];
        let palette = crate::color::make_palette(42);
        let knobs = TorusKnobs {
            speed: 0.0,
            tilt: 0.3,
            scale: 1.0,
            hue: 180.0,
            minor: 0.35,
            density: 1.0,
        };
        draw_haiku_1_torus(&mut grid, 80, 24, 42, &palette, 0.0, &knobs);
        let output = grid_to_string(&grid);
        insta::assert_snapshot!("haiku_1_torus_seed42_t0", output);
    }

    #[test]
    fn snapshot_haiku_1_torus_animated() {
        let mut grid = vec![vec![Cell::blank(); 80]; 24];
        let palette = crate::color::make_palette(42);
        let knobs = TorusKnobs {
            speed: 1.0,
            tilt: 0.5,
            scale: 1.0,
            hue: 180.0,
            minor: 0.35,
            density: 1.0,
        };
        draw_haiku_1_torus(&mut grid, 80, 24, 42, &palette, 3.14, &knobs);
        let output = grid_to_string(&grid);
        insta::assert_snapshot!("haiku_1_torus_seed42_t3", output);
    }
}
