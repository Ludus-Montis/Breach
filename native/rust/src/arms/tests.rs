use std::collections::BTreeMap;

use super::*;
use crate::content::tests::repo_content;
use crate::content::{Armour, Content, Item, Weapon};

/// The repo's content, with a buckler (a shield granting hauler 1), a pike (needing more
/// strength than ordinary, untrained) and fists to go with them.
pub(crate) fn content() -> Content {
    let mut content = Content::load(&repo_content()).unwrap_or_else(|error| panic!("{error}"));
    let grants = BTreeMap::from([("hauler".to_string(), 1)]);
    let buckler = Item {
        slots: vec!["hand".into()],
        weight: 4.0,
        tags: vec!["shield".into()],
        grants,
        ..Item::default()
    };
    content.armour.insert("buckler".into(), Armour { item: buckler, armour: 2, ward: 1 });
    let pike = Item {
        slots: vec!["hand".into()],
        weight: 6.0,
        strength_requirement: 13,
        tags: vec!["pike".into()],
        ..Item::default()
    };
    let pike = Weapon { item: pike, damage: 8, damage_floor: 0.25, ..Weapon::default() };
    content.weapons.insert("pike".into(), pike);
    let fists = Item {
        slots: vec!["hand".into(), "hand".into()],
        innate: true,
        traits: BTreeMap::from([("wounding".to_string(), 2)]),
        ..Item::default()
    };
    content.weapons.insert("fists".into(), Weapon { item: fists, damage: 2, ..Weapon::default() });
    content
}

fn arms<'a>(content: &'a Content, unit: &'a Unit) -> Arms<'a> {
    Arms::of(content, unit)
}

fn unit(content: &Content, id: &str) -> Unit {
    content.units[id].clone()
}

/// A militia man with a pike and a buckler, and fists his hands are too full for.
fn piker(content: &Content) -> Unit {
    let mut piker = unit(content, "kingdom_militia");
    piker.items = vec!["weapons/pike".into(), "armour/buckler".into(), "weapons/fists".into()];
    piker
}

#[test]
fn a_grem_strikes_with_its_bite_and_claw_together_and_cant_parry() {
    let content = content();
    let grem = unit(&content, "grem");
    let arms = arms(&content, &grem);
    let parts: Vec<_> = arms.parts(false);
    assert_eq!(parts, vec![(3.0, "piercing", false), (3.0, "slashing", false)]);
    assert!(arms.parts(true).is_empty() && arms.ranged_weapon().is_none());
    assert!(!arms.can_parry());
    assert_eq!(
        (arms.melee_seconds(), arms.floor_share(false), arms.floor_share(true)),
        (1.0, 0.5, 1.0)
    );
    assert_eq!((arms.carried_weight(), arms.load_stage(), arms.protection()), (0.0, 0, (0, 0)));
}

#[test]
fn a_brute_s_fists_hit_harder_for_its_strength_over_their_need() {
    let content = content();
    let brute = unit(&content, "grem_brute");
    let arms = arms(&content, &brute);
    assert_eq!(arms.parts(false), vec![(7.0, "bludgeoning", false), (3.0, "piercing", false)]);
    assert_eq!(arms.skill(false), 35);
}

#[test]
fn a_spitter_shoots_its_hardest_hitting_ranged_weapon() {
    let content = content();
    let spitter = unit(&content, "grem_spitter");
    let arms = arms(&content, &spitter);
    assert_eq!(arms.ranged_weapon().map(|w| w.attack_range), Some(5));
    assert_eq!(arms.parts(true), vec![(4.0, "acid", false)]);
    assert_eq!(arms.parts(false).len(), 2);
}

#[test]
fn a_skill_shifts_with_proficiency_trained_mastered_and_untrained() {
    let content = content();
    let tuning = &content.tuning;
    let militia = unit(&content, "kingdom_militia");
    let captain = unit(&content, "kingdom_captain");
    assert_eq!(arms(&content, &militia).skill(false), 40);
    let mastered = (55.0 + tuning.item_mastered_skill).round() as i64;
    assert_eq!(arms(&content, &captain).skill(false), mastered);
    let piker = piker(&content);
    let short = 13 - piker.strength;
    let untrained = 40.0 + tuning.item_untrained_skill - short as f64 * tuning.item_weak_skill;
    assert_eq!(arms(&content, &piker).skill(false), (untrained.round() as i64).max(0));
}

#[test]
fn hands_full_leave_the_fists_unused_and_a_shield_parries_and_protects() {
    let content = content();
    let piker = piker(&content);
    let arms = arms(&content, &piker);
    let used: Vec<_> = arms.weapons().iter().map(|w| w.damage).collect();
    assert_eq!(used, vec![8]);
    assert!(arms.can_parry());
    assert_eq!(arms.protection(), (2, 1));
    assert_eq!((arms.carried_weight(), arms.floor_share(false)), (10.0, 0.25));
}

#[test]
fn a_trait_an_item_grants_is_its_carrier_s_at_the_higher_level() {
    let content = content();
    let mut piker = piker(&content);
    assert_eq!(
        (
            arms(&content, &piker).trait_level("hauler"),
            arms(&content, &piker).trait_level("climber")
        ),
        (1, 0)
    );
    piker.traits.insert("hauler".into(), 3);
    assert_eq!(arms(&content, &piker).trait_level("hauler"), 3);
}

#[test]
fn the_load_stage_rises_with_the_weight_against_the_carry_limit() {
    let content = content();
    let tuning = &content.tuning;
    let limit = 10.0 * tuning.load_per_strength;
    let most = tuning.load_most + tuning.load_hauler;
    assert_eq!(stage_of(limit * tuning.load_easy, 10, 0, tuning), 0);
    assert_eq!(stage_of(limit, 10, 0, tuning), 1);
    assert_eq!(stage_of(limit * most, 10, 1, tuning), 2);
    assert_eq!(stage_of(limit * most + 0.5, 10, 1, tuning), 3);
}
