# Pelagium

Experiment: ART_INPUT_NUMBER=p01, ART_RUN_NAME=p01__crazy-astra-high.
Owned mode: `src/modes/_101_p01_crazy_astra_high.rs`; registry name `pelagium`.

## Signatures before implementation

```rust
fn solve(seed: u64, time: f32, p: &[f32; 5]) -> Mantle;
fn surface(m: &Mantle, u: f32, v: f32, p: &[f32; 5]) -> Vertex;
fn project(v: Vertex, width: usize, height: usize, time: f32) -> Vertex;
fn draw(frame: &mut ModeFrame<'_>, p: &[f32; 5]);
```

1. Lifetime: the existing CLI/native player allocates the grid, resets its RNG,
   and borrows inputs for one frame. Reconstruct the coupled system from seed and
   absolute time, project, shade, discard all frame scratch. No history dependency.
2. Storage: five resolved floats; a periodic 128-node mantle of energy and radial
   displacement; projected mesh vertices; a clipped depth buffer. No retained
   simulation, process environment writes, or files inside rendering.
3. Identity: seed-derived harmonic phases and amplitudes; ring sample index and
   filament anchor angle are stable identities. No time-dependent RNG consumption.
4. Reads/writes: initialize energy and displacement, perform 32 double-buffered
   coupled relaxation steps, read the converged mantle into mesh and filaments,
   depth-test fragments into the borrowed grid.
5. Sequence: periodic energy conduction, strain-dependent conductivity, elastic
   displacement driven by energy, pleated toroidal surface, tilted projection,
   membrane engraving and descending material streamlines.
6. Uniqueness: one canonical mode, one MODE static, five distinct parameter keys,
   exactly the generated dispatch; periodic endpoints refer to the same node.
7. Bounds: 128 nodes, 32 updates, at most 256x64 surface samples, 32,768 triangles,
   12 filaments x 128 segments and two sutures x 256 segments; no recursion,
   particles, stored frames, or branching. The endpoint-inclusive mesh has at
   most 257x65 = 16,705 vertices.
   Raster work is clipped to the visible grid; scratch O(W*H + 16,384).
8. Projection: two columns per row; common scale fits the full vertical silhouette
   and horizontal crown, with an additional 1.25 lateral organism stretch.
   Empty and tiny grids return or clip through one write path.
9. Layers: dark background, opaque shaded mantle with depth occlusion, depth-tested
   ribs and filaments. Fine engraving follows the same surface coordinates.
10. Motion: forcing phases travel through fixed seed harmonics. Projection precesses
    slowly and energy pulses travel down the same filaments. Equal explicit inputs
    reproduce cells and colors, independent of playback order.
11. Continuous controls: APERTURE narrows tube thickness and changes energy forcing;
    TENSION changes elastic smoothing and vertical shell stretch; COUPLING changes
    strain/energy interaction and pleating; FLOW scales all physical time;
    TRAIL changes filament extent, energy drain and membrane loading. No renderer
    selection or integer artistic variants.
12. Feedback: energy drives radial displacement; displacement gradients resist
    energy transport. Filament loading drains the common mantle energy, whose
    resulting field sets filament curvature, length and emitted light.

Visual thesis: an abyssal crown suspended around an empty oculus, with nacreous
pleats carrying light into tapering filaments. Dark water occupies the aperture
and surrounding frame; the bright rim establishes the focal hierarchy.

Validation and visual observations will be recorded below after inspection.

## Rendering milestone

`cargo test rosette -- --nocapture`: 6 unit and 2 integration tests passed before
implementation. Foundation commit: `c98a998`.

The first colored review used seeds 42, 7, 913; times 0, 4, 11; 80x24, 160x56,
24x9; aperture 0, .25, .49, .50, .51, .75, 1. The undersized crown was enlarged,
filaments were attached to frontal material meridians to preserve the dark
oculus, and energy gradients replaced uniform filament torsion. Two surface
sutures clarify the lips. Filaments were then shortened to leave a bottom margin.
The color hierarchy is gold upper membrane, cyan sidewalls, dim blue-green tails.

The five controls share the same solver and projection. APERTURE moves the source
phase and tube radius; TENSION changes strain smoothing and vertical radius;
COUPLING changes conductance, radial strain, pleating and filament curvature;
FLOW advances the common clock; TRAIL changes energy drain and filament extent.
No artistic control is rounded to an integer or selects a different renderer.
