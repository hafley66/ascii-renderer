#!/usr/bin/env bash
# One command: pinned tools, current binary, safety tests, real-terminal E2E.
set -euo pipefail
cd "$(dirname "$0")/.."
e2e_venv="${TMPDIR:-/tmp}/ascii-e2e-venv"
if [[ ! -x "$e2e_venv/bin/python" ]]; then
  python3 -m venv "$e2e_venv"
fi
"$e2e_venv/bin/pip" --disable-pip-version-check install --quiet -r scripts/11_e2e_requirements.txt
cargo build --release --locked
"$e2e_venv/bin/python" scripts/5_probe_guard_test.py
"$e2e_venv/bin/python" scripts/12_test_e2e_test.py
exec "$e2e_venv/bin/python" scripts/12_test_e2e.py "$@"
