"""Ten Jev frame experiments. Replay cached requests without contacting the API."""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import time
import urllib.error
import urllib.request

from PIL import Image, ImageDraw, ImageFont

spec = importlib.util.spec_from_file_location('pixels', Path(__file__).with_name('27_jev_pixels.py'))
pixels = importlib.util.module_from_spec(spec)
spec.loader.exec_module(pixels)
ROOT = pixels.ROOT / 'art/jev/ten-experiments'
PALETTE = dict(black=[0,0,0], navy=[14,24,48], blue=[45,85,150], cyan=[65,190,207],
               white=[235,240,225], gold=[240,175,60], red=[180,55,55], gray=[105,115,125])
SCENE = ('Create a recognizable front-facing owl at night. Two separate large bright eyes, '
         'a small pointed beak between and below them, pointed ear tufts, rounded body, '
         'two folded wings, and two feet. Dark empty margin around the owl. '
         'Choose the composition yourself. Coordinates start at top left; x grows right and y grows down.')
USER = 'keep trying, come up with 10 unique experiments'
NAMES = ['binary-silhouette', 'direct-palette', 'ascii-glyphs', 'ansi-cells',
         'probability-field', 'coordinate-control', 'semantic-regions',
         'adaptive-tiles', 'sequential-commands', 'grid-revision']


def choice(instructions, options):
    return dict(type='choice', instructions=instructions, criteria=dict.fromkeys(map(str, options)))


class Run:
    def __init__(self, number, offline=False):
        self.number = number
        self.folder = ROOT / f'{number:02}_{NAMES[number]}'
        self.folder.mkdir(parents=True, exist_ok=True)
        self.offline = offline
        self.records = ([json.loads(s) for s in (self.folder/'io.jsonl').read_text().splitlines()]
                        if (self.folder/'io.jsonl').exists() else [])
        if not self.records:
            pixels.append(self.folder, 'prompt', role='user', content=USER)
            pixels.append(self.folder, 'plan', experiment=NAMES[number], scene=SCENE,
                          palette=PALETTE, renderer='28_jev_experiments.py',
                          note='All choices returned by Jev. Fixed display mappings are documented per request.')
        reqs = {r['id']:r['body'] for r in self.records if r['event']=='request'}
        resolved = {r['request_id'] for r in self.records if r['event'] in ('response','network_error')}
        assert set(reqs)<=resolved, 'Unresolved paid request; inspect before resuming'
        self.cache = {json.dumps(reqs[r['request_id']],sort_keys=True):r['body']['answers']
                      for r in self.records if r['event']=='response' and r['status']==200}
        self.attempt = len(reqs)

    def ask(self, state, questions):
        out = {}
        items = list(questions.items())
        for start in range(0,len(items),48):
            payload = dict(model='jev-latest',state=state,questions=dict(items[start:start+48]))
            cache_key = json.dumps(payload,sort_keys=True)
            if cache_key in self.cache:
                out.update(self.cache[cache_key]); continue
            assert not self.offline, 'Missing cached response'
            key=(Path.home()/'.config/typesafe/api-key').read_text().strip()
            for retry in range(5):
                rid=f'request_{self.attempt:04}';self.attempt+=1
                pixels.append(self.folder,'request',id=rid,body=payload,endpoint=pixels.ENDPOINT)
                req=urllib.request.Request(pixels.ENDPOINT,data=json.dumps(payload).encode(),
                    headers={'Authorization':'Bearer '+key,'Content-Type':'application/json'})
                began=time.monotonic()
                try:
                    with urllib.request.urlopen(req,timeout=60) as res: raw=res.read().decode()
                except urllib.error.HTTPError as exc:
                    pixels.append(self.folder,'response',request_id=rid,status=exc.code,
                        body_text=exc.read().decode().replace(key,'[REDACTED]'))
                    if exc.code in (429,529) and retry<4:
                        time.sleep(2**retry);continue
                    raise
                except (urllib.error.URLError,TimeoutError) as exc:
                    pixels.append(self.folder,'network_error',request_id=rid,error=str(exc).replace(key,'[REDACTED]'))
                    raise
                try: body=json.loads(raw)
                except json.JSONDecodeError:
                    pixels.append(self.folder,'response',request_id=rid,status=200,body_text=raw.replace(key,'[REDACTED]'))
                    raise
                pixels.append(self.folder,'response',request_id=rid,status=200,body=body,seconds=time.monotonic()-began)
                answers=body['answers'];assert set(answers)==set(payload['questions'])
                for name,a in answers.items():
                    q=payload['questions'][name]
                    if q['type']=='choice':assert a['choice'] in q['criteria']
                    else:assert 0<=a['noul']<=1
                self.cache[cache_key]=answers;out.update(answers)
                print(self.number,rid,start+len(answers),'/',len(items),flush=True)
                break
        return out

    def save(self, im, **metadata):
        im.save(self.folder/'image.png')
        pixels.append(self.folder,'image',file='image.png',size=list(im.size),
            sha256=hashlib.sha256((self.folder/'image.png').read_bytes()).hexdigest(),**metadata)
        assert sorted(p.name for p in self.folder.iterdir())==['image.png','io.jsonl']
        print('COMPLETE',self.number,self.folder,flush=True)


def grid(run):
    n=20;kind=run.number
    state=dict(scene=SCENE,canvas=[n,n],palette=PALETTE,
               instruction='Each question selects the content at its explicit coordinate in the same complete frame.')
    if kind==5:
        state['scene']='Coordinate control: white if (x-9.5)^2+(y-9.5)^2 <= 49, otherwise black. A white disk on black.'
    if kind==9:
        source=ROOT/'01_direct-palette/io.jsonl'
        original={}
        for line in source.read_text().splitlines():
            r=json.loads(line)
            if r['event']=='response' and r['status']==200:original.update(r['body']['answers'])
        state['previous_rows']=[[original[f'{x}_{y}']['choice'] for x in range(n)] for y in range(n)]
        state['revision']='Review this complete previous attempt. Select every pixel anew to make the requested owl recognizable. Repair eyes, beak, silhouette and wings. Coordinates are identical.'
    questions={}
    glyphs=[' ','.',':','-','=','+','*','#','@','/','\\','|','_','^','v','<','>','(',')','O','o']
    regions={'background':'black','body':'gray','wing':'blue','eye':'gold','pupil':'black','beak':'red','foot':'gold','ear':'gray'}
    if kind==6:state['region_colors']=regions
    if kind==4:state['display']='Your probability p is displayed directly as grayscale round(255*p), with no threshold.'
    if kind==0:state['display']='Every owl silhouette cell is white, background black. Include the whole solid silhouette.'
    for y in range(n):
        for x in range(n):
            pos=f'At column x={x}, row y={y} of the {n}x{n} frame'
            k=f'{x}_{y}'
            if kind==0:questions[k]=choice(pos+', is this inside the owl silhouette or background?', ['owl','background'])
            elif kind in (1,5,9):questions[k]=choice(pos+', select its final pixel color.', ['black','white'] if kind==5 else PALETTE)
            elif kind in (2,3):
                questions[k]=choice(pos+', select the single ASCII character that depicts this part of the owl.',glyphs)
                if kind==3:
                    questions[k+'_fg']=choice(pos+', select foreground glyph color.',PALETTE)
                    questions[k+'_bg']=choice(pos+', select background cell color.',PALETTE)
            elif kind==4:questions[k]=dict(type='noul',instructions=pos+', should this pixel be bright in a monochrome drawing of the owl?')
            elif kind==6:questions[k]=choice(pos+', which visible semantic region occupies this cell? Eye means iris; pupil means its dark center.',regions)
    a=run.ask(state,questions)
    if kind in (2,3):
        im=Image.new('RGB',(320,400));d=ImageDraw.Draw(im)
        font=ImageFont.truetype('/System/Library/Fonts/Menlo.ttc',19)
        ansi=[]
        for y in range(n):
            row=''
            for x in range(n):
                k=f'{x}_{y}';g=a[k]['choice'];fg=PALETTE[a[k+'_fg']['choice']] if kind==3 else PALETTE['white'];bg=PALETTE[a[k+'_bg']['choice']] if kind==3 else PALETTE['black']
                d.rectangle((x*16,y*20,x*16+15,y*20+19),fill=tuple(bg))
                d.text((x*16+2,y*20+10),g,font=font,anchor='lm',fill=tuple(fg))
                row+='\x1b[38;2;'+ ';'.join(map(str,fg))+';48;2;'+ ';'.join(map(str,bg))+'m'+g
            ansi.append(row+'\x1b[0m')
        pixels.append(run.folder,'ansi',content='\n'.join(ansi),columns=n,rows=n)
    else:
        im=Image.new('RGB',(n,n));colors=[]
        for y in range(n):
            for x in range(n):
                v=a[f'{x}_{y}']
                if kind==4:c=[round(255*v['noul'])]*3
                else:
                    name=v['choice']
                    if kind==0:name='white' if name=='owl' else 'black'
                    if kind==6:name=regions[name]
                    c=PALETTE[name]
                colors.append(tuple(c))
        im.putdata(colors);im=im.resize((320,320),Image.Resampling.NEAREST)
    metadata=dict(logical_size=[n,n],questions=len(questions))
    if kind==5:
        metadata['correct_cells']=sum(a[f'{x}_{y}']['choice']==('white' if (x-9.5)**2+(y-9.5)**2<=49 else 'black') for y in range(n) for x in range(n))
    run.save(im,**metadata)


def tiles(run):
    im=Image.new('RGB',(32,32));d=ImageDraw.Draw(im);nodes=[(0,0,32)];depth=0
    while nodes:
        state=dict(scene=SCENE,canvas=[32,32],palette=PALETTE,
                   instruction='Choose a uniform tile color or split into four equal quadrants to capture owl details. Splitting continues to 2x2 tiles.')
        q={str(i):choice(f'Tile with top-left ({x},{y}), size {s}x{s}: choose its uniform color'+(' or split if it contains differing regions.' if s>2 else '.'),list(PALETTE)+(['split'] if s>2 else [])) for i,(x,y,s) in enumerate(nodes)}
        a=run.ask(state,q);next_nodes=[]
        for i,(x,y,s) in enumerate(nodes):
            c=a[str(i)]['choice']
            if c=='split':
                h=s//2;next_nodes.extend((x+dx,y+dy,h) for dy in (0,h) for dx in (0,h))
            else:d.rectangle((x,y,x+s-1,y+s-1),fill=tuple(PALETTE[c]))
        nodes=next_nodes;depth+=1
    run.save(im.resize((320,320),Image.Resampling.NEAREST),logical_size=[32,32],depth=depth)


def commands(run):
    im=Image.new('RGB',(100,100),'black');d=ImageDraw.Draw(im);history=[]
    for step in range(24):
        state=dict(scene=SCENE,canvas=[100,100],palette=PALETTE,previous_commands=history,
            instruction='Create the entire composition yourself through 24 ordered drawing commands. Each command is painted over previous ones. Background starts black. x,y are centers; width,height are full extents. Lines run between opposite corners of that box; slash reverses the diagonal. Use large shapes first, then eyes, beak, feathers, feet and highlights. stop ends the painting.')
        q={'shape':choice(f'Choose drawing command {step} shape.', ['ellipse','rectangle','triangle','line','slash','stop']),
           'color':choice(f'Choose command {step} color.',PALETTE)}
        for field in ('x','y'):q[field]=choice(f'Choose command {step} center {field} coordinate, 0..99.',range(0,100,3))
        for field in ('width','height'):q[field]=choice(f'Choose command {step} {field} in pixels.',range(3,100,3))
        a=run.ask(state,q);c={k:v['choice'] for k,v in a.items()};history.append(c)
        if c['shape']=='stop':break
        x,y,w,h=(int(c[k]) for k in ('x','y','width','height'));box=(x-w//2,y-h//2,x+w//2,y+h//2);color=tuple(PALETTE[c['color']]);l,t,r,b=box
        if c['shape']=='ellipse':d.ellipse(box,fill=color)
        elif c['shape']=='rectangle':d.rectangle(box,fill=color)
        elif c['shape']=='triangle':d.polygon([(x,t),(l,b),(r,b)],fill=color)
        elif c['shape']=='line':d.line((l,t,r,b),fill=color,width=1)
        else:d.line((l,b,r,t),fill=color,width=1)
    pixels.append(run.folder,'commands',values=history)
    run.save(im.resize((400,400),Image.Resampling.NEAREST),logical_size=[100,100],command_count=len(history))


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('numbers',nargs='*',type=int,default=list(range(10)));p.add_argument('--offline',action='store_true');args=p.parse_args()
    for number in args.numbers:
        assert 0<=number<10
        run=Run(number,args.offline)
        if number==7:tiles(run)
        elif number==8:commands(run)
        else:grid(run)
