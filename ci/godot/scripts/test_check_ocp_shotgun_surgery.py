"""Tests for check_ocp_shotgun_surgery.py's waiver: a broad change the owner authorised
in a Decision names it in a commit trailer, and the check reports and passes."""
import unittest

from check_ocp_shotgun_surgery import waiver


class WaiverTest(unittest.TestCase):
    def test_a_trailer_naming_a_decision_waives(self):
        messages = ["feat: x\n\nbody\n\nWaived-by: Decision 142\nCo-Authored-By: someone"]
        self.assertEqual(waiver(messages), "Decision 142")

    def test_any_commit_of_the_branch_may_carry_it(self):
        self.assertEqual(waiver(["fix: y", "feat: x\n\nWaived-by: Decision 130"]), "Decision 130")

    def test_no_trailer_no_waiver(self):
        self.assertIsNone(waiver(["feat: x\n\nmentions Decision 142 in passing"]))

    def test_the_trailer_must_name_a_decision_by_number(self):
        self.assertIsNone(waiver(["feat: x\n\nWaived-by: the user"]))


if __name__ == "__main__":
    unittest.main()
