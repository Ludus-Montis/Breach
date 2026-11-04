//! The scrum's preparing and lines' scenarios: squads of two grems abreast built here, each
//! unit standing in its front rank.

use super::*;
use crate::ground::Walker;
use crate::roster::Member;

const EAST: f64 = 90.0;
const NORTH: f64 = 0.0;
const TICK: i64 = 6;

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

/// A squad per `fronts` entry (front, heading, faction), ids from 1, each two grems (ids
/// 100 s + column) standing on its front at columns 0 and 1; those `dead` are in the store
/// but not listed living.
fn built(fronts: &[(V2, f64, u32)], dead: &[i64]) -> Built {
    let (mut squads, mut field, mut roster) =
        (Squads::default(), Field::default(), Roster::default());
    for (s, &(at, heading, faction)) in fronts.iter().enumerate() {
        squads.open(s as i64 + 1);
        squads.position[s] = at;
        squads.heading[s] = heading;
        squads.width[s] = 2;
        squads.faction[s] = faction;
        let mut living = Vec::new();
        for column in 0..2 {
            let id = 100 * (s as i64 + 1) + column;
            let slot = field.hold(id) as usize;
            field.columns.column[slot] = column as i32;
            let across = bearing_vector(heading + 90.0).scale(column as f64 - 0.5);
            field.columns.position[slot] = at.add(across);
            squads.members[s].push(slot as i32);
            if dead.contains(&id) {
                field.columns.state[slot] = DEAD;
            } else {
                living.push(grem(id, slot));
            }
        }
        let listed = Listed { squad: s, id: s as i64 + 1, faction, living, ..Listed::default() };
        roster.listed.push(listed);
    }
    Built { squads, field, roster }
}

impl Built {
    fn prepare(&mut self) {
        prepare(&mut self.squads, &mut self.field, &self.roster, TICK);
    }

    /// The lines with every unit's discipline `discipline` against 25 to meet threats; the
    /// events they logged.
    fn anticipate(&mut self, discipline: i64) -> Vec<i64> {
        for slot in 0..self.field.columns.sheet.len() {
            self.field.columns.sheet[slot].discipline = discipline;
        }
        let tuning = DisciplineTuning { meets_threats: 25, ..DisciplineTuning::default() };
        let lines = Lines { anticipate: 6.0, seed: 3, discipline: &tuning };
        let mut log = Log::default();
        let (squads, field) = (&mut self.squads, &self.field);
        anticipate(squads, field, &self.roster, None, (&lines, TICK), &mut log);
        log.take()
    }

    fn loose_at(&self, id: i64) -> V2 {
        self.field.columns.loose_at[self.field.slot_of(id).unwrap() as usize]
    }
}

#[test]
fn a_routing_squad_lets_its_loose_units_its_stance_and_its_fight_go() {
    let mut b = built(&[(V2::new(0.0, 0.0), EAST, 0)], &[]);
    b.squads.state[0] = ROUTING;
    b.squads.fight_since[0] = 2;
    b.squads.stance[0] = Some(Stance { anchor: V2::ZERO, heading: 0.0, foe: 9 });
    entries::loosen(&mut b.squads, &mut b.field.columns, 0, (100, 0), V2::ZERO);
    b.prepare();
    assert!(b.squads.loose[0].is_empty());
    assert_eq!(b.field.columns.flags[0] & entries::LOOSE, 0);
    assert_eq!((b.squads.stance[0], b.squads.fight_since[0]), (None, -1));
}

#[test]
fn a_fight_beginning_is_marked_the_fallen_unloosed_and_the_living_sent_loose_where_they_stand() {
    let mut b = built(&[(V2::new(0.0, 0.0), EAST, 0)], &[101]);
    b.squads.state[0] = FIGHTING;
    b.squads.stall_ticks[0] = 4;
    entries::loosen(&mut b.squads, &mut b.field.columns, 0, (101, 1), V2::ZERO);
    b.prepare();
    assert_eq!(b.squads.loose[0], vec![100]);
    assert_eq!(b.loose_at(100), b.field.columns.position[0]);
    assert_eq!(b.field.columns.flags[1] & entries::LOOSE, 0);
    let marked = (b.squads.fight_since[0], b.squads.fought[0], b.squads.stall_ticks[0]);
    assert_eq!(marked, (TICK, 1, 0));
}

#[test]
fn a_fight_ending_closes_the_ranks_over_the_dead_and_sends_nobody_loose() {
    let mut b = built(&[(V2::new(0.0, 0.0), EAST, 0)], &[100]);
    b.squads.state[0] = HOLDING;
    b.squads.fight_since[0] = 2;
    b.prepare();
    assert_eq!(b.field.columns.column[1], 0); // the living one closes up to the middle
    assert_eq!(b.squads.fight_since[0], -1);
    assert!(b.squads.loose[0].is_empty() && b.squads.swaps[0].is_empty());
}

#[test]
fn a_squad_holding_a_stance_sends_its_units_loose_out_of_a_fight() {
    let mut b = built(&[(V2::new(0.0, 0.0), EAST, 0)], &[]);
    b.squads.state[0] = HOLDING;
    b.squads.stance[0] = Some(Stance { anchor: V2::ZERO, heading: 0.0, foe: 9 });
    b.prepare();
    assert_eq!(b.squads.loose[0], vec![100, 101]);
}

#[test]
fn a_disciplined_squad_takes_a_line_facing_an_enemy_closing_on_its_flank_and_reports_it() {
    // squad 1 holds facing east; squad 2 marches north up at it from the south
    let fronts = [(V2::new(0.0, 0.0), EAST, 0), (V2::new(0.0, 5.0), NORTH, 1)];
    let mut b = built(&fronts, &[]);
    b.squads.state[0] = HOLDING;
    let logged = b.anticipate(30);
    let south: f64 = 180.0;
    let stance = Stance { anchor: V2::new(-0.5, 1.0), heading: south, foe: 2 };
    assert_eq!(b.squads.stance, vec![Some(stance), None]);
    assert_eq!(logged, vec![events::FACED as i64, TICK, 1, 0, south.to_bits() as i64, 2]);
}

#[test]
fn a_squad_short_of_the_discipline_meets_the_threat_unit_by_unit() {
    let fronts = [(V2::new(0.0, 0.0), EAST, 0), (V2::new(0.0, 5.0), NORTH, 1)];
    let mut b = built(&fronts, &[]);
    b.squads.state[0] = HOLDING;
    b.squads.stance[0] = Some(Stance { anchor: V2::ZERO, heading: 0.0, foe: 2 });
    assert!(b.anticipate(20).is_empty());
    assert_eq!(b.squads.stance, vec![None, None]); // a holding line it can't keep
}
