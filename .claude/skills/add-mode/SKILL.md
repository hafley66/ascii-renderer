---
name: add-mode
description: Scaffold a new rendering mode for ascii-renderer. Trigger on "new mode", "add mode", "make a X mode", or any request to create a new visual mode.
---

# Add Mode

Scaffold a new rendering mode in ascii-renderer with snapshot test coverage.

## Steps

1. **Pick a name.** Lowercase, no spaces. Check it doesn't collide with existing modes in main.rs (`} else if mode == "`).

2. **Find insertion point.** New modes go before the `} else if mode == "world"` block in main.rs. This keeps world/noise/default as the fallback section.

3. **Write the mode block.** Follow this skeleton:

```rust
} else if mode == "NEW_NAME" {
    // Grid is already initialized: width x height of Cell::blank()
    // palette: [Color; 5] from theme
    // rng: StdRng seeded from CLI arg

    // 1. Background (optional): fill grid with truchet/noise/blank
    // 2. Main content: draw sprites, fills, or compositions
    // 3. Post-processing (optional): atmosphere, borders, overlays
}
```

4. **Wire CLI args.** Any tuning knob should be a positional arg after the theme:
```rust
let my_param: u32 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(DEFAULT);
```

5. **Add snapshot test.** In `tests/snapshot_modes.rs`:
```rust
#[test]
fn NEW_NAME_seed_42() {
    insta::assert_snapshot!(render(&["42", "NEW_NAME", "ember"]));
}
```

6. **Run tests.** `cargo test` must pass. Accept new snapshot with `cargo insta accept`.

7. **Update CLAUDE.md.** Add the mode name to the modes list.

## Rules

- Never modify existing modes
- Use fixed seed for deterministic output
- Expose constants as CLI args, don't hardcode
- Keep the mode self-contained in the `else if` block (call into walker/scene/sprites as needed)
- Background fill patterns: use `fill_truchet`, `fill_noise`, or `Cell::blank()`
- Sprites: `draw_tree`, `draw_flower`, `draw_fruit`, `draw_mask` from sprites.rs
- Composition: `render_scene` with `Layer` and `FillGen` from scene.rs
