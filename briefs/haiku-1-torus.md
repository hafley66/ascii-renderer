# haiku-1-torus

Subject: a three-dimensional torus rotating about its minor axis, the parallels and meridians drawn as concentric and spiral glyphs, revealing its topology as it tilts through a full rotation.

What moves and why: the torus rotates continuously around its vertical axis via parameter u (0 to 2π), and also tilts by rotating around a horizontal axis via parameter v_offset. The meridian circles (vertical cross-sections) appear as nested regions; the parallels (horizontal circles) spiral outward and inward. The tilt creates depth perception: the back half of the torus dims and uses sparse glyphs, while the front half brightens. A secondary ring pulses at the innermost hole to accent the topological center.

Glyph families:
1. Inner hole rim: `@`, `o`, `*`
2. Meridian rings (dense): `#`, `%`, `=`
3. Parallel edges (medium): `+`, `-`, `:`
4. Antialias/sparse (thin): `.`, `` ` ``, `~`
5. Accent pulse at center: `o`, `*`

Knobs:
- SPEED: rotation speed in radians per second (default 1.0)
- TILT: tilt amplitude away from vertical in radians (default 0.5)
- SCALE: overall torus size (default 1.0)
- HUE: primary hue rotation (0-360, default 180)
- MINOR: minor radius fraction of major (default 0.35)
- DENSITY: glyph density factor (default 1.0)

Positional order: speed tilt scale hue minor density.

Perf results: TBD after knob sweep.

Render commands:
```bash
ASCII_GRID_W=110 ASCII_GRID_H=36 ./target/release/ascii-renderer 42 haiku-1-torus moss | sed 's/\x1b\[[0-9;]*m//g'
ASCII_GRID_W=110 ASCII_GRID_H=36 ASCII_T=12 ./target/release/ascii-renderer 42 haiku-1-torus moss | sed 's/\x1b\[[0-9;]*m//g'
```
