//! The scenario list read, each scenario's behaviours seen, and what is seen read from events.

use std::fs;

use super::*;
use crate::content::tests::repo_content;
use crate::events::{DISENGAGED, ENGAGED, FLANKED};
use crate::play;

fn content() -> Content {
    Content::load(&repo_content()).expect("the repo's content loads")
}

/// A content folder in the temp dir holding `list` as scenarios.json and one record, `a.record`.
fn folder_with(name: &str, list: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("breach_scenarios_{name}"));
    let folder = root.join(FOLDER);
    fs::create_dir_all(&folder).expect("a temp folder");
    fs::write(folder.join(LIST), list).expect("the list written");
    fs::write(folder.join("a.record"), "record 1 seed 1 captain off\n40 player send A")
        .expect("a record");
    root
}

#[test]
fn every_listed_scenario_shows_what_it_is_named_for_and_meets_its_limits() {
    let (content, list) = (content(), list(&repo_content()).expect("the list reads"));
    assert!(!list.is_empty());
    let most = (MOST_SECONDS / TICK_SECONDS) as i64;
    for scenario in &list {
        let (seen, ticks) = watch(scenario, &content).expect("it plays");
        assert!(seen.missing().is_empty(), "{}: not seen {:?}", scenario.name, seen.missing());
        let contact = seen.contact.expect("it reaches contact");
        assert!(contact <= 150, "{}: first contact at tick {contact}", scenario.name);
        assert!(ticks <= most, "{}: {ticks} ticks", scenario.name);
        assert!(scenario.name.starts_with("field_"), "{}", scenario.name);
    }
}

#[test]
fn the_list_covers_the_six_behaviours() {
    let list = list(&repo_content()).expect("the list reads");
    for behaviour in BEHAVIOURS {
        assert!(list.iter().any(|s| s.shows.iter().any(|b| b == behaviour)), "{behaviour}");
    }
}

#[test]
fn a_scenario_watched_twice_sees_the_same() {
    let (content, list) = (content(), list(&repo_content()).expect("the list reads"));
    let scenario = &list[0];
    assert_eq!(watch(scenario, &content), watch(scenario, &content));
}

#[test]
fn a_name_not_on_the_list_is_an_error_naming_it() {
    let list = list(&repo_content()).expect("the list reads");
    let wrong = named(&list, "field_nowhere").expect_err("not listed");
    assert!(wrong.contains("field_nowhere"), "{wrong}");
    assert!(named(&list, &list[0].name).is_ok());
}

#[test]
fn a_list_naming_an_unknown_map_behaviour_or_record_is_refused() {
    let cases = [
        (
            "map",
            r#"{"scenarios": [{"name": "x", "map": "moon", "record": "a.record", "shows": []}]}"#,
            "no map moon",
        ),
        (
            "shows",
            r#"{"scenarios": [{"name": "x", "map": "field", "record": "a.record", "shows": ["dancing"]}]}"#,
            "no behaviour dancing",
        ),
        (
            "record",
            r#"{"scenarios": [{"name": "x", "map": "field", "record": "b.record", "shows": []}]}"#,
            "b.record",
        ),
        (
            "twice",
            r#"{"scenarios": [{"name": "x", "map": "field", "record": "a.record", "shows": []}, {"name": "x", "map": "field", "record": "a.record", "shows": []}]}"#,
            "listed twice",
        ),
    ];
    for (name, text, says) in cases {
        let wrong = list(&folder_with(name, text)).expect_err(name);
        assert!(wrong.contains(says), "{name}: {wrong}");
    }
    let read =
        list(&folder_with("good", cases[0].1.replace("moon", "field").as_str())).expect("it reads");
    assert_eq!(read[0].record.seed, 1);
    assert_eq!(read[0].text, "record 1 seed 1 captain off\n40 player send A");
}

/// An event as the log writes it: kind, tick, squad, faction (0), then its fields.
fn event(kind: usize, tick: i64, squad: i64, fields: &[i64]) -> Vec<i64> {
    [&[kind as i64, tick, squad, 0][..], fields].concat()
}

#[test]
fn a_squad_fighting_a_second_foe_is_cornered_and_breaking_off_clears_its_foes() {
    let mut seen = Seen::new(&[]);
    let at = |seen: &mut Seen, tick: i64, flat: Vec<i64>| {
        for start in play::starts(&flat) {
            seen.event(tick, &flat[start..]);
        }
    };
    at(&mut seen, 10, event(ENGAGED, 10, 1, &[7]));
    at(&mut seen, 11, event(ENGAGED, 11, 1, &[7]));
    assert_eq!((seen.at("engaging"), seen.at("cornering")), (Some(10), None));
    at(&mut seen, 12, event(DISENGAGED, 12, 1, &[]));
    at(&mut seen, 13, event(FLANKED, 13, 1, &[8, 1]));
    assert_eq!((seen.at("breaking_off"), seen.at("flanking")), (Some(12), Some(13)));
    assert_eq!(seen.at("cornering"), None, "the first foe was left behind");
    at(&mut seen, 14, event(ENGAGED, 14, 2, &[7]));
    at(&mut seen, 15, event(ENGAGED, 15, 1, &[9]));
    assert_eq!(seen.at("cornering"), Some(15));
}

#[test]
fn a_timed_scenario_times_every_tick_it_plays_and_ends_as_played() {
    let (content, list) = (content(), list(&repo_content()).expect("the list reads"));
    let scenario = named(&list, "field_break_off").expect("listed");
    let (_, ticks) = watch(scenario, &content).expect("it plays");
    let timed = timed::timed(scenario, &content, Threads::One).expect("it plays timed");
    assert_eq!(timed.series.ticks, (0..ticks).collect::<Vec<_>>());
    assert!(!timed.series.spent.is_empty());
    assert!(timed.series.spent.iter().all(|(_, usec)| usec.len() == ticks as usize));
    assert!(!timed.apart, "the timed replay ended apart from the untimed one");
}

#[test]
fn timing_a_name_not_on_the_list_is_an_error_naming_it() {
    let content = content();
    let wrong = timed::named_timed(&repo_content(), &content, "field_nowhere", Threads::One)
        .expect_err("not listed");
    assert!(wrong.contains("field_nowhere"), "{wrong}");
}
