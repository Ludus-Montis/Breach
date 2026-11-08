# Documentation

Organised by [Diátaxis](https://diataxis.fr). Every page under `docs/` must be linked from
here (`AI_First_Development_Kit/scripts/check-docs.sh docs`). A document that lives next to
what it describes stays there and is linked from its quadrant below.

## Tutorials
Learning-oriented. Guided first runs.
- (none yet)

## How-to guides
Task-oriented. Steps to achieve a specific goal.
- [Design a map and save it](../tools/designer/README.md)
- [Build the Rust core](../native/README.md#building)
- [Run the checks locally](../ci/godot/README.md#running-the-checks-locally)
- [See an outcome change side by side](../tools/trace_view/README.md)
- [Replace the placeholder art](../assets/placeholder/README.md)
- [Run a spec's steps back to back, one session each](../.claude/skills/cascade/SKILL.md)
  (`/cascade`; why it works this way: [agent sessions, rule 8](../AI_First_Development_Kit/principles/agent-sessions.md#8-a-long-series-of-steps-runs-as-a-cascade))
- [Open a design round with a pre-mortem](../AI_First_Development_Kit/templates/pre-mortem.md)
  (why: [agent sessions, rule 3](../AI_First_Development_Kit/principles/agent-sessions.md#3-design-and-implementation-are-separate-sessions), Decision 168)
- [Archive the bench and sim-checks logs before they expire](../ci/metrics/README.md)
- [The metrics records, their schema and check, and data-only PRs](../ci/metrics/README.md#the-records-and-their-check)
- [The data-PR route: measuring workflows merge their own data PRs](../ci/metrics/README.md#the-data-pr-route)
- [The bench history: a record for each merge that touches the simulation](../ci/metrics/README.md#bench-records)
- [The bench history's backfill, from 2026-10-14](../ci/metrics/README.md#the-backfill)
- [Harvest the archived logs into records](../ci/metrics/README.md#harvest-the-archived-logs)
- [The project site: its tabs, how one is added and how it is published](../tools/site/README.md)
- [The tick history page, on the web or from disk](../tools/metrics_view/README.md)
- [Raise, check or close an open question](../OPEN_QUESTIONS.md) (GitHub issues labelled
  `question`; labels, milestones and step issues come from the `github-sync` workflow)

## Reference
Information-oriented. Precise descriptions of APIs, config, formats.
- [CI checks: each rubric row and the tool that checks it](../ci/godot/README.md)
- [Rust core: what it owns and how it is loaded](../native/README.md)
- [The content schema: units, weapons, armour, tuning, the terrain library and maps as JSON](../content/schema/README.md)
- [The GitHub sync: labels, milestones and step issues](../ci/github/README.md),
  [the Project's Status per PR](../ci/github/README.md#the-projects-status),
  and [the Project backfill](../ci/github/README.md#the-project-backfill)
- [The showcase's scenarios: the list, its records and how each behaviour is read](../content/scenarios/README.md)
- [The showcase capture: which PRs, what it makes, where it is kept, the PR comment](../ci/showcase/README.md)
- [Specs, one per feature or round](../specs/)
- [The outcome trace: the digest's per-tick JSON lines](../specs/37-outcome-trace.md#settled)

## Explanation
Understanding-oriented. Background and design decisions.
- [Design spec](../Breach%20—%20Reverse%20Tower%20Defense%20Design%20Spec.md), with its
  addenda at the project root (`breach-addendum-*.md`)
- [Decisions index](../DECISIONS_INDEX.md): the Decision ledger is this project's
  architecture decision record
- [AI-First Development Kit](../AI_First_Development_Kit/README.md): how work is done
