#!/usr/bin/env python3
"""Native animation stress through the actual Unix PTY supervisor.
Usage: python3 scripts/3_test_animation.py --mode gem-aetherium-2 --max --size 2000x2000
Builds release, isolates config, saves input/timing NDJSON, drains PTY (no emulator painting).
"""
import argparse
import subprocess
import shlex
from pathlib import Path
import fcntl
import json
import os
import pty
import select
import signal
import statistics
import struct
import sys
import tempfile
import termios
import time
import threading

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('binary', nargs='?', help='existing binary; otherwise build release automatically')
parser.add_argument('--mode', default='gem-aetherium-2')
parser.add_argument('--max', action='store_true', help='set every declared knob to maximum simultaneously')
parser.add_argument('--size', action='append', help='render WIDTHxHEIGHT; repeat for multiple sizes')
parser.add_argument('--frames', type=int, default=100)
parser.add_argument('--timeout', type=float, default=120, help='seconds allowed per size')
parser.add_argument('--trace', help='new NDJSON output file (default: perf/results timestamped file)')
args = parser.parse_args()
if args.frames < 1 or args.timeout <= 0:
    parser.error('frames and timeout must be positive')
sizes = []
for size in args.size or (['2000x2000'] if args.max else ['320x103', '2000x2000']):
    try:
        width, height = map(int, size.lower().split('x'))
        if not (0 < width <= 65501 and 0 < height <= 65534):
            raise ValueError()
    except ValueError:
        parser.error(f'invalid PTY render size: {size}')
    sizes.append((width, height))
if args.binary is None:
    subprocess.run(['cargo', 'build', '--release', '--locked'], check=True)
binary = os.path.abspath(args.binary or 'target/release/ascii-renderer')
fixture = json.loads(subprocess.check_output([binary, 'inputs', args.mode, 'max' if args.max else 'default']))
trace = Path(args.trace or f'perf/results/{args.mode}-{time.time_ns() // 1_000_000}.ndjson').resolve()
trace.parent.mkdir(parents=True, exist_ok=True)
# Exclusive creation preserves earlier measurements and stable replay line numbers.
trace.touch(exist_ok=False)
print(f'Trace: {trace}', flush=True)


def check(width, height):
    with tempfile.TemporaryDirectory(prefix='ascii-animation-') as directory:
        config = Path(directory) / 'ascii-renderer'
        config.mkdir()
        (config / 'options.tsv').write_text('__global\tRAND\t0\n' + ''.join(
            f'{args.mode}\t{key}\t{value}\n' for key, value in fixture['knobs'].items()))
        first_line = len(trace.read_text().splitlines())
        gate_read, gate_write = os.pipe()
        pid, fd = pty.fork()
        if pid == 0:
            os.close(gate_write)
            os.read(gate_read, 1)
            os.close(gate_read)
            env = dict(os.environ, ASCII_TRACE_PATH=str(trace), ASCII_TRACE_ALL='1',
                       XDG_CONFIG_HOME=directory)
            os.execve(binary, [binary, '42', 'morph', 'auto', args.mode,
                               '42', args.mode, '43', 'iterate'], env)
        fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack('HHHH', height + 1, width + 34, 0, 0))
        initial = termios.tcgetattr(fd)
        os.close(gate_read)
        os.write(gate_write, b'1')
        os.close(gate_write)
        exited = False
        stop = threading.Event()
        output_tail = bytearray()
        def drain():
            while not stop.is_set():
                if select.select([fd], [], [], .01)[0]:
                    try:
                        chunk = os.read(fd, 65536)
                        if not chunk:
                            return
                        output_tail.extend(chunk)
                        del output_tail[:-8192]
                    except OSError:
                        return
        reader = threading.Thread(target=drain)
        reader.start()
        frames = []
        seen = 0
        sent_roll = 0
        started = time.monotonic()
        try:
            deadline = started + args.timeout
            while len(frames) < args.frames:
                assert time.monotonic() < deadline, f'{width}x{height}: timed out at {len(frames)} frames'
                done, status = os.waitpid(pid, os.WNOHANG)
                if done:
                    exited = True
                    reader.join(timeout=1)
                    raise AssertionError(f'animation exited {os.waitstatus_to_exitcode(status)}: {output_tail.decode(errors="replace")}')
                time.sleep(.01)
                if os.path.exists(trace):
                    with open(trace) as file:
                        lines = file.read().split('\n')[first_line:-1]
                    events = [json.loads(line) for line in lines]
                    frames = [event for event in events if event['kind'] == 'animation_frame' or event['kind'] == 'slow_animation_frame']
                    # Enable random controls in the isolated config, then reroll.
                    if not args.max and frames and seen == 0:
                        os.write(fd, b'g')
                    if not args.max and frames and len(frames) // 4 > sent_roll:
                        os.write(fd, b'+')
                        sent_roll = len(frames) // 4
                    seen = len(frames)
            # No endpoint CLI render should run during native startup or cycle wrap.
            assert all(event['kind'] in ('animation_frame', 'slow_animation_frame') for event in events)
            if args.frames >= 100:
                assert frames[-1]['time'] > 5.4, frames[-1]
            if args.max:
                assert all(event['knobs'] == fixture['knobs'] for event in frames), 'knobs differed from declared maxima'
            elif args.frames >= 40:
                assert len({event['roll'] for event in frames}) >= 10
                assert len({json.dumps(event['knobs'], sort_keys=True) for event in frames}) >= 10
            assert all(event['grid'] == {'w': width, 'h': height} for event in frames)
            assert all(event['terminal_size'] == {'w': width + 34, 'h': height + 1} for event in frames)
            for stage in ('render_us', 'encoding_us', 'presentation_us', 'dur_us'):
                values = [event[stage] / 1000 for event in frames]
                print(f'{width}x{height} {stage}: median={statistics.median(values):.3f} ms max={max(values):.3f} ms')
            print(f'{width}x{height}: {len(frames)} frames, {len({e["roll"] for e in frames})} rolls, wall={time.monotonic()-started:.3f}s')
            slowest = max(range(len(events)), key=lambda i: events[i]['dur_us'])
            print('Replay slowest frame: ' + shlex.join([binary, 'replay', str(trace), str(first_line + slowest + 1)]), flush=True)
            if width == 320 and not args.max:
                # Deferred endpoints must still initialize when leaving native mode.
                for key, strategy in [(b'5', 'wind'), (b'i', 'iterate')]:
                    before = len(events)
                    os.write(fd, key)
                    until = time.monotonic() + 10
                    while time.monotonic() < until:
                        time.sleep(.01)
                        with open(trace) as file:
                            events = [json.loads(line) for line in file.read().split('\n')[first_line:-1]]
                        if any(e.get('strategy') == strategy for e in events[before:]):
                            break
                    else:
                        raise AssertionError(f'strategy switch failed: {strategy}')
            os.write(fd, b'q')
            quit_at = time.monotonic()
            while time.monotonic() - quit_at < 2:
                time.sleep(.002)
                done, status = os.waitpid(pid, os.WNOHANG)
                if done:
                    exited = True
                    assert os.waitstatus_to_exitcode(status) == 0
                    restored = termios.tcgetattr(fd)
                    mask = termios.ICANON | termios.ISIG
                    assert restored[3] & mask == initial[3] & mask
                    break
            assert exited, 'quit did not finish'
        finally:
            if not exited:
                # Let the supervisor cancel its worker process group first.
                os.write(fd, b'q')
                until = time.monotonic() + 2
                while time.monotonic() < until:
                    time.sleep(.002)
                    if os.waitpid(pid, os.WNOHANG)[0]:
                        exited = True
                        break
                if not exited:
                    os.kill(pid, signal.SIGKILL)
                    os.waitpid(pid, 0)
            stop.set()
            reader.join(timeout=1)
            os.close(fd)


for width, height in sizes:
    check(width, height)
