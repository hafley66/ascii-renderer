use crossterm::style::Color;

use crate::_0_profile::measure_layer;
use crate::color::{darken, rgb};
use crate::opts::param_f32;
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
        let w = frame.width.min(frame.grid.first().map_or(0, Vec::len));
        let h = frame.height.min(frame.grid.len());
        if w < 32 || h < 12 { return; }
        let time = frame.time.max(0.0);
        let story = (frame.seed % DIALOGUES.len() as u64) as usize;
        let speech = speech_at(DIALOGUES[story], time * knobs.signal);
        let horizon = (h as f32 * 0.62) as usize;
        let colors = Colors::new(frame.palette);
        measure_layer(NAME, "night", || night(frame.grid, w, h, horizon, colors));
        measure_layer(NAME, "towers", || towers(frame.grid, w, horizon, &speech, colors, knobs.glow));
        measure_layer(NAME, "captions", || caption(frame.grid, w, h, &speech, DIALOGUES[story], colors));
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
        for ch in phrase.chars() {
            if ch == ' ' {
                if units < 7.0 { return Speech { turn, completed, pulse: false, done: false }; }
                units -= 7.0;
                completed += 1;
                continue;
            }
            let marks = morse(ch);
            for (i, mark) in marks.iter().enumerate() {
                let duration = if *mark == b'-' { 3.0 } else { 1.0 };
                if units < duration { return Speech { turn, completed, pulse: true, done: false }; }
                units -= duration;
                if i + 1 < marks.len() {
                    if units < 1.0 { return Speech { turn, completed, pulse: false, done: false }; }
                    units -= 1.0;
                }
            }
            if units < 3.0 { return Speech { turn, completed, pulse: false, done: false }; }
            units -= 3.0;
            completed += 1;
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
        Self { dark: rgb(3, 8, 18), stone: darken(palette[1], 70), lamp: palette[3], ink: palette[4] }
    }
}

fn put(grid: &mut Grid, x: i32, y: i32, ch: char, fg: Color) {
    if y >= 0 && (y as usize) < grid.len() && x >= 0 && (x as usize) < grid[y as usize].len() {
        grid[y as usize][x as usize] = Cell::with_bg(ch, fg, grid[y as usize][x as usize].bg);
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
    let prefix = format!("{} > ", side);
    let text = dialogue[speech.turn].chars().take(speech.completed);
    let mut x = ((w.saturating_sub(46)) / 2) as i32;
    for ch in prefix.chars().chain(text).chain(std::iter::once('_')) {
        put(grid, x, y, ch, colors.ink);
        x += 1;
        if x >= w as i32 - 2 { break; }
    }
}
