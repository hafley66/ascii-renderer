"""Verify saved API contracts, PNGs, Grid/plain/ANSI equivalence and motion.

Run using the E2E venv, which supplies Pillow and pyte, or install those packages.
No network calls; no terminal process is launched.
"""
import hashlib
import json
import math
from pathlib import Path

from PIL import Image
import pyte

ROOT=Path(__file__).resolve().parents[1]
OUT=ROOT/'art/jev/exploration'
metadata=sorted(OUT.rglob('*.meta.json'))
cells_checked=0
for path in metadata:
    stem=path.name.removesuffix('.meta.json')
    base=path.parent
    m=json.loads(path.read_text())
    grid=(base/(stem+'.grid')).read_text().splitlines()
    w,h=map(int,grid[0].split())
    assert (w,h)==(m['width'],m['height']) and len(grid)==1+w*h, path
    assert hashlib.sha256((base/(stem+'.grid')).read_bytes()).hexdigest()==m['grid_sha256'],path
    plain=(base/(stem+'.txt')).read_text().splitlines()
    assert len(plain)==h and all(len(row)==w for row in plain),path
    # Account for the tty's ordinary ONLCR conversion; leave a spare row/column.
    screen=pyte.Screen(w+1,h+1)
    pyte.ByteStream(screen).feed((base/(stem+'.ansi')).read_bytes().replace(b'\n',b'\r\n'))
    for y in range(h):
        for x in range(w):
            ch,fg,bg=grid[1+y*w+x].split()
            assert chr(int(ch))==plain[y][x],(path,x,y)
            terminal=screen.buffer[y][x]
            assert terminal.data==plain[y][x],(path,x,y,'glyph',terminal.data,plain[y][x])
            for field,expected in [('fg',fg),('bg',bg)]:
                rgb=tuple(map(int,expected.split(',')))
                assert len(rgb)==3 and all(0<=v<=255 for v in rgb)
                assert getattr(terminal,field)==''.join(f'{v:02x}' for v in rgb),(path,x,y,field)
            cells_checked+=1
    with Image.open(base/(stem+'.png')) as im:
        im.verify()

questions=0
for request_path in OUT.glob('*/request.json'):
    request=json.loads(request_path.read_text())
    response=json.loads(request_path.with_name('response.json').read_text())
    assert set(request['questions'])==set(response['answers']),request_path
    for key,q in request['questions'].items():
        a=response['answers'][key]
        assert a['type']==q['type'],(request_path,key)
        if a['type']=='noul':
            assert math.isfinite(a['noul']) and 0<=a['noul']<=1
        else:
            probabilities=a['probabilities']
            assert all(math.isfinite(v) and 0<=v<=1 for v in probabilities.values())
            assert abs(sum(probabilities.values())-1)<.015,(request_path,key)
            if a['type']=='choice':
                assert a['choice'] in q['criteria']
                assert set(probabilities)==set(q['criteria'])
            else:
                assert 0<=a['score']<=len(q['criteria'])-1
        questions+=1

with Image.open(OUT/'22_abyssal_motion.gif') as im:
    assert im.n_frames==12
motion=list((OUT/'motion').glob('*.grid'))
assert len(motion)==12 and len({hashlib.sha256(p.read_bytes()).hexdigest() for p in motion})==12
report=dict(status='passed',pictures=len(metadata),terminal_cells_checked=cells_checked,
            api_questions_validated=questions,distinct_motion_frames=12,
            scope='offline API/Grid/plain/PNG validation and pyte ANSI replay; GUI painting untested')
(OUT/'validation.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(report))
