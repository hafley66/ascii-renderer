// ORGANIC_CHAR_BUILDER.rs
// Expanded character set + exits for tree/vine/root rendering
// Ready to integrate into sprites.rs after char_exits()

// ============================================================================
// EXPANDED CHAR_EXITS: Add to match in char_exits() function
// ============================================================================

// Paste into char_exits() as new arms before the catch-all `_ => &[]`:

// Organic wavy/vine characters
'∿' | '∽' => &[Up, Down, Left, Right],  // sine waves, multi-directional tendrils
'~' => &[Left, Right],                   // horizontal wave, droop
'≈' => &[Left, Right],                   // wavy equals, thick wave

// Root/spreading diagonals (unique angles, not standard ╱╲)
'⌿' => &[UpRight, DownLeft],             // root spreading right
'⍀' => &[UpLeft, DownRight],             // root spreading left

// Half-block thick trunks
'▌' => &[Up, Down],                      // left block, vertical trunk (right-heavy)
'▐' => &[Up, Down],                      // right block, vertical trunk (left-heavy)
'▀' => &[Left, Right],                   // upper half block, horizontal branch
'▄' => &[Left, Right],                   // lower half block, horizontal branch

// Full blocks (heavy nodes, knots, compound junctions)
'█' => &[Up, Down, Left, Right],         // full block, all directions
'▓' => &[Up, Down, Left, Right],         // dark shade, acts like junction

// Branching organics (integral family)
'⌠' => &[Down, Right],                   // integral top, start of split
'∫' => &[Up, Down],                      // integral body, continuous branch
'⌡' => &[Up, Left],                      // integral bottom, end of split

// Leaf/fruit/flower terminators (no exits — endpoints)
'·' | '•' | '●' | '◆' | '◇' | '○' | '◎' | '*' => &[],
'꘎' => &[],  // leaf symbol (if Unicode available)

// ============================================================================
// CONNECTION VALIDATOR
// ============================================================================

/// Can character `from` connect to character `to` when traveling in `dir`?
/// Returns true if the connection maintains visual continuity.
///
/// Usage: `can_follow('│', MoveDir::Down, '├')` → checks if down from │
///        can validly reach ├. Answer: Yes (├ has Up exit).
pub fn can_follow_organic(from: char, dir: MoveDir, to: char) -> bool {
    // Check if 'from' has an exit in this direction
    if !char_exits(from).contains(&dir) {
        return false;
    }

    // Check if 'to' accepts entry from opposite direction
    let opposite_dir = opposite(dir);
    let to_exits = char_exits(to);

    // Terminators (leaf/fruit chars) accept entry from any direction
    if to_exits.is_empty() {
        return matches!(to, '·' | '•' | '●' | '◆' | '◇' | '○' | '◎' | '*' | '꘎');
    }

    // Standard connection: 'to' must have an exit toward where we came from
    to_exits.contains(&opposite_dir)
}

// ============================================================================
// CHARACTER FAMILIES (for intent-based branching)
// ============================================================================

pub fn is_terminator(ch: char) -> bool {
    matches!(ch, '·' | '•' | '●' | '◆' | '◇' | '○' | '◎' | '*' | '꘎')
}

pub fn is_vertical_line(ch: char) -> bool {
    matches!(ch, '│' | '┃' | '╷' | '╵')
}

pub fn is_horizontal_line(ch: char) -> bool {
    matches!(ch, '─' | '━' | '╴' | '╶')
}

pub fn is_diagonal_line(ch: char) -> bool {
    matches!(ch, '╱' | '╲' | '⌿' | '⍀')
}

pub fn is_junction(ch: char) -> bool {
    matches!(ch, '├' | '┤' | '┬' | '┴' | '┼' | '┣' | '┫' | '┳' | '┻' | '╋')
}

pub fn is_corner(ch: char) -> bool {
    matches!(ch, '╭' | '╮' | '╰' | '╯')
}

pub fn is_organic_wave(ch: char) -> bool {
    matches!(ch, '∿' | '∽' | '~' | '≈')
}

pub fn is_block_element(ch: char) -> bool {
    matches!(ch, '▌' | '▐' | '▀' | '▄' | '█' | '▓')
}

// ============================================================================
// DIRECTION-BASED GLYPH SELECTION
// ============================================================================

/// For organic branches, suggest the best glyph for a direction
/// considering both existing tree structure and aesthetic intent.
pub fn suggest_organic_glyph(dir: MoveDir, context: &str) -> char {
    match (dir, context) {
        // Vertical growth: standard or organic
        (MoveDir::Up, "thick") => '┃',
        (MoveDir::Up, "organic") | (MoveDir::Up, "tendril") => '│',
        (MoveDir::Up, _) => '│',

        // Horizontal: standard or organic
        (MoveDir::Left, "thick") | (MoveDir::Right, "thick") => '━',
        (MoveDir::Left, "organic") | (MoveDir::Right, "organic") => '─',
        (MoveDir::Left, _) | (MoveDir::Right, _) => '─',

        // Diagonals: standard or root-like
        (MoveDir::UpRight, "root") => '⌿',
        (MoveDir::UpRight, _) => '╱',
        (MoveDir::DownLeft, "root") => '⌿',
        (MoveDir::DownLeft, _) => '╱',

        (MoveDir::UpLeft, "root") => '⍀',
        (MoveDir::UpLeft, _) => '╲',
        (MoveDir::DownRight, "root") => '⍀',
        (MoveDir::DownRight, _) => '╲',

        // Tendril waves
        (MoveDir::Up, "vine") | (MoveDir::Down, "vine") => '∿',
        (MoveDir::Left, "vine") | (MoveDir::Right, "vine") => '~',

        _ => '·', // fallback to endpoint
    }
}

// ============================================================================
// FRUIT/LEAF PLACEMENT RULES
// ============================================================================

/// Select fruit character based on branch depth and tree type
pub fn fruit_for_depth(depth: usize, tree_type: &str) -> char {
    match (depth, tree_type) {
        // Shallow branches get bigger fruit
        (0..=2, "tendril") => '●',
        (0..=2, "wild") => '◆',
        (0..=2, _) => '●',

        // Mid branches get medium fruit
        (3..=5, "tendril") => '•',
        (3..=5, _) => '•',

        // Deep branches get tiny fruit/buds
        (6.., "tendril") => '·',
        (6.., _) => '·',

        _ => '·',
    }
}

/// Leaf cluster terminator based on branch direction
pub fn leaf_for_direction(dir: MoveDir) -> char {
    match dir {
        MoveDir::Up | MoveDir::UpLeft | MoveDir::UpRight => '꘎', // upward leaf
        MoveDir::Down | MoveDir::DownLeft | MoveDir::DownRight => '·', // drooping
        MoveDir::Left | MoveDir::Right => '•', // lateral leaf cluster
    }
}

// ============================================================================
// INTEGRATION CHECKLIST
// ============================================================================

// TODO: In sprites.rs, char_exits() function:
//   1. Add organic character arms (see EXPANDED_CHAR_EXITS above)
//   2. Update connect_glyph() to include diagonal root chars (⌿ ⍀)
//   3. Update dir_glyph() to support "organic" or "root" mode variants
//
// TODO: In tree growth functions (grow_tree, grow_wild_tree, etc.):
//   1. Use can_follow_organic() when validating next step
//   2. Call suggest_organic_glyph() instead of hardcoded chars
//   3. Use fruit_for_depth() when placing endpoint fruits
//   4. Use leaf_for_direction() for organic leaf placement
//
// TODO: Add tests in tests/snapshot_modes.rs:
//   - Verify organic waves chain correctly (∿ → ∿ → •)
//   - Verify roots spread correctly (│ → ├ → ⌿ → ·)
//   - Verify thick trunks don't merge with thin (█ ≠ │ at junction)
//   - Verify terminators block further drawing
