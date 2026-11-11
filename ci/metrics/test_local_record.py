"""Tests for local_record (spec 48 step 1, Decision 178): the owner's PC's
`breach_tick_phases` outputs, named `s<side>_q<squads>_r<round>.txt`, become one record per
battle (cell) under `metrics/local/`, in a series of their own that CI's charts don't draw."""
import json
import os
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


if __name__ == "__main__":
    unittest.main()
