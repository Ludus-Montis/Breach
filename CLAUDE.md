# Breach

A reverse tower defence in Godot 4.7 (GDScript, with a Rust core under `native/`). The
player is the overlord sending waves against a kingdom's forts. The deterministic
simulation is in `sim/`, the scenes in `presentation/`, content and schema in `content/`,
and the map designer (a local web app) in `tools/designer/`.

The process is `AI_First_Development_Kit/`. This file points to it and doesn't repeat it.
How sessions work together is `principles/agent-sessions.md`.

## Non-negotiables

- **Every change goes on a branch and through a PR, and the user merges.** Never merge,
  and don't poll CI after opening a PR. Use `/run-phase` (`.claude/skills/run-phase/`).
  The branch is a Conventional Branch (`feature/`, `fix/` or `chore/`, then kebab-case;
  `/run-phase` step 1); a design session's `docs:` PR goes on `chore/`. A cloud session's
  assigned `claude/…` or `ccr-…` branch is not one: create your own from `main`, push that.
- **Tests come first:** red, then green (`principles/tdd-bdd-workflow.md`). Run the suite
  with `bash ci/godot/scripts/run_tests.sh`, which needs `GODOT_BIN` set.
- **CI is the test runner. No long-running tests in a session, ever.** In a session, run
  only the test you are writing red, then green (`-a res://tests/<file>.gd`, or for the
  Rust crate `cargo test <name>` in `native/rust/`), plus lint and the static checks.
  Everything else runs in CI: the other test files a change touches, the full suite, the
  digests (`formation_digest.gd`), the trials, `scaling_check`, `tick_phases` and the other
  benches. CI's `sim-checks` runs the suite and digests on every PR, and its `rust` job the
  crate's format, clippy and cargo tests on every PR that touches `native/`. When CI goes
  red, read the failing test's part of the log and run that one file. For a comparison,
  start `bench` (`.github/workflows/bench.yml`) and read its `SUMMARY`. To judge a change of
  a few percent, give it `rounds` of 5 or more: one run each swings by more than that
  whenever the code differs.
- **Usage is a budget** (`principles/agent-sessions.md`, rule 4). One step per session.
  Keep output small: show only failures from a run, only the failing part of a log, and
  never echo whole files, diffs or listings to look at them.
- **Documentation is part of done** (`principles/documentation-phases.md`, Decision 149).
  Before a PR: the impact assessment, then `README.md`, `CHANGELOG.md` and `docs/`, each
  updated or marked N/A with a reason in the PR's Documentation block. CI's `docs-gate`
  runs `AI_First_Development_Kit/scripts/check-docs.sh`.
- **Every finished step names the next command.** The last message of a step gives the
  exact next command (`/run-phase spec N step M`), or says the next work needs a design
  session and names its subject. The user starts it in a new session.
- **Decisions are append-only** (`principles/decision-ledger.md`). Run `/design-check` on
  every design decision and every change to simulation outcomes. Only the user authorises
  a Decision.
- **Hand-offs go through GitHub** (`/handoff`). Claim another agent took the work only
  if you can give its session link.
- **A schema change updates the designer in the same PR.** Changes under
  `content/definitions/` or to the crate's content structs (`native/rust/src/content/`, the
  schema, Decision 153) must also update `tools/designer/` (export, `loadFromExport`, the
  importer under `content/import/`).
- **Anything that should change how the next agent works goes in the repo,** not in your
  own memory: here, a skill, a Decision or an issue labelled `question` (Decision 148).

## Which session are you?

- **Design:** you discuss options with the user, one subject at a time. You write
  Decisions, spec rounds under `specs/`, and open questions as issues labelled `question`
  (how: `OPEN_QUESTIONS.md`'s header). You don't write code. You finish a round by pushing a
  `docs:` PR whose hand-off names the next `/run-phase` item, and which says `Closes #N` for
  each question it settles. `github-sync` gives the PR and those questions the spec's
  milestone; set none yourself.
- **Implementation:** you build one spec round with `/run-phase`. Build what the spec
  settles as written. Stop and ask only where it is silent, and record the answer in the
  spec or a Decision.
- When the user's request mixes both, ask which this session is. When a session has been
  reset (its context summarised) more than once, suggest handing off and starting fresh.

## Where things live

| What | Where |
|---|---|
| Decisions | `DECISIONS_INDEX.md` lists each Decision with its line. Read by line; don't read the whole design spec (it is 400 KB) |
| Design spec and addenda | `Breach — Reverse Tower Defense Design Spec.md`, `breach-addendum-*.md` |
| Specs, one per feature or round | `specs/NN-*.md`, the source. `github-sync` makes each spec's steps issues under `Spec NN: <title>`, one per `### Step <id>`; a step's PR closes its issue |
| Open questions and the user's "remind me when…" | Issues labelled `question` (Decision 148); check those whose `raise-when:` label or **Raise when** matches your work. `OPEN_QUESTIONS.md` says how to raise one and keeps the closed archive |
| README, changelog, docs index | `README.md`, `CHANGELOG.md`, `docs/index.md` (Diátaxis; links to docs that live beside their code) |
| Gates | `PROGRESS_LOG.md` (Gate 1, generated), `QUALITATIVE_REVIEW_LOG.md` (Gate 2, the user's) |
| CI and checks | `ci/godot/`, with known environment problems listed in `ci/godot/README.md` |
