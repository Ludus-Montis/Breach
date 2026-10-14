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


## Units of squads out of the fight walk to their places (in the stance, if any) at the
## march pace, paying the ground they cross (TerrainStep: one whose place is barred makes
## for the nearest it can walk to it); back in place they rejoin the squad's frame unless
## it holds a stance. A drilled retreat's units back away facing the foe each touched as
## the phase began.
static func step(
	squads: Array, pace: float, seconds: float, fight_seed: int, terrain: FormationTerrain = null
) -> void:
	var regrouping := []
	for squad in squads:
		if squad.state in [SkirmishSquad.State.FIGHTING, SkirmishSquad.State.ROUTING]:
			continue
		var held: bool = squad.loose.keys().any(func(id): return not squad.chasers.has(id))
		if squad.state == SkirmishSquad.State.MOVING and held:
			squad.state = SkirmishSquad.State.HOLDING  # it stands while its units regroup
		regrouping.append([squad, _foes_faced(squad, squads, fight_seed)])
	var moving := {"pace": pace, "seconds": seconds, "seed": fight_seed, "terrain": terrain}
	if regrouping.any(func(entry): return not entry[0].loose.is_empty()):  # anyone to walk
		moving["bodies"] = ScrumSlots.bodies(squads)
		moving["crowd"] = BodyGrid.of_bodies(moving["bodies"])
	for entry in regrouping:
		_walk_back(entry[0], entry[1], moving)
		ScrumTrade.trade(entry[0], fight_seed)  # a unit its own ranks hold off trades places


## unit id -> where the foe it touches stands, for a drilled retreat's units ({} for any
## other squad).
static func _foes_faced(squad: SkirmishSquad, squads: Array, fight_seed: int) -> Dictionary:
	var out := {}
	if squad.order != SkirmishUnit.Order.RETREAT or not FormationDiscipline.meets_threats(squad):
		return out
	var near := ScrumNear.index(ScrumBlows.hostile_units(squad, squads))
	var reach := BattleTuning.current().reach_contact
	for unit_id in squad.loose:
		var unit: SkirmishUnit = squad.loose[unit_id]["unit"]
		var where := ScrumReach.at(squad, unit)
		var close := ScrumNear.around(near, where, ScrumReach.radius(unit) + reach)
		out[unit_id] = ScrumBlows.nearest_touching(squad, unit, close, fight_seed)
	return out


## `moving` = {pace, seconds, seed, terrain, bodies, crowd}: they step round bodies in their
## way (UnitSteer).
static func _walk_back(squad: SkirmishSquad, foes: Dictionary, moving: Dictionary) -> void:
	if squad.loose.is_empty():
		return
	var seconds: float = moving["seconds"]
	var reform := FormationDiscipline.reform_pace(squad)  # the same for each of its units
	var homes := TerrainStep.returns(squad, moving["terrain"], moving["seed"])
	for unit_id in squad.loose.keys():
		if ScrumPursuit.away(squad, unit_id):
			continue
		var entry: Dictionary = squad.loose[unit_id]
		var unit: SkirmishUnit = entry["unit"]
		var place: Vector2 = homes.get(unit_id, ScrumStance.anchor(squad, unit))
		var step: float = unit.speed * moving["pace"] * reform
		var heading: float = squad.stance.get("heading", squad.heading)
		var foe_at = foes.get(unit_id)
		if foe_at == null:  # it takes its place the quicker way, arriving facing its squad's
			foe_at = UnitShuffle.look(unit, entry["at"], place, heading, step / seconds)
		var gap: float = entry["at"].distance_to(place)
		ScrumTrade.track(entry, gap, seconds)
		var arrived := gap < 0.000001
		if gap >= 0.000001:  # a drilled retreat backs away facing the foe it touches
			var to := UnitSteer.toward(
				unit, entry["at"], place, moving["bodies"], moving["seed"], moving["crowd"]
			)
			entry["at"] = UnitMover.step(
				unit, entry["at"], to, step, seconds, moving["terrain"], foe_at
			)
		entry["next"] = entry["at"]
		entry["goal"] = null
		var faced := arrived and UnitMotion.turn(unit, heading, seconds)
		if squad.stance.is_empty() and faced:
			squad.loose.erase(unit_id)  # in its place and facing the squad's way
