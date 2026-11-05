class_name ItemDef
extends Resource
## What presentation reads of anything a unit carries (Decision 120; spec 39 step 9e-ii): the
## slots it takes, and whether it is part of the creature. A weapon is a WeaponDef; armour and
## tools are plain items here. The whole item is the crate's (`native/rust/src/content/items.rs`).

## The slots it takes (hand, hand for a two-handed spear). An innate item takes none:
## these are the body parts it is part of (fists the hands, a bite or spit the mouth), and
## it can't be used while a carried item holds one of them.
@export var slots: Array[String] = []
## Part of the creature (fists, a bite, a claw, its spit).
@export var innate: bool = false
