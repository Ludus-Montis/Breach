# AI-First Development Kit — Documentation Layer

This is the **stack-agnostic** layer of a two-layer starter kit for AI-driven,
quality-gated software development. It defines principles and rubrics that do not
depend on any particular language, framework, or engine. A separate,
**stack-specific implementation layer** (`ci/<stack>/`, named for whatever language or
engine the adopting project actually uses — e.g. `ci/godot/`, `ci/node/` — built once per
project, not shipped here) wires these rubrics into actual tooling: linters, coverage
tools, contract-test runners, and CI/pre-commit configuration.

This kit is independent of any other project-specific tooling — it introduces no
shared terminology, file paths, or dependencies beyond what is documented here, and is
intended to be dropped into any greenfield project as-is.

**Bootstrapping a brand-new (or currently bare) repository from this kit? Start with
[`QUICKSTART.md`](QUICKSTART.md)**, not the reading order below — it turns everything in
this kit into an ordered, ask-then-do checklist covering source control, the remote, the
stack-specific `ci/<stack>/` layer, and testing, for whatever stack you name. The reading
order below is for understanding the kit's own reasoning once that's done, or for
retrofitting it onto a project that already has some of this in place.

## Layout

```
QUICKSTART.md                Ask-then-do checklist for bootstrapping a new project
README.md                    This file

principles/
  ai-first-organisation.md   Four file-organisation principles (concern, locality,
                              naming, no cross-cutting helpers)
  solid-mechanical.md        SOLID as mechanical checks + honestly-scoped manual review
  agent-workflow.md          Branch-and-review is a standing rule, not an inference;
                              what an agent never does alone
  agent-sessions.md          The entry file every session loads; design and
                              implementation in separate sessions; the open-questions
                              register; hand-overs through the remote
  decision-ledger.md         Append-only decision convention
  progress-tracking.md       Mechanical hard gate + separate human qualitative gate
  tdd-bdd-workflow.md        Behaviour-first workflow, spec-type taxonomy, coverage floor
  rules-over-cases.md        General rules over special cases; identity and order never
                              decide outcomes
  bounded-work.md            Per-tick work is local and bounded; work follows change

rubrics/
  spec-baseline.rubrics.md   Checked when a specification is authored
  run-baseline.rubrics.md    Checked at the end of every implementation pass

templates/
  decision-entry.md          Copy-paste blocks for new/superseding decisions
  commit-message.md          Conventional Commits template + type-vs-diff correctness
  pr-description.md          PR template linking rubric checklist + progress log
  ci-tool-mapping.md         Template for a stack-specific ci/<stack>/README.md

config/
  thresholds.yaml            All configurable numeric thresholds in one place
```

## Reading order for a new project

1. `principles/ai-first-organisation.md`, `principles/solid-mechanical.md`,
   `principles/rules-over-cases.md` and `principles/bounded-work.md` — the design rules
   everything else enforces.
2. `principles/tdd-bdd-workflow.md` — how behaviour gets specified and implemented.
3. `principles/agent-workflow.md` and `principles/agent-sessions.md` — how a change
   actually gets proposed and landed, and how work passes between agent sessions.
4. `principles/decision-ledger.md` and `principles/progress-tracking.md` — how the
   project records its own history and trend.
5. `rubrics/spec-baseline.rubrics.md` and `rubrics/run-baseline.rubrics.md` — the
   checklists that tie 1–4 into concrete pass/fail gates.
6. `templates/` — the boilerplate you'll actually copy while working.
7. The stack-specific `ci/<stack>/` layer (not yet built) — how the rubrics in step 5
   actually get executed, locally and in CI, for your chosen language/engine. Document
   it using `templates/ci-tool-mapping.md`'s shape: a rubric→tool mapping table, an
   honest heuristic-limits section, and a procedural-not-scripted section for whatever
   isn't mechanically checkable yet.

## Versioning note

This kit itself is a set of specifications. Changes to any file here follow the same
rules it defines for everything else: a `policy`-type spec, changes to a Decision are
append-only, and any threshold change in `config/thresholds.yaml` needs a recorded
Decision.
