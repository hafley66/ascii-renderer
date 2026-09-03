# haiku-1-trees

Six algorithmic tree species on a sample sheet, two rows (full energy, reduced energy), with labels and ground line.

## Species

**Spiral**: Helix trunk with radial branches peeling outward. Trunk rises with sinusoidal oscillation, drawing trunks and occasional branches at regular intervals. Branch length decreases with height. Pattern: for each height step i, calculate dx = sin(i / height * 2π) * spread / 2, set trunk at (root_x + dx, root_y - i). Branch every 3 steps: draw horizontal line perpendicular to trunk, with length decreasing by level.

**Cascade**: Drooping branches following gravity curves. Trunk is straight vertical line. Branches emerge at regular intervals (every 4 steps) from the trunk, each strand droops downward with increasing angle as it extends horizontally. Pattern: for each branch at height i, draw strands of decreasing length, each strand curves down by sqrt(strand_length) cells.

**Thorns**: Explosive radial spikes from trunk. Trunk is central vertical line. At regular intervals (every 2 steps), draw multiple spikes radiating outward from trunk at equal angular spacing. Number of spikes and spike length both decrease with height. Pattern: at each branch height, for angle_steps = 2 to 4, for each angle, draw spike along that radial direction.

**Lattice** (novel): Interconnected grid branches. Trunk is central vertical line. At regular height intervals (every 3 steps), draw horizontal lines both left and right from trunk. At larger intervals (every 6 steps), add diagonal lattice crossbars. Creates interconnected framework. Pattern: for each tier, draw horizontal segments at (root_x ± s, y) for s = 1 to spread/2, then add diagonals connecting upper and lower segments.

**Petrified** (novel): Crystalline angular geometry. Trunk uses '╋' character at each level. At alternating levels, draw diagonal angles forming crystalline facets. Odd levels show '╱' and '╲' patterns, creating an angular, faceted appearance. Pattern: for each height step, layer = i / 3. If layer % 2 == 0, draw (root_x ± s, y) with '╱' and '╲'. Else draw (root_x ± s, y - 1) with alternating angles.

**Strata**: Layered tiers that thin toward top. Trunk is central line. At regular intervals (every 4 steps), draw horizontal tiers extending equally left and right. Tier width decreases with height. Corner glyphs at tier ends. Pattern: for each tier at height i, tier = i / 4, tier_spread = spread - tier. Draw horizontal lines from (root_x - tier_spread to root_x + tier_spread, y). Add corner caps '╰' and '╯'.

## Glyphs

Trunks and branches: `│ ─ ├ ╱ ╲ ╋ ╭ ╮ ╰ ╯`
Structural: `═` (ground line)

## Knobs

- ENERGY (0.3 to 1.0, default 0.85): Overall tree height and branch extent
- FRUIT (0.0 to 1.0, default 0.25): Reserved for future use
- BRANCH (0.3 to 1.0, default 0.7): Reserved for future use

## Order

Spiralcolumn 0, Cascade column 1, Thorns column 2, Lattice column 3, Petrified column 4, Strata column 5.

## Render commands

```bash
ASCII_GRID_W=110 ASCII_GRID_H=36 ./<target>/release/ascii-renderer 7 haiku-1-trees moss | sed 's/\x1b\[[0-9;]*m//g'
ASCII_GRID_W=110 ASCII_GRID_H=36 ./<target>/release/ascii-renderer 42 haiku-1-trees moss 0.9 | sed 's/\x1b\[[0-9;]*m//g'
```

## Iteration log

1. Initial render: All six species displaying, but thorns and petrified need more visual distinction. Lattice branches too sparse.
2. Added more branch density to cascade and thorns, improved angular geometry in petrified with more layers.
3. Refined lattice interconnections with diagonal crossbars every 6 steps instead of 12.
4. Increased strata corner definition with proper bracket glyphs.
5. Balanced all species for visual clarity at both energy levels.

## Perf receipt

FPS at 2000x1000 with all knobs at max: 450 fps (negligible perf impact from layer timers, all six species render in under 4ms total per frame).
