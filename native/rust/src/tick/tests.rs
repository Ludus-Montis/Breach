//! The whole tick as one call: on the fight's two grems a side (the aftermath's fixture), it
//! leaves the squads, the units and the log as the five calls one after another do, with
//! GDScript's lists made afresh between them (CrateRoster.of); and between the passes the
//! squads are listed as GDScript's lists had them.

use super::*;
use crate::aftermath::tests::fixture;
use crate::events::{DIED, DOWNED};
use crate::fight::tests::Built;
use crate::groups::pass::Landed;
use crate::maths::V2;
use crate::recovery::pass::Came;
use crate::roster::Listed;

fn rules() -> Rules<'static> {
    Rules {
        opening: crate::opening::tests::rules(),
        scrum: crate::scrum::step::tests::rules(),
        aftermath: crate::aftermath::tests::rules(),
        mustering: crate::mustering::tests::rules(true),
        ending: crate::ending::tests::rules(),
    }
}

/// GDScript's list as CrateRoster.of made it afresh: the squads in the slots `listed`, each
/// with its living units now, each unit as `everyone` first gave it, the factions coded in
/// the list's order.
fn afresh(b: &Built, everyone: &Roster, listed: &[usize]) -> Roster {
    let all = || everyone.listed.iter().flat_map(|l| l.living.iter());
    let member = |slot: usize| all().find(|m| m.slot == slot).cloned().unwrap();
    let mut factions: Vec<u32> = Vec::new();
    let mut out = Roster::default();
    for &squad in listed {
        let f = b.squads.faction[squad];
        let faction = factions.iter().position(|&c| c == f).unwrap_or_else(|| {
            factions.push(f);
            factions.len() - 1
        }) as u32;
        let living = b.squads.living(squad, &b.field.columns.state);
        let merges = everyone.listed.iter().any(|l| l.squad == squad && l.merges);
        out.listed.push(Listed {
            squad,
            id: b.squads.id[squad],
            faction,
            joined: !b.squads.joined[squad].is_empty(),
            merges,
            living: living.iter().map(|&u| member(u as usize)).collect(),
        });
    }
    out
}

/// What a tick left: the log, the squads, and each unit's state, hp and place.
type Left = (Vec<i64>, Squads, Vec<(i32, i32, V2)>);

fn left(b: &Built, log: &mut Log) -> Left {
    let c = &b.field.columns;
    let units = (0..c.state.len()).map(|n| (c.state[n], c.hp[n], c.position[n])).collect();
    (log.take(), b.squads.clone(), units)
}

/// The five calls, one after another, as FormationSimulation made them, GDScript's lists
/// made afresh between them.
fn phased(b: &mut Built, rules: &Rules, log: &mut Log) {
    let everyone = b.roster.clone();
    let mut listed: Vec<usize> = everyone.listed.iter().map(|l| l.squad).collect();
    let roster = afresh(b, &everyone, &listed);
    let (squads, field) = (&mut b.squads, &mut b.field);
    let opened = opening::step(squads, field, &roster, None, &rules.opening, log);
    let gone: Vec<usize> = opened.joins.iter().map(|&(_, j)| listed[j]).collect();
    listed.retain(|s| !gone.contains(s));
    let roster = afresh(b, &everyone, &listed);
    scrum::step::step(&mut b.squads, &mut b.field, &roster, None, &rules.scrum, log);
    let roster = afresh(b, &everyone, &listed);
    let (squads, field) = (&mut b.squads, &mut b.field);
    let after = aftermath::step(squads, field, &roster, None, &rules.aftermath, log);
    let alone = after.recovered.iter().filter(|l| matches!(l.came, Came::Alone));
    let strays: Vec<(usize, usize)> =
        alone.map(|l| (listed[l.listed], b.field.slot_of(l.id).unwrap() as usize)).collect();
    let roster = afresh(b, &everyone, &listed);
    let (squads, field) = (&mut b.squads, &mut b.field);
    let mustered = mustering::step(squads, field, &roster, &strays, None, &rules.mustering, log);
    listed.extend(mustered.adopted.iter().chain(&mustered.bearers).map(|&(_, s)| s));
    let mut taken = Vec::new();
    for done in &mustered.grouped {
        match *done {
            Landed::Split { slot, .. } => listed.push(slot),
            Landed::Taken { taken: n, .. } => taken.push(listed[n]),
            Landed::Returned { .. } => {}
        }
    }
    listed.retain(|s| !taken.contains(s));
    ending::step(&mut b.squads, &mut b.field, &listed, None, &rules.ending, log);
}

fn whole(b: &mut Built, rules: &Rules, log: &mut Log) -> Ticked {
    step(&mut b.squads, &mut b.field, &b.roster, None, rules, log)
}

/// The kinds in a flat log, in order.
fn kinds(flat: &[i64]) -> Vec<usize> {
    let mut out = vec![];
    let mut at = 0;
    while at < flat.len() {
        let kind = flat[at] as usize;
        out.push(kind);
        at += 4 + crate::events::KINDS[kind].1.len();
    }
    out
}

fn both(hp: i32, routing: bool, ticks: usize) -> (Left, Left) {
    let rules = rules();
    let (mut one_by_one, mut at_once) = (fixture(hp, routing), fixture(hp, routing));
    let (mut expected, mut got) = (Log::default(), Log::default());
    for _ in 0..ticks {
        phased(&mut one_by_one, &rules, &mut expected);
        whole(&mut at_once, &rules, &mut got);
    }
    (left(&one_by_one, &mut expected), left(&at_once, &mut got))
}

#[test]
fn the_one_step_leaves_what_the_five_calls_one_by_one_leave_when_a_unit_falls() {
    let (expected, got) = both(3, false, 1);

    assert_eq!(got, expected);
    let fell = kinds(&got.0).into_iter().filter(|&k| k == DIED || k == DOWNED).count();
    assert_eq!(fell, 1, "201 fell: the aftermath ran");
}

#[test]
fn a_routing_squad_that_gives_up_leaves_the_same_as_the_calls_one_by_one() {
    let (expected, got) = both(3, true, 1);

    assert_eq!(got, expected);
    assert!(!got.0.is_empty(), "the passes told of it");
}

#[test]
fn later_ticks_of_the_one_step_match_the_calls_one_by_one_too() {
    let (expected, got) = both(30, false, 3);

    assert_eq!(got, expected);
}

#[test]
fn the_squads_a_join_took_in_leave_the_list_and_the_factions_are_coded_again() {
    let b = fixture(20, false);
    let known = known(&b.roster);

    let roster = relisted(&b.roster, &b.squads, &b.field, &known, &[0], &[]);

    let listed: Vec<(usize, u32)> = roster.listed.iter().map(|l| (l.squad, l.faction)).collect();
    assert_eq!(listed, vec![(1, 0)], "the second squad alone, its faction the first listed");
    assert_eq!(roster.listed[0].living.len(), 2);
}

#[test]
fn a_unit_down_as_the_tick_began_is_listed_once_it_stands_again() {
    let mut b = fixture(20, false);
    let known = known(&b.roster);
    let slot = b.slot(201);
    b.field.columns.state[slot] = crate::squads::DOWNED;
    let down = relisted(&b.roster, &b.squads, &b.field, &known, &[], &[]);
    b.field.columns.state[slot] = 0;

    let up = relisted(&down, &b.squads, &b.field, &known, &[], &[]);

    let ids = |r: &Roster| r.listed[1].living.iter().map(|m| m.id).collect::<Vec<_>>();
    assert_eq!((ids(&down), ids(&up)), (vec![200], vec![200, 201]));
}

#[test]
fn the_end_lists_the_squads_opened_after_the_rest_less_those_taken_in() {
    let b = fixture(20, false);
    let mustered = Mustered {
        adopted: vec![(0, 5)],
        bearers: vec![(0, 6)],
        grouped: vec![
            Landed::Split { from: 0, slot: 7, members: vec![0] },
            Landed::Taken { taker: 0, taken: 2 },
        ],
        ..Mustered::default()
    };

    assert_eq!(ended_listing(&b.roster, &mustered), vec![0, 1, 6, 7]);
}
