class_name CrateMarch
extends RefCounted
## The crate's march (`GroundMarch`, native/rust/src/march/; spec 39 step 3d), which
## FormationMarch, FormationSweep, FormationWalk, FormationPour, FormationShuffle and
## PlaceOrder are views over, and what of a squad and its units they pass it. Stateless, so
## one is shared.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const ScrumReach = preload("res://sim/skirmish/formation/scrum_reach.gd")
const TerrainWalker = preload("res://sim/skirmish/formation/terrain_walker.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")

const MARCH_CLASS := "GroundMarch"
## FormationWalk.rule_of's traits, by the crate's codes (0: neither).
const RULES := ["", "fall_behind_left_behind", "no_man_left_behind"]

static var _core: Object


## The shared one.
static func core() -> Object:
	if _core == null:
		NativeLibrary.load_for(MARCH_CLASS)
		_core = ClassDB.instantiate(MARCH_CLASS)
	return _core


## The squad's frame (StoredSquad.frame): [anchor x, y, heading, width, centre shift].
static func frame(squad) -> PackedFloat64Array:
	var at: Vector2 = squad.position
	return PackedFloat64Array([at.x, at.y, squad.heading, squad.width, squad.centre_shift])


## `units` of `squad` as a pour reads them, seventeen numbers each: id, where it stands,
## its place (rank, column), footprint width and depth, how far it is through a swap
## (FormationShuffle.offset, `moving` its movers or null), its walker's five, and 1 and
## where the downed foe it is out taking lies, or three 0s.
static func pourers(squad, units: Array, moving) -> PackedFloat64Array:
	var at: PackedVector2Array = squad.store.columns.column(UnitColumns.POSITION)
	var places := UnitView.read(squad, UnitView.PLACES)  # read once a pass (Decision 146)
	var ranks: PackedInt32Array = places.rank
	var files: PackedInt32Array = places.column
	var taking: Dictionary = squad.taking  # taker's id -> its body's store slot
	var out := PackedFloat64Array()
	for unit in units:
		var here := at[unit.slot]
		var swapping := FormationShuffle.offset(squad, unit, moving)
		var place := [ranks[unit.slot], files[unit.slot]]
		var shape := [unit.footprint_width, unit.footprint_depth, swapping.x, swapping.y]
		out.append_array(PackedFloat64Array([unit.id, here.x, here.y] + place + shape))
		out.append_array(TerrainWalker.of_unit(unit).fields())
		if taking.has(unit.id):
			var body: Vector2 = at[taking[unit.id]]
			out.append_array(PackedFloat64Array([1.0, body.x, body.y]))
		else:
			out.append_array(PackedFloat64Array([0.0, 0.0, 0.0]))
	return out


## What of `unit` its walk reads, eleven numbers: its walker's five, its most stamina, its
## footprint's area, its turn rate and backward pace, its body's radius, and 0.
static func marcher(unit) -> PackedFloat64Array:
	var area := float(unit.footprint_width * unit.footprint_depth)
	var out := TerrainWalker.of_unit(unit).fields()
	var rest := [unit.max_stamina, area, unit.turn_rate, unit.backward_pace]
	out.append_array(PackedFloat64Array(rest + [ScrumReach.radius(unit), 0.0]))
	return out


## The tuning a walk reads as it stands now.
static func walk_tuning() -> PackedFloat64Array:
	var tuning := BattleTuning.current()
	return PackedFloat64Array(
		[
			tuning.walk_face_travel,
			tuning.bodies_ground_drag,
			tuning.bodies_ground_block,
			tuning.stamina_climb,
			tuning.stamina_swim,
			tuning.stamina_spent,
			tuning.ground_cliff_quarters,
			tuning.fall_damage_per_cell,
			tuning.fall_mass_power,
		]
	)


## The bodies lying on the field (GroundBodies.of_lying's `bodies`) as the walk reads them:
## [where each lies, its radius and footprint area], worked out once and kept in `bodies`.
static func lying(bodies: Dictionary) -> Array:
	if not bodies.has("crate"):
		var sizes := PackedFloat64Array()
		for body in bodies["lying"]:
			var area := float(body.footprint_width * body.footprint_depth)
			sizes.append_array(PackedFloat64Array([ScrumReach.radius(body), area]))
		bodies["crate"] = [PackedVector2Array(bodies["ats"]), sizes]
	return bodies["crate"]
