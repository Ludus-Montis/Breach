#!/usr/bin/env python3
"""trace_diff: two outcome traces of one run, the base's and the head's (specs/37), compared.

A trace is the digest's JSON lines (tools/formation_digest.gd `trace=`), one per tick:
{"run", "tick", "events", "squads": [{"id", <fields>, "units": [{"id", <fields>}]}]}.
Squads and units are matched by id, never by place (Decision 97); "first" is first in the
base trace's order, which only orders the report. Prints:

  1. the first tick, squad, unit and field to differ, with both values;
  2. the events of that tick only one side has;
  3. a line per tick from there: how many units and squads differ, and in which fields;
  4. the units and squads only one side has, from the tick each first does.

    trace_diff.py <base trace> <head trace> [--json <file>]

--json writes the same per tick (each differing unit's id, squad and fields) for the page
(tools/trace_view/). Exits 0 when the traces are the same, 1 when they differ.
"""
import json
import sys


def _ticks(path: str) -> dict:
    """{tick: line}, in the file's order."""
    out = {}
    with open(path, encoding="utf-8") as lines:
        for text in lines:
            if text.strip():
                line = json.loads(text)
                out[line["tick"]] = line
    return out


def _same(a, b) -> bool:
    """Equal, NaN equal to NaN (the trace writes it as Python reads it)."""
    return a == b or json.dumps(a) == json.dumps(b)


def _fields(base: dict, head: dict, skip: tuple) -> list:
    """The fields whose values differ, in the base's order then the head's."""
    names = list(base) + [name for name in head if name not in base]
    return [n for n in names if n not in skip and not _same(base.get(n), head.get(n))]


def _units(line: dict) -> dict:
    """{unit id: (squad id, unit)} across the tick's squads, in the trace's order."""
    out = {}
    for squad in line["squads"] if line else []:
        for unit in squad["units"]:
            out[unit["id"]] = (squad["id"], unit)
    return out


def _one_side(base: list, head: list) -> tuple:
    """The events only the base has and only the head has, each kept as often as it's extra."""
    rest = [json.dumps(e) for e in head]
    base_only = []
    for event in base:
        key = json.dumps(event)
        if key in rest:
            rest.remove(key)
        else:
            base_only.append(event)
    return base_only, [json.loads(e) for e in rest]


def _tick(tick: int, base: dict, head: dict) -> dict:
    """One tick compared: its differing and one-sided units and squads, and its events."""
    out = {"tick": tick, "units": [], "squads": [], "events": {"base": [], "head": []}}
    base_units, head_units = _units(base), _units(head)
    for unit_id, (squad, unit) in base_units.items():
        if unit_id not in head_units:
            out["units"].append({"id": unit_id, "squad": squad, "only": "base"})
            continue
        head_squad, head_unit = head_units[unit_id]
        fields = (["squad"] if head_squad != squad else []) + _fields(unit, head_unit, ())
        if fields:
            out["units"].append({"id": unit_id, "squad": squad, "fields": fields})
    for unit_id, (squad, _unit) in head_units.items():
        if unit_id not in base_units:
            out["units"].append({"id": unit_id, "squad": squad, "only": "head"})
    base_squads = {s["id"]: s for s in base["squads"]} if base else {}
    head_squads = {s["id"]: s for s in head["squads"]} if head else {}
    for squad_id in list(base_squads) + [s for s in head_squads if s not in base_squads]:
        if squad_id not in head_squads or squad_id not in base_squads:
            side = "base" if squad_id in base_squads else "head"
            out["squads"].append({"id": squad_id, "only": side})
            continue
        fields = _fields(base_squads[squad_id], head_squads[squad_id], ("units",))
        if fields:
            out["squads"].append({"id": squad_id, "fields": fields})
    base_events, head_events = _one_side(
        base["events"] if base else [], head["events"] if head else []
    )
    out["events"] = {"base": base_events, "head": head_events}
    return out


def _differs(entry: dict) -> bool:
    return bool(entry["units"] or entry["squads"] or any(entry["events"].values()))


def _first(entry: dict, base: dict, head: dict) -> dict:
    """The tick's first difference: a unit's before a squad's, the events' last."""
    first = {"tick": entry["tick"], "squad": None, "unit": None, "field": None}
    if entry["units"]:
        unit = entry["units"][0]
        first.update(squad=unit["squad"], unit=unit["id"])
        if "only" in unit:
            first["only"] = unit["only"]
        else:
            first["field"] = unit["fields"][0]
            first["base"], first["head"] = _values(base, head, "unit", unit["id"], first["field"])
    elif entry["squads"]:
        squad = entry["squads"][0]
        first["squad"] = squad["id"]
        if "only" in squad:
            first["only"] = squad["only"]
        else:
            first["field"] = squad["fields"][0]
            first["base"], first["head"] = _values(base, head, "squad", squad["id"], first["field"])
    return first


def _values(base: dict, head: dict, kind: str, item_id, field: str) -> tuple:
    if kind == "unit":
        found = [_units(line)[item_id] for line in (base, head)]
        if field == "squad":
            return found[0][0], found[1][0]
        return found[0][1].get(field), found[1][1].get(field)
    found = [next(s for s in line["squads"] if s["id"] == item_id) for line in (base, head)]
    return found[0].get(field), found[1].get(field)


def compare(base: dict, head: dict) -> dict:
    """The report: the first difference, every tick from it, and what one side only has."""
    ticks = list(base) + [t for t in head if t not in base]
    report = {"first": None, "ticks": [], "only": []}
    seen = {}
    for tick in sorted(ticks):
        entry = _tick(tick, base.get(tick), head.get(tick))
        if report["first"] is None:
            if not _differs(entry):
                continue
            report["first"] = _first(entry, base.get(tick), head.get(tick))
            if tick not in base or tick not in head:
                report["first"]["only"] = "base" if tick in base else "head"
        report["ticks"].append(entry)
        for kind in ("unit", "squad"):
            for item in entry[kind + "s"]:
                key = (kind, item["id"], item.get("only"))
                if "only" in item and key not in seen:
                    seen[key] = {"kind": kind, "id": item["id"], "only": item["only"], "from": tick}
                    if kind == "unit":
                        seen[key]["squad"] = item["squad"]
    report["only"] = list(seen.values())
    return report


def _first_line(first: dict) -> str:
    parts = [f"tick {first['tick']}"]
    if first["squad"] is not None:
        parts.append(f"squad {first['squad']}")
    if first["unit"] is not None:
        parts.append(f"unit {first['unit']}")
    if first.get("field") is not None:
        parts.append(
            f"{first['field']}: {json.dumps(first['base'])} -> {json.dumps(first['head'])}"
        )
    elif "only" in first:
        parts.append(f"{first['only']} only")
    else:
        parts.append("events differ")
    return "  ".join(parts)


def _tick_line(entry: dict) -> str:
    parts = [f"tick {entry['tick']}"]
    for kind in ("units", "squads"):
        changed = [item for item in entry[kind] if "only" not in item]
        if changed:
            names = []
            for item in changed:
                names += [n for n in item["fields"] if n not in names]
            parts.append(f"{kind} {len(changed)}  {', '.join(names)}")
    for side in ("base", "head"):
        count = sum(1 for k in ("units", "squads") for i in entry[k] if i.get("only") == side)
        if count:
            parts.append(f"{side} only {count}")
    events = sum(len(v) for v in entry["events"].values())
    if events:
        parts.append(f"events {events}")
    return "  ".join(parts if len(parts) > 1 else parts + ["same"])


def lines(report: dict) -> list:
    """The report as text."""
    if report["first"] is None:
        return ["the traces are the same"]
    out = [_first_line(report["first"])]
    first = report["ticks"][0]
    if any(first["events"].values()):
        out.append(f"events at tick {first['tick']} only one side has:")
        for side in ("base", "head"):
            out += [f"  {side}: {json.dumps(event)}" for event in first["events"][side]]
    out += [_tick_line(entry) for entry in report["ticks"]]
    for item in report["only"]:
        squad = f" (squad {item['squad']})" if item["kind"] == "unit" else ""
        side = f"{item['only']} only, from tick {item['from']}"
        out.append(f"{item['kind']} {item['id']}{squad}: {side}")
    return out


def main(argv: list) -> int:
    if len(argv) not in (2, 4) or (len(argv) == 4 and argv[2] != "--json"):
        print("usage: trace_diff.py <base trace> <head trace> [--json <file>]", file=sys.stderr)
        return 2
    report = compare(_ticks(argv[0]), _ticks(argv[1]))
    print("\n".join(lines(report)))
    if len(argv) == 4:
        with open(argv[3], "w", encoding="utf-8") as out:
            json.dump(report, out)
    return 0 if report["first"] is None else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
