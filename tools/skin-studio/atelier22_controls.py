"""Pure Studio Pen recipe for Catamp 22 native controls.

Owns complete control cells and all 28 native track frames. EQ response graph,
preamp trace, static panel glass and playlist rail remain with the frame artist.
Every visible mark is a solid palette cluster; no live calls or image imports.
"""
from connected_canvas import CanvasPen, bridge
from atelier22_common import FONT

C = dict(ink='#102b40', frame='#193e55', well='#21465c', mid='#32627b',
         teal='#508ea3', cyan='#8cc5d2', ice='#c7e8e7', shine='#f1f5e6',
         rose='#b98190')
KEY = '#ff00ff'


def source(p, sheet, rect, label):
    return bridge(p.ops, [dict(sheet=sheet, rect=rect, destination=[0, 0],
                               label='Atelier 22 · '+label)])


def text(p, word, x, y, color):
    for ch in word:
        p.stamp(x, y, [r.replace('0', ' ') for r in FONT[ch]], {'1': color})
        x += 4


def crystal(w, h, held=False):
    """Oval jewel with authored shoulder runs and a broad upper-left facet.

    The short runs turn the light around the curved shoulders instead of
    tracing a rectangular metal key. Pressed glass keeps precisely its outline.
    """
    p = CanvasPen()
    p.rect(0, 0, w, h, KEY)
    shoulders = {
        18: [None, 7, 4, 3, 2, 1, 1, 0, 0, 0, 1, 1, 2, 3, 4, 6, None, None],
        16: [None, 6, 4, 2, 1, 1, 0, 0, 0, 1, 1, 2, 4, 6, None, None],
        15: [None, 6, 3, 2, 1, 0, 0, 0, 1, 1, 2, 3, 6, None, None],
        12: [None, 5, 3, 2, 1, 0, 0, 1, 2, 3, 5, None],
    }[h]
    rows = [y for y, inset in enumerate(shoulders) if inset is not None]
    bottom = rows[-1]
    for y in rows:
        inset = shoulders[y]
        p.rect(inset, y, w-2*inset, 1, C['ink'])
        if y in (rows[0], bottom):
            continue
        # Solid planes: pale face above, broad aquamarine belly, dark lower cut.
        tone = 'cyan' if y < bottom-3 else 'teal' if y < bottom-1 else 'mid'
        if held:
            tone = 'teal' if y < bottom-3 else 'mid' if y < bottom-1 else 'cyan'
        p.rect(inset+1, y, w-2*inset-2, 1, C[tone])
        if 2 <= y <= h//2:
            edge = min(w-inset-2, w//2+2-(y-2))
            if edge >= inset+1:
                p.rect(inset+1, y, edge-inset, 1, C['teal'] if held else C['ice'])
        if 3 <= y <= h//2+1:
            p.pixel(inset+1, y, C['mid'] if held else C['ice'])
        if h//2 <= y < bottom-1:
            p.pixel(w-inset-2, y, C['cyan'] if held else C['mid'])
    # Connected crest glint; the opposite lower glint appears only when held.
    crest = shoulders[2]+1
    p.rect(crest, 2, min(w//3, w-2*crest), 1, C['mid'] if held else C['shine'])
    if held:
        p.rect(shoulders[bottom-1]+2, bottom-1,
               w-2*shoulders[bottom-1]-4, 1, C['ice'])
    return p


# Pixel-authored marks share a seven-row optical height and a two-pixel stem.
ICONS = {
    'prev': ['ss    ss ', 'ss   sss ', 'ss  ssss ', 'ss sssss ',
             'ss  ssss ', 'ss   sss ', 'ss    ss '],
    'play': ['  ss     ', '  ssss   ', '  sssss  ', '  ssssss ',
             '  sssss  ', '  ssss   ', '  ss     '],
    'pause': [' ss  ss  ']*7,
    'stop': ['         ', '  sssss  ', '  sssss  ', '  sssss  ',
             '  sssss  ', '  sssss  ', '         '],
    'next': [' ss    ss', ' sss   ss', ' ssss  ss', ' sssss ss',
             ' ssss  ss', ' sss   ss', ' ss    ss'],
    'eject': ['    s    ', '   sss   ', '  sssss  ', ' sssssss ',
              '         ', ' sssssss ', ' sssssss '],
    'shuffle': ['           s   ', ' sss      sss  ', '   ss    ss ss ',
                '    ss  ss     ', '     ssss      ', '    ss  ss     ',
                '   ss    ss ss ', ' sss      sss  ', '           s   '],
    'repeat': ['         s   ', '   ssssssss  ', '  ss     sss ',
               ' ss      s   ', ' ss          ', ' ss          ',
               '  ss      ss ', '   ssssssss  ', '             '],
}


def icon(p, kind, cx, cy, color):
    rows = ICONS[kind]
    p.stamp(cx-len(rows[0])//2, cy-len(rows)//2, rows, {'s': color})


# Footer artist uses the same plate and mark vocabulary at assembled positions.
label = text
symbol = icon


def indicator(p, x, y, on, held=False):
    p.rect(x, y, 3, 4, C['ink'])
    p.rect(x+1, y+1, 1, 2, C['rose'] if on else C['mid'])
    p.pixel(x+1, y+1, C['ice'] if on and held else C['shine'] if on else C['teal'])


def main_controls():
    out = []
    for held in (False, True):
        state = 'held' if held else 'released'
        for name, w, h in [('prev', 23, 18), ('play', 23, 18),
                           ('pause', 23, 18), ('stop', 23, 18),
                           ('next', 22, 18), ('eject', 22, 16)]:
            p = crystal(w, h, held)
            icon(p, name, w//2, h//2+int(held), C['ice'] if held else C['ink'])
            out += p.control_parts(name, state=state)
    for on in (False, True):
        for held in (False, True):
            state = ('on' if on else 'off')+('-held' if held else '')
            for name, w in [('shuffle', 47), ('repeat', 28)]:
                p = crystal(w, 15, held)
                icon(p, name, w//2-2, 7, C['ice'] if held else C['ink'])
                indicator(p, w-7, 5, on, held)
                out += p.control_parts(name, state=state)
            for i, word in enumerate(('EQ', 'PL')):
                p = crystal(23, 12, held)
                text(p, word, 5, 4, C['ice'] if held else C['ink'])
                indicator(p, 16, 4, on, held)
                out += source(p, 'shufrep.bmp',
                              [i*23+(46 if held else 0), 73 if on else 61, 23, 12],
                              word+' '+state)
    for on in (False, True):
        for word, x, w in [('STEREO', 0, 29), ('MONO', 29, 27)]:
            p = CanvasPen()
            p.rect(0, 0, w, 12, KEY)
            text(p, word, (w-(len(word)*4-1))//2, 4, C['ink'] if on else C['teal'])
            out += source(p, 'monoster.bmp', [x, 0 if on else 12, w, 12], word)
    for i, name in enumerate(('play', 'pause', 'stop')):
        p = CanvasPen()
        p.rect(0, 0, 9, 9, KEY)
        icon(p, name, 4, 4, C['ice'])
        out += source(p, 'playpaus.bmp', [i*9, 0, 9, 9], name+' indicator')
    return out


# A forked tail, raised dorsal fin, single dark eye and continuous silver belly.
FISH = [
    '                             ',
    ' ss             ss           ',
    ' sws         ssslwsss        ',
    '  sws      sslwwwwwwwsss     ',
    '   swss   slffffffffflels    ',
    '    slfsssffffffffffffflfs   ',
    '   slss   sfffffffffggffs    ',
    '  sls      sllbbbrrbbbss     ',
    ' sbs         sssssbsss       ',
    ' ss                          ',
]

SMALL_FISH = [
    '              ',
    '       ss     ',
    ' ss  sslwss   ',
    ' swssllwwwls  ',
    '  slffffffels ',
    '   sffffffflfs',
    '  slfffffrrfs ',
    ' sbssllbbbss  ',
    ' ss  sssss    ',
    '              ',
    '              ',
]

# One connected glass silhouette; four rose toes separate inside the lit face.
PAW = [
    '   sssss   ',
    '  slwwlls  ',
    ' sllrlrlls ',
    'slrlrlrlrls',
    'slrlflflrls',
    'slfflllffls',
    ' slfrrrfls ',
    ' slrrrrrls ',
    '  slrrrls  ',
    '   sblls   ',
    '    sss    ',
]


def sprite_palette(held=False):
    return dict(s=C['ink'], e=C['ink'], b=C['mid'],
                f=C['teal'] if held else C['cyan'],
                l=C['cyan'] if held else C['ice'],
                w=C['ice'] if held else C['shine'], g=C['mid'], r=C['rose'])


def rails():
    out = []
    p = CanvasPen()
    p.rect(0, 0, 248, 10, KEY)
    p.rect(3, 3, 242, 4, C['ink'])
    p.rect(1, 4, 246, 2, C['ink'])
    p.rect(3, 4, 242, 1, C['mid'])
    p.rect(3, 5, 242, 1, C['well'])
    p.rect(4, 3, 240, 1, C['teal'])
    p.rect(4, 7, 240, 1, C['mid'])
    p.rect(5, 3, 35, 1, C['ice'])
    p.rect(207, 6, 35, 1, C['cyan'])
    out += source(p, 'posbar.bmp', [0, 0, 248, 10], 'continuous seek channel')
    for held in (False, True):
        state = 'held' if held else 'released'
        p = CanvasPen()
        p.sprite_cell(0, 0, 29, 10, FISH, sprite_palette(held))
        out += p.control_parts('seek', state=state)
        for name in ('volume', 'balance'):
            p = CanvasPen()
            p.sprite_cell(0, 0, 14, 11, SMALL_FISH, sprite_palette(held))
            out += p.control_parts(name, state=state)
    for frame in range(28):
        for name, w in [('volume', 68), ('balance', 38)]:
            p = CanvasPen()
            p.rect(0, 0, w, 13, KEY)
            # Top/bottom rails run the full channel width in every native frame.
            p.rect(1, 4, w-2, 6, C['ink'])
            p.rect(2, 5, w-4, 1, C['teal'])
            p.rect(2, 6, w-4, 2, C['well'])
            p.rect(2, 8, w-4, 1, C['mid'])
            p.rect(3, 9, w-6, 1, C['teal'])
            extent = 2+round((w-5)*frame/27)
            if name == 'volume':
                p.rect(2, 6, max(1, extent-1), 1, C['cyan'])
            else:
                center = w//2
                p.rect(min(center, extent), 6, max(1, abs(extent-center)), 1, C['cyan'])
                p.pixel(center, 10, C['ice'])
            p.pixel(2, 5, C['ice'])
            out += p.control_parts(name+'_track', state='frame', frame=frame)
    return out


def eq_controls():
    out = []
    for on in (False, True):
        for held in (False, True):
            state = ('on' if on else 'off')+('-held' if held else '')
            for name, word, w in [('eq_on', 'ON', 26), ('eq_auto', 'AUTO', 32)]:
                p = crystal(w, 12, held)
                text(p, word, 5, 4, C['ice'] if held else C['ink'])
                indicator(p, w-7, 4, on, held)
                out += p.control_parts(name, state=state)
    for held in (False, True):
        state = 'held' if held else 'released'
        p = crystal(44, 12, held)
        text(p, 'PRESETS', 8, 4, C['ice'] if held else C['ink'])
        out += p.control_parts('eq_presets', state=state)
        p = CanvasPen()
        p.sprite_cell(0, 0, 11, 11, PAW, sprite_palette(held))
        out += p.control_parts('eq', state=state)
    for frame in range(28):
        p = CanvasPen()
        p.rect(0, 0, 14, 63, KEY)
        # A fixed full-height cross-section joins the common cap and sill.
        # Frame-dependent paint is confined inside the groove, away from joins.
        p.rect(5, 0, 4, 63, C['ink'])
        p.rect(4, 0, 1, 63, C['teal'])
        p.rect(5, 0, 1, 63, C['cyan'])
        p.rect(6, 0, 1, 63, C['mid'])
        p.rect(8, 0, 1, 63, C['frame'])
        p.rect(9, 0, 1, 63, C['mid'])
        pos = 5+round((27-frame)*52/27)
        if pos < 57:
            p.rect(6, pos, 1, 57-pos, C['teal'])
        out += p.control_parts('eq_track', state='frame', frame=frame)
    return out


SCROLL = [
    '  ssss  ',
    ' slwwls ',
    'slffffbs',
    'slffffbs',
    'slffffbs',
    'slffffbs',
    'slffffbs',
    'slffffbs',
    'slffffbs',
    'slffffbs',
    'slffffbs',
    'slffffbs',
    'slffffbs',
    'slffffbs',
    'slffffbs',
    ' slbbbs ',
    '  ssss  ',
    '        ',
]


def scrollbar():
    out = []
    for held in (False, True):
        p = CanvasPen()
        p.sprite_cell(0, 0, 8, 18, SCROLL, sprite_palette(held))
        # Ink moat keeps the tiny paw legible against both thumb and dark rail.
        p.stamp(1, 5, [' s s  ', 'rl lr ', 's   s ', ' srs  ',
                      'srrrs ', ' srs  ', '  s   '], sprite_palette(held))
        out += p.control_parts('scroll', state='held' if held else 'released')
    return out


def build():
    """Return complete native source patches; never inspect or mutate Studio."""
    return main_controls()+rails()+eq_controls()+scrollbar()


if __name__ == '__main__':
    parts = build()
    print(len(parts), 'parts;', sum(len(p['operations']) for p in parts), 'native marks;',
          sum(p['rect'][2]*p['rect'][3] for p in parts), 'owned atlas pixels')
