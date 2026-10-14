"""Tests for check_one_mover.py: a position written outside the mover is caught; the
mover's own steps and marked placements are not."""
import unittest

from check_one_mover import violations_in


class CheckOneMoverTest(unittest.TestCase):
    def test_a_bare_position_write_is_caught(self):
        for code in [
            "\tunit.position = at + step",
            '\tentry["at"] = UnitMotion.walk(unit, at, to, full, seconds)',
            '\tentry["at"] += push',
            '\tsquad.fleeing[unit.id]["offset"] += side * aside',
            '\tentry["along"] = move_toward(entry["along"], home, full)',
        ]:
            self.assertEqual(len(violations_in(code)), 1, code)

    def test_the_movers_steps_are_fine(self):
        code = '\tentry["at"] = UnitMover.step(unit, entry["at"], to, full, seconds, terrain)'
        self.assertEqual(violations_in(code), [])

    def test_a_marked_placement_is_fine(self):
        self.assertEqual(violations_in("\tunit.position = bearer.position  # placement: carried"), [])
        above = "\t# placement: a carried body is where its bearer is\n\tunit.position = bearer.position"
        self.assertEqual(violations_in(above), [])

    def test_a_placement_mark_needs_a_reason(self):
        self.assertEqual(len(violations_in("\tunit.position = x  # placement:")), 1)

    def test_paid_needs_the_mover_in_the_file(self):
        line = '\tentry["offset"] += side * aside * paid  # paid: UnitMover.share'
        self.assertEqual(len(violations_in(line)), 1)
        with_mover = "\tvar paid := UnitMover.share(unit, at, to, terrain)\n" + line
        self.assertEqual(violations_in(with_mover), [])

    def test_reading_or_comparing_a_position_is_fine(self):
        for code in [
            "\tvar at := unit.position",
            '\tif entry["at"] == entry["next"]:',
            '\tvar d: float = entry["at"].distance_to(goal)',
        ]:
            self.assertEqual(violations_in(code), [], code)


if __name__ == "__main__":
    unittest.main()
