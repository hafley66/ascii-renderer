"""Native portable-pty/vt100 terminal; Python exchanges only E2E control messages."""
import asyncio
import hashlib
import importlib.util
import json
import os
import re
from pathlib import Path
import time
from types import SimpleNamespace

ROOT = Path(__file__).resolve().parent.parent
spec = importlib.util.spec_from_file_location('e2e_base', ROOT/'scripts/12_test_e2e.py')
base = importlib.util.module_from_spec(spec)
spec.loader.exec_module(base)


class HeadlessCase(base.DemoCase):
    async def request(self, op, **fields):
        async with self.control_lock:
            self.driver.stdin.write((json.dumps(dict(op=op, **fields))+'\n').encode())
            await self.driver.stdin.drain()
            line = await self.driver.stdout.readline()
            if not line:
                raise AssertionError('native terminal driver closed its control pipe')
            return json.loads(line)

    async def open_terminal(self, size):
        self.control_lock = asyncio.Lock()
        self.size = size
        self.driver_log = (self.d/'native-terminal.stderr').open('wb')
        self.driver = await asyncio.create_subprocess_exec(
            str(ROOT/'target/release/examples/1_native_terminal'), str(size[0]), str(size[1]),
            str(self.d/'terminal.ansi.gz'), '/bin/bash', '--noprofile', '--norc', '-i',
            cwd=str(ROOT), stdin=asyncio.subprocess.PIPE, stdout=asyncio.subprocess.PIPE,
            stderr=self.driver_log, limit=8*1024*1024)
        ready = json.loads(await asyncio.wait_for(self.driver.stdout.readline(), 2))
        assert ready['ready'] and (ready['cols'],ready['rows']) == size, ready
        self.session = SimpleNamespace(async_send_text=self.send_text, async_set_grid_size=self.resize)
        self.window = SimpleNamespace(async_close=self.close_terminal)
        self.result['scope'] = 'actual demo and portable-pty/vt100; no Python frame parsing or GUI painting'
        base.support.write_json(self.d/'terminal.json', {
            'requested':list(size), 'actual':list(size), 'transport':'portable-pty',
            'screen':'vt100', 'focus_required':False, 'driver_pid':self.driver.pid})
        return self.driver.pid

    async def heartbeat(self):
        try:
            while True:
                status = await self.rpc('native-status', self.request('status'))
                state = {'ts_ms':time.time_ns()//1_000_000, 'mode':'headless',
                         'transport_alive':status['alive'] and not status['reader_done']}
                if status['error']:
                    state['error'] = status['error']
                base.support.write_json(self.d/'observer.json', state)
                base.support.write_json(self.d/'native-terminal-stats.json', status)
                if not state['transport_alive'] or status['error']:
                    self.observer_error = 'native terminal transport failed: '+str(status)
                    return
                await asyncio.sleep(.1)
        except asyncio.CancelledError:
            raise
        except Exception as error:
            self.observer_error = f'native terminal observer: {error}'

    async def send_text(self, text, suppress_broadcast=True):
        await self.request('send', text=text)

    async def resize(self, size):
        await self.request('resize', cols=size.width, rows=size.height)
        self.size = (size.width,size.height)

    async def screen(self):
        self.healthy()
        return (await self.rpc('native-screen',self.request('screen')))['text']

    async def capture(self, name):
        self.healthy()
        snapshot = await self.rpc('native-capture',self.request('capture',
            width=max(1,self.size[0]-34),height=self.size[1]-1))
        assert any(row.strip() for row in snapshot['text']), 'art cells are blank'
        encoded = json.dumps(snapshot['rows'],ensure_ascii=False).encode()
        assert any(re.search(r'\x1b\[[0-9;]*(?:38|48);[25];', row)
                   for row in snapshot['rows']), 'native art capture contains no indexed/RGB colors'
        (self.d/f'{name}.cells.json').write_bytes(encoded)
        return hashlib.sha256(encoded).hexdigest()

    async def close_terminal(self, force=True):
        if hasattr(self,'driver') and self.driver.returncode is None:
            try:
                self.driver.stdin.write(b'{"op":"quit"}\n')
                await self.driver.stdin.drain()
                await asyncio.wait_for(self.driver.wait(),1)
            except (TimeoutError, BrokenPipeError, ConnectionResetError):
                if self.driver.returncode is None:
                    self.driver.kill()
                    await self.driver.wait()
        if hasattr(self,'driver_log'):
            self.driver_log.close()


async def run(args):
    names = ['workflow', 'bad-400x200', 'max-400x200', 'backpressure'] if args.case == 'all' else [args.case]
    report = {'status': 'running', 'complete_suite': args.case == 'all',
              'scope': 'native headless real demo/PTY and terminal-cell assertions; GUI painting untested',
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
