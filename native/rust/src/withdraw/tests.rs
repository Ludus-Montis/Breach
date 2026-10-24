//! FormationWithdraw's scenarios (Decision 99), on squads built here: a player squad
//! withdrawing west along a straight route on y = 0.5, home at x = 0, and kingdom grems.

use super::*;
use crate::ground::Walker;
use crate::roster::Listed;

const GREM: Walker = Walker { height: 1.0, swimmer: 0, climber: 0, swims: false, climbs: false };

fn rules() -> Rules {
    Rules { pace: 0.25, seconds: 0.1, reach: 0.2, near: 4.0, fan_cells: 3.0 }
}

fn grem(id: i64, slot: usize) -> Member {
    Member {
        id,
        slot,
        foot: (1, 1),
        preferred: 0,
        kind: 0,
        turn_rate: 3600.0,
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

/// The player squad (slot 0, id 1) of grems loose at `ours`, facing west, and the kingdom
/// squad (slot 1, id 2) of grems at `foes`.
fn built(ours: &[V2], foes: &[V2]) -> Built {
    let mut squads = Squads::default();
    let mut columns = Columns::default();
    let mut roster = Roster::default();
    for (slot, (id, faction)) in [(1i64, 0u32), (2, 1)].iter().enumerate() {
        squads.open(*id);
        roster.listed.push(Listed { squad: slot, id: *id, faction: *faction, living: vec![] });
    }
    for (n, &at) in ours.iter().enumerate() {
        columns.grow();
        columns.speed[n] = 1.0;
        columns.bearing[n] = 270.0;
        let unit = grem(n as i64 + 1, n);
        crate::entries::loosen(&mut squads, &mut columns, 0, (unit.id, n), at);
        roster.listed[0].living.push(unit);
    }
    for (k, &at) in foes.iter().enumerate() {
        let slot = columns.position.len();
        columns.grow();
        columns.position[slot] = at;
        roster.listed[1].living.push(grem(101 + k as i64, slot));
    }
    Built { squads, columns, roster }
}

fn road() -> Route {
    Route::new(vec![V2::new(0.0, 0.5), V2::new(40.0, 0.5)], 0.0)
}

fn withdrawing(b: &Built, disorder: f64, fan: f64) -> Withdrawing {
    Withdrawing {
        listed: 0,
        pursued: false,
        disorder,
        reform: 0.5,
        home: 0.0,
        route: road(),
        fans: vec![fan; b.roster.listed[0].living.len()],
    }
}

fn run(b: &mut Built, w: &[Withdrawing]) -> Vec<(bool, bool)> {
    step(&b.squads, &mut b.columns, &b.roster, w, None, &rules())
}

#[test]
fn a_clear_unit_flees_home_along_its_route_at_its_full_pace() {
    let mut b = built(&[V2::new(20.0, 0.5)], &[]);
    let w = withdrawing(&b, 0.0, 0.0);
    run(&mut b, &[w]);
    let at = b.columns.loose_at[0];
    assert!((at.x - 19.75).abs() < 0.0001 && (at.y - 0.5).abs() < 0.0001, "{at:?}");
    assert_eq!(b.columns.next[0], at);
}

#[test]
fn a_unit_still_touching_a_foe_steps_away_at_its_reform_pace() {
    let mut b = built(&[V2::new(20.0, 0.5)], &[V2::new(21.0, 0.5)]);
    let w = withdrawing(&b, 0.0, 0.0);
    run(&mut b, &[w]);
    assert!((b.columns.loose_at[0].x - 19.875).abs() < 0.0001);
}

#[test]
fn a_ragged_squads_units_fan_out_from_the_route_and_a_drilled_ones_do_not() {
    let mut ragged = built(&[V2::new(20.0, 0.5)], &[]);
    let w = withdrawing(&ragged, 1.0, 0.5);
    run(&mut ragged, &[w]);
    assert!(ragged.columns.loose_at[0].y != 0.5);
    let mut drilled = built(&[V2::new(20.0, 0.5)], &[]);
    let w = withdrawing(&drilled, 0.0, 0.5);
    run(&mut drilled, &[w]);
    assert!((drilled.columns.loose_at[0].y - 0.5).abs() < 0.0001);
}

#[test]
fn a_unit_fanned_out_as_far_as_its_disorder_takes_it_runs_straight_home() {
    let fan = 0.5;
    let outward = rotated(V2::new(-1.0, 0.0), fan).y.signum(); // the side its fan takes it
    let y = 0.5 + 4.0 * outward; // 4 cells out that side: past rout_fan_cells
    let mut b = built(&[V2::new(20.0, y)], &[]);
    let w = withdrawing(&b, 1.0, fan);
    run(&mut b, &[w]);
    assert!((b.columns.loose_at[0].y - y).abs() < 0.0001);
    assert!(b.columns.loose_at[0].x < 20.0);
}

#[test]
fn a_unit_home_stays_where_it_is() {
    let mut b = built(&[V2::new(0.3, 0.5)], &[]);
    let w = withdrawing(&b, 0.0, 0.0);
    let out = run(&mut b, &[w]);
    assert_eq!(b.columns.loose_at[0], V2::new(0.3, 0.5));
    assert_eq!(out, vec![(true, true)]);
}

#[test]
fn a_squad_is_safe_with_no_enemy_near_and_none_pursuing() {
    let mut far = built(&[V2::new(20.0, 0.5)], &[V2::new(30.0, 0.5)]);
    let w = withdrawing(&far, 0.0, 0.0);
    assert_eq!(run(&mut far, &[w]), vec![(true, false)]);
    let mut near = built(&[V2::new(20.0, 0.5)], &[V2::new(23.0, 0.5)]);
    let w = withdrawing(&near, 0.0, 0.0);
    assert_eq!(run(&mut near, &[w]), vec![(false, false)]);
    let mut chased = built(&[V2::new(20.0, 0.5)], &[V2::new(30.0, 0.5)]);
    let w = Withdrawing { pursued: true, ..withdrawing(&chased, 0.0, 0.0) };
    assert_eq!(run(&mut chased, &[w]), vec![(false, false)]);
}

#[test]
fn withdrawals_are_the_same_whatever_the_lists_order() {
    let ours = [V2::new(20.0, 0.5), V2::new(20.0, 1.5), V2::new(21.0, 0.5)];
    let foes = [V2::new(21.0, 1.5), V2::new(22.0, 0.5)];
    let mut b = built(&ours, &foes);
    let w = Withdrawing { fans: vec![0.1, -0.2, 0.3], ..withdrawing(&b, 0.5, 0.0) };
    let out = run(&mut b, &[w]);
    let mut r = built(&ours, &foes);
    r.roster.listed.reverse();
    for listed in &mut r.roster.listed {
        listed.living.reverse();
    }
    let w = Withdrawing { listed: 1, fans: vec![0.3, -0.2, 0.1], ..withdrawing(&r, 0.5, 0.0) };
    assert_eq!(run(&mut r, &[w]), out);
    assert_eq!(b.columns.loose_at, r.columns.loose_at);
}
