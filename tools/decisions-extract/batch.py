"""Step 5: extract about 20 Decisions in one Message Batch (task statement 4.5).

    python3 tools/decisions-extract/batch.py --dry-run                  # ids, sizes, cost estimate
    python3 tools/decisions-extract/batch.py --collect fixtures/canned_results.jsonl
    python3 tools/decisions-extract/batch.py --submit                   # needs an API key and credits
    python3 tools/decisions-extract/batch.py --fetch msgbatch_...       # status; results once ended

A batch is a list of ordinary Messages requests, each with a custom_id. It runs
asynchronously (most within an hour, at most 24 hours) at 50% of the price.
Results come back as JSONL in any order, so everything here is keyed by custom_id.

Each request is single-shot: there is no in-batch retry. A result that needs
another turn becomes a request in a *follow-up batch* (next_batch below).

Lines marked STUDY carry the exam topic.
"""

import argparse
import copy
import json
import re
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Iterable

import jsonschema

sys.path.insert(0, str(Path(__file__).resolve().parent))
from excerpt import load_excerpt  # noqa: E402
from extract_one import AUTO, build_request  # noqa: E402
from retry import MAX_TOKENS_CAP, NUDGE, error_feedback  # noqa: E402
from schema import DECISION_TOOL  # noqa: E402
from validate import known_decisions, validate  # noqa: E402

MODEL = "claude-opus-5-5"
MAX_FOLLOW_UPS = 2  # validation follow-ups per Decision, as in retry.py

# 20 Decisions: superseded, superseded in part, amended and plain active ones.
# Never 22, 23 or 47: they are the few-shot examples in the system prompt.
BATCH_DECISIONS = (9, 14, 16, 20, 24, 25, 27, 62, 63, 66, 77, 78, 89, 98, 110, 129, 131, 145, 148, 155)

_ID = re.compile(r"^decision-(\d{3})(?:-r(\d+))?$")


def custom_id(number: int, attempt: int = 0) -> str:
    # STUDY 4.5: the custom_id is the only link from a result back to its request.
    # It must be unique within one batch; a follow-up batch adds -r1, -r2.
    return f"decision-{number:03d}" + (f"-r{attempt}" if attempt else "")


def parse_id(cid: str) -> tuple[int, int]:
    """(Decision number, attempt) from a custom_id."""
    match = _ID.match(cid)
    if not match:
        raise ValueError(f"not a custom_id from this tool: {cid!r}")
    return int(match.group(1)), int(match.group(2) or 0)


def batch_request(number: int, model: str = MODEL) -> dict:
    params = build_request(number, model, AUTO)  # auto: forced tool_choice 400s in batches too
    # STUDY 4.5 + caching: cache the ~3.6k-token system prompt that all 20 requests share.
    # Identical in every request; "1h" because a batch can outlast the 5-minute TTL.
    # Hits are best-effort in a batch (requests run concurrently); the discounts stack.
    params["system"] = [
        {"type": "text", "text": params["system"], "cache_control": {"type": "ephemeral", "ttl": "1h"}}
    ]
    # STUDY 4.5: the request shape is {custom_id, params}; params is a normal Messages body
    # without stream, speed, fallbacks or max_tokens 0 (those fail validation).
    return {"custom_id": custom_id(number), "params": params}


def batch_requests(numbers: Iterable[int] = BATCH_DECISIONS, model: str = MODEL) -> list[dict]:
    return [batch_request(n, model) for n in numbers]


# ---- results ---------------------------------------------------------------

# What to do with each result.
ACCEPT = "accept"            # valid record
FOLLOW_UP = "follow_up"      # content problem: send errors back as a tool_result, next batch
RESUBMIT = "resubmit"        # transient (server error, expired, missing): same request
DECIDE = "decide"            # canceled: you stopped the batch; a person decides whether to resend
MORE_ROOM = "more_room"      # stop_reason max_tokens: same request, larger max_tokens
FIX_REQUEST = "fix_request"  # invalid_request_error: our request is wrong; resending fails again
REFUSED = "refused"          # stop_reason refusal: not resent unchanged
GIVE_UP = "give_up"          # still invalid after MAX_FOLLOW_UPS follow-ups

RESEND = (FOLLOW_UP, RESUBMIT, MORE_ROOM)


@dataclass
class Outcome:
    custom_id: str
    action: str
    record: dict | None = None
    errors: list[str] = field(default_factory=list)
    message: dict | None = None  # the succeeded message, kept for a follow-up


def check_message(message: dict, number: int, known: set[int]) -> tuple[str, dict | None, list[str]]:
    """Action, record and errors for one succeeded message (layers 2 and 3 of retry.py)."""
    if message["stop_reason"] == "max_tokens":
        return MORE_ROOM, None, ["stop_reason max_tokens: the tool input is incomplete"]
    if message["stop_reason"] == "refusal":
        return REFUSED, None, ["stop_reason refusal"]

    block = next((b for b in message["content"] if b["type"] == "tool_use"), None)
    if block is None:
        return FOLLOW_UP, None, ["no extract_decision call in the response"]
    record = block["input"]
    try:
        jsonschema.validate(record, DECISION_TOOL["input_schema"])
        errors = validate(record, load_excerpt(number), known)
    except jsonschema.ValidationError as error:
        errors = [f"schema: {error.message}"]
    return (FOLLOW_UP if errors else ACCEPT), record, errors


def sort_results(results: Iterable[dict], sent: Iterable[str]) -> dict[str, Outcome]:
    """One Outcome per custom_id sent. `results` are JSONL lines as dicts, in any order."""
    known = known_decisions()
    outcomes: dict[str, Outcome] = {}

    for line in results:
        cid, result = line["custom_id"], line["result"]
        number, attempt = parse_id(cid)

        # STUDY 4.5: four result types, each handled differently.
        if result["type"] == "succeeded":
            action, record, errors = check_message(result["message"], number, known)
            if action == FOLLOW_UP and attempt >= MAX_FOLLOW_UPS:
                action = GIVE_UP
            outcomes[cid] = Outcome(cid, action, record, errors, result["message"])
        elif result["type"] == "errored":
            error = result["error"]["error"]  # the standard error shape: {"type": "error", "error": {...}}
            action = FIX_REQUEST if error["type"] == "invalid_request_error" else RESUBMIT
            outcomes[cid] = Outcome(cid, action, errors=[f"{error['type']}: {error.get('message', '')}"])
        elif result["type"] == "expired":  # the 24-hour window ran out; the request is fine
            outcomes[cid] = Outcome(cid, RESUBMIT, errors=["expired"])
        elif result["type"] == "canceled":  # someone canceled, maybe because the request is wrong
            outcomes[cid] = Outcome(cid, DECIDE, errors=["canceled: fix the cause before resending"])
        else:
            raise ValueError(f"{cid}: unknown result type {result['type']!r}")

    for cid in sent:
        if cid not in outcomes:  # a request with no result line at all
            outcomes[cid] = Outcome(cid, RESUBMIT, errors=["no result for this custom_id"])
    return outcomes


def next_batch(outcomes: dict[str, Outcome], sent: dict[str, dict]) -> list[dict]:
    """The follow-up batch: only the requests that another try can fix, by custom_id.

    `sent` maps each custom_id of the last batch to the request sent under it.
    """
    requests = []
    for cid, outcome in sorted(outcomes.items()):
        if outcome.action not in RESEND:
            continue
        number, attempt = parse_id(cid)
        params = copy.deepcopy(sent[cid]["params"])

        if outcome.action == MORE_ROOM:
            if params["max_tokens"] >= MAX_TOKENS_CAP:
                continue
            params["max_tokens"] = min(params["max_tokens"] * 2, MAX_TOKENS_CAP)
            next_attempt = attempt  # more room is not a validation follow-up
        elif outcome.action == FOLLOW_UP:
            content = outcome.message["content"]
            block = next((b for b in content if b["type"] == "tool_use"), None)
            feedback = NUDGE if block is None else error_feedback(block["id"], outcome.errors)
            params["messages"] += [
                {"role": "assistant", "content": content},  # thinking blocks included, unchanged
                {"role": "user", "content": [feedback]},
            ]
            next_attempt = attempt + 1
        else:  # RESUBMIT: the same request
            next_attempt = attempt

        # A custom_id only has to be unique within a batch. A resubmit keeps its id;
        # a follow-up is a different conversation, so it gets the next -rN.
        requests.append({"custom_id": custom_id(number, next_attempt), "params": params})
    return requests


# ---- cost ------------------------------------------------------------------

# claude-opus-5-5, $ per million tokens (claude-api skill, cached 2026-11-03).
INPUT, OUTPUT = 4.00, 20.00
BATCH = 0.5          # batch discount; it stacks with caching
CACHE_WRITE_1H = 2.0  # x input price
CACHE_READ = 0.05     # x input price on Opus 5.5


def estimate_cost(requests: list[dict], output_tokens: int = 2000) -> dict[str, float]:
    """Rough $ for the batch, at ~4 characters a token. Thinking counts as output."""
    def chars(params):
        system = params["system"] if isinstance(params["system"], str) else params["system"][0]["text"]
        return len(system), len(json.dumps(params["messages"])) + len(json.dumps(params["tools"]))

    system_tokens = chars(requests[0]["params"])[0] / 4
    rest_tokens = sum(chars(r["params"])[1] for r in requests) / 4
    n = len(requests)
    output = n * output_tokens * OUTPUT / 1e6
    plain_input = (n * system_tokens + rest_tokens) * INPUT / 1e6
    # Best case for caching: one write, every other request a hit (in a batch hits are best-effort).
    cached_input = (system_tokens * (CACHE_WRITE_1H + (n - 1) * CACHE_READ) + rest_tokens) * INPUT / 1e6
    return {
        "standard": plain_input + output,
        "batch": BATCH * (plain_input + output),
        "batch_cached": BATCH * (cached_input + output),
    }


# ---- command line ------------------------------------------------------------


def load_jsonl(path: Path) -> list[dict]:
    return [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines() if line.strip()]


def report(outcomes: dict[str, Outcome]) -> None:
    for cid, outcome in sorted(outcomes.items()):
        if outcome.action != ACCEPT:
            print(f"  {cid:<18} {outcome.action:<12} {outcome.errors[0][:70] if outcome.errors else ''}")
    counts = {}
    for outcome in outcomes.values():
        counts[outcome.action] = counts.get(outcome.action, 0) + 1
    print("  " + ", ".join(f"{k} {v}" for k, v in sorted(counts.items())))


def main() -> None:
    parser = argparse.ArgumentParser()
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--dry-run", action="store_true")
    group.add_argument("--collect", type=Path, metavar="RESULTS_JSONL")
    group.add_argument("--submit", action="store_true")
    group.add_argument("--fetch", metavar="BATCH_ID")
    parser.add_argument("--records", type=Path, help="with --collect: write accepted records here")
    args = parser.parse_args()

    requests = batch_requests()
    sent = {r["custom_id"]: r for r in requests}

    if args.dry_run:
        print(f"{len(requests)} requests: {', '.join(sent)}")
        cost = estimate_cost(requests)
        print("estimate: " + ", ".join(f"{k} ${v:.2f}" for k, v in cost.items()))
        return

    if args.collect:
        outcomes = sort_results(load_jsonl(args.collect), sent)
        report(outcomes)
        follow = next_batch(outcomes, sent)
        print(f"  follow-up batch: {len(follow)} requests: {', '.join(r['custom_id'] for r in follow)}")
        if args.records:
            records = {parse_id(c)[0]: o.record for c, o in outcomes.items() if o.action == ACCEPT}
            args.records.write_text(json.dumps(records, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        return

    import anthropic

    client = anthropic.Anthropic()
    if args.submit:
        batch = client.messages.batches.create(requests=requests)
        print(f"{batch.id} {batch.processing_status}")
        return

    batch = client.messages.batches.retrieve(args.fetch)
    print(f"{batch.id} {batch.processing_status} {batch.request_counts}")
    if batch.processing_status == "ended":
        out = Path(f"{args.fetch}.jsonl")
        with out.open("w", encoding="utf-8") as handle:
            for result in client.messages.batches.results(args.fetch):
                handle.write(json.dumps(result.to_dict()) + "\n")
        print(f"results in {out}; next: --collect {out}")


if __name__ == "__main__":
    main()
