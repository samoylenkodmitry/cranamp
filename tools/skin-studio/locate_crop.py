#!/usr/bin/env python3
"""Read-only pixel-crop locator. No resizing, filtering, art writes or MCP calls.

python3 tools/skin-studio/locate_crop.py --reference 'assets/skins/Catamp Silverplay refinement 17.png' --crop '/path/crop.png'
Reference must start at canvas (0,0); default reference scale is 2. Crop scale is
inferred from pixel-block boundaries, or set --crop-scale. RGB errors tolerate
small display-profile differences. Ambiguous/poor matches are reported, never
promoted to exact coordinates. PIL is the only nonstandard dependency.
"""
import argparse
from collections import Counter
import json
import math
from pathlib import Path
from PIL import Image


def block_grid(image, max_scale=8, exact_scale=None):
    """Infer integer block size and first complete block boundary, without resize."""
    w, h = image.size
    px = image.load()
    dx = Counter(x for y in range(h) for x in range(1, w) if px[x,y] != px[x-1,y])
    dy = Counter(y for y in range(1, h) for x in range(w) if px[x,y] != px[x,y-1])
    total = sum(dx.values()) + sum(dy.values())
    if not total:
        return {'scale': 1, 'phase': [0,0], 'agreement': 0.0}
    candidates = []
    for scale in ([exact_scale] if exact_scale else range(1,max_scale+1)):
        counts = [Counter(),Counter()]
        for count, edges in zip(counts,(dx,dy)):
            for i,n in edges.items(): count[i%scale] += n
        phase = [c.most_common(1)[0][0] if c else 0 for c in counts]
        agreement = sum(c.get(p,0) for c,p in zip(counts,phase))/total
        if agreement >= .94 or exact_scale: candidates.append((scale,phase,agreement))
    scale,phase,agreement = candidates[-1]
    return {'scale': scale, 'phase': phase, 'agreement': agreement}


def _sample(image, scale, phase=(0,0)):
    return [[image.getpixel((x,y)) for x in range(phase[0],image.width,scale)]
            for y in range(phase[1],image.height,scale)]


def locate(reference, crop, reference_scale=2, crop_scale=None, padding=32, limit=5):
    """Return JSON-safe candidates in native canvas coordinates (including padding)."""
    if reference_scale < 1 or int(reference_scale) != reference_scale:
        raise ValueError('reference_scale must be a positive integer')
    if padding < 0 or limit < 1: raise ValueError('padding >= 0 and limit >= 1 required')
    reference, crop = reference.convert('RGB'), crop.convert('RGB')
    if reference.width % reference_scale or reference.height % reference_scale:
        raise ValueError('reference dimensions must be divisible by reference_scale')
    rp = reference.load()
    for y in range(reference.height):
        for x in range(reference.width):
            if rp[x,y] != rp[x//reference_scale*reference_scale,y//reference_scale*reference_scale]:
                raise ValueError('reference is not exact integer-scale pixels; specify its actual scale')
    grid = block_grid(crop)
    scale = grid['scale'] if crop_scale is None else crop_scale
    if scale < 1 or int(scale) != scale: raise ValueError('crop_scale must be a positive integer')
    if crop_scale and crop_scale != grid['scale']:
        # Determine phase for the explicitly requested grid.
        grid = block_grid(crop,exact_scale=crop_scale)
        grid['scale'] = crop_scale
        if grid['phase'][0] >= scale or grid['phase'][1] >= scale: grid['phase']=[0,0]
    phase=grid['phase']
    native = _sample(reference,reference_scale)
    templ = _sample(crop,scale,phase)
    if not templ or not templ[0]: raise ValueError('crop has no complete pixel samples')
    nh,nw=len(native),len(native[0]); th,tw=len(templ),len(templ[0])
    rw=nw+padding*2; rh=nh+padding*2
    if tw>rw or th>rh: raise ValueError('crop exceeds padded reference at chosen scale')
    # Padding is comparison-only; bounds outside the actual reference are reported.
    rows=[[(0,0,0)]*rw for _ in range(padding)]
    rows += [[(0,0,0)]*padding+r+[(0,0,0)]*padding for r in native]
    rows += [[(0,0,0)]*rw for _ in range(padding)]
    flat=[c for row in rows for c in row]
    palette=list(set(flat)); ids={c:i for i,c in enumerate(palette)}
    packed=[ids[c] for c in flat]
    count=min(64,tw*th)
    points=sorted(set(round(i*(tw*th-1)/max(1,count-1)) for i in range(count)))
    anchors=[]
    for point in points:
        y,x=divmod(point,tw); c=templ[y][x]
        costs=[sum(abs(a-b) for a,b in zip(c,r))/3 for r in palette]
        anchors.append((y*rw+x,costs))
    # Sparse candidate search, followed by all-pixel verification for the top 48.
    import heapq
    scored=[]
    for y in range(rh-th+1):
        for x in range(rw-tw+1):
            start=y*rw+x
            cost=sum(costs[packed[start+offset]] for offset,costs in anchors)/len(anchors)
            item=(-cost,x,y)
            if len(scored)<48: heapq.heappush(scored,item)
            elif cost < -scored[0][0]: heapq.heapreplace(scored,item)
    verified=[]
    for _,x,y in scored:
        error=sum(sum(abs(a-b) for a,b in zip(templ[j][i],rows[y+j][x+i]))
                  for j in range(th) for i in range(tw))/(th*tw*3)
        verified.append((error,x-padding,y-padding))
    verified.sort()
    results=[]
    for error,x,y in verified:
        left=x-phase[0]/scale;top=y-phase[1]/scale
        right=left+crop.width/scale;bottom=top+crop.height/scale
        rect=[math.floor(left),math.floor(top),math.ceil(right)-math.floor(left),math.ceil(bottom)-math.floor(top)]
        l,t=max(0,rect[0]),max(0,rect[1]);rr,bb=min(nw,rect[0]+rect[2]),min(nh,rect[1]+rect[3])
        results.append({'rect':rect,'intersection':[l,t,max(0,rr-l),max(0,bb-t)],
                        'sample_origin':[x,y],'fractional_origin':[left,top],'rgb_mae':round(error,4)})
        if len(results)>=limit:break
    best=results[0]['rgb_mae'];gap=results[1]['rgb_mae']-best if len(results)>1 else None
    flat_crop=len(set(c for row in templ for c in row))<3
    if best>20: confidence='poor'
    elif flat_crop or (gap is not None and gap<2): confidence='ambiguous'
    elif best<=12 and grid['agreement']>=.94 and (gap is None or gap>=3): confidence='high'
    else: confidence='moderate'
    return {'confidence':confidence,'grid':grid,'reference_scale':reference_scale,
            'reference_native_size':[nw,nh],'candidate_gap':None if gap is None else round(gap,4),
            'candidates':results,
            'note':'Heuristic read-only RGB match; rect covers partial edge pixels. Review ambiguous/poor matches visually. Negative coordinates denote screenshot outside skin.'}


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--reference',required=True,type=Path)
    p.add_argument('--crop',required=True,type=Path,action='append',help='Repeat for several crops')
    p.add_argument('--reference-scale',type=int,default=2)
    p.add_argument('--crop-scale',type=int)
    p.add_argument('--padding',type=int,default=32)
    args=p.parse_args()
    with Image.open(args.reference) as r:
        for path in args.crop:
            with Image.open(path) as c:
                result=locate(r,c,args.reference_scale,args.crop_scale,args.padding)
            print(json.dumps({'crop':str(path),**result}),flush=True)

if __name__=='__main__':main()
