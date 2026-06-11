use crate::content::*;
use crate::types::*;
use crossterm::style::Color;
use rand::RngExt;
use rand::rngs::StdRng;

pub struct BspNode {
    pub rect: Rect,
    pub left: Option<Box<BspNode>>,
    pub right: Option<Box<BspNode>>,
}

impl BspNode {
    pub fn new(x: usize, y: usize, w: usize, h: usize) -> Self {
        BspNode {
            rect: Rect { x, y, w, h },
            left: None,
            right: None,
        }
    }

    pub fn is_leaf(&self) -> bool {
        self.left.is_none() && self.right.is_none()
    }

    /// Recursively split until we have enough leaves or hit min size.
    /// gap: number of cells reserved between children (for grid lines).
    pub fn split_with_gap(
        &mut self,
        min_w: usize,
        min_h: usize,
        max_depth: usize,
        gap: usize,
        rng: &mut StdRng,
    ) {
        if max_depth == 0 {
            return;
        }

        let can_split_h = self.rect.w >= min_w * 2 + gap;
        let can_split_v = self.rect.h >= min_h * 2 + gap;

        if !can_split_h && !can_split_v {
            return;
        }

        // bias split axis by aspect ratio: wide nodes split horizontally (left/right),
        // tall nodes split vertically (top/bottom)
        let split_horizontal = if !can_split_v {
            true
        } else if !can_split_h {
            false
        } else {
            let ratio = self.rect.w as f64 / self.rect.h as f64;
            // terminal cells are ~1:2 aspect, so adjust threshold
            if ratio > 1.5 {
                true
            } else if ratio < 0.75 {
                false
            } else {
                rng.random_range(0..2) == 0
            }
        };

        if split_horizontal {
            // split along x axis (left/right children)
            let range_lo = min_w;
            let range_hi = self.rect.w.saturating_sub(min_w + gap);
            if range_lo >= range_hi {
                return;
            }
            let split = rng.random_range(range_lo..range_hi);

            let mut left = Box::new(BspNode::new(self.rect.x, self.rect.y, split, self.rect.h));
            let mut right = Box::new(BspNode::new(
                self.rect.x + split + gap,
                self.rect.y,
                self.rect.w.saturating_sub(split + gap),
                self.rect.h,
            ));
            left.split_with_gap(min_w, min_h, max_depth - 1, gap, rng);
            right.split_with_gap(min_w, min_h, max_depth - 1, gap, rng);
            self.left = Some(left);
            self.right = Some(right);
        } else {
            // split along y axis (top/bottom children)
            let range_lo = min_h;
            let range_hi = self.rect.h.saturating_sub(min_h + gap);
            if range_lo >= range_hi {
                return;
            }
            let split = rng.random_range(range_lo..range_hi);

            let mut top = Box::new(BspNode::new(self.rect.x, self.rect.y, self.rect.w, split));
            let mut bottom = Box::new(BspNode::new(
                self.rect.x,
                self.rect.y + split + gap,
                self.rect.w,
                self.rect.h.saturating_sub(split + gap),
            ));
            top.split_with_gap(min_w, min_h, max_depth - 1, gap, rng);
            bottom.split_with_gap(min_w, min_h, max_depth - 1, gap, rng);
            self.left = Some(top);
            self.right = Some(bottom);
        }
    }

    /// Split with default gap of 1.
    pub fn split(&mut self, min_w: usize, min_h: usize, max_depth: usize, rng: &mut StdRng) {
        self.split_with_gap(min_w, min_h, max_depth, 1, rng);
    }

    /// Collect all leaf rects in traversal order.
    pub fn leaves(&self) -> Vec<&Rect> {
        if self.is_leaf() {
            return vec![&self.rect];
        }
        let mut out = Vec::new();
        if let Some(ref l) = self.left {
            out.extend(l.leaves());
        }
        if let Some(ref r) = self.right {
            out.extend(r.leaves());
        }
        out
    }
}

/// Two-column layout. Returns the rects placed so pattern fills can avoid them.
pub fn layout_two_col(
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
        if h == 0 {
            break;
        }
        let rect = Rect {
            x: left_x,
            y: cy,
            w: col_w,
            h,
        };
        render_block(grid, block, &rect, fg, bar_fg);
        rects.push(rect);
        cy += h + 1; // 1 row gap between blocks
    }

    // right column
    cy = margin;
    for block in right {
        let (_, h) = measure_block(block, col_w);
        let h = h.min(canvas_h.saturating_sub(cy));
        if h == 0 {
            break;
        }
        let rect = Rect {
            x: right_x,
            y: cy,
            w: col_w,
            h,
        };
        render_block(grid, block, &rect, fg, bar_fg);
        rects.push(rect);
        cy += h + 1;
    }

    rects
}

/// BSP layout: split canvas into regions, assign content blocks to leaves,
/// render blocks, return all leaf rects (content + empty pattern zones).
pub fn layout_bsp(
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
    let mut leaf_rects: Vec<Rect> = leaves
        .iter()
        .map(|r| Rect {
            x: r.x,
            y: r.y,
            w: r.w,
            h: r.h,
        })
        .collect();

    // sort by area descending so content goes in the biggest regions
    leaf_rects.sort_by(|a, b| (b.w * b.h).cmp(&(a.w * a.h)));

    let mut all_rects = Vec::new();
    for (i, block) in blocks.iter().enumerate() {
        if i >= leaf_rects.len() {
            break;
        }
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
        all_rects.push(Rect {
            x: r.x,
            y: r.y,
            w: r.w,
            h: r.h,
        });
    }

    all_rects
}
