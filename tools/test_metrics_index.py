"""Tests for tools/metrics_view/build_index.py (spec 43 step 7): every record under metrics/,
apart from metrics/raw/ and metrics/schema/, listed into the one bundle the page loads."""
import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent / "metrics_view"))
import build_index  # noqa: E402


class BuildIndexTest(unittest.TestCase):
    def setUp(self):
        self.root = Path(tempfile.mkdtemp())
        metrics = self.root / "metrics"
        for name, body in [("bench/2026-11-08-2.json", {"schema": "record-v1", "series": "bench"}),
                           ("bench/2026-11-07-1.json", {"schema": "record-v1", "series": "bench"}),
                           ("scaling/2026-11-07-3.json", {"schema": "record-v1"}),
                           ("raw/bench/9.json", {"run": {}}),
                           ("schema/record-v1.schema.json", {"title": "x"})]:
            path = metrics / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(json.dumps(body))

    def test_records_are_listed_with_their_paths_and_raw_and_schema_are_not(self):
        bundle = build_index.bundle(self.root / "metrics")
        self.assertEqual([item["path"] for item in bundle],
                         ["metrics/bench/2026-11-07-1.json", "metrics/bench/2026-11-08-2.json",
                          "metrics/scaling/2026-11-07-3.json"])
        self.assertEqual(bundle[0]["record"]["series"], "bench")

    def test_the_bundle_is_written_where_the_page_reads_it(self):
        out = self.root / "site" / "records.json"
        build_index.main([str(self.root / "metrics"), str(out)])
        self.assertEqual(len(json.loads(out.read_text())), 3)


if __name__ == "__main__":
    unittest.main()
