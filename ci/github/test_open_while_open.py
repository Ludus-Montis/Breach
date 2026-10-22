"""Tests for open_while_open.py: a spec's parent and milestone follow their items (spec 40
step 5, Decision 156)."""
import io
import sys
import tempfile
import unittest
from contextlib import redirect_stdout
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import open_while_open  # noqa: E402
import sync  # noqa: E402
from fake_client import FakeClient  # noqa: E402

REPO_URL = "https://github.com/o/r"
SPEC_40 = "# Spec 40: Step issues\n\n### Step 1, a\n\n### Step 2, b\n\n### Step 3, c\n"
SPEC_01 = "# Spec 01: Clock\n"


def _quiet(function, *args):
    with redirect_stdout(io.StringIO()) as out:
        function(*args)
    return out.getvalue()


class RuleTest(unittest.TestCase):
    def test_no_steps_no_change(self):
        self.assertIsNone(open_while_open.parent_state([]))

    def test_open_while_any_step_is(self):
        self.assertEqual(open_while_open.parent_state([("closed", "completed"), ("open", None)]), ("open", None))

    def test_closed_completed_if_any_step_completed(self):
        steps = [("closed", "not_planned"), ("closed", "completed")]
        self.assertEqual(open_while_open.parent_state(steps), ("closed", "completed"))

    def test_a_parent_whose_steps_were_all_dropped_is_not_planned(self):
        steps = [("closed", "not_planned"), ("closed", "duplicate")]
        self.assertEqual(open_while_open.parent_state(steps), ("closed", "not_planned"))

    def test_a_milestone_is_open_while_any_item_is(self):
        self.assertEqual(open_while_open.milestone_state(["closed", "open"]), "open")
        self.assertEqual(open_while_open.milestone_state(["closed"]), "closed")

    def test_only_spec_milestones_are_touched(self):
        self.assertTrue(open_while_open.is_spec_milestone("Spec 37: The outcome trace"))
        self.assertFalse(open_while_open.is_spec_milestone("Gate 2"))


class SyncRunTest(unittest.TestCase):
    def setUp(self):
        self._dir = tempfile.TemporaryDirectory()
        self.specs = Path(self._dir.name)
        (self.specs / "40-step-issues.md").write_text(SPEC_40, encoding="utf-8")
        (self.specs / "01-clock.md").write_text(SPEC_01, encoding="utf-8")
        self.plan = sync.plan(self.specs, REPO_URL)
        self.client = FakeClient(labels=[n for n, _, _ in self.plan.labels], milestones=self.plan.milestones)
        self.milestone = self.client.milestones["Spec 40: Step issues"]

    def tearDown(self):
        self._dir.cleanup()

    def add_marked(self, number, marker, state="open", reason=None, milestone=None, parent=None):
        self.client.add_issue(
            marker, body=f"x\n<!-- github-sync: {marker} -->", number=number, milestone=milestone or self.milestone
        )
        self.client.records[number].update(state=state, state_reason=reason)
        if parent is not None:
            self.client.parents[number] = parent

    def test_a_new_step_reopens_the_parent(self):
        self.add_marked(300, "spec 40", "closed", "completed")
        for number, step in enumerate("12", 301):
            self.add_marked(number, f"step 40-{step}", "closed", "completed", parent=300)
        _quiet(sync.apply, self.client, self.plan)
        step_3 = self.client.number_of("Spec 40 step 3: c")
        self.assertEqual(self.client.parents[step_3], 300)
        self.assertEqual(self.client.reopened, [300])
        self.assertEqual(self.client.records[300]["state"], "open")

    def test_a_parent_with_all_steps_closed_closes_on_the_run(self):
        self.add_marked(300, "spec 40")
        for number, step in enumerate("123", 301):
            self.add_marked(number, f"step 40-{step}", "closed", "completed", parent=300)
        _quiet(sync.apply, self.client, self.plan)
        self.assertIn((300, "completed"), self.client.closed)
        self.assertEqual(self.client.milestone_writes, [(self.milestone, "closed")])

    def test_a_parent_with_no_steps_is_left_alone(self):
        spec_01 = self.client.milestones["Spec 01: Clock"]
        self.add_marked(200, "spec 01", milestone=spec_01)
        _quiet(sync.apply, self.client, self.plan)
        self.assertEqual(self.client.records[200]["state"], "open")
        self.assertNotIn(200, [number for number, _ in self.client.closed])
        self.assertNotIn((spec_01, "closed"), self.client.milestone_writes)

    def test_another_milestone_is_never_touched(self):
        self.client.milestones["Gate 2"] = 99
        self.client.milestone_states[99] = "open"
        _quiet(sync.apply, self.client, self.plan)
        self.assertNotIn(99, [number for number, _ in self.client.milestone_writes])

    def test_a_second_run_changes_nothing(self):
        self.add_marked(300, "spec 40")
        for number, step in enumerate("123", 301):
            self.add_marked(number, f"step 40-{step}", "closed", "completed", parent=300)
        _quiet(sync.apply, self.client, self.plan)
        self.client.reset_writes()
        _quiet(sync.apply, self.client, self.plan)
        self.assertEqual(self.client.closed, [])
        self.assertEqual(self.client.reopened, [])
        self.assertEqual(self.client.milestone_writes, [])
        self.assertEqual(self.client.comments, [])


class IssueEventTest(unittest.TestCase):
    def setUp(self):
        self.client = FakeClient(milestones=["Spec 40: Step issues", "Spec 37: The outcome trace"])
        self.milestone = self.client.milestones["Spec 40: Step issues"]
        self.add_marked(300, "spec 40")
        self.add_marked(301, "step 40-1", "closed", "completed", parent=300)
        self.add_marked(302, "step 40-2", "closed", "completed", parent=300)
        self.add_marked(303, "step 40-3", parent=300)

    def add_marked(self, number, marker, state="open", reason=None, parent=None):
        self.client.add_issue(marker, body=f"x\n<!-- github-sync: {marker} -->", number=number, milestone=self.milestone)
        self.client.records[number].update(state=state, state_reason=reason)
        if parent is not None:
            self.client.parents[number] = parent

    def test_the_last_step_closes_its_parent_as_completed(self):
        self.client.close_issue(303, "completed")
        self.client.reset_writes()
        _quiet(open_while_open.apply_issue_event, self.client, 303)
        self.assertEqual(self.client.closed, [(300, "completed")])
        self.assertEqual(self.client.milestone_writes, [(self.milestone, "closed")])
        self.assertEqual(self.client.comments, [])
        self.assertEqual(self.client.created_issues, [])

    def test_a_step_still_open_leaves_the_parent_open(self):
        self.client.close_issue(302, "completed")
        self.client.reset_writes()
        _quiet(open_while_open.apply_issue_event, self.client, 302)
        self.assertEqual(self.client.closed, [])
        self.assertEqual(self.client.milestone_writes, [])

    def test_the_milestone_closes_with_its_last_item_and_reopens(self):
        spec_37 = self.client.milestones["Spec 37: The outcome trace"]
        question = self.client.add_issue("Q1. x", number=400, milestone=spec_37)
        parent = self.client.add_issue("Spec 37", body="<!-- github-sync: spec 37 -->", number=401, milestone=spec_37)
        self.client.close_issue(question, "completed")
        self.client.close_issue(parent, "completed")
        self.client.reset_writes()
        _quiet(open_while_open.apply_issue_event, self.client, parent)
        self.assertEqual(self.client.milestone_writes, [(spec_37, "closed")])
        self.client.reopen_issue(question)
        self.client.reset_writes()
        _quiet(open_while_open.apply_issue_event, self.client, question)
        self.assertEqual(self.client.milestone_writes, [(spec_37, "open")])

    def test_an_issue_that_isnt_the_syncs_does_nothing(self):
        question = self.client.add_issue("Q2. y", number=500)
        self.client.close_issue(question, "completed")
        self.client.reset_writes()
        _quiet(open_while_open.apply_issue_event, self.client, question)
        self.assertEqual(self.client.closed, [])
        self.assertEqual(self.client.reopened, [])
        self.assertEqual(self.client.milestone_writes, [])


class EventRoutingTest(unittest.TestCase):
    def test_an_issues_event_creates_nothing(self):
        client = FakeClient()
        client.add_issue("Q3. z", number=600)
        _quiet(sync.apply_issue_event, client, 600)
        self.assertEqual(client.created_issues, [])
        self.assertEqual(client.created_labels, [])
        self.assertEqual(client.created_milestones, [])


if __name__ == "__main__":
    unittest.main()
