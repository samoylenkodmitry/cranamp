"""Cloth, floss and felt for Catamp Sampler, as native Studio operations.

Every other Catamp is made of something hard -- glass, silver, crystal, kraft
board. This one is made of cloth, and cloth has no specular highlight at all:
there is nothing here for a light to glint off. Depth comes from three things
and only those three:

  * a seam casts one pixel of shadow, and the thread beside it catches one
    pixel of light;
  * a raised thing (a felt patch, a button, a hem) sits on its own shadow, and
    a pressed thing sits in one instead -- the shadow moves to the top-left and
    the cast shadow goes away, because the thing is now level with the cloth;
  * a run of floss is two values, not one, because the twist of the thread
    turns over along its length.

The second rule is the readouts. Cranamp draws the timer, the title, the
spectrum and the playlist text live, in colours from the palette files, and pale
floss on undyed linen is unreadable. So every live readout sits on a patch of
indigo-dyed cloth, which is where cream floss belongs anyway.

The third rule is the grammar: no edge in this skin is a plain line. A hem is a
running stitch, a felt patch is blanket-stitched down, a patchwork join is a
seam with topstitching either side. That is what makes eleven different objects
read as one piece of sewing.

Nothing here composes an image or needs an image library; `grain`, `opacity`
and palette ramps say all of it, so the recipe runs wherever Studio does.
"""
import random

# Undyed linen, the ground everything is sewn onto.
FLAX_DEEP = '#6f5c40'   # inside a fold, under a hem
FLAX_DARK = '#9c8763'
FLAX = '#c0a87e'
FLAX_LIT = '#d8c39a'
FLAX_PALE = '#efe1bf'
# Indigo-dyed cloth: the ground under everything Cranamp writes live.
INDIGO_DEEP = '#131a2b'
INDIGO = '#22304c'
INDIGO_LIT = '#334768'
INDIGO_PALE = '#4a608a'
# Floss. A sampler is worked in a handful of colours and no more.
CREAM = '#f6ecd2'       # the display ink
BONE = '#e3d5b4'
MADDER = '#b4432f'      # red
MADDER_DARK = '#7a2b1d'
MADDER_LIT = '#d4705a'
MUSTARD = '#d9a33c'
MUSTARD_DARK = '#9a6d20'
OLIVE = '#71803f'
OLIVE_DARK = '#4a5528'
TEAL = '#2f7d75'
TEAL_DARK = '#1d534e'
TEAL_LIT = '#55a89c'
ROSE = '#c78d84'
SLATE = '#6a6f7e'       # the felt a switch that is off is cut from
INK = '#4a3a26'         # the darkest brown floss, used as an outline
INK_SOFT = '#6b563a'
# Felt: thicker, fuzzier, and the only opaque thing in the skin.
FELT = '#2f6b64'
FELT_LIT = '#3f8981'
FELT_DARK = '#1f4a45'
FELT_WARM = '#a8503c'
FELT_WARM_LIT = '#c9705a'
FELT_WARM_DARK = '#753126'
# The cats, worked in wool.
FUR_DEEP = '#6d3d18'
FUR_DARK = '#a4602a'
FUR = '#d08a3f'
FUR_LIT = '#e8b06a'
FUR_PALE = '#f7e0bb'
NOSE = '#d38c86'
NOSE_DARK = '#a45f5c'
EYE = '#8cc84f'
EYE_DARK = '#3f6f27'
GREY = '#8a8c92'
GREY_DARK = '#5a5c63'
GREY_LIT = '#b9bbc0'
SOOT = '#2e2b2c'
SOOT_LIT = '#55504f'
SNOW = '#f2eee4'
KEY = '#ff00ff'         # the classic transparency key


def rgb(value):
    return tuple(int(value[i:i + 2], 16) for i in (1, 3, 5))


def hexcolor(channels):
    return '#%02x%02x%02x' % tuple(max(0, min(255, int(round(c)))) for c in channels)


def mix(a, b, t):
    a, b = rgb(a) if isinstance(a, str) else a, rgb(b) if isinstance(b, str) else b
    return tuple(a[i] + (b[i] - a[i]) * t for i in range(3))


def shade(color, t):
    """Toward the deepest fold, or toward the palest bleached thread."""
    return hexcolor(mix(color, FLAX_DEEP if t > 0 else FLAX_PALE, abs(t)))


def darken(color, t):
    return hexcolor(mix(color, '#000000', t))


def lighten(color, t):
    return hexcolor(mix(color, '#ffffff', t))


def ramp_between(a, b, steps=24):
    """Exact palette stops from a to b. Studio picks the nearest stop rather
    than interpolating, so a two-colour ramp is a hard split, not a gradient."""
    return [hexcolor(mix(a, b, i / (steps - 1))) for i in range(steps)]


# --------------------------------------------------------------------- ground


def linen(pen, x, y, w, h, *, seed=0, top=FLAX_LIT, bottom=FLAX_DARK, grain=8,
          slubs=None, base=None):
    """A field of woven linen: a shallow gradient, the tooth of the weave, and
    the occasional slub -- the thicker length of thread that makes handwoven
    cloth handwoven. A flat rectangle at this size reads as painted card.
    """
    pen.rect(x, y, w, h, base or top, ramp=ramp_between(top, bottom),
             axis=[x, y, x, y + h - 1])
    pen.ops[-1].update(grain=grain, grain_seed=seed)
    rnd = random.Random(seed)
    for _ in range(slubs if slubs is not None else max(1, w * h // 900)):
        sx, sy = x + rnd.randrange(w), y + rnd.randrange(h)
        run = min(rnd.randint(3, 8), x + w - sx)
        pen.rect(sx, sy, run, 1, FLAX_PALE if rnd.random() < .6 else FLAX_DEEP)
        pen.ops[-1].update(opacity=52)
    return pen


def dyed(pen, x, y, w, h, *, seed=0, top=INDIGO_LIT, bottom=INDIGO_DEEP,
         grain=6):
    """Indigo-dyed cloth. Dye takes unevenly, so the grain is the point."""
    pen.rect(x, y, w, h, top, ramp=ramp_between(top, bottom),
             axis=[x, y, x, y + h - 1])
    pen.ops[-1].update(grain=grain, grain_seed=seed)
    return pen


def felt(pen, x, y, w, h, color, *, seed=0, grain=5, fuzz=True):
    """Felt: no weave, a matte body, and an edge that is never quite a line."""
    pen.rect(x, y, w, h, lighten(color, .10),
             ramp=ramp_between(lighten(color, .12), darken(color, .18)),
             axis=[x, y, x, y + h - 1])
    pen.ops[-1].update(grain=grain, grain_seed=seed)
    if fuzz:
        rnd = random.Random(seed * 7 + 1)
        for i in range(h):
            if rnd.random() < .35:
                pen.pixel(x - 1, y + i, lighten(color, .18))
                pen.ops[-1].update(opacity=90)
            if rnd.random() < .35:
                pen.pixel(x + w, y + i, darken(color, .10))
                pen.ops[-1].update(opacity=90)
    return pen


# -------------------------------------------------------------------- stitches


def running(pen, x, y, length, color, *, axis='x', dash=2, gap=2, phase=0):
    """A running stitch: the thread goes over and under, so it is dashes.

    The phase is taken from the absolute coordinate, not from the start of the
    run, so two lines drawn separately still line up -- and a tile that repeats
    meets itself when its period divides the tile.
    """
    i = -(phase % (dash + gap))
    while i < length:
        start = max(i, 0)
        run = min(i + dash, length) - start
        if run > 0:
            if axis == 'x':
                pen.rect(x + start, y, run, 1, color)
            else:
                pen.rect(x, y + start, 1, run, color)
        i += dash + gap
    return pen


CROSS = ('x x', ' x ', 'x x')
CROSS_BIG = ('x   x', ' x x ', '  x  ', ' x x ', 'x   x')


def cross(pen, x, y, color, *, big=False):
    """One cross stitch. Three pixels is the smallest mark that still reads as
    an X rather than as a speck."""
    pen.stamp(x, y, list(CROSS_BIG if big else CROSS), {'x': color})
    return pen


def blanket(pen, x, y, w, h, color, *, pitch=3, tooth=2, sides='tblr'):
    """Blanket stitch: the thread runs along the edge of the felt and loops
    back into it at a fixed pitch. This is what holds an appliqué down, and it
    is the only thing that makes a felt patch look sewn rather than glued."""
    if 't' in sides:
        pen.rect(x, y, w, 1, color)
        for i in range(0, w, pitch):
            pen.rect(x + i, y, 1, tooth, color)
    if 'b' in sides:
        pen.rect(x, y + h - 1, w, 1, color)
        for i in range(0, w, pitch):
            pen.rect(x + i, y + h - tooth, 1, tooth, color)
    if 'l' in sides:
        pen.rect(x, y, 1, h, color)
        for i in range(0, h, pitch):
            pen.rect(x, y + i, tooth, 1, color)
    if 'r' in sides:
        pen.rect(x + w - 1, y, 1, h, color)
        for i in range(0, h, pitch):
            pen.rect(x + w - tooth, y + i, tooth, 1, color)
    return pen


# ------------------------------------------------------------ made-up objects


def hem(pen, x, y, w, h, *, thread=INK, seam=FLAX_DEEP, sides='tblr', inset=2,
        dash=2, gap=2):
    """A turned edge: the cloth doubles back, so there is a raised band with a
    shadow on its inner side and a running stitch holding it down."""
    if 't' in sides:
        pen.rect(x, y, w, 2, FLAX_LIT)
        pen.rect(x, y + 2, w, 1, seam)
        running(pen, x, y + inset - 1, w, thread, dash=dash, gap=gap, phase=x)
    if 'b' in sides:
        pen.rect(x, y + h - 2, w, 2, FLAX_LIT)
        pen.rect(x, y + h - 3, w, 1, seam)
        running(pen, x, y + h - inset, w, thread, dash=dash, gap=gap, phase=x)
    if 'l' in sides:
        pen.rect(x, y, 2, h, FLAX_LIT)
        pen.rect(x + 2, y, 1, h, seam)
        running(pen, x + inset - 1, y, h, thread, axis='y', dash=dash, gap=gap,
                phase=y)
    if 'r' in sides:
        pen.rect(x + w - 2, y, 2, h, FLAX_LIT)
        pen.rect(x + w - 3, y, 1, h, seam)
        running(pen, x + w - inset, y, h, thread, axis='y', dash=dash, gap=gap,
                phase=y)
    return pen


def patch(pen, x, y, w, h, *, face=FELT, held=False, thread=BONE, seed=0,
          ground=None, cast=True):
    """A felt patch appliquéd onto the cloth: the skin's one button shape.

    Released it stands proud of the ground, on a cast shadow, lit from the
    top-left. Held it is pressed flat into the cloth, so the shadow moves
    inside the top-left edge and the cast shadow goes away -- a pressed thing
    cannot cast one. Nothing moves; if the art shifted a pixel the label would
    shiver, and a stitched label that shivers reads as a rendering bug.
    """
    if cast and not held and ground is not None:
        pen.rect(x + 1, y + h, w, 1, shade(ground, .40))
        pen.rect(x + w, y + 1, 1, h, shade(ground, .40))
    # A pressed patch is level with the cloth, so it is in the cloth's own
    # shadow: it goes darker all over, not just on one edge. A flipped bevel
    # alone is two pixels of difference and reads as nothing at all.
    felt(pen, x, y, w, h, darken(face, .32) if held else face, seed=seed)
    if held:
        pen.rect(x, y, w, 2, darken(face, .52))
        pen.rect(x, y, 2, h, darken(face, .46))
        pen.rect(x, y + h - 1, w, 1, lighten(darken(face, .32), .14))
        pen.rect(x + w - 1, y, 1, h, lighten(darken(face, .32), .08))
    else:
        pen.rect(x, y, w, 1, lighten(face, .22))
        pen.rect(x, y, 1, h, lighten(face, .14))
        pen.rect(x, y + h - 1, w, 1, darken(face, .30))
        pen.rect(x + w - 1, y, 1, h, darken(face, .24))
    blanket(pen, x, y, w, h, darken(thread, .34) if held else thread,
            pitch=4, tooth=2)
    return pen


def sewn_button(pen, x, y, size, face, *, thread=None, held=False, hole=None):
    """A button sewn on through two holes.

    Nine pixels across is enough for a rim, two holes and the thread between
    them, and not enough for four holes. The thread has to be a colour the
    button is not: holes and thread in the same dark value merge into one dash
    and the button arrives with a slot in it, like a screw.
    """
    from_face = darken(face, .55)
    hole = hole or from_face
    thread = thread or MADDER
    pen.ellipse(x, y, size, size, face if not held else darken(face, .10),
                fill=True)
    # A button pressed into cloth takes its shadow on the side the light comes
    # from, which is the whole of what says pressed at this size.
    near, far = ((darken(face, .40), lighten(face, .22)) if held
                 else (lighten(face, .34), darken(face, .38)))
    pen.path([[x, y + size - 3], [x, y, x + size - 3, y]], near, False, 1)
    pen.path([[x + size - 1, y + 2], [x + size - 1, y + size - 1,
                                      x + 2, y + size - 1]], far, False, 1)
    cx, cy = x + size // 2, y + size // 2
    pen.rect(cx - 1, cy, 3, 1, thread)
    pen.pixel(cx - 2, cy, hole)
    pen.pixel(cx + 2, cy, hole)
    pen.pixel(cx - 2, cy - 1, lighten(face, .20))
    pen.pixel(cx + 2, cy - 1, lighten(face, .20))
    return pen


def ribbon(pen, x, y, w, h, color, *, axis='x', seed=0, selvedge=None):
    """Woven ribbon: a body, two tight selvedge edges, and the weft showing as
    a fine tick across the run."""
    selvedge = selvedge or darken(color, .30)
    if axis == 'x':
        pen.rect(x, y, w, h, color,
                 ramp=ramp_between(lighten(color, .14), darken(color, .16)),
                 axis=[x, y, x, y + h - 1])
        pen.ops[-1].update(grain=4, grain_seed=seed)
        pen.rect(x, y, w, 1, selvedge)
        pen.rect(x, y + h - 1, w, 1, selvedge)
        for i in range(0, w, 4):
            pen.rect(x + i, y + 1, 1, max(1, h - 2), lighten(color, .10))
            pen.ops[-1].update(opacity=22)
    else:
        pen.rect(x, y, w, h, color,
                 ramp=ramp_between(lighten(color, .14), darken(color, .16)),
                 axis=[x, y, x + w - 1, y])
        pen.ops[-1].update(grain=4, grain_seed=seed)
        pen.rect(x, y, 1, h, selvedge)
        pen.rect(x + w - 1, y, 1, h, selvedge)
        for i in range(0, h, 4):
            pen.rect(x + 1, y + i, max(1, w - 2), 1, lighten(color, .10))
            pen.ops[-1].update(opacity=22)
    return pen


def cord(pen, x, y, length, color, *, axis='x', period=4, phase=0):
    """A twisted cord. Two strands wound together read as a repeating diagonal
    tick, which is the only thing that separates yarn from a drawn line."""
    lit, dark = lighten(color, .28), darken(color, .30)
    for i in range(length):
        t = (i + phase) % period
        top = lit if t < period // 2 else color
        bottom = color if t < period // 2 else dark
        if axis == 'x':
            pen.pixel(x + i, y, top)
            pen.pixel(x + i, y + 1, bottom)
        else:
            pen.pixel(x, y + i, top)
            pen.pixel(x + 1, y + i, bottom)
    return pen


def yarn_ball(pen, x, y, w, h, color, *, held=False):
    """A wound ball of yarn.

    What makes a ball read as wound is a few clear wraps *curving* across it in
    two directions. Straight lines at a fixed pitch were tried first: at ten
    pixels across they all come out vertical and the ball arrives as a barrel.
    """
    body = darken(color, .12) if held else color
    lit, dark = lighten(color, .34), darken(color, .34)
    pen.ellipse(x, y, w, h, body, fill=True)
    for u, v, c in ((.22, .78, lit), (.52, 1.02, lit), (.80, .30, dark),
                    (.46, -.04, dark)):
        pen.path([[x + w * u, y],
                  [x + w * (u + v) / 2 + w * .18, y + h * .5,
                   x + w * v, y + h - 1]], c, False, 1)
    pen.ellipse(x, y, w, h, darken(color, .46), fill=False, width=1)
    if not held:
        pen.pixel(x + max(1, w // 5), y + 1, lighten(color, .55))
    return pen


# At ten pixels across there is no room to construct a ball of yarn: every
# wrap comes out either vertical or a single pixel. Counted, it works.
BALL_10 = (
    '   oooo   ',
    '  oYyydo  ',
    ' oyYyydyo ',
    'oyyYyydyyo',
    'odyyYyydyo',
    'oyddyYyydo',
    'oyyyddyYyo',
    ' oyyyddyyo',
    '  oyyyddo ',
    '   oooo   ',
)


def ball_palette(color):
    return {'o': darken(color, .50), 'y': color, 'Y': lighten(color, .40),
            'd': darken(color, .30)}


# ----------------------------------------------------------------- lettering


def stitched(pen, x, y, text, color, *, scale=1, spacing=1, shadow=None):
    """A word worked in floss. The shadow underneath is what separates thread
    lying on cloth from ink printed into it."""
    if shadow:
        pen.ops.append(dict(op='text', x=int(x) + 1, y=int(y) + 1, text=text,
                            color=shadow, scale=scale, spacing=spacing,
                            opacity=140))
    pen.ops.append(dict(op='text', x=int(x), y=int(y), text=text, color=color,
                        scale=scale, spacing=spacing))
    return pen


def text_width(text, scale=1, spacing=1):
    return max(0, len(text) * (5 * scale + spacing) - spacing)


def micro(pen, x, y, text, color, *, spacing=1, shadow=None):
    """A word in the editor's own four-by-five small-caps face.

    A classic skin has cells that need a word and have no room for one -- the
    mono lamp is 27 pixels, an equalizer band caption is 14 -- and the 5x7 face
    runs off the end of all of them. This used to be a glyph table in this file,
    copied from the recipe before it, which meant the capability existed for a
    skin with a build script and for nobody drawing by hand. It is an engine
    face now, so a `text` operation asks for it by name.
    """
    if shadow:
        pen.ops.append(dict(op='text', x=int(x) + 1, y=int(y) + 1, text=text,
                            color=shadow, face='small', spacing=spacing,
                            opacity=130))
    pen.ops.append(dict(op='text', x=int(x), y=int(y), text=text, color=color,
                        face='small', spacing=spacing))
    return pen


def micro_width(text, spacing=1):
    return max(0, len(text) * (4 + spacing) - spacing)


# The timer is the most-read number in the skin, and it gets a nine-by-thirteen
# cell. Cross stitch is the honest way to write a number on linen, and at this
# size a three-pixel cross on a two-pixel pitch fills the cell exactly: five
# stitches across by seven down, which is a legible digit with a hole in the
# middle of every stroke, the way counted work actually looks.
_DIGIT_CELLS = {
    0: ('.xx.', 'x..x', 'x..x', 'x..x', 'x..x', '.xx.'),
    1: ('..x.', '.xx.', '..x.', '..x.', '..x.', '.xxx'),
    2: ('.xx.', 'x..x', '...x', '..x.', '.x..', 'xxxx'),
    3: ('xxx.', '...x', '.xx.', '...x', 'x..x', '.xx.'),
    4: ('..x.', '.xx.', 'x.x.', 'xxxx', '..x.', '..x.'),
    5: ('xxxx', 'x...', 'xxx.', '...x', 'x..x', '.xx.'),
    6: ('.xx.', 'x...', 'xxx.', 'x..x', 'x..x', '.xx.'),
    7: ('xxxx', '...x', '..x.', '.x..', '.x..', '.x..'),
    8: ('.xx.', 'x..x', '.xx.', 'x..x', 'x..x', '.xx.'),
    9: ('.xx.', 'x..x', 'x..x', '.xxx', '...x', '.xx.'),
}


def counted_digit(pen, x, y, value, color, *, shadow=None):
    """One digit, worked as counted cross stitch in its 9x13 cell.

    Each square of the chart is a three-pixel cross on a two-pixel pitch, so
    neighbours share their arms the way counted work actually does. Four
    squares across and six down land in exactly nine by thirteen, which is why
    the chart is that shape and not the five-by-seven a printed face would be.
    """
    chart = _DIGIT_CELLS[value]
    for r, row in enumerate(chart):
        for c, ch in enumerate(row):
            if ch != 'x':
                continue
            cx, cy = x + c * 2, y + r * 2
            if shadow:
                pen.stamp(cx + 1, cy + 1, list(CROSS), {'x': shadow})
                pen.ops[-1].update(opacity=120)
            pen.stamp(cx, cy, list(CROSS), {'x': color})
    return pen
