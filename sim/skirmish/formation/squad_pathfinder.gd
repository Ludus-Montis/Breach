class_name SquadPathfinder
extends RefCounted
## Who finds the way for a group (spec 30): one pathfinder per squad, the living unit with
## the best wits, whoever leads it - what the way is for still follows the leader's and
## units' behaviour. Equal wits go to the battle's seeded draw (ScrumContest.draw), never
## to ids or the order the units are listed in (Decision 97). A view over the crate's
## (native/rust/src/pathing.rs, spec 39 step 3c). Pure.

const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")


## The squad's pathfinder in the battle seeded `fight_seed`; null with none standing.
static func pick(squad: SkirmishSquad, fight_seed: int) -> SkirmishUnit:
	var living := squad.living()
	var index: int = CratePaths.core().pathfinder(CratePaths.members(squad), fight_seed)
	return null if index < 0 else living[index]
