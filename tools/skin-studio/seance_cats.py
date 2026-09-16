"""The nine cats of Catamp Seance, and the two ways this skin has of drawing one.

A skin whose only lights are inside it has exactly two kinds of cat in it, and
which one a cat is has nothing to do with the cat. It is decided by where the
cat is sitting.

**A cat near a flame is a crescent.** You do not see a cat; you see the edge of
one. The whole animal is a hole in the light -- filled one step *darker* than
whatever it is standing in front of -- and the only drawn thing is the two or
three pixels of fur along the side that faces the candle, brightest where the
edge turns square-on to the flame and gone by the time it turns away. `rimlit()`
does that arithmetic from the outline and the flame's position, so the same cat
put down beside a different candle relights itself and cannot disagree with the
room. Shading the *inside* of one of these is what turns a cat in the dark into
a grey cat in a grey room, which is the mistake this file exists to make
impossible: there is no call here that fills a cat with fur colour.

**A cat away from every flame is two eyes.** Not a dim cat: no cat. Eyeshine is
reflected rather than received, so it does not fall off, and a cat eight feet
back has exactly the discs the one by the candle has while every other part of
it has gone. The dark half of this skin is a room of eyes at different heights,
and each pair is a whole cat.

Small cells get a third treatment for a plain reason of size: below about twelve
pixels there is no rim left to walk -- a 9x9 status lamp has room for an eye and
nothing else -- so those are hand-authored character grids, checked against
their cell and their palette before a single operation is emitted.
"""
import math
import random

import seance_room as R


# ------------------------------------------------------------------- outlines

def spline(points, samples=6, closed=True):
    """A smooth closed outline through a handful of control points.

    Catmull-Rom, so the points are *on* the curve and a cat can be adjusted by
    moving the place a shoulder actually is. Construction stays fractional all
    the way to the engine: a cat built at 1.0 and the same cat at 0.55 are the
    same numbers, not a resample.
    """
    n = len(points)
    out = []
    span = n if closed else n - 1
    for i in range(span):
        p0 = points[(i - 1) % n] if closed else points[max(0, i - 1)]
        p1 = points[i % n]
        p2 = points[(i + 1) % n]
        p3 = points[(i + 2) % n] if closed else points[min(n - 1, i + 2)]
        for s in range(samples):
            t = s / float(samples)
            t2, t3 = t * t, t * t * t
            out.append((
                0.5 * ((2 * p1[0]) + (-p0[0] + p2[0]) * t +
                       (2 * p0[0] - 5 * p1[0] + 4 * p2[0] - p3[0]) * t2 +
                       (-p0[0] + 3 * p1[0] - 3 * p2[0] + p3[0]) * t3),
                0.5 * ((2 * p1[1]) + (-p0[1] + p2[1]) * t +
                       (2 * p0[1] - 5 * p1[1] + 4 * p2[1] - p3[1]) * t2 +
                       (-p0[1] + 3 * p1[1] - 3 * p2[1] + p3[1]) * t3)))
    return out


def place(outline, x, y, *, scale=1.0, flip=False):
    """Put a unit cat somewhere at some size. Nothing is rounded here."""
    return [(x + (-p[0] if flip else p[0]) * scale, y + p[1] * scale) for p in outline]


# The cats, as unit outlines. x runs right, y runs down, and the ground is y=0,
# so a cat is placed by the spot it is sitting on and grows upward from it.

SITTING = spline([
    (0.06, 0.00), (-0.14, -0.04), (-0.34, -0.14), (-0.44, -0.36),   # front paws
    (-0.42, -0.60), (-0.52, -0.82), (-0.58, -1.04),                 # chest, throat
    (-0.50, -1.18), (-0.62, -1.50), (-0.38, -1.32),                 # near ear
    (-0.20, -1.30), (-0.02, -1.52), (0.02, -1.26),                  # far ear
    (0.12, -1.04), (0.06, -0.80),                                   # back of the head
    (0.26, -0.60), (0.42, -0.30), (0.46, -0.08),                    # haunch
    (0.30, 0.02), (0.06, 0.04), (-0.30, 0.04), (-0.46, -0.02),      # tail round the paws
    (-0.54, -0.14), (-0.40, -0.10), (-0.16, -0.08),
])

LOAF = spline([
    (0.02, 0.00), (-0.24, -0.02), (-0.46, -0.14), (-0.54, -0.34),
    (-0.46, -0.48), (-0.56, -0.72), (-0.34, -0.54),          # near ear
    (-0.14, -0.52), (0.02, -0.74), (0.04, -0.48),            # far ear
    (0.24, -0.44), (0.46, -0.32), (0.56, -0.14),             # back and haunch
    (0.50, 0.00), (0.24, -0.04), (0.10, 0.02),               # tail tucked in front
])

CURLED = spline([
    (0.00, 0.00), (-0.34, -0.02), (-0.58, -0.20), (-0.60, -0.46),
    (-0.42, -0.66), (-0.50, -0.86), (-0.28, -0.66),          # ear
    (-0.12, -0.62), (-0.02, -0.84), (0.04, -0.60),           # ear
    (0.24, -0.66), (0.46, -0.54), (0.58, -0.32), (0.56, -0.10),
    (0.30, 0.02), (0.10, -0.06),
])

# A cat with its back to the room: a haystack with two ears cut into it and a
# tail laid round one side. There is no face to draw and that is the point --
# at the sizes this one is used at, a face is four pixels of mud.
BEHIND = spline([
    (0.00, 0.00), (-0.34, -0.02), (-0.44, -0.20), (-0.38, -0.52),
    (-0.30, -0.72), (-0.44, -0.96), (-0.22, -0.80),          # near ear
    (0.00, -0.78), (0.22, -0.98), (0.28, -0.74),             # far ear
    (0.36, -0.52), (0.42, -0.22), (0.50, -0.04),
    (0.30, 0.04), (0.10, 0.00),
])

WALKING = spline([
    (0.00, 0.00), (-0.06, -0.14), (-0.14, -0.30),            # front leg
    (-0.26, -0.34), (-0.42, -0.32), (-0.48, -0.42),
    (-0.62, -0.46), (-0.56, -0.62), (-0.44, -0.54),          # near ear
    (-0.34, -0.56), (-0.26, -0.70), (-0.22, -0.52),          # far ear
    (-0.10, -0.44), (0.14, -0.46), (0.36, -0.44),            # the back
    (0.52, -0.36), (0.64, -0.50), (0.72, -0.34),             # tail up
    (0.58, -0.30), (0.46, -0.24), (0.44, 0.00),              # back leg
    (0.36, -0.02), (0.34, -0.26), (0.10, -0.28), (0.08, 0.00),
])


# ------------------------------------------------------------------ rim light

def _densify(points, step=0.35):
    """Put the outline's samples closer together than a pixel.

    A unit cat has a fixed number of control samples and a cat may be drawn at
    any size, so at 44 pixels the spline's own points are a pixel and a half
    apart and the rim comes out as a string of beads with the room showing
    between them. The fix belongs here rather than in the cat: nothing about the
    shape of a cat says how many samples it wants.
    """
    out = []
    n = len(points)
    for i, (x, y) in enumerate(points):
        nx, ny = points[(i + 1) % n]
        out.append((x, y))
        far = math.hypot(nx - x, ny - y)
        for k in range(1, int(far / step)):
            t = k * step / far
            out.append((x + (nx - x) * t, y + (ny - y) * t))
    return out


def _outward(points):
    """Which way is out. A polygon's winding decides the sign of its normals,
    and getting it backwards lights the far side of every cat -- which looks
    almost right, and is the reason this is measured rather than assumed."""
    area = 0.0
    for i, (x, y) in enumerate(points):
        nx, ny = points[(i + 1) % len(points)]
        area += x * ny - nx * y
    # y runs down on a screen, which flips the sign of that cross product
    # against the one every textbook prints.
    return 1.0 if area > 0 else -1.0


def silhouette(pen, points, color):
    """The cat itself: a hole in the light, filled flat. Every cat in this skin
    is one of these first, and most of them are only this."""
    ox, oy = int(math.floor(points[0][0])), int(math.floor(points[0][1]))
    rel = [[points[0][0] - ox, points[0][1] - oy]]
    rel += [[p[0] - ox, p[1] - oy] for p in points[1:]]
    pen.ops.append(dict(op='path', x=ox, y=oy, points=rel, color=color,
                        fill=True, brush_size=1))
    return pen


def rimlit(pen, points, light, *, ground, depth=1, reach=5, softness=0.55,
           radius=26.0, thickness=3, body=None, seed=0):
    """A cat as the light actually leaves it: a silhouette, plus the band of fur
    along the edge that faces the flame.

    For every step round the outline the normal is taken, dotted with the
    direction to the flame and divided by the distance, and the answer is how
    many steps up the ladder that piece of edge climbs. Square-on and close is
    `reach` steps; side-on or far is one; facing away is not drawn at all, which
    is what lets the far edge of a cat merge into the room instead of outlining
    it.

    The lit edge is a *band*, not a line. One pixel of rim is a cartoon outline
    and reads as a sticker at any size; the light wraps two or three pixels
    round a piece of fur before it gives up, and each step inward is one step
    back down the ladder. `thickness` is how many it is allowed, and the bright
    parts of the edge use all of them while the grazing parts use one.

    `ground` is what the cat is standing *in front of*, not what the cat is made
    of. That is the whole trick: a cat has no colour of its own in this skin.
    """
    sign = _outward(points)
    silhouette(pen, points, body or R.farther(ground, depth))
    rng = random.Random(seed)
    placed = {}
    points = _densify(points)
    n = len(points)
    for i, (px, py) in enumerate(points):
        ax, ay = points[(i - 1) % n]
        bx, by = points[(i + 1) % n]
        tx, ty = bx - ax, by - ay
        length = math.hypot(tx, ty)
        if length < 1e-6:
            continue
        nx, ny = sign * ty / length, -sign * tx / length
        lx, ly = light[0] - px, light[1] - py
        far = math.hypot(lx, ly)
        if far < 1e-6:
            continue
        facing = (nx * lx + ny * ly) / far
        if facing <= 0.02:
            continue
        # Facing decides whether the edge is lit; distance decides how much.
        # With facing alone a cat across the room gets the same rim as the one
        # against the candle, and the room stops having any depth in it.
        lift = facing ** softness * reach / (1.0 + (far / radius) ** 2)
        if lift < 0.5:
            continue
        for d in range(max(1, min(thickness, int(round(lift))))):
            key = (int(round(px + nx * 0.25 - nx * d)),
                   int(round(py + ny * 0.25 - ny * d)))
            placed[key] = max(placed.get(key, 0.0), lift - d)
    for (kx, ky), lift in sorted(placed.items()):
        steps = int(round(lift + rng.uniform(-0.22, 0.22)))
        if steps < 1:
            continue
        pen.ops.append(dict(op='rect', x=kx, y=ky, width=1, height=1,
                            color=R.nearer(ground, steps), fill=True))
    return pen


def tail(pen, root, control, tip, light, *, ground, width=6.0, reach=6,
         radius=40.0, thickness=2, seed=0, samples=36):
    """A tail, and nothing else: the rest of the cat is off the edge of the
    window.

    A classic skin's margins are twenty-odd pixels wide and a whole cat put in
    one comes out as a smear with ears. A tail is the one part of a cat that is
    *supposed* to be a long thin thing, it reads instantly at two pixels wide,
    and it says there is a cat there without claiming any room for one. It is
    built as a closed outline so it lights by the same rule as every other cat
    here -- a curve stroked in fur colour would be the one thing in the skin
    with a colour of its own.
    """
    curve = R.curve_pts(root, control, tip, samples)
    left, right = [], []
    for i, (px, py) in enumerate(curve):
        ax, ay = curve[max(0, i - 1)]
        bx, by = curve[min(len(curve) - 1, i + 1)]
        tx, ty = bx - ax, by - ay
        length = math.hypot(tx, ty) or 1.0
        nx, ny = ty / length, -tx / length
        half = width * (1.0 - 0.72 * (i / float(len(curve) - 1))) / 2.0
        left.append((px + nx * half, py + ny * half))
        right.append((px - nx * half, py - ny * half))
    rimlit(pen, left + list(reversed(right)), light, ground=ground, reach=reach,
           radius=radius, thickness=thickness, seed=seed)
    return pen


def dark_cat(pen, points, *, ground, gaze=None, gap=3, w=2, h=2, shut=0.0, seed=0):
    """A cat with no candle anywhere near it: a shape you can only tell is there
    because it is blocking something, and two eyes that do not care how far away
    they are. `gaze` is where the eyes go; leave it and they go where the head
    is, which for every cat in this skin is the wrong answer -- all nine of them
    are looking at the same place."""
    silhouette(pen, points, R.farther(ground, 1))
    if gaze:
        R.eyes(pen, gaze[0], gaze[1], gap=gap, w=w, h=h, shut=shut)
    return pen
