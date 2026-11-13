"""prune.py: what the prune of the `native-store` pre-release deletes (spec 49 step 9,
Decision 179). The plan is a pure function of the assets and the trees to keep, so these
tests need no repo and no gh.
"""

import sys
import unittest
from datetime import datetime, timedelta, timezone
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import prune  # noqa: E402

NOW = datetime(2026, 11, 12, tzinfo=timezone.utc)
GB = 1024**3


def tree(n):
    return f"{n:040x}"


def asset(n, label, days_old=1, size=1, profile="release",
          target="x86_64-unknown-linux-gnu"):
    return prune.Asset(
        id=n * 10 + len(profile) + len(target),
        name=f"{tree(n)}-1.99.1-{profile}-{target}.zip",
        label=label,
        size=size,
        created=NOW - timedelta(days=days_old),
    )


def deleted(assets, **kwargs):
    kwargs.setdefault("forever", set())
    kwargs.setdefault("main_history", [])
    kwargs.setdefault("cap", GB)
    return sorted(a.name for a in prune.plan(assets, now=NOW, **kwargs))


class PlanTest(unittest.TestCase):
    def test_the_five_oldest_of_25_main_keys_go_and_references_and_releases_stay(self):
        mains = [asset(n, f"main {tree(n)}", days_old=100 - n) for n in range(1, 26)]
        reference = asset(100, "dispatch x", days_old=400)
        release = asset(101, "release x", days_old=300)
        history = [tree(n) for n in range(25, 0, -1)]  # newest first
        gone = deleted(mains + [reference, release],
                       forever={tree(100)}, main_history=history)
        self.assertEqual(gone, sorted(a.name for a in mains[:5]))

    def test_a_pr_only_key_15_days_old_goes_and_one_13_days_old_stays(self):
        old = asset(1, "pr 990 x", days_old=15)
        fresh = asset(2, "pr 991 x", days_old=13)
        self.assertEqual(deleted([old, fresh]), [old.name])

    def test_a_key_first_built_on_a_pr_that_main_then_reuses_counts_as_main(self):
        merged = asset(1, "pr 990 x", days_old=30)
        self.assertEqual(deleted([merged], main_history=[tree(1)]), [])

    def test_a_key_kept_forever_stays_whatever_its_label_and_age(self):
        reference = asset(1, "pr 1 x", days_old=999)
        unlabelled = asset(2, "", days_old=999)
        self.assertEqual(deleted([reference, unlabelled], forever={tree(1)}),
                         [unlabelled.name])

    def test_a_release_label_keeps_the_key(self):
        self.assertEqual(deleted([asset(1, "release v0.1.0 x", days_old=999)]), [])

    def test_every_asset_of_a_kept_main_key_stays(self):
        linux = asset(1, "main x")
        windows = asset(1, "main x", target="x86_64-pc-windows-msvc")
        profiling = asset(1, "main x", profile="profiling")
        self.assertEqual(deleted([linux, windows, profiling], main_history=[tree(1)],
                                 keep_main=1), [])

    def test_past_the_cap_the_oldest_unkept_keys_go_first(self):
        reference = asset(1, "dispatch x", days_old=50, size=6)
        older = asset(2, "main x", days_old=5, size=3)
        newer = asset(3, "main x", days_old=2, size=3)
        newest = asset(4, "pr 7 x", days_old=1, size=3)
        gone = deleted([reference, older, newer, newest], forever={tree(1)},
                       main_history=[tree(3), tree(2)], cap=9)
        self.assertEqual(gone, sorted([older.name, newer.name]))

    def test_a_name_that_is_not_a_key_is_left_alone(self):
        self.assertEqual(
            deleted([prune.Asset(1, "notes.txt", "", 1, NOW - timedelta(days=99))]), []
        )


class ReferencesTest(unittest.TestCase):
    def test_full_shas_in_workflows_are_references_but_pinned_actions_are_not(self):
        text = (
            "      REFERENCE: 8ca422726e68909554eddad7eeee2199e0fe656d\n"
            "      - uses: actions/cache@0123456789abcdef0123456789abcdef01234567\n"
            "    for sha in eee3e81fc92b026f869fdfced2f16a5306884d0b abc; do\n"
        )
        self.assertEqual(
            prune.reference_shas([text]),
            {"8ca422726e68909554eddad7eeee2199e0fe656d",
             "eee3e81fc92b026f869fdfced2f16a5306884d0b"},
        )


class OriginTest(unittest.TestCase):
    def test_a_label_names_where_the_key_was_first_built(self):
        self.assertEqual(prune.origin("main 94a603e"), "main")
        self.assertEqual(prune.origin("pr 990 94a603e"), "pr")
        self.assertEqual(prune.origin(""), "")


if __name__ == "__main__":
    unittest.main()
