#![allow(warnings)]

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
use crate::automata; use crate::avant; use crate::biomes; use crate::borders; use crate::color; use crate::content; use crate::fills; use crate::layout; use crate::markdown; use crate::mondrian; use crate::render; use crate::scene; use crate::sprites; use crate::tree_draw; use crate::types; use crate::walker;
use crate::cli::*;
use crate::gridio::*;
use crate::ink::*;
use crate::modes_creatures::*;
use crate::modes_geo::*;
use crate::modes_sky::*;
use crate::modes_tree::*;
use crate::morph::*;
use crate::pp::*;
use crate::registry::*;
use crate::warps::*;

/// Read a runtime knob set by the demo panel (env ASCII_P_<KEY>), or `default`.
pub(crate) fn param_f32(key: &str, default: f32) -> f32 {
    std::env::var(format!("ASCII_P_{}", key))
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

// ============================================================================
// Knob persistence. The demo panel's tuned knob values are saved to a small TSV
// (`mode<TAB>KEY<TAB>value` per line) under the user's config dir, so they
// survive across runs. run_demo and morph_session both load on entry and write
// on every change.
// ============================================================================


/// Path to the persisted-options file (`~/.config/ascii-renderer/options.tsv`),
/// or None if HOME is unset.
pub(crate) fn options_path() -> Option<std::path::PathBuf> {
    let home = std::env::var_os("HOME")?;
    let mut p = std::path::PathBuf::from(home);
    p.push(".config");
    p.push("ascii-renderer");
    p.push("options.tsv");
    Some(p)
}

pub(crate) type OptMap = std::collections::HashMap<String, std::collections::HashMap<String, f32>>;


/// Parse the TSV at `path` into mode -> (KEY -> value). Missing/unreadable -> empty.
pub(crate) fn load_options_from(path: &std::path::Path) -> OptMap {
    let mut map: OptMap = std::collections::HashMap::new();
    if let Ok(s) = std::fs::read_to_string(path) {
        for line in s.lines() {
            let mut it = line.splitn(3, '\t');
            if let (Some(m), Some(k), Some(v)) = (it.next(), it.next(), it.next()) {
                if let Ok(val) = v.parse::<f32>() {
                    map.entry(m.to_string()).or_default().insert(k.to_string(), val);
                }
            }
        }
    }
    map
}


/// Write the whole option map to `path` (creates parent dirs as needed).
pub(crate) fn save_options_to(path: &std::path::Path, map: &OptMap) {
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let mut s = String::new();
    for (m, ks) in map {
        for (k, v) in ks {
            s.push_str(&format!("{}\t{}\t{}\n", m, k, v));
        }
    }
    let _ = std::fs::write(path, s);
}


/// Load all saved knob values from the default options file.
pub(crate) fn load_options() -> OptMap {
    match options_path() {
        Some(p) => load_options_from(&p),
        None => std::collections::HashMap::new(),
    }
}


/// Persist the whole option map to the default options file.
pub(crate) fn save_options(map: &OptMap) {
    if let Some(p) = options_path() {
        save_options_to(&p, map);
    }
}


/// Initial knob values for `mode`: saved value (clamped to range) if present,
/// else the param default.
pub(crate) fn pvals_for(
    spec: &ModeSpec,
    mode: &str,
    saved: &std::collections::HashMap<String, std::collections::HashMap<String, f32>>,
) -> Vec<f32> {
    spec.params
        .iter()
        .map(|p| {
            saved
                .get(mode)
                .and_then(|m| m.get(p.key))
                .map(|v| v.clamp(p.min, p.max))
                .unwrap_or(p.default)
        })
        .collect()
}


/// A deterministic-but-random value for knob `p`, hashed from (seed, key) into the
/// param's range and snapped to its step. Stable for a given seed; re-rolls when
/// the seed changes -- "controlled randomness" rather than per-frame jitter.
pub(crate) fn rand_knob(seed: u64, p: &Param) -> f32 {
    let mut h = seed ^ 0x9E37_79B9_7F4A_7C15;
    for b in p.key.bytes() {
        h = h.wrapping_mul(0x0000_0100_0000_01B3).wrapping_add(b as u64);
    }
    let u = ((h >> 11) as f64 / (1u64 << 53) as f64) as f32; // 0..1
    let raw = p.min + u * (p.max - p.min);
    let snapped = p.min + ((raw - p.min) / p.step).round() * p.step;
    snapped.clamp(p.min, p.max)
}


/// The values actually pushed to the renderer: the tuned `pvals` (deterministic),
/// or random samples for every knob when `randomize` is on. `roll` is a nonce the
/// UI bumps with left/right to re-roll a fresh random set without changing seed.
pub(crate) fn effective_pvals(spec: &ModeSpec, pvals: &[f32], seed: u64, randomize: bool, roll: u64) -> Vec<f32> {
    if randomize {
        let s = seed ^ roll.wrapping_mul(0x9E37_79B9_7F4A_7C15);
        spec.params.iter().map(|p| rand_knob(s, p)).collect()
    } else {
        pvals.to_vec()
    }
}


/// The global deterministic-vs-random toggle, stored under a reserved pseudo-mode.
pub(crate) fn load_randomize(saved: &OptMap) -> bool {
    saved.get("__global").and_then(|m| m.get("RAND")).copied().unwrap_or(0.0) > 0.5
}

pub(crate) fn store_randomize(saved: &mut OptMap, on: bool) {
    saved
        .entry("__global".to_string())
        .or_default()
        .insert("RAND".to_string(), if on { 1.0 } else { 0.0 });
    save_options(saved);
}


/// Record `mode`'s current knob values into `saved` and flush to disk.
pub(crate) fn store_pvals(
    mode: &str,
    spec: &ModeSpec,
    pvals: &[f32],
    saved: &mut std::collections::HashMap<String, std::collections::HashMap<String, f32>>,
) {
    let m = saved.entry(mode.to_string()).or_default();
    for (p, v) in spec.params.iter().zip(pvals.iter()) {
        m.insert(p.key.to_string(), *v);
    }
    save_options(saved);
}


/// Indices of modes whose name contains `query` (case-insensitive). Empty query
/// matches all, preserving order.
pub(crate) fn demo_filter_modes(all_modes: &[&str], query: &str) -> Vec<usize> {
    let ql = query.to_lowercase();
    all_modes
        .iter()
        .enumerate()
        .filter(|(_, m)| ql.is_empty() || m.to_lowercase().contains(&ql))
        .map(|(i, _)| i)
        .collect()
}


/// Full-screen list+filter picker. Type to filter (substring, case-insensitive),
/// Up/Down to move, Enter to select, Esc to cancel. Returns the chosen index into
/// `all_modes`, or None if cancelled. Caller must have raw mode enabled.
pub(crate) fn demo_pick_mode(all_modes: &[&str], current: usize) -> Option<usize> {
    use crossterm::{
        cursor,
        event::{self, Event, KeyCode, KeyModifiers},
        execute,
        terminal::{self, ClearType},
    };
    use std::io::Write;

    let mut query = String::new();
    let mut sel: usize = current; // index into the filtered list

    loop {
        let filtered = demo_filter_modes(all_modes, &query);
        if sel >= filtered.len() {
            sel = filtered.len().saturating_sub(1);
        }

        let (tw, th) = terminal::size().unwrap_or((80, 45));
        let tw = tw as usize;
        let th = th as usize;
        // `cancel_idx` is a virtual last entry pinned below a divider. sel ranges
        // 0..=cancel_idx; landing on it and pressing enter cancels.
        let cancel_idx = filtered.len();
        if sel > cancel_idx {
            sel = cancel_idx;
        }
        // reserve 2 header rows + 2 rows for the divider and the cancel entry.
        let list_h = th.saturating_sub(5).max(1);
        let anchor = sel.min(filtered.len().saturating_sub(1));
        let offset = if filtered.len() <= list_h || anchor < list_h / 2 {
            0
        } else {
            (anchor - list_h / 2).min(filtered.len().saturating_sub(list_h))
        };

        let query_disp = if query.is_empty() {
            "\u{2026}".to_string()
        } else {
            query.clone()
        };
        let mut buf = String::new();
        buf.push_str(&format!(
            "\x1b[7m \u{1f50d} search \x1b[0m {}\u{2588}  \u{2502}  {}/{} match  \u{2502}  \u{2191}\u{2193} move \u{00b7} type to filter \u{00b7} enter select \u{00b7} esc cancel\r\n\r\n",
            query_disp,
            filtered.len(),
            all_modes.len()
        ));
        for row in 0..list_h {
            let fi = offset + row;
            if fi >= filtered.len() {
                buf.push_str("\r\n");
                continue;
            }
            let name = all_modes[filtered[fi]];
            if fi == sel {
                let pad = tw.saturating_sub(name.chars().count() + 3);
                buf.push_str(&format!("\x1b[7m \u{25b8} {}{} \x1b[0m\r\n", name, " ".repeat(pad)));
            } else {
                buf.push_str(&format!("   {}\r\n", name));
            }
        }
        // divider + pinned cancel entry.
        buf.push_str(&format!("\x1b[90m{}\x1b[0m\r\n", "\u{2500}".repeat(tw.min(40))));
        if sel == cancel_idx {
            let label = "\u{2715} cancel";
            let pad = tw.saturating_sub(label.chars().count() + 3);
            buf.push_str(&format!("\x1b[7m \u{25b8} {}{} \x1b[0m", label, " ".repeat(pad)));
        } else {
            buf.push_str("   \x1b[90m\u{2715} cancel\x1b[0m");
        }

        execute!(
            io::stdout(),
            terminal::Clear(ClearType::All),
            cursor::MoveTo(0, 0)
        )
        .unwrap();
        print!("{}", buf);
        io::stdout().flush().unwrap();

        if let Ok(Event::Key(key)) = event::read() {
            match key.code {
                KeyCode::Esc => return None,
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => return None,
                KeyCode::Enter => {
                    if sel == filtered.len() {
                        return None; // cancel entry
                    }
                    return filtered.get(sel).copied();
                }
                // wrap around top<->bottom; cancel_idx is the bottom-most entry.
                KeyCode::Up => sel = if sel == 0 { cancel_idx } else { sel - 1 },
                KeyCode::Down => sel = if sel >= cancel_idx { 0 } else { sel + 1 },
                KeyCode::Backspace => {
                    query.pop();
                    sel = 0;
                }
                KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    query.push(c);
                    sel = 0;
                }
                _ => {}
            }
        }
    }
}


pub(crate) fn run_demo(initial_seed: u64) {
    use crossterm::{
        cursor,
        event::{self, Event, KeyCode, KeyModifiers},
        execute,
        terminal::{self, ClearType},
    };
    use std::io::Write;
    use std::process::Command;

    let mut all_modes = vec![
        "party",
        "soup",
        "tree",
        "trees",
        "forest",
        "forest2",
        "forest3",
        "forest4",
        "forest5",
        "forest6",
        "forest7",
        "forest8",
        "forest9",
        "aztec",
        "fret",
        "flowers",
        "fruits",
        "masks",
        "shapes",
        "tiles",
        "tiles-rand",
        "tiles-skew",
        "mondrian",
        "mondrian2",
        "bsp",
        "layout",
        "terrain",
        "flow",
        "watershed",
        "noise",
        "ca",
        "stem",
        "scene-walk",
        "scene-walk-2",
        "scene-walk-3",
        "aurora",
        "aura2",
        "solar-system",
        "harbor",
        "labyrinth",
        "eyes",
        "eyes2",
        "fullmetal-eyes",
        "fullmetal-eyes2",
        "fullmetal-alchemist",
        "fullmetal-alchemist2",
        "fa3",
        "fa4",
        "fa5",
        "fa6",
        "spiro",
        "spiro-tile",
        "weave",
        "gears",
        "kaleido",
        "contour",
        "eyes3",
        "rainfall",
        "meadow",
        "world",
        "world2",
        "boles1",
        "boles2",
        "boles3",
        "trunks1",
        "trees1",
        "trees2",
        "trees3",
        "trees4",
        "trees8",
        "trees9",
        "trees10",
        "boles4",
        "boles5",
        "boles6",
        "bushes",
        "kintsugi",
        "constellation",
        "strata",
        "circuit",
        "snakes",
        "quilt",
        "patchwalk",
        "eyes++",
        "fullmetal-eyes++",
        "trees++",
        "forest++",
        "phyllotaxis",
        "moire",
        "nebula",
        "delta",
        "stained",
        "metro",
        "koi",
        "skyline",
        "hive",
        "jelly",
        "jelly2",
        "hypercube",
        "flux",
        "fireworks",
        "rhizome",
        "effigy",
        "dendrite",
        "totem",
        "chimera",
        "murmuration",
        "lanterns",
        "tide",
        "fireflies",
        "ink",
        "meteors",
        "elevator",
        "ferris",
        "arboretum",
        "astrolabe",
        "sauron",
        "mahoraga-2",
        "mahoraga-3",
    ];
    all_modes.extend(registered_modes().iter().map(|(name, _)| name));
    let all_themes: &[&str] = &[
        "",
        "ember",
        "terracotta",
        "sakura",
        "arctic",
        "deep",
        "moss",
        "bone",
        "silver",
        "neon",
        "nerv",
        "mitla",
    ];

    let mut seed = initial_seed;
    let mut mode_idx: usize = 0;
    let mut theme_idx: usize = 0;

    // Options pane state. `spec`/`pvals` mirror the current mode's declared config;
    // they reload whenever the mode changes. When the pane is open, the up/down/
    // left/right keys edit knobs instead of seed/theme.
    let mut pane_open = false;
    let mut last_mode = "";
    let mut spec = mode_spec(all_modes[mode_idx]);
    let mut saved = load_options();
    let mut randomize = load_randomize(&saved);
    let mut roll: u64 = 0; // re-roll nonce for randomize mode
    let mut pvals: Vec<f32> = pvals_for(&spec, all_modes[mode_idx], &saved);
    let mut psel: usize = 0;

    let exe = std::env::current_exe().unwrap();

    terminal::enable_raw_mode().unwrap();
    execute!(io::stdout(), terminal::EnterAlternateScreen).unwrap();

    loop {
        let current_mode = all_modes[mode_idx];
        let current_theme = all_themes[theme_idx];

        // Reload the declared config + saved knob values when the mode changes.
        if current_mode != last_mode {
            spec = mode_spec(current_mode);
            pvals = pvals_for(&spec, current_mode, &saved);
            psel = 0;
            last_mode = current_mode;
        }

        let (tw, th) = terminal::size().unwrap_or((80, 45));
        // When the pane is open, reserve the right columns for it and render the
        // mode narrower (ASCII_GRID_W). Closed -> full-screen, identical to before.
        let pane_w: usize = if pane_open {
            34.min((tw as usize) / 2)
        } else {
            0
        };
        let render_w = (tw as usize).saturating_sub(pane_w);

        // Push the current knob values down to the render subprocess via env. In
        // randomize mode these are per-seed random samples instead of the tuned
        // pvals. Child processes (preview + iterate animator) inherit them.
        // SAFETY: the demo loop is single-threaded.
        let eff = effective_pvals(&spec, &pvals, seed, randomize, roll);
        for (p, v) in spec.params.iter().zip(eff.iter()) {
            unsafe { std::env::set_var(format!("ASCII_P_{}", p.key), format!("{}", v)) };
        }

        // Disable raw mode so child process writes normal line endings
        terminal::disable_raw_mode().unwrap();
        execute!(
            io::stdout(),
            terminal::Clear(ClearType::All),
            cursor::MoveTo(0, 0)
        )
        .unwrap();

        let mut cmd = Command::new(&exe);
        cmd.arg(seed.to_string()).arg(current_mode);
        if !current_theme.is_empty() {
            cmd.arg(current_theme);
        }
        if pane_open {
            cmd.env("ASCII_GRID_W", render_w.to_string());
            cmd.env("ASCII_GRID_H", th.saturating_sub(1).to_string());
        } else {
            // SAFETY: single-threaded demo loop.
            unsafe {
                std::env::remove_var("ASCII_GRID_W");
                std::env::remove_var("ASCII_GRID_H");
            }
        }
        let _ = cmd.status();

        // Re-enable raw mode for keyboard input
        terminal::enable_raw_mode().unwrap();

        if pane_open {
            draw_options_pane(
                render_w, th, current_mode, &spec, &eff, psel, seed, current_theme, randomize,
            );
        }

        execute!(io::stdout(), cursor::MoveTo(0, th.saturating_sub(1))).unwrap();
        let theme_label = if current_theme.is_empty() {
            "auto"
        } else {
            current_theme
        };
        let knob_tag = if randomize { "knobs:RANDOM " } else { "" };
        let lr_hint = if randomize { "\u{2190}\u{2192}=reroll" } else { "\u{2190}\u{2192}=adjust" };
        let status = if pane_open {
            format!(
                " {} | {}o=close opts  \u{2191}\u{2193}=select  {}  r=reset  g=rand-knobs  a=animate  q=quit ",
                current_mode, knob_tag, lr_hint
            )
        } else {
            format!(
                " {} | seed:{} | theme:{} | {}/=find  o=opts  g=rand  a=animate  f/j=prev/next  \u{2191}\u{2193}=seed  \u{2190}\u{2192}=theme  enter=reseed  q=quit ",
                current_mode, seed, theme_label, knob_tag
            )
        };
        // Pad to terminal width, inverse video (char-safe truncation)
        let status_w = status.chars().count();
        let padded: String = if status_w < tw as usize {
            format!("{}{}", status, " ".repeat(tw as usize - status_w))
        } else {
            status.chars().take(tw as usize).collect()
        };
        print!("\x1b[7m{}\x1b[0m", padded);
        io::stdout().flush().unwrap();

        let has_params = !spec.params.is_empty();
        if let Ok(Event::Key(key)) = event::read() {
            match key.code {
                KeyCode::Char('q') => break,
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => break,
                KeyCode::Char('o') => pane_open = !pane_open,
                KeyCode::Char('g') => {
                    randomize = !randomize;
                    store_randomize(&mut saved, randomize);
                }
                KeyCode::Char('j') => mode_idx = (mode_idx + 1) % all_modes.len(),
                KeyCode::Char('f') => mode_idx = (mode_idx + all_modes.len() - 1) % all_modes.len(),
                KeyCode::Char('/') | KeyCode::Char('m') => {
                    if let Some(idx) = demo_pick_mode(&all_modes, mode_idx) {
                        mode_idx = idx;
                    }
                }
                KeyCode::Char('r') if pane_open && has_params => {
                    pvals[psel] = spec.params[psel].default;
                    store_pvals(current_mode, &spec, &pvals, &mut saved);
                }
                KeyCode::Char('a') => {
                    // Animate via the declared strategy. Knob env is already set, so
                    // the iterate subprocess inherits the tuned values.
                    morph_session(
                        current_mode,
                        seed,
                        current_mode,
                        seed.wrapping_add(1),
                        anim_strat(spec.animate),
                        current_theme,
                    );
                }
                KeyCode::Up => {
                    if pane_open && has_params {
                        psel = (psel + spec.params.len() - 1) % spec.params.len();
                    } else {
                        seed = seed.wrapping_add(1);
                    }
                }
                KeyCode::Down => {
                    if pane_open && has_params {
                        psel = (psel + 1) % spec.params.len();
                    } else {
                        seed = seed.wrapping_sub(1);
                    }
                }
                KeyCode::Right => {
                    if pane_open && has_params && randomize {
                        roll = roll.wrapping_add(1); // re-roll the random set
                    } else if pane_open && has_params {
                        let p = &spec.params[psel];
                        pvals[psel] = (pvals[psel] + p.step).min(p.max);
                        store_pvals(current_mode, &spec, &pvals, &mut saved);
                    } else {
                        theme_idx = (theme_idx + 1) % all_themes.len();
                    }
                }
                KeyCode::Left => {
                    if pane_open && has_params && randomize {
                        roll = roll.wrapping_sub(1);
                    } else if pane_open && has_params {
                        let p = &spec.params[psel];
                        pvals[psel] = (pvals[psel] - p.step).max(p.min);
                        store_pvals(current_mode, &spec, &pvals, &mut saved);
                    } else {
                        theme_idx = (theme_idx + all_themes.len() - 1) % all_themes.len();
                    }
                }
                KeyCode::Enter => {
                    seed = rand::rng().random_range(0..10000u64);
                }
                _ => {}
            }
        }
    }

    execute!(io::stdout(), terminal::LeaveAlternateScreen).unwrap();
    terminal::disable_raw_mode().unwrap();
}
