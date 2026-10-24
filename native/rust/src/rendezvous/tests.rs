//! FormationRendezvous' scenarios (Decision 87, spec 27 round 2): the predicted march at 8
//! cells a second, a tenth of a second a tick (0.8 cells a tick). The scenarios through a
//! battle (the prediction matching the march, waves arriving together) stay gdUnit.

use super::*;
use crate::content::tests::repo_content;
use crate::content::Content;
use crate::ground::{GroundTuning, Paint};

fn march(cells: f64, width: i32) -> March {
    March { cells, width, cells_per_second: 8.0, tick_seconds: 0.1, from: 0.0, slack: 2.0 }
}

fn straight() -> Route {
    Route::new(vec![V2::ZERO, V2::new(200.0, 0.0)], 0.0)
}

fn bend() -> Route {
    Route::new(vec![V2::ZERO, V2::new(40.0, 0.0), V2::new(40.0, 160.0)], 0.0)
}

#[test]
fn a_straight_march_takes_its_length_at_its_pace() {
    assert_eq!(ticks_to(&straight(), &march(80.0, 4), None), 100);
}

#[test]
fn a_march_set_out_part_way_takes_only_the_rest() {
    let from_half_way = March { from: 40.0, ..march(80.0, 4) };

    assert_eq!(ticks_to(&straight(), &from_half_way, None), 50);
}

#[test]
fn a_bend_holds_a_wide_line_while_it_wheels_and_a_narrow_one_hardly_at_all() {
    let narrow = ticks_to(&bend(), &march(80.0, 2), None);
    let wide = ticks_to(&bend(), &march(80.0, 12), None);

    assert!(narrow >= 100, "{narrow}");
    assert!(wide > narrow, "a wider line sweeps slower: {wide} against {narrow}");
}

#[test]
fn a_tighter_slack_waits_longer_for_the_files_round_a_bend() {
    let loose = ticks_to(&bend(), &march(80.0, 8), None);
    let tight = ticks_to(&bend(), &March { slack: 0.5, ..march(80.0, 8) }, None);

    assert!(tight > loose, "{tight} against {loose}");
}

#[test]
fn slow_ground_on_the_way_slows_the_march() {
    let content = Content::load(&repo_content()).unwrap_or_else(|error| panic!("{error}"));
    let mut ground = Ground::new(256, 64, GroundTuning::of(&content.tuning));
    let open = ticks_to(&straight(), &march(80.0, 4), Some(&ground));
    ground.paint((20, 0, 20, 64), &Paint { cost: Some(0.5), ..Paint::default() });

    assert_eq!(open, 100);
    assert!(ticks_to(&straight(), &march(80.0, 4), Some(&ground)) > open);
}

#[test]
fn every_wave_waits_for_the_longest_march() {
    assert_eq!(waits(&[120, 95, 120, 0]), vec![0, 25, 0, 120]);
    assert!(waits(&[]).is_empty());
}
