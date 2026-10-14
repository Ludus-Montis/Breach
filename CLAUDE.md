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
- **Tests come first:** red, then green (`principles/tdd-bdd-workflow.md`). Run the suite
  with `bash ci/godot/scripts/run_tests.sh`, which needs `GODOT_BIN` set.
- **Decisions are append-only** (`principles/decision-ledger.md`). Run `/design-check` on
  every design decision and every change to simulation outcomes. Only the user authorises
  a Decision.
- **Hand-offs go through GitHub** (`/handoff`). Claim another agent took the work only
  if you can give its session link.
- **A schema change updates the designer in the same PR.** Changes under
  `content/definitions/` must also update `tools/designer/` (export, `loadFromExport`, the
  importer under `content/import/`).
- **Anything that should change how the next agent works goes in the repo,** not in your
  own memory: here, a skill, a Decision or `OPEN_QUESTIONS.md`.

## Which session are you?

- **Design:** you discuss options with the user, one subject at a time. You write
  Decisions, spec rounds under `specs/`, and `OPEN_QUESTIONS.md` entries. You don't write
  code. You finish a round by pushing a `docs:` PR whose hand-off names the next `/run-phase`
  item.
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
| Specs, one per feature or round | `specs/NN-*.md` |
| Open questions and the user's "remind me when…" | `OPEN_QUESTIONS.md`. Check entries whose **Raise when** matches your work |
| Gates | `PROGRESS_LOG.md` (Gate 1, generated), `QUALITATIVE_REVIEW_LOG.md` (Gate 2, the user's) |
| CI and checks | `ci/godot/`, with known environment problems listed in `ci/godot/README.md` |
