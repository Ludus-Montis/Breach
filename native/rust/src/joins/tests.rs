//! The joins' scenarios (Decisions 44, 51 and 97), on squads built here: a wave of two grems
//! abreast at x 0 behind a friend of two at x 1, both facing east, each unit standing in its
//! front rank.

use super::*;
use crate::ground::Walker;
use crate::maths::V2;
use crate::roster::{Listed, Member};

const EAST: f64 = 90.0;
const TICK: i64 = 6;
/// SkirmishUnit.State's DEAD.
const DEAD: i32 = 4;
/// The friend's command's slot.
const COMMAND: i64 = 4;

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

/// The wave (squad 1, listed 0, units 100 and 101 in slots 0 and 1) and the friend (squad 2,
/// listed 1, units 200 and 201 in slots 2 and 3, under command `COMMAND`), all of faction 0;
/// the wave's units started under no command.
fn built(wave_at: f32) -> Built {
    let (mut squads, mut field, mut roster) =
        (Squads::default(), Field::default(), Roster::default());
    for (s, x) in [wave_at, 1.0].into_iter().enumerate() {
        squads.open(s as i64 + 1);
        squads.position[s] = V2::new(x, 0.0);
        squads.heading[s] = EAST;
        squads.width[s] = 2;
        let mut living = Vec::new();
        for column in 0..2 {
            let id = 100 * (s as i64 + 1) + column;
            let slot = field.hold(id) as usize;
            field.columns.column[slot] = column as i32;
            field.columns.sheet[slot].squad_id = s as i64 + 1;
            squads.members[s].push(slot as i32);
            living.push(grem(id, slot));
        }
        roster.listed.push(Listed { squad: s, id: s as i64 + 1, living, ..Listed::default() });
    }
    squads.command[1] = Some(COMMAND as usize);
    Built { squads, field, roster }
}

impl Built {
    /// The joins of the wave, by seed 7; and the events they logged.
    fn run(&mut self) -> (Vec<(usize, usize)>, Vec<i64>) {
        let mut log = Log::default();
        let joins = step(&mut self.squads, &mut self.field, &self.roster, &[0], 7, TICK, &mut log);
        (joins, log.take())
    }
}

#[test]
fn a_wave_reaching_a_fighting_friends_back_rank_is_taken_in_behind_it_and_reported() {
    let mut b = built(0.0);
    b.squads.state[1] = FIGHTING;

    let (joins, logged) = b.run();

    assert_eq!(joins, vec![(1, 0)]);
    assert_eq!(logged, vec![events::REINFORCED as i64, TICK, 1, 0, 2]); // nobody steps up
    assert_eq!((b.field.columns.rank[0], b.field.columns.rank[1]), (1, 1));
    assert_eq!(b.squads.members[1], vec![2, 3, 0, 1]);
    assert!(b.squads.members[0].is_empty());
    assert_eq!(b.squads.joined[1], vec![100, 101]);
    assert_eq!(b.field.columns.sheet[0].squad_id, 2);
    let objects = &b.field.columns.objects[1];
    assert_eq!((objects.command, objects.origin), (COMMAND, COMMAND));
    assert_eq!(b.squads.reforming[1], 1);
}

#[test]
fn a_merging_wave_steps_up_where_the_friends_front_has_fallen() {
    let mut b = built(0.0);
    b.roster.listed[0].merges = true;
    b.field.columns.state[3] = DEAD; // the friend's unit in column 1

    let (joins, logged) = b.run();

    assert_eq!(joins, vec![(1, 0)]);
    let merged = [events::MERGED as i64, TICK, 1, 0, 2];
    let stepped = [events::STEPPED_UP as i64, TICK, 2, 0, 101, 0];
    assert_eq!(logged, [&merged[..], &stepped[..]].concat());
    assert_eq!((b.field.columns.rank[0], b.field.columns.rank[1]), (1, 0));
    assert_eq!(b.squads.members[1], vec![2, 3, 0, 1]); // its fallen stay its members
}

#[test]
fn a_joining_unit_keeps_the_command_it_started_under() {
    let mut b = built(0.0);
    b.squads.state[1] = FIGHTING;
    b.field.columns.objects[0].origin = 9;

    b.run();

    let objects = &b.field.columns.objects[0];
    assert_eq!((objects.command, objects.origin), (COMMAND, 9));
}

#[test]
fn a_wave_short_of_its_friend_or_one_not_merging_with_a_marching_friend_joins_nobody() {
    for (wave_at, fighting) in [(-4.0, true), (0.0, false)] {
        let mut b = built(wave_at);
        if fighting {
            b.squads.state[1] = FIGHTING;
        }

        let (joins, logged) = b.run();

        assert!(joins.is_empty() && logged.is_empty());
        assert_eq!(b.squads.members[0], vec![0, 1]);
        assert_eq!(b.squads.reforming[1], 0);
    }
}

#[test]
fn taking_in_hands_back_the_living_closed_up_and_which_stepped_up() {
    let mut b = built(0.0);
    b.field.columns.state[3] = DEAD;

    let took = take_in(&mut b.squads, &mut b.field, 1, 0);

    assert_eq!(took.closed, vec![2, 0, 1]);
    assert_eq!(took.stepped, vec![2]);
}
