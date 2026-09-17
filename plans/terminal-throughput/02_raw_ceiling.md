# Page 2: raw ceiling of the terminal path

`perf/tty_ceiling.sh`: pre-render N full frames with the CLI, `cat` the file to the tty, `tcdrain`, divide. No relay, no encoder, no worker in the loop.

## Measured here (tmux 3.7b, hidden window: tmux parses, iTerm draws nothing)

`perf/results/30_tty_ceiling/tmux-hidden.txt`, prismata, 30 full frames each:

| grid | bytes/frame | bytes/s | ms/frame | full-frame fps |
| --- | ---: | ---: | ---: | ---: |
| 200x50 | 144,729 | 18.2 M | 7.9 | 126 |
| 400x100 | 501,557 | 20.7 M | 24.2 | 41 |
| 1000x1000 | 4,234,239 | 18.9 M | 224.6 | 4.5 |

Same path, synthetic payloads (scratch, 1 run each):

| payload | bytes | bytes/s | cells/s |
| --- | ---: | ---: | ---: |
| plain `x` rows, no escapes | 33.5 M | 28.7 M | 28 M |
| prismata full frame (4.2 B/cell) | 4.2 M | 18.9 M | 4.5 M |
| one `\e[38;5;Nm` per cell (12 B/cell) | 2.3 M | 15.3 M | 1.3 M |

tmux's parser is the unit of cost: ~35 ns per plain cell, ~220 ns per prismata cell, ~780 ns per SGR-per-cell cell. Escape count per cell, not bytes per cell, sets the rate.

## What this bounds

fps ceiling = bytes/s ÷ bytes/frame. For a delta frame, bytes/frame = changed cells × bytes/changed cell.

| grid | cells | full frame at 19 MB/s | 50% churn | 10% churn |
| --- | ---: | ---: | ---: | ---: |
| 200x50 | 10k | 126 fps | 250 | 60 (cap) |
| 400x100 | 40k | 41 | 82 | 60 (cap) |
| 1000x1000 | 1M | 4.5 | 9 | 45 |

These are tmux-only upper bounds. iTerm sits after tmux and has its own ceiling (page 04 measures it; the 15_samply capture says it is the lower of the two).

## Not measured yet: needs a visible pane

Run each in its own terminal; results land in `perf/results/30_tty_ceiling/<label>.txt`:

| where | command | measures |
| --- | --- | --- |
| this tmux pane (visible) | `perf/tty_ceiling.sh` | tmux parse + tmux redraw to iTerm + iTerm draw |
| iTerm, no tmux | `perf/tty_ceiling.sh` | iTerm alone |
| Ghostty / kitty / Alacritty, no tmux | `LABEL=ghostty perf/tty_ceiling.sh` | GPU emulator alone |
| tmux hidden (done) | `perf/tty_ceiling.sh --tmux-hidden` | tmux parse only |

Each row prints bytes/s; the ratio between rows is the cost of that layer.
