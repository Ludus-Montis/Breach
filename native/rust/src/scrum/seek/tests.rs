//! The scrum's seek, walk and facing scenarios: two squads of two grems abreast built here,
//! each loosened where it stands (`prepare`), facing each other across `gap` cells.

use super::*;
use crate::locks::{HOLDING, MOVING};
use crate::maths::bearing_vector;
use crate::roster::Listed;
use crate::scrum::prepare::prepare;

const EAST: f64 = 90.0;
const WEST: f64 = 270.0;
const NORTH: f64 = 0.0;

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

fn rules() -> Rules {
    Rules {
        tick: 6,
        seed: 3,
        die: 6,
        reach: 1.5,
        leash: 8.0,
        seconds: 0.5,
        pace: 2.0,
        crowding: 0.5,
        look: 1.0,
        clear: 0.25,
        drag: 0.5,
        block: 2.0,
        front: 0.0,
    }
}

struct Built {
    squads: Squads,
    field: Field,
    roster: Roster,
}

/// Squad 1 on x = 0 facing east and squad 2 on x = `gap` facing west, each two grems (ids
/// 100 s + column) abreast, every unit facing `bearing`; both in `state`, locked on each
/// other, and readied for the scrum (loose where they stand while fighting).
fn built(gap: f32, state: i32, bearing: f64) -> Built {
    let (mut squads, mut field, mut roster) =
        (Squads::default(), Field::default(), Roster::default());
    for (s, (x, heading)) in [(0.0, EAST), (gap, WEST)].into_iter().enumerate() {
        let at = V2::new(x, 0.0);
        squads.open(s as i64 + 1);
        squads.position[s] = at;
        squads.heading[s] = heading;
        squads.width[s] = 2;
        squads.faction[s] = s as u32;
        squads.state[s] = state;
        squads.engaged_with[s] = 2 - s as i64;
        let mut living = Vec::new();
        for column in 0..2 {
            let id = 100 * (s as i64 + 1) + column;
            let slot = field.hold(id) as usize;
            field.columns.column[slot] = column as i32;
            let across = bearing_vector(heading + 90.0).scale(column as f64 - 0.5);
            field.columns.position[slot] = at.add(across);
            field.columns.bearing[slot] = bearing;
            field.columns.speed[slot] = 1.0;
            squads.members[s].push(slot as i32);
            living.push(grem(id, slot));
        }
        let faction = s as u32;
        let listed = Listed { squad: s, id: s as i64 + 1, faction, living, ..Listed::default() };
        roster.listed.push(listed);
    }
    prepare(&mut squads, &mut field, &roster, 6);
    Built { squads, field, roster }
}

impl Built {
    fn step(&mut self) -> Vec<i64> {
        step(&self.squads, &mut self.field, &self.roster, None, &rules())
    }

    fn slot(&self, id: i64) -> usize {
        self.field.slot_of(id).unwrap() as usize
    }
}

#[test]
fn fighting_squads_units_make_for_slots_beside_their_foes_and_step_towards_them() {
    let mut b = built(6.0, FIGHTING, EAST);
    let active = b.step();
    let slot = b.slot(100);
    assert_eq!(b.field.columns.goal_foe[slot] / 100, 2); // a foe unit of squad 2
    assert!(b.field.columns.loose_at[slot].x > 0.0); // a step towards it
    assert_eq!(active, vec![1, 2]);
}

#[test]
fn a_squad_ordered_to_retreat_seeks_no_slot() {
    let mut b = built(6.0, FIGHTING, EAST);
    b.squads.order[0] = locks::RETREAT;
    let active = b.step();
    let goals: Vec<i64> =
        [100, 101].iter().map(|&id| b.field.columns.goal_slot[b.slot(id)]).collect();
    assert_eq!(goals, vec![-1, -1]);
    assert_eq!(active, vec![2]);
}

#[test]
fn a_unit_touching_a_foe_turns_towards_it() {
    let mut b = built(1.0, FIGHTING, NORTH);
    b.step();
    let bearing = b.field.columns.bearing[b.slot(100)];
    assert!(bearing > NORTH && bearing <= EAST, "{bearing}");
}

#[test]
fn squads_out_of_a_fight_are_not_walked() {
    let mut b = built(6.0, HOLDING, EAST);
    b.squads.state[1] = MOVING;
    let stood: Vec<V2> = b.field.columns.loose_at.clone();
    assert!(b.step().is_empty());
    assert_eq!(b.field.columns.loose_at, stood);
}
