"""Tests for a step's `**Needs:**`, `**Touches:**` and `**Split at build:**` lines (spec 45
step 1, Decision 171): the sync copies the needs and files into each step's issue, says
`every step above it` where a step declares none, closes a split step's issue naming the
steps that replaced it, and fails on a need the spec doesn't have."""
import io
import sys
import tempfile
import unittest
from contextlib import redirect_stdout
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import step_lines  # noqa: E402
import sync  # noqa: E402
from fake_client import FakeClient  # noqa: E402

REPO_URL = "https://github.com/o/r"

SPEC_50 = """# Spec 50: Lines

## Steps

### Step 1, the first

**Needs:** none
**Touches:** `a/one.py`

### Step 2a, the second

**Needs:** 1
**Touches:** `a/b.py`,
`c/`

### Step 3, the third

**Needs:** 1, 2a
**Touches:** a/b.py, c/

### Step 4, the last

No lines here.
"""

SPEC_50_SPLIT = """# Spec 50: Lines

## Steps

### Step 1, the first

**Needs:** none
**Touches:** `a/one.py`

### Step 2a, the second

**Needs:** 1
**Touches:** `a/b.py`, `c/`

### Step 3a, the third's parsing

**Split at build:** from step 3, PR #900, 2026-11-11: two test files
**Needs:** 1, 2a
**Touches:** `a/b.py`

### Step 3b, the third's body

**Split at build:** from step 3, PR #900, 2026-11-11: two test files
**Needs:** 3a
**Touches:** `c/`

### Step 4, the last
"""


def _quiet(function, *args, **kwargs):
    with redirect_stdout(io.StringIO()):
        return function(*args, **kwargs)


class StepLinesParseTest(unittest.TestCase):
    def test_reads_needs_touches_and_split_per_step(self):
        lines = step_lines.parse(SPEC_50_SPLIT)
        self.assertEqual(lines["1"].needs, [])
        self.assertTrue(lines["1"].needs_declared)
        self.assertEqual(lines["3a"].needs, ["1", "2a"])
        self.assertEqual(lines["2a"].touches, ["a/b.py", "c/"])
        self.assertEqual(lines["3b"].split_from, "3")
        self.assertIn("PR #900", lines["3b"].split_note)
        self.assertFalse(lines["4"].needs_declared)
        self.assertEqual(lines["4"].touches, [])

    def test_touches_runs_over_lines_and_backticks_are_optional(self):
        lines = step_lines.parse(SPEC_50)
        self.assertEqual(lines["2a"].touches, ["a/b.py", "c/"])
        self.assertEqual(lines["3"].touches, ["a/b.py", "c/"])


class StepLinesSyncTest(unittest.TestCase):
    def setUp(self):
        self._dir = tempfile.TemporaryDirectory()
        self.specs = Path(self._dir.name)
        self.write(SPEC_50)

    def tearDown(self):
        self._dir.cleanup()

    def write(self, text):
        (self.specs / "50-lines.md").write_text(text, encoding="utf-8")
        self.plan = sync.plan(self.specs, REPO_URL)

    def client(self):
        return FakeClient(labels=[n for n, _, _ in self.plan.labels], milestones=self.plan.milestones)

    def body(self, client, step):
        return client.records[client.number_of(f"Spec 50 step {step}:", prefix=True)]["body"]

    def test_issue_lists_its_needs_as_links_and_its_files(self):
        client = self.client()
        _quiet(sync.apply, client, self.plan)
        one = client.number_of("Spec 50 step 1:", prefix=True)
        two = client.number_of("Spec 50 step 2a:", prefix=True)
        body = self.body(client, "3")
        self.assertIn(f"**Needs:** step 1 (#{one}), step 2a (#{two})", body)
        self.assertEqual(client.edited, [])
        self.assertIn("**Touches:** `a/b.py`, `c/`", body)
        self.assertIn("<!-- github-sync: step 50-3 -->", body)

    def test_a_step_with_no_lines_needs_every_step_above_it(self):
        client = self.client()
        _quiet(sync.apply, client, self.plan)
        self.assertIn("**Needs:** every step above it (none declared)", self.body(client, "4"))
        self.assertIn("**Needs:** none", self.body(client, "1"))

    def test_an_open_issue_takes_new_lines_and_a_second_run_writes_nothing(self):
        client = self.client()
        old = client.add_issue("Spec 50 step 3: the third", body="x\n<!-- github-sync: step 50-3 -->")
        _quiet(sync.apply, client, self.plan)
        self.assertIn("**Touches:** `a/b.py`, `c/`", client.records[old]["body"])
        client.reset_writes()
        _quiet(sync.apply, client, self.plan)
        self.assertEqual(client.edited, [])
        self.assertEqual(client.created_issues, [])

    def test_a_closed_issue_keeps_its_body(self):
        client = self.client()
        old = client.add_issue("Spec 50 step 3: the third", body="x\n<!-- github-sync: step 50-3 -->")
        client.close_issue(old, "completed")
        _quiet(sync.apply, client, self.plan)
        self.assertEqual(client.records[old]["body"], "x\n<!-- github-sync: step 50-3 -->")

    def test_a_split_step_is_closed_naming_the_steps_that_replaced_it(self):
        client = self.client()
        _quiet(sync.apply, client, self.plan)
        three = client.number_of("Spec 50 step 3:", prefix=True)
        self.write(SPEC_50_SPLIT)
        client.reset_writes()
        _quiet(sync.apply, client, self.plan)
        a = client.number_of("Spec 50 step 3a:", prefix=True)
        b = client.number_of("Spec 50 step 3b:", prefix=True)
        self.assertIn((three, "not_planned"), client.closed)
        [comment] = [body for number, body in client.comments if number == three]
        self.assertIn("Split at build", comment)
        self.assertIn(f"step 3a (#{a})", comment)
        self.assertIn(f"step 3b (#{b})", comment)
        self.assertIn("PR #900", comment)
        self.assertNotIn("any more", comment)

    def test_a_need_the_spec_does_not_have_fails_the_sync(self):
        with self.assertRaisesRegex(SystemExit, r"specs/50-lines\.md.*step 3.*needs step 9"):
            self.write(SPEC_50.replace("**Needs:** 1, 2a", "**Needs:** 1, 9"))


if __name__ == "__main__":
    unittest.main()
