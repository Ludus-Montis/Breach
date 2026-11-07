"""Tests for check_records (spec 43 step 2, Decision 162): every record under metrics/,
apart from metrics/raw/, validates against the schema its `schema` field names; a gap is
never written 0; a file is named <date>-<run id>[-<n>].json."""
import json
import os
import sys
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import check_records  # noqa: E402

REPO = HERE.parent.parent
SCHEMA_DIR = REPO / "metrics" / "schema"

try:
    import jsonschema  # noqa: F401
    HAVE_JSONSCHEMA = True
except ImportError:  # CI installs it (ci.yml, metrics.yml); a bare local Python may not
    HAVE_JSONSCHEMA = os.environ.get("METRICS_REQUIRE_JSONSCHEMA") == "1"


def good_record():
    return {
        "schema": "record-v1",
        "series": "bench",
        "commit": {"sha": "95d592e", "position": 812, "pr": 777, "title": "t",
                   "summary": "s", "step": "step:43-1", "outcomes_changed": False},
        "source": {"harness": "breach_tick_phases", "blob": "abc", "args": ["--ticks", "200"],
                   "rounds": 5, "profile": "release"},
        "machine": {"runner_image": "ubuntu-24.04", "runner_version": "20261101.1",
                    "cpu": "AMD EPYC 7763", "cores": 4, "ram_mb": 15990, "os": "Ubuntu 24.04",
                    "godot": {"na": "not-applicable"}, "rustc": "1.92.0"},
        "results": {
            "tick": {"unit": "ms", "values": [12.1, 12.3, 12.0], "median": 12.1},
            "tick/scrum/seek": {"unit": "ms", "values": {"na": "not-measured"},
                                "median": {"na": "not-measured"}},
        },
        "reference": {"commit": "8ca4227",
                      "results": {"tick": {"unit": "ms", "values": [10.0], "median": 10.0}},
                      "ratio": {"tick": 1.21}},
        "provenance": {"run_id": 19203344, "url": "https://example.invalid/run",
                       "date": "2026-11-07", "kind": "fresh"},
    }


@unittest.skipUnless(HAVE_JSONSCHEMA, "jsonschema is not installed")
class CheckRecordsTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name) / "metrics"
        (self.root / "bench").mkdir(parents=True)

    def tearDown(self):
        self.tmp.cleanup()

    def write(self, name, record, series="bench"):
        path = self.root / series / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(record), encoding="utf-8")
        return path

    def problems(self):
        return check_records.check_tree(self.root, SCHEMA_DIR)

    def test_a_good_record_passes(self):
        self.write("2026-11-07-19203344.json", good_record())
        self.write("2026-11-07-19203344-2.json", good_record())
        self.assertEqual(self.problems(), [])

    def test_every_field_is_optional_but_the_version(self):
        self.write("2026-11-07-1.json", {"schema": "record-v1"})
        self.assertEqual(self.problems(), [])

    def test_a_gap_written_zero_fails_and_names_the_file_and_field(self):
        record = good_record()
        record["results"]["tick/scrum"] = {"unit": "ms", "values": [0, 0], "median": 0}
        self.write("2026-11-07-1.json", record)
        problems = self.problems()
        self.assertTrue(problems)
        self.assertTrue(all("2026-11-07-1.json" in p for p in problems), problems)
        self.assertTrue(any("results/tick/scrum" in p and "median" in p for p in problems),
                        problems)
        self.assertTrue(any("results/tick/scrum" in p and "values" in p for p in problems),
                        problems)

    def test_a_zero_in_the_reference_fails_too(self):
        record = good_record()
        record["reference"]["results"]["tick"]["median"] = 0
        self.write("2026-11-07-1.json", record)
        self.assertTrue(any("reference" in p for p in self.problems()))

    def test_an_empty_values_list_fails(self):
        record = good_record()
        record["results"]["tick"]["values"] = []
        self.write("2026-11-07-1.json", record)
        self.assertTrue(self.problems())

    def test_a_phase_path_with_a_capital_fails_and_names_the_file_and_field(self):
        record = good_record()
        record["results"]["tick/Scrum"] = {"unit": "ms", "values": [1.0], "median": 1.0}
        self.write("2026-11-07-1.json", record)
        problems = self.problems()
        self.assertTrue(problems)
        self.assertTrue(any("2026-11-07-1.json" in p and "tick/Scrum" in p for p in problems),
                        problems)

    def test_an_unknown_gap_reason_fails(self):
        record = good_record()
        record["machine"]["cpu"] = {"na": "forgot"}
        self.write("2026-11-07-1.json", record)
        self.assertTrue(any("machine" in p for p in self.problems()))

    def test_an_unknown_schema_version_fails(self):
        record = good_record()
        record["schema"] = "record-v99"
        self.write("2026-11-07-1.json", record)
        problems = self.problems()
        self.assertTrue(any("record-v99" in p for p in problems), problems)

    def test_a_record_without_a_version_fails(self):
        record = good_record()
        del record["schema"]
        self.write("2026-11-07-1.json", record)
        self.assertTrue(any("schema" in p for p in self.problems()))

    def test_a_badly_named_file_fails(self):
        for name in ["19203344.json", "2026-11-7-1.json", "2026-11-07-abc.json",
                     "2026-11-07-1-x.json", "record.json"]:
            path = self.write(name, good_record())
            problems = self.problems()
            self.assertTrue(any(name in p and "name" in p for p in problems), (name, problems))
            path.unlink()

    def test_a_file_that_is_not_json_fails(self):
        (self.root / "bench" / "2026-11-07-1.json").write_text("{", encoding="utf-8")
        self.assertTrue(self.problems())

    def test_raw_material_and_the_schema_folder_are_not_records(self):
        self.write("19203344.json", {"anything": 0}, series="raw/bench")
        (self.root / "schema").mkdir()
        (self.root / "schema" / "record-v1.schema.json").write_text("{}", encoding="utf-8")
        self.assertEqual(self.problems(), [])

    def test_main_exits_non_zero_on_a_problem(self):
        self.write("bad.json", good_record())
        self.assertEqual(check_records.main([str(self.root), str(SCHEMA_DIR)]), 1)

    def test_main_exits_zero_when_all_pass(self):
        self.write("2026-11-07-1.json", good_record())
        self.assertEqual(check_records.main([str(self.root), str(SCHEMA_DIR)]), 0)


if __name__ == "__main__":
    unittest.main()
