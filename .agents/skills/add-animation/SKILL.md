---
name: add-animation
description: Add deterministic time-based motion and live tuning to an ascii-renderer mode. Use when creating animation, making a static mode move, adding particles or trails, or wiring a mode into the demo and morph player.
---

# Add Animation

Make each frame reproducible from explicit inputs while supporting native playback and live knobs.

## Select the lifecycle

- Parametric animation: the current runtime recreates the grid and seeded RNG for each `t`. Use this for orbital motion, waves, particles with analytically derived ages, procedural wind, trails reconstructed from time, and other frames computable from explicit inputs.
- Retained simulation: requests involving mutable velocity, collision history, cellular state, user-controlled entities, or effects that depend on prior frames require an owner for per-mode state in the interactive loop. The current `iterate_grid` contract has no retained mode state. Define the state type, initialization, update, render, resize, reset, and seed-change signatures before changing the runtime. Keep that state out of globals and environment variables.

## Define the frame

State the renderer signature before editing. Preserve the closest family signature and include `t: f32`:

```rust
fn draw_name(
    grid: &mut Grid,
    width: usize,
    height: usize,
    seed: u64,
    palette: &[Color; 5],
    rng: &mut StdRng,
    t: f32,
    // mode parameters
)
```

Add pseudocode comments for initialization, position or phase evaluation, bounded drawing, and overlays before filling in the body.

## Timeline and storage

- A normal CLI render allocates a blank grid, seeds `StdRng` from `seed`, reads `ASCII_T`, and renders one frame.
- The demo and morph loops repeatedly evaluate frames at new `t` values.
- `iterate_grid` in `src/morph.rs` recreates grid and RNG for each in-process frame.
- `render_frame_t` is the subprocess fallback and forwards `ASCII_T`.
- `ModeForm` in `src/registry.rs` declares slider metadata.
- Slider values persist by `(mode, key)` in the options TSV and enter the renderer through `ASCII_P_<KEY>` plus `param_f32`.
- Parameter keys must be unique within a form. Aliases that share behavior may share one form entry.

## Implement and wire

1. Run `cargo test` before modifying an existing mode.
2. For a parametric animation, make positions, topology, glyph choice, and colors functions of the explicit frame inputs. Use `t` for phase and motion. Derive stable per-object identities from the seed or a freshly seeded RNG.
3. Keep particle, trail, recursion, and walk work bounded by the grid dimensions and declared parameters.
4. Forward `t_anim` through the mode's `cli_<mode>` handler.
5. Add or update its `ModeForm`. Use `AnimKind::Iterate` for native time animation.
6. Add the canonical mode to `iterate_grid` so interactive playback stays in process.
7. For live knobs, keep the `Param.default` value identical to the renderer's `param_f32` fallback and clamp at the renderer boundary.
8. Run `.agents/skills/add-mode/scripts/0_check_mode.sh <mode> --animated`.

## Tests

Cover these frame properties:

- Same dimensions, seed, parameters, and `t` produce the same plain grid.
- Two selected time values produce different plain grids when visible motion is expected.
- A fixed-seed integration snapshot covers the ordinary CLI frame.
- At least one fixed nonzero time snapshot covers the animated composition when the zero-time frame omits important motion states.
- Small dimensions and parameter extrema stay within bounds and terminate.

Inspect changed snapshots before accepting them. Run the focused tests, then `cargo test`.
