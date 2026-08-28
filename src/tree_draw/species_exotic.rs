//! Exotic species drawers (oak..helix).
use super::*;
pub struct WideTree;
impl TreeDrawer for WideTree {
    fn draw_trunk(
        &self,
        grid: &mut Grid,
        pen: &mut TreePen,
        params: &TreeParams,
        _rng: &mut StdRng,
    ) -> Vec<TrunkNode> {
        // Short trunk: bottom quarter
        let height = params.plot.h as i32;
        let trunk_len = (height / 4).max(2);
        let mut nodes = Vec::new();
        for _ in 0..trunk_len {
            pen.step(grid, MoveDir::Up);
            nodes.push(TrunkNode {
                x: pen.x,
                y: pen.y,
                dir: MoveDir::Up,
            });
        }
        nodes
    }

    fn should_branch(
        &self,
        idx: usize,
        count: usize,
        _params: &TreeParams,
        _rng: &mut StdRng,
    ) -> Option<BranchIntent> {
        // Single branch event at trunk top -- draw_branch handles all 3 tiers
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
        let rx = _pen.x;
        let trunk_top_y = _pen.y;
        let top_y = params.canopy_top();
        let height = (trunk_top_y - top_y).max(6);
        let spread = params.spread();
        let mut tips = Vec::new();

        // 3 tiers with asymmetric arm lengths per side
        let tier_ys = [trunk_top_y, top_y + height * 2 / 3, top_y + height / 3];
        let base_arms = [spread * 2, spread, spread / 2];

        // Store (y, left_arm, right_arm) so sub-trunks connect properly
        let mut tiers: Vec<(i32, i32, i32)> = Vec::new();
        for ti in 0..3 {
            let base = base_arms[ti];
            // Each side gets 50-150% of the base arm, independently
            let left_arm = (base * rng.random_range(50..151u32) as i32 / 100).max(1);
            let right_arm = (base * rng.random_range(50..151u32) as i32 / 100).max(1);
            tiers.push((tier_ys[ti], left_arm, right_arm));
        }

        for (ti, &(sy, left_arm, right_arm)) in tiers.iter().enumerate() {
            let c = lighten(params.branch_color, (ti as u8 * 20).min(40));
            let lx = rx - left_arm;
            let rx2 = rx + right_arm;

            // Horizontal bar
            set(grid, rx, sy, '┼', c);
            for x in lx..rx {
                set(grid, x, sy, '─', c);
            }
            for x in rx + 1..=rx2 {
                set(grid, x, sy, '─', c);
            }
            set(grid, lx, sy, '╭', c);
            set(grid, rx2, sy, '╮', c);

            // Vertical sub-trunks to next tier
            let next_sy = if ti + 1 < tiers.len() {
                tiers[ti + 1].0
            } else {
                top_y
            };
            for y in next_sy..sy {
                set(grid, lx, y, '│', c);
            }
            for y in next_sy..sy {
                set(grid, rx2, y, '│', c);
            }

            if ti + 1 >= tiers.len() {
                tips.push((lx, next_sy));
                tips.push((rx2, next_sy));
            }
        }
        // Center trunk between tiers
        for y in tiers[1].0..trunk_top_y {
            set(grid, rx, y, '│', params.trunk_color);
        }
        for y in tiers[2].0..tiers[1].0 {
            set(grid, rx, y, '│', params.trunk_color);
        }

        BranchResult { tips }
    }

    fn draw_tip(&self, grid: &mut Grid, x: i32, y: i32, params: &TreeParams) {
        set(grid, x, y, '╷', lighten(params.tip_color, 30));
    }

    fn draw_fruit(&self, grid: &mut Grid, x: i32, y: i32, params: &TreeParams, _rng: &mut StdRng) {
        set(grid, x, y + 1, '◆', params.fruit_color);
    }
}

pub struct AsymmetricTree;
impl TreeDrawer for AsymmetricTree {
    fn draw_trunk(
        &self,
        grid: &mut Grid,
        pen: &mut TreePen,
        params: &TreeParams,
        _rng: &mut StdRng,
    ) -> Vec<TrunkNode> {
        let height = params.plot.h as i32;
        let trunk_len = (height / 3).max(2);
        let mut nodes = Vec::new();
        for _ in 0..trunk_len {
            pen.step(grid, MoveDir::Up);
            nodes.push(TrunkNode {
                x: pen.x,
                y: pen.y,
                dir: MoveDir::Up,
            });
        }
        nodes
    }

    fn should_branch(
        &self,
        idx: usize,
        count: usize,
        _params: &TreeParams,
        _rng: &mut StdRng,
    ) -> Option<BranchIntent> {
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
        let rx = _pen.x;
        let split_y = _pen.y;
        let top_y = params.canopy_top();
        let spread = params.spread();
        let mut tips = Vec::new();

        let heavy_left = rng.random_range(0..2u32) == 0;
        let base = spread as i32;
        // Heavy side gets 2x spread, light side gets 1/2 -- 4:1 ratio
        let (left_spread, right_spread) = if heavy_left {
            (base * 2, base / 2)
        } else {
            (base / 2, base * 2)
        };
        let left_max_d = if heavy_left { 4usize } else { 1 };
        let right_max_d = if heavy_left { 1 } else { 4 };

        // Initial junction
        set(grid, rx, split_y, '┼', params.trunk_color);
        for x in rx - left_spread..rx {
            set(grid, x, split_y, '─', params.trunk_color);
        }
        for x in rx + 1..=rx + right_spread {
            set(grid, x, split_y, '─', params.trunk_color);
        }
        set(grid, rx - left_spread, split_y, '╭', params.trunk_color);
        set(grid, rx + right_spread, split_y, '╮', params.trunk_color);

        // BFS: (x, top, bottom, depth, max_depth)
        let mut queue: Vec<(i32, i32, i32, usize, usize)> = vec![
            (rx - left_spread, top_y, split_y, 0, left_max_d),
            (rx + right_spread, top_y, split_y, 0, right_max_d),
        ];

        while let Some((x, top, bottom, depth, max_d)) = queue.pop() {
            let c = lighten(params.branch_color, (depth as u8 * 18).min(60));
            for y in top + 1..bottom {
                set(grid, x, y, '│', c);
            }

            if depth >= max_d || bottom - top <= 2 {
                tips.push((x, top));
                continue;
            }

            // Randomize split height (20-60% of segment)
            let split_frac = 20 + rng.random_range(0..41u32) as i32;
            let split_at = (top + (bottom - top) * split_frac / 100)
                .max(top + 1)
                .min(bottom - 1);
            // Unequal arms: one side 50-150% of base arm
            let base_arm = (base >> (depth + 1) as u32).max(1);
            let left_arm = (base_arm * rng.random_range(30..120u32) as i32 / 100).max(1);
            let right_arm = (base_arm * rng.random_range(50..170u32) as i32 / 100).max(1);
            set(grid, x, split_at, '┼', c);
            for ax in x - left_arm..x {
                set(grid, ax, split_at, '─', c);
            }
            for ax in x + 1..=x + right_arm {
                set(grid, ax, split_at, '─', c);
            }
            set(grid, x - left_arm, split_at, '╭', c);
            set(grid, x + right_arm, split_at, '╮', c);

            queue.push((x - left_arm, top, split_at, depth + 1, max_d));
            queue.push((x + right_arm, top, split_at, depth + 1, max_d));
        }

        BranchResult { tips }
    }

    fn draw_tip(&self, grid: &mut Grid, x: i32, y: i32, params: &TreeParams) {
        set(grid, x, y, '╷', lighten(params.tip_color, 30));
    }

    fn draw_fruit(&self, grid: &mut Grid, x: i32, y: i32, params: &TreeParams, rng: &mut StdRng) {
        let ch = if rng.random_range(0..2u32) == 0 {
            '◇'
        } else {
            '●'
        };
        set(grid, x, y - 1, ch, params.fruit_color);
    }
}

pub struct KaijuTree;
impl TreeDrawer for KaijuTree {
    fn draw_trunk(
        &self,
        grid: &mut Grid,
        pen: &mut TreePen,
        params: &TreeParams,
        rng: &mut StdRng,
    ) -> Vec<TrunkNode> {
        // Thick 3-wide base for bottom third
        let height = params.plot.h as i32;
        let base_len = (height / 3).max(3);
        let bark = darken(params.trunk_color, 15);

        let mut nodes = Vec::new();
        for _ in 0..base_len {
            pen.step(grid, MoveDir::Up);
            // Thick: flanking columns
            set(grid, pen.x - 1, pen.y, '│', bark);
            set(grid, pen.x + 1, pen.y, '│', bark);
            nodes.push(TrunkNode {
                x: pen.x,
                y: pen.y,
                dir: MoveDir::Up,
            });
        }
        // Overwrite center with thick char
        for n in &nodes {
            set(grid, n.x, n.y, '┃', params.trunk_color);
        }
        nodes
    }

    fn should_branch(
        &self,
        idx: usize,
        count: usize,
        _params: &TreeParams,
        _rng: &mut StdRng,
    ) -> Option<BranchIntent> {
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
        let rx = _pen.x;
        let base_top = _pen.y;
        let top_y = params.canopy_top();
        let spread = params.spread();
        let mut tips = Vec::new();

        // Fork connector
        let trunk_count = rng.random_range(2..4u32) as i32;
        let total_spread = spread * 2;
        let c0 = lighten(params.trunk_color, 10);

        struct SubTrunk {
            target_x: i32,
            lean: i32,
            branch_side: i32,
        }
        let mut sub_trunks: Vec<SubTrunk> = Vec::new();
        for i in 0..trunk_count {
            let frac = i as f32 / (trunk_count - 1).max(1) as f32;
            let target_x = rx - total_spread + (frac * (total_spread * 2) as f32) as i32;
            let lean = if target_x < rx {
                -1
            } else if target_x > rx {
                1
            } else {
                0
            };
            let branch_side = if rng.random_range(0..2u32) == 0 {
                -1i32
            } else {
                1
            };
            sub_trunks.push(SubTrunk {
                target_x,
                lean,
                branch_side,
            });
        }

        let leftmost = sub_trunks.iter().map(|t| t.target_x).min().unwrap_or(rx);
        let rightmost = sub_trunks.iter().map(|t| t.target_x).max().unwrap_or(rx);
        for x in leftmost..=rightmost {
            set(grid, x, base_top, '─', c0);
        }
        set(grid, rx, base_top, '┬', c0);

        let lean_every = ((base_top - top_y) / 5).max(3);

        for st in &sub_trunks {
            let trunk_top = top_y + rng.random_range(0..4u32) as i32;
            let mut cx = st.target_x;

            for y in (trunk_top..base_top).rev() {
                let rows_up = base_top - y;
                let (ch, do_lean) = if st.lean != 0 && rows_up > 0 && rows_up % lean_every == 0 {
                    cx += st.lean;
                    (if st.lean > 0 { '╱' } else { '╲' }, true)
                } else {
                    ('│', false)
                };
                let c = lighten(params.trunk_color, ((base_top - y) as u8).min(40));
                set(grid, cx, y, ch, c);
            }

            // Branches at intervals
            let branch_count = rng.random_range(3..7u32) as usize;
            let trunk_h = (base_top - trunk_top) as usize;
            let interval = (trunk_h / (branch_count + 1)).max(2);

            for b in 0..branch_count {
                let jitter = rng.random_range(0..3u32) as i32 - 1;
                let by = trunk_top + (interval * (b + 1)) as i32 + jitter;
                if by >= base_top || by <= trunk_top {
                    continue;
                }

                let rows_up = base_top - by;
                let tx = st.target_x + st.lean * (rows_up / lean_every);
                let base_arm = (spread / 3).max(2) as i32 - (b as i32 / 2);
                let base_arm = base_arm.max(1);

                let long_f = rng.random_range(15..30u32) as i32;
                let short_f = rng.random_range(5..12u32) as i32;
                let (left_arm, right_arm) = if st.branch_side < 0 {
                    (base_arm * long_f / 10, base_arm * short_f / 10)
                } else {
                    (base_arm * short_f / 10, base_arm * long_f / 10)
                };

                let c = lighten(params.branch_color, (b as u8 * 12 + 15).min(60));
                if left_arm > 0 {
                    for i in 1..=left_arm {
                        set(grid, tx - i, by, '─', c);
                    }
                    set(grid, tx - left_arm, by, '╮', c);
                    tips.push((tx - left_arm - 1, by - 1));
                }
                if right_arm > 0 {
                    for i in 1..=right_arm {
                        set(grid, tx + i, by, '─', c);
                    }
                    set(grid, tx + right_arm, by, '╭', c);
                    tips.push((tx + right_arm + 1, by - 1));
                }

                let jc = if left_arm > 0 && right_arm > 0 {
                    '┼'
                } else if left_arm > 0 {
                    '┤'
                } else {
                    '├'
                };
                set(grid, tx, by, jc, c);
            }
            tips.push((cx, trunk_top));
        }
        BranchResult { tips }
    }

    fn draw_tip(&self, grid: &mut Grid, x: i32, y: i32, params: &TreeParams) {
        set(grid, x, y, '╷', lighten(params.tip_color, 35));
    }

    fn draw_fruit(&self, grid: &mut Grid, x: i32, y: i32, params: &TreeParams, _rng: &mut StdRng) {
        set(grid, x, y, '◆', params.fruit_color);
    }
}

pub struct ZigzagTree;
impl TreeDrawer for ZigzagTree {
    fn draw_trunk(
        &self,
        grid: &mut Grid,
        pen: &mut TreePen,
        params: &TreeParams,
        rng: &mut StdRng,
    ) -> Vec<TrunkNode> {
        let height = params.plot.h as i32;
        let zig_width = rng.random_range(2..4u32) as i32;
        let mut going_right = rng.random_range(0..2u32) == 0;
        let mut nodes = Vec::new();
        let bark = darken(params.trunk_color, 15);

        for i in 0..height {
            let dir = if going_right {
                MoveDir::UpRight
            } else {
                MoveDir::UpLeft
            };
            pen.step(grid, dir);
            // Thick: parallel char
            let ch = if going_right { '╱' } else { '╲' };
            set(grid, pen.x + 1, pen.y, ch, bark);
            nodes.push(TrunkNode {
                x: pen.x,
                y: pen.y,
                dir,
            });

            if i > 0 && i % (zig_width * 2 + 1) == 0 {
                going_right = !going_right;
            }
        }
        nodes
    }

    fn should_branch(
        &self,
        idx: usize,
        count: usize,
        _params: &TreeParams,
        rng: &mut StdRng,
    ) -> Option<BranchIntent> {
        // Branch at random trunk positions, ~30% of nodes
        if count > 4 && idx > 1 && idx < count - 1 && rng.random_range(0..10u32) < 3 {
            let go_left = rng.random_range(0..2u32) == 0;
            let length = rng.random_range(3..10u32) as i32;
            Some(BranchIntent {
                go_left,
                length,
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
        intent: &BranchIntent,
        _depth: usize,
        params: &TreeParams,
        rng: &mut StdRng,
    ) -> BranchResult {
        let mut tips = Vec::new();
        let bx = _pen.x;
        let by = _pen.y;
        let dx: i32 = if intent.go_left { -1 } else { 1 };
        let dy: i32 = -1; // upward

        fn draw_ray(
            grid: &mut Grid,
            x: i32,
            y: i32,
            dx: i32,
            dy: i32,
            len: i32,
            color: Color,
            depth: usize,
            max_depth: usize,
            tips: &mut Vec<(i32, i32)>,
            rng: &mut StdRng,
        ) {
            let ch = match (dx < 0, dy < 0) {
                (true, true) => '╲',
                (false, true) => '╱',
                (true, false) => '╱',
                (false, false) => '╲',
            };
            let c = lighten(color, (depth as u8 * 18).min(60));
            for step in 1..=len {
                set(grid, x + dx * step, y + dy * step, ch, c);
                if depth < max_depth
                    && step > 1
                    && step < len
                    && rng.random_range(0..(3 + depth as u32)) == 0
                {
                    let sub_dx = if rng.random_range(0..2u32) == 0 {
                        -dx
                    } else {
                        dx
                    };
                    let sub_len = rng.random_range(1..(len / 2 + 1).max(2) as u32) as i32;
                    draw_ray(
                        grid,
                        x + dx * step,
                        y + dy * step,
                        sub_dx,
                        -dy,
                        sub_len,
                        color,
                        depth + 1,
                        max_depth,
                        tips,
                        rng,
                    );
                }
            }
            tips.push((x + dx * (len + 1), y + dy * (len + 1)));
        }

        let max_depth = rng.random_range(1..3u32) as usize;
        draw_ray(
            grid,
            bx,
            by,
            dx,
            dy,
            intent.length,
            params.branch_color,
            0,
            max_depth,
            &mut tips,
            rng,
        );
        BranchResult { tips }
    }

    fn draw_tip(&self, grid: &mut Grid, x: i32, y: i32, params: &TreeParams) {
        set(grid, x, y, '·', lighten(params.tip_color, 30));
    }

    fn draw_fruit(&self, grid: &mut Grid, x: i32, y: i32, params: &TreeParams, _rng: &mut StdRng) {
        set(grid, x, y, '◇', params.fruit_color);
    }
}

pub struct BrailleCanopyTree;
impl TreeDrawer for BrailleCanopyTree {
    fn draw_trunk(
        &self,
        grid: &mut Grid,
        pen: &mut TreePen,
        params: &TreeParams,
        _rng: &mut StdRng,
    ) -> Vec<TrunkNode> {
        let height = params.plot.h as i32;
        let trunk_len = (height / 3).max(2);
        let bark = darken(params.trunk_color, 20);
        let mut nodes = Vec::new();
        for _ in 0..trunk_len {
            pen.step(grid, MoveDir::Up);
            set(grid, pen.x, pen.y, '│', bark);
            nodes.push(TrunkNode {
                x: pen.x,
                y: pen.y,
                dir: MoveDir::Up,
            });
        }
        nodes
    }

    fn should_branch(
        &self,
        idx: usize,
        count: usize,
        _params: &TreeParams,
        _rng: &mut StdRng,
    ) -> Option<BranchIntent> {
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
        let cx = _pen.x as f32;
        let trunk_top = _pen.y;
        let top_y = params.canopy_top();
        let canopy_h = (trunk_top - top_y).max(2) as f32;
        let canopy_w = params.spread() as f32;
        let center_y = top_y as f32 + canopy_h / 2.0;

        let braille_dense = ['⣿', '⣾', '⣷', '⣯', '⣻', '⣽', '⣖', '⣶'];
        let braille_sparse = ['⡇', '⢸', '⣤', '⣀', '⠛', '⠶'];

        // 15% chance of cuttlefish mode
        let cuttlefish = rng.random_range(0..7u32) == 0;
        let base_hue: f64 = if let Color::Rgb { r, g, .. } = params.branch_color {
            (r as f64 * 1.4 + g as f64 * 0.7) % 360.0
        } else {
            180.0
        };

        let mut tips = Vec::new();

        for y in top_y..=trunk_top {
            let fy = y as f32;
            let dy = (fy - center_y) / (canopy_h / 2.0);
            let vert_t = ((y - top_y) as f32 / canopy_h).clamp(0.0, 1.0);

            let noise = (rng.random_range(0..4u32) as f32 - 1.5) * 0.15;
            let row_width = ((1.0 - dy * dy).max(0.0).sqrt() + noise) * canopy_w;
            let half_w = (row_width * 1.5) as i32;

            for x in (cx as i32 - half_w)..=(cx as i32 + half_w) {
                let dx_norm = ((x as f32 - cx) / (half_w as f32).max(1.0)).abs();

                let ch = if dx_norm < 0.6 {
                    braille_dense[rng.random_range(0..braille_dense.len() as u32) as usize]
                } else if dx_norm < 0.85 {
                    braille_sparse[rng.random_range(0..braille_sparse.len() as u32) as usize]
                } else {
                    if rng.random_range(0..3u32) == 0 {
                        continue;
                    }
                    braille_sparse[rng.random_range(0..braille_sparse.len() as u32) as usize]
                };

                let hue_shift = if cuttlefish {
                    rng.random_range(0..180u32) as f64 - 90.0
                } else {
                    vert_t as f64 * 40.0 - 20.0
                };
                let h = (base_hue + hue_shift).rem_euclid(360.0);
                let s = if cuttlefish {
                    0.8
                } else {
                    0.5 + (1.0 - dx_norm) as f64 * 0.3
                };
                let l = 0.2 + (1.0 - dx_norm) as f64 * 0.3 + vert_t as f64 * 0.15;
                let c = crate::color::hsl_to_rgb(h, s, l.min(0.65));

                set(grid, x, y, ch, c);
            }

            // Tips at canopy edges
            if half_w > 0 {
                tips.push((cx as i32 - half_w, y));
                tips.push((cx as i32 + half_w, y));
            }
        }
        BranchResult { tips }
    }

    fn draw_tip(&self, _grid: &mut Grid, _x: i32, _y: i32, _params: &TreeParams) {
        // Canopy edges are already braille -- no extra tip char
    }

    fn draw_fruit(&self, grid: &mut Grid, x: i32, y: i32, params: &TreeParams, rng: &mut StdRng) {
        if rng.random_range(0..4u32) == 0 {
            set(grid, x, y, '●', params.fruit_color);
        }
    }
}

pub struct TendrilTree;
impl TreeDrawer for TendrilTree {
    fn draw_trunk(
        &self,
        grid: &mut Grid,
        pen: &mut TreePen,
        params: &TreeParams,
        _rng: &mut StdRng,
    ) -> Vec<TrunkNode> {
        let height = params.plot.h as i32;
        let trunk_len = (height / 3).max(2);
        let mut nodes = Vec::new();
        for _ in 0..trunk_len {
            pen.step(grid, MoveDir::Up);
            nodes.push(TrunkNode {
                x: pen.x,
                y: pen.y,
                dir: MoveDir::Up,
            });
        }
        nodes
    }

    fn should_branch(
        &self,
        idx: usize,
        count: usize,
        _params: &TreeParams,
        _rng: &mut StdRng,
    ) -> Option<BranchIntent> {
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
        let cx = _pen.x as f32;
        let cy = _pen.y as f32;
        let spread = params.spread() as f32;
        let mut tips = Vec::new();

        fn draw_tendril(
            grid: &mut Grid,
            x: f32,
            y: f32,
            angle: f32,
            length: f32,
            min_len: f32,
            color: Color,
            depth: usize,
            tips: &mut Vec<(i32, i32)>,
            rng: &mut StdRng,
        ) {
            if length < min_len || depth > 5 {
                return;
            }
            let c = lighten(color, (depth as u8 * 15).min(60));
            let steps = length as i32;
            let dx = angle.cos();
            let dy = angle.sin();

            for step in 1..=steps {
                let px = (x + dx * step as f32 * 1.8) as i32;
                let py = (y + dy * step as f32) as i32;
                let abs_dx = dx.abs();
                let abs_dy = dy.abs();
                let ch = if abs_dx > abs_dy * 1.5 {
                    '─'
                } else if abs_dy > abs_dx * 1.5 {
                    '│'
                } else if (dx > 0.0) == (dy > 0.0) {
                    '╲'
                } else {
                    '╱'
                };
                set(grid, px, py, ch, c);
            }

            let tip_x = x + dx * steps as f32 * 1.8;
            let tip_y = y + dy * steps as f32;
            tips.push((tip_x as i32, tip_y as i32));

            let sub_count = rng.random_range(1..4u32);
            for _ in 0..sub_count {
                let jitter = (rng.random::<f32>() - 0.5) * 1.2;
                let sub_angle = angle + jitter;
                let sub_len = length * (0.4 + rng.random::<f32>() * 0.2);
                draw_tendril(
                    grid,
                    tip_x,
                    tip_y,
                    sub_angle,
                    sub_len,
                    min_len,
                    color,
                    depth + 1,
                    tips,
                    rng,
                );
            }
        }

        let ray_count = rng.random_range(3..7u32);
        let base_len = spread.max(3.0).min(15.0);

        for i in 0..ray_count {
            let base_angle =
                -std::f32::consts::PI + (i as f32 / ray_count as f32) * std::f32::consts::PI;
            let angle = base_angle + (rng.random::<f32>() - 0.5) * 0.5;
            let len = base_len * (0.6 + rng.random::<f32>() * 0.4);
            draw_tendril(
                grid,
                cx,
                cy,
                angle,
                len,
                1.5,
                params.branch_color,
                0,
                &mut tips,
                rng,
            );
        }
        BranchResult { tips }
    }

    fn draw_tip(&self, grid: &mut Grid, x: i32, y: i32, params: &TreeParams) {
        set(grid, x, y, '·', lighten(params.tip_color, 30));
    }

    fn draw_fruit(&self, grid: &mut Grid, x: i32, y: i32, params: &TreeParams, _rng: &mut StdRng) {
        set(grid, x, y, '◆', params.fruit_color);
    }
}

pub struct OakTree;
fn oak_limb(
    grid: &mut Grid,
    x: i32,
    y: i32,
    dx: i32,
    len: i32,
    depth: usize,
    params: &TreeParams,
    tips: &mut Vec<(i32, i32)>,
    rng: &mut StdRng,
) {
    if len < 2 || depth > 2 {
        return;
    }
    let c = params.color_at_depth(depth as f32 * 0.3);
    let mut cx = x;
    let mut cy = y;
    let mut horiz = rng.random_range(0..2u32) == 0;
    for step in 0..len {
        if horiz {
            cx += dx;
            set(grid, cx, cy, '─', c);
        } else {
            cx += dx;
            cy -= 1;
            set(grid, cx, cy, if dx > 0 { '╱' } else { '╲' }, c);
        }
        if rng.random::<f32>() < 0.35 {
            // elbow joint where the limb changes pitch
            if horiz {
                set(grid, cx, cy, if dx > 0 { '╮' } else { '╭' }, darken(c, 10));
            }
            horiz = !horiz;
        }
        if step > 1 && rng.random::<f32>() < 0.25 * params.branch_factor {
            let sub_dx = if rng.random::<f32>() < 0.3 { -dx } else { dx };
            oak_limb(grid, cx, cy, sub_dx, len / 2, depth + 1, params, tips, rng);
        }
    }
    tips.push((cx + dx, cy));
}
impl TreeDrawer for OakTree {
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
        // limbs from the upper 2/3 of the trunk, denser near the top
        if count < 4 || idx < count / 3 {
            return None;
        }
        let top_frac = idx as f32 / count as f32;
        if idx != count - 1 && rng.random::<f32>() > 0.25 + top_frac * 0.3 {
            return None;
        }
        let go_left = rng.random_range(0..2u32) == 0;
        let length = (params.spread() as f32 * (0.5 + rng.random::<f32>() * 0.5)) as i32;
        let level = ((1.0 - top_frac) * 3.0) as usize;
        Some(BranchIntent {
            go_left,
            length: length.max(2),
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
        let dx = if intent.go_left { -1 } else { 1 };
        // knot where the limb leaves the trunk
        let jc = if intent.go_left { '┤' } else { '├' };
        set(grid, pen.x, pen.y, jc, darken(params.trunk_color, 10));
        let mut tips = Vec::new();
        oak_limb(
            grid,
            pen.x,
            pen.y,
            dx,
            intent.length,
            0,
            params,
            &mut tips,
            rng,
        );
        BranchResult { tips }
    }

    fn draw_tip(&self, grid: &mut Grid, x: i32, y: i32, params: &TreeParams) {
        // braille leaf cluster hugging the tip; shape varies by position parity
        let leaf = params.tip_color;
        let dim = darken(leaf, 25);
        set(grid, x, y, '⣿', leaf);
        if (x + y).rem_euclid(2) == 0 {
            set(grid, x - 1, y, '⣶', dim);
            set(grid, x + 1, y, '⣷', dim);
            set(grid, x, y - 1, '⠿', dim);
        } else {
            set(grid, x + 1, y, '⣾', dim);
            set(grid, x + 1, y - 1, '⠶', darken(leaf, 40));
            set(grid, x - 1, y - 1, '⠛', darken(leaf, 40));
        }
    }

    fn draw_fruit(&self, grid: &mut Grid, x: i32, y: i32, params: &TreeParams, rng: &mut StdRng) {
        // acorns hang one cell below the leaf cluster
        let ch = if rng.random_range(0..2u32) == 0 {
            '●'
        } else {
            '◍'
        };
        set(grid, x, y + 1, ch, params.fruit_color);
    }
}

pub struct FountainTree;
impl TreeDrawer for FountainTree {
    fn draw_trunk(
        &self,
        grid: &mut Grid,
        pen: &mut TreePen,
        params: &TreeParams,
        rng: &mut StdRng,
    ) -> Vec<TrunkNode> {
        StraightTrunk {
            height_fraction: 0.45,
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
        pen: &mut TreePen,
        _intent: &BranchIntent,
        _depth: usize,
        params: &TreeParams,
        rng: &mut StdRng,
    ) -> BranchResult {
        let apex_x = pen.x as f32;
        let apex_y = pen.y as f32;
        let floor_y = params.root().1 as f32;
        let mut tips = Vec::new();

        for side in [-1.0f32, 1.0] {
            let jets = rng.random_range(2..5u32);
            for j in 0..jets {
                let mut vx = side * (0.15 + rng.random::<f32>() * 0.5);
                let mut vy = -(0.9 + rng.random::<f32>() * 0.6);
                let mut px = apex_x;
                let mut py = apex_y;
                let c = lighten(params.branch_color, (j as u8) * 15);
                let steps = rng.random_range(6..13u32);
                let mut last = (pen.x, pen.y);
                for _ in 0..steps {
                    px += vx * 1.8;
                    py += vy;
                    vy += 0.22; // gravity pulls the jet over
                    vx *= 1.04; // slight outward fan as it falls
                    if py >= floor_y {
                        break;
                    }
                    let abs_vx = vx.abs();
                    let abs_vy = vy.abs();
                    let ch = if abs_vx > abs_vy * 1.5 {
                        '─'
                    } else if abs_vy > abs_vx * 1.5 {
                        '│'
                    } else if (vx > 0.0) == (vy > 0.0) {
                        '╲'
                    } else {
                        '╱'
                    };
                    set(grid, px as i32, py as i32, ch, c);
                    last = (px as i32, py as i32);
                }
                tips.push(last);
            }
        }
        BranchResult { tips }
    }

    fn draw_tip(&self, grid: &mut Grid, x: i32, y: i32, params: &TreeParams) {
        set(grid, x, y, '❋', lighten(params.tip_color, 20));
    }

    fn draw_fruit(&self, grid: &mut Grid, x: i32, y: i32, params: &TreeParams, _rng: &mut StdRng) {
        // droplet falls one row below the spray tip
        set(grid, x, y + 1, '∘', params.fruit_color);
    }
}

pub struct WindsweptTree {
    pub lean_right: bool,
}
impl WindsweptTree {
    pub fn new(rng: &mut StdRng) -> Self {
        WindsweptTree {
            lean_right: rng.random_range(0..2u32) == 0,
        }
    }
}
impl TreeDrawer for WindsweptTree {
    fn draw_trunk(
        &self,
        grid: &mut Grid,
        pen: &mut TreePen,
        params: &TreeParams,
        rng: &mut StdRng,
    ) -> Vec<TrunkNode> {
        let height = (params.plot.h as f32 * params.energy.clamp(0.3, 1.0)) as i32;
        let lean_dx: i32 = if self.lean_right { 1 } else { -1 };
        let diag = if self.lean_right { '╱' } else { '╲' };
        let bark = darken(params.trunk_color, 15);
        let mut nodes = Vec::new();
        for i in 0..height.max(3) {
            // lean grows stronger with height: straight low, diagonal high
            let lean_here = i > height / 4 && (i % 2 == 0 || i > height / 2);
            if lean_here {
                pen.x += lean_dx;
                pen.y -= 1;
                set(grid, pen.x, pen.y, diag, params.trunk_color);
                // doubled cell low on the trunk for thickness
                if i < height / 2 {
                    set(grid, pen.x - lean_dx, pen.y, diag, bark);
                }
            } else {
                pen.y -= 1;
                set(grid, pen.x, pen.y, '│', params.trunk_color);
            }
            nodes.push(TrunkNode {
                x: pen.x,
                y: pen.y,
                dir: MoveDir::Up,
            });
        }
        nodes
    }

    fn should_branch(
        &self,
        idx: usize,
        count: usize,
        params: &TreeParams,
        rng: &mut StdRng,
    ) -> Option<BranchIntent> {
        if count < 4 || idx < count / 4 {
            return None;
        }
        if idx != count - 1 && rng.random::<f32>() > 0.55 * params.branch_factor.max(0.4) {
            return None;
        }
        // every branch streams leeward
        let go_left = !self.lean_right;
        let top_frac = idx as f32 / count as f32;
        let length = (params.spread() as f32 * (0.7 + top_frac * 0.8)) as i32;
        Some(BranchIntent {
            go_left,
            length: length.max(3),
            level: 0,
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
        let dx: i32 = if intent.go_left { -1 } else { 1 };
        let jc = if intent.go_left { '┤' } else { '├' };
        set(grid, pen.x, pen.y, jc, darken(params.trunk_color, 10));

        let c = params.branch_color;
        let mut cx = pen.x;
        let mut cy = pen.y;
        let mut tips = Vec::new();
        for step in 0..intent.length {
            cx += dx;
            // streamers sag mid-run and kick up at the very end
            if step == intent.length - 1 {
                cy -= 1;
                set(grid, cx, cy, if dx > 0 { '╱' } else { '╲' }, lighten(c, 25));
            } else if step > 2 && rng.random::<f32>() < 0.15 {
                cy += 1;
                set(grid, cx, cy, if dx > 0 { '╲' } else { '╱' }, c);
            } else {
                set(grid, cx, cy, '─', c);
            }
            // wisps trailing off the streamer
            if step > 1 && rng.random::<f32>() < 0.2 {
                set(grid, cx, cy - 1, '╴', darken(c, 30));
            }
        }
        tips.push((cx + dx, cy - 1));
        BranchResult { tips }
    }

    fn draw_tip(&self, grid: &mut Grid, x: i32, y: i32, params: &TreeParams) {
        set(grid, x, y, '╸', lighten(params.tip_color, 30));
    }

    fn draw_fruit(&self, grid: &mut Grid, x: i32, y: i32, params: &TreeParams, _rng: &mut StdRng) {
        // wind carries the fruit one cell past the tip
        let dx = if self.lean_right { 1 } else { -1 };
        set(grid, x + dx, y, '◌', params.fruit_color);
    }
}

pub struct FractalTree;
fn fractal_limb(
    grid: &mut Grid,
    x: f32,
    y: f32,
    slope: f32,
    len: f32,
    depth: usize,
    params: &TreeParams,
    tips: &mut Vec<(i32, i32)>,
    rng: &mut StdRng,
) {
    if len < 1.2 || depth > 5 {
        tips.push((x as i32, y as i32));
        return;
    }
    let c = params.color_at_depth(depth as f32 * 0.2);
    let mut px = x;
    let mut py = y;
    for _ in 0..len as i32 {
        let nx = px + slope * 1.6;
        let ny = py - 1.0;
        if nx < params.plot.x as f32
            || nx >= (params.plot.x + params.plot.w) as f32
            || ny < params.plot.y as f32
        {
            // limb hit the plot edge: end it here, no children
            tips.push((px as i32, py as i32));
            return;
        }
        px = nx;
        py = ny;
        let ch = if slope > 0.25 {
            '╱'
        } else if slope < -0.25 {
            '╲'
        } else {
            '│'
        };
        set(grid, px as i32, py as i32, ch, c);
    }
    let tilt = 0.6 + rng.random::<f32>() * 0.4;
    let child_len = len * (0.72 + rng.random::<f32>() * 0.12);
    for side in [-1.0f32, 1.0] {
        if rng.random::<f32>() > 0.12 {
            let jitter = (rng.random::<f32>() - 0.5) * 0.2;
            fractal_limb(
                grid,
                px,
                py,
                slope + side * tilt + jitter,
                child_len,
                depth + 1,
                params,
                tips,
                rng,
            );
        } else {
            tips.push((px as i32, py as i32));
        }
    }
}
impl TreeDrawer for FractalTree {
    fn draw_trunk(
        &self,
        grid: &mut Grid,
        pen: &mut TreePen,
        params: &TreeParams,
        rng: &mut StdRng,
    ) -> Vec<TrunkNode> {
        StraightTrunk {
            height_fraction: 0.25,
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
        pen: &mut TreePen,
        _intent: &BranchIntent,
        _depth: usize,
        params: &TreeParams,
        rng: &mut StdRng,
    ) -> BranchResult {
        let canopy_h = (pen.y - params.canopy_top()).max(4) as f32;
        let mut tips = Vec::new();
        let slope = (rng.random::<f32>() - 0.5) * 0.4;
        fractal_limb(
            grid,
            pen.x as f32,
            pen.y as f32,
            slope,
            canopy_h * 0.35,
            0,
            params,
            &mut tips,
            rng,
        );
        BranchResult { tips }
    }

    fn draw_tip(&self, grid: &mut Grid, x: i32, y: i32, params: &TreeParams) {
        set(grid, x, y, '✶', lighten(params.tip_color, 15));
    }

    fn draw_fruit(&self, grid: &mut Grid, x: i32, y: i32, params: &TreeParams, _rng: &mut StdRng) {
        set(grid, x, y - 1, '◦', params.fruit_color);
    }
}

pub struct LSystemTree;
fn lsystem_expand(energy: f32) -> String {
    let iters = if energy > 0.7 {
        4
    } else if energy > 0.45 {
        3
    } else {
        2
    };
    let mut s = String::from("X");
    for _ in 0..iters {
        let mut next = String::new();
        for ch in s.chars() {
            match ch {
                'X' => next.push_str("F[+X]F[-X]+X"),
                other => next.push(other),
            }
        }
        s = next;
    }
    s
}
impl TreeDrawer for LSystemTree {
    fn draw_trunk(
        &self,
        grid: &mut Grid,
        pen: &mut TreePen,
        params: &TreeParams,
        rng: &mut StdRng,
    ) -> Vec<TrunkNode> {
        StraightTrunk {
            height_fraction: 0.2,
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
        pen: &mut TreePen,
        _intent: &BranchIntent,
        _depth: usize,
        params: &TreeParams,
        rng: &mut StdRng,
    ) -> BranchResult {
        // 8 directions, N=0 clockwise; + turns left 45, - turns right 45
        const DXY: [(i32, i32); 8] = [
            (0, -1),
            (1, -1),
            (1, 0),
            (1, 1),
            (0, 1),
            (-1, 1),
            (-1, 0),
            (-1, -1),
        ];
        const GLYPH: [char; 8] = ['│', '╱', '─', '╲', '│', '╱', '─', '╲'];
        let s = lsystem_expand(params.energy);
        let mut x = pen.x;
        let mut y = pen.y;
        // whole-plant tilt: start one step off vertical half the time
        let mut dir: usize = match rng.random_range(0..4u32) {
            0 => 7,
            1 => 1,
            _ => 0,
        };
        let mut stack: Vec<(i32, i32, usize)> = Vec::new();
        let mut tips = Vec::new();
        let mut moves = 0;
        let floor = params.root().1;
        let in_plot = |px: i32, py: i32| {
            px >= params.plot.x as i32
                && px < (params.plot.x + params.plot.w) as i32
                && py >= params.plot.y as i32
                && py < floor
        };
        for ch in s.chars() {
            match ch {
                'F' => {
                    moves += 1;
                    if moves > 220 {
                        break;
                    }
                    let (dx, dy) = DXY[dir];
                    x += dx;
                    y += dy;
                    if in_plot(x, y) {
                        let depth_frac = (stack.len() as f32 / 4.0).min(1.0);
                        set(grid, x, y, GLYPH[dir], params.color_at_depth(depth_frac));
                    }
                }
                '+' => dir = (dir + 7) % 8,
                '-' => dir = (dir + 1) % 8,
                '[' => stack.push((x, y, dir)),
                ']' => {
                    if stack.len() >= 2 && in_plot(x, y) && rng.random::<f32>() < 0.4 {
                        tips.push((x, y));
                    }
                    if let Some((sx, sy, sd)) = stack.pop() {
                        x = sx;
                        y = sy;
                        dir = sd;
                    }
                }
                _ => {}
            }
        }
        tips.push((x, y));
        BranchResult { tips }
    }

    fn draw_tip(&self, grid: &mut Grid, x: i32, y: i32, params: &TreeParams) {
        set(grid, x, y, '✳', lighten(params.tip_color, 20));
    }

    fn draw_fruit(&self, grid: &mut Grid, x: i32, y: i32, params: &TreeParams, rng: &mut StdRng) {
        let dx = if rng.random_range(0..2u32) == 0 {
            -1
        } else {
            1
        };
        set(grid, x + dx, y, '✿', params.fruit_color);
    }
}

pub struct DragonTree;
/// Fold parity for segment i of the dragon curve: true = turn left.
fn dragon_turn_left(i: u32) -> bool {
    let b = i & i.wrapping_neg();
    (i & (b << 1)) == 0
}
fn dragon_arm(
    grid: &mut Grid,
    x0: i32,
    y0: i32,
    start_dir: usize,
    mirror: bool,
    segments: u32,
    params: &TreeParams,
    tips: &mut Vec<(i32, i32)>,
) {
    // 4 directions, N=0 clockwise. Corner char connects came-from + go-to.
    const DXY: [(i32, i32); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];
    let corner = |din: usize, dout: usize| -> char {
        if din == dout {
            return if din % 2 == 0 { '│' } else { '─' };
        }
        // exits: opposite of incoming movement, plus outgoing
        match (din, dout) {
            (0, 1) | (3, 2) => '╭',
            (0, 3) | (1, 2) => '╮',
            (2, 1) | (3, 0) => '╰',
            (2, 3) | (1, 0) => '╯',
            _ => '┼',
        }
    };
    let floor = params.root().1;
    let mut x = x0;
    let mut y = y0;
    let mut dir = start_dir;
    for i in 1..=segments {
        let left = dragon_turn_left(i) != mirror;
        let new_dir = if left { (dir + 3) % 4 } else { (dir + 1) % 4 };
        let frac = i as f32 / segments as f32;
        if x >= params.plot.x as i32
            && x < (params.plot.x + params.plot.w) as i32
            && y >= params.plot.y as i32
        {
            set(
                grid,
                x,
                y,
                corner(dir, new_dir),
                params.color_at_depth(frac),
            );
        }
        let (dx, dy) = DXY[new_dir];
        x += dx;
        y += dy;
        if y >= floor {
            break;
        }
        dir = new_dir;
        // fold-back points (two same turns in a row) sprout occasional tips
        if i > 4 && i % 16 == 0 {
            tips.push((x, y));
        }
    }
    tips.push((x, y));
}
impl TreeDrawer for DragonTree {
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
        pen: &mut TreePen,
        _intent: &BranchIntent,
        _depth: usize,
        params: &TreeParams,
        rng: &mut StdRng,
    ) -> BranchResult {
        let base = if params.energy > 0.6 { 40 } else { 20 };
        let mut tips = Vec::new();
        // arms unfold sideways from the apex, mirrored fold parity;
        // unequal lengths keep the two sides from twinning
        let seg_l = base + rng.random_range(0..16u32);
        let seg_r = base + rng.random_range(0..16u32);
        dragon_arm(
            grid,
            pen.x - 1,
            pen.y - 1,
            3,
            false,
            seg_l,
            params,
            &mut tips,
        );
        dragon_arm(
            grid,
            pen.x + 1,
            pen.y - 1,
            1,
            true,
            seg_r,
            params,
            &mut tips,
        );
        set(grid, pen.x, pen.y - 1, '┴', params.trunk_color);
        // small jitter arm straight up on some trees
        if rng.random_range(0..2u32) == 0 {
            dragon_arm(
                grid,
                pen.x,
                pen.y - 2,
                0,
                false,
                base / 2,
                params,
                &mut tips,
            );
        }
        BranchResult { tips }
    }

    fn draw_tip(&self, grid: &mut Grid, x: i32, y: i32, params: &TreeParams) {
        set(grid, x, y, '✦', lighten(params.tip_color, 20));
    }

    fn draw_fruit(&self, grid: &mut Grid, x: i32, y: i32, params: &TreeParams, _rng: &mut StdRng) {
        set(grid, x, y + 1, '◉', params.fruit_color);
    }
}

pub struct HelixTree;
impl TreeDrawer for HelixTree {
    fn draw_trunk(
        &self,
        grid: &mut Grid,
        pen: &mut TreePen,
        params: &TreeParams,
        rng: &mut StdRng,
    ) -> Vec<TrunkNode> {
        let height = ((params.plot.h as f32 * params.energy.clamp(0.3, 1.0)) as i32).max(6);
        // strand offset cycles with period 6: cross, part, hold, cross, ...
        const OFF: [i32; 6] = [0, 1, 1, 0, -1, -1];
        let bright = params.trunk_color;
        let dim = darken(params.bark_color, 12);
        let phase = rng.random_range(0..6u32) as usize;
        let mut nodes = Vec::new();
        let mut prev = OFF[phase % 6];
        for i in 0..height {
            let y = pen.y - i;
            let off = OFF[(i as usize + phase) % 6];
            if off == 0 {
                set(grid, pen.x, y, '╳', bright);
                nodes.push(TrunkNode {
                    x: pen.x,
                    y,
                    dir: MoveDir::Up,
                });
            } else {
                let da = off - prev;
                let ca = if da > 0 {
                    '╱'
                } else if da < 0 {
                    '╲'
                } else {
                    '│'
                };
                // mirrored strand: opposite offset and slope
                let cb = if da > 0 {
                    '╲'
                } else if da < 0 {
                    '╱'
                } else {
                    '│'
                };
                set(grid, pen.x + off, y, ca, bright);
                set(grid, pen.x - off, y, cb, dim);
            }
            prev = off;
        }
        // crown: strands part and curl outward
        let top = pen.y - height;
        set(grid, pen.x - 1, top, '╮', dim);
        set(grid, pen.x + 1, top, '╭', bright);
        set(grid, pen.x, top, '┴', bright);
        nodes.push(TrunkNode {
            x: pen.x,
            y: top,
            dir: MoveDir::Up,
        });
        nodes
    }

    fn should_branch(
        &self,
        idx: usize,
        count: usize,
        params: &TreeParams,
        rng: &mut StdRng,
    ) -> Option<BranchIntent> {
        // sprigs from the knots only; skip the lowest, alternate sides
        if idx == 0 {
            return None;
        }
        if idx < count - 1 && rng.random::<f32>() > 0.4 + params.branch_factor * 0.4 {
            return None;
        }
        let go_left = idx % 2 == 0;
        let length = (params.spread() / 2 + rng.random_range(0..3u32) as i32).max(2);
        Some(BranchIntent {
            go_left,
            length,
            level: 0,
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
        let dx = if intent.go_left { -1 } else { 1 };
        let c = params.branch_color;
        let mut cx = pen.x;
        let mut cy = pen.y;
        for _ in 0..intent.length {
            cx += dx;
            if rng.random::<f32>() < 0.4 {
                cy -= 1;
                set(grid, cx, cy, if dx > 0 { '╱' } else { '╲' }, c);
            } else {
                set(grid, cx, cy, '─', c);
            }
        }
        BranchResult {
            tips: vec![(cx + dx, cy)],
        }
    }

    fn draw_tip(&self, grid: &mut Grid, x: i32, y: i32, params: &TreeParams) {
        set(grid, x, y, '❉', lighten(params.tip_color, 15));
    }

    fn draw_fruit(&self, grid: &mut Grid, x: i32, y: i32, params: &TreeParams, _rng: &mut StdRng) {
        set(grid, x, y + 1, '○', params.fruit_color);
    }
}

pub const TREE_KIND_COUNT: usize = 24;
