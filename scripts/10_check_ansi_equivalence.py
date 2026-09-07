"""Compare visible terminal cells after each frame with pyte==0.8.2.
Usage: python3 scripts/10_check_ansi_equivalence.py BEFORE AFTER OUTPUT.json
Inputs are bounded frame-NN.ansi sequences for the 400x200 workload.
"""
import json
import pathlib
import sys
import pyte

before, after, output = map(pathlib.Path, sys.argv[1:])
files = sorted(before.glob('frame-*.ansi'))
assert 1 <= len(files) <= 20
assert [p.name for p in files] == [p.name for p in sorted(after.glob('frame-*.ansi'))]
assert sum(p.stat().st_size + (after/p.name).stat().st_size for p in files) <= 16*1024**2
screens = [pyte.Screen(400, 200), pyte.Screen(400, 200)]
streams = [pyte.ByteStream(screen) for screen in screens]
results = []
for index, path in enumerate(files):
    streams[0].feed(path.read_bytes())
    streams[1].feed((after/path.name).read_bytes())
    for y in range(200):
        for x in range(400):
            left, right = (screen.buffer[y][x] for screen in screens)
            # pyte retains an empty continuation marker after overwriting a
            # wide glyph. It paints no text, just as a space does. Preserve
            # background and modifiers in the comparison.
            if left.data in ('', ' ') and right.data in ('', ' '):
                left, right = (cell._replace(data=' ', fg='default') for cell in (left, right))
            assert left == right, (index, x, y, left, right)
    cursor = [(screen.cursor.x, screen.cursor.y) for screen in screens]
    assert cursor[0] == cursor[1], (index, cursor)
    results.append(dict(frame=index+1, compared_cells=80000, differing_visible_cells=0,
                        cursor=list(cursor[0])))
output.write_text(json.dumps(results, indent=2)+'\n')
print(f'{len(files)*80000} visible cell comparisons matched')
