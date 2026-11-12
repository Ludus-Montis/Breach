"""The number check (spec 51 step 1, Decision 181): a spec or Decision number belongs to the
first PR that takes it.

`main` holds the numbers its tree holds. Each open PR, drafts included, adds the numbers its
head tree holds that its merge-base with `main` doesn't, and holds them unless `main` or an
open PR with a lower PR number already does. A PR fails if it adds a number held by `main` or
by an earlier open PR; the message names the holder and the next free number of that kind,
one more than the highest held by `main` or any open PR, so following it never takes a later
PR's number. Closed PRs and branches without a PR hold nothing.

The rule (`check`, `statuses`) takes plain sets of numbers and PR numbers. The front (spec 51
step 2) reads trees from git (`git_numbers`), lists open PRs with `gh` and sets each PR's
`numbers` commit status; the `numbers` workflow runs it as `python3 ci/github/check_numbers.py
<pr>` (`main`, below).
"""
import json
import os
import re
import subprocess
import sys
from dataclasses import dataclass, field
from pathlib import Path

SPEC_FILE = re.compile(r"^(\d+)-.*\.md$")
DECISION_HEADING = re.compile(r"^### Decision (\d+) —", re.MULTILINE)
KINDS = (("specs", "spec", "specs"), ("decisions", "Decision", "Decisions"))


@dataclass(frozen=True)
class Numbers:
    """The spec and Decision numbers a tree holds, or a PR adds."""

    specs: frozenset = field(default_factory=frozenset)
    decisions: frozenset = field(default_factory=frozenset)

    def __post_init__(self):
        object.__setattr__(self, "specs", frozenset(self.specs))
        object.__setattr__(self, "decisions", frozenset(self.decisions))


@dataclass
class Verdict:
    pr: int
    holds: Numbers
    clashes: list  # (singular kind, number, holder: "main" or "#N", next free)

    @property
    def passed(self):
        return not self.clashes

    def lines(self):
        if self.clashes:
            return [
                f"{kind} {number} is held by {holder}; the next free {kind} is {free}"
                for kind, number, holder, free in self.clashes
            ]
        parts = []
        for attr, one, many in KINDS:
            numbers = sorted(getattr(self.holds, attr))
            if numbers:
                label = one if len(numbers) == 1 else many
                parts.append(f"{label} {', '.join(str(n) for n in numbers)}")
        if not parts:
            return [f"#{self.pr} adds no spec or Decision number"]
        return [f"#{self.pr} holds {'; '.join(parts)}"]


def tree_numbers(root):
    """The numbers a checked-out tree holds: the `NN` of each `specs/NN-*.md`, and the `N` of
    each `### Decision N —` line in a Markdown file at the root."""
    root = Path(root)
    specs_dir = root / "specs"
    specs = set()
    if specs_dir.is_dir():
        for path in specs_dir.iterdir():
            match = SPEC_FILE.match(path.name)
            if match and path.is_file():
                specs.add(int(match.group(1)))
    decisions = set()
    for path in root.glob("*.md"):
        text = path.read_text(encoding="utf-8")
        decisions.update(int(n) for n in DECISION_HEADING.findall(text))
    return Numbers(specs=specs, decisions=decisions)


def added(head, base):
    """The numbers `head` holds that `base` doesn't. Editing an existing one adds nothing."""
    return Numbers(specs=head.specs - base.specs, decisions=head.decisions - base.decisions)


def check(pr, main, open_prs):
    """Check PR `pr` against `main`'s numbers and `open_prs` (PR number -> the numbers it
    adds; it includes `pr` itself)."""
    adds = open_prs.get(pr, Numbers())
    clashes = []
    for attr, one, _ in KINDS:
        held = getattr(main, attr)
        taken = set(held).union(*(getattr(n, attr) for n in open_prs.values()))
        free = max(taken, default=0) + 1
        for number in sorted(getattr(adds, attr)):
            holder = _holder(number, attr, pr, held, open_prs)
            if holder:
                clashes.append((one, number, holder, free))
    return Verdict(pr=pr, holds=adds, clashes=clashes)


def _holder(number, attr, pr, held_by_main, open_prs):
    if number in held_by_main:
        return "main"
    for earlier in sorted(n for n in open_prs if n < pr):
        if number in getattr(open_prs[earlier], attr):
            return f"#{earlier}"
    return None


STATUS_LIMIT = 140  # GitHub cuts a commit status's description at 140 characters.


def statuses(pr, main, open_prs):
    """The `numbers` statuses a run for `pr` sets: (PR, "success" or "failure", description).
    `pr`'s own, then a failure on each later open PR that adds a number `pr` adds and fails
    the check, so a push to an earlier PR can't leave a later one green."""
    own = check(pr, main, open_prs)
    result = [(pr, "success" if own.passed else "failure", "; ".join(own.lines()))]
    adds = open_prs.get(pr, Numbers())
    for later in sorted(n for n in open_prs if n > pr):
        theirs = open_prs[later]
        if not (adds.specs & theirs.specs or adds.decisions & theirs.decisions):
            continue
        verdict = check(later, main, open_prs)
        if not verdict.passed:
            note = f" (set by #{pr}'s run; re-run or push to check again)"
            result.append((later, "failure", "; ".join(verdict.lines()) + note))
    return result


def git_numbers(repo, ref):
    """The numbers the tree at `ref` holds, read from git with no checkout."""

    def git(*args):
        out = subprocess.run(
            ["git", "-C", str(repo), *args], check=True, capture_output=True, text=True
        )
        return out.stdout

    specs = set()
    # -z: no quoting, so a name with an em dash (the design spec's) reads as itself.
    for name in git("ls-tree", "-z", "--name-only", ref, "specs/").split("\0"):
        match = SPEC_FILE.match(name.removeprefix("specs/"))
        if match:
            specs.add(int(match.group(1)))
    decisions = set()
    for line in filter(None, git("ls-tree", "-z", ref).split("\0")):
        meta, name = line.split("\t", 1)
        if meta.split()[1] == "blob" and name.endswith(".md"):
            text = git("show", f"{ref}:{name}")
            decisions.update(int(n) for n in DECISION_HEADING.findall(text))
    return Numbers(specs=specs, decisions=decisions)


def _run(*args):
    return subprocess.run(args, check=True, capture_output=True, text=True).stdout


def _pr_adds(pr, base_ref):
    """What open PR `pr` adds over its merge-base with its own base branch (`main`, or the
    branch a stacked PR targets), fetched as `refs/pull/<pr>/head`."""
    head = f"refs/numbers/{pr}"
    _run("git", "fetch", "-q", "origin", f"+refs/pull/{pr}/head:{head}")
    base = f"refs/remotes/origin/{base_ref}"
    fetched = subprocess.run(
        ["git", "fetch", "-q", "origin", f"+refs/heads/{base_ref}:{base}"], capture_output=True
    )
    if fetched.returncode != 0:
        base = "refs/remotes/origin/main"  # its base branch is gone
    merge_base = _run("git", "merge-base", head, base).strip()
    sha = _run("git", "rev-parse", head).strip()
    return added(git_numbers(".", head), git_numbers(".", merge_base)), sha

def main(argv):
    """Check PR `argv[0]` in the checked-out repo, set the `numbers` statuses, and exit 1 if
    it fails. Needs `gh` with GH_TOKEN and GITHUB_REPOSITORY."""
    pr = int(argv[0])
    listed = json.loads(
        _run("gh", "pr", "list", "--state", "open", "--limit", "500", "--json", "number,baseRefName")
    )
    _run("git", "fetch", "-q", "origin", "+refs/heads/main:refs/remotes/origin/main")
    open_prs, heads = {}, {}
    for item in listed:
        open_prs[item["number"]], heads[item["number"]] = _pr_adds(
            item["number"], item["baseRefName"]
        )
    main_numbers = git_numbers(".", "origin/main")
    verdict = check(pr, main_numbers, open_prs)
    report = [f"## numbers: #{pr}", "", *(f"- {line}" for line in verdict.lines())]
    for number, state, description in statuses(pr, main_numbers, open_prs):
        if number not in heads:
            continue  # this PR closed meanwhile
        _run(
            "gh", "api", f"repos/{os.environ['GITHUB_REPOSITORY']}/statuses/{heads[number]}",
            "-f", f"state={state}", "-f", "context=numbers",
            "-f", f"description={description[:STATUS_LIMIT]}",
        )
        if number != pr:
            report.append(f"- told #{number}: {description}")
    print("\n".join(report))
    summary = os.environ.get("GITHUB_STEP_SUMMARY")
    if summary:
        with open(summary, "a", encoding="utf-8") as out:
            out.write("\n".join(report) + "\n")
    return 0 if verdict.passed else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
