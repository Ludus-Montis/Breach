"""Tests for entry (spec 44 step 3): one scenario's part of a capture, from its capture.json,
and which frames its stills are cut from (10 ticks a second, 3 frames a tick at 30 fps)."""
import json
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import entry  # noqa: E402

CAPTURE = json.loads((HERE / "fixtures" / "capture.json").read_text())
BASE = "https://github.com/o/r/releases/download/showcase-2026-11"


class EntryTest(unittest.TestCase):
    def test_assets_are_named_by_pr_commit_and_scenario(self):
        self.assertEqual(entry.prefix(844, "5eb56236f64a", "field_break_off"),
                         "pr844-5eb5623-field_break_off")

    def test_a_still_is_cut_three_frames_a_tick_and_never_past_the_last_frame(self):
        self.assertEqual(entry.stills(CAPTURE, 873),
                         [("first", 0), ("contact", 384), ("midpoint", 627), ("last", 870)])
        self.assertEqual(entry.stills(CAPTURE, 800)[-1], ("last", 799))

    def test_a_scenario_with_no_contact_has_no_contact_still(self):
        capture = dict(CAPTURE, moments=dict(CAPTURE["moments"], contact=None))
        self.assertEqual([m for m, _ in entry.stills(capture, 873)], ["first", "midpoint", "last"])

    def test_the_entry_holds_urls_moments_and_behaviours_seen_and_not(self):
        timed = {"rounds": 5, "tick_median_ms": 0.05}
        made = entry.scenario_entry(CAPTURE, BASE, "pr844-5eb5623-field_break_off", timed)
        self.assertEqual(made["name"], "field_break_off")
        self.assertEqual(made["clip"], f"{BASE}/pr844-5eb5623-field_break_off.webm")
        self.assertEqual(made["stills"]["contact"],
                         f"{BASE}/pr844-5eb5623-field_break_off-contact.webp")
        self.assertEqual(sorted(made["stills"]), ["contact", "first", "last", "midpoint"])
        self.assertEqual(made["moments"], CAPTURE["moments"])
        self.assertEqual(made["ticks"], 291)
        self.assertEqual(made["seen"], {"engaging": 124})
        self.assertEqual(made["not_seen"],
                         ["fording", "flanking", "breaking_off", "cornering", "routing"])
        self.assertEqual(made["missing"], ["breaking_off"])
        self.assertEqual(made["timings"], timed)
        self.assertEqual(entry.assets(made), [
            "pr844-5eb5623-field_break_off.webm",
            "pr844-5eb5623-field_break_off-first.webp",
            "pr844-5eb5623-field_break_off-contact.webp",
            "pr844-5eb5623-field_break_off-midpoint.webp",
            "pr844-5eb5623-field_break_off-last.webp",
        ])


if __name__ == "__main__":
    unittest.main()
