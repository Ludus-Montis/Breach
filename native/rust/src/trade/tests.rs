//! ScrumTrade's scenarios (Decision 110), on a squad built here: a file of like units facing
//! east from the origin, one cell a rank.

use super::*;

const AFTER: f64 = 2.0 - 0.000001;

/// A squad (slot 0) whose units, of `kinds`, stand in one file at their places; `kinds[n]`
/// is unit n+1's, in store slot n, at rank n.
fn file(kinds: &[u32]) -> (Squads, Columns, Vec<Trader>) {
    let mut squads = Squads::default();
    squads.open(1);
    squads.width[0] = 1;
    let mut columns = Columns::default();
    let mut units = Vec::new();
    for (n, &kind) in kinds.iter().enumerate() {
        columns.grow();
        columns.rank[n] = n as i32;
        let unit =
            Trader { id: n as i64 + 1, slot: n, kind, preferred: 0, foot: (1, 1), living: true };
        columns.position[n] = Frame::kept(&squads, 0).place(unit.place(&columns), unit.foot);
        units.push(unit);
    }
    (squads, columns, units)
}

/// Unit `n` stands loose `at`, stuck `stuck` seconds.
fn hold(squads: &mut Squads, columns: &mut Columns, unit: &Trader, at: V2, stuck: f64) {
    entries::loosen(squads, columns, 0, (unit.id, unit.slot), at);
    entries::keep(columns, unit.slot, Key::Stuck, Value::Real(stuck));
}

fn trading(units: &[Trader]) -> Trading<'_> {
    Trading { squad: 0, units, seed: 7, after: AFTER }
}

/// The rear unit of a file of two held a cell in front of the front one.
fn held_in_front(kinds: [u32; 2], stuck: f64) -> (Squads, Columns, Vec<Trader>) {
    let (mut squads, mut columns, units) = file(&kinds);
    let front = columns.position[0];
    let ahead = front.add(front.sub(columns.position[1]));
    hold(&mut squads, &mut columns, &units[1], ahead, stuck);
    (squads, columns, units)
}

#[test]
fn a_unit_held_off_by_its_own_ranks_trades_places_with_a_like_friend() {
    let (mut squads, mut columns, units) = held_in_front([1, 1], 2.0);
    trade(&mut squads, &mut columns, &trading(&units));
    assert_eq!((columns.rank[1], columns.rank[0]), (0, 1));
    assert_eq!(squads.loose[0], vec![2, 1]); // its friend walks to the place it left
    assert_eq!((columns.keys[1], columns.closest[1]), (0, f64::INFINITY));
}

#[test]
fn a_unit_with_no_like_friend_takes_its_own_place() {
    let (mut squads, mut columns, units) = held_in_front([1, 2], 2.0);
    trade(&mut squads, &mut columns, &trading(&units));
    assert_eq!(columns.rank[1], 1);
    assert!(squads.loose[0].is_empty());
    assert_eq!(columns.position[1], Frame::kept(&squads, 0).place((1, 0), (1, 1)));
}

#[test]
fn a_unit_not_yet_held_long_enough_does_not_trade() {
    let (mut squads, mut columns, units) = held_in_front([1, 1], 1.0);
    trade(&mut squads, &mut columns, &trading(&units));
    assert_eq!(columns.rank[1], 1);
    assert_eq!(squads.loose[0], vec![2]);
}

#[test]
fn a_chaser_holds_its_place_and_a_line_holding_a_stance_keeps_its_units_loose() {
    let (mut squads, mut columns, units) = held_in_front([1, 1], 2.0);
    squads.chase(
        0,
        crate::squads::Chaser { unit: 1, foe: 9, from: V2::ZERO, leash: 4.0, returning: false },
    );
    squads.stance[0] = Some(crate::squads::Stance { anchor: V2::ZERO, heading: 90.0, foe: 9 });
    trade(&mut squads, &mut columns, &trading(&units));
    assert_eq!(columns.rank[1], 1);
    assert_eq!(squads.loose[0], vec![2]);
}

#[test]
fn trades_in_turn_find_the_places_earlier_trades_left_whatever_the_list_order() {
    // a file of four: the rear two held in front of the line, both stalled
    let run = |reversed: bool| {
        let (mut squads, mut columns, mut units) = file(&[1, 1, 1, 1]);
        let front = columns.position[0];
        let step = front.sub(columns.position[1]);
        hold(&mut squads, &mut columns, &units[2], front.add(step), 2.0);
        hold(&mut squads, &mut columns, &units[3], front.add(step.scale(2.0)), 2.0);
        if reversed {
            units.reverse();
            squads.loose[0].reverse();
        }
        trade(&mut squads, &mut columns, &trading(&units));
        let places: Vec<i32> = (0..4).map(|s| columns.rank[s]).collect();
        let mut loose = squads.loose[0].clone();
        loose.sort();
        (places, loose)
    };
    let (places, loose) = run(false);
    let mut ranks = places.clone();
    ranks.sort();
    assert_eq!(ranks, vec![0, 1, 2, 3]); // places stay one unit each
    assert!(places[2] == 0 || places[3] == 0); // whichever trades last takes the front
    assert_eq!(run(true), (places, loose));
}
