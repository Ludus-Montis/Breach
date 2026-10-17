"""Plans the labels and milestones the tree implies and creates the missing ones.

Spec 36 step 1, Decision 148. Every label in labels.FIXED; per `specs/NN-*.md` a
milestone `Spec NN: <title>`, a `raise-when:spec-NN` label and a `step:NN-<id>` label per
`### Step <id>` heading. It creates only what is missing and never renames or deletes.

Usage (the github-sync workflow): GITHUB_TOKEN=... GITHUB_REPOSITORY=owner/repo
python3 ci/github/sync.py
"""
import re
import sys
from dataclasses import dataclass, field
from pathlib import Path

import labels

SPECS_DIR = Path(__file__).resolve().parents[2] / "specs"
SPEC_FILE = re.compile(r"^(\d+)-.*\.md$")
SPEC_HEADING = re.compile(r"^# Spec \d+:\s*(.+?)\s*$", re.MULTILINE)
FIRST_HEADING = re.compile(r"^# (.+?)\s*$", re.MULTILINE)
STEP_HEADING = re.compile(r"^### Step (\w+)(?:,|\s*$)", re.MULTILINE)


@dataclass
class Plan:
    labels: list = field(default_factory=list)
    milestones: list = field(default_factory=list)


def _title(text, spec):
    heading = SPEC_HEADING.search(text) or FIRST_HEADING.search(text)
    return f"Spec {spec}: {heading.group(1) if heading else 'untitled'}"


def plan(specs_dir=SPECS_DIR):
    result = Plan(labels=list(labels.FIXED))
    for path in sorted(Path(specs_dir).glob("*.md")):
        match = SPEC_FILE.match(path.name)
        if not match:
            continue
        spec = match.group(1)
        text = path.read_text(encoding="utf-8")
        result.milestones.append(_title(text, spec))
        result.labels.append(
            (labels.raise_when_spec_label(spec), labels.RAISE_WHEN_COLOR, f"Raise when spec {spec} is reached")
        )
        steps = dict.fromkeys(STEP_HEADING.findall(text))
        for step in steps:
            result.labels.append(
                (labels.step_label(spec, step), labels.STEP_COLOR, f"Spec {spec}, step {step}")
            )
    return result


def apply(client, wanted):
    """Creates every planned label and milestone the client doesn't report."""
    have_labels = set(client.list_labels())
    for name, color, description in wanted.labels:
        if name not in have_labels:
            client.create_label(name, color, description)
            have_labels.add(name)
            print(f"label {name}")
    have_milestones = set(client.list_milestones())
    for title in wanted.milestones:
        if title not in have_milestones:
            client.create_milestone(title)
            have_milestones.add(title)
            print(f"milestone {title}")


def main():
    from github_client import GitHubClient

    apply(GitHubClient.from_env(), plan())
    return 0


if __name__ == "__main__":
    sys.exit(main())
