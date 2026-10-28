class_name FormationContact
extends RefCounted
## Contact between squads on one lane (specs/22-formation-feel-test.md, Decisions 40 and
## 44). Covers:
## - who can fight, and which hostile front is within melee reach
## - where an advancing squad must stop: at reach of a hostile front, or behind the back
##   rank of a friendly squad ahead (squads never pass through their own side); judged with
##   every move's landing (`moving::land`, spec 39 step 8b1)
## - how a wave that reaches a friendly squad in combat joins it from the back, as rear
##   ranks that step up as the front falls; a wave that merges (Decision 51) joins a
##   friendly squad on the march the same way
## - when a squad without front units holds to skirmish (Decision 47)
## Pure over the squads it is given; FormationSimulation calls it each tick. The rules are
## the crate's (`contact.rs`, spec 39 step 4c), on the squads' roster (CrateRoster); joining's
## moves too (`compact.rs`, spec 39 step 5b).

const FormationCommand = preload("res://sim/skirmish/formation/formation_command.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")

## Tiles between engaged fronts: a little over one cell (Decision 68).
const MELEE_REACH := 0.0175


## True if the squad seeks combat: not one retreating or routing (it is getting away), nor
## one that has arrived or is waiting (Decision 111).
## The crate's rule (`locks::can_engage`, spec 39 step 4b).
static func can_engage(candidate: SkirmishSquad) -> bool:
	return candidate.store.field().squad_can_engage(candidate.slot)


## True if an advancing squad with no front-preferring units has an enemy within its
## ranged reach: it holds there, skirmishing, rather than marching into melee (Decision 47).
## `roster`: the phase's (CrateRoster) of `squads`, made here if none is given.
static func skirmishing(mover: SkirmishSquad, squads: Array, roster: Object = null) -> bool:
	if roster == null:
		roster = CrateRoster.of(squads)
	return mover.store.field().contact_skirmishing(roster, squads.find(mover))


## The friendly squad whose back rank `mover` has reached and may join, or null: one in
## combat, or - when `mover` merges - one on the march that isn't retreating. `roster`: the
## phase's (CrateRoster) of `squads`, made here if none is given.
static func joinable(mover: SkirmishSquad, squads: Array, roster: Object = null) -> SkirmishSquad:
	if roster == null:
		roster = CrateRoster.of(squads)
	var found: int = mover.store.field().contact_joinable(roster, squads.find(mover))
	return squads[found] if found >= 0 else null


## Moves the joining squad's living units in behind the leader's back rank, both lines
## centred on the wider of the two, and steps them up. They are the leader's joined units,
## free to spread in a fight (Decision 51). Returns the units that moved up. The moves are
## the crate's (`compact.rs`, spec 39 step 5b); the units change lists and command here.
static func reinforce(leader: SkirmishSquad, joining: SkirmishSquad) -> Array[SkirmishUnit]:
	var frame := PackedInt32Array()  # all its units: the fallen shift with the frame
	for unit in leader.units:
		frame.append(unit.slot)
	var leading := leader.living()
	var taken := joining.living()
	var columns: UnitColumns = leader.store.columns
	var field: Object = ColumnsLent.field(columns)
	var front := leader.front_distance
	var stepped: PackedInt32Array = ColumnsLent.all(
		columns,
		func():
			return field.squad_reinforce(
				leader.slot,
				joining.slot,
				frame,
				SquadCompact.closers(leading),
				SquadCompact.closers(taken),
				SquadCompact.movers(leader)
			)
	)
	SquadCompact.placed(leader, front)
	for unit in taken:
		FormationCommand.enlist(leader, unit)
		leader.joined.append(unit)
	joining.units.clear()
	UnitState.regather(joining)
	return SquadCompact.units_at(leading + taken, stepped)
