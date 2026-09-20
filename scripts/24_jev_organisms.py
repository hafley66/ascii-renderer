"""Procedural ASCII specimens driven by saved Jev scores and probability fields.

All rendering is offline. `python3 scripts/24_jev_organisms.py moth` exports a
specimen; medusa and engine select the other experiments. No API requests here.
"""
import argparse
import importlib.util
import json
import math
from pathlib import Path
import random

spec=importlib.util.spec_from_file_location('lab',Path(__file__).with_name('22_jev_explore.py'))
lab=importlib.util.module_from_spec(spec);spec.loader.exec_module(lab)
W,H=192,88
TAU=math.tau
SCORES=json.loads((lab.OUT/'13_organisms/response.json').read_text())['answers']


def controls(scene):
    return {k:SCORES[f'{scene}_{k}']['score']/4 for k in
            ['complexity','symmetry','luminescence','turbulence','ornament','warmth']}


class Ink:
    def __init__(self):
        self.energy=[0.]*(W*H)
        self.colors=[[0.,0.,0.] for _ in range(W*H)]
        self.symbols=[' ']*(W*H)

    def dot(self,x,y,power,color,ch=None):
        # Continuous positions accumulate into the four adjacent terminal cells.
        ix,iy=math.floor(x),math.floor(y)
        for dx in [0,1]:
            for dy in [0,1]:
                xx,yy=ix+dx,iy+dy
                if 0<=xx<W and 0<=yy<H:
                    weight=(1-abs(x-xx))*(1-abs(y-yy))*power
                    i=yy*W+xx
                    self.energy[i]+=weight
                    for c in range(3):self.colors[i][c]+=weight*color[c]
                    if ch and weight>.12:self.symbols[i]=ch

    def point(self,u,v,power,color,ch=None):
        self.dot(W/2+u*W*.45,H/2+v*H*.45,power,color,ch)

    def cells(self,exposure=1,etch=False):
        result=[]
        for y in range(H):
            row=[]
            for x in range(W):
                i=y*W+x;e=self.energy[i]
                brightness=1-math.exp(-e*exposure)
                col=[v/max(e,1e-10) for v in self.colors[i]]
                if etch:
                    col=[219,211,174]
                    brightness=brightness**.6
                ch=self.symbols[i] if self.symbols[i]!=' ' and brightness>.3 else ' .,:;i+*oO#@'[min(11,int(brightness*12))]
                fg=[v*(.2+.8*brightness) for v in col]
                bg=[4+v*brightness*.075 for v in col]
                row.append((ch,fg,bg))
            result.append(row)
        return result

    def stars(self,seed,count=160):
        rng=random.Random(seed)
        for _ in range(count):
            x,y=rng.randrange(W),rng.randrange(H)
            if self.energy[y*W+x]<.12:
                self.dot(x,y,rng.uniform(.2,.9),(109,168,200),'.')


def moth(phase=0):
    k=controls('moth');ink=Ink()
    answers=json.loads((lab.OUT/'05_moth/response.json').read_text())['answers']
    planes=lab.fields(answers,list(lab.PALETTE),24,24,(W,H))
    palette=list(lab.PALETTE.values())
    for y in range(H):
        v=(y-H*.48)/(H*.45)
        for x in range(W):
            u=(x-W/2)/(W*.45);a=abs(u)
            # Two oblique wing lobes on each side, sharing a narrow thorax.
            dx=a-.43;dy=v+.22
            X=dx*.90-dy*.43;Y=dx*.43+dy*.90
            upper=(X/.51)**2+(Y/.31)**2
            lower=((a-.32)/.31)**2+((v-.38)/.43)**2
            radius=min(upper,lower)
            if radius<=1 and a>.055:
                angle=math.atan2(v+.1,a-.045)
                veins=abs(math.sin(angle*(15+round(k['complexity']*14))+a*4))
                edge=math.exp(-abs(radius-.96)*55)
                filigree=(.5+.5*math.sin(radius*(38+22*k['ornament'])+math.sin(angle*7)*3+phase))**12
                eye=math.hypot((a-(.52 if upper<lower else .32))*1.3,(v-(-.22 if upper<lower else .39))*1.8)
                eyes=math.exp(-abs(eye-.15)*65)+.65*math.exp(-abs(eye-.22)*85)
                iris=.65*math.exp(-abs(eye-.09)*65)
                power=(.07+.55*filigree+edge*1.2+eyes*1.8+iris)*(.45+.55*min(1,veins*5))
                if veins<.035:power+=.5
                i=y*W+x
                mean=[sum(p[i]*color[c] for p,color in zip(planes,palette)) for c in range(3)]
                warm=min(1,eyes+edge*.2)
                color=[mean[c]*(1-warm)+[245,183,94][c]*warm for c in range(3)]
                color=[min(255,v*1.5+30) for v in color]
                ink.dot(x,y,power*2,color,'/' if u*v<0 else '\\' if veins<.08 else None)
            body=(u/.038)**2+((v-.14)/.55)**2
            if body<1:
                ink.dot(x,y,1.5*(1-body)+.3,(245,182+35*math.sin(v*65),95),'=')
    for side in [-1,1]:
        for j in range(320):
            t=j/319
            u=side*(.035+.23*t+.07*math.sin(t*5))
            v=-.39-.48*t+.08*t*t
            ink.point(u,v,.8,(209,221,165))
            if j%12==0:
                for n in range(10):
                    ink.point(u+side*n*.006,v+n*.003,.22,(151,205,179))
    ink.stars(42,110)
    return ink


def medusa(phase=0):
    k=controls('medusa');ink=Ink();rng=random.Random(731)
    answers=json.loads((lab.OUT/'04_jellyfish/response.json').read_text())['answers']
    occupancy=[answers[f'x{x}_y{y}']['noul'] for y in range(24) for x in range(24)]
    # Meridians and parallels on a translucent lobed shell.
    ribs=round(20+k['complexity']*34)
    for rib in range(ribs):
        phi=TAU*rib/ribs
        for j in range(160):
            t=j/159*math.pi/2
            radius=.76*math.sin(t)*(1+.035*math.cos(phi*9+t*5+phase))
            u=radius*math.cos(phi)
            v=-.24-.52*math.cos(t)+.12*math.sin(phi)*math.sin(t)
            front=.5+.5*(math.sin(phi)+1)/2
            ink.point(u,v,.3*front,(82,209,225))
    for band in range(1,12):
        t=band/12*math.pi/2
        for j in range(480):
            phi=TAU*j/480
            r=.76*math.sin(t)*(1+.035*math.cos(phi*9+t*5+phase))
            ink.point(r*math.cos(phi),-.24-.52*math.cos(t)+.12*math.sin(phi)*math.sin(t),.13,(59,143,205))
    for j in range(1800):
        a=j*.031
        r=.17*(.5+.5*math.sin(j*.011))
        ink.point(r*math.cos(a),-.34+r*.65*math.sin(a),.65,(255,176,78))
    filaments=round(24+k['ornament']*55)
    for n in range(filaments):
        start=(n/(filaments-1)*2-1)*.73
        length=rng.uniform(.55,1.25)*(1-.2*abs(start))
        frequency=rng.uniform(5,12)
        for j in range(240):
            t=j/239
            u=start*(1-.34*t)+math.sin(t*frequency+phase+n*.71)*(.015+.055*t)+.13*t*t*math.sin(n*.21+phase)
            v=-.2+t*length+.06*math.cos(n*.63)
            ix=max(0,min(23,int((u+1)*12)));iy=max(0,min(23,int((v+1)*12)))
            model=occupancy[iy*24+ix]
            color=(80+100*(n%7==0),170+65*model,225-70*(n%7==0))
            ink.point(u,v,(.19+.25*model)*(1-.55*t),color)
            if n%8==0 and j%24<3:ink.point(u,v,.8,(245,212,145),'o')
    ink.stars(87,220)
    return ink


def engine(phase=0):
    k=controls('engine');ink=Ink();rng=random.Random(919)
    data=json.loads((lab.OUT/'06_drowned/response.json').read_text())['answers']
    planes=lab.fields(data,['root','water','glass','spore'],20,16,(W,H))
    count=round(1400+1600*k['complexity'])
    for particle in range(count):
        u,v=rng.uniform(-1.1,1.1),rng.uniform(-1.1,1.1)
        warm=particle%4==0
        color=(242,161,77) if warm else (58,184,213)
        for step in range(150):
            x=int(W/2+u*W*.45);y=int(H/2+v*H*.45)
            if not (1<=x<W-1 and 1<=y<H-1):break
            idx=y*W+x
            root,water,glass,spore=[p[idx] for p in planes]
            # Jev material probabilities shift stream direction and turbulence.
            du=.18*math.sin(v*8+root*4+phase)
            dv=.15*math.sin(u*7+water*3-phase)
            for cx,cy,spin in [(-.36,-.12,1),(.36,.16,-1)]:
                dx,dy=u-cx,v-cy;r2=dx*dx+dy*dy+.055
                du+=-dy/r2*spin*.16;dv+=dx/r2*spin*.16
            du+=.08*(planes[2][idx+1]-planes[2][idx-1])*W
            dv+=.08*(planes[0][idx+W]-planes[0][idx-W])*H
            norm=math.hypot(du,dv)+1e-9
            u+=du/norm*.008;v+=dv/norm*.008
            if math.hypot(u,v)<.10:break
            ink.point(u,v,.025*(.3+glass+spore+step/150),color)
    ink.stars(391,100)
    return ink


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('scene',choices=['moth','medusa','engine'])
    parser.add_argument('--phase',type=float,default=0)
    parser.add_argument('--etch',action='store_true')
    args=parser.parse_args()
    fn={'moth':moth,'medusa':medusa,'engine':engine}[args.scene]
    ink=fn(args.phase)
    number={'moth':13,'medusa':14,'engine':15}[args.scene]
    suffix='etch' if args.etch else f'p{args.phase:g}'
    title={'moth':'THE MOTH THAT EATS STARS','medusa':'ABYSSAL OBSERVATORY','engine':'THE UNCERTAINTY ENGINE'}[args.scene]
    lab.save_art(f'{number}_{args.scene}_{suffix}',title+(' / ETCHING' if args.etch else ''),
                 ink.cells(1.8 if args.scene!='engine' else 1.1,args.etch),
                 f'Jev scores + saved spatial probabilities / procedural geometry / phase {args.phase:g} / offline')
