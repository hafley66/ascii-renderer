# ASCII Renderer: Pattern Research & Algorithm Reference

A comparative analysis of procedural pattern generation techniques for character-grid rendering. Each section covers the visual characteristics, algorithmic construction, grid mapping strategy, and relevant Unicode glyph palettes.

---

## Table of Contents

1. [Truchet Tiles](#1-truchet-tiles)
2. [Diffusion-Limited Aggregation (DLA)](#2-diffusion-limited-aggregation-dla)
3. [GRIS-Style Binary Trees](#3-gris-style-binary-trees)
4. [Flower Stamps](#4-flower-stamps)
5. [Stepped Frets (Xicalcoliuhqui)](#5-stepped-frets-xicalcoliuhqui)
6. [Aztec Diamond Tilings](#6-aztec-diamond-tilings)
7. [Masonry / Bond Patterns](#7-masonry--bond-patterns)
8. [Textbook Fitting & Layout](#8-textbook-fitting--layout)
9. [Color](#9-color)
10. [Glyph Harmony Rules](#10-glyph-harmony-rules)
11. [Comparative Summary](#11-comparative-summary)
12. [Sources & Further Reading](#12-sources--further-reading)

---

## 1. Truchet Tiles

### What it is

Two tiles -- `╱` and `╲` -- placed via coin flip per cell. The simplest possible generative primitive. Zero adjacency constraints, zero contradiction risk. Despite carrying almost no information per cell (1 bit), the emergent visual is complex: curving paths, enclosed regions, and labyrinthine texture appear from pure randomness.

### Algorithm

```
for each cell (x, y):
    grid[y][x] = if coin_flip() { '╱' } else { '╲' }
```

That's it. The diagonal characters connect at cell edges, so adjacent cells always form continuous paths regardless of orientation. This is why Truchet works -- there are no invalid configurations.

### Variants worth exploring

- **Stepped Truchet**: Replace smooth diagonals with staircases within each cell (top-left to bottom-right or top-right to bottom-left). When tiled randomly, produces meandering stepped paths that closely resemble Mesoamerican fretwork. See [McNeel Forum discussion](https://discourse.mcneel.com/t/replicating-a-stepped-truchet-pattern/118986/3).
- **Quarter-circle Truchet**: Arc from one edge midpoint to an adjacent edge midpoint. Creates flowing organic curves. Harder to render in Unicode but possible with `╭╮╰╯`.
- **Multi-state Truchet**: More than 2 tile states. Each additional state multiplies visual complexity but requires careful adjacency thinking.

### Grid mapping

Direct 1:1. Each grid cell is one tile. Works at any scale. The diagonal box-drawing characters `╱╲` are the canonical choice; they connect cleanly at cell boundaries.

### References

- [Truchet Tiling -- Wolfram MathWorld](https://mathworld.wolfram.com/TruchetTiling.html)
- Sébastien Truchet's original 1704 memoir on combinatorial tiling
- [Smith (1987)](https://doi.org/10.1007/BF01385860) -- the quarter-circle variant

---

## 2. Diffusion-Limited Aggregation (DLA)

### What it is

Particles random-walk from a spawn region and freeze ("stick") when they contact an existing frozen cluster. The result is fractal, dendritic crystal growth -- branching tendrils that look like lightning, coral, or frost on a window.

### Algorithm

```
1. Seed: freeze one or more initial cells (the "nucleus")
2. For each of N particles:
   a. Spawn at a random position NEAR the existing cluster
      (spawning from edges wastes walk steps in empty space)
   b. Random walk: move to a random cardinal neighbor each step
   c. If adjacent to a frozen cell: freeze this particle, done
   d. If exceeded max_steps or left bounds: discard particle
3. Repeat until N particles placed or desired density reached
```

### Key tuning parameters

| Parameter | Effect |
|-----------|--------|
| Spawn distance from cluster | Close = dense, compact. Far = sparse, stringy |
| Number of particles | More = denser fill |
| Max walk steps | Higher = particles can reach further, more branching |
| Sticking probability | < 1.0 = smoother, less fractal. 1.0 = maximally branchy |
| Number of seeds | Multiple seeds grow toward each other, creating collision boundaries |

### Glyph zones

The current POC uses distance-from-seed to assign three visual zones:

| Zone | Distance | Glyphs | Visual role |
|------|----------|--------|-------------|
| Border | 0-2 cells from seed | `╍ ┅ ┉` | Structural, dense |
| Mid | 3-5 cells | `⟋ ⟍ ⧸ ⧄ ⧈` | Geometric transition |
| Far | 6+ cells | `⡷ ⣟ ⢿ ⣻ ⣿` | Braille corruption, organic dissolution |

**Lesson learned**: Round glyphs (`○●◆◈◇`) clash with the angular Truchet background. Angular crystal glyphs (`╳╬┼╪▪`) blend much better. Glyph harmony is real and matters (see [Section 10](#10-glyph-harmony-rules)).

### References

- Witten & Sander (1981) -- original DLA paper
- [DLA on Wikipedia](https://en.wikipedia.org/wiki/Diffusion-limited_aggregation)

---

## 3. GRIS-Style Binary Trees

### What it is

Inspired by the tree visuals in the game GRIS (Nomada Studio, 2018). Binary recursive splits using box-drawing characters. All tips align at a flat canopy line. Strict grid alignment -- no diagonal branches, no curves beyond the split corners.

### Algorithm

```
1. Draw trunk from root_y upward to first_split_y
2. Queue: [(root_x, canopy_y, first_split_y, depth=0)]
3. While queue not empty:
   a. Pop (x, top_y, bottom_y, depth)
   b. If depth >= max_depth or no vertical room: draw terminal │ to canopy, cap with ╷
   c. Otherwise:
      - split_y = midpoint of (top_y, bottom_y)
      - Draw │ from bottom_y down to split_y
      - Arm length = spread >> depth (halves each level)
      - Left arm: ╭───── from (x - arm_len) to x
      - Right arm: ─────╮ from x to (x + arm_len)
      - Center junction: ┼ (or ┤ if left-only)
      - Push left child and right child onto queue
```

### Visual characteristics

- Perfectly flat canopy line (all tips at the same y)
- Binary branching only (no ternary or variable fan-out)
- Branch spread halves with each depth level, creating naturalistic taper
- Box-drawing curves (`╭╮`) at branch points give organic feel despite grid constraint

### Grid mapping

Horizontal arms use `─`, vertical segments use `│`, splits use `╭─┼─╮`. Terminal tips use `╷`. The tree occupies a rectangular region defined by (root_x ± total_spread, canopy_y, root_y).

### References

- GRIS (2018), Nomada Studio -- visual reference for the branching style
- General recursive binary space partitioning

---

## 4. Flower Stamps

### What it is

Small fixed-pattern sprites (3x3 to 5x5 cells) stamped at specific coordinates. Five styles in the current POC:

| Style | Center | Visual |
|-------|--------|--------|
| Diamond | `✦` | `◆◇` cardinal, structural |
| Circle | `◉` | `◠◟◞◡` organic bloom |
| Star | `✧` | `∧⟨⟩∨` + diagonal `╱╲` rays |
| Box | `╬` | `╔╗╚╝╥╟╢╨` architectural |
| Braille | `⣿` | `⡇⢸⣤⣶` dense texture |

### Algorithm

Each stamp is a lookup table of `(dx, dy, char)` offsets from center. Stamp by iterating offsets and writing to grid if in bounds.

### Design notes

Flowers serve as visual anchors and rhythm breakers. They work best placed at structural intersections (between content regions, at column gaps, along divider lines). Scattering them randomly degrades into noise.

---

## 5. Stepped Frets (Xicalcoliuhqui)

### What it is

The fundamental geometric motif of Mesoamerican art. The Nahuatl word *xicalcoliuhqui* means "twisted gourd." It's a rectangular spiral where each successive arm is one unit shorter than the previous, creating the characteristic stepped profile.

The most extraordinary physical example: the walls of Mitla (Oaxaca, Mexico), where over **100,000 individually carved stone pieces** are fitted without mortar into mosaic panels. Six base fret designs are combined three at a time in horizontal bands, yielding 100+ distinct permutations from a small alphabet. The frets at Mitla use 5 steps, corresponding (possibly) to the 5 visible planets.

### Algorithm: Turtle-walk construction

```
fn stepped_fret(grid, start_x, start_y, steps, direction):
    x, y = start_x, start_y
    arm_length = steps
    dir = direction  // RIGHT, DOWN, LEFT, UP (cycles)

    for i in 0..steps:
        // Draw horizontal/vertical arm
        for j in 0..arm_length:
            grid[y][x] = glyph_for(dir)
            (x, y) = advance(x, y, dir)

        // Turn 90 degrees
        dir = turn_right(dir)

        // Step inward by 1
        grid[y][x] = corner_glyph(dir)
        (x, y) = advance(x, y, dir)

        // Turn 90 degrees again
        dir = turn_right(dir)

        arm_length -= 1  // each arm shrinks by 1
```

The shrinking arm length is the key. It forces the path to spiral inward to a terminal point. Each fret unit connects to the next via the spiral hook, creating continuous meander borders.

### Symmetry operations

Mesoamerican pattern design uses four symmetry transforms applied to the base fret:

| Transform | Effect | Visual result |
|-----------|--------|---------------|
| Horizontal reflection | Bilateral mirror | Most common, flanking doorways |
| 180° rotation | Creates S-shapes | Double spirals, interlocking pairs |
| Translation | Repeat along band | Border meanders |
| Glide reflection | Translate + flip | Interlocking positive/negative space |

The **interlocking spiral** variant is particularly powerful: generate one fret, rotate 180°, offset so the second fret fills the negative space of the first. The positive and negative shapes are congruent.

### Grid mapping

Box-drawing characters map directly:

| Segment | Glyphs |
|---------|--------|
| Horizontal arm | `─ ━ ═` |
| Vertical arm | `│ ┃ ║` |
| Corners | `┌ ┐ └ ┘` (sharp) or `╭ ╮ ╰ ╯` (rounded) |
| Filled interior | `█ ▓ ▒ ░` |

For border bands: generate one fret unit, then repeat with translation along the edge. The fret width (number of steps) determines how deep the border intrudes into the canvas.

### Comparison with Greek Key

The Greek key (meander) and xicalcoliuhqui are structurally identical -- both are stepped spirals with 90° turns. The Mesoamerican variants tend toward:
- Deeper spirals (more steps per unit)
- Bilateral/quadrilateral symmetry over simple translation
- Integration with stepped pyramid profiles
- Denser fill (less negative space between fret units)

These are independent inventions. The Mediterranean and Mesoamerican traditions had no contact, but converged on the same geometric primitive because the stepped spiral is one of a small number of patterns that tile a rectangular band without gaps.

### References

- [Xicalcoliuhqui -- Wikipedia](https://en.wikipedia.org/wiki/Xicalcoliuhqui)
- [Mitla: The Mysterious Stepped-Fret Mosaics](https://uncoveredhistory.com/mexico/mitla-the-mysterious-stepped-fret-mosaics/)
- [Meander (art) -- Wikipedia](https://en.wikipedia.org/wiki/Meander_(art))
- [CMU 15-104: Turtle Graphics Meander Assignment](https://courses.ideate.cmu.edu/15-104/f2017/week-10-due-nov-4/)
- [Unraveling Roman Mosaic Meander Patterns -- McGill](http://www-cgrl.cs.mcgill.ca/~godfried/teaching/dm-reading-assignments/Chedworth.pdf)

---

## 6. Aztec Diamond Tilings

### What it is

The Aztec diamond of order *n* is the set of unit squares whose centers (x, y) satisfy |x| + |y| <= n. It looks like a diamond made of stacked rows: widths 2, 4, 6, ..., 2n, 2n, ..., 6, 4, 2. The number of domino tilings of this shape is exactly **2^(n(n+1)/2)** -- a staggeringly large number even for small n.

The visual payoff: for large n, a random tiling exhibits the **Arctic Circle phenomenon**. Four frozen corners emerge where every domino is deterministically aligned the same way (brickwork), surrounding a central "temperate zone" of randomness. The boundary between frozen and temperate converges to a circle of radius n/√2. The frozen regions are four different colors/orientations, creating a natural four-quadrant color scheme.

### Algorithm: Domino Shuffling

Iterative growth from order 1 to order n. Each step expands the diamond by one ring.

```
1. INITIALIZE: Order-1 diamond (2x2). Fill randomly with 2 horizontal
   or 2 vertical dominoes (coin flip).

2. For k = 1 to n-1, expand from order k to k+1:

   a. MOVE (deterministic):
      - Cells labeled N shift up by 1
      - Cells labeled S shift down by 1
      - Cells labeled E shift right by 1
      - Cells labeled W shift left by 1
      Each cell carries a direction indicating which half of its domino
      it is and which way the domino points.

   b. DESTROY (deterministic):
      - If an N-S pair would swap positions (head-on collision),
        remove both dominoes. Mark cells as empty.
      - Same for E-W pairs colliding horizontally.

   c. FILL (probabilistic):
      - After expansion, empty cells form disjoint 2x2 blocks
        (guaranteed by diamond geometry).
      - Each 2x2 block: randomly fill with 2 horizontal or 2 vertical
        dominoes.

3. Repeat until order n reached.
```

The algorithm is due to Elkies, Kuperberg, Larsen, and Propp (1992). The probabilistic step at each level is what creates the frozen/temperate phase transition.

### Grid mapping for the renderer

Each domino is a 1x2 or 2x1 rectangle on the character grid:

| Domino orientation | Glyphs | Visual |
|-------------------|--------|--------|
| Horizontal (E-W) | `═══` or `▀▀` | Stretcher brick |
| Vertical (N-S) | `║` stacked or `█` over `█` | Header brick |
| Frozen N corner | All vertical, same direction | Regular vertical stripes |
| Frozen S corner | All vertical, opposite | Regular vertical stripes |
| Frozen E corner | All horizontal | Regular horizontal stripes |
| Frozen W corner | All horizontal | Regular horizontal stripes |
| Temperate zone | Mixed random | Organic texture |

The diamond boundary `|x|+|y| <= n` maps to a grid loop:
```
for y in -n..=n:
    x_range = n - |y|
    for x in -x_range..=x_range:
        // this cell is inside the diamond
```

### Why this matters for the renderer

The frozen corners are free structured borders. The temperate zone is free organic fill. The arctic circle boundary is a natural frame. And the whole thing is seed-deterministic (the only randomness is in the FILL step coin flips). This is one of the most visually rich primitives available per line of code.

### References

- Elkies, Kuperberg, Larsen, Propp (1992) -- "Alternating-Sign Matrices and Domino Tilings"
- [Arctic Circle Theorem (arXiv: math/9801068)](https://arxiv.org/abs/math/9801068)
- [UC Louvain Aztec Diamond Interactive](https://sites.uclouvain.be/aztecdiamond/algorithm/)
- [UC Louvain Domino Shuffling Implementation](https://sites.uclouvain.be/aztecdiamond/domino-shuffling-implementation.html)
- [Jim Propp, "My Life with Aztec Diamonds"](https://mathenchant.wordpress.com/2021/01/16/my-life-with-aztec-diamonds/)
- [Python Implementation (mango314)](https://gist.github.com/mango314/4511055)

---

## 7. Masonry / Bond Patterns

### What it is

Brick-laying patterns ("bonds") for grid-based background textures. The key variables are brick dimensions, row offset, and perpend (vertical joint) alignment rules. Different bonds create different visual rhythms and can serve as substrate layers beneath other primitives.

### Terminology

- **Stretcher**: long face of a brick (2+ cells wide on grid)
- **Header**: short face (1 cell wide on grid)
- **Course**: a single horizontal row of bricks
- **Perpend**: vertical joint between adjacent bricks in a course

### Bond patterns

#### Running Bond (Stretcher Bond)

The most common. Each course offset by half a brick from the one above. No perpend ever aligns across adjacent courses.

```
for each row y:
    offset = if y % 2 == 0 { 0 } else { brick_width / 2 }
    x = offset
    while x < grid_width:
        place_brick(x, y, brick_width)
        x += brick_width
```

Visual (brick_width = 4):
```
[══][══][══][══]
  [══][══][══][══]
[══][══][══][══]
```

#### Stack Bond

No offset. All perpends align vertically. Grid-trivial but visually static.

```
[══][══][══][══]
[══][══][══][══]
[══][══][══][══]
```

#### Herringbone (90°)

Alternating horizontal/vertical dominoes in a checkerboard of 2x2 super-cells:

```
for each 2x2 block at (bx, by):
    if (bx + by) % 2 == 0:
        place horizontal pair
    else:
        place vertical pair
```

Creates a zigzag visual that reads as diagonal motion despite all elements being axis-aligned.

#### Basket Weave

Pairs of bricks alternate orientation in a checkerboard:

```
for each super-cell (cx, cy):
    if (cx + cy) % 2 == 0:
        two horizontal bricks stacked
    else:
        two vertical bricks side by side
```

Produces a woven textile appearance.

#### Random Bond (Nebos algorithm)

The "Math of Masonry" approach:

```
1. Place bricks row by row, choosing header (1-wide) or stretcher (2-wide)
2. After each brick, check previous 3 rows for perpend alignment:
   - "Standing teeth": perpend continues upward through multiple rows (bad)
   - "Falling teeth": perpend continues downward through multiple rows (bad)
3. If a perpend would extend > 2-3 rows, force a stretcher to break it
4. Goal: no perpend aligns for more than 2-3 courses
```

This produces structurally sound and visually varied patterns that look hand-laid.

### Grid mapping

| Element | Glyphs |
|---------|--------|
| Stretcher face | `══ ▓▓ ██ ░░` |
| Header face | `▓ █ ░` |
| Horizontal mortar | `─ ━` or space |
| Vertical mortar (perpend) | `│ ┃` or space |
| Corner mortar | `┼ ╋` |

For the renderer, masonry works best as a background texture layer: fill the grid with a bond pattern first, then overlay content, trees, frets, and other primitives on top. The mortar lines provide subtle visual rhythm without dominating.

### References

- [The Math of Masonry -- J. Nebos (Medium)](https://medium.com/@jnebos/the-math-of-masonry-8b027e6fa48)
- [Brick Bonds -- Architextures](https://architextures.org/stories/brick-bonds)

---

## 8. Textbook Fitting & Layout

### What it is

The meta-algorithm: given content (text, data, labels) and a canvas size, compute a layout that places content regions and fills remaining space with generative patterns. This is the composition layer that turns individual primitives into a complete rendering.

### Layout model

The renderer treats the canvas as a set of **regions** with roles:

| Region type | Role | Fill strategy |
|-------------|------|---------------|
| Content | Text, data, labels | Placed first, carved out of canvas |
| Border | Edge decoration | Stepped fret meander, chain pattern, or rail |
| Background | Fills behind everything | Truchet, masonry, or Aztec diamond |
| Ornament | Visual anchors between regions | Flowers, stamps, small diamonds |
| Structure | Trees, crystals, large-scale features | DLA, binary trees |

### Fitting algorithm (proposed)

```
1. MEASURE: Calculate content bounding boxes from input data
   - Each content block has (min_width, min_height, preferred_width, preferred_height)
   - Headers, bars, flow text, gauges each have their own sizing rules

2. PARTITION: Divide canvas into regions
   - Border band: outermost N rows/cols (N = fret step count)
   - Content zones: placed according to spec (top-center, left-column, etc.)
   - Structural zones: corners and margins for trees/crystals
   - Ornament slots: gaps between content zones

3. FILL (inside-out):
   a. Place content text into content zones
   b. Generate border pattern along edge band
   c. Fill background in remaining empty space
   d. Grow structural elements (trees, DLA) in their zones
   e. Stamp ornaments at computed anchor points

4. COMPOSITE: Resolve overlaps
   - Content overwrites everything (highest priority)
   - Borders overwrite background
   - Structural elements overwrite background but not borders
   - Ornaments overwrite background but not content
```

### Content sizing rules (textbook-style)

For text fitting, the approach borrows from typesetting:

- **Measure phase**: iterate characters, track line width, break at word boundaries or explicit newlines
- **Line breaking**: Knuth-Plass is overkill for a terminal. Simple greedy: break at last space before max_width. If no space, hard break.
- **Vertical stacking**: each content block gets `ceil(text_lines) + padding` rows
- **Column layout**: if multiple columns, divide available width minus gap. Each column fits independently.

### Dynamic gauge/bar rendering

Bars and gauges are proportional fills:

```
bar_width = (value / max_value) * available_width
filled = "█".repeat(bar_width)
empty = "░".repeat(available_width - bar_width)
```

### Why layout matters

Without layout, the renderer is a toy that stamps patterns at hardcoded coordinates. With layout, it becomes a compositing engine that accepts structured input and produces fitted output. The YAML spec format (see session notes) describes the input; this section describes how that input gets mapped to pixel (cell) positions.

---

## 9. Color

### What it is

ANSI/crossterm color support. Each cell carries a foreground color, background color, and character. The output uses ANSI escape sequences for 256-color or RGB true color terminals.

### Implementation approach

The grid type changes from:
```rust
type Grid = Vec<Vec<char>>;
```
to:
```rust
struct Cell {
    ch: char,
    fg: Color,
    bg: Color,
}
type Grid = Vec<Vec<Cell>>;
```

Where `Color` is from the `crossterm` crate (already a transitive dependency via `console_engine`). Output uses `crossterm::style::SetForegroundColor` and `SetBackgroundColor` per cell, with run-length optimization (only emit escape codes when color changes from previous cell).

### Color strategy per primitive

| Primitive | Color approach |
|-----------|---------------|
| Truchet background | Muted single hue, low saturation, serves as canvas |
| DLA crystals | Distance-from-seed gradient (bright core to dim tendrils) |
| Trees | Brown trunk, green canopy gradient (darker at base) |
| Flowers | Accent color, high saturation, contrast with background |
| Stepped frets | Two-tone (fill + outline) or monochrome gold/terracotta |
| Aztec diamond | Four-quadrant scheme from frozen corner orientations |
| Masonry | Warm earth tones, slight per-brick hue variation |
| Content text | High contrast (white on dark or black on light) |

### Palette selection

Seed-deterministic palette generation:
```
base_hue = hash(seed) % 360
palette = [
    hsl(base_hue, 0.3, 0.15),        // background
    hsl(base_hue + 30, 0.6, 0.5),    // primary
    hsl(base_hue + 180, 0.5, 0.4),   // complement
    hsl(base_hue + 60, 0.4, 0.6),    // accent
]
```

Rotating base_hue ensures every seed produces a visually distinct but harmonious palette.

### References

- crossterm crate: `crossterm::style` module
- console_engine wraps crossterm and provides `Color` enum with RGB support

---

## 10. Glyph Harmony Rules

### What it is

Empirical rules about which Unicode characters look good together on a character grid. These were discovered through iteration, not theory.

### Rules

1. **Angular + angular = good.** Truchet diagonals (`╱╲`) blend with angular crystals (`╳╬┼╪▪`) and box-drawing frets (`┌─┐│`).

2. **Round + angular = bad.** Circle glyphs (`○●◆◈◇`) clash with diagonal Truchet and box-drawing. They work as isolated accents (flower centers) but not as fill.

3. **Braille + anything = corruption zone.** Braille characters (`⡷⣟⢿⣻⣿`) are so visually dense they read as "static" or "glitch." Use them for dissolution boundaries, not structure.

4. **Block elements are neutral.** `█▓▒░` blend with everything because they have no directional bias. Good for backgrounds and fills.

5. **Double-line box drawing (`║═╔╗╚╝`) reads as "heavy" or "structural."** Use for borders and frames, not for organic growth.

6. **Single-line box drawing (`│─┌┐└┘`) reads as "light" or "organic."** Use for trees, frets, and internal structure.

7. **Mixing single and double box drawing** creates visual hierarchy. Double for outer frame, single for inner detail.

### Why this matters

A randomly chosen glyph palette will look like garbage. These rules constrain the palette per-primitive so that the composite output has visual coherence. The constraint is similar to typeface pairing rules in typography: contrast in weight and style, but harmony in underlying geometry.

---

## 11. Comparative Summary

### Primitives by role

| Role | Best primitive | Why |
|------|---------------|-----|
| Background texture | Truchet, masonry | Low visual weight, fills uniformly |
| Border decoration | Stepped fret, Aztec diamond frozen zones | Structured, repeating, bounded |
| Organic fill | DLA, Aztec diamond temperate zone | Fractal/random, high visual interest |
| Structural accent | Binary trees, large fret units | Occupy significant space, anchor the eye |
| Point accent | Flower stamps | Small, high contrast, rhythm markers |
| Content frame | Box-drawing rectangles | Clear boundary, readable content inside |

### Primitives by complexity

| Primitive | Lines of code | Parameters | Visual richness |
|-----------|--------------|------------|-----------------|
| Truchet | ~5 | 0 (just RNG) | Medium (emergent) |
| Flower stamps | ~20 | 2 (position, style) | Low (fixed) |
| Masonry | ~30 | 3 (bond type, brick size, mortar) | Medium (structured) |
| Stepped fret | ~40 | 4 (steps, direction, symmetry, position) | High (cultural) |
| Binary trees | ~80 | 5 (root, canopy, spread, depth, RNG) | High (organic) |
| DLA | ~60 | 5 (seed, particles, steps, stick prob, RNG) | Very high (fractal) |
| Aztec diamond | ~100 | 2 (order, RNG) | Very high (phase transition) |

### Primitives by computational cost

| Primitive | Cost | Notes |
|-----------|------|-------|
| Truchet | O(w*h) | One pass, one random call per cell |
| Masonry | O(w*h) | One pass with offset logic |
| Flower stamps | O(k) | k = number of stamps, each is ~9 cells |
| Stepped fret | O(steps * arm_length) | Linear in border length |
| Binary trees | O(2^depth) | Exponential in depth but depth is small (4-6) |
| DLA | O(particles * steps) | The expensive one. 3000 * 2000 = 6M random walks |
| Aztec diamond | O(n^3) | n shuffling steps, each touching O(n^2) cells |

---

## 12. Sources & Further Reading

### Truchet & Tiling
- [Truchet Tiling -- Wolfram MathWorld](https://mathworld.wolfram.com/TruchetTiling.html)
- [Stepped Truchet Pattern discussion -- McNeel Forum](https://discourse.mcneel.com/t/replicating-a-stepped-truchet-pattern/118986/3)

### DLA & Crystal Growth
- Witten, T. A., & Sander, L. M. (1981). "Diffusion-Limited Aggregation, a Kinetic Critical Phenomenon." *Physical Review Letters*, 47(19), 1400-1403.
- [DLA -- Wikipedia](https://en.wikipedia.org/wiki/Diffusion-limited_aggregation)

### Mesoamerican Geometry
- [Xicalcoliuhqui -- Wikipedia](https://en.wikipedia.org/wiki/Xicalcoliuhqui)
- [Mitla: The Mysterious Stepped-Fret Mosaics](https://uncoveredhistory.com/mexico/mitla-the-mysterious-stepped-fret-mosaics/)
- [Meander (art) -- Wikipedia](https://en.wikipedia.org/wiki/Meander_(art))
- [CMU 15-104: Turtle Graphics Meander](https://courses.ideate.cmu.edu/15-104/f2017/week-10-due-nov-4/)
- [Unraveling Roman Mosaic Meander Patterns -- McGill](http://www-cgrl.cs.mcgill.ca/~godfried/teaching/dm-reading-assignments/Chedworth.pdf)

### Aztec Diamond & Domino Tilings
- Elkies, N., Kuperberg, G., Larsen, M., & Propp, J. (1992). "Alternating-Sign Matrices and Domino Tilings."
- [Arctic Circle Theorem (arXiv: math/9801068)](https://arxiv.org/abs/math/9801068)
- [UC Louvain: Aztec Diamond Algorithm](https://sites.uclouvain.be/aztecdiamond/algorithm/)
- [UC Louvain: Domino Shuffling Implementation](https://sites.uclouvain.be/aztecdiamond/domino-shuffling-implementation.html)
- [Jim Propp, "My Life with Aztec Diamonds"](https://mathenchant.wordpress.com/2021/01/16/my-life-with-aztec-diamonds/)
- [Python Aztec Diamond (mango314)](https://gist.github.com/mango314/4511055)

### Masonry
- [The Math of Masonry -- J. Nebos](https://medium.com/@jnebos/the-math-of-masonry-8b027e6fa48)
- [Brick Bonds -- Architextures](https://architextures.org/stories/brick-bonds)

### ASCII Art & Terminal Rendering
- [ASCII Automata v2 -- hlnet.neocities.org](https://hlnet.neocities.org/ascii-automata) -- edge-matching glyph propagation
- Cambridge North railway station -- Rule 30 cellular automaton on architectural panels
- console_engine crate -- terminal canvas with composable screens
- crossterm crate -- ANSI color and terminal control

### Related Algorithms
- [wfc crate](https://crates.io/crates/wfc) -- Wave Function Collapse for Rust (46k downloads)
- [noise crate](https://crates.io/crates/noise) -- Perlin/Simplex noise (1.87M downloads)
- [fast_hilbert crate](https://crates.io/crates/fast_hilbert) -- space-filling curves (85k downloads)
- [L-system -- Wikipedia](https://en.wikipedia.org/wiki/L-system)

---

*This document is a living reference. Update as new primitives are implemented and new patterns discovered.*
