"""Translate hand-authored assembled-canvas geometry into clipped atlas parts.

No MCP calls, rasterization, image imports, resizing or implicit state inference.
Each target maps destination [x,y] to the top-left of source rect [x,y,w,h].
Give every active/inactive atlas variant explicitly. Drawing remains whole until
Studio rasterizes and clips it, preserving brush rhythm and palette ramps.

    from canvas_bridge import bridge
    parts = bridge(pen.ops, [
        {'sheet': 'main.bmp', 'rect': [0,100,275,15], 'destination': [0,100]},
        {'sheet': 'eqmain.bmp', 'rect': [0,134,275,14], 'destination': [0,116]},
    ])

Pass parts to regional_patch.prepare, then validate/apply through Studio.
This does not change skin layout. Cranamp's dock edge aliases the main's final
source row; select explicit atlas regions rather than inventing an extra row.
"""
from copy import deepcopy
import math

_COMMON = {'op', 'x', 'y', 'color', 'brush_size', 'fill', 'clean_corners',
           'ramp', 'ramp_axis'}
_FIELDS = {
    'pixel': set(), 'line': {'x2', 'y2'},
    'rect': {'width', 'height'}, 'ellipse': {'width', 'height'},
    'path': {'points'},
    'curve': {'x2', 'y2', 'control', 'curve_bend'},
    'tuft': {'x2', 'y2', 'control', 'curve_bend'},
    'stamp': {'rows', 'palette'},
}


def _numbers(value, count, label, integer=False):
    if not isinstance(value, (list, tuple)) or len(value) != count:
        raise ValueError(f'{label} needs {count} coordinates')
    if any(isinstance(v, bool) or not isinstance(v, (int, float))
           or not math.isfinite(v) or (integer and not isinstance(v, int)) for v in value):
        raise ValueError(f'{label} needs finite {"integer " if integer else ""}coordinates')
    return list(value)


def translate(operation, dx, dy):
    """Return an independent translated native Pen operation; reject unknowns.

    Path command points are relative to x/y. Shift their origin exactly once;
    curve control/end coordinates and palette axes are absolute and shift too.
    Brush widths, dimensions, curve bend, colors and stamp rows never change.
    Procedural materials and mirrors are intentionally unsupported because their
    shading/mirroring reference surface can change between atlas sections.
    """
    _numbers([dx, dy], 2, 'translation', integer=True)
    op = deepcopy(operation)
    kind = op.get('op')
    if kind not in _FIELDS:
        raise ValueError(f'Unsupported native operation: {kind!r}')
    unknown = set(op) - _COMMON - _FIELDS[kind]
    if unknown:
        raise ValueError(f'Unsupported fields for {kind}: {sorted(unknown)}')
    x, y = _numbers([op.get('x', 0), op.get('y', 0)], 2, 'origin', integer=True)
    op.update(x=x+dx, y=y+dy)
    if kind in ('line', 'curve', 'tuft'):
        x2, y2 = _numbers([op.get('x2', x), op.get('y2', y)], 2, 'end', integer=True)
        op.update(x2=x2+dx, y2=y2+dy)
    if 'control' in op:
        cx, cy = _numbers(op['control'], 2, 'control')
        op['control'] = [cx+dx, cy+dy]
    if 'points' in op:
        if not isinstance(op['points'], (list, tuple)) or not op['points']:
            raise ValueError('Path needs relative commands')
        for i, command in enumerate(op['points']):
            if not isinstance(command, (list, tuple)) or len(command) not in ((2,) if i == 0 else (2,4,6)):
                raise ValueError('Path starts with [x,y], followed by 2/4/6-coordinate commands')
            _numbers(command, len(command), 'relative path command')
        # Relative command coordinates deliberately stay unchanged.
    if 'ramp_axis' in op:
        a,b,c,d = _numbers(op['ramp_axis'], 4, 'ramp_axis')
        op['ramp_axis'] = [a+dx,b+dy,c+dx,d+dy]
    return op


def bridge(operations, targets):
    """Build isolated patch parts from one assembled drawing and explicit maps.

    Target fields: sheet, rect, destination; optional label. Rect dimensions
    establish ownership, never a scale factor. No default state propagation.
    Source sheet dimensions and baseline tokens are validated by Studio.
    """
    operations = list(operations)
    if not operations:
        raise ValueError('Bridge needs native drawing operations')
    parts = []
    for target in targets:
        unknown = set(target) - {'sheet','rect','destination','label'}
        if unknown:
            raise ValueError(f'Unsupported target fields: {sorted(unknown)}')
        if not isinstance(target.get('sheet'), str) or not target['sheet']:
            raise ValueError('Target needs an atlas sheet name')
        sx,sy,w,h = _numbers(target.get('rect'),4,'source rect',integer=True)
        x,y = _numbers(target.get('destination'),2,'destination',integer=True)
        if min(sx,sy) < 0 or min(w,h) <= 0:
            raise ValueError('Source rect needs nonnegative origin and positive dimensions')
        part = {'sheet': target['sheet'], 'rect':[sx,sy,w,h], 'clip_to_rect':True,
                'operations':[translate(op,sx-x,sy-y) for op in operations]}
        if 'label' in target:
            part['label'] = target['label']
        parts.append(part)
    if not 1 <= len(parts) <= 64:
        raise ValueError('Bridge needs 1..64 explicit source targets')
    return parts


def _selftest():
    """Translation contracts; native raster identity is covered by Rust tests."""
    from pixel_pen import Pen
    p=Pen()
    p.round(5,104,30,18,4,'#ffffff',ramp=['#000000','#ffffff'],axis=[5,104,35,122])
    p.curve([5,110],[16.5,117],[35,122],2,'#abcdef')
    p.stamp(6,112,['xy'],{'x':'#ffffff','y':'#000000'})
    original=deepcopy(p.ops)
    parts=bridge(p.ops,[{'sheet':'eqmain.bmp','rect':[0,134,50,14],'destination':[0,116]}])
    path,curve,stamp=parts[0]['operations']
    assert path['points']==original[0]['points'] and path['y']==18
    assert path['ramp_axis']==[5,122,35,140]
    assert curve['y']==128 and curve['y2']==140 and curve['control']==[16.5,135]
    assert stamp['y']==130 and stamp['rows']==['xy']
    assert p.ops==original
    for op in ({'op':'cluster'}, {'op':'line','mirror_x':True}, {'op':'rect','material':'glass'}):
        try: translate(op,0,0)
        except ValueError: pass
        else: raise AssertionError(f'Unsupported geometry accepted: {op}')
    parts[0]['operations'][0]['points'][0][0] = -999
    assert p.ops==original
    print('Canvas bridge translation checks passed')


if __name__ == '__main__':
    import argparse
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--selftest',action='store_true')
    args=parser.parse_args()
    if args.selftest: _selftest()
    else: parser.print_help()
