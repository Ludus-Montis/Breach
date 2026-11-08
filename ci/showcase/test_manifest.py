"""Tests for manifest (spec 44 step 3, Decision 167): each PR's record of its captures, the
retention rule (an open PR keeps its last two, a closed one its last), and manifest.json,
built from every PR's record, newest capture first."""
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import manifest  # noqa: E402


def capture(sha: str, date: str) -> dict:
    name = f"pr7-{sha[:7]}-field_break_off"
    return {
        "commit": sha,
        "date": date,
        "release": "showcase-2026-11",
        "run": "https://github.com/o/r/actions/runs/1",
        "scenarios": [{"name": "field_break_off", "clip": f"https://x/{name}.webm",
                       "stills": {"first": f"https://x/{name}-first.webp"}}],
    }


def removed_names(removed: list) -> list:
    return [name for _, name in removed]


class RetentionTest(unittest.TestCase):
    def setUp(self):
        self.record = manifest.new_record(7, "feat: a thing", "https://github.com/o/r/pull/7")

    def test_an_open_pr_keeps_its_last_two_captures(self):
        first = capture("a" * 40, "2026-11-08T10:00:00Z")
        record, removed = manifest.add_capture(self.record, first)
        self.assertEqual(removed, [])
        record, removed = manifest.add_capture(record, capture("b" * 40, "2026-11-08T11:00:00Z"))
        self.assertEqual(removed, [])
        record, removed = manifest.add_capture(record, capture("c" * 40, "2026-11-08T12:00:00Z"))
        self.assertEqual([c["commit"][0] for c in record["captures"]], ["b", "c"])
        self.assertEqual(removed, [("showcase-2026-11", "pr7-aaaaaaa-field_break_off.webm"),
                                   ("showcase-2026-11", "pr7-aaaaaaa-field_break_off-first.webp")])

    def test_a_new_push_on_the_same_commit_replaces_its_capture(self):
        record, _ = manifest.add_capture(self.record, capture("a" * 40, "2026-11-08T10:00:00Z"))
        record, removed = manifest.add_capture(record, capture("a" * 40, "2026-11-08T10:30:00Z"))
        self.assertEqual(len(record["captures"]), 1)
        self.assertEqual(record["captures"][0]["date"], "2026-11-08T10:30:00Z")
        self.assertEqual(removed, [])  # the same names, clobbered on upload

    def test_a_closed_pr_keeps_only_its_last_merged_or_not(self):
        for state in ("merged", "closed"):
            record, _ = manifest.add_capture(self.record, capture("a" * 40, "2026-11-08T10:00:00Z"))
            record, _ = manifest.add_capture(record, capture("b" * 40, "2026-11-08T11:00:00Z"))
            record, removed = manifest.close(record, state, "feat: renamed", "2026-11-08T13:00:00Z")
            self.assertEqual(record["state"], state)
            self.assertEqual(record["title"], "feat: renamed")
            self.assertEqual(record["closed_at"], "2026-11-08T13:00:00Z")
            self.assertEqual([c["commit"][0] for c in record["captures"]], ["b"])
            self.assertEqual(removed_names(removed), ["pr7-aaaaaaa-field_break_off.webm",
                                                      "pr7-aaaaaaa-field_break_off-first.webp"])

    def test_closing_a_pr_with_no_capture_keeps_none(self):
        record, removed = manifest.close(self.record, "closed", "t", "2026-11-08T13:00:00Z")
        self.assertEqual((record["captures"], removed), ([], []))


class MergeTest(unittest.TestCase):
    def test_the_manifest_lists_every_capture_newest_first_with_its_pr(self):
        seven, _ = manifest.add_capture(manifest.new_record(7, "seven", "u7"),
                                        capture("a" * 40, "2026-11-08T10:00:00Z"))
        seven, _ = manifest.add_capture(seven, capture("b" * 40, "2026-11-08T12:00:00Z"))
        nine, _ = manifest.add_capture(manifest.new_record(9, "nine", "u9"),
                                       capture("c" * 40, "2026-11-08T11:00:00Z"))
        nine, _ = manifest.close(nine, "merged", "nine", "2026-11-08T11:30:00Z")
        built = manifest.merge([seven, nine])
        self.assertEqual(built["version"], 1)
        rows = [(c["pr"], c["commit"][0], c["state"]) for c in built["captures"]]
        self.assertEqual(rows, [(7, "b", "open"), (9, "c", "merged"), (7, "a", "open")])
        merged = built["captures"][1]
        self.assertEqual((merged["title"], merged["url"], merged["closed_at"]),
                         ("nine", "u9", "2026-11-08T11:30:00Z"))
        self.assertNotIn("closed_at", built["captures"][0])

    def test_a_record_with_no_captures_adds_nothing(self):
        self.assertEqual(manifest.merge([manifest.new_record(3, "t", "u")])["captures"], [])


if __name__ == "__main__":
    unittest.main()
