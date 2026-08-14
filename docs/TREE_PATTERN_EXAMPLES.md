# ASCII Tree Pattern Examples

Real, working patterns for organic/natural trees using the character connection system.

---

## PATTERN 1: Classic Upright Tree

**Description**: Simple tree with centered trunk, side branches, and fruit clusters.

```
        •
        │
       ╭┴╮
      ◆ │ ◇
       ├ ├ ├
       │ │ │
       • • •
```

**Step-by-step (pseudocode):**
```rust
// Trunk: bottom to top
grid[10][5] = '│';
grid[9][5] = '│';
grid[8][5] = '│';
grid[7][5] = '├';  // First branch point

// Left branch from (7,5) going left
grid[7][4] = '─';
grid[7][3] = '●';  // Fruit

// Right branch continues up
grid[6][5] = '│';
grid[5][5] = '├';  // Second branch point

// Left off second fork
grid[5][4] = '─';
grid[5][3] = '◇';

// Right off second fork
grid[5][6] = '─';
grid[5][7] = '●';

// Top continues
grid[4][5] = '│';
grid[3][5] = '╷';
grid[2][5] = '•';
```

**Visual stability**: Clear center line, branches off junctions, fruits on terminal lines.

---

## PATTERN 2: Asymmetric Spreading Tree (GRIS-style)

**Description**: Trunk with unequal left/right branching, depth-varying fruit sizes.

```
  •     ◇           ●
  │     │           │
 ╱      └─╮      ╭──┘
│        │ │    │ │
├─────┴──┼─┴─┬──┴─├
│        │    │    │
│        │    │    │
•        •    •    •
```

**Character sequence for left arm (depth 0→1→2):**
```
Start (x=5, y=10, depth=0):
  grid[10][5] = '│'     // trunk down
  grid[9][5] = '├'      // junction
  grid[9][3] = '╭'      // turn left
  grid[9][2] = '─'
  grid[9][1] = '─'
  grid[9][0] = '●'      // BIG fruit (shallow depth)

Start (x=5, y=7, depth=1):
  grid[7][5] = '├'      // left fork from trunk
  grid[7][2] = '╭'      // turn left
  grid[7][1] = '─'
  grid[7][0] = '•'      // medium fruit (mid depth)

Start (x=5, y=4, depth=2):
  grid[4][5] = '├'      // right-most fork
  grid[4][6] = '─'
  grid[4][7] = '─'
  grid[4][8] = '·'      // tiny fruit (deep)
```

**Key principle**: Depth → fruit size (inverse: shallow branches = large fruit)

---

## PATTERN 3: Drooping Willow (Hanging Vines)

**Description**: Trunk with long, wavy downward branches drooping to ground.

```
    │
    │
   ╭┴╮
   │ │
  ╱ ╲
~     ~
~     ~
•     •
```

**Character flow (right droop arm):**
```
Attachment at (9, 5):
  grid[9][5] = '╮'      // start of droop

Down the main arm:
  grid[8][6] = '│'
  grid[7][6] = '╱'      // diagonal toward right

Wavy descent:
  grid[6][7] = '~'
  grid[5][7] = '∿'
  grid[4][8] = '~'
  grid[3][8] = '∿'
  grid[2][9] = '~'

Terminate:
  grid[1][9] = '•'      // hanging fruit
```

**Key principle**: Use `~` and `∿` to suggest organic, meandering motion. No rigid joints.

---

## PATTERN 4: Root System at Base

**Description**: Trunk meets ground with spreading roots radiating outward.

```
    │
    │
    ├  ← split point at ground
   ╱ ╲
  ⌿   ⍀
 ╱     ╲
⌿       ⍀
·       ·
```

**Implementation:**
```rust
// Trunk down to ground
grid[10][10] = '│';
grid[11][10] = '├';  // Fork at ground level

// Left root spreading (down-left)
grid[12][9] = '⌿';   // Root diagonal
grid[13][8] = '⌿';   // Continue spreading
grid[14][7] = '·';    // Root tip

// Right root spreading (down-right)
grid[12][11] = '⍀';  // Root diagonal
grid[13][12] = '⍀';  // Continue spreading
grid[14][13] = '·';   // Root tip
```

**Key principle**: Roots use `⌿` and `⍀` (unique diagonals) to visually distinguish from branch `╱` and `╲`.

---

## PATTERN 5: Multi-Stage Branching (Fractal-like)

**Description**: Recursive tree where each branch can sub-branch.

```
         •
         │
        ╭┴╮
       │ │ │
       ├ ├ ├  ← stage 1 forks
      ╭┘│└╮
     │  ├  │ ← stage 2 sub-forks
     •  •  •
```

**Recursive structure:**
```rust
fn draw_branch(x, y, depth, direction) {
    if depth == 0 {
        grid[y][x] = '╷';
        grid[y-1][x] = '•';  // terminal
        return;
    }

    // Draw vertical segment
    for i in 0..segment_length {
        grid[y-i][x] = '│';
    }

    let new_y = y - segment_length;
    grid[new_y][x] = '├';  // junction for branches

    // Recursive calls: left, center, right
    if rng() < 0.7 {
        draw_branch(x - 2, new_y, depth - 1, LEFT);
    }
    draw_branch(x, new_y - 2, depth - 1, UP);    // continue main
    if rng() < 0.7 {
        draw_branch(x + 2, new_y, depth - 1, RIGHT);
    }
}
```

**Key principle**: Each depth level uses a single junction (`├` or `┤`) to split, then recurse.

---

## PATTERN 6: Thick-Trunked Tree with Block Elements

**Description**: Wide base tapering upward, using block characters for solidity.

```
      •
      │
      │
     ┃│┃
    ┃┃┃┃┃  ← thick base (age/strength)
    ┃┃┃┃┃
    ┃│┃│┃
     └┼┘
    ╱ ╲
   ·   ·
```

**Implementation (simpler version):**
```rust
// Thick trunk segment (3-cell-wide)
for y in 0..8 {
    grid[y][x - 1] = '│';
    grid[y][x] = '┃';       // main trunk
    grid[y][x + 1] = '│';
}

// Transition to thin
grid[7][x] = '├';
grid[7][x - 1] = '─';
grid[7][x - 2] = '●';

// Continue upward (thin)
grid[6][x] = '│';
grid[5][x] = '│';
grid[4][x] = '╷';
grid[3][x] = '•';
```

**Key principle**: `█` or `┃` at trunk center, `│` on flanks. Maintains thickness through row count, not character width.

---

## PATTERN 7: Dense Fruit Cluster (Terminal Burst)

**Description**: Multiple fruits clustered at one point from a fork.

```
  ● ◆ ●
  │╱│╲│
  └─┼─┘
    │
    │
```

**Implementation:**
```rust
// Main junction
grid[5][5] = '┼';

// Fruits radiating out (max 4 directions)
// Up-left
grid[4][4] = '●';
// Up
grid[4][5] = '◆';
// Up-right
grid[4][6] = '●';
// Right
grid[5][6] = '◇';

// Note: Can't do all 8 directions without visual chaos.
// Stick to 3-4 max per junction.
```

**Key principle**: Limit fruits to 3-4 per junction. Use different symbols to add visual variety.

---

## PATTERN 8: Tendril/Firework Burst

**Description**: Organic radial growth from a single point, each tendril spiraling outward.

```
     •     •
      \   /
    •──┼──•
      / │ \
     •  │  •
        │
        │
```

**Implementation:**
```rust
// Center burst point
grid[5][5] = '╋';  // all 4 directions available

// 8-directional radiation (use diagonals)
for angle in 0..8 {
    let (dx, dy) = angle_offset(angle);  // (±1, ±1) combinations
    let mut x = 5;
    let mut y = 5;

    for step in 1..4 {  // grow 3 steps per tendril
        x += dx;
        y += dy;
        if step < 3 {
            grid[y][x] = dir_to_glyph(dx, dy);  // ╱ ╲ │ ─ etc.
        } else {
            grid[y][x] = '•';  // endpoint
        }
    }
}
```

**Key principle**: Use `╋` as burst center. Diagonals (╱ ╲ and diagonal-moving) suggest energy outward.

---

## PATTERN 9: Weeping Vine (Tendril with Loops)

**Description**: Single stem with recursive downward loops, fruit hanging mid-loop.

```
  │
  ├─┐
  │ └─•  ← fruit mid-vine
  ├───┐
  │   └─•
  ├─────┐
  │     └─•
  ·
```

**Implementation:**
```rust
let mut x = 5;
let mut y = 10;

// Main vertical with periodic loops
for segment in 0..4 {
    // Vertical drop
    for i in 0..2 {
        grid[y][x] = '│';
        y += 1;
    }

    // Loop out to the right
    grid[y][x] = '├';
    let loop_len = 2 + segment;  // longer loops as we go down
    for i in 0..loop_len {
        x += 1;
        grid[y][x] = '─';
    }

    // Fruit hanging at loop end
    grid[y - 1][x] = '•';

    // Retract back (optional visual)
    // or just jump back to main stem
    x = 5;
    y += 1;
}

// Final endpoint
grid[y][x] = '·';
```

**Key principle**: Alternating `├` with horizontal runs creates looping effect. Fruits placed slightly above loop end.

---

## PATTERN 10: Twisted/Spiral Trunk

**Description**: Trunk that spirals outward as it grows, creating organic waviness.

```
    •
    │
   ╱│
  │ │
  ╲│
   ╱│
  │ │
  ╲│
   ├─•
```

**Implementation:**
```rust
let mut x = 5;
let mut y = 10;
let mut dir_left = true;

for step in 0..8 {
    // Straight segment
    grid[y][x] = '│';
    y -= 1;

    if step % 2 == 0 {
        // Twist left
        x -= 1;
        grid[y][x] = '╱';
        y -= 1;
        grid[y][x] = '│';
        y -= 1;
    } else {
        // Twist right
        x += 1;
        grid[y][x] = '╲';
        y -= 1;
        grid[y][x] = '│';
        y -= 1;
    }
}

// Endpoint
grid[y][x] = '╷';
grid[y - 1][x] = '•';
```

**Key principle**: Alternating `╱` and `╲` at regular intervals creates helical motion. Works best with slower growth rate.

---

## PATTERN 11: Asymmetric Canopy (Unbalanced)

**Description**: One side denser/longer than the other, suggesting wind damage or growth bias.

```
     •           ◇
     │           │
    ╭┴╮       ╭──┘
   │ │ │     │
  ╭┘ │ └╮    ├───•
 │ │ │ │ │  ╱
 • • • • •  •
```

**Tree profile:**
- Left side: 3 levels, 2-3 fruits per level
- Right side: 1 long branch, single fruit
- Imbalance = visual interest

**Implementation strategy:**
```rust
// Left heavy (3 branch points)
tset(grid, 5, 10, '│', color);
tset(grid, 5, 8, '├', color);
tset(grid, 3, 8, '●', color);      // left fruit

tset(grid, 5, 6, '├', color);
tset(grid, 4, 6, '●', color);      // left
tset(grid, 6, 6, '◆', color);      // center
tset(grid, 7, 6, '●', color);      // right

tset(grid, 5, 3, '├', color);
tset(grid, 2, 3, '◇', color);      // left
tset(grid, 8, 3, '●', color);      // right (far)

// Right sparse (single long branch)
tset(grid, 5, 4, '─', color);
tset(grid, 6, 4, '─', color);
tset(grid, 7, 4, '─', color);
tset(grid, 8, 4, '─', color);
tset(grid, 9, 4, '•', color);      // far right fruit
```

**Key principle**: Use different fruit symbols (●, ◆, ◇) to add visual variation even within asymmetry.

---

## Pattern Composition Rules

1. **Always terminate with a leaf/fruit character**. Never end with `│` or `─` unless blocked by grid boundary.

2. **Use junction characters (├ ┤ ┬ ┴ ┼) ONLY at actual splits**. Don't overuse.

3. **Maintain visual hierarchy**:
   - Trunk: solid, central, upright
   - Primary branches: thick or prominent characters
   - Secondary branches: thinner, diagonal
   - Tertiary+: thin or wavy

4. **Depth controls fruit size**:
   - Depth 0-2: `●` (big)
   - Depth 3-5: `•` (medium)
   - Depth 6+: `·` (tiny)

5. **Asymmetry beats symmetry** for naturalism:
   - Unequal branch lengths
   - Off-center splits
   - Varied fruit counts per branch

6. **Color variation helps distinguish overlapping trees**:
   - Each tree gets a hue
   - Darken by depth (far = dim, near = bright)

---

## Testing Your Patterns

Use this checklist before committing:

```rust
#[test]
fn test_pattern_connections() {
    let mut grid = Grid::new(25, 80);

    // Draw your pattern
    draw_my_tree(&mut grid);

    // Verify:
    // 1. Every non-empty cell is either a valid char or leaf
    for row in grid {
        for cell in row {
            if cell.ch != ' ' {
                assert!(is_valid_tree_char(cell.ch), "Bad char: {}", cell.ch);
            }
        }
    }

    // 2. Snapshot test
    let output = grid_to_string(&grid);
    insta::assert_snapshot!("pattern_name", output);
}
```

---

## Copy-Paste Template for New Patterns

```rust
/// Draw [PATTERN_NAME] tree at (root_x, root_y) with (width, height) bounds.
fn draw_pattern_template(
    grid: &mut Grid,
    root_x: usize, root_y: usize,
    width: usize, height: usize,
    color: Color,
) {
    // 1. Draw main trunk/stem
    // 2. Add primary branches with junctions
    // 3. Add secondary growth
    // 4. Terminate all paths with leaf/fruit
    // 5. (Optional) Color variation by depth

    // Stub:
    tset_over(grid, root_x as i32, root_y as i32, '│', color);
}
```

Keep the pattern confined to the specified bounding box to prevent overlap issues and allow multiple trees on same canvas.
