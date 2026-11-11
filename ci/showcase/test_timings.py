"""Tests for timings (spec 44 step 3): a scenario's tick median, 95th percentile and its
phases' medians, over the rounds of `breach_tick_phases scenario=<name>`."""
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import timings  # noqa: E402

ROUND = (HERE / "fixtures" / "tick_phases_round.txt").read_text()


def scaled(text: str, factor: float) -> str:
    """The fixture round with every series figure multiplied by `factor`."""
    out, in_series = [], False
    for line in text.splitlines():
        if line.startswith("series:"):
            in_series = True
        elif in_series:
            name, parts = line[:34], line[34:].split()
            figures = [f"{float(p) * factor:>8.2f}" for p in parts[:4]]
            line = name + " " + " ".join([*figures, f"{parts[4]:>8}"])
        out.append(line)
    return "\n".join(out)


class TimingsTest(unittest.TestCase):
    def test_one_round_reads_the_series_rows(self):
        parsed = timings.parse(ROUND)
        self.assertEqual(parsed["tick"], {"mean": 0.05, "p95": 0.09})
        self.assertEqual(parsed["phases"], {"march (squads move)": 0.03, "scrum: seek": 0.02})

    def test_the_rounds_give_medians(self):
        rounds = [scaled(ROUND, f) for f in (1, 3, 2, 10, 2)]
        summary = timings.summarise(rounds)
        self.assertEqual(summary["rounds"], 5)
        self.assertAlmostEqual(summary["tick_median_ms"], 0.10)
        self.assertAlmostEqual(summary["tick_p95_ms"], 0.18)
        self.assertAlmostEqual(summary["phases_median_ms"]["march (squads move)"], 0.06)
        self.assertEqual(list(summary["phases_median_ms"]), ["march (squads move)", "scrum: seek"])

    def test_output_without_a_series_is_an_error(self):
        with self.assertRaises(ValueError):
            timings.parse("breach_tick_phases: no scenario named x\n")


if __name__ == "__main__":
    unittest.main()
