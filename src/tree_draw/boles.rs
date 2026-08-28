//! Bole/trunk styles and taper caps.
use crate::color::*;
use crate::sprites::*;
use crate::types::*;
use crossterm::style::Color;
use rand::RngExt;
use rand::rngs::StdRng;
use std::cell::Cell;
use super::*;

pub struct BoleExit {
    pub x: i32,
    pub y: i32,
    pub left: i32,  // half-width extending left of x (0 = just center)
    pub right: i32, // half-width extending right of x (0 = just center)
}
impl BoleExit {
    pub fn point(x: i32, y: i32) -> Self {
        BoleExit {
            x,
            y,
            left: 0,
            right: 0,
        }
    }
}
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

#[derive(Clone, Copy, Debug)]
pub enum TaperKind {
    Diagonal, // classic ╱─│─╲ triangle
    Shelf,    // └──┬──┘ horizontal ledges stepping inward
    Bracket,  // ╭───╮ / ╰─┴─╯ curved cradle
    Step,     // ├──┼──┤ rectangular frames shrinking per row
    Melt,     // braille density fade
}
impl Default for TaperKind {
    fn default() -> Self {
        TaperKind::Diagonal
    }
}
pub(crate) fn draw_taper(grid: &mut Grid, exit: &BoleExit, color: Color, kind: TaperKind) -> (i32, i32) {
    if exit.left == 0 && exit.right == 0 {
        set(grid, exit.x, exit.y, '│', color);
        return (exit.x, exit.y);
    }
    match kind {
        TaperKind::Diagonal => taper_diagonal(grid, exit, color),
        TaperKind::Shelf => taper_shelf(grid, exit, color),
        TaperKind::Bracket => taper_bracket(grid, exit, color),
        TaperKind::Step => taper_step(grid, exit, color),
        TaperKind::Melt => taper_melt(grid, exit, color),
    }
}
pub(crate) fn taper_diagonal(grid: &mut Grid, exit: &BoleExit, color: Color) -> (i32, i32) {
    let mut left = exit.left;
    let mut right = exit.right;
    let mut cy = exit.y;
    let bark = darken(color, 15);

    while left > 0 || right > 0 {
        if left > 0 {
            set(grid, exit.x - left, cy, '╱', bark);
            for dx in 1..left {
                set(grid, exit.x - dx, cy, '─', bark);
            }
        }
        if right > 0 {
            set(grid, exit.x + right, cy, '╲', bark);
            for dx in 1..right {
                set(grid, exit.x + dx, cy, '─', bark);
            }
        }
        set(grid, exit.x, cy, '│', color);

        let dl = if left + right > 6 {
            (left + 1) / 2
        } else {
            1.min(left)
        };
        let dr = if left + right > 6 {
            (right + 1) / 2
        } else {
            1.min(right)
        };
        left -= dl;
        right -= dr;
        cy -= 1;
    }
    set(grid, exit.x, cy, '│', color);
    (exit.x, cy)
}
pub(crate) fn taper_shelf(grid: &mut Grid, exit: &BoleExit, color: Color) -> (i32, i32) {
    let mut left = exit.left;
    let mut right = exit.right;
    let mut cy = exit.y;
    let bark = darken(color, 15);

    while left > 0 || right > 0 {
        // Horizontal shelf with corner brackets
        set(grid, exit.x - left, cy, '└', bark);
        set(grid, exit.x + right, cy, '┘', bark);
        for dx in (-left + 1)..right {
            set(grid, exit.x + dx, cy, '─', bark);
        }
        set(grid, exit.x, cy, '┬', color);

        let dl = if left + right > 6 {
            (left + 1) / 2
        } else {
            1.min(left)
        };
        let dr = if left + right > 6 {
            (right + 1) / 2
        } else {
            1.min(right)
        };
        left -= dl;
        right -= dr;
        cy -= 1;
    }
    set(grid, exit.x, cy, '│', color);
    (exit.x, cy)
}
pub(crate) fn taper_bracket(grid: &mut Grid, exit: &BoleExit, color: Color) -> (i32, i32) {
    let mut left = exit.left;
    let mut right = exit.right;
    let mut cy = exit.y;
    let bark = darken(color, 15);

    // Bottom row: open bracket ╰───┴───╯
    set(grid, exit.x - left, cy, '╰', bark);
    set(grid, exit.x + right, cy, '╯', bark);
    for dx in (-left + 1)..right {
        set(grid, exit.x + dx, cy, '─', bark);
    }
    set(grid, exit.x, cy, '┴', color);
    cy -= 1;

    let dl = if left + right > 6 {
        (left + 1) / 2
    } else {
        1.min(left)
    };
    let dr = if left + right > 6 {
        (right + 1) / 2
    } else {
        1.min(right)
    };
    left -= dl;
    right -= dr;

    // Middle rows: vertical walls │   │
    while left > 0 || right > 0 {
        if left > 0 {
            set(grid, exit.x - left, cy, '│', bark);
        }
        if right > 0 {
            set(grid, exit.x + right, cy, '│', bark);
        }
        set(grid, exit.x, cy, '│', color);

        let dl = if left + right > 4 {
            (left + 1) / 2
        } else {
            1.min(left)
        };
        let dr = if left + right > 4 {
            (right + 1) / 2
        } else {
            1.min(right)
        };
        left -= dl;
        right -= dr;
        cy -= 1;
    }

    // Top row: closing bracket ╭─╮
    if exit.left > 0 || exit.right > 0 {
        set(grid, exit.x, cy, '│', color);
    }
    (exit.x, cy)
}
pub(crate) fn taper_step(grid: &mut Grid, exit: &BoleExit, color: Color) -> (i32, i32) {
    let mut left = exit.left;
    let mut right = exit.right;
    let mut cy = exit.y;
    let bark = darken(color, 15);

    while left > 0 || right > 0 {
        // Rectangular step: ├──┼──┤
        set(grid, exit.x - left, cy, '├', bark);
        set(grid, exit.x + right, cy, '┤', bark);
        for dx in (-left + 1)..right {
            set(grid, exit.x + dx, cy, '═', lighten(bark, 5));
        }
        set(grid, exit.x, cy, '╪', color);

        // Shrink by 1 each side per row (slower, more steps visible)
        left = (left - 1).max(0);
        right = (right - 1).max(0);
        cy -= 1;
    }
    set(grid, exit.x, cy, '│', color);
    (exit.x, cy)
}
pub(crate) fn taper_melt(grid: &mut Grid, exit: &BoleExit, color: Color) -> (i32, i32) {
    let mut left = exit.left;
    let mut right = exit.right;
    let mut cy = exit.y;
    let bark = darken(color, 15);
    let dense = ['⣿', '⣾', '⣷', '⣶'];
    let mid = ['⡇', '⢸', '⠿', '⠶'];
    let thin = ['⠃', '⠆', '⠁', '⠈'];

    let total_rows = (left.max(right) + 1) as usize;
    let mut row = 0;
    while left > 0 || right > 0 {
        let frac = row as f32 / total_rows as f32;
        let palette = if frac < 0.33 {
            &dense[..]
        } else if frac < 0.66 {
            &mid[..]
        } else {
            &thin[..]
        };

        for dx in -left..=right {
            if dx == 0 {
                set(grid, exit.x, cy, '│', color);
            } else {
                let idx = ((dx.unsigned_abs() as usize + row) % palette.len()) as usize;
                let c = if frac < 0.5 {
                    bark
                } else {
                    lighten(bark, (frac * 30.0) as u8)
                };
                set(grid, exit.x + dx, cy, palette[idx], c);
            }
        }

        let dl = if left + right > 6 {
            (left + 1) / 2
        } else {
            1.min(left)
        };
        let dr = if left + right > 6 {
            (right + 1) / 2
        } else {
            1.min(right)
        };
        left -= dl;
        right -= dr;
        cy -= 1;
        row += 1;
    }
    set(grid, exit.x, cy, '│', color);
    (exit.x, cy)
}

pub trait TrunkAlgo {
    fn draw(
        &self,
        grid: &mut Grid,
        pen: &mut TreePen,
        params: &TreeParams,
        rng: &mut StdRng,
    ) -> Vec<TrunkNode>;
}

pub struct StraightTrunk {
    pub height_fraction: f32,
}
impl TrunkAlgo for StraightTrunk {
    fn draw(
        &self,
        grid: &mut Grid,
        pen: &mut TreePen,
        params: &TreeParams,
        _rng: &mut StdRng,
    ) -> Vec<TrunkNode> {
        let top_y = params.canopy_top();
        let ry = params.root().1;
        let full_height = (ry - top_y).max(1);
        let height = (full_height as f32 * self.height_fraction) as i32;
        let mut path = Vec::with_capacity(height as usize);

        for _ in 0..height {
            pen.step(grid, MoveDir::Up);
            path.push(TrunkNode {
                x: pen.x,
                y: pen.y,
                dir: MoveDir::Up,
            });
        }

        path
    }
}
pub struct ThickTrunk {
    pub height_fraction: f32,
}
impl TrunkAlgo for ThickTrunk {
    fn draw(
        &self,
        grid: &mut Grid,
        pen: &mut TreePen,
        params: &TreeParams,
        _rng: &mut StdRng,
    ) -> Vec<TrunkNode> {
        let top_y = params.canopy_top();
        let ry = params.root().1;
        let full_height = (ry - top_y).max(3);
        let trunk_h = (full_height as f32 * self.height_fraction) as i32;
        let bark = darken(params.trunk_color, 15);
        let mut path = Vec::with_capacity(trunk_h as usize);

        for _ in 0..trunk_h {
            pen.step(grid, MoveDir::Up);
            set(grid, pen.x, pen.y, '┃', params.trunk_color);
            set(grid, pen.x - 1, pen.y, '│', bark);
            set(grid, pen.x + 1, pen.y, '│', bark);
            path.push(TrunkNode {
                x: pen.x,
                y: pen.y,
                dir: MoveDir::Up,
            });
        }

        path
    }
}
pub struct WobbleTrunk {
    pub height_fraction: f32,
}
impl TrunkAlgo for WobbleTrunk {
    fn draw(
        &self,
        grid: &mut Grid,
        pen: &mut TreePen,
        params: &TreeParams,
        rng: &mut StdRng,
    ) -> Vec<TrunkNode> {
        let top_y = params.canopy_top();
        let ry = params.root().1;
        let full_height = (ry - top_y).max(3);
        let trunk_h = (full_height as f32 * self.height_fraction).max(2.0) as i32;
        let freq = rng.random_range(3..6u32) as i32;
        let mut path = Vec::with_capacity(trunk_h as usize);

        for i in 0..trunk_h {
            if i > 0 && i % freq == 0 && rng.random_range(0..3u32) == 0 {
                let h_dir = if rng.random::<bool>() {
                    MoveDir::Right
                } else {
                    MoveDir::Left
                };
                pen.step(grid, h_dir);
                pen.step(grid, MoveDir::Up);
            } else {
                pen.step(grid, MoveDir::Up);
            }
            path.push(TrunkNode {
                x: pen.x,
                y: pen.y,
                dir: MoveDir::Up,
            });
        }

        path
    }
}
pub struct LeanTrunk {
    pub lean: StdCell<i32>,
}
impl LeanTrunk {
    pub fn new() -> Self {
        LeanTrunk {
            lean: StdCell::new(0),
        }
    }
}
impl TrunkAlgo for LeanTrunk {
    fn draw(
        &self,
        grid: &mut Grid,
        pen: &mut TreePen,
        params: &TreeParams,
        rng: &mut StdRng,
    ) -> Vec<TrunkNode> {
        let top_y = params.canopy_top();
        let ry = params.root().1;
        let height = (ry - top_y).max(1);
        let spread = params.spread().max(2);
        let lean: i32 = if rng.random_range(0..2u32) == 0 {
            1
        } else {
            -1
        };
        self.lean.set(lean);
        let lean_every = (height / (spread.min(8))).max(2);
        let mut path = Vec::with_capacity(height as usize);

        let mut shifts = 0i32;
        for y in (top_y..=ry).rev() {
            let rows_from_root = ry - y;
            let new_shifts = rows_from_root / lean_every;

            if new_shifts > shifts {
                shifts = new_shifts;
                let h_dir = if lean > 0 {
                    MoveDir::Right
                } else {
                    MoveDir::Left
                };
                pen.step(grid, h_dir);
                pen.step(grid, MoveDir::Up);
            } else {
                pen.step(grid, MoveDir::Up);
            }

            path.push(TrunkNode {
                x: pen.x,
                y: pen.y,
                dir: MoveDir::Up,
            });
        }

        path
    }
}
pub struct GnarledTrunk;
impl TrunkAlgo for GnarledTrunk {
    fn draw(
        &self,
        grid: &mut Grid,
        pen: &mut TreePen,
        params: &TreeParams,
        rng: &mut StdRng,
    ) -> Vec<TrunkNode> {
        let top_y = params.canopy_top();
        let ry = params.root().1;
        let height = (ry - top_y).max(1);
        let trunk_color = darken(params.trunk_color, 10);
        let mut path = Vec::with_capacity(height as usize);

        pen.color = trunk_color;
        for i in 0..height {
            let from_root = height - i;
            if from_root > 2 && from_root % 7 == 0 && rng.random_range(0..3u32) == 0 {
                let h_dir = if rng.random::<bool>() {
                    MoveDir::Right
                } else {
                    MoveDir::Left
                };
                pen.step(grid, h_dir);
                pen.step(grid, MoveDir::Up);
            } else {
                pen.step(grid, MoveDir::Up);
            }
            path.push(TrunkNode {
                x: pen.x,
                y: pen.y,
                dir: MoveDir::Up,
            });
        }

        path
    }
}
/// Trunk that wanders laterally with organic S-curves using diagonal directions.
/// Nodes record actual travel direction so branches sprout naturally.
pub struct OrganicTrunk {
    pub height_fraction: f32,
}
impl TrunkAlgo for OrganicTrunk {
    fn draw(
        &self,
        grid: &mut Grid,
        pen: &mut TreePen,
        params: &TreeParams,
        rng: &mut StdRng,
    ) -> Vec<TrunkNode> {
        let top_y = params.canopy_top();
        let ry = params.root().1;
        let rx = params.root().0;
        let full_height = (ry - top_y).max(3);
        let trunk_h = (full_height as f32 * self.height_fraction).max(3.0) as i32;
        let max_drift = (params.spread() / 3).max(2);
        let mut path = Vec::with_capacity(trunk_h as usize);
        let mut drift: i32 = 0;
        // Pick a wander bias that flips every few steps
        let mut bias: i32 = if rng.random::<bool>() { 1 } else { -1 };
        let flip_every = rng.random_range(3..7u32) as i32;

        for i in 0..trunk_h {
            // Flip bias periodically for S-curve
            if i > 0 && i % flip_every == 0 {
                bias = -bias;
            }
            // Decide direction: mostly up, sometimes diagonal
            let dir = if i < 2 {
                // First 2 steps always straight up for clean base
                MoveDir::Up
            } else if rng.random_range(0..3u32) == 0 && drift.abs() < max_drift {
                if bias > 0 {
                    MoveDir::UpRight
                } else {
                    MoveDir::UpLeft
                }
            } else if drift.abs() >= max_drift {
                // Correct back toward center
                if drift > 0 {
                    MoveDir::UpLeft
                } else {
                    MoveDir::UpRight
                }
            } else {
                MoveDir::Up
            };

            pen.step(grid, dir);
            drift += dir.dx();
            path.push(TrunkNode {
                x: pen.x,
                y: pen.y,
                dir,
            });
        }

        path
    }
}
/// Trunk that follows a sine wave, creating regular undulation.
/// Uses diagonal steps at wave peaks for smooth curves.
pub struct SineTrunk {
    pub height_fraction: f32,
    pub amplitude: i32,
}
impl TrunkAlgo for SineTrunk {
    fn draw(
        &self,
        grid: &mut Grid,
        pen: &mut TreePen,
        params: &TreeParams,
        rng: &mut StdRng,
    ) -> Vec<TrunkNode> {
        let top_y = params.canopy_top();
        let ry = params.root().1;
        let full_height = (ry - top_y).max(3);
        let trunk_h = (full_height as f32 * self.height_fraction).max(3.0) as i32;
        let amp = self.amplitude.max(1).min(params.spread() / 2);
        let period = rng.random_range(4..9u32) as f32;
        let phase = rng.random_range(0..628u32) as f32 / 100.0; // 0..2π
        let mut path = Vec::with_capacity(trunk_h as usize);
        let mut prev_target_x = 0i32;
        let mut rows_drawn = 0i32;

        // Track actual y rows consumed, not loop iterations.
        // Horizontal shifts cost an extra row, so budget accordingly.
        let mut i = 0;
        while rows_drawn < trunk_h {
            let t = i as f32 / period;
            let target_x = ((t + phase).sin() * amp as f32).round() as i32;
            let dx = (target_x - prev_target_x).clamp(-1, 1);
            prev_target_x = target_x;
            i += 1;

            if dx != 0 {
                let h_dir = if dx < 0 {
                    MoveDir::Left
                } else {
                    MoveDir::Right
                };
                pen.step(grid, h_dir);
                pen.step(grid, MoveDir::Up);
                rows_drawn += 1;
            } else {
                pen.step(grid, MoveDir::Up);
                rows_drawn += 1;
            }
            path.push(TrunkNode {
                x: pen.x,
                y: pen.y,
                dir: MoveDir::Up,
            });
        }

        path
    }
}

pub trait BoleStyle {
    fn draw(&self, grid: &mut Grid, params: &TreeParams, rng: &mut StdRng) -> BoleExit;
}
/// No bole
pub struct NoBole;
impl BoleStyle for NoBole {
    fn draw(&self, _grid: &mut Grid, params: &TreeParams, _rng: &mut StdRng) -> BoleExit {
        let (x, y) = params.root();
        BoleExit::point(x, y)
    }
}
/// Procedural bole: generates a compact sprite pattern at the trunk base.
/// `style` selects the char family. `width` controls horizontal spread.
/// Each style is a coherent glyph vocabulary like the flower sprites.
#[derive(Clone, Copy)]
pub struct Bole {
    pub style: usize,
}
/// Bole pattern: array of (dx, dy, char) offsets from center, like draw_flower.
/// Generated procedurally based on width + style + rng.
impl BoleStyle for Bole {
    fn draw(&self, grid: &mut Grid, params: &TreeParams, rng: &mut StdRng) -> BoleExit {
        let (root_x, root_y) = params.root();
        let color = params.trunk_color;
        let max_w = (params.spread() as i32).max(2);
        let w = ((max_w as f32 * params.energy.clamp(0.3, 1.0)) as i32).max(2);
        let lw = (w / 2 + rng.random_range(0..(w / 2 + 1).max(1) as u32) as i32)
            .max(1)
            .min(max_w);
        let rw = (w - lw + rng.random_range(0..(w / 3 + 1).max(1) as u32) as i32)
            .max(1)
            .min(max_w);
        let bark = darken(color, 15);
        let dim = darken(color, 30);
        draw_bole_pattern(
            grid,
            root_x,
            root_y,
            lw,
            rw,
            color,
            bark,
            dim,
            params.energy,
            self.style,
            rng,
            true,
        )
    }
}
/// Gradient direction for bush color fading.
#[derive(Clone, Copy, Debug)]
pub enum FadeDir {
    Down,      // crown bright, base fades toward ground
    Up,        // base bright, crown fades
    CenterOut, // core bright, all edges fade toward ground
}
/// Standalone bush sprite: renders full-size bole patterns as independent shrubs.
/// Not attached to trees -- takes raw coordinates instead of TreeParams.
pub struct BushSprite {
    pub style: usize,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub color: Color,
    pub ground: Color,
    pub fade: FadeDir,
    pub energy: f32,
}
impl BushSprite {
    pub fn draw(&self, grid: &mut Grid, rng: &mut StdRng) -> BoleExit {
        let w = self.width.max(2);
        let lw = w / 2 + rng.random_range(0..(w / 2 + 1).max(1) as u32) as i32;
        let rw = w - lw + rng.random_range(0..(w / 3 + 1).max(1) as u32) as i32;
        let bark = darken(self.color, 15);
        let dim = darken(self.color, 30);
        let exit = draw_bole_pattern(
            grid,
            self.x,
            self.y,
            lw.max(1),
            rw.max(1),
            self.color,
            bark,
            dim,
            self.energy,
            self.style,
            rng,
            false,
        );

        // Post-pass: gradient fade toward ground color
        self.apply_fade(grid, &exit);

        exit
    }

    fn apply_fade(&self, grid: &mut Grid, exit: &BoleExit) {
        // Compute bounding box from exit info + root position
        let left = self.x - exit.left - 3; // padding for sprawl chars
        let right = self.x + exit.right + 3;
        let top = exit.y - 1;
        let bot = self.y + 2; // some styles draw below root

        let h = (bot - top).max(1) as f32;
        let half_w = ((right - left) / 2).max(1) as f32;

        for gy in top.max(0)..=bot.min(grid.len() as i32 - 1) {
            let row = gy as usize;
            for gx in left.max(0)..=right.min(grid[0].len() as i32 - 1) {
                let col = gx as usize;
                if grid[row][col].ch == ' ' {
                    continue;
                }

                let dx = (gx - self.x).abs() as f32;
                let dy_from_top = (gy - top) as f32;
                let dy_from_bot = (bot - gy) as f32;

                let t = match self.fade {
                    FadeDir::Down => dy_from_top / h, // 0 at top, 1 at bottom
                    FadeDir::Up => dy_from_bot / h,   // 0 at bottom, 1 at top
                    FadeDir::CenterOut => {
                        // radial: distance from center normalized to 0..1
                        let nx = dx / half_w;
                        let ny = ((gy - self.y).abs() as f32) / (h * 0.5).max(1.0);
                        (nx * nx + ny * ny).sqrt().min(1.0)
                    }
                };

                // Blend: t=0 keeps original color, t=1 fully ground color
                // Use a softer curve (t^0.7) so center stays vivid longer
                let blend = t.powf(0.7).min(0.85); // never fully erase to ground
                grid[row][col].fg = lerp_color(grid[row][col].fg, self.ground, blend);
            }
        }
    }
}
pub struct TreeWithTrunk<T: TreeDrawer> {
    pub tree: T,
    pub trunk: Box<dyn TrunkAlgo>,
}

impl<T: TreeDrawer> TreeDrawer for TreeWithTrunk<T> {
    fn draw_trunk(
        &self,
        grid: &mut Grid,
        pen: &mut TreePen,
        params: &TreeParams,
        rng: &mut StdRng,
    ) -> Vec<TrunkNode> {
        self.trunk.draw(grid, pen, params, rng)
    }

    fn should_branch(
        &self,
        idx: usize,
        count: usize,
        params: &TreeParams,
        rng: &mut StdRng,
    ) -> Option<BranchIntent> {
        self.tree.should_branch(idx, count, params, rng)
    }

    fn draw_branch(
        &self,
        grid: &mut Grid,
        pen: &mut TreePen,
        intent: &BranchIntent,
        depth: usize,
        params: &TreeParams,
        rng: &mut StdRng,
    ) -> BranchResult {
        self.tree.draw_branch(grid, pen, intent, depth, params, rng)
    }

    fn draw_tip(&self, grid: &mut Grid, x: i32, y: i32, params: &TreeParams) {
        self.tree.draw_tip(grid, x, y, params);
    }

    fn draw_fruit(&self, grid: &mut Grid, x: i32, y: i32, params: &TreeParams, rng: &mut StdRng) {
        self.tree.draw_fruit(grid, x, y, params, rng);
    }
}

