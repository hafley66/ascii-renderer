use crate::color::darken;
use crate::types::*;
use crossterm::style::Color;

pub enum ContentItem {
    Text(String),
    Bar { label: String, value: f64, max: f64 },
    Rule,
}

pub struct ContentBlock {
    pub items: Vec<ContentItem>,
    pub padding: usize,
}

/// Wrap a string to fit within max_width using greedy line breaking.
/// Returns the wrapped lines and the actual max line width used.
pub fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
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
pub fn measure_block(block: &ContentBlock, available_width: usize) -> (usize, usize) {
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
    (
        max_line_w + block.padding * 2,
        total_lines + block.padding * 2,
    )
}

/// Minimum width a block needs to avoid any text wrapping.
pub fn min_block_width(block: &ContentBlock) -> usize {
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
pub fn render_block(grid: &mut Grid, block: &ContentBlock, rect: &Rect, fg: Color, bar_fg: Color) {
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
                        if gx + cw > rect.x + rect.w {
                            break;
                        }
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
                        if gx + cw > rect.x + rect.w {
                            break;
                        }
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

/// Like render_block but preserves existing bg color of each cell
/// instead of clearing to blank. Used by Mondrian to keep color fills.
pub fn render_block_preserve_bg(
    grid: &mut Grid,
    block: &ContentBlock,
    rect: &Rect,
    fg: Color,
    bar_fg: Color,
) {
    let inner_x = rect.x + block.padding;
    let inner_y = rect.y + block.padding;
    let inner_w = rect.w.saturating_sub(block.padding * 2);
    let max_x = rect.x + rect.w;
    let max_y = rect.y + rect.h;

    let mut cy = inner_y;
    for item in &block.items {
        if cy >= max_y {
            break;
        }
        match item {
            ContentItem::Text(s) => {
                let wrapped = wrap_text(s, inner_w);
                for line in &wrapped {
                    if cy >= max_y {
                        break;
                    }
                    let mut col = 0usize; // display column offset
                    for ch in line.chars() {
                        let cw = char_width(ch);
                        let x = inner_x + col;
                        if x + cw > max_x {
                            break;
                        }
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
                        if x + cw > max_x {
                            break;
                        }
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
                if cy >= max_y {
                    continue;
                }
                let bar_w = inner_w.min(max_x.saturating_sub(inner_x));
                let filled = ((value / max) * bar_w as f64) as usize;
                for j in 0..bar_w {
                    let x = inner_x + j;
                    if x >= max_x {
                        break;
                    }
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
