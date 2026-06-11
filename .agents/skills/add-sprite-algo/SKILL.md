---
name: add-sprite-algo
description: Design and implement algorithmic sprites that draw themselves via walk/growth algorithms instead of stamp-from-array. Trigger on "algorithmic sprite", "L-system", "turtle walk", "fractal", "generative tree", "growth algorithm", "procedural sprite", or requests for sprites with controlled random variability.
---

# Add Sprite Algorithm

Create sprites that grow/draw themselves via algorithms rather than hardcoded character arrays.

## Design Principles

- **Seed-deterministic**: same seed = same output, different seeds = visibly different shapes
- **Asymmetry is good**: unbalanced trees, lopsided growth, weighted branching
- **Algorithm families**:
  - **Turtle walk**: position + direction + rules. Branch = push state, draw segment, pop
  - **L-system**: string rewrite rules iterated N times, then interpret as drawing commands
  - **Recursive subdivision**: split space, recurse with probability decay
  - **Radial growth**: angle sweep with per-ray length variation
  - **DLA (diffusion-limited aggregation)**: particles stick on contact, organic clusters

## Implementation Pattern

All sprites in sprites.rs follow this signature pattern:

```rust
pub fn grow_THING(
    grid: &mut Grid,
    root_x: usize, root_y: usize,  // anchor point
    canopy_y: usize,                // vertical extent (or similar bound)
    spread: usize,                  // horizontal extent
    color: Color,
    rng: &mut StdRng,               // controlled randomness
) {
    // Algorithm writes directly to grid cells
    // Use Cell::new(ch, color) to place characters
    // Respect grid bounds: check y < grid.len() && x < grid[0].len()
}
```

## Character Palette

Terminal cells are ~1:2 aspect ratio (taller than wide). Adjust horizontal distances by 1.5-2x.

**Trunk/branch chars**: `│ ┃ ╱ ╲ ╭ ╮ ╰ ╯ ┤ ├ ┬ ┴`
**Leaf/canopy chars**: `◆ ◇ ● ○ ∙ · ⟨ ⟩ ∧ ∨`
**Dense fill chars**: `█ ▓ ▒ ░ ⣿ ⡇ ⢸`
**Organic chars**: `~ ≈ ∿ ⌇ ⌒`

## Steps

1. **Pick the algorithm family** based on what visual you're targeting
2. **Write the growth function** in sprites.rs with the standard signature
3. **Register it** in `draw_tree` match (if tree-like) or as standalone
4. **Wire into modes** that use sprites (forest2, party NegativeSpace, etc.)
5. **Add snapshot test** -- the algorithm must be deterministic given seed
6. **Test with multiple seeds** to verify variety: `cargo run -- 42 forest2 ember`, seeds 1-10

## Example: Turtle Tree

```
Start at root. Direction = up. Stack = empty.
For each step:
  - Draw trunk char at current position
  - With probability P (decreasing with height): branch
    - Push current state
    - Rotate left/right by random angle
    - Reduce spread
    - Continue drawing
    - Pop state
  - Move forward (up, adjusted for terminal aspect ratio)
  - With probability Q: slight angle jitter
Terminate when: above canopy_y, or spread < 1
```

## Anti-patterns

- Don't stamp from a fixed char array -- that's what we're replacing
- Don't make everything symmetric -- real plants aren't
- Don't ignore terminal aspect ratio -- horizontal movement needs 1.5-2x scaling
- Don't let the algorithm run unbounded -- always clamp to grid dimensions
