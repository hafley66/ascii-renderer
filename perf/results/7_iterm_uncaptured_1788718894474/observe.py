import json,pathlib,subprocess,time,threading
import argparse
parser=argparse.ArgumentParser()
parser.add_argument("--window-id", type=int, required=True)
parser.add_argument("--output", type=pathlib.Path, required=True)
args=parser.parse_args()
T=pathlib.Path(__file__).resolve().parent
D=args.output.resolve()
D.mkdir(parents=True,exist_ok=False)
(D/'config/ascii-renderer').mkdir(parents=True)
fixture=json.loads((T/'inputs.json').read_text())
(D/'inputs.json').write_text(json.dumps(fixture,indent=2))
(D/'config/ascii-renderer/options.tsv').write_text('__global\tRAND\t0\n'+''.join(f'gem-aetherium-2\t{k}\t{v}\n' for k,v in fixture['knobs'].items()))
(D/'run.sh').write_text((T/'run.sh').read_text().replace(str(T),str(D)))
(D/'metadata.json').write_text((T/'metadata.json').read_text())
W=args.window_id
log=(D/'ui-events.ndjson').open('a',buffering=1)
def record(kind,**values):
 row=dict(kind=kind,ts_ms=time.time_ns()//1000000,**values);log.write(json.dumps(row)+'\n');print(json.dumps(row),flush=True)
def script(source,label,timeout=20):
 record(label+'_requested')
 r=subprocess.run(['osascript','-e',source],capture_output=True,text=True,timeout=timeout)
 record(label+'_returned',code=r.returncode,output=r.stdout.strip(),error=r.stderr.strip())
 if r.returncode:raise RuntimeError(r.stderr)
 return r.stdout.strip()
def key(s):
 script(f'''tell application id "com.googlecode.iterm2"
 if id of current window is not {W} then error "Repro window changed"
end tell
tell application "System Events" to tell process "iTerm2"
 if not frontmost then error "Repro window lost focus"
 keystroke {json.dumps(s)}
end tell''','key_'+s)
def screenshot(name):
 record('screenshot_requested',name=name)
 subprocess.run(['/usr/sbin/screencapture','-x',f'-l{W}',str(D/name)],check=True,timeout=10)
 record('screenshot_finished',name=name)
def read_trace():
    p=D/'animation.ndjson'
    records=[]
    if p.exists():
        for line in p.read_text().splitlines():
            try:records.append(json.loads(line))
            except json.JSONDecodeError:pass
    return records

def wait_for(predicate,label,seconds=15):
    end=time.monotonic()+seconds
    while time.monotonic()<end:
        if predicate(read_trace()):
            record(label);return
        time.sleep(.02)
    raise RuntimeError(label+' timed out')

command='/bin/bash '+str(D/'run.sh')
script(f'''tell application id "com.googlecode.iterm2"
 activate
 select window id {W}
 tell current session of window id {W} to write text {json.dumps(command)}
end tell''','launch')
wait_for(lambda rows: any(x.get('kind')=='playback_relay' for x in rows),'demo_started')
key('/')
key('gem-aetherium-2')
script('tell application "System Events" to tell process "iTerm2" to key code 36','search_enter')
wait_for(lambda rows: any(x.get('mode')=='gem-aetherium-2' and x.get('kind') in ('render','slow_render') for x in rows),'gem_preview')
record('capture_disabled')
key('a')
wait_for(lambda rows: any('frame_index' in x for x in rows),'animation_started')
time.sleep(18)
record('capture_disabled_before_key')
key('q')
wait_for(lambda rows: any(x.get('stage')=='session_exit' and x.get('detail',{}).get('result')=='Ok(Quit)' for x in rows),'animation_quit_received')
screenshot('after-worker-quit.png')
time.sleep(8)
screenshot('settled-demo.png')
key('q')
end=time.monotonic()+10
while time.monotonic()<end:
    p=D/'exit_ms.txt'
    if p.exists() and p.read_text().strip():break
    time.sleep(.02)
record('renderer_exit_observed',exit_ms=p.read_text().strip() if p.exists() else None)
screenshot('final.png')
