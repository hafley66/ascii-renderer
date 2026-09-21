"""Fixed integer pixel indices; compare probability mapping and explicit RGBA choices."""
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


def render(offline=False, choices=False):
    state=dict(STATE)
    values=list(range(0,256,17))
    if choices:
        state['output_mapping']='Select an explicit integer channel value from the available options. The selected integer is written directly to that RGBA channel. Alpha 0 is transparent, 255 opaque. ANSI displays RGB composited over black. Origin top-left.'
        state['channel_values']=values
    folder=experiments.pixels.ROOT/('art/jev/face-integer-rgba-choices' if choices else 'art/jev/face-integer-rgba')
    fresh=not (folder/'io.jsonl').exists()
    batch_size=100 if choices else 400
    run=experiments.Run(0,offline,folder=folder,
        prompt=USER,plan=dict(state=state,renderer='29_jev_rgba.py',batch_size=batch_size,choices=choices),batch_size=batch_size)
    if fresh and choices:
        experiments.pixels.append(folder,'prompt',role='assistant',content='A closer implementation would have Jev select explicit channel values from enumerated options, then assemble those values at the fixed indices. The last run did not test that.')
        experiments.pixels.append(folder,'prompt',role='user',content='yes')
        experiments.pixels.append(folder,'prompt',role='assistant',content='Running the face again with fixed integer indices and explicit RGBA channel choices. Each channel will select from 0, 17, 34, …, 255; the selected values will go directly into the image.')
    rgba=[]
    for y in range(100):
        questions={}
        for x in range(100):
            index=y*100+x
            for channel,name in enumerate(('red','green','blue','alpha')):
                questions[str(index*4+channel)]=dict(type='noul',instructions=dict(
                    pixel=dict(index=index,x=x,y=y,type='rgba'),channel=name,
                    question='Should this channel be high for this pixel in the requested image? Your probability directly supplies its channel intensity.'))
                if choices:
                    q=questions[str(index*4+channel)]
                    q['type']='choice'
                    q['instructions']['question']='Select the integer value of this RGBA channel at this pixel to render the requested image.'
                    q['criteria']={str(value):dict(channel=name,value=value) for value in values}
        answers=run.ask(state,questions)
        if choices:
            rgba.extend(tuple(int(answers[str((y*100+x)*4+c)]['choice']) for c in range(4)) for x in range(100))
        else:
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
    p=argparse.ArgumentParser();p.add_argument('--offline',action='store_true');p.add_argument('--choices',action='store_true');args=p.parse_args()
    render(args.offline,args.choices)
