class_name FormationDeaths
extends RefCounted
## The fallen (specs/22-formation-feel-test.md, Decisions 47, 82 and 121): a unit brought to
## 0 hp is downed - its hp stops at 0, the excess lost, and it lies where it fell - unless
## the blows took it past minus its constitution, which kills it outright. Either way it
## leaves the fight and shakes its squad, the ranks behind step up (units find the fight
## themselves: Decision 88), and a squad with no one left is destroyed, freeing whoever
## fought it. What becomes of the downed is FormationWounds'. The fallen are those on the
## store's fell queue (UnitHealth, Decision 145), so deaths costs per fallen unit, not per
## army. Pure over the squads.

const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")
const FormationEvents = preload("res://sim/skirmish/formation/formation_events.gd")
const FormationLocks = preload("res://sim/skirmish/formation/formation_locks.gd")
const FormationMorale = preload("res://sim/skirmish/formation/formation_morale.gd")


static func bury(squads: Array, tick: int, events: Array) -> void:
	var by_squad := _fallen(squads)
	for index in range(squads.size()):
		if not by_squad.has(index):
			continue
		var fallen_squad: SkirmishSquad = squads[index]
		var fallen: Array = by_squad[index]
		for unit in fallen:  # store: scattered (the tick's fallen, a few)
			if UnitHealth.of(fallen_squad, unit) <= -depth(unit):
				unit.state = SkirmishUnit.State.DEAD
				events.append(FormationEvents.unit_event("died", tick, fallen_squad, unit))
			else:
				UnitHealth.put(fallen_squad, unit, 0)
				unit.state = SkirmishUnit.State.DOWNED
				unit.wounded += 1 + unit.last_wounding  # lowering its condition (Decision 126)
				unit.wake_left = -1.0
				events.append(FormationEvents.unit_event("downed", tick, fallen_squad, unit))
		FormationMorale.losses(fallen_squad, fallen, tick, events)
		# store: scattered (step-ups: the few units that moved up)
		for moved in fallen_squad.compact():
			var extra := {"rank": UnitPlace.rank(fallen_squad, moved)}
			events.append(
				FormationEvents.unit_event("stepped_up", tick, fallen_squad, moved, extra)
			)
		fallen_squad.reforming = true  # front units may close gaps (Decision 49)
		if fallen_squad.is_destroyed() and fallen_squad.state != SkirmishSquad.State.DESTROYED:
			fallen_squad.state = SkirmishSquad.State.DESTROYED
			FormationLocks.release(fallen_squad, squads)
			events.append(FormationEvents.squad_event("destroyed", tick, fallen_squad))


## The fell queue's units still living at 0 hp or below, by the index of the squad each is
## in now, each squad's in its own list's order (Decision 145): the order the scan of every
## squad's living met them in, never the order they fell. A unit healed above 0 again, no
## longer living, or in no squad is left out, and one that fell twice is taken once.
static func _fallen(squads: Array) -> Dictionary:
	var where := {}  # squad id -> its index in squads
	var stores := []
	for index in range(squads.size()):
		where[squads[index].id] = index
		if not stores.has(squads[index].store):
			stores.append(squads[index].store)
	var found := {}  # squad index -> [[index in its list, unit], ...]
	var seen := {}
	for store: UnitStore in stores:
		for unit_id in store.fallen.take():
			if seen.has(unit_id):
				continue
			seen[unit_id] = true
			var unit = store.record_of(unit_id)
			if unit == null or store.slot_of(unit_id) < 0 or not unit.is_alive():
				continue
			var at := _place(unit, squads, where)
			if at.is_empty() or UnitHealth.of(squads[at[0]], unit) > 0:
				continue
			if not found.has(at[0]):
				found[at[0]] = []
			found[at[0]].append([at[1], unit])
	var out := {}
	for index in found:
		found[index].sort_custom(func(a, b): return a[0] < b[0])
		out[index] = found[index].map(func(entry): return entry[1])
	return out


## [the index of the squad `unit` is in now, its index in that squad's list], or []. Its
## own squad's id is looked at first; a unit moved since (joined, reinforced, split off)
## is found by looking through the rest.
static func _place(unit: SkirmishUnit, squads: Array, where: Dictionary) -> Array:
	if where.has(unit.squad_id):
		var at: int = squads[where[unit.squad_id]].units.find(unit)
		if at >= 0:
			return [where[unit.squad_id], at]
	for index in range(squads.size()):
		var at: int = squads[index].units.find(unit)
		if at >= 0:
			return [index, at]
	return []


## How far below 0 hp a unit lives on, at death's door: its constitution (Decision 121).
static func depth(unit: SkirmishUnit) -> int:
	return int(unit.attributes.get("constitution", UnitDef.AVERAGE))
