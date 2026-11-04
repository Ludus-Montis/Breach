---
doc: templates/cascade-issue
status: active
applies_to: the one record a cascade keeps (principles/agent-sessions.md, rule 8)
---

# Template: a cascade's record

A cascade runs a series of steps, one fresh session each (`principles/agent-sessions.md`,
rule 8). Its whole state lives in **one record**, the body of an issue (or a file, on a
tracker without issues), **edited in place** by each step. The record is the standing
instructions: a step session reads it whole and needs nothing else. Nobody writes "apply
comment A, then B's list, then these changes": that makes every later session rebuild the
instructions from layers, which costs a dozen reads and invites mistakes.

- The **body** is now: replace a line when it changes, delete what no longer holds.
- The **comments** are the log: one short comment per step, never instructions.
- **Standing rules** change only when the owner says so; a step session copies them
  unchanged and records who changed one and when.
- Read the body again just before writing it, and write it whole. Only one step runs at a
  time, so there is no other writer but the coordinator, which touches only its own line.

Copy the block below into the issue's body.

```markdown
## Cascade: <subject, e.g. Spec NN: title>

**State:** <running | blocked on <what> | stopped | done> as of <YYYY-MM-DD HH:MM, timezone>
**Coordinator:** <the coordinating session's link and id, or "none (chain mode)">
**Next:** `<the exact command the next step session runs>`
**Base:** `<branch the next step branches from>` (PR #<n>, unmerged; or `main`)
**Stop when:** <the step whose PR ends the cascade, or "the spec's last step">

## Standing rules

From the owner, <date>. Hand them on unchanged; only the owner changes one.

1. **Stacking:** <each step branches from the previous step's branch, unmerged | each
   step waits for the previous PR to merge>. Nothing is merged by an agent.
2. **Splitting:** <a step too big for one session may be split, recorded in the spec,
   without asking | ask first>.
3. **Checkpoints:** <pause for the owner to review and merge after every N PRs | none>.
4. **Allowed without asking:** <e.g. rebuilding the native library>.
5. <any rule the owner added>

## Chain

| Step | PR | Branch | Session |
|---|---|---|---|
| <id> | #<n> | `<branch>` | <link> |

## Still to come

- <step id>: <one line on what it is, and any split already recorded>

## Gotchas

Curated: add what the next step needs, delete what no longer holds. Facts that outlive the
cascade go into the spec, a procedure or a Decision instead.

- <a surprise the next step would otherwise rediscover>
```
