"""Tests for stage_data (spec 43 step 3, Decision 164): the data-PR route copies a folder of
new files into the checkout, and refuses the whole folder, copying nothing, when any path in
it leaves metrics/ or lies under metrics/schema/."""
import os
import sys
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import stage_data  # noqa: E402


def write(root, relative, text="{}"):
    path = Path(root) / relative
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")
    return path


class StageDataTest(unittest.TestCase):
    def setUp(self):
        self._dirs = [tempfile.TemporaryDirectory() for _ in range(2)]
        self.folder, self.repo = (Path(d.name) for d in self._dirs)

    def tearDown(self):
        for d in self._dirs:
            d.cleanup()

    def repo_files(self):
        return sorted(str(p.relative_to(self.repo)) for p in self.repo.rglob("*") if p.is_file())

    def test_copies_records_and_raw_material_under_metrics(self):
        write(self.folder, "metrics/bench/2026-11-07-19203344.json", '{"schema": "record-v1"}')
        write(self.folder, "metrics/raw/bench/19203344.log", "log")
        copied = stage_data.stage(self.folder, self.repo)
        self.assertEqual(
            copied, ["metrics/bench/2026-11-07-19203344.json", "metrics/raw/bench/19203344.log"]
        )
        self.assertEqual(copied, self.repo_files())
        self.assertEqual(
            (self.repo / copied[0]).read_text(encoding="utf-8"), '{"schema": "record-v1"}'
        )

    def test_a_path_outside_metrics_refuses_the_folder_and_copies_nothing(self):
        write(self.folder, "metrics/bench/2026-11-07-1.json")
        write(self.folder, "sim/skirmish/formation/unit_store.gd", "extends Node")
        with self.assertRaises(stage_data.Refused) as refused:
            stage_data.stage(self.folder, self.repo)
        self.assertIn("sim/skirmish/formation/unit_store.gd", str(refused.exception))
        self.assertEqual([], self.repo_files())

    def test_a_path_under_the_schema_is_refused(self):
        write(self.folder, "metrics/schema/record-v2.schema.json")
        with self.assertRaises(stage_data.Refused) as refused:
            stage_data.stage(self.folder, self.repo)
        self.assertIn("metrics/schema/record-v2.schema.json", str(refused.exception))
        self.assertEqual([], self.repo_files())

    def test_a_link_is_refused_even_inside_metrics(self):
        target = write(self.repo, "README.md", "readme")
        link = self.folder / "metrics" / "bench" / "2026-11-07-1.json"
        link.parent.mkdir(parents=True)
        os.symlink(target, link)
        with self.assertRaises(stage_data.Refused) as refused:
            stage_data.stage(self.folder, self.repo)
        self.assertIn("metrics/bench/2026-11-07-1.json", str(refused.exception))
        self.assertEqual(["README.md"], self.repo_files())

    def test_an_empty_folder_copies_nothing(self):
        self.assertEqual([], stage_data.stage(self.folder, self.repo))

    def test_main_fails_on_a_refusal(self):
        write(self.folder, "README.md")
        self.assertEqual(1, stage_data.main(["stage_data.py", str(self.folder), str(self.repo)]))


if __name__ == "__main__":
    unittest.main()
