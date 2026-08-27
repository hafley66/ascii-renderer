//! Scene-walk placement.
use crate::color::*;
use crate::sprites::*;
use crate::types::*;
use crossterm::style::Color;
use rand::RngExt;
use rand::rngs::StdRng;
use std::cell::Cell;
use super::*;

/// One placed scene element with per-stop randomized params.
/// Trees carry their OWN energy/height/spread (no per-layer banding).
#[derive(Clone, Copy)]
pub enum SceneEl {
    Tree {
        kind: usize,
        energy: f32,
        spread: usize,
        tree_h: usize,
        bole: Option<Bole>,
        taper: TaperKind,
    },
    Bush {
        style: usize,
        bush_w: i32,
        fade: u8,
    },
    Flowers,
    FruitVine,
    Grass,
    /// intentional negative space -- keeps the scene from being jammed to 11
    Gap,
}
pub struct SceneStop {
    pub x: usize,
    pub root_y: usize,
    pub layer: u8,
    pub hue: f64,
    pub sat: f32,
    pub light: f32,
    pub el: SceneEl,
}
pub struct SceneOpts {
    pub layer_count: u8,
    /// 0.1..1.0 -- shortens hops; higher = more stops (denser)
    pub density: f32,
    /// fraction of stops that become trees (rest split among bush/flower/vine/grass/gap)
    pub tree_rate: f32,
    pub bole_rate: f32,
    pub ground_frac: f32,
    pub kind_filter: Option<&'static [usize]>,
    pub vines: bool,
    /// degrees of per-stop hue jitter around a single per-scene base hue (coherent palette)
    pub hue_range: f32,
}
impl Default for SceneOpts {
    fn default() -> Self {
        SceneOpts {
            layer_count: 3,
            density: 0.4,
            tree_rate: 0.6,
            bole_rate: 0.4,
            ground_frac: 0.42,
            kind_filter: None,
            vines: true,
            hue_range: 32.0,
        }
    }
}
/// Walk the terrain placing mixed scene elements with high per-stop variance.
///
/// Entropy levers (vs the banded `pack_forest`):
/// - per-tree energy/height/spread sampled from WIDE, layer-overlapping ranges
/// - irregular hop distances + occasional gaps (no regular rows)
/// - mixed element vocabulary: trees, bushes, flowers, fruit vines, grass, gaps
/// - per-stop hue/sat/light jitter (no per-layer color banding)
/// Layers still bias size/color by depth, but distributions overlap so layers blur.
pub fn scene_walk(
    width: usize,
    height: usize,
    rng: &mut StdRng,
    opts: &SceneOpts,
) -> (usize, Vec<SceneStop>) {
    use rand::Rng;
    let layer_count = opts.layer_count.clamp(1, 6) as usize;
    let ground_y = ((height as f32 * opts.ground_frac.clamp(0.2, 0.8)) as usize).max(2);
    let all_tapers = [
        TaperKind::Diagonal,
        TaperKind::Shelf,
        TaperKind::Bracket,
        TaperKind::Step,
        TaperKind::Melt,
    ];

    // undulating horizon so roots don't sit on a straight line
    let mut ground_heights: Vec<usize> = Vec::with_capacity(width);
    let mut gh = ground_y as i32;
    for _ in 0..width {
        gh += rng.random_range(0..3u32) as i32 - 1;
        gh = gh.clamp(ground_y as i32 - 3, ground_y as i32 + 3);
        ground_heights.push(gh.max(1) as usize);
    }

    let ground_depth = (height - ground_y).max(1);
    let hop_mul = (1.25 - opts.density.clamp(0.1, 1.0)).max(0.15);
    let tree_rate = opts.tree_rate.clamp(0.1, 0.95);
    let hue_drift = opts.hue_range.clamp(0.0, 180.0);
    // one base hue per scene -> coherent palette, variety lives across seeds
    let scene_hue = rng.random_range(0..360u32) as f64;
    let mut stops: Vec<SceneStop> = Vec::new();

    for li in 0..layer_count {
        let lfrac = if layer_count > 1 {
            li as f32 / (layer_count - 1) as f32
        } else {
            0.6
        };

        // root baseline: back near horizon, front lower -- but root_y jitters per stop
        let root_base = (ground_y as f32 + lfrac * ground_depth as f32 * 0.75) as usize;

        // overlapping canopy-height ranges per layer (overlap = entropy)
        let (h_lo, h_hi) = match li {
            0 => (3u32, 14u32),
            1 => (8, 30),
            _ => (16, 48),
        };
        // hop distances grow toward the front
        let (hop_lo, hop_hi) = match li {
            0 => (3u32, 9u32),
            1 => (5, 14),
            _ => (7, 18),
        };

        let mut x = rng.random_range(0..hop_lo) as usize;
        while x < width {
            let col = x.min(width - 1);
            let ghere = ground_heights[col];
            let root_y = (root_base + ghere.saturating_sub(ground_y))
                .saturating_add(rng.random_range(0..3u32) as usize)
                .min(height.saturating_sub(2));

            // per-stop color: narrow drift around the scene base hue + sat/light depth bias
            let hue = (scene_hue + rng.random_range(-hue_drift..hue_drift) as f64).rem_euclid(360.0);
            let sat = (0.20 + lfrac * 0.35 + rng.random_range(-0.08f32..0.08f32)).clamp(0.08, 0.85);
            let light =
                (0.12 + lfrac * 0.22 + rng.random_range(-0.05f32..0.06f32)).clamp(0.08, 0.50);

            let r = rng.random::<f32>();
            let el = if r < tree_rate {
                // per-tree energy drawn from a wide band, only gently biased by depth
                let energy = (0.30 + lfrac * 0.45 + rng.random_range(-0.20f32..0.25f32))
                    .clamp(0.20, 1.0);
                let tree_h = rng.random_range(h_lo..=h_hi) as usize;
                // spread tracks THIS tree's height so canopies stay proportional
                let spread = (tree_h as f32 * rng.random_range(0.40f32..0.80)).max(1.0) as usize + 1;
                let kind = match opts.kind_filter {
                    Some(s) => s[rng.random_range(0..s.len() as u32) as usize],
                    None => rng.random_range(0..TREE_KIND_COUNT as u32) as usize,
                };
                let bole = if rng.random::<f32>() < opts.bole_rate.clamp(0.0, 1.0) {
                    Some(Bole {
                        style: rng.random_range(0..10u32) as usize,
                    })
                } else {
                    None
                };
                let taper = all_tapers[rng.random_range(0..all_tapers.len() as u32) as usize];
                SceneEl::Tree {
                    kind,
                    energy,
                    spread,
                    tree_h,
                    bole,
                    taper,
                }
            } else if r < tree_rate + 0.12 {
                SceneEl::Bush {
                    style: rng.random_range(0..18u32) as usize,
                    bush_w: rng.random_range(3..9u32) as i32,
                    fade: rng.random_range(0..3u32) as u8,
                }
            } else if r < tree_rate + 0.24 {
                SceneEl::Flowers
            } else if opts.vines && r < tree_rate + 0.32 {
                SceneEl::FruitVine
            } else if r < tree_rate + 0.44 {
                SceneEl::Grass
            } else {
                SceneEl::Gap
            };

            stops.push(SceneStop {
                x: col,
                root_y,
                layer: li as u8,
                hue,
                sat,
                light,
                el,
            });

            let hop = ((rng.random_range(hop_lo..=hop_hi) as f32) * hop_mul).max(1.0) as usize;
            // occasional open gap in the spacing
            let extra = if rng.random::<f32>() < 0.15 {
                rng.random_range(2..8u32) as usize
            } else {
                0
            };
            x += hop + extra;
        }
    }

    // back-to-front, then lower-rooted within a layer drawn later (closer)
    stops.sort_by(|a, b| a.layer.cmp(&b.layer).then(a.root_y.cmp(&b.root_y)));
    (ground_y, stops)
}
