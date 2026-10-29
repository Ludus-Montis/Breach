//! The fight's pass over the choice's scenarios (`fight::tests::built`): two grems a side,
//! each unit at 20 hp and full stamina, every blow a plain hit (no rolls).

use super::*;
use crate::events::{HIT, SPAT};
use crate::fight::tests::{built, landing, rules, Built};
use crate::landing::HIT as LANDED_HIT;
use crate::locks::{FIGHTING, MOVING};

const BLOW: f64 = 2.0;

fn fresh(b: &mut Built) {
    for id in [100, 101, 200, 201] {
        let slot = b.slot(id);
        b.field.columns.hp[slot] = 20;
        b.field.columns.stamina[slot] = 100.0;
    }
}

impl Built {
    fn step(&mut self, log: &mut Log) -> Vec<Blow> {
        let rules = Rules { choose: rules(), land: landing(None), tick: 9, blow: BLOW };
        step(&mut self.squads, &mut self.field, &self.roster, &rules, None, log)
    }

    fn hp(&self, id: i64) -> i32 {
        self.field.columns.hp[self.slot(id)]
    }
}

#[test]
fn each_blow_hurts_its_target_says_hit_and_tires_its_striker() {
    let mut b = built(1.0, FIGHTING);
    fresh(&mut b);
    let slot = b.slot(100);
    b.field.columns.sheet[slot].tiring = 1.5;
    let faction = b.field.lexicon.code("player");
    b.field.columns.words[slot].faction_id = faction;
    let mut log = Log::default();
    let melee = b.step(&mut log);
    let dealt: Vec<(Who, i64)> = melee.iter().map(|blow| (blow.striker, blow.damage)).collect();
    assert_eq!(dealt, vec![((0, 0), 7), ((0, 1), 7), ((1, 0), 7), ((1, 1), 7)]);
    let struck: i32 = [100, 101, 200, 201].iter().map(|&id| 20 - b.hp(id)).sum();
    assert_eq!(struck, 28);
    let first = (b.field.columns.target_id[slot], b.hp(b.field.columns.target_id[slot]));
    let flat = log.take();
    let hit = |n: usize| flat[9 * n..9 * n + 9].to_vec();
    assert_eq!(flat.len(), 36);
    let hp = i64::from(first.1);
    let landed = i64::from(LANDED_HIT);
    assert_eq!(hit(0), vec![HIT as i64, 9, 100, i64::from(faction), first.0, 7, 0, landed, hp]);
    assert_eq!(b.field.columns.stamina[slot], 100.0 - BLOW * 1.5);
    assert_eq!(b.field.columns.stamina[b.slot(200)], 100.0 - BLOW);
}

#[test]
fn a_shot_hurts_first_and_says_spat_with_its_shooters_damage_type() {
    let mut b = built(3.0, MOVING);
    fresh(&mut b);
    b.roster.listed[0].living[0].attack_range = 4;
    let slot = b.slot(100);
    b.field.columns.sheet[slot].attack_range = 4;
    let arrow = b.field.lexicon.code("pierce");
    b.field.columns.words[slot].damage_type = arrow;
    let mut log = Log::default();
    assert!(b.step(&mut log).is_empty());
    let target = b.field.columns.target_id[slot];
    let squad = b.squads.id[0];
    let values = [100, target, 5, i64::from(arrow), i64::from(LANDED_HIT)];
    let mut spat = vec![SPAT as i64, 9, squad, i64::from(b.squads.faction[0])];
    spat.extend(values);
    assert_eq!(log.take(), spat);
    assert_eq!(b.hp(target), 15);
    assert_eq!(b.field.columns.stamina[slot], 100.0 - BLOW);
}

#[test]
fn a_unit_struck_to_nothing_falls_onto_the_fell_queue() {
    let mut b = built(1.0, FIGHTING);
    fresh(&mut b);
    let slot = b.slot(200);
    b.field.columns.hp[slot] = 5;
    let mut log = Log::default();
    b.step(&mut log);
    assert!(b.hp(200) <= 0);
    assert_eq!(b.field.take_fallen(), vec![200]);
}
