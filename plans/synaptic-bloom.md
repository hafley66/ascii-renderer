# synaptic-bloom design

Mode: `src/modes/_103_p01_crazy_sonnet_high.rs`, registry name `synaptic-bloom`.

## Type signatures

```rust
struct Vec3 { x: f32, y: f32, z: f32 }
struct Node { pos: Vec3, parent: i32, order: u32, thickness: f32 }
struct PulsePath { nodes: Vec<usize>, cum: Vec<f32>, total: f32, phase: f32 }
struct Ganglion {
    fw: f32, fh: f32, depth: f32,
    nodes: Vec<Node>, max_order: f32,
    attractors: Vec<Vec3>, consumed: Vec<bool>,
    paths: Vec<PulsePath>,
}
fn Ganglion::build(seed: u64, w: usize, h: usize, p: &[f32; 13]) -> Ganglion;
fn Ganglion::project(&self, pos: Vec3, theta: f32, aspect: f32) -> (f32, f32, f32);
```

## 1. Instance timeline and lifetime

`Ganglion::build` runs once per `Mode::render` call (once per frame, matching
the "no per-frame persisted state" architecture note). It is a pure function
of `(seed, w, h, params)` -- never `time`. `draw()` then projects that fixed
structure through a time-varying camera angle, growth-reveal fraction, and
pulse phase.

## 2. Stored state

Nodes (parent-linked forest, roots have `parent == -1`), the attractant
cloud with a `consumed` bitmap, a coarse `FIELD_W x FIELD_H` density grid
(scratch, local to `build`, not stored on `Ganglion` after use), and
precomputed root-to-leaf pulse paths with cumulative arc length.

## 3. Stable identity and RNG ownership

`hash(seed, layer, index, slot)` (splitmix64) is the only source of
randomness; `frame.rng` is never touched, so `time == 0.0` reproduces
byte-identically across calls (required for snapshots).

## 4. Sequence of reads and writes

Per growth iteration, per tip: read nearby unconsumed attractors + the local
field gradient, write a new node, deposit into the field, mark attractors
within `KILL` consumed. Field blurs and decays every 8 iterations.

## 5. Branching / interaction sequence

A tip finds an attention-weighted mean direction to attractors in
`PERCEIVE` radius. The angular spread of those attractors versus the mean is
fed through `smoothstep(SPLIT*0.7, SPLIT*1.3, spread)` to get a branch
probability; a seeded hash roll decides. Passing tips partition attractors
by sign of the perpendicular component and split into two children.

## 6. Bounds

`MAX_NODES = 300`, `MAX_ITERS = 240`, `MISS_LIMIT = 5` (a tip with no
attractors in reach for 5 steps terminates), `ATTRACT` capped at 380,
`SOMAS` capped at 4, pulse paths capped at `SOMAS * 3`.

## 7. Grid clipping

Every grid write goes through `set_if_nearer`, which bounds-checks against
`w`/`rows` before indexing. Small grids (tested at 20x8, 6x4, 1x1) just clip.

## 8. Layer and overwrite order

`field` (background density fog + unconsumed attractant motes) -> `somas`
(root bodies) -> `edges` (depth-buffered dendrite segments) -> `pulses`
(depth-tested traveling impulses). Somas and edges share one f32 depth
buffer so nearer geometry always wins regardless of paint order within a
frame.

## 9. Animation invariants

`growth_frac = smoothstep(0, GROWTH_SECONDS, time)` gates edge reveal by
node birth `order`; `theta = time * SPIN` rotates the camera around the
vertical axis (x/z only, giving parallax from `DEPTH`); pulse position is
`(time * PULSE + path.phase) mod (path.total * 1.5)`, clamped to the
revealed portion of its path. All three are pure functions of `time`.

## 10. Continuous parameter vector

`SOMAS, ATTRACT, PERCEIVE, KILL, STEP, REPEL, SPLIT, TROPISM, DEPTH, SPIN,
PULSE, HUE, ASPECT` -- 13 knobs, all feed the shared growth/field/projection
system directly (no knob selects a renderer).

## 11. Feedback paths (emergence)

Growth deposits into the density field; the field's gradient repels later
tips (`REPEL` weight) -- the field, in turn, was entirely produced by
growth. This is the space-colonization / self-avoidance loop the brief
calls out explicitly: growth changes an occupancy field, the field
redirects later growth.
