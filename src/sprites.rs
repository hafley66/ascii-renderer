use crossterm::style::Color;
use rand::rngs::StdRng;
use rand::RngExt;
use crate::types::*;
use crate::color::*;

/// Cardinal directions for the turtle walker.
#[derive(Clone, Copy)]
pub enum Dir { Right, Down, Left, Up }

impl Dir {
    pub fn turn_right(self) -> Dir {
        match self {
            Dir::Right => Dir::Down,
            Dir::Down => Dir::Left,
            Dir::Left => Dir::Up,
            Dir::Up => Dir::Right,
        }
    }

    pub fn advance(self, x: i32, y: i32) -> (i32, i32) {
        match self {
            Dir::Right => (x + 1, y),
            Dir::Down => (x, y + 1),
            Dir::Left => (x - 1, y),
            Dir::Up => (x, y - 1),
        }
    }

    pub fn horizontal_glyph(self) -> char {
        match self {
            Dir::Right | Dir::Left => '─',
            Dir::Down | Dir::Up => '│',
        }
    }
}

/// Draw a single stepped fret (xicalcoliuhqui) spiral.
pub fn draw_stepped_fret(grid: &mut Grid, start_x: i32, start_y: i32, steps: usize, initial_dir: Dir, color: Color) {
    if steps == 0 { return; }

    let set = |grid: &mut Grid, x: i32, y: i32, ch: char| {
        if x >= 0 && y >= 0 && (y as usize) < grid.len() && (x as usize) < grid[0].len() {
            grid[y as usize][x as usize] = Cell::new(ch, color);
        }
    };

    let mut arms: Vec<usize> = Vec::new();
    for i in (1..=steps).rev() {
        arms.push(i);
        arms.push(i);
    }

    let mut x = start_x;
    let mut y = start_y;
    let mut dir = initial_dir;

    for (arm_idx, &arm_length) in arms.iter().enumerate() {
        for j in 0..arm_length {
            set(grid, x, y, dir.horizontal_glyph());
            if j < arm_length - 1 {
                let (nx, ny) = dir.advance(x, y);
                x = nx;
                y = ny;
            }
        }

        let corner = match dir {
            Dir::Right => '┐',
            Dir::Down  => '┘',
            Dir::Left  => '└',
            Dir::Up    => '┌',
        };
        set(grid, x, y, corner);

        dir = dir.turn_right();
        if arm_idx < arms.len() - 1 {
            let (nx, ny) = dir.advance(x, y);
            x = nx;
            y = ny;
        }
    }
}

/// Draw a stepped fret border band along an edge of a rectangular region.
pub fn draw_fret_border(grid: &mut Grid, x0: usize, y0: usize, w: usize, h: usize, band_depth: usize, edge: usize, color: Color) {
    let (start_x, start_y, dir, count) = match edge {
        0 => (x0 as i32, y0 as i32, Dir::Right, w),
        1 => ((x0 + w - 1) as i32, y0 as i32, Dir::Down, h),
        2 => ((x0 + w - 1) as i32, (y0 + h - 1) as i32, Dir::Left, w),
        _ => (x0 as i32, (y0 + h - 1) as i32, Dir::Up, h),
    };

    let unit_spacing = band_depth * 2 + 1;
    let mut pos = 0;
    while pos + band_depth <= count {
        let (sx, sy) = match edge {
            0 => (start_x + pos as i32, start_y),
            1 => (start_x, start_y + pos as i32),
            2 => (start_x - pos as i32, start_y),
            _ => (start_x, start_y - pos as i32),
        };
        draw_stepped_fret(grid, sx, sy, band_depth, dir, color);
        pos += unit_spacing;
    }
}

/// Draw a conifer/pine tree.
pub fn draw_pine(grid: &mut Grid, root_x: usize, root_y: usize, tiers: usize, base_width: usize, color: Color) {
    let set = |grid: &mut Grid, x: usize, y: usize, ch: char, fg: Color| {
        if y < grid.len() && x < grid[0].len() {
            grid[y][x] = Cell::new(ch, fg);
        }
    };

    let trunk_top = root_y.saturating_sub(2);
    for y in trunk_top..=root_y {
        set(grid, root_x, y, '│', darken(color, 60));
    }

    let mut tier_bottom = trunk_top;
    for t in 0..tiers {
        let tier_height = (base_width / 2).saturating_sub(t).max(2);
        let tier_width = base_width.saturating_sub(t * 2).max(3);
        let half = tier_width / 2;
        let tier_top = tier_bottom.saturating_sub(tier_height);

        for row in tier_top..tier_bottom {
            let progress = tier_bottom - row;
            let row_half = (half * progress) / tier_height;
            let left = root_x.saturating_sub(row_half);
            let right = (root_x + row_half).min(grid[0].len() - 1);

            if left < root_x {
                set(grid, left, row, '╱', color);
            }
            if right > root_x {
                set(grid, right, row, '╲', color);
            }
            let needles = ['▪', '◆', '●', '▫'];
            for x in (left + 1)..right {
                let needle = needles[(x + row) % needles.len()];
                let nc = if (x + row) % 3 == 0 { lighten(color, 20) } else { color };
                set(grid, x, row, needle, nc);
            }
        }
        if tier_top > 0 && t == tiers - 1 {
            set(grid, root_x, tier_top.saturating_sub(1), '▲', lighten(color, 30));
        }

        tier_bottom = tier_top + 1;
    }
}

/// Draw a weeping willow.
pub fn draw_willow(grid: &mut Grid, root_x: usize, root_y: usize, canopy_y: usize, spread: usize, color: Color) {
    let set = |grid: &mut Grid, x: usize, y: usize, ch: char, fg: Color| {
        if y < grid.len() && x < grid[0].len() && grid[y][x].ch == ' ' {
            grid[y][x] = Cell::new(ch, fg);
        }
    };

    let height = root_y.saturating_sub(canopy_y);
    if height < 4 { return; }
    let first_split = root_y.saturating_sub((height / 3).max(2));

    for y in first_split..root_y {
        set(grid, root_x, y, '│', darken(color, 40));
    }

    let mut queue: Vec<(usize, usize, usize, usize)> = vec![(root_x, canopy_y, first_split, 0)];
    let max_depth = 3;
    let mut tips: Vec<(usize, usize, Color)> = Vec::new();

    while let Some((x, top, bottom, depth)) = queue.pop() {
        let branch_color = lighten(color, (depth as u8) * 15);

        if depth >= max_depth || bottom <= top + 1 {
            for y in top..bottom {
                set(grid, x, y, '│', branch_color);
            }
            tips.push((x, bottom, lighten(branch_color, 20)));
            continue;
        }

        let split_y = top + (bottom - top) / 2;
        for y in (split_y + 1)..bottom {
            set(grid, x, y, '│', branch_color);
        }

        let arm_len = (spread >> depth).max(2);
        let left_x = x.saturating_sub(arm_len);
        let right_x = (x + arm_len).min(grid[0].len() - 1);

        set(grid, x, split_y, '┼', branch_color);
        if left_x < x {
            set(grid, left_x, split_y, '╭', branch_color);
            for ax in (left_x + 1)..x {
                set(grid, ax, split_y, '─', branch_color);
            }
        }
        if right_x > x {
            set(grid, right_x, split_y, '╮', branch_color);
            for ax in (x + 1)..right_x {
                set(grid, ax, split_y, '─', branch_color);
            }
        }

        queue.push((left_x, top, split_y, depth + 1));
        queue.push((right_x, top, split_y, depth + 1));
    }

    let droop_chars = ['╎', '┊', '╏', '┆', '│'];
    for (tx, ty, tc) in &tips {
        let droop_len = (root_y - ty).min(8);
        for d in 0..droop_len {
            let dy = ty + d;
            let ch = droop_chars[d % droop_chars.len()];
            set(grid, *tx, dy, ch, lighten(*tc, (d as u8) * 8));
            if d > 1 && d % 2 == 0 {
                if *tx > 0 {
                    set(grid, tx - 1, dy, '╲', lighten(*tc, (d as u8) * 10));
                }
                if tx + 1 < grid[0].len() {
                    set(grid, tx + 1, dy, '╱', lighten(*tc, (d as u8) * 10));
                }
            }
        }
    }
}

/// Draw a palm tree.
pub fn draw_palm(grid: &mut Grid, root_x: usize, root_y: usize, trunk_height: usize, color: Color, rng: &mut StdRng) {
    let set = |grid: &mut Grid, x: usize, y: usize, ch: char, fg: Color| {
        if y < grid.len() && x < grid[0].len() {
            grid[y][x] = Cell::new(ch, fg);
        }
    };

    let trunk_color = darken(color, 50);
    let crown_y = root_y.saturating_sub(trunk_height);

    let trunk_chars = ['┃', '┃', '╿', '┃'];
    for y in crown_y..root_y {
        let ch = trunk_chars[(y + root_x) % trunk_chars.len()];
        set(grid, root_x, y, ch, trunk_color);
    }

    let frond_color = lighten(color, 10);
    let frond_defs: &[(i32, i32, &[char])] = &[
        (-2, -1, &['╲', '─', '╲', '╲', '╷']),
        (-2,  0, &['─', '─', '╲', '╲', '╷']),
        (-1,  1, &['╲', '╲', '╲', '╷', '╵']),
        ( 2, -1, &['╱', '─', '╱', '╱', '╷']),
        ( 2,  0, &['─', '─', '╱', '╱', '╷']),
        ( 1,  1, &['╱', '╱', '╱', '╷', '╵']),
        ( 0, -1, &['│', '│', '╷', '·']),
        (-1, -1, &['╲', '╲', '╲', '╷']),
        ( 1, -1, &['╱', '╱', '╱', '╷']),
    ];

    for (dx, dy, glyphs) in frond_defs {
        let mut fx = root_x as i32;
        let mut fy = crown_y as i32;
        for (step, &ch) in glyphs.iter().enumerate() {
            fx += dx;
            fy += dy;
            if fx >= 0 && fy >= 0 && (fy as usize) < grid.len() && (fx as usize) < grid[0].len() {
                let fc = if step < 2 { frond_color } else { lighten(frond_color, (step as u8) * 12) };
                set(grid, fx as usize, fy as usize, ch, fc);
            }
        }
    }

    let coconut_positions: &[(i32, i32)] = &[(-1, 0), (1, 0), (0, 1)];
    let num_coconuts = rng.random_range(2..=3);
    for i in 0..num_coconuts {
        let (cdx, cdy) = coconut_positions[i];
        let cx = (root_x as i32 + cdx) as usize;
        let cy = (crown_y as i32 + cdy) as usize;
        if cy < grid.len() && cx < grid[0].len() {
            set(grid, cx, cy, '●', darken(color, 30));
        }
    }
}

/// Draw a fruit at position (x, y).
pub fn draw_fruit(grid: &mut Grid, cx: usize, cy: usize, style: usize, color: Color) {
    let patterns: &[&[(i32, i32, char)]] = &[
        &[(0, 0, '●'), (0, -1, '╿'), (1, -1, '╌')],
        &[(-1, 0, '●'), (1, 0, '●'), (-1, -1, '╱'), (1, -1, '╲'), (0, -1, '┬')],
        &[(0, 0, '◉'), (0, -1, '╷')],
        &[(0, 0, '●'), (-1, 0, '•'), (1, 0, '•'), (0, -1, '•'), (0, 1, '•')],
        &[(0, 0, '◆'), (0, 1, '●'), (0, -1, '╿'), (1, -1, '╌')],
    ];

    let pattern = patterns[style % patterns.len()];
    for &(dx, dy, ch) in pattern {
        let x = cx as i32 + dx;
        let y = cy as i32 + dy;
        if x >= 0 && y >= 0 && (y as usize) < grid.len() && (x as usize) < grid[0].len() {
            let c = if dx == 0 && dy == 0 { lighten(color, 30) } else { color };
            grid[y as usize][x as usize] = Cell::new(ch, c);
        }
    }
}

/// Grow a GRIS-style tree upward from (root_x, root_y).
pub fn grow_tree(grid: &mut Grid, root_x: usize, root_y: usize, canopy_y: usize, spread: usize, color: Color, _rng: &mut StdRng) {
    let set = |grid: &mut Grid, x: usize, y: usize, ch: char, fg: Color| {
        if y < grid.len() && x < grid[0].len() {
            grid[y][x] = Cell::new(ch, fg);
        }
    };

    if canopy_y >= root_y { return; }
    let height = root_y - canopy_y;
    let first_split = root_y.saturating_sub((height / 3).max(2));

    for y in first_split..root_y {
        set(grid, root_x, y, '│', color);
    }

    let mut queue: Vec<(usize, usize, usize, usize)> = vec![(root_x, canopy_y, first_split, 0)];
    let max_depth = 4;

    while let Some((x, top, bottom, depth)) = queue.pop() {
        let branch_color = match depth {
            0 => color,
            1 => lighten(color, 20),
            2 => lighten(color, 40),
            _ => lighten(color, 60),
        };

        if depth >= max_depth || bottom <= top + 1 {
            for y in top..bottom {
                if y < grid.len() && x < grid[0].len() && grid[y][x].ch == ' ' {
                    set(grid, x, y, '│', branch_color);
                }
            }
            if top < grid.len() && x < grid[0].len() {
                set(grid, x, top, '╷', lighten(branch_color, 30));
            }
            continue;
        }

        let split_y = top + (bottom - top) / 2;
        for y in split_y + 1..bottom {
            if y < grid.len() && x < grid[0].len() && grid[y][x].ch == ' ' {
                set(grid, x, y, '│', branch_color);
            }
        }

        let arm_len = (spread >> depth).max(2);
        let left_x = x.saturating_sub(arm_len);
        let right_x = (x + arm_len).min(grid[0].len() - 1);

        if split_y < grid.len() && x < grid[0].len() {
            set(grid, x, split_y, '┤', branch_color);
        }

        if left_x < x {
            set(grid, left_x, split_y, '╭', branch_color);
            for ax in left_x + 1..x {
                if ax < grid[0].len() {
                    set(grid, ax, split_y, '─', branch_color);
                }
            }
        }

        if right_x > x {
            set(grid, x, split_y, '┼', branch_color);
            for ax in x + 1..right_x {
                if ax < grid[0].len() {
                    set(grid, ax, split_y, '─', branch_color);
                }
            }
            set(grid, right_x, split_y, '╮', branch_color);
        }

        queue.push((left_x, top, split_y, depth + 1));
        queue.push((right_x, top, split_y, depth + 1));
    }
}

// ── GRIS-style tree family ───────────────────────────────────────────
//
// All variants: thin trunk, bare branches, no foliage.
// Junction vocabulary: ├ ┤ ┼ ┬ for splits, ╭ ╮ ╰ ╯ for curves, ╷ for tips.

/// Shared cell setter used by all tree functions.
fn tset(grid: &mut Grid, x: i32, y: i32, ch: char, fg: Color) {
    if x >= 0 && y >= 0 && (y as usize) < grid.len() && (x as usize) < grid[0].len() {
        let cell = &mut grid[y as usize][x as usize];
        if cell.ch == ' ' { *cell = Cell::new(ch, fg); }
    }
}

fn tset_over(grid: &mut Grid, x: i32, y: i32, ch: char, fg: Color) {
    if x >= 0 && y >= 0 && (y as usize) < grid.len() && (x as usize) < grid[0].len() {
        grid[y as usize][x as usize] = Cell::new(ch, fg);
    }
}

/// Spiral / Fibonacci tree.
/// Main trunk runs the full height. Branches peel off alternating sides,
/// each shorter than the last. Secondary twigs curl upward off the tips.
pub fn grow_spiral_tree(
    grid: &mut Grid,
    root_x: usize, root_y: usize, canopy_y: usize,
    spread: usize, color: Color, rng: &mut StdRng,
) {
    if canopy_y >= root_y { return; }
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
        let c = lighten(color, (60 - level * 15) as u8);

        if left {
            tset_over(grid, rx, y, '┤', c);
            for i in 1..arm { tset(grid, rx - i, y, '─', c); }
            tset(grid, rx - arm, y, '╴', c);
            if level < 3 {
                tset(grid, rx - arm, y - 1, '╮', c);
                tset(grid, rx - arm - 1, y - 1, '╷', lighten(c, 25));
            }
        } else {
            tset_over(grid, rx, y, '├', c);
            for i in 1..arm { tset(grid, rx + i, y, '─', c); }
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
    root_x: usize, root_y: usize, canopy_y: usize,
    spread: usize, color: Color, rng: &mut StdRng,
) {
    if canopy_y >= root_y { return; }
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
        let jc = if i == 0 { '└' } else if i == arm_count - 1 { '┘' } else { '┴' };
        tset_over(grid, ax, split_y, jc, color);

        // Each arm goes straight up with a small tilt
        let lean: i32 = if ax < rx { -1 } else if ax > rx { 1 } else { 0 };
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
    root_x: usize, root_y: usize, canopy_y: usize,
    spread: usize, color: Color, rng: &mut StdRng,
) {
    if canopy_y >= root_y { return; }
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
        if rng.random_range(0..4u32) == 0 { y += interval; left = !left; continue; }

        let arm = (rng.random_range(2..=spread.min(6)) as i32).max(1);
        let c = lighten(color, rng.random_range(10..50) as u8);

        if left {
            tset_over(grid, rx, y, '┤', c);
            for i in 1..arm { tset(grid, rx - i, y, '─', c); }
            // spray tip: two short diagonals
            tset(grid, rx - arm, y, '╮', c);
            tset(grid, rx - arm - 1, y - 1, '╷', lighten(c, 20));
            if arm > 2 { tset(grid, rx - arm + 1, y - 1, '╷', lighten(c, 10)); }
        } else {
            tset_over(grid, rx, y, '├', c);
            for i in 1..arm { tset(grid, rx + i, y, '─', c); }
            tset(grid, rx + arm, y, '╭', c);
            tset(grid, rx + arm + 1, y - 1, '╷', lighten(c, 20));
            if arm > 2 { tset(grid, rx + arm - 1, y - 1, '╷', lighten(c, 10)); }
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
    root_x: usize, root_y: usize, canopy_y: usize,
    spread: usize, color: Color, rng: &mut StdRng,
) {
    if canopy_y >= root_y { return; }
    let height = root_y - canopy_y;
    let lean: i32 = if rng.random_range(0..2u32) == 0 { 1 } else { -1 };
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
    root_x: usize, root_y: usize, canopy_y: usize,
    spread: usize, color: Color, _rng: &mut StdRng,
) {
    if canopy_y >= root_y { return; }
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
        for x in lx..rx { tset(grid, x, sy, '─', c); }
        for x in rx+1..=rx2 { tset(grid, x, sy, '─', c); }
        tset(grid, lx, sy, '╭', c);
        tset(grid, rx2, sy, '╮', c);

        let next_sy = if li + 1 < levels.len() { levels[li + 1].0 } else { canopy_y as i32 };

        // left and right sub-trunks
        for y in next_sy..sy { tset(grid, lx, y, '│', c); }
        for y in next_sy..sy { tset(grid, rx2, y, '│', c); }

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
    root_x: usize, root_y: usize, canopy_y: usize,
    spread: usize, color: Color, rng: &mut StdRng,
) {
    if canopy_y >= root_y { return; }
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
        (rx - left_spread, canopy_y as i32, first_split as i32, 0, left_depth),
        (rx + right_spread, canopy_y as i32, first_split as i32, 0, right_depth),
    ];

    // junction at first split
    let c0 = color;
    tset_over(grid, rx, first_split as i32, '┼', c0);
    for x in rx - left_spread..rx { tset(grid, x, first_split as i32, '─', c0); }
    for x in rx + 1..=rx + right_spread { tset(grid, x, first_split as i32, '─', c0); }
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
        for ax in x - arm..x { tset(grid, ax, split_y, '─', c); }
        for ax in x + 1..=x + arm { tset(grid, ax, split_y, '─', c); }
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
    root_x: usize, root_y: usize, canopy_y: usize,
    _spread: usize, color: Color, _rng: &mut StdRng,
) {
    if canopy_y >= root_y { return; }
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
        for ax in x - arm + 1..x - 1 { tset(grid, ax, split_y, '─', c); }
        tset_over(grid, x, split_y, '├', c);
        tset(grid, x + 1, split_y, '─', c);
        tset(grid, x + arm, split_y, '╮', c);
        for ax in x + 2..x + arm { tset(grid, ax, split_y, '─', c); }

        queue.push((x - arm, top, split_y, depth + 1));
        queue.push((x + arm, top, split_y, depth + 1));
        // Continue center upward
        for y in top + 1..split_y { tset(grid, x, y, '│', c); }
    }
}

/// Dead / skeletal tree.
/// Sparse angular branches. Uses diagonal chars and sharp tips. Eerie.
pub fn grow_dead_tree(
    grid: &mut Grid,
    root_x: usize, root_y: usize, canopy_y: usize,
    spread: usize, color: Color, rng: &mut StdRng,
) {
    if canopy_y >= root_y { return; }
    let height = root_y - canopy_y;
    let rx = root_x as i32;

    // Gnarled trunk: mostly vertical but with occasional diagonal offsets
    let mut cx = rx;
    for y in (canopy_y..root_y).rev() {
        let iy = y as i32;
        let from_root = root_y as i32 - iy;
        let ch = if from_root > 2 && from_root % 7 == 0 && rng.random_range(0..3u32) == 0 {
            let lean = if rng.random_range(0..2u32) == 0 { -1i32 } else { 1 };
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
        if by >= root_y as i32 - 1 { break; }
        let c = lighten(color, (b * 12) as u8);
        let arm = rng.random_range(2..=(spread.min(8))) as i32;

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
    root_x: usize, root_y: usize, canopy_y: usize,
    spread: usize, color: Color, rng: &mut StdRng,
) {
    if canopy_y >= root_y { return; }
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
            for x in x0..=x1 { tset(grid, x, first_split, '─', c0); }
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

/// Dispatch all tree variants by kind index (0..12).
/// Expands the original 4 kinds with 8 new GRIS-style variants.
pub fn draw_tree(
    grid: &mut Grid,
    root_x: usize, root_y: usize, canopy_y: usize,
    spread: usize, kind: usize, color: Color, rng: &mut StdRng,
) {
    match kind % 12 {
        0  => grow_tree(grid, root_x, root_y, canopy_y, spread, color, rng),
        1  => draw_pine(grid, root_x, root_y, 3, (spread * 2).min(12), color),
        2  => draw_willow(grid, root_x, root_y, canopy_y, spread, color),
        3  => draw_palm(grid, root_x, root_y, root_y.saturating_sub(canopy_y).saturating_sub(4), color, rng),
        4  => grow_spiral_tree(grid, root_x, root_y, canopy_y, spread, color, rng),
        5  => grow_candelabra(grid, root_x, root_y, canopy_y, spread, color, rng),
        6  => grow_birch(grid, root_x, root_y, canopy_y, spread, color, rng),
        7  => grow_storm_tree(grid, root_x, root_y, canopy_y, spread, color, rng),
        8  => grow_wide_tree(grid, root_x, root_y, canopy_y, spread, color, rng),
        9  => grow_asymmetric_tree(grid, root_x, root_y, canopy_y, spread, color, rng),
        10 => grow_tall_narrow(grid, root_x, root_y, canopy_y, spread, color, rng),
        _  => grow_drooping_tree(grid, root_x, root_y, canopy_y, spread, color, rng),
    }
}

/// Draw a small flower/rosette at (cx, cy)
pub fn draw_flower(grid: &mut Grid, cx: usize, cy: usize, style: usize, color: Color) {
    let patterns: &[&[(i32, i32, char)]] = &[
        &[(0,-1,'◆'), (-1,0,'◇'), (1,0,'◇'), (0,1,'◆'), (0,0,'✦')],
        &[(0,-1,'◠'), (-1,0,'◟'), (1,0,'◞'), (0,1,'◡'), (0,0,'◉')],
        &[(0,-1,'∧'), (-1,0,'⟨'), (1,0,'⟩'), (0,1,'∨'), (0,0,'✧'), (-1,-1,'╱'), (1,-1,'╲'), (-1,1,'╲'), (1,1,'╱')],
        &[(0,-1,'╥'), (-1,0,'╟'), (1,0,'╢'), (0,1,'╨'), (0,0,'╬'), (-1,-1,'╔'), (1,-1,'╗'), (-1,1,'╚'), (1,1,'╝')],
        &[(0,0,'⣿'), (-1,0,'⡇'), (1,0,'⢸'), (0,-1,'⣤'), (0,1,'⣶'), (-1,-1,'⠁'), (1,-1,'⠈'), (-1,1,'⢀'), (1,1,'⡀')],
    ];

    let pattern = patterns[style % patterns.len()];
    for (i, &(dx, dy, ch)) in pattern.iter().enumerate() {
        let x = cx as i32 + dx;
        let y = cy as i32 + dy;
        if x >= 0 && y >= 0 && (y as usize) < grid.len() && (x as usize) < grid[0].len() {
            let c = if i == pattern.len() - 1 || (dx == 0 && dy == 0) {
                lighten(color, 40)
            } else {
                color
            };
            grid[y as usize][x as usize] = Cell::new(ch, c);
        }
    }
}

/// Mask/firework sprite: two eyes on a vertical stem with radiating diagonals.
/// Emergent pattern captured from flower + tree + diamond lattice overlap.
/// `size` controls the radius of the radiating lines (1 = compact, 2-4 = larger).
pub fn draw_mask(grid: &mut Grid, cx: usize, cy: usize, size: usize, style: usize, color: Color) {
    let bright = lighten(color, 40);
    let dim = darken(color, 30);

    let set = |grid: &mut Grid, dx: i32, dy: i32, ch: char, c: Color| {
        let x = cx as i32 + dx;
        let y = cy as i32 + dy;
        if x >= 0 && y >= 0 && (y as usize) < grid.len() && (x as usize) < grid[0].len() {
            grid[y as usize][x as usize] = Cell::new(ch, c);
        }
    };

    // eyes: two dots flanking center
    let eye_ch = match style % 4 {
        0 => '●',
        1 => '◉',
        2 => '◆',
        _ => '⬤',
    };
    set(grid, -1, 0, eye_ch, bright);
    set(grid, 1, 0, eye_ch, bright);

    // nose/mouth below eyes
    let nose_ch = match style % 4 {
        0 => '●',
        1 => '◡',
        2 => '◆',
        _ => '▪',
    };
    set(grid, 0, 1, nose_ch, bright);

    // horizontal brow dashes
    for i in 2..=(size as i32 + 1) {
        set(grid, -i, 0, '─', dim);
        set(grid, i, 0, '─', dim);
    }

    // vertical stem above and below
    for i in 1..=(size as i32) {
        set(grid, 0, -(i as i32), '│', dim);
    }
    for i in 2..=(size as i32 + 2) {
        set(grid, 0, i as i32, '│', dim);
    }

    // radiating diagonals
    for i in 1..=(size as i32) {
        set(grid, -i, -i, '╲', color);
        set(grid, i, -i, '╱', color);
        set(grid, -i, i, '╱', color);
        set(grid, i, i, '╲', color);
    }

    // secondary diagonals (wider spread, dimmer)
    if size >= 2 {
        for i in 1..=(size as i32) {
            set(grid, -i - 1, -i, '╲', dim);
            set(grid, i + 1, -i, '╱', dim);
            set(grid, -i - 1, i, '╱', dim);
            set(grid, i + 1, i, '╲', dim);
        }
    }
}

pub const MASK_STYLE_COUNT: usize = 4;

/// Aztec diamond domino tiling via domino shuffling.
pub fn draw_aztec_diamond(grid: &mut Grid, center_x: usize, center_y: usize, order: usize, palette: &[Color; 5], rng: &mut StdRng) {
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum D { N, S, E, W, Empty }

    let in_diamond = |r: usize, c: usize, ord: usize| -> bool {
        let size = 2 * ord;
        if r >= size || c >= size { return false; }
        let rr = (2 * r + 1) as i32 - size as i32;
        let cc = (2 * c + 1) as i32 - size as i32;
        rr.abs() + cc.abs() <= size as i32
    };

    let mut state: Vec<Vec<D>> = vec![vec![D::Empty; 2]; 2];
    if rng.random_range(0..2) == 0 {
        state[0][0] = D::W; state[0][1] = D::E;
        state[1][0] = D::W; state[1][1] = D::E;
    } else {
        state[0][0] = D::N; state[0][1] = D::N;
        state[1][0] = D::S; state[1][1] = D::S;
    }

    for k in 2..=order {
        let old_size = 2 * (k - 1);
        let new_size = 2 * k;

        for r in 0..old_size - 1 {
            for c in 0..old_size - 1 {
                let tl = state[r][c];
                let tr = state[r][c + 1];
                let bl = state[r + 1][c];
                let br = state[r + 1][c + 1];
                if tl == D::S && tr == D::S && bl == D::N && br == D::N {
                    state[r][c] = D::Empty;     state[r][c + 1] = D::Empty;
                    state[r + 1][c] = D::Empty; state[r + 1][c + 1] = D::Empty;
                }
                if tl == D::E && tr == D::W && bl == D::E && br == D::W {
                    state[r][c] = D::Empty;     state[r][c + 1] = D::Empty;
                    state[r + 1][c] = D::Empty; state[r + 1][c + 1] = D::Empty;
                }
            }
        }

        let mut ns: Vec<Vec<D>> = vec![vec![D::Empty; new_size]; new_size];
        for r in 0..old_size {
            for c in 0..old_size {
                let d = state[r][c];
                if d == D::Empty { continue; }
                let (nr, nc) = match d {
                    D::N => (r,     c + 1),
                    D::S => (r + 2, c + 1),
                    D::W => (r + 1, c),
                    D::E => (r + 1, c + 2),
                    D::Empty => unreachable!(),
                };
                if nr < new_size && nc < new_size && in_diamond(nr, nc, k) {
                    ns[nr][nc] = d;
                }
            }
        }

        for r in 0..new_size - 1 {
            for c in 0..new_size - 1 {
                if ns[r][c] == D::Empty
                    && ns[r][c + 1] == D::Empty
                    && ns[r + 1][c] == D::Empty
                    && ns[r + 1][c + 1] == D::Empty
                    && in_diamond(r, c, k)
                    && in_diamond(r + 1, c, k)
                    && in_diamond(r, c + 1, k)
                    && in_diamond(r + 1, c + 1, k)
                {
                    if rng.random_range(0..2) == 0 {
                        ns[r][c] = D::N;     ns[r][c + 1] = D::N;
                        ns[r + 1][c] = D::S; ns[r + 1][c + 1] = D::S;
                    } else {
                        ns[r][c] = D::W;     ns[r][c + 1] = D::E;
                        ns[r + 1][c] = D::W; ns[r + 1][c + 1] = D::E;
                    }
                }
            }
        }

        state = ns;
    }

    let size = 2 * order;
    let render_w = size * 2;
    let off_r = center_y.saturating_sub(order);
    let off_c = center_x.saturating_sub(render_w / 2);
    let colors = [palette[1], palette[2], palette[3], lighten(palette[2], 40)];
    for r in 0..size {
        for c in 0..size {
            if state[r][c] == D::Empty { continue; }
            let gr = off_r + r;
            let gc = off_c + c * 2;
            let color = match state[r][c] {
                D::N => colors[0],
                D::S => colors[1],
                D::E => colors[2],
                D::W => colors[3],
                D::Empty => unreachable!(),
            };
            for dx in 0..2 {
                let gx = gc + dx;
                if gr < grid.len() && gx < grid[0].len() {
                    grid[gr][gx] = Cell::with_bg(' ', Color::Reset, color);
                }
            }
        }
    }
}
