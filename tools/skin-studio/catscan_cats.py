"""The cats of Catamp Cat Scan: the ones on the films, and the one on the box.

There are two kinds of cat in this skin and they are drawn two different ways,
because a backlit skin has two different kinds of cat in it.

**Inside a film** a cat is an *anatomy*: layers of density seen through each
other. A skull is a braincase with a brain in it, an orbit that is a bright
ring around a black hole, a muzzle full of air, and a mandible in front of all
of it. Freehanding that as a grid of characters produces a ring -- the first
skull drawn for this skin was one -- because the hand draws the outline it can
see and leaves the inside empty, and on a radiograph the inside is the picture.
So anatomy is composed here out of discs, arcs and wedges on a character grid,
one density at a time, deepest first: it is the same construction a radiograph
is, and each part can be checked on its own.

**On the box** a cat is a *silhouette*: opaque, unmodelled, with no interior at
all, and the only thing that says what it is is the shape of its edge -- plus
the one place in the whole skin where light gets through something alive, which
is a cat's ears.

Nothing here needs an image library. The grids are exact characters; the
silhouettes are native paths.
"""
import math

import catscan_film as F

# The density alphabet. Every grid in this file is written in it, so a part
# lifted from one cat lands at the same density in another.
PALETTE = {
    '.': F.TISSUE_DEEP,   # the faintest thing the film caught
    ',': F.TISSUE,
    ':': F.TISSUE_LIT,
    '-': F.CARTILAGE,
    '=': F.BONE_DIM,
    'x': F.BONE,
    'X': F.BONE_LIT,
    'o': F.CORTEX,        # the dense rind of a bone
    'O': F.ENAMEL,        # a tooth
    '#': F.METAL,         # something that should not be in there
    '~': F.AIR,           # air *inside* the animal: a sinus, the airway
}
ORDER = '.,:-=xXoO#'


class Grid:
    """A character canvas. Anatomy is built on one of these deepest-first, so
    every part is composed against what is already behind it rather than cut
    out of it."""

    def __init__(self, w, h, fill=' '):
        self.w, self.h = w, h
        self.cells = [[fill] * w for _ in range(h)]

    def at(self, x, y):
        if 0 <= x < self.w and 0 <= y < self.h:
            return self.cells[y][x]
        return ' '

    def put(self, x, y, ch, *, over=True):
        if not (0 <= x < self.w and 0 <= y < self.h) or ch == ' ':
            return
        if not over and self.cells[y][x] != ' ':
            return
        self.cells[y][x] = ch

    def disc(self, cx, cy, rx, ry, ch, *, rim=None, over=True):
        """A filled ellipse, optionally with its own rind one pixel thick --
        which is what makes a bone a bone rather than a blob."""
        for y in range(self.h):
            for x in range(self.w):
                d = ((x - cx) / rx) ** 2 + ((y - cy) / ry) ** 2
                if d <= 1.0:
                    inner = ((x - cx) / max(.4, rx - 1)) ** 2 + \
                            ((y - cy) / max(.4, ry - 1)) ** 2
                    self.put(x, y, rim if (rim and inner > 1.0) else ch, over=over)
        return self

    def ring(self, cx, cy, rx, ry, ch, thickness=1.0):
        for y in range(self.h):
            for x in range(self.w):
                d = math.sqrt(((x - cx) / rx) ** 2 + ((y - cy) / ry) ** 2)
                if 1.0 - thickness / max(rx, ry) <= d <= 1.0:
                    self.put(x, y, ch)
        return self

    def bar(self, x0, y0, x1, y1, ch, width=1.0):
        """A straight bone of a given thickness, endpoints included."""
        steps = int(max(abs(x1 - x0), abs(y1 - y0)) * 2) + 1
        for i in range(steps + 1):
            t = i / steps
            px, py = x0 + (x1 - x0) * t, y0 + (y1 - y0) * t
            r = width / 2
            for dy in range(int(-r - 1), int(r + 2)):
                for dx in range(int(-r - 1), int(r + 2)):
                    if dx * dx + dy * dy <= r * r + .25:
                        self.put(round(px) + dx, round(py) + dy, ch)
        return self

    def arc(self, cx, cy, rx, ry, a0, a1, ch, width=1.0):
        """A rib, a zygomatic arch, the curve of a spine: a piece of an ellipse
        rather than a straight line, because nothing in an animal is straight.
        Angles are degrees, zero to the right, growing downward."""
        steps = int(max(rx, ry) * abs(a1 - a0) / 6) + 6
        for i in range(steps + 1):
            a = math.radians(a0 + (a1 - a0) * i / steps)
            px, py = cx + rx * math.cos(a), cy + ry * math.sin(a)
            r = width / 2
            for dy in range(int(-r - 1), int(r + 2)):
                for dx in range(int(-r - 1), int(r + 2)):
                    if dx * dx + dy * dy <= r * r + .25:
                        self.put(round(px) + dx, round(py) + dy, ch)
        return self

    def wedge(self, x0, y0, x1, y1, top, bottom, ch):
        """A muzzle, a tail root: a shape that tapers along its length."""
        for x in range(min(x0, x1), max(x0, x1) + 1):
            t = (x - x0) / max(1, x1 - x0)
            hi = y0 + (y1 - y0) * t - (top + (bottom - top) * t) / 2
            lo = hi + (top + (bottom - top) * t)
            for y in range(int(round(hi)), int(round(lo)) + 1):
                self.put(x, y, ch)
        return self

    def halo(self, ch, under=' ', around=ORDER):
        """One step of soft tissue around whatever is already drawn. A bone
        with nothing around it is a bone on a table, not a bone in a cat."""
        add = []
        for y in range(self.h):
            for x in range(self.w):
                if self.cells[y][x] != under:
                    continue
                if any(self.at(x + dx, y + dy) in around
                       for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1),
                                      (1, 1), (-1, -1), (1, -1), (-1, 1))):
                    add.append((x, y))
        for x, y in add:
            self.cells[y][x] = ch
        return self

    def rows(self):
        return [''.join(row) for row in self.cells]

    def stamp(self, pen, x, y, palette=None):
        pen.stamp(int(x), int(y), self.rows(), palette or PALETTE)
        return pen


# ------------------------------------------------------------------- the skull


def skull_lateral(w=24, h=18):
    """A cat skull from the side, nose to the left.

    Built in the order a beam passes through one: the soft outline of the head,
    the braincase and the brain inside it, the air in the muzzle, then the
    bones in front of the lot -- zygomatic arch, orbit, mandible, teeth. The
    orbit is the shape that makes it read as a cat rather than as a dog: it is
    nearly a full ring and it is enormous relative to the braincase.
    """
    g = Grid(w, h)
    cx, cy = w * .68, h * .40          # centre of the braincase
    # the head, as a soft outline nothing in it will quite fill
    g.disc(cx, cy + .8, w * .33, h * .42, ',')
    g.wedge(1, int(h * .56), int(w * .60), int(h * .48), h * .26, h * .50, ',')
    # braincase: a dome with the brain inside it and a dense rind on the outside
    g.disc(cx, cy, w * .29, h * .36, 'x')
    g.disc(cx + .4, cy + .4, w * .22, h * .27, '-')
    # the muzzle: nasal bone over the top, hard palate under, air between them
    g.arc(w * .40, h * .62, w * .38, h * .26, 185, 265, 'X', width=1.6)
    g.wedge(2, int(h * .56), int(w * .34), int(h * .52), h * .12, h * .20, '~')
    g.bar(1.5, h * .68, w * .44, h * .66, 'x', width=1.4)
    # the orbit: a bright ring with nothing inside it, and a cat's is nearly
    # a closed circle and enormous next to the braincase
    ox, oy, orx, ory = w * .53, h * .40, w * .16, h * .24
    # the eye itself is soft tissue, so the socket is dim rather than black --
    # and `put` ignores a space, so clearing it with one would leave whatever
    # the braincase happened to put there.
    g.disc(ox, oy, orx, ory, ',')
    g.ring(ox, oy, orx, ory, 'X', thickness=1.5)
    g.arc(ox, oy, orx * 1.08, ory * 1.08, 190, 350, 'o', width=1.2)
    # zygomatic arch: the thin bright bar that swings out under the eye
    g.arc(w * .56, h * .40, w * .28, h * .30, 25, 135, 'X', width=1.3)
    # tympanic bulla: the round bright bubble at the back of a cat's skull
    g.disc(w * .86, h * .72, w * .10, h * .14, 'o', rim='O')
    # mandible: the body along the bottom, the ramus rising behind it
    g.bar(w * .06, h * .84, w * .64, h * .86, 'x', width=1.6)
    g.bar(w * .64, h * .86, w * .80, h * .52, 'x', width=1.6)
    g.arc(w * .60, h * .58, w * .22, h * .30, 20, 90, 'X', width=1.2)
    # teeth: one upper canine and one lower, which is the whole cat in two marks
    g.bar(w * .11, h * .70, w * .08, h * .84, 'O', width=1.1)
    g.bar(w * .15, h * .82, w * .18, h * .70, 'O', width=1.1)
    for i in range(3):
        g.put(int(w * .24) + i * 2, int(h * .72), 'O')
    g.halo(',')
    return g


def skull_frontal(w=26, h=24):
    """A cat skull head-on: two enormous orbits and a short face. This is the
    picture the phrase `cat scan` puts in somebody's head, so it is worth the
    twenty-six pixels."""
    g = Grid(w, h)
    cx = (w - 1) / 2
    g.disc(cx, h * .42, w * .44, h * .42, ',')
    g.disc(cx, h * .38, w * .40, h * .36, ':', rim='x')
    g.disc(cx, h * .34, w * .33, h * .28, '-')
    for side in (-1, 1):
        ox = cx + side * w * .20
        g.disc(ox, h * .40, w * .15, h * .17, ',')
        g.ring(ox, h * .40, w * .15, h * .17, 'X', thickness=1.3)
        g.arc(ox, h * .40, w * .16, h * .18, 200, 340, 'o', width=1.1)
    # the nasal aperture: a black heart between and below the orbits
    g.disc(cx, h * .60, w * .10, h * .10, '~')
    g.arc(cx, h * .60, w * .12, h * .12, 190, 350, 'x', width=1.2)
    # the two upper canines, which is where the joke lives
    for side in (-1, 1):
        g.bar(cx + side * w * .13, h * .68, cx + side * w * .15, h * .86, 'O',
              width=1.3)
    g.bar(cx - w * .22, h * .74, cx + w * .22, h * .74, 'x', width=1.6)
    g.arc(cx, h * .60, w * .26, h * .30, 20, 160, 'x', width=1.6)
    g.halo(',')
    return g


def vertebra(w=9, h=10, lean=0.0):
    """One vertebra seen from the side, with its spinous process on top. `lean`
    tips it, which is the only thing the balance slider has to say."""
    g = Grid(w, h)
    cx = (w - 1) / 2 + lean * (w * .22)
    g.disc(cx, h * .60, w * .38, h * .26, 'x', rim='o')
    g.bar(cx, h * .50, cx + lean * w * .30, h * .10, 'X', width=1.8)
    g.bar(cx - w * .34, h * .78, cx + w * .34, h * .78, '=', width=1.2)
    g.halo(',')
    return g


def rib_cage(pen, x, y, w, h, *, spine, ribs=8, seed=0):
    """A thorax: the ribs springing from the spine and sweeping down and back,
    the heart as a haze with no edge, and the lungs as the air that is neither.

    `spine` is the row the vertebrae sit on, and every rib starts on it. The
    first version of this took its own top edge instead, which put the whole
    cage a few pixels below the spine it was supposed to hang off -- eight
    curves in a row with nothing holding them, which reads as a comb.
    """
    F.haze(pen, x, y + 2, w, h - 4, F.TISSUE_DEEP, strength=170)
    F.haze(pen, x + int(w * .26), y + int(h * .40), int(w * .40),
           int(h * .48), F.TISSUE_LIT, strength=110)          # the heart
    for i in range(ribs):
        t = i / max(1, ribs - 1)
        sx = x + w * (.10 + .78 * t)
        drop = h * (.62 + .30 * (1 - abs(t - .45) * 1.6))
        pen.curve((sx, spine), (sx - w * .17, spine + drop * .58),
                  (sx + w * .05, spine + drop), 1, F.BONE)
        pen.ops[-1].update(opacity=235)
        pen.curve((sx + .5, spine), (sx - w * .16, spine + drop * .54),
                  (sx + w * .05, spine + drop * .92), 1, F.BONE_LIT)
        pen.ops[-1].update(opacity=175)
    # the sternum, along the bottom where the ribs come back together
    pen.curve((x + w * .12, spine + h * .84), (x + w * .40, spine + h * .96),
              (x + w * .74, spine + h * .78), 2, F.BONE)
    pen.ops[-1].update(opacity=170)
    F.trabecular(pen, x, y, w, h, seed=seed, color=F.BONE_DIM, density=26)
    return pen


# --------------------------------------------------------- the cat on the box


def sitting_cat(pen, x, y, w, h, *, ear_light=True):
    """A cat sitting with its back to you, between you and the viewer.

    It is opaque, so it has no inside: one closed silhouette, the hair standing
    off its outline, and the ears -- the only thing in this skin the light
    actually passes through. The ears are cut into the outline rather than
    added on top of it, because a silhouette with two triangles laid over it is
    a cat wearing a hat.
    """
    cx = x + w / 2
    base = y + h - h * .05
    # The tail goes down before the body does, and it comes out past the
    # silhouette's own edge: a black tail drawn under a black cat is a tail
    # nobody will ever see.
    pen.taper((cx + w * .44, base - h * .05),
              (cx + w * .05, base + h * .07),
              (cx - w * .58, base - h * .01), 4, CAT_COLOR)
    pen.path([
        [cx - w * .48, base],
        [cx - w * .52, base - h * .26, cx - w * .32, base - h * .46],
        [cx - w * .25, base - h * .57, cx - w * .23, base - h * .66],
        [cx - w * .21, base - h * .76],
        [cx - w * .30, base - h * .99],
        [cx - w * .07, base - h * .86],
        [cx, base - h * .93, cx + w * .07, base - h * .86],
        [cx + w * .30, base - h * .99],
        [cx + w * .21, base - h * .76],
        [cx + w * .23, base - h * .66],
        [cx + w * .25, base - h * .57, cx + w * .32, base - h * .46],
        [cx + w * .52, base - h * .26, cx + w * .48, base],
    ], CAT_COLOR)
    if ear_light:
        # Light through an ear is the one warm colour in the file, and the one
        # place in it the beam passes through something alive.
        for side in (-1, 1):
            tip = (cx + side * w * .285, base - h * .975)
            inner = (cx + side * w * .075, base - h * .862)
            root = (cx + side * w * .205, base - h * .765)
            pen.path([list(tip), list(inner), list(root)], F.EAR_DEEP)
            pen.path([[tip[0] - side * w * .03, tip[1] + h * .025],
                      [inner[0] + side * w * .02, inner[1] - h * .012],
                      [root[0] - side * w * .02, root[1] - h * .012]], F.EAR)
            pen.path([[tip[0] - side * w * .055, tip[1] + h * .05],
                      [inner[0] + side * w * .05, inner[1] - h * .022],
                      [root[0] - side * w * .045, root[1] - h * .03]],
                     F.EAR_LIT)
    # the hair that stands off the edge of a cat in front of a light
    for i, (sx, sy, ex, ey) in enumerate((
            (cx - w * .33, base - h * .44, cx - w * .47, base - h * .50),
            (cx - w * .23, base - h * .70, cx - w * .34, base - h * .78),
            (cx + w * .33, base - h * .40, cx + w * .46, base - h * .46),
            (cx + w * .23, base - h * .68, cx + w * .34, base - h * .76))):
        F.hair(pen, (sx, sy), ((sx + ex) / 2, (sy + ey) / 2 - 1), (ex, ey),
               opacity=110 + 20 * (i % 2))
    return pen


CAT_COLOR = F.CAT


LOAF = (
    "          ,======,,           ",
    "   ,=,  ,=xxxxxxxxx=,   ,=,   ",
    "  ,=x=,=xxxxxxxxxxxxx=,=x=,   ",
    " ,=xxx=xxxxxxxxxxxxxxxxxxx=,  ",
    ",=xxxxxxxxxxxxxxxxxxxxxxxxx=, ",
    "=xxxxxxxxxxxxxxxxxxxxxxxxxxx=,",
    "=xxxxxxxxxxxxxxxxxxxxxxxxxxxx=",
    ",=xxxxxxxxxxxxxxxxxxxxxxxxxxx=",
    " ,=xxxxxxxxxxxxxxxxxxxxxxxxxx=",
    "  ,=xxxxxxxxxxxxxxxxxxxxxxxx=,",
    "   ,,=xxxxxxxxxxxxxxxxxxxxx=, ",
    "     ,,===============xxx=,,  ",
    "        ,,,,,,,,,,,,,,,,,     ",
)


def curled_cat(pen, x, y, w, h, *, seed=0):
    """A cat asleep in a loaf, radiographed: the spine curled right round on
    itself, the tail tucked along the front of it, the skull at one end.

    The silhouette version of this is a black shape, and a black shape on a
    strip of film is a hole. Inside a film a cat is an anatomy, so the same cat
    is drawn the other way: haze first, then the curve of the spine, then the
    head in front of it.
    """
    F.haze(pen, x, y + 1, w, h - 1, F.TISSUE, strength=160)
    g = Grid(w, h)
    g.arc(w * .52, h * .92, w * .40, h * .62, 190, 350, 'x', width=1.8)
    for i in range(11):
        a = 190 + i * 16
        import math as _m
        px = w * .52 + w * .40 * _m.cos(_m.radians(a))
        py = h * .92 + h * .62 * _m.sin(_m.radians(a))
        g.put(round(px), round(py) - 1, 'X')
    # the tail, tucked along the front, and the two back feet under it
    g.arc(w * .50, h * .62, w * .44, h * .34, 10, 170, '=', width=1.4)
    g.disc(w * .20, h * .58, w * .13, h * .30, ':', rim='=')
    # the head, at the near end, with an ear on it
    g.disc(w * .84, h * .48, w * .13, h * .34, 'x', rim='o')
    g.disc(w * .86, h * .46, w * .07, h * .18, '-')
    g.bar(w * .78, h * .22, w * .82, h * .02, 'x', width=1.2)
    g.bar(w * .92, h * .22, w * .94, h * .04, 'x', width=1.2)
    g.halo(',')
    g.stamp(pen, x, y)
    return pen


def loaf(pen, x, y, color=F.CAT, *, rim=F.CAT_RIM):
    """A cat asleep, seen from the side, folded into a loaf. Opaque like every
    other cat on the outside of the box, so it is one colour and an edge."""
    palette = {'=': rim, 'x': color, ',': rim}
    pen.stamp(int(x), int(y), list(LOAF), palette)
    return pen


PAW = (
    " ,=, ,=,  ",
    ",=x=,=x=, ",
    ",=x=,=x=,=",
    " ,=, ,=,,x",
    "  ,=xx=,=,",
    " ,=xxxx=, ",
    ",=xxxxxx=,",
    ",=xxxxxx=,",
    " ,=xxxx=, ",
    "  ,====,  ",
)


def paw_print(pen, x, y, color=F.STEEL_EDGE, *, opacity=90):
    """The smudge a cat leaves when it walks across a lit viewer. It is not a
    drawing on the film; it is grease on the diffuser, so it is a wash."""
    pen.stamp(int(x), int(y), list(PAW), {'x': color, '=': color, ',': color})
    pen.ops[-1].update(opacity=opacity)
    return pen


# ------------------------------------------ eleven things that were swallowed


def foreign_bodies():
    """The eleven equalizer handles.

    A classic skin gives every band the same eleven-pixel head. Unique
    equalizer art buys eleven cells of 14x25 instead, and eleven copies of one
    object is a pattern where eleven different ones are a story: this is the
    tray after the surgery, in the order they came out.

    They are drawn to fill the cell rather than to sit in the middle of it. The
    first set was nine pixels by five in a cell nearly three times that, which
    made eleven distinct objects into eleven identical specks.
    """
    return [
        ('hair tie', (
            "  ,#####,  ",
            " ##,,,,,## ",
            "##,     ,##",
            "#,       ,#",
            "#,       ,#",
            "#,       ,#",
            "##,     ,##",
            " ##,,,,,## ",
            "  ,#####,  ")),
        ('bottle cap', (
            " ,#######, ",
            "###########",
            "#,#,#,#,#,#",
            "###########",
            "#,#,#,#,#,#",
            "###########",
            " ,#######, ")),
        ('spring', (
            " ,#######, ",
            "##,,,,,,,##",
            " ,#######, ",
            "##,,,,,,,##",
            " ,#######, ",
            "##,,,,,,,##",
            " ,#######, ",
            "##,,,,,,,##",
            " ,#######, ",
            "##,,,,,,,##",
            " ,#######, ")),
        ('bead', (
            "  ,###,  ",
            " ,#####, ",
            ",##,,,##,",
            "###, ,###",
            "##,   ,##",
            "###, ,###",
            ",##,,,##,",
            " ,#####, ",
            "  ,###,  ")),
        ('bell', (
            "    ,#,    ",
            "   ,###,   ",
            "  ,#####,  ",
            " ,##===##, ",
            " ,#=====#, ",
            ",##=====##,",
            "###########",
            ",#########,",
            "   ,###,   ",
            "    ,#,    ")),
        ('paperclip', (
            " ,#####, ",
            ",#######,",
            "##,,,,,##",
            "#,,###,,#",
            "#,,#=#,,#",
            "#,,#=#,,#",
            "#,,#=#,,#",
            "#,,###,,#",
            "##,,,,,##",
            ",#######,",
            " ,#####, ")),
        ('screw', (
            " ##### ",
            " ##### ",
            " #,,,# ",
            "  ,#,  ",
            "  ###  ",
            "  ,#,  ",
            "  ###  ",
            "  ,#,  ",
            "  ###  ",
            "  ,#,  ",
            "  ###  ",
            "  ,#,  ",
            "   #   ")),
        ('button', (
            "  ,#####,  ",
            " ,#######, ",
            ",#########,",
            "##,,,,,,,##",
            "##,#,,#,,##",
            "##,,,,,,,##",
            "##,#,,#,,##",
            "##,,,,,,,##",
            ",#########,",
            " ,#######, ",
            "  ,#####,  ")),
        ('battery', (
            "   ,#,   ",
            "  ,###,  ",
            " ,#####, ",
            ",#######,",
            "#,,,,,,,#",
            "#,=====,#",
            "#,=====,#",
            "#,=====,#",
            "#,=====,#",
            "#,=====,#",
            "#,=====,#",
            "#,=====,#",
            "#,,,,,,,#",
            ",#######,",
            " ,#####, ")),
        ('brick', (
            ",##,  ,##, ",
            ",##,  ,##, ",
            "###########",
            "##=======##",
            "##=======##",
            "##=======##",
            "###########",
            ",#########,")),
        ('milk ring', (
            "  ,#####,  ",
            " ,#######, ",
            ",##,,,,,##,",
            "##,     ,##",
            "#,       ,#",
            "##,     ,##",
            ",##,,,,,##,",
            " ,#######, ",
            "  ,#####,  ")),
    ]


FISH = (
    "    ,==,    ",
    " ,=xxxxxx=, ",
    "=xXoXoXoXx=,",
    "OX=,=,=,=Xx=",
    "=xXoXoXoXx=,",
    " ,=xxxxxx=, ",
    "    ,==,    ",
)


def fish_bone(pen, x, y):
    """The preamp handle. Everything else in the tray came out of the cat; this
    one is what the cat thinks it is entitled to."""
    pen.stamp(int(x), int(y), list(FISH), PALETTE)
    return pen


def check(extra=None):
    """Every grid against its own alphabet and its own width, before a single
    operation is emitted. A row typed one character short is a defect that only
    shows up as a cat with a dent in it."""
    named = {'LOAF': LOAF, 'PAW': PAW, 'FISH': FISH}
    named.update({name: rows for name, rows in foreign_bodies()})
    named.update(extra or {})
    for name, rows in named.items():
        width = max(len(row) for row in rows)
        for i, row in enumerate(rows):
            unknown = {c for c in row if c != ' ' and c not in PALETTE}
            if unknown:
                raise ValueError(f'{name} row {i}: undefined {sorted(unknown)}')
            if len(row) != width:
                raise ValueError(
                    f'{name} row {i} is {len(row)} of {width} characters')
    for name, grid in (('skull_lateral', skull_lateral()),
                       ('skull_frontal', skull_frontal()),
                       ('vertebra', vertebra())):
        if not any(ch != ' ' for row in grid.rows() for ch in row):
            raise ValueError(f'{name} came out empty')
    return True
