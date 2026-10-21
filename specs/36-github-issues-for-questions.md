---
spec_type: code
status: active
parent_spec: ../Breach — Reverse Tower Defense Design Spec.md
---

# Spec 36: GitHub issues for open questions

## Purpose

Open questions move from `OPEN_QUESTIONS.md` to GitHub issues, and each PR names its spec
(a milestone) and its step (a label), so the owner sees what is in flight on GitHub and in a
Project of their own. Decisions, specs, their steps and hand-offs stay in the repo.

**Settled by Decision 148.** Read it by its line in `DECISIONS_INDEX.md`. It answers Q30.

## Status

Agreed in the design session of 2026-10-17. Two steps, each its own PR, built with
`/run-phase` as "spec 36 step N". Between them the owner runs the migration (step 1's last
scenario), so the entries are still on `main` when it reads them.

**Built:** step 1 in [#254](https://github.com/SSidey/Breach/pull/254); the owner ran the
migration on 2026-10-21 (Q1 to Q26 became #257 to #281, the table in `OPEN_QUESTIONS.md`);
step 2 after it.

## What cloud agents can and can't do

Cloud sessions use the GitHub MCP tools, not `gh` (it is not logged in). They can create,
update, search and close issues, set an issue's or PR's labels and milestone by number
(`issue_write` with the PR's number: a PR is an issue), and trigger a workflow
(`actions_run_trigger`). They can't create labels or milestones, or touch a Project. So
labels and milestones are made by a workflow, and the Project by the owner.

## Steps

1. **The sync workflow and the migration.** Scripts in `ci/github/` (stdlib Python, tests
   beside them), and `.github/workflows/github-sync.yml`, run by hand (`workflow_dispatch`,
   input `migrate`, default false), with `permissions: issues: write` and `GITHUB_TOKEN`.
   - `labels.py` holds the fixed label set (Decision 148) with a colour and description each.
   - `sync.py` plans, from the tree, every label and milestone that should exist: the fixed
     set; a milestone `Spec NN: <title>` per `specs/NN-*.md` (the title from its `# Spec NN:`
     heading); a label `step:NN-<id>` per `### Step <id>,` heading in spec NN (the spec's number in the
     label, so `step:35-1` and `step:36-1` never share one). It reads what
     exists through the REST API and creates only what is missing. It never renames or
     deletes.
   - `migrate_questions.py` parses `OPEN_QUESTIONS.md`'s Open section into entries (number,
     title, Raised, Raise when, Possibly settled by, body) and creates one issue each, titled
     `Q<n>. <title>`, labelled `question` and one `raise-when:` label, with the spec's
     milestone when **Raise when** names one spec. It skips an entry whose title already
     exists, and prints `Q<n> -> #<issue>` per entry.
   - The `tools-python-tests` pre-commit hook also discovers `ci/github`.
2. **The register becomes a pointer, and the skills change.**
   - `OPEN_QUESTIONS.md`: the Open section is replaced by a pointer to issues labelled
     `question` and the `Q<n> -> #<issue>` table from the migration's log; the header says
     how to raise and close one; Closed stays as written.
   - `ci/github/check_open_questions.py` (and a pre-commit hook): fails when a `### Q` entry
     appears under Open.
   - `CLAUDE.md`: the "where things live" row, the design-session line and the "goes in the
     repo" line name issues for open questions. `principles/agent-sessions.md` rule 5 says
     the register may be an issue tracker with the same fields, pointing to Decision 148.
   - `/run-phase`: list open `question` issues (`list_issues`, `labels: [question]`,
     `fields: [number, title, labels]`) instead of reading the file; open the PR with
     `create_pull_request` (not `gh pr create`); then one `issue_write` on the PR's number
     setting the spec's milestone, `step:NN-<id>`, kind and area labels; `Closes #N` in the body
     for a question it settles; if a label or milestone is missing, trigger `github-sync`
     and say so rather than wait.
   - `/handoff`: an open question becomes an issue (`issue_write`, `question` and a
     `raise-when:` label); the PR's *Open questions* line lists issue numbers.
   - Design sessions raise questions the same way, and a design PR that settles one says
     `Closes #N`.

## Scenarios

### Step 1

**Scenario: the plan names the labels and milestones the tree implies**
- Given specs `35-unit-store.md` (`# Spec 35: The unit store`, headings `### Step 1a, …` and
  `### Step 7b, …`) and `36-github-issues-for-questions.md`
- When `sync.py` plans
- Then the plan holds every fixed label, milestones `Spec 35: The unit store` and
  `Spec 36: GitHub issues for open questions`, and labels `step:35-1a` and `step:35-7b`

**Scenario: two specs' steps of the same name get two labels**
- Given spec 35 and spec 36 each have a `### Step 1,` heading
- When `sync.py` plans
- Then it holds `step:35-1` and `step:36-1`, and no `step:1`

**Scenario: a second run changes nothing**
- Given the API reports every planned label and milestone as existing
- When `sync.py` runs
- Then it creates nothing, and renames and deletes nothing

**Scenario: only what is missing is created**
- Given the API reports `question` and `Spec 35: The unit store` as existing
- When `sync.py` runs
- Then it creates every other planned item, once each

**Scenario: an entry parses into its fields**
- Given the Open section's Q30 entry
- When `migrate_questions.py` parses it
- Then it yields number 30, title "Track open questions and upcoming work as GitHub issues,
  milestones, labels and a project?", its Raised and Raise when text, and its body

**Scenario: Raise when maps to one label**
- Given entries raised for "The next design session", "spec 35 done" and "any PR whose
  digests move"
- When they are migrated
- Then they are labelled `raise-when:next-design`, `raise-when:spec-35` (and milestone
  `Spec 35: The unit store`) and `raise-when:other`, and each body keeps the trigger's words

**Scenario: a migrated entry is not migrated twice**
- Given an issue titled `Q29. A side-by-side view of an outcome change, …` exists
- When the migration runs
- Then it creates no issue for Q29, and prints `Q29 -> #<that issue>`

**Scenario: Closed entries stay**
- Given the Closed section holds entries
- When the migration runs
- Then it creates no issue for any of them

**Scenario: the owner runs it (after the PR merges, not in a test)**
- Given step 1 is on `main`
- When the owner runs `github-sync` with `migrate` true
- Then every open entry is an issue, and the run's log holds the `Q<n> -> #<issue>` table

### Step 2

**Scenario: an entry under Open fails the check**
- Given `OPEN_QUESTIONS.md` with a `### Q31.` heading under `## Open`
- When `check_open_questions.py` runs
- Then it exits 1 naming the line

**Scenario: the pointer file passes**
- Given the step's `OPEN_QUESTIONS.md` (pointer, table, Closed)
- When `check_open_questions.py` runs
- Then it exits 0

## Test-first order

1. Step 1: `test_sync.py` red on the plan and on the existing-items cases (the API behind a
   small client the tests replace), then `sync.py`; `test_migrate_questions.py` red on
   parsing, mapping and skipping (a fixture copy of today's `OPEN_QUESTIONS.md`), then
   `migrate_questions.py`; then the workflow file.
2. Step 2: `test_check_open_questions.py` red, then the check and the hook; then the pointer
   file, `CLAUDE.md`, rule 5 and the two skills.

## Rubric answers

- **single-noun-phrase:** the question sync.
- **ocp-extension-point:** a new label is a line in `labels.py`; a new spec gets its
  milestone and step labels from its file.
- **lsp-contract-scope:** not applicable; no subtypes.
- **isp-fit:** the API client offers list and create for labels, milestones and issues,
  nothing more.
- **dip-direction:** the planning and parsing depend on the small client, never on HTTP.
- **general-over-special:** general. Every open question is an issue; every spec a
  milestone; every step heading a label.
- **order-independent:** not applicable; nothing under `sim/` changes.
- **bounded-work:** not applicable to the tick. The workflow makes a few list calls and one
  create per missing item.

## The owner's Project (not built by an agent)

After step 1 and the migration, the owner sets up a Project as given in this round's PR:
Status (Inbox, Next, In progress, In review, Done) and Priority fields, the built-in
workflows (auto-add from `SSidey/Breach`, item added → Inbox, closed or merged → Done,
auto-archive), and views by Status, by Milestone and of `label:question`.
