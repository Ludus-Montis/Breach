class_name BodyFieldSync
extends RefCounted
## Brings a native BodyField (NativeKernels, Decision 129) to the squads as they stand: one
## call a sync, flat over the squads in list order - each squad's living units and how each
## stands (loose, fleeing, as its own squad's entries say). Where each stands the field
## reads from the unit store's columns, which live on it (UnitColumnsNative, spec 35). The
## field keeps its roster across ticks: units it hasn't
## seen are enrolled once, with GDScript's own radius, footprint area, initiative and
## seeded draw (ScrumContest.draw), so no rule is copied natively; the dead and the gone
## drop out; units that changed squad move with it. Two snapshots, as the passes read
## them: the scrum's (where ScrumReach puts a unit, and its bearing) and the bodies' (as
## UnitBodies.at: a router's offset from its point on its route). Returns the units by the
## sync's flat indices, for writing results back.

const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const ScrumReach = preload("res://sim/skirmish/formation/scrum_reach.gd")
const ScrumContest = preload("res://sim/skirmish/formation/scrum_contest.gd")
const FormationRout = preload("res://sim/skirmish/formation/formation_rout.gd")

## A body's flags, as the field reads them.
const LOOSE := 1
const FLEEING := 2
## A squad's flags.
const ROUTING := 1
const DESTROYED := 2


## One sync's arrays, flat over the squads.
class Batch:
	var units: Array = []
	var owners: Array = []
	var squad_ids := PackedInt64Array()
	var factions := PackedStringArray()
	var squad_flags := PackedByteArray()
	var counts := PackedInt32Array()
	var ids := PackedInt64Array()
	var flags := PackedByteArray()
	var nexts := PackedVector2Array()
	var bases := PackedVector2Array()
	var bearings := PackedFloat64Array()


## Syncs the field to the scrum's snapshot; returns [units, their squads] by flat index.
static func scrum(field: Object, squads: Array, fight_seed: int) -> Array:
	var batch := Batch.new()
	for squad in squads:
		var before := batch.units.size()
		var loose: Dictionary = squad.loose
		for unit in squad.units:
			if not unit.is_alive():
				continue
			batch.units.append(unit)
			batch.owners.append(squad)
			batch.ids.append(unit.id)
			batch.flags.append(LOOSE if loose.has(unit.id) else 0)  # a router: where it stands
			batch.bearings.append(unit.bearing)
		_squad(batch, squad, batch.units.size() - before)
	return _send(field, batch, fight_seed)


## Syncs the field to the bodies as UnitBodies sees them; returns [units, their squads].
static func bodies(field: Object, squads: Array, fight_seed: int) -> Array:
	var batch := Batch.new()
	for squad in squads:
		var before := batch.units.size()
		var loose: Dictionary = squad.loose
		var fleeing: Dictionary = squad.fleeing
		for unit in squad.units:
			if not unit.is_alive():
				continue
			var entry = loose.get(unit.id)
			batch.units.append(unit)
			batch.owners.append(squad)
			batch.ids.append(unit.id)
			var flag := 0 if entry == null else LOOSE
			if fleeing.has(unit.id):
				batch.nexts.append(Vector2.ZERO)
				batch.bases.append(FormationRout.route_point(squad, unit.id))
				flag |= FLEEING
			else:
				batch.nexts.append(Vector2.ZERO if entry == null else entry["next"])
				batch.bases.append(Vector2.ZERO)
			batch.flags.append(flag)
		_squad(batch, squad, batch.units.size() - before)
	return _send(field, batch, fight_seed)


static func _squad(batch: Batch, squad: SkirmishSquad, count: int) -> void:
	batch.squad_ids.append(squad.id)
	batch.factions.append(squad.faction_id)
	var flags := ROUTING if squad.state == SkirmishSquad.State.ROUTING else 0
	batch.squad_flags.append(DESTROYED if squad.state == SkirmishSquad.State.DESTROYED else flags)
	batch.counts.append(count)


static func _send(field: Object, batch: Batch, fight_seed: int) -> Array:
	if not batch.owners.is_empty() and batch.owners[0].store.field() != field:
		push_error("BodyFieldSync: the squads' unit store isn't on this field (spec 35)")
	var unknown: PackedInt32Array = field.sync(
		fight_seed,
		batch.squad_ids,
		batch.factions,
		batch.squad_flags,
		batch.counts,
		batch.ids,
		batch.flags,
		batch.nexts,
		batch.bases,
		batch.bearings
	)
	if not unknown.is_empty():
		_enrol(field, unknown, batch.units, fight_seed)
	return [batch.units, batch.owners]


## Hands the field what it needs once about each new unit, as GDScript reckons it.
static func _enrol(field: Object, flat: PackedInt32Array, units: Array, fight_seed: int) -> void:
	var radii := PackedFloat64Array()
	var areas := PackedFloat64Array()
	var draws := PackedInt64Array()
	var initiatives := PackedInt64Array()
	for index in flat:
		var unit: SkirmishUnit = units[index]
		radii.append(ScrumReach.radius(unit))
		areas.append(float(unit.footprint_width * unit.footprint_depth))
		draws.append(ScrumContest.draw(unit, fight_seed))
		initiatives.append(unit.initiative)
	field.enrol(flat, radii, areas, draws, initiatives)
