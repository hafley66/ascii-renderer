"""Direct Jev pixels. Each run contains io.jsonl and image.png.

init records a prompt and pixel questions; call sends missing batches and renders;
render reconstructs the image offline from the saved API responses.
"""
import argparse
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import time
import urllib.error
import urllib.request

from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
ENDPOINT = 'https://api.typesafe.ai/v1/systemone'
PALETTE = {
    'black': [0, 0, 0], 'midnight': [8, 16, 32], 'deep_blue': [17, 42, 80],
    'blue': [27, 86, 145], 'teal': [30, 125, 146], 'cyan': [65, 190, 207],
    'pale_cyan': [139, 226, 232], 'white': [231, 252, 244],
    'dark_amber': [121, 61, 23], 'amber': [235, 155, 62], 'pale_gold': [255, 220, 139],
}


def append(folder, event, **values):
    record = dict(v=1, recorded_at=datetime.now(timezone.utc).isoformat(), event=event, **values)
    with (folder / 'io.jsonl').open('a') as stream:
        stream.write(json.dumps(record, ensure_ascii=False) + '\n')
        stream.flush()
        os.fsync(stream.fileno())


def read(folder):
    records = [json.loads(line) for line in (folder / 'io.jsonl').read_text().splitlines()]
    plans = [r for r in records if r['event'] == 'plan']
    assert len(plans) == 1, 'Expected one run plan'
    return records, plans[0]


def init(folder, prompt, width, height, strategy='independent', display_scale=20):
    if not prompt or not (1 <= width <= 256 and 1 <= height <= 256):
        raise SystemExit('Supply --prompt and dimensions from 1 to 256')
    if display_scale < 1 or display_scale > 20:
        raise SystemExit('Display scale must be from 1 to 20')
    folder.mkdir(parents=True, exist_ok=True)
    if any(folder.iterdir()):
        raise SystemExit('Use a new empty run folder')
    state = dict(task='Render one coherent pixel-art image. Each question selects the final color of one actual pixel.',
                 image=prompt, canvas=dict(width=width, height=height,
                 coordinates=f'Square pixels. Integer coordinates, origin top-left. x=0..{width-1}, y=0..{height-1}.'),
                 palette_rgb=PALETTE)
    if strategy == 'rows':
        state.update(
            input_structure='image describes the entire target image; canvas gives its size and coordinates; palette_rgb gives available colors; completed_rows contains your already chosen rows, top to bottom; requested_row is the row being drawn now.',
            output_structure='Return one Choice answer for every question key xX_yY. Its choice must be one palette color name. Each answer supplies exactly the final RGB color of that one pixel. The caller places these colors directly into the image.',
            history_encoding='Each character in a completed row is one pixel. Decode it with palette_codes. Array index is y, character index is x. Rows after completed_rows are still undrawn. Continue the intended whole composition into those rows; earlier background pixels do not imply that later rows should remain background.',
            palette_codes=dict(zip('0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ', PALETTE)),
        )
    questions = {f'x{x}_y{y}': dict(type='choice',
                 instructions=f'What is the final color of pixel (x={x}, y={y}) in this {width} by {height} image?',
                 criteria=dict.fromkeys(PALETTE)) for y in range(height) for x in range(width)}
    append(folder, 'prompt', role='user', content=prompt)
    append(folder, 'plan', request=dict(model='jev-latest', state=state, questions=questions),
           batch_size=width if strategy == 'rows' else 256, display_scale=display_scale, strategy=strategy)


def collect(records, request):
    inputs = {r['id']: r['body'] for r in records if r['event'] == 'request'}
    answers = {}
    for r in records:
        if r['event'] == 'response' and r['status'] == 200:
            expected = inputs[r['request_id']]['questions']
            assert set(r['body']['answers']) == set(expected)
            assert not set(answers).intersection(expected), 'Duplicate successful pixel answers'
            answers.update(r['body']['answers'])
    assert set(answers) <= set(request['questions'])
    return answers


def render(folder, plan=None, answers=None, partial=False):
    if plan is None:
        records, plan = read(folder)
        answers = collect(records, plan['request'])
    request = plan['request']
    if not partial:
        assert set(answers) == set(request['questions']), 'Pixel answers incomplete'
    state = request['state']; w, h = state['canvas']['width'], state['canvas']['height']
    target_h = h
    if partial:
        h = 0
        while h < target_h and all(f'x{x}_y{h}' in answers for x in range(w)):
            h += 1
        if h == 0:
            return
    palette = state['palette_rgb']; scale = plan['display_scale']
    pixels = [tuple(palette[answers[f'x{x}_y{y}']['choice']]) for y in range(h) for x in range(w)]
    im = Image.new('RGB', (w, h)); im.putdata(pixels)
    im.resize((w * scale, h * scale), Image.Resampling.NEAREST).save(folder / 'image.png')
    with Image.open(folder / 'image.png') as saved:
        assert all(saved.getpixel((x*scale+scale//2, y*scale+scale//2)) == pixels[y*w+x]
                   for y in range(h) for x in range(w))
    append(folder, 'image', file='image.png', source_size=[w, h], display_scale=scale,
           pixels_verified=w*h, complete=h == target_h, target_size=[w, target_h],
           method='exact returned palette choice, nearest-neighbor display scaling')
    print(f'{w*h} Jev pixels: {folder / "image.png"}')


def call(folder):
    records, plan = read(folder); request = plan['request']
    answered = collect(records, request)
    resolved = {r['request_id'] for r in records if r['event'] in ('response', 'network_error')}
    if any(r['event'] == 'request' and r['id'] not in resolved for r in records):
        raise SystemExit('A logged request has no recorded outcome; inspect io.jsonl before another paid request')
    if set(answered) == set(request['questions']):
        render(folder)
        return
    key = (Path.home() / '.config/typesafe/api-key').read_text().strip()
    attempt = sum(r['event'] == 'request' for r in records)
    batch_size = plan['batch_size']
    history_encoding = 'characters'
    for record in records:
        if record['event'] == 'batch_size':
            batch_size = record['value']
        elif record['event'] == 'history_encoding':
            history_encoding = record['value']
    rate_retries = 0
    state = request['state']; width = state['canvas']['width']
    total = len(request['questions'])
    usage = {'input_tokens': 0, 'output_tokens': 0}
    while len(answered) < total:
        pending = [(k, v) for k, v in request['questions'].items() if k not in answered]
        payload = dict(request, questions=dict(pending[:batch_size]))
        current_row = None
        if plan.get('strategy') == 'rows':
            current_row = int(pending[0][0].split('_y')[1])
            row_questions = [(k, v) for k, v in pending if k.endswith(f'_y{current_row}')]
            encode = {name: code for code, name in state['palette_codes'].items()}
            completed = [''.join(encode[answered[f'x{x}_y{y}']['choice']] for x in range(width))
                         for y in range(current_row)]
            payload = dict(request, state=dict(state, completed_rows=completed, requested_row=current_row),
                           questions=dict(row_questions[:batch_size]))
            if history_encoding == 'runs':
                from itertools import groupby
                payload['state']['completed_rows'] = [
                    ' '.join(f'{code}:{sum(1 for _ in group)}' for code, group in groupby(row))
                    for row in completed]
                payload['state']['history_encoding'] = (
                    'completed_rows is lossless run-length encoding. Array index is y. '
                    'Each space-separated code:count repeats that palette_codes code count times, '
                    'left to right. For example 0:100 5:100 means 100 black pixels then 100 cyan pixels. '
                    'Every decoded row has canvas.width pixels. Later rows are undrawn. '
                    'Continue the entire intended composition into requested_row.')
        request_id = f'attempt_{attempt:04}'; attempt += 1
        append(folder, 'request', id=request_id, method='POST', endpoint=ENDPOINT, body=payload)
        req = urllib.request.Request(ENDPOINT, data=json.dumps(payload).encode(),
                    headers={'Authorization': 'Bearer '+key, 'Content-Type': 'application/json'})
        began = time.monotonic()
        try:
            with urllib.request.urlopen(req, timeout=60) as response:
                raw = response.read().decode()
        except urllib.error.HTTPError as exc:
            raw = exc.read().decode(errors='replace').replace(key, '[REDACTED]')
            append(folder, 'response', request_id=request_id, status=exc.code, body_text=raw,
                   seconds=time.monotonic()-began)
            if exc.code == 400 and 'max_tokens_exceeded' in raw and len(payload['questions']) > 1:
                batch_size = max(1, len(payload['questions']) // 2)
                append(folder, 'batch_size', value=batch_size, reason='API max_tokens_exceeded; same row history retained')
                continue
            if exc.code == 429 and rate_retries < 5:
                delay = min(10, 2 ** rate_retries)
                rate_retries += 1
                append(folder, 'retry', request_id=request_id, reason='rate_limit', delay_seconds=delay)
                time.sleep(delay)
                continue
            raise SystemExit(f'API HTTP {exc.code}; outcome saved in io.jsonl')
        except (urllib.error.URLError, TimeoutError) as exc:
            append(folder, 'network_error', request_id=request_id,
                   error=str(getattr(exc, 'reason', exc)).replace(key, '[REDACTED]'), seconds=time.monotonic()-began)
            raise SystemExit('Network error saved in io.jsonl')
        try:
            data = json.loads(raw)
        except json.JSONDecodeError:
            append(folder, 'response', request_id=request_id, status=200, body_text=raw.replace(key, '[REDACTED]'))
            raise SystemExit('Non-JSON API response saved in io.jsonl')
        append(folder, 'response', request_id=request_id, status=200, body=data, seconds=time.monotonic()-began)
        assert set(data['answers']) == set(payload['questions']), 'Incomplete response saved; inspect io.jsonl'
        assert all(a['choice'] in state['palette_rgb'] for a in data['answers'].values())
        answered.update(data['answers'])
        rate_retries = 0
        for k in usage:
            usage[k] += data.get('usage', {}).get(k, 0)
        print(request_id, f'{len(answered)}/{total} pixels', f'row={current_row}', data.get('usage'), flush=True)
        if current_row is not None and (current_row + 1) % 20 == 0:
            render(folder, plan, answered, partial=True)
    append(folder, 'session_usage', **usage)
    render(folder, plan, answered)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=['init', 'call', 'render'])
    parser.add_argument('--folder', type=Path, default=ROOT/'art/jev/direct-pixels')
    parser.add_argument('--prompt')
    parser.add_argument('--width', type=int, default=32)
    parser.add_argument('--height', type=int, default=32)
    parser.add_argument('--strategy', choices=['independent', 'rows'], default='independent')
    parser.add_argument('--scale', type=int, default=20)
    args = parser.parse_args()
    if args.action == 'init': init(args.folder, args.prompt, args.width, args.height, args.strategy, args.scale)
    elif args.action == 'call': call(args.folder)
    else: render(args.folder)
