# Visual Connection Diagrams

ASCII diagrams showing how characters connect and when connections are valid/invalid.

---

## Basic Connection Rule

```
    FROM_CHAR
       │
       │ (moving in DIRECTION)
       ▼
    TO_CHAR

VALID if:
  1. FROM_CHAR has exit in DIRECTION
  2. TO_CHAR has exit in opposite(DIRECTION)
  3. OR TO_CHAR is a terminator (·•●)
```

---

## Valid Vertical Connection

```
                    │  ← FROM_CHAR (│)
                    │
                    ├ ← TO_CHAR (├)
                    │
        Exits: Up   │  ↓  Exits: Up, Down, Right
            ↑ ↓     │
                    │

Check:
  FROM (│): has Down exit ✓
  TO   (├): has Up exit (opposite of Down) ✓
  RESULT: VALID ✓
```

---

## Invalid Horizontal Connection (Bad!)

```
    ─  ←  FROM_CHAR (─)
         ├  ← TO_CHAR (├)

    Exits: Left, Right      Exits: Up, Down, Right
         ← →                   ↑ ↓ →

Check:
  FROM (─): trying to exit Right ✓
  TO   (├): has Right exit ✓
  BUT wait... ├ needs entry from Left (opposite of Right)
  ├ has exits Up, Down, Right... NO Left!
  RESULT: INVALID ✗

Why: ├ is oriented vertically (U/D/R), can't accept horizontal entry from Left

Fix: Use ┬ instead
    ─ ┬  (┬ has exits Left, Right, Down)
```

---

## Vertical Stem Growing Upward

```
GOAL: Grow stem from ground (y=10) to sky (y=0)

y=0:   •   ← terminator (fruit at top)
       │
y=1:   ├   ← junction (optional, for branches)
       │
y=2:   │   ← vertical continuation
       │
y=3:   │
       │
y=10:  ├   ← start (split from main trunk)

VALIDATION BY ROW:
  y=10→9: │→│ valid (both have U/D exits) ✓
  y=9→8:  │→│ valid ✓
  y=8→7:  │→├ valid (│ exits U, ├ accepts U) ✓
  y=7→6:  ├→│ valid (├ exits U, │ accepts U) ✓
  y=1→0:  ├→• valid (├ exits U, • is terminator) ✓
```

---

## Fork: Splitting Left and Right

```
GOAL: Trunk continues up and splits left/right

        ╷   y=0 (top tip)
        •
        │   y=1
        ├   y=2 (SPLIT POINT)
      ╱ ╲   y=3 (two branches down)
     •   •  y=4 (fruits)

VALIDATION:
  y=2→1: ├→│  (├ has U exit, │ accepts from below) ✓
  y=2→3: ├→╱  wait...├ has exits U,D,R not left!
                     need┤ for left split

CORRECTED:
      y=2: │ branches to ├ and ┤
      ├├  OR use ┼ for full cross

  y=2→3L: ├→─  (├ exits R, ─ accepts L) ✓
  y=2→3R: ┤→─  (┤ exits L, ─ accepts R) ✓
```

---

## Turn Corner (90° Bend)

```
GOAL: Branch going right, suddenly goes up

       │    ← continuation upward
       ├    ← CORNER (╰ connects right→up)
    ───╰    ← approaching from right

VALIDATION:
  Right→UP at corner (╰):
    ╰ has exits: Up, Right
    Entering from Right, need Right exit in ╰ ✓
    Exiting toward Up, need Up exit in ╰ ✓

RESULT: Valid ✓

Correct character: ╰ (up+right corner)

Other corners:
  ╭ = down+right
  ╮ = down+left
  ╯ = up+left
```

---

## Wave Sequence (Organic Vine)

```
GOAL: Wavy vine drooping down with fruits

          │     y=0 (vertical start)
          ╯     y=1 (corner, start droop)
          ─     y=2 (horizontal run)
          ∿     y=3 (first wave)
          ∿     y=4 (wave continues)
          ~     y=5 (change wave type)
          •     y=6 (fruit hang)

VALIDATION:
  y=0→1: │→╯  (│ exits U, ╯ has U) ✓
  y=1→2: ╯→─  (╯ exits R, ─ has R) ✓
  y=2→3: ─→∿  (─ exits R, ∿ has R) ✓
  y=3→4: ∿→∿  (∿ exits R,D,L,U, ∿ all directions) ✓
  y=4→5: ∿→~  (∿ exits L/R, ~ has L/R) ✓
  y=5→6: ~→•  (~ exits R, • is terminator) ✓

RESULT: Valid vine sequence ✓
```

---

## Root System Spreading

```
GOAL: Trunk at ground splits into radiating roots

          │  y=0 (trunk descending)
          │  y=1
          ├  y=2 (SPLIT AT GROUND)
        ╱ ╲ y=3
       ⌿   ⌿ y=4 (ROOT DIAGONALS)
      ·     · y=5 (root tips)

VALIDATION:
  y=2→3L: ├→⌿  (├ has R but moving Left→R is ~Down...
                  ├ exits R, ⌿ needs Right entry... ⌿ accepts UpRight)
                  Actually: ├ at (5,2), ⌿ at (4,3)
                  Direction: Down+Left = DownLeft
                  ├ doesn't exit DownLeft!

CORRECTED: Use different approach
  ├ at (5, 2):  exits Up, Down, Right
  Left branch:  go Down to (5,3), then Left-Down diagonally

Sequence:
  (5,2): ├
  (5,3): ╱     (from ├ going Down, then turn DownLeft)
  (4,4): ⌿     (continuing DownLeft)
  (3,5): ·     (root tip)

  Right branch:
  (5,3): ╲     (from ├ going Down, then turn DownRight)
  (6,4): ⍀     (continuing DownRight)
  (7,5): ·     (root tip)

CLEAN VERSION:
  y=2: ├        (fork point)
  y=3: │ ╯ ╭    (intermediate layer: continue down-center, start turns)
  y=4: ⌿ ┼ ⍀    (diagonal roots spreading)
  y=5: · · ·    (root tips)
```

---

## Invalid Connection Examples

### Bad 1: Wrong exit direction
```
│   (vertical line)
├   (try to continue horizontally)

PROBLEM:
  │ exits: Up, Down (no Left/Right)
  Cannot exit toward ├ going Right

FIX: Use ┬ instead (has Left/Right exits)
  ├   becomes   ┬
    \            /
     ─         ─
```

### Bad 2: Terminator with further drawing
```
•     (fruit terminator)
│     (try to continue)

PROBLEM:
  • has no exits (it's an endpoint)
  Cannot draw from •

FIX: Reverse order
  │
  ╷     (stub to prepare endpoint)
  •     (place fruit last)
```

### Bad 3: Incompatible character families
```
∿     (wave, multi-directional)
│     (straight vertical)

Going UP from ∿ to │:
  ∿ exits: U, D, L, R (all) ✓
  │ exits: U, D
  │ has Down (opposite of Up) ✓

Actually VALID! But looks awkward visually
Worse: going Right from ∿ to │
  ∿ exits: R ✓
  │ accepts Left? Has U, D only ✗

FIX: Use intermediate: ∿ → ─ → │
```

### Bad 4: Thick trunk narrowing without junction
```
┃      (thick trunk)
│      (thin trunk)

PROBLEM:
  ┃ exits: Up, Down ✓
  │ accepts: Up, Down ✓
  Actually valid, but visually jarring (no tapering effect)

FIX: Use intermediate mixed character
  ┃
  ├  (creates junction feel)
  │
```

---

## Fruit Placement Patterns

### At Endpoint (stub + fruit)
```
Approach: │
Endpoint: ╷
Fruit:    •

Visual:
  │     (stem)
  ├─    (optional branch)
  ╷     (stub pointing direction)
  •     (fruit attached)
```

### At Fork (multiple fruits)
```
Approach:    │
Fork:        ┬
Left fruit:  •
Center down: │
Right fruit: •

Visual:
     •   •     (two fruits)
      \ │ /
       ┬       (fork)
       │       (continue down)
       │
```

### In Wave (mid-sequence)
```
Wave:  ~ ~ ∿ ~ •

Visual:
  ─ ─ ∿ ∿ ~ •
    \  \   /
     waves  fruit

Coordinates:
  (5,0): ─
  (6,0): ─
  (7,0): ∿
  (8,1): ∿
  (9,1): ~
  (10,1): •
```

### Hanging Fruit (droop end)
```
Approach:  │
Turn:      ╯
Hang:      ─ ─ ~ • (drooping sequence)

Visual:
  │
  ├─────•  (horizontal run with fruit at end)
  OR
  │
  ╯        (turn down)
  └────•   (droop with fruit)
```

---

## Multi-Layer Tree

```
LAYER 1 (top):
       •
       │
      ╭┴╮

LAYER 2 (mid):
      │ │ │
      ├ ├ ├

LAYER 3 (base):
      • • •

Connection between layers:
  y=1: ╭, ┴, ╮  (top fork)
  y=2: │, │, │  (vertical continuations)
  y=3: ├, ├, ├  (mid forks)
  y=4: │, │, │
  y=5: •, •, •  (base fruits)

Each column is independent:
  Left: ╭→│→├→│→•
  Mid:  ┴→│→├→│→•
  Right:╮→│→├→│→•

All sequences valid if chars chosen correctly!
```

---

## Complex Burst Pattern

```
GOAL: Firework/explosion effect from center

        •   •   •
         \ | /
        • ╋ •    (╋ = all 4 directions)
         / | \
        •   •   •

Directions from ╋ (x=5, y=5):
  Up    (5,4): │ → •
  Down  (5,6): │ → •
  Left  (4,5): ─ → •
  Right (6,5): ─ → •
  UL    (4,4): ╲ → •
  UR    (6,4): ╱ → •
  DL    (4,6): ╱ → •
  DR    (6,6): ╲ → •

VALIDATION:
  ╋ exits: U, D, L, R (standard cardinal)
         U (5,4): need │, accepts Down ✓
         D (5,6): need │, accepts Up ✓
         L (4,5): need ─, accepts Right ✓
         R (6,5): need ─, accepts Left ✓

  Diagonals NOT available from ╋ (no UR, UL, DR, DL)
  Need different center or intermediate layer

FIX: Use intermediate layer for diagonals
  ╋ (center)
  ├─┐   (add horizontals for diagonals to attach)
  └─┘
  (then place diagonals)
```

---

## Gradient Path (zigzag)

```
GOAL: Tree zigzags as it grows

      │
      ├─╮
      │ └─╮
      ├───╯
      │
      ├─╮
      │ └─•

PATTERN:
  y=0: │
  y=1: ├  (turn right)
  y=1: ─  (horizontal arm)
  y=1: ╮  (corner up-right)
  y=0: •  (fruit)
  y=2: ├  (continue down, split right)
  y=2: ─  (horizontal arm)
  y=2: ╭  (corner down-right)
  y=3: │  (drop down)
  y=4: •  (fruit)

Creates sawtooth effect by alternating
├ (split right) with ┤ (split left)
```

---

## Summary: Connection Validity

| From | Direction | To | Valid? | Reason |
|------|-----------|----|----|--------|
| `│` | Up | `│` | ✓ | Both have U/D exits |
| `│` | Right | `─` | ✗ | `│` has no Right exit |
| `│` | Up | `•` | ✓ | `•` is terminator |
| `├` | Right | `─` | ✓ | `├` exits R, `─` accepts L |
| `─` | Right | `╱` | ✓ | Both have R/L exits |
| `∿` | Up | `│` | ✓ | `∿` exits U, `│` exits D |
| `⌿` | DownLeft | `⌿` | ✓ | Both exit DL/UR |
| `~` | Up | `∿` | ✗ | `~` has no Up exit |
| `•` | Any | Any | ✗ | `•` has no exits |

---

## Mental Model

Think of each character as a **physical joint**:

```
│ = straight joint (only lets line pass through vertically)
├ = T-joint (line goes U/D, branch comes off R)
╰ = elbow joint (line comes in from R, bends up)
• = end cap (line stops, nothing comes out)

Trying to connect incompatible joints = broken pipe ✗
```

Each exit is a **port** on the character. Connections require compatible ports.
