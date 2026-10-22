"""Plans the labels, milestones and step issues the tree implies and creates the missing ones.

Spec 36 step 1 and spec 40 step 1, Decisions 148 and 155. Every label in labels.FIXED; per
`specs/NN-*.md` a milestone `Spec NN: <title>`, a `raise-when:spec-NN` label, a
`step:NN-<id>` label per `### Step <id>` heading, and a parent issue with a sub-issue per
step (step_issues.py). It creates only what is missing and never renames or deletes; a
step issue whose heading is gone is closed as not planned. On a pull request it creates
labels and the changed spec's milestone and sets it on the PR (spec_pr.py), nothing more.

Usage (the github-sync workflow): GITHUB_TOKEN=... GITHUB_REPOSITORY=owner/repo
GITHUB_EVENT_NAME=... [GITHUB_EVENT_PATH=...] python3 ci/github/sync.py
"""
import json
import os
import re
import sys
from dataclasses import dataclass, field
from pathlib import Path

import labels
import spec_pr
import step_issues

ROOT = Path(__file__).resolve().parents[2]
SPECS_DIR = ROOT / "specs"
SPEC_FILE = re.compile(r"^(\d+)-.*\.md$")
SPEC_HEADING = re.compile(r"^# Spec \d+:\s*(.+?)\s*$", re.MULTILINE)
FIRST_HEADING = re.compile(r"^# (.+?)\s*$", re.MULTILINE)

closed_issues = spec_pr.closed_issues


@dataclass
class Plan:
    labels: list = field(default_factory=list)
    milestones: list = field(default_factory=list)
    specs: list = field(default_factory=list)


def _title(text, spec):
    heading = SPEC_HEADING.search(text) or FIRST_HEADING.search(text)
    return f"Spec {spec}: {heading.group(1) if heading else 'untitled'}"


def plan(specs_dir=SPECS_DIR, repo_url=""):
    result = Plan(labels=list(labels.FIXED))
    for path in sorted(Path(specs_dir).glob("*.md")):
        match = SPEC_FILE.match(path.name)
        if not match:
            continue
        spec = match.group(1)
        text = path.read_text(encoding="utf-8")
        milestone = _title(text, spec)
        result.milestones.append(milestone)
        result.labels.append(
            (labels.raise_when_spec_label(spec), labels.RAISE_WHEN_COLOR, f"Raise when spec {spec} is reached")
        )
        issues = step_issues.plan_spec(spec, f"specs/{path.name}", milestone, text, repo_url)
        for step in issues.steps:
            result.labels.append((step.labels[0], labels.STEP_COLOR, f"Spec {spec}, step {step.step}"))
        result.specs.append(issues)
    return result


def apply_labels(client, wanted):
    have_labels = set(client.list_labels())
    for name, color, description in wanted.labels:
        if name not in have_labels:
            client.create_label(name, color, description)
            have_labels.add(name)
            print(f"label {name}")


def apply(client, wanted):
    """Creates every planned label, milestone and step issue the client doesn't report."""
    own = step_issues.read(client)
    apply_labels(client, wanted)
    have_milestones = client.list_milestones()
    for title in wanted.milestones:
        if title not in have_milestones:
            have_milestones[title] = client.create_milestone(title)
            print(f"milestone {title}")
    step_issues.apply(client, wanted.specs, have_milestones, own)


def apply_pull_request(client, wanted, pr_number):
    """Labels, and the milestone of the spec the PR changes on the PR and what it closes."""
    apply_labels(client, wanted)
    spec_pr.apply(client, {spec.number: spec.milestone for spec in wanted.specs}, pr_number)


def main():
    from github_client import GitHubClient

    client = GitHubClient.from_env()
    wanted = plan(repo_url=f"{os.environ.get('GITHUB_SERVER_URL', 'https://github.com')}/{os.environ['GITHUB_REPOSITORY']}")
    if os.environ.get("GITHUB_EVENT_NAME") == "pull_request":
        event = json.loads(Path(os.environ["GITHUB_EVENT_PATH"]).read_text(encoding="utf-8"))
        apply_pull_request(client, wanted, event["pull_request"]["number"])
    else:
        apply(client, wanted)
    return 0


if __name__ == "__main__":
    sys.exit(main())
