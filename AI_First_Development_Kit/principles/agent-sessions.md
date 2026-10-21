---
doc: principles/agent-sessions
status: active
applies_to: every agent session that works on the project, local or remote
---

# Agent Sessions: What Every Agent Knows, What Each One Holds, and How Work Passes Between Them

## Why this exists

`principles/agent-workflow.md` says how one change lands. It doesn't say how many agents,
across many sessions and machines, keep working the same way. Long-running projects hit
the same failures again and again:

- **Rules that live only in one session.** A preference the owner stated, or a reminder
  they asked for, gets saved to one agent's private memory or conversation. The next agent,
  on another machine or in a fresh session, never sees it. So the owner either repeats
  themselves or stops trusting a hand-over.
- **One session doing everything.** Design discussion and implementation share a session
  for days. Every message re-sends the whole conversation, so the cost grows with its
  length, and most of that length is test output, diffs and logs the discussion never
  needed. Context resets then lose the thread of the discussion anyway.
- **Hand-overs that weren't.** An agent reports that another agent has taken the work. In
  fact that agent ran on the same device and stopped when the device did.
- **Questions that get lost.** Open questions and "remind me when…" requests sit in a
  conversation and disappear when it resets or ends.

The rules below are tool-agnostic. Each names the place a fact must live so that the
next agent finds it without being told.

## 1. One entry file, loaded by every session

The project keeps **one short file that the agent tooling loads automatically at the
start of every session**. Its name depends on the tooling, for example `CLAUDE.md` or
`AGENTS.md`. Procedures (skills, custom commands) are not enough on their own: they
load only when a request happens to trigger them, so an agent that never triggers one
never learns its rules.

The entry file holds:

- the project's non-negotiables, one line each (branching and who merges, test-first,
  how Decisions are recorded, how hand-overs work)
- which role the session is in (rule 3) and what that role reads first
- where things live: specs, the Decision ledger and its index, the open-questions
  register, the procedures

It **points; it does not copy**. A rule's full text lives in one place (this kit, a
procedure or a Decision), and the entry file says where. Keep it to about a page: every
session pays for every line, every time.

## 2. Nothing that governs work lives only in an agent's private memory

Anything learned in a session that should change how *any* agent works on the project
is written into the repository before the session ends:

| What was learned | Where it goes |
|---|---|
| A standing rule or preference from the owner | The entry file, or the procedure it belongs to |
| A design or process choice | A Decision (`principles/decision-ledger.md`) |
| A question still open, or a "remind me when…" | The open-questions register (rule 5) |
| State of unfinished work | The hand-over (rule 6) |

An agent's private memory (per-machine notes, the conversation itself) is for things
that matter only to that agent on that machine. If losing it would make the next agent
work differently, it was in the wrong place.

## 3. Design and implementation are separate sessions

| | Design session | Implementation session |
|---|---|---|
| Works with the owner on | Discussion, options, decisions | Only gaps the spec leaves open |
| Writes | Decisions, spec rounds, register entries | Code, tests, proposals (via the `agent-workflow.md` procedure) |
| Hands over through | The spec and its Decisions, pushed | The proposal and its hand-over |
| Lives for | One topic or spec; ended when it grows long | One spec round; then discarded |
| Never carries | Test output, diffs, build logs | The design discussion |

- **The spec is the hand-over.** A design session ends a round by writing its outcome
  as spec scenarios and Decisions, not as a summary of the discussion. The
  implementation session starts fresh from that, so the debate that led to a settled
  choice never reaches it, and nothing settled gets reopened by accident.
- **An implementation session acts on what the spec settles.** It doesn't re-plan
  or re-ask settled choices. It stops and asks only where the spec is silent or
  ambiguous, and the answer goes back into the spec or a Decision, not just into its
  own conversation.
- **Feedback routes by kind.** Feel and design feedback ("this should behave
  differently") goes to a design session and becomes the next spec round. A defect
  against the spec as written ("this doesn't do what the spec says") can go straight
  to an implementation session.
- **Design sessions end too.** When one grows long, it ends. The spec and the register
  are its memory, and the next design session starts from them.
- **One step per session, and every step ends by naming the next.** An implementation
  session builds one step (one spec round, or one numbered step of a series) and stops.
  Its last message gives the exact command that starts the next step in a fresh
  session. If the next work isn't in a spec yet, it says a design session comes next
  and names its subject. Nobody has to work out where to pick up.

## 4. Context is a budget

Every message re-sends the session's whole context, so whatever an agent reads it pays
for again on every later turn. On a metered plan this is the owner's weekly allowance:
a session that reads large outputs or runs long spends it many times over.

- **A large ledger or spec gets a generated index**: one line per entry, with where
  it is and what supersedes it. Agents read the index, then only the entries they need,
  by location. The index is generated by a script and checked in the same gate as
  everything else, so it can't drift.
- **Long output is summarised, not kept.** Read the failing part of a log, not all of it.
  Show only the failures of a test run. Don't echo whole files, diffs or listings to
  look at them; search for what you need.
- **CI is the test runner.** A session runs only the test it is writing, red then green,
  and the fast static checks. Every other test (the rest of the files a change touches,
  the full suite, determinism digests, benchmarks) runs in CI, which costs no context.
  When CI fails, the session reads the failing test's part of the log and runs that one
  test file. A session never runs a long test itself, and never polls CI.
- **A session that has been reset (its context summarised) more than once has
  outgrown its job.** Hand over and start fresh rather than carrying on.

## 5. One register for open questions

The project keeps **one open-questions register file**. Each entry has:

- **the question**, and **where and when it was raised**
- **raise when**: the trigger for bringing it back to the owner (a spec, a feature, a
  date, or "next design session"). This is how a "remind me when…" is kept.
- **status**: open, or closed with a link to the Decision that answered it

An agent adds an entry when a question is raised and not settled in the same session.
It checks entries whose trigger matches the work it is starting. It may **propose**
closing an entry it thinks a Decision has settled, but only the owner closes one, for
the same reason only a human supersedes a Decision.

The register may be an issue tracker instead of a file, if each entry keeps the same
fields (the trigger as a label, its words in the body) and only the owner closes one. A
change that settles an entry names it (`Closes #N`), so the owner's merge closes it and
the answer links to the change. Breach does this (Decision 148).

## 6. Hand-overs go through the shared remote

Agents on different machines can't see each other's sessions, terminals or uncommitted
files, so the shared remote is the only shared memory. A hand-over exists only once it
is pushed: commits, a proposal (a draft if unfinished), and a hand-over section saying
what's done, what's next and what's open.

**A claim that another agent has taken the work is real only if it comes with that
session's link** (or equivalent address on the remote service). If starting an agent
returns no link, the agent is running locally and stops when the device does. Say so
plainly, and push the hand-over so a remote session can pick it up.

## 7. One writer per working copy

An agent switching branches in a working copy that the owner's own tools also write to
(an editor, a content tool, a build) will collide with them: files changed under it,
checkouts blocked by uncommitted output. Each agent session gets its own working copy
(a worktree or clone). Where that isn't possible, the owner's uncommitted output is
committed as theirs before any branch switch.

## Mechanical vs procedural

- **Mechanical:** the ledger index (rule 4) is generated and checked by the gate.
- **Procedural:** the rest is enforced by being in the entry file, which every session
  loads. That makes the rules impossible to miss, not impossible to break. The gate's
  mechanical checks remain the backstop for anything that reaches a proposal.

## What this document deliberately leaves open

The entry file's name, the session-hosting service, and how a remote session is
started all depend on the tooling, and are out of scope per this kit's README. What
isn't open is where each kind of fact lives, and that design and implementation don't
share a session.
