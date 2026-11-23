"""Step 3: the system prompt, with explicit criteria and few-shot examples.

The examples use whole excerpts (what the model really sees) and full records
(what the tool really takes). Decisions 22, 23 and 47 are therefore not fair
to score: the model has seen their answers.
"""

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from excerpt import load_excerpt  # noqa: E402
from schema import CONFIDENCE_FIELDS  # noqa: E402

ALL_HIGH = {name: "high" for name in CONFIDENCE_FIELDS}

FEW_SHOT_DECISIONS = (23, 47, 22)

CRITERIA = """\
You extract one design Decision from the Breach project. The Decision is supplied
as markdown in <decision> tags. Call extract_decision once with the record.

Criteria:
- date: the date on this Decision's own "**Date:**" line. Null if there is no such
  line. A date inside a "> Superseded by Decision M on <date>" note belongs to
  Decision M, so never use it here.
- status: any supersession note, full or "in part", makes the status "superseded".
  When it is in part, status_detail is "in part: " and what is revised; when it is
  full, status_detail is null.
- supersedes and superseded_by: only links this excerpt states in a supersession note
  or a "**Supersedes:**" line. A Decision mentioned for any other reason is not a
  link. Don't infer links the excerpt doesn't state.
- specifications_affected: only numbers NN from specs/NN-*.md, and only when this
  Decision changes, adds or renumbers that spec. A spec mentioned in passing doesn't
  count. Addenda (breach-addendum-*.md) are not specs and never go here.
- confidence: for each listed field, how sure you are the value is right. "high"
  when the excerpt settles it, "medium" when you had to interpret or choose between
  readings, "low" when it is a guess. Rate each field on its own.
"""

EXAMPLE_RECORDS = {
    23: {
        "decision_number": 23,
        "title": (
            "Resource typing, assignment roles, faction, and reward: additive schema, "
            "zero sim/ behavior change"
        ),
        "date": None,
        "status": "superseded",
        "status_detail": (
            "in part: placement of can_sortie, patrol_route, delivery_target_id and "
            "garrison_faction on NodeDef, and the closed FactionId enum"
        ),
        "supersedes": [],
        "superseded_by": [24],
        "specifications_affected": [],
        "summary": (
            "NodeDef gains resource typing and reserves, assignment roles, a faction "
            "and a capture reward, with no change to sim behaviour."
        ),
        "confidence": dict(ALL_HIGH),
    },
    47: {
        "decision_number": 47,
        "title": (
            "Units fight with weapons, and hold their preferred band instead of "
            "stepping into it"
        ),
        "date": "2026-10-01",
        "status": "superseded",
        "status_detail": "in part: WeaponDef generalises into items",
        "supersedes": [],
        "superseded_by": [54],
        "specifications_affected": [],
        "summary": (
            "Units carry weapons that replace attack_range, only the front rank "
            "fights, and units keep to their preferred bands."
        ),
        "confidence": {**ALL_HIGH, "specifications_affected": "medium"},
    },
    22: {
        "decision_number": 22,
        "title": 'Explicit extra edges refine "tracks not a grid," not contradict it',
        "date": None,
        "status": "active",
        "status_detail": None,
        "supersedes": [],
        "superseded_by": [],
        "specifications_affected": [],
        "summary": (
            "Maps may list explicit extra edges in MapDef.edges, refining rather than "
            "replacing the tracks-not-a-grid rule."
        ),
        "confidence": {**ALL_HIGH, "specifications_affected": "medium"},
    },
}

WHY = {
    23: (
        "No **Date:** line, so date is null; 2026-09-26 is Decision 24's date. "
        "'In part' still means superseded, with the revised parts in status_detail. "
        "Decisions 19 and 22 are only mentioned, so they aren't links."
    ),
    47: (
        "Contrast with 23: this one has its own **Date:** line, so that date is used, "
        "not the note's. Spec 21 is mentioned in passing, not changed, so it isn't "
        "affected."
    ),
    22: (
        "The addendum it names is not a spec, so specifications_affected stays empty. "
        "No supersession note and no **Date:** line: active, date null."
    ),
}


def _example(number: int) -> str:
    return (
        "<example>\n"
        f"<input>\n<decision>\n{load_excerpt(number)}</decision>\n</input>\n"
        f"<output>{json.dumps(EXAMPLE_RECORDS[number], ensure_ascii=False)}</output>\n"
        f"<why>{WHY[number]}</why>\n"
        "</example>"
    )


def system_prompt() -> str:
    examples = "\n".join(_example(n) for n in FEW_SHOT_DECISIONS)
    return f"{CRITERIA}\n<examples>\n{examples}\n</examples>\n"
