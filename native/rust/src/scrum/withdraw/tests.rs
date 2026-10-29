//! The scrum's withdrawal scenarios: a player squad (slot 0, id 1) of grems withdrawing west
//! along a straight route on y = 0.5, home at x = 0, facing east up it, and a kingdom
//! squad (slot 1, id 2) of one grem at `foe`.

use super::*;
use crate::entries;
use crate::ground::Walker;
use crate::locks;

/// The hold order (SkirmishUnit.Order.HOLD).
const HOLD: i32 = 1;
use crate::maths::V2;
use crate::roster::{Listed, Member};
use crate::route::Route;

const GREM: Walker = Walker { height: 1.0, swimmer: 0, climber: 0, swims: false, climbs: false };

fn rules() -> Rules {
    Rules {
        flight: withdraw::Rules { pace: 0.25, seconds: 0.1, reach: 0.2, near: 4.0, fan_cells: 3.0 },
        rally_seconds: 0.3,
        fan_degrees: 30.0,
        discipline: DisciplineTuning { meets_threats: 50, ..DisciplineTuning::default() },
        reform: [60.0, 0.25, 1.5],
        seed: 7,
        tick: 4,
    }
}

fn grem(id: i64, slot: usize) -> Member {
    Member {
        id,
        slot,
        foot: (1, 1),
        preferred: 0,
        attack_range: 0,
        kind: 0,
        turn_rate: 3600.0,
        backward_pace: 0.5,
        detection: 8.0,
        max_stamina: 100.0,
        walker: GREM,
    }
}

struct Built {
    squads: Squads,
    field: Field,
    roster: Roster,
    log: Log,
}

/// The player's grems loose at `ours`, withdrawing (safe for `safe` ticks), its front 30
/// cells up the route; the kingdom's grem at `foe`.
fn built(ours: &[V2], foe: V2, safe: i64) -> Built {
    let (mut squads, mut field, mut roster) =
        (Squads::default(), Field::default(), Roster::default());
    for s in 0..2 {
        let id = s as i64 + 1;
        squads.open(id);
        squads.faction[s] = s as u32;
        roster.listed.push(Listed { squad: s, id, faction: s as u32, ..Listed::default() });
    }
    squads.route[0] = Some(Route::new(vec![V2::new(0.0, 0.5), V2::new(40.0, 0.5)], 0.0));
    squads.direction[0] = 1;
    squads.heading[0] = 90.0;
    squads.front_distance[0] = 30.0 / CELLS;
    squads.withdraw[0] = Some(safe);
    for (n, &at) in ours.iter().enumerate() {
        let unit = n as i64 + 1;
        let slot = field.hold(unit) as usize;
        field.columns.speed[slot] = 1.0;
        field.columns.bearing[slot] = 270.0;
        squads.members[0].push(slot as i32);
        entries::loosen(&mut squads, &mut field.columns, 0, (unit, slot), at);
        roster.listed[0].living.push(grem(unit, slot));
    }
    let slot = field.hold(101) as usize;
    field.columns.position[slot] = foe;
    squads.members[1].push(slot as i32);
    roster.listed[1].living.push(grem(101, slot));
    Built { squads, field, roster, log: Log::default() }
}

impl Built {
    fn step(&mut self) {
        let (squads, field, log) = (&mut self.squads, &mut self.field, &mut self.log);
        step(squads, field, &self.roster, None, &rules(), log);
    }

    /// The kinds of the events logged, in order.
    fn kinds(&mut self) -> Vec<&'static str> {
        let flat = self.log.take();
        let mut out = Vec::new();
        let mut at = 0;
        while at < flat.len() {
            let (name, fields) = events::KINDS[flat[at] as usize];
            out.push(name);
            at += 4 + fields.len();
        }
        out
    }
}

#[test]
fn a_withdrawing_squads_units_flee_home_along_its_route() {
    let mut b = built(&[V2::new(20.0, 0.5)], V2::new(22.5, 0.5), 0);
    b.step();
    assert!(b.field.columns.loose_at[0].x < 20.0);
    assert_eq!(b.squads.withdraw[0], Some(0), "an enemy near: not safe");
    assert!(b.kinds().is_empty());
}

#[test]
fn safe_long_enough_it_turns_for_home_and_regroups() {
    let mut b = built(&[V2::new(20.0, 0.5), V2::new(24.0, 0.5)], V2::new(39.0, 30.0), 2);
    b.step();
    assert!(b.squads.withdraw[0].is_none());
    assert_eq!(b.squads.direction[0], -1, "it about-faced for home");
    assert_eq!(b.kinds(), vec!["turning", "turned", "regrouping"]);
}

#[test]
fn an_about_face_cancels_the_swaps_under_way() {
    let mut b = built(&[V2::new(20.0, 0.5)], V2::new(39.0, 30.0), 2);
    b.squads.swaps[0].push(crate::swaps::Held {
        ticks: 3,
        total: 3,
        units: Vec::new(),
        passed: Vec::new(),
    });
    b.step();
    assert!(b.squads.swaps[0].is_empty());
}

#[test]
fn with_all_its_units_home_it_holds_there_facing_out_and_returns() {
    let mut b = built(&[V2::new(0.2, 0.5)], V2::new(39.0, 30.0), 0);
    b.step();
    assert_eq!(b.squads.order[0], HOLD);
    assert_eq!(b.kinds(), vec!["regrouping", "returned"], "facing out already: no turn");
}

#[test]
fn a_routing_squad_withdraws_no_more() {
    let mut b = built(&[V2::new(20.0, 0.5)], V2::new(39.0, 30.0), 2);
    b.squads.state[0] = locks::ROUTING;
    b.step();
    assert!(b.squads.withdraw[0].is_none());
    assert!(b.kinds().is_empty());
}

#[test]
fn its_disorder_reform_pace_home_and_fans_are_read_in_the_crate() {
    let mut b = built(&[V2::new(20.0, 0.5)], V2::new(39.0, 30.0), 0);
    b.squads.home_distance[0] = 0.25;
    let w = withdrawer(&b.squads, &b.field, &b.roster, 0, &rules());
    assert_eq!((w.disorder, w.reform, w.home), (0.4, 0.5, 16.0)); // discipline 30 of 50, of 60
    assert_eq!(w.fans, vec![flight::fan(1, 7, 30.0)]);
}
