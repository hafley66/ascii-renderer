use crossterm::style::Color;
use rand::rngs::StdRng;
use rand::RngExt;
use crate::types::*;
use crate::color::*;
use crate::content::*;
use crate::borders::*;
use crate::layout::BspNode;

/// Mondrian palette: classic Piet Mondrian primary colors + white.
/// Returns (fill_colors, line_color).
pub fn mondrian_colors() -> ([Color; 5], Color) {
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
pub fn draw_mondrian_lines(grid: &mut Grid, node: &BspNode, line_w: usize, color: Color) {
    if node.is_leaf() { return; }

    if let (Some(left), Some(right)) = (&node.left, &node.right) {
        let l = &left.rect;
        let r = &right.rect;
        let p = &node.rect;

        if l.y == r.y {
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
/// primary-color filled regions.
pub fn layout_mondrian(
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

    let inset = line_w + margin;
    let canvas_w = grid_w.saturating_sub(inset * 2);
    let canvas_h = grid_h.saturating_sub(inset * 2);

    let target_leaves = blocks.len().max(6);
    let max_depth = (target_leaves as f64).log2().ceil() as usize + 2;

    let mut root = BspNode::new(inset, inset, canvas_w, canvas_h);
    root.split_with_gap(min_cell_w, min_cell_h, max_depth, line_w, rng);

    let leaves = root.leaves();
    for (i, leaf) in leaves.iter().enumerate() {
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

    let leaf_rects: Vec<Rect> = leaves.iter().map(|r| {
        Rect { x: r.x, y: r.y, w: r.w, h: r.h }
    }).collect();
    let mut used = vec![false; leaf_rects.len()];
    let mut content_rects = Vec::new();

    for block in blocks.iter() {
        let needed_w = min_block_width(block);
        let (_, needed_h) = measure_block(block, needed_w);

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

    let mut all_rects = content_rects;
    for (li, r) in leaf_rects.iter().enumerate() {
        if !used[li] {
            all_rects.push(Rect { x: r.x, y: r.y, w: r.w, h: r.h });
        }
    }

    all_rects
}
