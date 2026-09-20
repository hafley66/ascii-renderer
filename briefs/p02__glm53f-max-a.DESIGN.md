# kolam design note (design before body)

Region: India. Traditions: Tamil pulli-kolam knot grids + rangoli mandala radial symmetry.
One coupled system, not composites: a continuous knot-trace scalar field weaves isolines
around a pulli dot lattice and deposits an ink field; a phyllotaxis lotus is placed and
steered by that ink and deposits reinforcement back into it; embers ride the reinforced ink.

## Proposed type signatures

```rust
pub(super) struct Kolam;
pub(super) static MODE: Kolam;
const NAME: &str = "kolam"; const KNOBS: usize = 11;
const PARAMS: &[Param] = &[CURL, RINGS, PETALS, FLORETS, SPREAD, PERSIST, EMBERS, SPIN, TILT, HUE, GLOW];
impl Mode for Kolam { fn render(&self, frame: &mut ModeFrame); /* -> draw(frame, &p) */ }

struct Look;                 impl Look { fn new(seed: u64, w: usize, h: usize, palette: &[Color; 5], time: f32, p: &[f32; KNOBS]) -> Self }
struct Dot { x: f32, y: f32, amp: f32, inv_s2: f32 }         // pulli dot, world coords
struct Floret { r: f32, a: f32, size: f32, hue: f32, ink: f32 } // cp2: phyllotaxis seed

fn hash(seed: u64, layer: u64, index: u64, slot: u64) -> u64; fn unit(h: u64) -> f32; fn smoothstep(t: f32) -> f32;
fn field_at(look: &Look, dx: f32, dy: f32, dots: &[Dot]) -> f32;   // knot scalar field phi
fn trace_field(phi: &mut [f32], w: usize, h: usize, look: &Look);  // pass 1: phi per cell (row parallel)
fn paint_knots(grid: &mut Grid, ink: &mut [f32], phi: &[f32], w: usize, h: usize, look: &Look); // pass 2: isolines + dots + ink deposit
fn grow_lotus(grid: &mut Grid, ink: &mut [f32], look: &Look);      // cp2: petals + florets, reads and writes ink
fn paint_embers(grid: &mut Grid, ink: &[f32], w: usize, h: usize, look: &Look); // cp3: light riding ink
fn draw(frame: &mut ModeFrame, p: &[f32; KNOBS]);
```

## 1. Instance timeline and lifetime
Stateless mode; every frame recomputes Look (geometry, dot lattice, colors) from
(seed, time, params). Static frame at time == 0.0. No persistence across frames.

## 2. Stored state
Thread-local scratch only: PHI: Vec<f32> (w*h field values), INK: Vec<f32> (w*h ink),
reused across frames, sized up on demand. Look owns a bounded dot Vec and row buckets.

## 3. Stable identity and RNG ownership
frame.rng untouched. All per-ring, per-dot, per-petal identity from splitmix
hash(seed, LAYER, index, slot); identical inputs give identical frames.

## 4. Sequence of reads and writes
look -> trace_field (writes PHI) -> paint_knots (reads PHI, writes grid + INK)
-> grow_lotus (reads INK, writes grid + INK) -> paint_embers (reads INK, writes grid).

## 5. Branching, projection, interaction, rewrite sequence
phi(r,a) = r + sum_k amp_k * gauss(r, ring_k) * sin(m_k * (a + spin*t) + phase_k)
  + sum_dots amp_d * gauss(dist, dot_d).  Isolines of phi at n_rings+1 target radii weave
between dot rings and bow around dots (all-positive bumps: lines never cross dots, the
kolam rule). No rewrite system; interaction is the shared INK field.

## 6. Uniqueness conditions
Per-cell overwrite: knots then lotus then embers, each cell written at most once per layer;
lotus skips cells already holding knot cores; embers only brighten existing ink cells.

## 7. Bounds
rings <= 7, dots <= 7 rings x 26 = 182, petals <= 36, florets <= 360, isoline targets <= 9,
two full-grid passes + bounded per-item painters. O(cells) work per frame, no recursion.

## 8. Grid clipping and aspect
World coords: dx = (x + 0.5 - cx)/TILT, dy = y + 0.5 - cy; radius = min(h*0.46, w*0.5/TILT)
* SPREAD, floors at 2.0; rings auto-reduce on tiny grids; all writes inside row slices.

## 9. Layer and overwrite order
measure_layer wrap: look, field, knots, (lotus, ornament, embers as they land).
Background stays blank: deliberate negative space between mandala and frame edge.

## 10. Animation invariants
time == 0 identical to static render. Motion: SPIN turns theta; per-ring phase wobble is
sin(time * . . .), bounded and slow; embers advance (time * rate + hash offset) mod 1;
petal breathing scales length by bounded sin. No per-frame rng draws: adjacent frames differ
only through continuous time terms.

## 11. Continuous parameter vector
CURL lobe amplitude, RINGS lattice depth, PETALS lotus count, FLORETS phyllotaxis fill,
SPREAD radius, PERSIST feedback strength, EMBERS transport rate, SPIN rotation,
TILT projection, HUE palette bias deg, GLOW exposure. All feed the one field system;
RINGS and PETALS round to counts (rosette FOLD precedent), the rest act continuously.

## 12. Feedback paths (emergence)
Knots deposit INK (A -> B): petal placement weighting and floret sizes read INK; petals
deposit reinforcement (B -> A) scaled by PERSIST: reinforced ink thickens nearby knot
halo glyphs and speeds ember light. Nearby parameter values produce nearby ink, so
petal steering and ornament clustering move continuously through the control space.
