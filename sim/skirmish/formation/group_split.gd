class_name GroupSplit
extends RefCounted
## A group's units fallen behind go on as a group of their own (spec 30 round 3, part 4):
## a cluster of the group's units (each within group_join of another) cut off from the
## rest, all further from their places than walk_lost - the frame no longer waits for them
## (FormationWalk), unless no man is left behind - splits off as a group. Each new group
## follows the same command and orders, along the same route, its front where its foremost
## unit stands;
## its units line up across it by where they stand. New groups are numbered in the order
## of their units' seeded draws, never by ids or lists (Decision 97). Pure. The decisions are
## the crate's (`split.rs`); this makes the new groups.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const FormationEvents = preload("res://sim/skirmish/formation/formation_events.gd")
const FormationWalk = preload("res://sim/skirmish/formation/formation_walk.gd")
const FormationShuffle = preload("res://sim/skirmish/formation/formation_shuffle.gd")
const MapLayoutDef = preload("res://content/definitions/map_layout_def.gd")

const AT_EASE := [
	SkirmishSquad.State.MOVING, SkirmishSquad.State.HOLDING, SkirmishSquad.State.ARRIVED
]


## Splits off every group's fallen-behind units; new groups are numbered from `next_id`
## and added to `squads`. Returns the next free squad id. Which units split off, in what
## order and where each new group's front is are the crate's (`split.rs`, spec 39 step 5a):
## this hands it the squads at ease on a route (CrateRoster), whether each waits for every
## unit, its route, and who is out taking and how far through a swap each unit is.
static func step(squads: Array, tick: int, fight_seed: int, events: Array, next_id: int) -> int:
	var candidates := squads.filter(func(s): return s.state in AT_EASE and s.route != null)
	if candidates.is_empty():
		return next_id
	var livings := []  # each candidate's living units as the crate is given them
	var listed := PackedInt32Array()
	var taking := PackedByteArray()
	var offsets := PackedVector2Array()
	for index in candidates.size():
		var squad: SkirmishSquad = candidates[index]
		var waits := FormationWalk.rule_of(squad) == "no_man_left_behind"
		listed.append_array([index, int(waits)])
		var living := squad.living()
		livings.append(living)
		var moving := FormationShuffle.movers(squad)
		var out_taking := squad.taking
		for unit in living:
			taking.append(int(out_taking.has(unit.id)))
			offsets.append(FormationShuffle.offset(squad, unit, moving))
	var tuning := BattleTuning.current()
	var rules := PackedFloat64Array(
		[tuning.walk_lost, tuning.group_join, MapLayoutDef.CELLS_PER_TILE]
	)
	var out: PackedFloat64Array = squads[0].store.field().group_split(
		CrateRoster.of(candidates), listed, taking, offsets, rules, fight_seed
	)
	var i := 0
	while i < out.size():  # no split moves a unit: each was decided on where they stood
		var source: SkirmishSquad = candidates[int(out[i])]
		var living: Array = livings[int(out[i])]
		var members := []
		for k in int(out[i + 2]):
			members.append(living[int(out[i + 3 + k])])
		var group := _group(source, members, out[i + 1])
		i += 3 + members.size()
		group.id = next_id
		for unit in group.units:
			unit.squad_id = next_id
		next_id += 1
		squads.append(group)
		events.append(FormationEvents.squad_event("split", tick, group, {"from": source.id}))
	return next_id


## A new group of `cluster` (in the order it lines up), taken out of `squad`: the same
## command, route and orders, its front `front` tiles along the route.
static func _group(squad: SkirmishSquad, cluster: Array, front: float) -> SkirmishSquad:
	var members: Array[SkirmishUnit] = []
	# store: scattered (few: a split, once as a group breaks off)
	for unit: SkirmishUnit in cluster:
		squad.units.erase(unit)
		UnitState.regather(squad)
		SquadEntries.unloose(squad, unit.id)
		SquadEntries.unflee(squad, unit.id)
		UnitPlace.put(squad, unit, Vector2i(0, members.size()))
		members.append(unit)
	var group := SkirmishSquad.new(
		0,
		squad.faction_id,
		squad.direction,
		squad.home_distance,
		members.size(),
		members,
		squad.command,
		squad.store
	)
	group.route = squad.route
	group.front_distance = front
	group.heading = squad.heading
	group.order = squad.order
	group.state = squad.state
	group.morale = squad.morale
	return group
