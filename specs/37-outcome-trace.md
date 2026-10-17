---
spec_type: code
status: active
parent_spec: ../Breach — Reverse Tower Defense Design Spec.md
---

# Spec 37: The outcome trace

## Purpose

When a branch's digests differ from its base's, `check_outcomes.py` names the runs and the
first tick each differs, but nothing shows what changed. This round lets the owner see it
and sign it off: a trace of the ticks around the first difference, taken on the base and on
the branch, a text diff naming the first unit and field to differ, CI attaching both, and a
page that plays the two side by side.

**Settled with the user, 2026-10-17 (Q29).** No Decision: tooling that changes no outcome
and no rule. It relies on Decision 93 (a battle replays from its seed and orders) and Decision
130 (bit for bit across machines), and keeps Decision 97 (an id names a unit, never ranks
it). Gate 2 stays the owner's: the tool shows a change, and the owner judges it.

## Status

Agreed in the design session of 2026-10-17. Four steps, each its own PR, built with
`/run-phase` as "spec 37 step N". Steps 1 and 2 are useful on their own; step 4 is the page.

## Settled

- **The trace comes from the digest, not a second tool.** `tools/formation_digest.gd` gains
  `only=<run label>`, `trace=<file>`, `from=<tick>` and `to=<tick>`. The runs and the state
  are the digest's own (`_state`, `_unit_state`, the tick's events), so any change the digest
  sees, the trace holds. The digest's hashes don't change.
- **Named fields beside the arrays.** Lists of field names (`SQUAD_FIELDS`, `UNIT_FIELDS`)
  mirror the arrays `_state` and `_unit_state` build, nested as they nest. The trace zips each
  array with its names. A test fails when a list's length differs from its array's, so a field
  added to the digest without a name is caught.
- **A trace is JSON lines, one per tick in the window:** `{"run", "tick", "events",
  "squads": [{"id", <named fields>, "units": [{"id", <named fields>}, ...]}, ...]}`. Values
  are the digest's plain values (`_plain`: objects by id), vectors as `[x, y]`, floats
  written so they read back bit for bit (`%.17g`).
- **The window:** 20 ticks before the first difference to 40 after (`from` = first − 20,
  clamped to 1; `to` = first + 40, clamped to the run's end). The CI step passes them;
  the options take any window.
- **The engine:** GDScript, as `check_outcomes.py` compares. `engine=rust` works too, since
  the digest already takes it.
- **`ci/godot/scripts/trace_diff.py <base> <head> [--json <file>]`** matches squads and units
  by id (matching, never ordering: Decision 97) and prints:
  1. the first tick, squad, unit and field to differ, with both values (`tick 412  squad 3
     unit 57  hp: 18 -> 15`);
  2. the events of that tick that only one side has;
  3. a line per tick from there: how many units differ, and in which fields;
  4. units or squads only one side has.
  `--json` writes the same, per tick (each differing unit's id and fields), for the page.
  Exit 0 when the traces are the same, 1 when they differ.
- **CI attaches them.** `check_outcomes.py` gains `--changed <file>`, writing `<run>\t<tick>`
  per changed run. In `sim-checks`' `digests` job, when the file is non-empty, the first 3
  runs are traced on the base's worktree and on the branch, diffed, and added to the set's
  `sim-digests-<set>` artifact under `traces/`. The diff's first line goes to the `summary`
  job's output. A base that can't trace yet (before step 1 is on `main`) is said, not failed.
  This runs only when outcomes changed: about 6 runs of a few seconds each.
- **The page, not a Godot scene:** `tools/trace_view/` (static HTML and JS, no build, opened
  from disk like the designer). It loads a base trace, a head trace and the diff JSON, and
  shows two canvases with one scrubber, play, pause and step, each unit drawn as a dot with
  its facing, coloured by side, the differing ones ringed, and the selected unit's fields on
  both sides. It draws positions, not the skirmish layer's art (the user, 2026-10-17: a
  page, opened without Godot, is enough to sign off).

## Steps

1. **The trace.** The digest's options and named fields; `tests/tools/test_digest_trace.gd` (a new folder; check the suite's runner and shards find it).
2. **The diff.** `trace_diff.py` and `test_trace_diff.py` on fixture traces.
3. **CI attaches them.** `check_outcomes.py --changed` (tested in `test_check_outcomes.py`),
   and the `digests` job's trace step.
4. **The page.** `tools/trace_view/` with its logic in `trace_view.js` (loading, indexing by
   tick, which units to ring), tested from Python through Node as the designer's JS is
   (`test_trace_view_js.py`, skipped where Node isn't installed).

## Scenarios

### Step 1

**Scenario: every field has a name**
- Given the digest's `_state` for a mirror run's first tick
- When its arrays are compared with `SQUAD_FIELDS` and `UNIT_FIELDS`
- Then each array, at each depth, has as many entries as its list of names

**Scenario: the trace holds the window and nothing else**
- Given run `mirror_headon s1 rtrue`, `from=10`, `to=15`
- When the digest runs with `only` and `trace`
- Then the trace has six lines, ticks 10 to 15, each with that run's label

**Scenario: the trace says what the digest hashes**
- Given the same run and a tick in the window
- When the trace line's values are put back in the digest's order
- Then their `var_to_bytes` hash equals that tick's line in the digest's `out` file

**Scenario: the digest is unchanged**
- Given the branch with step 1
- When CI's `check_outcomes` compares its digests with the base's
- Then every set reads "same as the base, tick for tick" (no `Outcomes-changed:` trailer)

### Step 2

**Scenario: the first difference is named**
- Given two traces that agree to tick 411 and differ at tick 412 in unit 57's `hp` (18, 15)
  and unit 60's `position`
- When `trace_diff.py` runs
- Then its first line names tick 412, unit 57 (the first in the base trace's order), its
  squad, `hp`, 18 and 15, and it exits 1

**Scenario: one side's events are listed**
- Given tick 412's events differ by one `downed` event only the head has
- When `trace_diff.py` runs
- Then it lists that event under tick 412 as the head's

**Scenario: units are matched by id, not by place**
- Given the head trace lists the same units as the base in reverse order, every value equal
- When `trace_diff.py` runs
- Then it reports no difference and exits 0

**Scenario: a unit only one side has**
- Given unit 61 is in the base's tick 420 but not the head's
- When `trace_diff.py` runs
- Then it lists unit 61 as the base's only, from tick 420

**Scenario: the JSON for the page**
- Given the traces above and `--json out.json`
- When `trace_diff.py` runs
- Then `out.json` holds, per tick from 412, each differing unit's id and fields

### Step 3

**Scenario: the changed runs are written**
- Given base and head digests where runs `a` (from tick 7) and `b` (from tick 30) differ
- When `check_outcomes.py` runs with `--changed out.txt`
- Then `out.txt` reads `a\t7` and `b\t30`, in the head's order

**Scenario: nothing changed, nothing written**
- Given identical digests
- When `check_outcomes.py` runs with `--changed out.txt`
- Then `out.txt` is empty

**Scenario: CI attaches the traces (seen on a PR, not in a test)**
- Given a PR whose digests change, with an `Outcomes-changed:` trailer
- When `sim-checks` runs
- Then `sim-digests-<set>` holds `traces/<run>.base.jsonl`, `.head.jsonl`, `.diff.txt` and
  `.diff.json` for up to 3 runs, and the summary shows each diff's first line

### Step 4

**Scenario: the page indexes both traces by tick**
- Given fixture base and head traces of ticks 10 to 15
- When `trace_view.js` loads them
- Then each tick from 10 to 15 gives both sides' squads and units

**Scenario: the differing units are ringed**
- Given the diff JSON lists unit 57 at tick 12
- When the page asks which units to ring at tick 12, and at tick 11
- Then it gets unit 57 at tick 12, and none at tick 11

**Scenario: a tick only one trace has**
- Given the head's trace ends at tick 14 and the base's at 15
- When tick 15 is asked for
- Then the base side has its units and the head side is empty, without an error

## Test-first order

1. Step 1: the field-name and window cases red in `test_digest_trace.gd`, then the digest's
   options; the hash case last. Run that one file; CI runs the digests.
2. Step 2: `test_trace_diff.py` red on each scenario, then `trace_diff.py`.
3. Step 3: `test_check_outcomes.py` red on `--changed`, then the option; then the workflow
   step (seen on the PR's own run only if its digests change; otherwise on the next that does).
4. Step 4: `test_trace_view_js.py` red, then `trace_view.js`; then the page around it.

## Rubric answers

- **single-noun-phrase:** the outcome trace.
- **ocp-extension-point:** a field the digest adds gets a name in one list; the trace,
  the diff and the page pick it up.
- **lsp-contract-scope:** not applicable; no subtypes.
- **isp-fit:** the diff reads only traces; the page reads only traces and the diff JSON.
- **dip-direction:** the trace depends on the digest's state, and the diff and page on the
  trace's format, never on `sim/`.
- **general-over-special:** general. Any digest run, any window.
- **order-independent:** units and squads are matched by id; "first" means first in the base
  trace's order, the digest's own, and only orders a report.
- **bounded-work:** not on the tick. A trace is one replay of one run; CI traces at most 3
  runs a set, only when outcomes change.
