"""Render all 168 panel/frame/pressed combinations through Skin Studio MCP.

No document pixels are changed. The user's preview state is restored afterward.
"""
from pathlib import Path
import argparse, base64, io, json, urllib.request
from PIL import Image, ImageDraw

parser=argparse.ArgumentParser()
parser.add_argument('--output',type=Path,default=Path('target/skin-studio-states'))
args=parser.parse_args();args.output.mkdir(parents=True,exist_ok=True)
def call(name,arguments=None):
    request=urllib.request.Request('http://127.0.0.1:18765/mcp',json.dumps({
        'jsonrpc':'2.0','id':1,'method':'tools/call','params':{
            'name':name,'arguments':arguments or {}}}).encode(),{'Content-Type':'application/json'})
    result=json.load(urllib.request.urlopen(request))['result']
    if result.get('isError'):raise RuntimeError(result)
    return result

saved=json.loads(next(c['text'] for c in call('studio_status')['content'] if c['type']=='text'))['view']
try:
    call('studio_state',{'presentation':False,'zoom':1})
    for panel in ['main','equalizer','playlist']:
        for pressed in [False,True]:
            montage=None
            for frame in range(28):
                call('studio_state',{'panel':panel,'layer':'auto','pressed':pressed,'active':True,
                                    'volume':frame,'balance':frame,'position':frame,'scroll':frame,
                                    'eq':[frame]*11})
                result=call('studio_render')
                im=Image.open(io.BytesIO(base64.b64decode(next(c['data'] for c in result['content'] if c['type']=='image')))).convert('RGB')
                if montage is None:
                    montage=Image.new('RGB',(7*(im.width+4),4*(im.height+14)),'#285471')
                x=(frame%7)*(im.width+4);y=(frame//7)*(im.height+14)
                montage.paste(im,(x,y+12));ImageDraw.Draw(montage).text((x+3,y),str(frame),fill='white')
            montage.save(args.output/f'{panel}-{"pressed" if pressed else "released"}-28.png')
        print(f'{panel}: 56 states rendered',flush=True)
finally:
    call('studio_state',saved)
