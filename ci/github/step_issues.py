"""A spec's parent issue and its step issues (spec 40 steps 1 and 4, Decisions 155 and 156).

Each spec gets a parent `Spec NN: <title>` and a sub-issue per distinct `### Step <id>`
heading, wherever it sits. Bodies link the spec and never copy it; a marker line
`<!-- github-sync: spec NN -->` or `<!-- github-sync: step NN-<id> -->` is how the sync
knows an issue is its own; where several carry one marker, the lowest-numbered is (spec 40
step 4) and the later ones are closed as duplicates. It closes only its own issues.

A step's issue also carries the step's `Needs` and `Touches` lines (step_lines.py, spec 45
step 1, Decision 171), its needs linked to their issues. An open issue whose body differs from
the one the spec now gives is rewritten; a closed one keeps its body. A step whose heading is
gone and which later steps say they were split from (`**Split at build:** from step <id>`) is
closed naming those steps.

A heading that only the spec's `## Built` section carries (a step split while it was built,
like 7b) was added by the PR that built it, so the sync only learns of the step once that PR
has merged, and the PR, labelled with the parent step's label, can't have closed it. Its
issue is closed against that PR, found from git: the merge that first added the heading.
"""
import re
import subprocess
from dataclasses import dataclass, field

import labels
import step_lines

STEP_HEADING = re.compile(r"^### Step (\w+)(?:,\s*(.*?)|)\s*$", re.MULTILINE)
MARKER = re.compile(r"<!-- github-sync: (spec \d+|step \d+-\w+) -->")
SECTION = re.compile(r"^## (.*?)\s*$")
MERGED_PR = re.compile(r"\(#(\d+)\)\s*$|^Merge pull request #(\d+)\b")


@dataclass
class Issue:
    marker: str
    title: str
    body: str
    labels: list = field(default_factory=list)
    step: str = ""
    built: str = ""  # its heading line, when only the Built section carries it
    head: str = ""  # a step's first line: its heading and the spec's link
    lines: step_lines.StepLines = None

    def render(self, numbers):
        """The body, with each needed step linked where `numbers` (step id to issue) has it."""
        if self.lines is None:
            return self.body
        needs = step_lines.body_lines(self.lines, numbers)
        return f"{self.head}\n\n{needs}\n{_marker_line(self.marker)}\n"


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


def built_steps(text):
    """Each step id whose headings all sit under `## Built`, with its first heading line."""
    section, built, elsewhere = "", {}, set()
    for line in text.splitlines():
        found = SECTION.match(line)
        if found:
            section = found.group(1)
            continue
        heading = STEP_HEADING.match(line)
        if not heading:
            continue
        if section.startswith("Built"):
            built.setdefault(heading.group(1), line.rstrip())
        else:
            elsewhere.add(heading.group(1))
    return {step: line for step, line in built.items() if step not in elsewhere}


def pr_of_subject(subject):
    """The PR a merge's subject names: a squash's `(#N)` at its end, or `Merge pull request #N`."""
    found = MERGED_PR.search(subject)
    return int(found.group(1) or found.group(2)) if found else None


def built_by_git(root):
    """(path, heading) -> the PR whose merge first added the heading to the file, from git."""

    def built_by(path, heading):
        log = subprocess.run(
            ["git", "-C", str(root), "log", "--reverse", "--format=%s", "-S", heading, "--", path],
            check=True,
            capture_output=True,
            text=True,
        ).stdout.splitlines()
        return pr_of_subject(log[0]) if log else None

    return built_by


def plan_spec(number, path, milestone, text, repo_url):
    link = f"[`{path}`]({repo_url}/blob/main/{path})"
    parent = Issue(
        f"spec {number}",
        milestone,
        f"The steps of {link}, one sub-issue each. The spec is the source.\n\n{_marker_line(f'spec {number}')}\n",
    )
    steps = []
    built = built_steps(text)
    lines = step_lines.parse(text)
    unknown = step_lines.unknown_needs(lines)
    if unknown:
        raise SystemExit(
            "; ".join(f"{path}: step {step} needs step {need}, which the spec doesn't have" for step, need in unknown)
        )
    for step, title in step_ids(text).items():
        marker = f"step {number}-{step}"
        heading = f"### Step {step}, {title}" if title else f"### Step {step}"
        issue = Issue(
            marker,
            f"Spec {number} step {step}: {title}" if title else f"Spec {number} step {step}",
            "",
            [labels.step_label(number, step)],
            step,
            built.get(step, ""),
            f"`{heading}` in {link}. The spec is the source.",
            lines[step],
        )
        issue.body = issue.render({})
        steps.append(issue)
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
    return copies, merged, {record["number"]: record for record in records}


def _create(client, issue, milestone, records):
    number = client.create_issue(issue.title, issue.body, issue.labels, milestone)
    print(f"issue #{number} {issue.title}")
    records[number] = {
        "number": number,
        "title": issue.title,
        "body": issue.body,
        "state": "open",
        "state_reason": None,
        "labels": list(issue.labels),
        "milestone": milestone,
        "pull_request": False,
        "merged": False,
    }
    return number


def _close(client, record, reason):
    """Closes an issue and records it, so the parent and milestone rule sees the run's writes."""
    client.close_issue(record["number"], reason)
    record.update(state="closed", state_reason=reason)


def _missing(spec, copies):
    return any(issue.marker not in copies for issue in [spec.parent] + spec.steps)


def _read_milestone(client, spec, milestone, copies, records):
    """A second read before creating: the milestone's own listing (spec 40 step 4)."""
    listing = [records.setdefault(r["number"], r) for r in client.list_milestone_issue_records(milestone)]
    found, _ = _own(listing)
    print(f"read milestone {spec.milestone}: {sum(len(f) for f in found.values())} with sync markers")
    _add(copies, found)


def _keep_under(client, parent, step):
    """Attaches a kept step to the kept parent, open or closed, where it sits elsewhere or
    nowhere; a step made under a copy moves (spec 40 step 4)."""
    if client.get_parent(step) != parent:
        client.add_sub_issue(parent, step, replace_parent=True)
        print(f"moved #{step} under #{parent}")


def _create_spec(client, spec, milestone, marked, merged, records):
    parent = marked.get(spec.parent.marker)
    parent_number = parent["number"] if parent else _create(client, spec.parent, milestone, records)
    numbers = _step_numbers(spec, marked)
    for step in spec.steps:
        if step.marker in marked:
            _keep_under(client, parent_number, marked[step.marker]["number"])
            continue
        step.body = step.render(numbers)  # links the steps it needs that have issues already
        number = numbers[step.step] = _create(client, step, milestone, records)
        client.add_sub_issue(parent_number, number)
        if step.labels[0] in merged:
            _close(client, records[number], "completed")


def _close_copies(client, copies):
    """Closes each open later copy of a marker as a duplicate of the lowest-numbered one."""
    for marker, records in copies.items():
        kept = min(records)
        for number in sorted(records):
            if number == kept or records[number]["state"] != "open":
                continue
            client.comment(number, f"A copy of #{kept}, which github-sync keeps for `{marker}`.")
            _close(client, records[number], "duplicate")
            print(f"closed #{number} (a copy of #{kept})")


def _step_numbers(spec, marked):
    """Each of a spec's step ids to its kept issue's number."""
    return {step.step: marked[step.marker]["number"] for step in spec.steps if step.marker in marked}


def _update_bodies(client, specs, marked):
    """Rewrites each open step issue whose body differs from what the spec now gives."""
    for spec in specs:
        numbers = _step_numbers(spec, marked)
        for step in spec.steps:
            record = marked.get(step.marker)
            body = step.render(numbers)
            if record is None or record["state"] != "open" or record["body"] == body:
                continue
            client.update_issue_body(record["number"], body)
            record["body"] = body
            print(f"updated #{record['number']} ({step.marker})")


def _gone_comment(spec, path, step, marked):
    """Names the steps a gone step was split into, or else the spec's steps now."""
    split = [s for s in spec.steps if s.lines.split_from == step] if spec else []
    if split:
        into = [s.step for s in split]
        return step_lines.split_comment(step, path, into, split[0].lines.split_note, _step_numbers(spec, marked))
    ids = ", ".join(s.step for s in spec.steps) if spec else "none"
    return f"No `### Step {step}` heading in `{path}` any more; the spec's steps are now: {ids}."


def _close_gone(client, specs, marked):
    planned = {step.marker for spec in specs for step in spec.steps}
    by_number = {spec.number: spec for spec in specs}
    for marker, record in marked.items():
        if not marker.startswith("step ") or marker in planned or record["state"] != "open":
            continue
        number, step = marker[len("step ") :].split("-", 1)
        spec = by_number.get(number)
        path = spec.path if spec else f"specs/{number}-*.md"
        client.comment(record["number"], _gone_comment(spec, path, step, marked))
        _close(client, record, "not_planned")
        print(f"closed #{record['number']} ({marker} is gone)")


def _close_built(client, specs, marked, records, built_by):
    """Closes each open built step's issue against the merged PR that added its heading."""
    for spec in specs:
        for step in spec.steps:
            record = marked.get(step.marker)
            if not step.built or record is None or record["state"] != "open":
                continue
            pr = built_by(spec.path, step.built)
            if pr is None or not records.get(pr, {}).get("merged"):
                continue
            client.comment(record["number"], f"Built by #{pr}, which added this step to the spec's Built section.")
            _close(client, record, "completed")
            print(f"closed #{record['number']} (built by #{pr})")


def apply(client, specs, milestones, own, built_by=None):
    """Creates the missing parents and steps, rewrites open step bodies the spec has changed,
    closes later copies of a marker as duplicates, closes the sync's steps that are gone, and
    with `built_by` (built_by_git) the built steps whose PR merged. `own` is what read()
    found; its records take the run's writes."""
    copies, merged, records = own
    for spec in specs:
        milestone = milestones[spec.milestone]
        if _missing(spec, copies):
            _read_milestone(client, spec, milestone, copies, records)
        _create_spec(client, spec, milestone, _kept(copies), merged, records)
    _close_copies(client, copies)
    made, _ = _own(list(records.values()))  # this run's new steps too
    _update_bodies(client, specs, _kept(made))
    _close_gone(client, specs, _kept(made))
    if built_by is not None:
        _close_built(client, specs, _kept(made), records, built_by)
