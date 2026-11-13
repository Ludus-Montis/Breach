"""Tests for the number check (spec 51 step 1, Decision 181): a spec or Decision number
belongs to `main`, then to the open PR with the lowest PR number that adds it."""
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import check_numbers  # noqa: E402
from check_numbers import Numbers  # noqa: E402

FIXTURES = Path(__file__).resolve().parent / "fixtures" / "numbers"
MAIN = Numbers(specs={44, 45}, decisions={171, 172})


def _adds(name):
    """What the fixture tree `name` adds over the fixture's `main` (its merge-base)."""
    base = check_numbers.tree_numbers(FIXTURES / "main")
    return check_numbers.added(check_numbers.tree_numbers(FIXTURES / name), base)


class ReplayTest(unittest.TestCase):
    """The known clash: #875 took spec 46 and Decisions 173-176, then #878 took spec 46 and
    Decision 173 again (renumbered by hand to spec 47 and Decision 177)."""

    def setUp(self):
        self.main = check_numbers.tree_numbers(FIXTURES / "main")
        self.open_prs = {875: _adds("pr875"), 878: _adds("pr878")}

    def test_the_later_pr_fails_naming_the_earlier_and_the_next_free_numbers(self):
        verdict = check_numbers.check(878, self.main, self.open_prs)
        self.assertFalse(verdict.passed)
        self.assertEqual(
            verdict.lines(),
            [
                "spec 46 is held by #875; the next free spec is 47",
                "Decision 173 is held by #875; the next free Decision is 177",
            ],
        )

    def test_the_earlier_pr_passes_and_prints_what_it_holds(self):
        verdict = check_numbers.check(875, self.main, self.open_prs)
        self.assertTrue(verdict.passed)
        self.assertEqual(verdict.lines(), ["#875 holds spec 46; Decisions 173, 174, 175, 176"])


class TreeTest(unittest.TestCase):
    def test_a_tree_holds_its_spec_files_and_its_root_decision_headings(self):
        # specs/README.md is no spec, and docs/notes.md isn't at the root.
        self.assertEqual(check_numbers.tree_numbers(FIXTURES / "main"), MAIN)

    def test_retitling_a_decision_adds_nothing_and_passes(self):
        with tempfile.TemporaryDirectory() as tmp:
            head = Path(tmp) / "head"
            shutil.copytree(FIXTURES / "main", head)
            spec = head / "Design Spec.md"
            spec.write_text(
                spec.read_text(encoding="utf-8").replace(
                    "### Decision 171 — A spec's steps say",
                    "### Decision 171 — Each step of a spec says",
                ),
                encoding="utf-8",
            )
            adds = check_numbers.added(check_numbers.tree_numbers(head), MAIN)
        self.assertEqual(adds, Numbers())
        verdict = check_numbers.check(900, MAIN, {900: adds})
        self.assertTrue(verdict.passed)
        self.assertEqual(verdict.lines(), ["#900 adds no spec or Decision number"])


class HolderTest(unittest.TestCase):
    def test_a_number_main_holds_fails_naming_main(self):
        main = Numbers(specs={51, 52}, decisions={181})
        verdict = check_numbers.check(950, main, {950: Numbers(specs={52})})
        self.assertFalse(verdict.passed)
        self.assertEqual(verdict.lines(), ["spec 52 is held by main; the next free spec is 53"])

    def test_the_earlier_open_pr_keeps_a_number_a_later_one_holds_too(self):
        main = Numbers(specs={51}, decisions={181})
        open_prs = {960: Numbers(decisions={182}), 970: Numbers(decisions={182, 183})}
        self.assertTrue(check_numbers.check(960, main, open_prs).passed)
        later = check_numbers.check(970, main, open_prs)
        self.assertFalse(later.passed)
        # The next free is past every open PR's numbers, earlier or later.
        self.assertEqual(
            later.lines(), ["Decision 182 is held by #960; the next free Decision is 184"]
        )

    def test_the_next_free_counts_later_prs_too(self):
        main = Numbers(specs={51}, decisions={181})
        open_prs = {960: Numbers(specs={52}), 961: Numbers(specs={52}), 990: Numbers(specs={53})}
        self.assertEqual(
            check_numbers.check(961, main, open_prs).lines(),
            ["spec 52 is held by #960; the next free spec is 54"],
        )


def _git(repo, *args):
    subprocess.run(["git", "-C", str(repo), *args], check=True, capture_output=True)


class GitTreeTest(unittest.TestCase):
    """Step 2's front reads a tree from a git ref, with no checkout."""

    def test_a_ref_holds_what_its_checked_out_tree_holds(self):
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            _git(repo, "init", "-q")
            shutil.copytree(FIXTURES / "main", repo, dirs_exist_ok=True)
            _git(repo, "add", "-A")
            _git(repo, "-c", "user.name=t", "-c", "user.email=t@t", "commit", "-qm", "main")
            shutil.copytree(FIXTURES / "pr875", repo, dirs_exist_ok=True)
            _git(repo, "add", "-A")
            _git(repo, "-c", "user.name=t", "-c", "user.email=t@t", "commit", "-qm", "875")
            self.assertEqual(check_numbers.git_numbers(repo, "HEAD~1"), MAIN)
            self.assertEqual(
                check_numbers.git_numbers(repo, "HEAD"),
                check_numbers.tree_numbers(FIXTURES / "pr875"),
            )

    def test_a_root_file_with_an_em_dash_in_its_name_is_read(self):
        # git quotes such a name unless asked not to; the design spec's name has one.
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            _git(repo, "init", "-q")
            (repo / "Breach — Spec.md").write_text("### Decision 181 — A number\n", "utf-8")
            _git(repo, "add", "-A")
            _git(repo, "-c", "user.name=t", "-c", "user.email=t@t", "commit", "-qm", "main")
            self.assertEqual(check_numbers.git_numbers(repo, "HEAD"), Numbers(decisions={181}))


class StatusTest(unittest.TestCase):
    """A run sets this PR's `numbers` status, and fails each later PR it clashes with."""

    def test_this_pr_and_each_later_clashing_pr_get_a_status(self):
        main = Numbers(specs={51}, decisions={181})
        open_prs = {
            960: Numbers(decisions={182}),
            970: Numbers(decisions={182}),
            980: Numbers(specs={52}),
            950: Numbers(decisions={182}),
        }
        self.assertEqual(
            check_numbers.statuses(960, main, open_prs),
            [
                (960, "failure", "Decision 182 is held by #950; the next free Decision is 183"),
                (
                    970,
                    "failure",
                    "Decision 182 is held by #950; the next free Decision is 183"
                    " (set by #960's run; re-run or push to check again)",
                ),
            ],
        )

    def test_a_passing_pr_reports_what_it_holds_and_leaves_unrelated_prs_alone(self):
        main = Numbers(specs={51})
        open_prs = {960: Numbers(specs={52}), 970: Numbers(specs={52}), 980: Numbers(specs={53})}
        self.assertEqual(
            check_numbers.statuses(960, main, open_prs),
            [
                (960, "success", "#960 holds spec 52"),
                (
                    970,
                    "failure",
                    "spec 52 is held by #960; the next free spec is 54"
                    " (set by #960's run; re-run or push to check again)",
                ),
            ],
        )


if __name__ == "__main__":
    unittest.main()
