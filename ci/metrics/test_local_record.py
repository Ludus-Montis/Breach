"""Tests for local_record (spec 48 step 1, Decision 178): the owner's PC's
`breach_tick_phases` outputs, named `s<side>_q<squads>_r<round>.txt`, become one record per
battle (cell) under `metrics/local/`, in a series of their own that CI's charts don't draw."""
import json
import os
import re
import shutil
import sys
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import bench_chart  # noqa: E402
import check_records  # noqa: E402
import local_record  # noqa: E402

FIXTURES = HERE / "fixtures" / "local"
SETTLED = FIXTURES / "settled"
REPEATS = FIXTURES / "repeat"
SCHEMA_DIR = HERE.parent.parent / "metrics" / "schema"
MACHINE = HERE / "local_machine.json"

try:
    import jsonschema  # noqa: F401
    HAVE_JSONSCHEMA = True
except ImportError:  # CI installs it (metrics.yml); a bare local Python may not
    HAVE_JSONSCHEMA = os.environ.get("METRICS_REQUIRE_JSONSCHEMA") == "1"


def context():
    return {"commit": {"sha": "a5f574d52f7772795023e965eb44156ac30d256d"},
            "source": {"harness": "breach_tick_phases", "profile": "release"},
            "date": "2026-11-11"}


class OneCell(unittest.TestCase):
    def setUp(self):
        self.machine = local_record.load_machine(MACHINE)
        self.cells = local_record.cells(FIXTURES, self.machine)
        self.record = local_record.build_record(3072, 1, self.cells[(3072, 1)], self.machine,
                                                context())

    def test_the_cell_is_named_by_its_files(self):
        self.assertEqual(list(self.cells), [(3072, 1)])

    def test_each_phase_holds_its_three_rounds_and_their_median(self):
        results = self.record["results"]
        self.assertEqual(results["tick"]["values"], [13.3, 16.5, 13.7])
        self.assertEqual(results["tick"]["median"], 13.7)
        self.assertEqual(results["tick/scrum/seek"]["values"], [4.80, 6.05, 4.94])
        self.assertEqual(results["tick/scrum/seek"]["median"], 4.94)
        self.assertEqual(results["tick/units-walk-to-places"]["median"], 2.59)

    def test_a_phase_under_the_resolution_is_a_gap(self):
        self.assertEqual(self.record["results"]["tick/orders"]["median"],
                         {"na": "not-measured"})

    def test_it_is_a_local_record_of_the_pc(self):
        record = self.record
        self.assertEqual(record["series"], "local")
        self.assertEqual(record["provenance"]["kind"], "local")
        self.assertEqual(record["provenance"]["date"], "2026-11-11")
        self.assertIn("na", record["provenance"]["run_id"])
        self.assertEqual(record["reference"], {"na": "not-measured"})
        self.assertIn("i7-12700K", record["machine"]["cpu"])
        self.assertEqual(record["source"]["args"], ["side=3072", "squads=1", "fight=10"])
        self.assertEqual(record["source"]["rounds"], 3)

    @unittest.skipUnless(HAVE_JSONSCHEMA, "needs jsonschema (pip install jsonschema)")
    def test_it_passes_check_records(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder) / "metrics"
            shutil.copytree(SCHEMA_DIR, root / "schema")
            local_record.write_records([self.record], root, "2026-11-11")
            self.assertEqual(check_records.check_tree(root, root / "schema"), [])

    def test_ci_charts_do_not_draw_it(self):
        self.assertEqual(bench_chart.points([self.record]), [])


class Refusals(unittest.TestCase):
    def setUp(self):
        self.machine = local_record.load_machine(MACHINE)
        self.folder = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, self.folder)

    def put(self, name, text):
        (self.folder / name).write_text(text, encoding="utf-8")

    def test_a_file_whose_side_differs_from_its_name_is_refused_by_name(self):
        text = (FIXTURES / "s3072_q1_r1.txt").read_text(encoding="utf-8")
        self.put("s6144_q1_r1.txt", text)
        with self.assertRaises(local_record.Refused) as raised:
            local_record.cells(self.folder, self.machine)
        self.assertIn("s6144_q1_r1.txt", str(raised.exception))
        self.assertIn("side", str(raised.exception))

    def test_a_file_from_another_thread_count_is_refused_by_name(self):
        text = (FIXTURES / "s3072_q1_r1.txt").read_text(encoding="utf-8")
        self.put("s3072_q1_r1.txt", text.replace("20 threads", "4 threads"))
        with self.assertRaises(local_record.Refused) as raised:
            local_record.cells(self.folder, self.machine)
        self.assertIn("s3072_q1_r1.txt", str(raised.exception))
        self.assertIn("threads", str(raised.exception))


class Writing(unittest.TestCase):
    def test_records_are_numbered_after_those_already_there_that_day(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            (root / "local").mkdir()
            (root / "local" / "2026-11-11-1.json").write_text("{}", encoding="utf-8")
            paths = local_record.write_records([{"schema": "record-v1"}] * 2, root,
                                               "2026-11-11")
            self.assertEqual([p.name for p in paths], ["2026-11-11-2.json", "2026-11-11-3.json"])
            self.assertEqual(json.loads(paths[0].read_text())["schema"], "record-v1")


class SettledSeries(unittest.TestCase):
    """Spec 48 step 5a: a grid cell run with `series=1` keeps its p95 and p99 over the whole
    series, and the `inspect:` ticks each round named."""

    def setUp(self):
        self.machine = local_record.load_machine(MACHINE)
        texts = local_record.cells(SETTLED, self.machine)[(3072, 1)]
        self.texts = texts
        self.record = local_record.build_record(3072, 1, texts, self.machine, context())

    def test_its_args_say_it_is_the_settled_series(self):
        self.assertEqual(self.record["source"]["args"],
                         ["side=3072", "squads=1", "series=1", "series=settled"])

    def test_it_holds_the_series_p95_and_p99(self):
        results = self.record["results"]
        self.assertEqual(len(results["tick-p95"]["values"]), 2)
        self.assertEqual(len(results["tick-p99"]["values"]), 2)
        self.assertIn("tick", results)

    def test_it_keeps_no_quartiles(self):
        """Spec 48 step 5d: only a repeat record keeps quartiles."""
        self.assertFalse([path for path, m in self.record["results"].items() if "quartiles" in m])

    def test_it_holds_the_inspect_ticks_each_round_named(self):
        inspect = self.record["source"]["inspect"]
        self.assertEqual(len(inspect), 2)
        for ticks in inspect:
            self.assertEqual(ticks, sorted(ticks))
            self.assertTrue(all(isinstance(t, int) for t in ticks))
        self.assertEqual(inspect[0], local_record.inspect_ticks(self.texts[0]))

    @unittest.skipUnless(HAVE_JSONSCHEMA, "needs jsonschema (pip install jsonschema)")
    def test_it_passes_check_records(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder) / "metrics"
            shutil.copytree(SCHEMA_DIR, root / "schema")
            local_record.write_records([self.record], root, "2026-11-11")
            self.assertEqual(check_records.check_tree(root, root / "schema"), [])


class RepeatMode(unittest.TestCase):
    """Spec 48 step 5a: the repeat mode's output (`at=<t>,<t> repeat=<n>`), 2 rounds at 2
    ticks, becomes one record per (battle, tick), each round's median a value."""

    def setUp(self):
        self.machine = local_record.load_machine(MACHINE)
        self.found = local_record.repeats(REPEATS, self.machine)
        self.records = [local_record.build_repeat_record(side, squads, tick, n, texts,
                                                         self.machine, context())
                        for (side, squads, tick), (n, texts) in self.found.items()]

    def test_there_is_a_record_per_tick(self):
        self.assertEqual(sorted(tick for _, _, tick in self.found), [91, 101])
        self.assertEqual(len(self.records), 2)

    def test_each_round_is_a_value_with_their_median(self):
        for record in self.records:
            self.assertEqual(record["source"]["rounds"], 2)
            for path in ("tick", "tick/scrum/seek"):
                measure = record["results"][path]
                self.assertEqual(len(measure["values"]), 2)
                self.assertAlmostEqual(measure["median"], sum(measure["values"]) / 2)

    def test_each_round_keeps_its_quartiles_holding_its_value(self):
        """Spec 48 step 5d: each round's q1 and q3 are that round's pair, beside its median."""
        for record in self.records:
            for path, measure in record["results"].items():
                if "na" in measure["values"]:
                    self.assertNotIn("quartiles", measure, path)
                    continue
                self.assertEqual(len(measure["quartiles"]), 2, path)
                for (q1, q3), value in zip(measure["quartiles"], measure["values"]):
                    self.assertLessEqual(q1, value, path)
                    self.assertLessEqual(value, q3, path)

    def test_the_pairs_are_the_printed_q1_and_q3(self):
        record = next(r for r in self.records if "at=91" in r["source"]["args"])
        self.assertEqual(record["results"]["tick"]["quartiles"][0], [26.08, 31.16])
        self.assertEqual(record["results"]["tick/scrum/seek"]["quartiles"][0], [10.94, 14.74])

    def test_output_with_the_iqr_width_alone_keeps_no_quartiles(self):
        """Output from before step 5d prints `IQR`, one width, so there is no pair to keep."""
        old = [re.sub(r"^(  \S.*?\s+\d+\.\d\d)\s+\d+\.\d\d\s+(\d+\.\d\d)(\s+\[)",
                      lambda m: m.group(1) + "     1.00" + m.group(3), text, flags=re.M)
               .replace("median       q1       q3", "median      IQR")
               for text in next(iter(self.found.values()))[1]]
        record = local_record.build_repeat_record(3072, 1, 91, 50, old, self.machine, context())
        self.assertNotIn("quartiles", record["results"]["tick"])

    def test_the_spread_table_is_not_read_as_phases(self):
        for record in self.records:
            self.assertNotIn("tick/ms", record["results"])
            self.assertNotIn("tick/tick", record["results"])

    def test_it_is_the_repeat_harness_with_at_and_repeat(self):
        record = self.records[0]
        self.assertEqual(record["source"]["harness"], "tick_phases-repeat")
        self.assertEqual(record["source"]["args"],
                         ["side=3072", "squads=1", "at=91", "repeat=50"])
        self.assertEqual(record["series"], "local")

    def test_a_repeat_file_from_another_side_is_refused_by_name(self):
        folder = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, folder)
        text = (REPEATS / "s3072_q1_r1_repeat.txt").read_text(encoding="utf-8")
        (folder / "s6144_q1_r1_repeat.txt").write_text(text, encoding="utf-8")
        with self.assertRaises(local_record.Refused) as raised:
            local_record.repeats(folder, self.machine)
        self.assertIn("s6144_q1_r1_repeat.txt", str(raised.exception))

    @unittest.skipUnless(HAVE_JSONSCHEMA, "needs jsonschema (pip install jsonschema)")
    def test_they_pass_check_records(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder) / "metrics"
            shutil.copytree(SCHEMA_DIR, root / "schema")
            local_record.write_records(self.records, root, "2026-11-11")
            self.assertEqual(check_records.check_tree(root, root / "schema"), [])


class PrintedMachine(unittest.TestCase):
    """Spec 48 step 3's rest, built in step 5a: a machine line that names the CPU (step 3's
    `breach_tick_phases`) is preferred over `local_machine.json`."""

    def setUp(self):
        self.machine = local_record.load_machine(MACHINE)

    def test_a_known_cpu_is_taken_from_the_output(self):
        text = ("machine: 12th Gen Intel(R) Core(TM) i7-12700K, 12 cores, 20 threads, 32 GB, "
                "Windows\n3072 a side, 32 wide: 13.3 ms a tick\n")
        found = local_record.machine_of([text], self.machine)
        self.assertEqual(found["cpu"], "12th Gen Intel(R) Core(TM) i7-12700K")
        self.assertEqual(found["cores"], 12)
        self.assertEqual(found["ram_mb"], 32768)
        self.assertEqual(found["os"], "Windows")

    def test_cores_and_ram_are_optional(self):
        text = "machine: Some CPU, 20 threads, Windows\n"
        found = local_record.machine_of([text], self.machine)
        self.assertEqual(found["cpu"], "Some CPU")
        self.assertEqual(found["cores"], self.machine["machine"]["cores"])

    def test_an_unknown_cpu_keeps_the_machine_file(self):
        text = "machine: unknown, 20 threads, Windows\n"
        self.assertEqual(local_record.machine_of([text], self.machine),
                         self.machine["machine"])


if __name__ == "__main__":
    unittest.main()
