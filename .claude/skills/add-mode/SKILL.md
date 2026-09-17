# Add Mode

Scaffold a new rendering mode in ascii-renderer as a trait mode with snapshot coverage.
Reference implementations: `src/modes/_54_nightglass.rs` (12 knobs, rayon rows, scratch buffer),
`src/modes/_55_rosette.rs` (10 knobs, 5 layers, small).

## One file, one script (the only wiring)

1. Create `src/modes/_NN_<name>.rs` where NN is the next unused number (`ls src/modes | tail -1`).
2. Run `scripts/0_generate_modes.sh`. It rewrites `src/modes/mod.rs` (generated, never hand-edit).
3. Append the mode name at the end of the modes list in `CLAUDE.md`.

Registration through `registered_modes()` feeds everything else with no list edits:
CLI dispatch (`src/cli.rs:613`), `a`-key animation (`src/morph.rs:304`), the demo picker
(`src/opts.rs:712`), help (`src/cli.rs:472`), the perf roster (`src/perf_sweep.rs:526`),
and the layer-timer gate. Never touch `cli.rs`, `main.rs`, `opts.rs`, `registry.rs` `MODE_FORMS`,
`morph.rs` `iterate_grid`, or `perf_sweep.rs` `NATIVE_MODES` for a new mode; that chain is legacy.

## File skeleton

```rust
use crate::_0_profile::measure_layer;
use crate::opts::param_f32;
use crate::registry::{AnimKind, Mode, ModeFrame, Param};
use crate::types::{Cell, Grid};

pub(super) struct Foo;
pub(super) static MODE: Foo = Foo;

const NAME: &str = "foo";
const KNOBS: usize = 3;
const HELP: &str = "foo: one line [knob1] [knob2] [knob3]";

const PARAMS: &[Param] = &[
    param!("KNOB1", "label", min, max, default, step),
    ...
];

impl Mode for Foo {
    fn name(&self) -> &'static str { NAME }
    fn help(&self) -> &'static str { HELP }
    fn animation(&self) -> AnimKind { AnimKind::Iterate }
    fn params(&self) -> &'static [Param] { PARAMS }
    fn render(&self, frame: &mut ModeFrame<'_>) {
        // positional args (i + 4), then frame.param_values, then param_f32 env; clamp; see rosette
        draw(frame, &p);
    }
}
```

`ModeFrame` fields: `grid, width, height, seed, palette: &[Color; 5], rng, time, args, param_values`.
Knob resolution order is fixed: positional CLI arg `args[i + 4]`, then `param_values[i]`
(live demo knobs), then `param_f32(KEY, default)` from env `ASCII_P_<KEY>`; clamp to `[min, max]`.

## Animation

`frame.time` is the clock. `time == 0.0` MUST render the static frame byte-identical (snapshots).
Per-element phases come from a splitmix hash of `(seed, layer, index)` (copy `hash`/`unit` from
rosette), never from `frame.rng`, so the static render and the rng stream stay untouched.

## Layer timers

Wrap each painter in `measure_layer(NAME, "<layer>", || ...)`, 3 to 8 layers covering at least
85 percent of the frame. Rules: `perf/INSTRUMENT.md`. Do it while writing; retrofitting re-indents.
The registry gate `every_registered_mode_that_renders_in_process_has_layer_timers` fails without them.
Bound every painter to the cells it touches (a 3-cell boss scanning the whole grid was 14 percent
of a rosette frame).

## Tests

In-module `#[cfg(test)]` (pattern: rosette `tests`): `MODE.render(&mut ModeFrame { .. })` at 80x24
seed 42, `insta::assert_snapshot!("<name>_80x24", text)`, plus a `t6` snapshot, determinism,
seed sensitivity, time sensitivity, and `frame_cost` at 200x60 (release budget under 6 ms avg).
Integration: append at the END of `tests/snapshot_modes.rs`:
`insta::assert_snapshot!(render(&["42", "<name>", "<theme>"]))`.

`cargo insta` may be missing. Accept after visual inspection: `mv *.snap.new *.snap`.
Snapshots land in `src/modes/snapshots/` and `tests/snapshots/`.

## Perf receipt

```bash
perf/layer_coverage.sh 400 120 3 moss <name>     # width height secs theme filter
perf/knob_sweep.sh <name> 2000 1000 2            # fps table + hotspot table
```
Paste both tables into `briefs/<name>.md`. `perf/results/<name>.md` holds the raw output.

## Rules

- Never modify existing modes. Check the name is free: `grep -rn '"<name>"' src/ | head`.
- Fixed seed for deterministic output. All knobs exposed through `PARAMS`; nothing hardcoded.
- Glyph safety: reuse chars already in `src/sprites.rs`, `src/tree_draw/species*.rs`, or another
  mode; exotic glyphs risk the unicode-width assertions in `main.rs`.
- `hsl_to_rgb` takes f64; knob values are f32, cast.
- Max 2 consecutive comment lines (`///` and `//!` count); the Stop hook rejects more.
- No em dashes. Banned identifiers: `provenance`, `substrate`, `load_bearing`, `regime`.
- Grids at or above 20_480 cells: run row passes on rayon (`par_chunks_mut(w)`), pattern in rosette.
