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

// ── Taper styles ────────────────────────────────────────────────────
// Connects wide bole exit to narrow trunk. Each variant has a different look.

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

fn draw_taper(grid: &mut Grid, exit: &BoleExit, color: Color, kind: TaperKind) -> (i32, i32) {
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

fn taper_diagonal(grid: &mut Grid, exit: &BoleExit, color: Color) -> (i32, i32) {
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

fn taper_shelf(grid: &mut Grid, exit: &BoleExit, color: Color) -> (i32, i32) {
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

fn taper_bracket(grid: &mut Grid, exit: &BoleExit, color: Color) -> (i32, i32) {
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

fn taper_step(grid: &mut Grid, exit: &BoleExit, color: Color) -> (i32, i32) {
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

fn taper_melt(grid: &mut Grid, exit: &BoleExit, color: Color) -> (i32, i32) {
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

// ── TrunkAlgo ───────────────────────────────────────────────────────────

pub trait TrunkAlgo {
    fn draw(
        &self,
        grid: &mut Grid,
        pen: &mut TreePen,
        params: &TreeParams,
        rng: &mut StdRng,
    ) -> Vec<TrunkNode>;
}

// ── Trunk Algorithms ────────────────────────────────────────────────────

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

// ── Bole styles (trunk base/flare at ground level) ──────────────────
// Each bole is a walk-based algorithm that grows outward from trunk center.
// Returns (x, y) where the vertical trunk should connect above.

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

/// Shared bole/bush pattern renderer. 34 style variants.
/// `compact`: true clamps layer counts to keep height <= 3 rows (for tree boles).
///            false renders full size (for standalone bush sprites).
fn draw_bole_pattern(
    grid: &mut Grid,
    root_x: i32,
    root_y: i32,
    lw: i32,
    rw: i32,
    color: Color,
    bark: Color,
    dim: Color,
    energy: f32,
    style: usize,
    rng: &mut StdRng,
    compact: bool,
) -> BoleExit {
    match style % 34 {
        // Style 0: Crescent -- connected via │ at inner edge positions
        0 => {
            // Ground row: wide crescent
            set(grid, root_x, root_y, '┴', color);
            set(grid, root_x - 1, root_y, '◟', bark);
            set(grid, root_x + 1, root_y, '◞', bark);
            for dx in 2..=lw {
                set(
                    grid,
                    root_x - dx,
                    root_y,
                    '◠',
                    lighten(bark, ((dx - 2) as u8 * 8).min(40)),
                );
            }
            for dx in 2..=rw {
                set(
                    grid,
                    root_x + dx,
                    root_y,
                    '◠',
                    lighten(bark, ((dx - 2) as u8 * 8).min(40)),
                );
            }
            set(grid, root_x - lw - 1, root_y, '◜', lighten(bark, 30));
            set(grid, root_x + rw + 1, root_y, '◝', lighten(bark, 30));
            // Inner crescent with │ connectors down to ground row
            let ilw = (lw * 2 / 3).max(1);
            let irw = (rw * 2 / 3).max(1);
            set(grid, root_x - 1, root_y - 1, '◟', color);
            set(grid, root_x + 1, root_y - 1, '◞', color);
            for dx in 2..=ilw {
                set(grid, root_x - dx, root_y - 1, '◡', bark);
            }
            for dx in 2..=irw {
                set(grid, root_x + dx, root_y - 1, '◡', bark);
            }
            set(grid, root_x - ilw - 1, root_y - 1, '◜', bark);
            set(grid, root_x + irw + 1, root_y - 1, '◝', bark);
            // Vertical connectors: │ at inner crescent edges link to outer crescent
            set(grid, root_x - ilw - 1, root_y, '│', bark);
            set(grid, root_x + irw + 1, root_y, '│', bark);
            // Horizontal bar connecting crescents at mid-width
            let bar_l = (ilw + lw) / 2;
            let bar_r = (irw + rw) / 2;
            if bar_l > ilw + 1 {
                set(grid, root_x - bar_l, root_y - 1, '─', bark);
            }
            if bar_r > irw + 1 {
                set(grid, root_x + bar_r, root_y - 1, '─', bark);
            }
            BoleExit {
                x: root_x,
                y: root_y - 1,
                left: ilw,
                right: irw,
            }
        }
        // Style 1: Braille cluster -- compact 1-2 row spread
        1 => {
            let energy = energy.clamp(0.2, 1.0);
            let dense = ['⣿', '⣾', '⣷', '⣤', '⣶'];
            let thin = ['⡇', '⢸', '⠿', '⠶', '⠛'];
            let rows = if energy > 0.5 { 2 } else { 1 };
            let mut cy = root_y;

            // Ground row: dense, full width
            let hw = lw.max(rw);
            set(grid, root_x, cy, dense[0], color);
            for dx in 1..=hw {
                let ch = dense[rng.random_range(0..dense.len() as u32) as usize];
                set(
                    grid,
                    root_x - dx,
                    cy,
                    ch,
                    darken(color, ((dx as u8) * 3).min(15)),
                );
                set(
                    grid,
                    root_x + dx,
                    cy,
                    ch,
                    darken(color, ((dx as u8) * 3).min(15)),
                );
            }
            cy -= 1;

            // Optional second row: thin, half width
            if rows > 1 {
                let hw2 = (hw / 2).max(1);
                let row_col = darken(color, 10);
                set(grid, root_x, cy, thin[0], row_col);
                for dx in 1..=hw2 {
                    let ch = thin[rng.random_range(0..thin.len() as u32) as usize];
                    set(
                        grid,
                        root_x - dx,
                        cy,
                        ch,
                        darken(row_col, ((dx as u8) * 4).min(15)),
                    );
                    set(
                        grid,
                        root_x + dx,
                        cy,
                        ch,
                        darken(row_col, ((dx as u8) * 4).min(15)),
                    );
                }
                BoleExit {
                    x: root_x,
                    y: cy,
                    left: hw2,
                    right: hw2,
                }
            } else {
                BoleExit {
                    x: root_x,
                    y: cy + 1,
                    left: hw,
                    right: hw,
                }
            }
        }
        // Style 2: Frame -- energy-scaled nested box frames
        2 => {
            let energy = energy.clamp(0.2, 1.0);
            let hlw = lw.max(2);
            let hrw = rw.max(2);
            let layers = if compact {
                1
            } else {
                ((energy * 3.0).ceil() as i32).clamp(1, 3)
            };
            let mut cy = root_y;

            for layer in 0..layers {
                let shrink = layer as f32 * 0.3;
                let ll = ((hlw as f32) * (1.0 - shrink)).max(1.0) as i32;
                let lr = ((hrw as f32) * (1.0 - shrink)).max(1.0) as i32;
                let layer_dim = darken(bark, (layer as u8) * 12);
                let layer_col = if layer == 0 { bark } else { layer_dim };

                set(grid, root_x - ll, cy, '╚', layer_col);
                set(grid, root_x + lr, cy, '╝', layer_col);
                for dx in (-ll + 1)..lr {
                    let ch = if layer == 0 {
                        ['░', '▒', '░', '·'][rng.random_range(0..4u32) as usize]
                    } else {
                        ['▒', '▓', '█', '▒'][rng.random_range(0..4u32) as usize]
                    };
                    set(grid, root_x + dx, cy, ch, dim);
                }
                set(
                    grid,
                    root_x,
                    cy,
                    '╩',
                    if layer == 0 { color } else { layer_col },
                );

                cy -= 1;
                set(grid, root_x - ll, cy, '╔', layer_col);
                set(grid, root_x + lr, cy, '╗', layer_col);
                for dx in (-ll + 1)..lr {
                    set(grid, root_x + dx, cy, '═', layer_col);
                }
                set(
                    grid,
                    root_x,
                    cy,
                    '╦',
                    if layer == 0 { color } else { layer_col },
                );

                if layer == 0 && energy > 0.6 {
                    set(grid, root_x - ll - 1, root_y, '╱', dim);
                    set(grid, root_x + lr + 1, root_y, '╲', dim);
                }

                cy -= 1;
            }

            let last_shrink = (layers - 1) as f32 * 0.3;
            let exit_l = ((hlw as f32) * (1.0 - last_shrink)).max(1.0) as i32;
            let exit_r = ((hrw as f32) * (1.0 - last_shrink)).max(1.0) as i32;
            BoleExit {
                x: root_x,
                y: cy + 1,
                left: exit_l,
                right: exit_r,
            }
        }
        // Style 3: Diamond -- compact: wide ground row + 1-2 taper rows
        3 => {
            let energy = energy.clamp(0.2, 1.0);
            let max_half_w = lw.max(rw).max(2);
            let mut cy = root_y;

            // Ground row: widest point with diamond endpoints and arrow caps
            set(grid, root_x, cy, '◆', color);
            for dx in 1..=max_half_w {
                let ch = if dx == max_half_w { '◇' } else { '═' };
                let c = lighten(color, ((dx as u8) * 4).min(25));
                set(grid, root_x - dx, cy, ch, c);
                set(grid, root_x + dx, cy, ch, c);
            }
            set(grid, root_x - max_half_w - 1, cy, '◁', dim);
            set(grid, root_x + max_half_w + 1, cy, '▷', dim);
            cy -= 1;

            // 1-2 taper rows contracting upward
            let taper_rows = if energy > 0.5 { 2 } else { 1 };
            for row in 0..taper_rows {
                let hw = ((taper_rows - row) as f32 / (taper_rows + 1) as f32 * max_half_w as f32)
                    .ceil() as i32;
                let row_col = lighten(bark, ((row + 1) as u8 * 8).min(35));
                set(grid, root_x, cy, '│', row_col);
                for dx in 1..=hw {
                    let ch = if dx == hw {
                        '◇'
                    } else if dx % 2 == 0 {
                        '─'
                    } else {
                        '◆'
                    };
                    set(grid, root_x - dx, cy, ch, row_col);
                    set(grid, root_x + dx, cy, ch, row_col);
                }
                cy -= 1;
            }

            let exit_hw = (1.0f32 / (taper_rows + 1) as f32 * max_half_w as f32).ceil() as i32;
            BoleExit {
                x: root_x,
                y: cy + 1,
                left: exit_hw,
                right: exit_hw,
            }
        }
        // Style 4: Chevron -- energy-scaled layered V-shapes with variable center
        4 => {
            let energy = energy.clamp(0.2, 1.0);
            // Number of chevron layers: 1 at low, up to 4 at high
            let layers = if compact {
                1
            } else {
                ((energy * 3.5).ceil() as i32).clamp(1, 4)
            };
            let mut cy = root_y;

            // Ground row: base chevron V
            set(grid, root_x, cy, '┴', color);
            let ll = lw.max(2);
            let rl = rw.max(2);
            for dx in 1..=ll {
                let c = lighten(bark, ((dx as u8) * 5).min(35));
                set(grid, root_x - dx, cy, '╱', c);
            }
            for dx in 1..=rl {
                let c = lighten(bark, ((dx as u8) * 5).min(35));
                set(grid, root_x + dx, cy, '╲', c);
            }
            set(grid, root_x - ll - 1, cy, '─', dim);
            set(grid, root_x + rl + 1, cy, '─', dim);
            cy -= 1;

            // Stacked chevron layers, each narrower
            for layer in 0..layers {
                let shrink = (layer + 1) as f32 * 0.22;
                let cl = ((ll as f32) * (1.0 - shrink)).max(1.0) as i32;
                let cr = ((rl as f32) * (1.0 - shrink)).max(1.0) as i32;
                let lc = if layer == 0 {
                    bark
                } else {
                    lighten(bark, (layer as u8 * 8).min(30))
                };

                // Inverted V (∧ shape)
                let center_ch = match rng.random_range(0..3u32) {
                    0 => '∧',
                    1 => '△',
                    _ => '▵',
                };
                set(grid, root_x, cy, center_ch, color);
                for dx in 1..=cl {
                    set(grid, root_x - dx, cy, '╱', lc);
                }
                for dx in 1..=cr {
                    set(grid, root_x + dx, cy, '╲', lc);
                }
                // Horizontal stubs at tips
                if cl > 1 {
                    set(grid, root_x - cl - 1, cy, '─', lighten(lc, 15));
                }
                if cr > 1 {
                    set(grid, root_x + cr + 1, cy, '─', lighten(lc, 15));
                }

                cy -= 1;

                // Only add V shape between layers if not last
                if layer < layers - 1 {
                    let vcl = ((cl as f32) * 0.7).max(1.0) as i32;
                    let vcr = ((cr as f32) * 0.7).max(1.0) as i32;
                    let vc = match rng.random_range(0..3u32) {
                        0 => '∨',
                        1 => '▽',
                        _ => '▿',
                    };
                    set(grid, root_x, cy, vc, lc);
                    for dx in 1..=vcl {
                        set(grid, root_x - dx, cy, '╲', lighten(lc, 10));
                    }
                    for dx in 1..=vcr {
                        set(grid, root_x + dx, cy, '╱', lighten(lc, 10));
                    }
                    cy -= 1;
                }
            }

            BoleExit::point(root_x, cy + 1)
        }
        // Style 5: Frame2 -- connected/overlapping stacked frames, shared borders
        5 => {
            let energy = energy.clamp(0.2, 1.0);
            // Layer count: 1-3, varies with energy but not always max
            let max_layers = if compact {
                1
            } else {
                ((energy * 3.0).ceil() as i32).clamp(1, 3)
            };
            let layers = if max_layers > 1 {
                rng.random_range(1..(max_layers + 1) as u32) as i32
            } else {
                1
            };
            let mut cy = root_y;
            let mut cur_lw = lw.max(2);
            let mut cur_rw = rw.max(2);

            for layer in 0..layers {
                let layer_col = if layer == 0 { color } else { bark };
                let fill_col = if layer == 0 { bark } else { dim };
                // Variable height per layer: 1-3 interior rows
                let interior_h = if compact {
                    1
                } else if energy > 0.7 {
                    rng.random_range(1..4u32) as i32
                } else if energy > 0.4 {
                    rng.random_range(1..3u32) as i32
                } else {
                    1
                };

                // Bottom border (shared with previous layer's top if not first)
                if layer == 0 {
                    set(grid, root_x - cur_lw, cy, '╚', layer_col);
                    set(grid, root_x + cur_rw, cy, '╝', layer_col);
                    for dx in (-cur_lw + 1)..cur_rw {
                        set(grid, root_x + dx, cy, '═', layer_col);
                    }
                    set(grid, root_x, cy, '╩', color);
                    // Buttress legs at base
                    if energy > 0.5 {
                        set(grid, root_x - cur_lw - 1, cy, '╱', fill_col);
                        set(grid, root_x + cur_rw + 1, cy, '╲', fill_col);
                    }
                }

                // Interior fill rows
                for row in 0..interior_h {
                    cy -= 1;
                    set(grid, root_x - cur_lw, cy, '║', color);
                    set(grid, root_x + cur_rw, cy, '║', color);
                    let fills = if row == 0 {
                        ['░', '▒', '░', '·']
                    } else {
                        ['▒', '▓', '█', '▒']
                    };
                    for dx in (-cur_lw + 1)..cur_rw {
                        let ch = fills[rng.random_range(0..4u32) as usize];
                        set(grid, root_x + dx, cy, ch, layer_col);
                    }
                    set(grid, root_x, cy, '│', color);
                }

                // Top border / shared border with next layer
                cy -= 1;
                if layer < layers - 1 {
                    // Shared border: next layer is narrower, so draw T-junctions
                    let next_lw =
                        ((cur_lw as f32) * (0.55 + rng.random::<f32>() * 0.25)).max(1.0) as i32;
                    let next_rw =
                        ((cur_rw as f32) * (0.55 + rng.random::<f32>() * 0.25)).max(1.0) as i32;
                    // Full width top of current layer
                    set(grid, root_x - cur_lw, cy, '╔', layer_col);
                    set(grid, root_x + cur_rw, cy, '╗', layer_col);
                    for dx in (-cur_lw + 1)..cur_rw {
                        set(grid, root_x + dx, cy, '═', layer_col);
                    }
                    // Overwrite with junction chars where next layer's walls will be
                    set(grid, root_x - next_lw, cy, '╠', layer_col);
                    set(grid, root_x + next_rw, cy, '╣', layer_col);
                    set(grid, root_x, cy, '╬', color);
                    cur_lw = next_lw;
                    cur_rw = next_rw;
                } else {
                    // Final top border
                    set(grid, root_x - cur_lw, cy, '╔', layer_col);
                    set(grid, root_x + cur_rw, cy, '╗', layer_col);
                    for dx in (-cur_lw + 1)..cur_rw {
                        set(grid, root_x + dx, cy, '═', layer_col);
                    }
                    set(grid, root_x, cy, '╦', color);
                }
            }

            BoleExit {
                x: root_x,
                y: cy,
                left: cur_lw,
                right: cur_rw,
            }
        }
        // Style 6: Crescent2 -- turbo crescent with hips, valid box-drawing connections
        6 => {
            let energy = energy.clamp(0.2, 1.0);
            let layers = if compact {
                2
            } else {
                ((energy * 4.0).ceil() as i32).clamp(2, 5)
            };
            let mut cy = root_y;

            // Ground layer: widest crescent with hip flares
            set(grid, root_x, cy, '┴', color);
            for dx in 1..=lw {
                let ch = if dx <= 2 { '═' } else { '◠' };
                set(
                    grid,
                    root_x - dx,
                    cy,
                    ch,
                    lighten(color, ((dx as u8) * 3).min(25)),
                );
            }
            for dx in 1..=rw {
                let ch = if dx <= 2 { '═' } else { '◠' };
                set(
                    grid,
                    root_x + dx,
                    cy,
                    ch,
                    lighten(color, ((dx as u8) * 3).min(25)),
                );
            }
            // Hip flares: curved outward kicks
            set(grid, root_x - lw - 1, cy, '╮', bark);
            set(grid, root_x + rw + 1, cy, '╭', bark);
            if lw > 2 {
                set(grid, root_x - lw - 2, cy, '─', dim);
                set(grid, root_x - lw - 1, cy - 1, '│', bark);
                set(grid, root_x - lw - 1, cy + 1, '╯', dim);
            }
            if rw > 2 {
                set(grid, root_x + rw + 2, cy, '─', dim);
                set(grid, root_x + rw + 1, cy - 1, '│', bark);
                set(grid, root_x + rw + 1, cy + 1, '╰', dim);
            }
            cy -= 1;

            // Stacked crescent arcs, each narrower with random horizontal offsets
            for layer in 1..layers {
                let shrink = layer as f32 * 0.2;
                let ll = ((lw as f32) * (1.0 - shrink)).max(1.0) as i32;
                let lr = ((rw as f32) * (1.0 - shrink)).max(1.0) as i32;
                let offset = rng.random_range(0..3u32) as i32 - 1; // -1, 0, or 1
                let cx = root_x + offset;
                let lc = lighten(bark, (layer as u8 * 6).min(30));

                set(grid, cx, cy, '┴', lc);
                for dx in 1..=ll {
                    let ch = ['◠', '◡', '◟', '◞'][rng.random_range(0..4u32) as usize];
                    set(grid, cx - dx, cy, ch, lighten(lc, ((dx as u8) * 4).min(20)));
                }
                for dx in 1..=lr {
                    let ch = ['◠', '◡', '◟', '◞'][rng.random_range(0..4u32) as usize];
                    set(grid, cx + dx, cy, ch, lighten(lc, ((dx as u8) * 4).min(20)));
                }
                // Connect back to center if offset
                if offset != 0 {
                    set(grid, root_x, cy, '│', color);
                }
                // Nip details at crescent tips
                set(grid, cx - ll - 1, cy, '◜', lighten(lc, 15));
                set(grid, cx + lr + 1, cy, '◝', lighten(lc, 15));
                cy -= 1;
            }

            let last_shrink = (layers - 1) as f32 * 0.2;
            let exit_l = ((lw as f32) * (1.0 - last_shrink)).max(1.0) as i32;
            let exit_r = ((rw as f32) * (1.0 - last_shrink)).max(1.0) as i32;
            BoleExit {
                x: root_x,
                y: cy + 1,
                left: exit_l,
                right: exit_r,
            }
        }
        // Style 7: Braille2 -- thick braille with tapered trunk exit
        7 => {
            let energy = energy.clamp(0.2, 1.0);
            let dense = ['⣿', '⣾', '⣷', '⣶', '⣤'];
            let mid = ['⡇', '⢸', '⠿', '⠶', '⠛'];
            let thin = ['⡀', '⢀', '⠂', '⠈', '⠁'];
            let rows = if compact {
                2
            } else {
                ((energy * 4.0).ceil() as i32 + 1).clamp(2, 5)
            };
            let mut cy = root_y;
            let base_w = lw.max(rw).max(2);

            for row in 0..rows {
                let frac = row as f32 / rows as f32;
                let hw = ((base_w as f32) * (1.0 - frac * 0.5)).max(1.0) as i32;
                let chars = if frac < 0.3 {
                    &dense
                } else if frac < 0.6 {
                    &mid
                } else {
                    &thin
                };
                let rc = if row == 0 {
                    color
                } else {
                    darken(color, (row as u8 * 4).min(15))
                };

                // Asymmetric: left and right can have different widths
                let lhw = hw + rng.random_range(0..2u32) as i32;
                let rhw = hw + rng.random_range(0..2u32) as i32;

                set(grid, root_x, cy, chars[0], rc);
                for dx in 1..=lhw {
                    set(
                        grid,
                        root_x - dx,
                        cy,
                        chars[rng.random_range(0..chars.len() as u32) as usize],
                        darken(rc, ((dx as u8) * 2).min(10)),
                    );
                }
                for dx in 1..=rhw {
                    set(
                        grid,
                        root_x + dx,
                        cy,
                        chars[rng.random_range(0..chars.len() as u32) as usize],
                        darken(rc, ((dx as u8) * 2).min(10)),
                    );
                }
                cy -= 1;
            }

            // Taper: 1-2 rows of transition from thick braille to single │
            let taper_rows = if base_w > 3 { 2 } else { 1 };
            for t in 0..taper_rows {
                let tw = (taper_rows - t).min(2);
                let tc = ['⡇', '⢸', '│'][t as usize % 3];
                set(grid, root_x, cy, tc, bark);
                if tw > 1 {
                    set(grid, root_x - 1, cy, '⡀', dim);
                    set(grid, root_x + 1, cy, '⢀', dim);
                }
                cy -= 1;
            }

            let exit_hw = if base_w > 3 { 1 } else { 0 };
            BoleExit {
                x: root_x,
                y: cy + 1,
                left: exit_hw,
                right: exit_hw,
            }
        }
        // Style 8: Frame3 -- stacked boxes, heaviest at bottom, randomly off-center
        8 => {
            let energy = energy.clamp(0.2, 1.0);
            let boxes = if compact {
                1
            } else {
                ((energy * 3.0).ceil() as i32).clamp(1, 4)
            };
            let mut cy = root_y;
            let mut cur_lw = lw.max(3);
            let mut cur_rw = rw.max(3);
            let mut cx = root_x;

            for b in 0..boxes {
                let bc = if b == 0 {
                    color
                } else {
                    lighten(bark, (b as u8 * 8).min(25))
                };
                let fc = if b == 0 {
                    bark
                } else {
                    lighten(dim, (b as u8 * 5).min(20))
                };
                // Interior height: biggest box at bottom, smaller going up
                let interior = if compact {
                    1
                } else if b == 0 {
                    ((energy * 3.0).ceil() as i32).clamp(1, 3)
                } else {
                    rng.random_range(1..3u32) as i32
                };

                // Bottom edge
                set(grid, cx - cur_lw, cy, '╘', bc);
                set(grid, cx + cur_rw, cy, '╛', bc);
                for dx in (-cur_lw + 1)..cur_rw {
                    set(grid, cx + dx, cy, '═', bc);
                }
                set(grid, cx, cy, '╧', bc);

                // Interior rows
                for row in 0..interior {
                    cy -= 1;
                    set(grid, cx - cur_lw, cy, '│', bc);
                    set(grid, cx + cur_rw, cy, '│', bc);
                    let fills = if row == 0 {
                        ['░', '▒', '░', '·']
                    } else {
                        ['▒', '▓', '▒', '░']
                    };
                    for dx in (-cur_lw + 1)..cur_rw {
                        set(
                            grid,
                            cx + dx,
                            cy,
                            fills[rng.random_range(0..4u32) as usize],
                            fc,
                        );
                    }
                    set(grid, cx, cy, '│', bc);
                }

                // Top edge
                cy -= 1;
                set(grid, cx - cur_lw, cy, '╒', bc);
                set(grid, cx + cur_rw, cy, '╓', bc);
                for dx in (-cur_lw + 1)..cur_rw {
                    set(grid, cx + dx, cy, '═', bc);
                }
                set(grid, cx, cy, '╤', bc);

                // Next box: narrower and randomly offset
                let next_lw = ((cur_lw as f32) * (0.5 + rng.random::<f32>() * 0.3)).max(1.0) as i32;
                let next_rw = ((cur_rw as f32) * (0.5 + rng.random::<f32>() * 0.3)).max(1.0) as i32;
                let drift = rng.random_range(0..3u32) as i32 - 1;
                cx += drift;
                cur_lw = next_lw;
                cur_rw = next_rw;

                // Connector between boxes: vertical line back to root_x
                if b < boxes - 1 {
                    cy -= 1;
                    if cx != root_x {
                        // Draw connector from root_x to cx
                        let dir = if cx > root_x { 1 } else { -1 };
                        set(grid, root_x, cy, if dir > 0 { '╰' } else { '╯' }, bc);
                        for sx in 1..(cx - root_x).abs() {
                            set(grid, root_x + sx * dir, cy, '─', bc);
                        }
                        set(grid, cx, cy, if dir > 0 { '╮' } else { '╭' }, bc);
                        cy -= 1;
                    }
                }
            }

            // Final trunk connector at root_x
            if cx != root_x {
                let dir = if root_x > cx { 1 } else { -1 };
                set(grid, cx, cy, if dir > 0 { '╰' } else { '╯' }, bark);
                for sx in 1..(root_x - cx).abs() {
                    set(grid, cx + sx * dir, cy, '─', bark);
                }
                set(grid, root_x, cy, if dir > 0 { '╮' } else { '╭' }, bark);
                cy -= 1;
            }
            BoleExit {
                x: root_x,
                y: cy + 1,
                left: cur_lw,
                right: cur_rw,
            }
        }
        // Style 9: Diamond2 -- asymmetric diamond with cross-hatching
        9 => {
            let energy = energy.clamp(0.2, 1.0);
            let total_h = if compact {
                3
            } else {
                ((energy * 5.0).ceil() as i32 + 2).clamp(3, 7)
            };
            let mut cy = root_y;
            // Asymmetric: bottom half taller than top
            let bot_h = if compact { 1 } else { (total_h * 2 / 3).max(2) };
            let top_h = if compact { 1 } else { (total_h - bot_h).max(1) };
            let max_lw = lw.max(2);
            let max_rw = rw.max(2);

            // Bottom: expanding upward, left and right sides grow at different rates
            for row in 0..bot_h {
                let frac = (row + 1) as f32 / bot_h as f32;
                let hw_l = (frac * max_lw as f32).ceil() as i32;
                let hw_r = (frac * max_rw as f32).ceil() as i32;
                let rc = lighten(bark, ((bot_h - row) as u8 * 4).min(25));
                set(grid, root_x, cy, if row == 0 { '╨' } else { '│' }, color);
                for dx in 1..=hw_l {
                    let ch = if rng.random_range(0..4u32) == 0 {
                        '╳'
                    } else if dx % 2 == 0 {
                        '─'
                    } else {
                        '◆'
                    };
                    set(grid, root_x - dx, cy, ch, rc);
                }
                for dx in 1..=hw_r {
                    let ch = if rng.random_range(0..4u32) == 0 {
                        '╳'
                    } else if dx % 2 == 0 {
                        '─'
                    } else {
                        '◆'
                    };
                    set(grid, root_x + dx, cy, ch, rc);
                }
                cy -= 1;
            }

            // Widest row: asymmetric
            set(grid, root_x, cy, '◆', color);
            for dx in 1..=max_lw {
                let ch = if dx % 3 == 0 { '╳' } else { '═' };
                set(
                    grid,
                    root_x - dx,
                    cy,
                    ch,
                    lighten(color, ((dx as u8) * 3).min(20)),
                );
            }
            for dx in 1..=max_rw {
                let ch = if dx % 3 == 0 { '╳' } else { '═' };
                set(
                    grid,
                    root_x + dx,
                    cy,
                    ch,
                    lighten(color, ((dx as u8) * 3).min(20)),
                );
            }
            set(grid, root_x - max_lw - 1, cy, '◁', dim);
            set(grid, root_x + max_rw + 1, cy, '▷', dim);
            cy -= 1;

            // Top: contracting, shorter than bottom
            for row in 0..top_h {
                let frac = (top_h - row) as f32 / top_h as f32;
                let hw_l = (frac * max_lw as f32).ceil() as i32;
                let hw_r = (frac * max_rw as f32).ceil() as i32;
                let rc = lighten(bark, ((row + 1) as u8 * 7).min(35));
                set(grid, root_x, cy, '│', rc);
                for dx in 1..=hw_l {
                    let ch = if rng.random_range(0..5u32) == 0 {
                        '╳'
                    } else if dx % 2 == 0 {
                        '─'
                    } else {
                        '◇'
                    };
                    set(grid, root_x - dx, cy, ch, rc);
                }
                for dx in 1..=hw_r {
                    let ch = if rng.random_range(0..5u32) == 0 {
                        '╳'
                    } else if dx % 2 == 0 {
                        '─'
                    } else {
                        '◇'
                    };
                    set(grid, root_x + dx, cy, ch, rc);
                }
                cy -= 1;
            }

            let exit_hw_l = (1.0f32 / top_h as f32 * max_lw as f32).ceil() as i32;
            let exit_hw_r = (1.0f32 / top_h as f32 * max_rw as f32).ceil() as i32;
            BoleExit {
                x: root_x,
                y: cy + 1,
                left: exit_hw_l,
                right: exit_hw_r,
            }
        }
        // Style 10: Chevron2 -- chevron with horizontal sprawl near base
        10 => {
            let energy = energy.clamp(0.2, 1.0);
            let layers = if compact {
                1
            } else {
                ((energy * 3.5).ceil() as i32).clamp(1, 4)
            };
            let mut cy = root_y;
            let ll = lw.max(2);
            let rl = rw.max(2);

            // Ground sprawl: horizontal bars at base
            set(grid, root_x, cy, '┴', color);
            for dx in 1..=ll {
                set(
                    grid,
                    root_x - dx,
                    cy,
                    '═',
                    lighten(bark, ((dx as u8) * 3).min(20)),
                );
            }
            for dx in 1..=rl {
                set(
                    grid,
                    root_x + dx,
                    cy,
                    '═',
                    lighten(bark, ((dx as u8) * 3).min(20)),
                );
            }
            // Extended sprawl wings
            let sprawl_l = ll + rng.random_range(1..4u32) as i32;
            let sprawl_r = rl + rng.random_range(1..4u32) as i32;
            for dx in (ll + 1)..=sprawl_l {
                set(grid, root_x - dx, cy, '─', dim);
            }
            for dx in (rl + 1)..=sprawl_r {
                set(grid, root_x + dx, cy, '─', dim);
            }
            set(grid, root_x - sprawl_l - 1, cy, '╴', lighten(dim, 10));
            set(grid, root_x + sprawl_r + 1, cy, '╶', lighten(dim, 10));
            cy -= 1;

            // Base V with extra width
            set(grid, root_x, cy, '∨', color);
            for dx in 1..=ll {
                set(
                    grid,
                    root_x - dx,
                    cy,
                    '╲',
                    lighten(bark, ((dx as u8) * 4).min(25)),
                );
            }
            for dx in 1..=rl {
                set(
                    grid,
                    root_x + dx,
                    cy,
                    '╱',
                    lighten(bark, ((dx as u8) * 4).min(25)),
                );
            }
            // Horizontal extensions at V tips
            set(grid, root_x - ll - 1, cy, '─', dim);
            set(grid, root_x + rl + 1, cy, '─', dim);
            cy -= 1;

            // Chevron layers, each narrower, less horizontal sprawl
            for layer in 0..layers {
                let shrink = (layer + 1) as f32 * 0.2;
                let cl = ((ll as f32) * (1.0 - shrink)).max(1.0) as i32;
                let cr = ((rl as f32) * (1.0 - shrink)).max(1.0) as i32;
                let lc = if layer == 0 {
                    bark
                } else {
                    lighten(bark, (layer as u8 * 7).min(30))
                };

                // ∧ row
                let center_ch = ['∧', '△', '▵', '⟋'][rng.random_range(0..4u32) as usize];
                set(grid, root_x, cy, center_ch, color);
                for dx in 1..=cl {
                    set(grid, root_x - dx, cy, '╱', lc);
                }
                for dx in 1..=cr {
                    set(grid, root_x + dx, cy, '╲', lc);
                }
                // Sprawl decreases with height
                let h_sprawl = ((layers - layer) as f32 * 0.5).ceil() as i32;
                if h_sprawl > 0 {
                    for s in 1..=h_sprawl {
                        set(grid, root_x - cl - s, cy, '─', lighten(lc, 15));
                        set(grid, root_x + cr + s, cy, '─', lighten(lc, 15));
                    }
                }
                cy -= 1;

                if layer < layers - 1 {
                    // ∨ row between layers
                    let vcl = ((cl as f32) * 0.7).max(1.0) as i32;
                    let vcr = ((cr as f32) * 0.7).max(1.0) as i32;
                    let vc = ['∨', '▽', '▿'][rng.random_range(0..3u32) as usize];
                    set(grid, root_x, cy, vc, lc);
                    for dx in 1..=vcl {
                        set(grid, root_x - dx, cy, '╲', lighten(lc, 10));
                    }
                    for dx in 1..=vcr {
                        set(grid, root_x + dx, cy, '╱', lighten(lc, 10));
                    }
                    cy -= 1;
                }
            }

            BoleExit::point(root_x, cy + 1)
        }
        // Style 11: Frame4 -- same as Frame3 but different corner style
        11 => {
            let energy = energy.clamp(0.2, 1.0);
            let boxes = if compact {
                1
            } else {
                ((energy * 3.0).ceil() as i32).clamp(1, 3)
            };
            let mut cy = root_y;
            let mut cur_lw = lw.max(3);
            let mut cur_rw = rw.max(3);
            let mut cx = root_x;

            for b in 0..boxes {
                let bc = if b == 0 {
                    color
                } else {
                    lighten(bark, (b as u8 * 10).min(30))
                };
                let fc = if b == 0 { bark } else { dim };
                let interior = if compact {
                    1
                } else if b == 0 {
                    ((energy * 2.0).ceil() as i32).clamp(1, 3)
                } else {
                    1
                };

                // Bottom
                set(grid, cx - cur_lw, cy, '└', bc);
                set(grid, cx + cur_rw, cy, '┘', bc);
                for dx in (-cur_lw + 1)..cur_rw {
                    set(grid, cx + dx, cy, '─', bc);
                }
                set(grid, cx, cy, '┴', bc);

                for row in 0..interior {
                    cy -= 1;
                    set(grid, cx - cur_lw, cy, '│', bc);
                    set(grid, cx + cur_rw, cy, '│', bc);
                    let fills = ['·', '∙', '·', ' '];
                    for dx in (-cur_lw + 1)..cur_rw {
                        set(
                            grid,
                            cx + dx,
                            cy,
                            fills[rng.random_range(0..4u32) as usize],
                            fc,
                        );
                    }
                    set(grid, cx, cy, '│', bc);
                }

                cy -= 1;
                set(grid, cx - cur_lw, cy, '┌', bc);
                set(grid, cx + cur_rw, cy, '┐', bc);
                for dx in (-cur_lw + 1)..cur_rw {
                    set(grid, cx + dx, cy, '─', bc);
                }
                set(grid, cx, cy, '┬', bc);

                let next_lw = ((cur_lw as f32) * (0.5 + rng.random::<f32>() * 0.3)).max(1.0) as i32;
                let next_rw = ((cur_rw as f32) * (0.5 + rng.random::<f32>() * 0.3)).max(1.0) as i32;
                let drift = rng.random_range(0..3u32) as i32 - 1;
                cx += drift;
                cur_lw = next_lw;
                cur_rw = next_rw;

                if b < boxes - 1 {
                    cy -= 1;
                    if cx != root_x {
                        let dir = if cx > root_x { 1 } else { -1 };
                        set(grid, root_x, cy, if dir > 0 { '╰' } else { '╯' }, bc);
                        for sx in 1..(cx - root_x).abs() {
                            set(grid, root_x + sx * dir, cy, '─', bc);
                        }
                        set(grid, cx, cy, if dir > 0 { '╮' } else { '╭' }, bc);
                        cy -= 1;
                    }
                }
            }

            if cx != root_x {
                let dir = if root_x > cx { 1 } else { -1 };
                set(grid, cx, cy, if dir > 0 { '╰' } else { '╯' }, bark);
                for sx in 1..(root_x - cx).abs() {
                    set(grid, cx + sx * dir, cy, '─', bark);
                }
                set(grid, root_x, cy, if dir > 0 { '╮' } else { '╭' }, bark);
                cy -= 1;
            }
            BoleExit {
                x: root_x,
                y: cy + 1,
                left: cur_lw,
                right: cur_rw,
            }
        }
        // Style 15: Keel -- short fat asymmetric hull, 2-4 rows max
        15 => {
            let energy = energy.clamp(0.2, 1.0);
            let total_h = if compact {
                2
            } else {
                ((energy * 2.0).ceil() as i32 + 1).clamp(2, 4)
            };
            let mut cy = root_y;
            let bias = if rng.random::<bool>() { 1.4f32 } else { 0.6f32 };
            let max_lw = ((lw as f32) * bias * 1.3).max(3.0) as i32;
            let max_rw = ((rw as f32) * (2.0 - bias) * 1.3).max(3.0) as i32;

            for row in 0..total_h {
                let frac = 1.0 - (row as f32 / total_h as f32);
                let hl = (max_lw as f32 * frac).ceil() as i32;
                let hr = (max_rw as f32 * frac).ceil() as i32;
                let rc = if row == 0 {
                    color
                } else {
                    lighten(bark, ((row as u8) * 6).min(25))
                };

                if row == 0 {
                    set(grid, root_x, cy, '╨', color);
                    set(grid, root_x - 1, cy, '═', color);
                    set(grid, root_x + 1, cy, '═', color);
                    for dx in 2..=hl {
                        let ch = if dx % 3 == 0 { '◆' } else { '═' };
                        set(grid, root_x - dx, cy, ch, rc);
                    }
                    for dx in 2..=hr {
                        let ch = if dx % 3 == 0 { '◇' } else { '═' };
                        set(grid, root_x + dx, cy, ch, rc);
                    }
                    set(grid, root_x - hl - 1, cy, '╘', dim);
                    set(grid, root_x + hr + 1, cy, '╛', dim);
                } else {
                    set(grid, root_x, cy, '│', rc);
                    for dx in 1..=hl {
                        let ch = ['─', '─', '◇', '─', '═'][rng.random_range(0..5u32) as usize];
                        set(grid, root_x - dx, cy, ch, rc);
                    }
                    for dx in 1..=hr {
                        let ch = ['─', '─', '◇', '─', '═'][rng.random_range(0..5u32) as usize];
                        set(grid, root_x + dx, cy, ch, rc);
                    }
                    if hl > 0 {
                        set(grid, root_x - hl, cy, '╲', rc);
                    }
                    if hr > 0 {
                        set(grid, root_x + hr, cy, '╱', rc);
                    }
                }
                cy -= 1;
            }

            let exit_frac = 1.0 - ((total_h - 1) as f32 / total_h as f32);
            let exit_l = (max_lw as f32 * exit_frac).ceil() as i32;
            let exit_r = (max_rw as f32 * exit_frac).ceil() as i32;
            BoleExit {
                x: root_x,
                y: cy + 1,
                left: exit_l,
                right: exit_r,
            }
        }
        // Style 17: Buttress -- bright bold curved grounding legs
        17 => {
            let energy = energy.clamp(0.2, 1.0);
            let mut cy = root_y;

            let left_reach = lw.max(2) + rng.random_range(1..4u32) as i32;
            let right_reach = rw.max(2) + rng.random_range(0..3u32) as i32;

            // Ground anchor: bold and bright
            set(grid, root_x, cy, '╨', color);
            set(grid, root_x - 1, cy, '═', color);
            set(grid, root_x + 1, cy, '═', color);
            if lw > 2 {
                set(grid, root_x - 2, cy, '═', bark);
            }
            if rw > 2 {
                set(grid, root_x + 2, cy, '═', bark);
            }

            // Left leg: curved, BRIGHT
            let mut lx = root_x - 2;
            let mut ly = cy;
            set(grid, lx, ly, '╮', color);
            for step in 0..left_reach {
                if step < left_reach / 3 {
                    lx -= 1;
                    set(grid, lx, ly, '─', color);
                } else if step == left_reach / 3 {
                    lx -= 1;
                    set(grid, lx, ly, '╮', lighten(color, 10));
                    ly += 1;
                    if ly <= root_y + 2 {
                        set(grid, lx, ly, '│', lighten(color, 10));
                    }
                } else {
                    lx -= 1;
                    ly += 1;
                    if ly <= root_y + 2 {
                        set(grid, lx, ly, '╲', lighten(color, 5));
                    }
                }
            }
            if ly <= root_y + 2 {
                set(grid, lx, ly, '╰', lighten(color, 10));
            }

            // Right leg: different curve, BRIGHT
            let mut rx = root_x + 2;
            let mut ry = cy;
            set(grid, rx, ry, '╭', color);
            for step in 0..right_reach {
                if step < right_reach / 2 {
                    rx += 1;
                    set(grid, rx, ry, '─', color);
                } else if step == right_reach / 2 {
                    rx += 1;
                    set(grid, rx, ry, '╭', lighten(color, 10));
                    ry += 1;
                    if ry <= root_y + 2 {
                        set(grid, rx, ry, '│', lighten(color, 10));
                    }
                } else {
                    rx += 1;
                    ry += 1;
                    if ry <= root_y + 2 {
                        set(grid, rx, ry, '╱', lighten(color, 5));
                    }
                }
            }
            if ry <= root_y + 2 {
                set(grid, rx, ry, '╯', lighten(color, 10));
            }

            // Cross-brace at high energy: BRIGHT
            if !compact && energy > 0.5 {
                let brace_y = cy - 1;
                let bl = (left_reach / 3).max(1);
                let br = (right_reach / 3).max(1);
                for dx in 1..=bl {
                    set(grid, root_x - dx, brace_y, '─', color);
                }
                set(grid, root_x - bl - 1, brace_y, '╴', bark);
                for dx in 1..=br {
                    set(grid, root_x + dx, brace_y, '─', color);
                }
                set(grid, root_x + br + 1, brace_y, '╶', bark);
                set(grid, root_x, brace_y, '┼', color);
                cy = brace_y;
            }

            // Upper secondary hints
            if !compact && energy > 0.4 {
                cy -= 1;
                let sl = (left_reach / 3).max(1);
                let sr = (right_reach / 3).max(1);
                for dx in 1..=sl {
                    set(grid, root_x - dx, cy, '╱', lighten(color, 15));
                }
                for dx in 1..=sr {
                    set(grid, root_x + dx, cy, '╲', lighten(color, 15));
                }
                set(grid, root_x, cy, '│', color);
            }

            cy -= 1;
            let sl = (left_reach / 3).max(1);
            let sr = (right_reach / 3).max(1);
            BoleExit {
                x: root_x,
                y: cy + 1,
                left: sl,
                right: sr,
            }
        }
        12 => BoleExit::point(root_x, root_y),
        // Style 13: Braille -- horizontal shelf bole, wide ground then sharp drop
        13 => {
            let energy = energy.clamp(0.2, 1.0);
            let dense = ['⣿', '⣾', '⣷', '⣶', '⣤'];
            let mid = ['⡇', '⢸', '⠿', '⠶', '⠛'];
            let edge = ['⡀', '⢀', '⠂', '⠁', '⠈'];

            // Shelf structure: 1-2 wide ground rows, then sharp narrow
            let shelf_rows = if compact {
                1
            } else if energy > 0.5 {
                2
            } else {
                1
            };
            let upper_rows = if compact {
                1
            } else {
                ((energy * 2.0).ceil() as i32).clamp(0, 3)
            };
            let base_l = lw.max(3) + rng.random_range(0..3u32) as i32;
            let base_r = rw.max(3) + rng.random_range(0..2u32) as i32;
            let mut cy = root_y;

            // SHELF: wide dense ground rows (the horizontal emphasis)
            for row in 0..shelf_rows {
                let sl = base_l - row;
                let sr = base_r - row;
                let rc = if row == 0 { color } else { darken(color, 4) };
                set(grid, root_x, cy, '⣿', rc);
                for dx in 1..=sl {
                    let ch = dense[rng.random_range(0..dense.len() as u32) as usize];
                    set(
                        grid,
                        root_x - dx,
                        cy,
                        ch,
                        darken(rc, ((dx as u8) * 2).min(8)),
                    );
                }
                for dx in 1..=sr {
                    let ch = dense[rng.random_range(0..dense.len() as u32) as usize];
                    set(
                        grid,
                        root_x + dx,
                        cy,
                        ch,
                        darken(rc, ((dx as u8) * 2).min(8)),
                    );
                }
                set(
                    grid,
                    root_x - sl - 1,
                    cy,
                    edge[rng.random_range(0..edge.len() as u32) as usize],
                    dim,
                );
                set(
                    grid,
                    root_x + sr + 1,
                    cy,
                    edge[rng.random_range(0..edge.len() as u32) as usize],
                    dim,
                );
                cy -= 1;
            }

            // SHARP DROP: immediately much narrower (40-60% of shelf width)
            let drop_frac = 0.4 + rng.random::<f32>() * 0.2;
            let mut cur_l = (base_l as f32 * drop_frac).max(1.0) as i32;
            let mut cur_r = (base_r as f32 * drop_frac).max(1.0) as i32;

            for row in 0..upper_rows {
                let rc = darken(color, ((shelf_rows + row) as u8 * 3).min(12));
                let chars = if row == 0 { &mid } else { &mid };
                set(
                    grid,
                    root_x,
                    cy,
                    dense[rng.random_range(0..2u32) as usize],
                    rc,
                );
                for dx in 1..=cur_l {
                    set(
                        grid,
                        root_x - dx,
                        cy,
                        chars[rng.random_range(0..chars.len() as u32) as usize],
                        darken(rc, ((dx as u8) * 2).min(8)),
                    );
                }
                for dx in 1..=cur_r {
                    set(
                        grid,
                        root_x + dx,
                        cy,
                        chars[rng.random_range(0..chars.len() as u32) as usize],
                        darken(rc, ((dx as u8) * 2).min(8)),
                    );
                }
                set(
                    grid,
                    root_x - cur_l - 1,
                    cy,
                    edge[rng.random_range(0..edge.len() as u32) as usize],
                    dim,
                );
                set(
                    grid,
                    root_x + cur_r + 1,
                    cy,
                    edge[rng.random_range(0..edge.len() as u32) as usize],
                    dim,
                );
                // Taper each upper row slightly
                cur_l = (cur_l - rng.random_range(0..2u32) as i32).max(1);
                cur_r = (cur_r - rng.random_range(0..2u32) as i32).max(1);
                cy -= 1;
            }

            BoleExit {
                x: root_x,
                y: cy,
                left: cur_l,
                right: cur_r,
            }
        }
        // Style 14: Frame -- overlapping rects with foreground cross glyphs
        14 => {
            let energy = energy.clamp(0.2, 1.0);
            let rects = if compact {
                1
            } else {
                ((energy * 2.0).ceil() as i32).clamp(1, 3)
            };
            let mut cy = root_y;

            let mut specs: Vec<(i32, i32, i32, i32)> = Vec::new();
            for r in 0..rects {
                let drift = if r == 0 {
                    0
                } else {
                    rng.random_range(0..3u32) as i32 - 1
                };
                let rlw = if r == 0 {
                    lw.max(3)
                } else {
                    (lw as f32 * (0.4 + rng.random::<f32>() * 0.4)).max(2.0) as i32
                };
                let rrw = if r == 0 {
                    rw.max(3)
                } else {
                    (rw as f32 * (0.4 + rng.random::<f32>() * 0.4)).max(2.0) as i32
                };
                let ih = if compact {
                    1
                } else if r == 0 {
                    ((energy * 2.0).ceil() as i32).clamp(1, 2)
                } else {
                    1
                };
                specs.push((drift, rlw, rrw, ih));
            }

            let mut accumulated_drift = 0i32;
            let mut prev_top_y: Option<i32> = None;
            let mut last_lw = lw;
            let mut last_rw = rw;

            for (ri, &(drift, rlw, rrw, ih)) in specs.iter().enumerate() {
                last_lw = rlw;
                last_rw = rrw;
                accumulated_drift += drift;
                let cx = root_x + accumulated_drift;
                let heavy = ri == 0;
                let bc = if heavy {
                    color
                } else {
                    lighten(bark, (ri as u8 * 10).min(25))
                };
                let fc = if heavy {
                    bark
                } else {
                    lighten(dim, (ri as u8 * 6).min(20))
                };

                let on_shared_edge = prev_top_y.map_or(false, |py| py == cy);

                if on_shared_edge {
                    set(grid, cx - rlw, cy, '╬', bc);
                    set(grid, cx + rrw, cy, '╬', bc);
                    for dx in (-rlw + 1)..rrw {
                        set(grid, cx + dx, cy, '╪', bc);
                    }
                    set(grid, root_x, cy, '╬', color);
                } else if heavy {
                    set(grid, cx - rlw, cy, '╚', bc);
                    set(grid, cx + rrw, cy, '╝', bc);
                    for dx in (-rlw + 1)..rrw {
                        set(grid, cx + dx, cy, '═', bc);
                    }
                    set(grid, root_x, cy, '╩', color);
                } else {
                    set(grid, cx - rlw, cy, '└', bc);
                    set(grid, cx + rrw, cy, '┘', bc);
                    for dx in (-rlw + 1)..rrw {
                        set(grid, cx + dx, cy, '─', bc);
                    }
                    set(grid, root_x, cy, '┴', bc);
                }

                for row in 0..ih {
                    cy -= 1;
                    set(grid, cx - rlw, cy, if heavy { '║' } else { '│' }, bc);
                    set(grid, cx + rrw, cy, if heavy { '║' } else { '│' }, bc);
                    let fills = if row == 0 {
                        ['░', '▒', '·', '░']
                    } else {
                        ['▒', '▓', '░', '·']
                    };
                    for dx in (-rlw + 1)..rrw {
                        set(
                            grid,
                            cx + dx,
                            cy,
                            fills[rng.random_range(0..4u32) as usize],
                            fc,
                        );
                    }
                    set(grid, root_x, cy, if heavy { '║' } else { '│' }, bc);
                }

                cy -= 1;
                if heavy {
                    set(grid, cx - rlw, cy, '╔', bc);
                    set(grid, cx + rrw, cy, '╗', bc);
                    for dx in (-rlw + 1)..rrw {
                        set(grid, cx + dx, cy, '═', bc);
                    }
                    set(grid, root_x, cy, '╦', color);
                } else {
                    set(grid, cx - rlw, cy, '┌', bc);
                    set(grid, cx + rrw, cy, '┐', bc);
                    for dx in (-rlw + 1)..rrw {
                        set(grid, cx + dx, cy, '─', bc);
                    }
                    set(grid, root_x, cy, '┬', bc);
                }

                prev_top_y = Some(cy);
            }

            BoleExit {
                x: root_x,
                y: cy,
                left: last_lw,
                right: last_rw,
            }
        }
        // Style 16: Chevron -- off-center layers that overlap into diamond patterns
        16 => {
            let energy = energy.clamp(0.2, 1.0);
            let layers = if compact {
                1
            } else {
                ((energy * 4.0).ceil() as i32).clamp(2, 5)
            };
            let mut cy = root_y;
            let ll = lw.max(2);
            let rl = rw.max(2);

            // Ground row is also the first ∧ -- no gap between base and chevrons
            set(grid, root_x, cy, '∧', color);
            for dx in 1..=ll {
                set(
                    grid,
                    root_x - dx,
                    cy,
                    '╱',
                    lighten(bark, ((dx as u8) * 2).min(20)),
                );
            }
            for dx in 1..=rl {
                set(
                    grid,
                    root_x + dx,
                    cy,
                    '╲',
                    lighten(bark, ((dx as u8) * 2).min(20)),
                );
            }
            // Sprawl wings extending from chevron tips
            let sprawl_l = rng.random_range(1..4u32) as i32;
            let sprawl_r = rng.random_range(1..3u32) as i32;
            for s in 1..=sprawl_l {
                set(grid, root_x - ll - s, cy, '─', lighten(bark, 12));
            }
            for s in 1..=sprawl_r {
                set(grid, root_x + rl + s, cy, '─', lighten(bark, 12));
            }
            set(grid, root_x - ll - sprawl_l - 1, cy, '╴', dim);
            set(grid, root_x + rl + sprawl_r + 1, cy, '╶', dim);
            cy -= 1;

            // Remaining chevron layers with random drift -- overlaps create diamonds
            for layer in 0..layers {
                let shrink = (layer + 1) as f32 * 0.15;
                let cl = ((ll as f32) * (1.0 - shrink)).max(1.0) as i32;
                let cr = ((rl as f32) * (1.0 - shrink)).max(1.0) as i32;
                let lc = if layer == 0 {
                    bark
                } else {
                    lighten(bark, (layer as u8 * 6).min(30))
                };
                // Random drift: each layer can be off-center
                let drift = rng.random_range(0..3u32) as i32 - 1;
                let lcx = root_x + drift;

                // ∧ row (upward V)
                set(grid, lcx, cy, '∧', color);
                for dx in 1..=cl {
                    set(grid, lcx - dx, cy, '╱', lc);
                }
                for dx in 1..=cr {
                    set(grid, lcx + dx, cy, '╲', lc);
                }
                // Horizontal stubs, sprawl decreases with height
                let h_sprawl = ((layers - layer) as f32 * 0.7).ceil() as i32;
                for s in 1..=h_sprawl {
                    set(grid, lcx - cl - s, cy, '─', lighten(lc, 12));
                    set(grid, lcx + cr + s, cy, '─', lighten(lc, 12));
                }
                cy -= 1;

                // ∨ row (downward V) -- slightly different drift for overlap
                if layer < layers - 1 {
                    let drift2 = rng.random_range(0..3u32) as i32 - 1;
                    let vcx = root_x + drift2;
                    let vcl = ((cl as f32) * 0.75).max(1.0) as i32;
                    let vcr = ((cr as f32) * 0.75).max(1.0) as i32;
                    set(grid, vcx, cy, '∨', lighten(lc, 5));
                    for dx in 1..=vcl {
                        set(grid, vcx - dx, cy, '╲', lighten(lc, 8));
                    }
                    for dx in 1..=vcr {
                        set(grid, vcx + dx, cy, '╱', lighten(lc, 8));
                    }
                    cy -= 1;
                }
            }

            // Chevron's ∧ shape already tapers to a point -- no generic taper needed
            BoleExit::point(root_x, cy + 1)
        }
        // ── Squat boles: max 2 rows, horizontal emphasis, single-column flares ──

        // Squat Crescent: wide single-row arc with flare pokes
        18 => {
            set(grid, root_x, root_y, '┴', color);
            for dx in 1..=lw {
                set(
                    grid,
                    root_x - dx,
                    root_y,
                    '═',
                    lighten(bark, ((dx as u8) * 3).min(20)),
                );
            }
            for dx in 1..=rw {
                set(
                    grid,
                    root_x + dx,
                    root_y,
                    '═',
                    lighten(bark, ((dx as u8) * 3).min(20)),
                );
            }
            set(grid, root_x - lw - 1, root_y, '◜', dim);
            set(grid, root_x + rw + 1, root_y, '◝', dim);
            // Flares: single-column pokes above at random positions
            for _ in 0..rng.random_range(1..4u32) {
                let fx = root_x + rng.random_range(0..(lw + rw + 1) as u32) as i32 - lw;
                if fx != root_x {
                    set(grid, fx, root_y - 1, '╷', lighten(bark, 15));
                }
            }
            BoleExit::point(root_x, root_y)
        }
        // Squat Braille: 2-row dense shelf, no vertical growth
        19 => {
            let dense = ['⣿', '⣾', '⣷', '⣶', '⣤'];
            let edge = ['⡀', '⢀', '⠂', '⠁', '⠈'];
            // Row 1: dense ground
            set(grid, root_x, root_y, '⣿', color);
            for dx in 1..=lw {
                set(
                    grid,
                    root_x - dx,
                    root_y,
                    dense[rng.random_range(0..dense.len() as u32) as usize],
                    darken(color, ((dx as u8) * 2).min(10)),
                );
            }
            for dx in 1..=rw {
                set(
                    grid,
                    root_x + dx,
                    root_y,
                    dense[rng.random_range(0..dense.len() as u32) as usize],
                    darken(color, ((dx as u8) * 2).min(10)),
                );
            }
            set(
                grid,
                root_x - lw - 1,
                root_y,
                edge[rng.random_range(0..edge.len() as u32) as usize],
                dim,
            );
            set(
                grid,
                root_x + rw + 1,
                root_y,
                edge[rng.random_range(0..edge.len() as u32) as usize],
                dim,
            );
            // Row 2: sparser, narrower
            let sl = (lw as f32 * 0.5).max(1.0) as i32;
            let sr = (rw as f32 * 0.5).max(1.0) as i32;
            let mid = ['⡇', '⢸', '⠿', '⠶', '⠛'];
            set(
                grid,
                root_x,
                root_y - 1,
                mid[rng.random_range(0..mid.len() as u32) as usize],
                bark,
            );
            for dx in 1..=sl {
                set(
                    grid,
                    root_x - dx,
                    root_y - 1,
                    mid[rng.random_range(0..mid.len() as u32) as usize],
                    darken(bark, ((dx as u8) * 3).min(12)),
                );
            }
            for dx in 1..=sr {
                set(
                    grid,
                    root_x + dx,
                    root_y - 1,
                    mid[rng.random_range(0..mid.len() as u32) as usize],
                    darken(bark, ((dx as u8) * 3).min(12)),
                );
            }
            BoleExit {
                x: root_x,
                y: root_y - 1,
                left: sl,
                right: sr,
            }
        }
        // Squat Frame: 2-row nested frame with diamond accents and pillar legs
        20 => {
            // Row 1 (ground): outer frame base with diamond endpoints
            set(grid, root_x - lw, root_y, '◇', dim);
            set(grid, root_x + rw, root_y, '◇', dim);
            for dx in (-lw + 1)..rw {
                let ch = if (root_x + dx) % 2 == 0 { '═' } else { '─' };
                set(grid, root_x + dx, root_y, ch, bark);
            }
            set(grid, root_x, root_y, '╧', color);
            // Inner accent: ◆ markers at 1/3 and 2/3 across
            let third_l = lw / 3;
            let third_r = rw / 3;
            if third_l > 0 {
                set(grid, root_x - third_l, root_y, '◆', lighten(bark, 10));
            }
            if third_r > 0 {
                set(grid, root_x + third_r, root_y, '◆', lighten(bark, 10));
            }

            // Row 2 (above): narrower inner shelf with box corners
            let iw_l = (lw * 2 / 3).max(1);
            let iw_r = (rw * 2 / 3).max(1);
            set(grid, root_x - iw_l, root_y - 1, '╰', lighten(bark, 8));
            set(grid, root_x + iw_r, root_y - 1, '╯', lighten(bark, 8));
            for dx in (-iw_l + 1)..iw_r {
                set(grid, root_x + dx, root_y - 1, '─', lighten(bark, 12));
            }
            set(grid, root_x, root_y - 1, '┼', color);
            // Pillar legs: drop below at diamond endpoints
            set(grid, root_x - lw, root_y + 1, '│', dim);
            set(grid, root_x + rw, root_y + 1, '│', dim);
            // Random inner flares above inner shelf
            if rng.random_range(0..2u32) == 0 {
                let fx = root_x + rng.random_range(1..iw_r.max(2) as u32) as i32;
                set(grid, fx, root_y - 2, '╷', lighten(bark, 20));
            }
            if rng.random_range(0..2u32) == 0 {
                let fx = root_x - rng.random_range(1..iw_l.max(2) as u32) as i32;
                set(grid, fx, root_y - 2, '╷', lighten(bark, 20));
            }
            BoleExit {
                x: root_x,
                y: root_y - 1,
                left: iw_l,
                right: iw_r,
            }
        }
        // Squat Diamond: 2-row flat diamond, single chevron + base
        21 => {
            // Ground: wide base
            set(grid, root_x, root_y, '╨', color);
            for dx in 1..=lw {
                let ch = if dx == lw { '◇' } else { '═' };
                set(
                    grid,
                    root_x - dx,
                    root_y,
                    ch,
                    lighten(bark, ((dx as u8) * 3).min(20)),
                );
            }
            for dx in 1..=rw {
                let ch = if dx == rw { '◇' } else { '═' };
                set(
                    grid,
                    root_x + dx,
                    root_y,
                    ch,
                    lighten(bark, ((dx as u8) * 3).min(20)),
                );
            }
            // Row 2: single V narrowing
            let hw = (lw.max(rw) / 2).max(1);
            set(grid, root_x, root_y - 1, '│', color);
            for dx in 1..=hw {
                set(grid, root_x - dx, root_y - 1, '╱', bark);
                set(grid, root_x + dx, root_y - 1, '╲', bark);
            }
            // Tip flares at base ends
            if lw > 2 {
                set(grid, root_x - lw, root_y + 1, '╵', dim);
            }
            if rw > 2 {
                set(grid, root_x + rw, root_y + 1, '╵', dim);
            }
            BoleExit {
                x: root_x,
                y: root_y - 1,
                left: hw,
                right: hw,
            }
        }
        // Squat Chevron: 2-row diamond chevron with inverted V counter-layer
        22 => {
            // Row 1 (ground): wide V with diamond at apex
            let center = ['∧', '△', '▵'][rng.random_range(0..3u32) as usize];
            set(grid, root_x, root_y, center, color);
            for dx in 1..=lw {
                let c = lighten(bark, ((dx as u8) * 4).min(25));
                set(grid, root_x - dx, root_y, '╱', c);
            }
            for dx in 1..=rw {
                let c = lighten(bark, ((dx as u8) * 4).min(25));
                set(grid, root_x + dx, root_y, '╲', c);
            }
            // Sprawl arms with stubs
            let sl = rng.random_range(1..4u32) as i32;
            let sr = rng.random_range(1..4u32) as i32;
            for s in 1..=sl {
                set(grid, root_x - lw - s, root_y, '─', lighten(bark, 12));
            }
            for s in 1..=sr {
                set(grid, root_x + rw + s, root_y, '─', lighten(bark, 12));
            }
            set(grid, root_x - lw - sl - 1, root_y, '◁', dim);
            set(grid, root_x + rw + sr + 1, root_y, '▷', dim);

            // Row 2 (above): inverted mini-V counter-layer (creates diamond negative space)
            let hw = (lw.max(rw) * 2 / 3).max(1);
            let inv = ['∨', '▽', '▿'][rng.random_range(0..3u32) as usize];
            set(grid, root_x, root_y - 1, inv, lighten(bark, 10));
            for dx in 1..=hw {
                set(grid, root_x - dx, root_y - 1, '╲', lighten(bark, 15));
                set(grid, root_x + dx, root_y - 1, '╱', lighten(bark, 15));
            }
            // Horizontal stubs at inverted tips
            if hw > 1 {
                set(grid, root_x - hw - 1, root_y - 1, '─', dim);
                set(grid, root_x + hw + 1, root_y - 1, '─', dim);
            }

            // Anchor drops below sprawl endpoints
            set(grid, root_x - lw - sl, root_y + 1, '╵', dim);
            set(grid, root_x + rw + sr, root_y + 1, '╵', dim);

            BoleExit {
                x: root_x,
                y: root_y - 1,
                left: hw,
                right: hw,
            }
        }
        // Squat Buttress: ground anchor with curved legs, max 2 rows
        23 => {
            set(grid, root_x, root_y, '╨', color);
            set(grid, root_x - 1, root_y, '═', color);
            set(grid, root_x + 1, root_y, '═', color);
            // Left leg: horizontal then down-kick
            let ll_reach = lw.max(2);
            set(grid, root_x - 2, root_y, '╮', bark);
            for dx in 3..=ll_reach {
                set(
                    grid,
                    root_x - dx,
                    root_y,
                    '─',
                    lighten(bark, ((dx as u8) * 3).min(20)),
                );
            }
            set(grid, root_x - ll_reach - 1, root_y, '╴', dim);
            // Left leg flare down
            set(grid, root_x - 2, root_y + 1, '│', dim);
            set(grid, root_x - 2, root_y + 2, '╵', lighten(dim, 10));
            // Right leg
            let rr_reach = rw.max(2);
            set(grid, root_x + 2, root_y, '╭', bark);
            for dx in 3..=rr_reach {
                set(
                    grid,
                    root_x + dx,
                    root_y,
                    '─',
                    lighten(bark, ((dx as u8) * 3).min(20)),
                );
            }
            set(grid, root_x + rr_reach + 1, root_y, '╶', dim);
            // Right leg flare down
            set(grid, root_x + 2, root_y + 1, '│', dim);
            set(grid, root_x + 2, root_y + 2, '╵', lighten(dim, 10));
            BoleExit::point(root_x, root_y)
        }
        // ── Winding boles: serpentine runs, woven strands, coiled arcs ──

        // Serpent: a root snakes across the ground in S-curves, switching
        // rows with curve corners; the trunk rises wherever it crosses center
        24 => {
            let y_b = root_y;
            let y_t = root_y - 1;
            let start = root_x - lw.max(3) - 1;
            let end = root_x + rw.max(3) + 1;
            let mut on_top = rng.random_range(0..2u32) == 0;
            let mut run = rng.random_range(2..5u32) as i32;
            let mut row_at_root = false; // true if snake is on top row at root_x
            set(grid, start, if on_top { y_t } else { y_b }, '╶', dim);
            let mut x = start + 1;
            while x <= end {
                let dist = ((x - root_x).abs() as u8 * 2).min(20);
                let c = if (x - root_x).abs() <= 1 {
                    color
                } else {
                    darken(bark, dist)
                };
                if run == 0 && x < end - 1 {
                    // switch rows: corner pair links the two runs
                    if on_top {
                        set(grid, x, y_t, '╮', c);
                        set(grid, x, y_b, '╰', c);
                    } else {
                        set(grid, x, y_b, '╯', c);
                        set(grid, x, y_t, '╭', c);
                    }
                    on_top = !on_top;
                    run = rng.random_range(2..5u32) as i32;
                } else {
                    set(grid, x, if on_top { y_t } else { y_b }, '─', c);
                    run -= 1;
                }
                if x == root_x {
                    row_at_root = on_top;
                }
                x += 1;
            }
            set(grid, end + 1, if on_top { y_t } else { y_b }, '╴', dim);
            // root tips digging below ground at 1-2 spots
            for _ in 0..rng.random_range(1..3u32) {
                let fx = root_x + rng.random_range(0..(lw + rw).max(2) as u32) as i32 - lw;
                set(grid, fx, root_y + 1, '╷', dim);
            }
            // trunk junction on whichever row the snake occupies at center
            if row_at_root {
                set(grid, root_x, y_t, '┴', color);
                BoleExit::point(root_x, y_t - 1)
            } else {
                set(grid, root_x, y_b, '┴', color);
                BoleExit::point(root_x, y_t)
            }
        }
        // Braid: two strands weave over-under in a period-4 diamond chain
        25 => {
            let w = lw.max(rw).max(3);
            let y_b = root_y;
            let y_t = root_y - 1;
            for k in -w..=w {
                let x = root_x + k;
                let dist = (k.abs() as u8 * 3).min(20);
                let (ct, cb) = match k.rem_euclid(4) {
                    0 => ('─', '─'),
                    1 => ('╲', '╱'),
                    2 => ('╱', '╲'),
                    _ => ('─', '─'),
                };
                // crossing columns stay bright, straight runs fade outward
                let crossing = k.rem_euclid(4) == 1 || k.rem_euclid(4) == 2;
                let c = if crossing {
                    lighten(bark, 10)
                } else {
                    darken(bark, dist)
                };
                set(grid, x, y_t, ct, c);
                set(grid, x, y_b, cb, darken(c, 8));
            }
            set(grid, root_x - w - 1, y_b, '╾', dim);
            set(grid, root_x + w + 1, y_b, '╼', dim);
            set(grid, root_x, y_t, '┼', color);
            BoleExit {
                x: root_x,
                y: y_t,
                left: 1,
                right: 1,
            }
        }
        // Coil: nested arcs stacked into a flattened spiral; the tail
        // sweeps out one side and the gap rotates ring to ring
        26 => {
            let w0 = lw.max(rw).max(4);
            let w1 = (w0 * 2 / 3).max(2);
            let tail_right = rng.random_range(0..2u32) == 0;
            let ts = if tail_right { 1 } else { -1 };
            // outer ring: upward cup on the ground row
            set(grid, root_x - w0 - 1, root_y, '◟', dim);
            for dx in -w0..=w0 {
                set(
                    grid,
                    root_x + dx,
                    root_y,
                    '◡',
                    darken(bark, (dx.abs() as u8 * 2).min(16)),
                );
            }
            set(grid, root_x + w0 + 1, root_y, '◞', dim);
            // tail sweeps out from the outer ring
            set(grid, root_x + ts * (w0 + 2), root_y, '─', dim);
            set(
                grid,
                root_x + ts * (w0 + 3),
                root_y,
                if tail_right { '╴' } else { '╶' },
                darken(dim, 10),
            );
            // inner ring: downward cap, shifted opposite the tail (spiral offset)
            let ox = -ts;
            set(grid, root_x + ox - w1 - 1, root_y - 1, '◜', bark);
            for dx in -w1..=w1 {
                set(
                    grid,
                    root_x + ox + dx,
                    root_y - 1,
                    '◠',
                    lighten(bark, (dx.abs() as u8 * 3).min(18)),
                );
            }
            set(grid, root_x + ox + w1 + 1, root_y - 1, '◝', bark);
            // coil eye where the spiral terminates
            set(grid, root_x + ox * 2, root_y - 1, '◉', color);
            set(grid, root_x, root_y - 1, '╂', color);
            BoleExit {
                x: root_x,
                y: root_y - 1,
                left: 1,
                right: 1,
            }
        }
        // Taproot: low mound above ground, winding roots dig below it
        27 => {
            set(grid, root_x, root_y, '┴', color);
            for dx in 1..=lw.max(2) {
                let ch = if dx == lw.max(2) { '╮' } else { '─' };
                set(
                    grid,
                    root_x - dx,
                    root_y,
                    ch,
                    darken(bark, (dx as u8 * 3).min(18)),
                );
            }
            for dx in 1..=rw.max(2) {
                let ch = if dx == rw.max(2) { '╭' } else { '─' };
                set(
                    grid,
                    root_x + dx,
                    root_y,
                    ch,
                    darken(bark, (dx as u8 * 3).min(18)),
                );
            }
            // winding roots: walkers descend 1-2 rows, drifting and reversing
            let n_roots = rng.random_range(3..5u32);
            for r in 0..n_roots {
                let span = (lw + rw).max(2);
                let mut x = root_x + rng.random_range(0..span as u32) as i32 - lw;
                if x == root_x {
                    x += if r % 2 == 0 { 1 } else { -1 };
                }
                let mut drift: i32 = if x < root_x { -1 } else { 1 };
                let depth = if compact {
                    1
                } else {
                    rng.random_range(1..3u32) as i32
                };
                let mut y = root_y;
                for _ in 0..depth {
                    y += 1;
                    if rng.random::<f32>() < 0.6 {
                        x += drift;
                        set(grid, x, y, if drift > 0 { '╲' } else { '╱' }, dim);
                    } else {
                        set(grid, x, y, '│', dim);
                    }
                    if rng.random::<f32>() < 0.3 {
                        drift = -drift;
                    }
                }
                set(grid, x, y + 1, '╷', darken(dim, 10));
            }
            BoleExit {
                x: root_x,
                y: root_y,
                left: 1,
                right: 1,
            }
        }
        // ── Structural boles: legs, piles, dens, claws, shelves, grass ──

        // Stilts: the trunk stands on splayed mangrove prop legs with
        // open air underneath
        28 => {
            set(grid, root_x, root_y - 1, '┴', color);
            set(grid, root_x - 1, root_y - 1, '╭', bark);
            set(grid, root_x + 1, root_y - 1, '╮', bark);
            let pairs = (lw.max(rw) / 2).clamp(1, 3);
            for k in 1..=pairs {
                // each pair splays one cell further out per row down
                let hip = k;
                set(
                    grid,
                    root_x - hip - 1,
                    root_y,
                    '╱',
                    darken(bark, (k as u8 * 6).min(18)),
                );
                set(
                    grid,
                    root_x + hip + 1,
                    root_y,
                    '╲',
                    darken(bark, (k as u8 * 6).min(18)),
                );
                set(grid, root_x - hip - 2, root_y + 1, '╱', dim);
                set(grid, root_x + hip + 2, root_y + 1, '╲', dim);
            }
            // feet grip the ground below the outermost legs
            set(grid, root_x - pairs - 3, root_y + 2, '╷', darken(dim, 10));
            set(grid, root_x + pairs + 3, root_y + 2, '╷', darken(dim, 10));
            // one off-center brace leg for asymmetry
            let bx = if rng.random_range(0..2u32) == 0 {
                -1
            } else {
                1
            };
            set(grid, root_x + bx, root_y, '│', bark);
            set(grid, root_x + bx, root_y + 1, '╷', dim);
            BoleExit {
                x: root_x,
                y: root_y - 1,
                left: 1,
                right: 1,
            }
        }
        // Cairn: rounded stones piled against the trunk base
        29 => {
            let stones = ['◯', '●', '○', '◦'];
            let hw = lw.max(rw).max(2);
            for dx in -hw..=hw {
                let ch = stones[rng.random_range(0..stones.len() as u32) as usize];
                set(
                    grid,
                    root_x + dx,
                    root_y,
                    ch,
                    darken(bark, (dx.abs() as u8 * 4).min(20)),
                );
            }
            // top course: fewer, smaller stones nested in the gaps
            let tw = (hw / 2).max(1);
            for dx in -tw..=tw {
                if dx == 0 {
                    continue;
                }
                let ch = if rng.random_range(0..2u32) == 0 {
                    '○'
                } else {
                    '◦'
                };
                set(grid, root_x + dx, root_y - 1, ch, bark);
            }
            set(grid, root_x, root_y - 1, '╨', color);
            BoleExit::point(root_x, root_y - 2)
        }
        // Hollow: a den opening framed in the trunk base, sloped
        // shoulders on either side
        30 => {
            let hw = lw.max(rw).max(3);
            // rim row
            set(grid, root_x, root_y - 1, '┴', color);
            for dx in 1..hw {
                set(grid, root_x - dx, root_y - 1, '─', bark);
                set(grid, root_x + dx, root_y - 1, '─', bark);
            }
            set(grid, root_x - hw, root_y - 1, '╭', bark);
            set(grid, root_x + hw, root_y - 1, '╮', bark);
            // ground row: arch entrance at center, shoulders slope away
            set(grid, root_x - 1, root_y, '╭', lighten(bark, 15));
            set(grid, root_x + 1, root_y, '╮', lighten(bark, 15));
            // the hole itself stays blank at root_x
            for dx in 2..hw {
                set(
                    grid,
                    root_x - dx,
                    root_y,
                    '▒',
                    darken(bark, (dx as u8 * 3).min(15)),
                );
                set(
                    grid,
                    root_x + dx,
                    root_y,
                    '▒',
                    darken(bark, (dx as u8 * 3).min(15)),
                );
            }
            set(grid, root_x - hw, root_y, '╱', dim);
            set(grid, root_x + hw, root_y, '╲', dim);
            BoleExit {
                x: root_x,
                y: root_y - 1,
                left: 2,
                right: 2,
            }
        }
        // Talon: clawed digits radiate from a center pad and dig in
        31 => {
            set(grid, root_x, root_y - 1, '┴', color);
            let reach_l = lw.max(2).min(4);
            let reach_r = rw.max(2).min(4);
            // outer digits: out along the pad, bend, then dig
            for (side, reach) in [(-1i32, reach_l), (1i32, reach_r)] {
                let digits = (reach / 2).max(1);
                // longest digit first so shorter bends overwrite its run
                for d in (1..=digits).rev() {
                    let bend = side * (d * 2);
                    for s in 1..(d * 2) {
                        set(grid, root_x + side * s, root_y - 1, '─', bark);
                    }
                    set(
                        grid,
                        root_x + bend,
                        root_y - 1,
                        if side < 0 { '╭' } else { '╮' },
                        bark,
                    );
                    set(grid, root_x + bend, root_y, '│', darken(bark, 8));
                    set(grid, root_x + bend, root_y + 1, '╷', dim);
                }
            }
            // center digit digs straight down
            set(grid, root_x, root_y, '│', bark);
            set(grid, root_x, root_y + 1, '╷', dim);
            BoleExit::point(root_x, root_y - 2)
        }
        // Tiers: stacked pagoda shelves shrinking upward, drip legs
        // under the outer edges
        32 => {
            let w0 = lw.max(rw).max(3);
            let levels = if energy > 0.6 { 3 } else { 2 };
            let mut cy = root_y;
            let mut w = w0;
            for lvl in 0..levels {
                let center = if lvl == levels - 1 { '┴' } else { '╪' };
                set(grid, root_x, cy, center, color);
                for dx in 1..=w {
                    set(
                        grid,
                        root_x - dx,
                        cy,
                        '═',
                        darken(bark, (dx as u8 * 3).min(18)),
                    );
                    set(
                        grid,
                        root_x + dx,
                        cy,
                        '═',
                        darken(bark, (dx as u8 * 3).min(18)),
                    );
                }
                if lvl == 0 {
                    // drip legs under the widest shelf
                    set(grid, root_x - w, cy + 1, '╷', dim);
                    set(grid, root_x + w, cy + 1, '╷', dim);
                }
                cy -= 1;
                w = (w * 3 / 5).max(1);
                if w < 1 {
                    break;
                }
            }
            BoleExit::point(root_x, root_y - levels)
        }
        // Tussock: a clump of grass blades hides the trunk base
        33 => {
            let blades = ['⌇', '╿', '╽', '┆', '╵'];
            let hw = lw.max(rw).max(2);
            for dx in -hw..=hw {
                let ch = blades[rng.random_range(0..blades.len() as u32) as usize];
                set(
                    grid,
                    root_x + dx,
                    root_y,
                    ch,
                    lighten(bark, rng.random_range(0..25u32) as u8),
                );
            }
            // taller blades near the center on a second row
            let tw = (hw / 2).max(1);
            for dx in -tw..=tw {
                if rng.random::<f32>() < 0.5 {
                    let ch = blades[rng.random_range(0..blades.len() as u32) as usize];
                    set(grid, root_x + dx, root_y - 1, ch, bark);
                }
            }
            // seed heads drift above the clump
            for _ in 0..rng.random_range(1..4u32) {
                let sx = root_x + rng.random_range(0..(hw * 2 + 1) as u32) as i32 - hw;
                set(grid, sx, root_y - 2, '·', dim);
            }
            set(grid, root_x, root_y - 1, '│', color);
            BoleExit::point(root_x, root_y - 1)
        }
        _ => BoleExit::point(root_x, root_y),
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

// ── Trait ────────────────────────────────────────────────────────────

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

// ── SpiralTree ──────────────────────────────────────────────────────
// Tall straight trunk. Alternating branches at regular intervals.
// Stub-capped arms with hooks on lower branches.

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

// ── CandelabraTree ──────────────────────────────────────────────────
// Short thick trunk (bottom 1/3). should_branch fires once at trunk top.
// draw_branch handles the entire crown: horizontal bar, vertical arms
// with corner-pair leans, two-way tip splits.

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

// ── SplitTree ───────────────────────────────────────────────────────
// Short wobble trunk (bottom 1/3). should_branch fires once at trunk top.
// draw_branch does recursive binary subdivision: each segment picks
// an off-center split point and forks left/right. Max depth 4.

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

// ── BirchTree ───────────────────────────────────────────────────
// Tall straight trunk. Branches alternate left/right at interval=2.
// 25% chance to skip a branch. Short arms (2-6 cells) with corner caps
// and diagonal spray tips.

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

// ── WavyBirch ──────────────────────────────────────────────────
// Like BirchTree but branch endpoints trace a vertical waveform on each side.
// Arms undulate up/down as they extend outward. Left and right sides run
// independent waveforms with different phase/amplitude for asymmetry.

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

// ── StormTree ───────────────────────────────────────────────────
// Leaning trunk (diagonal shifts at regular intervals). Branches
// peel off windward side with progressive lighten. Compact, windy look.

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

// ── DeadTree ────────────────────────────────────────────────────────
// Skeletal, eerie tree with gnarled trunk and sparse angular branches.
// Gnarled trunk: mostly │ with random diagonal offsets (╱/╲) every 7 rows.
// Sparse branches: diagonal then horizontal, alternating left/right.
// Leans progressively lighter; tips use a cycling char set.

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

// ── DroopingTree ────────────────────────────────────────────────
// Short straight trunk (bottom 2/3). should_branch fires once at trunk top.
// draw_branch handles the entire crown: fan of 3-6 arms spread across width.
// Each arm has a horizontal bar from center, vertical rise, then drooping
// horizontal arms with hanging drips (╎) extending downward.

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

// ── PineTree ────────────────────────────────────────────────────────
// Conifer with triangle tiers. Short trunk, stacked needle layers.

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

// ── WillowTree ──────────────────────────────────────────────────────
// Weeping tree: trunk splits into drooping tendrils that hang down.

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

// ── PalmTree ────────────────────────────────────────────────────────
// Tall curved trunk with radiating fronds at the crown.

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

// ── WideTree ─────────────────────────────────────────────────────────
// Pyramidal silhouette: 3 tiered horizontal splits decreasing in width.
// Architectural/formal geometric branching.

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

// ── AsymmetricTree ───────────────────────────────────────────────────
// Wind-blown: one side 40-70% longer. Recursive splits with unequal depth per side.

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

// ── KaijuTree ────────────────────────────────────────────────────────
// Thick 3-wide base, 2-4 trunks diverge with lean, unequal branches.

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

// ── ZigzagTree ───────────────────────────────────────────────────────
// Diagonal-only trunk that zigzags, double-wide. Branches fork as diagonal rays.

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

// ── BrailleCanopyTree ────────────────────────────────────────────────
// Filled elliptical canopy of braille chars with color gradient. Simple trunk.

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

// ── TendrilTree ──────────────────────────────────────────────────────
// Radial explosion: short trunk, rays burst from center with recursive sub-tendrils.

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

// ── OakTree ──────────────────────────────────────────────────────────
// Gnarled trunk, thick recursive zigzag limbs with elbow joints, braille
// leaf clusters at the tips. Foliage grows on the skeleton, not as a blob.

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

// ── FountainTree ─────────────────────────────────────────────────────
// Short trunk; jets launch from the apex and arc under gravity, rising
// near-vertical then spilling outward and down like a fountain.

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

// ── WindsweptTree ────────────────────────────────────────────────────
// Trunk leans hard with the wind; every branch streams to the lee side
// in long near-horizontal runs with small upward kicks at the ends.

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

// ── FractalTree ──────────────────────────────────────────────────────
// Recursive binary fractal: every limb splits into two shorter children
// tilted apart, five levels deep. Random tilt jitter and occasional
// dropped children keep the self-similar silhouette from going symmetric.

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

// ── LSystemTree ──────────────────────────────────────────────────────
// String-rewrite tree: axiom X with rule X -> F[+X]F[-X]+X, expanded
// 2-4 times by energy, then walked by an 8-direction turtle with a
// state stack. Leaf tips appear where deep brackets close.

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

// ── DragonTree ───────────────────────────────────────────────────────
// Two mirrored dragon-curve arms unfold from the apex, drawn with
// box-drawing corners at every fold like a writhing paper crease.

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

// ── HelixTree ────────────────────────────────────────────────────────
// Twin trunk strands wind around each other, crossing in ╳ knots; sprig
// pairs sprout from the knots and the strands part into curls at the crown.

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

// ── Dispatch + space-packing engine ──────────────────────────────────

pub const TREE_KIND_COUNT: usize = 24;

/// Grow any archetype by index (mod TREE_KIND_COUNT). DRY replacement for the
/// per-mode `match kind % N { ... }` blocks.
pub fn grow_tree_by_index(idx: usize, grid: &mut Grid, params: &TreeParams, rng: &mut StdRng) {
    match idx % TREE_KIND_COUNT {
        0 => SpiralTree.grow(grid, params, rng),
        1 => CandelabraTree.grow(grid, params, rng),
        2 => SplitTree.grow(grid, params, rng),
        3 => BirchTree.grow(grid, params, rng),
        4 => WavyBirch.grow(grid, params, rng),
        5 => StormTree::new().grow(grid, params, rng),
        6 => DeadTree.grow(grid, params, rng),
        7 => DroopingTree.grow(grid, params, rng),
        8 => PineTree.grow(grid, params, rng),
        9 => WillowTree.grow(grid, params, rng),
        10 => PalmTree.grow(grid, params, rng),
        11 => WideTree.grow(grid, params, rng),
        12 => AsymmetricTree.grow(grid, params, rng),
        13 => KaijuTree.grow(grid, params, rng),
        14 => ZigzagTree.grow(grid, params, rng),
        15 => BrailleCanopyTree.grow(grid, params, rng),
        16 => TendrilTree.grow(grid, params, rng),
        17 => OakTree.grow(grid, params, rng),
        18 => FountainTree.grow(grid, params, rng),
        19 => WindsweptTree::new(rng).grow(grid, params, rng),
        20 => FractalTree.grow(grid, params, rng),
        21 => LSystemTree.grow(grid, params, rng),
        22 => DragonTree.grow(grid, params, rng),
        _ => HelixTree.grow(grid, params, rng),
    }
}

/// Tuning for the space-packing layout engine.
pub struct PackOpts {
    /// depth bands; layer 0 = back (small/faint), last = front (large/detailed)
    pub layer_count: u8,
    /// 0.0..0.6 -- fraction by which neighboring canopies interleave horizontally
    pub overlap: f32,
    /// 0.0..1.0 -- probability a tree gets a bole base
    pub bole_rate: f32,
    /// 0.2..0.8 -- fraction of canvas height reserved as ground
    pub ground_frac: f32,
    /// restrict archetype pool (None = all TREE_KIND_COUNT)
    pub kind_filter: Option<&'static [usize]>,
}

impl Default for PackOpts {
    fn default() -> Self {
        PackOpts {
            layer_count: 3,
            overlap: 0.25,
            bole_rate: 0.4,
            ground_frac: 0.45,
            kind_filter: None,
        }
    }
}

pub struct PackedSlot {
    pub plot: Rect,
    pub layer: u8,
    pub hue: f64,
    pub energy: f32,
    pub kind: usize,
    pub bole: Option<Bole>,
    pub taper: TaperKind,
    pub root_y: usize,
}

/// Tile the canvas with depth-layered tree plots so every column is covered.
/// Returns (ground_y, slots) with slots sorted back-to-front.
///
/// Coverage strategy: each layer walks x=0..width placing trees whose canopies
/// interleave by `overlap`. Layer index raises both root_y (closer = lower) and
/// canopy height (closer = taller), producing an aerial-perspective tier wall.
pub fn pack_forest(
    width: usize,
    height: usize,
    rng: &mut StdRng,
    opts: &PackOpts,
) -> (usize, Vec<PackedSlot>) {
    use rand::Rng;
    let layer_count = opts.layer_count.clamp(1, 6) as usize;
    let ground_y = ((height as f32 * opts.ground_frac.clamp(0.2, 0.8)) as usize).max(2);
    let all_tapers = [
        TaperKind::Diagonal,
        TaperKind::Shelf,
        TaperKind::Bracket,
        TaperKind::Step,
        TaperKind::Melt,
    ];

    let sky = ground_y;
    let band = (sky / layer_count).max(1);
    let mut slots: Vec<PackedSlot> = Vec::new();

    for li in 0..layer_count {
        let lfrac = if layer_count > 1 {
            li as f32 / (layer_count - 1) as f32
        } else {
            1.0
        };

        // slot width grows toward the front (closer trees are wider)
        let slot_min = (6 + (lfrac * 6.0) as usize).max(4);
        let slot_max = (slot_min + 8 + (lfrac * 10.0) as usize).min(width / 2).max(slot_min);

        // canopy reaches higher toward the front
        let canopy_top = ((sky as i32) - (band as i32) * (li as i32 + 1)).max(1) as usize;

        // roots step downward toward the front (closer sits lower on screen)
        let root_y = (ground_y + li * (height - ground_y) / layer_count.max(1))
            .min(height.saturating_sub(2))
            .max(ground_y);

        let base_energy = 0.40 + lfrac * 0.55;

        let mut x = rng.random_range(0..slot_min as u32) as i32;
        while x < width as i32 {
            let slot_w = rng.random_range(slot_min as u32..=slot_max as u32) as usize;
            let cx = (x + slot_w as i32 / 2).clamp(2, width as i32 - 3) as usize;
            let plot_w = (slot_w + 4).min(width);
            let plot_x = cx.saturating_sub(plot_w / 2);
            let plot_h = root_y.saturating_sub(canopy_top) + 3;
            let plot = Rect {
                x: plot_x,
                y: canopy_top,
                w: plot_w,
                h: plot_h,
            };

            let kind = match opts.kind_filter {
                Some(set) => set[rng.random_range(0..set.len() as u32) as usize],
                None => rng.random_range(0..TREE_KIND_COUNT as u32) as usize,
            };
            let bole = if rng.random::<f32>() < opts.bole_rate.clamp(0.0, 1.0) {
                Some(Bole {
                    style: rng.random_range(0..10u32) as usize,
                })
            } else {
                None
            };
            let taper = all_tapers[rng.random_range(0..all_tapers.len() as u32) as usize];
            let hue = rng.random_range(0..360u32) as f64;

            slots.push(PackedSlot {
                plot,
                layer: li as u8,
                hue,
                energy: base_energy,
                kind,
                bole,
                taper,
                root_y,
            });

            let step = ((slot_w as f32) * (1.0 - opts.overlap.clamp(0.0, 0.6))).max(2.0) as i32;
            x += step;
        }
    }

    // back-to-front: lower layer first, then lower root_y within a layer
    slots.sort_by(|a, b| a.layer.cmp(&b.layer).then(a.root_y.cmp(&b.root_y)));
    (ground_y, slots)
}

// ── Scene-walk placement (high-entropy mixed scenes) ────────────────

/// One placed scene element with per-stop randomized params.
/// Trees carry their OWN energy/height/spread (no per-layer banding).
#[derive(Clone, Copy)]
pub enum SceneEl {
    Tree {
        kind: usize,
        energy: f32,
        spread: usize,
        tree_h: usize,
        bole: Option<Bole>,
        taper: TaperKind,
    },
    Bush {
        style: usize,
        bush_w: i32,
        fade: u8,
    },
    Flowers,
    FruitVine,
    Grass,
    /// intentional negative space -- keeps the scene from being jammed to 11
    Gap,
}

pub struct SceneStop {
    pub x: usize,
    pub root_y: usize,
    pub layer: u8,
    pub hue: f64,
    pub sat: f32,
    pub light: f32,
    pub el: SceneEl,
}

pub struct SceneOpts {
    pub layer_count: u8,
    /// 0.1..1.0 -- shortens hops; higher = more stops (denser)
    pub density: f32,
    /// fraction of stops that become trees (rest split among bush/flower/vine/grass/gap)
    pub tree_rate: f32,
    pub bole_rate: f32,
    pub ground_frac: f32,
    pub kind_filter: Option<&'static [usize]>,
    pub vines: bool,
    /// degrees of per-stop hue jitter around a single per-scene base hue (coherent palette)
    pub hue_range: f32,
}

impl Default for SceneOpts {
    fn default() -> Self {
        SceneOpts {
            layer_count: 3,
            density: 0.4,
            tree_rate: 0.6,
            bole_rate: 0.4,
            ground_frac: 0.42,
            kind_filter: None,
            vines: true,
            hue_range: 32.0,
        }
    }
}

/// Walk the terrain placing mixed scene elements with high per-stop variance.
///
/// Entropy levers (vs the banded `pack_forest`):
/// - per-tree energy/height/spread sampled from WIDE, layer-overlapping ranges
/// - irregular hop distances + occasional gaps (no regular rows)
/// - mixed element vocabulary: trees, bushes, flowers, fruit vines, grass, gaps
/// - per-stop hue/sat/light jitter (no per-layer color banding)
/// Layers still bias size/color by depth, but distributions overlap so layers blur.
pub fn scene_walk(
    width: usize,
    height: usize,
    rng: &mut StdRng,
    opts: &SceneOpts,
) -> (usize, Vec<SceneStop>) {
    use rand::Rng;
    let layer_count = opts.layer_count.clamp(1, 6) as usize;
    let ground_y = ((height as f32 * opts.ground_frac.clamp(0.2, 0.8)) as usize).max(2);
    let all_tapers = [
        TaperKind::Diagonal,
        TaperKind::Shelf,
        TaperKind::Bracket,
        TaperKind::Step,
        TaperKind::Melt,
    ];

    // undulating horizon so roots don't sit on a straight line
    let mut ground_heights: Vec<usize> = Vec::with_capacity(width);
    let mut gh = ground_y as i32;
    for _ in 0..width {
        gh += rng.random_range(0..3u32) as i32 - 1;
        gh = gh.clamp(ground_y as i32 - 3, ground_y as i32 + 3);
        ground_heights.push(gh.max(1) as usize);
    }

    let ground_depth = (height - ground_y).max(1);
    let hop_mul = (1.25 - opts.density.clamp(0.1, 1.0)).max(0.15);
    let tree_rate = opts.tree_rate.clamp(0.1, 0.95);
    let hue_drift = opts.hue_range.clamp(0.0, 180.0);
    // one base hue per scene -> coherent palette, variety lives across seeds
    let scene_hue = rng.random_range(0..360u32) as f64;
    let mut stops: Vec<SceneStop> = Vec::new();

    for li in 0..layer_count {
        let lfrac = if layer_count > 1 {
            li as f32 / (layer_count - 1) as f32
        } else {
            0.6
        };

        // root baseline: back near horizon, front lower -- but root_y jitters per stop
        let root_base = (ground_y as f32 + lfrac * ground_depth as f32 * 0.75) as usize;

        // overlapping canopy-height ranges per layer (overlap = entropy)
        let (h_lo, h_hi) = match li {
            0 => (3u32, 14u32),
            1 => (8, 30),
            _ => (16, 48),
        };
        // hop distances grow toward the front
        let (hop_lo, hop_hi) = match li {
            0 => (3u32, 9u32),
            1 => (5, 14),
            _ => (7, 18),
        };

        let mut x = rng.random_range(0..hop_lo) as usize;
        while x < width {
            let col = x.min(width - 1);
            let ghere = ground_heights[col];
            let root_y = (root_base + ghere.saturating_sub(ground_y))
                .saturating_add(rng.random_range(0..3u32) as usize)
                .min(height.saturating_sub(2));

            // per-stop color: narrow drift around the scene base hue + sat/light depth bias
            let hue = (scene_hue + rng.random_range(-hue_drift..hue_drift) as f64).rem_euclid(360.0);
            let sat = (0.20 + lfrac * 0.35 + rng.random_range(-0.08f32..0.08f32)).clamp(0.08, 0.85);
            let light =
                (0.12 + lfrac * 0.22 + rng.random_range(-0.05f32..0.06f32)).clamp(0.08, 0.50);

            let r = rng.random::<f32>();
            let el = if r < tree_rate {
                // per-tree energy drawn from a wide band, only gently biased by depth
                let energy = (0.30 + lfrac * 0.45 + rng.random_range(-0.20f32..0.25f32))
                    .clamp(0.20, 1.0);
                let tree_h = rng.random_range(h_lo..=h_hi) as usize;
                // spread tracks THIS tree's height so canopies stay proportional
                let spread = (tree_h as f32 * rng.random_range(0.40f32..0.80)).max(1.0) as usize + 1;
                let kind = match opts.kind_filter {
                    Some(s) => s[rng.random_range(0..s.len() as u32) as usize],
                    None => rng.random_range(0..TREE_KIND_COUNT as u32) as usize,
                };
                let bole = if rng.random::<f32>() < opts.bole_rate.clamp(0.0, 1.0) {
                    Some(Bole {
                        style: rng.random_range(0..10u32) as usize,
                    })
                } else {
                    None
                };
                let taper = all_tapers[rng.random_range(0..all_tapers.len() as u32) as usize];
                SceneEl::Tree {
                    kind,
                    energy,
                    spread,
                    tree_h,
                    bole,
                    taper,
                }
            } else if r < tree_rate + 0.12 {
                SceneEl::Bush {
                    style: rng.random_range(0..18u32) as usize,
                    bush_w: rng.random_range(3..9u32) as i32,
                    fade: rng.random_range(0..3u32) as u8,
                }
            } else if r < tree_rate + 0.24 {
                SceneEl::Flowers
            } else if opts.vines && r < tree_rate + 0.32 {
                SceneEl::FruitVine
            } else if r < tree_rate + 0.44 {
                SceneEl::Grass
            } else {
                SceneEl::Gap
            };

            stops.push(SceneStop {
                x: col,
                root_y,
                layer: li as u8,
                hue,
                sat,
                light,
                el,
            });

            let hop = ((rng.random_range(hop_lo..=hop_hi) as f32) * hop_mul).max(1.0) as usize;
            // occasional open gap in the spacing
            let extra = if rng.random::<f32>() < 0.15 {
                rng.random_range(2..8u32) as usize
            } else {
                0
            };
            x += hop + extra;
        }
    }

    // back-to-front, then lower-rooted within a layer drawn later (closer)
    stops.sort_by(|a, b| a.layer.cmp(&b.layer).then(a.root_y.cmp(&b.root_y)));
    (ground_y, stops)
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
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
