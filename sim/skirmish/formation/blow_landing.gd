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
## each target and each striker's morale band, landed in the fight's pass (`fight::step`,
## 8b5-xix). This hands the pass the landing's tuning and names how each blow landed.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")

## How a blow landed, as the events name it, by the crate's code (`landing.rs`).
const PARRIED := "parried"
const DODGED := "dodged"
const GRAZED := "grazed"
const HIT := "hit"
const CRITICAL := "critical"
const LANDED := [PARRIED, DODGED, GRAZED, HIT, CRITICAL]


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
