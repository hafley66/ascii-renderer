# streamnet

Subject: the flow of a single complex potential `w(z) = c1*z + c2*z^2 + ... + cN*z^N`. Its real and imaginary parts are conjugate harmonic fields, so the level sets of `Re w` (equipotentials) and `Im w` (streamlines) cross at right angles everywhere the velocity is nonzero. The whole piece is that orthogonal net drawn in two colors.

What moves and why: each term's coefficient `ck` carries a seeded phase and a seeded drift rate `omega_k` proportional to `(k - center)` so the harmonics precess past one another. The clock advances those phases, which morphs the web smoothly, and rotates `z` by SPIN to turn the picture. At `t = 0` every phase is its seed value, so the static frame is deterministic. Line brightness follows the local velocity `|w'(z)|`: where the flow slows the contours bunch and glow, which is exactly where the net's critical points live.

Glyph families:
1. Equipotential rungs, oriented by the local tangent: `-`, `|`, `/`, `\`
2. Streamline rungs, same orientation set, warm hue
3. Net crossings where both families are bright: `+`
4. Line underglow halo: `.` and a dim fill

Knobs (PARAMS order, also the positional order after seed/theme):
- ZOOM: plane scale, corner maps to `1/ZOOM` (default 1.0)
- DEGREE: highest harmonic N (default 3)
- DENSITY: contour lines per potential unit (default 3.5)
- THICK: line weight in contour-fraction units (default 0.16)
- WARP: value-noise distortion of z before the potential (default 0.12)
- SPIN: plane rotation rad/s (default 0.06)
- MORPH: harmonic phase-drift rate (default 0.45)
- HUE: streamline hue; equipotentials sit 36 degrees warmer (default 198)
- ASPECT: plane columns per row (default 2.0)
- DEPTH: velocity response of line brightness (default 0.7)
- GROUND: background wash tied to speed (default 0.35)
- HALO: line underglow (default 0.3)

Positional order: zoom degree density thick warp spin morph hue aspect depth ground halo.

Layers: field (Horner evaluate `w` and `w'` per cell), wash (dark plate tinted by `|w'|`), ink (both contour families, oriented glyphs, `+` nodes, halo). Layer coverage 93.6 percent, 3 timers.

Perf: release 200x60 frame_cost avg 0.348 ms, worst 0.654 ms. Knob sweep 800x480 2s: baseline 439.9 fps (2.27 ms), worst knob DEGREE=6 at 372.0 fps (1.18x); hotspots at DEGREE=6: field 61.7 percent, ink 22.7 percent, wash 8.4 percent. Raw: perf/results/streamnet.md.

Render commands:
```bash
ASCII_GRID_W=110 ASCII_GRID_H=40 ./target/release/ascii-renderer 42 streamnet deep | sed 's/\x1b\[[0-9;]*m//g'
ASCII_GRID_W=110 ASCII_GRID_H=40 ASCII_T=8 ./target/release/ascii-renderer 42 streamnet deep | sed 's/\x1b\[[0-9;]*m//g'
```

Prompts that produced this piece (verbatim, in order):
1. "yo"
2. "i just got dgx sparks and ur running on them, welcome. lets jam! i want to see what you create without too much input context bias, see this code and use its idioms sure but in reality just try to see if u can make a new art piece without too much guidance"
