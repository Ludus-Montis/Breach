//! The items a unit carries (Decision 120, spec 28): `ItemDef`'s fields, which every item
//! has, and the two kinds of item, weapons (`WeaponDef`) and armour (`ArmourDef`). A file
//! under `content/weapons/` is a weapon, and one under `content/armour/` is armour. The
//! fields and their defaults are GDScript's, name for name: until their last reader ports,
//! GDScript's definitions are filled from these (Decision 153).

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// What every item has (`ItemDef`): its slots, weight, strength needed, tags and traits.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct Item {
    pub item_name: String,
    /// The slots it takes (hand, hand for a two-handed spear). An innate item takes none:
    /// these are the body parts it is part of (fists the hands, a bite or spit the mouth),
    /// and it can't be used while a carried item holds one of them.
    pub slots: Vec<String>,
    /// Its weight, in load.
    pub weight: f64,
    /// The strength needed to wield it well.
    pub strength_requirement: i64,
    /// What kind of item it is, for proficiency (axe, polearm, heavy armour...).
    pub tags: Vec<String>,
    /// Its own rated traits, marking what it does, e.g. {"siege": 1} for a weapon that
    /// damages structures; each must be one an item of its kind may have (ContentRegistry).
    pub traits: BTreeMap<String, i64>,
    /// Rated traits it grants whoever carries it, e.g. {"burrower": 1} for a shovel
    /// (Decision 128).
    pub grants: BTreeMap<String, i64>,
    /// Part of the creature (fists, a bite, a claw, its spit): it can't be dropped or
    /// taken, weighs nothing, and its slots are the body parts it belongs to.
    pub innate: bool,
}

/// What a unit fights with (Decision 47, `WeaponDef`): an item with an attack profile.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct Weapon {
    pub item: Item,
    pub damage: i64,
    /// acid, piercing, slashing, bludgeoning... no effect yet; resistances come later.
    pub damage_type: String,
    /// Reach in ranks (a rank is one cell - Decisions 48 and 68); 0 is melee.
    pub attack_range: i64,
    /// The least share of its damage a blow deals before its wielder's skill lifts it
    /// toward the full (Decision 119): a club varies more than a spear.
    pub damage_floor: f64,
    /// Damage per point of its wielder's strength over its strength requirement
    /// (Decision 119): a maul much, a dagger little.
    pub strength_scaling: f64,
    /// Whether its blows are magic (Decision 119): ward, not armour, stands against them.
    pub magical: bool,
    /// Seconds between its blows (Decision 120): an unwieldy weapon strikes more slowly.
    /// The wielder's attack speed scales it.
    pub attack_seconds: f64,
}

impl Default for Weapon {
    fn default() -> Self {
        Weapon {
            item: Item::default(),
            damage: 0,
            damage_type: String::new(),
            attack_range: 0,
            damage_floor: 0.5,
            strength_scaling: 0.0,
            magical: false,
            attack_seconds: 1.0,
        }
    }
}

/// Armour (Decisions 119 and 120, `ArmourDef`): `armour` off each mundane blow its wearer
/// suffers, `ward` off each magical one.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct Armour {
    pub item: Item,
    pub armour: i64,
    pub ward: i64,
}
