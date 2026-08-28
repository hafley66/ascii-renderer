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


/// Dispatch arm for mode(s): boles1 (moved verbatim from run()).
pub(crate) fn cli_boles1(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        // boles1: bole styles at 3 energy levels (low/mid/high)
        let styles = [
            "Crescent", "Braille", "Frame", "Diamond", "Chevron", "Frame2",
        ];
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
                if ground_y < 2 || ground_y as usize >= height - 2 {
                    continue;
                }

                let plot_w = (col_w as i32 - 2).max(6);
                let tp = TreeParams {
                    plot: Rect {
                        x: (cx - plot_w / 2).max(0) as usize,
                        y: 0,
                        w: plot_w as usize,
                        h: (ground_y + 1) as usize,
                    },
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
    (grid, false)
}

/// Dispatch arm for mode(s): boles2 (moved verbatim from run()).
pub(crate) fn cli_boles2(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        // boles2: experimental bole styles v2
        let styles = [
            "Crescent2",
            "Braille2",
            "Frame3",
            "Diamond2",
            "Chevron2",
            "Frame4",
        ];
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
                if ground_y < 2 || ground_y as usize >= height - 2 {
                    continue;
                }

                let plot_w = (col_w as i32 - 2).max(6);
                let tp = TreeParams {
                    plot: Rect {
                        x: (cx - plot_w / 2).max(0) as usize,
                        y: 0,
                        w: plot_w as usize,
                        h: (ground_y + 1) as usize,
                    },
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
    (grid, false)
}

/// Dispatch arm for mode(s): boles3 (moved verbatim from run()).
pub(crate) fn cli_boles3(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        // boles3: refined bole styles with descriptive names
        let styles = [
            "Croissant",
            "Braille",
            "Frame",
            "Keel",
            "Chevron",
            "Buttress",
        ];
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
                if ground_y < 2 || ground_y as usize >= height - 2 {
                    continue;
                }

                let plot_w = (col_w as i32 - 2).max(6);
                let tp = TreeParams {
                    plot: Rect {
                        x: (cx - plot_w / 2).max(0) as usize,
                        y: 0,
                        w: plot_w as usize,
                        h: (ground_y + 1) as usize,
                    },
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
    (grid, false)
}

/// Dispatch arm for mode(s): boles4 (moved verbatim from run()).
pub(crate) fn cli_boles4(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        // boles4: winding bole styles (24-27)
        let styles = ["Serpent", "Braid", "Coil", "Taproot"];
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
                let ground_y = ((ei + 1) * row_h - 3) as i32;
                if ground_y < 2 || ground_y as usize >= height - 2 {
                    continue;
                }

                let plot_w = (col_w as i32 - 2).max(6);
                let tp = TreeParams {
                    plot: Rect {
                        x: (cx - plot_w / 2).max(0) as usize,
                        y: 0,
                        w: plot_w as usize,
                        h: (ground_y + 1) as usize,
                    },
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

                let bole = Bole { style: si + 24 };
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
    (grid, false)
}

/// Dispatch arm for mode(s): boles5 (moved verbatim from run()).
pub(crate) fn cli_boles5(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        // boles5: structural bole styles (28-33)
        let styles = ["Stilts", "Cairn", "Hollow", "Talon", "Tiers", "Tussock"];
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
                let ground_y = ((ei + 1) * row_h - 4) as i32;
                if ground_y < 2 || ground_y as usize >= height - 2 {
                    continue;
                }

                let plot_w = (col_w as i32 - 2).max(6);
                let tp = TreeParams {
                    plot: Rect {
                        x: (cx - plot_w / 2).max(0) as usize,
                        y: 0,
                        w: plot_w as usize,
                        h: (ground_y + 1) as usize,
                    },
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

                let bole = Bole { style: si + 28 };
                let exit = bole.draw(&mut grid, &tp, &mut rng);
                let (tx, ty) = (exit.x, exit.y);

                for y in (ground_y - (row_h as i32 / 2) + 1)..ty {
                    if y >= 0 && (y as usize) < height && (tx as usize) < width {
                        grid[y as usize][tx as usize] = Cell::new('│', color);
                    }
                }

                if si == 0 {
                    let elabel = energy_labels[ei];
                    let ly = ground_y as usize;
                    if ly < height {
                        for (j, ch) in elabel.chars().enumerate() {
                            if j < cx as usize - 5 {
                                grid[ly][j] = Cell::new(ch, rgb(120, 120, 120));
                            }
                        }
                    }
                }
            }
        }
    (grid, false)
}

/// Dispatch arm for mode(s): trunks1 (moved verbatim from run()).
pub(crate) fn cli_trunks1(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        // trunks1: horizontal trunk algorithms + direction-aware branching
        let labels = [
            "Straight", "Wobble", "Organic", "Sine(2)", "Sine(4)", "Gnarled",
        ];
        let col_w = width / labels.len();
        let ground_y = (height as i32) - 3;

        for (i, label) in labels.iter().enumerate() {
            let cx = (i * col_w + col_w / 2) as i32;
            let color = palette[i % palette.len()];

            let plot = Rect {
                x: (i * col_w).max(1),
                y: 2,
                w: col_w.min(20),
                h: (ground_y as usize).saturating_sub(2),
            };
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
            use tree_draw::{
                GnarledTrunk, OrganicTrunk, SineTrunk, StraightTrunk, TreeWithTrunk, WobbleTrunk,
            };

            let tree = SpiralTree;
            match i {
                0 => TreeWithTrunk {
                    tree,
                    trunk: Box::new(StraightTrunk {
                        height_fraction: 0.5,
                    }),
                }
                .grow(&mut grid, &params, &mut rng),
                1 => TreeWithTrunk {
                    tree,
                    trunk: Box::new(WobbleTrunk {
                        height_fraction: 0.5,
                    }),
                }
                .grow(&mut grid, &params, &mut rng),
                2 => TreeWithTrunk {
                    tree,
                    trunk: Box::new(OrganicTrunk {
                        height_fraction: 0.5,
                    }),
                }
                .grow(&mut grid, &params, &mut rng),
                3 => TreeWithTrunk {
                    tree,
                    trunk: Box::new(SineTrunk {
                        height_fraction: 0.3,
                        amplitude: 2,
                    }),
                }
                .grow(&mut grid, &params, &mut rng),
                4 => TreeWithTrunk {
                    tree,
                    trunk: Box::new(SineTrunk {
                        height_fraction: 0.3,
                        amplitude: 3,
                    }),
                }
                .grow(&mut grid, &params, &mut rng),
                5 => TreeWithTrunk {
                    tree,
                    trunk: Box::new(GnarledTrunk),
                }
                .grow(&mut grid, &params, &mut rng),
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
    (grid, false)
}

/// Dispatch arm for mode(s): trees1 (moved verbatim from run()).
pub(crate) fn cli_trees1(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        // trees1: full pipeline demo -- tree + trunk algo + bole
        // args: [energy] [fruit_factor] [branch_factor] [bole_override]
        let energy: f32 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(0.8);
        let fruit_factor: f32 = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(0.3);
        let branch_factor: f32 = args.get(6).and_then(|s| s.parse().ok()).unwrap_or(0.5);
        let bole_override: Option<usize> = args.get(7).and_then(|s| s.parse().ok());

        let combos: Vec<(&str, Box<dyn TreeDrawer>, usize)> = vec![
            (
                "Spiral+Straight\n+Frame",
                Box::new(SpiralTree) as Box<dyn TreeDrawer>,
                14,
            ),
            (
                "Spiral+Wobble\n+Chevron",
                Box::new(TreeWithTrunk {
                    tree: SpiralTree,
                    trunk: Box::new(WobbleTrunk {
                        height_fraction: 0.6,
                    }),
                }),
                16,
            ),
            (
                "Candelabra+Organic\n+Keel",
                Box::new(TreeWithTrunk {
                    tree: CandelabraTree,
                    trunk: Box::new(OrganicTrunk {
                        height_fraction: 0.5,
                    }),
                }),
                15,
            ),
            (
                "Split+Sine\n+Buttress",
                Box::new(TreeWithTrunk {
                    tree: SplitTree,
                    trunk: Box::new(SineTrunk {
                        height_fraction: 0.3,
                        amplitude: 2,
                    }),
                }),
                17,
            ),
            (
                "Birch+Gnarled\n+Braille",
                Box::new(TreeWithTrunk {
                    tree: BirchTree,
                    trunk: Box::new(GnarledTrunk),
                }),
                13,
            ),
            (
                "Drooping+Sine\n+Frame",
                Box::new(TreeWithTrunk {
                    tree: DroopingTree,
                    trunk: Box::new(SineTrunk {
                        height_fraction: 0.3,
                        amplitude: 3,
                    }),
                }),
                14,
            ),
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
    (grid, false)
}

/// Dispatch arm for mode(s): trees2 (moved verbatim from run()).
pub(crate) fn cli_trees2(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        // trees2: squat horizontal boles (styles 18-23) + tree combos
        let energy: f32 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(0.8);
        let fruit_factor: f32 = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(0.2);
        let branch_factor: f32 = args.get(6).and_then(|s| s.parse().ok()).unwrap_or(0.5);

        let combos: Vec<(&str, Box<dyn TreeDrawer>, usize)> = vec![
            (
                "Spiral\n+SqCrescent",
                Box::new(SpiralTree) as Box<dyn TreeDrawer>,
                18,
            ),
            (
                "Spiral+Wobble\n+SqBraille",
                Box::new(TreeWithTrunk {
                    tree: SpiralTree,
                    trunk: Box::new(WobbleTrunk {
                        height_fraction: 0.6,
                    }),
                }),
                19,
            ),
            (
                "Candelabra\n+SqFrame",
                Box::new(CandelabraTree) as Box<dyn TreeDrawer>,
                20,
            ),
            (
                "Split+Sine\n+SqDiamond",
                Box::new(TreeWithTrunk {
                    tree: SplitTree,
                    trunk: Box::new(SineTrunk {
                        height_fraction: 0.3,
                        amplitude: 2,
                    }),
                }),
                21,
            ),
            (
                "Birch\n+SqChevron",
                Box::new(BirchTree) as Box<dyn TreeDrawer>,
                22,
            ),
            (
                "Drooping\n+SqButtress",
                Box::new(DroopingTree) as Box<dyn TreeDrawer>,
                23,
            ),
            (
                "WavyBirch\n+SqCrescent",
                Box::new(WavyBirch) as Box<dyn TreeDrawer>,
                18,
            ),
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
                taper: [
                    TaperKind::Diagonal,
                    TaperKind::Shelf,
                    TaperKind::Bracket,
                    TaperKind::Step,
                    TaperKind::Melt,
                    TaperKind::Shelf,
                    TaperKind::Bracket,
                ][i % 7],
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
    (grid, false)
}

/// Dispatch arm for mode(s): trees3 (moved verbatim from run()).
pub(crate) fn cli_trees3(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        // trees3: vertical catalog -- all tree types, trunk algos, taper styles, bole styles
        let page_w = 80usize;
        let tree_h = 28usize;
        let label_h = 2usize;
        let section_gap = 2usize;
        let header_h = 2usize;

        // Section heights
        let sec1_h = header_h + 2 * (tree_h + label_h) + section_gap; // 8 tree types
        let sec2_h = header_h + tree_h + label_h + section_gap; // 7 trunk algos
        let sec3_h = header_h + tree_h + label_h + section_gap; // 5 taper styles
        let bole_tree_h = 20usize;
        let sec4_h = header_h + 3 * (bole_tree_h + label_h) + section_gap; // 24 bole styles

        let page_h = sec1_h + sec2_h + sec3_h + sec4_h + 4;
        let mut pg = vec![vec![Cell::blank(); page_w]; page_h];

        let energy = 0.8f32;

        let write_header = |pg: &mut Vec<Vec<Cell>>, y: usize, text: &str, color: Color| {
            let lx = (page_w / 2).saturating_sub(text.len() / 2);
            for (j, ch) in text.chars().enumerate() {
                if lx + j < page_w {
                    pg[y][lx + j] = Cell::new(ch, color);
                }
            }
            for x in 0..page_w {
                pg[y + 1][x] = Cell::new('─', darken(color, 30));
            }
        };

        let write_label =
            |pg: &mut Vec<Vec<Cell>>, row_y: usize, cx: i32, label: &str, color: Color| {
                let lx = (cx - label.len() as i32 / 2).max(0) as usize;
                for (j, ch) in label.chars().enumerate() {
                    if lx + j < page_w && row_y < pg.len() {
                        pg[row_y][lx + j] = Cell::new(ch, color);
                    }
                }
            };

        // ── Section 1: Tree Types ─────────────────────────────────
        let mut cy = 1usize;
        write_header(&mut pg, cy, "── TREE TYPES ──", palette[4]);
        cy += header_h;

        let tree_labels = [
            "Spiral",
            "Candelabra",
            "Split",
            "Birch",
            "WavyBirch",
            "Storm",
            "Dead",
            "Drooping",
        ];
        let cols8 = 4usize;
        let col_w8 = page_w / cols8;

        for idx in 0..8usize {
            let row = idx / cols8;
            let col = idx % cols8;
            let row_y = cy + row * (tree_h + label_h);
            let color = palette[idx % palette.len()];
            let cx_i = (col * col_w8 + col_w8 / 2) as i32;
            let params = TreeParams {
                plot: Rect {
                    x: col * col_w8 + 1,
                    y: row_y,
                    w: col_w8 - 2,
                    h: tree_h,
                },
                energy,
                trunk_color: color,
                bark_color: darken(color, 15),
                branch_color: lighten(color, 20),
                tip_color: lighten(color, 40),
                fruit_color: palette[(idx + 2) % palette.len()],
                fruit_factor: 0.2,
                branch_factor: 0.5,
                direction: GrowDir::Up,
                bole: Some(Bole { style: idx % 8 }),
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
            write_label(
                &mut pg,
                row_y + tree_h,
                cx_i,
                tree_labels[idx],
                lighten(color, 40),
            );
        }
        cy += 2 * (tree_h + label_h) + section_gap;

        // ── Section 2: Trunk Algorithms ───────────────────────────
        write_header(&mut pg, cy, "── TRUNK ALGORITHMS ──", palette[4]);
        cy += header_h;

        let trunk_labels = [
            "Straight", "Thick", "Wobble", "Lean", "Gnarled", "Organic", "Sine",
        ];
        let cols7 = 7usize;
        let col_w7 = page_w / cols7;

        for i in 0..7usize {
            let color = palette[i % palette.len()];
            let cx_i = (i * col_w7 + col_w7 / 2) as i32;
            let params = TreeParams {
                plot: Rect {
                    x: i * col_w7 + 1,
                    y: cy,
                    w: col_w7 - 2,
                    h: tree_h,
                },
                energy,
                trunk_color: color,
                bark_color: darken(color, 15),
                branch_color: lighten(color, 20),
                tip_color: lighten(color, 40),
                fruit_color: palette[(i + 2) % palette.len()],
                fruit_factor: 0.2,
                branch_factor: 0.5,
                direction: GrowDir::Up,
                bole: Some(Bole { style: 14 }),
                taper: TaperKind::default(),
            };
            let drawer: Box<dyn TreeDrawer> = match i {
                0 => Box::new(TreeWithTrunk {
                    tree: SpiralTree,
                    trunk: Box::new(StraightTrunk {
                        height_fraction: 0.5,
                    }),
                }),
                1 => Box::new(TreeWithTrunk {
                    tree: SpiralTree,
                    trunk: Box::new(ThickTrunk {
                        height_fraction: 0.5,
                    }),
                }),
                2 => Box::new(TreeWithTrunk {
                    tree: SpiralTree,
                    trunk: Box::new(WobbleTrunk {
                        height_fraction: 0.5,
                    }),
                }),
                3 => Box::new(TreeWithTrunk {
                    tree: SpiralTree,
                    trunk: Box::new(LeanTrunk::new()),
                }),
                4 => Box::new(TreeWithTrunk {
                    tree: SpiralTree,
                    trunk: Box::new(GnarledTrunk),
                }),
                5 => Box::new(TreeWithTrunk {
                    tree: SpiralTree,
                    trunk: Box::new(OrganicTrunk {
                        height_fraction: 0.5,
                    }),
                }),
                _ => Box::new(TreeWithTrunk {
                    tree: SpiralTree,
                    trunk: Box::new(SineTrunk {
                        height_fraction: 0.3,
                        amplitude: 2,
                    }),
                }),
            };
            drawer.grow(&mut pg, &params, &mut rng);
            write_label(
                &mut pg,
                cy + tree_h,
                cx_i,
                trunk_labels[i],
                lighten(color, 40),
            );
        }
        cy += tree_h + label_h + section_gap;

        // ── Section 3: Taper Styles ───────────────────────────────
        write_header(&mut pg, cy, "── TAPER STYLES ──", palette[4]);
        cy += header_h;

        let taper_data = [
            ("Diagonal", TaperKind::Diagonal),
            ("Shelf", TaperKind::Shelf),
            ("Bracket", TaperKind::Bracket),
            ("Step", TaperKind::Step),
            ("Melt", TaperKind::Melt),
        ];
        let cols5 = 5usize;
        let col_w5 = page_w / cols5;

        for (i, (label, taper)) in taper_data.iter().enumerate() {
            let color = palette[i % palette.len()];
            let cx_i = (i * col_w5 + col_w5 / 2) as i32;
            let params = TreeParams {
                plot: Rect {
                    x: i * col_w5 + 1,
                    y: cy,
                    w: col_w5 - 2,
                    h: tree_h,
                },
                energy,
                trunk_color: color,
                bark_color: darken(color, 15),
                branch_color: lighten(color, 20),
                tip_color: lighten(color, 40),
                fruit_color: palette[(i + 2) % palette.len()],
                fruit_factor: 0.2,
                branch_factor: 0.5,
                direction: GrowDir::Up,
                bole: Some(Bole { style: 0 }),
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
                plot: Rect {
                    x: col * bole_col_w + 1,
                    y: row_y,
                    w: bole_col_w - 2,
                    h: bole_tree_h,
                },
                energy,
                trunk_color: color,
                bark_color: darken(color, 15),
                branch_color: lighten(color, 20),
                tip_color: lighten(color, 40),
                fruit_color: palette[(bole_i + 2) % palette.len()],
                fruit_factor: 0.2,
                branch_factor: 0.5,
                direction: GrowDir::Up,
                bole: Some(Bole { style: bole_i }),
                taper: TaperKind::Bracket,
            };
            SpiralTree.grow(&mut pg, &params, &mut rng);
            write_label(
                &mut pg,
                row_y + bole_tree_h,
                cx_i,
                &label,
                lighten(color, 40),
            );
        }

        emit_grid(&pg);
        return (grid, true);
    (grid, false)
}

/// Dispatch arm for mode(s): trees4 (moved verbatim from run()).
pub(crate) fn cli_trees4(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        // trees4: showcase all TreeDrawer types including new ports
        // One tree per slot, labeled, with boles and fruit
        let energy: f32 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(0.8);

        let all_trees: Vec<(&str, Box<dyn TreeDrawer>)> = vec![
            ("Spiral", Box::new(SpiralTree)),
            ("Candelabra", Box::new(CandelabraTree)),
            ("Split", Box::new(SplitTree)),
            ("Birch", Box::new(BirchTree)),
            ("WavyBirch", Box::new(WavyBirch)),
            ("Storm", Box::new(StormTree::new())),
            ("Dead", Box::new(DeadTree)),
            ("Drooping", Box::new(DroopingTree)),
            ("Pine", Box::new(PineTree)),
            ("Willow", Box::new(WillowTree)),
            ("Palm", Box::new(PalmTree)),
            ("Wide", Box::new(WideTree)),
            ("Asymmetric", Box::new(AsymmetricTree)),
            ("Kaiju", Box::new(KaijuTree)),
            ("Zigzag", Box::new(ZigzagTree)),
            ("BrailleCanopy", Box::new(BrailleCanopyTree)),
            ("Tendril", Box::new(TendrilTree)),
        ];

        let count = all_trees.len();
        let cols = 6usize;
        let rows = (count + cols - 1) / cols;
        let cell_w = width / cols;
        let cell_h = 28usize; // tall cells like trees3
        let page_h = rows * cell_h + 2;
        let mut grid = vec![vec![Cell::blank(); width]; page_h];

        for (i, (label, drawer)) in all_trees.iter().enumerate() {
            let col = i % cols;
            let row = i / cols;
            let px = col * cell_w;
            let py = row * cell_h;
            let color = palette[i % palette.len()];

            let params = TreeParams {
                plot: Rect {
                    x: px + 1,
                    y: py + 1,
                    w: cell_w - 2,
                    h: cell_h - 3,
                },
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

        emit_grid(&grid);
        return (grid, true);
    (grid, false)
}

/// Dispatch arm for mode(s): trees8 (moved verbatim from run()).
pub(crate) fn cli_trees8(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        // trees8: [energy] [fruit] [branch]
        // Three new TreeDrawers (Oak, Fountain, Windswept), each shown at
        // full and low energy with cycling boles and tapers.
        let energy: f32 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(0.85);
        let fruit: f32 = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(0.3);
        let branch: f32 = args.get(6).and_then(|s| s.parse().ok()).unwrap_or(0.7);

        let drawers: Vec<(&str, Box<dyn TreeDrawer>)> = vec![
            ("Oak", Box::new(OakTree)),
            ("Fountain", Box::new(FountainTree)),
            ("Windswept", Box::new(WindsweptTree::new(&mut rng))),
        ];
        let tapers = [TaperKind::Bracket, TaperKind::Diagonal, TaperKind::Melt];

        let cols = drawers.len();
        let cell_w = width / cols;
        let cell_h = 24usize;
        let rows = 2usize;
        let page_h = rows * cell_h + 2;
        let mut pg = vec![vec![Cell::blank(); width]; page_h];

        for row in 0..rows {
            // top row full energy, bottom row scrub-sized
            let row_energy = if row == 0 { energy } else { energy * 0.6 };
            for (i, (label, drawer)) in drawers.iter().enumerate() {
                let px = i * cell_w;
                let py = row * cell_h;
                let color = palette[(i + row * 3) % palette.len()];

                let params = TreeParams {
                    plot: Rect {
                        x: px + 1,
                        y: py + 1,
                        w: cell_w - 2,
                        h: cell_h - 3,
                    },
                    energy: row_energy,
                    trunk_color: color,
                    bark_color: darken(color, 15),
                    branch_color: lighten(color, 20),
                    tip_color: lighten(color, 40),
                    fruit_color: palette[(i + 3) % palette.len()],
                    fruit_factor: fruit,
                    branch_factor: branch,
                    direction: GrowDir::Up,
                    bole: Some(Bole { style: i * 2 + row }),
                    taper: tapers[(i + row) % tapers.len()],
                };
                drawer.grow(&mut pg, &params, &mut rng);

                let lx = px + cell_w / 2 - label.len() / 2;
                let ly = py + cell_h - 1;
                for (j, ch) in label.chars().enumerate() {
                    if lx + j < width && ly < page_h {
                        pg[ly][lx + j] = Cell::new(ch, darken(color, 20));
                    }
                }
            }
        }

        emit_grid(&pg);
        return (grid, true);
    (grid, false)
}

/// Dispatch arm for mode(s): trees9 (moved verbatim from run()).
pub(crate) fn cli_trees9(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        // trees9: [energy] [fruit] [branch]
        // Esoteric drawers (Fractal, L-System, Dragon, Helix) at two
        // energies, planted on the winding boles (24-27).
        let energy: f32 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(0.85);
        let fruit: f32 = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(0.25);
        let branch: f32 = args.get(6).and_then(|s| s.parse().ok()).unwrap_or(0.7);

        let drawers: Vec<(&str, Box<dyn TreeDrawer>)> = vec![
            ("Fractal", Box::new(FractalTree)),
            ("L-System", Box::new(LSystemTree)),
            ("Dragon", Box::new(DragonTree)),
            ("Helix", Box::new(HelixTree)),
        ];
        let tapers = [TaperKind::Diagonal, TaperKind::Bracket, TaperKind::Shelf];

        let cols = drawers.len();
        let cell_w = width / cols;
        let cell_h = 24usize;
        let rows = 2usize;
        let page_h = rows * cell_h + 2;
        let mut pg = vec![vec![Cell::blank(); width]; page_h];

        for row in 0..rows {
            // top row full energy, bottom row scrub-sized
            let row_energy = if row == 0 { energy } else { energy * 0.6 };
            for (i, (label, drawer)) in drawers.iter().enumerate() {
                let px = i * cell_w;
                let py = row * cell_h;
                let color = palette[(i + row * 3) % palette.len()];

                let params = TreeParams {
                    plot: Rect {
                        x: px + 1,
                        y: py + 1,
                        w: cell_w - 2,
                        h: cell_h - 5,
                    },
                    energy: row_energy,
                    trunk_color: color,
                    bark_color: darken(color, 15),
                    branch_color: lighten(color, 20),
                    tip_color: lighten(color, 40),
                    fruit_color: palette[(i + 3) % palette.len()],
                    fruit_factor: fruit,
                    branch_factor: branch,
                    direction: GrowDir::Up,
                    bole: Some(Bole {
                        style: 24 + (i + row) % 4,
                    }),
                    taper: tapers[(i + row) % tapers.len()],
                };
                drawer.grow(&mut pg, &params, &mut rng);

                let lx = px + cell_w / 2 - label.len() / 2;
                let ly = py + cell_h - 1;
                for (j, ch) in label.chars().enumerate() {
                    if lx + j < width && ly < page_h {
                        pg[ly][lx + j] = Cell::new(ch, darken(color, 20));
                    }
                }
            }
        }

        emit_grid(&pg);
        return (grid, true);
    (grid, false)
}

/// Dispatch arm for mode(s): bushes (moved verbatim from run()).
pub(crate) fn cli_bushes(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
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
                x: cx,
                y: cy,
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
    (grid, false)
}

/// Dispatch arm for mode(s): boles6 (moved verbatim from run()).
pub(crate) fn cli_boles6(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        // boles6 [layers=0] -- close-packed bole forest, every trunk rooted in a bole
        let layers_arg: u8 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(0);
        let layer_count = if layers_arg == 0 {
            (3 + seed % 2) as u8
        } else {
            layers_arg.clamp(2, 5)
        };
        let strong: &'static [usize] = &[0, 1, 3, 8, 6, 17, 22, 23];
        let opts = PackOpts {
            layer_count,
            overlap: 0.18,
            bole_rate: 1.0,
            ground_frac: 0.4,
            kind_filter: Some(strong),
            ..Default::default()
        };
        let (ground_y, slots) = pack_forest(width, height, &mut rng, &opts);

        // faint sky
        let sky_color = darken(palette[0], 85);
        for y in 0..ground_y {
            for x in 0..width {
                if rng.random_range(0..22u32) == 0 {
                    grid[y][x] = Cell::new('·', sky_color);
                }
            }
        }
        // dim ground
        for x in 0..width {
            for y in ground_y..height {
                let depth = y - ground_y;
                grid[y][x] =
                    Cell::new('·', hsl_to_rgb(30.0, 0.25, (0.18 - depth as f64 * 0.004).max(0.08)));
            }
        }

        let lf_denom = (layer_count - 1).max(1) as f64;
        for s in &slots {
            let lfrac = s.layer as f64 / lf_denom;
            let color = hsl_to_rgb(s.hue, 0.40 + lfrac * 0.20, 0.16 + lfrac * 0.16);
            let tp = TreeParams {
                plot: s.plot,
                energy: s.energy,
                trunk_color: lighten(color, 8),
                bark_color: darken(color, 12),
                branch_color: color,
                tip_color: lighten(color, 26),
                fruit_color: shift_hue(color, 50.0),
                fruit_factor: 0.1,
                branch_factor: 0.7,
                direction: GrowDir::Up,
                bole: s.bole,
                taper: s.taper,
            };
            grow_tree_by_index(s.kind, &mut grid, &tp, &mut rng);
        }
    (grid, false)
}

/// Dispatch arm for mode(s): trees10 (moved verbatim from run()).
pub(crate) fn cli_trees10(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        // trees10 [count=0] -- specimen row, every archetype side by side
        let count_arg: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(0);
        let count = if count_arg == 0 {
            (width / 7).max(8)
        } else {
            count_arg.clamp(4, 48)
        };
        let ground_y = (height as f32 * 0.82) as usize;
        for x in 0..width {
            for y in ground_y..height {
                grid[y][x] = Cell::new('·', darken(palette[1], 60));
            }
        }
        let slot = (width / count).max(6);
        let tapers = [
            TaperKind::Diagonal,
            TaperKind::Shelf,
            TaperKind::Bracket,
            TaperKind::Step,
            TaperKind::Melt,
        ];
        for i in 0..count {
            let kind = i % TREE_KIND_COUNT;
            let cx = i * slot + slot / 2;
            let plot_w = slot;
            let canopy_top = 2usize;
            let plot = Rect {
                x: cx.saturating_sub(plot_w / 2),
                y: canopy_top,
                w: plot_w,
                h: ground_y.saturating_sub(canopy_top) + 1,
            };
            let hue = (i as f64 * (360.0 / count as f64)) % 360.0;
            let color = hsl_to_rgb(hue, 0.55, 0.32);
            let tp = TreeParams {
                plot,
                energy: 0.92,
                trunk_color: color,
                bark_color: darken(color, 16),
                branch_color: color,
                tip_color: lighten(color, 30),
                fruit_color: shift_hue(color, 60.0),
                fruit_factor: 0.3,
                branch_factor: 0.85,
                direction: GrowDir::Up,
                bole: if i % 2 == 0 {
                    Some(Bole { style: i % 10 })
                } else {
                    None
                },
                taper: tapers[i % tapers.len()],
            };
            grow_tree_by_index(kind, &mut grid, &tp, &mut rng);
        }
    (grid, false)
}
