class_name FormationProduction
extends RefCounted
## One lane's wave (Decisions 39, 42 and 45, specs/22-formation-feel-test.md): its painted
## WaveTemplate and which places are filled. The domain's builders and reserve fill it
## (fill, front-first); a new template takes effect at once - filled units fold into
## matching places (same unit type, front-first) and the rest are handed back for the
## domain reserve. "wave_full" is emitted once when every place is filled; the wave
## departs as one squad, manually (send()) or automatically. Never pauses itself. With
## auto_merge, the squads it sends merge into friendly squads they catch up with (Decision
## 51).

enum Departure { MANUAL, AUTO_WHEN_FULL }

const FormationMarch = preload("res://sim/skirmish/formation/formation_march.gd")
const SquadRanks = preload("res://sim/skirmish/formation/squad_ranks.gd")
const WaveTemplate = preload("res://sim/skirmish/formation/wave_template.gd")
const FormationSimulation = preload("res://sim/skirmish/formation/formation_simulation.gd")
const SkirmishSquad = preload("res://sim/skirmish/formation/skirmish_squad.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")
const FormationRoute = preload("res://sim/skirmish/formation/formation_route.gd")

var faction_id: String
var at_player_end: bool
var departure: Departure = Departure.MANUAL
var auto_merge := false
## The route its waves follow (Decision 75); null keeps the simulation's lane.
var route: FormationRoute = null
## A hold-until order its waves carry (FormationStaging, Decision 87); empty for none.
var staging := {}

var _template := WaveTemplate.new(1, 0)
var _filled := []  # one bool per _template.ordered() place
var _announced := false


func _init(faction: String, player_end: bool) -> void:
	faction_id = faction
	at_player_end = player_end


## Switches to a new template now: filled units fold into matching places. Returns a
## "folded" event; its "leftovers" are the units that no longer fit, for the reserve.
func set_template(template: WaveTemplate) -> Array:
	var filled_defs := []
	var places := _template.ordered()
	for index in range(places.size()):
		if _filled[index]:
			filled_defs.append(places[index][0])
	_template = template
	_filled = []
	var kept := 0
	for place in _template.ordered():
		var match_at := filled_defs.find(place[0])
		_filled.append(match_at != -1)
		if match_at != -1:
			filled_defs.remove_at(match_at)
			kept += 1
	_announced = _is_full() and _announced
	var folded := {"type": "folded", "faction": faction_id, "kept": kept}
	folded.merge({"banked": filled_defs.size(), "leftovers": filled_defs})
	return [folded]


## Unfilled places by unit type: {UnitDef: count}.
func wanted() -> Dictionary:
	var counts := {}
	var places := _template.ordered()
	for index in range(places.size()):
		if not _filled[index]:
			counts[places[index][0]] = counts.get(places[index][0], 0) + 1
	return counts


## Fills the front-most unfilled place of this type; false if there is none.
func fill(unit_def: UnitDef) -> bool:
	var places := _template.ordered()
	for index in range(places.size()):
		if not _filled[index] and places[index][0] == unit_def:
			_filled[index] = true
			return true
	return false


## Announces a full wave once, and sends it if departure is automatic.
func step(sim: FormationSimulation) -> Array:
	var events := []
	if _is_full() and not _announced:
		_announced = true
		events.append(_event("wave_full", sim, {"built": built()}))
		if departure == Departure.AUTO_WHEN_FULL:
			send(sim)
			events.append(_event("departed", sim, {"units": events[-1]["built"]}))
	return events


## Deploys the filled places as one squad (the painted layout; a partial wave closes up
## into a solid block as wide as its built front band, SquadRanks), setting out after
## `wait_ticks` (a planned rendezvous, Decision 87), and restarts the same template, empty;
## null when nothing is filled.
func send(sim: FormationSimulation, wait_ticks: int = 0) -> SkirmishSquad:
	if built() == 0:
		return null
	var layout := _template.layout()
	var placements := []
	for index in range(_filled.size()):
		if _filled[index]:
			placements.append(layout[1][index])
	var squad := sim.spawn_squad(
		layout[0], placements, faction_id, at_player_end, wait_ticks, route
	)
	squad.merges = auto_merge
	squad.staging = staging.duplicate()
	if not _is_full():  # its unbuilt places are holes: it closes up, as over its dead
		var front: Array = squad.units.filter(func(u): return u.preferred_position == 0)
		var span: int = front.reduce(func(total, u): return total + u.footprint_width, 0)
		squad.width = clampi(span, 1, squad.width)
		SquadRanks.close(squad)
	_centre(squad)
	FormationMarch.sync_units([squad])  # placed as it is sent: its units stand on their places
	_filled.fill(false)
	_announced = false
	return squad


func built() -> int:
	return _filled.count(true)


## [[rank, column, depth, width, filled, UnitDef], ...] for every template place.
func preview() -> Array:
	var places := _template.ordered()
	var out := []
	for index in range(places.size()):
		var unit_def: UnitDef = places[index][0]
		var at: Vector2i = places[index][1]
		out.append(
			[
				at.x,
				at.y,
				unit_def.footprint_depth,
				unit_def.footprint_width,
				_filled[index],
				unit_def
			]
		)
	return out


func _is_full() -> bool:
	return not _filled.is_empty() and not _filled.has(false)


func _event(kind: String, sim: FormationSimulation, extra: Dictionary) -> Dictionary:
	var event := {"type": kind, "tick": sim.tick_number(), "faction": faction_id}
	event.merge(extra)
	return event


## A partial wave keeps its painted columns: shifts it so its units stand centred on the
## route, not off to one side of it.
static func _centre(squad: SkirmishSquad) -> void:
	var low := INF
	var high := -INF
	# store: scattered (production: once, as a wave is sent)
	for unit in squad.living():
		var column := UnitPlace.of(squad, unit).y
		low = minf(low, column)
		high = maxf(high, column + unit.footprint_width)
	if low < INF:
		squad.centre_shift = squad.width / 2.0 - (low + high) / 2.0
