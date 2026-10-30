//! The tick's rules read in the crate: each pass's arguments as GDScript's `args` built them
//! (test_formation_tick.gd holds them equal to GDScript's, value for value).

use super::*;
use crate::content::tests::repo_content;
use crate::content::Content;

fn tuning() -> BattleTuning {
    Content::load(&repo_content()).unwrap_or_else(|error| panic!("{error}")).tuning
}

fn given(tick: i64) -> Given {
    let at = At { tick, seed: 7, ticks_a_second: 10, rolls: true, threaded: false };
    let paces = Paces { travel_scale: 0.5, seconds: 0.1, route_length: 12.0, cells_a_second: 32.0 };
    of(&tuning(), &at, &paces)
}

#[test]
fn the_scrum_stalls_after_its_seconds_in_whole_ticks() {
    let t = tuning();
    let stall = (t.scrum_stall_seconds / 0.1).round() as i64;
    let ints = Arg::Ints(vec![30, 7, t.combat_contest_die, stall]);
    assert_eq!(given(30).scrum.first(), Some(&ints));
    assert_eq!(given(30).scrum.get(1), Some(&Arg::Real(t.reach_engage)));
}

#[test]
fn morale_steps_only_on_a_seconds_tick() {
    assert_eq!(given(30).mustering.first(), Some(&Arg::Ints(vec![30, 7, 1])));
    assert_eq!(given(31).mustering.first(), Some(&Arg::Ints(vec![31, 7, 0])));
}

#[test]
fn the_pursuit_hands_over_its_leash_after_its_paces() {
    let t = tuning();
    let Some(Arg::Floats(pursue)) = given(1).scrum.get(6).cloned() else {
        panic!("no pursuit's rules")
    };
    let shares = t.discipline_leash_shares.len();
    let mut expected = vec![32.0, 0.1, t.stamina_give_up, t.reach_pursuit_lag];
    expected.extend([t.discipline_per_leadership as f64, t.discipline_expected_max]);
    expected.push(shares as f64);
    expected.extend(&t.discipline_leash_shares);
    expected.extend(&t.discipline_leash_steps);
    assert_eq!(pursue, expected);
}

#[test]
fn each_pass_gets_as_many_arguments_as_its_binding_reads() {
    let given = given(1);
    let counts = [&given.opening, &given.scrum, &given.aftermath, &given.mustering, &given.ending]
        .map(|args| args.len());
    assert_eq!(counts, [6, 7, 5, 3, 5]);
}
