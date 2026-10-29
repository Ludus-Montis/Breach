class_name FormationCombat
extends RefCounted
## How hard a blow lands, and ranged strikes, per specs/22-formation-feel-test.md and
## Decision 40. Pure: FormationSimulation calls it each tick.
##
## Ranged units (Decisions 46-47) strike with their best ranged weapon from anywhere in
## their squad, moving or fighting: the nearest enemy unit within its range in ranks,
## preferring one they overlap laterally, then by the seeded draw (Decision 97). A ranged
## unit in the front rank of an engaged squad strikes with its melee weapons instead. Whom
## each strikes, its target and its cooldown are the crate's (`shots.rs`, spec 39 step 8a1):
## this hands it the squads' roster (CrateRoster) and the shooters with the cooldown each
## starts again at, and makes the shots.

const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")


## This tick's ranged strikes: [[shooter, target, damage, shooter's squad], ...]. Updates
## each shooter's target and cooldown, like melee fighters.
static func ranged_blows(squads: Array, interval_ticks: int, fight_seed: int = 0) -> Array:
	var shooters := []  # [shooter, its squad]
	var picked := PackedInt64Array()
	for index in squads.size():
		var own: SkirmishSquad = squads[index]
		if own.is_destroyed() or own.state == SkirmishSquad.State.ARRIVED:
			continue
		var melee := {}
		if own.state == SkirmishSquad.State.FIGHTING:
			for fighter in own.fighters():
				melee[fighter] = true
		var living := own.living()
		for member in living.size():
			var shooter: SkirmishUnit = living[member]
			if shooter.attack_range <= 0 or melee.has(shooter):
				continue
			shooters.append([shooter, own])
			picked.append_array(
				[index, member, maxi(1, roundi(interval_ticks * shooter.ranged_seconds))]
			)
	if shooters.is_empty():
		return []
	var columns: UnitColumns = squads[0].store.columns
	var field: Object = ColumnsLent.field(columns)
	var roster := CrateRoster.of(squads)
	var shot: PackedInt64Array = ColumnsLent.all(
		columns, func(): return field.shots(roster, picked, fight_seed)
	)
	var livings := {}  # a struck squad's index -> its living, as the roster lists them
	var blows := []
	for at in range(0, shot.size(), 3):
		var shooter: SkirmishUnit = shooters[shot[at]][0]
		if not livings.has(shot[at + 1]):
			livings[shot[at + 1]] = squads[shot[at + 1]].living()
		var target: SkirmishUnit = livings[shot[at + 1]][shot[at + 2]]
		blows.append([shooter, target, shooter.ranged_dmg, shooters[shot[at]][1]])
	return blows


## A melee blow's damage before it lands: all the striker's melee weapons together. How
## it lands - a flank finding no parry - is BlowLanding's (Decision 118).
static func damage(fighter: SkirmishUnit) -> int:
	return fighter.dmg
