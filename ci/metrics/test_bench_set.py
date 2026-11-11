"""Tests for bench_set (spec 43 step 8a, Decision 165): the 25k set's shapes, the fit of
`b·x + c·x²` on each shape's grid, the miss at 24,576 a side and the data PR's flags."""
import json
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import bench_set  # noqa: E402

FIXTURES = json.loads((HERE / "fixtures" / "bench_fit.json").read_text(encoding="utf-8"))


def curve(b, c):
    return lambda x: b * x + c * x * x


def records(position, tick_at, check_ticks):
    """One fresh record per shape of the set at `position`: the grid's ticks from `tick_at`
    (per family), the checks' from `check_ticks` ({family: ms})."""
    out = []
    for family, shapes in bench_set.FAMILIES.items():
        for shape in shapes["grid"]:
            out.append(record(position, shape, tick_at[family](bench_set.side(shape))))
        out.append(record(position, shapes["check"], check_ticks[family]))
    return out


def record(position, shape, tick):
    return {"series": "bench", "commit": {"sha": f"{position:040d}", "position": position},
            "source": {"harness": "breach_tick_phases", "args": shape.split() + ["series=1"]},
            "results": {"tick": {"unit": "ms", "values": [tick], "median": tick}},
            "provenance": {"kind": "fresh", "run_id": position}}


class ShapesTest(unittest.TestCase):
    def test_the_set_is_nine_shapes_with_the_bridge_beside_the_two_checks(self):
        shapes = bench_set.shapes()
        self.assertEqual(len(shapes), 9)
        bridged = [args for args, bridge in shapes if bridge]
        self.assertEqual(bridged, ["side=24576 squads=24", "side=24576 squads=1"])
        self.assertIn(("side=1024 squads=1", False), shapes)
        self.assertIn(("side=12288 squads=1", False), shapes)

    def test_a_record_marked_series_settled_has_the_shape_of_one_without(self):
        before = record(900, "side=4096 squads=4", 30.0)
        after = record(901, "side=4096 squads=4", 40.0)
        after["source"]["args"].append(bench_set.SETTLED_ARG)
        self.assertEqual(bench_set.by_commit([before, after]),
                         {900: {"side=4096 squads=4": 30.0}, 901: {"side=4096 squads=4": 40.0}})


class FitTest(unittest.TestCase):
    def test_points_on_the_curve_give_back_b_and_c_and_the_24576_tick(self):
        case = FIXTURES["exact"]
        b, c = bench_set.fit(case["points"])
        self.assertAlmostEqual(b, case["b"], places=9)
        self.assertAlmostEqual(c / case["c"], 1.0, places=6)
        self.assertAlmostEqual(bench_set.predict(b, c, case["at"]), case["predicted"], places=6)

    def test_a_grid_that_would_need_a_negative_c_fits_c_zero(self):
        case = FIXTURES["negative_c"]
        b, c = bench_set.fit(case["points"])
        self.assertEqual(c, 0)
        self.assertAlmostEqual(b, case["b"], places=12)


class SummaryTest(unittest.TestCase):
    def setUp(self):
        self.grid = {"lanes": curve(0.002, 0.0), "one squad": curve(0.001, 1e-8)}
        self.lanes_at = 0.002 * 24576
        self.squad_at = 0.001 * 24576 + 1e-8 * 24576 ** 2

    def summary(self, new, history=()):
        return "\n".join(bench_set.summary(new, list(history)))

    def test_a_miss_within_10_percent_is_printed_and_not_flagged(self):
        text = self.summary(records(900, self.grid, {"lanes": self.lanes_at,
                                                    "one squad": self.squad_at}))
        self.assertIn("lanes", text)
        self.assertIn("+0.0%", text)
        self.assertNotIn("FLAG", text)

    def test_a_check_predicted_12_percent_over_the_measured_is_flagged(self):
        new = records(900, self.grid, {"lanes": self.lanes_at / 1.12, "one squad": self.squad_at})
        text = self.summary(new)
        self.assertIn("lanes: miss +12.0%", text)
        self.assertIn("FLAG", text)
        self.assertNotIn("no longer fits", text)

    def test_two_flagged_merges_in_a_row_on_one_shape_say_the_form_no_longer_fits(self):
        before = records(899, self.grid, {"lanes": self.lanes_at / 1.15,
                                          "one squad": self.squad_at})
        new = records(900, self.grid, {"lanes": self.lanes_at / 1.12, "one squad": self.squad_at})
        text = self.summary(new, before)
        self.assertIn("lanes: the form no longer fits", text)
        self.assertNotIn("one squad: the form no longer fits", text)

    def test_a_flag_two_merges_back_with_a_clean_one_between_is_not_in_a_row(self):
        older = records(898, self.grid, {"lanes": self.lanes_at / 1.15,
                                         "one squad": self.squad_at})
        clean = records(899, self.grid, {"lanes": self.lanes_at, "one squad": self.squad_at})
        new = records(900, self.grid, {"lanes": self.lanes_at / 1.12, "one squad": self.squad_at})
        self.assertNotIn("no longer fits", self.summary(new, older + clean))

    def test_records_outside_the_set_are_ignored(self):
        new = records(900, self.grid, {"lanes": self.lanes_at, "one squad": self.squad_at})
        other = record(900, "side=4096 squads=4 fight=3", 1000.0)
        other["source"]["args"] = ["side=4096", "squads=4", "fight=3"]
        self.assertNotIn("FLAG", self.summary(new + [other]))


if __name__ == "__main__":
    unittest.main()
