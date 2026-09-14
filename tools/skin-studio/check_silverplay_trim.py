"""Read-only live verification of Silverplay's revised title and repeat tiles."""
from pathlib import Path
from zipfile import ZipFile
from io import BytesIO
import json
from PIL import Image
from pixel_pen import call
ROOT=Path(__file__).resolve().parents[2]
OUT=ROOT/'target/silverplay-trim-check';OUT.mkdir(exist_ok=True)
def value(r):return json.loads(next(c['text'] for c in r['content'] if c['type']=='text'))
saved=value(call('studio_status'))['view']
with ZipFile(ROOT/'assets/skins/Catamp Silverplay.wsz') as z:
    sheets={n:Image.open(BytesIO(z.read(n))).convert('RGB') for n in ['titlebar.bmp','eqmain.bmp','pledit.bmp']}
count=0
try:
    for active in [True,False]:
        for pressed in [False,True]:
            call('studio_state',{'presentation':True,'zoom':2,'active':active,'pressed':pressed,
                 'volume':27 if pressed else 0,'position':27 if pressed else 0,
                 'scroll':27 if pressed else 0,'eq':[27 if (i+pressed)%2 else 0 for i in range(11)]})
            path=OUT/f'{active}-{pressed}.png'
            call('studio_screenshot',{'path':str(path),'crop':[305,48,550,754]})
            im=Image.open(path).convert('RGB')
            for y in range(377):
                for x in range(275):
                    px=im.getpixel((x*2,y*2))
                    assert all(im.getpixel((x*2+dx,y*2+dy))==px for dx,dy in [(1,0),(0,1),(1,1)])
            # New ribbon ornament, outside the native window buttons.
            for sheet,sx,sy,dx,dy,w,h in [
                ('titlebar.bmp',27+188,0 if active else 15,188,0,52,14),
                ('eqmain.bmp',10,134 if active else 149,10,116,72,14),
                ('eqmain.bmp',220,134 if active else 149,220,116,34,14),
                ('eqmain.bmp',140,134 if active else 149,140,116,77,14),
                ('eqmain.bmp',140,14,140,130,77,3),
                ('eqmain.bmp',203,17,203,133,14,13),
                ('pledit.bmp',26,21 if active else 0,87,232,100,20),
                ('pledit.bmp',0,42,0,252,12,87),
                ('pledit.bmp',45,42,269,252,6,87)]:
                for yy in range(h):
                    for xx in range(w):
                        source_y=sy+(yy%29 if h==87 else yy)
                        expected=sheets[sheet].getpixel((sx+xx,source_y))
                        assert im.getpixel(((dx+xx)*2,(dy+yy)*2))==expected,(path,sheet,xx,yy)
                        count+=1
            for x in list(range(25,87))+list(range(187,250)):
                for y in range(20):
                    expected=sheets['pledit.bmp'].getpixel((127+(x-25)%25,(21 if active else 0)+y))
                    assert im.getpixel((x*2,(232+y)*2))==expected,(path,'top repeat',x,y)
                    count+=1
    print(f'Four actual Cranamp state captures: native pixels intact; {count} title/rail pixels match the original atlas cells.')
finally:call('studio_state',saved)
