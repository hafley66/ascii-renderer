"""Place Jev's exact palette choices into pixels. No procedural image synthesis.

`call` makes paid requests in batches of 256 pixels; `render` replays offline.
"""
import argparse
import importlib.util
import json
from pathlib import Path

from PIL import Image

ROOT=Path(__file__).resolve().parents[1]


def render(folder):
    request=json.loads((folder/'request.json').read_text())
    response=json.loads((folder/'response.json').read_text())
    w=request['state']['canvas']['width'];h=request['state']['canvas']['height']
    palette=request['state']['palette_rgb']
    assert set(response['answers'])==set(request['questions'])
    rows=[[palette[response['answers'][f'x{x}_y{y}']['choice']] for x in range(w)] for y in range(h)]
    im=Image.new('RGB',(w,h))
    im.putdata([tuple(pixel) for row in rows for pixel in row])
    im.save(folder/'pixels.png')
    im.resize((w*20,h*20),Image.Resampling.NEAREST).save(folder/'preview.png')
    (folder/'pixels.json').write_text(json.dumps(rows)+'\n')
    # One terminal cell stores two actual pixels using a half-block character.
    assert h%2==0
    ansi=[];grid=[f'{w} {h//2}']
    for y in range(0,h,2):
        line=[]
        for x in range(w):
            fg,bg=rows[y][x],rows[y+1][x]
            line.append(f'\x1b[38;2;{fg[0]};{fg[1]};{fg[2]}m\x1b[48;2;{bg[0]};{bg[1]};{bg[2]}m▀')
            grid.append(f"9600 {','.join(map(str,fg))} {','.join(map(str,bg))}")
        ansi.append(''.join(line)+'\x1b[0m')
    (folder/'pixels.ansi').write_text('\n'.join(ansi)+'\n')
    (folder/'pixels.grid').write_text('\n'.join(grid)+'\n')
    # Verify every stored PNG pixel against the corresponding unmodified answer.
    with Image.open(folder/'pixels.png') as saved:
        assert list(saved.getdata())==[tuple(pixel) for row in rows for pixel in row]
    print(f'{w*h} exact Jev pixel choices saved: {folder}/preview.png')


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action',choices=['call','render'])
    parser.add_argument('--folder',type=Path,default=ROOT/'art/jev/direct-pixels')
    args=parser.parse_args()
    if args.action=='call':
        spec=importlib.util.spec_from_file_location('jev_api',Path(__file__).with_name('22_jev_explore.py'))
        api=importlib.util.module_from_spec(spec);spec.loader.exec_module(api)
        folder=args.folder.resolve()
        if (folder/'response.json').exists():
            raise SystemExit('Saved response exists; use render to replay it')
        request=json.loads((folder/'request.json').read_text())
        entries=list(request['questions'].items())
        combined={'model':None,'answers':{},'usage':{'input_tokens':0,'output_tokens':0}}
        for start in range(0,len(entries),256):
            batch=folder/f'batch_{start//256:02}'
            payload=dict(request,questions=dict(entries[start:start+256]))
            if (batch/'request.json').exists():
                assert json.loads((batch/'request.json').read_text())==payload
            else:
                api.save_json(batch/'request.json',payload)
            if not (batch/'response.json').exists():
                api.call(batch)
            response=json.loads((batch/'response.json').read_text())
            assert set(response['answers'])==set(payload['questions'])
            if combined['model'] is not None:assert combined['model']==response['model']
            combined['model']=response['model']
            combined['answers'].update(response['answers'])
            for key in combined['usage']:combined['usage'][key]+=response['usage'][key]
        assert set(combined['answers'])==set(request['questions'])
        api.save_json(folder/'response.json',combined)
        print('Combined',len(combined['answers']),'pixels; usage',combined['usage'])
    else:
        render(args.folder)
