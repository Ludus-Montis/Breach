---
doc: principles/agent-workflow
status: active
applies_to: every change to source, specifications, or this kit itself
---

# Agent Workflow — Branching, Review, and What an Agent Never Does Alone

## Why this exists

A rubric that describes what "done" looks like doesn't, by itself, stop a change from
landing directly on the trunk branch. `rubrics/run-baseline.rubrics.md` has always had a
`branch-name-conforms` row, but a row that's merely *checked* is weaker than a workflow
where skipping it isn't an available path. This document is the missing standing rule:
not "branch names should conform," but "every change goes through a branch and a
reviewable proposal, always, with no exception for a change that feels small." The
distinction matters most for an AI agent working autonomously across many turns, where
"just this once, straight to trunk" quietly becomes the default unless something
concrete prevents it.

## The rule

**Nothing lands on the trunk branch directly.** Every change — regardless of size,
regardless of who or what is making it — goes on its own branch and lands through a
reviewable proposal (a pull/merge request) that a human approves. This applies equally to
a human contributor and an AI agent; the kit states it here because an agent has no
instinct to fall back on when a task description doesn't mention branching at all.

**Branch naming** follows [Conventional Branch](https://conventionalbranch.org/):
purpose-driven prefix — `feature/`, `fix/` (or `bugfix/`), `hotfix/`, `release/`,
`chore/` — followed by a lowercase-kebab-case description. Pick the prefix by what the
diff actually is, using the same type-vs-diff judgement as
`templates/commit-message.md`: new capability visible to a consumer → `feature/`;
correcting broken behaviour → `fix/`; tooling/process/CI with no product-facing
behaviour change → `chore/`. This is what gives the `branch-name-conforms` rubric row
something concrete to check, mechanically or procedurally (see below).

## Codify this as a procedure, not a memory

Restating "remember to branch" at the top of every task is fragile — it depends on
whichever agent picks up the work happening to recall it. Wherever the project's agentic
tooling supports a reusable, invocable procedure (a custom command, skill, macro, or
equivalent), the full per-change sequence below should be encoded there once, so it
runs identically every time rather than being re-derived per session:

1. Confirm the trunk branch is clean and up to date before branching from it.
2. Branch, named per the rule above.
3. Implement via `principles/tdd-bdd-workflow.md`.
4. Run the `rubrics/run-baseline.rubrics.md` gate and record the
   `principles/progress-tracking.md` Gate 1 entry.
5. Run the documentation phases (`principles/documentation-phases.md`): the impact
   assessment, then README, changelog and `docs/`, each updated or marked N/A with a
   reason.
6. Open the reviewable proposal, with its Documentation block, and **stop** — do not
   merge it (see below).

A stack's `ci/<stack>/` layer is where the *mechanical* half of this lives (linters, test
runners, CI config); this procedure is the *sequencing* half, and belongs in whatever
tool actually drives the agent's turns. Not every rubric row can be a script — see
`rubrics/run-baseline.rubrics.md`'s own acknowledgment that some rows stay
procedural rather than mechanical until better tooling exists; a codified procedure is
how a procedural row still gets enforced reliably instead of becoming an honor system.

## What an agent never does alone

The following are human-only actions, restated here as one coherent list rather than
scattered across the documents that first raise each one — the underlying reason is the
same conflict of interest each time: whoever made the change judging whether the change
was good is not a credible judge of that.

- **Merging the proposal.** An agent opens it and stops; a human reviews and merges (or
  requests changes). A clean mechanical gate is not authorization to merge — that's
  exactly what `principles/progress-tracking.md` Gate 2 exists to keep separate.
- **Superseding a Decision.** `principles/decision-ledger.md` already requires
  `authorised_by: <human>` on every superseding entry — an agent can *propose* the new
  Decision's text, but cannot authorise it.
- **Recording the Gate 2 verdict.** The qualitative judgement belongs to a human, kept
  separate from the mechanical log entry the agent generates — see
  `principles/progress-tracking.md`.

## What this document deliberately leaves open

Which specific hosting platform, review mechanism, or agentic tooling a project uses is
out of scope here — this kit stays independent of that choice, per its own README. What
isn't open is the rule itself: branch, propose, human merges. A project may layer
additional requirements on top (required reviewers, status checks, merge queues) but may
not weaken this baseline without a recorded Decision explaining why.
