"""Offline native-mode compositions controlled by recorded Jev decisions."""
import importlib.util
import json
import math
import os
from pathlib import Path
import random
import subprocess
import hashlib

spec = importlib.util.spec_from_file_location('jev_lab', Path(__file__).with_name('22_jev_explore.py'))
lab = importlib.util.module_from_spec(spec)
spec.loader.exec_module(lab)
OUT = lab.OUT
W, H = 160, 68
BLANK = (' ', (0, 0, 0), (5, 9, 17))


def read_grid(text):
    lines = text.splitlines()
    w, h = map(int, lines[0].split())
    assert len(lines) == 1+w*h
    cells = []
    for y in range(h):
        row = []
        for line in lines[1+y*w:1+(y+1)*w]:
            char, fg, bg = line.split()
            color = lambda s: tuple(map(int, s.split(','))) if s != 'x' else (5,9,17)
            row.append((chr(int(char)), color(fg), color(bg)))
        cells.append(row)
    return cells


def capture(name, scene, answers):
    cache = OUT/'07_director'/f'{name}.native.grid'
    settings = {}
    for key, values in scene['params'].items():
        choice = answers[f'{name}_{key}']['choice']
        settings['ASCII_P_'+key] = str(values[['low','medium','high'].index(choice)])
    settings.update(ASCII_GRID_DUMP='1', ASCII_GRID_W=str(W), ASCII_GRID_H=str(H), ASCII_T='5.0')
    command = [str(lab.ROOT/'target/release/ascii-renderer'), '4269', scene['mode'], scene['theme']]
    if not cache.exists():
        env = {k:v for k,v in os.environ.items() if not k.startswith('ASCII_')}
        result = subprocess.run(command, env=env|settings, text=True, capture_output=True, timeout=15, check=True)
        cells = read_grid(result.stdout)
        assert len(cells)==H and len(cells[0])==W
        cache.write_text(result.stdout)
        lab.save_json(cache.with_suffix('.receipt.json'), dict(command=command, environment=settings,
             binary_sha256=hashlib.sha256(Path(command[0]).read_bytes()).hexdigest(), stderr=result.stderr))
    return read_grid(cache.read_text())


def tint(cells, palette):
    colors = {'abyss':[(33,76,129),(76,202,211),(242,209,140)],
              'ember':[(94,35,46),(224,93,47),(255,222,150)],
              'lichen':[(30,78,74),(119,204,151),(231,244,212)]}[palette]
    def recolor(c):
        v=max(c)/255
        a,b = (colors[0],colors[1]) if v<.55 else (colors[1],colors[2])
        t=v/.55 if v<.55 else (v-.55)/.45
        return lab.rgb([(aa*(1-t)+bb*t)*min(1,v*2) for aa,bb in zip(a,b)])
    return [[(ch,recolor(fg),recolor(bg)) for ch,fg,bg in row] for row in cells]


def warp(cells, style, phase=0):
    result=[]
    for y in range(H):
        row=[]
        for x in range(W):
            u=(x-W/2)/(H*2);v=(y-H/2)/H
            r=math.hypot(u,v);a=math.atan2(v,u)
            if style=='vortex':
                a+=2.8*math.exp(-r*3)+phase
            elif style=='kaleidoscope':
                a=abs((a+phase)%(math.pi/3)-math.pi/6)
                r*=1.15
            elif style=='mirror':
                u=abs(u);a=math.atan2(v,u)
            sx=round(W/2+math.cos(a)*r*H*2)
            sy=round(H/2+math.sin(a)*r*H)
            row.append(cells[sy][sx] if 0<=sx<W and 0<=sy<H else BLANK)
        result.append(row)
    return result


def put(cells,x,y,ch,color):
    x,y=round(x),round(y)
    if 0<=x<W and 0<=y<H:
        cells[y][x]=(ch,lab.rgb(color),cells[y][x][2])


def drown(cells, amount):
    result=[row[:] for row in cells]
    level=round(H*(.82-.2*amount))
    for y in range(level,H):
        sy=max(0,level-1-(y-level)*2)
        for x in range(W):
            sx=round(x+math.sin(y*1.8+amount*5)*3+math.sin(x*.1+y)*1.2)%W
            ch,fg,bg=cells[sy][sx]
            light=.58*(1-(y-level)/max(1,H-level))+.12
            result[y][x]=(ch if (x+y)%4 else '~',tuple(v*light for v in fg),(5,14,24))
    return result


def grow_roots(cells, material, amount, seed=42):
    rng=random.Random(seed)
    result=[row[:] for row in cells]
    planes=lab.fields(material,['root','spore','glass'],20,16,(W,H))
    def branch(x,y,a,length,width,depth):
        for step in range(int(length)):
            a+=rng.uniform(-.09,.09)
            x+=math.cos(a)*1.6;y+=math.sin(a)*.65
            ix,iy=round(x),round(y)
            if not (0<=ix<W and 0<=iy<H):break
            root=planes[0][iy*W+ix]
            color=(60+root*80,105+root*105,86+root*80)
            for offset in range(-width,width+1):
                put(result,x+offset,y,'|' if abs(math.sin(a))>.65 else '/' if math.cos(a)*math.sin(a)<0 else '\\',color)
            if depth and step in [int(length*.4),int(length*.7)]:
                branch(x,y,a+rng.choice([-1,1])*rng.uniform(.35,.85),length*.5,max(0,width-1),depth-1)
            if rng.random()<planes[1][iy*W+ix]*.25*amount:
                put(result,x+2,y-1,'*',(255,190,89))
    for side in [-1,1]:
        for n in range(round(3+amount*5)):
            branch(W/2+side*(W*.43-n*2),H*.8, -math.pi/2-side*.25, (25+n*3)*amount,1,3)
    return result


def main():
    scenes=json.loads((OUT/'07_director/spec.json').read_text())
    answers=json.loads((OUT/'07_director/response.json').read_text())['answers']
    sources={}
    for i,(name,scene) in enumerate(scenes.items(),7):
        native=capture(name,scene,answers)
        palette=answers[f'{name}_palette']['choice']
        style=answers[f'{name}_transform']['choice']
        sources[name]=tint(native,palette)
        lab.save_art(f'{i:02}_{name}_native',scene['prompt'].split('.')[0].upper()[:80],native,
                     f'Native {scene["mode"]} / Jev selected parameters / seed 4269 / t=5')
        transformed=warp(sources[name],style)
        lab.save_art(f'{i:02}_{name}_{style}',f'{name.upper()} / {style.upper()} / {palette.upper()}',transformed,
                     'Jev selected native controls, transformation and palette / saved native grid / offline composition')
    material=json.loads((OUT/'06_drowned/response.json').read_text())['answers']
    for n,amount in enumerate([.35,.7,1.0],10):
        result=grow_roots(sources['cathedral'],material,amount)
        result=drown(result,amount)
        lab.save_art(f'{n:02}_monastery_growth',f'THE MONASTERY REMEMBERS / {round(amount*100)}%',result,
                     'Jev material probabilities + Jev-directed native cathedral / procedural roots and rising reflected water')


if __name__=='__main__':
    main()
