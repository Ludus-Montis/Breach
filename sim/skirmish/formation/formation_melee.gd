class_name FormationMelee
extends RefCounted
## This tick's blows and shots (Decisions 46, 82 and 88): units in the scrum strike the foes
## they touch, hostiles strike routers within reach from behind, and ranged units shoot.
## Whom each strikes, its target and its cooldown are the crate's, in one pass over the
## squads' roster (`fight::choose`, spec 39 step 8b5-xvii): the scrum's search on the
## battle's native bodies (`blows.rs`), the blows on routers (`rout_blows.rs`) and the shots
## (`shots.rs`), each blow's damage read from its striker's sheet. How each lands - high
## ground and flanks among what shifts it - is BlowLanding's. Pure over the squads it is
## given. The whole fight is one crate pass (`fight`, `fight::step`, spec 39 step 8b5-xix):
## the choice, the landing, the hurts, "spat" and "hit" in the crate's log, the strikers'
## stamina and the surrender (spec 39 step 8b5-xx).

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const BlowLanding = preload("res://sim/skirmish/formation/blow_landing.gd")
const DetMath = preload("res://sim/skirmish/formation/det_math.gd")
const FormationTerrain = preload("res://sim/skirmish/formation/formation_terrain.gd")

## Numbers a unit the fight took takes in what the crate returns.
const TAKEN := 2


## The tick's fight over the squads, in the crate (`fight_step`): every living unit's target
## cleared, then the blows and shots chosen, each striker's target and cooldown set; each
## landed as BlowLanding says (with rolls if `rolls`), hurt, "spat" and "hit" in the crate's
## log (CrateEvents takes them), each striker's stamina spent, then the routers struck that
## surrender ("surrendered" and "destroyed" in the log, the squads' states and locks the
## crate's). `ticks` = [the ticks a second's strike takes, the seed that breaks ties
## between equal targets (Decision 97) and rolls the blows, the tick]. Returns the units it
## took, [[squad, unit], ...] in the order they gave up, whose state GDScript copies.
static func fight(
	squads: Array, ticks: Array, rolls: bool, terrain: FormationTerrain, field: Object
) -> Array:
	if squads.is_empty():
		return []
	var tuning := BattleTuning.current()
	var front := DetMath.cos(deg_to_rad(tuning.reach_front_arc_degrees)) - 0.000001
	var floats := PackedFloat64Array(
		[
			tuning.reach_contact,
			front,
			tuning.rout_strike_reach,
			tuning.stamina_blow,
			tuning.wounds_surrender,
		]
	)
	var ints := PackedInt64Array(ticks + [int(rolls)])
	var roster := CrateRoster.of(squads)
	var ground: Object = terrain.core if terrain != null else null
	var columns: UnitColumns = squads[0].store.columns
	var taken: PackedInt64Array = ColumnsLent.all(
		columns,
		func():
			return field.fight_step(
				roster, ints, floats, BandTuning.rules(), BlowLanding.tuning(), ground
			)
	)
	var units := []
	for at in range(0, taken.size(), TAKEN):
		var squad: SkirmishSquad = squads[taken[at]]
		units.append([squad, squad.living()[taken[at + 1]]])
	return units
