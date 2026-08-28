//! Species renderers for arboretum: same genome knobs, different habits.
//! Each fn reuses the bole/taper base then draws its own geometry.
use super::*;

pub(super) fn grow_species(
    grid: &mut Grid,
    rx: i32,
    ry: i32,
    budget: i32,
    genome: &TreeGenome,
    cols: &TreeColors,
    rng: &mut StdRng,
    grow: f32,
    foliage: f32,
    sway: f32,
) {
    let budget = (3.0 + (budget.max(2) as f32 - 3.0) * ease_in_out(grow.clamp(0.0, 1.0))) as i32;
    let budget = budget.max(2);
    match genome.style {
        TreeStyle::Conifer => conifer(grid, rx, ry, budget, genome, cols, rng, foliage, sway),
        TreeStyle::Broadleaf => broadleaf(grid, rx, ry, budget, genome, cols, rng, foliage, sway),
        TreeStyle::Willow => willow(grid, rx, ry, budget, genome, cols, rng, foliage, sway),
        TreeStyle::Cypress => cypress(grid, rx, ry, budget, genome, cols, rng, foliage, sway),
        TreeStyle::Babel => babel(grid, rx, ry, budget, genome, cols, rng, foliage, sway),
        TreeStyle::Pleach => pleach(grid, rx, ry, budget, genome, cols, rng, foliage, sway),
        TreeStyle::Uzumaki => uzumaki(grid, rx, ry, budget, genome, cols, rng, foliage, sway),
        TreeStyle::Classic => {}
    }
}

fn base_start(grid: &mut Grid, rx: i32, ry: i32, budget: i32, genome: &TreeGenome, cols: &TreeColors, rng: &mut StdRng) -> (i32, i32) {
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
            let exit = Bole { style }.draw(grid, &tp, rng);
            return crate::tree_draw::draw_taper(
                grid,
                &BoleExit {
                    x: exit.x,
                    y: exit.y,
                    left: exit.left,
                    right: exit.right,
                },
                cols.trunk,
                genome.taper,
            );
        }
    }
    set(grid, rx, ry, '│', cols.trunk);
    (rx, ry)
}

// ── Conifer: straight spire, tiered needle whorls, cones ────────────

fn conifer(grid: &mut Grid, rx: i32, ry: i32, budget: i32, genome: &TreeGenome, cols: &TreeColors, rng: &mut StdRng, foliage: f32, sway: f32) {
    let (tx, ty) = base_start(grid, rx, ry, budget, genome, cols, rng);
    let trunk_h = ((budget as f32) * (0.5 + 0.4 * genome.vigor)).max(2.0) as i32;
    let lean_dx = (genome.lean + sway) * 0.4;

    let mut pen = TreePen::new(tx, ty, cols.trunk);
    pen.last_dir = Some(MoveDir::Up);
    let mut nodes: Vec<(i32, i32)> = Vec::new();
    for k in 0..trunk_h {
        let dir = if (k + 1) % 4 == 0 && lean_dx.abs() > 0.15 {
            if lean_dx > 0.0 { MoveDir::UpRight } else { MoveDir::UpLeft }
        } else {
            MoveDir::Up
        };
        pen.step(grid, dir);
        nodes.push((pen.x, pen.y));
    }

    // Tiers: widest at the bottom, one row of needles every 2 trunk rows.
    let step = if budget >= 14 { 2 } else { 1 };
    let n = nodes.len();
    for (i, &(nx, ny)) in nodes.iter().enumerate() {
        if i % step != 0 {
            continue;
        }
        let from_top = (n - i) as f32 / n.max(1) as f32; // 0 top .. 1 bottom
        let half = (1.0 + from_top * from_top * budget as f32 * 0.28 * genome.spread)
            .floor()
            .max(0.0) as i32;
        for dx in -half..=half {
            if dx == 0 {
                continue;
            }
            let x = nx + dx;
            if rng.random::<f32>() < foliage * 0.92 {
                let edge = dx.abs() == half;
                let ch = if edge && dx > 0 {
                    '╱'
                } else if edge {
                    '╲'
                } else {
                    '∧'
                };
                let c = if rng.random::<f32>() < 0.3 { lighten(cols.leaf, 15) } else { cols.leaf };
                if blank_at(grid, x, ny) {
                    set(grid, x, ny, ch, c);
                }
            }
            if rng.random::<f32>() < genome.fruition * foliage * 0.12 && dx.abs() == half {
                set(grid, x, ny - 1, '◆', cols.fruit);
            }
        }
    }
    if let Some(&(ax, ay)) = nodes.last() {
        set(grid, ax, ay, '▲', lighten(cols.leaf, 25));
    }
}

// ── Broadleaf: short trunk, boughs flatten into a leafy dome ────────

fn broadleaf(grid: &mut Grid, rx: i32, ry: i32, budget: i32, genome: &TreeGenome, cols: &TreeColors, rng: &mut StdRng, foliage: f32, sway: f32) {
    let (tx, ty) = base_start(grid, rx, ry, budget, genome, cols, rng);
    let trunk_h = ((budget as f32) * (0.2 + 0.18 * genome.vigor)).max(2.0) as i32;

    let mut pen = TreePen::new(tx, ty, cols.trunk);
    pen.last_dir = Some(MoveDir::Up);
    let mut nodes: Vec<(i32, i32)> = Vec::new();
    let lean = genome.lean + sway;
    for k in 0..trunk_h {
        let dir = if k > 0 && k % 4 == 0 && lean.abs() > 0.4 {
            if lean > 0.0 { MoveDir::UpRight } else { MoveDir::UpLeft }
        } else {
            MoveDir::Up
        };
        pen.step(grid, dir);
        nodes.push((pen.x, pen.y));
    }
    if budget >= 10 {
        for k in 0..(trunk_h / 2).max(1) {
            if let Some(&(nx, ny)) = nodes.get(k as usize) {
                set(grid, nx - 1, ny, '┆', darken(cols.trunk, 10));
                set(grid, nx + 1, ny, '┆', darken(cols.trunk, 10));
            }
        }
    }

    // Boughs climb briefly then flatten outward; ends seed canopy blobs.
    let mut crown_pts: Vec<(i32, i32)> = Vec::new();
    let bough_n = 3 + (genome.boughs * 3.0) as i32;
    let mut side = if rng.random::<f32>() < 0.5 { 1 } else { -1 };
    let n = nodes.len();
    for (i, &(nx, ny)) in nodes.iter().enumerate() {
        if i < n / 3 || crown_pts.len() >= bough_n as usize {
            continue;
        }
        let mut bp = TreePen::new(nx, ny, cols.branch);
        bp.last_dir = Some(MoveDir::Up);
        let climb = (budget as f32 * 0.12).max(1.0) as i32;
        for _ in 0..climb {
            bp.step(grid, if side > 0 { MoveDir::UpRight } else { MoveDir::UpLeft });
        }
        let run = ((budget as f32) * (0.12 + 0.16 * i as f32 / n as f32) * genome.spread).max(2.0) as i32;
        for _ in 0..run {
            bp.step(grid, if side > 0 { MoveDir::Right } else { MoveDir::Left });
        }
        crown_pts.push((bp.x, bp.y));
        side = -side;
    }
    if let Some(&(ax, ay)) = nodes.last() {
        crown_pts.push((ax, ay));
    }

    let leaf_glyphs = ['⠿', '⣿', '❀', '✿', '✳', '✶'];
    let radius = 2.5 + genome.spread * 2.0 + budget as f32 * 0.08;
    for &(cx, cy) in &crown_pts {
        let r = radius + rng.random::<f32>() * 1.5;
        let r2 = r * r;
        let ir = (r - 2.2).max(0.3);
        for dy in -r.ceil() as i32..=r.ceil() as i32 {
            for dx in -r.ceil() as i32..=r.ceil() as i32 {
                let d2 = (dx * dx + dy * dy) as f32;
                if d2 > r2 || d2 < ir * ir {
                    continue;
                }
                let p = foliage * (0.3 + genome.leafage * 0.7) * (1.0 - d2 / r2 * 0.3);
                if rng.random::<f32>() < p {
                    let g = leaf_glyphs[rng.random_range(0..leaf_glyphs.len() as u32) as usize];
                    let c = if rng.random::<f32>() < 0.35 { lighten(cols.leaf, 18) } else { cols.leaf };
                    if blank_at(grid, cx + dx, cy + dy) {
                        set(grid, cx + dx, cy + dy, g, c);
                    }
                }
            }
        }
        if rng.random::<f32>() < genome.fruition * foliage {
            set(grid, cx, cy + r.ceil() as i32, '●', cols.fruit);
        }
    }
}

// ── Willow: S-curved trunk, crown of falling strands ────────────────

fn willow(grid: &mut Grid, rx: i32, ry: i32, budget: i32, genome: &TreeGenome, cols: &TreeColors, rng: &mut StdRng, foliage: f32, sway: f32) {
    let (tx, ty) = base_start(grid, rx, ry, budget, genome, cols, rng);
    let trunk_h = ((budget as f32) * (0.3 + 0.25 * genome.vigor)).max(2.0) as i32;

    let mut pen = TreePen::new(tx, ty, cols.trunk);
    pen.last_dir = Some(MoveDir::Up);
    let mut nodes: Vec<(i32, i32)> = Vec::new();
    let lean = genome.lean + sway;
    for k in 0..trunk_h {
        let dir = match k % 5 {
            1 => {
                if lean >= 0.0 { MoveDir::UpRight } else { MoveDir::UpLeft }
            }
            3 => {
                if lean >= 0.0 { MoveDir::UpLeft } else { MoveDir::UpRight }
            }
            _ => MoveDir::Up,
        };
        pen.step(grid, dir);
        nodes.push((pen.x, pen.y));
    }
    if let Some(&(ax, ay)) = nodes.last() {
        set(grid, ax, ay, '┬', cols.branch);
    }

    let strand_n = (4.0 + genome.spread * 7.0) as usize;
    let max_len = (budget as f32 * 0.5) as i32;
    let strand_ch = ['╎', '┆'];
    let mut seeds_rng = StdRng::seed_from_u64(rx as u64 * 31 + ry as u64 * 17);
    for s in 0..strand_n {
        let srng = &mut seeds_rng;
        let spread_w = (genome.spread * budget as f32 * 0.35).max(2.0) as i32;
        let sx = nodes
            .last()
            .map(|&(ax, _)| ax)
            .unwrap_or(rx)
            + srng.random_range(-spread_w..=spread_w);
        let sy = nodes.last().map(|&(_, ay)| ay).unwrap_or(ry)
            + srng.random_range(0..2);
        let len = srng.random_range((max_len / 3).max(2)..=(max_len.max(3)));
        let ch = strand_ch[srng.random_range(0..2) as usize];
        let mut x = sx;
        let mut y = sy;
        let wob: f32 = srng.random::<f32>();
        for k in 0..len {
            let sway_nudge = if sway != 0.0 { sway * 2.0 } else { 0.0 };
            let drift = (k as f32 * 0.35 + wob * 6.0 + sway_nudge).sin() * 0.8;
            if drift > 0.6 && k > len / 3 {
                x += 1;
            } else if drift < -0.6 && k > len / 3 {
                x -= 1;
            } else if k % 2 == 1 {
                y += 1;
            }
            y += 1;
            if y >= ry {
                break;
            }
            if blank_at(grid, x, y) {
                set(grid, x, y, ch, cols.branch);
            }
            if rng.random::<f32>() < genome.leafage * foliage * 0.45
                && blank_at(grid, x + 1, y)
            {
                set(grid, x + 1, y, '⠿', cols.leaf);
            }
        }
        if rng.random::<f32>() < genome.fruition * foliage * 0.3 {
            set(grid, x, y, '◉', cols.fruit);
        }
    }
}

// ── Babel: sane trunk that loses its mind as it climbs ──────────────

fn babel(grid: &mut Grid, rx: i32, ry: i32, budget: i32, genome: &TreeGenome, cols: &TreeColors, rng: &mut StdRng, foliage: f32, sway: f32) {
    let (tx, ty) = base_start(grid, rx, ry, budget, genome, cols, rng);
    let trunk_h = ((budget as f32) * (0.55 + 0.4 * genome.vigor)).max(6.0) as i32;
    let helix = rng.random::<f32>() * std::f32::consts::TAU;
    let lean = genome.lean + sway;

    let mut pen = TreePen::new(tx, ty, cols.trunk);
    pen.last_dir = Some(MoveDir::Up);
    let mut nodes: Vec<(i32, i32, f32)> = Vec::new();
    for k in 0..trunk_h {
        let f = k as f32 / trunk_h.max(1) as f32;
        let wood = lighten(cols.trunk, (f * 55.0) as u8);
        pen.color = wood;
        let dir = if f < 0.35 {
            if rng.random::<f32>() < genome.gnarl * 0.15 {
                if lean >= 0.0 { MoveDir::UpRight } else { MoveDir::UpLeft }
            } else {
                MoveDir::Up
            }
        } else if f < 0.55 {
            let s = (k as f32 * 0.55 + helix + sway * 2.0).sin();
            if s > 0.4 {
                MoveDir::UpRight
            } else if s < -0.4 {
                MoveDir::UpLeft
            } else {
                MoveDir::Up
            }
        } else {
            match rng.random_range(0..5u32) {
                0 => MoveDir::Up,
                1 => MoveDir::UpRight,
                2 => MoveDir::UpLeft,
                3 => MoveDir::Right,
                _ => MoveDir::Left,
            }
        };
        let broken = f >= 0.75;
        if broken && k % 2 == 1 {
            // gap: ascend without drawing -- the trunk stops being continuous
            pen.x += dir.dx();
            pen.y += dir.dy();
            pen.last_dir = Some(dir);
        } else {
            let prev = pen.last_dir;
            pen.step(grid, dir);
            if f >= 0.55 && prev != pen.last_dir && rng.random::<f32>() < 0.5 {
                set(grid, pen.x, pen.y, if rng.random::<f32>() < 0.5 { '┼' } else { '╋' }, wood);
            }
        }
        nodes.push((pen.x, pen.y, f));
    }

    // Zone 1 boughs: sane alternating diagonals.
    let mut side: i32 = if rng.random::<f32>() < 0.5 { 1 } else { -1 };
    for (i, &(nx, ny, f)) in nodes.iter().enumerate() {
        if f >= 0.35 || i < 2 {
            continue;
        }
        if i % 3 == 0 && rng.random::<f32>() < genome.boughs {
            let mut bp = TreePen::new(nx, ny, cols.branch);
            bp.last_dir = Some(MoveDir::Up);
            let len = ((budget as f32) * 0.14 * genome.spread).max(2.0) as i32;
            for s in 0..len {
                bp.step(grid, if s < len - 1 {
                    if side > 0 { MoveDir::UpRight } else { MoveDir::UpLeft }
                } else if side > 0 { MoveDir::Right } else { MoveDir::Left });
            }
            set(grid, bp.x, bp.y, '╷', lighten(cols.branch, 25));
            if rng.random::<f32>() < genome.leafage * foliage {
                set(grid, bp.x, bp.y - 1, '⠿', cols.leaf);
            }
            side = -side;
        }
    }

    // Zone 2 boughs: hooks that curl out, down, and back toward the trunk.
    for &(nx, ny, f) in nodes.iter() {
        if !(0.35..0.55).contains(&f) || rng.random::<f32>() > 0.45 {
            continue;
        }
        let side = if rng.random::<f32>() < 0.5 { 1 } else { -1 };
        let mut hx = nx;
        let mut hy = ny;
        let out = rng.random_range(2..5);
        for _ in 0..out {
            hx += side;
            if blank_at(grid, hx, hy) {
                set(grid, hx, hy, '─', cols.branch);
            }
        }
        for _ in 0..rng.random_range(1..3) {
            hy += 1;
            if blank_at(grid, hx, hy) {
                set(grid, hx, hy, '╯', cols.branch);
            }
        }
        for _ in 0..out - 1 {
            hx -= side;
            if blank_at(grid, hx, hy) {
                set(grid, hx, hy, '╶', cols.branch);
            }
        }
        if rng.random::<f32>() < genome.leafage * foliage {
            set(grid, hx, hy, '❉', lighten(cols.leaf, 12));
        }
    }

    // Zone 3: fork bombs off chaos nodes.
    let mut used = 0usize;
    let chaos: Vec<(i32, i32)> = nodes
        .iter()
        .filter(|&&(_, _, f)| (0.55..0.75).contains(&f))
        .step_by(3)
        .map(|&(x, y, _)| (x, y))
        .collect();
    for &(cx, cy) in chaos.iter().take(4) {
        madness(grid, cx, cy, 0, cols, rng, genome, foliage, &mut used);
    }

    // Crown + floating islands above the broken tip.
    if let Some(&(ax, ay, _)) = nodes.last() {
        crown(grid, ax, ay, cols, rng, genome, foliage);
    }
    let islands = rng.random_range(2..5u32);
    let (lx, ly) = (ax_of(&nodes), ay_of(&nodes));
    for _ in 0..islands {
        let ix = lx + rng.random_range(-7..8);
        let iy = ly - rng.random_range(1..7);
        island(grid, ix, iy, cols, rng, foliage);
    }
}

fn ax_of(nodes: &[(i32, i32, f32)]) -> i32 {
    nodes.last().map(|&(x, _, _)| x).unwrap_or(0)
}

fn ay_of(nodes: &[(i32, i32, f32)]) -> i32 {
    nodes.last().map(|&(_, y, _)| y).unwrap_or(0)
}

const MADNESS_CAP: usize = 240;

fn madness(grid: &mut Grid, x: i32, y: i32, order: u8, cols: &TreeColors, rng: &mut StdRng, genome: &TreeGenome, foliage: f32, used: &mut usize) {
    if *used >= MADNESS_CAP || order > 3 {
        return;
    }
    let dirs = [
        MoveDir::Up,
        MoveDir::UpRight,
        MoveDir::UpLeft,
        MoveDir::Right,
        MoveDir::Left,
        MoveDir::DownRight,
        MoveDir::DownLeft,
    ];
    let mut pen = TreePen::new(x, y, lighten(cols.branch, order as u8 * 15));
    pen.last_dir = Some(dirs[rng.random_range(0..dirs.len() as u32) as usize]);
    let run = rng.random_range(1..4);
    for _ in 0..run {
        let dir = if rng.random::<f32>() < 0.4 {
            dirs[rng.random_range(0..dirs.len() as u32) as usize]
        } else {
            pen.last_dir.unwrap()
        };
        pen.step(grid, dir);
        *used += 1;
    }
    if rng.random::<f32>() < genome.leafage * foliage {
        let g = ['⠿', '❀', '✳', '✶', '❉'][rng.random_range(0..5) as usize];
        set(grid, pen.x, pen.y - 1, g, lighten(cols.leaf, rng.random_range(0..30)));
    }
    if rng.random::<f32>() < genome.fruition * foliage * 0.4 {
        set(grid, pen.x, pen.y + 1, '◆', cols.fruit);
    }
    let kids = if order < 2 { 3 } else { 2 };
    for _ in 0..kids {
        madness(grid, pen.x, pen.y, order + 1, cols, rng, genome, foliage, used);
    }
}

fn crown(grid: &mut Grid, cx: i32, cy: i32, cols: &TreeColors, rng: &mut StdRng, genome: &TreeGenome, foliage: f32) {
    for r in [3i32, 5] {
        let pts = (r as f32 * 4.0) as i32;
        for p in 0..pts {
            let a = p as f32 / pts as f32 * std::f32::consts::TAU;
            let x = cx + (a.cos() * r as f32).round() as i32;
            let y = cy - (a.sin() * r as f32 * 0.8).round() as i32;
            if rng.random::<f32>() < 0.8 {
                set(grid, x, y, '◌', lighten(cols.leaf, 20));
            }
        }
    }
    set(grid, cx, cy, '✦', lighten(cols.fruit, 20));
    if rng.random::<f32>() < genome.leafage * foliage {
        set(grid, cx - 2, cy, '⣿', cols.leaf);
        set(grid, cx + 2, cy, '⣿', cols.leaf);
    }
    for dy in [2i32, 4] {
        if rng.random::<f32>() < genome.fruition * foliage {
            set(grid, cx + rng.random_range(-2..3), cy + dy, '◆', cols.fruit);
        }
    }
}

fn island(grid: &mut Grid, cx: i32, cy: i32, cols: &TreeColors, rng: &mut StdRng, foliage: f32) {
    let r = rng.random_range(1..3);
    let pts = (r as f32 * 6.0).max(6.0) as i32;
    for p in 0..pts {
        let a = p as f32 / pts as f32 * std::f32::consts::TAU;
        let x = cx + (a.cos() * r as f32).round() as i32;
        let y = cy - (a.sin() * r as f32 * 0.7).round() as i32;
        set(grid, x, y, if p % 2 == 0 { '·' } else { '◌' }, lighten(cols.leaf, 30));
    }
    set(grid, cx, cy, '❉', lighten(cols.leaf, 10));
    if rng.random::<f32>() < foliage * 0.6 {
        set(grid, cx, cy + r + 1 + rng.random_range(0..2), '◆', cols.fruit);
    }
}

// ── Pleach: trained to one ceiling -- every tip lands on the same row ──

fn pleach(grid: &mut Grid, rx: i32, ry: i32, budget: i32, genome: &TreeGenome, cols: &TreeColors, rng: &mut StdRng, foliage: f32, sway: f32) {
    let (tx, ty) = base_start(grid, rx, ry, budget, genome, cols, rng);
    let trunk_h = ((budget as f32) * (0.4 + 0.35 * genome.vigor)).max(4.0) as i32;
    let ceiling = ty - trunk_h;
    let lean = genome.lean + sway;

    let mut pen = TreePen::new(tx, ty, cols.trunk);
    pen.last_dir = Some(MoveDir::Up);
    let mut nodes: Vec<(i32, i32)> = Vec::new();
    while pen.y > ceiling {
        let dir = if pen.y - ceiling > trunk_h * 2 / 3 && lean.abs() > 0.5 {
            if lean > 0.0 { MoveDir::UpRight } else { MoveDir::UpLeft }
        } else {
            MoveDir::Up
        };
        pen.step(grid, dir);
        nodes.push((pen.x, pen.y));
    }

    // Candelabra arms: start low on the trunk, arc out and up, tip exactly on the ceiling.
    let arm_n = 2 + (genome.spread * 3.0) as i32;
    let n = nodes.len();
    let mut xmin = tx;
    let mut xmax = tx;
    for i in 0..arm_n {
        for side in [1i32, -1] {
            let start_i = (n as f32 * (0.25 + 0.55 * (i as f32 + 1.0) / arm_n as f32)) as usize;
            if start_i >= n {
                continue;
            }
            let (sx, sy) = nodes[start_i];
            let mut ap = TreePen::new(sx, sy, cols.branch);
            ap.last_dir = Some(MoveDir::Up);
            let reach = ((budget as f32) * 0.16 * (i as f32 + 1.0) / arm_n as f32 * genome.spread)
                .max(2.0) as i32;
            let out_steps = reach / 2;
            for _ in 0..out_steps {
                ap.step(grid, if side > 0 { MoveDir::UpRight } else { MoveDir::UpLeft });
            }
            while ap.y > ceiling {
                let climb = if ap.y - ceiling > 2 {
                    if side > 0 { MoveDir::Right } else { MoveDir::Left }
                } else {
                    MoveDir::Up
                };
                let d = if ap.y - ceiling > 2 && rng.random::<f32>() < 0.4 {
                    MoveDir::Up
                } else {
                    climb
                };
                ap.step(grid, d);
            }
            set(grid, ap.x, ap.y, '╷', lighten(cols.branch, 30));
            xmin = xmin.min(ap.x);
            xmax = xmax.max(ap.x);
        }
    }

    // Ceiling band: the trained line itself, leaves knuckling on it, pendants below.
    let mut x = xmin;
    while x <= xmax {
        let inb = ceiling >= 0 && x >= 0 && (ceiling as usize) < grid.len() && (x as usize) < grid[0].len();
        let on_line = if inb { grid[ceiling as usize][x as usize].ch } else { ' ' };
        if on_line == ' ' || on_line == '│' {
            set(grid, x, ceiling, '─', cols.branch);
        }
        if rng.random::<f32>() < genome.leafage * foliage * 0.5 {
            let g = ['⠿', '❀', '✿'][rng.random_range(0..3) as usize];
            if on_line == ' ' {
                set(grid, x, ceiling, g, lighten(cols.leaf, 15));
            }
        }
        if x % 3 == 1 {
            let plen = rng.random_range(1..=(budget / 4).max(2)) as i32;
            let mut py = ceiling + 1;
            let mut px = x;
            let ch = if rng.random::<f32>() < 0.5 { '╎' } else { '┆' };
            for _ in 0..plen {
                if py > ry {
                    break;
                }
                if blank_at(grid, px, py) {
                    set(grid, px, py, ch, cols.branch);
                }
                if rng.random::<f32>() < sway.abs() * 0.4 {
                    px += if sway > 0.0 { 1 } else { -1 };
                }
                py += 1;
            }
            if rng.random::<f32>() < genome.fruition * foliage * 0.5 && py <= ry {
                set(grid, px, py.saturating_sub(1), '◆', cols.fruit);
            }
        }
        x += 1;
    }
}

// ── Uzumaki: spiral horror -- helix trunk tightening into a vortex ──

const DIR_CYCLE: [MoveDir; 8] = [
    MoveDir::Up,
    MoveDir::UpRight,
    MoveDir::Right,
    MoveDir::DownRight,
    MoveDir::Down,
    MoveDir::DownLeft,
    MoveDir::Left,
    MoveDir::UpLeft,
];

fn turn(dir: MoveDir, by: i32) -> MoveDir {
    let i = DIR_CYCLE.iter().position(|&d| d == dir).unwrap_or(0) as i32;
    DIR_CYCLE[((i + by).rem_euclid(8)) as usize]
}

fn plot_spiral(grid: &mut Grid, cx: i32, cy: i32, r0: f32, turns: f32, cols: &TreeColors, rng: &mut StdRng, foliage: f32) {
    let theta_max = turns * std::f32::consts::TAU;
    let mut theta = 0.0;
    while theta < theta_max {
        let r = r0 * (1.0 - theta / theta_max);
        let x = cx + (theta.cos() * r * 1.6).round() as i32; // widen: cells are tall
        let y = cy - (theta.sin() * r * 0.8).round() as i32;
        let frac = theta / theta_max;
        let ch = if frac > 0.75 {
            '◌'
        } else if frac > 0.45 {
            '∘'
        } else if rng.random::<f32>() < 0.85 {
            '·'
        } else {
            '○'
        };
        if rng.random::<f32>() < 0.9 * foliage.max(0.35) {
            let c = lighten(cols.leaf, (frac * 35.0) as u8);
            if blank_at(grid, x, y) {
                set(grid, x, y, ch, c);
            }
        }
        theta += 0.45;
    }
    set(grid, cx, cy, '◉', cols.fruit);
}

fn uzumaki(grid: &mut Grid, rx: i32, ry: i32, budget: i32, genome: &TreeGenome, cols: &TreeColors, rng: &mut StdRng, foliage: f32, sway: f32) {
    let (tx, ty) = base_start(grid, rx, ry, budget, genome, cols, rng);
    let trunk_h = ((budget as f32) * (0.45 + 0.35 * genome.vigor)).max(5.0) as i32;
    let phase = rng.random::<f32>() * std::f32::consts::TAU;
    let amp0 = 1.0 + genome.spread * 2.0;
    let spin = if rng.random::<f32>() < 0.5 { 1.0 } else { -1.0 };

    let mut pen = TreePen::new(tx, ty, cols.trunk);
    pen.last_dir = Some(MoveDir::Up);
    let mut nodes: Vec<(i32, i32)> = Vec::new();
    for k in 0..trunk_h {
        let f = k as f32 / trunk_h.max(1) as f32;
        let amp = (amp0 * (1.0 - f * 0.8)).max(0.0);
        let target = tx + (spin * (k as f32 * 0.7 + phase + sway * 3.0).sin() * amp).round() as i32;
        while pen.x != target {
            pen.step(grid, if pen.x < target { MoveDir::Right } else { MoveDir::Left });
        }
        pen.step(grid, MoveDir::Up);
        nodes.push((pen.x, pen.y));
        // ghost strand: the far side of the helix, drawn dim
        let gx = tx + (spin * (k as f32 * 0.7 + phase + std::f32::consts::PI + sway * 3.0).sin() * amp).round() as i32;
        if (gx - pen.x).abs() > 1 && blank_at(grid, gx, pen.y) {
            set(grid, gx, pen.y, '┆', darken(cols.trunk, 12));
        }
        // whorls knotting up the trunk
        if k % 5 == 2 && rng.random::<f32>() < 0.7 {
            let w = if spin > 0.0 { '◠' } else { '◡' };
            if blank_at(grid, pen.x + 1, pen.y) {
                set(grid, pen.x + 1, pen.y, w, darken(cols.branch, 8));
            }
        }
    }
    // the eye, mid-trunk: it watches
    let (ex, ey) = nodes[trunk_h as usize / 2];
    set(grid, ex, ey, '◉', cols.fruit);
    for d in [(0i32, -1), (0, 1), (-1, 0), (1, 0)] {
        if blank_at(grid, ex + d.0, ey + d.1) {
            set(grid, ex + d.0, ey + d.1, '·', darken(cols.fruit, 20));
        }
    }

    // coiling tendrils: constant-turn turtles that wind themselves up
    let tendril_n = 3 + (genome.boughs * 3.0) as u32;
    for t in 0..tendril_n {
        let (sx, sy) = nodes[((t as f32 / tendril_n as f32 * nodes.len() as f32 * 0.7) as usize).min(nodes.len() - 1)];
        let side = if t % 2 == 0 { 1 } else { -1 };
        let mut tp = TreePen::new(sx, sy, cols.branch);
        let mut dir = if side > 0 { MoveDir::UpRight } else { MoveDir::UpLeft };
        let len = rng.random_range(3..((budget as f32 * 0.16) as i32).max(5));
        let curl = if rng.random::<f32>() < 0.5 { 1 } else { -1 };
        for s in 0..len {
            if s % 2 == 0 {
                dir = turn(dir, curl);
            }
            tp.step(grid, dir);
            if rng.random::<f32>() < genome.leafage * foliage * 0.35 && blank_at(grid, tp.x, tp.y - 1) {
                set(grid, tp.x, tp.y - 1, '∘', lighten(cols.leaf, 10));
            }
        }
        if rng.random::<f32>() < genome.fruition * foliage {
            set(grid, tp.x, tp.y, '◌', cols.fruit);
        }
    }

    // vortex crown
    if let Some(&(ax, ay)) = nodes.last() {
        let r0 = 2.0 + genome.spread * 2.5 + budget as f32 * 0.07;
        plot_spiral(grid, ax, ay - 1, r0, 2.2 + genome.spread, cols, rng, foliage);
        let offshoots = rng.random_range(1..3u32);
        for _ in 0..offshoots {
            let ox = ax + rng.random_range(-6..7);
            let oy = ay - rng.random_range(1..5);
            plot_spiral(grid, ox, oy, r0 * 0.5, 1.6, cols, rng, foliage);
        }
    }
}

// ── Cypress: tight flame column, swaying tip ────────────────────────

fn cypress(grid: &mut Grid, rx: i32, ry: i32, budget: i32, genome: &TreeGenome, cols: &TreeColors, rng: &mut StdRng, foliage: f32, sway: f32) {
    let (tx, ty) = base_start(grid, rx, ry, budget, genome, cols, rng);
    let h = ((budget as f32) * (0.65 + 0.3 * genome.vigor)).max(3.0) as i32;
    let max_w = (1.0 + genome.spread * budget as f32 * 0.09).max(1.0) as i32;
    let lean = genome.lean + sway;

    for k in 0..h {
        let frac = k as f32 / h.max(1) as f32; // 0 base .. 1 tip
        let y = ty - k;
        // width swells mid-column, narrows to the tip
        let bulge = (1.0 - (frac - 0.55).abs() / 0.55).max(0.0);
        let w = ((max_w as f32) * bulge).round() as i32;
        let bend = (lean * frac * frac * 2.5).round() as i32;
        let cx = tx + bend;
        set(grid, cx, y, '│', cols.trunk);
        for dx in 1..=w {
            let lx = cx - dx;
            let rxx = cx + dx;
            if rng.random::<f32>() < foliage * 0.9 {
                set(grid, lx, y, if k % 2 == 0 { '╱' } else { '┆' }, cols.leaf);
                set(grid, rxx, y, if k % 2 == 0 { '╲' } else { '┆' }, lighten(cols.leaf, 12));
            }
            if rng.random::<f32>() < genome.fruition * foliage * 0.06 {
                set(grid, if dx % 2 == 0 { lx } else { rxx }, y, '◆', cols.fruit);
            }
        }
    }
    set(grid, tx + (lean * h as f32 * h as f32 * 0.002).round() as i32, ty - h, '╷', lighten(cols.branch, 30));
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    fn plain(grid: &Grid) -> String {
        grid.iter()
            .map(|row| row.iter().map(|c| c.ch).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn cols() -> TreeColors {
        TreeColors {
            trunk: crate::color::rgb(90, 110, 60),
            branch: crate::color::rgb(90, 140, 60),
            leaf: crate::color::rgb(110, 170, 70),
            fruit: crate::color::rgb(200, 90, 60),
        }
    }

    fn species_grid(style: TreeStyle) -> String {
        let mut g = vec![vec![Cell::blank(); 34]; 18];
        let mut r = StdRng::seed_from_u64(42);
        let genome = TreeGenome { style, ..TreeGenome::roll(&mut r) };
        grow_tree(&mut g, 17, 16, 15, &genome, &cols(), &mut r, 1.0, 1.0, 0.0);
        plain(&g)
    }

    #[test]
    fn snapshot_conifer() {
        insta::assert_snapshot!("arboretum_conifer", species_grid(TreeStyle::Conifer));
    }

    #[test]
    fn snapshot_broadleaf() {
        insta::assert_snapshot!("arboretum_broadleaf", species_grid(TreeStyle::Broadleaf));
    }

    #[test]
    fn snapshot_willow() {
        insta::assert_snapshot!("arboretum_willow", species_grid(TreeStyle::Willow));
    }

    #[test]
    fn snapshot_cypress() {
        insta::assert_snapshot!("arboretum_cypress", species_grid(TreeStyle::Cypress));
    }

    #[test]
    fn snapshot_babel() {
        insta::assert_snapshot!("arboretum_babel", species_grid(TreeStyle::Babel));
    }

    #[test]
    fn snapshot_pleach() {
        insta::assert_snapshot!("arboretum_pleach", species_grid(TreeStyle::Pleach));
    }

    #[test]
    fn snapshot_uzumaki() {
        insta::assert_snapshot!("arboretum_uzumaki", species_grid(TreeStyle::Uzumaki));
    }

    #[test]
    fn uzumaki_trunk_coils_and_crown_vortexes() {
        let mut g = vec![vec![Cell::blank(); 40]; 20];
        let mut r = StdRng::seed_from_u64(42);
        let genome = TreeGenome { style: TreeStyle::Uzumaki, ..TreeGenome::roll(&mut r) };
        grow_tree(&mut g, 20, 18, 16, &genome, &cols(), &mut r, 1.0, 1.0, 0.0);
        let rows: Vec<String> = g
            .iter()
            .map(|row| row.iter().map(|c| c.ch).collect())
            .collect();
        // trunk zone must wander: not all wood in one column
        let mut cols_with_wood = std::collections::HashSet::new();
        for row in rows.iter().take(16).skip(2) {
            for (x, ch) in row.char_indices() {
                if ch == '│' || ch == '╱' || ch == '╲' {
                    cols_with_wood.insert(x);
                }
            }
        }
        assert!(cols_with_wood.len() >= 2, "helix trunk should span columns: {:?}", cols_with_wood);
        // crown must hold spiral glyphs and an eye
        let crown: String = rows[1..8].concat();
        assert!(crown.contains('◌'), "crown needs inner spiral marks");
        assert!(crown.contains('◉'), "crown needs a vortex eye");
    }

    #[test]
    fn pleach_tips_meet_one_row() {
        let mut g = vec![vec![Cell::blank(); 40]; 20];
        let mut r = StdRng::seed_from_u64(42);
        let genome = TreeGenome { style: TreeStyle::Pleach, ..TreeGenome::roll(&mut r) };
        grow_tree(&mut g, 20, 18, 16, &genome, &cols(), &mut r, 1.0, 1.0, 0.0);
        // every arm tip '╷' must sit on a single row, and nothing may sit above it
        let mut tip_rows: Vec<usize> = Vec::new();
        let mut topmost = g.len();
        for (y, row) in g.iter().enumerate() {
            for c in row {
                if c.ch != ' ' {
                    topmost = topmost.min(y);
                }
                if c.ch == '╷' {
                    tip_rows.push(y);
                }
            }
        }
        assert!(tip_rows.len() >= 2, "expected several trained tips, got {}", tip_rows.len());
        assert_eq!(tip_rows[0], tip_rows[tip_rows.len() - 1], "tips must share one row: {:?}", tip_rows);
        assert_eq!(topmost, tip_rows[0], "nothing may rise above the shared ceiling");
    }

    #[test]
    fn babel_grows_sane_then_crazy() {
        // low rows must hold ordinary trunk wood; upper rows must break it
        let mut g = vec![vec![Cell::blank(); 30]; 26];
        let mut r = StdRng::seed_from_u64(42);
        let genome = TreeGenome { style: TreeStyle::Babel, ..TreeGenome::roll(&mut r) };
        grow_tree(&mut g, 15, 24, 22, &genome, &cols(), &mut r, 1.0, 1.0, 0.0);
        let rows: Vec<String> = g
            .iter()
            .map(|row| row.iter().map(|c| c.ch).collect())
            .collect();
        let sane = rows[18..23].concat();
        assert!(sane.contains('│'), "lower trunk should be ordinary wood: {:?}", sane);
        let crazy = rows[2..9].concat();
        assert!(
            crazy.contains('◌') || crazy.contains('❉') || crazy.contains('◆') || crazy.contains('✦'),
            "crown zone should hold impossible things: {:?}", crazy
        );
    }
}
