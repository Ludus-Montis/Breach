---
spec_type: code
status: active
parent_spec: ../Breach — Reverse Tower Defense Design Spec.md
---

# Spec 40: Step issues and the Project's status

## Purpose

The owner's Project shows the work still to come in each spec, and what is moving now. Each
spec gets a parent issue with a sub-issue per step, the questions and design PRs behind a
spec join its milestone, a step's PR closes its issue, and each PR (and the issues it
closes) sits in the Status column its state implies.

**Settled by Decision 155** (amends Decision 148). Read it by its line in
`DECISIONS_INDEX.md`. It answers Q31 (#287).

## Status

Agreed in the design session of 2026-10-22. Three steps, each its own PR, built with
`/run-phase` as "spec 40 step N". Step 1 must merge first: its sync, run by its own merge,
creates the issues step 2's skills look up. Step 3 needs the owner's token (below) before
its workflow can run, not before it is built.

## What runs where

- **Cloud agents** use the GitHub MCP tools. They can create, update and close issues, set
  a PR's labels and milestone, and attach sub-issues, but can't create labels or
  milestones or touch a Project (spec 36).
- **`github-sync`** (`GITHUB_TOKEN`, `issues: write`, `pull-requests: write`) creates
  labels, milestones and issues, and sets milestones.
- **`project-status`** (a new workflow, the owner's `PROJECT_TOKEN` secret) sets the
  Project's Status field. `GITHUB_TOKEN` can't write to a user-owned Project.

## Steps

### Step 1, the sync makes step issues and runs itself

Extend `ci/github/sync.py`, `github_client.py` and the workflow. Stdlib Python, tests
beside them, the API behind the client the tests replace (`fake_client.py`).

- **Plan, per spec:** the milestone (as today); a parent issue `Spec NN: <title>`; one step
  per distinct id among the spec's `### Step <id>` headings, wherever they sit (the same
  ids as the `step:` labels; specs 35 and 37 keep theirs under `## Built` and
  `## Scenarios`). A step's title is `Spec NN step <id>: <title>`, the title from the first
  heading of that id that has text after the comma, else `Spec NN step <id>`.
- **Bodies point, never copy:** the parent's body links `specs/NN-*.md`; a step's body
  names its heading and links the file. Each body carries a marker line
  `<!-- github-sync: spec NN -->` or `<!-- github-sync: step NN-<id> -->`, which is how the
  sync knows an issue is its own. It never edits a body after creating it.
- **Apply on push to `main`:** create the missing parent and step issues in the spec's
  milestone, steps labelled `step:NN-<id>` and attached as sub-issues of the parent (REST
  `POST /repos/{o}/{r}/issues/{n}/sub_issues` with the child's issue id). A step whose id
  has a merged PR labelled `step:NN-<id>` is created closed (completed).
- **A step that is gone:** an open issue with a sync step marker whose id has no heading
  left is closed as not planned, with one comment: "No `### Step <id>` heading in
  `specs/NN-*.md` any more; the spec's steps are now: <ids>." It leaves it under the parent.
- **Apply on a PR that changes `specs/NN-*.md`:** create each such spec's milestone if
  missing; set it on the PR and on each open issue the PR's body names with
  `Closes #N`, `Fixes #N` or `Resolves #N`, when that item has no milestone. One spec per
  PR is the normal case; if a PR changes several specs, the lowest-numbered one is used,
  and the run says so. No step issues are made from a PR: the spec may change in review.
- **The workflow:** `on: push` to `main` with `paths: ["specs/**"]`;
  `on: pull_request` (opened, edited, synchronize, reopened) with the same paths; and
  `workflow_dispatch` with `migrate` as now. A `concurrency` group so two runs never
  overlap. Labels are still created on every run.
- **The `handoff` label** joins `labels.FIXED` (a flag: "parked; waiting to be picked up").

### Step 2, the skills use the step issues

- `/run-phase`, section 6: after the PR's `issue_write`, find the step's issue with
  `list_issues`, `labels: [step:NN-<id>]`, `state: OPEN`, `fields: [number, title]`, and
  say `Closes #<it>` in the PR body. If none is found (the sync hasn't run), say so in the
  PR and trigger `github-sync`, rather than wait. A PR that builds two steps closes both.
- `/handoff`: a parked draft PR gets the `handoff` label (in the same `issue_write` as its
  other labels) and still says `Closes #N` for its step. Picking up removes the label
  (`issue_write` with the PR's labels less `handoff`) before the first push.
- Design sessions (`CLAUDE.md`'s line, and `/handoff`'s design section): a design PR that
  writes or extends a spec says `Closes #N` for each question it settles. The sync
  gives it and them the milestone, so the session sets none.
- `CLAUDE.md`'s "where things live" table: specs' steps are issues under `Spec NN:`, made
  by `github-sync`; the spec stays the source.

### Step 3, the Project's Status follows each PR

`.github/workflows/project-status.yml` and `ci/github/project_status.py`, with tests.

- **Trigger:** `pull_request` (`opened`, `reopened`, `ready_for_review`,
  `converted_to_draft`, `labeled`, `unlabeled`), skipped when `PROJECT_TOKEN` is unset (the
  job says so and passes).
- **The rule** (a pure function of the PR's draft flag and labels, the tested core):

  | PR | Status |
  |---|---|
  | Draft with `handoff` | Next |
  | Draft without `handoff` | In progress |
  | Not a draft | In review |

  Merged or closed is left to the Project's built-in workflows (Done).
- **Apply** (GraphQL, `PROJECT_TOKEN`): find the Project by the repo variables
  `PROJECT_OWNER` and `PROJECT_NUMBER`; its Status field and the option by name (a missing
  option fails the job naming it); the PR's item, adding it if the built-in auto-add hasn't
  yet; `updateProjectV2ItemFieldValue`. Then the same for every issue in the PR's
  `closingIssuesReferences`, so a step and its PR share a column.
- A `labeled`/`unlabeled` event for any label but `handoff` does nothing.

## The owner's setup (not built by an agent)

Before step 3's workflow can run:
1. A fine-grained personal access token: resource owner you, access to `SSidey/Breach`,
   account permission **Projects: read and write**, repository permissions **Issues** and
   **Pull requests: read**. Store it as the repo secret `PROJECT_TOKEN`.
2. Repo variables `PROJECT_OWNER` (`SSidey`) and `PROJECT_NUMBER` (the number in the
   Project's URL).
3. The Project's Status options stay named Inbox, Next, In progress, In review, Done.
   The built-in "Pull request linked to issue" workflow can stay on: `project-status` runs
   after it and sets the same issue's column.

## Scenarios

### Step 1

**Scenario: a spec plans a parent and a step per id**
- Given spec 38 with `### Step 1a, groups bounded` under `## Steps` and `### Step 1a` again
  under `## Scenarios`
- When `sync.py` plans
- Then it holds the parent `Spec 38: Tick performance after the unit store` and one step
  `Spec 38 step 1a: groups bounded`, labelled `step:38-1a`

**Scenario: headings outside `## Steps` still count**
- Given spec 35, whose step headings are under `## Built`
- When `sync.py` plans
- Then it holds a step for each of `0`, `1a` … `7c`, 20 in all

**Scenario: a heading without a title**
- Given spec 37's `### Step 4`
- When `sync.py` plans
- Then the step is titled `Spec 37 step 4`

**Scenario: steps are created as sub-issues of the parent, in the milestone**
- Given no issues exist for spec 37
- When the sync applies on push
- Then it creates the parent and four step issues in `Spec 37: …`, each step labelled and
  attached to the parent

**Scenario: a merged step's issue is created closed**
- Given a merged PR labelled `step:37-1`
- When the sync creates step 37-1's issue
- Then the issue is closed as completed

**Scenario: a second run changes nothing**
- Given every planned issue exists with its marker
- When the sync applies
- Then it creates, closes and comments nothing

**Scenario: a split step's issue is closed as not planned**
- Given an open issue marked `step 38-1` and spec 38 with headings `1a` to `1d` and no `1`
- When the sync applies
- Then it closes that issue as not planned with one comment naming the spec's step ids,
  and creates `1a` to `1d`

**Scenario: the sync never closes what isn't its own**
- Given an open issue labelled `step:38-1` with no sync marker, and no heading `1`
- When the sync applies
- Then it leaves that issue open

**Scenario: a spec PR gets its milestone, and so do the questions it closes**
- Given a PR changing `specs/41-x.md` (`# Spec 41: X`), its body saying `Closes #300`, and
  no milestone `Spec 41: X`
- When the sync runs for the PR
- Then milestone `Spec 41: X` exists and is set on the PR and on issue #300, and no step
  issues exist for spec 41

**Scenario: a set milestone is kept**
- Given the PR's `Closes #301`, and #301 already in milestone `Spec 38: …`
- When the sync runs for the PR
- Then #301 keeps its milestone

### Step 2

No code; the skills' text. Checked by reading the PR (`/run-phase`'s own end-of-item gate).

### Step 3

**Scenario: the rule**
- Given PRs: a draft labelled `handoff`; a draft without it; a ready PR; a ready PR
  labelled `handoff`
- When `project_status.py` decides
- Then they get Next, In progress, In review and In review

**Scenario: the closed issues follow the PR**
- Given a ready PR whose `closingIssuesReferences` are #310 and #311
- When the workflow applies
- Then the PR, #310 and #311 are set to In review

**Scenario: an item not yet in the Project is added**
- Given the PR has no item in the Project
- When the workflow applies
- Then it adds the item, then sets its Status

**Scenario: another label does nothing**
- Given an `unlabeled` event for `sim`
- When the workflow runs
- Then it makes no Project call

**Scenario: no token, no failure**
- Given `PROJECT_TOKEN` is unset
- When the workflow runs
- Then the job passes and says the token is missing

**Scenario: an unknown Status option fails clearly**
- Given the Project's Status has no option `Next`
- When a parked hand-off is applied
- Then the job fails naming `Next`

## Test-first order

1. Step 1: `test_sync.py` red on the plan scenarios (fixtures of today's specs 35, 37, 38),
   then the plan; red on apply (the fake client grows issues, sub-issues, closing, comments,
   merged-PR lookup and milestones on items), then apply; then the PR path's `Closes`
   parsing; then the workflow file.
2. Step 2: the skills and `CLAUDE.md`.
3. Step 3: `test_project_status.py` red on the rule, then on apply with a fake GraphQL
   client, then the workflow file.

## Rubric answers

- **single-noun-phrase:** the step issue sync; the project status rule.
- **ocp-extension-point:** a new spec gets its parent and steps from its file; a new status
  rule is a row in one function.
- **lsp-contract-scope:** not applicable; no subtypes.
- **isp-fit:** the REST client gains only the calls the sync makes; the GraphQL client is
  separate and only `project_status.py` uses it.
- **dip-direction:** planning and the rule depend on the small clients, never on HTTP.
- **general-over-special:** general. Every step id has one issue; a PR's Status is a
  function of its state.
- **order-independent:** not applicable; nothing under `sim/` changes.
- **bounded-work:** not applicable to the tick. A sync run lists each kind once and writes
  only what differs; the status job makes a few calls per PR event.
