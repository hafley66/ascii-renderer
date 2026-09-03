//! haiku-2-ripple -- concentric waves expanding from center with traveling particles.
//! Liquid surface disturbance viewed from above; amplitude fades from center outward.

use crate::_0_profile::measure_layer;
use crate::color::*;
use crate::opts::param_f32;
use crate::types::*;
use crossterm::style::Color;
use rand::RngExt;
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::f32::consts::{PI, TAU};

pub(crate) struct RippleKnobs {
    pub speed: f32,
    pub hue: f32,
    pub freq: f32,
    pub decay: f32,
    pub particle_density: f32,
    pub waveform: f32,
}

impl RippleKnobs {
    pub(crate) fn from_env() -> Self {
        RippleKnobs {
            speed: param_f32("SPEED", 1.0).clamp(0.3, 3.0),
            hue: param_f32("HUE", 200.0).clamp(0.0, 360.0),
            freq: param_f32("FREQ", 5.0).clamp(2.0, 12.0),
            decay: param_f32("DECAY", 0.5).clamp(0.1, 1.0),
            particle_density: param_f32("PDENSE", 1.0).clamp(0.1, 1.2),
            waveform: param_f32("WFORM", 0.0).clamp(0.0, 2.0),
        }
    }
}

fn wave_value(distance: f32, phase: f32, freq: f32, waveform: u32) -> f32 {
    let angle = distance * freq * TAU / 24.0 - phase;
    match waveform {
        0 => angle.sin(),
        1 => if angle.sin() > 0.0 { 1.0 } else { -1.0 },
        _ => {
            let frac = (angle / TAU).fract();
            (frac * 2.0 - 1.0).clamp(-1.0, 1.0)
        }
    }
}

fn choose_glyph_for_wave(amp: f32, particle: bool) -> char {
    if particle {
        match (amp * 3.0).round() as i32 {
            2.. => '~',
            1 => '+',
            -1 => 'x',
            _ => '·',
        }
    } else {
        match (amp * 4.0).round() as i32 {
            3.. => '#',
            2 => 'O',
            1 => 'o',
            0 => '*',
            -1 => '.',
            _ => ' ',
        }
    }
}

pub(crate) fn draw_haiku_2_ripple(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    t: f32,
    k: &RippleKnobs,
) {
    let has_animation = t > 0.0;
    let phase = if has_animation { t * k.speed } else { 0.0 };
    let waveform = k.waveform.round() as u32;

    let cx = width as f32 / 2.0 - 0.5;
    let cy = height as f32 / 2.0 - 0.5;
    let max_rad = ((width as f32 / 2.0) * (width as f32 / 2.0)
        + (height as f32 / 4.0) * (height as f32 / 4.0))
        .sqrt();

    let mut seed_rng = StdRng::seed_from_u64(seed ^ 0x3A7E);
    let freq_offset = (seed_rng.random_range(-2..2) as f32) * 0.15;
    let decay_offset = (seed_rng.random_range(-10..10) as f32) * 0.02;
    let freq_adj = (k.freq + freq_offset).clamp(2.0, 12.0);
    let decay_adj = (k.decay + decay_offset).clamp(0.1, 1.0);

    let primary_color = palette[1];
    let secondary_color = palette[2];
    let accent_color = palette[3];
    let text_color = palette[4];

    measure_layer("haiku-2-ripple", "clear", || {
        for row in grid.iter_mut().take(height) {
            row.fill(Cell::blank());
        }
    });

    measure_layer("haiku-2-ripple", "waves", || {
        for y in 0..height {
            for x in 0..width {
                let dy = y as f32 - cy;
                let dx = (x as f32 - cx) * 0.5;
                let dist = (dx * dx + dy * dy).sqrt();

                if dist > max_rad * 1.1 {
                    continue;
                }

                let amp = wave_value(dist, phase, freq_adj, waveform);
                let decay_factor = (-dist * decay_adj / max_rad).exp();
                let final_amp = amp * decay_factor;

                if final_amp.abs() > 0.15 {
                    let ch = choose_glyph_for_wave(final_amp, false);
                    let intensity = decay_factor.clamp(0.0, 1.0);
                    let col = if intensity > 0.7 {
                        primary_color
                    } else if intensity > 0.4 {
                        secondary_color
                    } else {
                        darken(primary_color, 15)
                    };
                    grid[y][x] = Cell::new(ch, col);
                }
            }
        }
    });

    if has_animation {
        measure_layer("haiku-2-ripple", "particles", || {
            let particle_rng_seed = seed ^ 0x7F3C;
            let mut prng = StdRng::seed_from_u64(particle_rng_seed);
            let n_particles = ((width * height) as f32 * k.particle_density * 0.08) as usize;

            for _ in 0..n_particles {
                let px = prng.random_range(0..width);
                let py = prng.random_range(0..height);

                if grid[py][px].ch != ' ' {
                    continue;
                }

                let dy = py as f32 - cy;
                let dx = (px as f32 - cx) * 0.5;
                let dist = (dx * dx + dy * dy).sqrt();

                if dist > max_rad * 1.1 {
                    continue;
                }

                let particle_phase = phase + prng.random_range(0..360) as f32 / 360.0 * TAU;
                let amp = wave_value(dist, particle_phase, freq_adj * 0.7, waveform);
                let decay_factor = (-dist * decay_adj / max_rad).exp();
                let final_amp = amp * decay_factor;

                if final_amp.abs() > 0.2 {
                    let ch = choose_glyph_for_wave(final_amp, true);
                    grid[py][px] = Cell::new(ch, accent_color);
                }
            }
        });
    }

    measure_layer("haiku-2-ripple", "guide", || {
        let center_marker = if has_animation { '○' } else { '●' };
        if cx >= 0.0
            && cx < width as f32
            && cy >= 0.0
            && cy < height as f32
            && grid[cy as usize][cx as usize].ch == ' '
        {
            grid[cy as usize][cx as usize] = Cell::new(center_marker, text_color);
        }
    });
}

pub(crate) fn cli_haiku_2_ripple(
    mut grid: Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: [Color; 5],
    _rng: StdRng,
    t_anim: f32,
    _term_w: u16,
    _term_h: u16,
    args: &[String],
    _mode: &str,
    _theme_name: &str,
) -> (Grid, bool) {
    let mut knobs = RippleKnobs::from_env();
    if let Some(speed) = args.get(4).and_then(|s| s.parse().ok()) {
        knobs.speed = speed;
    }
    if let Some(hue) = args.get(5).and_then(|s| s.parse().ok()) {
        knobs.hue = hue;
    }
    if let Some(freq) = args.get(6).and_then(|s| s.parse().ok()) {
        knobs.freq = freq;
    }
    if let Some(decay) = args.get(7).and_then(|s| s.parse().ok()) {
        knobs.decay = decay;
    }
    if let Some(density) = args.get(8).and_then(|s| s.parse().ok()) {
        knobs.particle_density = density;
    }
    if let Some(waveform) = args.get(9).and_then(|s| s.parse().ok()) {
        knobs.waveform = waveform;
    }

    draw_haiku_2_ripple(&mut grid, width, height, seed, &palette, t_anim, &knobs);
    (grid, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(w: usize, h: usize, seed: u64, t: f32) -> String {
        let mut g = vec![vec![Cell::blank(); w]; h];
        let p = crate::color::make_palette(seed);
        let k = RippleKnobs::from_env();
        draw_haiku_2_ripple(&mut g, w, h, seed, &p, t, &k);
        g.iter()
            .map(|row| row.iter().map(|c| c.ch).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn snapshot_haiku_2_ripple_static() {
        insta::assert_snapshot!("haiku_2_ripple_80x24_static", run(80, 24, 42, 0.0));
    }

    #[test]
    fn snapshot_haiku_2_ripple_animated() {
        insta::assert_snapshot!("haiku_2_ripple_80x24_animated", run(80, 24, 42, 5.0));
    }

    #[test]
    fn snapshot_deterministic() {
        assert_eq!(run(80, 24, 42, 0.0), run(80, 24, 42, 0.0));
        assert_ne!(run(80, 24, 42, 0.0), run(80, 24, 7, 0.0));
    }

    #[test]
    fn snapshot_t_changes_frame() {
        assert_ne!(run(80, 24, 42, 0.0), run(80, 24, 42, 3.0));
        assert_ne!(run(80, 24, 42, 3.0), run(80, 24, 42, 6.0));
    }
}
