#!/usr/bin/env python3
"""The pair summary of the owner's PC (spec 48 step 5b): `local_grid.sh pair <head> <base>`
times both commits in turn at the base's `inspect:` ticks, one tick a run (the repeat mode,
spec 47), into files named `<size>_t<tick>_<head|base>_r<round>.txt`. This prints, per size
and tick, each phase's medians over the rounds, the head's change against the base, and `*`
where the rounds' ranges are apart, as `bench`'s summary does (`bench_compare.py`, reused).
When the folder holds `<size>_series_<head|base>.txt` (each side's settled series, run once),
their 95th and 99th percentiles follow.

    python3 ci/metrics/local_pair.py <folder>
"""
import re
import sys
from collections import defaultdict
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "godot" / "scripts"))
import bench_compare  # noqa: E402

PAIR_FILE = re.compile(r"^(?P<size>.+)_t(?P<tick>\d+)_(?P<side>head|base)_r(?P<round>\d+)\.txt$")
SERIES_FILE = re.compile(r"^(?P<size>.+)_series_(?P<side>head|base)\.txt$")
CAVEAT = ("The repeats are a guide, not the judge (Q47, #909): they under-report phases whose "
          "cost builds up across ticks (strays, groups, bodies: pushes, stamina); "
          "the whole tick is the judge.")


class Refused(Exception):
    pass


def gather(folder) -> dict:
    """{size: {tick: {"head": [text by round], "base": [...]}}}, the rounds in order. Raises
    Refused when a tick has one side only, or the folder has no pair outputs."""
    found = defaultdict(lambda: defaultdict(lambda: {"head": [], "base": []}))
    if not Path(folder).is_dir():
        raise Refused(f"no folder {folder}")
    for path in sorted(Path(folder).iterdir()):
        match = PAIR_FILE.match(path.name)
        if match:
            found[match["size"]][int(match["tick"])][match["side"]].append(
                (int(match["round"]), path.read_text(encoding="utf-8")))
    if not found:
        raise Refused(f"no pair outputs (<size>_t<tick>_<head|base>_r<round>.txt) in {folder}")
    out = {}
    for size, ticks in found.items():
        out[size] = {}
        for tick, sides in sorted(ticks.items()):
            for side in ("head", "base"):
                if not sides[side]:
                    raise Refused(f"{size}, tick {tick}: no {side} runs")
            out[size][tick] = {side: [text for _, text in sorted(runs)]
                               for side, runs in sides.items()}
    return out


def series(folder, size: str) -> dict:
    """{"head": text, "base": text} of a size's settled series, or {} without both."""
    texts = {}
    for path in Path(folder).iterdir():
        match = SERIES_FILE.match(path.name)
        if match and match["size"] == size:
            texts[match["side"]] = path.read_text(encoding="utf-8")
    return texts if len(texts) == 2 else {}


def summary(folder) -> list:
    """The pair's summary as lines to print."""
    lines = [CAVEAT]
    for size, ticks in sorted(gather(folder).items()):
        for tick, sides in ticks.items():
            head = [bench_compare.phases(text) for text in sides["head"]]
            base = [bench_compare.phases(text) for text in sides["base"]]
            if not all(head + base):
                raise Refused(f"{size}, tick {tick}: a run printed no phases; read its output")
            lines.append(f"== {size}, tick {tick}")
            lines += bench_compare.report(bench_compare.compare(head, base), len(head))
        texts = series(folder, size)
        spreads = {side: bench_compare.spreads(text) for side, text in texts.items()}
        if spreads and all(spreads.values()):
            lines.append(f"== {size}, the settled series (one run each)")
            lines += bench_compare.report_spreads(
                bench_compare.compare_spreads([spreads["head"]], [spreads["base"]]))
    return lines


def main(argv: list) -> int:
    if len(argv) != 1:
        print(__doc__)
        return 2
    try:
        print("\n".join(summary(argv[0])))
    except Refused as why:
        print(f"local_pair: refused: {why}")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
