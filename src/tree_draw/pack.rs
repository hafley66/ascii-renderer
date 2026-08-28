//! Dispatch + space-packing engine.
use super::*;
/// Grow any archetype by index (mod TREE_KIND_COUNT). DRY replacement for the
/// per-mode `match kind % N { ... }` blocks.
pub fn grow_tree_by_index(idx: usize, grid: &mut Grid, params: &TreeParams, rng: &mut StdRng) {
    match idx % TREE_KIND_COUNT {
        0 => SpiralTree.grow(grid, params, rng),
        1 => CandelabraTree.grow(grid, params, rng),
        2 => SplitTree.grow(grid, params, rng),
        3 => BirchTree.grow(grid, params, rng),
        4 => WavyBirch.grow(grid, params, rng),
        5 => StormTree::new().grow(grid, params, rng),
        6 => DeadTree.grow(grid, params, rng),
        7 => DroopingTree.grow(grid, params, rng),
        8 => PineTree.grow(grid, params, rng),
        9 => WillowTree.grow(grid, params, rng),
        10 => PalmTree.grow(grid, params, rng),
        11 => WideTree.grow(grid, params, rng),
        12 => AsymmetricTree.grow(grid, params, rng),
        13 => KaijuTree.grow(grid, params, rng),
        14 => ZigzagTree.grow(grid, params, rng),
        15 => BrailleCanopyTree.grow(grid, params, rng),
        16 => TendrilTree.grow(grid, params, rng),
        17 => OakTree.grow(grid, params, rng),
        18 => FountainTree.grow(grid, params, rng),
        19 => WindsweptTree::new(rng).grow(grid, params, rng),
        20 => FractalTree.grow(grid, params, rng),
        21 => LSystemTree.grow(grid, params, rng),
        22 => DragonTree.grow(grid, params, rng),
        _ => HelixTree.grow(grid, params, rng),
    }
}
/// Tuning for the space-packing layout engine.
pub struct PackOpts {
    /// depth bands; layer 0 = back (small/faint), last = front (large/detailed)
    pub layer_count: u8,
    /// 0.0..0.6 -- fraction by which neighboring canopies interleave horizontally
    pub overlap: f32,
    /// 0.0..1.0 -- probability a tree gets a bole base
    pub bole_rate: f32,
    /// 0.2..0.8 -- fraction of canvas height reserved as ground
    pub ground_frac: f32,
    /// restrict archetype pool (None = all TREE_KIND_COUNT)
    pub kind_filter: Option<&'static [usize]>,
}
impl Default for PackOpts {
    fn default() -> Self {
        PackOpts {
            layer_count: 3,
            overlap: 0.25,
            bole_rate: 0.4,
            ground_frac: 0.45,
            kind_filter: None,
        }
    }
}
pub struct PackedSlot {
    pub plot: Rect,
    pub layer: u8,
    pub hue: f64,
    pub energy: f32,
    pub kind: usize,
    pub bole: Option<Bole>,
    pub taper: TaperKind,
    pub root_y: usize,
}
/// Tile the canvas with depth-layered tree plots so every column is covered.
/// Returns (ground_y, slots) with slots sorted back-to-front.
///
/// Coverage strategy: each layer walks x=0..width placing trees whose canopies
/// interleave by `overlap`. Layer index raises both root_y (closer = lower) and
/// canopy height (closer = taller), producing an aerial-perspective tier wall.
pub fn pack_forest(
    width: usize,
    height: usize,
    rng: &mut StdRng,
    opts: &PackOpts,
) -> (usize, Vec<PackedSlot>) {
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

    let sky = ground_y;
    let band = (sky / layer_count).max(1);
    let mut slots: Vec<PackedSlot> = Vec::new();

    for li in 0..layer_count {
        let lfrac = if layer_count > 1 {
            li as f32 / (layer_count - 1) as f32
        } else {
            1.0
        };

        // slot width grows toward the front (closer trees are wider)
        let slot_min = (6 + (lfrac * 6.0) as usize).max(4);
        let slot_max = (slot_min + 8 + (lfrac * 10.0) as usize).min(width / 2).max(slot_min);

        // canopy reaches higher toward the front
        let canopy_top = ((sky as i32) - (band as i32) * (li as i32 + 1)).max(1) as usize;

        // roots step downward toward the front (closer sits lower on screen)
        let root_y = (ground_y + li * (height - ground_y) / layer_count.max(1))
            .min(height.saturating_sub(2))
            .max(ground_y);

        let base_energy = 0.40 + lfrac * 0.55;

        let mut x = rng.random_range(0..slot_min as u32) as i32;
        while x < width as i32 {
            let slot_w = rng.random_range(slot_min as u32..=slot_max as u32) as usize;
            let cx = (x + slot_w as i32 / 2).clamp(2, width as i32 - 3) as usize;
            let plot_w = (slot_w + 4).min(width);
            let plot_x = cx.saturating_sub(plot_w / 2);
            let plot_h = root_y.saturating_sub(canopy_top) + 3;
            let plot = Rect {
                x: plot_x,
                y: canopy_top,
                w: plot_w,
                h: plot_h,
            };

            let kind = match opts.kind_filter {
                Some(set) => set[rng.random_range(0..set.len() as u32) as usize],
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
            let hue = rng.random_range(0..360u32) as f64;

            slots.push(PackedSlot {
                plot,
                layer: li as u8,
                hue,
                energy: base_energy,
                kind,
                bole,
                taper,
                root_y,
            });

            let step = ((slot_w as f32) * (1.0 - opts.overlap.clamp(0.0, 0.6))).max(2.0) as i32;
            x += step;
        }
    }

    // back-to-front: lower layer first, then lower root_y within a layer
    slots.sort_by(|a, b| a.layer.cmp(&b.layer).then(a.root_y.cmp(&b.root_y)));
    (ground_y, slots)
}


