//! haiku-2-trees: six novel tree-growth algorithms with gnarled detail, cascading strands,
//! twisted spirals, and visible root systems.

use crate::_0_profile::measure_layer;
use crate::color::*;
use crate::opts::param_f32;
use crate::types::*;
use crossterm::style::Color;
use rand::RngExt;
use rand::SeedableRng;
use rand::rngs::StdRng;

// ── Tree drawing algorithm implementations ───────────────────────────────

fn set(grid: &mut Grid, x: i32, y: i32, ch: char, fg: Color) {
    if x >= 0 && y >= 0 && (y as usize) < grid.len() && (x as usize) < grid[0].len() {
        grid[y as usize][x as usize] = Cell::new(ch, fg);
    }
}

/// Gnarled Oak: thick trunk with visible knots, asymmetric major branches with tapers.
fn draw_gnarled_oak(
    grid: &mut Grid,
    root_x: i32,
    root_y: i32,
    height: i32,
    color: Color,
    rng: &mut StdRng,
) {
    let trunk_color = color;
    let bark_color = darken(color, 15);
    let canopy_color = lighten(color, 20);

    let trunk_w = (height / 12).max(2) as usize;

    // Draw trunk with varying width (taper)
    for i in 0..height {
        let frac = i as f32 / height as f32;
        let w = ((trunk_w as f32) * (1.0 - frac * 0.4)) as usize;

        for j in 0..w {
            let offset = (j as i32) - (w as i32 / 2);
            let ch = if rng.random::<f32>() < 0.1 && frac < 0.7 {
                '●'
            } else if offset == 0 || offset == -1 {
                '│'
            } else {
                '┃'
            };
            set(grid, root_x + offset, root_y - i, ch, if rng.random::<f32>() < 0.15 {
                bark_color
            } else {
                trunk_color
            });
        }

        // Branch opportunities
        if i > height / 4 && i < height - height / 3 && i % 3 == 0 {
            let branch_dir = if rng.random_range(0..2u32) == 0 { -1 } else { 1 };
            let branch_len = ((height - i) / 4).max(2) as i32;
            draw_branch(grid, root_x, root_y - i, branch_dir, branch_len, canopy_color, rng);
        }
    }

    // Crown canopy
    let crown_y = root_y - height;
    let crown_w = trunk_w as i32 * 3;
    for y in crown_y..(crown_y + height / 3) {
        for x in (root_x - crown_w)..(root_x + crown_w) {
            if rng.random::<f32>() < 0.3 + (y - crown_y) as f32 / (height / 3) as f32 * 0.3 {
                set(grid, x, y, '◆', canopy_color);
            }
        }
    }
}

/// Weeping Willow: high anchor with cascading strand layers.
fn draw_weeping_willow(
    grid: &mut Grid,
    root_x: i32,
    root_y: i32,
    height: i32,
    color: Color,
    rng: &mut StdRng,
) {
    let trunk_color = color;
    let strand_color = lighten(color, 15);
    let tip_color = lighten(color, 30);

    // Short main trunk to anchor point
    let anchor_y = root_y - height / 3;
    for i in 0..height / 3 {
        set(grid, root_x, root_y - i, '│', trunk_color);
    }

    // Multiple cascading strands from anchor
    let strand_count = 5;
    for s in 0..strand_count {
        let offset_x = ((s as i32 - strand_count as i32 / 2) * 2);
        let start_x = root_x + offset_x;
        let start_y = anchor_y;
        let strand_len = height / 2 + rng.random_range(0..(height / 8).max(1));

        let mut x = start_x;
        let mut y = start_y;
        for i in 0..strand_len {
            let wave = ((i as f32 / strand_len as f32) * 3.14159).sin() * 2.0;
            let drift = (rng.random::<f32>() - 0.5) * 0.8;
            x = start_x + (wave + drift) as i32;
            y = start_y + i;

            let ch = if i < strand_len / 3 {
                '╱'
            } else if i < strand_len * 2 / 3 {
                '│'
            } else {
                '╲'
            };

            set(
                grid,
                x,
                y,
                ch,
                if i > strand_len - 3 {
                    tip_color
                } else {
                    strand_color
                },
            );
        }
    }
}

/// Twisted Spiral: trunk rotating around itself with branches following the twist.
fn draw_twisted_spiral(
    grid: &mut Grid,
    root_x: i32,
    root_y: i32,
    height: i32,
    color: Color,
    rng: &mut StdRng,
) {
    let trunk_color = color;
    let branch_color = lighten(color, 20);
    let canopy_color = lighten(color, 35);

    let trunk_w = (height / 14).max(1) as f32;
    let turns = 2.5;

    for i in 0..height {
        let frac = i as f32 / height as f32;
        let angle = frac * turns * 6.28318;
        let twist_x = (angle.cos() * trunk_w) as i32;

        // Trunk follows helix
        let trunk_chars = ['│', '╱', '─', '╲'];
        let ch = trunk_chars[(angle as usize / 2) % 4];
        set(grid, root_x + twist_x, root_y - i, ch, trunk_color);

        // Branches perpendicular to twist
        if i > height / 6 && i < height - height / 4 && i % 4 == 0 {
            let branch_dir = if (angle as i32 / 4) % 2 == 0 { 1 } else { -1 };
            let branch_len = ((height - i) / 5).max(2) as i32;
            draw_branch(
                grid,
                root_x + twist_x,
                root_y - i,
                branch_dir,
                branch_len,
                branch_color,
                rng,
            );
        }
    }

    // Spiral canopy
    let crown_y = root_y - height;
    let crown_h = height / 4;
    for i in 0..crown_h {
        let angle = (i as f32 / crown_h as f32) * 6.28318;
        let radius = i as i32 - crown_h as i32 / 2;
        let x = root_x + (angle.cos() * radius as f32) as i32;
        let y = crown_y + i;
        if rng.random::<f32>() < 0.4 {
            set(grid, x, y, '◇', canopy_color);
        }
    }
}

/// Bottle Tree: massive tapered trunk (baobab-like) with small crown.
fn draw_bottle_tree(
    grid: &mut Grid,
    root_x: i32,
    root_y: i32,
    height: i32,
    color: Color,
    rng: &mut StdRng,
) {
    let trunk_color = color;
    let bark_color = darken(color, 20);
    let canopy_color = lighten(color, 25);

    // Massive base taper
    let base_w = (height / 4).max(3) as i32;
    let upper_w = (height / 16).max(1) as i32;

    for i in 0..height {
        let frac = i as f32 / height as f32;
        let ease = 1.0 - (frac * frac); // Ease in: fast taper then slow
        let w = base_w as f32 + (upper_w as f32 - base_w as f32) * (1.0 - ease);
        let w = w as i32;

        for j in -w..=w {
            let ch = if j == 0 {
                '│'
            } else if j.abs() == w {
                if rng.random::<f32>() < 0.2 {
                    '◇'
                } else {
                    '┃'
                }
            } else if rng.random::<f32>() < 0.08 {
                '●'
            } else {
                '┃'
            };

            set(
                grid,
                root_x + j,
                root_y - i,
                ch,
                if rng.random::<f32>() < 0.15 {
                    bark_color
                } else {
                    trunk_color
                },
            );
        }

        // Small branches high up
        if i > height * 2 / 3 && i % 5 == 0 {
            let branch_dir = if rng.random_range(0..2u32) == 0 { -1 } else { 1 };
            draw_branch(grid, root_x, root_y - i, branch_dir, 2, canopy_color, rng);
        }
    }

    // Small crown
    let crown_y = root_y - height;
    for y in crown_y..(crown_y + height / 6) {
        for x in (root_x - 3)..(root_x + 4) {
            if rng.random::<f32>() < 0.25 {
                set(grid, x, y, '◆', canopy_color);
            }
        }
    }
}

/// Root System: visible roots spreading downward from base (inverted tree).
fn draw_root_system(
    grid: &mut Grid,
    root_x: i32,
    root_y: i32,
    height: i32,
    color: Color,
    rng: &mut StdRng,
) {
    let root_color = color;
    let hair_color = lighten(color, 15);
    let soil_color = darken(color, 25);

    // Main taproot
    let taproot_len = height * 3 / 5;
    for i in 0..taproot_len {
        let frac = i as f32 / taproot_len as f32;
        let ch = if rng.random::<f32>() < 0.1 {
            '●'
        } else {
            '│'
        };
        set(
            grid,
            root_x,
            root_y + i,
            ch,
            if frac > 0.7 { soil_color } else { root_color },
        );
    }

    // Lateral roots branching from taproot
    let branch_count = 4;
    for b in 0..branch_count {
        let branch_start = height / 3 + (b as i32) * height / 6;
        let dir = if b % 2 == 0 { -1 } else { 1 };
        let branch_len = (height / 4) + rng.random_range(0..(height / 8).max(1));

        for i in 0..branch_len {
            let x = root_x + dir * i;
            let y = root_y + branch_start + (i / 2);

            let ch = if rng.random::<f32>() < 0.08 {
                '●'
            } else {
                '─'
            };

            set(
                grid,
                x,
                y,
                ch,
                if y > root_y + height / 2 {
                    soil_color
                } else {
                    root_color
                },
            );

            // Root hairs
            if i > 2 && i % 2 == 0 && rng.random::<f32>() < 0.3 {
                let hair_y = y + if rng.random::<bool>() { 1 } else { -1 };
                set(grid, x, hair_y, '·', hair_color);
            }
        }
    }
}

/// Clump Shrub: multiple stems from base forming irregular mass (novel growth).
fn draw_clump_shrub(
    grid: &mut Grid,
    root_x: i32,
    root_y: i32,
    height: i32,
    color: Color,
    rng: &mut StdRng,
) {
    let trunk_color = color;
    let branch_color = lighten(color, 18);
    let canopy_color = lighten(color, 32);

    let stem_count = 5 + rng.random_range(0..3usize);

    for s in 0..stem_count {
        let stem_x = root_x + ((s as i32) - (stem_count as i32 / 2)) * 2;
        let stem_height = height as f32 * (0.7 + rng.random::<f32>() * 0.3);

        for i in 0..(stem_height as i32) {
            let frac = i as f32 / stem_height;
            let wobble = ((s as f32 + i as f32) * 0.3).sin() * 1.5;

            let ch = if rng.random::<f32>() < frac * 0.15 {
                '◆'
            } else {
                '│'
            };

            set(grid, (stem_x as f32 + wobble) as i32, root_y - i, ch, trunk_color);

            // Side branches
            if i > (stem_height / 3.0) as i32 && i % 2 == 0 && rng.random::<f32>() < 0.5 {
                let side_dir = if rng.random::<bool>() { -1 } else { 1 };
                let side_len = ((stem_height - i as f32) / 4.0) as i32;
                for j in 1..=side_len {
                    set(
                        grid,
                        stem_x + side_dir * j,
                        root_y - i,
                        '─',
                        branch_color,
                    );
                }
            }
        }

        // Crown for this stem
        let crown_y = root_y - (stem_height as i32);
        let crown_w = 2 + rng.random_range(0..2usize) as i32;
        for dy in 0..(height / 6) {
            for dx in -crown_w..=crown_w {
                if rng.random::<f32>() < 0.35 {
                    set(
                        grid,
                        stem_x + dx,
                        crown_y + dy,
                        '◇',
                        canopy_color,
                    );
                }
            }
        }
    }
}

fn draw_branch(
    grid: &mut Grid,
    start_x: i32,
    start_y: i32,
    direction: i32,
    length: i32,
    color: Color,
    _rng: &mut StdRng,
) {
    for i in 1..=length {
        let x = start_x + direction * i;
        let ch = if i == length { '╴' } else { '─' };
        set(grid, x, start_y, ch, color);
    }
}

// ── Sample sheet mode ────────────────────────────────────────────────────────

pub struct Haiku2TreesKnobs {
    pub energy: f32,
    pub fruit: f32,
    pub branch: f32,
}

impl Haiku2TreesKnobs {
    pub fn from_env() -> Self {
        Haiku2TreesKnobs {
            energy: param_f32("ENERGY", 0.8),
            fruit: param_f32("FRUIT", 0.3),
            branch: param_f32("BRANCH", 0.5),
        }
    }
}

pub(crate) fn draw_haiku_2_trees(
    grid: &mut Grid,
    width: usize,
    height: usize,
    _seed: u64,
    palette: &[Color; 5],
    _t: f32,
    knobs: &Haiku2TreesKnobs,
) {
    measure_layer("haiku-2-trees", "clear", || {
        for row in grid.iter_mut().take(height) {
            for cell in row.iter_mut().take(width) {
                *cell = Cell::blank();
            }
        }
    });

    measure_layer("haiku-2-trees", "trees", || {
        let labels = ["Gnarled", "Weeping", "Spiral", "Bottle", "Roots", "Clump"];
        let mut rng = StdRng::seed_from_u64(42);
        let cols = labels.len();
        let cell_w = width / cols;
        let cell_h = 20;
        let rows = 2;

        for row in 0..rows {
            let energy = if row == 0 {
                knobs.energy
            } else {
                knobs.energy * 0.55
            };
            let tree_h = (cell_h as f32 * energy) as i32 - 3;

            for i in 0..cols {
                let px = i * cell_w;
                let py = row * cell_h;
                let color = palette[(i + row * 3) % palette.len()];

                let tree_x = (px + cell_w / 2) as i32;
                let tree_y = (py + cell_h - 3) as i32;

                match i {
                    0 => draw_gnarled_oak(grid, tree_x, tree_y, tree_h, color, &mut rng),
                    1 => draw_weeping_willow(grid, tree_x, tree_y, tree_h, color, &mut rng),
                    2 => draw_twisted_spiral(grid, tree_x, tree_y, tree_h, color, &mut rng),
                    3 => draw_bottle_tree(grid, tree_x, tree_y, tree_h, color, &mut rng),
                    4 => draw_root_system(grid, tree_x, tree_y, tree_h, color, &mut rng),
                    _ => draw_clump_shrub(grid, tree_x, tree_y, tree_h, color, &mut rng),
                }

                // Label
                let label = labels[i];
                let lx = px + cell_w / 2 - label.len() / 2;
                let ly = py + cell_h - 1;
                for (j, ch) in label.chars().enumerate() {
                    if lx + j < width && ly < height {
                        grid[ly][lx + j] = Cell::new(ch, darken(color, 20));
                    }
                }

                // Ground line
                if py + cell_h < height {
                    for gx in px..(px + cell_w) {
                        if gx < width {
                            grid[py + cell_h][gx] = Cell::new('─', darken(color, 25));
                        }
                    }
                }
            }
        }
    });
}

pub(crate) fn cli_haiku_2_trees(
    mut grid: Grid,
    width: usize,
    height: usize,
    _seed: u64,
    palette: [Color; 5],
    _rng: StdRng,
    _t_anim: f32,
    _term_w: u16,
    _term_h: u16,
    args: &[String],
    _mode: &str,
    _theme_name: &str,
) -> (Grid, bool) {
    let knobs = Haiku2TreesKnobs {
        energy: args.get(4).and_then(|s| s.parse().ok()).unwrap_or(0.8),
        fruit: args.get(5).and_then(|s| s.parse().ok()).unwrap_or(0.3),
        branch: args.get(6).and_then(|s| s.parse().ok()).unwrap_or(0.5),
    };

    draw_haiku_2_trees(&mut grid, width, height, 42, &palette, 0.0, &knobs);
    (grid, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_haiku_2_trees_snapshot() {
        let mut grid = vec![vec![Cell::blank(); 80]; 24];
        let palette = [
            Color::Rgb { r: 0, g: 0, b: 0 },
            Color::Rgb {
                r: 100,
                g: 200,
                b: 100,
            },
            Color::Rgb {
                r: 150,
                g: 150,
                b: 100,
            },
            Color::Rgb { r: 200, g: 150, b: 100 },
            Color::Rgb { r: 200, g: 200, b: 200 },
        ];

        let knobs = Haiku2TreesKnobs {
            energy: 0.8,
            fruit: 0.3,
            branch: 0.5,
        };

        draw_haiku_2_trees(&mut grid, 80, 24, 42, &palette, 0.0, &knobs);

        let output = grid
            .iter()
            .map(|row| row.iter().map(|c| c.ch).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n");

        insta::assert_snapshot!("haiku_2_trees_seed42", output);
    }
}
