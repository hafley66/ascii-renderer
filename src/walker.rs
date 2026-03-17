use crossterm::style::Color;
use rand::rngs::StdRng;
use rand::RngExt;
use crate::types::*;
use crate::color::*;
use crate::fills::*;
use crate::sprites::*;
use crate::scene::*;

#[derive(Clone, Copy)]
pub enum WalkerMood { Organic, Geometric, Empty }

pub struct LeafWalker {
    pub mood: WalkerMood,
    pub energy: f32,
    pub prev_x: usize,
    pub prev_y: usize,
}

impl LeafWalker {
    pub fn new(center_x: usize, center_y: usize) -> Self {
        LeafWalker { mood: WalkerMood::Organic, energy: 1.0, prev_x: center_x, prev_y: center_y }
    }

    pub fn pick_fill(&self, rect: &Rect, rng: &mut StdRng) -> FillGen {
        let area = rect.w * rect.h;

        // energy-based thinning
        if self.energy < 0.5 && rng.random_range(0..10) < 3 {
            return FillGen::Nothing;
        }

        match self.mood {
            WalkerMood::Empty => FillGen::Nothing,
            WalkerMood::Organic => {
                if area > 300 && rect.h > 15 && rect.w > 20 {
                    match rng.random_range(0..7) {
                        0..=2 => FillGen::Tree(rng.random_range(0..4)),
                        3 => FillGen::Noise(NoiseVariant::Grass),
                        4 => FillGen::Noise(NoiseVariant::Dot),
                        _ => {
                            let s = (rect.w.min(rect.h) / 6).max(2).min(4);
                            FillGen::Mask(s, rng.random_range(0..MASK_STYLE_COUNT))
                        }
                    }
                } else if area > 80 && rect.h > 6 && rect.w > 10 {
                    match rng.random_range(0..7) {
                        0 => FillGen::Fruit(rng.random_range(0..5)),
                        1 => FillGen::Noise(NoiseVariant::Grass),
                        2 => {
                            let s = (rect.w.min(rect.h) / 4).max(1).min(3);
                            FillGen::Mask(s, rng.random_range(0..MASK_STYLE_COUNT))
                        }
                        _ => FillGen::Flower(rng.random_range(0..5)),
                    }
                } else if area > 20 && rect.w >= 5 && rect.h >= 3 {
                    match rng.random_range(0..4) {
                        0 => FillGen::Flower(rng.random_range(0..5)),
                        1 => FillGen::Fruit(rng.random_range(0..5)),
                        _ => FillGen::Noise(NoiseVariant::Dot),
                    }
                } else {
                    FillGen::Nothing
                }
            }
            WalkerMood::Geometric => {
                if area > 300 && rect.h > 15 && rect.w > 20 {
                    match rng.random_range(0..10) {
                        0 => { let order = (rect.h / 2).min(rect.w / 4).max(2).min(6); FillGen::AztecDiamond(order) }
                        1 => FillGen::Guilloche,
                        2 => FillGen::Weave,
                        3 => FillGen::DiamondLattice,
                        4 => FillGen::Crosshatch,
                        5 => FillGen::Noise(noise_variant_from_index(rng.random_range(0..NOISE_VARIANT_COUNT))),
                        _ => FillGen::Tile(TileParams::randomized(rng)),
                    }
                } else if area > 80 && rect.h > 6 && rect.w > 10 {
                    match rng.random_range(0..12) {
                        0 => { let steps = (rect.w.min(rect.h) / 3).max(2).min(5); FillGen::Fret(steps) }
                        1 => { let order = (rect.h / 2).min(rect.w / 4).max(2).min(6); FillGen::AztecDiamond(order) }
                        2 => FillGen::Crosshatch,
                        3 => FillGen::Guilloche,
                        4 => FillGen::Weave,
                        5 => FillGen::Zigzag,
                        6 => FillGen::DiamondLattice,
                        7 => FillGen::Noise(noise_variant_from_index(rng.random_range(0..NOISE_VARIANT_COUNT))),
                        _ => FillGen::Tile(TileParams::randomized(rng)),
                    }
                } else if area > 20 && rect.w >= 5 && rect.h >= 3 {
                    match rng.random_range(0..8) {
                        0 => FillGen::Flower(rng.random_range(3..5)),
                        1 => FillGen::Crosshatch,
                        2 => FillGen::Zigzag,
                        3 => FillGen::DiamondLattice,
                        4 => FillGen::Noise(noise_variant_from_index(rng.random_range(0..NOISE_VARIANT_COUNT))),
                        _ => FillGen::Tile(TileParams::randomized(rng)),
                    }
                } else {
                    FillGen::Nothing
                }
            }
        }
    }

    pub fn step(&mut self, rect: &Rect, rng: &mut StdRng) {
        self.prev_x = rect.x + rect.w / 2;
        self.prev_y = rect.y + rect.h / 2;
        self.energy -= rng.random_range(15..26) as f32 / 100.0;

        if matches!(self.mood, WalkerMood::Empty) {
            self.energy = 0.6;
            self.mood = if rng.random_range(0..2) == 0 { WalkerMood::Organic } else { WalkerMood::Geometric };
        } else if self.energy < 0.3 {
            self.mood = match self.mood {
                WalkerMood::Organic => {
                    if rng.random_range(0..10) < 7 { WalkerMood::Geometric } else { WalkerMood::Empty }
                }
                WalkerMood::Geometric => {
                    if rng.random_range(0..10) < 5 { WalkerMood::Organic } else { WalkerMood::Empty }
                }
                WalkerMood::Empty => unreachable!(),
            };
        }
    }
}

/// Walk through leaf rects in nearest-neighbor order, filling each based on
/// walker mood/energy state.
pub fn walk_and_fill_leaves(
    grid: &mut Grid,
    leaves: &[Rect],
    palette: &[Color; 5],
    rng: &mut StdRng,
) {
    if leaves.is_empty() { return; }

    let mut walker = LeafWalker::new(grid[0].len() / 2, grid.len() / 2);
    let mut visited = vec![false; leaves.len()];

    for _ in 0..leaves.len() {
        let mut best_idx = None;
        let mut best_dist = usize::MAX;
        for (i, leaf) in leaves.iter().enumerate() {
            if visited[i] { continue; }
            let cx = leaf.x + leaf.w / 2;
            let cy = leaf.y + leaf.h / 2;
            let dist = cx.abs_diff(walker.prev_x) + cy.abs_diff(walker.prev_y);
            if dist < best_dist {
                best_dist = dist;
                best_idx = Some(i);
            }
        }

        let Some(idx) = best_idx else { break };
        visited[idx] = true;
        let rect = &leaves[idx];
        let fill = walker.pick_fill(rect, rng);
        let prim_color = palette[rng.random_range(1..4)];
        let color2 = darken(prim_color, 30);

        // Tree gets scatter on top; everything else goes through render_fill
        match fill {
            FillGen::Tree(_) => {
                render_fill(grid, rect, fill, prim_color, color2, palette, rng);
                for _ in 0..rng.random_range(1..=3) {
                    let fx = rect.x + rng.random_range(2..rect.w.saturating_sub(2).max(3));
                    let fy = rect.y + rng.random_range(2..rect.h.saturating_sub(2).max(3));
                    if rng.random_range(0..2) == 0 {
                        draw_fruit(grid, fx, fy, rng.random_range(0..5), palette[3]);
                    } else {
                        draw_flower(grid, fx, fy, rng.random_range(0..5), palette[3]);
                    }
                }
            }
            FillGen::Tile(params) => {
                let jittered = TileParams { jitter: (1.0 - walker.energy).max(0.0) * 0.15, ..params };
                render_fill(grid, rect, FillGen::Tile(jittered), prim_color, color2, palette, rng);
            }
            _ => render_fill(grid, rect, fill, prim_color, color2, palette, rng),
        }

        walker.step(rect, rng);
    }
}
