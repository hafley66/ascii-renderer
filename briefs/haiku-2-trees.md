# haiku-2-trees

Sample sheet displaying six tree-growth algorithms with visible trunk structure, asymmetric branching, and varying canopy densities. Two rows per tree: full energy on top, reduced energy on bottom.

## Species

### Gnarled Oak
Thick trunk with visible knots and lenticular markings (●), tapering upward. Major branches sprout at intervals with asymmetric reach. Canopy fills with diamond glyphs (◆) at decreasing density toward the tips.

### Weeping Willow
Short anchor trunk, then 5 cascading strands hanging from the anchor point. Each strand waves sinusoidally with random drift, creating the classic weeping silhouette. Tips taper to lighter color for visual softness.

### Twisted Spiral
Trunk follows a helix path with 2.5 rotations over tree height. Branches perpendicular to the twist at regular intervals. Crown spirals outward, canopy shows the helical structure as a pinwheel from above.

### Bottle Tree
Baobab-inspired: massive base width (h/4) tapering rapidly then slowly to narrow crown (h/16). Trunk shows textural variation (┃ and ●). Small branches only in upper third. Crown is minimal, suited for arid regions.

### Root System
Inverted tree: main taproot descends vertically, with four lateral roots branching left/right alternately from trapezoid levels. Root hairs (·) attach to laterals. Base transitions to soil color (darkened). Novel: first species that grows downward, showing subsurface structure.

### Clump Shrub
Multiple stems (5–7 per tree) rise from a shared base, each with independent height variance (0.7–1.0x full energy). Stems wobble sinusoidally, branches shoot sideways at regular intervals, crowns overlap to form an irregular mass. Novel: decentralized growth habit, no single trunk.

## Glyph Families

1. Trunk/bark chars: `│ ┃ ● ─ ╱ ╲`
2. Canopy chars: `◆ ◇ ●`
3. Branch junction: `├ ┤ ╴`
4. Ground/soil: `─ ▒`
5. Atmosphere: ` ` (blank as fill)

## Knobs

- **ENERGY**: trunk height and canopy density multiplier (default 0.8, range 0.3–1.0)
- **FRUIT**: density knob for future fruiting bodies (default 0.3, range 0.0–1.0)
- **BRANCH**: branch reach and complexity (default 0.5, range 0.0–1.0)

Positional order: energy fruit branch.

## Render Commands

```bash
ASCII_GRID_W=110 ASCII_GRID_H=36 ./target/release/ascii-renderer 7 haiku-2-trees moss | sed 's/\x1b\[[0-9;]*m//g'
ASCII_GRID_W=110 ASCII_GRID_H=36 ./target/release/ascii-renderer 7 haiku-2-trees moss 0.9 0.2 0.6 | sed 's/\x1b\[[0-9;]*m//g'
```

## Iteration Log

1. Initial render: trees visible but trunks too thin, canopy density too sparse. Fixed trunk width scaling (height/12 instead of /16), doubled canopy fill probability.
2. Second pass: trees now readable but weeping willow strands too regular. Added random drift per strand, varied strand start positions horizontally.
3. Third iteration: bottle tree base too narrow (was using h/6), baobab look lost. Increased base width to h/4, added exponential taper (ease_in_out).
4. Fourth iteration: clump stems all same height, lost irregularity. Added height variance per stem (0.7–1.0x energy), increased wobble amplitude.
5. Fifth iteration: root system hairs too sparse, laterals too short. Increased lateral length (h/4 base instead of h/6), added root hairs at every other segment, darkened soil section below h/2.

## Performance Receipt

Worst knob: ENERGY=1, 715.2 fps at 2000x1000.

| knob at max | fps | avg ms | p99 ms |
| --- | ---: | ---: | ---: |
| baseline | 761.8 | 1.31 | 1.41 |
| ENERGY=1 | 748.5 | 1.34 | 1.45 |
| BRANCH=1 | 757.7 | 1.32 | 1.46 |
| FRUIT=1 | 770.1 | 1.30 | 1.39 |

Hotspots at ENERGY=1: clear (30.5%, 426.6 µs), trees (0.6%, 8.4 µs).
