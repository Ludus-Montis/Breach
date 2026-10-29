//! FormationMorale's ceiling, bands and shocks (Decisions 81 and 82), from
//! test_formation_morale.gd. The tuning is the repo's.

use super::*;
use crate::content::tests::repo_content;
use crate::content::Content;

fn tuning() -> BandTuning {
    let content = Content::load(&repo_content()).unwrap_or_else(|error| panic!("{error}"));
    let t = &content.tuning;
    BandTuning::of_slice(&[
        t.morale_per_leadership,
        t.morale_steady_at,
        t.morale_shaken_at,
        t.morale_loss,
        t.morale_leader_loss,
    ])
}

#[test]
fn the_ceiling_is_the_mean_courage_rounded_plus_the_best_leaders_steps() {
    let t = tuning();
    assert_eq!(ceiling(&[60, 61], &[0, 0], &t), 61); // 60.5 rounds away from zero
    assert_eq!(ceiling(&[50, 50, 50], &[0, 2, 1], &t), 50 + 2 * t.per_leadership);
}

#[test]
fn the_ceiling_is_at_most_100_and_0_with_none_living() {
    let t = tuning();
    assert_eq!(ceiling(&[100, 100], &[0, 50], &t), 100);
    assert_eq!(ceiling(&[], &[], &t), 0);
}

#[test]
fn the_best_living_leader_leads() {
    assert_eq!(leadership(&[0, 3, 1]), 3);
    assert_eq!(leadership(&[]), 0);
}

#[test]
fn the_bands_fall_by_morale_and_routing_only_at_0() {
    let t = tuning();
    assert_eq!(band(t.steady_at, &t), STEADY);
    assert_eq!(band(t.steady_at - 1, &t), SHAKEN);
    assert_eq!(band(t.shaken_at, &t), SHAKEN);
    assert_eq!(band(t.shaken_at - 1, &t), WAVERING);
    assert_eq!(band(1, &t), WAVERING);
    assert_eq!(band(0, &t), ROUTING);
}

#[test]
fn a_wavering_formation_strikes_half_again_more_slowly() {
    assert_eq!(interval(WAVERING, 3), 5); // 4.5 rounds away from zero
    assert_eq!(interval(WAVERING, 4), 6);
    for other in [STEADY, SHAKEN, ROUTING] {
        assert_eq!(interval(other, 3), 3);
    }
}

#[test]
fn morale_is_held_to_0_to_100() {
    assert_eq!((held(-5), held(0), held(42), held(100), held(130)), (0, 0, 42, 100, 100));
}

#[test]
fn the_dead_shock_by_each_loss_and_each_leaders_leadership() {
    let t = tuning();
    assert_eq!(losses(&[0, 0], &t), 2 * t.loss);
    assert_eq!(losses(&[0, 3], &t), 2 * t.loss + 3 * t.leader_loss);
    assert_eq!(losses(&[], &t), 0);
}
