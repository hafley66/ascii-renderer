"""Derive stable-animation I/O rates and RSS trends from the dedicated probe."""
import collections
import json
import pathlib
import statistics
import sys

directory = pathlib.Path(sys.argv[1])
def rows(name):
    return [json.loads(s) for s in (directory / name).read_text().splitlines()]

trace = rows('animation.ndjson')
frames = [r for r in trace if 'frame_index' in r]
relays = [r for r in trace if r.get('kind') == 'playback_relay' and r.get('animation')]
worker, supervisor = frames[0]['pid'], relays[0]['pid']
start, end = frames[0]['ts_ms'] + 3000, frames[-1]['ts_ms'] - 2000
summary = {'worker': worker, 'supervisor': supervisor, 'io': [], 'memory': {}}
for pid in [worker, supervisor]:
    groups = collections.defaultdict(list)
    for row in rows(f'io-{pid}.ndjson'):
        if start <= row['ts_ms'] <= end:
            groups[(row['op'], row['fd'])].append(row)
    for (op, fd), series in groups.items():
        if len(series) < 2:
            continue
        a, b = series[0], series[-1]
        dt = (b['ts_ms'] - a['ts_ms']) / 1000
        delta = {k: b[k] - a[k] for k in
                 ['calls', 'requested', 'bytes', 'eagain', 'errors', 'partial', 'call_ns']}
        if not delta['calls']:
            continue
        successes = delta['calls'] - delta['eagain'] - delta['errors']
        summary['io'].append(dict(pid=pid, op=op, fd=fd, fd_kind_now=b['fd_kind_now'],
            from_ms=a['ts_ms'], to_ms=b['ts_ms'], seconds=dt, **delta,
            calls_per_second=delta['calls']/dt, eagain_fraction=delta['eagain']/delta['calls'],
            bytes_per_second=delta['bytes']/dt,
            bytes_per_success=delta['bytes']/successes if successes else None))
by_pid = collections.defaultdict(list)
for row in rows('memory.ndjson'):
    by_pid[row['pid']].append(row)
for pid, series in by_pid.items():
    values = [r['rss_kib'] for r in series]
    summary['memory'][pid] = dict(first_kib=values[0], min_kib=min(values), max_kib=max(values),
        last_kib=values[-1], samples=len(values),
        timeline=[{'seconds': round((r['ts_ms']-series[0]['ts_ms'])/1000, 3),
                   'rss_kib': r['rss_kib']} for r in series[::10]])
summary['frames'] = {'count': len(frames)}
for key in ['render_us', 'encoding_us', 'presentation_us', 'dur_us', 'bytes', 'runs']:
    values = [r[key] for r in frames]
    summary['frames'][key] = dict(median=statistics.median(values), max=max(values))
ui = rows('ui-events.ndjson')
q = next(r for r in ui if r['kind'] == 'key_q_requested')
returned = next(r for r in ui if r['kind'] == 'key_q_returned')
received = next(r for r in trace if r.get('stage') == 'input_received')
summary['quit'] = dict(request_to_receive_ms=received['ts_ms']-q['ts_ms'],
                       call_return_to_receive_ms=received['ts_ms']-returned['ts_ms'])
(directory / 'io-summary.json').write_text(json.dumps(summary, indent=2) + '\n')
print(json.dumps(summary, indent=2))
