"""A spec's parent issue and its step issues (spec 40 steps 1 and 4, Decisions 155 and 156).

Each spec gets a parent `Spec NN: <title>` and a sub-issue per distinct `### Step <id>`
heading, wherever it sits. Bodies link the spec and never copy it; a marker line
`<!-- github-sync: spec NN -->` or `<!-- github-sync: step NN-<id> -->` is how the sync
knows an issue is its own; where several carry one marker, the lowest-numbered is (spec 40
step 4) and the later ones are closed as duplicates. It never edits a body, and closes only
its own issues.
"""
import re
from dataclasses import dataclass, field

import labels

STEP_HEADING = re.compile(r"^### Step (\w+)(?:,\s*(.*?)|)\s*$", re.MULTILINE)
MARKER = re.compile(r"<!-- github-sync: (spec \d+|step \d+-\w+) -->")


@dataclass
class Issue:
    marker: str
    title: str
    body: str
    labels: list = field(default_factory=list)
    step: str = ""


@dataclass
class SpecIssues:
    number: str
    path: str
    milestone: str
    parent: Issue
    steps: list


def _marker_line(marker):
    return f"<!-- github-sync: {marker} -->"


def step_ids(text):
    """Each distinct step id, in file order, with the first title given for it."""
    titles = {}
    for step, title in STEP_HEADING.findall(text):
        if not titles.get(step):
            titles[step] = title
    return titles


def plan_spec(number, path, milestone, text, repo_url):
    link = f"[`{path}`]({repo_url}/blob/main/{path})"
    parent = Issue(
        f"spec {number}",
        milestone,
        f"The steps of {link}, one sub-issue each. The spec is the source.\n\n{_marker_line(f'spec {number}')}\n",
    )
    steps = []
    for step, title in step_ids(text).items():
        marker = f"step {number}-{step}"
        heading = f"### Step {step}, {title}" if title else f"### Step {step}"
        steps.append(
            Issue(
                marker,
                f"Spec {number} step {step}: {title}" if title else f"Spec {number} step {step}",
                f"`{heading}` in {link}. The spec is the source.\n\n{_marker_line(marker)}\n",
                [labels.step_label(number, step)],
                step,
            )
        )
    return SpecIssues(number, path, milestone, parent, steps)


def _own(records):
    """Every issue carrying each sync marker, by number, and the step labels of merged PRs."""
    copies, merged = {}, set()
    for record in records:
        if record["pull_request"]:
            if record["merged"]:
                merged.update(record["labels"])
            continue
        match = MARKER.search(record["body"] or "")
        if match:
            copies.setdefault(match.group(1), {})[record["number"]] = record
    return copies, merged


def _add(copies, more):
    for marker, records in more.items():
        copies.setdefault(marker, {}).update(records)


def _kept(copies):
    """The sync's own issue per marker: the lowest-numbered, the first it made."""
    return {marker: records[min(records)] for marker, records in copies.items()}


def read(client):
    """Lists every issue and PR once and says what it found, before the run writes anything."""
    records = client.list_issue_records()
    copies, merged = _own(records)
    prs = sum(1 for record in records if record["pull_request"])
    marked = sum(len(found) for found in copies.values())
    print(f"read {len(records) - prs} issues and {prs} PRs; {marked} with sync markers")
    return copies, merged


def _create(client, issue, milestone):
    number = client.create_issue(issue.title, issue.body, issue.labels, milestone)
    print(f"issue #{number} {issue.title}")
    return number


def _missing(spec, copies):
    return any(issue.marker not in copies for issue in [spec.parent] + spec.steps)


def _read_milestone(client, spec, milestone, copies):
    """A second read before creating: the milestone's own listing (spec 40 step 4)."""
    found, _ = _own(client.list_milestone_issue_records(milestone))
    print(f"read milestone {spec.milestone}: {sum(len(f) for f in found.values())} with sync markers")
    _add(copies, found)


def _create_spec(client, spec, milestone, marked, merged):
    parent = marked.get(spec.parent.marker)
    parent_number = parent["number"] if parent else _create(client, spec.parent, milestone)
    for step in spec.steps:
        if step.marker in marked:
            continue
        number = _create(client, step, milestone)
        client.add_sub_issue(parent_number, number)
        if step.labels[0] in merged:
            client.close_issue(number, "completed")


def _close_copies(client, copies):
    """Closes each open later copy of a marker as a duplicate of the lowest-numbered one."""
    for marker, records in copies.items():
        kept = min(records)
        for number in sorted(records):
            if number == kept or records[number]["state"] != "open":
                continue
            client.comment(number, f"A copy of #{kept}, which github-sync keeps for `{marker}`.")
            client.close_issue(number, "duplicate")
            print(f"closed #{number} (a copy of #{kept})")


def _close_gone(client, specs, marked):
    planned = {step.marker for spec in specs for step in spec.steps}
    by_number = {spec.number: spec for spec in specs}
    for marker, record in marked.items():
        if not marker.startswith("step ") or marker in planned or record["state"] != "open":
            continue
        number, step = marker[len("step ") :].split("-", 1)
        spec = by_number.get(number)
        path = spec.path if spec else f"specs/{number}-*.md"
        ids = ", ".join(s.step for s in spec.steps) if spec else "none"
        client.comment(
            record["number"],
            f"No `### Step {step}` heading in `{path}` any more; the spec's steps are now: {ids}.",
        )
        client.close_issue(record["number"], "not_planned")
        print(f"closed #{record['number']} ({marker} is gone)")


def apply(client, specs, milestones, own):
    """Creates the missing parents and steps, closes later copies of a marker as duplicates,
    and closes the sync's steps that are gone. `own` is what read() found."""
    copies, merged = own
    for spec in specs:
        milestone = milestones[spec.milestone]
        if _missing(spec, copies):
            _read_milestone(client, spec, milestone, copies)
        _create_spec(client, spec, milestone, _kept(copies), merged)
    _close_copies(client, copies)
    _close_gone(client, specs, _kept(copies))
