#!/usr/bin/env bash
# One command: pinned tools, current binary, safety tests, real-terminal E2E.
set -euo pipefail
cd "$(dirname "$0")/.."
e2e_venv="${TMPDIR:-/tmp}/ascii-e2e-venv"
if [[ ! -x "$e2e_venv/bin/python" ]]; then
  python3 -m venv "$e2e_venv"
fi
"$e2e_venv/bin/pip" --disable-pip-version-check install --quiet -r scripts/11_e2e_requirements.txt
build_artifacts="perf/results/e2e-build-$(date +%s)"
mkdir -p "$build_artifacts"
python3 scripts/5_probe_guard.py \
  --state "$build_artifacts/guard.ndjson" --artifact-dir "$build_artifacts" \
  --max-seconds 900 --max-owned-mib 1024 --max-artifact-mib 32 \
  -- /usr/sbin/taskpolicy -b /usr/bin/nice -n 19 \
  cargo build --release --locked -j 1 --bin ascii-renderer --example 1_native_terminal \
  > "$build_artifacts/build.log" 2>&1
"$e2e_venv/bin/python" scripts/5_probe_guard_test.py
"$e2e_venv/bin/python" scripts/12_test_e2e_test.py
exec "$e2e_venv/bin/python" scripts/12_test_e2e.py "$@"
