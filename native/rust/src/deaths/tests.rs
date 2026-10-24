//! FormationDeaths' scenarios (Decision 121), from test_formation_wounds.gd and
//! test_formation_recovery.gd: each unit in the next store slot.

use super::*;

/// A store of units with these hp, the wounds each's last blow would add 0.
fn store(hps: &[i32]) -> Columns {
    let mut columns = Columns::default();
    for (slot, &hp) in hps.iter().enumerate() {
        columns.grow();
        columns.hp[slot] = hp;
    }
    columns
}

#[test]
fn brought_to_zero_it_is_downed_and_past_its_constitution_it_dies() {
    let mut columns = store(&[-5, -6]); // within its constitution of 6, and past it
    let fates = fate(&mut columns, &[(0, 6.0), (1, 6.0)]);
    assert_eq!(fates, vec![Some(1), None]);
    assert_eq!(columns.hp[0], 0); // the excess is lost
    assert_eq!(columns.wake_left[0], -1.0); // not yet reckoned
}

#[test]
fn a_wounding_blow_that_downs_adds_its_wounds() {
    let mut columns = store(&[-2]);
    columns.last_wounding[0] = 2;
    columns.wake_left[0] = 4.0;
    assert_eq!(fate(&mut columns, &[(0, 10.0)]), vec![Some(3)]);
    assert_eq!(columns.wake_left[0], -1.0);
}
