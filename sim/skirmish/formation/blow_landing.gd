class_name BlowLanding
extends RefCounted
## Lands a tick's blows (Decision 118, spec 28 part 4): each blow's roll, seeded by the
## battle, the tick and the striker (Decision 93; never the list, Decision 97), read by
## BlowRoll against its target, with the striker's margin shifted by its formation's
## morale band, by high ground, by each foe beyond the first that the target is fighting
## off, and by how tired it is (FormationStamina). With rolls off (the plain arithmetic
## the rule tests lean on) every blow is a hit at its weapons' full damage. What each
## deals is BlowDamage's. Pure over the squads it is given.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")
const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const BlowRoll = preload("res://sim/skirmish/formation/blow_roll.gd")
const BlowDamage = preload("res://sim/skirmish/formation/blow_damage.gd")
const FormationRegeneration = preload("res://sim/skirmish/formation/formation_regeneration.gd")
const FormationStamina = preload("res://sim/skirmish/formation/formation_stamina.gd")
const BattleRolls = preload("res://sim/skirmish/formation/battle_rolls.gd")
const FormationMorale = preload("res://sim/skirmish/formation/formation_morale.gd")
const FormationTerrain = preload("res://sim/skirmish/formation/formation_terrain.gd")
const ScrumBlowsField = preload("res://sim/skirmish/formation/scrum_blows_field.gd")
const UnitColumns = preload("res://sim/skirmish/formation/unit_columns.gd")
## The columns a landed blow writes, held for the landing (spec 35 step 7b): the seconds a
## blow of a type it fears stops the target's regeneration, and the wounds the last blow that
## struck it would add.
const MARKED := [UnitColumns.REGENERATION_HALT, UnitColumns.LAST_WOUNDING]


## Lands the melee blows ([striker, target, damage, flank]) and ranged shots ([shooter,
## target, damage, its squad]): sets each one's damage to what landed and appends how it
## landed (BlowRoll's outcomes). `rolls` = [battle seed, tick], or [] for no rolls. The foes
## pressing each target are counted on the battle's native bodies (`field`; without one,
## its squads' store's), as FormationMelee.blows synced them this tick, or synced now if
## it had no need to (ScrumBlowsField).
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
	if rolls.is_empty():
		var marks := UnitView.hold([squads[0]], MARKED)  # a write per blow: held
		var halts: PackedFloat64Array = marks.regeneration_halt
		var woundings: PackedInt32Array = marks.last_wounding
		for blow in melee:
			blow[2] = BlowDamage.dealt(blow[0], blow[1], blow[2], BlowRoll.HIT, false, 1.0)
			FormationRegeneration.scorch(halts, blow[1], blow[0].melee_parts, blow[2])
			if blow[2] > 0:
				woundings[blow[1].slot] = blow[0].wounding
			blow.append(BlowRoll.HIT)
		for shot in shots:
			shot[2] = BlowDamage.dealt(shot[0], shot[1], shot[2], BlowRoll.HIT, true, 1.0)
			FormationRegeneration.scorch(halts, shot[1], shot[0].ranged_parts, shot[2])
			shot.append(BlowRoll.HIT)
		UnitView.release(marks)
		return
	if field == null:
		field = squads[0].store.field()
	if not ScrumBlowsField.synced(field):
		ScrumBlowsField.sync(field, squads, rolls[0])  # no melee search ran on it this tick
	var whose := ScrumBlowsField.pressed(field, squads, melee + shots)
	var squad_of: Dictionary = whose[0]  # unit id -> [unit, its squad]
	var pressed: Dictionary = whose[1]  # unit id -> foes pressing it
	var at: PackedVector2Array = squads[0].store.columns.column(UnitColumns.POSITION)
	var bearings: PackedFloat64Array = squads[0].store.columns.column(UnitColumns.BEARING)
	var stamina: PackedFloat64Array = squads[0].store.columns.column(UnitColumns.STAMINA)
	var marks := UnitView.hold([squads[0]], MARKED)  # a write per blow, after the native count
	var known := [squad_of, pressed, at, bearings, stamina, marks.regeneration_halt]
	known.append(marks.last_wounding)
	for blow in melee:
		_land(blow, false, blow[3], known, terrain, rolls)
	for shot in shots:
		var slot: int = shot[1].slot
		var flank := BlowRoll.from_flank(bearings[slot], at[slot], at[shot[0].slot])
		_land(shot, true, flank, known, terrain, rolls)
	UnitView.release(marks)


static func _land(
	blow: Array, ranged: bool, flank: bool, known: Array, terrain: FormationTerrain, rolls: Array
) -> void:
	var squad_of: Dictionary = known[0]  # unit id -> [unit, its squad]
	var pressed: Dictionary = known[1]  # unit id -> foes pressing it
	var at: PackedVector2Array = known[2]  # where each stands, by slot
	var bearings: PackedFloat64Array = known[3]  # the way each faces, by slot
	var stamina: PackedFloat64Array = known[4]  # each one's stamina, by slot
	var halts: PackedFloat64Array = known[5]  # REGENERATION_HALT, held
	var woundings: PackedInt32Array = known[6]  # LAST_WOUNDING, held
	var striker: SkirmishUnit = blow[0]
	var target: SkirmishUnit = blow[1]
	var tuning := BattleTuning.current()
	var shift := 0.0
	if squad_of.has(striker.id):
		shift += tuning.blow_morale_shift[FormationMorale.band(squad_of[striker.id][1])]
	var from := at[striker.slot]
	if terrain != null and terrain.high_ground(from, at[target.slot]):
		shift += tuning.blow_high_ground
	shift += maxi(0, pressed.get(target.id, 0) - 1) * tuning.blow_surrounded
	shift += BlowDamage.spill(striker, ranged)
	shift += FormationStamina.skill_shift(striker, stamina[striker.slot])
	var roll := BattleRolls.uniform(rolls[0], [rolls[1], striker.id, "blow", ranged])
	var side := BlowRoll.side(bearings[target.slot], at[target.slot], from, flank)
	var landed := BlowRoll.outcome(
		BlowRoll.margin(striker, ranged, shift, roll), target, side, ranged
	)
	var spread := BattleRolls.uniform(rolls[0], [rolls[1], striker.id, "damage", ranged])
	blow[2] = BlowDamage.dealt(striker, target, blow[2], landed, ranged, spread)
	FormationRegeneration.scorch(
		halts, target, striker.ranged_parts if ranged else striker.melee_parts, blow[2]
	)
	if blow[2] > 0:
		woundings[target.slot] = striker.wounding
	blow.append(landed)
