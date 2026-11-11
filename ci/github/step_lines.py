"""A step's `**Needs:**`, `**Touches:**` and `**Split at build:**` lines (spec 45 step 1,
Decision 171), read from the lines under each `### Step <id>` heading up to the next heading.

`Needs` is `none` or step ids split by commas (`1, 2a`, a `step` before one is allowed);
`Touches` is paths split by commas, in backticks or not, and may run over several lines;
`Split at build` is `from step <id>, <PR, date and why>`. A step without a `Needs` line
needs every step above it. The first heading of a step that carries a line is the one read.
"""
import re
from dataclasses import dataclass, field

HEADING = re.compile(r"^#{1,6} ")
STEP_HEADING = re.compile(r"^### Step (\w+)\b")
LINE = re.compile(r"^\*\*(Needs|Touches|Split at build):\*\*\s*(.*?)\s*$")
SPLIT = re.compile(r"^from step (\w+)\s*[,:;]?\s*(.*)$", re.IGNORECASE)
NEED = re.compile(r"^(?:step\s+)?(\w+)$", re.IGNORECASE)


@dataclass
class StepLines:
    needs: list = field(default_factory=list)
    needs_declared: bool = False
    touches: list = field(default_factory=list)
    split_from: str = ""
    split_note: str = ""


def _paths(value):
    quoted = re.findall(r"`([^`]+)`", value)
    found = quoted if quoted else value.split(",")
    return [path.strip() for path in found if path.strip()]


def _needs(value):
    if value.strip().lower() in ("", "none"):
        return []
    needs = []
    for part in value.split(","):
        match = NEED.match(part.strip())
        needs.append(match.group(1) if match else part.strip())
    return needs


def _fields(block):
    """The step lines in a heading's block, with a line's continuation joined to it."""
    fields, name = {}, None
    for line in block:
        found = LINE.match(line)
        if found:
            name = found.group(1)
            fields[name] = found.group(2)
        elif name and line.strip() and not line.startswith(("**", "- ", "* ")):
            fields[name] += " " + line.strip()
        else:
            name = None
    return fields


def _lines(fields):
    lines = StepLines()
    if "Needs" in fields:
        lines.needs, lines.needs_declared = _needs(fields["Needs"]), True
    lines.touches = _paths(fields.get("Touches", ""))
    split = SPLIT.match(fields.get("Split at build", ""))
    if split:
        lines.split_from, lines.split_note = split.group(1), split.group(2)
    return lines


def _blocks(text):
    """(step id, the lines under its heading) for each `### Step` heading, in file order."""
    blocks, current = [], None
    for line in text.splitlines():
        if HEADING.match(line):
            heading = STEP_HEADING.match(line)
            current = (heading.group(1), []) if heading else None
            if current:
                blocks.append(current)
        elif current:
            current[1].append(line)
    return blocks


def parse(text):
    """Each step id's lines; the first of its headings that carries any line wins."""
    found = {}
    for step, block in _blocks(text):
        fields = _fields(block)
        if step not in found or (fields and not found[step][0]):
            found[step] = (fields, _lines(fields))
    return {step: lines for step, (_, lines) in found.items()}


def unknown_needs(lines):
    """(step, need) for each need that names a step the spec doesn't have."""
    return [(step, need) for step, own in lines.items() for need in own.needs if need not in lines]


def _step_refs(steps, numbers):
    return ", ".join(f"step {s} (#{numbers[s]})" if s in numbers else f"step {s}" for s in steps)


def body_lines(lines, numbers):
    """The `Needs` and `Touches` lines of a step's issue; `numbers` maps step id to issue."""
    if not lines.needs_declared:
        needs = "every step above it (none declared)"
    else:
        needs = _step_refs(lines.needs, numbers) if lines.needs else "none"
    touches = ", ".join(f"`{path}`" for path in lines.touches) if lines.touches else "not declared"
    return f"**Needs:** {needs}\n**Touches:** {touches}\n"


def split_comment(step, path, into, note, numbers):
    """The closing comment of a step split at build into the steps `into`."""
    return (
        f"Split at build: `### Step {step}` in `{path}` is now {_step_refs(into, numbers)}"
        f" ({note}). Those issues carry the work."
    )
