#!/usr/bin/env python3
"""one-health, per the Breach design ledger (Decision 145, spec 35 step 5b).

Every change to a unit's hp goes through one writer, UnitHealth
(sim/skirmish/formation/unit_health.gd): a write that takes a unit from above 0 to 0 or
below puts it on the unit store's fell queue, which FormationDeaths.bury drains in place of
scanning every living unit. A write that skips UnitHealth would leave a fallen unit off the
queue, and it would never be buried. Flags, in simulation code (`sim/`), a write to the
store's HP column - `put(..., UnitColumns.HP, ...)`, `commit(UnitColumns.HP, ...)`, or a
write into a fetched UnitHealth table (`.hp[...] =`, `+=`, `-=`) - outside UnitHealth.

Usage: check_one_health.py [files...]  (no files: every .gd under sim/)
"""
import re
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(
    subprocess.check_output(["git", "rev-parse", "--show-toplevel"], text=True).strip()
)
COLUMN_WRITE = re.compile(r"\b(?:put\([^,]+,\s*UnitColumns\.HP\b|commit\(\s*UnitColumns\.HP\b)")
TABLE_WRITE = re.compile(r"\.hp\[[^\]]+\]\s*[-+]?=(?!=)")
WRITER = "unit_health.gd"


def violations_in(text: str) -> list:
    found = []
    for number, line in enumerate(text.splitlines(), start=1):
        code = line.split("#", 1)[0]
        if COLUMN_WRITE.search(code) or TABLE_WRITE.search(code):
            found.append((number, "hp written outside UnitHealth"))
    return found


def files_to_check(arguments: list) -> list:
    if arguments:
        paths = [REPO_ROOT / a for a in arguments if a.startswith("sim/") and a.endswith(".gd")]
    else:
        paths = sorted((REPO_ROOT / "sim").rglob("*.gd"))
    return [p for p in paths if p.name != WRITER]


def main(arguments: list) -> int:
    report = []
    for path in files_to_check(arguments):
        if not path.exists():
            continue
        for number, why in violations_in(path.read_text(encoding="utf-8")):
            report.append(f"  {path.relative_to(REPO_ROOT)}:{number}: {why}")
    if report:
        print("check_one_health: hp written outside UnitHealth (Decision 145):")
        print("\n".join(report))
        print("Write hp with UnitHealth.hurt, heal or put: it puts the fallen on the fell queue.")
        return 1
    print("check_one_health: OK")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
