use crossterm::style::Color;
use rand::rngs::StdRng;
use rand::RngExt;
use crate::types::*;
use crate::color::*;
use crate::sprites::{TreePen, MoveDir};

// ── Inputs ──────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub enum GrowDir { Up, UpLeft, UpRight, Outward }

pub struct TreeParams {
    pub plot: Rect,
    pub energy: f32,
    pub trunk_color: Color,
    pub bark_color: Color,
    pub branch_color: Color,
    pub tip_color: Color,
    pub fruit_color: Color,
    pub fruit_factor: f32,
    pub branch_factor: f32,
    pub direction: GrowDir,
}

impl TreeParams {
    pub fn root(&self) -> (i32, i32) {
        let x = self.plot.x as i32 + self.plot.w as i32 / 2;
        let y = self.plot.y as i32 + self.plot.h as i32 - 1;
        (x, y)
    }

    pub fn canopy_top(&self) -> i32 {
        let top = self.plot.y as i32;
        let ry = self.root().1;
        ry - ((ry - top) as f32 * self.energy.clamp(0.1, 1.0)) as i32
    }

    pub fn spread(&self) -> i32 {
        (self.plot.w as f32 / 2.0 * self.energy.clamp(0.2, 1.0)) as i32
    }

    pub fn color_at_depth(&self, frac: f32) -> Color {
        lighten(self.branch_color, (frac * 60.0) as u8)
    }
}

fn set(grid: &mut Grid, x: i32, y: i32, ch: char, fg: Color) {
    if x >= 0 && y >= 0 && (y as usize) < grid.len() && (x as usize) < grid[0].len() {
        grid[y as usize][x as usize] = Cell::new(ch, fg);
    }
}

// ── Outputs ─────────────────────────────────────────────────────────

pub struct TrunkNode {
    pub x: i32,
    pub y: i32,
    pub dir: MoveDir,
}

pub struct BranchResult {
    pub tips: Vec<(i32, i32)>,
}

pub struct BranchIntent {
    pub go_left: bool,
    pub length: i32,
    pub level: usize,
}

// ── Trait ────────────────────────────────────────────────────────────

pub trait TreeDrawer {
    fn draw_trunk(
        &self, grid: &mut Grid, pen: &mut TreePen,
        params: &TreeParams, rng: &mut StdRng,
    ) -> Vec<TrunkNode>;

    /// idx = trunk node index, count = total trunk nodes.
    fn should_branch(
        &self, idx: usize, count: usize,
        params: &TreeParams, rng: &mut StdRng,
    ) -> Option<BranchIntent>;

    fn draw_branch(
        &self, grid: &mut Grid, pen: &mut TreePen,
        intent: &BranchIntent, depth: usize,
        params: &TreeParams, rng: &mut StdRng,
    ) -> BranchResult;

    fn draw_tip(&self, grid: &mut Grid, x: i32, y: i32, params: &TreeParams);

    fn draw_fruit(&self, grid: &mut Grid, x: i32, y: i32, params: &TreeParams, rng: &mut StdRng);

    /// Default growth loop: trunk → branches at intervals → tips → fruit.
    fn grow(&self, grid: &mut Grid, params: &TreeParams, rng: &mut StdRng) {
        let (rx, ry) = params.root();
        let mut pen = TreePen::new(rx, ry, params.trunk_color);
        set(grid, rx, ry, '│', params.trunk_color);

        let trunk = self.draw_trunk(grid, &mut pen, params, rng);
        if trunk.is_empty() { return; }

        let trunk_len = trunk.len();
        let mut all_tips: Vec<(i32, i32)> = Vec::new();
        let mut apex_branched = false;

        for (i, node) in trunk.iter().enumerate() {
            if let Some(intent) = self.should_branch(i, trunk_len, params, rng) {
                // Pen at the trunk node -- draw_branch owns the junction and everything outward
                let mut bp = TreePen::new(node.x, node.y, params.trunk_color);
                bp.last_dir = Some(MoveDir::Up);

                let result = self.draw_branch(grid, &mut bp, &intent, 0, params, rng);
                all_tips.extend(result.tips);

                if i == trunk_len - 1 { apex_branched = true; }
            }
        }

        // Tip at trunk apex only if no branch was placed there
        if !apex_branched {
            if let Some(last) = trunk.last() {
                self.draw_tip(grid, last.x, last.y, params);
            }
        }

        // Tips and fruit
        for &(tx, ty) in &all_tips {
            self.draw_tip(grid, tx, ty, params);
            if rng.random::<f32>() < params.fruit_factor {
                self.draw_fruit(grid, tx, ty, params, rng);
            }
        }
    }
}

// ── SpiralTree ──────────────────────────────────────────────────────
// Tall straight trunk. Alternating branches at regular intervals.
// Stub-capped arms with hooks on lower branches.

pub struct SpiralTree;

impl TreeDrawer for SpiralTree {
    fn draw_trunk(
        &self, grid: &mut Grid, pen: &mut TreePen,
        params: &TreeParams, _rng: &mut StdRng,
    ) -> Vec<TrunkNode> {
        let top_y = params.canopy_top();
        let ry = params.root().1;
        let height = (ry - top_y).max(1);
        let mut path = Vec::with_capacity(height as usize);

        for _ in 0..height {
            pen.step(grid, MoveDir::Up);
            path.push(TrunkNode { x: pen.x, y: pen.y, dir: MoveDir::Up });
        }

        path
    }

    fn should_branch(
        &self, idx: usize, count: usize,
        params: &TreeParams, _rng: &mut StdRng,
    ) -> Option<BranchIntent> {
        let interval = (count / 5).max(2);
        if idx < interval || idx >= count - 1 { return None; }
        if idx % interval != 0 { return None; }

        let level = idx / interval - 1;
        let go_left = level % 2 == 0;

        let max_arm = params.spread();
        let arm = (max_arm - level as i32 * 2).max(2);

        Some(BranchIntent { go_left, length: arm, level })
    }

    fn draw_branch(
        &self, grid: &mut Grid, pen: &mut TreePen,
        intent: &BranchIntent, _depth: usize,
        params: &TreeParams, _rng: &mut StdRng,
    ) -> BranchResult {
        use MoveDir::*;
        let h_dir = if intent.go_left { Left } else { Right };
        let mut tips = Vec::new();

        // Color: lighten more near top (level 0 = lightest), matching old algo
        let c = lighten(params.trunk_color, 60u8.saturating_sub((intent.level * 15) as u8));
        pen.color = c;

        // Junction at trunk attachment point
        let jc = if intent.go_left { '┤' } else { '├' };
        set(grid, pen.x, pen.y, jc, c);

        // First horizontal cell
        pen.x += h_dir.dx();
        pen.last_dir = Some(h_dir);
        set(grid, pen.x, pen.y, '─', c);

        // Horizontal run
        for _ in 0..intent.length.saturating_sub(2) {
            pen.step(grid, h_dir);
        }

        // Stub cap at arm end
        let stub_x = pen.x + h_dir.dx();
        let stub_y = pen.y;
        let stub = if intent.go_left { '╴' } else { '╶' };
        set(grid, stub_x, stub_y, stub, c);

        // Hook for lower branches: corner turning up + tip one cell further out
        if intent.level < 3 {
            let corner = if intent.go_left { '╮' } else { '╭' };
            set(grid, stub_x, stub_y - 1, corner, c);
            let tip_x = stub_x + h_dir.dx();
            set(grid, tip_x, stub_y - 1, '╷', lighten(c, 25));
            tips.push((tip_x, stub_y - 1));
        }

        BranchResult { tips }
    }

    fn draw_tip(&self, grid: &mut Grid, x: i32, y: i32, params: &TreeParams) {
        set(grid, x, y, '╷', lighten(params.trunk_color, 50));
    }

    fn draw_fruit(&self, _grid: &mut Grid, _x: i32, _y: i32, _params: &TreeParams, _rng: &mut StdRng) {}
}

// ── CandelabraTree ──────────────────────────────────────────────────
// Short thick trunk (bottom 1/3). should_branch fires once at trunk top.
// draw_branch handles the entire crown: horizontal bar, vertical arms
// with corner-pair leans, two-way tip splits.

pub struct CandelabraTree;

impl TreeDrawer for CandelabraTree {
    fn draw_trunk(
        &self, grid: &mut Grid, pen: &mut TreePen,
        params: &TreeParams, _rng: &mut StdRng,
    ) -> Vec<TrunkNode> {
        let top_y = params.canopy_top();
        let ry = params.root().1;
        let height = (ry - top_y).max(3);
        let trunk_h = height / 3;
        let bark = darken(params.trunk_color, 15);
        let mut path = Vec::with_capacity(trunk_h as usize);

        // Thick trunk: ┃ center flanked by │
        for _ in 0..trunk_h {
            pen.step(grid, MoveDir::Up);
            set(grid, pen.x, pen.y, '┃', params.trunk_color);
            set(grid, pen.x - 1, pen.y, '│', bark);
            set(grid, pen.x + 1, pen.y, '│', bark);
            path.push(TrunkNode { x: pen.x, y: pen.y, dir: MoveDir::Up });
        }

        path
    }

    fn should_branch(
        &self, idx: usize, count: usize,
        _params: &TreeParams, _rng: &mut StdRng,
    ) -> Option<BranchIntent> {
        // Fire once at the trunk top -- draw_branch builds the whole crown
        if idx == count - 1 {
            Some(BranchIntent { go_left: false, length: 0, level: 0 })
        } else {
            None
        }
    }

    fn draw_branch(
        &self, grid: &mut Grid, _pen: &mut TreePen,
        _intent: &BranchIntent, _depth: usize,
        params: &TreeParams, rng: &mut StdRng,
    ) -> BranchResult {
        let (rx, _) = params.root();
        let top_y = params.canopy_top();
        let ry = params.root().1;
        let height = (ry - top_y).max(3);
        let split_y = ry - height / 3;
        let arm_count = rng.random_range(3..6usize);
        let total_spread = params.spread();
        let bar_color = darken(params.trunk_color, 10);
        let arm_color = lighten(params.trunk_color, 20);
        let tip_c = lighten(arm_color, 30);
        let mut tips = Vec::new();

        // Horizontal connector bar at split
        let start_x = rx - total_spread;
        let end_x = rx + total_spread;
        for x in start_x..=end_x {
            set(grid, x, split_y, '─', bar_color);
        }
        set(grid, rx, split_y, '┬', params.trunk_color);

        // Arms: evenly spaced along bar
        let step = (total_spread * 2) / (arm_count as i32 - 1).max(1);

        for i in 0..arm_count {
            let ax = start_x + i as i32 * step;

            // Junction char at bar
            let jc = if i == 0 { '└' } else if i == arm_count - 1 { '┘' } else { '┴' };
            set(grid, ax, split_y, jc, params.trunk_color);

            // Lean direction: arms left of center lean left, right lean right
            let lean: i32 = if ax < rx { -1 } else if ax > rx { 1 } else { 0 };
            let arm_top = top_y + rng.random_range(0..3u32) as i32;

            // Vertical arm with corner-pair lean at midpoint
            let mut cx = ax;
            let mid_y = (arm_top + split_y) / 2;
            for y in (arm_top..split_y).rev() {
                set(grid, cx, y, '│', arm_color);
                if y == mid_y && lean != 0 {
                    if lean < 0 {
                        set(grid, cx, y, '╮', arm_color);
                        set(grid, cx - 1, y, '╰', arm_color);
                    } else {
                        set(grid, cx, y, '╭', arm_color);
                        set(grid, cx + 1, y, '╯', arm_color);
                    }
                    cx += lean;
                }
            }

            // Two-way tip split at arm top
            set(grid, cx, arm_top, '┤', tip_c);
            set(grid, cx - 1, arm_top, '─', tip_c);
            set(grid, cx - 2, arm_top, '╷', tip_c);
            set(grid, cx, arm_top, '├', tip_c);
            set(grid, cx + 1, arm_top, '─', tip_c);
            set(grid, cx + 2, arm_top, '╷', tip_c);

            tips.push((cx - 2, arm_top));
            tips.push((cx + 2, arm_top));
        }

        BranchResult { tips }
    }

    fn draw_tip(&self, grid: &mut Grid, x: i32, y: i32, params: &TreeParams) {
        set(grid, x, y, '╷', lighten(params.tip_color, 30));
    }

    fn draw_fruit(&self, _grid: &mut Grid, _x: i32, _y: i32, _params: &TreeParams, _rng: &mut StdRng) {}
}

// ── SplitTree ───────────────────────────────────────────────────────
// Short wobble trunk (bottom 1/3). should_branch fires once at trunk top.
// draw_branch does recursive binary subdivision: each segment picks
// an off-center split point and forks left/right. Max depth 4.

pub struct SplitTree;

impl TreeDrawer for SplitTree {
    fn draw_trunk(
        &self, grid: &mut Grid, pen: &mut TreePen,
        params: &TreeParams, rng: &mut StdRng,
    ) -> Vec<TrunkNode> {
        let top_y = params.canopy_top();
        let ry = params.root().1;
        let height = (ry - top_y).max(3);
        let trunk_h = (height / 3).max(2);
        let freq = rng.random_range(3..6u32) as i32;
        let mut path = Vec::with_capacity(trunk_h as usize);

        // Wobble trunk: mostly │ with occasional ╱╲ lateral shifts
        for i in 0..trunk_h {
            if i > 0 && i % freq == 0 && rng.random_range(0..3u32) == 0 {
                // 2-column curve: corner out, step sideways, corner back up
                let lean: i32 = if rng.random::<bool>() { -1 } else { 1 };
                let (corner_out, corner_in) = if lean > 0 {
                    ('╰', '╭')
                } else {
                    ('╯', '╮')
                };
                set(grid, pen.x, pen.y, corner_out, pen.color);
                pen.x += lean;
                set(grid, pen.x, pen.y, corner_in, pen.color);
                pen.last_dir = Some(MoveDir::Up);
                pen.step(grid, MoveDir::Up);
            } else {
                pen.step(grid, MoveDir::Up);
            }
            path.push(TrunkNode { x: pen.x, y: pen.y, dir: MoveDir::Up });
        }

        path
    }

    fn should_branch(
        &self, idx: usize, count: usize,
        _params: &TreeParams, _rng: &mut StdRng,
    ) -> Option<BranchIntent> {
        // Fire once at the trunk top -- draw_branch does recursive forking
        if idx == count - 1 {
            Some(BranchIntent { go_left: false, length: 0, level: 0 })
        } else {
            None
        }
    }

    fn draw_branch(
        &self, grid: &mut Grid, _pen: &mut TreePen,
        _intent: &BranchIntent, _depth: usize,
        params: &TreeParams, rng: &mut StdRng,
    ) -> BranchResult {
        let (rx, ry) = params.root();
        let top_y = params.canopy_top();
        let height = (ry - top_y).max(3);
        let first_split = ry - (height / 3).max(2);
        let spread = params.spread();
        let mut tips = Vec::new();

        // BFS queue: (x, top_y, bottom_y, depth)
        let mut queue: Vec<(i32, i32, i32, usize)> = vec![(rx, top_y, first_split, 0)];
        let max_depth = 4usize;

        while let Some((x, top, bottom, depth)) = queue.pop() {
            let branch_color = match depth {
                0 => params.trunk_color,
                1 => lighten(params.trunk_color, 20),
                2 => lighten(params.trunk_color, 40),
                _ => lighten(params.trunk_color, 60),
            };

            // Terminal: too deep or segment too short
            if depth >= max_depth || bottom <= top + 1 {
                for y in top..bottom {
                    set(grid, x, y, '│', branch_color);
                }
                tips.push((x, top));
                continue;
            }

            // Off-center split: 30-70% of segment height
            let split_frac = 30 + rng.random_range(0..41u32) as i32;
            let split_y = (top + (bottom - top) * split_frac / 100).max(top + 1).min(bottom - 1);

            // Vertical segment below split
            for y in (split_y + 1)..bottom {
                set(grid, x, y, '│', branch_color);
            }

            // Independent left/right arm lengths, halving with depth
            let base_arm = (spread >> depth as u32).max(2);
            let left_arm = (base_arm * rng.random_range(50..150u32) as i32 / 100).max(1);
            let right_arm = (base_arm * rng.random_range(50..150u32) as i32 / 100).max(1);
            let left_x = x - left_arm;
            let right_x = x + right_arm;

            // Horizontal bar: ╭───┼───╮
            set(grid, x, split_y, '┤', branch_color);

            // Left arm
            set(grid, left_x, split_y, '╭', branch_color);
            for ax in (left_x + 1)..x {
                set(grid, ax, split_y, '─', branch_color);
            }

            // Right arm (overwrites junction to ┼)
            set(grid, x, split_y, '┼', branch_color);
            for ax in (x + 1)..right_x {
                set(grid, ax, split_y, '─', branch_color);
            }
            set(grid, right_x, split_y, '╮', branch_color);

            queue.push((left_x, top, split_y, depth + 1));
            queue.push((right_x, top, split_y, depth + 1));
        }

        BranchResult { tips }
    }

    fn draw_tip(&self, grid: &mut Grid, x: i32, y: i32, params: &TreeParams) {
        set(grid, x, y, '╷', lighten(params.tip_color, 30));
    }

    fn draw_fruit(&self, _grid: &mut Grid, _x: i32, _y: i32, _params: &TreeParams, _rng: &mut StdRng) {}
}

// ── BirchTree ───────────────────────────────────────────────────
// Tall straight trunk. Branches alternate left/right at interval=2.
// 25% chance to skip a branch. Short arms (2-6 cells) with corner caps
// and diagonal spray tips.

pub struct BirchTree;

impl TreeDrawer for BirchTree {
    fn draw_trunk(
        &self, grid: &mut Grid, pen: &mut TreePen,
        params: &TreeParams, _rng: &mut StdRng,
    ) -> Vec<TrunkNode> {
        let top_y = params.canopy_top();
        let ry = params.root().1;
        let height = (ry - top_y).max(1);
        let mut path = Vec::with_capacity(height as usize);

        for _ in 0..height {
            pen.step(grid, MoveDir::Up);
            path.push(TrunkNode { x: pen.x, y: pen.y, dir: MoveDir::Up });
        }

        path
    }

    fn should_branch(
        &self, idx: usize, count: usize,
        params: &TreeParams, rng: &mut StdRng,
    ) -> Option<BranchIntent> {
        let interval = 2;

        // Skip first and last node
        if idx == 0 || idx >= count - 1 { return None; }

        // Branch at interval=2, alternating left/right
        if idx % interval != 0 { return None; }

        // 25% chance to skip this branch
        if rng.random_range(0..4u32) == 0 { return None; }

        let level = idx / interval - 1;
        let go_left = level % 2 == 0;
        let max_arm = params.spread().max(2).min(6);
        let length = rng.random_range(2..=max_arm);

        Some(BranchIntent { go_left, length, level })
    }

    fn draw_branch(
        &self, grid: &mut Grid, pen: &mut TreePen,
        intent: &BranchIntent, _depth: usize,
        params: &TreeParams, rng: &mut StdRng,
    ) -> BranchResult {
        use MoveDir::*;
        let h_dir = if intent.go_left { Left } else { Right };
        let mut tips = Vec::new();

        // Random lighten per branch: 10-50
        let c = lighten(params.branch_color, rng.random_range(10..50u8) as u8);
        pen.color = c;

        // Junction at trunk attachment
        let jc = if intent.go_left { '┤' } else { '├' };
        set(grid, pen.x, pen.y, jc, c);

        // Horizontal arm
        for i in 1..intent.length {
            pen.x += h_dir.dx();
            pen.last_dir = Some(h_dir);
            set(grid, pen.x, pen.y, '─', c);
        }

        // Corner cap at arm end
        let corner = if intent.go_left { '╮' } else { '╭' };
        pen.x += h_dir.dx();
        set(grid, pen.x, pen.y, corner, c);

        // Spray tips: one cell diagonally up from corner
        let spray_y = pen.y - 1;
        let spray_light = lighten(c, 20);
        set(grid, pen.x, spray_y, '╷', spray_light);
        tips.push((pen.x, spray_y));

        // Second spray tip if arm > 2
        if intent.length > 2 {
            let second_x = pen.x - h_dir.dx();
            let second_light = lighten(c, 10);
            set(grid, second_x, spray_y, '╷', second_light);
            tips.push((second_x, spray_y));
        }

        BranchResult { tips }
    }

    fn draw_tip(&self, grid: &mut Grid, x: i32, y: i32, params: &TreeParams) {
        set(grid, x, y, '╷', lighten(params.tip_color, 60));
    }

    fn draw_fruit(&self, _grid: &mut Grid, _x: i32, _y: i32, _params: &TreeParams, _rng: &mut StdRng) {}
}

// ── StormTree ───────────────────────────────────────────────────
// Leaning trunk (diagonal shifts at regular intervals). Branches
// peel off windward side with progressive lighten. Compact, windy look.

use std::cell::Cell as StdCell;

pub struct StormTree {
    /// Lean direction picked during draw_trunk, consumed by draw_branch.
    /// +1 = lean right (branches go left), -1 = lean left (branches go right).
    lean: StdCell<i32>,
}

impl StormTree {
    pub fn new() -> Self {
        StormTree { lean: StdCell::new(0) }
    }
}

impl TreeDrawer for StormTree {
    fn draw_trunk(
        &self, grid: &mut Grid, pen: &mut TreePen,
        params: &TreeParams, rng: &mut StdRng,
    ) -> Vec<TrunkNode> {
        let top_y = params.canopy_top();
        let ry = params.root().1;
        let height = (ry - top_y).max(1);
        let spread = params.spread().max(2);
        let lean: i32 = if rng.random_range(0..2u32) == 0 { 1 } else { -1 };
        self.lean.set(lean);
        let lean_every = (height / (spread.min(8))).max(2);
        let mut path = Vec::with_capacity(height as usize);

        let mut shifts = 0i32;
        for y in (top_y..=ry).rev() {
            let rows_from_root = ry - y;
            let new_shifts = rows_from_root / lean_every;

            if new_shifts > shifts {
                shifts = new_shifts;
                // 2-column curve transition instead of single-cell diagonal:
                // Draw corner at current pos turning horizontal, step sideways, then corner turning back up
                let (corner_out, corner_in) = if lean > 0 {
                    ('╰', '╭')  // turn right then back up
                } else {
                    ('╯', '╮')  // turn left then back up
                };
                let h_dir = if lean > 0 { MoveDir::Right } else { MoveDir::Left };
                // Corner out: current cell turns from vertical to horizontal
                set(grid, pen.x, pen.y, corner_out, pen.color);
                pen.x += h_dir.dx();
                set(grid, pen.x, pen.y, corner_in, pen.color);
                pen.last_dir = Some(MoveDir::Up);
                // Continue upward from new x
                pen.step(grid, MoveDir::Up);
            } else {
                pen.step(grid, MoveDir::Up);
            }

            path.push(TrunkNode { x: pen.x, y: pen.y, dir: MoveDir::Up });
        }

        path
    }

    fn should_branch(
        &self, idx: usize, count: usize,
        params: &TreeParams, _rng: &mut StdRng,
    ) -> Option<BranchIntent> {
        if count < 4 { return None; }

        let height = params.root().1 - params.canopy_top();
        let interval = (height / 4).max(2);

        // idx 0 = nearest root, idx count-1 = apex
        let distance_from_root = count as i32 - 1 - idx as i32;

        if distance_from_root < 2 { return None; }
        if (distance_from_root - 2) % interval != 0 { return None; }

        let level = ((distance_from_root - 2) / interval) as usize;
        let max_spread = params.spread();
        let arm = (max_spread - level as i32 * 2).max(2);

        // go_left encodes windward side (opposite lean)
        let go_left = self.lean.get() > 0;

        Some(BranchIntent { go_left, length: arm, level })
    }

    fn draw_branch(
        &self, grid: &mut Grid, pen: &mut TreePen,
        intent: &BranchIntent, _depth: usize,
        params: &TreeParams, _rng: &mut StdRng,
    ) -> BranchResult {
        use MoveDir::*;

        let c = lighten(params.trunk_color, (intent.level * 20) as u8);
        let h_dir = if intent.go_left { Left } else { Right };

        let mut tips = Vec::new();
        let jc = if intent.go_left { '┤' } else { '├' };
        set(grid, pen.x, pen.y, jc, c);

        // Horizontal run
        let arm = intent.length;
        for i in 1..=arm {
            let nx = pen.x + h_dir.dx() * i;
            set(grid, nx, pen.y, '─', c);
        }

        // Corner curl at arm tip
        let tip_x = pen.x + h_dir.dx() * arm;
        let curl = if intent.go_left { '╮' } else { '╭' };
        set(grid, tip_x, pen.y, curl, c);
        set(grid, tip_x + h_dir.dx(), pen.y - 1, '╷', lighten(c, 25));
        tips.push((tip_x + h_dir.dx(), pen.y - 1));

        BranchResult { tips }
    }

    fn draw_tip(&self, grid: &mut Grid, x: i32, y: i32, params: &TreeParams) {
        set(grid, x, y, '╷', lighten(params.trunk_color, 55));
    }

    fn draw_fruit(&self, _grid: &mut Grid, _x: i32, _y: i32, _params: &TreeParams, _rng: &mut StdRng) {}
}

// ── DeadTree ────────────────────────────────────────────────────────
// Skeletal, eerie tree with gnarled trunk and sparse angular branches.
// Gnarled trunk: mostly │ with random diagonal offsets (╱/╲) every 7 rows.
// Sparse branches: diagonal then horizontal, alternating left/right.
// Leans progressively lighter; tips use a cycling char set.

pub struct DeadTree;

impl TreeDrawer for DeadTree {
    fn draw_trunk(
        &self, grid: &mut Grid, pen: &mut TreePen,
        params: &TreeParams, rng: &mut StdRng,
    ) -> Vec<TrunkNode> {
        let top_y = params.canopy_top();
        let ry = params.root().1;
        let height = (ry - top_y).max(1);
        let trunk_color = darken(params.trunk_color, 10);
        let mut path = Vec::with_capacity(height as usize);

        pen.color = trunk_color;
        for i in 0..height {
            let from_root = height - i;
            // Every 7 rows with 33% chance: random diagonal offset
            if from_root > 2 && from_root % 7 == 0 && rng.random_range(0..3u32) == 0 {
                let lean = if rng.random::<bool>() { -1 } else { 1 };
                // 2-column curve: corner out, step sideways, corner back up
                let (corner_out, corner_in) = if lean > 0 {
                    ('╰', '╭')
                } else {
                    ('╯', '╮')
                };
                let h_dir = if lean > 0 { MoveDir::Right } else { MoveDir::Left };
                set(grid, pen.x, pen.y, corner_out, pen.color);
                pen.x += h_dir.dx();
                set(grid, pen.x, pen.y, corner_in, pen.color);
                pen.last_dir = Some(MoveDir::Up);
                pen.step(grid, MoveDir::Up);
            } else {
                pen.step(grid, MoveDir::Up);
            }
            path.push(TrunkNode { x: pen.x, y: pen.y, dir: MoveDir::Up });
        }

        path
    }

    fn should_branch(
        &self, idx: usize, count: usize,
        params: &TreeParams, rng: &mut StdRng,
    ) -> Option<BranchIntent> {
        // Sparse branches: ~5-6 branches evenly spaced
        let interval = (count / 6).max(2);
        if idx < interval || idx >= count - 1 { return None; }
        if idx % interval != 0 { return None; }

        let level = idx / interval;
        let go_left = level % 2 == 0;
        let max_arm = params.spread().max(2).min(8);
        let length = rng.random_range(2..=max_arm);

        Some(BranchIntent { go_left, length, level })
    }

    fn draw_branch(
        &self, grid: &mut Grid, _pen: &mut TreePen,
        intent: &BranchIntent, _depth: usize,
        params: &TreeParams, _rng: &mut StdRng,
    ) -> BranchResult {
        use MoveDir::*;
        let tip_chars = ['╴', '╶', '·', '╷'];
        let mut tips = Vec::new();

        // Branch color lightens with level
        let c = lighten(params.branch_color, (intent.level as u8 * 12).min(60));
        let h_dir = if intent.go_left { Left } else { Right };

        // Start at pen position (trunk attachment)
        let mut bx = _pen.x;
        let mut yy = _pen.y;

        // Junction char at trunk
        let jc = if intent.go_left { '┐' } else { '┌' };
        set(grid, bx, yy, jc, c);

        // Diagonal segment (up at an angle)
        let arm = intent.length;
        let diag_len = (arm / 3).max(1);
        let horiz_len = arm - diag_len;
        let diag_ch = if intent.go_left { '╲' } else { '╱' };

        for _ in 0..diag_len {
            bx += h_dir.dx();
            yy -= 1;
            set(grid, bx, yy, diag_ch, c);
        }

        // Horizontal segment
        for _ in 0..horiz_len {
            bx += h_dir.dx();
            set(grid, bx, yy, '─', c);
        }

        // Tip char (cycle through set)
        let tip = tip_chars[intent.level % tip_chars.len()];
        set(grid, bx + h_dir.dx(), yy, tip, lighten(c, 20));
        tips.push((bx + h_dir.dx(), yy));

        // Sub-twig for longer arms
        if arm > 3 {
            set(grid, bx, yy - 1, '╷', lighten(c, 30));
        }

        BranchResult { tips }
    }

    fn draw_tip(&self, grid: &mut Grid, x: i32, y: i32, params: &TreeParams) {
        set(grid, x, y, '╷', lighten(params.tip_color, 30));
    }

    fn draw_fruit(&self, _grid: &mut Grid, _x: i32, _y: i32, _params: &TreeParams, _rng: &mut StdRng) {}
}

// ── DroopingTree ────────────────────────────────────────────────
// Short straight trunk (bottom 2/3). should_branch fires once at trunk top.
// draw_branch handles the entire crown: fan of 3-6 arms spread across width.
// Each arm has a horizontal bar from center, vertical rise, then drooping
// horizontal arms with hanging drips (╎) extending downward.

pub struct DroopingTree;

impl TreeDrawer for DroopingTree {
    fn draw_trunk(
        &self, grid: &mut Grid, pen: &mut TreePen,
        params: &TreeParams, _rng: &mut StdRng,
    ) -> Vec<TrunkNode> {
        let top_y = params.canopy_top();
        let ry = params.root().1;
        let height = (ry - top_y).max(3);
        let trunk_h = (height * 2) / 3;
        let mut path = Vec::with_capacity(trunk_h as usize);

        // Straight trunk: bottom 2/3
        for _ in 0..trunk_h {
            pen.step(grid, MoveDir::Up);
            path.push(TrunkNode { x: pen.x, y: pen.y, dir: MoveDir::Up });
        }

        path
    }

    fn should_branch(
        &self, idx: usize, count: usize,
        _params: &TreeParams, _rng: &mut StdRng,
    ) -> Option<BranchIntent> {
        // Fire once at the trunk top -- draw_branch builds the whole crown
        if idx == count - 1 {
            Some(BranchIntent { go_left: false, length: 0, level: 0 })
        } else {
            None
        }
    }

    fn draw_branch(
        &self, grid: &mut Grid, _pen: &mut TreePen,
        _intent: &BranchIntent, _depth: usize,
        params: &TreeParams, rng: &mut StdRng,
    ) -> BranchResult {
        let (rx, ry) = params.root();
        let top_y = params.canopy_top();
        let height = (ry - top_y).max(3);
        let first_split = ry - (height / 3);
        let spread = params.spread();
        let arm_count = rng.random_range(3..6usize);
        let bar_color = lighten(params.trunk_color, 10);
        let mut tips = Vec::new();

        // Fan of arms: evenly distributed across width
        for i in 0..arm_count {
            let t = if arm_count > 1 {
                i as f32 / (arm_count - 1) as f32
            } else {
                0.5
            };
            let arm_x_offset = ((t * 2.0 - 1.0) * spread as f32) as i32;
            let bx = rx + arm_x_offset;
            let arm_top_y = top_y + rng.random_range(0..4u32) as i32;
            let c = lighten(params.trunk_color, (i * 15) as u8);

            // Horizontal segment from trunk center to arm x at first_split
            if arm_x_offset != 0 {
                let (x0, x1) = if arm_x_offset < 0 {
                    (bx, rx)
                } else {
                    (rx, bx)
                };
                for x in x0..=x1 {
                    set(grid, x, first_split, '─', bar_color);
                }
                let corner = if arm_x_offset < 0 { '╭' } else { '╮' };
                set(grid, bx, first_split, corner, bar_color);
                set(grid, rx, first_split, '┼', bar_color);
            } else {
                set(grid, rx, first_split, '│', bar_color);
            }

            // Vertical rise from first_split to arm_top_y
            for y in arm_top_y..first_split {
                set(grid, bx, y, '│', c);
            }

            // Drooping feature: horizontal arms hanging at arm_top_y + 1
            let droop_arm = (spread / 3).max(1);
            if arm_top_y + 2 < first_split {
                let droop_y = arm_top_y + 1;
                let dc = lighten(c, 20);

                // Hanging arms to left and right
                for dx in 1..=droop_arm {
                    set(grid, bx - dx, droop_y, '─', dc);
                    set(grid, bx + dx, droop_y, '─', dc);
                }

                // Corner caps at droop endpoints
                set(grid, bx - droop_arm, droop_y, '╮', dc);
                set(grid, bx + droop_arm, droop_y, '╭', dc);
                set(grid, bx, droop_y, '┬', dc);

                // Hanging drips (╎) extending 3 cells down from endpoints
                for d in 1..=3 {
                    let dc2 = lighten(dc, (d * 15) as u8);
                    set(grid, bx - droop_arm, droop_y + d, '╎', dc2);
                    set(grid, bx + droop_arm, droop_y + d, '╎', dc2);
                }
            }

            // Tip at arm top
            tips.push((bx, arm_top_y));
        }

        BranchResult { tips }
    }

    fn draw_tip(&self, grid: &mut Grid, x: i32, y: i32, params: &TreeParams) {
        set(grid, x, y, '╷', lighten(params.tip_color, 40));
    }

    fn draw_fruit(&self, _grid: &mut Grid, _x: i32, _y: i32, _params: &TreeParams, _rng: &mut StdRng) {}
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use crossterm::style::Color;

    fn make_grid(w: usize, h: usize) -> Grid {
        vec![vec![Cell::new(' ', Color::Reset); w]; h]
    }

    fn grid_to_string(grid: &Grid) -> String {
        grid.iter()
            .map(|row| row.iter().map(|c| c.ch).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn test_params(plot_x: usize, plot_y: usize, plot_w: usize, plot_h: usize) -> TreeParams {
        let green = Color::Rgb { r: 80, g: 140, b: 60 };
        TreeParams {
            plot: Rect { x: plot_x, y: plot_y, w: plot_w, h: plot_h },
            energy: 0.9,
            trunk_color: green,
            bark_color: green,
            branch_color: green,
            tip_color: green,
            fruit_color: green,
            fruit_factor: 0.0,
            branch_factor: 0.7,
            direction: GrowDir::Up,
        }
    }

    #[test]
    fn snapshot_spiral_tree() {
        let mut grid = make_grid(40, 20);
        let tp = test_params(10, 1, 20, 18);
        let mut rng = StdRng::seed_from_u64(42);
        SpiralTree.grow(&mut grid, &tp, &mut rng);
        insta::assert_snapshot!("spiral_tree_42", grid_to_string(&grid));
    }

    #[test]
    fn snapshot_candelabra_tree() {
        let mut grid = make_grid(40, 20);
        let tp = test_params(10, 1, 20, 18);
        let mut rng = StdRng::seed_from_u64(42);
        CandelabraTree.grow(&mut grid, &tp, &mut rng);
        insta::assert_snapshot!("candelabra_tree_42", grid_to_string(&grid));
    }

    #[test]
    fn snapshot_split_tree() {
        let mut grid = make_grid(40, 20);
        let tp = test_params(10, 1, 20, 18);
        let mut rng = StdRng::seed_from_u64(42);
        SplitTree.grow(&mut grid, &tp, &mut rng);
        insta::assert_snapshot!("split_tree_42", grid_to_string(&grid));
    }

    #[test]
    fn snapshot_birch_tree() {
        let mut grid = make_grid(40, 20);
        let tp = test_params(10, 1, 20, 18);
        let mut rng = StdRng::seed_from_u64(42);
        BirchTree.grow(&mut grid, &tp, &mut rng);
        insta::assert_snapshot!("birch_tree_42", grid_to_string(&grid));
    }

    #[test]
    fn snapshot_storm_tree() {
        let mut grid = make_grid(40, 20);
        let tp = test_params(10, 1, 20, 18);
        let mut rng = StdRng::seed_from_u64(42);
        StormTree::new().grow(&mut grid, &tp, &mut rng);
        insta::assert_snapshot!("storm_tree_42", grid_to_string(&grid));
    }

    #[test]
    fn snapshot_dead_tree() {
        let mut grid = make_grid(40, 20);
        let tp = test_params(10, 1, 20, 18);
        let mut rng = StdRng::seed_from_u64(42);
        DeadTree.grow(&mut grid, &tp, &mut rng);
        insta::assert_snapshot!("dead_tree_42", grid_to_string(&grid));
    }

    #[test]
    fn snapshot_drooping_tree() {
        let mut grid = make_grid(40, 20);
        let tp = test_params(10, 1, 20, 18);
        let mut rng = StdRng::seed_from_u64(42);
        DroopingTree.grow(&mut grid, &tp, &mut rng);
        insta::assert_snapshot!("drooping_tree_42", grid_to_string(&grid));
    }
}
