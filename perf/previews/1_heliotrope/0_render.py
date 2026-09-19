"""Render heliotrope .grid dumps (from the ignored `preview` test with HP=path) to PNG."""
from PIL import Image, ImageDraw, ImageFont
from pathlib import Path
import sys

font = ImageFont.truetype('/System/Library/Fonts/Menlo.ttc', 13)


def rgb(s):
    return tuple(map(int, s.split(','))) if s != 'x' else (0, 0, 0)


for f in map(Path, sys.argv[1:]):
    lines = f.read_text().splitlines()
    w, h = map(int, lines[0].split())
    cw, ch = 8, 16
    im = Image.new('RGB', (w * cw, h * ch))
    d = ImageDraw.Draw(im)
    for i, line in enumerate(lines[1:]):
        c, fg, bg = line.split()
        x, y = i % w * cw, i // w * ch
        d.rectangle((x, y, x + cw - 1, y + ch - 1), fill=rgb(bg))
        if int(c) != 32:
            d.text((x, y - 1), chr(int(c)), font=font, fill=rgb(fg))
    im.save(f.with_suffix('.png'))
