use crate::color::*;
use crate::sprites::{MoveDir, TreePen};
use crate::types::*;
use crossterm::style::Color;
use rand::RngExt;
use rand::rngs::StdRng;
use std::cell::Cell as StdCell;

// ── Inputs ──────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub enum GrowDir {
    Up,
    UpLeft,
    UpRight,
    Outward,
}

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
    pub bole: Option<Bole>,
    pub taper: TaperKind,
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


// --- bole/trunk machinery ---
mod bole_pattern;
mod boles;
mod pack;
mod species;
mod species_exotic;
mod scene;
pub use bole_pattern::*;
pub use boles::*;
pub use pack::*;
pub use species::*;
pub use species_exotic::*;
pub use scene::*;

pub trait TreeDrawer {
    fn draw_trunk(
        &self,
        grid: &mut Grid,
        pen: &mut TreePen,
        params: &TreeParams,
        rng: &mut StdRng,
    ) -> Vec<TrunkNode>;

    /// idx = trunk node index, count = total trunk nodes.
    fn should_branch(
        &self,
        idx: usize,
        count: usize,
        params: &TreeParams,
        rng: &mut StdRng,
    ) -> Option<BranchIntent>;

    fn draw_branch(
        &self,
        grid: &mut Grid,
        pen: &mut TreePen,
        intent: &BranchIntent,
        depth: usize,
        params: &TreeParams,
        rng: &mut StdRng,
    ) -> BranchResult;

    fn draw_tip(&self, grid: &mut Grid, x: i32, y: i32, params: &TreeParams);

    fn draw_fruit(&self, grid: &mut Grid, x: i32, y: i32, params: &TreeParams, rng: &mut StdRng);

    /// Default growth loop: trunk → branches at intervals → tips → fruit.
    fn grow(&self, grid: &mut Grid, params: &TreeParams, rng: &mut StdRng) {
        let exit = if let Some(ref bole) = params.bole {
            bole.draw(grid, params, rng)
        } else {
            let (rx, ry) = params.root();
            BoleExit::point(rx, ry)
        };
        let (rx, ry) = draw_taper(grid, &exit, params.trunk_color, params.taper);
        let mut pen = TreePen::new(rx, ry, params.trunk_color);
        pen.last_dir = Some(MoveDir::Up);

        let trunk = self.draw_trunk(grid, &mut pen, params, rng);
        if trunk.is_empty() {
            return;
        }

        let trunk_len = trunk.len();
        let mut all_tips: Vec<(i32, i32)> = Vec::new();
        let mut apex_branched = false;

        for (i, node) in trunk.iter().enumerate() {
            if let Some(intent) = self.should_branch(i, trunk_len, params, rng) {
                // Pen at the trunk node -- draw_branch owns the junction and everything outward
                let mut bp = TreePen::new(node.x, node.y, params.trunk_color);
                bp.last_dir = Some(node.dir);

                let result = self.draw_branch(grid, &mut bp, &intent, 0, params, rng);
                all_tips.extend(result.tips);

                if i == trunk_len - 1 {
                    apex_branched = true;
                }
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


mod tests {
    use super::*;
    use crossterm::style::Color;
    use rand::SeedableRng;

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
        let green = Color::Rgb {
            r: 80,
            g: 140,
            b: 60,
        };
        TreeParams {
            plot: Rect {
                x: plot_x,
                y: plot_y,
                w: plot_w,
                h: plot_h,
            },
            energy: 0.9,
            trunk_color: green,
            bark_color: green,
            branch_color: green,
            tip_color: green,
            fruit_color: green,
            fruit_factor: 0.0,
            branch_factor: 0.7,
            direction: GrowDir::Up,
            bole: None,
            taper: TaperKind::default(),
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

    #[test]
    fn snapshot_pine_tree() {
        let mut grid = make_grid(40, 20);
        let tp = test_params(20, 1, 20, 18);
        let mut rng = StdRng::seed_from_u64(42);
        PineTree.grow(&mut grid, &tp, &mut rng);
        insta::assert_snapshot!("pine_tree_42", grid_to_string(&grid));
    }

    #[test]
    fn snapshot_willow_tree() {
        let mut grid = make_grid(40, 20);
        let tp = test_params(20, 1, 20, 18);
        let mut rng = StdRng::seed_from_u64(42);
        WillowTree.grow(&mut grid, &tp, &mut rng);
        insta::assert_snapshot!("willow_tree_42", grid_to_string(&grid));
    }

    #[test]
    fn snapshot_palm_tree() {
        let mut grid = make_grid(40, 20);
        let tp = test_params(20, 1, 20, 18);
        let mut rng = StdRng::seed_from_u64(42);
        PalmTree.grow(&mut grid, &tp, &mut rng);
        insta::assert_snapshot!("palm_tree_42", grid_to_string(&grid));
    }

    #[test]
    fn snapshot_oak_tree() {
        let mut grid = make_grid(40, 20);
        let tp = test_params(10, 1, 20, 18);
        let mut rng = StdRng::seed_from_u64(42);
        OakTree.grow(&mut grid, &tp, &mut rng);
        insta::assert_snapshot!("oak_tree_42", grid_to_string(&grid));
    }

    #[test]
    fn snapshot_fountain_tree() {
        let mut grid = make_grid(40, 20);
        let tp = test_params(10, 1, 20, 18);
        let mut rng = StdRng::seed_from_u64(42);
        FountainTree.grow(&mut grid, &tp, &mut rng);
        insta::assert_snapshot!("fountain_tree_42", grid_to_string(&grid));
    }

    #[test]
    fn snapshot_windswept_tree() {
        let mut grid = make_grid(40, 20);
        let tp = test_params(10, 1, 20, 18);
        let mut rng = StdRng::seed_from_u64(42);
        WindsweptTree { lean_right: true }.grow(&mut grid, &tp, &mut rng);
        insta::assert_snapshot!("windswept_tree_42", grid_to_string(&grid));
    }

    #[test]
    fn snapshot_fractal_tree() {
        let mut grid = make_grid(40, 20);
        let tp = test_params(10, 1, 20, 18);
        let mut rng = StdRng::seed_from_u64(42);
        FractalTree.grow(&mut grid, &tp, &mut rng);
        insta::assert_snapshot!("fractal_tree_42", grid_to_string(&grid));
    }

    #[test]
    fn snapshot_lsystem_tree() {
        let mut grid = make_grid(40, 20);
        let tp = test_params(10, 1, 20, 18);
        let mut rng = StdRng::seed_from_u64(42);
        LSystemTree.grow(&mut grid, &tp, &mut rng);
        insta::assert_snapshot!("lsystem_tree_42", grid_to_string(&grid));
    }

    #[test]
    fn snapshot_dragon_tree() {
        let mut grid = make_grid(40, 20);
        let tp = test_params(10, 1, 20, 18);
        let mut rng = StdRng::seed_from_u64(42);
        DragonTree.grow(&mut grid, &tp, &mut rng);
        insta::assert_snapshot!("dragon_tree_42", grid_to_string(&grid));
    }

    #[test]
    fn snapshot_helix_tree() {
        let mut grid = make_grid(40, 20);
        let tp = test_params(10, 1, 20, 18);
        let mut rng = StdRng::seed_from_u64(42);
        HelixTree.grow(&mut grid, &tp, &mut rng);
        insta::assert_snapshot!("helix_tree_42", grid_to_string(&grid));
    }

    #[test]
    fn snapshot_winding_boles() {
        // all four winding bole styles (24-27) on one grid
        let mut grid = make_grid(80, 10);
        let mut rng = StdRng::seed_from_u64(42);
        for (i, style) in (24..28).enumerate() {
            let tp = test_params(i * 20, 1, 18, 8);
            let bole = Bole { style };
            bole.draw(&mut grid, &tp, &mut rng);
        }
        insta::assert_snapshot!("winding_boles_42", grid_to_string(&grid));
    }

    #[test]
    fn snapshot_structural_boles() {
        // all six structural bole styles (28-33) on one grid
        let mut grid = make_grid(120, 12);
        let mut rng = StdRng::seed_from_u64(42);
        for (i, style) in (28..34).enumerate() {
            let tp = test_params(i * 20, 1, 18, 8);
            let bole = Bole { style };
            bole.draw(&mut grid, &tp, &mut rng);
        }
        insta::assert_snapshot!("structural_boles_42", grid_to_string(&grid));
    }
}

