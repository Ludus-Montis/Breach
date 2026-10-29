class_name SkirmishUnit
extends RefCounted
## One unit in the formation battle (specs/22-formation-feel-test.md). Plain data; the
## formation simulation's passes own every rule that changes it.

## The player's standing instruction. Applied by the simulation on the next tick.
enum Order { ADVANCE, HOLD, RETREAT }
## What the unit is doing right now (derived by the simulation each tick). Out of the
## fight (Decision 121): DEAD; DOWNED, on the ground at 0 HP or below it at death's door;
## TAKEN, captured or surrendered; RELEASED, let go to carry word home; CARRIED, downed and
## borne by a friend (Decision 126).
enum State { MOVING, HOLDING, FIGHTING, ARRIVED, DEAD, DOWNED, TAKEN, RELEASED, CARRIED }

var id: int = 0
var faction_id: String = "":
	set(value):
		faction_id = _said(&"faction_id", value)
var max_hp: int = 0:
	set(value):
		max_hp = _held(&"max_hp", value)
var dmg: int = 0:
	set(value):
		dmg = _held(&"dmg", value)
## Formation sim: its slot in the battle's unit store (spec 35), given when it enrols and
## fixed while it is held. Where it stands and the rest of its per-tick state are there.
var slot := -1
## Formation sim: how far it detects others, in cells (Decision 87).
var detection := 40.0:
	set(value):
		detection = _held(&"detection", value)
## Formation sim: its courage and leadership (Decisions 81 and 82).
var courage := 60:
	set(value):
		courage = _held(&"courage", value)
var leadership := 0:
	set(value):
		leadership = _held(&"leadership", value)
var tactics: Array[String] = []:
	set(value):
		tactics = _said(&"tactics", value)
var height := 1.0:
	set(value):
		height = _held(&"height", value)
## Formation sim: its sheet as it stands now (Decision 117) - archetype tags, attributes
## (UnitDef.ATTRIBUTES, name -> value) and rated traits (name -> level) - copied from its
## definition, so tech, items and ranks can change one unit and not its type.
var tags: Array[String] = []:
	set(value):
		tags = _said(&"tags", value)
var attributes := {}:
	set(value):
		attributes = _said(&"attributes", value)
var traits := {}:
	set(value):
		traits = _said(&"traits", value)
## Formation sim: how quickly it acts, and how drilled it is (Decisions 88 and 92).
var initiative := 10:
	set(value):
		initiative = _held(&"initiative", value)
var discipline := 30:
	set(value):
		discipline = _held(&"discipline", value)
## Formation sim: how it turns and backs away (UnitMotion). The bearing it faces is the unit
## store's (UnitColumns.BEARING).
var turn_rate := 450.0:
	set(value):
		turn_rate = _held(&"turn_rate", value)
var backward_pace := 0.4:
	set(value):
		backward_pace = _held(&"backward_pace", value)
## Formation sim: the definition it was made from (a router fleeing home rejoins the
## reserve as one of these).
var definition: Resource = null
## +1 advances toward the kingdom's end, -1 toward the player's.
var advance_direction: int = 1:
	set(value):
		advance_direction = _held(&"advance_direction", value)
var state: State = State.MOVING

# Formation fields (specs/22-formation-feel-test.md).
## The squad (wave) this unit marches in; 0 = none.
var squad_id: int = 0:
	set(value):
		squad_id = _held(&"squad_id", value)
## The command it follows, and the one it started under (FormationCommand, spec 30 round
## 3); typed loosely to keep the unit free of formation code.
var command: RefCounted = null
var origin: RefCounted = null
## How it crosses terrain (TerrainWalker: swimming, climbing), from its definition. The
## last place it could stand, whether it is on a climb face and the climb still above it
## live in the unit store, as do the unit it fights and how far along its route it stands
## (UnitColumns.FOOTHOLD, ON_FACE, CLIMB_LEFT, TARGET_ID, DISTANCE; spec 35 step 7c). Its
## order, home and wait are its squad's.
var walker: RefCounted = null
## Its rank and column in its squad's formation live in the unit store (UnitColumns.RANK,
## COLUMN; spec 35 step 4a).
## Slots it occupies, depth (ranks) x width (columns) - Decision 40.
var footprint_depth: int = 1:
	set(value):
		footprint_depth = _held(&"footprint_depth", value)
var footprint_width: int = 1:
	set(value):
		footprint_width = _held(&"footprint_width", value)
## UnitDef.Position band and claim within it (Decision 46).
var preferred_position: int = 0:
	set(value):
		preferred_position = _held(&"preferred_position", value)
var position_priority: int = 0:
	set(value):
		position_priority = _held(&"position_priority", value)
## From its best ranged weapon (Decision 47): reach in ranks (0 = melee only), damage, type.
## `dmg` is the unit's melee strike - all its melee weapons together.
var attack_range: int = 0:
	set(value):
		attack_range = _held(&"attack_range", value)
var ranged_dmg: int = 0:
	set(value):
		ranged_dmg = _held(&"ranged_dmg", value)
var damage_type: String = "":
	set(value):
		damage_type = _said(&"damage_type", value)
## Formation sim: its skills, defence and critical multiplier, and whether it can parry
## (Decision 118, UnitDef).
var melee_skill := 40:
	set(value):
		melee_skill = _held(&"melee_skill", value)
var ranged_skill := 40:
	set(value):
		ranged_skill = _held(&"ranged_skill", value)
var defence := 10:
	set(value):
		defence = _held(&"defence", value)
var critical := 1.5:
	set(value):
		critical = _held(&"critical", value)
var parries := false:
	set(value):
		parries = _held(&"parries", value)
## Formation sim: what its blows are made of and what stands against them (Decision 119,
## UnitArms): its melee and ranged weapons' parts [[damage, type, magical], ...], their
## floor shares, its weaknesses, resistances and immunities, and its armour and ward.
var melee_parts := []:
	set(value):
		melee_parts = _said(&"melee_parts", value)
var ranged_parts := []:
	set(value):
		ranged_parts = _said(&"ranged_parts", value)
var melee_floor := 1.0:
	set(value):
		melee_floor = _held(&"melee_floor", value)
var ranged_floor := 1.0:
	set(value):
		ranged_floor = _held(&"ranged_floor", value)
var weaknesses: Array[String] = []:
	set(value):
		weaknesses = _said(&"weaknesses", value)
var resistances: Array[String] = []:
	set(value):
		resistances = _said(&"resistances", value)
var immunities: Array[String] = []:
	set(value):
		immunities = _said(&"immunities", value)
var armour := 0:
	set(value):
		armour = _held(&"armour", value)
var ward := 0:
	set(value):
		ward = _held(&"ward", value)
## Formation sim: its stamina at most (spec 28 part 7), its pace fresh, how fast it tires
## and how well it dodges under its load (UnitArms.load_stage). Its stamina now, its pace
## this tick, whether it exerted itself, seconds since it last did (Decision 125), its run
## (Decision 126) and its condition are the unit store's (spec 35 step 6c: UnitColumns).
var max_stamina := 100.0:
	set(value):
		max_stamina = _held(&"max_stamina", value)
var fresh_speed := 0.0:
	set(value):
		fresh_speed = _held(&"fresh_speed", value)
var tiring := 1.0:
	set(value):
		tiring = _held(&"tiring", value)
var dodging := 1.0:
	set(value):
		dodging = _held(&"dodging", value)
## Formation sim: how much faster it runs than it marches, and its stage of load.
var run_pace := 1.5:
	set(value):
		run_pace = _held(&"run_pace", value)
var load_stage := 0:
	set(value):
		load_stage = _held(&"load_stage", value)
## Formation sim: its regeneration (Decision 121): HP a second, and the damage types that
## stop it. Its hp, the HP it has left to regain before it must rest and the ticks till its
## next blow are the unit store's (UnitHealth, UnitColumns.REGENERATION_LEFT and
## ATTACK_COOLDOWN, spec 35 step 5b), as are the seconds it stays stopped and the part of a
## point regained so far (REGENERATION_HALT, REGENERATION_CARRY, step 7b).
var regeneration := 0.0:
	set(value):
		regeneration = _held(&"regeneration", value)
var regeneration_stops: Array[String] = []:
	set(value):
		regeneration_stops = _said(&"regeneration_stops", value)
## Formation sim: its wounds (Decision 126) - a time downed each, lowering its condition. The
## seconds it has yet to lie downed and the part of a point its poor condition has drained
## are the store's (WAKE_LEFT, DRAIN_CARRY, step 7b).
var wounded := 0:
	set(value):
		wounded = _held(&"wounded", value)
## Formation sim: the wounds its weapons add to a foe they down ("wounding N", the most of
## its weapons'). Those the last blow that struck it would add are the store's
## (LAST_WOUNDING, step 7b).
var wounding := 0:
	set(value):
		wounding = _held(&"wounding", value)
## Formation sim: lying still while foes are near, though it could get up (Decision 127).
var playing_dead := false:
	set(value):
		playing_dead = _held(&"playing_dead", value)
## Formation sim: the friend it bears, or the friend bearing it (unit ids; 0: none), and the
## weight of its own gear, for its load with a body on it (Decision 126).
var carrying := 0:
	set(value):
		carrying = _held(&"carrying", value)
var carried_by := 0:
	set(value):
		carried_by = _held(&"carried_by", value)
var gear_weight := 0.0:
	set(value):
		gear_weight = _held(&"gear_weight", value)
## Seconds between its melee blows and its ranged shots (Decision 120): its weapons'
## intervals, scaled by its attack speed.
var melee_seconds := 1.0:
	set(value):
		melee_seconds = _held(&"melee_seconds", value)
var ranged_seconds := 1.0:
	set(value):
		ranged_seconds = _held(&"ranged_seconds", value)

## The native field whose store holds its numbers, words and lists while it holds it (null
## before it enrols and once it is let go). Its own copy is read without a call: no crate
## pass writes them.
var _sheet_field: Object = null


## Its numbers, words and lists are the crate's from now on (UnitStore.admit, once it has
## its slot): all written at once, then each by its own setter (UnitWrites).
func hold_sheet(field: Object) -> void:
	_sheet_field = field
	if field != null:
		UnitWrites.all(self, field)


## The store has let it go: its slot may be another's, so it writes there no more.
func let_go() -> void:
	_sheet_field = null


## Sets one of its traits' levels in place, written to the crate as `traits = ...` is.
func set_trait(trait_id: String, level: Variant) -> void:
	traits[trait_id] = level
	_said(&"traits", traits)


## Still in the fight: standing, not dead, downed, taken or released.
func is_alive() -> bool:
	return state < State.DEAD


func is_hostile_to(other: SkirmishUnit) -> bool:
	return other.faction_id != faction_id


## A number just set (returned), written to the crate's sheet while a store holds it.
func _held(name: StringName, value: Variant) -> Variant:
	if _sheet_field != null:
		UnitWrites.number(_sheet_field, slot, name, value)
	return value


## A word or list just set (returned), written to the crate while a store holds it.
func _said(name: StringName, value: Variant) -> Variant:
	if _sheet_field != null:
		UnitWrites.word(_sheet_field, slot, name, value)
	return value
