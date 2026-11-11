#!/usr/bin/env python3
"""The bench summary's chart (spec 43 step 4): a Mermaid xychart of the last records' whole
tick (`tick`) as a ratio to the reference, in first-parent order, read from `metrics/bench/`
and the record a run just wrote. Below 1 is faster than the reference.

    bench_chart.py <folder or record> ... [--last 20]
"""
import json
import sys
from pathlib import Path

PATH = "tick"
HARNESS = "breach_tick_phases"
SET_ARG = "series=1"


def read(sources: list) -> list:
    found = []
    for source in map(Path, sources):
        files = sorted(source.glob("*.json")) if source.is_dir() else [source]
        for path in files:
            if path.is_file():
                found.append(json.loads(path.read_text(encoding="utf-8")))
    return found


def _number(value):
    return value if isinstance(value, (int, float)) and not isinstance(value, bool) else None


def points(records: list, last: int = 20) -> list:
    """(short sha, ratio) for each record of the harness with a tick ratio, by position."""
    found = []
    for record in records:
        commit, source = record.get("commit") or {}, record.get("source") or {}
        ratio = _number(((record.get("reference") or {}).get("ratio") or {}).get(PATH))
        where = _number(commit.get("position"))
        in_set = SET_ARG in (source.get("args") or [])  # the 25k set's (step 8a), not this chart's
        if source.get("harness") == HARNESS and ratio is not None and where is not None \
                and not in_set:
            found.append((where, str(commit.get("sha", ""))[:7], ratio))
    found.sort()
    return [(sha, ratio) for _, sha, ratio in found[-last:]]


def chart(found: list, reference: str) -> list:
    if not found:
        return ["No bench records with a tick ratio yet."]
    top = max(1.2, max(ratio for _, ratio in found) * 1.1)
    labels = ", ".join(f'"{sha}"' for sha, _ in found)
    values = ", ".join(str(round(ratio, 4)) for _, ratio in found)
    return [
        "```mermaid",
        "xychart-beta",
        f'  title "Whole tick, ratio to the reference {reference} (below 1 is faster)"',
        f"  x-axis [{labels}]",
        f'  y-axis "ratio" 0 --> {round(top, 2)}',
        f"  line [{values}]",
        "```",
    ]


def main(argv: list) -> int:
    last = int(argv[argv.index("--last") + 1]) if "--last" in argv else 20
    sources = [a for i, a in enumerate(argv) if a != "--last" and (i == 0 or argv[i - 1] != "--last")]
    records = read(sources)
    found = points(records, last)
    reference = next((str((r.get("reference") or {}).get("commit", ""))[:7]
                      for r in reversed(records) if r.get("reference")), "")
    print("\n".join(chart(found, reference)))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
