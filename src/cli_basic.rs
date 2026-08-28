use crossterm::style::Color;
use rand::RngExt;
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::io::{self, IsTerminal, Read as _};

use crate::automata::*;
use crate::biomes::*;
use crate::color::*;
use crate::content::*;
use crate::fills::*;
use crate::layout::*;
use crate::markdown::*;
use crate::mondrian::*;
use crate::render::*;
use crate::scene::*;
use crate::sprites::*;
use crate::tree_draw::*;
use crate::types::*;
use crate::walker::*;
use crate::avant::*;
use crate::cli::*;
use crate::gridio::*;
use crate::ink::*;
use crate::modes_creatures::*;
use crate::modes_geo::*;
use crate::modes_sky::*;
use crate::modes_tree::*;
use crate::morph::*;
use crate::opts::*;
use crate::pp::*;
use crate::registry::*;
use crate::warps::*;
use crate::automata; use crate::avant; use crate::biomes; use crate::borders; use crate::color; use crate::content; use crate::fills; use crate::layout; use crate::markdown; use crate::mondrian; use crate::render; use crate::scene; use crate::sprites; use crate::tree_draw; use crate::types; use crate::walker;


/// Dispatch arm for mode(s): swatch (moved verbatim from run()).
pub(crate) fn cli_swatch(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
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
        emit_grid(&swatch_grid);
        return (grid, true);
    (grid, false)
}

/// Dispatch arm for mode(s): tree (moved verbatim from run()).
pub(crate) fn cli_tree(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        grow_tree(&mut grid, 20, 40, 5, 16, palette[1], &mut rng);
        grow_tree(&mut grid, 55, 42, 8, 12, palette[2], &mut rng);

        draw_flower(&mut grid, 10, 42, 0, palette[3]);
        draw_flower(&mut grid, 70, 43, 1, palette[3]);
        draw_flower(&mut grid, 38, 38, 2, palette[3]);
        draw_flower(&mut grid, 45, 20, 3, palette[1]);
        draw_flower(&mut grid, 5, 10, 4, palette[2]);
    (grid, false)
}

/// Dispatch arm for mode(s): trees (moved verbatim from run()).
pub(crate) fn cli_trees(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
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
                draw_tree(
                    &mut grid, cx, root_y, canopy_y, spread, kind, color, &mut rng,
                );
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
    (grid, false)
}

/// Dispatch arm for mode(s): aztec (moved verbatim from run()).
pub(crate) fn cli_aztec(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        draw_aztec_diamond(
            &mut grid,
            width / 2,
            height / 2,
            height / 2 - 2,
            &palette,
            &mut rng,
        );
    (grid, false)
}

/// Dispatch arm for mode(s): fret (moved verbatim from run()).
pub(crate) fn cli_fret(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        draw_stepped_fret(&mut grid, 5, 5, 3, Dir::Right, palette[1]);
        draw_stepped_fret(&mut grid, 25, 5, 5, Dir::Right, palette[2]);
        draw_stepped_fret(&mut grid, 50, 5, 7, Dir::Right, palette[3]);

        draw_stepped_fret(&mut grid, 10, 20, 5, Dir::Right, palette[1]);
        draw_stepped_fret(&mut grid, 30, 30, 5, Dir::Left, palette[2]);

        draw_fret_border(&mut grid, 0, 0, width, height, 4, 0, palette[1]);
        draw_fret_border(&mut grid, 0, 0, width, height, 4, 1, palette[2]);
        draw_fret_border(&mut grid, 0, 0, width, height, 4, 2, palette[3]);
        draw_fret_border(&mut grid, 0, 0, width, height, 4, 3, palette[1]);
    (grid, false)
}

/// Dispatch arm for mode(s): flowers (moved verbatim from run()).
pub(crate) fn cli_flowers(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
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
    (grid, false)
}

/// Dispatch arm for mode(s): fruits (moved verbatim from run()).
pub(crate) fn cli_fruits(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
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
    (grid, false)
}

/// Dispatch arm for mode(s): forest (moved verbatim from run()).
pub(crate) fn cli_forest(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
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
    (grid, false)
}

/// Dispatch arm for mode(s): layout (moved verbatim from run()).
pub(crate) fn cli_layout(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
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
    (grid, false)
}

/// Dispatch arm for mode(s): md (moved verbatim from run()).
pub(crate) fn cli_md(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
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
    (grid, false)
}

/// Dispatch arm for mode(s): bsp (moved verbatim from run()).
pub(crate) fn cli_bsp(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
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
    (grid, false)
}

/// Dispatch arm for mode(s): mondrian (moved verbatim from run()).
pub(crate) fn cli_mondrian(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
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
    (grid, false)
}

/// Dispatch arm for mode(s): tiles (moved verbatim from run()).
pub(crate) fn cli_tiles(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
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
    (grid, false)
}

/// Dispatch arm for mode(s): tiles-rand (moved verbatim from run()).
pub(crate) fn cli_tiles_rand(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
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
    (grid, false)
}

/// Dispatch arm for mode(s): tiles-skew (moved verbatim from run()).
pub(crate) fn cli_tiles_skew(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
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
    (grid, false)
}

/// Dispatch arm for mode(s): terrain (moved verbatim from run()).
pub(crate) fn cli_terrain(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        let rect = Rect {
            x: 0,
            y: 0,
            w: width,
            h: height,
        };
        render_terrain(&mut grid, &rect, &palette, &mut rng);
    (grid, false)
}

/// Dispatch arm for mode(s): flow (moved verbatim from run()).
pub(crate) fn cli_flow(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        let rect = Rect {
            x: 0,
            y: 0,
            w: width,
            h: height,
        };
        let zones = random_flow(&rect, &palette, &mut rng);
        render_flow(&mut grid, &rect, &zones, &palette, &mut rng);
    (grid, false)
}

/// Dispatch arm for mode(s): watershed (moved verbatim from run()).
pub(crate) fn cli_watershed(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        // watershed [channels] -- terrain contours carved by tapered, dissolving flow strips
        let channel_count: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(3);
        let channel_count = channel_count.clamp(1, 6);
        let rect = Rect {
            x: 0,
            y: 0,
            w: width,
            h: height,
        };

        let ctx = terrain_scene(&rect, &palette, &mut rng);
        render_scene(&mut grid, &rect, &ctx.scene, &mut rng);
        terrain_post_pass(&mut grid, &rect, &ctx, &palette, &mut rng);

        let water_palette = [
            palette[0],
            lighten(palette[2], 28),
            lighten(palette[3], 18),
            lighten(shift_hue(palette[1], 45.0), 10),
            palette[4],
        ];

        let mut channel_centers: Vec<(usize, usize, f32, f32, f32)> = Vec::new();
        for ci in 0..channel_count {
            let slot_w = width / (channel_count + 1);
            let base_x = (slot_w * (ci + 1)
                + rng
                    .random_range(0..slot_w.max(1))
                    .saturating_sub(slot_w / 2))
            .clamp(4, width.saturating_sub(5).max(4));
            let channel_w = rng.random_range(12..24usize).min(width.max(1)).max(6);
            let phase = rng.random::<f32>() * std::f32::consts::TAU;
            let amp = rng.random_range(3..(width / 9).max(5) as u32) as f32;
            let freq = rng.random_range(9..21u32) as f32;
            channel_centers.push((base_x, channel_w, phase, amp, freq));

            let mut tile = TileParams::new(TileVariant::Seigaiha);
            tile.density = rng.random_range(70..95) as f32 / 100.0;
            tile.jitter = 0.04;
            tile.skew = 35;
            let runnel_fill = match ci % 4 {
                0 => FillGen::Tile(tile),
                1 => FillGen::Zigzag,
                2 => FillGen::Noise(NoiseVariant::Higaki),
                _ => FillGen::Weave,
            };
            let zones = vec![
                FlowZone {
                    fill: FillGen::Noise(NoiseVariant::Dot),
                    height_frac: 0.18,
                    taper: Taper::Opening,
                    width_start: 0.18,
                    width_end: 0.55,
                },
                FlowZone {
                    fill: runnel_fill,
                    height_frac: 0.27,
                    taper: Taper::Diamond,
                    width_start: 0.42,
                    width_end: 0.96,
                },
                FlowZone {
                    fill: FillGen::Tile(tile),
                    height_frac: 0.27,
                    taper: Taper::Constant,
                    width_start: 0.68,
                    width_end: 0.68,
                },
                FlowZone {
                    fill: FillGen::Noise(NoiseVariant::Grass),
                    height_frac: 0.28,
                    taper: Taper::Closing,
                    width_start: 0.95,
                    width_end: 0.48,
                },
            ];

            let mut flow_grid = vec![vec![Cell::blank(); channel_w]; height];
            let flow_rect = Rect {
                x: 0,
                y: 0,
                w: channel_w,
                h: height,
            };
            render_flow(&mut flow_grid, &flow_rect, &zones, &water_palette, &mut rng);

            for y in 0..height {
                let depth_bias = if y > height * 2 / 3 { 1.35 } else { 1.0 };
                let center = base_x as f32
                    + (y as f32 / freq + phase).sin() * amp * depth_bias
                    + (y as f32 / (freq * 0.57) + phase * 0.3).sin() * amp * 0.35;
                let center = center.round() as i32;
                for lx in 0..channel_w {
                    let cell = flow_grid[y][lx];
                    let local = lx as i32 - channel_w as i32 / 2;
                    let x = center + local;
                    if x < 0 || (x as usize) >= width || cell.ch == ' ' {
                        continue;
                    }
                    let edge = local.unsigned_abs() as f32 / (channel_w as f32 / 2.0).max(1.0);
                    if edge > 0.82 && rng.random::<f32>() < edge - 0.45 {
                        continue;
                    }
                    grid[y][x as usize] = cell;

                    if edge > 0.62 && rng.random::<f32>() < 0.28 {
                        let bank_x = x + if local < 0 { -1 } else { 1 };
                        if bank_x >= 0 && (bank_x as usize) < width {
                            let ch = DISSOLVE[rng.random_range(2..6)];
                            grid[y][bank_x as usize] = Cell::new(ch, darken(water_palette[1], 35));
                        }
                    }
                }
            }
        }

        // Pools and brighter shelves where flow channels meet terrain contours.
        for &(base_x, channel_w, phase, amp, freq) in &channel_centers {
            let col = base_x.min(width - 1);
            let crossings = [
                ctx.mountain_contour[col],
                ctx.foothill_contour[col],
                ctx.ground_contour[col],
            ];
            for (i, &cy) in crossings.iter().enumerate() {
                if cy >= height {
                    continue;
                }
                let center = base_x as f32
                    + (cy as f32 / freq + phase).sin() * amp
                    + (cy as f32 / (freq * 0.57) + phase * 0.3).sin() * amp * 0.35;
                let center = center.round() as i32;
                let rx = (channel_w as i32 / 3).max(3) + i as i32;
                let ry = 1 + i as i32;
                for dy in -ry..=ry {
                    for dx in -rx..=rx {
                        let x = center + dx;
                        let y = cy as i32 + dy;
                        if x < 0 || y < 0 || (x as usize) >= width || (y as usize) >= height {
                            continue;
                        }
                        let nx = dx as f32 / rx as f32;
                        let ny = dy as f32 / ry.max(1) as f32;
                        if nx * nx + ny * ny <= 1.0 {
                            let ch = ['≈', '~', '∿', '─'][rng.random_range(0..4usize)];
                            grid[y as usize][x as usize] =
                                Cell::new(ch, lighten(water_palette[2], 10));
                        }
                    }
                }
            }
        }

        draw_contour_ridge(
            &mut grid,
            &rect,
            &ctx.mountain_contour,
            lighten(palette[1], 35),
        );
        draw_contour_ridge(
            &mut grid,
            &rect,
            &ctx.foothill_contour,
            lighten(palette[2], 15),
        );
        draw_contour_ridge(
            &mut grid,
            &rect,
            &ctx.ground_contour,
            lighten(palette[3], 25),
        );
    (grid, false)
}

/// Dispatch arm for mode(s): masks (moved verbatim from run()).
pub(crate) fn cli_masks(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        // background: diamond lattice to recreate the emergent effect
        let bg_rect = Rect {
            x: 0,
            y: 0,
            w: width,
            h: height,
        };
        draw_diamond_lattice(
            &mut grid,
            &bg_rect,
            darken(palette[1], 60),
            darken(palette[1], 80),
        );
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
    (grid, false)
}

/// Dispatch arm for mode(s): ca (moved verbatim from run()).
pub(crate) fn cli_ca(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
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
            "cave" => (0.50, 5),
            "maze" => (0.38, 12),
            "coral" => (0.50, 8),
            _ => (0.30, 8),
        };

        let rect = Rect {
            x: 0,
            y: 0,
            w: width,
            h: height,
        };
        render_automata(
            &mut grid, &rect, rule_name, density, gens, style, &palette, true, &mut rng,
        );
    (grid, false)
}

/// Dispatch arm for mode(s): ca-layout (moved verbatim from run()).
pub(crate) fn cli_ca_layout(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        let rect = Rect {
            x: 0,
            y: 0,
            w: width,
            h: height,
        };

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
                ],
                padding: 1,
            },
        ];

        let text_rects = ca_layout(&mut grid, &rect, "life", 0.35, 6, &palette, &mut rng);

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
    (grid, false)
}

/// Dispatch arm for mode(s): shapes (moved verbatim from run()).
pub(crate) fn cli_shapes(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        // 2x2 grid, shapes sized to ~30% of each quadrant, hard edges (dissolve=0).
        // rx = 2*ry throughout to correct for 2:1 terminal cell aspect ratio.
        let hw = width / 2;
        let hh = height / 2;
        let cxs = [hw / 2, hw + hw / 2];
        let cys = [hh / 2, hh + hh / 2];

        // label just above the shape
        let write_label = |grid: &mut Grid, lx: usize, ly: usize, text: &str, color: Color| {
            for (j, ch) in text.chars().enumerate() {
                if lx + j < width && ly < grid.len() {
                    grid[ly][lx + j] = Cell::new(ch, color);
                }
            }
        };

        // 1 -- Diamond (top-left)
        {
            let cx = cxs[0] as f32;
            let cy = cys[0] as f32;
            let ry = hh as f32 * 0.30;
            let rx = ry * 2.0;
            let r = Rect {
                x: 1,
                y: 1,
                w: hw - 2,
                h: hh - 2,
            };
            let scene = Scene {
                layers: vec![Layer {
                    fill: FillGen::Tile(TileParams::new(TileVariant::BishamonKikko)),
                    mask: Some(Box::new(mask_diamond(cx, cy, rx, ry, 0.0))),
                    palette,
                }],
            };
            render_scene(&mut grid, &r, &scene, &mut rng);
            let lx = cxs[0].saturating_sub(3);
            let ly = (cy - ry - 2.0).max(1.0) as usize;
            write_label(&mut grid, lx, ly, "diamond", palette[4]);
        }

        // 2 -- Parallelogram (top-right)
        {
            let cx = cxs[1] as f32;
            let cy = cys[0] as f32;
            let w = hw as f32 * 0.50;
            let h = hh as f32 * 0.55;
            let r = Rect {
                x: hw + 1,
                y: 1,
                w: hw - 2,
                h: hh - 2,
            };
            let scene = Scene {
                layers: vec![Layer {
                    fill: FillGen::Tile(TileParams::new(TileVariant::Asanoha)),
                    mask: Some(Box::new(mask_parallelogram(cx, cy, w, h, 8.0, 0.0))),
                    palette,
                }],
            };
            render_scene(&mut grid, &r, &scene, &mut rng);
            let lx = cxs[1].saturating_sub(6);
            let ly = (cy - h * 0.5 - 2.0).max(1.0) as usize;
            write_label(&mut grid, lx, ly, "parallelogram", palette[4]);
        }

        // 3 -- Triangle apex-up (bottom-left)
        {
            let cx = cxs[0] as f32;
            let cy = cys[1] as f32;
            let ry = hh as f32 * 0.35;
            let rx = ry * 2.0;
            let r = Rect {
                x: 1,
                y: hh + 1,
                w: hw - 2,
                h: hh - 2,
            };
            let scene = Scene {
                layers: vec![Layer {
                    fill: FillGen::Tile(TileParams::new(TileVariant::Yabane)),
                    mask: Some(Box::new(mask_triangle(cx, cy, rx, ry, TriDir::Up, 0.0))),
                    palette,
                }],
            };
            render_scene(&mut grid, &r, &scene, &mut rng);
            let lx = cxs[0].saturating_sub(3);
            let ly = (cy - ry - 2.0).max((hh + 1) as f32) as usize;
            write_label(&mut grid, lx, ly, "triangle", palette[4]);
        }

        // 4 -- Trapezoid wide-at-bottom (bottom-right)
        {
            let cx = cxs[1] as f32;
            let cy = cys[1] as f32;
            let h = hh as f32 * 0.55;
            let w_top = hw as f32 * 0.12;
            let w_bot = hw as f32 * 0.55;
            let r = Rect {
                x: hw + 1,
                y: hh + 1,
                w: hw - 2,
                h: hh - 2,
            };
            let scene = Scene {
                layers: vec![Layer {
                    fill: FillGen::Tile(TileParams::new(TileVariant::Higaki)),
                    mask: Some(Box::new(mask_trapezoid(cx, cy, w_top, w_bot, h, 0.0))),
                    palette,
                }],
            };
            render_scene(&mut grid, &r, &scene, &mut rng);
            let lx = cxs[1].saturating_sub(4);
            let ly = (cy - h * 0.5 - 2.0).max((hh + 1) as f32) as usize;
            write_label(&mut grid, lx, ly, "trapezoid", palette[4]);
        }

        // grid dividers
        for y in 0..height {
            if y < grid.len() {
                grid[y][hw] = Cell::new('│', darken(palette[2], 50));
            }
        }
        for x in 0..width {
            if hh < grid.len() {
                grid[hh][x] = Cell::new('─', darken(palette[2], 50));
            }
        }
        if hh < grid.len() {
            grid[hh][hw] = Cell::new('┼', darken(palette[2], 50));
        }
    (grid, false)
}

/// Dispatch arm for mode(s): mondrian2 (moved verbatim from run()).
pub(crate) fn cli_mondrian2(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
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
            if inset.w < 3 || inset.h < 3 {
                continue;
            }

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
                            if y < height && x < width {
                                grid[y][x] = Cell::blank();
                            }
                        }
                    }
                    draw_tree(
                        &mut grid,
                        tx,
                        root,
                        canopy,
                        spread,
                        rng.random_range(0..12),
                        color,
                        &mut rng,
                    );
                }
                2 => {
                    // Flower garden -- clear + scatter flowers
                    for y in inset.y..inset.y + inset.h {
                        for x in inset.x..inset.x + inset.w {
                            if y < height && x < width {
                                grid[y][x] = Cell::blank();
                            }
                        }
                    }
                    let cx = inset.x + inset.w / 2;
                    let cy = inset.y + inset.h / 2;
                    draw_flower(&mut grid, cx, cy, rng.random_range(0..5), palette[3]);
                    let count = rng.random_range(2..6u32);
                    for _ in 0..count {
                        let angle = rng.random::<f32>() * std::f32::consts::TAU;
                        let r =
                            rng.random_range(2..((inset.w.min(inset.h) / 2).max(3)) as u32) as f32;
                        let fx = (cx as f32 + angle.cos() * r * 1.5) as usize;
                        let fy = (cy as f32 + angle.sin() * r * 0.7) as usize;
                        if fx > inset.x
                            && fx < inset.x + inset.w - 1
                            && fy > inset.y
                            && fy < inset.y + inset.h - 1
                        {
                            draw_flower(
                                &mut grid,
                                fx,
                                fy,
                                rng.random_range(0..5),
                                palette[rng.random_range(2..4)],
                            );
                        }
                    }
                }
                3 => {
                    // Rain in this cell only
                    let rain_color = darken(palette[2], 40);
                    let rain_chars = ['│', '┊', '╎', '┆'];
                    for y in inset.y..inset.y + inset.h {
                        for x in inset.x..inset.x + inset.w {
                            if y >= height || x >= width {
                                continue;
                            }
                            if grid[y][x].ch != ' ' {
                                continue;
                            }
                            if rng.random::<f32>() > 0.12 {
                                continue;
                            }
                            let streak = ((x * 7 + 13) % 11) < 3;
                            if !streak && rng.random::<f32>() > 0.3 {
                                continue;
                            }
                            let ch = rain_chars[rng.random_range(0..rain_chars.len())];
                            grid[y][x] = Cell::new(ch, darken(rain_color, rng.random_range(0..20)));
                        }
                    }
                }
                4 => {
                    // Fruit still life
                    for y in inset.y..inset.y + inset.h {
                        for x in inset.x..inset.x + inset.w {
                            if y < height && x < width {
                                grid[y][x] = Cell::blank();
                            }
                        }
                    }
                    let count = rng.random_range(2..5u32);
                    for _ in 0..count {
                        let fx = inset.x
                            + rng.random_range(2..inset.w.saturating_sub(2).max(3) as u32) as usize;
                        let fy = inset.y
                            + rng.random_range(1..inset.h.saturating_sub(2).max(2) as u32) as usize;
                        draw_fruit(
                            &mut grid,
                            fx,
                            fy,
                            rng.random_range(0..5),
                            palette[rng.random_range(1..4)],
                        );
                    }
                }
                5 => {
                    // Stars / night sky in this cell
                    let star_color = lighten(palette[4], 20);
                    let star_chars = ['·', '∙', '°', '*', '⋅', '✦'];
                    for y in inset.y..inset.y + inset.h {
                        for x in inset.x..inset.x + inset.w {
                            if y >= height || x >= width {
                                continue;
                            }
                            if grid[y][x].ch != ' ' {
                                continue;
                            }
                            if rng.random::<f32>() > 0.06 {
                                continue;
                            }
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
    (grid, false)
}

/// Dispatch arm for mode(s): quilt (moved verbatim from run()).
pub(crate) fn cli_quilt(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        // quilt [min_patch] [max_patch] -- stitched patchwork of tile patterns
        let min_p: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(10);
        let max_p: usize = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(22);
        let min_p = min_p.clamp(4, 30);
        let max_p = max_p.clamp(min_p + 1, 40);

        let col_strips = allocate_strips(width, min_p, max_p, &mut rng);
        let row_strips = allocate_strips(height, (min_p / 2).max(3), (max_p / 2).max(4), &mut rng);

        let mut patch_rects: Vec<Rect> = Vec::new();
        for &(ry, rh) in &row_strips {
            for &(cx, cw) in &col_strips {
                let r = Rect {
                    x: cx,
                    y: ry,
                    w: cw,
                    h: rh,
                };
                let variant = tile_variant_from_index(rng.random_range(0..TILE_VARIANT_COUNT));
                let c1 = darken(palette[1 + rng.random_range(0..3)], rng.random_range(0..50));
                let c2 = darken(
                    palette[1 + rng.random_range(0..3)],
                    rng.random_range(20..70),
                );
                fill_tile_pure(&mut grid, &r, variant, c1, c2);
                patch_rects.push(r);
            }
        }

        // stitched seams between patches
        let thread = darken(palette[4], 40);
        for &(cx, _) in col_strips.iter().skip(1) {
            for y in 0..height {
                grid[y][cx] = Cell::new('┆', thread);
            }
        }
        for &(ry, _) in row_strips.iter().skip(1) {
            for x in 0..width {
                grid[ry][x] = Cell::new('┄', thread);
            }
        }
        for &(ry, _) in row_strips.iter().skip(1) {
            for &(cx, _) in col_strips.iter().skip(1) {
                grid[ry][cx] = Cell::new('+', thread);
            }
        }

        // applique: stamp flowers on a few of the larger patches
        let mut candidates: Vec<&Rect> = patch_rects
            .iter()
            .filter(|r| r.w >= 9 && r.h >= 7)
            .collect();
        for _ in 0..3 {
            if candidates.is_empty() {
                break;
            }
            let idx = rng.random_range(0..candidates.len());
            let r = candidates.remove(idx);
            let cx = r.x + r.w / 2;
            let cy = r.y + r.h / 2;
            draw_flower(
                &mut grid,
                cx,
                cy,
                rng.random_range(0..5),
                lighten(palette[3], 20),
            );
        }
    (grid, false)
}

/// Dispatch arm for mode(s): world (moved verbatim from run()).
pub(crate) fn cli_world(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        render_world(&mut grid, width, height, &palette, &mut rng);
    (grid, false)
}

/// Dispatch arm for mode(s): default (moved verbatim from run()).
pub(crate) fn cli_default(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
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
    (grid, false)
}
