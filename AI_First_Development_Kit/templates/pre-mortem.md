---
doc: templates/pre-mortem
status: active
---

# Pre-mortem Template

A design round opens with a pre-mortem (`principles/agent-sessions.md`, §3). Before any
options are discussed, the design session asks the owner, or answers with them:

1. **What will we regret in a month?** Imagine the round shipped and went wrong. What
   went wrong?
2. **What is the riskiest assumption?** The one belief that, if false, sinks the design.
3. **What is the cheapest test of it?** The smallest trial, measurement, prototype or
   feel test that would show it false before much is built on it.

Copy the block below into the round's specification, after its purpose. Write it once
per round, at the start, and leave it as written: if the test later proves the
assumption wrong, the next round records that and opens with its own pre-mortem.

```markdown
## Pre-mortem

- **Regret in a month:** <the most likely way this round goes wrong>
- **Riskiest assumption:** <the belief the design most depends on>
- **Cheapest test:** <the test> — <scheduled as Step N | already answered by <what>>
```

The spec-baseline rubric's `pre-mortem` row checks that the section is present and that
its test is either scheduled or already answered.
