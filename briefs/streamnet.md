# streamnet

Subject: the flow of one complex potential `w(z) = c1*z + ... + cN*z^N + sum q_p / (z - p_p)`. Its real and imaginary parts are conjugate harmonic fields, so the level sets of `Re w` (equipotentials) and `Im w` (streamlines) cross at right angles wherever the velocity `|w'(z)|` is nonzero. The piece is that orthogonal net in two colours, plus flow comets riding the streamlines.

What moves and why:
- The polynomial terms carry seeded phases that precess at rates `omega_k ~ (k - centre)`, so the harmonics slide past one another and the whole web morphs. SPIN rotates the plane.
- Up to six complex dipoles `q_p / (z - p_p)` sit inside the view and orbit slowly with time. A dipole makes a symmetric source/sink-with-circulation, so the net bows into twin circles and spirals around each pole and the figure changes character completely from seed to seed.
- Flow comets start at seeded cells and are advected along the streamlines (`dz/ds = conj(w'(z))` normalized), leaving a short bright tail. Their position along each path advances with the clock, so beads stream through the net.
- Line luminance follows `|w'(z)|`: the flow slows at the net's critical points, where contours bunch and glow. Streamlines sit at HUE, equipotentials at HUE + 90.

Glyph families:
1. Equipotential rungs, oriented by the local velocity: `-`, `|`, `/`, `\`
2. Streamline rungs, same orientation set, cool hue
3. Net nodes where both families are bright: `*`
4. Line underglow halo: `.`
5. Comet head and tail: `o`, `+`, `.`

Knobs (PARAMS order, also positional order after seed/theme):
- ZOOM plane scale, corner maps to `1/ZOOM` (default 1.0)
- DEGREE highest harmonic N (default 3)
- DENSITY contour lines per potential unit (default 3.5)
- THICK line weight in contour-fraction units (default 0.15)
- WARP value-noise distortion of z (default 0.10)
- SPIN plane rotation rad/s (default 0.05)
- MORPH harmonic phase-drift rate (default 0.5)
- HUE streamline hue, equipotentials +90 (default 200)
- ASPECT plane columns per row (default 2.0)
- DEPTH velocity response of brightness (default 0.8)
- GROUND background wash tied to speed (default 0.32)
- HALO line underglow (default 0.28)
- POLES dipole count 0..6 (default 3)
- STRENGTH dipole strength (default 0.5)
- TRACERS flow-comet count (default 90)

Positional order: zoom degree density thick warp spin morph hue aspect depth ground halo poles strength tracers.

Layers: field (Horner potential + dipole sum per cell), wash (dark plate tinted by `|w'|`), ink (both contour families, oriented glyphs, node `*`, halo), flow (advect and stamp comets). 4 timers, 95.7 percent attributed.

Perf: release 200x60 frame_cost avg 0.757 ms, worst 1.165 ms. Knob sweep 800x480 2s: baseline 312.6 fps (3.20 ms), worst knob POLES=6 at 241.6 fps (1.29x); hotspots at POLES=6: field 66.1 percent, ink 16.3 percent, flow 6.6 percent. Raw: perf/results/streamnet.md.

Render commands:
```bash
ASCII_GRID_W=110 ASCII_GRID_H=40 ./target/release/ascii-renderer 42 streamnet deep | sed 's/\x1b\[[0-9;]*m//g'
ASCII_GRID_W=110 ASCII_GRID_H=40 ASCII_T=5 ./target/release/ascii-renderer 314 streamnet deep | sed 's/\x1b\[[0-9;]*m//g'
```

Prompts that produced this piece (verbatim, in order):
1. "yo"
2. "i just got dgx sparks and ur running on them, welcome. lets jam! i want to see what you create without too much input context bias, see this code and use its idioms sure but in reality just try to see if u can make a new art piece without too much guidance"
3. "all good, can u make it even cooler and variable? also commit ur work. dont worr yabout others work"
