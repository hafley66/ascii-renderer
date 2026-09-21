"""Fixed integer pixel indices; four probability channels mapped directly to RGBA."""
import argparse
import importlib.util
from pathlib import Path

from PIL import Image

spec=importlib.util.spec_from_file_location('experiments',Path(__file__).with_name('28_jev_experiments.py'))
experiments=importlib.util.module_from_spec(spec)
spec.loader.exec_module(experiments)

USER='ur jimmin it too much. just make the structure have the hardcoded indexes as integers, then have it say that the type there is the pixel rgba/that we map into ansi and ask it for a face'
STATE=dict(prompt='a face',width=100,height=100,
    pixel_type=dict(index='integer, 0..9999, row-major: index = y*100+x',
                    type='rgba',rgba='[red, green, blue, alpha], integer channels 0..255'),
    output_mapping='Each numeric answer p becomes round(255*p) for the requested channel. Alpha 0 is transparent, 255 opaque. RGBA is saved directly; ANSI displays RGB composited over black. Origin top-left.')


def render(offline=False):
    run=experiments.Run(0,offline,folder=experiments.pixels.ROOT/'art/jev/face-integer-rgba',
        prompt=USER,plan=dict(state=STATE,renderer='29_jev_rgba.py',batch_size=400),batch_size=400)
    rgba=[]
    for y in range(100):
        questions={}
        for x in range(100):
            index=y*100+x
            for channel,name in enumerate(('red','green','blue','alpha')):
                questions[str(index*4+channel)]=dict(type='noul',instructions=dict(
                    pixel=dict(index=index,x=x,y=y,type='rgba'),channel=name,
                    question='Should this channel be high for this pixel in the requested image? Your probability directly supplies its channel intensity.'))
        answers=run.ask(STATE,questions)
        rgba.extend(tuple(round(255*answers[str((y*100+x)*4+c)]['noul']) for c in range(4)) for x in range(100))
        if (y+1)%20==0:
            preview=Image.new('RGBA',(100,y+1));preview.putdata(rgba)
            run.save(preview,complete=y==99,completed_pixels=len(rgba))
    im=Image.new('RGBA',(100,100));im.putdata(rgba)
    ansi=[]
    for y in range(100):
        line=''
        for x in range(100):
            r,g,b,a=rgba[y*100+x]
            rgb=[round(v*a/255) for v in (r,g,b)]
            line+='\x1b[48;2;'+ ';'.join(map(str,rgb))+'m '
        ansi.append(line+'\x1b[0m')
    experiments.pixels.append(run.folder,'ansi',columns=100,rows=100,background='black',content='\n'.join(ansi))
    run.save(im,complete=True,pixels=10000,channels=40000)


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--offline',action='store_true');args=p.parse_args()
    render(args.offline)
