"""Bounded color review, using the repository's serialized Cell format.

python3 perf/previews/101_pelagium/0_render.py BINARY OUTPUT_DIRECTORY
Adapted from perf/previews/0_azulejo/0_render.py; no terminal playback.
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
cases = [(f'seed{seed}_t{t}', 80, 24, seed, t, .5)
         for seed, t in [(42, 0), (42, 4), (42, 11), (7, 0), (913, 0)]]
cases += [('large', 160, 56, 42, 4, .5), ('small', 24, 9, 42, 0, .5)]
cases += [(f'aperture_{a:.2f}', 80, 24, 42, 0, a)
          for a in [0, .25, .49, .5, .51, .75, 1]]
manifest = []
thumbnails = []
for name, w, h, seed, t, aperture in cases:
    env = dict(os.environ, ASCII_GRID_W=str(w), ASCII_GRID_H=str(h),
               ASCII_GRID_DUMP='1', ASCII_T=str(t), ASCII_TRACE='0')
    command = [binary, str(seed), 'pelagium', 'deep', str(aperture), '.55', '.7', '.6', '.65']
    result = subprocess.run(command, env=env, capture_output=True, check=True, timeout=10)
    (root / f'{name}.grid').write_bytes(result.stdout)
    rows = result.stdout.decode().splitlines()
    assert tuple(map(int, rows[0].split())) == (w, h)
    cells = [line.split() for line in rows[1:]]
    assert len(cells) == w*h
    image = Image.new('RGB', (w*8, h*16+28), (2, 7, 13))
    draw = ImageDraw.Draw(image)
    draw.text((8, 4), name, font=font, fill=(156, 179, 188))
    for i, (ch, fg, bg) in enumerate(cells):
        color = lambda s: tuple(map(int, s.split(','))) if s != 'x' else (2, 7, 13)
        x, y = (i % w)*8, (i // w)*16+28
        draw.rectangle((x, y, x+7, y+15), fill=color(bg))
        draw.text((x, y-1), chr(int(ch)), font=font, fill=color(fg))
    image.save(root / f'{name}.png')
    if name != 'large':
        tile = Image.new('RGB', (640, 412), (2, 7, 13))
        tile.paste(image, ((640-image.width)//2, 0))
        thumbnails.append(tile)
    manifest.append(dict(name=name, command=command, width=w, height=h, time=t))
gallery = Image.new('RGB', (640*3, 412*((len(thumbnails)+2)//3)), (2, 7, 13))
for i, tile in enumerate(thumbnails):
    gallery.paste(tile, ((i%3)*640, (i//3)*412))
gallery.save(root/'gallery.png')
(root/'manifest.json').write_text(json.dumps(manifest, indent=2)+'\n')
print(root/'gallery.png')
