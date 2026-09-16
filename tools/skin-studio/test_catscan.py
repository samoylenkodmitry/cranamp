"""Checks for the Catamp Cat Scan recipe that do not need a running Studio.

A grid typed a pixel short and a caption four pixels too long are both
invisible until they are drawn, and by then they are in a sprite cell where
they read as a rendering fault rather than as a typo. These run without the
editor, so the recipe can be checked before it is replayed.
"""
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import catscan_cats as cats
import catscan_film as film


class CatScanGrids(unittest.TestCase):
    def test_every_hand_authored_grid_fills_its_cell(self):
        self.assertTrue(cats.check())

    def test_a_short_row_is_reported_with_its_number(self):
        rows = ('###', '##')
        with self.assertRaises(ValueError) as caught:
            cats.check(extra={'TEST': rows})
        self.assertIn('row 1', str(caught.exception))

    def test_eleven_things_were_swallowed_and_each_fits_a_handle(self):
        bodies = cats.foreign_bodies()
        self.assertEqual(len(bodies), 11)
        self.assertEqual(len({name for name, _ in bodies}), 11)
        for name, rows in bodies:
            self.assertLessEqual(max(len(r) for r in rows), 12, name)
            self.assertLessEqual(len(rows), 23, name)
            # ...and fills enough of it to be told apart from its neighbours.
            self.assertGreaterEqual(len(rows), 7, name)

    def test_the_preamp_fish_fits_the_same_cell(self):
        self.assertLessEqual(max(len(r) for r in cats.FISH), 12)
        self.assertLessEqual(len(cats.FISH), 25)

    def test_an_anatomy_has_an_inside(self):
        # Freehanding a skull produces a ring: the hand draws the outline it
        # can see and leaves the middle empty, and on a radiograph the middle
        # is the picture. Every composed anatomy has to have ink in its centre.
        for name, grid in (('lateral', cats.skull_lateral()),
                           ('frontal', cats.skull_frontal()),
                           ('vertebra', cats.vertebra())):
            rows = grid.rows()
            middle = rows[len(rows) // 2]
            self.assertNotEqual(middle[len(middle) // 2], ' ', name)

    def test_a_grid_only_uses_densities_the_palette_defines(self):
        composed = [('lateral', cats.skull_lateral().rows()),
                    ('frontal', cats.skull_frontal().rows()),
                    ('curled', _built(cats.curled_cat, 40, 17))]
        for name, rows in composed:
            for row in rows:
                for ch in row:
                    self.assertTrue(ch == ' ' or ch in cats.PALETTE,
                                    f'{name}: {ch!r}')


class CatScanFilm(unittest.TestCase):
    def test_the_small_face_advance_matches_the_engines(self):
        # The engine advances 4*scale+spacing for face "small"; centring a word
        # in a cell and right-aligning one both depend on this agreeing.
        self.assertEqual(film.text_width('16K'), 14)
        self.assertEqual(film.text_width('PRE'), 14)
        self.assertEqual(film.text_width('CATAMP', face='5x7'), 35)

    def test_a_right_aligned_caption_ends_where_it_is_told(self):
        # The playlist header tile is drawn nine times and the title cell ends
        # at sheet column 125, so a caption laid out left to right runs into
        # the tile and is repeated across the whole header.
        pen = _Pen()
        film.right(pen, 125, 0, 'CASE NOTES')
        end = pen.ops[-1]['x'] + film.text_width('CASE NOTES')
        self.assertEqual(end, 125)
        self.assertGreaterEqual(pen.ops[-1]['x'], 0)

    def test_the_density_ladder_is_ordered_dark_to_light(self):
        values = [sum(film.rgb(c)) for c in film.LADDER]
        self.assertEqual(values, sorted(values))

    def test_denser_and_thinner_walk_that_ladder(self):
        self.assertEqual(film.denser(film.BONE), film.BONE_LIT)
        self.assertEqual(film.thinner(film.BONE), film.BONE_DIM)
        # and they stop at the ends rather than falling off them
        self.assertEqual(film.denser(film.METAL), film.METAL)
        self.assertEqual(film.thinner(film.AIR), film.AIR)

    def test_a_pressed_chip_is_brighter_than_a_released_one(self):
        # The one state change in this skin that runs the other way: a chip
        # pressed flat against the diffuser loses its shadow and the light
        # behind it comes up. Flipping a bevel moves two pixels of a 23x18 cell.
        released, pressed = _Pen(), _Pen()
        film.chip(released, 1, 1, 21, 16, pressed=False)
        film.chip(pressed, 1, 1, 21, 16, pressed=True)
        self.assertGreater(_brightest(pressed), _brightest(released))
        self.assertIn(film.GLOW, [op['color'] for op in pressed.ops])
        # ...and the released one still has the shadow it is floating on
        self.assertIn(film.STEEL_DEEP, [op['color'] for op in released.ops])
        self.assertNotIn(film.STEEL_DEEP, [op['color'] for op in pressed.ops])

    def test_a_lit_plate_and_a_dark_one_are_not_two_brightnesses_of_one(self):
        on, off = _Pen(), _Pen()
        film.lit_plate(on, 0, 0, 20, 12, on=True)
        film.lit_plate(off, 0, 0, 20, 12, on=False)
        self.assertGreater(_brightest(on), _brightest(off) + 300)


def _built(builder, w, h):
    pen = _Pen()
    builder(pen, 0, 0, w, h)
    return [op['rows'] for op in pen.ops if op['op'] == 'stamp'][-1]


def _brightest(pen):
    return max(sum(film.rgb(op['color'])) for op in pen.ops if 'color' in op)


class _Pen:
    """The operations these checks need, without a socket behind them."""

    def __init__(self):
        self.ops = []

    def rect(self, x, y, w, h, c, ramp=None, axis=None):
        self.ops.append(dict(op='rect', x=x, y=y, width=w, height=h, color=c))

    def ellipse(self, x, y, w, h, c, fill=True, width=1):
        self.ops.append(dict(op='ellipse', x=x, y=y, width=w, height=h,
                             color=c))

    def stamp(self, x, y, rows, palette):
        self.ops.append(dict(op='stamp', x=x, y=y, rows=rows, palette=palette))

    def pixel(self, x, y, c):
        self.rect(x, y, 1, 1, c)

    def curve(self, start, control, end, width, c, taper=False,
              clean_corners=False):
        self.ops.append(dict(op='curve', x=start[0], y=start[1], color=c))

    def taper(self, start, control, end, width, c):
        self.ops.append(dict(op='path', x=start[0], y=start[1], color=c))

    def path(self, points, c, fill=True, width=1, ramp=None, axis=None):
        self.ops.append(dict(op='path', x=0, y=0, points=points, color=c))


if __name__ == '__main__':
    unittest.main()
