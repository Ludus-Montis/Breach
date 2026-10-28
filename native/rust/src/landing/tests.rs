//! BlowLanding's, BlowRoll's and BlowDamage's scenarios (Decisions 118 and 119), on arms built
//! here and the repo's tuning: a striker in slot 0 and a target in slot 1, a cell apart, the
//! target facing east.

use super::*;
use crate::content::tests::repo_content;
use crate::content::Content;

const EAST: f64 = 90.0;
const SLASHING: u32 = 0;
const FIRE: u32 = 1;

fn tuning() -> LandingTuning {
    let content = Content::load(&repo_content()).unwrap_or_else(|error| panic!("{error}"));
    LandingTuning::of(&content.tuning)
}

/// SkirmishUnit's defaults: skill 40, defence 10, critical 1.5, floors 1, agility 10.
fn unit(id: i64, slot: usize) -> Arms {
    Arms {
        id,
        slot,
        melee_skill: 40.0,
        ranged_skill: 40.0,
        critical: 1.5,
        melee_floor: 1.0,
        ranged_floor: 1.0,
        max_stamina: 100.0,
        agility: 10.0,
        dodging: 1.0,
        defence: 10,
        ..Arms::default()
    }
}

/// A target of skill 40, agility 10 and defence 10: with the tuning a parry of 10, a dodge of
/// 10, grazes to 30 and hits to 90.
fn target(parries: bool) -> Arms {
    Arms { parries, ..unit(2, 1) }
}

/// A striker of 10 slashing (floor half), at `skill`.
fn striker(skill: f64, parts: &[(f64, u32, bool)]) -> Arms {
    Arms {
        melee_skill: skill,
        melee_floor: 0.5,
        melee_parts: parts
            .iter()
            .map(|&(amount, kind, magical)| Part { amount, kind, magical })
            .collect(),
        ..unit(1, 0)
    }
}

fn slashing(skill: f64) -> Arms {
    striker(skill, &[(10.0, SLASHING, false)])
}

fn bare() -> Arms {
    Arms { defence: 0, ..unit(2, 1) }
}

/// Two slots: the striker at the origin, the target a cell east of it facing east (so the
/// striker stands at its rear), both fresh.
fn columns() -> Columns {
    let mut columns = Columns::default();
    columns.grow();
    columns.grow();
    columns.position[1] = V2::new(1.0, 0.0);
    columns.bearing[1] = EAST;
    columns.stamina[0] = 100.0;
    columns.stamina[1] = 100.0;
    columns
}

fn blow(nominal: i64, ranged: bool, flank: bool) -> Blow {
    Blow { striker: 0, target: 1, nominal, ranged, flank, band: Some(0), pressed: 1 }
}

#[test]
fn the_margin_falls_through_parry_dodge_defence_and_hit_into_a_critical() {
    let (t, target) = (tuning(), target(true));
    let cases = [
        (-20.0, PARRIED),
        (5.0, PARRIED),
        (15.0, DODGED),
        (25.0, GRAZED),
        (50.0, HIT),
        (95.0, CRITICAL),
    ];
    for (margin, landed) in cases {
        assert_eq!(outcome(margin, &target, FRONT, false, &t), landed, "margin {margin}");
    }
}

#[test]
fn a_flank_finds_no_parry_the_rear_no_dodge_and_range_no_parry() {
    let t = tuning();
    assert_eq!(outcome(5.0, &target(true), FLANK, false, &t), DODGED);
    assert_eq!(outcome(5.0, &target(true), REAR, false, &t), GRAZED);
    assert_eq!(outcome(-40.0, &target(true), REAR, false, &t), GRAZED);
    assert_eq!(outcome(5.0, &target(true), FRONT, true, &t), DODGED);
    assert_eq!(outcome(5.0, &target(false), FRONT, false, &t), DODGED);
}

#[test]
fn the_margin_is_skill_and_shift_with_the_roll_across_the_die() {
    let t = tuning();
    let striker = Arms { melee_skill: 40.0, ranged_skill: 70.0, ..unit(1, 0) };
    assert_eq!(margin(&striker, false, 15.0, 0.5, &t), 55.0);
    assert_eq!(margin(&striker, false, 0.0, 0.0, &t), 40.0 - t.die / 2.0);
    assert_eq!(margin(&striker, true, 0.0, 0.5, &t), 70.0);
}

#[test]
fn the_side_a_blow_falls_on() {
    let t = tuning();
    let at = V2::ZERO;
    assert_eq!(side(EAST, at, V2::new(-1.0, 0.0), false, &t), FRONT);
    assert_eq!(side(EAST, at, V2::new(0.0, 1.0), true, &t), FLANK);
    assert_eq!(side(EAST, at, V2::new(-1.0, 0.2), true, &t), REAR);
    assert!(!from_flank(EAST, at, V2::new(3.0, 0.0), &t));
    assert!(from_flank(EAST, at, V2::new(-3.0, 0.0), &t));
}

#[test]
fn the_roll_falls_between_the_floor_skill_lifts_and_the_full() {
    let t = tuning();
    let target = bare();
    assert_eq!(dealt(&slashing(0.0), &target, 10, HIT, false, 0.0, &t), 5);
    assert_eq!(dealt(&slashing(0.0), &target, 10, HIT, false, 1.0, &t), 10);
    assert_eq!(dealt(&slashing(t.floor_skill), &target, 10, HIT, false, 0.0, &t), 10);
    assert!(spill(&slashing(t.floor_skill + 10.0), false, &t) > 0.0);
    assert_eq!(spill(&slashing(t.floor_skill), false, &t), 0.0);
}

#[test]
fn how_it_landed_takes_its_share() {
    let (t, target, striker) = (tuning(), bare(), slashing(0.0));
    assert_eq!(dealt(&striker, &target, 10, PARRIED, false, 1.0, &t), 0);
    assert_eq!(dealt(&striker, &target, 10, DODGED, false, 1.0, &t), 0);
    assert_eq!(dealt(&striker, &target, 10, GRAZED, false, 1.0, &t), 5);
    assert_eq!(dealt(&striker, &target, 10, CRITICAL, false, 1.0, &t), 15);
}

#[test]
fn weakness_resistance_and_immunity_by_part() {
    let t = tuning();
    let striker = striker(0.0, &[(6.0, SLASHING, false), (4.0, FIRE, false)]);
    let troll = Arms { weak: 1 << FIRE, ..bare() };
    let golem = Arms { resists: 1 << SLASHING, immune: 1 << FIRE, ..bare() };
    assert_eq!(dealt(&striker, &troll, 10, HIT, false, 1.0, &t), 12);
    assert_eq!(dealt(&striker, &golem, 10, HIT, false, 1.0, &t), 3);
}

#[test]
fn armour_stops_mundane_blows_and_ward_magical_ones() {
    let t = tuning();
    let mundane = striker(0.0, &[(6.0, SLASHING, false)]);
    let magical = striker(0.0, &[(6.0, FIRE, true)]);
    let knight = Arms { armour: 4, ..bare() };
    let mage = Arms { ward: 10, ..bare() };
    assert_eq!(dealt(&mundane, &knight, 6, HIT, false, 1.0, &t), 2);
    assert_eq!(dealt(&magical, &knight, 6, HIT, false, 1.0, &t), 6);
    assert_eq!(dealt(&magical, &mage, 6, HIT, false, 1.0, &t), 0);
    assert_eq!(dealt(&mundane, &mage, 6, GRAZED, false, 0.0, &t), 2);
}

#[test]
fn a_unit_made_without_weapons_parts_strikes_for_its_damage() {
    let t = tuning();
    assert_eq!(dealt(&unit(1, 0), &bare(), 7, HIT, false, 1.0, &t), 7);
}

#[test]
fn without_rolls_every_blow_is_a_plain_hit_at_full_damage() {
    let t = tuning();
    let mut columns = columns();
    let arms = [Arms { wounding: 2, ..unit(1, 0) }, bare()];
    let blows = [blow(4, false, false), blow(4, true, true)];
    assert_eq!(land(&mut columns, &arms, &blows, &t, None, None), vec![(4, HIT), (4, HIT)]);
    assert_eq!(columns.last_wounding[1], 2);
}

#[test]
fn a_shot_without_rolls_marks_no_wounds() {
    let t = tuning();
    let mut columns = columns();
    let arms = [Arms { wounding: 2, ..unit(1, 0) }, bare()];
    land(&mut columns, &arms, &[blow(4, true, false)], &t, None, None);
    assert_eq!(columns.last_wounding[1], 0);
}

#[test]
fn a_blow_of_a_type_it_fears_stops_the_targets_regeneration() {
    let t = tuning();
    let mut columns = columns();
    let troll = Arms { stops: 1 << FIRE, ..bare() };
    let arms = [striker(0.0, &[(3.0, SLASHING, false)]), troll.clone()];
    land(&mut columns, &arms, &[blow(3, false, false)], &t, None, None);
    assert_eq!(columns.regeneration_halt[1], 0.0);
    let arms = [striker(0.0, &[(3.0, FIRE, true)]), troll];
    land(&mut columns, &arms, &[blow(3, false, false)], &t, None, None);
    assert_eq!(columns.regeneration_halt[1], t.regeneration_halt);
}

/// How many of `ticks` ticks' rolled blows of `blow` land (graze, hit or critical).
fn landed_over(blow: Blow, ticks: i64) -> usize {
    let t = tuning();
    let mut columns = columns();
    let arms = [slashing(40.0), target(true)];
    (1..=ticks)
        .filter(|&tick| {
            land(&mut columns, &arms, &[blow], &t, Some((11, tick)), None)[0].1 >= GRAZED
        })
        .count()
}

#[test]
fn a_wavering_formation_lands_fewer_of_its_blows() {
    let steady = landed_over(blow(4, false, false), 400);
    let wavering = landed_over(Blow { band: Some(2), ..blow(4, false, false) }, 400);
    assert!(wavering + 20 < steady, "{wavering} of 400 wavering, {steady} steady");
}

#[test]
fn a_unit_fought_by_two_is_struck_more_surely() {
    let alone = landed_over(blow(4, false, false), 400);
    let pressed = landed_over(Blow { pressed: 2, ..blow(4, false, false) }, 400);
    assert!(pressed > alone, "{pressed} of 400 pressed, {alone} alone");
}

#[test]
fn a_tired_striker_lands_fewer() {
    let t = tuning();
    let arms = [slashing(40.0), target(true)];
    let count = |stamina: f64| {
        let mut columns = columns();
        columns.stamina[0] = stamina;
        (1..=400)
            .filter(|&tick| {
                land(&mut columns, &arms, &[blow(4, false, false)], &t, Some((11, tick)), None)[0].1
                    >= GRAZED
            })
            .count()
    };
    assert!(count(10.0) < count(100.0));
}

#[test]
fn the_same_seed_lands_the_same_blows_and_another_seed_others() {
    let t = tuning();
    let arms = [slashing(40.0), target(true)];
    let run = |seed: i64| -> Vec<(i64, u8)> {
        let mut columns = columns();
        (1..=200)
            .flat_map(|tick| {
                land(&mut columns, &arms, &[blow(4, false, false)], &t, Some((seed, tick)), None)
            })
            .collect()
    };
    assert_eq!(run(11), run(11));
    assert_ne!(run(11), run(12));
}

#[test]
fn a_blow_lands_the_same_whatever_the_arms_order() {
    let t = tuning();
    let forward = [slashing(40.0), target(true)];
    let reversed = [target(true), slashing(40.0)];
    for tick in 1..=50 {
        let mut a = columns();
        let mut b = columns();
        let there = land(&mut a, &forward, &[blow(4, false, true)], &t, Some((11, tick)), None);
        let back = Blow { striker: 1, target: 0, ..blow(4, false, true) };
        assert_eq!(there, land(&mut b, &reversed, &[back], &t, Some((11, tick)), None));
    }
}
