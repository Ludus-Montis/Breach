"""Step 6: calibrate Claude's per-field confidence against DECISIONS_INDEX.md, and route.

    python3 tools/decisions-extract/calibrate.py RECORDS_JSON

RECORDS_JSON is what `batch.py --collect ... --records` writes: accepted records keyed
by Decision number. No API call.

1. compare() + calibration(): Claude's own records, before any post-processing,
   against the index (the labelled set). Accuracy per (field, confidence).
2. fill_from_peers(): deterministic post-processing. A superseded Decision often
   doesn't say so in its own excerpt; only the later one does ("supersedes 62").
   No retry can fix that, but the other records in the batch can.
3. route(): which fields a person checks. In production there is no index for new
   Decisions, so routing uses only the confidence, the calibration and the signals.
"""

import copy
import json
import re
import sys
from collections import defaultdict
from pathlib import Path

from excerpt import INDEX
from schema import CONFIDENCE_FIELDS, CONFIDENCE_LEVELS

CHECKED = ("title", "status", "superseded_by", "supersedes")  # the fields the index can check

# STUDY 5.5: a (field, confidence) cell skips review only if calibration shows it is
# right often enough, on enough samples. A bucket that is never measured, or measured
# on 3 fields, has earned nothing.
TARGET = 0.95
MIN_N = 10

_ROW = re.compile(r"^\|\s*(\d+)\s*\|(.*)\|\s*\d+\s*\|([^|]*)\|\s*$")


def _numbers(text: str, verb: str) -> list[int]:
    match = re.search(rf"{verb} by ([\d, ]+)", text)
    return sorted(int(n) for n in match.group(1).split(",") if n.strip()) if match else []


def answer_key() -> dict[int, dict]:
    """The checkable fields of every Decision, from the index."""
    key = {}
    for line in INDEX.read_text(encoding="utf-8").splitlines():
        match = _ROW.match(line)
        if match:
            number, status = int(match.group(1)), match.group(3).strip()
            later = _numbers(status, "superseded")
            key[number] = {
                "title": match.group(2).strip(),
                "status": "superseded" if later else "active",
                "superseded_by": later,
                "supersedes": [],
            }
    for number, fields in key.items():
        for later in fields["superseded_by"]:
            key[later]["supersedes"].append(number)
    return key


def fill_from_peers(records: dict[int, dict]) -> list[str]:
    """Add the superseded_by links that only the later record states. Edits in place.

    Returns one line per change, and lists the changed fields in record["filled"].
    Only links between records in this set can be filled: a Decision superseded by
    one outside the batch stays wrong.
    """
    changes = []
    for number in sorted(records):
        for earlier in records[number]["supersedes"]:
            target = records.get(earlier)
            if target is None or number in target["superseded_by"]:
                continue
            target["superseded_by"] = sorted(target["superseded_by"] + [number])
            target["status"] = "superseded"
            target.setdefault("filled", {}).update(superseded_by=number, status=number)
            changes.append(f"{earlier}: superseded_by += {number} (from Decision {number}'s supersedes)")
    return changes


def compare(records: dict[int, dict], key: dict[int, dict]) -> list[dict]:
    """One row per (Decision, checked field): value, expected value, confidence, correct?"""
    rows = []
    for number in sorted(records):
        record, expected = records[number], key[number]
        for name in CHECKED:
            got, want = record[name], expected[name]
            if isinstance(want, list):
                got, want = sorted(got), sorted(want)
            rows.append({
                "decision": number,
                "field": name,
                "confidence": record["confidence"][name],
                "got": got,
                "want": want,
                "correct": got == want,
            })
    return rows


def calibration(rows: list[dict]) -> dict[tuple[str, str], list[int]]:
    """(field, confidence) -> [fields, correct]."""
    cells = defaultdict(lambda: [0, 0])
    for row in rows:
        cell = cells[(row["field"], row["confidence"])]
        cell[0] += 1
        cell[1] += row["correct"]
    return dict(cells)


def by_bucket(cells: dict[tuple[str, str], list[int]]) -> dict[str, list[int]]:
    """confidence -> [fields, correct], over all checked fields."""
    totals = {level: [0, 0] for level in CONFIDENCE_LEVELS}
    for (_, level), (n, correct) in cells.items():
        totals[level][0] += n
        totals[level][1] += correct
    return totals


def trusted(cells: dict[tuple[str, str], list[int]]) -> set[tuple[str, str]]:
    return {cell for cell, (n, correct) in cells.items() if n >= MIN_N and correct / n >= TARGET}


def route(records: dict[int, dict], trust: set[tuple[str, str]]) -> dict[int, dict[str, str]]:
    """Decision -> {field: why a person checks it}. Decisions with nothing to check are left out."""
    review = {}
    for number in sorted(records):
        record, reasons = records[number], {}
        for name in CONFIDENCE_FIELDS:
            level = record["confidence"][name]
            if name not in CHECKED:
                reasons[name] = "no labelled data to calibrate this field"
            elif (name, level) not in trust:
                reasons[name] = f"{level} confidence is not calibrated to {TARGET:.0%} on this field"
        # STUDY 5.5 (option D's signals): evidence from the pipeline, not from Claude.
        for name, source in record.get("filled", {}).items():
            reasons[name] = f"filled by post-processing from Decision {source}"
        if record.get("followed_up"):
            reasons["record"] = "needed a validation follow-up"
        if reasons:
            review[number] = reasons
    return review


def table(cells: dict[tuple[str, str], list[int]]) -> str:
    """Markdown: accuracy by field and confidence, then by confidence alone."""

    def cell(n: int, correct: int) -> str:
        return f"{correct}/{n} ({correct / n:.0%})" if n else "-"

    lines = ["| Field | " + " | ".join(CONFIDENCE_LEVELS) + " |", "|---" * (len(CONFIDENCE_LEVELS) + 1) + "|"]
    for name in CHECKED:
        lines.append(f"| {name} | " + " | ".join(cell(*cells.get((name, lv), [0, 0])) for lv in CONFIDENCE_LEVELS) + " |")
    totals = by_bucket(cells)
    lines.append("| **all** | " + " | ".join(cell(*totals[lv]) for lv in CONFIDENCE_LEVELS) + " |")
    return "\n".join(lines)


def main() -> None:
    records = {int(n): r for n, r in json.loads(Path(sys.argv[1]).read_text(encoding="utf-8")).items()}
    key = answer_key()

    cells = calibration(compare(copy.deepcopy(records), key))
    print(table(cells))
    trust = trusted(cells)
    print(f"\ntrusted (>= {TARGET:.0%} on >= {MIN_N}): {sorted(trust) or 'none'}")

    for line in fill_from_peers(records):
        print("filled", line)
    still_wrong = [r for r in compare(records, key) if not r["correct"]]
    for row in still_wrong:
        print(f"wrong after fill: {row['decision']} {row['field']} ({row['confidence']}): "
              f"got {row['got']!r}, index says {row['want']!r}")

    review = route(records, trust)
    print(f"\nreview: {len(review)} of {len(records)} records")
    for number, reasons in review.items():
        print(f"  {number}: " + "; ".join(f"{k}: {v}" for k, v in reasons.items()))


if __name__ == "__main__":
    main()
