"""The fixed label set (Decision 148), and the names of the labels a spec implies.

A new label is a line in FIXED; sync.py creates whatever is missing.
"""

STEP_COLOR = "c5def5"
RAISE_WHEN_COLOR = "fbca04"

FIXED = [
    ("design", "5319e7", "A design session's work: Decisions, specs, questions"),
    ("implementation", "0e8a16", "Builds a spec step with /run-phase"),
    ("chore", "bfdadc", "Tooling, CI or process; no gameplay behaviour"),
    ("sim", "1d76db", "Touches the simulation under sim/"),
    ("native", "b60205", "Touches the Rust core under native/"),
    ("designer", "006b75", "Touches the map designer under tools/designer/"),
    ("ci", "0052cc", "Touches CI or the checks"),
    ("tooling", "c2e0c6", "Touches other tools or skills"),
    ("outcomes-changed", "d93f0b", "Moves simulation outcomes (digests)"),
    ("needs-design", "e99695", "Waits on a design session"),
    ("question", "d876e3", "An open question for the owner (Decision 148)"),
    ("raise-when:next-design", RAISE_WHEN_COLOR, "Raise at the next design session"),
    ("raise-when:other", RAISE_WHEN_COLOR, "Raise when the body's trigger is met"),
]


def step_label(spec, step):
    return f"step:{spec}-{step}"


def raise_when_spec_label(spec):
    return f"raise-when:spec-{spec}"
