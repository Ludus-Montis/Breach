# Spec 33: Base stats and their derivations

Draft, for a future piece: a review of every base stat a unit carries, what derives from
it in the code today, what Decisions 117 to 128 promise but haven't built, and where the
code is inconsistent. The user: "it would be good to run through that as a future
piece". It builds on spec 28 (unit levers) and Decisions 117 (attributes and derived
stats), 120 (load), 121 and 124 to 127 (wounds), and 122 (pools and statuses).

**Status:** review only, to be worked through with the user. The code it describes is at
`feature/unit-levers-registry` (with `feature/movement-walk` for walking).

- **Average** means the unit has no authored value, so the attribute is 10.
- Tunable values come from `content/tuning/battle_tuning.tres`.
- File paths are under `sim/skirmish/formation/` unless shown in full.

## Attributes

### Strength

Only the brute authors a value (14).

| Derived | Formula | Status |
|---|---|---|
| Carry limit | strength × 2 (`load_per_strength`) | built |
| Load stage 0–3 | The stages are: under 0.5× the limit; up to the limit; up to 2× (+0.5× per `hauler` level); beyond | built |
| From the load stage | Pace × [1, 0.9, 0.6, 0], tiring × [1, 1.25, 2, 2], dodge × [1, 0.85, 0.5, 0], and a march stamina cost per second | built |
| Re-staged while carrying a body | Body weight is its footprint area × 15 | built |
| Weapon damage | damage + scaling × max(0, strength − the weapon's requirement) | built |
| Wield limit | −2 skill per point of strength short of a weapon's requirement | built |
| Shoving | Mass is footprint area only | promised (Decision 117), not built |

### Agility

No unit authors a value; all are average.

| Derived | Formula | Status |
|---|---|---|
| Dodge | agility × 1.0 × the dodge factor for the load stage; none from the rear | built |
| Attack speed, turn rate, initiative | Authored directly, not derived | promised (Decision 117), not built |

### Constitution

No unit authors a value; all are average. Hardiness = constitution / 10.

| Derived | Formula | Status |
|---|---|---|
| Max stamina | constitution × 10 | built |
| Breather delay | 2 s ÷ hardiness | built |
| Stamina recovery | 3/s × hardiness × condition (capped at 1.25) | built |
| Death's door | dies at HP ≤ −constitution | built |
| Wake (come-to) time | 30 s × (0.5–1.5 roll) ÷ hardiness | built |
| Bonus HP, physical maladies | — | promised (Decisions 117, 122), not built |

Stamina thresholds:
- **Tired** (below 50% of max): pace × 0.85, skill −10.
- **Spent** (below 15%): pace × 0.6, skill −25, can't run.
- **Gives up a chase:** below 25%.

### Willpower

It is never read anywhere. Its promised uses are fear, courage, ward and magical maladies, and none of them is built.

### Wits

No unit authors a value; all are average.

| Derived | Formula | Status |
|---|---|---|
| Pursuer reaction delay | 1.0 s × 10 ÷ wits | built (Decision 126) |
| Play-dead chance | 0.4 × wits/10 + 0.25 per level of `cunning` | built (Decision 127) |
| Taken for dead | (the body's wits + 10 × `cunning`) − (the taker's wits + 10 × `thorough`), ± a die of 20 | built (Decision 127) |
| Pathfinder (best wits in the group) | — | agreed for spec 30 round 3, not built |
| Initiative, perception, casting, mana | — | promised, not built |

## Other authored stats

| Stat | What reads it | Status |
|---|---|---|
| **hp** | max HP; the regeneration limit is a share of it; downed at 0 | built |
| **speed** | Pace through the chain load stage → gait (run build-up) → stamina stage. A squad goes at its slowest unit; also the scrum's tie-break | built |
| **run_pace** | Gait builds from 1 to run_pace over 1.5 s | built |
| **courage** | Morale ceiling = mean courage + 10 × best leadership. Surrender chance 0.5 × (1 − courage/100) | built raw. From willpower: not built |
| **leadership** | +10 morale ceiling and +10 discipline per point; morale recovery; the shock when a leader falls. Being a leader gates rallying, catching routers, messengers, and leaders' cunning and march traits | built |
| **discipline** | The formation's discipline = mean discipline + 10 × leadership. It feeds re-forming as a whole (at 50+), re-form pace, the pursuit leash steps, the chance to break ranks, withdrawal disorder, and the walking slack | built; nobody authors it, so every unit has 30 |
| **initiative** | The scrum's contest key (a roll + initiative) | built raw; promised from agility and wits |
| **detection_range** | Sight between squads, chase limits, joining a formation when coming to | built raw; perception not built |
| **height** | Wading and depth bands on terrain | built |
| **turn_rate, backward_pace** | Turning; pace by facing | built raw |
| **melee/ranged skill** | Effective skill = skill ± proficiency − strength shortfall. It feeds the blow margin, parry (25% of melee skill), and the damage floor and spill | built |
| **defence** | Width of the graze layer | built raw |
| **critical** | Crit multiplier | built raw |
| **armour / ward** | Flat reduction (own + armour items) | built; ward from willpower not built |
| **attack_speed** | Blow interval = slowest weapon ÷ attack speed | built raw |
| **weak / resist / immune** | Damage × 1.5 / 0.5 / 0 | built |
| **regeneration** (+ limit, stops) | HP/s × condition, up to the limit, halts when hit | built |
| **footprint** | Mass, body weight, crush and ground drag | built |

## Base stats with no derivations

- **willpower:** authored, never read.
- **tags:** copied onto units, never read by the sim (they serve the registry and the Library).
- **Read raw though Decision 117 says they should derive from attributes:** initiative, turn rate, attack speed, detection range, courage, ward and HP.

## Findings

1. **Some values are fixed at spawn from the unit type.** Damage scaling, the skill penalty, the first load stage and max stamina come from the unit type at spawn. Later checks read the unit's own copy of its attributes. So a change to one unit (tech, items, ranks) would move its death's door and recovery but not its damage, skill or max stamina.
2. **Discipline is scaled three different ways.**
   - the pursuit leash: ÷ `discipline_expected_max`
   - walking slack and the chance to break ranks: ÷ a literal 100
   - withdrawal disorder: ÷ 50

   These should share one rule.
3. **Tiredness and morale only weaken attacks.** A spent or wavering unit parries and dodges as well as a fresh one, though spec 28 says tired units are less skilled.
4. **Finishing a downed body** subtracts the striker's raw damage, skipping the roll, damage type, armour and ward.
5. **Surrender** reads courage only. Decision 121 also names morale and disposition.
6. **Decision 124's "a downed regenerator rises at a quarter of its HP"** isn't in the code; Decision 126's wake timer replaced it. The ledger still states it.
7. **The UnitDef doc comment promises derivations that don't exist yet:** agility → attack speed, turning and initiative; constitution → HP; willpower → all four of its uses; wits → casting, mana and initiative.
8. **Authored data is thin.** Only the brute sets an attribute. No unit sets discipline, initiative, detection range, height, run pace, critical or attack speed. So nearly every derivation runs at its default in trials.
9. **The old spec 21 sim** takes raw weapon damage with no strength scaling. It differs from the formation sim.
10. **Spec 28's round 1 record and "What waits"** live only on the docs branches, not on the code branch.
11. **Agenda: stat ranges within a unit type.** The user asked whether one grem might have wits 5 and another wits 7, each rolled at creation within a type's range (say 4–8). If adopted, a type authors a range per attribute and each unit draws its value from the battle's seed. That would give every derivation (decision timing, initiative, skill) natural spread without per-tick noise. Open questions: whether the range is uniform or weighted to the middle, and whether ranks or traits shift it.
