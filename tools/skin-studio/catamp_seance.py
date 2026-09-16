#!/usr/bin/env python3
"""Catamp Seance -- the player as a table at seven minutes past three.

Nine cats have got the spirit board out. There are candles, because a cat will
knock a candle over but will not put one out. There is spilled salt, and the
sigils are drawn in it, badly, by a paw. The controls are not buttons dressed up
as objects; they are the objects. The seek bar is the board, with the planchette
sliding along the alphabet. The volume is how many cats have woken up and turned
round to look at you. The balance is the saucer of milk, and it tips. The
equalizer is eleven candles on the mantelpiece, and each one burns down to its
own band.

And the thing they are calling up is the small red dot.

    python3 tools/skin-studio/catamp_seance.py             # build it all
    python3 tools/skin-studio/catamp_seance.py main eq     # one stage

Requires a running `cranamp --skin-studio` and nothing else: no image library,
no external asset. Every pixel is a native Studio operation.
"""
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import seance_cats as C
import seance_room as R
from pixel_pen import Pen, answer, call

STAGES = {}


def stage(fn):
    STAGES[fn.__name__] = fn
    return fn


def sheet(name):
    call('studio_atlas', {'sheet': name})
    return Pen()


def canvas(**view):
    """Back to the joined skin, with the sprite states the next stroke means.

    Most of this recipe draws on the canvas rather than in `studio_atlas`, and
    the reason is `layers`: named there, a stroke is clipped to those sprites
    and the result says how much fell outside. On a sheet opened on its own
    there are no sprites to name, so a pool of candlelight laid across the title
    row spills into the four window-key cells beside it and every report comes
    back clean -- the ink is a legal part of some cell, which is exactly the
    trap the catalogue exists to close. `active` and `pressed` say which variant
    the canvas is showing, and that is the variant a stroke lands in, so two
    passes with the same code draw a lit title and a dark one.
    """
    call('studio_atlas', {})
    if view:
        call('studio_canvas', view)
    return Pen()


def into(pen, label, *sprites, **draw):
    """Commit a stroke routed into named sprites and clipped to them."""
    args = dict(label=label, operations=pen.ops, layers=list(sprites))
    args.update(draw)
    result = answer('studio_draw', args)
    told = {k: result[k] for k in
            ('clipped_pixels', 'overwrites', 'unsampled_pixels', 'unmapped_pixels')
            if result.get(k)}
    for k in ('identical_variants', 'crossed_cells', 'unsupported_characters'):
        if result.get(k):
            told[k] = result[k]
    print(f'  {label}: {result.get("pixels_written")} px'
          + (f'  !! {told}' if told else ''))
    return result


def variants(sprite):
    """Where every cell of a sprite actually lives, asked rather than worked
    out. Twenty-eight slider frames sit at two sheet rows with a stride that
    changes halfway, and a recipe that computes them lands all twenty-eight on
    frame 0 -- legal ink in a legal place, so nothing reports it and the control
    simply never changes."""
    found_in = answer('studio_targets', {'id': sprite, 'variants': True})
    for found in found_in['sprites']:
        if found['id'].endswith(sprite):
            return list(zip(found['labels'], found['variants']))
    raise KeyError(f'no sprite matching {sprite}')


def commit(pen, label):
    result = answer('studio_draw', {'label': label, 'operations': pen.ops})
    told = {k: result[k] for k in
            ('clipped_pixels', 'overwrites', 'unsampled_pixels', 'unmapped_pixels')
            if result.get(k)}
    for k in ('identical_variants', 'crossed_cells', 'unsupported_characters'):
        if result.get(k):
            told[k] = result[k]
    print(f'  {label}: {result.get("pixels_written")} px'
          + (f'  !! {told}' if told else ''))
    return result


def measure(word, *, face='5x7', scale=1):
    """How wide a word actually comes out, from the engine's own glyph walk.

    Every recipe in this tree used to carry its own copy of this arithmetic and
    every copy had it wrong the same way: the pen advances (cell+spacing)*scale,
    not cell*scale+spacing, and those agree at scale 1 and nowhere else.
    """
    answer('studio_canvas', {'face': face, 'text_scale': scale})
    got = answer('studio_canvas', {'measure': word})['measured'][0]
    return got['width'], got['height']


def centred(pen, x, y, w, word, color, *, face='5x7', scale=1, opacity=None):
    width, _ = measure(word, face=face, scale=scale)
    op = dict(op='text', x=int(x + (w - width) // 2), y=int(y), text=word,
              color=color, face=face, scale=scale)
    if opacity:
        op['opacity'] = opacity
    pen.ops.append(op)
    return pen


# ------------------------------------------------------------------ the table

# Where the light in the main window comes from. There are no other lights, and
# every rim, every pool and every colour in the window is measured from these
# three: two candles standing on the table and the row of them along its far
# edge, which lives in the title bar and goes out when the window loses focus.
CANDLE_A = (105.0, 38.0)
CANDLE_B = (199.0, 44.0)
FAR_EDGE = (34.0, 4.0)


@stage
def main():
    """The table: cloth, two candles, the salt they knocked over, and the cats
    that are not near enough to a flame to be more than a pair of eyes."""
    pen = sheet('main.bmp')
    R.cloth(pen, 0, 0, 275, 115, seed=11)
    # The near edge of the table is further from every flame than the far edge,
    # so it is darker -- and that is the only reason the window has any depth.
    # Six bands rather than one: a single rectangle of shadow lays a hairline
    # across the table at its own top edge, and the eye finds it instantly.
    for i in range(6):
        pen.ops.append(dict(op='rect', x=0, y=62 + i * 9, width=275, height=115 - 62 - i * 9,
                            color=R.CLOTH_DEEP, fill=True, opacity=26 + i * 7,
                            grain=5, grain_size=2, grain_seed=5 + i))

    # What the candles along the far edge throw down the table. Wide, weak, and
    # nowhere near the readouts.
    R.pool(pen, 34, 6, 86, 40, peak=7, span=6, rings=11, strength=170, seed=4)
    R.pool(pen, 236, 5, 54, 30, peak=6, span=5, rings=9, strength=150, seed=6)

    # Candle A, the one the board is laid out around. It stands in the eleven
    # pixels between the spectrum and the bitrate readout, which is the only gap
    # in the top half of this window a candle fits in.
    R.pool(pen, 105, 52, 46, 19, peak=8, span=6, rings=12, strength=205, seed=1)
    R.wax_pool(pen, 105, 55, 6, seed=2)
    R.candle_body(pen, 102, 41, 7, 15, seed=3)
    R.lit(pen, 105, 41, height=7, width=4, reach=12, seed=1)

    # Candle B, a stub in the gap between the sample-rate readout and the mono
    # lamp. It is guttering, so it leans.
    R.pool(pen, 199, 52, 30, 14, peak=7, span=5, rings=9, strength=190, seed=7)
    R.wax_pool(pen, 199, 53, 5, seed=8)
    R.candle_body(pen, 196, 47, 6, 7, seed=9)
    R.lit(pen, 199, 47, height=5, width=3, lean=1.1, reach=9, seed=2)

    # The salt cellar, on its side in the near corner, which is where the salt
    # in this skin comes from: every mark in the window is drawn in it.
    pen.ops.append(dict(op='rect', x=0, y=99, width=13, height=7, color=R.WAX_DIM,
                        fill=True, ramp=R.ramp_between(R.WAX, R.GLOOM, 6),
                        ramp_axis=[0, 99, 0, 106]))
    pen.ops.append(dict(op='rect', x=11, y=98, width=4, height=9, color=R.SALT_DIM, fill=True))
    pen.ops.append(dict(op='rect', x=0, y=99, width=13, height=1, color=R.SALT, fill=True,
                        opacity=120))
    R.spill(pen, 20, 105, 16, seed=21, grains=130, shade=R.SALT_DIM)
    R.spill(pen, 46, 101, 26, seed=22, grains=90, shade=R.SALT_DIM)
    R.spill(pen, 96, 98, 34, seed=23, grains=70, shade=R.SALT_DIM)
    R.spill(pen, 150, 96, 30, seed=24, grains=40, shade=R.SALT_DIM)
    R.spill(pen, 60, 84, 30, seed=25, grains=36, shade=R.SALT_DIM)

    # Three claw marks dragged along the near edge, from the last time this
    # went well.
    for i, (x0, y0) in enumerate(((98, 107), (99, 110), (101, 113))):
        R.salt_curve(pen, (x0, y0), (x0 + 11, y0 - 2), (x0 + 24, y0 - 1),
                     shade=R.CLOTH_NAP, seed=40 + i, weight=0.8, scatter=0)

    # The cats. Two of them are near enough to a candle to have an edge; the
    # rest of the room is eyes, and each pair is a whole cat.
    # A tail up the left edge, and its owner off the side of the window. The
    # margin is twenty-two pixels wide and a whole cat squeezed into it comes
    # out as a cat-shaped smear; one tail says the same thing and says it at
    # the size the space actually is.
    C.tail(pen, (4, 112), (26, 74), (2, 40), CANDLE_A, ground=R.GLOOM,
           width=6.0, reach=7, radius=90, seed=53)

    # The one behind candle B. It is on the far side of the flame, so the side
    # facing the room is the side with no light on it at all: a hole with two
    # eyes in it, floating over the fire. Rim-lighting this one would mean
    # lighting the half of it the candle cannot reach.
    C.dark_cat(pen, C.place(C.LOAF, 184, 55, scale=25, flip=True),
               ground=R.UMBER, gaze=(186, 42), gap=3, w=3, h=3)

    # The one sitting on the corner you change skins with, back to the room,
    # watching the board. Nothing is behind it, so it is a hole with ears.
    C.dark_cat(pen, C.place(C.BEHIND, 260, 110, scale=28), ground=R.DUSK)

    R.eyes(pen, 243, 20, gap=4, w=3, h=3)
    R.eyes(pen, 264, 29, gap=3, w=2, h=2)
    R.eyes(pen, 213, 24, gap=2, w=1, h=1, shade=R.EYE_DIM)
    # Two under the table, looking up through the near edge.
    R.eyes(pen, 119, 109, gap=3, w=2, h=2, shade=R.EYE_DIM)
    R.eyes(pen, 172, 111, gap=2, w=1, h=1, shade=R.EYE_DIM)

    # The colon between the two pairs of timer digits. The digits are sprites
    # and this is not: numbers.bmp has ten cells and no eleventh for a colon, so
    # every classic skin paints it on the window behind them. Without it the
    # timer reads `00 00`.
    for row in (30, 35):
        pen.rect(72, row, 2, 2, R.DISPLAY)
        pen.rect(71, row - 1, 4, 4, R.SCORCH)
        pen.rect(72, row, 2, 2, R.DISPLAY)

    # The front edge of the table, the one thing in the window between you and
    # everything else.
    pen.ops.append(dict(op='rect', x=0, y=113, width=275, height=2, color=R.CLOTH_NAP,
                        fill=True, opacity=150))

    commit(pen, 'seance · the cloth and the two candles on it')




# --------------------------------------------------------------- the far edge


def _far_edge(pen, x, y, *, alight):
    """The far edge of the table, and the candles standing along it.

    This strip is where the main window's light comes from, which makes it the
    one place in Catamp where losing focus changes the weather: the focused
    title has four candles burning on it and the unfocused one has four wicks
    and some smoke. Every other skin dims its unfocused title by a shade or two,
    which is a true thing to do to a lit object and a meaningless one to do to a
    room whose only light is the thing being dimmed.
    """
    wall = R.DUSK if alight else R.CLOTH_DEEP
    pen.ops.append(dict(op='rect', x=x, y=y, width=275, height=14, color=wall,
                        fill=True, grain=7, grain_size=2, grain_seed=31))
    if alight:
        R.pool(pen, x + 34, y + 12, 44, 9, peak=7, span=5, rings=8, strength=150, seed=12)
        R.pool(pen, x + 236, y + 12, 26, 7, peak=6, span=4, rings=6, strength=130, seed=13)
    # The table's own edge: the one horizontal in the skin, and it is what makes
    # the strip read as a table rather than as a bar across the top.
    pen.ops.append(dict(op='rect', x=x, y=y + 12, width=275, height=1,
                        color=R.nearer(R.CLOTH_NAP, 2 if alight else 0), fill=True,
                        opacity=170))

    for i, (cx, height) in enumerate(((26, 6), (34, 8), (43, 5), (236, 7))):
        pen.ops.append(dict(op='rect', x=x + cx - 1, y=y + 12 - height, width=3,
                            height=height, color=R.WAX if alight else R.WAX_DIM,
                            fill=True,
                            ramp=R.ramp_between(R.WAX_LIT if alight else R.WAX_DIM,
                                                R.UMBER if alight else R.CLOTH_DEEP, 5),
                            ramp_axis=[x + cx, y + 12 - height, x + cx, y + 12]))
        if alight:
            R.lit(pen, x + cx, y + 11 - height, height=4, width=2,
                  lean=(-0.6, 0.5, 0.9, -0.4)[i], reach=7, seed=i)
        else:
            R.snuffed(pen, x + cx, y + 11 - height, drift=2 - i % 3, seed=i)

    # Two cats along the back of the table. Eyeshine does not care whether the
    # window has focus, so the unfocused title is a dark strip with four wicks
    # and four eyes in it, which is exactly what the room looks like.
    R.eyes(pen, x + 218, y + 4, gap=4, w=3, h=3)
    R.eyes(pen, x + 204, y + 7, gap=2, w=1, h=1, shade=R.EYE_DIM)

    ink = R.SALT_LIT if alight else R.SALT_DIM
    centred(pen, x, y + 4, 275, 'CATAMP SEANCE', ink)
    R.spill(pen, x + 138, y + 11, 22, seed=14, grains=26,
            shade=R.SALT_DIM if alight else R.SALT_SHADOW)
    return pen


def _key_mark(pen, x, y, kind, *, pressed):
    """One of the four keys on the table's edge, drawn in salt.

    Pressed is not a darker mark here. The whole skin is a room being lit by
    small fires, so a pressed control is one that has just caught: the grains go
    up the salt ladder to the top of it and the cell gets a flare it does not
    have when it is resting. A bevel flip would move four pixels on a nine-pixel
    cell and say nothing.
    """
    pen.rect(x, y, 9, 9, R.KEY)
    shade = R.SALT_HOT if pressed else R.SALT
    seed = {'options': 1, 'minimize': 2, 'shade': 3, 'close': 4}[kind] * 9
    if pressed:
        R.pool(pen, x + 4, y + 4, 4, 4, peak=8, span=6, rings=6, strength=200,
               seed=seed, over=R.CLOTH_DEEP)
    if kind == 'options':
        for i in range(3):
            R.salt(pen, [(x + 2 + i * 2.2, y + 4)], shade=shade, seed=seed + i,
                   weight=1.0, scatter=0.9)
            pen.rect(x + 2 + i * 2, y + 4, 1, 1, shade)
    elif kind == 'minimize':
        R.salt_line(pen, (x + 2, y + 6), (x + 6, y + 6), shade=shade, seed=seed, weight=0.95)
    elif kind == 'shade':
        R.salt_curve(pen, (x + 2, y + 3), (x + 4, y + 7), (x + 6, y + 3),
                     shade=shade, seed=seed, weight=0.95)
    else:
        R.salt_line(pen, (x + 2, y + 2), (x + 6, y + 6), shade=shade, seed=seed, weight=0.95)
        R.salt_line(pen, (x + 6, y + 2), (x + 2, y + 6), shade=shade, seed=seed + 1, weight=0.95)
    return pen


@stage
def titlebar():
    """The far edge of the table, twice: once with the candles going and once
    after the draught got them."""
    for alight in (True, False):
        pen = canvas(active=alight)
        _far_edge(pen, 0, 0, alight=alight)
        into(pen, 'seance · far edge ' + ('lit' if alight else 'gone out'),
             'main.title')
    for pressed in (False, True):
        pen = canvas(pressed=pressed)
        for kind, kx in (('options', 6), ('minimize', 244), ('shade', 254), ('close', 264)):
            _key_mark(pen, kx, 3, kind, pressed=pressed)
        into(pen, 'seance · window keys ' + ('pressed' if pressed else 'resting'),
             'main.options', 'main.minimize', 'main.shade', 'main.close')


# --------------------------------------------------------------- the six marks

# The six transport marks, as the polylines a paw would push through salt.
# Classic silhouettes on purpose. The skin takes every liberty it likes with the
# sliders and the equalizer, where a wrong guess costs a second of confusion,
# and none at all with the six keys, where it costs the record.
MARKS = {
    'previous': [[(.30, .18), (.30, .82)],
                 [(.76, .18), (.40, .50), (.76, .82), (.76, .18)]],
    'play': [[(.34, .16), (.78, .50), (.34, .84), (.34, .16)]],
    'pause': [[(.38, .18), (.38, .82)], [(.62, .18), (.62, .82)]],
    'stop': [[(.30, .22), (.72, .22), (.72, .78), (.30, .78), (.30, .22)]],
    'next': [[(.24, .18), (.60, .50), (.24, .82), (.24, .18)],
             [(.70, .18), (.70, .82)]],
    'eject': [[(.50, .14), (.78, .52), (.22, .52), (.50, .14)],
              [(.24, .74), (.76, .74)]],
}


def sigil(pen, x, y, w, h, mark, *, pressed, seed, ring=True, scale=1.0, dot=False):
    """One control, drawn in spilled salt by a paw.

    Every key in this skin is a sigil inside a circle, which is what makes six
    of them in a row read as a set of six rather than as six unrelated marks --
    and it is also what a summoning circle is, so the joke and the legibility
    are the same decision.

    Pressed is where this skin says its one thing. Everywhere else in Catamp a
    pressed control goes darker: felt sits down in its own shadow, a chip pushes
    flat against a diffuser. Here the room is lit by small fires and a pressed
    sigil is one that has *caught* -- the salt goes to the top of its own ladder
    and a light comes up inside the ring that is not there at rest. It moves the
    whole cell rather than a bevel's four pixels, and it is the answer to the
    question the cats are asking.
    """
    pen.rect(x, y, w, h, R.KEY)
    shade = R.SALT_HOT if pressed else R.SALT
    cx, cy = x + w / 2.0, y + h / 2.0
    if pressed:
        R.pool(pen, cx, cy, w * 0.44, h * 0.44, peak=9, span=7, rings=8,
               strength=235, seed=seed, over=R.CLOTH_DEEP)
    if ring:
        R.salt_arc(pen, cx - 0.5, cy - 0.5, w * 0.44, h * 0.44, n=52,
                   shade=R.SALT_LIT if pressed else R.SALT_DIM,
                   seed=seed + 1, weight=0.62, scatter=0.7)
    size = min(w, h) * scale
    for run in MARKS[mark]:
        points = []
        for i in range(len(run) - 1):
            a = (cx + (run[i][0] - .5) * size, cy + (run[i][1] - .5) * size)
            b = (cx + (run[i + 1][0] - .5) * size, cy + (run[i + 1][1] - .5) * size)
            points += R.line_pts(a, b, 0.4)
        R.salt(pen, points, shade=shade, seed=seed + 2, weight=0.86, scatter=0.55)
    if dot:
        R.dot(pen, int(cx) - 1, int(cy) - 1)
    return pen


@stage
def transport():
    """Six sigils pushed through the salt along the near edge of the table, and
    the one place in the skin the dot turns up where you can see it: press play
    and it is in the circle."""
    keys = (('previous', 16, 88, 23, 18), ('play', 39, 88, 23, 18),
            ('pause', 62, 88, 23, 18), ('stop', 85, 88, 23, 18),
            ('next', 108, 88, 22, 18), ('eject', 136, 89, 22, 16))
    for pressed in (False, True):
        pen = canvas(pressed=pressed)
        for i, (mark, kx, ky, kw, kh) in enumerate(keys):
            sigil(pen, kx, ky, kw, kh, mark, pressed=pressed, seed=i * 17 + 3,
                  scale=0.62 if mark != 'eject' else 0.74,
                  dot=(pressed and mark == 'play'))
        into(pen, 'seance · transport sigils ' + ('caught' if pressed else 'resting'),
             'main.previous', 'main.play', 'main.pause', 'main.stop',
             'main.next', 'main.eject')


# ---------------------------------------------------------------- the board

@stage
def seek():
    """The spirit board, with the planchette sliding along it.

    A classic seek bar is a groove with a knob in it. This one is the thing the
    whole skin is about: a board with the alphabet on it, YES at one end and NO
    at the other, and a planchette being dragged across it. Where you are in the
    track is which letter is being spelled, and the twenty-nine pixels of
    planchette cover the letters the way a planchette does.

    The track has one state, so the board cannot change as the track plays --
    which is right: a board does not change. The planchette has two, and the
    difference between them is not a bevel. Resting it sits on the wood. Dragged
    it is being pushed, so the dot inside its lens comes up to full and the salt
    round its feet is scuffed.
    """
    pen = sheet('posbar.bmp')
    # The board: old wood, darker than the cloth, with a burnt border.
    pen.rect(0, 0, 248, 10, R.farther(R.CLOTH, 1))
    pen.ops.append(dict(op='rect', x=1, y=1, width=246, height=8, color=R.UMBER,
                        fill=True, ramp=R.ramp_between(R.RUST, R.GLOOM, 6),
                        ramp_axis=[0, 1, 0, 9], grain=8, grain_size=2, grain_seed=61))
    for edge in (0, 9):
        pen.ops.append(dict(op='rect', x=0, y=edge, width=248, height=1,
                            color=R.SCORCH, fill=True, opacity=120))
    # Candle A stands just above the middle of the board, so the middle of the
    # board is the part of it you can read.
    R.pool(pen, 88, 5, 74, 7, peak=8, span=5, rings=9, strength=120, seed=62)
    letters = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ'
    pen.ops.append(dict(op='text', x=60, y=3, text=letters, color=R.SALT_LIT,
                        face='small', scale=1))
    pen.ops.append(dict(op='text', x=5, y=3, text='YES', color=R.SALT,
                        face='small', scale=1))
    pen.ops.append(dict(op='text', x=234, y=3, text='NO', color=R.SALT,
                        face='small', scale=1))
    # Two grains of salt on the board, because the board is on the table.
    R.spill(pen, 200, 7, 12, seed=63, grains=14, shade=R.SALT_SHADOW)

    for x0, dragged in ((248, False), (278, True)):
        planchette(pen, x0, 0, dragged=dragged)
    commit(pen, 'seance · the board and the planchette on it')


def planchette(pen, x, y, *, dragged):
    """Twenty-nine by ten of heart-shaped wood with a lens in it, and the dot
    sitting in the lens."""
    pen.rect(x, y, 29, 10, R.KEY)
    body = R.SCORCH if dragged else R.UMBER
    pen.ops.append(dict(op='path', x=x, y=y, color=body, fill=True, brush_size=1,
                        points=[[2, 8], [0.5, 4.5, 5, 0.5], [14.5, -0.6, 24, 0.5],
                                [28.5, 4.5, 27, 8], [20, 10.4, 14.5, 9.6],
                                [9, 10.4, 2, 8]]))
    # The rim is the only part of it the candle reaches.
    pen.ops.append(dict(op='path', x=x, y=y, color=R.nearer(body, 3 if dragged else 2),
                        fill=False, brush_size=1,
                        points=[[2, 8], [0.5, 4.5, 5, 0.5], [14.5, -0.6, 24, 0.5],
                                [28.5, 4.5, 27, 8]]))
    # The lens: a ring of pale horn with the dot inside it.
    pen.ops.append(dict(op='ellipse', x=x + 10, y=y + 2, width=9, height=7,
                        color=R.SALT_LIT if dragged else R.SALT_DIM,
                        fill=False, brush_size=1))
    pen.ops.append(dict(op='ellipse', x=x + 11, y=y + 3, width=7, height=5,
                        color=R.VOID, fill=True))
    R.dot(pen, x + 13, y + 4, size=2)
    if dragged:
        # Pushed, not lifted: the feet scuff the salt rather than casting a
        # shadow, because there is no light above this to cast one.
        R.spill(pen, x + 5, y + 8, 5, seed=64, grains=9, shade=R.SALT)
        R.spill(pen, x + 24, y + 8, 5, seed=65, grains=9, shade=R.SALT)
    else:
        for foot in (4, 14, 24):
            pen.rect(x + foot, y + 8, 1, 2, R.farther(body, 1))
    return pen


# --------------------------------------------------------------- the sliders

# Where each of the fourteen cats in the volume strip is sitting, in the order
# they wake up, and how big its eyes are from where you are. Hand-placed rather
# than scattered by a seed: a crowd wants the far ones high and small and the
# near ones low and large, and a random scatter gives a starfield.
CROWD = ((2, 8, 2), (8, 4, 1), (13, 9, 2), (18, 2, 1), (23, 7, 2), (28, 4, 1),
         (33, 9, 2), (38, 2, 1), (43, 6, 2), (48, 3, 1), (52, 9, 2),
         (57, 5, 1), (61, 8, 2), (65, 3, 1))


@stage
def volume():
    """Volume is how many cats have woken up and turned round to look at you.

    A classic volume track is a groove that fills up. This one fills up with
    eyes: silence is a strip of dark with nothing in it at all, and every step
    up wakes one more cat somewhere in the room. It works because eyeshine is
    the one thing in this skin that does not fall off with distance -- a cat at
    the back of the strip has the same two discs as the one at the front, so a
    crowd can be drawn in a space thirteen pixels tall without any of it needing
    to be lit.

    The thumb is the nearest one, which is the only cat in the crowd with a head
    as well as a pair of eyes.
    """
    pen = sheet('volume.bmp')
    for index, (label, (vx, vy, vw, vh)) in enumerate(variants('main.volume.track')):
        # The ground is the table, carrying the light the table carries there:
        # this strip begins two pixels from candle A and ends sixty-eight
        # pixels away from it, so it runs from the edge of the pool into the
        # dark. Filled flat with the room's own black instead, the control came
        # out as a rectangular hole in the middle of a lit table -- correct art,
        # and the one thing in the window that was obviously a sprite.
        pen.ops.append(dict(op='rect', x=vx, y=vy, width=vw, height=vh,
                            color=R.RUST, fill=True,
                            ramp=R.ramp_between(R.RUST, R.CLOTH_DEEP, 7),
                            ramp_axis=[vx, vy, vx + vw, vy],
                            grain=7, grain_size=2, grain_seed=70 + index))
        awake = int(round(index / 27.0 * len(CROWD)))
        for (cx, cy, size) in CROWD[:awake]:
            R.eyes(pen, vx + cx, vy + cy - size, gap=size, w=size, h=size,
                   shade=R.EYE if size > 1 else R.EYE_DIM, glint=False)
    for (tx, ty, tw, th), pressed in zip(
            [v for _, v in variants('main.volume.thumb')], (False, True)):
        near_cat(pen, tx, ty, tw, th, pressed=pressed)
    commit(pen, 'seance · fourteen cats waking up')


def near_cat(pen, x, y, w, h, *, pressed):
    """The volume handle: the cat nearest you, in front of all the others."""
    pen.rect(x, y, w, h, R.KEY)
    C.silhouette(pen, C.place(C.BEHIND, x + w / 2.0, y + h, scale=h * 1.02),
                 R.VOID)
    R.eyes(pen, x + 4, y + 3, gap=3, w=3, h=3,
           shade=R.EYE_HOT if pressed else R.EYE)
    return pen


@stage
def balance():
    """Balance is the saucer of milk, and it tips.

    Centre needs no mark on it: milk sitting level is what level looks like, and
    a cat pushing the saucer to one side is what a balance control does. The two
    ends are where the joke is -- at hard left and hard right the milk has gone
    over the rim, and there is a wet patch on the cloth.
    """
    pen = sheet('balance.bmp')
    for index, (label, (bx, by, bw, bh)) in enumerate(variants('main.balance.track')):
        lean = (index - 13.5) / 13.5
        pen.rect(bx, by, bw, bh, R.KEY)
        # The saucer, seen nearly edge-on: a pale rim with a dark well in it.
        pen.ops.append(dict(op='ellipse', x=bx + 2, y=by + 3, width=34, height=10,
                            color=R.SALT_SHADOW, fill=True))
        pen.ops.append(dict(op='ellipse', x=bx + 2, y=by + 2, width=34, height=9,
                            color=R.SALT_DIM, fill=True))
        pen.ops.append(dict(op='ellipse', x=bx + 4, y=by + 3, width=30, height=7,
                            color=R.VOID, fill=True))
        # The milk. Its surface is one straight line across the well, tilted by
        # the lean; everything below the line is milk and everything above it is
        # an empty saucer, which is what a cat leaves.
        for column in range(30):
            t = (column - 14.5) / 14.5
            surface = 7.0 - lean * t * 2.7
            top = max(3.6, surface)
            if top >= 9.6:
                continue
            pen.rect(bx + 4 + column, by + int(round(top)), 1,
                     max(1, int(round(9.6 - top))), R.WAX)
            pen.rect(bx + 4 + column, by + int(round(top)), 1, 1, R.WAX_LIT)
        # The glint, and it is here because of `identical_variants`: the surface
        # is quantised to whole pixels, so the four frames either side of centre
        # came out the same picture and a third of the control's travel did
        # nothing. Every other report was clean -- correct ink, right cell,
        # right colours, four frames identical. A glint that slides one pixel a
        # frame cannot agree with itself.
        pen.rect(bx + 5 + index, by + int(round(7.0 - lean * ((index - 13.5) / 13.5) * 2.7)),
                 2, 1, R.SALT_HOT)
        # Over the rim at the ends, and a wet patch on the cloth under it.
        if abs(lean) > 0.78:
            side = bx + (33 if lean > 0 else 3)
            pen.rect(side, by + 8, 2, 3, R.WAX_DIM)
            R.spill(pen, side + 1, by + 10, 2, seed=80 + index, grains=6, shade=R.WAX_DIM)
    for (tx, ty, tw, th), pressed in zip(
            [v for _, v in variants('main.balance.thumb')], (False, True)):
        muzzle(pen, tx, ty, tw, th, pressed=pressed)
    commit(pen, 'seance · the saucer, and which way it has gone')


def muzzle(pen, x, y, w, h, *, pressed):
    """The balance handle: a cat leaning in from above, drawn only in its top
    six rows so the milk it is pushing about stays visible under it."""
    pen.rect(x, y, w, h, R.KEY)
    pen.ops.append(dict(op='path', x=x, y=y, color=R.VOID, fill=True, brush_size=1,
                        points=[[1, 0], [1, 3], [3, 6, 7, 6.6], [11, 6, 13, 3],
                                [13, 0], [1, 0]]))
    R.eyes(pen, x + 3, y + 1, gap=4, w=2, h=2,
           shade=R.EYE_HOT if pressed else R.EYE, glint=False)
    # Whiskers: three grains of light each side, which is all a whisker is at
    # this size and more than an unbroken line would say.
    for side, sx in ((-1, x + 3), (1, x + 10)):
        for i in range(3):
            pen.rect(int(sx + side * (i + 1)), y + 5 + (i > 1), 1, 1,
                     R.SALT_DIM if not pressed else R.SALT)
    return pen


@stage
def options():
    """Four of this skin's ideas are skin options rather than artwork, and three
    of them create a surface to draw on, so they go before anything is drawn:
    eleven equalizer handles of their own, a bitmap under the track list,
    artwork for the row that is playing, and a visualizer whose unlit pixels are
    transparent. Without that last one the spectrum is drawn inside an opaque
    rectangle of its own darkest colour, and a black box sits in the middle of
    the table however dark the table is."""
    changed = answer('studio_options', {
        'eq_handles': True, 'playlist_background': True,
        'playlist_selection': True, 'visualizer_glass': True})
    for note in changed.get('sheets_changed') or []:
        print(' ', note)


# ------------------------------------------------------------- the mantelpiece

BAND_X = (21, 78, 96, 114, 132, 150, 168, 186, 204, 222, 240)


def plate(pen, x, y, w, h, word, *, on, pressed, seed):
    """A word cut into the mantel and filled with salt. Four states, and the
    difference between them is light rather than bevel: off is cold salt, on is
    salt with a flame near it, and pressed is salt that has caught."""
    pen.rect(x, y, w, h, R.KEY)
    # Four states and four brightnesses. A pressed switch that is off has
    # caught a little; a pressed switch that is on has caught properly. Using
    # one pressed look for both made the two the same picture, which is what
    # `identical_variants` said about this cell the first time it was drawn.
    peak = {(False, False): None, (False, True): 6,
            (True, False): 7, (True, True): 9}[(on, pressed)]
    if peak:
        R.pool(pen, x + w / 2.0, y + h / 2.0, w * 0.5, h * 0.62,
               peak=peak, span=6, rings=7, strength=200 + 35 * int(pressed),
               seed=seed, over=R.CLOTH_DEEP)
    ink = (R.SALT_HOT if on else R.SALT) if pressed else \
          (R.SALT_LIT if on else R.SALT_DIM)
    centred(pen, x, y + (h - 7) // 2, w, word, ink)
    return pen


@stage
def eq():
    """The mantelpiece: eleven candles, the wall they have been smoking for
    years, and the cat that sits between the preamp and the rest of them.

    The eleven bands share one set of twenty-eight track cells -- every band
    draws from the same rectangle -- so nothing about a band can be said in its
    groove. What each band has of its own is a 14x25 handle, which is where the
    eleven different candles are: a taper, a birthday candle, a tea light in its
    tin, a stub in a bottle neck, a stick of incense, one in a jar, one shaped
    like a cat, a beeswax coil, a match, and one that has fallen over and is
    burning sideways. Eleven copies of one flame is a pattern. This is a shelf.
    """
    pen = canvas(active=True)
    pen.ops.append(dict(op='rect', x=0, y=116, width=275, height=116, color=R.VOID,
                        fill=True, grain=6, grain_size=2, grain_seed=90))
    # The wall behind the shelf, warmed by eleven flames standing in front of
    # it, and stained above each one by years of them.
    for i, bx in enumerate(BAND_X):
        R.pool(pen, bx + 7, 178, 15, 34, peak=5, span=4, rings=7, strength=120,
               seed=91 + i)
        R.smoke(pen, bx + 7, 168, height=22, drift=1 if i % 2 else -1, seed=91 + i)
    R.pool(pen, 28, 176, 26, 44, peak=6, span=5, rings=9, strength=130, seed=110)

    # The shelf: the one horizontal in the window, and the thing the candles
    # are standing on.
    pen.ops.append(dict(op='rect', x=0, y=214, width=275, height=18, color=R.UMBER,
                        fill=True, ramp=R.ramp_between(R.SCORCH, R.GLOOM, 7),
                        ramp_axis=[0, 214, 0, 232], grain=7, grain_size=2, grain_seed=111))
    pen.ops.append(dict(op='rect', x=0, y=214, width=275, height=1, color=R.COPPER,
                        fill=True, opacity=140))
    R.spill(pen, 60, 220, 26, seed=112, grains=40, shade=R.SALT_SHADOW)

    # The cat between the preamp and the other ten, watching the big one burn.
    # Forty-two pixels of gap is the only place in this skin a whole cat fits.
    C.rimlit(pen, C.place(C.SITTING, 60, 216, scale=42, flip=True), (28, 168),
             ground=R.GLOOM, reach=8, radius=46, thickness=3, seed=113)
    R.eyes(pen, 262, 150, gap=4, w=3, h=3)
    R.eyes(pen, 6, 140, gap=3, w=2, h=2, shade=R.EYE_DIM)
    into(pen, 'seance · the mantel and the wall behind it', 'equalizer.background')

    # The sooty panel the equalizer curve is drawn on, and the nought line
    # somebody has scratched across it.
    pen = canvas()
    pen.ops.append(dict(op='rect', x=86, y=133, width=113, height=19, color=R.GLOOM,
                        fill=True, ramp=R.ramp_between(R.UMBER, R.VOID, 6),
                        ramp_axis=[86, 133, 86, 152], grain=9, grain_size=2,
                        grain_seed=114))
    into(pen, 'seance · the soot panel', 'equalizer.graph')
    pen = canvas()
    R.salt_line(pen, (86, 142), (198, 142), shade=R.SALT_DIM, seed=115,
                weight=0.55, scatter=0)
    into(pen, 'seance · the nought line', 'equalizer.preamp.line')

    for alight in (True, False):
        pen = canvas(active=alight)
        _mantel_top(pen, 0, 116, alight=alight)
        into(pen, 'seance · mantel top ' + ('lit' if alight else 'gone out'),
             'equalizer.title')

    for on in (False, True):
        for pressed in (False, True):
            pen = canvas(active=on, pressed=pressed)
            plate(pen, 14, 134, 26, 12, 'ON', on=on, pressed=pressed, seed=116)
            plate(pen, 40, 134, 32, 12, 'AUTO', on=on, pressed=pressed, seed=117)
            into(pen, f'seance · ON/AUTO {"on" if on else "off"}'
                      f'{" pressed" if pressed else ""}',
                 'equalizer.on', 'equalizer.auto')
    for pressed in (False, True):
        pen = canvas(pressed=pressed)
        plate(pen, 217, 134, 44, 12, 'PRESETS', on=True, pressed=pressed, seed=118)
        _key_mark(pen, 264, 119, 'close', pressed=pressed)
        into(pen, 'seance · presets and close ' + ('pressed' if pressed else 'resting'),
             'equalizer.presets', 'equalizer.close')


def _mantel_top(pen, x, y, *, alight):
    """The wall above the shelf, and what it looks like when the eleven candles
    below it are not being looked at."""
    pen.ops.append(dict(op='rect', x=x, y=y, width=275, height=14,
                        color=R.DUSK if alight else R.CLOTH_DEEP, fill=True,
                        grain=7, grain_size=2, grain_seed=120))
    if alight:
        for cx in (30, 118, 206):
            R.pool(pen, x + cx, y + 15, 40, 10, peak=6, span=4, rings=7,
                   strength=130, seed=121 + cx)
    pen.ops.append(dict(op='rect', x=x, y=y + 12, width=275, height=2,
                        color=R.nearer(R.UMBER, 1 if alight else -1), fill=True,
                        opacity=180))
    R.eyes(pen, x + 246, y + 4, gap=3, w=2, h=2, shade=R.EYE_DIM)
    centred(pen, x, y + 4, 275, 'EQUALIZER', R.SALT_LIT if alight else R.SALT_DIM)
    return pen


# The eleven candles, in the order they stand on the shelf. The first is the
# preamp, which is why it is the big one everything else is arranged around.
CANDLES = ('pillar', 'taper', 'birthday', 'tealight', 'bottle', 'incense',
           'jar', 'cat', 'coil', 'match', 'fallen')


def candle_top(pen, x, y, style, *, pressed):
    """One band's handle: the top twenty-five pixels of one particular candle.

    The eleven bands share a single set of twenty-eight track cells -- every
    band draws its groove from the same rectangle -- so nothing that is true of
    only one band can be said down there. All of it has to be said in fourteen
    by twenty-five, which is why these are eleven different *objects* and not
    eleven flames at different heights.

    Pressed is a flame that has taken: it grows, and the wax under it goes up
    the ladder with it. No halo, because a halo has to blend with what is behind
    it and what is behind a handle is a wall it slides up and down.
    """
    pen.rect(x, y, 14, 25, R.KEY)
    step = 1 if pressed else 0
    wax = R.nearer(R.WAX, step) if not pressed else R.WAX_LIT
    dim = R.WAX_DIM if not pressed else R.WAX
    height = 6 + 3 * step

    def body(x0, w, top, *, color=None, runs=0, seed=0):
        pen.ops.append(dict(op='rect', x=x + x0, y=y + top, width=w,
                            height=25 - top, color=color or wax, fill=True,
                            ramp=R.ramp_between(R.WAX_LIT if pressed else wax, dim, 5),
                            ramp_axis=[x, y + top, x, y + 25],
                            grain=4, grain_size=1, grain_seed=seed))
        for i in range(runs):
            pen.rect(x + x0 + 1 + i * 2, y + top + 1, 1, 3 + i * 2, R.WAX_LIT)

    if style == 'pillar':
        body(2, 10, 8, runs=2, seed=1)
        pen.rect(x + 2, y + 8, 10, 1, R.WAX_LIT)
        R.flame(pen, x + 7, y + 8, height=height + 1, width=4, lean=-0.5)
    elif style == 'taper':
        body(5, 5, 7, seed=2)
        R.flame(pen, x + 7, y + 7, height=height, width=3, lean=0.4)
    elif style == 'birthday':
        body(5, 4, 6, color=R.SALT_LIT, seed=3)
        for row in range(6, 25, 3):
            pen.rect(x + 5, y + row, 4, 1, '#c8544a' if not pressed else '#e8776a')
        R.flame(pen, x + 7, y + 6, height=max(4, height - 2), width=2, lean=-0.3)
    elif style == 'tealight':
        pen.ops.append(dict(op='rect', x=x + 1, y=y + 17, width=12, height=8,
                            color=R.SALT_DIM, fill=True,
                            ramp=R.ramp_between(R.SALT, R.SALT_SHADOW, 5),
                            ramp_axis=[x, y + 17, x, y + 25]))
        for crimp in range(1, 13, 2):
            pen.rect(x + crimp, y + 19, 1, 6, R.SALT_SHADOW)
        pen.rect(x + 2, y + 17, 10, 2, wax)
        R.flame(pen, x + 7, y + 17, height=max(4, height - 1), width=3, lean=0.6)
    elif style == 'bottle':
        # Glass, drawn rather than baked. `material: "glass"` refracts what is
        # under the shape, and what is under a sprite cell is the transparency
        # key, so the engine's own glass comes out of a cleared cell as hot
        # pink. Two of these eleven handles were magenta bottles before I looked
        # at the sheet, and every report said the transaction was fine.
        pen.ops.append(dict(op='path', x=x, y=y, color=R.VOID, fill=True,
                            brush_size=1,
                            points=[[5, 10], [5, 15], [2, 17, 2, 21], [2, 25],
                                    [11, 25], [11, 21], [11, 17, 8, 15],
                                    [8, 10], [5, 10]]))
        pen.ops.append(dict(op='path', x=x, y=y, color=R.SALT_DIM, fill=False,
                            brush_size=1,
                            points=[[5, 10], [5, 15], [2, 17, 2, 21], [2, 25]]))
        pen.rect(x + 3, y + 19, 1, 6, R.SALT if pressed else R.SALT_DIM)
        pen.rect(x + 5, y + 5, 4, 6, wax)
        for run, length in ((5, 7), (8, 10), (3, 4)):
            pen.rect(x + run, y + 11, 1, length, R.WAX_LIT if pressed else R.WAX)
        R.flame(pen, x + 7, y + 5, height=max(4, height - 1), width=3, lean=-0.7)
    elif style == 'incense':
        pen.rect(x + 7, y + 9, 1, 16, R.UMBER)
        pen.rect(x + 7, y + 9, 1, 1, R.DOT if pressed else '#d2452c')
        R.smoke(pen, x + 7, y + 8, height=8, drift=2, seed=4)
        pen.rect(x + 5, y + 21, 5, 1, R.SALT_DIM)
    elif style == 'jar':
        pen.rect(x + 2, y + 9, 11, 16, R.VOID)
        pen.rect(x + 5, y + 16, 5, 9, dim)
        R.flame(pen, x + 7, y + 16, height=max(4, height - 2), width=3, lean=0.3)
        for side in (2, 12):
            pen.rect(x + side, y + 9, 1, 16, R.SALT_LIT if pressed else R.SALT)
        pen.rect(x + 1, y + 9, 13, 2, R.SALT_LIT if pressed else R.SALT)
        pen.rect(x + 3, y + 13, 1, 10, R.SALT_HOT if pressed else R.SALT_LIT)
        pen.rect(x + 4, y + 24, 7, 1, R.SALT_DIM)
    elif style == 'cat':
        # A novelty candle shaped like a cat, which is what a cat looks like
        # after a few hours of being one: the ears have gone over and the wick
        # is coming out of the top of its head.
        # Twelve pixels wide, not twenty-two. The first one was built at the
        # scale the other handles use and a cat that wide in a fourteen-pixel
        # cell is not a cat, it is a cream rectangle with the ears cut off by
        # the cell wall.
        C.silhouette(pen, C.place(C.LOAF, x + 7, y + 25, scale=12, flip=True), wax)
        pen.rect(x + 2, y + 24, 11, 1, R.WAX_LIT)
        # A wick out of the top of its head, and one ear gone over in the heat.
        pen.rect(x + 5, y + 13, 1, 4, R.WAX_LIT)
        pen.rect(x + 10, y + 17, 3, 1, wax)
        R.flame(pen, x + 5, y + 13, height=max(4, height - 2), width=3, lean=0.9)
    elif style == 'coil':
        for turn in range(4):
            pen.ops.append(dict(op='ellipse', x=x + 2 + turn % 2, y=y + 10 + turn * 4,
                                width=10 - turn % 2 * 2, height=5,
                                color=wax if turn % 2 else dim, fill=True))
        R.flame(pen, x + 7, y + 11, height=max(4, height - 2), width=3, lean=-0.4)
    elif style == 'match':
        pen.ops.append(dict(op='path', x=x, y=y, color=R.UMBER, fill=False,
                            brush_size=1, points=[[9, 24], [7, 14, 6, 7]]))
        pen.rect(x + 5, y + 6, 2, 2, R.VOID)
        pen.rect(x + 4, y + 22, 6, 3, R.WAX_LIT if pressed else R.WAX)
        R.flame(pen, x + 6, y + 7, height=max(4, height - 1), width=3, lean=0.8)
    else:  # fallen: the one that is going to be a problem
        pen.ops.append(dict(op='rect', x=x + 3, y=y + 17, width=11, height=5,
                            color=wax, fill=True,
                            ramp=R.ramp_between(R.WAX_LIT if pressed else wax, dim, 4),
                            ramp_axis=[x, y + 17, x, y + 22]))
        pen.rect(x + 3, y + 22, 9, 1, R.WAX_LIT)
        R.flame(pen, x + 3, y + 17, height=height + 2, width=3, lean=-1.6)
        R.spill(pen, x + 8, y + 23, 4, seed=5, grains=8, shade=R.SALT_SHADOW)
    return pen


@stage
def eq_bands():
    """The grooves and the eleven candle tops.

    A track frame says how much candle is left: full boost is a new one and full
    cut is a stub in a lake of its own wax, and the handle is where the top of
    it is, so the two meet exactly wherever the band is set.
    """
    pen = sheet('eqmain.bmp')
    for index, (label, (tx, ty, tw, th)) in enumerate(variants('equalizer.band0.track')):
        pen.rect(tx, ty, tw, th, R.KEY)
        top = int(round((27 - index) / 27.0 * 38)) + 24
        # Not a candle: the years of wax the candle has run down onto the shelf,
        # which is the one thing that can be true of all eleven of them. Eleven
        # bands share one set of twenty-eight track cells, so a groove that
        # tried to be the body of a particular candle would be the body of the
        # wrong candle ten times out of eleven -- and a tea light in a tin does
        # not have a body at all. A heap of wax has whatever is standing on it.
        for row in range(max(0, top), 61):
            down = (row - top) / max(1.0, 60.0 - top)
            half = 2.6 + 3.4 * down ** 1.6
            pen.rect(tx + 7 - int(round(half)), ty + row,
                     max(1, int(round(half * 2))), 1,
                     R.WAX if row % 5 else R.WAX_LIT)
        pen.ops.append(dict(op='ellipse', x=tx + 1, y=ty + 57, width=12, height=5,
                            color=R.WAX_DIM, fill=True))
        pen.ops.append(dict(op='ellipse', x=tx + 2, y=ty + 57, width=10, height=3,
                            color=R.WAX, fill=True))
        # Spattered wax, and there is more of it the further the candle has
        # burned. It is here because `identical_variants` said the three lowest
        # frames were one picture: below a certain point there is no candle left
        # for the groove to draw and the handle covers the rest, so three
        # settings of the control looked the same. A puddle seeded on the frame
        # number cannot do that, and it is what the foot of a candle that has
        # been burning for a year looks like anyway.
        R.spill(pen, tx + 7, ty + 58, 6, seed=150 + index,
                grains=3 + (27 - index) // 2, shade=R.WAX_DIM)
    commit(pen, 'seance · eleven grooves, one candle at a time')

    pen = sheet('eqhandles.bmp')
    for index, style in enumerate(CANDLES):
        candle_top(pen, index * 14, 0, style, pressed=False)
        candle_top(pen, index * 14, 25, style, pressed=True)
    commit(pen, 'seance · eleven different candles')


# --------------------------------------------------------------- the readouts

# Seven by eleven, because nine by thirteen with a pixel of air round it is
# what the timer cell gives you. Written rather than segmented: the room is
# lit by fire and nothing in it is an LED.
DIGITS = (
    ('.#####.', '##...##', '##...##', '##...##', '##...##', '##...##',
     '##...##', '##...##', '##...##', '##...##', '.#####.'),
    ('..###..', '.####..', '...##..', '...##..', '...##..', '...##..',
     '...##..', '...##..', '...##..', '...##..', '.######'),
    ('.#####.', '##...##', '.....##', '.....##', '....##.', '...##..',
     '..##...', '.##....', '##.....', '##.....', '#######'),
    ('.#####.', '##...##', '.....##', '.....##', '..####.', '.....##',
     '.....##', '.....##', '.....##', '##...##', '.#####.'),
    ('....##.', '...###.', '..####.', '.##.##.', '##..##.', '##..##.',
     '#######', '....##.', '....##.', '....##.', '....##.'),
    ('#######', '##.....', '##.....', '##.....', '######.', '.....##',
     '.....##', '.....##', '.....##', '##...##', '.#####.'),
    ('..####.', '.##....', '##.....', '##.....', '######.', '##...##',
     '##...##', '##...##', '##...##', '##...##', '.#####.'),
    ('#######', '.....##', '....##.', '....##.', '...##..', '...##..',
     '..##...', '..##...', '.##....', '.##....', '.##....'),
    ('.#####.', '##...##', '##...##', '##...##', '.#####.', '##...##',
     '##...##', '##...##', '##...##', '##...##', '.#####.'),
    ('.#####.', '##...##', '##...##', '##...##', '##...##', '.######',
     '.....##', '.....##', '....##.', '..###..', '.###...'),
)


def written(pen, x, y, rows, ink, *, glow):
    """A numeral, and the light it is giving off.

    Everything the player writes in this skin is a light rather than a mark, so
    a digit gets a halo: the shape is stamped once dilated in a dim warm
    colour, then again on top at full. Without it the timer is four pale marks
    stuck on the cloth; with it they are four small things burning.
    """
    wide = len(rows[0])
    halo = [[' '] * (wide + 2) for _ in range(len(rows) + 2)]
    for ry, row in enumerate(rows):
        for rx, ch in enumerate(row):
            if ch != '#':
                continue
            for dy in (0, 1, 2):
                for dx in (0, 1, 2):
                    halo[ry + dy][rx + dx] = 'o'
    pen.stamp(x - 1, y - 1, [''.join(r) for r in halo], {'o': glow})
    pen.stamp(x, y, list(rows), {'#': ink})
    return pen


@stage
def readouts():
    """The four timer digits, the ink every live readout is written in, and the
    eye that says whether anything is playing."""
    pen = sheet('numbers.bmp')
    pen.rect(0, 0, 99, 13, R.KEY)
    for index, (label, (dx, dy, dw, dh)) in enumerate(variants('main.digit0')):
        written(pen, dx + 1, dy + 1, DIGITS[index], R.DISPLAY, glow=R.SCORCH)
    commit(pen, 'seance · the timer, written in light')

    # text.bmp is not a glyph sheet here: Cranamp sets every readout in its own
    # 5x7 face and opens this sheet only to sample the display ink -- the second
    # most common colour, when two thirds of it is opaque. So it is painted as
    # the glyph sheet it looks like, in the one colour the room speaks in.
    pen = sheet('text.bmp')
    pen.rect(0, 0, 155, 18, R.VOID)
    for i, line in enumerate(('ABCDEFGHIJKLMNOPQRSTUVWXYZ0123',
                              '456789.-:()+=_!?&#%*/<>ABCDEFG',
                              'abcdefghijklmnopqrstuvwxyz0123')):
        pen.ops.append(dict(op='text', x=1, y=i * 6, text=line, color=R.DISPLAY,
                            face='small', scale=1))
    commit(pen, 'seance · the display ink')

    # The status lamp is an eye: open while it plays, half shut while it is
    # paused, and asleep when it is stopped. Nine by nine has room for one eye
    # and no room for a triangle, a pair of bars and a square that are all
    # different from each other.
    pen = sheet('playpaus.bmp')
    pen.rect(0, 0, 42, 9, R.KEY)
    for (label, (sx, sy, sw, sh)) in variants('main.status'):
        shut = {'playing': 0.0, 'paused': 0.45, 'stopped': 1.0}[label]
        open_rows = max(2, int(round(7 * (1 - shut))))
        top = sy + 1 + (7 - open_rows) // 2
        if shut >= 1.0:
            # Asleep. A flat bar reads as a dash rather than as a shut eye, and
            # the curve is the whole difference between the two.
            pen.ops.append(dict(op='path', x=sx, y=top, color=R.CLOTH_NAP,
                                fill=False, brush_size=1,
                                points=[[1, 1], [4, 4, 7, 1]]))
            continue
        pen.ops.append(dict(op='ellipse', x=sx + 1, y=top, width=7,
                            height=open_rows, color=R.EYE, fill=True))
        if open_rows >= 3:
            pen.ops.append(dict(op='ellipse', x=sx + 3, y=top + 1, width=3,
                                height=open_rows - 2, color=R.PUPIL, fill=True))
            pen.rect(sx + 6, top + 1, 1, 1, R.EYE_HOT)
        pen.rect(sx + 1, top - 1, 7, 1, R.CLOTH_NAP)
    commit(pen, 'seance · the eye that says what it is doing')

    # MONO and STEREO are ears. One ear up, or two. Off is the ear folded flat,
    # which is what a cat does with an ear it is not using.
    for up in (False, True):
        pen = canvas(active=up)
        for (mx, my, mw, mh, ears, word) in ((212, 41, 27, 12, 1, 'MONO'),
                                             (239, 41, 29, 12, 2, 'STEREO')):
            pen.rect(mx, my, mw, mh, R.KEY)
            for ear in range(ears):
                _ear(pen, mx + 5 + ear * 10, my, up=up)
            centred(pen, mx, my + 6, mw, word,
                    R.SALT_LIT if up else R.SALT_SHADOW, face='small')
        into(pen, 'seance · ears ' + ('up' if up else 'down'),
             'main.mono', 'main.stereo')


def _ear(pen, x, y, *, up):
    """An ear, from behind, which is the only view of one that says whether it
    is up."""
    if up:
        pen.ops.append(dict(op='path', x=x, y=y, color=R.VOID, fill=True,
                            brush_size=1,
                            points=[[0, 6], [1.5, 0.5, 5, 0], [7, 4, 7, 6], [0, 6]]))
        pen.ops.append(dict(op='path', x=x, y=y, color=R.SCORCH, fill=False,
                            brush_size=1, points=[[0, 6], [1.5, 0.5, 5, 0]]))
        pen.ops.append(dict(op='path', x=x + 1, y=y + 1, color=R.EAR_GLOW,
                            fill=True, brush_size=1,
                            points=[[1, 4], [2, 1.5, 4, 1], [4, 4], [1, 4]]))
    else:
        pen.ops.append(dict(op='path', x=x, y=y, color=R.VOID, fill=True,
                            brush_size=1,
                            points=[[0, 6], [2, 4, 7, 4.5], [7, 6], [0, 6]]))
    return pen


# ---------------------------------------------------------------- the switches

@stage
def switches():
    """Shuffle is three cups with something under one of them. Repeat is a cat
    with its own tail in its mouth. The equalizer and playlist keys are the two
    words in this skin that have to be words."""
    # Drawn on the canvas rather than in the sheet, and `active` and `pressed`
    # choose the variant. These four controls all carry a light when they are
    # on, and a light is an ellipse that does not fit its own cell: on the sheet
    # nothing clips it and it lands in the cell next door, which is a dark blob
    # with no owner and no report. Named as layers on the canvas the engine
    # clips it and says how much it took off.
    for on in (False, True):
        for pressed in (False, True):
            pen = canvas(active=on, pressed=pressed)
            ouroboros(pen, 210, 89, 28, 15, on=on, pressed=pressed)
            cups(pen, 164, 89, 47, 15, on=on, pressed=pressed)
            plate(pen, 219, 58, 23, 12, 'EQ', on=on, pressed=pressed, seed=160)
            plate(pen, 242, 58, 23, 12, 'PL', on=on, pressed=pressed, seed=161)
            into(pen, f'seance · switches {"on" if on else "off"}'
                      f'{" pressed" if pressed else ""}',
                 'main.repeat', 'main.shuffle', 'main.eq.toggle',
                 'main.playlist.toggle')


def ouroboros(pen, x, y, w, h, *, on, pressed):
    """Repeat: a cat curled round until it has its own tail in its mouth. On is
    the ring closed; off is the same cat having let go, with a gap you can see
    from across the room -- which is the only thing a 28x15 cell has room to say
    about a difference between two states."""
    pen.rect(x, y, w, h, R.KEY)
    cx, cy = x + w / 2.0, y + h / 2.0
    if pressed:
        R.pool(pen, cx, cy, w * 0.44, h * 0.48, peak=8, span=7, rings=7,
               strength=225, seed=161, over=R.CLOTH_DEEP)
    rim = R.SALT_HOT if pressed else (R.SALT_LIT if on else R.SALT_DIM)
    fur = R.VOID
    sweep = 1.0 if on else 0.78
    ring = R.arc_pts(cx - 0.5, cy - 0.5, 8.0, 4.4, -0.5, -0.5 + math.tau * sweep, 72)
    for radius, color in ((1.9, fur), (0.7, rim)):
        for (px, py) in ring:
            nx = (px - cx) / 8.5
            ny = (py - cy) / 5.0
            pen.rect(int(round(px + nx * radius * 0.4)),
                     int(round(py + ny * radius * 0.4)), 1, 1, color)
    # Two ears on the top of the ring, and an eye, so it is a cat and not a hoop.
    for ear in (-4, 1):
        pen.ops.append(dict(op='path', x=int(cx) + ear, y=int(cy) - 6, color=fur,
                            fill=True, brush_size=1,
                            points=[[0, 3], [1.5, -1.5, 3, 3], [0, 3]]))
    R.eyes(pen, int(cx) - 3, int(cy) - 4, gap=2, w=1, h=1,
           shade=R.EYE if on else R.EYE_DIM)
    return pen


def cups(pen, x, y, w, h, *, on, pressed):
    """Shuffle: three cups, and the dot is under one of them. Off they are
    standing still in a row; on they are mid-swap, and the two that are moving
    have left a smear of salt behind them."""
    pen.rect(x, y, w, h, R.KEY)
    if pressed:
        R.pool(pen, x + w / 2.0, y + h / 2.0, w * 0.46, h * 0.5, peak=8, span=7,
               rings=8, strength=225, seed=162, over=R.CLOTH_DEEP)
    lip = R.SALT_HOT if pressed else (R.SALT_LIT if on else R.SALT_DIM)
    for i, (cx, lift) in enumerate(((8, 0), (23, 3 if on else 0), (38, 0))):
        base = y + h - 2 - lift
        pen.ops.append(dict(op='path', x=x + cx, y=base - 9, color=R.VOID,
                            fill=True, brush_size=1,
                            points=[[-3, 9], [-2, 0], [2, 0], [3, 9], [-3, 9]]))
        pen.ops.append(dict(op='path', x=x + cx, y=base - 9, color=lip,
                            fill=False, brush_size=1,
                            points=[[-2, 0], [2, 0]]))
        pen.rect(x + cx - 3, base, 7, 1, lip)
        if on and i != 1:
            R.salt(pen, R.arc_pts(x + cx, base - 4, 7, 3, 3.6, 5.6, 14),
                   shade=R.SALT_DIM, seed=163 + i, weight=0.5, scatter=0)
    if on:
        # Whatever they are shuffling is under the middle one, and it is up.
        R.dot(pen, x + 22, y + h - 4, size=2)
    return pen


# --------------------------------------------------------------- the transcript

@stage
def playlist():
    """The transcript: a spiral notebook open on the table, with whatever the
    board has spelled out so far written down the page.

    The header tile is twenty-five pixels wide and the player draws it nine
    times, so the pattern has to have a period that divides twenty-five or it
    reads as a row of seams. One wire loop of the spiral per tile is the answer
    the cell shape was asking for.
    """
    pen = sheet('pledit.bmp')

    # The header, twice. Cranamp always draws the lower row -- there is no
    # unfocused playlist -- so the upper one is seen by nobody; it is drawn
    # snuffed anyway, because a copy of the lit row would be two variants that
    # are the same picture and because this skin already knows what a window
    # that is not being looked at looks like.
    for row, alight in ((0, False), (21, True)):
        _notebook_top(pen, 0, row, 25, alight=alight)      # top.left
        _notebook_top(pen, 127, row, 25, alight=alight, wire=True)   # the tile
        _notebook_top(pen, 153, row, 25, alight=alight)    # top.right
        _notebook_top(pen, 26, row, 100, alight=alight, title=True)

    # The two rails. The left one is the page's margin and the table beside it;
    # the right one carries the groove the scroll candle slides down.
    for i in range(3):
        pen.rect(0, 42, 12, 29, R.CLOTH)
        pen.ops.append(dict(op='rect', x=6, y=42, width=6, height=29,
                            color=R.PAPER_EDGE, fill=True, grain=5,
                            grain_size=2, grain_seed=170))
        pen.rect(5, 42, 1, 29, R.CLOTH_DEEP)
        for wire in range(1, 29, 14):
            pen.rect(7, 42 + wire, 4, 2, R.SALT_DIM)
        break
    pen.rect(31, 42, 20, 29, R.CLOTH)
    pen.ops.append(dict(op='rect', x=31, y=42, width=5, height=29,
                        color=R.PAPER_EDGE, fill=True, grain=5, grain_size=2,
                        grain_seed=171))
    pen.rect(36, 42, 1, 29, R.CLOTH_DEEP)
    # The groove: a channel worn into the table by the candle being pushed
    # up and down it.
    pen.ops.append(dict(op='rect', x=40, y=42, width=8, height=29,
                        color=R.VOID, fill=True))
    pen.rect(40, 42, 1, 29, R.GLOOM)
    pen.rect(47, 42, 1, 29, R.GLOOM)

    for x0, dragged in ((52, False), (61, True)):
        _scroll_candle(pen, x0, 53, dragged=dragged)

    _footer(pen, 0, 72)
    commit(pen, 'seance · the notebook, the rails and the footer')

    # The page itself: ruled, foxed, and with the wax that has dripped on it.
    pen = sheet('plbg.bmp')
    pen.ops.append(dict(op='rect', x=0, y=0, width=243, height=203,
                        color=R.PAPER, fill=True, grain=7, grain_size=2,
                        grain_seed=172))
    for row in range(0, 203, 11):
        pen.ops.append(dict(op='rect', x=0, y=row + 10, width=243, height=1,
                            color=R.PAPER_RULE, fill=True, opacity=120))
    pen.ops.append(dict(op='rect', x=17, y=0, width=1, height=203,
                        color=R.PAPER_MARGIN, fill=True, opacity=150))
    # Foxing and a wax drip, both seeded so the page is the same page every
    # time the recipe runs.
    R.spill(pen, 190, 60, 26, seed=173, grains=40, shade=R.PAPER_RULE)
    R.spill(pen, 60, 150, 20, seed=174, grains=26, shade=R.PAPER_RULE)
    for drip, (dx, dy) in enumerate(((212, 8), (36, 120))):
        pen.ops.append(dict(op='ellipse', x=dx, y=dy, width=9, height=5,
                            color=R.WAX_DIM, fill=True, opacity=200))
        pen.ops.append(dict(op='ellipse', x=dx + 1, y=dy, width=6, height=3,
                            color=R.WAX, fill=True, opacity=180))
    commit(pen, 'seance · the page')

    # The row being written on now: the candle is over it, so it is the one
    # line of the page with any light on it.
    pen = sheet('plselection.bmp')
    pen.ops.append(dict(op='rect', x=0, y=0, width=243, height=11,
                        color=R.PAPER_LIT, fill=True, grain=6, grain_size=2,
                        grain_seed=175))
    R.pool(pen, 120, 5, 118, 7, peak=6, span=4, rings=8, strength=80, seed=176)
    pen.ops.append(dict(op='rect', x=0, y=10, width=243, height=1,
                        color=R.PAPER_RULE, fill=True, opacity=120))
    commit(pen, 'seance · the line being written')


def _notebook_top(pen, x, y, w, *, alight, wire=False, title=False):
    """Twenty pixels of the top of the window: table, then the wire, then the
    first of the paper."""
    pen.ops.append(dict(op='rect', x=x, y=y, width=w, height=20,
                        color=R.CLOTH if alight else R.CLOTH_DEEP, fill=True,
                        grain=6, grain_size=2, grain_seed=177))
    if alight:
        # 0.44 rather than 0.7, and `crossed_cells` is why: the header's tile is
        # twenty-five pixels wide and the player draws it nine times, so a pool
        # of light that runs four pixels past the end of the cell it is lighting
        # is repeated nine times across the top of the window. Nothing else
        # reports it -- every one of those pixels is a legal part of some cell.
        R.pool(pen, x + w / 2.0, y + 2, w * 0.44, 9, peak=6, span=5, rings=7,
               strength=120, seed=178, over=R.CLOTH)
    pen.ops.append(dict(op='rect', x=x, y=y + 14, width=w, height=6,
                        color=R.PAPER if alight else R.PAPER_SHADE, fill=True,
                        grain=5, grain_size=2, grain_seed=179))
    pen.ops.append(dict(op='rect', x=x, y=y + 13, width=w, height=1,
                        color=R.PAPER_EDGE if alight else R.CLOTH_DEEP, fill=True))
    if wire:
        # One loop of the spiral per tile, so nine tiles are nine loops rather
        # than nine seams.
        pen.ops.append(dict(op='ellipse', x=x + 8, y=y + 6, width=9, height=11,
                            color=R.SALT_DIM if alight else R.SALT_SHADOW,
                            fill=False, brush_size=2))
        pen.ops.append(dict(op='rect', x=x + 9, y=y + 6, width=6, height=2,
                            color=R.SALT if alight else R.SALT_SHADOW, fill=True))
    if title:
        # A strip of tape stuck across the cover, with what this is on it.
        pen.ops.append(dict(op='rect', x=x + 8, y=y + 3, width=84, height=10,
                            color=R.TAPE if alight else R.TAPE_DIM, fill=True,
                            grain=6, grain_size=2, grain_seed=180))
        pen.ops.append(dict(op='rect', x=x + 8, y=y + 3, width=84, height=1,
                            color=R.SALT_DIM if alight else R.SALT_SHADOW,
                            fill=True, opacity=120))
        centred(pen, x + 8, y + 5, 84, 'TRANSCRIPT',
                R.INK if alight else R.TAPE_DIM)
    return pen


def _scroll_candle(pen, x, y, *, dragged):
    """The scrollbar handle: a candle stub being pushed down the groove, which
    is what a scrollbar is if the groove is a groove worn into a table."""
    # The groove behind it rather than the transparency key: the handle is
    # always drawn over its own groove, and a cell of key here comes back from
    # the canvas as a magenta square.
    pen.rect(x, y, 8, 18, R.VOID)
    pen.ops.append(dict(op='rect', x=x + 2, y=y + 5, width=4, height=13,
                        color=R.WAX, fill=True,
                        ramp=R.ramp_between(R.WAX_LIT, R.WAX_DIM, 5),
                        ramp_axis=[x, y + 5, x, y + 18]))
    R.lit(pen, x + 4, y + 5, height=6 if dragged else 4, width=3,
          lean=1.4 if dragged else -0.2, reach=5, seed=181)
    return pen


def _footer(pen, x, y):
    """The bottom of the page, the fold, and the eleven controls down there that
    have no sprite at all.

    `studio_rectangles {"hit": true}` is the only thing that says where those
    are: five menu words and six transport keys that Cranamp hit-tests and draws
    nothing for. Sheet column 125 is never drawn either, so the page has to end
    and the table has to start exactly there.
    """
    pen.ops.append(dict(op='rect', x=x, y=y, width=125, height=38,
                        color=R.PAPER_SHADE, fill=True, grain=6, grain_size=2,
                        grain_seed=182))
    pen.ops.append(dict(op='rect', x=x + 126, y=y, width=150, height=38,
                        color=R.CLOTH, fill=True, grain=6, grain_size=2,
                        grain_seed=183))
    # The page's torn bottom edge, and the shadow it throws on the table.
    for column in range(125):
        depth = 3 + int(2.5 * math.sin(column * 0.7) + 2.5 * math.sin(column * 0.23))
        pen.rect(x + column, y + 30 + depth, 1, 38 - 30 - depth, R.CLOTH)
        pen.rect(x + column, y + 30 + depth - 1, 1, 1, R.PAPER_EDGE)
    pen.rect(x + 124, y, 1, 32, R.PAPER_EDGE)

    # The five menu words, written on the page in pencil rather than in salt,
    # because this is the one surface in the skin somebody has a pencil for.
    # MISC's hit area is canvas 99..134 and this flap ends at sheet column 124,
    # with column 125 never drawn at all -- so a caption centred in its own hit
    # area comes out as MIS on the page and a C on the table a pixel out of
    # step. It is set in the room it has rather than in the room it is owed.
    for word, wx, ww in (('ADD', 10, 28), ('REM', 39, 28), ('SEL', 69, 28),
                         ('MISC', 99, 25)):
        centred(pen, x + wx, y + 4, ww, word, R.SALT_LIT, face='small')
        R.salt_line(pen, (x + wx + 3, y + 11), (x + wx + ww - 4, y + 11),
                    shade=R.PAPER_RULE, seed=184, weight=0.7, scatter=0)
    centred(pen, x + 229, y + 4, 28, 'LIST', R.SALT_LIT, face='small')

    # Six transport sigils, eight pixels each, the same marks as the big ones.
    for i, (mark, mx, mw) in enumerate((('previous', 139, 8), ('play', 148, 8),
                                        ('pause', 157, 8), ('stop', 166, 8),
                                        ('next', 175, 8), ('eject', 185, 12))):
        sigil(pen, x + mx, y + 25, mw, 8, mark, pressed=False, seed=190 + i,
              ring=False, scale=0.9)
    return pen


# ---------------------------------------------------------------- the palettes

@stage
def palettes():
    """The two text files: what the track list is written in, and what colour
    the spirit's voice is."""
    call('studio_options', {'playlist_colors': {
        'Normal': R.LIST_INK, 'Current': R.LIST_NOW, 'NormalBG': R.PAPER,
        'SelectedBG': R.PAPER_LIT, 'MbFG': R.LIST_NOW, 'MbBG': R.PAPER_SHADE}})
    # The spectrum stands in the candle's own pool and rises out of it, so it is
    # coloured off the same ladder as everything else: the bottom of a bar is
    # the table one step up and the top of one is the inside of a flame.
    spectrum = [R.GLOW[3 + round(8 * i / 15)] for i in range(16)]
    colors = ([R.VOID, R.CLOTH_DEEP] + spectrum
              + [R.CREAM, R.FLAME, R.HONEY, R.AMBER, R.SALT_HOT, R.DOT])
    call('studio_options', {'visualizer_colors': colors[:24]})
    print('  palettes: warm on dark paper, and a spectrum that burns')


@stage
def measured():
    """Every caption in the recipe, handed to the engine and checked against the
    room it was laid out in.

    Three recipes before this one carried their own copy of the glyph walk and
    all three had the same bug in it, so this one never does the arithmetic: it
    asks. What is checked here is the other half -- that the answer fits the
    cell it was aimed at, which is the thing a repeated tile turns into nine
    copies of a mistake.
    """
    fits = (('CATAMP SEANCE', '5x7', 275), ('EQUALIZER', '5x7', 275),
            ('ON', '5x7', 26), ('AUTO', '5x7', 32), ('PRESETS', '5x7', 44),
            ('EQ', '5x7', 23), ('PL', '5x7', 23), ('TRANSCRIPT', '5x7', 84),
            ('MONO', 'small', 27), ('STEREO', 'small', 29),
            ('ADD', 'small', 28), ('REM', 'small', 28), ('SEL', 'small', 28),
            ('MISC', 'small', 25), ('LIST', 'small', 28),
            ('ABCDEFGHIJKLMNOPQRSTUVWXYZ', 'small', 248),
            ('YES', 'small', 248), ('NO', 'small', 248))
    for word, face, room in fits:
        width, _ = measure(word, face=face)
        if width > room:
            raise ValueError(f'{word!r} in the {face} face is {width} pixels '
                             f'and the cell it is set in has {room}')
    print(f'  {len(fits)} captions measured against the engine, all fitting')


# What the skin claims about itself, and where. Every cat in this skin is lit
# for a ground the recipe *told* it it was standing on, and every hand-drawn
# mark is a colour chosen against artwork drawn somewhere else in the file. Both
# were eyeballed off renders for the whole of the first build, and both are
# questions the document can answer.
GROUNDS = (
    # what,                       rect,                    claimed
    ('the tail up the left edge', (0, 40, 22, 60), R.GLOOM),
    ('the cat behind candle B',   (172, 40, 25, 16), R.UMBER),
    ('the cat on the corner',     (250, 85, 24, 26), R.DUSK),
    ('the cat on the mantel',     (38, 160, 40, 56), R.GLOOM),
)

READS = (
    # what,                        rect,                     ink,        floor
    ('a resting transport sigil',  (39, 88, 23, 18), R.SALT, 3.0),
    ('the board alphabet',         (60, 73, 180, 8), R.SALT_LIT, 4.5),
    ('YES on the board',           (22, 73, 16, 8), R.SALT, 3.0),
    ('the EQ and PL keys',         (219, 58, 46, 12), R.SALT_LIT, 4.5),
    ('the footer menu words',      (10, 346, 125, 18), R.SALT_LIT, 4.5),
    ('LIST on the table',          (228, 346, 28, 18), R.SALT_LIT, 4.5),
    ('the wordmark on the table',  (99, 4, 78, 7), R.SALT_LIT, 4.5),
    ('EQUALIZER on the wall',      (99, 120, 78, 7), R.SALT_LIT, 4.5),
    ('TRANSCRIPT on its tape',     (95, 235, 84, 10), R.INK, 4.5),
)


@stage
def checked():
    """Ask the document the two questions the first build guessed at.

    `rimlit()` lights a cat for the ground it is told it is in front of, and a
    cat lit for the wrong room is the one mistake in this skin that no report
    catches: the ink is correct, in the right cell, in colours that are on the
    ladder, and the animal is lit by a candle that is not there. And the export
    check covers the eight readouts Cranamp writes, which in a skin made of
    hand-drawn marks on hand-drawn artwork is most of nothing.

    `studio_pixel` answers both -- the artwork under any rectangle, and the same
    WCAG arithmetic pointed at it -- so what was eyeballed off renders for a
    whole build is now a stage that fails.
    """
    # Say which state the probe means. `ground` is composited from the variants
    # the view is showing, so asking about a resting mark while the view is left
    # on `pressed` measures the released ink against the pressed cell's glow --
    # a pairing that exists nowhere, reads as a finding, and sent me looking for
    # a legibility problem this skin does not have.
    answer('studio_canvas', {'active': True, 'pressed': False})
    for what, rect, claimed in GROUNDS:
        here = answer('studio_pixel', dict(zip(('x', 'y', 'width', 'height'), rect)))
        got = here.get('ground')
        if got is None:
            raise ValueError(f'{what}: nothing is drawn at {rect}')
        drift = abs(R.step(got) - R.step(claimed))
        if drift > 1:
            raise ValueError(
                f'{what} is lit for {claimed} (ladder {R.step(claimed)}) and is '
                f'standing in front of {got} (ladder {R.step(got)}) -- it has '
                f'been lit for a room it is not in')
    print(f'  {len(GROUNDS)} cats lit for the ground they are actually on')

    for what, rect, ink, floor in READS:
        here = answer('studio_pixel', dict(
            zip(('x', 'y', 'width', 'height'), rect), ink=ink))
        if here['contrast'] < floor:
            raise ValueError(
                f'{what}: {ink} on {here["ground"]} is {here["contrast"]} to one '
                f'and wants {floor}')
    print(f'  {len(READS)} hand-drawn marks measured against their own artwork')


@stage
def export():
    path = str(Path(__file__).resolve().parents[2] / 'assets/skins/Catamp Seance.wsz')
    told = answer('studio_export', {'path': path})
    print(f"  {told['path']} · {told['bytes']} bytes")
    for key in ('undrawn_sprites', 'hard_to_read'):
        if told.get(key):
            print(f'  {key}: {told[key]}')


ORDER = ['blank', 'options', 'main', 'titlebar', 'readouts', 'transport',
         'switches', 'seek', 'volume', 'balance', 'eq', 'eq_bands', 'playlist',
         'palettes', 'measured', 'checked', 'export']


@stage
def blank():
    """Thirteen transparent classic atlases and nothing inherited. The recipe
    says what it starts from rather than taking whatever the editor happens to
    have open."""
    print(' ', answer('studio_new', {'discard': True})['message'])


if __name__ == '__main__':
    for name in sys.argv[1:] or ORDER:
        print(name)
        STAGES[name]()
