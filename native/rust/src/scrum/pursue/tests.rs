//! The scrum's pursuit scenarios: a player squad (slot 0, id 1) of two grems pursuing east
//! along a road on y = 0 from its post at x = 10, which it held on its own route on y = 4,
//! and the kingdom squad (slot 1, id 2) of two grems it pursues, retreating east.

use super::*;
use crate::frame::Frame;
use crate::ground::Walker;
use crate::locks;
use crate::maths::V2;
use crate::roster::{Listed, Member};
use crate::route::Route;
use crate::squads::Pursuit;

const GREM: Walker = Walker { height: 1.0, swimmer: 0, climber: 0, swims: false, climbs: false };
const CELLS: f64 = crate::geometry::CELLS;
/// The hold order (SkirmishUnit.Order.HOLD).
const HOLD: i32 = 1;

fn rules() -> Rules {
    Rules {
        whole: whole::Rules { cells_per_second: 20.0, seconds: 0.1, give_up: 0.2, lag: 2.0 },
        discipline: DisciplineTuning {
            expected_max: 100.0,
            leash_shares: vec![0.8, 0.5],
            leash_steps: vec![2.0, 4.0, 8.0, 16.0, 32.0],
            ..DisciplineTuning::default()
        },
        tick: 6,
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
        turn_rate: 90.0,
        backward_pace: 0.5,
        detection: 8.0,
        max_stamina: 100.0,
        walker: GREM,
    }
}

fn road() -> Route {
    Route::new(vec![V2::new(0.0, 0.0), V2::new(64.0, 0.0)], 0.0)
}

fn own() -> Route {
    Route::new(vec![V2::new(0.0, 4.0), V2::new(64.0, 4.0)], 0.5)
}

struct Built {
    squads: Squads,
    field: Field,
    roster: Roster,
    log: Log,
}

/// The pursuer at its post, its grems at their places, and its quarry's grems at `foes_at`.
fn built(foes_at: &[V2]) -> Built {
    let (mut squads, mut field, mut roster) =
        (Squads::default(), Field::default(), Roster::default());
    for s in 0..2 {
        let id = s as i64 + 1;
        squads.open(id);
        squads.width[s] = 1;
        squads.faction[s] = s as u32;
        roster.listed.push(Listed { squad: s, id, faction: s as u32, ..Listed::default() });
    }
    squads.route[0] = Some(road());
    squads.post_route[0] = Some(own());
    squads.front_distance[0] = 10.0 / CELLS;
    squads.position[0] = V2::new(10.0, 0.0);
    squads.direction[0] = 1;
    squads.heading[0] = 90.0;
    squads.engaged_with[0] = 2;
    squads.pursuit[0] = Some(Pursuit {
        foe: 2,
        post: 10.0 / CELLS,
        post_at: V2::new(10.0, 0.0),
        home: 0.25,
        direction: -1,
        order: HOLD,
        returning: false,
    });
    for rank in 0..2i32 {
        let unit = i64::from(rank) + 1;
        let slot = field.hold(unit) as usize;
        field.columns.rank[slot] = rank;
        let member = grem(unit, slot);
        field.columns.position[slot] = Frame::kept(&squads, 0).place((rank, 0), member.foot);
        squads.members[0].push(slot as i32);
        roster.listed[0].living.push(member);
    }
    for (k, &at) in foes_at.iter().enumerate() {
        let unit = 101 + k as i64;
        let slot = field.hold(unit) as usize;
        field.columns.position[slot] = at;
        squads.members[1].push(slot as i32);
        roster.listed[1].living.push(grem(unit, slot));
    }
    for slot in 0..field.columns.position.len() {
        field.columns.speed[slot] = 1.0;
        field.columns.stamina[slot] = 100.0;
    }
    squads.order[1] = locks::RETREAT;
    Built { squads, field, roster, log: Log::default() }
}

fn ahead_of_it() -> Built {
    built(&[V2::new(14.0, 0.0), V2::new(15.0, 0.0)])
}

impl Built {
    fn step(&mut self) -> Vec<usize> {
        let (squads, field, log) = (&mut self.squads, &mut self.field, &mut self.log);
        step(squads, field, &self.roster, &rules(), log)
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

    /// Gives the pursuer's living units `tactic`.
    fn tactic(&mut self, tactic: &str) {
        let code = self.field.lexicon.code(tactic);
        for m in &self.roster.listed[0].living {
            self.field.columns.words[m.slot].tactics.push(code);
        }
    }
}

#[test]
fn it_advances_after_its_enemy_along_the_route_it_is_on() {
    let mut b = ahead_of_it();
    assert_eq!(b.step(), Vec::<usize>::new());
    assert_eq!(b.squads.position[0], V2::new(12.0, 0.0), "along the road, not its own route");
    assert!(b.kinds().is_empty());
}

#[test]
fn its_leash_is_its_disciplines_step_moved_by_its_leaders_tactics() {
    let mut b = ahead_of_it(); // discipline 30 of 100: below both shares, the third step
    let leash = |b: &Built| pursuer(&b.squads, &b.field, &b.roster, 0, &rules().discipline).leash;
    assert_eq!(leash(&b), 16.0);
    b.tactic("pursues");
    assert_eq!(leash(&b), 32.0, "a step out");
    b.tactic("cautious");
    assert_eq!(leash(&b), 16.0, "and a step back in");
    let mut b = ahead_of_it();
    b.tactic("cautious");
    assert_eq!(leash(&b), 8.0);
}

#[test]
fn at_its_leash_it_gives_up_and_the_end_is_logged() {
    let mut b = ahead_of_it();
    b.tactic("cautious");
    b.squads.position[0] = V2::new(18.0, 0.0); // 8 cells out: its leash
    assert_eq!(b.step(), Vec::<usize>::new());
    assert!(b.squads.pursuit[0].is_some_and(|p| p.returning));
    assert_eq!(b.kinds(), vec!["pursuit_ended"]);
}

#[test]
fn back_at_its_post_it_takes_up_its_own_route_and_turns_to_face_the_way_it_held_it() {
    let mut b = ahead_of_it();
    let held = b.squads.pursuit[0].unwrap();
    b.squads.pursuit[0] = Some(Pursuit { returning: true, ..held });
    b.squads.order[0] = 0; // ADVANCE: its march home ended
    assert_eq!(b.step(), vec![0]);
    assert!(b.squads.pursuit[0].is_none());
    assert_eq!(b.squads.route[0], Some(own()));
    assert_eq!(b.squads.post_route[0], None);
    assert_eq!(b.squads.direction[0], -1, "it about-faced the way it held its post");
    assert_eq!(b.squads.heading[0], 270.0);
    assert_eq!(b.kinds(), vec!["turning", "turned"]);
}

#[test]
fn a_destroyed_pursuer_pursues_no_more() {
    let mut b = ahead_of_it();
    b.roster.listed[0].living.clear();
    assert_eq!(b.step(), Vec::<usize>::new());
    assert_eq!(b.squads.position[0], V2::new(10.0, 0.0));
    assert!(b.squads.pursuit[0].is_some_and(|p| !p.returning));
}
