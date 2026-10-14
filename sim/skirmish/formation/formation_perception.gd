class_name FormationPerception
extends RefCounted
## What each formation sees at the start of a tick (Decision 134; spec 34, item 2): the
## standing hostile formations it detects (FormationSight: within its units' detection
## range and in sight), and whether it is **threatened** - it sees one. Gathered once a
## tick, before anything moves, and read by the passes that decide (taking the downed now;
## seeking, rescuers and the wits beat to come), so each decides on the same picture and
## none searches again. A pair too far apart for either's widest sight to reach is skipped
## before any unit is looked at, so the gather grows with the formations near each other,
## not with every pair. Pure.

const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const FormationSight = preload("res://sim/skirmish/formation/formation_sight.gd")
const FormationTerrain = preload("res://sim/skirmish/formation/formation_terrain.gd")
const SquadEdges = preload("res://sim/skirmish/formation/squad_edges.gd")
const SquadMemo = preload("res://sim/skirmish/formation/squad_memo.gd")

const UNSEEN := [SkirmishSquad.State.ROUTING, SkirmishSquad.State.DESTROYED]


## squad id -> {"seen": [the standing hostile squads it detects], "threat": true if any}.
static func see(squads: Array, terrain: FormationTerrain = null) -> Dictionary:
	var memo := SquadMemo.new()
	var bounds := {}  # squad -> its living units' bounds
	var reach := {}  # squad -> those grown by its widest detection
	for squad in squads:
		bounds[squad] = SquadEdges.bounds(squad)
		reach[squad] = bounds[squad].grow(_widest(squad) + 1.0)
	var out := {}
	for squad in squads:
		var seen := []
		for other in squads:
			if other.faction_id == squad.faction_id or other.state in UNSEEN:
				continue
			if not reach[squad].intersects(bounds[other]):
				continue  # beyond its sight wherever its units stand
			if FormationSight.detects(squad, other, terrain, memo):
				seen.append(other)
		out[squad.id] = {"seen": seen, "threat": not seen.is_empty()}
	return out


## True if `squad` was threatened in `perception` (FormationPerception.see), false if it
## wasn't or isn't in it.
static func threatened(perception: Dictionary, squad: SkirmishSquad) -> bool:
	return perception.get(squad.id, {}).get("threat", false)


static func _widest(squad: SkirmishSquad) -> float:
	var widest := 0.0
	for unit in squad.living():
		widest = maxf(widest, unit.detection)
	return widest
