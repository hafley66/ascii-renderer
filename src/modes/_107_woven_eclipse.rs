/*
AI art prompt sequence, verbatim:

hello! please make unique art in here! i dont want to bias you i am sampling yhou're raw creativity. BUT. please dont read the other pieces, unfortunately simply seeing hte fs will skew you. nnothing we can do about that so the ccodebase has a "fond" you may use lib tools. any questions? my normal pattern here is animation mode with random knobs and hitting right and left and up and down and back (b) and save(s) so yea, we want that mode to pop. so each render is good on its own, and then animation should erun smooth, ignore tests that aint your's. may you please use luna6 max/high effort to read the code for you to avoid you receiving input bias/corruption? so your job is to write the 1 file. luna6 is meant to gather all the tools and research the codebase for you _so_ that you may just write the 1 file. luna is your buddy on this one our combined goal is keeping you unbiased. i have tried very hard to avoid artistic proximal input tokens for your latent spaces to avoid obsessing.

Do you have any questions for me or the rules or instructions
*/

use crate::registry::{AnimKind, Mode, ModeFrame, Param};
use crate::types::Cell;

pub(super) static MODE: WovenEclipse = WovenEclipse;

pub(super) struct WovenEclipse;

static PARAMS: [Param; 5] = [
    Param {
        key: "threads",
        label: "Threads",
        min: 8.0,
        max: 22.0,
        default: 14.0,
        step: 1.0,
        choices: &[],
        randomize: true,
    },
    Param {
        key: "twist",
        label: "Twist",
        min: 0.3,
        max: 2.4,
        default: 1.35,
        step: 0.05,
        choices: &[],
        randomize: true,
    },
    Param {
        key: "aperture",
        label: "Aperture",
        min: 0.13,
        max: 0.30,
        default: 0.21,
        step: 0.01,
        choices: &[],
        randomize: true,
    },
    Param {
        key: "current",
        label: "Current",
        min: 0.0,
        max: 0.10,
        default: 0.035,
        step: 0.005,
        choices: &[],
        randomize: true,
    },
    Param {
        key: "speed",
        label: "Speed",
        min: 0.25,
        max: 2.0,
        default: 1.0,
        step: 0.05,
        choices: &[],
        randomize: true,
    },
];

fn param(frame: &ModeFrame<'_>, index: usize) -> f32 {
    let p = &PARAMS[index];
    frame
        .param_values
        .and_then(|values| values.get(index).copied())
        .unwrap_or_else(|| crate::opts::param_f32(p.key, p.default))
        .clamp(p.min, p.max)
}

fn hash(x: usize, y: usize, seed: u64) -> u32 {
    let mut n = seed
        ^ (x as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15)
        ^ (y as u64).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    n ^= n >> 30;
    n = n.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    n ^= n >> 27;
    n = n.wrapping_mul(0x94d0_49bb_1331_11eb);
    (n ^ (n >> 31)) as u32
}

impl Mode for WovenEclipse {
    fn name(&self) -> &'static str {
        "woven_eclipse"
    }

    fn help(&self) -> &'static str {
        "A moving weave curves around a dark aperture"
    }

    fn animation(&self) -> AnimKind {
        AnimKind::Iterate
    }

    fn params(&self) -> &'static [Param] {
        &PARAMS
    }

    fn render(&self, frame: &mut ModeFrame<'_>) {
        let width = frame.width;
        let height = frame.height;
        if width == 0 || height == 0 {
            return;
        }

        let threads = param(frame, 0);
        let twist = param(frame, 1);
        let aperture = param(frame, 2);
        let current = param(frame, 3);
        let speed = param(frame, 4);
        let time = frame.time * speed;
        let seed_phase = (frame.seed as u32 as f32 / u32::MAX as f32) * std::f32::consts::TAU;
        let scale = (width as f32).min(height as f32 * 2.0).max(1.0);
        let spacing = 1.0 / threads;
        let radius = aperture * (1.0 + 0.025 * (time * 0.4 + seed_phase).sin());
        let radius_inner = (radius - 0.022).max(0.0);
        let radius_outer = radius + 0.05;
        let glint_x = radius * 0.43 * (time * 0.37 + seed_phase).cos();
        let glint_y = radius * 0.43 * (time * 0.37 + seed_phase).sin();
        let row_wave: Vec<f32> = (0..height)
            .map(|y| {
                let py = (y as f32 + 0.5 - height as f32 * 0.5) * 2.0 / scale;
                current * (py * 18.0 + time * 0.46 + seed_phase).sin()
            })
            .collect();
        let col_wave: Vec<f32> = (0..width)
            .map(|x| {
                let px = (x as f32 + 0.5 - width as f32 * 0.5) / scale;
                current * (px * 21.0 - time * 0.54 + seed_phase * 0.67).sin()
            })
            .collect();

        for y in 0..height {
            let py = (y as f32 + 0.5 - height as f32 * 0.5) * 2.0 / scale;
            for x in 0..width {
                let px = (x as f32 + 0.5 - width as f32 * 0.5) / scale;
                let r2 = px * px + py * py;
                let h = hash(x, y, frame.seed);
                let cell = &mut frame.grid[y][x];
                *cell = Cell::blank();

                if r2 < radius_inner * radius_inner {
                    let dx = (px - glint_x) * scale;
                    let dy = (py - glint_y) * scale * 0.5;
                    if dx * dx + dy * dy < 0.36 {
                        *cell = Cell::new('*', frame.palette[4]);
                    }
                    continue;
                }

                if r2 < radius_outer * radius_outer {
                    let distance = (r2.sqrt() - radius).abs();
                    if distance < 0.017 {
                        let ax = px.abs();
                        let ay = py.abs();
                        let ch = if ax < ay * 0.48 {
                            '-'
                        } else if ay < ax * 0.48 {
                            '|'
                        } else if px * py > 0.0 {
                            '\\'
                        } else {
                            '/'
                        };
                        *cell = Cell::new(ch, frame.palette[4]);
                        continue;
                    }
                    if distance < 0.038 && h % 3 == 0 {
                        *cell = Cell::new('.', frame.palette[3]);
                    }
                    continue;
                }

                if r2 > 0.75 * 0.75 {
                    if h % 89 == 0 {
                        *cell = Cell::new('.', frame.palette[0]);
                    }
                    continue;
                }

                let lens = twist / (1.0 + 18.0 * r2);
                let u = px - lens * py + row_wave[y];
                let v = py + lens * px + col_wave[x];
                let ui = (u / spacing).round() as i32;
                let vi = (v / spacing).round() as i32;
                let du = (u - ui as f32 * spacing).abs() * scale;
                let dv = (v - vi as f32 * spacing).abs() * scale * 0.5;
                let vertical = du < 0.59;
                let horizontal = dv < 0.59;

                if !vertical && !horizontal {
                    if r2 < 0.51 * 0.51 && h % 31 == 0 {
                        *cell = Cell::new('.', frame.palette[0]);
                    }
                    continue;
                }

                if r2 > 0.56 * 0.56 {
                    let fade = ((0.75 * 0.75 - r2) / (0.75 * 0.75 - 0.56 * 0.56)).clamp(0.0, 1.0);
                    if (h % 1024) as f32 / 1024.0 > fade {
                        continue;
                    }
                }

                let raised_vertical = (ui + vi) & 1 == 0;
                let show_vertical = if vertical && horizontal {
                    raised_vertical
                } else {
                    vertical
                };
                let ch = if show_vertical {
                    if lens > 1.0 {
                        '-'
                    } else if lens > 0.36 {
                        '\\'
                    } else {
                        '|'
                    }
                } else if lens > 1.0 {
                    '|'
                } else if lens > 0.36 {
                    '/'
                } else {
                    '-'
                };
                let index = if vertical && horizontal {
                    4
                } else {
                    ((ui * 3 + vi * 7).rem_euclid(4)) as usize
                };
                *cell = Cell::new(ch, frame.palette[index]);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::style::Color;
    use rand::{rngs::StdRng, SeedableRng};

    fn render(time: f32) -> String {
        let (width, height) = (76, 27);
        let mut grid = vec![vec![Cell::blank(); width]; height];
        let mut rng = StdRng::seed_from_u64(0x57e4_2115);
        let palette = [
            Color::DarkBlue,
            Color::Blue,
            Color::Cyan,
            Color::Yellow,
            Color::White,
        ];
        let args = Vec::<String>::new();
        let values = [14.0, 1.35, 0.21, 0.035, 1.0];
        let mut frame = ModeFrame {
            grid: &mut grid,
            width,
            height,
            seed: 0x57e4_2115,
            palette: &palette,
            rng: &mut rng,
            time,
            args: &args,
            param_values: Some(&values),
        };
        MODE.render(&mut frame);
        grid.iter()
            .map(|row| row.iter().map(|cell| cell.ch).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn woven_eclipse_frames() {
        insta::assert_snapshot!(&render(0.0), @r#"
              ----  -|---|  ---| --|. -|-  | --| ---|     ---               
        -     -  |--|  |---|------|--|- -----||-- |--------|          -     
           --| ----| --|  |---- |---|---|-  |   -|-  ||-- ||-----  |        
     -   |  -|--| ----|----  | /// /// \\ ///---||---||  ---      ---       
       | -|- |  -|- |  |  -//\//\ //\///////\\///-| ------ |----|   |- -    
     ---|  |----- |--|--// \  \ /\ //\   \  /\\  //\  - | ---|-----   |     
  |   | -|- |   |---- \ ////\//// /      /\/////\ //// .--|   ||  -|  -  -  
  -- - -- |------|  |///. \//\     ------  .   /\\  //\--  |------  |-  |   
 --    |---  |-  |-/\/////\    \\-       .-//    //   \\  ---   |-------||  
     ---  ---|----/ \ .\ / .\\\              ///  ////////-| --||   |-      
 -|     -|- ||--|  ////\/ .\\.                 //   /\\// |----|---|   |- - 
    ||--| ---- ||//// \/  ||                    ||   \\  //   --  |---| - | 
 -  |   ---.|---|  \////  ||                    ||  //////\---|-----  | -   
   ----- |---|  -///\\//..|                      |.    \ //|  .|   |----  | 
 - |  ||--|| -||- \\//// .||                    ||. /////\ --|------|       
  - ----| --|----/// \    ||       *            ||  \  \ //|  |   |--   -||-
-    || -||   |   \//////  //                 .\\. //////\ -|------|   | - |
  - -- -- |------- \\   \\  ///              \\\  /\  \ //|  |--|  ---- |   
         ---  || --//  /\\//  .//-        -\\.  ///\ /\/  |-----|---|  | -- 
  - -----|  --|-- ||-/////   \     ------     ///  ///\---- |---|  |- - - | 
        | ---- --||   \\  //\/// \/      \//\//\ /// \  | -|-  | -| - |     
 --|   ---- | - ||---||-////////\ //\ //\//\   \//\ ----|- |------|  |--    
      - |  -|-----  ------ \  .\//  ////\///////  |--|  |---  |  --- |  -   
           - |  --|-   |  -||--/\ //\\///   \  |--|. |-- | ----|--          
      |     ----  ------|--  | --| --|-------|---  ---|----  |     -| -     
          -  ---|--  |-- ||----  ---  --- |  |   |--|  |  ---  -- -         
         -   -    ---|| ------|---||------|------|  |-- -- |  -   |
"#);
        insta::assert_snapshot!(&render(3.6), @r#"
               -|- --- |--|----- |  --- |----|-- |   ||-  |                 
        -  -  - |  |  --- |  | --|---| --|  ||  ----- |---|| ---            
               -----|--| -----|- |  -|- -|--------|----| -----  - --        
     |   --|---| ----| -|-  | -\///// /// ///-||   || --||   |- - |         
    -- - |  | ---  | --- |-/\/  \  \///\///\   /\-----  ------|| -- -- |    
      | -|----  ---|--| ///////////\/ /\  /\/////\  -||-  ||   |---|  --    
  |   ---|   ---|  |  \//\  \. \  \ //\ //\\// \\ // \ ---| ----  --     -  
  |- -  ----|  |-----/\..\/////     ----   .  \\//////  ------| --||-- -    
 |  | ---  |---| --//\ /\///   \\---    ---//   // \////\    |---||   |--- -
     - | ---. |---\  ///\  ..\\\            ///   \\   /\/-----  ----|      
    | -||------| -\///  \ .\\\                ///  //// \/---| --|   | -- - 
    --  |---| --|  \  /\/  ||                  ||    \\///\ --|- -|----   - 
     -|-------| -\\////   ||     *              ||  /   ///-|  -|-  | --|   
   ---  |---| -||  \  /\ .||                    ||. //\// \  -|-  |---|  -- 
 -   |---- --|-  \/////  .||                    ||. /   ///|-- |---| -||    
  - -|  -|---||--/\   \//  ||                  ||.  /\///   |---  ---    -||
-   -----|  ---  -////\ /  ///                \\\. //\  \/---  ----| - ---  
  - -   ----|  -||//////  /  ///            \\\.  \  \//\/ |--|-  |---   -  
   |-      ||---|    \////\\  .//---.   ---\\.  /\/////\  |-- |---   |----  
  |    |------  -----\  //\\///     ----.     \  \///  ---|----  ---- - | - 
  --    | - | --||  --//  ///// \\ /// // /////\  \. \--|  |  --- |  |--    
       ------||   ||  -|\/  \\ //\\  /\ /\/ \ //\///--| -|-----| --| --     
      | - ||-  |------  ---///\/////// // \//\   |  | ----|  -- -|      |   
             ----|   ||-  |--- \   \ //\ ////-|-----|-- |-----| --|         
           ----  ----||---||  -----|-  |-  |  | -----| -|-----|             
              --|------  -----|   |  -|  -|------| .|--|   |-  -- ---       
         - | - -   || ---   || ------|.-||  |  -|-----| --|-  - | -
"#);
    }
}
