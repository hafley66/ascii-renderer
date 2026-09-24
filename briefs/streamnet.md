# streamnet

Subject: the flow of a seeded analytic complex potential, drawn as its orthogonal equipotential and streamline families. The clock morphs the web, dipoles orbit and bend it into spirals, and flow comets ride the field.

Potential, chosen by FORM:
- harmonic: `w(z) = c1*z + c2*z^2 + ... + cN*z^N`
- dipolar: `+ sum_p q_p / (z - p_p)` (up to 6 orbiting dipoles)
- banded: `+ sum_b A_b sin(B_b*z)` (up to 3 tilted sine bands, giving periodic dislocations)
- chaotic: all three
`Re w` and `Im w` are conjugate harmonic fields, so the two level-set families cross at right angles wherever `|w'(z)|` is nonzero; that net is the whole picture.

What moves and why:
- Polynomial term phases precess at `omega_k ~ (k - centre)`, so harmonics slide past each other. SPIN rotates the plane.
- Dipole centres orbit with time, so the spiral/twin-circle figures continuously reconfigure.
- Band phases drift with MORPH, sliding the dislocation lines.
- Flow comets start at seeded cells and are advected along the local tangent `dir = (1-a)*grad(phi) + a*grad(psi)`, `a = FLOWDIR`. At a=0 they stream along streamlines; at a=1 they pool along equipotentials. Position along each path advances with the clock so beads flow.
- Line luminance follows `|w'(z)|`: glow rises where the flow slows and contours bunch. Streamlines sit at HUE, equipotentials at HUE + 90.

Glyph families:
1. Equipotential rungs, velocity-oriented: `-`, `|`, `/`, `\`
2. Streamline rungs, same set, cool hue
3. Net nodes where both families are bright: `*`
4. Line underglow halo: `.`
5. Comet head and tail: `o`, `+`, `.`

Knobs (PARAMS order, also positional order after seed/theme):
- ZOOM plane scale (1.0) · DEGREE highest harmonic (3) · DENSITY lines per unit (3.5) · THICK line weight (0.15) · WARP coordinate noise (0.10) · SPIN rotation rad/s (0.05) · MORPH phase-drift rate (0.5) · HUE base hue (200) · ASPECT cols per row (2.0) · DEPTH speed response (0.8) · GROUND field wash (0.32) · HALO line underglow (0.28) · POLES dipole count (3) · STRENGTH dipole strength (0.5) · TRACERS flow-comet count (90) · FORM potential family: harmonic/dipolar/banded/chaotic (dipolar) · FLOWDIR comet drift to equipotentials (0)

Positional order: zoom degree density thick warp spin morph hue aspect depth ground halo poles strength tracers form flowdir.

Layers: field (Horner potential + dipole sum + band sum per cell), wash (dark plate tinted by `|w'|`), ink (both contour families, oriented glyphs, node `*`, halo), flow (advect and stamp comets). 4 timers, 96.8 percent attributed.

Perf: release 200x60 frame_cost avg 0.756 ms, worst 1.111 ms. Knob sweep 800x480 2s: baseline 302.2 fps, worst knob FORM=3 at 209.3 fps (1.44x, the band exp/sin in field); hotspots at FORM=3: field 72.4 percent. Raw: perf/results/streamnet.md.

Render commands:
```bash
ASCII_GRID_W=110 ASCII_GRID_H=40 ./target/release/ascii-renderer 42 streamnet deep | sed 's/\x1b\[[0-9;]*m//g'
ASCII_P_FORM=2 ASCII_GRID_W=110 ASCII_GRID_H=40 ./target/release/ascii-renderer 42 streamnet deep | sed 's/\x1b\[[0-9;]*m//g'
ASCII_P_FORM=3 ASCII_P_FLOWDIR=1 ASCII_GRID_W=110 ASCII_GRID_H=40 ASCII_T=6 ./target/release/ascii-renderer 314 streamnet deep | sed 's/\x1b\[[0-9;]*m//g'
```

Prompts that produced this piece (verbatim, in order):
1. "yo"
2. "i just got dgx sparks and ur running on them, welcome. lets jam! i want to see what you create without too much input context bias, see this code and use its idioms sure but in reality just try to see if u can make a new art piece without too much guidance"
3. "all good, can u make it even cooler and variable? also commit ur work. dont worr yabout others work"
4. "yes"
