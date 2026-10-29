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

/// Squad 9 (slot 0), "kingdom", heading west down `route()` from 2 tiles home, under
/// command 3, its units `ids` standing a cell apart along y = 1.5 from x = 32.
fn adopting(ids: &[i64]) -> (Squads, Field) {
    let (mut squads, mut field) = (Squads::default(), Field::default());
    squads.open(9);
    squads.squads_issued = 20;
    squads.faction[0] = field.lexicon.code("kingdom");
    squads.direction[0] = -1;
    squads.home_distance[0] = 2.0;
    squads.heading[0] = 270.0;
    squads.width[0] = 4;
    squads.command[0] = Some(3);
    squads.route[0] = Some(route());
    for (n, &id) in ids.iter().enumerate() {
        let slot = field.hold(id) as usize;
        field.columns.position[slot] = V2::new(32.0 + n as f32, 1.5);
        field.columns.sheet[slot].squad_id = 9;
        squads.members[0].push(slot as i32);
    }
    (squads, field)
}

#[test]
fn a_stray_leaves_its_squad_as_a_routing_squad_of_its_own_set_off_home() {
    let (mut squads, mut field) = adopting(&[4, 5]);
    let slot = field.slot_of(4).unwrap() as usize;
    squads.loose[0].push(4);

    let opened = adopt(&mut squads, &mut field, &[(0, slot)], 7);

    assert_eq!(opened, vec![(0, 1)]);
    assert_eq!(squads.members[0], vec![field.slot_of(5).unwrap() as i32]);
    assert!(squads.loose[0].is_empty());
    assert_eq!(squads.members[1], vec![slot as i32]);
    assert_eq!(squads.id[1], 21); // the battle's next id
    assert_eq!(field.columns.sheet[slot].squad_id, 21);
    assert_eq!(field.columns.objects[slot].command, 3);
    assert_eq!(squads.faction[1], squads.faction[0]);
    assert_eq!(
        (squads.direction[1], squads.home_distance[1], squads.heading[1], squads.width[1]),
        (-1, 2.0, 270.0, 1)
    );
    assert_eq!((squads.state[1], squads.morale[1]), (ROUTING, 0));
    assert_eq!(squads.command[1], Some(3));
    assert_eq!(squads.route[1], Some(route()));
    assert_eq!(squads.front_distance[1], 0.5); // where it stands: 32 cells along
    assert_eq!(squads.position[1], V2::new(32.0, 0.0));
    assert_eq!(squads.fleeing[1], vec![4]);
}

#[test]
fn strays_adopted_together_take_the_battles_ids_in_their_draws_order() {
    let ids = [12, 5, 40, 7];
    let (mut squads, mut field) = adopting(&ids);
    let strays: Vec<(usize, usize)> =
        ids.iter().map(|&id| (0, field.slot_of(id).unwrap() as usize)).collect();

    let opened = adopt(&mut squads, &mut field, &strays, 7);

    let order: Vec<usize> = opened.iter().map(|&(n, _)| n).collect();
    assert_eq!(order, numbered(&ids, 7));
    let numbered_ids: Vec<i64> = opened.iter().map(|&(_, s)| squads.id[s]).collect();
    assert_eq!(numbered_ids, vec![21, 22, 23, 24]);
    assert_eq!(squads.squads_issued, 24);
    assert!(squads.members[0].is_empty());
}
