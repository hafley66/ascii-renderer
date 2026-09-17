#!/bin/sh
# cat pre-rendered full frames to this tty; no application code in the loop.
# usage: perf/tty_ceiling.sh [--tmux-hidden] [WxH ...]   env: ASCII_PERF_BIN FRAMES MODE OUT LABEL
set -eu
cd "$(dirname "$0")/.."
BIN=${ASCII_PERF_BIN:-target/release/ascii-renderer}
FRAMES=${FRAMES:-30}
MODE=${MODE:-prismata}
mkdir -p perf/results/30_tty_ceiling
hidden=0
if [ "${1:-}" = "--tmux-hidden" ]; then hidden=1; shift; fi
sizes=${*:-"200x50 400x100 1000x1000"}

if [ $hidden = 1 ]; then
    label=tmux-hidden
    OUT=${OUT:-perf/results/30_tty_ceiling/$label.txt}
    tmux new-window -d -n tty-ceiling "OUT=$OUT FRAMES=$FRAMES MODE=$MODE $0 $sizes > /dev/null 2>&1; tmux wait -S tty-ceiling"
    tmux wait tty-ceiling
    cat "$OUT"
    exit 0
fi

label=${LABEL:-$(printf '%s' "${TERM_PROGRAM:-unknown}${TMUX:+-tmux}")}
OUT=${OUT:-perf/results/30_tty_ceiling/$label.txt}
scratch=$(mktemp -d)
trap 'rm -rf "$scratch"' EXIT
{
    printf 'tty ceiling  %s  TERM_PROGRAM=%s TMUX=%s pane=%s frames=%s mode=%s\n' \
        "$(date -u +%FT%TZ)" "${TERM_PROGRAM:-}" "${TMUX:+yes}" "$(stty size 2>/dev/null | tr ' ' x)" "$FRAMES" "$MODE"
    printf '%-10s %12s %12s %10s %8s\n' grid bytes/frame bytes/s ms/frame fps
} > "$OUT"
for size in $sizes; do
    w=${size%x*}; h=${size#*x}
    file=$scratch/$size.ans
    : > "$file"
    i=0
    while [ $i -lt "$FRAMES" ]; do
        ASCII_GRID_W=$w ASCII_GRID_H=$h ASCII_T=$(awk "BEGIN{print $i*0.06}") \
            "$BIN" 42 "$MODE" moss >> "$file"
        i=$((i+1))
    done
    total=$(wc -c < "$file" | tr -d ' ')
    start=$(python3 -c 'import time;print(time.time())')
    cat "$file" > /dev/tty
    # tcdrain: return only when the kernel queue is empty, so the timing covers
    # every byte the terminal pulled, not just the last write call.
    python3 -c 'import termios,sys; termios.tcdrain(sys.stdout.fileno())' > /dev/tty
    end=$(python3 -c 'import time;print(time.time())')
    printf '\033[0m\033[2J\033[H' > /dev/tty
    awk -v g="$size" -v b="$total" -v f="$FRAMES" -v s="$start" -v e="$end" \
        'BEGIN{d=e-s; printf "%-10s %12d %12.0f %10.2f %8.1f\n", g, b/f, b/d, d/f*1000, f/d}' >> "$OUT"
done
cat "$OUT"
