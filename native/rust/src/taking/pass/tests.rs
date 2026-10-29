//! The taking's scenarios (Decisions 121 and 124), from test_formation_taking.gd and
//! test_formation_perception.gd, on squads built here: squad 1 (listed 0, faction "player"),
//! units 100, 101 and 102 holding at x 0, 1 and 2 along y 0; squad 2 (listed 1, faction
//! "the_kingdom"), unit 200 downed 4 cells ahead of unit 100. Each sees 40 cells.

use super::*;
use crate::maths::V2;
use crate::squads::DOWNED;

const TICK: i64 = 10;
const SECONDS: f64 = 0.1;
const SEED: i64 = 7;

fn tuning() -> TakingTuning {
    TakingTuning { hold: 2.0, reach: 8.0 }
}

struct Built {
    squads: Squads,
    field: Field,
}

fn built() -> Built {
    let (mut squads, mut field) = (Squads::default(), Field::default());
    let factions = [field.lexicon.code("player"), field.lexicon.code("the_kingdom")];
    for (s, ids) in [&[100, 101, 102][..], &[200][..]].into_iter().enumerate() {
        squads.open(s as i64 + 1);
        squads.faction[s] = factions[s];
        squads.state[s] = HOLDING;
        let command = squads.commands.open(0.0);
        squads.command[s] = Some(command);
        for (n, &id) in ids.iter().enumerate() {
            let slot = field.hold(id) as usize;
            field.columns.position[slot] = V2::new(n as f32, 0.0);
            field.columns.words[slot].faction_id = factions[s];
            field.columns.sheet[slot].detection = 40.0;
            squads.members[s].push(slot as i32);
        }
    }
    let body = field.slot_of(200).unwrap() as usize;
    field.columns.position[body] = V2::new(0.0, 4.0);
    field.columns.state[body] = DOWNED;
    Built { squads, field }
}

impl Built {
    fn slot(&self, id: i64) -> usize {
        self.field.slot_of(id).unwrap() as usize
    }

    /// The pass over both squads on `tick`, squad 1 threatened or not; what it logged.
    fn take(&mut self, tick: i64, threatened: bool) -> Vec<i64> {
        let mut log = Log::default();
        let listed = (&[0, 1][..], &[threatened, false][..]);
        step(&mut self.squads, &self.field, listed, (tick, SECONDS, SEED), &tuning(), &mut log);
        log.take()
    }

    fn trait_all(&mut self, name: &str) {
        let code = self.field.lexicon.code(name);
        for &u in &self.squads.members[0] {
            self.field.columns.words[u as usize].traits.push((code, 1.0));
        }
    }
}

#[test]
fn the_nearest_free_unit_walks_out_to_the_body_and_its_group_holds() {
    let mut b = built();

    let logged = b.take(TICK, false);

    assert_eq!(b.squads.taking[0], vec![(100, b.slot(200))]);
    assert_eq!(b.squads.taking_since[0], TICK);
    assert_eq!(logged, vec![events::SENT_TO_TAKE as i64, TICK, 1, b.squads.faction[0] as i64, 100]);
}

#[test]
fn a_taker_out_is_not_sent_again_nor_its_body_claimed_twice() {
    let mut b = built();
    b.take(TICK, false);

    let logged = b.take(TICK + 1, false);

    assert!(logged.is_empty());
    assert_eq!(b.squads.taking[0], vec![(100, b.slot(200))]);
    assert_eq!(b.squads.taking_since[0], TICK);
}

#[test]
fn a_body_beyond_reach_is_left() {
    let mut b = built();
    let body = b.slot(200);
    b.field.columns.position[body] = V2::new(0.0, 9.0);

    assert!(b.take(TICK, false).is_empty());
    assert!(b.squads.taking[0].is_empty());
}

#[test]
fn fall_behind_left_behind_and_a_command_that_leaves_the_downed_take_none() {
    let mut b = built();
    b.trait_all("fall_behind_left_behind");
    assert!(b.take(TICK, false).is_empty());

    let mut b = built();
    let command = b.squads.command[0].unwrap();
    b.squads.commands.flags[command] &= !TAKES;
    assert!(b.take(TICK, false).is_empty());
}

#[test]
fn a_threatened_group_sends_none_and_calls_back_those_out() {
    let mut b = built();
    assert!(b.take(TICK, true).is_empty());

    b.take(TICK + 1, false);
    b.take(TICK + 2, true);

    assert!(b.squads.taking[0].is_empty());
    assert_eq!((b.squads.taking_since[0], b.squads.took_until[0]), (-1, -1));
}

#[test]
fn a_taker_comes_back_once_its_body_is_no_longer_downed_or_it_has_fallen() {
    let mut b = built();
    b.take(TICK, false);
    let body = b.slot(200);
    b.field.columns.state[body] = DEAD;
    b.take(TICK + 1, false);
    assert!(b.squads.taking[0].is_empty());

    let mut b = built();
    b.take(TICK, false);
    let taker = b.slot(100);
    b.field.columns.state[taker] = DEAD;
    b.take(TICK + 1, false);
    assert_eq!(b.squads.taking[0], vec![(101, b.slot(200))]); // the next nearest goes instead
    assert_eq!(b.squads.taking_since[0], TICK + 1);
}

#[test]
fn the_hold_runs_out_and_the_group_gives_up_till_its_next_fight() {
    let mut b = built();
    b.take(TICK, false);
    let out = TICK + 20; // take_hold, 2 seconds of 0.1

    b.take(out - 1, false);
    assert_eq!(b.squads.taking[0].len(), 1);
    assert!(b.take(out, false).is_empty());
    assert!(b.squads.taking[0].is_empty());
    assert_eq!(b.squads.took_until[0], out);
    assert!(b.take(out + 1, false).is_empty()); // given up: sends none

    b.squads.state[0] = FIGHTING;
    b.take(out + 2, false);
    assert_eq!(b.squads.took_until[0], -1); // a new fight: worth taking again
}

#[test]
fn no_man_left_behind_holds_for_good() {
    let mut b = built();
    b.trait_all("no_man_left_behind");
    b.take(TICK, false);

    b.take(TICK + 500, false);

    assert_eq!(b.squads.taking[0], vec![(100, b.slot(200))]);
}

#[test]
fn a_group_no_longer_at_ease_calls_its_takers_back() {
    let mut b = built();
    b.take(TICK, false);
    b.squads.state[0] = FIGHTING;

    b.take(TICK + 1, false);

    assert!(b.squads.taking[0].is_empty());
}
