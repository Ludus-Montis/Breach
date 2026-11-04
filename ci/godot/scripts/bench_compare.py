#!/usr/bin/env python3
"""Compares a branch with its base over alternating bench rounds (`bench.yml`'s `rounds`).

One bench run can't tell a 5% change from the swing any change of code brings: a run of
one commit against itself repeats to within 1%, but two commits that differ move phases
neither touched by 5 to 20% (spec 35 step 5b). So `bench` runs head and base in turn,
several times on one runner, and this reads each run's output (`tools/tick_phases.gd`: a
"... ms a tick" line, then one "  <phase>  <ms>" line per phase) and prints, per phase, each
side's median and range and the change in the medians. A phase is marked `*` when the two
ranges don't overlap: every head round faster, or every one slower, than every base round.
A `series=1` run (`breach_tick_phases`, spec 38 step 2) carries its means on those same lines
and its percentiles after a "series:" line; when every run has them, each phase's 95th and
99th are compared too, by their medians.
Two sides that don't differ fall apart that way by chance 2 times in 20 with 3 rounds each
(about 4 of tick_phases' 40 phases a run), 2 in 252 with 5: use 5 or more to judge.

    bench_compare.py --head head_1.txt head_2.txt ... --base base_1.txt base_2.txt ...
"""
import re
import statistics
import sys

TICK = re.compile(r": (\d+(?:\.\d+)?) ms a tick$")
PHASE = re.compile(r"^  (\S.*?)\s+(-?\d+\.\d+)$")
STOP = "copies a tick"  # tick_phases' copy count follows the phases: not times
SERIES = "series:"  # a series run's percentiles follow its means
SPREAD = re.compile(
    r"^  (\S.*?)\s+(-?\d+\.\d+)\s+(-?\d+\.\d+)\s+(-?\d+\.\d+)\s+(-?\d+\.\d+)\s+(\d+)$"
)


def phases(text):
    """{"tick": ms, phase: ms, ...} read from one run's output."""
    out = {}
    for line in text.splitlines():
        if line.startswith(STOP) or line.startswith(SERIES):
            break
        tick = TICK.search(line)
        if tick:
            out["tick"] = float(tick.group(1))
            continue
        phase = PHASE.match(line)
        if phase:
            out[phase.group(1)] = float(phase.group(2))
    return out


def spreads(text):
    """{"tick": (95th, 99th), phase: (95th, 99th), ...} read from a series run's output; {}
    when it has no series."""
    out = {}
    lines = text.splitlines()
    start = next((i for i, line in enumerate(lines) if line.startswith(SERIES)), len(lines))
    for line in lines[start + 1:]:
        spread = SPREAD.match(line)
        if not spread:
            break
        out[spread.group(1)] = (float(spread.group(3)), float(spread.group(4)))
    return out


def compare_spreads(head_runs, base_runs):
    """One row per phase both sides have: each side's median 95th and 99th, the tick first,
    then the slowest 95th on the base."""
    shared = set.intersection(*(set(run) for run in head_runs + base_runs))
    rows = []
    for name in shared:
        head = tuple(statistics.median(run[name][at] for run in head_runs) for at in (0, 1))
        base = tuple(statistics.median(run[name][at] for run in base_runs) for at in (0, 1))
        rows.append({"phase": name, "head": head, "base": base})
    rows.sort(key=lambda row: (row["phase"] != "tick", -row["base"][0], row["phase"]))
    return rows


def report_spreads(rows):
    """The percentiles' comparison as lines to print."""
    lines = [
        "95th and 99th percentiles, medians over the rounds (ms):",
        f"  {'phase':<36} {'base 95th':>9} {'head 95th':>9} {'base 99th':>9} {'head 99th':>9}",
    ]
    for row in rows:
        lines.append(
            f"  {row['phase']:<36} {row['base'][0]:9.2f} {row['head'][0]:9.2f}"
            f" {row['base'][1]:9.2f} {row['head'][1]:9.2f}"
        )
    return lines


def compare(head_runs, base_runs):
    """One row per phase both sides have: the tick first, then the slowest on the base."""
    shared = set.intersection(*(set(run) for run in head_runs + base_runs))
    rows = []
    for name in shared:
        head = [run[name] for run in head_runs]
        base = [run[name] for run in base_runs]
        head_median = statistics.median(head)
        base_median = statistics.median(base)
        rows.append({
            "phase": name,
            "head": head_median,
            "base": base_median,
            "change": (head_median - base_median) / base_median * 100.0 if base_median else 0.0,
            "head_range": (min(head), max(head)),
            "base_range": (min(base), max(base)),
            "apart": max(head) < min(base) or min(head) > max(base),
        })
    rows.sort(key=lambda row: (row["phase"] != "tick", -row["base"], row["phase"]))
    return rows


def report(rows, rounds):
    """The comparison as lines to print."""
    lines = [
        f"medians over {rounds} rounds, head and base in turn (ms; * = ranges apart):",
        f"  {'phase':<36} {'base':>8} {'head':>8} {'change':>8}   {'base range':>15}"
        f"   {'head range':>15}",
    ]
    for row in rows:
        base_range = "%.2f-%.2f" % row["base_range"]
        head_range = "%.2f-%.2f" % row["head_range"]
        lines.append(
            f"  {row['phase']:<36} {row['base']:8.2f} {row['head']:8.2f}"
            f" {row['change']:+7.1f}%   {base_range:>15}   {head_range:>15}"
            + ("  *" if row["apart"] else "")
        )
    return lines


def main(argv):
    if "--head" not in argv or "--base" not in argv:
        print(__doc__)
        return 2
    at_base = argv.index("--base")
    head_files = argv[argv.index("--head") + 1:at_base]
    base_files = argv[at_base + 1:]
    head_runs = [phases(open(path, encoding="utf-8").read()) for path in head_files]
    base_runs = [phases(open(path, encoding="utf-8").read()) for path in base_files]
    if not head_runs or not base_runs or not all(head_runs + base_runs):
        print("bench_compare: a run printed no phases; read its raw output")
        return 1
    print("\n".join(report(compare(head_runs, base_runs), len(head_runs))))
    head_spreads = [spreads(open(path, encoding="utf-8").read()) for path in head_files]
    base_spreads = [spreads(open(path, encoding="utf-8").read()) for path in base_files]
    if all(head_spreads + base_spreads):
        print("\n".join(report_spreads(compare_spreads(head_spreads, base_spreads))))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
