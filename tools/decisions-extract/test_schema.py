"""Schema-level tests for the Decision extraction tool (no API calls)."""

import sys
import unittest
from pathlib import Path

import jsonschema

sys.path.insert(0, str(Path(__file__).resolve().parent))
from schema import DECISION_TOOL  # noqa: E402

DECISION_SCHEMA = DECISION_TOOL["input_schema"]

VALID = {
    "decision_number": 1,
    "title": "Test Decision",
    "date": "2026-11-21",
    "status": "active",
    "status_detail": None,
    "supersedes": [],
    "superseded_by": [],
    "specifications_affected": [1],
    "summary": "Test decision has no direct consequences",
}


class TestSchema(unittest.TestCase):
    def test_schema_accepts_valid_decision(self):
        jsonschema.validate(VALID, DECISION_SCHEMA)

    def test_schema_accepts_null_date(self):
        jsonschema.validate({**VALID, "date": None}, DECISION_SCHEMA)

    def test_schema_rejects_missing_date(self):
        record = dict(VALID)
        del record["date"]
        with self.assertRaises(jsonschema.ValidationError):
            jsonschema.validate(record, DECISION_SCHEMA)

    def test_schema_rejects_invalid_status(self):
        with self.assertRaises(jsonschema.ValidationError):
            jsonschema.validate({**VALID, "status": "invalid_status"}, DECISION_SCHEMA)

    def test_schema_rejects_null_link_list(self):
        with self.assertRaises(jsonschema.ValidationError):
            jsonschema.validate({**VALID, "supersedes": None}, DECISION_SCHEMA)

    def test_schema_rejects_unexpected_key(self):
        with self.assertRaises(jsonschema.ValidationError):
            jsonschema.validate({**VALID, "an_extra_key": "x"}, DECISION_SCHEMA)


if __name__ == "__main__":
    unittest.main()
