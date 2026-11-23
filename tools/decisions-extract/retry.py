"""Step 4, part B: extract one Decision, retrying with validation errors fed back.

Three layers of failure, each handled differently:
  1. HTTP errors are raised by the SDK. 429, 5xx, 529 and connection errors are
     retried by the SDK itself (max_retries, default 2). A 400 is a broken
     request: it is not caught here, because sending it again gives the same 400.
  2. stop_reason on a 200 response. max_tokens means the output was cut off, not
     wrong: re-issue with more room, without counting a validation retry.
     refusal is not retried with the same request.
  3. Content: a missing tool call, or a record that fails the schema or
     validate(). Only this layer is sent back to Claude as a tool_result with
     is_error, so it can fix its own answer.
"""

import sys
from pathlib import Path

import jsonschema

sys.path.insert(0, str(Path(__file__).resolve().parent))
from excerpt import load_excerpt  # noqa: E402
from extract_one import AUTO, build_request  # noqa: E402
from schema import DECISION_TOOL  # noqa: E402
from validate import known_decisions, validate  # noqa: E402

MAX_TOKENS_CAP = 16000
NUDGE = {"type": "text", "text": "Call the extract_decision tool once to record the Decision."}


def error_feedback(tool_use_id: str, errors: list[str]) -> dict:
    """The tool_result that sends validation errors back to Claude (also used by batch.py)."""
    return {
        "type": "tool_result",
        "tool_use_id": tool_use_id,
        "is_error": True,
        "content": "The record has these problems. Fix them and call extract_decision again:\n- "
        + "\n- ".join(errors),
    }


def extract_with_retry(client, number: int, model: str = "claude-opus-5-5", max_retries: int = 2):
    """(record or None, errors left). An empty error list means the record passed."""
    request = build_request(number, model, AUTO)
    excerpt, known = load_excerpt(number), known_decisions()
    retries = 0

    while True:
        resp = client.messages.create(**request)  # layer 1: raises on 4xx; SDK retries 429/5xx

        # Layer 2: why did generation stop?
        if resp.stop_reason == "max_tokens":
            if request["max_tokens"] >= MAX_TOKENS_CAP:
                return None, ["stop_reason max_tokens at the cap; the tool input is incomplete"]
            request["max_tokens"] = min(request["max_tokens"] * 2, MAX_TOKENS_CAP)
            continue  # same conversation, more room; not a validation retry
        if resp.stop_reason == "refusal":
            return None, ["stop_reason refusal; not retried with the same request"]

        # Layer 3: what did Claude produce?
        block = next((b for b in resp.content if b.type == "tool_use"), None)
        if block is None:
            record, errors = None, ["no extract_decision call in the response"]
            feedback = NUDGE
        else:
            record = block.input
            try:
                jsonschema.validate(record, DECISION_TOOL["input_schema"])
                errors = validate(record, excerpt, known)
            except jsonschema.ValidationError as error:
                errors = [f"schema: {error.message}"]
            feedback = error_feedback(block.id, errors)

        if not errors or retries >= max_retries:
            return record, errors

        retries += 1
        request["messages"] = request["messages"] + [
            # Claude's whole turn, thinking blocks included, exactly as returned.
            {"role": "assistant", "content": resp.content},
            {"role": "user", "content": [feedback]},
        ]
