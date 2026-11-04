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
## rules are the crate's (`stance.rs`, spec 39 step 4b; the pass `scrum::prepare`, step
## 8b5-xii).

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const SquadFrame = preload("res://sim/skirmish/formation/squad_frame.gd")
const FormationTerrain = preload("res://sim/skirmish/formation/formation_terrain.gd")


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


## Every squad, in turn, meets the threats it sees, over the scrum's roster (CrateRoster), in
## the crate (`scrum::prepare::anticipate`, spec 39 step 8b5-xii): a squad marching, holding
## or fighting, its front free and disciplined enough (FormationDiscipline), meets the
## hostiles it sees (FormationSight); stances let go are cleared, and each new one is taken,
## its anchor at the face of its footprints towards its heading, its "faced" event taken from
## the crate's log into `events`. `when`: the tick and the battle's seed.
static func anticipate_all(
	field: Object, roster: Object, when: Array, terrain: FormationTerrain, events: Array
) -> void:
	if field == null:
		return  # no squads: no stances
	var tuning := BattleTuning.current()
	var rules := PackedFloat64Array(
		[tuning.scrum_anticipate, tuning.discipline_per_leadership, tuning.discipline_meets_threats]
	)
	field.scrum_stance(roster, UnitMover.ground(terrain), rules, when[0], when[1])
	events.append_array(CrateEvents.taken(field))
