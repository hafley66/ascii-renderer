"""Pexpect transport and pyte terminal cells for the shared real-demo E2E workflow."""
import asyncio
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import threading
import time
from types import SimpleNamespace
import pexpect
import pyte

ROOT = Path(__file__).resolve().parent.parent
spec = importlib.util.spec_from_file_location('e2e_base', ROOT/'scripts/12_test_e2e.py')
base = importlib.util.module_from_spec(spec)
spec.loader.exec_module(base)


class HeadlessCase(base.DemoCase):
    async def open_terminal(self, size):
        self.lock = threading.Lock()
        self.cells = pyte.Screen(*size)
        self.stream = pyte.ByteStream(self.cells)
        self.terminal = pexpect.spawn('/bin/bash', ['--noprofile', '--norc', '-i'],
            cwd=str(ROOT), env=dict(os.environ, TERM='xterm-256color'),
            dimensions=(size[1], size[0]), encoding=None, maxread=65536)
        self.terminal.delaybeforesend = None
        self.session = SimpleNamespace(async_send_text=self.send_text, async_set_grid_size=self.resize)
        self.window = SimpleNamespace(async_close=self.close_terminal)
        self.output = (self.d/'terminal.ansi').open('wb')
        self.reader = asyncio.create_task(self.read_output())
        self.result['scope'] = 'actual demo and PTY, pyte terminal cells; no GUI paint measurement'
        actual = self.terminal.getwinsize()
        assert actual == (size[1], size[0]), actual
        base.support.write_json(self.d/'terminal.json', {
            'requested': list(size), 'actual': [actual[1], actual[0]],
            'transport': 'pexpect', 'screen': 'pyte', 'focus_required': False})
        return None

    async def read_output(self):
        try:
            while True:
                try:
                    data = self.terminal.read_nonblocking(65536, timeout=0)
                except pexpect.TIMEOUT:
                    await asyncio.sleep(.002)
                    continue
                self.output.write(data)
                self.output.flush()
                def feed():
                    with self.lock:
                        self.stream.feed(data)
                await asyncio.to_thread(feed)
        except asyncio.CancelledError:
            raise
        except Exception as error:
            self.observer_error = f'PTY reader: {type(error).__name__}: {error}'

    async def heartbeat(self):
        try:
            while True:
                alive = self.terminal.isalive() and not self.reader.done()
                state = {'ts_ms': time.time_ns()//1_000_000,
                         'mode': 'headless', 'transport_alive': alive}
                if self.observer_error:
                    state['error'] = self.observer_error
                base.support.write_json(self.d/'observer.json', state)
                if not alive:
                    self.observer_error = 'headless transport stopped'
                    return
                await asyncio.sleep(.1)
        except asyncio.CancelledError:
            raise
        except Exception as error:
            self.observer_error = f'headless heartbeat: {error}'

    async def send_text(self, text, suppress_broadcast=True):
        self.terminal.send(text.encode())

    async def resize(self, size):
        with self.lock:
            self.cells.resize(lines=size.height, columns=size.width)
        self.terminal.setwinsize(size.height, size.width)

    async def screen(self):
        self.healthy()
        with self.lock:
            # pyte 0.8.2 display indexes char[0] on an empty wide-cell marker.
            # Read the library's parsed cells directly, preserving that marker.
            return '\n'.join(''.join(self.cells.buffer[y][x].data
                for x in range(self.cells.columns)) for y in range(self.cells.lines))

    async def capture(self, name):
        self.healthy()
        with self.lock:
            width = max(1, self.cells.columns-34)
            rows = [[tuple(self.cells.buffer[y][x]) for x in range(width)]
                    for y in range(self.cells.lines-1)]
        assert any(c[0].strip() for row in rows for c in row), 'art cells are blank'
        encoded = json.dumps(rows, ensure_ascii=False).encode()
        (self.d/f'{name}.cells.json').write_bytes(encoded)
        return hashlib.sha256(encoded).hexdigest()

    async def close_terminal(self, force=True):
        if hasattr(self, 'reader'):
            self.reader.cancel()
            await asyncio.gather(self.reader, return_exceptions=True)
        if hasattr(self, 'terminal'):
            self.terminal.close(force=force)
        if hasattr(self, 'output'):
            self.output.close()


async def run(args):
    names = ['workflow', 'bad-400x200', 'max-400x200', 'backpressure'] if args.case == 'all' else [args.case]
    report = {'status': 'running', 'complete_suite': args.case == 'all',
              'scope': 'headless real demo/PTY and terminal-cell assertions; GUI painting untested',
              'function_trace': args.function_trace,
              'cases': [{'case': name, 'status': 'not_run'} for name in names]}
    base.support.write_json(args.directory/'report.json', report)
    for index, name in enumerate(names):
        if name == 'backpressure':
            result = await asyncio.to_thread(base.backpressure_case, args)
        else:
            case = HeadlessCase(None, None, args, name, args.directory/name, args.binary)
            try:
                await case.run()
            except Exception as error:
                import traceback
                case.d.mkdir(parents=True, exist_ok=True)
                (case.d/'failure.txt').write_text(traceback.format_exc())
                case.result.update(status='failed', error=f'{type(error).__name__}: {error}')
            finally:
                try:
                    await case.cleanup()
                except Exception as error:
                    case.result.update(status='failed', cleanup_error=str(error))
                    await case.close_terminal()
            result = case.result
        report['cases'][index] = result
        base.support.write_json(args.directory/'report.json', report)
        print(json.dumps({k: result[k] for k in ('case','status','error') if k in result}), flush=True)
        if any(row.get('kind') == 'breaker' for row in result.get('guard', [])):
            break
    report['status'] = 'passed' if all(c['status'] == 'passed' for c in report['cases']) else 'failed'
    base.support.write_json(args.directory/'report.json', report)
