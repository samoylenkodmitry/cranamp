"""The cats of Catamp Sampler, worked in wool.

There are two ways to make a cat in this skin and the size decides which.

A big cat is *embroidered*: a filled silhouette, then thirty or forty tapered
strokes laid along the form in four values. That is what long-and-short satin
stitch is, and Studio's `tuft` brush -- a curve narrowing to a single-pixel tip
-- is exactly one stitch of it. Drawing the same cat as flat shaded regions
gives a decal; drawing it as strokes gives a thing made of thread. The
construction points are fractional and the native rasteriser resolves them
once, so the same cat can be built at three sizes without a resample anywhere.

A small cat is *counted*: a hand-authored grid, one character per pixel, with
an explicit palette. Below about twenty pixels a stroke is one pixel long and
there is nothing left to taper, so the honest thing is to place every pixel --
and `Pen.sprite_cell` then checks the grid against its cell and its palette
before a single operation is emitted, which is how a row typed a pixel short
stops being a defect to hunt for later.

Eleven of the cats here are the same kitten in eleven coats. One grid and
eleven palettes is not a shortcut: a litter is one shape and many colours, and
relighting a shape by palette is the only way to change a sprite's material
without scaling it.
"""
from sampler_cloth import (CREAM, EYE, EYE_DARK, FUR, FUR_DARK, FUR_DEEP,
                           FUR_LIT, FUR_PALE, GREY, GREY_DARK, GREY_LIT, INK,
                           MADDER, MUSTARD, NOSE, NOSE_DARK, SNOW, SOOT,
                           SOOT_LIT, TEAL, darken, lighten, mix, hexcolor)


def coat(deep, dark, mid, lit, pale, *, eye=EYE, pupil=EYE_DARK, nose=NOSE,
         ink=INK, collar=MADDER):
    return {'deep': deep, 'dark': dark, 'mid': mid, 'lit': lit, 'pale': pale,
            'eye': eye, 'pupil': pupil, 'nose': nose, 'ink': ink,
            'collar': collar}


def wearing(coat_, collar):
    return dict(coat_, collar=collar)


GINGER = coat(FUR_DEEP, FUR_DARK, FUR, FUR_LIT, FUR_PALE)
TABBY = coat('#3d3b3c', GREY_DARK, GREY, GREY_LIT, '#e2e3e6', eye=MUSTARD,
             pupil='#4a3410')
SOOTY = coat('#151314', '#2e2b2c', SOOT_LIT, '#7a7371', '#a9a19d', eye=MUSTARD,
             pupil='#3a2a0a')
CREAMY = coat('#9d7f54', '#c3a377', '#e0c79b', '#f1e0bd', SNOW, eye=TEAL,
              pupil='#123a37')
TORTIE = coat('#3a2418', '#6b3d1f', '#9c5a26', '#c98a45', '#e8c48d',
              eye=MUSTARD, pupil='#42300c')
BLUE = coat('#3c4753', '#5b6b7a', '#8296a6', '#a9bccb', '#dfe8ef', eye=MUSTARD,
            pupil='#3f2f0d')
SIAMESE = coat('#4a382c', '#7d6552', '#d3c0a5', '#ecdfc7', SNOW, eye='#5fa8d8',
               pupil='#173c58')
SNOWSHOE = coat('#5b4a3a', '#8b7561', '#c3ae94', SNOW, SNOW, eye='#5fa8d8',
                pupil='#173c58')
RUSSET = coat('#4a2010', '#7d3b18', '#b05f27', '#d99050', '#f3cf9e')
SMOKE = coat('#2a2c31', '#4d5058', '#7b7f89', '#a8acb6', '#dcdee4', eye=EYE,
             pupil=EYE_DARK)
CALICO = coat('#3a3230', '#6d4a34', '#c0a08a', '#e8dccb', SNOW, eye=MUSTARD,
              pupil='#42300c')
LITTER = (GINGER, TABBY, CREAMY, SOOTY, TORTIE, BLUE, SIAMESE, RUSSET, SMOKE,
          CALICO, SNOWSHOE)


def _pal(coat_, extra=None):
    """The stamp palette for a hand-authored grid, in one coat."""
    table = {'k': coat_['ink'], 'D': coat_['deep'], 'd': coat_['dark'],
             'f': coat_['mid'], 'l': coat_['lit'], 'p': coat_['pale'],
             'e': coat_['eye'], 'E': coat_['pupil'], 'n': coat_['nose'],
             'N': NOSE_DARK, 'w': SNOW, 'c': coat_.get('collar', MADDER)}
    table.update(extra or {})
    return table


# ------------------------------------------------------------ embroidered cats


def _satin(pen, strokes, palette):
    """Lay a run of satin stitches. Each is a tapered curve, so it is wide at
    the root and one pixel at the tip -- which is a stitch, and is why the fur
    has a direction at all."""
    for (sx, sy), (cx, cy), (ex, ey), width, key in strokes:
        pen.taper((sx, sy), (cx, cy), (ex, ey), width, palette[key])
    return pen


def sitting(pen, x, y, w, h, pal, *, facing=1, tail=True, stripes=True):
    """A cat sitting upright, three-quarters on, built from fractional
    construction points so it can be worked at any size.

    The silhouette goes down first and the stitches go over it, because that is
    the order thread goes onto cloth: a ground in the darker value, then the
    light laid over it one stitch at a time. It answers with the rectangle its
    head occupies, since the face and the whiskers have to be aimed at that and
    not at the whole animal.
    """
    def p(u, v):
        return [x + (u if facing > 0 else 1 - u) * w, y + v * h]

    ink, deep, dark, mid, lit, pale = (pal['ink'], pal['deep'], pal['dark'],
                                       pal['mid'], pal['lit'], pal['pale'])
    # Ears first, so the head's own outline crosses their roots and the two
    # masses join instead of the ears sitting on the skull like leaves.
    for base, tip, inner in ((.26, .06, .20), (.74, .94, .80)):
        pen.path([p(base - .10, .20), p(tip, -.01), p(base + .12, .17)],
                 dark, True)
        pen.path([p(inner - .02, .15), p(tip + (.06 if inner < .5 else -.06), .05),
                  p(inner + .04, .14)], darken(pal['nose'], .18), True)
    # Body: a bell from the neck down, wide at the haunch.
    body = [p(.30, .26), [*p(.10, .40), *p(.06, .66)],
            [*p(.03, .90), *p(.16, 1.0)], p(.84, 1.0),
            [*p(.97, .90), *p(.94, .66)], [*p(.90, .40), *p(.70, .26)]]
    pen.path(body, dark, True)
    # Head: its own mass, or the cat is a bag with ears.
    hx, hy = round(x + w * .12), round(y + h * .015)
    hw, hh = max(3, round(w * .76)), max(3, round(h * .34))
    pen.ellipse(hx, hy, hw, hh, mid, fill=True)
    strokes = []
    # Crown: stitches radiate back from the brow.
    for i in range(9):
        t = i / 8
        strokes.append((p(.18 + t * .64, .30), p(.16 + t * .68, .16),
                        p(.22 + t * .56, .03 + abs(t - .5) * .10), 2,
                        'lit' if .15 < t < .85 else 'dark'))
    # Chest: a narrow pale bib. A wide one swallows the animal.
    for i in range(5):
        t = i / 4
        strokes.append((p(.38 + t * .24, .30), p(.36 + t * .28, .50),
                        p(.40 + t * .20, .70), 2, 'pale'))
    # Flanks: long stitches down the curve of the back, dark on the shaded side.
    for i in range(5):
        t = i / 4
        strokes.append((p(.74 - t * .04, .34), p(.92 - t * .06, .58),
                        p(.84 - t * .08, .92), 2, 'deep' if i < 2 else 'dark'))
        strokes.append((p(.26 + t * .04, .34), p(.08 + t * .06, .58),
                        p(.16 + t * .08, .92), 2, 'mid' if i < 3 else 'lit'))
    # Forelegs: two columns, the near one lighter.
    for i in range(3):
        t = i / 2
        strokes.append((p(.30 + t * .10, .66), p(.29 + t * .10, .82),
                        p(.30 + t * .10, .99), 2, 'lit'))
        strokes.append((p(.58 + t * .10, .66), p(.59 + t * .10, .82),
                        p(.60 + t * .10, .99), 2, 'mid'))
    _satin(pen, strokes, pal)
    if stripes:
        # A tabby's markings are rows of darker stitches lying across the form,
        # which is the one place where the material and the animal agree.
        for i, v in enumerate((.42, .52, .62, .72, .82)):
            pen.taper(p(.72, v), p(.86, v + .02), p(.92, v + .06),
                      2, pal['deep'])
            pen.taper(p(.28, v + .04), p(.14, v + .06), p(.08, v + .10),
                      2, pal['dark'])
        for u in (.34, .50, .66):
            pen.taper(p(u, .04), p(u, .12), p(u - .02, .20), 2, pal['dark'])
    # Outline last: a couched thread around the silhouette is what turns a
    # painted shape into an appliqué.
    pen.path(body, ink, False, 1)
    # Only the crown of the head is outlined. A closed ellipse outline drew a
    # ring right across the chest and the cat arrived wearing a collar.
    pen.path([p(.12, .30), [*p(.10, .04), *p(.50, .01)], p(.50, .01),
              [*p(.90, .04), *p(.88, .30)]], ink, False, 1)
    if tail:
        # The tail comes round the front and lies over the feet, which keeps it
        # inside the cell -- a tail swung out behind needs room the skin has
        # nowhere to spare.
        pen.taper(p(.88, .90), p(.80, 1.04), p(.30, 1.0),
                  max(3, round(w * .17)), dark)
        pen.taper(p(.86, .90), p(.78, 1.0), p(.34, .97),
                  max(1, round(w * .07)), lit)
    return hx, hy, hw, hh


def face(pen, x, y, w, h, pal, *, facing=1, closed=False):
    """The face goes on last, in flat stitches. At this size it is six marks
    and every one has to be exact: two eyes, a nose, two muzzle pads and the
    line under the chin that separates the head from the chest.

    The eyes sit nearer the middle of the skull than they feel they should. Put
    them where they look right on a drawn cat -- high, just under the ears --
    and the animal reads as a kitten with a swollen forehead.
    """
    def at(u, v):
        u = u if facing > 0 else 1 - u
        return round(x + u * w), round(y + v * h)

    eyes = [at(.26, .44), at(.72, .44)]
    if closed:
        for cx, cy in eyes:
            pen.rect(cx - 1, cy, 3, 1, pal['ink'])
            pen.pixel(cx - 2, cy - 1, pal['ink'])
            pen.pixel(cx + 2, cy - 1, pal['ink'])
    else:
        for cx, cy in eyes:
            pen.rect(cx - 1, cy - 1, 3, 3, pal['eye'])
            pen.rect(cx - 1, cy - 2, 3, 1, pal['ink'])
            pen.rect(cx, cy - 1, 1, 3, pal['pupil'])
            pen.pixel(cx - 1, cy - 1, lighten(pal['eye'], .50))
    mx, my = at(.49, .66)
    pen.rect(mx - 3, my - 1, 3, 3, pal['pale'])
    pen.rect(mx + 1, my - 1, 3, 3, pal['pale'])
    pen.rect(mx - 1, my - 2, 3, 1, pal['nose'])
    pen.pixel(mx, my - 1, NOSE_DARK)
    pen.rect(mx - 2, my + 1, 2, 1, pal['ink'])
    pen.rect(mx + 1, my + 1, 2, 1, pal['ink'])
    return pen


def whiskers(pen, x, y, w, h, color, *, facing=1, opacity=150):
    """Three each side, in one pale thread, running well clear of the head --
    a whisker that stops at the cheek is a scratch on the fur."""
    def at(u, v):
        u = u if facing > 0 else 1 - u
        return [x + u * w, y + v * h]

    for dv, reach in ((-.06, .52), (.0, .60), (.08, .50)):
        pen.line([at(.34, .64 + dv), at(.34 - reach, .50 + dv * 2.2)], color)
        pen.ops[-1].update(opacity=opacity)
        pen.line([at(.64, .64 + dv), at(.64 + reach, .50 + dv * 2.2)], color)
        pen.ops[-1].update(opacity=opacity)
    return pen


# --------------------------------------------------------------- counted cats


# The equalizer handle: 14x25, one kitten per band -- one grid, eleven coats,
# and a collar in the colour of the thread in that band's own groove.
#
# The embroidered construction was tried here first, at the size it is
# parameterised for, and it does not survive: at fourteen pixels the ears fall
# outside the cell and the face has nowhere to sit but the top row. Below about
# twenty pixels there is no stitch left to taper and the honest thing is to
# count every pixel.
KITTEN = (
    '  k        k  ',
    ' kdk      kdk ',
    ' kdfk    kfdk ',
    ' kdffkkkkffdk ',
    ' kdffffffffdk ',
    ' kdfflllllfdk ',
    ' kdfleelleefk ',
    ' kdflEelleEfk ',
    ' kdfllpnpllfk ',
    ' kdfflpppplfk ',
    ' kdffppppppfk ',
    ' kddfppppffdk ',
    '  kcccccccck  ',
    '  kdfppppfdk  ',
    ' kddfppppffdk ',
    'kddflppppplfdk',
    'kdfflpppplffdk',
    'kdfflpppplffdk',
    'kdfflpppplffdk',
    'kdfflpppplffdk',
    'kdfflpppplffdk',
    'kdfflppppplfdk',
    'kdpppffffpppdk',
    ' kkkkkkkkkkkk ',
    '              ',
)
# Held: the ears fold and the eyes shut. The silhouette is unchanged, because a
# handle whose outline moves when it is grabbed reads as a rendering fault
# rather than as a cat being squeezed.
KITTEN_HELD = (
    '              ',
    ' kdk      kdk ',
    ' kdfk    kfdk ',
    ' kdffkkkkffdk ',
    ' kdffffffffdk ',
    ' kdfflllllfdk ',
    ' kdflkkllkkfk ',
    ' kdfllllllllk ',
    ' kdfllpnpllfk ',
    ' kdfflpppplfk ',
    ' kdffppppppfk ',
    ' kddfppppffdk ',
    '  kcccccccck  ',
    '  kdfppppfdk  ',
    ' kddfppppffdk ',
    'kddflppppplfdk',
    'kdfflpppplffdk',
    'kdfflpppplffdk',
    'kdfflpppplffdk',
    'kdfflpppplffdk',
    'kdfflpppplffdk',
    'kdfflppppplfdk',
    'kdpppffffpppdk',
    ' kkkkkkkkkkkk ',
    '              ',
)
# Eleven collars, so the litter is eleven cats rather than one cat eleven
# times -- and so a band can be found by the colour of its kitten.
COLLARS = (MADDER, TEAL, MUSTARD, '#6f7a3c', '#8d5fa8', '#c0788f', '#2f6b8f',
           '#b4682f', '#4a7f52', '#a03c5a', '#3f5f9c')


# A loaf: a cat with every leg tucked under it, asleep. 26x13.
#
# The first one was worked almost entirely in the lightest value and arrived as
# a white lump: a sleeping cat is a single smooth mass and the only thing that
# gives it a back, a shoulder and a haunch is where the light stops.
LOAF = (
    '  k  k       kkkkkkk      ',
    ' kdk kdk   kkfffffffkk    ',
    ' kdffkkdfk kflllllllffk   ',
    ' kdfffffffkflllllllllffk  ',
    'kdffffffffffllllllllllffk ',
    'kdflkkllfffffllllllllllffk',
    'kdflfpnpffffflllllllllffdk',
    'kdfffpppfffffllllllllffddk',
    ' kdffpppffffffllllllffdddk',
    ' kdffppfffffffffffffdddk  ',
    '  kddfffffffffffffddddk   ',
    '   kkddffffffffffddkkk    ',
    '     kkkkkkkkkkkkkk       ',
)

# The volume and balance handle: 14x11, a cat's head. A whole cat does not fit
# in eleven rows -- the first attempt arrived as a bean with one eye in it --
# and ears are what say cat at this size.
HANDLE_HEAD = (
    ' k          k ',
    ' kdk      kdk ',
    ' kdfk    kfdk ',
    ' kdffkkkkffdk ',
    ' kdffffffffdk ',
    ' kdfleelleefk ',
    ' kdflEelleEfk ',
    ' kdfllpnpllfk ',
    ' kdfflpppflfk ',
    '  kddfffffdk  ',
    '   kkkkkkkk   ',
)
HANDLE_HEAD_HELD = (
    ' k          k ',
    ' kdk      kdk ',
    ' kdfk    kfdk ',
    ' kdffkkkkffdk ',
    ' kdffffffffdk ',
    ' kdflkkllkkfk ',
    ' kdfllllllllk ',
    ' kdfllpnpllfk ',
    ' kdfflpppflfk ',
    '  kddfffffdk  ',
    '   kkkkkkkk   ',
)

# A paw print, stitched. Five pixels is the whole of it.
PAW = (
    ' x x x ',
    'x x x x',
    ' xxxxx ',
    ' xxxxx ',
    '  xxx  ',
)

# The playlist scroll handle: 8x18. A bobbin with thread wound on it, because
# eight pixels across is not a cat, and pretending otherwise gives a smudge.
BOBBIN = (
    'wwwwwwww',
    'wWWWWWWw',
    ' tttttt ',
    ' TtTtTt ',
    ' tTtTtT ',
    ' TtTtTt ',
    ' tTtTtT ',
    ' TtTtTt ',
    ' tTtTtT ',
    ' TtTtTt ',
    ' tTtTtT ',
    ' TtTtTt ',
    ' tTtTtT ',
    ' TtTtTt ',
    ' tTtTtT ',
    ' tttttt ',
    'wWWWWWWw',
    'wwwwwwww',
)


def bobbin_palette(thread):
    return {'w': '#c8a86e', 'W': '#8d7245', 't': thread,
            'T': darken(thread, .26)}


# Every hand-authored grid, with the sprite cell it has to land in.
CELLS = (
    ('KITTEN', KITTEN, 14, 25),
    ('KITTEN_HELD', KITTEN_HELD, 14, 25),
    ('LOAF', LOAF, 26, 13),
    ('HANDLE_HEAD', HANDLE_HEAD, 14, 11),
    ('HANDLE_HEAD_HELD', HANDLE_HEAD_HELD, 14, 11),
    ('BOBBIN', BOBBIN, 8, 18),
)


def check(cells=CELLS):
    """A row typed a pixel short is invisible until it is drawn. Check the
    grids against their cells before anything reaches the sheet."""
    for name, rows, w, h in cells:
        if len(rows) > h:
            raise ValueError(f'{name}: {len(rows)} rows in a {h}-pixel cell')
        for i, row in enumerate(rows):
            if len(row) != w:
                raise ValueError(
                    f'{name} row {i}: {len(row)} characters in a {w}-pixel cell')
    return True
