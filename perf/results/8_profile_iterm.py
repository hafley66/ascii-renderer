"""Sample the exact raw-iTerm demo repro during animation and across q.

Run 7_prepare_iterm.applescript first, then pass its window ID. Outputs are
process stack observations, not CPU percentages or terminal paint timestamps.
"""
import argparse
import hashlib
import json
import pathlib
import subprocess
import time

parser = argparse.ArgumentParser()
parser.add_argument('--window-id', type=int, required=True)
parser.add_argument('--output', type=pathlib.Path, required=True)
args = parser.parse_args()
base = pathlib.Path(__file__).resolve().parent
output = args.output.resolve()
observer = subprocess.Popen([
    '/usr/bin/python3', str(base / '7_iterm_uncaptured_1788718894474/observe.py'),
    '--window-id', str(args.window_id), '--output', str(output),
])
deadline = time.monotonic() + 40
while time.monotonic() < deadline:
    trace = output / 'animation.ndjson'
    rows = []
    if trace.exists():
        for line in trace.read_text().splitlines():
            try:
                rows.append(json.loads(line))
            except json.JSONDecodeError:
                pass
    frames = [r for r in rows if 'frame_index' in r]
    relays = [r for r in rows if r.get('kind') == 'playback_relay' and r.get('animation')]
    if frames and relays:
        break
    if observer.poll() is not None:
        raise RuntimeError('Observer exited before animation')
    time.sleep(.02)
else:
    raise RuntimeError('Animation startup timed out')

started = time.monotonic()
processes = {'worker': frames[-1]['pid'], 'supervisor': relays[-1]['pid']}
for line in subprocess.check_output(['ps', '-axo', 'pid,ppid,comm'], text=True).splitlines()[1:]:
    pid, ppid, command = line.strip().split(None, 2)
    if command == '/Applications/iTerm.app/Contents/MacOS/iTerm2':
        processes['iterm'] = int(pid)
    elif 'iTermServer-' in command:
        processes['iterm_server'] = int(pid)
binary = base.parents[1] / 'target/release/ascii-renderer'
(output / 'metadata.json').write_text(json.dumps({
    'created_ms': time.time_ns() // 1_000_000,
    'scenario': 'raw iTerm2 exact demo, all maxima, sampling during animation and q',
    'binary': str(binary), 'sha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
    'processes': processes, 'window_id': args.window_id,
    'terminal_size': frames[-1]['terminal_size'], 'grid': frames[-1]['grid'],
}, indent=2))
events = (output / 'samples.ndjson').open('a', buffering=1)
jobs = []
def sample(label, name, pid, seconds):
    path = output / f'{label}_{name}.sample.txt'
    log = (output / f'{label}_{name}.sample.log').open('w')
    command = ['/usr/bin/sample', str(pid), str(seconds), '1', '-mayDie', '-file', str(path)]
    events.write(json.dumps({'kind': 'sample_started', 'ts_ms': time.time_ns() // 1_000_000,
                            'label': label, 'name': name, 'pid': pid, 'command': command}) + '\n')
    jobs.append((name, label, subprocess.Popen(command, stdout=log, stderr=log), log))

for name, pid in processes.items():
    sample('steady', name, pid, 6)
# The observer sends q eighteen seconds after its first completed frame.
# Start before that request, leaving the sampler active throughout the delay.
while time.monotonic() - started < 15:
    time.sleep(.05)
for name, pid in processes.items():
    sample('quit', name, pid, 8)
for name, label, process, log in jobs:
    code = process.wait(timeout=30)
    log.close()
    events.write(json.dumps({'kind': 'sample_reaped', 'ts_ms': time.time_ns() // 1_000_000,
                            'name': name, 'label': label, 'returncode': code}) + '\n')
code = observer.wait(timeout=35)
events.write(json.dumps({'kind': 'observer_exit', 'ts_ms': time.time_ns() // 1_000_000,
                        'returncode': code}) + '\n')
if code:
    raise RuntimeError(f'Observer failed: {code}')
