---
doc: rubrics/run-baseline
status: active
verified_by: automated tooling, at the end of every implementation pass (local pre-commit and CI)
---

# Run Baseline Rubric

Every implementation pass — a full plan stage or an ad-hoc prompt that changes source —
is checked against this table at its declared end. Every row here is either mechanical
(a script/tool with a real pass/fail) or procedurally enforced per
`principles/agent-workflow.md` where no such tool exists yet for a given stack — never a
bare manual-review question with no enforcement at all (qualitative judgement belongs in
`principles/progress-tracking.md` Gate 2, which runs separately and later). A stack's
`ci/<stack>/README.md` states which is which for every row, per
`templates/ci-tool-mapping.md`.

## Structured (machine-checkable, hard gate)

| Name | Description | Threshold | Pass Condition |
|---|---|---|---|
| `tests-red-then-green` | For `code`/`hybrid` specs, the test(s) for new behaviour failed before implementation and pass after | Documented red→green transition per `principles/tdd-bdd-workflow.md` | Test run log shows both states |
| `contract-tests-pass` | Every implementation of a base type/interface with 2+ implementations passes the shared contract-test suite unmodified | 100% pass | Test runner exit 0 across all implementations |
| `coverage-overall` | Overall line coverage | ≥90% (configurable, `config/thresholds.yaml`) | Coverage tool report |
| `coverage-changed-lines` | Coverage on new/changed lines specifically | ≥90% (configurable) | Coverage tool report, diff-scoped |
| `lint-clean` | Project linter (stack-specific) reports zero errors | Zero errors | Linter exit 0 |
| `srp-size` | No file/function exceeds configured size thresholds without a Decision | Zero unrecorded violations | Static check against `config/thresholds.yaml` |
| `ocp-shotgun-surgery` | No new case/behaviour touches ≥ configured number of pre-existing files | Below threshold | Diff-scoped static check |
| `isp-method-count` | No interface exceeds configured method-count threshold | Below threshold | Static check |
| `isp-stub-detection` | No implementer body is a not-implemented/no-op stub for a required method | Zero | Static/AST scan |
| `dip-direction` | No dependency edge from high-level/domain module to low-level/infrastructure module in the wrong direction | Zero violations | Dependency-graph analysis |
| `naming-grep-discoverable` | No identifier exceeds configured max grep-hit count while being generic | Zero unrecorded violations | Grep-count check |
| `no-cross-cutting-helper-violation` | No helper below the promotion threshold has been extracted into a shared file; no helper at/above threshold remains duplicated instead of promoted | Zero violations | Static check |
| `order-independent-simulation` | No simulation outcome depends on identifiers or creation order: no identifier compared, sorted, or used as a tie-break; a symmetric scenario run both ways round agrees within noise (`principles/rules-over-cases.md`, Principle 2) | Zero violations; swapped-order results within noise | Static check for identifiers used to rank, plus a swapped-order trial (procedural where no trial harness exists) |
| `bounded-scaling` | A tick's cost grows no faster than n log n in the things simulated: doubling the scenario costs at most the configured factor per doubling, marching and in contact (`principles/bounded-work.md`) | Every doubling within the limit (Breach: 2.5) | The stack's scaling check (Breach: `tools/scaling_check.gd`, in CI), then a per-phase timing to find a failing pass |
| `progress-trend` | Rubric pass count, test count, lint warnings, and size budgets have not regressed versus the pre-change baseline | No regression, per `principles/progress-tracking.md` Gate 1 | Log comparison |
| `commit-message-conforms` | Commit message(s) conform to Conventional Commits, and the declared type matches the actual diff (e.g. no `fix:` that adds a new public API) | 100% conformance | Commit-lint + diff-type cross-check, or procedural per `principles/agent-workflow.md` if no linter exists for this stack yet |
| `docs-current` | README, changelog and `docs/` are updated for the change or marked N/A with a reason, per `principles/documentation-phases.md` | Gate passes; Documentation block present | `scripts/check-docs.sh all` exit 0, plus the PR's Documentation block (procedural: whether the words are still true) |
| `branch-name-conforms` | Branch name conforms to Conventional Branch, per `principles/agent-workflow.md`'s purpose-driven-prefix rule | Conformance | Branch-name lint, or procedural per `principles/agent-workflow.md` if no linter exists for this stack yet |

## Where this runs

- **Locally**, via the pre-commit framework, on changed files, for fast feedback before
  a commit is made.
- **In CI**, on the full changed set, as the authoritative, required gate before merge —
  see the stack-specific `ci/<stack>/` implementation for exact wiring.

A failure in any row here means the implementation pass is **not complete**. This is
independent of and prior to the human qualitative gate in
`principles/progress-tracking.md`. Passing this table is also not, by itself,
authorization to merge — the change still lands via the branch-and-review workflow in
`principles/agent-workflow.md`, which the implementing agent opens and then stops at.
