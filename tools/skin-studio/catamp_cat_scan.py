#!/usr/bin/env python3
"""Catamp Cat Scan -- the player as a vet's lightbox at two in the morning.

Somebody's cat has eaten eleven things it should not have, and the films are up
on the viewer. The whole skin is that viewer: a case of brushed steel with
fluorescent tubes behind frosted glass, films clipped to it, and the vet's
grease pencil still on the ones that matter. The controls are not buttons drawn
to look like objects; they are the objects. The equalizer handles are the
eleven foreign bodies, in the order they came out. The balance slider is the
cat's tail, which leans the way a cat's tail leans. The seek bar is a spine
survey with a loupe sliding along it, and the part of the study you have
already looked at is ringed in wax. A cat is sitting on the warm box in the
middle of the equalizer, with its back to you, because that is what cats do to
a lightbox.

    python3 tools/skin-studio/catamp_cat_scan.py             # build it all
    python3 tools/skin-studio/catamp_cat_scan.py main eq      # one stage

Requires a running `cranamp --skin-studio` and nothing else: no image library,
no external asset. Every pixel is a native Studio operation.
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import catscan_cats as cats
import catscan_film as F
from pixel_pen import Pen, answer, call

STAGES = {}


def stage(fn):
    STAGES[fn.__name__] = fn
    return fn


def sheet(name):
    call('studio_atlas', {'sheet': name})
    return Pen()


def variants(sprite):
    """Where every cell of a sprite actually lives, asked rather than worked
    out. Twenty-eight slider frames are at two different sheet rows with a
    stride that changes halfway, and a recipe that computes them lands all
    twenty-eight on frame 0 -- which is legal ink in a legal place, so nothing
    reports it and the control simply never changes."""
    found_in = answer('studio_targets', {'id': sprite, 'variants': True})
    for found in found_in['sprites']:
        if found['id'].endswith(sprite):
            return list(zip(found['labels'], found['variants']))
    raise KeyError(f'no sprite matching {sprite}')


def commit(pen, label):
    result = call('studio_draw', {'label': label, 'operations': pen.ops})
    note = result['content'][0]['text'] if result.get('content') else ''
    print(f'  {label}: {note}')
    return result


# ------------------------------------------------------------------ vocabulary


# The six transport marks, cut out of bone. Classic silhouettes on purpose: the
# skin takes its liberties with the sliders and the equalizer, where a wrong
# guess costs a second, and not with the six keys where it costs the record.
MARKS = {
    'prev': ('o    xx', 'o   Xxx', 'o  XXxx', 'o XXXxx', 'o  XXxx',
             'o   Xxx', 'o    xx'),
    'play': ('xx     ', 'xXx    ', 'xXXx   ', 'xXXXo  ', 'xXXx   ',
             'xXx    ', 'xx     '),
    'pause': ('xox  xox', 'xXx  xXx', 'xXx  xXx', 'xXx  xXx', 'xXx  xXx',
              'xXx  xXx', 'xox  xox'),
    'stop': ('ooooooo', 'oXXXXXo', 'oXXXXXo', 'oXXXXXo', 'oXXXXXo',
             'oXXXXXo', 'ooooooo'),
    'next': ('xx    o', 'xxX   o', 'xxXX  o', 'xxXXX o', 'xxXX  o',
             'xxX   o', 'xx    o'),
    'eject': ('   x   ', '  xXx  ', ' xXXXx ', 'xXXXXXx', '       ',
              'ooooooo', 'xXXXXXx'),
}
MARK_PALETTE = {'x': F.BONE, 'X': F.BONE_LIT, 'o': F.CORTEX}


def marked(pen, x, y, w, h, name, *, dim=False):
    """Centre a bone mark in a cell, with the haze a bone always has round it."""
    rows = MARKS[name]
    mx = x + (w - len(rows[0])) // 2
    my = y + (h - len(rows)) // 2
    grid = cats.Grid(len(rows[0]) + 2, len(rows) + 2)
    for ry, row in enumerate(rows):
        for rx, ch in enumerate(row):
            grid.put(rx + 1, ry + 1, ch)
    grid.halo(',')
    palette = dict(MARK_PALETTE, **{',': F.TISSUE_LIT if dim else F.CARTILAGE})
    pen.stamp(mx - 1, my - 1, grid.rows(), palette)
    return pen


def key_cell(pen, x, y, w, h):
    """Clear a sprite cell to the transparency key, so the box shows through
    wherever the control does not cover it."""
    pen.rect(x, y, w, h, F.KEY)
    return pen


def legend(pen, x, y, w, h, word, *, on, seed=0, scale=1):
    """The skin's one switch: a legend plate with a tube behind it.

    On is not a brighter off. On is the light coming *through* the plate, and
    off is the same plate with the tube dark, which on a steel case is nearly
    the colour of the case. Two states a whole skin's range apart.
    """
    F.lit_plate(pen, x, y, w, h, on=on, seed=seed)
    ink = F.INK if on else F.STEEL_EDGE
    F.centred(pen, x, y + (h - 5 * scale) // 2, w, word, ink, scale=scale)
    return pen


def pressed_legend(pen, x, y, w, h, word, *, on, seed=0, scale=1):
    """Pressed is pressed against the light: the plate comes up a step and the
    lip of the case above it stops casting into it."""
    legend(pen, x, y, w, h, word, on=on, seed=seed, scale=scale)
    pen.rect(x, y, w, h, F.GLOW)
    pen.ops[-1].update(opacity=64 if on else 40)
    pen.rect(x, y, w, 1, F.GLOW if on else F.STEEL_EDGE)
    ink = F.INK if on else F.GLOW_DEEP
    F.centred(pen, x, y + (h - 5 * scale) // 2, w, word, ink, scale=scale)
    return pen


# ---------------------------------------------------------------- the top rail


@stage
def titlebar():
    """The top of the case: a long slot of diffuser with the study header
    printed on a ribbon of film across it, and four small keys set into the
    steel beside it."""
    pen = sheet('titlebar.bmp')
    # The title cell is sheet [27,0,275,14], not [0,0,...]: the four window
    # keys live in the 27 columns before it. Drawing the bar from the sheet's
    # own left edge puts the whole header 27 pixels to the left of where it
    # will be seen and leaves the last 27 columns of it transparent -- and
    # nothing reports it, because every one of those pixels is a legal part of
    # some cell. studio_rectangles is the thing that settles it.
    for ox, row, lit in ((27, 0, True), (27, 15, False)):
        F.steel(pen, ox, row, 275, 14, seed=row, near='bottom')
        F.brushed(pen, ox, row, 275, 14, seed=row + 3, density=26)
        pen.rect(ox, row, 275, 1, F.STEEL_EDGE)
        pen.rect(ox, row + 13, 275, 1, F.STEEL_DEEP)
        if lit:
            F.clipped(pen, ox + 18, row + 3, 220, 8, seed=row + 7,
                      base=F.TISSUE_DEEP)
            F.plate(pen, ox + 20, row + 4, 78, 6, seed=row + 11)
            F.printed(pen, ox + 22, row + 4, 'CATAMP CAT SCAN', F.INK)
            F.printed(pen, ox + 108, row + 4, 'LAT / 44KVP / 2.5MAS',
                      F.BONE_LIT)
            F.printed(pen, ox + 214, row + 4, 'ON', F.METAL)
            F.dust(pen, ox + 18, row + 3, 220, 8, seed=row + 2, count=9)
        else:
            # The tube is off. A viewer with no light in it is a black slot in
            # a steel case, and that is the whole difference between the two
            # states -- not a duller blue for the title.
            pen.rect(ox + 17, row + 2, 222, 10, F.STEEL_DEEP)
            pen.rect(ox + 17, row + 2, 222, 1, F.STEEL_DARK)
            pen.rect(ox + 17, row + 11, 222, 1, F.STEEL_DARK)
            F.printed(pen, ox + 22, row + 4, 'CATAMP CAT SCAN', F.STEEL_EDGE)
            F.printed(pen, ox + 108, row + 4, 'LAT / 44KVP / 2.5MAS', F.STEEL)
    # Four keys set into the steel. Released they are chips floating on the
    # case; pressed they are flat against it and lit from behind.
    keys = (('options', 0, 0, '|||'), ('minimize', 9, 0, '__'),
            ('close', 18, 0, 'X'), ('shade', 0, 18, '^'))
    glyphs = {
        '|||': (' x x x ', ' x x x ', ' x x x '),
        '__': ('       ', '       ', 'xxxxxxx'),
        'X': ('x     x', ' x   x ', '  x x  ', '   x   ', '  x x  ',
              ' x   x ', 'x     x'),
        '^': ('   x   ', '  xxx  ', ' xx xx ', 'xx   xx'),
    }
    for name, kx, ky, glyph in keys:
        px = kx + (9 if name == 'shade' else 0)
        py = ky + (0 if name == 'shade' else 9)
        for x, y, down in ((kx, ky, False), (px, py, True)):
            key_cell(pen, x, y, 9, 9)
            if down:
                pen.rect(x, y, 9, 9, F.GLOW_DIM)
                pen.rect(x + 1, y + 1, 7, 7, F.GLOW)
                ink = F.INK
            else:
                F.steel(pen, x, y, 9, 9, seed=x + y, near='top')
                pen.rect(x, y, 9, 1, F.STEEL_EDGE)
                pen.rect(x, y + 8, 9, 1, F.STEEL_DEEP)
                ink = F.GLOW_DIM
            rows = glyphs[glyph]
            gx = x + (9 - len(rows[0])) // 2
            gy = y + (9 - len(rows)) // 2
            pen.stamp(gx, gy, list(rows), {'x': ink})
    commit(pen, 'Cat Scan - the case lid, lit and dark')


# ------------------------------------------------------------- the main window


@stage
def main():
    """The viewer itself: two films clipped to a steel case, a channel of
    diffuser under the sliders, and a cat's tail over the far corner."""
    pen = sheet('main.bmp')
    F.steel(pen, 0, 0, 275, 115, seed=1)
    F.brushed(pen, 0, 0, 275, 115, seed=2)

    # --- the rails first, then the films over them. The spectrum runs from
    # row 43 to row 58 and the volume slider from 57 to 69, side by side rather
    # than stacked, so the slider channel is only the right of the window: a
    # rail drawn the whole way across cut five rows off the bottom of the chest
    # and left the spectrum rising out of a steel bar.
    F.steel(pen, 102, 52, 173, 20, seed=3, near='top')
    F.brushed(pen, 102, 52, 173, 20, seed=4, density=30)
    F.slot(pen, 105, 55, 112, 16, falloff=1)
    F.film(pen, 106, 56, 110, 14, seed=16, base=F.FOG, seat=False)
    F.steel(pen, 0, 61, 108, 11, seed=17, near='top')
    F.brushed(pen, 0, 61, 108, 11, seed=18, density=30)
    F.printed(pen, 4, 64, 'VIEWER 2 / BANK B', F.STEEL_EDGE)

    # --- the study film: head and chest, lateral. Everything Cranamp draws
    # live on the left of this window is drawn inside this one radiograph --
    # the timer in the gaps between the vertebrae, the spectrum in the chest.
    F.clipped(pen, 2, 16, 104, 44, seed=11, clips=(20, 62))
    F.printed(pen, 4, 17, 'THORAX / LAT / 0441', F.BONE_DIM)
    cats.skull_lateral(26, 20).stamp(pen, 3, 22)
    for i in range(4):
        cats.vertebra(7, 8, lean=.25).stamp(pen, 25 + i * 5, 26 + i)
    cats.rib_cage(pen, 28, 30, 74, 28, spine=32, ribs=10, seed=5)
    for i in range(10):
        cats.vertebra(8, 9, lean=0).stamp(pen, 28 + i * 7, 24)
    # the diaphragm: the one line on a chest film that says where it stops
    pen.curve((28, 54), (64, 62), (102, 50), 1, F.BONE_DIM)
    pen.ops[-1].update(opacity=190)
    F.printed(pen, 96, 53, 'L', F.METAL, face='5x7')

    # --- the report film. Everything Cranamp writes live is written into one of
    # the two black windows in it, in the brightest colour in the skin.
    F.clipped(pen, 108, 16, 160, 35, seed=12, base=F.TISSUE_DEEP,
              clips=(24, 120))
    F.printed(pen, 110, 18, 'PATIENT', F.BONE_DIM)
    F.right(pen, 266, 18, 'STUDY 0441 / FELINE', F.BONE_DIM)
    F.window(pen, 109, 25, 158, 12, seed=13)
    F.window(pen, 109, 39, 100, 11, seed=14)
    F.printed(pen, 131, 42, 'KVP', F.BONE_DIM)
    F.printed(pen, 170, 42, 'MAS', F.BONE_DIM)
    F.tape(pen, 250, 12, 14, 9, seed=15)

    # --- the rail the spine survey is clipped to
    F.steel(pen, 0, 72, 275, 16, seed=5, near='top')
    F.brushed(pen, 0, 72, 275, 16, seed=6, density=30)
    F.slot(pen, 16, 71, 250, 12, falloff=1)
    pen.rect(17, 72, 248, 10, F.AIR)
    F.printed(pen, 4, 83, 'SPINE SURVEY C1-CD12', F.STEEL_EDGE)
    F.right(pen, 272, 83, 'WAX MARKS WHAT HAS BEEN READ', F.STEEL)

    # --- the bottom rail, with the transport chips seated in their own slot
    F.steel(pen, 0, 88, 275, 27, seed=7, near='top')
    F.brushed(pen, 0, 88, 275, 27, seed=8, density=22)
    F.slot(pen, 15, 87, 144, 20, falloff=1)
    pen.rect(0, 114, 275, 1, F.STEEL_DEEP)
    F.printed(pen, 4, 108, 'CATAMP VETERINARY', F.STEEL_EDGE)
    F.right(pen, 272, 108, 'DO NOT REMOVE FILMS FROM THIS ROOM', F.STEEL)

    # --- the far corner, where a tail has been draped over the warm case.
    # Clicking it changes the skin, which is exactly the sort of thing a tail
    # left on a control panel does.
    pen.taper((274, 82), (256, 86), (250, 108), 5, F.CAT)
    pen.taper((251, 106), (248, 112), (256, 113), 3, F.CAT)
    F.hair(pen, (262, 88), (258, 92), (255, 99), opacity=120)
    F.hair(pen, (268, 90), (264, 96), (262, 104), opacity=90)
    # and the hair and the dust that are always on a viewer that has been on
    # all night in a room nobody sweeps
    F.hair(pen, (12, 20), (8, 34), (14, 48))
    F.hair(pen, (200, 60), (214, 63), (230, 58), opacity=90)
    cats.paw_print(pen, 92, 96, opacity=70)
    F.dust(pen, 0, 14, 275, 100, seed=9)
    commit(pen, 'Cat Scan - the viewer, its films and the tail on the corner')


@stage
def readouts():
    """The timer, the status marker and the two channel lamps."""
    # Bone numerals, with a dim rind so a digit still separates from the ribs
    # it is standing in front of.
    pen = sheet('numbers.bmp')
    for (label, rect), rows in zip(variants('main.digit0'), DIGITS):
        # The labels say which digit each cell is, so the grids can be checked
        # against them rather than trusted to be in order.
        assert label == str(DIGITS.index(rows)), f'{label} is not {rows[0]!r}'
        x, y, w, h = rect
        grid = cats.Grid(w, h)
        for ry, row in enumerate(rows):
            for rx, ch in enumerate(row):
                if ch != ' ':
                    grid.put(rx + 1, ry + 1, 'o' if ch == '#' else 'X')
        grid.halo('=')
        grid.halo(',')
        key_cell(pen, x, y, w, h)
        grid.stamp(pen, x, y)
    commit(pen, 'Cat Scan - the exposure counter, cut out of bone')

    # Three lead markers: the little chips a radiographer drops on a film.
    pen = sheet('playpaus.bmp')
    for label, (x, y, w, h) in variants('main.status'):
        key_cell(pen, x, y, w, h)
        pen.rect(x, y, w, h, F.STEEL_DEEP)
        pen.rect(x, y, w, 1, F.STEEL_EDGE)
        pen.rect(x, y + h - 1, w, 1, F.STEEL_DEEP)
        marked(pen, x + 1, y + 1, w - 2, h - 2,
               {'playing': 'play', 'paused': 'pause'}.get(label, 'stop'),
               dim=True)
    commit(pen, 'Cat Scan - the lead markers')

    # The channel lamps: two tubes behind two legend plates.
    pen = sheet('monoster.bmp')
    for x, w, word in ((0, 29, 'STEREO'), (29, 27, 'MONO')):
        for y, on in ((0, True), (12, False)):
            key_cell(pen, x, y, w, 12)
            legend(pen, x, y, w, 12, word, on=on, seed=x + y)
    commit(pen, 'Cat Scan - the channel lamps')


DIGITS = (
    (" ##### ", "##   ##", "##   ##", "##   ##", "##   ##", "##   ##",
     "##   ##", "##   ##", "##   ##", "##   ##", " ##### "),
    ("   ##  ", "  ###  ", " ####  ", "   ##  ", "   ##  ", "   ##  ",
     "   ##  ", "   ##  ", "   ##  ", "   ##  ", " ######"),
    (" ##### ", "##   ##", "##   ##", "     ##", "    ## ", "   ##  ",
     "  ##   ", " ##    ", "##     ", "##     ", "#######"),
    (" ##### ", "##   ##", "     ##", "     ##", "  #### ", "     ##",
     "     ##", "     ##", "##   ##", "##   ##", " ##### "),
    ("     ##", "    ###", "   ####", "  ## ##", " ##  ##", "##   ##",
     "#######", "#######", "     ##", "     ##", "     ##"),
    ("#######", "##     ", "##     ", "##     ", "###### ", "     ##",
     "     ##", "     ##", "##   ##", "##   ##", " ##### "),
    ("  #### ", " ##  ##", "##     ", "##     ", "###### ", "##   ##",
     "##   ##", "##   ##", "##   ##", "##   ##", " ##### "),
    ("#######", "#######", "     ##", "    ## ", "    ## ", "   ##  ",
     "   ##  ", "  ##   ", "  ##   ", " ##    ", " ##    "),
    (" ##### ", "##   ##", "##   ##", "##   ##", " ##### ", "##   ##",
     "##   ##", "##   ##", "##   ##", "##   ##", " ##### "),
    (" ##### ", "##   ##", "##   ##", "##   ##", "##   ##", " ######",
     "     ##", "     ##", "     ##", "##   ##", " ##### "),
)


@stage
def transport():
    """Six film chips on the bottom rail, and the pressed row underneath."""
    pen = sheet('cbuttons.bmp')
    # Five of the six pressed cells are one row of 18 below their released one
    # and eject's is not: it is 22x16, and its pressed cell starts at y=16
    # rather than y=18. Assuming the stride put eject's pressed art two rows
    # low, which is legal ink in a legal sheet -- `unsampled_pixels` named the
    # two rows that fell off the bottom, and that is the only thing that did.
    for index, (sprite, mark) in enumerate((
            ('main.previous', 'prev'), ('main.play', 'play'),
            ('main.pause', 'pause'), ('main.stop', 'stop'),
            ('main.next', 'next'), ('main.eject', 'eject'))):
        for label, (x, y, w, h) in variants(sprite):
            down = label == 'pressed'
            key_cell(pen, x, y, w, h)
            F.chip(pen, x + 1, y + 1, w - 2, h - 2, pressed=down,
                   seed=index * 3 + (1 if down else 0))
            marked(pen, x + 1, y + 1, w - 2, h - 2, mark, dim=down)
            # every chip in a study carries its own number in the corner
            F.printed(pen, x + w - 5, y + h - 6, str(index + 1),
                      F.CARTILAGE if down else F.BONE_DIM)
    commit(pen, 'Cat Scan - six chips of film, and the same six pressed flat')


@stage
def switches():
    """Shuffle, repeat and the two window keys: legend plates with tubes."""
    pen = sheet('shufrep.bmp')
    for x, w, word in ((0, 28, 'REPEAT'), (28, 47, 'SHUFFLE')):
        for i, (on, down) in enumerate(((False, False), (False, True),
                                        (True, False), (True, True))):
            y = i * 15
            key_cell(pen, x, y, w, 15)
            if down:
                pressed_legend(pen, x, y + 1, w, 13, word, on=on, seed=x + y)
            else:
                legend(pen, x, y + 1, w, 13, word, on=on, seed=x + y)
    for x, word in ((0, 'EQ'), (23, 'PL')):
        for (dx, dy), (on, down) in zip(((0, 61), (46, 61), (0, 73), (46, 73)),
                                        ((False, False), (False, True),
                                         (True, False), (True, True))):
            key_cell(pen, x + dx, dy, 23, 12)
            if down:
                pressed_legend(pen, x + dx, dy, 23, 12, word, on=on,
                               seed=x + dx + dy)
            else:
                legend(pen, x + dx, dy, 23, 12, word, on=on, seed=x + dx + dy)
    commit(pen, 'Cat Scan - the legend plates and their tubes')


# --------------------------------------------------------------- the surfaces


@stage
def blank():
    """Thirteen transparent classic atlases and nothing inherited.

    The recipe says what it starts from rather than taking whatever the editor
    happens to have open: run against the bundled Catamp it would have drawn
    this skin on top of Silverplay's seven painting planes, which is both wrong
    and slow -- every stroke composites all seven.
    """
    print(' ', answer('studio_new', {'discard': True})['message'])


@stage
def options():
    """Four of the skin's ideas are skin options rather than artwork, and three
    of them create a sheet to draw on, so they go first: eleven equalizer
    handles of their own, a bitmap under the track list, artwork for the
    playing row, and a visualizer whose unlit pixels are transparent -- which
    is what lets the spectrum rise inside a ribcage instead of inside a box."""
    changed = answer('studio_options', {
        'eq_handles': True, 'playlist_background': True,
        'playlist_selection': True, 'visualizer_glass': True})
    for sheet_note in changed.get('sheets_changed') or []:
        print(' ', sheet_note)


# -------------------------------------------------------------- the seek bar


@stage
def seek():
    """A spine survey, nose to tail, with a loupe sliding along it.

    Twenty-eight frames of a slider track say something a handle cannot, and
    this one is the other way round: the track is one long film of the whole
    animal and the handle is the thing being moved over it, so the position of
    the song is literally how far down the cat you have read.
    """
    pen = sheet('posbar.bmp')
    key_cell(pen, 0, 0, 248, 10)
    F.film(pen, 0, 0, 248, 10, seed=41, base=F.AIR, seat=False)
    pen.rect(0, 0, 248, 1, F.CARTILAGE)
    pen.ops[-1].update(opacity=120)
    pen.rect(0, 9, 248, 1, F.TISSUE_LIT)
    pen.ops[-1].update(opacity=110)
    # thirty vertebrae, tapering from the neck at the left to the tail tip at
    # the right, with the soft tissue of the back over the top of them
    for i in range(30):
        x = 3 + i * 8.2
        size = 8 - 4.4 * (i / 29)
        F.haze(pen, int(x - 1), 2, int(size) + 3, 6, F.TISSUE, strength=90)
        pen.ellipse(int(x), 4, max(2, int(size)), 4, F.BONE)
        pen.rect(int(x + size / 2) - 1, 2, 2, 3, F.BONE_LIT)
        if i % 4 == 0:
            pen.pixel(int(x), 3, F.CORTEX)
    # the vet has been through the first third of it and left the wax on
    F.grease(pen, [[6, 1], [70, 2], [74, 8]], F.WAX_RED, seed=3, width=1)

    # the loupe. It is the one piece of glass in a skin made of film and steel,
    # and the engine's own glass material is what a loupe is for.
    for x, dragged in ((248, False), (278, True)):
        key_cell(pen, x, 0, 29, 10)
        pen.rect(x + 1, 0, 27, 10, F.STEEL_DEEP)
        pen.rect(x + 2, 1, 25, 8, F.AIR)
        # a vertebra, seen at twice the size through the lens
        pen.ellipse(x + 10, 2, 9, 6, F.BONE_LIT)
        pen.ellipse(x + 12, 3, 5, 4, F.CORTEX)
        pen.rect(x + 13, 1, 3, 3, F.BONE)
        pen.ops.append(dict(op='ellipse', x=x + 2, y=1, width=25, height=8,
                            fill=True, color=F.GLOW_DIM, material='glass',
                            bevel=3, refraction=2,
                            opacity=150 if dragged else 110))
        pen.rect(x + 1, 0, 27, 1, F.STEEL_EDGE if dragged else F.STEEL_LIT)
        pen.rect(x + 1, 9, 27, 1, F.STEEL_DEEP)
        pen.rect(x, 3, 1, 4, F.STEEL_LIT if dragged else F.STEEL_DARK)
        pen.rect(x + 28, 3, 1, 4, F.STEEL_LIT if dragged else F.STEEL_DARK)
        if dragged:
            # pressed against the light, like every other control here
            pen.rect(x + 2, 1, 25, 8, F.GLOW)
            pen.ops[-1].update(opacity=40)
    commit(pen, 'Cat Scan - the spine survey, and the loupe on it')


# --------------------------------------------------------------- the sliders


@stage
def sliders():
    """Volume is a step wedge. Balance is the tail."""
    # A step wedge is the aluminium staircase a radiographer puts in the beam
    # to calibrate an exposure: thirteen steps, each denser than the last. The
    # level is how many of them the beam has got through, so silence is a strip
    # of unexposed film and full volume is the whole wedge burned out.
    pen = sheet('volume.bmp')
    steps = 13
    for index, (_, rect) in enumerate(variants('volume.track')):
        x, y, w, h = rect
        key_cell(pen, x, y, w, h)
        F.film(pen, x, y, w, h, seed=50 + index, base=F.FOG, seat=False,
               grain=3)
        pen.rect(x, y, w, 1, F.TISSUE_LIT)
        pen.ops[-1].update(opacity=110)
        lit = round((index + 1) * steps / 28)
        for i in range(steps):
            sx = x + 1 + i * 5
            if i < lit:
                shade = F.LADDER[5 + round(6 * i / (steps - 1))]
                pen.rect(sx, y + 2, 4, h - 4, shade)
                pen.rect(sx, y + 2, 4, 1, F.thinner(shade, 1))
            else:
                pen.rect(sx, y + 3, 4, h - 6, F.TISSUE_DEEP)
            pen.rect(sx + 4, y + 2, 1, h - 4, F.AIR)
    marker(pen, 'R')
    commit(pen, 'Cat Scan - the step wedge, exposed step by step')

    # A cat says what it thinks with its tail, and a balance slider says the
    # same thing with the same gesture: six caudal vertebrae leaning together,
    # upright in the middle with no mark needed to say so.
    pen = sheet('balance.bmp')
    for index, (_, rect) in enumerate(variants('balance.track')):
        x, y, w, h = rect
        key_cell(pen, x, y, w, h)
        F.film(pen, x, y, w, h, seed=80 + index, base=F.AIR, seat=False,
               grain=3)
        lean = (index - 13.5) / 13.5
        # the soft tissue of the tail, which bends further than the bone does
        pen.curve((x + w / 2 - lean * 2, y + h - 3),
                  (x + w / 2 + lean * 9, y + h / 2),
                  (x + w / 2 + lean * 15, y + 2), 4, F.TISSUE)
        pen.ops[-1].update(opacity=170)
        for i in range(6):
            t = i / 5
            vx = x + w / 2 + lean * 15 * t
            vy = y + h - 2 - t * (h - 5)
            cats.vertebra(5, 4, lean=lean).stamp(pen, round(vx) - 2, round(vy) - 2)
    marker(pen, 'L')
    commit(pen, 'Cat Scan - the tail, leaning')


def marker(pen, letter):
    """The slider handle: a lead marker chip. Every film a radiographer takes
    has one of these dropped on it to say which side of the animal is which,
    and it is the only opaque thing small enough to be a slider handle."""
    for _, rect in variants('volume.thumb' if letter == 'R' else 'balance.thumb'):
        x, y, w, h = rect
        pressed = x == 0
        key_cell(pen, x, y, w, h)
        pen.rect(x, y, w, h, F.STEEL_DEEP)
        pen.rect(x + 1, y + 1, w - 2, h - 2, F.STEEL_DARK)
        pen.rect(x, y, w, 1, F.STEEL_EDGE if pressed else F.STEEL_LIT)
        pen.rect(x, y + h - 1, w, 1, F.STEEL_DEEP)
        if pressed:
            pen.rect(x, y, w, h, F.GLOW)
            pen.ops[-1].update(opacity=46)
        F.centred(pen, x, y + 3, w, letter, F.METAL)
    return pen


# ------------------------------------------------------------- the equalizer


BANDS = ('60', '170', '310', '600', '1K', '3K', '6K', '12K', '14K', '16K')


@stage
def eq():
    """Eleven bands of gut with eleven things in them, and a cat sitting on the
    warm part of the box in the gap the preamp leaves."""
    pen = sheet('eqmain.bmp')
    F.steel(pen, 0, 0, 275, 116, seed=20)
    F.brushed(pen, 0, 0, 275, 116, seed=21)
    # the film the eleven bands are cut into -- in two pieces, because the gap
    # the preamp leaves is not film at all: it is bare diffuser, and a cat.
    F.clipped(pen, 12, 36, 24, 66, seed=22, clips=(8,))
    F.clipped(pen, 76, 36, 186, 66, seed=23, clips=(10, 150))
    F.slot(pen, 38, 34, 36, 70, falloff=2)
    # the graph window's surround, and the scale down the left of the bands
    F.printed(pen, 2, 39, '+12', F.STEEL_EDGE)
    F.printed(pen, 4, 66, '0', F.STEEL_EDGE)
    F.printed(pen, 2, 93, '-12', F.STEEL_EDGE)
    for i, caption in enumerate(BANDS):
        F.centred(pen, 78 + i * 18, 104, 14, caption, F.STEEL_EDGE)
    F.centred(pen, 14, 104, 28, 'PRE', F.STEEL_EDGE)
    F.right(pen, 272, 110, 'FOREIGN BODIES / 11 RECOVERED', F.STEEL)

    # A cat, on the warm box, with its back to you. Everything else in this
    # skin is something seen through; this is the one thing the light stops at,
    # which is why it sits in front of bare diffuser and not in front of a
    # film. Drawn on the film it was a black cat on a black radiograph, which
    # is a perfectly correct silhouette of nothing.
    F.dust(pen, 38, 34, 36, 70, seed=24, count=14)
    cats.sitting_cat(pen, 38, 42, 36, 60)
    F.hair(pen, (37, 64), (33, 72), (36, 82), opacity=140)

    # the specimen jar on the right of the bank: what came out last time
    F.slot(pen, 258, 44, 14, 52, falloff=1)
    pen.rect(259, 45, 12, 50, F.STEEL_DEEP)
    pen.rect(259, 66, 12, 29, F.FOG)
    pen.rect(259, 45, 12, 4, F.STEEL_EDGE)
    for i, (_, rows) in enumerate(cats.foreign_bodies()[:3]):
        pen.stamp(261, 70 + i * 8, list(rows)[:5], {'#': F.METAL, '=': F.BONE,
                                                    ',': F.CARTILAGE})
    F.printed(pen, 259, 97, 'JAR', F.STEEL_EDGE)

    # the two title rows, lit and dark, exactly as the main window's are
    for row, lit in ((134, True), (149, False)):
        F.steel(pen, 0, row, 275, 14, seed=row, near='bottom')
        F.brushed(pen, 0, row, 275, 14, seed=row + 3, density=26)
        pen.rect(0, row, 275, 1, F.STEEL_EDGE)
        pen.rect(0, row + 13, 275, 1, F.STEEL_DEEP)
        if lit:
            F.clipped(pen, 76, row + 3, 130, 8, seed=row, base=F.TISSUE_DEEP)
            F.plate(pen, 78, row + 4, 62, 6, seed=row + 1)
            F.printed(pen, 80, row + 4, 'FILTER BANK', F.INK)
            F.printed(pen, 146, row + 4, '11 CHANNEL', F.BONE_LIT)
        else:
            pen.rect(75, row + 2, 132, 10, F.STEEL_DEEP)
            F.printed(pen, 80, row + 4, 'FILTER BANK', F.STEEL_EDGE)

    # the window key and the three legend plates
    for y, down in ((116, False), (125, True)):
        key_cell(pen, 0, y, 9, 9)
        if down:
            pen.rect(0, y, 9, 9, F.GLOW_DIM)
            pen.rect(1, y + 1, 7, 7, F.GLOW)
            ink = F.INK
        else:
            F.steel(pen, 0, y, 9, 9, seed=y, near='top')
            pen.rect(0, y, 9, 1, F.STEEL_EDGE)
            ink = F.GLOW_DIM
        pen.stamp(1, y + 1, ['x     x', ' x   x ', '  x x  ', '   x   ',
                             '  x x  ', ' x   x ', 'x     x'], {'x': ink})
    plates = (('on', 'ON', 26, ((10, False, False), (128, False, True),
                                (69, True, False), (187, True, True))),
              ('auto', 'AUTO', 32, ((36, False, False), (154, False, True),
                                    (95, True, False), (213, True, True))))
    for _, word, w, cells in plates:
        for x, on, down in cells:
            key_cell(pen, x, 119, w, 12)
            if down:
                pressed_legend(pen, x, 119, w, 12, word, on=on, seed=x)
            else:
                legend(pen, x, 119, w, 12, word, on=on, seed=x)
    for y, down in ((164, False), (176, True)):
        key_cell(pen, 224, y, 44, 12)
        if down:
            pressed_legend(pen, 224, y, 44, 12, 'PRESETS', on=True, seed=y)
        else:
            legend(pen, 224, y, 44, 12, 'PRESETS', on=False, seed=y)

    # the curve window: fully exposed film with a faint grid on it, so the
    # equalizer's own line is drawn on something rather than into a hole
    key_cell(pen, 0, 294, 113, 19)
    F.window(pen, 0, 294, 113, 19, seed=23, rule=False)
    for gx in range(0, 113, 8):
        pen.rect(gx, 294, 1, 19, F.TISSUE_DEEP)
    for gy in range(294, 313, 6):
        pen.rect(0, gy, 113, 1, F.TISSUE_DEEP)
    key_cell(pen, 0, 314, 113, 1)
    pen.rect(0, 314, 113, 1, F.CARTILAGE)

    # eleven tubes, each filling with contrast from the bottom up to its
    # handle: a boosted band has more in it than a cut one, which is the one
    # thing twenty-eight frames of a track can say and a handle cannot.
    for index, (_, rect) in enumerate(variants('band0.track')):
        x, y, w, h = rect
        key_cell(pen, x, y, w, h)
        F.film(pen, x, y, w, h, seed=200 + index, base=F.AIR, seat=False,
               grain=3)
        pen.rect(x + 1, y, 1, h, F.TISSUE_LIT)
        pen.rect(x + w - 2, y, 1, h, F.TISSUE_LIT)
        top = round((27 - index) * 38 / 27) + 25
        if top < h - 1:
            shade = F.LADDER[4 + round(6 * index / 27)]
            pen.rect(x + 2, y + top, w - 4, h - top, shade)
            pen.rect(x + 2, y + top, w - 4, 1, F.thinner(shade, 2))
            F.trabecular(pen, x + 2, y + top, w - 4, h - top,
                         seed=index, color=F.thinner(shade, 1), density=9)
        for tick in range(4, h, 8):
            pen.rect(x + w - 3, y + tick, 2, 1, F.TISSUE_LIT)
    commit(pen, 'Cat Scan - the filter bank, and the cat on the warm box')

    # Eleven handles of their own. Eleven copies of one head is a pattern;
    # eleven different objects is a tray after a surgery.
    pen = sheet('eqhandles.bmp')
    bodies = cats.foreign_bodies()
    for band in range(11):
        x = band * 14
        for y, down in ((0, False), (25, True)):
            key_cell(pen, x, y, 14, 25)
            pen.rect(x + 1, y, 12, 25, F.AIR)
            pen.rect(x + 1, y, 1, 25, F.TISSUE_LIT)
            pen.rect(x + 12, y, 1, 25, F.TISSUE_LIT)
            if down:
                pen.rect(x + 1, y, 12, 25, F.GLOW)
                pen.ops[-1].update(opacity=44)
            # No grip bar across the middle: the object *is* the handle, and a
            # steel bar laid over it turned eleven different things into
            # eleven identical grips with a smudge on each.
            pen.rect(x + 1, y, 12, 1, F.CARTILAGE)
            pen.rect(x + 1, y + 24, 12, 1, F.CARTILAGE)
            pen.rect(x, y + 1, 1, 23, F.STEEL_DEEP)
            pen.rect(x + 13, y + 1, 1, 23, F.STEEL_DEEP)
            # the mass the object makes in the tissue around it, which is what
            # a foreign body actually looks like and what makes the handle read
            # as something to take hold of rather than as more tube
            F.haze(pen, x + 1, y + 2, 12, 21, F.TISSUE_LIT, strength=130)
            if band == 0:
                cats.fish_bone(pen, x + 1, y + 9)
                continue
            rows = bodies[band - 1][1]
            ox = x + (14 - len(rows[0])) // 2
            oy = y + (25 - len(rows)) // 2
            pen.stamp(ox, oy, list(rows), {'#': F.METAL, '=': F.BONE_LIT,
                                           ',': F.CARTILAGE})
    commit(pen, 'Cat Scan - eleven things that were swallowed')


# --------------------------------------------------------------- the playlist


def flap(canvas_x):
    """The footer is two cells a pixel apart: sheet 0..124 is drawn at canvas
    0..124 and sheet 126..275 at canvas 125..274, so sheet column 125 is never
    drawn by anything. Nothing continuous may cross it."""
    return canvas_x if canvas_x < 125 else canvas_x + 1


def leader(pen, x, y, w, h, canvas_x, *, seed=0):
    """A length of film leader: the perforated edge of the strip.

    The header tile is 25 pixels wide and repeats nine times, so a pattern
    whose period does not divide 25 shifts at every repeat and reads as a row
    of seams. Five divides it, and the phase is taken from the *canvas*
    coordinate, so the title cell -- which starts at canvas 87 -- puts its
    holes in step with the tiles either side of it rather than two pixels out.
    """
    F.film(pen, x, y, w, h, seed=seed, base=F.FOG, seat=False, grain=3)
    pen.rect(x, y, w, 1, F.STEEL_EDGE)
    pen.rect(x, y + h - 1, w, 1, F.STEEL_DEEP)
    for i in range(w):
        if (canvas_x + i) % 5 < 3:
            pen.rect(x + i, y + 3, 1, 4, F.GLOW)
            pen.rect(x + i, y + 7, 1, 1, F.GLOW_DEEP)
    return pen


@stage
def playlist():
    """The case notes: a strip of film leader across the top, the film's own
    perforated edges down the sides, a steel rail along the bottom, and a whole
    cat asleep on the study title."""
    pen = sheet('pledit.bmp')
    header = ((0, 25, 0), (127, 25, 25), (26, 100, 87), (153, 25, 250))
    for row in (0, 21):
        for sx, w, cx in header:
            key_cell(pen, sx, row, w, 20)
            leader(pen, sx, row, w, 20, cx, seed=sx + row)
        # the study title, and a cat asleep on top of it
        # The title cell is sheet 26..125 and the repeating tile starts at
        # 127. A caption that runs past 125 does not spill into empty sheet:
        # it lands in the tile, which the player draws nine times across the
        # header. The first one read CASE NOTES with SCAN SCAN SCAN either
        # side of it, and nothing reported a thing, because every pixel of it
        # was a legal part of some cell.
        F.plate(pen, 70, row + 11, 52, 7, seed=row)
        F.printed(pen, 72, row + 12, 'CASE NOTES', F.INK)
        cats.curled_cat(pen, 28, row + 2, 40, 17, seed=row)
    pen.rect(26, 21, 100, 1, F.STEEL_EDGE)

    # the film's edges, down both sides. The rail repeats every 29 rows and 29
    # has no useful divisor, so everything down it is constant except one
    # perforation per tile, which a cropped final tile may cut in half -- which
    # is what the edge of a film does where it goes under something.
    for sx, w in ((0, 12), (31, 20)):
        key_cell(pen, sx, 42, w, 29)
        F.film(pen, sx, 42, w, 29, seed=sx, base=F.FOG, seat=False, grain=3)
        edge = sx + (0 if sx == 0 else w - 1)
        pen.rect(edge, 42, 1, 29, F.STEEL_EDGE)
        inner = sx + (w - 1 if sx == 0 else 0)
        pen.rect(inner, 42, 1, 29, F.STEEL_DEEP)
        hole = sx + (4 if sx == 0 else w - 8)
        pen.rect(hole, 54, 3, 4, F.GLOW_DIM)
        pen.rect(hole, 58, 3, 1, F.GLOW_DEEP)
        pen.rect(hole - 1, 53, 5, 1, F.TISSUE_DEEP)

    # the scrollbar clip, which is the one that holds the strip to the viewer
    for sx, dragged in ((52, False), (61, True)):
        key_cell(pen, sx, 53, 8, 18)
        pen.rect(sx, 53, 8, 18, F.STEEL_DEEP)
        pen.rect(sx + 1, 54, 6, 16, F.STEEL_DARK)
        pen.rect(sx, 53, 8, 1, F.STEEL_EDGE if dragged else F.STEEL_LIT)
        pen.rect(sx + 1, 60, 6, 4, F.GLOW if dragged else F.GLOW_DIM)
        pen.rect(sx, 70, 8, 1, F.STEEL_DEEP)

    # --- the footer: the bottom rail of the case, in two pieces
    for sx, w in ((0, 125), (126, 150)):
        F.steel(pen, sx, 72, w, 38, seed=sx, near='top')
        F.brushed(pen, sx, 72, w, 38, seed=sx + 1, density=24)
        pen.rect(sx, 72, w, 1, F.STEEL_EDGE)
        pen.rect(sx, 109, w, 1, F.STEEL_DEEP)
    pen.rect(124, 72, 1, 38, F.STEEL_DEEP)
    pen.rect(126, 72, 1, 38, F.STEEL_LIT)
    for cx, w, word in ((10, 28, 'ADD'), (39, 28, 'REM'), (69, 28, 'SEL'),
                        (99, 24, 'MISC'), (228, 28, 'LIST')):
        legend(pen, flap(cx), 79, w, 18, word, on=False, seed=cx)
    # the playlist's own transport row, in the same bone the main window's is
    for i, name in enumerate(('prev', 'play', 'pause', 'stop', 'next')):
        marked(pen, flap(139 + i * 9), 97, 8, 8, name, dim=True)
    marked(pen, flap(184), 97, 8, 8, 'eject', dim=True)
    # the two readouts the footer writes: black windows, like every other one
    F.window(pen, flap(128), 80, 80, 12, seed=7)
    F.window(pen, flap(190), 94, 36, 11, seed=8)
    F.right(pen, flap(206), 75, 'TIME / TOTAL', F.STEEL_EDGE)
    cats.paw_print(pen, 100, 98, opacity=60)
    commit(pen, 'Cat Scan - the case notes, their edges and the bottom rail')

    # the film the track list is written on: one cat, whole, asleep across the
    # length of it, and the ruled lines of the report over the top
    pen = sheet('plbg.bmp')
    F.film(pen, 0, 0, 243, 203, seed=60, base=F.AIR, seat=False)
    cats.rib_cage(pen, 20, 30, 120, 70, spine=40, ribs=11, seed=61)
    cats.skull_lateral(34, 26).stamp(pen, 2, 26)
    for i in range(22):
        x = 36 + i * 9
        cats.vertebra(9, 10, lean=0).stamp(pen, x, 30 + int(i * 1.4))
    cats.rib_cage(pen, 120, 66, 90, 60, spine=62, ribs=6, seed=62)
    for i, (_, rows) in enumerate(cats.foreign_bodies()):
        pen.stamp(150 + (i % 6) * 12, 92 + (i // 6) * 12, list(rows),
                  {'#': F.METAL, '=': F.BONE_LIT, ',': F.CARTILAGE})
        pen.ops[-1].update(opacity=190)
    F.printed(pen, 4, 190, 'ABDOMEN / VD / 0441 / ELEVEN OPAQUE BODIES',
              F.TISSUE_LIT)
    # It is a ground, not a picture: the list is written over the whole of it
    # in the brightest colour in the skin, so the whole animal goes back down
    # to a ghost before the first track name lands on it.
    pen.rect(0, 0, 243, 203, F.AIR)
    pen.ops[-1].update(opacity=178)
    for y in range(10, 203, 11):
        pen.rect(0, y, 243, 1, F.TISSUE_DEEP)
        pen.ops[-1].update(opacity=90)
    F.dust(pen, 0, 0, 243, 203, seed=63)
    commit(pen, 'Cat Scan - the whole animal, under the track list')

    # the playing row, ringed in wax the way a radiologist rings a finding.
    # A bright band would have been the obvious selection and it cannot be one:
    # the list ink is the brightest colour in the skin, so the row it is
    # written on has to stay the darkest.
    pen = sheet('plselection.bmp')
    pen.rect(0, 0, 243, 11, F.FOG)
    pen.rect(0, 0, 243, 11, F.TISSUE_DEEP)
    pen.ops[-1].update(opacity=120)
    F.grease(pen, [[1, 1], [238, 0]], F.WAX_RED, seed=1, width=1)
    F.grease(pen, [[1, 10], [238, 9]], F.WAX_RED, seed=2, width=1)
    F.grease(pen, [[1, 1], [0, 10]], F.WAX_RED_DEEP, seed=3, width=1)
    F.grease(pen, [[240, 0], [242, 10]], F.WAX_RED_DEEP, seed=4, width=1)
    commit(pen, 'Cat Scan - the playing row, ringed in wax')


@stage
def display_ink():
    """text.bmp is not a glyph sheet here.

    Cranamp sets every readout in its own 5x7 face and opens this sheet only to
    sample the display ink -- the second most common colour, when two thirds of
    it is opaque. So it is painted as the glyph sheet it looks like, in the one
    colour every live readout in this skin is written in: the light through a
    clear window in a film, which is the brightest thing in the whole skin and
    is written on the blackest.
    """
    pen = sheet('text.bmp')
    pen.rect(0, 0, 155, 18, F.AIR)
    rows = ('ABCDEFGHIJKLMNOPQRSTUVWXYZ0123', '456789.-:()+=_!?&#%*/<>ABCDEFG',
            'abcdefghijklmnopqrstuvwxyz0123')
    for i, line in enumerate(rows):
        F.printed(pen, 1, i * 6, line, F.DISPLAY, face='small', spacing=1)
    commit(pen, 'Cat Scan - the display ink')


# ---------------------------------------------------------------- the palettes


@stage
def palettes():
    """The two text files: the list's ink, and the colour of the spectrum."""
    call('studio_options', {'playlist_colors': {
        'Normal': F.BONE_LIT, 'Current': F.DISPLAY, 'NormalBG': F.AIR,
        'SelectedBG': F.TISSUE_DEEP, 'MbFG': F.DISPLAY, 'MbBG': F.AIR}})
    # The spectrum rises inside the ribcage, so it is lit the way the film is:
    # dim tissue at the bottom, bone in the middle, and blown-out white at the
    # top, which is what a bar reaching the top of a chest would look like.
    spectrum = [F.LADDER[11 - round(7 * i / 15)] for i in range(16)]
    colors = ([F.AIR, F.TISSUE_DEEP] + spectrum
              + [F.BONE_LIT, F.CORTEX, F.BONE, F.CARTILAGE, F.ENAMEL]
              + [F.METAL])
    call('studio_options', {'visualizer_colors': colors[:24]})
    print('  palettes: bone on black, and a spectrum lit like a chest')


@stage
def export():
    path = str(Path(__file__).resolve().parents[2]
               / 'assets/skins/Catamp Cat Scan.wsz')
    print(' ', call('studio_export', {'path': path})['content'][0]['text'])


ORDER = ['blank', 'options', 'titlebar', 'main', 'readouts', 'transport', 'switches',
         'seek', 'sliders', 'eq', 'playlist', 'display_ink', 'palettes',
         'export']

if __name__ == '__main__':
    cats.check()
    for name in sys.argv[1:] or ORDER:
        print(name)
        STAGES[name]()
