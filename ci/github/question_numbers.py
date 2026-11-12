"""Flags a question issue whose `Q<n>` is already held (spec 51 step 3, Decision 181).

A number belongs to whoever took it first: an entry under `OPEN_QUESTIONS.md`'s Closed (the
archive comes before every issue), else the lowest-numbered issue labelled `question`, open
or closed, titled `Q<n>. …`. A later issue with that number gets one comment naming the
holder and the next free number: one more than the highest held anywhere. It never
retitles; the session or the owner does.

Usage (the github-sync workflow, on `issues: opened` and `labeled`):
GITHUB_TOKEN=... GITHUB_REPOSITORY=owner/repo GITHUB_EVENT_PATH=... python3 ci/github/question_numbers.py
"""
import json
import os
import re
import sys
from pathlib import Path

QUESTIONS = Path(__file__).resolve().parents[2] / "OPEN_QUESTIONS.md"
LABEL = "question"
TITLE = re.compile(r"^Q(\d+)\.")
ARCHIVE_ENTRY = re.compile(r"^(?:- |### )Q(\d+)\b")
MARKER = "<!-- github-sync: question-number -->"
ARCHIVE_NAME = "`OPEN_QUESTIONS.md`'s Closed archive"


def archive_numbers(text):
    """The `Q<n>` of each entry under the Closed section, outside code fences."""
    numbers, section, fenced = set(), None, False
    for line in text.splitlines():
        if line.startswith("```"):
            fenced = not fenced
        elif fenced:
            continue
        elif line.startswith("## "):
            section = line[3:].strip()
        elif section == "Closed" and (match := ARCHIVE_ENTRY.match(line)):
            numbers.add(int(match.group(1)))
    return numbers


def q_number(title):
    match = TITLE.match(title)
    return int(match.group(1)) if match else None


def clash(issue, held_by_issues, archive):
    """The comment for issue `issue` (its number and Q<n>), or None when it holds its number.

    `held_by_issues` maps each question issue's number to its Q<n>; `archive` is a set."""
    number, q = issue
    if q in archive:
        holder = ARCHIVE_NAME
    else:
        earlier = [n for n, other in held_by_issues.items() if other == q and n < number]
        if not earlier:
            return None
        holder = f"#{min(earlier)}"
    next_free = max(set(held_by_issues.values()) | archive) + 1
    return f"Q{q} is held by {holder}; the next free is Q{next_free}"


def apply(client, number, archive_text):
    issue = client.get_issue(number)
    if not issue or issue["pull_request"] or LABEL not in issue["labels"]:
        return
    q = q_number(issue["title"])
    if q is None:
        return
    questions = {
        record["number"]: q_number(record["title"])
        for record in client.list_issue_records()
        if LABEL in record["labels"] and not record["pull_request"] and q_number(record["title"]) is not None
    }
    questions[number] = q
    message = clash((number, q), questions, archive_numbers(archive_text))
    if message is None:
        print(f"Q{q} on #{number}: free")
        return
    if any(MARKER in body for body in client.list_comments(number)):
        print(f"Q{q} on #{number}: already flagged")
        return
    client.comment(number, f"{message}. Retitle this issue with the free number.\n\n{MARKER}")
    print(f"Q{q} on #{number}: {message}")


def main():
    from github_client import GitHubClient

    event = json.loads(Path(os.environ["GITHUB_EVENT_PATH"]).read_text(encoding="utf-8"))
    apply(GitHubClient.from_env(), event["issue"]["number"], QUESTIONS.read_text(encoding="utf-8"))
    return 0


if __name__ == "__main__":
    sys.exit(main())
