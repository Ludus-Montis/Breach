"""Part A: a research agent on the Messages API, with the agent loop written by hand.

This is way 1 of the four (manual loop): our code is the harness. Each turn it calls
Claude, runs any tools Claude asked for, sends ALL their results back in one user
message, and repeats until stop_reason says Claude is done.

    result = run_agent(anthropic.Anthropic(), "Which Decision replaced Decision 14, and why?")

Offline, the tests pass a FakeClient (fake_client.py) instead of anthropic.Anthropic().
"""

import json
import re
import sys
from dataclasses import dataclass, field
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from research_tools import TOOLS, run_tool  # noqa: E402

MODEL = "claude-opus-5-5"

SYSTEM = """You answer questions about the design Decisions of Breach, a reverse tower \
defence game, using only what your tools return. Don't answer from memory.

Look up before you answer: find the Decision with search_decisions_index if you don't \
know its number, then read it with read_decision. When you need several Decisions, ask \
for them in the same turn.

If a tool returns an error, read its errorCategory: fix your input after a validation \
error; say what you couldn't read after any other error. Never present a guess as a \
finding.

Sources:
- End every claim with its source, as [Decision N, line L] for a line inside Decision N, \
or [spec, lines A-B] for text read with read_spec_lines.
- If two sources disagree, give both, each with its source, and say which is newer \
(a later Decision, or one that supersedes or amends the other).
- If the Decisions don't answer part of the question, say "Not found in the Decisions" \
for that part.
"""
# STUDY(5.6): the "Sources" rules above are the provenance contract: a claim-to-source
# mapping, conflicts kept and annotated, gaps stated. check_citations() below checks
# the first rule in code, against what the tools actually returned in this run.


@dataclass
class AgentResult:
    answer: str | None
    complete: bool
    stopped: str  # the stop_reason that ended the run, or "max_turns"
    turns: int
    tool_calls: list = field(default_factory=list)  # (name, input, content, is_error)


def build_request(question: str, model: str = MODEL) -> dict:
    return {
        "model": model,
        "max_tokens": 16000,
        "system": SYSTEM,
        "tools": TOOLS,
        # tool_choice left at its default, auto: Claude decides whether and which tool
        # to call. Forcing one ("any"/"tool") 400s on claude-opus-5-5, and would stop
        # Claude from ever finishing with an answer.
        "messages": [{"role": "user", "content": question}],
    }


def _text(content) -> str:
    return "".join(block.text for block in content if block.type == "text").strip()


def run_agent(client, question: str, model: str = MODEL, max_turns: int = 12) -> AgentResult:
    return run_loop(client, build_request(question, model), run_tool, max_turns)


def run_loop(client, request: dict, tool_runner, max_turns: int = 12) -> AgentResult:
    """The loop itself, shared by this agent, the coordinator and its readers (Part B).

    tool_runner(name, input) -> (content, is_error) answers one tool_use block.
    """
    messages = request["messages"]
    calls = []

    for turn in range(1, max_turns + 1):
        response = client.messages.create(**request)
        # Claude's whole turn goes back exactly as returned (thinking blocks included).
        messages.append({"role": "assistant", "content": response.content})

        # STUDY(1.1): stop_reason, and nothing else, decides whether the loop goes on.
        # "tool_use" = run the tools and call again; "end_turn" = Claude is done.
        # Not: "the text looks like an answer", and not: "we've done N turns".
        if response.stop_reason == "end_turn":
            return AgentResult(_text(response.content), True, "end_turn", turn, calls)
        if response.stop_reason != "tool_use":
            # max_tokens, refusal, ...: stop and say so; don't pass a partial turn off as done.
            return AgentResult(_text(response.content) or None, False, response.stop_reason, turn, calls)

        results = []
        for block in response.content:
            if block.type != "tool_use":
                continue
            content, is_error = tool_runner(block.name, block.input)
            calls.append((block.name, block.input, content, is_error))
            # STUDY(2.2): a failed tool still answers its tool_use_id, with is_error and
            # the structured body, so Claude sees what failed and why. Dropping it, or
            # raising, would leave the tool_use unanswered (a 400 on the next call).
            results.append(
                {"type": "tool_result", "tool_use_id": block.id, "content": content, "is_error": is_error}
            )
        # All results from one turn go back in ONE user message (keeps parallel calls working).
        messages.append({"role": "user", "content": results})

    # A safety cap, not a stopping rule: reaching it means the run is unfinished.
    return AgentResult(None, False, "max_turns", max_turns, calls)


_DECISION_CITE = re.compile(r"\[Decision (\d+), line (\d+)\]")
_SPEC_CITE = re.compile(r"\[spec, lines (\d+)-(\d+)\]")


def check_citations(result: AgentResult) -> list[str]:
    """Problems with the answer's sources: each must point at text a tool returned in this run."""
    if not result.answer:
        return ["no answer"]
    read = []  # (number or None, first, last) of every successful read
    for name, _, content, is_error in result.tool_calls:
        if is_error or name not in ("read_decision", "read_spec_lines"):
            continue
        body = json.loads(content)
        read.append((body.get("number"), body["first_line"], body["last_line"]))

    problems = []
    cites = _DECISION_CITE.findall(result.answer) + [(None, a, b) for a, b in _SPEC_CITE.findall(result.answer)]
    if not cites:
        problems.append("the answer cites no sources")
    for cite in _DECISION_CITE.findall(result.answer):
        number, line = int(cite[0]), int(cite[1])
        if not any(n == number and first <= line <= last for n, first, last in read):
            problems.append(f"[Decision {number}, line {line}] is not in any Decision read this run")
    for a, b in _SPEC_CITE.findall(result.answer):
        if not any(first <= int(a) and int(b) <= last for _, first, last in read):
            problems.append(f"[spec, lines {a}-{b}] was not read this run")
    return problems
