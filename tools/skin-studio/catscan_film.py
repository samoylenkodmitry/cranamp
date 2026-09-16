"""Backlit radiography film for Catamp Cat Scan, as native Studio operations.

Every other Catamp is lit from above-left: silver glints, kraft catches a key
light, felt sits on its own cast shadow. This one has no key light at all. The
only light in the skin is *behind* the picture, in the box, and everything you
can see is something standing in front of it. That inverts every rule the other
skins are drawn by, and gives this one four of its own:

  * **A bright pixel is a thin one.** Value is density, not illumination.
    Air is black, soft tissue is a broad dim haze, bone is bright, and a
    swallowed staple is pure white. Nothing is ever lit; things are only more
    or less in the way.
  * **An opaque thing has no interior.** Clips, tape, the vet's grease pencil
    and the cat sitting on the box are silhouettes -- flat, unmodelled -- and
    the only thing that describes their shape is the light that leaks past the
    edge. Shading the inside of a silhouette is what makes it read as a sticker.
  * **Bone is three values and a haze, never one.** The haze goes down first
    and is wider than the bone; then the spongy middle, a speckle one step up;
    then the dense rind, one step up again and only on the outline. One flat
    white shape is a paper cut-out, and three values is a radiograph.
  * **Pressed means pressed against the light.** Everywhere else in Catamp a
    pressed control goes darker. Here a chip pushed flat against the diffuser
    loses the shadow it was floating on and the light behind it comes up. It is
    the one state change in this skin that is not a change of brightness in the
    same direction as its neighbours, and it is the whole reason the skin works.

And one more, which is the only warm colour in the file: the light coming
through a cat's ear. Everything else in the skin is the green-grey of a
fluorescent viewer behind a sheet of polyester, and the ears are the single
place the light passes through something alive.

No image library: `grain`, `opacity` and exact palette ramps say all of it, so
the recipe runs wherever Studio does.
"""
import random

# ------------------------------------------------------------------- the light

GLOW = '#eef8f0'        # bare diffuser, nothing in front of it
GLOW_DIM = '#c3d8c8'    # the diffuser seen through the frame's own edge
GLOW_DEEP = '#93ab9d'

# The box. Brushed steel, lit by nothing but the leak out of its own slots, so
# it is brighter the closer it gets to one and never brighter at the top-left.
STEEL_DEEP = '#0f1314'
STEEL_DARK = '#171d1f'
STEEL = '#212a2c'
STEEL_LIT = '#2f3b3d'
STEEL_EDGE = '#48585b'

# Film, from the black the beam leaves where it hits nothing to the white a
# staple leaves where it hits something the beam cannot get through at all.
AIR = '#0a1112'         # unattenuated: the space around the animal
FOG = '#121c1d'         # base fog -- unexposed film is never actually black
TISSUE_DEEP = '#1b2827'
TISSUE = '#263634'
TISSUE_LIT = '#334642'
CARTILAGE = '#455a53'
BONE_DIM = '#5e766c'
BONE = '#7f9789'
BONE_LIT = '#a3bbab'
CORTEX = '#c8ddcb'      # the dense rind of a bone
ENAMEL = '#e8f6e9'      # teeth, the densest thing a cat grows
METAL = '#ffffff'       # and the densest thing a cat swallows

# What the vet leaves on the film.
WAX_RED = '#c8382f'
WAX_RED_DEEP = '#8d241e'
WAX_BLUE = '#3f74c0'
TAPE = '#cbb887'
PLATE = '#dfeee2'       # a clear window in the film: a printed data strip
PLATE_DIM = '#bed2c4'
INK = '#0c1415'         # what is printed on a clear strip, in black
# The display ink: everything Cranamp writes live. It is the light coming
# through a window in the film, not ink on a page -- so it is the brightest
# thing in the skin and it is written on the blackest. A clear strip with black
# lettering is how the *printed* parts of a film read, and putting the live
# readouts on one too would have put a glaring white block in the middle of
# every dark window; the checker would have passed it and the room would not.
DISPLAY = '#e3f2e8'

# The one warm thing in the skin.
EAR_DEEP = '#9c4f3c'
EAR = '#e08a6c'
EAR_LIT = '#f2b79c'
CAT = '#0b0f10'         # a cat in front of a light is a shape and nothing else
CAT_RIM = '#3c4a47'

KEY = '#ff00ff'         # the classic transparency key

# The density ladder, darkest first. Indexing it is how a bone gets three
# values that are a fixed distance apart instead of three colours picked twice.
LADDER = [AIR, FOG, TISSUE_DEEP, TISSUE, TISSUE_LIT, CARTILAGE, BONE_DIM,
          BONE, BONE_LIT, CORTEX, ENAMEL, METAL]


def rgb(value):
    return tuple(int(value[i:i + 2], 16) for i in (1, 3, 5))


def hexcolor(channels):
    return '#%02x%02x%02x' % tuple(max(0, min(255, int(round(c)))) for c in channels)


def mix(a, b, t):
    a, b = rgb(a) if isinstance(a, str) else a, rgb(b) if isinstance(b, str) else b
    return tuple(a[i] + (b[i] - a[i]) * t for i in range(3))


def denser(color, steps=1):
    """Up the density ladder from wherever this colour sits on it: the way a
    bone gets its rind, without a second colour being chosen by eye."""
    best = min(range(len(LADDER)),
               key=lambda i: sum((p - q) ** 2 for p, q in zip(rgb(LADDER[i]), rgb(color))))
    return LADDER[max(0, min(len(LADDER) - 1, best + steps))]


def thinner(color, steps=1):
    return denser(color, -steps)


def darken(color, t):
    return hexcolor(mix(color, '#000000', t))


def lighten(color, t):
    return hexcolor(mix(color, '#ffffff', t))


def ramp_between(a, b, steps=24):
    """Exact palette stops. Studio picks the nearest stop rather than
    interpolating, so a two-colour ramp is a hard split, not a gradient."""
    return [hexcolor(mix(a, b, i / (steps - 1))) for i in range(steps)]


# ------------------------------------------------------------------- the box


def steel(pen, x, y, w, h, *, seed=0, near=None, grain=4):
    """A panel of the viewer's case. `near` is the edge the diffuser is behind,
    so the metal brightens toward the light rather than toward the top-left.
    """
    top, bottom = STEEL_DARK, STEEL
    axis = [x, y, x, y + h - 1]
    if near == 'top':
        top, bottom = STEEL_LIT, STEEL_DEEP
    elif near == 'bottom':
        top, bottom = STEEL_DEEP, STEEL_LIT
    elif near == 'left':
        top, bottom, axis = STEEL_LIT, STEEL_DEEP, [x, y, x + w - 1, y]
    elif near == 'right':
        top, bottom, axis = STEEL_DEEP, STEEL_LIT, [x, y, x + w - 1, y]
    pen.rect(x, y, w, h, top, ramp=ramp_between(top, bottom), axis=axis)
    pen.ops[-1].update(grain=grain, grain_size=1, grain_seed=seed)
    return pen


def brushed(pen, x, y, w, h, *, seed=0, density=14):
    """The horizontal scratch of brushed steel. One pixel of it is invisible
    and twenty of them are a material."""
    rnd = random.Random(seed * 31 + 7)
    for _ in range(max(1, w * h // density)):
        sx, sy = x + rnd.randrange(w), y + rnd.randrange(h)
        run = min(rnd.randint(2, 9), x + w - sx)
        pen.rect(sx, sy, run, 1, STEEL_EDGE if rnd.random() < .45 else STEEL_DEEP)
        pen.ops[-1].update(opacity=rnd.randint(22, 54))
    return pen


def slot(pen, x, y, w, h, *, falloff=3):
    """A slot of bare diffuser, and the light spilling out of it onto the metal
    around its edge. Nothing in this skin is lit from a direction; things are
    lit by how close they are to one of these."""
    pen.rect(x, y, w, h, GLOW)
    for i in range(1, falloff + 1):
        wash = hexcolor(mix(GLOW, STEEL, i / (falloff + 1)))
        strength = int(150 * (1 - (i - 1) / falloff) + 30)
        pen.rect(x - i, y - i, w + 2 * i, 1, wash)
        pen.ops[-1].update(opacity=strength)
        pen.rect(x - i, y + h - 1 + i, w + 2 * i, 1, wash)
        pen.ops[-1].update(opacity=strength)
        pen.rect(x - i, y - i, 1, h + 2 * i, wash)
        pen.ops[-1].update(opacity=strength)
        pen.rect(x + w - 1 + i, y - i, 1, h + 2 * i, wash)
        pen.ops[-1].update(opacity=strength)
    return pen


# ------------------------------------------------------------------- the film


def film(pen, x, y, w, h, *, seed=0, base=AIR, top=None, seat=True, grain=5):
    """A sheet of film lying on the diffuser.

    The beam is not even across a sheet: the middle of the field is hotter than
    the corners, so a real film is a shade denser at its edges. That is drawn
    as the ramp, not as an outline -- an outlined rectangle reads as a window
    and a ramped one reads as a sheet.
    """
    top = top or thinner(base, 1)
    pen.rect(x, y, w, h, top, ramp=ramp_between(top, base),
             axis=[x, y, x, y + h - 1])
    pen.ops[-1].update(grain=grain, grain_size=1, grain_seed=seed)
    if seat:
        # Film is thicker where it is doubled over its own cut edge, and the
        # emulsion stops a pixel short of it.
        pen.rect(x, y, w, 1, denser(base, 1))
        pen.ops[-1].update(opacity=120)
        pen.rect(x, y + h - 1, w, 1, darken(base, .4))
        pen.rect(x, y, 1, h, denser(base, 1))
        pen.ops[-1].update(opacity=120)
        pen.rect(x + w - 1, y, 1, h, darken(base, .4))
    return pen


def chip(pen, x, y, w, h, *, pressed=False, seed=0, base=AIR):
    """One small film chip clipped to the box: the shape every button in this
    skin is cut from.

    Released, it stands a hair off the diffuser, so there is a rim of escaping
    light down its left and top and its own shadow under its right and bottom.
    Pressed, it is flat against the light: the shadow goes, the leak closes to
    a hairline, and the whole chip comes up a step because there is less air
    under it. Flipping a highlight would be two pixels of difference across a
    23x18 cell, which is no difference at all.
    """
    if pressed:
        pen.rect(x - 1, y - 1, w + 2, h + 2, GLOW)
        pen.ops[-1].update(opacity=210)
        film(pen, x, y, w, h, seed=seed, base=thinner(base, 1), seat=False,
             grain=4)
        pen.rect(x, y, w, 1, denser(base, 1))
        pen.ops[-1].update(opacity=90)
    else:
        # The gap it floats over, brightest where the light gets out.
        pen.rect(x - 1, y - 1, w + 2, h + 2, GLOW_DEEP)
        pen.rect(x - 1, y - 1, w + 1, 1, GLOW_DIM)
        pen.rect(x - 1, y - 1, 1, h + 1, GLOW_DIM)
        pen.rect(x, y + h, w + 1, 1, STEEL_DEEP)
        pen.rect(x + w, y, 1, h + 1, STEEL_DEEP)
        film(pen, x, y, w, h, seed=seed, base=base, seat=False, grain=4)
    return pen


def haze(pen, x, y, w, h, color=TISSUE, *, strength=150):
    """Soft tissue: no edge at all. An ellipse at part strength, and then a
    wider one at a third of it, because the thing that says `body` on a
    radiograph is the absence of a boundary."""
    pen.ellipse(x + 1, y + 1, max(1, w - 2), max(1, h - 2), color)
    pen.ops[-1].update(opacity=strength)
    pen.ellipse(x, y, w, h, color)
    pen.ops[-1].update(opacity=max(20, strength // 3))
    return pen


def bone_rows(rows, palette):
    """Three values from one drawn shape: the palette a bone grid uses.

    `o` is the dense rind, `x` the spongy middle, `.` the haze that spills past
    it. Writing three grids by hand is how a bone ends up with a rind on one
    side only.
    """
    return {'o': palette[0], 'x': palette[1], '.': palette[2]}


CORTICAL = (CORTEX, BONE, TISSUE_LIT)
SMALL_BONE = (BONE_LIT, BONE_DIM, TISSUE)


def trabecular(pen, x, y, w, h, *, seed=0, color=BONE_LIT, density=5):
    """The spongy inside of a bone: a speckle one step brighter than the body
    it sits in. Flat bone is a paper cut-out."""
    rnd = random.Random(seed * 17 + 3)
    for _ in range(max(1, w * h // density)):
        pen.pixel(x + rnd.randrange(w), y + rnd.randrange(h), color)
        pen.ops[-1].update(opacity=rnd.randint(40, 110))
    return pen


def bone(pen, points, width, color=BONE, *, rind=True, taper=False):
    """A long bone: the body, then the rind on the outline. `points` is
    (start, control, end) in the pen's own fractional construction space."""
    start, control, end = points
    if taper:
        pen.taper(start, control, end, width, color)
    else:
        pen.curve(start, control, end, width, color)
    if rind and width >= 3:
        pen.curve(start, control, end, max(1, width - 2), denser(color, 1))
    return pen


# -------------------------------------------------------- what the vet leaves


def grease(pen, points, color=WAX_RED, *, seed=0, width=2):
    """A grease-pencil mark. Wax on polyester does not take evenly: the line is
    two values and misses pixels, which is the only reason it reads as wax
    rather than as a drawn line."""
    pen.line(points, color, width)
    rnd = random.Random(seed * 13 + 5)
    start = points[0]
    for point in points[1:]:
        steps = int(max(abs(point[0] - start[0]), abs(point[1] - start[1]))) + 1
        for i in range(steps):
            t = i / max(1, steps - 1)
            px = start[0] + (point[0] - start[0]) * t
            py = start[1] + (point[1] - start[1]) * t
            if rnd.random() < .34:
                pen.pixel(int(px), int(py), lighten(color, .35))
                pen.ops[-1].update(opacity=rnd.randint(90, 180))
        start = point
    return pen


def tape(pen, x, y, w, h, *, seed=0, strength=70):
    """A strip of tape over the light: not opaque, so it is a wash rather than
    a shape, with two brighter lines where the roll creased it."""
    pen.rect(x, y, w, h, TAPE)
    pen.ops[-1].update(opacity=strength, grain=6, grain_size=2, grain_seed=seed)
    rnd = random.Random(seed * 11 + 2)
    for _ in range(2):
        if h > w:
            pen.rect(x + rnd.randrange(max(1, w)), y, 1, h, lighten(TAPE, .4))
        else:
            pen.rect(x, y + rnd.randrange(max(1, h)), w, 1, lighten(TAPE, .4))
        pen.ops[-1].update(opacity=strength // 2)
    return pen


def spring_clip(pen, x, y, w=9, h=7):
    """The clip that holds a film to the viewer: opaque, so a silhouette and
    one line of light escaping under its jaw, and nothing else."""
    pen.rect(x, y, w, h - 2, STEEL_DEEP)
    pen.rect(x + 1, y + h - 2, w - 2, 1, STEEL_DARK)
    pen.rect(x, y, w, 1, STEEL_EDGE)
    pen.rect(x + 1, y + h - 1, w - 2, 1, GLOW_DIM)
    pen.ops[-1].update(opacity=120)
    return pen


def plate(pen, x, y, w, h, *, seed=0, dim=False):
    """A clear strip in a film: where the beam never went, so the light comes
    through at full strength. This is where the *printed* part of a film lives
    -- the study header, the units beside a readout -- in black."""
    top = PLATE_DIM if dim else PLATE
    pen.rect(x, y, w, h, top, ramp=ramp_between(lighten(top, .10), top),
             axis=[x, y, x, y + h - 1])
    pen.ops[-1].update(grain=3, grain_size=1, grain_seed=seed)
    pen.rect(x, y, w, 1, darken(top, .22))
    pen.ops[-1].update(opacity=110)
    pen.rect(x, y + h - 1, w, 1, darken(top, .12))
    pen.ops[-1].update(opacity=90)
    return pen


def window(pen, x, y, w, h, *, seed=0, rule=True):
    """The other kind of window: the black one Cranamp writes its own light
    into. Fully exposed film, a hairline of clear stock above and below so it
    reads as a ruled field rather than as a hole, and nothing in between."""
    pen.rect(x, y, w, h, AIR)
    pen.ops[-1].update(grain=3, grain_size=1, grain_seed=seed)
    if rule:
        pen.rect(x, y, w, 1, CARTILAGE)
        pen.ops[-1].update(opacity=140)
        pen.rect(x, y + h - 1, w, 1, TISSUE_LIT)
        pen.ops[-1].update(opacity=120)
    return pen


def clipped(pen, x, y, w, h, *, seed=0, base=AIR, clips=()):
    """A film hung on the viewer: the hairline of light that escapes all round
    its edge, the film itself, and the clips holding it. A wide wash of glow
    round every film turns the case into a lamp -- the light belongs in the
    gap, which is one pixel wide."""
    pen.rect(x - 1, y - 1, w + 2, h + 2, GLOW_DEEP)
    pen.rect(x - 1, y - 1, w + 2, 1, GLOW_DIM)
    pen.rect(x - 2, y - 2, w + 4, 1, STEEL_LIT)
    pen.rect(x - 2, y + h + 1, w + 4, 1, STEEL_DEEP)
    film(pen, x, y, w, h, seed=seed, base=base)
    for cx in clips:
        spring_clip(pen, x + cx, y - 6)
    return pen


def lit_plate(pen, x, y, w, h, *, on, seed=0):
    """The skin's one switch: a legend plate with the tube behind it.

    On is not a brighter version of off; on is the light *through* it. Off is
    the same plate with the tube dark, which on a viewer is nearly the colour
    of the case it is set into -- so the two states differ by the whole range
    of the skin rather than by a bevel.
    """
    if on:
        pen.rect(x, y, w, h, GLOW, ramp=ramp_between(GLOW, GLOW_DIM),
                 axis=[x, y, x, y + h - 1])
        pen.ops[-1].update(grain=3, grain_size=1, grain_seed=seed)
        pen.rect(x, y, w, 1, lighten(GLOW, .6))
        pen.rect(x, y + h - 1, w, 1, GLOW_DEEP)
    else:
        pen.rect(x, y, w, h, STEEL_DARK, ramp=ramp_between(STEEL, STEEL_DEEP),
                 axis=[x, y, x, y + h - 1])
        pen.ops[-1].update(grain=4, grain_size=1, grain_seed=seed)
        pen.rect(x, y, w, 1, STEEL_LIT)
        pen.rect(x, y + h - 1, w, 1, STEEL_DEEP)
    return pen


def hair(pen, start, control, end, *, color=CAT_RIM, opacity=150):
    """A cat hair on the viewer. There is always a cat hair on the viewer."""
    pen.curve(start, control, end, 1, color)
    pen.ops[-1].update(opacity=opacity)
    return pen


def dust(pen, x, y, w, h, *, seed=0, count=None):
    """Dust on the diffuser, which is what tells you the box has been on all
    night in a room nobody has swept."""
    rnd = random.Random(seed * 23 + 9)
    for _ in range(count if count is not None else max(1, w * h // 240)):
        px, py = x + rnd.randrange(w), y + rnd.randrange(h)
        pen.pixel(px, py, STEEL_DEEP if rnd.random() < .6 else GLOW_DEEP)
        pen.ops[-1].update(opacity=rnd.randint(30, 90))
    return pen


def text_width(word, scale=1, face='small', spacing=1):
    """How wide a set word comes out. The default face is the small one,
    because it is the one everything in this skin is set in and because a
    measure that defaults to a different face than the thing it measures is a
    caption that ends ten pixels past where it was aimed."""
    cell = 4 if face == 'small' else 5
    return len(word) * (cell * scale + spacing) - spacing


def printed(pen, x, y, word, color=INK, *, face='small', scale=1, spacing=1):
    """Printed on a film: the field captions, the frequency scale, the words on
    the switch plates. Everything set on this viewer is set small, because
    every legend on real radiology kit is."""
    pen.ops.append(dict(op='text', x=int(x), y=int(y), text=word, color=color,
                        face=face, scale=scale, spacing=spacing))
    return pen


def right(pen, x, y, word, color=INK, *, face='small', scale=1, spacing=1):
    """Set a word ending at x. A classic skin's cells were measured for other
    artwork, and a caption laid out left to right runs off the end of half of
    them -- which nothing reports, because every pixel it lands on is a legal
    part of the sheet."""
    return printed(pen, x - text_width(word, scale, face, spacing), y, word,
                   color, face=face, scale=scale, spacing=spacing)


def centred(pen, x, y, w, word, color=INK, *, face='small', scale=1, spacing=1):
    """Set a word in the middle of a cell. Every legend in a classic skin lives
    in a cell it was not measured for, so the arithmetic belongs here once."""
    width = text_width(word, scale, face, spacing)
    return printed(pen, x + (w - width) // 2, y, word, color, face=face,
                   scale=scale, spacing=spacing)
