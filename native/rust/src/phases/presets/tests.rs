use std::collections::HashMap;

use super::*;

fn given(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs.iter().map(|(key, value)| (key.to_string(), value.to_string())).collect()
}

#[test]
fn the_25k_preset_is_the_targets_series_of_24_squads_of_1024() {
    let mut args = given(&[("preset", "25k")]);
    expand(&mut args).expect("25k is a preset");
    assert_eq!(args["side"], "24576");
    assert_eq!(args["squads"], "24");
    assert_eq!(args["series"], "1");
    assert_eq!(args["limit"], LIMIT.to_string());
}

#[test]
fn the_4096_preset_is_the_same_series_at_4_squads_of_1024() {
    let mut args = given(&[("preset", "4096")]);
    expand(&mut args).expect("4096 is a preset");
    assert_eq!(args["side"], "4096");
    assert_eq!(args["squads"], "4");
    assert_eq!(args["series"], "1");
    assert_eq!(args["limit"], LIMIT.to_string());
}

#[test]
fn an_argument_given_beside_a_preset_wins() {
    let mut args = given(&[("preset", "25k"), ("threaded", "false"), ("limit", "50")]);
    expand(&mut args).expect("25k is a preset");
    assert_eq!(args["limit"], "50");
    assert_eq!(args["threaded"], "false");
    assert_eq!(args["side"], "24576");
}

#[test]
fn an_unknown_preset_is_refused() {
    let mut args = given(&[("preset", "50k")]);
    assert!(expand(&mut args).is_err());
}

#[test]
fn without_a_preset_the_arguments_are_unchanged() {
    let mut args = given(&[("side", "64")]);
    expand(&mut args).expect("no preset is fine");
    assert_eq!(args, given(&[("side", "64")]));
}
