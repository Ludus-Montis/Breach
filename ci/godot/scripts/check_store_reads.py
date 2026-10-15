#!/usr/bin/env python3
"""store-reads-once, per the Breach design ledger (Decision 144, principles/bounded-work.md
Principle 3).

The unit store (spec 35) holds each unit's per-tick state as columns. Under Rust every
scattered read or write of one unit is a call into the native field, so a loop over units
that reads the store unit by unit costs a boundary crossing per unit: spec 35 step 4a's
place reads cost a 22.6% slower tick that way. Flags, in simulation code (`sim/`), a
scattered store call inside a `for` or `while` loop:
- `.value(` or `.put(` on the store's columns,
- `UnitPlace.of(`, `UnitPlace.rank(`, `UnitPlace.put(`,
- `SquadEntries.loose_at(`, `EntryKeys.held(`, `EntryKeys.goal(`, `EntryKeys.keep(`,
  `EntryKeys.forget(`,
unless the line, or the one above it, or the loop's own line (or the one above that),
carries `# store: scattered (<why>)`: a loop over a few units (an event's, a trade's
two), the GDScript reference's per-unit step where the Rust engine runs a native pass,
or one the bench shows doesn't matter. The reason is required and reviewed. A pass over many units fetches the column once
(`columns.column(...)`, `UnitPlace.table(...)`), indexes it by slot, and commits once.

Heuristic limit: a read hidden inside a helper the loop calls isn't seen (4a's were).
Helpers that read one unit take an optional fetched table; the bench is the backstop.

Usage: check_store_reads.py [files...]  (no files: every .gd under sim/)
"""
import re
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(
    subprocess.check_output(["git", "rev-parse", "--show-toplevel"], text=True).strip()
)
READ = re.compile(
    r"(\bcolumns\.(value|put)\(|\.columns\.(value|put)\(|\bUnitPlace\.(of|rank|put)\("
    r"|\bSquadEntries\.loose_at\(|\bEntryKeys\.(held|goal|keep|forget)\()"
)
LOOP = re.compile(r"^\s*(for|while)\b")
MARK = re.compile(r"#\s*store:\s*scattered\s*\(\s*[^)\s]")
# The store's own definitions, which every read goes through.
OWN_FILES = {"unit_columns.gd", "unit_columns_native.gd", "unit_place.gd", "entry_keys.gd"}


def violations_in(text: str) -> list:
    found = []
    lines = text.splitlines()
    loops = []  # [indentation, marked] of each enclosing loop
    for number, line in enumerate(lines, start=1):
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            continue
        indent = len(line) - len(line.lstrip("\t"))
        while loops and loops[-1][0] >= indent:
            loops.pop()
        if stripped.startswith("func ") or stripped.startswith("static func "):
            loops.clear()
        code = line.split("#", 1)[0]
        marked = MARK.search(line) or (number > 1 and MARK.search(lines[number - 2]))
        header = LOOP.match(line) is not None  # its own header runs once: outer loops count
        if READ.search(code) is not None and loops:
            if not (marked or any(loop[1] for loop in loops)):
                found.append((number, "a scattered store call inside a loop over units"))
        if header:
            loops.append([indent, bool(marked)])
    return found


def files_to_check(arguments: list) -> list:
    if arguments:
        paths = [REPO_ROOT / a for a in arguments if a.startswith("sim/") and a.endswith(".gd")]
    else:
        paths = sorted((REPO_ROOT / "sim").rglob("*.gd"))
    return [p for p in paths if p.name not in OWN_FILES]


def main(arguments: list) -> int:
    report = []
    for path in files_to_check(arguments):
        if not path.exists():
            continue
        for number, why in violations_in(path.read_text(encoding="utf-8")):
            report.append(f"  {path.relative_to(REPO_ROOT)}:{number}: {why}")
    if report:
        print("check_store_reads: store read unit by unit inside a loop (Decision 144):")
        print("\n".join(report))
        print("Fetch the column once before the loop (columns.column, UnitPlace.table) and")
        print("commit once after, or mark a loop over a few units '# store: scattered (<why>)'.")
        return 1
    print("check_store_reads: OK")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
