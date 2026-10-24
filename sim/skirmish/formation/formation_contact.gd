class_name FormationContact
extends RefCounted
## Contact between squads on one lane (specs/22-formation-feel-test.md, Decisions 40 and
## 44). Covers:
## - who can fight, and which hostile front is within melee reach
## - where an advancing squad must stop: at reach of a hostile front, or behind the back
##   rank of a friendly squad ahead (squads never pass through their own side)
## - how a wave that reaches a friendly squad in combat joins it from the back, as rear
##   ranks that step up as the front falls; a wave that merges (Decision 51) joins a
##   friendly squad on the march the same way
## - when a squad without front units holds to skirmish (Decision 47)
## Pure over the squads it is given; FormationSimulation calls it each tick. The rules are
## the crate's (`contact.rs`, spec 39 step 4c), on the squads' roster (CrateRoster); joining's
## moves (reinforce) stay here with the groups.

const FormationCommand = preload("res://sim/skirmish/formation/formation_command.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")

## Tiles between engaged fronts: a little over one cell (Decision 68).
const MELEE_REACH := 0.0175


## True if the squad seeks combat: not one retreating or routing (it is getting away), nor
## one that has arrived or is waiting (Decision 111).
## The crate's rule (`locks::can_engage`, spec 39 step 4b).
static func can_engage(candidate: SkirmishSquad) -> bool:
	return candidate.store.field().squad_can_engage(candidate.slot)


## The furthest an advancing squad may get this tick, wanting to reach `next`, judged
## from where everyone stood as the tick began: all squads move together (Decision 97).
## `marching` (squad id -> true) holds the squads advancing this tick; two hostiles
## marching at each other each close at most half the gap that stood between them.
## `roster`: the tick's (CrateRoster) of `squads`, made here if none is given.
static func limit(
	mover: SkirmishSquad,
	squads: Array,
	next: float,
	marching: Dictionary = {},
	roster: Object = null
) -> float:
	if roster == null:
		roster = CrateRoster.of(squads)
	var field: Object = mover.store.field()
	return field.contact_limit(roster, squads.find(mover), next, PackedInt64Array(marching.keys()))


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
## free to spread in a fight (Decision 51). Returns the units that moved up.
static func reinforce(leader: SkirmishSquad, joining: SkirmishSquad) -> Array[SkirmishUnit]:
	var width := maxi(leader.width, joining.width)
	var back := _back_rows(leader)
	var shift := (width - leader.width) / 2
	# store: scattered (few: a reinforcement, once as a squad joins)
	for unit in leader.units:
		UnitPlace.put(leader, unit, UnitPlace.of(leader, unit) + Vector2i(0, shift))
	var moved := Vector2i(back, (width - joining.width) / 2)  # read once (Decision 144)
	# store: scattered (few: a reinforcement, once as a squad joins)
	for unit in joining.living():
		UnitPlace.put(joining, unit, UnitPlace.of(joining, unit) + moved)
		joining.store.columns.put(unit.slot, UnitColumns.ATTACK_COOLDOWN, 1)
		SquadEntries.unloose(joining, unit.id)
		SquadEntries.unflee(joining, unit.id)
		FormationCommand.enlist(leader, unit)
		leader.joined.append(unit)
	leader.centre_shift += (width - leader.width) / 2.0 - shift  # the leader stays put
	leader.width = width
	joining.units.clear()
	UnitState.regather(joining)
	return leader.compact()


static func _back_rows(squad: SkirmishSquad) -> int:
	var rows := 0
	var ranks: PackedInt32Array = UnitView.read(squad, [UnitColumns.RANK]).rank  # read once
	for unit in squad.living():
		rows = maxi(rows, ranks[unit.slot] + unit.footprint_depth)
	return rows
