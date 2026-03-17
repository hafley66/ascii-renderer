mod biomes;
mod borders;
mod color;
mod content;
mod fills;
mod layout;
mod markdown;
mod mondrian;
mod render;
mod sprites;
mod types;
mod walker;

use crossterm::style::Color;
use rand::RngExt;
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::io::{self, IsTerminal, Read as _};

use biomes::*;
use color::*;
use content::*;
use fills::*;
use layout::*;
use markdown::*;
use mondrian::*;
use render::*;
use sprites::*;
use types::*;
use walker::*;

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
        eprintln!("  world     Vertical biome strips: forest, garden, temple, noise, geometric");
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
            fill_tile_ex(&mut grid, &r, &params, c1, c2, jitter, &mut rng);
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
}
