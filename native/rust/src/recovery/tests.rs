//! FormationRecovery's and FormationRegeneration's scenarios (Decisions 121, 125, 126 and
//! 127), from test_formation_recovery.gd, test_formation_near_passes.gd and
//! test_formation_wounds_order.gd, on squads built here: each unit in the next store slot,
//! at 10 hp of 10, constitution 10, seeing 10 cells; ticks of 0.1 s under seed 3. Faction 0
//! is the player's, 1 the kingdom's.

use super::*;
use crate::content::tests::repo_content;
use crate::content::Content;

const SEED: i64 = 3;

fn tuning() -> WoundsTuning {
    let content = Content::load(&repo_content()).unwrap_or_else(|error| panic!("{error}"));
    WoundsTuning::of(&content.tuning)
}

#[derive(Default)]
struct Built {
    columns: Columns,
    wards: Vec<Ward>,
}

impl Built {
    /// A standing unit at `at` in squad `squad` (made as needed, joinable, id `squad + 1`).
    fn standing(&mut self, squad: usize, id: i64, at: (f32, f32)) -> At {
        let unit = self.add(squad, id, at, 10);
        self.wards[squad].living.push(unit);
        (squad, self.wards[squad].living.len() - 1)
    }

    /// A downed unit, once wounded, at `at` in squad `squad`.
    fn downed(&mut self, squad: usize, id: i64, at: (f32, f32)) -> At {
        let mut unit = self.add(squad, id, at, 0);
        unit.wounded = 1;
        self.columns.wake_left[unit.slot] = -1.0;
        self.wards[squad].downed.push(unit);
        (squad, self.wards[squad].downed.len() - 1)
    }

    fn add(&mut self, squad: usize, id: i64, at: (f32, f32), hp: i32) -> Recovering {
        let slot = self.columns.position.len();
        self.columns.grow();
        self.columns.position[slot] = V2::new(at.0, at.1);
        self.columns.hp[slot] = hp;
        while self.wards.len() <= squad {
            let id = self.wards.len() as i64 + 1;
            self.wards.push(Ward { id, joinable: true, ..Default::default() });
        }
        Recovering::plain(id, slot)
    }

    fn lying(&mut self, at: At) -> &mut Recovering {
        &mut self.wards[at.0].downed[at.1]
    }

    fn up(&mut self, at: At) -> &mut Recovering {
        &mut self.wards[at.0].living[at.1]
    }

    fn hp_of(&self, unit: &Recovering) -> i32 {
        self.columns.hp[unit.slot]
    }

    /// Runs `seconds` in ticks of 0.1; what each tick decided.
    fn run(&mut self, seconds: f64, t: &WoundsTuning) -> Vec<Recovered> {
        let ticks = (seconds / 0.1).round() as usize;
        (0..ticks).map(|_| self.tick(t)).collect()
    }

    /// One tick, then lands what it decided as GDScript does: the dead and those that came to
    /// leave the downed (those joining a squad go to its standing), and those playing dead
    /// are marked so.
    fn tick(&mut self, t: &WoundsTuning) -> Recovered {
        let out = step(&mut self.columns, &self.wards, 0.1, SEED, t);
        let mut leaving: Vec<At> = vec![];
        for lay in &out.lay {
            match *lay {
                Lay::DiedOfWounds(at) => leaving.push(at),
                Lay::PlayingDead(at) => self.lying(at).playing_dead = true,
            }
        }
        let mut joining = vec![];
        for rising in &out.rising {
            leaving.push(rising.unit);
            if let Some(squad) = rising.joins {
                let mut unit = self.wards[rising.unit.0].downed[rising.unit.1].clone();
                unit.playing_dead = false;
                joining.push((squad, unit));
            }
        }
        leaving.sort();
        for at in leaving.into_iter().rev() {
            self.wards[at.0].downed.remove(at.1);
        }
        for (squad, unit) in joining {
            self.wards[squad].living.push(unit);
        }
        out
    }
}

#[test]
fn each_wound_lowers_its_condition_past_those_it_is_hardened_to() {
    let t = tuning();
    let mut unit = Recovering::plain(1, 0);
    unit.wounded = 1;
    assert!((condition_of(&unit, &t) - (1.0 - t.condition)).abs() < 0.0001);
    unit.hardened = 1;
    assert_eq!(condition_of(&unit, &t), 1.0);
}

#[test]
fn a_poor_condition_drains_it_to_death_the_faster_the_worse() {
    let t = tuning();
    let mut b = Built::default();
    let three = b.downed(0, 1, (0.0, 0.0));
    b.lying(three).wounded = 3; // just below the threshold with the usual tuning
    let four = b.downed(0, 2, (5.0, 0.0));
    b.lying(four).wounded = 4;
    let two = b.downed(0, 3, (10.0, 0.0));
    b.lying(two).wounded = 2; // survivable in otherwise good condition
    let units: Vec<Recovering> = b.wards[0].downed.clone();
    b.run(5.0, &t);
    assert!(b.hp_of(&units[1]) < b.hp_of(&units[0]));
    assert_eq!(b.hp_of(&units[2]), 0);
    let died =
        b.run(25.0, &t).iter().any(|r| r.lay.iter().any(|l| matches!(l, Lay::DiedOfWounds(_))));
    assert!(died);
    assert!(!b.wards[0].downed.iter().any(|u| u.id == 1)); // the one with three wounds is dead
}

#[test]
fn downed_it_comes_to_at_1_hp_and_rejoins_its_formation_in_sight() {
    let t = tuning();
    let mut b = Built::default();
    b.standing(0, 2, (3.0, 0.0));
    let downed = b.downed(0, 1, (0.0, 0.0));
    let unit = b.lying(downed).clone();
    let risen: Vec<Rising> = b.run(60.0, &t).into_iter().flat_map(|r| r.rising).collect();
    assert_eq!(risen, vec![Rising { unit: downed, joins: Some(0) }]);
    assert_eq!(b.hp_of(&unit), 1);
}

#[test]
fn with_no_formation_in_sight_it_makes_for_home_alone() {
    let t = tuning();
    let mut b = Built::default();
    let downed = b.downed(0, 1, (0.0, 0.0));
    let risen: Vec<Rising> = b.run(60.0, &t).into_iter().flat_map(|r| r.rising).collect();
    assert_eq!(risen, vec![Rising { unit: downed, joins: None }]);
}

#[test]
fn it_joins_the_nearest_friendly_formation_not_its_own_nor_a_routed_one() {
    let t = tuning();
    let mut b = Built::default();
    b.standing(0, 2, (30.0, 0.0)); // its own, out of sight
    let downed = b.downed(0, 1, (0.0, 0.0));
    b.standing(1, 5, (2.0, 0.0));
    b.standing(2, 6, (1.0, 0.0)); // nearer, but routing
    b.wards[2].joinable = false;
    let risen: Vec<Rising> = b.run(60.0, &t).into_iter().flat_map(|r| r.rising).collect();
    assert_eq!(risen, vec![Rising { unit: downed, joins: Some(1) }]);
}

/// Scattered standing units of three squads and one coming to: it joins the squad of the
/// nearest it can see, as a look at every unit finds it.
#[test]
fn one_coming_to_joins_the_formation_a_look_at_all_finds() {
    let t = tuning();
    let mut b = Built::default();
    let downed = b.downed(0, 1, (20.0, 20.0));
    b.lying(downed).clone().wakes_now(&mut b.columns);
    let mut seed: u32 = 7;
    let mut next = || {
        seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
        (seed >> 8) as f32 / (1 << 24) as f32 * 40.0
    };
    for id in 0..90 {
        let squad = 1 + (id % 3) as usize;
        b.standing(squad, 100 + id, (next(), next()));
    }
    let unit = b.lying(downed).clone();
    let here = b.columns.position[unit.slot];
    let mut best: Option<(f64, i64, usize)> = None;
    for (s, ward) in b.wards.iter().enumerate() {
        for other in &ward.living {
            let gap = b.columns.position[other.slot].distance_to(here) as f64;
            let key = (snapped(gap, 0.000001), contest::squad_draw(SEED, ward.id), s);
            if gap <= unit.detection && best.is_none_or(|k| key < k) {
                best = Some(key);
            }
        }
    }
    let out = b.tick(&t);
    assert_eq!(out.rising, vec![Rising { unit: downed, joins: best.map(|k| k.2) }]);
}

impl Recovering {
    /// Puts it due to come to this tick.
    fn wakes_now(&self, columns: &mut Columns) {
        columns.wake_left[self.slot] = 0.05;
    }
}

#[test]
fn regeneration_mends_to_its_max_and_no_further_than_its_limit() {
    let t = tuning();
    let mut b = Built::default();
    let troll = b.standing(0, 1, (0.0, 0.0));
    b.up(troll).regeneration = 2.0;
    let unit = b.up(troll).clone();
    b.columns.hp[unit.slot] = 2;
    b.columns.regeneration_left[unit.slot] = 5.0;
    b.run(3.0, &t);
    assert_eq!(b.hp_of(&unit), 7); // 5 regained: its limit until it rests
    b.columns.regeneration_left[unit.slot] = 100.0;
    b.run(7.0, &t);
    assert_eq!(b.hp_of(&unit), 10); // never past its max
}

#[test]
fn a_halted_regeneration_mends_nothing_till_the_halt_runs_out() {
    let t = tuning();
    let mut b = Built::default();
    let troll = b.standing(0, 1, (0.0, 0.0));
    b.up(troll).regeneration = 10.0;
    let unit = b.up(troll).clone();
    b.columns.hp[unit.slot] = 2;
    b.columns.regeneration_left[unit.slot] = 100.0;
    b.columns.regeneration_halt[unit.slot] = 1.05; // a blow of a type it fears
    b.run(1.0, &t);
    assert_eq!(b.hp_of(&unit), 2);
    b.run(0.5, &t);
    assert!(b.hp_of(&unit) > 2);
}

#[test]
fn a_downed_regenerator_mends_only_with_the_trait() {
    let t = tuning();
    let mut b = Built::default();
    let plain = b.downed(0, 1, (0.0, 0.0));
    let troll = b.downed(0, 2, (1.0, 0.0));
    for at in [plain, troll] {
        b.lying(at).regeneration = 10.0;
        let slot = b.lying(at).slot;
        b.columns.regeneration_left[slot] = 100.0;
    }
    b.lying(troll).regenerates_downed = true;
    let units: Vec<Recovering> = b.wards[0].downed.clone();
    b.run(0.5, &t);
    assert_eq!(b.hp_of(&units[0]), 0);
    assert!(b.hp_of(&units[1]) > 0);
}

/// `count` downed player units due to come to, a kingdom foe standing two cells off each.
fn due_with_foes(count: i64, cunning: i64) -> Built {
    let mut b = Built::default();
    for index in 0..count {
        let x = index as f32 * 20.0;
        let at = b.downed(0, index + 1, (x, 0.0));
        b.lying(at).cunning = cunning;
        b.lying(at).clone().wakes_now(&mut b.columns);
        let foe = b.standing(1, 100 + index, (x + 2.0, 0.0));
        b.up(foe).faction = 1;
    }
    b
}

fn playing_dead(b: &Built) -> usize {
    b.wards[0].downed.iter().filter(|u| u.playing_dead).count()
}

#[test]
fn with_a_foe_near_some_play_dead_and_the_cunning_more_often() {
    let t = tuning();
    let mut plain = due_with_foes(40, 0);
    let mut cunning = due_with_foes(40, 2);
    let rose = plain.tick(&t).rising.len();
    cunning.tick(&t);
    assert!((4..=36).contains(&playing_dead(&plain)));
    assert!(playing_dead(&cunning) > playing_dead(&plain));
    assert_eq!(rose + playing_dead(&plain), 40); // the rest get up into the fight
}

#[test]
fn one_playing_dead_gets_up_once_its_foes_are_gone() {
    let t = tuning();
    let mut b = due_with_foes(40, 5);
    b.tick(&t);
    let faker = b.wards[0].downed.iter().find(|u| u.playing_dead).unwrap().id;
    b.run(10.0, &t);
    assert!(b.wards[0].downed.iter().any(|u| u.id == faker)); // its foe still stands by
    for foe in b.wards[1].living.clone() {
        b.columns.position[foe.slot].y += 50.0;
    }
    assert!(b.run(5.0, &t).iter().any(|r| !r.rising.is_empty()));
    assert!(!b.wards[0].downed.iter().any(|u| u.id == faker));
}

/// Two downed friends behind a standing one, both coming to this tick, listed as given.
fn two_rising(reversed: bool) -> Built {
    let mut b = Built::default();
    b.standing(0, 9, (3.0, 0.0));
    let mut ids = vec![(1, (0.0, 1.0)), (2, (0.0, -1.0))];
    if reversed {
        ids.reverse();
    }
    for (id, at) in ids {
        let downed = b.downed(0, id, at);
        b.lying(downed).clone().wakes_now(&mut b.columns);
    }
    b
}

#[test]
fn two_who_come_to_together_rise_in_the_same_order_whichever_is_listed_first() {
    let t = tuning();
    let ids = |b: &mut Built| {
        let wards = b.wards.clone();
        let out = b.tick(&t);
        out.rising
            .iter()
            .map(|r| (wards[r.unit.0].downed[r.unit.1].id, r.joins))
            .collect::<Vec<_>>()
    };
    let forward = ids(&mut two_rising(false));
    let reversed = ids(&mut two_rising(true));
    assert_eq!(forward.len(), 2);
    assert_eq!(forward, reversed);
}
