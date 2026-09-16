"""Kraft-paper materials for Catamp Cardboard, as native Studio operations.

Nothing here composes an image. The grain and the pools of light were built in
Pillow at first and stamped in through the `image` operation, which works and
meant the whole recipe needed Python and an image library -- so none of these
materials could be drawn from the editor itself, or from Android. `grain` and
`opacity` on a drawing operation, and a palette ramp, say all of it natively.

The skin is one object -- a corrugated box -- so every surface comes from the
same short palette and the same two light directions: the box front is lit from
above-left, and everything inside the box is lit only by the glow that leaks
out of it. Paper is not smooth, so a flat rectangle of kraft reads as plastic;
these helpers carry the grain, the cut fibres and the flute pattern that say
cardboard before anything is drawn on top.
"""
import random

# The box itself.
DEEP = '#4b3420'      # inside a fold, or a cut wall in shadow
DARK = '#775132'      # kraft in shadow
KRAFT = '#9a7343'     # kraft, lit normally
LIT = '#b28a57'       # kraft facing the light
PALE = '#c9a978'      # the pale flute core where paper is torn
EDGE = '#e4cea2'      # raw cut fibres catching the light
# Ink printed on the box.
INK = '#3a2a1c'
INK_SOFT = '#5a4128'
STAMP = '#a8402f'     # rubber-stamp red
STAMP_DIM = '#7c3325'
PENCIL = '#5f5b50'
# Inside the box.
HOLE = '#241811'
HOLE_DEEP = '#130c07'
HOLE_SOFT = '#4a3826'  # a silhouette seen in the gloom
GLOW = '#ffb54a'
GLOW_HOT = '#ffe6ae'
GLOW_DIM = '#8d5b24'
# Packing tape.
TAPE = '#dbc9a1'
TAPE_LIT = '#eee0bc'
TAPE_DARK = '#b8a279'
TAPE_EDGE = '#9b865b'
# The cat.
FUR = '#d78a3c'
FUR_LIT = '#efb972'
FUR_PALE = '#fbe0b4'
FUR_DARK = '#a05b22'
FUR_SHADE = '#6d3b15'
PAD = '#d98d86'
PAD_DARK = '#a45f5c'
EYE = '#a8e05f'
EYE_DIM = '#5f8a34'
KEY = '#ff00ff'       # the classic transparency key


def rgb(value):
    return tuple(int(value[i:i + 2], 16) for i in (1, 3, 5))


def hexcolor(channels):
    return '#%02x%02x%02x' % tuple(max(0, min(255, int(round(c)))) for c in channels)


def mix(a, b, t):
    a, b = rgb(a) if isinstance(a, str) else a, rgb(b) if isinstance(b, str) else b
    return tuple(a[i] + (b[i] - a[i]) * t for i in range(3))


def shade(color, t):
    """Move a colour toward the deepest kraft shadow, or toward the cut fibre."""
    return hexcolor(mix(color, DEEP if t > 0 else EDGE, abs(t)))


def ramp_between(a, b, steps=24):
    """Exact palette stops from a to b. A two-colour ramp is a hard split --
    Studio picks the nearest stop, it does not interpolate."""
    return [hexcolor(mix(a, b, i / (steps - 1))) for i in range(steps)]


def kraft(pen, x, y, w, h, *, seed=0, top=LIT, bottom=DARK, grain=9, fibres=None,
          flecks=None):
    """A kraft field: a shallow light gradient, coarse pulp, short fibres.

    One native rectangle carries both the gradient and the grain; the fibres
    and flecks are short seeded strokes, which is what they are. The grain
    clumps on a two-pixel lattice inside Studio, because per-pixel noise at
    this size is invisible and read as suede.
    """
    pen.rect(x, y, w, h, top, ramp=ramp_between(top, bottom),
             axis=[x, y, x, y + h - 1])
    pen.ops[-1].update(grain=grain, grain_seed=seed)
    rnd = random.Random(seed)
    for _ in range(fibres if fibres is not None else max(3, w * h // 700)):
        fx, fy = x + rnd.randrange(w), y + rnd.randrange(h)
        length = min(rnd.randint(2, 5), x + w - fx)
        pale = rnd.random() < .5
        pen.rect(fx, fy, length, 1, PALE if pale else DEEP)
        pen.ops[-1].update(opacity=60 if pale else 52)
    for _ in range(flecks if flecks is not None else max(2, w * h // 900)):
        pen.pixel(x + rnd.randrange(w), y + rnd.randrange(h),
                  rnd.choice((DEEP, INK_SOFT, PALE)))
        pen.ops[-1].update(opacity=115)
    return pen


def glow(pen, x, y, w, h, *, color=GLOW, peak=36, rings=4):
    """A pool of light added to an interior that is already painted.

    Concentric filled ellipses, each baked over what is under it at a fraction
    of the strength, which is the same blend the image operation does and needs
    no image. For light that runs the height of an opening, put it in the
    opening's own ramp instead -- one operation, and the grain survives.
    """
    for i in range(rings, 0, -1):
        t = i / rings
        pen.ellipse(round(x + (w * (1 - t)) / 2), round(y + (h * (1 - t)) / 2),
                    max(1, round(w * t)), max(1, round(h * t)), color, fill=True)
        pen.ops[-1].update(opacity=max(1, round(peak / rings)))
    return pen


def flutes(pen, x, y, w, h, *, axis='x', phase=0, period=3,
           colors=(PALE, KRAFT, DEEP)):
    """The fluted core, exposed where the paper is cut through.

    The pattern is constant along the run, so a rail that repeats every 29 rows
    meets itself exactly; the phase is taken from absolute coordinates, not from
    the start of the patch, for the same reason.
    """
    if axis == 'x':
        for i in range(w):
            pen.rect(x + i, y, 1, h, colors[(x + i + phase) % period])
    else:
        for i in range(h):
            pen.rect(x, y + i, w, 1, colors[(y + i + phase) % period])
    return pen


def cut_hole(pen, x, y, w, h, *, floor=None, light=.16):
    """A window cut into the box front.

    The inside of the box is lit only by what leaks out of it, so the interior
    is one ramp: shadow down the top wall, and the glow pooled along the floor.
    That used to be a dark fill with a separate half-transparent stamp over it,
    which cost an image library and lost the grain it covered.
    """
    lit = hexcolor(mix(HOLE, GLOW, light))
    pen.rect(x, y, w, h, HOLE,
             ramp=[HOLE_DEEP, HOLE_DEEP] + ramp_between(HOLE_DEEP, HOLE, 6)
                  + ramp_between(HOLE, lit, 10),
             axis=[x, y, x, y + h - 1])
    pen.ops[-1].update(grain=4, grain_seed=x * 31 + y)
    pen.rect(x, y, w, 2, HOLE_DEEP)
    pen.rect(x, y, 1, h, '#1b120c')
    pen.rect(x + w - 1, y, 1, h, shade(HOLE, -.08))
    pen.rect(x, y + h - 1, w, 1, floor or GLOW_DIM)
    # The cut itself, one pixel outside the opening on every side.
    pen.rect(x - 1, y - 1, w + 2, 1, EDGE)
    pen.rect(x - 1, y + h, w + 2, 1, shade(PALE, .25))
    pen.rect(x - 1, y - 1, 1, h + 2, shade(EDGE, .12))
    pen.rect(x + w, y - 1, 1, h + 2, shade(PALE, .3))
    return pen


def tape(pen, x, y, w, h, *, axis='x', torn=True, seed=3):
    """A strip of packing tape: a bright core, dull edges, and the fine
    lengthwise scratches that catch light on real tape."""
    rnd = random.Random(seed)
    if axis == 'x':
        pen.rect(x, y, w, h, TAPE)
        pen.rect(x, y, w, 1, TAPE_LIT)
        pen.rect(x, y + 1, w, 1, shade(TAPE_LIT, .1))
        pen.rect(x, y + h - 1, w, 1, TAPE_EDGE)
        pen.rect(x, y + h - 2, w, 1, TAPE_DARK)
        for _ in range(max(2, w // 12)):
            sy = y + rnd.randrange(1, max(2, h - 1))
            sx = x + rnd.randrange(w)
            pen.rect(sx, sy, min(rnd.randint(3, 14), x + w - sx), 1,
                     rnd.choice((TAPE_LIT, TAPE_DARK)))
        if torn:
            for end, step in ((x, 1), (x + w - 1, -1)):
                for i in range(h):
                    if rnd.random() < .5:
                        pen.rect(end, y + i, 1, 1, TAPE_EDGE)
                    if rnd.random() < .3:
                        pen.rect(end + step, y + i, 1, 1, shade(TAPE_DARK, .2))
    else:
        pen.rect(x, y, w, h, TAPE)
        pen.rect(x, y, 1, h, TAPE_LIT)
        pen.rect(x + w - 1, y, 1, h, TAPE_EDGE)
        pen.rect(x + w - 2, y, 1, h, TAPE_DARK)
        for _ in range(max(2, h // 12)):
            sx = x + rnd.randrange(1, max(2, w - 1))
            sy = y + rnd.randrange(h)
            pen.rect(sx, sy, 1, min(rnd.randint(3, 14), y + h - sy), 
                     rnd.choice((TAPE_LIT, TAPE_DARK)))
    return pen


def stamped(pen, x, y, text, color=STAMP, scale=1, spacing=1, paper=None,
            worn=0.16, seed=5):
    """Set a word the way a rubber stamp puts it on a box: solid ink with some
    of it missing. A clean label on kraft reads as vinyl, not print.

    Wear is painted in the paper's own colour rather than the transparency key,
    because a sheet has nothing behind it -- keying the gaps would punch holes
    straight through the window.
    """
    pen.ops.append(dict(op='text', x=int(x), y=int(y), text=text, color=color,
                        scale=scale, spacing=spacing))
    if worn and paper:
        rnd = random.Random(seed)
        width = len(text) * (5 * scale + spacing)
        for _ in range(int(width * 7 * scale * worn / 6)):
            pen.rect(x + rnd.randrange(max(1, width)),
                     y + rnd.randrange(7 * scale), 1, 1, paper)
    return pen


def text_width(text, scale=1, spacing=1):
    """How wide a word comes out in the 5x7 face.

    The pen advances (cell + spacing) * scale, not cell * scale + spacing:
    the two agree at scale 1 and nowhere else, and this had the second form
    in every recipe until the engine was taught to answer the question itself.
    `studio_canvas {"measure": [...]}` is the authority; this is the same
    arithmetic so a recipe can lay a caption out before the editor is running.
    """
    return max(0, len(text) * (5 + spacing) * scale - spacing * scale)


# A four-by-five face, for the two places a classic skin has a word and no room
# for one: the mono and stereo lamps are 27 and 29 pixels wide, and the 5x7
# face spills STEREO straight off the end of its cell.
MICRO = {
    'M': ('x  x', 'xxxx', 'x  x', 'x  x', 'x  x'),
    'O': (' xx ', 'x  x', 'x  x', 'x  x', ' xx '),
    'N': ('x  x', 'xx x', 'x xx', 'x  x', 'x  x'),
    'S': (' xxx', 'x   ', ' xx ', '   x', 'xxx '),
    'T': ('xxxx', ' x  ', ' x  ', ' x  ', ' x  '),
    'E': ('xxxx', 'x   ', 'xxx ', 'x   ', 'xxxx'),
    'R': ('xxx ', 'x  x', 'xxx ', 'x x ', 'x  x'),
}


def micro_width(text, spacing=1, scale=1):
    """The same, for the 4x5 small-caps face."""
    return max(0, len(text) * (4 + spacing) * scale - spacing * scale)


def micro(pen, x, y, text, color, spacing=1):
    for i, ch in enumerate(text):
        rows = MICRO[ch]
        pen.stamp(x + i * (4 + spacing), y, list(rows), {'x': color})
    return pen


# Seven segments, two pixels thick, in the nine-by-thirteen cell a classic
# timer digit gets. The first pass set the clock in the editor's 5x7 face,
# which left the most-read number in the skin four pixels short of its cell and
# reading as a smudge behind its own glow.
_SEGMENTS = {
    'A': [(c, r) for r in (0, 1) for c in range(1, 6)],
    'F': [(c, r) for r in range(1, 6) for c in (0, 1)],
    'B': [(c, r) for r in range(1, 6) for c in (5, 6)],
    'G': [(c, r) for r in (5, 6) for c in range(1, 6)],
    'E': [(c, r) for r in range(6, 10) for c in (0, 1)],
    'C': [(c, r) for r in range(6, 10) for c in (5, 6)],
    'D': [(c, r) for r in (9, 10) for c in range(1, 6)],
}
_DIGIT_SEGMENTS = ('ABCDEF', 'BC', 'ABGED', 'ABGCD', 'FGBC', 'AFGCD', 'AFGECD',
                   'ABC', 'ABCDEFG', 'ABCDFG')


def digit_rows(value, lit='G'):
    """One seven-segment digit as 7x11 stamp rows."""
    grid = [[' '] * 7 for _ in range(11)]
    for name in _DIGIT_SEGMENTS[value]:
        for c, r in _SEGMENTS[name]:
            grid[r][c] = lit
    return [''.join(row) for row in grid]
