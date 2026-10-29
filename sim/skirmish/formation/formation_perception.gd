class_name FormationPerception
extends RefCounted
## What each formation sees at the start of a tick (Decision 134; spec 34, item 2): the
## standing hostile formations it detects (FormationSight: within its units' detection
## range and in sight), and whether it is **threatened** - it sees one. Gathered once a
## tick, before anything moves, and read by the passes that decide (taking the downed now;
## seeking, rescuers and the wits beat to come), so each decides on the same picture and
## none searches again. A pair too far apart for either's widest sight to reach is skipped
## before any unit is looked at, so the gather grows with the formations near each other,
## not with every pair. The rules are the crate's (`perception.rs`, spec 39 step 4c), which
## keeps each one's threat (SkirmishSquad.threatened, step 8b5-xxxvii).

const FormationTerrain = preload("res://sim/skirmish/formation/formation_terrain.gd")
const UnitMover = preload("res://sim/skirmish/formation/unit_mover.gd")


## squad id -> {"seen": [the standing hostile squads it detects], "threat": true if any}.
## `roster`: the tick's (CrateRoster), made here if none is given.
static func see(
	squads: Array, terrain: FormationTerrain = null, roster: Object = null
) -> Dictionary:
	var out := {}
	if squads.is_empty():
		return out
	if roster == null:
		roster = CrateRoster.of(squads)
	var field: Object = squads[0].store.field()
	var flat: PackedInt32Array = field.perception_see(roster, UnitMover.ground(terrain))
	var at := 0
	for squad in squads:
		var seen := []
		for index in range(at + 1, at + 1 + flat[at]):
			seen.append(squads[flat[index]])
		at += 1 + flat[at]
		out[squad.id] = {"seen": seen, "threat": not seen.is_empty()}
	return out
