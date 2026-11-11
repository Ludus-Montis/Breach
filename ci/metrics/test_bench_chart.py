"""Tests for bench_chart (spec 43 step 4): the bench summary's Mermaid xychart of the last
records' whole-tick ratio to the reference, read from metrics/bench/ and the new record."""
import json
import sys
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import bench_chart  # noqa: E402


def record(sha, position, ratio):
    reference = {"commit": "8ca4227" + "0" * 33, "results": {}, "ratio": {}}
    if ratio is not None:
        reference["ratio"]["tick"] = ratio
    return {"schema": "record-v1", "series": "bench",
            "commit": {"sha": sha * 40, "position": position},
            "source": {"harness": "breach_tick_phases"}, "reference": reference}


class ChartTest(unittest.TestCase):
    def test_points_follow_the_first_parent_line_and_keep_the_last(self):
        records = [record("c", 30, 0.9), record("a", 10, 1.0), record("b", 20, 0.95)]
        self.assertEqual(bench_chart.points(records, last=2),
                         [("bbbbbbb", 0.95), ("ccccccc", 0.9)])

    def test_the_25k_set_s_records_stay_off_the_4096_chart(self):
        check = record("b", 20, 0.5)
        check["source"]["args"] = ["side=24576", "squads=24", "series=1"]
        self.assertEqual(bench_chart.points([record("a", 10, 1.0), check]), [("aaaaaaa", 1.0)])

    def test_a_record_without_the_ratio_is_left_out(self):
        self.assertEqual(bench_chart.points([record("a", 10, None), record("b", 20, 1.1)]),
                         [("bbbbbbb", 1.1)])

    def test_the_chart_is_a_mermaid_xychart(self):
        lines = bench_chart.chart([("aaaaaaa", 1.0), ("bbbbbbb", 0.9)], "8ca4227")
        text = "\n".join(lines)
        self.assertTrue(text.startswith("```mermaid\nxychart-beta"))
        self.assertIn('x-axis ["aaaaaaa", "bbbbbbb"]', text)
        self.assertIn("line [1.0, 0.9]", text)
        self.assertIn("8ca4227", text)
        self.assertTrue(text.endswith("```"))

    def test_records_are_read_from_the_folder_and_the_new_one(self):
        folder = Path(tempfile.mkdtemp())
        (folder / "2026-11-07-1.json").write_text(json.dumps(record("a", 10, 1.0)))
        new = folder.parent / "new.json"
        new.write_text(json.dumps(record("b", 20, 0.9)))
        self.assertEqual(bench_chart.points(bench_chart.read([folder, new])),
                         [("aaaaaaa", 1.0), ("bbbbbbb", 0.9)])


if __name__ == "__main__":
    unittest.main()
