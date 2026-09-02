# Brief: one new rendering mode, subject open

You are a fresh session in `ascii-renderer`, a Rust CLI that renders animated ASCII/Unicode scenes
into a `Grid` of colored cells. Your job: add one new mode. The subject is yours. This file gives you
only the mechanics of the repo. It says nothing about what to draw on purpose.

## Read order, and what not to read

1. Read this file to the end.
2. Before opening any source file, write `briefs/<your-mode>.md` with: the subject in one sentence,
   what moves and why, the five glyph families you intend to use, and the three knobs a viewer would
   want. Commit that file first. This locks your idea in before the codebase can pull it anywhere.
3. Then read only: `src/types.rs` (Cell, Grid), `src/color.rs` (palette, hsl_to_rgb, darken,
   lighten, lerp_color), `src/opts.rs` lines 30 to 45 (`param_f32`), and `src/astrolabe.rs` as the
   one example of a self-contained mode with a `cli_*` handler and a `draw_*` function.
4. Do not read `chat_log/`, `src/lifetree*.rs`, `src/mahoraga*.rs`, `src/arboretum.rs`,
   `src/sauron.rs`, or any snapshot in `src/snapshots/` and `tests/snapshots/`. Those carry the
   previous sessions' taste. You are being asked for yours.

Subjects already taken, so pick something else: trees of any kind, eyes, fruit, hyperbolic or
non-Euclidean geometry, a scene split into two halves, seams, motes, particles rising, a heartbeat
pulse, seasons, gusts, ghosts, shrines, giant eyes, astrolabes, murmurations, lanterns, tides,
skylines, koi, jellyfish, circuits, quilts, nebulae, kintsugi, constellations, cities, forests.
If the first idea that arrives is a plant, an organism, a landscape, or a machine, take the second idea.

## Repo mechanics

Build and run:
```bash
cargo build --release
./target/release/ascii-renderer <seed> <mode> [theme] [positional knobs...]
ASCII_GRID_W=110 ASCII_GRID_H=36 ASCII_T=<seconds> ./target/release/ascii-renderer 42 <mode> moss | sed 's/\x1b\[[0-9;]*m//g'
```
`ASCII_T` is the animation clock. Themes: ember terracotta sakura arctic deep moss bone silver neon
nerv mitla. Palette is `[bg, primary, secondary, accent, text]` as `crossterm::style::Color::Rgb`.

Cells: `Cell::new(ch, fg)` or `Cell::with_bg(ch, fg, bg)`. `Grid` is `Vec<Vec<Cell>>`, row major,
`grid[y][x]`. Terminal cells are about twice as tall as wide; a circle needs x scaled by 2.

Sandbox note: the release link step runs a custom linker script and gets SIGTERM under the Claude
Code sandbox. If `cargo build` dies at the final `ascii-renderer(bin)` step with signal 15, run the
build and test commands with the sandbox off. `cargo check` works either way.

## The mode contract

New module `src/<name>.rs`, self-contained, exposing:
```rust
pub(crate) struct <Name>Knobs { /* ... */ }
impl <Name>Knobs { pub(crate) fn from_env() -> Self }   // each knob: crate::opts::param_f32("KEY", default)
pub(crate) fn draw_<name>(grid: &mut Grid, w: usize, h: usize, seed: u64, palette: &[Color; 5], t: f32, k: &<Name>Knobs)
pub(crate) fn cli_<name>(mut grid: Grid, width: usize, height: usize, seed: u64, palette: [Color; 5], rng: StdRng, t_anim: f32, term_w: u16, term_h: u16, args: &[String], mode: &str, theme_name: &str) -> (Grid, bool)
```
`cli_<name>` parses positional knobs from `args[4..]` (`args[1]` seed, `[2]` mode, `[3]` theme), calls
`draw_<name>`, returns `(grid, false)`.

Wiring, seven append-only touches. Every insertion goes at the END of its list; never splice into the
middle, never reorder:
1. `src/main.rs`: `mod <name>;` after the last `mod` line.
2. `src/cli.rs`: `use crate::<name>::cli_<name>;` next to the other `cli_*` imports; one
   `eprintln!("  <mode>  <one line description> (a=animate) [knob] [knob]")` after the last mode help
   line; one dispatch arm `} else if mode == "<mode>" { let (g, done) = cli_<name>(grid, width, height, seed, palette, rng, t_anim, term_w, term_h, &args, mode, theme_name); grid = g; if done { return; } }`
   as the LAST arm, directly before the final `} else {` that calls `cli_default`.
3. `src/opts.rs`: `"<mode>",` as the last entry of `all_modes` in `run_demo`.
4. `src/registry.rs`: one `ModeForm { names: &["<mode>"], animate: AnimKind::Iterate, params: &[param!("KEY", "label", min, max, default, step), ...] }`
   as the last element of `MODE_FORMS`. Find the static by `pub(crate) static MODE_FORMS` and insert
   before the first `\n];` after it. Keys must equal your `param_f32` keys.
5. `src/morph.rs`: in `iterate_grid_into`, an arm
   `"<mode>" => { let knobs = crate::<name>::<Name>Knobs::from_env(); crate::<name>::draw_<name>(grid, w, h, seed, palette, t, &knobs); true }`
   directly before `_ => false,`. This is what makes the `a` key animate natively.
6. `tests/snapshot_modes.rs`: append two tests using the file's `render(&[...])` helper, one with
   `["42", "<mode>", "moss"]`, one with positional knobs.
7. `CLAUDE.md`: append your mode name to the end of the modes list.

## Hard constraints

- `t == 0.0` renders the same static frame every run. All motion is a pure function of `t`.
  Per-item phases come from a build step seeded from `seed`, never from rng inside the frame.
- Performance: build geometry once into `thread_local! { static CACHE: RefCell<Option<Cached>> }`
  keyed by `(w, h, seed, geometry knobs)`. A frame does no rng and no heap allocation. Add a test
  that renders 200 frames at 200x60 and asserts under 4 ms each, printing the number with
  `eprintln!`. Measure it in release: `cargo test --release <name>::tests::frame_cost -- --nocapture`.
- Glyphs must be width 1. Reuse only chars that already appear in `src/*.rs`. Check each new one:
  `grep -c "<glyph>" src/*.rs`; zero hits means do not use it. A width-2 glyph fails the
  display-width tests in `main.rs`.
- Comments: at most 2 consecutive comment lines anywhere, `///` and `//!` included. A hook rejects
  the file otherwise. Module header is at most two `//!` lines.
- Prose and identifiers: no em dashes; never the words `provenance`, `substrate`, `load-bearing`,
  `regime`; `signal` is a reserved library name, do not use it as a word or identifier.
- Never edit an existing mode. Never remove anything.
- Expose every tunable as a knob (env `ASCII_P_<KEY>` via `param_f32`, and positional args). No
  hardcoded magic that a viewer would want to turn.

## Validation, in this order

```bash
cargo build --release
ASCII_GRID_W=110 ASCII_GRID_H=36 ./target/release/ascii-renderer 42 <mode> moss | sed 's/\x1b\[[0-9;]*m//g'
ASCII_GRID_W=110 ASCII_GRID_H=36 ASCII_T=5 ./target/release/ascii-renderer 42 <mode> moss | sed 's/\x1b\[[0-9;]*m//g'
cargo test          # first run writes *.snap.new for your new snapshots and stops at the first failing target
```
Look at both renders with your own eyes before accepting anything. Then accept only yours:
```bash
for f in src/snapshots/*<name>*.snap.new tests/snapshots/*<mode_underscored>*.snap.new; do [ -f "$f" ] && mv "$f" "${f%.new}"; done
cargo test          # run twice: in-module snapshots land on the first pass, integration ones on the second
```
`git status --short` must show only your files. Any other mode's snapshot changing means you broke
the append-only rule.

## Deliverable

One commit: `feat: <mode> -- <six words>`. Contents: the module, the seven touches, accepted
snapshots, `briefs/<your-mode>.md` updated with the measured frame time and the two render commands.
Report back with: the subject in one sentence, the knob list, the frame time, the commit hash.
