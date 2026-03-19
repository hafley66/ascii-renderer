# Character Connection System for Organic Tree Rendering

**Date**: 2026-03-18
**Status**: Reference documentation complete, ready for implementation
**Scope**: Comprehensive guide for ASCII character connections in tree/vine/root rendering

---

## Problem Statement

When rendering ASCII trees, characters need to visually **connect**. A branch growing upward should smoothly continue when it turns right, not jump awkwardly. Fruit shouldn't float disconnected from branches.

The codebase already has a partial solution (`char_exits()`, `connect_glyph()`, `TreePen`), but it only covers standard box-drawing characters. Trees need **organic characters** too:
- Wave/vine chars: `∿ ~ ≈`
- Root diagonals: `⌿ ⍀`
- Fruit/leaves: `· • ● ◆ ◇`
- Thick blocks: `█ ▓ ▌ ▐`
- Complex branches: `⌠ ∫ ⌡`

---

## Solution Overview

A character connection system based on **exits** and **entries**:

```
EXITS: Directions a character can travel FROM
  │ has exits: [Up, Down]
  ─ has exits: [Left, Right]
  ├ has exits: [Up, Down, Right]

ENTRIES: Directions a character can accept FROM (inverse of exits)
  To enter ├ from Above, need opposite(Up) = Down exit in ├ ✓
  To enter ├ from Left, need opposite(Left) = Right exit in ├ ✓
```

When moving from CHAR_A in DIRECTION to CHAR_B:
1. CHAR_A must have an exit in DIRECTION
2. CHAR_B must accept entry from opposite(DIRECTION)
3. If both true, connection is valid and visually continuous

---

## Character Families

### STRUCTURAL (17 standard box-drawing)
```
Verticals:   │ ┃ ╷ ╵
Horizontals: ─ ━ ╴ ╶
Diagonals:   ╱ ╲
Corners:     ╭ ╮ ╰ ╯
Junctions:   ├ ┤ ┬ ┴ ┼ ╋ ┣ ┫ ┳ ┻
```

**Status in code**: ✓ Fully implemented in char_exits()

### ORGANIC (7 new characters)
```
Waves:   ∿ ∽ ~ ≈
Roots:   ⌿ ⍀
```

**Status in code**: ✗ Need to add to char_exits()

### COMPLEX_BRANCHING (3 characters)
```
Integrals: ⌠ ∫ ⌡
```

**Status in code**: ✗ Need to add

### TERMINATORS (7 characters)
```
Fruits/Leaves: · • ● ◆ ◇ ○ ◎
```

**Status in code**: ~ Partially (· and • exist, need full set)

### THICK_BLOCKS (6 characters)
```
Heavy: █ ▓ ▌ ▐ ▀ ▄
```

**Status in code**: ✗ Not in char_exits()

---

## Key Insight: Exits Define Continuity

A character's **exits** determine where branches can go next. This is THE source of truth:

```rust
pub fn char_exits(ch: char) -> &'static [MoveDir] {
    match ch {
        '│' => &[Up, Down],          // Can continue up or down
        '─' => &[Left, Right],       // Can continue left or right
        '├' => &[Up, Down, Right],   // Can split: continue up/down, branch right
        '·' => &[],                  // TERMINAL: nowhere to go
    }
}
```

No exits = endpoint (leaf, fruit). Can't draw from it.

---

## Validation Function

```rust
pub fn can_follow(from: char, dir: MoveDir, to: char) -> bool {
    // Step 1: Can 'from' exit in 'dir'?
    if !char_exits(from).contains(&dir) {
        return false;  // No exit = can't travel in that direction
    }

    // Step 2: Can 'to' accept entry from opposite?
    let entry_dir = opposite(dir);
    let to_exits = char_exits(to);

    // Special case: terminators accept entry from anywhere
    if to_exits.is_empty() {
        return is_terminator(to);
    }

    // Standard: 'to' must have an exit back toward 'from'
    to_exits.contains(&entry_dir)
}
```

---

## Integration Map

### File: ORGANIC_CHAR_CONNECTIONS.md
- **What**: Complete reference for every organic character
- **Sections**: Exits, followers, use cases for each char
- **Use when**: Designing a new tree type, full detail needed
- **Size**: ~500 lines

### File: QUICK_CHAR_MATRIX.md
- **What**: Lookup tables, quick patterns
- **Sections**: Summary table, pick-by-intent, visual checklists
- **Use when**: Need answer in 10 seconds while coding
- **Size**: ~400 lines

### File: TREE_PATTERN_EXAMPLES.md
- **What**: 11 working tree patterns with code
- **Sections**: Each pattern with step-by-step Rust
- **Use when**: Building new tree type, need proven blueprint
- **Size**: ~600 lines

### File: ORGANIC_CHAR_BUILDER.rs
- **What**: Rust code ready to integrate into sprites.rs
- **Sections**: Expanded char_exits() arms, can_follow_organic(), predicates
- **Use when**: Implementing in codebase
- **Size**: ~150 lines of code + TODO checklist

### File: CHAR_CHEAT_SHEET.txt
- **What**: Printable reference card
- **Format**: Text art, patterns, rules on one page
- **Use when**: Need quick visual reference at desk
- **Size**: ~100 lines

### File: CHAR_CONNECTIONS_README.md (this doc's intro)
- **What**: Overview and navigation
- **Use when**: Onboarding to the reference set

---

## Implementation Roadmap

### Phase 1: Expand char_exits() — 1 hour
```rust
// In sprites.rs, add organic chars to char_exits():
'∿' | '∽' => &[Up, Down, Left, Right],
'~' | '≈' => &[Left, Right],
'⌿' => &[UpRight, DownLeft],
'⍀' => &[UpLeft, DownRight],
'⌠' => &[Down, Right],
'∫' => &[Up, Down],
'⌡' => &[Up, Left],
// ... (copy from ORGANIC_CHAR_BUILDER.rs)
```

### Phase 2: Add can_follow_organic() — 1 hour
```rust
pub fn can_follow_organic(from: char, dir: MoveDir, to: char) -> bool {
    // (see ORGANIC_CHAR_BUILDER.rs for full impl)
}
```

### Phase 3: Add character predicates — 30 mins
```rust
is_terminator(ch), is_vertical_line(ch), is_wave(ch), is_root(ch), etc.
```

### Phase 4: Update tree growth functions — 2-3 hours
- grow_tree(), grow_wild_tree(), grow_tendril_tree(), etc.
- Replace hardcoded chars with suggest_organic_glyph()
- Use fruit_for_depth() for size-based fruits
- Use can_follow_organic() for validation

### Phase 5: Test & snapshot — 1-2 hours
- Add snapshot tests for each pattern type
- Verify organic chars chain correctly
- Test root systems, wave sequences

**Total estimated effort**: 5-7 hours for full implementation

---

## Design Decisions & Rationale

### Why separate root chars (⌿ ⍀) from diagonal branches (╱ ╲)?
**Reason**: Grass uses ╱ ╲, so tree diagonals hard to distinguish from roots.
Using unique root chars prevents visual confusion in dense forests.

### Why terminators (·•●) have no exits?
**Reason**: Visually represents "endpoint." No further drawing possible.
Forces algorithm to place fruits/leaves last, preventing disconnected structures.

### Why include wave chars (∿) with multi-directional exits?
**Reason**: Vines are organic, non-linear. Single wave can continue any direction.
More natural than forcing straight paths; better for drooping, draping effects.

### Why use half-blocks (▌▐▀▄) for thick trunks?
**Reason**: Multi-cell-wide trunks need visual differentiation from single-cell lines.
Half-blocks allow tapering (thick base → thin top) without requiring extra width.

---

## Performance & Constraints

- **Char lookup**: O(1) — direct match in char_exits()
- **Validation**: O(1) — contains() check on small array
- **Memory**: Negligible — new chars add ~50 bytes
- **Compatibility**: Fully backward compatible with existing trees

---

## Test Coverage Needed

```rust
#[test]
fn test_organic_char_exits() {
    assert_eq!(char_exits('∿'), &[Up, Down, Left, Right]);
    assert_eq!(char_exits('⌿'), &[UpRight, DownLeft]);
}

#[test]
fn test_can_follow_organic() {
    // Wave chains
    assert!(can_follow_organic('∿', MoveDir::Right, '∿'));
    assert!(can_follow_organic('∿', MoveDir::Up, '│'));

    // Root spread
    assert!(can_follow_organic('├', MoveDir::Right, '⌿'));
    assert!(can_follow_organic('⌿', MoveDir::UpRight, '⌿'));
    assert!(can_follow_organic('⌿', MoveDir::UpRight, '·'));

    // Invalid combos
    assert!(!can_follow_organic('│', MoveDir::Left, '─'));
    assert!(!can_follow_organic('∿', MoveDir::Right, '│'));
}

#[test]
fn test_drooping_vine_pattern() {
    let mut grid = /* ... */;
    draw_drooping_vine(&mut grid);
    insta::assert_snapshot!("drooping_vine", grid_to_string(&grid));
}

#[test]
fn test_root_system_pattern() {
    let mut grid = /* ... */;
    draw_root_system(&mut grid);
    insta::assert_snapshot!("root_system", grid_to_string(&grid));
}

#[test]
fn test_wave_chain_pattern() {
    let mut grid = /* ... */;
    draw_wave_chain(&mut grid);
    insta::assert_snapshot!("wave_chain", grid_to_string(&grid));
}
```

---

## Documentation Structure

```
CHAR_CONNECTIONS_README.md  (START HERE)
  ├─ ORGANIC_CHAR_CONNECTIONS.md    (Full reference)
  ├─ QUICK_CHAR_MATRIX.md           (Fast lookup)
  ├─ TREE_PATTERN_EXAMPLES.md       (Working code)
  ├─ ORGANIC_CHAR_BUILDER.rs        (Integration)
  └─ CHAR_CHEAT_SHEET.txt           (Printable card)
```

**Reading order**:
1. README (2 min) — understand scope
2. Quick Matrix (5 min) — see patterns
3. Comprehensive (15 min) — deep dive
4. Examples (20 min) — study code
5. Builder (5 min) — implementation steps

---

## Future Enhancements

### Unicode Expansion
- Add `♣ ♦ ♥ ♠` for flowers/fruits (card suits)
- Add `⚘ ⚙ ☘` for leaves/flowers
- Add `🌲` emoji (if terminal supports)

### Context-Aware Glyphs
- Suggest different chars based on tree type
- Depth → fruit size (shallow = big, deep = small)
- Direction → angle (up/down vs left/right)

### Automatic Path Finding
- Given start + end, find best character sequence
- Avoid crossovers, optimize for readability
- Cost function: continuity + aesthetic balance

### Visual Validator
- Render tree with problems highlighted
- Show which connections are broken
- Suggest fixes (what to change)

---

## Related Files in Codebase

- `src/sprites.rs` — Tree rendering functions, contains char_exits(), connect_glyph()
- `src/main.rs` — Mode dispatch, forest3/forest4 mode definitions
- `src/color.rs` — Color utilities (lighten/darken for depth)
- `tests/snapshot_modes.rs` — Snapshot tests (add pattern tests here)

---

## Conclusion

This character connection system provides the foundation for **organic, visually continuous ASCII trees**.

By defining exits per character and validating connections, we ensure:
- ✓ Branches don't float disconnected
- ✓ Fruit attaches to trees naturally
- ✓ Roots spread convincingly at base
- ✓ Vines droop with organic curves
- ✓ Complex structures build without visual artifacts

The reference documents cover every use case from quick lookup to detailed implementation.

Ready to integrate. See ORGANIC_CHAR_BUILDER.rs for the code.
