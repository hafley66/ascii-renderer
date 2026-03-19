/// Constraint-based ASCII art: character-to-edge mapping
///
/// Each character's "exits" indicate which cell boundaries it touches.
/// Used for validating connections between adjacent cells in procedural layouts.
///
/// Direction enum (8-connected):
/// ```
///   UL  U  UR
///   L  [ ]  R
///   DL  D  DR
/// ```

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Dir {
    Up,        // top center
    Down,      // bottom center
    Left,      // left center
    Right,     // right center
    UpLeft,    // top-left corner
    UpRight,   // top-right corner
    DownLeft,  // bottom-left corner
    DownRight, // bottom-right corner
}

/// Get exits for a character. Returns &[Dir] of all edges the character touches.
pub fn char_exits(ch: char) -> &'static [Dir] {
    use Dir::*;

    // Inline small arrays for each group
    match ch {
        // ========================================
        // BOX DRAWING: SINGLE WEIGHT ORTHOGONAL
        // ========================================

        // Horizontal
        '─' => &[Left, Right],
        '━' => &[Left, Right],

        // Vertical
        '│' => &[Up, Down],
        '┃' => &[Up, Down],

        // Corners (4-way)
        '┌' => &[Right, Down],
        '┐' => &[Left, Down],
        '└' => &[Right, Up],
        '┘' => &[Left, Up],

        // T-junctions
        '├' => &[Right, Up, Down],
        '┤' => &[Left, Up, Down],
        '┬' => &[Left, Right, Down],
        '┴' => &[Left, Right, Up],

        // Cross
        '┼' => &[Up, Down, Left, Right],

        // ========================================
        // BOX DRAWING: DOUBLE WEIGHT ORTHOGONAL
        // ========================================

        // Horizontal
        '═' => &[Left, Right],

        // Vertical
        '║' => &[Up, Down],

        // Corners (4-way)
        '╔' => &[Right, Down],
        '╗' => &[Left, Down],
        '╚' => &[Right, Up],
        '╝' => &[Left, Up],

        // T-junctions
        '╠' => &[Right, Up, Down],
        '╣' => &[Left, Up, Down],
        '╦' => &[Left, Right, Down],
        '╩' => &[Left, Right, Up],

        // Cross
        '╬' => &[Up, Down, Left, Right],

        // ========================================
        // BOX DRAWING: ROUNDED CORNERS
        // ========================================

        '╭' => &[Right, Down],
        '╮' => &[Left, Down],
        '╰' => &[Right, Up],
        '╯' => &[Left, Up],

        // ========================================
        // BOX DRAWING: ARC CORNERS
        // ========================================

        '╪' => &[Up, Down, Left, Right], // not true arc, included for completeness
        '╫' => &[Up, Down, Left, Right],

        // ========================================
        // BOX DRAWING: MIXED WEIGHT
        // ========================================

        // Single to double: corners
        '┍' => &[Right, Down],
        '┎' => &[Right, Down],
        '┑' => &[Left, Down],
        '┒' => &[Left, Down],
        '┕' => &[Right, Up],
        '┖' => &[Right, Up],
        '┗' => &[Left, Up],
        '┘' => &[Left, Up], // already listed above
        '┙' => &[Left, Up],
        '┚' => &[Right, Up],

        // Single/double T-junctions
        '┟' => &[Right, Up, Down],
        '┠' => &[Right, Up, Down],
        '┢' => &[Left, Up, Down],
        '┣' => &[Left, Up, Down],
        '┤' => &[Left, Up, Down],
        '┥' => &[Left, Up, Down],
        '┦' => &[Left, Up, Down],
        '┧' => &[Left, Up, Down],
        '┨' => &[Left, Up, Down],
        '┩' => &[Left, Right, Down],
        '┪' => &[Left, Right, Down],
        '┫' => &[Left, Right, Down],
        '┬' => &[Left, Right, Down],
        '┭' => &[Left, Right, Down],
        '┮' => &[Left, Right, Down],
        '┯' => &[Left, Right, Down],
        '┰' => &[Left, Right, Down],
        '┱' => &[Left, Right, Up],
        '┲' => &[Left, Right, Up],
        '┳' => &[Left, Right, Down],
        '┴' => &[Left, Right, Up],
        '┵' => &[Left, Right, Up],
        '┶' => &[Left, Right, Up],
        '┷' => &[Left, Right, Up],
        '┸' => &[Left, Right, Up],
        '┹' => &[Left, Right, Up],
        '┺' => &[Left, Right, Up],

        // ========================================
        // BOX DRAWING: DASHED LINES (LIGHT)
        // ========================================

        '┄' => &[Left, Right],   // dash horizontal
        '┅' => &[Left, Right],   // dash horizontal (double)
        '┆' => &[Up, Down],      // dash vertical
        '┇' => &[Up, Down],      // dash vertical (double)
        '┈' => &[Left, Right],   // dash horizontal (2-4 ratio)
        '┉' => &[Left, Right],   // dash horizontal (double)
        '┊' => &[Up, Down],      // dash vertical (2-4 ratio)
        '┋' => &[Up, Down],      // dash vertical (double)

        // ========================================
        // BOX DRAWING: DASHED LINES (HEAVY)
        // ========================================

        '╌' => &[Left, Right],   // heavy dash horizontal
        '╍' => &[Left, Right],   // heavy dash horizontal (double)
        '╎' => &[Up, Down],      // heavy dash vertical
        '╏' => &[Up, Down],      // heavy dash vertical (double)

        // ========================================
        // BOX DRAWING: WIDE & THIN COMBINATIONS
        // ========================================

        // Wider box segments (3x or 4x)
        '▬' => &[Left, Right],   // heavy horizontal (used in some contexts)

        // ========================================
        // BLOCK DRAWING DIRECTIONAL SHAPES
        // ========================================

        '▀' => &[Left, Right],   // upper half block (horizontal connection)
        '▄' => &[Left, Right],   // lower half block (horizontal connection)
        '▌' => &[Up, Down],      // left half block (vertical connection)
        '▐' => &[Up, Down],      // right half block (vertical connection)
        '█' => &[Up, Down, Left, Right], // full block (connects all)
        '▓' => &[Up, Down, Left, Right], // dark shade (treat as full)
        '▒' => &[Up, Down, Left, Right], // medium shade (treat as full)
        '░' => &[], // light shade (sparse, doesn't connect)

        // ========================================
        // GEOMETRIC DIRECTIONAL SHAPES
        // ========================================

        // Triangles pointing in directions
        '▲' => &[Down, DownLeft, DownRight], // points up, connects down
        '▼' => &[Up, UpLeft, UpRight],       // points down, connects up
        '◄' => &[Right, UpRight, DownRight], // points left, connects right
        '►' => &[Left, UpLeft, DownLeft],    // points right, connects left

        // Diamonds (4-directional)
        '◆' => &[Up, Down, Left, Right],
        '◇' => &[Up, Down, Left, Right],

        // ========================================
        // ARROWS (8-directional)
        // ========================================

        // Orthogonal arrows
        '↑' => &[Down],                    // arrow up, points out top, connects from below
        '↓' => &[Up],                      // arrow down, points out bottom, connects from above
        '←' => &[Right],                   // arrow left, points out left, connects from right
        '→' => &[Left],                    // arrow right, points out right, connects from left

        // Diagonal arrows
        '↗' => &[Left, Down],              // northeast: connects from left and down
        '↘' => &[Left, Up],                // southeast: connects from left and up
        '↙' => &[Right, Up],               // southwest: connects from right and up
        '↖' => &[Right, Down],             // northwest: connects from right and down

        // Curved arrows (treat as directional)
        '↻' => &[Up, Down, Left, Right],   // curved: multi-directional
        '↺' => &[Up, Down, Left, Right],

        // ========================================
        // BRAIDS, SPIRALS, CORNERS
        // ========================================

        // Braille patterns (sparse representation)
        '⠿' => &[Up, Down, Left, Right],   // all 8 dots: full connection
        '⠀' => &[],                        // blank

        // ========================================
        // FILLED DIRECTIONAL SHAPES
        // ========================================

        '◀' => &[Right, UpRight, DownRight], // filled left triangle
        '▶' => &[Left, UpLeft, DownLeft],    // filled right triangle
        '▲' => &[Down, DownLeft, DownRight], // filled up triangle
        '▼' => &[Up, UpLeft, UpRight],       // filled down triangle

        // ========================================
        // CIRCLE VARIANTS (FULL CONNECTION)
        // ========================================

        '●' => &[Up, Down, Left, Right],
        '○' => &[Up, Down, Left, Right],
        '◉' => &[Up, Down, Left, Right],
        '◎' => &[Up, Down, Left, Right],

        // ========================================
        // CROSS VARIANTS
        // ========================================

        '✕' => &[Up, Down, Left, Right],
        '✗' => &[UpLeft, UpRight, DownLeft, DownRight],
        '✖' => &[UpLeft, UpRight, DownLeft, DownRight],
        '✙' => &[Up, Down, Left, Right],
        '✚' => &[Up, Down, Left, Right],
        '✜' => &[Up, Down, Left, Right],

        // ========================================
        // STAR VARIANTS
        // ========================================

        '★' => &[Up, Down, Left, Right, UpLeft, UpRight, DownLeft, DownRight],
        '☆' => &[Up, Down, Left, Right, UpLeft, UpRight, DownLeft, DownRight],
        '✡' => &[Up, Down, Left, Right, UpLeft, UpRight, DownLeft, DownRight],

        // ========================================
        // ORNAMENTAL & SPECIAL CHARACTERS
        // ========================================

        '○' => &[Up, Down, Left, Right],
        '◑' => &[Up, Down, Left, Right],
        '◒' => &[Up, Down, Left, Right],
        '◓' => &[Up, Down, Left, Right],

        // ========================================
        // ASCII FALLBACKS (for when Unicode unavailable)
        // ========================================

        '-' => &[Left, Right],
        '|' => &[Up, Down],
        '+' => &[Up, Down, Left, Right],
        '/' => &[UpLeft, DownRight],
        '\\' => &[UpRight, DownLeft],
        '*' => &[Up, Down, Left, Right],

        // ========================================
        // DEFAULT: NO CONNECTIONS
        // ========================================

        _ => &[],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_orthogonal_lines() {
        assert_eq!(char_exits('─'), &[Dir::Left, Dir::Right]);
        assert_eq!(char_exits('│'), &[Dir::Up, Dir::Down]);
    }

    #[test]
    fn test_corners() {
        assert_eq!(char_exits('┌'), &[Dir::Right, Dir::Down]);
        assert_eq!(char_exits('┐'), &[Dir::Left, Dir::Down]);
        assert_eq!(char_exits('└'), &[Dir::Right, Dir::Up]);
        assert_eq!(char_exits('┘'), &[Dir::Left, Dir::Up]);
    }

    #[test]
    fn test_cross() {
        assert_eq!(char_exits('┼'), &[Dir::Up, Dir::Down, Dir::Left, Dir::Right]);
        assert_eq!(char_exits('+'), &[Dir::Up, Dir::Down, Dir::Left, Dir::Right]);
    }

    #[test]
    fn test_arrows() {
        assert_eq!(char_exits('→'), &[Dir::Left]); // points right, connects from left
        assert_eq!(char_exits('↓'), &[Dir::Up]);    // points down, connects from up
    }

    #[test]
    fn test_diagonal_arrows() {
        assert_eq!(char_exits('↘'), &[Dir::Left, Dir::Up]);
    }

    #[test]
    fn test_dashes() {
        assert_eq!(char_exits('┄'), &[Dir::Left, Dir::Right]);
        assert_eq!(char_exits('┆'), &[Dir::Up, Dir::Down]);
    }

    #[test]
    fn test_no_connection() {
        assert_eq!(char_exits(' '), &[]);
        assert_eq!(char_exits('a'), &[]);
    }
}
