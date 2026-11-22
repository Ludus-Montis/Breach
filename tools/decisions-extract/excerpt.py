"""Load one Decision's text from the design spec by its line in DECISIONS_INDEX.md.

Reads only the lines of that Decision (from its heading to the next
`### Decision` heading), never the whole spec into a prompt.
"""

import re
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
SPEC = REPO / "Breach — Reverse Tower Defense Design Spec.md"
INDEX = REPO / "DECISIONS_INDEX.md"

_ROW = re.compile(r"^\|\s*(\d+)\s*\|.*\|\s*(\d+)\s*\|[^|]*\|\s*$")


def decision_line(number: int) -> int:
    """The 1-based line of Decision `number`'s heading in the spec."""
    for row in INDEX.read_text(encoding="utf-8").splitlines():
        match = _ROW.match(row)
        if match and int(match.group(1)) == number:
            return int(match.group(2))
    raise KeyError(f"Decision {number} is not in {INDEX.name}")


def load_excerpt(number: int) -> str:
    """Decision `number`'s text, heading included, up to the next Decision."""
    start = decision_line(number)
    lines = []
    with SPEC.open(encoding="utf-8") as spec:
        for line_no, line in enumerate(spec, start=1):
            if line_no < start:
                continue
            if line_no > start and line.startswith("### Decision "):
                break
            lines.append(line)
    return "".join(lines).rstrip() + "\n"
