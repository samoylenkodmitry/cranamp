"""Read-only native artwork join audit over every playlist tile phase."""
import json
from pathlib import Path
from PIL import Image
from pixel_pen import call
from regional_patch import value
from connected_canvas import capture_canvas
ROOT=Path(__file__).resolve().parents[2]
OUT=ROOT/'target/catamp22/continuity';OUT.mkdir(parents=True,exist_ok=True)
saved=value(call('studio_status'))['view'];results=[];failures=[]
try:
 call('studio_state',{'panel':'canvas','layers':[],'paint_layer':None,'guides':False,'presentation':True,'zoom':2})
 for height in list(range(145,174))+[261,384,522]:
  for active,pressed in [(False,False),(False,True),(True,False),(True,True)]:
   call('studio_state',{'preview_playlist_height':height,'active':active,'pressed':pressed,'scroll':27 if pressed else 0})
   p=OUT/f'native-{height}-{active}-{pressed}.png'
   call('studio_render',{'zoom':1,'path':str(p)})
   im=Image.open(p).convert('RGB')
   joins=[116,232,252,232+height-38]
   bad=[(y,x,im.getpixel((x,y-1)),im.getpixel((x,y))) for y in joins for x in range(275) if im.getpixel((x,y-1))!=im.getpixel((x,y))]
   if bad:failures.append({'height':height,'active':active,'pressed':pressed,'pixels':bad[:24]})
   results.append({'height':height,'active':active,'pressed':pressed,'join_mismatches':len(bad)})
   if height in [145,384]:
    gpu=OUT/f'gpu-{height}-{active}-{pressed}.png';capture_canvas(gpu)
    g=Image.open(gpu).convert('RGB');scale=g.width//275
    assert g.size==(275*scale,(232+height)*scale)
    for y in joins:
     for yy in [y-1,y]:
      for x in range(275):
       for sy in range(scale):
        for sx in range(scale):
         assert g.getpixel((x*scale+sx,yy*scale+sy))==im.getpixel((x,yy)),('GPU/source',height,active,pressed,x,yy)
  if height in [145,173,261,384,522]:print('Checked height',height,flush=True)
 (OUT/'report.json').write_text(json.dumps({'cases':results,'failures':failures},indent=2))
 print('Join comparisons:',len(results)*4*275,'mismatches:',sum(r['join_mismatches'] for r in results),flush=True)
 if failures:print(json.dumps(failures[:4],indent=2));raise AssertionError('Artwork endpoints differ at joins')
finally:call('studio_state',saved)
