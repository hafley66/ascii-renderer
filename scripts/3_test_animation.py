#!/usr/bin/env python3
"""Native animation + random knob rerolls through the actual Unix PTY supervisor.
Usage: python3 scripts/3_test_animation.py [target/release/ascii-renderer]
Uses isolated config/logs and a continuously drained PTY (no emulator painting).
"""
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

binary = os.path.abspath(sys.argv[1] if len(sys.argv) > 1 else 'target/release/ascii-renderer')


def check(width, height):
    with tempfile.TemporaryDirectory(prefix='ascii-animation-') as directory:
        trace = os.path.join(directory, 'renders.ndjson')
        gate_read, gate_write = os.pipe()
        pid, fd = pty.fork()
        if pid == 0:
            os.close(gate_write)
            os.read(gate_read, 1)
            os.close(gate_read)
            env = dict(os.environ, ASCII_TRACE_PATH=trace, ASCII_TRACE_ALL='1',
                       XDG_CONFIG_HOME=directory)
            os.execve(binary, [binary, '42', 'morph', 'auto', 'gem-aetherium-2',
                               '42', 'gem-aetherium-2', '43', 'iterate'], env)
        fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack('HHHH', height + 1, width + 34, 0, 0))
        initial = termios.tcgetattr(fd)
        os.close(gate_read)
        os.write(gate_write, b'1')
        os.close(gate_write)
        exited = False
        stop = threading.Event()
        def drain():
            while not stop.is_set():
                if select.select([fd], [], [], .01)[0]:
                    try:
                        if not os.read(fd, 65536):
                            return
                    except OSError:
                        return
        reader = threading.Thread(target=drain)
        reader.start()
        frames = []
        seen = 0
        sent_roll = 0
        started = time.monotonic()
        try:
            deadline = started + 60
            while len(frames) < 100:
                assert time.monotonic() < deadline, f'{width}x{height}: timed out at {len(frames)} frames'
                time.sleep(.01)
                if os.path.exists(trace):
                    with open(trace) as file:
                        lines = file.read().split('\n')[:-1]
                    events = [json.loads(line) for line in lines]
                    frames = [event for event in events if event['kind'] == 'animation_frame' or event['kind'] == 'slow_animation_frame']
                    # Enable random controls in the isolated config, then reroll.
                    if frames and seen == 0:
                        os.write(fd, b'g')
                    if frames and len(frames) // 4 > sent_roll:
                        os.write(fd, b'+')
                        sent_roll = len(frames) // 4
                    seen = len(frames)
            # No endpoint CLI render should run during native startup or cycle wrap.
            assert all(event['kind'] in ('animation_frame', 'slow_animation_frame') for event in events)
            assert frames[-1]['time'] > 5.4, frames[-1]
            assert len({event['roll'] for event in frames}) >= 10
            assert len({json.dumps(event['knobs'], sort_keys=True) for event in frames}) >= 10
            assert all(event['grid'] == {'w': width, 'h': height} for event in frames)
            assert all(event['terminal_size'] == {'w': width + 34, 'h': height + 1} for event in frames)
            for stage in ('render_us', 'encoding_us', 'presentation_us', 'dur_us'):
                values = [event[stage] / 1000 for event in frames]
                print(f'{width}x{height} {stage}: median={statistics.median(values):.3f} ms max={max(values):.3f} ms')
            print(f'{width}x{height}: {len(frames)} frames, {len({e["roll"] for e in frames})} rolls, wall={time.monotonic()-started:.3f}s')
            if width == 320:
                # Deferred endpoints must still initialize when leaving native mode.
                for key, strategy in [(b'5', 'wind'), (b'i', 'iterate')]:
                    before = len(events)
                    os.write(fd, key)
                    until = time.monotonic() + 10
                    while time.monotonic() < until:
                        time.sleep(.01)
                        with open(trace) as file:
                            events = [json.loads(line) for line in file.read().split('\n')[:-1]]
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
                os.kill(pid, signal.SIGKILL)
                # Drain while reaping: macOS PTY teardown can wait for output.
                until = time.monotonic() + 2
                while time.monotonic() < until:
                    time.sleep(.002)
                    if os.waitpid(pid, os.WNOHANG)[0]:
                        break
            stop.set()
            reader.join(timeout=1)
            os.close(fd)


check(320, 103)
check(2000, 2000)
