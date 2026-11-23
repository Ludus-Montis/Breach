"""Semantic checks on one extracted Decision record, beyond the JSON Schema.

`strict: true` guarantees the record's shape. These checks catch records that
are well-formed but wrong, and return messages specific enough to send back to
Claude as a tool_result with is_error (step 4, part B).
"""

import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from excerpt import INDEX, _ROW  # noqa: E402

_DATE_LINE = re.compile(r"^\*\*Date:\*\*\s*(\d{4}-\d{2}-\d{2})\s*$", re.MULTILINE)
_HEADING = re.compile(r"^### Decision (\d+)\b", re.MULTILINE)


def known_decisions() -> set[int]:
    """Every Decision number listed in DECISIONS_INDEX.md."""
    numbers = set()
    for row in INDEX.read_text(encoding="utf-8").splitlines():
        match = _ROW.match(row)
        if match:
            numbers.add(int(match.group(1)))
    return numbers


def validate(record: dict, excerpt: str, known: set[int]) -> list[str]:
    """Error messages for `record`; an empty list means it passed."""
    errors = []
    number = record["decision_number"]
    status = record["status"]
    detail = record["status_detail"]

    heading = _HEADING.search(excerpt)
    if heading and int(heading.group(1)) != number:
        errors.append(
            f"decision_number is {number} but the excerpt's heading is "
            f"Decision {heading.group(1)}."
        )

    if status == "other" and not detail:
        errors.append("status is 'other', so status_detail must say what the status is.")

    if status == "superseded" and not record["superseded_by"]:
        errors.append(
            "status is 'superseded' but superseded_by is empty; list the Decision(s) "
            "that supersede it, or use 'unclear' if the excerpt does not name one."
        )

    if detail and detail.startswith("in part:") and status != "superseded":
        errors.append(
            f"status_detail starts with 'in part:' but status is {status!r}; "
            "'in part:' is only used with status 'superseded'."
        )

    if record["date"] is not None:
        own_dates = _DATE_LINE.findall(excerpt)
        if record["date"] not in own_dates:
            where = (
                "it appears only elsewhere in the excerpt, such as a 'Superseded by' note"
                if record["date"] in excerpt
                else "it does not appear in the excerpt"
            )
            errors.append(
                f"date {record['date']!r} is not on a '**Date:**' line ({where}). "
                "Use the '**Date:**' line's date, or null if there is none."
            )

    for field in ("supersedes", "superseded_by"):
        if number in record[field]:
            errors.append(f"{field} lists {number}, the Decision's own number.")
        unknown = sorted(n for n in record[field] if n not in known)
        if unknown:
            errors.append(f"{field} lists {unknown}, which are not Decisions in the index.")

    return errors
