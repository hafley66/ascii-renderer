# opus-5-5-prothesis prompt sequence

Verbatim prompts and steering messages, in the order they were given.

## 1. Task message (as received by this session)

> Repo: /Users/chrishafley/projects/ascii-renderer.
>
> Make ASCII art. Choose the opposite vector to all that has come before in any capacity. "All that has come before" = every mode in the repo (src/ and the mode list in CLAUDE.md).
>
> Deliver it as a new registered mode named `opus-5-5-prothesis`.
>
> Work in the main working tree. Do not commit; the user reviews first. Another session is editing mahoraga files (examples/3_maha_rig.rs, maha rig code) at the same time; don't touch those.

No brief file: the message above is the whole brief.

## What came before, measured

Every snapshot under `tests/snapshots`, `src/snapshots` and `src/modes/snapshots` (527 files),
and every help string and knob label in `src/`.

| axis | the corpus | prothesis |
|---|---|---|
| subject | life: trees, forests, six trees of life, bloom, breath, growth, the cosmos | one death. No file in `src/` mentions death, mourning, a bier, a shroud or a vigil |
| commonest words | speed 146, depth 91, hue 86, density 58, spin 53, motion 52, sway 51, glow 51 | vigil, mourners, stature, bier, night |
| ink | mean density 0.38; ink in 79 percent of 8x4 blocks | 0.07 at 80x24, 0.03 at 200x60, in one low band |
| glyphs | 591 distinct; 69 percent of snapshots use non-ASCII; `─` is the commonest | 7-bit ASCII only: `` _ . - ' ` ^ / \ ( ) \| = o `` and capitals |
| colour | palettes, hue knobs, glow | neutral grays only. The theme is ignored: no hue anywhere |
| time | cycles and loops forever | one vigil, then rest. Every frame after the vigil is identical, colours included |
| what time does | grows, accretes, blooms | only takes away. The ink count never rises (tested) |
| motion | everything moves | only the lamp flame flickers. The body never moves |
| composition | centred, symmetric, full-bleed, often framed | low, horizontal, asymmetric, no border, no fill |
| making | generated: noise, automata, L-systems, DLA, physics | authored: a hand-placed silhouette drawn in hand ASCII line idioms |
| the seed | randomises everything | names the dead; the room is the same for everyone |
| knobs | about ten per mode | five |
| the figure | `\o/` appears twice, as jubilation | `/o\`: hands to the head, the lament |

## Design chosen

A prothesis (Greek: a setting-forth) is the laying-out of the dead before burial: the body on
a bier, a lamp, mourners with their hands raised to their heads, as on Geometric funerary vases.
Here the scene is drawn with nothing but plain ASCII and sits low in an empty terminal.

- The shroud is an authored height profile (pillow, crown, brow, nose, lips, chin, throat, chest,
  folded hands, hips, knees, shins, toes tenting the cloth). It is drawn the way a hand draws ASCII
  line: every glyph attaches left and right at quarter-row heights (`_` 0-0, `.` 0-2, `-` 2-2,
  `'` 2-4, `/` 0-4), and a Viterbi pass picks the connected glyph run closest to the curve.
  The line is traced once per size and cached, because the body never moves.
- The seed chooses one of 28 names from Attic grave stelai (Hegeso, Ampharete, Mnesarete,
  Phrasikleia, Kroisos, Dexileos, Eupheros, ...), set below the bier one letter per cell with
  spaces between, as on an inscription.
- Time is one vigil (default 273 s of player time, 4'33"). Each mourner lowers their arms and
  leaves at a seeded moment between 10 and 80 percent of it. The flame burns, gutters to a `.` at
  88 percent, goes out at 92 percent, and the dark settles by 100 percent. After that, nothing
  changes again. There is no smoke and no afterglow: nothing is added, only taken away.
- In the dark the white shroud keeps the most light (gray 214 lit, 143 at default night).
- Knobs: `VIGIL` (seconds), `MOURNERS` (0 to 7), `STATURE` (body length as a share of width),
  `BIER` (bier line as a share of height), `NIGHT` (how dark the lamp leaves the room).

## Frames, 80x24, seed 42

Start of the vigil:

```text
             '                   /o\       /o\      /o\
            \_/                   |         |        |
             |                    |         |        |
             |       .-.    __________      |        |       /\
             |      /   \_.'          `----.________________/  )
             |     (                                           )
             |     (                                           )
             |   =================================================
             |    |                                             |
            _|_   |                                             |

                                  E U P H E R O S
```

Halfway (one mourner left):

```text
             '                             /o\
            \_/                             |
             |                              |
             |       .-.    __________      |                /\
             |      /   \_.'          `----.________________/  )
```

The end, and every frame after it:

```text

            \_/
             |
             |       .-.    __________                       /\
             |      /   \_.'          `----.________________/  )
             |     (                                           )
             |     (                                           )
             |   =================================================
             |    |                                             |
            _|_   |                                             |

                                  E U P H E R O S
```
