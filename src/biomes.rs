use crossterm::style::Color;
use rand::rngs::StdRng;
use rand::RngExt;
use crate::types::*;
use crate::color::*;
use crate::fills::*;
use crate::sprites::*;
use crate::scene::*;
use crate::walker::*;
use crate::sprites::draw_fret_border;
use crate::layout::*;

/// Biome types that can fill a vertical strip or arbitrary rect.
#[derive(Clone, Copy, Debug)]
pub enum Biome {
    Forest,
    Garden,
    Temple,
    Noise,
    Geometric,
    Flow,
    Terrain,
}

pub const BIOME_COUNT: usize = 7;

pub fn biome_from_index(i: usize) -> Biome {
    match i % BIOME_COUNT {
        0 => Biome::Forest,
        1 => Biome::Garden,
        2 => Biome::Temple,
        3 => Biome::Noise,
        4 => Biome::Geometric,
        5 => Biome::Flow,
        _ => Biome::Terrain,
    }
}

/// Pick a random biome, seed-driven.
pub fn random_biome(rng: &mut StdRng) -> Biome {
    biome_from_index(rng.random_range(0..BIOME_COUNT))
}

// ── Forest biome ────────────────────────────────────────────────────
// Truchet background, 1-3 trees scaled to rect, scattered fruits/flowers.

pub fn render_forest(grid: &mut Grid, rect: &Rect, palette: &[Color; 5], rng: &mut StdRng) {
    // background: truchet noise
    let bg_color = darken(palette[1], 90);
    fill_noise(grid, rect, NoiseVariant::Truchet, bg_color, bg_color, rng);

    let ground_y = rect.y + rect.h.saturating_sub(3);

    // how many trees fit? roughly one per 20 columns, at least 1
    let tree_count = (rect.w / 20).max(1).min(4);
    let tree_spacing = rect.w / (tree_count + 1);

    for i in 0..tree_count {
        let tx = rect.x + tree_spacing * (i + 1);
        let canopy_y = rect.y + 2;
        let spread = (tree_spacing / 3).max(3);
        let tree_type = rng.random_range(0..4);
        let color = palette[(i % 3) + 1];

        match tree_type {
            0 => grow_tree(grid, tx, ground_y, canopy_y, spread, color, rng),
            1 => {
                let tiers = (rect.h / 8).max(2).min(4);
                let base_w = (spread * 2).min(12);
                draw_pine(grid, tx, ground_y, tiers, base_w, color);
            }
            2 => draw_willow(grid, tx, ground_y, canopy_y, spread, color),
            _ => {
                let trunk_h = rect.h.saturating_sub(6);
                draw_palm(grid, tx, ground_y, trunk_h, color, rng);
            }
        }
    }

    // scatter fruits and flowers along the ground
    let decoration_count = (rect.w / 10).max(1);
    for _ in 0..decoration_count {
        let fx = rect.x + rng.random_range(2..rect.w.saturating_sub(2).max(3));
        let fy = rect.y + rng.random_range(rect.h / 3..rect.h.saturating_sub(2).max(rect.h / 3 + 1));
        match rng.random_range(0..6) {
            0 => draw_fruit(grid, fx, fy, rng.random_range(0..5), palette[3]),
            1 => {
                let s = (rect.w.min(rect.h) / 6).max(1).min(3);
                draw_mask(grid, fx, fy, s, rng.random_range(0..MASK_STYLE_COUNT), palette[rng.random_range(1..4)]);
            }
            _ => draw_flower(grid, fx, fy, rng.random_range(0..5), palette[3]),
        }
    }
}

// ── Garden biome ────────────────────────────────────────────────────
// Tile pattern floor with flowers on top.

pub fn render_garden(grid: &mut Grid, rect: &Rect, palette: &[Color; 5], rng: &mut StdRng) {
    // pick a random tile pattern as the floor
    let variant = tile_variant_from_index(rng.random_range(0..TILE_VARIANT_COUNT));
    let c1 = palette[rng.random_range(1..4)];
    let c2 = darken(c1, 30);
    fill_tile_pure(grid, rect, variant, c1, c2);

    // scatter flowers
    let count = (rect.w * rect.h / 80).max(1).min(8);
    for _ in 0..count {
        if rect.w < 5 || rect.h < 3 { break; }
        let fx = rect.x + rng.random_range(2..rect.w.saturating_sub(2).max(3));
        let fy = rect.y + rng.random_range(2..rect.h.saturating_sub(2).max(3));
        let style = rng.random_range(0..5);
        draw_flower(grid, fx, fy, style, palette[rng.random_range(1..4)]);
    }
}

// ── Temple biome ────────────────────────────────────────────────────
// Line art background + fret border + optional content area.

pub fn render_temple(grid: &mut Grid, rect: &Rect, palette: &[Color; 5], rng: &mut StdRng) {
    // background: pick a line art fill
    let c1 = palette[rng.random_range(1..4)];
    let c2 = darken(c1, 30);
    match rng.random_range(0..5) {
        0 => draw_crosshatch(grid, rect, c1, c2),
        1 => draw_guilloche(grid, rect, c1, c2),
        2 => draw_weave(grid, rect, c1, lighten(c1, 30)),
        3 => draw_zigzag(grid, rect, c1, c2),
        _ => draw_diamond_lattice(grid, rect, c1, c2),
    }

    // fret border if rect is large enough
    if rect.w > 12 && rect.h > 8 {
        let band = (rect.w.min(rect.h) / 8).max(2).min(4);
        let border_color = palette[rng.random_range(1..4)];
        for edge in 0..4 {
            draw_fret_border(grid, rect.x, rect.y, rect.w, rect.h, band, edge, border_color);
        }
    }

    // optional centered fret spiral
    if rect.w > 20 && rect.h > 12 && rng.random_range(0..3) == 0 {
        let cx = rect.x + rect.w / 2;
        let cy = rect.y + rect.h / 2;
        let steps = (rect.w.min(rect.h) / 6).max(2).min(5);
        draw_stepped_fret(grid, cx as i32, cy as i32, steps, Dir::Right, palette[rng.random_range(1..4)]);
    }
}

// ── Noise biome ─────────────────────────────────────────────────────
// One or two noise fills layered.

pub fn render_noise(grid: &mut Grid, rect: &Rect, palette: &[Color; 5], rng: &mut StdRng) {
    let variant = noise_variant_from_index(rng.random_range(0..NOISE_VARIANT_COUNT));
    let c1 = palette[rng.random_range(1..4)];
    let c2 = darken(c1, 30);
    fill_noise(grid, rect, variant, c1, c2, rng);

    // 50% chance: overlay a second sparse noise on top
    if rng.random_range(0..2) == 0 {
        let overlay = noise_variant_from_index(rng.random_range(0..NOISE_VARIANT_COUNT));
        let oc = palette[rng.random_range(1..4)];
        fill_noise(grid, rect, overlay, oc, darken(oc, 30), rng);
    }
}

// ── Geometric biome ─────────────────────────────────────────────────
// BSP subdivision within the rect, walker fills each leaf.

pub fn render_geometric(grid: &mut Grid, rect: &Rect, palette: &[Color; 5], rng: &mut StdRng) {
    // background
    let bg_color = darken(palette[1], 80);
    fill_noise(grid, rect, NoiseVariant::Truchet, bg_color, bg_color, rng);

    // subdivide the rect
    let min_w = 8.min(rect.w);
    let min_h = 4.min(rect.h);
    let depth = if rect.w * rect.h > 500 { 3 } else { 2 };
    let mut root = BspNode::new(rect.x, rect.y, rect.w, rect.h);
    root.split_with_gap(min_w, min_h, depth, 1, rng);
    let leaves: Vec<Rect> = root.leaves().into_iter().copied().collect();

    walk_and_fill_leaves(grid, &leaves, palette, rng);
}

// ── Dispatch ────────────────────────────────────────────────────────

pub fn render_biome(biome: Biome, grid: &mut Grid, rect: &Rect, palette: &[Color; 5], rng: &mut StdRng) {
    match biome {
        Biome::Forest => render_forest(grid, rect, palette, rng),
        Biome::Garden => render_garden(grid, rect, palette, rng),
        Biome::Temple => render_temple(grid, rect, palette, rng),
        Biome::Noise => render_noise(grid, rect, palette, rng),
        Biome::Geometric => render_geometric(grid, rect, palette, rng),
        Biome::Flow => {
            let zones = random_flow(rect, palette, rng);
            render_flow(grid, rect, &zones, palette, rng);
        }
        Biome::Terrain => render_terrain(grid, rect, palette, rng),
    }
}

// ── Flow system ─────────────────────────────────────────────────────
// A flow is a vertical sequence of zones. Each zone has a fill, a width
// taper (opening cone vs closing cone vs constant), and dissolve margins
// where it blends into the next zone via dot/space gradient.

/// Width envelope: how the active region narrows or widens across a zone.
#[derive(Clone, Copy)]
pub enum Taper {
    Constant,       // same width top to bottom
    Opening,        // narrow at top, wide at bottom (tree canopy, pyramid)
    Closing,        // wide at top, narrow at bottom (funnel, trunk)
    Diamond,        // narrow → wide → narrow
}

/// A single zone in a flow sequence.
#[derive(Clone, Copy)]
pub struct FlowZone {
    pub fill: FillGen,
    pub height_frac: f32,   // fraction of total strip height this zone occupies
    pub taper: Taper,
    pub width_start: f32,   // 0.0-1.0, fraction of strip width at zone top
    pub width_end: f32,     // 0.0-1.0, fraction of strip width at zone bottom
}

/// Compute the active x-range for a row within a zone, given taper.
fn zone_x_range(zone: &FlowZone, strip_x: usize, strip_w: usize, t: f32) -> (usize, usize) {
    let (w_start, w_end) = match zone.taper {
        Taper::Constant => (zone.width_start, zone.width_start),
        Taper::Opening => (zone.width_start, zone.width_end),
        Taper::Closing => (zone.width_start, zone.width_end),
        Taper::Diamond => {
            if t < 0.5 {
                let t2 = t * 2.0;
                (zone.width_start + (zone.width_end - zone.width_start) * t2, 0.0)
            } else {
                let t2 = (t - 0.5) * 2.0;
                (zone.width_end + (zone.width_start - zone.width_end) * t2, 0.0)
            }
        }
    };
    let w_frac = match zone.taper {
        Taper::Diamond => w_start, // already computed
        _ => w_start + (w_end - w_start) * t,
    };
    let active_w = (strip_w as f32 * w_frac).max(1.0) as usize;
    let margin = (strip_w.saturating_sub(active_w)) / 2;
    let x0 = strip_x + margin;
    let x1 = x0 + active_w;
    (x0, x1)
}

/// Draw dissolve glyphs in a horizontal band, fading from `density` to 0.
fn draw_dissolve_row(
    grid: &mut Grid, y: usize, x0: usize, x1: usize,
    density: f32, color: Color, rng: &mut StdRng,
) {
    if y >= grid.len() { return; }
    for x in x0..x1 {
        if x >= grid[0].len() { return; }
        let r: f32 = rng.random::<f32>();
        if r < density * 0.3 {
            let gi = rng.random_range(0..4); // denser dissolve glyphs
            let ch = DISSOLVE[gi];
            grid[y][x] = Cell::new(ch, darken(color, 20));
        } else if r < density * 0.6 {
            let ch = DISSOLVE[rng.random_range(3..6)]; // sparser dots
            grid[y][x] = Cell::new(ch, darken(color, 40));
        }
    }
}

/// Render a flow: a vertical sequence of zones within a strip rect.
pub fn render_flow(
    grid: &mut Grid, rect: &Rect, zones: &[FlowZone],
    palette: &[Color; 5], rng: &mut StdRng,
) {
    let dissolve_rows = 3; // rows of dissolve between zones
    let mut y_cursor = rect.y;

    for (zi, zone) in zones.iter().enumerate() {
        let zone_h = (rect.h as f32 * zone.height_frac) as usize;
        if zone_h == 0 || y_cursor >= rect.y + rect.h { continue; }
        let zone_h = zone_h.min(rect.y + rect.h - y_cursor);

        // for each row in this zone, compute the active x range and fill
        // we fill a clipped rect per-zone, then mask out the taper
        let (x0_top, x1_top) = zone_x_range(zone, rect.x, rect.w, 0.0);
        let (x0_bot, x1_bot) = zone_x_range(zone, rect.x, rect.w, 1.0);
        let fill_x0 = x0_top.min(x0_bot);
        let fill_x1 = x1_top.max(x1_bot);
        let fill_rect = Rect {
            x: fill_x0,
            y: y_cursor,
            w: fill_x1.saturating_sub(fill_x0),
            h: zone_h,
        };
        let c1 = palette[1];
        let c2 = darken(c1, 30);
        render_fill(grid, &fill_rect, zone.fill, c1, c2, palette, rng);

        // mask: clear cells outside the taper envelope
        for row_i in 0..zone_h {
            let y = y_cursor + row_i;
            if y >= grid.len() { break; }
            let t = if zone_h > 1 { row_i as f32 / (zone_h - 1) as f32 } else { 0.5 };
            let (x0, x1) = zone_x_range(zone, rect.x, rect.w, t);

            // clear left of active zone
            for x in fill_x0..x0 {
                if x < grid[0].len() {
                    grid[y][x] = Cell::blank();
                }
            }
            // clear right of active zone
            for x in x1..fill_x1 {
                if x < grid[0].len() {
                    grid[y][x] = Cell::blank();
                }
            }

            // edge dissolve: soften the taper boundary
            let edge_fade = 2;
            for e in 0..edge_fade {
                let density = 1.0 - (e as f32 / edge_fade as f32);
                let lx = x0.saturating_sub(e + 1);
                let rx = x1 + e;
                if lx < grid[0].len() && y < grid.len() {
                    if rng.random::<f32>() < density {
                        let ch = DISSOLVE[rng.random_range(3..6)];
                        grid[y][lx] = Cell::new(ch, darken(palette[1], 40));
                    }
                }
                if rx < grid[0].len() && y < grid.len() {
                    if rng.random::<f32>() < density {
                        let ch = DISSOLVE[rng.random_range(3..6)];
                        grid[y][rx] = Cell::new(ch, darken(palette[1], 40));
                    }
                }
            }
        }

        // dissolve transition into next zone
        if zi < zones.len() - 1 {
            let next_color = palette[rng.random_range(1..4)];
            for d in 0..dissolve_rows {
                let y = y_cursor + zone_h + d;
                let density = 1.0 - (d as f32 / dissolve_rows as f32);
                let (x0, x1) = zone_x_range(zone, rect.x, rect.w, 1.0);
                draw_dissolve_row(grid, y, x0, x1, density, next_color, rng);
            }
        }

        y_cursor += zone_h;
    }
}

/// Generate a random flow sequence for a strip.
pub fn random_flow(rect: &Rect, palette: &[Color; 5], rng: &mut StdRng) -> Vec<FlowZone> {
    let zone_count = rng.random_range(3..=5);
    let mut zones = Vec::with_capacity(zone_count);
    let mut remaining = 1.0f32;

    let line_art: FillGen = match rng.random_range(0..5) {
        0 => FillGen::Crosshatch,
        1 => FillGen::Guilloche,
        2 => FillGen::Weave,
        3 => FillGen::Zigzag,
        _ => FillGen::DiamondLattice,
    };
    let fills = [
        FillGen::TilePure(tile_variant_from_index(rng.random_range(0..TILE_VARIANT_COUNT))),
        FillGen::AztecDiamond((rect.h / 2).min(rect.w / 4).max(2).min(8)),
        FillGen::Tree(rng.random_range(0..12)),
        FillGen::Noise(noise_variant_from_index(rng.random_range(0..NOISE_VARIANT_COUNT))),
        line_art,
        FillGen::Noise(NoiseVariant::Dot),
    ];

    let tapers = [Taper::Constant, Taper::Opening, Taper::Closing, Taper::Diamond];

    // first zone: wide
    let mut prev_w_end = rng.random_range(60..100) as f32 / 100.0;

    for i in 0..zone_count {
        let frac = if i == zone_count - 1 {
            remaining
        } else {
            let f = rng.random_range(15..40) as f32 / 100.0;
            f.min(remaining - 0.1 * (zone_count - i - 1) as f32)
        };
        remaining -= frac;

        let fill = fills[rng.random_range(0..fills.len())];
        let taper = tapers[rng.random_range(0..tapers.len())];

        // width continuity: this zone starts where the last one ended
        let w_start = prev_w_end;
        let w_end = match taper {
            Taper::Constant => w_start,
            Taper::Opening => (w_start + rng.random_range(10..30) as f32 / 100.0).min(1.0),
            Taper::Closing => (w_start - rng.random_range(10..40) as f32 / 100.0).max(0.1),
            Taper::Diamond => w_start, // diamond returns to start width
        };
        prev_w_end = w_end;

        zones.push(FlowZone {
            fill,
            height_frac: frac,
            taper,
            width_start: w_start,
            width_end: w_end,
        });
    }

    zones
}

// ── Contour / terrain system ────────────────────────────────────────
// A contour is a 1D heightmap: one y-value per column. Fills render
// below (or above) the contour, giving organic non-rectangular boundaries.

/// Midpoint displacement with multiple anchor points for varied ridgelines.
/// Seeds 3-6 anchors across the width before running displacement, preventing
/// the single-peak pyramid shape that 2-endpoint displacement always produces.
pub fn gen_contour(width: usize, base: usize, amplitude: usize, roughness: f32, rng: &mut StdRng) -> Vec<usize> {
    if width == 0 { return vec![]; }
    if width == 1 { return vec![base]; }

    let mut heights = vec![0.0f32; width];
    let amp = amplitude as i32;
    let base_f = base as f32;

    // seed multiple anchors: endpoints + 2-4 interior points
    let anchor_count = rng.random_range(3..=6).min(width);
    let spacing = width / anchor_count;
    for a in 0..anchor_count {
        let idx = (a * spacing).min(width - 1);
        heights[idx] = base_f + rng.random_range(-amp..=amp) as f32;
    }
    heights[0] = base_f + rng.random_range(-amp / 2..=amp / 2) as f32;
    heights[width - 1] = base_f + rng.random_range(-amp / 2..=amp / 2) as f32;

    // linear interpolation between anchors to fill gaps
    let mut anchors: Vec<usize> = (0..anchor_count)
        .map(|a| (a * spacing).min(width - 1))
        .collect();
    anchors.push(width - 1);
    anchors.sort();
    anchors.dedup();
    for w in anchors.windows(2) {
        let (i0, i1) = (w[0], w[1]);
        if i1 <= i0 { continue; }
        let h0 = heights[i0];
        let h1 = heights[i1];
        for i in i0 + 1..i1 {
            let t = (i - i0) as f32 / (i1 - i0) as f32;
            heights[i] = h0 + (h1 - h0) * t;
        }
    }

    // midpoint displacement on top of the interpolated baseline
    let mut step = width.next_power_of_two() / 2;
    let mut scale = amplitude as f32 * 0.5; // reduced since anchors already provide structure
    while step > 1 {
        let half = step / 2;
        let mut i = half;
        while i < width {
            let left = if i >= half { heights[i - half] } else { heights[0] };
            let right = if i + half < width { heights[i + half] } else { heights[width - 1] };
            let mid = (left + right) / 2.0;
            let offset = (rng.random::<f32>() - 0.5) * scale;
            heights[i] = mid + offset;
            i += step;
        }
        scale *= roughness;
        step = half;
    }

    // smooth pass to reduce single-cell spikes
    let mut smoothed = heights.clone();
    for i in 1..width - 1 {
        smoothed[i] = (heights[i - 1] + heights[i] * 2.0 + heights[i + 1]) / 4.0;
    }

    smoothed.iter().map(|h| (*h).max(0.0) as usize).collect()
}

/// Draw the contour line itself with ridge glyphs.
pub fn draw_contour_ridge(
    grid: &mut Grid,
    rect: &Rect,
    contour: &[usize],
    color: Color,
) {
    for col in 0..rect.w.min(contour.len()) {
        let x = rect.x + col;
        let y = contour[col];
        if x >= grid[0].len() || y >= grid.len() { continue; }

        // pick glyph based on slope
        let prev = if col > 0 { contour[col - 1] } else { y };
        let next = if col + 1 < contour.len() { contour[col + 1] } else { y };
        let ch = if next < y && prev < y {
            '╰' // valley
        } else if next > y && prev > y {
            '╭' // peak
        } else if next < y || prev > y {
            '╱' // rising right
        } else if next > y || prev < y {
            '╲' // falling right
        } else {
            '─' // flat
        };
        grid[y][x] = Cell::new(ch, color);
    }
}

/// Contour data + scene layers for a terrain. Separated so the post-pass
/// can place sprites along the contour lines after the scene composites.
pub struct TerrainContext {
    pub scene: Scene,
    pub mountain_contour: Vec<usize>,
    pub foothill_contour: Vec<usize>,
    pub ground_contour: Vec<usize>,
}

/// Pick a random fill from a pool, weighted toward variety.
fn rand_mountain_fill(rng: &mut StdRng) -> FillGen {
    match rng.random_range(0..6) {
        0 => FillGen::Zigzag,
        1 => FillGen::DiamondLattice,
        2 => FillGen::Crosshatch,
        3 => FillGen::Weave,
        4 => FillGen::Noise(NoiseVariant::Truchet),
        _ => FillGen::Guilloche,
    }
}

fn rand_hill_fill(rng: &mut StdRng) -> FillGen {
    match rng.random_range(0..5) {
        0 => FillGen::TilePure(tile_variant_from_index(rng.random_range(0..TILE_VARIANT_COUNT))),
        1 => FillGen::Crosshatch,
        2 => FillGen::Noise(NoiseVariant::Higaki),
        3 => FillGen::Weave,
        _ => FillGen::TilePure(tile_variant_from_index(rng.random_range(0..TILE_VARIANT_COUNT))),
    }
}

fn rand_ground_fill(rng: &mut StdRng) -> FillGen {
    match rng.random_range(0..4) {
        0 => FillGen::Noise(NoiseVariant::Grass),
        1 => FillGen::Noise(NoiseVariant::Dot),
        2 => FillGen::TilePure(tile_variant_from_index(rng.random_range(0..TILE_VARIANT_COUNT))),
        _ => FillGen::Noise(NoiseVariant::Grass),
    }
}

/// Rotate a palette: shift slots 1-3 by `n` positions for visual variety.
fn rotate_palette(palette: &[Color; 5], n: usize) -> [Color; 5] {
    let inner = [palette[1], palette[2], palette[3]];
    [
        palette[0],
        inner[n % 3],
        inner[(n + 1) % 3],
        inner[(n + 2) % 3],
        palette[4],
    ]
}

/// Generate overlay patch layers within a contour band.
/// Each patch is an ellipse intersected with the band's contour mask,
/// filled with a different pattern and palette rotation.
fn terrain_patches(
    rect: &Rect,
    contour: &[usize],
    x_offset: usize,
    contour_dissolve: f32,
    below: bool,
    base_palette: &[Color; 5],
    fill_picker: fn(&mut StdRng) -> FillGen,
    darken_amount: u8,
    patch_count: usize,
    rng: &mut StdRng,
) -> Vec<Layer> {
    let w = rect.w;
    let h = rect.h;
    let mut patches = Vec::with_capacity(patch_count);

    for i in 0..patch_count {
        let cx = rect.x as f32 + rng.random_range(w / 6..w * 5 / 6) as f32;
        let cy = rect.y as f32 + rng.random_range(h / 4..h * 3 / 4) as f32;
        let rx = rng.random_range((w / 6).max(5)..(w / 3).max(6)) as f32;
        let ry = rng.random_range((h / 6).max(3)..(h / 3).max(4)) as f32;

        let contour_clone = contour.to_vec();
        let contour_mask: MaskFn = if below {
            Box::new(mask_below_contour(contour_clone, x_offset, contour_dissolve))
        } else {
            Box::new(mask_above_contour(contour_clone, x_offset, contour_dissolve))
        };
        let ellipse = mask_ellipse(cx, cy, rx, ry, 3.0);
        let combined = mask_intersect(contour_mask, ellipse);

        let mut patch_palette = rotate_palette(base_palette, i + 1);
        if darken_amount > 0 {
            patch_palette[1] = darken(patch_palette[1], darken_amount);
            patch_palette[2] = darken(patch_palette[2], darken_amount);
            patch_palette[3] = darken(patch_palette[3], darken_amount);
        }

        patches.push(Layer {
            fill: fill_picker(rng),
            mask: Some(combined),
            palette: patch_palette,
        });
    }
    patches
}

/// Build terrain as compositable layers with patchy variation within each band.
/// Base layers (sky, mountains, foothills, ground) + overlay patches per band.
pub fn terrain_scene(rect: &Rect, palette: &[Color; 5], rng: &mut StdRng) -> TerrainContext {
    let w = rect.w;
    let h = rect.h;

    let mountain_base = rect.y + h / 2;
    let foothill_base = rect.y + h * 7 / 10;
    let ground_base = rect.y + h * 17 / 20;

    let mountain_contour = gen_contour(w, mountain_base, h / 3, 0.6, rng);
    let foothill_contour = gen_contour(w, foothill_base, h / 5, 0.5, rng);
    let ground_contour = gen_contour(w, ground_base, h / 8, 0.4, rng);

    let mtn_palette = [
        palette[0],
        darken(palette[1], 50),
        darken(palette[2], 50),
        darken(palette[3], 50),
        palette[4],
    ];
    let hill_palette = [palette[0], palette[2], palette[3], palette[1], palette[4]];
    let gnd_palette = [palette[0], palette[3], palette[1], palette[2], palette[4]];

    let mut layers = vec![
        // sky base
        Layer {
            fill: FillGen::Noise(NoiseVariant::Dot),
            mask: Some(Box::new(mask_above_contour(mountain_contour.clone(), rect.x, 5.0))),
            palette: *palette,
        },
        // mountain base
        Layer {
            fill: rand_mountain_fill(rng),
            mask: Some(Box::new(mask_below_contour(mountain_contour.clone(), rect.x, 4.0))),
            palette: mtn_palette,
        },
    ];

    // mountain overlay patches
    let mtn_patches = rng.random_range(2..=3);
    layers.extend(terrain_patches(
        rect, &mountain_contour, rect.x, 4.0, true,
        &mtn_palette, rand_mountain_fill, 20, mtn_patches, rng,
    ));

    // foothill base
    layers.push(Layer {
        fill: rand_hill_fill(rng),
        mask: Some(Box::new(mask_below_contour(foothill_contour.clone(), rect.x, 4.0))),
        palette: hill_palette,
    });

    // foothill overlay patches
    let hill_patches = rng.random_range(1..=3);
    layers.extend(terrain_patches(
        rect, &foothill_contour, rect.x, 4.0, true,
        &hill_palette, rand_hill_fill, 0, hill_patches, rng,
    ));

    // ground base
    layers.push(Layer {
        fill: rand_ground_fill(rng),
        mask: Some(Box::new(mask_below_contour(ground_contour.clone(), rect.x, 3.0))),
        palette: gnd_palette,
    });

    // ground overlay patches
    let gnd_patches = rng.random_range(1..=2);
    layers.extend(terrain_patches(
        rect, &ground_contour, rect.x, 3.0, true,
        &gnd_palette, rand_ground_fill, 0, gnd_patches, rng,
    ));

    TerrainContext {
        scene: Scene { layers },
        mountain_contour,
        foothill_contour,
        ground_contour,
    }
}

/// Procedural post-pass: contour ridges, trees, flowers, moon.
/// These are imperative sprite placements that don't fit the fill+mask model.
pub fn terrain_post_pass(
    grid: &mut Grid,
    rect: &Rect,
    ctx: &TerrainContext,
    palette: &[Color; 5],
    rng: &mut StdRng,
) {
    let w = rect.w;
    let h = rect.h;

    draw_contour_ridge(grid, rect, &ctx.mountain_contour, lighten(palette[1], 30));
    draw_contour_ridge(grid, rect, &ctx.foothill_contour, palette[2]);
    draw_contour_ridge(grid, rect, &ctx.ground_contour, lighten(palette[3], 20));

    // scatter trees along the ground contour
    let tree_count = (w / 15).max(1).min(6);
    let tree_spacing = w / (tree_count + 1);
    for i in 0..tree_count {
        let col = tree_spacing * (i + 1);
        if col >= w || col >= ctx.ground_contour.len() { continue; }
        let tx = rect.x + col;
        let ty = ctx.ground_contour[col].saturating_sub(1);
        let canopy = ctx.foothill_contour.get(col).copied()
            .unwrap_or(ty.saturating_sub(5))
            .min(ty.saturating_sub(3));
        let spread = (tree_spacing / 3).max(2).min(6);
        let tree_color = palette[(i % 3) + 1];
        match rng.random_range(0..3) {
            0 => grow_tree(grid, tx, ty, canopy, spread, tree_color, rng),
            1 => draw_pine(grid, tx, ty, 2, spread * 2, tree_color),
            _ => draw_willow(grid, tx, ty, canopy, spread, tree_color),
        }
    }

    // scatter flowers along ground
    for _ in 0..(w / 20).max(1) {
        let col = rng.random_range(0..w);
        if col >= ctx.ground_contour.len() { continue; }
        let fx = rect.x + col;
        let fy = ctx.ground_contour[col] + rng.random_range(1..4);
        if fy < rect.y + h {
            draw_flower(grid, fx, fy, rng.random_range(0..5), palette[3]);
        }
    }

    // optional moon
    if w > 30 && h > 20 && rng.random_range(0..3) == 0 {
        let moon_x = rect.x + rng.random_range(w / 4..w * 3 / 4);
        let moon_y = rect.y + rng.random_range(2..h / 5);
        let moon_mask = mask_ellipse(moon_x as f32, moon_y as f32, 4.0, 2.5, 1.5);
        let moon_rect = Rect {
            x: moon_x.saturating_sub(6),
            y: moon_y.saturating_sub(4),
            w: 12,
            h: 8,
        };
        fill_masked(grid, &moon_rect, FillGen::TilePure(TileVariant::Shippo), &moon_mask, palette, rng);
    }
}

/// Terrain biome: layered contour fills creating a landscape.
/// Builds a 4-layer scene, composites it, then runs procedural post-pass.
pub fn render_terrain(grid: &mut Grid, rect: &Rect, palette: &[Color; 5], rng: &mut StdRng) {
    let ctx = terrain_scene(rect, palette, rng);
    render_scene(grid, rect, &ctx.scene, rng);
    terrain_post_pass(grid, rect, &ctx, palette, rng);
}

// ── Strip allocator ─────────────────────────────────────────────────
// Divides the grid into vertical strips of random width, assigns a biome to each.

pub fn allocate_strips(width: usize, min_strip: usize, max_strip: usize, rng: &mut StdRng) -> Vec<(usize, usize)> {
    let mut strips = Vec::new(); // (x, w)
    let mut x = 0;
    while x < width {
        let remaining = width - x;
        if remaining <= max_strip {
            strips.push((x, remaining));
            break;
        }
        let w = rng.random_range(min_strip..=max_strip.min(remaining));
        strips.push((x, w));
        x += w;
    }
    strips
}

/// Render the full grid as a world of vertical biome strips.
pub fn render_world(grid: &mut Grid, width: usize, height: usize, palette: &[Color; 5], rng: &mut StdRng) {
    let min_strip = 15.min(width);
    let max_strip = 40.min(width);
    let strips = allocate_strips(width, min_strip, max_strip, rng);

    for (x, w) in &strips {
        let biome = random_biome(rng);
        let rect = Rect { x: *x, y: 0, w: *w, h: height };
        render_biome(biome, grid, &rect, palette, rng);
    }
}
