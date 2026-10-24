class_name ScrumStance
extends RefCounted
## A disciplined squad meets a threat as a line (Decisions 88 and 92, spec 27 rounds 5
## and 6): marching or holding, with its front free, when it sees an enemy closing in on
## another face within scrum_anticipate cells and is disciplined enough (FormationDiscipline), it
## re-lays its places facing the threat at that face (its stance: its frame turned a
## quarter, half or three quarters about) before contact, and
## commits to it: it faces that enemy while it is still a threat, not swinging to another. Its
## units walk there at its re-form pace and keep to those places until the threat has
## gone; it doesn't march meanwhile. A less disciplined squad meets it unit by unit. Two
## threats equally near: the squads' seeded draw picks, not the list (Decision 97). The
## rules are the crate's (`stance.rs`, spec 39 step 4b). Pure.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const SquadFrame = preload("res://sim/skirmish/formation/squad_frame.gd")
const UnitMotion = preload("res://sim/skirmish/formation/unit_motion.gd")
const FormationDiscipline = preload("res://sim/skirmish/formation/formation_discipline.gd")
const FormationSight = preload("res://sim/skirmish/formation/formation_sight.gd")
const FormationEvents = preload("res://sim/skirmish/formation/formation_events.gd")
const FormationTerrain = preload("res://sim/skirmish/formation/formation_terrain.gd")
const FormationLocks = preload("res://sim/skirmish/formation/formation_locks.gd")
const SquadEdges = preload("res://sim/skirmish/formation/squad_edges.gd")


## The place a unit keeps to: in its squad's stance if it has one. `basis`: its squad's
## `frame`, read once a pass (Decision 144); `places`: a view of the squad's store holding
## UnitView.PLACES.
static func anchor(basis: Array, unit: SkirmishUnit, places: UnitView) -> Vector2:
	return SquadFrame.place_in(basis, unit, UnitPlace.in_view(places, unit))


## `anchor` for one unit, its place read from the store: scattered, for an event's few.
static func anchor_one(squad: SkirmishSquad, unit: SkirmishUnit) -> Vector2:
	return _anchor(squad, unit, UnitPlace.of(squad, unit))


static func _anchor(squad: SkirmishSquad, unit: SkirmishUnit, at: Vector2i) -> Vector2:
	if squad.stance.is_empty():
		return SquadFrame.place(
			squad.position, squad.heading, squad.width, squad.centre_shift, unit, at
		)
	var stance := squad.stance
	return SquadFrame.place(stance["anchor"], stance["heading"], squad.width, 0.0, unit, at)


## anchor()'s frame for the squad, worked out once for all its units: SquadFrame.place_in
## gives each unit's anchor from it, bit for bit.
static func frame(squad: SkirmishSquad) -> Array:
	if squad.stance.is_empty():
		return SquadFrame.basis(squad.position, squad.heading, squad.width, squad.centre_shift)
	return SquadFrame.basis(squad.stance["anchor"], squad.stance["heading"], squad.width, 0.0)


## Every squad, in turn, meets the threats it sees, sharing one roster (CrateRoster): a
## stance moves no squad's frame or units. The rules are the crate's (`stance.rs`, spec 39
## step 4b): this hands it each squad's bounds, whether it meets threats as a line
## (FormationDiscipline), and the hostiles each that may look sees (FormationSight); it lets
## go of stances in place and returns the new ones, whose anchors are worked out here
## (SquadEdges.reach).
static func anticipate_all(
	squads: Array, tick: int, events: Array, terrain: FormationTerrain, fight_seed: int = 0
) -> void:
	if squads.is_empty():
		return
	var roster := CrateRoster.of(squads)
	var factions := FormationLocks.factions(squads)
	var flags := PackedByteArray()
	var boxes := PackedVector2Array()
	for index in squads.size():
		var squad: SkirmishSquad = squads[index]
		var area := SquadEdges.bounds(squad, [roster, index])
		boxes.append_array([area.position, area.size])
		var meets := _looks(squad) and FormationDiscipline.meets_threats(squad)
		flags.append(int(not squad.living().is_empty()) | (2 if meets else 0))
	var counts := PackedInt32Array()
	var seen := PackedInt32Array()
	for index in squads.size():
		var before := seen.size()
		if flags[index] & 2:
			_sees(index, [squads, factions, flags], [terrain, roster], seen)
		counts.append(seen.size() - before)
	var field: Object = squads[0].store.field()
	var turns: PackedFloat64Array = field.anticipate(
		FormationLocks.slots(squads),
		factions,
		flags,
		boxes,
		counts,
		seen,
		BattleTuning.current().scrum_anticipate,
		fight_seed
	)
	for at in range(0, turns.size(), 3):
		var turning := int(turns[at])
		var when := [tick, roster, turning]
		_turn(squads[turning], [turns[at + 1], int(turns[at + 2])], when, events)


## True if the squad may meet a threat: marching, holding or fighting, its front free.
static func _looks(squad: SkirmishSquad) -> bool:
	var able := (
		squad.state
		in [SkirmishSquad.State.HOLDING, SkirmishSquad.State.MOVING, SkirmishSquad.State.FIGHTING]
	)
	return able and squad.engaged_with == 0


## Appends to `seen` the indices of the listed hostiles, living and not routing, that the
## squad at `index` detects. `listed` = [the squads, their faction codes, their flags];
## `seeing` = [the terrain, the phase's roster].
static func _sees(index: int, listed: Array, seeing: Array, seen: PackedInt32Array) -> void:
	var squads: Array = listed[0]
	var squad: SkirmishSquad = squads[index]
	for other_index in squads.size():
		var other: SkirmishSquad = squads[other_index]
		if listed[1][other_index] == listed[1][index] or not listed[2][other_index] & 1:
			continue
		if other.state == SkirmishSquad.State.ROUTING:
			continue
		if FormationSight.detects(squad, other, seeing[0], [seeing[1], index, other_index]):
			seen.append(other_index)


## The squad takes up a line facing `facing` = [heading, the foe's id]: its frame's anchor
## at the face of its footprints that way. `when` = [the tick, the phase's roster, the
## squad's index in it].
static func _turn(squad: SkirmishSquad, facing: Array, when: Array, events: Array) -> void:
	var listed := [when[1], when[2]]
	var heading: float = facing[0]
	var centre := SquadEdges.bounds(squad, listed).get_center()
	var outward := UnitMotion.vector(heading)
	var face := (
		centre + outward * (SquadEdges.reach(squad, outward, listed).y - centre.dot(outward))
	)
	squad.stance = {"anchor": face, "heading": heading, "foe": facing[1]}
	var faced := {"heading": heading, "facing": posmod(roundi(heading / 90.0), 4)}
	events.append(FormationEvents.squad_event("faced", when[0], squad, faced))
