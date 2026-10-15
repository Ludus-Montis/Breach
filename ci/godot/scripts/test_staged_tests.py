"""Tests for staged_tests.py: which gdUnit4 suites the pre-commit hook runs."""
import unittest

from staged_tests import staged_tests


class StagedTestsTest(unittest.TestCase):
    def test_staged_suites_run_and_nothing_else(self):
        self.assertEqual(
            staged_tests(
                [
                    "tests/sim/skirmish/formation/test_unit_store.gd",
                    "sim/skirmish/formation/unit_store.gd",
                    "OPEN_QUESTIONS.md",
                ]
            ),
            ["res://tests/sim/skirmish/formation/test_unit_store.gd"],
        )

    def test_helpers_under_tests_are_not_suites(self):
        self.assertEqual(staged_tests(["tests/sim/skirmish/formation/unit_places.gd"]), [])

    def test_no_suite_staged_runs_nothing(self):
        for paths in [[], ["CLAUDE.md"], ["sim/skirmish/formation/unit_store.gd"]]:
            self.assertEqual(staged_tests(paths), [], paths)

    def test_order_is_kept_and_repeats_dropped(self):
        self.assertEqual(
            staged_tests(["tests/b/test_two.gd", "tests/a/test_one.gd", "tests/b/test_two.gd"]),
            ["res://tests/b/test_two.gd", "res://tests/a/test_one.gd"],
        )

    def test_windows_separators_are_normalised(self):
        self.assertEqual(staged_tests(["tests\\a\\test_one.gd"]), ["res://tests/a/test_one.gd"])


if __name__ == "__main__":
    unittest.main()
