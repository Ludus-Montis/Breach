//! A unit's sheet (`UnitDef`): its attributes, body, pace, sight, courage, skills,
//! protection and what it carries. The fields and their defaults are GDScript's, name for
//! name: until their last reader ports, GDScript's definitions are filled from these
//! (Decision 153). `items` names each item by its path under `content/`, without the
//! extension (`weapons/grem_bite`), and the loader checks each is there.

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Where a unit prefers to stand in its squad (`UnitDef.Position`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Position {
    #[default]
    Front,
    Mid,
    Back,
}

/// A unit's sheet (`UnitDef`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct Unit {
    /// Archetype tags tech can target as a group, beside the type itself (human, martial,
    /// cavalry, archer...).
    pub tags: Vec<String>,
    /// Attributes (Decision 117): strength (load, wield and damage scaling, shoving), agility
    /// (dodge, attack speed, turning, initiative), constitution (HP, stamina, physical maladies,
    /// death's door), willpower (magical maladies, fear, courage, ward) and wits (casting,
    /// mana, initiative, perception). AVERAGE is ordinary.
    pub strength: i64,
    pub agility: i64,
    pub constitution: i64,
    pub willpower: i64,
    pub wits: i64,
    /// Rated traits (Decision 64), id -> level: climber 2, darksight 1, mob 16...
    pub traits: BTreeMap<String, i64>,
    pub cost_food: i64,
    pub hp: i64,
    pub speed: f64,
    /// How much faster than its march it runs (Decision 125): a formation hurrying, pursuing or
    /// fleeing runs at its slowest unit's run, spending stamina.
    pub run_pace: f64,
    /// Formation slots the unit occupies, depth (ranks) x width (columns), per Decision 40:
    /// 1x1 a grem, 2x1 cavalry, 2x2 a brute, up to 8x8 (a dragon, the widest lane).
    pub footprint_depth: i64,
    pub footprint_width: i64,
    /// Seconds one of the domain's builders takes to build this unit (Decision 45).
    pub build_seconds: f64,
    /// The formation band the unit prefers, and its claim within that band (higher claims a
    /// forward place first). Reinforcements re-form by these (Decision 46).
    pub preferred_position: Position,
    pub position_priority: i64,
    /// How far it detects others, in cells (Decision 87; placeholder). Line of sight and light
    /// come with terrain.
    pub detection_range: f64,
    /// The shape of its sight when it plans a way (spec 30, PathSight): how far it sees ahead,
    /// square to either side and behind, as shares of its detection range, eased between by
    /// the angle off its facing. Placeholders.
    pub sight_ahead: f64,
    pub sight_side: f64,
    pub sight_behind: f64,
    /// Its will to fight (Decision 82): a formation's morale ceiling is its units' mean courage.
    pub courage: i64,
    /// How much it steadies a formation it leads (Decision 81); 0 for rank and file.
    pub leadership: i64,
    /// How it leads (Decision 81), e.g. "coordinated": times a waiting wave's departure from
    /// what it sees (Decision 87).
    pub tactics: Vec<String>,
    /// How tall it stands, in cells (Decision 85): water a quarter of this deep slows it.
    pub height: f64,
    /// How quickly it acts (Decision 88; placeholder, equal for all until spec 28): it wins a
    /// contested cell in the scrum over a slower-witted unit arriving at the same time.
    pub initiative: i64,
    /// How drilled it is (Decision 92; placeholder: rank and file 30, drilled 60): a
    /// formation's mean, bolstered by its leader, decides whether it re-forms as a whole to meet
    /// a flank and how fast it re-forms.
    pub discipline: i64,
    /// How it turns and backs away (Decision 95; placeholders until spec 28): degrees a second
    /// it turns, and the share of its speed it makes moving straight back (sideways is between).
    pub turn_rate: f64,
    pub backward_pace: f64,
    /// How well it fights hand to hand and at range (Decision 118), 0 to 100: a blow's roll
    /// is centred on it. Its parry comes from its melee skill, its dodge from its agility.
    pub melee_skill: i64,
    pub ranged_skill: i64,
    /// What a blow must get past, after its parry and dodge, to land fully: short of it, it
    /// grazes (Decision 118).
    pub defence: i64,
    /// A critical blow's damage, times a hit's (Decision 118).
    pub critical: f64,
    /// Damage types it takes more of, less of, or none of (Decision 119): weak x1.5,
    /// resistant x0.5, immune x0 (BattleTuning).
    pub weaknesses: Vec<String>,
    pub resistances: Vec<String>,
    pub immunities: Vec<String>,
    /// Its own armour (a hide) and ward (a charm), taken off each mundane or magical blow
    /// before what it wears adds (Decision 119, ArmourDef).
    pub armour: i64,
    pub ward: i64,
    /// HP it regenerates a second (Decision 121), whether struck or not, up to its limit per
    /// rest - that many times its max HP - and stopped for a while by a blow of a type listed
    /// in regeneration_stops (a troll's by fire or acid). Downed, it stops unless it has the
    /// "regenerates_downed" trait.
    pub regeneration: f64,
    pub regeneration_limit: f64,
    pub regeneration_stops: Vec<String>,
    /// What it carries (Decisions 47 and 120): weapons, armour and tools (ItemDef), and its
    /// innate weapons (fists, a bite) - its every blow comes from a weapon.
    pub items: Vec<String>,
    /// The slots its body has for items (hand, hand, back...): an item fits if its slots are
    /// free. A creature with no hands has no hand slot.
    pub slots: Vec<String>,
    /// How well it handles items by tag (axe, polearm...), tag -> level: 0 untrained (none
    /// listed), 1 trained, 2 mastered.
    pub proficiencies: BTreeMap<String, i64>,
    /// How quickly it strikes (Decision 120): its weapons' intervals are divided by this, so
    /// 1.25 makes a 1-second weapon strike every 0.8 s.
    pub attack_speed: f64,
}

impl Default for Unit {
    fn default() -> Self {
        Unit {
            tags: Vec::new(),
            strength: 10,
            agility: 10,
            constitution: 10,
            willpower: 10,
            wits: 10,
            cost_food: 0,
            hp: 0,
            speed: 0.0,
            run_pace: 1.5,
            footprint_depth: 1,
            footprint_width: 1,
            build_seconds: 1.0,
            preferred_position: Position::Front,
            position_priority: 0,
            detection_range: 40.0,
            sight_ahead: 1.0,
            sight_side: 0.5,
            sight_behind: 0.25,
            courage: 60,
            leadership: 0,
            tactics: Vec::new(),
            height: 1.0,
            initiative: 10,
            discipline: 30,
            turn_rate: 450.0,
            backward_pace: 0.4,
            melee_skill: 40,
            ranged_skill: 40,
            defence: 10,
            critical: 1.5,
            weaknesses: Vec::new(),
            resistances: Vec::new(),
            immunities: Vec::new(),
            armour: 0,
            ward: 0,
            regeneration: 0.0,
            regeneration_limit: 1.0,
            regeneration_stops: Vec::new(),
            items: Vec::new(),
            slots: Vec::new(),
            attack_speed: 1.0,
            traits: BTreeMap::new(),
            proficiencies: BTreeMap::new(),
        }
    }
}
