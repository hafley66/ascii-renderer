//! The monolithic bole-pattern renderer.
use super::boles::*;
use super::*;
/// Shared bole/bush pattern renderer. 34 style variants.
/// `compact`: true clamps layer counts to keep height <= 3 rows (for tree boles).
///            false renders full size (for standalone bush sprites).
pub fn draw_bole_pattern(
    grid: &mut Grid,
    root_x: i32,
    root_y: i32,
    lw: i32,
    rw: i32,
    color: Color,
    bark: Color,
    dim: Color,
    energy: f32,
    style: usize,
    rng: &mut StdRng,
    compact: bool,
) -> BoleExit {
    match style % 34 {
        // Style 0: Crescent -- connected via │ at inner edge positions
        0 => {
            // Ground row: wide crescent
            set(grid, root_x, root_y, '┴', color);
            set(grid, root_x - 1, root_y, '◟', bark);
            set(grid, root_x + 1, root_y, '◞', bark);
            for dx in 2..=lw {
                set(
                    grid,
                    root_x - dx,
                    root_y,
                    '◠',
                    lighten(bark, ((dx - 2) as u8 * 8).min(40)),
                );
            }
            for dx in 2..=rw {
                set(
                    grid,
                    root_x + dx,
                    root_y,
                    '◠',
                    lighten(bark, ((dx - 2) as u8 * 8).min(40)),
                );
            }
            set(grid, root_x - lw - 1, root_y, '◜', lighten(bark, 30));
            set(grid, root_x + rw + 1, root_y, '◝', lighten(bark, 30));
            // Inner crescent with │ connectors down to ground row
            let ilw = (lw * 2 / 3).max(1);
            let irw = (rw * 2 / 3).max(1);
            set(grid, root_x - 1, root_y - 1, '◟', color);
            set(grid, root_x + 1, root_y - 1, '◞', color);
            for dx in 2..=ilw {
                set(grid, root_x - dx, root_y - 1, '◡', bark);
            }
            for dx in 2..=irw {
                set(grid, root_x + dx, root_y - 1, '◡', bark);
            }
            set(grid, root_x - ilw - 1, root_y - 1, '◜', bark);
            set(grid, root_x + irw + 1, root_y - 1, '◝', bark);
            // Vertical connectors: │ at inner crescent edges link to outer crescent
            set(grid, root_x - ilw - 1, root_y, '│', bark);
            set(grid, root_x + irw + 1, root_y, '│', bark);
            // Horizontal bar connecting crescents at mid-width
            let bar_l = (ilw + lw) / 2;
            let bar_r = (irw + rw) / 2;
            if bar_l > ilw + 1 {
                set(grid, root_x - bar_l, root_y - 1, '─', bark);
            }
            if bar_r > irw + 1 {
                set(grid, root_x + bar_r, root_y - 1, '─', bark);
            }
            BoleExit {
                x: root_x,
                y: root_y - 1,
                left: ilw,
                right: irw,
            }
        }
        // Style 1: Braille cluster -- compact 1-2 row spread
        1 => {
            let energy = energy.clamp(0.2, 1.0);
            let dense = ['⣿', '⣾', '⣷', '⣤', '⣶'];
            let thin = ['⡇', '⢸', '⠿', '⠶', '⠛'];
            let rows = if energy > 0.5 { 2 } else { 1 };
            let mut cy = root_y;

            // Ground row: dense, full width
            let hw = lw.max(rw);
            set(grid, root_x, cy, dense[0], color);
            for dx in 1..=hw {
                let ch = dense[rng.random_range(0..dense.len() as u32) as usize];
                set(
                    grid,
                    root_x - dx,
                    cy,
                    ch,
                    darken(color, ((dx as u8) * 3).min(15)),
                );
                set(
                    grid,
                    root_x + dx,
                    cy,
                    ch,
                    darken(color, ((dx as u8) * 3).min(15)),
                );
            }
            cy -= 1;

            // Optional second row: thin, half width
            if rows > 1 {
                let hw2 = (hw / 2).max(1);
                let row_col = darken(color, 10);
                set(grid, root_x, cy, thin[0], row_col);
                for dx in 1..=hw2 {
                    let ch = thin[rng.random_range(0..thin.len() as u32) as usize];
                    set(
                        grid,
                        root_x - dx,
                        cy,
                        ch,
                        darken(row_col, ((dx as u8) * 4).min(15)),
                    );
                    set(
                        grid,
                        root_x + dx,
                        cy,
                        ch,
                        darken(row_col, ((dx as u8) * 4).min(15)),
                    );
                }
                BoleExit {
                    x: root_x,
                    y: cy,
                    left: hw2,
                    right: hw2,
                }
            } else {
                BoleExit {
                    x: root_x,
                    y: cy + 1,
                    left: hw,
                    right: hw,
                }
            }
        }
        // Style 2: Frame -- energy-scaled nested box frames
        2 => {
            let energy = energy.clamp(0.2, 1.0);
            let hlw = lw.max(2);
            let hrw = rw.max(2);
            let layers = if compact {
                1
            } else {
                ((energy * 3.0).ceil() as i32).clamp(1, 3)
            };
            let mut cy = root_y;

            for layer in 0..layers {
                let shrink = layer as f32 * 0.3;
                let ll = ((hlw as f32) * (1.0 - shrink)).max(1.0) as i32;
                let lr = ((hrw as f32) * (1.0 - shrink)).max(1.0) as i32;
                let layer_dim = darken(bark, (layer as u8) * 12);
                let layer_col = if layer == 0 { bark } else { layer_dim };

                set(grid, root_x - ll, cy, '╚', layer_col);
                set(grid, root_x + lr, cy, '╝', layer_col);
                for dx in (-ll + 1)..lr {
                    let ch = if layer == 0 {
                        ['░', '▒', '░', '·'][rng.random_range(0..4u32) as usize]
                    } else {
                        ['▒', '▓', '█', '▒'][rng.random_range(0..4u32) as usize]
                    };
                    set(grid, root_x + dx, cy, ch, dim);
                }
                set(
                    grid,
                    root_x,
                    cy,
                    '╩',
                    if layer == 0 { color } else { layer_col },
                );

                cy -= 1;
                set(grid, root_x - ll, cy, '╔', layer_col);
                set(grid, root_x + lr, cy, '╗', layer_col);
                for dx in (-ll + 1)..lr {
                    set(grid, root_x + dx, cy, '═', layer_col);
                }
                set(
                    grid,
                    root_x,
                    cy,
                    '╦',
                    if layer == 0 { color } else { layer_col },
                );

                if layer == 0 && energy > 0.6 {
                    set(grid, root_x - ll - 1, root_y, '╱', dim);
                    set(grid, root_x + lr + 1, root_y, '╲', dim);
                }

                cy -= 1;
            }

            let last_shrink = (layers - 1) as f32 * 0.3;
            let exit_l = ((hlw as f32) * (1.0 - last_shrink)).max(1.0) as i32;
            let exit_r = ((hrw as f32) * (1.0 - last_shrink)).max(1.0) as i32;
            BoleExit {
                x: root_x,
                y: cy + 1,
                left: exit_l,
                right: exit_r,
            }
        }
        // Style 3: Diamond -- compact: wide ground row + 1-2 taper rows
        3 => {
            let energy = energy.clamp(0.2, 1.0);
            let max_half_w = lw.max(rw).max(2);
            let mut cy = root_y;

            // Ground row: widest point with diamond endpoints and arrow caps
            set(grid, root_x, cy, '◆', color);
            for dx in 1..=max_half_w {
                let ch = if dx == max_half_w { '◇' } else { '═' };
                let c = lighten(color, ((dx as u8) * 4).min(25));
                set(grid, root_x - dx, cy, ch, c);
                set(grid, root_x + dx, cy, ch, c);
            }
            set(grid, root_x - max_half_w - 1, cy, '◁', dim);
            set(grid, root_x + max_half_w + 1, cy, '▷', dim);
            cy -= 1;

            // 1-2 taper rows contracting upward
            let taper_rows = if energy > 0.5 { 2 } else { 1 };
            for row in 0..taper_rows {
                let hw = ((taper_rows - row) as f32 / (taper_rows + 1) as f32 * max_half_w as f32)
                    .ceil() as i32;
                let row_col = lighten(bark, ((row + 1) as u8 * 8).min(35));
                set(grid, root_x, cy, '│', row_col);
                for dx in 1..=hw {
                    let ch = if dx == hw {
                        '◇'
                    } else if dx % 2 == 0 {
                        '─'
                    } else {
                        '◆'
                    };
                    set(grid, root_x - dx, cy, ch, row_col);
                    set(grid, root_x + dx, cy, ch, row_col);
                }
                cy -= 1;
            }

            let exit_hw = (1.0f32 / (taper_rows + 1) as f32 * max_half_w as f32).ceil() as i32;
            BoleExit {
                x: root_x,
                y: cy + 1,
                left: exit_hw,
                right: exit_hw,
            }
        }
        // Style 4: Chevron -- energy-scaled layered V-shapes with variable center
        4 => {
            let energy = energy.clamp(0.2, 1.0);
            // Number of chevron layers: 1 at low, up to 4 at high
            let layers = if compact {
                1
            } else {
                ((energy * 3.5).ceil() as i32).clamp(1, 4)
            };
            let mut cy = root_y;

            // Ground row: base chevron V
            set(grid, root_x, cy, '┴', color);
            let ll = lw.max(2);
            let rl = rw.max(2);
            for dx in 1..=ll {
                let c = lighten(bark, ((dx as u8) * 5).min(35));
                set(grid, root_x - dx, cy, '╱', c);
            }
            for dx in 1..=rl {
                let c = lighten(bark, ((dx as u8) * 5).min(35));
                set(grid, root_x + dx, cy, '╲', c);
            }
            set(grid, root_x - ll - 1, cy, '─', dim);
            set(grid, root_x + rl + 1, cy, '─', dim);
            cy -= 1;

            // Stacked chevron layers, each narrower
            for layer in 0..layers {
                let shrink = (layer + 1) as f32 * 0.22;
                let cl = ((ll as f32) * (1.0 - shrink)).max(1.0) as i32;
                let cr = ((rl as f32) * (1.0 - shrink)).max(1.0) as i32;
                let lc = if layer == 0 {
                    bark
                } else {
                    lighten(bark, (layer as u8 * 8).min(30))
                };

                // Inverted V (∧ shape)
                let center_ch = match rng.random_range(0..3u32) {
                    0 => '∧',
                    1 => '△',
                    _ => '▵',
                };
                set(grid, root_x, cy, center_ch, color);
                for dx in 1..=cl {
                    set(grid, root_x - dx, cy, '╱', lc);
                }
                for dx in 1..=cr {
                    set(grid, root_x + dx, cy, '╲', lc);
                }
                // Horizontal stubs at tips
                if cl > 1 {
                    set(grid, root_x - cl - 1, cy, '─', lighten(lc, 15));
                }
                if cr > 1 {
                    set(grid, root_x + cr + 1, cy, '─', lighten(lc, 15));
                }

                cy -= 1;

                // Only add V shape between layers if not last
                if layer < layers - 1 {
                    let vcl = ((cl as f32) * 0.7).max(1.0) as i32;
                    let vcr = ((cr as f32) * 0.7).max(1.0) as i32;
                    let vc = match rng.random_range(0..3u32) {
                        0 => '∨',
                        1 => '▽',
                        _ => '▿',
                    };
                    set(grid, root_x, cy, vc, lc);
                    for dx in 1..=vcl {
                        set(grid, root_x - dx, cy, '╲', lighten(lc, 10));
                    }
                    for dx in 1..=vcr {
                        set(grid, root_x + dx, cy, '╱', lighten(lc, 10));
                    }
                    cy -= 1;
                }
            }

            BoleExit::point(root_x, cy + 1)
        }
        // Style 5: Frame2 -- connected/overlapping stacked frames, shared borders
        5 => {
            let energy = energy.clamp(0.2, 1.0);
            // Layer count: 1-3, varies with energy but not always max
            let max_layers = if compact {
                1
            } else {
                ((energy * 3.0).ceil() as i32).clamp(1, 3)
            };
            let layers = if max_layers > 1 {
                rng.random_range(1..(max_layers + 1) as u32) as i32
            } else {
                1
            };
            let mut cy = root_y;
            let mut cur_lw = lw.max(2);
            let mut cur_rw = rw.max(2);

            for layer in 0..layers {
                let layer_col = if layer == 0 { color } else { bark };
                let fill_col = if layer == 0 { bark } else { dim };
                // Variable height per layer: 1-3 interior rows
                let interior_h = if compact {
                    1
                } else if energy > 0.7 {
                    rng.random_range(1..4u32) as i32
                } else if energy > 0.4 {
                    rng.random_range(1..3u32) as i32
                } else {
                    1
                };

                // Bottom border (shared with previous layer's top if not first)
                if layer == 0 {
                    set(grid, root_x - cur_lw, cy, '╚', layer_col);
                    set(grid, root_x + cur_rw, cy, '╝', layer_col);
                    for dx in (-cur_lw + 1)..cur_rw {
                        set(grid, root_x + dx, cy, '═', layer_col);
                    }
                    set(grid, root_x, cy, '╩', color);
                    // Buttress legs at base
                    if energy > 0.5 {
                        set(grid, root_x - cur_lw - 1, cy, '╱', fill_col);
                        set(grid, root_x + cur_rw + 1, cy, '╲', fill_col);
                    }
                }

                // Interior fill rows
                for row in 0..interior_h {
                    cy -= 1;
                    set(grid, root_x - cur_lw, cy, '║', color);
                    set(grid, root_x + cur_rw, cy, '║', color);
                    let fills = if row == 0 {
                        ['░', '▒', '░', '·']
                    } else {
                        ['▒', '▓', '█', '▒']
                    };
                    for dx in (-cur_lw + 1)..cur_rw {
                        let ch = fills[rng.random_range(0..4u32) as usize];
                        set(grid, root_x + dx, cy, ch, layer_col);
                    }
                    set(grid, root_x, cy, '│', color);
                }

                // Top border / shared border with next layer
                cy -= 1;
                if layer < layers - 1 {
                    // Shared border: next layer is narrower, so draw T-junctions
                    let next_lw =
                        ((cur_lw as f32) * (0.55 + rng.random::<f32>() * 0.25)).max(1.0) as i32;
                    let next_rw =
                        ((cur_rw as f32) * (0.55 + rng.random::<f32>() * 0.25)).max(1.0) as i32;
                    // Full width top of current layer
                    set(grid, root_x - cur_lw, cy, '╔', layer_col);
                    set(grid, root_x + cur_rw, cy, '╗', layer_col);
                    for dx in (-cur_lw + 1)..cur_rw {
                        set(grid, root_x + dx, cy, '═', layer_col);
                    }
                    // Overwrite with junction chars where next layer's walls will be
                    set(grid, root_x - next_lw, cy, '╠', layer_col);
                    set(grid, root_x + next_rw, cy, '╣', layer_col);
                    set(grid, root_x, cy, '╬', color);
                    cur_lw = next_lw;
                    cur_rw = next_rw;
                } else {
                    // Final top border
                    set(grid, root_x - cur_lw, cy, '╔', layer_col);
                    set(grid, root_x + cur_rw, cy, '╗', layer_col);
                    for dx in (-cur_lw + 1)..cur_rw {
                        set(grid, root_x + dx, cy, '═', layer_col);
                    }
                    set(grid, root_x, cy, '╦', color);
                }
            }

            BoleExit {
                x: root_x,
                y: cy,
                left: cur_lw,
                right: cur_rw,
            }
        }
        // Style 6: Crescent2 -- turbo crescent with hips, valid box-drawing connections
        6 => {
            let energy = energy.clamp(0.2, 1.0);
            let layers = if compact {
                2
            } else {
                ((energy * 4.0).ceil() as i32).clamp(2, 5)
            };
            let mut cy = root_y;

            // Ground layer: widest crescent with hip flares
            set(grid, root_x, cy, '┴', color);
            for dx in 1..=lw {
                let ch = if dx <= 2 { '═' } else { '◠' };
                set(
                    grid,
                    root_x - dx,
                    cy,
                    ch,
                    lighten(color, ((dx as u8) * 3).min(25)),
                );
            }
            for dx in 1..=rw {
                let ch = if dx <= 2 { '═' } else { '◠' };
                set(
                    grid,
                    root_x + dx,
                    cy,
                    ch,
                    lighten(color, ((dx as u8) * 3).min(25)),
                );
            }
            // Hip flares: curved outward kicks
            set(grid, root_x - lw - 1, cy, '╮', bark);
            set(grid, root_x + rw + 1, cy, '╭', bark);
            if lw > 2 {
                set(grid, root_x - lw - 2, cy, '─', dim);
                set(grid, root_x - lw - 1, cy - 1, '│', bark);
                set(grid, root_x - lw - 1, cy + 1, '╯', dim);
            }
            if rw > 2 {
                set(grid, root_x + rw + 2, cy, '─', dim);
                set(grid, root_x + rw + 1, cy - 1, '│', bark);
                set(grid, root_x + rw + 1, cy + 1, '╰', dim);
            }
            cy -= 1;

            // Stacked crescent arcs, each narrower with random horizontal offsets
            for layer in 1..layers {
                let shrink = layer as f32 * 0.2;
                let ll = ((lw as f32) * (1.0 - shrink)).max(1.0) as i32;
                let lr = ((rw as f32) * (1.0 - shrink)).max(1.0) as i32;
                let offset = rng.random_range(0..3u32) as i32 - 1; // -1, 0, or 1
                let cx = root_x + offset;
                let lc = lighten(bark, (layer as u8 * 6).min(30));

                set(grid, cx, cy, '┴', lc);
                for dx in 1..=ll {
                    let ch = ['◠', '◡', '◟', '◞'][rng.random_range(0..4u32) as usize];
                    set(grid, cx - dx, cy, ch, lighten(lc, ((dx as u8) * 4).min(20)));
                }
                for dx in 1..=lr {
                    let ch = ['◠', '◡', '◟', '◞'][rng.random_range(0..4u32) as usize];
                    set(grid, cx + dx, cy, ch, lighten(lc, ((dx as u8) * 4).min(20)));
                }
                // Connect back to center if offset
                if offset != 0 {
                    set(grid, root_x, cy, '│', color);
                }
                // Nip details at crescent tips
                set(grid, cx - ll - 1, cy, '◜', lighten(lc, 15));
                set(grid, cx + lr + 1, cy, '◝', lighten(lc, 15));
                cy -= 1;
            }

            let last_shrink = (layers - 1) as f32 * 0.2;
            let exit_l = ((lw as f32) * (1.0 - last_shrink)).max(1.0) as i32;
            let exit_r = ((rw as f32) * (1.0 - last_shrink)).max(1.0) as i32;
            BoleExit {
                x: root_x,
                y: cy + 1,
                left: exit_l,
                right: exit_r,
            }
        }
        // Style 7: Braille2 -- thick braille with tapered trunk exit
        7 => {
            let energy = energy.clamp(0.2, 1.0);
            let dense = ['⣿', '⣾', '⣷', '⣶', '⣤'];
            let mid = ['⡇', '⢸', '⠿', '⠶', '⠛'];
            let thin = ['⡀', '⢀', '⠂', '⠈', '⠁'];
            let rows = if compact {
                2
            } else {
                ((energy * 4.0).ceil() as i32 + 1).clamp(2, 5)
            };
            let mut cy = root_y;
            let base_w = lw.max(rw).max(2);

            for row in 0..rows {
                let frac = row as f32 / rows as f32;
                let hw = ((base_w as f32) * (1.0 - frac * 0.5)).max(1.0) as i32;
                let chars = if frac < 0.3 {
                    &dense
                } else if frac < 0.6 {
                    &mid
                } else {
                    &thin
                };
                let rc = if row == 0 {
                    color
                } else {
                    darken(color, (row as u8 * 4).min(15))
                };

                // Asymmetric: left and right can have different widths
                let lhw = hw + rng.random_range(0..2u32) as i32;
                let rhw = hw + rng.random_range(0..2u32) as i32;

                set(grid, root_x, cy, chars[0], rc);
                for dx in 1..=lhw {
                    set(
                        grid,
                        root_x - dx,
                        cy,
                        chars[rng.random_range(0..chars.len() as u32) as usize],
                        darken(rc, ((dx as u8) * 2).min(10)),
                    );
                }
                for dx in 1..=rhw {
                    set(
                        grid,
                        root_x + dx,
                        cy,
                        chars[rng.random_range(0..chars.len() as u32) as usize],
                        darken(rc, ((dx as u8) * 2).min(10)),
                    );
                }
                cy -= 1;
            }

            // Taper: 1-2 rows of transition from thick braille to single │
            let taper_rows = if base_w > 3 { 2 } else { 1 };
            for t in 0..taper_rows {
                let tw = (taper_rows - t).min(2);
                let tc = ['⡇', '⢸', '│'][t as usize % 3];
                set(grid, root_x, cy, tc, bark);
                if tw > 1 {
                    set(grid, root_x - 1, cy, '⡀', dim);
                    set(grid, root_x + 1, cy, '⢀', dim);
                }
                cy -= 1;
            }

            let exit_hw = if base_w > 3 { 1 } else { 0 };
            BoleExit {
                x: root_x,
                y: cy + 1,
                left: exit_hw,
                right: exit_hw,
            }
        }
        // Style 8: Frame3 -- stacked boxes, heaviest at bottom, randomly off-center
        8 => {
            let energy = energy.clamp(0.2, 1.0);
            let boxes = if compact {
                1
            } else {
                ((energy * 3.0).ceil() as i32).clamp(1, 4)
            };
            let mut cy = root_y;
            let mut cur_lw = lw.max(3);
            let mut cur_rw = rw.max(3);
            let mut cx = root_x;

            for b in 0..boxes {
                let bc = if b == 0 {
                    color
                } else {
                    lighten(bark, (b as u8 * 8).min(25))
                };
                let fc = if b == 0 {
                    bark
                } else {
                    lighten(dim, (b as u8 * 5).min(20))
                };
                // Interior height: biggest box at bottom, smaller going up
                let interior = if compact {
                    1
                } else if b == 0 {
                    ((energy * 3.0).ceil() as i32).clamp(1, 3)
                } else {
                    rng.random_range(1..3u32) as i32
                };

                // Bottom edge
                set(grid, cx - cur_lw, cy, '╘', bc);
                set(grid, cx + cur_rw, cy, '╛', bc);
                for dx in (-cur_lw + 1)..cur_rw {
                    set(grid, cx + dx, cy, '═', bc);
                }
                set(grid, cx, cy, '╧', bc);

                // Interior rows
                for row in 0..interior {
                    cy -= 1;
                    set(grid, cx - cur_lw, cy, '│', bc);
                    set(grid, cx + cur_rw, cy, '│', bc);
                    let fills = if row == 0 {
                        ['░', '▒', '░', '·']
                    } else {
                        ['▒', '▓', '▒', '░']
                    };
                    for dx in (-cur_lw + 1)..cur_rw {
                        set(
                            grid,
                            cx + dx,
                            cy,
                            fills[rng.random_range(0..4u32) as usize],
                            fc,
                        );
                    }
                    set(grid, cx, cy, '│', bc);
                }

                // Top edge
                cy -= 1;
                set(grid, cx - cur_lw, cy, '╒', bc);
                set(grid, cx + cur_rw, cy, '╓', bc);
                for dx in (-cur_lw + 1)..cur_rw {
                    set(grid, cx + dx, cy, '═', bc);
                }
                set(grid, cx, cy, '╤', bc);

                // Next box: narrower and randomly offset
                let next_lw = ((cur_lw as f32) * (0.5 + rng.random::<f32>() * 0.3)).max(1.0) as i32;
                let next_rw = ((cur_rw as f32) * (0.5 + rng.random::<f32>() * 0.3)).max(1.0) as i32;
                let drift = rng.random_range(0..3u32) as i32 - 1;
                cx += drift;
                cur_lw = next_lw;
                cur_rw = next_rw;

                // Connector between boxes: vertical line back to root_x
                if b < boxes - 1 {
                    cy -= 1;
                    if cx != root_x {
                        // Draw connector from root_x to cx
                        let dir = if cx > root_x { 1 } else { -1 };
                        set(grid, root_x, cy, if dir > 0 { '╰' } else { '╯' }, bc);
                        for sx in 1..(cx - root_x).abs() {
                            set(grid, root_x + sx * dir, cy, '─', bc);
                        }
                        set(grid, cx, cy, if dir > 0 { '╮' } else { '╭' }, bc);
                        cy -= 1;
                    }
                }
            }

            // Final trunk connector at root_x
            if cx != root_x {
                let dir = if root_x > cx { 1 } else { -1 };
                set(grid, cx, cy, if dir > 0 { '╰' } else { '╯' }, bark);
                for sx in 1..(root_x - cx).abs() {
                    set(grid, cx + sx * dir, cy, '─', bark);
                }
                set(grid, root_x, cy, if dir > 0 { '╮' } else { '╭' }, bark);
                cy -= 1;
            }
            BoleExit {
                x: root_x,
                y: cy + 1,
                left: cur_lw,
                right: cur_rw,
            }
        }
        // Style 9: Diamond2 -- asymmetric diamond with cross-hatching
        9 => {
            let energy = energy.clamp(0.2, 1.0);
            let total_h = if compact {
                3
            } else {
                ((energy * 5.0).ceil() as i32 + 2).clamp(3, 7)
            };
            let mut cy = root_y;
            // Asymmetric: bottom half taller than top
            let bot_h = if compact { 1 } else { (total_h * 2 / 3).max(2) };
            let top_h = if compact { 1 } else { (total_h - bot_h).max(1) };
            let max_lw = lw.max(2);
            let max_rw = rw.max(2);

            // Bottom: expanding upward, left and right sides grow at different rates
            for row in 0..bot_h {
                let frac = (row + 1) as f32 / bot_h as f32;
                let hw_l = (frac * max_lw as f32).ceil() as i32;
                let hw_r = (frac * max_rw as f32).ceil() as i32;
                let rc = lighten(bark, ((bot_h - row) as u8 * 4).min(25));
                set(grid, root_x, cy, if row == 0 { '╨' } else { '│' }, color);
                for dx in 1..=hw_l {
                    let ch = if rng.random_range(0..4u32) == 0 {
                        '╳'
                    } else if dx % 2 == 0 {
                        '─'
                    } else {
                        '◆'
                    };
                    set(grid, root_x - dx, cy, ch, rc);
                }
                for dx in 1..=hw_r {
                    let ch = if rng.random_range(0..4u32) == 0 {
                        '╳'
                    } else if dx % 2 == 0 {
                        '─'
                    } else {
                        '◆'
                    };
                    set(grid, root_x + dx, cy, ch, rc);
                }
                cy -= 1;
            }

            // Widest row: asymmetric
            set(grid, root_x, cy, '◆', color);
            for dx in 1..=max_lw {
                let ch = if dx % 3 == 0 { '╳' } else { '═' };
                set(
                    grid,
                    root_x - dx,
                    cy,
                    ch,
                    lighten(color, ((dx as u8) * 3).min(20)),
                );
            }
            for dx in 1..=max_rw {
                let ch = if dx % 3 == 0 { '╳' } else { '═' };
                set(
                    grid,
                    root_x + dx,
                    cy,
                    ch,
                    lighten(color, ((dx as u8) * 3).min(20)),
                );
            }
            set(grid, root_x - max_lw - 1, cy, '◁', dim);
            set(grid, root_x + max_rw + 1, cy, '▷', dim);
            cy -= 1;

            // Top: contracting, shorter than bottom
            for row in 0..top_h {
                let frac = (top_h - row) as f32 / top_h as f32;
                let hw_l = (frac * max_lw as f32).ceil() as i32;
                let hw_r = (frac * max_rw as f32).ceil() as i32;
                let rc = lighten(bark, ((row + 1) as u8 * 7).min(35));
                set(grid, root_x, cy, '│', rc);
                for dx in 1..=hw_l {
                    let ch = if rng.random_range(0..5u32) == 0 {
                        '╳'
                    } else if dx % 2 == 0 {
                        '─'
                    } else {
                        '◇'
                    };
                    set(grid, root_x - dx, cy, ch, rc);
                }
                for dx in 1..=hw_r {
                    let ch = if rng.random_range(0..5u32) == 0 {
                        '╳'
                    } else if dx % 2 == 0 {
                        '─'
                    } else {
                        '◇'
                    };
                    set(grid, root_x + dx, cy, ch, rc);
                }
                cy -= 1;
            }

            let exit_hw_l = (1.0f32 / top_h as f32 * max_lw as f32).ceil() as i32;
            let exit_hw_r = (1.0f32 / top_h as f32 * max_rw as f32).ceil() as i32;
            BoleExit {
                x: root_x,
                y: cy + 1,
                left: exit_hw_l,
                right: exit_hw_r,
            }
        }
        // Style 10: Chevron2 -- chevron with horizontal sprawl near base
        10 => {
            let energy = energy.clamp(0.2, 1.0);
            let layers = if compact {
                1
            } else {
                ((energy * 3.5).ceil() as i32).clamp(1, 4)
            };
            let mut cy = root_y;
            let ll = lw.max(2);
            let rl = rw.max(2);

            // Ground sprawl: horizontal bars at base
            set(grid, root_x, cy, '┴', color);
            for dx in 1..=ll {
                set(
                    grid,
                    root_x - dx,
                    cy,
                    '═',
                    lighten(bark, ((dx as u8) * 3).min(20)),
                );
            }
            for dx in 1..=rl {
                set(
                    grid,
                    root_x + dx,
                    cy,
                    '═',
                    lighten(bark, ((dx as u8) * 3).min(20)),
                );
            }
            // Extended sprawl wings
            let sprawl_l = ll + rng.random_range(1..4u32) as i32;
            let sprawl_r = rl + rng.random_range(1..4u32) as i32;
            for dx in (ll + 1)..=sprawl_l {
                set(grid, root_x - dx, cy, '─', dim);
            }
            for dx in (rl + 1)..=sprawl_r {
                set(grid, root_x + dx, cy, '─', dim);
            }
            set(grid, root_x - sprawl_l - 1, cy, '╴', lighten(dim, 10));
            set(grid, root_x + sprawl_r + 1, cy, '╶', lighten(dim, 10));
            cy -= 1;

            // Base V with extra width
            set(grid, root_x, cy, '∨', color);
            for dx in 1..=ll {
                set(
                    grid,
                    root_x - dx,
                    cy,
                    '╲',
                    lighten(bark, ((dx as u8) * 4).min(25)),
                );
            }
            for dx in 1..=rl {
                set(
                    grid,
                    root_x + dx,
                    cy,
                    '╱',
                    lighten(bark, ((dx as u8) * 4).min(25)),
                );
            }
            // Horizontal extensions at V tips
            set(grid, root_x - ll - 1, cy, '─', dim);
            set(grid, root_x + rl + 1, cy, '─', dim);
            cy -= 1;

            // Chevron layers, each narrower, less horizontal sprawl
            for layer in 0..layers {
                let shrink = (layer + 1) as f32 * 0.2;
                let cl = ((ll as f32) * (1.0 - shrink)).max(1.0) as i32;
                let cr = ((rl as f32) * (1.0 - shrink)).max(1.0) as i32;
                let lc = if layer == 0 {
                    bark
                } else {
                    lighten(bark, (layer as u8 * 7).min(30))
                };

                // ∧ row
                let center_ch = ['∧', '△', '▵', '⟋'][rng.random_range(0..4u32) as usize];
                set(grid, root_x, cy, center_ch, color);
                for dx in 1..=cl {
                    set(grid, root_x - dx, cy, '╱', lc);
                }
                for dx in 1..=cr {
                    set(grid, root_x + dx, cy, '╲', lc);
                }
                // Sprawl decreases with height
                let h_sprawl = ((layers - layer) as f32 * 0.5).ceil() as i32;
                if h_sprawl > 0 {
                    for s in 1..=h_sprawl {
                        set(grid, root_x - cl - s, cy, '─', lighten(lc, 15));
                        set(grid, root_x + cr + s, cy, '─', lighten(lc, 15));
                    }
                }
                cy -= 1;

                if layer < layers - 1 {
                    // ∨ row between layers
                    let vcl = ((cl as f32) * 0.7).max(1.0) as i32;
                    let vcr = ((cr as f32) * 0.7).max(1.0) as i32;
                    let vc = ['∨', '▽', '▿'][rng.random_range(0..3u32) as usize];
                    set(grid, root_x, cy, vc, lc);
                    for dx in 1..=vcl {
                        set(grid, root_x - dx, cy, '╲', lighten(lc, 10));
                    }
                    for dx in 1..=vcr {
                        set(grid, root_x + dx, cy, '╱', lighten(lc, 10));
                    }
                    cy -= 1;
                }
            }

            BoleExit::point(root_x, cy + 1)
        }
        // Style 11: Frame4 -- same as Frame3 but different corner style
        11 => {
            let energy = energy.clamp(0.2, 1.0);
            let boxes = if compact {
                1
            } else {
                ((energy * 3.0).ceil() as i32).clamp(1, 3)
            };
            let mut cy = root_y;
            let mut cur_lw = lw.max(3);
            let mut cur_rw = rw.max(3);
            let mut cx = root_x;

            for b in 0..boxes {
                let bc = if b == 0 {
                    color
                } else {
                    lighten(bark, (b as u8 * 10).min(30))
                };
                let fc = if b == 0 { bark } else { dim };
                let interior = if compact {
                    1
                } else if b == 0 {
                    ((energy * 2.0).ceil() as i32).clamp(1, 3)
                } else {
                    1
                };

                // Bottom
                set(grid, cx - cur_lw, cy, '└', bc);
                set(grid, cx + cur_rw, cy, '┘', bc);
                for dx in (-cur_lw + 1)..cur_rw {
                    set(grid, cx + dx, cy, '─', bc);
                }
                set(grid, cx, cy, '┴', bc);

                for row in 0..interior {
                    cy -= 1;
                    set(grid, cx - cur_lw, cy, '│', bc);
                    set(grid, cx + cur_rw, cy, '│', bc);
                    let fills = ['·', '∙', '·', ' '];
                    for dx in (-cur_lw + 1)..cur_rw {
                        set(
                            grid,
                            cx + dx,
                            cy,
                            fills[rng.random_range(0..4u32) as usize],
                            fc,
                        );
                    }
                    set(grid, cx, cy, '│', bc);
                }

                cy -= 1;
                set(grid, cx - cur_lw, cy, '┌', bc);
                set(grid, cx + cur_rw, cy, '┐', bc);
                for dx in (-cur_lw + 1)..cur_rw {
                    set(grid, cx + dx, cy, '─', bc);
                }
                set(grid, cx, cy, '┬', bc);

                let next_lw = ((cur_lw as f32) * (0.5 + rng.random::<f32>() * 0.3)).max(1.0) as i32;
                let next_rw = ((cur_rw as f32) * (0.5 + rng.random::<f32>() * 0.3)).max(1.0) as i32;
                let drift = rng.random_range(0..3u32) as i32 - 1;
                cx += drift;
                cur_lw = next_lw;
                cur_rw = next_rw;

                if b < boxes - 1 {
                    cy -= 1;
                    if cx != root_x {
                        let dir = if cx > root_x { 1 } else { -1 };
                        set(grid, root_x, cy, if dir > 0 { '╰' } else { '╯' }, bc);
                        for sx in 1..(cx - root_x).abs() {
                            set(grid, root_x + sx * dir, cy, '─', bc);
                        }
                        set(grid, cx, cy, if dir > 0 { '╮' } else { '╭' }, bc);
                        cy -= 1;
                    }
                }
            }

            if cx != root_x {
                let dir = if root_x > cx { 1 } else { -1 };
                set(grid, cx, cy, if dir > 0 { '╰' } else { '╯' }, bark);
                for sx in 1..(root_x - cx).abs() {
                    set(grid, cx + sx * dir, cy, '─', bark);
                }
                set(grid, root_x, cy, if dir > 0 { '╮' } else { '╭' }, bark);
                cy -= 1;
            }
            BoleExit {
                x: root_x,
                y: cy + 1,
                left: cur_lw,
                right: cur_rw,
            }
        }
        // Style 15: Keel -- short fat asymmetric hull, 2-4 rows max
        15 => {
            let energy = energy.clamp(0.2, 1.0);
            let total_h = if compact {
                2
            } else {
                ((energy * 2.0).ceil() as i32 + 1).clamp(2, 4)
            };
            let mut cy = root_y;
            let bias = if rng.random::<bool>() { 1.4f32 } else { 0.6f32 };
            let max_lw = ((lw as f32) * bias * 1.3).max(3.0) as i32;
            let max_rw = ((rw as f32) * (2.0 - bias) * 1.3).max(3.0) as i32;

            for row in 0..total_h {
                let frac = 1.0 - (row as f32 / total_h as f32);
                let hl = (max_lw as f32 * frac).ceil() as i32;
                let hr = (max_rw as f32 * frac).ceil() as i32;
                let rc = if row == 0 {
                    color
                } else {
                    lighten(bark, ((row as u8) * 6).min(25))
                };

                if row == 0 {
                    set(grid, root_x, cy, '╨', color);
                    set(grid, root_x - 1, cy, '═', color);
                    set(grid, root_x + 1, cy, '═', color);
                    for dx in 2..=hl {
                        let ch = if dx % 3 == 0 { '◆' } else { '═' };
                        set(grid, root_x - dx, cy, ch, rc);
                    }
                    for dx in 2..=hr {
                        let ch = if dx % 3 == 0 { '◇' } else { '═' };
                        set(grid, root_x + dx, cy, ch, rc);
                    }
                    set(grid, root_x - hl - 1, cy, '╘', dim);
                    set(grid, root_x + hr + 1, cy, '╛', dim);
                } else {
                    set(grid, root_x, cy, '│', rc);
                    for dx in 1..=hl {
                        let ch = ['─', '─', '◇', '─', '═'][rng.random_range(0..5u32) as usize];
                        set(grid, root_x - dx, cy, ch, rc);
                    }
                    for dx in 1..=hr {
                        let ch = ['─', '─', '◇', '─', '═'][rng.random_range(0..5u32) as usize];
                        set(grid, root_x + dx, cy, ch, rc);
                    }
                    if hl > 0 {
                        set(grid, root_x - hl, cy, '╲', rc);
                    }
                    if hr > 0 {
                        set(grid, root_x + hr, cy, '╱', rc);
                    }
                }
                cy -= 1;
            }

            let exit_frac = 1.0 - ((total_h - 1) as f32 / total_h as f32);
            let exit_l = (max_lw as f32 * exit_frac).ceil() as i32;
            let exit_r = (max_rw as f32 * exit_frac).ceil() as i32;
            BoleExit {
                x: root_x,
                y: cy + 1,
                left: exit_l,
                right: exit_r,
            }
        }
        // Style 17: Buttress -- bright bold curved grounding legs
        17 => {
            let energy = energy.clamp(0.2, 1.0);
            let mut cy = root_y;

            let left_reach = lw.max(2) + rng.random_range(1..4u32) as i32;
            let right_reach = rw.max(2) + rng.random_range(0..3u32) as i32;

            // Ground anchor: bold and bright
            set(grid, root_x, cy, '╨', color);
            set(grid, root_x - 1, cy, '═', color);
            set(grid, root_x + 1, cy, '═', color);
            if lw > 2 {
                set(grid, root_x - 2, cy, '═', bark);
            }
            if rw > 2 {
                set(grid, root_x + 2, cy, '═', bark);
            }

            // Left leg: curved, BRIGHT
            let mut lx = root_x - 2;
            let mut ly = cy;
            set(grid, lx, ly, '╮', color);
            for step in 0..left_reach {
                if step < left_reach / 3 {
                    lx -= 1;
                    set(grid, lx, ly, '─', color);
                } else if step == left_reach / 3 {
                    lx -= 1;
                    set(grid, lx, ly, '╮', lighten(color, 10));
                    ly += 1;
                    if ly <= root_y + 2 {
                        set(grid, lx, ly, '│', lighten(color, 10));
                    }
                } else {
                    lx -= 1;
                    ly += 1;
                    if ly <= root_y + 2 {
                        set(grid, lx, ly, '╲', lighten(color, 5));
                    }
                }
            }
            if ly <= root_y + 2 {
                set(grid, lx, ly, '╰', lighten(color, 10));
            }

            // Right leg: different curve, BRIGHT
            let mut rx = root_x + 2;
            let mut ry = cy;
            set(grid, rx, ry, '╭', color);
            for step in 0..right_reach {
                if step < right_reach / 2 {
                    rx += 1;
                    set(grid, rx, ry, '─', color);
                } else if step == right_reach / 2 {
                    rx += 1;
                    set(grid, rx, ry, '╭', lighten(color, 10));
                    ry += 1;
                    if ry <= root_y + 2 {
                        set(grid, rx, ry, '│', lighten(color, 10));
                    }
                } else {
                    rx += 1;
                    ry += 1;
                    if ry <= root_y + 2 {
                        set(grid, rx, ry, '╱', lighten(color, 5));
                    }
                }
            }
            if ry <= root_y + 2 {
                set(grid, rx, ry, '╯', lighten(color, 10));
            }

            // Cross-brace at high energy: BRIGHT
            if !compact && energy > 0.5 {
                let brace_y = cy - 1;
                let bl = (left_reach / 3).max(1);
                let br = (right_reach / 3).max(1);
                for dx in 1..=bl {
                    set(grid, root_x - dx, brace_y, '─', color);
                }
                set(grid, root_x - bl - 1, brace_y, '╴', bark);
                for dx in 1..=br {
                    set(grid, root_x + dx, brace_y, '─', color);
                }
                set(grid, root_x + br + 1, brace_y, '╶', bark);
                set(grid, root_x, brace_y, '┼', color);
                cy = brace_y;
            }

            // Upper secondary hints
            if !compact && energy > 0.4 {
                cy -= 1;
                let sl = (left_reach / 3).max(1);
                let sr = (right_reach / 3).max(1);
                for dx in 1..=sl {
                    set(grid, root_x - dx, cy, '╱', lighten(color, 15));
                }
                for dx in 1..=sr {
                    set(grid, root_x + dx, cy, '╲', lighten(color, 15));
                }
                set(grid, root_x, cy, '│', color);
            }

            cy -= 1;
            let sl = (left_reach / 3).max(1);
            let sr = (right_reach / 3).max(1);
            BoleExit {
                x: root_x,
                y: cy + 1,
                left: sl,
                right: sr,
            }
        }
        12 => BoleExit::point(root_x, root_y),
        // Style 13: Braille -- horizontal shelf bole, wide ground then sharp drop
        13 => {
            let energy = energy.clamp(0.2, 1.0);
            let dense = ['⣿', '⣾', '⣷', '⣶', '⣤'];
            let mid = ['⡇', '⢸', '⠿', '⠶', '⠛'];
            let edge = ['⡀', '⢀', '⠂', '⠁', '⠈'];

            // Shelf structure: 1-2 wide ground rows, then sharp narrow
            let shelf_rows = if compact {
                1
            } else if energy > 0.5 {
                2
            } else {
                1
            };
            let upper_rows = if compact {
                1
            } else {
                ((energy * 2.0).ceil() as i32).clamp(0, 3)
            };
            let base_l = lw.max(3) + rng.random_range(0..3u32) as i32;
            let base_r = rw.max(3) + rng.random_range(0..2u32) as i32;
            let mut cy = root_y;

            // SHELF: wide dense ground rows (the horizontal emphasis)
            for row in 0..shelf_rows {
                let sl = base_l - row;
                let sr = base_r - row;
                let rc = if row == 0 { color } else { darken(color, 4) };
                set(grid, root_x, cy, '⣿', rc);
                for dx in 1..=sl {
                    let ch = dense[rng.random_range(0..dense.len() as u32) as usize];
                    set(
                        grid,
                        root_x - dx,
                        cy,
                        ch,
                        darken(rc, ((dx as u8) * 2).min(8)),
                    );
                }
                for dx in 1..=sr {
                    let ch = dense[rng.random_range(0..dense.len() as u32) as usize];
                    set(
                        grid,
                        root_x + dx,
                        cy,
                        ch,
                        darken(rc, ((dx as u8) * 2).min(8)),
                    );
                }
                set(
                    grid,
                    root_x - sl - 1,
                    cy,
                    edge[rng.random_range(0..edge.len() as u32) as usize],
                    dim,
                );
                set(
                    grid,
                    root_x + sr + 1,
                    cy,
                    edge[rng.random_range(0..edge.len() as u32) as usize],
                    dim,
                );
                cy -= 1;
            }

            // SHARP DROP: immediately much narrower (40-60% of shelf width)
            let drop_frac = 0.4 + rng.random::<f32>() * 0.2;
            let mut cur_l = (base_l as f32 * drop_frac).max(1.0) as i32;
            let mut cur_r = (base_r as f32 * drop_frac).max(1.0) as i32;

            for row in 0..upper_rows {
                let rc = darken(color, ((shelf_rows + row) as u8 * 3).min(12));
                let chars = if row == 0 { &mid } else { &mid };
                set(
                    grid,
                    root_x,
                    cy,
                    dense[rng.random_range(0..2u32) as usize],
                    rc,
                );
                for dx in 1..=cur_l {
                    set(
                        grid,
                        root_x - dx,
                        cy,
                        chars[rng.random_range(0..chars.len() as u32) as usize],
                        darken(rc, ((dx as u8) * 2).min(8)),
                    );
                }
                for dx in 1..=cur_r {
                    set(
                        grid,
                        root_x + dx,
                        cy,
                        chars[rng.random_range(0..chars.len() as u32) as usize],
                        darken(rc, ((dx as u8) * 2).min(8)),
                    );
                }
                set(
                    grid,
                    root_x - cur_l - 1,
                    cy,
                    edge[rng.random_range(0..edge.len() as u32) as usize],
                    dim,
                );
                set(
                    grid,
                    root_x + cur_r + 1,
                    cy,
                    edge[rng.random_range(0..edge.len() as u32) as usize],
                    dim,
                );
                // Taper each upper row slightly
                cur_l = (cur_l - rng.random_range(0..2u32) as i32).max(1);
                cur_r = (cur_r - rng.random_range(0..2u32) as i32).max(1);
                cy -= 1;
            }

            BoleExit {
                x: root_x,
                y: cy,
                left: cur_l,
                right: cur_r,
            }
        }
        // Style 14: Frame -- overlapping rects with foreground cross glyphs
        14 => {
            let energy = energy.clamp(0.2, 1.0);
            let rects = if compact {
                1
            } else {
                ((energy * 2.0).ceil() as i32).clamp(1, 3)
            };
            let mut cy = root_y;

            let mut specs: Vec<(i32, i32, i32, i32)> = Vec::new();
            for r in 0..rects {
                let drift = if r == 0 {
                    0
                } else {
                    rng.random_range(0..3u32) as i32 - 1
                };
                let rlw = if r == 0 {
                    lw.max(3)
                } else {
                    (lw as f32 * (0.4 + rng.random::<f32>() * 0.4)).max(2.0) as i32
                };
                let rrw = if r == 0 {
                    rw.max(3)
                } else {
                    (rw as f32 * (0.4 + rng.random::<f32>() * 0.4)).max(2.0) as i32
                };
                let ih = if compact {
                    1
                } else if r == 0 {
                    ((energy * 2.0).ceil() as i32).clamp(1, 2)
                } else {
                    1
                };
                specs.push((drift, rlw, rrw, ih));
            }

            let mut accumulated_drift = 0i32;
            let mut prev_top_y: Option<i32> = None;
            let mut last_lw = lw;
            let mut last_rw = rw;

            for (ri, &(drift, rlw, rrw, ih)) in specs.iter().enumerate() {
                last_lw = rlw;
                last_rw = rrw;
                accumulated_drift += drift;
                let cx = root_x + accumulated_drift;
                let heavy = ri == 0;
                let bc = if heavy {
                    color
                } else {
                    lighten(bark, (ri as u8 * 10).min(25))
                };
                let fc = if heavy {
                    bark
                } else {
                    lighten(dim, (ri as u8 * 6).min(20))
                };

                let on_shared_edge = prev_top_y.map_or(false, |py| py == cy);

                if on_shared_edge {
                    set(grid, cx - rlw, cy, '╬', bc);
                    set(grid, cx + rrw, cy, '╬', bc);
                    for dx in (-rlw + 1)..rrw {
                        set(grid, cx + dx, cy, '╪', bc);
                    }
                    set(grid, root_x, cy, '╬', color);
                } else if heavy {
                    set(grid, cx - rlw, cy, '╚', bc);
                    set(grid, cx + rrw, cy, '╝', bc);
                    for dx in (-rlw + 1)..rrw {
                        set(grid, cx + dx, cy, '═', bc);
                    }
                    set(grid, root_x, cy, '╩', color);
                } else {
                    set(grid, cx - rlw, cy, '└', bc);
                    set(grid, cx + rrw, cy, '┘', bc);
                    for dx in (-rlw + 1)..rrw {
                        set(grid, cx + dx, cy, '─', bc);
                    }
                    set(grid, root_x, cy, '┴', bc);
                }

                for row in 0..ih {
                    cy -= 1;
                    set(grid, cx - rlw, cy, if heavy { '║' } else { '│' }, bc);
                    set(grid, cx + rrw, cy, if heavy { '║' } else { '│' }, bc);
                    let fills = if row == 0 {
                        ['░', '▒', '·', '░']
                    } else {
                        ['▒', '▓', '░', '·']
                    };
                    for dx in (-rlw + 1)..rrw {
                        set(
                            grid,
                            cx + dx,
                            cy,
                            fills[rng.random_range(0..4u32) as usize],
                            fc,
                        );
                    }
                    set(grid, root_x, cy, if heavy { '║' } else { '│' }, bc);
                }

                cy -= 1;
                if heavy {
                    set(grid, cx - rlw, cy, '╔', bc);
                    set(grid, cx + rrw, cy, '╗', bc);
                    for dx in (-rlw + 1)..rrw {
                        set(grid, cx + dx, cy, '═', bc);
                    }
                    set(grid, root_x, cy, '╦', color);
                } else {
                    set(grid, cx - rlw, cy, '┌', bc);
                    set(grid, cx + rrw, cy, '┐', bc);
                    for dx in (-rlw + 1)..rrw {
                        set(grid, cx + dx, cy, '─', bc);
                    }
                    set(grid, root_x, cy, '┬', bc);
                }

                prev_top_y = Some(cy);
            }

            BoleExit {
                x: root_x,
                y: cy,
                left: last_lw,
                right: last_rw,
            }
        }
        // Style 16: Chevron -- off-center layers that overlap into diamond patterns
        16 => {
            let energy = energy.clamp(0.2, 1.0);
            let layers = if compact {
                1
            } else {
                ((energy * 4.0).ceil() as i32).clamp(2, 5)
            };
            let mut cy = root_y;
            let ll = lw.max(2);
            let rl = rw.max(2);

            // Ground row is also the first ∧ -- no gap between base and chevrons
            set(grid, root_x, cy, '∧', color);
            for dx in 1..=ll {
                set(
                    grid,
                    root_x - dx,
                    cy,
                    '╱',
                    lighten(bark, ((dx as u8) * 2).min(20)),
                );
            }
            for dx in 1..=rl {
                set(
                    grid,
                    root_x + dx,
                    cy,
                    '╲',
                    lighten(bark, ((dx as u8) * 2).min(20)),
                );
            }
            // Sprawl wings extending from chevron tips
            let sprawl_l = rng.random_range(1..4u32) as i32;
            let sprawl_r = rng.random_range(1..3u32) as i32;
            for s in 1..=sprawl_l {
                set(grid, root_x - ll - s, cy, '─', lighten(bark, 12));
            }
            for s in 1..=sprawl_r {
                set(grid, root_x + rl + s, cy, '─', lighten(bark, 12));
            }
            set(grid, root_x - ll - sprawl_l - 1, cy, '╴', dim);
            set(grid, root_x + rl + sprawl_r + 1, cy, '╶', dim);
            cy -= 1;

            // Remaining chevron layers with random drift -- overlaps create diamonds
            for layer in 0..layers {
                let shrink = (layer + 1) as f32 * 0.15;
                let cl = ((ll as f32) * (1.0 - shrink)).max(1.0) as i32;
                let cr = ((rl as f32) * (1.0 - shrink)).max(1.0) as i32;
                let lc = if layer == 0 {
                    bark
                } else {
                    lighten(bark, (layer as u8 * 6).min(30))
                };
                // Random drift: each layer can be off-center
                let drift = rng.random_range(0..3u32) as i32 - 1;
                let lcx = root_x + drift;

                // ∧ row (upward V)
                set(grid, lcx, cy, '∧', color);
                for dx in 1..=cl {
                    set(grid, lcx - dx, cy, '╱', lc);
                }
                for dx in 1..=cr {
                    set(grid, lcx + dx, cy, '╲', lc);
                }
                // Horizontal stubs, sprawl decreases with height
                let h_sprawl = ((layers - layer) as f32 * 0.7).ceil() as i32;
                for s in 1..=h_sprawl {
                    set(grid, lcx - cl - s, cy, '─', lighten(lc, 12));
                    set(grid, lcx + cr + s, cy, '─', lighten(lc, 12));
                }
                cy -= 1;

                // ∨ row (downward V) -- slightly different drift for overlap
                if layer < layers - 1 {
                    let drift2 = rng.random_range(0..3u32) as i32 - 1;
                    let vcx = root_x + drift2;
                    let vcl = ((cl as f32) * 0.75).max(1.0) as i32;
                    let vcr = ((cr as f32) * 0.75).max(1.0) as i32;
                    set(grid, vcx, cy, '∨', lighten(lc, 5));
                    for dx in 1..=vcl {
                        set(grid, vcx - dx, cy, '╲', lighten(lc, 8));
                    }
                    for dx in 1..=vcr {
                        set(grid, vcx + dx, cy, '╱', lighten(lc, 8));
                    }
                    cy -= 1;
                }
            }

            // Chevron's ∧ shape already tapers to a point -- no generic taper needed
            BoleExit::point(root_x, cy + 1)
        }
        // ── Squat boles: max 2 rows, horizontal emphasis, single-column flares ──

        // Squat Crescent: wide single-row arc with flare pokes
        18 => {
            set(grid, root_x, root_y, '┴', color);
            for dx in 1..=lw {
                set(
                    grid,
                    root_x - dx,
                    root_y,
                    '═',
                    lighten(bark, ((dx as u8) * 3).min(20)),
                );
            }
            for dx in 1..=rw {
                set(
                    grid,
                    root_x + dx,
                    root_y,
                    '═',
                    lighten(bark, ((dx as u8) * 3).min(20)),
                );
            }
            set(grid, root_x - lw - 1, root_y, '◜', dim);
            set(grid, root_x + rw + 1, root_y, '◝', dim);
            // Flares: single-column pokes above at random positions
            for _ in 0..rng.random_range(1..4u32) {
                let fx = root_x + rng.random_range(0..(lw + rw + 1) as u32) as i32 - lw;
                if fx != root_x {
                    set(grid, fx, root_y - 1, '╷', lighten(bark, 15));
                }
            }
            BoleExit::point(root_x, root_y)
        }
        // Squat Braille: 2-row dense shelf, no vertical growth
        19 => {
            let dense = ['⣿', '⣾', '⣷', '⣶', '⣤'];
            let edge = ['⡀', '⢀', '⠂', '⠁', '⠈'];
            // Row 1: dense ground
            set(grid, root_x, root_y, '⣿', color);
            for dx in 1..=lw {
                set(
                    grid,
                    root_x - dx,
                    root_y,
                    dense[rng.random_range(0..dense.len() as u32) as usize],
                    darken(color, ((dx as u8) * 2).min(10)),
                );
            }
            for dx in 1..=rw {
                set(
                    grid,
                    root_x + dx,
                    root_y,
                    dense[rng.random_range(0..dense.len() as u32) as usize],
                    darken(color, ((dx as u8) * 2).min(10)),
                );
            }
            set(
                grid,
                root_x - lw - 1,
                root_y,
                edge[rng.random_range(0..edge.len() as u32) as usize],
                dim,
            );
            set(
                grid,
                root_x + rw + 1,
                root_y,
                edge[rng.random_range(0..edge.len() as u32) as usize],
                dim,
            );
            // Row 2: sparser, narrower
            let sl = (lw as f32 * 0.5).max(1.0) as i32;
            let sr = (rw as f32 * 0.5).max(1.0) as i32;
            let mid = ['⡇', '⢸', '⠿', '⠶', '⠛'];
            set(
                grid,
                root_x,
                root_y - 1,
                mid[rng.random_range(0..mid.len() as u32) as usize],
                bark,
            );
            for dx in 1..=sl {
                set(
                    grid,
                    root_x - dx,
                    root_y - 1,
                    mid[rng.random_range(0..mid.len() as u32) as usize],
                    darken(bark, ((dx as u8) * 3).min(12)),
                );
            }
            for dx in 1..=sr {
                set(
                    grid,
                    root_x + dx,
                    root_y - 1,
                    mid[rng.random_range(0..mid.len() as u32) as usize],
                    darken(bark, ((dx as u8) * 3).min(12)),
                );
            }
            BoleExit {
                x: root_x,
                y: root_y - 1,
                left: sl,
                right: sr,
            }
        }
        // Squat Frame: 2-row nested frame with diamond accents and pillar legs
        20 => {
            // Row 1 (ground): outer frame base with diamond endpoints
            set(grid, root_x - lw, root_y, '◇', dim);
            set(grid, root_x + rw, root_y, '◇', dim);
            for dx in (-lw + 1)..rw {
                let ch = if (root_x + dx) % 2 == 0 { '═' } else { '─' };
                set(grid, root_x + dx, root_y, ch, bark);
            }
            set(grid, root_x, root_y, '╧', color);
            // Inner accent: ◆ markers at 1/3 and 2/3 across
            let third_l = lw / 3;
            let third_r = rw / 3;
            if third_l > 0 {
                set(grid, root_x - third_l, root_y, '◆', lighten(bark, 10));
            }
            if third_r > 0 {
                set(grid, root_x + third_r, root_y, '◆', lighten(bark, 10));
            }

            // Row 2 (above): narrower inner shelf with box corners
            let iw_l = (lw * 2 / 3).max(1);
            let iw_r = (rw * 2 / 3).max(1);
            set(grid, root_x - iw_l, root_y - 1, '╰', lighten(bark, 8));
            set(grid, root_x + iw_r, root_y - 1, '╯', lighten(bark, 8));
            for dx in (-iw_l + 1)..iw_r {
                set(grid, root_x + dx, root_y - 1, '─', lighten(bark, 12));
            }
            set(grid, root_x, root_y - 1, '┼', color);
            // Pillar legs: drop below at diamond endpoints
            set(grid, root_x - lw, root_y + 1, '│', dim);
            set(grid, root_x + rw, root_y + 1, '│', dim);
            // Random inner flares above inner shelf
            if rng.random_range(0..2u32) == 0 {
                let fx = root_x + rng.random_range(1..iw_r.max(2) as u32) as i32;
                set(grid, fx, root_y - 2, '╷', lighten(bark, 20));
            }
            if rng.random_range(0..2u32) == 0 {
                let fx = root_x - rng.random_range(1..iw_l.max(2) as u32) as i32;
                set(grid, fx, root_y - 2, '╷', lighten(bark, 20));
            }
            BoleExit {
                x: root_x,
                y: root_y - 1,
                left: iw_l,
                right: iw_r,
            }
        }
        // Squat Diamond: 2-row flat diamond, single chevron + base
        21 => {
            // Ground: wide base
            set(grid, root_x, root_y, '╨', color);
            for dx in 1..=lw {
                let ch = if dx == lw { '◇' } else { '═' };
                set(
                    grid,
                    root_x - dx,
                    root_y,
                    ch,
                    lighten(bark, ((dx as u8) * 3).min(20)),
                );
            }
            for dx in 1..=rw {
                let ch = if dx == rw { '◇' } else { '═' };
                set(
                    grid,
                    root_x + dx,
                    root_y,
                    ch,
                    lighten(bark, ((dx as u8) * 3).min(20)),
                );
            }
            // Row 2: single V narrowing
            let hw = (lw.max(rw) / 2).max(1);
            set(grid, root_x, root_y - 1, '│', color);
            for dx in 1..=hw {
                set(grid, root_x - dx, root_y - 1, '╱', bark);
                set(grid, root_x + dx, root_y - 1, '╲', bark);
            }
            // Tip flares at base ends
            if lw > 2 {
                set(grid, root_x - lw, root_y + 1, '╵', dim);
            }
            if rw > 2 {
                set(grid, root_x + rw, root_y + 1, '╵', dim);
            }
            BoleExit {
                x: root_x,
                y: root_y - 1,
                left: hw,
                right: hw,
            }
        }
        // Squat Chevron: 2-row diamond chevron with inverted V counter-layer
        22 => {
            // Row 1 (ground): wide V with diamond at apex
            let center = ['∧', '△', '▵'][rng.random_range(0..3u32) as usize];
            set(grid, root_x, root_y, center, color);
            for dx in 1..=lw {
                let c = lighten(bark, ((dx as u8) * 4).min(25));
                set(grid, root_x - dx, root_y, '╱', c);
            }
            for dx in 1..=rw {
                let c = lighten(bark, ((dx as u8) * 4).min(25));
                set(grid, root_x + dx, root_y, '╲', c);
            }
            // Sprawl arms with stubs
            let sl = rng.random_range(1..4u32) as i32;
            let sr = rng.random_range(1..4u32) as i32;
            for s in 1..=sl {
                set(grid, root_x - lw - s, root_y, '─', lighten(bark, 12));
            }
            for s in 1..=sr {
                set(grid, root_x + rw + s, root_y, '─', lighten(bark, 12));
            }
            set(grid, root_x - lw - sl - 1, root_y, '◁', dim);
            set(grid, root_x + rw + sr + 1, root_y, '▷', dim);

            // Row 2 (above): inverted mini-V counter-layer (creates diamond negative space)
            let hw = (lw.max(rw) * 2 / 3).max(1);
            let inv = ['∨', '▽', '▿'][rng.random_range(0..3u32) as usize];
            set(grid, root_x, root_y - 1, inv, lighten(bark, 10));
            for dx in 1..=hw {
                set(grid, root_x - dx, root_y - 1, '╲', lighten(bark, 15));
                set(grid, root_x + dx, root_y - 1, '╱', lighten(bark, 15));
            }
            // Horizontal stubs at inverted tips
            if hw > 1 {
                set(grid, root_x - hw - 1, root_y - 1, '─', dim);
                set(grid, root_x + hw + 1, root_y - 1, '─', dim);
            }

            // Anchor drops below sprawl endpoints
            set(grid, root_x - lw - sl, root_y + 1, '╵', dim);
            set(grid, root_x + rw + sr, root_y + 1, '╵', dim);

            BoleExit {
                x: root_x,
                y: root_y - 1,
                left: hw,
                right: hw,
            }
        }
        // Squat Buttress: ground anchor with curved legs, max 2 rows
        23 => {
            set(grid, root_x, root_y, '╨', color);
            set(grid, root_x - 1, root_y, '═', color);
            set(grid, root_x + 1, root_y, '═', color);
            // Left leg: horizontal then down-kick
            let ll_reach = lw.max(2);
            set(grid, root_x - 2, root_y, '╮', bark);
            for dx in 3..=ll_reach {
                set(
                    grid,
                    root_x - dx,
                    root_y,
                    '─',
                    lighten(bark, ((dx as u8) * 3).min(20)),
                );
            }
            set(grid, root_x - ll_reach - 1, root_y, '╴', dim);
            // Left leg flare down
            set(grid, root_x - 2, root_y + 1, '│', dim);
            set(grid, root_x - 2, root_y + 2, '╵', lighten(dim, 10));
            // Right leg
            let rr_reach = rw.max(2);
            set(grid, root_x + 2, root_y, '╭', bark);
            for dx in 3..=rr_reach {
                set(
                    grid,
                    root_x + dx,
                    root_y,
                    '─',
                    lighten(bark, ((dx as u8) * 3).min(20)),
                );
            }
            set(grid, root_x + rr_reach + 1, root_y, '╶', dim);
            // Right leg flare down
            set(grid, root_x + 2, root_y + 1, '│', dim);
            set(grid, root_x + 2, root_y + 2, '╵', lighten(dim, 10));
            BoleExit::point(root_x, root_y)
        }
        // ── Winding boles: serpentine runs, woven strands, coiled arcs ──

        // Serpent: a root snakes across the ground in S-curves, switching
        // rows with curve corners; the trunk rises wherever it crosses center
        24 => {
            let y_b = root_y;
            let y_t = root_y - 1;
            let start = root_x - lw.max(3) - 1;
            let end = root_x + rw.max(3) + 1;
            let mut on_top = rng.random_range(0..2u32) == 0;
            let mut run = rng.random_range(2..5u32) as i32;
            let mut row_at_root = false; // true if snake is on top row at root_x
            set(grid, start, if on_top { y_t } else { y_b }, '╶', dim);
            let mut x = start + 1;
            while x <= end {
                let dist = ((x - root_x).abs() as u8 * 2).min(20);
                let c = if (x - root_x).abs() <= 1 {
                    color
                } else {
                    darken(bark, dist)
                };
                if run == 0 && x < end - 1 {
                    // switch rows: corner pair links the two runs
                    if on_top {
                        set(grid, x, y_t, '╮', c);
                        set(grid, x, y_b, '╰', c);
                    } else {
                        set(grid, x, y_b, '╯', c);
                        set(grid, x, y_t, '╭', c);
                    }
                    on_top = !on_top;
                    run = rng.random_range(2..5u32) as i32;
                } else {
                    set(grid, x, if on_top { y_t } else { y_b }, '─', c);
                    run -= 1;
                }
                if x == root_x {
                    row_at_root = on_top;
                }
                x += 1;
            }
            set(grid, end + 1, if on_top { y_t } else { y_b }, '╴', dim);
            // root tips digging below ground at 1-2 spots
            for _ in 0..rng.random_range(1..3u32) {
                let fx = root_x + rng.random_range(0..(lw + rw).max(2) as u32) as i32 - lw;
                set(grid, fx, root_y + 1, '╷', dim);
            }
            // trunk junction on whichever row the snake occupies at center
            if row_at_root {
                set(grid, root_x, y_t, '┴', color);
                BoleExit::point(root_x, y_t - 1)
            } else {
                set(grid, root_x, y_b, '┴', color);
                BoleExit::point(root_x, y_t)
            }
        }
        // Braid: two strands weave over-under in a period-4 diamond chain
        25 => {
            let w = lw.max(rw).max(3);
            let y_b = root_y;
            let y_t = root_y - 1;
            for k in -w..=w {
                let x = root_x + k;
                let dist = (k.abs() as u8 * 3).min(20);
                let (ct, cb) = match k.rem_euclid(4) {
                    0 => ('─', '─'),
                    1 => ('╲', '╱'),
                    2 => ('╱', '╲'),
                    _ => ('─', '─'),
                };
                // crossing columns stay bright, straight runs fade outward
                let crossing = k.rem_euclid(4) == 1 || k.rem_euclid(4) == 2;
                let c = if crossing {
                    lighten(bark, 10)
                } else {
                    darken(bark, dist)
                };
                set(grid, x, y_t, ct, c);
                set(grid, x, y_b, cb, darken(c, 8));
            }
            set(grid, root_x - w - 1, y_b, '╾', dim);
            set(grid, root_x + w + 1, y_b, '╼', dim);
            set(grid, root_x, y_t, '┼', color);
            BoleExit {
                x: root_x,
                y: y_t,
                left: 1,
                right: 1,
            }
        }
        // Coil: nested arcs stacked into a flattened spiral; the tail
        // sweeps out one side and the gap rotates ring to ring
        26 => {
            let w0 = lw.max(rw).max(4);
            let w1 = (w0 * 2 / 3).max(2);
            let tail_right = rng.random_range(0..2u32) == 0;
            let ts = if tail_right { 1 } else { -1 };
            // outer ring: upward cup on the ground row
            set(grid, root_x - w0 - 1, root_y, '◟', dim);
            for dx in -w0..=w0 {
                set(
                    grid,
                    root_x + dx,
                    root_y,
                    '◡',
                    darken(bark, (dx.abs() as u8 * 2).min(16)),
                );
            }
            set(grid, root_x + w0 + 1, root_y, '◞', dim);
            // tail sweeps out from the outer ring
            set(grid, root_x + ts * (w0 + 2), root_y, '─', dim);
            set(
                grid,
                root_x + ts * (w0 + 3),
                root_y,
                if tail_right { '╴' } else { '╶' },
                darken(dim, 10),
            );
            // inner ring: downward cap, shifted opposite the tail (spiral offset)
            let ox = -ts;
            set(grid, root_x + ox - w1 - 1, root_y - 1, '◜', bark);
            for dx in -w1..=w1 {
                set(
                    grid,
                    root_x + ox + dx,
                    root_y - 1,
                    '◠',
                    lighten(bark, (dx.abs() as u8 * 3).min(18)),
                );
            }
            set(grid, root_x + ox + w1 + 1, root_y - 1, '◝', bark);
            // coil eye where the spiral terminates
            set(grid, root_x + ox * 2, root_y - 1, '◉', color);
            set(grid, root_x, root_y - 1, '╂', color);
            BoleExit {
                x: root_x,
                y: root_y - 1,
                left: 1,
                right: 1,
            }
        }
        // Taproot: low mound above ground, winding roots dig below it
        27 => {
            set(grid, root_x, root_y, '┴', color);
            for dx in 1..=lw.max(2) {
                let ch = if dx == lw.max(2) { '╮' } else { '─' };
                set(
                    grid,
                    root_x - dx,
                    root_y,
                    ch,
                    darken(bark, (dx as u8 * 3).min(18)),
                );
            }
            for dx in 1..=rw.max(2) {
                let ch = if dx == rw.max(2) { '╭' } else { '─' };
                set(
                    grid,
                    root_x + dx,
                    root_y,
                    ch,
                    darken(bark, (dx as u8 * 3).min(18)),
                );
            }
            // winding roots: walkers descend 1-2 rows, drifting and reversing
            let n_roots = rng.random_range(3..5u32);
            for r in 0..n_roots {
                let span = (lw + rw).max(2);
                let mut x = root_x + rng.random_range(0..span as u32) as i32 - lw;
                if x == root_x {
                    x += if r % 2 == 0 { 1 } else { -1 };
                }
                let mut drift: i32 = if x < root_x { -1 } else { 1 };
                let depth = if compact {
                    1
                } else {
                    rng.random_range(1..3u32) as i32
                };
                let mut y = root_y;
                for _ in 0..depth {
                    y += 1;
                    if rng.random::<f32>() < 0.6 {
                        x += drift;
                        set(grid, x, y, if drift > 0 { '╲' } else { '╱' }, dim);
                    } else {
                        set(grid, x, y, '│', dim);
                    }
                    if rng.random::<f32>() < 0.3 {
                        drift = -drift;
                    }
                }
                set(grid, x, y + 1, '╷', darken(dim, 10));
            }
            BoleExit {
                x: root_x,
                y: root_y,
                left: 1,
                right: 1,
            }
        }
        // ── Structural boles: legs, piles, dens, claws, shelves, grass ──

        // Stilts: the trunk stands on splayed mangrove prop legs with
        // open air underneath
        28 => {
            set(grid, root_x, root_y - 1, '┴', color);
            set(grid, root_x - 1, root_y - 1, '╭', bark);
            set(grid, root_x + 1, root_y - 1, '╮', bark);
            let pairs = (lw.max(rw) / 2).clamp(1, 3);
            for k in 1..=pairs {
                // each pair splays one cell further out per row down
                let hip = k;
                set(
                    grid,
                    root_x - hip - 1,
                    root_y,
                    '╱',
                    darken(bark, (k as u8 * 6).min(18)),
                );
                set(
                    grid,
                    root_x + hip + 1,
                    root_y,
                    '╲',
                    darken(bark, (k as u8 * 6).min(18)),
                );
                set(grid, root_x - hip - 2, root_y + 1, '╱', dim);
                set(grid, root_x + hip + 2, root_y + 1, '╲', dim);
            }
            // feet grip the ground below the outermost legs
            set(grid, root_x - pairs - 3, root_y + 2, '╷', darken(dim, 10));
            set(grid, root_x + pairs + 3, root_y + 2, '╷', darken(dim, 10));
            // one off-center brace leg for asymmetry
            let bx = if rng.random_range(0..2u32) == 0 {
                -1
            } else {
                1
            };
            set(grid, root_x + bx, root_y, '│', bark);
            set(grid, root_x + bx, root_y + 1, '╷', dim);
            BoleExit {
                x: root_x,
                y: root_y - 1,
                left: 1,
                right: 1,
            }
        }
        // Cairn: rounded stones piled against the trunk base
        29 => {
            let stones = ['◯', '●', '○', '◦'];
            let hw = lw.max(rw).max(2);
            for dx in -hw..=hw {
                let ch = stones[rng.random_range(0..stones.len() as u32) as usize];
                set(
                    grid,
                    root_x + dx,
                    root_y,
                    ch,
                    darken(bark, (dx.abs() as u8 * 4).min(20)),
                );
            }
            // top course: fewer, smaller stones nested in the gaps
            let tw = (hw / 2).max(1);
            for dx in -tw..=tw {
                if dx == 0 {
                    continue;
                }
                let ch = if rng.random_range(0..2u32) == 0 {
                    '○'
                } else {
                    '◦'
                };
                set(grid, root_x + dx, root_y - 1, ch, bark);
            }
            set(grid, root_x, root_y - 1, '╨', color);
            BoleExit::point(root_x, root_y - 2)
        }
        // Hollow: a den opening framed in the trunk base, sloped
        // shoulders on either side
        30 => {
            let hw = lw.max(rw).max(3);
            // rim row
            set(grid, root_x, root_y - 1, '┴', color);
            for dx in 1..hw {
                set(grid, root_x - dx, root_y - 1, '─', bark);
                set(grid, root_x + dx, root_y - 1, '─', bark);
            }
            set(grid, root_x - hw, root_y - 1, '╭', bark);
            set(grid, root_x + hw, root_y - 1, '╮', bark);
            // ground row: arch entrance at center, shoulders slope away
            set(grid, root_x - 1, root_y, '╭', lighten(bark, 15));
            set(grid, root_x + 1, root_y, '╮', lighten(bark, 15));
            // the hole itself stays blank at root_x
            for dx in 2..hw {
                set(
                    grid,
                    root_x - dx,
                    root_y,
                    '▒',
                    darken(bark, (dx as u8 * 3).min(15)),
                );
                set(
                    grid,
                    root_x + dx,
                    root_y,
                    '▒',
                    darken(bark, (dx as u8 * 3).min(15)),
                );
            }
            set(grid, root_x - hw, root_y, '╱', dim);
            set(grid, root_x + hw, root_y, '╲', dim);
            BoleExit {
                x: root_x,
                y: root_y - 1,
                left: 2,
                right: 2,
            }
        }
        // Talon: clawed digits radiate from a center pad and dig in
        31 => {
            set(grid, root_x, root_y - 1, '┴', color);
            let reach_l = lw.max(2).min(4);
            let reach_r = rw.max(2).min(4);
            // outer digits: out along the pad, bend, then dig
            for (side, reach) in [(-1i32, reach_l), (1i32, reach_r)] {
                let digits = (reach / 2).max(1);
                // longest digit first so shorter bends overwrite its run
                for d in (1..=digits).rev() {
                    let bend = side * (d * 2);
                    for s in 1..(d * 2) {
                        set(grid, root_x + side * s, root_y - 1, '─', bark);
                    }
                    set(
                        grid,
                        root_x + bend,
                        root_y - 1,
                        if side < 0 { '╭' } else { '╮' },
                        bark,
                    );
                    set(grid, root_x + bend, root_y, '│', darken(bark, 8));
                    set(grid, root_x + bend, root_y + 1, '╷', dim);
                }
            }
            // center digit digs straight down
            set(grid, root_x, root_y, '│', bark);
            set(grid, root_x, root_y + 1, '╷', dim);
            BoleExit::point(root_x, root_y - 2)
        }
        // Tiers: stacked pagoda shelves shrinking upward, drip legs
        // under the outer edges
        32 => {
            let w0 = lw.max(rw).max(3);
            let levels = if energy > 0.6 { 3 } else { 2 };
            let mut cy = root_y;
            let mut w = w0;
            for lvl in 0..levels {
                let center = if lvl == levels - 1 { '┴' } else { '╪' };
                set(grid, root_x, cy, center, color);
                for dx in 1..=w {
                    set(
                        grid,
                        root_x - dx,
                        cy,
                        '═',
                        darken(bark, (dx as u8 * 3).min(18)),
                    );
                    set(
                        grid,
                        root_x + dx,
                        cy,
                        '═',
                        darken(bark, (dx as u8 * 3).min(18)),
                    );
                }
                if lvl == 0 {
                    // drip legs under the widest shelf
                    set(grid, root_x - w, cy + 1, '╷', dim);
                    set(grid, root_x + w, cy + 1, '╷', dim);
                }
                cy -= 1;
                w = (w * 3 / 5).max(1);
                if w < 1 {
                    break;
                }
            }
            BoleExit::point(root_x, root_y - levels)
        }
        // Tussock: a clump of grass blades hides the trunk base
        33 => {
            let blades = ['⌇', '╿', '╽', '┆', '╵'];
            let hw = lw.max(rw).max(2);
            for dx in -hw..=hw {
                let ch = blades[rng.random_range(0..blades.len() as u32) as usize];
                set(
                    grid,
                    root_x + dx,
                    root_y,
                    ch,
                    lighten(bark, rng.random_range(0..25u32) as u8),
                );
            }
            // taller blades near the center on a second row
            let tw = (hw / 2).max(1);
            for dx in -tw..=tw {
                if rng.random::<f32>() < 0.5 {
                    let ch = blades[rng.random_range(0..blades.len() as u32) as usize];
                    set(grid, root_x + dx, root_y - 1, ch, bark);
                }
            }
            // seed heads drift above the clump
            for _ in 0..rng.random_range(1..4u32) {
                let sx = root_x + rng.random_range(0..(hw * 2 + 1) as u32) as i32 - hw;
                set(grid, sx, root_y - 2, '·', dim);
            }
            set(grid, root_x, root_y - 1, '│', color);
            BoleExit::point(root_x, root_y - 1)
        }
        _ => BoleExit::point(root_x, root_y),
    }
}
