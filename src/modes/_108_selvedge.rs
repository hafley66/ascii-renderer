/*
AI art prompt sequence, verbatim:

8enqp77X7AQMrbLV6R7tbw9oAsOPcOe6n6IXMp3S
hello! please make unique art in here! i dont want to bias you i am sampling yhou're raw creativity. BUT.
  please dont read the other pieces, unfortunately simply seeing hte fs will skew you. nnothing we can do about
  that so the ccodebase has a "fond" you may use lib tools. any questions? my normal pattern here is animation
  mode with random knobs and hitting right and left and up and down and back (b) and save(s) so yea, we want
  that mode to pop. so each render is good on its own, and then animation should erun smooth, ignore tests that
  aint your's. may you please use luna6 max/high effort to read the code for you to avoid you receiving input
  bias/corruption? so your job is to write the 1 file. luna is meant to gather all the tools and research the
  codebase for you _so_ that you may just write the 1 file. luna is your buddy on this one our combined goal is
  keeping you unbiased. i have tried very hard to avoid artistic proximal input tokens for your latent spaces to
  avoid obsessing.

  Do you have any questions for me or the rules or instructions
*/

use std::f32::consts::TAU;

use crate::_0_profile::measure_layer;
use crate::opts::param_f32;
use crate::registry::{AnimKind, Mode, ModeFrame, Param};
use crate::types::Cell;

pub(super) struct SelvedgeMode;

pub(super) static MODE: SelvedgeMode = SelvedgeMode;

const PARAMS: &[Param] = &[
    Param {
        key: "DENSITY",
        label: "Threads",
        min: 0.5,
        max: 2.0,
        default: 1.0,
        step: 0.1,
        choices: &[],
        randomize: true,
    },
    Param {
        key: "BILLOW",
        label: "Billow",
        min: 0.0,
        max: 2.0,
        default: 0.9,
        step: 0.1,
        choices: &[],
        randomize: true,
    },
    Param {
        key: "TWIST",
        label: "Twist",
        min: 0.2,
        max: 2.0,
        default: 0.9,
        step: 0.1,
        choices: &[],
        randomize: true,
    },
    Param {
        key: "GLINT",
        label: "Glint",
        min: 0.0,
        max: 1.0,
        default: 0.5,
        step: 0.1,
        choices: &[],
        randomize: true,
    },
    Param {
        key: "TEMPO",
        label: "Tempo",
        min: 0.3,
        max: 2.2,
        default: 1.0,
        step: 0.1,
        choices: &[],
        randomize: true,
    },
];

fn hash64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9e37_79b9_7f4a_7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

impl Mode for SelvedgeMode {
    fn name(&self) -> &'static str {
        "selvedge"
    }

    fn help(&self) -> &'static str {
        "A breathing aperture of interlaced threads"
    }

    fn animation(&self) -> AnimKind {
        AnimKind::Iterate
    }

    fn params(&self) -> &'static [Param] {
        PARAMS
    }

    fn render(&self, frame: &mut ModeFrame<'_>) {
        measure_layer(self.name(), "render", || {
            // Read the five live controls and derive one stable geometry from this frame.
            // Place sparse seed-fixed marks, then trace both thread directions.
            // Resolve alternating crossings, add a moving shuttle, and seal the aperture.
            let (width, height) = (frame.width, frame.height);
            if width == 0 || height == 0 {
                return;
            }

            let density = param_f32("DENSITY", 1.0).clamp(0.5, 2.0);
            let billow = param_f32("BILLOW", 0.9).clamp(0.0, 2.0);
            let twist = param_f32("TWIST", 0.9).clamp(0.2, 2.0);
            let glint = param_f32("GLINT", 0.5).clamp(0.0, 1.0);
            let tempo = param_f32("TEMPO", 1.0).clamp(0.3, 2.2);
            let time = frame.time * tempo;
            let phase = (hash64(frame.seed) as f64 / u64::MAX as f64) as f32 * TAU;
            let cx = (width as f32 - 1.0) * 0.5;
            let cy = (height as f32 - 1.0) * 0.5;
            let rx = (width as f32 * 0.43).max(0.5);
            let ry = (height as f32 * 0.42).max(0.5);
            let warp_count = ((width as f32 / 7.2 * density).round() as usize).clamp(3, 96);
            let weft_count = ((height as f32 / 3.7 * density).round() as usize).clamp(3, 96);
            let tension = density * density;

            let star_count = (width.saturating_mul(height) / 170).min(30_000);
            for i in 0..star_count {
                let h = hash64(frame.seed ^ (i as u64).wrapping_mul(0x632b_e59b_d9b4_e019));
                let x = h as usize % width;
                let y = (h >> 32) as usize % height;
                let ch = if h & 31 == 0 { '+' } else { '.' };
                frame.grid[y][x] = Cell::new(ch, frame.palette[0]);
            }

            for i in 0..warp_count {
                let strand = (i as f32 + 0.5) / warp_count as f32 * 2.0 - 1.0;
                let strand_phase = phase + i as f32 * 0.73;
                for y in 0..height {
                    let v = (y as f32 - cy) / ry;
                    if v.abs() >= 0.94 {
                        continue;
                    }
                    let envelope = 1.0 - v * v;
                    let angle = v * TAU * 1.15 * twist + strand_phase + time;
                    let bend = angle.sin() * rx * 0.055 * billow * envelope / tension;
                    let shear = v * strand * rx * 0.13 * twist / density;
                    let xf = cx + strand * rx + bend + shear;
                    if ((xf - cx) / rx).powi(2) + v * v >= 0.89 {
                        continue;
                    }
                    let x = xf.round() as isize;
                    if x < 0 || x >= width as isize {
                        continue;
                    }
                    let slope = angle.cos() * rx * 0.055 * billow * envelope * TAU * 1.15 * twist
                        / (ry * tension)
                        + strand * rx * 0.13 * twist / (ry * density);
                    let ch = if slope > 0.55 {
                        '\\'
                    } else if slope < -0.55 {
                        '/'
                    } else {
                        '|'
                    };
                    frame.grid[y][x as usize] = Cell::new(ch, frame.palette[1 + i % 2]);
                }
            }

            for j in 0..weft_count {
                let strand = (j as f32 + 0.5) / weft_count as f32 * 2.0 - 1.0;
                let strand_phase = phase * 0.7 + j as f32 * 0.91;
                for x in 0..width {
                    let u = (x as f32 - cx) / rx;
                    if u.abs() >= 0.94 {
                        continue;
                    }
                    let envelope = 1.0 - u * u;
                    let angle = u * TAU * 1.35 * twist + strand_phase - time * 0.86;
                    let bend = angle.sin() * ry * 0.14 * billow * envelope / tension;
                    let shear = -u * strand * ry * 0.12 * twist / density;
                    let yf = cy + strand * ry + bend + shear;
                    if u * u + ((yf - cy) / ry).powi(2) >= 0.89 {
                        continue;
                    }
                    let y = yf.round() as isize;
                    if y < 0 || y >= height as isize {
                        continue;
                    }
                    let cell = &mut frame.grid[y as usize][x];
                    if matches!(cell.ch, '|' | '/' | '\\') {
                        let warp_index =
                            (((x as f32 - cx + rx) / (2.0 * rx)) * warp_count as f32) as usize;
                        let shimmer =
                            (warp_index as f32 * 1.7 + j as f32 * 2.3 + time * 2.0 + phase).sin();
                        let ch = if shimmer > 1.0 - glint * 0.55 {
                            '*'
                        } else if (warp_index + j) % 2 == 0 {
                            'o'
                        } else {
                            'x'
                        };
                        *cell = Cell::new(ch, frame.palette[4]);
                    } else {
                        let slope =
                            angle.cos() * ry * 0.14 * billow * envelope * TAU * 1.35 * twist
                                / (rx * tension)
                                - strand * ry * 0.12 * twist / (rx * density);
                        let ch = if slope > 0.38 {
                            '\\'
                        } else if slope < -0.38 {
                            '/'
                        } else {
                            '-'
                        };
                        *cell = Cell::new(ch, frame.palette[2 + j % 2]);
                    }
                }
            }

            let shuttle_angle = phase + time * 0.42;
            let shuttle_x = (cx + rx * 0.78 * shuttle_angle.cos()).round() as isize;
            let shuttle_y = (cy + ry * 0.78 * shuttle_angle.sin()).round() as isize;
            if shuttle_x >= 0
                && shuttle_x < width as isize
                && shuttle_y >= 0
                && shuttle_y < height as isize
            {
                frame.grid[shuttle_y as usize][shuttle_x as usize] =
                    Cell::new('@', frame.palette[4]);
            }

            let border_steps = width
                .saturating_add(height)
                .saturating_mul(3)
                .clamp(64, 24_000);
            for i in 0..border_steps {
                let angle = TAU * i as f32 / border_steps as f32;
                let cos = angle.cos();
                let sin = angle.sin();
                for (scale, ch, color) in
                    [(1.0, ':', frame.palette[3]), (0.925, '.', frame.palette[1])]
                {
                    let x = (cx + rx * scale * cos).round() as isize;
                    let y = (cy + ry * scale * sin).round() as isize;
                    if x >= 0 && x < width as isize && y >= 0 && y < height as isize {
                        frame.grid[y as usize][x as usize] = Cell::new(ch, color);
                    }
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::style::Color;
    use rand::{rngs::StdRng, SeedableRng};

    fn rendered(width: usize, height: usize, seed: u64, time: f32, maxima: bool) -> String {
        let mut grid = vec![vec![Cell::blank(); width]; height];
        let mut rng = StdRng::seed_from_u64(seed);
        let palette = [Color::Reset; 5];
        let param_values: Vec<f32> = PARAMS
            .iter()
            .map(|param| if maxima { param.max } else { param.default })
            .collect();
        crate::opts::LIVE_PARAMS.with(|values| {
            let mut values = values.borrow_mut();
            values.clear();
            for (param, value) in PARAMS.iter().zip(&param_values) {
                values.insert(param.key, Some(*value));
            }
        });
        let mut frame = ModeFrame {
            grid: &mut grid,
            width,
            height,
            seed,
            palette: &palette,
            rng: &mut rng,
            time,
            args: &[],
            param_values: Some(&param_values),
        };
        MODE.render(&mut frame);
        crate::opts::LIVE_PARAMS.with(|values| values.borrow_mut().clear());
        grid.iter()
            .map(|row| row.iter().map(|cell| cell.ch).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn fixed_seed_snapshot() {
        let output = rendered(80, 24, 42, 0.0, false);
        insta::with_settings!({ snapshot_path => "snapshots" }, {
            insta::assert_snapshot!("selvedge_seed_42", output);
        });
    }

    #[test]
    fn moving_snapshot() {
        let output = rendered(80, 24, 42, 2.75, false);
        insta::with_settings!({ snapshot_path => "snapshots" }, {
            insta::assert_snapshot!("selvedge_seed_42_t2_75", output);
        });
    }

    #[test]
    fn maximum_knobs_snapshot() {
        let output = rendered(80, 24, 42, 0.0, true);
        insta::with_settings!({ snapshot_path => "snapshots" }, {
            insta::assert_snapshot!("selvedge_seed_42_max", output);
        });
    }

    #[test]
    fn time_and_bounds() {
        let initial = rendered(80, 24, 42, 0.0, false);
        assert_eq!(initial, rendered(80, 24, 42, 0.0, false));
        assert_ne!(initial, rendered(80, 24, 42, 2.75, false));
        for (width, height) in [(0, 0), (1, 1), (2, 9), (80, 1), (3, 3)] {
            let _ = rendered(width, height, 42, 9.0, true);
        }
    }
}
