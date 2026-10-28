class_name FormationCombat
extends RefCounted
## How hard a blow lands, and ranged strikes, per specs/22-formation-feel-test.md and
## Decision 40. Pure: FormationSimulation calls it each tick.
##
## Ranged units (Decisions 46-47) strike with their best ranged weapon from anywhere in
## their squad, moving or fighting: the nearest enemy unit within its range in ranks,
## preferring one they overlap laterally, then by the seeded draw (Decision 97). A ranged
## unit in the front rank of an engaged squad strikes with its melee weapons instead. Whom
## each strikes, its target and its cooldown are the crate's (`shots.rs`, spec 39 step 8a1),
## and who may shoot (step 8a2-v): this hands it the squads' roster (CrateRoster) and each
## living unit's ranged time, and makes the shots.

const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")


## This tick's ranged strikes: [[shooter, target, damage, shooter's squad], ...]. Updates
## each shooter's target and cooldown, like melee fighters.
static func ranged_blows(squads: Array, interval_ticks: int, fight_seed: int = 0) -> Array:
	if squads.is_empty():
		return []
	var livings := []  # each squad's living, as the roster lists them
	var seconds := PackedFloat64Array()
	var ranged := false  # none with a range: nothing to ask the crate
	for own: SkirmishSquad in squads:
		var living := own.living()
		livings.append(living)
		for unit: SkirmishUnit in living:
			seconds.append(unit.ranged_seconds)
			ranged = ranged or unit.attack_range > 0
	if not ranged:
		return []
	var columns: UnitColumns = squads[0].store.columns
	var field: Object = ColumnsLent.field(columns)
	var roster := CrateRoster.of(squads)
	var shot: PackedInt64Array = ColumnsLent.all(
		columns, func(): return field.shots(roster, seconds, interval_ticks, fight_seed)
	)
	var blows := []
	for at in range(0, shot.size(), 4):
		var shooter: SkirmishUnit = livings[shot[at]][shot[at + 1]]
		var target: SkirmishUnit = livings[shot[at + 2]][shot[at + 3]]
		blows.append([shooter, target, shooter.ranged_dmg, squads[shot[at]]])
	return blows


## A melee blow's damage before it lands: all the striker's melee weapons together. How
## it lands - a flank finding no parry - is BlowLanding's (Decision 118).
static func damage(fighter: SkirmishUnit) -> int:
	return fighter.dmg
