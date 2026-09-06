"""Summarize per-thread inclusive sample counts without recursive double counting."""
import collections
import json
import pathlib
import re
import statistics
import sys

directory = pathlib.Path(sys.argv[1])
summary = {'profiles': {}}
patterns = {
    'terminal_drawing': 'PTYTextView drawRect:inView:',
    'construct_and_draw_runs': 'constructAndDrawRunsForLine:',
    'build_attributed_strings': 'attributedStringsForLine:',
    'draw_attributed_strings': 'drawMultipartAttributedString:',
    'iterm_timer_instrumentation': 'iTermPreciseTimer',
    'append_string': 'terminalAppendString:',
    'worker_write_frame': '11write_frame',
    'sleep': 'functions5sleep',
}
for path in sorted(directory.glob('*.sample.txt')):
    threads = []
    stack = []
    for line in path.read_text().split('Total number in stack')[0].splitlines():
        match = re.match(r'([ +!:|]*)(\d+) (.+)', line)
        if not match:
            continue
        prefix, count, name = match.groups()
        if 'Thread_' in name:
            thread = {'thread': name, 'samples': int(count), 'nodes': []}
            threads.append(thread)
            stack = []
            continue
        if not threads:
            continue
        depth = len(prefix)
        while stack and stack[-1]['depth'] >= depth:
            stack.pop()
        node = {'depth': depth, 'count': int(count), 'children': 0,
                'name': name.split('  (in ')[0], 'path': [n['name'] for n in stack]}
        node['path'].append(node['name'])
        if stack:
            stack[-1]['children'] += node['count']
        stack.append(node)
        thread['nodes'].append(node)
    for thread in threads:
        nodes = thread.pop('nodes')
        assert sum(n['count'] - n['children'] for n in nodes) == thread['samples']
        assert all(n['count'] >= n['children'] for n in nodes)
        thread['inclusive'] = {
            label: sum(n['count'] - n['children'] for n in nodes
                       if any(pattern in ancestor for ancestor in n['path']))
            for label, pattern in patterns.items()
        }
        exclusive = collections.Counter()
        for node in nodes:
            exclusive[node['name']] += node['count'] - node['children']
        thread['top_exclusive'] = exclusive.most_common(10)
    summary['profiles'][path.name] = threads

rows = [json.loads(line) for line in (directory / 'animation.ndjson').read_text().splitlines()]
frames = [row for row in rows if 'frame_index' in row]
summary['frames'] = {'count': len(frames)}
for key in ('render_us', 'encoding_us', 'presentation_us', 'dur_us', 'bytes'):
    values = [frame[key] for frame in frames]
    summary['frames'][key] = {'median': statistics.median(values), 'min': min(values),
                              'max': max(values), 'sum': sum(values)}
ui = [json.loads(line) for line in (directory / 'ui-events.ndjson').read_text().splitlines()]
requested = next(r['ts_ms'] for r in ui if r['kind'] == 'key_q_requested')
returned = next(r['ts_ms'] for r in ui if r['kind'] == 'key_q_returned')
received = next(r for r in rows if r.get('stage') == 'input_received')
stopped = next(r for r in rows if r.get('stage') == 'worker_stopped'
               and r['worker_pid'] == received['worker_pid'])
summary['quit'] = {'requested_ms': requested, 'call_returned_ms': returned,
                   'received_ms': received['ts_ms'], 'stopped_ms': stopped['ts_ms'],
                   'request_to_receive_ms': received['ts_ms'] - requested,
                   'call_return_to_receive_ms': received['ts_ms'] - returned,
                   'receive_to_stopped_ms': stopped['ts_ms'] - received['ts_ms'],
                   'stop_us': stopped['detail']['stop_us']}
(directory / 'summary.json').write_text(json.dumps(summary, indent=2) + '\n')
print(json.dumps({'frames': summary['frames'], 'quit': summary['quit']}, indent=2))
