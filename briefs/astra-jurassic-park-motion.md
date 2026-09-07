# Astra: Jurassic Park motion and variation

Own only `src/modes/_48_astra_jurassic_park.rs`.

The existing preserve has procedural dinosaur silhouettes. Revise it so the dinosaurs visibly move and the herd reads as distinct individuals at both 80x24 and large grids.

Use parametric animation from explicit frame inputs only. At `ASCII_T=0`, retain a full static scene. At nonzero times, give each species a deterministic identity-driven gait and pose: leg stride, body bob, head or neck motion, tail swing, and terrain-relative ground contact. Use separate phase offsets, stride lengths, and cadence ranges per dinosaur identity. Add species-specific behavior, such as long-neck browsing, horned head dips, raptor hops or alert turns, and large-biped steps. Do not introduce retained runtime state.

Increase visible variety with bounded deterministic differences in silhouette, scale, age, markings, facing, spacing, herd formation, and foreground/background placement. Retain all existing controls and add useful live controls for gait amplitude, migration or formation, juvenile mix, pose activity, and visible herd variation. Keep small-grid output legible and each render bounded by grid dimensions.

Update inline snapshots and tests in this source file. Test deterministic identical frames, visibly distinct zero/nonzero times, parameter clamping, and small-grid termination. Do not edit generated registration, integration tests, external snapshots, performance reports, dependencies, playback, or other modes. Run focused tests and `cargo test`.

Commit with this exact subject:

`feat: animate Astra Jurassic Park herd`

Report the movement functions, variation controls, test command, and commit hash.
