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
- Check the open questions (Decision 148): `list_issues` with `labels: [question]`,
  `state: OPEN` and `fields: [number, title, labels]` (`gh` isn't logged in on a cloud
  session; use the GitHub MCP tools). Read an issue's body (`issue_read`) only when its
  `raise-when:` label or title may match this item, and raise the ones that match with the
  user before planning.
- What the spec and its Decisions settle, build as written: don't re-plan or re-ask it.
  Plan mode is for a new spec, or for a gap the spec leaves open
  (`AI_First_Development_Kit/principles/agent-sessions.md`, rule 3).
- Confirm the working tree is clean (`git status`) and `main` is up to date
  (`git checkout main && git pull --ff-only`) before branching. If there are
  uncommitted changes that aren't yours to discard, stop and ask.
- Confirm the branch this item depends on (if any) is already merged to `main` before
  branching from it — don't stack on an unmerged PR without asking first.
- **In a cascade** (you were started with `/cascade step #N <id>`,
  `.claude/skills/cascade/`): the cascade's issue is your standing instructions, and the
  base in your step's **Running** row is the branch you stack on (the owner authorised
  that when the cascade started). Read it whole, once. Don't rebuild instructions from
  older PR comments. Don't edit the record's body. Finish with the cascade skill's "Step"
  section (log comment, report), then stop.
- **Size the step before any code** ("Splitting a step", below). If it is too large, split
  it first.

## Splitting a step

A step is sized at design (`AI_First_Development_Kit/principles/agent-sessions.md` §3,
Decision 171): one behaviour, one test file from red to green, and a PR that can be
reviewed in one sitting, about 15 files or 400 changed lines, not counting tests and docs.

- **When:** at the plan, before any code. Once you have read the step and know which files
  it changes, judge it against the guide. A step that is over it, or that holds two
  behaviours, is split now, without asking (Decision 171), unless a cascade's standing
  rule 2 says ask first. Don't split halfway through the build to shed work you have
  already started; finish the part you are in.
- **How:**
  - in the spec, replace `### Step <id>` with `### Step <id>a`, `### Step <id>b`, … each
    keeping its share of the scenarios, with a line
    `**Split at build:** from step <id>, PR #<n>, <YYYY-MM-DD>: <why>`, and its own
    `**Needs:**` and `**Touches:**`. The later parts usually need the first. Steps that
    needed `<id>` now need the part that carries what they used. The part this PR builds
    goes under the spec's `## Built` section (add one at the end if there is none); the
    rest stay under `## Steps`. That spec edit goes in this PR;
  - the PR body carries `Split: step <id> into <id>a (this PR) and <id>b; <why>`
    (`templates/pr-description.md`) and **no** `Closes #<old step's issue>`. Once it merges,
    `github-sync` closes the old step's issue naming the parts, makes the parts' issues,
    and closes the built part's against this PR (`ci/github/step_issues.py`);
  - comment on the old step's issue now, so it is signposted before the merge:
    `Split at build into <id>a (PR #<n>) and <id>b: <why>`;
  - in a cascade, put the split in your log comment and report, and the coordinator adds
    the parts to the record.
- **Outside a cascade:** build the first part, and name the next part's command as the next
  step (`/run-phase spec NN step <id>b`).

## 1. Branch — Conventional Branch, purpose-driven prefix only

A cloud session is often given a branch to work on (`claude/<words>`, `ccr-<id>`). Don't
use it: it fails `check_branch_name.sh`, and the owner's naming wins over the session's
default. Create the branch below from `main` and push that; never skip the
`branch-name-conforms` hook to get past it.

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

**Adding a spec or a Decision?** Push the branch and open the PR, as a draft, as soon as you
pick the numbers, before writing the body: the first PR to take a number keeps it (Decision
181). If CI's `numbers` check fails, take the numbers its message names.

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
   `native/` or the store's columns, or its gdUnit test needs the core (every battle and
   unit store runs on it since spec 39 step 6a). Otherwise don't: cargo tests need no
   build, and CI's `sim-checks` runs the suite and the digests on every PR.
   A cloud session without Godot (`GODOT_BIN` unset) fetches it when a step first needs it:
   `bash ci/godot/scripts/get_godot.sh`, then export the `GODOT_BIN` it prints. It installs
   the version CI pins, checks it against Godot's published sums and imports the project
   (so the import above is done). If its download is blocked, say so and name the host the
   environment's network policy must allow; don't skip red then green.
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

**Running long commands in a session.** A shell call times out after two minutes by
default, and a first `cargo build` or `cargo test` takes several:
- pass a longer timeout (up to ten minutes);
- or run the command in the background and wait for its completion notice;
- never chain `sleep`s to wait;
- don't edit a Rust source or run `cargo fmt` while a build is running: the crate
  compiles from whatever is on disk.

**Choosing fixtures from real runs.** To find the ticks, seeds or cases where a behaviour
shows (which phases change state in which tick, say), write a throwaway probe: an
`#[ignore]`d cargo test, or a script under the scratchpad. Run it once, read only its
summary lines, and delete it before committing. Record what it found in the spec, so
the next agent doesn't run it again.

## 4. Commit as you go

**Never commit a `// RED-STUB`:** finish the red-then-green pair first. A stop hook that
asks for a commit while a stub is in the tree waits until the stub is gone.

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
   CI is the test runner). Speed-ups are compared through the `bench` workflow. A local
   pair on the owner's PC shows a change's direction only, never its size (spec 48 step 4).
2. Run the static checks directly (they also run in `pre-commit`/CI, but run them here
   first to catch anything before pushing): `check_function_length.py`,
   `check_generic_naming.py`, `check_isp.py`, `check_dependency_direction.py`,
   `check_helper_promotion.py`, `check_ocp_shotgun_surgery.py origin/main`
   (on a PR stacked on an unmerged branch, run it against that base branch instead and say
   so in the PR: against `main` it counts every unmerged step below yours),
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

## 5b. Documentation phases (before opening a PR)

Per `AI_First_Development_Kit/principles/documentation-phases.md`: the change is not done
until its documentation is. Read only the sections the change touches.

1. **Impact assessment** (`templates/doc-impact-assessment.md`, kept in your head or the PR,
   not committed). In Breach: a change a player or the map designer would notice (units,
   combat, content, the designer) is user-visible; a change under `content/definitions/` is
   a data-format change (reference docs, alongside the designer update `CLAUDE.md` already
   requires); a new Decision is an architectural decision.
2. **README** (`README.md`): only Install, Usage and Background can go stale. Update them
   when setup, a command or the project's layout changes; otherwise "verified, no change".
3. **Changelog** (`CHANGELOG.md`): one line per notable change under `## [Unreleased]`,
   written for the player or designer ("Squads now X", not "Moved Y into Z"). A refactor,
   a speed-up nobody sees, tests or CI: `Changelog: N/A (<reason>)` in the PR body, which
   the `docs-gate` check reads. **Write exactly that, on one line, label first.** Rewording
   it ("No changelog", "Changelog not needed") fails the gate; this has cost a fix-up on
   several PRs.
4. **Docs** (`docs/index.md`, Diátaxis): add or fix the page in its quadrant and link it
   from the index. A tool's own README stays where it is and is linked. Decisions are this
   project's ADRs: never write a separate ADR, and never record a Decision yourself without
   the user's authorisation (step 3 above).
5. Run `bash AI_First_Development_Kit/scripts/check-docs.sh all origin/main` (seconds) and
   fix any failure, with `DOCS_SKIP_CHANGELOG=1` when the PR body will say
   `Changelog: N/A`.

## 6. Open the PR — do not merge it

Push the branch and open a PR with `create_pull_request` (the GitHub MCP tools, not
`gh pr create`), body from
`AI_First_Development_Kit/templates/pr-description.md`: summary, link to the spec/plan
item, the rubric checklist from step 5, the Documentation block from step 5b, and for the
two Gate sections:
- **Progress log entry** — link/quote the row `gate1_progress_log.py` just appended to
  `PROGRESS_LOG.md`, so the reviewer doesn't have to open another file.
- **Human qualitative review (Gate 2)** — link to `QUALITATIVE_REVIEW_LOG.md` and note
  there's a pending entry for a human to fill in. Never write the play-feel judgement
  yourself — `progress-tracking.md`: "an agent judging whether its own change was an
  improvement has an obvious conflict of interest."
- **Closes** — `Closes #N` for the step's issue (below) and for each `question` issue this
  PR settles, so the owner's merge closes them. Never close a question yourself; on one this PR may settle without being sure,
  comment "Possibly settled by …" instead.

Then name the PR's spec and step (Decision 148): one `issue_write` with `method: update` on
the PR's number (a PR is an issue) setting `milestone` to the spec's (`Spec NN: <title>`;
its number from the milestone's link or an issue in it) and `labels` to `step:NN-<id>`, one
kind (`implementation`, `design` or `chore`), the areas it touches (`sim`, `native`,
`designer`, `ci`, `tooling`) and any flag (`outcomes-changed`, `needs-design`). If a label or
milestone is missing (a new spec or step), start the `github-sync` workflow
(`actions_run_trigger`, `workflow_id: github-sync.yml`, `ref: main`), set what exists, and
tell the user what is missing rather than wait for the run.

Then close the step's issue from the PR (spec 40). `github-sync` makes one issue per
`### Step <id>` heading, under the spec's parent issue `Spec NN: <title>`. Find it with
`list_issues`, `labels: [step:NN-<id>]`, `state: OPEN`, `fields: [number, title]`, and add
`Closes #<it>` to the PR body (`update_pull_request`). A PR that builds two steps closes
both. A PR that splits its step closes no issue for it ("Splitting a step").
If none is found (the sync hasn't run for this spec), say so in the PR body and start
`github-sync` as above, rather than wait for it.

Then turn on Auto-fix for the PR, which the owner has standing-authorised for every PR
`/run-phase` opens. Read the session's PR with the app's `get_status` (the `ccd_pr`
tools), bind it with `bind_pr` if it isn't reported, then `set_monitor` with
`auto_fix: true` and the PR's url. The app then wakes this session on a CI failure, a
merge conflict or a review comment; answer each the way the next paragraphs say (read the
failing test's part of the log, run that one file, push a fix to the same branch). That
is the app watching, not this session polling, so it keeps the rule below. Where the
`ccd_pr` tools aren't there (some cloud sessions), skip this and say in the hand-off that
Auto-fix is off, so the owner can turn it on from the PR pane.

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
