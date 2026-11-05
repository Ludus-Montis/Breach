#!/usr/bin/env python3
"""Checks that a small bench battle can stand in for a large one (spec 39 step 10a, Decision
159): `breach_tick_phases` run at both sizes, each over several rounds (`bench.yml` with
`base_args` and `scale`), and each phase's median at the large size set over its median at the
small one. The tick grows linearly with the number of squads when every squad is the same
size (4096 a side as 4 squads of 1,024, 24,576 as 24): its expected growth, `--factor`, is the
ratio of the squads (6), as `breach_scaling` shows one squad's size isn't what changes. The
small battle stands in when the tick's growth is within a fifth of the factor either way. A
phase that grew more than a fifth past it is marked `^`: work that doesn't scale with the
squads, for 10b to find.

    bench_scaling.py --factor 6 --large head_1.txt ... --small base_1.txt ...
"""
import statistics
import sys

from bench_compare import phases

BAND = 1.2  # how far the growth may be from the factor, either way, as a ratio


def grown(large_texts, small_texts):
    """[(phase, small median ms, large median ms, ratio)], the tick first, then the phases
    largest first at the large size; a phase either side lacks in any run is left out."""
    large = [phases(text) for text in large_texts]
    small = [phases(text) for text in small_texts]
    names = set.intersection(*(set(run) for run in large + small))
    rows = []
    for name in names:
        before = statistics.median(run[name] for run in small)
        after = statistics.median(run[name] for run in large)
        rows.append((name, before, after, after / before if before else float("inf")))
    rows.sort(key=lambda row: (row[0] != "tick", -row[2]))
    return rows


def stands_in(rows, factor):
    """Whether the tick grew within BAND of `factor`, either way."""
    tick = next(row for row in rows if row[0] == "tick")
    return factor / BAND <= tick[3] <= factor * BAND


def report(rows, factor):
    """The lines printed: each phase's medians and growth, `^` past the band, then the verdict."""
    lines = [f"scaling: large over small, expected x{factor:.2f} (within x{BAND:.2f} either way)"]
    lines.append(f"  {'phase':<36} {'small':>9} {'large':>9} {'grown':>7}")
    for name, before, after, ratio in rows:
        mark = " ^" if ratio > factor * BAND else ""
        lines.append(f"  {name:<36} {before:>9.2f} {after:>9.2f} {ratio:>6.2f}x{mark}")
    tick = next(row for row in rows if row[0] == "tick")[3]
    verdict = "stands in" if stands_in(rows, factor) else "doesn't stand in"
    lines.append(f"the tick grew x{tick:.2f} against x{factor:.2f}: the small battle {verdict}")
    return lines


def main(argv):
    if "--factor" not in argv or "--large" not in argv or "--small" not in argv:
        print(__doc__)
        return 2
    factor = float(argv[argv.index("--factor") + 1])
    at_large, at_small = argv.index("--large"), argv.index("--small")
    ends = sorted([at_large, at_small, argv.index("--factor"), len(argv)])
    large_files = argv[at_large + 1:ends[ends.index(at_large) + 1]]
    small_files = argv[at_small + 1:ends[ends.index(at_small) + 1]]
    texts = [[open(path, encoding="utf-8").read() for path in files]
             for files in (large_files, small_files)]
    if not all(texts) or not all(phases(text) for text in texts[0] + texts[1]):
        print("bench_scaling: a run printed no phases; read its raw output")
        return 1
    rows = grown(*texts)
    print("\n".join(report(rows, factor)))
    return 0 if stands_in(rows, factor) else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
