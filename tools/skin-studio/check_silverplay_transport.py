"""Compare every transport sprite to actual Cranamp, including keyed underlay."""
from pathlib import Path
from zipfile import ZipFile
from io import BytesIO
import json
from PIL import Image
from pixel_pen import call
ROOT=Path(__file__).resolve().parents[2];OUT=ROOT/'target/silverplay-transport-check';OUT.mkdir(exist_ok=True)
def value(r):return json.loads(next(c['text'] for c in r['content'] if c['type']=='text'))
saved=value(call('studio_status'))['view']
with ZipFile(ROOT/'assets/skins/Catamp Silverplay.wsz') as z:
    main=Image.open(BytesIO(z.read('main.bmp'))).convert('RGB')
    buttons=Image.open(BytesIO(z.read('cbuttons.bmp'))).convert('RGB')
    toggles=Image.open(BytesIO(z.read('shufrep.bmp'))).convert('RGB')
count=0
try:
    for active in [True,False]:
        for pressed in [False,True]:
            call('studio_state',{'presentation':True,'zoom':2,'active':active,'pressed':pressed,
                 'position':27 if pressed else 0,'scroll':27 if pressed else 0,
                 'eq':[27 if (i+pressed)%2 else 0 for i in range(11)]})
            path=OUT/f'{active}-{pressed}.png'
            call('studio_screenshot',{'path':str(path),'crop':[305,48,550,754]})
            im=Image.open(path).convert('RGB')
            for y in range(377):
                for x in range(275):
                    px=im.getpixel((x*2,y*2))
                    assert all(im.getpixel((x*2+dx,y*2+dy))==px for dx,dy in [(1,0),(0,1),(1,1)])
            for sx,x,y,w,h in [(0,16,88,23,18),(23,39,88,23,18),(46,62,88,23,18),
                               (69,85,88,23,18),(92,108,88,22,18),(114,136,89,22,16)]:
                sy=h if pressed else 0
                for yy in range(h):
                    for xx in range(w):
                        px=buttons.getpixel((sx+xx,sy+yy))
                        if px==(255,0,255):px=main.getpixel((x+xx,y+yy))
                        assert im.getpixel(((x+xx)*2,(y+yy)*2))==px,(path,sx,xx,yy)
                        count+=1
            toggle_y=(30 if active else 0)+(15 if pressed else 0)
            for label,sx,dx,width in [('repeat tail',0,210,28),('shuffle fish',28,164,47)]:
                for yy in range(15):
                    for xx in range(width):
                        px=toggles.getpixel((sx+xx,toggle_y+yy))
                        if px==(255,0,255):px=main.getpixel((dx+xx,89+yy))
                        assert im.getpixel(((dx+xx)*2,(89+yy)*2))==px,(path,label,xx,yy)
                        count+=1
    print(f'Four fresh GPU captures; {count} transport pixels match all six transport cells, four-state repeat/shuffle objects, and their native underlays. Native blocks intact.')
finally:call('studio_state',saved)
