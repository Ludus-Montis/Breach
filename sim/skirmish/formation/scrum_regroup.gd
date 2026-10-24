class_name ScrumRegroup
extends RefCounted
## Regrouping after a fight (Decisions 88 and 92, spec 27 rounds 5 and 6): units of squads
## out of the fight walk back to their places - in the squad's stance, if it holds one - at
## its re-form pace (FormationDiscipline), and rejoin its frame once in place and facing its
## way; one held off its place trades places or takes it (`trade.rs`, Decision 110). A
## drilled retreat's units back away facing the foe they touch (a fighting withdrawal,
## Decision 95). Every squad's units face the foes they touched as the phase began, so no
## squad listed earlier moves first and changes what a later one sees (Decision 97).
## Chasers and withdrawing units are left to ScrumPursuit and FormationWithdraw. Pure over
## the squads it is given. The rules are the crate's (`regroup.rs`, spec 39 step 4c): this
## hands it the squads' roster (CrateRoster), each one's re-form pace and whether it is a
## drilled retreat (FormationDiscipline), whether it is withdrawing, and where its loose
## units walk back to on the ground (TerrainStep.returns).

const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const FormationDiscipline = preload("res://sim/skirmish/formation/formation_discipline.gd")
const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const TerrainStep = preload("res://sim/skirmish/formation/terrain_step.gd")
const UnitMover = preload("res://sim/skirmish/formation/unit_mover.gd")
const FormationTerrain = preload("res://sim/skirmish/formation/formation_terrain.gd")


## Units of squads out of the fight walk to their places (in the stance, if any) at the
## march pace, paying the ground they cross (TerrainStep: one whose place is barred makes
## for the nearest it can walk to it); back in place they rejoin the squad's frame unless
## it holds a stance. A drilled retreat's units back away facing the foe each touched as
## the phase began. `roster`: the tick's (CrateRoster), made here if none is given.
static func step(
	squads: Array,
	pace: float,
	seconds: float,
	fight_seed: int,
	terrain: FormationTerrain = null,
	roster: Object = null
) -> void:
	if squads.is_empty():
		return
	var listed := PackedInt32Array()
	var flags := PackedByteArray()
	var reforms := PackedFloat64Array()
	var counts := PackedInt32Array()
	var homes := PackedInt64Array()
	var home_at := PackedVector2Array()
	for index in squads.size():
		var squad: SkirmishSquad = squads[index]
		if squad.state in [SkirmishSquad.State.FIGHTING, SkirmishSquad.State.ROUTING]:
			continue
		listed.append(index)
		var loose: bool = not squad.loose.is_empty()  # only a squad with units out walks
		var drilled: bool = (
			loose
			and squad.order == SkirmishUnit.Order.RETREAT
			and FormationDiscipline.meets_threats(squad)
		)
		flags.append(int(drilled) | (2 if not squad.withdraw.is_empty() else 0))
		reforms.append(FormationDiscipline.reform_pace(squad) if loose else 0.0)
		var returns := TerrainStep.returns(squad, terrain, fight_seed) if loose else {}
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
			pace,
			seconds,
			tuning.reach_contact,
			tuning.reach_front_arc_degrees,
			tuning.bodies_steer_look,
			tuning.bodies_steer_clear,
			tuning.scrum_trade_seconds,
		]
	)
	squads[0].store.field().regroup(
		roster,
		listed,
		flags,
		reforms,
		counts,
		homes,
		home_at,
		UnitMover.ground(terrain),
		rules,
		fight_seed
	)
