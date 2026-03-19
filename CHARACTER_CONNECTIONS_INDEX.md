# Character Connections Reference Index

**Date**: 2026-03-18
**Project**: ascii-renderer
**Topic**: ASCII character connection system for organic tree rendering

---

## Quick Navigation

### I have 2 minutes — read this:
👉 **CHAR_CHEAT_SHEET.txt** — One-page printable reference. All patterns, rules, lookup tables.

### I have 10 minutes — read this:
👉 **QUICK_CHAR_MATRIX.md** — Fast lookup tables. "What can follow X in direction Y?"

### I have 30 minutes — read this:
👉 **CHARACTER_CONNECTION_DESIGN.md** — Problem, solution, design decisions, roadmap.

### I'm building a tree — read these:
👉 **TREE_PATTERN_EXAMPLES.md** — 11 working patterns with step-by-step code.
👉 **VISUAL_CONNECTION_DIAGRAMS.md** — ASCII diagrams of valid/invalid connections.

### I need complete reference — read this:
👉 **ORGANIC_CHAR_CONNECTIONS.md** — Every character, every exit, every valid follower.

### I'm integrating into code — read this:
👉 **ORGANIC_CHAR_BUILDER.rs** — Rust code to add to sprites.rs.

---

## File Descriptions

| File | Format | Length | Purpose | When to Use |
|------|--------|--------|---------|------------|
| **CHAR_CHEAT_SHEET.txt** | ASCII text art | 1 page | Printable reference card | At desk, quick lookup |
| **QUICK_CHAR_MATRIX.md** | Markdown tables | 10-15 min read | Fast pattern lookup | Coding, designing patterns |
| **CHARACTER_CONNECTION_DESIGN.md** | Markdown prose | 20-30 min read | Design doc, roadmap | Understanding scope, planning |
| **CHAR_CONNECTIONS_README.md** | Markdown overview | 5-10 min read | Navigation guide | Getting oriented |
| **ORGANIC_CHAR_CONNECTIONS.md** | Markdown reference | 30-40 min read | Complete reference | Full details, edge cases |
| **TREE_PATTERN_EXAMPLES.md** | Markdown + Rust | 30-40 min read | Working patterns | Building new trees |
| **VISUAL_CONNECTION_DIAGRAMS.md** | Markdown + ASCII | 20-30 min read | Animated examples | Understanding validation |
| **ORGANIC_CHAR_BUILDER.rs** | Rust code | 5-10 min review | Integration code | Adding to sprites.rs |

---

## By Use Case

### "I need to understand the system quickly"
1. Read CHAR_CHEAT_SHEET.txt (2 min)
2. Skim QUICK_CHAR_MATRIX.md (5 min)
3. Look at VISUAL_CONNECTION_DIAGRAMS.md (10 min)
4. Done! You understand the core concept.

### "I'm designing a new tree type"
1. Read QUICK_CHAR_MATRIX.md patterns (5 min)
2. Find similar pattern in TREE_PATTERN_EXAMPLES.md (10 min)
3. Copy and adapt (20 min)
4. Test with snapshot (5 min)
5. Done!

### "I need to debug a broken connection"
1. Check VISUAL_CONNECTION_DIAGRAMS.md for your scenario (5 min)
2. Verify in QUICK_CHAR_MATRIX.md table (2 min)
3. Check ORGANIC_CHAR_CONNECTIONS.md detailed exits (5 min)
4. Trace through the character sequence (10 min)

### "I'm integrating the system into code"
1. Review CHARACTER_CONNECTION_DESIGN.md roadmap (5 min)
2. Read ORGANIC_CHAR_BUILDER.rs (5 min)
3. Copy code blocks into sprites.rs (30 min)
4. Run existing tests, snapshot new patterns (30 min)
5. Done!

### "I need the complete reference (no skipping)"
**Reading order** (2-3 hours total):
1. CHAR_CONNECTIONS_README.md (5 min) — navigation
2. CHARACTER_CONNECTION_DESIGN.md (15 min) — scope & decisions
3. QUICK_CHAR_MATRIX.md (15 min) — patterns overview
4. VISUAL_CONNECTION_DIAGRAMS.md (20 min) — examples
5. ORGANIC_CHAR_CONNECTIONS.md (40 min) — detailed reference
6. TREE_PATTERN_EXAMPLES.md (30 min) — code patterns
7. ORGANIC_CHAR_BUILDER.rs (10 min) — integration code

---

## What Each File Covers

### CHAR_CHEAT_SHEET.txt
- Vertical, horizontal, diagonal continuations
- Junctions, corners, terminators
- Organic/wave characters
- Thick/heavy characters
- 10 copy-paste patterns
- Connection rules & examples
- Character families by intent
- DON'Ts (common mistakes)

### QUICK_CHAR_MATRIX.md
- Summary table: Character → Valid Exits → Valid Followers
- Quick lookup patterns ("I'm going UP, what's next?")
- Character pick-by-intent tables
- Termination rules
- Visual continuity checklist
- Copy-paste Rust snippets
- When to use each character

### CHARACTER_CONNECTION_DESIGN.md
- Problem statement
- Solution overview (exits/entries)
- Character families (17 structural + 7 organic + 6 thick)
- Integration map (which file does what?)
- Implementation roadmap (phases & timelines)
- Design decisions with rationale
- Test coverage needed
- Future enhancements

### CHAR_CONNECTIONS_README.md
- Overview of the reference set
- When to use each file
- Core concept explained
- Visual continuity checklist
- Integration roadmap
- Example: using references together
- Summary

### ORGANIC_CHAR_CONNECTIONS.md
- Vertical stems & trunks (│ ┃ ╷ ╵)
- Horizontal branches (─ ━ ╴ ╶)
- Diagonal trunks & branches (╱ ╲ ⌿ ⍀)
- Turns & corners (╭ ╮ ╰ ╯)
- Junctions & splits (├ ┤ ┬ ┴ ┼ ╋)
- Terminators (·•●◆◇)
- Organic leaf/foliage (*, ✱, ✲)
- Bracketed shapes ((){}[])
- Wave & organic lines (∿ ~ ≈)
- Block elements (█▓▌▐▀▄)
- Root & spreading (⌿⍀)
- Termination strategies
- Rust implementation pattern
- Usage examples

### TREE_PATTERN_EXAMPLES.md
- Pattern 1: Classic upright tree (step-by-step)
- Pattern 2: Asymmetric spreading tree (GRIS-style)
- Pattern 3: Drooping willow (hanging vines)
- Pattern 4: Root system at base
- Pattern 5: Multi-stage branching (fractal)
- Pattern 6: Thick-trunked tree
- Pattern 7: Dense fruit cluster
- Pattern 8: Tendril/firework burst
- Pattern 9: Weeping vine (loops)
- Pattern 10: Twisted/spiral trunk
- Pattern 11: Asymmetric canopy
- Pattern composition rules
- Testing checklist
- Copy-paste template

### VISUAL_CONNECTION_DIAGRAMS.md
- Basic connection rule (visual)
- Valid vertical connection (traced)
- Invalid horizontal connection (annotated)
- Vertical stem growing upward (step-by-step)
- Fork: splitting left/right
- Turn corner (90° bend)
- Wave sequence (organic vine)
- Root system spreading
- Invalid connection examples (4 types with fixes)
- Fruit placement patterns
- Multi-layer tree
- Complex burst pattern
- Gradient path (zigzag)
- Summary table: valid/invalid examples
- Mental model (physical joints)

### ORGANIC_CHAR_BUILDER.rs
- Expanded char_exits() arms (copy/paste)
- can_follow_organic() function (full impl)
- Character family predicates (is_terminator, is_wave, etc.)
- Direction-based glyph selection
- Fruit/leaf placement rules
- Integration checklist (TODO)

---

## Core Concepts (All Files)

### The Connection Rule
```
FROM_CHAR at (x1,y1) moving DIRECTION to TO_CHAR at (x2,y2)

VALID if:
  1. FROM_CHAR has exit in DIRECTION
  2. TO_CHAR accepts entry from opposite(DIRECTION)
     (has exit in opposite direction)
  3. OR TO_CHAR is a terminator (· • ●)
```

### Character Exits
Each character has a list of directions it can travel:
```
│  → [Up, Down]
├  → [Up, Down, Right]
•  → []  (no exits = endpoint)
```

### Terminators
Characters with no exits: `· • ● ◆ ◇`
- Represent endpoints (fruit, leaf, bud)
- Can accept entry from any direction
- Cannot draw FROM them
- Always placed last

### Character Families
- **Structural** (17): Box-drawing chars
- **Organic** (7): Waves, roots, complex branching
- **Thick** (6): Block elements, tapers
- **Terminators** (7): Fruits, leaves, buds

---

## Implementation Status

### Already in codebase (sprites.rs)
- ✓ char_exits() for 17 structural chars
- ✓ connect_glyph() for turns
- ✓ dir_glyph() for directions
- ✓ TreePen struct for connected drawing
- ✓ Some terminator chars

### Not yet implemented
- ✗ 7 organic characters (∿ ~ ⌿ ⍀ ⌠ ∫ ⌡)
- ✗ 6 thick characters (█ ▓ ▌ ▐ ▀ ▄)
- ✗ can_follow_organic() validator
- ✗ Character family predicates
- ✗ Fruit sizing by depth
- ✗ Snapshot tests for patterns

### Estimated effort to implement
- Phase 1 (expand char_exits): 1 hour
- Phase 2 (can_follow_organic): 1 hour
- Phase 3 (predicates): 30 min
- Phase 4 (update tree functions): 2-3 hours
- Phase 5 (tests): 1-2 hours
- **Total**: 5-7 hours

---

## Cross-References

### In ORGANIC_CHAR_CONNECTIONS.md
- "Wave & Organic Lines" section covers ∿ ~ ≈
- "Root & Spreading" section covers ⌿ ⍀
- "Block Elements" section covers █ ▓ etc.
- "Termination Strategies" shows copy-paste patterns

### In TREE_PATTERN_EXAMPLES.md
- Pattern 3 shows waves (∿ ~)
- Pattern 4 shows roots (⌿ ⍀)
- Pattern 6 shows thick blocks (█)
- Pattern 8 shows complex branching (⌠ ∫ ⌡)

### In QUICK_CHAR_MATRIX.md
- Summary table shows all exits/followers
- Quick patterns section has 6 scenarios
- Character pick-by-intent shows when to use what

### In VISUAL_CONNECTION_DIAGRAMS.md
- Wave sequence diagram shows ∿ chaining
- Root system diagram shows ⌿ ⍀ spreading
- Invalid examples show common mistakes

---

## Common Questions

### "Which file answers question X?"

**"What characters can follow │ going UP?"**
→ QUICK_CHAR_MATRIX.md table, or ORGANIC_CHAR_CONNECTIONS.md under "Vertical Stems"

**"Show me a working example of roots"**
→ TREE_PATTERN_EXAMPLES.md Pattern 4, or VISUAL_CONNECTION_DIAGRAMS.md Root System section

**"How do I add new organic characters to the code?"**
→ ORGANIC_CHAR_BUILDER.rs, or CHARACTER_CONNECTION_DESIGN.md Phase 1-2

**"Why is my branch looking disconnected?"**
→ VISUAL_CONNECTION_DIAGRAMS.md Invalid Connection Examples, or QUICK_CHAR_MATRIX.md troubleshooting

**"Should I use ╱ or ⌿ for roots?"**
→ QUICK_CHAR_MATRIX.md Character families, or CHARACTER_CONNECTION_DESIGN.md Design Decisions

**"How do I snapshot test a new pattern?"**
→ TREE_PATTERN_EXAMPLES.md Testing Checklist, or codebase tests/snapshot_modes.rs

---

## Printing Guide

### Print-Friendly Files
- CHAR_CHEAT_SHEET.txt — **Print this!** (1 page, ASCII art, desk reference)
- QUICK_CHAR_MATRIX.md — Print pages 1-3 (patterns, character families)
- CHARACTER_CONNECTION_DESIGN.md — Print pages 2-3 (roadmap, checklist)

### Digital-Only Files
- VISUAL_CONNECTION_DIAGRAMS.md — Interactive, better on screen
- TREE_PATTERN_EXAMPLES.md — Code blocks, copy-paste, better on screen
- ORGANIC_CHAR_CONNECTIONS.md — Large reference, use search function

---

## Version & Maintenance

**Created**: 2026-03-18
**Status**: Complete reference, ready for implementation
**Last Updated**: 2026-03-18
**Maintainer**: [Your name]
**Related Issues**: [Link to issue if applicable]

---

## Quick Links to Code Locations

- **char_exits()** — src/sprites.rs line ~488
- **connect_glyph()** — src/sprites.rs line ~443
- **TreePen** — src/sprites.rs line ~585
- **Snapshot tests** — tests/snapshot_modes.rs

---

## Summary

This reference set provides everything needed to understand, design, and implement ASCII character connections for organic tree rendering.

**Start with**: CHAR_CHEAT_SHEET.txt (2 min)
**Deepen with**: QUICK_CHAR_MATRIX.md (10 min)
**Master with**: ORGANIC_CHAR_CONNECTIONS.md + examples (1 hour)
**Implement with**: ORGANIC_CHAR_BUILDER.rs + CHARACTER_CONNECTION_DESIGN.md (6 hours)

All files cross-reference each other for easy navigation. Print the cheat sheet, keep it at hand, refer to detailed docs as needed.

Ready to build convincing ASCII trees!
