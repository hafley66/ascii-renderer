# polytope

Subject: one regular 4D polytope (5-cell, tesseract, 16-cell, 24-cell, 600-cell, a p x q duoprism, or the 120-cell on request) turning in two or three rotation planes at incommensurate rates, projected 4D to 3D then 3D to 2D, drawn as a depth-shaded wireframe over a receding tiled floor that carries its shadow, with phosphor vertex trails and a plan-view inset.

What moves and why: the seed picks the polytope, the rotation planes (always at least one plane touching w, so the motion is a true 4D turn), the sign and phase of each plane, the base hue, and the style (floor, trails, or both). The primary plane turns once per SPEED seconds; the second and third planes run at 1/phi and 1/phi^2 of that rate, so the pose never repeats on a short cycle. A slow 3D yaw (ORBIT degrees per second) adds parallax against the floor. The 4D perspective scales each vertex by FOV/(FOV - w), so cells nearer the 4D eye swell and the far ones shrink; the 3D perspective then dims and thins what is far from the 3D eye. Edge color follows w (hue sweep of SPAN degrees across the 4D depth), brightness follows z. Where a near edge crosses over a far one, the crossing cell glows. The floor is a perspective grid fixed to the camera, drifting toward the viewer at FLOW units per second; the shadow drops every edge onto that plane along a slanted light and rasterizes it in shade blocks. Trails sample each vertex at TRAIL past instants TAIL seconds apart and fade them into a dot ramp, so the wireframe leaves the arcs it just traced. The inset is the orthographic xy plan of the same rotated vertices, dim, in the upper right.

Glyph families:
1. Near edges, box drawing by direction: `─`, `│`, `╱`, `╲`
2. Mid edges, ASCII by direction: `-`, `|`, `/`, `\`
3. Far edges and trails: `·`, `:`, `.`, `•`
4. Vertices by w-depth, far to near: `·`, `o`, `●`, `◆`; crossings `*`, `+`
5. Floor grid and shadow: `·`, `:`, `.`, `+`, horizon `─`, shadow `░`, `▒`

Knobs:
- POLY: 0 seed picks, 1 5-cell, 2 tesseract, 3 16-cell, 4 24-cell, 5 600-cell, 6 120-cell, 7 duoprism (default 0; the seed never picks 6)
- SPEED: seconds per turn of the primary plane (default 40)
- PLANES: simultaneous rotation planes, 1 to 3 (default 3)
- FOV: 4D eye distance (default 3)
- ZOOM: fit fraction of the shorter grid axis (default 0.46)
- STYLE: 0 seed picks, 1 floor and shadow, 2 trails, 3 both (default 0)
- FLOOR: floor depth under the polytope center (default 0.8)
- TRAIL: trail samples per vertex (default 40)
- TAIL: seconds between trail samples (default 0.3)
- HUE: hue offset in degrees on top of the seed hue (default 0)
- SPAN: hue sweep across w, far to near (default 100)
- ORBIT: 3D yaw in degrees per second (default 1.5)
- PITCH: camera pitch in degrees (default 22)
- TILE: floor tile size in world units (default 0.6)
- FLOW: floor drift toward the viewer in units per second (default 0.08)
- GLOW: crossing glow on or off (default 1)
- LABEL: Schlafli symbol, name, planes, counts caption (default 1)
- CAM: 3D eye distance (default 4.5)
- ASPECT: columns per row (default 2)
- CULL: far-side cut for dense polytopes, scaled by edge count above 200 (default 0.35)
- INSET: plan-view inset radius as a fraction of height, 0 hides it (default 0.2)

Positional order: poly speed planes fov zoom style floor trail tail hue span orbit pitch tile flow glow label cam aspect cull inset.

Cycle lengths at defaults: primary plane 40 s, second 64.7 s, third 104.7 s, yaw 240 s.

Measured frame time, release, 200x60, 200 frames, 120-cell with floor and trails: avg 0.559 ms, worst 0.688 ms.

Render commands:
```bash
ASCII_GRID_W=110 ASCII_GRID_H=36 ./target/release/ascii-renderer 7 polytope moss | sed 's/\x1b\[[0-9;]*m//g'
ASCII_GRID_W=110 ASCII_GRID_H=36 ASCII_T=12 ./target/release/ascii-renderer 7 polytope moss | sed 's/\x1b\[[0-9;]*m//g'
ASCII_GRID_W=110 ASCII_GRID_H=36 ASCII_T=6 ./target/release/ascii-renderer 5 polytope ember 5 40 3 3 0.46 3 | sed 's/\x1b\[[0-9;]*m//g'
```

Perf receipt, `perf/knob_sweep.sh polytope 2000 1000 1`:

| knob at max | frames | fps | avg ms | p50 ms | p99 ms | max ms | vs baseline |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline | 192 | 191.3 | 5.23 | 5.14 | 5.81 | 6.09 | 1.00x |
| CAM=12 | 115 | 114.3 | 8.75 | 7.72 | 16.28 | 23.76 | 1.67x |
| CULL=0.9 | 135 | 134.1 | 7.46 | 7.05 | 13.85 | 14.93 | 1.43x |
| ASPECT=4 | 135 | 134.5 | 7.44 | 6.41 | 17.33 | 19.56 | 1.42x |
| FLOW=2 | 136 | 135.7 | 7.37 | 6.15 | 17.21 | 20.70 | 1.41x |
| INSET=0.5 | 141 | 140.7 | 7.11 | 6.74 | 11.82 | 13.12 | 1.36x |
| PITCH=60 | 143 | 142.7 | 7.01 | 6.71 | 10.21 | 11.30 | 1.34x |
| GLOW=1 | 146 | 145.7 | 6.87 | 5.74 | 14.14 | 34.65 | 1.31x |
| LABEL=1 | 151 | 150.7 | 6.63 | 5.60 | 13.92 | 14.41 | 1.27x |
| ZOOM=1.5 | 159 | 158.6 | 6.30 | 6.23 | 7.24 | 8.25 | 1.21x |
| POLY=7 | 183 | 182.3 | 5.49 | 5.41 | 6.15 | 6.35 | 1.05x |
| TILE=4 | 184 | 183.4 | 5.45 | 5.16 | 9.78 | 12.26 | 1.04x |
| TRAIL=240 | 185 | 184.4 | 5.42 | 5.35 | 6.15 | 6.43 | 1.04x |
| ORBIT=30 | 186 | 185.3 | 5.40 | 5.27 | 6.64 | 7.34 | 1.03x |
| TAIL=1 | 187 | 186.5 | 5.36 | 5.28 | 6.32 | 6.50 | 1.03x |
| HUE=360 | 188 | 187.6 | 5.33 | 5.25 | 6.22 | 6.81 | 1.02x |
| PLANES=3 | 190 | 189.3 | 5.28 | 5.18 | 6.30 | 8.14 | 1.01x |
| SPAN=240 | 190 | 189.6 | 5.27 | 5.19 | 5.91 | 6.11 | 1.01x |
| STYLE=3 | 190 | 189.7 | 5.27 | 5.18 | 6.14 | 6.40 | 1.01x |
| FOV=8 | 192 | 191.2 | 5.23 | 5.16 | 5.84 | 6.39 | 1.00x |
| SPEED=180 | 193 | 192.8 | 5.19 | 5.12 | 5.86 | 6.31 | 0.99x |
| FLOOR=4 | 227 | 226.7 | 4.41 | 4.32 | 4.92 | 5.80 | 0.84x |

Hotspots at CAM=12, 147 frames, 146.7 fps:

| layer | calls/frame | avg us | max us | share of frame |
| --- | ---: | ---: | ---: | ---: |
| floor | 1.0 | 3715.9 | 7854.5 | 54.5% |
| trails | 1.0 | 1104.3 | 1640.5 | 16.2% |
| clear | 1.0 | 839.9 | 1430.0 | 12.3% |
| wire | 1.0 | 69.4 | 157.6 | 1.0% |
| overlay | 1.0 | 31.9 | 59.4 | 0.5% |
| shadow | 1.0 | 10.1 | 110.3 | 0.1% |
| pose | 1.0 | 0.3 | 1.5 | 0.0% |

Timed layers sum to 84.6 percent; the sweep harness clears the 2000x1000 grid once more per frame outside the draw fn (about 0.8 ms, the same cost as the `clear` layer), which accounts for the remainder.
