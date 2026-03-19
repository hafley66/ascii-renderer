# Flower Energy System -- Synthesis of 3 Haiku Agents + Sonnet Filter

## Core Concept

A single `energy: f32` (0.0-1.0) drives the entire flower. NOT TreePen -- flowers use
radial direct placement. Box-drawing chars look like tree branches on flowers.

## Energy Parameter Mapping (concrete formulas)

```
petal_count  = (energy * 16.0).floor() as u32    // 0 at bud, 12+ at bloom
radius       = (energy * 4.0).max(0.5)           // 0.5-4.0 cells from center
stem_length  = (energy * 6.0) as usize           // 0-6 rows
symmetry     = if e < 0.5 { 2 } else if e < 0.75 { 4 } else { 6 }
color_stops  = ((energy * 5.0).ceil() as usize).max(1)
stamen_alpha = ((energy - 0.30) / 0.70).max(0.0) // hidden until e=0.3
```

## Energy Stages

| Energy | Stage | Size | Center | Petals | Stamen |
|--------|-------|------|--------|--------|--------|
| 0.0-0.2 | Bud | 1x1-3x3 | `⬤` | none | hidden |
| 0.2-0.35 | Tight bud | 3x3-4x4 | `✦` | 3-4 `◆◇` | hidden |
| 0.35-0.55 | Opening | 5x5-6x6 | `✦`/`✧` | 5-6 filled | faint outline |
| 0.55-0.75 | Half-open | 6x6-8x8 | `◉` | 7-8, 4-fold | `·∙` ring |
| 0.75-0.9 | Full bloom | 8x8-10x10 | `❋` | 9-12, 6-fold | double layer |
| 0.9-1.0 | Overbloom | 10x10-12x12 | `❋` | 12+, chaotic | pollen scatter |

## Character Palette

**Bud (e < 0.35):** `⬤` `✦` `◆` `◇` -- compact, diamond/dot shapes
**Opening (0.35-0.75):** `✧` `◆` `●` `◉` `◑` `◐` -- denser, rounder
**Bloom (e > 0.75):** `❋` `✿` `◉` `●` -- layered, maximal

**Stamen (inner ring):** `·` `∙` `•` -- dots in 1-1.5 cell circle
**Stem:** `┊` (e<0.3), `│` (e<0.6), `┃` (e>=0.6)

## Drawing Algorithm (7 steps)

```
1. Draw stem downward from center (stem_length rows)
   - char = stem_char(energy), color = green HSL(120, 0.35, 0.30)

2. Place center glyph
   - select_center_glyph(energy), lightened by color_stops

3. Place outer petals radially
   - N = petal_count, evenly spaced at 360/N degrees
   - Position: cx + cos(angle) * radius * 1.8, cy + sin(angle) * radius
   - The 1.8x on x compensates for terminal chars being ~2:1 height:width
   - Quantize to grid, place petal char at each position

4. Place stamen ring (if stamen_alpha > 0)
   - 4-6 dot chars in circle of radius 1.0-1.5 around center

5. Place inner petal layer (if energy > 0.75)
   - Second ring at 0.6 * radius, rotated TAU/(2*petal_count)
   - Darker color than outer petals

6. Pollen scatter (if energy >= 0.90)
   - Random `·` dots within radius, sparse, shift_hue from base

7. Apply color gradient
   - Outer petals: base color
   - Inner petals: darken(base, 15)
   - Stamen: triadic hue offset +120 degrees from petal hue
   - Alternate petals: lighten(base, 15) for depth
```

## Color Strategy

```
saturation(e) = 0.30 + e * 0.55                  // 0.30 -> 0.85
petal_lightness(e) = 0.30 + e * 0.30             // 0.30 -> 0.60
stamen_hue = (base_hue + 120.0).rem_euclid(360.0) // triadic
stem_color = hsl_to_rgb(120.0, 0.35, 0.25 + e * 0.15)
```

Flower type presets (base hue):
- Rose: H=0 (shifts toward H=355 at high energy)
- Sunflower: H=45 (shifts toward H=42)
- Lavender: H=280 (shifts toward H=278)
- Wildflower: random H in [30, 330]

## Sharp Insights (from haiku filtering)

1. **Stem and bloom grow on different energy curves.** Stem is nearly linear
   with energy (structural). Bloom accelerates 0.4-0.7 then plateaus (attraction).
   Don't couple them.

2. **Skip TreePen for flowers entirely.** Box-drawing junction chars (├┤╭╮)
   produce tree-branch shapes. Flowers need radial Unicode glyphs placed directly.

3. **Stamen hue is type-specific.** Not a fixed offset -- roses get golden stamen
   (H=45), lavender gets near-white. Triadic offset (+120) is the starting formula,
   individual presets can override.

4. **Never call tip() on flower pens.** The `╷` endpoint marker is tree vocabulary.

5. **Aspect ratio matters.** Terminal chars are ~2:1 height:width. Without the 1.8x
   correction on x-coordinates, "circles" render as tall ovals.

6. **Bloom/stem ratio stays 0.5-1.2** for natural proportions. Prevents lollipop
   (huge bloom, thin stem) or root (thick stem, tiny bloom) silhouettes.

## What to SKIP

- **Mirror-pen bilateral approach**: Center glyph gets overwritten by second pen pass
- **Braille chars for petals**: Double-height in many terminal fonts, inconsistent
- **petal_spread_degrees variable**: Sub-resolution at radius < 3, not worth tracking
- **Existing stamp pattern arrays** (draw_flower patterns 0-4): Not scalable by radius
- **.max(3) clamp on petal_count**: Let it be 0-2 at low energy (a bud HAS no petals)

## Open Decisions

1. **Aspect ratio constant**: 1.8x or 2.0x? Needs visual testing.
2. **Rotation offset**: Fixed per flower type, or random per instance?
3. **Coexist or replace**: Keep old `grow_flower_spiral` / `draw_flower` or replace?
4. **Energy assignment in scatter modes**: Who gives each flower its energy value?
5. **Inner layer guard**: Skip inner petal layer if radius < 2? (too small to see)
6. **Stamen formula depth**: Simple +120 for all, or per-flower-type override table?

## Function Signature

```rust
pub fn grow_flower(
    grid: &mut Grid,
    cx: usize, cy: usize,
    energy: f32,       // 0.0-1.0
    color: Color,      // base petal color
    rng: &mut StdRng,
)
```

Could also become a trait like TreeDrawer if we want flower "species" with
different petal shapes, but start with one function and extract later.
