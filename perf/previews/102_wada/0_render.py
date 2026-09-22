"""Bounded color review for wada using the repository's serialized Cell format.

python3 perf/previews/102_wada/0_render.py BINARY OUTPUT_DIRECTORY
Adapted from perf/previews/101_pelagium/0_render.py; no terminal playback.
"""
import json
import os
from pathlib import Path
import subprocess
import sys

from PIL import Image, ImageDraw, ImageFont

binary = str(Path(sys.argv[1]).resolve())
root = Path(sys.argv[2])
root.mkdir(parents=True, exist_ok=True)
font = ImageFont.truetype('/System/Library/Fonts/Menlo.ttc', 13)

DEFAULT = ['5', '1', '.45', '.5', '.15', '.7', '.8', '6', '34', '.35', '2']


def knobs(**over):
    keys = ['ORDER', 'RELAX', 'TWIST', 'WARP', 'SPIN', 'TRAP', 'VEIN',
            'CONTOUR', 'HUE', 'GRAIN', 'ASPECT']
    values = list(DEFAULT)
    for key, value in over.items():
        values[keys.index(key.upper())] = str(value)
    return values


cases = [
    ('s42_t0', 80, 24, '42', '0', 'moss', knobs()),
    ('s42_t5', 80, 24, '42', '5', 'moss', knobs()),
    ('s42_t11', 80, 24, '42', '11', 'moss', knobs()),
    ('s7_t0', 80, 24, '7', '0', 'moss', knobs()),
    ('s913_t0', 80, 24, '913', '0', 'moss', knobs()),
    ('order3', 80, 24, '42', '0', 'moss', knobs(ORDER=3)),
    ('order7', 80, 24, '42', '0', 'moss', knobs(ORDER=7)),
    ('order12', 80, 24, '42', '0', 'moss', knobs(ORDER=12)),
    ('warp0', 80, 24, '42', '0', 'moss', knobs(WARP=0)),
    ('warp_max', 80, 24, '42', '0', 'moss', knobs(WARP=1.5)),
    ('twist_max', 80, 24, '42', '0', 'moss', knobs(TWIST=0.9)),
    ('relax_low', 80, 24, '42', '0', 'moss', knobs(RELAX=0.6)),
    ('relax_high', 80, 24, '42', '0', 'moss', knobs(RELAX=1.3)),
    ('traps_veins_max', 80, 24, '42', '0', 'moss', knobs(TRAP=1.5, VEIN=1.5)),
    ('bare', 80, 24, '42', '0', 'moss', knobs(CONTOUR=0, GRAIN=0, TRAP=0, VEIN=0)),
    ('neon', 80, 24, '42', '0', 'neon', knobs()),
    ('large', 160, 56, '42', '4', 'moss', knobs()),
    ('small', 24, 9, '42', '0', 'moss', knobs()),
]

manifest = []
thumbnails = []
for name, w, h, seed, t, theme, values in cases:
    env = dict(os.environ, ASCII_GRID_W=str(w), ASCII_GRID_H=str(h),
               ASCII_GRID_DUMP='1', ASCII_T=t, ASCII_TRACE='0')
    command = [binary, seed, 'wada', theme] + values
    result = subprocess.run(command, env=env, capture_output=True, check=True, timeout=30)
    (root / f'{name}.grid').write_bytes(result.stdout)
    rows = result.stdout.decode().splitlines()
    assert tuple(map(int, rows[0].split())) == (w, h)
    cells = [line.split() for line in rows[1:]]
    assert len(cells) == w * h
    image = Image.new('RGB', (w * 8, h * 16 + 28), (2, 7, 13))
    draw = ImageDraw.Draw(image)
    draw.text((8, 4), f'wada {name}', font=font, fill=(156, 179, 188))
    for i, (ch, fg, bg) in enumerate(cells):
        color = lambda s: tuple(map(int, s.split(','))) if s != 'x' else (2, 7, 13)
        x, y = (i % w) * 8, (i // w) * 16 + 28
        draw.rectangle((x, y, x + 7, y + 15), fill=color(bg))
        draw.text((x, y - 1), chr(int(ch)), font=font, fill=color(fg))
    image.save(root / f'{name}.png')
    tile = Image.new('RGB', (640, 412), (2, 7, 13))
    tile.paste(image, ((640 - image.width) // 2, (412 - 28 - image.height) // 2 + 28))
    thumbnails.append(tile)
    manifest.append(dict(name=name, command=command[1:], width=w, height=h, time=t))
gallery = Image.new('RGB', (640 * 3, 412 * ((len(thumbnails) + 2) // 3)), (2, 7, 13))
for i, tile in enumerate(thumbnails):
    gallery.paste(tile, ((i % 3) * 640, (i // 3) * 412))
gallery.save(root / 'gallery.png')
(root / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
print(root / 'gallery.png')
