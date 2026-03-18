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
    Spiral,
    Concentric,
    Labyrinth,
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
/// `skew_boundary`: optional shape boundary fn forwarded to tile fills for
/// shape-aware edge dissolution. Pass `None` for default rect-edge behavior.
pub fn render_fill(
    grid: &mut Grid,
    rect: &Rect,
    fill: FillGen,
    color: Color,
    color2: Color,
    palette: &[Color; 5],
    skew_boundary: Option<&dyn Fn(usize, usize) -> f32>,
    rng: &mut StdRng,
) {
    match fill {
        FillGen::TilePure(v) => fill_tile_pure(grid, rect, v, color, color2),
        FillGen::Tile(params) => fill_tile_ex(grid, rect, &params, color, color2, params.jitter, skew_boundary, rng),
        FillGen::Noise(v) => fill_noise(grid, rect, v, color, color2, rng),
        FillGen::Crosshatch => draw_crosshatch(grid, rect, color, color2),
        FillGen::Guilloche => draw_guilloche(grid, rect, color, color2),
        FillGen::Weave => draw_weave(grid, rect, color, lighten(color, 30)),
        FillGen::Zigzag => draw_zigzag(grid, rect, color, color2),
        FillGen::DiamondLattice => draw_diamond_lattice(grid, rect, color, color2),
        FillGen::Spiral => draw_spiral(grid, rect, color, color2),
        FillGen::Concentric => draw_concentric(grid, rect, color, color2),
        FillGen::Labyrinth => draw_labyrinth(grid, rect, color, color2),
        FillGen::Tree(kind) => {
            let cx = rect.x + rect.w / 2;
            let root_y = rect.y + rect.h.saturating_sub(2);
            let canopy_y = rect.y + 2;
            let spread = (rect.w / 4).max(3);
            draw_tree(grid, cx, root_y, canopy_y, spread, kind, color, rng);
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
    // Pass mask_fn as skew_boundary so tile fills dissolve along the container's
    // shape contour rather than uniformly from rect edges.
    render_fill(grid, rect, fill, c1, c2, palette, Some(mask_fn), rng);

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
                render_fill(grid, rect, layer.fill, c1, c2, &layer.palette, None, rng);
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

// ── Extended shape masks ─────────────────────────────────────────────

/// Diamond (rhombus) mask: L1 norm with soft dissolve at edges.
/// rx/ry are half-widths along each axis. In terminal cells x:y ≈ 2:1,
/// so pass rx ≈ 2*ry for a visually balanced diamond.
pub fn mask_diamond(cx: f32, cy: f32, rx: f32, ry: f32, dissolve: f32) -> impl Fn(usize, usize) -> f32 {
    move |x, y| {
        let dx = (x as f32 - cx) / rx;
        let dy = (y as f32 - cy) / ry;
        let d = dx.abs() + dy.abs();
        if d <= 1.0 {
            1.0
        } else if dissolve > 0.0 && d <= 1.0 + dissolve {
            1.0 - (d - 1.0) / dissolve
        } else {
            0.0
        }
    }
}

/// Parallelogram mask: a rectangle sheared along x by `shear` cells per
/// unit of normalized dy from center. Positive shear leans right going down.
/// w/h are full width and height, centered at (cx, cy).
pub fn mask_parallelogram(cx: f32, cy: f32, w: f32, h: f32, shear: f32, dissolve: f32) -> impl Fn(usize, usize) -> f32 {
    move |x, y| {
        let dy = y as f32 - cy;
        let sx = x as f32 - cx - shear * (dy / (h * 0.5));
        let from_left  = sx + w * 0.5;
        let from_right = w * 0.5 - sx;
        let from_top   = dy + h * 0.5;
        let from_bot   = h * 0.5 - dy;
        let edge = from_left.min(from_right).min(from_top).min(from_bot);
        if edge <= 0.0 { 0.0 }
        else if dissolve <= 0.0 || edge >= dissolve { 1.0 }
        else { edge / dissolve }
    }
}

/// Which direction the apex of a triangle points.
#[derive(Clone, Copy)]
pub enum TriDir { Up, Down, Left, Right }

/// Triangle mask. Apex points in `dir`, base is opposite.
/// rx/ry are half-extents from center to the widest cross-section.
pub fn mask_triangle(cx: f32, cy: f32, rx: f32, ry: f32, dir: TriDir, dissolve: f32) -> impl Fn(usize, usize) -> f32 {
    move |x, y| {
        let fx = x as f32;
        let fy = y as f32;
        // t: 0.0 at apex, 1.0 at base. x_off: 1.0 means on the slanted edge.
        // base_dist: pixel distance from the base edge (opposite the apex).
        let (t, x_off, base_dist) = match dir {
            TriDir::Up => {
                let t = (fy - (cy - ry)) / (2.0 * ry);
                (t, (fx - cx).abs() / (t * rx + 0.01), (cy + ry) - fy)
            }
            TriDir::Down => {
                let t = ((cy + ry) - fy) / (2.0 * ry);
                (t, (fx - cx).abs() / (t * rx + 0.01), fy - (cy - ry))
            }
            TriDir::Left => {
                let t = ((cx + rx) - fx) / (2.0 * rx);
                (t, (fy - cy).abs() / (t * ry + 0.01), fx - (cx - rx))
            }
            TriDir::Right => {
                let t = (fx - (cx - rx)) / (2.0 * rx);
                (t, (fy - cy).abs() / (t * ry + 0.01), (cx + rx) - fx)
            }
        };

        if t < 0.0 || t > 1.0 || x_off > 1.0 { return 0.0; }

        // Approximate pixel distance from slanted edge, capped by base distance.
        let slant_dist = (1.0 - x_off) * (t * rx.min(ry));
        let edge = slant_dist.min(base_dist);

        if edge <= 0.0 { 0.0 }
        else if dissolve <= 0.0 || edge >= dissolve { 1.0 }
        else { edge / dissolve }
    }
}

/// Hexagon mask. Regular hexagon with flat top/bottom.
/// rx is half-width (horizontal), ry is half-height (vertical).
/// Hex shape: top/bottom edges are flat (w = rx), sides are angled.
pub fn mask_hexagon(cx: f32, cy: f32, rx: f32, ry: f32, dissolve: f32) -> impl Fn(usize, usize) -> f32 {
    move |x, y| {
        let dx = (x as f32 - cx).abs();
        let dy = (y as f32 - cy).abs();
        // Hex: flat top means the constraint is:
        //   dy <= ry  AND  dx + dy * (rx / ry) * 0.5 <= rx
        // Normalized: test against the hex boundary
        let ny = dy / ry;
        let nx = dx / rx;
        // Hex distance: max of vertical and combined
        let d = ny.max(nx + ny * 0.5);
        if d <= 1.0 {
            1.0
        } else if dissolve > 0.0 && d <= 1.0 + dissolve / rx.min(ry) {
            1.0 - (d - 1.0) / (dissolve / rx.min(ry))
        } else {
            0.0
        }
    }
}

/// Trapezoid mask. Top edge width `w_top`, bottom edge width `w_bot`,
/// total height `h`, centered at (cx, cy). Sides taper linearly.
pub fn mask_trapezoid(cx: f32, cy: f32, w_top: f32, w_bot: f32, h: f32, dissolve: f32) -> impl Fn(usize, usize) -> f32 {
    move |x, y| {
        let dy = y as f32 - cy;
        let from_top = dy + h * 0.5;
        let from_bot = h * 0.5 - dy;
        if from_top < 0.0 || from_bot < 0.0 { return 0.0; }
        let t = from_top / h; // 0.0 at top edge, 1.0 at bottom edge
        let half_w = w_top * 0.5 * (1.0 - t) + w_bot * 0.5 * t;
        let from_side = half_w - (x as f32 - cx).abs();
        let edge = from_side.min(from_top).min(from_bot);
        if edge <= 0.0 { 0.0 }
        else if dissolve <= 0.0 || edge >= dissolve { 1.0 }
        else { edge / dissolve }
    }
}
