//! Node-scene builders (landscape, centerpiece, cluster).
#![allow(unused)]
use crate::color::*;
use crate::fills::*;
use crate::scene::*;
use crate::sprites::*;
use crate::types::*;
use crossterm::style::Color;
use rand::RngExt;
use rand::rngs::StdRng;
use super::*;
use super::shift_hue;

/// Generate a multi-layer scene for one node of a party walk.
/// `rect` is the bounding box, `detail` is 0-100 controlling density.
pub fn make_node_scene(
    rect: &Rect,
    mode: NodeMode,
    palette: &[Color; 5],
    detail: u32,
    rng: &mut StdRng,
) -> Vec<Layer> {
    match mode {
        NodeMode::Landscape => make_landscape(rect, palette, detail, rng),
        NodeMode::CenterpieceWithSurround => make_centerpiece(rect, palette, rng),
        NodeMode::Cluster(arr, n) => make_cluster(rect, arr, n, palette, rng),
        NodeMode::NegativeSpace => make_negative_space(rect, palette, rng),
    }
}
/// Generate a wavy contour line across a rect width.
/// Returns a Vec of y-values (one per column from rect.x to rect.x+rect.w).
/// `base_y`: average y of the contour.
/// `amplitude`: max deviation from base.
/// `freq`: wave frequency (higher = more peaks).
fn gen_contour(
    rect: &Rect,
    base_y: usize,
    amplitude: f32,
    freq: f32,
    rng: &mut StdRng,
) -> Vec<usize> {
    let phase = rng.random::<f32>() * std::f32::consts::TAU;
    let phase2 = rng.random::<f32>() * std::f32::consts::TAU;
    (0..rect.w)
        .map(|col| {
            let t = col as f32 / rect.w.max(1) as f32;
            // Two sine waves at different frequencies for organic feel
            let wave = (t * freq * std::f32::consts::TAU + phase).sin() * amplitude
                + (t * freq * 2.3 * std::f32::consts::TAU + phase2).sin() * amplitude * 0.4;
            let y = base_y as f32 + wave;
            y.clamp(rect.y as f32 + 2.0, (rect.y + rect.h - 2) as f32) as usize
        })
        .collect()
}
/// Landscape: 3-pass rendering.
/// Pass 1: sky background (sparse dots or nothing -- mostly empty).
/// Pass 2: ground below a wavy contour line (grass/tile/crosshatch).
/// Pass 3: foreground sprites rooted ON the ground line.
pub(crate) fn make_landscape(rect: &Rect, palette: &[Color; 5], detail: u32, rng: &mut StdRng) -> Vec<Layer> {
    let mut layers = Vec::new();

    // Horizon: wavy contour line at 40-60% from top
    let horizon_frac = rng.random_range(35..60) as f32 / 100.0;
    let base_horizon = rect.y + (rect.h as f32 * horizon_frac) as usize;
    let amplitude = (rect.h as f32 * 0.08).max(1.0);
    let freq = rng.random_range(8..20) as f32 / 10.0;
    let contour = gen_contour(rect, base_horizon, amplitude, freq, rng);

    // ── Pass 1: Sky (sparse dots, mostly empty) ──
    let sky_pal = {
        let mut p = *palette;
        p[1] = darken(palette[rng.random_range(1..4)], 70);
        p
    };
    // Sky is above the contour -- use mask_above_contour
    layers.push(Layer {
        fill: FillGen::Noise(NoiseVariant::Dot),
        mask: Some(Box::new(mask_above_contour(contour.clone(), rect.x, 2.0))),
        palette: sky_pal,
    });

    // ── Pass 2: Ground below the contour ──
    let ground_pal = {
        let mut p = *palette;
        p[1] = palette[rng.random_range(1..4)];
        p
    };
    let ground_fill = match rng.random_range(0..5u32) {
        0 => FillGen::Noise(NoiseVariant::Grass),
        1 => FillGen::Noise(NoiseVariant::Higaki),
        2 => FillGen::Tile(TileParams::randomized(rng)),
        3 => FillGen::Crosshatch,
        _ => FillGen::Noise(NoiseVariant::Truchet),
    };
    layers.push(Layer {
        fill: ground_fill,
        mask: Some(Box::new(mask_below_contour(contour.clone(), rect.x, 3.0))),
        palette: ground_pal,
    });

    // ── Pass 3: Foreground elements rooted on the contour ──
    let fg_count = 1 + (detail as f32 / 20.0) as u32 + rng.random_range(0..3u32);
    for fi in 0..fg_count {
        let mut pal = *palette;
        pal[1] = palette[rng.random_range(1..4)];

        // Pick a column along the contour, spread elements across the width
        let col_frac = (fi as f32 + 0.5) / fg_count as f32;
        let col = (rect.w as f32 * col_frac) as usize;
        let ground_y = contour.get(col).copied().unwrap_or(base_horizon);
        let ex = rect.x + col;

        let (fill, ew, eh) = match rng.random_range(0..10u32) {
            0..=3 => {
                // Trees rooted on the ground line
                let tw = rng.random_range(12..rect.w.min(24).max(13));
                let th = rng.random_range(8..rect.h.min(18).max(9));
                (FillGen::Tree(rng.random_range(0..12)), tw, th)
            }
            4..=5 => (FillGen::Flower(rng.random_range(0..5)), 5, 5),
            6 => {
                let s = rng.random_range(2..5);
                (
                    FillGen::Mask(s, rng.random_range(0..MASK_STYLE_COUNT)),
                    s * 4 + 4,
                    s * 4 + 4,
                )
            }
            7 => {
                let tw = rng.random_range(10..20);
                let th = rng.random_range(6..12);
                (FillGen::Tile(TileParams::randomized(rng)), tw, th)
            }
            8 => (FillGen::Fruit(rng.random_range(0..5)), 5, 5),
            _ => {
                // Aztec diamond accent
                let order = rng.random_range(2..5);
                (FillGen::AztecDiamond(order), order * 4 + 4, order * 2 + 4)
            }
        };

        // Root the element so its bottom sits on the ground line
        let elx = ex
            .saturating_sub(ew / 2)
            .min(rect.x + rect.w - ew.min(rect.w));
        let ely = ground_y
            .saturating_sub(eh)
            .max(rect.y)
            .min(rect.y + rect.h - eh.min(rect.h));
        let el_rect = Rect {
            x: elx,
            y: ely,
            w: ew.min(rect.w),
            h: eh.min(rect.h),
        };

        let mask: MaskFn = if fill_breaks_out(&fill) {
            Box::new(mask_rect(&el_rect, 0.0))
        } else {
            let cx = elx as f32 + ew as f32 * 0.5;
            let cy = ely as f32 + eh as f32 * 0.5;
            Box::new(mask_ellipse(cx, cy, ew as f32 * 0.5, eh as f32 * 0.5, 2.0))
        };

        layers.push(Layer {
            fill,
            mask: Some(mask),
            palette: pal,
        });
    }

    layers
}
/// Centerpiece with surround: one big element in a patterned field.
pub(crate) fn make_centerpiece(rect: &Rect, palette: &[Color; 5], rng: &mut StdRng) -> Vec<Layer> {
    let mut layers = Vec::new();
    let cx = rect.x as f32 + rect.w as f32 * 0.5;
    let cy = rect.y as f32 + rect.h as f32 * 0.5;
    let rx = rect.w as f32 * 0.5;
    let ry = rect.h as f32 * 0.5;

    // Background surround pattern (tile or noise)
    let mut bg_pal = *palette;
    bg_pal[1] = darken(palette[rng.random_range(1..4)], 40);
    let bg_fill = match rng.random_range(0..8u32) {
        0 => FillGen::Tile(TileParams::randomized(rng)),
        1 => FillGen::Crosshatch,
        2 => FillGen::Guilloche,
        3 => FillGen::Zigzag,
        4 => FillGen::DiamondLattice,
        5 => FillGen::Spiral,
        6 => FillGen::Concentric,
        _ => FillGen::Labyrinth,
    };
    // Use a shape mask for the whole node
    let node_mask: MaskFn = match rng.random_range(0..4u32) {
        0 => Box::new(mask_ellipse(cx, cy, rx, ry, 2.5)),
        1 => Box::new(mask_diamond(cx, cy, rx, ry, 2.0)),
        2 => Box::new(mask_rect(rect, 2.0)),
        _ => {
            let (wt, wb) = if rng.random_range(0..2u32) == 0 {
                (rect.w as f32 * 0.4, rect.w as f32 * 0.9)
            } else {
                (rect.w as f32 * 0.9, rect.w as f32 * 0.4)
            };
            Box::new(mask_trapezoid(cx, cy, wt, wb, rect.h as f32 * 0.9, 2.0))
        }
    };
    layers.push(Layer {
        fill: bg_fill,
        mask: Some(node_mask),
        palette: bg_pal,
    });

    // Centerpiece: big sprite
    let mut cp_pal = *palette;
    cp_pal[1] = palette[rng.random_range(1..4)];
    let (cp_fill, cw, ch) = match rng.random_range(0..6u32) {
        0..=1 => {
            let s = rng.random_range(3..6);
            (
                FillGen::Mask(s, rng.random_range(0..MASK_STYLE_COUNT)),
                s * 4 + 4,
                s * 4 + 4,
            )
        }
        2 => {
            let order = rng.random_range(3..7);
            (FillGen::AztecDiamond(order), order * 4 + 4, order * 2 + 4)
        }
        3 => {
            let steps = rng.random_range(3..6);
            (FillGen::Fret(steps), steps * 4 + 2, steps * 4 + 2)
        }
        _ => {
            let tw = rng.random_range(14..rect.w.min(28).max(15));
            let th = rng.random_range(10..rect.h.min(20).max(11));
            (FillGen::Tree(rng.random_range(0..12)), tw, th)
        }
    };
    let elx = (cx as usize)
        .saturating_sub(cw / 2)
        .min(rect.x + rect.w - cw.min(rect.w));
    let ely = (cy as usize)
        .saturating_sub(ch / 2)
        .min(rect.y + rect.h - ch.min(rect.h));
    let el_rect = Rect {
        x: elx,
        y: ely,
        w: cw.min(rect.w),
        h: ch.min(rect.h),
    };
    layers.push(Layer {
        fill: cp_fill,
        mask: Some(Box::new(mask_rect(&el_rect, 0.0))),
        palette: cp_pal,
    });

    layers
}
/// Cluster: N related patterns in a spatial arrangement.
pub(crate) fn make_cluster(
    rect: &Rect,
    arrangement: ClusterArrangement,
    n: usize,
    palette: &[Color; 5],
    rng: &mut StdRng,
) -> Vec<Layer> {
    let mut layers = Vec::new();
    let cx = rect.x as f32 + rect.w as f32 * 0.5;
    let cy = rect.y as f32 + rect.h as f32 * 0.5;

    // Cell size depends on node size and count
    let cell_w = (rect.w as f32 / (n as f32).sqrt().ceil().max(2.0)) as usize;
    let cell_h = (rect.h as f32 / (n as f32).sqrt().ceil().max(2.0)) as usize;

    for i in 0..n {
        let (ox, oy) = cluster_offset(arrangement, i, n, rect.w, rect.h);
        let ex = (cx as i32 + ox).clamp(rect.x as i32, (rect.x + rect.w) as i32 - 1) as usize;
        let ey = (cy as i32 + oy).clamp(rect.y as i32, (rect.y + rect.h) as i32 - 1) as usize;

        let mut pal = *palette;
        pal[1] = palette[rng.random_range(1..4)];

        let (fill, ew, eh) = match rng.random_range(0..8u32) {
            0..=2 => {
                let mut tp = TileParams::randomized(rng);
                tp.skew = rng.random_range(15..50);
                (FillGen::Tile(tp), cell_w, cell_h)
            }
            3 => (FillGen::Flower(rng.random_range(0..5)), 5, 5),
            4 => (FillGen::Fruit(rng.random_range(0..5)), 5, 5),
            5 => {
                let s = rng.random_range(2..4);
                (
                    FillGen::Mask(s, rng.random_range(0..MASK_STYLE_COUNT)),
                    s * 4 + 4,
                    s * 4 + 4,
                )
            }
            _ => (
                FillGen::Noise(noise_variant_from_index(
                    rng.random_range(0..NOISE_VARIANT_COUNT),
                )),
                cell_w,
                cell_h,
            ),
        };

        let elx = ex
            .saturating_sub(ew / 2)
            .min(rect.x + rect.w - ew.min(rect.w));
        let ely = ey
            .saturating_sub(eh / 2)
            .min(rect.y + rect.h - eh.min(rect.h));

        let ecx = elx as f32 + ew as f32 * 0.5;
        let ecy = ely as f32 + eh as f32 * 0.5;
        let mask: MaskFn = if fill_breaks_out(&fill) {
            let el_rect = Rect {
                x: elx,
                y: ely,
                w: ew.min(rect.w),
                h: eh.min(rect.h),
            };
            Box::new(mask_rect(&el_rect, 0.0))
        } else {
            pick_element_mask(ecx, ecy, ew as f32, eh as f32, rng)
        };

        layers.push(Layer {
            fill,
            mask: Some(mask),
            palette: pal,
        });
    }

    layers
}
/// Negative space: mostly empty box with one recognizable organic sprite.
/// No pattern fills. Just a tree, flower, or fruit sitting in open space.
pub(crate) fn make_negative_space(rect: &Rect, palette: &[Color; 5], rng: &mut StdRng) -> Vec<Layer> {
    let mut layers = Vec::new();
    let cx = rect.x as f32 + rect.w as f32 * 0.5;
    let cy = rect.y as f32 + rect.h as f32 * 0.5;

    let mut pal = *palette;
    pal[1] = palette[rng.random_range(1..4)];

    // One organic sprite, sized to fit comfortably inside the box
    match rng.random_range(0..5u32) {
        0..=2 => {
            // Tree -- the most readable thing we have
            let tw = rect.w.min(24).max(8);
            let th = rect.h.min(18).max(6);
            let gx = (cx as usize)
                .saturating_sub(tw / 2)
                .min(rect.x + rect.w - tw.min(rect.w));
            let gy = (cy as usize)
                .saturating_sub(th / 2)
                .min(rect.y + rect.h - th.min(rect.h));
            let el_rect = Rect {
                x: gx,
                y: gy,
                w: tw.min(rect.w),
                h: th.min(rect.h),
            };
            layers.push(Layer {
                fill: FillGen::Tree(rng.random_range(0..12)),
                mask: Some(Box::new(mask_rect(&el_rect, 0.0))),
                palette: pal,
            });
        }
        3 => {
            // Flower cluster -- 2-3 flowers spaced apart
            let count = rng.random_range(2..4u32).min(rect.w as u32 / 6).max(1);
            for i in 0..count {
                let fx =
                    rect.x + (rect.w as f32 * (i as f32 + 1.0) / (count as f32 + 1.0)) as usize;
                let fy = rect.y + rect.h / 2 + rng.random_range(0..3) as usize;
                let fy = fy.min(rect.y + rect.h - 5);
                let fx = fx.min(rect.x + rect.w - 5);
                let el_rect = Rect {
                    x: fx,
                    y: fy,
                    w: 5.min(rect.w),
                    h: 5.min(rect.h),
                };
                layers.push(Layer {
                    fill: FillGen::Flower(rng.random_range(0..5)),
                    mask: Some(Box::new(mask_rect(&el_rect, 0.0))),
                    palette: pal,
                });
            }
        }
        _ => {
            // Fruit cluster
            let count = rng.random_range(2..4u32).min(rect.w as u32 / 6).max(1);
            for i in 0..count {
                let fx =
                    rect.x + (rect.w as f32 * (i as f32 + 1.0) / (count as f32 + 1.0)) as usize;
                let fy = rect.y + rect.h / 2 + rng.random_range(0..3) as usize;
                let fy = fy.min(rect.y + rect.h - 5);
                let fx = fx.min(rect.x + rect.w - 5);
                let el_rect = Rect {
                    x: fx,
                    y: fy,
                    w: 5.min(rect.w),
                    h: 5.min(rect.h),
                };
                layers.push(Layer {
                    fill: FillGen::Fruit(rng.random_range(0..5)),
                    mask: Some(Box::new(mask_rect(&el_rect, 0.0))),
                    palette: pal,
                });
            }
        }
    }

    layers
}
/// Position offset for cluster element i of n within a bounding box of (w, h).
pub(crate) fn cluster_offset(
    arrangement: ClusterArrangement,
    i: usize,
    n: usize,
    w: usize,
    h: usize,
) -> (i32, i32) {
    match arrangement {
        ClusterArrangement::Ring => {
            let angle = (i as f32 / n as f32) * std::f32::consts::TAU;
            let rx = w as f32 * 0.3;
            let ry = h as f32 * 0.3;
            ((angle.cos() * rx) as i32, (angle.sin() * ry) as i32)
        }
        ClusterArrangement::Hex => {
            // Hex grid: offset every other row
            let cols = (n as f32).sqrt().ceil() as usize;
            let col = i % cols;
            let row = i / cols;
            let spacing_x = w as i32 / (cols as i32 + 1);
            let spacing_y = h as i32 / ((n / cols + 1) as i32 + 1);
            let offset = if row % 2 == 1 { spacing_x / 2 } else { 0 };
            let ox = (col as i32 + 1) * spacing_x - w as i32 / 2 + offset;
            let oy = (row as i32 + 1) * spacing_y - h as i32 / 2;
            (ox, oy)
        }
        ClusterArrangement::Grid => {
            let cols = (n as f32).sqrt().ceil() as usize;
            let col = i % cols;
            let row = i / cols;
            let spacing_x = w as i32 / (cols as i32 + 1);
            let spacing_y = h as i32 / ((n / cols + 1) as i32 + 1);
            let ox = (col as i32 + 1) * spacing_x - w as i32 / 2;
            let oy = (row as i32 + 1) * spacing_y - h as i32 / 2;
            (ox, oy)
        }
        ClusterArrangement::Loose => {
            // Spiral-ish scatter
            let angle = i as f32 * 2.4; // golden angle
            let r = (i as f32 + 1.0).sqrt() * (w.min(h) as f32 * 0.15);
            ((angle.cos() * r * 1.8) as i32, (angle.sin() * r) as i32)
        }
    }
}
/// Soup walk: overlapping node scenes along a wandering path.
/// Visually dense, nodes blend together -- no gap enforcement.
pub fn soup_walk(
    w: usize,
    h: usize,
    palette: &[Color; 5],
    rng: &mut StdRng,
) -> (Vec<Layer>, Vec<(usize, usize)>) {
    let character = PlantCharacter::random(rng);
    let mut layers = Vec::new();

    let node_count = rng
        .random_range(character.branch_factor.0 as u32..=character.branch_factor.1 as u32 + 2)
        as usize;
    let margin = 6usize;

    let mut stops = Vec::with_capacity(node_count);
    let mut px = rng.random_range(w / 5..w * 4 / 5);
    let mut py = rng.random_range(h / 5..h * 4 / 5);
    stops.push((px, py));

    for _ in 1..node_count {
        let base_angle: f32 = match character.taper_dir {
            TaperDir::Up => -std::f32::consts::FRAC_PI_2,
            TaperDir::Down => std::f32::consts::FRAC_PI_2,
            TaperDir::Left => std::f32::consts::PI,
            TaperDir::Right => 0.0,
            TaperDir::None => rng.random::<f32>() * std::f32::consts::TAU,
        };
        let jitter = (rng.random::<f32>() - 0.5) * character.angle_jitter * std::f32::consts::TAU;
        let angle = base_angle + jitter;
        let min_step = (w.min(h) / 5).max(10);
        let max_step = (w.min(h) / 3).max(min_step + 5);
        let dist = rng.random_range(min_step..max_step) as f32;
        px = (px as f32 + angle.cos() * dist * 1.8).clamp(margin as f32, (w - margin) as f32)
            as usize;
        py = (py as f32 + angle.sin() * dist).clamp(margin as f32, (h - margin) as f32) as usize;
        stops.push((px, py));
    }

    for (i, &(sx, sy)) in stops.iter().enumerate() {
        let t = if node_count > 1 {
            i as f32 / (node_count - 1) as f32
        } else {
            0.5
        };
        let sf = character.size_factor(t);
        let base_w = rng.random_range((w / 5).max(16)..(w / 3).max(20));
        let base_h = rng.random_range((h / 5).max(10)..(h / 3).max(14));
        let nw = (base_w as f32 * sf) as usize;
        let nh = (base_h as f32 * sf) as usize;
        let nx = sx.saturating_sub(nw / 2).min(w.saturating_sub(nw + margin));
        let ny = sy.saturating_sub(nh / 2).min(h.saturating_sub(nh + margin));
        let node_rect = Rect {
            x: nx,
            y: ny,
            w: nw.min(w - nx),
            h: nh.min(h - ny),
        };
        let mode = NodeMode::pick(character.landscape_bias, rng);
        let arc_shift = (i as f32 / node_count as f32 * 60.0) as u8;
        let mut node_pal = *palette;
        node_pal[1] = shift_hue(palette[1], arc_shift);
        node_pal[2] = shift_hue(palette[2], arc_shift);
        layers.extend(make_node_scene(&node_rect, mode, &node_pal, 50, rng));
    }

    (layers, stops)
}
