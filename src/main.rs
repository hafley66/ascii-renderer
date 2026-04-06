#![allow(warnings)]

mod automata;
mod biomes;
mod borders;
mod color;
mod content;
mod fills;
mod layout;
mod markdown;
mod mondrian;
mod render;
mod scene;
mod sprites;
mod tree_draw;
mod types;
mod walker;

use crossterm::style::Color;
use rand::RngExt;
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::io::{self, IsTerminal, Read as _};

use automata::*;
use biomes::*;
use color::*;
use content::*;
use fills::*;
use layout::*;
use markdown::*;
use mondrian::*;
use render::*;
use scene::*;
use sprites::*;
use tree_draw::*;
use types::*;
use walker::*;

fn run_demo(initial_seed: u64) {
    use crossterm::{
        cursor,
        event::{self, Event, KeyCode, KeyModifiers},
        execute,
        terminal::{self, ClearType},
    };
    use std::io::Write;
    use std::process::Command;

    let all_modes: &[&str] = &[
        "party", "soup", "tree", "trees", "forest", "forest2", "forest3", "forest4",
        "forest5", "forest6", "forest7", "aztec", "fret", "flowers", "fruits", "masks",
        "shapes", "tiles", "tiles-rand", "tiles-skew", "mondrian", "mondrian2", "bsp",
        "layout", "terrain", "flow", "noise", "ca", "stem", "scene-walk", "scene-walk-2",
        "scene-walk-3", "world", "boles1", "boles2", "boles3", "trunks1", "trees1",
        "trees2", "trees3", "trees4", "bushes",
    ];
    let all_themes: &[&str] = &[
        "", "ember", "terracotta", "sakura", "arctic", "deep", "moss",
        "bone", "silver", "neon", "nerv", "mitla",
    ];

    let mut seed = initial_seed;
    let mut mode_idx: usize = 0;
    let mut theme_idx: usize = 0;

    let exe = std::env::current_exe().unwrap();

    terminal::enable_raw_mode().unwrap();
    execute!(io::stdout(), terminal::EnterAlternateScreen).unwrap();

    loop {
        // Disable raw mode so child process writes normal line endings
        terminal::disable_raw_mode().unwrap();
        execute!(
            io::stdout(),
            terminal::Clear(ClearType::All),
            cursor::MoveTo(0, 0)
        )
        .unwrap();

        let current_mode = all_modes[mode_idx];
        let current_theme = all_themes[theme_idx];

        let mut cmd = Command::new(&exe);
        cmd.arg(seed.to_string()).arg(current_mode);
        if !current_theme.is_empty() {
            cmd.arg(current_theme);
        }
        let _ = cmd.status();

        // Re-enable raw mode for keyboard input
        terminal::enable_raw_mode().unwrap();

        let (tw, _th) = terminal::size().unwrap_or((80, 45));
        execute!(io::stdout(), cursor::MoveTo(0, _th.saturating_sub(1))).unwrap();
        let theme_label = if current_theme.is_empty() {
            "auto"
        } else {
            current_theme
        };
        let status = format!(
            " {} | seed:{} | theme:{} | f/j=prev/next  \u{2191}\u{2193}=seed  \u{2190}\u{2192}=theme  enter=random  q=quit ",
            current_mode, seed, theme_label
        );
        // Pad to terminal width, inverse video
        let padded: String = if status.len() < tw as usize {
            format!("{}{}", status, " ".repeat(tw as usize - status.len()))
        } else {
            status[..tw as usize].to_string()
        };
        print!("\x1b[7m{}\x1b[0m", padded);
        io::stdout().flush().unwrap();

        if let Ok(Event::Key(key)) = event::read() {
            match key.code {
                KeyCode::Char('q') => break,
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => break,
                KeyCode::Char('j') => mode_idx = (mode_idx + 1) % all_modes.len(),
                KeyCode::Char('f') => {
                    mode_idx = (mode_idx + all_modes.len() - 1) % all_modes.len()
                }
                KeyCode::Up => seed = seed.wrapping_add(1),
                KeyCode::Down => seed = seed.wrapping_sub(1),
                KeyCode::Right => theme_idx = (theme_idx + 1) % all_themes.len(),
                KeyCode::Left => {
                    theme_idx = (theme_idx + all_themes.len() - 1) % all_themes.len()
                }
                KeyCode::Enter => {
                    seed = rand::rng().random_range(0..10000u64);
                }
                _ => {}
            }
        }
    }

    execute!(io::stdout(), terminal::LeaveAlternateScreen).unwrap();
    terminal::disable_raw_mode().unwrap();
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 && (args[1] == "--help" || args[1] == "-h") {
        eprintln!("ascii-renderer <seed> [mode] [theme]");
        eprintln!();
        eprintln!("ARGS:");
        eprintln!("  seed     Integer seed for deterministic RNG (default: 42)");
        eprintln!("  mode     Rendering mode (default: full demo)");
        eprintln!("  theme    Named color theme (default: seed-derived palette)");
        eprintln!();
        eprintln!("MODES:");
        eprintln!("  demo      Interactive browser: f/j=mode, arrows=seed/theme, enter=random, q=quit");
        eprintln!("  (none)    Full demo: Truchet bg, trees, content, flowers");
        eprintln!("  tree      GRIS-style binary trees with flowers");
        eprintln!("  forest    Mixed scene: pine, willow, palm, GRIS tree, fruits");
        eprintln!("  aztec     Aztec diamond domino tiling");
        eprintln!("  fret      Stepped fret spirals and border bands");
        eprintln!("  flowers   All 5 flower stamp styles with labels");
        eprintln!("  fruits    All 5 fruit stamp styles with labels");
        eprintln!("  layout    Two-column layout engine demo");
        eprintln!("  md        Render markdown from stdin");
        eprintln!("  bsp       BSP randomized layout demo");
        eprintln!("  mondrian  Mondrian-style colored grid layout");
        eprintln!("  tiles     Showcase all 10 tile patterns (pure deterministic)");
        eprintln!("  tiles-rand  Same patterns with randomized params");
        eprintln!("  noise     Showcase all 5 noise variants (truchet, higaki, etc.)");
        eprintln!("  terrain   Layered landscape: mountains, foothills, ground with contour boundaries");
        eprintln!("  flow      Vertical flow: fills morph through tapered zones");
        eprintln!("  masks     All 4 mask/firework sprite styles");
        eprintln!("  ca        Cellular automata: life|cave|maze|coral [style] [primitives]");
        eprintln!("  ca-layout CA as organic layout engine (text in largest regions)");
        eprintln!("  world     Vertical biome strips: forest, garden, temple, noise, geometric");
        eprintln!("  party     Node islands along a path [gap] [nodes] [scale] [detail] [weather] [path]");
        eprintln!("            weather: rain|snow|fog|stars|none (default: random)");
        eprintln!("            path:    line|dots|vine|river|double (default: random)");
        eprintln!("  soup      Dense overlapping node scenes along a path");
        eprintln!("  stem      Sinuous stalk with alternating shape-masked tile leaves");
        eprintln!("  boles1    Bole styles at 3 energy levels (low/mid/high)");
        eprintln!("  boles2    Experimental bole styles v2");
        eprintln!("  boles3    Refined bole styles with descriptive names");
        eprintln!("  trunks1   Horizontal trunk algorithms + direction-aware branching");
        eprintln!("  trees1    Full pipeline: tree+trunk+bole combos [energy] [fruit] [branch] [bole]");
        eprintln!("  trees2    Squat horizontal boles (1-2 rows) [energy] [fruit] [branch]");
        eprintln!("  trees3    Vertical catalog: all tree types, trunks, tapers, boles");
        eprintln!("  trees4    All 17 TreeDrawer types with boles and fruit");
        eprintln!("  bushes    Full-size bole patterns as standalone bush sprites");
        eprintln!("  forest7   Layered showcase forest with boles, tapers, fruit");
        eprintln!("  swatch    Color swatches for all named themes");
        eprintln!();
        eprintln!("THEMES:");
        eprintln!("  warm:  ember, terracotta, sakura");
        eprintln!("  cool:  arctic, deep, moss");
        eprintln!("  mono:  bone, silver");
        eprintln!("  vivid: neon, nerv, mitla");
        eprintln!();
        eprintln!("EXAMPLES:");
        eprintln!("  ascii-renderer 42");
        eprintln!("  ascii-renderer 42 tree mitla");
        eprintln!("  ascii-renderer 99 forest moss");
        eprintln!("  ascii-renderer 7 aztec nerv");
        eprintln!("  ascii-renderer 0 fret neon");
        eprintln!("  ascii-renderer 42 fruits");
        eprintln!("  ascii-renderer 42 layout ember");
        eprintln!("  echo '# Hello' | ascii-renderer 42 md nerv");
        eprintln!("  cat notes.md | ascii-renderer 42 md moss");
        eprintln!("  ascii-renderer 42 bsp nerv");
        eprintln!("  ascii-renderer 42 mondrian");
        eprintln!("  ascii-renderer 42 swatch");
        std::process::exit(0);
    }

    let seed: u64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(42);

    let mode = args.get(2).map(|s| s.as_str()).unwrap_or("");
    let theme_name = args.get(3).map(|s| s.as_str()).unwrap_or("");

    if mode == "demo" {
        run_demo(seed);
        return;
    }

    let (term_w, term_h) = crossterm::terminal::size().unwrap_or((80, 45));
    let width = term_w as usize;
    let height = term_h as usize;
    let mut grid = vec![vec![Cell::blank(); width]; height];
    let mut rng = StdRng::seed_from_u64(seed);

    let palette = if !theme_name.is_empty() {
        named_theme(&theme_name).unwrap_or_else(|| {
            let themes = [
                "ember",
                "terracotta",
                "sakura",
                "arctic",
                "deep",
                "moss",
                "bone",
                "silver",
                "neon",
                "nerv",
                "mitla",
            ];
            eprintln!(
                "unknown theme '{}'. available: {}",
                theme_name,
                themes.join(", ")
            );
            make_palette(seed)
        })
    } else {
        make_palette(seed)
    };

    if mode == "swatch" {
        let themes = [
            "ember",
            "terracotta",
            "sakura",
            "arctic",
            "deep",
            "moss",
            "bone",
            "silver",
            "neon",
            "nerv",
            "mitla",
        ];
        let mut swatch_grid = vec![vec![Cell::blank(); 80]; themes.len() * 3 + 1];
        for (ti, name) in themes.iter().enumerate() {
            let p = named_theme(name).unwrap();
            let row = ti * 3;

            for (j, ch) in name.chars().enumerate() {
                if j < 12 {
                    swatch_grid[row][j] = Cell::new(ch, p[4]);
                }
            }

            let labels = ["bg", "pri", "sec", "acc", "txt"];
            for (ci, &color) in p.iter().enumerate() {
                let x_start = 13 + ci * 13;
                for (j, ch) in labels[ci].chars().enumerate() {
                    if x_start + j < 80 {
                        swatch_grid[row][x_start + j] = Cell::new(ch, color);
                    }
                }
                for x in x_start..x_start + 10 {
                    if x < 80 {
                        swatch_grid[row + 1][x] = Cell::with_bg('█', color, Color::Reset);
                    }
                }
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
    } else if mode == "trees" {
        // Grid of all 12 tree variants. 4 columns x 3 rows.
        let cols = 4usize;
        let rows = 3usize;
        let cell_w = width / cols;
        let cell_h = height / rows;
        for row in 0..rows {
            for col in 0..cols {
                let kind = row * cols + col;
                let cx = col * cell_w + cell_w / 2;
                let root_y = (row + 1) * cell_h - 2;
                let canopy_y = row * cell_h + 2;
                let spread = (cell_w / 4).max(3);
                let color = palette[(kind % 3) + 1];
                draw_tree(&mut grid, cx, root_y, canopy_y, spread, kind, color, &mut rng);
                // kind label
                let label = format!("{}", kind);
                let lx = col * cell_w + 1;
                let ly = row * cell_h + 1;
                for (j, ch) in label.chars().enumerate() {
                    if lx + j < width && ly < height {
                        grid[ly][lx + j] = Cell::new(ch, darken(palette[4], 20));
                    }
                }
            }
        }
    } else if mode == "aztec" {
        draw_aztec_diamond(
            &mut grid,
            width / 2,
            height / 2,
            height / 2 - 2,
            &palette,
            &mut rng,
        );
    } else if mode == "fret" {
        draw_stepped_fret(&mut grid, 5, 5, 3, Dir::Right, palette[1]);
        draw_stepped_fret(&mut grid, 25, 5, 5, Dir::Right, palette[2]);
        draw_stepped_fret(&mut grid, 50, 5, 7, Dir::Right, palette[3]);

        draw_stepped_fret(&mut grid, 10, 20, 5, Dir::Right, palette[1]);
        draw_stepped_fret(&mut grid, 30, 30, 5, Dir::Left, palette[2]);

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
    } else if mode == "fruits" {
        let fruit_colors = [
            rgb(220, 50, 50),
            rgb(180, 30, 60),
            rgb(240, 180, 30),
            rgb(100, 50, 160),
            rgb(180, 200, 40),
        ];
        let labels = ["apple", "cherry", "citrus", "berry", "pear"];
        for i in 0..5 {
            draw_fruit(&mut grid, 8 + i * 15, 5, i, fruit_colors[i]);
            for (j, ch) in labels[i].chars().enumerate() {
                if 8 + i * 15 - 2 + j < width {
                    grid[9][8 + i * 15 - 2 + j] = Cell::new(ch, palette[4]);
                }
            }
        }
    } else if mode == "forest" {
        let ground_color = darken(palette[1], 90);
        let tiles = ['╱', '╲'];
        for y in 0..height {
            for x in 0..width {
                grid[y][x] = Cell::new(tiles[rng.random_range(0..2)], ground_color);
            }
        }

        let ground_y = height - 4;

        for y in 3..ground_y {
            for x in 2..22 {
                grid[y][x] = Cell::blank();
            }
        }
        grow_tree(&mut grid, 12, ground_y - 1, 4, 8, palette[1], &mut rng);

        for y in 5..(ground_y + 1) {
            for x in 24..40 {
                grid[y][x] = Cell::blank();
            }
        }
        draw_pine(&mut grid, 32, ground_y - 1, 3, 10, palette[2]);

        for y in 3..(ground_y + 3) {
            for x in 42..62 {
                grid[y][x] = Cell::blank();
            }
        }
        draw_willow(&mut grid, 52, ground_y - 1, 6, 8, palette[1]);

        for y in 2..(ground_y + 1) {
            for x in 64..78 {
                grid[y][x] = Cell::blank();
            }
        }
        draw_palm(&mut grid, 71, ground_y - 1, 20, palette[3], &mut rng);

        draw_fruit(&mut grid, 8, 12, 0, rgb(220, 50, 50));
        draw_fruit(&mut grid, 15, 10, 0, rgb(200, 60, 40));
        draw_fruit(&mut grid, 11, 8, 1, rgb(180, 30, 60));

        draw_fruit(&mut grid, 30, 25, 3, rgb(100, 50, 160));
        draw_fruit(&mut grid, 35, 28, 3, rgb(120, 40, 140));

        draw_fruit(&mut grid, 48, 20, 2, rgb(240, 180, 30));
        draw_fruit(&mut grid, 55, 18, 4, rgb(180, 200, 40));

        for i in 0..6 {
            let fx = 5 + i * 13;
            if fx < width - 2 {
                draw_flower(
                    &mut grid,
                    fx,
                    ground_y + 1,
                    rng.random_range(0..5),
                    palette[3],
                );
            }
        }
    } else if mode == "layout" {
        let truchet_color = darken(palette[1], 90);
        let tiles = ['╱', '╲'];
        for y in 0..height {
            for x in 0..width {
                grid[y][x] = Cell::new(tiles[rng.random_range(0..2)], truchet_color);
            }
        }

        let left = vec![
            ContentBlock {
                items: vec![
                    ContentItem::Text("「 STATUS 」".into()),
                    ContentItem::Rule,
                    ContentItem::Text("All systems operational. Last deploy 2h ago.".into()),
                ],
                padding: 1,
            },
            ContentBlock {
                items: vec![
                    ContentItem::Text("METRICS".into()),
                    ContentItem::Rule,
                    ContentItem::Bar {
                        label: "cpu".into(),
                        value: 72.0,
                        max: 100.0,
                    },
                    ContentItem::Bar {
                        label: "mem".into(),
                        value: 4.8,
                        max: 8.0,
                    },
                    ContentItem::Bar {
                        label: "disk".into(),
                        value: 120.0,
                        max: 500.0,
                    },
                    ContentItem::Bar {
                        label: "net".into(),
                        value: 340.0,
                        max: 1000.0,
                    },
                ],
                padding: 1,
            },
        ];

        let right = vec![
            ContentBlock {
                items: vec![
                    ContentItem::Text("「 SKILLS 」".into()),
                    ContentItem::Rule,
                    ContentItem::Text("typespec ···· 12".into()),
                    ContentItem::Text("ast-grep ···· 5".into()),
                    ContentItem::Text("tree-sit ···· 3".into()),
                    ContentItem::Text("alloy    ···· 2".into()),
                    ContentItem::Rule,
                    ContentItem::Text("◁━━ 43 LOADED".into()),
                ],
                padding: 1,
            },
            ContentBlock {
                items: vec![
                    ContentItem::Text("TASKS".into()),
                    ContentItem::Rule,
                    ContentItem::Text("▪ layout engine".into()),
                    ContentItem::Text("▪ masonry fills".into()),
                    ContentItem::Text("▪ yaml parsing".into()),
                    ContentItem::Text("▫ snapshot tests".into()),
                    ContentItem::Text("▫ fret connect".into()),
                ],
                padding: 1,
            },
        ];

        let _rects = layout_two_col(&mut grid, &left, &right, 4, 2, palette[4], palette[3]);

        draw_flower(&mut grid, width / 2, 3, rng.random_range(0..5), palette[3]);
        draw_flower(
            &mut grid,
            width / 2,
            height - 4,
            rng.random_range(0..5),
            palette[3],
        );
        draw_flower(&mut grid, 1, height / 2, rng.random_range(0..5), palette[2]);
        draw_flower(
            &mut grid,
            width - 2,
            height / 2,
            rng.random_range(0..5),
            palette[2],
        );
    } else if mode == "md" {
        let mut input = String::new();
        io::stdin().read_to_string(&mut input).unwrap_or_default();
        let blocks = parse_markdown(&input);

        if blocks.is_empty() {
            eprintln!("no content on stdin. usage: echo '# Title' | ascii-renderer 42 md [theme]");
            std::process::exit(1);
        }

        let truchet_color = darken(palette[1], 90);
        let tiles = ['╱', '╲'];
        for y in 0..height {
            for x in 0..width {
                grid[y][x] = Cell::new(tiles[rng.random_range(0..2)], truchet_color);
            }
        }

        let border_band = if width > 40 && height > 20 { 3 } else { 0 };
        let content_margin = border_band + 1;

        let rects = if blocks.len() <= 2 {
            let col_w = width.saturating_sub(content_margin * 2);
            let mut cy = content_margin;
            let mut rects = Vec::new();
            for block in &blocks {
                let (_, h) = measure_block(block, col_w);
                let h = h.min(height.saturating_sub(cy + content_margin));
                if h == 0 {
                    break;
                }
                let rect = Rect {
                    x: content_margin,
                    y: cy,
                    w: col_w,
                    h,
                };
                render_block(&mut grid, block, &rect, palette[4], palette[3]);
                rects.push(rect);
                cy += h + 1;
            }
            rects
        } else {
            layout_bsp(
                &mut grid,
                &blocks,
                content_margin,
                14,
                4,
                palette[4],
                palette[3],
                &mut rng,
            )
        };

        let content_count = blocks.len().min(rects.len());
        for i in 0..content_count {
            let style = borders::pick_border_style(&mut rng, rects[i].w, rects[i].h);
            borders::draw_box_border(&mut grid, &rects[i], &style, palette[4]);
        }

        let empty_leaves: Vec<Rect> = rects.into_iter().skip(content_count).collect();
        walk_and_fill_leaves(&mut grid, &empty_leaves, &palette, &mut rng);

        if width > 40 && height > 20 {
            let band = 3;
            draw_fret_border(&mut grid, 0, 0, width, height, band, 0, palette[2]);
            draw_fret_border(&mut grid, 0, 0, width, height, band, 1, palette[2]);
            draw_fret_border(&mut grid, 0, 0, width, height, band, 2, palette[2]);
            draw_fret_border(&mut grid, 0, 0, width, height, band, 3, palette[2]);
        }
    } else if mode == "bsp" {
        let truchet_color = darken(palette[1], 90);
        let tiles = ['╱', '╲'];
        for y in 0..height {
            for x in 0..width {
                grid[y][x] = Cell::new(tiles[rng.random_range(0..2)], truchet_color);
            }
        }

        let blocks = vec![
            ContentBlock {
                items: vec![
                    ContentItem::Text("「 STATUS 」".into()),
                    ContentItem::Rule,
                    ContentItem::Text("All systems operational.".into()),
                    ContentItem::Text("Last deploy 2h ago.".into()),
                ],
                padding: 1,
            },
            ContentBlock {
                items: vec![
                    ContentItem::Text("METRICS".into()),
                    ContentItem::Rule,
                    ContentItem::Bar { label: "cpu".into(), value: 72.0, max: 100.0 },
                    ContentItem::Bar { label: "mem".into(), value: 4.8, max: 8.0 },
                    ContentItem::Bar { label: "disk".into(), value: 120.0, max: 500.0 },
                ],
                padding: 1,
            },
            ContentBlock {
                items: vec![
                    ContentItem::Text("「 SKILLS 」".into()),
                    ContentItem::Rule,
                    ContentItem::Text("typespec ···· 12".into()),
                    ContentItem::Text("ast-grep ···· 5".into()),
                    ContentItem::Text("tree-sit ···· 3".into()),
                    ContentItem::Text("alloy    ···· 2".into()),
                ],
                padding: 1,
            },
            ContentBlock {
                items: vec![
                    ContentItem::Text("TASKS".into()),
                    ContentItem::Rule,
                    ContentItem::Text("▪ layout engine".into()),
                    ContentItem::Text("▪ masonry fills".into()),
                    ContentItem::Text("▫ yaml parsing".into()),
                ],
                padding: 1,
            },
            ContentBlock {
                items: vec![
                    ContentItem::Text("NOTES".into()),
                    ContentItem::Rule,
                    ContentItem::Text("BSP splits the canvas into randomized regions. Each content block gets assigned to the largest available leaf. Remaining leaves stay as pattern fill.".into()),
                ],
                padding: 1,
            },
        ];

        let rects = layout_bsp(
            &mut grid, &blocks, 1, 12, 5, palette[4], palette[3], &mut rng,
        );

        for rect in rects.iter().skip(blocks.len()) {
            let cx = rect.x + rect.w / 2;
            let cy = rect.y + rect.h / 2;
            if rect.w >= 5 && rect.h >= 3 {
                draw_flower(&mut grid, cx, cy, rng.random_range(0..5), palette[3]);
            }
        }
    } else if mode == "mondrian" {
        let line_w = 2;

        let mut stdin_buf = String::new();
        let has_stdin = !std::io::stdin().is_terminal();
        if has_stdin {
            io::stdin()
                .read_to_string(&mut stdin_buf)
                .unwrap_or_default();
        }

        let blocks = if !stdin_buf.is_empty() {
            parse_markdown(&stdin_buf)
        } else {
            let status_msgs = [
                "All systems nominal.",
                "Drift detected. Compensating.",
                "Awaiting signal.",
                "Calibrating.",
                "Standing by.",
                "Online.",
                "Synchronizing.",
                "Lattice stable.",
            ];
            let task_sets: [&[&str]; 4] = [
                &["▪ layout engine", "▪ masonry fills", "▫ fret connect"],
                &["▪ wave collapse", "▪ L-systems", "▫ snapshot tests"],
                &["▪ signal graph", "▪ render pass", "▫ cache layer"],
                &["▪ parse phase", "▪ emit codegen", "▫ type resolve"],
            ];
            let stat = status_msgs[rng.random_range(0..status_msgs.len())];
            let tasks = task_sets[rng.random_range(0..task_sets.len())];

            let cpu_v = rng.random_range(20..95) as f64;
            let mem_v = rng.random_range(10..80) as f64 / 10.0;
            let disk_v = rng.random_range(30..450) as f64;
            let net_v = rng.random_range(50..900) as f64;

            let mut b = vec![
                ContentBlock {
                    items: vec![
                        ContentItem::Text("「 STATUS 」".into()),
                        ContentItem::Rule,
                        ContentItem::Text(stat.into()),
                    ],
                    padding: 1,
                },
                ContentBlock {
                    items: vec![
                        ContentItem::Text("METRICS".into()),
                        ContentItem::Rule,
                        ContentItem::Bar {
                            label: "cpu".into(),
                            value: cpu_v,
                            max: 100.0,
                        },
                        ContentItem::Bar {
                            label: "mem".into(),
                            value: mem_v,
                            max: 8.0,
                        },
                        ContentItem::Bar {
                            label: "disk".into(),
                            value: disk_v,
                            max: 500.0,
                        },
                        ContentItem::Bar {
                            label: "net".into(),
                            value: net_v,
                            max: 1000.0,
                        },
                    ],
                    padding: 1,
                },
            ];
            let mut task_items = vec![ContentItem::Text("TASKS".into()), ContentItem::Rule];
            for t in tasks {
                task_items.push(ContentItem::Text((*t).into()));
            }
            b.push(ContentBlock {
                items: task_items,
                padding: 1,
            });

            if rng.random_range(0..3) == 0 {
                let notes = [
                    "The map is not the territory.",
                    "Form follows function, but function follows context.",
                    "Every system is perfectly designed to produce the results it gets.",
                    "Constraints breed creativity.",
                ];
                b.push(ContentBlock {
                    items: vec![
                        ContentItem::Text("NOTES".into()),
                        ContentItem::Rule,
                        ContentItem::Text(notes[rng.random_range(0..notes.len())].into()),
                    ],
                    padding: 1,
                });
            }
            b
        };

        let fill_colors = if theme_name.is_empty() {
            let (fills, _) = mondrian_colors();
            fills
        } else {
            [
                lighten(palette[0], 40),
                palette[1],
                palette[2],
                palette[3],
                lighten(palette[0], 40),
            ]
        };
        let line_color = if theme_name.is_empty() {
            rgb(20, 20, 20)
        } else {
            darken(palette[0], 60)
        };
        let text_fg = if theme_name.is_empty() {
            rgb(20, 20, 20)
        } else {
            palette[4]
        };

        let rects = layout_mondrian(
            &mut grid,
            &blocks,
            0,
            line_w,
            12,
            5,
            text_fg,
            text_fg,
            &fill_colors,
            line_color,
            &mut rng,
        );

        let content_count = blocks.len().min(rects.len());
        let empty_leaves: Vec<Rect> = rects.into_iter().skip(content_count).collect();
        walk_and_fill_leaves(&mut grid, &empty_leaves, &palette, &mut rng);
    } else if mode == "tiles" {
        let names = [
            "asanoha",
            "seigaiha",
            "shippo",
            "bishamon",
            "yabane",
            "nowaki",
            "higaki",
            "shell",
            "granny",
            "crocodile",
        ];
        let cols = 5.min(TILE_VARIANT_COUNT);
        let rows = (TILE_VARIANT_COUNT + cols - 1) / cols;
        let cell_w = width / cols;
        let cell_h = height / rows;
        for i in 0..TILE_VARIANT_COUNT {
            let col = i % cols;
            let row = i / cols;
            let x0 = col * cell_w;
            let y0 = row * cell_h;
            let r = Rect {
                x: x0,
                y: y0 + 1,
                w: cell_w,
                h: cell_h.saturating_sub(1),
            };
            let variant = tile_variant_from_index(i);
            let c1 = palette[(i % 3) + 1];
            let c2 = darken(c1, 30);
            fill_tile_pure(&mut grid, &r, variant, c1, c2);
            for (j, ch) in names[i].chars().enumerate() {
                if x0 + j < width && y0 < height {
                    grid[y0][x0 + j] = Cell::new(ch, palette[4]);
                }
            }
        }
    } else if mode == "tiles-rand" {
        let names = [
            "asanoha",
            "seigaiha",
            "shippo",
            "bishamon",
            "yabane",
            "nowaki",
            "higaki",
            "shell",
            "granny",
            "crocodile",
        ];
        let cols = 5.min(TILE_VARIANT_COUNT);
        let rows = (TILE_VARIANT_COUNT + cols - 1) / cols;
        let cell_w = width / cols;
        let cell_h = height / rows;
        for i in 0..TILE_VARIANT_COUNT {
            let col = i % cols;
            let row = i / cols;
            let x0 = col * cell_w;
            let y0 = row * cell_h;
            let r = Rect {
                x: x0,
                y: y0 + 1,
                w: cell_w,
                h: cell_h.saturating_sub(1),
            };
            let mut params = TileParams::randomized(&mut rng);
            params.variant = tile_variant_from_index(i);
            let c1 = palette[(i % 3) + 1];
            let c2 = darken(c1, 30);
            let jitter = rng.random_range(0..15) as f32 / 100.0;
            fill_tile_ex(&mut grid, &r, &params, c1, c2, jitter, None, &mut rng);
            let label = format!(
                "{} d{:.0} s{} r{}",
                names[i],
                params.density * 100.0,
                params.stagger_override,
                params.rhythm_override,
            );
            for (j, ch) in label.chars().enumerate() {
                if x0 + j < width && y0 < height {
                    grid[y0][x0 + j] = Cell::new(ch, palette[4]);
                }
            }
        }
    } else if mode == "tiles-skew" {
        let names = [
            "asanoha", "seigaiha", "shippo", "bishamon", "yabane",
            "nowaki", "higaki", "shell", "granny", "crocodile",
        ];
        let cols = 5.min(TILE_VARIANT_COUNT);
        let rows = (TILE_VARIANT_COUNT + cols - 1) / cols;
        let cell_w = width / cols;
        let cell_h = height / rows;
        let inset = 4; // shrink rect so bleed has room to show
        for i in 0..TILE_VARIANT_COUNT {
            let col = i % cols;
            let row = i / cols;
            let x0 = col * cell_w + inset;
            let y0 = row * cell_h + 2;
            let r = Rect {
                x: x0,
                y: y0,
                w: cell_w.saturating_sub(inset * 2),
                h: cell_h.saturating_sub(4),
            };
            let mut params = TileParams::new(tile_variant_from_index(i));
            params.skew = 80;
            let c1 = palette[(i % 3) + 1];
            let c2 = darken(c1, 30);
            fill_tile_ex(&mut grid, &r, &params, c1, c2, 0.0, None, &mut rng);
            let label = format!("{} skew=80", names[i]);
            let lx = col * cell_w;
            let ly = row * cell_h;
            for (j, ch) in label.chars().enumerate() {
                if lx + j < width && ly < height {
                    grid[ly][lx + j] = Cell::new(ch, palette[4]);
                }
            }
        }
    } else if mode == "terrain" {
        let rect = Rect { x: 0, y: 0, w: width, h: height };
        render_terrain(&mut grid, &rect, &palette, &mut rng);
    } else if mode == "flow" {
        let rect = Rect { x: 0, y: 0, w: width, h: height };
        let zones = random_flow(&rect, &palette, &mut rng);
        render_flow(&mut grid, &rect, &zones, &palette, &mut rng);
    } else if mode == "masks" {
        // background: diamond lattice to recreate the emergent effect
        let bg_rect = Rect { x: 0, y: 0, w: width, h: height };
        draw_diamond_lattice(&mut grid, &bg_rect, darken(palette[1], 60), darken(palette[1], 80));
        let labels = ["circle", "eye", "diamond", "square"];
        for i in 0..MASK_STYLE_COUNT {
            let cx = (width / (MASK_STYLE_COUNT + 1)) * (i + 1);
            let cy = height / 2;
            let size = (height / 6).max(2).min(4);
            draw_mask(&mut grid, cx, cy, size, i, palette[(i % 3) + 1]);
            for (j, ch) in labels[i].chars().enumerate() {
                let lx = cx.saturating_sub(labels[i].len() / 2) + j;
                let ly = cy + size + 4;
                if lx < width && ly < height {
                    grid[ly][lx] = Cell::new(ch, palette[4]);
                }
            }
        }
    } else if mode == "ca" || (mode.starts_with("ca-") && mode != "ca-layout") {
        // ca, ca-life, ca-cave, ca-maze, ca-coral, ca-B3/S23
        let rule_name = if mode == "ca" { "life" } else { &mode[3..] };

        // Derive style from seed for variety
        let style = match seed % 4 {
            0 => GlyphStyle::Box,
            1 => GlyphStyle::Round,
            2 => GlyphStyle::Diagonal,
            _ => GlyphStyle::Heavy,
        };

        let (density, gens) = match rule_name {
            "cave"  => (0.50, 5),
            "maze"  => (0.38, 12),
            "coral" => (0.50, 8),
            _       => (0.30, 8),
        };

        let rect = Rect { x: 0, y: 0, w: width, h: height };
        render_automata(
            &mut grid, &rect, rule_name, density, gens,
            style, &palette, true, &mut rng,
        );
    } else if mode == "ca-layout" {
        let rect = Rect { x: 0, y: 0, w: width, h: height };

        // Content blocks to place in the largest CA regions
        let blocks = vec![
            ContentBlock {
                items: vec![
                    ContentItem::Text("「 STATUS 」".into()),
                    ContentItem::Rule,
                    ContentItem::Text("All systems operational.".into()),
                ],
                padding: 1,
            },
            ContentBlock {
                items: vec![
                    ContentItem::Text("METRICS".into()),
                    ContentItem::Rule,
                    ContentItem::Bar { label: "cpu".into(), value: 72.0, max: 100.0 },
                    ContentItem::Bar { label: "mem".into(), value: 4.8, max: 8.0 },
                ],
                padding: 1,
            },
            ContentBlock {
                items: vec![
                    ContentItem::Text("「 SKILLS 」".into()),
                    ContentItem::Rule,
                    ContentItem::Text("typespec ···· 12".into()),
                    ContentItem::Text("ast-grep ···· 5".into()),
                ],
                padding: 1,
            },
        ];

        let text_rects = ca_layout(
            &mut grid, &rect, "life", 0.35, 6, &palette, &mut rng,
        );

        // Render text content into the largest CA regions
        let mut placed = 0;
        for block in &blocks {
            // Find next region large enough for this block
            let (min_w, min_h) = measure_block(block, 40);
            let min_w = min_w.max(12);
            while placed < text_rects.len() {
                let r = &text_rects[placed];
                placed += 1;
                if r.w >= min_w && r.h >= min_h + 2 {
                    // Clear and render
                    for y in r.y..r.y + r.h {
                        for x in r.x..r.x + r.w {
                            if y < height && x < width {
                                grid[y][x] = Cell::blank();
                            }
                        }
                    }
                    render_block(&mut grid, block, r, palette[4], palette[3]);
                    let style = borders::pick_border_style(&mut rng, r.w, r.h);
                    borders::draw_box_border(&mut grid, r, &style, palette[4]);
                    break;
                }
            }
        }
    } else if mode == "shapes" {
        // 2x2 grid, shapes sized to ~30% of each quadrant, hard edges (dissolve=0).
        // rx = 2*ry throughout to correct for 2:1 terminal cell aspect ratio.
        let hw = width / 2;
        let hh = height / 2;
        let cxs = [hw / 2, hw + hw / 2];
        let cys = [hh / 2, hh + hh / 2];

        // label just above the shape
        let write_label = |grid: &mut Grid, lx: usize, ly: usize, text: &str, color: Color| {
            for (j, ch) in text.chars().enumerate() {
                if lx + j < width && ly < grid.len() { grid[ly][lx + j] = Cell::new(ch, color); }
            }
        };

        // 1 -- Diamond (top-left)
        {
            let cx = cxs[0] as f32; let cy = cys[0] as f32;
            let ry = hh as f32 * 0.30;
            let rx = ry * 2.0;
            let r = Rect { x: 1, y: 1, w: hw - 2, h: hh - 2 };
            let scene = Scene { layers: vec![Layer {
                fill: FillGen::Tile(TileParams::new(TileVariant::BishamonKikko)),
                mask: Some(Box::new(mask_diamond(cx, cy, rx, ry, 0.0))),
                palette,
            }]};
            render_scene(&mut grid, &r, &scene, &mut rng);
            let lx = cxs[0].saturating_sub(3);
            let ly = (cy - ry - 2.0).max(1.0) as usize;
            write_label(&mut grid, lx, ly, "diamond", palette[4]);
        }

        // 2 -- Parallelogram (top-right)
        {
            let cx = cxs[1] as f32; let cy = cys[0] as f32;
            let w = hw as f32 * 0.50;
            let h = hh as f32 * 0.55;
            let r = Rect { x: hw + 1, y: 1, w: hw - 2, h: hh - 2 };
            let scene = Scene { layers: vec![Layer {
                fill: FillGen::Tile(TileParams::new(TileVariant::Asanoha)),
                mask: Some(Box::new(mask_parallelogram(cx, cy, w, h, 8.0, 0.0))),
                palette,
            }]};
            render_scene(&mut grid, &r, &scene, &mut rng);
            let lx = cxs[1].saturating_sub(6);
            let ly = (cy - h * 0.5 - 2.0).max(1.0) as usize;
            write_label(&mut grid, lx, ly, "parallelogram", palette[4]);
        }

        // 3 -- Triangle apex-up (bottom-left)
        {
            let cx = cxs[0] as f32; let cy = cys[1] as f32;
            let ry = hh as f32 * 0.35;
            let rx = ry * 2.0;
            let r = Rect { x: 1, y: hh + 1, w: hw - 2, h: hh - 2 };
            let scene = Scene { layers: vec![Layer {
                fill: FillGen::Tile(TileParams::new(TileVariant::Yabane)),
                mask: Some(Box::new(mask_triangle(cx, cy, rx, ry, TriDir::Up, 0.0))),
                palette,
            }]};
            render_scene(&mut grid, &r, &scene, &mut rng);
            let lx = cxs[0].saturating_sub(3);
            let ly = (cy - ry - 2.0).max((hh + 1) as f32) as usize;
            write_label(&mut grid, lx, ly, "triangle", palette[4]);
        }

        // 4 -- Trapezoid wide-at-bottom (bottom-right)
        {
            let cx = cxs[1] as f32; let cy = cys[1] as f32;
            let h = hh as f32 * 0.55;
            let w_top = hw as f32 * 0.12;
            let w_bot = hw as f32 * 0.55;
            let r = Rect { x: hw + 1, y: hh + 1, w: hw - 2, h: hh - 2 };
            let scene = Scene { layers: vec![Layer {
                fill: FillGen::Tile(TileParams::new(TileVariant::Higaki)),
                mask: Some(Box::new(mask_trapezoid(cx, cy, w_top, w_bot, h, 0.0))),
                palette,
            }]};
            render_scene(&mut grid, &r, &scene, &mut rng);
            let lx = cxs[1].saturating_sub(4);
            let ly = (cy - h * 0.5 - 2.0).max((hh + 1) as f32) as usize;
            write_label(&mut grid, lx, ly, "trapezoid", palette[4]);
        }

        // grid dividers
        for y in 0..height { if y < grid.len() { grid[y][hw] = Cell::new('│', darken(palette[2], 50)); } }
        for x in 0..width { if hh < grid.len() { grid[hh][x] = Cell::new('─', darken(palette[2], 50)); } }
        if hh < grid.len() { grid[hh][hw] = Cell::new('┼', darken(palette[2], 50)); }

    } else if mode == "party" {
        // party [gap] [nodes] [scale] [detail] [weather] [path] [atmo]
        let pp = PartyParams {
            gap:    args.get(4).and_then(|s| s.parse().ok()).unwrap_or(0),
            nodes:  args.get(5).and_then(|s| s.parse().ok()).unwrap_or(0),
            scale:  args.get(6).and_then(|s| s.parse().ok()).unwrap_or(50),
            detail: args.get(7).and_then(|s| s.parse().ok()).unwrap_or(50),
        };
        let weather = args.get(8)
            .and_then(|s| Weather::from_name(s))
            .unwrap_or_else(|| Weather::pick(&mut rng));
        let path_style = args.get(9)
            .and_then(|s| PathStyle::from_name(s))
            .unwrap_or_else(|| PathStyle::pick(&mut rng));
        let atmo_intensity: u32 = args.get(10).and_then(|s| s.parse().ok()).unwrap_or(50);
        let rect = Rect { x: 0, y: 0, w: width, h: height };
        let (layers, stops, boxes) = party_walk(width, height, &palette, &pp, &mut rng);
        let scene = Scene { layers };
        render_scene(&mut grid, &rect, &scene, &mut rng);
        // Draw connecting path between node centers
        draw_styled_path(&mut grid, &stops, path_style, darken(palette[2], 30), &mut rng);
        // Draw box borders around each node
        let border_color = palette[4];
        for &(bx, by, bw, bh) in &boxes {
            draw_box_border(&mut grid, bx, by, bw, bh, border_color);
        }
        // Weather overlay
        apply_atmosphere(&mut grid, weather, atmo_intensity, &palette, &mut rng);
    } else if mode == "soup" {
        let rect = Rect { x: 0, y: 0, w: width, h: height };
        let (layers, stops) = soup_walk(width, height, &palette, &mut rng);
        let scene = Scene { layers };
        render_scene(&mut grid, &rect, &scene, &mut rng);
        draw_path_trail(&mut grid, &stops, palette[2], &mut rng);
    } else if mode == "stem" {
        let rect = Rect { x: 0, y: 0, w: width, h: height };
        let (layers, spine) = path_walk_stem(width, height, &palette, &mut rng);
        let scene = Scene { layers };
        render_scene(&mut grid, &rect, &scene, &mut rng);
        draw_stalk(&mut grid, &spine, palette[2]);
    } else if mode == "scene-walk" {
        let rect = Rect { x: 0, y: 0, w: width, h: height };
        let layers = path_walk_layers(width, height, &palette, &mut rng);
        let scene = Scene { layers };
        render_scene(&mut grid, &rect, &scene, &mut rng);
    } else if mode == "scene-walk-2" {
        let rect = Rect { x: 0, y: 0, w: width, h: height };
        let (layers, stops) = path_walk_layers_2(width, height, &palette, &mut rng);
        let scene = Scene { layers };
        render_scene(&mut grid, &rect, &scene, &mut rng);
        draw_path_trail(&mut grid, &stops, palette[2], &mut rng);
    } else if mode == "scene-walk-3" {
        let rect = Rect { x: 0, y: 0, w: width, h: height };
        let density = 50u32;
        let (layers, stops, _boxes) = path_walk_layers_3(width, height, &palette, density, &mut rng);
        let scene = Scene { layers };
        render_scene(&mut grid, &rect, &scene, &mut rng);
        draw_path_trail(&mut grid, &stops, palette[2], &mut rng);
    } else if mode == "forest2" {
        // Ground: truchet dirt
        let ground_color = darken(palette[1], 90);
        let tiles = ['╱', '╲'];
        for y in 0..height {
            for x in 0..width {
                grid[y][x] = Cell::new(tiles[rng.random_range(0..2)], ground_color);
            }
        }

        let ground_y = height.saturating_sub(4);

        // Place trees with varied size, position, and type
        let tree_count = rng.random_range(4..9u32) as usize;
        struct TreeSlot { x: usize, kind: usize, spread: usize, canopy_y: usize }
        let mut slots: Vec<TreeSlot> = Vec::new();

        // One big centerpiece tree
        let big_x = rng.random_range((width / 4) as u32..(width * 3 / 4) as u32) as usize;
        let big_spread = rng.random_range(8..14u32) as usize;
        let big_canopy = rng.random_range(3..6u32) as usize;
        let big_kind = rng.random_range(0..12u32) as usize;
        slots.push(TreeSlot { x: big_x, kind: big_kind, spread: big_spread, canopy_y: big_canopy });

        // Remaining trees scattered, varied sizes
        for _ in 0..tree_count - 1 {
            let tx = rng.random_range(6..(width - 6) as u32) as usize;
            let spread = rng.random_range(3..9u32) as usize;
            let canopy = rng.random_range(4..ground_y.saturating_sub(6).max(5) as u32) as usize;
            let kind = rng.random_range(0..12u32) as usize;
            slots.push(TreeSlot { x: tx, kind: kind, spread: spread, canopy_y: canopy });
        }

        // Sort by x so they layer left to right
        slots.sort_by_key(|s| s.x);

        for slot in &slots {
            // Clear space for this tree
            let clear_left = slot.x.saturating_sub(slot.spread + 2);
            let clear_right = (slot.x + slot.spread + 2).min(width);
            for y in slot.canopy_y.saturating_sub(1)..ground_y + 2 {
                for x in clear_left..clear_right {
                    if y < height && x < width {
                        grid[y][x] = Cell::blank();
                    }
                }
            }
            let color = palette[rng.random_range(1..4)];
            draw_tree(&mut grid, slot.x, ground_y - 1, slot.canopy_y, slot.spread, slot.kind, color, &mut rng);
        }

        // Flower/fruit burst radiating from the biggest tree's base
        let burst_cx = big_x;
        let burst_cy = ground_y + 1;
        let burst_count = rng.random_range(5..12u32);
        // One big flower at center
        draw_flower(&mut grid, burst_cx, burst_cy, rng.random_range(0..5), palette[3]);
        // Radial scatter around it
        for _ in 0..burst_count {
            let angle = rng.random::<f32>() * std::f32::consts::TAU;
            let radius = rng.random_range(3..16u32) as f32;
            let fx = (burst_cx as f32 + angle.cos() * radius * 1.8) as i32; // aspect correction
            let fy = (burst_cy as f32 + angle.sin() * radius * 0.6) as i32;
            if fx >= 2 && fy >= 2 && (fx as usize) < width - 2 && (fy as usize) < height - 2 {
                if rng.random_range(0..3u32) == 0 {
                    draw_fruit(&mut grid, fx as usize, fy as usize, rng.random_range(0..5), palette[rng.random_range(2..4)]);
                } else {
                    draw_flower(&mut grid, fx as usize, fy as usize, rng.random_range(0..5), palette[rng.random_range(2..4)]);
                }
            }
        }

        // Scatter a few more flower clusters near other trees
        for slot in &slots {
            let count = rng.random_range(1..4u32);
            for _ in 0..count {
                let fx = (slot.x as i32 + rng.random_range(-6..7i32)) as usize;
                let fy = ground_y + rng.random_range(0..2u32) as usize;
                if fx >= 2 && fx < width - 2 && fy < height - 2 {
                    if rng.random_range(0..2u32) == 0 {
                        draw_flower(&mut grid, fx, fy, rng.random_range(0..5), palette[rng.random_range(2..4)]);
                    } else {
                        draw_fruit(&mut grid, fx, fy, rng.random_range(0..5), palette[rng.random_range(2..4)]);
                    }
                }
            }
        }

    } else if mode == "forest3" {
        // Background: sky (sparse dots) above horizon, ground (truchet) below
        let horizon = height * 2 / 3 + rng.random_range(0..(height / 8).max(1) as u32) as usize;
        let sky_color = darken(palette[0], 95);
        let ground_color = darken(palette[1], 85);
        let ground_tiles = ['╱', '╲', '·', '·'];

        // Sky: sparse scattered dots
        for y in 0..horizon {
            for x in 0..width {
                if rng.random_range(0..12u32) == 0 {
                    grid[y][x] = Cell::new('·', sky_color);
                }
            }
        }
        // Ground: truchet with some grass chars mixed in
        let grass_chars = ['╌', '╌', '∿', '~', '·'];
        for y in horizon..height {
            for x in 0..width {
                let depth = y - horizon;
                if depth < 2 {
                    // Grass transition line
                    grid[y][x] = Cell::new(
                        grass_chars[rng.random_range(0..grass_chars.len() as u32) as usize],
                        lighten(ground_color, 20),
                    );
                } else {
                    grid[y][x] = Cell::new(
                        ground_tiles[rng.random_range(0..ground_tiles.len() as u32) as usize],
                        darken(ground_color, (depth * 3) as u8),
                    );
                }
            }
        }

        // Tree placement: staggered roots, varied sizes
        let tree_count = rng.random_range(5..10u32) as usize;
        struct TreeSlot { x: usize, root_y: usize, kind: usize, spread: usize, canopy_y: usize }
        let mut slots: Vec<TreeSlot> = Vec::new();

        // One kaiju tree (kind 13 = grow_kaiju_tree in the dispatch)
        let kaiju_x = rng.random_range((width / 6) as u32..(width * 5 / 6) as u32) as usize;
        let kaiju_root = horizon + rng.random_range(0..3u32) as usize;
        let kaiju_spread = rng.random_range(12..20u32) as usize;
        let kaiju_canopy = rng.random_range(2..5u32) as usize;
        slots.push(TreeSlot {
            x: kaiju_x, root_y: kaiju_root, kind: 13, spread: kaiju_spread, canopy_y: kaiju_canopy,
        });

        // Remaining trees: staggered roots along horizon zone
        for _ in 0..tree_count - 1 {
            let tx = rng.random_range(4..(width - 4) as u32) as usize;
            let root_offset = rng.random_range(0..5u32) as usize; // roots at different depths
            let root_y = horizon + root_offset;
            if root_y >= height - 1 { continue; }
            let spread = rng.random_range(3..10u32) as usize;
            let tree_height = rng.random_range(8..(root_y.saturating_sub(2).max(9)) as u32) as usize;
            let canopy_y = root_y.saturating_sub(tree_height).max(1);
            // Favor asymmetric/storm/dead kinds (9, 7, 12) alongside others
            let kind = rng.random_range(0..14u32) as usize;
            slots.push(TreeSlot { x: tx, root_y, kind, spread, canopy_y });
        }

        // Sort by root_y descending so farther trees draw first (back to front)
        slots.sort_by(|a, b| a.root_y.cmp(&b.root_y).then(a.x.cmp(&b.x)));

        // Draw trees directly on background -- no clearing rectangles
        for slot in &slots {
            let color = palette[rng.random_range(1..5)];
            draw_tree(&mut grid, slot.x, slot.root_y, slot.canopy_y, slot.spread, slot.kind, color, &mut rng);
        }

        // Scatter flowers/fruit along the ground, clustering near tree bases
        for slot in &slots {
            let burst_count = rng.random_range(1..5u32);
            for _ in 0..burst_count {
                let angle = rng.random::<f32>() * std::f32::consts::TAU;
                let radius = rng.random_range(2..10u32) as f32;
                let fx = (slot.x as f32 + angle.cos() * radius * 1.5) as i32;
                let fy = (slot.root_y as f32 + angle.sin() * radius * 0.4 + 1.0) as i32;
                if fx >= 1 && fy >= 1 && (fx as usize) < width - 1 && (fy as usize) < height - 1 {
                    let c = palette[rng.random_range(2..5)];
                    if rng.random_range(0..3u32) == 0 {
                        draw_fruit(&mut grid, fx as usize, fy as usize, rng.random_range(0..5), c);
                    } else {
                        draw_flower(&mut grid, fx as usize, fy as usize, rng.random_range(0..5), c);
                    }
                }
            }
        }

    } else if mode == "forest4" {
        // Like forest3 but with wild/unbalanced trees and algorithmic sprites.
        // More trees planted lower, more ground coverage.
        // Horizon at 60-80% down the screen (more sky, less grass domination)
        let horizon = height * 3 / 5 + rng.random_range(0..(height / 5).max(1) as u32) as usize;
        let sky_color = darken(palette[0], 95);
        let ground_color = darken(palette[1], 80);

        // Sky: sparse dots
        for y in 0..horizon {
            for x in 0..width {
                if rng.random_range(0..15u32) == 0 {
                    grid[y][x] = Cell::new('·', sky_color);
                }
            }
        }
        // Clouds: 1-4 in the upper sky
        let cloud_count = rng.random_range(1..5u32);
        let cloud_color = lighten(palette[0], 15);
        for _ in 0..cloud_count {
            let cx = rng.random_range(5..(width - 5) as u32) as usize;
            let cy = rng.random_range(2..(horizon / 2).max(3) as u32) as usize;
            let cw = rng.random_range(8..20u32) as usize;
            draw_cloud(&mut grid, cx, cy, cw, cloud_color, &mut rng);
        }
        // Per-column ground height: random walk so the grass edge is ragged
        let jitter_range = rng.random_range(2..6u32) as i32; // how wild the edge gets
        let mut ground_heights: Vec<usize> = Vec::with_capacity(width);
        let mut gh = horizon as i32;
        for _ in 0..width {
            gh += rng.random_range(0..3u32) as i32 - 1; // random walk: -1, 0, or +1
            gh = gh.clamp(horizon as i32 - jitter_range, horizon as i32 + jitter_range);
            ground_heights.push(gh.max(1) as usize);
        }

        // Ground: hue gradient with random direction sweeping across
        let ground_chars = ['╱', '╲', '·', '∿', '~'];
        let ground_depth = (height - horizon).max(1);
        // Random gradient direction
        let grad_dir = rng.random_range(0..6u32);
        // Base hue from palette
        let ground_base_hue: f64 = if let Color::Rgb { r, g, .. } = ground_color {
            (r as f64 * 1.4 + g as f64 * 0.7) % 360.0
        } else { 120.0 };
        let hue_sweep = rng.random_range(30..80u32) as f64;

        for x in 0..width {
            let col_horizon = ground_heights[x];
            for y in col_horizon..height {
                let depth = y - col_horizon;
                let ch = ground_chars[rng.random_range(0..ground_chars.len() as u32) as usize];

                // Gradient parameter t: 0.0 to 1.0, direction varies per seed
                let t = match grad_dir {
                    0 => x as f64 / width as f64,                    // left to right
                    1 => 1.0 - x as f64 / width as f64,              // right to left
                    2 => depth as f64 / ground_depth as f64,         // top to bottom
                    3 => (x as f64 / width as f64 + depth as f64 / ground_depth as f64) / 2.0, // diagonal ↘
                    4 => ((1.0 - x as f64 / width as f64) + depth as f64 / ground_depth as f64) / 2.0, // diagonal ↙
                    _ => {
                        // Radial from center of ground
                        let cx = width as f64 / 2.0;
                        let cy = ground_depth as f64 / 2.0;
                        let dx = (x as f64 - cx) / cx;
                        let dy = (depth as f64 - cy) / cy.max(1.0);
                        (dx * dx + dy * dy).sqrt().min(1.0)
                    }
                };
                let h = (ground_base_hue + t * hue_sweep).rem_euclid(360.0);
                let l = (0.25 - depth as f64 * 0.006).max(0.10);
                let s = 0.4 + t * 0.2;
                let c = hsl_to_rgb(h, s.min(0.8), l);
                grid[y][x] = Cell::new(ch, c);
            }
        }

        // Tree placement: more trees, wider root stagger
        let tree_count = rng.random_range(5..10u32) as usize;
        struct TreeSlot { x: usize, root_y: usize, kind: usize, spread: usize, canopy_y: usize }
        let mut slots: Vec<TreeSlot> = Vec::new();

        // One kaiju tree -- root at the grass line
        let kaiju_x = rng.random_range((width / 8) as u32..(width * 7 / 8) as u32) as usize;
        let kaiju_root = ground_heights[kaiju_x.min(width - 1)] + rng.random_range(0..3u32) as usize;
        let kaiju_root = kaiju_root.min(height - 2);
        slots.push(TreeSlot {
            x: kaiju_x, root_y: kaiju_root, kind: 13,
            spread: rng.random_range(14..22u32) as usize,
            canopy_y: rng.random_range(1..4u32) as usize,
        });

        // Remaining trees: favor wild (14), asymmetric (9), storm (7), dead (12)
        // Enforce minimum spacing so trees don't pile on top of each other
        let unbalanced_kinds = [14, 14, 9, 9, 7, 7, 12, 13, 15, 15, 16, 17, 17, 4, 5, 6, 11];
        let min_spacing = (width / (tree_count + 1)).max(14);
        for _ in 0..tree_count - 1 {
            let mut tx = 0usize;
            let mut placed = false;
            for _ in 0..10 {
                tx = rng.random_range(3..(width - 3) as u32) as usize;
                let too_close = slots.iter().any(|s| ((s.x as i32 - tx as i32).unsigned_abs() as usize) < min_spacing);
                if !too_close { placed = true; break; }
            }
            if !placed { tx = rng.random_range(3..(width - 3) as u32) as usize; }

            // Root at grass line + small offset so trunk meets the ground
            let grass_y = ground_heights[tx.min(width - 1)];
            let root_offset = rng.random_range(0..4u32) as usize;
            let root_y = (grass_y + root_offset).min(height - 2);

            // Height tiers: some scrubby (3-8), some medium (8-20), some towering (20-root_y)
            let max_possible = root_y.saturating_sub(1).max(4);
            let tree_height = match rng.random_range(0..10u32) {
                0..=2 => rng.random_range(3..8u32.min(max_possible as u32 + 1)) as usize,  // scrubby
                3..=6 => rng.random_range(8..20u32.min(max_possible as u32 + 1)) as usize, // medium
                _ => rng.random_range(20u32.min(max_possible as u32)..max_possible as u32 + 1) as usize, // towering
            };
            let canopy_y = root_y.saturating_sub(tree_height).max(1);

            // Spread also tiered: narrow (1-4), medium (4-10), wide (10-20)
            let spread = match rng.random_range(0..6u32) {
                0..=1 => rng.random_range(1..5u32) as usize,
                2..=4 => rng.random_range(4..11u32) as usize,
                _ => rng.random_range(10..21u32) as usize,
            };

            let kind = unbalanced_kinds[rng.random_range(0..unbalanced_kinds.len() as u32) as usize];
            slots.push(TreeSlot { x: tx, root_y, kind, spread, canopy_y });
        }

        // Back-to-front
        slots.sort_by(|a, b| a.root_y.cmp(&b.root_y).then(a.x.cmp(&b.x)));

        // Give each tree a distinct hue + depth-based brightness
        // Slots are sorted back-to-front (ascending root_y), so earlier = farther = dimmer
        let slot_count = slots.len();
        for (i, slot) in slots.iter().enumerate() {
            let base_hue = (i as f64 * 360.0 / slot_count as f64 + rng.random_range(0..30u32) as f64) % 360.0;
            // Depth factor: 0.0 = farthest (dim), 1.0 = closest (bright)
            let depth_t = i as f64 / (slot_count - 1).max(1) as f64;
            let lightness = 0.2 + depth_t * 0.3; // 0.2 (far) to 0.5 (near)
            let saturation = 0.4 + depth_t * 0.3;
            let color = hsl_to_rgb(base_hue, saturation, lightness);
            draw_tree(&mut grid, slot.x, slot.root_y, slot.canopy_y, slot.spread, slot.kind, color, &mut rng);
        }

        // Sprout braille leaf clusters at branch tips (~50% of tips)
        let leaf_hue = rng.random_range(60..180u32) as f64; // green-ish range
        let leaf_color = hsl_to_rgb(leaf_hue, 0.5, 0.3);
        sprout_leaves(&mut grid, leaf_color, 50, &mut rng);

        // Tighter flower/fruit scatter: fewer per tree, smaller radius, only at ground level
        for slot in &slots {
            let burst = rng.random_range(0..3u32); // 0-2 instead of 2-5
            for _ in 0..burst {
                let angle = rng.random::<f32>() * std::f32::consts::TAU;
                let radius = rng.random_range(1..6u32) as f32; // tighter radius
                let fx = (slot.x as f32 + angle.cos() * radius * 1.5) as i32;
                // Keep at or just below root, not floating in the sky
                let fy = slot.root_y as i32 + rng.random_range(1..3u32) as i32;
                if fx >= 1 && fy >= 1 && (fx as usize) < width - 1 && (fy as usize) < height - 1 {
                    let c = palette[rng.random_range(2..5)];
                    match rng.random_range(0..3u32) {
                        0 => grow_flower_spiral(&mut grid, fx as usize, fy as usize, c, &mut rng),
                        1 => grow_fruit_vine(&mut grid, fx as usize, fy as usize, c, &mut rng),
                        _ => draw_flower(&mut grid, fx as usize, fy as usize, rng.random_range(0..5), c),
                    }
                }
            }
        }

        // Foreground trees: 1-3 trees planted deep in the ground, drawn last (in front)
        let fg_count = rng.random_range(1..4u32);
        for _ in 0..fg_count {
            let tx = rng.random_range(3..(width - 3) as u32) as usize;
            let grass_y = ground_heights[tx.min(width - 1)];
            let root_y = (grass_y + rng.random_range(2..6u32) as usize).min(height - 2);
            let tree_height = rng.random_range(4..12u32) as usize;
            let canopy_y = root_y.saturating_sub(tree_height).max(1);
            let spread = rng.random_range(3..10u32) as usize;
            let kind = rng.random_range(0..18u32) as usize;
            let fg_hue = rng.random_range(0..360u32) as f64;
            let color = hsl_to_rgb(fg_hue, 0.6, 0.4);
            draw_tree(&mut grid, tx, root_y, canopy_y, spread, kind, color, &mut rng);
        }

    } else if mode == "forest5" {
        // Clustered forest: groups of same-family trees with slight color variation.
        // Center tree tallest in each cluster, edges taper. Per-tree tip decoration.
        // Root systems at trunk bases.
        let horizon = height * 3 / 5 + rng.random_range(0..(height / 5).max(1) as u32) as usize;
        let sky_color = darken(palette[0], 95);
        let ground_color = darken(palette[1], 80);

        // Sky: sparse dots
        for y in 0..horizon {
            for x in 0..width {
                if rng.random_range(0..15u32) == 0 {
                    grid[y][x] = Cell::new('·', sky_color);
                }
            }
        }
        // Clouds
        let cloud_count = rng.random_range(1..5u32);
        let cloud_color = lighten(palette[0], 15);
        for _ in 0..cloud_count {
            let cx = rng.random_range(5..(width - 5) as u32) as usize;
            let cy = rng.random_range(2..(horizon / 2).max(3) as u32) as usize;
            let cw = rng.random_range(8..20u32) as usize;
            draw_cloud(&mut grid, cx, cy, cw, cloud_color, &mut rng);
        }

        // Per-column ground height via random walk
        let jitter_range = rng.random_range(2..6u32) as i32;
        let mut ground_heights: Vec<usize> = Vec::with_capacity(width);
        let mut gh = horizon as i32;
        for _ in 0..width {
            gh += rng.random_range(0..3u32) as i32 - 1;
            gh = gh.clamp(horizon as i32 - jitter_range, horizon as i32 + jitter_range);
            ground_heights.push(gh.max(1) as usize);
        }

        // Ground fill with hue gradient
        let ground_chars = ['╱', '╲', '·', '∿', '~'];
        let ground_depth = (height - horizon).max(1);
        let grad_dir = rng.random_range(0..6u32);
        let ground_base_hue: f64 = if let Color::Rgb { r, g, .. } = ground_color {
            (r as f64 * 1.4 + g as f64 * 0.7) % 360.0
        } else { 120.0 };
        let hue_sweep = rng.random_range(30..80u32) as f64;

        for x in 0..width {
            let col_horizon = ground_heights[x];
            for y in col_horizon..height {
                let depth = y - col_horizon;
                let ch = ground_chars[rng.random_range(0..ground_chars.len() as u32) as usize];
                let t = match grad_dir {
                    0 => x as f64 / width as f64,
                    1 => 1.0 - x as f64 / width as f64,
                    2 => depth as f64 / ground_depth as f64,
                    3 => (x as f64 / width as f64 + depth as f64 / ground_depth as f64) / 2.0,
                    4 => ((1.0 - x as f64 / width as f64) + depth as f64 / ground_depth as f64) / 2.0,
                    _ => {
                        let cx = width as f64 / 2.0;
                        let cy = ground_depth as f64 / 2.0;
                        let dx = (x as f64 - cx) / cx;
                        let dy = (depth as f64 - cy) / cy.max(1.0);
                        (dx * dx + dy * dy).sqrt().min(1.0)
                    }
                };
                let h = (ground_base_hue + t * hue_sweep).rem_euclid(360.0);
                let l = (0.25 - depth as f64 * 0.006).max(0.10);
                let s = 0.4 + t * 0.2;
                let c = hsl_to_rgb(h, s.min(0.8), l);
                grid[y][x] = Cell::new(ch, c);
            }
        }

        // --- Cluster placement: 1 dominant tree + 0-2 small companions ---
        // Fewer trees, more breathing room. Each cluster owns a wide horizontal zone.
        let cluster_count = rng.random_range(2..5u32) as usize;
        let zone_width = width / cluster_count.max(1);

        // Mix old tree algos (visual personality) with pen trees (connectivity).
        // Dominant trees use the interesting old kinds, companions use pen trees.
        let dominant_kinds = [0, 7, 9, 13, 14, 15, 17]; // grow_tree, storm, asymmetric, kaiju, wild, zigzag, tendril
        let family_decos = [
            TipDeco::Fruit, TipDeco::Drip, TipDeco::Flower, TipDeco::Fruit,
            TipDeco::Fruit, TipDeco::Drip, TipDeco::Flower,
        ];

        struct PlacedTree { x: usize, root_y: usize, canopy_y: usize, spread: usize, kind: usize, use_pen: bool, is_dominant: bool }
        let mut all_trees: Vec<(PlacedTree, f64, usize)> = Vec::new();

        for ci in 0..cluster_count {
            let dom_kind_idx = rng.random_range(0..dominant_kinds.len() as u32) as usize;
            let dom_kind = dominant_kinds[dom_kind_idx];
            let base_hue = (ci as f64 * 360.0 / cluster_count as f64
                + rng.random_range(0..30u32) as f64) % 360.0;

            // Dominant tree: old algo with visual personality
            let zone_start = zone_width * ci;
            let dom_x = zone_start + zone_width / 2 + rng.random_range(0..(zone_width / 4).max(1) as u32) as usize;
            let dom_x = dom_x.clamp(5, width - 5);
            let grass_y = ground_heights[dom_x.min(width - 1)];
            let dom_root = (grass_y + rng.random_range(2..8u32) as usize).min(height - 2);
            let max_h = dom_root.saturating_sub(3).max(6);
            let dom_h = rng.random_range((max_h as u32 / 2).max(8).min(max_h as u32)..max_h as u32 + 1) as usize;
            let dom_canopy = dom_root.saturating_sub(dom_h).max(1);
            let dom_spread = rng.random_range(8..16u32) as usize;

            all_trees.push((
                PlacedTree { x: dom_x, root_y: dom_root, canopy_y: dom_canopy, spread: dom_spread, kind: dom_kind, use_pen: false, is_dominant: true },
                base_hue,
                dom_kind_idx,
            ));

            // 0-2 small companion trees: pen trees (connected, small)
            let companion_count = rng.random_range(0..3u32);
            for _ in 0..companion_count {
                let offset = rng.random_range(12..25u32) as i32 * if rng.random_range(0..2u32) == 0 { -1 } else { 1 };
                let cx = (dom_x as i32 + offset).clamp(3, width as i32 - 3) as usize;
                let cgrass = ground_heights[cx.min(width - 1)];
                let croot = (cgrass + rng.random_range(1..6u32) as usize).min(height - 2);
                let cmax = croot.saturating_sub(2).max(3);
                let lo = 3u32.min(cmax as u32);
                let hi = (cmax as u32 / 2 + 4).max(lo + 1);
                let ch = rng.random_range(lo..hi) as usize;
                let ccanopy = croot.saturating_sub(ch).max(1);
                let cspread = rng.random_range(2..7u32) as usize;
                let hue_jitter = rng.random_range(0..20u32) as f64 - 10.0;

                all_trees.push((
                    PlacedTree { x: cx, root_y: croot, canopy_y: ccanopy, spread: cspread, kind: 0, use_pen: true, is_dominant: false },
                    base_hue + hue_jitter,
                    dom_kind_idx,
                ));
            }
        }

        // Sort back-to-front
        all_trees.sort_by(|a, b| a.0.root_y.cmp(&b.0.root_y).then(a.0.x.cmp(&b.0.x)));

        let total = all_trees.len();
        for (i, (tree, hue, family_idx)) in all_trees.iter().enumerate() {
            let depth_t = i as f64 / total.max(1) as f64;
            let lightness = 0.22 + depth_t * 0.28;
            let saturation = 0.40 + depth_t * 0.25;
            let color = hsl_to_rgb(*hue, saturation, lightness);

            if tree.use_pen {
                // Companion: pen tree (connected, small)
                let recipe = if rng.random_range(0..2u32) == 0 { TreeRecipe::dead() } else { TreeRecipe::columnar() };
                grow_pen_tree(&mut grid, tree.x, tree.root_y, tree.canopy_y, tree.spread, color, &recipe, &mut rng);
            } else {
                // Dominant: old algo with visual personality
                draw_tree(&mut grid, tree.x, tree.root_y, tree.canopy_y, tree.spread, tree.kind, color, &mut rng);
            }

            // Collect and decorate tips
            let x0 = tree.x.saturating_sub(tree.spread + 5);
            let x1 = (tree.x + tree.spread + 5).min(width);
            let tips = collect_tips_in_rect(&grid, x0, tree.canopy_y, x1, tree.root_y + 1);
            let deco = family_decos[*family_idx];
            let fruit_color = shift_hue(color, 60.0 + rng.random_range(0..40u32) as f64);
            decorate_tips(&mut grid, &tips, deco, fruit_color, 15, &mut rng);
        }

        // Sprout braille leaf clusters
        let leaf_hue = rng.random_range(60..180u32) as f64;
        let leaf_color = hsl_to_rgb(leaf_hue, 0.5, 0.3);
        sprout_leaves(&mut grid, leaf_color, 35, &mut rng);

    } else if mode == "forest6" {
        // Forest6: bespoke pen trees drawn next to their old equivalents for comparison.
        // Reuses forest5 sky/grass/ground layout.

        let horizon = height * 3 / 5 + rng.random_range(0..(height / 5).max(1) as u32) as usize;
        let sky_color = darken(palette[0], 95);
        let ground_color = darken(palette[1], 80);

        // Sky: sparse dots
        for y in 0..horizon {
            for x in 0..width {
                if rng.random_range(0..15u32) == 0 {
                    grid[y][x] = Cell::new('·', sky_color);
                }
            }
        }
        // Clouds
        let cloud_count = rng.random_range(1..5u32);
        let cloud_color = lighten(palette[0], 15);
        for _ in 0..cloud_count {
            let cx = rng.random_range(5..(width - 5) as u32) as usize;
            let cy = rng.random_range(2..(horizon / 2).max(3) as u32) as usize;
            let cw = rng.random_range(8..20u32) as usize;
            draw_cloud(&mut grid, cx, cy, cw, cloud_color, &mut rng);
        }

        // Per-column ground height via random walk
        let jitter_range = rng.random_range(2..6u32) as i32;
        let mut ground_heights: Vec<usize> = Vec::with_capacity(width);
        let mut gh = horizon as i32;
        for _ in 0..width {
            gh += rng.random_range(0..3u32) as i32 - 1;
            gh = gh.clamp(horizon as i32 - jitter_range, horizon as i32 + jitter_range);
            ground_heights.push(gh.max(1) as usize);
        }

        // Ground fill with hue gradient (same as forest5)
        let ground_chars = ['╱', '╲', '·', '∿', '~'];
        let ground_depth = (height - horizon).max(1);
        let grad_dir = rng.random_range(0..6u32);
        let ground_base_hue: f64 = if let Color::Rgb { r, g, .. } = ground_color {
            (r as f64 * 1.4 + g as f64 * 0.7) % 360.0
        } else { 120.0 };
        let hue_sweep = rng.random_range(30..80u32) as f64;

        for x in 0..width {
            let col_horizon = ground_heights[x];
            for y in col_horizon..height {
                let depth = y - col_horizon;
                let ch = ground_chars[rng.random_range(0..ground_chars.len() as u32) as usize];
                let t = match grad_dir {
                    0 => x as f64 / width as f64,
                    1 => 1.0 - x as f64 / width as f64,
                    2 => depth as f64 / ground_depth as f64,
                    3 => (x as f64 / width as f64 + depth as f64 / ground_depth as f64) / 2.0,
                    4 => ((1.0 - x as f64 / width as f64) + depth as f64 / ground_depth as f64) / 2.0,
                    _ => {
                        let cx = width as f64 / 2.0;
                        let cy = ground_depth as f64 / 2.0;
                        let dx = (x as f64 - cx) / cx;
                        let dy = (depth as f64 - cy) / cy.max(1.0);
                        (dx * dx + dy * dy).sqrt().min(1.0)
                    }
                };
                let h = (ground_base_hue + t * hue_sweep).rem_euclid(360.0);
                let l = (0.25 - depth as f64 * 0.006).max(0.10);
                let s = 0.4 + t * 0.2;
                let c = hsl_to_rgb(h, s.min(0.8), l);
                grid[y][x] = Cell::new(ch, c);
            }
        }

        // --- Forest of trait trees (forest4-style composition) ---
        let tree_count = rng.random_range(6..12u32) as usize;
        let trait_kinds: [usize; 11] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

        struct TreeSlot {
            x: usize,
            root_y: usize,
            canopy_y: usize,
            spread: usize,
            kind: usize,
            hue: f64,
            energy: f32,
        }
        let mut slots: Vec<TreeSlot> = Vec::new();

        // One anchor tree -- tallest, widest, planted near center
        let anchor_x = rng.random_range((width / 8) as u32..(width * 7 / 8) as u32) as usize;
        let anchor_grass = ground_heights[anchor_x.min(width - 1)];
        let anchor_root = (anchor_grass + rng.random_range(0..3u32) as usize).min(height - 2);
        slots.push(TreeSlot {
            x: anchor_x, root_y: anchor_root,
            canopy_y: rng.random_range(1..4u32) as usize,
            spread: rng.random_range(14..22u32) as usize,
            kind: trait_kinds[rng.random_range(0..trait_kinds.len() as u32) as usize],
            hue: rng.random_range(0..360u32) as f64,
            energy: 0.95,
        });

        // Remaining trees with min spacing, height/spread tiers
        let min_spacing = (width / (tree_count + 1)).max(12);
        for _ in 0..tree_count - 1 {
            let mut tx = 0usize;
            let mut placed = false;
            for _ in 0..10 {
                tx = rng.random_range(3..(width - 3) as u32) as usize;
                let too_close = slots.iter().any(|s| ((s.x as i32 - tx as i32).unsigned_abs() as usize) < min_spacing);
                if !too_close { placed = true; break; }
            }
            if !placed { tx = rng.random_range(3..(width - 3) as u32) as usize; }

            let grass_y = ground_heights[tx.min(width - 1)];
            let root_y = (grass_y + rng.random_range(0..4u32) as usize).min(height - 2);

            // Height tiers: scrubby / medium / towering
            let max_possible = root_y.saturating_sub(1).max(4);
            let tree_height = match rng.random_range(0..10u32) {
                0..=2 => rng.random_range(3..8u32.min(max_possible as u32 + 1)) as usize,
                3..=6 => rng.random_range(8..20u32.min(max_possible as u32 + 1)) as usize,
                _ => rng.random_range(20u32.min(max_possible as u32)..max_possible as u32 + 1) as usize,
            };
            let canopy_y = root_y.saturating_sub(tree_height).max(1);

            // Spread tiers: narrow / medium / wide
            let spread = match rng.random_range(0..6u32) {
                0..=1 => rng.random_range(2..6u32) as usize,
                2..=4 => rng.random_range(5..12u32) as usize,
                _ => rng.random_range(10..20u32) as usize,
            };

            let kind = trait_kinds[rng.random_range(0..trait_kinds.len() as u32) as usize];
            let energy = match tree_height {
                0..=7 => rng.random_range(40..65u32) as f32 / 100.0,
                8..=19 => rng.random_range(65..85u32) as f32 / 100.0,
                _ => rng.random_range(85..100u32) as f32 / 100.0,
            };

            slots.push(TreeSlot {
                x: tx, root_y, canopy_y, spread, kind,
                hue: rng.random_range(0..360u32) as f64,
                energy,
            });
        }

        // Back-to-front depth sort
        slots.sort_by(|a, b| a.root_y.cmp(&b.root_y).then(a.x.cmp(&b.x)));

        // Depth-based brightness: farther (lower root_y) = dimmer
        let slot_count = slots.len();
        for (i, slot) in slots.iter().enumerate() {
            let depth_t = i as f64 / (slot_count - 1).max(1) as f64;
            let lightness = 0.2 + depth_t * 0.3;
            let saturation = 0.4 + depth_t * 0.3;
            let color = hsl_to_rgb(slot.hue, saturation, lightness);

            let plot_w = slot.spread * 2 + 6;
            let plot = Rect {
                x: slot.x.saturating_sub(plot_w / 2),
                y: slot.canopy_y,
                w: plot_w,
                h: slot.root_y - slot.canopy_y + 2,
            };
            let tp = TreeParams {
                plot,
                energy: slot.energy,
                trunk_color: color,
                bark_color: darken(color, 15),
                branch_color: color,
                tip_color: lighten(color, 30),
                fruit_color: shift_hue(color, 60.0),
                fruit_factor: 0.3,
                branch_factor: 0.8,
                direction: GrowDir::Up,
                bole: None,
                taper: TaperKind::default(),
            };
            match slot.kind {
                0 => SplitTree.grow(&mut grid, &tp, &mut rng),
                1 => SpiralTree.grow(&mut grid, &tp, &mut rng),
                2 => CandelabraTree.grow(&mut grid, &tp, &mut rng),
                3 => BirchTree.grow(&mut grid, &tp, &mut rng),
                4 => StormTree::new().grow(&mut grid, &tp, &mut rng),
                5 => DroopingTree.grow(&mut grid, &tp, &mut rng),
                6 => DeadTree.grow(&mut grid, &tp, &mut rng),
                7 => WavyBirch.grow(&mut grid, &tp, &mut rng),
                8 => PineTree.grow(&mut grid, &tp, &mut rng),
                9 => WillowTree.grow(&mut grid, &tp, &mut rng),
                10 => PalmTree.grow(&mut grid, &tp, &mut rng),
                _ => SpiralTree.grow(&mut grid, &tp, &mut rng),
            }
        }

        // Braille leaf clusters at branch tips
        let leaf_hue = rng.random_range(60..180u32) as f64;
        let leaf_color = hsl_to_rgb(leaf_hue, 0.5, 0.3);
        sprout_leaves(&mut grid, leaf_color, 45, &mut rng);

        // Flower/fruit scatter at ground level near tree bases
        for slot in &slots {
            let burst = rng.random_range(0..3u32);
            for _ in 0..burst {
                let angle = rng.random::<f32>() * std::f32::consts::TAU;
                let radius = rng.random_range(1..6u32) as f32;
                let fx = (slot.x as f32 + angle.cos() * radius * 1.5) as i32;
                let fy = slot.root_y as i32 + rng.random_range(1..3u32) as i32;
                if fx >= 1 && fy >= 1 && (fx as usize) < width - 1 && (fy as usize) < height - 1 {
                    let c = palette[rng.random_range(2..5)];
                    match rng.random_range(0..3u32) {
                        0 => grow_flower_spiral(&mut grid, fx as usize, fy as usize, c, &mut rng),
                        1 => grow_fruit_vine(&mut grid, fx as usize, fy as usize, c, &mut rng),
                        _ => draw_flower(&mut grid, fx as usize, fy as usize, rng.random_range(0..5), c),
                    }
                }
            }
        }

        // Foreground trees: 1-3 drawn last (in front of everything)
        let fg_count = rng.random_range(1..4u32);
        for _ in 0..fg_count {
            let tx = rng.random_range(3..(width - 3) as u32) as usize;
            let grass_y = ground_heights[tx.min(width - 1)];
            let root_y = (grass_y + rng.random_range(2..6u32) as usize).min(height - 2);
            let tree_height = rng.random_range(4..12u32) as usize;
            let canopy_y = root_y.saturating_sub(tree_height).max(1);
            let spread = rng.random_range(3..10u32) as usize;
            let kind = trait_kinds[rng.random_range(0..trait_kinds.len() as u32) as usize];
            let fg_hue = rng.random_range(0..360u32) as f64;
            let color = hsl_to_rgb(fg_hue, 0.6, 0.4);

            let plot_w = spread * 2 + 6;
            let plot = Rect {
                x: tx.saturating_sub(plot_w / 2),
                y: canopy_y,
                w: plot_w,
                h: root_y - canopy_y + 2,
            };
            let tp = TreeParams {
                plot,
                energy: 0.75,
                trunk_color: color,
                bark_color: darken(color, 15),
                branch_color: color,
                tip_color: lighten(color, 30),
                fruit_color: shift_hue(color, 60.0),
                fruit_factor: 0.2,
                branch_factor: 0.7,
                direction: GrowDir::Up,
                bole: None,
                taper: TaperKind::default(),
            };
            match kind {
                0 => SplitTree.grow(&mut grid, &tp, &mut rng),
                1 => SpiralTree.grow(&mut grid, &tp, &mut rng),
                2 => CandelabraTree.grow(&mut grid, &tp, &mut rng),
                3 => BirchTree.grow(&mut grid, &tp, &mut rng),
                4 => StormTree::new().grow(&mut grid, &tp, &mut rng),
                5 => DroopingTree.grow(&mut grid, &tp, &mut rng),
                6 => DeadTree.grow(&mut grid, &tp, &mut rng),
                7 => WavyBirch.grow(&mut grid, &tp, &mut rng),
                8 => PineTree.grow(&mut grid, &tp, &mut rng),
                9 => WillowTree.grow(&mut grid, &tp, &mut rng),
                10 => PalmTree.grow(&mut grid, &tp, &mut rng),
                _ => SpiralTree.grow(&mut grid, &tp, &mut rng),
            }
        }

    } else if mode == "boles1" {
        // boles1: bole styles at 3 energy levels (low/mid/high)
        let styles = ["Crescent", "Braille", "Frame", "Diamond", "Chevron", "Frame2"];
        let energies: [f32; 3] = [0.3, 0.6, 1.0];
        let energy_labels = ["Low", "Mid", "High"];
        let col_w = width / styles.len();
        let row_h = (height - 2) / energies.len(); // 2 rows for labels

        for (si, style_name) in styles.iter().enumerate() {
            let cx = (si * col_w + col_w / 2) as i32;
            let color = lighten(palette[si % palette.len()], 40);

            // Column label at bottom
            let lx = (cx - style_name.len() as i32 / 2).max(0) as usize;
            for (j, ch) in style_name.chars().enumerate() {
                if lx + j < width {
                    grid[height - 1][lx + j] = Cell::new(ch, lighten(color, 40));
                }
            }

            for (ei, &energy) in energies.iter().enumerate() {
                let ground_y = ((ei + 1) * row_h - 2) as i32;
                if ground_y < 2 || ground_y as usize >= height - 2 { continue; }

                let plot_w = (col_w as i32 - 2).max(6);
                let tp = TreeParams {
                    plot: Rect { x: (cx - plot_w / 2).max(0) as usize, y: 0, w: plot_w as usize, h: (ground_y + 1) as usize },
                    energy,
                    trunk_color: color,
                    bark_color: darken(color, 15),
                    branch_color: color,
                    tip_color: color,
                    fruit_color: color,
                    fruit_factor: 0.0,
                    branch_factor: 0.5,
                    direction: GrowDir::Up,
                    bole: None,
                taper: TaperKind::default(),
                };

                let bole = Bole { style: si };
                let exit = bole.draw(&mut grid, &tp, &mut rng);
                let (tx, ty) = (exit.x, exit.y);

                // Short trunk stub above bole
                for y in (ground_y - (row_h as i32 / 2))..ty {
                    if y >= 0 && (y as usize) < height && (tx as usize) < width {
                        grid[y as usize][tx as usize] = Cell::new('│', color);
                    }
                }

                // Energy label to the left of each row (only in first column)
                if si == 0 {
                    let elabel = energy_labels[ei];
                    let ly = ground_y as usize;
                    if ly < height {
                        for (j, ch) in elabel.chars().enumerate() {
                            if j < cx as usize - 1 {
                                grid[ly][j] = Cell::new(ch, rgb(120, 120, 120));
                            }
                        }
                    }
                }
            }
        }

    } else if mode == "boles2" {
        // boles2: experimental bole styles v2
        let styles = ["Crescent2", "Braille2", "Frame3", "Diamond2", "Chevron2", "Frame4"];
        let energies: [f32; 3] = [0.3, 0.6, 1.0];
        let energy_labels = ["Low", "Mid", "High"];
        let col_w = width / styles.len();
        let row_h = (height - 2) / energies.len();

        for (si, style_name) in styles.iter().enumerate() {
            let cx = (si * col_w + col_w / 2) as i32;
            let color = lighten(palette[si % palette.len()], 40);

            let lx = (cx - style_name.len() as i32 / 2).max(0) as usize;
            for (j, ch) in style_name.chars().enumerate() {
                if lx + j < width {
                    grid[height - 1][lx + j] = Cell::new(ch, lighten(color, 40));
                }
            }

            for (ei, &energy) in energies.iter().enumerate() {
                let ground_y = ((ei + 1) * row_h - 2) as i32;
                if ground_y < 2 || ground_y as usize >= height - 2 { continue; }

                let plot_w = (col_w as i32 - 2).max(6);
                let tp = TreeParams {
                    plot: Rect { x: (cx - plot_w / 2).max(0) as usize, y: 0, w: plot_w as usize, h: (ground_y + 1) as usize },
                    energy,
                    trunk_color: color,
                    bark_color: darken(color, 15),
                    branch_color: color,
                    tip_color: color,
                    fruit_color: color,
                    fruit_factor: 0.0,
                    branch_factor: 0.5,
                    direction: GrowDir::Up,
                    bole: None,
                taper: TaperKind::default(),
                };

                let bole = Bole { style: si + 6 };
                let exit = bole.draw(&mut grid, &tp, &mut rng);
                let (tx, ty) = (exit.x, exit.y);

                for y in (ground_y - (row_h as i32 / 2))..ty {
                    if y >= 0 && (y as usize) < height && (tx as usize) < width {
                        grid[y as usize][tx as usize] = Cell::new('│', color);
                    }
                }

                if si == 0 {
                    let elabel = energy_labels[ei];
                    let ly = ground_y as usize;
                    if ly < height {
                        for (j, ch) in elabel.chars().enumerate() {
                            if j < cx as usize - 1 {
                                grid[ly][j] = Cell::new(ch, rgb(120, 120, 120));
                            }
                        }
                    }
                }
            }
        }

    } else if mode == "boles3" {
        // boles3: refined bole styles with descriptive names
        let styles = ["Croissant", "Braille", "Frame", "Keel", "Chevron", "Buttress"];
        let energies: [f32; 3] = [0.3, 0.6, 1.0];
        let energy_labels = ["Low", "Mid", "High"];
        let col_w = width / styles.len();
        let row_h = (height - 2) / energies.len();

        for (si, style_name) in styles.iter().enumerate() {
            let cx = (si * col_w + col_w / 2) as i32;
            let color = lighten(palette[si % palette.len()], 40);

            let lx = (cx - style_name.len() as i32 / 2).max(0) as usize;
            for (j, ch) in style_name.chars().enumerate() {
                if lx + j < width {
                    grid[height - 1][lx + j] = Cell::new(ch, lighten(color, 40));
                }
            }

            for (ei, &energy) in energies.iter().enumerate() {
                let ground_y = ((ei + 1) * row_h - 2) as i32;
                if ground_y < 2 || ground_y as usize >= height - 2 { continue; }

                let plot_w = (col_w as i32 - 2).max(6);
                let tp = TreeParams {
                    plot: Rect { x: (cx - plot_w / 2).max(0) as usize, y: 0, w: plot_w as usize, h: (ground_y + 1) as usize },
                    energy,
                    trunk_color: color,
                    bark_color: darken(color, 15),
                    branch_color: color,
                    tip_color: color,
                    fruit_color: color,
                    fruit_factor: 0.0,
                    branch_factor: 0.5,
                    direction: GrowDir::Up,
                    bole: None,
                taper: TaperKind::default(),
                };

                let bole = Bole { style: si + 12 };
                let exit = bole.draw(&mut grid, &tp, &mut rng);
                let (tx, ty) = (exit.x, exit.y);

                for y in (ground_y - (row_h as i32 / 2))..ty {
                    if y >= 0 && (y as usize) < height && (tx as usize) < width {
                        grid[y as usize][tx as usize] = Cell::new('│', color);
                    }
                }

                if si == 0 {
                    let elabel = energy_labels[ei];
                    let ly = ground_y as usize;
                    if ly < height {
                        for (j, ch) in elabel.chars().enumerate() {
                            if j < cx as usize - 1 {
                                grid[ly][j] = Cell::new(ch, rgb(120, 120, 120));
                            }
                        }
                    }
                }
            }
        }

    } else if mode == "trunks1" {
        // trunks1: horizontal trunk algorithms + direction-aware branching
        let labels = ["Straight", "Wobble", "Organic", "Sine(2)", "Sine(4)", "Gnarled"];
        let col_w = width / labels.len();
        let ground_y = (height as i32) - 3;

        for (i, label) in labels.iter().enumerate() {
            let cx = (i * col_w + col_w / 2) as i32;
            let color = palette[i % palette.len()];

            let plot = Rect { x: (i * col_w).max(1), y: 2, w: col_w.min(20), h: (ground_y as usize).saturating_sub(2) };
            let params = TreeParams {
                plot,
                energy: 0.7,
                trunk_color: color,
                bark_color: darken(color, 15),
                branch_color: lighten(color, 20),
                tip_color: lighten(color, 40),
                fruit_color: color,
                fruit_factor: 0.0,
                branch_factor: 0.5,
                direction: GrowDir::Up,
                bole: None,
                taper: TaperKind::default(),
            };

            // Select trunk algo for this column
            use tree_draw::{StraightTrunk, WobbleTrunk, OrganicTrunk, SineTrunk, GnarledTrunk, TreeWithTrunk};

            let tree = SpiralTree;
            match i {
                0 => TreeWithTrunk { tree, trunk: Box::new(StraightTrunk { height_fraction: 0.5 }) }.grow(&mut grid, &params, &mut rng),
                1 => TreeWithTrunk { tree, trunk: Box::new(WobbleTrunk { height_fraction: 0.5 }) }.grow(&mut grid, &params, &mut rng),
                2 => TreeWithTrunk { tree, trunk: Box::new(OrganicTrunk { height_fraction: 0.5 }) }.grow(&mut grid, &params, &mut rng),
                3 => TreeWithTrunk { tree, trunk: Box::new(SineTrunk { height_fraction: 0.3, amplitude: 2 }) }.grow(&mut grid, &params, &mut rng),
                4 => TreeWithTrunk { tree, trunk: Box::new(SineTrunk { height_fraction: 0.3, amplitude: 3 }) }.grow(&mut grid, &params, &mut rng),
                5 => TreeWithTrunk { tree, trunk: Box::new(GnarledTrunk) }.grow(&mut grid, &params, &mut rng),
                _ => {}
            }

            // Label
            let lx = (cx - label.len() as i32 / 2).max(0) as usize;
            for (j, ch) in label.chars().enumerate() {
                if lx + j < width {
                    grid[height - 1][lx + j] = Cell::new(ch, lighten(color, 40));
                }
            }
        }

    } else if mode == "trees1" {
        // trees1: full pipeline demo -- tree + trunk algo + bole
        // args: [energy] [fruit_factor] [branch_factor] [bole_override]
        let energy: f32 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(0.8);
        let fruit_factor: f32 = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(0.3);
        let branch_factor: f32 = args.get(6).and_then(|s| s.parse().ok()).unwrap_or(0.5);
        let bole_override: Option<usize> = args.get(7).and_then(|s| s.parse().ok());

        let combos: Vec<(&str, Box<dyn TreeDrawer>, usize)> = vec![
            ("Spiral+Straight\n+Frame",
             Box::new(SpiralTree) as Box<dyn TreeDrawer>, 14),
            ("Spiral+Wobble\n+Chevron",
             Box::new(TreeWithTrunk { tree: SpiralTree, trunk: Box::new(WobbleTrunk { height_fraction: 0.6 }) }), 16),
            ("Candelabra+Organic\n+Keel",
             Box::new(TreeWithTrunk { tree: CandelabraTree, trunk: Box::new(OrganicTrunk { height_fraction: 0.5 }) }), 15),
            ("Split+Sine\n+Buttress",
             Box::new(TreeWithTrunk { tree: SplitTree, trunk: Box::new(SineTrunk { height_fraction: 0.3, amplitude: 2 }) }), 17),
            ("Birch+Gnarled\n+Braille",
             Box::new(TreeWithTrunk { tree: BirchTree, trunk: Box::new(GnarledTrunk) }), 13),
            ("Drooping+Sine\n+Frame",
             Box::new(TreeWithTrunk { tree: DroopingTree, trunk: Box::new(SineTrunk { height_fraction: 0.3, amplitude: 3 }) }), 14),
        ];
        let cols = combos.len();
        let col_w = width / cols;
        let ground_y = (height as i32) - 4;

        for (i, (label, drawer, default_bole)) in combos.iter().enumerate() {
            let cx = (i * col_w + col_w / 2) as i32;
            let color = palette[i % palette.len()];
            let bole_idx = bole_override.unwrap_or(*default_bole);

            let plot = Rect {
                x: (i * col_w + 1).min(width - 2),
                y: 2,
                w: (col_w - 2).max(4),
                h: (ground_y as usize).saturating_sub(2),
            };
            let params = TreeParams {
                plot,
                energy,
                trunk_color: color,
                bark_color: darken(color, 15),
                branch_color: lighten(color, 20),
                tip_color: lighten(color, 40),
                fruit_color: palette[(i + 2) % palette.len()],
                fruit_factor,
                branch_factor,
                direction: GrowDir::Up,
                bole: Some(Bole { style: bole_idx }),
                taper: TaperKind::default(),
            };

            drawer.grow(&mut grid, &params, &mut rng);

            // Multi-line label at bottom
            for (li, line) in label.split('\n').enumerate() {
                let lx = (cx - line.len() as i32 / 2).max(0) as usize;
                let ly = height - 2 + li;
                for (j, ch) in line.chars().enumerate() {
                    if lx + j < width && ly < height {
                        grid[ly][lx + j] = Cell::new(ch, lighten(color, 40));
                    }
                }
            }
        }

    } else if mode == "trees2" {
        // trees2: squat horizontal boles (styles 18-23) + tree combos
        let energy: f32 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(0.8);
        let fruit_factor: f32 = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(0.2);
        let branch_factor: f32 = args.get(6).and_then(|s| s.parse().ok()).unwrap_or(0.5);

        let combos: Vec<(&str, Box<dyn TreeDrawer>, usize)> = vec![
            ("Spiral\n+SqCrescent",
             Box::new(SpiralTree) as Box<dyn TreeDrawer>, 18),
            ("Spiral+Wobble\n+SqBraille",
             Box::new(TreeWithTrunk { tree: SpiralTree, trunk: Box::new(WobbleTrunk { height_fraction: 0.6 }) }), 19),
            ("Candelabra\n+SqFrame",
             Box::new(CandelabraTree) as Box<dyn TreeDrawer>, 20),
            ("Split+Sine\n+SqDiamond",
             Box::new(TreeWithTrunk { tree: SplitTree, trunk: Box::new(SineTrunk { height_fraction: 0.3, amplitude: 2 }) }), 21),
            ("Birch\n+SqChevron",
             Box::new(BirchTree) as Box<dyn TreeDrawer>, 22),
            ("Drooping\n+SqButtress",
             Box::new(DroopingTree) as Box<dyn TreeDrawer>, 23),
            ("WavyBirch\n+SqCrescent",
             Box::new(WavyBirch) as Box<dyn TreeDrawer>, 18),
        ];
        let cols = combos.len();
        let col_w = width / cols;
        let ground_y = (height as i32) - 4;

        for (i, (label, drawer, bole_idx)) in combos.iter().enumerate() {
            let cx = (i * col_w + col_w / 2) as i32;
            let color = palette[i % palette.len()];

            let plot = Rect {
                x: (i * col_w + 1).min(width - 2),
                y: 2,
                w: (col_w - 2).max(4),
                h: (ground_y as usize).saturating_sub(2),
            };
            let params = TreeParams {
                plot,
                energy,
                trunk_color: color,
                bark_color: darken(color, 15),
                branch_color: lighten(color, 20),
                tip_color: lighten(color, 40),
                fruit_color: palette[(i + 2) % palette.len()],
                fruit_factor,
                branch_factor,
                direction: GrowDir::Up,
                bole: Some(Bole { style: *bole_idx }),
                taper: [TaperKind::Diagonal, TaperKind::Shelf, TaperKind::Bracket,
                        TaperKind::Step, TaperKind::Melt, TaperKind::Shelf,
                        TaperKind::Bracket][i % 7],
            };

            drawer.grow(&mut grid, &params, &mut rng);

            for (li, line) in label.split('\n').enumerate() {
                let lx = (cx - line.len() as i32 / 2).max(0) as usize;
                let ly = height - 2 + li;
                for (j, ch) in line.chars().enumerate() {
                    if lx + j < width && ly < height {
                        grid[ly][lx + j] = Cell::new(ch, lighten(color, 40));
                    }
                }
            }
        }

    } else if mode == "trees3" {
        // trees3: vertical catalog -- all tree types, trunk algos, taper styles, bole styles
        let page_w = 80usize;
        let tree_h = 28usize;
        let label_h = 2usize;
        let section_gap = 2usize;
        let header_h = 2usize;

        // Section heights
        let sec1_h = header_h + 2 * (tree_h + label_h) + section_gap;  // 8 tree types
        let sec2_h = header_h + tree_h + label_h + section_gap;        // 7 trunk algos
        let sec3_h = header_h + tree_h + label_h + section_gap;        // 5 taper styles
        let bole_tree_h = 20usize;
        let sec4_h = header_h + 3 * (bole_tree_h + label_h) + section_gap;  // 24 bole styles

        let page_h = sec1_h + sec2_h + sec3_h + sec4_h + 4;
        let mut pg = vec![vec![Cell::blank(); page_w]; page_h];

        let energy = 0.8f32;

        let write_header = |pg: &mut Vec<Vec<Cell>>, y: usize, text: &str, color: Color| {
            let lx = (page_w / 2).saturating_sub(text.len() / 2);
            for (j, ch) in text.chars().enumerate() {
                if lx + j < page_w { pg[y][lx + j] = Cell::new(ch, color); }
            }
            for x in 0..page_w { pg[y + 1][x] = Cell::new('─', darken(color, 30)); }
        };

        let write_label = |pg: &mut Vec<Vec<Cell>>, row_y: usize, cx: i32, label: &str, color: Color| {
            let lx = (cx - label.len() as i32 / 2).max(0) as usize;
            for (j, ch) in label.chars().enumerate() {
                if lx + j < page_w && row_y < pg.len() { pg[row_y][lx + j] = Cell::new(ch, color); }
            }
        };

        // ── Section 1: Tree Types ─────────────────────────────────
        let mut cy = 1usize;
        write_header(&mut pg, cy, "── TREE TYPES ──", palette[4]);
        cy += header_h;

        let tree_labels = ["Spiral", "Candelabra", "Split", "Birch", "WavyBirch", "Storm", "Dead", "Drooping"];
        let cols8 = 4usize;
        let col_w8 = page_w / cols8;

        for idx in 0..8usize {
            let row = idx / cols8;
            let col = idx % cols8;
            let row_y = cy + row * (tree_h + label_h);
            let color = palette[idx % palette.len()];
            let cx_i = (col * col_w8 + col_w8 / 2) as i32;
            let params = TreeParams {
                plot: Rect { x: col * col_w8 + 1, y: row_y, w: col_w8 - 2, h: tree_h },
                energy, trunk_color: color, bark_color: darken(color, 15),
                branch_color: lighten(color, 20), tip_color: lighten(color, 40),
                fruit_color: palette[(idx + 2) % palette.len()],
                fruit_factor: 0.2, branch_factor: 0.5,
                direction: GrowDir::Up, bole: Some(Bole { style: idx % 8 }),
                taper: TaperKind::Bracket,
            };
            match idx {
                0 => SpiralTree.grow(&mut pg, &params, &mut rng),
                1 => CandelabraTree.grow(&mut pg, &params, &mut rng),
                2 => SplitTree.grow(&mut pg, &params, &mut rng),
                3 => BirchTree.grow(&mut pg, &params, &mut rng),
                4 => WavyBirch.grow(&mut pg, &params, &mut rng),
                5 => StormTree::new().grow(&mut pg, &params, &mut rng),
                6 => DeadTree.grow(&mut pg, &params, &mut rng),
                _ => DroopingTree.grow(&mut pg, &params, &mut rng),
            }
            write_label(&mut pg, row_y + tree_h, cx_i, tree_labels[idx], lighten(color, 40));
        }
        cy += 2 * (tree_h + label_h) + section_gap;

        // ── Section 2: Trunk Algorithms ───────────────────────────
        write_header(&mut pg, cy, "── TRUNK ALGORITHMS ──", palette[4]);
        cy += header_h;

        let trunk_labels = ["Straight", "Thick", "Wobble", "Lean", "Gnarled", "Organic", "Sine"];
        let cols7 = 7usize;
        let col_w7 = page_w / cols7;

        for i in 0..7usize {
            let color = palette[i % palette.len()];
            let cx_i = (i * col_w7 + col_w7 / 2) as i32;
            let params = TreeParams {
                plot: Rect { x: i * col_w7 + 1, y: cy, w: col_w7 - 2, h: tree_h },
                energy, trunk_color: color, bark_color: darken(color, 15),
                branch_color: lighten(color, 20), tip_color: lighten(color, 40),
                fruit_color: palette[(i + 2) % palette.len()],
                fruit_factor: 0.2, branch_factor: 0.5,
                direction: GrowDir::Up, bole: Some(Bole { style: 14 }),
                taper: TaperKind::default(),
            };
            let drawer: Box<dyn TreeDrawer> = match i {
                0 => Box::new(TreeWithTrunk { tree: SpiralTree, trunk: Box::new(StraightTrunk { height_fraction: 0.5 }) }),
                1 => Box::new(TreeWithTrunk { tree: SpiralTree, trunk: Box::new(ThickTrunk { height_fraction: 0.5 }) }),
                2 => Box::new(TreeWithTrunk { tree: SpiralTree, trunk: Box::new(WobbleTrunk { height_fraction: 0.5 }) }),
                3 => Box::new(TreeWithTrunk { tree: SpiralTree, trunk: Box::new(LeanTrunk::new()) }),
                4 => Box::new(TreeWithTrunk { tree: SpiralTree, trunk: Box::new(GnarledTrunk) }),
                5 => Box::new(TreeWithTrunk { tree: SpiralTree, trunk: Box::new(OrganicTrunk { height_fraction: 0.5 }) }),
                _ => Box::new(TreeWithTrunk { tree: SpiralTree, trunk: Box::new(SineTrunk { height_fraction: 0.3, amplitude: 2 }) }),
            };
            drawer.grow(&mut pg, &params, &mut rng);
            write_label(&mut pg, cy + tree_h, cx_i, trunk_labels[i], lighten(color, 40));
        }
        cy += tree_h + label_h + section_gap;

        // ── Section 3: Taper Styles ───────────────────────────────
        write_header(&mut pg, cy, "── TAPER STYLES ──", palette[4]);
        cy += header_h;

        let taper_data = [
            ("Diagonal", TaperKind::Diagonal), ("Shelf", TaperKind::Shelf),
            ("Bracket", TaperKind::Bracket), ("Step", TaperKind::Step),
            ("Melt", TaperKind::Melt),
        ];
        let cols5 = 5usize;
        let col_w5 = page_w / cols5;

        for (i, (label, taper)) in taper_data.iter().enumerate() {
            let color = palette[i % palette.len()];
            let cx_i = (i * col_w5 + col_w5 / 2) as i32;
            let params = TreeParams {
                plot: Rect { x: i * col_w5 + 1, y: cy, w: col_w5 - 2, h: tree_h },
                energy, trunk_color: color, bark_color: darken(color, 15),
                branch_color: lighten(color, 20), tip_color: lighten(color, 40),
                fruit_color: palette[(i + 2) % palette.len()],
                fruit_factor: 0.2, branch_factor: 0.5,
                direction: GrowDir::Up, bole: Some(Bole { style: 0 }),
                taper: *taper,
            };
            SpiralTree.grow(&mut pg, &params, &mut rng);
            write_label(&mut pg, cy + tree_h, cx_i, label, lighten(color, 40));
        }
        cy += tree_h + label_h + section_gap;

        // ── Section 4: Bole Styles ────────────────────────────────
        write_header(&mut pg, cy, "── BOLE STYLES ──", palette[4]);
        cy += header_h;

        let boles_per_row = 8usize;
        let bole_col_w = page_w / boles_per_row;

        for bole_i in 0..24usize {
            let row = bole_i / boles_per_row;
            let col = bole_i % boles_per_row;
            let row_y = cy + row * (bole_tree_h + label_h);
            let color = palette[bole_i % palette.len()];
            let cx_i = (col * bole_col_w + bole_col_w / 2) as i32;
            let label = format!("{}", bole_i);
            let params = TreeParams {
                plot: Rect { x: col * bole_col_w + 1, y: row_y, w: bole_col_w - 2, h: bole_tree_h },
                energy, trunk_color: color, bark_color: darken(color, 15),
                branch_color: lighten(color, 20), tip_color: lighten(color, 40),
                fruit_color: palette[(bole_i + 2) % palette.len()],
                fruit_factor: 0.2, branch_factor: 0.5,
                direction: GrowDir::Up, bole: Some(Bole { style: bole_i }),
                taper: TaperKind::Bracket,
            };
            SpiralTree.grow(&mut pg, &params, &mut rng);
            write_label(&mut pg, row_y + bole_tree_h, cx_i, &label, lighten(color, 40));
        }

        render_grid(&pg);
        return;

    } else if mode == "trees4" {
        // trees4: showcase all TreeDrawer types including new ports
        // One tree per slot, labeled, with boles and fruit
        let energy: f32 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(0.8);

        let all_trees: Vec<(&str, Box<dyn TreeDrawer>)> = vec![
            ("Spiral",       Box::new(SpiralTree)),
            ("Candelabra",   Box::new(CandelabraTree)),
            ("Split",        Box::new(SplitTree)),
            ("Birch",        Box::new(BirchTree)),
            ("WavyBirch",    Box::new(WavyBirch)),
            ("Storm",        Box::new(StormTree::new())),
            ("Dead",         Box::new(DeadTree)),
            ("Drooping",     Box::new(DroopingTree)),
            ("Pine",         Box::new(PineTree)),
            ("Willow",       Box::new(WillowTree)),
            ("Palm",         Box::new(PalmTree)),
            ("Wide",         Box::new(WideTree)),
            ("Asymmetric",   Box::new(AsymmetricTree)),
            ("Kaiju",        Box::new(KaijuTree)),
            ("Zigzag",       Box::new(ZigzagTree)),
            ("BrailleCanopy",Box::new(BrailleCanopyTree)),
            ("Tendril",      Box::new(TendrilTree)),
        ];

        let count = all_trees.len();
        let cols = 6usize;
        let rows = (count + cols - 1) / cols;
        let cell_w = width / cols;
        let cell_h = 28usize;  // tall cells like trees3
        let page_h = rows * cell_h + 2;
        let mut grid = vec![vec![Cell::blank(); width]; page_h];

        for (i, (label, drawer)) in all_trees.iter().enumerate() {
            let col = i % cols;
            let row = i / cols;
            let px = col * cell_w;
            let py = row * cell_h;
            let color = palette[i % palette.len()];

            let params = TreeParams {
                plot: Rect { x: px + 1, y: py + 1, w: cell_w - 2, h: cell_h - 3 },
                energy,
                trunk_color: color,
                bark_color: darken(color, 15),
                branch_color: lighten(color, 20),
                tip_color: lighten(color, 40),
                fruit_color: palette[(i + 3) % palette.len()],
                fruit_factor: 0.3,
                branch_factor: 0.7,
                direction: GrowDir::Up,
                bole: Some(Bole { style: i }),
                taper: TaperKind::Bracket,
            };
            drawer.grow(&mut grid, &params, &mut rng);

            // Label
            let lx = px + cell_w / 2 - label.len() / 2;
            let ly = py + cell_h - 1;
            for (j, ch) in label.chars().enumerate() {
                if lx + j < width && ly < page_h {
                    grid[ly][lx + j] = Cell::new(ch, darken(color, 20));
                }
            }
        }

        render_grid(&grid);
        return;

    } else if mode == "bushes" {
        // bushes: showcase full-size bole patterns as standalone bush sprites
        // args: [energy]
        let energy: f32 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(0.8);

        // Styles 0-17 only (squat styles 18-23 are too minimal as standalone bushes)
        let styles: Vec<usize> = (0..18).collect();
        let cols = 6usize;
        let rows = 3usize;
        let cell_w = width / cols;
        let cell_h = height / rows;

        for (i, &style_idx) in styles.iter().enumerate() {
            let col = i % cols;
            let row = i / cols;
            let cx = (col * cell_w + cell_w / 2) as i32;
            let cy = (row * cell_h + cell_h * 3 / 4) as i32;
            let bush_w = (cell_w as i32 / 3).max(3);
            let color = palette[style_idx % palette.len()];

            // Rotate through fade directions
            let fade = match style_idx % 3 {
                0 => FadeDir::Down,
                1 => FadeDir::CenterOut,
                _ => FadeDir::Up,
            };
            // Ground: dark version of the palette for contrast
            let ground = darken(palette[(style_idx + 3) % palette.len()], 40);

            let bush = BushSprite {
                style: style_idx,
                x: cx, y: cy,
                width: bush_w,
                color,
                ground,
                fade,
                energy,
            };
            bush.draw(&mut grid, &mut rng);

            // Label
            let label = format!("{}", style_idx);
            let lx = (cx - label.len() as i32 / 2).max(0) as usize;
            let label_y = (row * cell_h + cell_h - 1).min(height - 1);
            for (j, ch) in label.chars().enumerate() {
                if lx + j < width {
                    grid[label_y][lx + j] = Cell::new(ch, darken(color, 20));
                }
            }
        }

    } else if mode == "forest7" {
        // forest7: production layered forest with boles, tapers, fruit
        let horizon = height * 3 / 5 + rng.random_range(0..(height / 5).max(1) as u32) as usize;
        let sky_color = darken(palette[0], 95);
        let ground_color = darken(palette[1], 80);

        // Sky
        for y in 0..horizon {
            for x in 0..width {
                if rng.random_range(0..15u32) == 0 { grid[y][x] = Cell::new('·', sky_color); }
            }
        }
        let cloud_count = rng.random_range(1..5u32);
        let cloud_color = lighten(palette[0], 15);
        for _ in 0..cloud_count {
            let cx = rng.random_range(5..(width - 5) as u32) as usize;
            let cy = rng.random_range(2..(horizon / 2).max(3) as u32) as usize;
            let cw = rng.random_range(8..20u32) as usize;
            draw_cloud(&mut grid, cx, cy, cw, cloud_color, &mut rng);
        }

        // Per-column ground height
        let jitter_range = rng.random_range(2..6u32) as i32;
        let mut ground_heights: Vec<usize> = Vec::with_capacity(width);
        let mut gh = horizon as i32;
        for _ in 0..width {
            gh += rng.random_range(0..3u32) as i32 - 1;
            gh = gh.clamp(horizon as i32 - jitter_range, horizon as i32 + jitter_range);
            ground_heights.push(gh.max(1) as usize);
        }

        // Ground fill with hue gradient
        let ground_chars = ['╱', '╲', '·', '∿', '~'];
        let ground_depth = (height - horizon).max(1);
        let grad_dir = rng.random_range(0..6u32);
        let ground_base_hue: f64 = if let Color::Rgb { r, g, .. } = ground_color {
            (r as f64 * 1.4 + g as f64 * 0.7) % 360.0
        } else { 120.0 };
        let hue_sweep = rng.random_range(30..80u32) as f64;

        for x in 0..width {
            let col_horizon = ground_heights[x];
            for y in col_horizon..height {
                let depth = y - col_horizon;
                let ch = ground_chars[rng.random_range(0..ground_chars.len() as u32) as usize];
                let t = match grad_dir {
                    0 => x as f64 / width as f64,
                    1 => 1.0 - x as f64 / width as f64,
                    2 => depth as f64 / ground_depth as f64,
                    3 => (x as f64 / width as f64 + depth as f64 / ground_depth as f64) / 2.0,
                    4 => ((1.0 - x as f64 / width as f64) + depth as f64 / ground_depth as f64) / 2.0,
                    _ => {
                        let cx = width as f64 / 2.0;
                        let cy = ground_depth as f64 / 2.0;
                        let dx = (x as f64 - cx) / cx;
                        let dy = (depth as f64 - cy) / cy.max(1.0);
                        (dx * dx + dy * dy).sqrt().min(1.0)
                    }
                };
                let h = (ground_base_hue + t * hue_sweep).rem_euclid(360.0);
                let l = (0.25 - depth as f64 * 0.006).max(0.10);
                let s = 0.4 + t * 0.2;
                let c = hsl_to_rgb(h, s.min(0.8), l);
                grid[y][x] = Cell::new(ch, c);
            }
        }

        // ── Scene walk placement ──────────────────────────────────────
        // Walk across the terrain, placing elements at each stop.
        // Element types: tree, bush, flower cluster, fruit vine, empty gap.
        let all_tapers = [TaperKind::Diagonal, TaperKind::Shelf, TaperKind::Bracket, TaperKind::Step, TaperKind::Melt];

        #[derive(Clone, Copy)]
        enum F7Element { Tree { kind: usize, spread: usize, tree_h: usize, bole_style: Option<usize>, taper: TaperKind },
                         Bush { style: usize, bush_w: i32 },
                         Flowers, FruitVine }

        struct F7Stop { x: usize, root_y: usize, hue: f64, layer: u8, element: F7Element }

        let mut stops: Vec<F7Stop> = Vec::new();

        // Walk: start at random x, hop 8-20 cells each step, wrap around
        let stop_count = rng.random_range(12..22u32) as usize;
        let min_spacing = (width / (stop_count + 1)).max(6);
        let mut wx = rng.random_range(4..(width - 4) as u32) as usize;

        for si in 0..stop_count {
            // Hop forward with some jitter
            if si > 0 {
                let hop = rng.random_range(min_spacing as u32..(min_spacing as u32 * 3).min(width as u32 / 2));
                wx = (wx + hop as usize) % width;
                wx = wx.clamp(3, width - 4);
            }

            let grass_y = ground_heights[wx.min(width - 1)];
            // Layer assignment: first third bg, middle third mid, last third fg
            let layer = match si * 3 / stop_count {
                0 => 0u8,
                1 => 1,
                _ => 2,
            };
            let root_offset = match layer {
                0 => rng.random_range(0..2u32) as usize,
                1 => rng.random_range(1..5u32) as usize,
                _ => rng.random_range(2..7u32) as usize,
            };
            let root_y = (grass_y + root_offset).min(height - 2);

            // Pick element type: trees most common, bushes and flowers fill gaps
            let element = match rng.random_range(0..10u32) {
                0..=5 => {
                    let kind = rng.random_range(0..17u32) as usize;
                    let spread = match layer {
                        0 => rng.random_range(2..6u32) as usize,
                        1 => rng.random_range(5..14u32) as usize,
                        _ => rng.random_range(10..22u32) as usize,
                    };
                    let tree_h = match layer {
                        0 => rng.random_range(3..10u32) as usize,
                        1 => rng.random_range(10..25u32) as usize,
                        _ => rng.random_range(20..40u32.min(root_y.max(21) as u32)) as usize,
                    };
                    // ~40% of trees get a bole, rest go straight trunk into ground
                    let bole_style = if rng.random_range(0..10u32) < 4 {
                        Some(rng.random_range(0..10u32) as usize)  // simpler styles only
                    } else { None };
                    F7Element::Tree {
                        kind, spread, tree_h, bole_style,
                        taper: all_tapers[rng.random_range(0..all_tapers.len() as u32) as usize],
                    }
                }
                // 6..=7 => F7Element::Bush {
                //     style: rng.random_range(0..18u32) as usize,
                //     bush_w: rng.random_range(3..8u32) as i32,
                // },
                6..=7 => F7Element::Flowers,
                8 => F7Element::Flowers,
                _ => F7Element::FruitVine,
            };

            stops.push(F7Stop {
                x: wx, root_y,
                hue: rng.random_range(0..360u32) as f64,
                layer, element,
            });
        }

        // Sort back-to-front: bg (layer 0) first, then mid, then fg
        stops.sort_by(|a, b| a.layer.cmp(&b.layer).then(a.root_y.cmp(&b.root_y)));

        // ── Draw each stop ───────────────────────────────────────────
        for stop in &stops {
            let lightness = match stop.layer {
                0 => 0.15 + rng.random::<f64>() * 0.10,
                1 => 0.25 + rng.random::<f64>() * 0.10,
                _ => 0.35 + rng.random::<f64>() * 0.10,
            };
            let saturation = match stop.layer {
                0 => 0.30, 1 => 0.45, _ => 0.60,
            };
            let energy = match stop.layer {
                0 => rng.random_range(30..55u32) as f32 / 100.0,
                1 => rng.random_range(60..85u32) as f32 / 100.0,
                _ => rng.random_range(85..100u32) as f32 / 100.0,
            };
            let color = hsl_to_rgb(stop.hue, saturation, lightness);

            match stop.element {
                F7Element::Tree { kind, spread, tree_h, bole_style, taper } => {
                    let canopy_y = stop.root_y.saturating_sub(tree_h).max(1);
                    let plot_w = spread * 2 + 6;
                    let plot = Rect {
                        x: stop.x.saturating_sub(plot_w / 2),
                        y: canopy_y,
                        w: plot_w.min(width),
                        h: stop.root_y.saturating_sub(canopy_y) + 2,
                    };
                    let tp = TreeParams {
                        plot, energy,
                        trunk_color: color, bark_color: darken(color, 15),
                        branch_color: color, tip_color: lighten(color, 30),
                        fruit_color: shift_hue(color, 60.0),
                        fruit_factor: 0.3, branch_factor: 0.8,
                        direction: GrowDir::Up,
                        bole: bole_style.map(|s| Bole { style: s }),
                        taper,
                    };
                    match kind % 17 {
                        0 => SpiralTree.grow(&mut grid, &tp, &mut rng),
                        1 => CandelabraTree.grow(&mut grid, &tp, &mut rng),
                        2 => SplitTree.grow(&mut grid, &tp, &mut rng),
                        3 => BirchTree.grow(&mut grid, &tp, &mut rng),
                        4 => WavyBirch.grow(&mut grid, &tp, &mut rng),
                        5 => StormTree::new().grow(&mut grid, &tp, &mut rng),
                        6 => DeadTree.grow(&mut grid, &tp, &mut rng),
                        7 => DroopingTree.grow(&mut grid, &tp, &mut rng),
                        8 => PineTree.grow(&mut grid, &tp, &mut rng),
                        9 => WillowTree.grow(&mut grid, &tp, &mut rng),
                        10 => PalmTree.grow(&mut grid, &tp, &mut rng),
                        11 => WideTree.grow(&mut grid, &tp, &mut rng),
                        12 => AsymmetricTree.grow(&mut grid, &tp, &mut rng),
                        13 => KaijuTree.grow(&mut grid, &tp, &mut rng),
                        14 => ZigzagTree.grow(&mut grid, &tp, &mut rng),
                        15 => BrailleCanopyTree.grow(&mut grid, &tp, &mut rng),
                        16 => TendrilTree.grow(&mut grid, &tp, &mut rng),
                        _ => SpiralTree.grow(&mut grid, &tp, &mut rng),
                    }
                }
                F7Element::Bush { style, bush_w } => {
                    let fade = match rng.random_range(0..3u32) {
                        0 => FadeDir::Down, 1 => FadeDir::CenterOut, _ => FadeDir::Up,
                    };
                    let bush = BushSprite {
                        style, x: stop.x as i32, y: stop.root_y as i32,
                        width: bush_w, color, ground: color,  // no fade -- preserve ground colors
                        fade, energy,
                    };
                    bush.draw(&mut grid, &mut rng);
                }
                F7Element::Flowers => {
                    let burst = rng.random_range(3..7u32);
                    for _ in 0..burst {
                        let angle = rng.random::<f32>() * std::f32::consts::TAU;
                        let radius = rng.random_range(1..8u32) as f32;
                        let fx = (stop.x as f32 + angle.cos() * radius * 1.5) as i32;
                        let fy = stop.root_y as i32 + rng.random_range(0..3u32) as i32;
                        if fx >= 1 && fy >= 1 && (fx as usize) < width - 1 && (fy as usize) < height - 1 {
                            grow_flower_spiral(&mut grid, fx as usize, fy as usize, color, &mut rng);
                        }
                    }
                }
                F7Element::FruitVine => {
                    let burst = rng.random_range(2..5u32);
                    for _ in 0..burst {
                        let angle = rng.random::<f32>() * std::f32::consts::TAU;
                        let radius = rng.random_range(1..6u32) as f32;
                        let fx = (stop.x as f32 + angle.cos() * radius * 1.5) as i32;
                        let fy = stop.root_y as i32 + rng.random_range(0..2u32) as i32;
                        if fx >= 1 && fy >= 1 && (fx as usize) < width - 1 && (fy as usize) < height - 1 {
                            let c = shift_hue(color, rng.random_range(20..80u32) as f64);
                            grow_fruit_vine(&mut grid, fx as usize, fy as usize, c, &mut rng);
                        }
                    }
                    // Braille fruit dots near the vines
                    for _ in 0..rng.random_range(1..4u32) {
                        let fx = stop.x as i32 + rng.random_range(-4..5i32);
                        let fy = stop.root_y as i32 + rng.random_range(-2..3i32);
                        if fx >= 0 && fy >= 0 && (fx as usize) < width && (fy as usize) < height {
                            let fruit_c = shift_hue(color, 60.0);
                            draw_fruit(&mut grid, fx as usize, fy as usize, rng.random_range(0..5), fruit_c);
                        }
                    }
                }
            }
        }

        // Braille leaf clusters on branch tips
        let leaf_hue = rng.random_range(60..180u32) as f64;
        let leaf_color = hsl_to_rgb(leaf_hue, 0.5, 0.3);
        sprout_leaves(&mut grid, leaf_color, 45, &mut rng);

        // Extra ground-level flower/fruit scatter near tree stops
        for stop in &stops {
            if stop.layer == 0 { continue; }
            if let F7Element::Tree { .. } = stop.element {
                let burst = rng.random_range(0..3u32);
                for _ in 0..burst {
                    let angle = rng.random::<f32>() * std::f32::consts::TAU;
                    let radius = rng.random_range(1..6u32) as f32;
                    let fx = (stop.x as f32 + angle.cos() * radius * 1.5) as i32;
                    let fy = stop.root_y as i32 + rng.random_range(1..3u32) as i32;
                    if fx >= 1 && fy >= 1 && (fx as usize) < width - 1 && (fy as usize) < height - 1 {
                        let c = palette[rng.random_range(2..5)];
                        match rng.random_range(0..3u32) {
                            0 => grow_flower_spiral(&mut grid, fx as usize, fy as usize, c, &mut rng),
                            1 => grow_fruit_vine(&mut grid, fx as usize, fy as usize, c, &mut rng),
                            _ => draw_flower(&mut grid, fx as usize, fy as usize, rng.random_range(0..5), c),
                        }
                    }
                }
            }
        }

    } else if mode == "mondrian2" {
        let line_w = 2;

        let fill_colors = if theme_name.is_empty() {
            let (fills, _) = mondrian_colors();
            fills
        } else {
            [
                lighten(palette[0], 40),
                palette[1],
                palette[2],
                palette[3],
                lighten(palette[0], 40),
            ]
        };
        let line_color = if theme_name.is_empty() {
            rgb(20, 20, 20)
        } else {
            darken(palette[0], 60)
        };

        // Layout mondrian grid with no content blocks -- all leaves are empty
        let rects = layout_mondrian(
            &mut grid,
            &[],
            0,
            line_w,
            12,
            5,
            line_color,
            line_color,
            &fill_colors,
            line_color,
            &mut rng,
        );

        // Fill each leaf with something unexpected
        for rect in &rects {
            let inset = Rect {
                x: rect.x + 1,
                y: rect.y + 1,
                w: rect.w.saturating_sub(2),
                h: rect.h.saturating_sub(2),
            };
            if inset.w < 3 || inset.h < 3 { continue; }

            match rng.random_range(0..7u32) {
                0..=1 => {
                    // Tree centered in the rect
                    let tx = inset.x + inset.w / 2;
                    let canopy = inset.y + 1;
                    let root = inset.y + inset.h - 1;
                    let spread = (inset.w / 3).max(2);
                    let color = palette[rng.random_range(1..4)];
                    // Clear to blank first
                    for y in inset.y..inset.y + inset.h {
                        for x in inset.x..inset.x + inset.w {
                            if y < height && x < width { grid[y][x] = Cell::blank(); }
                        }
                    }
                    draw_tree(&mut grid, tx, root, canopy, spread, rng.random_range(0..12), color, &mut rng);
                }
                2 => {
                    // Flower garden -- clear + scatter flowers
                    for y in inset.y..inset.y + inset.h {
                        for x in inset.x..inset.x + inset.w {
                            if y < height && x < width { grid[y][x] = Cell::blank(); }
                        }
                    }
                    let cx = inset.x + inset.w / 2;
                    let cy = inset.y + inset.h / 2;
                    draw_flower(&mut grid, cx, cy, rng.random_range(0..5), palette[3]);
                    let count = rng.random_range(2..6u32);
                    for _ in 0..count {
                        let angle = rng.random::<f32>() * std::f32::consts::TAU;
                        let r = rng.random_range(2..((inset.w.min(inset.h) / 2).max(3)) as u32) as f32;
                        let fx = (cx as f32 + angle.cos() * r * 1.5) as usize;
                        let fy = (cy as f32 + angle.sin() * r * 0.7) as usize;
                        if fx > inset.x && fx < inset.x + inset.w - 1 && fy > inset.y && fy < inset.y + inset.h - 1 {
                            draw_flower(&mut grid, fx, fy, rng.random_range(0..5), palette[rng.random_range(2..4)]);
                        }
                    }
                }
                3 => {
                    // Rain in this cell only
                    let rain_color = darken(palette[2], 40);
                    let rain_chars = ['│', '┊', '╎', '┆'];
                    for y in inset.y..inset.y + inset.h {
                        for x in inset.x..inset.x + inset.w {
                            if y >= height || x >= width { continue; }
                            if grid[y][x].ch != ' ' { continue; }
                            if rng.random::<f32>() > 0.12 { continue; }
                            let streak = ((x * 7 + 13) % 11) < 3;
                            if !streak && rng.random::<f32>() > 0.3 { continue; }
                            let ch = rain_chars[rng.random_range(0..rain_chars.len())];
                            grid[y][x] = Cell::new(ch, darken(rain_color, rng.random_range(0..20)));
                        }
                    }
                }
                4 => {
                    // Fruit still life
                    for y in inset.y..inset.y + inset.h {
                        for x in inset.x..inset.x + inset.w {
                            if y < height && x < width { grid[y][x] = Cell::blank(); }
                        }
                    }
                    let count = rng.random_range(2..5u32);
                    for _ in 0..count {
                        let fx = inset.x + rng.random_range(2..inset.w.saturating_sub(2).max(3) as u32) as usize;
                        let fy = inset.y + rng.random_range(1..inset.h.saturating_sub(2).max(2) as u32) as usize;
                        draw_fruit(&mut grid, fx, fy, rng.random_range(0..5), palette[rng.random_range(1..4)]);
                    }
                }
                5 => {
                    // Stars / night sky in this cell
                    let star_color = lighten(palette[4], 20);
                    let star_chars = ['·', '∙', '°', '*', '⋅', '✦'];
                    for y in inset.y..inset.y + inset.h {
                        for x in inset.x..inset.x + inset.w {
                            if y >= height || x >= width { continue; }
                            if grid[y][x].ch != ' ' { continue; }
                            if rng.random::<f32>() > 0.06 { continue; }
                            let ch = star_chars[rng.random_range(0..star_chars.len())];
                            grid[y][x] = Cell::new(ch, darken(star_color, rng.random_range(0..40)));
                        }
                    }
                }
                _ => {
                    // Leave as flat color fill (original mondrian behavior)
                }
            }
        }

    } else if mode == "world" {
        render_world(&mut grid, width, height, &palette, &mut rng);
    } else if mode == "noise" {
        let names = ["truchet", "higaki", "higaki-s", "grass", "static", "dot"];
        let cols = NOISE_VARIANT_COUNT;
        let cell_w = width / cols;
        for i in 0..NOISE_VARIANT_COUNT {
            let x0 = i * cell_w;
            let r = Rect {
                x: x0,
                y: 1,
                w: cell_w,
                h: height - 1,
            };
            let variant = noise_variant_from_index(i);
            let c1 = palette[(i % 3) + 1];
            let c2 = darken(c1, 30);
            fill_noise(&mut grid, &r, variant, c1, c2, &mut rng);
            for (j, ch) in names[i].chars().enumerate() {
                if x0 + j < width {
                    grid[0][x0 + j] = Cell::new(ch, palette[4]);
                }
            }
        }
    } else {
        fill_truchet(&mut grid, width, height, darken(palette[1], 80), &mut rng);

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

        for y in 2..18 {
            for x in 2..22 {
                grid[y][x] = Cell::blank();
            }
        }
        grow_tree(&mut grid, 12, 17, 3, 8, palette[1], &mut rng);

        for y in 2..18 {
            for x in 58..78 {
                grid[y][x] = Cell::blank();
            }
        }
        grow_tree(&mut grid, 68, 17, 3, 8, palette[2], &mut rng);

        draw_flower(&mut grid, 30, 8, rng.random_range(0..5), palette[3]);
        draw_flower(&mut grid, 50, 8, rng.random_range(0..5), palette[3]);
        draw_flower(&mut grid, 15, 35, rng.random_range(0..5), palette[3]);
        draw_flower(&mut grid, 65, 35, rng.random_range(0..5), palette[3]);
        draw_flower(&mut grid, 40, 38, rng.random_range(0..5), palette[3]);
    }

    render_grid(&grid);
}

#[cfg(test)]
mod tests {
    use super::*;
    use unicode_width::UnicodeWidthStr;

    fn assert_uniform_display_width(grid: &Grid, expected: usize) {
        let lines = grid_to_plain(grid);
        for (i, line) in lines.iter().enumerate() {
            let w = UnicodeWidthStr::width(line.as_str());
            assert_eq!(
                w, expected,
                "row {} has display width {} (expected {}): {:?}",
                i, w, expected, line,
            );
        }
    }

    fn make_grid(width: usize, height: usize, seed: u64) -> (Grid, StdRng, [Color; 5]) {
        let grid = vec![vec![Cell::blank(); width]; height];
        let rng = StdRng::seed_from_u64(seed);
        let palette = make_palette(seed);
        (grid, rng, palette)
    }

    #[test]
    fn mondrian_display_width() {
        let (mut grid, mut rng, _) = make_grid(80, 45, 42);
        let blocks = vec![
            ContentBlock {
                items: vec![
                    ContentItem::Text("「 STATUS 」".into()),
                    ContentItem::Rule,
                    ContentItem::Text("All systems operational.".into()),
                ],
                padding: 1,
            },
            ContentBlock {
                items: vec![
                    ContentItem::Text("METRICS".into()),
                    ContentItem::Rule,
                    ContentItem::Bar {
                        label: "cpu".into(),
                        value: 72.0,
                        max: 100.0,
                    },
                    ContentItem::Bar {
                        label: "mem".into(),
                        value: 4.8,
                        max: 8.0,
                    },
                ],
                padding: 1,
            },
            ContentBlock {
                items: vec![
                    ContentItem::Text("「 SKILLS 」".into()),
                    ContentItem::Rule,
                    ContentItem::Text("typespec ···· 12".into()),
                    ContentItem::Text("ast-grep ···· 5".into()),
                    ContentItem::Text("tree-sit ···· 3".into()),
                ],
                padding: 1,
            },
        ];
        let (_, line_color) = mondrian_colors();
        let text_fg = rgb(20, 20, 20);
        let (fills, _) = mondrian_colors();
        layout_mondrian(
            &mut grid, &blocks, 0, 2, 10, 5, text_fg, line_color, &fills, line_color, &mut rng,
        );
        assert_uniform_display_width(&grid, 80);
    }

    #[test]
    fn mondrian_different_seeds_display_width() {
        for seed in [0, 1, 7, 42, 99, 1234] {
            let (mut grid, mut rng, _) = make_grid(80, 45, seed);
            let blocks = vec![ContentBlock {
                items: vec![
                    ContentItem::Text("「 STATUS 」".into()),
                    ContentItem::Rule,
                    ContentItem::Text("Online.".into()),
                ],
                padding: 1,
            }];
            let (fills, line_color) = mondrian_colors();
            layout_mondrian(
                &mut grid,
                &blocks,
                0,
                2,
                10,
                5,
                rgb(20, 20, 20),
                line_color,
                &fills,
                line_color,
                &mut rng,
            );
            assert_uniform_display_width(&grid, 80);
        }
    }

    #[test]
    fn default_mode_display_width() {
        let (mut grid, mut rng, palette) = make_grid(80, 45, 42);
        let truchet_color = darken(palette[1], 80);
        let tiles = ['╱', '╲'];
        for y in 0..45 {
            for x in 0..80 {
                grid[y][x] = Cell::new(tiles[rng.random_range(0..2)], truchet_color);
            }
        }
        let cx = 40;
        let cy = 22;
        let lines = ["「 技 」 S K I L L S", "", "  typespec ···· 12"];
        for (i, line) in lines.iter().enumerate() {
            let mut col = 0usize;
            for ch in line.chars() {
                let cw = char_width(ch);
                let gx = cx - 15 + col;
                if gx < 80 {
                    grid[cy - 5 + 1 + i][gx] = Cell::new(ch, palette[4]);
                    if cw == 2 && gx + 1 < 80 {
                        grid[cy - 5 + 1 + i][gx + 1] = Cell::blank();
                    }
                }
                col += cw;
            }
        }
        assert_uniform_display_width(&grid, 80);
    }

    #[test]
    fn bsp_display_width() {
        let (mut grid, mut rng, palette) = make_grid(80, 45, 42);
        let truchet_color = darken(palette[1], 90);
        let tiles = ['╱', '╲'];
        for y in 0..45 {
            for x in 0..80 {
                grid[y][x] = Cell::new(tiles[rng.random_range(0..2)], truchet_color);
            }
        }
        let blocks = vec![
            ContentBlock {
                items: vec![
                    ContentItem::Text("「 STATUS 」".into()),
                    ContentItem::Rule,
                    ContentItem::Text("All systems operational.".into()),
                ],
                padding: 1,
            },
            ContentBlock {
                items: vec![
                    ContentItem::Text("TASKS".into()),
                    ContentItem::Rule,
                    ContentItem::Text("▪ layout engine".into()),
                ],
                padding: 1,
            },
        ];
        layout_bsp(
            &mut grid, &blocks, 1, 12, 5, palette[4], palette[3], &mut rng,
        );
        assert_uniform_display_width(&grid, 80);
    }

    #[test]
    fn wrap_text_fullwidth_chars() {
        let lines = wrap_text("「 X 」", 7);
        assert_eq!(lines, vec!["「 X 」"]);

        let lines = wrap_text("「 X 」 extra", 7);
        assert_eq!(lines, vec!["「 X 」", "extra"]);
    }

    #[test]
    fn wrap_text_ascii_basic() {
        let lines = wrap_text("hello world foo", 11);
        assert_eq!(lines, vec!["hello world", "foo"]);
    }

    #[test]
    fn min_block_width_accounts_for_fullwidth() {
        let block = ContentBlock {
            items: vec![ContentItem::Text("「 SKILLS 」".into())],
            padding: 1,
        };
        assert_eq!(min_block_width(&block), 14);
    }

    #[test]
    fn bsp_split_gap_leaves_cover_canvas() {
        let mut rng = StdRng::seed_from_u64(42);
        let mut root = BspNode::new(0, 0, 80, 45);
        root.split_with_gap(10, 5, 4, 2, &mut rng);
        let leaves = root.leaves();
        assert!(leaves.len() >= 2, "should produce multiple leaves");
        for leaf in &leaves {
            assert!(leaf.x + leaf.w <= 80, "leaf x overflow");
            assert!(leaf.y + leaf.h <= 45, "leaf y overflow");
            assert!(leaf.w >= 10, "leaf too narrow");
            assert!(leaf.h >= 5, "leaf too short");
        }
    }

    #[test]
    fn bsp_split_gap1_backward_compat() {
        let mut rng1 = StdRng::seed_from_u64(99);
        let mut rng2 = StdRng::seed_from_u64(99);
        let mut a = BspNode::new(0, 0, 80, 45);
        let mut b = BspNode::new(0, 0, 80, 45);
        a.split(10, 5, 4, &mut rng1);
        b.split_with_gap(10, 5, 4, 1, &mut rng2);
        let la: Vec<_> = a.leaves().iter().map(|r| (r.x, r.y, r.w, r.h)).collect();
        let lb: Vec<_> = b.leaves().iter().map(|r| (r.x, r.y, r.w, r.h)).collect();
        assert_eq!(la, lb);
    }

    #[test]
    fn mondrian_content_not_wrapped() {
        let (mut grid, mut rng, _) = make_grid(80, 45, 42);
        let blocks = vec![ContentBlock {
            items: vec![ContentItem::Text("「 SKILLS 」".into())],
            padding: 1,
        }];
        let (fills, line_color) = mondrian_colors();
        layout_mondrian(
            &mut grid,
            &blocks,
            0,
            2,
            10,
            5,
            rgb(20, 20, 20),
            line_color,
            &fills,
            line_color,
            &mut rng,
        );
        let lines = grid_to_plain(&grid);
        let skill_rows: Vec<_> = lines.iter().filter(|l| l.contains("SKILLS")).collect();
        assert_eq!(
            skill_rows.len(),
            1,
            "「 SKILLS 」 should appear on exactly one row"
        );
        assert!(
            skill_rows[0].contains("「 SKILLS 」"),
            "full title should be on one line, got: {:?}",
            skill_rows[0]
        );
    }

    #[test]
    fn scene_walk_produces_layers() {
        let mut rng = StdRng::seed_from_u64(42);
        let palette = make_palette(42);
        let mut root = layout::BspNode::new(0, 0, 80, 45);
        root.split_with_gap(12, 6, 4, 2, &mut rng);
        let leaves: Vec<Rect> = root.leaves().into_iter().copied().collect();
        let layers = walk_to_layers(&leaves, (40, 22), &palette, &mut rng);
        assert!(layers.len() > 0, "walker should produce at least one layer");
        assert!(layers.len() <= leaves.len() * 4, "layers bounded by leaves + scatter");
        for layer in &layers {
            assert!(layer.mask.is_some(), "every scene-walk layer should be masked");
        }
    }

    #[test]
    fn scene_walk_renders_without_panic() {
        for seed in [0, 1, 7, 42, 99, 1234] {
            let (mut grid, mut rng, palette) = make_grid(80, 45, seed);
            let mut root = layout::BspNode::new(0, 0, 80, 45);
            root.split_with_gap(12, 6, 4, 2, &mut rng);
            let leaves: Vec<Rect> = root.leaves().into_iter().copied().collect();
            let layers = walk_to_layers(&leaves, (40, 22), &palette, &mut rng);
            let scene = Scene { layers };
            let rect = Rect { x: 0, y: 0, w: 80, h: 45 };
            render_scene(&mut grid, &rect, &scene, &mut rng);
            assert_uniform_display_width(&grid, 80);
        }
    }

    #[test]
    fn scene_walk_deterministic() {
        let run = |seed: u64| {
            let mut rng = StdRng::seed_from_u64(seed);
            let palette = make_palette(seed);
            let mut root = layout::BspNode::new(0, 0, 60, 30);
            root.split_with_gap(10, 5, 4, 2, &mut rng);
            let leaves: Vec<Rect> = root.leaves().into_iter().copied().collect();
            let layers = walk_to_layers(&leaves, (30, 15), &palette, &mut rng);
            let mut grid = vec![vec![Cell::blank(); 60]; 30];
            let rect = Rect { x: 0, y: 0, w: 60, h: 30 };
            let scene = Scene { layers };
            render_scene(&mut grid, &rect, &scene, &mut rng);
            grid_to_plain(&grid)
        };
        assert_eq!(run(42), run(42));
        assert_ne!(run(42), run(99));
    }

    #[test]
    fn tile_edge_seigaiha_skew_deterministic() {
        // Seigaiha with skew should produce identical output for same seed
        let run = |seed: u64| {
            let (mut grid, mut rng, palette) = make_grid(40, 20, seed);
            let rect = Rect { x: 5, y: 3, w: 25, h: 12 };
            let params = TileParams {
                variant: TileVariant::Seigaiha,
                density: 1.0, stagger_override: -1, rhythm_override: 0,
                jitter: 0.0, skew: 60,
            };
            fill_tile_ex(&mut grid, &rect, &params, palette[1], palette[2], 0.0, None, &mut rng);
            grid_to_plain(&grid)
        };
        assert_eq!(run(42), run(42));
        assert_ne!(run(42), run(99));
    }

    #[test]
    fn tile_edge_skew_bleeds_past_rect() {
        // With skew>0, cells outside the rect should get drawn
        let (mut grid, mut rng, palette) = make_grid(40, 20, 42);
        let rect = Rect { x: 10, y: 5, w: 15, h: 8 };
        let params = TileParams {
            variant: TileVariant::Seigaiha,
            density: 1.0, stagger_override: -1, rhythm_override: 0,
            jitter: 0.0, skew: 80,
        };
        fill_tile_ex(&mut grid, &rect, &params, palette[1], palette[2], 0.0, None, &mut rng);

        // Check that at least some cells outside the rect got drawn
        let mut outside_drawn = 0;
        for y in 0..20 {
            for x in 0..40 {
                let inside = x >= rect.x && x < rect.x + rect.w
                          && y >= rect.y && y < rect.y + rect.h;
                if !inside && grid[y][x].ch != ' ' {
                    outside_drawn += 1;
                }
            }
        }
        assert!(outside_drawn > 0, "skew=80 should bleed chars outside the rect");
    }

    #[test]
    fn tile_edge_all_variants_no_panic() {
        // Every variant with skew should render without panic
        for vi in 0..TILE_VARIANT_COUNT {
            let variant = tile_variant_from_index(vi);
            for skew in [0, 30, 60, 100] {
                let (mut grid, mut rng, palette) = make_grid(30, 15, 42);
                let rect = Rect { x: 3, y: 2, w: 20, h: 10 };
                let params = TileParams {
                    variant, density: 1.0, stagger_override: -1,
                    rhythm_override: 0, jitter: 0.0, skew,
                };
                fill_tile_ex(&mut grid, &rect, &params, palette[1], palette[2], 0.0, None, &mut rng);
            }
        }
    }
}
