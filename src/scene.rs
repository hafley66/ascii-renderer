use crossterm::style::Color;
use rand::rngs::StdRng;
use crate::types::*;
use crate::color::*;
use crate::fills::*;
use crate::sprites::*;

/// Unified fill type. Covers rect-filling patterns and positioned sprites.
/// Replaces the separate LeafFill (walker) and FlowFill (biomes) enums.
#[derive(Clone, Copy)]
pub enum FillGen {
    // Rect fills -- cover entire rect with pattern
    TilePure(TileVariant),
    Tile(TileParams),
    Noise(NoiseVariant),
    Crosshatch,
    Guilloche,
    Weave,
    Zigzag,
    DiamondLattice,
    // Sprites -- positioned within rect
    Tree(usize),           // tree type 0-3
    AztecDiamond(usize),   // order
    Flower(usize),         // style 0-4
    Fruit(usize),          // style 0-4
    Mask(usize, usize),    // (size, style)
    Fret(usize),           // steps
    // No-op
    Nothing,
}

/// Render a fill into a rect. Universal dispatch for all fill types.
///
/// `color`/`color2` are the caller's chosen primary/secondary colors.
/// `palette` is passed through for fills that need the full set (aztec diamond).
pub fn render_fill(
    grid: &mut Grid,
    rect: &Rect,
    fill: FillGen,
    color: Color,
    color2: Color,
    palette: &[Color; 5],
    rng: &mut StdRng,
) {
    match fill {
        FillGen::TilePure(v) => fill_tile_pure(grid, rect, v, color, color2),
        FillGen::Tile(params) => fill_tile_ex(grid, rect, &params, color, color2, params.jitter, rng),
        FillGen::Noise(v) => fill_noise(grid, rect, v, color, color2, rng),
        FillGen::Crosshatch => draw_crosshatch(grid, rect, color, color2),
        FillGen::Guilloche => draw_guilloche(grid, rect, color, color2),
        FillGen::Weave => draw_weave(grid, rect, color, lighten(color, 30)),
        FillGen::Zigzag => draw_zigzag(grid, rect, color, color2),
        FillGen::DiamondLattice => draw_diamond_lattice(grid, rect, color, color2),
        FillGen::Tree(kind) => {
            let cx = rect.x + rect.w / 2;
            let root_y = rect.y + rect.h.saturating_sub(2);
            let canopy_y = rect.y + 2;
            let spread = (rect.w / 4).max(3);
            match kind % 4 {
                0 => grow_tree(grid, cx, root_y, canopy_y, spread, color, rng),
                1 => draw_pine(grid, cx, root_y, 3, (rect.w / 2).min(12), color),
                2 => draw_willow(grid, cx, root_y, canopy_y, spread, color),
                _ => draw_palm(grid, cx, root_y, rect.h.saturating_sub(4), color, rng),
            }
        }
        FillGen::AztecDiamond(order) => {
            let cx = rect.x + rect.w / 2;
            let cy = rect.y + rect.h / 2;
            draw_aztec_diamond(grid, cx, cy, order, palette, rng);
        }
        FillGen::Flower(style) => {
            let cx = rect.x + rect.w / 2;
            let cy = rect.y + rect.h / 2;
            draw_flower(grid, cx, cy, style, color);
        }
        FillGen::Fruit(style) => {
            let cx = rect.x + rect.w / 2;
            let cy = rect.y + rect.h / 2;
            draw_fruit(grid, cx, cy, style, color);
        }
        FillGen::Mask(size, style) => {
            let cx = rect.x + rect.w / 2;
            let cy = rect.y + rect.h / 2;
            draw_mask(grid, cx, cy, size, style, color);
        }
        FillGen::Fret(steps) => {
            draw_stepped_fret(grid, rect.x as i32 + 2, rect.y as i32 + 1, steps, Dir::Right, color);
        }
        FillGen::Nothing => {}
    }
}
