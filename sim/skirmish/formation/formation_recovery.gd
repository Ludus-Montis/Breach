class_name FormationRecovery
extends RefCounted
## What the wounded come to (Decisions 121, 125 and 126). A unit's **condition** is 1 less
## wounds_condition for each wound past those it is hardened to (a "hardened N" trait), and
## later its other statuses; below condition_drain_below it loses HP, standing or downed,
## the faster the further below (condition_drain a second at condition 0) - so two wounds
## in good condition are survivable, a third bleeds it out unless it is aided soon. A
## **downed** unit lies a seeded while (wounds_wake_seconds, shorter the hardier it is) and
## then comes to at 1 HP where it lies: it joins the nearest standing friendly formation it
## can see - its own, or one that bore it - walking to a place at its back, and otherwise
## makes for home on its own, as a lone router (FormationStrays), to be caught, taken, or
## reach the reserve. With a foe within a few cells when it could get up, it may play dead
## instead (Decision 127). **Regeneration** mends HP up to its limit per rest, times its
## condition, through damage; a blow of a type it fears stops it a while; downed, it runs
## only with "regenerates_downed". Pure over the squads it is given.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")
const FormationCommand = preload("res://sim/skirmish/formation/formation_command.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const BattleRolls = preload("res://sim/skirmish/formation/battle_rolls.gd")
const FormationDeaths = preload("res://sim/skirmish/formation/formation_deaths.gd")
const FormationEvents = preload("res://sim/skirmish/formation/formation_events.gd")
const ScrumContest = preload("res://sim/skirmish/formation/scrum_contest.gd")
const BodyGrid = preload("res://sim/skirmish/formation/body_grid.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")
const SquadEntries = preload("res://sim/skirmish/formation/squad_entries.gd")


## One tick of `seconds`. Returns [[unit, its squad], ...] for those that came to apart from
## their formation, to make for home (FormationStrays).
static func step(squads: Array, seconds: float, tick: int, fight_seed: int, events: Array) -> Array:
	var waking := []  # [unit, its squad]: decided first, so no squad's order moves another's
	var standing := {"squads": squads, "cunning": {}}  # _lies_still
	for squad in squads:
		for unit in squad.units:
			var downed: bool = unit.state in [SkirmishUnit.State.DOWNED, SkirmishUnit.State.CARRIED]
			if not (unit.is_alive() or downed):
				continue
			unit.condition = condition_of(unit)
			_regenerate(unit, downed, seconds)
			if _drain(unit, seconds) and downed and unit.hp <= -FormationDeaths.depth(unit):
				unit.state = SkirmishUnit.State.DEAD
				events.append(FormationEvents.unit_event("died_of_wounds", tick, squad, unit))
				continue
			if downed and _wakes(unit, seconds, fight_seed):
				if not _lies_still(unit, squad, standing, fight_seed):
					waking.append([unit, squad])
				elif not unit.playing_dead:
					unit.playing_dead = true
					events.append(FormationEvents.unit_event("playing_dead", tick, squad, unit))
	return _rise(waking, squads, tick, fight_seed, events)


## Whether a unit due to come to lies still instead (Decision 127): with a standing foe
## near, one already playing dead keeps at it, and one not yet decides by a seeded chance
## from its wits and cunning; either looks again a few seconds on. `standing` holds the
## squads, and once found the standing units by faction (_index) and each squad's leaders'
## cunning.
static func _lies_still(
	unit: SkirmishUnit, squad: SkirmishSquad, standing: Dictionary, fight_seed: int
) -> bool:
	var tuning := BattleTuning.current()
	var reach := tuning.wounds_danger_reach
	var here: Vector2 = squad.store.columns.value(unit.slot, UnitColumns.POSITION)
	var danger := false
	if not standing.has("factions"):  # the standing, found once a tick when first asked
		standing["factions"] = _index(standing["squads"])
	for faction in standing["factions"]:
		if faction == unit.faction_id or danger:
			continue
		var index: Dictionary = standing["factions"][faction]
		for found in BodyGrid.near(index, here, reach + BodyGrid.MARGIN):
			if index["points"][found].distance_to(here) <= reach:
				danger = true
				break
	if not danger:
		return false
	if not unit.playing_dead:
		var wits: float = unit.attributes.get("wits", UnitDef.AVERAGE)
		var cunning := maxi(int(unit.traits.get("cunning", 0)), _led_cunning(squad, standing))
		var chance := tuning.wounds_play_dead * wits / UnitDef.AVERAGE
		chance += cunning * tuning.wounds_cunning
		if BattleRolls.uniform(fight_seed, [unit.id, unit.wounded, "play dead"]) >= chance:
			return false
	unit.wake_left = tuning.wounds_play_dead_check
	return true


## The most cunning among the squad's leaders (leadership > 0); very low with none. Found
## once a squad, kept in `standing`.
static func _led_cunning(squad: SkirmishSquad, standing: Dictionary) -> int:
	var known: Dictionary = standing["cunning"]
	if not known.has(squad):
		var most := -(1 << 62)
		for other in squad.living():
			if other.leadership > 0:
				most = maxi(most, int(other.traits.get("cunning", 0)))
		known[squad] = most
	return known[squad]


## {faction: a grid (BodyGrid) of where its standing units stand, with "units": [[unit,
## its squad, the squad's seeded draw], ...] in the squads' order, "points": where each
## stands, and "positions": the store's, by slot}; with `rising`, only
## formations a unit coming to may join: not destroyed or routing.
static func _index(squads: Array, rising := false, fight_seed := 0) -> Dictionary:
	var entries := {}
	for squad in squads:
		if rising and squad.state in [SkirmishSquad.State.DESTROYED, SkirmishSquad.State.ROUTING]:
			continue
		var draw := ScrumContest.squad_draw(squad, fight_seed) if rising else 0
		for unit in squad.living():
			if not entries.has(unit.faction_id):
				entries[unit.faction_id] = []
			entries[unit.faction_id].append([unit, squad, draw])
	var out := {}
	for faction in entries:
		var units: Array = entries[faction]
		var at: PackedVector2Array = units[0][1].store.columns.column(UnitColumns.POSITION)
		var points := units.map(func(entry): return at[entry[0].slot])
		out[faction] = BodyGrid.build(points)
		out[faction]["units"] = units
		out[faction]["points"] = points
		out[faction]["positions"] = at
	return out


## Its condition: 1, less a step for each wound past those it is hardened to; 0 to the cap.
static func condition_of(unit: SkirmishUnit) -> float:
	return clampf(_unclamped(unit), 0.0, BattleTuning.current().condition_cap)


## Its condition before it is held to 0: how far below 0 its state takes it (wounds, and
## later its other statuses).
static func _unclamped(unit: SkirmishUnit) -> float:
	var wounds := maxi(0, unit.wounded - int(unit.traits.get("hardened", 0)))
	return 1.0 - wounds * BattleTuning.current().wounds_condition


## A blow that landed of a type that stops the target's regeneration stops it a while.
static func scorch(target: SkirmishUnit, parts: Array, dealt: int) -> void:
	if dealt <= 0 or target.regeneration_stops.is_empty():
		return
	for part in parts:
		if target.regeneration_stops.has(part[1]):
			target.regeneration_halt = BattleTuning.current().wounds_regeneration_halt
			return


static func _regenerate(unit: SkirmishUnit, downed: bool, seconds: float) -> void:
	if unit.regeneration <= 0.0:
		return
	if downed and unit.traits.get("regenerates_downed", 0) <= 0:
		return
	if unit.regeneration_halt > 0.0:
		unit.regeneration_halt = maxf(0.0, unit.regeneration_halt - seconds)
		return
	var gain := minf(unit.regeneration * unit.condition * seconds, unit.regeneration_left)
	gain = minf(gain, float(unit.max_hp - unit.hp) - unit.regeneration_carry)
	if gain <= 0.0:
		return
	unit.regeneration_left -= gain
	unit.regeneration_carry += gain
	var whole := floori(unit.regeneration_carry + 0.000001)  # float sums fall short
	unit.hp += whole
	unit.regeneration_carry -= whole


## A poor condition drains its HP; true if it lost any this tick.
static func _drain(unit: SkirmishUnit, seconds: float) -> bool:
	var tuning := BattleTuning.current()
	var below := tuning.condition_drain_below - _unclamped(unit)
	if below <= 0.0:
		return false
	unit.drain_carry += tuning.condition_drain * below / tuning.condition_drain_below * seconds
	var whole := floori(unit.drain_carry + 0.000001)
	unit.hp -= whole
	unit.drain_carry -= whole
	return whole > 0


## Counts down its time downed, reckoning it first if it hasn't been; true once it is up.
static func _wakes(unit: SkirmishUnit, seconds: float, fight_seed: int) -> bool:
	var tuning := BattleTuning.current()
	if unit.wake_left < 0.0:
		var spread: Array[float] = tuning.wounds_wake_spread
		var roll := BattleRolls.uniform(fight_seed, [unit.id, unit.wounded, "wake"])
		var hardiness: float = (
			maxf(1.0, unit.attributes.get("constitution", UnitDef.AVERAGE)) / UnitDef.AVERAGE
		)
		unit.wake_left = tuning.wounds_wake_seconds * lerpf(spread[0], spread[1], roll) / hardiness
	unit.wake_left -= seconds
	return unit.wake_left <= 0.0


## Those coming to this tick ([unit, its squad]) get up together (Decision 97): each finds
## the nearest standing friendly formation it can see as all stood before any rose, then,
## the nearest first and ties by their seeded draws - never the list - each joins its
## formation, walking from where it lay to the next place at its back; one with none in
## sight makes for home alone. Returns those [unit, its squad].
static func _rise(waking: Array, squads: Array, tick: int, fight_seed: int, events: Array) -> Array:
	var plans := []  # [key, [unit, its squad], the formation it joins or null]
	var friends := _index(squads, true, fight_seed) if not waking.is_empty() else {}
	for entry in waking:
		var pick := _nearest(entry[0], friends)
		plans.append([[pick[1], ScrumContest.draw(entry[0], fight_seed)], entry, pick[0]])
	plans.sort_custom(func(a, b): return ScrumContest.before(a[0], b[0]))
	var strays := []
	for plan in plans:
		var unit: SkirmishUnit = plan[1][0]
		unit.hp = maxi(unit.hp, 1)
		unit.state = SkirmishUnit.State.MOVING
		unit.playing_dead = false
		var extra := {"alone": plan[2] == null}
		if plan[2] == null:
			strays.append(plan[1])
		else:
			_join(unit, plan[1][1], plan[2])
			extra["joined"] = plan[2].id
		events.append(FormationEvents.unit_event("came_to", tick, plan[1][1], unit, extra))
	return strays


## [the nearest standing friendly formation the unit can see, how far its nearest member
## is], or [null, INF] if there is none; `friends` are the formations it may join
## (_index). Looks ring by ring outwards from where it lies, stopping once no nearer member
## can lie further out; nearest first, then the squads' draws, then the squads' order.
static func _nearest(unit: SkirmishUnit, friends: Dictionary) -> Array:
	if not friends.has(unit.faction_id):
		return [null, INF]
	var index: Dictionary = friends[unit.faction_id]
	var units: Array = index["units"]
	var points: Array = index["points"]
	var at: Vector2 = index["positions"][unit.slot]
	var centre := BodyGrid.cell_of(at)
	var span := BodyGrid.ring_span(index, centre, at, unit.detection)
	var best: SkirmishSquad = null
	var best_key := [INF]
	for ring_number in range(span.x, span.y + 1):
		if BodyGrid.floor_of(ring_number) > best_key[0] + BodyGrid.MARGIN:
			break
		for found in BodyGrid.ring(index, centre, ring_number, at, unit.detection):
			var other: SkirmishUnit = units[found][0]
			var gap: float = points[found].distance_to(at)
			var key := [snappedf(gap, 0.000001), units[found][2], found]
			if other != unit and gap <= unit.detection and (best == null or key < best_key):
				best = units[found][1]
				best_key = key
	return [best, best_key[0]]


## The unit joins `best`, walking from where it lay to a place at its back.
static func _join(unit: SkirmishUnit, own: SkirmishSquad, best: SkirmishSquad) -> void:
	var back := 0
	for other in best.living():
		if other != unit:
			back = maxi(back, other.rank + other.footprint_depth)
	if best != own:
		SquadEntries.unloose(own, unit.id)
		SquadEntries.unflee(own, unit.id)
		own.units.erase(unit)
		FormationCommand.enlist(best, unit)
	unit.rank = back
	unit.column = 0
	best.reforming = true
	var at: Vector2 = best.store.columns.value(unit.slot, UnitColumns.POSITION)
	SquadEntries.loosen(best, unit, at)
