"""Tool definition for extracting one Breach Decision as a JSON record.

The tool is never executed: Claude's tool_use.input is the extracted record.
"""

DECISION_TOOL = {
    "name": "extract_decision",
    "description": (
        "Record one Breach design Decision from the excerpt provided. "
        "Call this tool exactly once, using only what the excerpt states."
    ),
    "strict": True,
    "input_schema": {
        "type": "object",
        "additionalProperties": False,
        "required": [
            "decision_number",
            "title",
            "date",
            "status",
            "status_detail",
            "supersedes",
            "superseded_by",
            "specifications_affected",
            "summary",
        ],
        "properties": {
            "decision_number": {
                "type": "integer",
                "description": "Unique integer id for the decision, this is present in the heading of the decision as e.g. ### Decision N would be decision_number N",
            },
            "title": {
                "type": "string",
                "description": "Exact heading text of the decision",
            },
            "date": {
                "type": ["string", "null"],
                "description": (
                    "The date on this decision's own '**Date:** yyyy-mm-dd' line. "
                    "Null if the excerpt has no such line. Never use the date from a "
                    "'> Superseded by Decision M on yyyy-mm-dd' note."
                ),
            },
            "status": {
                "enum": ["active", "superseded", "unclear", "other"],
                "description": "Status of the decision, active being the default newly created status, superseded used to identify decisions that have been superseded by a later instance",
            },
            "status_detail": {
                "type": ["string", "null"],
                "description": "Additional detail when the status of the decision is other, otherwise null",
            },
            "supersedes": {
                "type": "array",
                "items": {"type": "integer"},
                "description": "The decision(s) superseded by this one, empty if none",
            },
            "superseded_by": {
                "type": "array",
                "items": {"type": "integer"},
                "description": "The decision(s) that supersede this one, empty if none",
            },
            "specifications_affected": {
                "type": "array",
                "items": {"type": "integer"},
                "description": "Integer identifiers of specifications affected by the decision, these are taken from the specification file names e.g. NN-Specification_Name would represent a specification with id NN",
            },
            "summary": {
                "type": "string",
                "description": "Summary description (<25 words) taken from the excerpt, it should only consider the actual decision made and main consequence; ignore rationale and rejected alternatives here",
            },
        },
    },
}
