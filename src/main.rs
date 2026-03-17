use crossterm::style::Color;
use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};
use rand::RngExt;
use rand::rngs::StdRng;
use rand::SeedableRng;
use std::io::{self, IsTerminal, Read as _, Write};
use unicode_width::UnicodeWidthChar;

#[derive(Clone, Copy)]
struct Cell {
    ch: char,
    fg: Color,
    bg: Color,
}

impl Cell {
    fn blank() -> Self {
        Cell { ch: ' ', fg: Color::Reset, bg: Color::Reset }
    }

    fn new(ch: char, fg: Color) -> Self {
        Cell { ch, fg, bg: Color::Reset }
    }

    fn with_bg(ch: char, fg: Color, bg: Color) -> Self {
        Cell { ch, fg, bg }
    }
}

type Grid = Vec<Vec<Cell>>;

/// Display width of a string (accounts for fullwidth CJK chars).
fn display_width(s: &str) -> usize {
    s.chars().map(|c| c.width().unwrap_or(0)).sum()
}

/// Display width of a single char.
fn char_width(c: char) -> usize {
    c.width().unwrap_or(0)
}

// ── Layout engine ──────────────────────────────────────────────────

enum ContentItem {
    Text(String),
    Bar { label: String, value: f64, max: f64 },
    Rule,
}

struct ContentBlock {
    items: Vec<ContentItem>,
    padding: usize,
}

struct Rect {
    x: usize,
    y: usize,
    w: usize,
    h: usize,
}

/// Wrap a string to fit within max_width using greedy line breaking.
/// Returns the wrapped lines and the actual max line width used.
fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    if text.is_empty() {
        return vec![String::new()];
    }
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut current_w: usize = 0;
    for word in text.split_whitespace() {
        let word_w = display_width(word);
        if current_w == 0 {
            current = word.to_string();
            current_w = word_w;
        } else if current_w + 1 + word_w <= max_width {
            current.push(' ');
            current.push_str(word);
            current_w += 1 + word_w;
        } else {
            lines.push(current);
            current = word.to_string();
            current_w = word_w;
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

/// Measure a content block: returns (width, height) needed.
/// Width = max line width across all items + 2*padding.
/// Height = total lines + 2*padding.
fn measure_block(block: &ContentBlock, available_width: usize) -> (usize, usize) {
    let inner_w = available_width.saturating_sub(block.padding * 2);
    let mut max_line_w: usize = 0;
    let mut total_lines: usize = 0;

    for item in &block.items {
        match item {
            ContentItem::Text(s) => {
                let wrapped = wrap_text(s, inner_w);
                for line in &wrapped {
                    max_line_w = max_line_w.max(display_width(line));
                }
                total_lines += wrapped.len();
            }
            ContentItem::Bar { label, .. } => {
                max_line_w = max_line_w.max(inner_w);
                // label line + bar line
                total_lines += if label.is_empty() { 1 } else { 2 };
            }
            ContentItem::Rule => {
                max_line_w = max_line_w.max(inner_w);
                total_lines += 1;
            }
        }
    }
    (max_line_w + block.padding * 2, total_lines + block.padding * 2)
}

/// Minimum width a block needs to avoid any text wrapping.
fn min_block_width(block: &ContentBlock) -> usize {
    let mut max_w: usize = 0;
    for item in &block.items {
        match item {
            ContentItem::Text(s) => {
                max_w = max_w.max(display_width(s));
            }
            ContentItem::Bar { label, .. } => {
                max_w = max_w.max(display_width(label).max(8));
            }
            ContentItem::Rule => {}
        }
    }
    max_w + block.padding * 2
}

/// Render a content block into the grid at (rect.x, rect.y).
/// Clears the rect area first, then writes content.
fn render_block(grid: &mut Grid, block: &ContentBlock, rect: &Rect, fg: Color, bar_fg: Color) {
    // clear rect
    for y in rect.y..rect.y + rect.h {
        for x in rect.x..rect.x + rect.w {
            if y < grid.len() && x < grid[0].len() {
                grid[y][x] = Cell::blank();
            }
        }
    }

    let inner_w = rect.w.saturating_sub(block.padding * 2);
    let inner_x = rect.x + block.padding;
    let mut cy = rect.y + block.padding;

    for item in &block.items {
        match item {
            ContentItem::Text(s) => {
                let wrapped = wrap_text(s, inner_w);
                for line in &wrapped {
                    let mut col = 0usize;
                    for ch in line.chars() {
                        let cw = char_width(ch);
                        let gx = inner_x + col;
                        if gx + cw > rect.x + rect.w { break; }
                        if cy < grid.len() && gx < grid[0].len() {
                            grid[cy][gx] = Cell::new(ch, fg);
                            if cw == 2 && gx + 1 < grid[0].len() {
                                grid[cy][gx + 1] = Cell::blank();
                            }
                        }
                        col += cw;
                    }
                    cy += 1;
                }
            }
            ContentItem::Bar { label, value, max } => {
                if !label.is_empty() {
                    let mut col = 0usize;
                    for ch in label.chars() {
                        let cw = char_width(ch);
                        let gx = inner_x + col;
                        if gx + cw > rect.x + rect.w { break; }
                        if cy < grid.len() && gx < grid[0].len() {
                            grid[cy][gx] = Cell::new(ch, fg);
                            if cw == 2 && gx + 1 < grid[0].len() {
                                grid[cy][gx + 1] = Cell::blank();
                            }
                        }
                        col += cw;
                    }
                    cy += 1;
                }
                let fill_w = if *max > 0.0 {
                    ((value / max) * inner_w as f64).round() as usize
                } else {
                    0
                };
                for j in 0..inner_w {
                    let gx = inner_x + j;
                    if cy < grid.len() && gx < grid[0].len() {
                        let (ch, color) = if j < fill_w {
                            ('█', bar_fg)
                        } else {
                            ('░', darken(bar_fg, 80))
                        };
                        grid[cy][gx] = Cell::new(ch, color);
                    }
                }
                cy += 1;
            }
            ContentItem::Rule => {
                for j in 0..inner_w {
                    let gx = inner_x + j;
                    if cy < grid.len() && gx < grid[0].len() {
                        grid[cy][gx] = Cell::new('─', fg);
                    }
                }
                cy += 1;
            }
        }
    }
}

/// Two-column layout. Returns the rects placed so pattern fills can avoid them.
/// Each column gets half the canvas minus the gap. Blocks stack top-to-bottom in each column.
fn layout_two_col(
    grid: &mut Grid,
    left: &[ContentBlock],
    right: &[ContentBlock],
    gap: usize,
    margin: usize,
    fg: Color,
    bar_fg: Color,
) -> Vec<Rect> {
    let canvas_w = grid[0].len();
    let canvas_h = grid.len();
    let usable_w = canvas_w.saturating_sub(margin * 2 + gap);
    let col_w = usable_w / 2;
    let left_x = margin;
    let right_x = margin + col_w + gap;

    let mut rects = Vec::new();

    // left column
    let mut cy = margin;
    for block in left {
        let (_, h) = measure_block(block, col_w);
        let h = h.min(canvas_h.saturating_sub(cy));
        if h == 0 { break; }
        let rect = Rect { x: left_x, y: cy, w: col_w, h };
        render_block(grid, block, &rect, fg, bar_fg);
        rects.push(rect);
        cy += h + 1; // 1 row gap between blocks
    }

    // right column
    cy = margin;
    for block in right {
        let (_, h) = measure_block(block, col_w);
        let h = h.min(canvas_h.saturating_sub(cy));
        if h == 0 { break; }
        let rect = Rect { x: right_x, y: cy, w: col_w, h };
        render_block(grid, block, &rect, fg, bar_fg);
        rects.push(rect);
        cy += h + 1;
    }

    rects
}

// ── BSP layout ─────────────────────────────────────────────────────

struct BspNode {
    rect: Rect,
    left: Option<Box<BspNode>>,
    right: Option<Box<BspNode>>,
}

impl BspNode {
    fn new(x: usize, y: usize, w: usize, h: usize) -> Self {
        BspNode { rect: Rect { x, y, w, h }, left: None, right: None }
    }

    fn is_leaf(&self) -> bool {
        self.left.is_none() && self.right.is_none()
    }

    /// Recursively split until we have enough leaves or hit min size.
    /// gap: number of cells reserved between children (for grid lines).
    fn split_with_gap(&mut self, min_w: usize, min_h: usize, max_depth: usize, gap: usize, rng: &mut StdRng) {
        if max_depth == 0 { return; }

        let can_split_h = self.rect.w >= min_w * 2 + gap;
        let can_split_v = self.rect.h >= min_h * 2 + gap;

        if !can_split_h && !can_split_v { return; }

        // bias split axis by aspect ratio: wide nodes split horizontally (left/right),
        // tall nodes split vertically (top/bottom)
        let split_horizontal = if !can_split_v {
            true
        } else if !can_split_h {
            false
        } else {
            let ratio = self.rect.w as f64 / self.rect.h as f64;
            // terminal cells are ~1:2 aspect, so adjust threshold
            if ratio > 1.5 { true }
            else if ratio < 0.75 { false }
            else { rng.random_range(0..2) == 0 }
        };

        if split_horizontal {
            // split along x axis (left/right children)
            let range_lo = min_w;
            let range_hi = self.rect.w.saturating_sub(min_w + gap);
            if range_lo >= range_hi { return; }
            let split = rng.random_range(range_lo..range_hi);

            let mut left = Box::new(BspNode::new(
                self.rect.x, self.rect.y, split, self.rect.h,
            ));
            let mut right = Box::new(BspNode::new(
                self.rect.x + split + gap, self.rect.y,
                self.rect.w.saturating_sub(split + gap), self.rect.h,
            ));
            left.split_with_gap(min_w, min_h, max_depth - 1, gap, rng);
            right.split_with_gap(min_w, min_h, max_depth - 1, gap, rng);
            self.left = Some(left);
            self.right = Some(right);
        } else {
            // split along y axis (top/bottom children)
            let range_lo = min_h;
            let range_hi = self.rect.h.saturating_sub(min_h + gap);
            if range_lo >= range_hi { return; }
            let split = rng.random_range(range_lo..range_hi);

            let mut top = Box::new(BspNode::new(
                self.rect.x, self.rect.y, self.rect.w, split,
            ));
            let mut bottom = Box::new(BspNode::new(
                self.rect.x, self.rect.y + split + gap,
                self.rect.w, self.rect.h.saturating_sub(split + gap),
            ));
            top.split_with_gap(min_w, min_h, max_depth - 1, gap, rng);
            bottom.split_with_gap(min_w, min_h, max_depth - 1, gap, rng);
            self.left = Some(top);
            self.right = Some(bottom);
        }
    }

    /// Split with default gap of 1.
    fn split(&mut self, min_w: usize, min_h: usize, max_depth: usize, rng: &mut StdRng) {
        self.split_with_gap(min_w, min_h, max_depth, 1, rng);
    }

    /// Collect all leaf rects in traversal order.
    fn leaves(&self) -> Vec<&Rect> {
        if self.is_leaf() {
            return vec![&self.rect];
        }
        let mut out = Vec::new();
        if let Some(ref l) = self.left { out.extend(l.leaves()); }
        if let Some(ref r) = self.right { out.extend(r.leaves()); }
        out
    }
}

/// BSP layout: split canvas into regions, assign content blocks to leaves,
/// render blocks, return all leaf rects (content + empty pattern zones).
fn layout_bsp(
    grid: &mut Grid,
    blocks: &[ContentBlock],
    margin: usize,
    min_cell_w: usize,
    min_cell_h: usize,
    fg: Color,
    bar_fg: Color,
    rng: &mut StdRng,
) -> Vec<Rect> {
    let canvas_w = grid[0].len().saturating_sub(margin * 2);
    let canvas_h = grid.len().saturating_sub(margin * 2);

    // build BSP tree with enough depth to produce at least as many leaves as blocks
    let target_leaves = blocks.len().max(4);
    let max_depth = (target_leaves as f64).log2().ceil() as usize + 1;

    let mut root = BspNode::new(margin, margin, canvas_w, canvas_h);
    root.split(min_cell_w, min_cell_h, max_depth, rng);

    let leaves = root.leaves();

    // assign blocks to leaves: largest-area leaves get content first
    let mut leaf_rects: Vec<Rect> = leaves.iter().map(|r| {
        Rect { x: r.x, y: r.y, w: r.w, h: r.h }
    }).collect();

    // sort by area descending so content goes in the biggest regions
    leaf_rects.sort_by(|a, b| (b.w * b.h).cmp(&(a.w * a.h)));

    let mut all_rects = Vec::new();
    for (i, block) in blocks.iter().enumerate() {
        if i >= leaf_rects.len() { break; }
        let leaf = &leaf_rects[i];
        let (_, h) = measure_block(block, leaf.w);
        let render_rect = Rect {
            x: leaf.x,
            y: leaf.y,
            w: leaf.w,
            h: h.min(leaf.h),
        };
        render_block(grid, block, &render_rect, fg, bar_fg);
        all_rects.push(render_rect);
    }

    // return all leaf rects (content ones first, then empty ones for pattern fill)
    for r in &leaf_rects[blocks.len().min(leaf_rects.len())..] {
        all_rects.push(Rect { x: r.x, y: r.y, w: r.w, h: r.h });
    }

    all_rects
}

// ── Leaf walker ──────────────────────────────────────────────────────

#[derive(Clone, Copy)]
enum WalkerMood { Organic, Geometric, Empty }

/// Randomizable parameters for a tile fill instance.
#[derive(Clone, Copy)]
struct TileParams {
    variant: TileVariant,
    density: f32,           // 0.0-1.0, cell draw probability
    stagger_override: i8,   // -1 = use default, 0 = force no stagger, 1+ = override offset
    rhythm_override: u8,    // 0 = use default, 1+ = override stagger_rhythm
}

impl TileParams {
    fn new(variant: TileVariant) -> Self {
        TileParams { variant, density: 1.0, stagger_override: -1, rhythm_override: 0 }
    }

    fn randomized(rng: &mut StdRng) -> Self {
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
        TileParams { variant, density, stagger_override, rhythm_override }
    }
}

enum LeafFill {
    Tree(usize),
    Fret(usize),
    AztecDiamond(usize),
    Flower(usize),
    Fruit(usize),
    Crosshatch,
    Guilloche,
    Weave,
    Zigzag,
    DiamondLattice,
    Tile(TileParams),
    Noise(NoiseVariant),
    Nothing,
}

struct LeafWalker {
    mood: WalkerMood,
    energy: f32,
    prev_x: usize,
    prev_y: usize,
}

impl LeafWalker {
    fn new(center_x: usize, center_y: usize) -> Self {
        LeafWalker { mood: WalkerMood::Organic, energy: 1.0, prev_x: center_x, prev_y: center_y }
    }

    fn pick_fill(&self, rect: &Rect, rng: &mut StdRng) -> LeafFill {
        let area = rect.w * rect.h;

        // energy-based thinning
        if self.energy < 0.5 && rng.random_range(0..10) < 3 {
            return LeafFill::Nothing;
        }

        match self.mood {
            WalkerMood::Empty => LeafFill::Nothing,
            WalkerMood::Organic => {
                if area > 300 && rect.h > 15 && rect.w > 20 {
                    match rng.random_range(0..5) {
                        0..=2 => LeafFill::Tree(rng.random_range(0..4)),
                        3 => LeafFill::Noise(NoiseVariant::Grass),
                        _ => LeafFill::Noise(NoiseVariant::Dot),
                    }
                } else if area > 80 && rect.h > 6 && rect.w > 10 {
                    match rng.random_range(0..5) {
                        0 => LeafFill::Fruit(rng.random_range(0..5)),
                        1 => LeafFill::Noise(NoiseVariant::Grass),
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
                    // large: aztec diamond, line art, tiles, or noise
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
                    // medium: frets, line art, aztec, tiles, noise
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
                    // small: flowers, compact line art, tiles, noise
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

    fn step(&mut self, rect: &Rect, rng: &mut StdRng) {
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
/// walker mood/energy state. Creates visual flow instead of pure random.
fn walk_and_fill_leaves(
    grid: &mut Grid,
    leaves: &[Rect],
    palette: &[Color; 5],
    rng: &mut StdRng,
) {
    if leaves.is_empty() { return; }

    let mut walker = LeafWalker::new(grid[0].len() / 2, grid.len() / 2);
    let mut visited = vec![false; leaves.len()];

    for _ in 0..leaves.len() {
        // nearest unvisited leaf by Manhattan distance
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

// ── Mondrian layout ─────────────────────────────────────────────────

/// Mondrian palette: classic Piet Mondrian primary colors + white.
/// Returns (fill_colors, line_color).
fn mondrian_colors() -> ([Color; 5], Color) {
    let fills = [
        rgb(255, 255, 255),  // white (most common)
        rgb(220, 30, 30),    // red
        rgb(30, 60, 180),    // blue
        rgb(240, 210, 30),   // yellow
        rgb(255, 255, 255),  // white again (weight toward white)
    ];
    let line = rgb(20, 20, 20); // near-black grid lines
    (fills, line)
}

/// Draw thick Mondrian grid lines along all BSP split boundaries.
/// Walks the BSP tree and draws lines at each split seam.
fn draw_mondrian_lines(grid: &mut Grid, node: &BspNode, line_w: usize, color: Color) {
    if node.is_leaf() { return; }

    if let (Some(left), Some(right)) = (&node.left, &node.right) {
        let l = &left.rect;
        let r = &right.rect;
        let p = &node.rect; // parent rect for full-span lines

        if l.y == r.y {
            // horizontal split (side by side): vertical line spans parent height
            let line_x = l.x + l.w;
            for y in p.y..p.y + p.h {
                for dx in 0..line_w {
                    let x = line_x + dx;
                    if y < grid.len() && x < grid[0].len() {
                        grid[y][x] = Cell::with_bg(' ', color, color);
                    }
                }
            }
        } else {
            // vertical split (stacked): horizontal line spans parent width
            let line_y = l.y + l.h;
            for dy in 0..line_w {
                let y = line_y + dy;
                for x in p.x..p.x + p.w {
                    if y < grid.len() && x < grid[0].len() {
                        grid[y][x] = Cell::with_bg(' ', color, color);
                    }
                }
            }
        }

        draw_mondrian_lines(grid, left, line_w, color);
        draw_mondrian_lines(grid, right, line_w, color);
    }
}

/// Mondrian layout: BSP partition with thick black grid lines and
/// primary-color filled regions. Content blocks go in the largest leaves.
fn layout_mondrian(
    grid: &mut Grid,
    blocks: &[ContentBlock],
    margin: usize,
    line_w: usize,
    min_cell_w: usize,
    min_cell_h: usize,
    text_fg: Color,
    bar_fg: Color,
    fill_colors: &[Color; 5],
    line_color: Color,
    rng: &mut StdRng,
) -> Vec<Rect> {
    let grid_w = grid[0].len();
    let grid_h = grid.len();

    // inset BSP root by border (line_w) + margin so leaves never overlap the outer border
    let inset = line_w + margin;
    let canvas_w = grid_w.saturating_sub(inset * 2);
    let canvas_h = grid_h.saturating_sub(inset * 2);

    // more splits than regular BSP for the Mondrian look
    let target_leaves = blocks.len().max(6);
    let max_depth = (target_leaves as f64).log2().ceil() as usize + 2;

    let mut root = BspNode::new(inset, inset, canvas_w, canvas_h);
    root.split_with_gap(min_cell_w, min_cell_h, max_depth, line_w, rng);

    // fill each leaf with a Mondrian color
    let leaves = root.leaves();
    for (i, leaf) in leaves.iter().enumerate() {
        // weight toward white: small regions more likely to get color
        let area = leaf.w * leaf.h;
        let color_idx = if area > 400 {
            if rng.random_range(0..4) == 0 { rng.random_range(1..4) } else { 0 }
        } else if area > 100 {
            if rng.random_range(0..3) == 0 { rng.random_range(1..4) } else { 0 }
        } else {
            if rng.random_range(0..2) == 0 { rng.random_range(1..4) } else { 0 }
        };
        let _ = i;
        let bg = fill_colors[color_idx];

        for y in leaf.y..leaf.y + leaf.h {
            for x in leaf.x..leaf.x + leaf.w {
                if y < grid.len() && x < grid[0].len() {
                    grid[y][x] = Cell::with_bg(' ', Color::Reset, bg);
                }
            }
        }
    }

    // draw grid lines at every BSP split boundary
    draw_mondrian_lines(grid, &root, line_w, line_color);

    // outer border
    let w = grid_w;
    let h = grid_h;
    for dy in 0..line_w {
        for x in 0..w {
            if dy < h { grid[dy][x] = Cell::with_bg(' ', line_color, line_color); }
            if h - 1 - dy < h { grid[h - 1 - dy][x] = Cell::with_bg(' ', line_color, line_color); }
        }
    }
    for dx in 0..line_w {
        for y in 0..h {
            if dx < w { grid[y][dx] = Cell::with_bg(' ', line_color, line_color); }
            if w - 1 - dx < w { grid[y][w - 1 - dx] = Cell::with_bg(' ', line_color, line_color); }
        }
    }

    // assign content blocks to leaves by best fit:
    // for each block, find the leaf that is wide enough and has the most area.
    // fall back to widest leaf if none are wide enough.
    let leaf_rects: Vec<Rect> = leaves.iter().map(|r| {
        Rect { x: r.x, y: r.y, w: r.w, h: r.h }
    }).collect();
    let mut used = vec![false; leaf_rects.len()];
    let mut content_rects = Vec::new();

    for block in blocks.iter() {
        let needed_w = min_block_width(block);
        let (_, needed_h) = measure_block(block, needed_w);

        // find best leaf: wide enough and tall enough, prefer largest area
        let mut best: Option<usize> = None;
        let mut best_area: usize = 0;
        for (li, leaf) in leaf_rects.iter().enumerate() {
            if used[li] { continue; }
            let area = leaf.w * leaf.h;
            if leaf.w >= needed_w + 2 && leaf.h >= needed_h + 2 && area > best_area {
                best = Some(li);
                best_area = area;
            }
        }
        // fallback: widest unused leaf
        if best.is_none() {
            let mut best_w = 0;
            for (li, leaf) in leaf_rects.iter().enumerate() {
                if used[li] { continue; }
                if leaf.w > best_w {
                    best = Some(li);
                    best_w = leaf.w;
                }
            }
        }

        if let Some(li) = best {
            used[li] = true;
            let leaf = &leaf_rects[li];
            // measure at inner width (leaf minus border overhead) for accurate height
            let inner_w = leaf.w.saturating_sub(2);
            let (_, bh) = measure_block(block, inner_w);
            let render_rect = Rect {
                x: leaf.x,
                y: leaf.y,
                w: leaf.w,
                h: bh.min(leaf.h),
            };
            render_block_with_border(grid, block, &render_rect, text_fg, bar_fg, false, rng);
            content_rects.push(render_rect);
        }
    }

    // return content rects first, then unused leaves
    let mut all_rects = content_rects;
    for (li, r) in leaf_rects.iter().enumerate() {
        if !used[li] {
            all_rects.push(Rect { x: r.x, y: r.y, w: r.w, h: r.h });
        }
    }

    all_rects
}

/// Like render_block but preserves existing bg color of each cell
/// instead of clearing to blank. Used by Mondrian to keep color fills.
fn render_block_preserve_bg(grid: &mut Grid, block: &ContentBlock, rect: &Rect, fg: Color, bar_fg: Color) {
    let inner_x = rect.x + block.padding;
    let inner_y = rect.y + block.padding;
    let inner_w = rect.w.saturating_sub(block.padding * 2);
    let max_x = rect.x + rect.w;
    let max_y = rect.y + rect.h;

    let mut cy = inner_y;
    for item in &block.items {
        if cy >= max_y { break; }
        match item {
            ContentItem::Text(s) => {
                let wrapped = wrap_text(s, inner_w);
                for line in &wrapped {
                    if cy >= max_y { break; }
                    let mut col = 0usize; // display column offset
                    for ch in line.chars() {
                        let cw = char_width(ch);
                        let x = inner_x + col;
                        if x + cw > max_x { break; }
                        if cy < grid.len() && x < grid[0].len() {
                            let existing_bg = grid[cy][x].bg;
                            grid[cy][x] = Cell::with_bg(ch, fg, existing_bg);
                            // fullwidth chars: blank the next cell so it doesn't
                            // show stale content (terminal cursor advances 2 cols)
                            if cw == 2 && x + 1 < grid[0].len() {
                                grid[cy][x + 1] = Cell::with_bg(' ', fg, existing_bg);
                            }
                        }
                        col += cw;
                    }
                    cy += 1;
                }
            }
            ContentItem::Bar { label, value, max } => {
                if !label.is_empty() && cy < max_y {
                    let mut col = 0usize;
                    for ch in label.chars() {
                        let cw = char_width(ch);
                        let x = inner_x + col;
                        if x + cw > max_x { break; }
                        if cy < grid.len() && x < grid[0].len() {
                            let existing_bg = grid[cy][x].bg;
                            grid[cy][x] = Cell::with_bg(ch, fg, existing_bg);
                            if cw == 2 && x + 1 < grid[0].len() {
                                grid[cy][x + 1] = Cell::with_bg(' ', fg, existing_bg);
                            }
                        }
                        col += cw;
                    }
                    cy += 1;
                }
                if cy >= max_y { continue; }
                let bar_w = inner_w.min(max_x.saturating_sub(inner_x));
                let filled = ((value / max) * bar_w as f64) as usize;
                for j in 0..bar_w {
                    let x = inner_x + j;
                    if x >= max_x { break; }
                    let ch = if j < filled { '█' } else { '░' };
                    let color = if j < filled { bar_fg } else { fg };
                    if cy < grid.len() && x < grid[0].len() {
                        let existing_bg = grid[cy][x].bg;
                        grid[cy][x] = Cell::with_bg(ch, color, existing_bg);
                    }
                }
                cy += 1;
            }
            ContentItem::Rule => {
                let rule_w = inner_w.min(max_x.saturating_sub(inner_x));
                for j in 0..rule_w {
                    let x = inner_x + j;
                    if cy < grid.len() && x < grid[0].len() {
                        let existing_bg = grid[cy][x].bg;
                        grid[cy][x] = Cell::with_bg('─', fg, existing_bg);
                    }
                }
                cy += 1;
            }
        }
    }
}

/// Wrapper: draw a decorative border, then render content in the inset area.
/// If `clear` is true, clears the rect before drawing (use for truchet bg).
/// If false, preserves existing bg (use for mondrian color fills).
fn render_block_with_border(
    grid: &mut Grid,
    block: &ContentBlock,
    rect: &Rect,
    fg: Color,
    bar_fg: Color,
    clear: bool,
    rng: &mut StdRng,
) {
    if clear {
        for y in rect.y..rect.y + rect.h {
            for x in rect.x..rect.x + rect.w {
                if y < grid.len() && x < grid[0].len() {
                    grid[y][x] = Cell::blank();
                }
            }
        }
    }

    let style = pick_border_style(rng, rect.w, rect.h);
    let inset = border_inset(&style);

    // fall back to borderless if rect too small for border + content
    if rect.w <= inset * 2 + 4 || rect.h <= inset * 2 + 2 {
        render_block_preserve_bg(grid, block, rect, fg, bar_fg);
        return;
    }

    draw_box_border(grid, rect, &style, fg);

    // corner embellishments on non-fret, non-rounded borders (50% chance, needs space)
    if !matches!(style, BorderStyle::Fret | BorderStyle::Rounded) && rect.w >= 7 && rect.h >= 5 {
        if rng.random_range(0..2) == 0 {
            let corner_style = rng.random_range(0..6);
            draw_corner_embellishments(grid, rect, corner_style, fg);
        }
    }

    let inner = Rect {
        x: rect.x + inset,
        y: rect.y + inset,
        w: rect.w - inset * 2,
        h: rect.h - inset * 2,
    };
    render_block_preserve_bg(grid, block, &inner, fg, bar_fg);
}

// ── Markdown parser ────────────────────────────────────────────────

fn flush_block(blocks: &mut Vec<ContentBlock>, items: &mut Vec<ContentItem>) {
    if items.is_empty() { return; }
    blocks.push(ContentBlock {
        items: items.drain(..).collect(),
        padding: 1,
    });
}

fn parse_markdown(input: &str) -> Vec<ContentBlock> {
    let parser = Parser::new(input);
    let mut blocks: Vec<ContentBlock> = Vec::new();
    let mut items: Vec<ContentItem> = Vec::new();
    let mut text_buf = String::new();
    let mut in_blockquote = false;

    for event in parser {
        match event {
            Event::Text(t) => text_buf.push_str(&t),
            Event::Code(c) => {
                text_buf.push('`');
                text_buf.push_str(&c);
                text_buf.push('`');
            }
            Event::SoftBreak => text_buf.push(' '),
            Event::HardBreak => {
                if !text_buf.is_empty() {
                    let line = text_buf.drain(..).collect::<String>();
                    items.push(ContentItem::Text(line));
                }
            }

            Event::Start(Tag::Heading { .. }) => {
                // new heading = new section. flush previous section.
                flush_block(&mut blocks, &mut items);
            }
            Event::Start(Tag::BlockQuote(_)) => {
                in_blockquote = true;
            }
            Event::Start(Tag::Paragraph)
            | Event::Start(Tag::List(_))
            | Event::Start(Tag::Item)
            | Event::Start(Tag::CodeBlock(_)) => {}
            Event::Start(_) => {}

            Event::End(TagEnd::Heading(level)) => {
                // heading text becomes the section title, stays in current items
                let title = text_buf.drain(..).collect::<String>();
                if level == HeadingLevel::H1 {
                    items.push(ContentItem::Text(format!("「 {} 」", title.to_uppercase())));
                } else {
                    items.push(ContentItem::Text(title));
                }
                items.push(ContentItem::Rule);
                // don't flush - content below this heading belongs in the same block
            }
            Event::End(TagEnd::Paragraph) => {
                if !text_buf.is_empty() {
                    let line = text_buf.drain(..).collect::<String>();
                    if in_blockquote {
                        items.push(ContentItem::Text(format!("│ {}", line)));
                    } else {
                        items.push(ContentItem::Text(line));
                    }
                }
                // paragraphs accumulate into the current section
            }
            Event::End(TagEnd::Item) => {
                let line = text_buf.drain(..).collect::<String>();
                if !line.is_empty() {
                    items.push(ContentItem::Text(format!("▪ {}", line)));
                }
            }
            Event::End(TagEnd::List(_)) => {
                // list ends, but stays in current section
            }
            Event::End(TagEnd::BlockQuote(_)) => {
                in_blockquote = false;
            }
            Event::End(TagEnd::CodeBlock) => {
                let code = text_buf.drain(..).collect::<String>();
                for line in code.lines() {
                    items.push(ContentItem::Text(line.to_string()));
                }
            }
            Event::End(_) => {}

            Event::Rule => {
                // markdown --- is an explicit section break
                flush_block(&mut blocks, &mut items);
                items.push(ContentItem::Rule);
                flush_block(&mut blocks, &mut items);
            }

            _ => {}
        }
    }

    // flush remaining
    if !text_buf.is_empty() {
        items.push(ContentItem::Text(text_buf));
    }
    flush_block(&mut blocks, &mut items);
    blocks
}

fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::Rgb { r, g, b }
}

fn hsl_to_rgb(h: f64, s: f64, l: f64) -> Color {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;
    let (r1, g1, b1) = match h as u32 {
        0..=59 => (c, x, 0.0),
        60..=119 => (x, c, 0.0),
        120..=179 => (0.0, c, x),
        180..=239 => (0.0, x, c),
        240..=299 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    Color::Rgb {
        r: ((r1 + m) * 255.0) as u8,
        g: ((g1 + m) * 255.0) as u8,
        b: ((b1 + m) * 255.0) as u8,
    }
}

/// Named themes. Each is [background, primary, secondary, accent, text].
fn named_theme(name: &str) -> Option<[Color; 5]> {
    Some(match name {
        // --- warm ---
        "ember" => [
            rgb(25, 8, 2),       // near-black warm
            rgb(204, 85, 0),     // burnt orange
            rgb(140, 40, 50),    // dried blood
            rgb(255, 160, 50),   // amber glow
            rgb(240, 220, 200),  // warm white
        ],
        "terracotta" => [
            rgb(30, 15, 10),     // dark earth
            rgb(180, 100, 60),   // clay
            rgb(90, 130, 80),    // sage
            rgb(220, 170, 100),  // sand
            rgb(235, 225, 210),  // parchment
        ],
        "sakura" => [
            rgb(20, 10, 18),     // dark plum
            rgb(200, 120, 160),  // cherry blossom
            rgb(100, 80, 90),    // bark
            rgb(255, 180, 200),  // petal pink
            rgb(240, 235, 240),  // soft white
        ],

        // --- cool ---
        "arctic" => [
            rgb(5, 10, 20),      // deep night
            rgb(100, 160, 220),  // ice blue
            rgb(60, 80, 120),    // steel
            rgb(180, 230, 255),  // frost
            rgb(230, 240, 250),  // snow
        ],
        "deep" => [
            rgb(2, 5, 15),       // abyss
            rgb(30, 80, 160),    // ocean
            rgb(80, 40, 120),    // purple depth
            rgb(50, 200, 180),   // bioluminescent
            rgb(200, 220, 240),  // foam
        ],
        "moss" => [
            rgb(8, 15, 5),       // forest floor
            rgb(80, 140, 60),    // moss green
            rgb(50, 80, 40),     // dark fern
            rgb(160, 200, 80),   // lichen
            rgb(210, 230, 200),  // pale green
        ],

        // --- monochrome ---
        "bone" => [
            rgb(15, 14, 12),     // charcoal
            rgb(180, 170, 155),  // bone
            rgb(120, 115, 105),  // stone
            rgb(220, 210, 190),  // ivory
            rgb(240, 235, 225),  // cream
        ],
        "silver" => [
            rgb(10, 10, 12),     // gunmetal
            rgb(140, 145, 160),  // silver
            rgb(80, 85, 95),     // pewter
            rgb(200, 205, 220),  // bright silver
            rgb(235, 235, 240),  // platinum
        ],

        // --- vivid ---
        "neon" => [
            rgb(5, 0, 10),       // void
            rgb(0, 255, 128),    // neon green
            rgb(255, 0, 128),    // hot pink
            rgb(0, 200, 255),    // cyan
            rgb(255, 255, 255),  // pure white
        ],
        "nerv" => [
            rgb(10, 2, 15),      // eva purple-black
            rgb(200, 50, 20),    // nerv red
            rgb(100, 60, 160),   // eva purple
            rgb(255, 180, 0),    // warning orange
            rgb(220, 220, 230),  // terminal gray
        ],
        "mitla" => [
            rgb(20, 12, 5),      // obsidian earth
            rgb(190, 140, 60),   // gold stone
            rgb(140, 60, 40),    // red clay
            rgb(100, 170, 130),  // jade
            rgb(230, 220, 200),  // limestone
        ],

        _ => return None,
    })
}

/// Seed-deterministic palette: rotate hue based on seed, derive harmonious colors.
/// Returns [background, primary, secondary, accent, text].
fn make_palette(seed: u64) -> [Color; 5] {
    let base_hue = (seed % 360) as f64;
    [
        hsl_to_rgb(base_hue, 0.3, 0.15),
        hsl_to_rgb((base_hue + 30.0) % 360.0, 0.6, 0.55),
        hsl_to_rgb((base_hue + 180.0) % 360.0, 0.5, 0.45),
        hsl_to_rgb((base_hue + 60.0) % 360.0, 0.7, 0.65),
        rgb(220, 220, 220),
    ]
}

/// Draw a conifer/pine tree: layered triangular tiers narrowing toward the top.
/// Each tier is a triangle of needle glyphs with ╱╲ edges.
fn draw_pine(grid: &mut Grid, root_x: usize, root_y: usize, tiers: usize, base_width: usize, color: Color) {
    let set = |grid: &mut Grid, x: usize, y: usize, ch: char, fg: Color| {
        if y < grid.len() && x < grid[0].len() {
            grid[y][x] = Cell::new(ch, fg);
        }
    };

    // trunk: 3 cells tall
    let trunk_top = root_y.saturating_sub(2);
    for y in trunk_top..=root_y {
        set(grid, root_x, y, '│', darken(color, 60));
    }

    // tiers from bottom to top, each overlapping the one below
    let mut tier_bottom = trunk_top;
    for t in 0..tiers {
        let tier_height = (base_width / 2).saturating_sub(t).max(2);
        let tier_width = base_width.saturating_sub(t * 2).max(3);
        let half = tier_width / 2;
        let tier_top = tier_bottom.saturating_sub(tier_height);

        for row in tier_top..tier_bottom {
            let progress = tier_bottom - row; // 1 at bottom edge, tier_height at top
            let row_half = (half * progress) / tier_height;
            let left = root_x.saturating_sub(row_half);
            let right = (root_x + row_half).min(grid[0].len() - 1);

            // edges
            if left < root_x {
                set(grid, left, row, '╱', color);
            }
            if right > root_x {
                set(grid, right, row, '╲', color);
            }
            // fill
            let needles = ['▪', '◆', '●', '▫'];
            for x in (left + 1)..right {
                let needle = needles[(x + row) % needles.len()];
                let nc = if (x + row) % 3 == 0 { lighten(color, 20) } else { color };
                set(grid, x, row, needle, nc);
            }
        }
        // tip
        if tier_top > 0 && t == tiers - 1 {
            set(grid, root_x, tier_top.saturating_sub(1), '▲', lighten(color, 30));
        }

        tier_bottom = tier_top + 1; // next tier starts overlapping by 1 row
    }
}

/// Draw a weeping willow: binary splits where branch tips droop downward.
fn draw_willow(grid: &mut Grid, root_x: usize, root_y: usize, canopy_y: usize, spread: usize, color: Color) {
    let set = |grid: &mut Grid, x: usize, y: usize, ch: char, fg: Color| {
        if y < grid.len() && x < grid[0].len() && grid[y][x].ch == ' ' {
            grid[y][x] = Cell::new(ch, fg);
        }
    };

    let height = root_y.saturating_sub(canopy_y);
    if height < 4 { return; }
    let first_split = root_y - (height / 3).max(2);

    // trunk
    for y in first_split..root_y {
        set(grid, root_x, y, '│', darken(color, 40));
    }

    // branch queue: (x, top_y, bottom_y, depth)
    let mut queue: Vec<(usize, usize, usize, usize)> = vec![(root_x, canopy_y, first_split, 0)];
    let max_depth = 3;
    let mut tips: Vec<(usize, usize, Color)> = Vec::new();

    while let Some((x, top, bottom, depth)) = queue.pop() {
        let branch_color = lighten(color, (depth as u8) * 15);

        if depth >= max_depth || bottom <= top + 1 {
            // terminal branch: mark as droop point
            for y in top..bottom {
                set(grid, x, y, '│', branch_color);
            }
            tips.push((x, bottom, lighten(branch_color, 20)));
            continue;
        }

        let split_y = top + (bottom - top) / 2;
        for y in (split_y + 1)..bottom {
            set(grid, x, y, '│', branch_color);
        }

        let arm_len = (spread >> depth).max(2);
        let left_x = x.saturating_sub(arm_len);
        let right_x = (x + arm_len).min(grid[0].len() - 1);

        // horizontal branch
        set(grid, x, split_y, '┼', branch_color);
        if left_x < x {
            set(grid, left_x, split_y, '╭', branch_color);
            for ax in (left_x + 1)..x {
                set(grid, ax, split_y, '─', branch_color);
            }
        }
        if right_x > x {
            set(grid, right_x, split_y, '╮', branch_color);
            for ax in (x + 1)..right_x {
                set(grid, ax, split_y, '─', branch_color);
            }
        }

        queue.push((left_x, top, split_y, depth + 1));
        queue.push((right_x, top, split_y, depth + 1));
    }

    // droop: from each tip, hang strands downward
    let droop_chars = ['╎', '┊', '╏', '┆', '│'];
    for (tx, ty, tc) in &tips {
        let droop_len = (root_y - ty).min(8);
        for d in 0..droop_len {
            let dy = ty + d;
            let ch = droop_chars[d % droop_chars.len()];
            set(grid, *tx, dy, ch, lighten(*tc, (d as u8) * 8));
            // side strands
            if d > 1 && d % 2 == 0 {
                if *tx > 0 {
                    set(grid, tx - 1, dy, '╲', lighten(*tc, (d as u8) * 10));
                }
                if tx + 1 < grid[0].len() {
                    set(grid, tx + 1, dy, '╱', lighten(*tc, (d as u8) * 10));
                }
            }
        }
    }
}

/// Draw a palm tree: tall trunk with fronds radiating from the crown.
fn draw_palm(grid: &mut Grid, root_x: usize, root_y: usize, trunk_height: usize, color: Color, rng: &mut StdRng) {
    let set = |grid: &mut Grid, x: usize, y: usize, ch: char, fg: Color| {
        if y < grid.len() && x < grid[0].len() {
            grid[y][x] = Cell::new(ch, fg);
        }
    };

    let trunk_color = darken(color, 50);
    let crown_y = root_y.saturating_sub(trunk_height);

    // trunk with slight texture
    let trunk_chars = ['┃', '┃', '╿', '┃'];
    for y in crown_y..root_y {
        let ch = trunk_chars[(y + root_x) % trunk_chars.len()];
        set(grid, root_x, y, ch, trunk_color);
    }

    // fronds: 6-8 radiating lines from crown point
    let frond_color = lighten(color, 10);
    let frond_defs: &[(i32, i32, &[char])] = &[
        // (dx_per_step, dy_per_step, glyphs along the frond)
        (-2, -1, &['╲', '─', '╲', '╲', '╷']),  // upper-left
        (-2,  0, &['─', '─', '╲', '╲', '╷']),  // left
        (-1,  1, &['╲', '╲', '╲', '╷', '╵']),  // lower-left, drooping
        ( 2, -1, &['╱', '─', '╱', '╱', '╷']),  // upper-right
        ( 2,  0, &['─', '─', '╱', '╱', '╷']),  // right
        ( 1,  1, &['╱', '╱', '╱', '╷', '╵']),  // lower-right, drooping
        ( 0, -1, &['│', '│', '╷', '·']),        // straight up
        (-1, -1, &['╲', '╲', '╲', '╷']),        // diagonal upper-left
        ( 1, -1, &['╱', '╱', '╱', '╷']),        // diagonal upper-right
    ];

    for (dx, dy, glyphs) in frond_defs {
        let mut fx = root_x as i32;
        let mut fy = crown_y as i32;
        for (step, &ch) in glyphs.iter().enumerate() {
            fx += dx;
            fy += dy;
            if fx >= 0 && fy >= 0 && (fy as usize) < grid.len() && (fx as usize) < grid[0].len() {
                let fc = if step < 2 { frond_color } else { lighten(frond_color, (step as u8) * 12) };
                set(grid, fx as usize, fy as usize, ch, fc);
            }
        }
    }

    // coconuts: 2-3 near crown
    let coconut_positions: &[(i32, i32)] = &[(-1, 0), (1, 0), (0, 1)];
    let num_coconuts = rng.random_range(2..=3);
    for i in 0..num_coconuts {
        let (cdx, cdy) = coconut_positions[i];
        let cx = (root_x as i32 + cdx) as usize;
        let cy = (crown_y as i32 + cdy) as usize;
        if cy < grid.len() && cx < grid[0].len() {
            set(grid, cx, cy, '●', darken(color, 30));
        }
    }
}

/// Draw a fruit at position (x, y). Styles:
/// 0 = apple, 1 = cherry pair, 2 = citrus, 3 = berry cluster, 4 = pear
fn draw_fruit(grid: &mut Grid, cx: usize, cy: usize, style: usize, color: Color) {
    let patterns: &[&[(i32, i32, char)]] = &[
        // apple: round body with stem and leaf
        &[(0, 0, '●'), (0, -1, '╿'), (1, -1, '╌')],
        // cherry pair: two on stems from a shared branch
        &[(-1, 0, '●'), (1, 0, '●'), (-1, -1, '╱'), (1, -1, '╲'), (0, -1, '┬')],
        // citrus: segmented circle
        &[(0, 0, '◉'), (0, -1, '╷')],
        // berry cluster: tight group
        &[(0, 0, '●'), (-1, 0, '•'), (1, 0, '•'), (0, -1, '•'), (0, 1, '•')],
        // pear: narrow top, wide bottom
        &[(0, 0, '◆'), (0, 1, '●'), (0, -1, '╿'), (1, -1, '╌')],
    ];

    let pattern = patterns[style % patterns.len()];
    for &(dx, dy, ch) in pattern {
        let x = cx as i32 + dx;
        let y = cy as i32 + dy;
        if x >= 0 && y >= 0 && (y as usize) < grid.len() && (x as usize) < grid[0].len() {
            let c = if dx == 0 && dy == 0 { lighten(color, 30) } else { color };
            grid[y as usize][x as usize] = Cell::new(ch, c);
        }
    }
}

/// Grow a GRIS-style tree upward from (root_x, root_y).
fn grow_tree(grid: &mut Grid, root_x: usize, root_y: usize, canopy_y: usize, spread: usize, color: Color, _rng: &mut StdRng) {
    let set = |grid: &mut Grid, x: usize, y: usize, ch: char, fg: Color| {
        if y < grid.len() && x < grid[0].len() {
            grid[y][x] = Cell::new(ch, fg);
        }
    };

    let height = root_y - canopy_y;
    let first_split = root_y - (height / 3).max(2);

    // trunk
    for y in first_split..root_y {
        set(grid, root_x, y, '│', color);
    }

    let mut queue: Vec<(usize, usize, usize, usize)> = vec![(root_x, canopy_y, first_split, 0)];
    let max_depth = 4;

    while let Some((x, top, bottom, depth)) = queue.pop() {
        // lighten color at deeper branches
        let branch_color = match depth {
            0 => color,
            1 => lighten(color, 20),
            2 => lighten(color, 40),
            _ => lighten(color, 60),
        };

        if depth >= max_depth || bottom <= top + 1 {
            for y in top..bottom {
                if y < grid.len() && x < grid[0].len() && grid[y][x].ch == ' ' {
                    set(grid, x, y, '│', branch_color);
                }
            }
            if top < grid.len() && x < grid[0].len() {
                set(grid, x, top, '╷', lighten(branch_color, 30));
            }
            continue;
        }

        let split_y = top + (bottom - top) / 2;
        for y in split_y + 1..bottom {
            if y < grid.len() && x < grid[0].len() && grid[y][x].ch == ' ' {
                set(grid, x, y, '│', branch_color);
            }
        }

        let arm_len = (spread >> depth).max(2);
        let left_x = x.saturating_sub(arm_len);
        let right_x = (x + arm_len).min(grid[0].len() - 1);

        if split_y < grid.len() && x < grid[0].len() {
            set(grid, x, split_y, '┤', branch_color);
        }

        if left_x < x {
            set(grid, left_x, split_y, '╭', branch_color);
            for ax in left_x + 1..x {
                if ax < grid[0].len() {
                    set(grid, ax, split_y, '─', branch_color);
                }
            }
        }

        if right_x > x {
            set(grid, x, split_y, '┼', branch_color);
            for ax in x + 1..right_x {
                if ax < grid[0].len() {
                    set(grid, ax, split_y, '─', branch_color);
                }
            }
            set(grid, right_x, split_y, '╮', branch_color);
        }

        queue.push((left_x, top, split_y, depth + 1));
        queue.push((right_x, top, split_y, depth + 1));
    }
}

fn lighten(color: Color, amount: u8) -> Color {
    match color {
        Color::Rgb { r, g, b } => Color::Rgb {
            r: r.saturating_add(amount),
            g: g.saturating_add(amount),
            b: b.saturating_add(amount),
        },
        other => other,
    }
}

fn darken(color: Color, amount: u8) -> Color {
    match color {
        Color::Rgb { r, g, b } => Color::Rgb {
            r: r.saturating_sub(amount),
            g: g.saturating_sub(amount),
            b: b.saturating_sub(amount),
        },
        other => other,
    }
}

/// Cardinal directions for the turtle walker.
#[derive(Clone, Copy)]
enum Dir { Right, Down, Left, Up }

impl Dir {
    fn turn_right(self) -> Dir {
        match self {
            Dir::Right => Dir::Down,
            Dir::Down => Dir::Left,
            Dir::Left => Dir::Up,
            Dir::Up => Dir::Right,
        }
    }

    fn advance(self, x: i32, y: i32) -> (i32, i32) {
        match self {
            Dir::Right => (x + 1, y),
            Dir::Down => (x, y + 1),
            Dir::Left => (x - 1, y),
            Dir::Up => (x, y - 1),
        }
    }

    fn horizontal_glyph(self) -> char {
        match self {
            Dir::Right | Dir::Left => '─',
            Dir::Down | Dir::Up => '│',
        }
    }
}

/// Draw a single stepped fret (xicalcoliuhqui) spiral.
fn draw_stepped_fret(grid: &mut Grid, start_x: i32, start_y: i32, steps: usize, initial_dir: Dir, color: Color) {
    if steps == 0 { return; }

    let set = |grid: &mut Grid, x: i32, y: i32, ch: char| {
        if x >= 0 && y >= 0 && (y as usize) < grid.len() && (x as usize) < grid[0].len() {
            grid[y as usize][x as usize] = Cell::new(ch, color);
        }
    };

    let mut arms: Vec<usize> = Vec::new();
    for i in (1..=steps).rev() {
        arms.push(i);
        arms.push(i);
    }

    let mut x = start_x;
    let mut y = start_y;
    let mut dir = initial_dir;

    for (arm_idx, &arm_length) in arms.iter().enumerate() {
        for j in 0..arm_length {
            set(grid, x, y, dir.horizontal_glyph());
            if j < arm_length - 1 {
                let (nx, ny) = dir.advance(x, y);
                x = nx;
                y = ny;
            }
        }

        let corner = match dir {
            Dir::Right => '┐',
            Dir::Down  => '┘',
            Dir::Left  => '└',
            Dir::Up    => '┌',
        };
        set(grid, x, y, corner);

        dir = dir.turn_right();
        if arm_idx < arms.len() - 1 {
            let (nx, ny) = dir.advance(x, y);
            x = nx;
            y = ny;
        }
    }
}

/// Draw a stepped fret border band along an edge of a rectangular region.
fn draw_fret_border(grid: &mut Grid, x0: usize, y0: usize, w: usize, h: usize, band_depth: usize, edge: usize, color: Color) {
    let (start_x, start_y, dir, count) = match edge {
        0 => (x0 as i32, y0 as i32, Dir::Right, w),
        1 => ((x0 + w - 1) as i32, y0 as i32, Dir::Down, h),
        2 => ((x0 + w - 1) as i32, (y0 + h - 1) as i32, Dir::Left, w),
        _ => (x0 as i32, (y0 + h - 1) as i32, Dir::Up, h),
    };

    let unit_spacing = band_depth * 2 + 1;
    let mut pos = 0;
    while pos + band_depth <= count {
        let (sx, sy) = match edge {
            0 => (start_x + pos as i32, start_y),
            1 => (start_x, start_y + pos as i32),
            2 => (start_x - pos as i32, start_y),
            _ => (start_x, start_y - pos as i32),
        };
        draw_stepped_fret(grid, sx, sy, band_depth, dir, color);
        pos += unit_spacing;
    }
}

// ── Box borders ─────────────────────────────────────────────────────

enum BorderStyle { Light, Heavy, Double, Rounded, Fret }

fn border_glyphs(style: &BorderStyle) -> (char, char, char, char, char, char) {
    match style {
        BorderStyle::Light   => ('┌', '┐', '└', '┘', '─', '│'),
        BorderStyle::Heavy   => ('┏', '┓', '┗', '┛', '━', '┃'),
        BorderStyle::Double  => ('╔', '╗', '╚', '╝', '═', '║'),
        BorderStyle::Rounded => ('╭', '╮', '╰', '╯', '─', '│'),
        BorderStyle::Fret    => unreachable!(),
    }
}

/// Draw a box border around rect, preserving existing bg colors.
/// No-op if rect < 3x3. Fret falls back to Light if rect < 8x6.
fn draw_box_border(grid: &mut Grid, rect: &Rect, style: &BorderStyle, color: Color) {
    if rect.w < 3 || rect.h < 3 { return; }

    if matches!(style, BorderStyle::Fret) {
        if rect.w < 8 || rect.h < 6 {
            return draw_box_border(grid, rect, &BorderStyle::Light, color);
        }
        let band = 2;
        for edge in 0..4 {
            draw_fret_border(grid, rect.x, rect.y, rect.w, rect.h, band, edge, color);
        }
        return;
    }

    let (tl, tr, bl, br, horiz, vert) = border_glyphs(style);
    let x0 = rect.x;
    let y0 = rect.y;
    let x1 = rect.x + rect.w - 1;
    let y1 = rect.y + rect.h - 1;

    let set = |grid: &mut Grid, x: usize, y: usize, ch: char, fg: Color| {
        if y < grid.len() && x < grid[0].len() {
            let bg = grid[y][x].bg;
            grid[y][x] = Cell::with_bg(ch, fg, bg);
        }
    };

    set(grid, x0, y0, tl, color);
    set(grid, x1, y0, tr, color);
    set(grid, x0, y1, bl, color);
    set(grid, x1, y1, br, color);

    for x in (x0 + 1)..x1 {
        set(grid, x, y0, horiz, color);
        set(grid, x, y1, horiz, color);
    }
    for y in (y0 + 1)..y1 {
        set(grid, x0, y, vert, color);
        set(grid, x1, y, vert, color);
    }
}

/// Draw decorative corner embellishments on a bordered rect.
/// Overlays triangle/block motifs at each corner, preserving bg.
fn draw_corner_embellishments(grid: &mut Grid, rect: &Rect, style: usize, color: Color) {
    if rect.w < 5 || rect.h < 4 { return; }

    let x0 = rect.x;
    let y0 = rect.y;
    let x1 = rect.x + rect.w - 1;
    let y1 = rect.y + rect.h - 1;

    let set = |grid: &mut Grid, x: usize, y: usize, ch: char| {
        if y < grid.len() && x < grid[0].len() {
            let bg = grid[y][x].bg;
            grid[y][x] = Cell::with_bg(ch, color, bg);
        }
    };

    match style % 6 {
        0 => {
            // Fan: quarter-triangle + half-block extensions
            set(grid, x0, y0, '◤'); set(grid, x0 + 1, y0, '▀'); set(grid, x0, y0 + 1, '▌');
            set(grid, x1, y0, '◥'); set(grid, x1 - 1, y0, '▀'); set(grid, x1, y0 + 1, '▐');
            set(grid, x0, y1, '◣'); set(grid, x0 + 1, y1, '▄'); set(grid, x0, y1 - 1, '▌');
            set(grid, x1, y1, '◢'); set(grid, x1 - 1, y1, '▄'); set(grid, x1, y1 - 1, '▐');
        }
        1 => {
            // Double triangle: two quarter-triangles per corner
            set(grid, x0, y0, '◤'); set(grid, x0 + 1, y0, '◥'); set(grid, x0, y0 + 1, '◣');
            set(grid, x1, y0, '◥'); set(grid, x1 - 1, y0, '◤'); set(grid, x1, y0 + 1, '◢');
            set(grid, x0, y1, '◣'); set(grid, x0 + 1, y1, '◢'); set(grid, x0, y1 - 1, '◤');
            set(grid, x1, y1, '◢'); set(grid, x1 - 1, y1, '◣'); set(grid, x1, y1 - 1, '◥');
        }
        2 => {
            // Block: three-quadrant block chars
            set(grid, x0, y0, '▛'); set(grid, x0 + 1, y0, '▀'); set(grid, x0, y0 + 1, '▌');
            set(grid, x1, y0, '▜'); set(grid, x1 - 1, y0, '▀'); set(grid, x1, y0 + 1, '▐');
            set(grid, x0, y1, '▙'); set(grid, x0 + 1, y1, '▄'); set(grid, x0, y1 - 1, '▌');
            set(grid, x1, y1, '▟'); set(grid, x1 - 1, y1, '▄'); set(grid, x1, y1 - 1, '▐');
        }
        3 => {
            // Arrow: triangle + directional pointer
            set(grid, x0, y0, '◤'); set(grid, x0 + 1, y0, '▶'); set(grid, x0, y0 + 1, '▼');
            set(grid, x1, y0, '◥'); set(grid, x1 - 1, y0, '◀'); set(grid, x1, y0 + 1, '▼');
            set(grid, x0, y1, '◣'); set(grid, x0 + 1, y1, '▶'); set(grid, x0, y1 - 1, '▲');
            set(grid, x1, y1, '◢'); set(grid, x1 - 1, y1, '◀'); set(grid, x1, y1 - 1, '▲');
        }
        4 if rect.w >= 7 && rect.h >= 5 => {
            // Layered: 3-deep triangle cascade
            set(grid, x0, y0, '◤'); set(grid, x0+1, y0, '◥'); set(grid, x0+2, y0, '▀');
            set(grid, x0, y0+1, '◣'); set(grid, x0+1, y0+1, '◤'); set(grid, x0, y0+2, '▌');

            set(grid, x1, y0, '◥'); set(grid, x1-1, y0, '◤'); set(grid, x1-2, y0, '▀');
            set(grid, x1, y0+1, '◢'); set(grid, x1-1, y0+1, '◥'); set(grid, x1, y0+2, '▐');

            set(grid, x0, y1, '◣'); set(grid, x0+1, y1, '◢'); set(grid, x0+2, y1, '▄');
            set(grid, x0, y1-1, '◤'); set(grid, x0+1, y1-1, '◣'); set(grid, x0, y1-2, '▌');

            set(grid, x1, y1, '◢'); set(grid, x1-1, y1, '◣'); set(grid, x1-2, y1, '▄');
            set(grid, x1, y1-1, '◥'); set(grid, x1-1, y1-1, '◢'); set(grid, x1, y1-2, '▐');
        }
        _ => {
            // Bracket: half-bracket corners with side accents
            set(grid, x0, y0, '⌜'); set(grid, x0 + 1, y0, '▀'); set(grid, x0, y0 + 1, '▏');
            set(grid, x1, y0, '⌝'); set(grid, x1 - 1, y0, '▀'); set(grid, x1, y0 + 1, '▕');
            set(grid, x0, y1, '⌞'); set(grid, x0 + 1, y1, '▄'); set(grid, x0, y1 - 1, '▏');
            set(grid, x1, y1, '⌟'); set(grid, x1 - 1, y1, '▄'); set(grid, x1, y1 - 1, '▕');
        }
    }
}

// ── Tile pattern system ─────────────────────────────────────────────

/// A tile is a small rectangular char grid that repeats to fill any area.
///
/// Stagger controls:
/// - `row_offset`: how many columns each tile-row shifts rightward
/// - `stagger_rhythm`: how many tile-rows before the offset resets
///   (1 = every row staggers, 2 = every other row, etc.)
///
/// Example with period_x=8, row_offset=4, stagger_rhythm=1:
///   row 0: phase 0        (tile-row 0, offset = 0*4 = 0)
///   row 4: phase 4        (tile-row 1, offset = 1*4 = 4)
///   row 8: phase 0        (tile-row 2, offset = 2*4 = 8 mod 8 = 0)
///
/// With stagger_rhythm=2 and row_offset=4:
///   tile-rows 0,1: phase 0  (group 0)
///   tile-rows 2,3: phase 4  (group 1)
///   tile-rows 4,5: phase 0  (group 2)
struct TilePattern {
    cells: Vec<Vec<(char, u8)>>,  // [y][x] -> (char, color_index: 0=primary, 1=secondary)
    row_offset: usize,            // x-shift per stagger group (0 = no stagger)
    stagger_rhythm: usize,        // tile-rows per stagger group (1 = every row, 2 = pairs, etc.)
}

impl TilePattern {
    fn period_x(&self) -> usize { self.cells[0].len() }
    fn period_y(&self) -> usize { self.cells.len() }

    fn at(&self, x: usize, y: usize) -> (char, u8) {
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
enum TileVariant {
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

const TILE_VARIANT_COUNT: usize = 10;

fn tile_variant_from_index(i: usize) -> TileVariant {
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
fn make_tile(variant: TileVariant) -> TilePattern {
    // Helper: parse a visual template into cells.
    // Each char pair "c0" or "c1" means (char, color_index).
    // We use a simpler approach: Vec of string slices for chars, separate color map.
    match variant {
        TileVariant::Asanoha => {
            // Hemp leaf: 8x4, hex stagger (offset=4)
            // Star pattern from ╱╲ diagonals meeting at vertices
            let g = vec![
                vec![('╲', 0), ('╱', 1), ('╲', 0), ('╱', 1), ('╲', 1), ('╱', 0), ('╲', 1), ('╱', 0)],
                vec![('╱', 0), ('╲', 1), ('─', 0), ('─', 0), ('─', 1), ('─', 1), ('╱', 1), ('╲', 0)],
                vec![('╲', 1), ('╱', 0), ('╲', 1), ('╱', 0), ('╲', 0), ('╱', 1), ('╲', 0), ('╱', 1)],
                vec![('╱', 1), ('╲', 0), ('─', 1), ('─', 1), ('─', 0), ('─', 0), ('╱', 0), ('╲', 1)],
            ];
            TilePattern { cells: g, row_offset: 4, stagger_rhythm: 1 }
        }
        TileVariant::Seigaiha => {
            // Blue ocean waves: overlapping arcs, 8x4, hex stagger
            let g = vec![
                vec![('╰', 0), ('─', 0), ('─', 0), ('╯', 0), ('╰', 1), ('─', 1), ('─', 1), ('╯', 1)],
                vec![(' ', 0), ('╭', 0), ('╮', 0), (' ', 0), (' ', 1), ('╭', 1), ('╮', 1), (' ', 1)],
                vec![(' ', 0), ('│', 0), ('│', 0), (' ', 0), (' ', 1), ('│', 1), ('│', 1), (' ', 1)],
                vec![('╭', 0), ('╯', 0), ('╰', 0), ('╮', 0), ('╭', 1), ('╯', 1), ('╰', 1), ('╮', 1)],
            ];
            TilePattern { cells: g, row_offset: 4, stagger_rhythm: 1 }
        }
        TileVariant::Shippo => {
            // Seven treasures: interlocking vesica shapes, 6x4, hex stagger
            let g = vec![
                vec![('╲', 0), ('╭', 1), ('─', 1), ('╮', 1), ('╱', 0), (' ', 0)],
                vec![(' ', 0), ('│', 1), (' ', 0), ('│', 1), (' ', 0), (' ', 0)],
                vec![('╱', 0), ('╰', 1), ('─', 1), ('╯', 1), ('╲', 0), (' ', 0)],
                vec![(' ', 0), (' ', 0), ('╲', 0), ('╱', 0), (' ', 0), (' ', 0)],
            ];
            TilePattern { cells: g, row_offset: 3, stagger_rhythm: 1 }
        }
        TileVariant::BishamonKikko => {
            // Tortoiseshell hexagons: 6x4, hex stagger
            let g = vec![
                vec![('╱', 0), ('─', 0), ('─', 0), ('╲', 0), (' ', 1), (' ', 1)],
                vec![('│', 0), (' ', 0), (' ', 0), ('│', 0), (' ', 1), (' ', 1)],
                vec![('╲', 0), ('─', 0), ('─', 0), ('╱', 0), (' ', 1), (' ', 1)],
                vec![(' ', 0), (' ', 0), (' ', 0), (' ', 0), (' ', 1), (' ', 1)],
            ];
            TilePattern { cells: g, row_offset: 3, stagger_rhythm: 1 }
        }
        TileVariant::Yabane => {
            // Arrow feather chevrons: 6x4, no stagger
            let g = vec![
                vec![('╱', 0), ('╱', 0), ('╱', 0), ('╲', 1), ('╲', 1), ('╲', 1)],
                vec![('╱', 0), ('╱', 0), ('╱', 0), ('╲', 1), ('╲', 1), ('╲', 1)],
                vec![('╲', 1), ('╲', 1), ('╲', 1), ('╱', 0), ('╱', 0), ('╱', 0)],
                vec![('╲', 1), ('╲', 1), ('╲', 1), ('╱', 0), ('╱', 0), ('╱', 0)],
            ];
            TilePattern { cells: g, row_offset: 0, stagger_rhythm: 1 }
        }
        TileVariant::Nowaki => {
            // Autumn storm diagonal grass: 4x6, no stagger
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
            // Cypress fence: tight diagonal crosshatch, 4x4
            let g = vec![
                vec![('╱', 0), ('╳', 1), ('╲', 0), (' ', 0)],
                vec![('╳', 1), ('╲', 0), (' ', 0), ('╱', 0)],
                vec![('╲', 0), (' ', 0), ('╱', 0), ('╳', 1)],
                vec![(' ', 0), ('╱', 0), ('╳', 1), ('╲', 0)],
            ];
            TilePattern { cells: g, row_offset: 0, stagger_rhythm: 1 }
        }
        TileVariant::ShellStitch => {
            // Crochet shell: scalloped arcs, 8x3, hex stagger
            let g = vec![
                vec![('╰', 0), ('─', 0), ('╮', 0), (' ', 0), (' ', 0), ('╭', 1), ('─', 1), ('╯', 1)],
                vec![(' ', 0), (' ', 0), ('│', 0), ('◠', 0), ('◠', 1), ('│', 1), (' ', 1), (' ', 1)],
                vec![('─', 0), ('╮', 0), ('╰', 0), ('─', 0), ('─', 1), ('╯', 1), ('╭', 1), ('─', 1)],
            ];
            TilePattern { cells: g, row_offset: 4, stagger_rhythm: 1 }
        }
        TileVariant::GrannySquare => {
            // Crochet granny square: concentric frames, 6x6
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
            // Overlapping scales: 6x4, hex stagger
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

/// Fill a rect with a tile pattern, pure deterministic baseline.
/// No phase shift, no jitter, no density dropout. What-you-define-is-what-you-get.
fn fill_tile_pure(
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
///
/// Stagger/rhythm overrides in `params` modify the tile's default layout:
/// - `stagger_override >= 0` replaces the tile's `row_offset`
/// - `rhythm_override > 0` replaces the tile's `stagger_rhythm`
///
/// This means the same asanoha pattern can tile as a tight grid (stagger=0),
/// a hex offset (stagger=period/2), or a wide drift (stagger=1, rhythm=3).
fn fill_tile_ex(
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
    for y in rect.y..rect.y + rect.h {
        for x in rect.x..rect.x + rect.w {
            if y >= grid.len() || x >= grid[0].len() { continue; }
            if params.density < 1.0 && rng.random::<f32>() > params.density { continue; }
            let (mut ch, ci) = tile.at(x - rect.x + phase_x, y - rect.y + phase_y);
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
struct NoiseGlyph {
    ch: char,
    ci: u8,       // 0 = primary color, 1 = secondary
    weight: f32,  // relative probability
}

/// Predefined noise palettes.
///
/// Each variant has a `coherence` value (0.0-1.0) controlling run length.
/// At each cell, there's a `coherence` probability of repeating the previous
/// glyph instead of sampling fresh. High coherence = long uninterrupted runs
/// that occasionally break. 0.0 = fully independent (classic random).
#[derive(Clone, Copy)]
enum NoiseVariant {
    Truchet,       // classic ╱╲ 50/50, coherence 0.0
    Higaki,        // ╱╲╳ with gaps, coherence 0.7 (long runs, rare breaks)
    HigakiStatic,  // ╱╲╳ with gaps, coherence 0.0 (per-cell random, the original)
    Grass,         // ╱╲│ with spaces, coherence 0.5
    Static,        // ╱╲─│╳·░, coherence 0.0 (pure random)
    Dot,           // ·∙°, coherence 0.6
}

fn noise_coherence(variant: NoiseVariant) -> f32 {
    match variant {
        NoiseVariant::Truchet => 0.0,
        NoiseVariant::Higaki => 0.7,
        NoiseVariant::HigakiStatic => 0.0,
        NoiseVariant::Grass => 0.5,
        NoiseVariant::Static => 0.0,
        NoiseVariant::Dot => 0.6,
    }
}

fn noise_glyphs(variant: NoiseVariant) -> Vec<NoiseGlyph> {
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

const NOISE_VARIANT_COUNT: usize = 6;

fn noise_variant_from_index(i: usize) -> NoiseVariant {
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

/// Fill a rect with noise. Coherence controls run length:
/// each cell has `coherence` probability of repeating the previous glyph
/// instead of sampling fresh. Scans left-to-right, top-to-bottom, with
/// row starts seeded from the previous row's last glyph.
fn fill_noise(
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
fn fill_truchet(grid: &mut Grid, width: usize, height: usize, color: Color, rng: &mut StdRng) {
    let rect = Rect { x: 0, y: 0, w: width, h: height };
    fill_noise(grid, &rect, NoiseVariant::Truchet, color, color, rng);
}

// ── Line art fills ──────────────────────────────────────────────────

/// Crosshatch: deterministic diagonal tiling. Denser than random Truchet.
fn draw_crosshatch(grid: &mut Grid, rect: &Rect, color: Color, color2: Color) {
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
fn draw_guilloche(grid: &mut Grid, rect: &Rect, color: Color, color2: Color) {
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
fn draw_weave(grid: &mut Grid, rect: &Rect, color: Color, color2: Color) {
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
fn draw_zigzag(grid: &mut Grid, rect: &Rect, color: Color, color2: Color) {
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
fn draw_diamond_lattice(grid: &mut Grid, rect: &Rect, color: Color, color2: Color) {
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

fn pick_border_style(rng: &mut StdRng, w: usize, h: usize) -> BorderStyle {
    let area = w * h;
    if area < 100 {
        if rng.random_range(0..2) == 0 { BorderStyle::Light } else { BorderStyle::Rounded }
    } else {
        match rng.random_range(0..5) {
            0 => BorderStyle::Light,
            1 => BorderStyle::Heavy,
            2 => BorderStyle::Double,
            3 => BorderStyle::Rounded,
            _ => if w >= 8 && h >= 6 { BorderStyle::Fret } else { BorderStyle::Light },
        }
    }
}

fn border_inset(style: &BorderStyle) -> usize {
    match style {
        BorderStyle::Fret => 3,
        _ => 1,
    }
}

/// Aztec diamond domino tiling via domino shuffling.
/// Correct implementation: DELETE (on old grid) → SLIDE (into new grid) → CREATE (fill empty 2x2s).
/// Reference: Elkies-Kuperberg-Larsen-Propp (1992), pywonderland implementation.
fn draw_aztec_diamond(grid: &mut Grid, center_x: usize, center_y: usize, order: usize, palette: &[Color; 5], rng: &mut StdRng) {
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum D { N, S, E, W, Empty }

    let in_diamond = |r: usize, c: usize, ord: usize| -> bool {
        let size = 2 * ord;
        if r >= size || c >= size { return false; }
        let rr = (2 * r + 1) as i32 - size as i32;
        let cc = (2 * c + 1) as i32 - size as i32;
        rr.abs() + cc.abs() <= size as i32
    };

    // Order 1: 2x2 grid, one random filling
    let mut state: Vec<Vec<D>> = vec![vec![D::Empty; 2]; 2];
    if rng.random_range(0..2) == 0 {
        // two vertical dominoes side by side
        state[0][0] = D::W; state[0][1] = D::E;
        state[1][0] = D::W; state[1][1] = D::E;
    } else {
        // two horizontal dominoes stacked
        state[0][0] = D::N; state[0][1] = D::N;
        state[1][0] = D::S; state[1][1] = D::S;
    }

    for k in 2..=order {
        let old_size = 2 * (k - 1);
        let new_size = 2 * k;

        // --- DELETE: remove "bad" 2x2 blocks (facing-each-other pairs) ---
        for r in 0..old_size - 1 {
            for c in 0..old_size - 1 {
                let tl = state[r][c];
                let tr = state[r][c + 1];
                let bl = state[r + 1][c];
                let br = state[r + 1][c + 1];
                // Two horizontal dominoes facing each other: top=S (down), bottom=N (up)
                if tl == D::S && tr == D::S && bl == D::N && br == D::N {
                    state[r][c] = D::Empty;     state[r][c + 1] = D::Empty;
                    state[r + 1][c] = D::Empty; state[r + 1][c + 1] = D::Empty;
                }
                // Two vertical dominoes facing each other horizontally: [E,W,E,W]
                if tl == D::E && tr == D::W && bl == D::E && br == D::W {
                    state[r][c] = D::Empty;     state[r][c + 1] = D::Empty;
                    state[r + 1][c] = D::Empty; state[r + 1][c + 1] = D::Empty;
                }
            }
        }

        // --- SLIDE: move each cell one step in its labeled direction ---
        // Embedding: old (r,c) -> new (r+1, c+1), then slide by direction.
        let mut ns: Vec<Vec<D>> = vec![vec![D::Empty; new_size]; new_size];
        for r in 0..old_size {
            for c in 0..old_size {
                let d = state[r][c];
                if d == D::Empty { continue; }
                let (nr, nc) = match d {
                    D::N => (r,     c + 1), // embed (r+1,c+1), slide up (-1 row)
                    D::S => (r + 2, c + 1), // embed (r+1,c+1), slide down (+1 row)
                    D::W => (r + 1, c),     // embed (r+1,c+1), slide left (-1 col)
                    D::E => (r + 1, c + 2), // embed (r+1,c+1), slide right (+1 col)
                    D::Empty => unreachable!(),
                };
                if nr < new_size && nc < new_size && in_diamond(nr, nc, k) {
                    ns[nr][nc] = d;
                }
            }
        }

        // --- CREATE: fill empty 2x2 blocks with random domino pairs ---
        // After slide, checkerboard parity flips, so fill patterns are reversed
        // from the delete patterns: [S,S,N,N] and [W,E,W,E].
        for r in 0..new_size - 1 {
            for c in 0..new_size - 1 {
                if ns[r][c] == D::Empty
                    && ns[r][c + 1] == D::Empty
                    && ns[r + 1][c] == D::Empty
                    && ns[r + 1][c + 1] == D::Empty
                    && in_diamond(r, c, k)
                    && in_diamond(r + 1, c, k)
                    && in_diamond(r, c + 1, k)
                    && in_diamond(r + 1, c + 1, k)
                {
                    if rng.random_range(0..2) == 0 {
                        // two horizontal dominoes facing apart: top=N (up), bottom=S (down)
                        ns[r][c] = D::N;     ns[r][c + 1] = D::N;
                        ns[r + 1][c] = D::S; ns[r + 1][c + 1] = D::S;
                    } else {
                        // two vertical dominoes facing apart: left=W, right=E
                        ns[r][c] = D::W;     ns[r][c + 1] = D::E;
                        ns[r + 1][c] = D::W; ns[r + 1][c + 1] = D::E;
                    }
                }
            }
        }

        state = ns;
    }

    // Render: 2 grid columns per state cell for aspect ratio correction.
    let size = 2 * order;
    let render_w = size * 2;
    let off_r = center_y.saturating_sub(order);
    let off_c = center_x.saturating_sub(render_w / 2);
    let colors = [palette[1], palette[2], palette[3], lighten(palette[2], 40)];
    for r in 0..size {
        for c in 0..size {
            if state[r][c] == D::Empty { continue; }
            let gr = off_r + r;
            let gc = off_c + c * 2;
            let color = match state[r][c] {
                D::N => colors[0],
                D::S => colors[1],
                D::E => colors[2],
                D::W => colors[3],
                D::Empty => unreachable!(),
            };
            for dx in 0..2 {
                let gx = gc + dx;
                if gr < grid.len() && gx < grid[0].len() {
                    grid[gr][gx] = Cell::with_bg(' ', Color::Reset, color);
                }
            }
        }
    }
}

/// Draw a small flower/rosette at (cx, cy)
fn draw_flower(grid: &mut Grid, cx: usize, cy: usize, style: usize, color: Color) {
    let patterns: &[&[(i32, i32, char)]] = &[
        &[(0,-1,'◆'), (-1,0,'◇'), (1,0,'◇'), (0,1,'◆'), (0,0,'✦')],
        &[(0,-1,'◠'), (-1,0,'◟'), (1,0,'◞'), (0,1,'◡'), (0,0,'◉')],
        &[(0,-1,'∧'), (-1,0,'⟨'), (1,0,'⟩'), (0,1,'∨'), (0,0,'✧'), (-1,-1,'╱'), (1,-1,'╲'), (-1,1,'╲'), (1,1,'╱')],
        &[(0,-1,'╥'), (-1,0,'╟'), (1,0,'╢'), (0,1,'╨'), (0,0,'╬'), (-1,-1,'╔'), (1,-1,'╗'), (-1,1,'╚'), (1,1,'╝')],
        &[(0,0,'⣿'), (-1,0,'⡇'), (1,0,'⢸'), (0,-1,'⣤'), (0,1,'⣶'), (-1,-1,'⠁'), (1,-1,'⠈'), (-1,1,'⢀'), (1,1,'⡀')],
    ];

    let pattern = patterns[style % patterns.len()];
    for (i, &(dx, dy, ch)) in pattern.iter().enumerate() {
        let x = cx as i32 + dx;
        let y = cy as i32 + dy;
        if x >= 0 && y >= 0 && (y as usize) < grid.len() && (x as usize) < grid[0].len() {
            // center glyph brighter, petals slightly dimmer
            let c = if i == pattern.len() - 1 || (dx == 0 && dy == 0) {
                lighten(color, 40)
            } else {
                color
            };
            grid[y as usize][x as usize] = Cell::new(ch, c);
        }
    }
}

/// Print the grid with ANSI color escape sequences.
/// Run-length optimized: only emits color codes when the color changes.
/// Render grid to plain text (no ANSI escapes). Each row is one line.
/// Fullwidth chars consume 2 columns; the placeholder cell is skipped.
fn grid_to_plain(grid: &Grid) -> Vec<String> {
    let mut lines = Vec::with_capacity(grid.len());
    for row in grid {
        let mut line = String::with_capacity(row.len());
        let mut skip_next = false;
        for cell in row {
            if skip_next {
                skip_next = false;
                continue;
            }
            line.push(cell.ch);
            if char_width(cell.ch) == 2 {
                skip_next = true;
            }
        }
        lines.push(line);
    }
    lines
}

fn render_grid(grid: &Grid) {
    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());

    let mut cur_fg = Color::Reset;
    let mut cur_bg = Color::Reset;

    for row in grid {
        let mut skip_next = false;
        for cell in row {
            if skip_next {
                skip_next = false;
                continue;
            }
            if cell.fg != cur_fg {
                write!(out, "{}", crossterm::style::SetForegroundColor(cell.fg)).unwrap();
                cur_fg = cell.fg;
            }
            if cell.bg != cur_bg {
                write!(out, "{}", crossterm::style::SetBackgroundColor(cell.bg)).unwrap();
                cur_bg = cell.bg;
            }
            write!(out, "{}", cell.ch).unwrap();
            // fullwidth char advances terminal cursor 2 columns,
            // so skip the next grid cell (it's a placeholder)
            if char_width(cell.ch) == 2 {
                skip_next = true;
            }
        }
        // reset at end of each line to avoid bg bleeding
        if cur_bg != Color::Reset {
            write!(out, "{}", crossterm::style::SetBackgroundColor(Color::Reset)).unwrap();
            cur_bg = Color::Reset;
        }
        writeln!(out).unwrap();
    }

    // final reset
    write!(out, "{}", crossterm::style::ResetColor).unwrap();
    out.flush().unwrap();
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 && (args[1] == "--help" || args[1] == "-h") {
        eprintln!("ascii-renderer <seed> [mode] [theme]");
        eprintln!();
        eprintln!("ARGS:");
        eprintln!("  seed     Integer seed for deterministic RNG (default: 42)");
        eprintln!("  mode     Rendering mode (default: full demo)");
        eprintln!("  theme    Named color theme (default: seed-derived palette)");
        eprintln!();
        eprintln!("MODES:");
        eprintln!("  (none)    Full demo: Truchet bg, trees, content, flowers");
        eprintln!("  tree      GRIS-style binary trees with flowers");
        eprintln!("  forest    Mixed scene: pine, willow, palm, GRIS tree, fruits");
        eprintln!("  aztec     Aztec diamond domino tiling");
        eprintln!("  fret      Stepped fret spirals and border bands");
        eprintln!("  flowers   All 5 flower stamp styles with labels");
        eprintln!("  fruits    All 5 fruit stamp styles with labels");
        eprintln!("  layout    Two-column layout engine demo");
        eprintln!("  md        Render markdown from stdin");
        eprintln!("  bsp       BSP randomized layout demo");
        eprintln!("  mondrian  Mondrian-style colored grid layout");
        eprintln!("  tiles     Showcase all 10 tile patterns (pure deterministic)");
        eprintln!("  tiles-rand  Same patterns with randomized params");
        eprintln!("  noise     Showcase all 5 noise variants (truchet, higaki, etc.)");
        eprintln!("  swatch    Color swatches for all named themes");
        eprintln!();
        eprintln!("THEMES:");
        eprintln!("  warm:  ember, terracotta, sakura");
        eprintln!("  cool:  arctic, deep, moss");
        eprintln!("  mono:  bone, silver");
        eprintln!("  vivid: neon, nerv, mitla");
        eprintln!();
        eprintln!("EXAMPLES:");
        eprintln!("  ascii-renderer 42");
        eprintln!("  ascii-renderer 42 tree mitla");
        eprintln!("  ascii-renderer 99 forest moss");
        eprintln!("  ascii-renderer 7 aztec nerv");
        eprintln!("  ascii-renderer 0 fret neon");
        eprintln!("  ascii-renderer 42 fruits");
        eprintln!("  ascii-renderer 42 layout ember");
        eprintln!("  echo '# Hello' | ascii-renderer 42 md nerv");
        eprintln!("  cat notes.md | ascii-renderer 42 md moss");
        eprintln!("  ascii-renderer 42 bsp nerv");
        eprintln!("  ascii-renderer 42 mondrian");
        eprintln!("  ascii-renderer 42 swatch");
        std::process::exit(0);
    }

    let seed: u64 = args.get(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(42);

    let mode = args.get(2).map(|s| s.as_str()).unwrap_or("");
    let theme_name = args.get(3).map(|s| s.as_str()).unwrap_or("");

    let (term_w, term_h) = crossterm::terminal::size().unwrap_or((80, 45));
    let width = term_w as usize;
    let height = term_h as usize;
    let mut grid = vec![vec![Cell::blank(); width]; height];
    let mut rng = StdRng::seed_from_u64(seed);

    // Named theme overrides seed-based palette. Usage: <seed> <mode> <theme>
    // e.g. `42 "" nerv` or `42 tree mitla`
    let palette = if !theme_name.is_empty() {
        named_theme(&theme_name).unwrap_or_else(|| {
            let themes = ["ember", "terracotta", "sakura", "arctic", "deep", "moss",
                         "bone", "silver", "neon", "nerv", "mitla"];
            eprintln!("unknown theme '{}'. available: {}", theme_name, themes.join(", "));
            make_palette(seed)
        })
    } else {
        make_palette(seed)
    };

    if mode == "swatch" {
        // render all named themes as color swatches
        let themes = [
            "ember", "terracotta", "sakura",
            "arctic", "deep", "moss",
            "bone", "silver",
            "neon", "nerv", "mitla",
        ];
        let mut swatch_grid = vec![vec![Cell::blank(); 80]; themes.len() * 3 + 1];
        for (ti, name) in themes.iter().enumerate() {
            let p = named_theme(name).unwrap();
            let row = ti * 3;

            // label
            for (j, ch) in name.chars().enumerate() {
                if j < 12 {
                    swatch_grid[row][j] = Cell::new(ch, p[4]);
                }
            }

            // color blocks: bg, primary, secondary, accent, text
            let labels = ["bg", "pri", "sec", "acc", "txt"];
            for (ci, &color) in p.iter().enumerate() {
                let x_start = 13 + ci * 13;
                // label above
                for (j, ch) in labels[ci].chars().enumerate() {
                    if x_start + j < 80 {
                        swatch_grid[row][x_start + j] = Cell::new(ch, color);
                    }
                }
                // solid block
                for x in x_start..x_start + 10 {
                    if x < 80 {
                        swatch_grid[row + 1][x] = Cell::with_bg('█', color, Color::Reset);
                    }
                }
                // sample glyphs
                let sample = ['╱', '╲', '│', '─', '┌', '┐', '◆', '✦', '▀', '▄'];
                for (j, &ch) in sample.iter().enumerate() {
                    if x_start + j < 80 {
                        swatch_grid[row + 2][x_start + j] = Cell::new(ch, color);
                    }
                }
            }
        }
        render_grid(&swatch_grid);
        return;

    } else if mode == "tree" {
        grow_tree(&mut grid, 20, 40, 5, 16, palette[1], &mut rng);
        grow_tree(&mut grid, 55, 42, 8, 12, palette[2], &mut rng);

        draw_flower(&mut grid, 10, 42, 0, palette[3]);
        draw_flower(&mut grid, 70, 43, 1, palette[3]);
        draw_flower(&mut grid, 38, 38, 2, palette[3]);
        draw_flower(&mut grid, 45, 20, 3, palette[1]);
        draw_flower(&mut grid, 5, 10, 4, palette[2]);

    } else if mode == "aztec" {
        draw_aztec_diamond(&mut grid, width / 2, height / 2, height / 2 - 2, &palette, &mut rng);

    } else if mode == "fret" {
        // fret spirals at different sizes
        draw_stepped_fret(&mut grid, 5, 5, 3, Dir::Right, palette[1]);
        draw_stepped_fret(&mut grid, 25, 5, 5, Dir::Right, palette[2]);
        draw_stepped_fret(&mut grid, 50, 5, 7, Dir::Right, palette[3]);

        draw_stepped_fret(&mut grid, 10, 20, 5, Dir::Right, palette[1]);
        draw_stepped_fret(&mut grid, 30, 30, 5, Dir::Left, palette[2]);

        // border bands, each edge a different color
        draw_fret_border(&mut grid, 0, 0, width, height, 4, 0, palette[1]);
        draw_fret_border(&mut grid, 0, 0, width, height, 4, 1, palette[2]);
        draw_fret_border(&mut grid, 0, 0, width, height, 4, 2, palette[3]);
        draw_fret_border(&mut grid, 0, 0, width, height, 4, 3, palette[1]);

    } else if mode == "flowers" {
        for i in 0..5 {
            let color = [palette[1], palette[2], palette[3], palette[1], palette[2]][i];
            draw_flower(&mut grid, 8 + i * 15, 5, i, color);
            let labels = ["diamond", "circle", "star", "box", "braille"];
            for (j, ch) in labels[i].chars().enumerate() {
                if 8 + i * 15 - 2 + j < width {
                    grid[9][8 + i * 15 - 2 + j] = Cell::new(ch, palette[4]);
                }
            }
        }

    } else if mode == "fruits" {
        // showcase all fruit styles
        let fruit_colors = [
            rgb(220, 50, 50),   // apple red
            rgb(180, 30, 60),   // cherry
            rgb(240, 180, 30),  // citrus
            rgb(100, 50, 160),  // berry
            rgb(180, 200, 40),  // pear
        ];
        let labels = ["apple", "cherry", "citrus", "berry", "pear"];
        for i in 0..5 {
            draw_fruit(&mut grid, 8 + i * 15, 5, i, fruit_colors[i]);
            for (j, ch) in labels[i].chars().enumerate() {
                if 8 + i * 15 - 2 + j < width {
                    grid[9][8 + i * 15 - 2 + j] = Cell::new(ch, palette[4]);
                }
            }
        }

    } else if mode == "forest" {
        // mixed forest scene: truchet ground, multiple tree types, fruits scattered
        let ground_color = darken(palette[1], 90);
        let tiles = ['╱', '╲'];
        for y in 0..height {
            for x in 0..width {
                grid[y][x] = Cell::new(tiles[rng.random_range(0..2)], ground_color);
            }
        }

        // ground line
        let ground_y = height - 4;

        // GRIS tree left
        for y in 3..ground_y { for x in 2..22 { grid[y][x] = Cell::blank(); } }
        grow_tree(&mut grid, 12, ground_y - 1, 4, 8, palette[1], &mut rng);

        // pine tree center-left
        for y in 5..(ground_y + 1) { for x in 24..40 { grid[y][x] = Cell::blank(); } }
        draw_pine(&mut grid, 32, ground_y - 1, 3, 10, palette[2]);

        // willow center-right
        for y in 3..(ground_y + 3) { for x in 42..62 { grid[y][x] = Cell::blank(); } }
        draw_willow(&mut grid, 52, ground_y - 1, 6, 8, palette[1]);

        // palm right
        for y in 2..(ground_y + 1) { for x in 64..78 { grid[y][x] = Cell::blank(); } }
        draw_palm(&mut grid, 71, ground_y - 1, 20, palette[3], &mut rng);

        // scatter fruits on/near the GRIS tree
        draw_fruit(&mut grid, 8, 12, 0, rgb(220, 50, 50));   // apple
        draw_fruit(&mut grid, 15, 10, 0, rgb(200, 60, 40));   // apple
        draw_fruit(&mut grid, 11, 8, 1, rgb(180, 30, 60));    // cherry

        // fruits near the pine
        draw_fruit(&mut grid, 30, 25, 3, rgb(100, 50, 160));  // berry
        draw_fruit(&mut grid, 35, 28, 3, rgb(120, 40, 140));  // berry

        // fruits near willow
        draw_fruit(&mut grid, 48, 20, 2, rgb(240, 180, 30));  // citrus
        draw_fruit(&mut grid, 55, 18, 4, rgb(180, 200, 40));  // pear

        // flowers along ground
        for i in 0..6 {
            let fx = 5 + i * 13;
            if fx < width - 2 {
                draw_flower(&mut grid, fx, ground_y + 1, rng.random_range(0..5), palette[3]);
            }
        }

    } else if mode == "layout" {
        // two-column layout demo with truchet background
        let truchet_color = darken(palette[1], 90);
        let tiles = ['╱', '╲'];
        for y in 0..height {
            for x in 0..width {
                grid[y][x] = Cell::new(tiles[rng.random_range(0..2)], truchet_color);
            }
        }

        let left = vec![
            ContentBlock {
                items: vec![
                    ContentItem::Text("「 STATUS 」".into()),
                    ContentItem::Rule,
                    ContentItem::Text("All systems operational. Last deploy 2h ago.".into()),
                ],
                padding: 1,
            },
            ContentBlock {
                items: vec![
                    ContentItem::Text("METRICS".into()),
                    ContentItem::Rule,
                    ContentItem::Bar { label: "cpu".into(), value: 72.0, max: 100.0 },
                    ContentItem::Bar { label: "mem".into(), value: 4.8, max: 8.0 },
                    ContentItem::Bar { label: "disk".into(), value: 120.0, max: 500.0 },
                    ContentItem::Bar { label: "net".into(), value: 340.0, max: 1000.0 },
                ],
                padding: 1,
            },
        ];

        let right = vec![
            ContentBlock {
                items: vec![
                    ContentItem::Text("「 SKILLS 」".into()),
                    ContentItem::Rule,
                    ContentItem::Text("typespec ···· 12".into()),
                    ContentItem::Text("ast-grep ···· 5".into()),
                    ContentItem::Text("tree-sit ···· 3".into()),
                    ContentItem::Text("alloy    ···· 2".into()),
                    ContentItem::Rule,
                    ContentItem::Text("◁━━ 43 LOADED".into()),
                ],
                padding: 1,
            },
            ContentBlock {
                items: vec![
                    ContentItem::Text("TASKS".into()),
                    ContentItem::Rule,
                    ContentItem::Text("▪ layout engine".into()),
                    ContentItem::Text("▪ masonry fills".into()),
                    ContentItem::Text("▪ yaml parsing".into()),
                    ContentItem::Text("▫ snapshot tests".into()),
                    ContentItem::Text("▫ fret connect".into()),
                ],
                padding: 1,
            },
        ];

        let _rects = layout_two_col(
            &mut grid, &left, &right,
            4,  // gap between columns
            2,  // margin from canvas edge
            palette[4], palette[3],
        );

        // ornaments in the gap and margins
        draw_flower(&mut grid, width / 2, 3, rng.random_range(0..5), palette[3]);
        draw_flower(&mut grid, width / 2, height - 4, rng.random_range(0..5), palette[3]);
        draw_flower(&mut grid, 1, height / 2, rng.random_range(0..5), palette[2]);
        draw_flower(&mut grid, width - 2, height / 2, rng.random_range(0..5), palette[2]);

    } else if mode == "md" {
        // read markdown from stdin, parse into content blocks, lay out
        let mut input = String::new();
        io::stdin().read_to_string(&mut input).unwrap_or_default();
        let blocks = parse_markdown(&input);

        if blocks.is_empty() {
            eprintln!("no content on stdin. usage: echo '# Title' | ascii-renderer 42 md [theme]");
            std::process::exit(1);
        }

        // fill background with truchet
        let truchet_color = darken(palette[1], 90);
        let tiles = ['╱', '╲'];
        for y in 0..height {
            for x in 0..width {
                grid[y][x] = Cell::new(tiles[rng.random_range(0..2)], truchet_color);
            }
        }

        // fret border band depth - content must stay inside this
        let border_band = if width > 40 && height > 20 { 3 } else { 0 };
        let content_margin = border_band + 1;

        // lay out content blocks
        let rects = if blocks.len() <= 2 {
            let col_w = width.saturating_sub(content_margin * 2);
            let mut cy = content_margin;
            let mut rects = Vec::new();
            for block in &blocks {
                let (_, h) = measure_block(block, col_w);
                let h = h.min(height.saturating_sub(cy + content_margin));
                if h == 0 { break; }
                let rect = Rect { x: content_margin, y: cy, w: col_w, h };
                render_block(&mut grid, block, &rect, palette[4], palette[3]);
                rects.push(rect);
                cy += h + 1;
            }
            rects
        } else {
            layout_bsp(
                &mut grid, &blocks,
                content_margin, 14, 4,
                palette[4], palette[3],
                &mut rng,
            )
        };

        // borders around content rects
        let content_count = blocks.len().min(rects.len());
        for i in 0..content_count {
            let style = pick_border_style(&mut rng, rects[i].w, rects[i].h);
            draw_box_border(&mut grid, &rects[i], &style, palette[4]);
        }

        // fill empty leaves with walker-driven primitives
        let empty_leaves: Vec<Rect> = rects.into_iter().skip(content_count).collect();
        walk_and_fill_leaves(&mut grid, &empty_leaves, &palette, &mut rng);

        // fret border along edges if canvas is big enough
        if width > 40 && height > 20 {
            let band = 3;
            draw_fret_border(&mut grid, 0, 0, width, height, band, 0, palette[2]);
            draw_fret_border(&mut grid, 0, 0, width, height, band, 1, palette[2]);
            draw_fret_border(&mut grid, 0, 0, width, height, band, 2, palette[2]);
            draw_fret_border(&mut grid, 0, 0, width, height, band, 3, palette[2]);
        }

    } else if mode == "bsp" {
        // BSP randomized layout demo
        let truchet_color = darken(palette[1], 90);
        let tiles = ['╱', '╲'];
        for y in 0..height {
            for x in 0..width {
                grid[y][x] = Cell::new(tiles[rng.random_range(0..2)], truchet_color);
            }
        }

        let blocks = vec![
            ContentBlock {
                items: vec![
                    ContentItem::Text("「 STATUS 」".into()),
                    ContentItem::Rule,
                    ContentItem::Text("All systems operational.".into()),
                    ContentItem::Text("Last deploy 2h ago.".into()),
                ],
                padding: 1,
            },
            ContentBlock {
                items: vec![
                    ContentItem::Text("METRICS".into()),
                    ContentItem::Rule,
                    ContentItem::Bar { label: "cpu".into(), value: 72.0, max: 100.0 },
                    ContentItem::Bar { label: "mem".into(), value: 4.8, max: 8.0 },
                    ContentItem::Bar { label: "disk".into(), value: 120.0, max: 500.0 },
                ],
                padding: 1,
            },
            ContentBlock {
                items: vec![
                    ContentItem::Text("「 SKILLS 」".into()),
                    ContentItem::Rule,
                    ContentItem::Text("typespec ···· 12".into()),
                    ContentItem::Text("ast-grep ···· 5".into()),
                    ContentItem::Text("tree-sit ···· 3".into()),
                    ContentItem::Text("alloy    ···· 2".into()),
                ],
                padding: 1,
            },
            ContentBlock {
                items: vec![
                    ContentItem::Text("TASKS".into()),
                    ContentItem::Rule,
                    ContentItem::Text("▪ layout engine".into()),
                    ContentItem::Text("▪ masonry fills".into()),
                    ContentItem::Text("▫ yaml parsing".into()),
                ],
                padding: 1,
            },
            ContentBlock {
                items: vec![
                    ContentItem::Text("NOTES".into()),
                    ContentItem::Rule,
                    ContentItem::Text("BSP splits the canvas into randomized regions. Each content block gets assigned to the largest available leaf. Remaining leaves stay as pattern fill.".into()),
                ],
                padding: 1,
            },
        ];

        let rects = layout_bsp(
            &mut grid, &blocks,
            1,   // margin
            12,  // min cell width
            5,   // min cell height
            palette[4], palette[3],
            &mut rng,
        );

        // ornaments in empty leaf rects (those beyond the content blocks)
        for rect in rects.iter().skip(blocks.len()) {
            let cx = rect.x + rect.w / 2;
            let cy = rect.y + rect.h / 2;
            if rect.w >= 5 && rect.h >= 3 {
                draw_flower(&mut grid, cx, cy, rng.random_range(0..5), palette[3]);
            }
        }

    } else if mode == "mondrian" {
        let line_w = 2;

        // try reading markdown from stdin, fall back to generated content
        let mut stdin_buf = String::new();
        let has_stdin = !std::io::stdin().is_terminal();
        if has_stdin {
            io::stdin().read_to_string(&mut stdin_buf).unwrap_or_default();
        }

        let blocks = if !stdin_buf.is_empty() {
            parse_markdown(&stdin_buf)
        } else {
            // seed-varied procedural content
            let status_msgs = [
                "All systems nominal.", "Drift detected. Compensating.",
                "Awaiting signal.", "Calibrating.", "Standing by.",
                "Online.", "Synchronizing.", "Lattice stable.",
            ];
            let task_sets: [&[&str]; 4] = [
                &["▪ layout engine", "▪ masonry fills", "▫ fret connect"],
                &["▪ wave collapse", "▪ L-systems", "▫ snapshot tests"],
                &["▪ signal graph", "▪ render pass", "▫ cache layer"],
                &["▪ parse phase", "▪ emit codegen", "▫ type resolve"],
            ];
            let stat = status_msgs[rng.random_range(0..status_msgs.len())];
            let tasks = task_sets[rng.random_range(0..task_sets.len())];

            let cpu_v = rng.random_range(20..95) as f64;
            let mem_v = rng.random_range(10..80) as f64 / 10.0;
            let disk_v = rng.random_range(30..450) as f64;
            let net_v = rng.random_range(50..900) as f64;

            let mut b = vec![
                ContentBlock {
                    items: vec![
                        ContentItem::Text("「 STATUS 」".into()),
                        ContentItem::Rule,
                        ContentItem::Text(stat.into()),
                    ],
                    padding: 1,
                },
                ContentBlock {
                    items: vec![
                        ContentItem::Text("METRICS".into()),
                        ContentItem::Rule,
                        ContentItem::Bar { label: "cpu".into(), value: cpu_v, max: 100.0 },
                        ContentItem::Bar { label: "mem".into(), value: mem_v, max: 8.0 },
                        ContentItem::Bar { label: "disk".into(), value: disk_v, max: 500.0 },
                        ContentItem::Bar { label: "net".into(), value: net_v, max: 1000.0 },
                    ],
                    padding: 1,
                },
            ];
            let mut task_items = vec![
                ContentItem::Text("TASKS".into()),
                ContentItem::Rule,
            ];
            for t in tasks { task_items.push(ContentItem::Text((*t).into())); }
            b.push(ContentBlock { items: task_items, padding: 1 });

            // sometimes add a 4th block
            if rng.random_range(0..3) == 0 {
                let notes = [
                    "The map is not the territory.",
                    "Form follows function, but function follows context.",
                    "Every system is perfectly designed to produce the results it gets.",
                    "Constraints breed creativity.",
                ];
                b.push(ContentBlock {
                    items: vec![
                        ContentItem::Text("NOTES".into()),
                        ContentItem::Rule,
                        ContentItem::Text(notes[rng.random_range(0..notes.len())].into()),
                    ],
                    padding: 1,
                });
            }
            b
        };

        // theme-aware colors: derive mondrian fills from the palette
        let fill_colors = if theme_name.is_empty() {
            // classic mondrian
            let (fills, _) = mondrian_colors();
            fills
        } else {
            // use the theme: bg slot as the dominant fill, other palette colors as accents
            [
                lighten(palette[0], 40),  // lightened bg as dominant
                palette[1],               // primary
                palette[2],               // secondary
                palette[3],               // accent
                lighten(palette[0], 40),  // bg again for weight
            ]
        };
        let line_color = if theme_name.is_empty() {
            rgb(20, 20, 20)
        } else {
            darken(palette[0], 60)
        };
        let text_fg = if theme_name.is_empty() {
            rgb(20, 20, 20)
        } else {
            palette[4] // theme text color
        };

        let rects = layout_mondrian(
            &mut grid, &blocks,
            0, line_w, 12, 5,
            text_fg, text_fg,
            &fill_colors, line_color,
            &mut rng,
        );

        // fill empty leaves with walker-driven primitives
        let content_count = blocks.len().min(rects.len());
        let empty_leaves: Vec<Rect> = rects.into_iter().skip(content_count).collect();
        walk_and_fill_leaves(&mut grid, &empty_leaves, &palette, &mut rng);

    } else if mode == "tiles" {
        // showcase all tile patterns: pure deterministic baseline
        let names = [
            "asanoha", "seigaiha", "shippo", "bishamon", "yabane",
            "nowaki", "higaki", "shell", "granny", "crocodile",
        ];
        let cols = 5.min(TILE_VARIANT_COUNT);
        let rows = (TILE_VARIANT_COUNT + cols - 1) / cols;
        let cell_w = width / cols;
        let cell_h = height / rows;
        for i in 0..TILE_VARIANT_COUNT {
            let col = i % cols;
            let row = i / cols;
            let x0 = col * cell_w;
            let y0 = row * cell_h;
            let r = Rect { x: x0, y: y0 + 1, w: cell_w, h: cell_h.saturating_sub(1) };
            let variant = tile_variant_from_index(i);
            let c1 = palette[(i % 3) + 1];
            let c2 = darken(c1, 30);
            fill_tile_pure(&mut grid, &r, variant, c1, c2);
            // label
            for (j, ch) in names[i].chars().enumerate() {
                if x0 + j < width && y0 < height {
                    grid[y0][x0 + j] = Cell::new(ch, palette[4]);
                }
            }
        }

    } else if mode == "tiles-rand" {
        // showcase tile patterns with randomized params (phase, stagger, jitter)
        let names = [
            "asanoha", "seigaiha", "shippo", "bishamon", "yabane",
            "nowaki", "higaki", "shell", "granny", "crocodile",
        ];
        let cols = 5.min(TILE_VARIANT_COUNT);
        let rows = (TILE_VARIANT_COUNT + cols - 1) / cols;
        let cell_w = width / cols;
        let cell_h = height / rows;
        for i in 0..TILE_VARIANT_COUNT {
            let col = i % cols;
            let row = i / cols;
            let x0 = col * cell_w;
            let y0 = row * cell_h;
            let r = Rect { x: x0, y: y0 + 1, w: cell_w, h: cell_h.saturating_sub(1) };
            let mut params = TileParams::randomized(&mut rng);
            params.variant = tile_variant_from_index(i);
            let c1 = palette[(i % 3) + 1];
            let c2 = darken(c1, 30);
            let jitter = rng.random_range(0..15) as f32 / 100.0;
            fill_tile_ex(&mut grid, &r, &params, c1, c2, jitter, &mut rng);
            // label with params
            let label = format!("{} d{:.0} s{} r{}",
                names[i],
                params.density * 100.0,
                params.stagger_override,
                params.rhythm_override,
            );
            for (j, ch) in label.chars().enumerate() {
                if x0 + j < width && y0 < height {
                    grid[y0][x0 + j] = Cell::new(ch, palette[4]);
                }
            }
        }

    } else if mode == "noise" {
        // showcase all noise variants
        let names = ["truchet", "higaki", "higaki-s", "grass", "static", "dot"];
        let cols = NOISE_VARIANT_COUNT;
        let cell_w = width / cols;
        for i in 0..NOISE_VARIANT_COUNT {
            let x0 = i * cell_w;
            let r = Rect { x: x0, y: 1, w: cell_w, h: height - 1 };
            let variant = noise_variant_from_index(i);
            let c1 = palette[(i % 3) + 1];
            let c2 = darken(c1, 30);
            fill_noise(&mut grid, &r, variant, c1, c2, &mut rng);
            for (j, ch) in names[i].chars().enumerate() {
                if x0 + j < width {
                    grid[0][x0 + j] = Cell::new(ch, palette[4]);
                }
            }
        }

    } else {
        // full demo: truchet bg + trees + content + flowers
        fill_truchet(&mut grid, width, height, darken(palette[1], 80), &mut rng);

        // carve content region
        let cx = width / 2;
        let cy = height / 2;
        let content_w = 30;
        let content_h = 10;
        let x0 = cx - content_w / 2;
        let y0 = cy - content_h / 2;

        for y in y0..y0 + content_h {
            for x in x0..x0 + content_w {
                grid[y][x] = Cell::blank();
            }
        }

        let lines = [
            "「 技 」 S K I L L S",
            "",
            "  typespec ···· 12",
            "  ast-grep ···· 5",
            "  tree-sit ···· 3",
            "  alloy    ···· 2",
            "",
            "  ◁━━ 43 LOADED",
        ];

        for (i, line) in lines.iter().enumerate() {
            let y = y0 + 1 + i;
            if y < y0 + content_h {
                for (j, ch) in line.chars().enumerate() {
                    let x = x0 + 1 + j;
                    if x < x0 + content_w {
                        grid[y][x] = Cell::new(ch, palette[4]);
                    }
                }
            }
        }

        // trees in corners
        for y in 2..18 {
            for x in 2..22 { grid[y][x] = Cell::blank(); }
        }
        grow_tree(&mut grid, 12, 17, 3, 8, palette[1], &mut rng);

        for y in 2..18 {
            for x in 58..78 { grid[y][x] = Cell::blank(); }
        }
        grow_tree(&mut grid, 68, 17, 3, 8, palette[2], &mut rng);

        // flowers
        draw_flower(&mut grid, 30, 8, rng.random_range(0..5), palette[3]);
        draw_flower(&mut grid, 50, 8, rng.random_range(0..5), palette[3]);
        draw_flower(&mut grid, 15, 35, rng.random_range(0..5), palette[3]);
        draw_flower(&mut grid, 65, 35, rng.random_range(0..5), palette[3]);
        draw_flower(&mut grid, 40, 38, rng.random_range(0..5), palette[3]);
    }

    render_grid(&grid);
}

#[cfg(test)]
mod tests {
    use super::*;
    use unicode_width::UnicodeWidthStr;

    /// Every row in the plain-text output must have exactly `width` display columns.
    fn assert_uniform_display_width(grid: &Grid, expected: usize) {
        let lines = grid_to_plain(grid);
        for (i, line) in lines.iter().enumerate() {
            let w = UnicodeWidthStr::width(line.as_str());
            assert_eq!(
                w, expected,
                "row {} has display width {} (expected {}): {:?}",
                i, w, expected, line,
            );
        }
    }

    fn make_grid(width: usize, height: usize, seed: u64) -> (Grid, StdRng, [Color; 5]) {
        let grid = vec![vec![Cell::blank(); width]; height];
        let rng = StdRng::seed_from_u64(seed);
        let palette = make_palette(seed);
        (grid, rng, palette)
    }

    // ── display width tests ────────────────────────────────────────

    #[test]
    fn mondrian_display_width() {
        let (mut grid, mut rng, _) = make_grid(80, 45, 42);
        let blocks = vec![
            ContentBlock {
                items: vec![
                    ContentItem::Text("「 STATUS 」".into()),
                    ContentItem::Rule,
                    ContentItem::Text("All systems operational.".into()),
                ],
                padding: 1,
            },
            ContentBlock {
                items: vec![
                    ContentItem::Text("METRICS".into()),
                    ContentItem::Rule,
                    ContentItem::Bar { label: "cpu".into(), value: 72.0, max: 100.0 },
                    ContentItem::Bar { label: "mem".into(), value: 4.8, max: 8.0 },
                ],
                padding: 1,
            },
            ContentBlock {
                items: vec![
                    ContentItem::Text("「 SKILLS 」".into()),
                    ContentItem::Rule,
                    ContentItem::Text("typespec ···· 12".into()),
                    ContentItem::Text("ast-grep ···· 5".into()),
                    ContentItem::Text("tree-sit ···· 3".into()),
                ],
                padding: 1,
            },
        ];
        let (_, line_color) = mondrian_colors();
        let text_fg = rgb(20, 20, 20);
        let (fills, _) = mondrian_colors();
        layout_mondrian(
            &mut grid, &blocks, 0, 2, 10, 5,
            text_fg, line_color, &fills, line_color, &mut rng,
        );
        assert_uniform_display_width(&grid, 80);
    }

    #[test]
    fn mondrian_different_seeds_display_width() {
        for seed in [0, 1, 7, 42, 99, 1234] {
            let (mut grid, mut rng, _) = make_grid(80, 45, seed);
            let blocks = vec![
                ContentBlock {
                    items: vec![
                        ContentItem::Text("「 STATUS 」".into()),
                        ContentItem::Rule,
                        ContentItem::Text("Online.".into()),
                    ],
                    padding: 1,
                },
            ];
            let (fills, line_color) = mondrian_colors();
            layout_mondrian(
                &mut grid, &blocks, 0, 2, 10, 5,
                rgb(20, 20, 20), line_color, &fills, line_color, &mut rng,
            );
            assert_uniform_display_width(&grid, 80);
        }
    }

    #[test]
    fn default_mode_display_width() {
        let (mut grid, mut rng, palette) = make_grid(80, 45, 42);
        // truchet bg
        let truchet_color = darken(palette[1], 80);
        let tiles = ['╱', '╲'];
        for y in 0..45 {
            for x in 0..80 {
                grid[y][x] = Cell::new(tiles[rng.random_range(0..2)], truchet_color);
            }
        }
        // content with fullwidth chars
        let cx = 40; let cy = 22;
        let lines = ["「 技 」 S K I L L S", "", "  typespec ···· 12"];
        for (i, line) in lines.iter().enumerate() {
            let mut col = 0usize;
            for ch in line.chars() {
                let cw = char_width(ch);
                let gx = cx - 15 + col;
                if gx < 80 {
                    grid[cy - 5 + 1 + i][gx] = Cell::new(ch, palette[4]);
                    if cw == 2 && gx + 1 < 80 {
                        grid[cy - 5 + 1 + i][gx + 1] = Cell::blank();
                    }
                }
                col += cw;
            }
        }
        assert_uniform_display_width(&grid, 80);
    }

    #[test]
    fn bsp_display_width() {
        let (mut grid, mut rng, palette) = make_grid(80, 45, 42);
        let truchet_color = darken(palette[1], 90);
        let tiles = ['╱', '╲'];
        for y in 0..45 {
            for x in 0..80 {
                grid[y][x] = Cell::new(tiles[rng.random_range(0..2)], truchet_color);
            }
        }
        let blocks = vec![
            ContentBlock {
                items: vec![
                    ContentItem::Text("「 STATUS 」".into()),
                    ContentItem::Rule,
                    ContentItem::Text("All systems operational.".into()),
                ],
                padding: 1,
            },
            ContentBlock {
                items: vec![
                    ContentItem::Text("TASKS".into()),
                    ContentItem::Rule,
                    ContentItem::Text("▪ layout engine".into()),
                ],
                padding: 1,
            },
        ];
        layout_bsp(
            &mut grid, &blocks, 1, 12, 5,
            palette[4], palette[3], &mut rng,
        );
        assert_uniform_display_width(&grid, 80);
    }

    // ── wrap_text tests ────────────────────────────────────────────

    #[test]
    fn wrap_text_fullwidth_chars() {
        // 「」 are 2 display columns each, so "「 X 」" = 2+1+1+1+2 = 7 cols
        let lines = wrap_text("「 X 」", 7);
        assert_eq!(lines, vec!["「 X 」"]);

        // should wrap when too narrow
        let lines = wrap_text("「 X 」 extra", 7);
        assert_eq!(lines, vec!["「 X 」", "extra"]);
    }

    #[test]
    fn wrap_text_ascii_basic() {
        let lines = wrap_text("hello world foo", 11);
        assert_eq!(lines, vec!["hello world", "foo"]);
    }

    // ── min_block_width tests ──────────────────────────────────────

    #[test]
    fn min_block_width_accounts_for_fullwidth() {
        let block = ContentBlock {
            items: vec![ContentItem::Text("「 SKILLS 」".into())],
            padding: 1,
        };
        // 「(2) + space(1) + S(1)+K(1)+I(1)+L(1)+L(1)+S(1) + space(1) + 」(2) = 12
        // + padding*2 = 14
        assert_eq!(min_block_width(&block), 14);
    }

    // ── BSP split_with_gap tests ───────────────────────────────────

    #[test]
    fn bsp_split_gap_leaves_cover_canvas() {
        // leaves + gaps should account for the full parent rect
        let mut rng = StdRng::seed_from_u64(42);
        let mut root = BspNode::new(0, 0, 80, 45);
        root.split_with_gap(10, 5, 4, 2, &mut rng);
        let leaves = root.leaves();
        assert!(leaves.len() >= 2, "should produce multiple leaves");
        for leaf in &leaves {
            assert!(leaf.x + leaf.w <= 80, "leaf x overflow");
            assert!(leaf.y + leaf.h <= 45, "leaf y overflow");
            assert!(leaf.w >= 10, "leaf too narrow");
            assert!(leaf.h >= 5, "leaf too short");
        }
    }

    #[test]
    fn bsp_split_gap1_backward_compat() {
        // split() should behave identically to split_with_gap(..., 1, ...)
        let mut rng1 = StdRng::seed_from_u64(99);
        let mut rng2 = StdRng::seed_from_u64(99);
        let mut a = BspNode::new(0, 0, 80, 45);
        let mut b = BspNode::new(0, 0, 80, 45);
        a.split(10, 5, 4, &mut rng1);
        b.split_with_gap(10, 5, 4, 1, &mut rng2);
        let la: Vec<_> = a.leaves().iter().map(|r| (r.x, r.y, r.w, r.h)).collect();
        let lb: Vec<_> = b.leaves().iter().map(|r| (r.x, r.y, r.w, r.h)).collect();
        assert_eq!(la, lb);
    }

    // ── best-fit assignment test ───────────────────────────────────

    #[test]
    fn mondrian_content_not_wrapped() {
        // content with fullwidth chars should land in a cell wide enough
        // to render without wrapping
        let (mut grid, mut rng, _) = make_grid(80, 45, 42);
        let blocks = vec![
            ContentBlock {
                items: vec![ContentItem::Text("「 SKILLS 」".into())],
                padding: 1,
            },
        ];
        let (fills, line_color) = mondrian_colors();
        layout_mondrian(
            &mut grid, &blocks, 0, 2, 10, 5,
            rgb(20, 20, 20), line_color, &fills, line_color, &mut rng,
        );
        // find the row containing "SKILLS" and check it's on one line
        let lines = grid_to_plain(&grid);
        let skill_rows: Vec<_> = lines.iter()
            .filter(|l| l.contains("SKILLS"))
            .collect();
        assert_eq!(skill_rows.len(), 1, "「 SKILLS 」 should appear on exactly one row");
        assert!(skill_rows[0].contains("「 SKILLS 」"),
            "full title should be on one line, got: {:?}", skill_rows[0]);
    }
}
