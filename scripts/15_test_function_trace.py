"""Check real process logs and unchanged art with function tracing enabled.

Run under scripts/5_probe_guard.py; this is a finite instrumentation check.
"""
import collections
import gzip
import json
import os
from pathlib import Path
import subprocess
import sys

directory = Path(sys.argv[1]).resolve()
directory.mkdir(parents=True, exist_ok=True)
env = dict(os.environ, ASCII_GRID_W='24', ASCII_GRID_H='12',
           XDG_STATE_HOME=str(directory / 'state'))
env.pop('ASCII_FUNCTION_TRACE', None)
command = [str(Path('target/release/ascii-renderer').resolve()), '42', 'gem-aetherium-2']
baseline = subprocess.run(command, env=env, capture_output=True, timeout=5, check=True)
env['ASCII_FUNCTION_TRACE'] = str(directory / 'functions')
traced = subprocess.run(command, env=env, capture_output=True, timeout=5, check=True)
assert baseline.stdout == traced.stdout, 'Tracing changed rendered output'
paths = list((directory / 'functions').glob('functions-*.ndjson.gz'))
assert len(paths) == 1, paths
with gzip.open(paths[0], 'rt') as file:
    records = [json.loads(line) for line in file]
stacks = collections.defaultdict(list)
entered = collections.Counter()
closed = collections.Counter()
shutdown = []
for record in records:
    message = record['fields'].get('message')
    if message == 'function trace shutdown':
        shutdown.append(record['fields']['dropped_records'])
    if message not in ('new', 'close'):
        continue
    key = (record['filename'], record['line_number'], record['span']['name'])
    stack = stacks[record['threadId']]
    if message == 'new':
        stack.append(key)
        entered[key] += 1
    else:
        assert stack and stack.pop() == key, ('unpaired exit', record)
        closed[key] += 1
        assert 'time.busy' in record['fields'], record
assert entered == closed, (entered - closed, closed - entered)
assert all(not stack for stack in stacks.values()), stacks
assert shutdown == [0], shutdown
names = {key[2] for key in entered}
assert {'main', 'run', 'render', 'registered_modes'} <= names, names
report = {'status': 'passed', 'entries': sum(entered.values()),
          'exits': sum(closed.values()), 'distinct_functions': len(entered),
          'dropped_records': 0, 'identical_rendered_bytes': len(traced.stdout),
          'trace': str(paths[0])}
(directory / 'validation.json').write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps(report))
