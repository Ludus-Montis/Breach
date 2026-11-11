#!/usr/bin/env python3
"""The 25k set (spec 43 step 8a, Decision 165): the shapes `record-25k` times on each merge,
the fit of `t(x) = b·x + c·x²` on each shape's grid (`x` units a side, `b` and `c` at least
0), and the data PR's summary: each shape's miss at 24,576 a side, (predicted − measured) ÷
measured, flagged beyond 10%, and "the form no longer fits" when the merge before on main's
line was flagged on that shape too. The fit is never stored: the page (step 8c) computes it
from the records, from the same fixtures (`fixtures/bench_fit.json`).

    bench_set.py shapes                       one line a shape: its args, then `bridge` or `-`
    bench_set.py summary <folder> ... --new <folder>   the summary of the records in --new,
                                              against the history in the folders before it
"""
import json
import sys
from pathlib import Path

SERIES_ARG = "series=1"
TOLERANCE = 0.10
FAMILIES = {
    "lanes": {"grid": ["side=1024 squads=1", "side=4096 squads=4", "side=8192 squads=8"],
              "check": "side=24576 squads=24"},
    "one squad": {"grid": ["side=2048 squads=1", "side=4096 squads=1", "side=8192 squads=1",
                           "side=12288 squads=1"],
                  "check": "side=24576 squads=1"},
}


def shapes() -> list:
    """[(args, beside the bridge)]: the grid first, then the checks, each once."""
    grid = [shape for family in FAMILIES.values() for shape in family["grid"]]
    checks = [family["check"] for family in FAMILIES.values()]
    unique = list(dict.fromkeys(grid))
    return [(shape, False) for shape in unique] + [(shape, True) for shape in checks]


def side(shape: str) -> int:
    return int(dict(arg.split("=", 1) for arg in shape.split())["side"])


def fit(points: list) -> tuple:
    """(b, c) of b·x + c·x², each at least 0, least squares over [(x, t)]."""
    sxx = sum(x ** 2 for x, _ in points)
    sx3 = sum(x ** 3 for x, _ in points)
    sx4 = sum(x ** 4 for x, _ in points)
    sxt = sum(x * t for x, t in points)
    sx2t = sum(x * x * t for x, t in points)
    candidates = [(0.0, 0.0)]
    if sxx:
        candidates.append((max(sxt / sxx, 0.0), 0.0))
    if sx4:
        candidates.append((0.0, max(sx2t / sx4, 0.0)))
    det = sxx * sx4 - sx3 * sx3
    if det:
        b, c = (sxt * sx4 - sx2t * sx3) / det, (sx2t * sxx - sxt * sx3) / det
        if b >= 0 and c >= 0:
            candidates.append((b, c))

    def squares(bc):
        return sum((bc[0] * x + bc[1] * x * x - t) ** 2 for x, t in points)

    return min(candidates, key=squares)


def predict(b: float, c: float, x: float) -> float:
    return b * x + c * x * x


def _shape(record: dict):
    """The set's shape a record times, or None."""
    source = record.get("source") or {}
    args = [str(arg) for arg in source.get("args") or []]
    if SERIES_ARG not in args or (record.get("provenance") or {}).get("kind") != "fresh":
        return None
    return " ".join(arg for arg in args if arg != SERIES_ARG)


def _tick(record: dict):
    median = ((record.get("results") or {}).get("tick") or {}).get("median")
    return median if isinstance(median, (int, float)) and not isinstance(median, bool) else None


def by_commit(records: list) -> dict:
    """{position: {shape: mean tick}}, a commit's latest run winning."""
    found, runs = {}, {}
    for record in records:
        shape, tick = _shape(record), _tick(record)
        position = (record.get("commit") or {}).get("position")
        if shape is None or tick is None or not isinstance(position, int):
            continue
        run = (record.get("provenance") or {}).get("run_id") or 0
        if run >= runs.get((position, shape), -1):
            runs[(position, shape)] = run
            found.setdefault(position, {})[shape] = tick
    return found


def misses(ticks: dict) -> dict:
    """{family: (predicted, measured, miss)} for each family the commit's ticks cover."""
    out = {}
    for name, family in FAMILIES.items():
        points = [(side(shape), ticks[shape]) for shape in family["grid"] if shape in ticks]
        measured = ticks.get(family["check"])
        if len(points) < 2 or not measured:
            continue
        b, c = fit(points)
        predicted = predict(b, c, side(family["check"]))
        out[name] = (predicted, measured, (predicted - measured) / measured)
    return out


def summary(new: list, history: list) -> list:
    """The data PR's lines for the commit(s) in `new`, each against the merge before it."""
    known = by_commit(history)
    fresh = by_commit(new)
    lines = []
    for position in sorted(fresh):
        before = [p for p in known if p < position]
        previous = misses(known[max(before)]) if before else {}
        for name, (predicted, measured, miss) in misses(fresh[position]).items():
            flagged = abs(miss) > TOLERANCE
            line = (f"{name}: miss {miss:+.1%} (predicted {predicted:.2f} ms, measured "
                    f"{measured:.2f} ms at 24,576 a side)")
            if flagged:
                line += f" FLAG: beyond {TOLERANCE:.0%}"
            lines.append(line)
            if flagged and name in previous and abs(previous[name][2]) > TOLERANCE:
                lines.append(f"{name}: the form no longer fits: the merge before was flagged too "
                             "(Decision 165: raise a `question` issue to reopen the form)")
    return lines or ["No 25k set in these records."]


def read(folders: list) -> list:
    out = []
    for folder in map(Path, folders):
        for path in sorted(folder.rglob("*.json")) if folder.is_dir() else []:
            out.append(json.loads(path.read_text(encoding="utf-8")))
    return out


def main(argv: list) -> int:
    if argv[:1] == ["shapes"]:
        for shape, bridge in shapes():
            print(f"{shape}\t{'bridge' if bridge else '-'}")
        return 0
    if argv[:1] == ["summary"] and "--new" in argv:
        at = argv.index("--new")
        print("\n".join(summary(read(argv[at + 1:at + 2]), read(argv[1:at]))))
        return 0
    print(__doc__)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
