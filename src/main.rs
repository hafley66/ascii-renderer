use crossterm::style::Color;
use rand::RngExt;
use rand::rngs::StdRng;
use rand::SeedableRng;
use std::io::{self, Write};

#[derive(Clone, Copy)]
struct Cell {
    ch: char,
    fg: Color,
    bg: Color,
}

impl Cell {
    fn blank() -> Self {
        Cell { ch: ' ', fg: Color::Reset, bg: Color::Reset }
    }

    fn new(ch: char, fg: Color) -> Self {
        Cell { ch, fg, bg: Color::Reset }
    }

    fn with_bg(ch: char, fg: Color, bg: Color) -> Self {
        Cell { ch, fg, bg }
    }
}

type Grid = Vec<Vec<Cell>>;

fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::Rgb { r, g, b }
}

fn hsl_to_rgb(h: f64, s: f64, l: f64) -> Color {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;
    let (r1, g1, b1) = match h as u32 {
        0..=59 => (c, x, 0.0),
        60..=119 => (x, c, 0.0),
        120..=179 => (0.0, c, x),
        180..=239 => (0.0, x, c),
        240..=299 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    Color::Rgb {
        r: ((r1 + m) * 255.0) as u8,
        g: ((g1 + m) * 255.0) as u8,
        b: ((b1 + m) * 255.0) as u8,
    }
}

/// Named themes. Each is [background, primary, secondary, accent, text].
fn named_theme(name: &str) -> Option<[Color; 5]> {
    Some(match name {
        // --- warm ---
        "ember" => [
            rgb(25, 8, 2),       // near-black warm
            rgb(204, 85, 0),     // burnt orange
            rgb(140, 40, 50),    // dried blood
            rgb(255, 160, 50),   // amber glow
            rgb(240, 220, 200),  // warm white
        ],
        "terracotta" => [
            rgb(30, 15, 10),     // dark earth
            rgb(180, 100, 60),   // clay
            rgb(90, 130, 80),    // sage
            rgb(220, 170, 100),  // sand
            rgb(235, 225, 210),  // parchment
        ],
        "sakura" => [
            rgb(20, 10, 18),     // dark plum
            rgb(200, 120, 160),  // cherry blossom
            rgb(100, 80, 90),    // bark
            rgb(255, 180, 200),  // petal pink
            rgb(240, 235, 240),  // soft white
        ],

        // --- cool ---
        "arctic" => [
            rgb(5, 10, 20),      // deep night
            rgb(100, 160, 220),  // ice blue
            rgb(60, 80, 120),    // steel
            rgb(180, 230, 255),  // frost
            rgb(230, 240, 250),  // snow
        ],
        "deep" => [
            rgb(2, 5, 15),       // abyss
            rgb(30, 80, 160),    // ocean
            rgb(80, 40, 120),    // purple depth
            rgb(50, 200, 180),   // bioluminescent
            rgb(200, 220, 240),  // foam
        ],
        "moss" => [
            rgb(8, 15, 5),       // forest floor
            rgb(80, 140, 60),    // moss green
            rgb(50, 80, 40),     // dark fern
            rgb(160, 200, 80),   // lichen
            rgb(210, 230, 200),  // pale green
        ],

        // --- monochrome ---
        "bone" => [
            rgb(15, 14, 12),     // charcoal
            rgb(180, 170, 155),  // bone
            rgb(120, 115, 105),  // stone
            rgb(220, 210, 190),  // ivory
            rgb(240, 235, 225),  // cream
        ],
        "silver" => [
            rgb(10, 10, 12),     // gunmetal
            rgb(140, 145, 160),  // silver
            rgb(80, 85, 95),     // pewter
            rgb(200, 205, 220),  // bright silver
            rgb(235, 235, 240),  // platinum
        ],

        // --- vivid ---
        "neon" => [
            rgb(5, 0, 10),       // void
            rgb(0, 255, 128),    // neon green
            rgb(255, 0, 128),    // hot pink
            rgb(0, 200, 255),    // cyan
            rgb(255, 255, 255),  // pure white
        ],
        "nerv" => [
            rgb(10, 2, 15),      // eva purple-black
            rgb(200, 50, 20),    // nerv red
            rgb(100, 60, 160),   // eva purple
            rgb(255, 180, 0),    // warning orange
            rgb(220, 220, 230),  // terminal gray
        ],
        "mitla" => [
            rgb(20, 12, 5),      // obsidian earth
            rgb(190, 140, 60),   // gold stone
            rgb(140, 60, 40),    // red clay
            rgb(100, 170, 130),  // jade
            rgb(230, 220, 200),  // limestone
        ],

        _ => return None,
    })
}

/// Seed-deterministic palette: rotate hue based on seed, derive harmonious colors.
/// Returns [background, primary, secondary, accent, text].
fn make_palette(seed: u64) -> [Color; 5] {
    let base_hue = (seed % 360) as f64;
    [
        hsl_to_rgb(base_hue, 0.3, 0.15),
        hsl_to_rgb((base_hue + 30.0) % 360.0, 0.6, 0.55),
        hsl_to_rgb((base_hue + 180.0) % 360.0, 0.5, 0.45),
        hsl_to_rgb((base_hue + 60.0) % 360.0, 0.7, 0.65),
        rgb(220, 220, 220),
    ]
}

/// Grow a GRIS-style tree upward from (root_x, root_y).
fn grow_tree(grid: &mut Grid, root_x: usize, root_y: usize, canopy_y: usize, spread: usize, color: Color, _rng: &mut StdRng) {
    let set = |grid: &mut Grid, x: usize, y: usize, ch: char, fg: Color| {
        if y < grid.len() && x < grid[0].len() {
            grid[y][x] = Cell::new(ch, fg);
        }
    };

    let height = root_y - canopy_y;
    let first_split = root_y - (height / 3).max(2);

    // trunk
    for y in first_split..root_y {
        set(grid, root_x, y, '│', color);
    }

    let mut queue: Vec<(usize, usize, usize, usize)> = vec![(root_x, canopy_y, first_split, 0)];
    let max_depth = 4;

    while let Some((x, top, bottom, depth)) = queue.pop() {
        // lighten color at deeper branches
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

fn lighten(color: Color, amount: u8) -> Color {
    match color {
        Color::Rgb { r, g, b } => Color::Rgb {
            r: r.saturating_add(amount),
            g: g.saturating_add(amount),
            b: b.saturating_add(amount),
        },
        other => other,
    }
}

fn darken(color: Color, amount: u8) -> Color {
    match color {
        Color::Rgb { r, g, b } => Color::Rgb {
            r: r.saturating_sub(amount),
            g: g.saturating_sub(amount),
            b: b.saturating_sub(amount),
        },
        other => other,
    }
}

/// Cardinal directions for the turtle walker.
#[derive(Clone, Copy)]
enum Dir { Right, Down, Left, Up }

impl Dir {
    fn turn_right(self) -> Dir {
        match self {
            Dir::Right => Dir::Down,
            Dir::Down => Dir::Left,
            Dir::Left => Dir::Up,
            Dir::Up => Dir::Right,
        }
    }

    fn advance(self, x: i32, y: i32) -> (i32, i32) {
        match self {
            Dir::Right => (x + 1, y),
            Dir::Down => (x, y + 1),
            Dir::Left => (x - 1, y),
            Dir::Up => (x, y - 1),
        }
    }

    fn horizontal_glyph(self) -> char {
        match self {
            Dir::Right | Dir::Left => '─',
            Dir::Down | Dir::Up => '│',
        }
    }
}

/// Draw a single stepped fret (xicalcoliuhqui) spiral.
fn draw_stepped_fret(grid: &mut Grid, start_x: i32, start_y: i32, steps: usize, initial_dir: Dir, color: Color) {
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
fn draw_fret_border(grid: &mut Grid, x0: usize, y0: usize, w: usize, h: usize, band_depth: usize, edge: usize, color: Color) {
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

/// Aztec diamond domino tiling via domino shuffling.
fn draw_aztec_diamond(grid: &mut Grid, center_x: usize, center_y: usize, order: usize, palette: &[Color; 5], rng: &mut StdRng) {
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
        let new_size = 2 * k;
        let old_size = 2 * (k - 1);
        let mut ns: Vec<Vec<D>> = vec![vec![D::Empty; new_size]; new_size];

        for r in 0..old_size {
            for c in 0..old_size {
                let d = state[r][c];
                if d == D::Empty { continue; }
                let (nr, nc): (i32, i32) = match d {
                    D::N => (r as i32,     c as i32 + 1),
                    D::S => (r as i32 + 2, c as i32 + 1),
                    D::W => (r as i32 + 1, c as i32),
                    D::E => (r as i32 + 1, c as i32 + 2),
                    D::Empty => unreachable!(),
                };
                if nr >= 0 && nc >= 0 && (nr as usize) < new_size && (nc as usize) < new_size
                    && in_diamond(nr as usize, nc as usize, k)
                {
                    ns[nr as usize][nc as usize] = d;
                }
            }
        }

        let snap = ns.clone();
        for r in 0..new_size {
            for c in 0..new_size {
                if r + 1 < new_size && snap[r][c] == D::S && snap[r + 1][c] == D::N {
                    ns[r][c] = D::Empty;
                    ns[r + 1][c] = D::Empty;
                }
                if c + 1 < new_size && snap[r][c] == D::E && snap[r][c + 1] == D::W {
                    ns[r][c] = D::Empty;
                    ns[r][c + 1] = D::Empty;
                }
            }
        }

        for r in 0..new_size - 1 {
            for c in 0..new_size - 1 {
                if ns[r][c] == D::Empty
                    && ns[r + 1][c] == D::Empty
                    && ns[r][c + 1] == D::Empty
                    && ns[r + 1][c + 1] == D::Empty
                    && in_diamond(r, c, k)
                    && in_diamond(r + 1, c, k)
                    && in_diamond(r, c + 1, k)
                    && in_diamond(r + 1, c + 1, k)
                {
                    if rng.random_range(0..2) == 0 {
                        ns[r][c] = D::W;     ns[r][c + 1] = D::E;
                        ns[r + 1][c] = D::W; ns[r + 1][c + 1] = D::E;
                    } else {
                        ns[r][c] = D::N;     ns[r][c + 1] = D::N;
                        ns[r + 1][c] = D::S; ns[r + 1][c + 1] = D::S;
                    }
                }
            }
        }

        state = ns;
    }

    // Render with 4-quadrant coloring: each domino direction gets a palette color
    let size = 2 * order;
    let off_r = center_y.saturating_sub(order);
    let off_c = center_x.saturating_sub(order);
    for r in 0..size {
        for c in 0..size {
            if state[r][c] == D::Empty { continue; }
            let gr = off_r + r;
            let gc = off_c + c;
            if gr < grid.len() && gc < grid[0].len() {
                let (ch, color) = match state[r][c] {
                    D::N => ('▀', palette[1]),  // primary
                    D::S => ('▄', palette[2]),  // secondary
                    D::E => ('▐', palette[3]),  // accent
                    D::W => ('▌', palette[1]),  // primary variant
                    D::Empty => (' ', Color::Reset),
                };
                grid[gr][gc] = Cell::new(ch, color);
            }
        }
    }
}

/// Draw a small flower/rosette at (cx, cy)
fn draw_flower(grid: &mut Grid, cx: usize, cy: usize, style: usize, color: Color) {
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
            // center glyph brighter, petals slightly dimmer
            let c = if i == pattern.len() - 1 || (dx == 0 && dy == 0) {
                lighten(color, 40)
            } else {
                color
            };
            grid[y as usize][x as usize] = Cell::new(ch, c);
        }
    }
}

/// Print the grid with ANSI color escape sequences.
/// Run-length optimized: only emits color codes when the color changes.
fn render_grid(grid: &Grid) {
    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());

    let mut cur_fg = Color::Reset;
    let mut cur_bg = Color::Reset;

    for row in grid {
        for cell in row {
            if cell.fg != cur_fg {
                write!(out, "{}", crossterm::style::SetForegroundColor(cell.fg)).unwrap();
                cur_fg = cell.fg;
            }
            if cell.bg != cur_bg {
                write!(out, "{}", crossterm::style::SetBackgroundColor(cell.bg)).unwrap();
                cur_bg = cell.bg;
            }
            write!(out, "{}", cell.ch).unwrap();
        }
        // reset at end of each line to avoid bg bleeding
        if cur_bg != Color::Reset {
            write!(out, "{}", crossterm::style::SetBackgroundColor(Color::Reset)).unwrap();
            cur_bg = Color::Reset;
        }
        writeln!(out).unwrap();
    }

    // final reset
    write!(out, "{}", crossterm::style::ResetColor).unwrap();
    out.flush().unwrap();
}

fn main() {
    let seed: u64 = std::env::args().nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(42);

    let mode = std::env::args().nth(2).unwrap_or_default();
    let theme_name = std::env::args().nth(3).unwrap_or_default();

    let width = 80;
    let height = 45;
    let mut grid = vec![vec![Cell::blank(); width]; height];
    let mut rng = StdRng::seed_from_u64(seed);

    // Named theme overrides seed-based palette. Usage: <seed> <mode> <theme>
    // e.g. `42 "" nerv` or `42 tree mitla`
    let palette = if !theme_name.is_empty() {
        named_theme(&theme_name).unwrap_or_else(|| {
            let themes = ["ember", "terracotta", "sakura", "arctic", "deep", "moss",
                         "bone", "silver", "neon", "nerv", "mitla"];
            eprintln!("unknown theme '{}'. available: {}", theme_name, themes.join(", "));
            make_palette(seed)
        })
    } else {
        make_palette(seed)
    };

    if mode == "swatch" {
        // render all named themes as color swatches
        let themes = [
            "ember", "terracotta", "sakura",
            "arctic", "deep", "moss",
            "bone", "silver",
            "neon", "nerv", "mitla",
        ];
        let mut swatch_grid = vec![vec![Cell::blank(); 80]; themes.len() * 3 + 1];
        for (ti, name) in themes.iter().enumerate() {
            let p = named_theme(name).unwrap();
            let row = ti * 3;

            // label
            for (j, ch) in name.chars().enumerate() {
                if j < 12 {
                    swatch_grid[row][j] = Cell::new(ch, p[4]);
                }
            }

            // color blocks: bg, primary, secondary, accent, text
            let labels = ["bg", "pri", "sec", "acc", "txt"];
            for (ci, &color) in p.iter().enumerate() {
                let x_start = 13 + ci * 13;
                // label above
                for (j, ch) in labels[ci].chars().enumerate() {
                    if x_start + j < 80 {
                        swatch_grid[row][x_start + j] = Cell::new(ch, color);
                    }
                }
                // solid block
                for x in x_start..x_start + 10 {
                    if x < 80 {
                        swatch_grid[row + 1][x] = Cell::with_bg('█', color, Color::Reset);
                    }
                }
                // sample glyphs
                let sample = ['╱', '╲', '│', '─', '┌', '┐', '◆', '✦', '▀', '▄'];
                for (j, &ch) in sample.iter().enumerate() {
                    if x_start + j < 80 {
                        swatch_grid[row + 2][x_start + j] = Cell::new(ch, color);
                    }
                }
            }
        }
        render_grid(&swatch_grid);
        return;

    } else if mode == "tree" {
        grow_tree(&mut grid, 20, 40, 5, 16, palette[1], &mut rng);
        grow_tree(&mut grid, 55, 42, 8, 12, palette[2], &mut rng);

        draw_flower(&mut grid, 10, 42, 0, palette[3]);
        draw_flower(&mut grid, 70, 43, 1, palette[3]);
        draw_flower(&mut grid, 38, 38, 2, palette[3]);
        draw_flower(&mut grid, 45, 20, 3, palette[1]);
        draw_flower(&mut grid, 5, 10, 4, palette[2]);

    } else if mode == "aztec" {
        draw_aztec_diamond(&mut grid, width / 2, height / 2, 20, &palette, &mut rng);

    } else if mode == "fret" {
        // fret spirals at different sizes
        draw_stepped_fret(&mut grid, 5, 5, 3, Dir::Right, palette[1]);
        draw_stepped_fret(&mut grid, 25, 5, 5, Dir::Right, palette[2]);
        draw_stepped_fret(&mut grid, 50, 5, 7, Dir::Right, palette[3]);

        draw_stepped_fret(&mut grid, 10, 20, 5, Dir::Right, palette[1]);
        draw_stepped_fret(&mut grid, 30, 30, 5, Dir::Left, palette[2]);

        // border bands, each edge a different color
        draw_fret_border(&mut grid, 0, 0, width, height, 4, 0, palette[1]);
        draw_fret_border(&mut grid, 0, 0, width, height, 4, 1, palette[2]);
        draw_fret_border(&mut grid, 0, 0, width, height, 4, 2, palette[3]);
        draw_fret_border(&mut grid, 0, 0, width, height, 4, 3, palette[1]);

    } else if mode == "flowers" {
        for i in 0..5 {
            let color = [palette[1], palette[2], palette[3], palette[1], palette[2]][i];
            draw_flower(&mut grid, 8 + i * 15, 5, i, color);
            let labels = ["diamond", "circle", "star", "box", "braille"];
            for (j, ch) in labels[i].chars().enumerate() {
                if 8 + i * 15 - 2 + j < width {
                    grid[9][8 + i * 15 - 2 + j] = Cell::new(ch, palette[4]);
                }
            }
        }

    } else {
        // full demo: truchet bg + trees + content + flowers
        let truchet_color = darken(palette[1], 80);
        let tiles = ['╱', '╲'];
        for y in 0..height {
            for x in 0..width {
                grid[y][x] = Cell::new(
                    tiles[rng.random_range(0..2)],
                    truchet_color,
                );
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
                grid[y][x] = Cell::blank();
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
                        grid[y][x] = Cell::new(ch, palette[4]);
                    }
                }
            }
        }

        // trees in corners
        for y in 2..18 {
            for x in 2..22 { grid[y][x] = Cell::blank(); }
        }
        grow_tree(&mut grid, 12, 17, 3, 8, palette[1], &mut rng);

        for y in 2..18 {
            for x in 58..78 { grid[y][x] = Cell::blank(); }
        }
        grow_tree(&mut grid, 68, 17, 3, 8, palette[2], &mut rng);

        // flowers
        draw_flower(&mut grid, 30, 8, rng.random_range(0..5), palette[3]);
        draw_flower(&mut grid, 50, 8, rng.random_range(0..5), palette[3]);
        draw_flower(&mut grid, 15, 35, rng.random_range(0..5), palette[3]);
        draw_flower(&mut grid, 65, 35, rng.random_range(0..5), palette[3]);
        draw_flower(&mut grid, 40, 38, rng.random_range(0..5), palette[3]);
    }

    render_grid(&grid);
}
