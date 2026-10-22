"""Converts the simulation's .tres definitions (units, weapons, armour, tuning) to the crate's
JSON once (spec 39 step 2, Decision 153), then is deleted. Each number's text is copied as
the .tres wrote it, so no value passes through a float formatter on the way.

    python3 tools/convert_tres_content.py
"""

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent / "content"
ITEM_FIELDS = [
    "item_name", "slots", "weight", "strength_requirement", "tags", "traits", "grants", "innate",
]
POSITIONS = ["front", "mid", "back"]


class Raw(str):
    """A value's JSON text, written as it is."""


def ext_resources(text: str) -> dict:
    found = re.findall(r'\[ext_resource [^\]]*path="res://content/([^"]+)\.tres" id="([^"]+)"', text)
    return {rid: path for path, rid in found}


def resource_fields(text: str) -> list:
    """The [resource] section's key = value pairs, in order; a value may span lines."""
    body = text.split("[resource]\n", 1)[1]
    pairs = []
    for line in body.splitlines():
        match = re.match(r"^(\w+) = (.*)$", line)
        if match:
            pairs.append([match.group(1), match.group(2)])
        elif pairs and line.strip():
            pairs[-1][1] += line.strip()
    return [(key, value) for key, value in pairs if key != "script"]


def value_of(text: str, ext: dict):
    text = text.strip()
    typed = re.match(r"^Array\[[^\]]*\]\((\[.*\])\)$", text)
    if typed:
        text = typed.group(1)
    if text.startswith("["):
        inner = text[1:-1].strip()
        return [value_of(part, ext) for part in split_top(inner)] if inner else []
    if text.startswith("{"):
        inner = text[1:-1].strip()
        entries = []
        for part in split_top(inner):
            key, value = part.split(":", 1)
            entries.append((key.strip().strip('"'), value_of(value, ext)))
        return dict(entries)
    ref = re.match(r'^ExtResource\("([^"]+)"\)$', text)
    if ref:
        return ext[ref.group(1)]
    if text in ("inf", "-inf"):
        return text
    return Raw(text)


def split_top(text: str) -> list:
    parts, depth, start, quoted = [], 0, 0, False
    for index, char in enumerate(text):
        if char == '"':
            quoted = not quoted
        elif not quoted and char in "[{(":
            depth += 1
        elif not quoted and char in "]})":
            depth -= 1
        elif not quoted and char == "," and depth == 0:
            parts.append(text[start:index].strip())
            start = index + 1
    parts.append(text[start:].strip())
    return [part for part in parts if part]


def written(value, indent: int = 0) -> str:
    pad = "  " * (indent + 1)
    if isinstance(value, Raw):
        return str(value)
    if isinstance(value, str):
        return '"%s"' % value
    if isinstance(value, list):
        return "[" + ", ".join(written(item, indent) for item in value) + "]"
    if not value:
        return "{}"
    lines = ['%s"%s": %s' % (pad, key, written(item, indent + 1)) for key, item in value.items()]
    return "{\n" + ",\n".join(lines) + "\n" + "  " * indent + "}"


def convert(path: Path, kind: str) -> dict:
    text = path.read_text()
    ext = ext_resources(text)
    fields = dict((key, value_of(value, ext)) for key, value in resource_fields(text))
    if kind == "units":
        if "preferred_position" in fields:
            fields["preferred_position"] = POSITIONS[int(fields["preferred_position"])]
        return fields
    if kind in ("weapons", "armour"):
        item = {key: fields.pop(key) for key in ITEM_FIELDS if key in fields}
        return {"item": item, **fields}
    return fields


def main() -> None:
    for kind in ("units", "weapons", "armour", "tuning"):
        for path in sorted((ROOT / kind).glob("*.tres")):
            out = path.with_suffix(".json")
            out.write_text(written(convert(path, kind)) + "\n")
            print(out.relative_to(ROOT.parent))


main()
