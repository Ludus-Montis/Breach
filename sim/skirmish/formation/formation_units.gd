class_name FormationUnits
extends RefCounted
## Makes a formation SkirmishUnit from its UnitDef (specs/22-formation-feel-test.md): its
## footprint, preferred band and - from its weapons (Decision 47) - its melee strike (all
## melee weapons together) and its best ranged weapon. Pure.

const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const TerrainWalker = preload("res://sim/skirmish/formation/terrain_walker.gd")
const UnitDef = preload("res://content/definitions/unit_def.gd")
const UnitArms = preload("res://content/definitions/unit_arms.gd")
const BattleTuning = preload("res://content/definitions/battle_tuning.gd")


## Its place in its squad's frame is the unit store's, set once the squad enrols it (spec 35
## step 4a).
static func make(
	unit_def: UnitDef, faction_id: String, direction: int, unit_id: int
) -> SkirmishUnit:
	var unit := SkirmishUnit.new()
	unit.id = unit_id
	unit.faction_id = faction_id
	unit.walker = TerrainWalker.of(unit_def)
	unit.hp = unit_def.hp
	unit.max_hp = unit_def.hp
	unit.speed = unit_def.speed
	unit.footprint_depth = unit_def.footprint_depth
	unit.footprint_width = unit_def.footprint_width
	unit.preferred_position = unit_def.preferred_position
	unit.position_priority = unit_def.position_priority
	unit.detection = unit_def.detection_range
	unit.courage = unit_def.courage
	unit.leadership = unit_def.leadership
	unit.tactics = unit_def.tactics.duplicate()
	unit.height = unit_def.height
	unit.initiative = unit_def.initiative
	unit.discipline = unit_def.discipline
	unit.turn_rate = unit_def.turn_rate
	unit.backward_pace = unit_def.backward_pace
	unit.definition = unit_def
	unit.tags = unit_def.tags.duplicate()
	for attribute in UnitDef.ATTRIBUTES:
		unit.attributes[attribute] = unit_def.get(attribute)
	unit.traits = unit_def.traits.duplicate()
	for item in unit_def.items:  # what an item grants is its carrier's (Decision 128)
		for trait_id in item.grants:
			if unit_def.trait_level(trait_id) > 0:
				unit.traits[trait_id] = unit_def.trait_level(trait_id)
	unit.melee_seconds = unit_def.melee_seconds()
	unit.defence = unit_def.defence
	unit.critical = unit_def.critical
	unit.parries = unit_def.can_parry()
	_arm(unit, unit_def)
	var ranged := unit_def.ranged_weapon()
	if ranged != null:
		unit.attack_range = ranged.attack_range
		unit.damage_type = ranged.damage_type
		unit.ranged_seconds = ranged.attack_seconds / unit_def.attack_speed
	unit.advance_direction = direction
	return unit


## Its weapons as the fight reckons them (UnitArms, Decision 119): their parts and damage
## with its strength, the skill it wields them at, and what it wears.
static func _arm(unit: SkirmishUnit, unit_def: UnitDef) -> void:
	unit.melee_parts = UnitArms.parts(unit_def, false)
	unit.ranged_parts = UnitArms.parts(unit_def, true)
	unit.melee_floor = UnitArms.floor_share(unit_def, false)
	unit.ranged_floor = UnitArms.floor_share(unit_def, true)
	unit.melee_skill = UnitArms.skill(unit_def, false)
	unit.ranged_skill = UnitArms.skill(unit_def, true)
	unit.dmg = roundi(unit.melee_parts.reduce(func(sum, part): return sum + part[0], 0.0))
	if not unit.ranged_parts.is_empty():
		unit.ranged_dmg = roundi(unit.ranged_parts[0][0])
	unit.weaknesses = unit_def.weaknesses.duplicate()
	unit.resistances = unit_def.resistances.duplicate()
	unit.immunities = unit_def.immunities.duplicate()
	var tuning := BattleTuning.current()
	var stage := UnitArms.load_stage(unit_def)
	unit.speed = unit_def.speed * tuning.load_pace[stage]
	unit.fresh_speed = unit.speed
	unit.run_pace = unit_def.run_pace
	unit.load_stage = stage
	unit.gear_weight = UnitArms.carried_weight(unit_def)
	for weapon in unit_def.weapons():
		unit.wounding = maxi(unit.wounding, int(weapon.traits.get("wounding", 0)))
	unit.tiring = tuning.load_tiring[stage]
	unit.dodging = tuning.load_dodge[stage]
	unit.max_stamina = unit_def.constitution * tuning.stamina_per_constitution
	unit.stamina = unit.max_stamina
	unit.regeneration = unit_def.regeneration
	unit.regeneration_left = unit_def.regeneration_limit * unit_def.hp
	unit.regeneration_stops = unit_def.regeneration_stops.duplicate()
	var protection := UnitArms.protection(unit_def)
	unit.armour = protection.x
	unit.ward = protection.y
