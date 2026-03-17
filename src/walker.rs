use crossterm::style::Color;
use rand::rngs::StdRng;
use rand::RngExt;
use crate::types::*;
use crate::color::*;
use crate::fills::*;
use crate::sprites::*;

#[derive(Clone, Copy)]
pub enum WalkerMood { Organic, Geometric, Empty }

pub enum LeafFill {
    Tree(usize),
    Fret(usize),
    AztecDiamond(usize),
    Flower(usize),
    Fruit(usize),
    Mask(usize, usize),  // (size, style)
    Crosshatch,
    Guilloche,
    Weave,
    Zigzag,
    DiamondLattice,
    Tile(TileParams),
    Noise(NoiseVariant),
    Nothing,
}

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

    pub fn pick_fill(&self, rect: &Rect, rng: &mut StdRng) -> LeafFill {
        let area = rect.w * rect.h;

        // energy-based thinning
        if self.energy < 0.5 && rng.random_range(0..10) < 3 {
            return LeafFill::Nothing;
        }

        match self.mood {
            WalkerMood::Empty => LeafFill::Nothing,
            WalkerMood::Organic => {
                if area > 300 && rect.h > 15 && rect.w > 20 {
                    match rng.random_range(0..7) {
                        0..=2 => LeafFill::Tree(rng.random_range(0..4)),
                        3 => LeafFill::Noise(NoiseVariant::Grass),
                        4 => LeafFill::Noise(NoiseVariant::Dot),
                        _ => {
                            let s = (rect.w.min(rect.h) / 6).max(2).min(4);
                            LeafFill::Mask(s, rng.random_range(0..MASK_STYLE_COUNT))
                        }
                    }
                } else if area > 80 && rect.h > 6 && rect.w > 10 {
                    match rng.random_range(0..7) {
                        0 => LeafFill::Fruit(rng.random_range(0..5)),
                        1 => LeafFill::Noise(NoiseVariant::Grass),
                        2 => {
                            let s = (rect.w.min(rect.h) / 4).max(1).min(3);
                            LeafFill::Mask(s, rng.random_range(0..MASK_STYLE_COUNT))
                        }
                        _ => LeafFill::Flower(rng.random_range(0..5)),
                    }
                } else if area > 20 && rect.w >= 5 && rect.h >= 3 {
                    match rng.random_range(0..4) {
                        0 => LeafFill::Flower(rng.random_range(0..5)),
                        1 => LeafFill::Fruit(rng.random_range(0..5)),
                        _ => LeafFill::Noise(NoiseVariant::Dot),
                    }
                } else {
                    LeafFill::Nothing
                }
            }
            WalkerMood::Geometric => {
                if area > 300 && rect.h > 15 && rect.w > 20 {
                    match rng.random_range(0..10) {
                        0 => { let order = (rect.h / 2).min(rect.w / 4).max(2).min(6); LeafFill::AztecDiamond(order) }
                        1 => LeafFill::Guilloche,
                        2 => LeafFill::Weave,
                        3 => LeafFill::DiamondLattice,
                        4 => LeafFill::Crosshatch,
                        5 => LeafFill::Noise(noise_variant_from_index(rng.random_range(0..NOISE_VARIANT_COUNT))),
                        _ => LeafFill::Tile(TileParams::randomized(rng)),
                    }
                } else if area > 80 && rect.h > 6 && rect.w > 10 {
                    match rng.random_range(0..12) {
                        0 => { let steps = (rect.w.min(rect.h) / 3).max(2).min(5); LeafFill::Fret(steps) }
                        1 => { let order = (rect.h / 2).min(rect.w / 4).max(2).min(6); LeafFill::AztecDiamond(order) }
                        2 => LeafFill::Crosshatch,
                        3 => LeafFill::Guilloche,
                        4 => LeafFill::Weave,
                        5 => LeafFill::Zigzag,
                        6 => LeafFill::DiamondLattice,
                        7 => LeafFill::Noise(noise_variant_from_index(rng.random_range(0..NOISE_VARIANT_COUNT))),
                        _ => LeafFill::Tile(TileParams::randomized(rng)),
                    }
                } else if area > 20 && rect.w >= 5 && rect.h >= 3 {
                    match rng.random_range(0..8) {
                        0 => LeafFill::Flower(rng.random_range(3..5)),
                        1 => LeafFill::Crosshatch,
                        2 => LeafFill::Zigzag,
                        3 => LeafFill::DiamondLattice,
                        4 => LeafFill::Noise(noise_variant_from_index(rng.random_range(0..NOISE_VARIANT_COUNT))),
                        _ => LeafFill::Tile(TileParams::randomized(rng)),
                    }
                } else {
                    LeafFill::Nothing
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
        let cx = rect.x + rect.w / 2;
        let cy = rect.y + rect.h / 2;

        match fill {
            LeafFill::Tree(t) => {
                let root_y = rect.y + rect.h - 2;
                let canopy_y = rect.y + 2;
                match t {
                    0 => grow_tree(grid, cx, root_y, canopy_y, rect.w / 4, prim_color, rng),
                    1 => draw_pine(grid, cx, root_y, 3, (rect.w / 2).min(12), prim_color),
                    2 => draw_willow(grid, cx, root_y, canopy_y, rect.w / 4, prim_color),
                    _ => draw_palm(grid, cx, root_y, rect.h.saturating_sub(4), prim_color, rng),
                }
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
            LeafFill::Fret(steps) => {
                draw_stepped_fret(grid, rect.x as i32 + 2, rect.y as i32 + 1, steps, Dir::Right, prim_color);
            }
            LeafFill::AztecDiamond(order) => {
                draw_aztec_diamond(grid, cx, cy, order, palette, rng);
            }
            LeafFill::Flower(style) => {
                draw_flower(grid, cx, cy, style, prim_color);
            }
            LeafFill::Fruit(style) => {
                draw_fruit(grid, cx, cy, style, prim_color);
            }
            LeafFill::Mask(size, style) => {
                draw_mask(grid, cx, cy, size, style, prim_color);
            }
            LeafFill::Crosshatch => {
                let r = Rect { x: rect.x, y: rect.y, w: rect.w, h: rect.h };
                draw_crosshatch(grid, &r, prim_color, darken(prim_color, 40));
            }
            LeafFill::Guilloche => {
                let r = Rect { x: rect.x, y: rect.y, w: rect.w, h: rect.h };
                draw_guilloche(grid, &r, prim_color, darken(prim_color, 30));
            }
            LeafFill::Weave => {
                let r = Rect { x: rect.x, y: rect.y, w: rect.w, h: rect.h };
                draw_weave(grid, &r, prim_color, lighten(prim_color, 30));
            }
            LeafFill::Zigzag => {
                let r = Rect { x: rect.x, y: rect.y, w: rect.w, h: rect.h };
                draw_zigzag(grid, &r, prim_color, darken(prim_color, 30));
            }
            LeafFill::DiamondLattice => {
                let r = Rect { x: rect.x, y: rect.y, w: rect.w, h: rect.h };
                draw_diamond_lattice(grid, &r, prim_color, darken(prim_color, 30));
            }
            LeafFill::Tile(params) => {
                let r = Rect { x: rect.x, y: rect.y, w: rect.w, h: rect.h };
                let jitter = (1.0 - walker.energy).max(0.0) * 0.15;
                fill_tile_ex(grid, &r, &params, prim_color, darken(prim_color, 30), jitter, rng);
            }
            LeafFill::Noise(variant) => {
                let r = Rect { x: rect.x, y: rect.y, w: rect.w, h: rect.h };
                fill_noise(grid, &r, variant, prim_color, darken(prim_color, 30), rng);
            }
            LeafFill::Nothing => {}
        }

        walker.step(rect, rng);
    }
}
