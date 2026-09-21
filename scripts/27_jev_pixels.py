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


def init(folder, prompt, width, height):
    if not prompt or not (1 <= width <= 128 and 1 <= height <= 128):
        raise SystemExit('Supply --prompt and dimensions from 1 to 128')
    folder.mkdir(parents=True, exist_ok=True)
    if any(folder.iterdir()):
        raise SystemExit('Use a new empty run folder')
    state = dict(task='Render one coherent pixel-art image. Each question selects the final color of one actual pixel.',
                 image=prompt, canvas=dict(width=width, height=height,
                 coordinates=f'Square pixels. Integer coordinates, origin top-left. x=0..{width-1}, y=0..{height-1}.'),
                 palette_rgb=PALETTE)
    questions = {f'x{x}_y{y}': dict(type='choice',
                 instructions=f'What is the final color of pixel (x={x}, y={y}) in this {width} by {height} image?',
                 criteria=dict.fromkeys(PALETTE)) for y in range(height) for x in range(width)}
    append(folder, 'prompt', role='user', content=prompt)
    append(folder, 'plan', request=dict(model='jev-latest', state=state, questions=questions), batch_size=256, display_scale=20)


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


def render(folder):
    records, plan = read(folder)
    request = plan['request']; answers = collect(records, request)
    assert set(answers) == set(request['questions']), 'Pixel answers incomplete'
    state = request['state']; w, h = state['canvas']['width'], state['canvas']['height']
    palette = state['palette_rgb']; scale = plan['display_scale']
    pixels = [tuple(palette[answers[f'x{x}_y{y}']['choice']]) for y in range(h) for x in range(w)]
    im = Image.new('RGB', (w, h)); im.putdata(pixels)
    im.resize((w * scale, h * scale), Image.Resampling.NEAREST).save(folder / 'image.png')
    with Image.open(folder / 'image.png') as saved:
        assert all(saved.getpixel((x*scale+scale//2, y*scale+scale//2)) == pixels[y*w+x]
                   for y in range(h) for x in range(w))
    append(folder, 'image', file='image.png', source_size=[w, h], display_scale=scale,
           pixels_verified=w*h, method='exact returned palette choice, nearest-neighbor display scaling')
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
    pending = [(k, v) for k, v in request['questions'].items() if k not in answered]
    attempt = sum(r['event'] == 'request' for r in records)
    for start in range(0, len(pending), plan['batch_size']):
        payload = dict(request, questions=dict(pending[start:start+plan['batch_size']]))
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
            raise SystemExit(f'API HTTP {exc.code}; outcome saved in io.jsonl')
        except urllib.error.URLError as exc:
            append(folder, 'network_error', request_id=request_id,
                   error=str(exc.reason).replace(key, '[REDACTED]'), seconds=time.monotonic()-began)
            raise SystemExit('Network error saved in io.jsonl')
        try:
            data = json.loads(raw)
        except json.JSONDecodeError:
            append(folder, 'response', request_id=request_id, status=200, body_text=raw.replace(key, '[REDACTED]'))
            raise SystemExit('Non-JSON API response saved in io.jsonl')
        append(folder, 'response', request_id=request_id, status=200, body=data, seconds=time.monotonic()-began)
        assert set(data['answers']) == set(payload['questions']), 'Incomplete response saved; inspect io.jsonl'
        print(request_id, len(data['answers']), data.get('usage'), flush=True)
    render(folder)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=['init', 'call', 'render'])
    parser.add_argument('--folder', type=Path, default=ROOT/'art/jev/direct-pixels')
    parser.add_argument('--prompt')
    parser.add_argument('--width', type=int, default=32)
    parser.add_argument('--height', type=int, default=32)
    args = parser.parse_args()
    if args.action == 'init': init(args.folder, args.prompt, args.width, args.height)
    elif args.action == 'call': call(args.folder)
    else: render(args.folder)
