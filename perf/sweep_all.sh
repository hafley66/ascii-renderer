#!/usr/bin/env bash
# Sweep every mode that has knobs, short runs, then rank modes by their worst knob.
# usage: perf/sweep_all.sh [secs_per_run] [width] [height]
set -euo pipefail
secs="${1:-1}"
width="${2:-2000}"
height="${3:-1000}"
modes="$(grep -o 'names: &\["[^"]*"' src/registry.rs | sed 's/names: &\["//;s/"//'; grep -h 'fn name(&self)' -A1 src/modes/*.rs | grep -o '"[^"]*"' | tr -d '"')"
for m in $modes; do
  perf/knob_sweep.sh "$m" "$width" "$height" "$secs" >/dev/null 2>&1 || echo "sweep failed: $m"
done
python3 - "$width" "$height" "$secs" <<'PY' | tee perf/results/SUMMARY.md
import glob, re, sys
w, h, secs = sys.argv[1:4]
rows = []
for f in sorted(glob.glob("perf/results/*.md")):
    if f.endswith("SUMMARY.md"):
        continue
    mode = f.split("/")[-1][:-3]
    text = open(f).read()
    base = re.search(r"^\| baseline \| \d+ \| ([\d.]+) \|", text, re.M)
    worst = re.search(r"^worst: (.+)$", text, re.M)
    if not base:
        rows.append((0.0, mode, "not native", 0.0, "", 0.0))
        continue
    bfps = float(base.group(1))
    wk = worst.group(1) if worst else "baseline"
    wm = re.search(r"^\| " + re.escape(wk) + r" \| \d+ \| ([\d.]+) \| [\d.]+ \| [\d.]+ \| ([\d.]+) \|", text, re.M)
    wfps = float(wm.group(1)) if wm else bfps
    p99 = float(wm.group(2)) if wm else 0.0
    hot = re.search(r"^## hotspots.*?\n\n\| layer.*?\n\|[^\n]*\n\| ([^|]+) \| [^|]+ \| [^|]+ \| [^|]+ \| ([\d.]+)% \|", text, re.M | re.S)
    hotspot = f"{hot.group(1).strip()} {hot.group(2)}%" if hot else "no timers"
    rows.append((wfps, mode, wk, bfps, hotspot, p99))
rows.sort()
print(f"# all-mode sweep summary: {w}x{h}, {secs}s per run\n")
print("| mode | baseline fps | worst knob | worst fps | worst p99 ms | hotspot |")
print("| --- | ---: | --- | ---: | ---: | --- |")
for wfps, mode, wk, bfps, hotspot, p99 in rows:
    print(f"| {mode} | {bfps:.0f} | {wk} | {wfps:.0f} | {p99:.1f} | {hotspot} |")
PY
