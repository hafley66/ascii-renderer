use rand::RngExt;
use rand::rngs::StdRng;
use rand::SeedableRng;

type Grid = Vec<Vec<char>>;

/// Grow a GRIS-style tree upward from (root_x, root_y) into the grid.
/// The tree fills from canopy_y (top) to root_y (bottom).
/// Binary splits use box-drawing curves, all tips align at canopy_y.
fn grow_tree(grid: &mut Grid, root_x: usize, root_y: usize, canopy_y: usize, spread: usize, rng: &mut StdRng) {
    // recursive binary tree: each branch knows its x, the y range it owns,
    // and whether it goes left or right from its parent
    struct Branch {
        x: usize,
        split_y: usize,  // y where this branch splits into two children
        top_y: usize,     // y where tips terminate (canopy line)
        bottom_y: usize,  // y where this branch starts
    }

    let mut branches: Vec<Branch> = vec![];
    let height = root_y - canopy_y;

    // start with trunk
    // draw trunk from root_y up to first split
    let first_split = root_y - (height / 3).max(2);
    for y in first_split..root_y {
        if y < grid.len() && root_x < grid[0].len() {
            grid[y][root_x] = '│';
        }
    }

    // queue of (x, top_y, bottom_y, depth)
    let mut queue: Vec<(usize, usize, usize, usize)> = vec![(root_x, canopy_y, first_split, 0)];
    let max_depth = 4;

    while let Some((x, top, bottom, depth)) = queue.pop() {
        if depth >= max_depth || bottom <= top + 1 {
            // terminal branch: just draw vertical down to canopy
            for y in top..bottom {
                if y < grid.len() && x < grid[0].len() && grid[y][x] == ' ' {
                    grid[y][x] = '│';
                }
            }
            // tip
            if top < grid.len() && x < grid[0].len() {
                grid[top][x] = '╷';
            }
            continue;
        }

        let split_y = top + (bottom - top) / 2;

        // vertical trunk segment from bottom up to split point
        for y in split_y + 1..bottom {
            if y < grid.len() && x < grid[0].len() && grid[y][x] == ' ' {
                grid[y][x] = '│';
            }
        }

        // calculate spread for this depth
        let arm_len = (spread >> depth).max(2);
        let left_x = x.saturating_sub(arm_len);
        let right_x = (x + arm_len).min(grid[0].len() - 1);

        // draw the split: horizontal arms with curved corners
        // center gets the branch point
        if split_y < grid.len() && x < grid[0].len() {
            grid[split_y][x] = '┤';
        }

        // left arm: ╭ at end, ─ horizontal, then │ down from there
        if left_x < x {
            if split_y < grid.len() && left_x < grid[0].len() {
                grid[split_y][left_x] = '╭';
            }
            for ax in left_x + 1..x {
                if split_y < grid.len() && ax < grid[0].len() {
                    grid[split_y][ax] = '─';
                }
            }
        }

        // right arm: ╮ at end, ─ horizontal
        if right_x > x {
            // overwrite center to full cross
            if split_y < grid.len() && x < grid[0].len() {
                grid[split_y][x] = '┼';
            }
            for ax in x + 1..right_x {
                if split_y < grid.len() && ax < grid[0].len() {
                    grid[split_y][ax] = '─';
                }
            }
            if split_y < grid.len() && right_x < grid[0].len() {
                grid[split_y][right_x] = '╮';
            }
        }

        // recurse: left child and right child
        queue.push((left_x, top, split_y, depth + 1));
        queue.push((right_x, top, split_y, depth + 1));
    }
}

/// Draw a small flower/rosette at (cx, cy)
fn draw_flower(grid: &mut Grid, cx: usize, cy: usize, style: usize) {
    let patterns: &[&[(i32, i32, char)]] = &[
        // diamond flower
        &[(0,-1,'◆'), (-1,0,'◇'), (1,0,'◇'), (0,1,'◆'), (0,0,'✦')],
        // circle bloom
        &[(0,-1,'◠'), (-1,0,'◟'), (1,0,'◞'), (0,1,'◡'), (0,0,'◉')],
        // star burst
        &[(0,-1,'∧'), (-1,0,'⟨'), (1,0,'⟩'), (0,1,'∨'), (0,0,'✧'), (-1,-1,'╱'), (1,-1,'╲'), (-1,1,'╲'), (1,1,'╱')],
        // box flower
        &[(0,-1,'╥'), (-1,0,'╟'), (1,0,'╢'), (0,1,'╨'), (0,0,'╬'), (-1,-1,'╔'), (1,-1,'╗'), (-1,1,'╚'), (1,1,'╝')],
        // braille bloom
        &[(0,0,'⣿'), (-1,0,'⡇'), (1,0,'⢸'), (0,-1,'⣤'), (0,1,'⣶'), (-1,-1,'⠁'), (1,-1,'⠈'), (-1,1,'⢀'), (1,1,'⡀')],
    ];

    let pattern = patterns[style % patterns.len()];
    for &(dx, dy, ch) in pattern {
        let x = cx as i32 + dx;
        let y = cy as i32 + dy;
        if x >= 0 && y >= 0 && (y as usize) < grid.len() && (x as usize) < grid[0].len() {
            grid[y as usize][x as usize] = ch;
        }
    }
}

fn main() {
    let seed: u64 = std::env::args().nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(42);

    let mode = std::env::args().nth(2).unwrap_or_default();

    let width = 80;
    let height = 45;
    let mut grid = vec![vec![' '; width]; height];
    let mut rng = StdRng::seed_from_u64(seed);

    if mode == "tree" {
        // tree demo: clear background, grow trees
        grow_tree(&mut grid, 20, 40, 5, 16, &mut rng);
        grow_tree(&mut grid, 55, 42, 8, 12, &mut rng);

        // scatter some flowers
        draw_flower(&mut grid, 10, 42, 0);
        draw_flower(&mut grid, 70, 43, 1);
        draw_flower(&mut grid, 38, 38, 2);
        draw_flower(&mut grid, 45, 20, 3);
        draw_flower(&mut grid, 5, 10, 4);

    } else if mode == "flowers" {
        // flower sampler
        for i in 0..5 {
            draw_flower(&mut grid, 8 + i * 15, 5, i);
            // label
            let labels = ["diamond", "circle", "star", "box", "braille"];
            for (j, ch) in labels[i].chars().enumerate() {
                if 8 + i * 15 - 2 + j < width {
                    grid[9][8 + i * 15 - 2 + j] = ch;
                }
            }
        }

    } else {
        // full demo: truchet + DLA + tree + flowers
        let tiles = ['╱', '╲'];
        for y in 0..height {
            for x in 0..width {
                grid[y][x] = tiles[rng.random_range(0..2)];
            }
        }

        // carve content region
        let cx = width / 2;
        let cy = height / 2;
        let content_w = 30;
        let content_h = 10;
        let x0 = cx - content_w / 2;
        let y0 = cy - content_h / 2;

        for y in y0..y0 + content_h {
            for x in x0..x0 + content_w {
                grid[y][x] = ' ';
            }
        }

        let lines = [
            "「 技 」 S K I L L S",
            "",
            "  typespec ···· 12",
            "  ast-grep ···· 5",
            "  tree-sit ···· 3",
            "  alloy    ···· 2",
            "",
            "  ◁━━ 43 LOADED",
        ];

        for (i, line) in lines.iter().enumerate() {
            let y = y0 + 1 + i;
            if y < y0 + content_h {
                for (j, ch) in line.chars().enumerate() {
                    let x = x0 + 1 + j;
                    if x < x0 + content_w {
                        grid[y][x] = ch;
                    }
                }
            }
        }

        // grow trees in corners
        // clear small regions for trees
        for y in 2..18 {
            for x in 2..22 {
                grid[y][x] = ' ';
            }
        }
        grow_tree(&mut grid, 12, 17, 3, 8, &mut rng);

        for y in 2..18 {
            for x in 58..78 {
                grid[y][x] = ' ';
            }
        }
        grow_tree(&mut grid, 68, 17, 3, 8, &mut rng);

        // flowers between content and trees
        draw_flower(&mut grid, 30, 8, rng.random_range(0..5));
        draw_flower(&mut grid, 50, 8, rng.random_range(0..5));
        draw_flower(&mut grid, 15, 35, rng.random_range(0..5));
        draw_flower(&mut grid, 65, 35, rng.random_range(0..5));
        draw_flower(&mut grid, 40, 38, rng.random_range(0..5));
    }

    // print
    for row in &grid {
        let line: String = row.iter().collect();
        println!("{}", line);
    }
}
