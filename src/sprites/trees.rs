//! Tree-pen growth algorithms.
use crate::color::*;
use crate::types::*;
use crossterm::style::Color;
use rand::RngExt;
use rand::rngs::StdRng;
use super::*;

/// Movement direction for connected drawing.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum MoveDir {
    Up,
    Down,
    Left,
    Right,
    UpLeft,
    UpRight,
    DownLeft,
    DownRight,
}
impl MoveDir {
    pub fn dx(self) -> i32 {
        match self {
            MoveDir::Left | MoveDir::UpLeft | MoveDir::DownLeft => -1,
            MoveDir::Right | MoveDir::UpRight | MoveDir::DownRight => 1,
            _ => 0,
        }
    }
    pub fn dy(self) -> i32 {
        match self {
            MoveDir::Up | MoveDir::UpLeft | MoveDir::UpRight => -1,
            MoveDir::Down | MoveDir::DownLeft | MoveDir::DownRight => 1,
            _ => 0,
        }
    }
}
/// Given previous travel direction and next travel direction, return the
/// box-drawing character that connects them at the turning point.
pub(crate) fn connect_glyph(from: MoveDir, to: MoveDir) -> char {
    use MoveDir::*;
    match (from, to) {
        // Straight continuations
        (Up, Up) | (Down, Down) => '│',
        (Left, Left) | (Right, Right) => '─',
        (UpRight, UpRight) | (DownLeft, DownLeft) => '╱',
        (UpLeft, UpLeft) | (DownRight, DownRight) => '╲',

        // Cardinal turns: (prev_travel_dir, next_travel_dir)
        // Char needs exit toward opposite(from) to connect back + exit toward `to`.
        // Was going up (prev cell below), turning right
        (Up, Right) => '╭', // exits: Down, Right
        (Up, Left) => '╮',  // exits: Down, Left
        // Was going down (prev cell above), turning right/left
        (Down, Right) => '╰', // exits: Up, Right
        (Down, Left) => '╯',  // exits: Up, Left
        // Was going right (prev cell left), turning up/down
        (Right, Up) => '╯',   // exits: Up, Left
        (Right, Down) => '╮', // exits: Down, Left
        // Was going left (prev cell right), turning up/down
        (Left, Up) => '╰',   // exits: Up, Right
        (Left, Down) => '╭', // exits: Down, Right

        // T-junctions (straight + branch)
        (Up, _) | (Down, _) if to.dx() != 0 => '├',
        (Left, _) | (Right, _) if to.dy() != 0 => '┬',

        // Diagonal-to-cardinal transitions
        (UpRight, Up) | (UpLeft, Up) => '│',
        (UpRight, Right) | (DownRight, Right) => '─',
        (DownRight, Down) | (DownLeft, Down) => '│',
        (UpLeft, Left) | (DownLeft, Left) => '─',

        // Cardinal-to-diagonal
        (Up, UpRight) | (Up, UpLeft) => '│',
        (Right, UpRight) | (Right, DownRight) => '─',
        (Down, DownRight) | (Down, DownLeft) => '│',
        (Left, UpLeft) | (Left, DownLeft) => '─',

        _ => '·', // fallback
    }
}
/// What exits does a box-drawing character have?
/// Returns the set of directions you can travel FROM this character.
pub fn char_exits(ch: char) -> &'static [MoveDir] {
    use MoveDir::*;
    match ch {
        '│' | '┃' => &[Up, Down],
        '─' | '━' => &[Left, Right],
        '╱' => &[UpRight, DownLeft],
        '╲' => &[UpLeft, DownRight],

        // Corners / curves
        '╭' | '┌' => &[Down, Right],
        '╮' | '┐' => &[Down, Left],
        '╰' | '└' => &[Up, Right],
        '╯' | '┘' => &[Up, Left],

        // T-junctions
        '├' | '┣' => &[Up, Down, Right],
        '┤' | '┫' => &[Up, Down, Left],
        '┬' | '┳' => &[Left, Right, Down],
        '┴' | '┻' => &[Left, Right, Up],

        // Cross
        '┼' | '╋' => &[Up, Down, Left, Right],

        // Half-lines (stubs)
        '╷' => &[Up],    // stub pointing up (endpoint coming from above)
        '╵' => &[Down],  // stub pointing down
        '╴' => &[Left],  // stub pointing left
        '╶' => &[Right], // stub pointing right

        _ => &[],
    }
}
/// Opposite direction (for checking entrance compatibility).
pub fn opposite(dir: MoveDir) -> MoveDir {
    use MoveDir::*;
    match dir {
        Up => Down,
        Down => Up,
        Left => Right,
        Right => Left,
        UpLeft => DownRight,
        UpRight => DownLeft,
        DownLeft => UpRight,
        DownRight => UpLeft,
    }
}
/// Can we enter `ch` from `entry_dir`? (i.e. does the char have an exit
/// in the opposite direction, meaning the line continues toward us?)
pub fn can_enter_from(ch: char, entry_dir: MoveDir) -> bool {
    let need_exit = opposite(entry_dir);
    char_exits(ch).contains(&need_exit)
}
/// Given a desired entry direction and exit direction, return the char
/// that connects them. Entry is where the line comes FROM (so the char
/// needs an exit in opposite(entry)), exit is where it goes TO.
pub fn char_for_connection(entry: MoveDir, exit: MoveDir) -> char {
    use MoveDir::*;
    // entry_exit: the char needs exit toward opposite(entry) AND toward exit
    let from = opposite(entry); // char's exit back toward where we came from
    let to = exit; // char's exit toward where we're going

    match (from, to) {
        // Straight
        (Up, Down) | (Down, Up) => '│',
        (Left, Right) | (Right, Left) => '─',
        (UpRight, DownLeft) | (DownLeft, UpRight) => '╱',
        (UpLeft, DownRight) | (DownRight, UpLeft) => '╲',

        // Curves
        (Down, Right) | (Right, Down) => '╭',
        (Down, Left) | (Left, Down) => '╮',
        (Up, Right) | (Right, Up) => '╰',
        (Up, Left) | (Left, Up) => '╯',

        // T-junctions when one axis is straight and we're adding a branch
        _ => '┼', // fallback to cross
    }
}
/// From a given cell with char `ch`, what are the valid next positions
/// and the direction to get there?
pub fn valid_moves(ch: char) -> Vec<(MoveDir, i32, i32)> {
    char_exits(ch)
        .iter()
        .map(|&d| (d, d.dx(), d.dy()))
        .collect()
}
/// Direction-appropriate continuation glyph (what to draw while moving in a direction).
pub(crate) fn dir_glyph(dir: MoveDir) -> char {
    use MoveDir::*;
    match dir {
        Up | Down => '│',
        Left | Right => '─',
        UpRight | DownLeft => '╱',
        UpLeft | DownRight => '╲',
    }
}
/// A pen that draws connected paths on the grid.
/// Tracks current position and last movement direction.
pub struct TreePen {
    pub x: i32,
    pub y: i32,
    pub last_dir: Option<MoveDir>,
    pub color: Color,
}
impl TreePen {
    pub fn new(x: i32, y: i32, color: Color) -> Self {
        TreePen {
            x,
            y,
            last_dir: None,
            color,
        }
    }

    /// Move one step in the given direction, drawing the correct connecting glyph.
    /// Returns the new (x, y) position.
    pub fn step(&mut self, grid: &mut Grid, dir: MoveDir) -> (i32, i32) {
        // At current position, draw the turn/junction if changing direction
        if let Some(prev) = self.last_dir {
            if prev != dir {
                let ch = connect_glyph(prev, dir);
                tset_over(grid, self.x, self.y, ch, self.color);
            }
        }

        // Move
        self.x += dir.dx();
        self.y += dir.dy();

        // Draw continuation glyph at new position
        tset_over(grid, self.x, self.y, dir_glyph(dir), self.color);
        self.last_dir = Some(dir);

        (self.x, self.y)
    }

    /// Draw a straight run of `n` steps in the given direction.
    pub fn run(&mut self, grid: &mut Grid, dir: MoveDir, n: usize) {
        for _ in 0..n {
            self.step(grid, dir);
        }
    }

    /// Draw a tip/endpoint at current position.
    pub fn tip(&self, grid: &mut Grid) {
        tset_over(grid, self.x, self.y, '╷', lighten(self.color, 30));
    }

    /// Fork: return a new pen at the current position for drawing a branch.
    pub fn fork(&self, color: Color) -> TreePen {
        TreePen {
            x: self.x,
            y: self.y,
            last_dir: self.last_dir,
            color,
        }
    }
}
/// Trunk drawing styles -- each produces a different visual character.
/// draw_trunk returns the x-path (y -> x) so branches can attach correctly.
#[derive(Clone, Copy)]
pub enum TrunkStyle {
    Straight, // │
    Wobble,   // │ with random ╱╲ lateral shifts
    Curved,   // S-curve using ╭╯╰╮
    Thick,    // ┃ center flanked by │
    Gnarled,  // irregular width with knots ┼ ╋
}
/// Draw a trunk from bot_y (ground) up to top_y using the given style.
/// Returns Vec<(y, x)> sorted top-to-bottom for branch attachment lookup.
pub fn draw_trunk(
    grid: &mut Grid,
    start_x: i32,
    top_y: i32,
    bot_y: i32,
    style: TrunkStyle,
    color: Color,
    rng: &mut StdRng,
) -> Vec<(i32, i32)> {
    let mut path: Vec<(i32, i32)> = Vec::new();
    let bark = darken(color, 15);
    let mut cx = start_x;

    match style {
        TrunkStyle::Straight => {
            for y in top_y..bot_y {
                tset_over(grid, cx, y, '│', color);
                path.push((y, cx));
            }
        }
        TrunkStyle::Wobble => {
            let freq = rng.random_range(3..7u32) as i32;
            for y in (top_y..bot_y).rev() {
                let rows_up = bot_y - y;
                let ch = if rows_up > 1 && rows_up % freq == 0 && rng.random_range(0..3u32) == 0 {
                    let dir = rng.random_range(0..2u32) as i32 * 2 - 1;
                    cx += dir;
                    if dir > 0 { '╱' } else { '╲' }
                } else {
                    '│'
                };
                tset_over(grid, cx, y, ch, color);
                path.push((y, cx));
            }
        }
        TrunkStyle::Curved => {
            // S-curve: trunk bends left then right (or vice versa)
            let height = (bot_y - top_y).max(1);
            let bend_dir: i32 = if rng.random_range(0..2u32) == 0 {
                1
            } else {
                -1
            };
            let bend_amount = rng.random_range(1..4u32) as i32;
            for y in top_y..bot_y {
                let t = (y - top_y) as f32 / height as f32;
                // Sine-like offset
                let offset = (bend_dir as f32
                    * bend_amount as f32
                    * (t * std::f32::consts::PI).sin()) as i32;
                let x = start_x + offset;
                let ch = if offset > 0 && y > top_y && (y - top_y) < height / 2 {
                    '╲'
                } else if offset < 0 && y > top_y && (y - top_y) < height / 2 {
                    '╱'
                } else if offset > 0 && (y - top_y) >= height / 2 {
                    '╱'
                } else if offset < 0 && (y - top_y) >= height / 2 {
                    '╲'
                } else {
                    '│'
                };
                tset_over(grid, x, y, ch, color);
                path.push((y, x));
            }
        }
        TrunkStyle::Thick => {
            for y in top_y..bot_y {
                tset_over(grid, cx, y, '┃', color);
                tset_over(grid, cx - 1, y, '│', bark);
                tset_over(grid, cx + 1, y, '│', bark);
                path.push((y, cx));
            }
        }
        TrunkStyle::Gnarled => {
            let mut trunk_width: i32 = 1;
            for y in (top_y..bot_y).rev() {
                let rows_up = bot_y - y;
                // Width increases toward base
                if rows_up % 4 == 0 && trunk_width < 3 && rng.random_range(0..2u32) == 0 {
                    trunk_width += 1;
                }
                // Wobble
                if rng.random_range(0..5u32) == 0 {
                    cx += rng.random_range(0..2u32) as i32 * 2 - 1;
                }
                // Draw width
                tset_over(grid, cx, y, '│', color);
                for w in 1..trunk_width {
                    tset_over(grid, cx - w, y, '│', bark);
                    tset_over(grid, cx + w, y, '│', bark);
                }
                // Knots at random
                if rng.random_range(0..6u32) == 0 {
                    let knot_ch = if rng.random_range(0..2u32) == 0 {
                        '┼'
                    } else {
                        '╋'
                    };
                    tset_over(grid, cx, y, knot_ch, lighten(color, 10));
                }
                // Bark nubs
                if rng.random_range(0..5u32) == 0 {
                    let side = if rng.random_range(0..2u32) == 0 {
                        -1i32
                    } else {
                        1
                    };
                    tset(
                        grid,
                        cx + side * trunk_width,
                        y,
                        if side > 0 { '╶' } else { '╴' },
                        bark,
                    );
                }
                path.push((y, cx));
            }
        }
    }

    // Flare at base: widen the bottom 2 rows
    let flare_rows = 2i32.min(bot_y - top_y);
    for dy in 0..flare_rows {
        let y = bot_y - 1 - dy;
        let fw = (flare_rows - dy) as i32;
        let bx = path
            .iter()
            .find(|&&(py, _)| py == y)
            .map(|&(_, px)| px)
            .unwrap_or(start_x);
        tset_over(grid, bx - fw, y, '╱', bark);
        tset_over(grid, bx + fw, y, '╲', bark);
    }

    path
}
/// Spiral / Fibonacci tree.
/// Main trunk runs the full height. Branches peel off alternating sides,
/// each shorter than the last. Secondary twigs curl upward off the tips.
pub fn grow_spiral_tree(
    grid: &mut Grid,
    root_x: usize,
    root_y: usize,
    canopy_y: usize,
    spread: usize,
    color: Color,
    rng: &mut StdRng,
) {
    if canopy_y >= root_y {
        return;
    }
    let height = root_y - canopy_y;
    let rx = root_x as i32;

    // Trunk
    for y in canopy_y..root_y {
        tset_over(grid, rx, y as i32, '│', color);
    }
    tset_over(grid, rx, canopy_y as i32, '╷', lighten(color, 50));

    let interval = (height / 5).max(2);
    let mut left = rng.random_range(0..2u32) == 0;
    let mut level = 0usize;
    let mut y = (canopy_y + interval) as i32;

    while y < root_y as i32 - 1 {
        let arm = (spread.saturating_sub(level * 2)).max(2) as i32;
        let c = lighten(color, 60u8.saturating_sub((level * 15) as u8));

        if left {
            tset_over(grid, rx, y, '┤', c);
            for i in 1..arm {
                tset(grid, rx - i, y, '─', c);
            }
            tset(grid, rx - arm, y, '╴', c);
            if level < 3 {
                tset(grid, rx - arm, y - 1, '╮', c);
                tset(grid, rx - arm - 1, y - 1, '╷', lighten(c, 25));
            }
        } else {
            tset_over(grid, rx, y, '├', c);
            for i in 1..arm {
                tset(grid, rx + i, y, '─', c);
            }
            tset(grid, rx + arm, y, '╶', c);
            if level < 3 {
                tset(grid, rx + arm, y - 1, '╭', c);
                tset(grid, rx + arm + 1, y - 1, '╷', lighten(c, 25));
            }
        }

        left = !left;
        y += interval as i32;
        level += 1;
    }
}
/// Candelabra tree.
/// Short thick trunk splits into 3-5 near-vertical arms that each branch once at the top.
pub fn grow_candelabra(
    grid: &mut Grid,
    root_x: usize,
    root_y: usize,
    canopy_y: usize,
    spread: usize,
    color: Color,
    rng: &mut StdRng,
) {
    if canopy_y >= root_y {
        return;
    }
    let height = root_y - canopy_y;
    let rx = root_x as i32;
    let arm_count = rng.random_range(3..6usize);
    let split_y = (root_y - height / 3) as i32;

    // Main trunk to split point
    for y in split_y..root_y as i32 {
        tset_over(grid, rx, y, '│', color);
    }

    // Arm x-positions spread evenly
    let total_spread = spread as i32 * 2;
    let step = total_spread / (arm_count as i32 - 1).max(1);
    let start_x = rx - total_spread / 2;

    // Horizontal connector at split
    for x in start_x..=start_x + total_spread {
        tset_over(grid, x, split_y, '─', darken(color, 10));
    }
    tset_over(grid, rx, split_y, '┬', color);

    for i in 0..arm_count {
        let ax = start_x + i as i32 * step;
        let jc = if i == 0 {
            '└'
        } else if i == arm_count - 1 {
            '┘'
        } else {
            '┴'
        };
        tset_over(grid, ax, split_y, jc, color);

        // Each arm goes straight up with a small tilt
        let lean: i32 = if ax < rx {
            -1
        } else if ax > rx {
            1
        } else {
            0
        };
        let arm_top = canopy_y as i32 + rng.random_range(0..3u32) as i32;
        let arm_color = lighten(color, 20);

        let mut cx = ax;
        for y in (arm_top..split_y).rev() {
            tset(grid, cx, y, '│', arm_color);
            // Lean once near the middle
            if y == (arm_top + split_y) / 2 && lean != 0 {
                tset_over(grid, cx, y, if lean < 0 { '╲' } else { '╱' }, arm_color);
                cx += lean;
            }
        }

        // Two-way tip split
        let tip_c = lighten(arm_color, 30);
        tset_over(grid, cx, arm_top, '┤', tip_c);
        tset(grid, cx - 1, arm_top, '─', tip_c);
        tset(grid, cx - 2, arm_top, '╷', tip_c);
        tset_over(grid, cx, arm_top, '├', tip_c);
        tset(grid, cx + 1, arm_top, '─', tip_c);
        tset(grid, cx + 2, arm_top, '╷', tip_c);
    }
}
/// Birch tree.
/// Tall, thin trunk. Very short branches peeling off frequently. Spray tips.
pub fn grow_birch(
    grid: &mut Grid,
    root_x: usize,
    root_y: usize,
    canopy_y: usize,
    spread: usize,
    color: Color,
    rng: &mut StdRng,
) {
    if canopy_y >= root_y {
        return;
    }
    let height = root_y - canopy_y;
    let rx = root_x as i32;

    for y in canopy_y..root_y {
        tset_over(grid, rx, y as i32, '│', color);
    }

    let interval = 2i32;
    let mut left = true;
    let mut y = canopy_y as i32 + 1;

    while y < root_y as i32 - 1 {
        // Skip some for density variation
        if rng.random_range(0..4u32) == 0 {
            y += interval;
            left = !left;
            continue;
        }

        let arm = (rng.random_range(2..=spread.max(2).min(6)) as i32).max(1);
        let c = lighten(color, rng.random_range(10..50) as u8);

        if left {
            tset_over(grid, rx, y, '┤', c);
            for i in 1..arm {
                tset(grid, rx - i, y, '─', c);
            }
            // spray tip: two short diagonals
            tset(grid, rx - arm, y, '╮', c);
            tset(grid, rx - arm - 1, y - 1, '╷', lighten(c, 20));
            if arm > 2 {
                tset(grid, rx - arm + 1, y - 1, '╷', lighten(c, 10));
            }
        } else {
            tset_over(grid, rx, y, '├', c);
            for i in 1..arm {
                tset(grid, rx + i, y, '─', c);
            }
            tset(grid, rx + arm, y, '╭', c);
            tset(grid, rx + arm + 1, y - 1, '╷', lighten(c, 20));
            if arm > 2 {
                tset(grid, rx + arm - 1, y - 1, '╷', lighten(c, 10));
            }
        }

        left = !left;
        y += interval;
    }

    tset_over(grid, rx, canopy_y as i32, '╷', lighten(color, 60));
}
/// Storm-leaning tree.
/// Trunk drawn with diagonal chars, leaning to one side. Branches on windward side.
pub fn grow_storm_tree(
    grid: &mut Grid,
    root_x: usize,
    root_y: usize,
    canopy_y: usize,
    spread: usize,
    color: Color,
    rng: &mut StdRng,
) {
    if canopy_y >= root_y {
        return;
    }
    let height = root_y - canopy_y;
    let lean: i32 = if rng.random_range(0..2u32) == 0 {
        1
    } else {
        -1
    };
    let lean_every = (height / (spread.min(8))).max(2) as i32;

    // Draw leaning trunk
    let mut cx = root_x as i32;
    let mut shifts = 0i32;
    for y in (canopy_y..root_y).rev() {
        let iy = y as i32;
        let rows_from_root = root_y as i32 - iy;
        let new_shifts = rows_from_root / lean_every;
        let ch = if new_shifts > shifts {
            shifts = new_shifts;
            cx += lean;
            if lean > 0 { '╱' } else { '╲' }
        } else {
            '│'
        };
        tset_over(grid, cx, iy, ch, color);
    }

    // Branches peel off the windward side (opposite to lean)
    let branch_side = -lean;
    let interval = (height / 4).max(2) as i32;
    let tip_x = cx; // where trunk ended up at canopy

    let mut bx = root_x as i32;
    let mut bshifts = 0i32;
    let mut by = root_y as i32 - 2;
    let mut level = 0;

    while by > canopy_y as i32 + 2 {
        let arm = (spread.saturating_sub(level * 2)).max(2) as i32;
        let c = lighten(color, (level * 20) as u8);

        // Find trunk x at this y
        let rows_from_root = root_y as i32 - by;
        let tx = root_x as i32 + lean * (rows_from_root / lean_every);

        let jc = if branch_side < 0 { '┤' } else { '├' };
        tset_over(grid, tx, by, jc, c);

        for i in 1..=arm {
            tset(grid, tx + branch_side * i, by, '─', c);
        }
        // Tip curls up
        let tip = tx + branch_side * arm;
        let curl = if branch_side < 0 { '╮' } else { '╭' };
        tset(grid, tip, by, curl, c);
        tset(grid, tip + branch_side, by - 1, '╷', lighten(c, 25));

        by -= interval;
        level += 1;
    }

    tset_over(grid, tip_x, canopy_y as i32, '╷', lighten(color, 55));
}
/// Wide spreading tree.
/// Lower splits are very wide, upper ones narrow. Broad silhouette.
pub fn grow_wide_tree(
    grid: &mut Grid,
    root_x: usize,
    root_y: usize,
    canopy_y: usize,
    spread: usize,
    color: Color,
    _rng: &mut StdRng,
) {
    if canopy_y >= root_y {
        return;
    }
    let height = root_y - canopy_y;
    let rx = root_x as i32;
    let first_split = (root_y - height / 4) as i32;

    for y in first_split..root_y as i32 {
        tset_over(grid, rx, y, '│', color);
    }

    // 3 levels: base (very wide), mid, top (narrow)
    let levels: &[(i32, usize)] = &[
        (first_split, spread * 2),
        (canopy_y as i32 + height as i32 * 2 / 3, spread),
        (canopy_y as i32 + height as i32 / 3, spread / 2),
    ];

    let mut queue: Vec<(i32, i32, usize, usize)> = Vec::new();

    for (li, &(sy, arm)) in levels.iter().enumerate() {
        let c = lighten(color, (li * 20) as u8);
        let arm = arm as i32;
        let lx = rx - arm;
        let rx2 = rx + arm;

        tset_over(grid, rx, sy, '┼', c);
        for x in lx..rx {
            tset(grid, x, sy, '─', c);
        }
        for x in rx + 1..=rx2 {
            tset(grid, x, sy, '─', c);
        }
        tset(grid, lx, sy, '╭', c);
        tset(grid, rx2, sy, '╮', c);

        let next_sy = if li + 1 < levels.len() {
            levels[li + 1].0
        } else {
            canopy_y as i32
        };

        // left and right sub-trunks
        for y in next_sy..sy {
            tset(grid, lx, y, '│', c);
        }
        for y in next_sy..sy {
            tset(grid, rx2, y, '│', c);
        }

        if li + 1 >= levels.len() {
            tset(grid, lx, canopy_y as i32, '╷', lighten(c, 30));
            tset(grid, rx2, canopy_y as i32, '╷', lighten(c, 30));
        }
    }
}
/// Asymmetric tree.
/// Left and right arms are deliberately different lengths. Wind-blown feel.
pub fn grow_asymmetric_tree(
    grid: &mut Grid,
    root_x: usize,
    root_y: usize,
    canopy_y: usize,
    spread: usize,
    color: Color,
    rng: &mut StdRng,
) {
    if canopy_y >= root_y {
        return;
    }
    let height = root_y - canopy_y;
    let rx = root_x as i32;
    let first_split = root_y.saturating_sub((height / 3).max(2));

    for y in first_split..root_y {
        tset_over(grid, rx, y as i32, '│', color);
    }

    // One side is 40-70% longer than the other
    let heavy_left = rng.random_range(0..2u32) == 0;
    let base_spread = spread as i32;
    let (left_spread, right_spread) = if heavy_left {
        (base_spread * 5 / 3, base_spread * 2 / 3)
    } else {
        (base_spread * 2 / 3, base_spread * 5 / 3)
    };

    // Recursive asymmetric split -- left side, right side have different max depths
    let left_depth = if heavy_left { 4 } else { 2 };
    let right_depth = if heavy_left { 2 } else { 4 };

    let mut queue: Vec<(i32, i32, i32, usize, usize)> = vec![
        (
            rx - left_spread,
            canopy_y as i32,
            first_split as i32,
            0,
            left_depth,
        ),
        (
            rx + right_spread,
            canopy_y as i32,
            first_split as i32,
            0,
            right_depth,
        ),
    ];

    // junction at first split
    let c0 = color;
    tset_over(grid, rx, first_split as i32, '┼', c0);
    for x in rx - left_spread..rx {
        tset(grid, x, first_split as i32, '─', c0);
    }
    for x in rx + 1..=rx + right_spread {
        tset(grid, x, first_split as i32, '─', c0);
    }
    tset(grid, rx - left_spread, first_split as i32, '╭', c0);
    tset(grid, rx + right_spread, first_split as i32, '╮', c0);

    while let Some((x, top, bottom, depth, max_d)) = queue.pop() {
        let c = lighten(color, (depth * 18) as u8);

        for y in top + 1..bottom {
            tset(grid, x, y, '│', c);
        }

        if depth >= max_d || bottom - top <= 2 {
            tset(grid, x, top, '╷', lighten(c, 30));
            continue;
        }

        let split_y = top + (bottom - top) * 2 / 5; // off-center split
        let arm = ((base_spread >> (depth + 1)) as i32).max(1);

        tset_over(grid, x, split_y, '┼', c);
        for ax in x - arm..x {
            tset(grid, ax, split_y, '─', c);
        }
        for ax in x + 1..=x + arm {
            tset(grid, ax, split_y, '─', c);
        }
        tset(grid, x - arm, split_y, '╭', c);
        tset(grid, x + arm, split_y, '╮', c);

        queue.push((x - arm, top, split_y, depth + 1, max_d));
        queue.push((x + arm, top, split_y, depth + 1, max_d));
    }
}
/// Tall narrow tree.
/// Very little horizontal spread. Many levels of short branches. Columnar.
pub fn grow_tall_narrow(
    grid: &mut Grid,
    root_x: usize,
    root_y: usize,
    canopy_y: usize,
    _spread: usize,
    color: Color,
    _rng: &mut StdRng,
) {
    if canopy_y >= root_y {
        return;
    }
    let height = root_y - canopy_y;
    let rx = root_x as i32;

    for y in canopy_y..root_y {
        tset_over(grid, rx, y as i32, '│', color);
    }

    let mut queue: Vec<(i32, i32, i32, usize)> = vec![(rx, canopy_y as i32, root_y as i32, 0)];
    let max_depth = 5;

    while let Some((x, top, bottom, depth)) = queue.pop() {
        if depth >= max_depth || bottom - top < 2 {
            tset(grid, x, top, '╷', lighten(color, 60));
            continue;
        }
        let c = lighten(color, (depth * 15) as u8);
        let arm = (3i32 - depth as i32).max(1); // 3, 2, 1, 1, 1
        let split_y = top + (bottom - top) / 2;

        tset_over(grid, x, split_y, '┤', c);
        tset(grid, x - 1, split_y, '─', c);
        tset(grid, x - arm, split_y, '╭', c);
        for ax in x - arm + 1..x - 1 {
            tset(grid, ax, split_y, '─', c);
        }
        tset_over(grid, x, split_y, '├', c);
        tset(grid, x + 1, split_y, '─', c);
        tset(grid, x + arm, split_y, '╮', c);
        for ax in x + 2..x + arm {
            tset(grid, ax, split_y, '─', c);
        }

        queue.push((x - arm, top, split_y, depth + 1));
        queue.push((x + arm, top, split_y, depth + 1));
        // Continue center upward
        for y in top + 1..split_y {
            tset(grid, x, y, '│', c);
        }
    }
}
/// Dead / skeletal tree.
/// Sparse angular branches. Uses diagonal chars and sharp tips. Eerie.
pub fn grow_dead_tree(
    grid: &mut Grid,
    root_x: usize,
    root_y: usize,
    canopy_y: usize,
    spread: usize,
    color: Color,
    rng: &mut StdRng,
) {
    if canopy_y >= root_y {
        return;
    }
    let height = root_y - canopy_y;
    let rx = root_x as i32;

    // Gnarled trunk: mostly vertical but with occasional diagonal offsets
    let mut cx = rx;
    for y in (canopy_y..root_y).rev() {
        let iy = y as i32;
        let from_root = root_y as i32 - iy;
        let ch = if from_root > 2 && from_root % 7 == 0 && rng.random_range(0..3u32) == 0 {
            let lean = if rng.random_range(0..2u32) == 0 {
                -1i32
            } else {
                1
            };
            cx += lean;
            if lean > 0 { '╱' } else { '╲' }
        } else {
            '│'
        };
        tset_over(grid, cx, iy, ch, darken(color, 10));
    }

    // Sparse angular branches radiating outward
    let branch_count = rng.random_range(4..8usize);
    let interval = (height / branch_count).max(2) as i32;
    let mut by = canopy_y as i32 + interval;
    let tip_chars = ['╴', '╶', '·', '╷'];

    let mut trunk_cx = cx;
    for b in 0..branch_count {
        if by >= root_y as i32 - 1 {
            break;
        }
        let c = lighten(color, (b * 12) as u8);
        let arm = rng.random_range(2..=(spread.max(2).min(8))) as i32;

        // Recompute trunk x at this y
        let from_root = root_y as i32 - by;
        // approximate: walk forward
        let tx = rx; // simplified

        let go_left = b % 2 == 0;
        let diag_ch = if go_left { '╲' } else { '╱' };
        let horiz_ch = '─';

        // Diagonal first, then horizontal
        let diag_len = (arm / 3).max(1);
        let horiz_len = arm - diag_len;
        let dir: i32 = if go_left { -1 } else { 1 };

        let mut bx = tx;
        let mut yy = by;
        tset_over(grid, bx, yy, if go_left { '┐' } else { '┌' }, c);
        for _ in 0..diag_len {
            bx += dir;
            yy -= 1;
            tset(grid, bx, yy, diag_ch, c);
        }
        for _ in 0..horiz_len {
            bx += dir;
            tset(grid, bx, yy, horiz_ch, c);
        }
        let tip = tip_chars[b % tip_chars.len()];
        tset(grid, bx + dir, yy, tip, lighten(c, 20));
        // occasional sub-twig
        if arm > 3 {
            tset(grid, bx, yy - 1, '╷', lighten(c, 30));
        }

        by += interval;
    }

    tset_over(grid, cx, canopy_y as i32, '╷', lighten(color, 70));
}
/// Drooping tree.
/// Branches arc outward and curve downward with rounded corners. Elegant droop.
pub fn grow_drooping_tree(
    grid: &mut Grid,
    root_x: usize,
    root_y: usize,
    canopy_y: usize,
    spread: usize,
    color: Color,
    rng: &mut StdRng,
) {
    if canopy_y >= root_y {
        return;
    }
    let height = root_y - canopy_y;
    let rx = root_x as i32;
    let first_split = (root_y - height / 3) as i32;

    for y in first_split..root_y as i32 {
        tset_over(grid, rx, y, '│', color);
    }

    let arm_count = rng.random_range(3..6usize);
    let c0 = lighten(color, 10);

    // Fan of branches arcing upward then drooping
    for i in 0..arm_count {
        let t = i as f32 / (arm_count - 1) as f32; // 0..1
        let arm_x_offset = ((t * 2.0 - 1.0) * spread as f32) as i32;
        let arm_top_y = canopy_y as i32 + rng.random_range(0..4u32) as i32;
        let c = lighten(color, (i * 15) as u8);

        let bx = rx + arm_x_offset;

        // Curved arc from (rx, first_split) to (bx, arm_top_y)
        // Draw a simple L-shaped arc: horizontal then vertical
        let mid_y = first_split - (height / 4) as i32;

        // Horizontal segment from trunk to arm x
        if arm_x_offset != 0 {
            let (x0, x1) = if arm_x_offset < 0 { (bx, rx) } else { (rx, bx) };
            for x in x0..=x1 {
                tset(grid, x, first_split, '─', c0);
            }
            let corner = if arm_x_offset < 0 { '╭' } else { '╮' };
            tset(grid, bx, first_split, corner, c0);
            tset_over(grid, rx, first_split, '┼', c0);
        } else {
            tset_over(grid, rx, first_split, '│', c0);
        }

        // Vertical rise from mid to top
        for y in arm_top_y..first_split {
            tset(grid, bx, y, '│', c);
        }

        // Droop: horizontal arms hanging off the top segment
        let droop_arm = (spread / 3).max(1) as i32;
        if arm_top_y + 2 < first_split {
            let droop_y = arm_top_y + 1;
            let dc = lighten(c, 20);
            for dx in 1..=droop_arm {
                tset(grid, bx - dx, droop_y, '─', dc);
                tset(grid, bx + dx, droop_y, '─', dc);
            }
            tset(grid, bx - droop_arm, droop_y, '╮', dc);
            tset(grid, bx + droop_arm, droop_y, '╭', dc);
            tset_over(grid, bx, droop_y, '┬', dc);
            // Hanging drips
            for d in 1..=3 {
                let dc2 = lighten(dc, (d * 15) as u8);
                tset(grid, bx - droop_arm, droop_y + d, '╎', dc2);
                tset(grid, bx + droop_arm, droop_y + d, '╎', dc2);
            }
        }

        tset(grid, bx, arm_top_y, '╷', lighten(c, 40));
    }
}
/// Kaiju tree: massive multi-trunk ancient tree with unbalanced branching.
/// 2-3 trunks diverge from a thick base, each leaning at different angles.
/// Branches at irregular intervals with unequal arm lengths. Dominates the scene.
pub fn grow_kaiju_tree(
    grid: &mut Grid,
    root_x: usize,
    root_y: usize,
    canopy_y: usize,
    spread: usize,
    color: Color,
    rng: &mut StdRng,
) {
    if canopy_y + 4 >= root_y {
        return;
    }
    let height = root_y - canopy_y;
    let rx = root_x as i32;

    // Thick base: 3-wide trunk for bottom third
    let base_top = root_y.saturating_sub(height / 3) as i32;
    for y in base_top..root_y as i32 {
        tset_over(grid, rx, y, '┃', color);
        tset_over(grid, rx - 1, y, '│', darken(color, 15));
        tset_over(grid, rx + 1, y, '│', darken(color, 15));
    }

    // 2-3 trunks diverge from base_top
    let trunk_count = rng.random_range(2..4u32) as usize;
    let total_spread = spread as i32 * 2;

    struct Trunk {
        x: i32,
        lean: i32,
        branch_side: i32,
        depth: usize,
    }
    let mut trunks: Vec<Trunk> = Vec::new();

    for i in 0..trunk_count {
        let frac = i as f32 / (trunk_count - 1).max(1) as f32;
        let target_x = rx - total_spread / 2 + (frac * total_spread as f32) as i32;
        let lean = if target_x < rx {
            -1
        } else if target_x > rx {
            1
        } else {
            0
        };
        let branch_side = if rng.random_range(0..2u32) == 0 {
            -1
        } else {
            1
        };
        let depth = rng.random_range(3..6u32) as usize;
        trunks.push(Trunk {
            x: target_x,
            lean,
            branch_side,
            depth,
        });
    }

    // Fork connector at base_top
    let c0 = lighten(color, 10);
    let leftmost = trunks.iter().map(|t| t.x).min().unwrap_or(rx);
    let rightmost = trunks.iter().map(|t| t.x).max().unwrap_or(rx);
    for x in leftmost..=rightmost {
        tset_over(grid, x, base_top, '─', c0);
    }
    tset_over(grid, rx, base_top, '┬', c0);

    // Draw each trunk with its own lean and branches
    for trunk in &trunks {
        let lean_every = (height as i32 / 5).max(3);
        let mut cx = trunk.x;
        let trunk_top = canopy_y as i32 + rng.random_range(0..4u32) as i32;

        // Draw the leaning trunk
        for y in (trunk_top..base_top).rev() {
            let rows_up = base_top - y;
            let should_lean = trunk.lean != 0 && rows_up > 0 && rows_up % lean_every == 0;
            let ch = if should_lean {
                cx += trunk.lean;
                if trunk.lean > 0 { '╱' } else { '╲' }
            } else {
                '│'
            };
            let c = lighten(color, ((base_top - y) as u8).min(40));
            tset_over(grid, cx, y, ch, c);
        }
        tset_over(grid, cx, trunk_top, '╷', lighten(color, 60));

        // Branches at irregular intervals, unequal arm lengths
        let branch_count = rng.random_range(3..7u32) as usize;
        let trunk_height = (base_top - trunk_top) as usize;
        let base_interval = (trunk_height / (branch_count + 1)).max(2);

        for b in 0..branch_count {
            let jitter = rng.random_range(0..3u32) as i32 - 1;
            let by = trunk_top + (base_interval * (b + 1)) as i32 + jitter;
            if by >= base_top || by <= trunk_top {
                continue;
            }

            // Find trunk x at this y
            let rows_up = base_top - by;
            let tx = trunk.x + trunk.lean * (rows_up / lean_every);

            // Unequal arms: one side 1.5-3x the other
            let base_arm = (spread / 3).max(2) as i32 - (b as i32 / 2);
            let base_arm = base_arm.max(1);
            let long_factor = rng.random_range(15..30u32) as i32;
            let short_factor = rng.random_range(5..12u32) as i32;

            let (left_arm, right_arm) = if trunk.branch_side < 0 {
                (base_arm * long_factor / 10, base_arm * short_factor / 10)
            } else {
                (base_arm * short_factor / 10, base_arm * long_factor / 10)
            };

            let c = lighten(color, (b * 12 + 15) as u8);

            // Left arm
            if left_arm > 0 {
                for i in 1..=left_arm {
                    tset(grid, tx - i, by, '─', c);
                }
                tset(grid, tx - left_arm, by, '╮', c);
                tset(grid, tx - left_arm - 1, by - 1, '╷', lighten(c, 25));
                // Sub-twig on longer arms
                if left_arm > 3 && rng.random_range(0..3u32) != 0 {
                    let sub_x = tx - left_arm * 2 / 3;
                    tset(grid, sub_x, by - 1, '╷', lighten(c, 20));
                    tset(grid, sub_x, by - 2, '╷', lighten(c, 35));
                }
            }
            // Right arm
            if right_arm > 0 {
                for i in 1..=right_arm {
                    tset(grid, tx + i, by, '─', c);
                }
                tset(grid, tx + right_arm, by, '╭', c);
                tset(grid, tx + right_arm + 1, by - 1, '╷', lighten(c, 25));
                if right_arm > 3 && rng.random_range(0..3u32) != 0 {
                    let sub_x = tx + right_arm * 2 / 3;
                    tset(grid, sub_x, by - 1, '╷', lighten(c, 20));
                    tset(grid, sub_x, by - 2, '╷', lighten(c, 35));
                }
            }

            let jc = if left_arm > 0 && right_arm > 0 {
                '┼'
            } else if left_arm > 0 {
                '┤'
            } else {
                '├'
            };
            tset_over(grid, tx, by, jc, c);
        }
    }
}
/// Dispatch all tree variants by kind index (0..18).
pub fn draw_tree(
    grid: &mut Grid,
    root_x: usize,
    root_y: usize,
    canopy_y: usize,
    spread: usize,
    kind: usize,
    color: Color,
    rng: &mut StdRng,
) {
    match kind % 19 {
        0 => grow_tree(grid, root_x, root_y, canopy_y, spread, color, rng),
        1 => draw_pine(grid, root_x, root_y, 3, (spread * 2).min(12), color),
        2 => draw_willow(grid, root_x, root_y, canopy_y, spread, color),
        3 => draw_palm(
            grid,
            root_x,
            root_y,
            root_y.saturating_sub(canopy_y).saturating_sub(4),
            color,
            rng,
        ),
        4 => grow_spiral_tree(grid, root_x, root_y, canopy_y, spread, color, rng),
        5 => grow_candelabra(grid, root_x, root_y, canopy_y, spread, color, rng),
        6 => grow_birch(grid, root_x, root_y, canopy_y, spread, color, rng),
        7 => grow_storm_tree(grid, root_x, root_y, canopy_y, spread, color, rng),
        8 => grow_wide_tree(grid, root_x, root_y, canopy_y, spread, color, rng),
        9 => grow_asymmetric_tree(grid, root_x, root_y, canopy_y, spread, color, rng),
        10 => grow_tall_narrow(grid, root_x, root_y, canopy_y, spread, color, rng),
        11 => grow_drooping_tree(grid, root_x, root_y, canopy_y, spread, color, rng),
        12 => grow_dead_tree(grid, root_x, root_y, canopy_y, spread, color, rng),
        13 => grow_kaiju_tree(grid, root_x, root_y, canopy_y, spread, color, rng),
        14 => grow_wild_tree(grid, root_x, root_y, canopy_y, spread, color, rng),
        15 => grow_zigzag_tree(grid, root_x, root_y, canopy_y, spread, color, rng),
        16 => grow_braille_tree(grid, root_x, root_y, canopy_y, spread, color, rng),
        17 => grow_tendril_tree(grid, root_x, root_y, canopy_y, spread, color, rng),
        _ => grow_connected_tree(grid, root_x, root_y, canopy_y, spread, color, rng),
    }
}

/// Where branches concentrate along the trunk.
#[derive(Clone, Copy)]
pub enum BranchZone {
    TopHeavy,
    BottomHeavy,
    MidBand,
    Uniform,
}
/// How branch tips end.
#[derive(Clone, Copy)]
pub enum TipCurl {
    Up,
    Down,
    DiagOut,
    SCurve,
    Straight,
}
/// Recipe that fully parameterizes a path-aware tree's personality.
/// All fields can be overridden via CLI args for deterministic control.
#[derive(Clone)]
pub struct TreeRecipe {
    pub trunk_wobble_freq: u32, // how often trunk wobbles (higher = less frequent)
    pub trunk_wobble_prob: u32, // 0-100 chance of wobbling at each freq tick
    pub trunk_lean: i32,        // -1 left, 0 straight, 1 right
    pub branch_count: (u32, u32), // (min, max) branch count range
    pub branch_zone: BranchZone,
    pub left_bias: f32, // 0.0 = all right, 1.0 = all left, 0.5 = balanced
    pub arm_length_min: u32,
    pub arm_length_max: u32,
    pub tip_curls: &'static [TipCurl], // pool of tip styles to pick from
    pub sub_branch_prob: u32,          // 0-100 chance of sub-branching
    pub max_depth: usize,              // recursive sub-branch depth
    pub flare_width: u32,              // base flare (0 = none, 1-3 = wider)
}
impl TreeRecipe {
    /// Balanced branching tree (kind 0 equivalent).
    pub fn balanced() -> Self {
        TreeRecipe {
            trunk_wobble_freq: 5,
            trunk_wobble_prob: 20,
            trunk_lean: 0,
            branch_count: (4, 7),
            branch_zone: BranchZone::Uniform,
            left_bias: 0.5,
            arm_length_min: 2,
            arm_length_max: 8,
            tip_curls: &[TipCurl::Up, TipCurl::DiagOut, TipCurl::Straight],
            sub_branch_prob: 30,
            max_depth: 2,
            flare_width: 1,
        }
    }
    /// Wild / asymmetric (kind 14 equivalent).
    pub fn wild() -> Self {
        TreeRecipe {
            trunk_wobble_freq: 3,
            trunk_wobble_prob: 40,
            trunk_lean: 0,
            branch_count: (3, 9),
            branch_zone: BranchZone::TopHeavy,
            left_bias: 0.3,
            arm_length_min: 1,
            arm_length_max: 12,
            tip_curls: &[
                TipCurl::Up,
                TipCurl::Down,
                TipCurl::DiagOut,
                TipCurl::SCurve,
                TipCurl::Straight,
            ],
            sub_branch_prob: 50,
            max_depth: 3,
            flare_width: 0,
        }
    }
    /// Storm-leaning (kind 7 equivalent).
    pub fn storm(lean: i32) -> Self {
        TreeRecipe {
            trunk_wobble_freq: 4,
            trunk_wobble_prob: 0,
            trunk_lean: lean,
            branch_count: (3, 6),
            branch_zone: BranchZone::Uniform,
            left_bias: if lean < 0 { 0.8 } else { 0.2 }, // branches opposite to lean
            arm_length_min: 3,
            arm_length_max: 10,
            tip_curls: &[TipCurl::Up, TipCurl::DiagOut],
            sub_branch_prob: 20,
            max_depth: 1,
            flare_width: 1,
        }
    }
    /// Drooping / weeping (kind 11 equivalent).
    pub fn weeping() -> Self {
        TreeRecipe {
            trunk_wobble_freq: 6,
            trunk_wobble_prob: 15,
            trunk_lean: 0,
            branch_count: (5, 8),
            branch_zone: BranchZone::TopHeavy,
            left_bias: 0.5,
            arm_length_min: 3,
            arm_length_max: 10,
            tip_curls: &[TipCurl::Down, TipCurl::Down, TipCurl::Straight],
            sub_branch_prob: 40,
            max_depth: 2,
            flare_width: 2,
        }
    }
    /// Sparse / dead (kind 12 equivalent).
    pub fn dead() -> Self {
        TreeRecipe {
            trunk_wobble_freq: 4,
            trunk_wobble_prob: 30,
            trunk_lean: 0,
            branch_count: (2, 4),
            branch_zone: BranchZone::Uniform,
            left_bias: 0.5,
            arm_length_min: 2,
            arm_length_max: 6,
            tip_curls: &[TipCurl::Straight, TipCurl::Straight, TipCurl::DiagOut],
            sub_branch_prob: 10,
            max_depth: 1,
            flare_width: 0,
        }
    }
    /// Tall narrow columnar (kind 10 equivalent).
    pub fn columnar() -> Self {
        TreeRecipe {
            trunk_wobble_freq: 8,
            trunk_wobble_prob: 10,
            trunk_lean: 0,
            branch_count: (6, 10),
            branch_zone: BranchZone::Uniform,
            left_bias: 0.5,
            arm_length_min: 1,
            arm_length_max: 3,
            tip_curls: &[TipCurl::Up, TipCurl::Straight],
            sub_branch_prob: 5,
            max_depth: 1,
            flare_width: 0,
        }
    }
    /// Gnarled sprawler with thick base.
    pub fn gnarled() -> Self {
        TreeRecipe {
            trunk_wobble_freq: 2,
            trunk_wobble_prob: 50,
            trunk_lean: 0,
            branch_count: (3, 6),
            branch_zone: BranchZone::BottomHeavy,
            left_bias: 0.5,
            arm_length_min: 3,
            arm_length_max: 14,
            tip_curls: &[TipCurl::SCurve, TipCurl::DiagOut, TipCurl::Down],
            sub_branch_prob: 60,
            max_depth: 3,
            flare_width: 3,
        }
    }
}
/// Grow a tree using TreePen with the given recipe. Always path-connected.
pub fn grow_pen_tree(
    grid: &mut Grid,
    root_x: usize,
    root_y: usize,
    canopy_y: usize,
    spread: usize,
    color: Color,
    recipe: &TreeRecipe,
    rng: &mut StdRng,
) {
    use MoveDir::*;
    if canopy_y + 3 >= root_y {
        return;
    }
    let height = (root_y - canopy_y) as i32;

    // Trunk
    let mut trunk = TreePen::new(root_x as i32, root_y as i32, color);
    tset_over(grid, trunk.x, trunk.y, '│', color);

    let trunk_len = (height * 2 / 3).max(3);
    let mut trunk_path: Vec<(i32, i32)> = vec![(trunk.x, trunk.y)];

    // Base flare
    if recipe.flare_width > 0 {
        let fw = recipe.flare_width as i32;
        let bark = darken(color, 15);
        for dy in 0..fw.min(2) {
            let y = root_y as i32 + dy; // below root into ground
            tset_over(grid, root_x as i32 - fw + dy, y, '╱', bark);
            tset_over(grid, root_x as i32 + fw - dy, y, '╲', bark);
            tset_over(grid, root_x as i32, y, '┃', color);
        }
    }

    for i in 0..trunk_len {
        let dir = if recipe.trunk_lean != 0 && i > 0 && i % recipe.trunk_wobble_freq as i32 == 0 {
            if recipe.trunk_lean < 0 {
                UpLeft
            } else {
                UpRight
            }
        } else if i > 2
            && rng.random_range(0..100u32) < recipe.trunk_wobble_prob
            && i % recipe.trunk_wobble_freq as i32 == 0
        {
            if rng.random_range(0..2u32) == 0 {
                UpLeft
            } else {
                UpRight
            }
        } else {
            Up
        };
        trunk.step(grid, dir);
        trunk_path.push((trunk.x, trunk.y));
    }

    // Branch points
    let branch_count = rng
        .random_range(recipe.branch_count.0..recipe.branch_count.1.max(recipe.branch_count.0 + 1))
        as usize;

    // Generate branch y positions based on zone
    let mut branch_positions: Vec<usize> = Vec::new();
    for _ in 0..branch_count {
        let t = match recipe.branch_zone {
            BranchZone::TopHeavy => {
                let t = rng.random::<f32>();
                t * t
            }
            BranchZone::BottomHeavy => {
                let t = rng.random::<f32>();
                1.0 - (1.0 - t) * (1.0 - t)
            }
            BranchZone::MidBand => 0.3 + rng.random::<f32>() * 0.4,
            BranchZone::Uniform => rng.random::<f32>(),
        };
        let idx = (t * (trunk_path.len() - 2) as f32) as usize + 1;
        branch_positions.push(idx.min(trunk_path.len() - 1));
    }
    branch_positions.sort();
    branch_positions.dedup();

    fn draw_branch(
        grid: &mut Grid,
        bx: i32,
        by: i32,
        go_left: bool,
        arm_len: i32,
        branch_color: Color,
        recipe: &TreeRecipe,
        depth: usize,
        rng: &mut StdRng,
    ) {
        let h_dir = if go_left { Left } else { Right };

        // Junction at attachment point
        let jc = if go_left { '┤' } else { '├' };
        tset_over(grid, bx, by, jc, branch_color);

        let mut pen = TreePen::new(bx + h_dir.dx(), by, branch_color);
        pen.last_dir = Some(h_dir);
        tset_over(grid, pen.x, pen.y, '─', branch_color);

        // Horizontal run
        let h_run = rng.random_range(1..(arm_len as u32).max(2));
        for _ in 0..h_run {
            pen.step(grid, h_dir);
        }

        // Sub-branch before tip
        if depth < recipe.max_depth
            && arm_len > 2
            && rng.random_range(0..100u32) < recipe.sub_branch_prob
        {
            let sub_color = lighten(branch_color, 15);
            let sub_arm = rng.random_range(1..(arm_len as u32 / 2 + 1).max(2)) as i32;
            // Fork upward from current position
            let fork_x = pen.x;
            let fork_y = pen.y;
            tset_over(grid, fork_x, fork_y, '┬', sub_color);
            draw_branch(
                grid,
                fork_x,
                fork_y - 1,
                go_left,
                sub_arm,
                sub_color,
                recipe,
                depth + 1,
                rng,
            );
        }

        // Tip curl
        let curl = recipe.tip_curls[rng.random_range(0..recipe.tip_curls.len() as u32) as usize];
        match curl {
            TipCurl::Up => {
                let n = rng.random_range(1..4u32);
                for _ in 0..n {
                    pen.step(grid, Up);
                }
                pen.tip(grid);
            }
            TipCurl::Down => {
                let n = rng.random_range(1..3u32);
                for _ in 0..n {
                    pen.step(grid, Down);
                }
                pen.tip(grid);
            }
            TipCurl::DiagOut => {
                let diag = if go_left { UpLeft } else { UpRight };
                let n = rng.random_range(1..4u32);
                for _ in 0..n {
                    pen.step(grid, diag);
                }
                pen.tip(grid);
            }
            TipCurl::SCurve => {
                pen.step(grid, Up);
                pen.step(grid, Up);
                pen.step(grid, h_dir);
                pen.tip(grid);
            }
            TipCurl::Straight => {
                pen.tip(grid);
            }
        }
    }

    for (bi, &path_idx) in branch_positions.iter().enumerate() {
        let (bx, by) = trunk_path[path_idx];
        let go_left = (rng.random::<f32>() > recipe.left_bias) ^ (bi % 2 == 0); // alternate with bias

        let t = path_idx as f32 / trunk_path.len() as f32;
        let max_arm = (spread as f32 * (1.0 - t * 0.4)).max(2.0) as i32;
        let arm_max = (max_arm as u32)
            .min(recipe.arm_length_max)
            .max(recipe.arm_length_min + 1);
        let arm_len = rng.random_range(recipe.arm_length_min..arm_max) as i32;

        let branch_color = lighten(color, (bi * 10 + 10) as u8);
        draw_branch(grid, bx, by, go_left, arm_len, branch_color, recipe, 0, rng);
    }

    trunk.tip(grid);
}
/// Convenience: grow a pen tree from a recipe preset index.
/// 0=balanced, 1=wild, 2=storm, 3=weeping, 4=dead, 5=columnar, 6=gnarled
pub fn grow_connected_tree(
    grid: &mut Grid,
    root_x: usize,
    root_y: usize,
    canopy_y: usize,
    spread: usize,
    color: Color,
    rng: &mut StdRng,
) {
    // Pick a random recipe for backward compat
    let recipe = match rng.random_range(0..7u32) {
        0 => TreeRecipe::balanced(),
        1 => TreeRecipe::wild(),
        2 => TreeRecipe::storm(if rng.random_range(0..2u32) == 0 {
            -1
        } else {
            1
        }),
        3 => TreeRecipe::weeping(),
        4 => TreeRecipe::dead(),
        5 => TreeRecipe::columnar(),
        _ => TreeRecipe::gnarled(),
    };
    grow_pen_tree(grid, root_x, root_y, canopy_y, spread, color, &recipe, rng);
}

/// Split-pen overwrite: trunk overwrites everything, branches overwrite
/// blanks + grass + lighter branches but not other tree trunks.
pub(crate) fn split_set(grid: &mut Grid, x: i32, y: i32, ch: char, fg: Color) {
    if x >= 0 && y >= 0 && (y as usize) < grid.len() && (x as usize) < grid[0].len() {
        let cell = &mut grid[y as usize][x as usize];
        let ok = matches!(
            cell.ch,
            ' ' | '·' | '∙' | '∿' | '~' | '░' | '▒' | '▓' | '╱' | '╲' | '╷'
        );
        if ok {
            *cell = Cell::new(ch, fg);
        }
    }
}
pub(crate) fn split_set_over(grid: &mut Grid, x: i32, y: i32, ch: char, fg: Color) {
    if x >= 0 && y >= 0 && (y as usize) < grid.len() && (x as usize) < grid[0].len() {
        grid[y as usize][x as usize] = Cell::new(ch, fg);
    }
}
/// Pen rewrite of grow_tree: binary recursive split.
/// Thick wobbling trunk rises from a wide base flare. At each split level,
/// horizontal arms fork left and right with independent lengths. Recursive
/// subdivision fills the canopy densely. Tips marked with ╷.
pub fn grow_split_pen(
    grid: &mut Grid,
    root_x: usize,
    root_y: usize,
    canopy_y: usize,
    spread: usize,
    color: Color,
    rng: &mut StdRng,
) {
    use MoveDir::*;
    if canopy_y + 3 >= root_y {
        return;
    }
    let height = root_y - canopy_y;
    let first_split = root_y.saturating_sub((height / 3).max(2));
    let rx = root_x as i32;
    let bark = darken(color, 15);

    // ── Trunk: thick, widening base, wobble ──
    // Split-tree trunk is its own thing: ┃ center with ╱╲ flare at base,
    // occasional knots (┼) and bark nubs (╴╶), narrows as it rises.
    // Base flare: 2 rows of widening
    for dy in 0..2i32 {
        let y = root_y as i32 - dy;
        let fw = 2 - dy;
        split_set_over(grid, rx, y, '┃', color);
        split_set_over(grid, rx - fw, y, '╱', bark);
        split_set_over(grid, rx + fw, y, '╲', bark);
        // Fill between flare and center
        for fx in (rx - fw + 1)..rx {
            split_set(grid, fx, y, '│', bark);
        }
        for fx in (rx + 1)..=(rx + fw - 1) {
            split_set(grid, fx, y, '│', bark);
        }
    }

    // Trunk body: wobble upward from below first_split
    let mut cx = rx;
    let wobble_freq = rng.random_range(3..6u32) as i32;
    for y in (first_split as i32..root_y as i32 - 2).rev() {
        let rows_up = root_y as i32 - 2 - y;
        let ch = if rows_up > 1 && rows_up % wobble_freq == 0 && rng.random_range(0..3u32) == 0 {
            let dir = rng.random_range(0..2u32) as i32 * 2 - 1;
            cx += dir;
            if dir > 0 { '╱' } else { '╲' }
        } else {
            '│'
        };
        split_set_over(grid, cx, y, ch, color);

        // Bark nubs for texture
        if rng.random_range(0..5u32) == 0 {
            let side = if rng.random_range(0..2u32) == 0 {
                -1i32
            } else {
                1
            };
            split_set(grid, cx + side, y, if side > 0 { '╶' } else { '╴' }, bark);
        }
    }

    // ── Canopy: recursive binary split via pen forks ──
    // Each level: vertical segment + horizontal arms + recurse on endpoints
    struct Split {
        x: i32,
        y: i32,
        top_y: i32,
        depth: usize,
    }

    let max_depth = 4usize;
    let mut queue: Vec<Split> = vec![Split {
        x: cx,
        y: first_split as i32,
        top_y: canopy_y as i32,
        depth: 0,
    }];

    while let Some(seg) = queue.pop() {
        let seg_height = (seg.y - seg.top_y).max(0);
        let bc = match seg.depth {
            0 => color,
            1 => lighten(color, 20),
            2 => lighten(color, 40),
            _ => lighten(color, 60),
        };

        // Leaf: short vertical stub + tip
        if seg.depth >= max_depth || seg_height <= 2 {
            for dy in 1..=seg_height.max(1) {
                split_set(grid, seg.x, seg.y - dy, '│', bc);
            }
            split_set(grid, seg.x, seg.y - seg_height.max(1), '╷', lighten(bc, 30));
            continue;
        }

        // Off-center split point: 30-70%
        let frac = 30 + rng.random_range(0..41u32);
        let sy = seg.top_y + (seg_height as u32 * frac / 100) as i32;
        let sy = sy.max(seg.top_y + 1).min(seg.y - 1);

        // Vertical segment from seg.y up to split
        for dy in 1..=(seg.y - sy).max(0) {
            split_set(grid, seg.x, seg.y - dy, '│', bc);
        }

        // Horizontal arms with independent lengths
        let base_arm = (spread >> seg.depth).max(2) as i32;
        let left_arm = (base_arm as u32 * rng.random_range(60..150u32) / 100).max(1) as i32;
        let right_arm = (base_arm as u32 * rng.random_range(60..150u32) / 100).max(1) as i32;

        // Junction at split point
        split_set_over(grid, seg.x, sy, '┼', bc);

        // Left arm: ╭───...
        let lx = seg.x - left_arm;
        split_set(grid, lx, sy, '╭', bc);
        for ax in (lx + 1)..seg.x {
            split_set(grid, ax, sy, '─', bc);
        }

        // Right arm: ...───╮
        let rrx = seg.x + right_arm;
        split_set(grid, rrx, sy, '╮', bc);
        for ax in (seg.x + 1)..rrx {
            split_set(grid, ax, sy, '─', bc);
        }

        queue.push(Split {
            x: lx,
            y: sy,
            top_y: seg.top_y,
            depth: seg.depth + 1,
        });
        queue.push(Split {
            x: rrx,
            y: sy,
            top_y: seg.top_y,
            depth: seg.depth + 1,
        });
    }
}
/// Spiral-pen overwrite: trunk always wins, branches yield to trunk chars
/// but overwrite grass and blanks.
pub(crate) fn spiral_set(grid: &mut Grid, x: i32, y: i32, ch: char, fg: Color) {
    if x >= 0 && y >= 0 && (y as usize) < grid.len() && (x as usize) < grid[0].len() {
        let cell = &mut grid[y as usize][x as usize];
        let ok = matches!(
            cell.ch,
            ' ' | '·' | '∙' | '∿' | '~' | '░' | '▒' | '▓' | '╱' | '╲'
        );
        if ok {
            *cell = Cell::new(ch, fg);
        }
    }
}
/// Pen rewrite of grow_spiral_tree: single tall trunk, alternating branches.
/// Trunk is ruler-straight with this tree type. Branches peel off at regular
/// intervals, each shorter than the last. Tips curl upward with a secondary
/// twig sprouting from the curl.
pub fn grow_spiral_pen(
    grid: &mut Grid,
    root_x: usize,
    root_y: usize,
    canopy_y: usize,
    spread: usize,
    color: Color,
    rng: &mut StdRng,
) {
    use MoveDir::*;
    if canopy_y + 3 >= root_y {
        return;
    }
    let height = root_y - canopy_y;
    let rx = root_x as i32;

    // ── Trunk: perfectly straight, full height ──
    // Spiral trees stand tall and vertical. No wobble. That IS their character.
    for y in canopy_y as i32..root_y as i32 {
        tset_over(grid, rx, y, '│', color);
    }
    tset_over(grid, rx, canopy_y as i32, '╷', lighten(color, 50));

    // ── Alternating branches ──
    let interval = (height / 5).max(2);
    let mut left = rng.random_range(0..2u32) == 0;
    let mut level = 0usize;
    let mut y = (canopy_y + interval) as i32;

    while y < root_y as i32 - 1 {
        let arm = (spread.saturating_sub(level * 2)).max(2) as i32;
        let c = lighten(color, 60u8.saturating_sub((level * 15) as u8));

        if left {
            // Junction + horizontal arm leftward
            tset_over(grid, rx, y, '┤', c);
            for i in 1..arm {
                spiral_set(grid, rx - i, y, '─', c);
            }
            spiral_set(grid, rx - arm, y, '╴', c);
            // Curl-up tip with secondary twig
            if level < 3 {
                spiral_set(grid, rx - arm, y - 1, '╮', c);
                spiral_set(grid, rx - arm - 1, y - 1, '╷', lighten(c, 25));
                // Secondary twig off curl
                if arm > 3 {
                    spiral_set(grid, rx - arm + 1, y - 1, '─', lighten(c, 15));
                    spiral_set(grid, rx - arm + 2, y - 1, '╷', lighten(c, 35));
                }
            }
        } else {
            // Junction + horizontal arm rightward
            tset_over(grid, rx, y, '├', c);
            for i in 1..arm {
                spiral_set(grid, rx + i, y, '─', c);
            }
            spiral_set(grid, rx + arm, y, '╶', c);
            if level < 3 {
                spiral_set(grid, rx + arm, y - 1, '╭', c);
                spiral_set(grid, rx + arm + 1, y - 1, '╷', lighten(c, 25));
                if arm > 3 {
                    spiral_set(grid, rx + arm - 1, y - 1, '─', lighten(c, 15));
                    spiral_set(grid, rx + arm - 2, y - 1, '╷', lighten(c, 35));
                }
            }
        }

        left = !left;
        y += interval as i32;
        level += 1;
    }
}
/// Candelabra-pen overwrite: trunk and bar overwrite everything.
/// Arms overwrite blanks + grass but respect trunk/bar chars.
pub(crate) fn cand_set(grid: &mut Grid, x: i32, y: i32, ch: char, fg: Color) {
    if x >= 0 && y >= 0 && (y as usize) < grid.len() && (x as usize) < grid[0].len() {
        let cell = &mut grid[y as usize][x as usize];
        let ok = matches!(
            cell.ch,
            ' ' | '·' | '∙' | '∿' | '~' | '░' | '▒' | '▓' | '╱' | '╲' | '╷'
        );
        if ok {
            *cell = Cell::new(ch, fg);
        }
    }
}
/// Pen rewrite of grow_candelabra: short thick trunk, horizontal bar at 1/3
/// height, 3-5 arms rising from the bar. Each arm leans outward, has mid-branch
/// bark texture, and tips with a two-way fork. The bar uses ┬ at trunk, └/┘ at
/// ends, ┴ at arm attachments.
pub fn grow_candelabra_pen(
    grid: &mut Grid,
    root_x: usize,
    root_y: usize,
    canopy_y: usize,
    spread: usize,
    color: Color,
    rng: &mut StdRng,
) {
    if canopy_y + 4 >= root_y {
        return;
    }
    let height = root_y - canopy_y;
    let rx = root_x as i32;
    let arm_count = rng.random_range(3..6usize);
    let split_y = (root_y - height / 3) as i32;
    let bark = darken(color, 15);

    // ── Trunk: short, thick ──
    // Candelabra trunk is wide: 3 columns. Center ┃, flanks │.
    // No wobble. Solid pedestal.
    for y in split_y..root_y as i32 {
        tset_over(grid, rx, y, '┃', color);
        tset_over(grid, rx - 1, y, '│', bark);
        tset_over(grid, rx + 1, y, '│', bark);
    }
    // Base flare
    tset_over(grid, rx - 2, root_y as i32 - 1, '╱', bark);
    tset_over(grid, rx + 2, root_y as i32 - 1, '╲', bark);

    // ── Horizontal bar ──
    let total_spread = spread as i32 * 2;
    let start_x = rx - total_spread / 2;
    let end_x = rx + total_spread / 2;

    for x in start_x..=end_x {
        tset_over(grid, x, split_y, '─', darken(color, 10));
    }
    tset_over(grid, rx, split_y, '┬', color);

    // ── Arms ──
    let arm_step = total_spread / (arm_count as i32 - 1).max(1);

    for i in 0..arm_count {
        let ax = start_x + i as i32 * arm_step;
        let arm_color = lighten(color, 20);
        let arm_top = canopy_y as i32 + rng.random_range(0..3u32) as i32;

        // Attachment junction on bar
        let jc = if i == 0 {
            '└'
        } else if i == arm_count - 1 {
            '┘'
        } else {
            '┴'
        };
        tset_over(grid, ax, split_y, jc, color);

        // Each arm goes straight up with a lean near midpoint
        let lean: i32 = if ax < rx {
            -1
        } else if ax > rx {
            1
        } else {
            0
        };
        let arm_height = (split_y - arm_top).max(1);
        let mid_y = arm_top + arm_height / 2;

        let mut cx = ax;
        for y in (arm_top..split_y).rev() {
            // Lean once near the middle
            if y == mid_y && lean != 0 {
                let ch = if lean < 0 { '╲' } else { '╱' };
                cand_set(grid, cx, y, ch, arm_color);
                cx += lean;
            } else {
                cand_set(grid, cx, y, '│', arm_color);
            }
            // Bark nub texture
            if rng.random_range(0..4u32) == 0 {
                let side = if rng.random_range(0..2u32) == 0 {
                    -1i32
                } else {
                    1
                };
                cand_set(grid, cx + side, y, if side > 0 { '╶' } else { '╴' }, bark);
            }
        }

        // Tip: two-way fork with short horizontal + vertical stubs
        let tip_c = lighten(arm_color, 30);
        tset_over(grid, cx, arm_top, '┼', tip_c);
        // Left fork
        cand_set(grid, cx - 1, arm_top, '─', tip_c);
        cand_set(grid, cx - 2, arm_top, '╮', tip_c);
        cand_set(grid, cx - 2, arm_top - 1, '╷', lighten(tip_c, 20));
        // Right fork
        cand_set(grid, cx + 1, arm_top, '─', tip_c);
        cand_set(grid, cx + 2, arm_top, '╭', tip_c);
        cand_set(grid, cx + 2, arm_top - 1, '╷', lighten(tip_c, 20));
    }
}
pub fn grow_tendril_tree(
    grid: &mut Grid,
    root_x: usize,
    root_y: usize,
    canopy_y: usize,
    spread: usize,
    color: Color,
    rng: &mut StdRng,
) {
    if canopy_y + 2 >= root_y {
        return;
    }
    let height = (root_y - canopy_y) as i32;

    // Short trunk up to the burst center
    let center_y = root_y as i32 - height / 3;
    for y in center_y..root_y as i32 {
        tset_over(grid, root_x as i32, y, '│', color);
    }

    let cx = root_x as f32;
    let cy = center_y as f32;

    // Draw a ray from (x,y) at angle, length, recursing with halved length
    fn draw_tendril(
        grid: &mut Grid,
        x: f32,
        y: f32,
        angle: f32,
        length: f32,
        min_len: f32,
        color: Color,
        depth: usize,
        rng: &mut StdRng,
    ) {
        if length < min_len || depth > 5 {
            return;
        }

        let c = lighten(color, (depth * 15) as u8);
        let steps = length as i32;
        let dx = angle.cos();
        let dy = angle.sin();

        // Aspect ratio correction: horizontal movement needs ~2x
        for step in 1..=steps {
            let px = (x + dx * step as f32 * 1.8) as i32;
            let py = (y + dy * step as f32) as i32;

            // Pick char based on angle
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

            tset(grid, px, py, ch, c);
        }

        // Tip position
        let tip_x = x + dx * steps as f32 * 1.8;
        let tip_y = y + dy * steps as f32;

        // Tip dot
        tset(grid, tip_x as i32, tip_y as i32, '·', lighten(c, 30));

        // Spawn 1-3 sub-tendrils at the tip
        let sub_count = rng.random_range(1..4u32);
        for _ in 0..sub_count {
            let angle_jitter = (rng.random::<f32>() - 0.5) * 1.2; // +-0.6 radians
            let sub_angle = angle + angle_jitter;
            let sub_len = length * (0.4 + rng.random::<f32>() * 0.2); // 40-60% of parent
            draw_tendril(
                grid,
                tip_x,
                tip_y,
                sub_angle,
                sub_len,
                min_len,
                color,
                depth + 1,
                rng,
            );
        }
    }

    // Initial burst: 3-6 rays radiating mostly upward from center
    let ray_count = rng.random_range(3..7u32);
    let base_len = (spread as f32).max(3.0).min(15.0);
    let min_len = 1.5f32;

    for i in 0..ray_count {
        // Spread rays in upper semicircle with some randomness
        let base_angle =
            -std::f32::consts::PI + (i as f32 / ray_count as f32) * std::f32::consts::PI;
        let angle = base_angle + (rng.random::<f32>() - 0.5) * 0.5;
        let len = base_len * (0.6 + rng.random::<f32>() * 0.4);
        draw_tendril(grid, cx, cy, angle, len, min_len, color, 0, rng);
    }
}
/// Zigzag tree: diagonal-only trunk and branches with recursive splitting.
/// Thick trunk (double-wide diagonals), branches fork recursively off rays.
pub fn grow_zigzag_tree(
    grid: &mut Grid,
    root_x: usize,
    root_y: usize,
    canopy_y: usize,
    spread: usize,
    color: Color,
    rng: &mut StdRng,
) {
    if canopy_y + 3 >= root_y {
        return;
    }
    let height = (root_y - canopy_y) as i32;

    // Thick zigzag trunk: two parallel diagonal lines
    let mut cx = root_x as i32;
    let zig_width = rng.random_range(2..4u32) as i32;
    let mut going_right = rng.random_range(0..2u32) == 0;
    let mut trunk_path: Vec<(i32, i32)> = Vec::new();

    for y in (canopy_y as i32..root_y as i32).rev() {
        let ch = if going_right { '╱' } else { '╲' };
        // Main trunk line
        tset_over(grid, cx, y, ch, color);
        // Thick: parallel line offset by 1
        tset(grid, cx + 1, y, ch, darken(color, 15));
        trunk_path.push((cx, y));
        cx += if going_right { 1 } else { -1 };

        let rows_up = root_y as i32 - y;
        if rows_up % (zig_width * 2 + 1) == 0 {
            going_right = !going_right;
        }
    }

    // Recursive diagonal ray helper
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
        rng: &mut StdRng,
    ) {
        let ch = match (dx < 0, dy < 0) {
            (true, true) => '╲',
            (false, true) => '╱',
            (true, false) => '╱',
            (false, false) => '╲',
        };
        let c = lighten(color, (depth * 18) as u8);

        for step in 1..=len {
            let rx = x + dx * step;
            let ry = y + dy * step;
            tset(grid, rx, ry, ch, c);

            // Random sub-branch: probability decreases with depth
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
                let sub_dy = -dy; // flip vertical direction for variety
                let sub_len = rng.random_range(1..(len / 2 + 1).max(2) as u32) as i32;
                draw_ray(
                    grid,
                    rx,
                    ry,
                    sub_dx,
                    sub_dy,
                    sub_len,
                    color,
                    depth + 1,
                    max_depth,
                    rng,
                );
            }
        }
        // Tip
        let tip_x = x + dx * (len + 1);
        let tip_y = y + dy * (len + 1);
        tset(grid, tip_x, tip_y, '·', lighten(c, 30));
    }

    // Primary branches off the trunk at random positions
    let branch_count = rng.random_range(4..9u32);
    let max_depth = rng.random_range(2..4u32) as usize;
    for _ in 0..branch_count {
        let idx = rng.random_range(0..trunk_path.len() as u32) as usize;
        let (tx, ty) = trunk_path[idx];

        let ray_len = rng.random_range(3..(spread as u32 + 3).min(14)) as i32;
        let go_left = rng.random_range(0..2u32) == 0;
        let go_up = rng.random_range(0..3u32) != 0;

        let dx: i32 = if go_left { -1 } else { 1 };
        let dy: i32 = if go_up { -1 } else { 1 };

        draw_ray(grid, tx, ty, dx, dy, ray_len, color, 0, max_depth, rng);
    }
}
/// Braille canopy tree: trunk of box-drawing, but canopy is a filled region
/// drawn with braille block characters for an organic, dense look.
/// Vertical color gradient through canopy. Occasional cuttlefish hue shift.
pub fn grow_braille_tree(
    grid: &mut Grid,
    root_x: usize,
    root_y: usize,
    canopy_y: usize,
    spread: usize,
    color: Color,
    rng: &mut StdRng,
) {
    if canopy_y + 2 >= root_y {
        return;
    }
    let rx = root_x as i32;
    let height = (root_y - canopy_y) as i32;

    // Trunk: simple vertical, bottom third
    let trunk_top = root_y as i32 - height / 3;
    for y in trunk_top..root_y as i32 {
        tset_over(grid, rx, y, '│', darken(color, 20));
    }

    // Canopy: irregular ellipse from canopy_y to trunk_top
    let canopy_h = (trunk_top - canopy_y as i32).max(2) as f32;
    let canopy_w = spread as f32;
    let center_y = canopy_y as f32 + canopy_h / 2.0;
    let center_x = rx as f32;

    // Cuttlefish mode: ~15% chance of wild hue shifts
    let cuttlefish = rng.random_range(0..7u32) == 0;
    // Base hue for gradient (extract from color or randomize)
    let base_hue: f64 = if let Color::Rgb { r, g, .. } = color {
        (r as f64 * 1.4 + g as f64 * 0.7) % 360.0
    } else {
        180.0
    };

    let braille_dense = ['⣿', '⣾', '⣷', '⣯', '⣻', '⣽', '⣖', '⣶'];
    let braille_sparse = ['⡇', '⢸', '⣤', '⣀', '⠛', '⠶'];

    for y in canopy_y as i32..=trunk_top {
        let fy = y as f32;
        let dy = (fy - center_y) / (canopy_h / 2.0);
        // Vertical progress: 0.0 at top, 1.0 at bottom
        let vert_t = ((y - canopy_y as i32) as f32 / canopy_h).clamp(0.0, 1.0);

        let noise = (rng.random_range(0..4u32) as f32 - 1.5) * 0.15;
        let row_width = ((1.0 - dy * dy).max(0.0).sqrt() + noise) * canopy_w;
        let half_w = (row_width * 1.5) as i32;

        for x in (center_x as i32 - half_w)..=(center_x as i32 + half_w) {
            let fx = x as f32;
            let dx_norm = (fx - center_x) / (half_w as f32).max(1.0);
            let dist = dx_norm.abs();

            let ch = if dist < 0.6 {
                braille_dense[rng.random_range(0..braille_dense.len() as u32) as usize]
            } else if dist < 0.85 {
                braille_sparse[rng.random_range(0..braille_sparse.len() as u32) as usize]
            } else {
                if rng.random_range(0..3u32) == 0 {
                    continue;
                }
                braille_sparse[rng.random_range(0..braille_sparse.len() as u32) as usize]
            };

            // Gradient color: hue shifts vertically, lightness fades at edges
            let hue_shift = if cuttlefish {
                // Wild per-cell hue jitter
                rng.random_range(0..180u32) as f64 - 90.0
            } else {
                // Gentle vertical gradient: 40 degree sweep top to bottom
                vert_t as f64 * 40.0 - 20.0
            };
            let h = (base_hue + hue_shift).rem_euclid(360.0);
            let s = if cuttlefish {
                0.8
            } else {
                0.5 + (1.0 - dist) as f64 * 0.3
            };
            let l = 0.2 + (1.0 - dist) as f64 * 0.3 + vert_t as f64 * 0.15;
            let c = crate::color::hsl_to_rgb(h, s, l.min(0.65));

            tset(grid, x, y, ch, c);
        }
    }
}
/// Draw a cloud as a horizontal streak with per-column height variation,
/// ragged edges, and trailing wisps. Not a uniform ellipse.
/// Sprout braille leaf clusters at branch tips on the grid.
/// Scans for tip chars (╷ ╮ ╭ · ╴ ╶) and places small braille clusters
/// around them. Only overwrites blank cells so branches show through.
pub fn sprout_leaves(grid: &mut Grid, leaf_color: Color, density: u32, rng: &mut StdRng) {
    let tip_chars = ['╷', '╮', '╭', '╴', '╶'];
    let leaf_chars = ['⣿', '⣾', '⣷', '⡇', '⢸', '⣤', '⣀', '⠛'];
    let sparse_leaf = ['⠂', '⠄', '⡀', '⠈', '⠁'];
    let h = grid.len();
    let w = if h > 0 { grid[0].len() } else { return };

    // Collect tip positions first (avoid borrow conflict)
    let mut tips: Vec<(usize, usize, Color)> = Vec::new();
    for y in 0..h {
        for x in 0..w {
            if tip_chars.contains(&grid[y][x].ch) {
                tips.push((x, y, grid[y][x].fg));
            }
        }
    }

    for (tx, ty, branch_color) in tips {
        // Skip some tips based on density (0=none, 100=all)
        if rng.random_range(0..100u32) >= density {
            continue;
        }

        // Cluster radius: 1-3 cells
        let radius = rng.random_range(1..4u32) as i32;
        // Mix leaf color with branch color for variety
        let lc = if rng.random_range(0..3u32) == 0 {
            leaf_color
        } else {
            branch_color
        };

        for dy in -radius..=radius {
            for dx in (-radius * 2)..=(radius * 2) {
                // aspect correction
                let gx = tx as i32 + dx;
                let gy = ty as i32 + dy;
                if gx < 0 || gy < 0 || gy as usize >= h || gx as usize >= w {
                    continue;
                }

                let dist = ((dx as f32 / 2.0).powi(2) + (dy as f32).powi(2)).sqrt();
                if dist > radius as f32 {
                    continue;
                }

                let cell = &grid[gy as usize][gx as usize];
                // Only fill blank cells
                if cell.ch != ' ' {
                    continue;
                }

                let ch = if dist < radius as f32 * 0.5 {
                    leaf_chars[rng.random_range(0..leaf_chars.len() as u32) as usize]
                } else {
                    // Ragged edge: sometimes skip
                    if rng.random_range(0..3u32) == 0 {
                        continue;
                    }
                    sparse_leaf[rng.random_range(0..sparse_leaf.len() as u32) as usize]
                };

                let brightness = ((1.0 - dist / radius as f32) * 35.0) as u8;
                let c = lighten(lc, brightness);
                grid[gy as usize][gx as usize] = Cell::new(ch, c);
            }
        }
    }
}
/// Collect branch tip positions within a bounding rect.
/// Returns (x, y, color) for each tip char found in the region.
pub fn collect_tips_in_rect(
    grid: &Grid,
    x0: usize,
    y0: usize,
    x1: usize,
    y1: usize,
) -> Vec<(usize, usize, Color)> {
    let tip_chars = ['╷', '╮', '╭', '╴', '╶', '·'];
    let h = grid.len();
    let w = if h > 0 {
        grid[0].len()
    } else {
        return Vec::new();
    };
    let mut tips = Vec::new();
    for y in y0..y1.min(h) {
        for x in x0..x1.min(w) {
            if tip_chars.contains(&grid[y][x].ch) {
                tips.push((x, y, grid[y][x].fg));
            }
        }
    }
    tips
}
/// Tip decorator styles. Each transforms a tip position differently.
#[derive(Clone, Copy)]
pub enum TipDeco {
    Fruit,  // single fruit glyph
    Flower, // tiny radial flower
    Drip,   // hanging dot below
    None,   // leave as-is
}
/// Decorate collected tips with the chosen style.
/// `chance` is 0-100 probability per tip.
pub fn decorate_tips(
    grid: &mut Grid,
    tips: &[(usize, usize, Color)],
    deco: TipDeco,
    color: Color,
    chance: u32,
    rng: &mut StdRng,
) {
    let h = grid.len();
    let w = if h > 0 { grid[0].len() } else { return };
    let fruit_glyphs = ['●', '◆', '◉', '○', '◇', '✦', '•'];
    let flower_glyphs = ['✦', '✧', '◉', '❋', '✿'];

    for &(tx, ty, _tip_color) in tips {
        if rng.random_range(0..100u32) >= chance {
            continue;
        }
        let c = shift_hue(color, rng.random_range(0..40u32) as f64 - 20.0);

        match deco {
            TipDeco::Fruit => {
                let g = fruit_glyphs[rng.random_range(0..fruit_glyphs.len() as u32) as usize];
                grid[ty][tx] = Cell::new(g, c);
                // Sometimes hang a second below
                if ty + 1 < h && grid[ty + 1][tx].ch == ' ' && rng.random_range(0..4u32) == 0 {
                    grid[ty + 1][tx] = Cell::new('•', darken(c, 15));
                }
            }
            TipDeco::Flower => {
                let g = flower_glyphs[rng.random_range(0..flower_glyphs.len() as u32) as usize];
                grid[ty][tx] = Cell::new(g, lighten(c, 20));
                // Petals in cardinal directions
                if tx > 0 && grid[ty][tx - 1].ch == ' ' {
                    grid[ty][tx - 1] = Cell::new('·', c);
                }
                if tx + 1 < w && grid[ty][tx + 1].ch == ' ' {
                    grid[ty][tx + 1] = Cell::new('·', c);
                }
            }
            TipDeco::Drip => {
                if ty + 1 < h && grid[ty + 1][tx].ch == ' ' {
                    grid[ty + 1][tx] = Cell::new('╷', darken(c, 20));
                    if ty + 2 < h && grid[ty + 2][tx].ch == ' ' && rng.random_range(0..2u32) == 0 {
                        grid[ty + 2][tx] = Cell::new('·', darken(c, 35));
                    }
                }
            }
            TipDeco::None => {}
        }
    }
}
/// Pick a trunk style that matches a tree family index.
pub fn trunk_style_for_family(family_idx: usize, rng: &mut StdRng) -> TrunkStyle {
    match family_idx {
        0 => [TrunkStyle::Wobble, TrunkStyle::Straight, TrunkStyle::Curved]
            [rng.random_range(0..3u32) as usize],
        1 => [TrunkStyle::Straight, TrunkStyle::Curved][rng.random_range(0..2u32) as usize],
        2 => [TrunkStyle::Gnarled, TrunkStyle::Wobble][rng.random_range(0..2u32) as usize],
        3 => [TrunkStyle::Thick, TrunkStyle::Gnarled, TrunkStyle::Wobble]
            [rng.random_range(0..3u32) as usize],
        _ => TrunkStyle::Straight,
    }
}
/// Wild tree with truly independent left/right branching.
/// Each side has its own branch count, heights, and arm lengths.
/// Nothing is mirrored. Trunk wobbles randomly.
/// Branch zone biased: some trees branch only near top, others near bottom.
pub fn grow_wild_tree(
    grid: &mut Grid,
    root_x: usize,
    root_y: usize,
    canopy_y: usize,
    spread: usize,
    color: Color,
    rng: &mut StdRng,
) {
    if canopy_y + 2 >= root_y {
        return;
    }
    let rx = root_x as i32;
    let height = (root_y - canopy_y) as i32;
    if height < 3 {
        return;
    }

    // Wobbling trunk: variable wobble intensity
    let mut cx = rx;
    let wobble_freq = rng.random_range(2..8u32) as i32;
    let wobble_prob = rng.random_range(1..4u32); // 1=aggressive, 3=mild
    // Store trunk x positions for accurate branch attachment
    let mut trunk_xs: Vec<(i32, i32)> = Vec::new(); // (y, x)

    for y in (canopy_y as i32..root_y as i32).rev() {
        let rows_up = root_y as i32 - y;
        let ch =
            if rows_up > 1 && rows_up % wobble_freq == 0 && rng.random_range(0..wobble_prob) == 0 {
                let dir = rng.random_range(0..2u32) as i32 * 2 - 1;
                cx += dir;
                if dir > 0 { '╱' } else { '╲' }
            } else {
                '│'
            };
        tset_over(grid, cx, y, ch, color);
        trunk_xs.push((y, cx));
    }
    tset_over(grid, cx, canopy_y as i32, '╷', lighten(color, 60));

    // Trunk x lookup
    let trunk_x_at = |target_y: i32| -> i32 {
        trunk_xs
            .iter()
            .min_by_key(|&&(y, _)| (y - target_y).abs())
            .map(|&(_, x)| x)
            .unwrap_or(rx)
    };

    // Branch zone bias: where branches concentrate along the trunk
    // 0=top-heavy, 1=bottom-heavy, 2=uniform, 3=mid-band
    let zone_style = rng.random_range(0..4u32);

    let biased_y = |rng: &mut StdRng| -> i32 {
        let t = match zone_style {
            0 => {
                // Top-heavy: branches cluster in upper 40%
                let t = rng.random::<f32>();
                t * t // quadratic bias toward 0 (top)
            }
            1 => {
                // Bottom-heavy: branches cluster in lower 40%
                let t = rng.random::<f32>();
                1.0 - (1.0 - t) * (1.0 - t)
            }
            3 => {
                // Mid-band: cluster around 30-70%
                0.3 + rng.random::<f32>() * 0.4
            }
            _ => rng.random::<f32>(), // uniform
        };
        canopy_y as i32 + 1 + (t * (height - 2) as f32) as i32
    };

    // Asymmetric branch counts: 0-8 per side independently
    let left_count = rng.random_range(0..9u32) as usize;
    let right_count = rng.random_range(0..9u32) as usize;

    let mut left_ys: Vec<i32> = (0..left_count).map(|_| biased_y(&mut *rng)).collect();
    left_ys.sort();
    left_ys.dedup();

    let mut right_ys: Vec<i32> = (0..right_count).map(|_| biased_y(&mut *rng)).collect();
    right_ys.sort();
    right_ys.dedup();

    // Draw left branches
    for (i, &by) in left_ys.iter().enumerate() {
        let tx = trunk_x_at(by);
        // Arm length varies more: short twigs to long reaching branches
        let arm = rng.random_range(1..(spread as u32 + 3).min(20)) as i32;
        let c = lighten(color, (i * 12 + 10) as u8);

        tset_over(grid, tx, by, '┤', c);
        for j in 1..=arm {
            tset(grid, tx - j, by, '─', c);
        }
        // Tip style varies
        match rng.random_range(0..4u32) {
            0 => {
                tset(grid, tx - arm, by, '╮', c);
                tset(grid, tx - arm - 1, by - 1, '╷', lighten(c, 25));
            }
            1 => {
                tset(grid, tx - arm, by, '╴', lighten(c, 20));
            }
            2 => {
                tset(grid, tx - arm, by, '·', lighten(c, 35));
            }
            _ => {
                tset(grid, tx - arm, by, '╮', c);
                // Upward sub-branch
                let sub = rng.random_range(1..4u32) as i32;
                for j in 1..=sub {
                    tset(grid, tx - arm, by - j, '│', lighten(c, 20));
                }
                tset(grid, tx - arm, by - sub - 1, '╷', lighten(c, 35));
            }
        }

        // Fork with higher probability, variable direction
        if arm > 2 && rng.random_range(0..2u32) == 0 {
            let fork_at = rng.random_range(1..arm as u32) as i32;
            let fork_x = tx - fork_at;
            let fork_dir = if rng.random_range(0..2u32) == 0 {
                1i32
            } else {
                -1
            }; // up or down
            tset_over(grid, fork_x, by, '┬', c);
            let sub_len = rng.random_range(1..5u32) as i32;
            for j in 1..=sub_len {
                tset(grid, fork_x, by + j * fork_dir, '│', lighten(c, 20));
            }
        }
    }

    // Draw right branches (independently)
    for (i, &by) in right_ys.iter().enumerate() {
        let tx = trunk_x_at(by);
        let arm = rng.random_range(1..(spread as u32 + 3).min(20)) as i32;
        let c = lighten(color, (i * 12 + 10) as u8);

        tset_over(grid, tx, by, '├', c);
        for j in 1..=arm {
            tset(grid, tx + j, by, '─', c);
        }
        match rng.random_range(0..4u32) {
            0 => {
                tset(grid, tx + arm, by, '╭', c);
                tset(grid, tx + arm + 1, by - 1, '╷', lighten(c, 25));
            }
            1 => {
                tset(grid, tx + arm, by, '╶', lighten(c, 20));
            }
            2 => {
                tset(grid, tx + arm, by, '·', lighten(c, 35));
            }
            _ => {
                tset(grid, tx + arm, by, '╭', c);
                let sub = rng.random_range(1..4u32) as i32;
                for j in 1..=sub {
                    tset(grid, tx + arm, by - j, '│', lighten(c, 20));
                }
                tset(grid, tx + arm, by - sub - 1, '╷', lighten(c, 35));
            }
        }

        if arm > 2 && rng.random_range(0..2u32) == 0 {
            let fork_at = rng.random_range(1..arm as u32) as i32;
            let fork_x = tx + fork_at;
            let fork_dir = if rng.random_range(0..2u32) == 0 {
                1i32
            } else {
                -1
            };
            tset_over(grid, fork_x, by, '┬', c);
            let sub_len = rng.random_range(1..5u32) as i32;
            for j in 1..=sub_len {
                tset(grid, fork_x, by + j * fork_dir, '│', lighten(c, 20));
            }
        }
    }
}
