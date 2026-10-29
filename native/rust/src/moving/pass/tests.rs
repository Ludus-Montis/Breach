//! The move's pass over squads and units built here: lines of two grems abreast on a lane
//! ten tiles long, their fronts placed by hand and their units standing at them.

use super::*;
use crate::field::Field;
use crate::ground::Walker;
use crate::locks::MOVING;
use crate::roster::{Listed, Member};
use crate::route::Route;
use crate::staging::Stage;
use crate::swaps::Held;

const EAST: f64 = 90.0;
const TICK: i64 = 3;

/// The lane: ten tiles east along x.
fn lane() -> Route {
    Route::new(vec![V2::new(0.0, 0.0), V2::new(10.0 * CELLS as f32, 0.0)], 0.0)
}

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
        walker: Walker { height: 1.0, swimmer: 0, climber: 0, swims: false, climbs: false },
    }
}

struct Built {
    squads: Squads,
    field: Field,
    roster: Roster,
}

/// A squad per entry of `fronts` (tiles along the lane), ids from 1, of one faction, each
/// advancing east from home at 0 with two grems of speed 1 abreast in its front rank.
fn built(fronts: &[f64]) -> Built {
    let (mut squads, mut field, mut roster) =
        (Squads::default(), Field::default(), Roster::default());
    for (s, &front) in fronts.iter().enumerate() {
        squads.open(s as i64 + 1);
        squads.direction[s] = 1;
        squads.heading[s] = EAST;
        squads.width[s] = 2;
        squads.front_distance[s] = front;
        squads.position[s] = lane().point_at(front * CELLS);
        squads.route[s] = Some(lane());
        let mut living = Vec::new();
        for column in 0..2 {
            let id = 100 * (s as i64 + 1) + column;
            let slot = field.hold(id) as usize;
            field.columns.column[slot] = column as i32;
            field.columns.speed[slot] = 1.0;
            field.columns.position[slot] = lane().point_at(front * CELLS);
            squads.members[s].push(slot as i32);
            living.push(grem(id, slot));
        }
        roster.listed.push(Listed { squad: s, id: s as i64 + 1, living, ..Listed::default() });
    }
    Built { squads, field, roster }
}

impl Built {
    /// The pass on `tick`, every squad keeping `keeping` of its pace, on open ground; its
    /// joiners and the events it logged.
    fn run(&mut self, keeping: &[f64]) -> (Vec<usize>, Vec<i64>) {
        let routes = vec![0; self.roster.listed.len()];
        let given = Given {
            keeping,
            routes: &routes,
            tick: TICK,
            travel_scale: 0.125,
            seconds: 0.1,
            route_length: 10.0,
            slack: [0.0, 1.0, 4.0],
            drag: [0.5, 2.0],
        };
        let mut log = Log::default();
        let joiners = step(&mut self.squads, &mut self.field, &self.roster, None, &given, &mut log);
        (joiners, log.take())
    }
}

/// One event as the log holds it: its kind, the tick, squad 1 of faction 0, its values.
fn event(kind: usize, values: &[i64]) -> Vec<i64> {
    [&[kind as i64, TICK, 1, 0][..], values].concat()
}

#[test]
fn a_squad_steps_at_its_slowest_units_pace_within_the_share_its_units_keep_and_advances() {
    let mut b = built(&[2.0, 6.0]);
    b.field.columns.speed[1] = 0.5;
    let (joiners, log) = b.run(&[1.0, 0.5]);
    let step = 0.5 * 0.125 * 0.1;
    assert!((b.squads.front_distance[0] - (2.0 + step)).abs() < 1e-12);
    assert!((b.squads.front_distance[1] - (6.0 + 1.0 * 0.125 * 0.1 * 0.5)).abs() < 1e-12);
    assert_eq!(b.squads.position[0], lane().point_at(b.squads.front_distance[0] * CELLS));
    assert_eq!(b.squads.state[0], MOVING);
    assert_eq!(joiners, vec![0, 1], "advancing, for the joins");
    assert!(log.is_empty());
}

#[test]
fn a_squad_out_of_its_units_slack_waits_for_them() {
    let mut b = built(&[2.0]);
    let (joiners, _) = b.run(&[0.0]);
    assert_eq!(b.squads.front_distance[0], 2.0);
    assert!(joiners.is_empty());
}

#[test]
fn a_squad_reaching_the_enemys_end_arrives_there() {
    let mut b = built(&[9.999]);
    let (_, log) = b.run(&[1.0]);
    assert_eq!(b.squads.front_distance[0], 10.0);
    assert_eq!(b.squads.state[0], ARRIVED);
    assert_eq!(log, event(events::ARRIVED, &[]));
}

#[test]
fn a_squad_going_back_the_way_it_faces_turns_first_and_reports_its_new_heading() {
    let mut b = built(&[5.0]);
    b.squads.order[0] = RETREAT;
    b.squads.swaps[0] = vec![Held { ticks: 2, total: 4, units: Vec::new(), passed: Vec::new() }];
    let (joiners, log) = b.run(&[1.0]);
    assert_eq!(b.squads.direction[0], -1, "about-faced");
    assert!(b.squads.swaps[0].is_empty(), "its swaps under way cancelled");
    assert!(joiners.is_empty(), "it re-forms before it moves");
    let faced = [3, 270.0f64.to_bits() as i64];
    let turned = [event(events::TURNING, &faced), event(events::TURNED, &faced)].concat();
    assert_eq!(log, turned);
}

#[test]
fn a_staged_squad_holds_at_its_point_then_gives_up_and_turns_back_as_its_order_says() {
    let mut b = built(&[2.0]);
    let stage = Stage {
        at: 100.0,
        sees_fight: false,
        partner: 9,
        fallback: 2,
        then: Some("back".to_string()),
        meet: None,
    };
    b.squads.staging[0] = Some(stage);
    let (_, log) = b.run(&[1.0]);
    assert_eq!(log, event(events::STAGED, &[]));
    assert_eq!((b.squads.state[0], b.squads.front_distance[0]), (HOLDING, 2.0));
    let (_, log) = b.run(&[1.0]);
    assert!(b.squads.staging[0].is_none());
    assert_eq!(b.squads.order[0], RETREAT);
    let back = i64::from(b.field.lexicon.find("back").expect("coded"));
    assert_eq!(log[..5], event(events::GAVE_UP, &[back])[..]);
    assert_eq!(log[5], events::TURNING as i64, "then it turns for home");
}

#[test]
fn a_squad_still_waiting_counts_down_and_one_ordered_to_hold_holds() {
    let mut b = built(&[2.0, 6.0]);
    b.squads.wait_ticks[0] = 2;
    b.squads.order[1] = HOLD;
    b.run(&[1.0, 1.0]);
    assert_eq!(b.squads.wait_ticks[0], 1);
    assert_eq!((b.squads.state[1], b.squads.front_distance[1]), (HOLDING, 6.0));
}
