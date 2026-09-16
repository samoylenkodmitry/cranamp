"""A native geometry pen for Cranamp's own MCP drawing engine.
No image library, bitmap resampling or external image creation is used.
"""
import json,math,urllib.error,urllib.request
class Pen:
    def __init__(self):self.ops=[]
    def path(self,points,c,fill=True,width=1,ramp=None,axis=None):
        op=dict(op='path',x=0,y=0,points=points,color=c,fill=fill,brush_size=width)
        if ramp:op.update(ramp=ramp,ramp_axis=axis)
        self.ops.append(op)
    def line(self,points,c,width=1):self.path(points,c,False,width)
    def curve(self,start,control,end,width,c,taper=False,clean_corners=False):
        """Shared GUI/MCP native brush; control point is explicit and absolute."""
        self.ops.append(dict(op='tuft' if taper else 'curve',x=start[0],y=start[1],
                             x2=end[0],y2=end[1],control=control,brush_size=width,color=c,clean_corners=clean_corners))
    def taper(self,start,control,end,width,c):
        """A curved, pointed native brush mark, useful for fur and fine facets.
        Construction coordinates stay fractional; Studio rasterizes the closed
        outline once. No stamp stretching, filtering or opacity fringe.
        """
        if not 0 < width <= 32:raise ValueError('taper width must be 0..32 pixels')
        dx=control[0]-start[0];dy=control[1]-start[1]
        if dx==dy==0:dx=end[0]-start[0];dy=end[1]-start[1]
        length=math.hypot(dx,dy)
        if not length:return self.pixel(round(start[0]),round(start[1]),c)
        nx=-dy/length*width/2;ny=dx/length*width/2
        self.path([[start[0]+nx,start[1]+ny],
                   [control[0]+nx*.55,control[1]+ny*.55,*end],
                   [control[0]-nx*.55,control[1]-ny*.55,start[0]-nx,start[1]-ny]],c)
    def rect(self,x,y,w,h,c,ramp=None,axis=None):
        op=dict(op='rect',x=x,y=y,width=w,height=h,color=c,fill=True)
        if ramp:op.update(ramp=ramp,ramp_axis=axis)
        self.ops.append(op)
    def ellipse(self,x,y,w,h,c,fill=True,width=1):self.ops.append(dict(op='ellipse',x=x,y=y,width=w,height=h,color=c,fill=fill,brush_size=width))
    def pixel(self,x,y,c):self.rect(x,y,1,1,c)
    def round(self,x,y,w,h,r,c,ramp=None,axis=None):
        # Four cubic arcs; the native engine rounds each point once.
        x2=x+w-1;y2=y+h-1;k=.55228475*r
        self.path([[x+r,y],[x2-r,y],[x2-r+k,y,x2,y+r-k,x2,y+r],[x2,y2-r],[x2,y2-r+k,x2-r+k,y2,x2-r,y2],[x+r,y2],[x+r-k,y2,x,y2-r+k,x,y2-r],[x,y+r],[x,y+r-k,x+r-k,y,x+r,y]],c,True,ramp=ramp,axis=axis)
    def stamp(self,x,y,rows,palette):self.ops.append(dict(op='stamp',x=x,y=y,rows=rows,palette=palette))
    def sprite_cell(self,x,y,width,height,rows,palette,dx=0,dy=0,background='#ff00ff'):
        """Replace one exact native sprite cell with hand-authored pixel clusters.
        Spaces are transparent; explicit state palettes relight without scaling.
        Validate the complete mark before adding either clearing or stamp ops.
        """
        if width<=0 or height<=0 or len(rows)>height or any(len(r)>width for r in rows):
            raise ValueError('Artwork exceeds its native sprite cell')
        missing={c for r in rows for c in r if c!=' ' and c not in palette}
        if missing:raise ValueError(f'Undefined sprite colors: {sorted(missing)}')
        result=[[' ']*width for _ in range(height)]
        for yy,row in enumerate(rows):
            for xx,c in enumerate(row):
                if c==' ':continue
                if not (0<=xx+dx<width and 0<=yy+dy<height):
                    raise ValueError('State translation would clip an authored pixel')
                result[yy+dy][xx+dx]=c
        self.rect(x,y,width,height,background)
        self.stamp(x,y,[''.join(r) for r in result],{k:v for k,v in palette.items() if k!=' '})
    def commit(self,label):return call('studio_draw',{'label':label,'operations':self.ops})
JOURNAL=[]
STUDIO='http://127.0.0.1:18765/mcp'
class NoStudio(RuntimeError):
    """Nothing is listening on the Studio's port.

    Every recipe in this tree runs against a Studio that has to already be
    running, and the bare urllib failure for that is forty lines of
    ConnectionRefusedError that never name Studio, the port, or the command
    that starts one -- in the middle of a build that is halfway through a skin.
    """
def call(name,args={}):
    JOURNAL.append({'name':name,'arguments':args})
    req=urllib.request.Request(STUDIO,json.dumps({'jsonrpc':'2.0','id':1,'method':'tools/call','params':{'name':name,'arguments':args}}).encode(),{'Content-Type':'application/json'})
    try:r=json.load(urllib.request.urlopen(req,timeout=120))['result']
    except (urllib.error.URLError,ConnectionError) as why:
        raise NoStudio(f'no Skin Studio answering at {STUDIO} ({why}). '
                       f'Start one with `cargo run -- --skin-studio` and leave '
                       f'it running, then re-run this.') from None
    if r.get('isError'):raise RuntimeError(r)
    return r

def answer(name,args={}):
    """The tool's answer as data rather than as an MCP envelope.

    Every Studio tool replies with {'content':[{'type':'text','text':'<json>'}]},
    so a caller that wants a number out of it -- where the twenty-eight frames
    of a slider track live, what a sprite's variants are called -- has to reach
    through two layers and json.loads the third every time.
    """
    r=call(name,args)
    body=[c['text'] for c in r.get('content',[]) if c.get('type')=='text']
    if not body:return r
    try:return json.loads(body[0])
    except json.JSONDecodeError:return {'text':body[0]}

def ramp(stops,n=32):
    out=[]
    for i in range(n):
        t=i/(n-1)
        for j in range(len(stops)-1):
            a,ca=stops[j];b,cb=stops[j+1]
            if a<=t<=b:
                u=(t-a)/(b-a);c=bytes.fromhex(ca[1:]);d=bytes.fromhex(cb[1:]);out.append('#'+''.join(f'{round(x+(y-x)*u):02x}' for x,y in zip(c,d)));break
    return out
