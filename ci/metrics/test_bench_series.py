"""Tests for bench_series (spec 43 step 5, Decision 162): the backfill times each commit with
its own harness, the cargo binary where its tree has one and otherwise the GDScript tools (the
bridge with both), against its era's reference, and writes a record per harness."""
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import bench_series  # noqa: E402

BRIDGE = bench_series.BRIDGE
CARGO = "native/rust/src/bin/tick_phases.rs"
GD_PHASES = "tools/tick_phases.gd"
GD_WHOLE = "tools/formation_bench.gd"

PHASES = """machine: AMD EPYC 7763 64-Core Processor, 4 threads, Linux
rust, 4096 a side, 32 wide: {tick} ms a tick
  scrum: seek                             {seek}
  deaths (bury)                            0.24
"""

WHOLE = """width 32, ms a tick (avg / worst)
  a side              marching             mid-fight  contact tick
    1024        5.2 /      6.0       {fight} /     55.2  33
"""


def context():
    return {
        "commit": {"sha": "c" * 40},
        "machine": {"godot": "4.7.2.stable.official"},
        "provenance": {"run_id": 1, "kind": "fresh"},
        "blob": "b" * 40,
    }


class HarnessTest(unittest.TestCase):
    def test_a_gdscript_era_tree_takes_both_gdscript_tools(self):
        self.assertEqual(bench_series.harnesses({GD_PHASES, GD_WHOLE}, "a" * 40),
                         ["tick_phases.gd", "formation_bench.gd"])

    def test_a_tree_with_the_cargo_binary_takes_it_alone(self):
        self.assertEqual(bench_series.harnesses({CARGO, GD_PHASES, GD_WHOLE}, "a" * 40),
                         ["breach_tick_phases"])
        self.assertEqual(bench_series.harnesses({CARGO}, "a" * 40), ["breach_tick_phases"])

    def test_the_bridge_takes_every_harness(self):
        self.assertEqual(bench_series.harnesses({CARGO, GD_PHASES, GD_WHOLE}, BRIDGE),
                         ["breach_tick_phases", "tick_phases.gd", "formation_bench.gd"])

    def test_a_tree_with_only_one_gdscript_tool_takes_that_one(self):
        self.assertEqual(bench_series.harnesses({GD_WHOLE}, "a" * 40), ["formation_bench.gd"])
        self.assertEqual(bench_series.harnesses(set(), "a" * 40), [])

    def test_the_references_are_commits_on_main(self):
        import subprocess
        for sha in (BRIDGE, bench_series.GDSCRIPT_REFERENCE):
            done = subprocess.run(["git", "-C", str(HERE), "cat-file", "-t", sha],
                                  capture_output=True, text=True)
            self.assertEqual(done.stdout.strip(), "commit", sha)

    def test_each_era_has_its_reference(self):
        self.assertEqual(bench_series.HARNESSES["breach_tick_phases"]["reference"], BRIDGE)
        self.assertEqual(bench_series.HARNESSES["tick_phases.gd"]["reference"],
                         bench_series.GDSCRIPT_REFERENCE)
        self.assertEqual(bench_series.HARNESSES["formation_bench.gd"]["reference"],
                         bench_series.GDSCRIPT_REFERENCE)


class RecordTest(unittest.TestCase):
    def test_formation_bench_gives_the_mid_fight_average_as_the_tick(self):
        self.assertEqual(bench_series.whole_tick(WHOLE.format(fight="40.1")), {"tick": 40.1})
        self.assertEqual(bench_series.whole_tick("SCRIPT ERROR: Parse Error"), {})

    def test_a_commit_whose_tick_phases_fails_keeps_its_whole_tick_and_gaps_its_phases(self):
        reference = [PHASES.format(tick="33.4", seek="11.0")] * 3
        failed = bench_series.harness_record("tick_phases.gd", ["", "", ""], reference, context())
        gap = {"na": "not-measured"}
        self.assertEqual(failed["results"]["tick/scrum/seek"],
                         {"unit": "ms", "values": gap, "median": gap})
        self.assertEqual(failed["results"]["tick"]["median"], gap)
        whole = bench_series.harness_record(
            "formation_bench.gd", [WHOLE.format(fight=f) for f in ("40.1", "40.3", "40.2")],
            [WHOLE.format(fight="50.0")] * 3, context())
        self.assertEqual(whole["results"]["tick"]["median"], 40.2)
        self.assertEqual(whole["source"]["args"], ["sizes=1024"])
        self.assertEqual(whole["source"]["harness"], "formation_bench.gd")
        self.assertEqual(whole["reference"]["commit"], bench_series.GDSCRIPT_REFERENCE)

    def test_a_gdscript_record_names_godot_and_a_cargo_one_does_not(self):
        reference = [PHASES.format(tick="33.4", seek="11.0")] * 3
        cargo = bench_series.harness_record("breach_tick_phases", reference, reference, context())
        gdscript = bench_series.harness_record("tick_phases.gd", reference, reference, context())
        self.assertEqual(cargo["machine"]["godot"], {"na": "not-applicable"})
        self.assertEqual(gdscript["machine"]["godot"], "4.7.2.stable.official")
        self.assertEqual(gdscript["source"]["args"],
                         ["engine=rust", "side=4096", "squads=4", "fight=3"])


class SharedBuildTest(unittest.TestCase):
    """Every worktree builds into one shared folder, so a commit only rebuilds what changed."""

    def test_each_worktree_s_build_folder_is_the_shared_one(self):
        import tempfile
        root = Path(tempfile.mkdtemp())
        shared = root / "shared"
        for name in ("one", "two"):
            (root / name / "native" / "rust").mkdir(parents=True)
            bench_series.link_target(root / name, shared)
            target = root / name / "native" / "rust" / "target"
            self.assertTrue(target.is_symlink())
            self.assertEqual(target.resolve(), shared.resolve())

    def test_a_build_folder_already_there_is_replaced_by_the_link(self):
        import tempfile
        root = Path(tempfile.mkdtemp())
        (root / "one" / "native" / "rust" / "target" / "release").mkdir(parents=True)
        bench_series.link_target(root / "one", root / "shared")
        self.assertTrue((root / "one" / "native" / "rust" / "target").is_symlink())

    def test_a_worktree_with_its_binaries_needs_no_build(self):
        import tempfile
        folder = Path(tempfile.mkdtemp())
        self.assertTrue(bench_series.needs_build(folder))
        (folder / "native" / "bin").mkdir(parents=True)
        (folder / "native" / "bin" / "breach_tick_phases").write_text("")
        self.assertFalse(bench_series.needs_build(folder))


class PlanTest(unittest.TestCase):
    def test_commits_are_split_into_jobs_that_keep_their_place(self):
        jobs = bench_series.plan(["a", "b", "c", "d", "e"], 2)
        self.assertEqual(jobs, [{"index": 0, "offset": 0, "commits": "a b"},
                                {"index": 1, "offset": 2, "commits": "c d"},
                                {"index": 2, "offset": 4, "commits": "e"}])

    def test_each_record_has_its_own_number_across_jobs_and_attempts(self):
        self.assertEqual(bench_series.record_number(0, 0, 1), 1)
        self.assertEqual(bench_series.record_number(4, 2, 1), 15)
        self.assertEqual(bench_series.record_number(4, 2, 2), 1015)


if __name__ == "__main__":
    unittest.main()
