# Astra: Jurassic Park ultra-variable

Create one original animated ASCII mode: a dense, cute, ultra-variable dinosaur preserve viewed as one complete image at 80x24 and scalable to 2000x1000.

## Required drawing algorithms

Implement the dinosaur algorithms in the mode source, not as static stamps. Build each animal from bounded deterministic geometry: body, head or crest, neck where applicable, legs, tail, eye, and a ground-contact rule. Each rendered dinosaur must vary from a stable seed identity in species, facing, scale, gait phase, color, pose, and placement. Include at least four visibly distinct silhouettes such as a long-neck herbivore, horned herbivore, small raptor, and large biped. Keep every walk and raster pass bounded by the grid.

## Scene and controls

Layer a readable preserve scene around the herd: distant mountains or canopy, a fence or gate, vegetation, ground texture, atmospheric depth, and foreground occlusion. The mode must animate herd gait and environmental motion deterministically from `ASCII_T`.

Expose at least eight live controls, including herd density, species mix, dinosaur scale, vegetation density, time/weather or light, motion speed, scene depth, and a seed-sensitive variation control. Controls must produce materially different frames. Preserve negative space so small terminal frames remain legible.

Use the file-owned `Mode` API and fixed-seed inline snapshots at two times. Test deterministic output, animation movement, parameter clamping, and a small grid. Do not modify existing modes, dependencies, playback, generated registration, snapshots outside this file, or integration tests.

## Lease contract

Write only `src/modes/_D_next.rs`. Do not choose the final numeric prefix. The primary checkout leases its next `_N_` filename, generates the registry, reviews snapshots, runs release sweeps, installs, and commits after delivery.

Commit the draft with this exact subject:

`feat: astra jurassic park draft`

Report the canonical mode name, controls, species algorithms, tests, and commit hash.
