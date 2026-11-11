"""Tests for series_25k (spec 43 step 8b, Decision 165): `bench-series` with `set=25k` times
the 25k set (bench_set.shapes) on each Rust-era commit, the two checks beside the bridge, and
writes one record a shape; a shape the commit's harness can't run writes none."""
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import bench_series  # noqa: E402
import bench_set  # noqa: E402
import series_25k  # noqa: E402

CARGO = "native/rust/src/bin/tick_phases.rs"
GD_PHASES = "tools/tick_phases.gd"
GD_WHOLE = "tools/formation_bench.gd"

SERIES_ROUND = """machine: AMD EPYC 7763 64-Core Processor, 4 threads, Linux
rust, 24576 a side, 32 wide: {tick} ms a tick
  scrum: seek                             11.00
series: 120 ticks from tick 0 (ms)     mean      95th     99th    worst  at tick
  tick                                {tick}   {p95}   {p99}   99.00       40
  scrum: seek                             11.00    20.00    25.00    30.00       41
"""

# A harness from before `series`: the battle's own tick, with no series under it.
NO_SERIES_ROUND = """machine: AMD EPYC 7763 64-Core Processor, 4 threads, Linux
rust, 24576 a side, 32 wide: 40.00 ms a tick
  scrum: seek                             11.00
"""


def rounds(*ticks):
    return [SERIES_ROUND.format(tick=f"{t:.2f}", p95=f"{t + 10:.2f}", p99=f"{t + 20:.2f}")
            for t in ticks]


def context():
    return {
        "commit": {"sha": "c" * 40},
        "machine": {"godot": "4.7.2.stable.official"},
        "provenance": {"run_id": 1, "kind": "fresh"},
        "blob": "b" * 40,
    }


class CommitsTest(unittest.TestCase):
    """With `set=25k` the default commits are those the 4,096 backfill times with the crate's
    `breach_tick_phases`: the GDScript era is left out."""

    def test_only_commits_whose_tree_has_the_cargo_binary_are_kept(self):
        trees = {"gd1": {GD_PHASES, GD_WHOLE}, "bridge": {CARGO, GD_PHASES, GD_WHOLE},
                 "rust1": {CARGO}, "gd2": {GD_WHOLE}, "rust2": {CARGO}}
        commits = ["gd1", "bridge", "rust1", "gd2", "rust2"]
        self.assertEqual(series_25k.commits(commits, trees.get),
                         ["bridge", "rust1", "rust2"])

    def test_the_order_of_main_s_line_is_kept(self):
        trees = {"b": {CARGO}, "a": {CARGO}}
        self.assertEqual(series_25k.commits(["b", "a"], trees.get), ["b", "a"])

    def test_the_4096_set_keeps_every_commit(self):
        trees = {"gd": {GD_WHOLE}, "rust": {CARGO}}
        self.assertEqual(bench_series.set_commits(["gd", "rust"], "4096", trees.get),
                         ["gd", "rust"])
        self.assertEqual(bench_series.set_commits(["gd", "rust"], "25k", trees.get), ["rust"])

    def test_an_unknown_set_is_refused(self):
        with self.assertRaises(ValueError):
            bench_series.set_commits(["a"], "8k", lambda sha: set())


class ShapesTest(unittest.TestCase):
    def test_the_set_is_step_8a_s_with_the_bridge_beside_the_checks_only(self):
        self.assertEqual(series_25k.shapes(), bench_set.shapes())
        beside = [shape for shape, bridge in series_25k.shapes() if bridge]
        self.assertEqual(beside, ["side=24576 squads=24", "side=24576 squads=1"])


class ShapeRecordTest(unittest.TestCase):
    def test_a_grid_shape_writes_a_series_record_without_a_reference(self):
        record = series_25k.shape_record("side=4096 squads=4", rounds(40.1, 40.3, 40.2), [],
                                         context())
        self.assertEqual(record["source"]["args"], ["side=4096", "squads=4", "series=1"])
        self.assertEqual(record["source"]["harness"], "breach_tick_phases")
        self.assertEqual(record["results"]["tick"]["median"], 40.2)
        self.assertEqual(record["results"]["tick-p95"]["median"], 50.2)
        self.assertEqual(record["results"]["tick-p99"]["median"], 60.2)
        self.assertEqual(record["machine"]["godot"], {"na": "not-applicable"})
        self.assertNotIn("reference", record)
        self.assertEqual(bench_set._shape(record), "side=4096 squads=4")

    def test_a_check_writes_its_ratio_to_the_bridge(self):
        record = series_25k.shape_record("side=24576 squads=24", rounds(40.0, 40.0, 40.0),
                                         rounds(80.0, 80.0, 80.0), context())
        self.assertEqual(record["reference"]["commit"], bench_series.BRIDGE)
        self.assertEqual(record["reference"]["ratio"]["tick"], 0.5)

    def test_a_harness_that_rejects_the_shape_writes_no_record(self):
        """Given a Rust-era commit whose harness rejects `series=1` (exit 2, no output)."""
        self.assertIsNone(series_25k.shape_record("side=24576 squads=1", ["", "", ""],
                                                  rounds(80.0, 80.0, 80.0), context()))

    def test_a_harness_that_prints_no_series_writes_no_record(self):
        self.assertIsNone(series_25k.shape_record("side=4096 squads=4",
                                                  [NO_SERIES_ROUND] * 3, [], context()))

    def test_one_round_without_its_series_writes_no_record(self):
        heads = rounds(40.0, 40.0) + [""]
        self.assertIsNone(series_25k.shape_record("side=4096 squads=4", heads, [], context()))


class RunTest(unittest.TestCase):
    """The job still succeeds when a shape is skipped: the other shapes are written."""

    def test_a_skipped_shape_leaves_the_others_written(self):
        import tempfile
        rejected = "side=24576 squads=1"

        def timer(shape, folder):
            return "" if shape == rejected else rounds(40.0)[0]

        out = Path(tempfile.mkdtemp())
        written = series_25k.time_commit(timer, context(), 0, {
            "out": str(out), "date": "2026-11-11", "run_id": "7", "attempt": "1"})
        self.assertEqual(len(written), len(bench_set.shapes()) - 1)
        self.assertEqual(len({path.name for path in written}), len(written))

    def test_each_record_has_its_own_number_across_commits_and_attempts(self):
        numbers = {series_25k.record_number(place, index, attempt)
                   for place in range(30) for index in range(len(series_25k.shapes()))
                   for attempt in (1, 2)}
        self.assertEqual(len(numbers), 30 * len(series_25k.shapes()) * 2)


if __name__ == "__main__":
    unittest.main()
