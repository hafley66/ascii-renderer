"""Replay identical ANSI cells with REP enabled/expanded under 5_probe_guard.py.
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


def run(directory):
    fd = os.open('/dev/tty', os.O_RDWR | os.O_NONBLOCK)
    original = termios.tcgetattr(fd)
    log = (directory / 'measurements.ndjson').open('x', buffering=1)
    def record(**row):
        log.write(json.dumps(row) + '\n')
    def send(payload):
        started = time.perf_counter_ns()
        offset = calls = blocked = 0
        while offset < len(payload):
            if time.perf_counter_ns()-started > 2_000_000_000:
                raise TimeoutError('write deadline')
            calls += 1
            try:
                count = os.write(fd, payload[offset:])
                if not count: raise RuntimeError('zero write')
                offset += count
            except BlockingIOError:
                blocked += 1
                select.select([], [fd], [], .002)
        return dict(write_us=(time.perf_counter_ns()-started)//1000,
                    write_calls=calls, blocked=blocked)
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
        record(kind='start', terminal=[size.columns,size.lines])
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
        for arm in ['literal','rep','rep','literal']:
            payloads = literal if arm=='literal' else compressed
            send(b'\x1b[2J\x1b[H')
            for index, payload in enumerate(payloads):
                started = time.perf_counter_ns()
                metrics = send(payload)
                cursor = ack()
                record(kind='frame', arm=arm, index=index, bytes=len(payload),
                       acknowledged_us=(time.perf_counter_ns()-started)//1000,
                       cursor=cursor, **metrics)
    except Exception as error:
        record(kind='error', error=str(error))
        raise
    finally:
        try:send(b'\x1b[0m\x1b[?25h\x1b[?1049l')
        finally:
            termios.tcsetattr(fd, termios.TCSANOW, original)
            os.close(fd)
            log.close()

if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory',type=pathlib.Path)
    run(parser.parse_args().directory)
