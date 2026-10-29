//! Landing the hurts (spec 39 step 8b2), on a store of a few units built here.

use super::*;

/// A store of units at these hp, slot by slot.
fn store(hp: &[i32]) -> Columns {
    let mut columns = Columns::default();
    for _ in hp {
        columns.grow();
    }
    columns.hp.copy_from_slice(hp);
    columns
}

fn hurt(slot: usize, damage: i64) -> Hurt {
    Hurt { slot, id: 100 + slot as i64, damage }
}

#[test]
fn each_hurt_takes_its_damage_and_answers_the_hp_left() {
    let mut columns = store(&[10, 8]);
    let mut fell = vec![];
    let left = land(&mut columns, &[hurt(1, 3), hurt(0, 4)], &mut fell);
    assert_eq!(left, vec![5, 6]);
    assert_eq!(columns.hp, vec![6, 5]);
    assert!(fell.is_empty());
}

#[test]
fn hurts_on_one_target_land_in_order_each_answer_after_the_last() {
    let mut columns = store(&[10]);
    let mut fell = vec![];
    let left = land(&mut columns, &[hurt(0, 3), hurt(0, 0), hurt(0, 5)], &mut fell);
    assert_eq!(left, vec![7, 7, 2]);
}

#[test]
fn a_unit_taken_to_0_or_below_falls_once_in_the_order_it_fell() {
    let mut columns = store(&[5, 3, 4]);
    let mut fell = vec![];
    let hurts = [hurt(1, 3), hurt(0, 9), hurt(1, 2), hurt(2, 3)];
    let left = land(&mut columns, &hurts, &mut fell);
    assert_eq!(left, vec![0, -4, -2, 1]);
    assert_eq!(fell, vec![101, 100]); // the second blow on the fallen doesn't fell it again
}

#[test]
fn one_already_down_doesnt_fall_again_and_nothing_dealt_fells_nobody() {
    let mut columns = store(&[0, 2]);
    let mut fell = vec![];
    land(&mut columns, &[hurt(0, 4), hurt(1, 0)], &mut fell);
    assert!(fell.is_empty());
    assert_eq!(columns.hp, vec![-4, 2]);
}
