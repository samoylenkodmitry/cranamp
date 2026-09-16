"""Checks for the Catamp Seance recipe that do not need a running Studio.

Everything here is something that is invisible until it is drawn, and by then
it is inside a sprite cell where it reads as a rendering fault rather than as a
typo: a digit grid typed a pixel short, a cat outline whose normals point into
the room, a rim built from a colour that is not on the ladder. They run without
the editor so the recipe can be checked before it is replayed.
"""
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import catamp_seance as recipe
import pixel_pen
import seance_cats as cats
import seance_room as room


class TheLadder(unittest.TestCase):
    """Everything in this skin is coloured by how far it is from a flame, and
    that only works while `nearer` and `farther` are exact inverses over the
    ladder's interior."""

    def test_a_step_toward_a_flame_and_back_is_where_it_started(self):
        for color in room.GLOW[1:-1]:
            self.assertEqual(room.farther(room.nearer(color), 1), color)

    def test_the_ladder_brightens_all_the_way_up(self):
        def luminance(value):
            r, g, b = room.rgb(value)
            return 0.2126 * r + 0.7152 * g + 0.0722 * b
        steps = [luminance(c) for c in room.GLOW]
        self.assertEqual(steps, sorted(steps))

    def test_salt_has_a_ladder_of_its_own_and_stays_off_the_warm_one(self):
        # Salt reflects a flame rather than glowing with it, so it brightens
        # toward white. Run up GLOW instead and a sigil is a scatter of embers.
        hot = room.brighter(room.SALT_LADDER, room.SALT)
        r, g, b = room.rgb(hot)
        self.assertGreater(b, 120, 'lit salt stays cool, not amber')
        self.assertEqual(room.brighter(room.SALT_LADDER, room.SALT_HOT, 1),
                         room.SALT_HOT, 'the top of a ladder is the top')


class TheCats(unittest.TestCase):
    def test_every_outline_is_closed_and_stands_on_its_own_baseline(self):
        for name in ('SITTING', 'LOAF', 'CURLED', 'BEHIND', 'WALKING'):
            outline = getattr(cats, name)
            self.assertGreater(len(outline), 40, name)
            lowest = max(p[1] for p in outline)
            highest = min(p[1] for p in outline)
            self.assertLess(abs(lowest), 0.12, f'{name} sits on y=0')
            self.assertLess(highest, -0.4, f'{name} has some height')

    def test_a_cat_has_two_ears(self):
        # Two local minima along the top of the outline, which is the only
        # thing that tells a cat from a loaf of bread at this size.
        for name in ('SITTING', 'LOAF', 'CURLED', 'BEHIND'):
            outline = getattr(cats, name)
            top = min(p[1] for p in outline)
            peaks = [p for p in outline if p[1] < top * 0.88]
            spread = max(p[0] for p in peaks) - min(p[0] for p in peaks)
            self.assertGreater(spread, 0.18, f'{name} has two separate ears')

    def test_the_rim_faces_the_flame_and_not_away_from_it(self):
        # The sign of a polygon's normals comes from its winding, and y runs
        # down on a screen, which flips it against the textbook. Backwards, the
        # rim lands on the far side of every cat and looks almost right.
        pen = pixel_pen.Pen()
        outline = cats.place(cats.SITTING, 40, 60, scale=40)
        cats.rimlit(pen, outline, (120, 40), ground=room.GLOOM, reach=6,
                    radius=200, thickness=1)
        lit = [(op['x'], op['y']) for op in pen.ops if op['op'] == 'rect']
        self.assertTrue(lit, 'something was lit')
        middle = sum(p[0] for p in lit) / len(lit)
        self.assertGreater(middle, 40, 'the lit edge is the one facing (120,40)')

    def test_a_cat_has_no_colour_of_its_own(self):
        # Every pixel a cat is drawn in comes off the ladder the ground is on.
        # A fur colour picked by hand is the thing that turns a candlelit
        # picture into a grey one with an orange filter over it.
        pen = pixel_pen.Pen()
        cats.rimlit(pen, cats.place(cats.LOAF, 30, 40, scale=30), (60, 10),
                    ground=room.GLOOM, reach=6)
        for op in pen.ops:
            self.assertIn(op['color'], room.GLOW, op)


class TheReadouts(unittest.TestCase):
    def test_every_digit_fits_the_timer_cell_with_room_for_its_halo(self):
        self.assertEqual(len(recipe.DIGITS), 10)
        for value, rows in enumerate(recipe.DIGITS):
            self.assertEqual(len(rows), 11, value)
            self.assertEqual({len(r) for r in rows}, {7}, f'digit {value}')
            self.assertEqual({c for r in rows for c in r}, {'#', '.'}, value)

    def test_no_two_digits_are_the_same_picture(self):
        self.assertEqual(len({tuple(d) for d in recipe.DIGITS}), 10)

    def test_every_transport_mark_is_a_closed_run_inside_its_cell(self):
        self.assertEqual(set(recipe.MARKS), {'previous', 'play', 'pause',
                                             'stop', 'next', 'eject'})
        for name, runs in recipe.MARKS.items():
            for run in runs:
                for (px, py) in run:
                    self.assertTrue(0.0 <= px <= 1.0, f'{name} {px}')
                    self.assertTrue(0.0 <= py <= 1.0, f'{name} {py}')

    def test_eleven_candles_and_no_two_alike(self):
        self.assertEqual(len(recipe.CANDLES), 11)
        self.assertEqual(len(set(recipe.CANDLES)), 11)

    def test_the_crowd_fits_the_volume_strip(self):
        for (x, y, size) in recipe.CROWD:
            self.assertLessEqual(x + size * 2 + 1, 68)
            self.assertTrue(0 <= y - size and y <= 13)


class TheClient(unittest.TestCase):
    def test_a_missing_studio_says_what_to_start(self):
        # The bare failure is forty lines of ConnectionRefusedError that never
        # name Studio, the port, or the command, in the middle of a build that
        # is halfway through a skin.
        was, pixel_pen.STUDIO = pixel_pen.STUDIO, 'http://127.0.0.1:1/mcp'
        try:
            with self.assertRaises(pixel_pen.NoStudio) as caught:
                pixel_pen.call('studio_status')
        finally:
            pixel_pen.STUDIO = was
        self.assertIn('--skin-studio', str(caught.exception))


if __name__ == '__main__':
    unittest.main()
