#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
lab_venv="${TMPDIR:-/tmp}/ascii-terminal-lab-venv"
if [[ ! -x "$lab_venv/bin/python" ]]; then
    python3 -m venv "$lab_venv"
fi
"$lab_venv/bin/pip" --disable-pip-version-check install --quiet -r scripts/17_terminal_lab_requirements.txt
cargo build --release --locked --example 0_terminal_lab -j 2
cargo build --release --locked --example 1_native_terminal -j 2
exec "$lab_venv/bin/python" scripts/17_terminal_lab.py \
    --directory "${1:-perf/results/terminal-lab-$(date +%s)}"
