//! haiku-2-forest: composed forest with depth layers, parallax sway, falling leaves,
//! and a slow light cycle.

use crate::_0_profile::measure_layer;
use crate::color::*;
use crate::opts::param_f32;
use crate::types::*;
use crossterm::style::Color;
use rand::RngExt;
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::f32::consts::PI;

fn set(grid: &mut Grid, x: i32, y: i32, ch: char, fg: Color) {
    if x >= 0 && y >= 0 && (y as usize) < grid.len() && (x as usize) < grid[0].len() {
        grid[y as usize][x as usize] = Cell::new(ch, fg);
    }
}

fn set_with_bg(grid: &mut Grid, x: i32, y: i32, ch: char, fg: Color, bg: Color) {
    if x >= 0 && y >= 0 && (y as usize) < grid.len() && (x as usize) < grid[0].len() {
        grid[y as usize][x as usize] = Cell::with_bg(ch, fg, bg);
    }
}

fn blank_at(grid: &Grid, x: i32, y: i32) -> bool {
    x >= 0
        && y >= 0
        && (y as usize) < grid.len()
        && (x as usize) < grid[0].len()
        && grid[y as usize][x as usize].ch == ' '
}

// ── Simple tree drawing for forest ───────────────────────────────────────

fn draw_simple_tree(
    grid: &mut Grid,
    x: i32,
    y: i32,
    h: i32,
    color: Color,
    tree_type: usize,
    rng: &mut StdRng,
) {
    let trunk_color = color;
    let canopy_color = lighten(color, 20);

    match tree_type % 6 {
        0 => {
            // Pine: triangular
            for i in 0..h {
                let w = (h - i) / 3;
                for j in -w..=w {
                    if rng.random::<f32>() < 0.4 {
                        set(grid, x + j, y - i, '▓', canopy_color);
                    }
                }
            }
        }
        1 => {
            // Oak: round
            for i in 0..h {
                let r = (h / 3) as f32;
                for j in (-(h / 4))..=(h / 4) {
                    let dist = ((i as f32) * (i as f32) + (j as f32) * (j as f32)).sqrt();
                    if dist < r && rng.random::<f32>() < 0.45 {
                        set(grid, x + j, y - i, '◆', canopy_color);
                    }
                }
            }
        }
        2 => {
            // Willow: drooping
            for i in 0..h {
                let droop = (i as f32 / h as f32) * 2.0;
                let x_off = (droop.sin() * 2.0) as i32;
                if rng.random::<f32>() < 0.35 {
                    set(grid, x + x_off, y - i, '╱', canopy_color);
                }
            }
        }
        3 => {
            // Narrow: cypress
            for i in 0..h {
                if rng.random::<f32>() < 0.3 {
                    set(grid, x, y - i, '│', canopy_color);
                }
                if i % 3 == 0 && rng.random::<f32>() < 0.5 {
                    set(grid, x - 1, y - i, '├', canopy_color);
                    set(grid, x + 1, y - i, '┤', canopy_color);
                }
            }
        }
        4 => {
            // Bottle: thick taper
            for i in 0..h {
                let frac = i as f32 / h as f32;
                let w = ((h / 6) as f32 * (1.0 - frac)).max(1.0) as i32;
                for j in -w..=w {
                    if rng.random::<f32>() < 0.4 {
                        set(grid, x + j, y - i, '◇', canopy_color);
                    }
                }
            }
        }
        _ => {
            // Clump: irregular
            for i in 0..h {
                for j in -2..=2 {
                    if rng.random::<f32>() < 0.35 {
                        set(grid, x + j, y - i, '●', canopy_color);
                    }
                }
            }
        }
    }

    // Trunk
    for i in 0..(h / 3) {
        set(grid, x, y, '│', trunk_color);
    }
}

// ── Forest composition ───────────────────────────────────────────────────

pub struct Haiku2ForestKnobs {
    pub density: f32,
    pub layers: f32,
    pub sway: f32,
    pub speed: f32,
    pub hue: f32,
    pub atmos: f32,
}

impl Haiku2ForestKnobs {
    pub fn from_env() -> Self {
        Haiku2ForestKnobs {
            density: param_f32("DENSITY", 0.6),
            layers: param_f32("LAYERS", 0.8),
            sway: param_f32("SWAY", 0.15),
            speed: param_f32("SPEED", 0.5),
            hue: param_f32("HUE", 0.5),
            atmos: param_f32("ATMOS", 0.7),
        }
    }
}

pub(crate) fn draw_haiku_2_forest(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    knobs: &Haiku2ForestKnobs,
) {
    measure_layer("haiku-2-forest", "sky", || {
        // Sky gradient with slow light cycle
        let cycle = (t * knobs.speed).sin() * 0.5 + 0.5;
        let sky_color = if cycle < 0.33 {
            // Dawn: dark blue to purple
            lighten(darken(palette[0], 20), 5)
        } else if cycle < 0.66 {
            // Day: bright
            palette[0]
        } else {
            // Dusk: orange-brown
            darken(palette[4], 10)
        };

        for row in grid.iter_mut().take((height as f32 * 0.3) as usize) {
            for cell in row.iter_mut().take(width) {
                *cell = Cell::with_bg(' ', sky_color, sky_color);
            }
        }
    });

    let mut rng = StdRng::seed_from_u64(seed.wrapping_add(1));

    measure_layer("haiku-2-forest", "far_layer", || {
        // Far trees: small, dense, high on screen
        let far_h = (height as f32 * 0.15) as i32;
        let tree_spacing = ((width as f32) / (knobs.density * 12.0)) as usize;
        let far_color = darken(palette[1], 25);

        let sway_phase = (t * knobs.speed * 0.5).sin();
        let far_sway = sway_phase * 2.0 * knobs.sway;

        for tx in (0..width).step_by(tree_spacing.max(1)) {
            let swayed_x = ((tx as f32) + far_sway) as i32;
            let tree_y = (height as f32 * 0.25) as i32;
            let tree_type = rng.random_range(0..6usize);
            draw_simple_tree(grid, swayed_x, tree_y, far_h, far_color, tree_type, &mut rng);
        }
    });

    measure_layer("haiku-2-forest", "mid_layer", || {
        // Mid trees: medium size, medium density, parallax less than far
        let mid_h = (height as f32 * 0.25) as i32;
        let tree_spacing = ((width as f32) / (knobs.density * 8.0)) as usize;
        let mid_color = palette[1];

        let sway_phase = (t * knobs.speed * 0.6).sin();
        let mid_sway = sway_phase * 3.0 * knobs.sway;

        for tx in (0..width).step_by(tree_spacing.max(1)) {
            let swayed_x = ((tx as f32) + mid_sway) as i32;
            let tree_y = (height as f32 * 0.45) as i32;
            let tree_type = rng.random_range(0..6usize);
            draw_simple_tree(grid, swayed_x, tree_y, mid_h, mid_color, tree_type, &mut rng);
        }
    });

    measure_layer("haiku-2-forest", "near_layer", || {
        // Near trees: large, sparse, parallax most
        let near_h = (height as f32 * 0.35) as i32;
        let tree_spacing = ((width as f32) / (knobs.density * 4.0)) as usize;
        let near_color = lighten(palette[1], 10);

        let sway_phase = (t * knobs.speed * 0.7).sin();
        let near_sway = sway_phase * 4.0 * knobs.sway;

        for tx in (0..width).step_by(tree_spacing.max(1)) {
            let swayed_x = ((tx as f32) + near_sway) as i32;
            let tree_y = (height as f32 * 0.65) as i32;
            let tree_type = rng.random_range(0..6usize);
            draw_simple_tree(grid, swayed_x, tree_y, near_h, near_color, tree_type, &mut rng);
        }
    });

    measure_layer("haiku-2-forest", "ground", || {
        // Ground: dark soil with grass
        let ground_y = (height as f32 * 0.75) as usize;
        let ground_color = darken(palette[3], 30);
        let grass_color = palette[3];

        for y in ground_y..height {
            for x in 0..width {
                let ch = if (x + y) % 3 == 0 { '─' } else { '▒' };
                set(grid, x as i32, y as i32, ch, grass_color);
            }
        }
    });

    measure_layer("haiku-2-forest", "atmosphere", || {
        // Falling leaves with animation
        let leaf_count = ((width * height) as f32 * knobs.atmos * 0.005) as usize;

        for leaf_idx in 0..leaf_count {
            let leaf_seed = seed.wrapping_add(leaf_idx as u64).wrapping_add(1000);
            let mut leaf_rng = StdRng::seed_from_u64(leaf_seed);

            // Base position seeded
            let base_x = leaf_rng.random::<f32>() * width as f32;
            let fall_phase = t * knobs.speed;

            // Vertical drift over time
            let drift_period = 4.0 + (leaf_idx as f32 * 0.17) % 3.0;
            let y_offset = (fall_phase % drift_period) * (height as f32 / drift_period);

            // Horizontal sway
            let wave = (fall_phase + leaf_idx as f32 * 0.1).sin() * 3.0;
            let x = base_x + wave;
            let y = (y_offset as i32).max(0) as usize;

            if y < height {
                let leaf_ch = ['˙', '·', '∙', 'ˇ'][leaf_idx % 4];
                set(grid, x as i32, y as i32, leaf_ch, lighten(palette[2], 15));
            }
        }
    });

    measure_layer("haiku-2-forest", "horizon_mist", || {
        // Mist occlusion over mid/far trees
        let mist_strength = knobs.atmos * 0.3;
        let mist_wave = (t * knobs.speed * 0.3).sin() * 0.5 + 0.5;
        let mist_y = (height as f32 * (0.3 + mist_wave * 0.15)) as usize;

        for y in (mist_y)..(mist_y + 4).min(height) {
            for x in 0..width {
                let fade = (y - mist_y) as f32 / 4.0;
                if rng.random::<f32>() < mist_strength * (1.0 - fade) {
                    let current = grid[y][x];
                    if current.ch != ' ' {
                        let dimmed = darken(current.fg, 20);
                        set(grid, x as i32, y as i32, current.ch, dimmed);
                    }
                }
            }
        }
    });
}

pub(crate) fn cli_haiku_2_forest(
    mut grid: Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: [Color; 5],
    _rng: StdRng,
    t_anim: f32,
    _term_w: u16,
    _term_h: u16,
    args: &[String],
    _mode: &str,
    _theme_name: &str,
) -> (Grid, bool) {
    let density: f32 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(0.6);
    let layers: f32 = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(0.8);
    let sway: f32 = args.get(6).and_then(|s| s.parse().ok()).unwrap_or(0.15);
    let speed: f32 = args.get(7).and_then(|s| s.parse().ok()).unwrap_or(0.5);
    let hue: f32 = args.get(8).and_then(|s| s.parse().ok()).unwrap_or(0.5);
    let atmos: f32 = args.get(9).and_then(|s| s.parse().ok()).unwrap_or(0.7);

    let knobs = Haiku2ForestKnobs {
        density,
        layers,
        sway,
        speed,
        hue,
        atmos,
    };

    draw_haiku_2_forest(&mut grid, width, height, seed, &palette, t_anim, &knobs);
    (grid, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_haiku_2_forest_snapshot() {
        let mut grid = vec![vec![Cell::blank(); 80]; 24];
        let palette = [
            Color::Rgb { r: 0, g: 0, b: 0 },
            Color::Rgb {
                r: 50,
                g: 100,
                b: 50,
            },
            Color::Rgb { r: 100, g: 150, b: 80 },
            Color::Rgb { r: 150, g: 100, b: 50 },
            Color::Rgb { r: 200, g: 200, b: 200 },
        ];

        let knobs = Haiku2ForestKnobs {
            density: 0.6,
            layers: 0.8,
            sway: 0.15,
            speed: 0.5,
            hue: 0.5,
            atmos: 0.7,
        };

        draw_haiku_2_forest(&mut grid, 80, 24, 42, &palette, 0.0, &knobs);

        let output = grid
            .iter()
            .map(|row| row.iter().map(|c| c.ch).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n");

        insta::assert_snapshot!("haiku_2_forest_seed42", output);
    }

    #[test]
    fn test_haiku_2_forest_animated_snapshot() {
        let mut grid = vec![vec![Cell::blank(); 80]; 24];
        let palette = [
            Color::Rgb { r: 0, g: 0, b: 0 },
            Color::Rgb {
                r: 50,
                g: 100,
                b: 50,
            },
            Color::Rgb { r: 100, g: 150, b: 80 },
            Color::Rgb { r: 150, g: 100, b: 50 },
            Color::Rgb { r: 200, g: 200, b: 200 },
        ];

        let knobs = Haiku2ForestKnobs {
            density: 0.6,
            layers: 0.8,
            sway: 0.15,
            speed: 0.5,
            hue: 0.5,
            atmos: 0.7,
        };

        draw_haiku_2_forest(&mut grid, 80, 24, 42, &palette, 5.0, &knobs);

        let output = grid
            .iter()
            .map(|row| row.iter().map(|c| c.ch).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n");

        insta::assert_snapshot!("haiku_2_forest_t5", output);
    }
}
