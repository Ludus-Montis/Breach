class_name FormationMelee
extends RefCounted
## This tick's melee blows (Decision 88): units in the scrum strike the foes they touch
## (ScrumBlows), and hostiles strike routers within reach from behind (RoutBlows).
## How each lands - high ground and flanks among what shifts it - is BlowLanding's.
## Pure over the squads it is given; FormationSimulation applies the blows.

const RoutBlows = preload("res://sim/skirmish/formation/rout_blows.gd")
const ScrumBlows = preload("res://sim/skirmish/formation/scrum_blows.gd")
const ScrumBlowsField = preload("res://sim/skirmish/formation/scrum_blows_field.gd")


## [[attacker, target, damage, flank], ...]; clears every living unit's target, then
## updates each striker's target and cooldown. `fight_seed` breaks ties between equal
## targets (Decision 97). Given the battle's native bodies (`field`, NativeKernels), the
## scrum's search runs on them (ScrumBlowsField).
static func blows(squads: Array, interval: int, fight_seed: int, field: Object = null) -> Array:
	var out := []
	if field != null:
		out = ScrumBlowsField.blows(field, squads, interval, fight_seed)
	else:
		ScrumBlows.clear_targets(squads)
		out = ScrumBlows.blows(squads, interval, fight_seed)
	out.append_array(RoutBlows.blows(squads, interval, fight_seed))
	return out
