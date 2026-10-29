//! The scrum's regroup and chasers scenarios: a player squad (id 1) of one grem at its place,
//! facing east from the origin, and a kingdom squad (id 2) of one grem standing at `foe`.

use super::*;
use crate::entries;
use crate::frame::Frame as Kept;
use crate::ground::Walker;
use crate::locks::{self, HOLDING};
use crate::near::Steering;
use crate::roster::Member;
use crate::squads::Chaser;

const GREM: Walker = Walker { height: 1.0, swimmer: 0, climber: 0, swims: false, climbs: false };

fn rules() -> Rules {
    Rules {
        regroup: regroup::Rules {
            pace: 0.25,
            seconds: 0.1,
            seed: 7,
            reach: 0.2,
            arc: 60.0,
            look: 3.0,
            clear: 0.1,
            trade_seconds: 2.0,
        },
        chase: pursuit::Rules {
            cells_per_second: 2.5,
            seconds: 0.1,
            give_up: 0.2,
            steering: Steering { look: 3.0, clear: 0.1, seed: 7 },
        },
        discipline: DisciplineTuning { meets_threats: 25, ..DisciplineTuning::default() },
        reform: [60.0, 0.25, 1.5],
        seed: 7,
        wounds_reach: 1.0,
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

struct Built {
    squads: Squads,
    field: Field,
    roster: Roster,
}

fn built(foe: V2) -> Built {
    let (mut squads, mut field, mut roster) =
        (Squads::default(), Field::default(), Roster::default());
    for s in 0..2 {
        let id = s as i64 + 1;
        squads.open(id);
        squads.width[s] = 1;
        squads.faction[s] = s as u32;
        let unit = 100 * id;
        let slot = field.hold(unit) as usize;
        field.columns.speed[slot] = 1.0;
        field.columns.bearing[slot] = if s == 0 { 90.0 } else { 270.0 };
        field.columns.position[slot] =
            if s == 0 { Kept::kept(&squads, 0).place((0, 0), (1, 1)) } else { foe };
        squads.members[s].push(slot as i32);
        let living = vec![grem(unit, slot)];
        roster.listed.push(Listed { squad: s, id, faction: s as u32, living, ..Listed::default() });
    }
    squads.state[0] = HOLDING;
    squads.state[1] = locks::FIGHTING;
    Built { squads, field, roster }
}

impl Built {
    /// The player's grem goes loose two cells south of its place.
    fn loosen(&mut self) -> V2 {
        let at = self.field.columns.position[0].add(V2::new(0.0, 2.0));
        entries::loosen(&mut self.squads, &mut self.field.columns, 0, (100, 0), at);
        at
    }

    fn step(&mut self) {
        step(&mut self.squads, &mut self.field, &self.roster, None, &rules());
    }
}

#[test]
fn a_squad_out_of_the_fight_walks_its_loose_units_back_to_their_places() {
    let mut b = built(V2::new(20.0, 0.0));
    let home = b.field.columns.position[0];
    b.loosen();
    b.step();
    assert!(b.field.columns.loose_at[0].distance_to(home) < 2.0);
}

#[test]
fn a_fighting_squads_loose_units_are_not_walked_back() {
    let mut b = built(V2::new(20.0, 0.0));
    b.squads.state[0] = locks::FIGHTING;
    let at = b.loosen();
    b.step();
    assert_eq!(b.field.columns.loose_at[0], at);
}

#[test]
fn a_chaser_walks_after_its_quarry() {
    let foe = V2::new(4.0, 0.5);
    let mut b = built(foe);
    let at = b.loosen();
    b.squads.chase(0, Chaser { unit: 100, foe: 2, from: at, leash: 10.0, returning: false });
    b.step();
    assert!(b.field.columns.loose_at[0].distance_to(foe) < at.distance_to(foe));
}

#[test]
fn a_disciplined_retreat_with_units_out_regroups_drilled_at_its_reform_pace() {
    let mut b = built(V2::new(20.0, 0.0));
    b.squads.order[0] = RETREAT;
    b.loosen();
    let r = regrouper(&b.squads, &b.field, &b.roster, 0, None, &rules());
    assert_eq!((r.drilled, r.reform), (true, 0.5)); // discipline 30 of 60
}
