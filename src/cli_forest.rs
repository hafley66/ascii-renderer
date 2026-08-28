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


/// Dispatch arm for mode(s): forest2 (moved verbatim from run()).
pub(crate) fn cli_forest2(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        // Ground: truchet dirt
        let ground_color = darken(palette[1], 90);
        let tiles = ['╱', '╲'];
        for y in 0..height {
            for x in 0..width {
                grid[y][x] = Cell::new(tiles[rng.random_range(0..2)], ground_color);
            }
        }

        let ground_y = height.saturating_sub(4);

        // Place trees with varied size, position, and type
        let tree_count = rng.random_range(4..9u32) as usize;
        struct TreeSlot {
            x: usize,
            kind: usize,
            spread: usize,
            canopy_y: usize,
        }
        let mut slots: Vec<TreeSlot> = Vec::new();

        // One big centerpiece tree
        let big_x = rng.random_range((width / 4) as u32..(width * 3 / 4) as u32) as usize;
        let big_spread = rng.random_range(8..14u32) as usize;
        let big_canopy = rng.random_range(3..6u32) as usize;
        let big_kind = rng.random_range(0..12u32) as usize;
        slots.push(TreeSlot {
            x: big_x,
            kind: big_kind,
            spread: big_spread,
            canopy_y: big_canopy,
        });

        // Remaining trees scattered, varied sizes
        for _ in 0..tree_count - 1 {
            let tx = rng.random_range(6..(width - 6) as u32) as usize;
            let spread = rng.random_range(3..9u32) as usize;
            let canopy = rng.random_range(4..ground_y.saturating_sub(6).max(5) as u32) as usize;
            let kind = rng.random_range(0..12u32) as usize;
            slots.push(TreeSlot {
                x: tx,
                kind: kind,
                spread: spread,
                canopy_y: canopy,
            });
        }

        // Sort by x so they layer left to right
        slots.sort_by_key(|s| s.x);

        for slot in &slots {
            // Clear space for this tree
            let clear_left = slot.x.saturating_sub(slot.spread + 2);
            let clear_right = (slot.x + slot.spread + 2).min(width);
            for y in slot.canopy_y.saturating_sub(1)..ground_y + 2 {
                for x in clear_left..clear_right {
                    if y < height && x < width {
                        grid[y][x] = Cell::blank();
                    }
                }
            }
            let color = palette[rng.random_range(1..4)];
            draw_tree(
                &mut grid,
                slot.x,
                ground_y - 1,
                slot.canopy_y,
                slot.spread,
                slot.kind,
                color,
                &mut rng,
            );
        }

        // Flower/fruit burst radiating from the biggest tree's base
        let burst_cx = big_x;
        let burst_cy = ground_y + 1;
        let burst_count = rng.random_range(5..12u32);
        // One big flower at center
        draw_flower(
            &mut grid,
            burst_cx,
            burst_cy,
            rng.random_range(0..5),
            palette[3],
        );
        // Radial scatter around it
        for _ in 0..burst_count {
            let angle = rng.random::<f32>() * std::f32::consts::TAU;
            let radius = rng.random_range(3..16u32) as f32;
            let fx = (burst_cx as f32 + angle.cos() * radius * 1.8) as i32; // aspect correction
            let fy = (burst_cy as f32 + angle.sin() * radius * 0.6) as i32;
            if fx >= 2 && fy >= 2 && (fx as usize) < width - 2 && (fy as usize) < height - 2 {
                if rng.random_range(0..3u32) == 0 {
                    draw_fruit(
                        &mut grid,
                        fx as usize,
                        fy as usize,
                        rng.random_range(0..5),
                        palette[rng.random_range(2..4)],
                    );
                } else {
                    draw_flower(
                        &mut grid,
                        fx as usize,
                        fy as usize,
                        rng.random_range(0..5),
                        palette[rng.random_range(2..4)],
                    );
                }
            }
        }

        // Scatter a few more flower clusters near other trees
        for slot in &slots {
            let count = rng.random_range(1..4u32);
            for _ in 0..count {
                let fx = (slot.x as i32 + rng.random_range(-6..7i32)) as usize;
                let fy = ground_y + rng.random_range(0..2u32) as usize;
                if fx >= 2 && fx < width - 2 && fy < height - 2 {
                    if rng.random_range(0..2u32) == 0 {
                        draw_flower(
                            &mut grid,
                            fx,
                            fy,
                            rng.random_range(0..5),
                            palette[rng.random_range(2..4)],
                        );
                    } else {
                        draw_fruit(
                            &mut grid,
                            fx,
                            fy,
                            rng.random_range(0..5),
                            palette[rng.random_range(2..4)],
                        );
                    }
                }
            }
        }
    (grid, false)
}

/// Dispatch arm for mode(s): forest3 (moved verbatim from run()).
pub(crate) fn cli_forest3(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        // Background: sky (sparse dots) above horizon, ground (truchet) below
        let horizon = height * 2 / 3 + rng.random_range(0..(height / 8).max(1) as u32) as usize;
        let sky_color = darken(palette[0], 95);
        let ground_color = darken(palette[1], 85);
        let ground_tiles = ['╱', '╲', '·', '·'];

        // Sky: sparse scattered dots
        for y in 0..horizon {
            for x in 0..width {
                if rng.random_range(0..12u32) == 0 {
                    grid[y][x] = Cell::new('·', sky_color);
                }
            }
        }
        // Ground: truchet with some grass chars mixed in
        let grass_chars = ['╌', '╌', '∿', '~', '·'];
        for y in horizon..height {
            for x in 0..width {
                let depth = y - horizon;
                if depth < 2 {
                    // Grass transition line
                    grid[y][x] = Cell::new(
                        grass_chars[rng.random_range(0..grass_chars.len() as u32) as usize],
                        lighten(ground_color, 20),
                    );
                } else {
                    grid[y][x] = Cell::new(
                        ground_tiles[rng.random_range(0..ground_tiles.len() as u32) as usize],
                        darken(ground_color, (depth * 3) as u8),
                    );
                }
            }
        }

        // Tree placement: staggered roots, varied sizes
        let tree_count = rng.random_range(5..10u32) as usize;
        struct TreeSlot {
            x: usize,
            root_y: usize,
            kind: usize,
            spread: usize,
            canopy_y: usize,
        }
        let mut slots: Vec<TreeSlot> = Vec::new();

        // One kaiju tree (kind 13 = grow_kaiju_tree in the dispatch)
        let kaiju_x = rng.random_range((width / 6) as u32..(width * 5 / 6) as u32) as usize;
        let kaiju_root = horizon + rng.random_range(0..3u32) as usize;
        let kaiju_spread = rng.random_range(12..20u32) as usize;
        let kaiju_canopy = rng.random_range(2..5u32) as usize;
        slots.push(TreeSlot {
            x: kaiju_x,
            root_y: kaiju_root,
            kind: 13,
            spread: kaiju_spread,
            canopy_y: kaiju_canopy,
        });

        // Remaining trees: staggered roots along horizon zone
        for _ in 0..tree_count - 1 {
            let tx = rng.random_range(4..(width - 4) as u32) as usize;
            let root_offset = rng.random_range(0..5u32) as usize; // roots at different depths
            let root_y = horizon + root_offset;
            if root_y >= height - 1 {
                continue;
            }
            let spread = rng.random_range(3..10u32) as usize;
            let tree_height =
                rng.random_range(8..(root_y.saturating_sub(2).max(9)) as u32) as usize;
            let canopy_y = root_y.saturating_sub(tree_height).max(1);
            // Favor asymmetric/storm/dead kinds (9, 7, 12) alongside others
            let kind = rng.random_range(0..14u32) as usize;
            slots.push(TreeSlot {
                x: tx,
                root_y,
                kind,
                spread,
                canopy_y,
            });
        }

        // Sort by root_y descending so farther trees draw first (back to front)
        slots.sort_by(|a, b| a.root_y.cmp(&b.root_y).then(a.x.cmp(&b.x)));

        // Draw trees directly on background -- no clearing rectangles
        for slot in &slots {
            let color = palette[rng.random_range(1..5)];
            draw_tree(
                &mut grid,
                slot.x,
                slot.root_y,
                slot.canopy_y,
                slot.spread,
                slot.kind,
                color,
                &mut rng,
            );
        }

        // Scatter flowers/fruit along the ground, clustering near tree bases
        for slot in &slots {
            let burst_count = rng.random_range(1..5u32);
            for _ in 0..burst_count {
                let angle = rng.random::<f32>() * std::f32::consts::TAU;
                let radius = rng.random_range(2..10u32) as f32;
                let fx = (slot.x as f32 + angle.cos() * radius * 1.5) as i32;
                let fy = (slot.root_y as f32 + angle.sin() * radius * 0.4 + 1.0) as i32;
                if fx >= 1 && fy >= 1 && (fx as usize) < width - 1 && (fy as usize) < height - 1 {
                    let c = palette[rng.random_range(2..5)];
                    if rng.random_range(0..3u32) == 0 {
                        draw_fruit(
                            &mut grid,
                            fx as usize,
                            fy as usize,
                            rng.random_range(0..5),
                            c,
                        );
                    } else {
                        draw_flower(
                            &mut grid,
                            fx as usize,
                            fy as usize,
                            rng.random_range(0..5),
                            c,
                        );
                    }
                }
            }
        }
    (grid, false)
}

/// Dispatch arm for mode(s): forest4 (moved verbatim from run()).
pub(crate) fn cli_forest4(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        // Like forest3 but with wild/unbalanced trees and algorithmic sprites.
        // More trees planted lower, more ground coverage.
        // Horizon at 60-80% down the screen (more sky, less grass domination)
        let horizon = height * 3 / 5 + rng.random_range(0..(height / 5).max(1) as u32) as usize;
        let sky_color = darken(palette[0], 95);
        let ground_color = darken(palette[1], 80);

        // Sky: sparse dots
        for y in 0..horizon {
            for x in 0..width {
                if rng.random_range(0..15u32) == 0 {
                    grid[y][x] = Cell::new('·', sky_color);
                }
            }
        }
        // Clouds: 1-4 in the upper sky
        let cloud_count = rng.random_range(1..5u32);
        let cloud_color = lighten(palette[0], 15);
        for _ in 0..cloud_count {
            let cx = rng.random_range(5..(width - 5) as u32) as usize;
            let cy = rng.random_range(2..(horizon / 2).max(3) as u32) as usize;
            let cw = rng.random_range(8..20u32) as usize;
            draw_cloud(&mut grid, cx, cy, cw, cloud_color, &mut rng);
        }
        // Per-column ground height: random walk so the grass edge is ragged
        let jitter_range = rng.random_range(2..6u32) as i32; // how wild the edge gets
        let mut ground_heights: Vec<usize> = Vec::with_capacity(width);
        let mut gh = horizon as i32;
        for _ in 0..width {
            gh += rng.random_range(0..3u32) as i32 - 1; // random walk: -1, 0, or +1
            gh = gh.clamp(horizon as i32 - jitter_range, horizon as i32 + jitter_range);
            ground_heights.push(gh.max(1) as usize);
        }

        // Ground: hue gradient with random direction sweeping across
        let ground_chars = ['╱', '╲', '·', '∿', '~'];
        let ground_depth = (height - horizon).max(1);
        // Random gradient direction
        let grad_dir = rng.random_range(0..6u32);
        // Base hue from palette
        let ground_base_hue: f64 = if let Color::Rgb { r, g, .. } = ground_color {
            (r as f64 * 1.4 + g as f64 * 0.7) % 360.0
        } else {
            120.0
        };
        let hue_sweep = rng.random_range(30..80u32) as f64;

        for x in 0..width {
            let col_horizon = ground_heights[x];
            for y in col_horizon..height {
                let depth = y - col_horizon;
                let ch = ground_chars[rng.random_range(0..ground_chars.len() as u32) as usize];

                // Gradient parameter t: 0.0 to 1.0, direction varies per seed
                let t = match grad_dir {
                    0 => x as f64 / width as f64,            // left to right
                    1 => 1.0 - x as f64 / width as f64,      // right to left
                    2 => depth as f64 / ground_depth as f64, // top to bottom
                    3 => (x as f64 / width as f64 + depth as f64 / ground_depth as f64) / 2.0, // diagonal ↘
                    4 => {
                        ((1.0 - x as f64 / width as f64) + depth as f64 / ground_depth as f64) / 2.0
                    } // diagonal ↙
                    _ => {
                        // Radial from center of ground
                        let cx = width as f64 / 2.0;
                        let cy = ground_depth as f64 / 2.0;
                        let dx = (x as f64 - cx) / cx;
                        let dy = (depth as f64 - cy) / cy.max(1.0);
                        (dx * dx + dy * dy).sqrt().min(1.0)
                    }
                };
                let h = (ground_base_hue + t * hue_sweep).rem_euclid(360.0);
                let l = (0.25 - depth as f64 * 0.006).max(0.10);
                let s = 0.4 + t * 0.2;
                let c = hsl_to_rgb(h, s.min(0.8), l);
                grid[y][x] = Cell::new(ch, c);
            }
        }

        // Tree placement: more trees, wider root stagger
        let tree_count = rng.random_range(5..10u32) as usize;
        struct TreeSlot {
            x: usize,
            root_y: usize,
            kind: usize,
            spread: usize,
            canopy_y: usize,
        }
        let mut slots: Vec<TreeSlot> = Vec::new();

        // One kaiju tree -- root at the grass line
        let kaiju_x = rng.random_range((width / 8) as u32..(width * 7 / 8) as u32) as usize;
        let kaiju_root =
            ground_heights[kaiju_x.min(width - 1)] + rng.random_range(0..3u32) as usize;
        let kaiju_root = kaiju_root.min(height - 2);
        slots.push(TreeSlot {
            x: kaiju_x,
            root_y: kaiju_root,
            kind: 13,
            spread: rng.random_range(14..22u32) as usize,
            canopy_y: rng.random_range(1..4u32) as usize,
        });

        // Remaining trees: favor wild (14), asymmetric (9), storm (7), dead (12)
        // Enforce minimum spacing so trees don't pile on top of each other
        let unbalanced_kinds = [14, 14, 9, 9, 7, 7, 12, 13, 15, 15, 16, 17, 17, 4, 5, 6, 11];
        let min_spacing = (width / (tree_count + 1)).max(14);
        for _ in 0..tree_count - 1 {
            let mut tx = 0usize;
            let mut placed = false;
            for _ in 0..10 {
                tx = rng.random_range(3..(width - 3) as u32) as usize;
                let too_close = slots
                    .iter()
                    .any(|s| ((s.x as i32 - tx as i32).unsigned_abs() as usize) < min_spacing);
                if !too_close {
                    placed = true;
                    break;
                }
            }
            if !placed {
                tx = rng.random_range(3..(width - 3) as u32) as usize;
            }

            // Root at grass line + small offset so trunk meets the ground
            let grass_y = ground_heights[tx.min(width - 1)];
            let root_offset = rng.random_range(0..4u32) as usize;
            let root_y = (grass_y + root_offset).min(height - 2);

            // Height tiers: some scrubby (3-8), some medium (8-20), some towering (20-root_y)
            let max_possible = root_y.saturating_sub(1).max(4);
            let tree_height = match rng.random_range(0..10u32) {
                0..=2 => rng.random_range(3..8u32.min(max_possible as u32 + 1)) as usize, // scrubby
                3..=6 => rng.random_range(8..20u32.min(max_possible as u32 + 1)) as usize, // medium
                _ => rng.random_range(20u32.min(max_possible as u32)..max_possible as u32 + 1)
                    as usize, // towering
            };
            let canopy_y = root_y.saturating_sub(tree_height).max(1);

            // Spread also tiered: narrow (1-4), medium (4-10), wide (10-20)
            let spread = match rng.random_range(0..6u32) {
                0..=1 => rng.random_range(1..5u32) as usize,
                2..=4 => rng.random_range(4..11u32) as usize,
                _ => rng.random_range(10..21u32) as usize,
            };

            let kind =
                unbalanced_kinds[rng.random_range(0..unbalanced_kinds.len() as u32) as usize];
            slots.push(TreeSlot {
                x: tx,
                root_y,
                kind,
                spread,
                canopy_y,
            });
        }

        // Back-to-front
        slots.sort_by(|a, b| a.root_y.cmp(&b.root_y).then(a.x.cmp(&b.x)));

        // Give each tree a distinct hue + depth-based brightness
        // Slots are sorted back-to-front (ascending root_y), so earlier = farther = dimmer
        let slot_count = slots.len();
        for (i, slot) in slots.iter().enumerate() {
            let base_hue =
                (i as f64 * 360.0 / slot_count as f64 + rng.random_range(0..30u32) as f64) % 360.0;
            // Depth factor: 0.0 = farthest (dim), 1.0 = closest (bright)
            let depth_t = i as f64 / (slot_count - 1).max(1) as f64;
            let lightness = 0.2 + depth_t * 0.3; // 0.2 (far) to 0.5 (near)
            let saturation = 0.4 + depth_t * 0.3;
            let color = hsl_to_rgb(base_hue, saturation, lightness);
            draw_tree(
                &mut grid,
                slot.x,
                slot.root_y,
                slot.canopy_y,
                slot.spread,
                slot.kind,
                color,
                &mut rng,
            );
        }

        // Sprout braille leaf clusters at branch tips (~50% of tips)
        let leaf_hue = rng.random_range(60..180u32) as f64; // green-ish range
        let leaf_color = hsl_to_rgb(leaf_hue, 0.5, 0.3);
        sprout_leaves(&mut grid, leaf_color, 50, &mut rng);

        // Tighter flower/fruit scatter: fewer per tree, smaller radius, only at ground level
        for slot in &slots {
            let burst = rng.random_range(0..3u32); // 0-2 instead of 2-5
            for _ in 0..burst {
                let angle = rng.random::<f32>() * std::f32::consts::TAU;
                let radius = rng.random_range(1..6u32) as f32; // tighter radius
                let fx = (slot.x as f32 + angle.cos() * radius * 1.5) as i32;
                // Keep at or just below root, not floating in the sky
                let fy = slot.root_y as i32 + rng.random_range(1..3u32) as i32;
                if fx >= 1 && fy >= 1 && (fx as usize) < width - 1 && (fy as usize) < height - 1 {
                    let c = palette[rng.random_range(2..5)];
                    match rng.random_range(0..3u32) {
                        0 => grow_flower_spiral(&mut grid, fx as usize, fy as usize, c, &mut rng),
                        1 => grow_fruit_vine(&mut grid, fx as usize, fy as usize, c, &mut rng),
                        _ => draw_flower(
                            &mut grid,
                            fx as usize,
                            fy as usize,
                            rng.random_range(0..5),
                            c,
                        ),
                    }
                }
            }
        }

        // Foreground trees: 1-3 trees planted deep in the ground, drawn last (in front)
        let fg_count = rng.random_range(1..4u32);
        for _ in 0..fg_count {
            let tx = rng.random_range(3..(width - 3) as u32) as usize;
            let grass_y = ground_heights[tx.min(width - 1)];
            let root_y = (grass_y + rng.random_range(2..6u32) as usize).min(height - 2);
            let tree_height = rng.random_range(4..12u32) as usize;
            let canopy_y = root_y.saturating_sub(tree_height).max(1);
            let spread = rng.random_range(3..10u32) as usize;
            let kind = rng.random_range(0..18u32) as usize;
            let fg_hue = rng.random_range(0..360u32) as f64;
            let color = hsl_to_rgb(fg_hue, 0.6, 0.4);
            draw_tree(
                &mut grid, tx, root_y, canopy_y, spread, kind, color, &mut rng,
            );
        }
    (grid, false)
}

/// Dispatch arm for mode(s): forest5 (moved verbatim from run()).
pub(crate) fn cli_forest5(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        // Clustered forest: groups of same-family trees with slight color variation.
        // Center tree tallest in each cluster, edges taper. Per-tree tip decoration.
        // Root systems at trunk bases.
        let horizon = height * 3 / 5 + rng.random_range(0..(height / 5).max(1) as u32) as usize;
        let sky_color = darken(palette[0], 95);
        let ground_color = darken(palette[1], 80);

        // Sky: sparse dots
        for y in 0..horizon {
            for x in 0..width {
                if rng.random_range(0..15u32) == 0 {
                    grid[y][x] = Cell::new('·', sky_color);
                }
            }
        }
        // Clouds
        let cloud_count = rng.random_range(1..5u32);
        let cloud_color = lighten(palette[0], 15);
        for _ in 0..cloud_count {
            let cx = rng.random_range(5..(width - 5) as u32) as usize;
            let cy = rng.random_range(2..(horizon / 2).max(3) as u32) as usize;
            let cw = rng.random_range(8..20u32) as usize;
            draw_cloud(&mut grid, cx, cy, cw, cloud_color, &mut rng);
        }

        // Per-column ground height via random walk
        let jitter_range = rng.random_range(2..6u32) as i32;
        let mut ground_heights: Vec<usize> = Vec::with_capacity(width);
        let mut gh = horizon as i32;
        for _ in 0..width {
            gh += rng.random_range(0..3u32) as i32 - 1;
            gh = gh.clamp(horizon as i32 - jitter_range, horizon as i32 + jitter_range);
            ground_heights.push(gh.max(1) as usize);
        }

        // Ground fill with hue gradient
        let ground_chars = ['╱', '╲', '·', '∿', '~'];
        let ground_depth = (height - horizon).max(1);
        let grad_dir = rng.random_range(0..6u32);
        let ground_base_hue: f64 = if let Color::Rgb { r, g, .. } = ground_color {
            (r as f64 * 1.4 + g as f64 * 0.7) % 360.0
        } else {
            120.0
        };
        let hue_sweep = rng.random_range(30..80u32) as f64;

        for x in 0..width {
            let col_horizon = ground_heights[x];
            for y in col_horizon..height {
                let depth = y - col_horizon;
                let ch = ground_chars[rng.random_range(0..ground_chars.len() as u32) as usize];
                let t = match grad_dir {
                    0 => x as f64 / width as f64,
                    1 => 1.0 - x as f64 / width as f64,
                    2 => depth as f64 / ground_depth as f64,
                    3 => (x as f64 / width as f64 + depth as f64 / ground_depth as f64) / 2.0,
                    4 => {
                        ((1.0 - x as f64 / width as f64) + depth as f64 / ground_depth as f64) / 2.0
                    }
                    _ => {
                        let cx = width as f64 / 2.0;
                        let cy = ground_depth as f64 / 2.0;
                        let dx = (x as f64 - cx) / cx;
                        let dy = (depth as f64 - cy) / cy.max(1.0);
                        (dx * dx + dy * dy).sqrt().min(1.0)
                    }
                };
                let h = (ground_base_hue + t * hue_sweep).rem_euclid(360.0);
                let l = (0.25 - depth as f64 * 0.006).max(0.10);
                let s = 0.4 + t * 0.2;
                let c = hsl_to_rgb(h, s.min(0.8), l);
                grid[y][x] = Cell::new(ch, c);
            }
        }

        // --- Cluster placement: 1 dominant tree + 0-2 small companions ---
        // Fewer trees, more breathing room. Each cluster owns a wide horizontal zone.
        let cluster_count = rng.random_range(2..5u32) as usize;
        let zone_width = width / cluster_count.max(1);

        // Mix old tree algos (visual personality) with pen trees (connectivity).
        // Dominant trees use the interesting old kinds, companions use pen trees.
        let dominant_kinds = [0, 7, 9, 13, 14, 15, 17]; // grow_tree, storm, asymmetric, kaiju, wild, zigzag, tendril
        let family_decos = [
            TipDeco::Fruit,
            TipDeco::Drip,
            TipDeco::Flower,
            TipDeco::Fruit,
            TipDeco::Fruit,
            TipDeco::Drip,
            TipDeco::Flower,
        ];

        struct PlacedTree {
            x: usize,
            root_y: usize,
            canopy_y: usize,
            spread: usize,
            kind: usize,
            use_pen: bool,
            is_dominant: bool,
        }
        let mut all_trees: Vec<(PlacedTree, f64, usize)> = Vec::new();

        for ci in 0..cluster_count {
            let dom_kind_idx = rng.random_range(0..dominant_kinds.len() as u32) as usize;
            let dom_kind = dominant_kinds[dom_kind_idx];
            let base_hue = (ci as f64 * 360.0 / cluster_count as f64
                + rng.random_range(0..30u32) as f64)
                % 360.0;

            // Dominant tree: old algo with visual personality
            let zone_start = zone_width * ci;
            let dom_x = zone_start
                + zone_width / 2
                + rng.random_range(0..(zone_width / 4).max(1) as u32) as usize;
            let dom_x = dom_x.clamp(5, width - 5);
            let grass_y = ground_heights[dom_x.min(width - 1)];
            let dom_root = (grass_y + rng.random_range(2..8u32) as usize).min(height - 2);
            let max_h = dom_root.saturating_sub(3).max(6);
            let dom_h = rng
                .random_range((max_h as u32 / 2).max(8).min(max_h as u32)..max_h as u32 + 1)
                as usize;
            let dom_canopy = dom_root.saturating_sub(dom_h).max(1);
            let dom_spread = rng.random_range(8..16u32) as usize;

            all_trees.push((
                PlacedTree {
                    x: dom_x,
                    root_y: dom_root,
                    canopy_y: dom_canopy,
                    spread: dom_spread,
                    kind: dom_kind,
                    use_pen: false,
                    is_dominant: true,
                },
                base_hue,
                dom_kind_idx,
            ));

            // 0-2 small companion trees: pen trees (connected, small)
            let companion_count = rng.random_range(0..3u32);
            for _ in 0..companion_count {
                let offset = rng.random_range(12..25u32) as i32
                    * if rng.random_range(0..2u32) == 0 {
                        -1
                    } else {
                        1
                    };
                let cx = (dom_x as i32 + offset).clamp(3, width as i32 - 3) as usize;
                let cgrass = ground_heights[cx.min(width - 1)];
                let croot = (cgrass + rng.random_range(1..6u32) as usize).min(height - 2);
                let cmax = croot.saturating_sub(2).max(3);
                let lo = 3u32.min(cmax as u32);
                let hi = (cmax as u32 / 2 + 4).max(lo + 1);
                let ch = rng.random_range(lo..hi) as usize;
                let ccanopy = croot.saturating_sub(ch).max(1);
                let cspread = rng.random_range(2..7u32) as usize;
                let hue_jitter = rng.random_range(0..20u32) as f64 - 10.0;

                all_trees.push((
                    PlacedTree {
                        x: cx,
                        root_y: croot,
                        canopy_y: ccanopy,
                        spread: cspread,
                        kind: 0,
                        use_pen: true,
                        is_dominant: false,
                    },
                    base_hue + hue_jitter,
                    dom_kind_idx,
                ));
            }
        }

        // Sort back-to-front
        all_trees.sort_by(|a, b| a.0.root_y.cmp(&b.0.root_y).then(a.0.x.cmp(&b.0.x)));

        let total = all_trees.len();
        for (i, (tree, hue, family_idx)) in all_trees.iter().enumerate() {
            let depth_t = i as f64 / total.max(1) as f64;
            let lightness = 0.22 + depth_t * 0.28;
            let saturation = 0.40 + depth_t * 0.25;
            let color = hsl_to_rgb(*hue, saturation, lightness);

            if tree.use_pen {
                // Companion: pen tree (connected, small)
                let recipe = if rng.random_range(0..2u32) == 0 {
                    TreeRecipe::dead()
                } else {
                    TreeRecipe::columnar()
                };
                grow_pen_tree(
                    &mut grid,
                    tree.x,
                    tree.root_y,
                    tree.canopy_y,
                    tree.spread,
                    color,
                    &recipe,
                    &mut rng,
                );
            } else {
                // Dominant: old algo with visual personality
                draw_tree(
                    &mut grid,
                    tree.x,
                    tree.root_y,
                    tree.canopy_y,
                    tree.spread,
                    tree.kind,
                    color,
                    &mut rng,
                );
            }

            // Collect and decorate tips
            let x0 = tree.x.saturating_sub(tree.spread + 5);
            let x1 = (tree.x + tree.spread + 5).min(width);
            let tips = collect_tips_in_rect(&grid, x0, tree.canopy_y, x1, tree.root_y + 1);
            let deco = family_decos[*family_idx];
            let fruit_color = shift_hue(color, 60.0 + rng.random_range(0..40u32) as f64);
            decorate_tips(&mut grid, &tips, deco, fruit_color, 15, &mut rng);
        }

        // Sprout braille leaf clusters
        let leaf_hue = rng.random_range(60..180u32) as f64;
        let leaf_color = hsl_to_rgb(leaf_hue, 0.5, 0.3);
        sprout_leaves(&mut grid, leaf_color, 35, &mut rng);
    (grid, false)
}

/// Dispatch arm for mode(s): forest6 (moved verbatim from run()).
pub(crate) fn cli_forest6(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        // Forest6: bespoke pen trees drawn next to their old equivalents for comparison.
        // Reuses forest5 sky/grass/ground layout.

        let horizon = height * 3 / 5 + rng.random_range(0..(height / 5).max(1) as u32) as usize;
        let sky_color = darken(palette[0], 95);
        let ground_color = darken(palette[1], 80);

        // Sky: sparse dots
        for y in 0..horizon {
            for x in 0..width {
                if rng.random_range(0..15u32) == 0 {
                    grid[y][x] = Cell::new('·', sky_color);
                }
            }
        }
        // Clouds
        let cloud_count = rng.random_range(1..5u32);
        let cloud_color = lighten(palette[0], 15);
        for _ in 0..cloud_count {
            let cx = rng.random_range(5..(width - 5) as u32) as usize;
            let cy = rng.random_range(2..(horizon / 2).max(3) as u32) as usize;
            let cw = rng.random_range(8..20u32) as usize;
            draw_cloud(&mut grid, cx, cy, cw, cloud_color, &mut rng);
        }

        // Per-column ground height via random walk
        let jitter_range = rng.random_range(2..6u32) as i32;
        let mut ground_heights: Vec<usize> = Vec::with_capacity(width);
        let mut gh = horizon as i32;
        for _ in 0..width {
            gh += rng.random_range(0..3u32) as i32 - 1;
            gh = gh.clamp(horizon as i32 - jitter_range, horizon as i32 + jitter_range);
            ground_heights.push(gh.max(1) as usize);
        }

        // Ground fill with hue gradient (same as forest5)
        let ground_chars = ['╱', '╲', '·', '∿', '~'];
        let ground_depth = (height - horizon).max(1);
        let grad_dir = rng.random_range(0..6u32);
        let ground_base_hue: f64 = if let Color::Rgb { r, g, .. } = ground_color {
            (r as f64 * 1.4 + g as f64 * 0.7) % 360.0
        } else {
            120.0
        };
        let hue_sweep = rng.random_range(30..80u32) as f64;

        for x in 0..width {
            let col_horizon = ground_heights[x];
            for y in col_horizon..height {
                let depth = y - col_horizon;
                let ch = ground_chars[rng.random_range(0..ground_chars.len() as u32) as usize];
                let t = match grad_dir {
                    0 => x as f64 / width as f64,
                    1 => 1.0 - x as f64 / width as f64,
                    2 => depth as f64 / ground_depth as f64,
                    3 => (x as f64 / width as f64 + depth as f64 / ground_depth as f64) / 2.0,
                    4 => {
                        ((1.0 - x as f64 / width as f64) + depth as f64 / ground_depth as f64) / 2.0
                    }
                    _ => {
                        let cx = width as f64 / 2.0;
                        let cy = ground_depth as f64 / 2.0;
                        let dx = (x as f64 - cx) / cx;
                        let dy = (depth as f64 - cy) / cy.max(1.0);
                        (dx * dx + dy * dy).sqrt().min(1.0)
                    }
                };
                let h = (ground_base_hue + t * hue_sweep).rem_euclid(360.0);
                let l = (0.25 - depth as f64 * 0.006).max(0.10);
                let s = 0.4 + t * 0.2;
                let c = hsl_to_rgb(h, s.min(0.8), l);
                grid[y][x] = Cell::new(ch, c);
            }
        }

        // --- Forest of trait trees (forest4-style composition) ---
        let tree_count = rng.random_range(6..12u32) as usize;
        let trait_kinds: [usize; 11] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

        struct TreeSlot {
            x: usize,
            root_y: usize,
            canopy_y: usize,
            spread: usize,
            kind: usize,
            hue: f64,
            energy: f32,
        }
        let mut slots: Vec<TreeSlot> = Vec::new();

        // One anchor tree -- tallest, widest, planted near center
        let anchor_x = rng.random_range((width / 8) as u32..(width * 7 / 8) as u32) as usize;
        let anchor_grass = ground_heights[anchor_x.min(width - 1)];
        let anchor_root = (anchor_grass + rng.random_range(0..3u32) as usize).min(height - 2);
        slots.push(TreeSlot {
            x: anchor_x,
            root_y: anchor_root,
            canopy_y: rng.random_range(1..4u32) as usize,
            spread: rng.random_range(14..22u32) as usize,
            kind: trait_kinds[rng.random_range(0..trait_kinds.len() as u32) as usize],
            hue: rng.random_range(0..360u32) as f64,
            energy: 0.95,
        });

        // Remaining trees with min spacing, height/spread tiers
        let min_spacing = (width / (tree_count + 1)).max(12);
        for _ in 0..tree_count - 1 {
            let mut tx = 0usize;
            let mut placed = false;
            for _ in 0..10 {
                tx = rng.random_range(3..(width - 3) as u32) as usize;
                let too_close = slots
                    .iter()
                    .any(|s| ((s.x as i32 - tx as i32).unsigned_abs() as usize) < min_spacing);
                if !too_close {
                    placed = true;
                    break;
                }
            }
            if !placed {
                tx = rng.random_range(3..(width - 3) as u32) as usize;
            }

            let grass_y = ground_heights[tx.min(width - 1)];
            let root_y = (grass_y + rng.random_range(0..4u32) as usize).min(height - 2);

            // Height tiers: scrubby / medium / towering
            let max_possible = root_y.saturating_sub(1).max(4);
            let tree_height = match rng.random_range(0..10u32) {
                0..=2 => rng.random_range(3..8u32.min(max_possible as u32 + 1)) as usize,
                3..=6 => rng.random_range(8..20u32.min(max_possible as u32 + 1)) as usize,
                _ => rng.random_range(20u32.min(max_possible as u32)..max_possible as u32 + 1)
                    as usize,
            };
            let canopy_y = root_y.saturating_sub(tree_height).max(1);

            // Spread tiers: narrow / medium / wide
            let spread = match rng.random_range(0..6u32) {
                0..=1 => rng.random_range(2..6u32) as usize,
                2..=4 => rng.random_range(5..12u32) as usize,
                _ => rng.random_range(10..20u32) as usize,
            };

            let kind = trait_kinds[rng.random_range(0..trait_kinds.len() as u32) as usize];
            let energy = match tree_height {
                0..=7 => rng.random_range(40..65u32) as f32 / 100.0,
                8..=19 => rng.random_range(65..85u32) as f32 / 100.0,
                _ => rng.random_range(85..100u32) as f32 / 100.0,
            };

            slots.push(TreeSlot {
                x: tx,
                root_y,
                canopy_y,
                spread,
                kind,
                hue: rng.random_range(0..360u32) as f64,
                energy,
            });
        }

        // Back-to-front depth sort
        slots.sort_by(|a, b| a.root_y.cmp(&b.root_y).then(a.x.cmp(&b.x)));

        // Depth-based brightness: farther (lower root_y) = dimmer
        let slot_count = slots.len();
        for (i, slot) in slots.iter().enumerate() {
            let depth_t = i as f64 / (slot_count - 1).max(1) as f64;
            let lightness = 0.2 + depth_t * 0.3;
            let saturation = 0.4 + depth_t * 0.3;
            let color = hsl_to_rgb(slot.hue, saturation, lightness);

            let plot_w = slot.spread * 2 + 6;
            let plot = Rect {
                x: slot.x.saturating_sub(plot_w / 2),
                y: slot.canopy_y,
                w: plot_w,
                h: slot.root_y - slot.canopy_y + 2,
            };
            let tp = TreeParams {
                plot,
                energy: slot.energy,
                trunk_color: color,
                bark_color: darken(color, 15),
                branch_color: color,
                tip_color: lighten(color, 30),
                fruit_color: shift_hue(color, 60.0),
                fruit_factor: 0.3,
                branch_factor: 0.8,
                direction: GrowDir::Up,
                bole: None,
                taper: TaperKind::default(),
            };
            match slot.kind {
                0 => SplitTree.grow(&mut grid, &tp, &mut rng),
                1 => SpiralTree.grow(&mut grid, &tp, &mut rng),
                2 => CandelabraTree.grow(&mut grid, &tp, &mut rng),
                3 => BirchTree.grow(&mut grid, &tp, &mut rng),
                4 => StormTree::new().grow(&mut grid, &tp, &mut rng),
                5 => DroopingTree.grow(&mut grid, &tp, &mut rng),
                6 => DeadTree.grow(&mut grid, &tp, &mut rng),
                7 => WavyBirch.grow(&mut grid, &tp, &mut rng),
                8 => PineTree.grow(&mut grid, &tp, &mut rng),
                9 => WillowTree.grow(&mut grid, &tp, &mut rng),
                10 => PalmTree.grow(&mut grid, &tp, &mut rng),
                _ => SpiralTree.grow(&mut grid, &tp, &mut rng),
            }
        }

        // Braille leaf clusters at branch tips
        let leaf_hue = rng.random_range(60..180u32) as f64;
        let leaf_color = hsl_to_rgb(leaf_hue, 0.5, 0.3);
        sprout_leaves(&mut grid, leaf_color, 45, &mut rng);

        // Flower/fruit scatter at ground level near tree bases
        for slot in &slots {
            let burst = rng.random_range(0..3u32);
            for _ in 0..burst {
                let angle = rng.random::<f32>() * std::f32::consts::TAU;
                let radius = rng.random_range(1..6u32) as f32;
                let fx = (slot.x as f32 + angle.cos() * radius * 1.5) as i32;
                let fy = slot.root_y as i32 + rng.random_range(1..3u32) as i32;
                if fx >= 1 && fy >= 1 && (fx as usize) < width - 1 && (fy as usize) < height - 1 {
                    let c = palette[rng.random_range(2..5)];
                    match rng.random_range(0..3u32) {
                        0 => grow_flower_spiral(&mut grid, fx as usize, fy as usize, c, &mut rng),
                        1 => grow_fruit_vine(&mut grid, fx as usize, fy as usize, c, &mut rng),
                        _ => draw_flower(
                            &mut grid,
                            fx as usize,
                            fy as usize,
                            rng.random_range(0..5),
                            c,
                        ),
                    }
                }
            }
        }

        // Foreground trees: 1-3 drawn last (in front of everything)
        let fg_count = rng.random_range(1..4u32);
        for _ in 0..fg_count {
            let tx = rng.random_range(3..(width - 3) as u32) as usize;
            let grass_y = ground_heights[tx.min(width - 1)];
            let root_y = (grass_y + rng.random_range(2..6u32) as usize).min(height - 2);
            let tree_height = rng.random_range(4..12u32) as usize;
            let canopy_y = root_y.saturating_sub(tree_height).max(1);
            let spread = rng.random_range(3..10u32) as usize;
            let kind = trait_kinds[rng.random_range(0..trait_kinds.len() as u32) as usize];
            let fg_hue = rng.random_range(0..360u32) as f64;
            let color = hsl_to_rgb(fg_hue, 0.6, 0.4);

            let plot_w = spread * 2 + 6;
            let plot = Rect {
                x: tx.saturating_sub(plot_w / 2),
                y: canopy_y,
                w: plot_w,
                h: root_y - canopy_y + 2,
            };
            let tp = TreeParams {
                plot,
                energy: 0.75,
                trunk_color: color,
                bark_color: darken(color, 15),
                branch_color: color,
                tip_color: lighten(color, 30),
                fruit_color: shift_hue(color, 60.0),
                fruit_factor: 0.2,
                branch_factor: 0.7,
                direction: GrowDir::Up,
                bole: None,
                taper: TaperKind::default(),
            };
            match kind {
                0 => SplitTree.grow(&mut grid, &tp, &mut rng),
                1 => SpiralTree.grow(&mut grid, &tp, &mut rng),
                2 => CandelabraTree.grow(&mut grid, &tp, &mut rng),
                3 => BirchTree.grow(&mut grid, &tp, &mut rng),
                4 => StormTree::new().grow(&mut grid, &tp, &mut rng),
                5 => DroopingTree.grow(&mut grid, &tp, &mut rng),
                6 => DeadTree.grow(&mut grid, &tp, &mut rng),
                7 => WavyBirch.grow(&mut grid, &tp, &mut rng),
                8 => PineTree.grow(&mut grid, &tp, &mut rng),
                9 => WillowTree.grow(&mut grid, &tp, &mut rng),
                10 => PalmTree.grow(&mut grid, &tp, &mut rng),
                _ => SpiralTree.grow(&mut grid, &tp, &mut rng),
            }
        }
    (grid, false)
}

/// Dispatch arm for mode(s): forest7 (moved verbatim from run()).
pub(crate) fn cli_forest7(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        // forest7: production layered forest with boles, tapers, fruit
        let horizon = height * 3 / 5 + rng.random_range(0..(height / 5).max(1) as u32) as usize;
        let sky_color = darken(palette[0], 95);
        let ground_color = darken(palette[1], 80);

        // Sky
        for y in 0..horizon {
            for x in 0..width {
                if rng.random_range(0..15u32) == 0 {
                    grid[y][x] = Cell::new('·', sky_color);
                }
            }
        }
        let cloud_count = rng.random_range(1..5u32);
        let cloud_color = lighten(palette[0], 15);
        for _ in 0..cloud_count {
            let cx = rng.random_range(5..(width - 5) as u32) as usize;
            let cy = rng.random_range(2..(horizon / 2).max(3) as u32) as usize;
            let cw = rng.random_range(8..20u32) as usize;
            draw_cloud(&mut grid, cx, cy, cw, cloud_color, &mut rng);
        }

        // Per-column ground height
        let jitter_range = rng.random_range(2..6u32) as i32;
        let mut ground_heights: Vec<usize> = Vec::with_capacity(width);
        let mut gh = horizon as i32;
        for _ in 0..width {
            gh += rng.random_range(0..3u32) as i32 - 1;
            gh = gh.clamp(horizon as i32 - jitter_range, horizon as i32 + jitter_range);
            ground_heights.push(gh.max(1) as usize);
        }

        // Ground fill with hue gradient
        let ground_chars = ['╱', '╲', '·', '∿', '~'];
        let ground_depth = (height - horizon).max(1);
        let grad_dir = rng.random_range(0..6u32);
        let ground_base_hue: f64 = if let Color::Rgb { r, g, .. } = ground_color {
            (r as f64 * 1.4 + g as f64 * 0.7) % 360.0
        } else {
            120.0
        };
        let hue_sweep = rng.random_range(30..80u32) as f64;

        for x in 0..width {
            let col_horizon = ground_heights[x];
            for y in col_horizon..height {
                let depth = y - col_horizon;
                let ch = ground_chars[rng.random_range(0..ground_chars.len() as u32) as usize];
                let t = match grad_dir {
                    0 => x as f64 / width as f64,
                    1 => 1.0 - x as f64 / width as f64,
                    2 => depth as f64 / ground_depth as f64,
                    3 => (x as f64 / width as f64 + depth as f64 / ground_depth as f64) / 2.0,
                    4 => {
                        ((1.0 - x as f64 / width as f64) + depth as f64 / ground_depth as f64) / 2.0
                    }
                    _ => {
                        let cx = width as f64 / 2.0;
                        let cy = ground_depth as f64 / 2.0;
                        let dx = (x as f64 - cx) / cx;
                        let dy = (depth as f64 - cy) / cy.max(1.0);
                        (dx * dx + dy * dy).sqrt().min(1.0)
                    }
                };
                let h = (ground_base_hue + t * hue_sweep).rem_euclid(360.0);
                let l = (0.25 - depth as f64 * 0.006).max(0.10);
                let s = 0.4 + t * 0.2;
                let c = hsl_to_rgb(h, s.min(0.8), l);
                grid[y][x] = Cell::new(ch, c);
            }
        }

        // ── Scene walk placement ──────────────────────────────────────
        // Walk across the terrain, placing elements at each stop.
        // Element types: tree, bush, flower cluster, fruit vine, empty gap.
        let all_tapers = [
            TaperKind::Diagonal,
            TaperKind::Shelf,
            TaperKind::Bracket,
            TaperKind::Step,
            TaperKind::Melt,
        ];

        #[derive(Clone, Copy)]
        enum F7Element {
            Tree {
                kind: usize,
                spread: usize,
                tree_h: usize,
                bole_style: Option<usize>,
                taper: TaperKind,
            },
            Bush {
                style: usize,
                bush_w: i32,
            },
            Flowers,
            FruitVine,
        }

        struct F7Stop {
            x: usize,
            root_y: usize,
            hue: f64,
            layer: u8,
            element: F7Element,
        }

        let mut stops: Vec<F7Stop> = Vec::new();

        // Walk: start at random x, hop 8-20 cells each step, wrap around
        let stop_count = rng.random_range(12..22u32) as usize;
        let min_spacing = (width / (stop_count + 1)).max(6);
        let mut wx = rng.random_range(4..(width - 4) as u32) as usize;

        for si in 0..stop_count {
            // Hop forward with some jitter
            if si > 0 {
                let hop = rng.random_range(
                    min_spacing as u32..(min_spacing as u32 * 3).min(width as u32 / 2),
                );
                wx = (wx + hop as usize) % width;
                wx = wx.clamp(3, width - 4);
            }

            let grass_y = ground_heights[wx.min(width - 1)];
            // Layer assignment: first third bg, middle third mid, last third fg
            let layer = match si * 3 / stop_count {
                0 => 0u8,
                1 => 1,
                _ => 2,
            };
            let root_offset = match layer {
                0 => rng.random_range(0..2u32) as usize,
                1 => rng.random_range(1..5u32) as usize,
                _ => rng.random_range(2..7u32) as usize,
            };
            let root_y = (grass_y + root_offset).min(height - 2);

            // Pick element type: trees most common, bushes and flowers fill gaps
            let element = match rng.random_range(0..10u32) {
                0..=5 => {
                    let kind = rng.random_range(0..17u32) as usize;
                    let spread = match layer {
                        0 => rng.random_range(2..6u32) as usize,
                        1 => rng.random_range(5..14u32) as usize,
                        _ => rng.random_range(10..22u32) as usize,
                    };
                    let tree_h = match layer {
                        0 => rng.random_range(3..10u32) as usize,
                        1 => rng.random_range(10..25u32) as usize,
                        _ => rng.random_range(20..40u32.min(root_y.max(21) as u32)) as usize,
                    };
                    // ~40% of trees get a bole, rest go straight trunk into ground
                    let bole_style = if rng.random_range(0..10u32) < 4 {
                        Some(rng.random_range(0..10u32) as usize) // simpler styles only
                    } else {
                        None
                    };
                    F7Element::Tree {
                        kind,
                        spread,
                        tree_h,
                        bole_style,
                        taper: all_tapers[rng.random_range(0..all_tapers.len() as u32) as usize],
                    }
                }
                // 6..=7 => F7Element::Bush {
                //     style: rng.random_range(0..18u32) as usize,
                //     bush_w: rng.random_range(3..8u32) as i32,
                // },
                6..=7 => F7Element::Flowers,
                8 => F7Element::Flowers,
                _ => F7Element::FruitVine,
            };

            stops.push(F7Stop {
                x: wx,
                root_y,
                hue: rng.random_range(0..360u32) as f64,
                layer,
                element,
            });
        }

        // Sort back-to-front: bg (layer 0) first, then mid, then fg
        stops.sort_by(|a, b| a.layer.cmp(&b.layer).then(a.root_y.cmp(&b.root_y)));

        // ── Draw each stop ───────────────────────────────────────────
        for stop in &stops {
            let lightness = match stop.layer {
                0 => 0.15 + rng.random::<f64>() * 0.10,
                1 => 0.25 + rng.random::<f64>() * 0.10,
                _ => 0.35 + rng.random::<f64>() * 0.10,
            };
            let saturation = match stop.layer {
                0 => 0.30,
                1 => 0.45,
                _ => 0.60,
            };
            let energy = match stop.layer {
                0 => rng.random_range(30..55u32) as f32 / 100.0,
                1 => rng.random_range(60..85u32) as f32 / 100.0,
                _ => rng.random_range(85..100u32) as f32 / 100.0,
            };
            let color = hsl_to_rgb(stop.hue, saturation, lightness);

            match stop.element {
                F7Element::Tree {
                    kind,
                    spread,
                    tree_h,
                    bole_style,
                    taper,
                } => {
                    let canopy_y = stop.root_y.saturating_sub(tree_h).max(1);
                    let plot_w = spread * 2 + 6;
                    let plot = Rect {
                        x: stop.x.saturating_sub(plot_w / 2),
                        y: canopy_y,
                        w: plot_w.min(width),
                        h: stop.root_y.saturating_sub(canopy_y) + 2,
                    };
                    let tp = TreeParams {
                        plot,
                        energy,
                        trunk_color: color,
                        bark_color: darken(color, 15),
                        branch_color: color,
                        tip_color: lighten(color, 30),
                        fruit_color: shift_hue(color, 60.0),
                        fruit_factor: 0.3,
                        branch_factor: 0.8,
                        direction: GrowDir::Up,
                        bole: bole_style.map(|s| Bole { style: s }),
                        taper,
                    };
                    match kind % 17 {
                        0 => SpiralTree.grow(&mut grid, &tp, &mut rng),
                        1 => CandelabraTree.grow(&mut grid, &tp, &mut rng),
                        2 => SplitTree.grow(&mut grid, &tp, &mut rng),
                        3 => BirchTree.grow(&mut grid, &tp, &mut rng),
                        4 => WavyBirch.grow(&mut grid, &tp, &mut rng),
                        5 => StormTree::new().grow(&mut grid, &tp, &mut rng),
                        6 => DeadTree.grow(&mut grid, &tp, &mut rng),
                        7 => DroopingTree.grow(&mut grid, &tp, &mut rng),
                        8 => PineTree.grow(&mut grid, &tp, &mut rng),
                        9 => WillowTree.grow(&mut grid, &tp, &mut rng),
                        10 => PalmTree.grow(&mut grid, &tp, &mut rng),
                        11 => WideTree.grow(&mut grid, &tp, &mut rng),
                        12 => AsymmetricTree.grow(&mut grid, &tp, &mut rng),
                        13 => KaijuTree.grow(&mut grid, &tp, &mut rng),
                        14 => ZigzagTree.grow(&mut grid, &tp, &mut rng),
                        15 => BrailleCanopyTree.grow(&mut grid, &tp, &mut rng),
                        16 => TendrilTree.grow(&mut grid, &tp, &mut rng),
                        _ => SpiralTree.grow(&mut grid, &tp, &mut rng),
                    }
                }
                F7Element::Bush { style, bush_w } => {
                    let fade = match rng.random_range(0..3u32) {
                        0 => FadeDir::Down,
                        1 => FadeDir::CenterOut,
                        _ => FadeDir::Up,
                    };
                    let bush = BushSprite {
                        style,
                        x: stop.x as i32,
                        y: stop.root_y as i32,
                        width: bush_w,
                        color,
                        ground: color, // no fade -- preserve ground colors
                        fade,
                        energy,
                    };
                    bush.draw(&mut grid, &mut rng);
                }
                F7Element::Flowers => {
                    let burst = rng.random_range(3..7u32);
                    for _ in 0..burst {
                        let angle = rng.random::<f32>() * std::f32::consts::TAU;
                        let radius = rng.random_range(1..8u32) as f32;
                        let fx = (stop.x as f32 + angle.cos() * radius * 1.5) as i32;
                        let fy = stop.root_y as i32 + rng.random_range(0..3u32) as i32;
                        if fx >= 1
                            && fy >= 1
                            && (fx as usize) < width - 1
                            && (fy as usize) < height - 1
                        {
                            grow_flower_spiral(
                                &mut grid,
                                fx as usize,
                                fy as usize,
                                color,
                                &mut rng,
                            );
                        }
                    }
                }
                F7Element::FruitVine => {
                    let burst = rng.random_range(2..5u32);
                    for _ in 0..burst {
                        let angle = rng.random::<f32>() * std::f32::consts::TAU;
                        let radius = rng.random_range(1..6u32) as f32;
                        let fx = (stop.x as f32 + angle.cos() * radius * 1.5) as i32;
                        let fy = stop.root_y as i32 + rng.random_range(0..2u32) as i32;
                        if fx >= 1
                            && fy >= 1
                            && (fx as usize) < width - 1
                            && (fy as usize) < height - 1
                        {
                            let c = shift_hue(color, rng.random_range(20..80u32) as f64);
                            grow_fruit_vine(&mut grid, fx as usize, fy as usize, c, &mut rng);
                        }
                    }
                    // Braille fruit dots near the vines
                    for _ in 0..rng.random_range(1..4u32) {
                        let fx = stop.x as i32 + rng.random_range(-4..5i32);
                        let fy = stop.root_y as i32 + rng.random_range(-2..3i32);
                        if fx >= 0 && fy >= 0 && (fx as usize) < width && (fy as usize) < height {
                            let fruit_c = shift_hue(color, 60.0);
                            draw_fruit(
                                &mut grid,
                                fx as usize,
                                fy as usize,
                                rng.random_range(0..5),
                                fruit_c,
                            );
                        }
                    }
                }
            }
        }

        // Braille leaf clusters on branch tips
        let leaf_hue = rng.random_range(60..180u32) as f64;
        let leaf_color = hsl_to_rgb(leaf_hue, 0.5, 0.3);
        sprout_leaves(&mut grid, leaf_color, 45, &mut rng);

        // Extra ground-level flower/fruit scatter near tree stops
        for stop in &stops {
            if stop.layer == 0 {
                continue;
            }
            if let F7Element::Tree { .. } = stop.element {
                let burst = rng.random_range(0..3u32);
                for _ in 0..burst {
                    let angle = rng.random::<f32>() * std::f32::consts::TAU;
                    let radius = rng.random_range(1..6u32) as f32;
                    let fx = (stop.x as f32 + angle.cos() * radius * 1.5) as i32;
                    let fy = stop.root_y as i32 + rng.random_range(1..3u32) as i32;
                    if fx >= 1 && fy >= 1 && (fx as usize) < width - 1 && (fy as usize) < height - 1
                    {
                        let c = palette[rng.random_range(2..5)];
                        match rng.random_range(0..3u32) {
                            0 => {
                                grow_flower_spiral(&mut grid, fx as usize, fy as usize, c, &mut rng)
                            }
                            1 => grow_fruit_vine(&mut grid, fx as usize, fy as usize, c, &mut rng),
                            _ => draw_flower(
                                &mut grid,
                                fx as usize,
                                fy as usize,
                                rng.random_range(0..5),
                                c,
                            ),
                        }
                    }
                }
            }
        }
    (grid, false)
}

/// Dispatch arm for mode(s): forest8 (moved verbatim from run()).
pub(crate) fn cli_forest8(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        // forest8 [layers=0] [density=0] -- high-entropy scene-walk forest: trees, bushes, flowers, fruit, grass
        let layers_arg: u8 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(0);
        let density_arg: f32 = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(0.0);
        let layer_count = if layers_arg == 0 {
            (3 + seed % 2) as u8
        } else {
            layers_arg.clamp(2, 5)
        };
        let density = if density_arg == 0.0 {
            0.4
        } else {
            density_arg.clamp(0.2, 1.0)
        };

        let opts = SceneOpts {
            layer_count,
            density,
            tree_rate: 0.62,
            bole_rate: 0.4,
            ground_frac: 0.42,
            kind_filter: None,
            vines: true,
            hue_range: 35.0,
        };
        let (ground_y, stops) = scene_walk(width, height, &mut rng, &opts);

        // sky
        let sky_color = darken(palette[0], 90);
        for y in 0..ground_y {
            for x in 0..width {
                if rng.random_range(0..20u32) == 0 {
                    grid[y][x] = Cell::new('·', sky_color);
                }
            }
        }
        let cloud_color = lighten(palette[0], 12);
        for _ in 0..rng.random_range(0..3u32) {
            let cx = rng.random_range(4..(width - 4) as u32) as usize;
            let cy = rng.random_range(2..(ground_y / 3).max(3) as u32) as usize;
            draw_cloud(
                &mut grid,
                cx,
                cy,
                rng.random_range(8..16u32) as usize,
                cloud_color,
                &mut rng,
            );
        }

        // ground with depth hue gradient + grass
        let ground_chars = ['╱', '╲', '·', '∿', '~', '˜'];
        let grass_chars = ['"', '"', '\'', '·', '˙', '‚'];
        let gdepth = (height - ground_y).max(1);
        for x in 0..width {
            for y in ground_y..height {
                let depth = y - ground_y;
                let t = depth as f64 / gdepth as f64;
                let ch = ground_chars[rng.random_range(0..ground_chars.len() as u32) as usize];
                grid[y][x] = Cell::new(ch, hsl_to_rgb(110.0, 0.35, (0.22 - t * 0.005).max(0.08)));
            }
        }
        // grass tufts, denser near the horizon
        for _ in 0..(width as u32) {
            let gx = rng.random_range(0..width as u32) as usize;
            let gy = ground_y + rng.random_range(0..(gdepth / 3).max(1) as u32) as usize;
            if gy < height {
                grid[gy][gx] = Cell::new(
                    grass_chars[rng.random_range(0..grass_chars.len() as u32) as usize],
                    lighten(palette[2], 14),
                );
            }
        }

        // draw stops back-to-front: trees, bushes, flowers, fruit vines, grass tufts
        for s in &stops {
            let color = hsl_to_rgb(s.hue, s.sat as f64, s.light as f64);
            match s.el {
                SceneEl::Tree {
                    kind,
                    energy,
                    spread,
                    tree_h,
                    bole,
                    taper,
                } => {
                    let canopy_y = s.root_y.saturating_sub(tree_h).max(1);
                    let plot_w = (spread * 2 + 6).min(width);
                    let plot = Rect {
                        x: s.x.saturating_sub(plot_w / 2),
                        y: canopy_y,
                        w: plot_w,
                        h: s.root_y.saturating_sub(canopy_y) + 2,
                    };
                    let tp = TreeParams {
                        plot,
                        energy,
                        trunk_color: color,
                        bark_color: darken(color, 18),
                        branch_color: color,
                        tip_color: lighten(color, 30),
                        fruit_color: shift_hue(color, 60.0),
                        fruit_factor: 0.3,
                        branch_factor: 0.8,
                        direction: GrowDir::Up,
                        bole,
                        taper,
                    };
                    grow_tree_by_index(kind, &mut grid, &tp, &mut rng);
                }
                SceneEl::Bush {
                    style,
                    bush_w,
                    fade,
                } => {
                    let fadedir = match fade % 3 {
                        0 => FadeDir::Down,
                        1 => FadeDir::CenterOut,
                        _ => FadeDir::Up,
                    };
                    BushSprite {
                        style,
                        x: s.x as i32,
                        y: s.root_y as i32,
                        width: bush_w,
                        color,
                        ground: color,
                        fade: fadedir,
                        energy: 0.7,
                    }
                    .draw(&mut grid, &mut rng);
                }
                SceneEl::Flowers => {
                    for _ in 0..rng.random_range(3..7u32) {
                        let angle = rng.random::<f32>() * std::f32::consts::TAU;
                        let radius = rng.random_range(1..8u32) as f32;
                        let fx = (s.x as f32 + angle.cos() * radius * 1.5) as i32;
                        let fy = s.root_y as i32 + rng.random_range(0..3u32) as i32;
                        if fx >= 1 && fy >= 1 && (fx as usize) < width - 1 && (fy as usize) < height - 1
                        {
                            grow_flower_spiral(&mut grid, fx as usize, fy as usize, color, &mut rng);
                        }
                    }
                }
                SceneEl::FruitVine => {
                    for _ in 0..rng.random_range(2..5u32) {
                        let angle = rng.random::<f32>() * std::f32::consts::TAU;
                        let radius = rng.random_range(1..6u32) as f32;
                        let fx = (s.x as f32 + angle.cos() * radius * 1.5) as i32;
                        let fy = s.root_y as i32 + rng.random_range(0..2u32) as i32;
                        if fx >= 1 && fy >= 1 && (fx as usize) < width - 1 && (fy as usize) < height - 1
                        {
                            grow_fruit_vine(
                                &mut grid,
                                fx as usize,
                                fy as usize,
                                shift_hue(color, rng.random_range(20..80u32) as f64),
                                &mut rng,
                            );
                        }
                    }
                    for _ in 0..rng.random_range(1..4u32) {
                        let fx = s.x as i32 + rng.random_range(-4..5i32);
                        let fy = s.root_y as i32 + rng.random_range(-2..3i32);
                        if fx >= 0 && fy >= 0 && (fx as usize) < width && (fy as usize) < height {
                            draw_fruit(
                                &mut grid,
                                fx as usize,
                                fy as usize,
                                rng.random_range(0..5),
                                shift_hue(color, 60.0),
                            );
                        }
                    }
                }
                SceneEl::Grass => {
                    for _ in 0..rng.random_range(2..6u32) {
                        let gx = s.x as i32 + rng.random_range(-3..4i32);
                        let gy = s.root_y as i32 + rng.random_range(0..2i32);
                        if gx >= 0 && gy >= 0 && (gx as usize) < width && (gy as usize) < height {
                            grid[gy as usize][gx as usize] = Cell::new(
                                grass_chars[rng.random_range(0..grass_chars.len() as u32) as usize],
                                lighten(color, 18),
                            );
                        }
                    }
                }
                SceneEl::Gap => {}
            }
        }

        // braille leaf clusters on branch tips
        let leaf_color = hsl_to_rgb(rng.random_range(60..180u32) as f64, 0.5, 0.3);
        sprout_leaves(&mut grid, leaf_color, 15, &mut rng);

        // ground-level flower/fruit scatter near mid + front trees
        for s in &stops {
            if s.layer == 0 {
                continue;
            }
            if let SceneEl::Tree { .. } = s.el {
                for _ in 0..rng.random_range(0..3u32) {
                    let angle = rng.random::<f32>() * std::f32::consts::TAU;
                    let radius = rng.random_range(1..6u32) as f32;
                    let fx = (s.x as f32 + angle.cos() * radius * 1.5) as i32;
                    let fy = s.root_y as i32 + rng.random_range(1..3u32) as i32;
                    if fx >= 1 && fy >= 1 && (fx as usize) < width - 1 && (fy as usize) < height - 1
                    {
                        let c = palette[rng.random_range(2..5)];
                        match rng.random_range(0..3u32) {
                            0 => grow_flower_spiral(&mut grid, fx as usize, fy as usize, c, &mut rng),
                            1 => grow_fruit_vine(&mut grid, fx as usize, fy as usize, c, &mut rng),
                            _ => draw_flower(
                                &mut grid,
                                fx as usize,
                                fy as usize,
                                rng.random_range(0..5),
                                c,
                            ),
                        }
                    }
                }
            }
        }
    (grid, false)
}

/// Dispatch arm for mode(s): forest9 (moved verbatim from run()).
pub(crate) fn cli_forest9(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        // forest9 [layers=0] [fog=0] -- misty high-entropy forest with fog drifts
        let layers_arg: u8 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(0);
        let fog_arg: u64 = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(0);
        let layer_count = if layers_arg == 0 {
            (4 + seed % 2) as u8
        } else {
            layers_arg.clamp(3, 6)
        };
        let fog = if fog_arg == 0 {
            6 + seed % 5
        } else {
            fog_arg.clamp(0, 16)
        };

        let opts = SceneOpts {
            layer_count,
            density: 0.42,
            tree_rate: 0.58,
            bole_rate: 0.5,
            ground_frac: 0.5,
            kind_filter: None,
            vines: true,
            hue_range: 30.0,
        };
        let (ground_y, stops) = scene_walk(width, height, &mut rng, &opts);

        // dusky sky
        let sky_color = darken(palette[0], 70);
        for y in 0..ground_y {
            for x in 0..width {
                if rng.random_range(0..24u32) == 0 {
                    grid[y][x] = Cell::new('·', sky_color);
                }
            }
        }

        // muted ground + grass
        let ground_chars = ['·', '·', '∼', '˜', '╱'];
        let grass_chars = ['"', '\'', '·', '˙', '‚'];
        let gdepth = (height - ground_y).max(1);
        for x in 0..width {
            for y in ground_y..height {
                let depth = y - ground_y;
                let t = depth as f64 / gdepth as f64;
                let ch = ground_chars[rng.random_range(0..ground_chars.len() as u32) as usize];
                grid[y][x] = Cell::new(ch, hsl_to_rgb(95.0, 0.18, (0.16 - t * 0.004).max(0.07)));
            }
        }
        for _ in 0..(width as u32 / 2) {
            let gx = rng.random_range(0..width as u32) as usize;
            let gy = ground_y + rng.random_range(0..(gdepth / 3).max(1) as u32) as usize;
            if gy < height {
                grid[gy][gx] = Cell::new(
                    grass_chars[rng.random_range(0..grass_chars.len() as u32) as usize],
                    lighten(palette[2], 8),
                );
            }
        }

        let mist_color = lighten(palette[0], 6);
        // draw stops back-to-front, colors desaturated for the misty look
        for s in &stops {
            let color = hsl_to_rgb(s.hue, (s.sat * 0.5) as f64, (s.light * 0.9) as f64);
            match s.el {
                SceneEl::Tree {
                    kind,
                    energy,
                    spread,
                    tree_h,
                    bole,
                    taper,
                } => {
                    let canopy_y = s.root_y.saturating_sub(tree_h).max(1);
                    let plot_w = (spread * 2 + 6).min(width);
                    let plot = Rect {
                        x: s.x.saturating_sub(plot_w / 2),
                        y: canopy_y,
                        w: plot_w,
                        h: s.root_y.saturating_sub(canopy_y) + 2,
                    };
                    let tp = TreeParams {
                        plot,
                        energy,
                        trunk_color: color,
                        bark_color: darken(color, 22),
                        branch_color: color,
                        tip_color: lighten(color, 24),
                        fruit_color: shift_hue(color, 40.0),
                        fruit_factor: 0.15,
                        branch_factor: 0.85,
                        direction: GrowDir::Up,
                        bole,
                        taper,
                    };
                    grow_tree_by_index(kind, &mut grid, &tp, &mut rng);
                }
                SceneEl::Bush {
                    style,
                    bush_w,
                    fade,
                } => {
                    let fadedir = match fade % 3 {
                        0 => FadeDir::Down,
                        1 => FadeDir::CenterOut,
                        _ => FadeDir::Up,
                    };
                    BushSprite {
                        style,
                        x: s.x as i32,
                        y: s.root_y as i32,
                        width: bush_w,
                        color,
                        ground: color,
                        fade: fadedir,
                        energy: 0.6,
                    }
                    .draw(&mut grid, &mut rng);
                }
                SceneEl::Flowers => {
                    for _ in 0..rng.random_range(2..6u32) {
                        let angle = rng.random::<f32>() * std::f32::consts::TAU;
                        let radius = rng.random_range(1..7u32) as f32;
                        let fx = (s.x as f32 + angle.cos() * radius * 1.5) as i32;
                        let fy = s.root_y as i32 + rng.random_range(0..3u32) as i32;
                        if fx >= 1 && fy >= 1 && (fx as usize) < width - 1 && (fy as usize) < height - 1
                        {
                            grow_flower_spiral(&mut grid, fx as usize, fy as usize, color, &mut rng);
                        }
                    }
                }
                SceneEl::FruitVine => {
                    for _ in 0..rng.random_range(2..4u32) {
                        let angle = rng.random::<f32>() * std::f32::consts::TAU;
                        let radius = rng.random_range(1..6u32) as f32;
                        let fx = (s.x as f32 + angle.cos() * radius * 1.5) as i32;
                        let fy = s.root_y as i32 + rng.random_range(0..2u32) as i32;
                        if fx >= 1 && fy >= 1 && (fx as usize) < width - 1 && (fy as usize) < height - 1
                        {
                            grow_fruit_vine(
                                &mut grid,
                                fx as usize,
                                fy as usize,
                                shift_hue(color, rng.random_range(20..80u32) as f64),
                                &mut rng,
                            );
                        }
                    }
                }
                SceneEl::Grass => {
                    for _ in 0..rng.random_range(2..5u32) {
                        let gx = s.x as i32 + rng.random_range(-3..4i32);
                        let gy = s.root_y as i32 + rng.random_range(0..2i32);
                        if gx >= 0 && gy >= 0 && (gx as usize) < width && (gy as usize) < height {
                            grid[gy as usize][gx as usize] = Cell::new(
                                grass_chars[rng.random_range(0..grass_chars.len() as u32) as usize],
                                lighten(color, 12),
                            );
                        }
                    }
                }
                SceneEl::Gap => {}
            }
        }

        // dim braille leaf clusters
        let leaf_color = hsl_to_rgb(rng.random_range(80..160u32) as f64, 0.3, 0.26);
        sprout_leaves(&mut grid, leaf_color, 12, &mut rng);

        // mist veil across the canopy region
        for _ in 0..(width as u32 / 3) {
            let fx = rng.random_range(0..width as u32) as usize;
            let fy = rng.random_range(2..ground_y as u32) as usize;
            if rng.random_range(0..2u32) == 0 {
                grid[fy][fx] = Cell::new('░', mist_color);
            }
        }

        // drifting fog streaks
        for _ in 0..fog {
            let fy = rng.random_range(2..ground_y as u32) as usize;
            let fx0 = rng.random_range(0..width as u32) as usize;
            let len = rng.random_range(10..30u32) as usize;
            for dx in 0..len {
                let fx = (fx0 + dx) % width;
                if rng.random_range(0..2u32) == 0 {
                    grid[fy][fx] = Cell::new('░', mist_color);
                }
            }
        }
    (grid, false)
}
