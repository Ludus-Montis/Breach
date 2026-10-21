# Documentation

Organised by [Diátaxis](https://diataxis.fr). Every page under `docs/` must be linked from
here (`AI_First_Development_Kit/scripts/check-docs.sh docs`). A document that lives next to
what it describes stays there and is linked from its quadrant below.

## Tutorials
Learning-oriented. Guided first runs.
- (none yet)

## How-to guides
Task-oriented. Steps to achieve a specific goal.
- [Design a map and import it](../tools/designer/README.md)
- [Build the Rust core](../native/README.md#building)
- [Run the checks locally](../ci/godot/README.md#running-the-checks-locally)
- [Replace the placeholder art](../assets/placeholder/README.md)
- [Raise, check or close an open question](../OPEN_QUESTIONS.md) (GitHub issues labelled
  `question`; labels and milestones come from the `github-sync` workflow)

## Reference
Information-oriented. Precise descriptions of APIs, config, formats.
- [CI checks: each rubric row and the tool that checks it](../ci/godot/README.md)
- [Rust core: what it owns and how it is loaded](../native/README.md)
- [Specs, one per feature or round](../specs/)
- [The outcome trace: the digest's per-tick JSON lines](../specs/37-outcome-trace.md#settled)

## Explanation
Understanding-oriented. Background and design decisions.
- [Design spec](../Breach%20—%20Reverse%20Tower%20Defense%20Design%20Spec.md), with its
  addenda at the project root (`breach-addendum-*.md`)
- [Decisions index](../DECISIONS_INDEX.md): the Decision ledger is this project's
  architecture decision record
- [AI-First Development Kit](../AI_First_Development_Kit/README.md): how work is done
