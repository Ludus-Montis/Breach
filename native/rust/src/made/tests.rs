use super::*;
use crate::arms::tests::content;

fn made(content: &Content, id: &str, lexicon: &mut Lexicon) -> Made {
    unit(content, &content.units[id], "player", -1, lexicon)
}

/// `levels` named by their words.
fn named<'a>(lexicon: &'a Lexicon, levels: &[(u32, f64)]) -> Vec<(&'a str, f64)> {
    levels.iter().map(|&(code, level)| (lexicon.word(code), level)).collect()
}

#[test]
fn a_grem_s_sheet_is_its_definition_s_with_its_arms_and_load_reckoned() {
    let content = content();
    let tuning = &content.tuning;
    let made = made(&content, "grem", &mut Lexicon::default());
    let sheet = &made.sheet;
    assert_eq!((sheet.max_hp, sheet.dmg, sheet.ranged_dmg, sheet.attack_range), (20, 6, 0, 0));
    assert_eq!((sheet.position_priority, sheet.preferred_position), (1, 0));
    assert_eq!((sheet.turn_rate, sheet.backward_pace), (720.0, 0.6));
    assert_eq!((sheet.advance_direction, sheet.squad_id, sheet.parries), (-1, 0, false));
    assert_eq!(sheet.fresh_speed, 1.0 * tuning.load_pace[0]);
    assert_eq!((sheet.tiring, sheet.dodging), (tuning.load_tiring[0], tuning.load_dodge[0]));
    assert_eq!(sheet.max_stamina, 10.0 * tuning.stamina_per_constitution);
    assert_eq!((sheet.melee_floor, sheet.ranged_floor, sheet.melee_seconds), (0.5, 1.0, 1.0));
    assert_eq!(made.walker, [1.0, 0.0, 0.0, 1.0, 1.0]);
}

#[test]
fn its_words_are_coded_in_the_order_the_store_writes_them() {
    let content = content();
    let mut lexicon = Lexicon::default();
    let made = made(&content, "grem_chieftain", &mut lexicon);
    let words = &made.words;
    assert_eq!((lexicon.word(words.faction_id), lexicon.word(words.damage_type)), ("player", ""));
    assert_eq!(lexicon.words()[1..4], ["player".to_string(), "grem".into(), "leader".into()]);
    assert_eq!(words.tactics.iter().map(|&c| lexicon.word(c)).collect::<Vec<_>>(), ["coordinated"]);
    let attributes = named(&lexicon, &words.attributes);
    let expected = [("strength", 10.0), ("agility", 10.0), ("constitution", 10.0)];
    assert_eq!(attributes[..3], expected);
    assert_eq!(attributes.len(), 5);
    assert!(words.traits.is_empty());
    let parts: Vec<_> =
        words.melee_parts.iter().map(|p| (p.damage, lexicon.word(p.kind))).collect();
    assert_eq!(parts, [(3.0, "piercing"), (3.0, "slashing")]);
    assert_eq!(made.sheet.preferred_position, 1);
}

#[test]
fn a_shooter_takes_its_reach_damage_type_and_interval_from_its_best_ranged_weapon() {
    let content = content();
    let mut lexicon = Lexicon::default();
    let made = made(&content, "grem_spitter", &mut lexicon);
    assert_eq!(
        (made.sheet.attack_range, made.sheet.ranged_dmg, made.sheet.ranged_seconds),
        (5, 4, 1.0)
    );
    assert_eq!(lexicon.word(made.words.damage_type), "acid");
    assert_eq!((made.sheet.dmg, made.sheet.ranged_skill), (2, 45));
}

#[test]
fn what_an_item_grants_and_wears_is_on_its_carrier_s_sheet() {
    let mut content = content();
    let mut piker = content.units["kingdom_militia"].clone();
    piker.items = vec!["weapons/pike".into(), "armour/buckler".into(), "weapons/fists".into()];
    content.units.insert("piker".into(), piker);
    let tuning = content.tuning.clone();
    let mut lexicon = Lexicon::default();
    let made = made(&content, "piker", &mut lexicon);
    let sheet = &made.sheet;
    assert_eq!(named(&lexicon, &made.words.traits), [("hauler", 1.0)]);
    assert_eq!((sheet.armour, sheet.ward, sheet.parries, sheet.wounding), (2, 1, true, 0));
    assert_eq!((sheet.gear_weight, sheet.dmg, sheet.melee_floor), (10.0, 8, 0.25));
    let stage = crate::arms::stage_of(10.0, 10, 1, &tuning);
    assert_eq!(sheet.load_stage, stage as i64);
    assert_eq!(sheet.fresh_speed, 0.8 * tuning.load_pace[stage]);
    let limit = 10.0 * tuning.load_per_strength;
    let easy = limit * tuning.load_easy;
    let burdened = 10.0 > easy + (limit - easy) * tuning.ground_mode_load_share;
    let moves = if burdened { 0.0 } else { 1.0 };
    assert_eq!(made.walker, [1.0, 0.0, 0.0, moves, moves]);
}
