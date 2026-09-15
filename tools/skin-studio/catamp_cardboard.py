#!/usr/bin/env python3
"""Catamp Cardboard -- a corrugated-box skin, drawn through Cranamp Skin Studio.

Every Catamp so far has been made of something cold and expensive: glass,
silver, crystal, a lake at night. This one is made of the thing a cat actually
wants, which is a box. The whole player is one piece of kraft board with
windows cut in it, and the cats are where cats are when they are in a box:
mostly out of sight. A paw lies along the open flap. Eleven heads come up
through eleven slots in the lid, which is the equalizer. A kitten walks the
seek bar. One cat sits so far back in the box that only its outline and its
eyes catch the light, behind the track title. Another is asleep in the corner
of the playlist.

The light has two rules and never breaks them: the outside of the box is lit
from above-left, and the inside of the box is lit only by the amber glow that
leaks out of it -- which is why every live readout Cranamp draws (the timer,
the spectrum, the track title, the playlist text) sits inside a cut window,
where a warm glow on dark board gives it more contrast than ink on kraft ever
could.

    python3 tools/skin-studio/catamp_cardboard.py            # build it all
    python3 tools/skin-studio/catamp_cardboard.py main eq     # one stage at a time

Requires a running `cranamp --skin-studio`, and nothing else: no image library,
no external asset. Every pixel is a native Studio operation -- a pen stroke, a
palette ramp, a hand-authored grid, or the engine's own grain and opacity -- so
nothing here resizes, resamples or blurs, and the same recipe runs wherever
Studio does.
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from cardboard_material import (DARK, DEEP, EDGE, GLOW, GLOW_DIM, GLOW_HOT,
                                HOLE, HOLE_DEEP, INK, INK_SOFT, KEY, KRAFT, LIT,
                                PALE, PAD, PENCIL, STAMP, STAMP_DIM, TAPE,
                                TAPE_DARK, TAPE_EDGE, TAPE_LIT, EYE, EYE_DIM,
                                FUR, FUR_DARK, FUR_LIT, FUR_PALE, FUR_SHADE,
                                cut_hole, digit_rows, flutes, glow, hexcolor,
                                kraft, micro, micro_width, mix, shade, stamped,
                                tape, text_width)
import cardboard_cats as cats
from pixel_pen import Pen, call, ramp

CATS = cats.PALETTE
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


def grid(pen, x, y, rows, palette=None):
    pen.stamp(int(x), int(y), rows, palette or CATS)
    return pen


# ---------------------------------------------------------------- box surfaces


def box_front(pen, x, y, w, h, *, seed, top=LIT, bottom=DARK):
    kraft(pen, x, y, w, h, seed=seed, top=top, bottom=bottom)
    return pen


def cut_edge_band(pen, x, y, w, *, phase=0):
    """A horizontal cut through the board: fibre, flute core, then shadow."""
    pen.rect(x, y, w, 1, EDGE)
    flutes(pen, x, y + 1, w, 1, phase=phase, colors=(PALE, LIT, DARK))
    pen.rect(x, y + 2, w, 1, shade(KRAFT, .28))
    return pen


def fold(pen, x, y, h, *, side='left'):
    """A scored fold: the board bends, so one pixel catches and one loses."""
    if side == 'left':
        pen.rect(x, y, 1, h, DEEP)
        pen.rect(x + 1, y, 1, h, DARK)
        pen.rect(x + 2, y, 1, h, shade(KRAFT, .10))
    else:
        pen.rect(x, y, 1, h, DEEP)
        pen.rect(x - 1, y, 1, h, DARK)
        pen.rect(x - 2, y, 1, h, shade(KRAFT, .10))
    return pen


def paw_print(pen, x, y, color=None):
    """The ink mark a cat leaves on a box it has walked over."""
    grid(pen, x, y, cats.PRINT_MARK, {'p': color or shade(INK, .30)})
    return pen


def cat_postmark(pen, x, y, w, h, *, color=STAMP, paper=KRAFT, wear=.22):
    """A round rubber stamp with a cat's face in it, half missing the way a
    stamp on board always is."""
    # A ring and a filled head. An inner ring and a detailed face were tried
    # first: at twenty-one pixels across, every extra mark subtracts a mark you
    # could already read, and the stamp arrived as a scribble.
    cx, cy = x + w / 2 - .5, y + h / 2 - .5
    pen.ellipse(x, y, w, h, color, fill=False, width=1)
    face = [' o     o ', ' oo   oo ', ' ooooooo ', 'oo o o oo',
            'ooooooooo', ' ooooooo ', '  ooooo  ']
    grid(pen, round(cx - 4), round(cy - 3), face, {'o': color})
    import random
    rnd = random.Random(int(x * 7 + y))
    for _ in range(int(w * h * wear)):
        pen.pixel(x + rnd.randrange(w), y + rnd.randrange(h), paper)
    return pen


def label_paper(pen, x, y, w, h, *, active=True, seed=17):
    """The shipping label glued across a title bar."""
    body = TAPE if active else shade(TAPE, .22)
    # No fibres on the label. A five-pixel one landed just after the wordmark
    # at the same height and read as a hyphen: "LIVE CATS-".
    kraft(pen, x, y, w, h, seed=seed,
          top=TAPE_LIT if active else shade(TAPE_LIT, .25),
          bottom=body, grain=4, fibres=0, flecks=w // 50)
    pen.rect(x, y, w, 1, TAPE_LIT if active else shade(TAPE_LIT, .3))
    pen.rect(x, y + h - 1, w, 1, TAPE_EDGE if active else shade(TAPE_EDGE, .3))
    pen.rect(x, y + h - 2, w, 1, TAPE_DARK if active else shade(TAPE_DARK, .3))
    return pen


def barcode(pen, x, y, h, width, *, color=INK, paper=TAPE, seed=9):
    import random
    rnd = random.Random(seed)
    cursor = x
    while cursor < x + width - 1:
        bar = rnd.choice((1, 1, 1, 2))
        pen.rect(cursor, y, min(bar, x + width - cursor), h, color)
        cursor += bar + rnd.choice((1, 2, 2))
    return pen


def window_button(pen, x, y, marks, *, held=False, paper=None, color=INK):
    """A window button printed on the label: ink, and a dent when it is held.

    These cells have no active and inactive variant of their own, so their
    paper is a tone between the two labels. Matching the active one left four
    bright buttons sitting on a dulled title bar.
    """
    paper = paper or shade(TAPE, .11)
    body = shade(paper, .18) if held else paper
    pen.rect(x, y, 9, 9, body)
    pen.rect(x, y, 9, 1, shade(paper, .3) if held else shade(TAPE_LIT, .11))
    pen.rect(x, y + 8, 9, 1, TAPE_EDGE if held else shade(paper, .16))
    dx = 1 if held else 0
    grid(pen, x + 2 + dx, y + 2 + dx, marks,
         {'i': color, 's': shade(color, -.45)})
    return pen


OPTIONS_MARKS = ['iiiii', '     ', 'iiiii', '     ', 'iiiii']
MINIMIZE_MARKS = ['     ', '     ', '     ', '     ', 'iiiii']
SHADE_MARKS = ['     ', 'iiiii', '     ', 'iiiii', '     ']
CLOSE_MARKS = ['i   i', ' i i ', '  i  ', ' i i ', 'i   i']
EQ_CLOSE_MARKS = ['i   i', ' i i ', '  i  ', ' i i ', 'i   i']


# ------------------------------------------------------------------- titlebar


def title_label(pen, x, y, *, active, word, stamp_word=None):
    """One 275x14 title bar: label paper, a postmark, a barcode, the wordmark."""
    ink = INK if active else INK_SOFT
    red = STAMP if active else STAMP_DIM
    paper = TAPE if active else shade(TAPE, .22)
    label_paper(pen, x, y, 275, 14, active=active, seed=17 + len(word))
    cat_postmark(pen, x + 17, y + 1, 21, 12, color=red, paper=paper,
                 wear=.16)
    barcode(pen, x + 44, y + 4, 6, 42, color=shade(ink, -.28), paper=paper,
            seed=len(word) * 3)
    pen.rect(x + 44, y + 11, 42, 1, shade(ink, -.35))
    mark = x + 100
    stamped(pen, mark, y + 4, word, color=ink, spacing=1, paper=paper, seed=4)
    width = text_width(word)
    pen.rect(mark, y + 2, width, 1, shade(ink, -.55))
    pen.rect(mark, y + 12, width, 1, shade(ink, -.55))
    if stamp_word:
        stamped(pen, mark + width + 10, y + 4, stamp_word, color=red,
                spacing=1, paper=paper, seed=8)
    return pen


@stage
def titlebar():
    """titlebar.bmp: the label, the four window buttons, both title states."""
    pen = sheet('titlebar.bmp')
    kraft(pen, 0, 0, 344, 87, seed=5)
    for row, active in ((0, True), (15, False)):
        title_label(pen, 27, row, active=active, word='CATAMP',
                    stamp_word='LIVE CATS')
        # Classic shade-mode bars, so a player that uses them is not left bare.
        title_label(pen, 27, row + 29, active=active, word='CATAMP')
    # Each button's two variants sit where the classic sheet puts them, which
    # is not one grid: options, minimize and close stack downward, shade goes
    # sideways. Walking one offset over all four wrote three of them on top of
    # each other and the fourth into the title strip at x27.
    for x, y, marks, held in ((0, 0, OPTIONS_MARKS, False),
                              (0, 9, OPTIONS_MARKS, True),
                              (9, 0, MINIMIZE_MARKS, False),
                              (9, 9, MINIMIZE_MARKS, True),
                              (18, 0, CLOSE_MARKS, False),
                              (18, 9, CLOSE_MARKS, True),
                              (0, 18, SHADE_MARKS, False),
                              (9, 18, SHADE_MARKS, True)):
        window_button(pen, x, y, marks, held=held)
    commit(pen, 'Cardboard · shipping label and window buttons')


# ------------------------------------------------------------- main.bmp scene

HATCH = (18, 25, 88, 34)
SHELF = (108, 25, 160, 28)
# What a box is actually printed with, on the flap where nothing else goes.
LEGEND = 'FRAGILE - DO NOT STACK - 11 CATS INSIDE'



@stage
def main():
    """main.bmp: the box front, its two cut windows, the flap paw, the tail."""
    pen = sheet('main.bmp')
    box_front(pen, 0, 0, 275, 115, seed=11, top=LIT, bottom=shade(DARK, .10))
    # The open flap across the top, and the shadow it throws on the front.
    cut_edge_band(pen, 0, 14, 275)
    pen.rect(0, 17, 275, 1, shade(KRAFT, .30))
    stamped(pen, (275 - text_width(LEGEND)) // 2, 17, LEGEND,
            color=shade(INK, .42), spacing=1, worn=0)
    fold(pen, 0, 14, 101, side='left')
    fold(pen, 274, 14, 101, side='right')
    # The bottom rim. Row 114 is also the docking edge the equalizer sits on,
    # so it has to read as the bottom of the box, not as a stray line.
    pen.rect(0, 110, 275, 1, shade(KRAFT, .34))
    cut_edge_band(pen, 0, 111, 275, phase=1)
    pen.rect(0, 114, 275, 1, DEEP)

    # Prints where a cat has walked across the front, and a postmark.
    for i, x in enumerate(range(22, 232, 47)):
        paw_print(pen, x, 95 + (i % 2) * 4, shade(INK, .44))

    # Two windows cut through the front: the timer and spectrum in one, the
    # track title and its readouts in the other. Live text on kraft would have
    # to fight the grain; warm light on dark board does not.
    cut_hole(pen, *HATCH, light=.20)
    glow(pen, 46, 25, 58, 18, peak=40)
    grid(pen, 36, 28, cats.EYES_IN_THE_DARK)
    cut_hole(pen, *SHELF, light=.14)
    glow(pen, 112, 25, 150, 14, peak=26)
    grid(pen, 178, 38, cats.CAT_IN_THE_DARK)

    # The tail, down the scored left edge, tapering to a curled tip. Four
    # strokes of falling width make a tube; one stroke makes a smear.
    pen.taper([15, 19], [1, 52], [14, 84], 9, FUR_SHADE)
    pen.taper([15, 21], [2, 52], [14, 81], 7, FUR_DARK)
    pen.taper([14, 24], [4, 52], [13, 78], 4, FUR)
    pen.taper([13, 28], [6, 52], [12, 72], 2, FUR_LIT)
    pen.taper([14, 80], [13, 92], [2, 90], 7, FUR_SHADE)
    pen.taper([14, 79], [12, 89], [3, 88], 5, FUR_DARK)
    pen.taper([13, 78], [11, 86], [5, 86], 2, FUR_LIT)

    # The routed tray the transport keys sit in, visible in the gaps between.
    pen.rect(16, 86, 222, 1, shade(KRAFT, .40))
    pen.rect(16, 87, 222, 1, shade(KRAFT, .18))
    pen.rect(16, 106, 222, 1, shade(PALE, .30))
    # Asleep beside the keys. This was tried in the playlist footer first,
    # where it lay across the SEL and MISC buttons: the classic footer has no
    # thirty-pixel gap, and the main window's switch row does.
    grid(pen, 241, 85, cats.CAT_CURLED)
    grid(pen, 249, 93, cats.SLEEPING_HEAD)
    commit(pen, 'Cardboard · box front, cut windows, flap paw and tail')


# ------------------------------------------------------------ transport plate


def stamped_button(pen, x, y, w, h, marks, *, held, paper_seed):
    """A transport key: a kraft plate routed into the tray, with a rubber-stamp
    symbol on it. Held presses the plate in, so the light swaps ends."""
    kraft(pen, x, y, w, h, seed=paper_seed,
          top=shade(LIT, .30) if held else LIT,
          bottom=shade(DARK, .20) if held else KRAFT, grain=5,
          fibres=2, flecks=1)
    top, bottom = (shade(KRAFT, .45), PALE) if held else (EDGE, shade(KRAFT, .40))
    pen.rect(x, y, w, 1, top)
    pen.rect(x, y + h - 1, w, 1, bottom)
    pen.rect(x, y, 1, h, shade(top, .12))
    pen.rect(x + w - 1, y, 1, h, shade(bottom, -.10))
    dx = 1 if held else 0
    mx = x + (w - len(marks[0])) // 2 + dx
    my = y + (h - len(marks)) // 2 + dx
    grid(pen, mx, my, marks, {'i': STAMP_DIM if held else STAMP,
                              's': shade(STAMP, .45)})
    return pen


PREV_MARKS = ['i    ii  ii', 'i   iii iii', 'i  iiiiiiii',
              'i iiiiiiiii', 'i  iiiiiiii', 'i   iii iii', 'i    ii  ii']
NEXT_MARKS = ['ii  ii    i', 'iii iii   i', 'iiiiiiii  i',
              'iiiiiiiii i', 'iiiiiiii  i', 'iii iii   i', 'ii  ii    i']
PLAY_MARKS = ['ii     ', 'iiii   ', 'iiiiii ', 'iiiiiii',
              'iiiiii ', 'iiii   ', 'ii     ']
PAUSE_MARKS = ['ii  ii', 'ii  ii', 'ii  ii', 'ii  ii', 'ii  ii', 'ii  ii',
               'ii  ii']
STOP_MARKS = ['iiiiiii'] * 7
EJECT_MARKS = ['   i   ', '  iii  ', ' iiiii ', 'iiiiiii', '       ', 'iiiiiii']
# The playlist footer's transport row gets eight pixels a button, so it needs
# its own glyphs rather than a crop of the big ones, which arrived as a smear.
SMALL_MARKS = {
    'prev': ['i  ii', 'i iii', 'iiiii', 'i iii', 'i  ii'],
    'play': ['i    ', 'iii  ', 'iiiii', 'iii  ', 'i    '],
    'pause': ['ii ii', 'ii ii', 'ii ii', 'ii ii', 'ii ii'],
    'stop': ['iiiii'] * 5,
    'next': ['ii  i', 'iii i', 'iiiii', 'iii i', 'ii  i'],
    'eject': ['  i  ', ' iii ', 'iiiii', '     ', 'iiiii'],
}


@stage
def transport():
    """cbuttons.bmp: five stamped keys, released and held."""
    pen = sheet('cbuttons.bmp')
    keys = ((0, 23, PREV_MARKS), (23, 23, PLAY_MARKS), (46, 23, PAUSE_MARKS),
            (69, 23, STOP_MARKS), (92, 22, NEXT_MARKS))
    for x, w, marks in keys:
        for row, held in ((0, False), (18, True)):
            stamped_button(pen, x, row, w, 18, marks, held=held,
                           paper_seed=x + row)
    for row, held in ((0, False), (16, True)):
        stamped_button(pen, 114, row, 22, 16, EJECT_MARKS, held=held,
                       paper_seed=200 + row)
    commit(pen, 'Cardboard · stamped transport keys')


# ----------------------------------------------------------------- switches


def switch_plate(pen, x, y, w, h, word, *, on, held, seed, extra=None):
    """A shuffle/repeat/EQ/PL switch. Off is a plate printed in dim ink; on is
    the plate with the ink freshly stamped and a warm edge, as though the light
    inside the box reached it."""
    kraft(pen, x, y, w, h, seed=seed,
          top=shade(LIT, .34) if held else LIT,
          bottom=shade(DARK, .18) if held else KRAFT, grain=5, fibres=2,
          flecks=1)
    top, bottom = (shade(KRAFT, .45), PALE) if held else (EDGE, shade(KRAFT, .40))
    pen.rect(x, y, w, 1, top)
    pen.rect(x, y + h - 1, w, 1, bottom)
    pen.rect(x, y, 1, h, shade(top, .14))
    pen.rect(x + w - 1, y, 1, h, shade(bottom, -.10))
    dx = 1 if held else 0
    ink = (STAMP if not held else STAMP_DIM) if on else shade(INK, .28)
    width = text_width(word)
    tx = x + (w - width) // 2 + dx
    ty = y + (h - 7) // 2 + dx
    stamped(pen, tx, ty, word, color=ink, spacing=1,
            paper=shade(KRAFT, .1), worn=.10 if on else .30, seed=seed)
    if on:
        pen.rect(x + 1, y + h - 2, w - 2, 1, shade(GLOW, .30))
    if extra:
        extra(pen, x + dx, y + dx, on)
    return pen


@stage
def switches():
    """shufrep.bmp: shuffle, repeat and the two window switches."""
    pen = sheet('shufrep.bmp')
    kraft(pen, 0, 0, 92, 85, seed=31)
    for row, (on, held) in enumerate(((False, False), (False, True),
                                      (True, False), (True, True))):
        y = row * 15
        switch_plate(pen, 0, y, 28, 15, 'RPT', on=on, held=held, seed=40 + row)
        switch_plate(pen, 28, y, 47, 15, 'SHUFFLE', on=on, held=held,
                     seed=60 + row)
    for (x, y), (on, held) in zip(((0, 61), (46, 61), (0, 73), (46, 73)),
                                  ((False, False), (False, True),
                                   (True, False), (True, True))):
        switch_plate(pen, x, y, 23, 12, 'EQ', on=on, held=held, seed=80 + x + y)
    for (x, y), (on, held) in zip(((23, 61), (69, 61), (23, 73), (69, 73)),
                                  ((False, False), (False, True),
                                   (True, False), (True, True))):
        switch_plate(pen, x, y, 23, 12, 'PL', on=on, held=held, seed=90 + x + y)
    commit(pen, 'Cardboard · shuffle, repeat and window switches')


# ------------------------------------------------------------------ seek tape


@stage
def seek():
    """posbar.bmp: a strip of packing tape across the box, and the kitten that
    walks it."""
    pen = sheet('posbar.bmp')
    pen.rect(0, 0, 307, 10, KRAFT)
    kraft(pen, 0, 0, 248, 10, seed=77, top=LIT, bottom=KRAFT, grain=6)
    pen.rect(0, 0, 248, 1, shade(KRAFT, .36))
    pen.rect(0, 1, 248, 1, shade(KRAFT, .14))
    tape(pen, 0, 2, 248, 6, seed=21)
    pen.rect(0, 8, 248, 1, shade(KRAFT, .34))
    pen.rect(0, 9, 248, 1, shade(KRAFT, .16))
    for x in range(6, 244, 29):
        pen.rect(x, 3, 1, 3, shade(TAPE_DARK, .12))
    for x, rows in ((248, cats.KITTEN_WALK), (278, cats.KITTEN_HELD)):
        kraft(pen, x, 0, 29, 10, seed=x, top=LIT, bottom=KRAFT, grain=6)
        pen.rect(x, 0, 29, 1, shade(KRAFT, .36))
        pen.rect(x, 1, 29, 1, shade(KRAFT, .14))
        tape(pen, x, 2, 29, 6, seed=x, torn=False)
        pen.rect(x, 8, 29, 1, shade(KRAFT, .34))
        pen.rect(x, 9, 29, 1, shade(KRAFT, .16))
        grid(pen, x, 0, rows)
    commit(pen, 'Cardboard · seek tape and the kitten on it')


# --------------------------------------------------------------- slider paws


def groove(pen, x, y, w, h, *, fill, centre=None):
    """A channel routed into the board, with the glow from inside the box
    showing through however far the slider has been pushed."""
    kraft(pen, x, y, w, h, seed=x * 13 + y, top=LIT, bottom=KRAFT, grain=5,
          fibres=2, flecks=1)
    top, bottom = y + 3, 7
    pen.rect(x + 2, top, w - 4, bottom, HOLE)
    pen.rect(x + 2, top, w - 4, 2, HOLE_DEEP)
    pen.rect(x + 2, top + bottom - 1, w - 4, 1, shade(GLOW_DIM, .30))
    pen.rect(x + 1, top - 1, w - 2, 1, EDGE)
    pen.rect(x + 1, top + bottom, w - 2, 1, shade(PALE, .22))
    pen.rect(x + 1, top - 1, 1, bottom + 2, shade(EDGE, .14))
    pen.rect(x + w - 2, top - 1, 1, bottom + 2, shade(PALE, .26))
    # The light inside the box is brightest where the slider has got to, so the
    # filled part of the groove is a ramp, not a flat brown bar -- which is what
    # it read as, being a shade away from the kraft around it.
    warm = ramp([(0, shade(GLOW_DIM, .30)), (.55, GLOW_DIM), (1, GLOW)], 12)
    if centre is None:
        lit = max(0, min(w - 4, fill))
        if lit:
            pen.rect(x + 2, top + 1, lit, bottom - 2, GLOW_DIM, ramp=warm,
                     axis=[x + 2, 0, x + 2 + lit, 0])
            pen.rect(x + 2 + max(0, lit - 1), top + 1, 1, bottom - 2, GLOW_HOT)
    else:
        a, b = sorted((centre, centre + fill))
        a, b = max(0, a), min(w - 4, b)
        if b > a:
            # The bright end of the ramp is the slider, which is `b` when the
            # balance is right of centre and `a` when it is left, so the axis
            # turns round with it. Running it left-to-right regardless put the
            # brightest pixel at the detent for half the travel.
            near, far = (b, a) if fill < 0 else (a, b)
            pen.rect(x + 2 + a, top + 1, b - a, bottom - 2, GLOW_DIM,
                     ramp=warm, axis=[x + 2 + near, 0, x + 2 + far, 0])
            pen.rect(x + 2 + (b - 1 if fill > 0 else a), top + 1, 1,
                     bottom - 2, GLOW_HOT)
        pen.rect(x + 2 + centre, top, 1, 2, shade(GLOW_HOT, .10))
        pen.rect(x + 2 + centre, top + bottom - 2, 1, 2, shade(GLOW_HOT, .10))
    return pen


@stage
def sliders():
    """volume.bmp and balance.bmp: 28 groove frames each, and the paw."""
    pen = sheet('volume.bmp')
    for i in range(28):
        groove(pen, 0, i * 15, 68, 13, fill=round((i + 1) / 28 * 64))
    grid(pen, 15, 422, cats.SLIDER_LOAF)
    grid(pen, 0, 422, cats.SLIDER_LOAF_HELD)
    commit(pen, 'Cardboard · volume groove and paw')

    pen = sheet('balance.bmp')
    for i in range(28):
        offset = round((i - 13.5) / 13.5 * 17)
        groove(pen, 9, i * 15, 38, 13, fill=offset, centre=17)
    grid(pen, 15, 422, cats.SLIDER_LOAF)
    grid(pen, 0, 422, cats.SLIDER_LOAF_HELD)
    commit(pen, 'Cardboard · balance groove and paw')


# --------------------------------------------------------------- readouts


@stage
def numbers():
    """numbers.bmp: the timer, as light rather than as plates.

    The cells are keyed through, so the window they sit in provides the dark
    and the glow behind them shows around the strokes. A digit drawn as an
    opaque tile would have cut a rectangle out of the light.
    """
    pen = sheet('numbers.bmp')
    pen.rect(0, 0, 99, 13, KEY)
    for value in range(10):
        rows = digit_rows(value)
        x = value * 9 + 1
        for dx, dy in ((-1, 0), (1, 0), (0, -1), (0, 1)):
            grid(pen, x + dx, 1 + dy,
                 [row.replace('G', 'h') for row in rows],
                 {'h': shade(GLOW_DIM, .18)})
        grid(pen, x, 1, rows, {'G': GLOW_HOT})
        grid(pen, x, 1, [row.replace('G', 'e') if i in (0, 10) else
                         ''.join('e' if c == 'G' and j in (0, 6) else ' '
                                 for j, c in enumerate(row))
                         for i, row in enumerate(rows)], {'e': GLOW})
    # The eleventh cell is the classic blank; leave it keyed.
    commit(pen, 'Cardboard · glowing timer digits')

    pen = sheet('playpaus.bmp')
    pen.rect(0, 0, 42, 9, KEY)
    lamps = ((STAMP, 'i'), (GLOW, 'H'), (EYE, 'E'))
    for i, (core, mark) in enumerate(lamps):
        rows = ['  ss  ', ' sSSs ', f'sS{mark}{mark}Ss', f'sS{mark}{mark}Ss',
                ' sSSs ', '  ss  ']
        grid(pen, i * 9 + 1, 1, rows,
             {'s': shade(core, .60), 'S': shade(core, .28), 'i': core,
              'H': GLOW_HOT, 'E': EYE})
    commit(pen, 'Cardboard · status lamp')

    pen = sheet('monoster.bmp')
    pen.rect(0, 0, 56, 24, KEY)
    for y, lit in ((0, True), (12, False)):
        for x, cell, word in ((0, 29, 'STEREO'), (29, 27, 'MONO')):
            colour = GLOW_HOT if lit else shade(GLOW_DIM, .50)
            left = x + (cell - micro_width(word)) // 2
            if lit:
                for dx, dy in ((-1, 0), (1, 0), (0, -1), (0, 1)):
                    micro(pen, left + dx, y + 4 + dy, word,
                          shade(GLOW_DIM, .22))
            micro(pen, left, y + 4, word, colour)
    commit(pen, 'Cardboard · mono and stereo lamps')

    # text.bmp is not a glyph sheet in Cranamp -- it renders titles with its own
    # face and samples this sheet only for the display ink. Paint it as the
    # glyph sheet it looks like, in the ink the readouts should use: the second
    # most common colour on an opaque sheet is the one that is taken.
    pen = sheet('text.bmp')
    pen.rect(0, 0, 155, 18, HOLE)
    rows = ('ABCDEFGHIJKLMNOPQRSTUVWXYZ', '0123456789.-:()+=_!?&#%*/\\<>',
            'abcdefghijklmnopqrstuvwxyz')
    for i, line in enumerate(rows):
        stamped(pen, 1, i * 6, line[:31], color=GLOW_HOT, spacing=0, worn=0)
    commit(pen, 'Cardboard · display ink')


@stage
def eq():
    """eqmain.bmp: the box lid, eleven slots, eleven heads coming through."""
    pen = sheet('eqmain.bmp')
    box_front(pen, 0, 0, 275, 116, seed=23, top=LIT, bottom=shade(DARK, .12))
    cut_edge_band(pen, 0, 14, 275)
    pen.rect(0, 17, 275, 1, shade(KRAFT, .40))
    fold(pen, 0, 14, 102, side='left')
    fold(pen, 274, 14, 102, side='right')
    pen.rect(0, 111, 275, 1, shade(KRAFT, .34))
    cut_edge_band(pen, 0, 112, 275, phase=2)
    pen.rect(0, 115, 275, 1, DEEP)

    # The taped-on graph-paper strip the response curve is drawn on.
    tape(pen, 84, 15, 117, 23, seed=13)
    pen.rect(86, 17, 113, 19, shade(TAPE, .16))
    for x in range(86, 199, 8):
        pen.rect(x, 17, 1, 19, shade(PENCIL, -.40))
    for y in range(17, 36, 6):
        pen.rect(86, y, 113, 1, shade(PENCIL, -.40))
    pen.rect(86, 26, 113, 1, shade(PENCIL, -.10))

    # Eleven slots cut through the lid.
    for i, x in enumerate([21] + [78 + 18 * (i - 1) for i in range(1, 11)]):
        slot = x - 1
        pen.rect(slot, 37, 14, 64, KRAFT)
        cut_hole(pen, slot + 3, 39, 8, 60, floor=shade(GLOW_DIM, .20),
                 light=.34)
    # The lid is a folded flap, not a flat field: score it above and below the
    # slots, and run the fibre band along the join.
    pen.rect(12, 34, 251, 1, shade(KRAFT, .40))
    pen.rect(12, 35, 251, 1, shade(PALE, .22))
    pen.rect(12, 101, 251, 1, shade(KRAFT, .34))
    pen.rect(12, 102, 251, 1, shade(PALE, .20))
    pen.rect(72, 37, 1, 64, shade(KRAFT, .30))
    pen.rect(73, 37, 1, 64, shade(PALE, .20))
    for px, py in ((40, 48), (52, 62), (41, 76)):
        paw_print(pen, px, py, shade(INK, .48))

    # Labels: the classic band captions, printed on the lid.
    for i, word in enumerate(('60', '170', '310', '600', '1K', '3K', '6K',
                              '12K', '14K', '16K')):
        x = 78 + 18 * i
        stamped(pen, x + (11 - text_width(word)) // 2 - 1, 103, word,
                color=shade(INK, .18), spacing=0, worn=0)
    stamped(pen, 16, 103, 'PRE', color=shade(INK, .18), spacing=0, worn=0)

    for row, active in ((134, True), (149, False)):
        title_label(pen, 0, row, active=active, word='EQUALIZER')
    for row, held in ((116, False), (125, True)):
        window_button(pen, 0, row, EQ_CLOSE_MARKS, held=held)

    switches = (('ON', 10, 26), ('AUTO', 36, 32))
    for word, x, w in switches:
        for (cx, cy), (on, held) in zip(
                ((x, 119), (x + 118, 119), (x + 59, 119), (x + 177, 119)),
                ((False, False), (False, True), (True, False), (True, True))):
            switch_plate(pen, cx, cy, w, 12, word, on=on, held=held,
                         seed=cx + cy)
    for y, held in ((164, False), (176, True)):
        switch_plate(pen, 224, y, 44, 12, 'PRESETS', on=False, held=held,
                     seed=300 + y)

    # 28 track frames. The slot itself does not change with the value -- the
    # head that rises out of it does -- so every frame is the same cut, which
    # is what keeps eleven of them from flickering against each other.
    for i in range(28):
        x, y = 13 + 15 * (i % 14), 164 + 65 * (i // 14)
        pen.rect(x, y, 14, 63, KRAFT)
        kraft(pen, x, y, 14, 63, seed=400 + i, top=LIT, bottom=KRAFT, grain=5,
              fibres=3, flecks=2)
        cut_hole(pen, x + 3, y + 2, 8, 59, floor=shade(GLOW_DIM, .20),
                 light=.34)
    grid(pen, 0, 164, cats.EQ_HEAD)
    grid(pen, 0, 176, cats.EQ_HEAD_HELD)

    # The response curve, drawn in pencil on the taped strip.
    pen.rect(0, 294, 113, 19, KEY)
    for x in range(113):
        pen.pixel(x, 9, PENCIL)
    pen.rect(0, 314, 113, 1, shade(PENCIL, -.2))
    commit(pen, 'Cardboard · box lid, eleven slots and eleven heads')


# ------------------------------------------------------------------- playlist


def flap(canvas_x):
    """The sheet column for a canvas column on the playlist's right flap.

    bottom.left is sheet 0..124 drawn at canvas 0..124; bottom.right is sheet
    126..275 drawn at canvas 125..274. Sheet column 125 is never drawn, so
    everything on the right flap is a pixel along, and arithmetic done by eye
    lands a button a pixel out of step with its own hit area.
    """
    return canvas_x + 1


def plain_cut(pen, x, y, w, *, phase=None):
    """A cut edge with no flute pattern, for anything that tiles.

    The header tile is 25 pixels wide and the flute period is three, so a
    fluted edge shifts a pixel at every one of the nine repeats and reads as a
    row of seams. Solid rows repeat at any width.
    """
    pen.rect(x, y, w, 1, EDGE)
    pen.rect(x, y + 1, w, 1, shade(PALE, .18))
    pen.rect(x, y + 2, w, 1, shade(KRAFT, .30))
    return pen


@stage
def playlist():
    """pledit.bmp: a flattened sheet of board, corrugated rails, a sleeping cat.

    Two things in this sheet are drawn nine times and twice: the 25-pixel header
    tile, and the two footer flaps, whose cells sit one pixel apart in the sheet
    and touch on the canvas. Nothing continuous may cross either boundary, so
    the footer is two flaps that meet at a fold -- which is what the bottom of
    a box looks like anyway.
    """
    call('studio_options', {'playlist_background': True,
                            'playlist_selection': True})
    pen = sheet('pledit.bmp')
    kraft(pen, 0, 0, 280, 190, seed=41)

    for row, active in ((0, True), (21, False)):
        for x, w in ((0, 25), (127, 25), (153, 25)):
            kraft(pen, x, row, w, 20, seed=50 + x + row, top=LIT, bottom=KRAFT)
            plain_cut(pen, x, row, w)
            pen.rect(x, row + 18, w, 1, shade(KRAFT, .22))
            pen.rect(x, row + 19, w, 1, shade(KRAFT, .38))
        fold(pen, 0, row, 20, side='left')
        fold(pen, 177, row, 20, side='right')
        # The title cell is a strip of tape with the word written on it.
        kraft(pen, 26, row, 100, 20, seed=70 + row, top=LIT, bottom=KRAFT)
        plain_cut(pen, 26, row, 100)
        pen.rect(26, row + 19, 100, 1, shade(KRAFT, .38))
        tape(pen, 30, row + 4, 92, 14, seed=33)
        title = 'PLAYLIST'
        stamped(pen, 30 + (92 - text_width(title)) // 2, row + 7, title,
                color=INK if active else INK_SOFT, spacing=1,
                paper=TAPE, worn=.18, seed=12)

    # Side rails: the exposed corrugated edge of the board. The flutes run
    # across the rail and are constant down it, so a rail repeated every 29
    # rows meets itself exactly at every phase.
    kraft(pen, 0, 42, 12, 29, seed=90, top=LIT, bottom=KRAFT, grain=5)
    flutes(pen, 1, 42, 9, 29, colors=(PALE, KRAFT, DARK))
    pen.rect(0, 42, 1, 29, DEEP)
    pen.rect(10, 42, 1, 29, shade(KRAFT, .40))
    pen.rect(11, 42, 1, 29, shade(DEEP, .30))
    # The right rail carries the scroll channel. Sheet x = 31 + (canvas - 255),
    # and the channel the player puts its handle in is canvas 260..267.
    kraft(pen, 31, 42, 20, 29, seed=94, top=LIT, bottom=KRAFT, grain=5)
    pen.rect(31, 42, 1, 29, shade(DEEP, .30))
    flutes(pen, 32, 42, 3, 29, colors=(PALE, KRAFT, DARK))
    pen.rect(35, 42, 1, 29, EDGE)
    pen.rect(36, 42, 8, 29, HOLE)
    pen.rect(36, 42, 1, 29, HOLE_DEEP)
    pen.rect(43, 42, 1, 29, shade(HOLE, -.14))
    pen.rect(44, 42, 1, 29, shade(PALE, .22))
    flutes(pen, 45, 42, 5, 29, colors=(PALE, KRAFT, DARK))
    pen.rect(50, 42, 1, 29, DEEP)
    for x, rows in ((52, cats.SCROLL_CAT), (61, cats.SCROLL_CAT_HELD)):
        cut_hole(pen, x + 1, 54, 6, 16, light=.22)
        grid(pen, x, 53, rows)

    # The footer: the box's two bottom flaps, folded down and meeting at a
    # fold. Everything on them is printed.
    for x, w, seed in ((0, 125, 110), (126, 150, 118)):
        kraft(pen, x, 72, w, 38, seed=seed, top=LIT, bottom=shade(DARK, .10))
        plain_cut(pen, x, 72, w)
        pen.rect(x, 105, w, 1, shade(KRAFT, .30))
        pen.rect(x, 106, w, 1, shade(PALE, .22))
        pen.rect(x, 107, w, 1, shade(KRAFT, .24))
        pen.rect(x, 108, w, 1, EDGE)
        pen.rect(x, 109, w, 1, shade(KRAFT, .40))
    fold(pen, 0, 72, 38, side='left')
    fold(pen, 275, 72, 38, side='right')
    # Where the two flaps meet: canvas 123 shadow, 124 the fold, 125 the lit
    # edge of the far flap -- which is sheet column 126.
    pen.rect(122, 72, 1, 38, shade(KRAFT, .22))
    pen.rect(123, 72, 1, 38, shade(KRAFT, .46))
    pen.rect(124, 72, 1, 38, DEEP)
    pen.rect(126, 72, 1, 38, shade(PALE, .28))
    pen.rect(127, 72, 1, 38, shade(KRAFT, .10))

    # ADD, REM and SEL fit on the left flap; MISC stops at the fold rather than
    # straddling it, because its two halves would arrive a pixel out of step.
    for x, w, word in ((10, 28, 'ADD'), (39, 28, 'REM'), (69, 28, 'SEL'),
                       (99, 24, 'MISC')):
        switch_plate(pen, x, 79, w, 18, word, on=False, held=False,
                     seed=500 + x)
    switch_plate(pen, 229, 79, 27, 18, 'LIST', on=False, held=False, seed=600)
    # The playlist's own transport row. Its buttons are eight canvas pixels
    # wide from canvas 139, and the right flap's sheet column is canvas + 1.
    for i, name in enumerate(('prev', 'play', 'pause', 'stop', 'next')):
        grid(pen, flap(139 + i * 9 + 1), 98, SMALL_MARKS[name], {'i': STAMP})
    grid(pen, flap(185), 98, SMALL_MARKS['eject'], {'i': STAMP})
    # Where the player writes the times: a shallow routed panel, not a hole.
    # A cut window here punched two black rectangles through the flap, and the
    # playlist ink Cranamp writes on them is dark.
    for cx, cw, cy in ((130, 76, 347), (190, 34, 361)):  # noqa: E501
        x, y = flap(cx), cy - 267
        pen.rect(x, y, cw, 12, shade(KRAFT, .14))
        pen.rect(x, y, cw, 1, shade(KRAFT, .40))
        pen.rect(x, y + 11, cw, 1, shade(PALE, .22))
        pen.rect(x, y, 1, 12, shade(KRAFT, .32))
        pen.rect(x + cw - 1, y, 1, 12, shade(PALE, .16))
    cat_postmark(pen, 259, 80, 16, 18, color=STAMP_DIM, paper=KRAFT,
                 wear=.16)
    commit(pen, 'Cardboard · playlist board, rails, footer and the sleeping cat')

    pen = sheet('plbg.bmp')
    kraft(pen, 0, 0, 243, 203, seed=131, top=KRAFT, bottom=shade(DARK, .12))
    for y in range(0, 203, 13):
        pen.rect(0, y, 243, 1, shade(KRAFT, .13))
    pen.rect(0, 0, 243, 1, shade(KRAFT, .44))
    pen.rect(0, 1, 243, 1, shade(KRAFT, .20))
    pen.rect(7, 0, 1, 203, shade(STAMP_DIM, .50))
    commit(pen, 'Cardboard · ruled board under the track list')

    # The playing row: a strip of tape laid over the board. It has to stay
    # pale, because the playlist ink written on it is dark.
    pen = sheet('plselection.bmp')
    tape(pen, 0, 0, 243, 11, seed=61, torn=False)
    pen.rect(0, 0, 243, 1, TAPE_LIT)
    pen.rect(0, 10, 243, 1, TAPE_EDGE)
    commit(pen, 'Cardboard · a strip of tape marks the playing row')


# -------------------------------------------------------------------- palettes


@stage
def palettes():
    """The two text files: playlist ink, and the spectrum's warm gradient."""
    # The list sits on kraft, not on a dark panel, so its ink is ink.
    call('studio_options', {'playlist_colors': {
        'Normal': '#3d2c1d', 'Current': '#a8402f', 'NormalBG': KRAFT,
        'SelectedBG': TAPE, 'MbFG': '#3d2c1d', 'MbBG': KRAFT}})
    warm = ['#241811', '#3a2617'] + [hexcolor(mix('#ff8a1e', GLOW_HOT, t / 13))
                                     for t in range(14)] + \
           ['#ffdf9e', '#ffb54a', '#f2892a', '#d9701f', '#b8571a', '#8d5b24',
            '#6a3c17', '#ffe6ae']
    call('studio_options', {'visualizer_colors': warm[:24]})
    print('  palettes: playlist ink and a warm spectrum')


@stage
def export():
    path = str(Path('assets/skins/Catamp Cardboard.wsz').resolve())
    print(' ', call('studio_export', {'path': path})['content'][0]['text'])


ORDER = ['titlebar', 'main', 'transport', 'switches', 'seek', 'sliders',
         'numbers', 'eq', 'playlist', 'palettes']

if __name__ == '__main__':
    cats.check()
    names = sys.argv[1:] or ORDER
    for name in names:
        print(name)
        STAGES[name]()
