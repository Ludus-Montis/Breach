"""Tests for local_pair (spec 48 step 5b): `local_grid.sh pair` times a head and a base in turn
on the owner's PC at the base's `inspect:` ticks, one tick a run (the repeat mode), and this
prints, per size and tick, the head's change against the base for the tick and each phase,
`*` where the rounds' ranges are apart, as `bench`'s summary does."""
import io
import sys
import tempfile
import unittest
from contextlib import redirect_stdout
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import local_pair  # noqa: E402


def output(tick: int, ms: float, seek: float, walk: float) -> str:
    """One repeat-mode run at one tick, as `breach_tick_phases ... at=<t> repeat=50` prints it."""
    return (
        "machine: unknown, 20 threads, Windows\n"
        f"repeat: tick {tick}, 50 repeats\n"
        f"4096 a side, 64 wide, tick {tick}: {ms:.2f} ms a tick\n"
        f"  scrum: seek                         {seek:.2f}\n"
        f"  units walk to places                {walk:.2f}\n"
        "  (ms)                               median      IQR   95% interval of the median\n"
        f"  tick                                {ms:.2f}     0.50   [{ms - 0.2:.2f}, {ms + 0.2:.2f}]\n"
    )


# (tick, side) -> per round (tick ms, seek, walk). At tick 91 the head's seek is faster in
# every round (apart); its walk overlaps the base's.
RUNS = {
    (91, "base"): [(30.0, 12.0, 4.0), (31.0, 12.5, 4.2), (29.0, 12.2, 4.1)],
    (91, "head"): [(27.0, 9.0, 4.1), (28.0, 9.4, 4.0), (27.5, 9.2, 4.3)],
    (101, "base"): [(32.0, 13.0, 4.0), (33.0, 13.4, 4.1), (31.0, 13.2, 4.2)],
    (101, "head"): [(32.5, 13.1, 4.0), (31.5, 13.0, 4.2), (33.5, 13.5, 4.1)],
}


def write_pair(folder: Path, size: str = "preset-4096", runs=None):
    for (tick, side), rounds in (runs or RUNS).items():
        for n, (ms, seek, walk) in enumerate(rounds, start=1):
            (folder / f"{size}_t{tick}_{side}_r{n}.txt").write_text(output(tick, ms, seek, walk))


def run(folder) -> tuple:
    out = io.StringIO()
    with redirect_stdout(out):
        code = local_pair.main([str(folder)])
    return code, out.getvalue()


class PairSummary(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.folder = Path(self.tmp.name)
        write_pair(self.folder)

    def tearDown(self):
        self.tmp.cleanup()

    def test_each_tick_has_its_own_table_in_tick_order(self):
        code, text = run(self.folder)
        self.assertEqual(code, 0)
        first, second = text.index("== preset-4096, tick 91"), text.index("== preset-4096, tick 101")
        self.assertLess(first, second)

    def test_the_change_and_the_apart_flag_per_phase(self):
        _, text = run(self.folder)
        at_91 = text[text.index("tick 91"):text.index("tick 101")]
        seek = next(line for line in at_91.splitlines() if "scrum: seek" in line)
        walk = next(line for line in at_91.splitlines() if "units walk" in line)
        tick = next(line for line in at_91.splitlines() if line.startswith("  tick "))
        self.assertIn("-24.6%", seek)  # medians 9.2 against 12.2
        self.assertTrue(seek.endswith("*"))
        self.assertFalse(walk.endswith("*"))
        self.assertIn("-8.3%", tick)  # medians 27.5 against 30.0
        self.assertTrue(tick.endswith("*"))

    def test_a_tick_that_moved_neither_way_flags_nothing(self):
        _, text = run(self.folder)
        at_101 = text[text.index("tick 101"):]
        self.assertNotIn("*\n", at_101.split("Q47")[0])

    def test_the_caveat_stands_beside_the_repeats(self):
        _, text = run(self.folder)
        self.assertIn("Q47", text)
        self.assertIn("the whole tick is the judge", text)

    def test_sizes_are_kept_apart(self):
        write_pair(self.folder, size="preset-25k", runs={k: v for k, v in RUNS.items() if k[0] == 91})
        _, text = run(self.folder)
        self.assertIn("== preset-25k, tick 91", text)
        self.assertNotIn("== preset-25k, tick 101", text)


class Refusals(unittest.TestCase):
    def test_a_tick_with_one_side_only_is_named(self):
        with tempfile.TemporaryDirectory() as tmp:
            write_pair(Path(tmp), runs={(91, "head"): RUNS[(91, "head")]})
            code, text = run(tmp)
        self.assertEqual(code, 1)
        self.assertIn("preset-4096, tick 91", text)
        self.assertIn("no base", text)

    def test_an_empty_folder_is_refused(self):
        with tempfile.TemporaryDirectory() as tmp:
            code, text = run(tmp)
        self.assertEqual(code, 1)
        self.assertIn("no pair outputs", text)


if __name__ == "__main__":
    unittest.main()
