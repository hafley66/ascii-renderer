#!/usr/bin/env python3
"""Run the same live-demo E2E case in an isolated WezTerm process using its CLI."""
import argparse
import asyncio
import importlib.util
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import time
from types import SimpleNamespace

ROOT=Path(__file__).resolve().parent.parent
spec=importlib.util.spec_from_file_location('e2e',ROOT/'scripts/12_test_e2e.py')
e2e=importlib.util.module_from_spec(spec)
spec.loader.exec_module(e2e)


class WezCase(e2e.DemoCase):
    async def cli(self,*args,text=None):
        process=await asyncio.create_subprocess_exec(str(self.args.wezterm),'cli','--no-auto-start',*args,
            env=dict(os.environ,WEZTERM_UNIX_SOCKET=self.endpoint['socket']),
            stdin=asyncio.subprocess.PIPE,stdout=asyncio.subprocess.PIPE,stderr=asyncio.subprocess.PIPE)
        try:
            out,err=await asyncio.wait_for(process.communicate(None if text is None else text.encode()),.7)
            if process.returncode:
                raise AssertionError(f'wezterm CLI {args[0]} failed: {err.decode()}')
            return out.decode()
        finally:
            if process.returncode is None:
                process.kill()
                await process.wait()

    async def open_terminal(self,size):
        import Quartz
        from AppKit import NSRunningApplication, NSApplicationActivateIgnoringOtherApps
        # The child records only the dedicated socket and pane, then starts an idle shell.
        config=self.d/'wezterm.lua'
        config.write_text('return { initial_cols='+str(size[0])+', initial_rows='+str(size[1])+',\n'
            'font_size=2.0, enable_tab_bar=false, check_for_updates=false,\n'
            'window_padding={left=0,right=0,top=0,bottom=0},\n'
            'scrollback_lines=0, exit_behavior="Close", automatically_reload_config=false }\n')
        log=(self.d/'wezterm.log').open('w')
        self.gui=subprocess.Popen(['/usr/bin/open','-W','-n',str(self.args.wezterm.parents[2]),'--args','--config-file',str(config),'start',
            '--always-new-process','--no-auto-connect','--cwd',str(ROOT),'--',sys.executable,
            str(Path(__file__).resolve()),'--boot',str(self.d)],stdout=log,stderr=log,start_new_session=True)
        log.close()
        deadline=time.monotonic()+5
        while not (self.d/'endpoint.json').exists():
            if self.gui.poll() is not None:
                raise AssertionError('WezTerm exited during startup; see wezterm.log')
            if time.monotonic()>deadline:
                raise AssertionError('WezTerm startup exceeded five seconds')
            await asyncio.sleep(.05)
        self.endpoint=json.loads((self.d/'endpoint.json').read_text())
        self.gui_pid=self.endpoint['parent_pid']
        application=NSRunningApplication.runningApplicationWithProcessIdentifier_(self.gui_pid)
        if application is None:
            raise AssertionError(f'WezTerm GUI process {self.gui_pid} is unavailable')
        application.activateWithOptions_(NSApplicationActivateIgnoringOtherApps | 1)
        self.window=SimpleNamespace(window_id=str(self.gui_pid),async_close=self.close_window)
        self.session=SimpleNamespace(async_send_text=self.send_text)
        self.app=SimpleNamespace(async_refresh_focus=self.refresh_focus,
            app_active=False,current_window=self.window,current_terminal_window_id=str(self.gui_pid))
        deadline=time.monotonic()+2
        while time.monotonic()<deadline:
            await self.refresh_focus()
            if self.app.app_active:
                break
            await asyncio.sleep(.05)
        else:
            from AppKit import NSWorkspace
            front=NSWorkspace.sharedWorkspace().frontmostApplication()
            raise AssertionError(f'WezTerm did not receive focus; foreground={front.localizedName()}, pid={front.processIdentifier()}, expected={self.gui_pid}')
        windows=[w for w in Quartz.CGWindowListCopyWindowInfo(Quartz.kCGWindowListOptionOnScreenOnly,0)
                 if w.get('kCGWindowOwnerPID')==self.gui_pid and w.get('kCGWindowLayer')==0]
        assert len(windows)==1, 'expected exactly one owned WezTerm window'
        self.capture_id=int(windows[0]['kCGWindowNumber'])
        panes=json.loads(await self.cli('list','--format','json'))
        pane=next(p for p in panes if p['pane_id']==int(self.endpoint['pane']))
        actual=(pane['size']['cols'],pane['size']['rows'])
        e2e.support.write_json(self.d/'terminal.json',dict(requested=list(size),actual=list(actual),
            capture_id=self.capture_id,pane=pane,version=subprocess.check_output([str(self.args.wezterm),'--version'],text=True).strip()))
        assert actual==size, f'WezTerm size differs: {actual} versus {size}'
        return self.gui_pid

    async def refresh_focus(self):
        from AppKit import NSWorkspace
        from Foundation import NSRunLoop, NSDate
        NSRunLoop.currentRunLoop().runUntilDate_(NSDate.dateWithTimeIntervalSinceNow_(.001))
        front=NSWorkspace.sharedWorkspace().frontmostApplication()
        self.app.app_active=bool(front and front.processIdentifier()==self.gui_pid)

    async def send_text(self,text,suppress_broadcast=True):
        await self.cli('send-text','--pane-id',str(self.endpoint['pane']),'--no-paste',text=text)

    async def screen(self):
        self.healthy()
        return await self.rpc('wezterm:get-text',self.cli('get-text','--pane-id',str(self.endpoint['pane'])))

    async def close_window(self,force=True):
        if hasattr(self,'endpoint'):
            try:
                await self.cli('kill-pane','--pane-id',str(self.endpoint['pane']))
            except Exception:
                pass
        if hasattr(self,'gui') and self.gui.poll() is None:
            try: os.killpg(self.gui.pid,signal.SIGTERM)
            except ProcessLookupError: pass
            try: await asyncio.to_thread(self.gui.wait,timeout=2)
            except subprocess.TimeoutExpired:
                os.killpg(self.gui.pid,signal.SIGKILL)
                await asyncio.to_thread(self.gui.wait,timeout=2)


async def run(args):
    case=WezCase(None,None,args,args.case,args.directory,args.binary)
    try:
        await case.run()
    except Exception as error:
        import traceback
        case.result.update(status='failed',error=f'{type(error).__name__}: {error}')
        args.directory.mkdir(parents=True,exist_ok=True)
        (args.directory/'failure.txt').write_text(traceback.format_exc())
    finally:
        try:
            await case.cleanup()
        except Exception as error:
            case.result.update(status='failed',cleanup_error=str(error))
        await case.close_window()
    e2e.support.write_json(args.directory/'report.json',case.result)
    print(json.dumps({'status':case.result['status'],'error':case.result.get('error'),
                     'directory':str(args.directory),'steady':case.result.get('steady')}),flush=True)
    return int(case.result['status']!='passed')


if __name__=='__main__':
    if sys.argv[1:2]==['--boot']:
        e2e.support.write_json(Path(sys.argv[2])/'endpoint.json',{
            'socket':os.environ['WEZTERM_UNIX_SOCKET'],'pane':os.environ['WEZTERM_PANE'],
            'parent_pid':os.getppid()})
        os.execv('/bin/bash',['bash','--noprofile','--norc','-i'])
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--wezterm',type=Path,required=True)
    parser.add_argument('--binary',type=Path,default=ROOT/'target/release/ascii-renderer')
    parser.add_argument('--directory',type=Path,default=ROOT/f'perf/results/wezterm-{time.time_ns()//1000000}')
    parser.add_argument('--direct',action='store_true',help='enter native animation through its CLI; separately labelled from demo E2E')
    parser.add_argument('--case',choices=['bad-400x200','max-400x200'],default='bad-400x200')
    args=parser.parse_args()
    args.wezterm,args.binary,args.directory=args.wezterm.resolve(),args.binary.resolve(),args.directory.resolve()
    if args.directory.exists(): parser.error('use a fresh result directory')
    sys.exit(asyncio.run(run(args)))
