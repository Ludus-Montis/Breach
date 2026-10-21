"""Moves OPEN_QUESTIONS.md's open entries to issues labelled `question` (spec 36 step 1).

Each entry becomes an issue `Q<n>. <title>` with a `raise-when:` label, and the spec's
milestone when **Raise when** names one spec. An entry whose `Q<n>.` issue exists is
skipped. Prints `Q<n> -> #<issue>` per entry: the table step 2 puts in the file.

Usage (the github-sync workflow, after sync.py): GITHUB_TOKEN=... GITHUB_REPOSITORY=owner/repo
python3 ci/github/migrate_questions.py
"""
import re
import sys
from dataclasses import dataclass
from pathlib import Path

import labels

QUESTIONS = Path(__file__).resolve().parents[2] / "OPEN_QUESTIONS.md"
OPEN_SECTION = re.compile(r"^## Open\s*$(.*?)(?=^## |\Z)", re.MULTILINE | re.DOTALL)
ENTRY_HEADING = re.compile(r"^### Q(\d+)\.\s*(.+?)\s*$", re.MULTILINE)
RAISED_LINE = re.compile(r"^\*\*Raised:\*\*\s*(.*?)\s*·\s*\*\*Raise when:\*\*\s*(.*?)\s*$")
SETTLED_LINE = re.compile(r"^\*\*Possibly settled by:\*\*\s*(.*?)\s*$")
SPEC_NAMED = re.compile(r"\bspec (\d+)", re.IGNORECASE)
NEXT_DESIGN = re.compile(r"^(the )?next design session\b", re.IGNORECASE)
ISSUE_NUMBER = re.compile(r"^Q(\d+)\.")


@dataclass
class Entry:
    number: int
    title: str
    raised: str
    raise_when: str
    settled_by: str
    body: str


def _entry(number, title, text):
    lines = text.strip("\n").splitlines()
    raised, raise_when = RAISED_LINE.match(lines[0]).groups() if lines else ("", "")
    rest = lines[1:]
    settled = SETTLED_LINE.match(rest[0]) if rest else None
    if settled:
        rest = rest[1:]
    return Entry(
        int(number), title, raised, raise_when, settled.group(1) if settled else "", "\n".join(rest).strip()
    )


def parse(text):
    """The entries under `## Open`, in file order."""
    section = OPEN_SECTION.search(text)
    if not section:
        return []
    parts = ENTRY_HEADING.split(section.group(1))
    return [_entry(*parts[i : i + 3]) for i in range(1, len(parts), 3)]


def _raise_when(text, milestones):
    specs = {number.zfill(2) for number in SPEC_NAMED.findall(text)}
    if len(specs) == 1:
        spec = specs.pop()
        milestone = next((m for m in milestones if m.startswith(f"Spec {spec}:")), None)
        return labels.raise_when_spec_label(spec), milestone
    if NEXT_DESIGN.match(text.strip()):
        return "raise-when:next-design", None
    return "raise-when:other", None


def issue_for(entry, milestones):
    """The issue an entry becomes; `milestone` is a title, or None."""
    label, milestone = _raise_when(entry.raise_when, milestones)
    fields = [f"**Raised:** {entry.raised}", f"**Raise when:** {entry.raise_when}"]
    if entry.settled_by:
        fields.append(f"**Possibly settled by:** {entry.settled_by}")
    body = "\n".join(fields) + f"\n\n{entry.body}\n\n_Moved from `OPEN_QUESTIONS.md` (Decision 148)._"
    return {
        "title": f"Q{entry.number}. {entry.title}",
        "body": body,
        "labels": ["question", label],
        "milestone": milestone,
    }


def migrate(client, text):
    milestones = client.list_milestones()
    existing = {}
    for number, title in client.list_issues():
        match = ISSUE_NUMBER.match(title)
        if match:
            existing.setdefault(int(match.group(1)), number)
    for entry in parse(text):
        if entry.number not in existing:
            issue = issue_for(entry, milestones)
            milestone = milestones.get(issue["milestone"])
            existing[entry.number] = client.create_issue(
                issue["title"], issue["body"], issue["labels"], milestone
            )
        print(f"Q{entry.number} -> #{existing[entry.number]}")


def main():
    from github_client import GitHubClient

    migrate(GitHubClient.from_env(), QUESTIONS.read_text(encoding="utf-8"))
    return 0


if __name__ == "__main__":
    sys.exit(main())
