"""Tests for check_outcomes.py: a PR's digests are compared with its base's, so an outcome
change both engines share is still seen; a branch meaning to change outcomes says why in an
`Outcomes-changed:` commit trailer, and the check reports and passes."""
import contextlib
import io
import tempfile
import unittest
from pathlib import Path
from unittest import mock

import check_outcomes
from check_outcomes import changed_runs, reason, verdict

BASE = [
    "mirror_headon s1 rtrue 1 aaaa",
    "mirror_headon s1 rtrue 2 bbbb",
    "clash s1 rfalse 1 cccc",
]


class ChangedRunsTest(unittest.TestCase):
    def test_the_same_lines_change_nothing(self):
        self.assertEqual(changed_runs(BASE, list(BASE)), [])

    def test_each_run_that_differs_is_named_with_its_first_differing_tick(self):
        head = [BASE[0], "mirror_headon s1 rtrue 2 dddd", "clash s1 rfalse 1 eeee"]
        self.assertEqual(
            changed_runs(BASE, head), [("mirror_headon s1 rtrue", "2"), ("clash s1 rfalse", "1")]
        )

    def test_a_run_that_plays_longer_or_shorter_differs(self):
        head = BASE + ["clash s1 rfalse 2 ffff"]
        self.assertEqual(changed_runs(BASE, head), [("clash s1 rfalse", "2")])

    def test_a_run_only_one_side_has_differs(self):
        head = BASE + ["wide s1 rtrue 1 9999"]
        self.assertEqual(changed_runs(BASE, head), [("wide s1 rtrue", "1")])


class ReasonTest(unittest.TestCase):
    def test_a_trailer_gives_the_reason(self):
        messages = ["feat: x\n\nbody\n\nOutcomes-changed: Decision 147\nCo-Authored-By: someone"]
        self.assertEqual(reason(messages), "Decision 147")

    def test_any_commit_of_the_branch_may_carry_it(self):
        self.assertEqual(
            reason(["docs: y", "fix: x\n\nOutcomes-changed: fix, routers no longer stall"]),
            "fix, routers no longer stall",
        )

    def test_no_trailer_no_reason(self):
        self.assertIsNone(reason(["feat: x\n\nthe outcomes changed, in passing"]))

    def test_the_trailer_must_say_why(self):
        self.assertIsNone(reason(["feat: x\n\nOutcomes-changed:   "]))


class VerdictTest(unittest.TestCase):
    def test_the_same_outcomes_pass(self):
        passed, lines = verdict("standard", BASE, list(BASE), [])
        self.assertTrue(passed)
        self.assertIn("same as the base", lines[0])

    def test_changed_outcomes_without_a_reason_fail_and_name_the_runs(self):
        head = [BASE[0], "mirror_headon s1 rtrue 2 dddd", BASE[2]]
        passed, lines = verdict("standard", BASE, head, ["feat: x"])
        self.assertFalse(passed)
        self.assertIn("1 run", lines[0])
        self.assertIn("mirror_headon s1 rtrue from tick 2", "\n".join(lines))
        self.assertIn("Outcomes-changed:", "\n".join(lines))

    def test_changed_outcomes_with_a_reason_pass_and_say_it(self):
        head = [BASE[0], "mirror_headon s1 rtrue 2 dddd", BASE[2]]
        passed, lines = verdict("standard", BASE, head, ["feat: x\n\nOutcomes-changed: Decision 147"])
        self.assertTrue(passed)
        self.assertIn("Decision 147", lines[0])

    def test_no_base_digest_passes_with_a_note(self):
        passed, lines = verdict("standard", [], BASE, [])
        self.assertTrue(passed)
        self.assertIn("no base digest", lines[0])


class ChangedFileTest(unittest.TestCase):
    """`--changed <file>`: each changed run and its first differing tick, for the trace step."""

    def _run(self, base, head):
        folder = Path(tempfile.mkdtemp())
        (folder / "base.txt").write_text("\n".join(base) + "\n")
        (folder / "head.txt").write_text("\n".join(head) + "\n")
        out = folder / "out.txt"
        argv = ["check_outcomes.py", "standard", str(folder / "base.txt"), str(folder / "head.txt")]
        with mock.patch.object(check_outcomes, "branch_messages", return_value=[]):
            with contextlib.redirect_stdout(io.StringIO()):
                check_outcomes.main(argv + ["origin/main", "--changed", str(out)])
        return out.read_text()

    def test_the_changed_runs_are_written(self):
        base = ["a 6 1111", "a 7 2222", "b 30 3333", "c 1 4444"]
        head = ["a 6 1111", "a 7 9999", "b 30 8888", "c 1 4444"]
        self.assertEqual(self._run(base, head), "a\t7\nb\t30\n")

    def test_nothing_changed_nothing_written(self):
        self.assertEqual(self._run(BASE, list(BASE)), "")


class EngineTest(unittest.TestCase):
    """`--engine <name>`: the engine both digests were taken on, named in the report
    (Decision 151: outcomes are guarded on Rust, the base built natively)."""

    def _run(self, base, head, messages):
        folder = Path(tempfile.mkdtemp())
        (folder / "base.txt").write_text("\n".join(base) + "\n")
        (folder / "head.txt").write_text("\n".join(head) + "\n")
        argv = ["check_outcomes.py", "standard", str(folder / "base.txt"), str(folder / "head.txt")]
        out = io.StringIO()
        with mock.patch.object(check_outcomes, "branch_messages", return_value=messages):
            with contextlib.redirect_stdout(out):
                code = check_outcomes.main(argv + ["origin/main", "--engine", "rust"])
        return code, out.getvalue()

    def test_a_rust_digest_that_differs_without_a_trailer_fails_naming_the_engine(self):
        head = [BASE[0], "mirror_headon s1 rtrue 2 dddd", BASE[2]]
        code, out = self._run(BASE, head, ["perf: x"])
        self.assertEqual(code, 1)
        self.assertIn("check_outcomes standard (rust): 1 run changed", out)
        self.assertIn("mirror_headon s1 rtrue from tick 2", out)

    def test_with_the_trailer_it_passes_and_names_the_runs(self):
        head = [BASE[0], "mirror_headon s1 rtrue 2 dddd", BASE[2]]
        code, out = self._run(BASE, head, ["perf: x\n\nOutcomes-changed: f32 columns"])
        self.assertEqual(code, 0)
        self.assertIn("check_outcomes standard (rust): 1 run changed, as the branch says: f32", out)
        self.assertIn("mirror_headon s1 rtrue from tick 2", out)

    def test_the_same_rust_digest_passes(self):
        code, out = self._run(BASE, list(BASE), [])
        self.assertEqual(code, 0)
        self.assertIn("check_outcomes standard (rust): same as the base", out)


if __name__ == "__main__":
    unittest.main()
