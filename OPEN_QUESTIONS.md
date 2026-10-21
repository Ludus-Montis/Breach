# Open questions

Design questions still open, and reminders the owner asked for, are GitHub issues labelled
[`question`](https://github.com/SSidey/Breach/issues?q=is%3Aissue+is%3Aopen+label%3Aquestion)
(Decision 148; `AI_First_Development_Kit/principles/agent-sessions.md`, rule 5). This file
keeps the entries closed before that, and a table from the old numbers to the issues.

**Raise one** when a question is raised and not settled in the same session: an issue
titled `Q<n>. <the question, short>` (the next number after the highest used, by an
issue or under Closed below; Q31 was next at the migration),
labelled `question` and one `raise-when:` label (`raise-when:next-design`,
`raise-when:spec-NN` with the spec's milestone, or `raise-when:other`), with this body:

```markdown
**Raised:** <date, where>
**Raise when:** <spec, feature, date, or next design session, in words>
**Possibly settled by:** <Decision N: why> (optional, for the owner to confirm)

<the detail>
```

**Check them** before starting work: the open `question` issues whose `raise-when:` label or
**Raise when** matches it.

**Close one:** only the owner does. A PR that settles a question says `Closes #N`, so the
owner's merge closes it. An agent may comment "Possibly settled by Decision N"; it never
closes one itself. `ci/github/check_open_questions.py` fails if an entry is added under Open.

## Open

Every open entry moved to an issue on 2026-10-21 (`github-sync`, `migrate`):

| Entry | Issue |
|---|---|
| Q1 | [#257](https://github.com/SSidey/Breach/issues/257) |
| Q2 | [#258](https://github.com/SSidey/Breach/issues/258) |
| Q3 | [#259](https://github.com/SSidey/Breach/issues/259) |
| Q4 | [#260](https://github.com/SSidey/Breach/issues/260) |
| Q5 | [#261](https://github.com/SSidey/Breach/issues/261) |
| Q6 | [#262](https://github.com/SSidey/Breach/issues/262) |
| Q7 | [#263](https://github.com/SSidey/Breach/issues/263) |
| Q8 | [#264](https://github.com/SSidey/Breach/issues/264) |
| Q9 | [#265](https://github.com/SSidey/Breach/issues/265) |
| Q10 | [#266](https://github.com/SSidey/Breach/issues/266) |
| Q11 | [#267](https://github.com/SSidey/Breach/issues/267) |
| Q12 | [#268](https://github.com/SSidey/Breach/issues/268) |
| Q13 | [#269](https://github.com/SSidey/Breach/issues/269) |
| Q14 | [#270](https://github.com/SSidey/Breach/issues/270) |
| Q15 | [#271](https://github.com/SSidey/Breach/issues/271) |
| Q16 | [#272](https://github.com/SSidey/Breach/issues/272) |
| Q17 | [#273](https://github.com/SSidey/Breach/issues/273) |
| Q18 | [#274](https://github.com/SSidey/Breach/issues/274) |
| Q19 | [#275](https://github.com/SSidey/Breach/issues/275) |
| Q21 | [#276](https://github.com/SSidey/Breach/issues/276) |
| Q22 | [#277](https://github.com/SSidey/Breach/issues/277) |
| Q23 | [#278](https://github.com/SSidey/Breach/issues/278) |
| Q24 | [#279](https://github.com/SSidey/Breach/issues/279) |
| Q25 | [#280](https://github.com/SSidey/Breach/issues/280) |
| Q26 | [#281](https://github.com/SSidey/Breach/issues/281) |

## Closed

- Q30, track open questions and upcoming work on GitHub. **Resolved by Decision 148 (2026-10-17):** open questions become issues labelled `question` (a `raise-when:` label each); a PR names its spec as a milestone and its step as a `step:<spec>-<step>` label (`step:35-7b`); Decisions, specs, their steps and hand-offs stay in the repo; a `github-sync` workflow makes the labels and milestones and migrates the entries; the Project is the owner's. Built as spec 36.
- Q29, a side-by-side view of an outcome change. **Resolved (2026-10-17), settled with the owner as spec 37, no Decision (tooling only):** the digest writes a named trace of 20 ticks before to 40 after a run's first difference; `trace_diff.py` names the first unit and field to differ; CI traces up to 3 changed runs a set on base and branch and attaches them; a page in `tools/trace_view/` plays the two side by side.
- Q28, one typed view onto the unit store for every pass. **Resolved by Decision 146 (2026-10-16):** a pass holds the columns it names as one typed view (`UnitView`), replacing `FleeColumns`, `UnitHealth.Table` and `UnitPlace.table`; a scattered mark gives `few:` or `reference:` only; no hold spans a native call; built as spec 35 steps 6a and 6b, before stamina (6c).
- Q27, whether one bench run can tell a 5% change from noise. **Resolved (2026-10-15):** it can't; `bench` takes `rounds` and alternates head and base, comparing each phase's medians and ranges ([SSidey/Breach#236](https://github.com/SSidey/Breach/pull/236)). Its first use settled #234: the tick 1.5% faster, deaths 31% faster (spec 35, "Built → Step 5b").

- Q20, the native field owning unit state so the glue goes. **Resolved by Decision 142 (2026-10-14):** a unit store owns every unit's per-tick state as columns, with GDScript and Rust backends, built in steps as spec 35.
- Whether an authored `MapEdgeDef`/link is a hard constraint (the only route that exists) or a soft default a future pathfinding/movement-cost system could override — raised alongside the terrain-resistance idea above. Not designed; depends on that idea's outcome. **Resolved by Decision 26 (2026-09-27):** both. An authored link is a hard constraint on *which* nodes connect, and its route *geometry* is pathfound over terrain cost.
- `NodeDef.structure_slots` semantics (raised while iterating on the tile designer prototype): per Decision 4's original framing this is a *count* of discrete buildable foundations at a location ("room for 2 towers here"), not a physical size/area value — but nothing consumes it yet, so this is genuinely underspecified. A real gap raised directly: if `structure_slots: 0` means "nothing permanent can be built here" (used in the prototype as a pure routing waypoint), can a barricade or other lightweight stationary defense still go there? A barricade may need to be a different, non-slot-consuming category entirely rather than something gated by `structure_slots`. Not designed — belongs to the still-unbuilt spatial-placement pass (Decision 4's Option 2). See "Future direction: layered tile model" below, which treats a barricade as a 0-floor structure and asks whether *footprint* should replace or redefine `structure_slots`. **Resolved by Decision 25 (2026-09-27):** superseded by the tile capacity + segment-profile model, and the field has been removed (`specs/14-structure-slots-removal.md`).
