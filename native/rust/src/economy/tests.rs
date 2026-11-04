//! The economy: the former gdUnit scenarios of EconomySystem and CaptureResolution, and what
//! GDScript's dictionaries did that the crate must too (a name outside the five, a node
//! captured again, a choice on a node that holds none).

use super::{decayed_yield, Captures, Choice, Economy};
use crate::content::{Node, NodeType};

fn farm() -> Node {
    Node {
        node_type: NodeType::Resource,
        yield_food_per_tick: 10,
        decay_interval_ticks: 5,
        decay_floor_food: 2,
        ravage_yield_food: 40,
        ..Node::default()
    }
}

fn fort() -> Node {
    Node {
        node_type: NodeType::Fort,
        garrison: 1,
        garrison_hp: 10,
        garrison_dmg: 3,
        dismantle_wood_yield: 8,
        dismantle_stone_yield: 5,
        fortify_wood_cost: 10,
        fortify_stone_cost: 6,
        ..Node::default()
    }
}

#[test]
fn a_new_pool_starts_at_zero() {
    assert_eq!(Economy::default().balance("food"), 0);
}

#[test]
fn adding_increases_the_balance() {
    let mut economy = Economy::default();
    economy.add("food", 10);
    economy.add("food", 5);
    assert_eq!(economy.balance("food"), 15);
}

#[test]
fn a_spend_it_can_afford_decreases_the_balance() {
    let mut economy = Economy::default();
    economy.add("wood", 10);
    assert!(economy.spend("wood", 4));
    assert_eq!(economy.balance("wood"), 6);
}

#[test]
fn a_spend_it_cannot_afford_fails_and_spends_nothing() {
    let mut economy = Economy::default();
    economy.add("stone", 3);
    assert!(!economy.spend("stone", 10));
    assert_eq!(economy.balance("stone"), 3);
}

#[test]
fn the_pools_are_independent() {
    let mut economy = Economy::default();
    economy.add("metal", 7);
    assert_eq!(economy.balance("metal"), 7);
    assert_eq!(economy.balance("crystal"), 0);
}

#[test]
fn a_name_outside_the_five_reads_zero_until_added_to() {
    let mut economy = Economy::default();
    assert_eq!(economy.balance("gold"), 0);
    assert!(!economy.spend("gold", 1));
    economy.add("gold", 3);
    assert!(economy.spend("gold", 1));
    assert_eq!(economy.balance("gold"), 2);
}

#[test]
fn a_yield_with_no_interval_elapsed_is_the_base() {
    assert_eq!(decayed_yield(10, 0, 5, 2), 10);
}

#[test]
fn a_yield_drops_ten_percent_after_one_interval() {
    assert_eq!(decayed_yield(10, 5, 5, 2), 9);
}

#[test]
fn a_yield_floors_after_enough_intervals() {
    assert_eq!(decayed_yield(10, 100, 5, 2), 2);
}

#[test]
fn a_yield_never_goes_below_a_floor_of_zero() {
    assert_eq!(decayed_yield(10, 1000, 5, 0), 0);
}

#[test]
fn a_yield_rounds_half_away_from_zero() {
    // 5 * 0.9 = 4.5 rounds to 5 (GDScript's `round`), and 15 * 0.81 = 12.15 to 12.
    assert_eq!(decayed_yield(5, 1, 1, 0), 5);
    assert_eq!(decayed_yield(15, 2, 1, 0), 12);
}

#[test]
fn a_resource_node_captured_harvests_and_awaits_no_choice() {
    let mut captures = Captures::default();
    captures.captured(1, &farm());
    assert!(!captures.is_awaiting_choice(1));
}

#[test]
fn a_fort_captured_awaits_a_choice() {
    let mut captures = Captures::default();
    captures.captured(2, &fort());
    assert!(captures.is_awaiting_choice(2));
}

#[test]
fn a_harvest_adds_the_current_decayed_yield_every_tick() {
    let (mut captures, mut economy) = (Captures::default(), Economy::default());
    captures.captured(1, &farm());
    captures.tick(&mut economy);
    assert_eq!(economy.balance("food"), 10);
}

#[test]
fn a_harvest_s_yield_decays_after_enough_ticks() {
    let (mut captures, mut economy) = (Captures::default(), Economy::default());
    captures.captured(1, &farm());
    for _ in 0..5 {
        captures.tick(&mut economy);
    }
    // Ticks 1 to 4 yield 10 each; tick 5, one interval in, yields 9.
    assert_eq!(economy.balance("food"), 49);
}

#[test]
fn ravaging_grants_a_lump_sum_and_stops_the_harvest() {
    let (mut captures, mut economy) = (Captures::default(), Economy::default());
    captures.captured(1, &farm());
    captures.choose(1, Choice::Ravage, &mut economy);
    captures.tick(&mut economy);
    captures.choose(1, Choice::Ravage, &mut economy);
    assert_eq!(economy.balance("food"), 40);
}

#[test]
fn dismantling_grants_the_salvage_and_ends_the_wait() {
    let (mut captures, mut economy) = (Captures::default(), Economy::default());
    captures.captured(2, &fort());
    captures.choose(2, Choice::Dismantle, &mut economy);
    captures.choose(2, Choice::Dismantle, &mut economy);
    assert_eq!(economy.balance("wood"), 8);
    assert_eq!(economy.balance("stone"), 5);
    assert!(!captures.is_awaiting_choice(2));
}

#[test]
fn fortifying_spends_its_cost_and_ends_the_wait() {
    let (mut captures, mut economy) = (Captures::default(), Economy::default());
    economy.add("wood", 10);
    economy.add("stone", 6);
    captures.captured(2, &fort());
    captures.choose(2, Choice::Fortify, &mut economy);
    assert_eq!(economy.balance("wood"), 0);
    assert_eq!(economy.balance("stone"), 0);
    assert!(!captures.is_awaiting_choice(2));
}

#[test]
fn fortifying_does_nothing_when_it_cannot_be_afforded() {
    let (mut captures, mut economy) = (Captures::default(), Economy::default());
    economy.add("wood", 10);
    captures.captured(2, &fort());
    captures.choose(2, Choice::Fortify, &mut economy);
    assert_eq!(economy.balance("wood"), 10);
    assert!(captures.is_awaiting_choice(2));
}

#[test]
fn a_choice_for_the_other_kind_of_node_does_nothing() {
    let (mut captures, mut economy) = (Captures::default(), Economy::default());
    captures.captured(1, &farm());
    captures.captured(2, &fort());
    captures.choose(2, Choice::Ravage, &mut economy);
    captures.choose(1, Choice::Dismantle, &mut economy);
    captures.choose(3, Choice::Fortify, &mut economy);
    assert_eq!((economy.balance("food"), economy.balance("wood")), (0, 0));
    assert!(captures.is_awaiting_choice(2));
}

#[test]
fn a_node_captured_again_starts_afresh() {
    let (mut captures, mut economy) = (Captures::default(), Economy::default());
    captures.captured(1, &farm());
    for _ in 0..4 {
        captures.tick(&mut economy);
    }
    captures.choose(1, Choice::Ravage, &mut economy);
    captures.captured(1, &farm());
    captures.tick(&mut economy);
    // 40 harvested, 40 ravaged, then 10: the count started again from the new capture.
    assert_eq!(economy.balance("food"), 90);
}

#[test]
fn other_kinds_of_node_hold_nothing() {
    let (mut captures, mut economy) = (Captures::default(), Economy::default());
    captures.captured(0, &Node { node_type: NodeType::Origin, ..farm() });
    captures.tick(&mut economy);
    assert_eq!(economy.balance("food"), 0);
    assert!(!captures.is_awaiting_choice(0));
}

#[test]
fn the_choices_are_named_by_their_words() {
    assert_eq!(Choice::named("ravage"), Some(Choice::Ravage));
    assert_eq!(Choice::named("fortify"), Some(Choice::Fortify));
    assert_eq!(Choice::named("dismantle"), Some(Choice::Dismantle));
    assert_eq!(Choice::named("raze"), None);
}
