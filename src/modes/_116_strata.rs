use crossterm::style::Color;

use crate::_0_profile::measure_layer;
use crate::opts::param_f32;
use crate::registry::{AnimKind, Mode, ModeFrame, Param};
use crate::types::Cell;

pub(super) struct Strata;
pub(super) static MODE: Strata = Strata;

const NAME: &str = "strata";
const HELP: &str = "strata: alien drill core [fold] [fault] [grain] [erosion] [fossils] [speed]";
const PARAMS: &[Param] = &[
    param!("FOLD", "fold", 0.0, 2.0, 1.0, 0.1),
    param!("FAULT", "fault", 0.0, 4.0, 2.0, 0.25),
    param!("GRAIN", "grain", 0.0, 1.0, 0.55, 0.05),
    param!("EROSION", "erosion", 0.0, 3.0, 1.5, 0.25),
    param!("FOSSILS", "fossils", 0.0, 1.0, 0.7, 0.05),
    param!("SPEED", "speed", 0.5, 2.0, 1.0, 0.1),
];

impl Mode for Strata {
    fn name(&self) -> &'static str { NAME }
    fn help(&self) -> &'static str { HELP }
    fn animation(&self) -> AnimKind { AnimKind::Iterate }
    fn params(&self) -> &'static [Param] { PARAMS }

    fn render(&self, frame: &mut ModeFrame<'_>) {
        let mut values = [0.0; 6];
        for (i, param) in PARAMS.iter().enumerate() {
            values[i] = frame.args.get(i + 4).and_then(|s| s.parse::<f32>().ok())
                .or_else(|| frame.param_values.and_then(|v| v.get(i).copied()))
                .unwrap_or_else(|| param_f32(param.key, param.default))
                .clamp(param.min, param.max);
        }
        let age = (frame.time * values[5]).rem_euclid(60.0) / 60.0;
        measure_layer(NAME, "plate", || {
            for row in frame.grid.iter_mut() {
                for cell in row.iter_mut() {
                    *cell = Cell::new(' ', Color::Rgb { r: 18, g: 16, b: 16 });
                }
            }
            let title = format!("VESPER-9 / CORE 116    {:.2} Ga", age * 4.0);
            if let Some(row) = frame.grid.get_mut(0) {
                for (x, ch) in title.chars().enumerate().take(row.len()) {
                    row[x] = Cell::new(ch, Color::Rgb { r: 220, g: 208, b: 183 });
                }
            }
        });
    }
}
