"""Tests for harvest (spec 43 step 6, Decision 162): the archived logs under metrics/raw/
become records marked `harvested`: each bench SUMMARY's head, its phases and medians, tool
and args (machine and reference not recorded), and each scaling_check section's timings as
a `scaling` series. A second run over the same archive writes nothing new."""
import json
import os
import sys
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import check_records  # noqa: E402
import harvest  # noqa: E402

SCHEMA_DIR = HERE.parent.parent / "metrics" / "schema"
try:
    import jsonschema  # noqa: F401
    HAVE_JSONSCHEMA = True
except ImportError:
    HAVE_JSONSCHEMA = os.environ.get("METRICS_REQUIRE_JSONSCHEMA") == "1"

STAMP = "2026-11-07T02:33:21.5649058Z "

ROUNDS_SUMMARY = """##[group]Run here=.; if [ -n "$HEAD_REF" ]; then here=head; fi
\x1b[36;1m  echo "SUMMARY BEGIN"\x1b[0m
##[endgroup]
SUMMARY BEGIN
== feature/spec-41-step-4-crate-habits (05af95e): breach_tick_phases preset=4096
== base main (92909e1): breach_tick_phases preset=4096
medians over 5 rounds, head and base in turn (ms; * = ranges apart):
  phase                                    base     head   change        base range        head range
  tick                                    33.40    30.30    -9.3%       33.30-33.70       29.90-30.40  *
  scrum: seek                             11.08    10.54    -4.9%       11.07-11.24       10.41-10.62  *
  rout: listing                            0.03     0.00   -66.7%         0.02-0.03         0.00-0.01  *
== round 1, feature/spec-41-step-4-crate-habits
rust, 4096 a side, 32 wide: 30.4 ms a tick
  scrum: seek                             10.50
== round 1, base main
rust, 4096 a side, 32 wide: 33.3 ms a tick
SUMMARY END
"""

ONE_ROUND_SUMMARY = """SUMMARY BEGIN
== main (a511f84): tick_phases engine=rust side=4096 squads=4 fight=3
machine: AMD EPYC 7763 64-Core Processor, 4 threads, Linux
rust, 4096 a side, 32 wide: 314.3 ms a tick
  deaths (bury)                       25.18
  scrum: seek                         31.65
SUMMARY END
"""

FAILED_SUMMARY = """SUMMARY BEGIN
== main (a511f84): breach_scaling stages=fight
SUMMARY END
"""

SCALING = """\x1b[36;1m  --script res://tools/scaling_check.gd -- stages=fight 2>/dev/null \\\x1b[0m
Godot Engine v4.7.2.stable.official.ed1daf0bf - https://godotengine.org
scaling_check: ms a tick, and its growth per doubling
Initialize godot-rust (API v4.7.stable.official)
  fight    128 a side      1.98 ms
  fight    256 a side      3.74 ms  x1.89
  fight    512 a side      7.59 ms  x2.03
##[group]Run actions/upload-artifact@v7
"""


def stamped(text):
    return "".join(STAMP + line + "\n" for line in text.splitlines())


def run_json(run_id, branch, sha, inputs=None, created="2026-11-07T02:23:11Z"):
    record = {"workflow": "bench.yml",
              "run": {"id": run_id, "created_at": created, "head_sha": sha,
                      "head_branch": branch, "display_title": "Bench",
                      "html_url": f"https://github.com/o/r/actions/runs/{run_id}"},
              "jobs": [], "log": {"file": f"{run_id}.log", "jobs": ["bench"]}}
    if inputs is not None:
        record["inputs_from_log"] = inputs
    return record


def archive(root):
    bench, sim = root / "bench", root / "sim-checks"
    bench.mkdir(parents=True)
    sim.mkdir(parents=True)
    (bench / "101.log").write_text("===== job: bench\n" + stamped(ROUNDS_SUMMARY))
    (bench / "101.json").write_text(json.dumps(run_json(
        101, "feature/x", "0" * 40, {"TOOL": "breach_tick_phases", "ARGS": "preset=4096",
                                    "ROUNDS": "5", "BASE": "main"})))
    (bench / "102.log").write_text("===== job: bench\n" + stamped(ONE_ROUND_SUMMARY))
    (bench / "102.json").write_text(json.dumps(run_json(
        102, "main", "1" * 40, {"TOOL": "tick_phases", "ROUNDS": "1"},
        created="2026-10-20T10:00:00Z")))
    (bench / "103.log").write_text("===== job: bench\n" + stamped(FAILED_SUMMARY))
    (bench / "103.json").write_text(json.dumps(run_json(103, "main", "2" * 40, {})))
    (bench / "104.json").write_text(json.dumps({**run_json(104, "main", "3" * 40),
                                                "log": {"na": "not-recorded"}}))
    (sim / "201.log").write_text("===== job: scaling (fight)\n" + stamped(SCALING))
    (sim / "201.json").write_text(json.dumps({**run_json(201, "feature/y", "4" * 40),
                                              "workflow": "sim-checks.yml"}))


def files(root):
    return sorted(str(p.relative_to(root)) for p in Path(root).rglob("*.json"))


class HarvestTest(unittest.TestCase):
    def setUp(self):
        self.raw = Path(tempfile.mkdtemp()) / "raw"
        self.out = Path(tempfile.mkdtemp()) / "metrics"
        archive(self.raw)

    def record(self, name):
        return json.loads((self.out / name).read_text(encoding="utf-8"))

    def test_each_readable_run_becomes_one_record(self):
        harvest.harvest(self.raw, self.out)
        self.assertEqual(files(self.out), ["bench/2026-10-20-102.json",
                                           "bench/2026-11-07-101.json",
                                           "scaling/2026-11-07-201.json"])

    def test_a_second_run_writes_nothing_new(self):
        first = harvest.harvest(self.raw, self.out)
        before = {name: (self.out / name).read_bytes() for name in files(self.out)}
        second = harvest.harvest(self.raw, self.out)
        self.assertEqual(len(first), 3)
        self.assertEqual(second, [])
        self.assertEqual({name: (self.out / name).read_bytes() for name in files(self.out)},
                         before)

    def test_a_rounds_summary_gives_the_head_s_medians(self):
        harvest.harvest(self.raw, self.out)
        record = self.record("bench/2026-11-07-101.json")
        not_recorded = {"na": "not-recorded"}
        self.assertEqual(record["results"]["tick"],
                         {"unit": "ms", "values": not_recorded, "median": 30.3})
        self.assertEqual(record["results"]["tick/scrum/seek"]["median"], 10.54)
        self.assertEqual(record["results"]["tick/rout/listing"]["median"],
                         {"na": "not-measured"})
        self.assertEqual(record["source"], {"harness": "breach_tick_phases", "args": ["preset=4096"],
                                            "rounds": 5})
        self.assertEqual(record["commit"]["sha"], "05af95e")
        self.assertEqual(record["commit"]["branch"], "feature/spec-41-step-4-crate-habits")
        self.assertEqual(record["commit"]["base"], "92909e1")
        self.assertEqual(record["machine"], not_recorded)
        self.assertEqual(record["reference"], not_recorded)
        self.assertEqual(record["provenance"]["kind"], "harvested")
        self.assertEqual(record["provenance"]["run_id"], 101)

    def test_a_one_round_summary_gives_its_values(self):
        harvest.harvest(self.raw, self.out)
        record = self.record("bench/2026-10-20-102.json")
        self.assertEqual(record["results"]["tick"], {"unit": "ms", "values": [314.3],
                                                     "median": 314.3})
        self.assertEqual(record["results"]["tick/deaths-bury"]["median"], 25.18)
        self.assertEqual(record["source"]["harness"], "tick_phases.gd")
        self.assertEqual(record["commit"]["base"], {"na": "not-applicable"})

    def test_scaling_sections_become_the_scaling_series(self):
        harvest.harvest(self.raw, self.out)
        record = self.record("scaling/2026-11-07-201.json")
        self.assertEqual(record["series"], "scaling")
        self.assertEqual(record["results"]["scaling/fight/256"],
                         {"unit": "ms", "values": [3.74], "median": 3.74})
        self.assertEqual(record["commit"]["sha"], "4" * 40)
        self.assertEqual(record["commit"]["branch"], "feature/y")
        self.assertEqual(record["source"]["harness"], "scaling_check")

    def test_a_run_with_a_fresh_record_is_left_alone(self):
        fresh = self.out / "bench" / "2026-11-07-101.json"
        fresh.parent.mkdir(parents=True)
        fresh.write_text(json.dumps({"schema": "record-v1",
                                     "provenance": {"run_id": 101, "kind": "fresh"}}))
        before = fresh.read_bytes()
        written = harvest.harvest(self.raw, self.out)
        self.assertEqual(fresh.read_bytes(), before)
        self.assertNotIn(fresh, written)

    @unittest.skipUnless(HAVE_JSONSCHEMA, "jsonschema isn't installed")
    def test_the_records_validate(self):
        harvest.harvest(self.raw, self.out)
        self.assertEqual(check_records.check_tree(self.out, SCHEMA_DIR), [])


if __name__ == "__main__":
    unittest.main()
