"""Measure actual demo animation in a dedicated iTerm window under probe_guard.
Use the same binary inputs and --frames for before/after runs. No screenshots.
"""
import argparse
import hashlib
import json
import os
import pathlib
import shlex
import subprocess
import time

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--window', type=int, required=True)
parser.add_argument('--directory', type=pathlib.Path, required=True)
parser.add_argument('--binary', type=pathlib.Path, required=True)
parser.add_argument('--watch-pid', type=int, required=True)
parser.add_argument('--frames', type=int, default=8)
parser.add_argument('--direct', action='store_true', help='enter the same app animation through its morph CLI')
args = parser.parse_args()
root = pathlib.Path(__file__).resolve().parent.parent
d = args.directory.resolve()
d.mkdir(parents=True, exist_ok=False)
(d / 'config/ascii-renderer').mkdir(parents=True)
fixture = json.loads((root / 'perf/fixtures/12_gem_aetherium_2_bad_roll6.json').read_text())
(d / 'config/ascii-renderer/options.tsv').write_text('__global\tRAND\t0\n' + ''.join(
    f'gem-aetherium-2\t{k}\t{v}\n' for k, v in fixture['knobs'].items()))
binary = args.binary.resolve()
(d / 'metadata.json').write_text(json.dumps(dict(binary=str(binary),
    sha256=hashlib.sha256(binary.read_bytes()).hexdigest(), requested_terminal=[400,200],
    frames=args.frames, direct=args.direct, knobs=fixture['knobs']), indent=2) + '\n')
log = (d / 'ui.ndjson').open('a', buffering=1)
def script(source):
    result = subprocess.run(['osascript', '-e', source], capture_output=True, text=True, timeout=3)
    log.write(json.dumps(dict(ts_ms=time.time_ns()//1000000, code=result.returncode,
                             out=result.stdout, error=result.stderr)) + '\n')
    if result.returncode:
        raise RuntimeError(result.stderr)
def key(value):
    value = f'(ASCII character {ord(value)})' if len(value)==1 and ord(value)<32 else json.dumps(value)
    script(f'tell application id "com.googlecode.iterm2" to tell current session of window id {args.window} to write text {value} newline NO')
def records(name):
    path = d / name
    result = []
    if path.exists():
        for line in path.read_text().splitlines():
            try: result.append(json.loads(line))
            except ValueError: pass
    return result
def wait(predicate):
    deadline = time.monotonic() + 6
    while time.monotonic() < deadline:
        if any(r['kind']=='breaker' for r in records('guard.ndjson')):
            raise RuntimeError('watchdog stopped workload')
        if predicate(records('animation.ndjson')):
            return
        time.sleep(.01)
    raise TimeoutError('app transition deadline')
command = ['env', f'XDG_CONFIG_HOME={d}/config', f'ASCII_TRACE_PATH={d}/animation.ndjson',
           'ASCII_TRACE_ALL=1', '/usr/bin/python3', str(root/'scripts/5_probe_guard.py'),
           '--state', str(d/'guard.ndjson'), '--artifact-dir', str(d),
           '--watch-pid', str(args.watch_pid), '--owner-pid', str(os.getpid()),
           '--max-growth-mib', '256',
           '--', str(binary)]
command += ['42', 'morph', '', 'gem-aetherium-2', '42', 'gem-aetherium-2', '43', 'iterate'] if args.direct else ['42', 'demo']
script(f'tell application id "com.googlecode.iterm2" to tell current session of window id {args.window}\nset columns to 400\nset rows to 200\nend tell')
key('\x15')
script(f'tell application id "com.googlecode.iterm2" to tell current session of window id {args.window} to write text {json.dumps(shlex.join(command))}')
if not args.direct:
    wait(lambda rows: any(r.get('stage')=='session_exit' for r in rows))
    time.sleep(.1)
    key('/')
    time.sleep(.2)
    key('gem-aetherium-2')
    time.sleep(.2)
    key('\r')
    wait(lambda rows: sum(r.get('stage')=='session_exit' for r in rows)>=2)
    time.sleep(.1)
    key('a')
wait(lambda rows: sum('frame_index' in r for r in rows)>=args.frames)
key('\x03')
deadline = time.monotonic()+3
while time.monotonic()<deadline:
    if any(r['kind'] in ('completed','stopped') for r in records('guard.ndjson')):
        break
    time.sleep(.03)
print(json.dumps(dict(directory=str(d), frames=sum('frame_index' in r for r in records('animation.ndjson')),
                      guard=records('guard.ndjson')[-1])), flush=True)
