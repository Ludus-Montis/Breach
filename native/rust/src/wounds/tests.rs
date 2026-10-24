//! FormationWounds' scenarios (Decision 121, spec 28 part 6), from test_formation_wounds.gd,
//! test_formation_recovery.gd's fooled taker and test_formation_wounds_order.gd's messenger,
//! on squads built here: each unit in the next store slot, at 10 hp, constitution 6, striking
//! for 4 a second; ten ticks a second. Faction 0 is the kingdom's, 1 the player's.

use super::*;
use crate::content::tests::repo_content;
use crate::content::Content;

const SECOND: i64 = 10;

fn tuning() -> WoundsTuning {
    let content = Content::load(&repo_content()).unwrap_or_else(|error| panic!("{error}"));
    WoundsTuning::of(&content.tuning)
}

/// The store the units stand in.
#[derive(Default)]
struct Built {
    columns: Columns,
    squads: Vec<Tended>,
}

impl Built {
    /// A unit standing at `at` with `hp`, in squad `squad` (made as needed).
    fn unit(&mut self, squad: usize, id: i64, faction: u32, at: (f32, f32), hp: i32) -> At {
        let slot = self.columns.position.len();
        self.columns.grow();
        self.columns.position[slot] = V2::new(at.0, at.1);
        self.columns.hp[slot] = hp;
        while self.squads.len() <= squad {
            self.squads.push(Tended::default());
        }
        let unit = Taking::plain(id, slot, faction);
        let list =
            if hp > 0 { &mut self.squads[squad].living } else { &mut self.squads[squad].downed };
        list.push(unit);
        (squad, list.len() - 1)
    }

    fn living(&mut self, at: At) -> &mut Taking {
        &mut self.squads[at.0].living[at.1]
    }

    fn tend(&mut self, t: &WoundsTuning) -> Tending {
        tend(&mut self.columns, &self.squads, SECOND, 1, t)
    }

    fn hp(&self, at: At) -> i32 {
        self.columns.hp[self.squads[at.0].downed[at.1].slot]
    }
}

/// A downed kingdom unit at the origin and a player unit one cell off: [the body, the taker].
fn scene() -> (Built, At, At) {
    let mut b = Built::default();
    let body = b.unit(0, 1, 0, (0.0, 0.0), 0);
    let taker = b.unit(1, 2, 1, (1.0, 0.0), 10);
    (b, body, taker)
}

#[test]
fn an_unguarded_body_is_struck_through_deaths_door_till_it_dies() {
    let t = tuning();
    let (mut b, body, _) = scene();
    let mut finished = false;
    for _ in 1..4 * SECOND {
        if b.tend(&t).taken.contains(&Taken::Finished { body }) {
            finished = true;
            break;
        }
    }
    assert!(finished);
    assert!(b.hp(body) <= -6);
}

#[test]
fn a_captor_captures_it_instead() {
    let t = tuning();
    let (mut b, body, taker) = scene();
    b.living(taker).captor = true;
    assert_eq!(b.tend(&t).taken, vec![Taken::Captured { body, by: taker }]);
    assert_eq!(b.hp(body), 0);
}

#[test]
fn a_friend_near_guards_it_and_a_foe_near_keeps_the_taker_fighting() {
    let t = tuning();
    let (mut guarded, body, _) = scene();
    guarded.unit(0, 3, 0, (-3.5, 0.0), 10); // guards it
    let (mut pressed, _, _) = scene();
    pressed.unit(0, 3, 0, (5.0, 0.0), 10); // near the taker
    for _ in 1..4 * SECOND {
        assert!(guarded.tend(&t).taken.is_empty());
        assert!(pressed.tend(&t).taken.is_empty());
    }
    assert_eq!(guarded.hp(body), 0);
    assert_eq!(pressed.hp(body), 0);
}

#[test]
fn a_leader_with_a_messenger_sends_one_home() {
    let t = tuning();
    let (mut b, body, _) = scene();
    let leader = b.unit(1, 4, 1, (30.0, 0.0), 10);
    b.living(leader).leads = true;
    b.living(leader).messengers = 1;
    assert_eq!(b.tend(&t).taken, vec![Taken::SentHome { body, leader }]);
}

/// Two kingdom bodies, each beside a player taker of one squad whose leader has a messenger
/// for only one of them.
fn two_bodies(reversed: bool) -> Built {
    let mut b = Built::default();
    let order: Vec<i64> = if reversed { vec![1, 0] } else { vec![0, 1] };
    for n in order {
        b.unit(0, 10 + n, 0, (20.0 * n as f32, 0.0), 0);
        b.unit(1, 20 + n, 1, (20.0 * n as f32 + 1.0, 0.0), 10);
    }
    let leader = b.unit(1, 30, 1, (60.0, 0.0), 10);
    b.living(leader).leads = true;
    b.living(leader).messengers = 1;
    b
}

/// What the takings came to, by the bodies' and units' ids.
fn by_id(b: &Built, tending: &Tending) -> Vec<(i64, i64)> {
    let body = |at: At| b.squads[at.0].downed[at.1].id;
    let unit = |at: At| b.squads[at.0].living[at.1].id;
    let mut out: Vec<(i64, i64)> = tending
        .taken
        .iter()
        .map(|taken| match *taken {
            Taken::Captured { body: at, by } => (body(at), unit(by)),
            Taken::SentHome { body: at, leader } => (body(at), -unit(leader)),
            Taken::Finished { body: at } => (body(at), 0),
        })
        .collect();
    out.sort();
    out
}

#[test]
fn which_body_a_messenger_takes_home_does_not_hang_on_the_list() {
    let t = tuning();
    let mut forward = two_bodies(false);
    let mut reversed = two_bodies(true);
    let ahead = forward.tend(&t);
    let behind = reversed.tend(&t);
    assert_eq!(ahead.taken.len(), 1);
    assert_eq!(by_id(&forward, &ahead), by_id(&reversed, &behind));
    let hp = |b: &Built, id: i64| {
        let unit = b.squads[0].downed.iter().find(|u| u.id == id).unwrap();
        b.columns.hp[unit.slot]
    };
    for id in [10, 11] {
        assert_eq!(hp(&forward, id), hp(&reversed, id));
    }
}

#[test]
fn a_thorough_foe_is_fooled_less_often() {
    let t = tuning();
    let mut fooled = [0, 0];
    for index in 0..60 {
        for (which, count) in fooled.iter_mut().enumerate() {
            let mut b = Built::default();
            let body = b.unit(0, index + 1, 1, (0.0, 0.0), 0);
            b.squads[body.0].downed[body.1].playing_dead = true;
            let taker = b.unit(1, 1000 + index, 0, (1.0, 0.0), 10);
            b.living(taker).thorough = 2 * which as i64;
            let slot = b.living(taker).slot;
            let tending = b.tend(&t);
            if tending.taken.is_empty() && b.columns.target_id[slot] == 0 {
                *count += 1;
            }
        }
    }
    assert!(fooled[0] > fooled[1]);
    assert!((5..=55).contains(&fooled[0]));
}

#[test]
fn a_struck_router_may_surrender_unless_it_never_does() {
    let mut columns = Columns::default();
    let mut caught = vec![];
    for index in 0..20 {
        columns.grow();
        columns.hp[index] = 10;
        caught.push(Caught { id: 10 + index as i64, slot: index, courage: 0.0 });
    }
    let taken = surrenders(&columns, &caught, 1, 1, 0.5);
    let count = taken.iter().filter(|&&t| t).count();
    assert!((3..=17).contains(&count)); // half each, by the seeded roll
    let brave: Vec<Caught> =
        caught.iter().map(|c| Caught { courage: 100.0, ..c.clone() }).collect();
    assert!(surrenders(&columns, &brave, 1, 1, 0.5).iter().all(|&t| !t));
    columns.hp.iter_mut().for_each(|hp| *hp = 0);
    assert!(surrenders(&columns, &caught, 1, 1, 0.5).iter().all(|&t| !t));
}
