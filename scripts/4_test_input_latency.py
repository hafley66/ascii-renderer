#!/usr/bin/env python3
"""Measure input under a stalled terminal consumer; config and renderer are isolated.
The reader pauses until knob application or --stall seconds; quit measures
terminal restoration separately from process exit while reads remain paused.
Usage: python3 scripts/4_test_input_latency.py [binary] --size 286x103 --stall 10
"""
import argparse
import fcntl
import json
import os
from pathlib import Path
import pty
import select
import shlex
import struct
import subprocess
import tempfile
import termios
import threading
import time

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('binary', nargs='?', default='target/release/ascii-renderer')
parser.add_argument('--mode', default='gem-aetherium-2')
parser.add_argument('--size', default='286x103')
parser.add_argument('--stall', type=float, default=10)
parser.add_argument('--key', choices=['both', 'knob', 'quit'], default='both')
parser.add_argument('--tmux', action='store_true', help='include an isolated tmux server and client')
parser.add_argument('--sampled', action='store_true', help='verify default periodic traces with slow-frame logging suppressed')
parser.add_argument('--max-ms', type=float, help='fail if applying an input exceeds this latency')
args = parser.parse_args()
binary = str(Path(args.binary).resolve())
width, height = map(int, args.size.split('x'))
fixture = json.loads(subprocess.check_output([binary, 'inputs', args.mode, 'max']))


def check(key):
    with tempfile.TemporaryDirectory(prefix='ascii-input-latency-') as directory:
        directory = Path(directory)
        config = directory / 'ascii-renderer' / 'options.tsv'
        config.parent.mkdir()
        initial_config = '__global\tRAND\t0\n' + ''.join(
            f'{args.mode}\t{k}\t{v}\n' for k, v in fixture['knobs'].items())
        config.write_text(initial_config)
        trace = directory / 'frames.ndjson'
        gate_read, gate_write = os.pipe()
        pid, fd = pty.fork()
        if pid == 0:
            os.close(gate_write)
            os.read(gate_read, 1)
            os.close(gate_read)
            env = dict(os.environ, XDG_CONFIG_HOME=str(directory),
                       ASCII_TRACE_PATH=str(trace), ASCII_TRACE_ALL='1')
            if args.sampled:
                env.update(ASCII_TRACE_ALL='0', ASCII_TRACE_SLOW_MS='3600000')
            command = [binary, '42', 'morph', '', args.mode, '42', args.mode, '43', 'iterate']
            if args.tmux:
                env['TERM'] = 'xterm-256color'
                os.execvpe('tmux', ['tmux', '-S', str(directory / 'tmux.sock'), '-f', '/dev/null',
                                    'new-session', '-s', 'latency', shlex.join(command)], env)
            os.execve(binary, command, env)
        os.close(gate_read)
        fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack('HHHH', height + 1 + int(args.tmux), width + 34, 0, 0))
        initial_term = termios.tcgetattr(fd)
        os.write(gate_write, b'1')
        os.close(gate_write)
        reading = threading.Event()
        reading.set()
        stop = threading.Event()
        tail = bytearray()
        def drain():
            while not stop.is_set():
                if not reading.wait(.01):
                    continue
                if select.select([fd], [], [], .002)[0]:
                    try:
                        data = os.read(fd, 65536)
                        if not data: return
                        tail.extend(data)
                        del tail[:-8192]
                    except OSError:
                        return
        thread = threading.Thread(target=drain)
        thread.start()
        exited = False
        def reaped():
            nonlocal exited
            if exited: return True
            done, status = os.waitpid(pid, os.WNOHANG)
            if done:
                exited = True
                assert os.waitstatus_to_exitcode(status) == 0, tail.decode(errors='replace')
            return exited
        try:
            deadline = time.monotonic() + 10
            while not trace.exists() or not trace.read_bytes().endswith(b'\n'):
                assert not reaped(), tail.decode(errors='replace')
                assert time.monotonic() < deadline, 'first frame timed out'
                time.sleep(.002)
            if args.sampled:
                deadline = time.monotonic() + 5
                while True:
                    samples = [json.loads(line) for line in trace.read_text().split('\n')[:-1]]
                    if len(samples) >= 3:
                        break
                    assert time.monotonic() < deadline, 'periodic frame samples missing'
                    time.sleep(.01)
                assert samples[0]['frame_index'] == 1
                assert all(e['sampled'] and e['pid'] > 0 for e in samples)
                assert all(e['grid'] == {'w': width, 'h': height} and e['knobs'] == fixture['knobs'] for e in samples)
                assert all(e['interval_frames'] > 1 and e['interval_bytes'] >= e['bytes'] and e['interval_ms'] >= 1000 for e in samples[1:])
                print(json.dumps({'periodic_samples': len(samples), 'last_frame_index': samples[-1]['frame_index']}), flush=True)
            reading.clear()
            # Let the in-flight read finish and fill the PTY and worker pipe.
            time.sleep(.25)
            before = config.read_text()
            sent = time.monotonic()
            os.write(fd, key)
            print(json.dumps(dict(pid=pid, key=key.decode(), sent_epoch_ms=time.time_ns() // 1_000_000)), flush=True)
            applied = None
            resumed = None
            restored_at = None
            while time.monotonic() - sent < args.stall + 3:
                now = time.monotonic()
                if now - sent >= args.stall and resumed is None:
                    resumed = now
                    reading.set()
                if key == b'q':
                    if restored_at is None and termios.tcgetattr(fd)[3] & termios.ICANON:
                        restored_at = now
                    if reaped():
                        applied = now
                        break
                elif config.read_text() != before:
                    applied = now
                    break
                time.sleep(.002)
            completed = restored_at if key == b'q' else applied
            elapsed = None if completed is None else (completed - sent) * 1000
            result = dict(mode=args.mode, grid=[width, height], key=key.decode(), tmux=args.tmux,
                          consumer_stall_ms=args.stall * 1000, input_applied_ms=elapsed,
                          process_exit_ms=(applied - sent) * 1000 if key == b'q' and applied else None,
                          terminal_restored_ms=None if restored_at is None else (restored_at - sent) * 1000,
                          applied_while_output_blocked=completed is not None and (resumed is None or completed < resumed))
            print(json.dumps(result), flush=True)
            reading.set()
            if not exited:
                os.write(fd, b'q')
                deadline = time.monotonic() + 3
                while not reaped() and time.monotonic() < deadline:
                    time.sleep(.002)
            assert exited, 'quit cleanup timed out'
            restored = termios.tcgetattr(fd)
            mask = termios.ICANON | termios.ISIG
            assert restored[3] & mask == initial_term[3] & mask
            return elapsed
        finally:
            reading.set()
            if not exited:
                os.write(fd, b'q')
                deadline = time.monotonic() + 3
                while not reaped() and time.monotonic() < deadline:
                    time.sleep(.002)
            stop.set()
            thread.join(timeout=1)
            os.close(fd)
            if args.tmux:
                subprocess.run(['tmux', '-S', str(directory / 'tmux.sock'), 'kill-server'],
                               stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)


keys = {'both': [b'-', b'q'], 'knob': [b'-'], 'quit': [b'q']}[args.key]
latencies = [check(key) for key in keys]
if args.max_ms is not None:
    assert all(ms is not None and ms <= args.max_ms for ms in latencies), latencies
