use crossterm::style::Color;
use rand::rngs::StdRng;
use rand::RngExt;
use crate::types::*;
use crate::color::*;
use crate::fills::*;
use crate::sprites::*;

// ── Types ───────────────────────────────────────────────────────────

/// Unified fill type. Covers rect-filling patterns and positioned sprites.
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

/// Mask function: (x, y) -> 0.0 (outside) to 1.0 (inside), with dissolve in between.
pub type MaskFn = Box<dyn Fn(usize, usize) -> f32>;

/// A single compositing layer: fill + optional mask + palette.
pub struct Layer {
    pub fill: FillGen,
    pub mask: Option<MaskFn>,
    pub palette: [Color; 5],
}

/// Composable scene: ordered stack of layers rendered bottom-to-top.
pub struct Scene {
    pub layers: Vec<Layer>,
}

/// Dissolve glyphs ordered from dense to sparse.
pub const DISSOLVE: &[char] = &['╳', '╱', '╲', '·', '∙', '°', ' '];

// ── Fill dispatch ───────────────────────────────────────────────────

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

// ── Mask composition ────────────────────────────────────────────────

/// Render a fill into `rect`, then mask every cell through `mask_fn`.
pub fn fill_masked(
    grid: &mut Grid,
    rect: &Rect,
    fill: FillGen,
    mask_fn: &dyn Fn(usize, usize) -> f32,
    palette: &[Color; 5],
    rng: &mut StdRng,
) {
    // Save existing cells so we can restore where mask says "outside"
    let y_end = (rect.y + rect.h).min(grid.len());
    let x_end = (rect.x + rect.w).min(if grid.is_empty() { 0 } else { grid[0].len() });
    let saved: Vec<Vec<Cell>> = (rect.y..y_end)
        .map(|y| grid[y][rect.x..x_end].to_vec())
        .collect();

    let c1 = palette[1];
    let c2 = darken(c1, 30);
    render_fill(grid, rect, fill, c1, c2, palette, rng);

    let dissolve_color = darken(palette[1], 40);
    for y in rect.y..y_end {
        for x in rect.x..x_end {
            let v = mask_fn(x, y);
            if v >= 1.0 {
                // fully inside, keep new fill
            } else if v <= 0.0 {
                // fully outside, restore what was there before
                grid[y][x] = saved[y - rect.y][x - rect.x];
            } else {
                // dissolve zone
                let r: f32 = rng.random::<f32>();
                if r > v {
                    if r < v + 0.15 {
                        let ch = DISSOLVE[rng.random_range(3..6)];
                        grid[y][x] = Cell::new(ch, dissolve_color);
                    } else {
                        grid[y][x] = saved[y - rect.y][x - rect.x];
                    }
                }
            }
        }
    }
}

/// Render a scene: iterate layers bottom-to-top, compositing each.
pub fn render_scene(
    grid: &mut Grid,
    rect: &Rect,
    scene: &Scene,
    rng: &mut StdRng,
) {
    for layer in &scene.layers {
        match &layer.mask {
            Some(mask) => fill_masked(grid, rect, layer.fill, mask.as_ref(), &layer.palette, rng),
            None => {
                let c1 = layer.palette[1];
                let c2 = darken(c1, 30);
                render_fill(grid, rect, layer.fill, c1, c2, &layer.palette, rng);
            }
        }
    }
}

// ── Mask constructors ───────────────────────────────────────────────

/// Circle/ellipse mask centered at (cx, cy).
pub fn mask_ellipse(cx: f32, cy: f32, rx: f32, ry: f32, dissolve: f32) -> impl Fn(usize, usize) -> f32 {
    move |x, y| {
        let dx = (x as f32 - cx) / rx;
        let dy = (y as f32 - cy) / ry;
        let d = (dx * dx + dy * dy).sqrt();
        if d <= 1.0 {
            1.0
        } else if d <= 1.0 + dissolve {
            1.0 - (d - 1.0) / dissolve
        } else {
            0.0
        }
    }
}

/// Contour mask: inside below the contour line.
/// `contour[x - x_offset]` gives the y-threshold.
pub fn mask_below_contour(contour: Vec<usize>, x_offset: usize, dissolve: f32) -> impl Fn(usize, usize) -> f32 {
    move |x, y| {
        let col = x.saturating_sub(x_offset);
        let threshold = contour.get(col).copied().unwrap_or(usize::MAX);
        if y >= threshold {
            1.0
        } else if y as f32 >= threshold as f32 - dissolve {
            let dist = threshold as f32 - y as f32;
            1.0 - dist / dissolve
        } else {
            0.0
        }
    }
}

/// Contour mask: inside above the contour line.
pub fn mask_above_contour(contour: Vec<usize>, x_offset: usize, dissolve: f32) -> impl Fn(usize, usize) -> f32 {
    move |x, y| {
        let col = x.saturating_sub(x_offset);
        let threshold = contour.get(col).copied().unwrap_or(0);
        if y <= threshold {
            1.0
        } else if (y as f32) <= threshold as f32 + dissolve {
            let dist = y as f32 - threshold as f32;
            1.0 - dist / dissolve
        } else {
            0.0
        }
    }
}

/// Horizontal band mask with dissolve at top and bottom.
pub fn mask_band(y_top: usize, y_bot: usize, dissolve: f32) -> impl Fn(usize, usize) -> f32 {
    move |_x, y| {
        if y >= y_top && y <= y_bot {
            let from_top = (y - y_top) as f32;
            let from_bot = (y_bot - y) as f32;
            let edge_dist = from_top.min(from_bot);
            if edge_dist >= dissolve { 1.0 } else { edge_dist / dissolve }
        } else {
            0.0
        }
    }
}

/// Rectangle mask: 1.0 inside rect, dissolve at edges, 0.0 outside.
pub fn mask_rect(rect: &Rect, dissolve: f32) -> impl Fn(usize, usize) -> f32 + use<> {
    let x0 = rect.x;
    let y0 = rect.y;
    let x1 = rect.x + rect.w;
    let y1 = rect.y + rect.h;
    move |x, y| {
        if x < x0 || x >= x1 || y < y0 || y >= y1 {
            0.0
        } else if dissolve <= 0.0 {
            1.0
        } else {
            let from_left = (x - x0) as f32;
            let from_right = (x1 - 1 - x) as f32;
            let from_top = (y - y0) as f32;
            let from_bot = (y1 - 1 - y) as f32;
            let edge = from_left.min(from_right).min(from_top).min(from_bot);
            if edge >= dissolve { 1.0 } else { edge / dissolve }
        }
    }
}

/// Combine two masks: intersection (min of both values).
pub fn mask_intersect(
    a: impl Fn(usize, usize) -> f32 + 'static,
    b: impl Fn(usize, usize) -> f32 + 'static,
) -> MaskFn {
    Box::new(move |x, y| a(x, y).min(b(x, y)))
}

/// Combine two masks: union (max of both values).
pub fn mask_union(
    a: impl Fn(usize, usize) -> f32 + 'static,
    b: impl Fn(usize, usize) -> f32 + 'static,
) -> MaskFn {
    Box::new(move |x, y| a(x, y).max(b(x, y)))
}
