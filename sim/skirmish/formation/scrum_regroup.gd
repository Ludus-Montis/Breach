class_name ScrumRegroup
extends RefCounted
## Regrouping after a fight (Decisions 88 and 92, spec 27 rounds 5 and 6): units of squads
## out of the fight walk back to their places - in the squad's stance, if it holds one - at
## its re-form pace (FormationDiscipline), and rejoin its frame once in place and facing its
## way; one held off its place trades places or takes it (ScrumTrade, Decision 110). A
## drilled retreat's units back away facing the foe they touch (a fighting withdrawal,
## Decision 95). Every squad's units face the foes they touched as the phase began, so no
## squad listed earlier moves first and changes what a later one sees (Decision 97).
## Chasers and withdrawing units are left to ScrumPursuit and FormationWithdraw. Pure over
## the squads it is given.

const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const ScrumBlows = preload("res://sim/skirmish/formation/scrum_blows.gd")
const ScrumPursuit = preload("res://sim/skirmish/formation/scrum_pursuit.gd")
const ScrumStance = preload("res://sim/skirmish/formation/scrum_stance.gd")
const UnitMotion = preload("res://sim/skirmish/formation/unit_motion.gd")
const UnitShuffle = preload("res://sim/skirmish/formation/unit_shuffle.gd")
const FormationDiscipline = preload("res://sim/skirmish/formation/formation_discipline.gd")
const ScrumTrade = preload("res://sim/skirmish/formation/scrum_trade.gd")
const ScrumSlots = preload("res://sim/skirmish/formation/scrum_slots.gd")
const UnitSteer = preload("res://sim/skirmish/formation/unit_steer.gd")
const ScrumNear = preload("res://sim/skirmish/formation/scrum_near.gd")
const ScrumReach = preload("res://sim/skirmish/formation/scrum_reach.gd")
const BodyGrid = preload("res://sim/skirmish/formation/body_grid.gd")
const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const TerrainStep = preload("res://sim/skirmish/formation/terrain_step.gd")
const UnitMover = preload("res://sim/skirmish/formation/unit_mover.gd")
const FormationTerrain = preload("res://sim/skirmish/formation/formation_terrain.gd")
const SquadEntries = preload("res://sim/skirmish/formation/squad_entries.gd")
const EntryKeys = preload("res://sim/skirmish/formation/entry_keys.gd")
## The walking-back units' loose points and bearings, held for the pass (UnitView).
## Their places too (UnitView.PLACES): a squad's trade (ScrumTrade, in the crate) writes
## them as the walks go on, lent the hold (ColumnsLent), so every walk reads the last
## write. How long each has stalled is the crate's to note (ScrumTrade.track): not held.
const HELD := [
	UnitColumns.LOOSE_AT,
	UnitColumns.NEXT,
	UnitColumns.BEARING,
	UnitColumns.RANK,
	UnitColumns.COLUMN,
]


## Units of squads out of the fight walk to their places (in the stance, if any) at the
## march pace, paying the ground they cross (TerrainStep: one whose place is barred makes
## for the nearest it can walk to it); back in place they rejoin the squad's frame unless
## it holds a stance. A drilled retreat's units back away facing the foe each touched as
## the phase began.
static func step(
	squads: Array, pace: float, seconds: float, fight_seed: int, terrain: FormationTerrain = null
) -> void:
	var regrouping := []
	var hostile := {}  # faction -> the index of its foes, built the first time it's needed
	for squad in squads:
		if squad.state in [SkirmishSquad.State.FIGHTING, SkirmishSquad.State.ROUTING]:
			continue
		var held: bool = squad.loose.keys().any(func(id): return not squad.chasers.has(id))
		if squad.state == SkirmishSquad.State.MOVING and held:
			squad.state = SkirmishSquad.State.HOLDING  # it stands while its units regroup
		regrouping.append([squad, _foes_faced(squad, squads, fight_seed, hostile)])
	var walking := (
		regrouping
		. map(func(entry): return entry[0])
		. filter(func(squad): return not squad.loose.is_empty())
	)
	var moving := {"pace": pace, "seconds": seconds, "seed": fight_seed, "terrain": terrain}
	if not walking.is_empty():  # anyone to walk
		moving["bodies"] = ScrumSlots.bodies(squads)
		moving["crowd"] = BodyGrid.of_bodies(moving["bodies"])
	# each unit read and stepped: held once for every squad walking (nothing native runs
	# in the walks; a trade is lent the hold)
	var held := UnitView.hold(walking, HELD)
	moving["paces"] = {}  # store -> a read view of SPEED, which nothing here writes
	for entry in regrouping:
		_walk_back(entry[0], entry[1], moving)
		ScrumTrade.trade(entry[0], fight_seed)  # a unit its own ranks hold off trades places
	UnitView.release(held)


## unit id -> where the foe it touches stands, for a drilled retreat's units ({} for any
## other squad). `hostile` (faction -> ScrumNear.index of its foes) keeps each faction's
## index for the pass: every squad of a faction has the same foes (ScrumBlows.hostile_units).
static func _foes_faced(
	squad: SkirmishSquad, squads: Array, fight_seed: int, hostile: Dictionary
) -> Dictionary:
	var out := {}
	if squad.order != SkirmishUnit.Order.RETREAT or not FormationDiscipline.meets_threats(squad):
		return out
	if not hostile.has(squad.faction_id):
		hostile[squad.faction_id] = ScrumNear.index(ScrumBlows.hostile_units(squad, squads))
	var near: Dictionary = hostile[squad.faction_id]
	var reach := BattleTuning.current().reach_contact
	for unit_id in squad.loose:
		var unit: SkirmishUnit = squad.loose[unit_id]
		var where := ScrumReach.at(squad, unit)
		var close := ScrumNear.around(near, where, ScrumReach.radius(unit) + reach)
		out[unit_id] = ScrumBlows.nearest_touching(squad, unit, close, fight_seed)
	return out


## `moving` = {pace, seconds, seed, terrain, bodies, crowd, paces}: they step round bodies
## in their way (UnitSteer). Its store is held (`step`).
static func _walk_back(squad: SkirmishSquad, foes: Dictionary, moving: Dictionary) -> void:
	if squad.loose.is_empty():
		return
	var seconds: float = moving["seconds"]
	var reform := FormationDiscipline.reform_pace(squad)  # the same for each of its units
	var homes := TerrainStep.returns(squad, moving["terrain"], moving["seed"])
	var held := UnitView.of(squad)  # once per squad: an index, not a copy
	var columns := squad.store.columns  # the hold's, for the helpers that step a unit
	var loose_at: PackedVector2Array = held.at
	var next: PackedVector2Array = held.next
	var bearing: PackedFloat64Array = held.bearing
	var paces: PackedFloat64Array = (
		UnitView.read_once(moving["paces"], squad, [UnitColumns.SPEED]).speed
	)
	var basis := ScrumStance.frame(squad)  # once a squad: no unit's walk moves its frame
	var heading: float = squad.stance.get("heading", squad.heading)
	for unit_id in squad.loose.keys():
		if ScrumPursuit.away(squad, unit_id):
			continue
		var unit: SkirmishUnit = squad.loose[unit_id]
		var place: Vector2 = homes.get(unit_id, ScrumStance.anchor(basis, unit, held))
		var step: float = paces[unit.slot] * moving["pace"] * reform
		var foe_at = foes.get(unit_id)
		var at: Vector2 = loose_at[unit.slot]
		if foe_at == null:  # it takes its place the quicker way, arriving facing its squad's
			var facing: float = bearing[unit.slot]
			foe_at = UnitShuffle.look(unit, at, facing, place, heading, step / seconds)
		var gap: float = at.distance_to(place)
		ScrumTrade.track(columns, unit.slot, gap, seconds)
		var arrived := gap < 0.000001
		if gap >= 0.000001:  # a drilled retreat backs away facing the foe it touches
			var to := UnitSteer.toward(
				unit, at, place, moving["bodies"], moving["seed"], moving["crowd"]
			)
			at = UnitMover.step(unit, columns, at, to, step, seconds, moving["terrain"], foe_at)
			loose_at[unit.slot] = at  # placement: its step, by UnitMover (Decision 139)
		next[unit.slot] = at
		EntryKeys.set_goal(columns, unit.slot, null)
		var faced := arrived and UnitMotion.turn(unit, columns, heading, seconds)
		if squad.stance.is_empty() and faced:
			SquadEntries.unloose(squad, unit_id)  # in its place and facing the squad's way
