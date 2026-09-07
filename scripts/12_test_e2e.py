#!/usr/bin/env python3
"""Actual demo E2E in a dedicated iTerm2 window. Run scripts/13_e2e.sh.

No saved-frame playback or PTY drain stands in for the GUI cases. The observer
uses iTerm2's official API. PNG captures verify pixel changes; frame logs report
application cadence, not display presentation timestamps. Each case is guarded.
"""
import argparse
import asyncio
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shlex
import shutil
import subprocess
import sys
import termios
import time

ROOT = Path(__file__).resolve().parent.parent
spec = importlib.util.spec_from_file_location('support', ROOT/'scripts/11_e2e_support.py')
support = importlib.util.module_from_spec(spec)
spec.loader.exec_module(support)


def app_child(directory, binary):
    """Observe app cleanup BEFORE the guard restores the terminal itself."""
    before = termios.tcgetattr(0)
    argv = ([binary,'42','morph','','gem-aetherium-2','42','gem-aetherium-2','43','iterate']
            if os.environ.get('ASCII_E2E_DIRECT')=='1' else [binary,'42','demo'])
    child = subprocess.Popen(argv)
    support.write_json(directory/'app.json', {'pid': child.pid, 'argv': argv})
    code = child.wait()
    after = termios.tcgetattr(0)
    support.write_json(directory/'app-exit.json', {
        'code': code, 'ts_ms': time.time_ns()//1_000_000,
        'termios_restored': before == after,
        'before': repr(before), 'after': repr(after)})
    return code


class DemoCase:
    def __init__(self, app, connection, args, name, directory, binary):
        self.app, self.connection, self.args = app, connection, args
        self.name, self.d, self.binary = name, directory, binary
        self.window = self.session = None
        self.observer = None
        self.trace = support.Trace(directory/'animation.ndjson')
        self.guard = support.Trace(directory/'guard.ndjson')
        self.inputs = []
        self.checks = []
        self.failures = []
        self.observer_error = None
        self.closed = False
        self.result = {'case': name, 'status': 'running', 'checks': self.checks}

    async def rpc(self, operation, request, timeout=.75):
        started = time.monotonic_ns()
        record = {'operation':operation, 'ts_ms':time.time_ns()//1_000_000, 'ok':False}
        try:
            value = await asyncio.wait_for(request,timeout)
            record['ok']=True
            return value
        except TimeoutError as error:
            record['error']='TimeoutError'
            raise AssertionError(f'terminal RPC {operation} exceeded {timeout*1000:.0f}ms') from error
        except BaseException as error:
            record['error']=type(error).__name__
            raise
        finally:
            record['elapsed_us']=(time.monotonic_ns()-started)//1000
            with (self.d/'rpc.ndjson').open('a') as file:
                file.write(json.dumps(record)+'\n')

    def checkpoint(self, name, **evidence):
        self.checks.append({'name': name, 'status': 'passed', **evidence})
        support.write_json(self.d/'result.json', self.result)

    def healthy(self):
        if self.observer_error:
            raise AssertionError(self.observer_error)
        for event in self.guard.read():
            if event['kind'] == 'breaker':
                raise AssertionError(f"watchdog: {event['reason']}")
        if any(e['kind'] == 'completed' for e in self.guard.rows) and not self.closed:
            raise AssertionError('application exited before the requested quit')

    async def heartbeat(self):
        try:
            while True:
                await self.rpc('refresh-focus',self.app.async_refresh_focus())
                focused = bool(self.app.app_active and self.app.current_window and
                               self.app.current_window.window_id == self.window.window_id)
                support.write_json(self.d/'observer.json', {
                    'ts_ms': time.time_ns()//1_000_000, 'focused': focused})
                if not focused:
                    raise AssertionError(f'dedicated window lost focus: app_active={self.app.app_active}, current={self.app.current_terminal_window_id}, owned={self.window.window_id}')
                await asyncio.sleep(.1)
        except asyncio.CancelledError:
            raise
        except Exception as error:
            from AppKit import NSWorkspace
            front = NSWorkspace.sharedWorkspace().frontmostApplication()
            self.observer_error = type(error).__name__+': '+str(error)+f'; foreground={front.localizedName()}'
            support.write_json(self.d/'observer.json', {
                'ts_ms': time.time_ns()//1_000_000, 'focused': False, 'error': self.observer_error})

    async def wait(self, name, predicate, timeout=2):
        deadline = time.monotonic()+timeout
        while time.monotonic() < deadline:
            self.healthy()
            if predicate(self.trace.read()):
                return
            await asyncio.sleep(.01)
        raise AssertionError(f'{name}: no matching application evidence within {timeout}s')

    async def screen(self):
        self.healthy()
        contents = await self.rpc('read-screen',self.session.async_get_screen_contents())
        return '\n'.join(contents.line(i).string for i in range(contents.number_of_lines))

    async def screen_until(self, name, predicate, timeout=2):
        deadline = time.monotonic()+timeout
        while time.monotonic()<deadline:
            text = await self.screen()
            if predicate(text):
                (self.d/f'{name}.txt').write_text(text)
                return text
            await asyncio.sleep(.02)
        (self.d/f'{name}-failed.txt').write_text(text)
        raise AssertionError(f'{name}: screen assertion timed out after {timeout}s; last row={text.splitlines()[-1:]!r}')

    async def key(self, name, text):
        self.healthy()
        event = {'name': name, 'text': text, 'ts_ms': time.time_ns()//1_000_000,
                 'monotonic_ns': time.monotonic_ns()}
        self.inputs.append(event)
        support.write_json(self.d/'sent-inputs.json', self.inputs)
        await self.rpc('key:'+name,self.session.async_send_text(text,suppress_broadcast=True))
        return event

    async def capture(self, name):
        """Capture only the owned window. Decode pixels so PNG metadata cannot fake motion."""
        from PIL import Image, ImageStat
        path = self.d/f'{name}.png'
        process = await asyncio.create_subprocess_exec('/usr/sbin/screencapture', '-x', '-o',
            '-l', str(self.capture_id), str(path), stderr=asyncio.subprocess.PIPE)
        _, error = await asyncio.wait_for(process.communicate(), 1)
        if process.returncode:
            raise AssertionError(f'owned-window screenshot failed: {error.decode()}')
        def pixels():
            with Image.open(path) as image:
                # Exclude window chrome, options panel, and status row.
                w,h = image.size
                region = image.crop((0, min(80,h//4), max(1,w*3//4), max(81,h-60))).convert('RGB')
                if max(ImageStat.Stat(region).stddev) < 2:
                    raise AssertionError('captured art region is blank')
                return hashlib.sha256(region.tobytes()).hexdigest()
        return await asyncio.to_thread(pixels)

    def frames(self):
        return [r for r in self.trace.read() if 'frame_index' in r]

    async def open_terminal(self, size):
        import iterm2
        import Quartz
        watch_pid = int(subprocess.check_output(['pgrep','-x','iTerm2'], text=True).strip())
        def window_ids():
            return {int(w['kCGWindowNumber']) for w in Quartz.CGWindowListCopyWindowInfo(
                Quartz.kCGWindowListOptionOnScreenOnly, 0)
                if w.get('kCGWindowOwnerPID') == watch_pid and w.get('kCGWindowLayer') == 0}
        existing_windows = window_ids()
        profile = iterm2.LocalWriteOnlyProfile()
        # Session-local font; no edits to the user's saved profile.
        profile.set_normal_font('Menlo-Regular 2')
        self.window = await iterm2.Window.async_create(self.connection,
            command='/bin/bash --noprofile --norc -i', profile_customizations=profile)
        if self.window is None:
            raise AssertionError('iTerm did not create the dedicated window')
        self.session = self.window.current_tab.current_session
        await self.app.async_activate(raise_all_windows=False, ignoring_other_apps=True)
        await self.window.async_activate()
        await self.app.async_refresh()
        created = window_ids() - existing_windows
        assert len(created)==1, f'cannot uniquely identify owned screenshot window: {created}'
        self.capture_id = created.pop()
        session_profile = await self.session.async_get_profile()
        await session_profile.async_set_normal_font('Menlo-Regular 2')
        await self.session.async_set_grid_size(iterm2.Size(*size))
        await self.app.async_refresh()
        actual = self.app.get_session_by_id(self.session.session_id).grid_size
        support.write_json(self.d/'terminal.json', {'requested':list(size), 'actual':[actual.width,actual.height], 'capture_id': self.capture_id})
        assert (actual.width,actual.height)==size, f'iTerm size clamped: requested={size}, actual={actual}'
        deadline = time.monotonic()+2
        while time.monotonic()<deadline:
            await self.app.async_refresh_focus()
            if self.app.app_active and self.app.current_window and self.app.current_window.window_id == self.window.window_id:
                break
            await asyncio.sleep(.05)
        else:
            raise AssertionError(f'cannot focus owned window: active={self.app.app_active}, current={self.app.current_terminal_window_id}, owned={self.window.window_id}')
        return watch_pid

    async def run(self):
        from PIL import Image, ImageStat
        Image.init()  # Import decoders before the live observer starts.
        self.d.mkdir(parents=True)
        self.result.update(binary=support.binary_identity(self.binary, ROOT))
        fixture = json.loads((ROOT/'perf/fixtures/12_gem_aetherium_2_bad_roll6.json').read_text())
        if self.name == 'max-400x200':
            fixture = json.loads(subprocess.check_output([str(self.binary), 'inputs', 'gem-aetherium-2', 'max'], timeout=2))
        config = self.d/'config/ascii-renderer'
        config.mkdir(parents=True)
        (config/'options.tsv').write_text('__global\tRAND\t0\n'+''.join(
            f'gem-aetherium-2\t{k}\t{v}\n' for k,v in fixture['knobs'].items()))
        support.write_json(self.d/'fixture.json', fixture)
        size = (160,40) if self.name == 'workflow' else (400,200)
        watch_pid = await self.open_terminal(size)
        support.write_json(self.d/'observer.json', {'ts_ms': time.time_ns()//1_000_000, 'focused': True})
        self.observer = asyncio.create_task(self.heartbeat())
        await asyncio.sleep(.15)
        self.healthy()
        child = [sys.executable, str(Path(__file__).resolve()), '--app-child', str(self.d), str(self.binary)]
        child = [shutil.which('samply'), 'record', '--rate', '1000', '--duration', '15',
                 '--save-only', '--output', str(self.d/'profile.json.gz'), '--', *child]
        direct = getattr(self.args,'direct',False)
        self.result['entry']='native-cli' if direct else 'demo'
        command = ['env', f'ASCII_E2E_DIRECT={int(direct)}', f'XDG_CONFIG_HOME={self.d}/config',
            f'ASCII_TRACE_PATH={self.d}/animation.ndjson', 'ASCII_TRACE_ALL=1',
            sys.executable, str(ROOT/'scripts/5_probe_guard.py'), '--state', str(self.d/'guard.ndjson'),
            '--artifact-dir', str(self.d), '--max-growth-mib', '256',
            '--owner-pid', str(os.getpid()), '--observer-state', str(self.d/'observer.json')]
        if watch_pid is not None:
            command += ['--watch-pid', str(watch_pid)]
        if getattr(self.args, 'function_trace', False):
            command.insert(1, f'ASCII_FUNCTION_TRACE={self.d}/functions')
            self.result['function_trace_directory'] = str(self.d/'functions')
        command += ['--save-profiler-on-stop']
        command += ['--', *child]
        support.write_json(self.d/'command.json', command)
        shell = shlex.join(command)+' 2>'+shlex.quote(str(self.d/'stderr.log'))+'; printf "E2E_COMMAND_EXIT:%s\\n" "$?"\r'
        await self.session.async_send_text('\x15'+shell, suppress_broadcast=True)
        if not direct:
            await self.screen_until('demo-start', lambda s: 'a=animate' in s and 'seed:42' in s, 3)
            self.checkpoint('actual-demo-startup', terminal=list(size))
            await self.key('open-mode-picker','/')
            await self.screen_until('mode-picker', lambda s: 'cancel' in s.lower())
            await self.key('find-gem','gem-aetherium-2')
            await self.screen_until('mode-filter', lambda s: 'gem-aetherium-2' in s)
            await self.key('select-gem','\r')
            await self.screen_until('gem-preview', lambda s: 'gem-aetherium-2' in s and 'a=animate' in s)
            self.checkpoint('mode-search-and-preview')
            if self.name == 'workflow':
                await self.key('save-exact-preset','s')
                await self.screen_until('saved-preset', lambda s: 'saved:gem-aetherium-2' in s)
                presets = json.loads((config/'presets.json').read_text())
                support.write_json(self.d/'saved-presets.json', presets)
                saved = presets['presets']
                assert len(saved)==1 and saved[0]['mode']=='gem-aetherium-2' and saved[0]['seed']==42, saved
                assert all(abs(saved[0]['knobs'][k]-v)<1e-5 for k,v in fixture['knobs'].items()), saved
                self.checkpoint('modeless-save', file=str(config/'presets.json'))
            await self.key('animate','a')
        await self.wait('first native frame', lambda _: bool(self.frames()), 3)
        first = self.frames()[0]
        assert first['seed']==42 and first['mode']=='gem-aetherium-2' and first['strategy']=='iterate', first
        assert first['terminal_size']==dict(w=size[0],h=size[1]), first
        assert first['grid']==dict(w=size[0]-34,h=size[1]-1), first
        assert all(abs(first['knobs'][k]-v)<1e-5 for k,v in fixture['knobs'].items()), first
        self.checkpoint('exact-animation-inputs', frame=first)
        await self.screen_until('animation-ui', lambda s: 't=' in s and 'gem-aetherium-2' in s)
        pixels_before = await self.capture('animation-before')
        start = len(self.frames())
        await self.wait('animation progresses', lambda _: len(self.frames())>=start+3)
        pixels_after = await self.capture('animation-after')
        assert pixels_before != pixels_after, 'art pixels did not change during animation'
        self.checkpoint('terminal-cell-motion' if getattr(self.args, 'headless', False) else 'visible-art-motion', before=pixels_before, after=pixels_after)
        if self.name == 'workflow':
            await self.workflow()
        else:
            # Validate steady-state cadence before changing the recorded configuration.
            await self.wait('ten recorded frames', lambda _: len(self.frames())>=10)
            steady = self.frames()[1:10]
            self.result['steady'] = support.frame_summary(steady)
            await self.knob_and_roll()
        if direct:
            self.closed=True
            sent=await self.key('exit-native-cli','q')
        else:
            await self.key('return-to-demo','q')
            await self.screen_until('returned-demo', lambda s: 'a=animate' in s and 'gem-aetherium-2' in s)
            self.checkpoint('q-returns-to-demo')
            self.closed = True
            sent = await self.key('exit-demo','\x03' if self.name != 'workflow' else 'q')
        await self.wait('app exit', lambda _: (self.d/'app-exit.json').exists(), 2)
        exit_info = json.loads((self.d/'app-exit.json').read_text())
        assert exit_info['code']==0 and exit_info['termios_restored'], exit_info
        latency = exit_info['ts_ms']-sent['ts_ms']
        assert latency <= 1000, f'app exit took {latency}ms'
        await self.wait('guard completion', lambda _: any(r['kind']=='completed' for r in self.guard.read()), 3)
        assert self.guard.rows[-1]['code']==0, self.guard.rows[-1]
        await self.screen_until('shell-restored', lambda s: 'E2E_COMMAND_EXIT:0' in s)
        self.checkpoint('exit-and-terminal-restoration', latency_ms=latency)
        summary = support.profile_summary(self.d/'profile.json.gz')
        assert summary['samples']>0, 'samply produced no samples'
        worker = str(first['pid'])
        assert any(str(t['pid'])==worker and t['samples']>0 for t in summary['threads']), 'animation worker absent from profile'
        support.write_json(self.d/'profile-summary.json', summary)
        self.checkpoint('profile-includes-render-worker', **summary)
        if self.name != 'workflow':
            self.result['budget'] = {'p95_frame_interval_ms': 33.34, 'max_frame_interval_ms': 100}
            measured = self.result['steady']['interval_ms']
            assert measured['p95'] <= 33.34 and measured['max'] <= 100, f'animation cadence budget failed: {measured}'
            self.checkpoint('animation-performance-budget', **measured)
        self.result['status'] = 'failed' if self.failures else 'passed'
        if self.failures:
            self.result['error']='; '.join(self.failures)

    async def knob_and_roll(self):
        before = self.frames()[-1]
        sent = await self.key('decrease-selected-knob','-')
        await self.wait('knob reaches frame', lambda _: self.frames()[-1]['knobs'] != before['knobs'])
        frame = self.frames()[-1]
        latency = frame['ts_ms']-sent['ts_ms']
        self.checkpoint('knob-reaches-render', latency_ms=latency, knobs=frame['knobs'])
        if latency > 250:
            self.checks[-1]['status']='failed'
            self.failures.append(f'knob-to-frame exceeded 250ms: {latency}')
        await self.key('enable-random-knobs','g')
        await self.wait('random enabled', lambda _: self.frames()[-1]['randomize'])
        for roll in (1,2):
            previous = self.frames()[-1]
            await self.key(f'random-roll-{roll}','+')
            await self.wait('random jump', lambda _: self.frames()[-1]['roll'] != previous['roll'])
            assert self.frames()[-1]['knobs'] != previous['knobs'], 'roll did not change knobs'
        self.checkpoint('random-knob-diversity', frame=self.frames()[-1])

    async def workflow(self):
        import iterm2
        await self.knob_and_roll()
        await self.key('pause',' ')
        await self.screen_until('paused', lambda s: 't=' in s)
        # In-flight output may finish after pause. Require eventual stable frame time.
        await asyncio.sleep(.1)
        paused = self.frames()[-1]['time']
        await asyncio.sleep(.15)
        assert self.frames()[-1]['time']==paused, 'animation time advanced while paused'
        self.checkpoint('pause-stops-animation-time', time=paused)
        await self.key('resume',' ')
        await self.wait('resume', lambda _: self.frames()[-1]['time']>paused)
        await self.session.async_set_grid_size(iterm2.Size(180,45))
        await self.wait('resize input', lambda _: self.frames()[-1]['terminal_size']==dict(w=180,h=45))
        assert self.frames()[-1]['grid']==dict(w=146,h=44), self.frames()[-1]
        try:
            await self.screen_until('resized-ui', lambda s: 'term=180x45 grid=146x44' in s, .5)
            self.checkpoint('resize-reaches-grid-and-footer', frame=self.frames()[-1])
        except AssertionError as error:
            self.failures.append(str(error))
            self.checks.append({'name':'resize-reaches-grid-and-footer','status':'failed','error':str(error)})

    async def cleanup(self):
        # Fail the heartbeat first, allowing the independent guard to stop its tree.
        if self.observer:
            self.observer.cancel()
            await asyncio.gather(self.observer, return_exceptions=True)
        support.write_json(self.d/'observer.json', {'ts_ms': time.time_ns()//1_000_000, 'focused': False, 'stop_reason':'cleanup'})
        for _ in range(25):
            rows = self.guard.read()
            if not rows or any(r['kind'] in ('completed','stopped') for r in rows):
                break
            await asyncio.sleep(.1)
        if self.window:
            await asyncio.wait_for(self.window.async_close(force=True), 2)
        if (self.d/'profile.json.gz').exists():
            try:
                self.result['profile'] = support.profile_summary(self.d/'profile.json.gz')
                support.write_json(self.d/'profile-summary.json', self.result['profile'])
            except Exception as error:
                self.result['profile_error'] = str(error)
        self.result['frames'] = support.frame_summary(self.frames())
        self.result['guard'] = self.guard.read()
        support.write_json(self.d/'result.json', self.result)


def backpressure_case(args):
    directory = args.directory/'backpressure'
    directory.mkdir()
    command = [sys.executable, str(ROOT/'scripts/5_probe_guard.py'),
        '--state', str(directory/'guard.ndjson'), '--artifact-dir', str(directory),
        '--owner-pid', str(os.getpid()), '--', sys.executable,
        str(ROOT/'scripts/4_test_input_latency.py'), str(args.binary),
        '--size', '366x199', '--stall', '1', '--sampled', '--max-ms', '250',
        '--directory', str(directory/'inputs')]
    if args.function_trace:
        command = ['env', f'ASCII_FUNCTION_TRACE={directory}/functions', *command]
    support.write_json(directory/'command.json', command)
    with (directory/'stdout.log').open('w') as out, (directory/'stderr.log').open('w') as err:
        process = subprocess.run(command, stdout=out, stderr=err, timeout=20)
    guards = support.Trace(directory/'guard.ndjson').read()
    result = {'case':'backpressure', 'status':'failed', 'code':process.returncode,
              'scope':'real app/PTY with stalled consumer; no emulator painting', 'guard':guards}
    if process.returncode==0 and guards[-1].get('kind')=='completed' and guards[-1].get('code')==0:
        result['status']='passed'
    support.write_json(directory/'result.json', result)
    return result


async def suite(connection, args):
    import iterm2
    app = await iterm2.async_get_app(connection)
    names = ['backpressure','workflow','bad-400x200','max-400x200'] if args.case == 'all' else [args.case]
    if args.case == 'all' and args.function_trace:
        names = ['workflow','bad-400x200','max-400x200','backpressure']
    report = {'status':'running','cases':[{'case':name,'status':'not_run'} for name in names],
              'scope':'actual iTerm2 demo; screenshots prove motion, no paint-FPS claim',
              'complete_suite':args.case=='all'}
    support.write_json(args.directory/'report.json', report)
    for index,name in enumerate(names):
        if name=='backpressure':
            result = await asyncio.to_thread(backpressure_case,args)
            report['cases'][index]=result
            if any(r['kind']=='breaker' for r in result['guard']):
                break
            continue
        case = DemoCase(app,connection,args,name,args.directory/name,args.binary)
        try:
            await case.run()
        except Exception as error:
            case.result.update(status='failed', error=f'{type(error).__name__}: {error}')
            import traceback
            (case.d/'failure.txt').write_text(traceback.format_exc())
        finally:
            try:
                await case.cleanup()
            except Exception as error:
                case.result.update(status='failed', cleanup_error=str(error))
        report['cases'][index] = case.result
        print(json.dumps({'case':name,'status':case.result['status'],'error':case.result.get('error')}),flush=True)
        if any(r['kind']=='breaker' for r in case.guard.rows):
            break  # A safety trip prevents subsequent live workloads.
    report['status'] = 'passed' if all(c['status']=='passed' for c in report['cases']) else 'failed'
    support.write_json(args.directory/'report.json', report)


def main():
    if sys.argv[1:2] == ['--app-child']:
        return app_child(Path(sys.argv[2]), sys.argv[3])
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, default=ROOT/'target/release/ascii-renderer')
    parser.add_argument('--directory', type=Path, default=ROOT/f'perf/results/e2e-{time.time_ns()//1_000_000}')
    parser.add_argument('--case', choices=['all','backpressure','workflow','bad-400x200','max-400x200'], default='all')
    parser.add_argument('--function-trace', action='store_true', help='Record every instrumented function into each guarded case directory; run GUI cases first')
    parser.add_argument('--headless', action='store_true', help='Real demo/PTY with Pexpect and pyte; no GUI focus or terminal paint measurements')
    args = parser.parse_args()
    args.binary, args.directory = args.binary.resolve(), args.directory.resolve()
    args.directory.mkdir(parents=True, exist_ok=False)
    support.write_json(args.directory/'report.json', {'status':'failed','stage':'preflight','error':'suite has not connected'})
    if not args.binary.is_file() or not shutil.which('samply'):
        raise SystemExit('Build the release binary and install samply before running E2E.')
    if args.case=='backpressure':
        result=backpressure_case(args)
        support.write_json(args.directory/'report.json', {'status':result['status'],'cases':[result], 'complete_suite':False})
        return 0 if result['status']=='passed' else 1
    print(f'E2E artifacts: {args.directory}', flush=True)
    if args.headless:
        spec = importlib.util.spec_from_file_location('headless', ROOT/'scripts/16_headless_e2e.py')
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        asyncio.run(module.run(args))
    else:
        import iterm2
        iterm2.run_until_complete(lambda connection: suite(connection,args))
    return 0 if json.loads((args.directory/'report.json').read_text())['status']=='passed' else 1


if __name__ == '__main__':
    sys.exit(main())
