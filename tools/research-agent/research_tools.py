"""Part A: the research agent's three tools (Claude API tool definitions + their handlers).

"Tool" here means a Claude API tool definition (name, description, input_schema) that
Claude can ask for; run_tool() is the code that answers it. Claude never reads the
files itself: it only sees what run_tool() returns.

Every result is JSON text. A failure is still a result, sent back with is_error: true
and a structured body, so Claude (or a coordinator) can decide what to do next:

    {"errorCategory": "validation", "isRetryable": false, "message": "..."}

A search that finds nothing is NOT an error: it is a valid empty result.
"""

import json
import re
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
SPEC = REPO / "Breach — Reverse Tower Defense Design Spec.md"
INDEX = REPO / "DECISIONS_INDEX.md"

MAX_SEARCH_ROWS = 10
MAX_SPEC_LINES = 80

_ROW = re.compile(r"^\|\s*(\d+)\s*\|\s*(.+?)\s*\|\s*(\d+)\s*\|\s*(.*?)\s*\|\s*$")
_WORD = re.compile(r"[a-z0-9_]+")


# STUDY(2.1): each description says what the tool returns AND when to use another tool
# instead. Two similar tools with vague descriptions ("reads the spec") make Claude pick
# at random; test_research_tools.DescriptionTest checks that each one names the others.
TOOLS = [
    {
        "name": "search_decisions_index",
        "description": (
            "Find Breach design Decisions by words in their titles or status. Returns up to "
            f"{MAX_SEARCH_ROWS} rows, each with number, title, the line where the Decision "
            "starts in the design spec, and status (for example 'superseded by 15'). Use it "
            "first when you don't know a Decision's number. It does not return a Decision's "
            "text: call read_decision for that. An empty list means no title matched."
        ),
        "input_schema": {
            "type": "object",
            "properties": {
                "query": {"type": "string", "description": "Words to match, e.g. 'touched files'"}
            },
            "required": ["query"],
            "additionalProperties": False,
        },
        "strict": True,
    },
    {
        "name": "read_decision",
        "description": (
            "Return the full text of one Decision by its number, from its heading to the "
            "next Decision, with each line prefixed by its line number in the design spec. "
            "Use it to read what a Decision says and why. For spec text that is not inside a "
            "Decision, use read_spec_lines instead; to find a number, use "
            "search_decisions_index."
        ),
        "input_schema": {
            "type": "object",
            "properties": {"number": {"type": "integer", "description": "The Decision's number"}},
            "required": ["number"],
            "additionalProperties": False,
        },
        "strict": True,
    },
    {
        "name": "read_spec_lines",
        "description": (
            f"Return lines start to end (at most {MAX_SPEC_LINES}) of the design spec, each "
            "prefixed by its line number. Use it only for spec text outside the Decisions, "
            "such as a section a Decision points to by line. To read a Decision, use "
            "read_decision, which finds where it ends for you."
        ),
        "input_schema": {
            "type": "object",
            "properties": {
                "start": {"type": "integer", "description": "First line, 1-based"},
                "end": {"type": "integer", "description": "Last line, inclusive"},
            },
            "required": ["start", "end"],
            "additionalProperties": False,
        },
        "strict": True,
    },
]


def index_rows() -> list[dict]:
    """Every row of DECISIONS_INDEX.md as {number, title, line, status}."""
    rows = []
    for row in INDEX.read_text(encoding="utf-8").splitlines():
        match = _ROW.match(row)
        if match:
            number, title, line, status = match.groups()
            rows.append({"number": int(number), "title": title, "line": int(line), "status": status})
    return rows


def _spec_lines() -> list[str]:
    return SPEC.read_text(encoding="utf-8").splitlines()


def _numbered(lines: list[str], first: int) -> str:
    return "\n".join(f"{first + i}: {line}" for i, line in enumerate(lines))


# STUDY(2.2): the shape of a structured error. errorCategory says what kind of failure
# (transient, validation, permission, business); isRetryable says whether the SAME call
# could succeed later. Claude can fix a validation error by changing its input; only a
# transient one is worth repeating unchanged.
def tool_error(category: str, message: str, retryable: bool) -> dict:
    return {"errorCategory": category, "isRetryable": retryable, "message": message}


class ToolFailure(Exception):
    def __init__(self, category: str, message: str, retryable: bool = False):
        super().__init__(message)
        self.body = tool_error(category, message, retryable)


def search_decisions_index(query: str) -> dict:
    words = _WORD.findall(query.lower())
    if not words:
        raise ToolFailure("validation", "query has no words to match; give one or more words")
    hits = [
        row
        for row in index_rows()
        if all(word in f"{row['title']} {row['status']}".lower() for word in words)
    ]
    # A valid empty result: the search worked and found nothing. Not is_error.
    return {"query": query, "results": hits[:MAX_SEARCH_ROWS], "more": max(0, len(hits) - MAX_SEARCH_ROWS)}


def read_decision(number: int) -> dict:
    rows = {row["number"]: row for row in index_rows()}
    if number not in rows:
        raise ToolFailure(
            "validation",
            f"Decision {number} does not exist; Decisions run from {min(rows)} to {max(rows)}",
        )
    row = rows[number]
    lines = _spec_lines()
    start = row["line"]
    end = start
    while end < len(lines) and not lines[end].startswith("### Decision "):
        end += 1  # `end` is the last line of this Decision, 1-based
    body = lines[start - 1 : end]
    while body and not body[-1].strip():
        body.pop()
        end -= 1
    return {
        "number": number,
        "title": row["title"],
        "status": row["status"],
        "first_line": start,
        "last_line": end,
        "text": _numbered(body, start),
    }


def read_spec_lines(start: int, end: int) -> dict:
    lines = _spec_lines()
    if start < 1 or end < start or end > len(lines):
        raise ToolFailure("validation", f"lines must satisfy 1 <= start <= end <= {len(lines)}")
    if end - start + 1 > MAX_SPEC_LINES:
        raise ToolFailure("validation", f"at most {MAX_SPEC_LINES} lines per call; split the range")
    return {"first_line": start, "last_line": end, "text": _numbered(lines[start - 1 : end], start)}


HANDLERS = {
    "search_decisions_index": search_decisions_index,
    "read_decision": read_decision,
    "read_spec_lines": read_spec_lines,
}


def run_tool(name: str, tool_input: dict) -> tuple[str, bool]:
    """(content, is_error) for one tool_use block. Never raises."""
    handler = HANDLERS.get(name)
    if handler is None:
        return json.dumps(tool_error("validation", f"no tool named {name!r}", False)), True
    try:
        return json.dumps(handler(**tool_input)), False
    except ToolFailure as failure:
        return json.dumps(failure.body), True
    except TypeError as error:  # wrong or missing arguments
        return json.dumps(tool_error("validation", str(error), False)), True
    except PermissionError as error:
        return json.dumps(tool_error("permission", str(error), False)), True
    except OSError as error:  # e.g. the file was mid-write; the same call may work later
        return json.dumps(tool_error("transient", str(error), True)), True
