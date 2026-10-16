class_name ScrumContest
extends RefCounted
## Who gets a contested cell in the scrum (Decision 88), and the seeded draws that settle
## any tie left once what things are and where they stand are equal (Decision 97). The
## order is the key (arrival time, roll + initiative, initiative, speed, unit's draw),
## compared item by item
## - the earliest arrival first, then the higher roll plus initiative, then the higher
## initiative, the faster unit, and last a seeded draw wide enough that it never ties in
## practice. The roll is the unit's seeded variance, drawn once per contest from the fight's
## seed, the tick and the unit, so a replay repeats. Pure.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")


## The unit's key for a cell it would reach in `arrival` seconds at its `speed` (its pace
## this tick, the store's SPEED).
static func key(
	unit: SkirmishUnit, arrival: float, speed: float, fight_seed: int, tick: int
) -> Array:
	var die := BattleTuning.current().combat_contest_die  # 0 decides by stats alone
	var roll := posmod(hash([fight_seed, tick, unit.id]), die + 1) if die > 0 else 0
	return [arrival, -(roll + unit.initiative), -unit.initiative, -speed, draw(unit, fight_seed)]


## The unit's draw: seeded by the fight and the unit, 62 bits wide, so two units' draws
## (almost never) tie; the unit's id salts the draw but never orders it (Decision 97).
static func draw(unit: SkirmishUnit, fight_seed: int) -> int:
	var high := posmod(hash([fight_seed, unit.id, 0]), 1 << 31)
	return high * (1 << 31) + posmod(hash([fight_seed, unit.id, 1]), 1 << 31)


## A squad's draw, as draw() is a unit's: for ties between whole formations - which foe a
## squad locks onto, whose contact holds an edge (Decision 97).
static func squad_draw(squad: RefCounted, fight_seed: int) -> int:
	var high := posmod(hash([fight_seed, "squad", squad.id, 0]), 1 << 31)
	return high * (1 << 31) + posmod(hash([fight_seed, "squad", squad.id, 1]), 1 << 31)


## True if key `a` goes before key `b`.
static func before(a: Array, b: Array) -> bool:
	for index in range(mini(a.size(), b.size())):
		if a[index] != b[index]:
			return a[index] < b[index]
	return false
