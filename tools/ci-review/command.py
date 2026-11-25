"""The `claude -p` command a CI job would run. Built here and tested, never run: no API calls."""

import json
from pathlib import Path

SCHEMA_PATH = Path(__file__).resolve().parent / "schema.json"

# Read-only tools only: a reviewer that can Edit or run Bash can change what it reviews.
ALLOWED_TOOLS = "Read,Grep,Glob,Bash(git diff *),Bash(git log *)"


def build_command(prompt: str, schema_path: Path = SCHEMA_PATH, max_turns: int = 20) -> list[str]:
    schema = json.dumps(json.loads(schema_path.read_text()))  # compact, one line
    return [
        "claude",
        "-p", prompt,                      # print mode: one run, no one to approve anything
        "--allowedTools", ALLOWED_TOOLS,   # pre-approved; anything else is denied in -p
        "--max-turns", str(max_turns),     # a bound, so a stuck review fails instead of running on
        # STUDY 3.6: one JSON object at the end (the envelope), with the review checked
        # against `schema` and put in its own field, `structured_output`. Print mode only.
        "--output-format", "json",
        "--json-schema", schema,
    ]
