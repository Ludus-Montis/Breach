//! The pass's scenarios (spec 28 part 7, Decisions 125 and 126), on a squad built here:
//! squad 1 (listed 0) marching under its own command, units 100 and 101 at fresh speed 1
//! with 100 stamina in condition 1; what GDScript handed the pass is read in the crate.

use super::*;
use crate::content::tests::repo_content;
use crate::content::Content;
use crate::locks;
use crate::maths::V2;
use crate::squads::Pursuit;

const SECONDS: f64 = 0.5;

fn tuning() -> StaminaTuning {
    let content = Content::load(&repo_content()).unwrap_or_else(|error| panic!("{error}"));
    StaminaTuning::of(&content.tuning)
}

struct Built {
    squads: Squads,
    field: Field,
}

fn built() -> Built {
    let (mut squads, mut field) = (Squads::default(), Field::default());
    squads.open(1);
    squads.state[0] = locks::MOVING;
    squads.command[0] = Some(squads.commands.open(0.0));
    for id in [100, 101] {
        let slot = field.hold(id) as usize;
        let columns = &mut field.columns;
        (columns.stamina[slot], columns.condition[slot]) = (100.0, 1.0);
        columns.speed[slot] = 1.0;
        columns.sheet[slot].fresh_speed = 1.0;
        squads.members[0].push(slot as i32);
    }
    Built { squads, field }
}

impl Built {
    fn slot(&self, id: i64) -> usize {
        self.field.slot_of(id).unwrap() as usize
    }

    fn step(&mut self) -> Vec<i64> {
        step(&self.squads, &mut self.field, &[0], SECONDS, &tuning())
    }

    fn stamina(&self, id: i64) -> f64 {
        self.field.columns.stamina[self.slot(id)]
    }

    fn pursue(&mut self) {
        let post = Pursuit {
            foe: 2,
            post: 0.0,
            post_at: V2::ZERO,
            home: 0.0,
            direction: 1,
            order: 0,
            returning: false,
        };
        self.squads.pursuit[0] = Some(post);
    }

    fn trait_of(&mut self, id: i64, name: &str) {
        let (code, slot) = (self.field.lexicon.code(name), self.slot(id));
        self.field.columns.words[slot].traits.push((code, 1.0));
    }
}

#[test]
fn a_squad_its_command_hurries_runs_and_tires() {
    let mut b = built();
    let c = b.squads.command[0].unwrap();
    b.squads.commands.give(c, HURRY, true);

    b.step();

    assert!(b.stamina(100) < 100.0, "it ran");
    assert!(b.field.columns.speed[b.slot(100)] > 1.0, "at more than its march");
}

#[test]
fn an_unhurried_squad_marches_light_at_no_cost() {
    let mut b = built();

    b.step();

    assert_eq!((b.stamina(100), b.field.columns.speed[b.slot(100)]), (100.0, 1.0));
}

#[test]
fn a_pursuer_reacts_the_sooner_the_quicker_its_wits() {
    let mut b = built();
    b.squads.state[0] = locks::FIGHTING;
    b.pursue();
    let wits = b.field.lexicon.code("wits");
    let quick = b.slot(101);
    b.field.columns.words[quick].attributes.push((wits, 20.0));

    b.step();

    let t = tuning();
    let built = |slot: usize| b.field.columns.run_build[slot];
    assert_eq!(built(b.slot(100)), -t.run_reaction + SECONDS);
    assert_eq!(built(quick), -t.run_reaction * 10.0 / 20.0 + SECONDS);
}

#[test]
fn a_pursuit_led_by_one_who_keeps_to_a_march_marches() {
    let mut b = built();
    b.pursue();
    let leader = b.slot(100);
    b.field.columns.sheet[leader].leadership = 5;
    b.trait_of(100, "pursues_at_march");

    b.step();

    assert_eq!(b.field.columns.was_running[b.slot(101)], 0);
}

#[test]
fn a_unit_made_by_hand_takes_its_speed_as_fresh() {
    let mut b = built();
    let slot = b.slot(101);
    b.field.columns.sheet[slot].fresh_speed = 0.0;
    b.field.columns.speed[slot] = 2.0;

    let made = b.step();

    assert_eq!((made, b.field.columns.sheet[slot].fresh_speed), (vec![101], 2.0));
}
