use crossterm::style::Color;
use rand::rngs::StdRng;
use rand::RngExt;
use crate::types::*;
use crate::color::*;

// ── Tile pattern system ─────────────────────────────────────────────

/// A tile is a small rectangular char grid that repeats to fill any area.
///
/// Stagger controls:
/// - `row_offset`: how many columns each tile-row shifts rightward
/// - `stagger_rhythm`: how many tile-rows before the offset resets
///   (1 = every row staggers, 2 = every other row, etc.)
pub struct TilePattern {
    pub cells: Vec<Vec<(char, u8)>>,  // [y][x] -> (char, color_index: 0=primary, 1=secondary)
    pub row_offset: usize,            // x-shift per stagger group (0 = no stagger)
    pub stagger_rhythm: usize,        // tile-rows per stagger group (1 = every row, 2 = pairs, etc.)
}

impl TilePattern {
    pub fn period_x(&self) -> usize { self.cells[0].len() }
    pub fn period_y(&self) -> usize { self.cells.len() }

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
                vec![('╲', 0), ('╱', 1), ('╲', 0), ('╱', 1), ('╲', 1), ('╱', 0), ('╲', 1), ('╱', 0)],
                vec![('╱', 0), ('╲', 1), ('─', 0), ('─', 0), ('─', 1), ('─', 1), ('╱', 1), ('╲', 0)],
                vec![('╲', 1), ('╱', 0), ('╲', 1), ('╱', 0), ('╲', 0), ('╱', 1), ('╲', 0), ('╱', 1)],
                vec![('╱', 1), ('╲', 0), ('─', 1), ('─', 1), ('─', 0), ('─', 0), ('╱', 0), ('╲', 1)],
            ];
            TilePattern { cells: g, row_offset: 4, stagger_rhythm: 1 }
        }
        TileVariant::Seigaiha => {
            let g = vec![
                vec![('╰', 0), ('─', 0), ('─', 0), ('╯', 0), ('╰', 1), ('─', 1), ('─', 1), ('╯', 1)],
                vec![(' ', 0), ('╭', 0), ('╮', 0), (' ', 0), (' ', 1), ('╭', 1), ('╮', 1), (' ', 1)],
                vec![(' ', 0), ('│', 0), ('│', 0), (' ', 0), (' ', 1), ('│', 1), ('│', 1), (' ', 1)],
                vec![('╭', 0), ('╯', 0), ('╰', 0), ('╮', 0), ('╭', 1), ('╯', 1), ('╰', 1), ('╮', 1)],
            ];
            TilePattern { cells: g, row_offset: 4, stagger_rhythm: 1 }
        }
        TileVariant::Shippo => {
            let g = vec![
                vec![('╲', 0), ('╭', 1), ('─', 1), ('╮', 1), ('╱', 0), (' ', 0)],
                vec![(' ', 0), ('│', 1), (' ', 0), ('│', 1), (' ', 0), (' ', 0)],
                vec![('╱', 0), ('╰', 1), ('─', 1), ('╯', 1), ('╲', 0), (' ', 0)],
                vec![(' ', 0), (' ', 0), ('╲', 0), ('╱', 0), (' ', 0), (' ', 0)],
            ];
            TilePattern { cells: g, row_offset: 3, stagger_rhythm: 1 }
        }
        TileVariant::BishamonKikko => {
            let g = vec![
                vec![('╱', 0), ('─', 0), ('─', 0), ('╲', 0), (' ', 1), (' ', 1)],
                vec![('│', 0), (' ', 0), (' ', 0), ('│', 0), (' ', 1), (' ', 1)],
                vec![('╲', 0), ('─', 0), ('─', 0), ('╱', 0), (' ', 1), (' ', 1)],
                vec![(' ', 0), (' ', 0), (' ', 0), (' ', 0), (' ', 1), (' ', 1)],
            ];
            TilePattern { cells: g, row_offset: 3, stagger_rhythm: 1 }
        }
        TileVariant::Yabane => {
            let g = vec![
                vec![('╱', 0), ('╱', 0), ('╱', 0), ('╲', 1), ('╲', 1), ('╲', 1)],
                vec![('╱', 0), ('╱', 0), ('╱', 0), ('╲', 1), ('╲', 1), ('╲', 1)],
                vec![('╲', 1), ('╲', 1), ('╲', 1), ('╱', 0), ('╱', 0), ('╱', 0)],
                vec![('╲', 1), ('╲', 1), ('╲', 1), ('╱', 0), ('╱', 0), ('╱', 0)],
            ];
            TilePattern { cells: g, row_offset: 0, stagger_rhythm: 1 }
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
            TilePattern { cells: g, row_offset: 0, stagger_rhythm: 1 }
        }
        TileVariant::Higaki => {
            let g = vec![
                vec![('╱', 0), ('╳', 1), ('╲', 0), (' ', 0)],
                vec![('╳', 1), ('╲', 0), (' ', 0), ('╱', 0)],
                vec![('╲', 0), (' ', 0), ('╱', 0), ('╳', 1)],
                vec![(' ', 0), ('╱', 0), ('╳', 1), ('╲', 0)],
            ];
            TilePattern { cells: g, row_offset: 0, stagger_rhythm: 1 }
        }
        TileVariant::ShellStitch => {
            let g = vec![
                vec![('╰', 0), ('─', 0), ('╮', 0), (' ', 0), (' ', 0), ('╭', 1), ('─', 1), ('╯', 1)],
                vec![(' ', 0), (' ', 0), ('│', 0), ('◠', 0), ('◠', 1), ('│', 1), (' ', 1), (' ', 1)],
                vec![('─', 0), ('╮', 0), ('╰', 0), ('─', 0), ('─', 1), ('╯', 1), ('╭', 1), ('─', 1)],
            ];
            TilePattern { cells: g, row_offset: 4, stagger_rhythm: 1 }
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
            TilePattern { cells: g, row_offset: 0, stagger_rhythm: 1 }
        }
        TileVariant::CrocodileScale => {
            let g = vec![
                vec![('╲', 0), (' ', 0), (' ', 0), (' ', 0), (' ', 0), ('╱', 0)],
                vec![(' ', 0), ('╲', 0), ('▁', 1), ('▁', 1), ('╱', 0), (' ', 0)],
                vec![(' ', 0), ('▕', 1), ('▓', 1), ('▓', 1), ('▏', 1), (' ', 0)],
                vec![('─', 0), ('╯', 0), (' ', 0), (' ', 0), ('╰', 0), ('─', 0)],
            ];
            TilePattern { cells: g, row_offset: 3, stagger_rhythm: 1 }
        }
    }
}

/// Randomizable parameters for a tile fill instance.
#[derive(Clone, Copy)]
pub struct TileParams {
    pub variant: TileVariant,
    pub density: f32,           // 0.0-1.0, cell draw probability
    pub stagger_override: i8,   // -1 = use default, 0 = force no stagger, 1+ = override offset
    pub rhythm_override: u8,    // 0 = use default, 1+ = override stagger_rhythm
    pub jitter: f32,            // 0.0-1.0, probability of replacing glyph with random line char
    pub skew: u32,              // 0-100, how much pattern bleeds past rect boundary
}

impl TileParams {
    pub fn new(variant: TileVariant) -> Self {
        TileParams { variant, density: 1.0, stagger_override: -1, rhythm_override: 0, jitter: 0.0, skew: 0 }
    }

    pub fn randomized(rng: &mut StdRng) -> Self {
        let variant = tile_variant_from_index(rng.random_range(0..TILE_VARIANT_COUNT));
        let density = if rng.random_range(0..4) == 0 { rng.random_range(60..95) as f32 / 100.0 } else { 1.0 };
        // 40% chance to vary stagger from default
        let stagger_override = if rng.random_range(0..5) < 2 {
            rng.random_range(0..6) as i8  // 0 = no stagger, 1-5 = various offsets
        } else {
            -1  // default
        };
        // 30% chance to vary rhythm from default
        let rhythm_override = if rng.random_range(0..10) < 3 {
            rng.random_range(1..5)  // group 1-4 tile-rows before shifting
        } else {
            0  // default
        };
        // 30% chance of skew (pattern bleeds past rect edges)
        let skew = if rng.random_range(0..10u32) < 3 {
            rng.random_range(15..60)
        } else {
            0
        };
        TileParams { variant, density, stagger_override, rhythm_override, jitter: 0.0, skew }
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
            if y >= grid.len() || x >= grid[0].len() { continue; }
            let (ch, ci) = tile.at(x - rect.x, y - rect.y);
            if ch == ' ' { continue; }
            let bg = grid[y][x].bg;
            let fg = if ci == 0 { color } else { color2 };
            grid[y][x] = Cell::with_bg(ch, fg, bg);
        }
    }
}

/// Fill a rect with a tile pattern, full control.
pub fn fill_tile_ex(
    grid: &mut Grid,
    rect: &Rect,
    params: &TileParams,
    color: Color,
    color2: Color,
    jitter: f32,
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

    for y in y0..y1 {
        for x in x0..x1 {
            // distance past rect boundary (0 if inside)
            let dx = if x < rect.x { rect.x - x }
                     else if x >= rect.x + rect.w { x - (rect.x + rect.w) + 1 }
                     else { 0 };
            let dy = if y < rect.y { rect.y - y }
                     else if y >= rect.y + rect.h { y - (rect.y + rect.h) + 1 }
                     else { 0 };
            let dist = dx.max(dy) as f32;

            // outside rect: probabilistic dropout
            if dist > 0.0 {
                if extend == 0 { continue; }
                let survive = 1.0 - (dist / extend as f32).powf(0.7);
                if survive <= 0.0 || rng.random::<f32>() > survive { continue; }
            }

            if params.density < 1.0 && rng.random::<f32>() > params.density { continue; }

            // sample tile using rect-relative coords so pattern is continuous
            // use i32 to handle coords before rect origin without overflow
            let px = tile.period_x();
            let py = tile.period_y();
            let raw_tx = (x as i32 - rect.x as i32 + phase_x as i32).rem_euclid(px as i32) as usize;
            let raw_ty = (y as i32 - rect.y as i32 + phase_y as i32).rem_euclid(py as i32) as usize;
            let mut tx = raw_tx;
            let mut ty = raw_ty;

            // boundary mirroring: near rect edges, reflect tile coords so
            // open shapes (arches, waves) fold back into closed loops
            if params.skew > 0 {
                // distance from bottom/right edges of the rect
                let ry_bot = if y < rect.y + rect.h { (rect.y + rect.h - 1) - y } else { 0 };
                let rx_right = if x < rect.x + rect.w { (rect.x + rect.w - 1) - x } else { 0 };
                let from_top = if y >= rect.y { y - rect.y } else { 0 };
                let from_left = if x >= rect.x { x - rect.x } else { 0 };

                // mirror within one tile period of the bottom/right edge
                if ry_bot < py && from_top >= py {
                    ty = py - 1 - ty;
                }
                if rx_right < px && from_left >= px {
                    tx = px - 1 - tx;
                }
            }

            let (mut ch, ci) = tile.at(tx, ty);
            if ch == ' ' { continue; }
            if jitter > 0.0 && rng.random::<f32>() < jitter {
                ch = jitter_glyphs[rng.random_range(0..jitter_glyphs.len())];
            }
            let bg = grid[y][x].bg;
            let mut fg = if ci == 0 { color } else { color2 };
            if jitter > 0.0 {
                let drift = rng.random_range(0..=20) as u8;
                if drift > 10 { fg = lighten(fg, drift - 10); } else { fg = darken(fg, 10 - drift); }
            }
            grid[y][x] = Cell::with_bg(ch, fg, bg);
        }
    }
}

// ── Noise fills (per-cell random, no periodic structure) ────────────

/// Weighted glyph entry: (char, color_index 0 or 1, cumulative weight).
/// Weights don't need to sum to 1.0 -- they're normalized at fill time.
pub struct NoiseGlyph {
    pub ch: char,
    pub ci: u8,       // 0 = primary color, 1 = secondary
    pub weight: f32,  // relative probability
}

/// Predefined noise palettes.
#[derive(Clone, Copy)]
pub enum NoiseVariant {
    Truchet,       // classic ╱╲ 50/50, coherence 0.0
    Higaki,        // ╱╲╳ with gaps, coherence 0.7 (long runs, rare breaks)
    HigakiStatic,  // ╱╲╳ with gaps, coherence 0.0 (per-cell random, the original)
    Grass,         // ╱╲│ with spaces, coherence 0.5
    Static,        // ╱╲─│╳·░, coherence 0.0 (pure random)
    Dot,           // ·∙°, coherence 0.6
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
            NoiseGlyph { ch: '╱', ci: 0, weight: 1.0 },
            NoiseGlyph { ch: '╲', ci: 0, weight: 1.0 },
        ],
        NoiseVariant::Higaki | NoiseVariant::HigakiStatic => vec![
            NoiseGlyph { ch: '╱', ci: 0, weight: 3.0 },
            NoiseGlyph { ch: '╲', ci: 0, weight: 3.0 },
            NoiseGlyph { ch: '╳', ci: 1, weight: 2.0 },
            NoiseGlyph { ch: ' ', ci: 0, weight: 1.0 },
        ],
        NoiseVariant::Grass => vec![
            NoiseGlyph { ch: '╱', ci: 0, weight: 2.0 },
            NoiseGlyph { ch: '╲', ci: 0, weight: 2.0 },
            NoiseGlyph { ch: '│', ci: 1, weight: 1.5 },
            NoiseGlyph { ch: ' ', ci: 0, weight: 3.0 },
        ],
        NoiseVariant::Static => vec![
            NoiseGlyph { ch: '╱', ci: 0, weight: 2.0 },
            NoiseGlyph { ch: '╲', ci: 0, weight: 2.0 },
            NoiseGlyph { ch: '─', ci: 1, weight: 1.0 },
            NoiseGlyph { ch: '│', ci: 1, weight: 1.0 },
            NoiseGlyph { ch: '╳', ci: 1, weight: 0.5 },
            NoiseGlyph { ch: '·', ci: 0, weight: 1.0 },
            NoiseGlyph { ch: '░', ci: 0, weight: 0.5 },
        ],
        NoiseVariant::Dot => vec![
            NoiseGlyph { ch: '·', ci: 0, weight: 3.0 },
            NoiseGlyph { ch: '∙', ci: 1, weight: 1.0 },
            NoiseGlyph { ch: '°', ci: 1, weight: 0.5 },
            NoiseGlyph { ch: ' ', ci: 0, weight: 5.0 },
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
            if y >= grid.len() || x >= grid[0].len() { continue; }
            // coherence check: continue run or break?
            if coherence <= 0.0 || rng.random::<f32>() >= coherence {
                cur = sample_glyph(&cdf, rng);
            }
            let g = &glyphs[cur];
            if g.ch == ' ' { continue; }
            let bg = grid[y][x].bg;
            let fg = if g.ci == 0 { color } else { color2 };
            grid[y][x] = Cell::with_bg(g.ch, fg, bg);
        }
    }
}

/// Fill entire grid with Truchet noise. Replaces the duplicated inline loops.
pub fn fill_truchet(grid: &mut Grid, width: usize, height: usize, color: Color, rng: &mut StdRng) {
    let rect = Rect { x: 0, y: 0, w: width, h: height };
    fill_noise(grid, &rect, NoiseVariant::Truchet, color, color, rng);
}

// ── Line art fills ──────────────────────────────────────────────────

/// Crosshatch: deterministic diagonal tiling. Denser than random Truchet.
pub fn draw_crosshatch(grid: &mut Grid, rect: &Rect, color: Color, color2: Color) {
    for y in rect.y..rect.y + rect.h {
        for x in rect.x..rect.x + rect.w {
            if y >= grid.len() || x >= grid[0].len() { continue; }
            let bg = grid[y][x].bg;
            let (ch, c) = if (x + y) % 2 == 0 { ('╱', color) } else { ('╲', color2) };
            grid[y][x] = Cell::with_bg(ch, c, bg);
        }
    }
}

/// Guilloche: interlocking wave curves from rounded box-drawing chars.
pub fn draw_guilloche(grid: &mut Grid, rect: &Rect, color: Color, color2: Color) {
    for y in rect.y..rect.y + rect.h {
        for x in rect.x..rect.x + rect.w {
            if y >= grid.len() || x >= grid[0].len() { continue; }
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
            if y >= grid.len() || x >= grid[0].len() { continue; }
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
            if y >= grid.len() || x >= grid[0].len() { continue; }
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
            if y >= grid.len() || x >= grid[0].len() { continue; }
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
