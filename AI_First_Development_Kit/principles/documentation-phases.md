---
doc: principles/documentation-phases
status: active
applies_to: every change, after implementation and before its proposal is review-ready
---

# Documentation Phases

## Why this exists

Documentation that is written "later" drifts from the code until nobody trusts it. These
three phases run **after implementation and before a change is considered done**. They are
part of the workflow, not an afterthought. A change is not review-ready until each phase has
either produced output or been explicitly marked "N/A" with a reason.

Baselines:
- README: [Standard Readme](https://github.com/RichardLitt/standard-readme)
- Changelog: [Keep a Changelog](https://keepachangelog.com) + [SemVer](https://semver.org)
- Documentation: [Diátaxis](https://diataxis.fr)

Paths below are relative to this kit, except `README.md`, `CHANGELOG.md` and `docs/`, which
are at the project root. Run the gate from the project root:
`bash <kit>/scripts/check-docs.sh [readme|changelog|docs|all] [base-ref]`.

## Step 0: Documentation impact assessment

Before the three phases, classify the change. Use `templates/doc-impact-assessment.md`.

| Change type                                  | README | Changelog | Docs |
|----------------------------------------------|:------:|:---------:|:----:|
| New feature / capability                     | check  | **yes**   | **yes** |
| Behaviour change / breaking change           | check  | **yes**   | **yes** |
| Bug fix (user-visible)                       | no     | **yes**   | if behaviour documented |
| Install / setup / dependency / config change | **yes**| **yes**   | check |
| Public API / interface change                | check  | **yes**   | **yes** (reference) |
| Architectural decision                       | no     | no        | **yes** (explanation / decision ledger) |
| Internal refactor, no behaviour change       | no     | no        | no |
| Tests, CI, formatting only                   | no     | no        | no |

"check" means: read the README section and confirm it is still true. If it is, no edit is
needed. Read only the section the change touches, not the whole file
(`principles/agent-sessions.md`, rule 4).

## Phase 1: README additions

**Goal:** the README stays accurate, minimal, and Standard Readme compliant.

**Inputs:** the diff, the impact assessment, the current `README.md`.

**Actions:**
1. For each README section the change touches (Install, Usage, API, Security, Background),
   verify it is still correct and update it.
2. Keep the README to orientation: what it is, how to install, how to use, how to
   contribute. Anything longer goes to Phase 3 and is linked.
3. Section order must follow Standard Readme: Title, Badges, Short description (120 chars
   max), Long description, Table of Contents (if README exceeds 100 lines), Security,
   Background, Install, Usage, Extra sections, API, Maintainers, Thanks, Contributing,
   License.
4. Add or fix links to any new docs from Phase 3.

**Exit criteria:**
- [ ] `scripts/check-docs.sh readme` passes
- [ ] Every command shown in Install/Usage has been run or verified against the code
- [ ] No content duplicated from `docs/` (link instead)

## Phase 2: Changelog

**Goal:** a human-readable record of notable changes for people using the software.

**Inputs:** the diff, commit messages (Conventional Commits, `templates/commit-message.md`),
`CHANGELOG.md`.

**Actions:**
1. Add entries under `## [Unreleased]` in the correct category: Added, Changed, Deprecated,
   Removed, Fixed, Security.
2. Write for the user, not the implementer: "Players can now X", not "Refactored Y".
3. Mark breaking changes with a leading `**BREAKING:**`.
4. Map commit types to categories: `feat` -> Added, `fix` -> Fixed, `perf`/`refactor` with
   visible effect -> Changed, `feat!`/`BREAKING CHANGE` -> Changed + BREAKING, security
   fixes -> Security.
5. At release time only: rename `[Unreleased]` to `[x.y.z] - YYYY-MM-DD`, choose the version
   bump (breaking -> major, Added -> minor, Fixed only -> patch), add a fresh empty
   `[Unreleased]`, and update the compare links at the bottom.

**Exit criteria:**
- [ ] `scripts/check-docs.sh changelog` passes
- [ ] Entry exists for every row marked "yes" in the impact assessment
- [ ] Version bump is consistent with the highest-impact entry

The changelog gate fails when source files changed and `CHANGELOG.md` did not. When a change
has nothing a user would notice, write `Changelog: N/A (<reason>)` in the proposal's
Documentation block (below): the CI wiring (`templates/ci/docs-gate.yml`) reads it and skips
that one check. Locally, `DOCS_SKIP_CHANGELOG=1` does the same.

## Phase 3: Additional documentation

**Goal:** everything too detailed for the README lives in `docs/`, organised by Diátaxis
quadrant.

| Quadrant        | Answers                 | Location              | Write it when |
|-----------------|-------------------------|-----------------------|---------------|
| Tutorials       | "Teach me"              | `docs/tutorials/`     | a new user-facing capability needs a guided first run |
| How-to guides   | "Help me do X"          | `docs/how-to/`        | a task is non-obvious or multi-step |
| Reference       | "What exactly is X?"    | `docs/reference/`     | an API, config option, CLI flag, data format, or schema changed |
| Explanation     | "Why is it this way?"   | `docs/explanation/`   | a design decision was made (see "Decisions" below) |

A document that already lives next to the thing it describes (a tool's own README, the
`ci/<stack>/README.md`) stays where it is: the index links to it in its quadrant rather than
moving or copying it.

**Actions:**
1. Using the impact assessment, update or create the doc in the matching quadrant. Do not
   mix quadrants in one page.
2. Update `docs/index.md` (the index) so every page is reachable.
3. Record architectural decisions (see "Decisions" below).
4. Remove or fix any doc now contradicted by the change.

**Exit criteria:**
- [ ] `scripts/check-docs.sh docs` passes (index links resolve, no orphan pages)
- [ ] Every new or changed public surface has a reference entry
- [ ] Every significant decision is recorded

### Decisions

The decision ledger (`principles/decision-ledger.md`, `templates/decision-entry.md`) is this
kit's architecture decision record. A project that keeps the ledger records decisions there
and does not also write ADRs: two records of one decision drift apart. The docs index links
to the ledger (or its index) under Explanation. A project without a ledger keeps ADRs in
`docs/explanation/adr/`, from `templates/adr.template.md`. Either way the same rule holds as
for any Decision: an agent proposes the text, a human authorises it
(`principles/agent-workflow.md`).

## Recording the outcome

Add this block to the PR description (or task completion note). "N/A" requires a reason.

```
### Documentation
- README: updated | verified, no change | N/A (reason)
- Changelog: entry added under [Unreleased] | N/A (reason)
- Docs: <paths updated/created> | N/A (reason)
```

Prompts that run each phase as a separate agent step are in
`templates/documentation-prompts.md`.

## Mechanical vs procedural

Mechanical: `scripts/check-docs.sh` checks README structure, the changelog's format and that
it changed with the source, and that the docs index has no broken links or orphan pages. It
runs in CI on every proposal (`templates/ci/docs-gate.yml`).

Procedural: whether the words are still *true*. No script can tell that an Install command
still works or that a reference page matches the code. The impact assessment and the exit
criteria above are how that gets checked, and the Documentation block is the record that it
was.
