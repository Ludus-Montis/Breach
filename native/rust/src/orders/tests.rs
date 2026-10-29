//! The orders' scenarios (Decisions 95, 99, 109 and 113), on squads built here: a player
//! squad (id 1) fighting a kingdom squad (id 2) front to front on a straight road along
//! y = 0, and a second kingdom squad (id 3) beside them, each of two grems.

use super::*;
use crate::bands::BandTuning;
use crate::discipline::DisciplineTuning;
use crate::events::{self, Log};
use crate::ground::Walker;
use crate::roster::{Listed, Member};
use crate::squads::Stance;

const GREM: Walker = Walker { height: 1.0, swimmer: 0, climber: 0, swims: false, climbs: false };
const ADVANCE: i32 = 0;
const HOLD: i32 = 1;
const RETREAT: i32 = 2;

fn grem(id: i64, slot: usize) -> Member {
    Member {
        id,
        slot,
        foot: (1, 1),
        preferred: 0,
        attack_range: 0,
        kind: 0,
        turn_rate: 90.0,
        backward_pace: 0.5,
        detection: 8.0,
        max_stamina: 100.0,
        walker: GREM,
    }
}

struct Built {
    squads: Squads,
    columns: Columns,
    roster: Roster,
}

/// Squads 1 (player, front at x = 10), 2 and 3 (kingdom, fronts at x = 11 and x = 11 y = 6),
/// each of two grems standing a cell apart behind its front; 1 and 2 locked front to front.
fn built() -> Built {
    let mut squads = Squads::default();
    let mut columns = Columns::default();
    let mut roster = Roster::default();
    let fronts = [V2::new(10.0, 0.0), V2::new(11.0, 0.0), V2::new(11.0, 6.0)];
    for (slot, (id, faction)) in [(1i64, 0u32), (2, 1), (3, 1)].into_iter().enumerate() {
        squads.open(id);
        squads.position[slot] = fronts[slot];
        squads.front_distance[slot] = fronts[slot].x as f64 / CELLS;
        squads.direction[slot] = if faction == 0 { 1 } else { -1 };
        let mut living = Vec::new();
        for k in 0..2 {
            let unit = columns.position.len();
            columns.grow();
            let behind = if faction == 0 { -(k as f32) } else { k as f32 };
            columns.position[unit] = fronts[slot].add(V2::new(behind, 0.0));
            living.push(grem(id * 10 + k as i64, unit));
        }
        roster.listed.push(Listed { squad: slot, id, faction, living, ..Listed::default() });
    }
    for (s, foe) in [(0, 2), (1, 1)] {
        squads.engaged_with[s] = foe;
        squads.state[s] = locks::FIGHTING;
    }
    Built { squads, columns, roster }
}

fn road() -> Route {
    Route::new(vec![V2::new(0.0, 0.0), V2::new(64.0, 0.0)], 0.0)
}

/// What a retreat reads: every squad on the road (one route), all may pursue, every unit
/// with `chance` to break ranks on a leash of 6 cells.
fn retreats(b: &Built, chance: f64) -> Retreats<'static> {
    let units: usize = b.roster.listed.iter().map(|l| l.living.len()).sum();
    let temptations: &'static [(f64, f64)] = vec![(chance, 6.0); units].leak();
    Retreats {
        routes: vec![(0, Some(road())); b.roster.listed.len()],
        pursues: vec![true; b.roster.listed.len()].leak(),
        temptations,
        within: 4.0,
        tick: 5,
        seed: 3,
    }
}

fn run(b: &mut Built, given: &[(i64, i32)], retreats: &mut Retreats) -> Vec<Applied> {
    apply(&mut b.squads, &mut b.columns, &b.roster, given, retreats)
}

#[test]
fn an_order_applies_to_its_squad_and_one_to_a_squad_gone_or_destroyed_to_none() {
    let mut b = built();
    b.squads.state[2] = locks::DESTROYED;
    let mut r = retreats(&b, 0.0);
    let applied = run(&mut b, &[(9, HOLD), (3, HOLD), (2, HOLD)], &mut r);
    assert_eq!(applied, vec![Applied { listed: 1, retreat: None }]);
    assert_eq!(b.squads.order[1], HOLD);
    assert_eq!(b.squads.order[2], ADVANCE);
}

#[test]
fn a_retreat_out_of_no_fight_only_sets_the_order() {
    let mut b = built();
    let mut r = retreats(&b, 1.0);
    let applied = run(&mut b, &[(3, RETREAT)], &mut r);
    assert_eq!(applied, vec![Applied { listed: 2, retreat: None }]);
    assert_eq!(b.squads.order[2], RETREAT);
    assert!(b.squads.withdraw[2].is_none() && b.squads.loose[2].is_empty());
}

#[test]
fn a_retreat_out_of_a_fight_ends_it_and_withdraws_from_where_its_units_stand() {
    let mut b = built();
    b.squads.stance[0] = Some(Stance { anchor: V2::ZERO, heading: 90.0, foe: 2 });
    let mut r = retreats(&b, 0.0);
    r.pursues = &[false, false, false];
    let applied = run(&mut b, &[(1, RETREAT)], &mut r);
    assert_eq!(applied, vec![Applied { listed: 0, retreat: Some(vec![]) }]);
    assert_eq!((b.squads.engaged_with[0], b.squads.engaged_with[1]), (0, 0));
    assert_eq!(b.squads.withdraw[0], Some(0));
    assert!(b.squads.stance[0].is_none());
    assert_eq!(b.squads.loose[0], vec![10, 11]);
    assert_eq!(b.columns.loose_at[1], V2::new(9.0, 0.0));
}

#[test]
fn an_enemy_that_may_pursue_locks_on_and_keeps_its_post_and_how_it_held_it() {
    let mut b = built();
    b.squads.order[1] = HOLD;
    let mut r = retreats(&b, 0.0);
    let applied = run(&mut b, &[(1, RETREAT)], &mut r);
    let pursuer = Pursuer { listed: 1, began: true, took: false };
    assert_eq!(applied[0].retreat, Some(vec![pursuer]));
    assert_eq!((b.squads.engaged_with[1], b.squads.state[1]), (1, locks::FIGHTING));
    let held = b.squads.pursuit[1].expect("a pursuit");
    assert_eq!((held.foe, held.post, held.post_at), (1, 11.0 / CELLS, V2::new(11.0, 0.0)));
    assert_eq!((held.direction, held.order, held.returning), (-1, HOLD, false));
    assert!(b.squads.pursuit[2].is_none(), "squad 3 never fought it");
}

#[test]
fn an_enemy_ordered_not_to_pursue_does_not_and_one_pursuing_already_keeps_its_post() {
    let mut b = built();
    let mut r = retreats(&b, 0.0);
    r.pursues = &[true, false, true];
    run(&mut b, &[(1, RETREAT)], &mut r);
    assert!(b.squads.pursuit[1].is_none());
    let mut b = built();
    let kept = Pursuit {
        foe: 9,
        post: 0.5,
        post_at: V2::new(32.0, 0.0),
        home: 1.0,
        direction: -1,
        order: HOLD,
        returning: false,
    };
    b.squads.pursuit[1] = Some(kept);
    let mut r = retreats(&b, 0.0);
    let applied = run(&mut b, &[(1, RETREAT)], &mut r);
    let pursuer = Pursuer { listed: 1, began: false, took: false };
    assert_eq!(applied[0].retreat, Some(vec![pursuer]));
    assert_eq!(b.squads.pursuit[1], Some(kept));
}

#[test]
fn a_pursuer_takes_its_quarrys_route_where_it_passes_by_towards_its_quarry() {
    let mut b = built();
    let mut r = retreats(&b, 0.0);
    let theirs = Route::new(vec![V2::new(20.0, 1.0), V2::new(0.0, 1.0)], 0.0);
    r.routes[0] = (7, Some(theirs.clone()));
    let applied = run(&mut b, &[(1, RETREAT)], &mut r);
    assert_eq!(applied[0].retreat, Some(vec![Pursuer { listed: 1, began: true, took: true }]));
    assert_eq!(b.squads.front_distance[1], 9.0 / CELLS);
    assert_eq!(b.squads.position[1], V2::new(11.0, 1.0));
    assert_eq!(b.squads.direction[1], 1, "towards x = 10: further along its quarry's route");
    assert_eq!(r.routes[1], (7, Some(theirs)), "its route from now on");
}

#[test]
fn a_pursuer_keeps_its_own_route_when_its_quarrys_passes_far_from_it_or_is_its_own() {
    for (code, y) in [(7, 5.0), (0, 1.0)] {
        let mut b = built();
        let mut r = retreats(&b, 0.0);
        r.routes[0] = (code, Some(Route::new(vec![V2::new(20.0, y), V2::new(0.0, y)], 0.0)));
        let applied = run(&mut b, &[(1, RETREAT)], &mut r);
        let pursuer = Pursuer { listed: 1, began: true, took: false };
        assert_eq!(applied[0].retreat, Some(vec![pursuer]), "code {code}");
        assert_eq!((b.squads.position[1], b.squads.direction[1]), (V2::new(11.0, 0.0), -1));
    }
}

#[test]
fn a_foe_on_an_edge_pursues_but_a_friend_does_not() {
    let mut b = built();
    b.squads.flank(2, 1, 1, 0);
    b.squads.engaged_with[2] = 0;
    let mut r = retreats(&b, 0.0);
    let applied = run(&mut b, &[(1, RETREAT)], &mut r);
    let pursuers: Vec<usize> =
        applied[0].retreat.as_ref().unwrap().iter().map(|p| p.listed).collect();
    assert_eq!(pursuers, vec![1, 2]);
    assert!(b.squads.flanks[2].is_empty(), "its locks ended");
    let mut b = built();
    b.roster.listed[2].faction = 0;
    b.squads.flank(2, 1, 1, 0);
    let mut r = retreats(&b, 0.0);
    let applied = run(&mut b, &[(1, RETREAT)], &mut r);
    assert_eq!(applied[0].retreat.as_ref().unwrap().len(), 1);
}

#[test]
fn its_enemies_units_near_it_may_break_ranks_to_chase_it() {
    for (chance, chasers) in [(1.0, 2), (0.0, 0)] {
        let mut b = built();
        let mut r = retreats(&b, chance);
        r.pursues = &[false, false, false];
        run(&mut b, &[(1, RETREAT)], &mut r);
        assert_eq!(b.squads.chasers[1].len(), chasers, "chance {chance}");
        assert!(b.squads.chasers[2].is_empty(), "squad 3 never fought it");
    }
}

#[test]
fn orders_given_together_act_together_whoever_is_listed_first() {
    for order in [[0, 1, 2], [2, 1, 0]] {
        let mut b = built();
        let listed: Vec<Listed> = order.iter().map(|&n| b.roster.listed[n].clone()).collect();
        b.roster.listed = listed;
        let mut r = retreats(&b, 0.0);
        r.pursues = &[false, false, false];
        run(&mut b, &[(2, RETREAT), (1, RETREAT)], &mut r);
        assert_eq!((b.squads.withdraw[0], b.squads.withdraw[1]), (Some(0), Some(0)));
        assert!(b.squads.withdraw[2].is_none());
    }
}

#[test]
fn orders_queue_in_the_order_given_and_a_second_replaces_the_first_in_place() {
    let mut squads = Squads::default();
    queue(&mut squads, 3, ADVANCE);
    queue(&mut squads, 1, HOLD);
    queue(&mut squads, 3, RETREAT);
    assert_eq!(squads.pending, vec![(3, RETREAT), (1, HOLD)]);
}

/// The pass's tunings: drilled at 50, a ragged retreat shocks by up to 20, steady from 70.
fn pass_given(b: &Built) -> (DisciplineTuning, BandTuning, Vec<Coded>) {
    let discipline = DisciplineTuning {
        per_leadership: 5,
        meets_threats: 50,
        expected_max: 100.0,
        leash_shares: vec![0.8, 0.5],
        leash_steps: vec![2.0, 4.0, 8.0, 16.0],
    };
    let bands =
        BandTuning { per_leadership: 5, steady_at: 70, shaken_at: 40, loss: 2, leader_loss: 3 };
    (discipline, bands, vec![(0, Some(road())); b.roster.listed.len()])
}

fn run_pass(b: &mut Built, given: &[(i64, i32)], log: &mut Log) -> Vec<Applied> {
    let (discipline, bands, routes) = pass_given(b);
    b.squads.pending = given.to_vec();
    let given = pass::Given {
        routes,
        within: 4.0,
        ragged_shock: 20,
        discipline: &discipline,
        bands: &bands,
        tick: 5,
        seed: 3,
    };
    pass::pass(&mut b.squads, &mut b.columns, &b.roster, given, log)
}

#[test]
fn the_pass_takes_the_queue_and_each_order_applied_is_an_event() {
    let mut b = built();
    b.squads.faction[2] = 6;
    let mut log = Log::default();
    run_pass(&mut b, &[(3, HOLD)], &mut log);
    assert!(b.squads.pending.is_empty());
    assert_eq!(log.take(), vec![events::ORDER_APPLIED as i64, 5, 3, 6, i64::from(HOLD)]);
}

#[test]
fn a_ragged_retreat_out_of_a_fight_is_shocked_and_its_pursuers_reported_in_order() {
    let mut b = built();
    b.squads.morale[0] = 75; // steady; its units' discipline 30 of 50: 0.4 short, a shock of 8
    let command = b.squads.commands.open(0.0);
    b.squads.command[1] = Some(command);
    let mut log = Log::default();
    let applied = run_pass(&mut b, &[(1, RETREAT)], &mut log);
    let pursuer = Pursuer { listed: 1, began: true, took: false };
    assert_eq!(applied[0].retreat, Some(vec![pursuer]), "squad 2 pursues, by its command");
    assert_eq!(b.squads.morale[0], 67);
    let kinds = [events::ORDER_APPLIED, events::DISENGAGED, events::MORALE_BAND];
    let mut expected: Vec<i64> = Vec::new();
    for (kind, values) in kinds.iter().zip([vec![i64::from(RETREAT)], vec![], vec![1, 67]]) {
        expected.extend([*kind as i64, 5, 1, 0]);
        expected.extend(values);
    }
    expected.extend([events::WITHDRAWING as i64, 5, 1, 0]);
    expected.extend([events::PURSUING as i64, 5, 2, 0, 1]);
    assert_eq!(log.take(), expected);
}

#[test]
fn a_retreats_enemies_break_ranks_by_their_own_discipline_on_their_sheets() {
    for (discipline, chasers) in [(-100, 2), (100, 0)] {
        let mut b = built();
        for m in &b.roster.listed[1].living {
            b.columns.sheet[m.slot].discipline = discipline;
        }
        b.squads.morale[0] = 75;
        let mut log = Log::default();
        run_pass(&mut b, &[(1, RETREAT)], &mut log);
        assert!(b.squads.pursuit[1].is_none(), "no command: it may not pursue");
        assert_eq!(b.squads.chasers[1].len(), chasers, "discipline {discipline}");
    }
}
