//! arboretum -- one-shot grove: TreeGenome (10 per-tree knobs) + ForestKnobs
//! (10 grove knobs via param_f32), scaling 3-row saplings to full-screen ancients.

use crate::color::*;
use crate::opts::param_f32;
use crate::pp::ease_in_out;
use crate::sprites::{MoveDir, TreePen};
use crate::tree_draw::{Bole, BoleExit, BoleStyle, GrowDir, TaperKind, TreeParams};
use crate::types::*;
use crossterm::style::Color;
use rand::RngExt;
use rand::SeedableRng;
use rand::rngs::StdRng;

// ── Tree genome: the 10 per-tree knobs ──────────────────────────────

mod species;
pub use species::*;

#[derive(Clone, Copy, PartialEq)]
pub enum TreeStyle {
    Classic,   // original alternating-bough grower (grow_tree body)
    Conifer,   // tiered whorls, needles, cones
    Broadleaf, // short trunk, wide dome of clustered leaves
    Willow,    // crown of falling strands
    Cypress,   // tight flame column
    Babel,     // sane trunk that loses its mind as it climbs
    Pleach,    // trained to one ceiling: every tip meets the same row
    Uzumaki,   // spiral horror: helix trunk tightening into a vortex crown
}

pub struct TreeGenome {
    pub vigor: f32,          // 0.15..1.0 -- energy: trunk fraction, tip count
    pub bole: Option<usize>, // root style 0..34 -- initial conditions
    pub taper: TaperKind,    // base cap shape
    pub gnarl: f32,          // 0..1 -- trunk wobble + branch droop
    pub lean: f32,           // -1..1 -- steady wind bias
    pub boughs: f32,         // 0..1 -- branch density along the trunk
    pub spread: f32,         // 0.25..1.5 -- canopy width ratio
    pub orders: u8,          // 1..=4 -- branching recursion (venation)
    pub leafage: f32,        // 0..1 -- leaf density on outer twigs
    pub fruition: f32,       // 0..1 -- fruit chance per tip
    pub style: TreeStyle,    // growth habit (picks the renderer)
}

/// Roll a style: `mix` 0 = all Classic, 1 = never Classic.
pub fn roll_style(rng: &mut StdRng, mix: f32) -> TreeStyle {
    if rng.random::<f32>() > mix {
        return TreeStyle::Classic;
    }
    match rng.random_range(0..7u32) {
        0 => TreeStyle::Conifer,
        1 => TreeStyle::Broadleaf,
        2 => TreeStyle::Willow,
        3 => TreeStyle::Cypress,
        4 => TreeStyle::Babel,
        5 => TreeStyle::Pleach,
        _ => TreeStyle::Uzumaki,
    }
}

impl TreeGenome {
    pub fn roll(rng: &mut StdRng) -> Self {
        let taper = [
            TaperKind::Diagonal,
            TaperKind::Shelf,
            TaperKind::Bracket,
            TaperKind::Step,
            TaperKind::Melt,
        ][rng.random_range(0..5) as usize];
        TreeGenome {
            vigor: rng.random_range(0.25..1.0),
            bole: if rng.random_range(0..10u32) < 6 {
                Some(rng.random_range(0..28) as usize)
            } else {
                None
            },
            taper,
            gnarl: rng.random::<f32>() * 0.9,
            lean: rng.random::<f32>() * 2.0 - 1.0,
            boughs: rng.random_range(0.25..1.0),
            spread: rng.random_range(0.3..1.3),
            orders: rng.random_range(1..=4),
            leafage: rng.random::<f32>(),
            fruition: rng.random::<f32>() * 0.8,
            style: TreeStyle::Classic,
        }
    }
}

struct TreeColors {
    trunk: Color,
    branch: Color,
    leaf: Color,
    fruit: Color,
}

fn set(grid: &mut Grid, x: i32, y: i32, ch: char, fg: Color) {
    if x >= 0 && y >= 0 && (y as usize) < grid.len() && (x as usize) < grid[0].len() {
        grid[y as usize][x as usize] = Cell::new(ch, fg);
    }
}

fn blank_at(grid: &Grid, x: i32, y: i32) -> bool {
    x >= 0
        && y >= 0
        && (y as usize) < grid.len()
        && (x as usize) < grid[0].len()
        && grid[y as usize][x as usize].ch == ' '
}

// ── Grower ──────────────────────────────────────────────────────────

struct GrowCtx<'a> {
    grid: &'a mut Grid,
    rng: &'a mut StdRng,
    cols: &'a TreeColors,
    genome: &'a TreeGenome,
    leaf_cells: Vec<(i32, i32)>, // terminal bough cells eligible for leaves
    tips: Vec<(i32, i32)>,       // stem tips eligible for fruit
}

/// Grow one tree rooted at (rx, ry) with `budget` rows of vertical space
/// (3 = sapling, full height = ancient); grow=1, foliage=1, sway=0 is the static tree.
pub fn grow_tree(grid: &mut Grid, rx: i32, ry: i32, budget: i32, genome: &TreeGenome, cols: &TreeColors, rng: &mut StdRng, grow: f32, foliage: f32, sway: f32) {
    if genome.style != TreeStyle::Classic {
        species::grow_species(grid, rx, ry, budget, genome, cols, rng, grow, foliage, sway);
        return;
    }
    let mut genome = TreeGenome {
        vigor: genome.vigor,
        bole: genome.bole,
        taper: genome.taper,
        gnarl: genome.gnarl,
        lean: (genome.lean + sway).clamp(-1.0, 1.0),
        boughs: genome.boughs,
        spread: genome.spread,
        orders: genome.orders,
        leafage: genome.leafage * foliage,
        fruition: genome.fruition * foliage,
        style: TreeStyle::Classic,
    };
    let budget = (3.0 + (budget.max(2) as f32 - 3.0) * ease_in_out(grow.clamp(0.0, 1.0))) as i32;
    let budget = budget.max(2);
    let trunk_len = ((budget as f32) * (0.35 + 0.45 * genome.vigor)).max(2.0) as i32;

    // 1. Initial conditions: bole flare + taper cap into the trunk base.
    let mut start = (rx, ry);
    if budget >= 7 {
        if let Some(style) = genome.bole {
            let pw = 13usize;
            let plot = Rect {
                x: (rx - pw as i32 / 2).max(0) as usize,
                y: (ry - budget).max(0) as usize,
                w: pw,
                h: budget.min(ry + 1).max(1) as usize,
            };
            let tp = TreeParams {
                plot,
                energy: genome.vigor,
                trunk_color: cols.trunk,
                bark_color: darken(cols.trunk, 15),
                branch_color: cols.branch,
                tip_color: lighten(cols.branch, 30),
                fruit_color: cols.fruit,
                fruit_factor: 0.0,
                branch_factor: genome.boughs,
                direction: GrowDir::Up,
                bole: None,
                taper: genome.taper,
            };
            let bole = Bole { style };
            let exit = bole.draw(grid, &tp, rng);
            start = (exit.x, exit.y);
        }
    }
    let (tx, ty) = if start == (rx, ry) {
        set(grid, rx, ry, '│', cols.trunk);
        start
    } else {
        draw_taper_pub(grid, &BoleExit::point(start.0, start.1), cols.trunk, genome.taper)
    };

    let mut ctx = GrowCtx {
        grid,
        rng,
        cols,
        genome: &genome,
        leaf_cells: Vec::new(),
        tips: Vec::new(),
    };

    // 2. Trunk: upward walk with gnarl wobble and steady lean.
    let mut pen = TreePen::new(tx, ty, cols.trunk);
    pen.last_dir = Some(MoveDir::Up);
    let mut nodes: Vec<(i32, i32, MoveDir)> = Vec::new();
    let lean = genome.lean;
    for _ in 0..trunk_len {
        let r: f32 = ctx.rng.random();
        let dir = if r < genome.gnarl * 0.28 {
            // wobble step, biased by the lean
            if ctx.rng.random::<f32>() < 0.5 + lean * 0.35 {
                MoveDir::UpRight
            } else {
                MoveDir::UpLeft
            }
        } else if lean.abs() > 0.65 && ctx.rng.random::<f32>() < lean.abs() * 0.22 {
            if lean > 0.0 { MoveDir::Right } else { MoveDir::Left }
        } else {
            MoveDir::Up
        };
        pen.step(ctx.grid, dir);
        nodes.push((pen.x, pen.y, dir));
    }
    // Thick base for big trees: flank the lower trunk.
    if budget >= 24 && trunk_len >= 8 {
        let flank_rows = (trunk_len / 3).max(2);
        let base_y = ty - 0;
        for k in 0..flank_rows {
            let y = base_y - k;
            // trace the trunk column at this row (nodes are in grow order)
            if let Some(&(nx, _, _)) = nodes.get(k as usize) {
                set(ctx.grid, nx - 1, y, '┆', darken(cols.trunk, 10));
                set(ctx.grid, nx + 1, y, '┆', darken(cols.trunk, 10));
            }
        }
    }

    // 3. Boughs off trunk nodes, plus an apex fork.
    let interval = ((6.0 - genome.boughs * 4.0).max(2.0)) as usize; // 2..6
    let mut next_slot = 2usize;
    let mut side: i32 = if ctx.rng.random::<f32>() < 0.5 { 1 } else { -1 };
    let n = nodes.len();
    for (i, &(nx, ny, ndir)) in nodes.iter().enumerate() {
        if i + 1 < n && i >= next_slot && ctx.rng.random::<f32>() < genome.boughs {
            let remain = (n - i) as f32 / n.max(1) as f32;
            let blen = ((budget as f32) * 0.22 * genome.spread * (0.5 + remain)).max(2.0) as i32;
            grow_bough(&mut ctx, nx, ny, ndir, side, 1, blen);
            side = -side;
            next_slot = i + interval;
        }
    }
    // Apex: fork or plain tip.
    if let Some(&(ax, ay, adir)) = nodes.last() {
        if budget >= 6 && genome.boughs > 0.3 {
            let blen = ((budget as f32) * 0.2 * genome.spread).max(2.0) as i32;
            grow_bough(&mut ctx, ax, ay, adir, 1, 1, blen);
            grow_bough(&mut ctx, ax, ay, adir, -1, 1, blen);
        } else {
            set(ctx.grid, ax, ay, '╷', lighten(cols.branch, 30));
            ctx.tips.push((ax, ay));
        }
    }

    // 4. Leaves along terminal wood.
    let leaf_glyphs = ['✿', '❀', '❉', '❋', '✳', '✶', '⠿', '⣿'];
    for &(x, y) in &ctx.leaf_cells {
        if ctx.rng.random::<f32>() < ctx.genome.leafage * 0.38 {
            let dx = if ctx.rng.random::<f32>() < 0.5 { -1 } else { 1 };
            let g = leaf_glyphs[ctx.rng.random_range(0..leaf_glyphs.len() as u32) as usize];
            let c = if ctx.rng.random::<f32>() < 0.5 {
                ctx.cols.leaf
            } else {
                lighten(ctx.cols.leaf, 20)
            };
            if blank_at(ctx.grid, x + dx, y) {
                set(ctx.grid, x + dx, y, g, c);
            } else if blank_at(ctx.grid, x, y - 1) {
                set(ctx.grid, x, y - 1, g, c);
            }
        }
    }

    // 5. Fruit hanging at stem tips.
    let fruit_glyphs = ['◉', '●', '◍', '◆', '✦', '❉'];
    for &(x, y) in &ctx.tips {
        if ctx.rng.random::<f32>() < ctx.genome.fruition * 0.8 {
            let g = fruit_glyphs[ctx.rng.random_range(0..fruit_glyphs.len() as u32) as usize];
            let dy = if ctx.rng.random::<f32>() < 0.7 { 1 } else { 0 };
            if blank_at(ctx.grid, x, y + dy) {
                set(ctx.grid, x, y + dy, g, ctx.cols.fruit);
            } else {
                set(ctx.grid, x, y, g, ctx.cols.fruit);
            }
        }
    }
}

/// Recursive bough: walk outward-up from (x,y), forking by `orders`.
fn grow_bough(ctx: &mut GrowCtx, x: i32, y: i32, from: MoveDir, side: i32, order: u8, len: i32) {
    let mut pen = TreePen::new(x, y, ctx.cols.branch);
    pen.last_dir = Some(from);
    let mut dir = if side > 0 { MoveDir::UpRight } else { MoveDir::UpLeft };
    let mut cells: Vec<(i32, i32)> = Vec::new();
    for k in 0..len.max(1) {
        // curvature: steepen with height, gnarl droops the tail
        let r: f32 = ctx.rng.random();
        dir = if r < 0.22 {
            MoveDir::Up // steepen toward the light
        } else if r < 0.22 + ctx.genome.gnarl * 0.18 && k > len / 2 {
            if side > 0 { MoveDir::DownRight } else { MoveDir::DownLeft } // droop
        } else if r < 0.5 + 0.3 * (1.0 - ctx.genome.spread) && ctx.rng.random::<f32>() < 0.3 {
            if side > 0 { MoveDir::Right } else { MoveDir::Left } // flatten wide canopies
        } else {
            dir
        };
        pen.step(ctx.grid, dir);
        cells.push((pen.x, pen.y));
    }
    let (ex, ey) = (pen.x, pen.y);
    if order < ctx.genome.orders && len >= 3 {
        // fork: keep the side, plus one re-curving child back over the trunk
        let sub = ((len as f32) * (0.5 + ctx.rng.random::<f32>() * 0.2)).max(2.0) as i32;
        grow_bough(ctx, ex, ey, dir, side, order + 1, sub);
        if ctx.rng.random::<f32>() < 0.75 {
            grow_bough(ctx, ex, ey, dir, -side, order + 1, (sub * 3 / 4).max(2));
        }
    } else {
        // terminal stem: one shy step then a tip
        let stem_dir = if side > 0 { MoveDir::UpRight } else { MoveDir::UpLeft };
        pen.step(ctx.grid, stem_dir);
        set(ctx.grid, pen.x, pen.y, '╷', lighten(ctx.cols.branch, 30));
        ctx.tips.push((pen.x, pen.y));
        // terminal wood carries the leaves
        for c in cells.iter() {
            ctx.leaf_cells.push(*c);
        }
    }
}

fn draw_taper_pub(grid: &mut Grid, exit: &BoleExit, color: Color, kind: TaperKind) -> (i32, i32) {
    // Thin wrapper so the taper cap is drawn even for point exits (no bole).
    let e = BoleExit {
        x: exit.x,
        y: exit.y,
        left: exit.left.max(if exit.left == 0 && exit.right == 0 { 0 } else { exit.left }),
        right: exit.right,
    };
    crate::tree_draw::draw_taper(grid, &e, color, kind)
}

// ── Forest knobs (env-tunable) ──────────────────────────────────────

pub struct ForestKnobs {
    pub density: f32,   // tree stops across the screen
    pub strata: u8,     // depth planes 1..=4
    pub girth: f32,     // size span: 0.3 sapling-heavy .. 3 ancient-heavy
    pub clumping: f32,  // 0 uniform .. 1 tight groves
    pub ferns: f32,     // undergrowth density
    pub relief: f32,    // ground ruggedness
    pub gale: f32,      // global wind lean added to every genome
    pub drift: f32,     // hue drift across the grove (degrees)
    pub haze: f32,      // atmospheric depth fade
    pub clearings: f32, // fraction of stops left open
    pub species_mix: f32, // 0 = classic only, 1 = no classic
}

impl ForestKnobs {
    pub fn from_env() -> Self {
        ForestKnobs {
            density: param_f32("DENS", 16.0).clamp(2.0, 60.0),
            strata: param_f32("STRATA", 3.0).round().clamp(1.0, 4.0) as u8,
            girth: param_f32("GIRTH", 1.2).clamp(0.3, 3.0),
            clumping: param_f32("CLUMP", 0.5).clamp(0.0, 1.0),
            ferns: param_f32("FERNS", 0.5).clamp(0.0, 1.0),
            relief: param_f32("RELIEF", 0.5).clamp(0.0, 1.0),
            gale: param_f32("GALE", 0.15).clamp(-1.0, 1.0),
            drift: param_f32("DRIFT", 60.0).clamp(-180.0, 180.0),
            haze: param_f32("HAZE", 0.45).clamp(0.0, 1.0),
            clearings: param_f32("CLEAR", 0.25).clamp(0.0, 1.0),
            species_mix: param_f32("SPECIES", 0.7).clamp(0.0, 1.0),
        }
    }
}

// ── Grove renderer ──────────────────────────────────────────────────

pub fn draw_arboretum(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    rng: &mut StdRng,
    t: f32,
    knobs: &ForestKnobs,
) {
    let strata = knobs.strata.max(1) as usize;
    // horizon: somewhere in the lower two-fifths
    let horizon = (height as f32 * (0.55 + rng.random::<f32>() * 0.15)) as usize;
    let sky_color = darken(palette[0], 95);

    // Sky: sparse stars
    for y in 0..horizon {
        for x in 0..width {
            if rng.random_range(0..18u32) == 0 {
                grid[y][x] = Cell::new('·', sky_color);
            }
        }
    }
    // Clouds
    let cloud_color = lighten(palette[0], 15);
    for _ in 0..rng.random_range(1..4u32) {
        let cx = rng.random_range(5..(width as u32 - 5).max(6)) as usize;
        let cy = rng.random_range(2..(horizon / 2).max(3) as u32) as usize;
        let cw = rng.random_range(8..20u32) as usize;
        crate::sprites::draw_cloud(grid, cx, cy, cw, cloud_color, rng);
    }

    // Per-column ground line: random walk scaled by relief
    let jitter = (1.0 + knobs.relief * 6.0) as i32;
    let mut ground: Vec<usize> = Vec::with_capacity(width);
    let mut gh = horizon as i32;
    for _ in 0..width {
        gh += rng.random_range(0..3u32) as i32 - 1;
        gh = gh.clamp(horizon as i32 - jitter, horizon as i32 + jitter);
        ground.push(gh.clamp(1, height as i32 - 1) as usize);
    }

    // Ground fill with a depth-graded hue
    let ground_chars = ['·', '·', '∙', ',', ',', '"', '~'];
    let base_hue: f64 = if let Color::Rgb { r, g, .. } = palette[1] {
        (r as f64 * 1.4 + g as f64 * 0.7) % 360.0
    } else {
        120.0
    };
    let ground_hue = base_hue * 0.3 + 95.0; // push toward forest greens
    for x in 0..width {
        for y in ground[x]..height {
            let depth = (y - ground[x]) as f64 / (height - ground[x]).max(1) as f64;
            let h = (ground_hue + x as f64 / width as f64 * knobs.drift as f64 * 0.5
                + depth * 18.0)
                .rem_euclid(360.0);
            let l = (0.16 + depth * 0.10).min(0.30);
            let c = hsl_to_rgb(h, 0.35, l);
            let ch = ground_chars[rng.random_range(0..ground_chars.len() as u32) as usize];
            grid[y][x] = Cell::new(ch, c);
        }
    }

    // Fog band hugging the horizon
    if knobs.haze > 0.05 {
        let fog = lighten(sky_color, 25);
        for y in (horizon.saturating_sub(1))..(horizon + 2).min(height) {
            for x in 0..width {
                if rng.random::<f32>() < knobs.haze * 0.18
                    && (y as i32 - ground[x] as i32).abs() <= 1
                {
                    grid[y][x] = Cell::new('▒', fog);
                }
            }
        }
    }

    // Per-layer tree pass: far strata first (smaller, dimmer), near last.
    // t drives per-tree life cycle (sprout -> hold -> wither) + sway; t==0 is static.
    let speed = param_f32("SPEED", 1.0).clamp(0.1, 3.0);
    let animating = t > 0.0;
    let tree_base_hue = ground_hue + rng.random_range(-25..25) as f64;
    for layer in 0..strata {
        let lfrac = if strata == 1 { 1.0 } else { layer as f32 / (strata - 1) as f32 };
        let depth_step = ((height - horizon) / strata.max(1)).max(1);

        // Stops for this layer
        let stop_count = ((knobs.density / strata as f32).ceil() as usize).max(2);
        let base_hop = width as f32 / stop_count as f32;
        let mut wx = rng.random_range(3..(width as u32 - 3).max(4)) as usize;
        for si in 0..stop_count {
            if si > 0 {
                let hop = if rng.random::<f32>() < knobs.clumping * 0.45 {
                    rng.random_range(2..5u32) as usize // cluster tight
                } else {
                    (base_hop as f64 * rng.random_range(70..170) as f64 / 100.0) as usize
                };
                wx = (wx + hop.max(1)) % width.max(1);
                wx = wx.clamp(3, width.saturating_sub(4).max(3));
            }

            let col = ground[wx.min(width - 1)];
            let root_y = (col + layer * depth_step).min(height - 2);

            // Clearing: skip the tree, scatter ground flora instead.
            if rng.random::<f32>() < knobs.clearings {
                for _ in 0..rng.random_range(2..6u32) {
                    let fx = wx as i32 + rng.random_range(-4..5i32);
                    let fy = root_y as i32 - rng.random_range(0..2i32);
                    let c = hsl_to_rgb(
                        (tree_base_hue + rng.random_range(-30..60) as f64).rem_euclid(360.0),
                        0.5,
                        0.3,
                    );
                    let g = ['✿', '❀', '*', '❉'][rng.random_range(0..4) as usize];
                    if blank_at(grid, fx, fy) {
                        set(grid, fx, fy, g, c);
                    }
                }
                continue;
            }

            // Size roll: girth biases the distribution toward ancients or saplings.
            let s_max = (((root_y - 1) as f32) * (0.35 + 0.65 * lfrac)).max(3.0) as i32;
            let u: f32 = rng.random::<f32>();
            let shape = 1.0 / knobs.girth.clamp(0.3, 3.0);
            let mut size = 3.0 + (s_max as f32 - 3.0) * u.powf(shape);
            // One champion per near layer, when there is room for an ancient.
            if layer == strata - 1 && si == stop_count / 2 && s_max >= 14 {
                size = s_max as f32;
            }
            let budget = size.round().clamp(3.0, (root_y - 1).max(3) as f32) as i32;

            // Per-tree rng: genome + growth draw from a stream keyed to
            // (seed, layer, si), so one tree's growth never re-rolls its neighbors.
            let mut trng = StdRng::seed_from_u64(
                seed ^ (layer as u64).wrapping_mul(0x9E37_79B9)
                    ^ (si as u64).wrapping_mul(0x85EB_CA6B),
            );
            let phase = trng.random::<f32>();
            let cycle = 70.0 + trng.random::<f32>() * 40.0;
            let (grow, foliage) = if animating {
                let p = ((t * speed) / cycle + phase).fract();
                let g = if p < 0.45 {
                    ease_in_out(p / 0.45)
                } else if p < 0.92 {
                    1.0
                } else {
                    1.0 - ease_in_out((p - 0.92) / 0.08)
                };
                let f = if p < 0.35 {
                    ease_in_out(p / 0.35)
                } else if p < 0.85 {
                    1.0
                } else {
                    1.0 - ease_in_out((p - 0.85) / 0.15)
                };
                (g, f)
            } else {
                (1.0, 1.0)
            };
            let sway = if animating {
                knobs.gale * 0.3 * (t * speed * 0.5 + phase * std::f32::consts::TAU).sin()
            } else {
                0.0
            };
            let mut genome = TreeGenome::roll(&mut trng);
            genome.style = roll_style(&mut trng, knobs.species_mix);
            genome.lean = (genome.lean + knobs.gale).clamp(-1.0, 1.0);
            let hue = (tree_base_hue
                + wx as f64 / width as f64 * knobs.drift as f64
                + lfrac as f64 * knobs.drift as f64 * 0.25)
                .rem_euclid(360.0);
            let dim = 1.0 - knobs.haze as f64 * 0.55 * (1.0 - lfrac as f64);
            let sat = (0.30 + 0.30 * lfrac as f64) * dim;
            let light = (0.14 + 0.20 * lfrac as f64) * (0.6 + 0.4 * dim);
            let trunk = hsl_to_rgb((hue - 20.0).rem_euclid(360.0), sat * 0.7, (light * 0.75).max(0.08));
            let branch = hsl_to_rgb(hue, sat, light);
            let leaf = hsl_to_rgb((hue + 25.0).rem_euclid(360.0), (sat * 1.2).min(0.85), (light + 0.08).min(0.5));
            let fruit = hsl_to_rgb((hue + 120.0).rem_euclid(360.0), (sat * 1.3).min(0.9), (light + 0.14).min(0.55));
            let cols = TreeColors { trunk, branch, leaf, fruit };
            grow_tree(grid, wx as i32, root_y as i32, budget, &genome, &cols, &mut trng, grow, foliage, sway);
        }
    }

    // Undergrowth: swaying tufts + bush clusters along the ground line
    let tufts = [',', '"', ';', 'w', 'W', '⋏'];
    let bush_glyphs = ['⠿', '⣿', '❀', '✿', '✳'];
    let sway_idx = |x: usize, tt: f32, n: usize| {
        (((tt * 2.0 + x as f32 * 0.8).sin() * 0.5 + 0.5) * n as f32) as usize % n
    };
    let mut x = 0usize;
    while x < width {
        if rng.random::<f32>() < knobs.ferns * 0.22 {
            let y = ground[x].saturating_sub(1);
            let h = (ground_hue + rng.random_range(-20..40) as f64).rem_euclid(360.0);
            let c = hsl_to_rgb(h, 0.5, 0.22 + rng.random::<f64>() * 0.1);
            let bushy = knobs.ferns > 0.55 && rng.random::<f32>() < 0.35 && x + 4 < width;
            if bushy {
                let n = 3 + rng.random_range(0..3) as usize;
                for dx in 0..n {
                    let bx = x + dx;
                    let peak = (n as i32 / 2) - (dx as i32 - n as i32 / 2).abs();
                    let hgt = (peak + 1).clamp(1, 3) as usize;
                    for dy in 0..hgt {
                        let by = ground[bx].saturating_sub(1 + dy);
                        let g = bush_glyphs[sway_idx(bx, t, bush_glyphs.len())];
                        if blank_at(grid, bx as i32, by as i32) {
                            grid[by][bx] = Cell::new(g, c);
                        }
                    }
                }
                x += 4;
            } else {
                let g = tufts[sway_idx(x, t, tufts.len())];
                if blank_at(grid, x as i32, y as i32) {
                    grid[y][x] = Cell::new(g, c);
                }
            }
        }
        x += 1;
    }
}

// ── CLI dispatch arm ────────────────────────────────────────────────

pub(crate) fn cli_arboretum(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
    // arboretum [strata] [density] -- knob overrides win over env/defaults
    let mut knobs = ForestKnobs::from_env();
    if let Some(s) = args.get(4).and_then(|v| v.parse::<u8>().ok()) {
        if s >= 1 {
            knobs.strata = s.min(4);
        }
    }
    if let Some(d) = args.get(5).and_then(|v| v.parse::<f32>().ok()) {
        if d > 0.0 {
            knobs.density = d.clamp(2.0, 60.0);
        }
    }
    let _ = (term_w, term_h, mode, theme_name);
    draw_arboretum(&mut grid, width, height, seed, &palette, &mut rng, t_anim, &knobs);
    (grid, false)
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    fn make(w: usize, h: usize, seed: u64) -> (Grid, StdRng, [Color; 5]) {
        (
            vec![vec![Cell::blank(); w]; h],
            StdRng::seed_from_u64(seed),
            crate::color::make_palette(seed),
        )
    }

    fn plain(grid: &Grid) -> String {
        grid.iter()
            .map(|row| row.iter().map(|c| c.ch).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn snapshot_arboretum_tiny_grid() {
        let (mut g, mut r, p) = make(46, 14, 42);
        let knobs = ForestKnobs::from_env();
        draw_arboretum(&mut g, 46, 14, 42, &p, &mut r, 0.0, &knobs);
        insta::assert_snapshot!("arboretum_tiny_42", plain(&g));
    }

    #[test]
    fn snapshot_arboretum_standard() {
        let (mut g, mut r, p) = make(80, 24, 42);
        let knobs = ForestKnobs::from_env();
        draw_arboretum(&mut g, 80, 24, 42, &p, &mut r, 0.0, &knobs);
        insta::assert_snapshot!("arboretum_42", plain(&g));
    }

    #[test]
    fn snapshot_arboretum_huge_grid() {
        let (mut g, mut r, p) = make(150, 50, 42);
        let knobs = ForestKnobs::from_env();
        draw_arboretum(&mut g, 150, 50, 42, &p, &mut r, 0.0, &knobs);
        insta::assert_snapshot!("arboretum_huge_42", plain(&g));
    }

    #[test]
    fn deterministic_and_seed_sensitive() {
        let run = |seed: u64| {
            let (mut g, mut r, p) = make(80, 24, seed);
            let knobs = ForestKnobs::from_env();
            draw_arboretum(&mut g, 80, 24, seed, &p, &mut r, 0.0, &knobs);
            plain(&g)
        };
        assert_eq!(run(42), run(42), "same seed -> same grove");
        assert_ne!(run(42), run(7), "new seed -> new grove");
    }

    #[test]
    fn lifecycle_animates_and_t0_is_static() {
        let run = |t: f32| {
            let (mut g, mut r, p) = make(80, 24, 42);
            let knobs = ForestKnobs::from_env();
            draw_arboretum(&mut g, 80, 24, 42, &p, &mut r, t, &knobs);
            plain(&g)
        };
        assert_eq!(run(0.0), run(0.0), "t=0 deterministic");
        assert_ne!(run(0.0), run(2.5), "t should move the life cycle");
        assert_ne!(run(2.5), run(4.0), "cycle keeps advancing");
    }

    #[test]
    fn animation_frames_are_locally_stable() {
        // one player tick must nudge the scene, not re-roll it: per-tree rng
        // streams are keyed to (seed, layer, si) and never shift neighbors.
        let frame = |t: f32| {
            let (mut g, mut r, p) = make(80, 24, 42);
            let knobs = ForestKnobs::from_env();
            draw_arboretum(&mut g, 80, 24, 42, &p, &mut r, t, &knobs);
            g
        };
        let a = frame(30.0);
        let b = frame(30.6);
        let mut changed = 0usize;
        let mut total = 0usize;
        for (ra, rb) in a.iter().zip(b.iter()) {
            for (ca, cb) in ra.iter().zip(rb.iter()) {
                total += 1;
                if ca.ch != cb.ch {
                    changed += 1;
                }
            }
        }
        assert!(changed > 0, "a tick must move something");
        assert!(
            (changed as f64) / (total as f64) < 0.08,
            "tick changed {}/{} cells -- trees are re-rolling, not growing",
            changed,
            total
        );
    }

    #[test]
    fn single_tree_scales_tiny_to_ancient() {
        // A 3-budget sapling must stay small; a 40-budget tree must reach high.
        let sapling_cells = |budget: i32| {
            let (mut g, mut r, _) = make(60, 24, 5);
            let genome = TreeGenome::roll(&mut r);
            let cols = TreeColors {
                trunk: crate::color::rgb(90, 120, 60),
                branch: crate::color::rgb(90, 140, 60),
                leaf: crate::color::rgb(120, 180, 70),
                fruit: crate::color::rgb(200, 90, 60),
            };
            grow_tree(&mut g, 30, 22, budget, &genome, &cols, &mut r, 1.0, 1.0, 0.0);
            plain(&g)
                .lines()
                .filter(|l| !l.trim().is_empty())
                .count()
        };
        let tiny = sapling_cells(3);
        let huge = sapling_cells(22);
        assert!(tiny <= 8, "sapling occupies few rows, got {}", tiny);
        assert!(huge > tiny, "bigger budget must reach higher: {} vs {}", huge, tiny);
    }
}
