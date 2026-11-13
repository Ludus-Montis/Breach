"""Tests for release (spec 49 step 11, Decision 180): which version a push to main releases,
and that version's notes, cut from the changelog."""
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import release  # noqa: E402

REPO = "https://github.com/Ludus-Montis/Breach"

RELEASED = f"""# Changelog

Intro.

## [Unreleased]

## [0.1.0] - 2026-11-20

### Added

- A new thing.

### Fixed

- A thing works.

## [0.0.1] - 2026-11-13

### Fixed

- An old thing works.

[Unreleased]: {REPO}/compare/v0.1.0...HEAD
[0.1.0]: {REPO}/compare/v0.0.1...v0.1.0
[0.0.1]: {REPO}/releases/tag/v0.0.1
"""

UNRELEASED_ONLY = """# Changelog

## [Unreleased]

### Added

- Not yet released.
"""


class Version(unittest.TestCase):
    def test_the_newest_untagged_version_is_released(self):
        self.assertEqual(release.version_to_release(RELEASED, {"v0.0.1"}), "0.1.0")

    def test_a_tagged_newest_version_is_not_released_again(self):
        self.assertIsNone(release.version_to_release(RELEASED, {"v0.0.1", "v0.1.0"}))

    def test_no_version_section_releases_nothing(self):
        self.assertIsNone(release.version_to_release(UNRELEASED_ONLY, set()))


class Notes(unittest.TestCase):
    def test_the_notes_are_the_versions_section(self):
        expected = "### Added\n\n- A new thing.\n\n### Fixed\n\n- A thing works.\n"
        self.assertEqual(release.notes(RELEASED, "0.1.0"), expected)

    def test_the_last_sections_notes_stop_at_the_link_lines(self):
        self.assertEqual(release.notes(RELEASED, "0.0.1"), "### Fixed\n\n- An old thing works.\n")

    def test_a_missing_version_has_no_notes(self):
        with self.assertRaises(KeyError):
            release.notes(RELEASED, "9.9.9")


class AssetName(unittest.TestCase):
    def test_the_assets_name_carries_version_and_target(self):
        self.assertEqual(
            release.asset_name("0.1.0", "x86_64-pc-windows-msvc"),
            "breach-native-0.1.0-x86_64-pc-windows-msvc.zip",
        )


if __name__ == "__main__":
    unittest.main()
