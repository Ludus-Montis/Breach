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
## Which groups meet and what each meeting does are the crate's (`groups.rs`, spec 39 step
## 5a): this hands it the groups at ease (CrateRoster), their commands and routes, and each
## unit's leading, its traits and the command it started under, and lands each meeting in
## the order given: a group taken in, or units going back where they started.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const FormationCommand = preload("res://sim/skirmish/formation/formation_command.gd")
const FormationContact = preload("res://sim/skirmish/formation/formation_contact.gd")
const FormationEvents = preload("res://sim/skirmish/formation/formation_events.gd")
const GroupSplit = preload("res://sim/skirmish/formation/group_split.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")
const SquadEntries = preload("res://sim/skirmish/formation/squad_entries.gd")

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
	var groups := squads.filter(func(s): return s.state in AT_EASE and not s.living().is_empty())
	if groups.size() < 2:
		return
	var commands := {}  # command -> its code
	var routes := {}  # route (or null) -> its code
	var described := PackedInt32Array()
	var followers := PackedInt32Array()
	var livings := []  # each group's living units as the crate is given them
	for group: SkirmishSquad in groups:
		var command: int = commands.get_or_add(group.command, commands.size())
		described.append_array([command, routes.get_or_add(group.route, routes.size())])
		var living := group.living()
		livings.append(living)
		for unit in living:
			var traits := int(unit.leadership > 0)
			traits |= (
				(2 if unit.traits.has("joins") else 0) | (4 if unit.traits.has("gathers") else 0)
			)
			var origin: int = (
				-1 if unit.origin == null else commands.get_or_add(unit.origin, commands.size())
			)
			followers.append_array([traits, origin])
	var out: PackedInt64Array = squads[0].store.field().group_form_up(
		CrateRoster.of(groups), described, followers, BattleTuning.current().group_join, fight_seed
	)
	var i := 0
	while i < out.size():
		if out[i] == 0:
			var taken: SkirmishSquad = groups[out[i + 2]]
			_take_in(groups[out[i + 1]], taken, tick, events)
			squads.erase(taken)
			i += 3
			continue
		var ids := Array(out.slice(i + 4, i + 4 + out[i + 3]))
		_return_origins(groups[out[i + 1]], groups[out[i + 2]], ids, tick, events)
		i += 4 + ids.size()


## The taker takes in the other group's units behind its back rank; they follow its
## command from now on.
static func _take_in(taker: SkirmishSquad, taken: SkirmishSquad, tick: int, events: Array):
	events.append(FormationEvents.squad_event("merged", tick, taken, {"into": taker.id}))
	for unit in taken.living():
		SquadEntries.unloose(taken, unit.id)
	FormationContact.reinforce(taker, taken)
	taker.reforming = true


## The units of `group` with these ids, started under `home`'s command, go back to it.
static func _return_origins(
	group: SkirmishSquad, home: SkirmishSquad, ids: Array, tick: int, events: Array
) -> void:
	var back := group.living().filter(func(u): return ids.has(u.id))
	# store: scattered (few: rejoining, once as a group comes home)
	for unit in back:
		group.units.erase(unit)
		UnitState.regather(group)
		SquadEntries.unloose(group, unit.id)
		home.store.columns.put(unit.slot, UnitColumns.RANK, _back_row(home))
		FormationCommand.enlist(home, unit)
		events.append(FormationEvents.unit_event("rejoined", tick, home, unit))
	home.reforming = true


static func _back_row(squad: SkirmishSquad) -> int:
	var ranks: PackedInt32Array = UnitView.read(squad, [UnitColumns.RANK]).rank  # read once
	var rows := 0
	for unit in squad.living():
		rows = maxi(rows, ranks[unit.slot] + unit.footprint_depth)
	return rows
