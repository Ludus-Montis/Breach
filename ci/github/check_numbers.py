"""The number check (spec 51 step 1, Decision 181): a spec or Decision number belongs to the
first PR that takes it.

`main` holds the numbers its tree holds. Each open PR, drafts included, adds the numbers its
head tree holds that its merge-base with `main` doesn't, and holds them unless `main` or an
open PR with a lower PR number already does. A PR fails if it adds a number held by `main` or
by an earlier open PR; the message names the holder and the next free number of that kind,
one more than the highest held by `main` or any open PR, so following it never takes a later
PR's number. Closed PRs and branches without a PR hold nothing.

The rule (`check`) takes plain sets of numbers and PR numbers. Reading a tree from disk
(`tree_numbers`) is here too; reading trees from git and listing open PRs is the caller's.
"""
import re
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
