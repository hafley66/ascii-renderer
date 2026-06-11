use crate::color::*;
use crate::types::*;
use crossterm::style::Color;
use rand::RngExt;
use rand::rngs::StdRng;

// ── Tile pattern system ─────────────────────────────────────────────

/// A tile is a small rectangular char grid that repeats to fill any area.
///
/// Stagger controls:
/// - `row_offset`: how many columns each tile-row shifts rightward
/// - `stagger_rhythm`: how many tile-rows before the offset resets
///   (1 = every row staggers, 2 = every other row, etc.)
pub struct TilePattern {
    pub cells: Vec<Vec<(char, u8)>>, // [y][x] -> (char, color_index: 0=primary, 1=secondary)
    pub row_offset: usize,           // x-shift per stagger group (0 = no stagger)
    pub stagger_rhythm: usize,       // tile-rows per stagger group (1 = every row, 2 = pairs, etc.)
}

impl TilePattern {
    pub fn period_x(&self) -> usize {
        self.cells[0].len()
    }
    pub fn period_y(&self) -> usize {
        self.cells.len()
    }

    pub fn at(&self, x: usize, y: usize) -> (char, u8) {
        let py = self.period_y();
        let px = self.period_x();
        let ty = y % py;
        let tile_row = y / py;
        let group = tile_row / self.stagger_rhythm;
        let tx = (x + group * self.row_offset) % px;
        self.cells[ty][tx]
    }
}

#[derive(Clone, Copy)]
pub enum TileVariant {
    Asanoha,
    Seigaiha,
    Shippo,
    BishamonKikko,
    Yabane,
    Nowaki,
    Higaki,
    ShellStitch,
    GrannySquare,
    CrocodileScale,
}

pub const TILE_VARIANT_COUNT: usize = 10;

pub fn tile_variant_from_index(i: usize) -> TileVariant {
    match i % TILE_VARIANT_COUNT {
        0 => TileVariant::Asanoha,
        1 => TileVariant::Seigaiha,
        2 => TileVariant::Shippo,
        3 => TileVariant::BishamonKikko,
        4 => TileVariant::Yabane,
        5 => TileVariant::Nowaki,
        6 => TileVariant::Higaki,
        7 => TileVariant::ShellStitch,
        8 => TileVariant::GrannySquare,
        _ => TileVariant::CrocodileScale,
    }
}

/// Build the repeating char grid for a tile variant.
pub fn make_tile(variant: TileVariant) -> TilePattern {
    match variant {
        TileVariant::Asanoha => {
            let g = vec![
                vec![
                    ('╲', 0),
                    ('╱', 1),
                    ('╲', 0),
                    ('╱', 1),
                    ('╲', 1),
                    ('╱', 0),
                    ('╲', 1),
                    ('╱', 0),
                ],
                vec![
                    ('╱', 0),
                    ('╲', 1),
                    ('─', 0),
                    ('─', 0),
                    ('─', 1),
                    ('─', 1),
                    ('╱', 1),
                    ('╲', 0),
                ],
                vec![
                    ('╲', 1),
                    ('╱', 0),
                    ('╲', 1),
                    ('╱', 0),
                    ('╲', 0),
                    ('╱', 1),
                    ('╲', 0),
                    ('╱', 1),
                ],
                vec![
                    ('╱', 1),
                    ('╲', 0),
                    ('─', 1),
                    ('─', 1),
                    ('─', 0),
                    ('─', 0),
                    ('╱', 0),
                    ('╲', 1),
                ],
            ];
            TilePattern {
                cells: g,
                row_offset: 4,
                stagger_rhythm: 1,
            }
        }
        TileVariant::Seigaiha => {
            let g = vec![
                vec![
                    ('╰', 0),
                    ('─', 0),
                    ('─', 0),
                    ('╯', 0),
                    ('╰', 1),
                    ('─', 1),
                    ('─', 1),
                    ('╯', 1),
                ],
                vec![
                    (' ', 0),
                    ('╭', 0),
                    ('╮', 0),
                    (' ', 0),
                    (' ', 1),
                    ('╭', 1),
                    ('╮', 1),
                    (' ', 1),
                ],
                vec![
                    (' ', 0),
                    ('│', 0),
                    ('│', 0),
                    (' ', 0),
                    (' ', 1),
                    ('│', 1),
                    ('│', 1),
                    (' ', 1),
                ],
                vec![
                    ('╭', 0),
                    ('╯', 0),
                    ('╰', 0),
                    ('╮', 0),
                    ('╭', 1),
                    ('╯', 1),
                    ('╰', 1),
                    ('╮', 1),
                ],
            ];
            TilePattern {
                cells: g,
                row_offset: 4,
                stagger_rhythm: 1,
            }
        }
        TileVariant::Shippo => {
            let g = vec![
                vec![('╲', 0), ('╭', 1), ('─', 1), ('╮', 1), ('╱', 0), (' ', 0)],
                vec![(' ', 0), ('│', 1), (' ', 0), ('│', 1), (' ', 0), (' ', 0)],
                vec![('╱', 0), ('╰', 1), ('─', 1), ('╯', 1), ('╲', 0), (' ', 0)],
                vec![(' ', 0), (' ', 0), ('╲', 0), ('╱', 0), (' ', 0), (' ', 0)],
            ];
            TilePattern {
                cells: g,
                row_offset: 3,
                stagger_rhythm: 1,
            }
        }
        TileVariant::BishamonKikko => {
            let g = vec![
                vec![('╱', 0), ('─', 0), ('─', 0), ('╲', 0), (' ', 1), (' ', 1)],
                vec![('│', 0), (' ', 0), (' ', 0), ('│', 0), (' ', 1), (' ', 1)],
                vec![('╲', 0), ('─', 0), ('─', 0), ('╱', 0), (' ', 1), (' ', 1)],
                vec![(' ', 0), (' ', 0), (' ', 0), (' ', 0), (' ', 1), (' ', 1)],
            ];
            TilePattern {
                cells: g,
                row_offset: 3,
                stagger_rhythm: 1,
            }
        }
        TileVariant::Yabane => {
            let g = vec![
                vec![('╱', 0), ('╱', 0), ('╱', 0), ('╲', 1), ('╲', 1), ('╲', 1)],
                vec![('╱', 0), ('╱', 0), ('╱', 0), ('╲', 1), ('╲', 1), ('╲', 1)],
                vec![('╲', 1), ('╲', 1), ('╲', 1), ('╱', 0), ('╱', 0), ('╱', 0)],
                vec![('╲', 1), ('╲', 1), ('╲', 1), ('╱', 0), ('╱', 0), ('╱', 0)],
            ];
            TilePattern {
                cells: g,
                row_offset: 0,
                stagger_rhythm: 1,
            }
        }
        TileVariant::Nowaki => {
            let g = vec![
                vec![('│', 0), (' ', 0), ('╱', 1), (' ', 0)],
                vec![('│', 0), ('╱', 1), (' ', 0), (' ', 0)],
                vec![('╱', 1), (' ', 0), (' ', 0), ('│', 0)],
                vec![(' ', 0), (' ', 0), ('│', 0), (' ', 0)],
                vec![(' ', 0), ('│', 0), (' ', 0), ('╱', 1)],
                vec![('│', 0), (' ', 0), ('╱', 1), (' ', 0)],
            ];
            TilePattern {
                cells: g,
                row_offset: 0,
                stagger_rhythm: 1,
            }
        }
        TileVariant::Higaki => {
            let g = vec![
                vec![('╱', 0), ('╳', 1), ('╲', 0), (' ', 0)],
                vec![('╳', 1), ('╲', 0), (' ', 0), ('╱', 0)],
                vec![('╲', 0), (' ', 0), ('╱', 0), ('╳', 1)],
                vec![(' ', 0), ('╱', 0), ('╳', 1), ('╲', 0)],
            ];
            TilePattern {
                cells: g,
                row_offset: 0,
                stagger_rhythm: 1,
            }
        }
        TileVariant::ShellStitch => {
            let g = vec![
                vec![
                    ('╰', 0),
                    ('─', 0),
                    ('╮', 0),
                    (' ', 0),
                    (' ', 0),
                    ('╭', 1),
                    ('─', 1),
                    ('╯', 1),
                ],
                vec![
                    (' ', 0),
                    (' ', 0),
                    ('│', 0),
                    ('◠', 0),
                    ('◠', 1),
                    ('│', 1),
                    (' ', 1),
                    (' ', 1),
                ],
                vec![
                    ('─', 0),
                    ('╮', 0),
                    ('╰', 0),
                    ('─', 0),
                    ('─', 1),
                    ('╯', 1),
                    ('╭', 1),
                    ('─', 1),
                ],
            ];
            TilePattern {
                cells: g,
                row_offset: 4,
                stagger_rhythm: 1,
            }
        }
        TileVariant::GrannySquare => {
            let g = vec![
                vec![('┌', 0), ('─', 0), ('┬', 1), ('┬', 1), ('─', 0), ('┐', 0)],
                vec![('│', 0), ('╭', 1), ('─', 1), ('─', 1), ('╮', 1), ('│', 0)],
                vec![('├', 0), ('│', 1), ('·', 0), ('·', 0), ('│', 1), ('┤', 0)],
                vec![('├', 0), ('│', 1), ('·', 0), ('·', 0), ('│', 1), ('┤', 0)],
                vec![('│', 0), ('╰', 1), ('─', 1), ('─', 1), ('╯', 1), ('│', 0)],
                vec![('└', 0), ('─', 0), ('┴', 1), ('┴', 1), ('─', 0), ('┘', 0)],
            ];
            TilePattern {
                cells: g,
                row_offset: 0,
                stagger_rhythm: 1,
            }
        }
        TileVariant::CrocodileScale => {
            let g = vec![
                vec![('╲', 0), (' ', 0), (' ', 0), (' ', 0), (' ', 0), ('╱', 0)],
                vec![(' ', 0), ('╲', 0), ('▁', 1), ('▁', 1), ('╱', 0), (' ', 0)],
                vec![(' ', 0), ('▕', 1), ('▓', 1), ('▓', 1), ('▏', 1), (' ', 0)],
                vec![('─', 0), ('╯', 0), (' ', 0), (' ', 0), ('╰', 0), ('─', 0)],
            ];
            TilePattern {
                cells: g,
                row_offset: 3,
                stagger_rhythm: 1,
            }
        }
    }
}

/// Randomizable parameters for a tile fill instance.
#[derive(Clone, Copy)]
pub struct TileParams {
    pub variant: TileVariant,
    pub density: f32,         // 0.0-1.0, cell draw probability
    pub stagger_override: i8, // -1 = use default, 0 = force no stagger, 1+ = override offset
    pub rhythm_override: u8,  // 0 = use default, 1+ = override stagger_rhythm
    pub jitter: f32,          // 0.0-1.0, probability of replacing glyph with random line char
    pub skew: u32,            // 0-100, how much pattern bleeds past rect boundary
}

impl TileParams {
    pub fn new(variant: TileVariant) -> Self {
        TileParams {
            variant,
            density: 1.0,
            stagger_override: -1,
            rhythm_override: 0,
            jitter: 0.0,
            skew: 0,
        }
    }

    pub fn randomized(rng: &mut StdRng) -> Self {
        let variant = tile_variant_from_index(rng.random_range(0..TILE_VARIANT_COUNT));
        let density = if rng.random_range(0..4) == 0 {
            rng.random_range(60..95) as f32 / 100.0
        } else {
            1.0
        };
        // 40% chance to vary stagger from default
        let stagger_override = if rng.random_range(0..5) < 2 {
            rng.random_range(0..6) as i8 // 0 = no stagger, 1-5 = various offsets
        } else {
            -1 // default
        };
        // 30% chance to vary rhythm from default
        let rhythm_override = if rng.random_range(0..10) < 3 {
            rng.random_range(1..5) // group 1-4 tile-rows before shifting
        } else {
            0 // default
        };
        // 30% chance of skew (pattern bleeds past rect edges)
        let skew = if rng.random_range(0..10u32) < 3 {
            rng.random_range(15..60)
        } else {
            0
        };
        TileParams {
            variant,
            density,
            stagger_override,
            rhythm_override,
            jitter: 0.0,
            skew,
        }
    }
}

/// Fill a rect with a tile pattern, pure deterministic baseline.
/// No phase shift, no jitter, no density dropout. What-you-define-is-what-you-get.
pub fn fill_tile_pure(
    grid: &mut Grid,
    rect: &Rect,
    variant: TileVariant,
    color: Color,
    color2: Color,
) {
    let tile = make_tile(variant);
    for y in rect.y..rect.y + rect.h {
        for x in rect.x..rect.x + rect.w {
            if y >= grid.len() || x >= grid[0].len() {
                continue;
            }
            let (ch, ci) = tile.at(x - rect.x, y - rect.y);
            if ch == ' ' {
                continue;
            }
            let bg = grid[y][x].bg;
            let fg = if ci == 0 { color } else { color2 };
            grid[y][x] = Cell::with_bg(ch, fg, bg);
        }
    }
}

// ── Tile edge behavior ───────────────────────────────────────────────

/// Per-cell context passed to edge_behavior so each variant can make
/// bespoke decisions about what happens at/beyond rect boundaries.
pub struct TileEdgeContext {
    pub tx: usize,         // tile-local x (0..px, phase-shifted)
    pub ty: usize,         // tile-local y (0..py, phase-shifted)
    pub px: usize,         // tile period x
    pub py: usize,         // tile period y
    pub normal_char: char, // what normal tile sampling returns here
    pub normal_ci: u8,     // color index from normal sampling
    pub dist_left: i32,    // positive = outside left edge by N cells
    pub dist_right: i32,   // positive = outside right edge
    pub dist_top: i32,     // positive = outside top edge
    pub dist_bottom: i32,  // positive = outside bottom edge
    pub outside: bool,     // true when cell is outside the rect
    pub extend: usize,     // max bleed distance in cells
    /// Shape-aware: 0.0 = at container boundary, 1.0 = deep inside.
    /// When Some, edge functions use this instead of rect-distance dropout.
    pub boundary_value: Option<f32>,
}

/// Per-variant edge strategy. Returns Some((char, color_index)) to draw,
/// None to skip the cell entirely.
///
/// Variants that don't need bespoke behavior fall through to default_edge,
/// which is the generic distance-based dropout.
fn edge_behavior(
    variant: TileVariant,
    ctx: &TileEdgeContext,
    rng: &mut StdRng,
) -> Option<(char, u8)> {
    match variant {
        // Arch-based patterns: mirror ty near bottom to close arches
        TileVariant::Seigaiha | TileVariant::ShellStitch => mirror_edge(ctx, true, false, rng),
        // Circles: mirror both axes to close
        TileVariant::Shippo => mirror_edge(ctx, true, true, rng),
        // Hexagons: mirror ty to close hex tops/bottoms
        TileVariant::BishamonKikko => mirror_edge(ctx, true, false, rng),
        // Scales: mirror ty near bottom, tx near right
        TileVariant::CrocodileScale => mirror_edge(ctx, true, true, rng),
        // Boxes: close with └─┘ at bottom edge
        TileVariant::GrannySquare => granny_edge(ctx, rng),
        // Strokes: extend individual lines with decay
        TileVariant::Nowaki => nowaki_edge(ctx, rng),
        // Self-similar: pure dropout
        TileVariant::Asanoha | TileVariant::Yabane | TileVariant::Higaki => default_edge(ctx, rng),
    }
}

/// Default: distance-based probabilistic dropout.
/// When boundary_value is set, uses it directly as survival probability
/// instead of rect-edge distance, so dropout follows the container's contour.
fn default_edge(ctx: &TileEdgeContext, rng: &mut StdRng) -> Option<(char, u8)> {
    if !ctx.outside {
        return Some((ctx.normal_char, ctx.normal_ci));
    }
    if ctx.extend == 0 {
        return None;
    }
    let survive = if let Some(bv) = ctx.boundary_value {
        bv.max(0.0)
    } else {
        let dist = ctx
            .dist_left
            .max(ctx.dist_right)
            .max(ctx.dist_top)
            .max(ctx.dist_bottom)
            .max(0) as f32;
        1.0 - (dist / ctx.extend as f32).powf(0.7)
    };
    if survive <= 0.0 || rng.random::<f32>() > survive {
        return None;
    }
    Some((ctx.normal_char, ctx.normal_ci))
}

/// Mirror tile coords near edges, then apply dropout for cells past the boundary.
/// `mirror_y`: fold ty near bottom/top edges.
/// `mirror_x`: fold tx near left/right edges.
fn mirror_edge(
    ctx: &TileEdgeContext,
    mirror_y: bool,
    mirror_x: bool,
    rng: &mut StdRng,
) -> Option<(char, u8)> {
    // Outside the rect: dropout with distance
    if ctx.outside {
        if ctx.extend == 0 {
            return None;
        }
        let dist = ctx
            .dist_left
            .max(ctx.dist_right)
            .max(ctx.dist_top)
            .max(ctx.dist_bottom)
            .max(0) as f32;
        let survive = 1.0 - (dist / ctx.extend as f32).powf(0.7);
        if survive <= 0.0 || rng.random::<f32>() > survive {
            return None;
        }
    }

    // Near-edge mirroring: reflect tile coords within one period of the edge
    // so open shapes (arches, circles, waves) fold back into closed forms.
    // dist_bottom < 0 means "inside, N cells from bottom edge"
    let near_bottom = ctx.dist_bottom <= 0 && (-ctx.dist_bottom as usize) < ctx.py;
    let near_top = ctx.dist_top <= 0 && (-ctx.dist_top as usize) < ctx.py;
    let near_right = ctx.dist_right <= 0 && (-ctx.dist_right as usize) < ctx.px;
    let near_left = ctx.dist_left <= 0 && (-ctx.dist_left as usize) < ctx.px;

    // Only mirror if we're far enough from the opposite edge to avoid double-mirror
    let far_from_top = ctx.dist_top <= 0 && (-ctx.dist_top as usize) >= ctx.py;
    let far_from_left = ctx.dist_left <= 0 && (-ctx.dist_left as usize) >= ctx.px;

    let mut ty = ctx.ty;
    let mut tx = ctx.tx;

    if mirror_y && near_bottom && far_from_top {
        ty = ctx.py - 1 - ty;
    }
    if mirror_x && near_right && far_from_left {
        tx = ctx.px - 1 - tx;
    }

    // If we mirrored, the char might differ from normal_char
    if ty != ctx.ty || tx != ctx.tx {
        // We can't re-sample the tile from here (no TilePattern ref), so
        // the caller handles this by passing already-mirrored coords.
        // For now, signal "use mirrored coords" by returning the mirrored ty/tx
        // encoded... Actually, let's just return normal_char since the caller
        // will handle mirroring before calling us.
        //
        // The real mirroring happens in fill_tile_ex before tile.at().
        // What this function controls is: should this cell draw or not?
        return Some((ctx.normal_char, ctx.normal_ci));
    }

    Some((ctx.normal_char, ctx.normal_ci))
}

/// GrannySquare: at bottom edge, emit box-closing characters.
fn granny_edge(ctx: &TileEdgeContext, rng: &mut StdRng) -> Option<(char, u8)> {
    if ctx.outside {
        return default_edge(ctx, rng);
    }

    // Near bottom edge (within one tile period), in the lower half of the tile:
    // replace with closing box chars
    let near_bottom = ctx.dist_bottom <= 0 && (-ctx.dist_bottom as usize) < ctx.py;
    let far_from_top = ctx.dist_top <= 0 && (-ctx.dist_top as usize) >= ctx.py;

    if near_bottom && far_from_top {
        // GrannySquare period is 6x6. Bottom row of tile is └─┴┴─┘
        // Mirror ty so the box closes
        let mirrored_ty = ctx.py - 1 - ctx.ty;
        // When mirrored ty maps to top row (row 0), emit closing chars
        if mirrored_ty == 0 {
            let ch = match ctx.tx {
                0 => '└',
                5 => '┘',
                1 | 4 => '─',
                2 | 3 => '┴',
                _ => ctx.normal_char,
            };
            return Some((ch, ctx.normal_ci));
        }
    }

    Some((ctx.normal_char, ctx.normal_ci))
}

/// Nowaki: extend stroke characters (│ and ╱) past the boundary with
/// length-based decay instead of random dropout. Non-stroke chars get
/// normal dropout.
fn nowaki_edge(ctx: &TileEdgeContext, rng: &mut StdRng) -> Option<(char, u8)> {
    if !ctx.outside {
        return Some((ctx.normal_char, ctx.normal_ci));
    }
    if ctx.extend == 0 {
        return None;
    }

    let is_stroke = matches!(ctx.normal_char, '│' | '╱');
    let survive = if let Some(bv) = ctx.boundary_value {
        // Strokes survive a bit further past the shape boundary
        let boost = if is_stroke { 0.3 } else { 0.0 };
        (bv + boost).min(1.0).max(0.0)
    } else {
        let dist = ctx
            .dist_left
            .max(ctx.dist_right)
            .max(ctx.dist_top)
            .max(ctx.dist_bottom)
            .max(0) as f32;
        let power = if is_stroke { 1.5 } else { 0.7 };
        1.0 - (dist / ctx.extend as f32).powf(power)
    };
    if survive <= 0.0 || rng.random::<f32>() > survive {
        return None;
    }
    Some((ctx.normal_char, ctx.normal_ci))
}

/// Fill a rect with a tile pattern, full control.
///
/// `skew_boundary`: optional shape boundary fn (same type as a mask fn: 0.0 = at
/// shape edge, 1.0 = deep inside). When provided, dropout at/past the rect edges
/// follows the container's contour instead of uniform rect-distance falloff.
/// Pass `None` for default rect-edge skew behavior.
pub fn fill_tile_ex(
    grid: &mut Grid,
    rect: &Rect,
    params: &TileParams,
    color: Color,
    color2: Color,
    jitter: f32,
    skew_boundary: Option<&dyn Fn(usize, usize) -> f32>,
    rng: &mut StdRng,
) {
    let mut tile = make_tile(params.variant);
    // apply stagger/rhythm overrides
    if params.stagger_override >= 0 {
        tile.row_offset = params.stagger_override as usize;
    }
    if params.rhythm_override > 0 {
        tile.stagger_rhythm = params.rhythm_override as usize;
    }
    // random phase shift so pattern origin varies per-instance
    let phase_x = rng.random_range(0..tile.period_x());
    let phase_y = rng.random_range(0..tile.period_y());
    let jitter_glyphs = ['╱', '╲', '╳', '·', '─', '│'];

    // skew: extend render bounds past the rect, dropout increases with distance
    let extend = (params.skew as f32 / 100.0 * 12.0) as usize;
    let grid_h = grid.len();
    let grid_w = if grid_h > 0 { grid[0].len() } else { 0 };
    let y0 = rect.y.saturating_sub(extend);
    let y1 = (rect.y + rect.h + extend).min(grid_h);
    let x0 = rect.x.saturating_sub(extend);
    let x1 = (rect.x + rect.w + extend).min(grid_w);

    let px = tile.period_x();
    let py = tile.period_y();

    for y in y0..y1 {
        for x in x0..x1 {
            // signed distances from each edge: positive = outside, negative = inside
            let dist_left = if x < rect.x {
                (rect.x - x) as i32
            } else {
                -((x - rect.x) as i32)
            };
            let dist_right = if x >= rect.x + rect.w {
                (x - (rect.x + rect.w) + 1) as i32
            } else {
                -(((rect.x + rect.w - 1) - x) as i32)
            };
            let dist_top = if y < rect.y {
                (rect.y - y) as i32
            } else {
                -((y - rect.y) as i32)
            };
            let dist_bottom = if y >= rect.y + rect.h {
                (y - (rect.y + rect.h) + 1) as i32
            } else {
                -(((rect.y + rect.h - 1) - y) as i32)
            };
            let outside = dist_left > 0 || dist_right > 0 || dist_top > 0 || dist_bottom > 0;

            if params.density < 1.0 && rng.random::<f32>() > params.density {
                continue;
            }

            // sample tile using rect-relative coords (phase-shifted)
            let tx = (x as i32 - rect.x as i32 + phase_x as i32).rem_euclid(px as i32) as usize;
            let ty = (y as i32 - rect.y as i32 + phase_y as i32).rem_euclid(py as i32) as usize;

            // Mirror tile coords near edges for variants that close shapes.
            // This happens before sampling so the tile.at() call gets mirrored coords.
            let mut mtx = tx;
            let mut mty = ty;
            if params.skew > 0 {
                let near_bottom = dist_bottom <= 0 && (-dist_bottom as usize) < py;
                let far_from_top = dist_top <= 0 && (-dist_top as usize) >= py;
                let near_right = dist_right <= 0 && (-dist_right as usize) < px;
                let far_from_left = dist_left <= 0 && (-dist_left as usize) >= px;

                let do_mirror_y = matches!(
                    params.variant,
                    TileVariant::Seigaiha
                        | TileVariant::ShellStitch
                        | TileVariant::Shippo
                        | TileVariant::BishamonKikko
                        | TileVariant::CrocodileScale
                );
                let do_mirror_x = matches!(
                    params.variant,
                    TileVariant::Shippo | TileVariant::CrocodileScale
                );

                if do_mirror_y && near_bottom && far_from_top {
                    mty = py - 1 - ty;
                }
                if do_mirror_x && near_right && far_from_left {
                    mtx = px - 1 - tx;
                }
            }

            let (normal_char, normal_ci) = tile.at(mtx, mty);

            // boundary_value: shape-aware survival weight, or None for rect-edge falloff.
            let boundary_value = skew_boundary.map(|f| f(x, y));

            // edge behavior decides draw vs skip for cells near/past boundaries
            let near_edge = params.skew > 0
                && ((dist_bottom <= 0 && (-dist_bottom as usize) < py)
                    || (dist_right <= 0 && (-dist_right as usize) < px));

            if outside || near_edge {
                let ctx = TileEdgeContext {
                    tx,
                    ty,
                    px,
                    py,
                    normal_char,
                    normal_ci,
                    dist_left,
                    dist_right,
                    dist_top,
                    dist_bottom,
                    outside,
                    extend,
                    boundary_value,
                };
                match edge_behavior(params.variant, &ctx, rng) {
                    Some((ch, ci)) => {
                        if ch == ' ' {
                            continue;
                        }
                        let mut ch = ch;
                        if jitter > 0.0 && rng.random::<f32>() < jitter {
                            ch = jitter_glyphs[rng.random_range(0..jitter_glyphs.len())];
                        }
                        let bg = grid[y][x].bg;
                        let mut fg = if ci == 0 { color } else { color2 };
                        if jitter > 0.0 {
                            let drift = rng.random_range(0..=20) as u8;
                            if drift > 10 {
                                fg = lighten(fg, drift - 10);
                            } else {
                                fg = darken(fg, 10 - drift);
                            }
                        }
                        grid[y][x] = Cell::with_bg(ch, fg, bg);
                    }
                    None => continue,
                }
            } else {
                // Fast path: inside rect, far from edges
                if normal_char == ' ' {
                    continue;
                }
                let mut ch = normal_char;
                if jitter > 0.0 && rng.random::<f32>() < jitter {
                    ch = jitter_glyphs[rng.random_range(0..jitter_glyphs.len())];
                }
                let bg = grid[y][x].bg;
                let mut fg = if normal_ci == 0 { color } else { color2 };
                if jitter > 0.0 {
                    let drift = rng.random_range(0..=20) as u8;
                    if drift > 10 {
                        fg = lighten(fg, drift - 10);
                    } else {
                        fg = darken(fg, 10 - drift);
                    }
                }
                grid[y][x] = Cell::with_bg(ch, fg, bg);
            }
        }
    }
}

// ── Noise fills (per-cell random, no periodic structure) ────────────

/// Weighted glyph entry: (char, color_index 0 or 1, cumulative weight).
/// Weights don't need to sum to 1.0 -- they're normalized at fill time.
pub struct NoiseGlyph {
    pub ch: char,
    pub ci: u8,      // 0 = primary color, 1 = secondary
    pub weight: f32, // relative probability
}

/// Predefined noise palettes.
#[derive(Clone, Copy)]
pub enum NoiseVariant {
    Truchet,      // classic ╱╲ 50/50, coherence 0.0
    Higaki,       // ╱╲╳ with gaps, coherence 0.7 (long runs, rare breaks)
    HigakiStatic, // ╱╲╳ with gaps, coherence 0.0 (per-cell random, the original)
    Grass,        // ╱╲│ with spaces, coherence 0.5
    Static,       // ╱╲─│╳·░, coherence 0.0 (pure random)
    Dot,          // ·∙°, coherence 0.6
}

pub fn noise_coherence(variant: NoiseVariant) -> f32 {
    match variant {
        NoiseVariant::Truchet => 0.0,
        NoiseVariant::Higaki => 0.7,
        NoiseVariant::HigakiStatic => 0.0,
        NoiseVariant::Grass => 0.5,
        NoiseVariant::Static => 0.0,
        NoiseVariant::Dot => 0.6,
    }
}

pub fn noise_glyphs(variant: NoiseVariant) -> Vec<NoiseGlyph> {
    match variant {
        NoiseVariant::Truchet => vec![
            NoiseGlyph {
                ch: '╱',
                ci: 0,
                weight: 1.0,
            },
            NoiseGlyph {
                ch: '╲',
                ci: 0,
                weight: 1.0,
            },
        ],
        NoiseVariant::Higaki | NoiseVariant::HigakiStatic => vec![
            NoiseGlyph {
                ch: '╱',
                ci: 0,
                weight: 3.0,
            },
            NoiseGlyph {
                ch: '╲',
                ci: 0,
                weight: 3.0,
            },
            NoiseGlyph {
                ch: '╳',
                ci: 1,
                weight: 2.0,
            },
            NoiseGlyph {
                ch: ' ',
                ci: 0,
                weight: 1.0,
            },
        ],
        NoiseVariant::Grass => vec![
            NoiseGlyph {
                ch: '╱',
                ci: 0,
                weight: 2.0,
            },
            NoiseGlyph {
                ch: '╲',
                ci: 0,
                weight: 2.0,
            },
            NoiseGlyph {
                ch: '│',
                ci: 1,
                weight: 1.5,
            },
            NoiseGlyph {
                ch: ' ',
                ci: 0,
                weight: 3.0,
            },
        ],
        NoiseVariant::Static => vec![
            NoiseGlyph {
                ch: '╱',
                ci: 0,
                weight: 2.0,
            },
            NoiseGlyph {
                ch: '╲',
                ci: 0,
                weight: 2.0,
            },
            NoiseGlyph {
                ch: '─',
                ci: 1,
                weight: 1.0,
            },
            NoiseGlyph {
                ch: '│',
                ci: 1,
                weight: 1.0,
            },
            NoiseGlyph {
                ch: '╳',
                ci: 1,
                weight: 0.5,
            },
            NoiseGlyph {
                ch: '·',
                ci: 0,
                weight: 1.0,
            },
            NoiseGlyph {
                ch: '░',
                ci: 0,
                weight: 0.5,
            },
        ],
        NoiseVariant::Dot => vec![
            NoiseGlyph {
                ch: '·',
                ci: 0,
                weight: 3.0,
            },
            NoiseGlyph {
                ch: '∙',
                ci: 1,
                weight: 1.0,
            },
            NoiseGlyph {
                ch: '°',
                ci: 1,
                weight: 0.5,
            },
            NoiseGlyph {
                ch: ' ',
                ci: 0,
                weight: 5.0,
            },
        ],
    }
}

pub const NOISE_VARIANT_COUNT: usize = 6;

pub fn noise_variant_from_index(i: usize) -> NoiseVariant {
    match i % NOISE_VARIANT_COUNT {
        0 => NoiseVariant::Truchet,
        1 => NoiseVariant::Higaki,
        2 => NoiseVariant::HigakiStatic,
        3 => NoiseVariant::Grass,
        4 => NoiseVariant::Static,
        _ => NoiseVariant::Dot,
    }
}

/// Sample a glyph index from the CDF.
fn sample_glyph(cdf: &[f32], rng: &mut StdRng) -> usize {
    let r = rng.random::<f32>();
    cdf.iter().position(|&c| r < c).unwrap_or(cdf.len() - 1)
}

/// Fill a rect with noise. Coherence controls run length.
pub fn fill_noise(
    grid: &mut Grid,
    rect: &Rect,
    variant: NoiseVariant,
    color: Color,
    color2: Color,
    rng: &mut StdRng,
) {
    let glyphs = noise_glyphs(variant);
    let coherence = noise_coherence(variant);
    let total: f32 = glyphs.iter().map(|g| g.weight).sum();
    let mut cdf: Vec<f32> = Vec::with_capacity(glyphs.len());
    let mut acc = 0.0;
    for g in &glyphs {
        acc += g.weight / total;
        cdf.push(acc);
    }
    // current glyph index (the "momentum" state)
    let mut cur = sample_glyph(&cdf, rng);
    for y in rect.y..rect.y + rect.h {
        for x in rect.x..rect.x + rect.w {
            if y >= grid.len() || x >= grid[0].len() {
                continue;
            }
            // coherence check: continue run or break?
            if coherence <= 0.0 || rng.random::<f32>() >= coherence {
                cur = sample_glyph(&cdf, rng);
            }
            let g = &glyphs[cur];
            if g.ch == ' ' {
                continue;
            }
            let bg = grid[y][x].bg;
            let fg = if g.ci == 0 { color } else { color2 };
            grid[y][x] = Cell::with_bg(g.ch, fg, bg);
        }
    }
}

/// Fill entire grid with Truchet noise. Replaces the duplicated inline loops.
pub fn fill_truchet(grid: &mut Grid, width: usize, height: usize, color: Color, rng: &mut StdRng) {
    let rect = Rect {
        x: 0,
        y: 0,
        w: width,
        h: height,
    };
    fill_noise(grid, &rect, NoiseVariant::Truchet, color, color, rng);
}

// ── Line art fills ──────────────────────────────────────────────────

/// Crosshatch: deterministic diagonal tiling. Denser than random Truchet.
pub fn draw_crosshatch(grid: &mut Grid, rect: &Rect, color: Color, color2: Color) {
    for y in rect.y..rect.y + rect.h {
        for x in rect.x..rect.x + rect.w {
            if y >= grid.len() || x >= grid[0].len() {
                continue;
            }
            let bg = grid[y][x].bg;
            let (ch, c) = if (x + y) % 2 == 0 {
                ('╱', color)
            } else {
                ('╲', color2)
            };
            grid[y][x] = Cell::with_bg(ch, c, bg);
        }
    }
}

/// Guilloche: interlocking wave curves from rounded box-drawing chars.
pub fn draw_guilloche(grid: &mut Grid, rect: &Rect, color: Color, color2: Color) {
    for y in rect.y..rect.y + rect.h {
        for x in rect.x..rect.x + rect.w {
            if y >= grid.len() || x >= grid[0].len() {
                continue;
            }
            let bg = grid[y][x].bg;
            let ry = (y - rect.y) % 4;
            let rx = (x - rect.x) % 6;
            let (ch, c) = match (ry, rx) {
                (0, 0) => ('╭', color),
                (0, 1) | (0, 2) => ('─', color),
                (0, 3) => ('╮', color),
                (1, 0) => ('│', color),
                (1, 3) => ('│', color),
                (2, 0) => ('╰', color),
                (2, 1) | (2, 2) => ('─', color),
                (2, 3) => ('╯', color),
                (2, 4) => ('╭', color2),
                (2, 5) => ('─', color2),
                (3, 4) => ('│', color2),
                (3, 5) => ('│', color2),
                (0, 4) => ('╰', color2),
                (0, 5) => ('─', color2),
                (1, 4) => ('╭', color2),
                (1, 5) => ('─', color2),
                (3, 0) => ('─', color2),
                (3, 3) => ('─', color2),
                (1, 1) => ('╰', color2),
                (1, 2) => ('╯', color2),
                (3, 1) => ('╭', color2),
                (3, 2) => ('╮', color2),
                _ => continue,
            };
            grid[y][x] = Cell::with_bg(ch, c, bg);
        }
    }
}

/// Basket weave: interlocking horizontal/vertical line segments.
pub fn draw_weave(grid: &mut Grid, rect: &Rect, color: Color, color2: Color) {
    for y in rect.y..rect.y + rect.h {
        for x in rect.x..rect.x + rect.w {
            if y >= grid.len() || x >= grid[0].len() {
                continue;
            }
            let bg = grid[y][x].bg;
            let ry = (y - rect.y) % 6;
            let rx = (x - rect.x) % 6;
            let (ch, c) = match (ry, rx) {
                // horizontal strand A (rows 0-1)
                (0, 0..=1) => ('─', color),
                (0, 2) => ('┐', color),
                (1, 2) => ('┘', color),
                (1, 0..=1) => ('─', color),
                // vertical strand (cols 3-4)
                (0, 3) => ('┌', color2),
                (1, 3) => ('│', color2),
                (2, 3) => ('│', color2),
                (0, 4) => ('┐', color2),
                (1, 4) => ('│', color2),
                (2, 4) => ('│', color2),
                // horizontal strand B (rows 3-4)
                (3, 3..=4) => ('─', color),
                (3, 5) => ('┐', color),
                (4, 5) => ('┘', color),
                (4, 3..=4) => ('─', color),
                // vertical strand (cols 0-1)
                (3, 0) => ('┌', color2),
                (4, 0) => ('│', color2),
                (5, 0) => ('│', color2),
                (3, 1) => ('┐', color2),
                (4, 1) => ('│', color2),
                (5, 1) => ('│', color2),
                // connecting
                (2, 2) => ('┌', color),
                (2, 0..=1) => ('─', color),
                (5, 5) => ('┌', color),
                (5, 3..=4) => ('─', color),
                _ => continue,
            };
            grid[y][x] = Cell::with_bg(ch, c, bg);
        }
    }
}

/// Zigzag: horizontal zigzag bands using diagonal chars.
pub fn draw_zigzag(grid: &mut Grid, rect: &Rect, color: Color, color2: Color) {
    for y in rect.y..rect.y + rect.h {
        for x in rect.x..rect.x + rect.w {
            if y >= grid.len() || x >= grid[0].len() {
                continue;
            }
            let bg = grid[y][x].bg;
            let ry = (y - rect.y) % 4;
            let rx = (x - rect.x) % 4;
            let (ch, c) = match (ry, rx) {
                (0, 0) | (0, 1) => ('╱', color),
                (0, 2) | (0, 3) => ('╲', color),
                (1, 0) | (1, 1) => ('╲', color2),
                (1, 2) | (1, 3) => ('╱', color2),
                (2, 0) | (2, 1) => ('╱', color),
                (2, 2) | (2, 3) => ('╲', color),
                (3, 0) | (3, 1) => ('╲', color2),
                (3, 2) | (3, 3) => ('╱', color2),
                _ => continue,
            };
            grid[y][x] = Cell::with_bg(ch, c, bg);
        }
    }
}

/// Diamond lattice: interlocking diamond shapes from box-drawing diagonals.
pub fn draw_diamond_lattice(grid: &mut Grid, rect: &Rect, color: Color, color2: Color) {
    for y in rect.y..rect.y + rect.h {
        for x in rect.x..rect.x + rect.w {
            if y >= grid.len() || x >= grid[0].len() {
                continue;
            }
            let bg = grid[y][x].bg;
            let ry = (y - rect.y) % 4;
            let rx = (x - rect.x) % 4;
            let (ch, c) = match (ry, rx) {
                (0, 1) => ('╱', color),
                (0, 2) => ('╲', color),
                (1, 0) => ('╲', color2),
                (1, 3) => ('╱', color2),
                (2, 1) => ('╲', color),
                (2, 2) => ('╱', color),
                (3, 0) => ('╱', color2),
                (3, 3) => ('╲', color2),
                (1, 1) | (1, 2) | (3, 1) | (3, 2) => ('·', darken(color, 40)),
                _ => continue,
            };
            grid[y][x] = Cell::with_bg(ch, c, bg);
        }
    }
}

/// Spiral: Archimedean spiral from center using box-drawing curves.
pub fn draw_spiral(grid: &mut Grid, rect: &Rect, color: Color, color2: Color) {
    let cx = rect.x as f32 + rect.w as f32 * 0.5;
    let cy = rect.y as f32 + rect.h as f32 * 0.5;
    let max_r = (rect.w.min(rect.h * 2)) as f32 * 0.45;
    let turns = 4.0;
    let steps = (max_r * turns * 8.0) as usize;

    let mut prev_x = cx as usize;
    let mut prev_y = cy as usize;

    for i in 0..steps {
        let t = i as f32 / steps as f32;
        let angle = t * turns * std::f32::consts::TAU;
        let r = t * max_r;
        let px = (cx + angle.cos() * r * 2.0) as usize; // *2 for terminal aspect ratio
        let py = (cy + angle.sin() * r) as usize;

        if py >= grid.len() || px >= grid[0].len() {
            continue;
        }
        if py < rect.y || py >= rect.y + rect.h || px < rect.x || px >= rect.x + rect.w {
            continue;
        }

        let dx = px as i32 - prev_x as i32;
        let dy = py as i32 - prev_y as i32;
        let ch = if dx == 0 && dy == 0 {
            '·'
        } else if dx.abs() > dy.abs() {
            '─'
        } else if dy > 0 {
            if dx > 0 { '╲' } else { '╱' }
        } else {
            if dx > 0 { '╱' } else { '╲' }
        };
        let c = if i % 2 == 0 { color } else { color2 };
        let bg = grid[py][px].bg;
        grid[py][px] = Cell::with_bg(ch, c, bg);
        prev_x = px;
        prev_y = py;
    }
}

/// Concentric: nested rectangles from center outward, alternating colors.
pub fn draw_concentric(grid: &mut Grid, rect: &Rect, color: Color, color2: Color) {
    let cx = rect.x + rect.w / 2;
    let cy = rect.y + rect.h / 2;
    let max_rings = (rect.w.min(rect.h * 2) / 4).max(1);

    for ring in 0..max_rings {
        let hw = (ring + 1) * 2;
        let hh = ring + 1;
        let c = if ring % 2 == 0 { color } else { color2 };
        let dim_c = darken(c, 20);

        for dx in 0..=hw * 2 {
            let x = cx + dx - hw;
            let y_top = cy.saturating_sub(hh);
            let y_bot = cy + hh;
            if x < rect.x || x >= rect.x + rect.w {
                continue;
            }

            if y_top >= rect.y && y_top < rect.y + rect.h && y_top < grid.len() && x < grid[0].len()
            {
                let bg = grid[y_top][x].bg;
                let ch = if dx == 0 {
                    '╭'
                } else if dx == hw * 2 {
                    '╮'
                } else {
                    '─'
                };
                grid[y_top][x] = Cell::with_bg(ch, c, bg);
            }
            if y_bot >= rect.y && y_bot < rect.y + rect.h && y_bot < grid.len() && x < grid[0].len()
            {
                let bg = grid[y_bot][x].bg;
                let ch = if dx == 0 {
                    '╰'
                } else if dx == hw * 2 {
                    '╯'
                } else {
                    '─'
                };
                grid[y_bot][x] = Cell::with_bg(ch, dim_c, bg);
            }
        }
        for dy in 1..hh {
            let y = cy.saturating_sub(hh) + dy;
            let x_left = cx.saturating_sub(hw);
            let x_right = cx + hw;
            if y < rect.y || y >= rect.y + rect.h || y >= grid.len() {
                continue;
            }
            if x_left >= rect.x && x_left < rect.x + rect.w && x_left < grid[0].len() {
                let bg = grid[y][x_left].bg;
                grid[y][x_left] = Cell::with_bg('│', c, bg);
            }
            if x_right >= rect.x && x_right < rect.x + rect.w && x_right < grid[0].len() {
                let bg = grid[y][x_right].bg;
                grid[y][x_right] = Cell::with_bg('│', dim_c, bg);
            }
        }
    }
}

/// Labyrinth: deterministic maze-like pattern using line segments.
pub fn draw_labyrinth(grid: &mut Grid, rect: &Rect, color: Color, color2: Color) {
    for y in rect.y..rect.y + rect.h {
        for x in rect.x..rect.x + rect.w {
            if y >= grid.len() || x >= grid[0].len() {
                continue;
            }
            let bg = grid[y][x].bg;
            let rx = x - rect.x;
            let ry = y - rect.y;

            let cell_w = 4;
            let cell_h = 2;
            let cell_row = ry / cell_h;
            let offset = if cell_row % 2 == 1 { cell_w / 2 } else { 0 };
            let adj_cx = (rx + offset) % cell_w;
            let cy = ry % cell_h;
            let cell_col = rx / cell_w;

            let (ch, c) = match (cy, adj_cx) {
                (0, 0) => ('┌', color),
                (0, 1) | (0, 2) => ('─', color),
                (0, 3) => ('┐', color2),
                (1, 0) => {
                    if cell_row % 3 == 0 {
                        ('├', color)
                    } else {
                        ('│', color)
                    }
                }
                (1, 3) => {
                    if (cell_col + cell_row) % 2 == 0 {
                        ('┤', color2)
                    } else {
                        ('│', color2)
                    }
                }
                (1, 1) => {
                    if cell_row % 2 == 0 {
                        ('·', darken(color, 40))
                    } else {
                        continue;
                    }
                }
                _ => continue,
            };
            grid[y][x] = Cell::with_bg(ch, c, bg);
        }
    }
}
