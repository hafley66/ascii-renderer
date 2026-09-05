# Architectural mosaic commission

Read AGENTS.md and the local add-mode and add-animation skills first. Work only in your assigned isolated checkout. Other agents are working in this repository; do not revert or overwrite their work. Own one new standalone mode (next available numbered file), its generated registration, snapshots, and its performance report. Do not change Opus quasicrystal renderers, Cargo dependencies, or playback infrastructure without coordinating with root.

## User direction

The user wants ornate art matching the ambition of the Opus and Fable labeled modes, and now specifically Islamic artful mosaics with architectural craft comparable to ordinary Dutch brickwork. They want a new standard of visual complexity AND implementation efficiency. This is a commission to implement and visually verify an artwork, not merely outline an idea. User explicitly requested Astra for this task. Preserve all existing modes.

## Visual intent

Study src/opus_1_quasicrystal.rs, src/opus_2_quasicrystal.rs, relevant snapshots, Opus/Fable forests, and src/modes/_42_bower.rs. Existing Bower uses real Opus venation/phyllotaxis trees, fruit, curls and pointed arches; it benchmarks at 2.69 ms/frame at 2000x1000. Vesper is a folded torus at 41.11 ms/frame. These are context, not templates to duplicate.

Build a composed architectural surface: coherent interlocking star/polygon geometry, ceramic tesserae, grout, subtle material variation, glazed highlights or relief, and borders with distinct scales. Draw on Islamic geometric ornament and the bond, joints, courses, lintels, and restrained material variation of Dutch masonry. Choose a coherent visual hierarchy rather than scattering symbols. Include breathing room so the geometry reads. Avoid text, pseudo-Arabic, random visual clutter, and relying solely on a global radial/trigonometric field. Real tile adjacency and intentional repeated craftsmanship should be visible. Artistic decisions are yours. If historically specific geometry is claimed or references would inform the work, browse authoritative museum/architectural sources and cite them in a short design note; distinguish interpretation from historical reconstruction.

Use colored cells and existing width handling. Render at terminal sizes (100x36, 160x60) and inspect color previews as well as plain snapshots. Inspect 2000x1000 output or a faithful downsample too, so high resolution preserves the design. Never hide work by reducing resolution or density only during benchmarking. Ornateness should survive both scales.

## Implementation and performance

File-owned generated registry Mode; canonical name unique; live knobs declared locally; defaults match render fallbacks. No special dispatch branches. State renderer signatures/pseudocode, lifecycle and storage before implementation. Deterministic seed/time/params; no frame-history dependence. Bounded geometry and memory. Prefer calculating repeated geometric terms once per tile/family/frame, precomputed color ramps, clipped rasterization and disjoint work ownership. Avoid abstractions that obscure loops. Rayon is available as an option if measured benefit and integration ownership are coordinated, not a requirement.

Target under 16.7 ms/frame at 2000x1000 default on this machine, with measured tail latency and expensive control settings; this is a target, not permission to degrade art. Report if unmet and why. Include measure_layer timers. Existing perf/knob_sweep.sh MODE 2000 1000 5 0.06 THEME covers native rendering, not terminal output. Ask root for the benchmark slot BEFORE long timed runs, since Sol is benchmarking another task. Build/tests can proceed independently. Capture cold construction cost as well if caching is used; validate invalidation for dimensions, seed, theme, and shape knobs.

## Validation/delivery

Run baseline tests; generator --check; fixed-seed static and nonzero-time snapshots; complete colored-grid repeatability, parameter effects, seed changes, small grids and finite/nonfinite boundary handling. Inspect snapshots before accepting. Run full cargo test after finishing. No blind snapshot acceptance. Keep current modes untouched. Commit milestones scoped to owned files (no push). Report commits, files, run command, visual choices, benchmark averages/p50/p99/max, cold costs, limits, and any remaining work. Root reviews and integrates.

## Subsequent user steering

Maximum meaningful visual variability is the priority. The user primarily uses demo, animation, random knob mode, reseeding, and arrow-key knob hopping. Opus tessellations are the reference because random combinations produce dramatically varied geometry and motion. Make seed and controls alter structural families, topology, repetition scale, layout, materials and temporal behavior. Inspect a deterministic gallery of random combinations. A fixed scene with surface-only variation is insufficient for this commission. Use the next numbered mode file and generator; root handles preserving numeric chronology in the demo.
