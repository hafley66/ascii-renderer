#![allow(unused)]
use crossterm::style::Color;
use rand::rngs::StdRng;
use rand::RngExt;
use crate::types::*;
use crate::color::*;
use crate::fills::*;
use crate::sprites::*;
use crate::scene::*;

// ── Plant character + node modes ────────────────────────────────────

/// Which direction a walk tapers (nodes shrink toward this end).
#[derive(Clone, Copy)]
pub enum TaperDir { Up, Down, Left, Right, None }

/// Global personality governing a walk's layout and flavor.
pub struct PlantCharacter {
    /// How many children branch from each node (range).
    pub branch_factor: (usize, usize),
    /// Direction nodes shrink toward.
    pub taper_dir: TaperDir,
    /// 0.0 = no taper, 1.0 = aggressive shrink.
    pub taper_strength: f32,
    /// 0.0 = all Composition, 1.0 = all Landscape.
    pub landscape_bias: f32,
    /// Walk angle jitter range in radians.
    pub angle_jitter: f32,
}

impl PlantCharacter {
    pub fn random(rng: &mut StdRng) -> Self {
        let taper_dir = match rng.random_range(0..5u32) {
            0 => TaperDir::Up,
            1 => TaperDir::Down,
            2 => TaperDir::Left,
            3 => TaperDir::Right,
            _ => TaperDir::None,
        };
        PlantCharacter {
            branch_factor: (rng.random_range(1..3), rng.random_range(3..6)),
            taper_dir,
            taper_strength: rng.random_range(20..80) as f32 / 100.0,
            landscape_bias: rng.random_range(30..80) as f32 / 100.0,
            angle_jitter: rng.random_range(20..80) as f32 / 100.0,
        }
    }

    /// Size multiplier for a node at normalized walk position t (0..1).
    pub fn size_factor(&self, t: f32) -> f32 {
        let raw = match self.taper_dir {
            TaperDir::Up | TaperDir::Right => 1.0 - t * self.taper_strength,
            TaperDir::Down | TaperDir::Left => t * self.taper_strength + (1.0 - self.taper_strength),
            TaperDir::None => 1.0,
        };
        raw.clamp(0.3, 1.0)
    }
}

/// What kind of scene a node renders.
#[derive(Clone, Copy)]
pub enum NodeMode {
    /// 3-pass: background (sky/rain/stars) + ground band + foreground elements.
    Landscape,
    /// Single centerpiece with surrounding pattern fill.
    CenterpieceWithSurround,
    /// N related patterns in a spatial arrangement.
    Cluster(ClusterArrangement, usize),
}

#[derive(Clone, Copy)]
pub enum ClusterArrangement { Hex, Ring, Loose, Grid }

impl NodeMode {
    pub fn pick(landscape_bias: f32, rng: &mut StdRng) -> Self {
        if rng.random::<f32>() < landscape_bias {
            NodeMode::Landscape
        } else {
            match rng.random_range(0..3u32) {
                0 => NodeMode::CenterpieceWithSurround,
                _ => {
                    let arr = match rng.random_range(0..4u32) {
                        0 => ClusterArrangement::Hex,
                        1 => ClusterArrangement::Ring,
                        2 => ClusterArrangement::Loose,
                        _ => ClusterArrangement::Grid,
                    };
                    NodeMode::Cluster(arr, rng.random_range(3..7) as usize)
                }
            }
        }
    }
}

// ── Walker mood ────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub enum WalkerMood { Organic, Geometric, Empty }

pub struct LeafWalker {
    pub mood: WalkerMood,
    pub energy: f32,
    pub prev_x: usize,
    pub prev_y: usize,
}

impl LeafWalker {
    pub fn new(center_x: usize, center_y: usize) -> Self {
        LeafWalker { mood: WalkerMood::Organic, energy: 1.0, prev_x: center_x, prev_y: center_y }
    }

    pub fn pick_fill(&self, rect: &Rect, rng: &mut StdRng) -> FillGen {
        let area = rect.w * rect.h;

        // energy-based thinning
        if self.energy < 0.5 && rng.random_range(0..10) < 3 {
            return FillGen::Nothing;
        }

        match self.mood {
            WalkerMood::Empty => FillGen::Nothing,
            WalkerMood::Organic => {
                if area > 300 && rect.h > 15 && rect.w > 20 {
                    match rng.random_range(0..7) {
                        0..=2 => FillGen::Tree(rng.random_range(0..12)),
                        3 => FillGen::Noise(NoiseVariant::Grass),
                        4 => FillGen::Noise(NoiseVariant::Dot),
                        _ => {
                            let s = (rect.w.min(rect.h) / 6).max(2).min(4);
                            FillGen::Mask(s, rng.random_range(0..MASK_STYLE_COUNT))
                        }
                    }
                } else if area > 80 && rect.h > 6 && rect.w > 10 {
                    match rng.random_range(0..7) {
                        0 => FillGen::Fruit(rng.random_range(0..5)),
                        1 => FillGen::Noise(NoiseVariant::Grass),
                        2 => {
                            let s = (rect.w.min(rect.h) / 4).max(1).min(3);
                            FillGen::Mask(s, rng.random_range(0..MASK_STYLE_COUNT))
                        }
                        _ => FillGen::Flower(rng.random_range(0..5)),
                    }
                } else if area > 20 && rect.w >= 5 && rect.h >= 3 {
                    match rng.random_range(0..4) {
                        0 => FillGen::Flower(rng.random_range(0..5)),
                        1 => FillGen::Fruit(rng.random_range(0..5)),
                        _ => FillGen::Noise(NoiseVariant::Dot),
                    }
                } else {
                    FillGen::Nothing
                }
            }
            WalkerMood::Geometric => {
                if area > 300 && rect.h > 15 && rect.w > 20 {
                    match rng.random_range(0..16) {
                        0 => { let order = (rect.h / 2).min(rect.w / 4).max(2).min(6); FillGen::AztecDiamond(order) }
                        1 => FillGen::Guilloche,
                        2 => FillGen::Weave,
                        3 => FillGen::DiamondLattice,
                        4 => FillGen::Crosshatch,
                        5 => FillGen::Noise(noise_variant_from_index(rng.random_range(0..NOISE_VARIANT_COUNT))),
                        6 => FillGen::Spiral,
                        7 => FillGen::Concentric,
                        8 => FillGen::Labyrinth,
                        9 => FillGen::CaSnapshot(rng.random_range(0..4) as u8),
                        10 => FillGen::Explosion,
                        11 => FillGen::Rule1D([30, 90, 110, 150][rng.random_range(0..4)]),
                        _ => FillGen::Tile(TileParams::randomized(rng)),
                    }
                } else if area > 80 && rect.h > 6 && rect.w > 10 {
                    match rng.random_range(0..15) {
                        0 => { let steps = (rect.w.min(rect.h) / 3).max(2).min(5); FillGen::Fret(steps) }
                        1 => { let order = (rect.h / 2).min(rect.w / 4).max(2).min(6); FillGen::AztecDiamond(order) }
                        2 => FillGen::Crosshatch,
                        3 => FillGen::Guilloche,
                        4 => FillGen::Weave,
                        5 => FillGen::Zigzag,
                        6 => FillGen::DiamondLattice,
                        7 => FillGen::Noise(noise_variant_from_index(rng.random_range(0..NOISE_VARIANT_COUNT))),
                        8 => FillGen::CaSnapshot(rng.random_range(0..4) as u8),
                        9 => FillGen::Explosion,
                        10 => FillGen::Rule1D([30, 90, 110][rng.random_range(0..3)]),
                        _ => FillGen::Tile(TileParams::randomized(rng)),
                    }
                } else if area > 20 && rect.w >= 5 && rect.h >= 3 {
                    match rng.random_range(0..8) {
                        0 => FillGen::Flower(rng.random_range(3..5)),
                        1 => FillGen::Crosshatch,
                        2 => FillGen::Zigzag,
                        3 => FillGen::DiamondLattice,
                        4 => FillGen::Noise(noise_variant_from_index(rng.random_range(0..NOISE_VARIANT_COUNT))),
                        _ => FillGen::Tile(TileParams::randomized(rng)),
                    }
                } else {
                    FillGen::Nothing
                }
            }
        }
    }

    pub fn step(&mut self, rect: &Rect, rng: &mut StdRng) {
        self.prev_x = rect.x + rect.w / 2;
        self.prev_y = rect.y + rect.h / 2;
        self.energy -= rng.random_range(15..26) as f32 / 100.0;

        if matches!(self.mood, WalkerMood::Empty) {
            self.energy = 0.6;
            self.mood = if rng.random_range(0..2) == 0 { WalkerMood::Organic } else { WalkerMood::Geometric };
        } else if self.energy < 0.3 {
            self.mood = match self.mood {
                WalkerMood::Organic => {
                    if rng.random_range(0..10) < 7 { WalkerMood::Geometric } else { WalkerMood::Empty }
                }
                WalkerMood::Geometric => {
                    if rng.random_range(0..10) < 5 { WalkerMood::Organic } else { WalkerMood::Empty }
                }
                WalkerMood::Empty => unreachable!(),
            };
        }
    }
}

/// Walk through leaf rects in nearest-neighbor order, producing layers
/// instead of writing to a grid. Each leaf becomes a masked layer.
/// Tree leaves get an extra scatter layer on top.
pub fn walk_to_layers(
    leaves: &[Rect],
    center: (usize, usize),
    palette: &[Color; 5],
    rng: &mut StdRng,
) -> Vec<Layer> {
    if leaves.is_empty() { return vec![]; }

    let mut walker = LeafWalker::new(center.0, center.1);
    let mut visited = vec![false; leaves.len()];
    let mut layers = Vec::with_capacity(leaves.len());

    for _ in 0..leaves.len() {
        let mut best_idx = None;
        let mut best_dist = usize::MAX;
        for (i, leaf) in leaves.iter().enumerate() {
            if visited[i] { continue; }
            let cx = leaf.x + leaf.w / 2;
            let cy = leaf.y + leaf.h / 2;
            let dist = cx.abs_diff(walker.prev_x) + cy.abs_diff(walker.prev_y);
            if dist < best_dist {
                best_dist = dist;
                best_idx = Some(i);
            }
        }

        let Some(idx) = best_idx else { break };
        visited[idx] = true;
        let rect = &leaves[idx];

        // Scene walker: force mood alternation every 2-3 leaves so we get
        // a mix of organic (trees/flowers/grass) and geometric (tiles/lineart).
        // Grid-walker drifts naturally but scene layers need variety per-leaf.
        walker.energy = walker.energy.max(0.6);
        if rng.random_range(0..3u32) == 0 {
            walker.mood = match walker.mood {
                WalkerMood::Organic => WalkerMood::Geometric,
                WalkerMood::Geometric => WalkerMood::Organic,
                WalkerMood::Empty => WalkerMood::Organic,
            };
        }

        let fill = walker.pick_fill(rect, rng);

        if matches!(fill, FillGen::Nothing) {
            walker.step(rect, rng);
            continue;
        }

        let mut pal = *palette;
        pal[1] = palette[rng.random_range(1..4)];

        let actual_fill = match fill {
            FillGen::Tile(params) => {
                FillGen::Tile(TileParams { jitter: (1.0 - walker.energy).max(0.0) * 0.15, ..params })
            }
            other => other,
        };

        // Mix ellipse and rect masks to break the BSP grid feel.
        // Sprites and organic fills get ellipses, geometric fills get rects.
        let mask: MaskFn = if matches!(walker.mood, WalkerMood::Organic) || rng.random_range(0..3u32) == 0 {
            let cx = rect.x as f32 + rect.w as f32 / 2.0;
            let cy = rect.y as f32 + rect.h as f32 / 2.0;
            let rx = rect.w as f32 / 2.0 + rng.random_range(0..4u32) as f32;
            let ry = rect.h as f32 / 2.0 + rng.random_range(0..3u32) as f32;
            Box::new(mask_ellipse(cx, cy, rx, ry, 2.0))
        } else {
            Box::new(mask_rect(rect, 0.0))
        };

        layers.push(Layer {
            fill: actual_fill,
            mask: Some(mask),
            palette: pal,
        });

        // Tree scatter: extra sprite layers on top
        if matches!(fill, FillGen::Tree(_)) {
            for _ in 0..rng.random_range(1..=3u32) {
                let fx = rect.x + rng.random_range(2..rect.w.saturating_sub(2).max(3));
                let fy = rect.y + rng.random_range(2..rect.h.saturating_sub(2).max(3));
                let sprite = if rng.random_range(0..2) == 0 {
                    FillGen::Fruit(rng.random_range(0..5))
                } else {
                    FillGen::Flower(rng.random_range(0..5))
                };
                let sprite_rect = Rect { x: fx, y: fy, w: 3, h: 3 };
                let mut spal = *palette;
                spal[1] = palette[3];
                layers.push(Layer {
                    fill: sprite,
                    mask: Some(Box::new(mask_rect(&sprite_rect, 0.0))),
                    palette: spal,
                });
            }
        }

        walker.step(rect, rng);
    }

    layers
}

/// Random-walk path across the canvas. At each waypoint, drop a scene element
/// (tree, face, pattern, flowers) with breathing room between stops.
/// The path itself is drawn as a subtle dot trail connecting waypoints.
pub fn path_walk_layers(
    w: usize,
    h: usize,
    palette: &[Color; 5],
    rng: &mut StdRng,
) -> Vec<Layer> {
    let mut layers = Vec::new();
    let mut walker = LeafWalker::new(
        rng.random_range(w / 6..w * 5 / 6),
        rng.random_range(h / 6..h * 5 / 6),
    );

    // How many stops along the path
    let stop_count = rng.random_range(4..8u32) as usize;
    let margin = 6usize;

    let mut stops: Vec<(usize, usize)> = Vec::with_capacity(stop_count);
    stops.push((walker.prev_x, walker.prev_y));

    // Generate waypoints by random walking with minimum spacing
    for _ in 1..stop_count {
        let min_step = (w.min(h) / 5).max(10);
        let max_step = (w.min(h) / 3).max(min_step + 5);
        let angle: f32 = rng.random::<f32>() * std::f32::consts::TAU;
        let dist = rng.random_range(min_step..max_step) as f32;
        let nx = (walker.prev_x as f32 + angle.cos() * dist * 1.8)
            .clamp(margin as f32, (w - margin) as f32) as usize;
        let ny = (walker.prev_y as f32 + angle.sin() * dist)
            .clamp(margin as f32, (h - margin) as f32) as usize;
        walker.prev_x = nx;
        walker.prev_y = ny;
        stops.push((nx, ny));
    }

    // Draw path trail between consecutive stops
    let trail_color = darken(palette[1], 60);
    for pair in stops.windows(2) {
        let (x0, y0) = pair[0];
        let (x1, y1) = pair[1];
        let steps = ((x1 as f32 - x0 as f32).abs().max((y1 as f32 - y0 as f32).abs())) as usize;
        if steps == 0 { continue; }
        // Trail as dot noise layers along the line, sparse
        let dot_count = (steps / 4).max(2).min(20);
        for i in 0..dot_count {
            let t = i as f32 / dot_count as f32;
            let tx = (x0 as f32 + (x1 as f32 - x0 as f32) * t) as usize;
            let ty = (y0 as f32 + (y1 as f32 - y0 as f32) * t) as usize;
            // Jitter the trail slightly
            let jx = (tx as i32 + rng.random_range(-1..=1i32)).clamp(0, w as i32 - 1) as usize;
            let jy = (ty as i32 + rng.random_range(-1..=1i32)).clamp(0, h as i32 - 1) as usize;
            let dot_rect = Rect { x: jx, y: jy, w: 2, h: 1 };
            layers.push(Layer {
                fill: FillGen::Noise(NoiseVariant::Dot),
                mask: Some(Box::new(mask_rect(&dot_rect, 0.0))),
                palette: [palette[0], trail_color, trail_color, trail_color, trail_color],
            });
        }
    }

    // Place a scene element at each stop
    for (i, &(sx, sy)) in stops.iter().enumerate() {
        let mood = if i % 2 == 0 { WalkerMood::Organic } else { WalkerMood::Geometric };
        walker.mood = mood;
        walker.energy = 0.9;

        let mut pal = *palette;
        pal[1] = palette[rng.random_range(1..4)];

        match rng.random_range(0..10u32) {
            0..=3 => {
                // Tree
                let tw = rng.random_range(16..(w / 4).max(17));
                let th = rng.random_range(12..(h / 3).max(13));
                let tx = sx.saturating_sub(tw / 2).min(w.saturating_sub(tw));
                let ty = sy.saturating_sub(th / 2).min(h.saturating_sub(th));
                let rect = Rect { x: tx, y: ty, w: tw, h: th };
                layers.push(Layer {
                    fill: FillGen::Tree(rng.random_range(0..12)),
                    mask: Some(Box::new(mask_rect(&rect, 0.0))),
                    palette: pal,
                });
            }
            4..=5 => {
                // Face
                let s = rng.random_range(3..6);
                let fw = s * 4 + 4;
                let fh = s * 4 + 4;
                let fx = sx.saturating_sub(fw / 2).min(w.saturating_sub(fw));
                let fy = sy.saturating_sub(fh / 2).min(h.saturating_sub(fh));
                let rect = Rect { x: fx, y: fy, w: fw, h: fh };
                layers.push(Layer {
                    fill: FillGen::Mask(s, rng.random_range(0..MASK_STYLE_COUNT)),
                    mask: Some(Box::new(mask_rect(&rect, 0.0))),
                    palette: pal,
                });
            }
            6 => {
                // Aztec diamond
                let order = rng.random_range(3..7);
                let aw = order * 4 + 4;
                let ah = order * 2 + 4;
                let ax = sx.saturating_sub(aw / 2).min(w.saturating_sub(aw));
                let ay = sy.saturating_sub(ah / 2).min(h.saturating_sub(ah));
                let rect = Rect { x: ax, y: ay, w: aw, h: ah };
                layers.push(Layer {
                    fill: FillGen::AztecDiamond(order),
                    mask: Some(Box::new(mask_rect(&rect, 0.0))),
                    palette: pal,
                });
            }
            7 => {
                // Fret spiral
                let steps = rng.random_range(3..6);
                let fw = steps * 4 + 2;
                let fh = steps * 4 + 2;
                let fx = sx.saturating_sub(fw / 2).min(w.saturating_sub(fw));
                let fy = sy.saturating_sub(fh / 2).min(h.saturating_sub(fh));
                let rect = Rect { x: fx, y: fy, w: fw, h: fh };
                layers.push(Layer {
                    fill: FillGen::Fret(steps),
                    mask: Some(Box::new(mask_rect(&rect, 0.0))),
                    palette: pal,
                });
            }
            8 => {
                // Flower cluster
                for _ in 0..rng.random_range(2..5u32) {
                    let fx = (sx as i32 + rng.random_range(-6..6i32)).clamp(2, w as i32 - 4) as usize;
                    let fy = (sy as i32 + rng.random_range(-4..4i32)).clamp(2, h as i32 - 4) as usize;
                    let rect = Rect { x: fx, y: fy, w: 5, h: 5 };
                    layers.push(Layer {
                        fill: FillGen::Flower(rng.random_range(0..5)),
                        mask: Some(Box::new(mask_rect(&rect, 0.0))),
                        palette: pal,
                    });
                }
            }
            _ => {
                // Tile patch with ellipse mask -- organic island
                let pw = rng.random_range(14..(w / 4).max(15));
                let ph = rng.random_range(8..(h / 4).max(9));
                let px = sx.saturating_sub(pw / 2).min(w.saturating_sub(pw));
                let py = sy.saturating_sub(ph / 2).min(h.saturating_sub(ph));
                let cx = px as f32 + pw as f32 / 2.0;
                let cy = py as f32 + ph as f32 / 2.0;
                let rect = Rect { x: px, y: py, w: pw, h: ph };
                let _ = rect; // rect only for sizing reference
                layers.push(Layer {
                    fill: FillGen::Tile(TileParams::randomized(rng)),
                    mask: Some(Box::new(mask_ellipse(cx, cy, pw as f32 / 2.0, ph as f32 / 2.0, 2.5))),
                    palette: pal,
                });
            }
        }
    }

    layers
}

/// Classify whether a fill type breaks out of its scene bounds or stays clipped.
/// Organic/geometric shapes that have natural edges (trees, flowers, diamonds,
/// frets) can exceed the scene ellipse. Rectangular fills (tiles, crosshatch,
/// noise) stay clipped inside.
fn fill_breaks_out(fill: &FillGen) -> bool {
    matches!(fill,
        FillGen::Tree(_) | FillGen::Flower(_) | FillGen::Fruit(_) |
        FillGen::AztecDiamond(_) | FillGen::Fret(_) | FillGen::Mask(_, _)
    )
}

/// Generate a multi-layer scene at a waypoint. Returns layers for one "world".
/// Bounded fills get the scene ellipse mask. Breakout fills get a larger rect.
fn waypoint_scene(
    sx: usize, sy: usize,
    scene_w: usize, scene_h: usize,
    w: usize, h: usize,
    palette: &[Color; 5],
    rng: &mut StdRng,
) -> Vec<Layer> {
    let mut layers = Vec::new();
    let cx = sx as f32;
    let cy = sy as f32;
    let rx = scene_w as f32 / 2.0;
    let ry = scene_h as f32 / 2.0;

    // Scene boundary rect (for render_fill dispatch)
    let bx = sx.saturating_sub(scene_w / 2).min(w.saturating_sub(scene_w));
    let by = sy.saturating_sub(scene_h / 2).min(h.saturating_sub(scene_h));
    let scene_rect = Rect { x: bx, y: by, w: scene_w.min(w - bx), h: scene_h.min(h - by) };

    // Pick 2-4 elements per scene
    let element_count = rng.random_range(2..5u32);

    // Optional: background fill inside the scene ellipse (tile or noise)
    if rng.random_range(0..3u32) > 0 {
        let bg_fill = match rng.random_range(0..4u32) {
            0 => FillGen::Tile(TileParams::randomized(rng)),
            1 => FillGen::Noise(NoiseVariant::Grass),
            2 => FillGen::Noise(NoiseVariant::Dot),
            _ => FillGen::Crosshatch,
        };
        let mut pal = *palette;
        pal[1] = darken(palette[rng.random_range(1..4)], 50);
        layers.push(Layer {
            fill: bg_fill,
            mask: Some(Box::new(mask_ellipse(cx, cy, rx, ry, 3.0))),
            palette: pal,
        });
    }

    for _ in 0..element_count {
        let mut pal = *palette;
        pal[1] = palette[rng.random_range(1..4)];

        // Offset from scene center
        let ox = rng.random_range(-(rx as i32 / 2)..=(rx as i32 / 2));
        let oy = rng.random_range(-(ry as i32 / 2)..=(ry as i32 / 2));
        let ex = (sx as i32 + ox).clamp(2, w as i32 - 4) as usize;
        let ey = (sy as i32 + oy).clamp(2, h as i32 - 4) as usize;

        let fill = match rng.random_range(0..12u32) {
            0..=3 => FillGen::Tree(rng.random_range(0..12)),
            4..=5 => {
                let s = rng.random_range(2..5);
                FillGen::Mask(s, rng.random_range(0..MASK_STYLE_COUNT))
            }
            6 => FillGen::AztecDiamond(rng.random_range(2..6)),
            7 => FillGen::Fret(rng.random_range(2..5)),
            8..=9 => FillGen::Flower(rng.random_range(0..5)),
            10 => FillGen::Fruit(rng.random_range(0..5)),
            _ => FillGen::Tile(TileParams::randomized(rng)),
        };

        // Size the element rect
        let (ew, eh) = match fill {
            FillGen::Tree(_) => (rng.random_range(14..24), rng.random_range(10..20)),
            FillGen::Mask(s, _) => (s * 4 + 4, s * 4 + 4),
            FillGen::AztecDiamond(o) => (o * 4 + 4, o * 2 + 4),
            FillGen::Fret(s) => (s * 4 + 2, s * 4 + 2),
            FillGen::Flower(_) | FillGen::Fruit(_) => (5, 5),
            _ => (scene_w, scene_h), // tile/noise fills span the scene
        };

        let elx = ex.saturating_sub(ew / 2).min(w.saturating_sub(ew));
        let ely = ey.saturating_sub(eh / 2).min(h.saturating_sub(eh));
        let el_rect = Rect { x: elx, y: ely, w: ew.min(w - elx), h: eh.min(h - ely) };

        let mask: MaskFn = if fill_breaks_out(&fill) {
            // Breakout: element gets its own rect mask, no scene clipping
            Box::new(mask_rect(&el_rect, 0.0))
        } else {
            // Bounded: clip to scene ellipse
            Box::new(mask_ellipse(cx, cy, rx, ry, 2.5))
        };

        layers.push(Layer { fill, mask: Some(mask), palette: pal });
    }

    layers
}

/// Path walk v2: random-walk waypoints, each is a multi-layer scene.
/// Returns (layers, stops) so the caller can draw the path trail after rendering.
pub fn path_walk_layers_2(
    w: usize,
    h: usize,
    palette: &[Color; 5],
    rng: &mut StdRng,
) -> (Vec<Layer>, Vec<(usize, usize)>) {
    let mut layers = Vec::new();
    let margin = 8usize;

    // Generate waypoints
    let stop_count = rng.random_range(3..6u32) as usize;
    let mut stops = Vec::with_capacity(stop_count);

    let mut px = rng.random_range(w / 6..w * 5 / 6);
    let mut py = rng.random_range(h / 6..h * 5 / 6);
    stops.push((px, py));

    for _ in 1..stop_count {
        let min_step = (w.min(h) / 4).max(12);
        let max_step = (w.min(h) / 2).max(min_step + 5);
        let angle: f32 = rng.random::<f32>() * std::f32::consts::TAU;
        let dist = rng.random_range(min_step..max_step) as f32;
        px = (px as f32 + angle.cos() * dist * 1.8)
            .clamp(margin as f32, (w - margin) as f32) as usize;
        py = (py as f32 + angle.sin() * dist)
            .clamp(margin as f32, (h - margin) as f32) as usize;
        stops.push((px, py));
    }

    // Build scene at each waypoint
    for &(sx, sy) in &stops {
        let sw = rng.random_range((w / 6).max(16)..(w / 3).max(20));
        let sh = rng.random_range((h / 5).max(10)..(h / 3).max(14));
        layers.extend(waypoint_scene(sx, sy, sw, sh, w, h, palette, rng));
    }

    (layers, stops)
}

/// Draw a path trail between waypoints directly on the grid using box-drawing chars.
/// Bresenham line with directional glyphs.
pub fn draw_path_trail(
    grid: &mut Grid,
    stops: &[(usize, usize)],
    color: Color,
    rng: &mut StdRng,
) {
    let h = grid.len();
    if h == 0 { return; }
    let w = grid[0].len();

    let trail_chars: &[char] = &['·', '∙', '°', '⋅'];

    for pair in stops.windows(2) {
        let (x0, y0) = pair[0];
        let (x1, y1) = pair[1];

        let dx = x1 as f32 - x0 as f32;
        let dy = y1 as f32 - y0 as f32;
        let steps = (dx.abs().max(dy.abs())) as usize;
        if steps == 0 { continue; }

        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let x = (x0 as f32 + dx * t) as usize;
            let y = (y0 as f32 + dy * t) as usize;

            if x >= w || y >= h { continue; }

            // Only draw trail on blank cells to avoid overwriting scene content
            if grid[y][x].ch != ' ' { continue; }

            // Sparse: skip some cells for a dotted trail feel
            if rng.random_range(0..3u32) > 0 { continue; }

            let slope = if dx.abs() < 1.0 {
                '│'
            } else {
                let ratio = dy / dx;
                if ratio.abs() < 0.3 {
                    '─'
                } else if ratio > 0.0 {
                    '╲'
                } else {
                    '╱'
                }
            };

            // Alternate between directional glyph and trail dots
            let ch = if rng.random_range(0..3u32) == 0 {
                slope
            } else {
                trail_chars[rng.random_range(0..trail_chars.len())]
            };

            grid[y][x] = Cell::new(ch, darken(color, rng.random_range(0..30)));
        }
    }
}

/// Growth pattern for placing elements within a waypoint box.
#[derive(Clone, Copy)]
pub enum Growth { Rect, Diamond, FlatHex }

/// Path walk v3: boxes along a path, each is a mini-world.
/// Overlapping boxes cannot reuse fill+color combos.
/// density: 0-100, controls element count per box.
pub fn path_walk_layers_3(
    w: usize,
    h: usize,
    palette: &[Color; 5],
    density: u32,
    rng: &mut StdRng,
) -> (Vec<Layer>, Vec<(usize, usize)>, Vec<(usize, usize, usize, usize)>) {
    let mut layers = Vec::new();
    let margin = 4usize;
    let gap = 6usize; // breathing room between boxes

    // ── PLACE BOXES VIA REJECTION SAMPLING ─────────
    let target_count = rng.random_range(4..8u32) as usize;
    let mut boxes: Vec<(usize, usize, usize, usize)> = Vec::new();
    let mut stops: Vec<(usize, usize)> = Vec::new();
    let mut box_claims: Vec<Vec<(u8, usize)>> = Vec::new();

    for _ in 0..target_count {
        let bw = rng.random_range(16u32..45.min(w as u32 * 2 / 5)) as usize;
        let bh = rng.random_range(10u32..28.min(h as u32 * 2 / 5)) as usize;

        let mut placed = false;
        for _ in 0..30 {
            let bx = rng.random_range(margin..w.saturating_sub(bw + margin).max(margin + 1));
            let by = rng.random_range(margin..h.saturating_sub(bh + margin).max(margin + 1));

            // Check overlap with gap padding against all placed boxes
            let overlaps = boxes.iter().any(|&(px, py, pw, ph)| {
                bx < px + pw + gap && bx + bw + gap > px &&
                by < py + ph + gap && by + bh + gap > py
            });

            if !overlaps {
                boxes.push((bx, by, bw, bh));
                stops.push((bx + bw / 2, by + bh / 2));
                placed = true;
                break;
            }
        }
        if !placed { continue; } // couldn't fit, skip this box
    }

    // Sort stops left-to-right so the path reads naturally
    let mut order: Vec<usize> = (0..stops.len()).collect();
    order.sort_by_key(|&i| stops[i].0);
    let sorted_stops: Vec<(usize, usize)> = order.iter().map(|&i| stops[i]).collect();
    let sorted_boxes: Vec<(usize, usize, usize, usize)> = order.iter().map(|&i| boxes[i]).collect();
    let stops = sorted_stops;
    let boxes = sorted_boxes;

    let max_elements = 8u32;
    let element_count = (2 + density * max_elements / 100) as usize;

    // ── FILL EACH BOX ──────────────────────────────
    for (si, &(bx, by, bw, bh)) in boxes.iter().enumerate() {
        let sx = bx + bw / 2;
        let sy = by + bh / 2;

        let box_rect = Rect { x: bx, y: by, w: bw, h: bh };

        // No overlaps by construction, so no forbidden fills needed
        let mut forbidden: Vec<(u8, usize)> = Vec::new();

        let mut claims: Vec<(u8, usize)> = Vec::new();

        // Growth pattern for this stop
        let growth = match rng.random_range(0..3u32) {
            0 => Growth::Rect,
            1 => Growth::Diamond,
            _ => Growth::FlatHex,
        };
        let spacing = rng.random_range(8..16u32) as i32;

        // Subtle background: dim dot noise so the box area has faint texture
        if rng.random_range(0..3u32) > 0 {
            let mut pal = *palette;
            pal[1] = darken(palette[rng.random_range(1..4)], 70);
            layers.push(Layer {
                fill: FillGen::Noise(NoiseVariant::Dot),
                mask: Some(Box::new(mask_rect(&box_rect, 0.0))),
                palette: pal,
            });
        }

        // Place elements using growth pattern
        for ei in 0..element_count {
            let (ox, oy) = growth_offset(growth, ei as i32, spacing);
            // Breakout fills can wander outside box, bounded fills stay inside
            let ex = (sx as i32 + ox).clamp(bx as i32, (bx + bw) as i32 - 1).clamp(2, w as i32 - 4) as usize;
            let ey = (sy as i32 + oy).clamp(by as i32, (by + bh) as i32 - 1).clamp(2, h as i32 - 4) as usize;

            let (fill, ci) = pick_unique_element(&forbidden, &claims, palette, rng);
            claims.push((fill_disc(&fill), ci));

            let mut pal = *palette;
            pal[1] = palette[ci];

            let (ew, eh) = element_size(&fill, rng);
            let elx = ex.saturating_sub(ew / 2).min(w.saturating_sub(ew));
            let ely = ey.saturating_sub(eh / 2).min(h.saturating_sub(eh));
            let el_rect = Rect { x: elx, y: ely, w: ew.min(w - elx), h: eh.min(h - ely) };

            let mask: MaskFn = if fill_breaks_out(&fill) {
                Box::new(mask_rect(&el_rect, 0.0))
            } else {
                let ecx = elx as f32 + ew as f32 * 0.5;
                let ecy = ely as f32 + eh as f32 * 0.5;
                pick_element_mask(ecx, ecy, ew as f32, eh as f32, rng)
            };

            layers.push(Layer { fill, mask: Some(mask), palette: pal });
        }

        box_claims.push(claims);
    }

    (layers, stops, boxes)
}

/// Pick a random container mask for a bounded fill element.
/// cx/cy are the element center, ew/eh are its char dimensions.
/// Returns a MaskFn shaped to the element, picked from the full shape vocabulary.
/// Breakout fills (trees, sprites) always get plain rects -- don't call this for them.
fn pick_element_mask(
    cx: f32, cy: f32, ew: f32, eh: f32,
    rng: &mut StdRng,
) -> MaskFn {
    // Terminal cells are ~2:1 (h:w), so rx = 2*ry for visually balanced shapes.
    let ry = eh * 0.5;
    let rx = ew * 0.5;

    match rng.random_range(0..7u32) {
        0 => Box::new(mask_rect(
            &Rect { x: (cx - rx) as usize, y: (cy - ry) as usize, w: ew as usize, h: eh as usize },
            0.0,
        )),
        1 => Box::new(mask_ellipse(cx, cy, rx, ry, 1.5)),
        2 => Box::new(mask_diamond(cx, cy, rx, ry, 0.0)),
        3 => Box::new(mask_hexagon(cx, cy, rx, ry, 1.0)),
        4 => {
            let shear = rng.random_range(3..9) as f32 * if rng.random_range(0..2u32) == 0 { 1.0 } else { -1.0 };
            Box::new(mask_parallelogram(cx, cy, ew * 0.85, eh * 0.85, shear, 0.0))
        }
        5 => {
            let dir = match rng.random_range(0..2u32) {
                0 => TriDir::Up,
                _ => TriDir::Down,
            };
            Box::new(mask_triangle(cx, cy, rx, ry, dir, 0.0))
        }
        _ => {
            let (w_top, w_bot) = if rng.random_range(0..2u32) == 0 {
                (ew * 0.25, ew * 0.85) // wide bottom
            } else {
                (ew * 0.85, ew * 0.25) // wide top
            };
            Box::new(mask_trapezoid(cx, cy, w_top, w_bot, eh * 0.85, 0.0))
        }
    }
}

/// Discriminant byte for a FillGen variant, used for overlap dedup.
fn fill_disc(f: &FillGen) -> u8 {
    match f {
        FillGen::TilePure(_) => 0,
        FillGen::Tile(_) => 1,
        FillGen::Noise(_) => 2,
        FillGen::Crosshatch => 3,
        FillGen::Guilloche => 4,
        FillGen::Weave => 5,
        FillGen::Zigzag => 6,
        FillGen::DiamondLattice => 7,
        FillGen::Tree(_) => 8,
        FillGen::AztecDiamond(_) => 9,
        FillGen::Flower(_) => 10,
        FillGen::Fruit(_) => 11,
        FillGen::Mask(_, _) => 12,
        FillGen::Fret(_) => 13,
        FillGen::Spiral => 14,
        FillGen::Concentric => 15,
        FillGen::Labyrinth => 16,
        FillGen::CaSnapshot(_) => 17,
        FillGen::Explosion => 18,
        FillGen::Rule1D(_) => 19,
        FillGen::Nothing => 20,
    }
}

/// Growth offset: position of element `i` relative to center, based on pattern.
fn growth_offset(growth: Growth, i: i32, spacing: i32) -> (i32, i32) {
    // Element 0 is always at center
    if i == 0 { return (0, 0); }
    // Remaining elements spiral/spread outward
    let ring = i;
    match growth {
        Growth::Rect => {
            // Spread in a grid: alternate left/right, up/down
            let angle = ring as f32 * std::f32::consts::FRAC_PI_2 + 0.3;
            let dist = ring as f32 * spacing as f32;
            ((angle.cos() * dist * 2.0) as i32, (angle.sin() * dist) as i32)
        }
        Growth::Diamond => {
            // Alternate sides, growing outward
            let side = if ring % 2 == 0 { 1.0f32 } else { -1.0 };
            let dist = ((ring + 1) / 2) as f32 * spacing as f32;
            let angle = side * std::f32::consts::FRAC_PI_4 + (ring as f32 * 0.8);
            ((angle.cos() * dist * 2.0) as i32, (angle.sin() * dist) as i32)
        }
        Growth::FlatHex => {
            // Hex-ish: 60-degree increments
            let angle = ring as f32 * std::f32::consts::FRAC_PI_3;
            let dist = ((ring + 1) / 2) as f32 * spacing as f32;
            ((angle.cos() * dist * 2.0) as i32, (angle.sin() * dist) as i32)
        }
    }
}

/// Pick a background fill that avoids forbidden combos.
fn pick_unique_fill(
    palette: &[Color; 5],
    forbidden: &[(u8, usize)],
    rng: &mut StdRng,
) -> (FillGen, usize) {
    for _ in 0..10 {
        let fill = match rng.random_range(0..4u32) {
            0 => FillGen::Tile(TileParams::randomized(rng)),
            1 => FillGen::Noise(NoiseVariant::Grass),
            2 => FillGen::Noise(NoiseVariant::Dot),
            _ => FillGen::Crosshatch,
        };
        let ci = rng.random_range(1..4);
        if !forbidden.contains(&(fill_disc(&fill), ci)) {
            return (fill, ci);
        }
    }
    // fallback
    (FillGen::Noise(NoiseVariant::Dot), rng.random_range(1..4))
}

/// Pick an element fill that avoids forbidden + already-claimed combos.
fn pick_unique_element(
    forbidden: &[(u8, usize)],
    claims: &[(u8, usize)],
    palette: &[Color; 5],
    rng: &mut StdRng,
) -> (FillGen, usize) {
    let wolfram_rules: &[u8] = &[30, 90, 110, 150, 184, 60, 73];
    for _ in 0..10 {
        let fill = match rng.random_range(0..19u32) {
            0..=3 => FillGen::Tree(rng.random_range(0..12)),
            4..=5 => {
                let s = rng.random_range(2..5);
                FillGen::Mask(s, rng.random_range(0..MASK_STYLE_COUNT))
            }
            6 => FillGen::AztecDiamond(rng.random_range(2..6)),
            7 => FillGen::Fret(rng.random_range(2..5)),
            8..=9 => FillGen::Flower(rng.random_range(0..5)),
            10 => FillGen::Fruit(rng.random_range(0..5)),
            11 => FillGen::Spiral,
            12 => FillGen::Concentric,
            13 => FillGen::Labyrinth,
            14 => FillGen::CaSnapshot(rng.random_range(0..4) as u8),
            15 => FillGen::Explosion,
            16 => FillGen::Rule1D(wolfram_rules[rng.random_range(0..wolfram_rules.len())]),
            _ => FillGen::Tile(TileParams::randomized(rng)),
        };
        let ci = rng.random_range(1..4);
        let disc = fill_disc(&fill);
        if !forbidden.contains(&(disc, ci)) && !claims.contains(&(disc, ci)) {
            return (fill, ci);
        }
    }
    // fallback
    (FillGen::Flower(rng.random_range(0..5)), rng.random_range(1..4))
}

/// Size in cells for a given fill type.
fn element_size(fill: &FillGen, rng: &mut StdRng) -> (usize, usize) {
    match fill {
        FillGen::Tree(_) => (rng.random_range(14..24), rng.random_range(10..20)),
        FillGen::Mask(s, _) => (s * 4 + 4, s * 4 + 4),
        FillGen::AztecDiamond(o) => (o * 4 + 4, o * 2 + 4),
        FillGen::Fret(s) => (s * 4 + 2, s * 4 + 2),
        FillGen::Flower(_) | FillGen::Fruit(_) => (5, 5),
        FillGen::Spiral | FillGen::Concentric | FillGen::Labyrinth => (20, 12),
        FillGen::CaSnapshot(_) => (rng.random_range(16..28), rng.random_range(10..18)),
        FillGen::Explosion => (rng.random_range(14..24), rng.random_range(8..14)),
        FillGen::Rule1D(_) => (rng.random_range(16..30), rng.random_range(10..20)),
        _ => (20, 12),
    }
}

/// Scatter a few big recognizable sprites as overlay layers.
/// Biased toward trees and faces -- the things that read well at distance.
pub fn scatter_layers(
    w: usize,
    h: usize,
    count: usize,
    palette: &[Color; 5],
    rng: &mut StdRng,
) -> Vec<Layer> {
    let mut layers = Vec::with_capacity(count);
    for _ in 0..count {
        let pal_idx = rng.random_range(1..5);
        let mut pal = *palette;
        pal[1] = palette[pal_idx];

        let (fill, rw, rh) = match rng.random_range(0..10u32) {
            0..=4 => {
                // Trees -- big, tall, unmistakable
                let tree_h = rng.random_range(14..h.min(30).max(15));
                let tree_w = rng.random_range(16..w.min(36).max(17));
                (FillGen::Tree(rng.random_range(0..12)), tree_w, tree_h)
            }
            5..=7 => {
                // Faces -- big masks
                let s = rng.random_range(3..6);
                (FillGen::Mask(s, rng.random_range(0..MASK_STYLE_COUNT)), s * 4 + 4, s * 4 + 4)
            }
            8 => {
                let order = rng.random_range(3..7);
                (FillGen::AztecDiamond(order), order * 4 + 4, order * 2 + 4)
            }
            _ => {
                let steps = rng.random_range(3..6);
                (FillGen::Fret(steps), steps * 4 + 2, steps * 4 + 2)
            }
        };

        let x = rng.random_range(2..w.saturating_sub(rw + 2).max(3));
        let y = rng.random_range(2..h.saturating_sub(rh + 2).max(3));

        let rect = Rect { x, y, w: rw.min(w - x), h: rh.min(h - y) };
        let cx = x as f32 + rw as f32 / 2.0;
        let cy = y as f32 + rh as f32 / 2.0;

        layers.push(Layer {
            fill,
            mask: Some(Box::new(mask_rect(&rect, 0.0))),
            palette: pal,
        });
    }
    layers
}

/// Sinuous stalk walk: generates leaf layers along a vertical spine.
/// Leaf shape varies by stalk position: trapezoid at base, parallelogram in mid, triangle at tip.
/// Leaves alternate sides and shrink toward the apex.
/// Returns (layers, spine) where spine is the integer stalk path for draw_stalk.
pub fn path_walk_stem(
    w: usize,
    h: usize,
    palette: &[Color; 5],
    rng: &mut StdRng,
) -> (Vec<Layer>, Vec<(usize, usize)>) {
    let mut layers = Vec::new();
    let margin_x = (w / 6).max(4);

    // Sinuous spine: 6 control points trending from bottom to top
    let n_ctrl = 6usize;
    let mut spine: Vec<(f32, f32)> = Vec::with_capacity(n_ctrl);
    let x0 = (w / 2) as f32 + rng.random_range(-4i32..5) as f32;
    spine.push((x0, (h - 3) as f32));
    for i in 1..n_ctrl {
        let t = i as f32 / (n_ctrl - 1) as f32;
        let base_y = (h as f32 - 3.0) - t * (h as f32 - 7.0);
        let wander = rng.random_range(-5i32..6) as f32;
        let x = (spine.last().unwrap().0 + wander)
            .clamp(margin_x as f32, (w - margin_x) as f32);
        spine.push((x, base_y));
    }

    // Cumulative arc length for even resampling
    let arc: Vec<f32> = std::iter::once(0.0f32)
        .chain(spine.windows(2).map(|s| {
            let dx = s[1].0 - s[0].0;
            let dy = s[1].1 - s[0].1;
            (dx * dx + dy * dy).sqrt()
        }))
        .scan(0.0f32, |acc, v| { *acc += v; Some(*acc) })
        .collect();
    let total_len = *arc.last().unwrap_or(&1.0);

    let leaf_count = rng.random_range(5..9) as usize;

    // Sample leaf positions at regular arc-length intervals
    let mut leaf_pts: Vec<(f32, f32)> = Vec::with_capacity(leaf_count);
    for i in 0..leaf_count {
        let target = ((i as f32 + 0.5) / leaf_count as f32) * total_len;
        let seg = arc.windows(2).enumerate()
            .find(|(_, w)| w[1] >= target)
            .map(|(i, _)| i)
            .unwrap_or(spine.len() - 2);
        let seg_len = arc[seg + 1] - arc[seg];
        let local_t = if seg_len > 0.0 { (target - arc[seg]) / seg_len } else { 0.0 };
        let px = spine[seg].0 + local_t * (spine[seg + 1].0 - spine[seg].0);
        let py = spine[seg].1 + local_t * (spine[seg + 1].1 - spine[seg].1);
        leaf_pts.push((px, py));
    }

    // Build one leaf layer per sampled point
    for (i, &(lx, ly)) in leaf_pts.iter().enumerate() {
        // t: 0.0 = base (bottom of screen), 1.0 = apex (top)
        let t = (1.0 - (ly - 3.0) / (h as f32 - 7.0).max(1.0)).clamp(0.0, 1.0);

        // Leaf dims shrink toward tip
        let leaf_w = ((22.0 - t * 12.0) as usize).max(8);
        let leaf_h = ((11.0 - t * 5.0) as usize).max(4);

        // Alternating sides
        let side = if i % 2 == 0 { 1f32 } else { -1f32 };
        let leaf_cx = lx + side * (leaf_w as f32 * 0.5 + 1.0);
        let leaf_cy = ly;

        let lrx = (leaf_cx - leaf_w as f32 * 0.5).max(1.0) as usize;
        let lrx = lrx.min(w.saturating_sub(leaf_w + 1));
        let lry = (leaf_cy - leaf_h as f32 * 0.5).max(1.0) as usize;
        let lry = lry.min(h.saturating_sub(leaf_h + 1));
        let leaf_rect = Rect {
            x: lrx, y: lry,
            w: leaf_w.min(w - lrx),
            h: leaf_h.min(h - lry),
        };

        // Stalk tangent at this point (for parallelogram shear)
        let prev = if i > 0 { leaf_pts[i - 1] } else { (lx, ly + 3.0) };
        let next = if i + 1 < leaf_count { leaf_pts[i + 1] } else { (lx, ly - 3.0) };
        let tan_x = next.0 - prev.0;
        let tan_y = (prev.1 - next.1).max(0.01); // upward: prev_y > next_y
        let shear = (tan_x / tan_y * 5.0 * side).clamp(-10.0, 10.0);

        let mut pal = *palette;
        pal[1] = palette[rng.random_range(1..4)];

        let mut tile_params = TileParams::randomized(rng);
        tile_params.skew = rng.random_range(30..70);

        let mask: MaskFn = if t < 0.3 {
            // base: wide-bottom trapezoid
            let w_top = leaf_w as f32 * 0.3;
            let w_bot = leaf_w as f32 * 0.9;
            Box::new(mask_trapezoid(leaf_cx, leaf_cy, w_top, w_bot, leaf_h as f32 * 0.85, 0.0))
        } else if t > 0.7 {
            // apex: upward-pointing triangle
            let rx = leaf_w as f32 * 0.5;
            let ry = leaf_h as f32 * 0.5;
            Box::new(mask_triangle(leaf_cx, leaf_cy, rx, ry, TriDir::Up, 0.0))
        } else {
            // mid: parallelogram leans with stalk tangent direction
            Box::new(mask_parallelogram(leaf_cx, leaf_cy, leaf_w as f32 * 0.85, leaf_h as f32 * 0.85, shear, 0.0))
        };

        layers.push(Layer { fill: FillGen::Tile(tile_params), mask: Some(mask), palette: pal });
    }

    let spine_pts: Vec<(usize, usize)> = spine.iter()
        .map(|&(x, y)| (x.round() as usize, y.round() as usize))
        .collect();

    (layers, spine_pts)
}

/// Draw a plant stalk on the grid using line-art chars.
/// Unlike draw_path_trail, this overwrites existing cells -- the stalk is structural.
pub fn draw_stalk(
    grid: &mut Grid,
    spine: &[(usize, usize)],
    color: Color,
) {
    let h = grid.len();
    if h == 0 { return; }
    let w = grid[0].len();

    for seg in spine.windows(2) {
        let (x0, y0) = seg[0];
        let (x1, y1) = seg[1];
        let dx = x1 as f32 - x0 as f32;
        let dy = y1 as f32 - y0 as f32;
        let steps = dx.abs().max(dy.abs()) as usize;
        if steps == 0 { continue; }

        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let x = (x0 as f32 + dx * t).round() as usize;
            let y = (y0 as f32 + dy * t).round() as usize;
            if x >= w || y >= h { continue; }

            // Going upward (dy < 0): classify by horizontal lean
            let ch = if dx.abs() < dy.abs() * 0.35 {
                '│'
            } else if dx > 0.0 {
                '╱'
            } else {
                '╲'
            };
            grid[y][x] = Cell::new(ch, color);
        }
    }
}

// ── Node scene generators ──────────────────────────────────────────

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
    }
}

/// Generate a wavy contour line across a rect width.
/// Returns a Vec of y-values (one per column from rect.x to rect.x+rect.w).
/// `base_y`: average y of the contour.
/// `amplitude`: max deviation from base.
/// `freq`: wave frequency (higher = more peaks).
fn gen_contour(rect: &Rect, base_y: usize, amplitude: f32, freq: f32, rng: &mut StdRng) -> Vec<usize> {
    let phase = rng.random::<f32>() * std::f32::consts::TAU;
    let phase2 = rng.random::<f32>() * std::f32::consts::TAU;
    (0..rect.w).map(|col| {
        let t = col as f32 / rect.w.max(1) as f32;
        // Two sine waves at different frequencies for organic feel
        let wave = (t * freq * std::f32::consts::TAU + phase).sin() * amplitude
                 + (t * freq * 2.3 * std::f32::consts::TAU + phase2).sin() * amplitude * 0.4;
        let y = base_y as f32 + wave;
        y.clamp(rect.y as f32 + 2.0, (rect.y + rect.h - 2) as f32) as usize
    }).collect()
}

/// Landscape: 3-pass rendering.
/// Pass 1: sky background (sparse dots or nothing -- mostly empty).
/// Pass 2: ground below a wavy contour line (grass/tile/crosshatch).
/// Pass 3: foreground sprites rooted ON the ground line.
fn make_landscape(
    rect: &Rect,
    palette: &[Color; 5],
    detail: u32,
    rng: &mut StdRng,
) -> Vec<Layer> {
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
            4..=5 => {
                (FillGen::Flower(rng.random_range(0..5)), 5, 5)
            }
            6 => {
                let s = rng.random_range(2..5);
                (FillGen::Mask(s, rng.random_range(0..MASK_STYLE_COUNT)), s * 4 + 4, s * 4 + 4)
            }
            7 => {
                let tw = rng.random_range(10..20);
                let th = rng.random_range(6..12);
                (FillGen::Tile(TileParams::randomized(rng)), tw, th)
            }
            8 => {
                (FillGen::Fruit(rng.random_range(0..5)), 5, 5)
            }
            _ => {
                // Aztec diamond accent
                let order = rng.random_range(2..5);
                (FillGen::AztecDiamond(order), order * 4 + 4, order * 2 + 4)
            }
        };

        // Root the element so its bottom sits on the ground line
        let elx = ex.saturating_sub(ew / 2).min(rect.x + rect.w - ew.min(rect.w));
        let ely = ground_y.saturating_sub(eh).max(rect.y).min(rect.y + rect.h - eh.min(rect.h));
        let el_rect = Rect { x: elx, y: ely, w: ew.min(rect.w), h: eh.min(rect.h) };

        let mask: MaskFn = if fill_breaks_out(&fill) {
            Box::new(mask_rect(&el_rect, 0.0))
        } else {
            let cx = elx as f32 + ew as f32 * 0.5;
            let cy = ely as f32 + eh as f32 * 0.5;
            Box::new(mask_ellipse(cx, cy, ew as f32 * 0.5, eh as f32 * 0.5, 2.0))
        };

        layers.push(Layer { fill, mask: Some(mask), palette: pal });
    }

    layers
}

/// Centerpiece with surround: one big element in a patterned field.
fn make_centerpiece(
    rect: &Rect,
    palette: &[Color; 5],
    rng: &mut StdRng,
) -> Vec<Layer> {
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
    layers.push(Layer { fill: bg_fill, mask: Some(node_mask), palette: bg_pal });

    // Centerpiece: big sprite
    let mut cp_pal = *palette;
    cp_pal[1] = palette[rng.random_range(1..4)];
    let (cp_fill, cw, ch) = match rng.random_range(0..6u32) {
        0..=1 => {
            let s = rng.random_range(3..6);
            (FillGen::Mask(s, rng.random_range(0..MASK_STYLE_COUNT)), s * 4 + 4, s * 4 + 4)
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
    let elx = (cx as usize).saturating_sub(cw / 2).min(rect.x + rect.w - cw.min(rect.w));
    let ely = (cy as usize).saturating_sub(ch / 2).min(rect.y + rect.h - ch.min(rect.h));
    let el_rect = Rect { x: elx, y: ely, w: cw.min(rect.w), h: ch.min(rect.h) };
    layers.push(Layer {
        fill: cp_fill,
        mask: Some(Box::new(mask_rect(&el_rect, 0.0))),
        palette: cp_pal,
    });

    layers
}

/// Cluster: N related patterns in a spatial arrangement.
fn make_cluster(
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
                (FillGen::Mask(s, rng.random_range(0..MASK_STYLE_COUNT)), s * 4 + 4, s * 4 + 4)
            }
            _ => (FillGen::Noise(noise_variant_from_index(rng.random_range(0..NOISE_VARIANT_COUNT))), cell_w, cell_h),
        };

        let elx = ex.saturating_sub(ew / 2).min(rect.x + rect.w - ew.min(rect.w));
        let ely = ey.saturating_sub(eh / 2).min(rect.y + rect.h - eh.min(rect.h));

        let ecx = elx as f32 + ew as f32 * 0.5;
        let ecy = ely as f32 + eh as f32 * 0.5;
        let mask: MaskFn = if fill_breaks_out(&fill) {
            let el_rect = Rect { x: elx, y: ely, w: ew.min(rect.w), h: eh.min(rect.h) };
            Box::new(mask_rect(&el_rect, 0.0))
        } else {
            pick_element_mask(ecx, ecy, ew as f32, eh as f32, rng)
        };

        layers.push(Layer { fill, mask: Some(mask), palette: pal });
    }

    layers
}

/// Position offset for cluster element i of n within a bounding box of (w, h).
fn cluster_offset(
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

    let node_count = rng.random_range(character.branch_factor.0 as u32..=character.branch_factor.1 as u32 + 2) as usize;
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
        px = (px as f32 + angle.cos() * dist * 1.8)
            .clamp(margin as f32, (w - margin) as f32) as usize;
        py = (py as f32 + angle.sin() * dist)
            .clamp(margin as f32, (h - margin) as f32) as usize;
        stops.push((px, py));
    }

    for (i, &(sx, sy)) in stops.iter().enumerate() {
        let t = if node_count > 1 { i as f32 / (node_count - 1) as f32 } else { 0.5 };
        let sf = character.size_factor(t);
        let base_w = rng.random_range((w / 5).max(16)..(w / 3).max(20));
        let base_h = rng.random_range((h / 5).max(10)..(h / 3).max(14));
        let nw = (base_w as f32 * sf) as usize;
        let nh = (base_h as f32 * sf) as usize;
        let nx = sx.saturating_sub(nw / 2).min(w.saturating_sub(nw + margin));
        let ny = sy.saturating_sub(nh / 2).min(h.saturating_sub(nh + margin));
        let node_rect = Rect { x: nx, y: ny, w: nw.min(w - nx), h: nh.min(h - ny) };
        let mode = NodeMode::pick(character.landscape_bias, rng);
        let arc_shift = (i as f32 / node_count as f32 * 60.0) as u8;
        let mut node_pal = *palette;
        node_pal[1] = shift_hue(palette[1], arc_shift);
        node_pal[2] = shift_hue(palette[2], arc_shift);
        layers.extend(make_node_scene(&node_rect, mode, &node_pal, 50, rng));
    }

    (layers, stops)
}

/// Tuning knobs for party_walk, exposed as CLI args.
pub struct PartyParams {
    /// Gap between nodes in cells. 0 = auto (scales with terminal).
    pub gap: usize,
    /// Target node count. 0 = auto (5-11 random).
    pub nodes: usize,
    /// Box size factor 0-100. 50 = default. Higher = bigger nodes.
    pub scale: u32,
    /// Detail level 0-100. Controls foreground elements per landscape node.
    pub detail: u32,
}

impl PartyParams {
    pub fn default() -> Self {
        PartyParams { gap: 0, nodes: 0, scale: 50, detail: 50 }
    }
}

/// Party walk: non-overlapping node islands along a path.
/// Each node is a distinct scene clipped to a shape, with breathing room between.
pub fn party_walk(
    w: usize,
    h: usize,
    palette: &[Color; 5],
    params: &PartyParams,
    rng: &mut StdRng,
) -> (Vec<Layer>, Vec<(usize, usize)>, Vec<(usize, usize, usize, usize)>) {
    let character = PlantCharacter::random(rng);
    let mut layers = Vec::new();
    let margin = 2usize;
    let gap = if params.gap > 0 { params.gap } else { (w.min(h) / 15).max(3) };

    // ── Place non-overlapping boxes via rejection sampling ──
    let target_count = if params.nodes > 0 {
        params.nodes
    } else {
        rng.random_range(5..12u32) as usize
    };
    let mut boxes: Vec<(usize, usize, usize, usize)> = Vec::new(); // (x, y, w, h)
    let mut stops: Vec<(usize, usize)> = Vec::new();
    let mut total_area = 0usize;
    let canvas_area = w * h;

    // Scale factor: 0=tiny, 50=default, 100=huge
    let size_mult = params.scale as f32 / 50.0; // 0.0 to 2.0

    for i in 0..target_count {
        let t = if target_count > 1 { i as f32 / (target_count - 1) as f32 } else { 0.5 };
        let sf = character.size_factor(t);

        let base_max_bw = (w as u32 * 2 / 5).max(24).min(55);
        let base_max_bh = (h as u32 * 2 / 5).max(14).min(28);
        let max_bw = ((base_max_bw as f32 * size_mult).max(16.0)) as u32;
        let max_bh = ((base_max_bh as f32 * size_mult).max(10.0)) as u32;
        let min_bw = (max_bw / 2).max(10);
        let min_bh = (max_bh / 2).max(6);
        let bw = ((rng.random_range(min_bw..max_bw.max(min_bw + 1)) as f32) * sf).max(8.0) as usize;
        let bh = ((rng.random_range(min_bh..max_bh.max(min_bh + 1)) as f32) * sf).max(5.0) as usize;

        let mut placed = false;
        for _ in 0..60 {
            let bx = rng.random_range(margin..w.saturating_sub(bw + margin).max(margin + 1));
            let by = rng.random_range(margin..h.saturating_sub(bh + margin).max(margin + 1));

            let overlaps = boxes.iter().any(|&(px, py, pw, ph)| {
                bx < px + pw + gap && bx + bw + gap > px &&
                by < py + ph + gap && by + bh + gap > py
            });

            if !overlaps {
                boxes.push((bx, by, bw, bh));
                stops.push((bx + bw / 2, by + bh / 2));
                total_area += bw * bh;
                placed = true;
                break;
            }
        }
        if !placed { continue; }
    }

    // Sort left-to-right for readable path
    let mut order: Vec<usize> = (0..stops.len()).collect();
    order.sort_by_key(|&i| stops[i].0);
    let stops: Vec<(usize, usize)> = order.iter().map(|&i| stops[i]).collect();
    let boxes: Vec<(usize, usize, usize, usize)> = order.iter().map(|&i| boxes[i]).collect();

    // ── Build each node ──
    let node_count = boxes.len();
    for (i, &(bx, by, bw, bh)) in boxes.iter().enumerate() {
        let node_rect = Rect { x: bx, y: by, w: bw, h: bh };
        let cx = bx as f32 + bw as f32 * 0.5;
        let cy = by as f32 + bh as f32 * 0.5;
        let rx = bw as f32 * 0.5;
        let ry = bh as f32 * 0.5;

        let mode = NodeMode::pick(character.landscape_bias, rng);

        // Color arc
        let arc_shift = if node_count > 1 { (i as f32 / (node_count - 1) as f32 * 60.0) as u8 } else { 0 };
        let mut node_pal = *palette;
        node_pal[1] = shift_hue(palette[1], arc_shift);
        node_pal[2] = shift_hue(palette[2], arc_shift);

        // Node boundary shape -- clips everything inside
        let node_dissolve = 2.5;
        let node_shape: NodeShape = match rng.random_range(0..5u32) {
            0 => NodeShape::Ellipse,
            1 => NodeShape::Diamond,
            2 => NodeShape::Rect,
            3 => NodeShape::Hexagon,
            _ => NodeShape::Trapezoid(
                if rng.random_range(0..2u32) == 0 {
                    (bw as f32 * 0.4, bw as f32 * 0.9)
                } else {
                    (bw as f32 * 0.9, bw as f32 * 0.4)
                }
            ),
        };

        // Generate the node's internal layers
        let inner_layers = make_node_scene(&node_rect, mode, &node_pal, params.detail, rng);

        // Wrap each inner layer's mask with the node boundary
        for layer in inner_layers {
            let clipped_mask: MaskFn = match node_shape {
                NodeShape::Ellipse => {
                    let boundary = mask_ellipse(cx, cy, rx, ry, node_dissolve);
                    match layer.mask {
                        Some(inner) => mask_intersect(boundary, move |x, y| inner(x, y)),
                        None => Box::new(boundary),
                    }
                }
                NodeShape::Diamond => {
                    let boundary = mask_diamond(cx, cy, rx, ry, node_dissolve);
                    match layer.mask {
                        Some(inner) => mask_intersect(boundary, move |x, y| inner(x, y)),
                        None => Box::new(boundary),
                    }
                }
                NodeShape::Rect => {
                    let boundary = mask_rect(&node_rect, node_dissolve);
                    match layer.mask {
                        Some(inner) => mask_intersect(boundary, move |x, y| inner(x, y)),
                        None => Box::new(boundary),
                    }
                }
                NodeShape::Trapezoid((wt, wb)) => {
                    let boundary = mask_trapezoid(cx, cy, wt, wb, bh as f32 * 0.9, node_dissolve);
                    match layer.mask {
                        Some(inner) => mask_intersect(boundary, move |x, y| inner(x, y)),
                        None => Box::new(boundary),
                    }
                }
                NodeShape::Hexagon => {
                    let boundary = mask_hexagon(cx, cy, rx, ry, node_dissolve);
                    match layer.mask {
                        Some(inner) => mask_intersect(boundary, move |x, y| inner(x, y)),
                        None => Box::new(boundary),
                    }
                }
            };

            layers.push(Layer {
                fill: layer.fill,
                mask: Some(clipped_mask),
                palette: layer.palette,
            });
        }
    }

    (layers, stops, boxes)
}

/// Draw a box-drawing border around a rect on the grid.
/// Overwrites whatever is there -- this is structural.
pub fn draw_box_border(
    grid: &mut Grid,
    bx: usize, by: usize, bw: usize, bh: usize,
    color: Color,
) {
    let gh = grid.len();
    if gh == 0 { return; }
    let gw = grid[0].len();
    // Clamp to grid
    let x0 = bx.min(gw.saturating_sub(1));
    let y0 = by.min(gh.saturating_sub(1));
    let x1 = (bx + bw).min(gw.saturating_sub(1));
    let y1 = (by + bh).min(gh.saturating_sub(1));
    if x1 <= x0 || y1 <= y0 { return; }

    // Top edge
    for x in (x0 + 1)..x1 {
        grid[y0][x] = Cell::new('─', color);
    }
    // Bottom edge
    for x in (x0 + 1)..x1 {
        grid[y1][x] = Cell::new('─', color);
    }
    // Left edge
    for y in (y0 + 1)..y1 {
        grid[y][x0] = Cell::new('│', color);
    }
    // Right edge
    for y in (y0 + 1)..y1 {
        grid[y][x1] = Cell::new('│', color);
    }
    // Corners
    grid[y0][x0] = Cell::new('┌', color);
    grid[y0][x1] = Cell::new('┐', color);
    grid[y1][x0] = Cell::new('└', color);
    grid[y1][x1] = Cell::new('┘', color);
}

/// Draw a solid connecting path between waypoints.
/// Uses box-drawing line chars, overwrites existing cells.
/// Much more visible than draw_path_trail.
pub fn draw_walk_path(
    grid: &mut Grid,
    stops: &[(usize, usize)],
    color: Color,
) {
    let h = grid.len();
    if h == 0 { return; }
    let w = grid[0].len();

    for pair in stops.windows(2) {
        let (x0, y0) = pair[0];
        let (x1, y1) = pair[1];
        let dx = x1 as f32 - x0 as f32;
        let dy = y1 as f32 - y0 as f32;
        let steps = (dx.abs().max(dy.abs())) as usize;
        if steps == 0 { continue; }

        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let x = (x0 as f32 + dx * t) as usize;
            let y = (y0 as f32 + dy * t) as usize;
            if x >= w || y >= h { continue; }

            let ch = if dx.abs() < 1.0 {
                '│'
            } else {
                let ratio = dy / dx;
                if ratio.abs() < 0.3 {
                    '─'
                } else if ratio > 0.0 {
                    '╲'
                } else {
                    '╱'
                }
            };

            grid[y][x] = Cell::new(ch, color);
        }
    }
}

#[derive(Clone, Copy)]
enum NodeShape {
    Ellipse,
    Diamond,
    Rect,
    Trapezoid((f32, f32)), // (w_top, w_bot)
    Hexagon,
}

/// Shift a color's hue by `degrees` (approximate, works on RGB).
fn shift_hue(color: Color, degrees: u8) -> Color {
    match color {
        Color::Rgb { r, g, b } => {
            // Rotate through simple channel cycling
            let d = degrees as f32 / 60.0;
            let phase = d as usize % 3;
            match phase {
                0 => Color::Rgb { r, g: g.saturating_add(degrees / 3), b: b.saturating_sub(degrees / 4) },
                1 => Color::Rgb { r: r.saturating_sub(degrees / 4), g, b: b.saturating_add(degrees / 3) },
                _ => Color::Rgb { r: r.saturating_add(degrees / 3), g: g.saturating_sub(degrees / 4), b },
            }
        }
        other => other,
    }
}

/// Walk through leaf rects in nearest-neighbor order, filling each based on
/// walker mood/energy state.
pub fn walk_and_fill_leaves(
    grid: &mut Grid,
    leaves: &[Rect],
    palette: &[Color; 5],
    rng: &mut StdRng,
) {
    if leaves.is_empty() { return; }

    let mut walker = LeafWalker::new(grid[0].len() / 2, grid.len() / 2);
    let mut visited = vec![false; leaves.len()];

    for _ in 0..leaves.len() {
        let mut best_idx = None;
        let mut best_dist = usize::MAX;
        for (i, leaf) in leaves.iter().enumerate() {
            if visited[i] { continue; }
            let cx = leaf.x + leaf.w / 2;
            let cy = leaf.y + leaf.h / 2;
            let dist = cx.abs_diff(walker.prev_x) + cy.abs_diff(walker.prev_y);
            if dist < best_dist {
                best_dist = dist;
                best_idx = Some(i);
            }
        }

        let Some(idx) = best_idx else { break };
        visited[idx] = true;
        let rect = &leaves[idx];
        let fill = walker.pick_fill(rect, rng);
        let prim_color = palette[rng.random_range(1..4)];
        let color2 = darken(prim_color, 30);

        // Tree gets scatter on top; everything else goes through render_fill
        match fill {
            FillGen::Tree(_) => {
                render_fill(grid, rect, fill, prim_color, color2, palette, None, rng);
                for _ in 0..rng.random_range(1..=3) {
                    let fx = rect.x + rng.random_range(2..rect.w.saturating_sub(2).max(3));
                    let fy = rect.y + rng.random_range(2..rect.h.saturating_sub(2).max(3));
                    if rng.random_range(0..2) == 0 {
                        draw_fruit(grid, fx, fy, rng.random_range(0..5), palette[3]);
                    } else {
                        draw_flower(grid, fx, fy, rng.random_range(0..5), palette[3]);
                    }
                }
            }
            FillGen::Tile(params) => {
                let jittered = TileParams { jitter: (1.0 - walker.energy).max(0.0) * 0.15, ..params };
                render_fill(grid, rect, FillGen::Tile(jittered), prim_color, color2, palette, None, rng);
            }
            _ => render_fill(grid, rect, fill, prim_color, color2, palette, None, rng),
        }

        walker.step(rect, rng);
    }
}

// ── Atmosphere overlay ─────────────────────────────────────────────

/// Weather type for atmosphere overlay.
#[derive(Clone, Copy)]
pub enum Weather { Rain, Snow, Fog, Stars, None }

impl Weather {
    pub fn pick(rng: &mut StdRng) -> Self {
        match rng.random_range(0..8u32) {
            0 => Weather::Rain,
            1 => Weather::Snow,
            2 => Weather::Fog,
            3 => Weather::Stars,
            _ => Weather::None,
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "rain" => Weather::Rain,
            "snow" => Weather::Snow,
            "fog" => Weather::Fog,
            "stars" => Weather::Stars,
            "none" => Weather::None,
            _ => return Option::None,
        })
    }
}

/// Apply a weather overlay to the grid. Runs AFTER all scene compositing.
/// Only writes on cells that are blank or very sparse, preserving scene content.
/// `intensity`: 0-100, how dense the weather is.
pub fn apply_atmosphere(
    grid: &mut Grid,
    weather: Weather,
    intensity: u32,
    palette: &[Color; 5],
    rng: &mut StdRng,
) {
    let h = grid.len();
    if h == 0 { return; }
    let w = grid[0].len();
    let prob = intensity as f32 / 100.0;

    match weather {
        Weather::Rain => {
            let rain_color = darken(palette[2], 40);
            let rain_chars = ['│', '┊', '╎', '┆'];
            for y in 0..h {
                for x in 0..w {
                    if grid[y][x].ch != ' ' { continue; }
                    if rng.random::<f32>() > prob * 0.15 { continue; }
                    // Rain falls in vertical streaks -- bias toward same x columns
                    let streak = ((x * 7 + 13) % 11) < 3;
                    if !streak && rng.random::<f32>() > 0.3 { continue; }
                    let ch = rain_chars[rng.random_range(0..rain_chars.len())];
                    grid[y][x] = Cell::new(ch, darken(rain_color, rng.random_range(0..20)));
                }
            }
        }
        Weather::Snow => {
            let snow_color = lighten(palette[4], 20);
            let snow_chars = ['·', '∙', '°', '*', '⋅'];
            for y in 0..h {
                for x in 0..w {
                    if grid[y][x].ch != ' ' { continue; }
                    if rng.random::<f32>() > prob * 0.08 { continue; }
                    let ch = snow_chars[rng.random_range(0..snow_chars.len())];
                    grid[y][x] = Cell::new(ch, darken(snow_color, rng.random_range(0..40)));
                }
            }
        }
        Weather::Fog => {
            // Fog: horizontal bands of dim chars that partially overwrite content
            let fog_color = darken(palette[4], 80);
            let fog_chars = ['░', '▒', '·', '∙'];
            for y in 0..h {
                // Fog density varies by row -- sine wave bands
                let row_fog = ((y as f32 / h as f32 * 3.0 * std::f32::consts::PI).sin() * 0.5 + 0.5) * prob;
                for x in 0..w {
                    if rng.random::<f32>() > row_fog * 0.12 { continue; }
                    // Fog can overwrite sparse chars but not dense structure
                    let existing = grid[y][x].ch;
                    if existing != ' ' && !matches!(existing, '·' | '∙' | '°' | '⋅') { continue; }
                    let ch = fog_chars[rng.random_range(0..fog_chars.len())];
                    grid[y][x] = Cell::new(ch, fog_color);
                }
            }
        }
        Weather::Stars => {
            let star_color = lighten(palette[4], 10);
            let star_chars = ['✦', '✧', '·', '∙', '°'];
            for y in 0..h {
                for x in 0..w {
                    if grid[y][x].ch != ' ' { continue; }
                    if rng.random::<f32>() > prob * 0.03 { continue; }
                    let ch = star_chars[rng.random_range(0..star_chars.len())];
                    let twinkle = rng.random_range(0..60);
                    grid[y][x] = Cell::new(ch, darken(star_color, twinkle));
                }
            }
        }
        Weather::None => {}
    }
}

// ── Path character system ──────────────────────────────────────────

/// Visual style of the path connecting nodes.
#[derive(Clone, Copy)]
pub enum PathStyle {
    /// Simple box-drawing line (the default from draw_walk_path)
    Line,
    /// Dotted trail with directional hints
    Dots,
    /// Vine/branch with organic chars
    Vine,
    /// River/water flow
    River,
    /// Double-line border path
    DoubleLine,
}

impl PathStyle {
    pub fn pick(rng: &mut StdRng) -> Self {
        match rng.random_range(0..5u32) {
            0 => PathStyle::Line,
            1 => PathStyle::Dots,
            2 => PathStyle::Vine,
            3 => PathStyle::River,
            _ => PathStyle::DoubleLine,
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "line" => PathStyle::Line,
            "dots" => PathStyle::Dots,
            "vine" => PathStyle::Vine,
            "river" => PathStyle::River,
            "double" => PathStyle::DoubleLine,
            _ => return Option::None,
        })
    }
}

/// Draw a styled path between waypoints on the grid.
pub fn draw_styled_path(
    grid: &mut Grid,
    stops: &[(usize, usize)],
    style: PathStyle,
    color: Color,
    rng: &mut StdRng,
) {
    match style {
        PathStyle::Line => draw_walk_path(grid, stops, color),
        PathStyle::Dots => draw_path_trail(grid, stops, color, rng),
        PathStyle::Vine => draw_vine_path(grid, stops, color, rng),
        PathStyle::River => draw_river_path(grid, stops, color, rng),
        PathStyle::DoubleLine => draw_double_path(grid, stops, color),
    }
}

/// Vine path: organic line with occasional leaf/bud chars branching off.
fn draw_vine_path(
    grid: &mut Grid,
    stops: &[(usize, usize)],
    color: Color,
    rng: &mut StdRng,
) {
    let h = grid.len();
    if h == 0 { return; }
    let w = grid[0].len();
    let vine_chars = ['╱', '╲', '│', '─', '╰', '╮', '╭', '╯'];
    let leaf_chars = ['◠', '◡', '·', '∙', '°'];

    for pair in stops.windows(2) {
        let (x0, y0) = pair[0];
        let (x1, y1) = pair[1];
        let dx = x1 as f32 - x0 as f32;
        let dy = y1 as f32 - y0 as f32;
        let steps = (dx.abs().max(dy.abs())) as usize;
        if steps == 0 { continue; }

        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            // Slight sinuous wobble
            let wobble = (t * 8.0 * std::f32::consts::PI).sin() * 1.2;
            let perp_x = -dy / (dx.abs() + dy.abs()).max(1.0);
            let perp_y = dx / (dx.abs() + dy.abs()).max(1.0);
            let x = (x0 as f32 + dx * t + perp_x * wobble) as usize;
            let y = (y0 as f32 + dy * t + perp_y * wobble) as usize;
            if x >= w || y >= h { continue; }

            let ch = if dx.abs() < 1.0 { '│' }
                     else if dy.abs() / dx.abs() < 0.3 { '─' }
                     else if (dx > 0.0) == (dy > 0.0) { '╲' }
                     else { '╱' };
            grid[y][x] = Cell::new(ch, color);

            // Branch off leaf/bud every 5-8 steps
            if i % rng.random_range(5..9) == 0 {
                let lx = (x as i32 + rng.random_range(-2..3i32)).clamp(0, w as i32 - 1) as usize;
                let ly = (y as i32 + rng.random_range(-1..2i32)).clamp(0, h as i32 - 1) as usize;
                if grid[ly][lx].ch == ' ' {
                    let lch = leaf_chars[rng.random_range(0..leaf_chars.len())];
                    grid[ly][lx] = Cell::new(lch, lighten(color, 30));
                }
            }
        }
    }
}

/// River path: wider line using water-like chars.
fn draw_river_path(
    grid: &mut Grid,
    stops: &[(usize, usize)],
    color: Color,
    rng: &mut StdRng,
) {
    let h = grid.len();
    if h == 0 { return; }
    let w = grid[0].len();
    let water_chars = ['~', '≈', '∿', '─', '╌'];

    for pair in stops.windows(2) {
        let (x0, y0) = pair[0];
        let (x1, y1) = pair[1];
        let dx = x1 as f32 - x0 as f32;
        let dy = y1 as f32 - y0 as f32;
        let steps = (dx.abs().max(dy.abs())) as usize;
        if steps == 0 { continue; }
        let perp_x = -dy / (dx.abs() + dy.abs()).max(1.0);
        let perp_y = dx / (dx.abs() + dy.abs()).max(1.0);

        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let cx = x0 as f32 + dx * t;
            let cy = y0 as f32 + dy * t;

            // Width of 3 cells perpendicular to flow
            for offset in -1..=1i32 {
                let px = (cx + perp_x * offset as f32).round() as usize;
                let py = (cy + perp_y * offset as f32).round() as usize;
                if px >= w || py >= h { continue; }
                let ch = water_chars[rng.random_range(0..water_chars.len())];
                let c = if offset == 0 { color } else { darken(color, 30) };
                grid[py][px] = Cell::new(ch, c);
            }
        }
    }
}

/// Double-line path: uses double box-drawing chars for a bolder connection.
fn draw_double_path(
    grid: &mut Grid,
    stops: &[(usize, usize)],
    color: Color,
) {
    let h = grid.len();
    if h == 0 { return; }
    let w = grid[0].len();

    for pair in stops.windows(2) {
        let (x0, y0) = pair[0];
        let (x1, y1) = pair[1];
        let dx = x1 as f32 - x0 as f32;
        let dy = y1 as f32 - y0 as f32;
        let steps = (dx.abs().max(dy.abs())) as usize;
        if steps == 0 { continue; }

        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let x = (x0 as f32 + dx * t) as usize;
            let y = (y0 as f32 + dy * t) as usize;
            if x >= w || y >= h { continue; }

            let ch = if dx.abs() < 1.0 { '║' }
                     else if dy.abs() / dx.abs() < 0.3 { '═' }
                     else { '║' }; // double lines only have H/V
            grid[y][x] = Cell::new(ch, color);
        }
    }
}
