---
name: add-sprite-algo
description: Add a deterministic procedural sprite or tree algorithm to ascii-renderer using the existing pen, growth, species, and packing systems. Use for turtle walks, L-systems, fractals, branching growth, vines, or controlled sprite variation.
---

# Add Sprite Algorithm

Add a bounded, seed-deterministic drawing algorithm and connect it to the subsystem that owns its lifecycle.

## Select the owning subsystem

| Shape and consumer | Location and interface |
|---|---|
| Trait-based tree archetype used by forest packing | `TreeDrawer` in `src/tree_draw.rs`; species in `src/tree_draw/species.rs` or `species_exotic.rs`; dispatch in `src/tree_draw/pack.rs` |
| Pen-grown tree used by older sprite modes | `TreePen` and growth functions in `src/sprites/trees.rs`; dispatch table near the existing growth functions |
| Flower, vine, fruit, or cloud growth | `src/sprites/flora.rs` |
| Generic stamp, mask, fret, or standalone sprite helper | `src/sprites.rs` |

Follow the colocated state-management and signature style. A tree intended for `pack_forest` must implement `TreeDrawer`, enter `grow_tree_by_index`, and update `TREE_KIND_COUNT` consistently. A pen-tree addition must enter the pen-tree dispatch used by its consumer.

## Shape the algorithm

State the type signatures first. Then add pseudocode comments for:

- Root or anchor initialization.
- Per-instance RNG reads and stable identity.
- Growth, branching, or rewrite sequence.
- Grid clipping and termination.
- Tip, leaf, fruit, or decoration pass.

Describe the instance timeline: the owning mode seeds the RNG, layout assigns a plot or anchor, the algorithm writes cells, and later layers may overwrite them. Describe storage: parameters live in the owning mode arguments or `ModeForm`; the sprite itself receives values through its function inputs. Record uniqueness conditions for species index, dispatch index, and any shared parameter keys.

## Drawing constraints

- Use the existing `Cell`, `Grid`, `TreePen`, `TreeParams`, and color helpers.
- Preserve deterministic RNG read order for unchanged branches.
- Clamp writes to the grid or route them through an existing bounded pen helper.
- Account for terminal cell aspect ratio when evaluating geometric distance or angles.
- Bound recursion depth, rewrite count, branch count, particle count, and walk length.
- Use connectivity-aware glyph selection already present in the owning subsystem when branches must join cleanly.

## Verification

1. Run existing subsystem tests before editing.
2. Add a fixed-seed snapshot beside the subsystem tests.
3. Add or update a mode integration snapshot when the sprite becomes reachable through a mode.
4. Render selected seeds and at least one small grid to inspect variation and clipping.
5. Run focused tests, inspect pending snapshots, then run `cargo test`.

Keep existing species and dispatch indices stable. Append new variants unless the user requests a migration.
