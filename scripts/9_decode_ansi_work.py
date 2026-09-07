"""Count ANSI commands and printable spans in bounded exported art frames."""
import collections
import json
import pathlib
import re
import statistics
import sys

root = pathlib.Path(sys.argv[1])
records = [json.loads(line) for line in (root/'inputs.ndjson').read_text().splitlines()]
results = []
for index, record in enumerate(records):
    raw = (root/f'frame-{index:02}.ansi').read_bytes()
    counts = collections.Counter()
    byte_counts = collections.Counter()
    colors = collections.Counter()
    chars = collections.Counter()
    spans = []
    tokens = list(re.finditer(rb'\x1b\[[0-?]*[ -/]*[@-~]|[^\x1b]+', raw))
    assert b''.join(t[0] for t in tokens) == raw
    for token in tokens:
        value = token[0]
        if value.startswith(b'\x1b['):
            command = value[-1:]
            if command == b'm':
                args = value[2:-1].decode()
                if args.startswith('38;'):
                    kind = 'foreground'
                    colors[args] += 1
                elif args.startswith('48;'):
                    kind = 'background'
                else:
                    kind = 'reset_or_other_sgr'
            elif command in (b'H', b'f'):
                kind = 'cursor_absolute'
            elif command in (b'C', b'D', b'G'):
                kind = 'cursor_horizontal'
            else:
                kind = 'other_csi_' + command.decode()
            counts[kind] += 1
            byte_counts[kind] += len(value)
        else:
            text = value.decode()
            assert all(ord(c) >= 32 for c in text), repr(text)
            chars.update(text)
            spans.append(len(text))
            counts['printable_characters'] += len(text)
            counts['ascii_characters'] += sum(ord(c)<128 for c in text)
            byte_counts['text'] += len(value)
    assert sum(byte_counts.values()) == len(raw)
    results.append(dict(frame=record['frame_index'], grid=record['grid'],
        changed_cells=record['changed_cells'], dirty_runs=record['runs'],
        logged_total_bytes=record['bytes'], art_bytes=len(raw),
        ui_and_sync_bytes=record['bytes']-len(raw),
        counts=dict(counts), byte_counts=dict(byte_counts),
        printable_spans=len(spans), median_span=statistics.median(spans),
        single_character_spans=sum(n==1 for n in spans),
        spaces=chars[' '], distinct_characters=len(chars), distinct_foregrounds=len(colors),
        most_common_characters=chars.most_common(12),
        prefix=raw.decode()[:240]))
(root/'ansi-summary.json').write_text(json.dumps(results,indent=2)+'\n')
for row in results:
    print(json.dumps({k:row[k] for k in ['frame','changed_cells','art_bytes','ui_and_sync_bytes','counts','byte_counts','printable_spans','median_span','single_character_spans','spaces','distinct_foregrounds']}))
