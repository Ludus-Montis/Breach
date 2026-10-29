class_name FormationGroups
extends RefCounted
## Groups of a command splitting and forming up (spec 30 round 3, part 4; the user's model
## of formations as commands). A squad is a group of one command's units standing together.
## - **Splitting:** units fallen further behind their places than walk_lost - where the
##   frame doesn't wait for them (FormationWalk) - go on as a group of their own, of the
##   same command and its orders: one group for each cluster of them within group_join of
##   one another.
## - **Forming up:** two friendly groups out of a fight whose units come within group_join
##   of each other - of one command; or of different commands, both having fought and
##   either both standing or marching the same way along the same route - form up by one
##   rule:
##   - of one command, they are one group again: the larger takes in the other;
##   - otherwise each keeps its own orders, unless a leader's trait says otherwise: a
##     leader that "gathers" takes in a leaderless group it meets; a leader that "joins"
##     brings its group into the other's command;
##   - two leaderless groups merge only if either is set to merge, the one with more
##     cells' worth of units taking in the other;
##   - two led groups each keep their own, and units each started under the other's
##     command go back to it.
##   A group taken in is "merged" into the taker. Units taken in follow the taker's
##   command from then on (FormationCommand.enlist).
## Every meeting is decided from one snapshot, nearest first; ties in size go to the
## groups' seeded draws, never to ids or the order they are listed in (Decision 97). Pure.
## - **Splitting**, the rest: a cluster split off is cut off from the rest, all its units
##   further from their places than walk_lost, unless no man is left behind. Each new group
##   follows the same command and orders along the same route, its front where its foremost
##   unit stands, its units lined up across it by where they stand, and is numbered in the
##   order of its units' seeded draws, never by ids or lists (Decision 97).
## Both are one crate pass (`groups/pass.rs`, spec 39 step 8b5-xxx): it splits the groups
## off, opening each new group's squad with the battle's next id, then lands every meeting,
## their events in its log. This hands it the squads and their routes, wraps each squad slot
## it opened, and mirrors its unit lists.

## What the crate's pass did, as it numbers it.
enum Landed { SPLIT, TAKEN, RETURNED }

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const MapLayoutDef = preload("res://content/definitions/map_layout_def.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const FormationCommand = preload("res://sim/skirmish/formation/formation_command.gd")
const FormationContact = preload("res://sim/skirmish/formation/formation_contact.gd")


## One tick: groups split, then those meeting form up. New groups are numbered from
## `next_id` and added to `squads`, groups taken in leave it; returns the next free squad id.
static func step(squads: Array, tick: int, fight_seed: int, events: Array, next_id: int) -> int:
	if squads.is_empty():
		return next_id
	var store: UnitStore = squads[0].store
	var livings := []  # each squad's living units as the crate is given them
	var routes := PackedInt64Array()
	var codes := {}  # route (or null) -> its code
	for squad: SkirmishSquad in squads:
		livings.append(squad.living())
		routes.append(codes.get_or_add(squad.route, codes.size()))
	var tuning := BattleTuning.current()
	var rules := PackedFloat64Array(
		[tuning.walk_lost, tuning.group_join, MapLayoutDef.CELLS_PER_TILE]
	)
	var field: Object = store.field()
	field.put_squads_issued(next_id - 1)
	var roster := CrateRoster.of(squads)
	var at := PackedInt64Array([tick, fight_seed])
	var out: PackedInt64Array = ColumnsLent.all(
		store.columns, func(): return field.groups_step(roster, routes, rules, at)
	)
	next_id = _mirrored(out, squads, livings, next_id)
	events.append_array(CrateEvents.taken(field, store))
	return next_id


## GDScript's lists after the crate's pass `out`: each split's group wrapped over the slot
## opened and listed, each group taken in out of `squads`, each unit gone back moved.
static func _mirrored(out: PackedInt64Array, squads: Array, livings: Array, next_id: int) -> int:
	var listed := squads.duplicate()  # as the crate numbers them: the splits after the rest
	var i := 0
	while i < out.size():
		if out[i] == Landed.TAKEN:
			FormationContact.took_in(listed[out[i + 1]], listed[out[i + 2]])
			squads.erase(listed[out[i + 2]])
			i += 3
			continue
		var count := int(out[i + 3])
		var picked := Array(out.slice(i + 4, i + 4 + count))
		if out[i] == Landed.SPLIT:
			var living: Array = livings[out[i + 1]]
			var members := picked.map(func(n): return living[n])
			var group := _split(listed[out[i + 1]], members, int(out[i + 2]), next_id)
			next_id += 1
			squads.append(group)
			listed.append(group)
		else:
			_returned(listed[out[i + 1]], listed[out[i + 2]], picked)
		i += 4 + count
	return next_id


## GDScript's squad `id` over the slot the crate opened for the units `cluster` (in the
## order they line up), which left `squad`: the same values assigned through its setters.
static func _split(squad: SkirmishSquad, cluster: Array, slot: int, id: int) -> SkirmishSquad:
	var members: Array[SkirmishUnit] = []
	for unit: SkirmishUnit in cluster:
		squad.units.erase(unit)
		members.append(unit)
	UnitState.regather(squad)
	var front: float = squad.store.squads.value(slot, SquadColumns.FRONT_DISTANCE)
	squad.store.squads.opened = slot
	var group := SkirmishSquad.new(
		id,
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


## The units of `group` with these `ids` (in its order) gone back to `home`'s command.
static func _returned(group: SkirmishSquad, home: SkirmishSquad, ids: Array) -> void:
	# store: scattered (few: rejoining, once as a group comes home)
	for id in ids:
		var unit: SkirmishUnit = group.units.filter(func(u): return u.id == id)[0]
		group.units.erase(unit)
		FormationCommand.enlist(home, unit)
	UnitState.regather(group)
