from PIL import Image,ImageDraw,ImageFont
from pathlib import Path
import sys
font=ImageFont.truetype('/System/Library/Fonts/Menlo.ttc',13)
root=Path(sys.argv[1]) if len(sys.argv)>1 else Path('/tmp/azulejo-review')
ims=[]
for f in sorted(root.glob('*.grid')):
 lines=f.read_text().splitlines(); w,h=map(int,lines[0].split())
 if w>160:
  im=Image.new('RGB',(w,h)); pix=im.load()
  for i,line in enumerate(lines[1:]):
   c,fg,bg=line.split();rgb=lambda s:tuple(map(int,s.split(','))) if s!='x' else (0,0,0)
   pix[i%w,i//w]=rgb(bg) if int(c)==32 else tuple((a*2+b)//3 for a,b in zip(rgb(fg),rgb(bg)))
  im.resize((1000,1000)).save(root/(f.stem+'.png'));continue
 cw,ch=8,16
 im=Image.new('RGB',(w*cw,h*ch+24));d=ImageDraw.Draw(im);d.text((0,2),f.stem,font=font)
 for i,line in enumerate(lines[1:]):
  c,fg,bg=line.split();rgb=lambda s:tuple(map(int,s.split(','))) if s!='x' else (0,0,0)
  x=i%w*cw;y=i//w*ch+24;d.rectangle((x,y,x+cw-1,y+ch-1),fill=rgb(bg))
  if int(c)!=32:d.text((x,y-1),chr(int(c)),font=font,fill=rgb(fg))
 im.save(root/(f.stem+'.png'))
 if 'random' in f.stem or 'motion' in f.stem:ims.append(im.resize((400,300)))
canvas=Image.new('RGB',(1600,1800))
for i,im in enumerate(ims):canvas.paste(im,(i%4*400,i//4*300))
canvas.save(root/'gallery.png')
