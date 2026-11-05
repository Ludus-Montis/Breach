class_name WeaponDef
extends ItemDef
## What presentation reads of a weapon (Decision 47; spec 39 step 9e-ii): its damage and
## whether it is melee, for a unit's melee strike. The whole weapon is the crate's
## (`native/rust/src/content/items.rs`).

@export var damage: int = 0
## Reach in ranks (a rank is one cell - Decisions 48 and 68); 0 is melee.
@export var attack_range: int = 0


## A natural melee weapon striking for `damage_of` (Decision 120): what a unit fights with
## when it carries nothing else.
static func innate_weapon(damage_of: int) -> WeaponDef:
	var weapon := WeaponDef.new()
	weapon.damage = damage_of
	weapon.innate = true
	return weapon


func is_melee() -> bool:
	return attack_range == 0
