use crossterm::style::Color;
use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};
use rand::RngExt;
use rand::rngs::StdRng;
use rand::SeedableRng;
use std::io::{self, Read as _, Write};

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
    for word in text.split_whitespace() {
        let word_len = word.chars().count();
        let current_len = current.chars().count();
        if current_len == 0 {
            // first word on line - take it even if it overflows
            current = word.to_string();
        } else if current_len + 1 + word_len <= max_width {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(current);
            current = word.to_string();
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
                    max_line_w = max_line_w.max(line.chars().count());
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
                    for (j, ch) in line.chars().enumerate() {
                        let gx = inner_x + j;
                        if cy < grid.len() && gx < grid[0].len() {
                            grid[cy][gx] = Cell::new(ch, fg);
                        }
                    }
                    cy += 1;
                }
            }
            ContentItem::Bar { label, value, max } => {
                if !label.is_empty() {
                    for (j, ch) in label.chars().enumerate() {
                        let gx = inner_x + j;
                        if cy < grid.len() && gx < grid[0].len() {
                            grid[cy][gx] = Cell::new(ch, fg);
                        }
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
    /// split_range: how far from center the split can deviate (0.3..0.7 means 30-70%)
    fn split(&mut self, min_w: usize, min_h: usize, max_depth: usize, rng: &mut StdRng) {
        if max_depth == 0 { return; }

        let can_split_h = self.rect.w >= min_w * 2 + 1;
        let can_split_v = self.rect.h >= min_h * 2 + 1;

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
            let range_hi = self.rect.w.saturating_sub(min_w);
            if range_lo >= range_hi { return; }
            let split = rng.random_range(range_lo..range_hi);

            let mut left = Box::new(BspNode::new(
                self.rect.x, self.rect.y, split, self.rect.h,
            ));
            let mut right = Box::new(BspNode::new(
                self.rect.x + split + 1, self.rect.y,
                self.rect.w.saturating_sub(split + 1), self.rect.h,
            ));
            left.split(min_w, min_h, max_depth - 1, rng);
            right.split(min_w, min_h, max_depth - 1, rng);
            self.left = Some(left);
            self.right = Some(right);
        } else {
            // split along y axis (top/bottom children)
            let range_lo = min_h;
            let range_hi = self.rect.h.saturating_sub(min_h);
            if range_lo >= range_hi { return; }
            let split = rng.random_range(range_lo..range_hi);

            let mut top = Box::new(BspNode::new(
                self.rect.x, self.rect.y, self.rect.w, split,
            ));
            let mut bottom = Box::new(BspNode::new(
                self.rect.x, self.rect.y + split + 1,
                self.rect.w, self.rect.h.saturating_sub(split + 1),
            ));
            top.split(min_w, min_h, max_depth - 1, rng);
            bottom.split(min_w, min_h, max_depth - 1, rng);
            self.left = Some(top);
            self.right = Some(bottom);
        }
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
fn render_grid(grid: &Grid) {
    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());

    let mut cur_fg = Color::Reset;
    let mut cur_bg = Color::Reset;

    for row in grid {
        for cell in row {
            if cell.fg != cur_fg {
                write!(out, "{}", crossterm::style::SetForegroundColor(cell.fg)).unwrap();
                cur_fg = cell.fg;
            }
            if cell.bg != cur_bg {
                write!(out, "{}", crossterm::style::SetBackgroundColor(cell.bg)).unwrap();
                cur_bg = cell.bg;
            }
            write!(out, "{}", cell.ch).unwrap();
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
        eprintln!("  ascii-renderer 42 swatch");
        std::process::exit(0);
    }

    let seed: u64 = args.get(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(42);

    let mode = args.get(2).map(|s| s.as_str()).unwrap_or("");
    let theme_name = args.get(3).map(|s| s.as_str()).unwrap_or("");

    let width = 80;
    let height = 45;
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

        // decorate empty leaf rects with primitives
        let content_count = blocks.len().min(rects.len());
        for rect in rects.iter().skip(content_count) {
            let cx = rect.x + rect.w / 2;
            let cy = rect.y + rect.h / 2;
            let area = rect.w * rect.h;

            if area > 300 && rect.h > 15 && rect.w > 20 {
                // large region: tree
                let tree_type = rng.random_range(0..4);
                // clear region for the tree
                for y in rect.y..rect.y + rect.h {
                    for x in rect.x..rect.x + rect.w {
                        if y < grid.len() && x < grid[0].len() {
                            grid[y][x] = Cell::blank();
                        }
                    }
                }
                let root_y = rect.y + rect.h - 2;
                let canopy_y = rect.y + 2;
                match tree_type {
                    0 => grow_tree(&mut grid, cx, root_y, canopy_y, rect.w / 4, palette[1], &mut rng),
                    1 => draw_pine(&mut grid, cx, root_y, 3, (rect.w / 2).min(12), palette[2]),
                    2 => draw_willow(&mut grid, cx, root_y, canopy_y, rect.w / 4, palette[1]),
                    _ => draw_palm(&mut grid, cx, root_y, rect.h.saturating_sub(4), palette[3], &mut rng),
                }
                // scatter some fruits/flowers near the tree
                for _ in 0..rng.random_range(1..=3) {
                    let fx = rect.x + rng.random_range(2..rect.w.saturating_sub(2).max(3));
                    let fy = rect.y + rng.random_range(2..rect.h.saturating_sub(2).max(3));
                    if rng.random_range(0..2) == 0 {
                        draw_fruit(&mut grid, fx, fy, rng.random_range(0..5), palette[3]);
                    } else {
                        draw_flower(&mut grid, fx, fy, rng.random_range(0..5), palette[3]);
                    }
                }
            } else if area > 80 && rect.h > 6 && rect.w > 10 {
                // medium region: fret spiral or small aztec diamond
                if rng.random_range(0..2) == 0 {
                    let steps = (rect.w.min(rect.h) / 3).max(2).min(5);
                    draw_stepped_fret(&mut grid, rect.x as i32 + 2, rect.y as i32 + 1, steps, Dir::Right, palette[2]);
                } else {
                    let order = (rect.h / 2).min(rect.w / 4).max(2).min(6);
                    draw_aztec_diamond(&mut grid, cx, cy, order, &palette, &mut rng);
                }
            } else if area > 20 && rect.w >= 5 && rect.h >= 3 {
                // small region: flower or fruit stamp
                if rng.random_range(0..2) == 0 {
                    draw_flower(&mut grid, cx, cy, rng.random_range(0..5), palette[3]);
                } else {
                    draw_fruit(&mut grid, cx, cy, rng.random_range(0..5), palette[3]);
                }
            }
        }

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

    } else {
        // full demo: truchet bg + trees + content + flowers
        let truchet_color = darken(palette[1], 80);
        let tiles = ['╱', '╲'];
        for y in 0..height {
            for x in 0..width {
                grid[y][x] = Cell::new(
                    tiles[rng.random_range(0..2)],
                    truchet_color,
                );
            }
        }

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
