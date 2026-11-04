//! The record's commands given to the field in the crate: the kingdom's pursuit, a wave sent,
//! a route's squads retreating, a wave departing when full, waiting in the wood, tending,
//! hurrying and rerouted, and a record replayed as the digest plays it.

use super::super::tests::{content, fresh, played};
use super::*;
use crate::commands::HURRY;
use crate::record::{command, parse};
use crate::rehearsal::LIMIT_TICKS;

#[test]
fn the_kingdom_has_its_line_and_reserve_pursue_or_not() {
    let content = content();
    let (mut squads, mut field, mut play) = fresh(&content);
    let off = command(0, KINGDOM, "pursue", "all", "off");
    assert!(play.apply(&off, (&mut squads, &mut field), &content));
    for s in [0, 1] {
        assert!(!squads.commands.gives(squads.command[s].unwrap(), PURSUES), "squad {s}");
    }
    let on = command(0, KINGDOM, "pursue", "all", "on");
    assert!(play.apply(&on, (&mut squads, &mut field), &content));
    assert!(squads.commands.gives(squads.command[1].unwrap(), PURSUES));
    let send = command(0, KINGDOM, "send", "A", "");
    assert!(!play.apply(&send, (&mut squads, &mut field), &content), "the kingdom only pursues");
}

#[test]
fn a_retreat_orders_back_the_player_s_squads_on_the_wave_s_route_only() {
    let content = content();
    let (mut squads, mut field, mut play) = fresh(&content);
    played((&mut squads, &mut field, &mut play), &content, 60);
    let send = |w: &str| command(60, PLAYER, "send", w, "");
    assert!(play.apply(&send("A"), (&mut squads, &mut field), &content));
    assert!(play.apply(&send("B"), (&mut squads, &mut field), &content));
    let [a, b] = [play.play.listed[2], play.play.listed[3]];
    assert_eq!(play.routes[A].1, Some(squads.route_code[0]), "A shares the kingdom's route");

    assert!(play.apply(
        &command(60, PLAYER, "retreat", "A", ""),
        (&mut squads, &mut field),
        &content
    ));

    assert_eq!(squads.pending, vec![(squads.id[a], RETREAT)], "not the kingdom's, not B's");
    squads.pending.clear();
    assert!(play.apply(
        &command(60, PLAYER, "retreat", "B", ""),
        (&mut squads, &mut field),
        &content
    ));
    assert_eq!(squads.pending, vec![(squads.id[b], RETREAT)]);
    assert!(!play.apply(
        &command(60, PLAYER, "retreat", "C", ""),
        (&mut squads, &mut field),
        &content
    ));
}

#[test]
fn a_wave_is_set_to_depart_when_full_hurried_and_tending_by_its_route() {
    let content = content();
    let (mut squads, mut field, mut play) = fresh(&content);
    let mut give = |given: Command| play.apply(&given, (&mut squads, &mut field), &content);
    assert!(give(command(0, PLAYER, "auto", "B", "on")));
    assert!(give(command(0, PLAYER, "hurry", "A", "on")));
    assert!(give(command(0, PLAYER, "hurry", "A", "on")));
    assert!(give(command(0, PLAYER, "tend", "B", "carry")));
    assert!(give(command(0, PLAYER, "tend", "A", "recover")));
    assert!(give(command(0, PLAYER, "tend", "B", "leave")));
    assert!(!give(command(0, PLAYER, "tend", "B", "bury")));
    assert!(!give(command(0, PLAYER, "hurry", "C", "on")));
    assert!(!give(command(0, PLAYER, "march", "A", "")));
    assert!(play.waves[B].auto);
    assert_eq!(play.hurried, vec![A]);
    assert_eq!(play.tending, vec![(B, String::new()), (A, "recover".to_string())]);
    let mut give = |given: Command| play.apply(&given, (&mut squads, &mut field), &content);
    assert!(give(command(0, PLAYER, "hurry", "A", "off")));
    assert!(give(command(0, PLAYER, "auto", "B", "off")));
    assert!(play.hurried.is_empty());
    assert!(!play.waves[B].auto);
}

#[test]
fn a_wave_rerouted_onto_c_is_sent_along_it_and_its_squads_hurry_by_it() {
    let content = content();
    let (mut squads, mut field, mut play) = fresh(&content);
    let route = command(0, PLAYER, "route", "A", "C");
    assert!(play.apply(&route, (&mut squads, &mut field), &content));
    assert!(!play.apply(
        &command(0, PLAYER, "route", "A", "D"),
        (&mut squads, &mut field),
        &content
    ));
    assert_eq!(play.waves[A].route, C);
    played((&mut squads, &mut field, &mut play), &content, 30);
    let s = play.send(A, 0, (&mut squads, &mut field), &content).expect("a squad sent");
    assert_eq!(play.routes[C].1, Some(squads.route_code[s]), "C coded as first used");
    assert_ne!(play.routes[C].1, play.routes[A].1);
    assert!(play.apply(
        &command(31, PLAYER, "hurry", "A", "on"),
        (&mut squads, &mut field),
        &content
    ));
    played((&mut squads, &mut field, &mut play), &content, 1);
    assert!(squads.commands.gives(squads.command[s].unwrap(), HURRY));
}

#[test]
fn b_s_waves_wait_in_the_wood_for_a_s_and_are_sent_so() {
    let content = content();
    let (mut squads, mut field, mut play) = fresh(&content);
    assert!(play.apply(
        &command(0, PLAYER, "wait", "waves", "on"),
        (&mut squads, &mut field),
        &content
    ));
    let stage = play.waves[B].staging.clone().expect("B waits");
    assert_eq!((stage.at, stage.sees_fight, stage.partner, stage.fallback), (56.0, false, 0, 600));
    assert_eq!(stage.then.as_deref(), Some("go"));
    assert_eq!(stage.meet, Some((V2::new(79.0, 32.0), 88.0)));
    assert_eq!(play.waves[A].staging, None, "only B's wait");
    played((&mut squads, &mut field, &mut play), &content, 40);
    let s = play.send(B, 0, (&mut squads, &mut field), &content).expect("a squad sent");
    assert_eq!(squads.staging[s], Some(stage), "sent with its staging");
    assert!(play.apply(
        &command(40, PLAYER, "wait", "waves", "off"),
        (&mut squads, &mut field),
        &content
    ));
    assert_eq!(play.waves[B].staging, None);
}

#[test]
fn a_record_is_replayed_to_its_last_command_and_on() {
    let content = content();
    let (mut squads, mut field) = (Squads::default(), Field::default());
    let record = parse("seed 109563 captain off\n0 via_c on\n87 send A\n229 retreat A").unwrap();
    let mut log = Log::default();
    let crate_ = (&mut squads, &mut field);
    let played = replay(&record, 20, crate_, &content, (true, false), &mut log);
    let (ticks, produced) = played.expect("replayed");
    assert_eq!(ticks, 249);
    assert!(produced > 0, "the builders built");
    assert_eq!(squads.id.iter().filter(|&&id| id > 0).count(), 3, "the line, the reserve, A's");
    assert!(log.take().is_empty(), "each tick's events dropped");
}

#[test]
fn the_waves_sent_together_wait_as_their_rehearsals_say_so_both_arrive_at_once() {
    let content = content();
    let (mut squads, mut field, mut play) = fresh(&content);
    played((&mut squads, &mut field, &mut play), &content, 158);
    let rehearsed = [A, B].map(|w| play.rehearsed(w, &content));
    assert!(rehearsed.iter().all(|&t| t > 0 && t < LIMIT_TICKS), "{rehearsed:?}");
    assert_ne!(rehearsed[A], rehearsed[B], "B's route is the longer");

    let together = command(158, PLAYER, "send", "A+B", "");
    assert!(play.apply(&together, (&mut squads, &mut field), &content));

    let [a, b] = [play.play.listed[2], play.play.listed[3]];
    let longest = rehearsed[A].max(rehearsed[B]);
    let waits = [squads.wait_ticks[a], squads.wait_ticks[b]].map(i64::from);
    assert_eq!(waits, [longest - rehearsed[A], longest - rehearsed[B]]);
    assert!(play.waves.iter().all(|w| w.built() == 0), "both waves emptied");
}

#[test]
fn a_record_sending_the_waves_together_is_replayed() {
    let content = content();
    let (mut squads, mut field) = (Squads::default(), Field::default());
    let record = parse("seed 492625 captain off\n96 via_c on\n158 send A+B").unwrap();
    let mut log = Log::default();
    let crate_ = (&mut squads, &mut field);
    let played = replay(&record, 20, crate_, &content, (true, false), &mut log);
    assert_eq!(played.map(|p| p.0), Some(178));
    assert_eq!(squads.id.iter().filter(|&&id| id > 0).count(), 4, "the line, the reserve, A, B");
}
