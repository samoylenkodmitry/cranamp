"""Verify all 112 live slider/pressed/on-off states in actual Cranamp GPU pixels.
Requires running Skin Studio with its native screenshot tool. Does not edit art.
"""
import argparse,json
from pathlib import Path
from PIL import Image
from pixel_pen import call
from connected_canvas import capture_canvas
ROOT=Path(__file__).resolve().parents[2]
parser=argparse.ArgumentParser();parser.add_argument('--output',type=Path,default=ROOT/'target/catamp-native-frames');args=parser.parse_args()
OUT=args.output.resolve();OUT.mkdir(parents=True,exist_ok=True)
saved=json.loads(next(c['text'] for c in call('studio_status')['content'] if c['type']=='text'))['view']
failures=[]
try:
 call('studio_state',{'presentation':True,'preview_playlist_height':145,'zoom':2})
 for active in [False,True]:
  for pressed in [False,True]:
   for frame in range(28):
    call('studio_state',{'active':active,'pressed':pressed,'eq':[frame]*11,'volume':frame,'balance':frame,'position':frame,'scroll':frame})
    # The capture endpoint waits for this exact document revision to compose.
    path=OUT/f'{"on" if active else "off"}-{"pressed" if pressed else "released"}-{frame:02}.png'
    capture_canvas(path)
    im=Image.open(path).convert('RGBA');pixels=im.load();bad=0
    for y in range(0,im.height,2):
     for x in range(0,im.width,2):
      c=pixels[x,y]
      if any(pixels[x+xx,y+yy]!=c for xx,yy in [(1,0),(0,1),(1,1)]):bad+=1
    if bad:failures.append((active,pressed,frame,bad))
   print(f'28 GPU captures: active={active}, pressed={pressed}',flush=True)
 print('112 actual GPU captures checked; nonuniform source-pixel blocks:',failures,flush=True)
finally:
 call('studio_state',saved)
assert not failures,failures
