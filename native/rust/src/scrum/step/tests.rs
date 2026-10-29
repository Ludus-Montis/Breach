//! The scrum as one pass: on two squads of grems closing on each other, it leaves the
//! squads, the units and the log as the nine phases called one after another do.

use super::*;
use crate::ground::Walker;
use crate::maths::V2;
use crate::near::Steering;
use crate::pursuit::{self, whole};
use crate::roster::{Listed, Member};
use crate::{regroup as regrouping, withdraw as withdrawing};

const GREM: Walker = Walker { height: 1.0, swimmer: 0, climber: 0, swims: false, climbs: false };

fn rules() -> Rules {
    let discipline = DisciplineTuning {
        meets_threats: 25,
        expected_max: 100.0,
        leash_shares: vec![0.8, 0.5],
        leash_steps: vec![2.0, 4.0, 8.0, 16.0, 32.0],
        ..DisciplineTuning::default()
    };
    let steering = Steering { look: 3.0, clear: 0.1, seed: 7 };
    Rules {
        tick: 4,
        seed: 7,
        reach_engage: 0.5,
        anticipate: 4.0,
        stance: discipline.clone(),
        seek: seek::Rules {
            tick: 4,
            seed: 7,
            die: 20,
            reach: 0.2,
            leash: 6.0,
            seconds: 0.1,
            pace: 0.25,
            crowding: 0.5,
            look: 3.0,
            clear: 0.1,
            drag: 0.5,
            block: 0.9,
            front: 0.5,
        },
        regroup: regroup::Rules {
            regroup: regrouping::Rules {
                pace: 0.25,
                seconds: 0.1,
                seed: 7,
                reach: 0.2,
                arc: 60.0,
                look: 3.0,
                clear: 0.1,
                trade_seconds: 2.0,
            },
            chase: pursuit::Rules { cells_per_second: 2.5, seconds: 0.1, give_up: 0.2, steering },
            discipline: discipline.clone(),
            reform: [60.0, 0.25, 1.5],
            seed: 7,
            wounds_reach: 1.0,
        },
        withdraw: withdraw::Rules {
            flight: withdrawing::Rules {
                pace: 0.25,
                seconds: 0.1,
                reach: 0.2,
                near: 4.0,
                fan_cells: 3.0,
            },
            rally_seconds: 0.3,
            fan_degrees: 30.0,
            discipline: discipline.clone(),
            reform: [60.0, 0.25, 1.5],
            seed: 7,
            tick: 4,
        },
        pursue: pursue::Rules {
            whole: whole::Rules { cells_per_second: 2.5, seconds: 0.1, give_up: 0.2, lag: 2.0 },
            discipline,
            tick: 4,
        },
        stall_ticks: 1,
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
    log: Log,
}

/// Squad 1 (the player's, facing east) and squad 2 (the kingdom's, facing west), two grems
/// abreast each, their fronts `apart` cells from each other across x.
fn built(apart: f32) -> Built {
    let (mut squads, mut field, mut roster) =
        (Squads::default(), Field::default(), Roster::default());
    for (s, (x, heading)) in [(0.0, 90.0), (apart, 270.0)].into_iter().enumerate() {
        let id = s as i64 + 1;
        squads.open(id);
        squads.faction[s] = s as u32;
        squads.width[s] = 2;
        squads.heading[s] = heading;
        squads.position[s] = V2::new(x, 0.5);
        roster.listed.push(Listed { squad: s, id, faction: s as u32, ..Listed::default() });
        for column in 0..2 {
            let unit = 100 * id + column as i64;
            let slot = field.hold(unit) as usize;
            field.columns.column[slot] = column;
            field.columns.speed[slot] = 1.0;
            field.columns.bearing[slot] = heading;
            field.columns.position[slot] = V2::new(x, column as f32);
            squads.members[s].push(slot as i32);
            roster.listed[s].living.push(grem(unit, slot));
        }
    }
    Built { squads, field, roster, log: Log::default() }
}

impl Built {
    /// The nine phases, one call each, as FormationScrum.step made them.
    fn phased(&mut self, rules: &Rules) -> Vec<usize> {
        let (squads, field, roster, log) =
            (&mut self.squads, &mut self.field, &self.roster, &mut self.log);
        let (tick, seed) = (rules.tick, rules.seed);
        pass::engage(squads, &mut field.columns, roster, (tick, seed, rules.reach_engage), log);
        prepare::prepare(squads, field, roster, tick);
        let lines =
            prepare::Lines { anticipate: rules.anticipate, seed, discipline: &rules.stance };
        prepare::anticipate(squads, field, roster, None, (&lines, tick), log);
        pass::manoeuvre(squads, &mut field.columns, roster);
        let active = seek::step(squads, field, roster, None, &rules.seek);
        regroup::step(squads, field, roster, None, &rules.regroup);
        withdraw::step(squads, field, roster, None, &rules.withdraw, log);
        let back = pursue::step(squads, field, roster, &rules.pursue, log);
        pass::stall(squads, roster, (&active, rules.stall_ticks), tick, log);
        back
    }

    fn whole(&mut self, rules: &Rules) -> Vec<usize> {
        let (squads, field, log) = (&mut self.squads, &mut self.field, &mut self.log);
        step(squads, field, &self.roster, None, rules, log).back
    }

    fn places(&self) -> Vec<(V2, f64)> {
        let c = &self.field.columns;
        (0..c.position.len()).map(|n| (c.position[n], c.bearing[n])).collect()
    }
}

#[test]
fn the_one_pass_leaves_the_squads_their_units_and_the_log_as_the_phases_one_by_one() {
    let rules = rules();
    let (mut phased, mut whole) = (built(1.4), built(1.4));
    let back = (phased.phased(&rules), whole.whole(&rules));
    let logs = (phased.log.take(), whole.log.take());
    assert!(!logs.0.is_empty(), "the squads engaged");
    assert_eq!(phased.squads.fight_since, vec![4, 4], "and their fight began");
    assert_eq!(logs.1, logs.0);
    assert_eq!(back.1, back.0);
    assert_eq!(whole.squads, phased.squads);
    assert_eq!(whole.places(), phased.places());
}

#[test]
fn a_second_tick_of_the_one_pass_matches_the_phases_too() {
    let rules = rules();
    let (mut phased, mut whole) = (built(1.4), built(1.4));
    for _ in 0..2 {
        phased.phased(&rules);
        whole.whole(&rules);
    }
    assert_eq!(whole.log.take(), phased.log.take());
    assert_eq!(whole.squads, phased.squads);
    assert_eq!(whole.places(), phased.places());
}
