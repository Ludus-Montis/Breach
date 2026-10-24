class_name SquadCompact
extends RefCounted
## A squad closing up when units fall (Decision 47; SkirmishSquad.compact): its living units
## step up a rank while their whole footprint is clear, but only front-preferring units step
## into the front rank; a squad whose front has all fallen re-anchors on its foremost rank.
## The rule is the crate's (`compact.rs`, spec 39 step 5b): this hands it the squad's living
## units and its moves under way, and finds the units that stepped up.

const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")


## Step-up, until nothing more can move. Returns the units that stepped up, in the order
## each first did. Only ranks change, and the front if the squad re-anchors.
static func compact(squad: SkirmishSquad) -> Array[SkirmishUnit]:
	var living := squad.living()
	var columns: UnitColumns = squad.store.columns
	var field: Object = ColumnsLent.field(columns)
	var front := squad.front_distance
	var stepped: PackedInt32Array = ColumnsLent.all(
		columns, func(): return field.squad_compact(squad.slot, closers(living), movers(squad))
	)
	placed(squad, front)
	return units_at(living, stepped)


## `living` as the crate closes them up, five numbers each: id, store slot, footprint width
## and depth, preferred band.
static func closers(living: Array) -> PackedInt64Array:
	var out := PackedInt64Array()
	for unit: SkirmishUnit in living:
		out.append_array(
			[
				unit.id,
				unit.slot,
				unit.footprint_width,
				unit.footprint_depth,
				unit.preferred_position
			]
		)
	return out


## The squad's moves under way (FormationShuffle's swaps), five numbers a unit in them: its
## store slot, the rank and column it heads for, its footprint width and depth.
static func movers(squad: SkirmishSquad) -> PackedInt32Array:
	var out := PackedInt32Array()
	for swap in squad.swaps:
		for mover in swap["to"]:
			var to: Vector2i = swap["to"][mover]
			out.append_array([mover.slot, to.x, to.y, mover.footprint_width, mover.footprint_depth])
	return out


## The squad's position follows its front if the crate moved it from `front` (StoredSquad).
static func placed(squad: SkirmishSquad, front: float) -> void:
	if squad.front_distance != front:
		squad.front_distance = squad.front_distance


## The units of `units` at `indices`, in that order.
static func units_at(units: Array, indices: PackedInt32Array) -> Array[SkirmishUnit]:
	var out: Array[SkirmishUnit] = []
	for index in indices:
		out.append(units[index])
	return out
