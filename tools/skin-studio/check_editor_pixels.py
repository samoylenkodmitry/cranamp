"""Read-only GPU regression check for editor canvas aspect ratio and sampling.
Run with the art editor open, no drawer and pan at its origin. A tall or wide
canvas must clip, never compress into the viewport. Restores the document view.
"""
import json
from pathlib import Path
from PIL import Image,ImageChops
from pixel_pen import call
ROOT=Path(__file__).resolve().parents[2];OUT=ROOT/'target/editor-pixel-checks';OUT.mkdir(exist_ok=True)
def value(r):return json.loads(next(c['text'] for c in r['content'] if c['type']=='text'))
saved=value(call('studio_status'))['view']
try:
    for zoom in [1,2,3,4,6,8]:
        call('studio_state',{'panel':'canvas','presentation':False,'guides':False,'grid':False,'zoom':zoom})
        w=min(275*zoom,900);h=min((232+saved['preview_playlist_height'])*zoom,540)
        source=OUT/f'source-{zoom}.png';gpu=OUT/f'gpu-{zoom}.png'
        call('studio_render',{'zoom':zoom,'path':str(source)})
        call('studio_screenshot',{'path':str(gpu),'crop':[230,188,w,h]})
        a=Image.open(source).convert('RGB').crop((0,0,w,h));b=Image.open(gpu).convert('RGB')
        mismatch=ImageChops.difference(a,b).getbbox()
        assert mismatch is None,(zoom,mismatch)
        print(f'{zoom}× editor: {w}×{h} visible pixels match native source exactly',flush=True)
finally:call('studio_state',saved)
