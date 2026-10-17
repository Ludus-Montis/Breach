class_name FormationWounds
extends RefCounted
## What becomes of the downed and the caught (Decision 121, spec 28 part 6). Units don't
## leave the downed: once no friend of a downed unit stands near it (wounds_guard_reach)
## and a standing foe beside it (wounds_reach) has no foe of its own near, that foe takes
## it - captures it if it is a captor, else strikes it on its blows' interval, down into
## death's door, constitution deep (FormationDeaths.depth), till it dies. The one
## exception: a foe whose formation's leader can send a messenger (its "messenger" trait,
## one a level) lets it go, to carry word home - a morale blow to every standing formation
## of its side. A routing unit struck may surrender in place, by a seeded chance from its
## want of courage (none for one with "never_surrenders"). What the downed come to, and
## regeneration, are FormationRecovery's. Decided from one snapshot, then applied (Decision
## 97). Pure over the squads it is given.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")
const BattleRolls = preload("res://sim/skirmish/formation/battle_rolls.gd")
const ScrumContest = preload("res://sim/skirmish/formation/scrum_contest.gd")
const FormationDeaths = preload("res://sim/skirmish/formation/formation_deaths.gd")
const FormationEvents = preload("res://sim/skirmish/formation/formation_events.gd")
const FormationLocks = preload("res://sim/skirmish/formation/formation_locks.gd")
const FormationMorale = preload("res://sim/skirmish/formation/formation_morale.gd")
const BodyGrid = preload("res://sim/skirmish/formation/body_grid.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")
## The columns the takers' strikes write, held for the pass: the body's hp, and each
## striker's cooldown and target (spec 35 step 7c).
const STRUCK := [UnitColumns.HP, UnitColumns.ATTACK_COOLDOWN, UnitColumns.TARGET_ID]


## This tick's takings of the downed. `interval` is ticks a second (a taker strikes on its
## melee interval). Only the standing near a downed body are weighed: its takers, its guards
## and the takers' foes all stand within wounds_reach + wounds_guard_reach of it.
static func tend(squads: Array, tick: int, interval: int, fight_seed: int, events: Array) -> void:
	var downed := []
	for squad in squads:
		for unit in UnitState.downed(squad):
			if unit.state == SkirmishUnit.State.DOWNED:  # a borne body isn't taken
				downed.append([unit, squad])
	if downed.is_empty():
		return
	var tuning := BattleTuning.current()
	var at: PackedVector2Array = downed[0][1].store.columns.column(UnitColumns.POSITION)
	var around := _cells_around(downed, at, tuning.wounds_reach + tuning.wounds_guard_reach)
	var standing := []  # [unit, squad], near a downed body
	for squad in squads:
		for unit in squad.living():
			if around.has(BodyGrid.cell_of(at[unit.slot])):
				standing.append([unit, squad])
	# taken and taking in the units' seeded draws' order, never the list's (Decision 97):
	# a captor or a leader's last messenger goes to the first of them
	standing = _by_draw(standing, fight_seed)
	downed = _by_draw(downed, fight_seed)
	var taken := {}  # body id -> [body, its squad, [taker, its squad], ...]
	var near := {
		"standing": BodyGrid.build(standing.map(func(entry): return at[entry[0].slot])),
		"downed": BodyGrid.build(downed.map(func(entry): return at[entry[0].slot])),
		"guarded": {},  # body -> whether a friend stands guard over it
		"at": at,  # where each stands, by slot
	}
	for taker in standing:
		var body := _within_reach(taker, standing, downed, [fight_seed, near])
		if not body.is_empty():
			if not taken.has(body[0].id):
				taken[body[0].id] = [body[0], body[1]]
			taken[body[0].id].append(taker)
	if taken.is_empty():
		return
	var health := UnitView.hold([downed[0][1]], STRUCK)
	var struck := [health.attack_cooldown, health.target_id]  # held for the pass
	for body_id in taken:
		_take(taken[body_id], squads, [tick, interval, health, struck], events)
	UnitView.release(health)


## The BodyGrid cells (a set) holding every point within `reach` of an entry's unit.
static func _cells_around(entries: Array, at: PackedVector2Array, reach: float) -> Dictionary:
	var span := floori((reach + BodyGrid.MARGIN) / BodyGrid.CELL) + 1
	var seen := {}
	var out := {}
	for entry in entries:
		var cell := BodyGrid.cell_of(at[entry[0].slot])
		if seen.has(cell):
			continue
		seen[cell] = true
		for y in range(cell.y - span, cell.y + span + 1):
			for x in range(cell.x - span, cell.x + span + 1):
				out[Vector2i(x, y)] = true
	return out


## `entries` ([unit, squad]) in their units' seeded draws' order, each draw taken once.
static func _by_draw(entries: Array, fight_seed: int) -> Array:
	var keyed := entries.map(func(entry): return [ScrumContest.draw(entry[0], fight_seed), entry])
	keyed.sort_custom(func(a, b): return a[0] < b[0])
	return keyed.map(func(pair): return pair[1])


## Routing units struck by this tick's `blows` ([striker, target, damage, ...]) that
## surrender.
static func surrender(
	blows: Array, squads: Array, fight_seed: int, tick: int, events: Array
) -> void:
	var routing := {}  # unit id -> its squad
	for squad in squads:
		if squad.state == SkirmishSquad.State.ROUTING:
			for unit in squad.living():
				routing[unit.id] = squad
	var chance := BattleTuning.current().wounds_surrender
	for blow in blows:
		var caught: SkirmishUnit = blow[1]
		if not routing.has(caught.id) or blow[2] <= 0 or not caught.is_alive():
			continue
		if UnitHealth.of_one(routing[caught.id], caught) <= 0:  # store: scattered (few: a struck router)
			continue
		if caught.traits.get("never_surrenders", 0) > 0:
			continue
		var want := chance * (1.0 - clampf(caught.courage / 100.0, 0.0, 1.0))
		if BattleRolls.uniform(fight_seed, [tick, caught.id, "surrender"]) < want:
			UnitState.put(routing[caught.id], caught, SkirmishUnit.State.TAKEN)
			events.append(
				FormationEvents.unit_event("surrendered", tick, routing[caught.id], caught)
			)
			_retire(routing[caught.id], squads, tick, events)


## The downed foe the taker would take, [body, its squad], or [] - none unguarded within
## reach, or a foe still near it. `drawn`: [the battle seed, BodyGrids of the standing and
## the downed where they stand, and which bodies are guarded as they are found].
static func _within_reach(taker: Array, standing: Array, downed: Array, drawn: Array) -> Array:
	var tuning := BattleTuning.current()
	var unit: SkirmishUnit = taker[0]
	var fight_seed: int = drawn[0]
	var near: Dictionary = drawn[1]
	var at: PackedVector2Array = near["at"]
	var here := at[unit.slot]
	var best := []
	var best_key := []
	for found in BodyGrid.near(near["downed"], here, tuning.wounds_reach + BodyGrid.MARGIN):
		var body: Array = downed[found]
		if body[0].faction_id == unit.faction_id:
			continue
		var gap: float = at[body[0].slot].distance_to(here)
		if gap > tuning.wounds_reach or _guarded(body[0], standing, near):
			continue
		if body[0].playing_dead and _fooled(unit, body[0], fight_seed):
			continue  # it takes the body for dead (Decision 127)
		var key := [snappedf(gap, 0.000001), ScrumContest.draw(body[0], fight_seed)]
		if best.is_empty() or key < best_key:
			best = body
			best_key = key
	if best.is_empty():
		return []
	var reach := tuning.wounds_guard_reach
	for found in BodyGrid.near(near["standing"], here, reach + BodyGrid.MARGIN):
		var other: Array = standing[found]
		if other[0].faction_id != unit.faction_id:
			if at[other[0].slot].distance_to(here) <= tuning.wounds_guard_reach:
				return []  # still fighting
	return best


## True if the taker believes a body playing dead is dead: the body's wits and cunning beat
## the taker's wits and thoroughness in a seeded roll, once for the pair.
static func _fooled(taker: SkirmishUnit, body: SkirmishUnit, fight_seed: int) -> bool:
	var tuning := BattleTuning.current()
	var guile: float = body.attributes.get("wits", UnitDef.AVERAGE)
	guile += int(body.traits.get("cunning", 0)) * tuning.wounds_trait_wits
	var sight: float = taker.attributes.get("wits", UnitDef.AVERAGE)
	sight += int(taker.traits.get("thorough", 0)) * tuning.wounds_trait_wits
	var roll := BattleRolls.uniform(fight_seed, [taker.id, body.id, "believe"])
	return guile - sight + (roll - 0.5) * tuning.wounds_believe_die > 0.0


## True if a friend of the body stands within wounds_guard_reach of it (once a body: `near`
## keeps the answer).
static func _guarded(body: SkirmishUnit, standing: Array, near: Dictionary) -> bool:
	if near["guarded"].has(body):
		return near["guarded"][body]
	var reach := BattleTuning.current().wounds_guard_reach
	near["guarded"][body] = false
	var at: PackedVector2Array = near["at"]
	for found in BodyGrid.near(near["standing"], at[body.slot], reach + BodyGrid.MARGIN):
		var other: Array = standing[found]
		if other[0].faction_id == body.faction_id:
			if at[other[0].slot].distance_to(at[body.slot]) <= reach:
				near["guarded"][body] = true
				break
	return near["guarded"][body]


## One body's takers act: a captor captures it; else a leader's messenger sends it home;
## else each taker due strikes it. `when` is [the tick, ticks a second, the pass's view and
## its cooldown and target columns, held for the pass].
static func _take(taking: Array, squads: Array, when: Array, events: Array) -> void:
	var tick: int = when[0]
	var interval: int = when[1]
	var health: UnitView = when[2]
	var cooldowns: PackedInt32Array = when[3][0]
	var targets: PackedInt64Array = when[3][1]
	var body: SkirmishUnit = taking[0]
	var takers := taking.slice(2)
	for taker in takers:
		if taker[0].traits.get("captor", 0) > 0:
			UnitState.put(taking[1], body, SkirmishUnit.State.TAKEN)
			var extra := {"by": taker[0].id}
			events.append(FormationEvents.unit_event("captured", tick, taking[1], body, extra))
			return
	for taker in takers:
		var leader := _messenger(taker[1])
		if leader != null:
			leader.traits["messenger"] = leader.traits["messenger"] - 1
			UnitState.put(taking[1], body, SkirmishUnit.State.RELEASED)
			events.append(FormationEvents.unit_event("sent_home", tick, taking[1], body))
			for squad in squads:
				if squad.faction_id == body.faction_id and not squad.is_destroyed():
					var shock := BattleTuning.current().wounds_messenger_shock
					FormationMorale.shock(squad, shock, tick, events)
			return
	for taker in takers:
		var striker: SkirmishUnit = taker[0]
		targets[striker.slot] = body.id
		cooldowns[striker.slot] -= 1
		if cooldowns[striker.slot] > 0:
			continue
		cooldowns[striker.slot] = maxi(1, roundi(interval * striker.melee_seconds))
		UnitHealth.hurt(health, body, striker.dmg)
	if UnitHealth.of(health, body) <= -FormationDeaths.depth(body):
		UnitState.put(taking[1], body, SkirmishUnit.State.DEAD)
		events.append(FormationEvents.unit_event("finished", tick, taking[1], body))


## The squad's leader that can still send a messenger, or null.
static func _messenger(squad: SkirmishSquad) -> SkirmishUnit:
	for unit in squad.living():
		if unit.leadership > 0 and unit.traits.get("messenger", 0) > 0:
			return unit
	return null


## A squad left with no one standing is destroyed, freeing whoever fought it.
static func _retire(squad: SkirmishSquad, squads: Array, tick: int, events: Array) -> void:
	if squad.is_destroyed() and squad.state != SkirmishSquad.State.DESTROYED:
		squad.state = SkirmishSquad.State.DESTROYED
		FormationLocks.release(squad, squads)
		events.append(FormationEvents.squad_event("destroyed", tick, squad))
