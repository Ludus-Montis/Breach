"""Step 6: check extracted records against DECISIONS_INDEX.md, the answer key.

    python3 tools/decisions-extract/calibrate.py RECORDS_JSON

RECORDS_JSON is what `batch.py --collect ... --records` writes: accepted records keyed
by Decision number. No API call.

Two things happen here, in this order:
1. fill_from_peers(): deterministic post-processing. A Decision that is superseded
   often doesn't say so in its own excerpt; only the later Decision does ("supersedes
   62"). No retry can fix that, but the other records in the batch can.
2. compare(): each checkable field against the index.
"""

import json
import re
import sys
from pathlib import Path

from excerpt import INDEX

CHECKED = ("title", "status", "superseded_by", "supersedes")

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

    Returns one line per change. Only links between records in this set can be filled:
    a Decision superseded by one outside the batch stays wrong.
    """
    changes = []
    for number in sorted(records):
        for earlier in records[number]["supersedes"]:
            target = records.get(earlier)
            if target is None or number in target["superseded_by"]:
                continue
            target["superseded_by"] = sorted(target["superseded_by"] + [number])
            target["status"] = "superseded"
            target.setdefault("filled", []).extend(["superseded_by", "status"])
            changes.append(f"{earlier}: superseded_by += {number} (from Decision {number}'s supersedes)")
    return changes


def compare(records: dict[int, dict], key: dict[int, dict]) -> list[dict]:
    """One row per (Decision, checked field): extracted value, expected value, correct?"""
    rows = []
    for number in sorted(records):
        record, expected = records[number], key[number]
        for name in CHECKED:
            got, want = record[name], expected[name]
            if isinstance(want, list):
                got, want = sorted(got), sorted(want)
            rows.append({"decision": number, "field": name, "got": got, "want": want, "correct": got == want})
    return rows


def main() -> None:
    records = {int(n): r for n, r in json.loads(Path(sys.argv[1]).read_text(encoding="utf-8")).items()}
    key = answer_key()
    for line in fill_from_peers(records):
        print("filled", line)
    wrong = [r for r in compare(records, key) if not r["correct"]]
    for row in wrong:
        print(f"wrong  {row['decision']:>3} {row['field']}: got {row['got']!r}, index says {row['want']!r}")
    print(f"{len(wrong)} wrong fields in {len(records)} records")


if __name__ == "__main__":
    main()
