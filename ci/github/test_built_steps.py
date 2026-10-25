"""Tests for the steps a build records: a `### Step <id>` heading that only the spec's
`## Built` section carries was added by the PR that built that step, so its issue, made
after that PR merged, is closed against it (#604, #598: a split step's issue was left open,
its PR labelled with the parent step's label, not its own, so the label rule missed it)."""
import io
import subprocess
import sys
import tempfile
import unittest
from contextlib import redirect_stdout
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import step_issues  # noqa: E402
import sync  # noqa: E402
from fake_client import FakeClient  # noqa: E402

REPO_URL = "https://github.com/o/r"

SPEC_39 = """# Spec 39: The simulation in Rust

## Steps

### Step 7, morale, the rout and orders

## Built

### Step 7, the first record of it

### Step 7b, the rout in the crate
"""


def _quiet(function, *args, **kwargs):
    with redirect_stdout(io.StringIO()):
        function(*args, **kwargs)


class BuiltStepsTest(unittest.TestCase):
    def setUp(self):
        self._dir = tempfile.TemporaryDirectory()
        self.specs = Path(self._dir.name)
        (self.specs / "39-simulation-in-rust.md").write_text(SPEC_39, encoding="utf-8")
        self.plan = sync.plan(self.specs, REPO_URL)
        self.client = FakeClient(labels=[n for n, _, _ in self.plan.labels], milestones=self.plan.milestones)
        self.asked = []

    def tearDown(self):
        self._dir.cleanup()

    def built_by(self, path, heading):
        self.asked.append((path, heading))
        return 602

    def run_sync(self):
        _quiet(sync.apply, self.client, self.plan, built_by=self.built_by)

    def test_only_a_heading_under_built_alone_is_a_built_step(self):
        spec = self.plan.specs[0]
        built = {step.step: step.built for step in spec.steps}
        self.assertEqual(built, {"7": "", "7b": "### Step 7b, the rout in the crate"})

    def test_a_built_steps_issue_is_closed_against_the_merged_pr_that_built_it(self):
        self.client.add_pull_request(602, labels=["implementation"], merged=True)
        self.run_sync()
        step = self.client.number_of("Spec 39 step 7b: the rout in the crate")
        self.assertEqual(self.client.closed, [(step, "completed")])
        self.assertEqual(self.client.comments, [(step, "Built by #602, which added this step to the spec's Built section.")])
        self.assertEqual(self.asked, [("specs/39-simulation-in-rust.md", "### Step 7b, the rout in the crate")])

    def test_one_already_open_is_closed_too(self):
        self.client.add_pull_request(602, labels=["implementation"], merged=True)
        _quiet(sync.apply, self.client, self.plan, built_by=lambda path, heading: None)
        self.assertEqual(self.client.closed, [])
        self.client.reset_writes()
        self.run_sync()
        step = self.client.number_of("Spec 39 step 7b: the rout in the crate")
        self.assertEqual(self.client.closed, [(step, "completed")])

    def test_it_stays_open_while_the_pr_found_is_not_merged(self):
        self.client.add_pull_request(602, labels=["implementation"], merged=False)
        self.run_sync()
        self.assertEqual(self.client.closed, [])

    def test_a_step_under_steps_is_never_closed_by_its_record(self):
        self.client.add_pull_request(602, labels=["implementation"], merged=True)
        self.run_sync()
        self.assertNotIn(self.client.number_of("Spec 39 step 7: morale, the rout and orders"), [n for n, _ in self.client.closed])
        self.assertTrue(all(heading.startswith("### Step 7b") for _, heading in self.asked))

    def test_a_second_run_changes_nothing(self):
        self.client.add_pull_request(602, labels=["implementation"], merged=True)
        self.run_sync()
        self.client.reset_writes()
        self.run_sync()
        self.assertEqual((self.client.closed, self.client.comments, self.client.created_issues), ([], [], []))


class BuiltByGitTest(unittest.TestCase):
    """The PR whose merge first added a heading, read from git: a squash merge's subject
    ends `(#N)`, a merge commit's starts `Merge pull request #N`."""

    def setUp(self):
        self._dir = tempfile.TemporaryDirectory()
        self.root = Path(self._dir.name)
        self.git("init", "-q")
        self.commit("specs/39-x.md", "# Spec 39\n", "docs: spec 39 (#500)")
        self.commit("specs/39-x.md", "# Spec 39\n\n## Built\n\n### Step 7b, the rout\n", "refactor: the rout (#602)")
        self.commit("specs/39-x.md", "# Spec 39\n\n## Built\n\n### Step 7b, the rout\n\nmore\n", "Merge pull request #610 from o/b")

    def tearDown(self):
        self._dir.cleanup()

    def git(self, *args):
        subprocess.run(["git", "-C", str(self.root), "-c", "user.name=t", "-c", "user.email=t@t", *args], check=True, capture_output=True)

    def commit(self, path, text, subject):
        file = self.root / path
        file.parent.mkdir(parents=True, exist_ok=True)
        file.write_text(text, encoding="utf-8")
        self.git("add", "-A")
        self.git("commit", "-q", "-m", subject)

    def test_the_pr_that_added_the_heading(self):
        built_by = step_issues.built_by_git(self.root)
        self.assertEqual(built_by("specs/39-x.md", "### Step 7b, the rout"), 602)

    def test_none_for_a_heading_never_added(self):
        built_by = step_issues.built_by_git(self.root)
        self.assertIsNone(built_by("specs/39-x.md", "### Step 7c, orders"))

    def test_a_merge_commits_subject(self):
        self.assertEqual(step_issues.pr_of_subject("Merge pull request #610 from o/b"), 610)
        self.assertEqual(step_issues.pr_of_subject("refactor: the rout (#602)"), 602)
        self.assertIsNone(step_issues.pr_of_subject("a commit pushed straight to main"))


if __name__ == "__main__":
    unittest.main()
