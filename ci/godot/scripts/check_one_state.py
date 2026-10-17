#!/usr/bin/env python3
"""one-state, per the Breach design ledger (Decision 147, spec 35 step 7a).

Every change to a unit's state goes through one writer, UnitState
(sim/skirmish/formation/unit_state.gd), which keeps each squad's living and downed lists:
`living()` and `UnitState.downed` return them without filtering. A write that skips
UnitState leaves the lists stale, and a caller that changes a list it was handed changes
every later reader's. Flags, in `sim/`, `tools/`, `presentation/` and `tests/`:
- a write `.state = SkirmishUnit.State.<...>` outside UnitState (a squad's state is
  `SkirmishSquad.State`, so the two don't mix);
- in `sim/`, `UnitState.start` (for a unit no squad lists yet: tests' and tools' own);
- a mutator (`append`, `erase`, `sort`, `reverse` ...) on `living()` or `UnitState.downed`
  itself, or on a variable a function took from one (take `.duplicate()` to change it).

Usage: check_one_state.py [files...]  (no files: every .gd under those folders)
"""
import re
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(
    subprocess.check_output(["git", "rev-parse", "--show-toplevel"], text=True).strip()
)
FOLDERS = ("sim", "tools", "presentation", "tests")
WRITE = re.compile(r"\.state\s*=(?!=)\s*SkirmishUnit\.State\.")
KEPT = r"(?:\bliving\(\w*\)|\bUnitState\.downed\(\w*\))"
MUTATOR = (
    r"\.(?:append|append_array|erase|insert|remove_at|reverse|sort|sort_custom|push_back"
    r"|push_front|pop_back|pop_front|clear|resize|assign|fill|shuffle)\("
)
CHAINED = re.compile(KEPT + MUTATOR)
TAKEN = re.compile(r"^\s*var\s+(\w+)\s*(?::[^=]*)?:?=\s*[\w.]*" + KEPT + r"\s*$")
START = re.compile(r"\bUnitState\.start\(")
FUNC = re.compile(r"^\s*(?:static\s+)?func\s")
WRITER = "unit_state.gd"


def violations_in(text: str, in_sim: bool = False) -> list:
    found = []
    taken = set()  # names this function took from a kept list
    for number, line in enumerate(text.splitlines(), start=1):
        code = line.split("#", 1)[0]
        if FUNC.match(code):
            taken = set()
        if WRITE.search(code):
            found.append((number, "a unit's state written outside UnitState"))
            continue
        if in_sim and START.search(code):
            found.append((number, "UnitState.start in sim/: a unit in a squad is put"))
            continue
        held = TAKEN.match(code)
        if held:
            taken.add(held.group(1))
            continue
        if CHAINED.search(code) or any(re.search(rf"\b{n}{MUTATOR}", code) for n in taken):
            found.append((number, "a kept list changed (take .duplicate() first)"))
    return found


def files_to_check(arguments: list) -> list:
    if arguments:
        paths = [
            REPO_ROOT / a for a in arguments if a.startswith(FOLDERS) and a.endswith(".gd")
        ]
    else:
        paths = sorted(p for folder in FOLDERS for p in (REPO_ROOT / folder).rglob("*.gd"))
    return [p for p in paths if p.name != WRITER and "addons" not in p.parts]


def main(arguments: list) -> int:
    report = []
    for path in files_to_check(arguments):
        if not path.exists():
            continue
        in_sim = path.relative_to(REPO_ROOT).parts[0] == "sim"
        for number, why in violations_in(path.read_text(encoding="utf-8"), in_sim):
            report.append(f"  {path.relative_to(REPO_ROOT)}:{number}: {why}")
    if report:
        print("check_one_state: a unit's state or a kept list changed outside UnitState")
        print("(Decision 147):")
        print("\n".join(report))
        print("Write a unit's state with UnitState.put: it keeps its squad's lists.")
        return 1
    print("check_one_state: OK")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
