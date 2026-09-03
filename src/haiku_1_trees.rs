//! haiku-1-trees -- six algorithmic tree species on a sample sheet.
use crate::_0_profile::measure_layer;
use crate::color::*;
use crate::gridio::*;
use crate::opts::param_f32;
use crate::types::*;
use crossterm::style::Color;
use rand::{Rng, RngExt};
use rand::SeedableRng;
use rand::rngs::StdRng;

// ── Species ────────────────────────────────────────────────────────────

fn set(grid: &mut Grid, x: i32, y: i32, ch: char, fg: Color) {
    if x >= 0 && y >= 0 && (y as usize) < grid.len() && (x as usize) < grid[0].len() {
        grid[y as usize][x as usize] = Cell::new(ch, fg);
    }
}

/// Spiral: helix trunk with radial branches peeling outward.
fn grow_spiral(
    grid: &mut Grid,
    root_x: i32,
    root_y: i32,
    energy: f32,
    color: Color,
    rng: &mut StdRng,
) {
    let height = ((root_y - 2) as f32 * energy) as i32;
    let spread = ((energy * 8.0) as i32).max(2);

    for i in 0..height {
        let phase = (i as f32 / height as f32 * 4.0 * std::f32::consts::PI).sin();
        let dx = (phase * spread as f32 / 2.0) as i32;
        let new_x = root_x + dx;
        let y = root_y - i;

        set(grid, new_x, y, '│', color);

        if i % 2 == 0 && i > height / 4 {
            let branch_len = (spread - i as i32 / 2).max(1);
            let dir = if ((i as i32 / 2) + dx.abs()) % 2 == 0 { 1 } else { -1 };
            for b in 1..=branch_len {
                let bx = new_x + dir * b;
                let by = y - (b / 3).max(1);
                let branch_col = lighten(color, (12 * b as u8).min(50));
                let ch = if b % 2 == 0 { '─' } else { '╲' };
                set(grid, bx, by, ch, branch_col);
            }
        }
    }
}

/// Cascade: drooping branches following gravity curves.
fn grow_cascade(
    grid: &mut Grid,
    root_x: i32,
    root_y: i32,
    energy: f32,
    color: Color,
    rng: &mut StdRng,
) {
    let height = ((root_y - 2) as f32 * energy) as i32;
    let spread = ((energy * 7.0) as i32).max(2);

    for i in 0..height {
        let y = root_y - i;
        set(grid, root_x, y, '│', color);

        if i % 3 == 0 && i > height / 4 {
            let strand_len = (spread - i as i32 / 4).max(2);
            for s in 0..strand_len {
                let drop = (s as f32 * 1.2).sqrt() as i32;
                let left_strand = (s / 3) as i32;
                let bx_left = root_x - left_strand;
                let bx_right = root_x + left_strand;
                let by = y - drop;
                let strand_col = lighten(color, (12 * s as u8).min(60));
                if left_strand > 0 {
                    set(grid, bx_left, by, '╱', strand_col);
                    set(grid, bx_right, by, '╲', strand_col);
                }
            }
        }
    }
}

/// Thorns: explosive radial spikes from trunk.
fn grow_thorns(
    grid: &mut Grid,
    root_x: i32,
    root_y: i32,
    energy: f32,
    color: Color,
    rng: &mut StdRng,
) {
    let height = ((root_y - 2) as f32 * energy) as i32;
    let spread = ((energy * 8.0) as i32).max(3);

    for i in 0..height {
        let y = root_y - i;
        set(grid, root_x, y, '│', color);

        if i % 2 == 0 && i > height / 5 {
            let spike_len = ((spread - i as i32 / 3).max(2)) as usize;
            let angle_steps = 3 + (i / 4) % 3;

            for angle in 0..angle_steps {
                let rad = std::f32::consts::PI * 2.0 * (angle as f32 / angle_steps as f32);
                let spike_col = lighten(color, (angle as u8 * 15).min(60));

                for s in 1..=spike_len {
                    let bx = root_x + (rad.cos() * (s as f32 / 1.5)) as i32;
                    let by = y - (s as i32 / 2).max(0);
                    let ch = if s == spike_len { '*' } else { '├' };
                    set(grid, bx, by, ch, spike_col);
                }
            }
        }
    }
}

/// Lattice: interconnected grid branches (novel).
fn grow_lattice(
    grid: &mut Grid,
    root_x: i32,
    root_y: i32,
    energy: f32,
    color: Color,
    rng: &mut StdRng,
) {
    let height = ((root_y - 2) as f32 * energy) as i32;
    let spread = ((energy * 6.0) as i32).max(2);

    for i in 0..height {
        let y = root_y - i;
        set(grid, root_x, y, '│', color);

        if i % 3 == 0 && i > height / 5 {
            for s in 1..=(spread / 2) {
                let lattice_col = lighten(color, (s * 10) as u8);
                set(grid, root_x - s, y, '─', lattice_col);
                set(grid, root_x + s, y, '─', lattice_col);

                if i % 6 == 0 {
                    set(grid, root_x - s, y - 2, '╲', lattice_col);
                    set(grid, root_x + s, y - 2, '╱', lattice_col);
                }
            }
        }
    }
}

/// Petrified: crystalline angular geometry (novel).
fn grow_petrified(
    grid: &mut Grid,
    root_x: i32,
    root_y: i32,
    energy: f32,
    color: Color,
    rng: &mut StdRng,
) {
    let height = ((root_y - 2) as f32 * energy) as i32;
    let spread = ((energy * 5.0) as i32).max(2);

    for i in 0..height {
        let y = root_y - i;
        let layer = i / 3;
        let layer_col = lighten(color, ((layer * 8) as u8).min(50));

        set(grid, root_x, y, '╋', color);

        if layer % 2 == 0 {
            for s in 1..=spread {
                let dx = s as i32;
                if (layer + s) % 2 == 0 {
                    set(grid, root_x - dx, y, '╱', layer_col);
                    set(grid, root_x + dx, y, '╲', layer_col);
                }
            }
        } else {
            for s in 1..=(spread - 1) {
                let dx = s as i32;
                set(grid, root_x - dx, y - 1, '╲', layer_col);
                set(grid, root_x + dx, y - 1, '╱', layer_col);
            }
        }
    }
}

/// Strata: layered tiers that thin toward top.
fn grow_strata(
    grid: &mut Grid,
    root_x: i32,
    root_y: i32,
    energy: f32,
    color: Color,
    rng: &mut StdRng,
) {
    let height = ((root_y - 2) as f32 * energy) as i32;
    let spread = ((energy * 8.0) as i32).max(2);

    for i in 0..height {
        let y = root_y - i;
        let tier = i / 4;
        let tier_col = lighten(color, ((tier * 10) as u8).min(50));

        set(grid, root_x, y, '│', color);

        if i % 4 == 0 {
            let tier_spread = (spread - tier as i32).max(1);
            for s in 1..=tier_spread {
                set(grid, root_x - s, y, '─', tier_col);
                set(grid, root_x + s, y, '─', tier_col);
            }

            if tier > 0 {
                set(grid, root_x - tier_spread - 1, y, '╰', tier_col);
                set(grid, root_x + tier_spread + 1, y, '╯', tier_col);
            }
        }
    }
}

// ── Sample sheet ───────────────────────────────────────────────────────

pub(crate) fn draw_haiku_1_trees(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    energy: f32,
    fruit: f32,
    branch: f32,
) {
    let mut rng = StdRng::seed_from_u64(seed);

    measure_layer("haiku-1-trees", "clear", || {
        for row in grid.iter_mut().take(height) {
            row.fill(Cell::blank());
        }
    });

    measure_layer("haiku-1-trees", "species", || {
        let species = [
            ("Spiral", grow_spiral as fn(&mut Grid, i32, i32, f32, Color, &mut StdRng)),
            ("Cascade", grow_cascade),
            ("Thorns", grow_thorns),
            ("Lattice", grow_lattice),
            ("Petrified", grow_petrified),
            ("Strata", grow_strata),
        ];

        let cols = species.len();
        let cell_w = width / cols;
        let cell_h = height.saturating_sub(3);
        let rows = 2usize;

        for row in 0..rows {
            let row_energy = if row == 0 { energy } else { energy * 0.6 };

            for (i, (label, drawer)) in species.iter().enumerate() {
                let px = i * cell_w;
                let py = row * (cell_h / rows);
                let root_y = (py + cell_h / rows - 2) as i32;
                let root_x = (px + cell_w / 2) as i32;

                let color = palette[(i + row * 3) % palette.len()];

                drawer(grid, root_x, root_y, row_energy, color, &mut rng);

                let lx = px + cell_w / 2 - label.len() / 2;
                let ly = py + cell_h / rows - 1;
                for (j, ch) in label.chars().enumerate() {
                    if lx + j < width && ly < height {
                        grid[ly][lx + j] = Cell::new(ch, darken(color, 20));
                    }
                }
            }
        }
    });

    measure_layer("haiku-1-trees", "ground", || {
        let ground_y = height.saturating_sub(1);
        if ground_y < height {
            for x in 0..width {
                grid[ground_y][x] = Cell::new('═', darken(palette[0], 30));
            }
        }
    });
}

pub(crate) struct HaikuTreesKnobs {
    pub energy: f32,
    pub fruit: f32,
    pub branch: f32,
}

impl HaikuTreesKnobs {
    pub(crate) fn from_env() -> Self {
        HaikuTreesKnobs {
            energy: param_f32("ENERGY", 0.85),
            fruit: param_f32("FRUIT", 0.25),
            branch: param_f32("BRANCH", 0.7),
        }
    }
}

pub(crate) fn cli_haiku_1_trees(
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
    let energy: f32 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(0.85);
    let fruit: f32 = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(0.25);
    let branch: f32 = args.get(6).and_then(|s| s.parse().ok()).unwrap_or(0.7);

    draw_haiku_1_trees(&mut grid, width, height, seed, &palette, t_anim, energy, fruit, branch);

    emit_grid(&grid);
    (grid, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_haiku_1_trees_snapshot() {
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

        draw_haiku_1_trees(&mut grid, width, height, 42, &palette, 0.0, 0.85, 0.25, 0.7);

        let output = grid
            .iter()
            .map(|row| row.iter().map(|c| c.ch).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n");

        insta::assert_snapshot!("haiku_1_trees_42", output);
    }
}
