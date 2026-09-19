# heliotrope

One coupled system: a vine grown by 3D space colonization toward the dawn sun,
lit by that sun through its own accumulated wood, thickened in proportion to the
light it carries, and thinned where the shade it casts leaves it starved. A
turning camera then reveals the depth of the structure the light shaped.

## Type signatures

```rust
struct V3 { x: f32, y: f32, z: f32 }               // world space, y up, ground at y=0
struct Field { data: Vec<f32>, nx, ny, nz }        // shared wood density, BX x BY x BX box
struct Node { pos, dir: V3, parent: u32, light, sugar: f32, leaves: u16,
              thick, along, avg: f32, gen: u8, tip: bool }
struct Stand { nodes: Vec<Node>, tips: Vec<u32>, attr: Vec<V3>, alive: Vec<bool>,
               attr_light: Vec<f32>, next: Vec<u32>, leaf: Vec<bool> }
struct Knobs { lean, crown, fork, crowd, light, prune, leaf, sap, sun, cam, haze, aspect: f32 }
struct Look { k: Knobs, seed, w, h, ppu, cx, cy, az, tilt, sun, sun_seed: V3,
              sun_px, sun_py, sun_r, horizon, time, palette roles ... }
struct Stamp { depth, ax, ay, bx, by, w, light, surv: f32, gen: u8, kind: u8 }
```

```rust
fn grow(seed: u64, k: &Knobs, sun: V3, b: &Budget, field: &mut Field, st: &mut Stand);
fn light_pass(st: &mut Stand, field: &Field, sun: V3, gain: f32);
fn transport(st: &mut Stand);                       // sugar up the lineage, thickness from sugar
fn deposit(st: &Stand, field: &mut Field, optical: bool);
fn solve(l: &Look, field: &mut Field, st: &mut Stand);
fn project(l: &Look, p: V3) -> (f32, f32, f32);     // column, row, camera depth
fn build_stamps(l: &Look, st: &Stand, sc: &mut Scratch);
fn paint_sky / paint_sun / build_shadow / paint_soil / paint_stand / paint_sap / paint_bloom
```

## 1 Timeline and lifetime

Parametric: every frame is a pure function of (seed, params, t). No retained
state; the runtime recreates the grid and RNG per time value, so a stand is grown
and solved inside one frame.

## 2 Stored state

Scratch only, per thread: the density field, the stand, the stamp list, the shadow
buffer, the bloom list. Nothing outlives a frame, nothing is static mutable.

## 3 Identity and RNG

Splitmix64 over (seed, layer, index, slot). No rng stream is consumed. A branch,
leaf, mote or tuft keeps the same identity for every t.

## 4 Read and write order

grow: deposit wood -> light march reads wood -> attractor weights -> next growth
step. Per frame: grow, then light_pass, transport, deposit(optical), light_pass,
transport. Two rounds of light -> mass -> opacity -> light.

## 5 Branching and projection

Attractor pull + phototropic pull + occupancy gradient repulsion + keyed jitter.
Forks spawn when grown length passes a light-scaled spacing; the fork plane rolls
by the golden angle so limbs spread in 3D. Orthographic projection with weak
perspective, camera azimuth turning with t.

## 6 Uniqueness

Seed sets the crown cloud, the sun azimuth, the light azimuth and the camera
offset; the stand is a deterministic consequence.

## 7 Bounds

steps <= 150, tips <= base 26, nodes <= base 900, attractors <= base 150, field
26x22x26, march <= 30 steps, stamps <= nodes + leaves, bloom <= 384 entries,
motes <= 64. All scale with sqrt(area), hard capped.

## 8 Clipping

Every plot goes through a bounds check; ppu is capped by width so a narrow
terminal shrinks the whole scene instead of cropping it.

## 9 Layer order

sky, sun, cast shadow, soil, stand (z-sorted wood and leaves), sap, bloom.

## 10 Animation invariants

Structure depends on the seed only; t drives the sun azimuth and elevation, the
camera azimuth, sap phase and leaf shimmer. t=0 renders the static frame.

## 11 Parameters

lean, crown, fork, crowd, light, prune, leaf, sap, sun, cam, haze, aspect. All
continuous, all fed into the same system: crowd is the occupancy feedback gain,
light is the shading gain, prune is the shade-thinning compensation point.

## 12 Feedback

Growth writes density; density shades later growth through both the attractor
weights and the direction gradient (the vine steers out of its own occupied
space). Light drives leaf sugar; sugar sets branch thickness; thickness sets
opacity; opacity re-shades the leaves. Subtrees whose average light falls under
the compensation point thin and pale out, so the crown hollows where it shades
itself.
