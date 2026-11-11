"""Tests for bench_inspect.py (spec 47 step 4): the plan reads each size's inspection ticks
from the base's series, and the summary joins the matrix's jobs, each head against base on
its own runner, and combines them across jobs only as ratios to each job's reference."""
import os
import tempfile
import unittest

from bench_inspect import combined, matrix, report_size, ticks

SERIES = """machine: AMD EPYC 7763 64-Core Processor, 4 threads, Linux
4096 a side, 32 wide: 30.00 ms a tick
  scrum: seek                          11.00
series: 54 ticks from tick 91 (ms)       mean     95th     99th    worst  at tick
  tick                                 30.00    40.00    42.00    44.00      105
  scrum: seek                          11.00    12.00    13.00    14.00      100
inspect: 91,100,105,108,126,127,144
"""

REPEAT = """repeat: tick {t}, 50 repeats
4096 a side, tick {t}: {tick:.2f} ms a tick
  scrum: seek                       {seek:8.2f}
  (ms)                               median      IQR   95% interval of the median
  tick                                 {tick:.2f}     0.10   [1.00, 2.00]
"""

REFERENCE = """1024 a side, 32 wide: {tick:.2f} ms a tick
  scrum: seek                          3.00
"""


def write(folder, name, text):
    path = os.path.join(folder, name)
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8") as out:
        out.write(text)


def job(root, size, tick, cpu, heads, bases, references):
    """One inspect job's folder: its rounds, its reference's and its CPU."""
    folder = os.path.join(root, f"inspect-{size}-{tick}")
    for n, (head, base, ref) in enumerate(zip(heads, bases, references), start=1):
        write(folder, f"head_{n}.txt", REPEAT.format(t=tick, tick=head, seek=head / 3))
        write(folder, f"base_{n}.txt", REPEAT.format(t=tick, tick=base, seek=base / 3))
        write(folder, f"reference_{n}.txt", REFERENCE.format(tick=ref))
    write(folder, "cpu.txt", cpu + "\n")


class PlanTest(unittest.TestCase):
    def test_the_inspection_ticks_are_read_from_the_series_inspect_line(self):
        self.assertEqual(ticks(SERIES), [91, 100, 105, 108, 126, 127, 144])

    def test_a_series_with_no_inspect_line_names_no_ticks(self):
        self.assertEqual(ticks(SERIES.replace("inspect: 91,100,105,108,126,127,144\n", "")), [])

    def test_the_matrix_is_one_job_per_size_and_tick_each_size_its_own_ticks(self):
        jobs = matrix({"4096": SERIES, "25k": "inspect: 91,100,101\n"})
        self.assertEqual(len(jobs), 10)
        self.assertEqual(jobs[0], {"size": "4096", "tick": 91})
        self.assertEqual([j["tick"] for j in jobs if j["size"] == "25k"], [91, 100, 101])


class SummaryTest(unittest.TestCase):
    def setUp(self):
        self.root = tempfile.mkdtemp()
        job(self.root, "4096", 91, "AMD EPYC 7763", [9.0, 9.1, 9.2], [10.0, 10.1, 10.2],
            [5.0, 5.0, 5.0])
        job(self.root, "4096", 105, "Intel Xeon 8573C", [20.0, 22.0, 24.0], [21.0, 23.0, 25.0],
            [4.0, 4.0, 4.0])
        write(self.root, "series/4096_base.txt", SERIES)
        write(self.root, "series/4096_head.txt", SERIES.replace("40.00", "38.00"))

    def test_each_tick_is_combined_only_as_a_ratio_to_its_own_reference_with_its_cpu(self):
        rows = combined(self.root, "4096")
        self.assertEqual([row["tick"] for row in rows], [91, 105])
        first, second = rows
        self.assertEqual(first["cpu"], "AMD EPYC 7763")
        self.assertAlmostEqual(first["reference"], 5.0)
        self.assertAlmostEqual(first["base"], 10.1 / 5.0)
        self.assertAlmostEqual(first["head"], 9.1 / 5.0)
        self.assertTrue(first["apart"])
        self.assertEqual(second["cpu"], "Intel Xeon 8573C")
        self.assertAlmostEqual(second["head"], 22.0 / 4.0)
        self.assertFalse(second["apart"])

    def test_the_report_has_each_tick_head_against_base_its_cpu_and_the_series_percentiles(self):
        text = "\n".join(report_size(self.root, "4096"))
        self.assertIn("tick 91 on AMD EPYC 7763, reference 5.00 ms", text)
        self.assertIn("tick 105 on Intel Xeon 8573C, reference 4.00 ms", text)
        self.assertIn("scrum: seek", text)
        self.assertIn("95th and 99th percentiles", text)
        self.assertIn("× the reference", text)
        self.assertIn("AMD EPYC 7763: 91; Intel Xeon 8573C: 105", text)

    def test_a_job_that_wrote_nothing_is_named_not_combined(self):
        os.makedirs(os.path.join(self.root, "inspect-4096-144"))
        text = "\n".join(report_size(self.root, "4096"))
        self.assertIn("tick 144: no runs", text)
        self.assertEqual([row["tick"] for row in combined(self.root, "4096")], [91, 105])


if __name__ == "__main__":
    unittest.main()
