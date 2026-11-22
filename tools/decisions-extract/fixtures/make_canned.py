"""Writes canned_results.jsonl: a Message Batches results file for batch.py's tests.

    python3 tools/decisions-extract/fixtures/make_canned.py

No API call. Good records come from the excerpt and DECISIONS_INDEX.md; the faults
below are put in on purpose, one per way a batch result can need handling. Lines are
shuffled with a fixed seed, as the real results file is in no particular order.
"""

import json
import random
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
from batch import BATCH_DECISIONS, custom_id  # noqa: E402
from excerpt import INDEX, load_excerpt  # noqa: E402
from schema import CONFIDENCE_FIELDS  # noqa: E402

_ROW = re.compile(r"^\|\s*(\d+)\s*\|.*\|\s*\d+\s*\|([^|]*)\|\s*$")
_HEADING = re.compile(r"^### Decision \d+ — (.+)$", re.MULTILINE)
_DATE = re.compile(r"^\*\*Date:\*\*\s*(\d{4}-\d{2}-\d{2})\s*$", re.MULTILINE)


def index_status() -> dict[int, str]:
    rows = {}
    for line in INDEX.read_text(encoding="utf-8").splitlines():
        match = _ROW.match(line)
        if match:
            rows[int(match.group(1))] = match.group(2).strip()
    return rows


def good_record(number: int, status_by: dict[int, str]) -> dict:
    excerpt = load_excerpt(number)
    later = [int(n) for n in re.findall(r"superseded by ([\d, ]+)", status_by[number]) for n in n.split(",") if n.strip()]
    earlier = sorted(n for n, s in status_by.items() if re.search(rf"superseded by [\d, ]*\b{number}\b", s))
    in_part = "Superseded in part" in excerpt
    date = _DATE.search(excerpt)
    return {
        "decision_number": number,
        "title": _HEADING.search(excerpt).group(1).strip(),
        "date": date.group(1) if date else None,
        "status": "superseded" if later else "active",
        "status_detail": "in part: which formations catch routers" if in_part else None,
        "supersedes": earlier,
        "superseded_by": later,
        "specifications_affected": [],
        "summary": "Canned summary for tests.",
        "confidence": {name: "high" for name in CONFIDENCE_FIELDS} | CONFIDENCE.get(number, {}),
    }


# Step 6: superseded, but the excerpt doesn't say so (only the later Decision does).
# Read from the excerpt alone, the record says active with nothing superseding it,
# and the excerpt gives no hint that anything is missing.
SOURCE_GAP = (62, 63, 77, 78, 89)


# Step 6: canned confidences. "high" unless listed here. The source gaps above stay
# "high": nothing in their excerpts hints that a later Decision exists.
CONFIDENCE = {24: {"title": "medium"}, 25: {"status": "medium"}, 98: {"superseded_by": "medium"}, 129: {"supersedes": "low"}}


def excerpt_only(record: dict) -> dict:
    record.update(status="active", status_detail=None, superseded_by=[])
    return record


def message(number: int, content: list, stop_reason: str) -> dict:
    return {
        "id": f"msg_canned_{number:03d}",
        "type": "message",
        "role": "assistant",
        "model": "claude-opus-5-5",
        "content": content,
        "stop_reason": stop_reason,
        "stop_sequence": None,
        "usage": {"input_tokens": 1200, "cache_read_input_tokens": 3600, "output_tokens": 900},
    }


def tool_turn(number: int, record: dict) -> list:
    return [
        {"type": "thinking", "thinking": "", "signature": f"sig-canned-{number:03d}"},
        {"type": "tool_use", "id": f"toolu_canned_{number:03d}", "name": "extract_decision", "input": record},
    ]


def errored(kind: str, text: str) -> dict:
    return {"type": "errored", "error": {"type": "error", "error": {"type": kind, "message": text}}}


def main() -> None:
    status_by = index_status()
    lines = []
    for number in BATCH_DECISIONS:
        record = good_record(number, status_by)
        if number in SOURCE_GAP:
            record = excerpt_only(record)
        if number == 24:
            record["title"] = record["title"].split(";")[0]  # cut short: a plain extraction slip
        cid = custom_id(number)
        if number == 16:
            continue  # no result line at all
        if number == 27:
            result = errored("invalid_request_error", "tool_choice: type 'tool' is not supported for this model")
        elif number == 62:
            result = errored("overloaded_error", "Overloaded")
        elif number == 89:
            result = {"type": "expired"}
        elif number == 110:
            result = {"type": "canceled"}
        elif number == 131:
            cut = [{"type": "thinking", "thinking": "", "signature": "sig-canned-131"}]
            result = {"type": "succeeded", "message": message(number, cut, "max_tokens")}
        elif number == 145:
            record["date"] = "2026-01-01"  # not in the excerpt: validate() rejects it
            result = {"type": "succeeded", "message": message(number, tool_turn(number, record), "tool_use")}
        elif number == 148:
            text = [{"type": "text", "text": "Decision 148 moves open questions to GitHub issues."}]
            result = {"type": "succeeded", "message": message(number, text, "end_turn")}
        elif number == 155:
            result = {"type": "succeeded", "message": message(number, [], "refusal")}
        else:
            result = {"type": "succeeded", "message": message(number, tool_turn(number, record), "tool_use")}
        lines.append({"custom_id": cid, "result": result})

    random.Random(5).shuffle(lines)
    out = HERE / "canned_results.jsonl"
    out.write_text("".join(json.dumps(line) + "\n" for line in lines), encoding="utf-8")
    print(f"{len(lines)} lines -> {out.name}")


if __name__ == "__main__":
    main()
