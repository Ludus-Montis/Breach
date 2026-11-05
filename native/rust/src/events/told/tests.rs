//! The log decoded as CrateEvents.taken decodes it: the head of each event, a unit's kind,
//! the fields named by code, the bits of a heading, a list, and a "joined" of none left out.

use super::*;
use crate::events::{HIT, SWAPPED};

fn index(kind: &str) -> i64 {
    KINDS.iter().position(|(k, _)| *k == kind).expect("a kind") as i64
}

fn words() -> Vec<String> {
    ["player", "the_kingdom", "go", "slash"].iter().map(|w| w.to_string()).collect()
}

#[test]
fn a_hit_is_told_of_its_unit_with_its_fields_named() {
    let fields = KINDS[HIT].1;
    let mut flat = vec![HIT as i64, 12, 77, 1];
    let values: Vec<i64> = fields
        .iter()
        .map(|&f| match f {
            "flank" => 1,
            "blow" => 4,
            "damage_type" => 3,
            _ => 5,
        })
        .collect();
    flat.extend(&values);
    let told = decoded(&flat, &words(), &[]);
    assert_eq!(told.len(), 1);
    let hit = &told[0];
    assert_eq!((hit.kind, hit.tick, hit.of_unit, hit.who), ("hit", 12, true, 77));
    assert_eq!(hit.faction, "the_kingdom");
    assert_eq!(hit.fields.len(), fields.len());
    for (name, said) in &hit.fields {
        let expected = match *name {
            "flank" => Said::Bool(true),
            "blow" => Said::Word("critical".into()),
            "damage_type" => Said::Word("slash".into()),
            _ => Said::Int(5),
        };
        assert_eq!(said, &expected, "{name}");
    }
}

#[test]
fn orders_bands_headings_and_definitions_are_decoded_and_a_list_is_its_numbers() {
    let order = index("order_applied");
    let mut flat = vec![order, 3, 9, 0, 2];
    flat.extend([index("faced"), 4, 9, 0, 1.5f64.to_bits() as i64, 6]);
    flat.extend([SWAPPED as i64, 4, 9, 0, 6, 2, 30, 31]);
    let told = decoded(&flat, &words(), &[]);
    assert_eq!(told.len(), 3);
    assert_eq!(told[0].kind, "order_applied");
    assert_eq!(told[0].fields, vec![("order", Said::Word("RETREAT".into()))]);
    assert!(!told[0].of_unit);
    assert_eq!(told[1].fields, vec![("heading", Said::Real(1.5)), ("facing", Said::Int(6))]);
    let last = told[2].fields.last().expect("a list");
    assert_eq!(last, &("passed", Said::List(vec![30, 31])));
    let band = KINDS.iter().position(|(_, f)| f.contains(&"band")).expect("a band's kind");
    let mut flat = vec![band as i64, 1, 2, 0];
    flat.extend(KINDS[band].1.iter().map(|&f| if f == "band" { 1 } else { 0 }));
    let told = decoded(&flat, &words(), &[]);
    assert!(told[0].fields.contains(&("band", Said::Word("SHAKEN".into()))));
}

#[test]
fn a_definition_is_its_id_and_a_joined_of_none_is_left_out() {
    let fled = index("fled_home");
    let fields = KINDS[fled as usize].1;
    let mut flat = vec![fled, 8, 2, 0];
    flat.extend(fields.iter().map(|&f| if f == "definition" { 1 } else { 0 }));
    let told = decoded(&flat, &words(), &["grem", "grem_chieftain"]);
    assert!(told[0].fields.contains(&("definition", Said::Word("grem_chieftain".into()))));
    let joined = KINDS.iter().position(|(_, f)| f.contains(&"joined")).expect("a join's kind");
    let mut flat = vec![joined as i64, 1, 2, 0];
    flat.extend(KINDS[joined].1.iter().map(|&f| if f == "joined" { -1 } else { 0 }));
    let told = decoded(&flat, &words(), &[]);
    assert!(told[0].fields.iter().all(|(name, _)| *name != "joined"));
    assert_eq!(told[0].fields.len(), KINDS[joined].1.len() - 1);
}
