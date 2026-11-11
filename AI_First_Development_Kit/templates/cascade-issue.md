---
doc: templates/cascade-issue
status: active
applies_to: the one record a cascade keeps (principles/agent-sessions.md, rule 8)
---

# Template: a cascade's record

A cascade runs a series of steps, one fresh session each, every ready step at once
(`principles/agent-sessions.md`, rule 8). Its whole state lives in **one record**, the body
of an issue (or a file, on a tracker without issues), **edited in place**. The record is
the standing instructions: a step session reads it whole and needs nothing else. Nobody
writes "apply comment A, then B's list, then these changes": that makes every later
session rebuild the instructions from layers, which costs a dozen reads and invites
mistakes.

- The **body** is now: replace a line when it changes, delete what no longer holds.
- **The coordinator is the only writer of the body.** Several steps run at once, so a step
  session never edits it: it writes only its own comment and reports to the coordinator,
  which moves the step from Running to Done and starts what has become ready.
- The **comments** are the log: one short comment per step, never instructions.
- **Standing rules** change only when the owner says so; the coordinator copies them
  unchanged and records who changed one and when.
- A step is **ready** when every step in its `Needs` has a proposal (or is merged, when the
  cascade waits for merges) and its `Touches` doesn't overlap a running step's. Files every
  step edits (the changelog, the README, the docs index and the spec) don't count.

Copy the block below into the issue's body.

```markdown
## Cascade: <subject, e.g. Spec NN: title>

**State:** <running | blocked on <what> | stopped | done> as of <YYYY-MM-DD HH:MM, timezone>
**Coordinator:** <the coordinating session's link and id, or "none (chain mode)">
**Stop when:** <the step whose PR ends the cascade, or "the spec's last step">

## Ready

Steps whose needs are met and whose files are free; the coordinator starts each at once.

| Step | Command | Base |
|---|---|---|
| <id> | `<the exact command its session runs>` | `<main, or the one unmerged step it needs (PR #<n>)>` |

## Running

| Step | Base | Session | Fallback check | Touches outside its line |
|---|---|---|---|---|
| <id> | `<branch it branched from>` | <link> | <check id> | <paths it reported, or "none"> |

## Done

| Step | PR | Branch | Session |
|---|---|---|---|
| <id> | #<n> | `<branch>` | <link> |

## Standing rules

From the owner, <date>. Hand them on unchanged; only the owner changes one.

1. **Stacking:** <a step branches from the one unmerged step it needs | each step waits
   for the steps it needs to merge>. Nothing is merged by an agent.
2. **Splitting:** allowed, signposted (the default): a step too big for one session is
   split without asking, and the split is recorded in the spec, the old step's issue, the
   PR's `Split:` line and this record. <Or, if the owner says so: ask first.> A step
   session writes only its log comment, never this body.
3. **Checkpoints:** <pause for the owner to review and merge after every N PRs | none>.
4. **Allowed without asking:** <e.g. rebuilding the native library>.
5. <any rule the owner added>

## Still to come

- <step id>: <one line on what it is>; needs <ids>; <any split, e.g. "split at build from 3, PR #n">

## Gotchas

Curated: add what the next step needs, delete what no longer holds. Facts that outlive the
cascade go into the spec, a procedure or a Decision instead.

- <a surprise the next step would otherwise rediscover>
```
