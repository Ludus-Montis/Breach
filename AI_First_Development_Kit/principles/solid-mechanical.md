---
doc: principles/solid-mechanical
status: active
applies_to: all specifications and all source files
---

# SOLID as Mechanical Checks

## Why this exists

"Follow SOLID" as prose is not enforceable — it reads differently to every agent and
every reviewer, and most SOLID checkers in mainstream ecosystems are heuristic at best.
This document commits, per letter, to either a genuine mechanical check or an honestly
labelled manual-review question. Nothing here is a slogan; each item below is either
testable by tooling or explicitly flagged as requiring human/agent judgement, with the
specific question the reviewer must answer.

All numeric thresholds are configurable — see `config/thresholds.yaml`.

## S — Single Responsibility

**Fully mechanical.** Covered directly by `principles/ai-first-organisation.md`,
Principle 1 (one concern per file) and its "single noun phrase" test. No separate check
needed here; this is not a distinct rule, it is the same rule under a different name.

Supporting size proxy: a file or function above `solid.srp.max_file_lines` (default:
**300**, matching the general context-budget policy) or
`solid.srp.max_function_lines` (default: **40**) is presumptively doing too much and
should be reviewed against the one-concern test.

## O — Open/Closed

**Mechanical: shotgun-surgery detection.**

**Test:** Does this change add one new case, type, or behaviour by requiring edits to
`solid.ocp.max_touched_files_per_new_case` (default: **3**) or more **pre-existing**
files? A common concrete pattern this catches: a large `switch`/`match` statement (or
equivalent chain of `if/elif`) whose cases are scattered across the codebase, so adding
one new case means touching every file that switches on that type.

**Pass:** New behaviour is added via a new file/module/case that plugs into an existing
extension point (a new implementation of an interface, a new registered handler) without
editing already-shipped logic.

**Waiver:** a deliberate broad change (a refactor across many files, not one new case)
is authorised by a Decision, and every commit series under it carries a
`Waived-by: Decision N` trailer. The check still lists the files it counted, then passes.
Only the owner authorises the Decision; the trailer only cites it.

## L — Liskov Substitution

**Primary mechanical enforcement: shared contract tests.**

Any base type, interface, or abstract class with `solid.lsp.min_implementations_for_
contract_test` (default: **2**) or more implementations **must** have a single shared
behavioural test suite written against the base contract, executed unmodified against
every implementation. A new implementation that fails the existing suite is a hard
failure, not a review comment — this ties directly into the TDD/BDD workflow (see
`principles/tdd-bdd-workflow.md`): the base contract's test suite *is* the specification
for that abstraction.

**Supporting mechanical smells** (raise a flag, do not by themselves fail the build):
- **Override-throws:** an overridden method whose body is primarily a
  not-implemented/not-supported error, or an empty no-op, where the base method has real
  behaviour. Detectable via static scan for `NotImplementedError`, `unimplemented!()`,
  `push_error`/`assert(false)`, or equivalent, inside an override.
- **Precondition-strengthening:** an override that adds validation/guard clauses not
  present in the base before performing the base's work. Detectable by comparing
  control-flow shape between base and override; flag for manual confirmation rather than
  auto-fail, since some added validation is legitimate.

**Honest limit, stated plainly:** contract tests only catch violations for behaviour the
suite actually exercises. This is a strong practical guarantee, not a formal proof of
substitutability. Do not treat a green contract-test run as a claim that LSP is fully
satisfied — only that the tested contract is honoured.

## I — Interface Segregation

**Mechanical, baseline tier:**
- **Method-count threshold:** an interface/trait/protocol above
  `solid.isp.max_interface_methods` (default: **7**) is presumptively too fat and should
  be split.
- **Stub-detection in implementers:** an implementer whose method body is a
  not-implemented/not-supported error, or a default value returned purely to satisfy the
  contract (with no real behaviour), is direct evidence that interface is forcing that
  implementer to depend on a method it does not need. This is the strongest ISP signal
  available and is treated as a hard violation, not a heuristic.

**Mechanical, optional/advanced tier:**
- **Usage-subset analysis:** for each caller of an interface, compute which subset of
  its methods that caller actually invokes. If callers cluster into non-overlapping
  subsets, that is a structural signal the interface should be split along those lines.
  Requires call-graph tooling; adopt when available, do not block v1 rollout on it.

## D — Dependency Inversion

**Mechanical: import/dependency-direction analysis.**

**Test:** Build a directed graph of module dependencies (via static imports/`extends`/
`preload`/equivalent). Flag any edge where a designated "low-level" module (I/O,
persistence, framework/engine-specific code) is imported by a designated "high-level"
module (domain/business logic) in the wrong direction — i.e. domain logic should not
import concrete infrastructure directly; infrastructure should depend on domain-defined
abstractions, not the reverse.

Each project's stack-specific implementation layer supplies the actual graph-building
tool (see the relevant `ci/<stack>/` directory); this document defines the check, not
the tool.

## Summary table

| Letter | Enforcement | Nature |
|---|---|---|
| S | One-concern-per-file test + size proxy | Fully mechanical |
| O | Shotgun-surgery detection (files touched per new case) | Fully mechanical |
| L | Shared contract-test suite (hard gate) + 2 supporting smells (flagged) | Mechanical, with a stated coverage limit |
| I | Method-count threshold + stub detection (hard) + usage-subset analysis (optional) | Mechanical, baseline + optional tier |
| D | Import/dependency-direction graph analysis | Mechanical, tool supplied per stack |

Nothing in this document is left as an unenforceable slogan. Where a check has a stated
limit (L in particular), that limit is documented here rather than silently assumed away.
