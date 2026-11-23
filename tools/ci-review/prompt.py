"""The review prompt. A re-review carries the earlier findings so Claude can say which
are fixed, which still stand, and which are new (task statement 3.6)."""

import json

CRITERIA = """Report only these categories: correctness, test-coverage, docs, security.
Skip style, naming and formatting: the linters own those.
Mark a finding blocking only if the PR would ship a wrong result or an untested behaviour."""


def build_prompt(diff_ref: str, prior: list[dict]) -> str:
    """The prompt for `claude -p`. `prior` is the last review's findings, each with an `id`."""
    parts = [f"Review the changes in {diff_ref}.", CRITERIA]
    if prior:
        parts.append(
            "An earlier review of this PR found the issues below. For each finding you report,"
            " set prior_id to the id of the earlier finding it repeats, or null if it is new."
            " List the ids the new commits fixed in resolved_prior_ids."
            " Do not report an earlier finding again in other words."
        )
        parts.append("Earlier findings:\n" + json.dumps(prior, indent=2))
    return "\n\n".join(parts)
