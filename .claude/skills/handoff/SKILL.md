---
name: handoff
description: "Hand Breach work between agents (a local agent on the user's PC and cloud agents) through GitHub, so nothing is lost when a device turns off. Use when asked to hand off, wrap up, pause, park or save work for later ('I'm turning off my PC', 'hand this to a cloud agent', 'leave notes'), and when picking up or continuing work someone else started ('continue PR #N', 'pick up the local agent's branch', 'what's in flight?'), or when invoked as /handoff."
---

# Handing work between agents

Agents on the user's PC and in the cloud can't see each other's sessions, terminals or
uncommitted files. **GitHub is the only shared memory:** a hand-off exists only once it
is pushed, and a claim to have started another agent is real only if it comes with that
session's link. Work stops mid-change whenever a device turns off, so a hand-off must be
cheap enough to do every time.

## Leaving: hand off before you stop

Do all of these, in order, whenever you stop with work unfinished, or when the user says
they're leaving.

1. **Commit everything on your branch,** finished or not. There is no `wip:` type:
   unfinished work takes the type that matches its diff (per
   `AI_First_Development_Kit/templates/commit-message.md`), and its body says what is
   incomplete. Never leave work only in the working tree or a stash.
2. **Push the branch:** `git push -u origin <branch>`.
3. **Make sure a pull request exists.** If the work isn't ready for review, open it as a
   **draft**. A draft is a hand-off vehicle, not a request for review: mark it ready only
   when Gate 1 passes (`run-phase` step 5). A parked draft gets the `handoff` label, in
   the same `issue_write` that sets its other labels (`run-phase` section 6), and still
   says `Closes #N` for its step's issue, so the Project shows it under **Next**.
4. **Write or update the `## Handoff` section** at the top of the PR description, using
   the template below. Replace it each time; it describes now, not history.
5. **Move open questions to the register.** Anything still undecided, and any "remind
   me when…", becomes an issue (Decision 148): `issue_write` with `method: create`, title
   `Q<n>. <the question>`, labels `question` and one `raise-when:` label, and the body
   shape in `OPEN_QUESTIONS.md`'s header, with its **Raise when** trigger. The PR's
   *Open questions* line lists those issue numbers rather than holding the only copy.
6. **Tell the user** the PR link and one line on what the next agent should do first. If
   you started another agent, give its session link (see "Starting another agent").

### Starting another agent

A started agent counts only if you can give the user its claude.ai session link. If
starting one returns no link (a background subagent, or a "remote" request that came
back without an address), **it is running on this device and stops when the device
does.** Say exactly that. Never tell the user the work will survive a shutdown without
a link to show for it. Then push the plan as a draft PR with a `## Handoff` section, so
a cloud session the user starts can pick it up.

### Design sessions hand off through the spec

A design session (`AI_First_Development_Kit/principles/agent-sessions.md`, rule 3) hands
off by pushing its outcome as Decisions and a spec round in a `docs:` PR, with the
`## Handoff` section's **Next** naming the `/run-phase` item for a fresh implementation
session. The implementation session starts from the spec, not from the discussion. The
PR says `Closes #N` for each `question` issue it settles. It sets no milestone:
`github-sync` gives the PR and those issues the spec's milestone, and makes the spec's
step issues once the PR merges.

### The `## Handoff` template

```markdown
## Handoff

**State:** <done | in progress | blocked> as of <YYYY-MM-DD HH:MM, timezone>, by <local agent | cloud agent>
**Branch:** `<branch>` (stacked on `<base branch>`), last commit `<short sha>`

**Done**
- <what landed, by commit or file>

**Next**
1. <the very next step, concrete enough to start without reading the whole PR>
2. <then this>

**How to check it**
- <test command, scripted run, or designer view to open, and what passing looks like>

**Open questions for the user**
- <`question` issues this work raised or depends on, as #N; none if none>

**Gotchas**
- <anything surprising: a check that is red and why, a file not to touch, an environment need>
```

Keep it short: a few lines per heading. Facts that matter beyond this PR belong in a
Decision or the spec, not here.

## Picking up: resume someone else's work

1. **Fetch first:** `git fetch origin`. Never start from a stale branch.
2. **Find what is in flight:**
   - open pull requests, and any with a `## Handoff` section
   - remote branches ahead of `main` that have no pull request (pushed but never handed
     off; read their last commit messages)
   - the stack: which branch each open PR is based on
3. **Read the hand-off,** then the PR's commits and spec, before changing anything.
   Read Decisions through `DECISIONS_INDEX.md`, not the whole design spec.
4. **Check out the branch and pull:** `git checkout <branch> && git pull`. Run the tests
   once, to confirm the state the hand-off describes.
5. **Claim it:** update the `## Handoff` section's State line to say you're working on it,
   with the time, and push that edit (the PR description, not a commit). Before your first
   push, remove the `handoff` label: `issue_write` with `method: update` on the PR's number
   and `labels` set to its labels less `handoff`.
6. Carry on with `run-phase` from the hand-off's **Next** step.

## Rules

- **One agent per branch at a time.** Before pushing to a branch another agent used,
  fetch and check its last commit time and hand-off State. If someone else is mid-change,
  stack a new branch on theirs instead of pushing to it.
- **Never rewrite pushed history** on a branch another agent may hold: no rebase, amend
  or force-push after a hand-off. Merge instead.
- **Pull before you push,** every time; resolve conflicts by merging.
- **A device-dependent step** (a local build, the user's own Godot editor, a file only on
  their PC) is written under **Gotchas** as needing the local agent or the user. A cloud
  agent doesn't pretend to have done it.
- **The user's hand edits** found uncommitted are theirs: commit them as they are, in a
  separate `chore:` commit that says so, before your own changes.
