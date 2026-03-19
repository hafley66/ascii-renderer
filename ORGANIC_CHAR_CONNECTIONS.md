# Organic Character Connection Reference

This document defines valid character connections for ASCII tree rendering, covering organic/natural shapes beyond standard box-drawing characters.

Format: Each character lists its **exits** (directions it can travel FROM), followed by **valid followers** (what characters can come NEXT in each direction).

Directions: `Up, Down, Left, Right, UpLeft, UpRight, DownLeft, DownRight`

---

## VERTICAL STEMS & TRUNKS

### `│` (thin vertical line)
- **Exits**: Up, Down
- **Valid followers**:
  - `Up` → `│, ┃, ┬, ┴, ├, ┤, ┼, ╷, ╵, ·, •, ●` (vertical continuations, junctions, endpoints, terminators)
  - `Down` → `│, ┃, ┬, ┴, ├, ┤, ┼, ╷, ╵, ·, •, ●`

### `┃` (thick vertical line)
- **Exits**: Up, Down
- **Valid followers**:
  - `Up` → `│, ┃, ┬, ┴, ├, ┤, ┼, ╷, ·, •, ●`
  - `Down` → `│, ┃, ┬, ┴, ├, ┤, ┼, ╷, ·, •, ●`

### `╷` (stub pointing up — endpoint)
- **Exits**: Up
- **Valid followers**:
  - `Up` → `·, •, ●, ◆, ◇` (terminators, fruit/buds)
- **Use case**: Terminal branch tip before fruit/leaf cluster

### `╵` (stub pointing down)
- **Exits**: Down
- **Valid followers**:
  - `Down` → `·, •, ●, ░, ▒` (terminators, ground decorations)

---

## HORIZONTAL BRANCHES

### `─` (thin horizontal line)
- **Exits**: Left, Right
- **Valid followers**:
  - `Left` → `─, ━, ├, ┤, ┬, ┴, ┼, ╭, ╮, ╰, ╯, ·` (continuations, junctions, turns)
  - `Right` → `─, ━, ├, ┤, ┬, ┴, ┼, ╭, ╮, ╰, ╯, ·`

### `━` (thick horizontal line)
- **Exits**: Left, Right
- **Valid followers**:
  - `Left` → `─, ━, ├, ┤, ┬, ┴, ┼, ╭, ╮` (continuations, junctions)
  - `Right` → `─, ━, ├, ┤, ┬, ┴, ┼, ╭, ╮`

### `╴` (stub pointing left)
- **Exits**: Left
- **Valid followers**:
  - `Left` → `·, •, ●, ◆, ◇` (terminators)

### `╶` (stub pointing right)
- **Exits**: Right
- **Valid followers**:
  - `Right` → `·, •, ●, ◆, ◇` (terminators)

---

## DIAGONAL TRUNKS & BRANCHES

### `╱` (forward slash — rising diagonal)
- **Exits**: UpRight, DownLeft
- **Valid followers**:
  - `UpRight` → `╱, ╲, │, ─, ├, ┤, ┬, ┴, ┼, ╭, ╮, ·, •` (diagonals, verticals, branches)
  - `DownLeft` → `╱, ╲, │, ─, ├, ┤, ┬, ┴, ┼, ╯, ╰, ·, •`

### `╲` (backslash — falling diagonal)
- **Exits**: UpLeft, DownRight
- **Valid followers**:
  - `UpLeft` → `╱, ╲, │, ─, ├, ┤, ┬, ┴, ┼, ╮, ╯, ·, •` (diagonals, verticals, branches)
  - `DownRight` → `╱, ╲, │, ─, ├, ┤, ┬, ┴, ┼, ╭, ╰, ·, •`

---

## TURNS & CORNERS

### `╭` (curve: Down + Right)
- **Exits**: Down, Right
- **Valid followers**:
  - `Down` → `│, ┃, ├, ┤, ┼, ╷, ·, •` (vertical continuations)
  - `Right` → `─, ━, ├, ┤, ┼, ╴, ·, •` (horizontal continuations)

### `╮` (curve: Down + Left)
- **Exits**: Down, Left
- **Valid followers**:
  - `Down` → `│, ┃, ├, ┤, ┼, ╷, ·, •`
  - `Left` → `─, ━, ├, ┤, ┼, ╶, ·, •`

### `╰` (curve: Up + Right)
- **Exits**: Up, Right
- **Valid followers**:
  - `Up` → `│, ┃, ├, ┤, ┼, ╷, ·, •`
  - `Right` → `─, ━, ├, ┤, ┼, ╴, ·, •`

### `╯` (curve: Up + Left)
- **Exits**: Up, Left
- **Valid followers**:
  - `Up` → `│, ┃, ├, ┤, ┼, ╷, ·, •`
  - `Left` → `─, ━, ├, ┤, ┼, ╶, ·, •`

---

## JUNCTIONS & SPLITS

### `├` (T-junction: Up, Down, Right)
- **Exits**: Up, Down, Right
- **Valid followers**:
  - `Up` → `│, ┃, ├, ┤, ┼, ╷, ·` (vertical)
  - `Down` → `│, ┃, ├, ┤, ┼, ╷, ·`
  - `Right` → `─, ━, ├, ┤, ┼, ╴, ·` (horizontal branch)

### `┤` (T-junction: Up, Down, Left)
- **Exits**: Up, Down, Left
- **Valid followers**:
  - `Up` → `│, ┃, ├, ┤, ┼, ╷, ·`
  - `Down` → `│, ┃, ├, ┤, ┼, ╷, ·`
  - `Left` → `─, ━, ├, ┤, ┼, ╶, ·`

### `┬` (T-junction: Left, Right, Down)
- **Exits**: Left, Right, Down
- **Valid followers**:
  - `Left` → `─, ━, ├, ┤, ┼, ╶, ·`
  - `Right` → `─, ━, ├, ┤, ┼, ╴, ·`
  - `Down` → `│, ┃, ├, ┤, ┼, ╷, ·`

### `┴` (T-junction: Left, Right, Up)
- **Exits**: Left, Right, Up
- **Valid followers**:
  - `Left` → `─, ━, ├, ┤, ┼, ╶, ·`
  - `Right` → `─, ━, ├, ┤, ┼, ╴, ·`
  - `Up` → `│, ┃, ├, ┤, ┼, ╷, ·`

### `┼` (cross: Up, Down, Left, Right)
- **Exits**: Up, Down, Left, Right
- **Valid followers**:
  - `Up` → `│, ┃, ├, ┤, ┼, ╷, ·`
  - `Down` → `│, ┃, ├, ┤, ┼, ╷, ·`
  - `Left` → `─, ━, ├, ┤, ┼, ╶, ·`
  - `Right` → `─, ━, ├, ┤, ┼, ╴, ·`

### `╋` (thick cross)
- **Exits**: Up, Down, Left, Right
- **Valid followers**: Same as `┼`

### `┣` (heavy T-junction: Up, Down, Right)
- **Exits**: Up, Down, Right
- **Valid followers**: Same as `├`

### `┫` (heavy T-junction: Up, Down, Left)
- **Exits**: Up, Down, Left
- **Valid followers**: Same as `┤`

### `┳` (heavy T-junction: Left, Right, Down)
- **Exits**: Left, Right, Down
- **Valid followers**: Same as `┬`

### `┻` (heavy T-junction: Left, Right, Up)
- **Exits**: Left, Right, Up
- **Valid followers**: Same as `┴`

---

## TERMINATORS & BUDS

### `·` (thin dot)
- **Exits**: None (endpoint)
- **Valid followers**: None
- **Use case**: Leaf node, tiny fruit, branch endpoint

### `•` (medium bullet)
- **Exits**: None (endpoint)
- **Valid followers**: None
- **Use case**: Fruit, berry, leaf cluster center

### `●` (filled circle)
- **Exits**: None (endpoint)
- **Valid followers**: None
- **Use case**: Apple, fruit, large berry

### `◆` (filled diamond)
- **Exits**: None (endpoint)
- **Valid followers**: None
- **Use case**: Star-shaped fruit, gem-like berry

### `◇` (hollow diamond)
- **Exits**: None (endpoint)
- **Valid followers**: None
- **Use case**: Delicate fruit, lighter berry

### `○` (hollow circle)
- **Exits**: None (endpoint)
- **Valid followers**: None
- **Use case**: Flower bud, light fruit

### `◎` (bullseye)
- **Exits**: None (endpoint)
- **Valid followers**: None
- **Use case**: Complex flower center, compound fruit

---

## ORGANIC LEAF/FOLIAGE CHARACTERS

### `*` (asterisk)
- **Exits**: None (endpoint)
- **Valid followers**: None
- **Use case**: Sparse leaves, twiggy cluster

### `✱` (six-petaled flower)
- **Exits**: None (endpoint)
- **Valid followers**: None
- **Use case**: Flower on branch

### `✲` (four-petaled flower)
- **Exits**: None (endpoint)
- **Valid followers**: None
- **Use case**: Simple flower

### `꘎` (leaf symbol — if available in Unicode)
- **Exits**: None (endpoint)
- **Valid followers**: None
- **Use case**: Single leaf

---

## BRACKETED SHAPES (terminal clusters)

### `( )` paired brackets
- **Exits**: None (as a unit)
- **Valid followers**: None
- **Use case**: Drooping leaves, calyx shape

### `{ }` braces
- **Exits**: None
- **Valid followers**: None
- **Use case**: Dense foliage node

### `[ ]` square brackets
- **Exits**: None
- **Valid followers**: None
- **Use case**: Structured leaf arrangement

---

## WAVE & ORGANIC LINES

### `∿` (sine wave)
- **Exits**: Left, Right, Up, Down (wavy — treat as multi-directional line)
- **Valid followers**:
  - `Left` → `∿, ∽, ~, ─, ◇` (continuation, transition, terminator)
  - `Right` → `∿, ∽, ~, ─, ◇`
  - `Up` → `∿, ∽, ~, │, ·` (organic curves)
  - `Down` → `∿, ∽, ~, │, ·`
- **Use case**: Vine, tendril, organic branch

### `∽` (sine curve variation)
- **Exits**: Left, Right, Up, Down
- **Valid followers**: Same as `∿`

### `~` (tilde — simple wave)
- **Exits**: Left, Right (horizontal wave)
- **Valid followers**:
  - `Left` → `~, ∿, ─, ◇` (wave continuations)
  - `Right` → `~, ∿, ─, ◇`
- **Use case**: Grass, droop, simple vine

### `≈` (wavy equals)
- **Exits**: Left, Right
- **Valid followers**:
  - `Left` → `≈, ~, ∿, ─` (wave family)
  - `Right` → `≈, ~, ∿, ─`
- **Use case**: Double-line vine, thicker wave

---

## BLOCK ELEMENTS (thick trunks, roots)

### `▌` (left half block)
- **Exits**: Up, Down
- **Valid followers**:
  - `Up` → `▌, ▍, │, ┃, ╷` (thick continuations)
  - `Down` → `▌, ▍, │, ┃, ╷`
- **Use case**: Thick trunk, right-side taper

### `▐` (right half block)
- **Exits**: Up, Down
- **Valid followers**:
  - `Up` → `▐, ▎, │, ┃, ╷`
  - `Down` → `▐, ▎, │, ┃, ╷`
- **Use case**: Thick trunk, left-side taper

### `▀` (upper half block)
- **Exits**: Left, Right
- **Valid followers**:
  - `Left` → `▀, ═, ─` (horizontal continuations)
  - `Right` → `▀, ═, ─`
- **Use case**: Horizontal branch top half, thick base

### `▄` (lower half block)
- **Exits**: Left, Right
- **Valid followers**:
  - `Left` → `▄, ═, ─`
  - `Right` → `▄, ═, ─`
- **Use case**: Horizontal branch bottom half, roots

### `█` (full block)
- **Exits**: Up, Down, Left, Right
- **Valid followers**: Any adjacent direction (acts like solid node)
  - `Up` → `█, │, ╷, ·`
  - `Down` → `█, │, ╷, ·`
  - `Left` → `█, ─, ╶, ·`
  - `Right` → `█, ─, ╴, ·`
- **Use case**: Knot, heavy junction, thick branch node

### `▓` (dark shade)
- **Exits**: Up, Down, Left, Right
- **Valid followers**: Same as `█` (slightly less aggressive)
- **Use case**: Softer knot, decay, aged wood

---

## SPECIAL BRANCHING (directional)

### `⌠` (integral top)
- **Exits**: Down, Right
- **Valid followers**:
  - `Down` → `∫, │, ⌡` (continuation into integral)
  - `Right` → `⌠, ─, ·` (horizontal branch)
- **Use case**: Branching start, cluster top

### `∫` (integral symbol)
- **Exits**: Up, Down
- **Valid followers**:
  - `Up` → `⌠, │, ·`
  - `Down` → `⌡, │, ·`
- **Use case**: Multi-level branch connector

### `⌡` (integral bottom)
- **Exits**: Up, Left
- **Valid followers**:
  - `Up` → `∫, │, ⌠`
  - `Left` → `⌡, ─, ·`
- **Use case**: Branching end, cluster bottom

---

## ROOT & SPREADING CHARACTERS

### `⌿` (root-like diagonal)
- **Exits**: UpRight, DownLeft
- **Valid followers**:
  - `UpRight` → `⌿, ╱, ·, ◇`
  - `DownLeft` → `⌿, ╱, ·, ◇`
- **Use case**: Spreading root, underground branch

### `⍀` (alternate root diagonal)
- **Exits**: UpLeft, DownRight
- **Valid followers**:
  - `UpLeft` → `⍀, ╲, ·, ◇`
  - `DownRight` → `⍀, ╲, ·, ◇`
- **Use case**: Spreading root (other direction)

---

## GROUND & GRASS (may terminate branches or appear under trunks)

### `░` (light shade)
- **Exits**: None (ground fill)
- **Valid followers**: None
- **Use case**: Grass, soil

### `▒` (medium shade)
- **Exits**: None
- **Valid followers**: None
- **Use case**: Denser grass

### `▓` (dark shade)
- **Exits**: Up, Down, Left, Right (can support structures)
- **Valid followers**: Any direction
- **Use case**: Solid ground, heavy soil, root base

---

## TERMINATION STRATEGIES

When a branch/vine ends, use these sequences:

**Simple leaf endpoint:**
```
─ → ·
│ → ╷ → ·
╱ → ·
```

**Flower/fruit:**
```
│ → ╷ → •
─ → ╴ → ●
```

**Branching terminal (splits into multiple fruits):**
```
│ → ┬ → (Left: •) (Right: •) (Down: •)
```

**Drooping vine:**
```
│ → ╯ → ~ → ~ → ·
```

**Root spreading:**
```
│ → ├ → ⌿ → ·  (left root)
│ → ┤ → ⍀ → ·  (right root)
```

---

## CONTINUITY RULES

1. **Straight continuations** preserve direction:
   - Vertical: `│` → `│` → `│` → `╷`
   - Horizontal: `─` → `─` → `─` → `·`
   - Diagonal: `╱` → `╱` → `╱` → `·`

2. **Turns** use corner characters:
   - Up + Right: `╰` (corner) OR `│` then `├` then horizontal
   - Down + Right: `╭` (corner) OR `├` then horizontal

3. **Branches** use junction characters:
   - Main stem continues, branch splits: `├`, `┤`, `┬`, `┴`, `┼`

4. **Organic curves** use wave characters:
   - `∿` or `~` for vines, tendrils
   - Can chain multiple wavy segments before terminating

5. **Endpoints** are always LEAF characters (not drawable-to):
   - `·, •, ●, ◆, ◇, ○, *` (no exits defined)
   - Once placed, these block further drawing in any direction

---

## Rust Implementation Pattern

```rust
/// Expanded organic character exits
pub fn char_exits_organic(ch: char) -> &'static [MoveDir] {
    use MoveDir::*;
    match ch {
        // Standard box-drawing (from char_exits)
        '│' | '┃' => &[Up, Down],
        '─' | '━' => &[Left, Right],
        '╱' => &[UpRight, DownLeft],
        '╲' => &[UpLeft, DownRight],

        // Organic/natural additions
        '∿' | '∽' => &[Up, Down, Left, Right],  // wavy lines
        '~' | '≈' => &[Left, Right],             // horizontal waves
        '⌿' => &[UpRight, DownLeft],             // root diagonals
        '⍀' => &[UpLeft, DownRight],
        '▌' | '▐' => &[Up, Down],                // half blocks (thick)
        '▀' | '▄' => &[Left, Right],             // half blocks (horizontal)
        '█' | '▓' => &[Up, Down, Left, Right],   // full blocks (heavy nodes)

        // Branching organics
        '⌠' => &[Down, Right],
        '∫' => &[Up, Down],
        '⌡' => &[Up, Left],

        // Terminators (no exits)
        '·' | '•' | '●' | '◆' | '◇' | '○' | '◎' | '*' => &[],

        // ... existing characters ...
        _ => &[],
    }
}

/// Valid followers in each direction (simplified matcher)
pub fn can_follow(from: char, direction: MoveDir, to: char) -> bool {
    // Check if 'from' has an exit in 'direction'
    if !char_exits_organic(from).contains(&direction) {
        return false;
    }

    // Check if 'to' can accept entry from opposite direction
    let entry_dir = opposite(direction);
    if char_exits_organic(to).is_empty() {
        // Terminal character (leaf/fruit) — accepts entry from any direction
        return matches!(to, '·' | '•' | '●' | '◆' | '◇' | '○' | '◎' | '*');
    }

    char_exits_organic(to).contains(&opposite(entry_dir))
}
```

---

## Usage Examples

**Growing a vine with a fruit:**
```
Start at (5, 10), move Right
 5,10: ─
 6,10: ─
 7,10: ╴ (stub, terminal direction)
 Then add adjacent:
 8,10: •  (fruit in can_follow check)
```

**Growing a branching tree:**
```
 5,10: │ (trunk)
 5, 9: ├ (split junction)
 4, 9: ─ (left branch)
 6, 9: ─ (right branch)
 4, 9 endpoint: ·
 6, 9 endpoint: •
```

**Growing a drooping vine:**
```
 5,10: │
 5, 9: ╯ (bend left)
 4, 9: ─
 3, 9: ~
 2, 9: ∿
 1, 9: ·
```

**Growing roots at base:**
```
 5,10: │ (main trunk in ground)
 5,11: ├ (split at base)
 4,11: ⌿ (left root diagonal)
 3,12: ⌿ (continue)
 2,13: ·  (root endpoint)

 6,11: ⍀ (right root diagonal)
 7,12: ⍀
 8,13: ·
```
