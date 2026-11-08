#!/usr/bin/env python3
"""A scenario's timings for the showcase (spec 44 step 3): over the rounds of
`breach_tick_phases scenario=<name>` (each printed as `series=1` prints, spec 44 "Settled in
2b"), the median of the tick's mean, the median of its 95th percentile, and each phase's
median mean, in ms.

Usage: timings.py <out.json> <round.txt>...
"""
import json
import statistics
import sys


def parse(text: str) -> dict:
    """One round's series block: the tick's mean and 95th, and each phase's mean (ms)."""
    lines = text.splitlines()
    start = next((i for i, line in enumerate(lines) if line.startswith("series:")), None)
    if start is None:
        raise ValueError("no series in breach_tick_phases' output")
    tick, phases = None, {}
    for line in lines[start + 1:]:
        parts = line.rsplit(None, 5)  # a phase's name may hold spaces
        if len(parts) != 6 or not line.startswith("  "):
            break
        name, mean, p95 = parts[0].strip(), float(parts[1]), float(parts[2])
        if name == "tick":
            tick = {"mean": mean, "p95": p95}
        else:
            phases[name] = mean
    if tick is None:
        raise ValueError("no tick row in the series")
    return {"tick": tick, "phases": phases}


def summarise(texts: list) -> dict:
    rounds = [parse(text) for text in texts]
    names = list(rounds[0]["phases"])
    return {
        "rounds": len(rounds),
        "tick_median_ms": statistics.median(r["tick"]["mean"] for r in rounds),
        "tick_p95_ms": statistics.median(r["tick"]["p95"] for r in rounds),
        "phases_median_ms": {
            name: statistics.median(r["phases"].get(name, 0.0) for r in rounds) for name in names
        },
    }


def main(arguments: list) -> int:
    if len(arguments) < 2:
        print(__doc__.strip(), file=sys.stderr)
        return 2
    texts = [open(path, encoding="utf-8").read() for path in arguments[1:]]
    with open(arguments[0], "w", encoding="utf-8") as out:
        json.dump(summarise(texts), out, indent=1)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
