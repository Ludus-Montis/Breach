---
doc: templates/pr-description
status: active
---

# PR Description Template

```markdown
## Summary

<one or two sentences: what changed and why>

## Specification

<link to the spec this PR implements, if any>

## Run-baseline rubric

- [ ] `tests-red-then-green` — red→green log attached or linked
- [ ] `contract-tests-pass` (if applicable)
- [ ] `coverage-overall` ≥ threshold
- [ ] `coverage-changed-lines` ≥ threshold
- [ ] `lint-clean`
- [ ] `srp-size` / `ocp-shotgun-surgery` / `isp-method-count` / `isp-stub-detection` /
      `dip-direction` / `naming-grep-discoverable` / `no-cross-cutting-helper-violation`
- [ ] `progress-trend` — no regression vs. baseline (see attached log entry)
- [ ] `commit-message-conforms` / `branch-name-conforms`

CI reports these automatically; this checklist is for the human reviewer's visibility,
not a manual re-check.

### Documentation

<per `principles/documentation-phases.md`; "N/A" needs a reason. "Changelog: N/A" also
tells the CI docs gate to skip its changelog check>

- README: updated | verified, no change | N/A (reason)
- Changelog: entry added under [Unreleased] | N/A (reason)
- Docs: <paths updated/created> | N/A (reason)

## Progress log entry

<link to the row the project's Gate 1 script appended to the durable progress log, per
`principles/progress-tracking.md`'s recommended mechanism — quote it inline too if the
reviewer shouldn't have to open another file>

## Human qualitative review (Gate 2)

<link to the entry in the project's durable Gate 2 log (see
`principles/progress-tracking.md`, "Where to record it") rather than filling
Reviewer/Verdict/Notes inline here — leave that entry's content for the human reviewer to
write, never the implementing agent>

- **Reviewer:**
- **Verdict:** <does this feel like genuine progress, independent of the numbers above?>
- **Notes:**

## Decisions

<link to any new or superseding Decision entries this PR introduces>
```
