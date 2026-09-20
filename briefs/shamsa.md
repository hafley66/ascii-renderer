# shamsa design note (checkpoint 1)

Iran, Safavid dome interiors (Isfahan). Pattern tradition: girih tessellation
(radial rosette construction with interlaced strapwork) on a muqarnas-vaulted
dome, with arabesque fronds growing through the lattice. Mode name: `shamsa`.

## Type signatures

```rust
pub(super) struct Shamsa;
impl Mode for Shamsa { fn render(&self, frame: &mut ModeFrame<'_>) }

struct Look { /* resolved per-frame geometry, palette, node rings, lantern */ }
struct NodeGlow { x: f32, y: f32, b: f32 }          // 21 fixed screen anchors
#[derive(Clone, Copy, Default)]
struct Sample { u, theta, n, light, strap, sdir: f32,
                over: u8, tier: u8, tier_cell: u8, owner: u8, rho: f32 }

fn u_of_r(n: f32, depth: f32) -> f32    // flat disc -> hemisphere projection
fn n_of_u(u: f32, depth: f32) -> f32    // inverse projection for node placement
fn sample_field(field: &mut [Sample], w, h, look)
fn paint_vault(grid, field, w, h, look) // muqarnas tiers, dome shading, wall
fn paint_straps(grid, field, w, h, look)
fn paint_boss(grid, w, h, look)
fn paint_glow(grid, w, h, look)
fn draw(frame: &mut ModeFrame<'_>, p: &[f32; 10])
```

## Design points

1. Timeline: parametric animation. Every frame is recomputed from
   `(seed, t, params)`; no state survives frames. t=0 is the static composition.
2. Stored state: two thread-local scratch buffers (field samples, later the
   vine ink field), sized to the grid. No globals, no env.
3. Identity: splitmix hash over `(seed, layer, index, slot)`; `frame.rng`
   stream untouched. Nodes, tiers, niches, vines have stable hash identities.
4. Reads/writes: one field pass reads Look and writes Sample per cell; paint
   passes read field + write grid, strictly layered (see 9).
5. Rewrite sequence: cell -> (u, theta) on the dome -> nearest of 3 candidate
   rosette frames (center, ring A, ring B) -> analytic girih star distance
   (spokes blended into kinked parallel chords) -> min with tier bands and
   radial connectors -> strap mask -> over/under parity -> light -> paint.
6. Uniqueness: each cell claims at most one owner rosette (min normalized
   distance); straps claim the cell only inside their half-width.
7. Bounds: per cell O(1) with 3 candidate evals; vine segments (checkpoint 3)
   capped at 240 branches x 44 segments; glow halos capped at 21 nodes x box.
8. Clipping: everything derived from (cx, cy, R) with knob aspect; all writes
   index-checked by row/col iteration; tested at 30x12 and smaller.
9. Layers: field, vault, straps, boss, glow (vines slot before glow). Later
   layers only add or darken; strap gaps restore vault by skipping writes.
10. Animation invariants: spin and lantern azimuth are the only global angle
    sources; all jitter from stable hashes; t=0 byte-identical to static.
11. Parameter vector (continuous, one system): FOLD star order, DEPTH dome
    projection, SPIN rad/s, FLOW lantern sweep rad/s, LANTERN height, VINE
    growth budget, STRAP width, BLOOM glow, HUE palette rotation, ASPECT.
12. Feedback: vines (checkpoint 3) steer toward light and away from an ink
    field they deposit; ink then stains the vault shading; straps occlude the
    light field that gates both strap brightness and vine steering.
