//! Weather overlay.
#![allow(unused)]
use crate::color::*;
use crate::fills::*;
use crate::scene::*;
use crate::sprites::*;
use crate::types::*;
use crossterm::style::Color;
use rand::RngExt;
use rand::rngs::StdRng;
use super::*;

/// Weather type for atmosphere overlay.
#[derive(Clone, Copy)]
pub enum Weather {
    Rain,
    Snow,
    Fog,
    Stars,
    None,
}
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
    if h == 0 {
        return;
    }
    let w = grid[0].len();
    let prob = intensity as f32 / 100.0;

    match weather {
        Weather::Rain => {
            let rain_color = darken(palette[2], 40);
            let rain_chars = ['│', '┊', '╎', '┆'];
            for y in 0..h {
                for x in 0..w {
                    if grid[y][x].ch != ' ' {
                        continue;
                    }
                    if rng.random::<f32>() > prob * 0.15 {
                        continue;
                    }
                    // Rain falls in vertical streaks -- bias toward same x columns
                    let streak = ((x * 7 + 13) % 11) < 3;
                    if !streak && rng.random::<f32>() > 0.3 {
                        continue;
                    }
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
                    if grid[y][x].ch != ' ' {
                        continue;
                    }
                    if rng.random::<f32>() > prob * 0.08 {
                        continue;
                    }
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
                let row_fog =
                    ((y as f32 / h as f32 * 3.0 * std::f32::consts::PI).sin() * 0.5 + 0.5) * prob;
                for x in 0..w {
                    if rng.random::<f32>() > row_fog * 0.12 {
                        continue;
                    }
                    // Fog can overwrite sparse chars but not dense structure
                    let existing = grid[y][x].ch;
                    if existing != ' ' && !matches!(existing, '·' | '∙' | '°' | '⋅') {
                        continue;
                    }
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
                    if grid[y][x].ch != ' ' {
                        continue;
                    }
                    if rng.random::<f32>() > prob * 0.03 {
                        continue;
                    }
                    let ch = star_chars[rng.random_range(0..star_chars.len())];
                    let twinkle = rng.random_range(0..60);
                    grid[y][x] = Cell::new(ch, darken(star_color, twinkle));
                }
            }
        }
        Weather::None => {}
    }
}

