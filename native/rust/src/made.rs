//! A unit made from its definition (spec 39 step 8c-iii; GDScript's FormationUnits.make and
//! TerrainWalker.of): its numbers (`Sheet`), its words and lists (`Words`, coded in the
//! field's lexicon in the order UnitWrites.all writes them: the words, the lists, the
//! levels, then the parts) and its walker's fields, as the store holds a unit once it is
//! admitted. Its footprint, preferred band, courage and the rest are its definition's; its
//! melee strike (all its melee weapons together), its best ranged weapon, its skills, its
//! armour and ward, and its pace, tiring and dodge under its load are reckoned from what it
//! carries (`arms`). Its place, hp, stamina and squad are its squad's to give (8c-iv). Pure.

use crate::arms::Arms;
use crate::content::{Content, Unit};
use crate::sheet::Sheet;
use crate::words::{Lexicon, Part, Words};

#[cfg(test)]
mod tests;

/// The attributes a unit's sheet lists, in order (UnitDef.ATTRIBUTES).
const ATTRIBUTES: [&str; 5] = ["strength", "agility", "constitution", "willpower", "wits"];

/// A unit as its definition makes it.
#[derive(Clone, Debug, PartialEq)]
pub struct Made {
    pub sheet: Sheet,
    pub words: Words,
    /// TerrainWalker.fields: [height, swimmer, climber, swims, climbs].
    pub walker: [f64; 5],
}

/// A unit of `faction` made from `unit` (a definition in `content`), advancing in
/// `direction` (+1 toward the kingdom's end, -1 toward the player's); its words coded in
/// `lexicon`.
pub fn unit(
    content: &Content,
    unit: &Unit,
    faction: &str,
    direction: i64,
    lexicon: &mut Lexicon,
) -> Made {
    let arms = Arms::of(content, unit);
    Made {
        sheet: sheet(content, unit, &arms, direction),
        words: words(unit, &arms, faction, lexicon),
        walker: walker(unit, &arms),
    }
}

/// Its numbers: its definition's, with its arms and load reckoned.
fn sheet(content: &Content, unit: &Unit, arms: &Arms, direction: i64) -> Sheet {
    let tuning = &content.tuning;
    let melee = arms.parts(false);
    let ranged = arms.ranged_weapon();
    let stage = arms.load_stage();
    let (armour, ward) = arms.protection();
    let weapons = arms.weapons();
    let wounding = weapons.iter().filter_map(|w| w.item.traits.get("wounding").copied());
    let attack_seconds = ranged.map_or(1.0, |w| w.attack_seconds / unit.attack_speed);
    Sheet {
        max_hp: unit.hp,
        dmg: melee.iter().fold(0.0, |sum, part| sum + part.0).round() as i64,
        detection: unit.detection_range,
        courage: unit.courage,
        leadership: unit.leadership,
        height: unit.height,
        initiative: unit.initiative,
        discipline: unit.discipline,
        turn_rate: unit.turn_rate,
        backward_pace: unit.backward_pace,
        advance_direction: direction,
        footprint_depth: unit.footprint_depth,
        footprint_width: unit.footprint_width,
        preferred_position: unit.preferred_position as i64,
        position_priority: unit.position_priority,
        attack_range: ranged.map_or(0, |w| w.attack_range),
        ranged_dmg: arms.parts(true).first().map_or(0, |part| part.0.round() as i64),
        melee_skill: arms.skill(false),
        ranged_skill: arms.skill(true),
        defence: unit.defence,
        critical: unit.critical,
        parries: arms.can_parry(),
        melee_floor: arms.floor_share(false),
        ranged_floor: arms.floor_share(true),
        armour,
        ward,
        max_stamina: unit.constitution as f64 * tuning.stamina_per_constitution,
        fresh_speed: unit.speed * tuning.load_pace[stage],
        tiring: tuning.load_tiring[stage],
        dodging: tuning.load_dodge[stage],
        run_pace: unit.run_pace,
        load_stage: stage as i64,
        regeneration: unit.regeneration,
        wounding: wounding.fold(0, i64::max),
        gear_weight: arms.carried_weight(),
        melee_seconds: arms.melee_seconds(),
        ranged_seconds: attack_seconds,
        ..Sheet::default()
    }
}

/// Its words and lists, coded in the order UnitWrites.all writes them. Its traits are its
/// definition's, in order, then those its items grant it that it hasn't, each at its level
/// (Decision 128).
fn words(unit: &Unit, arms: &Arms, faction: &str, lexicon: &mut Lexicon) -> Words {
    let ranged = arms.ranged_weapon();
    let faction_id = lexicon.code(faction);
    let damage_type = lexicon.code(ranged.map_or("", |w| w.damage_type.as_str()));
    let mut list = |words: &[String]| words.iter().map(|w| lexicon.code(w)).collect::<Vec<_>>();
    let tags = list(&unit.tags);
    let tactics = list(&unit.tactics);
    let weaknesses = list(&unit.weaknesses);
    let resistances = list(&unit.resistances);
    let immunities = list(&unit.immunities);
    let regeneration_stops = list(&unit.regeneration_stops);
    let levels = [unit.strength, unit.agility, unit.constitution, unit.willpower, unit.wits];
    let attributes =
        ATTRIBUTES.iter().zip(levels).map(|(name, level)| (lexicon.code(name), level as f64));
    let attributes = attributes.collect();
    let mut traits: Vec<(String, i64)> = unit.traits.clone().into_iter().collect();
    for granted in arms.granted() {
        let level = arms.trait_level(granted);
        if level > 0 {
            match traits.iter_mut().find(|(name, _)| name == granted) {
                Some(held) => held.1 = level,
                None => traits.push((granted.to_string(), level)),
            }
        }
    }
    let traits = traits.iter().map(|(name, level)| (lexicon.code(name), *level as f64)).collect();
    let mut parts = |ranged: bool| {
        let parts = arms.parts(ranged).into_iter();
        let part = |(damage, kind, magical)| Part { damage, kind: lexicon.code(kind), magical };
        parts.map(part).collect::<Vec<_>>()
    };
    let melee_parts = parts(false);
    let ranged_parts = parts(true);
    Words {
        faction_id,
        damage_type,
        tags,
        tactics,
        weaknesses,
        resistances,
        immunities,
        regeneration_stops,
        attributes,
        traits,
        melee_parts,
        ranged_parts,
    }
}

/// How it crosses terrain (TerrainWalker.of): its height, its swimmer and climber levels,
/// and whether it swims and climbs at all (not when too laden, nor when it sinks or can't
/// climb).
fn walker(unit: &Unit, arms: &Arms) -> [f64; 5] {
    let free = !arms.too_heavy_for_modes();
    let swims = free && arms.trait_level("sinks") == 0;
    let climbs = free && arms.trait_level("cant_climb") == 0;
    let (swimmer, climber) = (arms.trait_level("swimmer"), arms.trait_level("climber"));
    [unit.height, swimmer as f64, climber as f64, f64::from(swims), f64::from(climbs)]
}
