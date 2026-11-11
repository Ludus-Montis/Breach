---
spec_type: code
status: active
parent_spec: ../Breach — Reverse Tower Defense Design Spec.md
---

# Spec 45: Steps that say what they need, cascades that run them side by side, and signposted splits

## Purpose

The owner asked for steps to run in parallel where they can, with that worked out at the
design stage as each step's prerequisites, and for `/cascade` to route through them. Steps
have also often been too large for one session. Where that isn't caught at design, the
owner allows a step to be split during the build, provided the split is signposted.

**Settled by Decision 171**, which amends Decision 158. Each step names its `Needs` and
`Touches`. A step is ready when its needs are met and its files are free, and the
coordinator starts every ready step, with no cap. Steps are sized at design, and a split
during the build is recorded in the spec, the old issue, the PR and the cascade record.

## Status

Agreed in the design session of 2026-11-11. Three steps, each its own PR, built with
`/run-phase` as "spec 45 step N". The steps carry Decision 171's own lines: steps 1 and 2
can be built side by side, and step 3 needs step 2.

## Pre-mortem

- **Regret in a month:** steps run side by side, but their PRs keep conflicting. The
  owner's review time goes on rebases, and the parallel run saves nothing. A second regret
  is a flood of PRs at once, since there is no cap (the owner's choice).
- **Riskiest assumption:** that steps a spec calls independent change separate files.
- **Cheapest test:** already answered, before the Decision (Decision 171, Measured). Spec 44's
  steps 3 and 4 were called independent but shared 5 files, and spec 41's steps 3 and 4
  shared `parting.rs`. So the design declares files (`Touches`) as well as needs.

## Steps

### Step 1, `github-sync` reads `Needs`, `Touches` and splits

**Needs:** none
**Touches:** `ci/github/step_issues.py`, `ci/github/sync.py`, `ci/github/test_*.py`,
`ci/github/fixtures/specs/`, `ci/github/README.md`

- Given a step with `**Needs:** 1, 2a` and `**Touches:** a/b.py, c/`, when `github-sync`
  makes or updates its issue, then the issue body lists the needed steps as links to
  their issues, and the touched paths.
- Given a step with neither line, then the issue says
  `Needs: every step above it (none declared)`.
- Given step 3's heading is gone and steps `3a` and `3b` each carry
  `**Split at build:** from step 3, PR #n, <date>: <why>`, when `github-sync` closes step 3's
  issue, then its closing comment names the split and links `3a` and `3b`, not the generic
  "no heading any more" list.
- Given a `Needs` that names a step the spec doesn't have, then the sync reports it and
  fails, as it does today for a malformed marker.

### Step 2, the development kit

**Needs:** none
**Touches:** `AI_First_Development_Kit/principles/agent-sessions.md`,
`AI_First_Development_Kit/templates/cascade-issue.md`,
`AI_First_Development_Kit/templates/pr-description.md`,
`AI_First_Development_Kit/rubrics/spec-baseline.rubrics.md`, `CLAUDE.md`

- `agent-sessions.md` §3 gains these points. A design session gives every step `Needs` and
  `Touches` lines, and sizes it to one behaviour, one test file from red to green, and about
  15 files or 400 changed lines, not counting tests and docs. A larger step is split before
  the spec is pushed.
- Rule 8 replaces "one step at a time" with Decision 171's readiness rule:
  - needs met, files free, with the shared files exempt;
  - the base follows the needs;
  - no cap;
  - the coordinator is the only writer of the body;
  - a step reports any path it changed outside its `Touches`.
- `cascade-issue.md` replaces **Next** with **Ready**, **Running** (step, session, fallback
  check id) and **Done**. Standing rule 2 becomes "splitting allowed, signposted", as the
  default, and it says a step session writes only its log comment.
- `pr-description.md` gains an optional `Split:` line (`Split: step 3 into 3a (this PR) and
  3b; why`).
- The spec-baseline rubric gains a `step-shape` row: does every step have `Needs` and
  `Touches`, and does it fit the size guide?
- `CLAUDE.md`'s Design bullet says a spec's steps carry `Needs` and `Touches` and are sized
  to the guide, pointing to §3.

### Step 3, the `cascade` and `run-phase` skills

**Needs:** 2
**Touches:** `.claude/skills/cascade/SKILL.md`, `.claude/skills/run-phase/SKILL.md`

- **Start:**
  - The coordinator reads each step's `Needs` and `Touches` from the spec.
  - The standing-rules question no longer asks about splitting. It is on by default, and
    the owner can still say "ask first".
  - The coordinator works out the Ready list and starts every ready step.
- **Starting a step:** the prompt becomes `/cascade step #N <id>`. Each step gets its own
  fallback check and its own `Started step` comment.
- **Waking:**
  - On each report, the coordinator moves the step to Done and adds its Chain row.
  - It folds in the step's gotchas and any extra paths the step touched.
  - It works out Ready again and starts whatever has become ready.
  - The cascade is done when nothing is running and nothing is left before the stop point.
  - A blocked step stops only the steps that need it. The others carry on, and the owner is
    told.
- **Step:**
  - The step session doesn't edit the record's body. Its log comment carries the PR, its
    gotchas and any paths it changed outside its `Touches`, and its report says the same.
  - It branches from the base the coordinator named in its Running row.
- **`run-phase` gains a "Splitting a step" section:**
  - when to split: the size guide, judged at the plan, before any code;
  - how: the spec's headings, the `Split at build` line, `Needs` and `Touches` for each
    part, the `Split:` PR line, and a comment on the old step's issue;
  - outside a cascade: name the next part's command as the next step.
- **Chain mode** (no coordinator) stays one step at a time.

## Test-first order

1. Step 1: red on the parsing of `Needs`, `Touches` and `Split at build` from fixture specs.
   Then red on the issue body and the split's closing comment (Python, as
   `test_built_steps.py` does, run alone). Then the sync.
2. Step 2: no code. It is checked by `check-docs.sh` and the rubric row, read against this
   spec.
3. Step 3: no code. It is checked by reading the skills against Decision 171, and by the
   first cascade run over a spec that carries the lines (this spec's steps 1 and 2 are the
   first such pair).

## Rubric answers

- **single-noun-phrase:** the step lines' sync; the kit's text; the skills.
- **ocp-extension-point:** a new step line is a regex and a line in the issue body.
- **lsp-contract-scope:** not applicable; no subtypes.
- **isp-fit:** `github-sync` reads only headings and their lines; the coordinator reads only
  the issues and the record.
- **dip-direction:** the cascade depends on the spec's lines, never on an order of headings
  it guesses.
- **general-over-special:** general: one readiness rule, one size guide and one split
  procedure for every step (Decision 171).
- **pre-mortem:** the riskiest assumption was tested before the Decision. Declaring files
  is the response to it.
- **order-independent:** not applicable; no simulation outcome changes.
- **bounded-work:** not applicable to the tick.
