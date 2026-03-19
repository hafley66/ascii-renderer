# Quick Character Connection Matrix

## Summary Table: Character → Valid Exits → Valid Followers

| Char | Exits | Follow Up | Follow Down | Follow Left | Follow Right | Follow UL | Follow UR | Follow DL | Follow DR |
|------|-------|-----------|------------|-------------|--------------|-----------|-----------|-----------|-----------|
| `│` | U,D | `│┃┬┴├┤┼╷` | `│┃┬┴├┤┼╷` | — | — | — | — | — | — |
| `─` | L,R | — | — | `─━├┤┼╴·` | `─━├┤┼╶·` | — | — | — | — |
| `╱` | UR,DL | — | — | `╱╲│─·` | — | `╱╲│─·` | `╱╲│─·` | `╱╲│─·` | — |
| `╲` | UL,DR | — | — | — | `╱╲│─·` | `╱╲│─·` | — | — | `╱╲│─·` |
| `⌿` | UR,DL | — | — | `⌿╱·◇` | — | `⌿╱·◇` | `⌿╱·◇` | — | — |
| `⍀` | UL,DR | — | — | — | `⍀╲·◇` | `⍀╲·◇` | — | — | `⍀╲·◇` |
| `├` | U,D,R | `│┃├┤┼` | `│┃├┤┼` | — | `─━├┤┼` | — | — | — | — |
| `┤` | U,D,L | `│┃├┤┼` | `│┃├┤┼` | `─━├┤┼` | — | — | — | — | — |
| `┬` | L,R,D | — | `│┃├┤┼` | `─━├┤┼` | `─━├┤┼` | — | — | — | — |
| `┴` | L,R,U | — | `│┃├┤┼` | `─━├┤┼` | `─━├┤┼` | — | — | — | — |
| `┼` | U,D,L,R | `│┃├┤┼` | `│┃├┤┼` | `─━├┤┼` | `─━├┤┼` | — | — | — | — |
| `╭` | D,R | `│┃├┤┼·` | — | — | `─━├┤┼·` | — | — | — | — |
| `╮` | D,L | `│┃├┤┼·` | — | `─━├┤┼·` | — | — | — | — | — |
| `╰` | U,R | `│┃├┤┼·` | — | — | `─━├┤┼·` | — | — | — | — |
| `╯` | U,L | `│┃├┤┼·` | — | `─━├┤┼·` | — | — | — | — | — |
| `╷` | U | `·•●◆◇` | — | — | — | — | — | — | — |
| `╵` | D | — | `·•●◆◇` | — | — | — | — | — | — |
| `╴` | L | — | — | `·•●◆◇` | — | — | — | — | — |
| `╶` | R | — | — | — | `·•●◆◇` | — | — | — | — |
| `∿` | U,D,L,R | `∿∽~·◇` | `∿∽~·◇` | `∿∽~─◇` | `∿∽~─◇` | — | — | — | — |
| `~` | L,R | — | — | `~∿─◇` | `~∿─◇` | — | — | — | — |
| `⌠` | D,R | `│∫·` | — | — | `⌠─·` | — | — | — | — |
| `∫` | U,D | `⌠│·` | `⌡│·` | — | — | — | — | — | — |
| `⌡` | U,L | `∫│·` | — | `⌡─·` | — | — | — | — | — |
| `▌` | U,D | `▌▍│┃` | `▌▍│┃` | — | — | — | — | — | — |
| `▐` | U,D | `▐▎│┃` | `▐▎│┃` | — | — | — | — | — | — |
| `▀` | L,R | — | — | `▀═─` | `▀═─` | — | — | — | — |
| `▄` | L,R | — | — | `▄═─` | `▄═─` | — | — | — | — |
| `█` | U,D,L,R | `█·│` | `█·│` | `█·─` | `█·─` | — | — | — | — |
| `·` | NONE | — | — | — | — | — | — | — | — |
| `•` | NONE | — | — | — | — | — | — | — | — |
| `●` | NONE | — | — | — | — | — | — | — | — |
| `◆` | NONE | — | — | — | — | — | — | — | — |

Key: Blank cells (`—`) = no valid followers in that direction

---

## Quick Lookup Patterns

### "I'm drawing a vertical stem going UP, what can follow?"
```
From: │ (up exit)
Followers: │ ┃ ┬ ┴ ├ ┤ ┼ ╷ ·

Pick one:
  │ = continue straight
  ├ or ┤ = branch left or right
  ╷ = prepare endpoint
  · = terminate here
```

### "I'm drawing a horizontal branch going RIGHT, what can follow?"
```
From: ─ (right exit)
Followers: ─ ━ ├ ┤ ┼ ╴ · ╭ ╮ ╰ ╯

Pick one:
  ─ = continue straight
  ╴ = prepare endpoint
  · = terminate
  ╭ ╰ = turn down or up
```

### "I'm at a fork (├), what can exit?"
```
From: ├ (up, down, right exits)
UP:    │ ┃ ├ ┤ ┼ ╷ ·
DOWN:  │ ┃ ├ ┤ ┼ ╷ ·
RIGHT: ─ ━ ├ ┤ ┼ ╴ ·
```

### "I want roots! What's a good pattern?"
```
Vertical trunk down:
│ → │ → │ → ├ (fork at base)

Left root:
├ → ⌿ → ⌿ → ·

Right root:
├ → ⍀ → ⍀ → ·

Full root system:
    │
    │
    ├ ← base fork
   ╱ ╲
  ⌿   ⍀
 /     \
·       ·
```

### "I want a drooping vine with fruit!"
```
│ → ╯ → ~ → ~ → ∿ → •

Or simpler:
─ → ╴ → ·

Or organic wave:
│ → ├ → ∿ → ∿ → • (hanging fruit)
```

### "I want a branching cluster"
```
      ╷ ← tip
      │
      ├ ← split
    ╱─┴─╲
   •     •
  (fruit) (fruit)
```

---

## Character Pick by Intent

### ENDPOINTS (Pick ONE when ready to stop drawing)
Use when branch length is done:
- `·` – thin leaf, small endpoint
- `•` – medium fruit/berry
- `●` – large apple/fruit
- `◆` – star-shaped, complex fruit
- `◇` – delicate, light fruit
- `꘎` – single leaf (Unicode)

### CONTINUATIONS (What to draw while going in a direction)
- **UP or DOWN**: `│` (thin) or `┃` (thick)
- **LEFT or RIGHT**: `─` (thin) or `━` (thick)
- **UpRight or DownLeft**: `╱` (standard) or `⌿` (root-like)
- **UpLeft or DownRight**: `╲` (standard) or `⍀` (root-like)
- **WAVY VINE**: `∿` (multi-dir) or `~` (horizontal wave)

### JUNCTIONS (When splitting into multiple paths)
- **Trunk splits right**: `├`
- **Trunk splits left**: `┤`
- **Branch splits down**: `┬`
- **Branch splits up**: `┴`
- **4-way split**: `┼`

### CORNERS (When changing direction 90°)
- **UP→RIGHT**: `╰`
- **UP→LEFT**: `╯`
- **DOWN→RIGHT**: `╭`
- **DOWN→LEFT**: `╮`

### ROOTS (Spreading at base)
- Pattern: `│ → ├ → ⌿ → ·` (left)
- Pattern: `│ → ┤ → ⍀ → ·` (right)
- Alternative: `│ → ┼ → /⌿ /⍀ \·` (both)

### THICK STRUCTURES (Multi-cell-wide)
- `█` (full block) = heavy junction, knot
- `▓` (dark shade) = softer knot, aging
- `▌` `▐` = left/right half blocks (vertical)
- `▀` `▄` = top/bottom half blocks (horizontal)
- `┃` `━` = double-line thick variants

### ORGANIC/NATURAL (Vines, tendrils)
- `∿` (sine wave) = organic multi-dir vine
- `~` (tilde) = simple horizontal wave
- `⌠` `∫` `⌡` (integral family) = complex split clusters
- `⌿` `⍀` (root diagonals) = spreading organics

---

## Termination Rules

**Rule 1: Endpoints block all further drawing**
Once you place `· • ● ◆ ◇`, you cannot draw from those cells. They are final.

**Rule 2: Use stubs (╷ ╵ ╴ ╶) to prepare endpoints**
Don't jump directly from `│` to `·`. Go: `│ → ╷ → ·` (better visual flow).

**Rule 3: Combine branching with terminators**
For fruit clusters: `├ → LEFT: ● RIGHT: ●` (fork, then add fruits on left/right).

**Rule 4: Waves can chain**
`∿ → ∿ → ∿ → •` is valid (waves continue until a terminator).

**Rule 5: Thick blocks are junctions**
`█` acts like `┼` (all 4 directions). Use when you need a heavy knot/split.

---

## Visual Continuity Checklist

Before placing a character:

- [ ] **Previous char has an exit in this direction?**
  - Example: `│` has Up/Down exits. Can't go Left from it.

- [ ] **Next char can accept entry from opposite direction?**
  - Example: Going Right into a char → that char must have a Left exit.

- [ ] **Is next char a terminator?**
  - If yes (`· • ●`), then this is the final cell. No further draws after.

- [ ] **Does character family match?**
  - Don't mix thick (█) with thin (│) without a junction.
  - Don't mix wavy (∿) with straight (─) unless intentional.

- [ ] **Does depth suggest fruit size?**
  - Shallow branch → bigger fruit (●)
  - Deep branch → tiny fruit (·)

---

## Copy-Paste Patterns for Rust

### Simple vertical stem with fruit:
```rust
tset_over(grid, x, y,     '│', color);
tset_over(grid, x, y - 1, '╷', color);
tset_over(grid, x, y - 2, '•', color);
```

### Fork with left/right branches:
```rust
tset_over(grid, x, y,     '├', color);
tset_over(grid, x - 1, y, '─', color);
tset_over(grid, x + 1, y, '─', color);
tset_over(grid, x - 2, y, '●', color);  // left fruit
tset_over(grid, x + 2, y, '●', color);  // right fruit
```

### Drooping vine with wave:
```rust
tset_over(grid, x, y,     '╯', color);
tset_over(grid, x - 1, y, '─', color);
tset_over(grid, x - 2, y, '~', color);
tset_over(grid, x - 3, y, '~', color);
tset_over(grid, x - 4, y, '•', color);  // hanging fruit
```

### Root system:
```rust
tset_over(grid, x, y,     '├', color);
tset_over(grid, x - 1, y, '⌿', color);  // left root
tset_over(grid, x + 1, y, '⍀', color);  // right root
tset_over(grid, x - 2, y, '·', color);
tset_over(grid, x + 2, y, '·', color);
```

---

## When to Use Each Character

| Character | Tree Part | Density | Aesthetic | Notes |
|-----------|-----------|---------|-----------|-------|
| `│` | Main trunk, branches | High | Standard | Thin, readable |
| `┃` | Thick trunk base | Med | Heavy | Double-line, robust |
| `─` | Horizontal branches | High | Standard | Thin, clean |
| `━` | Thick branches | Med | Heavy | For wide trees |
| `├` `┤` | Main splits | Med | Formal | Clear junction |
| `┬` `┴` | Branch splits | Low | Formal | Secondary junctions |
| `╱` `╲` | Diagonal branches | Low | Dynamic | Angled, asymmetric |
| `⌿` `⍀` | Root spread | Low | Organic | Natural roots |
| `∿` `~` | Vines, droops | Very Low | Organic | Casual, natural |
| `╷` | Branch tip prep | Low | Organic | Better than direct │→· |
| `•` `●` `◆` | Fruit, flowers | Very Low | Decorative | Dense areas look busy |
| `·` | Leaf, tiny nodes | Any | Minimal | Safe terminator |
| `█` `▓` | Heavy knots, old wood | Low | Gnarled | Use sparingly |
