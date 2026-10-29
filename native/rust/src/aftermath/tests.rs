//! The fight and its aftermath as one call: on the fight's two grems a side
//! (`fight::tests::built`), it leaves the squads, the units and the log as the five passes
//! called one after another do.

use super::*;
use crate::content::tests::repo_content;
use crate::content::Content;
use crate::events::{DIED, DOWNED};
use crate::fight::tests::{built, landing, rules as choosing, Built};
use crate::locks::{FIGHTING, ROUTING};

fn rules() -> Rules {
    let content = Content::load(&repo_content()).unwrap_or_else(|error| panic!("{error}"));
    let fight = fighting::Rules {
        choose: choosing(),
        land: landing(None),
        tick: 9,
        blow: 2.0,
        surrender: 1.0,
    };
    Rules {
        fight,
        seconds: 0.1,
        taking: TakingTuning { hold: 5.0, reach: 1.0 },
        wounds: WoundsTuning::of(&content.tuning),
        messenger_shock: 10,
        bands: choosing().bands,
    }
}

/// The fight's squads at 20 hp and full stamina, the second's 201 at `hp`; the second
/// routing and without courage if `routing` (every one struck gives up).
fn fixture(hp: i32, routing: bool) -> Built {
    let mut b = built(1.0, FIGHTING);
    for id in [100, 101, 200, 201] {
        let slot = b.slot(id);
        b.field.columns.hp[slot] = if id == 201 { hp } else { 20 };
        b.field.columns.stamina[slot] = 100.0;
        if routing && id >= 200 {
            b.field.columns.sheet[slot].courage = 0;
        }
    }
    if routing {
        b.squads.state[1] = ROUTING;
    }
    b
}

/// What a call left: the aftermath, the log, the squads, and each unit's state and hp.
type Left = (Aftermath, Vec<i64>, Squads, Vec<(i32, i32)>);

fn left(b: &mut Built, aftermath: Aftermath, log: &mut Log) -> Left {
    let c = &b.field.columns;
    let units = (0..c.state.len()).map(|n| (c.state[n], c.hp[n])).collect();
    (aftermath, log.take(), b.squads.clone(), units)
}

/// The five passes, one call each, as FormationSimulation made them.
fn phased(b: &mut Built, rules: &Rules) -> Left {
    let mut log = Log::default();
    let (squads, field, roster) = (&mut b.squads, &mut b.field, &b.roster);
    let (tick, seed, interval) = (9, rules.fight.choose.seed, rules.fight.choose.interval);
    let fought = fighting::step(squads, field, roster, &rules.fight, None, &mut log);
    let taken = fought.taken.iter().map(|&(n, m)| (n, roster.listed[n].living[m].id)).collect();
    let listed: Vec<usize> = roster.listed.iter().map(|l| l.squad).collect();
    let buried = bury::bury(squads, field, &listed, (&rules.bands, tick), &mut log);
    let threatened: Vec<bool> = listed.iter().map(|&s| squads.threatened[s]).collect();
    let lists = (&listed[..], &threatened[..]);
    taking::step(squads, field, lists, (tick, 0.1, seed), &rules.taking, &mut log);
    let t = wounds::Tuning { wounds: &rules.wounds, shock: 10, bands: &rules.bands };
    let tended = wounds::step(squads, field, &listed, (tick, interval, seed), &t, &mut log);
    let at = (tick, 0.1, seed);
    let recovered = recovery::step(squads, field, &listed, at, &rules.wounds, &mut log);
    left(b, Aftermath { taken, buried, tended, recovered }, &mut log)
}

fn whole(b: &mut Built, rules: &Rules) -> Left {
    let mut log = Log::default();
    let aftermath = step(&mut b.squads, &mut b.field, &b.roster, None, rules, &mut log);
    left(b, aftermath, &mut log)
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

#[test]
fn the_one_call_leaves_what_the_five_passes_one_by_one_leave_when_a_unit_falls() {
    let rules = rules();
    let (phased, whole) =
        (phased(&mut fixture(5, false), &rules), whole(&mut fixture(5, false), &rules));
    let fell = kinds(&whole.1);
    assert!(fell.contains(&DIED) || fell.contains(&DOWNED), "201 fell and was buried");
    assert_eq!(whole.0.buried.len(), 1);
    assert_eq!(whole.0.buried, phased.0.buried);
    assert_eq!(whole.0.tended, phased.0.tended);
    assert_eq!(whole.0.recovered, phased.0.recovered);
    assert_eq!(whole.1, phased.1);
    assert_eq!(whole.2, phased.2);
    assert_eq!(whole.3, phased.3);
}

#[test]
fn the_units_the_surrender_took_are_named_by_their_ids() {
    let rules = rules();
    let (phased, whole) =
        (phased(&mut fixture(20, true), &rules), whole(&mut fixture(20, true), &rules));
    assert_eq!(whole.0.taken, vec![(1, 201)]);
    assert_eq!(whole.0.taken, phased.0.taken);
    assert_eq!(whole.1, phased.1);
    assert_eq!(whole.2, phased.2);
    assert_eq!(whole.3, phased.3);
}
