"""Saved Jev experiments. Network calls are explicit: `call JOB`; render is offline.

Requires Pillow. Each job retains exact requests, responses, usage, and pictures.
"""
import argparse
import colorsys
import hashlib
import json
import math
import os
from pathlib import Path
import random
import subprocess
import time
import urllib.error
import urllib.request

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "art/jev/exploration"
FONT_PATH = "/System/Library/Fonts/Menlo.ttc"
PALETTE = {
    "void": [5, 9, 20], "midnight": [24, 39, 83],
    "cobalt": [44, 96, 201], "cyan": [81, 219, 222],
    "ivory": [245, 237, 199], "amber": [242, 164, 71],
    "coral": [225, 87, 88], "moss": [93, 148, 113],
}
RAMP = " .,:;irsXA253hMHGS#9B&@"


def save_json(path, data):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(data, indent=2) + "\n")


def rgb(value):
    return tuple(max(0, min(255, round(v))) for v in value)


def save_art(name, title, cells, note):
    """Persist one exact character grid, its PNG preview, and replay formats."""
    h, w = len(cells), len(cells[0])
    assert all(len(row) == w for row in cells)
    OUT.mkdir(parents=True, exist_ok=True)
    text, ansi, serialized = [], [], [f"{w} {h}"]
    im = Image.new("RGB", (max(w * 8 + 48, 800), h * 14 + 104), (8, 12, 21))
    d = ImageDraw.Draw(im)
    font = ImageFont.truetype(FONT_PATH, 13)
    head = ImageFont.truetype(FONT_PATH, 17)
    small = ImageFont.truetype(FONT_PATH, 11)
    d.text((24, 14), title, font=head, fill=(142, 227, 228))
    for y, row in enumerate(cells):
        text.append("".join(c[0] for c in row))
        line = []
        for x, (ch, fg, bg) in enumerate(row):
            fg, bg = rgb(fg), rgb(bg)
            d.rectangle((24 + 8*x, 48 + 14*y, 31 + 8*x, 61 + 14*y), fill=bg)
            d.text((24 + 8*x, 47 + 14*y), ch, font=font, fill=fg)
            line.append(f"\x1b[38;2;{fg[0]};{fg[1]};{fg[2]}m\x1b[48;2;{bg[0]};{bg[1]};{bg[2]}m{ch}")
            serialized.append(f"{ord(ch)} {','.join(map(str, fg))} {','.join(map(str, bg))}")
        ansi.append("".join(line) + "\x1b[0m")
    d.text((24, h*14 + 66), note[:150], font=small, fill=(149, 160, 177))
    im.save(OUT / f"{name}.png")
    (OUT / f"{name}.txt").write_text("\n".join(text) + "\n")
    (OUT / f"{name}.ansi").write_text("\n".join(ansi) + "\n")
    (OUT / f"{name}.grid").write_text("\n".join(serialized) + "\n")
    save_json(OUT / f"{name}.meta.json", dict(title=title, note=note, width=w, height=h,
              grid_sha256=hashlib.sha256((OUT / f"{name}.grid").read_bytes()).hexdigest()))
    print(f"PICTURE {name}: {title}", flush=True)
    return im


def fields(answers, labels, w, h, size=(144, 64), prefix=""):
    planes = []
    for label in labels:
        plane = Image.new("F", (w, h))
        plane.putdata([answers[f"{prefix}x{x}_y{y}"]["probabilities"].get(label, 0)
                       for y in range(h) for x in range(w)])
        planes.append(list(plane.resize(size, Image.Resampling.BILINEAR).getdata()))
    return planes


def render_probabilities(name, answers, labels, palette, w, h, style):
    W, H = 144, 64
    planes = fields(answers, labels, w, h, (W, H))
    rng = random.Random(42)
    cells = []
    for y in range(H):
        row = []
        for x in range(W):
            i = y*W+x
            probs = [p[i] for p in planes]
            total = sum(probs)
            probs = [p / max(total, 1e-9) for p in probs]
            mean = [sum(p*palette[label][c] for p, label in zip(probs, labels)) for c in range(3)]
            entropy = -sum(p*math.log(max(p, 1e-12)) for p in probs)/math.log(len(labels))
            light = sum(mean)/765
            if style == "sample":
                color = palette[rng.choices(labels, probs)[0]]
                ch = rng.choice(".:+*#")
                bg = [v*.2 for v in mean]
            elif style == "contour":
                bands = entropy*22
                ch = "+" if bands % 1 < .15 else "." if bands % 1 < .3 else " "
                color = [75+160*entropy, 235-85*entropy, 215-130*entropy]
                bg = [v*.12 for v in mean]
            else:
                color = [min(255, v*1.35+15) for v in mean]
                ch = RAMP[min(len(RAMP)-1, int(light*(len(RAMP)-1)))]
                bg = [v*.35 for v in mean]
            row.append((ch, color, bg))
        cells.append(row)
    return save_art(name, name.replace('_', ' ').upper(), cells,
                    f"Jev distributions / {style} / bilinear field / seed 42 / {w}x{h} model samples")


def recovered():
    original = json.loads((ROOT / 'art/jev/response.json').read_text())
    palette = dict(zip(['black', 'navy', 'blue', 'teal', 'green', 'silver', 'white', 'brown'],
                       [(8,12,18),(15,26,52),(38,65,95),(32,88,91),(30,65,53),(143,179,192),(224,237,223),(67,51,42)]))
    for n, style in enumerate(['mean', 'sample', 'contour'], 1):
        render_probabilities(f'{n:02}_forest_{style}', original['answers'], list(palette), palette, 32, 16, style)


def prepare():
    jobs = {
        '04_jellyfish': dict(kind='noul', w=24, h=24,
            prompt='A single enormous alien jellyfish shaped like a Gothic cathedral. Frontal view, bilateral symmetry. Its luminous bell is a pointed dome centered at (0.50,0.28), spanning x=0.15 to 0.85 and y=0.08 to 0.48. Five long curling tentacles hang from the bell down to y=0.93. Black empty ocean surrounds the silhouette. No floor, no border, no text.'),
        '05_moth': dict(kind='choice', w=24, h=24,
            prompt='A cosmic death-head moth, symmetrical wings spread wide, seen from above. Its narrow amber body runs vertically x=0.5, y=0.25 to 0.8. Upper wings are huge ivory and coral triangles from the shoulders toward (0.08,0.12) and (0.92,0.12). Lower wings are cobalt and cyan with concentric eye spots at (0.25,0.63) and (0.75,0.63). Surrounding space is almost black. A tiny ivory skull pattern sits on the thorax. A centered specimen filling 85 percent of the square.'),
        '06_drowned': dict(kind='semantic', w=20, h=16,
            prompt='A drowned monastery remembering that it was once a forest. A symmetrical ruined Gothic cathedral fills the center. A rose window at (0.50,0.30); ribbed arches rise between x=0.25 and 0.75. Tree roots swallow columns from both sides. Luminous fungi sprout from stone. The lower third is dark water reflecting the architecture. The upper corners are night sky. Blue silver light from the window, amber fungi, deep teal water. Perspective view down the central nave.'),
    }
    materials = {'void': 'empty dark sky or negative space', 'stone': 'cathedral columns, vaults and masonry',
                 'root': 'tree trunks, roots, branches or organic growth', 'water': 'floodwater or reflections',
                 'glass': 'luminous stained glass or moonlight', 'spore': 'bioluminescent fungi and floating spores'}
    for name, job in jobs.items():
        questions = {}
        for y in range(job['h']):
            for x in range(job['w']):
                loc = f"normalized image point x={(x+.5)/job['w']:.4f}, y={(y+.5)/job['h']:.4f}, origin top-left"
                if job['kind'] == 'noul':
                    q = dict(type='noul', instructions=f'Is {loc} inside the jellyfish silhouette (bell or a tentacle)?')
                else:
                    criteria = materials if job['kind'] == 'semantic' else dict.fromkeys(PALETTE)
                    q = dict(type='choice', instructions=f"Which {'material occupies' if job['kind']=='semantic' else 'palette color belongs at'} {loc}?", criteria=criteria)
                questions[f'x{x}_y{y}'] = q
        request = dict(model='jev-latest', state=dict(image=job['prompt'], width=job['w'], height=job['h'],
                       task='Imagine one coherent image. Evaluate each question against this same composition.'), questions=questions)
        save_json(OUT / name / 'spec.json', job)
        save_json(OUT / name / 'request.json', request)
    print('Prepared:', ', '.join(jobs))


def call(name):
    folder = OUT / name
    target = folder / 'response.json'
    if target.exists():
        raise SystemExit('Saved response already exists; refusing to spend again or overwrite it')
    request = json.loads((folder / 'request.json').read_text())
    key = (Path.home() / '.config/typesafe/api-key').read_text().strip()
    req = urllib.request.Request('https://api.typesafe.ai/v1/systemone',
              data=json.dumps(request).encode(), headers={'Authorization': 'Bearer '+key, 'Content-Type': 'application/json'})
    started = time.monotonic()
    try:
        with urllib.request.urlopen(req, timeout=60) as response:
            data = json.load(response)
    except urllib.error.HTTPError as exc:
        detail = exc.read(4096).decode(errors='replace').replace(key, '[REDACTED]')
        save_json(folder/'error.json', dict(status=exc.code, detail=detail))
        raise SystemExit(f'HTTP {exc.code}: {detail}')
    save_json(target, data)
    save_json(folder/'receipt.json', dict(seconds=round(time.monotonic()-started, 3),
               question_count=len(request['questions']), model=data.get('model'), usage=data.get('usage')))
    if set(data.get('answers', {})) != set(request['questions']):
        raise SystemExit('Saved response has missing or unexpected answers')
    print(name, data.get('model'), len(data['answers']), data.get('usage'), flush=True)


def render_job(name):
    job = json.loads((OUT/name/'spec.json').read_text())
    data = json.loads((OUT/name/'response.json').read_text())
    if job['kind'] == 'choice':
        for style in ['mean', 'sample', 'contour']:
            render_probabilities(name+'_'+style, data['answers'], list(PALETTE), PALETTE, job['w'], job['h'], style)
    elif job['kind'] == 'noul':
        plane = Image.new('F', (job['w'], job['h']))
        plane.putdata([data['answers'][f'x{x}_y{y}']['noul'] for y in range(job['h']) for x in range(job['w'])])
        W,H=144,72
        p=list(plane.resize((W,H),Image.Resampling.BILINEAR).getdata())
        for style in ['fog','contours']:
            cells=[]
            for y in range(H):
                row=[]
                for x in range(W):
                    v=p[y*W+x]
                    if style=='fog':
                        ch=RAMP[min(len(RAMP)-1,int(v*(len(RAMP)-1)))]
                        color=(70+170*v,150+100*v,210+40*v)
                    else:
                        band=v*16
                        ch='+' if band%1<.2 and v>.04 else '.' if band%1<.4 and v>.04 else ' '
                        color=(80+175*v,210,235-140*v)
                    row.append((ch,color,(5+v*8,9+v*18,20+v*24)))
                cells.append(row)
            save_art(name+'_'+style,'CATHEDRAL JELLYFISH / '+style.upper(),cells,'Jev silhouette probabilities / 24x24 model samples / interpolated to 144x72')
    else:
        palette=dict(zip(['void','stone','root','water','glass','spore'],
                         [(8,13,26),(108,131,148),(49,112,97),(35,89,151),(181,230,236),(240,162,70)]))
        for style in ['mean','sample','contour']:
            render_probabilities(name+'_'+style,data['answers'],list(palette),palette,job['w'],job['h'],style)


if __name__ == '__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action',choices=['recover','prepare','call','render'])
    parser.add_argument('job',nargs='?')
    args=parser.parse_args()
    if args.action=='recover': recovered()
    elif args.action=='prepare': prepare()
    elif args.action=='call': call(args.job)
    elif args.action=='render': render_job(args.job)
