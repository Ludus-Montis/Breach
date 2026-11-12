"""Tests for the number check (spec 51 step 1, Decision 181): a spec or Decision number
belongs to `main`, then to the open PR with the lowest PR number that adds it."""
import shutil
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


if __name__ == "__main__":
    unittest.main()
