"""Offline sampled native scenes, motion export, contact sheets and gallery index."""
import argparse
import importlib.util
import json
import math
from pathlib import Path
import random

from PIL import Image, ImageDraw, ImageFont


def module(name,file):
    spec=importlib.util.spec_from_file_location(name,Path(__file__).with_name(file))
    mod=importlib.util.module_from_spec(spec);spec.loader.exec_module(mod)
    return mod


comp=module('compositions','23_jev_compositions.py')
lab=comp.lab


def sample():
    scenes=json.loads((lab.OUT/'07_director/spec.json').read_text())
    original=json.loads((lab.OUT/'07_director/response.json').read_text())['answers']
    n=16
    for name,scene in scenes.items():
        for seed,temp in [(71,2.0),(193,5.0)]:
            rng=random.Random(seed)
            answers={}
            for key,answer in original.items():
                probabilities=answer['probabilities']
                labels=list(probabilities)
                weights=[probabilities[label]**(1/temp) for label in labels]
                answers[key]=dict(answer,choice=rng.choices(labels,weights)[0])
            variant=f'_sample_{seed}'
            lab.save_json(lab.OUT/'07_director'/f'{name}{variant}.decisions.json',
                          dict(seed=seed,temperature=temp,answers=answers))
            cells=comp.capture(name,scene,answers,variant,seed)
            style=answers[f'{name}_transform']['choice']
            palette=answers[f'{name}_palette']['choice']
            cells=comp.warp(comp.tint(cells,palette),style)
            lab.save_art(f'{n:02}_{name}_sample_{seed}',f'{name.upper()} / DRAW {seed} / {style.upper()} / {palette.upper()}',
                         cells,f'Sampled Jev control distributions / temperature {temp:g} / seed {seed} / saved native renderer output')
            n+=1


def animate():
    organisms=module('organisms','24_jev_organisms.py')
    (lab.OUT/'motion').mkdir(exist_ok=True)
    frames=[]
    for n in range(12):
        phase=math.tau*n/12
        ink=organisms.medusa(phase)
        im=lab.save_art(f'motion/{n:02}_medusa',f'ABYSSAL OBSERVATORY / FRAME {n+1:02}/12',
                        ink.cells(1.8),f'Jev scores and silhouette field / phase {phase:.6f} / offline deterministic frame')
        frames.append(im.resize((792,668),Image.Resampling.LANCZOS))
    # One global palette avoids palette flicker across the animation.
    palette=frames[0].quantize(colors=128)
    frames=[f.quantize(palette=palette,dither=Image.Dither.NONE) for f in frames]
    frames[0].save(lab.OUT/'22_abyssal_motion.gif',save_all=True,append_images=frames[1:],duration=150,loop=0,disposal=2)
    lab.save_json(lab.OUT/'motion/recipe.json',dict(frames=12,phases=[math.tau*n/12 for n in range(12)],
                   duration_ms=150,source='scripts/24_jev_organisms.py:medusa',api_calls=0))
    contact(list((lab.OUT/'motion').glob('*.png')),lab.OUT/'22_motion_contact.png',4,320)


def contact(paths,target,columns=3,width=400):
    paths=sorted(paths)
    rowheight=round(width*.85)+38
    sheet=Image.new('RGB',(width*columns,rowheight*math.ceil(len(paths)/columns)),(8,12,21))
    draw=ImageDraw.Draw(sheet);font=ImageFont.truetype(lab.FONT_PATH,12)
    for i,path in enumerate(paths):
        im=Image.open(path).convert('RGB');im.thumbnail((width-12,rowheight-38),Image.Resampling.LANCZOS)
        x=(i%columns)*width;y=(i//columns)*rowheight
        sheet.paste(im,(x+(width-im.width)//2,y))
        draw.text((x+8,y+rowheight-28),path.stem[:48],font=font,fill=(166,217,221))
    sheet.save(target)


def index():
    pictures=sorted(lab.OUT.glob('*.meta.json'))
    lines=['# Jev exploration gallery','',
           '[Animated abyssal observatory](22_abyssal_motion.gif) · [All motion frames](22_motion_contact.png) · [Progress and prompts](0_progress.md)','']
    manifest=[]
    for i,path in enumerate(pictures,1):
        stem=path.name.removesuffix('.meta.json');m=json.loads(path.read_text())
        lines.extend([f'## {i}. {m["title"]}','',f'![{m["title"]}]({stem}.png)','',m['note'],'',
                      f'[PNG]({stem}.png) · [ANSI]({stem}.ansi) · [Grid]({stem}.grid)',''])
        manifest.append(dict(name=stem,**m))
    lines.extend(['## Motion frames','','![Every motion frame](22_motion_contact.png)',''])
    for path in sorted((lab.OUT/'motion').glob('*.png')):
        lines.append(f'- [{path.stem}](motion/{path.name})')
    (lab.OUT/'index.md').write_text('\n'.join(lines)+'\n')
    usage={'input_tokens':0,'output_tokens':0}
    receipts=[]
    for path in sorted(lab.OUT.glob('*/receipt.json')):
        data=json.loads(path.read_text());receipts.append(dict(job=path.parent.name,**data))
        for key in usage:usage[key]+=data['usage'][key]
    lab.save_json(lab.OUT/'manifest.json',dict(pictures=manifest,api_receipts=receipts,usage=usage,
                 still_count=len(pictures),motion_frame_count=len(list((lab.OUT/'motion').glob('*.meta.json')))))
    # Contact sheets grouped into readable chronological pages.
    for start in range(0,len(pictures),9):
        paths=[p.with_name(p.name.replace('.meta.json','.png')) for p in pictures[start:start+9]]
        contact(paths,lab.OUT/f'contact_{start//9+1:02}.png')
    print(f'Gallery: {len(pictures)} stills, 12 motion frames, usage {usage}')


if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('action',choices=['sample','animate','index'])
    args=p.parse_args();{'sample':sample,'animate':animate,'index':index}[args.action]()
