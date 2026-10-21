"""Tests for sync.py (spec 36 step 1): the labels and milestones the tree implies."""
import io
import sys
import tempfile
import unittest
from contextlib import redirect_stdout
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import labels  # noqa: E402
import sync  # noqa: E402
from fake_client import FakeClient  # noqa: E402

SPEC_35 = """---
spec_type: code
---

# Spec 35: The unit store

### Step 1a, `position`

### Step 7b, the downed's fields

### Step 1, a plain one
"""

SPEC_36 = """# Spec 36: GitHub issues for open questions

### Step 1

### Step 2
"""


class SyncTest(unittest.TestCase):
    def setUp(self):
        self._dir = tempfile.TemporaryDirectory()
        self.specs = Path(self._dir.name)
        (self.specs / "35-unit-store.md").write_text(SPEC_35, encoding="utf-8")
        (self.specs / "36-github-issues-for-questions.md").write_text(SPEC_36, encoding="utf-8")
        self.plan = sync.plan(self.specs)

    def tearDown(self):
        self._dir.cleanup()

    def label_names(self):
        return [name for name, _, _ in self.plan.labels]

    def test_plan_names_labels_and_milestones_the_tree_implies(self):
        names = self.label_names()
        for name, _, _ in labels.FIXED:
            self.assertIn(name, names)
        self.assertIn("Spec 35: The unit store", self.plan.milestones)
        self.assertIn("Spec 36: GitHub issues for open questions", self.plan.milestones)
        self.assertIn("step:35-1a", names)
        self.assertIn("step:35-7b", names)
        self.assertIn("raise-when:spec-35", names)

    def test_two_specs_steps_of_the_same_name_get_two_labels(self):
        names = self.label_names()
        self.assertIn("step:35-1", names)
        self.assertIn("step:36-1", names)
        self.assertNotIn("step:1", names)
        self.assertEqual(len(names), len(set(names)))

    def test_a_spec_without_a_spec_heading_takes_its_first_heading(self):
        (self.specs / "01-clock.md").write_text("# Simulation Clock\n", encoding="utf-8")
        self.assertIn("Spec 01: Simulation Clock", sync.plan(self.specs).milestones)

    def test_a_second_run_changes_nothing(self):
        client = FakeClient(labels=self.label_names(), milestones=self.plan.milestones)
        with redirect_stdout(io.StringIO()):
            sync.apply(client, self.plan)
        self.assertEqual(client.created_labels, [])
        self.assertEqual(client.created_milestones, [])

    def test_only_what_is_missing_is_created(self):
        client = FakeClient(labels=["question"], milestones=["Spec 35: The unit store"])
        with redirect_stdout(io.StringIO()):
            sync.apply(client, self.plan)
        expected_labels = [name for name in self.label_names() if name != "question"]
        self.assertEqual(client.created_labels, expected_labels)
        self.assertEqual(client.created_milestones, ["Spec 36: GitHub issues for open questions"])


if __name__ == "__main__":
    unittest.main()
