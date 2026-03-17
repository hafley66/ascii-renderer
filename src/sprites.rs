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
    let first_split = root_y - (height / 3).max(2);

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

    let height = root_y - canopy_y;
    let first_split = root_y - (height / 3).max(2);

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
