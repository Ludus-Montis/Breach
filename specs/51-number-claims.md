---
spec_type: code
status: active
parent_spec: ../Breach — Reverse Tower Defense Design Spec.md
---

# Spec 51: A number belongs to the first PR or issue that takes it

## Purpose

Spec numbers, Decision numbers and question numbers are each "the highest on `main`, plus
one", read when a session starts. Two sessions that start before either merges take the same
number. It happened with spec 46 and Decision 173 (#875, then #878, renumbered by hand to
spec 47 and Decision 177). With `/cascade` running steps side by side (Decision 171) and
several design sessions at once, it will happen more (Q46, #884).

Decision 181 settles it: the first claimant keeps the number. A PR claims by being opened,
so a session opens its PR, as a draft, as soon as it picks its numbers. A check fails a PR
that adds a number already held by `main` or an earlier open PR, and names the next free one.
`Q<n>` stays, and `github-sync` flags a duplicate when a question issue is opened.

## Status

Agreed in the design session of 2026-11-12 (the owner chose option 5 of #884, and keeping
`Q<n>` with a check). Three steps, each its own PR, built with `/run-phase` as
"spec 51 step N".
- Steps 1 and 3 can be built side by side. Step 2 needs step 1.
- None of them touches `native/`, `.github/workflows/native-store*`, `release*` or
  `ci/release/` (specs 49 and 50 are being built there).

## Pre-mortem

- **Regret in a month:** a clash reaches `main` anyway, because a session worked for hours on
  a branch with no PR, or because a check that passed went stale when another PR changed. On
  `main`, `github-sync` would give two specs the same `step:NN-<id>` labels and `/cascade`
  would route steps to the wrong spec.
- **Riskiest assumption:** that every number in use is visible to the check when it runs: on
  `main`, or in an open PR's added numbers, read correctly from its diff (a new
  `specs/NN-*.md`, a new `### Decision N` heading), and not confused by an edit to an existing
  heading.
- **Cheapest test:** replay the known clash. #875's tree (spec 46, Decisions 173–176) as the
  earlier PR and #878's first tree (spec 46, Decision 173) as the later one: the check must
  fail the later one and name spec 47 and Decision 177, and pass the earlier one. Scheduled as
  step 1's first test.

## Steps

### Step 1, the number check

**Needs:** none
**Touches:** `ci/github/check_numbers.py`, `ci/github/test_check_numbers.py`,
`ci/github/fixtures/numbers/` (new)

- **What a tree holds.** A tree's spec numbers are the `NN` of each `specs/NN-*.md`. Its
  Decision numbers are the `N` of each line `### Decision N —` in any Markdown file at the
  repo's root (the design spec and its addenda).
- **What a PR adds.** The numbers its head tree holds that its merge-base with `main` doesn't.
  An edit to an existing Decision's heading, or to an existing spec, adds nothing.
- **Who holds a number.** `main`, then each open PR, drafts included, in the order of their PR
  numbers. A PR fails if it adds a number that `main` holds, or that an open PR with a lower
  PR number adds. Closed PRs hold nothing.
- **The message.** One line for each clash: `spec 46 is held by #875; the next free spec is 47`
  (`main` in place of `#875` when `main` holds it). The next free number of each kind is one
  more than the highest held by `main` or any open PR, earlier or later, so following it never
  takes a later PR's number. A PR that clashes with nothing prints the numbers it holds.
- **A pure core.** The rule is a function of the trees' numbers and the PR numbers. Reading
  trees (git) and listing open PRs (the API) sit outside it, behind arguments, so the tests
  need neither.

Scenarios:
- Given #875's numbers as open PR 875 and #878's first numbers as PR 878, when PR 878 is
  checked, then it fails with spec 46 and Decision 173 named as held by #875, and the next
  free spec 47 and Decision 177; and when PR 875 is checked, it passes.
- Given a PR that only retitles `### Decision 170 —`, when it is checked, then it adds nothing
  and passes.
- Given a PR that adds spec 52 while `main` holds spec 52, then it fails, naming `main`.
- Given a later open PR that holds Decision 182 and an earlier one that adds it too, then the
  earlier one passes and the later one fails.

### Step 2, the check on every PR, and the words for sessions

**Needs:** 1
**Touches:** `.github/workflows/numbers.yml` (new), `ci/github/check_numbers.py` (its git and
API front), `ci/github/README.md`, `CLAUDE.md`, `.claude/skills/run-phase/SKILL.md`,
`AI_First_Development_Kit/principles/agent-sessions.md` (§3, one line)

- **The workflow.** On `pull_request` (opened, synchronize, reopened, ready_for_review), it runs
  step 1's tests, then the check: it lists open PRs (`gh pr list`), fetches each head
  (`refs/pull/N/head`) and reads each one's added numbers against its own merge-base. Read-only
  on contents.
- **A later PR is told too.** When this PR's run finds that a **later** open PR adds one of the
  numbers this PR adds, it sets a failing `numbers` commit status on that PR's head, with the
  same message. This PR's own result is the same `numbers` status (its job's result too). So a
  push to an earlier PR can't leave a later one green. A later PR that clashed with an earlier
  one that is since closed stays failed until it is re-run or pushed; the message says so.
- **The words.** `CLAUDE.md` (Design, and Non-negotiables' branch line) and `/run-phase` step 1:
  a session that adds a spec or a Decision pushes its branch and opens its PR, as a draft, as
  soon as it picks the numbers, before writing the body; `numbers` failing means take the
  numbers it names. `agent-sessions.md` §3: one line saying numbers are claimed by an early
  PR, checked in CI.

Scenarios:
- Given this PR (#N), when its `numbers` run passes, then the run's summary lists the numbers
  it holds.
- (The workflow itself is checked by the first PR after merge: step 2's own PR adds no number,
  so it passes; the next design PR's run shows its numbers.)

### Step 3, duplicate question numbers

**Needs:** none
**Touches:** `ci/github/question_numbers.py` (new), `ci/github/test_question_numbers.py`,
`.github/workflows/github-sync.yml` (the `issues` trigger and one job step),
`OPEN_QUESTIONS.md` (the header's "Raise one")

- **When.** `github-sync` also runs on `issues: opened` and `labeled`, for an issue labelled
  `question` whose title starts `Q<n>.`.
- **The rule.** Another `question` issue (open or closed) with the same `Q<n>`, or an entry
  `Q<n>` under `OPEN_QUESTIONS.md`'s Closed, holds the number if it came first (a lower issue
  number; the archive comes before every issue). Then the sync comments once on the later issue:
  `Q46 is held by #884; the next free is Q50`. It doesn't retitle; the session or the owner
  does.
- **The header.** `OPEN_QUESTIONS.md`'s "Raise one" says to create the issue at once after
  choosing the number, and that the sync flags a duplicate.

Scenarios:
- Given issues #884 `Q46. …` and #900 `Q46. …`, when #900 is opened, then one comment on #900
  names #884 and the next free `Q<n>`; and #884 gets none.
- Given an issue whose `Q<n>` is only in the archive, then it gets the comment, naming the
  archive.
- Given the comment is already there, when the issue is labelled again, then no second one.

## Not in this spec

- **Renumbering by a bot.** The check names the free number; the session renumbers.
- **Branches without a PR.** They hold nothing (Decision 181). No scan of branches.
- **Making `numbers` a required check.** Branch protection is the owner's setting.

## Test-first order

1. Step 1: red on the #875/#878 replay (`python3 -m unittest ci/github/test_check_numbers.py`,
   alone): the module is missing. Then the other scenarios one at a time.
2. Step 2: step 1's tests stay green; the workflow is read in review and seen on its own PR.
3. Step 3: red on the duplicate comment (`test_question_numbers.py`, alone, with
   `fake_client.py`), then the archive and the no-second-comment cases.

## Rubric answers

- **single-noun-phrase:** the number check; the check on every PR; duplicate question numbers.
- **ocp-extension-point:** a new kind of number is one more reader of a tree's numbers.
- **lsp-contract-scope:** not applicable; no subtypes.
- **isp-fit:** the core takes numbers and PR numbers only; git and the API are outside it.
- **dip-direction:** the rule depends on plain sets of numbers, never on the API's types.
- **general-over-special:** general. One rule, first claimant keeps the number, for specs,
  Decisions and questions, and for every kind of session.
- **pre-mortem:** the riskiest assumption is tested by step 1's first test (the replay); the
  stale-check case is answered in step 2 (a later PR is told).
- **order-independent:** not applicable; no simulation outcome changes. Claims rank by when
  PRs and issues were opened, on purpose.
- **bounded-work:** not applicable; CI only. One read of `main` and of each open PR.

## Built

(Each step's PR adds its line here.)
