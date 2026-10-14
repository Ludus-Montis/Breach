class_name ShuffleFront
extends RefCounted
## Where a unit behind the front steps up to when a front place falls free (Decision 49,
## FormationShuffle): the nearest free place in the front rank, within the squad's columns
## or - for one joined in a fight - spread within the combat width (Decision 51). The free
## places are found once a planning (open), so each unit weighs those few, not every
## column. Pure.

const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const SquadPlaces = preload("res://sim/skirmish/formation/squad_places.gd")

const EPSILON := 0.000001


## The front rank's columns whose first cell nobody stands in or has claimed as the moves
## are planned, in order, across every column any unit may reach: places only ever fill as
## moves are planned, so a place free to a unit later is among them - each unit looks at
## these, not every column (bounded-work).
static func open(
	squad: SkirmishSquad, units: Array, claimed: Dictionary, occupied: Dictionary
) -> PackedInt32Array:
	var reach := squad.combat_width
	for unit in units:
		reach = maxi(reach, unit.footprint_width)
	var cells: Dictionary = occupied["cells"]
	var open := PackedInt32Array()
	for column in range(-reach, squad.width + reach):
		var cell := Vector2i(0, column)
		if not cells.has(cell) and not claimed["cells"].has(cell):
			open.append(column)
	return open


## `held`: [SquadPlaces.occupancy, open()'s columns].
static func free_place(
	squad: SkirmishSquad, unit: SkirmishUnit, claimed: Dictionary, held: Array
) -> Dictionary:
	if unit.preferred_position != 0:
		return {}
	var occupied: Dictionary = held[0]
	var best := {}
	var spreading := (
		squad.state == SkirmishSquad.State.FIGHTING
		and squad.joined.has(unit)
		and squad.combat_width > 0
	)
	var reach := squad.combat_width if spreading else unit.footprint_width
	for column: int in held[1]:
		if column < -reach or column > squad.width + reach - unit.footprint_width:
			continue
		var place := Vector2i(0, column)
		var inside := column >= 0 and column + unit.footprint_width <= squad.width
		if not inside and not _within_combat_width(squad, column, unit, claimed):
			continue
		if not SquadPlaces.vacant(occupied, place, unit, [unit], claimed["cells"]):
			continue
		var distance := maxi(unit.rank, absi(column - unit.column))
		if best.is_empty() or distance < best["distance"]:
			best = {"mover": unit, "passed": [], "to": {unit: place}, "distance": distance}
	return best


## True if `unit` at `column` keeps the line, planned moves included, within the combat
## width, and no more than half a column past either edge of the lane (Decision 51).
static func _within_combat_width(
	squad: SkirmishSquad, column: int, unit: SkirmishUnit, claimed: Dictionary
) -> bool:
	if squad.combat_width <= 0:
		return false
	var left: int = mini(mini(0, column), claimed["left"])  # planned moves' columns too
	var right: int = maxi(maxi(squad.width, column + unit.footprint_width), claimed["right"])
	var edge := squad.combat_width / 2.0 + 0.5
	var lateral := column - squad.width / 2.0 + squad.centre_shift
	return (
		right - left <= squad.combat_width
		and lateral >= -edge - EPSILON
		and lateral + unit.footprint_width <= edge + EPSILON
	)
