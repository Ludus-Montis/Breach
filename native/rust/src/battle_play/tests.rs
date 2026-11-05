//! The 1D feel test's battle in the crate: FormationBattle's, FormationLane's and
//! SkirmishSlotPool's scenarios, the presets and what the scene sees of a lane.

use super::presets::{self, Saved};
use super::slots::SlotPool;
use super::*;
use crate::content::tests::repo_content;

const GREM: &str = "grem";
const BRUTE: &str = "grem_brute";

fn content() -> Content {
    Content::load(&repo_content()).expect("the repo's content loads")
}

fn line(content: &Content, count: i32, width: i32) -> Template {
    Template::line(GREM, foot(content, GREM).unwrap(), count, width)
}

/// test_formation_battle.gd's battle: 8 slots, lanes c (5 wide) and k (4), two grems painted
/// on each, one grem builder.
fn battle(content: &Content) -> BattlePlay {
    let widths = vec![("c".to_string(), 5), ("k".to_string(), 4)];
    let mut battle = BattlePlay::new(8, widths, 0.1, 3, content).expect("it sets up");
    assert!(battle.add_lane("c", 9.0, line(content, 2, 5), content));
    assert!(battle.add_lane("k", 9.0, line(content, 2, 4), content));
    battle.player.set(GREM, content.units[GREM].build_seconds, 1);
    battle
}

fn run(battle: &mut BattlePlay, ticks: usize, with_kingdom: bool, content: &Content) {
    for _ in 0..ticks {
        battle.step(with_kingdom, content);
    }
}

fn has_kingdom(battle: &BattlePlay, k: usize) -> bool {
    battle.lanes[k].seen().iter().any(|s| s.faction == "the_kingdom")
}

#[test]
fn a_lane_is_capped_and_an_assignment_clamped_to_what_is_free() {
    let mut pool = SlotPool::new(8, vec![5 * MAX_RANKS, 4 * MAX_RANKS, 1]);
    assert_eq!(pool.free(), 8);
    assert_eq!(pool.assign(0, 6), 6);
    assert_eq!(pool.assign(1, 5), 2, "only 2 are left");
    assert_eq!(pool.assign(0, 0), 0, "nothing on a lane is a real choice");
    assert_eq!(pool.assign(2, 3), 1, "its own cap");
    assert_eq!(pool.free(), 5);
}

#[test]
fn the_pool_starts_with_what_each_lane_has_painted() {
    let content = content();
    let battle = battle(&content);
    assert_eq!((battle.pool.assigned(0), battle.pool.assigned(1)), (2, 2));
    assert_eq!(battle.allowance(0), 6, "its own 2 plus the 4 free");
}

#[test]
fn one_builder_shares_grems_out_in_turn() {
    let content = content();
    let mut battle = battle(&content);
    battle.player.prefer(None);
    run(&mut battle, 40, false, &content); // a grem every 2 s
    assert_eq!((battle.waves[0].built(), battle.waves[1].built()), (1, 1));
}

#[test]
fn reshaping_banks_the_displaced_units_in_the_reserve() {
    let content = content();
    let mut battle = battle(&content);
    battle.player.set(GREM, content.units[GREM].build_seconds, 2);
    run(&mut battle, 40, false, &content); // both of lane c's grems
    battle.lanes[0].brush = Some((BRUTE, foot(&content, BRUTE).unwrap()));
    assert_eq!(battle.paint(0, (0, 0), false), 2);
    assert_eq!(battle.player.reserve.len(), 2);
    assert_eq!(battle.pool.assigned(0), 4);
}

#[test]
fn erasing_in_one_lane_frees_slots_for_the_other() {
    let content = content();
    let mut battle = battle(&content);
    battle.paint(1, (0, 0), true);
    assert_eq!(battle.pool.assigned(1), 1);
    assert_eq!(battle.allowance(0), 7);
}

#[test]
fn painting_off_the_lane_or_past_the_allowance_changes_nothing() {
    let content = content();
    let mut battle = battle(&content);
    battle.lanes[0].brush = Some((GREM, foot(&content, GREM).unwrap()));
    assert_eq!(battle.paint(0, (0, 7), false), 0);
    assert_eq!(battle.waves[0].template.ordered().len(), 2, "off the lane: refused");
    for column in 0..5 {
        battle.paint(0, (1, column), false);
    }
    assert_eq!(battle.waves[0].template.cells(), 6, "its 2 and the 4 free");
    battle.lanes[0].brush = None;
    battle.paint(0, (0, 0), false);
    assert_eq!(battle.waves[0].template.cells(), 5, "no brush erases");
}

#[test]
fn the_kingdom_builds_its_lines_only_when_asked() {
    let content = content();
    let mut battle = battle(&content);
    run(&mut battle, 200, false, &content);
    assert!(!has_kingdom(&battle, 0));
    run(&mut battle, 200, true, &content); // two militia builders, 5 s each, three to a line
    assert!(has_kingdom(&battle, 0));
}

#[test]
fn a_wave_that_departs_on_its_own_is_told_full_then_sent() {
    let content = content();
    let mut battle = battle(&content);
    battle.waves[0].auto = true;
    battle.waves[0].fill(GREM);
    battle.waves[0].fill(GREM);
    let stepped = battle.step(false, &content);
    let told: Vec<&str> = stepped.lanes[0].iter().map(|(side, _)| *side).collect();
    assert_eq!(told, vec!["player", "player"]);
    assert!(matches!(stepped.lanes[0][0].1, Produced::WaveFull { built: 2, .. }));
    assert!(matches!(stepped.lanes[0][1].1, Produced::Departed { units: 2, .. }));
    let seen = battle.lanes[0].seen();
    assert_eq!(seen.len(), 1);
    assert_eq!((seen[0].faction.as_str(), seen[0].units.len()), ("player", 2));
}

#[test]
fn a_lane_that_merges_marks_the_squads_it_sends() {
    let content = content();
    let mut battle = battle(&content);
    battle.lanes[1].merges = true;
    battle.waves[1].fill(GREM);
    let s = battle.send(1, &content).expect("a grem is built");
    let lane = &battle.lanes[1];
    let command = lane.squads.command[s].expect("it has a command");
    assert!(lane.squads.commands.gives(command, crate::commands::MERGES));
    assert!(battle.send(1, &content).is_none(), "the wave was emptied");
}

#[test]
fn the_militia_line_stands_at_the_far_end_facing_back() {
    let content = content();
    let mut battle = battle(&content);
    battle.spawn_kingdom_line(0, &content);
    battle.spawn_kingdom_line(1, &content);
    let (c, k) = (battle.lanes[0].seen(), battle.lanes[1].seen());
    assert_eq!(c[0].units.len(), 3);
    assert_eq!(c[0].direction, -1);
    assert!((c[0].units[0].distance - 9.0).abs() < 1e-6, "at the lane's far end");
    assert_eq!(c[0].units[0].id, k[0].units[0].id, "ids are per lane");
    let mut across: Vec<f64> = c[0].units.iter().map(|u| u.lateral).collect();
    across.dedup();
    assert_eq!(across.len(), 3, "side by side across the lane");
}

#[test]
fn a_saved_shape_fits_a_narrower_lane_front_first() {
    let content = content();
    let mut template = line(&content, 5, 5);
    template.slot_limit = 9;
    template.paint(BRUTE, foot(&content, BRUTE).unwrap(), (1, 0));
    let saved = presets::saved(&template);
    assert_eq!(saved.len(), 6);
    assert_eq!(saved[5], Saved { kind: "brute".into(), rank: 1, column: 0 });
    let fitted = presets::fitted(&saved, 3, 5, &content);
    let at: Vec<(&str, (i32, i32))> = fitted.ordered().iter().map(|p| (p.id, p.at)).collect();
    assert_eq!(at, vec![(GREM, (0, 0)), (GREM, (0, 1)), (GREM, (0, 2))]);
    let fitted = presets::fitted(&saved, 5, 9, &content);
    assert_eq!(fitted.ordered().len(), 6, "a brute keeps its kind");
}

#[test]
fn presets_saved_before_kinds_and_unknown_kinds() {
    let content = content();
    let old = |kind: &str, column| Saved { kind: kind.into(), rank: 0, column };
    let saved = [old("light", 0), old("heavy", 1), old("ogre", 4)];
    let fitted = presets::fitted(&saved, 8, 8, &content);
    let ids: Vec<&str> = fitted.ordered().iter().map(|p| p.id).collect();
    assert_eq!(ids, vec![GREM, BRUTE], "the ogre is skipped");
}
