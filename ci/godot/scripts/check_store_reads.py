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
- `UnitHealth.of_one(`, `UnitHealth.hurt_one(`, `UnitHealth.heal_one(`,
  `UnitHealth.put_one(` (hp's scattered forms; `of`, `hurt`, `heal`, `put` take a view),
- `FormationRout.where(`,
- `SquadEntries.loose_at(`, `EntryKeys.held(`, `EntryKeys.goal(`, `EntryKeys.keep(`,
  `EntryKeys.forget(`,
unless the line, or the one above it, or the loop's own line (or the one above that),
carries `# store: scattered (few: <why>)`, a loop over a few units (an event's, a trade's
two), or `# store: scattered (reference: <why>)`, the GDScript reference's per-unit step
where the Rust engine runs a native pass (Decision 146). Any other reason is flagged: a
bench's bound is no longer one. A pass over many units holds or reads one typed view
(`UnitView.hold(...)`, `UnitView.read(...)`), takes its arrays into typed locals and
indexes them by slot.

A pass also reads each column, and builds each view of the store, once a pass (Decision
150): it flags `UnitView.hold(`, `UnitView.read(` and `.column(` inside a loop, a copy of
the battle's column under Rust, unless the line, the one above it, or a loop it is in
carries `# store: per squad (<why>)`, a reason the bench has shown doesn't matter.
`UnitView.of(squad)` on a view held for the pass is an index, not a copy, and
`UnitView.read_once` reads once a store: neither is flagged.

Heuristic limit: a read hidden inside a helper the loop calls isn't seen (4a's were).
Helpers that read one unit take the view; the bench is the backstop.

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
    r"|\bUnitHealth\.(of_one|hurt_one|heal_one|put_one)\(|\bFormationRout\.where\("
    r"|\bSquadEntries\.loose_at\(|\bEntryKeys\.(held|goal|keep|forget)\()"
)
COPY = re.compile(r"(\bUnitView\.(hold|read)\(|\.column\()")
LOOP = re.compile(r"^\s*(for|while)\b")
MARK = re.compile(r"#\s*store:\s*scattered\s*\(\s*(few|reference):\s*[^)\s]")
PER_SQUAD = re.compile(r"#\s*store:\s*per squad\s*\(\s*[^)\s]")
# The store's own definitions, which every read goes through: the views and holds copy
# columns by name, once each.
OWN_FILES = {
    "unit_columns.gd",
    "unit_columns_native.gd",
    "unit_place.gd",
    "entry_keys.gd",
    "unit_view.gd",
    "columns_held.gd",
}


def violations_in(text: str) -> list:
    found = []
    lines = text.splitlines()
    loops = []  # [indentation, marked scattered, marked per squad] of each enclosing loop
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
        above = lines[number - 2] if number > 1 else ""
        marked = MARK.search(line) or MARK.search(above)
        per_squad = PER_SQUAD.search(line) or PER_SQUAD.search(above)
        header = LOOP.match(line) is not None  # its own header runs once: outer loops count
        if READ.search(code) is not None and loops:
            if not (marked or any(loop[1] for loop in loops)):
                found.append((number, "a scattered store call inside a loop over units"))
        if COPY.search(code) is not None and loops:
            if not (per_squad or any(loop[2] for loop in loops)):
                found.append((number, "a copy of the store's columns inside a loop"))
        if header:
            loops.append([indent, bool(marked), bool(per_squad)])
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
        print("check_store_reads: store read in a loop (Decisions 144 and 150):")
        print("\n".join(report))
        print("Hold or read a view before the loop (UnitView.hold, UnitView.read) and index its")
        print("arrays by slot, or mark the loop '# store: scattered (few: <why>)' (an event's")
        print("few units) or '# store: scattered (reference: <why>)' (the GDScript reference).")
        print("A view or column copied in a loop: take it once a pass before the loop")
        print("(UnitView.read_once, or one hold and UnitView.of), or mark it")
        print("'# store: per squad (<why>)' with a reason the bench has shown doesn't matter.")
        return 1
    print("check_store_reads: OK")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
