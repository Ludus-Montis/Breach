---
name: run-phase
description: "Implement one phase or numbered item of an approved Breach execution plan end-to-end, following AI_First_Development_Kit's branch/commit conventions and TDD/rubric workflow. Use when asked to implement, start, or continue a plan phase or item (e.g. \"do Phase 3 item 2\", \"implement the vertical slice\", \"continue where we left off\"), or when invoked as /run-phase. Do not use for one-off fixes unrelated to a plan phase, or for exploratory/research-only requests."
---

# Implementing a plan phase or item

This codifies the standing process for turning one phase/item of an approved execution
plan (the vertical-slice plan under `C:\Users\Simeon\.claude\plans\`, or the map/spec
scope in `specs/00-scope-and-map.md` and its sibling `specs/*.md` files) into merged,
rubric-compliant work — so every phase follows the same discipline instead of
re-deriving it each session. Adapted from
[SSidey/Sweepminer](https://github.com/SSidey/Sweepminer)'s own `run-phase` skill (a
sibling project on the same `AI_First_Development_Kit`), which had already codified
this per `AI_First_Development_Kit/principles/agent-workflow.md`'s own recommendation.

**Standing rule, not phase-specific: nothing lands on `main` directly.** Every change,
regardless of size, goes on its own branch and through a PR that a human merges.

## 0. Identify the phase/item

- If an argument names the phase/item, use it. Otherwise ask which one to implement —
  don't guess against a stale plan.
- Read the item's scope in full before starting: the plan file, and the relevant
  `specs/*.md` file(s) it implements, including that spec's Given/When/Then scenarios,
  test-first order, and any Decisions in `Breach — Reverse Tower Defense Design
  Spec.md`'s `## Decisions` section that bear on this item. Find them in
  `DECISIONS_INDEX.md` and read each by its line; don't read the whole spec.
- Check `OPEN_QUESTIONS.md` for entries whose **Raise when** matches this item, and
  raise them with the user before planning.
- What the spec and its Decisions settle, build as written: don't re-plan or re-ask it.
  Plan mode is for a new spec, or for a gap the spec leaves open
  (`AI_First_Development_Kit/principles/agent-sessions.md`, rule 3).
- Confirm the working tree is clean (`git status`) and `main` is up to date
  (`git checkout main && git pull --ff-only`) before branching. If there are
  uncommitted changes that aren't yours to discard, stop and ask.
- Confirm the branch this item depends on (if any) is already merged to `main` before
  branching from it — don't stack on an unmerged PR without asking first.

## 1. Branch — Conventional Branch, purpose-driven prefix only

`feature/`, `fix/` (bugfix), `hotfix/`, `release/`, or `chore/`, followed by a
lowercase-kebab-case description — see
[conventionalbranch.org](https://conventionalbranch.org/). Pick the prefix by what the
diff actually is, same judgement as `AI_First_Development_Kit/templates/commit-message.md`:
- New gameplay/simulation capability (most plan items) → `feature/`
- Correcting broken behaviour → `fix/`
- Tooling/CI/process only, no gameplay behaviour → `chore/`

```
git checkout main
git pull --ff-only
git checkout -b feature/<short-kebab-slug-for-this-item>
```

## 2. Spec first, if this item doesn't already have one

If the plan item doesn't map to an existing `specs/*.md` file, write one before any
code: `spec_type` frontmatter, Given/When/Then scenarios, a test-first order, and the
`spec-baseline.rubrics.md` qualitative answers (`single-noun-phrase`,
`ocp-extension-point`, `lsp-contract-scope`, `isp-fit`, `dip-direction`) inline — see
any `specs/0N-*.md` file for the shape to follow.

## 3. Implement via the TDD workflow

Follow `AI_First_Development_Kit/principles/tdd-bdd-workflow.md` for every code-bearing
step, in the order the spec's own "Test-first order" section lays out:

1. Translate each Given/When/Then scenario into a gdUnit4 test under `tests/`,
   mirroring the source path being tested (e.g. `sim/foo.gd` → `tests/sim/test_foo.gd`).
2. Run it and confirm **red** — it must fail for the right reason (missing behaviour,
   not a typo or setup error). Run only that test file in an agent session:
   `"$GODOT_BIN" --headless --path . -s res://addons/gdUnit4/bin/GdUnitCmdTool.gd -a
   res://tests/<path>.gd -c --ignoreHeadlessMode`. Heavy runs never go in the session
   (see `CLAUDE.md`).
   **Before the first run in a session**, import the project once
   (`"$GODOT_BIN" --headless --path . --import`) so gdUnit4's class names resolve. Build
   the Rust core (`bash native/build.sh`, a few minutes) only when the step changes
   `native/` or the store's columns, or its test compares both engines and you need the
   Rust half before CI. Otherwise don't: tests that compare the engines skip their Rust
   half when the core isn't built, and CI's `sim-checks` runs both engines on every PR.
   A cloud session gets Godot from its environment's setup script (`GODOT_BIN` set
   there). If it is missing, say so, and install it as `.github/actions/godot/action.yml`
   does rather than skipping red then green.
3. Implement the minimum to go **green**. Do not implement behaviour with no test.
4. Refactor with the suite as a safety net, checking against
   `AI_First_Development_Kit/principles/ai-first-organisation.md` and
   `solid-mechanical.md` as you go (one concern per file, size budgets, no cross-cutting
   helpers below the promotion threshold).
5. Where the item introduces a second implementation of an existing base
   type/contract, write the shared contract-test suite once and run it unmodified
   against every implementation (`solid-mechanical.md` criterion L) — do not write a
   bespoke test per implementation.

**If the plan or spec is ambiguous, or hits a decision point it doesn't already
resolve**, stop and ask the user (don't assume). Once resolved, append a new Decision
entry to `Breach — Reverse Tower Defense Design Spec.md`'s `## Decisions` section using
`AI_First_Development_Kit/templates/decision-entry.md` — never edit a prior Decision's
text, only append (mark superseded ones per `decision-ledger.md` if applicable).
Every new Decision ends with a **Pillar check** line against Decision 58's design
pillars: "holds", or which pillar it bends and why, and with the **Rules over cases** and
**Order** lines from the `design-check` skill (Decisions 96 and 97): run that skill on
every design decision and every change to simulation outcomes. A Decision that bends a pillar is a
pivot: say so to the user and get their explicit authorisation before recording it.

## 4. Commit as you go

Run the fast local gate before each commit: `pre-commit run --all-files` (runs
automatically on `git commit` once `pre-commit install` has been run once per clone).
Write commit messages per `AI_First_Development_Kit/templates/commit-message.md` —
Conventional Commits, with the type matching the actual diff, not the intent (a
`refactor` with behaviour-asserting test changes is actually a `feat`/`fix`; split the
commit if the diff genuinely mixes types).

## 5. End-of-item gate (before opening a PR)

This is `AI_First_Development_Kit/principles/progress-tracking.md` Gate 1 — mechanical,
hard, and not skippable:

1. Run the full suite: `bash ci/godot/scripts/run_tests.sh`. Do this only on the
   user's PC. In an agent session, run only the test file you wrote red-then-green; the
   other test files you touched, the full suite and the digests run in CI (`CLAUDE.md`:
   CI is the test runner). Speed-ups are compared through the `bench` workflow.
2. Run the static checks directly (they also run in `pre-commit`/CI, but run them here
   first to catch anything before pushing): `check_function_length.py`,
   `check_generic_naming.py`, `check_isp.py`, `check_dependency_direction.py`,
   `check_helper_promotion.py`, `check_id_order.py`, `check_ocp_shotgun_surgery.py origin/main`,
   `check_context_locality.py origin/main` (informational — read its output, it won't
   fail the gate, but a large touched-file count that *isn't* the schema-plus-consumer
   pattern Decision 12 covers is worth a second look before opening the PR), plus
   `gdlint`/`gdformat --check` (all under `ci/godot/scripts/`, or `gdlint`/`gdformat`
   directly).
3. Run `python3 ci/godot/scripts/gate1_progress_log.py` — computes the current
   metrics, diffs against the previous row in `PROGRESS_LOG.md`, and appends the new
   row itself. **A non-zero exit means a regression was detected; the item is not done
   until that's fixed** — loop back into step 2 rather than opening a PR on a known
   regression. It reads the last test run's report, so run it only right after a
   full-suite run. In an agent session, skip it: after a partial run it records a false
   drop in the test count.
4. Read each static check's heuristic-limit note in `ci/godot/README.md` ("Heuristic
   limits") before trusting a green run blindly — a pass means the specific heuristic
   found nothing, not that the underlying SOLID property is proven.
5. Coverage isn't automated (see `ci/godot/README.md`'s "Procedural, not scripted") —
   confirm every Given/When/Then scenario in the item's spec has a corresponding test
   instead. **Don't add a coverage line to the PR description for this** — Decision 10
   in the design spec settled coverage as permanently procedural; it's not an
   open item to re-flag on every PR. This check is for your own confirmation before
   opening the PR, not something the reviewer needs restated each time.

## 6. Open the PR — do not merge it

Push the branch and open a PR with `gh pr create`, body from
`AI_First_Development_Kit/templates/pr-description.md`: summary, link to the spec/plan
item, the rubric checklist from step 5, and for the two Gate sections:
- **Progress log entry** — link/quote the row `gate1_progress_log.py` just appended to
  `PROGRESS_LOG.md`, so the reviewer doesn't have to open another file.
- **Human qualitative review (Gate 2)** — link to `QUALITATIVE_REVIEW_LOG.md` and note
  there's a pending entry for a human to fill in. Never write the play-feel judgement
  yourself — `progress-tracking.md`: "an agent judging whether its own change was an
  improvement has an obvious conflict of interest."

**Do not poll or watch CI to completion after opening the PR.** Notify the user that
the PR is open (a link and a short summary of what landed), **end with the exact next
command** (the next `/run-phase spec N step M`, or "next is a design session on
<subject>" when the next work has no spec yet), and stop there — the user
reviews, watches CI if they want to, and decides when to kick off the next phase/item.
This was said directly after an agent spent several tool calls polling `gh run list`/
`gh api .../actions/runs` for a run that hadn't triggered yet, while the user had
already merged the PR in the meantime: notifying promptly and letting the human drive
the pace is the actual preference, not a one-off.

This doesn't mean CI reliability stopped mattering — it's the opposite: this project
has hit multiple real environment-specific failures that only showed up on an actual
CI run, never locally (a display-server requirement gdUnit4's own runner has that the
CI runner doesn't satisfy; a GitHub-generated synthetic merge commit tripping the
commit-message check; a `gdtoolkit` grammar-cache race condition — all catalogued in
`ci/godot/README.md`'s "Known environment gotchas"). If the user reports CI failed on
a PR, or asks you to check, investigate then and push a follow-up commit on the same
branch — don't leave a red PR open assuming it's someone else's problem. The change
here is *when* CI gets checked (on request, not as a blocking step every single time),
not whether it matters.

Per this project's decision, **stop here** either way. Do not merge, even if CI is
green — the user reviews and merges, and decides when the next phase/item starts.
