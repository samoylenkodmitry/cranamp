#!/usr/bin/env python3
"""Catamp Sampler -- a player sewn onto a piece of linen, drawn through Studio.

A sampler is the cloth a needleworker fills with every stitch they know, to
prove they know it; it is also what a machine does to sound. This skin is both.
The whole player is one piece of handwoven linen with things sewn to it: felt
patches for the transport keys, ribbon for the sliders, a length of yarn for
the seek bar with the ball still on the end of it, eleven kittens buttoned onto
the equalizer, and patches of indigo-dyed cloth wherever Cranamp writes
something live -- because cream floss on undyed linen cannot be read, and on
indigo it is the only thing you can see.

    python3 tools/skin-studio/catamp_sampler.py              # build it all
    python3 tools/skin-studio/catamp_sampler.py main eq       # one stage

Requires a running `cranamp --skin-studio` and nothing else: no image library,
no external asset. Every pixel is a native Studio operation, so nothing here
resizes, resamples or blurs, and the recipe runs wherever Studio does.
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import sampler_cats as cats
from pixel_pen import Pen, call
from sampler_cloth import (BALL_10, BONE, CREAM, FELT, FELT_WARM, FLAX,
                           FLAX_DARK, FLAX_DEEP, FLAX_LIT, FLAX_PALE, INDIGO,
                           INDIGO_DEEP, INDIGO_LIT, INDIGO_PALE, INK, INK_SOFT,
                           KEY, MADDER, MADDER_DARK, MADDER_LIT, MUSTARD,
                           MUSTARD_DARK, OLIVE, SLATE, TEAL, TEAL_LIT,
                           ball_palette, blanket, cord, counted_digit, cross,
                           darken, dyed, felt, hem, hexcolor, lighten, linen,
                           micro, micro_width, mix, patch, ribbon, running,
                           sewn_button, shade, stitched, text_width, yarn_ball)

STAGES = {}


def stage(fn):
    STAGES[fn.__name__] = fn
    return fn


def sheet(name):
    call('studio_atlas', {'sheet': name})
    return Pen()


def commit(pen, label):
    result = call('studio_draw', {'label': label, 'operations': pen.ops})
    note = result['content'][0]['text'] if result.get('content') else ''
    print(f'  {label}: {note}')
    return result


def grid(pen, x, y, rows, palette):
    pen.stamp(int(x), int(y), list(rows), palette)
    return pen


# ------------------------------------------------------------------- vocabulary


# The transport marks, stitched. Each is a bounded grid so it can be checked
# against the cell it goes in rather than discovered to be a pixel wide in the
# player.
MARKS = {
    'prev': ('x     xx   ', 'x    xxx   ', 'x   xxxx   ', 'x  xxxxx   ',
             'x   xxxx   ', 'x    xxx   ', 'x     xx   '),
    'play': ('xx     ', 'xxxx   ', 'xxxxxx ', 'xxxxxxx', 'xxxxxx ',
             'xxxx   ', 'xx     '),
    'pause': ('xx  xx', 'xx  xx', 'xx  xx', 'xx  xx', 'xx  xx', 'xx  xx',
              'xx  xx'),
    'stop': ('xxxxxxx', 'xxxxxxx', 'xxxxxxx', 'xxxxxxx', 'xxxxxxx',
             'xxxxxxx', 'xxxxxxx'),
    'next': ('xx     x', 'xxx    x', 'xxxx   x', 'xxxxx  x', 'xxxx   x',
             'xxx    x', 'xx     x'),
    'eject': ('   x   ', '  xxx  ', ' xxxxx ', 'xxxxxxx', '       ',
              'xxxxxxx', 'xxxxxxx'),
}
SMALL_MARKS = {
    'prev': ('x  xx  ', 'x xxx  ', 'xxxxx  ', 'x xxx  ', 'x  xx  '),
    'play': ('x    ', 'xxx  ', 'xxxxx', 'xxx  ', 'x    '),
    'pause': ('xx xx', 'xx xx', 'xx xx', 'xx xx', 'xx xx'),
    'stop': ('xxxxx', 'xxxxx', 'xxxxx', 'xxxxx', 'xxxxx'),
    'next': ('  xx  x', '  xxx x', '  xxxxx', '  xxx x', '  xx  x'),
    'eject': ('  x  ', ' xxx ', 'xxxxx', '     ', 'xxxxx'),
}


def marked(pen, x, y, w, h, name, color, *, shadow=None, marks=None):
    """Centre a stitched mark in a cell."""
    rows = (marks or MARKS)[name]
    mx = x + (w - len(rows[0])) // 2
    my = y + (h - len(rows)) // 2
    if shadow:
        grid(pen, mx + 1, my + 1, rows, {'x': shadow})
        pen.ops[-1].update(opacity=120)
    grid(pen, mx, my, rows, {'x': color})
    return pen


def key_cell(pen, x, y, w, h):
    """Clear a sprite cell to the classic transparency key, so the cloth the
    control sits on shows through wherever the control does not cover it."""
    pen.rect(x, y, w, h, KEY)
    return pen


def tab(pen, x, y, w, h, word, *, on, held, face=FELT, seed=0, small=False,
        ground=FLAX):
    """The skin's one switch: a felt tab with a word on it.

    On and off are two different felts rather than two brightnesses of one,
    because a stitched label has nowhere to put a lamp and a switch that only
    changes value reads as a rendering glitch rather than as a state.
    """
    body = face if on else SLATE
    thread = BONE if on else shade(BONE, .30)
    patch(pen, x, y, w, h, face=body, held=held, thread=thread, seed=seed,
          ground=ground)
    ink = CREAM if on else shade(CREAM, .30)
    if small:
        left = x + (w - micro_width(word)) // 2
        micro(pen, left, y + (h - 5) // 2, word, ink,
              shadow=darken(body, .40))
    else:
        left = x + (w - text_width(word)) // 2
        stitched(pen, left, y + (h - 7) // 2, word, ink,
                 shadow=darken(body, .40))
    return pen


def dyed_panel(pen, x, y, w, h, *, seed=0, thread=CREAM, evenweave=False):
    """A patch of indigo-dyed cloth appliquéd onto the linen: the ground for
    everything Cranamp writes live. The running stitch that holds it down is
    two pixels in from the edge, which is where a needle actually goes."""
    pen.rect(x - 1, y - 1, w + 2, h + 2, FLAX_DEEP)
    pen.rect(x - 1, y - 1, w + 2, 1, shade(FLAX, -.30))
    dyed(pen, x, y, w, h, seed=seed)
    pen.rect(x, y, w, 1, INDIGO_DEEP)
    pen.rect(x, y, 1, h, darken(INDIGO, .30))
    pen.rect(x, y + h - 1, w, 1, INDIGO_PALE)
    pen.rect(x + w - 1, y, 1, h, INDIGO_LIT)
    running(pen, x + 2, y + 2, w - 4, thread, dash=1, gap=3, phase=x)
    running(pen, x + 2, y + h - 3, w - 4, thread, dash=1, gap=3, phase=x)
    running(pen, x + 2, y + 2, h - 4, thread, axis='y', dash=1, gap=3, phase=y)
    running(pen, x + w - 3, y + 2, h - 4, thread, axis='y', dash=1, gap=3,
            phase=y)
    if evenweave:
        # Counted work is done on cloth with a countable weave. The lattice is
        # what makes the live spectrum read as stitches rising through it.
        for gy in range(y + 4, y + h - 4, 3):
            for gx in range(x + 4, x + w - 4, 3):
                pen.pixel(gx, gy, INDIGO_LIT)
                pen.ops[-1].update(opacity=90)
    return pen


# --------------------------------------------------------------------- title


@stage
def titlebar():
    """titlebar.bmp: the ribbon the player's name is worked on.

    Most of this sheet is shade-mode art Cranamp never draws, so it is filled
    with plain cloth and only the cells that are used are worked.
    """
    pen = sheet('titlebar.bmp')
    linen(pen, 0, 0, 344, 87, seed=11)
    for row, active in ((0, True), (15, False)):
        band = INDIGO if active else shade(INDIGO_LIT, .30)
        floss = CREAM if active else shade(BONE, .42)
        linen(pen, 27, row, 275, 14, seed=20 + row)
        ribbon(pen, 27, row + 1, 275, 12, band, seed=30 + row,
               selvedge=darken(band, .38))
        running(pen, 27, row + 1, 275, shade(FLAX_PALE, .10), phase=27)
        running(pen, 27, row + 12, 275, shade(FLAX_DEEP, .10), phase=27)
        title = 'CATAMP SAMPLER'
        left = 27 + (275 - text_width(title)) // 2
        stitched(pen, left, row + 4, title, floss, shadow=darken(band, .45))
        for i in range(4):
            cross(pen, left - 14 - i * 5, row + 5,
                  MADDER_LIT if active else shade(MADDER_DARK, .30))
            cross(pen, left + text_width(title) + 9 + i * 5, row + 5,
                  MUSTARD if active else shade(MUSTARD_DARK, .30))
    # The four window buttons are bone buttons sewn through the ribbon. The
    # shade button's two states sit side by side rather than stacked, which is
    # the one place this sheet breaks its own rule.
    for bx, by, held in ((0, 0, False), (0, 9, True), (9, 0, False),
                         (9, 9, True), (18, 0, False), (18, 9, True),
                         (0, 18, False), (9, 18, True)):
        key_cell(pen, bx, by, 9, 9)
        ribbon(pen, bx, by, 9, 9, INDIGO, seed=bx + by)
        sewn_button(pen, bx, by, 9, BONE, held=held, thread=MADDER)
    commit(pen, 'Sampler · the name worked on a ribbon')


# ------------------------------------------------------------------ main cloth


@stage
def main():
    """main.bmp: the piece of linen the whole player is sewn onto."""
    pen = sheet('main.bmp')
    linen(pen, 0, 0, 275, 115, seed=3, top=FLAX_LIT, bottom=FLAX, grain=9)
    # The title ribbon is sewn on along this line, so the cloth below it starts
    # with the shadow of that seam.
    pen.rect(0, 14, 275, 1, FLAX_DEEP)
    pen.rect(0, 15, 275, 1, shade(FLAX_PALE, .12))
    running(pen, 0, 16, 275, INK_SOFT, dash=2, gap=3)

    # Four patches of dyed cloth, one under each thing Cranamp writes live.
    # One tall patch behind the clock and the spectrum together was tried
    # first: the two readouts are eighteen pixels apart and the cloth between
    # them arrived as a void that neither of them filled.
    dyed_panel(pen, 22, 22, 82, 19, seed=41)
    dyed_panel(pen, 22, 42, 82, 19, seed=55, evenweave=True)
    dyed_panel(pen, 106, 22, 163, 15, seed=47)
    dyed_panel(pen, 106, 39, 163, 15, seed=51)
    # The colon between the minutes and the seconds is two cross stitches. The
    # player draws no colon of its own: it is the gap between two digit cells,
    # and whatever the skin puts there is the colon.
    cross(pen, 72, 27, CREAM)
    cross(pen, 72, 33, CREAM)
    # Captions, in the four-by-five face, beside the readouts rather than under
    # them -- the row is eight pixels tall and has nothing under it.
    micro(pen, 132, 42, 'KBPS', shade(CREAM, .34))
    micro(pen, 170, 42, 'KHZ', shade(CREAM, .34))

    # The cat that watches the spectrum, worked in the left margin.
    head = cats.sitting(pen, 0, 18, 21, 48, cats.GINGER)
    cats.whiskers(pen, *head, FLAX_PALE)
    cats.face(pen, *head, cats.GINGER)

    # Grooves for the two ribbon sliders, and the seats the moving handles run
    # in. A handle casts no shadow of its own -- it moves, and a painted shadow
    # would stay behind -- so the groove carries it.
    for gx, gw in ((107, 68), (177, 38)):
        pen.rect(gx, 57, gw, 13, shade(FLAX, .16))
        pen.rect(gx, 57, gw, 1, FLAX_DEEP)
        pen.rect(gx, 69, gw, 1, FLAX_PALE)

    # The seek line: a length of yarn couched down across the cloth, with the
    # little stitches that hold it every few pixels.
    pen.rect(17, 71, 248, 1, shade(FLAX, .20))
    pen.rect(17, 82, 248, 1, FLAX_PALE)

    # Seats for the fixed controls: a one-pixel shadow under and right of every
    # cell, drawn on the cloth because a sprite cannot paint outside its own.
    for cx, cy, cw, ch in ((16, 88, 23, 18), (39, 88, 23, 18), (62, 88, 23, 18),
                           (85, 88, 23, 18), (108, 88, 22, 18),
                           (136, 89, 22, 16), (164, 89, 47, 15),
                           (210, 89, 28, 15), (219, 58, 23, 12),
                           (242, 58, 23, 12)):
        pen.rect(cx + 1, cy + ch, cw, 1, shade(FLAX, .42))
        pen.rect(cx + cw, cy + 1, 1, ch, shade(FLAX, .42))

    # Bottom left: the pincushion, which is where the needles live.
    pincushion(pen, 1, 86)
    # Bottom right: the basket of wound colours. Cranamp opens the skin chooser
    # when this corner is clicked, so the corner is a choice of colours.
    basket(pen, 240, 84)

    # The bottom hem, and a row of counted paw prints marching along it.
    hem(pen, 0, 0, 275, 115, sides='b', thread=INK_SOFT)
    for i in range(16):
        x = 26 + i * 13
        if 130 < x < 246:
            continue
        grid(pen, x, 107, cats.PAW, {'x': shade(FLAX, .30)})
    # Row 114 is drawn twice -- once as the last row of the window and once as
    # the docking edge below it -- so it has to be a colour that can be both.
    pen.rect(0, 114, 275, 1, FLAX_DEEP)
    commit(pen, 'Sampler · the linen, the dyed patches and the watching cat')


def pincushion(pen, x, y):
    """A tomato pincushion with three pins in it. Sixteen by twenty-eight is
    not much, and the strawberry on its string is the mark that says what the
    red lump is."""
    pen.ellipse(x + 1, y + 10, 14, 13, MADDER, fill=True)
    pen.ellipse(x + 1, y + 10, 14, 13, MADDER_DARK, fill=False, width=1)
    for i in range(5):
        pen.taper((x + 8, y + 11), (x + 2 + i * 3, y + 15),
                  (x + 1 + i * 3.4, y + 22), 2, darken(MADDER, .22))
    pen.taper((x + 8, y + 11), (x + 8, y + 16), (x + 8, y + 22), 2,
              MADDER_LIT)
    pen.ellipse(x + 6, y + 8, 5, 4, OLIVE, fill=True)
    # Three pins: a shaft of pale thread and a head of coloured glass.
    for dx, dy, head in ((1, 4, MUSTARD), (6, 1, TEAL_LIT), (11, 3, CREAM)):
        pen.line([[x + dx + 2, y + dy + 10], [x + dx, y + dy]], FLAX_PALE)
        pen.line([[x + dx + 3, y + dy + 10], [x + dx + 1, y + dy]],
                 shade(FLAX_PALE, .30))
        pen.rect(x + dx - 1, y + dy - 1, 3, 3, head)
        pen.pixel(x + dx - 1, y + dy - 1, lighten(head, .40))
    return pen


def basket(pen, x, y):
    """A basket of wound colours in the corner Cranamp opens the skin chooser
    from: click the colours to change the colours."""
    for i, (bx, by, bw, color) in enumerate(((2, 4, 11, MADDER),
                                             (13, 2, 12, TEAL),
                                             (9, 11, 13, MUSTARD),
                                             (21, 9, 11, OLIVE))):
        yarn_ball(pen, x + bx, y + by, bw, bw - 2, color)
    pen.path([[x, y + 16], [x + 4, y + 30, x + 30, y + 30, x + 34, y + 16]],
             darken(FLAX_DARK, .18), True)
    for i in range(0, 34, 3):
        pen.line([[x + i, y + 17], [x + i + 4, y + 29]], FLAX_LIT)
        pen.ops[-1].update(opacity=110)
    for i in range(0, 34, 4):
        pen.line([[x + i + 4, y + 17], [x + i, y + 29]], FLAX_DEEP)
        pen.ops[-1].update(opacity=90)
    pen.path([[x, y + 16], [x + 4, y + 30, x + 30, y + 30, x + 34, y + 16]],
             INK_SOFT, False, 1)
    pen.rect(x, y + 15, 34, 2, darken(FLAX_DARK, .30))
    running(pen, x + 1, y + 15, 32, FLAX_PALE, dash=1, gap=2)
    return pen


# ----------------------------------------------------------------- transport


@stage
def transport():
    """cbuttons.bmp: six felt patches blanket-stitched onto the cloth."""
    pen = sheet('cbuttons.bmp')
    pen.rect(0, 0, 136, 36, KEY)
    faces = {'prev': FELT, 'play': FELT_WARM, 'pause': FELT, 'stop': FELT,
             'next': FELT, 'eject': FELT}
    cells = (('prev', 0, 23, 18), ('play', 23, 23, 18), ('pause', 46, 23, 18),
             ('stop', 69, 23, 18), ('next', 92, 22, 18), ('eject', 114, 22, 16))
    for name, x, w, h in cells:
        for row, held in ((0, False), (h if name == 'eject' else 18, True)):
            face = faces[name]
            patch(pen, x, row, w, h, face=face, held=held, thread=BONE,
                  seed=x + row)
            marked(pen, x, row, w, h, name,
                   shade(CREAM, .30) if held else CREAM,
                   shadow=darken(face, .55))
    commit(pen, 'Sampler · felt transport patches')


@stage
def switches():
    """shufrep.bmp: the four switches that have an on and an off."""
    pen = sheet('shufrep.bmp')
    pen.rect(0, 0, 92, 85, KEY)
    for x, w, word in ((0, 28, 'RPT'), (28, 47, 'SHUFFLE')):
        for i, (on, held) in enumerate(((False, False), (False, True),
                                        (True, False), (True, True))):
            tab(pen, x, i * 15, w, 15, word, on=on, held=held,
                face=FELT_WARM if on else FELT, seed=x + i,
                small=(w < 40))
    for x, base, word in ((0, 61, 'EQ'), (23, 61, 'PL')):
        for (dx, dy), (on, held) in zip(((0, 0), (46, 0), (0, 12), (46, 12)),
                                        ((False, False), (False, True),
                                         (True, False), (True, True))):
            tab(pen, x + dx, base + dy, 23, 12, word, on=on, held=held,
                face=TEAL if on else FELT, seed=x + dx + dy, small=True)
    commit(pen, 'Sampler · shuffle, repeat and the two window switches')


# ---------------------------------------------------------------- the seek yarn


@stage
def seek():
    """posbar.bmp: a length of yarn couched across the cloth, and the ball it
    came off. The ball is the handle, so the thread to the left of it is the
    part that has been unwound -- which is exactly what a played position is.
    """
    pen = sheet('posbar.bmp')
    pen.rect(0, 0, 307, 10, KEY)
    # The track: a shallow groove pressed into the cloth with the yarn lying in
    # it, and the couching stitches that hold the yarn down every eight pixels.
    linen(pen, 0, 0, 248, 10, seed=61, top=FLAX, bottom=FLAX_DARK, slubs=2)
    pen.rect(0, 0, 248, 1, FLAX_DEEP)
    pen.rect(0, 1, 248, 1, shade(FLAX, .26))
    pen.rect(0, 8, 248, 1, FLAX_PALE)
    pen.rect(0, 9, 248, 1, shade(FLAX, .20))
    cord(pen, 0, 3, 248, MADDER, period=4)
    cord(pen, 0, 5, 248, MADDER_DARK, period=4, phase=2)
    for i in range(4, 248, 8):
        pen.rect(i, 3, 1, 4, FLAX_PALE)
        pen.ops[-1].update(opacity=150)
    for x, held in ((248, False), (278, True)):
        key_cell(pen, x, 0, 29, 10)
        # The loose end first, so the ball sits on top of its own tail. It is
        # drawn as a wandering line rather than as more cord, because the yarn
        # to the left of the ball is the part that has come off it.
        # The loose end wanders above and below the couched line rather than
        # lying along it: drawn in the cord's own colour on top of the cord it
        # was invisible, which is what a loose end must never be. It also has
        # to stay inside ten rows -- an arc that leaves the cell is clipped
        # square and reads as a wire.
        pen.path([[x, 7], [x + 5, 2, x + 10, 5], [x + 15, 8, x + 19, 5]],
                 MADDER_DARK, False, 1)
        pen.path([[x, 6], [x + 5, 1, x + 10, 4], [x + 15, 7, x + 19, 4]],
                 MADDER_LIT, False, 1)
        grid(pen, x + 19, 0, BALL_10,
             ball_palette(darken(MADDER, .14) if held else MADDER))
    commit(pen, 'Sampler · the seek yarn and the ball on the end of it')


# -------------------------------------------------------------------- sliders


@stage
def sliders():
    """volume.bmp and balance.bmp: two bands of counted work in progress.

    Twenty-eight frames of a slider's track are twenty-eight chances to say
    something the handle cannot, so the track is a sampler band and the frame
    says how much of it has been stitched. Volume fills from the left; balance
    fills out from the middle, so centre reads as centre with no mark to say
    so.

    Both grooves are dyed rather than plain, for the same reason the readouts
    are: a band of pale floss on pale linen is a texture, and on indigo it is a
    row of stitches. The first pass put the work straight onto the cloth and
    the whole control disappeared into the ground it was sewn to.
    """
    pen = sheet('volume.bmp')
    pen.rect(0, 0, 68, 433, KEY)
    slots = 16
    for frame in range(28):
        y = frame * 15
        worked = round(frame / 27 * slots)
        dyed_band(pen, 0, y, 68, 13, seed=frame)
        for i in range(slots):
            cx = 2 + i * 4
            if i < worked:
                color = hexcolor(mix(OLIVE, MADDER, i / (slots - 1)))
                cross(pen, cx, y + 3, color)
                cross(pen, cx, y + 7, darken(color, .22))
            else:
                hole(pen, cx, y + 3)
                hole(pen, cx, y + 7)
    thumbs(pen)
    commit(pen, 'Sampler · the volume band, worked stitch by stitch')

    pen = sheet('balance.bmp')
    pen.rect(0, 0, 68, 433, KEY)
    slots, centre = 9, 4
    for frame in range(28):
        y = frame * 15
        dyed_band(pen, 9, y, 38, 13, seed=frame)
        here = round(frame / 27 * (slots - 1))
        for i in range(slots):
            cx = 10 + i * 4
            if min(centre, here) <= i <= max(centre, here):
                color = TEAL if i <= centre else MUSTARD
                cross(pen, cx, y + 3, color)
                cross(pen, cx, y + 7, darken(color, .22))
            else:
                hole(pen, cx, y + 3)
                hole(pen, cx, y + 7)
        pen.pixel(10 + centre * 4 + 1, y + 1, CREAM)
        pen.pixel(10 + centre * 4 + 1, y + 11, CREAM)
    thumbs(pen)
    commit(pen, 'Sampler · the balance band, worked out from the middle')


def dyed_band(pen, x, y, w, h, *, seed=0):
    """A band of dyed evenweave: the ground a counted row is worked on."""
    dyed(pen, x, y, w, h, seed=seed, top=INDIGO, bottom=INDIGO_DEEP)
    pen.rect(x, y, w, 1, darken(INDIGO, .45))
    pen.rect(x, y + h - 1, w, 1, INDIGO_PALE)
    pen.rect(x, y, 1, h, darken(INDIGO, .35))
    pen.rect(x + w - 1, y, 1, h, INDIGO_LIT)
    return pen


def hole(pen, x, y):
    """An unworked square of the chart: four holes of the weave, nothing in
    them. This is what makes the worked part read as work."""
    for dx, dy in ((0, 0), (2, 0), (0, 2), (2, 2)):
        pen.pixel(x + dx, y + dy, INDIGO_LIT)
        pen.ops[-1].update(opacity=120)
    return pen


def thumbs(pen):
    """The handle both sliders share: a cat's head, 14x11.

    A whole curled cat was tried in this cell and it is four pixels too short
    for one -- the body arrived as a bean with an eye. A head has ears, and
    ears are what say cat at eleven pixels.
    """
    for x, rows in ((15, cats.HANDLE_HEAD), (0, cats.HANDLE_HEAD_HELD)):
        key_cell(pen, x, 422, 14, 11)
        grid(pen, x, 422, rows, cats._pal(cats.GINGER))
    return pen


# ------------------------------------------------------------------- readouts


@stage
def readouts():
    """The four sheets Cranamp samples rather than lays out: the timer, the
    status lamp, the two channel lamps, and the sheet it takes its ink from."""
    pen = sheet('numbers.bmp')
    pen.rect(0, 0, 99, 13, KEY)
    for value in range(10):
        counted_digit(pen, value * 9, 0, value, CREAM,
                      shadow=darken(INDIGO_DEEP, .30))
    # The eleventh cell is the classic blank; leave it keyed.
    commit(pen, 'Sampler · the clock, in counted cross stitch')

    pen = sheet('playpaus.bmp')
    pen.rect(0, 0, 42, 9, KEY)
    # The mark alone, in bright floss, with nothing behind it. A dark disc was
    # tried first and vanished: this cell sits on the indigo panel, and a dark
    # thing on dark cloth is not a lamp, it is a hole.
    lamps = ((0, 'play', '#9ad14e'), (9, 'pause', MUSTARD),
             (18, 'stop', MADDER_LIT))
    for x, name, color in lamps:
        marked(pen, x, 0, 9, 9, name, color, shadow=INDIGO_DEEP,
               marks=SMALL_MARKS)
    commit(pen, 'Sampler · the status lamp')

    pen = sheet('monoster.bmp')
    pen.rect(0, 0, 56, 24, KEY)
    for y, lit in ((0, True), (12, False)):
        for x, cell, word in ((0, 29, 'STEREO'), (29, 27, 'MONO')):
            color = CREAM if lit else shade(INDIGO_PALE, -.05)
            left = x + (cell - micro_width(word)) // 2
            micro(pen, left, y + 4, word, color,
                  shadow=INDIGO_DEEP if lit else None)
            if lit:
                running(pen, x + 2, y + 1, cell - 4, MUSTARD, dash=1, gap=3)
    commit(pen, 'Sampler · the channel lamps')

    # text.bmp is not a glyph sheet in Cranamp -- it renders titles with its own
    # face and samples this sheet only for the display ink: the second most
    # common colour of an opaque sheet. Paint it as the glyph sheet it looks
    # like, in the floss the readouts should be worked in.
    pen = sheet('text.bmp')
    pen.rect(0, 0, 155, 18, INDIGO_DEEP)
    rows = ('ABCDEFGHIJKLMNOPQRSTUVWXYZ', '0123456789.-:()+=_!?&#%*/\\<>',
            'abcdefghijklmnopqrstuvwxyz')
    for i, line in enumerate(rows):
        stitched(pen, 1, i * 6, line[:31], CREAM, spacing=0)
    commit(pen, 'Sampler · the floss the readouts are worked in')


# ------------------------------------------------------------------ equalizer


@stage
def eq():
    """eqmain.bmp: the same cloth, and a litter of kittens on the sliders.

    Each band gets its own handle rather than the classic shared one, because
    eleven identical heads is a pattern and eleven different kittens is a
    litter -- and one grid relit by eleven palettes costs no more than one.
    """
    call('studio_options', {'eq_handles': True, 'eq_travel': 38})
    pen = sheet('eqmain.bmp')
    linen(pen, 0, 0, 275, 315, seed=7, top=FLAX_LIT, bottom=FLAX, grain=9)
    pen.rect(0, 14, 275, 1, FLAX_DEEP)
    pen.rect(0, 15, 275, 1, shade(FLAX_PALE, .12))
    running(pen, 0, 16, 275, INK_SOFT, dash=2, gap=3)

    # The curve Cranamp draws lives on its own patch of dyed cloth.
    dyed_panel(pen, 84, 15, 117, 23, seed=63, evenweave=True)
    # The preamp line sits halfway up it; a paler thread marks where zero is.
    pen.rect(86, 26, 113, 1, INDIGO_PALE)

    # Eleven grooves, and one for the preamp. A groove is a channel pressed
    # into the cloth: dark at the top where the light cannot reach.
    for x in [21] + [78 + i * 18 for i in range(10)]:
        pen.rect(x, 38, 14, 63, shade(FLAX, .12))
        pen.rect(x, 38, 14, 1, FLAX_DEEP)
        pen.rect(x, 100, 14, 1, FLAX_PALE)
        pen.rect(x, 38, 1, 63, shade(FLAX, .26))
        pen.rect(x + 13, 38, 1, 63, FLAX_PALE)
        dyed_band(pen, x + 5, 40, 4, 59, seed=x)
    # Band captions, in the four-by-five face -- fourteen pixels has no room
    # for the five-by-seven one.
    for i, word in enumerate(('60', '170', '310', '600', '1K', '3K', '6K',
                              '12K', '14K', '16K')):
        x = 78 + i * 18
        micro(pen, x + (14 - micro_width(word)) // 2, 103, word, INK_SOFT)
    micro(pen, 21 + (14 - micro_width('PRE')) // 2, 103, 'PRE', INK_SOFT)

    # The cat that watches the litter, in the gap the preamp leaves.
    head = cats.sitting(pen, 39, 44, 34, 55, cats.TABBY)
    cats.whiskers(pen, *head, FLAX_PALE)
    cats.face(pen, *head, cats.TABBY)

    for cx, cy, cw, ch in ((14, 18, 26, 12), (40, 18, 32, 12),
                           (217, 18, 44, 12), (264, 3, 9, 9)):
        pen.rect(cx + 1, cy + ch, cw, 1, shade(FLAX, .42))
        pen.rect(cx + cw, cy + 1, 1, ch, shade(FLAX, .42))
    hem(pen, 0, 0, 275, 116, sides='b', thread=INK_SOFT)
    pen.rect(0, 115, 275, 1, FLAX_DEEP)

    # The title ribbon, twice.
    for row, active in ((134, True), (149, False)):
        band = INDIGO if active else shade(INDIGO_LIT, .30)
        floss = CREAM if active else shade(BONE, .42)
        linen(pen, 0, row, 275, 14, seed=90 + row)
        ribbon(pen, 0, row + 1, 275, 12, band, seed=95 + row,
               selvedge=darken(band, .38))
        title = 'EQUALIZER'
        left = (275 - text_width(title)) // 2
        stitched(pen, left, row + 4, title, floss, shadow=darken(band, .45))
        for i in range(3):
            cross(pen, left - 12 - i * 5, row + 5,
                  MADDER_LIT if active else shade(MADDER_DARK, .30))
            cross(pen, left + text_width(title) + 7 + i * 5, row + 5,
                  MUSTARD if active else shade(MUSTARD_DARK, .30))
    # The equalizer's close button, a bone button like the main window's.
    for y, held in ((116, False), (125, True)):
        key_cell(pen, 0, y, 9, 9)
        ribbon(pen, 0, y, 9, 9, INDIGO, seed=y)
        sewn_button(pen, 0, y, 9, BONE, held=held)

    # ON, AUTO and PRESETS.
    for x, w, word in ((10, 26, 'ON'), (36, 32, 'AUTO')):
        for dx, (on, held) in zip((0, 118, 59, 177),
                                  ((False, False), (False, True),
                                   (True, False), (True, True))):
            tab(pen, x + dx, 119, w, 12, word, on=on, held=held,
                face=TEAL if on else FELT, seed=x + dx, small=True)
    for y, held in ((164, False), (176, True)):
        tab(pen, 224, y, 44, 12, 'PRESETS', on=True, held=held, face=FELT,
            seed=y, small=True)

    # The slider grooves, twenty-eight frames each. The ribbon in the groove is
    # worked from the bottom up to wherever the handle is, so a band that is
    # boosted has more colour in it than a band that is cut.
    for frame in range(28):
        fx = 13 + 15 * (frame % 14)
        fy = 164 if frame < 14 else 229
        pen.rect(fx, fy, 14, 63, shade(FLAX, .12))
        pen.rect(fx, fy, 14, 1, FLAX_DEEP)
        pen.rect(fx, fy + 62, 14, 1, FLAX_PALE)
        pen.rect(fx, fy, 1, 63, shade(FLAX, .26))
        pen.rect(fx + 13, fy, 1, 63, FLAX_PALE)
        dyed_band(pen, fx + 5, fy + 2, 4, 59, seed=frame)
        # Travel is 38 pixels with a 25-pixel handle, and frame 27 is the
        # handle at the top, so the worked part runs from the handle's waist
        # down to the bottom of the groove: a boosted band has more colour in
        # it than a cut one, which is the only thing a band can say about
        # itself while its handle is somewhere else.
        top = 2 + round((27 - frame) / 27 * 38) + 12
        color = hexcolor(mix(TEAL, MADDER, frame / 27))
        for i in range(fy + top, fy + 61, 2):
            cross(pen, fx + 5, i - 1, color if (i // 2) % 2 else
                  lighten(color, .22))
    # The graph and the preamp line Cranamp draws over.
    dyed(pen, 0, 294, 113, 19, seed=71, top=INDIGO, bottom=INDIGO_DEEP)
    for gy in range(295, 313, 3):
        for gx in range(1, 113, 3):
            pen.pixel(gx, gy, INDIGO_LIT)
            pen.ops[-1].update(opacity=80)
    pen.rect(0, 314, 113, 1, INDIGO_PALE)
    commit(pen, 'Sampler · the equalizer cloth, its grooves and its captions')

    # Eleven kittens, one per band: one grid, eleven coats.
    pen = sheet('eqhandles.bmp')
    pen.rect(0, 0, 154, 50, KEY)
    for slot in range(11):
        coat = cats.wearing(cats.LITTER[slot], cats.COLLARS[slot])
        pen.sprite_cell(slot * 14, 0, 14, 25, cats.KITTEN, cats._pal(coat))
        pen.sprite_cell(slot * 14, 25, 14, 25, cats.KITTEN_HELD,
                        cats._pal(coat))
    commit(pen, 'Sampler · eleven kittens, one per band')


# ------------------------------------------------------------------- playlist


def flap(canvas_x):
    """The sheet column for a canvas column on the playlist's right flap.

    bottom.left is sheet 0..124 drawn at canvas 0..124; bottom.right is sheet
    126..275 drawn at canvas 125..274, and sheet column 125 is never drawn. So
    everything on the right flap is a pixel along, and arithmetic done by eye
    lands a button a pixel out of step with its own hit area.
    """
    return canvas_x + 1


@stage
def playlist():
    """pledit.bmp: the cloth again, hemmed, with a patchwork seam down the
    middle of the footer.

    Two things here are drawn more than once: the 25-pixel header tile, nine
    times, and the two footer flaps. A pattern whose period does not divide 25
    shifts at every repeat and reads as a row of seams, and the side rails
    repeat every 29 rows, so anything on them has to be constant down the rail.
    Five divides 25, which is why the header border is a five-pixel motif.
    """
    call('studio_options', {'playlist_background': True,
                            'playlist_selection': True})
    pen = sheet('pledit.bmp')
    linen(pen, 0, 0, 280, 190, seed=17, top=FLAX_LIT, bottom=FLAX, grain=9)

    # The header rows are the opposite way round from the main window's. Classic
    # pledit.bmp keeps two of them and Cranamp draws the *lower* one always --
    # there is no unfocused playlist -- so the worked version goes at row 21 and
    # row 0 is only there because the format has a place for it. This skin had
    # them the other way round until `studio_targets` grew labels and said so;
    # nothing else would have, because both rows look right on their own.
    for row, active in ((0, False), (21, True)):
        band = INDIGO if active else shade(INDIGO_LIT, .30)
        floss = CREAM if active else shade(BONE, .42)
        # The corners and the repeating tile: linen with a counted border.
        for x, w in ((0, 25), (127, 25), (153, 25)):
            linen(pen, x, row, w, 20, seed=30 + x + row)
            hem(pen, x, row, w, 20, sides='t', thread=INK_SOFT)
            pen.rect(x, row + 18, w, 1, shade(FLAX, .26))
            pen.rect(x, row + 19, w, 1, FLAX_DEEP)
            for i in range(w // 5):
                cross(pen, x + i * 5 + 1, row + 7,
                      MADDER if (x // 5 + i) % 2 else TEAL)
                pen.pixel(x + i * 5 + 3, row + 13, MUSTARD)
        # The title cell is a woven name tape sewn across the header.
        linen(pen, 26, row, 100, 20, seed=70 + row)
        hem(pen, 26, row, 100, 20, sides='t', thread=INK_SOFT)
        pen.rect(26, row + 18, 100, 1, shade(FLAX, .26))
        pen.rect(26, row + 19, 100, 1, FLAX_DEEP)
        ribbon(pen, 30, row + 4, 92, 13, band, seed=33 + row,
               selvedge=darken(band, .38))
        title = 'PLAYLIST'
        stitched(pen, 30 + (92 - text_width(title)) // 2, row + 7, title,
                 floss, shadow=darken(band, .45))

    # The side rails: the hemmed edge of the cloth. A rail repeats every 29
    # rows, so everything on it runs across the rail and is constant down it.
    linen(pen, 0, 42, 12, 29, seed=90, grain=6)
    pen.rect(0, 42, 2, 29, FLAX_LIT)
    pen.rect(2, 42, 1, 29, FLAX_DEEP)
    pen.rect(9, 42, 1, 29, shade(FLAX, .20))
    pen.rect(10, 42, 1, 29, FLAX_PALE)
    pen.rect(11, 42, 1, 29, shade(FLAX, .30))
    for y in range(42, 71):
        if y % 4 < 2:
            pen.pixel(4, y, INK_SOFT)
            pen.pixel(7, y, INK_SOFT)
    # The right rail carries the scroll channel: canvas 260..267 is sheet
    # 36..43, because the rail is drawn from canvas 255 and starts at sheet 31.
    linen(pen, 31, 42, 20, 29, seed=94, grain=6)
    pen.rect(50, 42, 1, 29, FLAX_LIT)
    pen.rect(49, 42, 1, 29, FLAX_DEEP)
    for y in range(42, 71):
        if y % 4 < 2:
            pen.pixel(33, y, INK_SOFT)
            pen.pixel(46, y, INK_SOFT)
    pen.rect(35, 42, 1, 29, FLAX_DEEP)
    dyed(pen, 36, 42, 8, 29, seed=98, top=INDIGO, bottom=INDIGO_LIT)
    pen.rect(36, 42, 1, 29, darken(INDIGO, .40))
    pen.rect(43, 42, 1, 29, INDIGO_LIT)
    pen.rect(44, 42, 1, 29, FLAX_PALE)
    for x, thread in ((52, MUSTARD), (61, MADDER)):
        key_cell(pen, x, 53, 8, 18)
        grid(pen, x, 53, cats.BOBBIN, cats.bobbin_palette(thread))

    # The footer: two pieces of cloth seamed together. Sheet column 125 is
    # never drawn, so nothing continuous may cross it -- which is what a
    # patchwork seam is for.
    for x, w, seed in ((0, 125, 110), (126, 150, 118)):
        linen(pen, x, 72, w, 38, seed=seed, top=FLAX, bottom=FLAX_DARK)
        pen.rect(x, 72, w, 1, FLAX_DEEP)
        pen.rect(x, 73, w, 1, FLAX_PALE)
    # Canvas 123/124 take the shadow and the fold; canvas 125 -- sheet 126 --
    # is the lit edge of the far piece.
    pen.rect(122, 72, 1, 38, shade(FLAX, .26))
    pen.rect(123, 72, 1, 38, shade(FLAX, .50))
    pen.rect(124, 72, 1, 38, FLAX_DEEP)
    pen.rect(126, 72, 1, 38, FLAX_PALE)
    pen.rect(127, 72, 1, 38, shade(FLAX, .12))
    running(pen, 120, 72, 38, INK_SOFT, axis='y', phase=72)
    running(pen, flap(128), 72, 38, INK_SOFT, axis='y', phase=74)
    hem(pen, 0, 72, 125, 38, sides='bl', thread=INK_SOFT)
    hem(pen, 126, 72, 150, 38, sides='br', thread=INK_SOFT)

    # ADD, REM and SEL fit on the left piece; MISC stops at the seam rather
    # than straddling it, because its halves would arrive a pixel out of step.
    for x, w, word in ((10, 28, 'ADD'), (39, 28, 'REM'), (69, 28, 'SEL'),
                       (99, 24, 'MISC')):
        tab(pen, x, 79, w, 18, word, on=False, held=False, seed=500 + x,
            small=True)
    tab(pen, flap(228), 79, 27, 18, 'LIST', on=False, held=False, seed=600,
        small=True)
    # The playlist's own transport row: eight canvas pixels from canvas 139.
    for i, name in enumerate(('prev', 'play', 'pause', 'stop', 'next')):
        marked(pen, flap(139 + i * 9), 97, 8, 8, name, INK,
               marks=SMALL_MARKS)
    marked(pen, flap(185), 97, 12, 8, 'eject', INK, marks=SMALL_MARKS)
    # Where the player writes the times: two small patches of dyed cloth.
    for cx, cw, cy in ((130, 76, 347), (190, 34, 361)):
        dyed_panel(pen, flap(cx), cy - 267, cw, 12, seed=cx, thread=MUSTARD)
    # A cat asleep along the bottom of the left flap. It was on the right one
    # first, which put it exactly on top of the elapsed-time readout: the
    # footer's five menu buttons and six transport buttons have no sprites of
    # their own, so the only places a cat can lie are the ones nothing claims.
    grid(pen, 30, 97, cats.LOAF, cats._pal(cats.BLUE))
    commit(pen, 'Sampler · the playlist cloth, its rails, footer and seam')

    # The cloth under the track list: ruled with a stitch line every row, and a
    # margin thread down the left the way a ruled page has one.
    pen = sheet('plbg.bmp')
    linen(pen, 0, 0, 243, 203, seed=131, top=FLAX, bottom=shade(FLAX, .14),
          grain=8)
    for y in range(10, 203, 11):
        running(pen, 0, y, 243, shade(FLAX, .22), dash=1, gap=2)
    pen.rect(0, 0, 243, 1, FLAX_DEEP)
    pen.rect(0, 1, 243, 1, FLAX_PALE)
    running(pen, 6, 0, 203, shade(MADDER, .40), axis='y', dash=2, gap=2)
    commit(pen, 'Sampler · ruled cloth under the track list')

    # The playing row: a length of ribbon laid across the list. It has to stay
    # pale, because the playlist ink written on it is dark.
    pen = sheet('plselection.bmp')
    ribbon(pen, 0, 0, 243, 11, MUSTARD, seed=61, selvedge=MUSTARD_DARK)
    running(pen, 0, 1, 243, lighten(MUSTARD, .35), dash=1, gap=2)
    running(pen, 0, 9, 243, MUSTARD_DARK, dash=1, gap=2)
    commit(pen, 'Sampler · a ribbon marks the playing row')


# -------------------------------------------------------------------- palettes


@stage
def palettes():
    """The two text files: the playlist's ink, and the spectrum's floss."""
    call('studio_options', {'playlist_colors': {
        'Normal': INK, 'Current': MADDER_DARK, 'NormalBG': FLAX,
        'SelectedBG': MUSTARD, 'MbFG': CREAM, 'MbBG': INDIGO}})
    # The spectrum rises through the evenweave, so it is worked in the same
    # six colours the rest of the sampler is: olive at the bottom through
    # mustard to madder at the top, which is how a shaded thread is dyed.
    spectrum = [hexcolor(mix(MADDER, OLIVE, i / 15)) for i in range(16)]
    colors = ([INDIGO_DEEP, INDIGO_LIT] + spectrum
              + [CREAM, MUSTARD, MADDER_LIT, TEAL_LIT, OLIVE] + [CREAM])
    call('studio_options', {'visualizer_colors': colors[:24]})
    call('studio_options', {'visualizer_glass': True})
    print('  palettes: playlist ink, and a shaded floss for the spectrum')


@stage
def export():
    path = str(Path(__file__).resolve().parents[2]
               / 'assets/skins/Catamp Sampler.wsz')
    print(' ', call('studio_export', {'path': path})['content'][0]['text'])


ORDER = ['titlebar', 'main', 'transport', 'switches', 'seek', 'sliders',
         'readouts', 'eq', 'playlist', 'palettes']

if __name__ == '__main__':
    cats.check()
    for name in sys.argv[1:] or ORDER:
        print(name)
        STAGES[name]()
