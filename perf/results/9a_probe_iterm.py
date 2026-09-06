"""Run the known GUI repro with libc I/O counters in renderer processes only."""
import argparse
import json
import os
import pathlib
import signal
import subprocess
import time

parser = argparse.ArgumentParser()
parser.add_argument('--window-id', type=int, required=True)
parser.add_argument('--output', type=pathlib.Path, required=True)
parser.add_argument('--iterm-pid', type=int, required=True)
parser.add_argument('--hold-seconds', type=int, default=3)
parser.add_argument('--no-vmmap', action='store_true')
args = parser.parse_args()
base = pathlib.Path(__file__).resolve().parent
directory = args.output.resolve()
fixture = base / '7_iterm_uncaptured_1788718894474'
binary = base.parents[1] / 'target/release/ascii-renderer'
probe = pathlib.Path('/private/tmp/ascii_io_probe.dylib')
guard = base.parents[1] / 'scripts/5_probe_guard.py'
guard_command = (f'/usr/bin/python3 {guard} --state {directory}/guard.ndjson '
                 f'--artifact-dir {directory} --owner-pid {os.getpid()} '
                 f'--watch-pid {args.iterm_pid} -- ')
subprocess.run(['clang', '-dynamiclib', '-O2', '-Wall', '-o', str(probe),
                str(base / '9_io_probe.c')], check=True)
source = (fixture / 'observe.py').read_text().replace(
    'T=pathlib.Path(__file__).resolve().parent', f'T=pathlib.Path({str(fixture)!r})')
source = source.replace('time.sleep(18)', f'time.sleep({args.hold_seconds})')
guard_check = '''def check_guard():
 p=D/'guard.ndjson'
 if p.exists():
  for line in p.read_text().splitlines():
   try: row=json.loads(line)
   except json.JSONDecodeError: continue
   if row.get('kind')=='breaker': raise RuntimeError('Probe circuit breaker: '+row['reason'])
'''
source = source.replace('def script(source,label,timeout=20):',
                        guard_check+'def script(source,label,timeout=20):\n check_guard()')
source = source.replace('    while time.monotonic()<end:\n',
                        '    while time.monotonic()<end:\n        check_guard()\n')
source = source.replace("(T/'run.sh').read_text().replace(str(T),str(D))",
    "(T/'run.sh').read_text().replace(str(T),str(D)).replace("
    "'/Users/chrishafley/.cargo/bin/cargo run --release -- 42 demo', "
    f"'{guard_command}/usr/bin/env DYLD_INSERT_LIBRARIES={probe} ASCII_SYSCALL_DIR={directory} {binary} 42 demo')")
observer_path = pathlib.Path('/private/tmp/ascii_io_observe.py')
observer_path.write_text(source)
observer = subprocess.Popen(['/usr/bin/python3', str(observer_path), '--window-id',
                             str(args.window_id), '--output', str(directory)])
while not directory.exists():
    if observer.poll() is not None:
        raise RuntimeError('Observer failed during setup')
    time.sleep(.02)
memory = (directory / 'memory.ndjson').open('a', buffering=1)
known = {args.iterm_pid}
animation_pids = set()
mapped = set()
maps = []
started = time.monotonic()
while time.monotonic() - started < args.hold_seconds + 85:
    guard_state = directory / 'guard.ndjson'
    if guard_state.exists() and '"kind": "breaker"' in guard_state.read_text():
        observer.terminate()
        break
    rows = []
    path = directory / 'animation.ndjson'
    if path.exists():
        for line in path.read_text().splitlines():
            try:
                rows.append(json.loads(line))
            except json.JSONDecodeError:
                pass
    for row in rows:
        if row.get('pid'):
            known.add(row['pid'])
        if row.get('animation') and row.get('kind') == 'playback_relay':
            animation_pids.update((row['pid'], row['worker_pid']))
    # ps exposes size counters only. No tracing attaches to shared iTerm2.
    result = subprocess.run(['ps', '-p', ','.join(map(str, sorted(known))),
                             '-o', 'pid=,rss=,vsz='], capture_output=True, text=True)
    for line in result.stdout.splitlines():
        pid, rss, vsz = map(int, line.split())
        memory.write(json.dumps({'ts_ms': time.time_ns() // 1_000_000, 'pid': pid,
                                 'rss_kib': rss, 'vsz_kib': vsz}) + '\n')
    for pid in animation_pids:
        if args.no_vmmap:
            continue
        label = 'late' if time.monotonic() - started > 20 else 'early'
        if (pid, label) not in mapped:
            mapped.add((pid, label))
            log = (directory / f'vmmap-{pid}-{label}.txt').open('w')
            maps.append((subprocess.Popen(['vmmap', '-summary', str(pid)], stdout=log,
                                           stderr=subprocess.STDOUT), log))
    if observer.poll() is not None:
        break
    time.sleep(.5)
for process, log in maps:
    process.wait(timeout=15)
    log.close()
observer_code = observer.wait(timeout=10)
metadata = json.loads((directory / 'metadata.json').read_text())
metadata.update(created_ms=time.time_ns() // 1_000_000,
                scenario='same release binary, raw iTerm all-max demo, renderer-only libc interposition',
                renderer_pids=sorted(animation_pids), iterm_pid=args.iterm_pid,
                shared_iterm_tracing=False, hold_seconds=args.hold_seconds,
                vmmap_enabled=not args.no_vmmap, window_id=args.window_id,
                observer_exit_code=observer_code)
metadata['watchdog'] = dict(max_seconds=15, max_owned_mib=256, max_watched_mib=768,
                           max_growth_mib=128, max_artifact_mib=32, min_free_gib=2)
(directory / 'metadata.json').write_text(json.dumps(metadata, indent=2) + '\n')
if observer_code:
    # A lost-focus guard must also stop the experiment it launched. These are
    # fresh renderer PIDs from this run's own frame records, never iTerm2's PID.
    workers = {row['pid'] for row in rows if 'frame_index' in row}
    for pid in workers:
        try:
            os.kill(pid, signal.SIGTERM)
        except ProcessLookupError:
            pass
    subprocess.run(['osascript', '-e',
        f'tell application id "com.googlecode.iterm2" to close window id {args.window_id}'],
        timeout=20, check=False)
    raise RuntimeError('Observer failed; dedicated run stopped')
