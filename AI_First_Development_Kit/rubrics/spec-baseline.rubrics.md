---
doc: rubrics/spec-baseline
status: active
verified_by: the spec agent or reviewer, at authoring time, before implementation begins
---

# Spec Baseline Rubric

Every specification is checked against this table before implementation starts. A
specification that fails a `Structured` row is not ready to implement. `Qualitative`
rows require a stated manual-review answer in the specification's own text (typically in
a Decisions or Notes section), not a bare pass/fail claim.

## Structured (machine-checkable)

| Name | Description | Threshold | Pass Condition |
|---|---|---|---|
| `spec-type-declared` | Frontmatter declares `spec_type: code \| policy \| hybrid` | Present and valid | Frontmatter lint passes |
| `tdd-plan-present` | For `spec_type: code` or the tooling half of `hybrid`, Steps include Given/When/Then scenarios and a stated test-first order | Present for all code-bearing steps | Manual/spec-agent check per `principles/tdd-bdd-workflow.md` |
| `decision-ledger-append-only` | No diff modifies text inside an existing `### Decision N` block, other than an allowed `> Superseded by ...` note | Zero unauthorised edits | `git diff` inspection per `principles/decision-ledger.md` |
| `threshold-changes-are-decisions` | Any change to a value in `config/thresholds.yaml` is accompanied by a new Decision entry | 1:1 | Diff cross-reference |
| `ai-first-thresholds-respected` | Steps do not silently exceed the configured thresholds in `principles/ai-first-organisation.md` without a Decision | Zero unrecorded violations | Manual/spec-agent check |
| `no-drift` | Specification does not contradict any Decision in a linked parent specification | Zero contradictions | Manual review of every parent Decision in Backlinks |
| `spec-schema-compliance` | Required frontmatter fields and body sections present, correctly ordered | 100% | Schema validation |
| `commit-classification-plan` | Steps state the intended Conventional Commit type(s) for the change | Present | Manual/spec-agent check |

## Qualitative (manual review, answer recorded in the spec)

| Name | Review question |
|---|---|
| `single-noun-phrase` | Can every new file's purpose be stated as one noun phrase with no conjunction? |
| `ocp-extension-point` | Does new behaviour plug into an existing extension point rather than editing shipped logic in many places? |
| `lsp-contract-scope` | For any new implementation of an existing contract, does the spec identify which shared contract-test suite it must pass? |
| `isp-fit` | Does any interface a step introduces force an implementer to depend on methods it doesn't need? |
| `dip-direction` | Does the spec introduce any import of a low-level/infrastructure module from high-level/domain logic? |
| `pre-mortem` | Does the spec open with a `## Pre-mortem` (regret in a month, riskiest assumption, cheapest test), written at the start of the design round, and is the test either scheduled as a step or already answered (`principles/agent-sessions.md`, §3; `templates/pre-mortem.md`)? |
| `general-over-special` | Does every new behaviour come from a general rule, not a special case? For any special case, is the human owner's reason recorded with its Decision (`principles/rules-over-cases.md`, Principle 1)? |
| `order-independent` | Would any outcome change if the same things were created, or listed, in the opposite order (`principles/rules-over-cases.md`, Principle 2)? |
| `bounded-work` | What bounds each thing the design checks every tick: a radius or count, or the size of the world? Does a check repeat on things that can't have changed, and if it is driven by changes instead, what queues it (`principles/bounded-work.md`)? |
| `project-agnostic-language` | (Policy specs affecting shared principle documents only) Does the change avoid embedding project-specific paths, languages, or tooling names into a document meant to stay stack-agnostic? |

A specification may proceed to implementation once every Structured row passes and every
Qualitative row has a recorded answer — "unreviewed" is not an acceptable answer.
