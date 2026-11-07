"""Tests for data_only (spec 43 step 2, Decision 163): a PR is data-only when every path it
changes is under metrics/ and none is under metrics/schema/."""
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import data_only  # noqa: E402


class DataOnlyTest(unittest.TestCase):
    def test_records_and_raw_material_are_data(self):
        for paths in [
            ["metrics/bench/2026-11-07-19203344.json"],
            ["metrics/bench/2026-11-07-19203344.json", "metrics/bench/2026-11-07-19203344-2.json"],
            ["metrics/raw/bench/19203344.log", "metrics/raw/bench/19203344.json"],
        ]:
            self.assertTrue(data_only.is_data_only(paths), paths)

    def test_the_schema_or_any_other_path_is_not(self):
        for paths in [
            ["metrics/bench/2026-11-07-1.json", "metrics/schema/record-v1.schema.json"],
            ["metrics/schema/record-v2.schema.json"],
            ["metrics/bench/2026-11-07-1.json", "sim/skirmish/formation/unit_store.gd"],
            ["metrics/bench/2026-11-07-1.json", "README.md"],
            ["ci/metrics/check_records.py"],
            ["metricsx/bench/1.json"],
        ]:
            self.assertFalse(data_only.is_data_only(paths), paths)

    def test_an_empty_diff_is_not_data_only(self):
        self.assertFalse(data_only.is_data_only([]))

    def test_it_writes_its_answer_as_a_step_output(self):
        lines = data_only.output_lines(True)
        self.assertEqual(lines, ["data_only=true"])
        self.assertEqual(data_only.output_lines(False), ["data_only=false"])


if __name__ == "__main__":
    unittest.main()
