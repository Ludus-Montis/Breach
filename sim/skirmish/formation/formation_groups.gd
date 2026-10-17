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

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const FormationCommand = preload("res://sim/skirmish/formation/formation_command.gd")
const FormationContact = preload("res://sim/skirmish/formation/formation_contact.gd")
const FormationEvents = preload("res://sim/skirmish/formation/formation_events.gd")
const ScrumContest = preload("res://sim/skirmish/formation/scrum_contest.gd")
const ScrumReach = preload("res://sim/skirmish/formation/scrum_reach.gd")
const GroupSplit = preload("res://sim/skirmish/formation/group_split.gd")
const BodyGrid = preload("res://sim/skirmish/formation/body_grid.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")
const SquadEntries = preload("res://sim/skirmish/formation/squad_entries.gd")

const EPSILON := 0.000001
## Pairs of units two groups meeting may have and still be looked at pair by pair (_gap).
const ALL_PAIRS := 256
const AT_EASE := [
	SkirmishSquad.State.MOVING, SkirmishSquad.State.HOLDING, SkirmishSquad.State.ARRIVED
]


## One tick: groups split, then those meeting form up. New groups are numbered from
## `next_id` and added to `squads`; returns the next free squad id.
static func step(squads: Array, tick: int, fight_seed: int, events: Array, next_id: int) -> int:
	next_id = GroupSplit.step(squads, tick, fight_seed, events, next_id)
	_form_up(squads, tick, fight_seed, events)
	return next_id


static func _form_up(squads: Array, tick: int, fight_seed: int, events: Array) -> void:
	var meetings := []  # [key, one, other]
	var groups := squads.filter(func(s): return s.state in AT_EASE and not s.living().is_empty())
	var grids := {}  # squad -> a grid of its units (_gap), each made once
	for i in groups.size():
		for j in range(i + 1, groups.size()):
			var one: SkirmishSquad = groups[i]
			var other: SkirmishSquad = groups[j]
			if one.faction_id != other.faction_id or not _together(one, other):
				continue
			var gap := _gap(one, other, grids)
			if gap <= BattleTuning.current().group_join + EPSILON:
				var draws := [ScrumContest.squad_draw(one, fight_seed)]
				draws.append(ScrumContest.squad_draw(other, fight_seed))
				draws.sort()
				meetings.append([[snappedf(gap, 0.000001)] + draws, one, other])
	meetings.sort_custom(func(a, b): return a[0] < b[0])
	var gone := {}
	for meeting in meetings:
		if gone.has(meeting[1]) or gone.has(meeting[2]):
			continue
		var taken = _meet(meeting[1], meeting[2], fight_seed, tick, events)
		if taken != null:
			gone[taken] = true
			squads.erase(taken)


## Decides one meeting by the rule; returns the group taken in, or null.
static func _meet(
	one: SkirmishSquad, other: SkirmishSquad, fight_seed: int, tick: int, events: Array
):
	var taker := _taker(one, other, fight_seed)
	if taker == null:
		if _led(one) and _led(other):
			_return_origins(one, other, tick, events)
			_return_origins(other, one, tick, events)
		return null
	var taken: SkirmishSquad = other if taker == one else one
	_take_in(taker, taken, tick, events)
	return taken


## Which of two meeting groups takes in the other, or null if each keeps its own orders.
static func _taker(one: SkirmishSquad, other: SkirmishSquad, fight_seed: int) -> SkirmishSquad:
	if one.command == other.command:
		return _larger(one, other, fight_seed)
	var one_joins := _leader_has(one, "joins")
	var other_joins := _leader_has(other, "joins")
	if one_joins != other_joins:
		return other if one_joins else one  # its leader brings it into the other's command
	if one_joins:
		return _larger(one, other, fight_seed)
	var one_gathers := _leader_has(one, "gathers") and not _led(other)
	var other_gathers := _leader_has(other, "gathers") and not _led(one)
	if one_gathers != other_gathers:
		return one if one_gathers else other
	if not _led(one) and not _led(other) and _merging(one, other):
		return _larger(one, other, fight_seed)
	return null


## The taker takes in the other group's units behind its back rank; they follow its
## command from now on.
static func _take_in(taker: SkirmishSquad, taken: SkirmishSquad, tick: int, events: Array):
	events.append(FormationEvents.squad_event("merged", tick, taken, {"into": taker.id}))
	for unit in taken.living():
		SquadEntries.unloose(taken, unit.id)
	FormationContact.reinforce(taker, taken)
	taker.reforming = true


## Units of `group` that started under `home`'s command go back to it (both led).
static func _return_origins(
	group: SkirmishSquad, home: SkirmishSquad, tick: int, events: Array
) -> void:
	var back := group.living().filter(func(u): return u.origin == home.command)
	if back.is_empty() or back.size() == group.living().size():
		return
	# store: scattered (few: rejoining, once as a group comes home)
	for unit in back:
		group.units.erase(unit)
		UnitState.regather(group)
		SquadEntries.unloose(group, unit.id)
		home.store.columns.put(unit.slot, UnitColumns.RANK, _back_row(home))
		FormationCommand.enlist(home, unit)
		events.append(FormationEvents.unit_event("rejoined", tick, home, unit))
	home.reforming = true


## The larger of two groups by cells' worth of units; a tie by their seeded draws.
static func _larger(one: SkirmishSquad, other: SkirmishSquad, fight_seed: int) -> SkirmishSquad:
	var mine := [_worth(one), ScrumContest.squad_draw(one, fight_seed)]
	var theirs := [_worth(other), ScrumContest.squad_draw(other, fight_seed)]
	return one if mine > theirs else other


static func _worth(squad: SkirmishSquad) -> int:
	var cells := 0
	for unit in squad.living():
		cells += unit.footprint_width * unit.footprint_depth
	return cells


## True if the two groups may form up: one command's; or, of different commands, both
## having fought (coming back together after a fight, not two forces merely posted near
## each other) and either both standing or both marching the same way along the same
## route - not passing each other on ways of their own.
static func _together(one: SkirmishSquad, other: SkirmishSquad) -> bool:
	if one.command == other.command:
		return true
	if not (one.fought and other.fought):
		return false
	var standing := [SkirmishSquad.State.HOLDING, SkirmishSquad.State.ARRIVED]
	if one.state in standing and other.state in standing:
		return true
	return (
		one.route == other.route
		and one.direction == other.direction
		and one.order == other.order
		and one.order != SkirmishUnit.Order.HOLD
	)


## True if a leader of the squad (leadership > 0) has the trait.
static func _leader_has(squad: SkirmishSquad, trait_id: String) -> bool:
	return squad.living().any(func(u): return u.leadership > 0 and u.traits.has(trait_id))


static func _led(squad: SkirmishSquad) -> bool:
	return squad.living().any(func(u): return u.leadership > 0)


static func _merging(one: SkirmishSquad, other: SkirmishSquad) -> bool:
	return one.merges or other.merges


static func _back_row(squad: SkirmishSquad) -> int:
	var rows := 0
	var ranks: PackedInt32Array = UnitView.read(squad, [UnitColumns.RANK]).rank  # read once
	for unit in squad.living():
		rows = maxi(rows, ranks[unit.slot] + unit.footprint_depth)
	return rows


## Cells between the two groups' nearest bodies, exact up to group_join; beyond it, more
## than group_join (INF if their frames are too far apart). Small groups look at every
## pair; otherwise only the other's units that could come within group_join of each unit
## are looked at, through a grid of them (kept in `grids`).
static func _gap(one: SkirmishSquad, other: SkirmishSquad, grids: Dictionary) -> float:
	var join := BattleTuning.current().group_join
	var reach := one.width + other.width + _back_row(one) + _back_row(other)
	if one.position.distance_to(other.position) > reach + join:
		return INF
	var ours: Array = one.living()
	var theirs: Array = other.living()
	var at: PackedVector2Array = one.store.columns.column(UnitColumns.POSITION)
	var grid := {}
	if ours.size() * theirs.size() > ALL_PAIRS:
		if not grids.has(other):
			grids[other] = BodyGrid.of_bodies(
				theirs.map(func(u): return [at[u.slot], ScrumReach.radius(u)])
			)
		grid = grids[other]
	var least := INF
	for unit in ours:
		var near := range(theirs.size())
		if not grid.is_empty():
			var within: float = join + ScrumReach.radius(unit) + grid["widest"] + BodyGrid.MARGIN
			near = BodyGrid.near(grid, at[unit.slot], within)
		for found in near:
			var friend: SkirmishUnit = theirs[found]
			var apart: float = at[unit.slot].distance_to(at[friend.slot])
			least = minf(least, apart - ScrumReach.radius(unit) - ScrumReach.radius(friend))
	return maxf(least, 0.0)
