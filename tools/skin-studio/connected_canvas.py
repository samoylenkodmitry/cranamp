"""Pure native Pen geometry -> classic static surfaces, assembled coordinates.
Geometry/mapping methods are pure; prepare reads/validates, apply mutates.
No external rasterization or implicit control edits. See connected-canvas-api.md.
"""
from dataclasses import dataclass
from contextlib import contextmanager
from pathlib import Path
import base64
import json
import re
import struct
import zlib
from canvas_bridge import bridge, translate, _numbers
from pixel_pen import Pen


def _rect(r):
    x,y,w,h=_numbers(r,4,'rectangle',integer=True)
    if min(x,y)<0 or min(w,h)<=0: raise ValueError('Rectangle requires nonnegative origin and positive size')
    return [x,y,w,h]


def _intersection(a,b):
    x,y=max(a[0],b[0]),max(a[1],b[1])
    w,h=min(a[0]+a[2],b[0]+b[2])-x,min(a[1]+a[3],b[1]+b[3])-y
    return [x,y,w,h] if w>0 and h>0 else None


def _subtract(a,b):
    c=_intersection(a,b)
    if c is None:return [a]
    x,y,w,h=a;cx,cy,cw,ch=c
    return [r for r in ([x,y,w,cy-y],[x,cy+ch,w,y+h-cy-ch],
                         [x,cy,cx-x,ch],[cx+cw,cy,x+w-cx-cw,ch]) if r[2]>0 and r[3]>0]


def runtime_rects(playlist_height=145):
    """Classic live text/visualizer reservations in the assembled canvas."""
    _height(playlist_height)
    return [[48,26,9,13],[60,26,9,13],[78,26,9,13],[90,26,9,13],
            [24,43,76,16],[111,27,150,8],[111,41,18,8],[156,41,12,8],
            [86,133,113,19],[12,252,243,playlist_height-58],
            [132,232+playlist_height-28,72,8],[192,232+playlist_height-14,30,8]]


def _height(h):
    if isinstance(h,bool) or not isinstance(h,int) or not 145<=h<=522:
        raise ValueError('playlist_height must be an integer in 145..522')


def context_rect(rect, padding=4, playlist_height=145):
    """Pure native inspection bounds: expand ownership, clamp to canvas edges."""
    _height(playlist_height)
    rect=_rect(rect)
    canvas=[0,0,275,232+playlist_height]
    if _intersection(rect,canvas)!=rect:
        raise ValueError('Rectangle exceeds the native 275×(232+playlist_height) canvas')
    if isinstance(padding,bool) or not isinstance(padding,int) or padding<0:
        raise ValueError('padding must be a nonnegative integer')
    x,y,w,h=rect
    left,top=max(0,x-padding),max(0,y-padding)
    right,bottom=min(275,x+w+padding),min(canvas[3],y+h+padding)
    return [left,top,right-left,bottom-top]


def study_canvas(rect, path, zoom=4, padding=4, playlist_height=145):
    """Read-only native study with exterior halo; returns raw MCP image content.

    Requires current canvas panel and matching playlist height. Never changes
    view, selection or artwork. Writes only the requested study PNG.
    """
    bounds=context_rect(rect,padding,playlist_height)
    if isinstance(zoom,bool) or not isinstance(zoom,int) or not 1<=zoom<=8:
        raise ValueError('zoom must be an integer in 1..8')
    if not isinstance(path,(str,Path)) or not str(path).strip():
        raise ValueError('path must name a study image file')
    output=Path(path).expanduser().resolve()
    if output.is_dir():raise ValueError('path must name a study image file, not a directory')
    from pixel_pen import call
    from regional_patch import value
    view=value(call('studio_status'))['view']
    if view.get('panel')!='canvas' or view.get('preview_playlist_height')!=playlist_height:
        raise ValueError('study_canvas requires panel="canvas" and matching preview_playlist_height; view unchanged')
    return call('studio_study',{'rect':bounds,'path':str(output),'zoom':zoom,
                              'selected':False,'values':False,'grid':False,'geometry':False})


def presentation_bounds(scene_size, zoom=2, playlist_height=145):
    """Pure GPU crop bounds for Studio's centered presentation at scene pixels.

    Mirrors mod.rs presentation_zoom and pixel-snapped centering. Odd spare
    pixels go right/bottom. A too-small scene cannot show the full skin.
    """
    sw,sh=_numbers(scene_size,2,'scene size',integer=True)
    _height(playlist_height)
    if isinstance(zoom,bool) or not isinstance(zoom,int) or not 1<=zoom<=8:
        raise ValueError('zoom must be an integer in 1..8')
    scale=min(zoom,max(1,min(2,800//(232+playlist_height))))
    w,h=275*scale,(232+playlist_height)*scale
    if sw<w or sh<h:raise ValueError('Scene is too small for the full presentation canvas')
    return [(sw-w)//2,(sh-h)//2,w,h]


def _screenshot_size(result):
    """The captured size, from the text Studio returns or from the PNG header.

    Studio answers a screenshot with a path with that path and its size, and one
    without a path with the image inline. Reading the reported size is exact and
    free; the header path stays for the inline case. Never decodes or transforms
    raster pixels.
    """
    for item in result.get('content',[]):
        if item.get('type')=='text':
            try:
                size=json.loads(item['text']).get('size')
            except (KeyError,TypeError,ValueError):
                continue
            if (isinstance(size,list) and len(size)==2
                    and all(isinstance(n,int) and not isinstance(n,bool) and 0<n<2**31
                            for n in size)):
                return list(size)
    for item in result.get('content',[]):
        if item.get('type')!='image' or item.get('mimeType')!='image/png':continue
        try:
            # 44 base64 characters encode the complete 33-byte PNG/IHDR header.
            header=base64.b64decode(item['data'][:44],validate=True)
            if (len(header)!=33 or header[:16]!=b'\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR'
                or zlib.crc32(header[12:29])!=struct.unpack('>I',header[29:33])[0]):
                raise ValueError('Invalid PNG IHDR')
            size=list(struct.unpack('>II',header[16:24]))
            if not all(0<n<2**31 for n in size):raise ValueError('Invalid PNG dimensions')
            return size
        except (KeyError,TypeError,ValueError,struct.error) as e:
            raise ValueError('Screenshot has no valid PNG dimensions') from e
    raise ValueError('Screenshot has no PNG image content')


def capture_canvas(path):
    """Capture actual GPU presentation using measured scene bounds, raw MCP result.

    Requires presentation mode already enabled. Reads status and an uncropped
    screenshot, then asks Studio to crop a second screenshot into path. No view
    changes, raster editing or scaling. Keep window size fixed during the pair.
    """
    if not isinstance(path,(str,Path)) or not str(path).strip():
        raise ValueError('path must name a screenshot image file')
    output=Path(path).expanduser().resolve()
    if output.is_dir():raise ValueError('path must name a screenshot image file, not a directory')
    from pixel_pen import call
    from regional_patch import value
    view=value(call('studio_status'))['view']
    if view.get('presentation') is not True:
        raise ValueError('capture_canvas requires presentation=true; view unchanged')
    # Validate layout inputs before requesting any screenshots.
    presentation_bounds([10000,10000],view.get('zoom'),view.get('preview_playlist_height'))
    # Measure through a throwaway file rather than inline. The measuring shot is
    # the whole scene, and asking for it inline moved 420 KB of base64 to read
    # a width and a height.
    probe=output.with_name(output.name+'.measure.png')
    try:
        full=call('studio_screenshot',{'path':str(probe)})
    finally:
        probe.unlink(missing_ok=True)
    bounds=presentation_bounds(_screenshot_size(full),view['zoom'],view['preview_playlist_height'])
    result=call('studio_screenshot',{'path':str(output),'crop':bounds})
    if _screenshot_size(result)!=bounds[2:]:
        raise ValueError('Screenshot dimensions do not match requested canvas crop')
    return result


@dataclass
class SurfacePlan:
    targets: list
    skipped: list
    aliases: list
    readonly: list
    repeats: str = 'error'

    def parts(self,operations):
        """Clipped native patch parts; preserves fractional curves/palette axes."""
        missing = [p for p in self.skipped if p['label'] != 'playlist.list']
        if self.repeats == 'error' and missing:
            raise ValueError('Ownership crosses shared playlist tiles/rails: '
                             + ', '.join(p['label'] for p in missing)
                             + '. No patch produced. Use repeats="shared" for one source copy, '
                             'or explicitly repeats="skip" to omit these regions. Inspect plan.aliases.')
        if not self.targets:raise ValueError('No static surface selected; inspect plan.skipped/readonly')
        return bridge(operations,self.targets)


def surface_map(rect, *, playlist_height=145, titles='both', repeats='error',
                preserve_runtime=True, exclude=()):
    """Map static main/EQ backgrounds, title variants and playlist chrome.

    repeats='error' prevents producing partial patches across shared PL tiles/rails.
    repeats='skip' explicitly omits them (reported in skipped).
    repeats='shared' maps selected copies; rejects overlapping source ownership.
    A shared source change appears in ALL copies, including outside rect.
    Dynamic controls and classic PL list fill are never added. No WSZ extensions.
    """
    _height(playlist_height)
    rect=_rect(rect)
    if _intersection(rect,[0,0,275,232+playlist_height])!=rect:
        raise ValueError('Rectangle exceeds the native 275×(232+playlist_height) canvas')
    if titles not in ('both','active','inactive','none'):raise ValueError('titles: both/active/inactive/none')
    if repeats not in ('error','skip','shared'):raise ValueError('repeats: error/skip/shared')
    if not isinstance(preserve_runtime,bool):raise ValueError('preserve_runtime must be boolean')
    cuts=[_rect(r) for r in exclude]+(runtime_rects(playlist_height) if preserve_runtime else [])
    targets=[];skipped=[];aliases=[]

    def add(label,sheet,src,dest,shared=False):
        intersection=_intersection(rect,[*dest,*src[2:]])
        if intersection is None:return
        fragments=[intersection]
        for cut in cuts:fragments=[p for f in fragments for p in _subtract(f,cut)]
        if shared and repeats in ('error','skip'):
            skipped.extend({'label':label,'rect':f,'reason':'shared source; use repeats="shared" for one selected copy'} for f in fragments)
            return
        for f in fragments:
            x,y,w,h=f;sx,sy=src[0]+x-dest[0],src[1]+y-dest[1]
            source=[sx,sy,w,h]
            if shared:
                if any(t['sheet']==sheet and _intersection(t['rect'],source) for t in targets):
                    raise ValueError(f'{label}: multiple canvas copies overlap one atlas source; select one copy with a smaller rect/exclude mask')
                aliases.append({'label':label,'selected':f,'source':source,'sheet':sheet,
                                'effect':'Changes repeat in every instance of this source, outside the selected rectangle too'})
            targets.append({'label':label,'sheet':sheet,'rect':source,'destination':[x,y]})

    # The main dock seam repeats source row 114; it is not an independent row.
    add('main.background','main.bmp',[0,0,275,115],[0,0])
    if _intersection(rect,[0,115,275,1]):
        aliases.append({'label':'main.docking.edge','selected':_intersection(rect,[0,115,275,1]),
                        'effect':'Canvas row 115 repeats row 114; row 114 is the drawing authority'})
        if rect[1]==115:
            add('main.docking.edge','main.bmp',[0,114,275,1],[0,115])
    add('equalizer.background','eqmain.bmp',[0,0,275,116],[0,116])
    states=['active','inactive'] if titles=='both' else ([] if titles=='none' else [titles])
    for state in states:
        add(f'main.title.{state}','titlebar.bmp',[27,0 if state=='active' else 15,275,14],[0,0])
        add(f'equalizer.title.{state}','eqmain.bmp',[0,134 if state=='active' else 149,275,14],[0,116])
        ty=21 if state=='active' else 0
        add(f'playlist.top.left.{state}','pledit.bmp',[0,ty,25,20],[0,232])
        add(f'playlist.title.{state}','pledit.bmp',[26,ty,100,20],[87,232])
        add(f'playlist.top.right.{state}','pledit.bmp',[153,ty,25,20],[250,232])
        # Repeated fill underneath the fixed 100px title; only exposed pixels.
        for x in range(25,250,25):
            for f in _subtract([x,232,25,20],[87,232,100,20]):
                add(f'playlist.top.tile.{state}@{x}','pledit.bmp',[127+f[0]-x,ty,f[2],f[3]],f[:2],True)
    interior=playlist_height-58
    for y in range(0,interior,29):
        h=min(29,interior-y)
        add(f'playlist.left.rail@{y}','pledit.bmp',[0,42,12,h],[0,252+y],True)
        add(f'playlist.right.rail@{y}','pledit.bmp',[31,42,20,h],[255,252+y],True)
    add('playlist.bottom.left','pledit.bmp',[0,72,125,38],[0,232+playlist_height-38])
    add('playlist.bottom.right','pledit.bmp',[126,72,150,38],[125,232+playlist_height-38])
    list_cut=_intersection(rect,[12,252,243,interior])
    if list_cut:skipped.append({'label':'playlist.list','rect':list_cut,'reason':'Classic playlist fill/text has no static bitmap source; use PLEDIT.TXT colors'})
    if len(targets)>64:raise ValueError('Masks produce more than 64 source parts; split the drawing into smaller regions')
    return SurfacePlan(targets,skipped,aliases,[r for c in cuts if (r:=_intersection(c,rect))],repeats)


def surface_parts(operations,rect,**options):
    """Shortcut for surface_map(rect, **options).parts(operations)."""
    return surface_map(rect,**options).parts(operations)


# Source cells from src/winamp/sprites.rs. State selection is always explicit.
_CONTROL_CELLS = {
    'prev': ('cbuttons.bmp', (0,0,23,18), (0,18,23,18)),
    'play': ('cbuttons.bmp', (23,0,23,18), (23,18,23,18)),
    'pause': ('cbuttons.bmp', (46,0,23,18), (46,18,23,18)),
    'stop': ('cbuttons.bmp', (69,0,23,18), (69,18,23,18)),
    'next': ('cbuttons.bmp', (92,0,22,18), (92,18,22,18)),
    'eject': ('cbuttons.bmp', (114,0,22,16), (114,16,22,16)),
    'seek': ('posbar.bmp', (248,0,29,10), (278,0,29,10)),
    'volume': ('volume.bmp', (15,422,14,11), (0,422,14,11)),
    'balance': ('balance.bmp', (15,422,14,11), (0,422,14,11)),
    'eq': ('eqmain.bmp', (0,164,11,11), (0,176,11,11)),
    'scroll': ('pledit.bmp', (52,53,8,18), (61,53,8,18)),
    'eq_on': ('eqmain.bmp', (10,119,26,12), (128,119,26,12), (69,119,26,12), (187,119,26,12)),
    'eq_auto': ('eqmain.bmp', (36,119,32,12), (154,119,32,12), (95,119,32,12), (213,119,32,12)),
    'eq_presets': ('eqmain.bmp', (224,164,44,12), (224,176,44,12)),
    'eq_graph': ('eqmain.bmp', (0,294,113,19)),
    'eq_preamp': ('eqmain.bmp', (0,314,113,1)),
    'shuffle': ('shufrep.bmp', (28,0,47,15), (28,15,47,15), (28,30,47,15), (28,45,47,15)),
    'repeat': ('shufrep.bmp', (0,0,28,15), (0,15,28,15), (0,30,28,15), (0,45,28,15)),
}


_TRACK_CELLS = ('volume_track','balance_track','eq_track')


def control_cell(control, *, state, frame=None):
    """Pure named source target with explicit state and optional native frame.

    Tracks require state='frame', frame=0..27. Other controls reject frame.
    Metadata is independent; constructing a cell never reads or changes Studio.
    """
    if not isinstance(control,str) or control not in (*_CONTROL_CELLS,*_TRACK_CELLS):
        raise ValueError(f'Unknown control; choose {", ".join((*_CONTROL_CELLS,*_TRACK_CELLS))}')
    if control in _TRACK_CELLS:
        if state!='frame':raise ValueError(f"{control} state must be frame")
        if type(frame) is not int or not 0<=frame<=27:
            raise ValueError('Track frame must be an integer from 0 through 27')
        if control=='volume_track':sheet,rect='volume.bmp',[0,frame*15,68,13]
        elif control=='balance_track':sheet,rect='balance.bmp',[9,frame*15,38,13]
        else:sheet,rect='eqmain.bmp',[13+(frame%14)*15,164+(frame//14)*65,14,63]
        return {'label':f'control.{control}.frame.{frame}', 'sheet':sheet,
                'rect':rect, 'destination':[0,0]}
    if frame is not None:raise ValueError('frame is valid only for track controls')
    cells=_CONTROL_CELLS[control]
    states=(('normal',) if len(cells)==2 else
            ('off','off-held','on','on-held') if len(cells)==5 else ('released','held'))
    if state not in states:
        raise ValueError(f'{control} state must be {"/".join(states)}')
    return {'label':f'control.{control}.{state}', 'sheet':cells[0],
            'rect':list(cells[states.index(state)+1]), 'destination':[0,0]}


def control_parts(operations, control, *, state, frame=None, rect=None, exclude=()):
    """Map local native geometry into one exact control state, never clear it.

    rect/exclude are cell-local rectangles. Ownership must stay inside the cell.
    Complete geometry is translated intact, then clipped by Studio to ownership;
    a curve may extend outside without changing its rasterized interior pixels.
    Thumb artwork is shared by every slider position, not the track frames.
    """
    cell=control_cell(control,state=state,frame=frame)
    sx,sy,w,h=cell['rect']; bounds=[0,0,w,h]
    rect=bounds if rect is None else _rect(rect)
    if _intersection(rect,bounds)!=rect:
        raise ValueError(f'{control} rect exceeds its local {w}×{h} cell')
    fragments=[rect]
    for mask in exclude:
        cut=_rect(mask)
        if _intersection(cut,bounds)!=cut:
            raise ValueError(f'{control} exclude exceeds its local {w}×{h} cell')
        fragments=[p for f in fragments for p in _subtract(f,cut)]
        if len(fragments)>64:raise ValueError('Masks produce more than 64 source parts')
    if not fragments:raise ValueError('Control masks leave no drawable source pixels')
    return bridge(operations,[dict(cell,rect=[sx+x,sy+y,cw,ch],destination=[x,y])
                              for x,y,cw,ch in fragments])


class CanvasPen(Pen):
    """Native geometry: assembled coordinates for parts, local for control_parts."""
    def bands(self,x,y,w,h,colors,edges,*,axis='y'):
        """Fill a rectangle with explicit, solid palette bands; no interpolation.

        Edges are integer offsets from 0 through h (axis='y', top to bottom)
        or w (axis='x', left to right). Each color fills [edge[i],edge[i+1]).
        Bands must cover the rectangle exactly, with no gaps or overlaps.
        Appends native rect operations only; invalid input appends nothing.
        """
        _numbers([x,y,w,h],4,'bands rectangle',integer=True)
        if min(w,h)<=0:raise ValueError('bands needs positive width and height')
        if axis not in ('x','y'):raise ValueError('bands axis must be x or y')
        if not isinstance(colors,(list,tuple)) or not colors or any(
            not isinstance(c,str) or not re.fullmatch(r'#[0-9a-fA-F]{6}(?:[0-9a-fA-F]{2})?',c)
            for c in colors
        ):raise ValueError('bands colors must be a nonempty list/tuple of #rrggbb/#rrggbbaa strings')
        edges=_numbers(edges,len(colors)+1,'band edges',integer=True)
        extent=h if axis=='y' else w
        if edges[0]!=0 or edges[-1]!=extent or any(a>=b for a,b in zip(edges,edges[1:])):
            raise ValueError('band edges must strictly increase from 0 to the axis extent')
        motif=Pen()
        for c,a,b in zip(colors,edges,edges[1:]):
            if axis=='y':motif.rect(x,y+a,w,b-a,c)
            else:motif.rect(x+a,y,b-a,h,c)
        self.ops.extend(motif.ops)
        return self

    def round_outline(self,x,y,w,h,radius,color,width=1):
        """Append one unfilled native rounded path; same cubic arcs as round().

        Stroke width is an integer 1..32, centered on the path. Ownership masks
        clip only after native rasterization. Use for large rims instead of
        stacking filled polygons; it emits no interior clearing/filling.
        """
        _numbers([x,y,w,h,radius],5,'rounded outline')
        _numbers([width],1,'outline width',integer=True)
        if w <= 0 or h <= 0 or radius < 0 or not 1 <= width <= 32:
            raise ValueError('round_outline needs positive size, radius >= 0, width 1..32')
        motif=Pen()
        motif.round(x,y,w,h,radius,color)
        motif.ops[0].update(fill=False,brush_size=width)
        self.ops.extend(motif.ops)

    def place(self,motif,x=0,y=0,*,colors=None):
        """Append an independent native copy of Pen/operations at integer x/y.

        Exact color substitutions affect solid colors, ramp stops and stamp
        palettes. The motif is never mutated. Validate the entire copy before
        appending, so invalid geometry or palette maps leave this pen unchanged.
        """
        _numbers([x,y],2,'placement',integer=True)
        if colors is None:colors={}
        if not isinstance(colors,dict) or any(
            not isinstance(c,str) or not re.fullmatch(r'#[0-9a-fA-F]{6}(?:[0-9a-fA-F]{2})?',c)
            for pair in colors.items() for c in pair
        ):raise ValueError('colors must map exact #rrggbb/#rrggbbaa strings')
        source=motif.ops if isinstance(motif,Pen) else motif
        copied=[translate(op,x,y) for op in list(source)]
        for op in copied:
            if 'color' in op:op['color']=colors.get(op['color'],op['color'])
            if 'ramp' in op:op['ramp']=[colors.get(c,c) for c in op['ramp']]
            if 'palette' in op:op['palette']={k:colors.get(c,c) for k,c in op['palette'].items()}
        self.ops.extend(copied)
        return self

    @contextmanager
    def group(self,x=0,y=0,*,colors=None):
        """Draw in local coordinates; append once on successful context exit.

        Nestable, with the same native brush API. A failed block appends
        nothing. The yielded pen stays reusable through place(); it has no
        implicit ownership rectangle, layer or live Studio side effects.
        """
        _numbers([x,y],2,'group origin',integer=True)
        local=CanvasPen()
        yield local
        self.place(local,x,y,colors=colors)

    def parts(self,rect,**options):return surface_parts(self.ops,rect,**options)
    def plan(self,rect,**options):return surface_map(rect,**options)

    def control_parts(self,control,*,state,frame=None,rect=None,exclude=()):
        return control_parts(self.ops,control,state=state,frame=frame,rect=rect,exclude=exclude)

    def control_prepare(self,name,control,*,state,frame=None,rect=None,exclude=(),baseline=None):
        """Read and validate one named state from local geometry; never apply."""
        from regional_patch import prepare,validate
        patch=prepare(name,self.control_parts(control,state=state,frame=frame,rect=rect,exclude=exclude),baseline)
        validate(patch)
        return patch

    def prepare(self,name,rect,*,baseline=None,**options):
        """Read source baseline and validate one native plane; never apply it."""
        from regional_patch import prepare,validate
        patch=prepare(name,self.parts(rect,**options),baseline)
        validate(patch)
        return patch


# Mutating submission is deliberately explicit and separate from drawing/preparing.
from regional_patch import apply
