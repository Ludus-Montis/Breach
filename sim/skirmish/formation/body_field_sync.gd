class_name BodyFieldSync
extends RefCounted
## Brings a native BodyField (NativeKernels, Decision 129) to the squads as they stand: one
## call a sync, flat over the squads in list order - each squad's living units (and a
## fleeing one's point on its route). How each stands (loose, fleeing), where, its next
## point and its bearing the field reads from the unit store's columns, which live on it
## (UnitColumnsNative, spec 35 step 4e). The
## field keeps its roster across ticks: units it hasn't
## seen are enrolled once, with GDScript's own radius, footprint area, initiative and
## seeded draw (ScrumContest.draw), so no rule is copied natively; the dead and the gone
## drop out; units that changed squad move with it. Two snapshots, as the passes read
## them: the scrum's (where ScrumReach puts a unit, and its bearing) and the bodies' (as
## UnitBodies.at: a router's offset from its point on its route). Returns the units by the
## sync's flat indices, for writing results back. A new unit's record (its preferred band,
## footprint, turn rate and backward pace) is enrolled with it (spec 35 steps 4b, 4c).

const StoreHolds = preload("res://sim/skirmish/formation/store_holds.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const ScrumReach = preload("res://sim/skirmish/formation/scrum_reach.gd")
const ScrumContest = preload("res://sim/skirmish/formation/scrum_contest.gd")
const FormationRout = preload("res://sim/skirmish/formation/formation_rout.gd")

## Which snapshot a sync takes (BodyField.sync): the scrum's (each unit loose or not, and
## its bearing) or the bodies' (loose or fleeing, and a loose one's next point). How each
## stands, its next point and its bearing are the unit store's, read on the field (spec 35
## step 4e).
const SCRUM := 1
const BODIES := 2
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
	var mode := SCRUM
	var bases := PackedVector2Array()


## Syncs the field to the scrum's snapshot; returns [units, their squads] by flat index.
static func scrum(field: Object, squads: Array, fight_seed: int) -> Array:
	assert(StoreHolds.none_held(squads), StoreHolds.SPANS)
	var batch := Batch.new()
	for squad in squads:
		var before := batch.units.size()
		for unit in squad.units:
			if not unit.is_alive():
				continue
			batch.units.append(unit)
			batch.owners.append(squad)
			batch.ids.append(unit.id)
		_squad(batch, squad, batch.units.size() - before)
	return _send(field, batch, fight_seed)


## Syncs the field to the bodies as UnitBodies sees them; returns [units, their squads].
static func bodies(field: Object, squads: Array, fight_seed: int) -> Array:
	assert(StoreHolds.none_held(squads), StoreHolds.SPANS)
	var batch := Batch.new()
	batch.mode = BODIES
	for squad in squads:
		var before := batch.units.size()
		var fleeing: Dictionary = squad.fleeing
		for unit in squad.units:
			if not unit.is_alive():
				continue
			batch.units.append(unit)
			batch.owners.append(squad)
			batch.ids.append(unit.id)
			var fled := fleeing.has(unit.id)  # its base: its point on its squad's route
			batch.bases.append(FormationRout.route_point(squad, unit.id) if fled else Vector2.ZERO)
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
		batch.mode,
		batch.bases
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
	var records := PackedInt64Array()  # the unit's record (spec 35 step 4b), 3 per unit
	var paces := PackedFloat64Array()  # its turn rate and backward pace (step 4c)
	for index in flat:
		var unit: SkirmishUnit = units[index]
		radii.append(ScrumReach.radius(unit))
		areas.append(float(unit.footprint_width * unit.footprint_depth))
		draws.append(ScrumContest.draw(unit, fight_seed))
		initiatives.append(unit.initiative)
		records.append_array([unit.preferred_position, unit.footprint_width, unit.footprint_depth])
		paces.append_array([unit.turn_rate, unit.backward_pace])
	field.enrol(flat, radii, areas, draws, initiatives, records, paces)
