# Character Connections for Tree Rendering

A practical guide to understanding which ASCII characters can validly connect to which other characters when building organic/natural ASCII trees.

## Files in This Reference Set

### 1. **ORGANIC_CHAR_CONNECTIONS.md** (Comprehensive)
Complete reference for every organic character, its exits, valid followers, and use cases.

**Use this when**:
- Designing a new tree type and need to know all options
- Debugging why a branch looks disconnected
- Understanding the full character vocabulary

**Key sections**:
- Vertical stems & trunks
- Horizontal branches
- Diagonal trunks & branches
- Turns, corners, junctions
- Terminators (buds, fruit, leaves)
- Organic leaf/foliage characters
- Bracketed shapes
- Wave & organic lines
- Block elements (thick trunks, roots)
- Special branching characters
- Root & spreading characters
- Termination strategies & continuity rules

### 2. **QUICK_CHAR_MATRIX.md** (At-a-Glance)
Quick lookup tables and patterns for fast decision-making while coding.

**Use this when**:
- You need an answer in 10 seconds
- Building pattern logic and need quick validation
- Searching for "what can follow X in direction Y"

**Key sections**:
- Summary table: Character → Valid Exits → Valid Followers
- Quick lookup patterns by direction
- Character pick by intent
- Termination rules
- Visual continuity checklist

### 3. **TREE_PATTERN_EXAMPLES.md** (Practical)
Real, working patterns you can copy/paste and adapt.

**Use this when**:
- Building a new tree type and want proven patterns
- Stuck and need inspiration
- Testing character connections in practice

**Key sections**:
- 11 complete tree patterns with code
- Pattern composition rules
- Testing checklist
- Copy-paste template

### 4. **ORGANIC_CHAR_BUILDER.rs** (Implementation)
Rust code ready to integrate into sprites.rs.

**Use this when**:
- Integrating expanded character support into the codebase
- Implementing can_follow() validator
- Adding new character families (is_terminator, is_wave, etc.)

**Key sections**:
- Expanded char_exits() arms
- can_follow_organic() function
- Character family predicates
- Direction-based glyph selection
- Fruit/leaf placement rules
- Integration checklist

---

## The Core Concept

A character has **exits** (directions it can travel from) and **entry points** (directions it can accept).

When drawing a path:
```
Previous Cell (x1, y1)
    │
    ├─ Can it exit in direction D?
    │
    Next Cell (x2, y2) at distance D
    │
    └─ Can it enter from opposite(D)?
```

If both answers are YES, the connection is valid and visually continuous.

**Example**: Can `│` connect LEFT to `─`?
```
│ has exits: [Up, Down]      (no Left)
  → Can't exit Left
  → Connection INVALID
```

**Example**: Can `│` connect UP to `│`?
```
│ has exits: [Up, Down]      (yes Up)
  → Can exit Up ✓
│ has exits: [Up, Down]      (has Down, opposite of Up)
  → Can enter from Up ✓
  → Connection VALID
```

---

## When to Use Each Character Family

### STRUCTURAL (Building the tree frame)
- `│` – Vertical stem (thin, standard)
- `─` – Horizontal branch (thin, standard)
- `╱` `╲` – Diagonal branches (sharp, angular)
- `├` `┤` `┬` `┴` `┼` – Junctions (where splits happen)
- `╭` `╮` `╰` `╯` – Corners (90-degree turns)

### ORGANIC (Natural, flowing forms)
- `∿` `∽` – Sine waves (multi-directional, vines)
- `~` – Tilde wave (horizontal, droops)
- `⌿` `⍀` – Root diagonals (spreading outward)
- `⌠` `∫` `⌡` – Integral family (complex splits)

### TERMINALS (End of line — no further drawing)
- `·` – Thin dot (leaf, endpoint)
- `•` – Bullet (medium fruit)
- `●` – Filled circle (large fruit)
- `◆` – Diamond (star-shaped fruit)
- `◇` – Hollow diamond (delicate)

### THICK/HEAVY (Multi-cell-wide or weighted)
- `┃` – Double-line vertical
- `━` – Double-line horizontal
- `█` – Full block (heavy knot)
- `▓` – Dark shade (softer knot)
- `▌` `▐` – Half blocks vertical (taper)
- `▀` `▄` – Half blocks horizontal (taper)

---

## Visual Continuity Checklist

Before placing a character in a cell:

1. **Source has exit in target direction?**
   - Check char_exits(previous_char) contains direction D

2. **Target can accept entry from opposite?**
   - Check char_exits(next_char) contains opposite(D)
   - OR next_char is a terminator (· • ●)

3. **Is next a terminator?**
   - If yes (· • ●), this is the final cell
   - No further drawing from here

4. **Does character family fit?**
   - Don't mix thick (┃) with thin (│) at same junction
   - Don't mix wave (∿) with straight (─) unless intentional
   - Root chars (⌿) should only appear at base spreading

---

## Implementation Roadmap

### Phase 1: Expand char_exits()
```rust
// In sprites.rs, extend char_exits() match:
'∿' | '∽' => &[Up, Down, Left, Right],
'~' => &[Left, Right],
'⌿' => &[UpRight, DownLeft],
'⍀' => &[UpLeft, DownRight],
// ... (see ORGANIC_CHAR_BUILDER.rs)
```

### Phase 2: Add can_follow_organic()
```rust
pub fn can_follow_organic(from: char, dir: MoveDir, to: char) -> bool {
    // Check from→to connection validity
    // (see ORGANIC_CHAR_BUILDER.rs for full implementation)
}
```

### Phase 3: Add character predicates
```rust
is_terminator(ch), is_vertical_line(ch), is_organic_wave(ch), etc.
```

### Phase 4: Update tree growth functions
```rust
// In grow_tree() and variants:
// Use can_follow_organic() instead of hardcoding
// Use suggest_organic_glyph() for glyph selection
// Use fruit_for_depth() for size-based fruit
```

### Phase 5: Test & snapshot
```rust
// In tests/snapshot_modes.rs:
// Test each pattern type with snapshot asserts
// Verify organic chars chain correctly
```

---

## Common Patterns Quick-Ref

| Pattern | Characters | Use Case |
|---------|-----------|----------|
| Straight trunk | `│ → │ → ╷ → •` | Vertical growth, simple |
| Fork | `├ → LEFT: ─, RIGHT: ─` | Splitting branches |
| Droop | `╯ → ~ → ~ → •` | Hanging vine, drooping |
| Root system | `├ → ⌿ LEFT, ⍀ RIGHT` | Base spreading |
| Curved turn | `│ → ╰ → ─ → •` | Change direction 90° |
| Burst | `╋ → (8 directions radiate)` | Firework, cluster |
| Wave chain | `∿ → ∿ → ∿ → •` | Organic tendril |
| Fruit cluster | `┼ → (4 directions: fruit)` | Multi-fruit node |
| Thick base | `┃ → ┃ → ├ → thin` | Transition thick→thin |
| Spiral | Alternating `╱ │ ╲` | Twisted trunk |

---

## Troubleshooting

### "Branch looks disconnected or jagged"
- **Cause**: Character doesn't have an exit in the needed direction
- **Fix**: Choose a character from "Valid followers" table for your direction
- **Example**: Can't go LEFT from `│`. Use `├` and then `─` instead.

### "Tree overlaps itself oddly"
- **Cause**: Using `╱` or `╲` for both tree AND grass, hard to distinguish
- **Fix**: Use `⌿` or `⍀` for roots/spreading to visually separate
- **Example**: Roots should be ⌿ ⍀, not ╱ ╲

### "Fruit looks pasted on, not attached"
- **Cause**: Jumping directly `│ → •` without stub
- **Fix**: Use intermediate stub: `│ → ╷ → •`
- **Example**: `│` at (5,10) → `╷` at (5,9) → `•` at (5,8)

### "Too many junctions, tree looks busy"
- **Cause**: Overusing `├ ┤ ┬ ┴ ┼` at every branch
- **Fix**: Use simple horizontal `─` run, junction only at actual splits
- **Example**: `│ → ├ → ─ → ·` (one junction, clean)

### "Waves don't look organic"
- **Cause**: Single wave character `~`, looks too regular
- **Fix**: Chain multiple wave types: `∿ → ∿ → ~`
- **Example**: Tendril: `│ → ├ → ∿ → ∿ → ~ → •`

---

## Integration Status

### Currently in codebase (sprites.rs):
- ✓ `char_exits()` for box-drawing chars (│ ─ ╱ ╲ ├ ┤ ┬ ┴ ┼ ╭ ╮ ╰ ╯ ╷ ╵ ╴ ╶)
- ✓ `connect_glyph()` for turn selection
- ✓ `dir_glyph()` for direction-based chars
- ✓ `TreePen` struct for connected drawing
- ✓ Basic terminator chars (·, •)
- ✗ Organic characters (∿ ~ ⌿ ⍀ ⌠ ∫ ⌡)
- ✗ can_follow_organic() validator
- ✗ Character family predicates (is_wave, is_root, etc.)
- ✗ Fruit sizing by depth logic

### Next steps:
1. Copy ORGANIC_CHAR_BUILDER.rs arms into char_exits()
2. Implement can_follow_organic()
3. Add character predicates
4. Update tree growth functions to use validators
5. Add snapshot tests for each pattern type

---

## Example: Using These References Together

**Scenario**: Building a new tree type with drooping vines and root system.

**Step 1**: Check QUICK_CHAR_MATRIX for drooping pattern
```
Drooping vine: │ → ╯ → ~ → ~ → •
```

**Step 2**: Check ORGANIC_CHAR_CONNECTIONS for exit/entry compatibility
```
│ exits: [Up, Down]
  ╯ can accept Down from │ (╯ has Up exit) ✓

╯ exits: [Up, Left]
  ~ can accept from... wait, ~ needs Left/Right
  ~ has exits: [Left, Right]
  ~ cannot accept from Up. ✗
```

**Step 3**: Adjust based on TREE_PATTERN_EXAMPLES
```
Look at Pattern 3 (Drooping Willow) — it goes:
│ → ╯ → ─ → ~ → ~
                (NOT │ → ╯ → ~)
```

**Step 4**: Check ORGANIC_CHAR_BUILDER for can_follow_organic()
```rust
can_follow_organic('╯', MoveDir::Left, '~')
  → ╯ has Left exit ✓
  → ~ has Right exit (opposite of Left) ✓
  → TRUE, valid connection
```

**Step 5**: Copy pattern from TREE_PATTERN_EXAMPLES into code, snapshot test, done.

---

## Summary

The character connection system ensures **visual continuity** by validating:

1. **Source**: Does previous cell have an exit in this direction?
2. **Target**: Does next cell accept entry from opposite direction?
3. **Termination**: Are we using a valid leaf/fruit character?

Use these three documents + code:

- **Comprehensive** (ORGANIC_CHAR_CONNECTIONS.md) — When you need full details
- **Quick** (QUICK_CHAR_MATRIX.md) — When you need a fast answer
- **Practical** (TREE_PATTERN_EXAMPLES.md) — When you want working code
- **Code** (ORGANIC_CHAR_BUILDER.rs) — When you're integrating into sprites.rs

Together, they enable building organic ASCII trees that look connected, natural, and convincing.
