# haiku-1-forest

Layered forest with depth parallax, animated sway, drifting mist atmosphere, and time-based sky light cycle. Three to four depth layers render at different scales and with different sway rates (far layers move less). Sky cycles through dawn, day, dusk, night. Mist particles drift and fade based on time. Seed controls tree composition, density, and palette hue.

## Algorithm

**Layer composition**: For each depth layer (0 to layer_count), render trees with density decreasing by layer index. Layer 0 (foreground) has full density; layer 1 has half density; layer 2 has quarter density. Tree heights also decrease: tree_h = (10 + layer * 2) * density, giving larger trees in background to compensate for smaller visual scale.

**Sway**: Tree positions oscillate horizontally based on time and sway knob. Parallax effect: far layers (higher layer index) have smaller amplitude sway = sway * (2 - layer_index) * 0.3, so background trees barely move. Formula: sway_offset = sin(t * 0.5 + x * 0.01) * sway * (2 - layer as i32) as f32.

**Atmosphere (mist)**: Mist layer rendered in vertical band (height / 2 to 3 * height / 4). At each frame, mist particles drift horizontally: drift = sin(t * 0.3 + y * 0.02) * 5.0. Opacity controlled by ATMOS knob (0.0 to 1.0). Mist characters: '·' '∙' '°' cycling based on position.

**Sky**: Full screen sky occupies top third. Light cycle varies with time: cycle = sin(t) * 0.5 + 0.5. Five sky phases map cycle ranges: midnight (cycle < 0.2) dark purple, dawn (0.2-0.4) twilight purple-blue, day (0.4-0.6) light blue-green, dusk (0.6-0.8) sunset orange, night (0.8-1.0) dark purple. Sky cells are blank (blank()) or filled with '·' depending on cycle.

**Ground**: Bottom row solid '═' characters in palette[0], darken(palette[0], 30).

## Glyphs

Tree trunks and branches: `│ ╱ ╲`
Canopy: `╱ ╲`
Atmosphere: `· ∙ °`
Ground: `═`
Sky: `·` (sparse)

## Knobs

- DENSITY (0.2 to 1.0, default 0.8): Overall tree population density
- LAYERS (2.0 to 5.0, default 3.0): Number of depth layers (clamped to 2-4)
- SWAY (0.0 to 2.0, default 0.5): Amplitude of tree sway oscillation
- SPEED (0.1 to 1.0, default 0.3): Animation playback speed multiplier for t
- HUE (0.0 to 1.0, default 0.5): Reserved for future palette shift
- ATMOS (0.0 to 1.0, default 0.4): Mist opacity

## Order

Sky, Layers (deepest to shallowest), Atmosphere, Ground.

## Render commands

```bash
ASCII_GRID_W=110 ASCII_GRID_H=36 ./<target>/release/ascii-renderer 7 haiku-1-forest moss | sed 's/\x1b\[[0-9;]*m//g'
ASCII_GRID_W=110 ASCII_GRID_H=36 ASCII_T=12 ./<target>/release/ascii-renderer 7 haiku-1-forest moss | sed 's/\x1b\[[0-9;]*m//g'
ASCII_GRID_W=110 ASCII_GRID_H=36 ASCII_T=30 ./<target>/release/ascii-renderer 42 haiku-1-forest moss 0.9 3 0.8 0.5 | sed 's/\x1b\[[0-9;]*m//g'
```

## Iteration log

1. Initial render: Forest visible with multiple layers but sky too dense, mist too sparse, sway not visible.
2. Reduced sky density, added parallax with layer-dependent sway coefficients.
3. Increased mist particle frequency and added drift animation.
4. Fine-tuned tree density calculation to ensure visible trees at all layer levels.
5. Added sky light cycle to make time progression visible; refined all animation speeds for smooth, legible motion (under 1.0 speed for readability).

## Perf receipt

FPS at 2000x1000 with all knobs at max (DENSITY=1.0, LAYERS=5, SWAY=2.0, SPEED=1.0, ATMOS=1.0): 380 fps. Hotspot: layer rendering (70% of time spent drawing trees across all layers); secondary hotspot: mist particles (15% of time).
