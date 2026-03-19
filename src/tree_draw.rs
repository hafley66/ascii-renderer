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
    /// Trees with exotic structure override this.
    fn grow(&self, grid: &mut Grid, params: &TreeParams, rng: &mut StdRng) {
        let (rx, ry) = params.root();
        let mut pen = TreePen::new(rx, ry, params.trunk_color);
        set(grid, rx, ry, '│', params.trunk_color);

        let trunk = self.draw_trunk(grid, &mut pen, params, rng);
        if trunk.is_empty() { return; }

        let trunk_len = trunk.len();
        let mut all_tips: Vec<(i32, i32)> = Vec::new();

        for (i, node) in trunk.iter().enumerate() {
            if let Some(intent) = self.should_branch(i, trunk_len, params, rng) {
                let frac = i as f32 / trunk_len as f32;
                let bc = params.color_at_depth(frac);

                // Junction at trunk attachment
                let jc = if intent.go_left { '┤' } else { '├' };
                set(grid, node.x, node.y, jc, bc);

                // Branch pen starts one cell out from trunk
                let h_dir = if intent.go_left { MoveDir::Left } else { MoveDir::Right };
                let mut bp = TreePen::new(node.x + h_dir.dx(), node.y, bc);
                bp.last_dir = Some(h_dir);
                set(grid, bp.x, bp.y, '─', bc);

                let result = self.draw_branch(grid, &mut bp, &intent, 0, params, rng);
                all_tips.extend(result.tips);
            }
        }

        // Tip at trunk apex
        if let Some(last) = trunk.last() {
            self.draw_tip(grid, last.x, last.y, params);
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
// Curl-up tips with secondary twigs. Uses default growth loop.

pub struct SpiralTree;

impl TreeDrawer for SpiralTree {
    fn draw_trunk(
        &self, grid: &mut Grid, pen: &mut TreePen,
        params: &TreeParams, _rng: &mut StdRng,
    ) -> Vec<TrunkNode> {
        // Ruler-straight vertical. That IS this tree's personality.
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
        // Branch every `interval` trunk nodes, starting one interval in.
        // Same logic as old grow_spiral_tree: (height / 5).max(2)
        let interval = (count / 5).max(2);

        // Skip first interval (near root) and last node (apex)
        if idx < interval || idx >= count - 1 { return None; }
        if idx % interval != 0 { return None; }

        let level = idx / interval - 1; // 0-indexed branch level
        let go_left = level % 2 == 0;

        // Arms shrink higher up, just like old algo
        let max_arm = params.spread();
        let arm = (max_arm - level as i32 * 2).max(2);

        Some(BranchIntent { go_left, length: arm })
    }

    fn draw_branch(
        &self, grid: &mut Grid, pen: &mut TreePen,
        intent: &BranchIntent, _depth: usize,
        params: &TreeParams, _rng: &mut StdRng,
    ) -> BranchResult {
        use MoveDir::*;
        let h_dir = if intent.go_left { Left } else { Right };
        let mut tips = Vec::new();

        // Horizontal run
        for _ in 1..intent.length {
            pen.step(grid, h_dir);
        }

        // Curl-up: diagonal outward then vertical
        let energy_steps = (params.energy * 3.0).max(1.0) as usize;
        if energy_steps >= 2 {
            let curl_dir = if intent.go_left { UpLeft } else { UpRight };
            pen.step(grid, curl_dir);
            for _ in 0..energy_steps.saturating_sub(1) {
                pen.step(grid, Up);
            }
            tips.push((pen.x, pen.y));

            // Secondary twig from the curl elbow
            if intent.length > 3 && params.energy > 0.4 {
                let elbow_x = pen.x - curl_dir.dx();
                let elbow_y = pen.y + 1;
                let tc = lighten(pen.color, 15);
                let mut twig = TreePen::new(elbow_x, elbow_y, tc);
                twig.last_dir = Some(h_dir);
                twig.step(grid, h_dir);
                tips.push((twig.x, twig.y));
            }
        } else {
            tips.push((pen.x, pen.y));
        }

        BranchResult { tips }
    }

    fn draw_tip(&self, grid: &mut Grid, x: i32, y: i32, params: &TreeParams) {
        set(grid, x, y, '╷', lighten(params.tip_color, 25));
    }

    fn draw_fruit(&self, grid: &mut Grid, x: i32, y: i32, params: &TreeParams, rng: &mut StdRng) {
        let fruits = ['●', '◆', '◈', '✦'];
        let ch = fruits[rng.random_range(0..fruits.len() as u32) as usize];
        set(grid, x, y + 1, ch, params.fruit_color);
    }
}
