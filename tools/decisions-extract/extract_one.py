"""Step 2: extract one Decision through tool use, under three tool_choice settings.

    python3 tools/decisions-extract/extract_one.py 9 --dry-run   # print the requests only
    python3 tools/decisions-extract/extract_one.py 9             # needs an API key and credits

Variants:
  a  claude-opus-5,   forced tool   -> a tool_use block
  b  claude-opus-5-5, forced tool   -> HTTP 400 (forced tool_choice not supported)
  c  claude-opus-5-5, auto          -> usually a tool_use block, not guaranteed
"""

import argparse
import json
import sys
from pathlib import Path

import jsonschema

sys.path.insert(0, str(Path(__file__).resolve().parent))
from excerpt import load_excerpt  # noqa: E402
from schema import DECISION_TOOL  # noqa: E402

FORCED = {"type": "tool", "name": "extract_decision"}
AUTO = {"type": "auto"}

VARIANTS = {
    "a": ("claude-opus-5", FORCED),
    "b": ("claude-opus-5-5", FORCED),
    "c": ("claude-opus-5-5", AUTO),
}

INSTRUCTION = (
    "Extract the Decision in <decision> by calling the extract_decision tool once."
)


def build_request(number: int, model: str, tool_choice: dict) -> dict:
    return {
        "model": model,
        "max_tokens": 4096,  # thinking counts toward this too
        "tools": [DECISION_TOOL],
        "tool_choice": tool_choice,
        "messages": [
            {
                "role": "user",
                "content": f"<decision>\n{load_excerpt(number)}</decision>\n\n{INSTRUCTION}",
            }
        ],
    }


def run(client, request: dict) -> None:
    import anthropic

    try:
        resp = client.messages.create(**request)
    except anthropic.BadRequestError as error:
        print(f"  400: {error.message}")
        return

    print(f"  stop_reason: {resp.stop_reason}")
    # Your step: this raises StopIteration when no tool_use block came back.
    block = next(b for b in resp.content if b.type == "tool_use")
    record = block.input

    print(json.dumps(record, ensure_ascii=False, indent=2))
    jsonschema.validate(record, DECISION_TOOL["input_schema"])
    print("  schema: valid")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("decision", type=int)
    parser.add_argument("--variants", default="abc")
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()

    client = None
    if not args.dry_run:
        import anthropic

        client = anthropic.Anthropic()

    for key in args.variants:
        model, tool_choice = VARIANTS[key]
        request = build_request(args.decision, model, tool_choice)
        print(f"== variant {key}: {model}, tool_choice={tool_choice['type']}")
        if args.dry_run:
            shown = dict(request, tools=["<DECISION_TOOL>"])
            print(json.dumps(shown, ensure_ascii=False, indent=2))
        else:
            run(client, request)


if __name__ == "__main__":
    main()
