"""A PR that changes `specs/NN-*.md` gets the spec's milestone, and so do the issues it
closes (spec 40 step 1, Decision 155). It makes no step issues: the spec may change in
review. An item that already has a milestone keeps it.
"""
import re

SPEC_PATH = re.compile(r"^specs/(\d+)-[^/]*\.md$")
CLOSING = re.compile(r"\b(?:close[sd]?|fix(?:e[sd])?|resolve[sd]?):?\s+#(\d+)\b", re.IGNORECASE)


def closed_issues(body):
    """The issue numbers a PR body closes with Closes, Fixes or Resolves, in order."""
    return list(dict.fromkeys(int(n) for n in CLOSING.findall(body or "")))


def _spec_numbers(files):
    return sorted({match.group(1) for match in map(SPEC_PATH.match, files) if match}, key=int)


def _milestone(client, title, milestones):
    if title not in milestones:
        milestones[title] = client.create_milestone(title)
        print(f"milestone {title}")
    return milestones[title]


def _set_if_unset(client, number, milestone, title):
    record = client.get_issue(number)
    if record and record["milestone"] is None and record["state"] == "open":
        client.set_milestone(number, milestone)
        print(f"#{number} -> {title}")


def apply(client, titles, pr_number):
    """`titles` maps a spec number to its milestone title, from the PR's checkout."""
    numbers = [n for n in _spec_numbers(client.list_pr_files(pr_number)) if n in titles]
    if not numbers:
        return
    milestones = client.list_milestones()
    for number in numbers:
        _milestone(client, titles[number], milestones)
    title = titles[numbers[0]]
    if len(numbers) > 1:
        print(f"the PR changes specs {', '.join(numbers)}; using the lowest, {title}")
    milestone = milestones[title]
    pr = client.get_issue(pr_number)
    for number in [pr_number] + closed_issues(pr["body"]):
        _set_if_unset(client, number, milestone, title)
