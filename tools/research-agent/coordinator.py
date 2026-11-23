"""Part B: a coordinator agent that hands each Decision to a reader subagent.

Hub and spoke: the coordinator (the hub) plans, searches the index, and asks readers
(the spokes) one question each. Readers never talk to each other or to the user; every
finding comes back to the coordinator, which writes the answer.

    result = run_coordinator(anthropic.Anthropic(), "Which Decision replaced Decision 14, and why?")

Each reader is a fresh run of the same loop (agent.run_loop) with:
  - its own system prompt and only the reading tools (scoped tools, 2.3);
  - a first message holding ONLY what the coordinator wrote for it: a subagent inherits
    none of the coordinator's conversation (1.3);
  - a structured return: the finding, its status, and metadata kept apart (1.3, 5.3).
"""

import json
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from agent import MODEL, SYSTEM, AgentResult, check_citations, run_loop  # noqa: E402
from research_tools import TOOLS, run_tool, tool_error  # noqa: E402

_BY_NAME = {tool["name"]: tool for tool in TOOLS}
_SOURCES_RULES = SYSTEM[SYSTEM.index("Sources:"):]

READER_TOOLS = [_BY_NAME["read_decision"], _BY_NAME["read_spec_lines"]]

READER_SYSTEM = (
    "You are a reader for a coordinator agent. You get one Decision of Breach's design "
    "spec and one question. Read the Decision with read_decision, follow its line "
    "references with read_spec_lines if the question needs them, and answer only that "
    "question, briefly. You see nothing but this message, so rely on the context it "
    "gives you and on what your tools return.\n\n" + _SOURCES_RULES
)

ASK_READER = {
    "name": "ask_reader",
    "description": (
        "Hand one Decision to a reader subagent and get back its finding. The reader sees "
        "only the number, question and context you give it, so put in the context "
        "everything it needs (what you already know, what to compare). Returns JSON: "
        "status ('ok', 'unverified' or 'error'), finding, source_problems, error and meta. "
        "To find a Decision's number first, use search_decisions_index. Ask for several "
        "Decisions in the same turn to have them read side by side."
    ),
    "input_schema": {
        "type": "object",
        "properties": {
            "number": {"type": "integer", "description": "The Decision to read"},
            "question": {"type": "string", "description": "What to find out from it"},
            "context": {"type": "string", "description": "What the reader needs to know; it sees nothing else"},
        },
        "required": ["number", "question", "context"],
        "additionalProperties": False,
    },
    "strict": True,
}

COORDINATOR_TOOLS = [_BY_NAME["search_decisions_index"], ASK_READER]

COORDINATOR_SYSTEM = (
    "You coordinate research on the design Decisions of Breach, a reverse tower defence "
    "game. You don't read Decisions yourself: find their numbers with "
    "search_decisions_index, then hand each one to a reader with ask_reader. Ask for "
    "every Decision you need in the same turn. Each reader sees only what you write in "
    "its question and context.\n\n"
    "Each finding has a status. 'ok': use it with its sources. 'unverified': its sources "
    "don't match what was read; don't rely on it without saying so. 'error': read "
    "error.errorCategory and error.isRetryable. Ask again once if it is retryable; "
    "otherwise answer without it and say what is missing. Don't stop the whole answer "
    "because one reader failed.\n\n" + _SOURCES_RULES
)


def reader_request(number: int, question: str, context: str, model: str = MODEL) -> dict:
    # STUDY(1.3): the reader's whole world is this one message. Anything the coordinator
    # knows that the reader needs must be written into it explicitly.
    prompt = f"Decision {number}\n\nQuestion: {question}\n\nContext from the coordinator:\n{context}"
    return {
        "model": model,
        "max_tokens": 8000,
        "system": READER_SYSTEM,
        "tools": READER_TOOLS,
        "messages": [{"role": "user", "content": prompt}],
    }


def _sources_read(result: AgentResult) -> list[str]:
    sources = []
    for name, _, content, is_error in result.tool_calls:
        if not is_error:
            body = json.loads(content)
            label = f"Decision {body['number']}" if name == "read_decision" else "spec"
            sources.append(f"{label}, lines {body['first_line']}-{body['last_line']}")
    return sources


def finding_from(number: int, result: AgentResult) -> dict:
    """Turn one reader run into the structured finding the coordinator receives."""
    meta = {"turns": result.turns, "stopped": result.stopped, "sources_read": _sources_read(result)}
    base = {"number": number, "partial": None, "error": None, "source_problems": [], "meta": meta}

    # STUDY(5.3): a reader failure becomes structured context for the coordinator.
    # Not suppressed (returned as an empty "ok" finding), and not fatal (raised, which
    # would end the whole run): the coordinator decides whether to retry or report a gap.
    if not result.complete:
        if result.stopped == "refusal":
            error = tool_error("business", "the reader's request was declined", False)
        else:  # max_tokens, max_turns: the same request may finish with another try
            error = tool_error("transient", f"the reader stopped early ({result.stopped})", True)
        return {**base, "status": "error", "error": error, "partial": result.answer}

    failed = [json.loads(content) for _, _, content, is_error in result.tool_calls if is_error]
    if failed and not meta["sources_read"]:
        return {**base, "status": "error", "error": failed[0], "partial": result.answer}

    problems = check_citations(result)
    status = "unverified" if problems else "ok"
    return {**base, "status": status, "finding": result.answer, "source_problems": problems}


def run_reader(client, number: int, question: str, context: str, model: str = MODEL) -> dict:
    result = run_loop(client, reader_request(number, question, context, model), run_tool, max_turns=6)
    return finding_from(number, result), result


def run_coordinator(client, question: str, model: str = MODEL, max_turns: int = 8) -> AgentResult:
    reader_runs = []

    def coordinator_tools(name, tool_input):
        if name != "ask_reader":
            if name not in ("search_decisions_index",):
                return json.dumps(tool_error("validation", f"no tool named {name!r}", False)), True
            return run_tool(name, tool_input)
        finding, run = run_reader(client, tool_input["number"], tool_input["question"], tool_input["context"], model)
        reader_runs.append(run)
        return json.dumps(finding), finding["status"] == "error"

    request = {
        "model": model,
        "max_tokens": 16000,
        "system": COORDINATOR_SYSTEM,
        "tools": COORDINATOR_TOOLS,
        "messages": [{"role": "user", "content": question}],
    }
    result = run_loop(client, request, coordinator_tools, max_turns)
    # The readers' reads count as this run's sources, so check_citations can verify the answer.
    for run in reader_runs:
        result.tool_calls.extend(run.tool_calls)
    return result


_READER_FOR = re.compile(r"^Decision (\d+)\n")


def route(request: dict) -> str:
    """Which agent a request belongs to, for RoutedFakeClient in the tests."""
    if request["system"] == COORDINATOR_SYSTEM:
        return "coordinator"
    first = request["messages"][0]["content"]
    return f"reader:{_READER_FOR.match(first).group(1)}"
