class_name ScrumPursuit
extends RefCounted
## Retreat and pursuit (Decisions 95 and 109, spec 27 round 8). When a formation is
## ordered to retreat out of a fight:
## - **The retreat:** its locks end at once and it withdraws (FormationWithdraw, Decision
##   99): its units flee from where they stand until it is safe. A ragged one also pays a
##   scaled rout: a morale shock in proportion to how far short of drilled it is.
## - **Its enemies:** any of their units still touching it strike it as it goes
##   (ScrumBlows). Each follows it as a whole, to the leash its discipline gives it
##   (FormationPursuit, Decision 107), unless ordered not to pursue. And each of their units
##   near it may break ranks to chase on its own - decided per unit, by its discipline and a
##   seeded roll - as far from where it broke away as its own discipline leashes it, while
##   it can see its quarry; then it goes back to its formation.
## Chasers all pick their quarry before any moves: the nearest unit, ties by its seeded
## draw (Decision 97).
## Squads keep `pursues` and `chasers` (unit id -> {"unit", "foe", "from", "leash"}). Pure
## over the squads it is given. The chase and the breaking of ranks are the crate's
## (`pursuit.rs`, spec 39 step 4c): this hands it the squads' roster (CrateRoster) and where
## chasers walk back to on the ground (TerrainStep.returns). The retreat itself, and who is
## tempted to break ranks, are the crate's orders (FormationOrders, step 7c).

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const TerrainStep = preload("res://sim/skirmish/formation/terrain_step.gd")
const UnitMover = preload("res://sim/skirmish/formation/unit_mover.gd")
const FormationTerrain = preload("res://sim/skirmish/formation/formation_terrain.gd")


## One tick of chasing: each chaser walks at the march pace, paying the ground it crosses
## (UnitMover), after the nearest unit of the enemy it chases; once at its leash, out of
## sight of that enemy, or that enemy gone, it straggles back on its own to its place (or
## the nearest it can walk to it: TerrainStep.returns), wherever its formation now is, and
## rejoins it there - the formation doesn't wait for it (Decision 112). `roster`: the
## tick's (CrateRoster), made here if none is given.
static func step(
	squads: Array,
	_tick: int,
	cells_per_second: float,
	seconds: float,
	fight_seed: int = 0,
	terrain: FormationTerrain = null,
	roster: Object = null
) -> void:
	var listed := PackedInt32Array()
	var counts := PackedInt32Array()
	var homes := PackedInt64Array()
	var home_at := PackedVector2Array()
	for index in squads.size():
		var squad: SkirmishSquad = squads[index]
		if squad.chasers.is_empty():
			continue
		listed.append(index)
		var returns := TerrainStep.returns(squad, terrain, fight_seed)
		counts.append(returns.size())
		for unit_id in returns:
			homes.append(unit_id)
			home_at.append(returns[unit_id])
	if listed.is_empty():
		return
	if roster == null:
		roster = CrateRoster.of(squads, terrain)
	var tuning := BattleTuning.current()
	var rules := PackedFloat64Array(
		[
			cells_per_second,
			seconds,
			tuning.stamina_give_up,
			tuning.bodies_steer_look,
			tuning.bodies_steer_clear,
		]
	)
	squads[0].store.field().pursue_chasers(
		roster, listed, counts, homes, home_at, UnitMover.ground(terrain), rules, fight_seed
	)


## True if the squad's unit is away from its place on its own - chasing, or its squad
## withdrawing - so its squad leaves it out of regrouping.
static func away(squad: SkirmishSquad, unit_id: int) -> bool:
	return squad.chasers.has(unit_id) or not squad.withdraw.is_empty()
