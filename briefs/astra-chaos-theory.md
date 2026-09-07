# Astra: chaos theory, maximal knobbing

Create one original animated ASCII mode themed around visible chaos theory. The picture should read at 80x24 as a coherent dynamic instrument, not as a generic noise field. Use deterministic seed-driven math and a composition with an attractor or orbit system, phase-space trails, a bifurcation or sensitivity motif, and multiple visual depths.

Expose a maximal, useful set of live `Mode` parameters: at least sixteen independent controls. Include controls for attractor family, orbit count, integration step, trail length, divergence, coupling, damping, bifurcation threshold, temporal speed, zoom, rotation, projection, density, glyph palette, hue or light, grain, depth, and a seed-sensitive variation control. Each must affect the renderer. Group parameter names and labels so the demo remains usable.

Render deterministically from `(dimensions, seed, theme, parameters, ASCII_T)`. Bound every orbit, integration step, trail, and write by grid size. Animate through `ASCII_T` without persistent simulation state. Include inline fixed-seed snapshots at two times, deterministic/parameter-clamp tests, and a small-grid termination test.

Write only `src/modes/_D_next.rs`. Do not edit generated registration, snapshots outside this file, integration tests, performance reports, dependencies, playback, or existing modes. The primary checkout assigns the numeric filename, generates registration, snapshots, profiles, installs, and commits.

Commit the draft with this exact subject:

`feat: astra chaos theory draft`

Report the canonical mode name, parameter count and names, rendering algorithm, tests, and commit hash.
