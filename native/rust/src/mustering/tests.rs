//! The strays through the groups as one call: on squads built here (the player's, along a
//! route east from home at 0), it leaves the squads, the units and the log as the six passes
//! called one after another do, with GDScript's lists made afresh between them.

use super::*;
use crate::content::tests::repo_content;
use crate::content::Content;
use crate::locks::{DESTROYED, HOLDING};
use crate::maths::V2;
use crate::rout::tests::{east, member, SEED};

const TICK: i64 = 10;

pub(crate) fn rules(second: bool) -> Rules {
    let content = Content::load(&repo_content()).unwrap_or_else(|error| panic!("{error}"));
    let t = &content.tuning;
    let loads = LoadTuning {
        pace: t.load_pace.clone(),
        tiring: t.load_tiring.clone(),
        dodge: t.load_dodge.clone(),
    };
    Rules {
        morale: second.then(|| MoraleTuning::of(t)),
        rout: RoutTuning {
            pace: 1.0,
            tick_seconds: 0.1,
            fan_degrees: 45.0,
            fan_cells: 6.0,
            rally_reach: 1.5,
            steady_rally_seconds: 3.0,
            rally_seconds: 5.0,
            enemy_near: 6.0,
            reformed_morale: 30,
            steady_at: 50,
        },
        shocks: RoutShocks { crush: 2, panic: 5, seen: 7 },
        bands: BandTuning {
            per_leadership: 5,
            steady_at: 70,
            shaken_at: 40,
            loss: 2,
            leader_loss: 3,
        },
        carry: CarryTuning::of(t),
        loads,
        groups: split::Rules { lost: 2.0, join: 2.0, seed: SEED, cells_per_tile: 64.0 },
        tick: TICK,
        seed: SEED,
    }
}

#[derive(Default)]
struct Built {
    squads: Squads,
    field: Field,
    roster: Roster,
}

impl Built {
    /// A player's squad in `state` at `morale`, east along the route from home.
    fn squad(&mut self, state: i32, morale: i32) -> usize {
        let n = self.roster.listed.len();
        let squad = self.squads.open(n as i64 + 1) as usize;
        self.squads.squads_issued = 20;
        self.squads.faction[squad] = self.field.lexicon.code("player");
        self.squads.direction[squad] = 1;
        self.squads.state[squad] = state;
        self.squads.morale[squad] = morale;
        self.squads.route[squad] = east(100.0);
        self.roster.listed.push(Listed { squad, id: n as i64 + 1, ..Listed::default() });
        n
    }

    /// A unit of the listed squad `n` standing at `at`, at 10 hp.
    fn unit(&mut self, n: usize, id: i64, at: V2) -> usize {
        let slot = self.field.hold(id) as usize;
        let s = self.roster.listed[n].squad;
        let columns = &mut self.field.columns;
        columns.position[slot] = at;
        columns.speed[slot] = 1.0;
        columns.hp[slot] = 10;
        columns.sheet[slot].squad_id = self.squads.id[s];
        columns.words[slot].faction_id = self.squads.faction[s];
        self.roster.listed[n].living.push(member(id, slot));
        self.squads.members[s].push(slot as i32);
        slot
    }
}

/// What a call left: what it did, the log, the squads, and each unit's state, hp and place.
type Left = (Mustered, Vec<i64>, Squads, Vec<(i32, i32, V2)>);

fn left(b: &Built, mustered: Mustered, log: &mut Log) -> Left {
    let c = &b.field.columns;
    let units = (0..c.state.len()).map(|n| (c.state[n], c.hp[n], c.position[n])).collect();
    (mustered, log.take(), b.squads.clone(), units)
}

/// GDScript's lists as CrateRoster.of made them afresh: `b`'s roster, then a squad in each
/// slot `opened`, each with its living units now.
fn afresh(b: &Built, opened: &[usize]) -> Roster {
    let mut roster = b.roster.clone();
    for &squad in opened {
        roster.listed.push(Listed { squad, id: b.squads.id[squad], ..Listed::default() });
    }
    for listed in &mut roster.listed {
        let living = b.squads.living(listed.squad, &b.field.columns.state);
        listed.living = living
            .iter()
            .map(|&u| member(b.field.unit_at(u as u32).unwrap(), u as usize))
            .collect();
    }
    roster
}

/// The six passes, one call each, as FormationSimulation made them.
fn phased(b: &mut Built, strays: &[(usize, usize)], rules: &Rules) -> Left {
    let mut log = Log::default();
    let adopted = strays::adopt(&mut b.squads, &mut b.field, strays, SEED);
    let mut opened: Vec<usize> = adopted.iter().map(|&(_, s)| s).collect();
    let roster = afresh(b, &opened);
    if let Some(t) = &rules.morale {
        morale::step((&mut b.squads, &b.field.columns), &roster, (t, &rules.bands), TICK, &mut log);
    }
    let roster = afresh(b, &opened);
    let routs = roster
        .listed
        .iter()
        .any(|l| b.squads.state[l.squad] == ROUTING || b.squads.morale[l.squad] == 0);
    let mut routed = Vec::new();
    if routs {
        let tuning = (&rules.rout, &rules.shocks, &rules.bands);
        let (squads, field) = (&mut b.squads, &mut b.field);
        routed = rout::step(squads, field, &roster, tuning, (TICK, SEED), None, &mut log);
    }
    let homes: Vec<i64> = routed.iter().filter(|m| m.into.is_none()).map(|m| m.id).collect();
    let listed: Vec<usize> = roster.listed.iter().map(|l| l.squad).collect();
    let (squads, field) = (&mut b.squads, &mut b.field);
    let tuning = (&rules.carry, &rules.loads);
    let carried = carry::step(squads, field, (&listed, &homes), (TICK, SEED), tuning, &mut log);
    let setting_off: Vec<(usize, usize)> = carried
        .strays
        .iter()
        .map(|&(n, id)| (listed[n], b.field.slot_of(id).unwrap() as usize))
        .collect();
    let bearers = strays::adopt(&mut b.squads, &mut b.field, &setting_off, SEED);
    opened.extend(bearers.iter().map(|&(_, s)| s));
    let roster = afresh(b, &opened);
    let (squads, field) = (&mut b.squads, &mut b.field);
    let grouped = groups::step(squads, field, &roster, &rules.groups, TICK, &mut log);
    let split = groups::split_ids(&grouped, &roster);
    left(b, Mustered { adopted, routed, carried, bearers, grouped, split }, &mut log)
}

fn whole(b: &mut Built, strays: &[(usize, usize)], rules: &Rules) -> Left {
    let mut log = Log::default();
    let (squads, field) = (&mut b.squads, &mut b.field);
    let mustered = step(squads, field, &b.roster, strays, None, rules, &mut log);
    left(b, mustered, &mut log)
}

/// Squad 1 holding, its unit 2 ten cells out and its unit 1 a stray half a cell from home.
fn straying() -> (Built, Vec<(usize, usize)>) {
    let mut b = Built::default();
    let mine = b.squad(HOLDING, 60);
    let stray = b.unit(mine, 1, V2::new(0.5, 0.0));
    b.unit(mine, 2, V2::new(10.0, 0.0));
    let s = b.roster.listed[mine].squad;
    (b, vec![(s, stray)])
}

#[test]
fn a_stray_set_off_flees_home_the_same_tick_as_the_passes_one_by_one_have_it() {
    let rules = rules(true);
    let (mut one_by_one, strays) = straying();
    let (mut at_once, _) = straying();

    let expected = phased(&mut one_by_one, &strays, &rules);
    let got = whole(&mut at_once, &strays, &rules);

    assert_eq!(got, expected);
    let (mustered, _, squads, _) = &got;
    let (_, opened) = mustered.adopted[0];
    assert_eq!(mustered.routed, vec![Moved { listed: 1, id: 1, into: None }], "listed after");
    assert_eq!(squads.state[opened], DESTROYED, "home, it scattered");
}

#[test]
fn a_squad_at_no_morale_breaks_as_the_passes_one_by_one_have_it() {
    let rules = rules(false); // on a second, it would recover first
    let build = || {
        let mut b = Built::default();
        let mine = b.squad(HOLDING, 0);
        b.unit(mine, 1, V2::new(10.0, 0.0));
        let friend = b.squad(HOLDING, 60);
        b.unit(friend, 2, V2::new(12.0, 0.0));
        b
    };
    let (mut one_by_one, mut at_once) = (build(), build());

    let expected = phased(&mut one_by_one, &[], &rules);
    let got = whole(&mut at_once, &[], &rules);

    assert_eq!(got, expected);
    assert_eq!(got.2.state[0], ROUTING, "it broke");
}

#[test]
fn off_the_second_morale_does_not_step() {
    let (mut on, mut off) = (straying().0, straying().0);
    on.squads.morale[0] = -1;
    off.squads.morale[0] = -1;

    let stepped = whole(&mut on, &[], &rules(true));
    let unstepped = whole(&mut off, &[], &rules(false));

    assert_ne!(stepped.2.morale[0], -1, "read on a second");
    assert_eq!(unstepped.2.morale[0], -1);
}
