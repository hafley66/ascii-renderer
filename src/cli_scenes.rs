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
use crate::illuminarium::*;
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

/// Dispatch arm for the animated illuminarium mode.
pub(crate) fn cli_illuminarium(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, _term_w: u16, _term_h: u16, args: &[String], _mode: &str, _theme_name: &str) -> (Grid, bool) {
    let params = IlluminariumParams::from_args(args);
    draw_illuminarium(
        &mut grid,
        width,
        height,
        seed,
        &palette,
        &mut rng,
        t_anim,
        &params,
    );
    (grid, false)
}


/// Dispatch arm for mode(s): party (moved verbatim from run()).
pub(crate) fn cli_party(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        // party [gap] [nodes] [scale] [detail] [weather] [path] [atmo]
        let pp = PartyParams {
            gap: args.get(4).and_then(|s| s.parse().ok()).unwrap_or(0),
            nodes: args.get(5).and_then(|s| s.parse().ok()).unwrap_or(0),
            scale: args.get(6).and_then(|s| s.parse().ok()).unwrap_or(50),
            detail: args.get(7).and_then(|s| s.parse().ok()).unwrap_or(50),
        };
        let weather = args
            .get(8)
            .and_then(|s| Weather::from_name(s))
            .unwrap_or_else(|| Weather::pick(&mut rng));
        let path_style = args
            .get(9)
            .and_then(|s| PathStyle::from_name(s))
            .unwrap_or_else(|| PathStyle::pick(&mut rng));
        let atmo_intensity: u32 = args.get(10).and_then(|s| s.parse().ok()).unwrap_or(50);
        let rect = Rect {
            x: 0,
            y: 0,
            w: width,
            h: height,
        };
        let (layers, stops, boxes) = party_walk(width, height, &palette, &pp, &mut rng);
        let scene = Scene { layers };
        render_scene(&mut grid, &rect, &scene, &mut rng);
        // Draw connecting path between node centers
        draw_styled_path(
            &mut grid,
            &stops,
            path_style,
            darken(palette[2], 30),
            &mut rng,
        );
        // Draw box borders around each node
        let border_color = palette[4];
        for &(bx, by, bw, bh) in &boxes {
            draw_box_border(&mut grid, bx, by, bw, bh, border_color);
        }
        // Weather overlay
        apply_atmosphere(&mut grid, weather, atmo_intensity, &palette, &mut rng);
    (grid, false)
}

/// Dispatch arm for mode(s): soup (moved verbatim from run()).
pub(crate) fn cli_soup(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        let rect = Rect {
            x: 0,
            y: 0,
            w: width,
            h: height,
        };
        let (layers, stops) = soup_walk(width, height, &palette, &mut rng);
        let scene = Scene { layers };
        render_scene(&mut grid, &rect, &scene, &mut rng);
        draw_path_trail(&mut grid, &stops, palette[2], &mut rng);
    (grid, false)
}

/// Dispatch arm for mode(s): stem (moved verbatim from run()).
pub(crate) fn cli_stem(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        let rect = Rect {
            x: 0,
            y: 0,
            w: width,
            h: height,
        };
        let (layers, spine) = path_walk_stem(width, height, &palette, &mut rng);
        let scene = Scene { layers };
        render_scene(&mut grid, &rect, &scene, &mut rng);
        draw_stalk(&mut grid, &spine, palette[2]);
    (grid, false)
}

/// Dispatch arm for mode(s): scene-walk (moved verbatim from run()).
pub(crate) fn cli_scene_walk(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        let rect = Rect {
            x: 0,
            y: 0,
            w: width,
            h: height,
        };
        let layers = path_walk_layers(width, height, &palette, &mut rng);
        let scene = Scene { layers };
        render_scene(&mut grid, &rect, &scene, &mut rng);
    (grid, false)
}

/// Dispatch arm for mode(s): scene-walk-2 (moved verbatim from run()).
pub(crate) fn cli_scene_walk_2(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        let rect = Rect {
            x: 0,
            y: 0,
            w: width,
            h: height,
        };
        let (layers, stops) = path_walk_layers_2(width, height, &palette, &mut rng);
        let scene = Scene { layers };
        render_scene(&mut grid, &rect, &scene, &mut rng);
        draw_path_trail(&mut grid, &stops, palette[2], &mut rng);
    (grid, false)
}

/// Dispatch arm for mode(s): scene-walk-3 (moved verbatim from run()).
pub(crate) fn cli_scene_walk_3(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        let rect = Rect {
            x: 0,
            y: 0,
            w: width,
            h: height,
        };
        let density = 50u32;
        let (layers, stops, _boxes) =
            path_walk_layers_3(width, height, &palette, density, &mut rng);
        let scene = Scene { layers };
        render_scene(&mut grid, &rect, &scene, &mut rng);
        draw_path_trail(&mut grid, &stops, palette[2], &mut rng);
    (grid, false)
}

/// Dispatch arm for mode(s): kintsugi (moved verbatim from run()).
pub(crate) fn cli_kintsugi(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        // kintsugi [cracks] -- shattered tile shards repaired with gold seams
        let crack_count: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(4);
        let crack_count = crack_count.clamp(1, 12);

        // Each crack is a top-to-bottom polyline: one x per row, drifting with momentum.
        let mut cracks: Vec<Vec<i32>> = Vec::new();
        for i in 0..crack_count {
            let band = (width / (crack_count + 1)).max(2) as i32;
            let mut x = (i as i32 + 1) * band + rng.random_range(-band / 3..=band / 3);
            let mut drift: i32 = 0;
            let mut path = Vec::with_capacity(height);
            for _ in 0..height {
                path.push(x);
                if rng.random::<f32>() < 0.4 {
                    drift = rng.random_range(-1..=1);
                }
                x = (x + drift).clamp(1, width as i32 - 2);
            }
            cracks.push(path);
        }

        // Region id per cell = number of cracks left of it. Each shard gets its
        // own tile pattern and shade so the pieces read as separate pottery.
        let mut shard_tiles: Vec<TilePattern> = Vec::new();
        let mut shard_shade: Vec<u8> = Vec::new();
        for _ in 0..=crack_count {
            let v = tile_variant_from_index(rng.random_range(0..TILE_VARIANT_COUNT));
            shard_tiles.push(make_tile(v));
            shard_shade.push(rng.random_range(40..90));
        }
        for y in 0..height {
            for x in 0..width {
                let region = cracks.iter().filter(|c| c[y] < x as i32).count();
                let (ch, ci) = shard_tiles[region].at(x, y);
                if ch == ' ' {
                    continue;
                }
                let base = if ci == 0 { palette[1] } else { palette[2] };
                grid[y][x] = Cell::new(ch, darken(base, shard_shade[region]));
            }
        }

        // Gold seams over the top: slope-matched glyphs plus hairline branches.
        let gold = lighten(palette[3], 25);
        for path in &cracks {
            for y in 0..height {
                let x = path[y];
                let next = if y + 1 < height { path[y + 1] } else { x };
                let ch = if next > x {
                    '╲'
                } else if next < x {
                    '╱'
                } else {
                    '│'
                };
                if x >= 0 && (x as usize) < width {
                    grid[y][x as usize] = Cell::new(ch, gold);
                }
                if rng.random::<f32>() < 0.08 {
                    let dir: i32 = if rng.random::<f32>() < 0.5 { -1 } else { 1 };
                    let len = rng.random_range(2..5i32);
                    for k in 1..=len {
                        let bx = x + dir * k;
                        let by = y + k as usize;
                        if bx >= 0 && (bx as usize) < width && by < height {
                            let bch = if dir > 0 { '╲' } else { '╱' };
                            grid[by][bx as usize] = Cell::new(bch, darken(gold, 20));
                        }
                    }
                }
            }
        }
    (grid, false)
}

/// Dispatch arm for mode(s): constellation (moved verbatim from run()).
pub(crate) fn cli_constellation(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        // constellation [count] -- night sky with named, line-connected clusters
        let count: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(4);
        let count = count.clamp(1, 8);

        let field = Rect {
            x: 0,
            y: 0,
            w: width,
            h: height,
        };
        fill_noise(
            &mut grid,
            &field,
            NoiseVariant::Dot,
            darken(palette[2], 90),
            darken(palette[2], 70),
            &mut rng,
        );

        let syllables = [
            "vel", "ara", "cyg", "lyr", "tau", "rho", "nix", "ori", "eka", "sol",
        ];

        for _ in 0..count {
            let pad_x = (width / 8).max(1);
            let pad_y = (height / 6).max(1);
            let cx = rng.random_range(pad_x..(width - pad_x).max(pad_x + 1)) as i32;
            let cy = rng.random_range(pad_y..(height - pad_y).max(pad_y + 1)) as i32;
            let star_n = rng.random_range(4..8);
            let rx = rng.random_range(6..14i32);
            let ry = rng.random_range(2..5i32);

            let mut stars: Vec<(i32, i32)> = Vec::new();
            for _ in 0..star_n {
                let sx = (cx + rng.random_range(-rx..=rx)).clamp(0, width as i32 - 1);
                let sy = (cy + rng.random_range(-ry..=ry)).clamp(0, height as i32 - 2);
                if !stars.contains(&(sx, sy)) {
                    stars.push((sx, sy));
                }
            }
            // chain left-to-right so the figure doesn't crisscross itself
            stars.sort();

            let line_color = darken(palette[1], 40);
            for w in stars.windows(2) {
                let (x0, y0) = w[0];
                let (x1, y1) = w[1];
                let dx = x1 - x0;
                let dy = y1 - y0;
                let steps = dx.abs().max(dy.abs());
                for s in 1..steps {
                    let t = s as f32 / steps as f32;
                    let lx = (x0 as f32 + dx as f32 * t).round() as i32;
                    let ly = (y0 as f32 + dy as f32 * t).round() as i32;
                    if lx < 0 || ly < 0 || lx >= width as i32 || ly >= height as i32 {
                        continue;
                    }
                    let ch = if dy == 0 {
                        '─'
                    } else if dx == 0 {
                        '│'
                    } else if (dx > 0) == (dy > 0) {
                        '╲'
                    } else {
                        '╱'
                    };
                    grid[ly as usize][lx as usize] = Cell::new(ch, line_color);
                }
            }

            let star_chars = ['✦', '✧', '*', '◆'];
            for (si, &(sx, sy)) in stars.iter().enumerate() {
                let ch = star_chars[si % star_chars.len()];
                let c = if si == 0 {
                    lighten(palette[4], 20)
                } else {
                    palette[3]
                };
                grid[sy as usize][sx as usize] = Cell::new(ch, c);
            }

            let name = format!(
                "{}{}",
                syllables[rng.random_range(0..syllables.len())],
                syllables[rng.random_range(0..syllables.len())]
            );
            let ly = ((cy + ry + 1) as usize).min(height - 1);
            let lx = (cx as usize).saturating_sub(name.len() / 2);
            for (j, ch) in name.chars().enumerate() {
                if lx + j < width {
                    grid[ly][lx + j] = Cell::new(ch, darken(palette[4], 30));
                }
            }
        }
    (grid, false)
}

/// Dispatch arm for mode(s): strata (moved verbatim from run()).
pub(crate) fn cli_strata(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        // strata [layers] -- geological cross-section with fossils
        let layer_count: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(6);
        let layer_count = layer_count.clamp(2, 10);

        // stacked contour boundaries, forced monotonic per column
        let band_h = (height / (layer_count + 1)).max(2);
        let mut bounds: Vec<Vec<usize>> = Vec::new();
        for i in 0..layer_count {
            let base = band_h * (i + 1);
            let amp = (band_h / 2).max(1);
            let mut c = gen_contour(width, base, amp, 0.55, &mut rng);
            if let Some(prev) = bounds.last() {
                for x in 0..width {
                    if c[x] <= prev[x] {
                        c[x] = prev[x] + 1;
                    }
                }
            }
            for x in 0..width {
                c[x] = c[x].min(height - 1);
            }
            bounds.push(c);
        }

        // fill each band with its own sediment texture, darker with depth
        let glyph_pools: [&[char]; 6] = [
            &['·', '∙', ' ', ' ', ' '],
            &['─', '─', '·', ' ', ' '],
            &['╱', '╲', ' ', ' '],
            &['░', '░', '·', ' ', ' '],
            &['~', '─', ' ', ' '],
            &['▪', '·', ' ', ' ', ' ', ' '],
        ];
        for li in 0..layer_count {
            let pool = glyph_pools[rng.random_range(0..glyph_pools.len())];
            let shade = (li * 90 / layer_count) as u8;
            let c1 = darken(palette[1 + li % 3], shade);
            for x in 0..width {
                let top = bounds[li][x] + 1;
                let bot = if li + 1 < layer_count {
                    bounds[li + 1][x]
                } else {
                    height
                };
                for y in top..bot.min(height) {
                    let ch = pool[rng.random_range(0..pool.len())];
                    if ch == ' ' {
                        continue;
                    }
                    grid[y][x] = Cell::new(ch, c1);
                }
            }
        }

        // boundary ridges on top of the fills
        let full = Rect {
            x: 0,
            y: 0,
            w: width,
            h: height,
        };
        for (li, c) in bounds.iter().enumerate() {
            let ridge = darken(lighten(palette[2], 20), (li * 60 / layer_count) as u8);
            draw_contour_ridge(&mut grid, &full, c, ridge);
        }

        // fossils embedded in the deeper bands
        let fossil_count = rng.random_range(2..5);
        for _ in 0..fossil_count {
            let fx = rng.random_range(4..width.saturating_sub(4).max(5));
            let floor = bounds[(layer_count / 2).min(layer_count - 1)][fx];
            if floor + 3 >= height {
                continue;
            }
            let fy = rng.random_range(floor + 2..height.saturating_sub(1).max(floor + 3));
            if rng.random::<f32>() < 0.5 {
                draw_fruit(
                    &mut grid,
                    fx,
                    fy,
                    rng.random_range(0..5),
                    lighten(palette[3], 10),
                );
            } else {
                draw_mask(
                    &mut grid,
                    fx,
                    fy,
                    2,
                    rng.random_range(0..MASK_STYLE_COUNT),
                    lighten(palette[3], 10),
                );
            }
        }
    (grid, false)
}

/// Dispatch arm for mode(s): patchwalk (moved verbatim from run()).
pub(crate) fn cli_patchwalk(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        // patchwalk [stops] [line_w] -- quilt x scene-walk x mondrian2:
        // skewed BSP with big flat fields against small quilted clusters,
        // a heavy thread route stitched between clearings.
        let stop_count: usize = args
            .get(4)
            .and_then(|s| s.parse().ok())
            .unwrap_or(4)
            .clamp(2, 8);
        let line_w: usize = args
            .get(5)
            .and_then(|s| s.parse().ok())
            .unwrap_or(2)
            .clamp(1, 3);

        // 1. Binding everywhere; leaves get carved out of it
        let line_color = darken(palette[0], 60);
        for y in 0..height {
            for x in 0..width {
                grid[y][x] = Cell::with_bg(' ', line_color, line_color);
            }
        }

        // 2. Skewed BSP: splits land at 0.22-0.38, one branch can stop
        // early so big fields sit next to small clusters
        let mut leaves: Vec<Rect> = Vec::new();
        let mut stack: Vec<(Rect, usize)> = vec![(
            Rect {
                x: line_w,
                y: 1,
                w: width - line_w * 2,
                h: height - 2,
            },
            0,
        )];
        while let Some((r, d)) = stack.pop() {
            let can_v = r.w >= 15 + line_w;
            let can_h = r.h >= 8;
            let stop_p = match d {
                0 => 0.0,
                1 => 0.08,
                2 => 0.3,
                _ => 0.55,
            };
            if (!can_v && !can_h) || d >= 5 || rng.random::<f32>() < stop_p {
                leaves.push(r);
                continue;
            }
            let vert = if !can_h {
                true
            } else if !can_v {
                false
            } else if r.w > r.h * 3 {
                true
            } else if r.h * 2 > r.w {
                false
            } else {
                rng.random_range(0..2u32) == 0
            };
            let mut t = 0.22 + rng.random::<f32>() * 0.16;
            if rng.random_range(0..2u32) == 0 {
                t = 1.0 - t;
            }
            if vert {
                let sw = ((r.w as f32 * t) as usize).clamp(6, r.w - 6 - line_w);
                stack.push((
                    Rect {
                        x: r.x,
                        y: r.y,
                        w: sw,
                        h: r.h,
                    },
                    d + 1,
                ));
                stack.push((
                    Rect {
                        x: r.x + sw + line_w,
                        y: r.y,
                        w: r.w - sw - line_w,
                        h: r.h,
                    },
                    d + 1,
                ));
            } else {
                let sh = ((r.h as f32 * t) as usize).clamp(3, r.h - 4);
                stack.push((
                    Rect {
                        x: r.x,
                        y: r.y,
                        w: r.w,
                        h: sh,
                    },
                    d + 1,
                ));
                stack.push((
                    Rect {
                        x: r.x,
                        y: r.y + sh + 1,
                        w: r.w,
                        h: r.h - sh - 1,
                    },
                    d + 1,
                ));
            }
        }

        // 3. Treatments: big leaves lean flat color (mondrian fields),
        // small leaves lean quilted; white-weighted like the source
        let canvas = lighten(palette[0], 45);
        let field_colors = [canvas, canvas, canvas, palette[1], palette[2], palette[3]];
        let thread = darken(palette[4], 30);
        for r in &leaves {
            let area = r.w * r.h;
            let flat_p = if area > 400 {
                0.92
            } else if area > 300 {
                0.75
            } else if area > 120 {
                0.5
            } else {
                0.3
            };
            if rng.random::<f32>() < flat_p {
                let bg = field_colors[rng.random_range(0..field_colors.len())];
                for y in r.y..(r.y + r.h).min(height) {
                    for x in r.x..(r.x + r.w).min(width) {
                        grid[y][x] = Cell::with_bg(' ', bg, bg);
                    }
                }
            } else {
                let variant = tile_variant_from_index(rng.random_range(0..TILE_VARIANT_COUNT));
                let c1 = darken(palette[1 + rng.random_range(0..3)], rng.random_range(0..50));
                let c2 = darken(
                    palette[1 + rng.random_range(0..3)],
                    rng.random_range(20..70),
                );
                fill_tile_pure(&mut grid, r, variant, c1, c2);
                // running stitch just inside the patch edge
                let (x0, y0) = (r.x, r.y);
                let (x1, y1) = (r.x + r.w - 1, r.y + r.h - 1);
                for x in x0..=x1 {
                    if x % 2 == 0 && y0 < height && x < width {
                        grid[y0][x] = Cell::new('┈', thread);
                    }
                    if x % 2 == 0 && y1 < height && x < width {
                        grid[y1][x] = Cell::new('┈', thread);
                    }
                }
                for y in y0..=y1 {
                    if y % 2 == 0 && y < height && x0 < width {
                        grid[y][x0] = Cell::new('┊', thread);
                    }
                    if y % 2 == 0 && y < height && x1 < width {
                        grid[y][x1] = Cell::new('┊', thread);
                    }
                }
            }
        }

        // 4. Stops: centers of randomly chosen roomy leaves, walked
        // left to right
        let mut cands: Vec<(usize, usize)> = leaves
            .iter()
            .filter(|r| r.w >= 10 && r.h >= 5)
            .map(|r| (r.x + r.w / 2, r.y + r.h / 2))
            .collect();
        let mut stops: Vec<(usize, usize)> = Vec::new();
        while stops.len() < stop_count && !cands.is_empty() {
            let i = rng.random_range(0..cands.len() as u32) as usize;
            stops.push(cands.remove(i));
        }
        stops.sort_by_key(|s| s.0);

        // 5. Thread route: heavy box-drawing polyline, orthogonal runs
        // with elbows alternating horizontal-first / vertical-first
        let mut pts: Vec<(i32, i32)> = Vec::new();
        for (i, &(sx, sy)) in stops.iter().enumerate() {
            let p = (sx as i32, sy as i32);
            if i > 0 {
                let last = *pts.last().unwrap();
                let elbow = if i % 2 == 1 {
                    (p.0, last.1)
                } else {
                    (last.0, p.1)
                };
                if elbow != last && elbow != p {
                    pts.push(elbow);
                }
            }
            pts.push(p);
        }
        let path_color = lighten(palette[4], 25);
        let dir_of = |a: (i32, i32), b: (i32, i32)| -> (i32, i32) {
            ((b.0 - a.0).signum(), (b.1 - a.1).signum())
        };
        for seg in pts.windows(2) {
            let (a, b) = (seg[0], seg[1]);
            let d = dir_of(a, b);
            if d == (0, 0) {
                continue;
            }
            let ch = if d.0 != 0 { '━' } else { '┃' };
            let mut p = a;
            loop {
                p = (p.0 + d.0, p.1 + d.1);
                if p == b {
                    break;
                }
                if p.0 >= 0 && p.1 >= 0 && (p.0 as usize) < width && (p.1 as usize) < height {
                    grid[p.1 as usize][p.0 as usize] = Cell::new(ch, path_color);
                }
            }
        }
        for i in 1..pts.len().saturating_sub(1) {
            let din = dir_of(pts[i - 1], pts[i]);
            let dout = dir_of(pts[i], pts[i + 1]);
            if din == (0, 0) || dout == (0, 0) {
                continue;
            }
            let ch = match (din, dout) {
                ((1, 0), (0, 1)) | ((0, -1), (-1, 0)) => '┓',
                ((1, 0), (0, -1)) | ((0, 1), (-1, 0)) => '┛',
                ((-1, 0), (0, 1)) | ((0, -1), (1, 0)) => '┏',
                ((-1, 0), (0, -1)) | ((0, 1), (1, 0)) => '┗',
                _ if din == dout => {
                    if din.0 != 0 {
                        '━'
                    } else {
                        '┃'
                    }
                }
                _ => '╋',
            };
            let (vx, vy) = pts[i];
            if vx >= 0 && vy >= 0 && (vx as usize) < width && (vy as usize) < height {
                grid[vy as usize][vx as usize] = Cell::new(ch, path_color);
            }
        }

        // 6. Clearings at each stop: punch through, applique inside
        for (si, &(sx, sy)) in stops.iter().enumerate() {
            let rx = rng.random_range(3..7u32) as i32;
            let ry = rng.random_range(2..4u32) as i32;
            for y in (sy as i32 - ry)..=(sy as i32 + ry) {
                for x in (sx as i32 - rx)..=(sx as i32 + rx) {
                    if x < 0 || y < 0 || x as usize >= width || y as usize >= height {
                        continue;
                    }
                    let nx = (x - sx as i32) as f32 / rx as f32;
                    let ny = (y - sy as i32) as f32 / ry as f32;
                    if nx * nx + ny * ny <= 1.0 {
                        grid[y as usize][x as usize] = Cell::blank();
                    }
                }
            }
            match rng.random_range(0..4u32) {
                0 => {
                    draw_flower(&mut grid, sx, sy, rng.random_range(0..5), palette[3]);
                }
                1 => {
                    draw_fruit(&mut grid, sx, sy, rng.random_range(0..5), palette[2]);
                }
                2 => {
                    let canopy = sy.saturating_sub(ry as usize);
                    draw_tree(
                        &mut grid,
                        sx,
                        sy + ry as usize - 1,
                        canopy,
                        (rx as usize / 2).max(2),
                        rng.random_range(0..12),
                        palette[1],
                        &mut rng,
                    );
                }
                _ => {
                    draw_flower(
                        &mut grid,
                        sx,
                        sy,
                        rng.random_range(0..5),
                        palette[rng.random_range(1..4)],
                    );
                }
            }
            let label = format!("{}", si + 1);
            let ly = sy + ry as usize + 1;
            if ly < height && sx < width {
                grid[ly][sx] = Cell::new(label.chars().next().unwrap(), thread);
            }
        }
    (grid, false)
}

/// Dispatch arm for mode(s): aurora (moved verbatim from run()).
pub(crate) fn cli_aurora(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        // aurora [bands] -- layered night-sky ribbons over a snowy horizon
        let band_count: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(5);
        let band_count = band_count.clamp(1, 10);

        let sky = Rect {
            x: 0,
            y: 0,
            w: width,
            h: height,
        };
        fill_noise(
            &mut grid,
            &sky,
            NoiseVariant::Dot,
            darken(palette[2], 95),
            darken(palette[3], 80),
            &mut rng,
        );

        let horizon = (height * 3 / 4).max(1).min(height.saturating_sub(2));
        let star_chars = ['·', '∙', '°', '*'];
        let star_count = (width * horizon / 28).max(6);
        for _ in 0..star_count {
            let x = rng.random_range(0..width);
            let y = rng.random_range(0..horizon.max(1));
            let ch = star_chars[rng.random_range(0..star_chars.len())];
            grid[y][x] = Cell::new(ch, darken(lighten(palette[4], 10), rng.random_range(0..45)));
        }

        for b in 0..band_count {
            let color = shift_hue(lighten(palette[3], 20), b as f64 * 28.0);
            let glow = shift_hue(palette[1], b as f64 * 33.0);
            let base =
                height / 7 + (b + 1) * horizon.saturating_sub(height / 6).max(1) / (band_count + 2);
            let amp = rng.random_range(2..(height / 6).max(4) as u32) as f32;
            let thick = rng.random_range(2..5i32);
            let freq1 = rng.random_range(8..18u32) as f32;
            let freq2 = rng.random_range(18..35u32) as f32;
            let phase = rng.random::<f32>() * std::f32::consts::TAU;

            for x in 0..width {
                let xf = x as f32;
                let y_mid = base as f32
                    + (xf / freq1 + phase).sin() * amp
                    + (xf / freq2 + phase * 0.7).sin() * amp * 0.55;
                for dy in -thick..=thick {
                    let y = y_mid.round() as i32 + dy;
                    if y < 0 || y as usize >= horizon {
                        continue;
                    }
                    let falloff = 1.0 - (dy.abs() as f32 / (thick + 1) as f32);
                    if rng.random::<f32>() > 0.45 + falloff * 0.5 {
                        continue;
                    }
                    let ch = match dy.abs() {
                        0 => '═',
                        1 => '─',
                        2 => '~',
                        _ => '·',
                    };
                    let c = if dy == 0 {
                        color
                    } else {
                        darken(glow, (dy.abs() * 12) as u8)
                    };
                    grid[y as usize][x] = Cell::new(ch, c);
                }
            }
        }

        let snow_phase = rng.random::<f32>() * std::f32::consts::TAU;
        for x in 0..width {
            let crest = horizon as i32 + ((x as f32 / 9.0 + snow_phase).sin() * 2.0).round() as i32;
            for y in crest.max(0) as usize..height {
                let depth = y.saturating_sub(horizon);
                let ch = match (x + y * 3) % 9 {
                    0 | 1 => '·',
                    2 => '∿',
                    3 => '╱',
                    4 => '╲',
                    _ => ' ',
                };
                if ch != ' ' {
                    grid[y][x] = Cell::new(
                        ch,
                        darken(lighten(palette[1], 45), (depth * 5).min(90) as u8),
                    );
                }
            }
            if crest >= 0 && (crest as usize) < height {
                let ch = if x % 2 == 0 { '╱' } else { '╲' };
                grid[crest as usize][x] = Cell::new(ch, lighten(palette[4], 5));
            }
        }
    (grid, false)
}

/// Dispatch arm for mode(s): aura2 (moved verbatim from run()).
pub(crate) fn cli_aura2(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        // aura2 [rain] -- sparse rain behind aurora ribbons and snowfields
        let rain: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(34);
        let rain = rain.clamp(0, 120);

        let sky = Rect {
            x: 0,
            y: 0,
            w: width,
            h: height,
        };
        fill_noise(
            &mut grid,
            &sky,
            NoiseVariant::Dot,
            darken(palette[2], 97),
            darken(palette[1], 92),
            &mut rng,
        );

        let horizon = (height * 3 / 4).max(1).min(height.saturating_sub(2));
        let wind = if seed % 2 == 0 { 1i32 } else { -1i32 };
        let drops = width * horizon * rain / 420;
        for _ in 0..drops {
            let len = rng.random_range(1..4i32);
            let x0 = rng.random_range(0..width) as i32;
            let y0 = rng.random_range(0..horizon.max(1)) as i32;
            for step in 0..len {
                let x = x0 + wind * step / 2;
                let y = y0 + step;
                if x < 0 || y < 0 || x as usize >= width || y as usize >= horizon {
                    continue;
                }
                let ch = if wind > 0 { '╲' } else { '╱' };
                grid[y as usize][x as usize] = Cell::new(ch, darken(palette[4], 78));
            }
        }

        let star_chars = ['·', '∙', '°'];
        for _ in 0..(width * horizon / 42).max(4) {
            let x = rng.random_range(0..width);
            let y = rng.random_range(0..horizon.max(1));
            let ch = star_chars[rng.random_range(0..star_chars.len())];
            if grid[y][x].ch == ' ' || rng.random_range(0..3u32) == 0 {
                grid[y][x] =
                    Cell::new(ch, darken(lighten(palette[4], 8), rng.random_range(35..75)));
            }
        }

        let band_count = rng.random_range(4..7usize);
        for b in 0..band_count {
            let color = shift_hue(lighten(palette[3], 26), b as f64 * 31.0);
            let glow = shift_hue(palette[1], b as f64 * 41.0);
            let base =
                height / 8 + (b + 1) * horizon.saturating_sub(height / 7).max(1) / (band_count + 2);
            let amp = rng.random_range(2..(height / 5).max(5) as u32) as f32;
            let thick = if b % 2 == 0 { 3i32 } else { 2i32 };
            let freq1 = rng.random_range(9..22u32) as f32;
            let freq2 = rng.random_range(20..44u32) as f32;
            let phase = rng.random::<f32>() * std::f32::consts::TAU;

            for x in 0..width {
                let xf = x as f32;
                let y_mid = base as f32
                    + (xf / freq1 + phase).sin() * amp
                    + (xf / freq2 + phase * 0.5).sin() * amp * 0.45;
                for dy in -thick..=thick {
                    let y = y_mid.round() as i32 + dy;
                    if y < 0 || y as usize >= horizon {
                        continue;
                    }
                    let falloff = 1.0 - (dy.abs() as f32 / (thick + 1) as f32);
                    if rng.random::<f32>() > 0.38 + falloff * 0.57 {
                        continue;
                    }
                    let ch = match dy.abs() {
                        0 => '═',
                        1 => '─',
                        2 => '~',
                        _ => '·',
                    };
                    let c = if dy == 0 {
                        color
                    } else {
                        darken(glow, (dy.abs() * 16) as u8)
                    };
                    grid[y as usize][x] = Cell::new(ch, c);
                }
            }
        }

        let ridge = gen_contour(width, horizon, (height / 12).max(2), 0.55, &mut rng);
        for x in 0..width {
            let crest = ridge[x].min(height - 1);
            for y in crest..height {
                let depth = y.saturating_sub(crest);
                let ch = match (x * 2 + y * 3) % 11 {
                    0 | 1 => '·',
                    2 => '∿',
                    3 => '╱',
                    4 => '╲',
                    _ => ' ',
                };
                if ch != ' ' {
                    grid[y][x] = Cell::new(
                        ch,
                        darken(lighten(palette[1], 48), (depth * 4).min(80) as u8),
                    );
                }
            }
        }
        let full = Rect {
            x: 0,
            y: 0,
            w: width,
            h: height,
        };
        draw_contour_ridge(&mut grid, &full, &ridge, lighten(palette[4], 5));
    (grid, false)
}

/// Dispatch arm for mode(s): harbor (moved verbatim from run()).
pub(crate) fn cli_harbor(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        // harbor [boats] -- ridiculous neon harbor carnival with cranes and fireworks
        let boat_count: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(9);
        let boat_count = boat_count.clamp(1, 24);
        let horizon = (height * 9 / 20).max(6).min(height.saturating_sub(6));

        for y in 0..horizon {
            for x in 0..width {
                if rng.random_range(0..20u32) == 0 {
                    grid[y][x] = Cell::new('·', darken(palette[4], 72));
                }
            }
        }

        let moon_x = width * 7 / 10;
        let moon_y = (horizon / 3).max(2);
        let moon_rx = (width / 9).max(5) as i32;
        let moon_ry = (height / 7).max(3) as i32;
        for y in
            moon_y.saturating_sub(moon_ry as usize)..=(moon_y + moon_ry as usize).min(height - 1)
        {
            for x in
                moon_x.saturating_sub(moon_rx as usize)..=(moon_x + moon_rx as usize).min(width - 1)
            {
                let dx = (x as i32 - moon_x as i32) as f32 / moon_rx as f32;
                let dy = (y as i32 - moon_y as i32) as f32 / moon_ry as f32;
                if dx * dx + dy * dy <= 1.0 {
                    let face = if (x + y) % 11 == 0 { '◉' } else { '●' };
                    grid[y][x] = Cell::new(face, lighten(palette[3], 35));
                }
            }
        }

        for _ in 0..5 {
            let fx = rng.random_range(6..width.saturating_sub(6).max(7));
            let fy = rng.random_range(2..horizon.saturating_sub(2).max(3));
            let burst = ['✦', '*', '+', '·'];
            for r in 0..4i32 {
                let c = shift_hue(lighten(palette[3], 20), rng.random_range(0..180u32) as f64);
                for &(dx, dy, ch) in &[
                    (r, 0, '─'),
                    (-r, 0, '─'),
                    (0, r, '│'),
                    (0, -r, '│'),
                    (r, r / 2, '╲'),
                    (-r, r / 2, '╱'),
                    (r, -r / 2, '╱'),
                    (-r, -r / 2, '╲'),
                ] {
                    let x = fx as i32 + dx;
                    let y = fy as i32 + dy;
                    if x >= 0 && y >= 0 && (x as usize) < width && (y as usize) < horizon {
                        grid[y as usize][x as usize] = Cell::new(
                            if r == 0 {
                                burst[rng.random_range(0..burst.len())]
                            } else {
                                ch
                            },
                            c,
                        );
                    }
                }
            }
        }

        let wheel_cx = width / 4;
        let wheel_cy = horizon.saturating_sub((height / 9).max(2));
        let wheel_r = (height / 4).max(4).min(width / 7);
        for i in 0..64 {
            let a = i as f32 / 64.0 * std::f32::consts::TAU;
            let x = wheel_cx as i32 + (a.cos() * wheel_r as f32 * 2.0).round() as i32;
            let y = wheel_cy as i32 + (a.sin() * wheel_r as f32).round() as i32;
            if x >= 0 && y >= 0 && (x as usize) < width && (y as usize) < horizon {
                grid[y as usize][x as usize] = Cell::new('○', lighten(palette[3], 10));
            }
        }
        for spoke in 0..8 {
            let a = spoke as f32 / 8.0 * std::f32::consts::TAU;
            for r in 0..=wheel_r {
                let x = wheel_cx as i32 + (a.cos() * r as f32 * 2.0).round() as i32;
                let y = wheel_cy as i32 + (a.sin() * r as f32).round() as i32;
                if x >= 0 && y >= 0 && (x as usize) < width && (y as usize) < horizon {
                    grid[y as usize][x as usize] = Cell::new(
                        if spoke % 2 == 0 { '─' } else { '╱' },
                        darken(palette[4], 15),
                    );
                }
            }
        }
        if wheel_cx < width && wheel_cy < height {
            grid[wheel_cy][wheel_cx] = Cell::new('◉', lighten(palette[4], 20));
        }

        let mut x = 0usize;
        while x < width {
            let w = rng.random_range(3..8usize);
            let h = rng.random_range(3..horizon.max(4));
            let top = horizon.saturating_sub(h);
            let color = shift_hue(palette[2], rng.random_range(0..120u32) as f64);
            for bx in x..(x + w).min(width) {
                for by in top..horizon {
                    let lit = (bx + by + seed as usize) % 5 == 0;
                    let ch = if by == top {
                        '▄'
                    } else if lit {
                        '▪'
                    } else {
                        '█'
                    };
                    let c = if lit {
                        lighten(palette[3], 20)
                    } else {
                        darken(color, 50)
                    };
                    grid[by][bx] = Cell::new(ch, c);
                }
            }
            x += w + rng.random_range(0..3usize);
        }

        for &crane_x in &[width / 8, width * 6 / 8] {
            let top = horizon.saturating_sub((height / 4).max(4));
            for y in top..horizon {
                if crane_x < width {
                    grid[y][crane_x] = Cell::new('┃', lighten(palette[3], 10));
                }
            }
            for x in crane_x..(crane_x + width / 6).min(width) {
                grid[top][x] = Cell::new('━', lighten(palette[3], 10));
            }
            let hook_x = (crane_x + width / 7).min(width - 1);
            for y in top..(top + 5).min(horizon) {
                grid[y][hook_x] = Cell::new('│', lighten(palette[4], 5));
            }
            if top + 5 < horizon {
                grid[top + 5][hook_x] = Cell::new('◆', palette[3]);
            }
        }

        let wave_chars = ['~', '≈', '∿', '─', ' '];
        for y in horizon..height {
            for x in 0..width {
                let drift = ((x as f32 / 5.0) + (y as f32 / 2.0)).sin();
                let idx =
                    ((x + y * 2 + seed as usize) % wave_chars.len()).min(wave_chars.len() - 1);
                let ch = if drift > 0.15 { '≈' } else { wave_chars[idx] };
                if ch != ' ' {
                    let depth = y - horizon;
                    let c = if (x + y) % 9 == 0 {
                        shift_hue(lighten(palette[3], 15), (x * 5 % 180) as f64)
                    } else {
                        darken(palette[1], (depth * 3).min(80) as u8)
                    };
                    grid[y][x] = Cell::new(ch, c);
                }
            }
        }

        let pier_y = (height * 2 / 3).min(height.saturating_sub(2));
        for x in 0..width {
            if x % 2 == 0 {
                grid[pier_y][x] = Cell::new('━', darken(palette[4], 25));
            }
        }
        for px in (width / 12..width * 11 / 12).step_by(7) {
            for y in pier_y..height {
                grid[y][px] = Cell::new('┃', darken(palette[4], 45));
            }
        }

        for _ in 0..boat_count {
            if width < 12 || height < horizon + 5 {
                break;
            }
            let len = rng.random_range(6..16usize).min(width.saturating_sub(3));
            let bx = rng.random_range(1..width.saturating_sub(len + 1).max(2));
            let by = rng.random_range(horizon + 2..height.saturating_sub(2).max(horizon + 3));
            let hull = shift_hue(lighten(palette[2], 25), rng.random_range(0..240u32) as f64);
            grid[by][bx] = Cell::new('╲', hull);
            for i in 1..len - 1 {
                if bx + i < width {
                    grid[by][bx + i] = Cell::new('━', hull);
                }
            }
            if bx + len - 1 < width {
                grid[by][bx + len - 1] = Cell::new('╱', hull);
            }

            let mast_x = bx + len / 2;
            let mast_h = rng.random_range(3..8usize).min(by);
            for k in 1..=mast_h {
                grid[by - k][mast_x] = Cell::new('│', lighten(palette[4], 5));
            }
            for k in 1..mast_h {
                let sx = mast_x.saturating_sub(k);
                if by >= k && sx < width {
                    grid[by - k][sx] = Cell::new('╱', lighten(palette[3], 15));
                }
                let sx2 = mast_x + k;
                if by >= k && sx2 < width && k < mast_h - 1 {
                    grid[by - k][sx2] = Cell::new('╲', lighten(palette[4], 5));
                }
            }
        }
    (grid, false)
}

/// Dispatch arm for mode(s): labyrinth (moved verbatim from run()).
pub(crate) fn cli_labyrinth(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        // labyrinth [markers] -- nested stone walls, deliberate gates, and one glowing route
        let marker_count: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(18);
        let marker_count = marker_count.clamp(0, 96);

        let bg_color = darken(palette[0], 8);
        let dust_color = darken(palette[2], 50);
        let floor_color = darken(palette[4], 44);
        let wall_color = lighten(palette[1], 18);
        let wall_shadow = darken(palette[2], 18);
        let path_color = lighten(palette[3], 34);
        let relic_color = lighten(palette[3], 48);

        for y in 0..height {
            for x in 0..width {
                let n = (x * 11 + y * 7 + seed as usize * 3) % 41;
                let ch = match n {
                    0 => '·',
                    1 => '∙',
                    2 => '░',
                    _ => ' ',
                };
                grid[y][x] = if ch == ' ' {
                    Cell::new(' ', bg_color)
                } else {
                    Cell::new(ch, dust_color)
                };
            }
        }

        let cx = width as i32 / 2;
        let cy = height as i32 / 2;
        let margin_x = (width / 10).clamp(4, 18);
        let margin_y = (height / 8).clamp(2, 7);
        let left = margin_x as i32;
        let right = width.saturating_sub(margin_x + 1) as i32;
        let top = margin_y as i32;
        let bottom = height.saturating_sub(margin_y + 1) as i32;

        if right - left >= 18 && bottom - top >= 8 {
            for y in top.saturating_sub(1)..=(bottom + 1).min(height as i32 - 1) {
                for x in left.saturating_sub(2)..=(right + 2).min(width as i32 - 1) {
                    if x < 0 || y < 0 {
                        continue;
                    }
                    let floor_noise =
                        ((x as usize * 5 + y as usize * 13 + seed as usize) % 29) == 0;
                    grid[y as usize][x as usize] = if floor_noise {
                        Cell::new('·', floor_color)
                    } else {
                        Cell::blank()
                    };
                }
            }

            let put = |grid: &mut Grid, x: i32, y: i32, ch: char, fg: Color| {
                if x >= 0 && y >= 0 && (x as usize) < width && (y as usize) < height {
                    grid[y as usize][x as usize] = Cell::new(ch, fg);
                }
            };
            let put_path = |grid: &mut Grid, x: i32, y: i32, step: usize| {
                if x < 0 || y < 0 || (x as usize) >= width || (y as usize) >= height {
                    return;
                }
                if matches!(
                    grid[y as usize][x as usize].ch,
                    '═' | '║' | '╔' | '╗' | '╚' | '╝' | '█' | '▓' | '╫' | '╬'
                ) {
                    return;
                }
                let ch = match step % 5 {
                    0 => '•',
                    1 | 2 => '·',
                    _ => '∙',
                };
                grid[y as usize][x as usize] = Cell::new(ch, path_color);
            };
            let draw_line = |grid: &mut Grid, a: (i32, i32), b: (i32, i32), start_step: usize| {
                let mut step = start_step;
                let (mut x, mut y) = a;
                while x != b.0 {
                    put_path(grid, x, y, step);
                    x += if b.0 > x { 1 } else { -1 };
                    step += 1;
                }
                while y != b.1 {
                    put_path(grid, x, y, step);
                    y += if b.1 > y { 1 } else { -1 };
                    step += 1;
                }
                put_path(grid, x, y, step);
            };
            let side_at = |side: usize, l: i32, t: i32, r: i32, b: i32, gx: i32, gy: i32| match side
            {
                0 => (gx.clamp(l + 2, r - 2), t),
                1 => (r, gy.clamp(t + 1, b - 1)),
                2 => (gx.clamp(l + 2, r - 2), b),
                _ => (l, gy.clamp(t + 1, b - 1)),
            };
            let inside_gate =
                |side: usize, l: i32, t: i32, r: i32, b: i32, gx: i32, gy: i32| match side {
                    0 => (gx.clamp(l + 2, r - 2), t + 1),
                    1 => (r - 1, gy.clamp(t + 1, b - 1)),
                    2 => (gx.clamp(l + 2, r - 2), b - 1),
                    _ => (l + 1, gy.clamp(t + 1, b - 1)),
                };
            let outside_gate =
                |side: usize, l: i32, t: i32, r: i32, b: i32, gx: i32, gy: i32| match side {
                    0 => (gx.clamp(l + 2, r - 2), t - 1),
                    1 => (r + 1, gy.clamp(t + 1, b - 1)),
                    2 => (gx.clamp(l + 2, r - 2), b + 1),
                    _ => (l - 1, gy.clamp(t + 1, b - 1)),
                };
            let perimeter_dist = |side: usize, x: i32, y: i32, l: i32, t: i32, r: i32, b: i32| {
                let w = r - l;
                let h = b - t;
                match side {
                    0 => x - l,
                    1 => w + y - t,
                    2 => w + h + r - x,
                    _ => w + h + w + b - y,
                }
            };
            let perimeter_point = |dist: i32, l: i32, t: i32, r: i32, b: i32| {
                let w = r - l;
                let h = b - t;
                let p = 2 * (w + h);
                let d = dist.rem_euclid(p);
                if d <= w {
                    (l + d, t)
                } else if d <= w + h {
                    (r, t + d - w)
                } else if d <= w + h + w {
                    (r - (d - w - h), b)
                } else {
                    (l, b - (d - w - h - w))
                }
            };

            let max_layers =
                ((height.saturating_sub(10) / 4).min(width.saturating_sub(20) / 12)).clamp(3, 8);
            let inset_x = ((right - left) as usize / (max_layers * 2 + 3)).clamp(4, 14) as i32;
            let inset_y = ((bottom - top) as usize / (max_layers * 2 + 3)).clamp(2, 5) as i32;

            let mut rings: Vec<(i32, i32, i32, i32, usize, i32, i32)> = Vec::new();
            for layer in 0..max_layers {
                let l = left + layer as i32 * inset_x;
                let r = right - layer as i32 * inset_x;
                let t = top + layer as i32 * inset_y;
                let b = bottom - layer as i32 * inset_y;
                if r - l < 13 || b - t < 5 {
                    break;
                }
                let side = match layer % 4 {
                    0 => 2,
                    1 => 1,
                    2 => 0,
                    _ => 3,
                };
                let x_span = (r - l - 8).max(1);
                let y_span = (b - t - 4).max(1);
                let x_wobble =
                    ((seed as i32 + layer as i32 * 17).rem_euclid(11) - 5) * ((x_span / 18).max(1));
                let y_wobble = ((seed as i32 / 3 + layer as i32 * 13).rem_euclid(9) - 4)
                    * ((y_span / 10).max(1));
                let gx = (cx + x_wobble).clamp(l + 4, r - 4);
                let gy = (cy + y_wobble).clamp(t + 2, b - 2);
                rings.push((l, t, r, b, side, gx, gy));
            }

            for (layer, &(l, t, r, b, side, gx, gy)) in rings.iter().enumerate() {
                let tone = if layer % 2 == 0 {
                    wall_color
                } else {
                    darken(wall_color, 12)
                };
                let half_gate_x = 2 + (layer as i32 % 2);
                let half_gate_y = 1;

                for x in l..=r {
                    let top_gate = side == 0 && (gx - half_gate_x..=gx + half_gate_x).contains(&x);
                    let bottom_gate =
                        side == 2 && (gx - half_gate_x..=gx + half_gate_x).contains(&x);
                    if !top_gate {
                        put(&mut grid, x, t, '═', tone);
                    }
                    if !bottom_gate {
                        put(&mut grid, x, b, '═', tone);
                    }
                    if layer % 2 == 0 && x > l && x < r && x % 9 == (seed as i32 % 9) {
                        if !top_gate {
                            put(&mut grid, x, t, '╫', darken(tone, 6));
                        }
                        if !bottom_gate {
                            put(&mut grid, x, b, '╫', darken(tone, 6));
                        }
                    }
                }
                for y in t..=b {
                    let left_gate = side == 3 && (gy - half_gate_y..=gy + half_gate_y).contains(&y);
                    let right_gate =
                        side == 1 && (gy - half_gate_y..=gy + half_gate_y).contains(&y);
                    if !left_gate {
                        put(&mut grid, l, y, '║', tone);
                    }
                    if !right_gate {
                        put(&mut grid, r, y, '║', tone);
                    }
                    if layer % 2 == 1 && y > t && y < b && y % 5 == (seed as i32 % 5) {
                        if !left_gate {
                            put(&mut grid, l, y, '╫', darken(tone, 6));
                        }
                        if !right_gate {
                            put(&mut grid, r, y, '╫', darken(tone, 6));
                        }
                    }
                }

                put(&mut grid, l, t, '╔', tone);
                put(&mut grid, r, t, '╗', tone);
                put(&mut grid, l, b, '╚', tone);
                put(&mut grid, r, b, '╝', tone);

                let (gate_x, gate_y) = side_at(side, l, t, r, b, gx, gy);
                match side {
                    0 | 2 => {
                        put(
                            &mut grid,
                            gate_x - half_gate_x - 1,
                            gate_y,
                            '█',
                            wall_shadow,
                        );
                        put(
                            &mut grid,
                            gate_x + half_gate_x + 1,
                            gate_y,
                            '█',
                            wall_shadow,
                        );
                        put(&mut grid, gate_x, gate_y, '╬', relic_color);
                    }
                    _ => {
                        put(
                            &mut grid,
                            gate_x,
                            gate_y - half_gate_y - 1,
                            '█',
                            wall_shadow,
                        );
                        put(
                            &mut grid,
                            gate_x,
                            gate_y + half_gate_y + 1,
                            '█',
                            wall_shadow,
                        );
                        put(&mut grid, gate_x, gate_y, '╬', relic_color);
                    }
                }
            }

            if let Some(&(l0, t0, r0, b0, side0, gx0, gy0)) = rings.first() {
                let entry = outside_gate(side0, l0, t0, r0, b0, gx0, gy0);
                let mut current = inside_gate(side0, l0, t0, r0, b0, gx0, gy0);
                let label_y = (entry.1 + 1).min(height as i32 - 1);
                for y in entry.1.max(0)..=label_y {
                    for dx in -2..=2 {
                        put(&mut grid, entry.0 + dx, y, ' ', floor_color);
                    }
                }
                draw_line(&mut grid, entry, current, 0);
                put(&mut grid, entry.0, label_y, 'S', lighten(palette[1], 26));

                for i in 0..rings.len().saturating_sub(1) {
                    let (l, t, r, b, side, gx, gy) = rings[i];
                    let (nl, nt, nr, nb, next_side, next_gx, next_gy) = rings[i + 1];
                    let pl = (l + nl) / 2;
                    let pr = (r + nr) / 2;
                    let pt = (t + nt) / 2;
                    let pb = (b + nb) / 2;
                    if pr <= pl || pb <= pt {
                        continue;
                    }
                    let start = side_at(side, pl, pt, pr, pb, gx, gy);
                    let end = side_at(next_side, pl, pt, pr, pb, next_gx, next_gy);
                    draw_line(&mut grid, current, start, i * 97);

                    let p = 2 * ((pr - pl) + (pb - pt));
                    let d1 = perimeter_dist(side, start.0, start.1, pl, pt, pr, pb);
                    let d2 = perimeter_dist(next_side, end.0, end.1, pl, pt, pr, pb);
                    let cw = (d2 - d1).rem_euclid(p);
                    let ccw = (d1 - d2).rem_euclid(p);
                    let go_clockwise = if i % 3 == 1 { cw > ccw } else { cw <= ccw };
                    let steps = if go_clockwise { cw } else { ccw };
                    for step in 0..=steps {
                        let d = if go_clockwise { d1 + step } else { d1 - step };
                        let (px, py) = perimeter_point(d, pl, pt, pr, pb);
                        put_path(&mut grid, px, py, i * 131 + step as usize);
                    }

                    let next_outside = outside_gate(next_side, nl, nt, nr, nb, next_gx, next_gy);
                    let next_inside = inside_gate(next_side, nl, nt, nr, nb, next_gx, next_gy);
                    draw_line(&mut grid, end, next_outside, i * 173);
                    draw_line(&mut grid, next_outside, next_inside, i * 211);
                    current = next_inside;
                }

                let inner = rings
                    .last()
                    .copied()
                    .unwrap_or((l0, t0, r0, b0, side0, gx0, gy0));
                let chamber_w = ((inner.2 - inner.0) / 2).clamp(8, 22);
                let chamber_h = ((inner.3 - inner.1) / 2).clamp(3, 7);
                let cl = (cx - chamber_w / 2).clamp(inner.0 + 2, inner.2 - chamber_w - 1);
                let cr = cl + chamber_w;
                let ct = (cy - chamber_h / 2).clamp(inner.1 + 1, inner.3 - chamber_h - 1);
                let cb = ct + chamber_h;
                let door_side = inner.4;
                let door_x = cx.clamp(cl + 2, cr - 2);
                let door_y = cy.clamp(ct + 1, cb - 1);

                for x in cl..=cr {
                    if !(door_side == 0 && (door_x - 1..=door_x + 1).contains(&x)) {
                        put(&mut grid, x, ct, '═', relic_color);
                    }
                    if !(door_side == 2 && (door_x - 1..=door_x + 1).contains(&x)) {
                        put(&mut grid, x, cb, '═', relic_color);
                    }
                }
                for y in ct..=cb {
                    if !(door_side == 3 && (door_y - 1..=door_y + 1).contains(&y)) {
                        put(&mut grid, cl, y, '║', relic_color);
                    }
                    if !(door_side == 1 && (door_y - 1..=door_y + 1).contains(&y)) {
                        put(&mut grid, cr, y, '║', relic_color);
                    }
                }
                put(&mut grid, cl, ct, '╔', relic_color);
                put(&mut grid, cr, ct, '╗', relic_color);
                put(&mut grid, cl, cb, '╚', relic_color);
                put(&mut grid, cr, cb, '╝', relic_color);

                let chamber_entry = match door_side {
                    0 => (door_x, ct - 1),
                    1 => (cr + 1, door_y),
                    2 => (door_x, cb + 1),
                    _ => (cl - 1, door_y),
                };
                let chamber_inside = match door_side {
                    0 => (door_x, ct + 1),
                    1 => (cr - 1, door_y),
                    2 => (door_x, cb - 1),
                    _ => (cl + 1, door_y),
                };
                draw_line(&mut grid, current, chamber_entry, 901);
                draw_line(&mut grid, chamber_entry, chamber_inside, 941);
                draw_line(&mut grid, chamber_inside, (cx, cy), 991);
                put(&mut grid, cx, cy, '◉', relic_color);
            }

            let glyphs = ['◆', '◇', '✦', '✧', '+'];
            for _ in 0..marker_count {
                if rings.len() < 2 {
                    break;
                }
                let ring = rng.random_range(0..rings.len() - 1);
                let (ol, ot, or, ob, _, _, _) = rings[ring];
                let (il, it, ir, ib, _, _, _) = rings[ring + 1];
                let side = rng.random_range(0..4);
                let (x, y) = match side {
                    0 => (rng.random_range(il + 1..ir), (ot + it) / 2),
                    1 => ((or + ir) / 2, rng.random_range(it + 1..ib)),
                    2 => (rng.random_range(il + 1..ir), (ob + ib) / 2),
                    _ => ((ol + il) / 2, rng.random_range(it + 1..ib)),
                };
                if x <= 0 || y <= 0 || x as usize >= width - 1 || y as usize >= height - 1 {
                    continue;
                }
                if grid[y as usize][x as usize].ch == ' ' {
                    grid[y as usize][x as usize] =
                        Cell::new(glyphs[rng.random_range(0..glyphs.len())], relic_color);
                }
            }
        }
    (grid, false)
}

/// Dispatch arm for mode(s): rainfall (moved verbatim from run()).
pub(crate) fn cli_rainfall(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        // rainfall [intensity] -- wind-sheared rain, gutters, puddles, and bright strikes
        let intensity: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(65);
        let intensity = intensity.clamp(5, 150);
        let wind = match seed % 3 {
            0 => -1i32,
            1 => 0,
            _ => 1,
        };

        let field = Rect {
            x: 0,
            y: 0,
            w: width,
            h: height,
        };
        fill_noise(
            &mut grid,
            &field,
            NoiseVariant::Dot,
            darken(palette[2], 90),
            darken(palette[1], 85),
            &mut rng,
        );

        let drops = (width * height * intensity / 180).max(width / 2);
        for _ in 0..drops {
            let len = rng.random_range(2..8i32);
            let x0 = rng.random_range(0..width) as i32;
            let y0 = rng.random_range(0..height) as i32;
            for step in 0..len {
                let x = x0 + wind * step / 2;
                let y = y0 + step;
                if x < 0 || y < 0 || x as usize >= width || y as usize >= height {
                    continue;
                }
                let ch = match wind {
                    -1 => '╱',
                    1 => '╲',
                    _ => '│',
                };
                grid[y as usize][x as usize] =
                    Cell::new(ch, darken(lighten(palette[4], 5), rng.random_range(0..45)));
            }
        }

        let gutter_y = height.saturating_sub(5);
        for x in 0..width {
            if x % 2 == 0 {
                grid[gutter_y][x] = Cell::new('═', darken(palette[4], 45));
            }
            if x % 17 == 0 {
                for y in gutter_y..height {
                    grid[y][x] = Cell::new('║', darken(palette[4], 55));
                }
            }
        }

        for _ in 0..(width / 5).max(4) {
            let px = rng.random_range(0..width);
            let py = rng.random_range(gutter_y..height);
            let r = rng.random_range(2..7usize);
            for dx in 0..r {
                let x = px + dx;
                if x >= width {
                    break;
                }
                let ch = ['~', '≈', '∿', '_'][rng.random_range(0..4usize)];
                grid[py][x] = Cell::new(ch, lighten(palette[1], 15));
            }
        }

        let strikes = if intensity > 95 { 2 } else { 1 };
        for _ in 0..strikes {
            let mut x = rng.random_range(width / 5..(width * 4 / 5).max(width / 5 + 1)) as i32;
            let end_y = rng.random_range((height / 3).max(1)..(height * 2 / 3).max(2));
            for y in 0..end_y {
                if x >= 0 && (x as usize) < width {
                    let ch = if rng.random_range(0..2u32) == 0 {
                        '╲'
                    } else {
                        '╱'
                    };
                    grid[y][x as usize] = Cell::new(ch, lighten(palette[3], 35));
                }
                x += rng.random_range(-1..=1);
            }
        }
    (grid, false)
}

/// Dispatch arm for mode(s): meadow (moved verbatim from run()).
pub(crate) fn cli_meadow(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        // meadow [density] -- windy wildflower field with stems, seed heads, and grass
        let density: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(70);
        let density = density.clamp(10, 180);
        let horizon = (height / 3).max(2);
        let ground = gen_contour(width, horizon, (height / 10).max(2), 0.55, &mut rng);

        for y in 0..height {
            for x in 0..width {
                if y < ground[x].min(height - 1) {
                    if rng.random_range(0..24u32) == 0 {
                        grid[y][x] = Cell::new('·', darken(palette[4], 70));
                    }
                    continue;
                }
                let depth = y.saturating_sub(ground[x]);
                let ch = ['╱', '╲', '│', '∿', '·', ' '][rng.random_range(0..6usize)];
                if ch != ' ' {
                    grid[y][x] = Cell::new(ch, darken(palette[1], (depth * 4).min(85) as u8));
                }
            }
        }

        let full = Rect {
            x: 0,
            y: 0,
            w: width,
            h: height,
        };
        draw_contour_ridge(&mut grid, &full, &ground, darken(palette[3], 20));

        let stem_count = (width * density / 28).clamp(8, width.saturating_mul(3).max(8));
        for _ in 0..stem_count {
            let bx = rng.random_range(0..width);
            let base_y = rng.random_range(ground[bx].min(height - 1)..height);
            let len = rng.random_range(3..(height / 3).max(5) as u32) as i32;
            let lean = rng.random_range(-3..=3i32);
            let color = darken(palette[2], rng.random_range(0..45));
            let mut top = (bx as i32, base_y as i32);
            for i in 0..len {
                let t = i as f32 / len.max(1) as f32;
                let sway = (t * std::f32::consts::PI).sin() * lean as f32;
                let x = bx as i32 + sway.round() as i32;
                let y = base_y as i32 - i;
                if x < 0 || y < 0 || x as usize >= width || y as usize >= height {
                    continue;
                }
                let ch = if lean < -1 {
                    '╱'
                } else if lean > 1 {
                    '╲'
                } else {
                    '│'
                };
                grid[y as usize][x as usize] = Cell::new(ch, color);
                top = (x, y);
            }
            if top.0 <= 1
                || top.1 <= 1
                || top.0 as usize >= width - 1
                || top.1 as usize >= height - 1
            {
                continue;
            }
            let tx = top.0 as usize;
            let ty = top.1 as usize;
            match rng.random_range(0..5u32) {
                0 => draw_flower(
                    &mut grid,
                    tx,
                    ty,
                    rng.random_range(0..5),
                    lighten(palette[3], 15),
                ),
                1 => grow_flower_spiral(&mut grid, tx, ty, palette[3], &mut rng),
                2 => draw_fruit(
                    &mut grid,
                    tx,
                    ty,
                    rng.random_range(0..5),
                    lighten(palette[2], 20),
                ),
                _ => {
                    let seed_chars = ['✦', '✧', '*', '·'];
                    grid[ty][tx] = Cell::new(
                        seed_chars[rng.random_range(0..seed_chars.len())],
                        lighten(palette[4], 5),
                    );
                }
            }
        }
    (grid, false)
}

/// Dispatch arm for mode(s): world2 (moved verbatim from run()).
pub(crate) fn cli_world2(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        // world2 [shards] -- cracked/leaking biome partitions with aurora and scene islands
        let shard_count: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(6);
        let shard_count = shard_count.clamp(3, 10);
        let crack_count = shard_count.saturating_sub(1);

        let bg = Rect {
            x: 0,
            y: 0,
            w: width,
            h: height,
        };
        fill_noise(
            &mut grid,
            &bg,
            NoiseVariant::Dot,
            darken(palette[2], 96),
            darken(palette[3], 92),
            &mut rng,
        );

        let mut cracks: Vec<Vec<i32>> = Vec::new();
        for i in 0..crack_count {
            let band = (width / shard_count).max(2) as i32;
            let mut x = ((i + 1) as i32 * band + rng.random_range(-band / 4..=band / 4))
                .clamp(1, width as i32 - 2);
            let mut drift = 0i32;
            let mut path = Vec::with_capacity(height);
            for y in 0..height {
                path.push(x);
                if rng.random::<f32>() < 0.35 || y % 7 == 0 {
                    drift = rng.random_range(-1..=1);
                }
                x = (x + drift).clamp(1, width as i32 - 2);
            }
            cracks.push(path);
        }

        let mut bounds = vec![0usize];
        for path in &cracks {
            let avg = (path.iter().sum::<i32>() / path.len().max(1) as i32)
                .clamp(1, width as i32 - 1) as usize;
            bounds.push(avg);
        }
        bounds.push(width);
        bounds.sort();
        bounds.dedup();

        for i in 0..bounds.len().saturating_sub(1) {
            let left = bounds[i];
            let right = bounds[i + 1].max(left + 1).min(width);
            if right <= left {
                continue;
            }
            let rect = Rect {
                x: left,
                y: 0,
                w: right - left,
                h: height,
            };
            let biome = biome_from_index((i + seed as usize) % 5);
            render_biome(biome, &mut grid, &rect, &palette, &mut rng);
        }

        let base = grid.clone();
        for (ci, path) in cracks.iter().enumerate() {
            for y in 0..height {
                let seam_x = path[y];
                for spread in 1..=5i32 {
                    for side in [-1i32, 1i32] {
                        let tx = seam_x + side * spread;
                        let src = seam_x - side * rng.random_range(1..=7i32);
                        if tx < 0 || src < 0 || tx as usize >= width || src as usize >= width {
                            continue;
                        }
                        let leak_strength = 1.0 - spread as f32 / 6.0;
                        let upper_boost = if y < height / 3 { 0.18 } else { 0.0 };
                        if rng.random::<f32>() > leak_strength * 0.45 + upper_boost {
                            continue;
                        }
                        let mut cell = base[y][src as usize];
                        if cell.ch == ' ' && rng.random::<f32>() < 0.65 {
                            continue;
                        }
                        cell.fg = if (ci + y + spread as usize) % 3 == 0 {
                            shift_hue(darken(cell.fg, (spread * 8) as u8), 30.0)
                        } else {
                            darken(cell.fg, (spread * 10) as u8)
                        };
                        grid[y][tx as usize] = cell;
                    }
                }
            }
        }

        let aurora_bands = 4usize;
        for b in 0..aurora_bands {
            let color = shift_hue(lighten(palette[3], 25), b as f64 * 39.0);
            let base_y = height / 10 + b * (height / 4).max(1) / aurora_bands;
            let amp = rng.random_range(1..(height / 8).max(3) as u32) as f32;
            let phase = rng.random::<f32>() * std::f32::consts::TAU;
            for x in 0..width {
                let y = base_y as i32 + ((x as f32 / 13.0 + phase).sin() * amp).round() as i32;
                if y <= 0 || y as usize >= height / 2 {
                    continue;
                }
                if rng.random::<f32>() < 0.82 {
                    grid[y as usize][x] = Cell::new(if b % 2 == 0 { '═' } else { '~' }, color);
                }
                if y + 1 < height as i32 && rng.random::<f32>() < 0.45 {
                    grid[(y + 1) as usize][x] = Cell::new('·', darken(color, 30));
                }
            }
        }

        let seam_colors = [lighten(palette[3], 30), lighten(palette[4], 10), palette[1]];
        for (ci, path) in cracks.iter().enumerate() {
            let seam = seam_colors[ci % seam_colors.len()];
            for y in 0..height {
                let x = path[y];
                let next = if y + 1 < height { path[y + 1] } else { x };
                let ch = if next > x {
                    '╲'
                } else if next < x {
                    '╱'
                } else {
                    '│'
                };
                if x >= 0 && (x as usize) < width {
                    grid[y][x as usize] = Cell::new(ch, seam);
                }
                if rng.random::<f32>() < 0.09 {
                    let dir: i32 = if rng.random::<f32>() < 0.5 { -1 } else { 1 };
                    let len = rng.random_range(2..6i32);
                    for k in 1..=len {
                        let bx = x + dir * k;
                        let by = y + k as usize;
                        if bx >= 0 && (bx as usize) < width && by < height {
                            grid[by][bx as usize] = Cell::new(
                                if dir > 0 { '╲' } else { '╱' },
                                darken(seam, (k * 9) as u8),
                            );
                        }
                    }
                }
            }
        }

        let island_count = rng.random_range(5..9usize);
        let mut layers = Vec::new();
        let mut stops = Vec::new();
        for i in 0..island_count {
            let cx = rng.random_range(width / 8..(width * 7 / 8).max(width / 8 + 1));
            let cy = rng.random_range(height / 5..(height * 4 / 5).max(height / 5 + 1));
            let rx = rng.random_range(5..13usize);
            let ry = rng.random_range(3..7usize);
            let fill = match rng.random_range(0..9u32) {
                0 => FillGen::Tree(rng.random_range(0..12)),
                1 => FillGen::Flower(rng.random_range(0..5)),
                2 => FillGen::Fruit(rng.random_range(0..5)),
                3 => FillGen::Mask(
                    rng.random_range(2..5),
                    rng.random_range(0..MASK_STYLE_COUNT),
                ),
                4 => FillGen::AztecDiamond(rng.random_range(2..6)),
                5 => FillGen::Labyrinth,
                6 => FillGen::Noise(NoiseVariant::Grass),
                7 => FillGen::Tile(TileParams::randomized(&mut rng)),
                _ => FillGen::Concentric,
            };
            let mut p = palette;
            p[1] = shift_hue(palette[1], (i * 37) as f64);
            p[2] = shift_hue(palette[2], (i * 53) as f64);
            p[3] = shift_hue(lighten(palette[3], 10), (i * 71) as f64);
            layers.push(Layer {
                fill,
                mask: Some(Box::new(mask_ellipse(
                    cx as f32,
                    cy as f32,
                    rx as f32 * 2.0,
                    ry as f32,
                    0.75,
                ))),
                palette: p,
            });
            stops.push((cx, cy));
        }
        let scene = Scene { layers };
        let full = Rect {
            x: 0,
            y: 0,
            w: width,
            h: height,
        };
        render_scene(&mut grid, &full, &scene, &mut rng);
        stops.sort_by_key(|p| p.0);
        draw_path_trail(&mut grid, &stops, lighten(palette[4], 10), &mut rng);

        for &(sx, sy) in &stops {
            if sx < width && sy < height {
                grid[sy][sx] = Cell::new('◆', lighten(palette[3], 25));
            }
        }

        for (ci, path) in cracks.iter().enumerate() {
            let seam = seam_colors[ci % seam_colors.len()];
            for y in 0..height {
                let x = path[y];
                let next = if y + 1 < height { path[y + 1] } else { x };
                let ch = if next > x {
                    '╲'
                } else if next < x {
                    '╱'
                } else {
                    '┃'
                };
                if x >= 0 && (x as usize) < width {
                    grid[y][x as usize] = Cell::new(ch, lighten(seam, 12));
                }
                for dx in [-1i32, 1i32] {
                    let gx = x + dx;
                    if gx >= 0
                        && (gx as usize) < width
                        && rng.random::<f32>() < 0.35
                        && grid[y][gx as usize].ch != '◆'
                    {
                        grid[y][gx as usize] = Cell::new('·', darken(seam, 25));
                    }
                }
            }
        }
    (grid, false)
}
