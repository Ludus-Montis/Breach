#!/usr/bin/env python3
"""one-mover, per the Breach design ledger (Decision 139).

Units move through one mover (sim/skirmish/formation/unit_mover.gd), which pays the
ground. Twelve files once wrote positions themselves, and those that didn't pay the
ground let routers and retreats cross woods and fords at full pace. Flags, in simulation
code (`sim/`), a write to a unit's position - `.position =`, `["at"]`, `["offset"]`
or `["along"]` of a unit's entry (and their `+=`/`-=`), or a write to one of the unit
store's place columns - POSITION, LOOSE_AT, FLEE_OFFSET or ALONG (`put(..., UnitColumns.POSITION,
...)`, `commit(UnitColumns.LOOSE_AT, ...)`; spec 35) - unless:
- the value comes from the mover (`UnitMover.` on the line, or a column write of a
  variable the line before set from `UnitMover.`), or
- the line carries `# paid: <how>` (a mover keeping its own coordinates that scaled its
  step by UnitMover.share) in a file that calls UnitMover, or
- the line, or the one above it, carries `# placement: <why>`: not a move but a
  placement (a carried body on its bearer, a frame's slot, bodies parted, a native step
  paid elsewhere). The reason is required and reviewed.

Usage: check_one_mover.py [files...]  (no files: every .gd under sim/)
"""
import re
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(
    subprocess.check_output(["git", "rev-parse", "--show-toplevel"], text=True).strip()
)
WRITE = re.compile(r'(\.position|(\bentry|\])\["(at|offset|along)"\])\s*[-+]?=(?!=)')
PLACES = r"UnitColumns\.(?:POSITION|LOOSE_AT|FLEE_OFFSET|ALONG)\b"
COLUMN_WRITE = re.compile(rf"\b(?:put\([^,]+,\s*{PLACES},\s*(\w+)?|commit\(\s*{PLACES})")
PLACEMENT = re.compile(r"#\s*placement:\s*\S")
PAID = re.compile(r"#\s*paid:\s*\S")
MOVER_FILE = "unit_mover.gd"


def violations_in(text: str) -> list:
    found = []
    lines = text.splitlines()
    calls_mover = any("UnitMover." in line.split("#", 1)[0] for line in lines)
    for number, line in enumerate(lines, start=1):
        code = line.split("#", 1)[0]
        column = COLUMN_WRITE.search(code)
        if not (WRITE.search(code) or column) or "UnitMover." in code:
            continue
        if column and column.group(1) and number > 1:
            above = lines[number - 2].split("#", 1)[0]
            if re.search(rf"\b{column.group(1)}\s*:?=.*UnitMover\.", above):
                continue
        if PLACEMENT.search(line) or (number > 1 and PLACEMENT.search(lines[number - 2])):
            continue
        if PAID.search(line) and calls_mover:
            continue
        found.append((number, "a position written outside the mover"))
    return found


def files_to_check(arguments: list) -> list:
    if arguments:
        paths = [REPO_ROOT / a for a in arguments if a.startswith("sim/") and a.endswith(".gd")]
    else:
        paths = sorted((REPO_ROOT / "sim").rglob("*.gd"))
    return [p for p in paths if p.name != MOVER_FILE]


def main(arguments: list) -> int:
    report = []
    for path in files_to_check(arguments):
        if not path.exists():
            continue
        for number, why in violations_in(path.read_text(encoding="utf-8")):
            report.append(f"  {path.relative_to(REPO_ROOT)}:{number}: {why}")
    if report:
        print("check_one_mover: units moved outside the mover (Decision 139):")
        print("\n".join(report))
        print("Move with UnitMover (it pays the ground), or mark a placement with")
        print("'# placement: <why>'.")
        return 1
    print("check_one_mover: OK")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
