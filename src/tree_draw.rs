use crossterm::style::Color;
use rand::rngs::StdRng;
use rand::RngExt;
use std::cell::Cell as StdCell;
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
    pub bole: Option<Bole>,
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
    pub left: i32,   // half-width extending left of x (0 = just center)
    pub right: i32,  // half-width extending right of x (0 = just center)
}

impl BoleExit {
    pub fn point(x: i32, y: i32) -> Self {
        BoleExit { x, y, left: 0, right: 0 }
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

/// Draw a taper zone narrowing from bole exit width to a single trunk column.
/// Returns the (x, y) where the trunk should start drawing.
fn draw_taper(grid: &mut Grid, exit: &BoleExit, color: Color) -> (i32, i32) {
    let mut left = exit.left;
    let mut right = exit.right;
    let mut cy = exit.y;
    let bark = darken(color, 15);

    if left == 0 && right == 0 {
        set(grid, exit.x, cy, '│', color);
        return (exit.x, cy);
    }

    while left > 0 || right > 0 {
        // Draw current row: diagonal edges with horizontal fill
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

        // Shrink: reduce each side, faster for wide boles
        let dl = if left + right > 6 { (left + 1) / 2 } else { 1.min(left) };
        let dr = if left + right > 6 { (right + 1) / 2 } else { 1.min(right) };
        left -= dl;
        right -= dr;

        cy -= 1;
    }

    set(grid, exit.x, cy, '│', color);
    (exit.x, cy)
}

// ── TrunkAlgo ───────────────────────────────────────────────────────────

pub trait TrunkAlgo {
    fn draw(&self, grid: &mut Grid, pen: &mut TreePen,
            params: &TreeParams, rng: &mut StdRng) -> Vec<TrunkNode>;
}

// ── Trunk Algorithms ────────────────────────────────────────────────────

pub struct StraightTrunk {
    pub height_fraction: f32,
}

impl TrunkAlgo for StraightTrunk {
    fn draw(&self, grid: &mut Grid, pen: &mut TreePen,
            params: &TreeParams, _rng: &mut StdRng) -> Vec<TrunkNode> {
        let top_y = params.canopy_top();
        let ry = params.root().1;
        let full_height = (ry - top_y).max(1);
        let height = (full_height as f32 * self.height_fraction) as i32;
        let mut path = Vec::with_capacity(height as usize);

        for _ in 0..height {
            pen.step(grid, MoveDir::Up);
            path.push(TrunkNode { x: pen.x, y: pen.y, dir: MoveDir::Up });
        }

        path
    }
}

pub struct ThickTrunk {
    pub height_fraction: f32,
}

impl TrunkAlgo for ThickTrunk {
    fn draw(&self, grid: &mut Grid, pen: &mut TreePen,
            params: &TreeParams, _rng: &mut StdRng) -> Vec<TrunkNode> {
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
            path.push(TrunkNode { x: pen.x, y: pen.y, dir: MoveDir::Up });
        }

        path
    }
}

pub struct WobbleTrunk {
    pub height_fraction: f32,
}

impl TrunkAlgo for WobbleTrunk {
    fn draw(&self, grid: &mut Grid, pen: &mut TreePen,
            params: &TreeParams, rng: &mut StdRng) -> Vec<TrunkNode> {
        let top_y = params.canopy_top();
        let ry = params.root().1;
        let full_height = (ry - top_y).max(3);
        let trunk_h = (full_height as f32 * self.height_fraction).max(2.0) as i32;
        let freq = rng.random_range(3..6u32) as i32;
        let mut path = Vec::with_capacity(trunk_h as usize);

        for i in 0..trunk_h {
            if i > 0 && i % freq == 0 && rng.random_range(0..3u32) == 0 {
                let h_dir = if rng.random::<bool>() { MoveDir::Right } else { MoveDir::Left };
                pen.step(grid, h_dir);
                pen.step(grid, MoveDir::Up);
            } else {
                pen.step(grid, MoveDir::Up);
            }
            path.push(TrunkNode { x: pen.x, y: pen.y, dir: MoveDir::Up });
        }

        path
    }
}

pub struct LeanTrunk {
    pub lean: StdCell<i32>,
}

impl LeanTrunk {
    pub fn new() -> Self {
        LeanTrunk { lean: StdCell::new(0) }
    }
}

impl TrunkAlgo for LeanTrunk {
    fn draw(&self, grid: &mut Grid, pen: &mut TreePen,
            params: &TreeParams, rng: &mut StdRng) -> Vec<TrunkNode> {
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
                let h_dir = if lean > 0 { MoveDir::Right } else { MoveDir::Left };
                pen.step(grid, h_dir);
                pen.step(grid, MoveDir::Up);
            } else {
                pen.step(grid, MoveDir::Up);
            }

            path.push(TrunkNode { x: pen.x, y: pen.y, dir: MoveDir::Up });
        }

        path
    }
}

pub struct GnarledTrunk;

impl TrunkAlgo for GnarledTrunk {
    fn draw(&self, grid: &mut Grid, pen: &mut TreePen,
            params: &TreeParams, rng: &mut StdRng) -> Vec<TrunkNode> {
        let top_y = params.canopy_top();
        let ry = params.root().1;
        let height = (ry - top_y).max(1);
        let trunk_color = darken(params.trunk_color, 10);
        let mut path = Vec::with_capacity(height as usize);

        pen.color = trunk_color;
        for i in 0..height {
            let from_root = height - i;
            if from_root > 2 && from_root % 7 == 0 && rng.random_range(0..3u32) == 0 {
                let h_dir = if rng.random::<bool>() { MoveDir::Right } else { MoveDir::Left };
                pen.step(grid, h_dir);
                pen.step(grid, MoveDir::Up);
            } else {
                pen.step(grid, MoveDir::Up);
            }
            path.push(TrunkNode { x: pen.x, y: pen.y, dir: MoveDir::Up });
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
    fn draw(&self, grid: &mut Grid, pen: &mut TreePen,
            params: &TreeParams, rng: &mut StdRng) -> Vec<TrunkNode> {
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
                if bias > 0 { MoveDir::UpRight } else { MoveDir::UpLeft }
            } else if drift.abs() >= max_drift {
                // Correct back toward center
                if drift > 0 { MoveDir::UpLeft } else { MoveDir::UpRight }
            } else {
                MoveDir::Up
            };

            pen.step(grid, dir);
            drift += dir.dx();
            path.push(TrunkNode { x: pen.x, y: pen.y, dir });
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
    fn draw(&self, grid: &mut Grid, pen: &mut TreePen,
            params: &TreeParams, rng: &mut StdRng) -> Vec<TrunkNode> {
        let top_y = params.canopy_top();
        let ry = params.root().1;
        let full_height = (ry - top_y).max(3);
        let trunk_h = (full_height as f32 * self.height_fraction).max(3.0) as i32;
        let amp = self.amplitude.max(1).min(params.spread() / 2);
        let period = rng.random_range(4..9u32) as f32;
        let phase = rng.random_range(0..628u32) as f32 / 100.0; // 0..2π
        let mut path = Vec::with_capacity(trunk_h as usize);
        let mut prev_target_x = 0i32;

        for i in 0..trunk_h {
            let t = i as f32 / period;
            let target_x = ((t + phase).sin() * amp as f32).round() as i32;
            let dx = (target_x - prev_target_x).clamp(-1, 1);
            prev_target_x = target_x;

            let dir = match dx {
                -1 => MoveDir::UpLeft,
                1 => MoveDir::UpRight,
                _ => MoveDir::Up,
            };

            pen.step(grid, dir);
            path.push(TrunkNode { x: pen.x, y: pen.y, dir });
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
    fn draw(&self, _grid: &mut Grid, params: &TreeParams,
            _rng: &mut StdRng) -> BoleExit {
        let (x, y) = params.root();
        BoleExit::point(x, y)
    }
}

/// Procedural bole: generates a compact sprite pattern at the trunk base.
/// `style` selects the char family. `width` controls horizontal spread.
/// Each style is a coherent glyph vocabulary like the flower sprites.
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
        let lw = (w / 2 + rng.random_range(0..(w / 2 + 1).max(1) as u32) as i32).max(1).min(max_w);
        let rw = (w - lw + rng.random_range(0..(w / 3 + 1).max(1) as u32) as i32).max(1).min(max_w);
        let bark = darken(color, 15);
        let dim = darken(color, 30);

        match self.style % 18 {
            // Style 0: Crescent -- connected via │ at inner edge positions
            0 => {
                // Ground row: wide crescent
                set(grid, root_x, root_y, '┴', color);
                set(grid, root_x - 1, root_y, '◟', bark);
                set(grid, root_x + 1, root_y, '◞', bark);
                for dx in 2..=lw {
                    set(grid, root_x - dx, root_y, '◠', lighten(bark, ((dx - 2) as u8 * 8).min(40)));
                }
                for dx in 2..=rw {
                    set(grid, root_x + dx, root_y, '◠', lighten(bark, ((dx - 2) as u8 * 8).min(40)));
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
                if bar_l > ilw + 1 { set(grid, root_x - bar_l, root_y - 1, '─', bark); }
                if bar_r > irw + 1 { set(grid, root_x + bar_r, root_y - 1, '─', bark); }
                BoleExit { x: root_x, y: root_y - 1, left: ilw, right: irw }
            }
            // Style 1: Braille cluster -- energy-scaled height
            1 => {
                let energy = params.energy.clamp(0.2, 1.0);
                let dense = ['⣿', '⣾', '⣷', '⣤', '⣶'];
                let mid   = ['⡇', '⢸', '⠿', '⠶', '⠛'];
                let thin  = ['⡀', '⢀', '⠁', '⠈', '⠂'];
                // Row count: 2 at low energy, up to 5 at high
                let rows = ((energy * 4.0).ceil() as i32 + 1).clamp(2, 5);
                let mut cy = root_y;

                for row in 0..rows {
                    let frac = row as f32 / rows as f32;
                    let hw = if row == 0 {
                        lw.max(rw)
                    } else {
                        ((lw.max(rw) as f32) * (1.0 - frac * 0.6)).max(1.0) as i32
                    };
                    let chars = if frac < 0.33 { &dense } else if frac < 0.66 { &mid } else { &thin };
                    let row_col = if row == 0 { color } else { darken(color, (row as u8 * 5).min(20)) };

                    set(grid, root_x, cy, chars[0], row_col);
                    for dx in 1..=hw {
                        let ch = chars[rng.random_range(0..chars.len() as u32) as usize];
                        set(grid, root_x - dx, cy, ch, darken(row_col, ((dx as u8) * 3).min(15)));
                    }
                    for dx in 1..=hw {
                        let ch = chars[rng.random_range(0..chars.len() as u32) as usize];
                        set(grid, root_x + dx, cy, ch, darken(row_col, ((dx as u8) * 3).min(15)));
                    }

                    cy -= 1;
                }

                let exit_hw = ((lw.max(rw) as f32) * (1.0 - (rows - 1) as f32 / rows as f32 * 0.6)).max(1.0) as i32;
                BoleExit { x: root_x, y: cy + 1, left: exit_hw, right: exit_hw }
            }
            // Style 2: Frame -- energy-scaled nested box frames
            2 => {
                let energy = params.energy.clamp(0.2, 1.0);
                let hlw = lw.max(2);
                let hrw = rw.max(2);
                let layers = ((energy * 3.0).ceil() as i32).clamp(1, 3);
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
                    set(grid, root_x, cy, '╩', if layer == 0 { color } else { layer_col });

                    cy -= 1;
                    set(grid, root_x - ll, cy, '╔', layer_col);
                    set(grid, root_x + lr, cy, '╗', layer_col);
                    for dx in (-ll + 1)..lr {
                        set(grid, root_x + dx, cy, '═', layer_col);
                    }
                    set(grid, root_x, cy, '╦', if layer == 0 { color } else { layer_col });

                    if layer == 0 && energy > 0.6 {
                        set(grid, root_x - ll - 1, root_y, '╱', dim);
                        set(grid, root_x + lr + 1, root_y, '╲', dim);
                    }

                    cy -= 1;
                }

                let last_shrink = (layers - 1) as f32 * 0.3;
                let exit_l = ((hlw as f32) * (1.0 - last_shrink)).max(1.0) as i32;
                let exit_r = ((hrw as f32) * (1.0 - last_shrink)).max(1.0) as i32;
                BoleExit { x: root_x, y: cy + 1, left: exit_l, right: exit_r }
            }
            // Style 3: Diamond -- inverted diamond shape, wide at ground narrowing upward
            3 => {
                let energy = params.energy.clamp(0.2, 1.0);
                // Total height scales with energy: 3-7 rows
                let total_h = ((energy * 5.0).ceil() as i32 + 2).clamp(3, 7);
                let half_h = total_h / 2;
                let max_half_w = lw.max(rw).max(2);
                let mut cy = root_y;

                // Bottom half: expanding upward (inverted V)
                for row in 0..half_h {
                    let hw = ((row + 1) as f32 / half_h as f32 * max_half_w as f32).ceil() as i32;
                    let row_col = lighten(bark, ((half_h - row) as u8 * 5).min(30));
                    set(grid, root_x, cy, if row == 0 { '╨' } else { '│' }, color);
                    for dx in 1..=hw {
                        let ch = if dx == hw { '◇' } else if dx % 2 == 0 { '─' } else { '◆' };
                        set(grid, root_x - dx, cy, ch, row_col);
                        set(grid, root_x + dx, cy, ch, row_col);
                    }
                    cy -= 1;
                }

                // Middle row: widest point
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

                // Top half: contracting upward (V shape)
                let top_rows = total_h - half_h - 1;
                for row in 0..top_rows {
                    let hw = ((top_rows - row) as f32 / top_rows as f32 * max_half_w as f32).ceil() as i32;
                    let row_col = lighten(bark, ((row + 1) as u8 * 6).min(35));
                    set(grid, root_x, cy, '│', row_col);
                    for dx in 1..=hw {
                        let ch = if dx == hw { '◇' } else if dx % 2 == 0 { '─' } else { '◆' };
                        set(grid, root_x - dx, cy, ch, row_col);
                        set(grid, root_x + dx, cy, ch, row_col);
                    }
                    cy -= 1;
                }

                let exit_hw = (1.0f32 / top_rows as f32 * max_half_w as f32).ceil() as i32;
                BoleExit { x: root_x, y: cy + 1, left: exit_hw, right: exit_hw }
            }
            // Style 4: Chevron -- energy-scaled layered V-shapes with variable center
            4 => {
                let energy = params.energy.clamp(0.2, 1.0);
                // Number of chevron layers: 1 at low, up to 4 at high
                let layers = ((energy * 3.5).ceil() as i32).clamp(1, 4);
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
                    let lc = if layer == 0 { bark } else { lighten(bark, (layer as u8 * 8).min(30)) };

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
                let energy = params.energy.clamp(0.2, 1.0);
                // Layer count: 1-3, varies with energy but not always max
                let max_layers = ((energy * 3.0).ceil() as i32).clamp(1, 3);
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
                    let interior_h = if energy > 0.7 {
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
                        let next_lw = ((cur_lw as f32) * (0.55 + rng.random::<f32>() * 0.25)).max(1.0) as i32;
                        let next_rw = ((cur_rw as f32) * (0.55 + rng.random::<f32>() * 0.25)).max(1.0) as i32;
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

                BoleExit { x: root_x, y: cy, left: cur_lw, right: cur_rw }
            }
            // Style 6: Crescent2 -- turbo crescent with hips, valid box-drawing connections
            6 => {
                let energy = params.energy.clamp(0.2, 1.0);
                let layers = ((energy * 4.0).ceil() as i32).clamp(2, 5);
                let mut cy = root_y;

                // Ground layer: widest crescent with hip flares
                set(grid, root_x, cy, '┴', color);
                for dx in 1..=lw {
                    let ch = if dx <= 2 { '═' } else { '◠' };
                    set(grid, root_x - dx, cy, ch, lighten(color, ((dx as u8) * 3).min(25)));
                }
                for dx in 1..=rw {
                    let ch = if dx <= 2 { '═' } else { '◠' };
                    set(grid, root_x + dx, cy, ch, lighten(color, ((dx as u8) * 3).min(25)));
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
                BoleExit { x: root_x, y: cy + 1, left: exit_l, right: exit_r }
            }
            // Style 7: Braille2 -- thick braille with tapered trunk exit
            7 => {
                let energy = params.energy.clamp(0.2, 1.0);
                let dense = ['⣿', '⣾', '⣷', '⣶', '⣤'];
                let mid   = ['⡇', '⢸', '⠿', '⠶', '⠛'];
                let thin  = ['⡀', '⢀', '⠂', '⠈', '⠁'];
                let rows = ((energy * 4.0).ceil() as i32 + 1).clamp(2, 5);
                let mut cy = root_y;
                let base_w = lw.max(rw).max(2);

                for row in 0..rows {
                    let frac = row as f32 / rows as f32;
                    let hw = ((base_w as f32) * (1.0 - frac * 0.5)).max(1.0) as i32;
                    let chars = if frac < 0.3 { &dense } else if frac < 0.6 { &mid } else { &thin };
                    let rc = if row == 0 { color } else { darken(color, (row as u8 * 4).min(15)) };

                    // Asymmetric: left and right can have different widths
                    let lhw = hw + rng.random_range(0..2u32) as i32;
                    let rhw = hw + rng.random_range(0..2u32) as i32;

                    set(grid, root_x, cy, chars[0], rc);
                    for dx in 1..=lhw {
                        set(grid, root_x - dx, cy, chars[rng.random_range(0..chars.len() as u32) as usize], darken(rc, ((dx as u8) * 2).min(10)));
                    }
                    for dx in 1..=rhw {
                        set(grid, root_x + dx, cy, chars[rng.random_range(0..chars.len() as u32) as usize], darken(rc, ((dx as u8) * 2).min(10)));
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
                BoleExit { x: root_x, y: cy + 1, left: exit_hw, right: exit_hw }
            }
            // Style 8: Frame3 -- stacked boxes, heaviest at bottom, randomly off-center
            8 => {
                let energy = params.energy.clamp(0.2, 1.0);
                let boxes = ((energy * 3.0).ceil() as i32).clamp(1, 4);
                let mut cy = root_y;
                let mut cur_lw = lw.max(3);
                let mut cur_rw = rw.max(3);
                let mut cx = root_x;

                for b in 0..boxes {
                    let bc = if b == 0 { color } else { lighten(bark, (b as u8 * 8).min(25)) };
                    let fc = if b == 0 { bark } else { lighten(dim, (b as u8 * 5).min(20)) };
                    // Interior height: biggest box at bottom, smaller going up
                    let interior = if b == 0 {
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
                        let fills = if row == 0 { ['░', '▒', '░', '·'] } else { ['▒', '▓', '▒', '░'] };
                        for dx in (-cur_lw + 1)..cur_rw {
                            set(grid, cx + dx, cy, fills[rng.random_range(0..4u32) as usize], fc);
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
                BoleExit { x: root_x, y: cy + 1, left: cur_lw, right: cur_rw }
            }
            // Style 9: Diamond2 -- asymmetric diamond with cross-hatching
            9 => {
                let energy = params.energy.clamp(0.2, 1.0);
                let total_h = ((energy * 5.0).ceil() as i32 + 2).clamp(3, 7);
                let mut cy = root_y;
                // Asymmetric: bottom half taller than top
                let bot_h = (total_h * 2 / 3).max(2);
                let top_h = (total_h - bot_h).max(1);
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
                        let ch = if rng.random_range(0..4u32) == 0 { '╳' } else if dx % 2 == 0 { '─' } else { '◆' };
                        set(grid, root_x - dx, cy, ch, rc);
                    }
                    for dx in 1..=hw_r {
                        let ch = if rng.random_range(0..4u32) == 0 { '╳' } else if dx % 2 == 0 { '─' } else { '◆' };
                        set(grid, root_x + dx, cy, ch, rc);
                    }
                    cy -= 1;
                }

                // Widest row: asymmetric
                set(grid, root_x, cy, '◆', color);
                for dx in 1..=max_lw {
                    let ch = if dx % 3 == 0 { '╳' } else { '═' };
                    set(grid, root_x - dx, cy, ch, lighten(color, ((dx as u8) * 3).min(20)));
                }
                for dx in 1..=max_rw {
                    let ch = if dx % 3 == 0 { '╳' } else { '═' };
                    set(grid, root_x + dx, cy, ch, lighten(color, ((dx as u8) * 3).min(20)));
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
                        let ch = if rng.random_range(0..5u32) == 0 { '╳' } else if dx % 2 == 0 { '─' } else { '◇' };
                        set(grid, root_x - dx, cy, ch, rc);
                    }
                    for dx in 1..=hw_r {
                        let ch = if rng.random_range(0..5u32) == 0 { '╳' } else if dx % 2 == 0 { '─' } else { '◇' };
                        set(grid, root_x + dx, cy, ch, rc);
                    }
                    cy -= 1;
                }

                let exit_hw_l = (1.0f32 / top_h as f32 * max_lw as f32).ceil() as i32;
                let exit_hw_r = (1.0f32 / top_h as f32 * max_rw as f32).ceil() as i32;
                BoleExit { x: root_x, y: cy + 1, left: exit_hw_l, right: exit_hw_r }
            }
            // Style 10: Chevron2 -- chevron with horizontal sprawl near base
            10 => {
                let energy = params.energy.clamp(0.2, 1.0);
                let layers = ((energy * 3.5).ceil() as i32).clamp(1, 4);
                let mut cy = root_y;
                let ll = lw.max(2);
                let rl = rw.max(2);

                // Ground sprawl: horizontal bars at base
                set(grid, root_x, cy, '┴', color);
                for dx in 1..=ll {
                    set(grid, root_x - dx, cy, '═', lighten(bark, ((dx as u8) * 3).min(20)));
                }
                for dx in 1..=rl {
                    set(grid, root_x + dx, cy, '═', lighten(bark, ((dx as u8) * 3).min(20)));
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
                    set(grid, root_x - dx, cy, '╲', lighten(bark, ((dx as u8) * 4).min(25)));
                }
                for dx in 1..=rl {
                    set(grid, root_x + dx, cy, '╱', lighten(bark, ((dx as u8) * 4).min(25)));
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
                    let lc = if layer == 0 { bark } else { lighten(bark, (layer as u8 * 7).min(30)) };

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
                let energy = params.energy.clamp(0.2, 1.0);
                let boxes = ((energy * 3.0).ceil() as i32).clamp(1, 3);
                let mut cy = root_y;
                let mut cur_lw = lw.max(3);
                let mut cur_rw = rw.max(3);
                let mut cx = root_x;

                for b in 0..boxes {
                    let bc = if b == 0 { color } else { lighten(bark, (b as u8 * 10).min(30)) };
                    let fc = if b == 0 { bark } else { dim };
                    let interior = if b == 0 {
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
                            set(grid, cx + dx, cy, fills[rng.random_range(0..4u32) as usize], fc);
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
                BoleExit { x: root_x, y: cy + 1, left: cur_lw, right: cur_rw }
            }
            // Style 15: Keel -- short fat asymmetric hull, 2-4 rows max
            15 => {
                let energy = params.energy.clamp(0.2, 1.0);
                let total_h = ((energy * 2.0).ceil() as i32 + 1).clamp(2, 4);
                let mut cy = root_y;
                let bias = if rng.random::<bool>() { 1.4f32 } else { 0.6f32 };
                let max_lw = ((lw as f32) * bias * 1.3).max(3.0) as i32;
                let max_rw = ((rw as f32) * (2.0 - bias) * 1.3).max(3.0) as i32;

                for row in 0..total_h {
                    let frac = 1.0 - (row as f32 / total_h as f32);
                    let hl = (max_lw as f32 * frac).ceil() as i32;
                    let hr = (max_rw as f32 * frac).ceil() as i32;
                    let rc = if row == 0 { color } else { lighten(bark, ((row as u8) * 6).min(25)) };

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
                        if hl > 0 { set(grid, root_x - hl, cy, '╲', rc); }
                        if hr > 0 { set(grid, root_x + hr, cy, '╱', rc); }
                    }
                    cy -= 1;
                }

                let exit_frac = 1.0 - ((total_h - 1) as f32 / total_h as f32);
                let exit_l = (max_lw as f32 * exit_frac).ceil() as i32;
                let exit_r = (max_rw as f32 * exit_frac).ceil() as i32;
                BoleExit { x: root_x, y: cy + 1, left: exit_l, right: exit_r }
            }
            // Style 17: Buttress -- bright bold curved grounding legs
            17 => {
                let energy = params.energy.clamp(0.2, 1.0);
                let mut cy = root_y;

                let left_reach = lw.max(2) + rng.random_range(1..4u32) as i32;
                let right_reach = rw.max(2) + rng.random_range(0..3u32) as i32;

                // Ground anchor: bold and bright
                set(grid, root_x, cy, '╨', color);
                set(grid, root_x - 1, cy, '═', color);
                set(grid, root_x + 1, cy, '═', color);
                if lw > 2 { set(grid, root_x - 2, cy, '═', bark); }
                if rw > 2 { set(grid, root_x + 2, cy, '═', bark); }

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
                        if ly <= root_y + 2 { set(grid, lx, ly, '│', lighten(color, 10)); }
                    } else {
                        lx -= 1;
                        ly += 1;
                        if ly <= root_y + 2 { set(grid, lx, ly, '╲', lighten(color, 5)); }
                    }
                }
                if ly <= root_y + 2 { set(grid, lx, ly, '╰', lighten(color, 10)); }

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
                        if ry <= root_y + 2 { set(grid, rx, ry, '│', lighten(color, 10)); }
                    } else {
                        rx += 1;
                        ry += 1;
                        if ry <= root_y + 2 { set(grid, rx, ry, '╱', lighten(color, 5)); }
                    }
                }
                if ry <= root_y + 2 { set(grid, rx, ry, '╯', lighten(color, 10)); }

                // Cross-brace at high energy: BRIGHT
                if energy > 0.5 {
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
                if energy > 0.4 {
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
                BoleExit { x: root_x, y: cy + 1, left: sl, right: sr }
            }
            12 => {
                BoleExit::point(root_x, root_y)
            }
            // Style 13: Braille -- horizontal shelf bole, wide ground then sharp drop
            13 => {
                let energy = params.energy.clamp(0.2, 1.0);
                let dense = ['⣿', '⣾', '⣷', '⣶', '⣤'];
                let mid   = ['⡇', '⢸', '⠿', '⠶', '⠛'];
                let edge  = ['⡀', '⢀', '⠂', '⠁', '⠈'];

                // Shelf structure: 1-2 wide ground rows, then sharp narrow
                let shelf_rows = if energy > 0.5 { 2 } else { 1 };
                let upper_rows = ((energy * 2.0).ceil() as i32).clamp(0, 3);
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
                        set(grid, root_x - dx, cy, ch, darken(rc, ((dx as u8) * 2).min(8)));
                    }
                    for dx in 1..=sr {
                        let ch = dense[rng.random_range(0..dense.len() as u32) as usize];
                        set(grid, root_x + dx, cy, ch, darken(rc, ((dx as u8) * 2).min(8)));
                    }
                    set(grid, root_x - sl - 1, cy, edge[rng.random_range(0..edge.len() as u32) as usize], dim);
                    set(grid, root_x + sr + 1, cy, edge[rng.random_range(0..edge.len() as u32) as usize], dim);
                    cy -= 1;
                }

                // SHARP DROP: immediately much narrower (40-60% of shelf width)
                let drop_frac = 0.4 + rng.random::<f32>() * 0.2;
                let mut cur_l = (base_l as f32 * drop_frac).max(1.0) as i32;
                let mut cur_r = (base_r as f32 * drop_frac).max(1.0) as i32;

                for row in 0..upper_rows {
                    let rc = darken(color, ((shelf_rows + row) as u8 * 3).min(12));
                    let chars = if row == 0 { &mid } else { &mid };
                    set(grid, root_x, cy, dense[rng.random_range(0..2u32) as usize], rc);
                    for dx in 1..=cur_l {
                        set(grid, root_x - dx, cy, chars[rng.random_range(0..chars.len() as u32) as usize], darken(rc, ((dx as u8) * 2).min(8)));
                    }
                    for dx in 1..=cur_r {
                        set(grid, root_x + dx, cy, chars[rng.random_range(0..chars.len() as u32) as usize], darken(rc, ((dx as u8) * 2).min(8)));
                    }
                    set(grid, root_x - cur_l - 1, cy, edge[rng.random_range(0..edge.len() as u32) as usize], dim);
                    set(grid, root_x + cur_r + 1, cy, edge[rng.random_range(0..edge.len() as u32) as usize], dim);
                    // Taper each upper row slightly
                    cur_l = (cur_l - rng.random_range(0..2u32) as i32).max(1);
                    cur_r = (cur_r - rng.random_range(0..2u32) as i32).max(1);
                    cy -= 1;
                }

                BoleExit { x: root_x, y: cy, left: cur_l, right: cur_r }
            }
            // Style 14: Frame -- overlapping rects with foreground cross glyphs
            14 => {
                let energy = params.energy.clamp(0.2, 1.0);
                let rects = ((energy * 2.0).ceil() as i32).clamp(1, 3);
                let mut cy = root_y;

                let mut specs: Vec<(i32, i32, i32, i32)> = Vec::new();
                for r in 0..rects {
                    let drift = if r == 0 { 0 } else { rng.random_range(0..3u32) as i32 - 1 };
                    let rlw = if r == 0 { lw.max(3) } else { (lw as f32 * (0.4 + rng.random::<f32>() * 0.4)).max(2.0) as i32 };
                    let rrw = if r == 0 { rw.max(3) } else { (rw as f32 * (0.4 + rng.random::<f32>() * 0.4)).max(2.0) as i32 };
                    let ih = if r == 0 { ((energy * 2.0).ceil() as i32).clamp(1, 2) } else { 1 };
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
                    let bc = if heavy { color } else { lighten(bark, (ri as u8 * 10).min(25)) };
                    let fc = if heavy { bark } else { lighten(dim, (ri as u8 * 6).min(20)) };

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
                        let fills = if row == 0 { ['░', '▒', '·', '░'] } else { ['▒', '▓', '░', '·'] };
                        for dx in (-rlw + 1)..rrw {
                            set(grid, cx + dx, cy, fills[rng.random_range(0..4u32) as usize], fc);
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

                BoleExit { x: root_x, y: cy, left: last_lw, right: last_rw }
            }
            // Style 16: Chevron -- off-center layers that overlap into diamond patterns
            16 => {
                let energy = params.energy.clamp(0.2, 1.0);
                let layers = ((energy * 4.0).ceil() as i32).clamp(2, 5);
                let mut cy = root_y;
                let ll = lw.max(2);
                let rl = rw.max(2);

                // Ground sprawl
                set(grid, root_x, cy, '┴', color);
                let sprawl = ll + rl + rng.random_range(1..4u32) as i32;
                let sprawl_l = sprawl / 2 + rng.random_range(0..3u32) as i32;
                let sprawl_r = sprawl - sprawl_l;
                for dx in 1..=sprawl_l {
                    set(grid, root_x - dx, cy, if dx <= ll { '═' } else { '─' }, lighten(bark, ((dx as u8) * 2).min(20)));
                }
                for dx in 1..=sprawl_r {
                    set(grid, root_x + dx, cy, if dx <= rl { '═' } else { '─' }, lighten(bark, ((dx as u8) * 2).min(20)));
                }
                set(grid, root_x - sprawl_l - 1, cy, '╴', dim);
                set(grid, root_x + sprawl_r + 1, cy, '╶', dim);
                cy -= 1;

                // Chevron layers with random drift -- overlaps create diamonds
                for layer in 0..layers {
                    let shrink = (layer + 1) as f32 * 0.15;
                    let cl = ((ll as f32) * (1.0 - shrink)).max(1.0) as i32;
                    let cr = ((rl as f32) * (1.0 - shrink)).max(1.0) as i32;
                    let lc = if layer == 0 { bark } else { lighten(bark, (layer as u8 * 6).min(30)) };
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
            _ => BoleExit::point(root_x, root_y),
        }
    }
}

pub struct TreeWithTrunk<T: TreeDrawer> {
    pub tree: T,
    pub trunk: Box<dyn TrunkAlgo>,
}

impl<T: TreeDrawer> TreeDrawer for TreeWithTrunk<T> {
    fn draw_trunk(&self, grid: &mut Grid, pen: &mut TreePen,
                  params: &TreeParams, rng: &mut StdRng) -> Vec<TrunkNode> {
        self.trunk.draw(grid, pen, params, rng)
    }

    fn should_branch(&self, idx: usize, count: usize,
                     params: &TreeParams, rng: &mut StdRng) -> Option<BranchIntent> {
        self.tree.should_branch(idx, count, params, rng)
    }

    fn draw_branch(&self, grid: &mut Grid, pen: &mut TreePen,
                   intent: &BranchIntent, depth: usize,
                   params: &TreeParams, rng: &mut StdRng) -> BranchResult {
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
        let exit = if let Some(ref bole) = params.bole {
            bole.draw(grid, params, rng)
        } else {
            let (rx, ry) = params.root();
            BoleExit::point(rx, ry)
        };
        let (rx, ry) = draw_taper(grid, &exit, params.trunk_color);
        let mut pen = TreePen::new(rx, ry, params.trunk_color);
        pen.last_dir = Some(MoveDir::Up);

        let trunk = self.draw_trunk(grid, &mut pen, params, rng);
        if trunk.is_empty() { return; }

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
        params: &TreeParams, rng: &mut StdRng,
    ) -> Vec<TrunkNode> {
        StraightTrunk { height_fraction: 1.0 }.draw(grid, pen, params, rng)
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
        params: &TreeParams, rng: &mut StdRng,
    ) -> Vec<TrunkNode> {
        ThickTrunk { height_fraction: 1.0 / 3.0 }.draw(grid, pen, params, rng)
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
        WobbleTrunk { height_fraction: 1.0 / 3.0 }.draw(grid, pen, params, rng)
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
        params: &TreeParams, rng: &mut StdRng,
    ) -> Vec<TrunkNode> {
        StraightTrunk { height_fraction: 1.0 }.draw(grid, pen, params, rng)
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

pub struct StormTree {
    /// Lean direction picked during draw_trunk, consumed by draw_branch.
    /// +1 = lean right (branches go left), -1 = lean left (branches go right).
    lean_trunk: LeanTrunk,
}

impl StormTree {
    pub fn new() -> Self {
        StormTree { lean_trunk: LeanTrunk::new() }
    }
}

impl TreeDrawer for StormTree {
    fn draw_trunk(
        &self, grid: &mut Grid, pen: &mut TreePen,
        params: &TreeParams, rng: &mut StdRng,
    ) -> Vec<TrunkNode> {
        self.lean_trunk.draw(grid, pen, params, rng)
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
        let go_left = self.lean_trunk.lean.get() > 0;

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
        GnarledTrunk.draw(grid, pen, params, rng)
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
        params: &TreeParams, rng: &mut StdRng,
    ) -> Vec<TrunkNode> {
        StraightTrunk { height_fraction: 2.0 / 3.0 }.draw(grid, pen, params, rng)
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
            bole: None,
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
