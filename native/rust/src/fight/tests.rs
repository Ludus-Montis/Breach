//! The fight's choice scenarios: squad 1 of two grems abreast on x = 0 facing east, squad 2
//! of two on x = `gap` facing west, built here, each loosened where it stands (`prepare`).

use super::*;
use crate::ground::Walker;
use crate::locks::{HOLDING, MOVING};
use crate::maths::{bearing_vector, V2};
use crate::roster::{Listed, Member};
use crate::scrum::prepare::prepare;

const EAST: f64 = 90.0;
const WEST: f64 = 270.0;

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
        interval: 4,
        seed: 3,
        reach: 1.5,
        front: 0.0,
        rout_reach: 2.0,
        bands: BandTuning {
            per_leadership: 5,
            steady_at: 70,
            shaken_at: 40,
            loss: 2,
            leader_loss: 3,
        },
    }
}

struct Built {
    squads: Squads,
    field: Field,
    roster: Roster,
}

/// The two squads, both in `state` and locked on each other, every unit's cooldown 1 (it
/// strikes this tick), its melee damage 7 and its ranged damage 5, morale 80 (steady).
fn built(gap: f32, state: i32) -> Built {
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
        squads.morale[s] = 80;
        squads.engaged_with[s] = 2 - s as i64;
        let mut living = Vec::new();
        for column in 0..2 {
            let id = 100 * (s as i64 + 1) + column;
            let slot = field.hold(id) as usize;
            let columns = &mut field.columns;
            columns.column[slot] = column as i32;
            let across = bearing_vector(heading + 90.0).scale(column as f64 - 0.5);
            columns.position[slot] = at.add(across);
            columns.bearing[slot] = heading;
            columns.attack_cooldown[slot] = 1;
            columns.sheet[slot].dmg = 7;
            columns.sheet[slot].ranged_dmg = 5;
            columns.sheet[slot].melee_seconds = 1.5;
            columns.target_id[slot] = 42;
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
    fn choose(&mut self) -> Chosen {
        choose(&mut self.squads, &mut self.field, &self.roster, &rules())
    }

    fn slot(&self, id: i64) -> usize {
        self.field.slot_of(id).unwrap() as usize
    }

    fn column(&self, id: i64) -> (i64, i32) {
        let slot = self.slot(id);
        (self.field.columns.target_id[slot], self.field.columns.attack_cooldown[slot])
    }
}

#[test]
fn with_no_foe_in_reach_every_target_is_cleared_and_none_strikes() {
    let mut b = built(9.0, HOLDING);
    let chosen = b.choose();
    assert_eq!(chosen, Chosen::default());
    assert_eq!([b.column(100), b.column(201)], [(0, 1), (0, 1)]);
}

#[test]
fn fighting_units_strike_the_foes_they_touch_and_start_again_at_their_melee_time() {
    let mut b = built(1.0, FIGHTING);
    let chosen = b.choose();
    assert!(chosen.synced);
    let strikers: Vec<Who> = chosen.melee.iter().map(|blow| blow.striker).collect();
    assert_eq!(strikers, vec![(0, 0), (0, 1), (1, 0), (1, 1)]);
    let blow = chosen.melee[0];
    assert_eq!((blow.target.0, blow.damage, blow.flank), (1, 7, false));
    let (target, cooldown) = b.column(100);
    assert_eq!((target / 100, cooldown), (2, 6)); // 4 ticks a second, 1.5 s: 6 ticks
    assert!(chosen.shots.is_empty());
}

#[test]
fn a_wavering_squad_starts_again_half_again_more_slowly() {
    let mut b = built(1.0, FIGHTING);
    b.squads.morale[0] = 20;
    b.choose();
    assert_eq!([b.column(100).1, b.column(200).1], [9, 6]);
}

#[test]
fn a_squad_with_no_morale_yet_reads_its_band_from_its_ceiling() {
    let mut b = built(1.0, FIGHTING);
    b.squads.morale[0] = -1;
    b.choose();
    assert_eq!(b.squads.morale[0], 60); // the grems' courage, 60
}

#[test]
fn a_unit_still_cooling_down_takes_its_target_but_does_not_strike() {
    let mut b = built(1.0, FIGHTING);
    let slot = b.slot(100);
    b.field.columns.attack_cooldown[slot] = 3;
    let chosen = b.choose();
    assert!(chosen.melee.iter().all(|blow| blow.striker != (0, 0)));
    assert_eq!((b.column(100).0 / 100, b.column(100).1), (2, 2));
}

#[test]
fn a_squad_retreating_with_its_back_to_its_foe_strikes_no_blow() {
    let mut b = built(1.0, FIGHTING);
    b.squads.order[0] = RETREAT;
    for id in [100, 101] {
        let slot = b.slot(id);
        b.field.columns.bearing[slot] = WEST;
    }
    let chosen = b.choose();
    let strikers: Vec<(Who, bool)> = chosen.melee.iter().map(|b| (b.striker, b.flank)).collect();
    assert_eq!(strikers, vec![((1, 0), true), ((1, 1), true)]); // struck from behind
}

#[test]
fn hostiles_free_to_fight_strike_routers_they_reach_from_behind() {
    let mut b = built(1.0, MOVING);
    b.squads.state[1] = ROUTING;
    for id in [200, 201] {
        let slot = b.slot(id);
        let at = b.field.columns.position[slot];
        b.field.columns.along[slot] = at.x as f64;
        b.field.columns.flee_offset[slot] = V2::new(0.0, at.y);
    }
    let chosen = b.choose();
    assert!(!chosen.synced); // a routing squad is struck by none in the scrum
    let blows: Vec<(Who, bool)> = chosen.melee.iter().map(|b| (b.striker, b.flank)).collect();
    assert_eq!(blows, vec![((0, 0), true), ((0, 1), true)]);
    assert!(chosen.melee.iter().all(|blow| blow.target.0 == 1 && blow.damage == 7));
}

#[test]
fn a_ranged_unit_shoots_the_nearest_enemy_in_its_range_for_its_ranged_damage() {
    let mut b = built(3.0, MOVING);
    b.roster.listed[0].living[0].attack_range = 4;
    let slot = b.slot(100);
    b.field.columns.sheet[slot].attack_range = 4;
    let chosen = b.choose();
    assert!(chosen.melee.is_empty());
    let shots: Vec<(Who, usize, i64)> =
        chosen.shots.iter().map(|s| (s.striker, s.target.0, s.damage)).collect();
    assert_eq!(shots, vec![((0, 0), 1, 5)]);
    assert_eq!(b.column(100).1, 4); // its ranged time, 1 s: 4 ticks
}
