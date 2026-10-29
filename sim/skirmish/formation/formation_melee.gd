class_name FormationMelee
extends RefCounted
## This tick's blows and shots (Decisions 46, 82 and 88): units in the scrum strike the foes
## they touch, hostiles strike routers within reach from behind, and ranged units shoot.
## Whom each strikes, its target and its cooldown are the crate's, in one pass over the
## squads' roster (`fight::choose`, spec 39 step 8b5-xvii): the scrum's search on the
## battle's native bodies (`blows.rs`), the blows on routers (`rout_blows.rs`) and the shots
## (`shots.rs`), each blow's damage read from its striker's sheet. How each lands - high
## ground and flanks among what shifts it - is BlowLanding's. Pure over the squads it is
## given; FormationSimulation applies the blows.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const DetMath = preload("res://sim/skirmish/formation/det_math.gd")

## Numbers a melee blow and a shot take in what the crate returns.
const MELEE := 6
const SHOT := 5


## [the melee blows ([[attacker, target, damage, flank], ...]), the shots ([[shooter,
## target, damage, its squad], ...]), the squads' CrateRoster they were chosen over (for
## BlowLanding)]; clears every living unit's target, then sets each striker's target and
## cooldown. `interval`: the ticks a second's strike takes;
## `fight_seed` breaks ties between equal targets (Decision 97). The scrum's search runs on
## the battle's native bodies (`field`, NativeKernels; without one, its squads' store's).
static func blows(squads: Array, interval: int, fight_seed: int, field: Object = null) -> Array:
	if squads.is_empty():
		return [[], []]
	if field == null:
		field = squads[0].store.field()
	var tuning := BattleTuning.current()
	var front := DetMath.cos(deg_to_rad(tuning.reach_front_arc_degrees)) - 0.000001
	var floats := PackedFloat64Array([tuning.reach_contact, front, tuning.rout_strike_reach])
	var roster := CrateRoster.of(squads)
	var chosen: PackedInt64Array = field.fight_choose(
		roster, PackedInt64Array([interval, fight_seed]), floats, BandTuning.rules()
	)
	var livings := squads.map(func(squad): return squad.living())
	var melee := []
	var at := 2
	for _blow in chosen[1]:
		var striker: SkirmishUnit = livings[chosen[at]][chosen[at + 1]]
		var target: SkirmishUnit = livings[chosen[at + 2]][chosen[at + 3]]
		melee.append([striker, target, chosen[at + 4], chosen[at + 5] != 0])
		at += MELEE
	var shots := []
	for _shot in (chosen.size() - at) / SHOT:
		var shooter: SkirmishUnit = livings[chosen[at]][chosen[at + 1]]
		var target: SkirmishUnit = livings[chosen[at + 2]][chosen[at + 3]]
		shots.append([shooter, target, chosen[at + 4], squads[chosen[at]]])
		at += SHOT
	return [melee, shots, roster]
