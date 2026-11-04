//! The groups as one pass (spec 30 round 3, part 4), from test_formation_groups.gd, on lines of
//! grems built here: each line facing east, its units in rank 0 a cell apart across, its front
//! `front` cells along x, the player's, after a fight, its unit k with id 100 × its id + k.

use super::*;
use crate::ground::Walker;
use crate::maths::V2;
use crate::roster::Member;
use crate::route::Route;

const GREM: Walker = Walker { height: 1.0, swimmer: 0, climber: 0, swims: false, climbs: false };
const TICK: i64 = 4;
const SEED: i64 = 7;

#[derive(Default)]
struct Built {
    squads: Squads,
    field: Field,
    roster: Roster,
}

impl Built {
    /// A line of `count` holding with its front at x = `front`, of its own command.
    fn line(&mut self, id: i64, count: usize, front: f32) -> usize {
        let s = self.squads.open(id) as usize;
        self.squads.position[s] = V2::new(front, 0.0);
        self.squads.heading[s] = 90.0;
        self.squads.width[s] = count as i32;
        self.squads.direction[s] = 1;
        self.squads.state[s] = HOLDING;
        self.squads.order[s] = 1; // HOLD
        self.squads.fought[s] = 1;
        self.squads.faction[s] = self.field.lexicon.code("player");
        let command = self.squads.commands.open(0.0);
        self.squads.command[s] = Some(command);
        let mut living = vec![];
        for k in 0..count {
            let unit = 100 * id + k as i64;
            let slot = self.field.hold(unit) as usize;
            let columns = &mut self.field.columns;
            columns.position[slot] = V2::new(front - 0.5, k as f32 - (count as f32 - 1.0) / 2.0);
            columns.column[slot] = k as i32;
            columns.sheet[slot].squad_id = id;
            columns.objects[slot].command = command as i64;
            columns.objects[slot].origin = command as i64;
            self.squads.members[s].push(slot as i32);
            living.push(Member {
                id: unit,
                slot,
                foot: (1, 1),
                preferred: 0,
                attack_range: 0,
                kind: 0,
                turn_rate: 360.0,
                backward_pace: 0.5,
                detection: 8.0,
                max_stamina: 100.0,
                walker: GREM,
            });
        }
        let faction = self.squads.faction[s];
        self.roster.listed.push(Listed { squad: s, id, faction, living, ..Listed::default() });
        self.squads.route_code[s] = 0;
        s
    }

    fn slot(&mut self, unit: i64) -> usize {
        self.field.hold(unit) as usize
    }

    /// The pass on walk_lost and group_join of 2: what it did and what it logged.
    fn step(&mut self) -> (Vec<Landed>, Vec<i64>) {
        let rules = Rules { lost: 2.0, join: 2.0, seed: SEED, cells_per_tile: 64.0 };
        let mut log = Log::default();
        let (squads, field) = (&mut self.squads, &mut self.field);
        let out = step(squads, field, &self.roster, &rules, TICK, &mut log);
        (out, log.take())
    }

    fn event(&self, kind: usize, s: usize, values: &[i64]) -> Vec<i64> {
        let head = [kind as i64, TICK, self.squads.id[s], i64::from(self.squads.faction[s])];
        [&head[..], values].concat()
    }
}

#[test]
fn units_fallen_far_behind_go_on_as_a_group_the_crate_opens_with_the_next_id() {
    let mut b = Built::default();
    let line = b.line(1, 4, 40.0);
    b.squads.state[line] = MOVING;
    b.squads.route[line] = Some(Route::new(vec![V2::ZERO, V2::new(100.0, 0.0)], 0.0));
    b.squads.squads_issued = 20;
    b.squads.route_code[line] = 5;
    let (behind, further) = (b.slot(102), b.slot(103));
    b.field.columns.position[behind] = V2::new(20.0, 0.5);
    b.field.columns.position[further] = V2::new(21.0, 1.5);
    let (out, log) = b.step();
    // facing east, across runs north: the unit further south (larger y) lines up first
    assert_eq!(out, vec![Landed::Split { from: 0, slot: 1, members: vec![3, 2] }]);
    assert_eq!(log, b.event(events::SPLIT, 1, &[1]));
    let stayed = vec![b.slot(100) as i32, b.slot(101) as i32];
    let s = &b.squads;
    assert_eq!((s.id[1], s.command[1], s.state[1]), (21, s.command[0], MOVING));
    assert_eq!((s.front_distance[1], s.width[1]), (21.0 / 64.0, 2));
    assert_eq!(s.members[1], vec![further as i32, behind as i32]);
    assert_eq!(s.members[0], stayed);
    assert_eq!(s.route_code, vec![5, 5], "the group holds its squad's route and its code");
    let columns = &b.field.columns;
    let place = |u: usize| (columns.rank[u], columns.column[u], columns.sheet[u].squad_id);
    assert_eq!((place(further), place(behind)), ((0, 0, 21), (0, 1, 21)));
}

#[test]
fn one_commands_groups_meeting_are_one_group_again_the_larger_taking_in() {
    let mut b = Built::default();
    let whole = b.line(1, 4, 40.0);
    let part = b.line(2, 2, 38.5);
    b.squads.command[part] = b.squads.command[whole]; // split off before
    let (out, log) = b.step();
    assert_eq!(out, vec![Landed::Taken { taker: 0, taken: 1 }]);
    assert_eq!(log, b.event(events::MERGED, part, &[1]));
    let s = &b.squads;
    assert_eq!((s.members[whole].len(), s.members[part].len()), (6, 0));
    assert_eq!((s.joined[whole].clone(), s.reforming[whole]), (vec![200, 201], 1));
}

#[test]
fn two_led_groups_keep_their_own_and_units_go_back_where_they_started() {
    let mut b = Built::default();
    let first = b.line(1, 3, 40.0);
    let second = b.line(2, 3, 38.5);
    for unit in [100, 200] {
        let slot = b.slot(unit);
        b.field.columns.sheet[slot].leadership = 1;
    }
    let home = b.squads.command[second].unwrap() as i64;
    let going = b.slot(102);
    b.field.columns.objects[going].origin = home; // it started under the second's command
    let (out, log) = b.step();
    assert_eq!(out, vec![Landed::Returned { from: 0, home: 1, units: vec![102] }]);
    assert_eq!(log, b.event(events::REJOINED, second, &[102]));
    let s = &b.squads;
    assert_eq!((s.members[first].len(), s.members[second].last()), (2, Some(&(going as i32))));
    let columns = &b.field.columns;
    let enlisted =
        (columns.rank[going], columns.sheet[going].squad_id, columns.objects[going].command);
    assert_eq!(enlisted, (1, 2, home));
    assert_eq!(s.reforming[second], 1);
}
