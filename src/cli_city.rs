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


// --- elevator: cab banks running seeded service loops in a building shaft. ---
// Motion is a piecewise schedule (eased travel, then a dwell whose doors cycle
// closed -> opening -> open -> closing), evaluated as a pure function of
// (seed, t). Counterweights mirror the cars, cables pay out above them, call
// lamps light while tenants wait and go dark once a cab services the floor.

/// splitmix-style mix for per-floor/per-cycle tenant rolls.
fn emix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// How many tenants wait at `floor` during passenger-cycle `pc`.
fn elevator_waiting(seed: u64, floor: u64, pc: u64, crowd: f32) -> usize {
    let h = emix(seed ^ floor.wrapping_mul(0x517C_C1B7).wrapping_add(pc.wrapping_mul(0x2722_0A95)));
    let roll = (h % 1000) as f32 / 1000.0;
    if roll < 0.24 * crowd.clamp(0.0, 3.0) {
        1 + ((h >> 24) as usize) % 2
    } else {
        0
    }
}

pub(crate) fn draw_elevator(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    rng: &mut StdRng,
    t: f32,
    lifts: usize,
    speed: f32,
    crowd: f32,
) {
    if width < 10 || height < 6 {
        return;
    }
    let speed = speed.clamp(0.05, 4.0);
    let crowd = crowd.clamp(0.0, 3.0);
    let lifts = lifts.clamp(1, ((width - 7) / 5).max(1));

    let bg = darken(palette[0], 12);
    for row in grid.iter_mut() {
        for cell in row.iter_mut() {
            *cell = Cell::new(' ', bg);
        }
    }

    // Layout: sky band on top (stars, moon, antenna, water tower), then a setback
    // tower: shaft core full height on the right (5 cols per lift, shared walls),
    // offices stepping in toward the top on the left. floor_h = 2 rows: odd row
    // = slab, even row = room/cab row. Floor 0 is the lobby at the bottom.
    let block_w = lifts * 4 + 1;
    let x0 = (width - block_w - 1).max(1);
    let ground_y = height - 1;
    let sky_max = (height.saturating_sub(6)) / 2;
    let sky_h = ((height - 8) / 6).clamp(0, 4).min(sky_max);
    let floors = (height - 2 - sky_h) / 2;
    if floors < 2 {
        return;
    }
    let row_y = |f: usize| sky_h as f32 + 2.0 + 2.0 * (floors - 1 - f) as f32;
    // Setback tiers: the top 20% of floors sit on the mid 20% sit on the base.
    let tier2 = floors * 3 / 5;
    let tier3 = floors * 4 / 5;
    let inset = |f: usize| if f >= tier3.max(tier2 + 1) { 6 } else if f >= tier2 { 3 } else { 0 };
    let roof_y = sky_h;

    let wall_c = darken(palette[1], 34);
    let slab_c = darken(palette[1], 52);
    let roof_c = darken(palette[1], 18);
    let ground_c = darken(palette[1], 62);
    let cab_c = lighten(palette[2], 8);
    let inner_c = darken(palette[1], 58);
    let rider_c = lighten(palette[4], 10);
    let wait_c = lighten(palette[3], 12);
    let lamp_c = lighten(palette[4], 20);
    let win_lit = lighten(palette[3], 18);
    let win_dark = darken(palette[1], 48);
    let cable_c = darken(palette[1], 58);
    let cw_c = darken(palette[1], 22);
    let spill_c = lighten(palette[3], 8);
    let star_c = lighten(palette[1], 26);
    let moon_c = lighten(palette[1], 58);

    // Sky band: star field, then moon disc with a halo.
    for y in 0..sky_h {
        for x in 1..width - 1 {
            let h = emix(seed ^ 0x5EED ^ (y as u64 * 331).wrapping_add(x as u64 * 7));
            if h % 1000 < 8 {
                pp_put(grid, x as i32, y as i32, '·', star_c);
            } else if h % 1000 < 12 {
                pp_put(grid, x as i32, y as i32, '✦', star_c);
            }
        }
    }
    if sky_h >= 2 {
        let mh = emix(seed ^ 0xA0AA);
        let mcx = 4 + (mh % ((x0 / 2).max(3) as u64)) as usize;
        let mcy = sky_h / 2;
        for (dx, dy, ch) in
            [(0i32, 0i32, '●'), (-1, 0, '∙'), (0, -1, '∙'), (0, 1, '∙'), (2, 0, '·'), (1, 1, '·')]
        {
            pp_put(grid, mcx as i32 + dx, mcy as i32 + dy, ch, moon_c);
        }
    }

    // Setback facade: left wall per floor band, roofline, stepped slabs,
    // terraces with parapets where the tower steps in.
    for f in 0..floors {
        let sy = row_y(f) as usize - 1; // slab above this floor's room row
        let ins = inset(f);
        for x in 1 + ins..x0 - 1 {
            grid[sy][x] = Cell::new('─', slab_c);
        }
        // terrace where the tier above steps back (this slab is its floor)
        if f + 1 < floors {
            let ins_next = inset(f + 1);
            if ins_next > ins {
                for x in 1 + ins_next..1 + ins + 3 {
                    grid[sy][x] = Cell::new('═', slab_c);
                }
                let ry = sy - 1;
                grid[ry][1 + ins] = Cell::new('┌', wall_c);
                for x in 1 + ins + 1..ins + 3 {
                    grid[ry][x] = Cell::new('─', wall_c);
                }
                grid[ry][3 + ins] = Cell::new('┐', wall_c);
                let h = emix(seed ^ 0x1E5A ^ (sy as u64 * 41));
                if h % 3 == 0 {
                    grid[sy][2 + ins] = Cell::new('♣', win_lit);
                }
            }
        }
        // facade wall on the room rows of this band
        let ry = sy + 1;
        grid[ry][1 + ins] = Cell::new('▐', wall_c);
    }
    if x0 + block_w + 2 < width {
        for sy in (row_y(floors - 1) as usize - 1..ground_y).step_by(2) {
            for x in x0 + block_w + 1..width - 1 {
                grid[sy][x] = Cell::new('─', slab_c);
            }
        }
    }

    // Rooftop: roofline, AC units on the top tier, water tower + beacon antenna.
    for x in 1..width - 1 {
        grid[roof_y][x] = Cell::new('▄', roof_c);
    }
    for x in (2 + inset(floors - 1)..x0 - 3).step_by(7) {
        let h = emix(seed ^ 0xAC1D ^ (x as u64 * 29));
        if h % 100 < 45 {
            grid[roof_y][x] = Cell::new('▤', roof_c);
        }
    }
    if sky_h >= 2 {
        let th = emix(seed ^ 0x70E3);
        let tx = 1 + inset(floors - 1) + 3 + (th % 7) as usize;
        grid[roof_y - 2][tx] = Cell::new('▛', roof_c);
        grid[roof_y - 2][tx + 1] = Cell::new('▜', roof_c);
        grid[roof_y - 1][tx] = Cell::new('¦', roof_c);
        grid[roof_y - 1][tx + 1] = Cell::new('¦', roof_c);
    }
    let ax = 2 + inset(floors - 1);
    for y in 0..sky_h {
        grid[y][ax] = Cell::new('¦', wall_c);
    }

    // Ground line + storefront + shaft lobby doors.
    for x in 1..width - 1 {
        pp_put(grid, x as i32, ground_y as i32, '▂', ground_c);
    }
    {
        let ry = row_y(0) as usize;
        let dh = emix(seed ^ 0x570E3);
        let dx = 3 + (dh % ((x0 / 3).max(2) as u64)) as usize;
        if dx + 4 < x0 - 2 {
            grid[ry][dx] = Cell::new('▤', win_lit);
            grid[ry][dx + 1] = Cell::new('▤', win_lit);
            let sy = ry - 1;
            for x in dx - 1..dx + 3 {
                grid[sy][x] = Cell::new('▄', win_lit);
            }
            grid[ry][dx + 3] = Cell::new('♣', win_lit);
        }
    }
    for i in 0..lifts {
        let sx = x0 + i * 4;
        for y in roof_y + 1..ground_y {
            grid[y][sx] = Cell::new('│', wall_c);
            grid[y][sx + 4] = Cell::new('│', wall_c);
        }
        // lobby doors at the shaft base
        pp_put(grid, (sx + 1) as i32, ground_y as i32, '∙', wall_c);
        pp_put(grid, (sx + 2) as i32, ground_y as i32, '◉', lamp_c);
        pp_put(grid, (sx + 3) as i32, ground_y as i32, '∙', wall_c);
    }

    // Office windows: seeded mix, with a slow tenant-churn flicker driven by
    // the same clock that re-rolls the waiting crowds. Some windows toggle
    // on/off between tenant cycles; nobody home in the top two floors.
    let pc_pre = ((t * speed) / 7.5).floor().max(0.0) as u64;
    for f in 1..floors.saturating_sub(2) {
        let ry = row_y(f) as usize;
        let ins = inset(f);
        let mut x = 3 + ins;
        while x + 1 < x0 - 3 {
            let h = emix(seed ^ 0x51ED ^ (ry as u64 * 97).wrapping_add(x as u64 * 13));
            let h2 = emix(h ^ pc_pre.wrapping_mul(0x9E37_79B9));
            let lit = (h % 100 < 30) ^ (h2 % 100 < 9);
            let (ch, col) = if lit {
                ('☼', win_lit)
            } else {
                ('□', win_dark)
            };
            grid[ry][x] = Cell::new(ch, col);
            x += 3;
        }
        let xr0 = x0 + block_w + 2;
        if xr0 + 2 < width - 1 {
            let mut x = xr0;
            while x < width - 2 {
                let h = emix(seed ^ 0x77AA ^ (ry as u64 * 89).wrapping_add(x as u64 * 17));
                let (ch, col) = if h % 100 < 26 {
                    ('☼', win_lit)
                } else {
                    ('□', win_dark)
                };
                grid[ry][x] = Cell::new(ch, col);
                x += 3;
            }
        }
    }

    // Per-lift service loop: seeded floor sequence, eased travel, doored dwell.
    let mut serviced_until = vec![f32::NEG_INFINITY; floors];

    struct Lift {
        seq: Vec<usize>,
        trav: Vec<f32>,
        dwell: f32,
        cycle: f32,
        sx: usize,
        cab_col: Color,
    }
    let mut lift_setups: Vec<Lift> = Vec::new();
    for i in 0..lifts {
        let mut lcg = seed ^ (i as u64 + 1).wrapping_mul(0xD1B5_4A32_D192_ED03);
        let mut next = || {
            lcg = lcg
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            lcg
        };
        let mut seq = vec![0usize];
        for _ in 0..(floors * 2).max(4) {
            let mut f = (next() % floors as u64) as usize;
            while f == *seq.last().unwrap() {
                f = (next() % floors as u64) as usize;
            }
            seq.push(f);
        }
        let dwell = 1.15f32;
        let mut trav = Vec::new();
        for k in 0..seq.len() - 1 {
            let d = seq[k].max(seq[k + 1]) - seq[k].min(seq[k + 1]);
            trav.push(0.22 + 0.16 * d as f32);
        }
        let cycle: f32 = trav.iter().sum::<f32>() + dwell * (seq.len() - 1) as f32;
        let hue_jitter = rng.random_range(-14.0..14.0);
        let cab_col = shift_hue(cab_c, hue_jitter);
        lift_setups.push(Lift {
            seq,
            trav,
            dwell,
            cycle,
            sx: x0 + i * 4,
            cab_col,
        });
    }

    let t_abs = t * speed;
    // Tenant clock: waiting crowds re-roll every P seconds; a floor stays
    // "serviced" (no crowd drawn) for one window after a cab doors closed there.
    let window = 7.5f32;
    let pc = (t_abs / window).floor().max(0.0) as u64;

    struct Frame {
        y: f32,
        dir: char,
        open_floor: Option<usize>,
        open_q: f32,
        arriving: Option<(usize, bool)>, // (floor, has rider about to board/exit)
    }
    let mut frames = Vec::new();
    for lift in &lift_setups {
        let rep0 = (t_abs / lift.cycle).floor();
        let tau = t_abs - rep0 * lift.cycle;
        let mut y = row_y(lift.seq[0]);
        let mut dir = '◇';
        let mut dwell_state: Option<(usize, f32)> = None;
        let mut arriving = None;
        let mut acc = 0.0f32;
        for k in 0..lift.seq.len() - 1 {
            let (a, b) = (lift.seq[k], lift.seq[k + 1]);
            let trav = lift.trav[k];
            let travel_end = acc + trav;
            let dwell_end = travel_end + lift.dwell;
            if tau < travel_end {
                let p = (tau - acc) / trav;
                let mut yy = row_y(a) + (row_y(b) - row_y(a)) * ease_in_out(p);
                // arrival settle: the car overshoots the floor by under half a
                // row in the last stretch and eases back onto the landing
                if p >= 0.82 {
                    let s = (p - 0.82) / 0.18;
                    let bump = if s < 0.5 { s * 2.0 } else { 2.0 - 2.0 * s };
                    let sign_row = (row_y(b) - row_y(a)).signum();
                    yy += 0.45 * bump * sign_row;
                }
                y = yy;
                dir = if b > a { '▲' } else { '▼' };
                // arrival time in absolute clock decides the tenant wave boarding
                let arr_abs = t_abs + (travel_end - tau);
                let pc_arr = (arr_abs / window).floor().max(0.0) as u64;
                arriving = Some((b, elevator_waiting(seed, b as u64, pc_arr, crowd) > 0));
                acc = dwell_end;
                break;
            } else if tau < dwell_end {
                let q = (tau - travel_end) / lift.dwell;
                y = row_y(b);
                dwell_state = Some((b, q));
                let arr_abs = t_abs - q * lift.dwell;
                let pc_arr = (arr_abs / window).floor().max(0.0) as u64;
                arriving = Some((b, elevator_waiting(seed, b as u64, pc_arr, crowd) > 0 && q < 0.55));
                acc = dwell_end;
                break;
            }
            acc = dwell_end;
        }
        // Mark floors this cab has serviced inside the current tenant window.
        let mut acc2 = 0.0f32;
        for k in 0..lift.seq.len() - 1 {
            let dwell_end = acc2 + lift.trav[k] + lift.dwell;
            acc2 = dwell_end;
            let b = lift.seq[k + 1];
            let r = ((t_abs - dwell_end) / lift.cycle).floor();
            let last_end = dwell_end + r * lift.cycle;
            if t_abs - last_end < window {
                serviced_until[b] = serviced_until[b].max(last_end + window);
            }
        }
        let open_floor = match dwell_state {
            Some((f, q)) if q > 0.12 && q < 0.9 => Some(f),
            _ => None,
        };
        let open_q = dwell_state.map(|(_, q)| q).unwrap_or(0.0);
        frames.push(Frame {
            y,
            dir,
            open_floor,
            open_q,
            arriving,
        });
    }

    // Waiting tenants, boarding/exiting pedestrians, and call lamps on the
    // walkway left of the shaft block. A door open at the floor turns the
    // queue into a walk-in, then releases one rider who wanders off.
    for f in 0..floors {
        let ry = row_y(f) as usize;
        let open_here = frames.iter().find_map(|fr| {
            fr.open_floor.filter(|&ff| ff == f).map(|_| fr)
        });
        match open_here {
            Some(fr) => {
                let q = fr.open_q;
                let has = fr.arriving.map(|(_, b)| b).unwrap_or(false);
                if q < 0.5 {
                    let t_in = q / 0.5;
                    let n = elevator_waiting(seed, f as u64, pc, crowd);
                    for j in 0..n {
                        let xs = (x0 - 2 - j) as f32;
                        pp_put(
                            grid,
                            (xs + ((x0 - 1) as f32 - xs) * t_in).round() as i32,
                            ry as i32,
                            '☻',
                            if j == 0 { wait_c } else { darken(wait_c, 22) },
                        );
                    }
                } else if has {
                    let t_out = ((q - 0.5) / 0.4).min(1.0);
                    let x = (x0 - 1) as f32 - 2.0 * t_out;
                    pp_put(grid, x.round() as i32, ry as i32, '☻', rider_c);
                }
                if q > 0.2 && q < 0.8 {
                    pp_put(grid, (x0 - 1) as i32, ry as i32, '░', spill_c);
                }
            }
            None => {
                if serviced_until[f] > t_abs {
                    continue;
                }
                let n = elevator_waiting(seed, f as u64, pc, crowd);
                for j in 0..n {
                    pp_put(
                        grid,
                        (x0 - 2 - j) as i32,
                        ry as i32,
                        '☻',
                        if j == 0 { wait_c } else { darken(wait_c, 22) },
                    );
                    pp_put(grid, (x0 - 1) as i32, ry as i32, '◆', lamp_c);
                }
            }
        }
    }

    // Cabs: cable above, mirrored counterweight, doored cabin, direction lamp.
    for (lift, fr) in lift_setups.iter().zip(frames.iter()) {
        let sx = lift.sx;
        let cy = fr.y.round() as i32;
        // hoist cable from the shaft roof down to the car
        for y in (roof_y as i32 + 1)..cy {
            pp_put(grid, (sx + 2) as i32, y, '¦', cable_c);
        }
        // counterweight mirrors the car around the shaft midpoint
        let mid = 1.0 + floors as f32;
        let cw_row = 2.0 * mid - fr.y;
        let cw_q = (cw_row.round() as i32 / 2 * 2).max(2);
        if (cw_row - fr.y).abs() > 1.6 && (cw_q as usize) < ground_y {
            pp_put(grid, (sx + 4) as i32, cw_q, '▚', cw_c);
        }
        // cabin face by door phase
        let cells: [char; 3] = if fr.open_floor.is_some() {
            let q = fr.open_q;
            if q < 0.25 {
                ['▓', '▓', '·']
            } else if q < 0.75 {
                ['·', if fr.arriving.map(|(_, b)| b).unwrap_or(false) { '☻' } else { '·' }, '·']
            } else if q < 0.88 {
                ['·', '·', '▓']
            } else {
                ['▓', '▓', '▓']
            }
        } else if fr.dir != '◇' {
            ['▓', if fr.arriving.map(|(_, b)| b).unwrap_or(false) { '☻' } else { '▓' }, '▓']
        } else {
            ['▓', '▓', '▓']
        };
        for (j, ch) in cells.iter().enumerate() {
            let col = if *ch == '▓' {
                lift.cab_col
            } else if *ch == '☻' {
                rider_c
            } else if fr.open_floor.is_some() {
                lighten(palette[3], 6)
            } else {
                inner_c
            };
            pp_put(grid, (sx + 1 + j) as i32, cy, *ch, col);
        }
        if fr.dir != '◇' {
            pp_put(grid, (sx + 2) as i32, cy - 1, fr.dir, lighten(lift.cab_col, 14));
        }
    }

    // Antenna beacon: square-wave blink (no sine phase, plain on/off duty cycle).
    let bx = 2 + inset(floors - 1);
    let on = (t_abs * 1.6) % 2.0 < 1.0;
    pp_put(
        grid,
        bx as i32,
        0,
        if on { '✦' } else { '·' },
        if on { lighten(palette[4], 25) } else { darken(palette[1], 40) },
    );
}

/// Dispatch arm for mode(s): eyes (moved verbatim from run()).
pub(crate) fn cli_eyes(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        // eyes [density] [mutation] -- maximalist field of varied staring forms
        let density: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(42);
        let density = density.clamp(8, 120);
        let mutation: usize = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(78);
        let mutation = mutation.clamp(0, 140);

        let bg = darken(palette[0], 10);
        let vein = darken(palette[2], 42);
        let lid_base = lighten(palette[1], 18);
        let sclera = lighten(palette[4], 2);
        let iris_base = lighten(palette[3], 28);
        let pupil = darken(palette[0], 4);
        let glare = lighten(palette[4], 25);

        for y in 0..height {
            for x in 0..width {
                let n = (x * 13 + y * 19 + seed as usize * 7) % 67;
                let ch = match n {
                    0 => '·',
                    1 => '∙',
                    2 => '°',
                    3 if mutation > 65 => '╎',
                    _ => ' ',
                };
                grid[y][x] = if ch == ' ' {
                    Cell::new(' ', bg)
                } else {
                    Cell::new(ch, vein)
                };
            }
        }

        let put = |grid: &mut Grid, x: i32, y: i32, ch: char, fg: Color| {
            if x >= 0 && y >= 0 && (x as usize) < width && (y as usize) < height {
                grid[y as usize][x as usize] = Cell::new(ch, fg);
            }
        };
        let line_char = |dx: i32, dy: i32| {
            if dx.abs() > dy.abs() * 2 {
                '─'
            } else if dy.abs() > dx.abs() * 2 {
                '│'
            } else if dx.signum() == dy.signum() {
                '╲'
            } else {
                '╱'
            }
        };
        let draw_line =
            |grid: &mut Grid, mut x0: i32, mut y0: i32, x1: i32, y1: i32, ch: char, fg: Color| {
                let dx = (x1 - x0).abs();
                let sx = if x0 < x1 { 1 } else { -1 };
                let dy = -(y1 - y0).abs();
                let sy = if y0 < y1 { 1 } else { -1 };
                let mut err = dx + dy;
                loop {
                    put(grid, x0, y0, ch, fg);
                    if x0 == x1 && y0 == y1 {
                        break;
                    }
                    let e2 = 2 * err;
                    if e2 >= dy {
                        err += dy;
                        x0 += sx;
                    }
                    if e2 <= dx {
                        err += dx;
                        y0 += sy;
                    }
                }
            };
        let draw_eye = |grid: &mut Grid,
                        cx: i32,
                        cy: i32,
                        rx: i32,
                        ry: i32,
                        style: usize,
                        gaze_x: i32,
                        gaze_y: i32,
                        lid_color: Color,
                        iris_color: Color,
                        rng: &mut StdRng| {
            let rx = rx.max(2);
            let ry = ry.max(1);
            let iris_rx = (rx / 3).max(1);
            let iris_ry = (ry / 2).max(1);
            let pupil_rx = (iris_rx / 2).max(1);
            let pupil_ry = if style % 4 == 0 {
                iris_ry.max(2)
            } else {
                1.max(iris_ry / 2)
            };
            let blink_cut = if style % 9 == 3 { 0.28 } else { 1.0 };

            for dy in -ry - 1..=ry + 1 {
                for dx in -rx - 1..=rx + 1 {
                    let nx = dx as f32 / rx as f32;
                    let ny = dy as f32 / ry as f32;
                    let metric = nx * nx + ny * ny;
                    if metric > 1.28 || ny.abs() > blink_cut {
                        continue;
                    }
                    let x = cx + dx;
                    let y = cy + dy;
                    let edge = (metric - 1.0).abs();
                    if edge < 0.26 || dy.abs() == ry {
                        let ch = if dy < -ry / 3 {
                            if dx < -rx / 2 {
                                '╭'
                            } else if dx > rx / 2 {
                                '╮'
                            } else {
                                '─'
                            }
                        } else if dy > ry / 3 {
                            if dx < -rx / 2 {
                                '╰'
                            } else if dx > rx / 2 {
                                '╯'
                            } else {
                                '─'
                            }
                        } else if dx < 0 {
                            '╱'
                        } else if dx > 0 {
                            '╲'
                        } else {
                            '│'
                        };
                        put(grid, x, y, ch, lid_color);
                        continue;
                    }

                    let idy = dy - gaze_y;
                    let idx = dx - gaze_x;
                    let im = (idx as f32 / iris_rx as f32).powi(2)
                        + (idy as f32 / iris_ry as f32).powi(2);
                    if im <= 1.0 {
                        let pm = (idx as f32 / pupil_rx as f32).powi(2)
                            + (idy as f32 / pupil_ry as f32).powi(2);
                        if pm <= 1.0 {
                            let ch = match style % 8 {
                                0 => '┃',
                                1 => '●',
                                2 => '█',
                                3 => '◆',
                                4 => '◉',
                                5 => '◐',
                                6 => '◍',
                                _ => '◎',
                            };
                            put(grid, x, y, ch, pupil);
                        } else {
                            let ch = match (style
                                + dx.unsigned_abs() as usize
                                + dy.unsigned_abs() as usize)
                                % 7
                            {
                                0 => '◌',
                                1 => '○',
                                2 => '◍',
                                3 => '◐',
                                4 => '◑',
                                5 => '·',
                                _ => '•',
                            };
                            put(grid, x, y, ch, iris_color);
                        }
                    } else {
                        let ch = match (style
                            + dx.unsigned_abs() as usize * 2
                            + dy.unsigned_abs() as usize)
                            % 9
                        {
                            0 => '·',
                            1 => '∙',
                            2 if style % 5 == 0 => '╎',
                            3 if style % 7 == 0 => '◇',
                            _ => ' ',
                        };
                        put(
                            grid,
                            x,
                            y,
                            ch,
                            if ch == ' ' {
                                sclera
                            } else {
                                darken(sclera, 35)
                            },
                        );
                    }
                }
            }

            put(
                grid,
                cx + gaze_x - iris_rx / 2,
                cy + gaze_y - iris_ry,
                '˙',
                glare,
            );
            if style % 3 == 0 {
                put(
                    grid,
                    cx + gaze_x + iris_rx / 2,
                    cy + gaze_y + iris_ry,
                    '·',
                    glare,
                );
            }
            if style % 5 == 0 {
                put(grid, cx - rx, cy, '<', lid_color);
                put(grid, cx + rx, cy, '>', lid_color);
            }

            if style % 2 == 0 {
                let lash_count = (3 + mutation / 28).min(8);
                for i in 0..lash_count {
                    let t = if lash_count <= 1 {
                        0.5
                    } else {
                        i as f32 / (lash_count - 1) as f32
                    };
                    let lx = cx - rx + (t * rx as f32 * 2.0).round() as i32;
                    let ly = cy - ry + ((t - 0.5).abs() * 2.0).round() as i32;
                    let lean = rng.random_range(-2..=2);
                    let len = rng.random_range(1..=3 + (mutation / 50) as i32);
                    let tx = lx + lean;
                    let ty = ly - len;
                    draw_line(
                        grid,
                        lx,
                        ly,
                        tx,
                        ty,
                        line_char(tx - lx, ty - ly),
                        darken(lid_color, 8),
                    );
                }
            }
            if style % 6 == 1 {
                for side in [-1, 1] {
                    for k in 1..=3 {
                        put(
                            grid,
                            cx + side * (rx + k),
                            cy + (k % 2) - 1,
                            '·',
                            darken(iris_color, 20),
                        );
                    }
                }
            }
        };

        let mut row_y = 2i32;
        while row_y < height as i32 {
            let mut x = rng.random_range(-6..=3);
            while x < width as i32 + 6 {
                let rx = rng.random_range(3..=(7 + density / 22) as i32).min(12);
                let ry = rng.random_range(1..=3 + (mutation / 70) as i32).min(5);
                let style = rng.random_range(0..18usize);
                let gaze_x = rng.random_range(-(rx / 4).max(1)..=(rx / 4).max(1));
                let gaze_y = rng.random_range(-(ry / 3).max(0)..=(ry / 3).max(0));
                let lid = shift_hue(lid_base, rng.random_range(-45..=55) as f64);
                let iris = shift_hue(iris_base, rng.random_range(-120..=120) as f64);
                draw_eye(
                    &mut grid, x, row_y, rx, ry, style, gaze_x, gaze_y, lid, iris, &mut rng,
                );
                x += rng
                    .random_range(7i32..=13i32)
                    .saturating_sub(density as i32 / 24);
            }
            row_y += rng.random_range(3..=5);
        }

        let large_count = (3 + mutation / 25).min(8);
        for i in 0..large_count {
            let rx = rng.random_range(7..=(width / 4).max(9) as i32).min(22);
            let ry = rng.random_range(3..=(height / 4).max(4) as i32).min(8);
            let cx = rng.random_range(-(rx / 2)..=(width as i32 + rx / 2));
            let cy = rng.random_range(1..height as i32);
            let style = i + rng.random_range(0..24usize);
            let gaze_x = rng.random_range(-(rx / 3)..=(rx / 3));
            let gaze_y = rng.random_range(-(ry / 3)..=(ry / 3));
            let lid = shift_hue(lighten(lid_base, 14), rng.random_range(-80..=80) as f64);
            let iris = shift_hue(lighten(iris_base, 18), rng.random_range(-160..=160) as f64);
            draw_eye(
                &mut grid, cx, cy, rx, ry, style, gaze_x, gaze_y, lid, iris, &mut rng,
            );
        }

        let sigils = ['◉', '◎', '◌', '◍', '◐', '◑', '●', '•', '˙'];
        for _ in 0..density {
            let x = rng.random_range(0..width) as i32;
            let y = rng.random_range(0..height) as i32;
            if rng.random::<f32>() < mutation as f32 / 170.0 {
                put(
                    &mut grid,
                    x,
                    y,
                    sigils[rng.random_range(0..sigils.len())],
                    shift_hue(iris_base, rng.random_range(-180..=180) as f64),
                );
            }
        }
    (grid, false)
}

/// Dispatch arm for mode(s): eyes2 (moved verbatim from run()).
pub(crate) fn cli_eyes2(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        // eyes2 [count] [pupil-visible] -- anatomical eyes all staring at a focal lure
        let eye_count: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(6);
        let eye_count = eye_count.clamp(3, 20);
        let pupil_visible: usize = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(80);
        let pupil_visible = pupil_visible.clamp(50, 100);

        let bg = darken(palette[0], 12);
        let lid_base = lighten(palette[1], 12);
        let sclera = lighten(palette[4], 4);
        let iris_base = lighten(palette[3], 18);
        let pupil = darken(palette[0], 2);
        let shadow = darken(palette[2], 48);
        let highlight = lighten(palette[4], 26);

        for y in 0..height {
            for x in 0..width {
                let n = (x * 17 + y * 29 + seed as usize * 3) % 91;
                let ch = match n {
                    0 => '·',
                    1 if (x + y) % 3 == 0 => '∙',
                    2 if (x + seed as usize) % 11 == 0 => '°',
                    _ => ' ',
                };
                grid[y][x] = if ch == ' ' {
                    Cell::new(' ', bg)
                } else {
                    Cell::new(ch, shadow)
                };
            }
        }

        let put = |grid: &mut Grid, x: i32, y: i32, ch: char, fg: Color| {
            if x >= 0 && y >= 0 && (x as usize) < width && (y as usize) < height {
                grid[y as usize][x as usize] = Cell::new(ch, fg);
            }
        };
        let draw_line =
            |grid: &mut Grid, mut x0: i32, mut y0: i32, x1: i32, y1: i32, ch: char, fg: Color| {
                let dx = (x1 - x0).abs();
                let sx = if x0 < x1 { 1 } else { -1 };
                let dy = -(y1 - y0).abs();
                let sy = if y0 < y1 { 1 } else { -1 };
                let mut err = dx + dy;
                loop {
                    put(grid, x0, y0, ch, fg);
                    if x0 == x1 && y0 == y1 {
                        break;
                    }
                    let e2 = 2 * err;
                    if e2 >= dy {
                        err += dy;
                        x0 += sx;
                    }
                    if e2 <= dx {
                        err += dx;
                        y0 += sy;
                    }
                }
            };
        let draw_eye = |grid: &mut Grid,
                        cx: i32,
                        cy: i32,
                        rx: i32,
                        ry: i32,
                        open_pct: usize,
                        gaze_x: i32,
                        gaze_y: i32,
                        slant: f32,
                        style: usize,
                        lid_color: Color,
                        iris_color: Color| {
            let rx = rx.max(5);
            let ry = ry.max(2);
            let open = (open_pct as f32 / 100.0).clamp(0.50, 1.0);
            let iris_rx = ((rx as f32 * 0.27).round() as i32).max(2);
            let iris_ry = ((ry as f32 * 0.72).round() as i32).max(2);
            let pupil_rx = ((iris_rx as f32 * 0.42).round() as i32).max(1);
            let pupil_ry = if style % 4 == 0 {
                iris_ry.max(2)
            } else {
                ((iris_ry as f32 * 0.62).round() as i32).max(1)
            };

            for dx in -rx - 2..=rx + 2 {
                let nx = dx as f32 / rx as f32;
                if nx.abs() > 1.06 {
                    continue;
                }
                let curve = (1.0 - nx.abs().powf(1.72)).max(0.0).powf(0.56);
                let top = (-ry as f32 * open * curve - nx * slant).round() as i32;
                let bottom = (ry as f32 * open * 0.84 * curve + nx * slant * 0.38).round() as i32;
                if bottom < top {
                    continue;
                }
                for dy in top..=bottom {
                    let x = cx + dx;
                    let y = cy + dy;
                    let on_top = dy == top;
                    let on_bottom = dy == bottom;
                    if on_top || on_bottom {
                        let edge = dx.abs() as f32 / rx as f32;
                        let ch = if dx <= -rx {
                            if dx < 0 { '<' } else { '>' }
                        } else if dx >= rx {
                            '>'
                        } else if edge < 0.82 {
                            '─'
                        } else if on_top {
                            if dx < 0 { '╱' } else { '╲' }
                        } else if dx < 0 {
                            '╲'
                        } else {
                            '╱'
                        };
                        put(grid, x, y, ch, lid_color);
                        continue;
                    }

                    let idx = dx - gaze_x;
                    let idy = dy - gaze_y;
                    let im = (idx as f32 / iris_rx as f32).powi(2)
                        + (idy as f32 / iris_ry as f32).powi(2);
                    if im <= 1.08 {
                        let pm = (idx as f32 / pupil_rx as f32).powi(2)
                            + (idy as f32 / pupil_ry as f32).powi(2);
                        if pm <= 0.56 {
                            let ch = match style % 6 {
                                0 => '│',
                                1 => '●',
                                2 => '◐',
                                3 => '◑',
                                4 => '◉',
                                _ => '┃',
                            };
                            put(grid, x, y, ch, pupil);
                        } else if im > 0.72 {
                            let ch = if (idx + idy + style as i32) % 3 == 0 {
                                '◌'
                            } else {
                                '○'
                            };
                            put(grid, x, y, ch, darken(iris_color, 8));
                        } else {
                            let ch = match (idx.abs() + idy.abs() + style as i32) % 7 {
                                0 => '╎',
                                1 | 2 | 3 => '·',
                                4 => '∙',
                                _ => '˙',
                            };
                            put(grid, x, y, ch, iris_color);
                        }
                    } else if (dx * 5 + dy * 7 + style as i32) % 37 == 0 {
                        put(grid, x, y, '·', darken(sclera, 45));
                    } else if (dx * 3 + dy * 11 + style as i32) % 11 == 0 {
                        put(grid, x, y, '·', darken(sclera, 24));
                    } else {
                        put(grid, x, y, ' ', sclera);
                    }
                }
            }

            put(
                grid,
                cx + gaze_x - iris_rx / 2,
                cy + gaze_y - iris_ry / 2,
                '˙',
                highlight,
            );
            if style % 5 == 0 {
                let lid_y = cy + gaze_y - iris_ry / 2;
                draw_line(
                    grid,
                    cx - iris_rx,
                    lid_y,
                    cx + iris_rx,
                    lid_y,
                    '─',
                    darken(lid_color, 2),
                );
            }
        };

        let focus_x = (width as i32 / 2
            + rng.random_range(-(width as i32 / 12)..=(width as i32 / 12)))
        .clamp(6, width as i32 - 7);
        let focus_y = ((height as f32 * 0.72).round() as i32
            + rng.random_range(-(height as i32 / 18)..=(height as i32 / 18)))
        .clamp(8, height as i32 - 5);
        let lure_color = shift_hue(lighten(iris_base, 30), 55.0);
        for dy in -3i32..=3i32 {
            for dx in -6i32..=6i32 {
                let metric = (dx as f32 / 6.0).powi(2) + (dy as f32 / 3.0).powi(2);
                if metric <= 1.0 && (dx.abs() + dy.abs()) % 2 == 0 {
                    put(
                        &mut grid,
                        focus_x + dx,
                        focus_y + dy,
                        '·',
                        darken(lure_color, 22),
                    );
                }
            }
        }
        draw_line(
            &mut grid,
            focus_x,
            focus_y - 3,
            focus_x,
            focus_y + 2,
            '│',
            darken(lure_color, 8),
        );
        draw_line(
            &mut grid,
            focus_x - 3,
            focus_y,
            focus_x + 3,
            focus_y,
            '─',
            darken(lure_color, 8),
        );
        put(&mut grid, focus_x, focus_y, '◆', lighten(lure_color, 12));
        put(&mut grid, focus_x, focus_y - 2, '◇', lighten(highlight, 4));
        put(
            &mut grid,
            focus_x - 2,
            focus_y + 2,
            '╲',
            darken(lure_color, 2),
        );
        put(
            &mut grid,
            focus_x + 2,
            focus_y + 2,
            '╱',
            darken(lure_color, 2),
        );

        let gaze_for = |ex: i32, ey: i32, rx: i32, ry: i32| {
            let dx = (focus_x - ex) as f32;
            let dy = (focus_y - ey) as f32;
            let dist = (dx * dx + dy * dy).sqrt().max(1.0);
            let gx = ((dx / dist) * (rx as f32 * 0.24)).round() as i32;
            let gy = ((dy / dist) * (ry as f32 * 0.42)).round() as i32;
            let gx = gx.clamp(-(rx / 4).max(1), (rx / 4).max(1));
            let gy = gy.clamp(-(ry / 2).max(1), (ry / 2).max(1));
            let slant = (dx / dist * 1.35).clamp(-1.2, 1.2);
            (gx, gy, slant)
        };
        let draw_dotted_line =
            |grid: &mut Grid, mut x0: i32, mut y0: i32, x1: i32, y1: i32, fg: Color| {
                let dx = (x1 - x0).abs();
                let sx = if x0 < x1 { 1 } else { -1 };
                let dy = -(y1 - y0).abs();
                let sy = if y0 < y1 { 1 } else { -1 };
                let mut err = dx + dy;
                let mut step = 0usize;
                loop {
                    if step % 4 == 0
                        && x0 >= 0
                        && y0 >= 0
                        && (x0 as usize) < width
                        && (y0 as usize) < height
                        && grid[y0 as usize][x0 as usize].ch == ' '
                    {
                        put(grid, x0, y0, '·', fg);
                    }
                    if x0 == x1 && y0 == y1 {
                        break;
                    }
                    let e2 = 2 * err;
                    if e2 >= dy {
                        err += dy;
                        x0 += sx;
                    }
                    if e2 <= dx {
                        err += dx;
                        y0 += sy;
                    }
                    step += 1;
                }
            };

        let mut eye_specs: Vec<(i32, i32, i32, i32, usize, usize)> = Vec::new();
        for i in 0..eye_count {
            let t = (i as f32 + 0.5) / eye_count as f32;
            let angle =
                std::f32::consts::PI + t * std::f32::consts::PI + rng.random_range(-0.20..0.20);
            let arc_rx = width as f32 * rng.random_range(0.34..0.52);
            let arc_ry = height as f32 * rng.random_range(0.28..0.52);
            let mut ex = (focus_x as f32 + angle.cos() * arc_rx).round() as i32;
            let mut ey = (focus_y as f32 + angle.sin() * arc_ry).round() as i32;
            let mut rx = rng.random_range(7..=13);
            let mut ry = rng.random_range(3..=6);
            if i == eye_count / 2 {
                rx = ((width as f32 * 0.18).round() as i32).clamp(12, 20);
                ry = ((height as f32 * 0.20).round() as i32).clamp(4, 7);
                ex = (width as i32 / 2 + rng.random_range(-4..=4))
                    .clamp(rx + 2, width as i32 - rx - 3);
                ey = (focus_y - (height as i32 / 3).max(6) + rng.random_range(-2..=2))
                    .clamp(ry + 3, height as i32 - ry - 4);
            }
            ex = ex.clamp(-rx / 2, width as i32 + rx / 2);
            ey = ey.clamp(ry + 2, height as i32 - ry - 3);
            let visible =
                (pupil_visible as i32 + rng.random_range(-8..=14)).clamp(50, 100) as usize;
            eye_specs.push((ex, ey, rx, ry, visible, i));
        }

        for &(ex, ey, rx, ry, _, i) in &eye_specs {
            let (gx, gy, _) = gaze_for(ex, ey, rx, ry);
            let iris_x = ex + gx;
            let iris_y = ey + gy;
            draw_dotted_line(
                &mut grid,
                iris_x,
                iris_y,
                focus_x,
                focus_y,
                darken(shift_hue(iris_base, i as f64 * 23.0), 35),
            );
        }
        for &(ex, ey, rx, ry, visible, i) in &eye_specs {
            let (gaze_x, gaze_y, slant) = gaze_for(ex, ey, rx, ry);
            let lid = shift_hue(lid_base, rng.random_range(-34..=42) as f64);
            let iris = shift_hue(iris_base, rng.random_range(-120..=120) as f64);
            draw_eye(
                &mut grid, ex, ey, rx, ry, visible, gaze_x, gaze_y, slant, i, lid, iris,
            );
        }
    (grid, false)
}

/// Dispatch arm for mode(s): metro (moved verbatim from run()).
pub(crate) fn cli_metro(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        // metro [lines] -- transit map: orthogonal routes with rounded bends,
        // stations along each run, interchange rings where routes cross
        let line_count: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(5);
        let line_count = line_count.clamp(2, 9);

        let grid_dot = darken(palette[1], 82);
        for y in (0..height).step_by(3) {
            for x in (0..width).step_by(6) {
                grid[y][x] = Cell::new('·', grid_dot);
            }
        }

        // per-cell line ids by orientation (id = line index + 1); an interchange
        // is where two different lines meet in different orientations, so
        // parallel overlapping runs don't smear into rings and a line never
        // rings against its own bends
        let mut occ_h = vec![vec![0u8; width]; height];
        let mut occ_v = vec![vec![0u8; width]; height];
        macro_rules! rail {
            ($x:expr, $y:expr, $ch:expr, $col:expr, $id:expr, $dirbits:expr) => {{
                let sx = $x;
                let sy = $y;
                if sx >= 0 && sy >= 0 && (sx as usize) < width && (sy as usize) < height {
                    grid[sy as usize][sx as usize] = Cell::new($ch, $col);
                    // first writer keeps the cell: a later corner over an earlier
                    // straight run still registers as two lines meeting
                    if $dirbits & 1 != 0 && occ_h[sy as usize][sx as usize] == 0 {
                        occ_h[sy as usize][sx as usize] = $id;
                    }
                    if $dirbits & 2 != 0 && occ_v[sy as usize][sx as usize] == 0 {
                        occ_v[sy as usize][sx as usize] = $id;
                    }
                }
            }};
        }

        // line 0 is the circle line; the rest alternate between top-to-bottom
        // and left-to-right so the map fills both axes
        for li in 0..line_count {
            let id = (li + 1) as u8;
            let base = [palette[1], palette[2], palette[3]][li % 3];
            let col = shift_hue(lighten(base, 8), li as f64 * 37.0);

            if li == 0 && line_count >= 3 {
                let x0 = rng.random_range(width as i32 / 8..(width as i32 / 3).max(width as i32 / 8 + 1));
                let x1 = rng.random_range(width as i32 * 3 / 5..(width as i32 * 7 / 8).max(width as i32 * 3 / 5 + 1));
                let y0 = rng.random_range(2..(height as i32 / 3).max(3));
                let y1 = rng.random_range(height as i32 * 3 / 5..(height as i32 - 2).max(height as i32 * 3 / 5 + 1));
                let mut until_station: i32 = rng.random_range(3..6);
                for x in x0 + 1..x1 {
                    for yy in [y0, y1] {
                        until_station -= 1;
                        if until_station <= 0 {
                            rail!(x, yy, '○', lighten(col, 30), id, 1);
                            until_station = rng.random_range(5..10);
                        } else {
                            rail!(x, yy, '─', col, id, 1);
                        }
                    }
                }
                for y in y0 + 1..y1 {
                    for xx in [x0, x1] {
                        if rng.random_range(0..7) == 0 {
                            rail!(xx, y, '○', lighten(col, 30), id, 2);
                        } else {
                            rail!(xx, y, '│', col, id, 2);
                        }
                    }
                }
                rail!(x0, y0, '╭', col, id, 3);
                rail!(x1, y0, '╮', col, id, 3);
                rail!(x0, y1, '╰', col, id, 3);
                rail!(x1, y1, '╯', col, id, 3);
                continue;
            }

            if li % 2 == 0 {
                // vertical-major: top to bottom with horizontal jogs
                let mut x: i32 = rng.random_range(3..(width as i32 - 3).max(4));
                let mut y: i32 = 0;
                let mut until_station: i32 = rng.random_range(3..7);
                while y < height as i32 {
                    let run: i32 = rng.random_range(4..9);
                    for _ in 0..run {
                        if y >= height as i32 {
                            break;
                        }
                        until_station -= 1;
                        if until_station <= 0 {
                            rail!(x, y, '○', lighten(col, 30), id, 2);
                            until_station = rng.random_range(4..9);
                        } else {
                            rail!(x, y, '│', col, id, 2);
                        }
                        y += 1;
                    }
                    if y >= height as i32 - 2 {
                        while y < height as i32 {
                            rail!(x, y, '│', col, id, 2);
                            y += 1;
                        }
                        break;
                    }
                    let jog: i32 =
                        rng.random_range(3..9) * if rng.random_range(0..2) == 0 { 1 } else { -1 };
                    let jog = if x + jog < 1 {
                        jog.abs()
                    } else if x + jog > width as i32 - 2 {
                        -jog.abs()
                    } else {
                        jog
                    };
                    let step = jog.signum();
                    rail!(x, y, if step > 0 { '╰' } else { '╯' }, col, id, 3);
                    for _ in 0..jog.abs() - 1 {
                        x += step;
                        rail!(x, y, '─', col, id, 1);
                    }
                    x += step;
                    rail!(x, y, if step > 0 { '╮' } else { '╭' }, col, id, 3);
                    y += 1;
                }
            } else {
                // horizontal-major: left to right with vertical jogs
                let mut y: i32 = rng.random_range(2..(height as i32 - 3).max(3));
                let mut x: i32 = 0;
                let mut until_station: i32 = rng.random_range(4..9);
                while x < width as i32 {
                    let run: i32 = rng.random_range(7..16);
                    for _ in 0..run {
                        if x >= width as i32 {
                            break;
                        }
                        until_station -= 1;
                        if until_station <= 0 {
                            rail!(x, y, '○', lighten(col, 30), id, 1);
                            until_station = rng.random_range(6..13);
                        } else {
                            rail!(x, y, '─', col, id, 1);
                        }
                        x += 1;
                    }
                    if x >= width as i32 - 2 {
                        while x < width as i32 {
                            rail!(x, y, '─', col, id, 1);
                            x += 1;
                        }
                        break;
                    }
                    let jog: i32 =
                        rng.random_range(2..6) * if rng.random_range(0..2) == 0 { 1 } else { -1 };
                    let jog = if y + jog < 1 {
                        jog.abs()
                    } else if y + jog > height as i32 - 2 {
                        -jog.abs()
                    } else {
                        jog
                    };
                    let step = jog.signum();
                    rail!(x, y, if step > 0 { '╮' } else { '╯' }, col, id, 3);
                    for _ in 0..jog.abs() - 1 {
                        y += step;
                        rail!(x, y, '│', col, id, 2);
                    }
                    y += step;
                    rail!(x, y, if step > 0 { '╰' } else { '╭' }, col, id, 3);
                    x += 1;
                }
            }
        }

        for y in 0..height {
            for x in 0..width {
                if occ_h[y][x] > 0 && occ_v[y][x] > 0 && occ_h[y][x] != occ_v[y][x] {
                    grid[y][x] = Cell::new('◉', lighten(palette[4], 15));
                }
            }
        }
    (grid, false)
}

/// Dispatch arm for mode(s): koi (moved verbatim from run()).
pub(crate) fn cli_koi(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        // koi [fish] -- pond seen from above: still water, ripple rings, lily
        // pads, koi gliding with curled tails
        let fish_count: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(7);
        let fish_count = fish_count.clamp(1, 24);

        let pond = Rect {
            x: 0,
            y: 0,
            w: width,
            h: height,
        };
        fill_noise(
            &mut grid,
            &pond,
            NoiseVariant::Dot,
            darken(palette[1], 80),
            darken(palette[2], 86),
            &mut rng,
        );

        // ripple rings, squashed to ellipses
        for _ in 0..(width / 25).max(2) {
            let cx = rng.random_range(0..width) as f32;
            let cy = rng.random_range(0..height) as f32;
            let rings = rng.random_range(1..3usize);
            for ring in 0..rings {
                let r = 1.5 + ring as f32 * 2.0;
                let steps = (r * 9.0) as usize;
                let col = darken(lighten(palette[1], 8), 25 + (ring * 18) as u8);
                for s in 0..steps {
                    let a = s as f32 / steps as f32 * std::f32::consts::TAU;
                    let x = (cx + a.cos() * r * 2.0).round() as i32;
                    let y = (cy + a.sin() * r).round() as i32;
                    if x < 0 || y < 0 || x as usize >= width || y as usize >= height {
                        continue;
                    }
                    let sy = a.sin();
                    let ch = if sy < -0.55 {
                        '‾'
                    } else if sy > 0.55 {
                        '_'
                    } else if a.cos() < 0.0 {
                        '('
                    } else {
                        ')'
                    };
                    grid[y as usize][x as usize] = Cell::new(ch, col);
                }
            }
        }

        // lily pads, some flowering
        for _ in 0..(width / 16).max(3) {
            let px = rng.random_range(2..width.saturating_sub(2).max(3));
            let py = rng.random_range(1..height.saturating_sub(1).max(2));
            let pad = lighten(palette[2], rng.random_range(0..18));
            grid[py][px - 1] = Cell::new('(', darken(pad, 20));
            grid[py][px] = Cell::new('◍', pad);
            grid[py][px + 1] = Cell::new(')', darken(pad, 20));
            if py > 0 && rng.random_range(0..3) == 0 {
                grid[py - 1][px] = Cell::new('✿', lighten(palette[3], 25));
            }
        }

        // koi: bright head, body fading into the water, sine-curled tail
        let body = ['◉', '◎', '○', '∘', '·'];
        for _ in 0..fish_count {
            let dir: i32 = if rng.random_range(0..2) == 0 { 1 } else { -1 };
            let x0 = rng.random_range(6..width.saturating_sub(6).max(7)) as i32;
            let y0 = rng.random_range(1..height.saturating_sub(1).max(2)) as i32;
            let hue = if rng.random_range(0..3) == 0 {
                lighten(palette[4], 10)
            } else {
                lighten(palette[3], 15)
            };
            let phase = rng.random_range(0.0f32..6.28);
            let sway0 = phase.sin();
            for (i, &ch) in body.iter().enumerate() {
                let x = x0 - dir * i as i32;
                let y = y0 + (((i as f32) * 0.7 + phase).sin() - sway0).round() as i32;
                if x < 0 || y < 0 || x as usize >= width || y as usize >= height {
                    continue;
                }
                let col = lerp_color(hue, darken(palette[1], 60), i as f32 / 4.0);
                grid[y as usize][x as usize] = Cell::new(ch, col);
            }
        }
    (grid, false)
}

/// Dispatch arm for mode(s): skyline (moved verbatim from run()).
pub(crate) fn cli_skyline(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        // skyline [lit] -- night city in four depth layers: far slabs, mid
        // backdrop, near towers built from facade archetypes (glass curtain,
        // masonry, ziggurat, banded slab, spire, dome), and foreground hulks
        // cropped by the frame. lit = percent of windows glowing.
        let lit: u32 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(35);
        let lit = lit.clamp(0, 100);

        let horizon = height.saturating_sub(2);

        for _ in 0..(width * height / 40).max(10) {
            let x = rng.random_range(0..width);
            let y = rng.random_range(0..horizon.max(1));
            let ch = match rng.random_range(0..8) {
                0 => '✦',
                1 => '+',
                _ => '·',
            };
            grid[y][x] = Cell::new(ch, darken(palette[4], rng.random_range(30..70)));
        }

        for _ in 0..rng.random_range(2..5usize) {
            let cw = rng.random_range(5..13usize);
            let cx0 = rng.random_range(0..width.saturating_sub(cw).max(1));
            let cy = rng.random_range(1..(height / 3).max(2));
            for i in 0..cw {
                let ch = if i == 0 || i == cw - 1 { '░' } else { '▒' };
                grid[cy][cx0 + i] = Cell::new(ch, darken(palette[4], 62));
                if i > 1 && i < cw - 2 && cy + 1 < height {
                    grid[cy + 1][cx0 + i] = Cell::new('░', darken(palette[4], 70));
                }
            }
        }

        for _ in 0..rng.random_range(1..3usize) {
            let bx = rng.random_range(4..width.saturating_sub(10).max(5));
            let by = rng.random_range(2..(height / 3).max(3));
            for i in 0..rng.random_range(3..6usize) {
                let x = bx + i * 2;
                let y = by + (i % 2);
                if x < width && y < height {
                    grid[y][x] = Cell::new('∨', darken(palette[4], 35));
                }
            }
        }

        let mx = rng.random_range(width / 8..(width / 3).max(width / 8 + 1)) as i32;
        let my = rng.random_range(2..(height / 4).max(3)) as i32;
        for dy in -1..=1i32 {
            for dx in -2..=2i32 {
                let e = (dx as f32 / 2.2).powi(2) + (dy as f32 / 1.2).powi(2);
                if e <= 1.0 {
                    let x = mx + dx;
                    let y = my + dy;
                    if x >= 0 && y >= 0 && (x as usize) < width && (y as usize) < height {
                        let ch = if dx == 1 && dy == 0 { '▒' } else { '▓' };
                        grid[y as usize][x as usize] = Cell::new(ch, lighten(palette[4], 20));
                    }
                }
            }
        }

        // far layer: low distant slabs with gaps of sky
        let far = darken(palette[1], 72);
        let mut x = 0usize;
        while x < width {
            let w = rng.random_range(4..9usize).min(width - x);
            let h = rng.random_range((height / 7).max(2)..(height / 3).max(3));
            let top = horizon.saturating_sub(h);
            for bx in x..x + w {
                grid[top][bx] = Cell::new('▄', far);
                for by in top + 1..horizon {
                    grid[by][bx] = Cell::new('▓', far);
                }
            }
            x += w + rng.random_range(0..6usize);
        }

        // mid layer: continuous backdrop with sparse dim windows
        let mid = darken(palette[1], 55);
        let mid_win = darken(palette[3], 25);
        let mut x = 0usize;
        while x < width {
            let w = rng.random_range(5..11usize).min(width - x);
            let h = rng.random_range((height / 4).max(3)..(height / 2).max(4));
            let top = horizon.saturating_sub(h);
            for bx in x..x + w {
                grid[top][bx] = Cell::new('▄', mid);
                for by in top + 1..horizon {
                    grid[by][bx] = Cell::new('█', mid);
                }
            }
            for by in (top + 2..horizon.saturating_sub(1)).step_by(3) {
                for bx in (x + 1..(x + w).saturating_sub(1)).step_by(3) {
                    if rng.random_range(0..100) < lit / 2 {
                        grid[by][bx] = Cell::new('▪', mid_win);
                    }
                }
            }
            x += w + rng.random_range(2..7usize);
        }

        // near layer: randomly placed towers that overlap into clusters.
        // each rolls a facade archetype, so the variety is in the pattern
        // language of the building, never just its size
        let near = darken(palette[1], 40);
        let win_on = lighten(palette[3], 30);
        let win_off = darken(palette[1], 12);
        let mut street_free = vec![true; width];
        let mut tall_top = horizon;
        let mut tall_x = width / 2;
        // roof gear and beams only land on open sky, never inside another tower
        macro_rules! deco {
            ($x:expr, $y:expr, $ch:expr, $col:expr) => {{
                let dx = $x;
                let dy = $y;
                if !matches!(
                    grid[dy][dx].ch,
                    '█' | '▓' | '▄' | '▀' | '▪' | '▮' | '□' | '║' | '▐' | '▬' | '╥'
                ) {
                    grid[dy][dx] = Cell::new($ch, $col);
                }
            }};
        }
        for _ in 0..(width / 14).max(3) {
            let kind = rng.random_range(0..6usize);
            let w = match kind {
                0 => rng.random_range(6..11usize),
                1 => rng.random_range(7..13usize),
                2 => rng.random_range(9..15usize),
                3 => rng.random_range(8..13usize),
                4 => rng.random_range(3..5usize),
                _ => rng.random_range(7..12usize),
            };
            let w = w.min(width);
            let x = rng.random_range(0..width.saturating_sub(w).max(1));
            for bx in x..x + w {
                street_free[bx] = false;
            }
            let btop = match kind {
                0 => {
                    // glass curtain: whole mullion strips light at once
                    let h = rng.random_range((height / 2).max(5)..(height * 3 / 4).max(6));
                    let top = horizon.saturating_sub(h);
                    for bx in x..x + w {
                        grid[top][bx] = Cell::new('▄', near);
                        for by in top + 1..horizon {
                            let ch = if (bx - x) % 2 == 0 { '█' } else { '▐' };
                            grid[by][bx] = Cell::new(ch, near);
                        }
                    }
                    let mstep = rng.random_range(2..4usize);
                    for bx in (x + 1..(x + w).saturating_sub(1)).step_by(mstep) {
                        let on = rng.random_range(0..100) < lit;
                        let col = if on { win_on } else { darken(palette[1], 22) };
                        for by in top + 1..horizon.saturating_sub(1) {
                            grid[by][bx] = Cell::new('║', col);
                        }
                    }
                    top
                }
                1 => {
                    // masonry: pale cornice, string courses, square windows
                    let h = rng.random_range((height / 3).max(4)..(height / 2).max(5));
                    let top = horizon.saturating_sub(h);
                    for bx in x..x + w {
                        grid[top][bx] = Cell::new('▀', lighten(near, 14));
                        for by in top + 1..horizon {
                            grid[by][bx] = Cell::new('▓', near);
                        }
                    }
                    for by in (top + 2..horizon.saturating_sub(1)).step_by(2) {
                        for bx in (x + 1..(x + w).saturating_sub(1)).step_by(3) {
                            let on = rng.random_range(0..100) < lit;
                            grid[by][bx] = Cell::new('□', if on { win_on } else { win_off });
                        }
                    }
                    top
                }
                2 => {
                    // art-deco ziggurat: tiers stepping in, spire on the crown
                    let mut tx = x;
                    let mut tw = w;
                    let mut bottom = horizon;
                    let mut top = horizon;
                    while tw >= 3 && bottom > 3 {
                        let th = rng.random_range(3..6usize).min(bottom - 1);
                        let t_top = bottom - th;
                        for bx in tx..(tx + tw).min(width) {
                            grid[t_top][bx] = Cell::new('▄', near);
                            for by in t_top + 1..bottom {
                                grid[by][bx] = Cell::new('█', near);
                            }
                        }
                        for by in (t_top + 1..bottom).step_by(2) {
                            for bx in
                                (tx + 1..(tx + tw).min(width).saturating_sub(1)).step_by(2)
                            {
                                let on = rng.random_range(0..100) < lit;
                                grid[by][bx] =
                                    Cell::new('▪', if on { win_on } else { win_off });
                            }
                        }
                        top = t_top;
                        bottom = t_top;
                        tx += 2;
                        tw = tw.saturating_sub(4);
                    }
                    let sx = (x + w / 2).min(width - 1);
                    if top >= 3 {
                        deco!(sx, top - 1, '│', near);
                        deco!(sx, top - 2, '│', near);
                        deco!(sx, top - 3, '✦', lighten(palette[3], 45));
                    }
                    top
                }
                3 => {
                    // banded slab: dark floor stripes, wide lit slots between
                    let h = rng.random_range((height / 3).max(4)..(height * 2 / 3).max(5));
                    let top = horizon.saturating_sub(h);
                    for bx in x..x + w {
                        grid[top][bx] = Cell::new('▄', near);
                        for by in top + 1..horizon {
                            let floor = (by - top) % 3 == 0;
                            let (ch, col) =
                                if floor { ('▄', darken(near, 14)) } else { ('█', near) };
                            grid[by][bx] = Cell::new(ch, col);
                        }
                    }
                    for by in top + 1..horizon.saturating_sub(1) {
                        if (by - top) % 3 != 2 {
                            continue;
                        }
                        for bx in (x + 1..(x + w).saturating_sub(1)).step_by(2) {
                            if rng.random_range(0..100) < lit {
                                grid[by][bx] = Cell::new('▬', win_on);
                            }
                        }
                    }
                    top
                }
                4 => {
                    // needle: thin, very tall, single window column, long mast
                    let h =
                        rng.random_range((height * 3 / 5).max(5)..(height * 5 / 6).max(6));
                    let top = horizon.saturating_sub(h);
                    for bx in x..x + w {
                        grid[top][bx] = Cell::new('▄', near);
                        for by in top + 1..horizon {
                            grid[by][bx] = Cell::new('▓', near);
                        }
                    }
                    let bx = x + w / 2;
                    for by in (top + 2..horizon.saturating_sub(1)).step_by(2) {
                        let on = rng.random_range(0..100) < lit;
                        grid[by][bx] = Cell::new('▪', if on { win_on } else { win_off });
                    }
                    let ah = rng.random_range(3..6usize).min(top);
                    for i in 1..=ah {
                        deco!(bx, top - i, '│', near);
                    }
                    if ah > 0 {
                        deco!(bx, top - ah, '✦', lighten(palette[3], 45));
                    }
                    top
                }
                _ => {
                    // civic dome: squat block, rounded cap, finial
                    let h = rng.random_range((height / 4).max(3)..(height / 2).max(4));
                    let top = horizon.saturating_sub(h);
                    for bx in x..x + w {
                        for by in top..horizon {
                            grid[by][bx] = Cell::new('█', near);
                        }
                    }
                    for by in (top + 1..horizon.saturating_sub(1)).step_by(3) {
                        for bx in (x + 1..(x + w).saturating_sub(1)).step_by(3) {
                            let on = rng.random_range(0..100) < lit;
                            grid[by][bx] = Cell::new('□', if on { win_on } else { win_off });
                        }
                    }
                    if top >= 2 {
                        for bx in x + 1..(x + w).saturating_sub(1) {
                            deco!(bx, top - 1, '▄', lighten(near, 8));
                        }
                        for bx in x + w / 3..(x + w - w / 3).min(width) {
                            deco!(bx, top - 2, '▄', lighten(near, 8));
                        }
                        if top >= 3 {
                            deco!(x + w / 2, top - 3, '+', lighten(palette[3], 35));
                        }
                    }
                    top
                }
            };
            // rooftop water tank on the flat-roofed kinds
            if matches!(kind, 1 | 3) && w >= 7 && btop >= 2 && rng.random_range(0..3) == 0 {
                let wx = x + rng.random_range(1..w - 2);
                deco!(wx, btop - 2, '▄', darken(near, 10));
                deco!(wx + 1, btop - 2, '▄', darken(near, 10));
                deco!(wx, btop - 1, '╥', near);
                deco!(wx + 1, btop - 1, '╥', near);
            }
            if btop < tall_top {
                tall_top = btop;
                tall_x = x + w / 2;
            }
        }

        // searchlight beams off the tallest tower
        if tall_top < horizon && tall_top >= 1 {
            for i in 1..7i32 {
                let y = tall_top as i32 - i;
                if y < 0 {
                    break;
                }
                let f = darken(palette[4], (40 + i * 8).min(90) as u8);
                let lx = tall_x as i32 - i;
                let rx2 = tall_x as i32 + i;
                if lx >= 0 && (lx as usize) < width {
                    deco!(lx as usize, y as usize, '╲', f);
                }
                if rx2 >= 0 && (rx2 as usize) < width {
                    deco!(rx2 as usize, y as usize, '╱', f);
                }
            }
        }

        // parks with a streetlight wherever no near tower landed
        let park = darken(palette[2], 30);
        let mut gx = 0usize;
        while gx < width {
            if street_free[gx] {
                let start = gx;
                while gx < width && street_free[gx] {
                    gx += 1;
                }
                let glen = gx - start;
                if glen >= 4 && horizon >= 3 {
                    for tx in start..gx {
                        match rng.random_range(0..5usize) {
                            0 => grid[horizon - 1][tx] = Cell::new('♣', park),
                            1 => grid[horizon - 1][tx] = Cell::new('♠', darken(park, 12)),
                            2 => grid[horizon - 1][tx] = Cell::new('·', darken(park, 20)),
                            _ => {}
                        }
                    }
                    let lx = start + glen / 2;
                    grid[horizon - 1][lx] = Cell::new('│', darken(palette[4], 50));
                    grid[horizon - 2][lx] = Cell::new('✶', lighten(palette[3], 40));
                }
            } else {
                gx += 1;
            }
        }

        for gx in 0..width {
            if horizon < height {
                grid[horizon][gx] = Cell::new('─', darken(palette[4], 65));
            }
            if horizon + 1 < height && gx % 3 == 0 {
                grid[horizon + 1][gx] = Cell::new('·', darken(palette[1], 70));
            }
        }

        // foreground hulks: this side of the street, near-black, cropped by
        // the bottom of the frame so they read as closest
        let fg = darken(palette[1], 80);
        let fg_win = lighten(palette[3], 40);
        for _ in 0..rng.random_range(1..3usize) {
            let w = rng.random_range(12..22usize).min(width);
            let x = rng.random_range(0..width.saturating_sub(w).max(1));
            let top = rng.random_range((height * 3 / 5).max(2)..(height * 7 / 8).max(3));
            for bx in x..x + w {
                grid[top][bx] = Cell::new('▄', fg);
                for by in top + 1..height {
                    grid[by][bx] = Cell::new('█', fg);
                }
            }
            for by in (top + 2..height.saturating_sub(1)).step_by(3) {
                for bx in (x + 2..(x + w).saturating_sub(2)).step_by(4) {
                    if rng.random_range(0..100) < lit {
                        grid[by][bx] = Cell::new('▮', fg_win);
                        if bx + 1 + 2 < x + w {
                            grid[by][bx + 1] = Cell::new('▮', fg_win);
                        }
                    }
                }
            }
        }
    (grid, false)
}

/// Dispatch arm for mode(s): hive (moved verbatim from run()).
pub(crate) fn cli_hive(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        // hive [fill] -- a comb hanging from the top edge: hex lattice masked
        // to a noise-warped teardrop, ragged bare-wall rim, honey drips off the
        // tip, bees working the boundary. fill = percent of comb cells with honey.
        let fill: u32 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(45);
        let fill_f = fill.clamp(0, 100) as f32 / 100.0;

        let comb = darken(palette[3], 25);
        let honey = lighten(palette[3], 12);
        let ax = width as f32 * rng.random_range(0.38..0.62);
        let rx = width as f32 * rng.random_range(0.22..0.3);
        let ry = height as f32 * rng.random_range(0.75..0.95);
        let mseed = seed.wrapping_add(77);
        // > 0.12 full hex, > 0.0 bare rim wall, otherwise open air
        let mask = |x: f32, y: f32| -> f32 {
            let yn = (y / height.max(1) as f32).clamp(0.0, 1.0);
            let rx_eff = rx * (1.0 - 0.4 * yn);
            let dx = (x - ax) / rx_eff.max(1.0);
            let dy = y / ry.max(1.0);
            1.0 - (dx * dx + dy * dy).sqrt() + (pp_fbm(x / 9.0, y / 5.0, mseed) - 0.5) * 0.7
        };

        for y in 0..height {
            let a_row = y % 2 == 0;
            for x in 0..width {
                let m = mask(x as f32, y as f32);
                if m <= 0.0 {
                    continue;
                }
                let ph = x % 6;
                let ch = if a_row {
                    match ph {
                        0 => '/',
                        3 => '\\',
                        4 | 5 => '_',
                        _ => ' ',
                    }
                } else {
                    match ph {
                        0 => '\\',
                        1 | 2 => '_',
                        3 => '/',
                        _ => ' ',
                    }
                };
                if ch != ' ' {
                    let wall = if m < 0.12 { darken(comb, 18) } else { comb };
                    grid[y][x] = Cell::new(ch, wall);
                } else if m >= 0.12 {
                    // per-hex content, keyed on the hex so both interior cells agree
                    let r = pp_hash2((x / 6) as i32, y as i32, seed);
                    grid[y][x] = if r < fill_f * 0.7 {
                        Cell::new('▒', honey)
                    } else if r < fill_f {
                        Cell::new('▓', darken(honey, 25))
                    } else if r < fill_f + 0.12 && (ph == 1 || ph == 4) {
                        Cell::new('·', lighten(palette[4], 5))
                    } else {
                        Cell::blank()
                    };
                }
            }
        }

        // honey drips off the underside
        for _ in 0..rng.random_range(3..6usize) {
            let dx = rng.random_range(-(rx * 0.6) as i32..=(rx * 0.6) as i32);
            let x = (ax as i32 + dx).clamp(0, width as i32 - 1) as usize;
            let mut bottom = None;
            for y in (0..height).rev() {
                if mask(x as f32, y as f32) > 0.12 {
                    bottom = Some(y);
                    break;
                }
            }
            if let Some(by) = bottom {
                let len = rng.random_range(1..4usize);
                for i in 1..=len {
                    if by + i < height {
                        grid[by + i][x] = Cell::new('│', darken(honey, 25));
                    }
                }
                if by + len + 1 < height {
                    grid[by + len + 1][x] = Cell::new('∙', honey);
                }
            }
        }

        // bees swarm the rim, trails leading out into open air
        let mut placed = 0usize;
        let mut tries = 0usize;
        let want = (width / 8).max(5);
        while placed < want && tries < 600 {
            tries += 1;
            let bx = rng.random_range(0..width);
            let by = rng.random_range(0..height);
            let m = mask(bx as f32, by as f32);
            if m > -0.35 && m < 0.06 {
                let dir: i32 = if (bx as f32) < ax { -1 } else { 1 };
                for i in 1..4i32 {
                    let tx = bx as i32 + dir * i * 2;
                    let ty = by as i32 - (i % 2);
                    if tx >= 0 && ty >= 0 && (tx as usize) < width && (ty as usize) < height {
                        grid[ty as usize][tx as usize] = Cell::new('·', darken(palette[4], 45));
                    }
                }
                grid[by][bx] = Cell::new('ø', lighten(palette[3], 35));
                placed += 1;
            }
        }
    (grid, false)
}

/// Dispatch arm for mode(s): jelly (moved verbatim from run()).
pub(crate) fn cli_jelly(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        // jelly [count] -- deep-sea drift: translucent bells with swaying
        // tentacles, rising bubbles, light shafts from the surface
        let count: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(4);
        let count = count.clamp(1, 12);

        macro_rules! jput {
            ($x:expr, $y:expr, $ch:expr, $col:expr) => {{
                let sx = $x;
                let sy = $y;
                if sx >= 0 && sy >= 0 && (sx as usize) < width && (sy as usize) < height {
                    grid[sy as usize][sx as usize] = Cell::new($ch, $col);
                }
            }};
        }

        // depth-graded plankton
        for y in 0..height {
            let depth = y as f32 / height.max(1) as f32;
            let wc = lerp_color(darken(palette[1], 65), darken(palette[0], 30), depth);
            for x in 0..width {
                if rng.random_range(0..100) < 3 {
                    grid[y][x] = Cell::new('·', wc);
                }
            }
        }

        // broken light shafts slanting down from the surface
        for _ in 0..3 {
            let mut sx = rng.random_range(0..width as i32);
            let depth = rng.random_range(height / 3..(height * 3 / 4).max(height / 3 + 1));
            for y in 0..depth {
                if y % 3 == 2 {
                    sx += 1;
                }
                if rng.random_range(0..5) < 3 {
                    let ch = if y % 3 == 2 { '╲' } else { '│' };
                    let fade = 55 + (y * 30 / depth.max(1)) as u8;
                    jput!(sx, y as i32, ch, darken(palette[4], fade));
                }
            }
        }

        for _ in 0..(width * height / 90).max(6) {
            let x = rng.random_range(0..width);
            let y = rng.random_range(0..height);
            let ch = ['°', 'º', '∘', '·'][rng.random_range(0..4usize)];
            grid[y][x] = Cell::new(ch, darken(palette[4], rng.random_range(25..60)));
        }

        for _ in 0..count {
            let r: i32 = rng.random_range(2..5);
            let cx = rng.random_range(4..(width as i32 - 4).max(5));
            let cy = rng.random_range(2..(height as i32 * 2 / 3).max(3));
            let hue = shift_hue(lighten(palette[2], 18), rng.random_range(-40.0..40.0));

            for dx in -r..=r {
                let ch = if dx == -r {
                    '▗'
                } else if dx == r {
                    '▖'
                } else {
                    '▄'
                };
                jput!(cx + dx, cy - 1, ch, hue);
            }
            for dx in -r - 1..=r + 1 {
                let ch = if dx == -r - 1 {
                    '▐'
                } else if dx == r + 1 {
                    '▌'
                } else if (dx + r).rem_euclid(3) == 0 {
                    '░'
                } else {
                    '▒'
                };
                jput!(cx + dx, cy, ch, hue);
            }
            jput!(cx, cy, '✦', lighten(hue, 25));

            for dx in (-r..=r).step_by(2) {
                let len: i32 = rng.random_range(3..(height as i32 / 3).max(4));
                let phase = rng.random_range(0.0f32..6.3);
                let sway0 = phase.sin();
                let mut prev_off = 0i32;
                for j in 1..=len {
                    let sway = (j as f32 * 0.55 + phase).sin();
                    let off = ((sway - sway0) * 1.6).round() as i32;
                    let slope = off - prev_off;
                    prev_off = off;
                    let ch = if slope > 0 {
                        ')'
                    } else if slope < 0 {
                        '('
                    } else {
                        '|'
                    };
                    let fade = darken(hue, (j * 90 / len.max(1)).min(80) as u8);
                    jput!(cx + dx + off, cy + j, ch, fade);
                }
            }
        }
    (grid, false)
}

/// Dispatch arm for mode(s): jelly2 (moved verbatim from run()).
pub(crate) fn cli_jelly2(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        // jelly2 [count] -- generative jellies. every jelly rolls a species
        // from independent parts: bell shape (shaded dome, moon jelly, box
        // jelly, tall bulb, sideways swimmer), tentacle style (curtain,
        // ribbon, stingers, frill), and an orientation: the bell shears with
        // tilt and the tails lean against it with drift, so no two hang at
        // the same angle. test bed for the parts-generator the aquarium needs.
        let count: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(5);
        let count = count.clamp(1, 14);

        macro_rules! jput {
            ($x:expr, $y:expr, $ch:expr, $col:expr) => {{
                let sx = $x;
                let sy = $y;
                if sx >= 0 && sy >= 0 && (sx as usize) < width && (sy as usize) < height {
                    grid[sy as usize][sx as usize] = Cell::new($ch, $col);
                }
            }};
        }

        // marine snow, dimmer with depth
        for y in 0..height {
            let depth = y as f32 / height.max(1) as f32;
            let wc = lerp_color(darken(palette[1], 60), darken(palette[0], 30), depth);
            for x in 0..width {
                if rng.random_range(0..100) < 3 {
                    let ch = ['·', '˚', '.'][rng.random_range(0..3usize)];
                    grid[y][x] = Cell::new(ch, wc);
                }
            }
        }

        // faint current ribbons
        for _ in 0..2 {
            let ry0 = rng.random_range((height / 5).max(1)..(height * 4 / 5).max(2)) as i32;
            let ph = rng.random_range(0.0f32..6.3);
            let mut x = 0i32;
            while (x as usize) < width {
                let y = ry0 + ((x as f32 * 0.3 + ph).sin() * 1.4).round() as i32;
                if rng.random_range(0..3) < 2 {
                    jput!(x, y, '~', darken(palette[1], 52));
                }
                x += rng.random_range(1..3);
            }
        }

        for _ in 0..(width * height / 110).max(5) {
            let x = rng.random_range(0..width);
            let y = rng.random_range(0..height);
            let ch = ['°', 'º', '∘', '·'][rng.random_range(0..4usize)];
            grid[y][x] = Cell::new(ch, darken(palette[4], rng.random_range(28..60)));
        }

        for _ in 0..count {
            let r: i32 = rng.random_range(2..6);
            let cx = rng.random_range(4..(width as i32 - 4).max(5));
            let cy = rng.random_range(3..(height as i32 * 2 / 3).max(4));
            let base_hue = shift_hue(lighten(palette[2], 18), rng.random_range(-60.0..60.0));
            let hue = darken(base_hue, (cy * 22 / height.max(1) as i32).max(0) as u8);
            let bell = rng.random_range(0..5usize);
            let tilt = rng.random_range(-1.6f32..1.6);
            let ti = tilt.round() as i32;
            // tails trail against the lean
            let drift = -tilt * rng.random_range(0.25..0.6);
            let tstyle = rng.random_range(0..4usize);

            if bell == 4 {
                // sideways swimmer: bell opens along x, tentacles stream behind
                let dir: i32 = if rng.random_range(0..2) == 0 { 1 } else { -1 };
                jput!(cx, cy - 1, if dir > 0 { '▗' } else { '▖' }, darken(hue, 12));
                jput!(cx, cy + 1, if dir > 0 { '▝' } else { '▘' }, darken(hue, 12));
                jput!(cx + dir, cy - 1, if dir > 0 { '\\' } else { '/' }, hue);
                jput!(cx + dir, cy + 1, if dir > 0 { '/' } else { '\\' }, hue);
                jput!(cx, cy, '▒', hue);
                jput!(cx + dir, cy, '▒', hue);
                jput!(cx + 2 * dir, cy, if dir > 0 { ')' } else { '(' }, hue);
                let len: i32 = rng.random_range(5..(width as i32 / 5).max(6));
                for ty in [cy - 1, cy, cy + 1] {
                    let phase = rng.random_range(0.0f32..6.3);
                    let amp = rng.random_range(0.8f32..1.6);
                    let mut prev = 0i32;
                    for j in 1..=len {
                        let sway =
                            ((j as f32 * 0.5 + phase).sin() - phase.sin()) * amp;
                        let off = sway.round() as i32;
                        let slope = (off - prev) * dir;
                        prev = off;
                        let ch = if slope > 0 {
                            '/'
                        } else if slope < 0 {
                            '\\'
                        } else {
                            '~'
                        };
                        let fade = darken(hue, (j * 80 / len.max(1)).min(80) as u8);
                        jput!(cx - dir * (1 + j), ty + off, ch, fade);
                    }
                }
                continue;
            }

            // upright bells: crown row sheared by tilt, body row on cx
            let crown_x = cx + ti;
            match bell {
                0 => {
                    // shaded dome
                    for dx in -r + 1..=r - 1 {
                        let ch = if dx == -r + 1 {
                            '▗'
                        } else if dx == r - 1 {
                            '▖'
                        } else {
                            '▄'
                        };
                        jput!(crown_x + dx, cy - 1, ch, hue);
                    }
                    for dx in -r..=r {
                        let ch = if dx == -r {
                            '▐'
                        } else if dx == r {
                            '▌'
                        } else if (dx + r).rem_euclid(3) == 0 {
                            '░'
                        } else {
                            '▒'
                        };
                        jput!(cx + dx, cy, ch, hue);
                    }
                    jput!(cx, cy, '✦', lighten(hue, 25));
                }
                1 => {
                    // moon jelly: scalloped crown over a clear bell, gonad rings
                    for dx in -r + 1..=r - 1 {
                        jput!(crown_x + dx, cy - 1, '∩', hue);
                    }
                    jput!(cx - r, cy, '(', hue);
                    jput!(cx + r, cy, ')', hue);
                    jput!(cx - 1, cy, '∘', lighten(hue, 20));
                    jput!(cx + 1, cy, '∘', lighten(hue, 20));
                }
                2 => {
                    // box jelly, angular
                    for dx in -r..=r {
                        let ch = if dx == -r {
                            '┌'
                        } else if dx == r {
                            '┐'
                        } else {
                            '─'
                        };
                        jput!(crown_x + dx, cy - 1, ch, hue);
                    }
                    for dx in -r..=r {
                        let ch = if dx.abs() == r { '│' } else { '▒' };
                        jput!(cx + dx, cy, ch, hue);
                    }
                }
                _ => {
                    // tall bulb, two body rows, shear splits across them
                    let midx = cx + ti / 2;
                    for dx in -r + 1..=r - 1 {
                        jput!(crown_x + dx, cy - 2, '▄', hue);
                    }
                    for dx in -r..=r {
                        let ch = if dx == -r {
                            '▐'
                        } else if dx == r {
                            '▌'
                        } else if (dx + r) % 2 == 0 {
                            '▒'
                        } else {
                            '░'
                        };
                        jput!(midx + dx, cy - 1, ch, hue);
                    }
                    for dx in -r + 1..=r - 1 {
                        let ch = if dx == -r + 1 {
                            '('
                        } else if dx == r - 1 {
                            ')'
                        } else {
                            '░'
                        };
                        jput!(cx + dx, cy, ch, hue);
                    }
                    jput!(midx, cy - 1, '✦', lighten(hue, 25));
                }
            }

            let (step, base_len, amp) = match tstyle {
                0 => (2usize, height as i32 / 4, 1.6f32),
                1 => (3, height as i32 / 3, 2.6),
                2 => (2, height as i32 / 4, 0.0),
                _ => (1, 3, 0.9),
            };
            for (k, dx) in (-r + 1..=r - 1).step_by(step).enumerate() {
                let len: i32 = (base_len + rng.random_range(-2..3)).max(2);
                let phase = rng.random_range(0.0f32..6.3);
                let mut prev = 0i32;
                for j in 1..=len {
                    let sway = ((j as f32 * 0.55 + phase).sin() - phase.sin()) * amp;
                    let off = (sway + drift * j as f32).round() as i32;
                    let slope = off - prev;
                    prev = off;
                    if tstyle == 2 && j % 2 == 0 {
                        continue; // gappy stingers
                    }
                    let ch = match tstyle {
                        2 => ['¦', ':', '·'][(j % 3) as usize],
                        1 => {
                            if slope > 0 {
                                ')'
                            } else if slope < 0 {
                                '('
                            } else {
                                '~'
                            }
                        }
                        3 => {
                            if (k + j as usize) % 2 == 0 {
                                '}'
                            } else {
                                '{'
                            }
                        }
                        _ => {
                            if slope > 0 {
                                ')'
                            } else if slope < 0 {
                                '('
                            } else {
                                '|'
                            }
                        }
                    };
                    let fade = darken(hue, (j * 85 / len.max(1)).min(80) as u8);
                    jput!(cx + dx + off, cy + j, ch, fade);
                }
            }
            // frill species still get a long trailing pair at the bell edges
            if tstyle == 3 {
                for dx in [-r + 1, r - 1] {
                    let len: i32 = rng.random_range(
                        (height as i32 / 4).max(3)..(height as i32 / 2).max(4),
                    );
                    let phase = rng.random_range(0.0f32..6.3);
                    let mut prev = 0i32;
                    for j in 1..=len {
                        let sway = ((j as f32 * 0.5 + phase).sin() - phase.sin()) * 1.8;
                        let off = (sway + drift * j as f32).round() as i32;
                        let slope = off - prev;
                        prev = off;
                        let ch = if slope > 0 {
                            ')'
                        } else if slope < 0 {
                            '('
                        } else {
                            '|'
                        };
                        let fade = darken(hue, (j * 85 / len.max(1)).min(80) as u8);
                        jput!(cx + dx + off, cy + j, ch, fade);
                    }
                }
            }
        }

        // small fry drifting in the back
        for _ in 0..count / 2 + 1 {
            let x = rng.random_range(1..(width as i32 - 1).max(2));
            let y = rng.random_range(1..(height as i32 - 2).max(2));
            let dim = darken(palette[2], 42);
            jput!(x, y, '∩', dim);
            let tail = if rng.random_range(0..2) == 0 { '¦' } else { '\'' };
            jput!(x, y + 1, tail, darken(dim, 15));
        }
    (grid, false)
}

/// Dispatch arm for mode(s): elevator.
pub(crate) fn cli_elevator(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        // elevator [lifts] [speed] [crowd] -- building cross-section, cab banks
        // running seeded service loops with doored dwells and mirrored weights
        let lifts: usize = args
            .get(4)
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| param_f32("LIFTS", 3.0) as usize);
        let speed: f32 = args
            .get(5)
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| param_f32("SPEED", 1.0));
        let crowd: f32 = args
            .get(6)
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| param_f32("CROWD", 1.0));
        draw_elevator(
            &mut grid,
            width,
            height,
            seed,
            &palette,
            &mut rng,
            t_anim,
            lifts,
            speed,
            crowd,
        );
    (grid, false)
}

// --- ferris: a night carnival wheel running a boarding cycle. ---
// The wheel angle is linear in t (a real rotation, not a phase wobble). The
// custom machinery rides on top: each gondola's rider manifest re-rolls from
// (seed, gondola, revolution count) at the instant it passes the loading
// dock, the chase lights step around the rim on a discrete tick, the whole
// rim double-flashes at each full revolution, and the queue/exit pedestrians
// are walk interpolations keyed to exact pass times.

pub(crate) fn draw_ferris(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    rng: &mut StdRng,
    t: f32,
    radius: usize,
    gondolas: usize,
    speed: f32,
) {
    use std::f32::consts::TAU;

    if width < 14 || height < 8 {
        return;
    }
    let speed = speed.clamp(0.05, 3.0);
    let gondolas = gondolas.clamp(4, 14);
    let ground_y = height - 1;
    let max_ry = ((ground_y - 4) / 2).max(2);
    let mut ry = (radius as f32).clamp(2.0, max_ry as f32);
    let mut rx = ry * 2.0;
    let rx_cap = ((width - 12) / 2).max(3) as f32;
    if rx > rx_cap {
        rx = rx_cap;
        ry = rx / 2.0;
    }
    let cx = (width / 2) as f32;
    let cy = ground_y as f32 - 4.0 - ry;

    let bg = darken(palette[0], 16);
    for row in grid.iter_mut() {
        for cell in row.iter_mut() {
            *cell = Cell::new(' ', bg);
        }
    }

    let rim_c = darken(palette[1], 42);
    let spoke_c = darken(palette[1], 55);
    let strut_c = darken(palette[1], 30);
    let ground_c = darken(palette[1], 62);
    let light_dim = darken(palette[4], 30);
    let light_lit = lighten(palette[4], 22);
    let booth_c = lighten(palette[3], 14);
    let wait_c = lighten(palette[3], 12);
    let cab_c = lighten(palette[2], 10);
    let rider_c = lighten(palette[4], 10);
    let star_c = lighten(palette[1], 26);

    let omega = 0.55 * speed;
    let theta = t * omega;
    let hue_jitter = rng.random_range(-10.0..10.0);

    // Night sky: sparse stars above the wheel crown.
    let crown = (cy - ry).max(0.0) as usize;
    for y in 0..crown {
        for x in 1..width - 1 {
            let h = emix(seed ^ 0x5EED ^ (y as u64 * 331).wrapping_add(x as u64 * 7));
            if h % 1000 < 7 {
                pp_put(grid, x as i32, y as i32, '·', star_c);
            } else if h % 1000 < 10 {
                pp_put(grid, x as i32, y as i32, '✦', star_c);
            }
        }
    }

    // Ground, A-frame struts from hub to grade, cross-brace.
    for x in 1..width - 1 {
        pp_put(grid, x as i32, ground_y as i32, '▂', ground_c);
    }
    for side in [-1.0f32, 1.0] {
        let fx = cx + side * rx * 0.55;
        let steps = (ground_y as i32 - cy as i32).max(1);
        let per_step = (fx - cx) / steps as f32;
        for s in 0..=steps {
            let q = s as f32 / steps as f32;
            let px = cx + (fx - cx) * q;
            let py = cy + (ground_y as f32 - cy) * q;
            let ch = pp_stroke(per_step.round() as i32, 1);
            pp_put(grid, px.round() as i32, py.round() as i32, ch, strut_c);
        }
        // foot pads
        pp_put(grid, fx.round() as i32, ground_y as i32, '▆', strut_c);
    }
    let brace_y = (cy + (ground_y as f32 - cy) * 0.55).round() as i32;
    let lx = (cx - rx * 0.55 * 0.55).round() as i32;
    let rxx = (cx + rx * 0.55 * 0.55).round() as i32;
    for x in lx..=rxx {
        pp_put(grid, x, brace_y, '─', strut_c);
    }

    // Spokes (under everything), then the rim, then the chase lights.
    for s in 0..8 {
        let a = theta + s as f32 * (TAU / 8.0);
        for q in [0.2f32, 0.35, 0.5, 0.65, 0.8, 0.93] {
            let px = cx + a.cos() * rx * q;
            let py = cy + a.sin() * ry * q;
            pp_put(grid, px.round() as i32, py.round() as i32, '∙', spoke_c);
        }
    }
    let rim_n = 120;
    for i in 0..rim_n {
        let a = i as f32 / rim_n as f32 * TAU;
        let px = cx + a.cos() * rx;
        let py = cy + a.sin() * ry;
        pp_put(grid, px.round() as i32, py.round() as i32, '·', rim_c);
    }
    // Chase lights: 24 sockets stepping a 3-cell pattern; double-flash on the
    // revolution boundary (discrete tick, no phase wobble).
    let flash = (theta % TAU) < 0.28;
    for i in 0..24 {
        let a = i as f32 / 24.0 * TAU;
        let px = cx + a.cos() * rx;
        let py = cy + a.sin() * ry;
        let lit = flash || (i + (theta * 1.2) as usize) % 3 == 0;
        pp_put(
            grid,
            px.round() as i32,
            py.round() as i32,
            if lit { '•' } else { '·' },
            if lit { light_lit } else { light_dim },
        );
    }

    // Gondolas: pivot on the rim, cabin hangs upright one row below. Rider
    // manifest re-rolls per revolution, timed so the swap lands at the dock.
    let n = gondolas;
    for k in 0..n {
        let ang = theta + k as f32 * (TAU / n as f32);
        let px = cx + ang.cos() * rx;
        let py = cy + ang.sin() * ry;
        let phi = theta - k as f32 * (TAU / n as f32) - std::f32::consts::FRAC_PI_2;
        let rev = (phi / TAU).floor();
        let occupied = emix(seed ^ (k as u64).wrapping_mul(0x9E37).wrapping_add((rev as i64 as u64).wrapping_mul(0xBF58))) % 100 < 72;
        let col = shift_hue(cab_c, hue_jitter + k as f64 * 23.0);
        pp_put(grid, px.round() as i32, py.round() as i32 + 1, '¦', strut_c);
        let cy2 = py.round() as i32 + 2;
        pp_put(grid, px.round() as i32 - 1, cy2, '▐', col);
        pp_put(grid, px.round() as i32, cy2, if occupied { '☻' } else { '·' }, if occupied { rider_c } else { darken(col, 40) });
        pp_put(grid, px.round() as i32 + 1, cy2, '▌', col);
    }

    // Hub on top of the spokes.
    pp_put(grid, cx.round() as i32, cy.round() as i32, '◉', light_lit);

    // Loading dock: platform, ticket booth, waiting queue, walking riders.
    let dock_y = (ground_y - 1) as i32;
    for x in (cx as i32 - 4)..=(cx as i32 + 4) {
        pp_put(grid, x, dock_y, '▄', booth_c);
    }
    // ticket booth, clear of the wheel's widest gondolas
    let bx = cx as i32 + 11;
    if bx + 1 < width as i32 - 1 {
        pp_put(grid, bx - 1, dock_y - 1, '╔', booth_c);
        pp_put(grid, bx, dock_y - 1, '⚑', lighten(palette[4], 18));
        pp_put(grid, bx + 1, dock_y - 1, '╗', booth_c);
        pp_put(grid, bx - 1, dock_y, '▐', booth_c);
        pp_put(grid, bx, dock_y, '☰', lighten(booth_c, 12));
        pp_put(grid, bx + 1, dock_y, '▌', booth_c);
    }
    // queue re-rolls on a slow tenant clock, docked left of the platform
    let pc = (t / 6.0).floor().max(0.0) as u64;
    let n_wait = (emix(seed ^ 0xCAFE ^ pc.wrapping_mul(0x9E37_79B9)) % 4) as usize;
    for j in 0..n_wait {
        pp_put(
            grid,
            cx as i32 - 6 - j as i32,
            dock_y,
            '☻',
            if j == 0 { wait_c } else { darken(wait_c, 22) },
        );
    }
    // board/exit walks keyed to each gondola's exact bottom-pass time
    for k in 0..n {
        let ang0 = k as f32 * (TAU / n as f32);
        let phi = theta - ang0 - std::f32::consts::FRAC_PI_2;
        let rev = (phi / TAU).floor();
        let t_pass = (rev * TAU + ang0 + std::f32::consts::FRAC_PI_2) / omega;
        let since = t - t_pass;
        if since >= 0.0 && since < 0.4 {
            // a rider walks in from the queue head to the bottom cabin
            let q = since / 0.4;
            let x = cx - 5.0 + (cx - (cx - 5.0)) * q;
            pp_put(grid, x.round() as i32, dock_y, '☻', wait_c);
        }
        let since_exit = t - t_pass - 0.5;
        if since_exit >= 0.0 && since_exit < 0.4 {
            // the previous rider strolls off to the right
            let q = since_exit / 0.4;
            let x = cx + 1.0 + 4.0 * q;
            pp_put(grid, x.round() as i32, dock_y, '☻', rider_c);
        }
    }
}

/// Dispatch arm for mode(s): ferris.
pub(crate) fn cli_ferris(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], mut rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool) {
        // ferris [radius] [gondolas] [speed] -- carnival wheel: linear hub
        // rotation, per-revolution rider swaps, chase lights, boarding walks
        let radius: usize = args
            .get(4)
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| param_f32("RADIUS", 8.0) as usize);
        let gondolas: usize = args
            .get(5)
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| param_f32("GONDOLAS", 10.0) as usize);
        let speed: f32 = args
            .get(6)
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| param_f32("SPEED", 1.0));
        draw_ferris(
            &mut grid,
            width,
            height,
            seed,
            &palette,
            &mut rng,
            t_anim,
            radius,
            gondolas,
            speed,
        );
    (grid, false)
}
