class_name UnitDef
extends Resource
## What presentation reads of a unit type (spec 39 step 9e-ii): its cost, its hp and the damage
## of its melee strike, for the wave form panel. The whole sheet (Decision 117, spec 28) is the
## crate's (`native/rust/src/content/unit.rs`, content/units/<id>.json), which fills this
## field for field (CrateContent.unit) and keeps the rest; a screen that needs more of it reads
## it through the crate's binding as plain data.

@export var cost_food: int = 0
@export var hp: int = 0
## What it carries (Decisions 47 and 120): weapons, armour and tools, and its innate weapons
## (fists, a bite) - its every blow comes from a weapon.
@export var items: Array[ItemDef] = []
## The slots its body has for items (hand, hand, back...): an item fits if its slots are free.
@export var slots: Array[String] = []


## The damage of one melee strike: every melee weapon together (0 without one).
func melee_damage() -> int:
	var total := 0
	for weapon in _weapons():
		if weapon.is_melee():
			total += weapon.damage
	return total


## The weapons it can use: those it carries, and its innate ones whose body parts no
## carried item holds (fists are no use with a spear in both hands).
func _weapons() -> Array[WeaponDef]:
	var free := _free_slots()
	var out: Array[WeaponDef] = []
	for item in items:
		if item is WeaponDef and (not item.innate or _has_slots(free, item.slots)):
			out.append(item)
	return out


## The slots its body has left once its carried items take theirs.
func _free_slots() -> Array[String]:
	var free := slots.duplicate()
	for item in items:
		if item != null and not item.innate:
			for slot in item.slots:
				free.erase(slot)
	return free


## Whether `have` holds every slot `need` names, as many times as it names it.
static func _has_slots(have: Array[String], need: Array[String]) -> bool:
	var left := have.duplicate()
	for slot in need:
		if not left.has(slot):
			return false
		left.erase(slot)
	return true
