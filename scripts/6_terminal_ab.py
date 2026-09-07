"""Replay identical ANSI cells comparing REP or synchronized output under 5_probe_guard.py.
DSR measures terminal processing acknowledgement, not paint completion.
"""
import argparse, hashlib, json, os, pathlib, re, select, termios, time, tty


def expand_rep(text):
    result = []
    previous = None
    for token in re.findall(r'\x1b\[[0-?]*[ -/]*[@-~]|[^\x1b]', text):
        match = re.fullmatch(r'\x1b\[(\d*)b', token)
        if match:
            if previous is None:
                raise ValueError('REP without preceding glyph')
            result.append(previous * max(1, int(match[1] or 1)))
        else:
            result.append(token)
            if not token.startswith('\x1b'):
                previous = token
    return ''.join(result).encode()


def run(directory, experiment):
    fd = os.open('/dev/tty', os.O_RDWR | os.O_NONBLOCK)
    original = termios.tcgetattr(fd)
    log = (directory / 'measurements.ndjson').open('x', buffering=1)
    def record(**row):
        log.write(json.dumps(row) + '\n')
    def send(payload):
        started = time.perf_counter_ns()
        offset = calls = blocked = syscall_ns = wait_ns = 0
        payload = memoryview(payload)
        while offset < len(payload):
            if time.perf_counter_ns()-started > 2_000_000_000:
                raise TimeoutError('write deadline')
            calls += 1
            try:
                call_started = time.perf_counter_ns()
                try:
                    count = os.write(fd, payload[offset:])
                finally:
                    syscall_ns += time.perf_counter_ns() - call_started
                if not count: raise RuntimeError('zero write')
                offset += count
            except BlockingIOError:
                blocked += 1
                wait_started = time.perf_counter_ns()
                select.select([], [fd], [], .002)
                wait_ns += time.perf_counter_ns() - wait_started
        return dict(write_us=(time.perf_counter_ns()-started)//1000,
                    write_calls=calls, blocked=blocked,
                    syscall_us=syscall_ns//1000, capacity_wait_us=wait_ns//1000)
    def ack():
        send(b'\x1b[6n')
        data = b''
        deadline = time.monotonic()+2
        while time.monotonic() < deadline:
            if select.select([fd], [], [], .01)[0]:
                data += os.read(fd, 4096)
                if b'q' in data or b'\x03' in data:
                    raise InterruptedError('quit')
                match = re.search(rb'\x1b\[(\d+);(\d+)R', data)
                if match:return tuple(map(int, match.groups()))
        raise TimeoutError('terminal acknowledgement deadline')
    try:
        tty.setraw(fd)
        size = os.get_terminal_size(fd)
        record(kind='start', terminal=[size.columns,size.lines], experiment=experiment)
        if (size.columns, size.lines) != (426, 135):
            raise ValueError('terminal must match recorded 426x135 dimensions')
        send(b'\x1b[?1049h\x1b[?25l\x1b[2J\x1b[H')
        send('▒\x1b[11b'.encode())
        cursor = ack()
        record(kind='unicode_rep', cursor=cursor)
        if cursor != (1,13):raise ValueError('Unicode REP unsupported')
        frames = sorted(directory.glob('frame-*.ansi'))
        if not 1 <= len(frames) <= 20 or sum(p.stat().st_size for p in frames) > 8 * 1024**2:
            raise ValueError('expected at most 20 frames and 8 MiB of payload')
        compressed = [p.read_bytes() for p in frames]
        literal = [expand_rep(p.read_text()) for p in frames]
        record(kind='payloads', compressed_bytes=sum(map(len,compressed)),
               literal_bytes=sum(map(len,literal)),
               sha256=[hashlib.sha256(p).hexdigest() for p in compressed])
        variants = {'literal': literal, 'rep': compressed}
        arms = ['literal', 'rep', 'rep', 'literal']
        if experiment == 'sync':
            variants['sync'] = [b'\x1b[?2026h' + p + b'\x1b[?2026l' for p in literal]
            arms = ['literal', 'sync', 'sync', 'literal']
        for arm in arms:
            payloads = variants[arm]
            send(b'\x1b[2J\x1b[H')
            for index, payload in enumerate(payloads):
                started = time.perf_counter_ns()
                metrics = send(payload)
                ack_started = time.perf_counter_ns()
                cursor = ack()
                ack_wait_us = (time.perf_counter_ns()-ack_started)//1000
                record(kind='frame', arm=arm, index=index, bytes=len(payload),
                       acknowledged_us=(time.perf_counter_ns()-started)//1000,
                       cursor=cursor, ack_wait_us=ack_wait_us, **metrics)
    except Exception as error:
        record(kind='error', error=str(error))
        raise
    finally:
        try:send(b'\x1b[?2026l\x1b[0m\x1b[?25h\x1b[?1049l')
        finally:
            termios.tcsetattr(fd, termios.TCSANOW, original)
            os.close(fd)
            log.close()

if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory',type=pathlib.Path)
    parser.add_argument('--experiment', choices=['rep', 'sync'], default='rep')
    args = parser.parse_args()
    run(args.directory, args.experiment)
