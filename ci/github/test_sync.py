"""Tests for sync.py: the labels and milestones the tree implies (spec 36 step 1), and the
step issues and a spec PR's milestone (spec 40 step 1)."""
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


FIXTURES = Path(__file__).resolve().parent / "fixtures" / "specs"
REPO_URL = "https://github.com/o/r"


def _quiet(function, *args):
    with redirect_stdout(io.StringIO()) as out:
        function(*args)
    return out.getvalue()


class StepPlanTest(unittest.TestCase):
    def setUp(self):
        self.plan = sync.plan(FIXTURES, REPO_URL)

    def spec(self, number):
        return next(spec for spec in self.plan.specs if spec.number == number)

    def test_a_spec_plans_a_parent_and_a_step_per_id(self):
        spec = self.spec("38")
        self.assertEqual(spec.parent.title, "Spec 38: Tick performance after the unit store")
        step = next(s for s in spec.steps if s.step == "1a")
        self.assertEqual(step.title, "Spec 38 step 1a: groups bounded")
        self.assertEqual(step.labels, ["step:38-1a"])
        self.assertEqual([s.step for s in spec.steps].count("1a"), 1)

    def test_headings_outside_steps_still_count(self):
        ids = [s.step for s in self.spec("35").steps]
        self.assertEqual(len(ids), 20)
        self.assertEqual((ids[0], ids[-1]), ("0", "7c"))

    def test_a_heading_without_a_title(self):
        step = next(s for s in self.spec("37").steps if s.step == "4")
        self.assertEqual(step.title, "Spec 37 step 4")

    def test_bodies_point_at_the_spec_and_carry_a_marker(self):
        spec = self.spec("38")
        self.assertIn("<!-- github-sync: spec 38 -->", spec.parent.body)
        self.assertIn(f"{REPO_URL}/blob/main/specs/38-tick-performance.md", spec.parent.body)
        step = spec.steps[0]
        self.assertIn("<!-- github-sync: step 38-1a -->", step.body)
        self.assertIn("### Step 1a, groups bounded", step.body)
        self.assertIn(f"{REPO_URL}/blob/main/specs/38-tick-performance.md", step.body)


def _fake_from(plan):
    return FakeClient(labels=[name for name, _, _ in plan.labels], milestones=plan.milestones)


class StepApplyTest(unittest.TestCase):
    def setUp(self):
        self._dir = tempfile.TemporaryDirectory()
        self.specs = Path(self._dir.name)
        self.write("37-outcome-trace.md", (FIXTURES / "37-outcome-trace.md").read_text(encoding="utf-8"))

    def tearDown(self):
        self._dir.cleanup()

    def write(self, name, text):
        (self.specs / name).write_text(text, encoding="utf-8")

    def plan(self):
        return sync.plan(self.specs, REPO_URL)

    def test_steps_are_created_as_sub_issues_of_the_parent_in_the_milestone(self):
        plan = self.plan()
        client = _fake_from(plan)
        _quiet(sync.apply, client, plan)
        milestone = client.milestones["Spec 37: The outcome trace"]
        titles = [i["title"] for i in client.created_issues]
        self.assertEqual(
            titles,
            ["Spec 37: The outcome trace"] + [f"Spec 37 step {n}" for n in "1234"],
        )
        self.assertTrue(all(i["milestone"] == milestone for i in client.created_issues))
        parent = client.number_of("Spec 37: The outcome trace")
        steps = [client.number_of(f"Spec 37 step {n}") for n in "1234"]
        self.assertEqual(client.sub_issues, [(parent, s) for s in steps])
        self.assertEqual(client.records[steps[0]]["labels"], ["step:37-1"])

    def test_a_merged_steps_issue_is_created_closed(self):
        plan = self.plan()
        client = _fake_from(plan)
        client.add_pull_request(50, labels=["step:37-1"], merged=True)
        client.add_pull_request(51, labels=["step:37-2"], merged=False)
        _quiet(sync.apply, client, plan)
        self.assertEqual(client.closed, [(client.number_of("Spec 37 step 1"), "completed")])

    def test_a_second_run_changes_nothing(self):
        plan = self.plan()
        client = _fake_from(plan)
        _quiet(sync.apply, client, plan)
        client.reset_writes()
        _quiet(sync.apply, client, plan)
        self.assertEqual(client.created_issues, [])
        self.assertEqual(client.closed, [])
        self.assertEqual(client.comments, [])
        self.assertEqual(client.sub_issues, [])

    def test_a_split_steps_issue_is_closed_as_not_planned(self):
        self.write("38-tick-performance.md", (FIXTURES / "38-tick-performance.md").read_text(encoding="utf-8"))
        plan = self.plan()
        client = _fake_from(plan)
        old = client.add_issue("Spec 38 step 1", body="x\n<!-- github-sync: step 38-1 -->", labels=["step:38-1"])
        _quiet(sync.apply, client, plan)
        self.assertEqual(client.closed, [(old, "not_planned")])
        self.assertEqual(
            client.comments,
            [
                (
                    old,
                    "No `### Step 1` heading in `specs/38-tick-performance.md` any more; "
                    "the spec's steps are now: 1a, 1b, 1c, 1d, 2, 2b, 3, 4, 5.",
                )
            ],
        )
        for step in ("1a", "1b", "1c", "1d"):
            self.assertIsNotNone(client.number_of(f"Spec 38 step {step}: ", prefix=True))

    def test_the_sync_never_closes_what_isnt_its_own(self):
        self.write("38-tick-performance.md", (FIXTURES / "38-tick-performance.md").read_text(encoding="utf-8"))
        plan = self.plan()
        client = _fake_from(plan)
        client.add_issue("Step 1 by hand", body="no marker", labels=["step:38-1"])
        _quiet(sync.apply, client, plan)
        self.assertEqual(client.closed, [])
        self.assertEqual(client.comments, [])


class OneIssuePerMarkerTest(unittest.TestCase):
    """Spec 40 step 4: the run logs what it read, reads the milestone before creating, and
    keeps the lowest-numbered issue per marker."""

    def setUp(self):
        self._dir = tempfile.TemporaryDirectory()
        self.specs = Path(self._dir.name)
        text = (FIXTURES / "37-outcome-trace.md").read_text(encoding="utf-8")
        (self.specs / "37-outcome-trace.md").write_text(text, encoding="utf-8")
        self.plan = sync.plan(self.specs, REPO_URL)
        self.client = _fake_from(self.plan)
        self.milestone = self.client.milestones["Spec 37: The outcome trace"]

    def tearDown(self):
        self._dir.cleanup()

    def add_marked(self, number, marker, hidden=False, parent=None):
        self.client.add_issue(
            marker, body=f"x\n<!-- github-sync: {marker} -->", number=number, milestone=self.milestone, hidden=hidden
        )
        if parent is not None:
            self.client.parents[number] = parent

    def test_the_run_says_what_it_read_before_any_write(self):
        for number in range(1, 7):
            self.client.add_issue(f"plain {number}", number=number)
        for number, marker in enumerate(["spec 37", "step 37-1", "step 37-2", "step 37-3"], 7):
            self.add_marked(number, marker)
        out = _quiet(sync.apply, self.client, self.plan)
        self.assertEqual(out.splitlines()[0], "read 10 issues and 0 PRs; 4 with sync markers")
        self.assertEqual([i["title"] for i in self.client.created_issues], ["Spec 37 step 4"])

    def test_an_issue_the_listing_missed_is_found_in_the_milestone(self):
        self.add_marked(300, "spec 37", hidden=True)
        for number, step in enumerate("1234", 301):
            self.add_marked(number, f"step 37-{step}", hidden=True, parent=300)
        _quiet(sync.apply, self.client, self.plan)
        self.assertEqual(self.client.created_issues, [])
        self.assertEqual(self.client.sub_issues, [])

    def test_the_later_copy_is_closed_as_a_duplicate(self):
        self.add_marked(310, "step 37-1")
        self.client.records[310]["state"] = "closed"
        self.add_marked(400, "step 37-1")
        _quiet(sync.apply, self.client, self.plan)
        self.assertEqual(self.client.closed, [(400, "duplicate")])
        self.assertEqual(self.client.comments, [(400, "A copy of #310, which github-sync keeps for `step 37-1`.")])
        self.assertEqual(self.client.records[310]["state"], "closed")
        self.assertNotIn("Spec 37 step 1", [i["title"] for i in self.client.created_issues])

    def test_a_step_made_under_a_copy_moves_to_the_kept_parent(self):
        self.add_marked(300, "spec 37")
        self.add_marked(450, "spec 37")
        self.add_marked(548, "step 37-4", parent=450)
        _quiet(sync.apply, self.client, self.plan)
        self.assertIn((450, "duplicate"), self.client.closed)
        self.assertEqual(self.client.parents[548], 300)
        self.assertIn((300, 548), self.client.sub_issues)
        self.assertEqual(self.client.records[548]["state"], "open")

    def test_a_step_already_under_the_kept_parent_isnt_written(self):
        self.add_marked(300, "spec 37")
        self.add_marked(301, "step 37-1", parent=300)
        _quiet(sync.apply, self.client, self.plan)
        self.assertNotIn(301, [child for _, child in self.client.sub_issues])

    def test_a_second_run_still_changes_nothing(self):
        self.add_marked(310, "step 37-1")
        self.add_marked(400, "step 37-1")
        _quiet(sync.apply, self.client, self.plan)
        self.client.reset_writes()
        _quiet(sync.apply, self.client, self.plan)
        self.assertEqual(self.client.created_issues, [])
        self.assertEqual(self.client.closed, [])
        self.assertEqual(self.client.comments, [])
        self.assertEqual(self.client.sub_issues, [])


class SpecPullRequestTest(unittest.TestCase):
    def setUp(self):
        self._dir = tempfile.TemporaryDirectory()
        self.specs = Path(self._dir.name)
        (self.specs / "41-x.md").write_text("# Spec 41: X\n\n### Step 1, a\n", encoding="utf-8")
        (self.specs / "38-tick-performance.md").write_text("# Spec 38: Tick\n", encoding="utf-8")
        self.plan = sync.plan(self.specs, REPO_URL)

    def tearDown(self):
        self._dir.cleanup()

    def test_a_spec_pr_gets_its_milestone_and_so_do_the_questions_it_closes(self):
        client = FakeClient()
        pr = client.add_pull_request(77, body="Writes spec 41.\n\nCloses #300", files=["specs/41-x.md"])
        question = client.add_issue("Q40. x", number=300)
        _quiet(sync.apply_pull_request, client, self.plan, pr)
        milestone = client.milestones["Spec 41: X"]
        self.assertEqual(client.records[pr]["milestone"], milestone)
        self.assertEqual(client.records[question]["milestone"], milestone)
        self.assertEqual(client.created_issues, [])

    def test_a_set_milestone_is_kept(self):
        client = FakeClient(milestones=["Spec 38: Tick"])
        pr = client.add_pull_request(78, body="Fixes #301 and resolves #302", files=["specs/41-x.md"])
        kept = client.add_issue("Q41. y", number=301, milestone=1)
        other = client.add_issue("Q42. z", number=302)
        _quiet(sync.apply_pull_request, client, self.plan, pr)
        self.assertEqual(client.records[kept]["milestone"], 1)
        self.assertEqual(client.records[other]["milestone"], client.milestones["Spec 41: X"])

    def test_several_specs_use_the_lowest_and_say_so(self):
        client = FakeClient()
        pr = client.add_pull_request(79, files=["specs/41-x.md", "specs/38-tick-performance.md", "README.md"])
        out = _quiet(sync.apply_pull_request, client, self.plan, pr)
        self.assertEqual(client.records[pr]["milestone"], client.milestones["Spec 38: Tick"])
        self.assertIn("Spec 38: Tick", out)
        self.assertIn("41", out)

    def test_a_pr_without_a_spec_changes_nothing(self):
        client = FakeClient()
        pr = client.add_pull_request(80, body="Closes #5", files=["README.md"])
        _quiet(sync.apply_pull_request, client, self.plan, pr)
        self.assertEqual(client.created_milestones, [])
        self.assertIsNone(client.records[pr]["milestone"])

    def test_closing_keywords(self):
        body = "Closes #1, fixes #2\nResolves: #3\nclosed #4. Refs #5. Closes owner/repo#6"
        self.assertEqual(sync.closed_issues(body), [1, 2, 3, 4])


if __name__ == "__main__":
    unittest.main()
