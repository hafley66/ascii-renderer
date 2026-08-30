# Mode lifecycle and extension map

## Runtime sequence

```text
CLI args
  -> src/cli.rs allocates Grid, seeds StdRng, resolves palette, reads ASCII_T
  -> one cli_<mode> handler in a cli_*.rs family
  -> renderer or compositor in the closest source family
  -> src/gridio.rs emits serialized or ANSI output
```

Interactive animation uses the same inputs:

```text
src/opts.rs demo mode list and persisted knob values
  -> src/registry.rs ModeForm and AnimKind
  -> src/morph.rs iterate_grid for in-process frames
  -> subprocess render_frame_t fallback with ASCII_T
```

`iterate_grid` is the low-latency native path. The subprocess fallback also forwards `ASCII_T`, so a time-aware CLI handler can animate before an in-process arm is added.

## Handler signature

Current CLI family handlers use:

```rust
pub(crate) fn cli_name(
    mut grid: Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: [Color; 5],
    mut rng: StdRng,
    t_anim: f32,
    term_w: u16,
    term_h: u16,
    args: &[String],
    mode: &str,
    theme_name: &str,
) -> (Grid, bool) {
    // Parse positional arguments or parameter defaults.
    // Render into grid using seed, palette, rng, and t_anim.
    // Return true only when the handler emitted output itself.
    (grid, false)
}
```

Copy the exact formatting and ownership pattern from a nearby handler. Renderer signatures vary by family; follow the colocated family before introducing another shape.

## Placement map

| Concern | Current location |
|---|---|
| Core cell, grid, rectangle, Unicode width | `src/types.rs` |
| Palette and color transforms | `src/color.rs` |
| Grid rendering | `src/render.rs` |
| Serialized grid I/O and final emission | `src/gridio.rs` |
| Fill algorithms | `src/fills.rs` |
| Layer and mask composition | `src/scene.rs` |
| Generic and stamped sprites | `src/sprites.rs` |
| Pen-grown trees | `src/sprites/trees.rs` |
| Flora growth | `src/sprites/flora.rs` |
| Trait-based tree species and packing | `src/tree_draw.rs`, `src/tree_draw/` |
| Geometric modes | `src/modes_geo.rs` |
| Sky and particle modes | `src/modes_sky.rs` |
| Tree and symbol modes | `src/modes_tree.rs` |
| Creature modes | `src/modes_creatures.rs` |
| Standalone themed systems | `src/arboretum.rs`, `src/astrolabe.rs`, `src/sauron.rs`, `src/ink.rs` |
| CLI mode families | `src/cli_basic.rs`, `src/cli_scenes.rs`, `src/cli_forest.rs`, `src/cli_catalog.rs`, `src/cli_fa.rs`, `src/cli_city.rs` |
| CLI dispatch | `src/cli.rs` |
| Demo list and option persistence | `src/opts.rs` |
| Mode forms and animation kind | `src/registry.rs` |
| Native animation and morph player | `src/morph.rs` |
| Integration snapshots | `tests/snapshot_modes.rs`, `tests/snapshots/` |

## Registration checklist

| Registration | Required condition |
|---|---|
| `src/cli.rs` dispatch branch | Every standalone mode and alias |
| `cli_<mode>` family handler | Every standalone renderer path |
| `src/opts.rs` demo list | Every canonical visual mode intended for browsing |
| `src/registry.rs` `ModeForm` | Modes with knobs or a non-default animation kind |
| `src/morph.rs` `iterate_grid` arm | Native animated modes using the in-process path |
| `tests/snapshot_modes.rs` | Every canonical mode, fixed seed |
| `AGENTS.md` | Architecture or workflow changes; mode names are discovered from code |

## Useful searches

```bash
rg -n 'mode == "NAME"|"NAME"' src/cli.rs src/opts.rs src/registry.rs src/morph.rs tests/snapshot_modes.rs
rg -n '^pub\(crate\) fn cli_' src/cli_*.rs
rg -n '^pub fn draw_|^pub\(crate\) fn draw_' src/modes_*.rs
```
