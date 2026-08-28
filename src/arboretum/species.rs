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
}
