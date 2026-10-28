class_name BlowLanding
extends RefCounted
## Lands a tick's blows (Decision 118, spec 28 part 4): each blow's roll, seeded by the
## battle, the tick and the striker (Decision 93; never the list, Decision 97), read against
## its target's parry, dodge and defence, with the striker's margin shifted by its
## formation's morale band, by high ground, by each foe beyond the first that the target is
## fighting off, and by how tired it is; what each deals is rolled within its weapons' range
## and taken against the target's weaknesses, armour and ward (Decision 119). With rolls off
## (the plain arithmetic the rule tests lean on) every blow is a hit at its weapons' full
## damage. The rules are the crate's (`landing.rs`, spec 39 step 8a1): this hands it each
## unit's arms, who presses each target and each striker's morale band, and lands what it
## returns. Pure over the squads it is given.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const FormationMorale = preload("res://sim/skirmish/formation/formation_morale.gd")
const FormationTerrain = preload("res://sim/skirmish/formation/formation_terrain.gd")
const ScrumBlowsField = preload("res://sim/skirmish/formation/scrum_blows_field.gd")

## How a blow landed, as the events name it, by the crate's code (`landing.rs`).
const PARRIED := "parried"
const DODGED := "dodged"
const GRAZED := "grazed"
const HIT := "hit"
const CRITICAL := "critical"
const LANDED := [PARRIED, DODGED, GRAZED, HIT, CRITICAL]


## Lands the melee blows ([striker, target, damage, flank]) and ranged shots ([shooter,
## target, damage, its squad]): sets each one's damage to what landed and appends how it
## landed. `rolls` = [battle seed, tick], or [] for no rolls. The foes pressing each target
## are counted on the battle's native bodies (`field`; without one, its squads' store's), as
## FormationMelee.blows synced them this tick, or synced now if it had no need to
## (ScrumBlowsField).
static func land(
	melee: Array,
	shots: Array,
	squads: Array,
	terrain: FormationTerrain,
	rolls: Array,
	field: Object = null
) -> void:
	if melee.is_empty() and shots.is_empty():
		return
	var all := melee + shots
	var whose := [{}, {}]  # [unit id -> [unit, its squad], unit id -> foes pressing it]
	if not rolls.is_empty():
		if field == null:
			field = squads[0].store.field()
		if not ScrumBlowsField.synced(field):
			ScrumBlowsField.sync(field, squads, rolls[0])  # no melee search ran on it this tick
		whose = ScrumBlowsField.pressed(field, squads, all)
	var arms := _Arms.new()
	var blows := PackedInt64Array()
	for index in all.size():
		var blow: Array = all[index]
		var ranged := index >= melee.size()
		var band := -1
		if whose[0].has(blow[0].id):
			band = FormationMorale.band(whose[0][blow[0].id][1])
		blows.append_array(
			[
				arms.index(blow[0]),
				arms.index(blow[1]),
				blow[2],
				int(ranged),
				int(not ranged and blow[3]),
				band,
				whose[1].get(blow[1].id, 0)
			]
		)
	var columns: UnitColumns = squads[0].store.columns
	var native: Object = ColumnsLent.field(columns)
	var ground: Object = terrain.core if terrain != null else null
	var landed: PackedInt64Array = ColumnsLent.all(
		columns,
		func():
			return native.land_blows(
				arms.ints, arms.reals, arms.parts, blows, _tuning(), PackedInt64Array(rolls), ground
			)
	)
	for index in all.size():
		all[index][2] = landed[2 * index]
		all[index].append(LANDED[landed[2 * index + 1]])


## What landing reads of BattleTuning, in the order LandingTuning.of_slice reads it.
static func _tuning() -> PackedFloat64Array:
	var tuning := BattleTuning.current()
	var out := PackedFloat64Array(
		[
			tuning.blow_die,
			tuning.blow_parry_share,
			tuning.blow_dodge_per_agility,
			tuning.blow_hit_band,
			tuning.blow_graze_share,
			tuning.blow_high_ground,
			tuning.blow_surrounded,
			tuning.blow_rear_arc_degrees,
			tuning.damage_weak,
			tuning.damage_resist,
			tuning.damage_immune,
			tuning.damage_floor_skill,
			tuning.damage_spill,
			tuning.wounds_regeneration_halt,
			tuning.reach_front_arc_degrees,
			tuning.stamina_tired,
			tuning.stamina_spent,
			tuning.blow_morale_shift.size()
		]
	)
	out.append_array(tuning.blow_morale_shift)
	out.append_array(tuning.stamina_skill)
	return out


## The arms of the units a tick's blows strike with or land on, each once, as the crate's
## `land_blows` takes them; damage types by a code each, in the order first met.
class _Arms:
	var ints := PackedInt64Array()
	var reals := PackedFloat64Array()
	var parts := PackedFloat64Array()
	var _at := {}  # unit -> its index
	var _types := {}  # damage type -> its code

	## The unit's index, its arms added the first time it is met.
	func index(unit: SkirmishUnit) -> int:
		if _at.has(unit):
			return _at[unit]
		_at[unit] = _at.size()
		ints.append_array(
			[
				unit.id,
				unit.slot,
				unit.defence,
				unit.armour,
				unit.ward,
				unit.wounding,
				int(unit.parries),
				_bits(unit.weaknesses),
				_bits(unit.resistances),
				_bits(unit.immunities),
				_bits(unit.regeneration_stops),
				unit.melee_parts.size(),
				unit.ranged_parts.size()
			]
		)
		var agility: float = unit.attributes.get("agility", UnitDef.AVERAGE)
		reals.append_array(
			[
				unit.melee_skill,
				unit.ranged_skill,
				unit.critical,
				unit.melee_floor,
				unit.ranged_floor,
				unit.max_stamina,
				agility,
				unit.dodging
			]
		)
		for part in unit.melee_parts + unit.ranged_parts:
			parts.append_array([part[0], _code(part[1]), float(part[2])])
		return _at[unit]

	func _bits(types: Array) -> int:
		var bits := 0
		for type in types:
			bits |= 1 << _code(type)
		return bits

	func _code(type: String) -> int:
		var code: int = _types.get_or_add(type, _types.size())
		assert(code < 64, "a tick's blows name at most 64 damage types")
		return code
