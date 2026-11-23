"""Reads what `claude -p --output-format json` prints: one JSON object (the "envelope")."""

import json


class ReviewRunError(Exception):
    """The run gave no usable review. `category` says why; `retryable` says whether rerunning may help."""

    def __init__(self, category: str, retryable: bool, detail: str):
        super().__init__(f"{category}: {detail}")
        self.category = category
        self.retryable = retryable
        self.detail = detail


def read_review(stdout: str) -> dict:
    """The validated review object, or ReviewRunError. Never parses the free text in `result`."""
    envelope = json.loads(stdout)
    if envelope.get("is_error"):
        subtype = envelope.get("subtype", "unknown")
        # Hitting max-turns may pass on a rerun with a higher bound; other errors need a person.
        raise ReviewRunError(subtype, subtype == "error_max_turns", "the run ended in an error")
    # STUDY 3.6: the checked review is `structured_output`, never the free text in `result`.
    # Missing means no review was produced: an error, not an empty (passing) review.
    if "structured_output" not in envelope:
        raise ReviewRunError("no_structured_output", True, "the run gave no schema-valid review")
    return envelope["structured_output"]
