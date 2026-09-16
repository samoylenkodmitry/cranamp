"""Checks for the Catamp Sampler recipe that do not need a running Studio.

A grid typed a pixel short is invisible until it is drawn, and by then it is in
a sprite cell where it reads as a rendering fault rather than as a typo. These
run without the editor, so a recipe can be checked before it is replayed.
"""
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import sampler_cats as cats
import sampler_cloth as cloth


class SamplerGrids(unittest.TestCase):
    def test_every_hand_authored_grid_fills_its_cell(self):
        self.assertTrue(cats.check())

    def test_a_short_row_is_reported_with_its_number(self):
        with self.assertRaises(ValueError) as caught:
            cats.check(cells=(('TEST', ('xx', 'x'), 2, 2),))
        self.assertIn('row 1', str(caught.exception))

    def test_the_litter_has_one_coat_and_one_collar_per_band(self):
        self.assertEqual(len(cats.LITTER), 11)
        self.assertEqual(len(cats.COLLARS), 11)
        self.assertEqual(len({id(c) for c in cats.LITTER}), 11)

    def test_a_coat_defines_every_symbol_its_grids_use(self):
        used = {c for rows in (cats.KITTEN, cats.KITTEN_HELD, cats.LOAF,
                               cats.HANDLE_HEAD, cats.HANDLE_HEAD_HELD)
                for row in rows for c in row if c != ' '}
        for coat in cats.LITTER:
            table = cats._pal(coat)
            self.assertEqual(used - set(table), set())


class SamplerCloth(unittest.TestCase):
    def test_a_counted_digit_fills_exactly_its_nine_by_thirteen_cell(self):
        # Four chart squares across and six down, each a three-pixel cross on a
        # two-pixel pitch. Any other chart shape overruns the cell or leaves a
        # margin the rest of the sheet does not have.
        for value, chart in cloth._DIGIT_CELLS.items():
            self.assertEqual(len(chart), 6, f'digit {value}')
            for row in chart:
                self.assertEqual(len(row), 4, f'digit {value}')
        self.assertEqual((4 - 1) * 2 + 3, 9)
        self.assertEqual((6 - 1) * 2 + 3, 13)

    def test_the_small_face_advance_matches_the_engines(self):
        # The engine advances 4*scale+spacing for face "small"; centring a word
        # in a cell depends on this agreeing.
        self.assertEqual(cloth.micro_width('16K'), 14)
        self.assertEqual(cloth.micro_width('PRE'), 14)
        self.assertEqual(cloth.text_width('PLAYLIST'), 47)

    def test_micro_asks_the_engine_for_the_small_face(self):
        pen = _Pen()
        cloth.micro(pen, 0, 0, 'MONO', '#ffffff')
        self.assertEqual(pen.ops[-1]['face'], 'small')

    def test_a_running_stitch_phases_from_the_absolute_coordinate(self):
        # Two lines drawn separately have to line up, and a tile that repeats
        # has to meet itself. Both follow from the phase, not from the start.
        left, right = _Pen(), _Pen()
        cloth.running(left, 0, 0, 12, '#000000', phase=0)
        cloth.running(right, 12, 0, 12, '#000000', phase=12)
        starts = [op['x'] for op in left.ops] + [op['x'] for op in right.ops]
        self.assertEqual(starts, sorted(starts))
        for a, b in zip(starts, starts[1:]):
            self.assertEqual(b - a, 4)


class _Pen:
    """The two operations these checks need, without a socket behind them."""

    def __init__(self):
        self.ops = []

    def rect(self, x, y, w, h, c, ramp=None, axis=None):
        self.ops.append(dict(op='rect', x=x, y=y, width=w, height=h, color=c))

    def stamp(self, x, y, rows, palette):
        self.ops.append(dict(op='stamp', x=x, y=y, rows=rows, palette=palette))

    def pixel(self, x, y, c):
        self.rect(x, y, 1, 1, c)


if __name__ == '__main__':
    unittest.main()
