//! Classic species drawers (spiral..palm).
use super::*;
pub struct SpiralTree;
impl TreeDrawer for SpiralTree {
    fn draw_trunk(
        &self,
        grid: &mut Grid,
        pen: &mut TreePen,
        params: &TreeParams,
        rng: &mut StdRng,
    ) -> Vec<TrunkNode> {
        StraightTrunk {
            height_fraction: 1.0,
        }
        .draw(grid, pen, params, rng)
    }

    fn should_branch(
        &self,
        idx: usize,
        count: usize,
        params: &TreeParams,
        _rng: &mut StdRng,
    ) -> Option<BranchIntent> {
        let interval = (count / 5).max(2);
        if idx < interval || idx >= count - 1 {
            return None;
        }
        if idx % interval != 0 {
            return None;
        }

        let level = idx / interval - 1;
        let go_left = level % 2 == 0;

        let max_arm = params.spread();
        let arm = (max_arm - level as i32 * 2).max(2);

        Some(BranchIntent {
            go_left,
            length: arm,
            level,
        })
    }

    fn draw_branch(
        &self,
        grid: &mut Grid,
        pen: &mut TreePen,
        intent: &BranchIntent,
        _depth: usize,
        params: &TreeParams,
        _rng: &mut StdRng,
    ) -> BranchResult {
        use MoveDir::*;
        let h_dir = if intent.go_left { Left } else { Right };
        let mut tips = Vec::new();

        // Color: lighten more near top (level 0 = lightest), matching old algo
        let c = lighten(
            params.trunk_color,
            60u8.saturating_sub((intent.level * 15) as u8),
        );
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

    fn draw_fruit(&self, grid: &mut Grid, x: i32, y: i32, params: &TreeParams, _rng: &mut StdRng) {
        // Apple: stem above, round fruit at tip
        set(grid, x, y - 1, '╷', lighten(params.fruit_color, 40));
        set(grid, x, y, '●', params.fruit_color);
    }
}

pub struct CandelabraTree;
impl TreeDrawer for CandelabraTree {
    fn draw_trunk(
        &self,
        grid: &mut Grid,
        pen: &mut TreePen,
        params: &TreeParams,
        rng: &mut StdRng,
    ) -> Vec<TrunkNode> {
        ThickTrunk {
            height_fraction: 1.0 / 3.0,
        }
        .draw(grid, pen, params, rng)
    }

    fn should_branch(
        &self,
        idx: usize,
        count: usize,
        _params: &TreeParams,
        _rng: &mut StdRng,
    ) -> Option<BranchIntent> {
        // Fire once at the trunk top -- draw_branch builds the whole crown
        if idx == count - 1 {
            Some(BranchIntent {
                go_left: false,
                length: 0,
                level: 0,
            })
        } else {
            None
        }
    }

    fn draw_branch(
        &self,
        grid: &mut Grid,
        _pen: &mut TreePen,
        _intent: &BranchIntent,
        _depth: usize,
        params: &TreeParams,
        rng: &mut StdRng,
    ) -> BranchResult {
        let (rx, _) = params.root();
        let top_y = params.canopy_top();
        let ry = params.root().1;
        let height = (ry - top_y).max(3);
        let arm_count = rng.random_range(3..6usize);
        let total_spread = params.spread();
        let arm_color = lighten(params.trunk_color, 20);
        let tip_c = lighten(arm_color, 30);
        let mut tips = Vec::new();

        // Coin flip: uniform bar vs staggered forks
        let staggered = rng.random_range(0..2u32) == 0;

        // Per-arm fork heights: uniform = all same, staggered = spread across zone
        let uniform_split = ry - height / 3;
        let fork_ys: Vec<i32> = if staggered {
            let fork_lo = ry - height * 2 / 3;
            let fork_hi = ry - height / 4;
            let fork_range = (fork_hi - fork_lo).max(2);
            let mut ys: Vec<i32> = (0..arm_count)
                .map(|_| fork_lo + rng.random_range(0..fork_range as u32) as i32)
                .collect();
            ys.sort();
            ys
        } else {
            vec![uniform_split; arm_count]
        };

        if staggered {
            // Central trunk spine from root up to highest fork
            let spine_top = *fork_ys.iter().min().unwrap_or(&uniform_split);
            for y in spine_top..ry {
                set(grid, rx, y, '│', params.trunk_color);
            }
        } else {
            // Classic horizontal connector bar
            let bar_color = darken(params.trunk_color, 10);
            let start_x = rx - total_spread;
            let end_x = rx + total_spread;
            for x in start_x..=end_x {
                set(grid, x, uniform_split, '─', bar_color);
            }
            set(grid, rx, uniform_split, '┬', params.trunk_color);
        }

        let step = (total_spread * 2) / (arm_count as i32 - 1).max(1);
        let start_x = rx - total_spread;

        for i in 0..arm_count {
            let ax = start_x + i as i32 * step;
            let fork_y = fork_ys[i];

            if staggered {
                // Horizontal spur from trunk to arm at this fork_y
                let (spur_lo, spur_hi) = if ax <= rx { (ax, rx) } else { (rx, ax) };
                for x in spur_lo..=spur_hi {
                    set(grid, x, fork_y, '─', params.trunk_color);
                }
                let jc = if ax < rx {
                    '┘'
                } else if ax > rx {
                    '└'
                } else {
                    '┤'
                };
                set(grid, ax, fork_y, jc, params.trunk_color);
                set(grid, rx, fork_y, '┼', params.trunk_color);
            } else {
                // Classic: junction char on the shared bar
                let jc = if i == 0 {
                    '└'
                } else if i == arm_count - 1 {
                    '┘'
                } else {
                    '┴'
                };
                set(grid, ax, fork_y, jc, params.trunk_color);
            }

            // Lean direction: arms left of center lean left, right lean right
            let lean: i32 = if ax < rx {
                -1
            } else if ax > rx {
                1
            } else {
                0
            };
            let arm_top = top_y + rng.random_range(0..3u32) as i32;

            // Vertical arm with corner-pair lean at midpoint
            let mut cx = ax;
            let mid_y = (arm_top + fork_y) / 2;
            for y in (arm_top..fork_y).rev() {
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

    fn draw_fruit(&self, grid: &mut Grid, x: i32, y: i32, params: &TreeParams, _rng: &mut StdRng) {
        // Lantern: diamond hanging below on stem
        set(grid, x, y, '│', lighten(params.fruit_color, 30));
        set(grid, x, y + 1, '◇', params.fruit_color);
    }
}

pub struct SplitTree;
impl TreeDrawer for SplitTree {
    fn draw_trunk(
        &self,
        grid: &mut Grid,
        pen: &mut TreePen,
        params: &TreeParams,
        rng: &mut StdRng,
    ) -> Vec<TrunkNode> {
        // Minimal stump -- splitting IS the tree
        let mut nodes = Vec::new();
        pen.step(grid, MoveDir::Up);
        nodes.push(TrunkNode {
            x: pen.x,
            y: pen.y,
            dir: MoveDir::Up,
        });
        nodes
    }

    fn should_branch(
        &self,
        idx: usize,
        count: usize,
        _params: &TreeParams,
        _rng: &mut StdRng,
    ) -> Option<BranchIntent> {
        // Fire once at the trunk top -- draw_branch does recursive forking
        if idx == count - 1 {
            Some(BranchIntent {
                go_left: false,
                length: 0,
                level: 0,
            })
        } else {
            None
        }
    }

    fn draw_branch(
        &self,
        grid: &mut Grid,
        _pen: &mut TreePen,
        _intent: &BranchIntent,
        _depth: usize,
        params: &TreeParams,
        rng: &mut StdRng,
    ) -> BranchResult {
        // Start from where the trunk actually ended, not params.root()
        let trunk_top_x = _pen.x;
        let trunk_top_y = _pen.y;
        let top_y = params.canopy_top();
        let height = (trunk_top_y - top_y).max(3);
        let first_split = trunk_top_y; // split immediately from trunk top
        let spread = params.spread();
        let mut tips = Vec::new();

        // Stem from trunk top up to first_split (connects trunk to branching zone).
        // Fill every cell from first_split to trunk_top_y inclusive -- no gaps.
        for y in first_split..=trunk_top_y {
            set(grid, trunk_top_x, y, '│', params.trunk_color);
        }

        // BFS queue: (x, top_y, bottom_y, depth)
        let mut queue: Vec<(i32, i32, i32, usize)> = vec![(trunk_top_x, top_y, first_split, 0)];
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
            let split_y = (top + (bottom - top) * split_frac / 100)
                .max(top + 1)
                .min(bottom - 1);

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

    fn draw_fruit(&self, grid: &mut Grid, x: i32, y: i32, params: &TreeParams, rng: &mut StdRng) {
        // Berry cluster: 2-3 dots around tip
        set(grid, x, y, '•', params.fruit_color);
        if rng.random::<bool>() {
            set(grid, x - 1, y, '•', lighten(params.fruit_color, 20));
        }
        if rng.random::<bool>() {
            set(grid, x + 1, y - 1, '•', lighten(params.fruit_color, 10));
        }
    }
}

pub struct BirchTree;
impl TreeDrawer for BirchTree {
    fn draw_trunk(
        &self,
        grid: &mut Grid,
        pen: &mut TreePen,
        params: &TreeParams,
        rng: &mut StdRng,
    ) -> Vec<TrunkNode> {
        StraightTrunk {
            height_fraction: 1.0,
        }
        .draw(grid, pen, params, rng)
    }

    fn should_branch(
        &self,
        idx: usize,
        count: usize,
        params: &TreeParams,
        rng: &mut StdRng,
    ) -> Option<BranchIntent> {
        let interval = 2;

        // Skip first and last node
        if idx == 0 || idx >= count - 1 {
            return None;
        }

        // Branch at interval=2, alternating left/right
        if idx % interval != 0 {
            return None;
        }

        // 25% chance to skip this branch
        if rng.random_range(0..4u32) == 0 {
            return None;
        }

        let level = idx / interval - 1;
        let go_left = level % 2 == 0;
        let max_arm = params.spread().max(2).min(6);
        let length = rng.random_range(2..=max_arm);

        Some(BranchIntent {
            go_left,
            length,
            level,
        })
    }

    fn draw_branch(
        &self,
        grid: &mut Grid,
        pen: &mut TreePen,
        intent: &BranchIntent,
        _depth: usize,
        params: &TreeParams,
        rng: &mut StdRng,
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

    fn draw_fruit(&self, grid: &mut Grid, x: i32, y: i32, params: &TreeParams, rng: &mut StdRng) {
        // Catkin: braille dangle below tip
        let len = 1 + rng.random_range(0..2u32) as i32;
        for i in 0..len {
            set(
                grid,
                x,
                y + 1 + i,
                '⡇',
                lighten(params.fruit_color, (i * 20) as u8),
            );
        }
    }
}

pub struct WavyBirch;
impl TreeDrawer for WavyBirch {
    fn draw_trunk(
        &self,
        grid: &mut Grid,
        pen: &mut TreePen,
        params: &TreeParams,
        rng: &mut StdRng,
    ) -> Vec<TrunkNode> {
        StraightTrunk {
            height_fraction: 1.0,
        }
        .draw(grid, pen, params, rng)
    }

    fn should_branch(
        &self,
        idx: usize,
        count: usize,
        params: &TreeParams,
        rng: &mut StdRng,
    ) -> Option<BranchIntent> {
        let interval = 2;
        if idx == 0 || idx >= count - 1 {
            return None;
        }
        if idx % interval != 0 {
            return None;
        }
        // 15% skip (less than birch's 25% -- wavy looks better dense)
        if rng.random_range(0..7u32) == 0 {
            return None;
        }

        let level = idx / interval - 1;
        let go_left = level % 2 == 0;
        let max_arm = params.spread().max(3).min(8);
        let length = rng.random_range(3..=max_arm);

        Some(BranchIntent {
            go_left,
            length,
            level,
        })
    }

    fn draw_branch(
        &self,
        grid: &mut Grid,
        pen: &mut TreePen,
        intent: &BranchIntent,
        _depth: usize,
        params: &TreeParams,
        rng: &mut StdRng,
    ) -> BranchResult {
        use MoveDir::*;
        let h_dir = if intent.go_left { Left } else { Right };
        let mut tips = Vec::new();

        let c = lighten(params.branch_color, rng.random_range(10..50u8) as u8);
        pen.color = c;

        // Per-side waveform: use branch level to sample a sine wave
        // Each side has its own phase so L/R are asymmetric
        let side_phase: f32 = if intent.go_left { 0.0 } else { 1.8 };
        let wave_amp: f32 = 1.0 + rng.random_range(0..3u32) as f32 * 0.5;
        let wave_period: f32 = 2.5 + rng.random_range(0..3u32) as f32;

        // Junction at trunk
        let jc = if intent.go_left { '┤' } else { '├' };
        set(grid, pen.x, pen.y, jc, c);

        let start_y = pen.y;
        let mut prev_y = pen.y;

        // Wavy arm: horizontal with vertical displacement per cell
        for i in 1..=intent.length {
            pen.x += h_dir.dx();
            let t = i as f32 / wave_period;
            let wave_y =
                start_y + ((t + side_phase + intent.level as f32 * 0.7).sin() * wave_amp) as i32;
            let dy = (wave_y - prev_y).clamp(-1, 1);
            let cur_y = prev_y + dy;

            // Connect vertical displacement
            if dy < 0 {
                // Going up: corner then horizontal
                let corner = if intent.go_left { '╯' } else { '╰' };
                set(grid, pen.x, prev_y, corner, c);
                set(grid, pen.x, cur_y, '─', c);
            } else if dy > 0 {
                // Going down: corner then horizontal
                let corner = if intent.go_left { '╮' } else { '╭' };
                set(grid, pen.x, prev_y, corner, c);
                pen.x += h_dir.dx();
                set(grid, pen.x, cur_y, '─', c);
            } else {
                set(grid, pen.x, cur_y, '─', c);
            }

            pen.y = cur_y;
            prev_y = cur_y;
        }

        // Cap at arm end
        let cap = if intent.go_left { '╮' } else { '╭' };
        set(grid, pen.x, pen.y, cap, c);

        // Spray tip above cap
        let spray_y = pen.y - 1;
        let spray_c = lighten(c, 20);
        set(grid, pen.x, spray_y, '╷', spray_c);
        tips.push((pen.x, spray_y));

        // Second tip if arm long enough
        if intent.length > 3 {
            let second_x = pen.x - h_dir.dx();
            set(grid, second_x, spray_y, '╷', lighten(c, 10));
            tips.push((second_x, spray_y));
        }

        BranchResult { tips }
    }

    fn draw_tip(&self, grid: &mut Grid, x: i32, y: i32, params: &TreeParams) {
        set(grid, x, y, '╷', lighten(params.tip_color, 60));
    }

    fn draw_fruit(&self, grid: &mut Grid, x: i32, y: i32, params: &TreeParams, rng: &mut StdRng) {
        // Seed pod: diamond with scatter dots
        set(grid, x, y, '◆', params.fruit_color);
        if rng.random::<bool>() {
            set(grid, x + 1, y, '·', lighten(params.fruit_color, 30));
        }
    }
}

pub struct StormTree {
    /// Lean direction picked during draw_trunk, consumed by draw_branch.
    /// +1 = lean right (branches go left), -1 = lean left (branches go right).
    lean_trunk: LeanTrunk,
}
impl StormTree {
    pub fn new() -> Self {
        StormTree {
            lean_trunk: LeanTrunk::new(),
        }
    }
}
impl TreeDrawer for StormTree {
    fn draw_trunk(
        &self,
        grid: &mut Grid,
        pen: &mut TreePen,
        params: &TreeParams,
        rng: &mut StdRng,
    ) -> Vec<TrunkNode> {
        self.lean_trunk.draw(grid, pen, params, rng)
    }

    fn should_branch(
        &self,
        idx: usize,
        count: usize,
        params: &TreeParams,
        _rng: &mut StdRng,
    ) -> Option<BranchIntent> {
        if count < 4 {
            return None;
        }

        let height = params.root().1 - params.canopy_top();
        let interval = (height / 4).max(2);

        // idx 0 = nearest root, idx count-1 = apex
        let distance_from_root = count as i32 - 1 - idx as i32;

        if distance_from_root < 2 {
            return None;
        }
        if (distance_from_root - 2) % interval != 0 {
            return None;
        }

        let level = ((distance_from_root - 2) / interval) as usize;
        let max_spread = params.spread();
        let arm = (max_spread - level as i32 * 2).max(2);

        // go_left encodes windward side (opposite lean)
        let go_left = self.lean_trunk.lean.get() > 0;

        Some(BranchIntent {
            go_left,
            length: arm,
            level,
        })
    }

    fn draw_branch(
        &self,
        grid: &mut Grid,
        pen: &mut TreePen,
        intent: &BranchIntent,
        _depth: usize,
        params: &TreeParams,
        _rng: &mut StdRng,
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

    fn draw_fruit(&self, grid: &mut Grid, x: i32, y: i32, params: &TreeParams, rng: &mut StdRng) {
        // Lightning fruit: spark with scatter
        set(grid, x, y, '✦', params.fruit_color);
        if rng.random::<bool>() {
            set(grid, x + 1, y - 1, '·', lighten(params.fruit_color, 30));
        }
        if rng.random::<bool>() {
            set(grid, x - 1, y, '·', lighten(params.fruit_color, 20));
        }
    }
}

pub struct DeadTree;
impl TreeDrawer for DeadTree {
    fn draw_trunk(
        &self,
        grid: &mut Grid,
        pen: &mut TreePen,
        params: &TreeParams,
        rng: &mut StdRng,
    ) -> Vec<TrunkNode> {
        GnarledTrunk.draw(grid, pen, params, rng)
    }

    fn should_branch(
        &self,
        idx: usize,
        count: usize,
        params: &TreeParams,
        rng: &mut StdRng,
    ) -> Option<BranchIntent> {
        // Sparse branches: ~5-6 branches evenly spaced
        let interval = (count / 6).max(2);
        if idx < interval || idx >= count - 1 {
            return None;
        }
        if idx % interval != 0 {
            return None;
        }

        let level = idx / interval;
        let go_left = level % 2 == 0;
        let max_arm = params.spread().max(2).min(8);
        let length = rng.random_range(2..=max_arm);

        Some(BranchIntent {
            go_left,
            length,
            level,
        })
    }

    fn draw_branch(
        &self,
        grid: &mut Grid,
        _pen: &mut TreePen,
        intent: &BranchIntent,
        _depth: usize,
        params: &TreeParams,
        _rng: &mut StdRng,
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

    fn draw_fruit(&self, grid: &mut Grid, x: i32, y: i32, params: &TreeParams, _rng: &mut StdRng) {
        // Dried husk: hollow circle
        set(grid, x, y, '○', darken(params.fruit_color, 20));
    }
}

pub struct DroopingTree;
impl TreeDrawer for DroopingTree {
    fn draw_trunk(
        &self,
        grid: &mut Grid,
        pen: &mut TreePen,
        params: &TreeParams,
        rng: &mut StdRng,
    ) -> Vec<TrunkNode> {
        StraightTrunk {
            height_fraction: 2.0 / 3.0,
        }
        .draw(grid, pen, params, rng)
    }

    fn should_branch(
        &self,
        idx: usize,
        count: usize,
        _params: &TreeParams,
        _rng: &mut StdRng,
    ) -> Option<BranchIntent> {
        // Fire once at the trunk top -- draw_branch builds the whole crown
        if idx == count - 1 {
            Some(BranchIntent {
                go_left: false,
                length: 0,
                level: 0,
            })
        } else {
            None
        }
    }

    fn draw_branch(
        &self,
        grid: &mut Grid,
        _pen: &mut TreePen,
        _intent: &BranchIntent,
        _depth: usize,
        params: &TreeParams,
        rng: &mut StdRng,
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
                let (x0, x1) = if arm_x_offset < 0 { (bx, rx) } else { (rx, bx) };
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

    fn draw_fruit(&self, grid: &mut Grid, x: i32, y: i32, params: &TreeParams, _rng: &mut StdRng) {
        // Teardrop: hanging below with drip
        set(grid, x, y, '▽', params.fruit_color);
        set(grid, x, y + 1, '·', lighten(params.fruit_color, 30));
    }
}

pub struct PineTree;
impl TreeDrawer for PineTree {
    fn draw_trunk(
        &self,
        grid: &mut Grid,
        pen: &mut TreePen,
        params: &TreeParams,
        rng: &mut StdRng,
    ) -> Vec<TrunkNode> {
        StraightTrunk {
            height_fraction: 0.3,
        }
        .draw(grid, pen, params, rng)
    }

    fn should_branch(
        &self,
        idx: usize,
        count: usize,
        _params: &TreeParams,
        _rng: &mut StdRng,
    ) -> Option<BranchIntent> {
        // Branch at every 2-3 rows through the trunk for needle tiers
        if idx < 2 || idx >= count {
            return None;
        }
        let interval = 2;
        if idx % interval != 0 {
            return None;
        }
        let level = idx / interval;
        // Tiers get narrower toward top
        let length = ((count - idx) as i32 / 2).max(2);
        Some(BranchIntent {
            go_left: level % 2 == 0,
            length,
            level,
        })
    }

    fn draw_branch(
        &self,
        grid: &mut Grid,
        _pen: &mut TreePen,
        intent: &BranchIntent,
        _depth: usize,
        params: &TreeParams,
        _rng: &mut StdRng,
    ) -> BranchResult {
        let bx = _pen.x;
        let by = _pen.y;
        let half = intent.length;
        let color = params.branch_color;
        let needles = ['▪', '◆', '●', '▫'];
        let mut tips = Vec::new();

        // Draw V-shaped needle row spanning both sides
        set(grid, bx - half, by, '╱', color);
        set(grid, bx + half, by, '╲', color);
        for dx in 1..half {
            let needle = needles[((bx + dx) as usize + by as usize) % needles.len()];
            let nc = if (dx as usize + by as usize) % 3 == 0 {
                lighten(color, 20)
            } else {
                color
            };
            set(grid, bx - dx, by, needle, nc);
            set(grid, bx + dx, by, needle, nc);
        }
        set(grid, bx, by, '│', params.trunk_color);

        tips.push((bx - half, by));
        tips.push((bx + half, by));
        BranchResult { tips }
    }

    fn draw_tip(&self, grid: &mut Grid, x: i32, y: i32, params: &TreeParams) {
        set(grid, x, y, '▲', lighten(params.tip_color, 30));
    }

    fn draw_fruit(&self, grid: &mut Grid, x: i32, y: i32, params: &TreeParams, _rng: &mut StdRng) {
        // Pinecone hanging below
        set(grid, x, y + 1, '◆', darken(params.fruit_color, 10));
    }
}

pub struct WillowTree;
impl TreeDrawer for WillowTree {
    fn draw_trunk(
        &self,
        grid: &mut Grid,
        pen: &mut TreePen,
        params: &TreeParams,
        rng: &mut StdRng,
    ) -> Vec<TrunkNode> {
        StraightTrunk {
            height_fraction: 0.4,
        }
        .draw(grid, pen, params, rng)
    }

    fn should_branch(
        &self,
        idx: usize,
        count: usize,
        _params: &TreeParams,
        _rng: &mut StdRng,
    ) -> Option<BranchIntent> {
        // Branch in the top half, every 2 rows, both sides
        let start = count / 2;
        if idx < start {
            return None;
        }
        if (idx - start) % 2 != 0 {
            return None;
        }
        let go_left = (idx - start) % 4 < 2;
        let length = ((count - idx) as i32).max(8).min(16);
        Some(BranchIntent {
            go_left,
            length,
            level: idx - start,
        })
    }

    fn draw_branch(
        &self,
        grid: &mut Grid,
        _pen: &mut TreePen,
        intent: &BranchIntent,
        _depth: usize,
        params: &TreeParams,
        rng: &mut StdRng,
    ) -> BranchResult {
        let bx = _pen.x;
        let by = _pen.y;
        let arm_len = intent.length;
        let dir: i32 = if intent.go_left { -1 } else { 1 };
        let color = lighten(params.branch_color, (intent.level as u8 * 10).min(40));
        let mut tips = Vec::new();

        // Horizontal arm
        for dx in 1..=arm_len {
            set(grid, bx + dx * dir, by, '─', color);
        }
        set(grid, bx, by, '┼', color);
        let end_x = bx + arm_len * dir;
        set(
            grid,
            end_x,
            by,
            if intent.go_left { '╭' } else { '╮' },
            color,
        );

        // Droops at multiple points along the arm, no two on the same x
        let droop_chars = ['│', '┊', '╎', '┆'];
        let mut used_x: Vec<i32> = Vec::new();
        // Every 2-3 cells along the arm gets a tendril
        let mut dx = 2;
        while dx <= arm_len {
            let tx = bx + dx * dir;
            // Offset by 0 or 1 so neighbors don't line up vertically
            let offset = rng.random_range(0..2u32) as i32;
            let droop_x = tx + offset * dir;
            if !used_x.contains(&droop_x) {
                used_x.push(droop_x);
                let droop_len = rng.random_range(2..6u32) as i32;
                for dy in 1..=droop_len {
                    let ch = droop_chars[dy as usize % droop_chars.len()];
                    let dc = lighten(color, (dy * 8) as u8);
                    set(grid, droop_x, by + dy, ch, dc);
                }
                tips.push((droop_x, by + droop_len));
            }
            dx += rng.random_range(2..4u32) as i32;
        }

        // Always droop from the endpoint too
        if !used_x.contains(&end_x) {
            let droop_len = rng.random_range(2..6u32) as i32;
            for dy in 1..=droop_len {
                let ch = droop_chars[dy as usize % droop_chars.len()];
                let dc = lighten(color, (dy * 8) as u8);
                set(grid, end_x, by + dy, ch, dc);
            }
            tips.push((end_x, by + droop_len));
        }
        BranchResult { tips }
    }

    fn draw_tip(&self, grid: &mut Grid, x: i32, y: i32, params: &TreeParams) {
        set(grid, x, y, '·', lighten(params.tip_color, 50));
    }

    fn draw_fruit(
        &self,
        _grid: &mut Grid,
        _x: i32,
        _y: i32,
        _params: &TreeParams,
        _rng: &mut StdRng,
    ) {
        // Willows don't fruit visually
    }
}

pub struct PalmTree;
impl TreeDrawer for PalmTree {
    fn draw_trunk(
        &self,
        grid: &mut Grid,
        pen: &mut TreePen,
        params: &TreeParams,
        rng: &mut StdRng,
    ) -> Vec<TrunkNode> {
        SineTrunk {
            height_fraction: 0.7,
            amplitude: 2,
        }
        .draw(grid, pen, params, rng)
    }

    fn should_branch(
        &self,
        idx: usize,
        count: usize,
        _params: &TreeParams,
        _rng: &mut StdRng,
    ) -> Option<BranchIntent> {
        // Only branch at the apex (top 2 nodes)
        if count < 3 {
            return None;
        }
        if idx < count - 2 {
            return None;
        }
        let go_left = idx == count - 2;
        Some(BranchIntent {
            go_left,
            length: 6,
            level: 0,
        })
    }

    fn draw_branch(
        &self,
        grid: &mut Grid,
        _pen: &mut TreePen,
        intent: &BranchIntent,
        _depth: usize,
        params: &TreeParams,
        rng: &mut StdRng,
    ) -> BranchResult {
        let bx = _pen.x;
        let by = _pen.y;
        let color = params.branch_color;
        let mut tips = Vec::new();

        // Draw 4-6 fronds radiating outward and downward from crown
        let frond_count = rng.random_range(4..7u32) as i32;
        let frond_len = intent.length;

        for f in 0..frond_count {
            let go_left = if intent.go_left {
                f % 2 == 0
            } else {
                f % 2 == 1
            };
            let dir: i32 = if go_left { -1 } else { 1 };
            let droop_rate = rng.random_range(2..5u32) as i32; // droop every N cells

            let mut fx = bx;
            let mut fy = by;
            for step in 1..=frond_len {
                fx += dir;
                if step > 1 && step % droop_rate == 0 {
                    fy += 1;
                }
                let ch = if step == frond_len {
                    '~'
                } else if fy > by {
                    if go_left { '╲' } else { '╱' }
                } else {
                    '─'
                };
                let fc = lighten(color, ((step * 5) as u8).min(40));
                set(grid, fx, fy, ch, fc);
            }
            tips.push((fx, fy));
        }
        BranchResult { tips }
    }

    fn draw_tip(&self, grid: &mut Grid, x: i32, y: i32, params: &TreeParams) {
        set(grid, x, y, '✦', lighten(params.tip_color, 30));
    }

    fn draw_fruit(&self, grid: &mut Grid, x: i32, y: i32, params: &TreeParams, _rng: &mut StdRng) {
        // Coconut hanging below frond
        set(grid, x, y + 1, '●', darken(params.fruit_color, 10));
    }
}

