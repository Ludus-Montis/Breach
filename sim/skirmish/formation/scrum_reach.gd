class_name ScrumReach
extends RefCounted
## Who touches whom in the scrum (Decisions 88 and 106, spec 30): a unit's body is the circle in its
## footprint, and it reaches an enemy whose body comes within a touch of its own. A unit's front is
## the arc within some degrees of its bearing; a blow from anywhere else is a flank blow. Both are
## BattleTuning's (`reach_contact`, `reach_front_arc_degrees`). Pure; cells.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const UnitMotion = preload("res://sim/skirmish/formation/unit_motion.gd")
const DetMath = preload("res://sim/skirmish/formation/det_math.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")
const SquadEntries = preload("res://sim/skirmish/formation/squad_entries.gd")
## The columns where a unit stands is read from (at_in): its point, and its loose point.
const STANDING := [UnitColumns.POSITION, UnitColumns.LOOSE_AT]


## Where the unit stands: its place in the scrum, or where its squad puts it.
static func at(squad: SkirmishSquad, unit: SkirmishUnit) -> Vector2:
	if squad.loose.has(unit.id):
		return SquadEntries.loose_at(squad, unit)
	return squad.store.columns.value(unit.slot, UnitColumns.POSITION)


## `at` from a view of the squad's store holding STANDING: an index, not a call.
static func at_in(view: UnitView, squad: SkirmishSquad, unit: SkirmishUnit) -> Vector2:
	if squad.loose.has(unit.id):
		return view.at[unit.slot]
	return view.position[unit.slot]


## The cells the unit covers, centred where it stands, turned to its squad's `heading`, read
## once a pass (the box round its turned footprint).
static func area(squad: SkirmishSquad, unit: SkirmishUnit, heading: float) -> Rect2:
	var ahead := UnitMotion.vector(heading).abs()
	var right := Vector2(ahead.y, ahead.x)
	var size := right * unit.footprint_width + ahead * unit.footprint_depth
	return Rect2(at(squad, unit) - size / 2.0, size)


## The radius of the unit's body: the circle in its footprint (a grem 0.5, a brute 1).
static func radius(unit: SkirmishUnit) -> float:
	return minf(unit.footprint_width, unit.footprint_depth) / 2.0


## Cells between two units' bodies (0 where they touch or overlap).
static func gap(squad: SkirmishSquad, unit: SkirmishUnit, other: SkirmishSquad, foe) -> float:
	return gap_at(at(squad, unit), unit, at(other, foe), foe)


## `gap` for two units standing at `here` and `there`.
static func gap_at(here: Vector2, unit: SkirmishUnit, there: Vector2, foe) -> float:
	return maxf(here.distance_to(there) - radius(unit) - radius(foe), 0.0)


## True if the two units' bodies touch: within reach_contact.
static func touching(
	squad: SkirmishSquad, unit: SkirmishUnit, other: SkirmishSquad, foe: SkirmishUnit
) -> bool:
	return gap(squad, unit, other, foe) <= BattleTuning.current().reach_contact


## True if `point` lies in the front of a unit at `from` on `bearing` (UnitMotion).
static func in_front(bearing: float, from: Vector2, point: Vector2) -> bool:
	var direction := point - from
	if direction.length() < 0.000001:
		return true
	return (
		direction.normalized().dot(UnitMotion.vector(bearing))
		>= DetMath.cos(deg_to_rad(BattleTuning.current().reach_front_arc_degrees)) - 0.000001
	)
