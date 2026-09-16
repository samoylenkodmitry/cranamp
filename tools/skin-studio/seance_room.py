"""The room Catamp Seance happens in, as native Studio operations.

It is seven minutes past three in the morning. The cats have got the board out.

Every other Catamp has a light somewhere outside the picture: silver takes a key
light from above-left, kraft catches one, film has a whole fluorescent box
behind it. This one has none. **Every light in this skin is in this skin** --
nine candle flames, all of them drawn, all of them small -- and that single
decision is where the rest of the grammar comes from:

  * **Falloff is fast and it is coloured.** One step from a flame is cream, two
    is amber, three is a burnt brown, four is the room's violet and there is no
    fifth. A shadow here is never grey and never black; it is `VOID`, which is
    blue. Everything in the skin is coloured off one ladder, `GLOW`, and the
    only question ever asked about a pixel is how far it is from the nearest
    flame -- `nearer()` and `farther()` walk it, so a rim and the thing it is on
    are a fixed distance apart rather than two colours picked by eye twice.
  * **Bright side faces the flame it is near, not the same way as its
    neighbour.** There is no global light direction to be consistent with, so
    two objects a hand apart are lit from opposite sides, and getting this wrong
    is what makes a candlelit picture look like a grey picture with an orange
    filter on it.
  * **Eyeshine does not obey any of that.** It is reflected, not received: a cat
    across the room has exactly the same two discs as the one beside the candle.
    That is why the dark half of this skin is nothing but eyes, and why `EYE` is
    the one colour in the file that is not on the ladder.
  * **The dot is not a light.** Two pixels of flat `DOT`, no halo, no falloff,
    no shadow, and nothing it lands on gets brighter. Everything else here is
    warm or violet; it is neither, and it is the only thing in the room that the
    cats can see properly and you cannot.

The marks are drawn in spilled salt rather than chalk, because the table has a
cloth on it and a paw does not hold chalk. Salt is why every line in this skin
is broken: `salt()` is a dusty core with a scatter round it, and a sigil that
comes out as a clean stroke has stopped being something a cat drew.

No image library: `grain`, `opacity` and exact ramps say all of it.
"""

# --------------------------------------------------------------------- ladder

# How far a pixel is from the nearest flame, and nothing else. Index 0 is the
# far corner of the room; index 11 is the inside of a flame. Everything in the
# skin is one of these twelve, or is on its way between two of them.
VOID = '#0a0812'        # the room past every candle. Blue, not black.
DUSK = '#15111f'
GLOOM = '#231a28'
UMBER = '#352430'
RUST = '#4b2f31'
SCORCH = '#67402f'
BRONZE = '#8a562f'
COPPER = '#b0712f'
AMBER = '#d29240'
HONEY = '#eeb662'
CREAM = '#fbd693'
FLAME = '#fff1c8'       # the inside of a flame, and the brightest thing here

GLOW = [VOID, DUSK, GLOOM, UMBER, RUST, SCORCH, BRONZE, COPPER, AMBER, HONEY,
        CREAM, FLAME]

# The cloth on the table. Off the ladder on the cold side, because velvet in the
# dark goes blue where everything else goes brown.
CLOTH_DEEP = '#120e1c'
CLOTH = '#1b1526'
CLOTH_NAP = '#241c30'

# Salt. Pale and cool where no flame reaches it, and it warms like everything
# else -- SALT_LIT is salt three steps nearer a candle, not a second white.
SALT_SHADOW = '#2e2a3c'
SALT_DIM = '#585069'
SALT = '#9d95ad'
SALT_LIT = '#e8d3ae'
SALT_HOT = '#fff3d6'

# Salt has a ladder of its own and is the only material in the skin that does.
# Everything else brightens toward a flame and so brightens through amber;
# a grain of salt brightens toward *white*, because it is reflecting the flame
# rather than glowing with it. Run salt up GLOW instead and the sigils come out
# as a scatter of embers, which is a different and much worse picture.
SALT_LADDER = [SALT_SHADOW, SALT_DIM, SALT, SALT_LIT, SALT_HOT]

# Paper, which is the one thing in this skin that is not either cloth, wax,
# salt or a cat. It is aged and it is a long way from any flame, so it is a
# dull warm brown rather than white -- a white page here would be the brightest
# thing in the skin by a wide margin and the room would stop being dark.
PAPER = '#372b21'
PAPER_LIT = '#55432f'
PAPER_SHADE = '#2e2620'
PAPER_EDGE = '#6b5539'
PAPER_RULE = '#59462f'
PAPER_MARGIN = '#7a4a3c'
INK = '#1a1410'          # the only dark ink in the skin, and it is printed
# What the player writes into the track list. The transcript is not written in
# pencil, however much it looks as if it should be: pencil is dark and this page
# is dark, and dark on dark is the mistake that actually ships. Everything
# written anywhere in this skin is light.
LIST_INK = '#d8c49a'
LIST_NOW = '#fff3d2'
TAPE = '#cbb887'
TAPE_DIM = '#5e553f'

# Wax: the one thing in the room that is lit from the inside, because a candle
# body glows where the flame shines through it.
WAX = '#c9bda6'
WAX_DIM = '#6d6577'
WAX_LIT = '#f2e2bd'
WICK = '#2a2028'
WICK_EMBER = '#ff6a2a'

# Eyeshine. Reflected, so it is the same brightness at any distance, and the one
# green in a room of amber and violet.
EYE_DIM = '#6f8a3a'
EYE = '#b6d75c'
EYE_HOT = '#eafab4'
PUPIL = '#0a0812'

# The dot. Flat, unshaded, and the reason the cats are doing any of this.
# The one warm thing that is not a flame: candlelight coming through the thin
# skin of an ear. It is the only place in the skin a light gets through
# something alive, and it is why an ear that is up reads as an ear.
EAR_GLOW = '#e08a6c'

DOT = '#ff1f14'
DOT_SEAT = '#7d1109'    # the one pixel that stops it floating, never a halo

# Everything Cranamp writes live: the light of a flame falling on the cloth.
DISPLAY = '#ffdf9c'

KEY = '#ff00ff'         # the classic transparency key


def rgb(value):
    return tuple(int(value[i:i + 2], 16) for i in (1, 3, 5))


def hexcolor(channels):
    return '#%02x%02x%02x' % tuple(max(0, min(255, int(round(c)))) for c in channels)


def mix(a, b, t):
    a = rgb(a) if isinstance(a, str) else a
    b = rgb(b) if isinstance(b, str) else b
    return hexcolor(tuple(a[i] + (b[i] - a[i]) * t for i in range(3)))


def step(color):
    """Where a colour sits on the ladder, whether or not it is on it."""
    return min(range(len(GLOW)),
               key=lambda i: sum((p - q) ** 2 for p, q in zip(rgb(GLOW[i]), rgb(color))))


def brighter(ladder, color, steps=1):
    """Walk any ladder from wherever a colour sits on it."""
    here = min(range(len(ladder)),
               key=lambda i: sum((p - q) ** 2 for p, q in zip(rgb(ladder[i]), rgb(color))))
    return ladder[max(0, min(len(ladder) - 1, here + steps))]


def nearer(color, steps=1):
    """The same surface, that many steps closer to a flame. A rim light and the
    thing it sits on are a fixed distance apart; picking the second colour by
    eye is how a candlelit picture turns into a grey one with a filter on it."""
    return GLOW[max(0, min(len(GLOW) - 1, step(color) + steps))]


def farther(color, steps=1):
    return nearer(color, -steps)


def ramp_between(a, b, n):
    """Exact palette colours from a to b, for a shape's `ramp`."""
    return [mix(a, b, i / max(1, n - 1)) for i in range(n)]


# ---------------------------------------------------------------------- cloth

import math
import random


def cloth(pen, x, y, w, h, *, seed=7, shade=CLOTH):
    """The table cloth: dark velvet with a nap. It goes down opaque and
    everything else in the skin is blended over it, because `opacity` mixes with
    the surface as it stands and wants something under it to mix with."""
    pen.ops.append(dict(op='rect', x=x, y=y, width=w, height=h, color=shade,
                        fill=True, grain=9, grain_size=2, grain_seed=seed))
    return pen


# ---------------------------------------------------------------------- light


def pool(pen, cx, cy, rx, ry, *, peak=9, span=6, rings=12, strength=210,
         seed=1, wobble=0.06, over=None):
    """A pool of candlelight on a flat surface.

    Rings from the outside in, each one blended over what is already there, so
    the light lands *on* the cloth and the weave still shows through it.
    Painting the rings opaque gives a stack of orange saucers; painting them
    inside-out gives a bright ring with a dark middle, because every later ring
    covers the one before it.

    `over` is for the pools that land in a *sprite cell*: a cleared cell holds
    the transparency key, and `opacity` has no idea that colour is special, so a
    glow blended into one comes out as brown mud, quietly takes the cell's
    transparency away with it, and reports nothing wrong. Name the colour the
    cell will actually be seen against and every ring is composited here and
    written opaque instead, which is the same picture and is a picture rather
    than a surprise.

    `span` is how many ladder steps the falloff covers and `rings` is how many
    ellipses say it, and they are deliberately not the same number. Twelve rings
    over six steps means each ring is half a step from its neighbour -- a mix of
    two ladder colours rather than one of them -- and that is the whole
    difference between a pool of light and a contour map. Every ring also
    carries `grain`, and `wobble` pushes it off true by a seeded fraction,
    because a pool of candlelight with a true rim is a spotlight.
    """
    rng = random.Random(seed)
    for i in range(rings):
        t = 1.0 - i / float(rings)
        here = peak - span * (1.0 - i / float(max(1, rings - 1)))
        low = max(0, min(len(GLOW) - 2, int(math.floor(here))))
        color = mix(GLOW[low], GLOW[low + 1], max(0.0, min(1.0, here - low)))
        alpha = max(1, min(255, int(strength * (0.40 + 0.60 * (i / max(1, rings - 1))))))
        w = max(1, int(round(rx * 2 * t * (1 + rng.uniform(-wobble, wobble)))))
        h = max(1, int(round(ry * 2 * t * (1 + rng.uniform(-wobble, wobble)))))
        ring = dict(op='ellipse',
                    x=int(round(cx - w / 2 + rng.uniform(-1, 1) * rx * wobble)),
                    y=int(round(cy - h / 2 + rng.uniform(-1, 1) * ry * wobble)),
                    width=w, height=h, color=color, fill=True,
                    grain=10, grain_size=1, grain_seed=seed * 13 + i)
        if over is None:
            ring['opacity'] = alpha
        else:
            ring['color'] = mix(over, color, alpha / 255.0)
        pen.ops.append(ring)
    return pen


def halo(pen, cx, cy, r, *, peak=10, rings=3, strength=150):
    """The air round a flame. Round, unlike a pool, because it is not lying on
    anything."""
    return pool(pen, cx, cy, r, r, peak=peak, rings=rings, strength=strength)


def flame(pen, x, y, *, height=5, width=3, lean=0.0, seed=0):
    """A flame, drawn as the three things a flame is: a body of amber, a cream
    core, and a blue-hot foot where it meets the wick.

    `lean` is how far the tip is pushed sideways. Nothing in this skin is ever
    perfectly upright -- eleven vertical flames in a row read as a fence.
    """
    tip = (x + lean, y - height)
    pen.ops.append(dict(op='path', x=0, y=0, color=AMBER, fill=True, brush_size=1,
                        points=[[x - width / 2.0, y],
                                [x - width / 2.0 - 0.3, y - height * 0.55, tip[0], tip[1]],
                                [x + width / 2.0 + 0.3, y - height * 0.55, x + width / 2.0, y]]))
    if height >= 3:
        core = max(1, height - 2)
        pen.ops.append(dict(op='path', x=0, y=0, color=CREAM, fill=True, brush_size=1,
                            points=[[x - width / 4.0, y],
                                    [x - width / 4.0, y - core * 0.6, x + lean * 0.7, y - core],
                                    [x + width / 4.0, y - core * 0.6, x + width / 4.0, y]]))
    if height >= 5:
        pen.ops.append(dict(op='rect', x=int(round(x)), y=int(round(y - height + 1)),
                            width=1, height=1, color=FLAME, fill=True))
    pen.ops.append(dict(op='rect', x=int(round(x)), y=int(round(y)), width=1, height=1,
                        color=WICK_EMBER, fill=True))
    return pen


def lit(pen, x, y, *, height=5, width=3, lean=0.0, reach=9, seed=0):
    """A lit candle end: the halo goes down first, then the flame on top of it,
    because the halo blends and would otherwise dissolve the flame it is for."""
    halo(pen, x, y - height * 0.55, reach, peak=9, rings=3, strength=120)
    flame(pen, x, y, height=height, width=width, lean=lean, seed=seed)
    return pen


def smoke(pen, x, y, *, height=8, drift=3, seed=0):
    """What a snuffed candle leaves. A curl, never a straight line, and it is
    the one thing in the room that gets *darker* as it goes up."""
    rng = random.Random(seed)
    steps = max(2, height)
    for i in range(steps):
        t = i / float(steps - 1)
        px = x + drift * t * t + rng.uniform(-0.6, 0.6)
        py = y - i
        if rng.random() < 0.25 + 0.3 * t:
            continue
        pen.ops.append(dict(op='rect', x=int(round(px)), y=int(round(py)), width=1,
                            height=1, color=farther(GLOOM, 0) if t > 0.6 else UMBER,
                            fill=True))
    return pen


# ----------------------------------------------------------------------- salt

def line_pts(a, b, step=0.5):
    (x1, y1), (x2, y2) = a, b
    n = max(1, int(round(math.hypot(x2 - x1, y2 - y1) / step)))
    return [(x1 + (x2 - x1) * i / n, y1 + (y2 - y1) * i / n) for i in range(n + 1)]


def curve_pts(a, control, b, n=24):
    (x1, y1), (cx, cy), (x2, y2) = a, control, b
    out = []
    for i in range(n + 1):
        t = i / float(n)
        u = 1 - t
        out.append((u * u * x1 + 2 * u * t * cx + t * t * x2,
                    u * u * y1 + 2 * u * t * cy + t * t * y2))
    return out


def arc_pts(cx, cy, rx, ry, a0, a1, n=32):
    return [(cx + rx * math.cos(a0 + (a1 - a0) * i / n),
             cy + ry * math.sin(a0 + (a1 - a0) * i / n)) for i in range(n + 1)]


def salt(pen, points, *, shade=SALT, seed=0, weight=0.72, scatter=0.5, dust=None):
    """A mark drawn in spilled salt by something with no thumbs.

    Every line in this skin is broken, and this is the only place that is
    decided. A salt line is a scatter of grains along a path -- some of the path
    has none, some of it has a clump two grains wide -- with a fainter dust
    round it where the paw pushed salt aside. Drawn as a clean stroke the sigils
    stop being something the cats made and start being a logo, which is the one
    thing a summoning circle must not look like.

    `weight` is how much of the path actually gets a grain and `scatter` how far
    the dust goes; the seed is what makes it the same mark every replay.
    """
    rng = random.Random(seed)
    dust = dust or brighter(SALT_LADDER, shade, -1)
    placed = set()
    for (px, py) in points:
        if rng.random() < weight:
            key = (int(round(px)), int(round(py)))
            if key not in placed:
                placed.add(key)
                pen.ops.append(dict(op='rect', x=key[0], y=key[1], width=1, height=1,
                                    color=shade if rng.random() > 0.22
                                    else brighter(SALT_LADDER, shade),
                                    fill=True))
        if scatter and rng.random() < scatter * 0.35:
            key = (int(round(px + rng.uniform(-scatter, scatter))),
                   int(round(py + rng.uniform(-scatter, scatter))))
            if key not in placed:
                placed.add(key)
                pen.ops.append(dict(op='rect', x=key[0], y=key[1], width=1, height=1,
                                    color=dust, fill=True))
    return pen


def salt_line(pen, a, b, **kw):
    return salt(pen, line_pts(a, b), **kw)


def salt_curve(pen, a, control, b, n=28, **kw):
    return salt(pen, curve_pts(a, control, b, n), **kw)


def salt_arc(pen, cx, cy, rx, ry, a0=0.0, a1=math.tau, n=40, **kw):
    return salt(pen, arc_pts(cx, cy, rx, ry, a0, a1, n), **kw)


def spill(pen, cx, cy, r, *, seed=0, grains=60, shade=SALT_DIM):
    """A heap of spilled salt: dense in the middle, gone by the edge."""
    rng = random.Random(seed)
    for _ in range(grains):
        t = rng.random() ** 1.8
        a = rng.uniform(0, math.tau)
        px = int(round(cx + math.cos(a) * r * t))
        py = int(round(cy + math.sin(a) * r * t * 0.55))
        pen.ops.append(dict(op='rect', x=px, y=py, width=1, height=1, fill=True,
                            color=brighter(SALT_LADDER, shade) if t < 0.35 else shade))
    return pen


# ------------------------------------------------------------------- eyeshine

def eyes(pen, x, y, *, gap=4, w=2, h=2, shade=EYE, shut=0.0, glint=True):
    """Two discs of reflected candlelight, and the only thing in this skin that
    is the same brightness however far from a flame it is. A cat in the dark is
    drawn with this and nothing else; adding a body to it is what turns the far
    half of the room from a room into a wall with stickers on it.

    `shut` closes them: 0 is a stare, 1 is asleep and draws nothing at all.
    """
    if shut >= 1.0:
        return pen
    height = max(1, int(round(h * (1 - shut))))
    for side in (0, gap + w):
        ex = int(round(x + side))
        ey = int(round(y + (h - height) / 2.0))
        if w <= 1 or height <= 1:
            pen.ops.append(dict(op='rect', x=ex, y=ey, width=max(1, w), height=height,
                                color=shade, fill=True))
            continue
        pen.ops.append(dict(op='ellipse', x=ex, y=ey, width=w, height=height,
                            color=shade, fill=True))
        if glint and w >= 3 and height >= 3:
            pen.ops.append(dict(op='rect', x=ex + w - 2, y=ey, width=1, height=1,
                                color=EYE_HOT, fill=True))
        if w >= 4 and height >= 3:
            pen.ops.append(dict(op='rect', x=ex + w // 2 - 1 + (w % 2), y=ey + 1,
                                width=1, height=max(1, height - 2), color=PUPIL, fill=True))
    return pen


# ------------------------------------------------------------------- the dot

def dot(pen, x, y, *, size=2):
    """The small red god. Flat, unshaded, no halo, and nothing it lands on gets
    brighter -- it is the one thing in the skin that is not made of candlelight,
    which is exactly why the cats can see it and you can only nearly."""
    pen.ops.append(dict(op='rect', x=int(x), y=int(y), width=size, height=size,
                        color=DOT, fill=True))
    return pen


# ------------------------------------------------------------------------ wax

def candle_body(pen, x, y, w, h, *, seed=0, warm=True, runs=2, base=None):
    """A candle: a column of wax with the flame's own light coming down it.

    Wax is translucent, so a candle is not a cylinder with a light above it --
    the top two or three pixels are nearly as bright as the flame and the glow
    carries a surprising way down the inside. A flat column with a highlight
    stripe reads as a plastic tube; the ramp is what makes it a candle.
    """
    top = mix(WAX_LIT, FLAME, 0.35) if warm else WAX
    foot = base or (UMBER if warm else CLOTH_NAP)
    pen.ops.append(dict(op='rect', x=x, y=y, width=w, height=h, color=top, fill=True,
                        ramp=ramp_between(top, foot, 8), ramp_axis=[x, y, x, y + h],
                        grain=4, grain_size=1, grain_seed=seed))
    if w >= 3:
        pen.ops.append(dict(op='rect', x=x + w - 1, y=y + 1, width=1, height=h - 1,
                            color=farther(foot, 1), fill=True, opacity=170))
    rng = random.Random(seed + 91)
    for _ in range(runs):
        rx = x + rng.randrange(0, max(1, w))
        length = rng.randint(2, max(3, h // 2))
        pen.ops.append(dict(op='rect', x=rx, y=y + 1, width=1, height=length,
                            color=WAX_LIT, fill=True, opacity=150))
        pen.ops.append(dict(op='rect', x=rx, y=y + length, width=1, height=1,
                            color=WAX_LIT, fill=True))
    return pen


def wax_pool(pen, cx, y, r, *, seed=0, warm=True):
    """The puddle a candle stands in. It is lit from above by its own flame, so
    it is brightest at the rim nearest the wick and not at its centre."""
    pen.ops.append(dict(op='ellipse', x=int(cx - r), y=y, width=int(r * 2), height=3,
                        color=WAX if warm else WAX_DIM, fill=True,
                        grain=6, grain_size=1, grain_seed=seed))
    pen.ops.append(dict(op='ellipse', x=int(cx - r) + 1, y=y, width=max(1, int(r * 2) - 2),
                        height=1, color=WAX_LIT if warm else WAX, fill=True))
    return pen


def snuffed(pen, x, y, *, drift=3, seed=0):
    """A candle nobody has lit, or one the draught got: a black wick, a bead of
    set wax under it and a curl of smoke. Off is never a dimmer on in this
    skin -- it is the light being somewhere else."""
    pen.ops.append(dict(op='rect', x=int(x), y=int(y), width=1, height=2,
                        color=WICK, fill=True))
    smoke(pen, x, y - 2, height=6, drift=drift, seed=seed)
    return pen
