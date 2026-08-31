# ascii-renderer

## Build & Run

```bash
cargo build
cargo run -- [seed] [mode] [theme] [mode-args...]
```

## Testing

Visual output is the product. Use `insta` snapshot tests to lock down mode output so changes don't silently break things.

### How to add a snapshot test

1. Render to a `Grid` with a fixed seed (deterministic output)
2. Flatten the grid to a plain string (chars only, no ANSI -- strip color)
3. `insta::assert_snapshot!("mode_name_seed", output)`

### Running tests

```bash
cargo test                    # run all, fail on snapshot mismatch
cargo insta test              # run + review new/changed snapshots interactively
cargo insta review            # review pending snapshot changes
cargo insta accept            # accept all pending changes (use after visual review)
```

### Rules

- Every mode gets at least one snapshot test with a fixed seed
- When adding a new mode: add a snapshot test before committing
- When modifying a mode: run `cargo test` first. If snapshots break, visually verify the new output before accepting
- Never `cargo insta accept` blindly -- the whole point is to catch unintended visual regressions
- Snapshot files live next to their test file: `src/snapshots/` for unit tests, `tests/snapshots/` for integration tests
- Test grid size should be small enough to produce readable snapshots (e.g. 80x24) but large enough to exercise the mode

### Grid-to-string helper

```rust
fn grid_to_string(grid: &Grid) -> String {
    grid.iter()
        .map(|row| row.iter().map(|c| c.ch).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}
```

## Modes

Generated-registry mode names are declared by their `Mode::name` implementations. Query the current code instead of maintaining another copied list:

```bash
rg -n 'fn name\(&self\)|static MODE|impl Mode for' src/modes
```

## Architecture

Numbered-file convention per user prefs. Existing Rust modules predate that convention. New standalone modes use `src/modes/_N_name.rs`; the leading underscore makes the numeric prefix a stable Rust module identifier.

Key modules:

- `main.rs` -- module declarations, imports, thin binary entry, legacy unit tests
- `modes/_N_name.rs` -- file-owned generated-registry modes, parameters, renderers, and tests
- `modes/mod.rs` -- generated ordinary module declarations and registration calls
- `scripts/0_generate_modes.sh`, `scripts/1_dev.sh` -- registry generator and development watcher
- `cli.rs` -- CLI parsing and generic registered-mode dispatch plus legacy dispatch
- `cli_basic.rs`, `cli_scenes.rs`, `cli_forest.rs`, `cli_catalog.rs`, `cli_fa.rs`, `cli_city.rs` -- mode-family CLI handlers
- `registry.rs` -- `Mode`, `ModeFrame`, global `ModeRegistry`, legacy parameter forms, and animation kinds
- `opts.rs` -- demo browser, registered mode enumeration, and persisted live parameter values
- `morph.rs` -- morphing, generic registered-mode frame iteration, legacy iteration, and subprocess frame fallback
- Native animation currently recreates `Grid` and `StdRng` for every time value; no per-mode simulation state persists between frames
- `gridio.rs` -- serialized grid I/O and final output selection
- `render.rs` -- plain and ANSI grid rendering
- `types.rs` -- Cell, Grid, Rect
- `color.rs` -- palette, darken/lighten/shift
- `fills.rs` -- tile/noise pattern renderers
- `scene.rs` -- FillGen enum, Layer, Scene, render_scene
- `sprites.rs`, `sprites/` -- generic sprites, pen-grown trees, and flora
- `tree_draw.rs`, `tree_draw/` -- trait-based tree species, boles, packing, and scene walks
- `walker.rs`, `walker/` -- walk modes, atmosphere, paths, and scene construction
- `mondrian.rs` -- BSP layout + mondrian grid
- `automata.rs` -- cellular automata
- `avant.rs` -- avant-garde tree/face algorithmic modes
- `modes_geo.rs`, `modes_sky.rs`, `modes_tree.rs`, `modes_creatures.rs` -- renderer families
- `arboretum.rs`, `astrolabe.rs`, `sauron.rs`, `ink.rs` -- standalone themed systems

## Skills & Agents

- **add-mode** skill: Add file-owned generated-registry modes with snapshot tests
- **add-animation** skill: Add deterministic time motion, native playback, and live knobs
- **add-sprite-algo** skill: Add procedural sprites through the current tree, pen, flora, or generic sprite subsystem
- **session-digest** agent: Analyze chat_log/ for momentum, open threads, patterns

## Ground rules

- Never remove or break existing modes. Only add.
- A new standalone mode requires one `src/modes/_N_name.rs` file and fixed-seed snapshots colocated under `src/modes/snapshots/`.
- Run `scripts/0_generate_modes.sh` after adding or renaming a mode file. Never edit generated `src/modes/mod.rs` directly.
- `scripts/1_dev.sh` runs the generator watcher with `cargo run`.
- Do not add generated-registry mode branches to `src/main.rs`, `src/cli.rs`, `src/opts.rs`, `src/registry.rs`, or `src/morph.rs`.
- Expose tuning knobs through the file-owned `Mode::params` declaration. Keep registry defaults equal to renderer fallbacks.
- Commit at each milestone for rewind points.
