import pathlib,subprocess,json,time,os
root=pathlib.Path('/Users/chrishafley/projects/ascii-renderer');d=root/'perf/results/15_samply_animation';w=13802
log=(d/'ui-events.ndjson').open('a',buffering=1)
def script(s):
 r=subprocess.run(['osascript','-e',s],capture_output=True,text=True,timeout=3)
 log.write(json.dumps({'ts_ms':time.time_ns()//1000000,'code':r.returncode,'out':r.stdout,'err':r.stderr})+'\n')
 if r.returncode:raise RuntimeError(r.stderr)
def key(s):
 value = f'(ASCII character {ord(s)})' if len(s)==1 and ord(s)<32 else json.dumps(s)
 script(f'tell application id "com.googlecode.iterm2" to tell current session of window id {w} to write text {value} newline NO')
def rows():
 p=d/'animation.ndjson'
 if not p.exists():return []
 a=[]
 for l in p.read_text().splitlines():
  try:a.append(json.loads(l))
  except ValueError:pass
 return a
def wait(pred):
 until=time.monotonic()+5
 while time.monotonic()<until:
  if pred(rows()):return
  time.sleep(.03)
 raise RuntimeError('trace condition timed out')
cmd=f'cd {root} && XDG_CONFIG_HOME={d}/config ASCII_TRACE_PATH={d}/animation.ndjson ASCII_TRACE_ALL=1 /usr/bin/python3 scripts/5_probe_guard.py --state {d}/guard.ndjson --artifact-dir {d} --watch-pid 1510 --max-seconds 20 --max-growth-mib 256 --save-profiler-on-stop -- /Users/chrishafley/.cargo/bin/samply record --rate 1000 --duration 15 --save-only --output perf/results/ascii-functions.json.gz -- target/release/ascii-renderer 42 demo 2>{d}/samply.stderr'
key('\x15')
script(f'tell application id "com.googlecode.iterm2" to tell current session of window id {w} to write text {json.dumps(cmd)}')
wait(lambda rr:any(x.get('kind')=='playback_relay' for x in rr))
key('/')
time.sleep(.3)
key('gem-aetherium-2')
time.sleep(.3)
key('\r')
wait(lambda rr:any(x.get('mode')=='gem-aetherium-2' and x.get('kind') in ('render','slow_render') for x in rr))
time.sleep(.5)
key('a')
wait(lambda rr:any('frame_index' in x for x in rr))
print('animation observed',flush=True)

time.sleep(10)
if any(json.loads(line).get('kind') == 'breaker' for line in (d/'guard.ndjson').read_text().splitlines()):
 raise SystemExit('watchdog already stopped workload')
key('q')
time.sleep(.5)
key('q')
print('quit sent',flush=True)
