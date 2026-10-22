"""The open-while-open rule: a spec's parent and its milestone follow their items (spec 40
step 5, Decision 156).

A parent with step sub-issues is open while any step is, and closed when all are: as
completed if any step completed, else as not planned. A parent with no steps is the
owner's and is left alone. A milestone named `Spec NN: <title>` is open while any issue or
PR in it is, and closed when none is; no other milestone is touched. Each close or reopen
is one call, nothing already right is written, and no comments are.
"""
import re

from step_issues import MARKER

SPEC_MILESTONE = re.compile(r"^Spec \d+: ")


def parent_state(steps):
    """(state, state_reason) the parent should have, from its steps' (state, state_reason);
    None for a parent with no steps."""
    if not steps:
        return None
    if any(state == "open" for state, _ in steps):
        return ("open", None)
    completed = any(reason == "completed" for _, reason in steps)
    return ("closed", "completed" if completed else "not_planned")


def milestone_state(states):
    return "open" if "open" in states else "closed"


def is_spec_milestone(title):
    return bool(SPEC_MILESTONE.match(title))


def _marker(record):
    match = MARKER.search(record["body"] or "")
    return match.group(1) if match else None


def _set_parent(client, parent, steps):
    want = parent_state([(step["state"], step["state_reason"]) for step in steps])
    if want is None or want[0] == parent["state"]:
        return
    if want[0] == "open":
        client.reopen_issue(parent["number"])
        print(f"reopened #{parent['number']}")
    else:
        client.close_issue(parent["number"], want[1])
        print(f"closed #{parent['number']} ({want[1]}: its steps are closed)")
    parent.update(state=want[0], state_reason=want[1])


def _set_milestone(client, milestone, items):
    if not is_spec_milestone(milestone["title"]):
        return
    want = milestone_state([item["state"] for item in items])
    if want != milestone["state"]:
        client.set_milestone_state(milestone["number"], want)
        print(f"milestone {milestone['title']} {want}")


def _kept(records):
    """The sync's own issue per marker, the lowest-numbered (spec 40 step 4)."""
    kept = {}
    for number in sorted(records, reverse=True):
        record = records[number]
        marker = None if record["pull_request"] else _marker(record)
        if marker:
            kept[marker] = record
    return kept


def apply_run(client, records):
    """On a sync run: every sync parent and spec milestone, from `records` (number to
    record), the run's listing with its own writes applied."""
    kept = _kept(records)
    for marker, parent in kept.items():
        if marker.startswith("spec "):
            prefix = f"step {marker[len('spec '):]}-"
            _set_parent(client, parent, [r for m, r in kept.items() if m.startswith(prefix)])
    for milestone in client.list_milestone_records():
        _set_milestone(client, milestone, [r for r in records.values() if r["milestone"] == milestone["number"]])


def apply_issue_event(client, number):
    """On an issue closed or reopened: its parent, if it is a sync step, then its milestone
    and the parent's. Creates nothing."""
    issue = client.get_issue(number)
    if issue is None:
        return
    milestones = [issue["milestone"]]
    marker = _marker(issue)
    parent_number = client.get_parent(number) if marker and marker.startswith("step ") else None
    if parent_number is not None:
        parent = client.get_issue(parent_number)
        steps = [r for r in client.list_sub_issues(parent_number) if (_marker(r) or "").startswith("step ")]
        _set_parent(client, parent, steps)
        milestones.append(parent["milestone"])
    for milestone in dict.fromkeys(m for m in milestones if m is not None):
        _set_milestone(client, client.get_milestone(milestone), client.list_milestone_issue_records(milestone))
