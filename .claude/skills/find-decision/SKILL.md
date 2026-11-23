---
name: find-decision
description: "Find the Breach Decisions that settle a topic and return their numbers, lines and one-line rulings, without loading the 400 KB design spec into the conversation. Use when asked which Decision covers something ('is there a Decision on tie-breaks?', 'what did we decide about morale?'), before proposing a change that may contradict one, or when invoked as /find-decision."
argument-hint: "[topic words]"
allowed-tools: Read, Grep, Glob
# STUDY 3.2 (TODO): two lines. Run this skill in a separate subagent that cannot see the
# conversation and returns only its answer, and make that subagent the built-in read-only
# explorer. (One field says "separate context", the other names which agent.)
---

# Find the Decisions on a topic

The topic is: **$ARGUMENTS**

You start with no conversation history: everything you need is in this file and the repo.

1. Search `DECISIONS_INDEX.md` for the topic's words and close synonyms (Grep, case-insensitive).
   Each hit gives a Decision's number, title and line in
   `Breach — Reverse Tower Defense Design Spec.md`.
2. For at most the five best hits, read the Decision from its line to the next
   `### Decision` heading (Read with offset and limit). Never read the whole spec.
3. Check each for a later amendment or a "Superseded by" line, and follow it.

Return only this, one row per Decision that bears on the topic, best first:

| Decision | Line | Ruling in one line | Status |
|---|---|---|---|

Status is `in force`, `amended by N` or `superseded by N`. If nothing bears on the
topic, say so in one line and name the closest Decision you rejected.
