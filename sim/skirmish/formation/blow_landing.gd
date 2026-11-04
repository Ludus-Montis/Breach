class_name BlowLanding
extends RefCounted
## Lands a tick's blows (Decision 118, spec 28 part 4): each blow's roll, seeded by the
## battle, the tick and the striker (Decision 93; never the list, Decision 97), read against
## its target's parry, dodge and defence, with the striker's margin shifted by its
## formation's morale band, by high ground, by each foe beyond the first that the target is
## fighting off, and by how tired it is; what each deals is rolled within its weapons' range
## and taken against the target's weaknesses, armour and ward (Decision 119). With rolls off
## (the plain arithmetic the rule tests lean on) every blow is a hit at its weapons' full
## damage. The rules are the crate's (`landing.rs`, spec 39 step 8a1), and so is what they
## read (`fight/land.rs`, 8b5-xviii): each unit's arms from its sheet and words, who presses
## each target and each striker's morale band; this hands it the blows by slot and lands
## what it returns. Pure over the squads it is given.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const FormationTerrain = preload("res://sim/skirmish/formation/formation_terrain.gd")

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
## FormationMelee.blows synced them this tick, or synced now if it had no need to.
## `roster`: the squads' CrateRoster (made here if not given).
static func land(
	melee: Array,
	shots: Array,
	squads: Array,
	terrain: FormationTerrain,
	rolls: Array,
	field: Object = null,
	roster: Object = null
) -> void:
	if melee.is_empty() and shots.is_empty():
		return
	if field == null:
		field = squads[0].store.field()
	if roster == null:
		roster = CrateRoster.of(squads)
	var all := melee + shots
	var blows := PackedInt64Array()
	for index in all.size():
		var blow: Array = all[index]
		var ranged := index >= melee.size()
		blows.append_array(
			[blow[0].slot, blow[1].slot, blow[2], int(ranged), int(not ranged and blow[3])]
		)
	var ground: Object = terrain.core if terrain != null else null
	var reach := BattleTuning.current().reach_contact
	var columns: UnitColumns = squads[0].store.columns
	var landed: PackedInt64Array = ColumnsLent.all(
		columns,
		func():
			return field.fight_land(
				roster, blows, PackedInt64Array(rolls), tuning(), reach, BandTuning.rules(), ground
			)
	)
	for index in all.size():
		all[index][2] = landed[2 * index]
		all[index].append(LANDED[landed[2 * index + 1]])


## What landing reads of BattleTuning, in the order LandingTuning.of_slice reads it.
static func tuning() -> PackedFloat64Array:
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
