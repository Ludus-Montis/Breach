"""Tests for bench_record (spec 43 step 4, Decision 162): a bench run of a commit on main's
first-parent line becomes one record, holding every phase's rounds and median from
breach_tick_phases, the reference's same measurement and the ratio; a run of any other commit
writes none."""
import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import bench_record  # noqa: E402
import check_records  # noqa: E402

SCHEMA_DIR = HERE.parent.parent / "metrics" / "schema"

try:
    import jsonschema  # noqa: F401
    HAVE_JSONSCHEMA = True
except ImportError:  # CI installs it (metrics.yml); a bare local Python may not
    HAVE_JSONSCHEMA = os.environ.get("METRICS_REQUIRE_JSONSCHEMA") == "1"

ROUND = """machine: AMD EPYC 7763 64-Core Processor, 4 threads, Linux
rust, 4096 a side, 32 wide: {tick} ms a tick
  scrum: seek                             {seek}
  deaths (bury)                            0.24
  rout: regroup's search                   {regroup}
  scrum: engage, prepare, stance           1.64
"""


def rounds(*ticks, seek=11.0, regroup="0.07"):
    return [ROUND.format(tick=t, seek=seek + i, regroup=regroup) for i, t in enumerate(ticks)]


def context():
    return {
        "commit": {"sha": "a" * 40, "position": 812, "pr": 781, "title": "chore: x",
                   "summary": {"na": "not-recorded"}, "step": "step:43-3",
                   "outcomes_changed": False},
        "source": {"harness": "breach_tick_phases", "blob": "b" * 40,
                   "args": ["side=4096", "squads=4", "fight=3"], "rounds": 3,
                   "profile": "release"},
        "machine": {"runner_image": "ubuntu24", "runner_version": "20261101.1",
                    "cpu": "AMD EPYC 7763 64-Core Processor", "cores": 4, "ram_mb": 15990,
                    "os": "Ubuntu 24.04.3 LTS", "godot": {"na": "not-applicable"},
                    "rustc": "rustc 1.92.0"},
        "provenance": {"run_id": 42412663531, "url": "https://example/run",
                       "date": "2026-11-07", "kind": "fresh"},
        "reference_commit": "8ca422726e68909554eddad7eeee2199e0fe656d",
    }


def git(repo, *args):
    return subprocess.run(["git", "-C", str(repo), *args], check=True, capture_output=True,
                          text=True).stdout.strip()


class PathTest(unittest.TestCase):
    def test_phase_names_become_paths_under_the_tick(self):
        self.assertEqual(bench_record.phase_path("tick"), "tick")
        self.assertEqual(bench_record.phase_path("scrum: seek"), "tick/scrum/seek")
        self.assertEqual(bench_record.phase_path("deaths (bury)"), "tick/deaths-bury")
        self.assertEqual(bench_record.phase_path("rout: regroup's search"),
                         "tick/rout/regroups-search")
        self.assertEqual(bench_record.phase_path("scrum: engage, prepare, stance"),
                         "tick/scrum/engage-prepare-stance")


class RecordTest(unittest.TestCase):
    def setUp(self):
        self.out = Path(tempfile.mkdtemp())

    def record(self, **overrides):
        head = overrides.get("head", rounds("30.3", "30.1", "30.5"))
        reference = overrides.get("reference", rounds("33.4", "33.3", "33.7", seek=12.0))
        return bench_record.build_record(head, reference, context())

    def test_each_phase_has_its_rounds_and_median(self):
        results = self.record()["results"]
        self.assertEqual(results["tick"], {"unit": "ms", "values": [30.3, 30.1, 30.5],
                                           "median": 30.3})
        self.assertEqual(results["tick/scrum/seek"]["values"], [11.0, 12.0, 13.0])
        self.assertEqual(results["tick/scrum/seek"]["median"], 12.0)

    def test_the_reference_is_measured_the_same_and_the_ratio_is_head_over_it(self):
        reference = self.record()["reference"]
        self.assertEqual(reference["commit"], context()["reference_commit"])
        self.assertEqual(reference["results"]["tick"]["median"], 33.4)
        self.assertAlmostEqual(reference["ratio"]["tick"], 30.3 / 33.4, places=4)
        self.assertAlmostEqual(reference["ratio"]["tick/scrum/seek"], 12.0 / 13.0, places=4)

    def test_a_phase_under_the_printed_resolution_is_a_gap_not_zero(self):
        record = self.record(head=rounds("30.3", "30.1", "30.5", regroup="0.00"))
        gap = {"na": "not-measured"}
        self.assertEqual(record["results"]["tick/rout/regroups-search"],
                         {"unit": "ms", "values": gap, "median": gap})
        self.assertNotIn("tick/rout/regroups-search", record["reference"]["ratio"])

    @unittest.skipUnless(HAVE_JSONSCHEMA, "jsonschema isn't installed")
    def test_the_written_record_validates(self):
        path = bench_record.write_record(self.record(), self.out, "2026-11-07", 42412663531, 1)
        self.assertEqual(path, self.out / "metrics" / "bench" / "2026-11-07-42412663531.json")
        self.assertEqual(check_records.check_file(path, self.out / "metrics", SCHEMA_DIR, {}), [])
        self.assertEqual(json.loads(path.read_text(encoding="utf-8"))["series"], "bench")

    def test_a_rerun_gets_its_own_name(self):
        path = bench_record.write_record(self.record(), self.out, "2026-11-07", 42412663531, 2)
        self.assertEqual(path.name, "2026-11-07-42412663531-2.json")


BUSY = """series: 3 ticks from tick 98 (ms)       mean     95th     99th    worst  at tick
  tick                                {tick}    31.00    31.00    31.00       98
inspect: 98,99,100
busy threads (CPU time over wall time):
  tick                                    2.10 threads
  scrum: seek                             {seek_busy} threads
  deaths (bury)                           1.02 threads
  rout: regroup's search                  0.99 threads
  scrum: engage, prepare, stance          1.00 threads
"""


def busy_rounds(*seek_busy):
    return [ROUND.format(tick="30.3", seek=11.0, regroup="0.07")
            + BUSY.format(tick="30.30", seek_busy=b) for b in seek_busy]


class BusyThreadsTest(unittest.TestCase):
    """Spec 50 step 3: each phase's busy threads (CPU time over wall time), where the binary
    printed them, as `busy/<key>` in threads; none from an older binary or Windows."""

    def setUp(self):
        self.out = Path(tempfile.mkdtemp())

    def test_each_phase_has_its_busy_threads_in_threads(self):
        results = bench_record.build_record(busy_rounds("3.10", "3.30", "3.20"), [],
                                            context())["results"]
        self.assertEqual(results["busy/scrum/seek"],
                         {"unit": "threads", "values": [3.1, 3.3, 3.2], "median": 3.2})
        self.assertEqual(results["busy/tick"]["median"], 2.1)
        self.assertEqual(results["busy/rout/regroups-search"]["unit"], "threads")
        self.assertEqual(results["tick/scrum/seek"]["unit"], "ms")

    @unittest.skipUnless(HAVE_JSONSCHEMA, "jsonschema isn't installed")
    def test_the_record_with_busy_threads_validates(self):
        record = bench_record.build_record(busy_rounds("3.10", "3.30", "3.20"),
                                           rounds("33.4", "33.3", "33.7"), context())
        self.assertNotIn("busy/scrum/seek", record["reference"]["results"])
        path = bench_record.write_record(record, self.out, "2026-11-12", 43300000000, 1)
        self.assertEqual(check_records.check_file(path, self.out / "metrics", SCHEMA_DIR, {}), [])

    @unittest.skipUnless(HAVE_JSONSCHEMA, "jsonschema isn't installed")
    def test_output_without_busy_threads_has_no_busy_keys_and_validates(self):
        record = bench_record.build_record(rounds("30.3", "30.1", "30.5"), [], context())
        self.assertFalse([k for k in record["results"] if k.startswith("busy/")])
        path = bench_record.write_record(record, self.out, "2026-11-12", 43300000001, 1)
        self.assertEqual(check_records.check_file(path, self.out / "metrics", SCHEMA_DIR, {}), [])


class RepeatRecordTest(unittest.TestCase):
    """Spec 47 step 5a: a bench-inspect job's repeats of one tick become an `inspect` record."""

    def setUp(self):
        self.out = Path(tempfile.mkdtemp())

    def record(self):
        return bench_record.build_record(rounds("30.3", "30.1", "30.5"),
                                         rounds("33.4", "33.3", "33.7"), context(),
                                         series="inspect")

    def test_its_series_is_inspect(self):
        self.assertEqual(self.record()["series"], "inspect")

    @unittest.skipUnless(HAVE_JSONSCHEMA, "jsonschema isn't installed")
    def test_it_is_written_under_inspect_numbered_by_job_and_validates(self):
        record = self.record()
        record["source"].update(harness="tick_phases-repeat",
                                args=["preset=25k", "at=91", "repeat=50"])
        path = bench_record.write_record(record, self.out, "2026-11-11", 43071105924, 1,
                                         number=3, series="inspect")
        self.assertEqual(path, self.out / "metrics" / "inspect" / "2026-11-11-43071105924-3.json")
        self.assertEqual(check_records.check_file(path, self.out / "metrics", SCHEMA_DIR, {}), [])

    def test_the_harness_is_named_by_the_args(self):
        self.assertEqual(bench_record.harness_of(["preset=25k", "at=91", "repeat=50"]),
                         "tick_phases-repeat")
        self.assertEqual(bench_record.harness_of(["side=4096", "squads=4", "fight=3"]),
                         "breach_tick_phases")


SERIES_ROUND = """machine: AMD EPYC 7763 64-Core Processor, 4 threads, Linux
rust, 24576 a side, 32 wide: {tick} ms a tick
  scrum: seek                             {seek}
  deaths (bury)                            0.24
series: 120 ticks from tick 9 (ms)     mean      95th     99th    worst  at tick
  tick                                {tick}   {p95}   {p99}   99.00       40
  scrum: seek                             {seek}    20.00    25.00    30.00       41
"""


def series_rounds(*rows):
    """One `series=1` round per (mean, p95, p99)."""
    return [SERIES_ROUND.format(tick=t, p95=p95, p99=p99, seek=f"{10.0 + i:.2f}")
            for i, (t, p95, p99) in enumerate(rows)]


class SeriesRecordTest(unittest.TestCase):
    """Step 8a: a `series=1` shape's record holds the series' mean tick as `tick`, its 95th and
    99th as `tick-p95` and `tick-p99`, each with its rounds and their median; a shape timed
    without the bridge has no reference."""

    def setUp(self):
        self.out = Path(tempfile.mkdtemp())
        self.head = series_rounds(("40.10", "52.00", "61.00"), ("40.30", "51.00", "63.00"),
                                  ("40.20", "53.00", "62.00"))

    def test_the_mean_and_the_percentiles_have_their_rounds_and_median(self):
        results = bench_record.build_record(self.head, [], context())["results"]
        self.assertEqual(results["tick"], {"unit": "ms", "values": [40.1, 40.3, 40.2],
                                           "median": 40.2})
        self.assertEqual(results["tick-p95"], {"unit": "ms", "values": [52.0, 51.0, 53.0],
                                               "median": 52.0})
        self.assertEqual(results["tick-p99"], {"unit": "ms", "values": [61.0, 63.0, 62.0],
                                               "median": 62.0})
        self.assertEqual(results["tick/scrum/seek"]["values"], [10.0, 11.0, 12.0])

    def test_a_shape_without_the_bridge_has_no_reference(self):
        self.assertNotIn("reference", bench_record.build_record(self.head, [], context()))

    def test_a_check_beside_the_bridge_has_its_ratio(self):
        bridge = series_rounds(("80.40", "100.00", "120.00"), ("80.20", "104.00", "126.00"),
                               ("80.60", "102.00", "124.00"))
        reference = bench_record.build_record(self.head, bridge, context())["reference"]
        self.assertAlmostEqual(reference["ratio"]["tick"], 40.2 / 80.4, places=4)
        self.assertAlmostEqual(reference["ratio"]["tick-p95"], 52.0 / 102.0, places=4)

    @unittest.skipUnless(HAVE_JSONSCHEMA, "jsonschema isn't installed")
    def test_the_set_s_records_validate_and_are_numbered_apart(self):
        record = bench_record.build_record(self.head, [], context())
        path = bench_record.write_record(record, self.out, "2026-11-11", 42412663531, 1, 101)
        self.assertEqual(path.name, "2026-11-11-42412663531-101.json")
        self.assertEqual(check_records.check_file(path, self.out / "metrics", SCHEMA_DIR, {}), [])


class CommitTest(unittest.TestCase):
    def test_the_pr_and_the_outcomes_trailer_are_read_from_the_message(self):
        self.assertEqual(bench_record.pr_number("fix(sim): x (#775)"), 775)
        self.assertEqual(bench_record.pr_number("fix(sim): x"), {"na": "not-recorded"})
        self.assertEqual(bench_record.title("fix(sim): x (#775)"), "fix(sim): x")
        self.assertTrue(bench_record.outcomes_changed("s\n\nOutcomes-changed: Decision 161, y\n"))
        self.assertFalse(bench_record.outcomes_changed("s\n\nbody\n"))


class FirstParentTest(unittest.TestCase):
    """A run of a commit on main's first-parent line writes a record; a branch's writes none."""

    def setUp(self):
        self.repo = Path(tempfile.mkdtemp())
        git(self.repo, "init", "-q", "-b", "main")
        git(self.repo, "config", "user.email", "t@example.com")
        git(self.repo, "config", "user.name", "t")
        git(self.repo, "commit", "-q", "--allow-empty", "-m", "root")
        git(self.repo, "checkout", "-q", "-b", "feature/x")
        git(self.repo, "commit", "-q", "--allow-empty", "-m", "on the branch")
        self.branch = git(self.repo, "rev-parse", "HEAD")
        git(self.repo, "checkout", "-q", "main")
        git(self.repo, "merge", "-q", "--no-ff", "-m", "merge (#1)", "feature/x")
        self.merge = git(self.repo, "rev-parse", "HEAD")

    def test_a_merge_on_main_is_on_the_line_and_a_branch_commit_is_not(self):
        self.assertTrue(bench_record.on_first_parent(self.merge, "main", self.repo))
        self.assertFalse(bench_record.on_first_parent(self.branch, "main", self.repo))

    def test_its_position_counts_the_first_parent_line(self):
        self.assertEqual(bench_record.position(self.merge, self.repo), 2)


if __name__ == "__main__":
    unittest.main()
