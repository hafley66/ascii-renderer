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
- Snapshot files live in `src/snapshots/` (insta default)
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

party, soup, tree, trees, forest, forest2, forest3, forest4, aztec, fret, flowers, fruits, masks, shapes,
tiles, tiles-rand, tiles-skew, mondrian, mondrian2, bsp, layout, md, terrain, flow,
noise, ca, ca-layout, stem, scene-walk, scene-walk-2, scene-walk-3, world,
kintsugi, constellation, strata, circuit, quilt, patchwalk, trees8, trees9, boles4, boles5

## Architecture

Numbered-file convention per user prefs. Key modules:
- `main.rs` -- CLI dispatch, mode wiring
- `types.rs` -- Cell, Grid, Rect
- `color.rs` -- palette, darken/lighten/shift
- `fills.rs` -- tile/noise pattern renderers
- `scene.rs` -- FillGen enum, Layer, Scene, render_scene
- `sprites.rs` -- trees, flowers, fruits, masks, aztec diamonds, frets
- `walker.rs` -- walk modes (party, soup, stem, scene-walk), atmosphere, path styles
- `mondrian.rs` -- BSP layout + mondrian grid
- `automata.rs` -- cellular automata

## Skills & Agents

- **add-mode** skill: Scaffold new rendering modes with snapshot tests
- **add-sprite-algo** skill: Design algorithmic sprites (turtle walks, L-systems, fractals)
- **session-digest** agent: Analyze chat_log/ for momentum, open threads, patterns

## Ground rules

- Never remove or break existing modes. Only add.
- Expose tuning knobs as CLI args, don't hardcode.
- Commit at each milestone for rewind points.
