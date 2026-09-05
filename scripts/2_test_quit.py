#!/usr/bin/env python3
"""PTY regression: 2000x1000, quit during startup and terminal backpressure.
Usage: python3 scripts/2_test_quit.py [target/debug/ascii-renderer]
"""
import fcntl
import os
import pty
import select
import signal
import struct
import sys
import termios
import time

binary = os.path.abspath(sys.argv[1] if len(sys.argv) > 1 else 'target/debug/ascii-renderer')


def check(args, key, delay, drain, animate=False):
    start_read, start_write = os.pipe()
    pid, fd = pty.fork()
    if pid == 0:
        os.close(start_write)
        os.read(start_read, 1)
        os.close(start_read)
        os.execv(binary, [binary, *args])
    fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack('HHHH', 1000, 2000, 0, 0))
    initial = termios.tcgetattr(fd)
    os.close(start_read)
    os.write(start_write, b'1')
    os.close(start_write)
    exited = False
    try:
        deadline = time.monotonic() + 2
        # Wait for raw mode, which makes Ctrl+C a keyboard event.
        while termios.tcgetattr(fd)[3] & termios.ICANON:
            assert time.monotonic() < deadline, 'raw mode never enabled'
            time.sleep(.002)
        if animate:
            os.write(fd, b'a')
        until = time.monotonic() + delay
        while time.monotonic() < until:
            if drain and select.select([fd], [], [], .002)[0]:
                os.read(fd, 65536)
            else:
                time.sleep(.002)
        started = time.monotonic()
        os.write(fd, key)
        while time.monotonic() - started < 1:
            # Resume consuming after quit so the PTY can deliver terminal cleanup.
            if select.select([fd], [], [], 0)[0]:
                try: os.read(fd, 65536)
                except OSError: pass
            done, status = os.waitpid(pid, os.WNOHANG)
            if done:
                exited = True
                assert os.waitstatus_to_exitcode(status) == 0, status
                restored = termios.tcgetattr(fd)
                assert restored[3] & (termios.ICANON | termios.ISIG) == initial[3] & (termios.ICANON | termios.ISIG)
                print(f'{args[1]} animate={animate} key={key[-1:]!r} queued={len(key)-1} delay={delay}s drain={drain}: {(time.monotonic()-started)*1000:.1f} ms')
                return
            time.sleep(.002)
        raise AssertionError('quit took more than 1 second')
    finally:
        if not exited:
            os.kill(pid, signal.SIGKILL)
            os.set_blocking(fd, False)
            cleanup_deadline = time.monotonic() + 2
            while time.monotonic() < cleanup_deadline:
                try: os.read(fd, 65536)
                except OSError: pass
                if os.waitpid(pid, os.WNOHANG)[0]: break
                time.sleep(.002)
        os.close(fd)


for key in (b'q', b'Q', b'\x03'):
    for delay, drain in ((.03, True), (1, False)):
        check(['42', 'morph', 'auto', 'azulejo', '42', 'azulejo', '43', 'iterate'], key, delay, drain)
        check(['42', 'demo'], key, delay, drain)

check(['42', 'demo'], b'o' * 512 + b'\x03', 1, False, animate=True)
