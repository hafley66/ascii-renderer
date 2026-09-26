use crossterm::style::Color;

use crate::_0_profile::measure_layer;
use crate::color::{darken, lerp_color, rgb};
use crate::opts::param_f32;
use crate::pp::pp_hash2;
use crate::registry::{AnimKind, Mode, ModeFrame, Param};
use crate::types::{Cell, Grid};

pub(super) struct Semaphore;
pub(super) static MODE: Semaphore = Semaphore;

const NAME: &str = "semaphore";
const HELP: &str = "semaphore: two lighthouses in a Morse argument [beam] [sweep] [storm] [signal] [boat] [glow]";
const PARAMS: &[Param] = &[
    Param { key: "BEAM", label: "beam width", min: 0.5, max: 2.0, default: 1.0, step: 0.1, choices: &[], randomize: true },
    Param { key: "SWEEP", label: "sweep", min: 0.2, max: 2.0, default: 1.0, step: 0.1, choices: &[], randomize: true },
    Param { key: "STORM", label: "storm", min: 0.2, max: 2.0, default: 1.0, step: 0.1, choices: &[], randomize: true },
    Param { key: "SIGNAL", label: "Morse tempo", min: 0.5, max: 2.0, default: 1.0, step: 0.1, choices: &[], randomize: true },
    Param { key: "BOAT", label: "boat speed", min: 0.4, max: 2.0, default: 1.0, step: 0.1, choices: &[], randomize: true },
    Param { key: "GLOW", label: "lamp glow", min: 0.3, max: 2.0, default: 1.0, step: 0.1, choices: &[], randomize: true },
];

#[derive(Clone, Copy)]
struct Knobs {
    beam: f32,
    sweep: f32,
    storm: f32,
    signal: f32,
    boat: f32,
    glow: f32,
}

impl Mode for Semaphore {
    fn name(&self) -> &'static str { NAME }
    fn help(&self) -> &'static str { HELP }
    fn animation(&self) -> AnimKind { AnimKind::Iterate }
    fn params(&self) -> &'static [Param] { PARAMS }

    fn render(&self, frame: &mut ModeFrame<'_>) {
        let mut values = [0.0; 6];
        for (i, param) in PARAMS.iter().enumerate() {
            values[i] = frame.args.get(i + 4)
                .and_then(|value| value.parse::<f32>().ok())
                .or_else(|| frame.param_values.and_then(|live| live.get(i).copied()))
                .unwrap_or_else(|| param_f32(param.key, param.default))
                .clamp(param.min, param.max);
        }
        let knobs = Knobs {
            beam: values[0], sweep: values[1], storm: values[2],
            signal: values[3], boat: values[4], glow: values[5],
        };
        let w = frame.width.min(frame.grid.iter().map(Vec::len).min().unwrap_or(0));
        let h = frame.height.min(frame.grid.len());
        if w < 32 || h < 12 { return; }
        let time = frame.time.max(0.0);
        let story = (frame.seed % DIALOGUES.len() as u64) as usize;
        let speech = speech_at(DIALOGUES[story], time * knobs.signal);
        let horizon = (h as f32 * 0.62) as usize;
        let colors = Colors::new(frame.palette);
        let boat_x = ((0.27 + 0.46 * (time * knobs.boat / 105.0).min(1.0)) * w as f32) as i32;
        let boat_y = horizon as i32 + (h as f32 * 0.13) as i32
            + (time * 1.7).sin().round() as i32;
        let heat = if speech.done { 0.28 } else { 0.38 + speech.turn as f32 * 0.12 };
        let clock = time * knobs.signal;
        let flash = [25.4, 45.1, 63.7]
            .iter().any(|event| (clock - event).abs() < 0.12);
        let mut light = vec![0.0; w * h];
        measure_layer(NAME, "night", || night(frame.grid, w, h, horizon, colors));
        measure_layer(NAME, "beams", || beam_field(&mut light, w, h, horizon, boat_x, boat_y, time, &speech, knobs));
        measure_layer(NAME, "storm", || weather(frame.grid, &light, w, h, horizon, time, frame.seed, heat * knobs.storm, flash, colors));
        measure_layer(NAME, "sea", || sea(frame.grid, &light, w, h, horizon, time, heat * knobs.storm, flash, colors));
        measure_layer(NAME, "towers", || towers(frame.grid, w, horizon, &speech, colors, knobs.glow));
        measure_layer(NAME, "boat", || boat(frame.grid, &light, w, h, boat_x, boat_y, time, colors));
        measure_layer(NAME, "captions", || caption(frame.grid, w, h, &speech, DIALOGUES[story], colors));
        if flash { measure_layer(NAME, "lightning", || lightning(frame.grid, w, h, horizon, frame.seed, colors)); }
    }
}

const DIALOGUES: [[&str; 8]; 3] = [
    ["KEEP THE CHANNEL DARK", "A BOAT IS OUT THERE", "THAT IS MY SON", "THEN SHOW THE REEF", "THE REEF WILL TAKE HIM", "YOUR DARKNESS WILL TOO", "I AM TURNING", "I AM HERE"],
    ["PUT OUT YOUR LIGHT", "SOMEONE IS CROSSING", "THE BOAT IS MINE", "THEN GIVE IT A SHORE", "THE ROCKS KNOW MY NAME", "THE BOAT DOES NOT", "I WILL TURN", "I WILL STAY"],
    ["LEAVE THE WATER BLACK", "I CAN HEAR AN ENGINE", "SHE IS ON THAT BOAT", "THEN LET HER SEE US", "THE STORM WILL FIND HER", "SHE NEEDS BOTH LIGHTS", "I AM TURNING", "SO AM I"],
];

struct Speech {
    turn: usize,
    completed: usize,
    pulse: bool,
    done: bool,
}

fn morse(ch: char) -> &'static [u8] {
    match ch {
        'A' => b".-", 'B' => b"-...", 'C' => b"-.-.", 'D' => b"-..", 'E' => b".",
        'F' => b"..-.", 'G' => b"--.", 'H' => b"....", 'I' => b"..", 'J' => b".---",
        'K' => b"-.-", 'L' => b".-..", 'M' => b"--", 'N' => b"-.", 'O' => b"---",
        'P' => b".--.", 'Q' => b"--.-", 'R' => b".-.", 'S' => b"...", 'T' => b"-",
        'U' => b"..-", 'V' => b"...-", 'W' => b".--", 'X' => b"-..-", 'Y' => b"-.--",
        'Z' => b"--..", _ => b"",
    }
}

fn speech_at(dialogue: [&str; 8], seconds: f32) -> Speech {
    let mut units = seconds / 0.105;
    for (turn, phrase) in dialogue.iter().enumerate() {
        let mut completed = 0;
        let chars = phrase.as_bytes();
        for (index, ch) in chars.iter().enumerate() {
            if *ch == b' ' {
                completed += 1;
                continue;
            }
            let marks = morse(*ch as char);
            for (i, mark) in marks.iter().enumerate() {
                let duration = if *mark == b'-' { 3.0 } else { 1.0 };
                if units < duration { return Speech { turn, completed, pulse: true, done: false }; }
                units -= duration;
                if i + 1 < marks.len() {
                    if units < 1.0 { return Speech { turn, completed, pulse: false, done: false }; }
                    units -= 1.0;
                }
            }
            completed += 1;
            let pause = if chars.get(index + 1) == Some(&b' ') { 7.0 }
                else if index + 1 < chars.len() { 3.0 } else { 0.0 };
            if units < pause { return Speech { turn, completed, pulse: false, done: false }; }
            units -= pause;
        }
        if units < 12.0 { return Speech { turn, completed, pulse: false, done: false }; }
        units -= 12.0;
    }
    Speech { turn: 7, completed: dialogue[7].len(), pulse: false, done: true }
}

#[derive(Clone, Copy)]
struct Colors {
    dark: Color,
    stone: Color,
    lamp: Color,
    ink: Color,
}

impl Colors {
    fn new(palette: &[Color; 5]) -> Self {
        Self {
            dark: rgb(3, 8, 18),
            stone: lerp_color(darken(palette[1], 40), palette[4], 0.35),
            lamp: palette[3],
            ink: palette[4],
        }
    }
}

fn put(grid: &mut Grid, x: i32, y: i32, ch: char, fg: Color) {
    if y >= 0 && (y as usize) < grid.len() && x >= 0 && (x as usize) < grid[y as usize].len() {
        let bg = grid[y as usize][x as usize].bg;
        grid[y as usize][x as usize] = Cell::with_bg(ch, fg, bg);
    }
}

fn night(grid: &mut Grid, w: usize, h: usize, horizon: usize, colors: Colors) {
    for y in 0..h {
        for x in 0..w {
            let bg = if y < horizon { colors.dark } else { rgb(2, 12, 23) };
            grid[y][x] = Cell::with_bg(' ', colors.stone, bg);
        }
    }
    for x in 0..w { put(grid, x as i32, horizon as i32, '_', darken(colors.stone, 38)); }
    let cloud = darken(colors.stone, 48);
    for x in 2..w.saturating_sub(2) {
        let ridge = 2.5 + (x as f32 * 0.075).sin() * 1.2 + (x as f32 * 0.19).cos() * 0.5;
        if (x % 7) < 4 { put(grid, x as i32, ridge.round() as i32, '_', cloud); }
    }
}

fn beam_field(light: &mut [f32], w: usize, h: usize, horizon: usize, boat_x: i32, boat_y: i32, time: f32, speech: &Speech, knobs: Knobs) {
    let lamp_y = (horizon as f32 * 0.29) as i32 + 2;
    let sway = time * 0.48 * knobs.sweep;
    for side in 0..2 {
        let source_x = if side == 0 { w as f32 * 0.1 } else { w as f32 * 0.9 };
        let target_x = if speech.turn >= 6 || speech.done {
            boat_x as f32
        } else if side == 0 {
            w as f32 * 0.64
        } else {
            w as f32 * 0.36
        };
        let target_y = if speech.turn >= 6 || speech.done {
            boat_y as f32
        } else {
            h as f32 * (0.60 + 0.13 * (sway + side as f32 * 2.4).sin())
        };
        let dx = target_x - source_x;
        let dy = (target_y - lamp_y as f32) * 2.0;
        let len = (dx * dx + dy * dy).sqrt().max(1.0);
        let power = (if speech.done { 1.0 }
            else if speech.turn % 2 == side && speech.pulse { 1.0 }
            else { 0.11 }) * knobs.glow;
        for y in 0..h.saturating_sub(3) {
            for x in 0..w {
                let px = x as f32 - source_x;
                let py = (y as f32 - lamp_y as f32) * 2.0;
                let along = (px * dx + py * dy) / len;
                if along <= 0.0 || along >= len * 1.05 { continue; }
                let across = (px * dy - py * dx).abs() / len;
                let radius = (0.9 + along * 0.115) * knobs.beam;
                if across >= radius { continue; }
                let fill = 1.0 - across / radius;
                let strength = fill * fill * (1.0 - along / len * 0.28) * power;
                let index = y * w + x;
                light[index] = (light[index] + strength).min(1.0);
            }
        }
    }
}

fn weather(grid: &mut Grid, light: &[f32], w: usize, _h: usize, horizon: usize, time: f32, seed: u64, storm: f32, flash: bool, colors: Colors) {
    let drift = (time * (2.0 + storm)).floor() as i32;
    for y in 2..horizon {
        for x in 0..w {
            let lit = if flash { 1.0 } else { light[y * w + x] };
            if lit < 0.09 { continue; }
            let noise = pp_hash2(x as i32 + drift, y as i32 * 3 - drift, seed);
            let rain = (storm * 0.09 * lit).min(0.24);
            if noise < rain {
                let ch = if storm > 0.7 { '/' } else { '|' };
                put(grid, x as i32, y as i32, ch, lerp_color(colors.stone, colors.ink, lit));
            } else if noise < rain + lit * 0.15 {
                put(grid, x as i32, y as i32, '.', darken(colors.lamp, 90));
            }
        }
    }
}

fn sea(grid: &mut Grid, light: &[f32], w: usize, h: usize, horizon: usize, time: f32, storm: f32, flash: bool, colors: Colors) {
    for y in horizon + 1..h.saturating_sub(3) {
        for x in 0..w {
            let lit = if flash { 1.0 } else { light[y * w + x] };
            if lit < 0.16 { continue; }
            let wave = (x as f32 * 0.37 + y as f32 * 1.68 + time * (0.65 + storm * 0.3)).sin();
            let fold = (x as f32 * 0.11 - y as f32 * 0.61 + time * 0.27).cos();
            if wave + fold * 0.27 > 0.84 {
                let fg = lerp_color(colors.stone, colors.ink, (lit * 0.82).min(1.0));
                put(grid, x as i32, y as i32, if lit > 0.55 { '~' } else { '_' }, fg);
            }
        }
    }
}

fn boat(grid: &mut Grid, light: &[f32], w: usize, h: usize, x: i32, y: i32, time: f32, colors: Colors) {
    let lit = if x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h {
        light[y as usize * w + x as usize]
    } else { 0.0 };
    let hull = lerp_color(colors.stone, colors.ink, (lit + 0.22).min(1.0));
    put(grid, x, y - 3, '|', hull);
    put(grid, x - 1, y - 2, '/', hull);
    put(grid, x, y - 2, '|', hull);
    put(grid, x + 1, y - 2, '\\', hull);
    put(grid, x, y - 1, '|', hull);
    for (i, ch) in "\\_____/".chars().enumerate() {
        put(grid, x - 3 + i as i32, y, ch, hull);
    }
    if lit > 0.27 {
        let wake = if (time * 3.0).sin() > 0.0 { '~' } else { '_' };
        put(grid, x - 5, y + 1, wake, colors.stone);
        put(grid, x + 5, y + 1, wake, colors.stone);
    }
}

fn invert(color: Color) -> Color {
    match color {
        Color::Rgb { r, g, b } => rgb(255 - r, 255 - g, 255 - b),
        other => other,
    }
}

fn negative(grid: &mut Grid, w: usize, h: usize) {
    for row in grid.iter_mut().take(h) {
        for cell in row.iter_mut().take(w) {
            cell.fg = invert(cell.fg);
            cell.bg = invert(cell.bg);
        }
    }
}

fn lightning(grid: &mut Grid, w: usize, h: usize, horizon: usize, seed: u64, colors: Colors) {
    let mut x = (w as f32 * (0.46 + (seed % 11) as f32 * 0.008)) as i32;
    for y in 1..horizon.saturating_sub(2) {
        let step = match y % 4 { 0 => -2, 1 => 1, 2 => -1, _ => 2 };
        x += step;
        put(grid, x, y as i32, if step < 0 { '/' } else { '\\' }, colors.ink);
        if y % 5 == 0 { put(grid, x + 1, y as i32, '_', colors.ink); }
    }
    negative(grid, w, h);
}

fn towers(grid: &mut Grid, w: usize, horizon: usize, speech: &Speech, colors: Colors, glow: f32) {
    for side in 0..2 {
        let x = if side == 0 { (w as f32 * 0.1) as i32 } else { (w as f32 * 0.9) as i32 };
        let top = (horizon as f32 * 0.29) as i32;
        let lamp_y = top + 2;
        let lit = speech.pulse && speech.turn % 2 == side || speech.done;
        put(grid, x, top - 1, '^', colors.stone);
        put(grid, x - 1, top, '/', colors.stone);
        put(grid, x, top, '_', colors.stone);
        put(grid, x + 1, top, '\\', colors.stone);
        put(grid, x - 1, lamp_y, '|', colors.stone);
        put(grid, x, lamp_y, if lit { '*' } else { '.' }, if lit { colors.lamp } else { colors.stone });
        put(grid, x + 1, lamp_y, '|', colors.stone);
        for y in lamp_y + 1..horizon as i32 {
            let edge = if y > horizon as i32 - 3 { 2 } else { 1 };
            put(grid, x - edge, y, '/', colors.stone);
            put(grid, x + edge, y, '\\', colors.stone);
            if y % 3 == 0 { put(grid, x, y, '=', colors.stone); }
        }
        let halo = if glow > 1.2 && lit { ':' } else { ' ' };
        put(grid, x - 2, lamp_y, halo, colors.lamp);
        put(grid, x + 2, lamp_y, halo, colors.lamp);
    }
}

fn caption(grid: &mut Grid, w: usize, h: usize, speech: &Speech, dialogue: [&str; 8], colors: Colors) {
    let y = h as i32 - 2;
    let side = if speech.turn % 2 == 0 { "WEST" } else { "EAST" };
    let prefix = if speech.done { "BOTH > " } else if side == "WEST" { "WEST > " } else { "EAST > " };
    let text = dialogue[speech.turn].chars().take(speech.completed);
    let mut x = ((w.saturating_sub(46)) / 2) as i32;
    for ch in prefix.chars().chain(text).chain(std::iter::once(if speech.done { ' ' } else { '_' })) {
        put(grid, x, y, ch, colors.ink);
        x += 1;
        if x >= w as i32 - 2 { break; }
    }
    let left = "W E S T";
    let right = "E A S T";
    for (offset, ch) in left.chars().enumerate() {
        put(grid, (w as f32 * 0.1) as i32 - 3 + offset as i32, 1, ch, colors.stone);
    }
    for (offset, ch) in right.chars().enumerate() {
        put(grid, (w as f32 * 0.9) as i32 - 3 + offset as i32, 1, ch, colors.stone);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    fn rendered(seed: u64, time: f32, w: usize, h: usize) -> Grid {
        let palette = crate::color::named_theme("deep").unwrap();
        let mut grid = vec![vec![Cell::blank(); w]; h];
        let mut rng = StdRng::seed_from_u64(seed);
        MODE.render(&mut ModeFrame {
            grid: &mut grid, width: w, height: h, seed, palette: &palette,
            rng: &mut rng, time, args: &[], param_values: None,
        });
        grid
    }

    fn chars(grid: &Grid) -> String {
        grid.iter()
            .map(|row| row.iter().map(|cell| cell.ch).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn opening_frame() {
        insta::assert_snapshot!("semaphore_80x24_t0", chars(&rendered(42, 0.0, 80, 24)));
    }

    #[test]
    fn sixth_second() {
        insta::assert_snapshot!("semaphore_80x24_t6", chars(&rendered(42, 6.0, 80, 24)));
    }

    #[test]
    fn frame_inputs_control_the_picture() {
        let opening = rendered(42, 0.0, 80, 24);
        assert_eq!(opening, rendered(42, 0.0, 80, 24));
        assert_ne!(rendered(42, 6.0, 80, 24), rendered(43, 6.0, 80, 24));
        assert_ne!(opening, rendered(42, 6.0, 80, 24));
    }

    #[cfg(not(debug_assertions))]
    #[test]
    fn release_frame_cost_under_six_ms() {
        let start = std::time::Instant::now();
        for i in 0..32 {
            let _ = rendered(42, i as f32 * 0.25, 200, 60);
        }
        let mean_ms = start.elapsed().as_secs_f64() * 1000.0 / 32.0;
        assert!(mean_ms < 6.0, "mean frame cost: {mean_ms:.3} ms");
    }
}
