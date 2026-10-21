# Quickstart — Bootstrapping a New Project From This Kit

Read this first if you're an agent being pointed at this kit for a brand-new (or
currently bare) repository. Everything below is a re-derivation of the principles
elsewhere in this kit, turned into an ordered, ask-then-do checklist — so that sequence
doesn't need to be reasoned out from scratch every time. If some of this is already done
in the target repo, skip those steps; don't redo settled work.

Two habits worth internalizing before starting, both learned the hard way on a real
project that used this kit:

- **Propose, then confirm — don't ask open-ended.** For anything with multiple
  reasonable answers (test framework, CI host, merge policy), state a recommended
  default and ask for confirmation or override. "What would you like?" with no starting
  point wastes a round-trip on questions you can usually answer yourself.
- **Verify empirically; don't trust a tool's docs at face value.** Every piece of tooling
  below should be run against a trivial case before you consider it wired correctly.
  Package READMEs and official docs go stale or skip edge cases — a CLI flag that turns
  out not to exist, a plugin needing one extra initialization pass before its classes
  resolve, a permissions default that silently blocks a later step. What you observe
  from actually running the command is the source of truth, not what the doc claims.

## Step 0 — Scope the project (ask, batched, up front)

Before touching anything, get clear, confirmed answers to:

1. **What is this project?** A one-line description — used later for a repo description
   and README pitch.
2. **What stack?** Language/framework/engine. This names the `ci/<stack>/` layer you're
   about to build (Step 3) and it's the one answer everything else depends on.
3. **Is there a remote already**, or does one need creating? Which host? If creating,
   what should the repo be named, and what visibility?
4. **Merge policy** — does the agent open pull/merge requests and stop for human review,
   or auto-merge once the run-baseline rubric passes? `principles/agent-workflow.md`'s
   default assumption is the former; confirm this project isn't overriding it.
5. **Existing code, or greenfield?** If there's existing code, the stack-specific layer
   needs to run against it as a first health-check, not just against an empty scaffold —
   expect (and report) whatever it finds, rather than tuning the checks until they pass.

Record the answers as this project's first Decision entries
(`principles/decision-ledger.md`) once you act on them — don't leave them only in chat
history.

## Step 1 — Source control

- `git init` if not already a repository; default branch `main`.
- Add a `.gitignore` suited to the declared stack — use the ecosystem's known-good
  template rather than authoring one from scratch.
- First commit: project scaffold, plus this kit dropped in (commonly under `dev_kit/`,
  or wherever the project prefers) keeping its `principles/`/`rubrics/`/`templates/`/
  `config/` layout intact.

## Step 2 — Remote

- If a remote was named in Step 0: add it as `origin`, push `main`.
- If not: ask whether to create one now (most hosts have a CLI, e.g. `gh repo create`)
  or defer — don't silently skip this, since CI in Step 3 needs a remote to run against.
- If the host supports it and this project's plans need it (e.g. CI opening its own
  PRs, per the release-automation pattern `principles/progress-tracking.md` describes as
  an optional extension), check repo-level settings that step will require — e.g.
  Actions/CI default permissions. Confirm before changing any repo-wide setting; it
  affects more than the one change prompting it.

## Step 3 — Build `ci/<stack>/`

The most time-consuming step, and the one most tempting to under-invest in. Do not
default a rubric row to "heuristic" or "procedural" until you've actually checked
whether the declared stack has real tooling for it — ecosystem maturity varies a lot,
and the ceiling for one stack is not evidence of the ceiling for another:

- **Test framework** — the ecosystem's standard (ask if there's a house preference;
  otherwise propose the most common one and confirm).
- **Linter + formatter.**
- **Coverage tool**, wired to actually report a number. Only fall back to "checked
  manually" if you've confirmed no coverage tool integrates with the chosen test
  framework — and say so plainly in the layer's README, don't leave it unexplained.
- **Contract-test support** for `solid-mechanical.md` criterion L — most test
  frameworks can run one shared suite against every implementation of a base
  type/interface directly; this rarely needs a bespoke script.
- **OCP / ISP / DIP checks** — before writing a heuristic, check whether the ecosystem
  already has a real static-analysis or architecture-testing tool (dependency-direction
  linters, architecture-fitness-function libraries, call-graph tooling). Mature
  ecosystems often do, and a real tool beats a heuristic script every time. Only build a
  bespoke heuristic — and document its limits, per `templates/ci-tool-mapping.md` — where
  nothing existing covers it.
- **`branch-name-conforms` / `commit-message-conforms`** — almost always mechanizable
  directly (a hook reading the branch name or commit message), regardless of stack.
  Treat "procedural only" as the fallback for when no tool exists, not the default.
- **CI** — a fast local subset via pre-commit (or the ecosystem's closest equivalent),
  and a full authoritative run in CI, per `rubrics/run-baseline.rubrics.md`'s "Where
  this runs."
- Document the whole layer as `ci/<stack>/README.md` using `templates/ci-tool-mapping.md`'s
  three-section shape, and mean it: a row only belongs in "Procedural, not scripted"
  after an actual search for a mechanical option came up empty, not as a default landing
  spot.
- **Before moving on:** run every check just wired against the current (empty or
  trivial) scaffold and confirm it behaves as expected — including deliberately
  breaking one thing per check to confirm it actually catches it. A check that has never
  been seen to fail is unverified, not passing.

## Step 4 — Codify the agent workflow

Per `principles/agent-workflow.md`: if the agent tooling in use supports a reusable,
invocable procedure (a custom command, skill, or macro), create one now that encodes the
branch → implement → gate → open-proposal → stop sequence for this project specifically,
using the branch-prefix and merge-policy answers from Step 0.

Then, per `principles/agent-sessions.md`:

- **Write the entry file** the agent tooling loads at the start of every session (for
  example `CLAUDE.md` or `AGENTS.md`). Keep it to about a page: the non-negotiables, one
  line each, the design and implementation roles, and where specs, Decisions, the
  open-questions register and the procedures live. It points to rules; it doesn't copy them.
- **Create the open-questions register** (one file, empty to start) and say in the entry
  file where it is.
- **If the Decision ledger lives in a long spec,** add the generated index and its gate
  check once the ledger outgrows a quick read. A new project can wait until it does.
- **Set up the documentation** (`principles/documentation-phases.md`): `README.md`,
  `CHANGELOG.md` and `docs/index.md` from `templates/README.template.md`,
  `templates/CHANGELOG.template.md` and `templates/docs/index.template.md`, and
  `templates/ci/docs-gate.yml` as a CI workflow. Add the documentation phases to the
  procedure above, between the gate and opening the proposal.

## Step 5 — Progress and qualitative logging

- Set up the Gate 1 mechanical progress log and its generating script, per
  `principles/progress-tracking.md`'s recommended mechanism: compute the metrics, diff
  against the previous logged row, append, fail on regression.
- Set up a durable file for Gate 2 human qualitative review. Ask what "the quality the
  numbers don't capture" actually means for *this* project before choosing its fields —
  a consumer app's fields (does it feel good to use, bugs hit) don't fit a backend
  service (operability, latency regression, API ergonomics). Don't default to copying
  another project's field set.
- Ask whether this project has discrete versioned releases that would benefit from the
  optional per-release extension in `principles/progress-tracking.md`. If yes, its
  trigger and entry shape are themselves a Decision to record, not an assumption to bake
  in silently.

## Step 6 — Confirm green, then stop

- `pre-commit run --all-files` (or equivalent) clean.
- Push the initial branch, confirm CI is actually green on a real proposal — not
  "should work," observed passing.
- Report back: what got set up, what was asked-and-decided (pointing at the Decisions
  just recorded), and what's left procedural or manual and why (pointing at
  `ci/<stack>/README.md`'s own "Heuristic limits" / "Procedural, not scripted"
  sections). Then stop for review, per `principles/agent-workflow.md` — this bootstrap
  is itself a change, and it lands the same way every other change does.
