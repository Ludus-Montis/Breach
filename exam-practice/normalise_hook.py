"""Exercise (topic 5, task 1.5): a PostToolUse hook that normalises tool results.

Two order systems answer in different shapes:
  legacy_orders  -> {"order_id": "A-1", "created": 1764067200, "status": 3}
  new_orders     -> {"order_id": "B-7", "created": "2025-11-25T10:40:00Z", "status": "SHIPPED"}

The model should see ONE shape: ISO 8601 UTC timestamps and status words.
A hook does this in code, every time, before the model reads the result.

Run the tests (offline, no API key):  python3 normalise_hook.py
"""

import unittest
from datetime import datetime, timezone

# Legacy numeric status codes -> the words the new system uses.
STATUS_CODES = {1: "PENDING", 2: "PAID", 3: "SHIPPED", 4: "DELIVERED", 9: "CANCELLED"}


def to_iso(value):
    """Return an ISO 8601 UTC string ending in 'Z' for a Unix timestamp or an ISO string."""
    if isinstance(value, str):
        return value  # already ISO 8601
    return datetime.fromtimestamp(value, timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def to_status(value):
    """Return a status word for a numeric code or a word."""
    if not isinstance(value, int):
        return value
    elif value in STATUS_CODES:
        return STATUS_CODES[value]
    else:
        return f"UNKNOWN({value})"


def normalise(result):
    """The pure function the hook calls. One shape out, whatever came in."""
    return {
        "order_id": result["order_id"],
        "created": to_iso(result["created"]),
        "status": to_status(result["status"]),
    }


# How it is wired into the Agent SDK (shown, not run here):
#
#   async def normalise_hook(input_data, tool_use_id, context):
#       ...replace the tool's output with normalise(...) ...
#
#   options = ClaudeAgentOptions(hooks={
#       "PostToolUse": [HookMatcher(matcher="mcp__orders__.*", hooks=[normalise_hook])],
#   })
#
# PostToolUse runs after the tool and before the model sees the result.


class NormaliseTest(unittest.TestCase):
    def test_legacy_result_gets_iso_time_and_status_word(self):
        out = normalise({"order_id": "A-1", "created": 1764067200, "status": 3})
        self.assertEqual(
            out,
            {"order_id": "A-1", "created": "2025-11-25T10:40:00Z", "status": "SHIPPED"},
        )

    def test_new_result_passes_through_unchanged(self):
        r = {"order_id": "B-7", "created": "2025-11-25T10:40:00Z", "status": "SHIPPED"}
        self.assertEqual(normalise(r), r)

    def test_unknown_code_is_flagged_not_guessed(self):
        out = normalise({"order_id": "A-2", "created": 1764067200, "status": 7})
        self.assertEqual(out["status"], "UNKNOWN(7)")


if __name__ == "__main__":
    unittest.main()
