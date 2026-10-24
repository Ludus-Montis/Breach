//! FormationStrays' scenarios (Decision 126): a unit on its own setting off home along its
//! formation's route (east along y = 0), and strays setting off together numbered by their
//! draws. Its command and orders, the squad's own, stay gdUnit (test_formation_command).

use super::*;
use crate::contest;
use crate::entries::FLEEING;
use crate::maths::V2;

fn built(at: V2) -> (Squads, Columns) {
    let mut squads = Squads::default();
    let mut columns = Columns::default();
    squads.open(9);
    columns.grow();
    columns.position[0] = at;
    columns.rank[0] = 2;
    columns.column[0] = 3;
    (squads, columns)
}

fn route() -> Route {
    Route::new(vec![V2::ZERO, V2::new(128.0, 0.0)], 0.0)
}

#[test]
fn a_stray_flees_home_from_where_it_stands_off_its_route() {
    let (mut squads, mut columns) = built(V2::new(32.0, 1.5));

    let front = set_off(&mut squads, &mut columns, 0, (4, 0), &route());

    assert_eq!(front, 0.5); // 32 cells: half a tile
    assert_eq!(squads.fleeing[0], vec![4]);
    assert_ne!(columns.flags[0] & FLEEING, 0);
    assert_eq!(columns.along[0], 32.0);
    assert_eq!(columns.flee_offset[0], V2::new(0.0, 1.5));
    assert_eq!((columns.rank[0], columns.column[0]), (0, 0)); // alone in its own frame
}

#[test]
fn a_stray_walks_home_straight_not_fanning_out() {
    let (mut squads, mut columns) = built(V2::new(32.0, 1.5));

    set_off(&mut squads, &mut columns, 0, (4, 0), &route());

    assert_eq!(columns.fanned[0], f64::INFINITY);
    assert_ne!(columns.keys[0] & 512, 0); // EntryKeys: FANNED kept
}

#[test]
fn strays_setting_off_together_are_numbered_by_their_draws_whatever_their_order() {
    let ids = [12, 5, 40, 7];
    let order = numbered(&ids, 7);
    let mut reversed = ids;
    reversed.reverse();

    let draws: Vec<i64> = order.iter().map(|&n| contest::draw(7, ids[n])).collect();
    assert!(draws.windows(2).all(|w| w[0] <= w[1]));
    let one_way: Vec<i64> = order.iter().map(|&n| ids[n]).collect();
    let other_way: Vec<i64> = numbered(&reversed, 7).iter().map(|&n| reversed[n]).collect();
    assert_eq!(one_way, other_way);
}
