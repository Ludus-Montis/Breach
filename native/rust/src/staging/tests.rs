//! FormationStaging's trigger and a coordinated leader's timing (Decision 87, spec 27 round
//! 2), on squads built here: each one unit (detection 8 cells) standing at its point, the
//! staged squad first at the origin. The scenarios through a battle (holding, signalled,
//! giving up, turning back) stay gdUnit.

use super::*;
use crate::ground::Walker;
use crate::locks::{FIGHTING, HOLDING};
use crate::roster::Member;

const GREM: Walker = Walker { height: 1.0, swimmer: 0, climber: 0, swims: false, climbs: false };
const PACE: [f64; 2] = [8.0, 0.1];

/// A squad of one: its id, faction, route code, where it stands, its pace and state.
#[derive(Clone, Copy)]
struct S(i64, u32, u32, f32, f64, i32);

fn friend(id: i64, route: u32, x: f32) -> S {
    S(id, 0, route, x, 1.0, HOLDING)
}

struct Built {
    squads: Squads,
    columns: Columns,
    roster: Roster,
    routes: Vec<u32>,
}

fn built(all: &[S]) -> Built {
    let mut b = Built {
        squads: Squads::default(),
        columns: Columns::default(),
        roster: Roster::default(),
        routes: vec![],
    };
    for (k, s) in all.iter().enumerate() {
        b.squads.open(s.0);
        b.squads.state[k] = s.5;
        b.squads.width[k] = 1;
        b.squads.position[k] = V2::new(s.3, 0.0);
        b.columns.grow();
        b.columns.position[k] = V2::new(s.3, 0.0);
        b.columns.speed[k] = s.4;
        let member = Member {
            id: 100 + s.0,
            slot: k,
            foot: (1, 1),
            preferred: 0,
            attack_range: 0,
            kind: 0,
            turn_rate: 360.0,
            backward_pace: 0.5,
            detection: 8.0,
            max_stamina: 100.0,
            walker: GREM,
        };
        let listed =
            Listed { squad: k, id: s.0, faction: s.1, living: vec![member], ..Listed::default() };
        b.roster.listed.push(listed);
        b.routes.push(s.2);
    }
    b
}

fn staged(b: &Built, sees_fight: bool, partner: i64) -> Staged<'_> {
    Staged { listed: 0, sees_fight, partner, routes: &b.routes, meet: None, pace: PACE }
}

fn fires(b: &Built, staged: &Staged) -> bool {
    signalled(&b.squads, &b.columns, &b.roster, None, staged)
}

#[test]
fn a_wave_waiting_to_see_its_partner_goes_once_it_detects_it() {
    let near = built(&[friend(1, 0, 0.0), friend(2, 1, 6.0)]);
    let far = built(&[friend(1, 0, 0.0), friend(2, 1, 20.0)]);

    assert!(fires(&near, &staged(&near, false, 2)));
    assert!(!fires(&far, &staged(&far, false, 2)));
}

#[test]
fn only_the_named_partner_fires_it() {
    let b = built(&[friend(1, 0, 0.0), friend(2, 1, 6.0), friend(3, 1, 20.0)]);

    assert!(!fires(&b, &staged(&b, false, 3)));
}

#[test]
fn with_no_partner_named_a_friend_on_another_route_fires_it_and_none_on_its_own() {
    let own_route = built(&[friend(1, 0, 0.0), friend(2, 0, 6.0)]);
    let other_route = built(&[friend(1, 0, 0.0), friend(2, 1, 6.0)]);
    let hostile = built(&[friend(1, 0, 0.0), S(2, 1, 1, 6.0, 1.0, HOLDING)]);

    assert!(!fires(&own_route, &staged(&own_route, false, 0)));
    assert!(fires(&other_route, &staged(&other_route, false, 0)));
    assert!(!fires(&hostile, &staged(&hostile, false, 0)));
}

#[test]
fn a_wave_waiting_to_see_a_fight_goes_only_once_its_partner_fights() {
    let holding = built(&[friend(1, 0, 0.0), friend(2, 1, 6.0)]);
    let fighting = built(&[friend(1, 0, 0.0), S(2, 0, 1, 6.0, 1.0, FIGHTING)]);

    assert!(!fires(&holding, &staged(&holding, true, 2)));
    assert!(fires(&fighting, &staged(&fighting, true, 2)));
}

/// A meeting 40 cells east along the staged squad's route (its front at 0 cells).
fn meeting() -> Meeting {
    Meeting {
        at: V2::new(40.0, 0.0),
        cells: 40.0,
        route: Route::new(vec![V2::ZERO, V2::new(100.0, 0.0)], 0.0),
        slack: 2.0,
    }
}

/// Whether a coordinated leader goes, its partner (seen 6 cells away) standing `partner_x`
/// cells from the meeting point at `partner_pace`.
fn coordinated(partner_x: f32, partner_pace: f64) -> bool {
    let mut b = built(&[friend(1, 0, 0.0), S(2, 0, 1, 6.0, partner_pace, HOLDING)]);
    b.squads.position[1] = V2::new(40.0, partner_x);
    fires(&b, &Staged { meet: Some(meeting()), ..staged(&b, false, 2) })
}

#[test]
fn a_coordinated_leader_waits_while_it_would_arrive_before_its_partner() {
    // its own march: 50 ticks; the partner's 80 cells at 0.8 a tick: 100
    assert!(!coordinated(80.0, 1.0));
}

#[test]
fn a_coordinated_leader_goes_when_it_would_arrive_no_earlier_than_its_partner() {
    assert!(coordinated(40.0, 1.0)); // 50 ticks each
    assert!(coordinated(41.0, 1.0)); // the partner 51 ticks away: within the slack
    assert!(coordinated(80.0, 0.0)); // a partner standing still is not waited for
}

#[test]
fn the_lists_order_does_not_change_whether_it_fires() {
    let all = [friend(1, 0, 0.0), friend(2, 1, 6.0), friend(3, 1, 7.0), friend(4, 0, 30.0)];
    let mut reversed = all;
    reversed[1..].reverse();
    let (a, b) = (built(&all), built(&reversed));

    for partner in [0, 2, 4] {
        assert_eq!(fires(&a, &staged(&a, false, partner)), fires(&b, &staged(&b, false, partner)));
    }
}

/// The staged squad alone, `front` tiles along from home (0), in `state`.
fn alone(front: f64, state: i32) -> Built {
    let mut b = built(&[S(1, 0, 0, 0.0, 1.0, state)]);
    b.squads.front_distance[0] = front;
    b
}

fn holds(b: &mut Built, at: f64, fallback: i64) -> Hold {
    let routes = b.routes.clone();
    let staged = Staged {
        listed: 0,
        sees_fight: false,
        partner: 0,
        routes: &routes,
        meet: None,
        pace: PACE,
    };
    hold(&mut b.squads, &b.columns, &b.roster, None, &staged, at, fallback)
}

#[test]
fn short_of_its_staging_point_it_marches_on() {
    let mut b = alone(1.0, crate::locks::MOVING);
    assert_eq!(holds(&mut b, 2.0 * CELLS_PER_TILE as f64, 5), Hold::OnItsWay);
    assert_eq!(b.squads.state[0], crate::locks::MOVING);
}

#[test]
fn at_its_staging_point_it_holds_first_staged_then_holding() {
    let mut b = alone(2.0, crate::locks::MOVING);
    let at = 2.0 * CELLS_PER_TILE as f64;
    assert_eq!(holds(&mut b, at, 5), Hold::Holds { staged: true });
    assert_eq!(b.squads.state[0], HOLDING);
    assert_eq!(holds(&mut b, at, 4), Hold::Holds { staged: false });
}

#[test]
fn it_gives_up_as_its_last_tick_of_fallback_runs_out() {
    let mut b = alone(3.0, HOLDING);
    assert_eq!(holds(&mut b, 2.0 * CELLS_PER_TILE as f64, 2), Hold::Holds { staged: false });
    assert_eq!(holds(&mut b, 2.0 * CELLS_PER_TILE as f64, 1), Hold::GivesUp { staged: false });
    let mut fresh = alone(3.0, crate::locks::MOVING);
    let gave = holds(&mut fresh, 2.0 * CELLS_PER_TILE as f64, 1);
    assert_eq!(gave, Hold::GivesUp { staged: true });
}

#[test]
fn a_trigger_fired_goes_before_it_holds() {
    let mut b = built(&[S(1, 0, 0, 0.0, 1.0, HOLDING), friend(2, 1, 6.0)]);
    b.squads.front_distance[0] = 3.0;
    assert_eq!(holds(&mut b, 2.0 * CELLS_PER_TILE as f64, 5), Hold::Signalled);
}
