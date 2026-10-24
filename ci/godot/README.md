# Godot Tooling Layer

Wires the stack-agnostic rubrics in `AI_First_Development_Kit/rubrics/` into actual
GDScript/Godot 4.7 tooling: gdUnit4 for tests, gdtoolkit for lint/format, and a handful
of small local scripts (`ci/godot/scripts/`) for the checks nothing off-the-shelf covers
yet. Every script here was run against a deliberately broken case before being trusted
— see each script's own docstring/comment for exactly what was verified and how.

## Rubric → tool mapping

| Rubric row | Tool | Where |
|---|---|---|
| `tests-red-then-green` | gdUnit4 v6.2.1 (`addons/gdUnit4`) via `ci/godot/scripts/run_tests.sh` | pre-commit (the staged `tests/**/test_*.gd` only, `staged_tests.py`) and CI (the full suite) |
| `lint-clean` | `gdlint` (gdtoolkit) | pre-commit, on changed `*.gd` files (excludes `addons/`) |
| formatting (not a named rubric row, but `commit-message-conforms`/PR review expects it) | `gdformat --check` | pre-commit |
| `srp-size` (file-length half) | gdlint's `max-file-lines` (`.gdlintrc`, set to 300 to match `config/thresholds.yaml`) | pre-commit |
| `srp-size` (function-length half) | `ci/godot/scripts/check_function_length.py` | pre-commit |
| `dip-direction` | `ci/godot/scripts/check_dependency_direction.py` | pre-commit |
| `naming-grep-discoverable` | `ci/godot/scripts/check_generic_naming.py` | pre-commit |
| `order-independent-simulation` (static half) | `ci/godot/scripts/check_id_order.py` | pre-commit and CI, on `sim/` |
| `one-mover` | `ci/godot/scripts/check_one_mover.py` | pre-commit and CI, on `sim/`: a position written outside UnitMover unless marked `# placement: <why>` (Decision 139) |
| `one-state` | `ci/godot/scripts/check_one_state.py` | pre-commit and CI, on `sim/`, `tools/`, `presentation/` and `tests/`: a unit's state written outside UnitState (`.state = SkirmishUnit.State.`), `UnitState.start` in `sim/`, or a kept list from `living()` or `UnitState.downed` changed by its caller (Decision 147) |
| `store-reads-once` | `ci/godot/scripts/check_store_reads.py` | pre-commit and CI, on `sim/`: the unit store read or written unit by unit inside a loop, unless the line or its loop is marked `# store: scattered (few: <why>)` or `(reference: <why>)` (Decisions 144 and 146) |
| `order-independent-simulation` (trial half) | swapped-spawn-order trial, per `.claude/skills/design-check` | procedural, on any change to simulation outcomes |
| `isp-method-count` / `isp-stub-detection` | `ci/godot/scripts/check_isp.py` | pre-commit and CI |
| `no-cross-cutting-helper-violation` | `ci/godot/scripts/check_helper_promotion.py` | pre-commit and CI |
| `ocp-shotgun-surgery` | `ci/godot/scripts/check_ocp_shotgun_surgery.py <base-ref>` | CI only — needs a diff against a real base ref, not meaningful on a bare working tree |
| `ai-first-thresholds-respected` (context-locality half) | `ci/godot/scripts/check_context_locality.py <base-ref>` | CI only, informational (never fails) — see Decision 12 and Heuristic limits below |
| outcomes kept (not a named rubric row: spec 35's "a speed-up must leave every line the same") | `ci/godot/scripts/check_outcomes.py <set> <base digest> <head digest> <base-ref>` | CI only, in `sim-checks`' `digests` jobs: each set's Rust digest against the base's, tick for tick, the base's core built once by `base-build` (Decision 151); `--engine rust` names the engine in the report. A change fails unless one of the branch's commits says why in an `Outcomes-changed: <why>` trailer (a Decision, a fix, a digest that hashes more). When runs changed, the first 3 are traced on the base and the branch around their first difference and diffed (`trace_changed.sh`, `trace_diff.py`, spec 37): `traces/` in the set's `sim-digests-<set>` artifact, each diff's first line in the SUMMARY |
| `branch-name-conforms` | `ci/godot/scripts/check_branch_name.sh` | pre-commit stage |
| `docs-current` | `AI_First_Development_Kit/scripts/check-docs.sh all <base-ref>` (README structure, changelog format and that it changed with the source, docs index links) | CI (`docs-gate` workflow); skips the changelog check when the PR body says `Changelog: N/A`. Whether the words are still true is procedural: the PR's Documentation block |
| `commit-message-conforms` (format half) | `ci/godot/scripts/check_commit_message.sh` | commit-msg stage |
| `commit-message-conforms` (type-vs-diff half) | same script, heuristic warning only — see Heuristic limits | commit-msg stage |
| `progress-trend` | `ci/godot/scripts/gate1_progress_log.py` | run manually at the end of an implementation pass; see `principles/progress-tracking.md` |

Four rows above (`isp-method-count`/`isp-stub-detection`, `no-cross-cutting-helper-violation`,
`ocp-shotgun-surgery`, and the `extends`-a-Node-type half of `dip-direction`) were ported
from [SSidey/Sweepminer](https://github.com/SSidey/Sweepminer)'s `dev_kit/ci/godot/`
layer — a sibling project built from the same `AI_First_Development_Kit`, which had
already solved these as real mechanical checks rather than leaving them procedural.
One claim from that project was checked and found stale, not ported: its README states
coverage is automated via a GdUnit4 "`-c` coverage flag" — `-c` is actually
`--continue` (run the full suite without failing fast, per gdUnit4's own `CmdOptions`),
not a coverage flag, and that project's own `run_tests.ps1`/`report_progress.py`
comments admit coverage is still manual. Confirms this repo's own coverage gap below is
real, not an oversight.

## Which checks a change runs

CI decides by the files a PR changes, not by its commit type
(`ci/godot/scripts/needs_godot.py`, with its tests):

- **Docs, specs, skills, the kit, Python-only tools and checks** (`*.md`, `.claude/`,
  `specs/`, `AI_First_Development_Kit/`, `ci/godot/scripts/*.py`, `tools/designer/`):
  lint, the static checks and the Python tests. The gdUnit4 suite, the scaling check and
  `sim-checks` are skipped; a skipped job counts as passing.
- **Anything Godot or the simulation reads** (`.gd`, `.tres`, `.tscn`, `native/`,
  `content/`, `sim/`, `tests/`, `presentation/`, `project.godot`) **and the CI wiring
  itself** (`.github/`, `.pre-commit-config.yaml`, `run_tests.sh`): everything.
- Pushes to `main` always run everything.

## How CI is laid out

Every check that doesn't need another's output runs in its own job, side by side.

- **`CI`** (`ci.yml`): `run-baseline` (lint, the static checks, the Python tests and
  commit messages: pre-commit with its `run-tests` hook skipped), `tests` (the gdUnit4
  suite, in shards) and `scaling` (the scaling check, one job per stage; both on the core,
  the one engine since spec 39 step 6a).
- **`Sim checks`** (`sim-checks.yml`): `build` compiles the Rust core once and hands it on
  as an artifact, and `base-build` does the same for the merge base; then `suite` (the suite
  under Rust, in shards, each failing when a test does: it keeps gdUnit's exit status
  past the `sed` that writes its log), `digests` (one job per set, each comparing
  its digest with the base's from a worktree of the merge base given the base's
  core: `check_outcomes.py`, Decision 151), `trials` and `scaling` (one job per
  stage) run together; `sim-checks` waits for them all and prints the SUMMARY block.
- **Why the digests compare with the base:** two engines agreeing doesn't show an outcome
  is kept: a change in shared GDScript rules moves both alike. Spec 35's steps were checked
  only engine against engine until 2026-10-17 (found reviewing step 6c). The run under each
  engine retired with the last twin (spec 39 step 6a).
- **The scaling check splits by stage, never by size.** It checks how a tick's time grows
  from one size to the next, and timings only compare when taken on one machine: each
  stage (march, fight, stacks) runs all its sizes in one job (`scaling_check.gd`'s
  `stages=`). Determinism makes the battles the same everywhere, not the runners' speed.
- **The diff-scoped checks' base** (Decision 50): a PR's base branch, or for a push the
  commit before it (`github.event.before`). A push checks out the branch itself, and
  `base_ref.py`'s detection would then pick the nearest other branch and count changes
  the push didn't make; that is why pushes to `main` failed `run-baseline` before
  2026-10-15.
- Godot setup is one composite action, `.github/actions/godot` (cached binary, import).
- **Shards.** `shard_tests.py <count> <index>` splits the test files into balanced groups
  by the per-file times in `ci/godot/test_times.json`; a whole file always runs in one
  shard, since each file is self-contained. Both workflows' `SHARDS` and their matrices
  must agree. A new file with no recorded time counts as the median, so the times only
  need refreshing when the shards drift apart: download a run's `gdunit4-reports-*`
  artifacts and run `shard_tests.py --update <each results.xml>`.

## Heuristic limits (stated plainly, not hidden)

- `check_store_reads.py` — flags a scattered store call (`columns.value`/`put`,
  `UnitPlace.of`/`rank`/`put`, `UnitHealth.*_one`, `FormationRout.where`,
  `SquadEntries.loose_at`, `EntryKeys.*`) written directly inside a `for` or `while` body,
  unless it is marked `few:` (an event's few units) or `reference:` (the GDScript
  reference's per-unit seek, walk and facing). Any other reason is flagged (Decision 146).
  It can't see a call hidden in a helper the loop calls, which is how spec 35 step 4a's place
  reads slipped through. Such helpers take the pass's view (`UnitView`) instead, and
  `bench` (`tick_phases`) against the base is the backstop. A multi-line loop header's
  reads count as the loop's body.

- `check_id_order.py` — flags an id compared with `<`/`>`, arithmetic on an id, an id
  inside an array-literal key (unless the line salts a seeded draw: `hash(`,
  `BattleRolls.`), and `ids.sort()`. Cannot see a tie kept by list order (`if key <
  best_key` keeping the first of equals, where the list is in spawn order), nor a
  sequential update where the first squad processed changes what the second sees.
  The backstops for both are the swapped-spawn-order trial and the reversed-lists tests
  (`tests/sim/skirmish/formation/test_formation_list_order.gd` and
  `test_formation_rout_order.gd`): they step a battle twice, the second time with every
  squad and unit list reversed (same ids, same seed), and require the same state each
  tick, which catches a list-order tie the trial's noise would hide. A line may be allowed with
  `# id-order-ok: <reason>`, reviewed like any other exception.
- `check_dependency_direction.py` — textual scan for `preload(`/`load(`/`extends`
  references from `sim/` into `presentation/`, plus a fixed list of Godot's common
  Node-family base types to catch a `sim/` file directly `extends`-ing an engine scene
  type. Cannot see an indirect dependency routed through a third file, does not
  understand dead/conditional code paths, and the Node-family list is fixed, not a
  lookup against the engine's actual class hierarchy — an obscure Node-derived type not
  on the list would be missed.
- `check_function_length.py` — measures a top-level `func` to the next top-level
  declaration. A function nested inside an inner `class` block is not measured
  separately (known blind spot, not silently wrong — see the script's own docstring).
- `check_generic_naming.py` — flags the kit's example generic names
  (`process`/`handle`/`util`/`helper`/`common`/`base`/`manager`/`data`) at or above the
  grep-hit threshold. Cannot distinguish the kit's allowed "scoped to a single class,
  never called by the short name externally" exception from a genuine violation by grep
  alone — flags the hit count and leaves the scoped-exception judgment to manual review.
  (Considered broadening this to flag *every* identifier at/above the threshold,
  matching Sweepminer's literal reading of the rubric text — kept narrow instead, since
  that would flag legitimately specific, widely-used names as the codebase grows; a
  deliberate choice, not an oversight.)
- `check_isp.py` — scans production code only (`tests/` is excluded — a test suite's
  many `test_*` methods aren't an ISP concern; scanning it produced exactly that false
  positive, a 9-test suite flagged as "too fat," before the exclusion was added).
  Method-count only counts non-underscore-prefixed functions (GDScript's private/
  internal convention, which `_init` and engine virtuals also follow) — counting
  everything flagged `LaneSimulation` (7 real public methods, a private helper, and
  `_init`) as "8 methods, too fat" before this filter was added, another real false
  positive. Stub-detection flags *any* function (public or private) whose only
  statement is `pass`/`push_error(...)`/`assert(false, ...)`, not only true overrides
  of a base method with real behaviour (no inheritance graph is built) — a legitimate
  no-op virtual hook will false-positive. Even restricted to public methods,
  method-count is applied to every remaining `.gd` file, not only files genuinely
  acting as an interface for multiple implementers, since GDScript has no formal
  interface keyword to detect that distinction
  mechanically — treat a hit as a prompt to check which case it is.
- `base_ref.py` (Decision 50) — picks the base the diff-scoped checks compare against.
  A stacked branch compares with the branch below it, so changes the lower PR already
  justified aren't counted twice. Detection is "the closest remote branch whose tip is an
  ancestor of HEAD", which can be fooled by a stray remote branch sitting on HEAD's
  history; CI therefore passes the PR's real base in `BASE_REF`.
- `check_ocp_shotgun_surgery.py` — counts pre-existing files modified in a diff; it
  cannot distinguish "one new case forced N files open" from any other reason N files
  changed together (e.g. a deliberate, justified refactor). Only meaningful with a real
  base ref (a PR's merge-base), so it only runs in CI, not the working-tree-only
  pre-commit gate.
- `check_context_locality.py` — deliberately never fails (see Decision 12 in the
  design spec): the underlying principle is explicitly a judgement call about
  co-location of a feature's definition/usage/support, not a raw file-count ceiling,
  and this project's own architecture (data schema in `content/`, its `sim/` consumer,
  both their tests) routinely and legitimately exceeds the configured default of 2
  files. The script surfaces the count so a reviewer can judge "accepted pattern or
  genuine scatter," rather than the threshold going unchecked entirely — which is what
  was actually happening before a direct self-audit caught the gap. Same base-ref
  requirement as `check_ocp_shotgun_surgery.py`, CI only.
- `check_helper_promotion.py` — only catches the catch-all-filename shape of the
  violation (`utils.gd`, `helpers.gd`, `common.gd`, `base.gd`, `manager.gd`, `data.gd`);
  it can't detect a helper duplicated past the promotion threshold without call-graph
  tooling.
- `check_commit_message.sh`'s type-vs-diff cross-check — textual heuristic (new
  top-level `func`/`class_name`/`signal` lines ⇒ "looks like a capability was added").
  Both false positives (a `feat` adding only a private helper) and false negatives (a
  new public API added without a new top-level declaration, e.g. a new exported
  property) are possible. It's a warning, not a hard block, for exactly this reason.
- `check_commit_message.sh`'s merge-commit exemption — matches any subject starting
  `Merge ` (git's own default for `git merge`, and GitHub's default for a PR's
  "Create a merge commit" button) and skips Conventional Commits validation entirely
  for it, since these are mechanically generated, never freeform-authored. Hit for
  real: resolving a conflict on an open PR needs a local `git merge main`, which runs
  this hook via `commit-msg` and rejected git's own default message before the
  exemption existed. Narrow enough not to exempt a human-authored subject that merely
  starts with the word "Merged" (past tense) or similar — only the exact git/GitHub
  prefix.
- `gate1_progress_log.py`'s test-count regression check — a raw count comparison,
  which can't tell "lost coverage" from "obsolete tests deleted alongside
  intentionally deleted behaviour." A `Test-count-decrease-reason: <text>` trailer on
  any commit in the branch (Decision 17) reports the drop as disclosed rather than
  failing the gate — the lower count still shows in the log row either way, only the
  automatic FAIL is skipped, on the strength of a reason recorded in the commit
  itself. No such mechanism exists for the *lint warnings increased* half of this
  same check; that one has no known legitimate-decrease-shaped exception yet.

## Known environment gotchas

- **`gdtoolkit`'s grammar cache race (CI only).** Confirmed upstream bug —
  [Scony/godot-gdscript-toolkit#428](https://github.com/Scony/godot-gdscript-toolkit/issues/428).
  On a cold cache (every fresh CI runner), `gdlint`/`gdformat` can race to create the
  same grammar-cache directory via an unguarded `os.makedirs` — the loser raises
  `FileExistsError`, which gets misreported as `Cannot open file '<unrelated .gd
  file>': File exists`, naming whatever file happened to be mid-parse rather than the
  real cause. Hit this for real on this repo's own CI. `.github/workflows/ci.yml`
  pre-creates the cache directory before either tool runs, per the issue's documented
  workaround — a developer machine only ever races once (the directory persists after
  that), so this isn't needed locally.

## Procedural, not scripted

- `coverage-overall` / `coverage-changed-lines` — **permanently procedural, per
  Decision 10** in `Breach — Reverse Tower Defense Design Spec.md`, not an open TODO.
  **No working GDScript coverage tool was found**, verified empirically, not assumed
  from documentation. The most-starred candidate (`jamie-pate/godot-code-coverage`) was
  vendored and run against a trivial test on this project's actual Godot 4.7.2 install;
  it crashed with a real compile error (`NullCoverage` failing its own declared return
  type, `addons/coverage/coverage.gd:463`) — a genuine incompatibility with Godot 4.7's
  stricter static type checker, not a stale-docs mismatch. A sibling project
  (SSidey/Sweepminer) was checked too and confirmed to have the identical gap despite
  its README initially appearing to claim otherwise (see above) — this isn't a gap
  specific to how this repo looked for a tool. Coverage is reviewed procedurally
  instead: does every Given/When/Then scenario in the relevant `specs/*.md` file have a
  corresponding automated test, checked by a human/agent at spec-baseline review time
  rather than by a coverage percentage. Decision 10 can be superseded if a maintained
  tool later appears — that's new information, not a reason to keep this row open-ended
  in the meantime.
- `contract-tests-pass` — mechanically enforceable via gdUnit4 once a base
  contract/interface has 2+ implementations (per `solid-mechanical.md` criterion L,
  e.g. the Messenger/Hero Party Task Force contract in
  `specs/04-suspicion-and-response.md`), but none exist yet. No row to wire until
  Phase 3 introduces the first one — will be added then, against the real shape of that
  code, rather than guessed at now. (Sweepminer's approach is a naming convention —
  `tests/**/test_*_contract.gd` run by the same generic test runner — not a distinct
  script; adopt the same convention here once a real contract exists, rather than
  building anything new.)
- `tests-red-then-green`'s *red* half — gdUnit4 confirms tests pass (green); that a
  test actually failed first, for the right reason, before implementation existed is a
  process fact captured in each Phase 3 branch's own red→green log (per
  `principles/tdd-bdd-workflow.md`), not something a single CI run can verify
  retroactively.

## Running the checks locally

```bash
# One-time setup
pip install pre-commit "gdtoolkit==4.*"
pre-commit install                       # wires .git/hooks/pre-commit and commit-msg
export GODOT_BIN=/path/to/Godot_v4.7.2-stable_win64.exe   # or add godot to PATH
# Linux, or a cloud session without Godot: fetch the version CI pins (once; it imports the
# project too), then export the GODOT_BIN it prints
bash ci/godot/scripts/get_godot.sh

# Before every commit (pre-commit runs this automatically once installed)
pre-commit run --all-files

# Test run only: the full suite with no arguments (CI's job, not a session's - CLAUDE.md),
# or just the suites named, as the pre-commit hook does with the staged ones
bash ci/godot/scripts/run_tests.sh
bash ci/godot/scripts/run_tests.sh tests/sim/skirmish/formation/test_unit_store.gd

# OCP shotgun-surgery (diff-scoped). With no argument it compares against BASE_REF if set,
# else the branch this one is stacked on (e.g. the PR below it), else origin/main -
# see ci/godot/scripts/base_ref.py and Decision 50.
python3 ci/godot/scripts/check_ocp_shotgun_surgery.py
python3 ci/godot/scripts/check_ocp_shotgun_surgery.py origin/main   # an explicit base

# Context-locality (diff-scoped, informational; CI-only, but runnable locally too)
python3 ci/godot/scripts/check_context_locality.py origin/main

# Gate 1 mechanical progress log (end of an implementation pass)
python3 ci/godot/scripts/gate1_progress_log.py
```

First-time Godot setup note (verified empirically while wiring this layer): a fresh
clone needs one `godot --headless --path . --import` pass before gdUnit4's classes
resolve — without it, gdUnit4 (and previously GUT) fails with an "not been imported"
class-name error. CI's workflow runs this automatically before tests. The same pass is
needed after a pull that adds or renames `class_name` scripts: a stale class cache shows
as parse errors such as `Identifier "UnitStore" not declared in the current scope`
(seen 2026-10-15).
