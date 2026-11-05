//! The digest's runs as formation_digest.gd names, plays and prints them.

use super::*;
use crate::content::tests::repo_content;
use crate::digest;

fn content() -> Content {
    Content::load(&repo_content()).expect("the repo's content loads")
}

fn labels(set: &str) -> Vec<String> {
    runs(set).expect("a set the digest knows").into_iter().map(|run| run.label).collect()
}

#[test]
fn the_standard_set_is_formation_digest_s_runs_in_its_order() {
    let mut expected = Vec::new();
    for rolls in ["rtrue", "rfalse"] {
        for scenario in ["mirror_headon", "mirror_flank"] {
            for seed in 1..=3 {
                expected.push(format!("{scenario} s{seed} {rolls}"));
            }
        }
        for index in 0..5 {
            expected.push(format!("field{index} {rolls}"));
        }
        for seed in 1..=2 {
            expected.push(format!("clash40 s{seed} {rolls}"));
        }
    }
    assert_eq!(labels("standard"), expected);
}

#[test]
fn the_clash160_and_wide_sets_are_the_digest_s_too() {
    assert_eq!(labels("clash160"), ["clash160 s1 rtrue"]);
    assert_eq!(labels("wide"), ["wide96 s1 rtrue", "wide96 s2 rfalse"]);
    assert!(runs("no_such_set").is_none());
}

#[test]
fn a_run_s_line_and_its_ticks_lines_are_printed_as_the_godot_digest_prints_them() {
    let hash = [0xab; 32];
    let hex = "ab".repeat(32);
    assert_eq!(
        line("mirror_flank s2 rfalse", 87, &hash),
        format!("mirror_flank s2 rfalse{} ticks   87  {hex}", " ".repeat(10))
    );
    assert_eq!(tick_line("field3 rtrue", 12, &hash), "field3 rtrue 12 abababababababab");
}

#[test]
fn the_chain_is_sha_256() {
    let mut chain = Chain::default();
    chain.update(b"ab");
    chain.update(b"c");
    let expected = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
    assert_eq!(hex(&chain.finish()), expected);
    assert_eq!(hex(&Chain::of(b"abc")), expected);
}

/// The run's hash chains each tick's record as `digest::battle` gives it, and each tick is told
/// with its record's own hash.
#[test]
fn a_battle_s_hash_chains_every_tick_s_record() {
    let content = content();
    let mut run = runs("standard").unwrap().remove(0);
    run.limit = 4;
    let mut told = Vec::new();
    let (ticks, hash) = run.digest(&content, false, &mut |tick, one, _| told.push((tick, *one)));

    let (mut squads, mut field, mut log) = (Squads::default(), Field::default(), Log::default());
    let battle = crate::battles::named("mirror_headon", &[0]).unwrap();
    let mut play =
        Play::new(&battle, (&mut squads, &mut field), &content, (1, true, false)).unwrap();
    let mut records = Vec::new();
    let mut watch = Watch::new(4, None);
    digest::battle(
        &mut play,
        (&mut squads, &mut field),
        &content,
        &mut log,
        &mut watch,
        &mut |r| records.push(r.record()),
    );
    assert_eq!(ticks, 4);
    assert_eq!(hash, Some(Chain::of(&records.concat())));
    let each: Vec<_> =
        records.iter().enumerate().map(|(k, r)| (k as i64 + 1, Chain::of(r))).collect();
    assert_eq!(told, each);
}

/// A field run plays its record `limit` ticks past its last command, as `digest::field` does.
#[test]
fn a_field_s_hash_chains_every_tick_s_record() {
    let content = content();
    let text = "record 1 seed 515557 captain off\n0 kingdom pursue all off\n2 player send B";
    let run = Run { label: "field r".into(), kind: Kind::Field(text), rolls: false, limit: 3 };
    let (ticks, hash) = run.digest(&content, false, &mut |_, _, _| {});

    let (mut squads, mut field, mut log) = (Squads::default(), Field::default(), Log::default());
    let record = crate::record::parse(text).unwrap();
    let mut records = Vec::new();
    let crate_ = (&mut squads, &mut field);
    digest::field(&record, 3, crate_, &content, (false, false), &mut log, &mut |r| {
        records.push(r.record())
    });
    assert_eq!(ticks, 5);
    assert_eq!(hash, Some(Chain::of(&records.concat())));
}

#[test]
fn a_run_that_can_t_be_set_up_has_no_hash() {
    let content = content();
    let run =
        Run { label: "field r".into(), kind: Kind::Field("not a record"), rolls: true, limit: 3 };
    assert_eq!(run.digest(&content, false, &mut |_, _, _| {}), (0, None));
}
