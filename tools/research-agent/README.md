# Decisions research agent

A study exercise: a Claude agent that answers questions about Breach's Decisions, citing each source.

It is not part of the game or its tooling. It was written to practise the Claude
Certified Architect exam's agent topics (Domain 1, task statements 2.1–2.3, 5.3, 5.6).
Lines marked `STUDY(<task>)` carry the exam point and say what it is.

- **Part A, `agent.py`:** a manual agent loop on the Messages API that stops on
  `stop_reason`, never on the text or a turn count. It has three tools in
  `research_tools.py` that read `DECISIONS_INDEX.md` and the design spec by line, return
  structured errors, and treat an empty search as a valid result. `check_citations()`
  checks that every `[Decision N, line L]` points at text a tool returned.
- **Part B, `coordinator.py`:** a coordinator that hands each Decision to a reader
  subagent. Readers get only the coordinator's prompt and the two reading tools, and
  return a finding with `status`, `error` and `meta` kept apart.

## Install

Nothing to install for the tests: they use Python 3.11+ and the standard library only.
A live run needs the `anthropic` package and an API key.

## Usage

Run the tests offline. A fake client plays back scripted responses, and nothing calls the API:

```sh
python3 -m unittest discover -s tools/research-agent
```

CI does not run these tests, so run them locally after a change.

A live run, which is billed per token:

```python
import anthropic
from coordinator import run_coordinator

result = run_coordinator(anthropic.Anthropic(), "Which Decision replaced Decision 14, and why?")
print(result.answer)
```

## Contributing

Follow the repository's process (`CLAUDE.md`): a branch, a PR, and the owner merges.

## License

As the repository.
