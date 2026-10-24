class_name CrateRoster
extends RefCounted
## The squads a phase in the crate is given (`SquadRoster`, native/rust/src/roster.rs, spec
## 39 step 4c): each squad of the lists, in their order, with its living units (UnitState
## keeps who is living, Decision 147) and what of each the crate doesn't hold - its id, store
## slot, footprint, preferred band, kind, turn rate, backward pace, detection range, and on
## terrain the walker it crosses the ground as. Made once a tick and handed to each phase.

const ROSTER_CLASS := "SquadRoster"


## The roster of `squads` on `terrain` (null: open ground, no walkers).
static func of(squads: Array, terrain: FormationTerrain = null) -> Object:
	NativeLibrary.load_for(ROSTER_CLASS)
	var roster: Object = ClassDB.instantiate(ROSTER_CLASS)
	var ids := PackedInt64Array()
	var counts := PackedInt32Array()
	var unit_ids := PackedInt64Array()
	var ints := PackedInt32Array()
	var reals := PackedFloat64Array()
	var kinds := {}  # definition -> its code
	for squad in squads:
		var living: Array = squad.living()
		ids.append(squad.id)
		counts.append(living.size())
		for unit: SkirmishUnit in living:
			unit_ids.append(unit.id)
			var kind: int = kinds.get_or_add(unit.definition, kinds.size())
			ints.append_array(
				[
					unit.slot,
					unit.footprint_width,
					unit.footprint_depth,
					unit.preferred_position,
					kind
				]
			)
			reals.append_array([unit.turn_rate, unit.backward_pace, unit.detection])
			if terrain != null:
				reals.append_array(TerrainWalker.of_unit(unit).fields())
	roster.fill(
		FormationLocks.slots(squads),
		ids,
		FormationLocks.factions(squads),
		counts,
		unit_ids,
		ints,
		reals
	)
	return roster
