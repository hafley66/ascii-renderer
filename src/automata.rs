use crossterm::style::Color;
use rand::rngs::StdRng;
use rand::RngExt;
use crate::types::*;
use crate::color::*;
use crate::fills::*;
use crate::sprites::*;
use crate::walker::*;

// ── CA rules (Birth/Survival) ───────────────────────────────────────

pub struct CaRule {
    birth: [bool; 9],
    survival: [bool; 9],
}

impl CaRule {
    pub fn parse(spec: &str) -> Self {
        let mut birth = [false; 9];
        let mut survival = [false; 9];
        let parts: Vec<&str> = spec.split('/').collect();
        if parts.len() == 2 {
            for ch in parts[0].trim_start_matches('B').chars() {
                if let Some(n) = ch.to_digit(10) { birth[n as usize] = true; }
            }
            for ch in parts[1].trim_start_matches('S').chars() {
                if let Some(n) = ch.to_digit(10) { survival[n as usize] = true; }
            }
        }
        CaRule { birth, survival }
    }

    pub fn life() -> Self  { Self::parse("B3/S23") }
    pub fn cave() -> Self  { Self::parse("B5678/S45678") }
    pub fn maze() -> Self  { Self::parse("B3/S12345") }
    pub fn coral() -> Self { Self::parse("B3/S45678") }
}

// ── CA grid ─────────────────────────────────────────────────────────

pub struct CaGrid {
    pub cells: Vec<Vec<bool>>,
    pub w: usize,
    pub h: usize,
}

impl CaGrid {
    pub fn new(w: usize, h: usize) -> Self {
        CaGrid { cells: vec![vec![false; w]; h], w, h }
    }

    pub fn seed_random(&mut self, density: f64, rng: &mut StdRng) {
        let thresh = (density * 1000.0) as u32;
        for row in &mut self.cells {
            for cell in row.iter_mut() {
                *cell = rng.random_range(0u32..1000) < thresh;
            }
        }
    }

    pub fn seed_points(&mut self, count: usize, rng: &mut StdRng) {
        for _ in 0..count {
            let x = rng.random_range(0..self.w);
            let y = rng.random_range(0..self.h);
            self.cells[y][x] = true;
            // cross seed: also light immediate neighbors for faster growth
            if x > 0 { self.cells[y][x - 1] = true; }
            if x + 1 < self.w { self.cells[y][x + 1] = true; }
            if y > 0 { self.cells[y - 1][x] = true; }
            if y + 1 < self.h { self.cells[y + 1][x] = true; }
        }
    }

    /// Moore neighborhood count (8-connected)
    fn neighbors(&self, x: usize, y: usize) -> u8 {
        let mut n = 0u8;
        for dy in -1i32..=1 {
            for dx in -1i32..=1 {
                if dx == 0 && dy == 0 { continue; }
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                if nx >= 0 && ny >= 0 && (nx as usize) < self.w && (ny as usize) < self.h
                    && self.cells[ny as usize][nx as usize]
                {
                    n += 1;
                }
            }
        }
        n
    }

    /// Cardinal neighbor bitmask: N=1 E=2 S=4 W=8
    pub fn cardinal(&self, x: usize, y: usize) -> u8 {
        let mut m = 0u8;
        if y > 0 && self.cells[y - 1][x]            { m |= 1; }
        if x + 1 < self.w && self.cells[y][x + 1]   { m |= 2; }
        if y + 1 < self.h && self.cells[y + 1][x]    { m |= 4; }
        if x > 0 && self.cells[y][x - 1]            { m |= 8; }
        m
    }

    /// Diagonal neighbor bitmask: NE=1 SE=2 SW=4 NW=8
    pub fn diagonal(&self, x: usize, y: usize) -> u8 {
        let mut m = 0u8;
        if y > 0 && x + 1 < self.w && self.cells[y - 1][x + 1]  { m |= 1; }
        if y + 1 < self.h && x + 1 < self.w && self.cells[y + 1][x + 1] { m |= 2; }
        if y + 1 < self.h && x > 0 && self.cells[y + 1][x - 1]  { m |= 4; }
        if y > 0 && x > 0 && self.cells[y - 1][x - 1]           { m |= 8; }
        m
    }

    pub fn step(&mut self, rule: &CaRule) {
        let mut next = vec![vec![false; self.w]; self.h];
        for y in 0..self.h {
            for x in 0..self.w {
                let n = self.neighbors(x, y) as usize;
                next[y][x] = if self.cells[y][x] { rule.survival[n] } else { rule.birth[n] };
            }
        }
        self.cells = next;
    }

    pub fn evolve(&mut self, rule: &CaRule, generations: usize) {
        for _ in 0..generations { self.step(rule); }
    }

    pub fn live_count(&self) -> usize {
        self.cells.iter().flat_map(|r| r.iter()).filter(|&&c| c).count()
    }
}

// ── Glyph style ─────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub enum GlyphStyle {
    Box,
    Round,
    Diagonal,
    Heavy,
}

/// Cardinal bitmask (N=1 E=2 S=4 W=8) → box-drawing char
fn cardinal_glyph(mask: u8, diag: u8, style: GlyphStyle) -> char {
    match style {
        GlyphStyle::Box => match mask {
            0  => '·',
            1  => '╵', 2 => '╶', 4 => '╷', 8 => '╴',
            5  => '│', 10 => '─',
            3  => '└', 6 => '┌', 9 => '┘', 12 => '┐',
            7  => '├', 11 => '┴', 13 => '┤', 14 => '┬',
            15 => '┼',
            _  => '·',
        },
        GlyphStyle::Round => match mask {
            0  => '·',
            1  => '╵', 2 => '╶', 4 => '╷', 8 => '╴',
            5  => '│', 10 => '─',
            3  => '╰', 6 => '╭', 9 => '╯', 12 => '╮',
            7  => '├', 11 => '┴', 13 => '┤', 14 => '┬',
            15 => '┼',
            _  => '·',
        },
        GlyphStyle::Diagonal => {
            // Prefer diagonal glyphs where geometry allows
            let card_n = mask.count_ones();
            let diag_n = diag.count_ones();
            if card_n == 0 && diag_n == 0 { return '·'; }
            // If more diagonal than cardinal, use slash/backslash
            if diag_n > card_n {
                let has_ne = diag & 1 != 0;
                let has_sw = diag & 4 != 0;
                let has_nw = diag & 8 != 0;
                let has_se = diag & 2 != 0;
                if (has_ne || has_sw) && !(has_nw || has_se) { return '╱'; }
                if (has_nw || has_se) && !(has_ne || has_sw) { return '╲'; }
                return '╳';
            }
            // Fall through to box drawing
            match mask {
                0  => '·',
                1 | 4 | 5 => '│',
                2 | 8 | 10 => '─',
                3 => '╱', 12 => '╱',
                6 => '╲', 9 => '╲',
                _ => '┼',
            }
        },
        GlyphStyle::Heavy => match mask {
            0  => '·',
            1  => '╹', 2 => '╺', 4 => '╻', 8 => '╸',
            5  => '┃', 10 => '━',
            3  => '┗', 6 => '┏', 9 => '┛', 12 => '┓',
            7  => '┣', 11 => '┻', 13 => '┫', 14 => '┳',
            15 => '╋',
            _  => '·',
        },
    }
}

// ── Cell classification ─────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
pub enum CellRole {
    Isolated,
    Endpoint,
    Straight,
    Corner,
    Junction,
    Cross,
}

fn classify(cardinal: u8) -> CellRole {
    match cardinal.count_ones() {
        0 => CellRole::Isolated,
        1 => CellRole::Endpoint,
        2 if cardinal == 5 || cardinal == 10 => CellRole::Straight,
        2 => CellRole::Corner,
        3 => CellRole::Junction,
        _ => CellRole::Cross,
    }
}

// ── Render pipeline ─────────────────────────────────────────────────

/// Render CA grid onto the display grid.
/// Phase 1: every live cell gets a glyph from its cardinal neighborhood.
/// Phase 2 (if seed_primitives): endpoints → flowers, crosses → frets, junctions → trees.
pub fn render_ca(
    grid: &mut Grid,
    rect: &Rect,
    ca: &CaGrid,
    style: GlyphStyle,
    palette: &[Color; 5],
    seed_primitives: bool,
    rng: &mut StdRng,
) {
    // Phase 1: glyph from neighborhood
    for y in 0..ca.h.min(rect.h) {
        for x in 0..ca.w.min(rect.w) {
            if !ca.cells[y][x] { continue; }
            let gx = rect.x + x;
            let gy = rect.y + y;
            if gy >= grid.len() || gx >= grid[0].len() { continue; }

            let card = ca.cardinal(x, y);
            let diag = ca.diagonal(x, y);
            let role = classify(card);
            let ch = cardinal_glyph(card, diag, style);

            let color = match role {
                CellRole::Isolated  => palette[3],
                CellRole::Endpoint  => lighten(palette[1], 20),
                CellRole::Straight  => palette[1],
                CellRole::Corner    => palette[2],
                CellRole::Junction  => lighten(palette[2], 30),
                CellRole::Cross     => palette[3],
            };

            grid[gy][gx] = Cell::new(ch, color);
        }
    }

    if !seed_primitives { return; }

    // Phase 2: seed primitives at structural sites
    // Collect sites first to avoid borrow issues
    let mut flower_sites: Vec<(usize, usize)> = Vec::new();
    let mut fret_sites: Vec<(usize, usize)> = Vec::new();
    let mut tree_sites: Vec<(usize, usize)> = Vec::new();

    for y in 0..ca.h.min(rect.h) {
        for x in 0..ca.w.min(rect.w) {
            if !ca.cells[y][x] { continue; }
            let card = ca.cardinal(x, y);
            let role = classify(card);
            let gx = rect.x + x;
            let gy = rect.y + y;

            match role {
                CellRole::Endpoint => {
                    if rng.random_range(0u32..5) == 0 {
                        flower_sites.push((gx, gy));
                    }
                }
                CellRole::Cross => {
                    if rng.random_range(0u32..6) == 0 {
                        fret_sites.push((gx, gy));
                    }
                }
                CellRole::Junction => {
                    // Only grow upward from junctions that have a north connection
                    if card & 1 != 0 && rng.random_range(0u32..8) == 0 && y > 6 {
                        tree_sites.push((gx, gy));
                    }
                }
                _ => {}
            }
        }
    }

    for (gx, gy) in flower_sites {
        let s = rng.random_range(0..5);
        draw_flower(grid, gx, gy, s, palette[3]);
    }

    for (gx, gy) in fret_sites {
        let steps = rng.random_range(2..4);
        let dim = darken(palette[2], 20);
        draw_stepped_fret(grid, gx as i32, gy as i32, steps, Dir::Right, dim);
    }

    for (gx, gy) in tree_sites {
        let tree_h = rng.random_range(4..8).min(gy.saturating_sub(rect.y));
        if tree_h >= 4 {
            let spread = rng.random_range(3..6);
            grow_tree(grid, gx, gy, gy.saturating_sub(tree_h), spread, palette[1], rng);
        }
    }
}

// ── Full pipeline entry point ───────────────────────────────────────

pub fn render_automata(
    grid: &mut Grid,
    rect: &Rect,
    rule_name: &str,
    density: f64,
    generations: usize,
    style: GlyphStyle,
    palette: &[Color; 5],
    seed_primitives: bool,
    rng: &mut StdRng,
) {
    let mut ca = CaGrid::new(rect.w, rect.h);

    let rule = match rule_name {
        "life"  => CaRule::life(),
        "cave"  => CaRule::cave(),
        "maze"  => CaRule::maze(),
        "coral" => CaRule::coral(),
        s       => CaRule::parse(s),
    };

    match rule_name {
        "cave" => {
            // Start dense, rule carves out caves
            ca.seed_random(density.max(0.48), rng);
        }
        "maze" => {
            // Maze (B3/S12345): random fill ~38% grows into dense corridors
            ca.seed_random(0.38, rng);
        }
        _ => {
            ca.seed_random(density, rng);
        }
    }

    ca.evolve(&rule, generations);
    render_ca(grid, rect, &ca, style, palette, seed_primitives, rng);
}

// ── Coarse-scale CA layout ──────────────────────────────────────────
//
// Each CA cell maps to a cell_w x cell_h block of the render grid.
// Connected components of live cells become regions.
// Region shape determines what primitive fills it.

/// A connected component of CA cells, mapped to render-grid coordinates.
pub struct CaRegion {
    pub cells: Vec<(usize, usize)>,  // CA coords
    pub bounds: Rect,                 // render-grid bounding rect
    pub area: usize,                  // cell count
}

/// Flood-fill connected components (4-connected) on the CA grid.
fn find_components(ca: &CaGrid) -> Vec<Vec<(usize, usize)>> {
    let mut visited = vec![vec![false; ca.w]; ca.h];
    let mut components = Vec::new();

    for y in 0..ca.h {
        for x in 0..ca.w {
            if !ca.cells[y][x] || visited[y][x] { continue; }
            let mut comp = Vec::new();
            let mut stack = vec![(x, y)];
            while let Some((cx, cy)) = stack.pop() {
                if cx >= ca.w || cy >= ca.h { continue; }
                if visited[cy][cx] || !ca.cells[cy][cx] { continue; }
                visited[cy][cx] = true;
                comp.push((cx, cy));
                if cx > 0 { stack.push((cx - 1, cy)); }
                if cy > 0 { stack.push((cx, cy - 1)); }
                stack.push((cx + 1, cy));
                stack.push((cx, cy + 1));
            }
            if !comp.is_empty() {
                components.push(comp);
            }
        }
    }
    components
}

/// Convert CA components to render-grid regions.
fn components_to_regions(
    components: Vec<Vec<(usize, usize)>>,
    cell_w: usize,
    cell_h: usize,
    offset_x: usize,
    offset_y: usize,
) -> Vec<CaRegion> {
    components.into_iter().map(|cells| {
        let min_x = cells.iter().map(|c| c.0).min().unwrap();
        let max_x = cells.iter().map(|c| c.0).max().unwrap();
        let min_y = cells.iter().map(|c| c.1).min().unwrap();
        let max_y = cells.iter().map(|c| c.1).max().unwrap();
        let bounds = Rect {
            x: offset_x + min_x * cell_w,
            y: offset_y + min_y * cell_h,
            w: (max_x - min_x + 1) * cell_w,
            h: (max_y - min_y + 1) * cell_h,
        };
        let area = cells.len();
        CaRegion { cells, bounds, area }
    }).collect()
}

/// Draw the CA skeleton at coarse scale: each live cell gets a box-drawing
/// char at its center in the render grid, connecting to neighbors.
fn draw_ca_skeleton(
    grid: &mut Grid,
    ca: &CaGrid,
    cell_w: usize,
    cell_h: usize,
    offset_x: usize,
    offset_y: usize,
    style: GlyphStyle,
    color: Color,
) {
    for y in 0..ca.h {
        for x in 0..ca.w {
            if !ca.cells[y][x] { continue; }
            let card = ca.cardinal(x, y);
            let diag = ca.diagonal(x, y);
            let ch = cardinal_glyph(card, diag, style);

            // Center of this CA cell in render coords
            let cx = offset_x + x * cell_w + cell_w / 2;
            let cy = offset_y + y * cell_h + cell_h / 2;
            if cy < grid.len() && cx < grid[0].len() {
                grid[cy][cx] = Cell::new(ch, color);
            }

            // Draw connecting lines to cardinal neighbors
            if card & 2 != 0 {
                // East: draw ─ from center to right edge
                for dx in 1..cell_w {
                    let gx = cx + dx;
                    if gx < grid[0].len() { grid[cy][gx] = Cell::new('─', darken(color, 30)); }
                }
            }
            if card & 4 != 0 {
                // South: draw │ from center to bottom edge
                for dy in 1..cell_h {
                    let gy = cy + dy;
                    if gy < grid.len() { grid[gy][cx] = Cell::new('│', darken(color, 30)); }
                }
            }
        }
    }
}

/// Full CA layout pipeline.
/// Returns regions sorted by area descending (largest first for text placement).
pub fn ca_layout(
    grid: &mut Grid,
    rect: &Rect,
    rule_name: &str,
    density: f64,
    generations: usize,
    palette: &[Color; 5],
    rng: &mut StdRng,
) -> Vec<Rect> {
    // Coarse scale: each CA cell = cell_w x cell_h render cells
    // Scale cells down for small terminals, up for large ones
    let cell_w = if rect.w >= 120 { 10 } else { 6 };
    let cell_h = if rect.h >= 60 { 5 } else { 3 };
    let ca_w = rect.w / cell_w;
    let ca_h = rect.h / cell_h;
    if ca_w < 4 || ca_h < 4 { return Vec::new(); }

    let mut ca = CaGrid::new(ca_w, ca_h);
    let rule = match rule_name {
        "life"  => CaRule::life(),
        "cave"  => CaRule::cave(),
        "maze"  => CaRule::maze(),
        "coral" => CaRule::coral(),
        s       => CaRule::parse(s),
    };

    match rule_name {
        "cave" => ca.seed_random(density.max(0.48), rng),
        _      => ca.seed_random(density, rng),
    }
    ca.evolve(&rule, generations);

    // Draw the coarse skeleton as connective tissue
    let style = match rng.random_range(0u32..4) {
        0 => GlyphStyle::Box,
        1 => GlyphStyle::Round,
        2 => GlyphStyle::Heavy,
        _ => GlyphStyle::Diagonal,
    };
    draw_ca_skeleton(grid, &ca, cell_w, cell_h, rect.x, rect.y, style, darken(palette[2], 40));

    // Find connected components and convert to render regions
    let components = find_components(&ca);
    let mut regions = components_to_regions(components, cell_w, cell_h, rect.x, rect.y);
    regions.sort_by(|a, b| b.area.cmp(&a.area));

    // Clip regions to grid bounds
    let grid_w = grid[0].len();
    let grid_h = grid.len();
    for r in &mut regions {
        if r.bounds.x + r.bounds.w > grid_w { r.bounds.w = grid_w.saturating_sub(r.bounds.x); }
        if r.bounds.y + r.bounds.h > grid_h { r.bounds.h = grid_h.saturating_sub(r.bounds.y); }
    }

    // Separate regions into text-worthy (large enough) and art-fill
    // A region needs at least 14 cols and 5 rows to hold any text block
    let min_text_w = 14;
    let min_text_h = 5;
    let max_text_slots = 4;
    let mut text_slots = 0;
    for r in &regions {
        if text_slots >= max_text_slots { break; }
        if r.bounds.w >= min_text_w && r.bounds.h >= min_text_h {
            text_slots += 1;
        } else {
            break; // regions are sorted by area, so no point continuing
        }
    }
    let art_regions: Vec<&CaRegion> = regions.iter().skip(text_slots).collect();

    // Collect as Rects for walk_and_fill_leaves
    let mut fill_rects: Vec<Rect> = Vec::new();
    let mut small_sites: Vec<(usize, usize, usize)> = Vec::new(); // (x, y, area)

    for region in &art_regions {
        let b = &region.bounds;
        if b.w < 3 || b.h < 2 { continue; }

        if region.area <= 2 {
            // Tiny: stamp a flower or fruit at center
            small_sites.push((b.x + b.w / 2, b.y + b.h / 2, region.area));
        } else if region.area <= 5 {
            // Small: individual primitive (tree, small fret)
            let cx = b.x + b.w / 2;
            let cy = b.y + b.h / 2;
            match rng.random_range(0u32..3) {
                0 => {
                    if b.h >= 6 {
                        let tree_top = b.y + 1;
                        let tree_root = b.y + b.h - 1;
                        let spread = (b.w / 3).max(2).min(8);
                        grow_tree(grid, cx, tree_root, tree_top, spread, palette[1], rng);
                    }
                }
                1 => {
                    let steps = (b.w / 4).max(2).min(5);
                    draw_stepped_fret(grid, cx as i32, cy as i32, steps, Dir::Right, palette[2]);
                }
                _ => {
                    if b.h >= 4 {
                        draw_pine(grid, cx, b.y + b.h - 1, 2, (b.w / 2).max(3), palette[1]);
                    }
                }
            }
        } else {
            // Medium/large: use the walker fill system
            fill_rects.push(*b);
        }
    }

    // Fill large regions with the existing walker system
    if !fill_rects.is_empty() {
        walk_and_fill_leaves(grid, &fill_rects, palette, rng);
    }

    // Stamp small sites
    for (x, y, _area) in small_sites {
        if y < grid_h && x < grid_w {
            let s = rng.random_range(0..5);
            if rng.random_range(0u32..2) == 0 {
                draw_flower(grid, x, y, s, palette[3]);
            } else {
                draw_fruit(grid, x, y, s, palette[1]);
            }
        }
    }

    // Return text-placement rects (largest regions first)
    regions.iter().take(text_slots).map(|r| r.bounds).collect()
}

