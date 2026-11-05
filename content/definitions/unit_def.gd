class_name UnitDef
extends Resource
## A unit type's sheet (Decision 117, spec 28; specs/07-data-resource-schemas.md): its
## archetype tags, attributes and rated traits; body; movement; mind; senses; band; what
## it fights with; and its cost. Derived stats (load, death's door...) are worked out from
## the attributes where they are used, never set here.

## Where in a formation the unit wants to stand (Decision 46).
enum Position { FRONT, MID, BACK }

const ContentRegistry = preload("res://content/definitions/content_registry.gd")
const MAX_FOOTPRINT := 8
## The five attributes (Decision 117), each defaulting to AVERAGE.
const ATTRIBUTES := ["strength", "agility", "constitution", "willpower", "wits"]
const AVERAGE := 10

## Archetype tags tech can target as a group, beside the type itself (human, martial,
## cavalry, archer...).
@export var tags: Array[String] = []
## Attributes (Decision 117): strength (load, wield and damage scaling, shoving), agility
## (dodge, attack speed, turning, initiative), constitution (HP, stamina, physical maladies,
## death's door), willpower (magical maladies, fear, courage, ward) and wits (casting,
## mana, initiative, perception). AVERAGE is ordinary.
@export var strength: int = AVERAGE
@export var agility: int = AVERAGE
@export var constitution: int = AVERAGE
@export var willpower: int = AVERAGE
@export var wits: int = AVERAGE
## Rated traits (Decision 64), id -> level: climber 2, darksight 1, mob 16...
@export var traits: Dictionary = {}
@export var cost_food: int = 0
@export var hp: int = 0
@export var speed: float = 0.0
## How much faster than its march it runs (Decision 125): a formation hurrying, pursuing or
## fleeing runs at its slowest unit's run, spending stamina.
@export var run_pace: float = 1.5
## Formation slots the unit occupies, depth (ranks) x width (columns), per Decision 40:
## 1x1 a grem, 2x1 cavalry, 2x2 a brute, up to 8x8 (a dragon, the widest lane).
@export var footprint_depth: int = 1
@export var footprint_width: int = 1
## Seconds one of the domain's builders takes to build this unit (Decision 45).
@export var build_seconds: float = 1.0
## The formation band the unit prefers, and its claim within that band (higher claims a
## forward place first). Reinforcements re-form by these (Decision 46).
@export var preferred_position: Position = Position.FRONT
@export var position_priority: int = 0
## How far it detects others, in cells (Decision 87; placeholder). Line of sight and light
## come with terrain.
@export var detection_range: float = 40.0
## The shape of its sight when it plans a way (spec 30, PathSight): how far it sees ahead,
## square to either side and behind, as shares of its detection range, eased between by
## the angle off its facing. Placeholders.
@export var sight_ahead: float = 1.0
@export var sight_side: float = 0.5
@export var sight_behind: float = 0.25
## Its will to fight (Decision 82): a formation's morale ceiling is its units' mean courage.
@export var courage: int = 60
## How much it steadies a formation it leads (Decision 81); 0 for rank and file.
@export var leadership: int = 0
## How it leads (Decision 81), e.g. "coordinated": times a waiting wave's departure from
## what it sees (Decision 87).
@export var tactics: Array[String] = []
## How tall it stands, in cells (Decision 85): water a quarter of this deep slows it.
@export var height: float = 1.0
## How quickly it acts (Decision 88; placeholder, equal for all until spec 28): it wins a
## contested cell in the scrum over a slower-witted unit arriving at the same time.
@export var initiative: int = 10
## How drilled it is (Decision 92; placeholder: rank and file 30, drilled 60): a
## formation's mean, bolstered by its leader, decides whether it re-forms as a whole to meet
## a flank and how fast it re-forms.
@export var discipline: int = 30
## How it turns and backs away (Decision 95; placeholders until spec 28): degrees a second
## it turns, and the share of its speed it makes moving straight back (sideways is between).
@export var turn_rate: float = 450.0
@export var backward_pace: float = 0.4
## How well it fights hand to hand and at range (Decision 118), 0 to 100: a blow's roll
## is centred on it. Its parry comes from its melee skill, its dodge from its agility.
@export var melee_skill: int = 40
@export var ranged_skill: int = 40
## What a blow must get past, after its parry and dodge, to land fully: short of it, it
## grazes (Decision 118).
@export var defence: int = 10
## A critical blow's damage, times a hit's (Decision 118).
@export var critical: float = 1.5
## Damage types it takes more of, less of, or none of (Decision 119): weak x1.5,
## resistant x0.5, immune x0 (BattleTuning).
@export var weaknesses: Array[String] = []
@export var resistances: Array[String] = []
@export var immunities: Array[String] = []
## Its own armour (a hide) and ward (a charm), taken off each mundane or magical blow
## before what it wears adds (Decision 119, ArmourDef).
@export var armour: int = 0
@export var ward: int = 0
## HP it regenerates a second (Decision 121), whether struck or not, up to its limit per
## rest - that many times its max HP - and stopped for a while by a blow of a type listed
## in regeneration_stops (a troll's by fire or acid). Downed, it stops unless it has the
## "regenerates_downed" trait.
@export var regeneration: float = 0.0
@export var regeneration_limit: float = 1.0
@export var regeneration_stops: Array[String] = []
## What it carries (Decisions 47 and 120): weapons, armour and tools (ItemDef), and its
## innate weapons (fists, a bite) - its every blow comes from a weapon.
@export var items: Array[ItemDef] = []
## The slots its body has for items (hand, hand, back...): an item fits if its slots are
## free. A creature with no hands has no hand slot.
@export var slots: Array[String] = []
## How well it handles items by tag (axe, polearm...), tag -> level: 0 untrained (none
## listed), 1 trained, 2 mastered.
@export var proficiencies: Dictionary = {}
## How quickly it strikes (Decision 120): its weapons' intervals are divided by this, so
## 1.25 makes a 1-second weapon strike every 0.8 s.
@export var attack_speed: float = 1.0


## The level of a rated trait it has, its own or one an item it carries grants, the
## higher; one it doesn't list it has at the registry's default (0: not at all).
func trait_level(trait_id: String) -> int:
	var level := int(traits.get(trait_id, ContentRegistry.implicit().get(trait_id, 0)))
	for item in items:  # what its items grant it; their own traits are theirs (Decision 128)
		if item != null:
			level = maxi(level, int(item.grants.get(trait_id, 0)))
	return level


## The damage of one melee strike: every melee weapon together (0 without one).
func melee_damage() -> int:
	var total := 0
	for weapon in _weapons():
		if weapon.is_melee():
			total += weapon.damage
	return total


## Whether it can parry (Decision 118): it holds a melee weapon, not an innate one, or a
## shield.
func can_parry() -> bool:
	for weapon in _weapons():
		if weapon.is_melee() and not weapon.innate:
			return true
	return items.any(func(item): return item != null and item.tags.has("shield"))


## Seconds between its melee blows: its melee weapons strike together, as often as the
## slowest of them allows, scaled by its attack speed (a second without weapons).
func melee_seconds() -> float:
	var longest := 0.0
	for weapon in _weapons():
		if weapon.is_melee():
			longest = maxf(longest, weapon.attack_seconds)
	return (longest if longest > 0.0 else 1.0) / attack_speed


## The hardest-hitting ranged weapon, or null for a melee-only unit.
func ranged_weapon() -> WeaponDef:
	var best: WeaponDef = null
	for weapon in _weapons():
		if not weapon.is_melee() and (best == null or weapon.damage > best.damage):
			best = weapon
	return best


## The weapons it can use: those it carries, and its innate ones free to strike.
func weapons() -> Array[WeaponDef]:
	return _weapons()


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
