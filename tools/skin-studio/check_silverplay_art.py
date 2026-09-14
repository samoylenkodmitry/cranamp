"""Read-only comparison of exported original sprite definitions to actual GPU captures."""
from pathlib import Path
from zipfile import ZipFile
from io import BytesIO
from PIL import Image
ROOT=Path(__file__).resolve().parents[2]
OUT=ROOT/'target/catamp-silverplay-frames'
with ZipFile(ROOT/'assets/skins/Catamp Silverplay.wsz') as z:
    names=set(z.namelist());assert len(names)==15 and len([n for n in names if n.endswith('.bmp')])==13,names
    assert not any(n in names for n in ['cranamp.json','plbg.bmp','eqhandles.bmp','plselection.bmp'])
    sheets={n:Image.open(BytesIO(z.read(n))).convert('RGB') for n in names if n.endswith('.bmp')}
key=(255,0,255)
def sample(sheet,x,y,under):
    c=sheets[sheet].getpixel((x,y));return under if c==key else c
errors=[];regions=0
for active in [False,True]:
 for pressed in [False,True]:
  for f in range(28):
   name=f'{"on" if active else "off"}-{"pressed" if pressed else "released"}-{f:02}.png'
   im=Image.open(OUT/name).convert('RGB')
   for i,x in enumerate([21,78,96,114,132,150,168,186,204,222,240]):
    ty=round(52*(1-f/27));bad=0
    for yy in range(63):
     for xx in range(14):
      c=sample('eqmain.bmp',x+xx,38+yy,key)
      c=sample('eqmain.bmp',13+(f%14)*15+xx,164+(f//14)*65+yy,c)
      if 1<=xx<12 and ty<=yy<ty+11:c=sample('eqmain.bmp',xx-1,(176 if pressed else 164)+yy-ty,c)
      if im.getpixel(((x+xx)*2,(154+yy)*2))!=c:bad+=1
    if bad:errors.append((name,'EQ',i,bad))
    regions+=1
   for sheet,x,y,w,h,sx,stride,travel,thumbw,thumby,thumby0,thumbx in [
       ('volume.bmp',107,57,68,13,0,15,54,14,58,422,0 if pressed else 15),
       ('balance.bmp',177,57,38,13,9,15,24,14,58,422,0 if pressed else 15),
       ('posbar.bmp',17,72,248,10,0,0,219,29,72,0,278 if pressed else 248)]:
    tx=round(travel*f/27);bad=0
    for yy in range(h):
     for xx in range(w):
      c=sample('main.bmp',x+xx,y+yy,key);c=sample(sheet,sx+xx,f*stride+yy,c)
      if tx<=xx<tx+thumbw and thumby<=y+yy<thumby+(10 if sheet=='posbar.bmp' else 11):c=sample(sheet,thumbx+xx-tx,thumby0+y+yy-thumby,c)
      if im.getpixel(((x+xx)*2,(y+yy)*2))!=c:bad+=1
    if bad:errors.append((name,sheet,bad))
    regions+=1
   sy=round(69*f/27);bad=0
   for yy in range(87):
    for xx in range(8):
     c=sample('pledit.bmp',36+xx,42+yy%29,key)
     if sy<=yy<sy+18:c=sample('pledit.bmp',(61 if pressed else 52)+xx,53+yy-sy,c)
     if im.getpixel(((260+xx)*2,(252+yy)*2))!=c:bad+=1
   if bad:errors.append((name,'playlist scroll',bad))
   regions+=1
print(f'{regions} complete moving-control regions checked against the original atlases. Errors: {errors[:20]}')
assert not errors,errors[:20]
