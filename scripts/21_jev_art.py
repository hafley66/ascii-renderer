"""Generate with --generate (paid API call); otherwise replay the saved response."""
import json, sys, urllib.request, urllib.error
from pathlib import Path

out = Path(__file__).resolve().parents[1] / 'art' / 'jev'
out.mkdir(parents=True, exist_ok=True)
w, h = 32, 16
palette = {
    'black': (8, 12, 18), 'navy': (15, 26, 52),
    'blue': (38, 65, 95), 'teal': (32, 88, 91),
    'green': (30, 65, 53), 'silver': (143, 179, 192),
    'white': (224, 237, 223), 'brown': (67, 51, 42),
}
state = {
    'image_description': 'A moonlit pine forest. A luminous full moon in the upper right, tall dark evergreen silhouettes on both sides, a winding silver stream through the center, distant blue mountains beneath a deep navy night sky. Restrained teal and silver pixel art, readable silhouettes, no text or border.',
    'width': w, 'height': h,
    'coordinates': 'Origin (0,0) at top left. x increases right, y increases down. Each cell is half as wide as it is tall, so the full image appears square.',
    'task': 'Compose one coherent recognizable pixel-art image. Choose the palette color at each requested coordinate.',
}
request = {'model': 'jev-latest', 'state': state, 'questions': {
    f'x{x}_y{y}': {'type': 'choice', 'instructions': f'What color is the pixel at x={x}, y={y} in the described image?', 'criteria': dict.fromkeys(palette)}
    for y in range(h) for x in range(w)
}}
if '--generate' in sys.argv:
    (out/'request.json').write_text(json.dumps(request, indent=2)+'\n')
    key = (Path.home()/'.config/typesafe/api-key').read_text().strip()
    req = urllib.request.Request('https://api.typesafe.ai/v1/systemone', data=json.dumps(request).encode(), headers={'Authorization': 'Bearer '+key, 'Content-Type': 'application/json'})
    try:
        with urllib.request.urlopen(req, timeout=60) as response:
            data = json.load(response)
    except urllib.error.HTTPError as e:
        print('API HTTP status:', e.code)
        print(e.read(2048).decode(errors='replace').replace(key, '[REDACTED]'))
        raise SystemExit(1)
    except urllib.error.URLError as e:
        print('Network error:', str(e.reason).replace(key, '[REDACTED]'))
        raise SystemExit(2)
    (out/'response.json').write_text(json.dumps(data, indent=2)+'\n')
else:
    data = json.loads((out/'response.json').read_text())
print('Model:', data.get('model'))
print('Answers:', len(data.get('answers', {})))
print('Usage:', json.dumps(data.get('usage', {})))

# The selected colors are Jev's argmax choices; no hand-authored scene geometry.
glyphs = dict(zip(palette, ' .:+^*@|'))
answers = data['answers']
expected = {f'x{x}_y{y}' for y in range(h) for x in range(w)}
if set(answers) != expected:
    raise SystemExit('Response does not contain exactly the requested coordinates')
plain, ansi, grid = [], [], [f'{w} {h}']
for y in range(h):
    text_row, ansi_row = [], []
    for x in range(w):
        name = answers[f'x{x}_y{y}']['choice']
        rgb = palette[name]
        ch = glyphs[name]
        text_row.append(ch)
        ansi_row.append(f'\x1b[38;2;{rgb[0]};{rgb[1]};{rgb[2]}m{ch}')
        grid.append(f"{ord(ch)} {','.join(map(str, rgb))} 8,12,18")
    plain.append(''.join(text_row))
    ansi.append(''.join(ansi_row) + '\x1b[0m')
(out/'forest.txt').write_text('\n'.join(plain)+'\n')
(out/'forest.ansi').write_text('\x1b[48;2;8;12;18m' + '\n\x1b[48;2;8;12;18m'.join(ansi)+'\n')
(out/'forest.grid').write_text('\n'.join(grid)+'\n')
print('\n'.join(plain))
print(f'Artifacts: {out}')
