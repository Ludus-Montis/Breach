//! A digest battle played in the crate: its squads listed with every unit as CrateRoster.of
//! lists them, a tick that counts and clears the orders, the units no squad holds let go, and
//! the digest's stop rules.

use super::*;
use crate::content::tests::repo_content;
use crate::events::SWAPPED;

fn content() -> Content {
    Content::load(&repo_content()).expect("the repo's content loads")
}

/// `battle` set up on a fresh crate, seed 1, blows rolled.
fn fresh(battle: &Battle, content: &Content) -> (Squads, Field, Play) {
    let (mut squads, mut field) = (Squads::default(), Field::default());
    let play = Play::new(battle, (&mut squads, &mut field), content, (1, true, false));
    (squads, field, play.expect("the battle sets up"))
}

const HEAD_ON: Battle = Battle::Mirror { flank: false, swap: false };

#[test]
fn each_squad_is_listed_with_every_unit_it_holds_as_its_sheet_and_objects_have_it() {
    let content = content();
    let (squads, mut field, play) = fresh(&HEAD_ON, &content);
    field.columns.state[squads.members[1][0] as usize] = crate::squads::DEAD;

    let roster = listed(&squads, &field, &play.listed, false);

    assert_eq!(play.listed, vec![0, 1]);
    assert_eq!(roster.listed.iter().map(|l| l.living.len()).collect::<Vec<_>>(), vec![8, 8]);
    let first = &roster.listed[0].living[0];
    let sheet = &field.columns.sheet[first.slot];
    assert_eq!(first.id, field.unit_at(first.slot as u32).unwrap());
    assert_eq!(first.foot, (sheet.footprint_width, sheet.footprint_depth));
    assert_eq!((first.turn_rate, first.max_stamina), (sheet.turn_rate, sheet.max_stamina));
    assert_eq!(first.kind, 0, "the grem's definition code");
    assert_eq!(first.walker.height, 0.0, "open ground's walker");
    let walked = listed(&squads, &field, &play.listed, true);
    assert!(walked.listed[0].living[0].walker.height > 0.0, "its own walker");
}

#[test]
fn a_tick_counts_clears_the_orders_and_keeps_the_squads_listed() {
    let content = content();
    let flank = Battle::Mirror { flank: true, swap: false };
    let (mut squads, mut field, mut play) = fresh(&flank, &content);
    assert_eq!(squads.pending.len(), 1, "the line's hold, queued");

    let ticked = play.step(&mut squads, &mut field, &content, &mut Log::default());

    assert!(ticked.is_some());
    assert_eq!(play.tick, 1);
    assert!(squads.pending.is_empty());
    assert_eq!(play.listed, vec![0, 1]);
}

#[test]
fn the_units_no_listed_squad_holds_give_up_their_slots() {
    let content = content();
    let (mut squads, mut field, play) = fresh(&HEAD_ON, &content);
    let gone = squads.members[1].pop().unwrap();
    let id = field.unit_at(gone as u32).unwrap();

    settle(&squads, &mut field, &play.listed);

    assert_eq!(field.slot_of(id), None);
    assert_eq!(field.held_count(), 15);
}

#[test]
fn the_head_on_mirror_plays_to_a_side_broken_and_on_for_the_tail() {
    let content = content();
    let (mut squads, mut field, mut play) = fresh(&HEAD_ON, &content);
    let mut watch = Watch::new(1500, None);

    let ticks =
        run(&mut play, (&mut squads, &mut field), &content, &mut Log::default(), &mut watch);

    assert!(ticks < 1500, "a side broke: {ticks}");
    assert!(!play.both_stand(&squads, &field));
    assert_eq!(watch.broken.map(|b| ticks - b), Some(TAIL));
    assert!(watch.contact.is_some_and(|c| c < watch.broken.unwrap()));
}

#[test]
fn a_run_stops_so_many_ticks_past_first_contact_or_at_its_limit() {
    let mut past = Watch::new(100, Some(3));
    let stops: Vec<bool> = [false, true, false, false, false]
        .iter()
        .enumerate()
        .map(|(n, &engaged)| past.stops(n as i64 + 1, engaged, true))
        .collect();
    assert_eq!(stops, vec![false, false, false, false, true]);

    let mut limited = Watch::new(2, None);
    assert!(!limited.stops(1, false, true));
    assert!(limited.stops(2, false, true));
}

#[test]
fn a_side_broken_plays_on_for_the_tail_then_stops() {
    let mut watch = Watch::new(1500, None);
    assert!(!watch.stops(10, true, false));
    assert!(!watch.stops(10 + TAIL - 1, false, true));
    assert!(watch.stops(10 + TAIL, false, true));
}

#[test]
fn an_event_is_told_past_a_list_field() {
    let mut squads = Squads::default();
    squads.open(7);
    let mut log = Log::default();
    log.squad_listed(SWAPPED, (1, &squads, 0), &[100], &[5, 6, 7]);
    log.squad(ENGAGED, 1, &squads, 0, &vec![0; KINDS[ENGAGED].1.len()]);
    let flat = log.take();

    assert!(told(&flat, ENGAGED));
    assert!(told(&flat, SWAPPED));
    assert!(!told(&flat[..flat.len() - 4 - KINDS[ENGAGED].1.len()], ENGAGED));
}
