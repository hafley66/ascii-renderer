//! haiku-1-forest -- forest mode with layered depth, atmosphere, and sway animation.
use crate::_0_profile::measure_layer;
use crate::color::*;
use crate::gridio::*;
use crate::opts::param_f32;
use crate::types::*;
use crossterm::style::Color;
use rand::{Rng, RngExt};
use rand::SeedableRng;
use rand::rngs::StdRng;

fn set(grid: &mut Grid, x: i32, y: i32, ch: char, fg: Color) {
    if x >= 0 && y >= 0 && (y as usize) < grid.len() && (x as usize) < grid[0].len() {
        grid[y as usize][x as usize] = Cell::new(ch, fg);
    }
}

/// Draw a simple tree for the forest.
fn draw_tree(
    grid: &mut Grid,
    x: i32,
    y: i32,
    height: i32,
    width: i32,
    color: Color,
    rng: &mut StdRng,
) {
    for i in 0..height {
        let iy = y - i;
        set(grid, x, iy, '│', color);

        if i % 2 == 0 && i > height / 3 {
            let spread = (width - i as i32 / 2).max(1);
            for s in 1..=spread {
                let leaf_col = lighten(color, (s * 8) as u8);
                set(grid, x - s, iy, '╱', leaf_col);
                set(grid, x + s, iy, '╲', leaf_col);
            }
        }
    }
}

/// Draw layer of trees at given depth level.
fn draw_layer(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    layer: usize,
    density: f32,
    color: Color,
    t: f32,
    sway: f32,
) {
    let mut rng = StdRng::seed_from_u64(seed.wrapping_add(layer as u64));
    let tree_h = ((10 + layer * 2) as f32 * density) as i32;
    let tree_spacing = ((8 - layer as i32).max(2)) as usize;
    let y_base = (height as f32 * (0.7 + layer as f32 * 0.1)) as i32;

    let mut x = 0;
    while x < width {
        let x_int = x as i32;
        let sway_offset = if t > 0.0 {
            ((t * 0.5 + (x as f32 * 0.01)).sin() * sway * (2 - layer as i32) as f32) as i32
        } else {
            0
        };

        draw_tree(grid, x_int + sway_offset, y_base, tree_h, 3 + layer as i32, color, &mut rng);

        x += tree_spacing + rng.random_range(1..=2) as usize;
    }
}

/// Draw mist that drifts and occludes.
fn draw_mist(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    t: f32,
    opacity: f32,
) {
    let mut rng = StdRng::seed_from_u64(seed);
    let mist_chars = ['·', '∙', '°'];

    for y in height / 2..height * 3 / 4 {
        let phase = t * 0.3 + (y as f32 * 0.02);
        let drift = (phase.sin() * 5.0) as i32;

        for x in 0..width {
            if rng.random::<f32>() < opacity * 0.3 {
                let char_idx = (x + y) % mist_chars.len();
                let mist_x = ((x as i32 + drift) % width as i32).max(0) as usize;
                set(
                    grid,
                    mist_x as i32,
                    y as i32,
                    mist_chars[char_idx],
                    Color::Rgb {
                        r: 100,
                        g: 100,
                        b: 100,
                    },
                );
            }
        }
    }
}

/// Draw ground.
fn draw_ground(grid: &mut Grid, width: usize, height: usize, palette: &[Color; 5]) {
    let ground_y = height - 1;
    for x in 0..width {
        grid[ground_y][x] = Cell::new('═', palette[0]);
    }
}

/// Draw sky with light cycle.
fn draw_sky(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    t: f32,
    palette: &[Color; 5],
) {
    let cycle = t.sin() * 0.5 + 0.5;
    let sky_col = if cycle < 0.2 {
        Color::Rgb { r: 40, g: 20, b: 50 }
    } else if cycle < 0.4 {
        Color::Rgb { r: 60, g: 40, b: 80 }
    } else if cycle < 0.6 {
        Color::Rgb { r: 80, g: 100, b: 140 }
    } else if cycle < 0.8 {
        Color::Rgb { r: 100, g: 120, b: 100 }
    } else {
        Color::Rgb { r: 40, g: 20, b: 50 }
    };

    for y in 0..height / 3 {
        for x in 0..width {
            if grid[y][x].ch == ' ' {
                grid[y][x] = Cell::new('·', sky_col);
            }
        }
    }
}

pub(crate) fn draw_haiku_1_forest(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    density: f32,
    layers: u32,
    sway: f32,
    speed: f32,
    hue: f32,
    atmos: f32,
) {
    let effective_t = t * speed;

    measure_layer("haiku-1-forest", "clear", || {
        for row in grid.iter_mut().take(height) {
            row.fill(Cell::blank());
        }
    });

    measure_layer("haiku-1-forest", "sky", || {
        draw_sky(grid, width, height, seed, effective_t, palette);
    });

    measure_layer("haiku-1-forest", "layers", || {
        let layer_count = (layers as usize).min(4).max(2);
        for layer_idx in 0..layer_count {
            let layer_color = lighten(palette[1], ((layer_idx as u8) * 20).min(50));
            let layer_density = density / (1.0 + layer_idx as f32);
            draw_layer(
                grid,
                width,
                height,
                seed.wrapping_add(layer_idx as u64),
                layer_idx,
                layer_density,
                layer_color,
                effective_t * 0.5 * (1.0 - layer_idx as f32 * 0.3),
                sway,
            );
        }
    });

    measure_layer("haiku-1-forest", "atmosphere", || {
        if atmos > 0.0 {
            draw_mist(grid, width, height, seed, effective_t, atmos);
        }
    });

    measure_layer("haiku-1-forest", "ground", || {
        draw_ground(grid, width, height, palette);
    });
}

pub(crate) struct HaikuForestKnobs {
    pub density: f32,
    pub layers: u32,
    pub sway: f32,
    pub speed: f32,
    pub hue: f32,
    pub atmos: f32,
}

impl HaikuForestKnobs {
    pub(crate) fn from_env() -> Self {
        HaikuForestKnobs {
            density: param_f32("DENSITY", 0.8),
            layers: param_f32("LAYERS", 3.0) as u32,
            sway: param_f32("SWAY", 0.5),
            speed: param_f32("SPEED", 0.3),
            hue: param_f32("HUE", 0.5),
            atmos: param_f32("ATMOS", 0.4),
        }
    }
}

pub(crate) fn cli_haiku_1_forest(
    mut grid: Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: [Color; 5],
    mut rng: StdRng,
    t_anim: f32,
    term_w: u16,
    term_h: u16,
    args: &[String],
    mode: &str,
    theme_name: &str,
) -> (Grid, bool) {
    let density: f32 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(0.8);
    let layers: u32 = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(3);
    let sway: f32 = args.get(6).and_then(|s| s.parse().ok()).unwrap_or(0.5);
    let speed: f32 = args.get(7).and_then(|s| s.parse().ok()).unwrap_or(0.3);
    let hue: f32 = args.get(8).and_then(|s| s.parse().ok()).unwrap_or(0.5);
    let atmos: f32 = args.get(9).and_then(|s| s.parse().ok()).unwrap_or(0.4);

    draw_haiku_1_forest(
        &mut grid, width, height, seed, &palette, t_anim, density, layers, sway, speed, hue,
        atmos,
    );

    emit_grid(&grid);
    (grid, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_haiku_1_forest_snapshot() {
        let width = 80;
        let height = 24;
        let mut grid = vec![vec![Cell::blank(); width]; height];
        let palette = [
            Color::Rgb {
                r: 20,
                g: 20,
                b: 20,
            },
            Color::Rgb {
                r: 100,
                g: 180,
                b: 120,
            },
            Color::Rgb {
                r: 150,
                g: 200,
                b: 100,
            },
            Color::Rgb {
                r: 200,
                g: 150,
                b: 80,
            },
            Color::Rgb {
                r: 220,
                g: 220,
                b: 220,
            },
        ];

        draw_haiku_1_forest(
            &mut grid, width, height, 42, &palette, 0.0, 0.8, 3, 0.5, 0.3, 0.5, 0.4,
        );

        let output = grid
            .iter()
            .map(|row| row.iter().map(|c| c.ch).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n");

        insta::assert_snapshot!("haiku_1_forest_42", output);
    }

    #[test]
    fn test_haiku_1_forest_animated_snapshot() {
        let width = 80;
        let height = 24;
        let mut grid = vec![vec![Cell::blank(); width]; height];
        let palette = [
            Color::Rgb {
                r: 20,
                g: 20,
                b: 20,
            },
            Color::Rgb {
                r: 100,
                g: 180,
                b: 120,
            },
            Color::Rgb {
                r: 150,
                g: 200,
                b: 100,
            },
            Color::Rgb {
                r: 200,
                g: 150,
                b: 80,
            },
            Color::Rgb {
                r: 220,
                g: 220,
                b: 220,
            },
        ];

        draw_haiku_1_forest(
            &mut grid, width, height, 42, &palette, 3.0, 0.8, 3, 0.5, 0.3, 0.5, 0.4,
        );

        let output = grid
            .iter()
            .map(|row| row.iter().map(|c| c.ch).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n");

        insta::assert_snapshot!("haiku_1_forest_42_t3", output);
    }
}
