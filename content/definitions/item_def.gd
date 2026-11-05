class_name ItemDef
extends Resource
## Anything a unit carries (Decision 120, spec 28): a weapon (WeaponDef), armour or a tool.
## An item names the slots it takes - a unit carries it only if its body has them free (a
## creature with no hands has no hand slot) - and has a weight (load), a strength needed
## to wield it well, tags a unit may be proficient with (axe, polearm, heavy armour), its
## own rated traits, and those it grants whoever carries it (a shovel, burrower 1: Decision
## 54; Decision 128). A natural weapon (fists, a bite, a claw) is innate.

@export var item_name: String = ""
## The slots it takes (hand, hand for a two-handed spear). An innate item takes none:
## these are the body parts it is part of (fists the hands, a bite or spit the mouth), and
## it can't be used while a carried item holds one of them.
@export var slots: Array[String] = []
## Its weight, in load.
@export var weight: float = 0.0
## The strength needed to wield it well.
@export var strength_requirement: int = 0
## What kind of item it is, for proficiency (axe, polearm, heavy armour...).
@export var tags: Array[String] = []
## Its own rated traits, marking what it does, e.g. {"siege": 1} for a weapon that damages
## structures; each must be one an item of its kind may have (ContentRegistry).
@export var traits: Dictionary = {}
## Rated traits it grants whoever carries it, e.g. {"burrower": 1} for a shovel - a
## weapon could grant "cunning" without being cunning itself (Decision 128).
@export var grants: Dictionary = {}
## Part of the creature (fists, a bite, a claw, its spit): it can't be dropped or taken,
## weighs nothing, and its slots are the body parts it belongs to.
@export var innate: bool = false
