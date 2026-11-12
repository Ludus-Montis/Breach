"""Tests for release_pr (spec 49 step 10, Decision 180): the next 0.x.y version from the last
tag, `[Unreleased]` and the commits since the tag, and the changelog the release PR holds."""
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import release_pr  # noqa: E402

REPO = "https://github.com/Ludus-Montis/Breach"

HEAD = """# Changelog

Intro.

"""


def changelog(unreleased, rest=""):
    return HEAD + "## [Unreleased]\n\n" + unreleased + rest


FIXED_ONLY = "### Fixed\n\n- A thing works.\n\n"
ADDED = "### Added\n\n- A new thing.\n\n### Fixed\n\n- A thing works.\n\n"


class NextVersion(unittest.TestCase):
    def test_no_tag_and_some_entries_is_0_0_1(self):
        text = changelog(ADDED)
        self.assertEqual(release_pr.next_version(None, text, []), "0.0.1")

    def test_an_added_entry_bumps_the_minor(self):
        text = changelog(ADDED)
        self.assertEqual(release_pr.next_version("v0.0.1", text, ["fix: x"]), "0.1.0")

    def test_only_fixed_bumps_the_patch(self):
        text = changelog(FIXED_ONLY)
        self.assertEqual(release_pr.next_version("v0.0.1", text, ["fix: x"]), "0.0.2")

    def test_a_breaking_commit_bumps_the_minor_never_the_major(self):
        text = changelog(FIXED_ONLY)
        self.assertEqual(release_pr.next_version("v0.3.4", text, ["feat!: drop y"]), "0.4.0")
        self.assertEqual(
            release_pr.next_version("v0.3.4", text, ["fix(sim): z\n\nBREAKING CHANGE: w"]),
            "0.4.0",
        )

    def test_a_breaking_entry_bumps_the_minor(self):
        text = changelog("### Changed\n\n- **BREAKING:** maps need a key.\n\n")
        self.assertEqual(release_pr.next_version("v0.3.4", text, []), "0.4.0")

    def test_a_scoped_commit_without_bang_is_not_breaking(self):
        text = changelog(FIXED_ONLY)
        self.assertEqual(release_pr.next_version("v0.3.4", text, ["feat(sim): y"]), "0.3.5")

    def test_empty_unreleased_is_no_release(self):
        text = changelog("### Added\n\n", "## [0.0.1] - 2026-11-01\n\n" + FIXED_ONLY)
        self.assertIsNone(release_pr.next_version("v0.0.1", text, ["feat!: x"]))

    def test_entries_of_older_versions_do_not_count(self):
        text = changelog(FIXED_ONLY, "## [0.0.1] - 2026-11-01\n\n" + ADDED)
        self.assertEqual(release_pr.next_version("v0.0.1", text, []), "0.0.2")


class Rewrite(unittest.TestCase):
    def test_unreleased_becomes_the_version_under_a_fresh_unreleased(self):
        text = changelog(FIXED_ONLY + "[Unreleased]: https://example.com/old\n")
        out = release_pr.rewrite(text, "0.0.1", "2026-11-12", REPO)
        self.assertIn("## [Unreleased]\n\n## [0.0.1] - 2026-11-12\n\n### Fixed\n", out)
        self.assertIsNone(release_pr.next_version("v0.0.1", out, ["feat: x"]))

    def test_the_first_version_links_to_its_tag(self):
        out = release_pr.rewrite(changelog(FIXED_ONLY), "0.0.1", "2026-11-12", REPO)
        self.assertTrue(
            out.endswith(
                f"\n[Unreleased]: {REPO}/compare/v0.0.1...HEAD\n"
                f"[0.0.1]: {REPO}/releases/tag/v0.0.1\n"
            ),
            out,
        )

    def test_later_versions_link_to_their_compare(self):
        text = changelog(
            ADDED,
            "## [0.0.1] - 2026-11-01\n\n" + FIXED_ONLY
            + f"[Unreleased]: {REPO}/compare/v0.0.1...HEAD\n"
            + f"[0.0.1]: {REPO}/releases/tag/v0.0.1\n",
        )
        out = release_pr.rewrite(text, "0.1.0", "2026-11-12", REPO)
        self.assertTrue(
            out.endswith(
                f"\n[Unreleased]: {REPO}/compare/v0.1.0...HEAD\n"
                f"[0.1.0]: {REPO}/compare/v0.0.1...v0.1.0\n"
                f"[0.0.1]: {REPO}/releases/tag/v0.0.1\n"
            ),
            out,
        )
        self.assertEqual(out.count("[Unreleased]: "), 1)

    def test_a_link_line_in_the_middle_moves_to_the_bottom(self):
        text = changelog("### Added\n\n- A.\n\n[Unreleased]: https://x/commits/main\n\n" + FIXED_ONLY)
        out = release_pr.rewrite(text, "0.0.1", "2026-11-12", REPO)
        self.assertNotIn("https://x/commits/main", out)
        self.assertIn("- A.\n\n### Fixed\n", out)


if __name__ == "__main__":
    unittest.main()
