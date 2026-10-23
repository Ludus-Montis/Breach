"""The designer's terrain library and the crate's schema for it (spec 39 step 2b, Decision 153).

The designer edits the library in its own shape (label, relief {amplitude, scale}, strata
[{material, min, max}], heat transitions [{above|below, becomes|gains}]); the file on disk,
content/terrain/terrain_library.json, is in the crate's schema (content/schema/
terrain_library.schema.json), with GDScript's names. to_schema() writes it, from_schema()
reads it back for the designer. The rules are the ones the Godot library import had, saves
from before Decisions 62 to 66 included.

in_use_problems() keeps a library edit from breaking a map: every terrain and feature a
saved designer map (content/maps_src/*.designer.json) uses must still be in the library.
"""

from __future__ import annotations

import json
import math
from pathlib import Path

MAP_SUFFIX = ".designer.json"
CAPACITY = ("stability", "max_height", "max_width", "max_length", "max_depth")


def to_schema(library: dict) -> dict:
    """The designer's library in the crate's schema. Raises ValueError on an entry with no id."""
    materials = [_material(entry) for entry in _entries(library, "materials", "material")]
    # A liquid library saved before Decision 64: each a material that flows (lava slowly).
    for entry in _entries(library, "liquids", "liquid"):
        liquid = _material(entry)
        liquid["traits"].setdefault("flows", 1 if liquid["heat"] > 4 else 3)
        materials.append(liquid)
    return {
        "terrains": [_terrain(entry) for entry in _entries(library, "terrains", "terrain")],
        "features": [_feature(entry) for entry in _entries(library, "features", "feature")],
        "materials": materials,
        "road_move_multiplier": library.get("road_move_multiplier", 0.5),
    }


def from_schema(library: dict) -> dict:
    """The crate's terrain library in the designer's shape."""
    return {
        "terrains": [_designer_terrain(entry) for entry in library.get("terrains", [])],
        "materials": [_designer_material(entry) for entry in library.get("materials", [])],
        "features": [
            {
                "id": entry["id"],
                "label": entry.get("display_name", ""),
                "glyph": entry.get("glyph", ""),
                "stub_effect": entry.get("effect_notes", ""),
            }
            for entry in library.get("features", [])
        ],
        "road_move_multiplier": library.get("road_move_multiplier", 0.5),
    }


def used_ids(maps_src: Path) -> dict:
    """{"terrains": {id: [map names]}, "features": {id: [map names]}} for every saved map."""
    used = {"terrains": {}, "features": {}}
    for path in sorted(maps_src.glob("*" + MAP_SUFFIX)) if maps_src.is_dir() else []:
        export = json.loads(path.read_text(encoding="utf-8"))
        if not isinstance(export, dict):
            continue
        name = path.name[: -len(MAP_SUFFIX)]
        terrain_ids = [str(export.get("default_terrain", ""))]
        feature_ids = []
        for tile in export.get("tiles", []):
            if not tile.get("terrain_is_base", False):
                terrain_ids.append(str(tile.get("terrain", "")))
            if tile.get("feature") is not None:
                feature_ids.append(str(tile["feature"]))
        for kind, ids in (("terrains", terrain_ids), ("features", feature_ids)):
            for entry_id in ids:
                if entry_id and name not in used[kind].setdefault(entry_id, []):
                    used[kind][entry_id].append(name)
    return used


def in_use_problems(library: dict, maps_src: Path) -> list:
    """Each terrain or feature a saved map uses that `library` (the schema's shape) lacks."""
    used = used_ids(maps_src)
    problems = []
    for kind, bucket in (("terrain", "terrains"), ("feature", "features")):
        have = {entry.get("id") for entry in library.get(bucket, [])}
        for entry_id, maps in used[bucket].items():
            if entry_id not in have:
                problems.append(
                    f"{kind} '{entry_id}' is still used by {', '.join(maps)} - change those "
                    "maps first, or keep it in the library"
                )
    return problems


def _entries(library: dict, key: str, kind: str) -> list:
    entries = library.get(key, [])
    for entry in entries:
        if not str(entry.get("id", "")):
            raise ValueError(f"a {kind} entry has no id: {json.dumps(entry)}")
    return entries


def _terrain(entry: dict) -> dict:
    relief = entry.get("relief") if isinstance(entry.get("relief"), dict) else {}
    bearing = int(entry.get("bearing", 0))
    blocked = [part.strip() for part in str(entry.get("blocks_unit_classes", "")).split(",")]
    terrain = {
        "id": str(entry["id"]),
        "display_name": str(entry.get("label", entry["id"])),
        "glyph": str(entry.get("glyph", "")),
        "color": str(entry.get("color", "#ffffff")),
        "can_be_base": bool(entry.get("can_be_base", True)),
        "needs_bridge": bool(entry.get("needs_bridge", False)),
        "move_cost": entry.get("move_cost", 1.0),
        "blocks_unit_classes": [part for part in blocked if part],
        "bearing": bearing,
        "default_elevation": int(entry.get("default_elevation", 0)),
        "relief_amplitude": int(relief.get("amplitude", 0)),
        "relief_scale": int(relief.get("scale", 0)),
        "foundation_max": int(entry.get("foundation_max", bearing)),
        "dig_depth": int(entry.get("dig_depth", 0)),
        "liquids": [_liquid(body) for body in entry.get("liquids", _legacy_water(entry))],
        "strata": [_stratum(band) for band in entry.get("strata", [])],
    }
    for field in CAPACITY:
        terrain["default_" + field] = int(entry.get("default_" + field, 0))
    return terrain


def _liquid(body: dict) -> dict:
    least = int(body.get("min", 0))
    return {
        "material_id": str(body.get("material", body.get("liquid", ""))),
        "min_cells": least,
        "max_cells": int(body.get("max", least)),
        "chance": body.get("chance", 1.0),
        "surface_chance": body.get("surface_chance", 0.0),
    }


def _stratum(band: dict) -> dict:
    least = int(band.get("min", 1))
    return {
        "material_id": str(band.get("material", "")),
        "min_cells": least,
        "max_cells": int(band.get("max", least)),
    }


def _legacy_water(entry: dict) -> list:
    """A water table saved before Decision 62, as a water body that is always there."""
    water = entry.get("water_table")
    if not isinstance(water, dict):
        return []
    return [{"material": "WATER", "min": water.get("min", 0), "max": water.get("max", 0)}]


def _material(entry: dict) -> dict:
    heat = int(entry["heat"]) if "heat" in entry else _heat_of(int(entry.get("temperature", 15)))
    source = entry.get("traits") if isinstance(entry.get("traits"), dict) else {}
    traits = {str(key): int(value) for key, value in source.items()}
    # Saved before Decisions 63-64: a loose flag, and difficulties as fields.
    if entry.get("loose", False):
        traits["loose"] = 1
    for field in ("dig_difficulty", "climb_difficulty"):
        if int(entry.get(field, 0)) > 0:
            traits[field] = int(entry[field])
    return {
        "id": str(entry["id"]),
        "display_name": str(entry.get("label", entry["id"])),
        "color": str(entry.get("color", "#ffffff")),
        "weight": int(entry.get("weight", 1)),
        "span": int(entry.get("span", 1)),
        "strength": int(entry.get("strength", 0)),
        "heat": heat,
        "traits": traits,
        "heat_transitions": [_transition(rule) for rule in entry.get("heat_transitions", [])],
    }


def _transition(rule: dict) -> dict:
    """{"above"|"below": heat level, "becomes": id | "gains": trait} (Decisions 63, 66). A
    threshold over 10 is degrees, saved before Decision 66."""
    threshold = int(rule.get("above", rule.get("below", 0)))
    return {
        "threshold": _heat_of(threshold) if threshold > 10 else threshold,
        "rising": "above" in rule,
        "becomes": str(rule.get("becomes", "")),
        "gains_trait": str(rule.get("gains", "")),
    }


def _heat_of(degrees: int) -> int:
    """A temperature in degrees, saved before Decision 66, as a heat level 0-10."""
    return 1 if degrees <= 50 else min(10, math.ceil(degrees / 150.0))


def _feature(entry: dict) -> dict:
    return {
        "id": str(entry["id"]),
        "display_name": str(entry.get("label", entry["id"])),
        "glyph": str(entry.get("glyph", "")),
        "effect_notes": str(entry.get("stub_effect", "")),
    }


def _designer_terrain(entry: dict) -> dict:
    terrain = {key: value for key, value in entry.items() if key not in _RENAMED_TERRAIN}
    terrain["label"] = entry.get("display_name", "")
    terrain["blocks_unit_classes"] = ", ".join(entry.get("blocks_unit_classes", []))
    terrain["relief"] = {
        "amplitude": entry.get("relief_amplitude", 0),
        "scale": entry.get("relief_scale", 0),
    }
    terrain["strata"] = [
        {"material": band["material_id"], "min": band["min_cells"], "max": band["max_cells"]}
        for band in entry.get("strata", [])
    ]
    terrain["liquids"] = [
        {
            "material": body["material_id"],
            "min": body["min_cells"],
            "max": body["max_cells"],
            "chance": body["chance"],
            "surface_chance": body["surface_chance"],
        }
        for body in entry.get("liquids", [])
    ]
    return terrain


_RENAMED_TERRAIN = ("display_name", "relief_amplitude", "relief_scale")


def _designer_material(entry: dict) -> dict:
    material = {key: value for key, value in entry.items() if key != "display_name"}
    material["label"] = entry.get("display_name", "")
    material["heat_transitions"] = [_designer_transition(rule) for rule in entry.get("heat_transitions", [])]
    return material


def _designer_transition(rule: dict) -> dict:
    out = {"above" if rule.get("rising", True) else "below": rule.get("threshold", 0)}
    if rule.get("becomes"):
        out["becomes"] = rule["becomes"]
    if rule.get("gains_trait"):
        out["gains"] = rule["gains_trait"]
    return out
