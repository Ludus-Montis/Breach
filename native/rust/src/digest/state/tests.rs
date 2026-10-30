//! DigestState.state read in the crate: each listed squad's fields and its units', its
//! records, and its units' entries with exactly the keys they hold.

use super::*;
use crate::battles::Battle;
use crate::content::tests::repo_content;
use crate::content::Content;
use crate::entries::Key;
use crate::maths::V2;
use crate::play::Play;
use crate::squads::{Chaser, Flank};

/// The head-on mirror set up on a fresh crate.
fn fresh() -> (Squads, Field, Vec<usize>) {
    let content = Content::load(&repo_content()).expect("the repo's content loads");
    let (mut squads, mut field) = (Squads::default(), Field::default());
    let battle = Battle::Mirror { flank: false, swap: false };
    let play = Play::new(&battle, (&mut squads, &mut field), &content, (1, true, false));
    (squads, field, play.expect("the battle sets up").listed)
}

fn list(value: &Plain) -> &Vec<Plain> {
    match value {
        Plain::List(values) => values,
        other => panic!("not a list: {other:?}"),
    }
}

/// The squad's [[fields], [fields], [fight_since, records, units]].
fn squad_of(state: &Plain, index: usize) -> &Vec<Plain> {
    list(&list(state)[index])
}

/// The unit's held entries: [loose, fleeing, chaser].
fn held_of(state: &Plain, squad: usize, unit: usize) -> Vec<Plain> {
    let units = list(&list(&squad_of(state, squad)[2])[2]);
    list(&list(&list(&units[unit])[1])[3]).clone()
}

/// A dictionary's keys as written, in order.
fn keys(dict: &Plain) -> Vec<String> {
    let pairs = list(&list(dict)[1]);
    pairs
        .iter()
        .map(|pair| match &list(pair)[0] {
            Plain::Str(key) => key.clone(),
            other => panic!("a key: {other:?}"),
        })
        .collect()
}

#[test]
fn each_listed_squad_is_read_in_order_with_its_units_and_no_records() {
    let (squads, field, listed) = fresh();
    let state = state(&squads, &field, &[listed[1], listed[0]]);

    let first = squad_of(&state, 0);
    assert_eq!(list(&first[0])[0], Plain::Int(squads.id[listed[1]]));
    let units = list(&list(&first[2])[2]);
    assert_eq!(units.len(), squads.members[listed[1]].len());
    let m = squads.members[listed[1]][0] as usize;
    let unit = list(&units[0]);
    assert_eq!(list(&unit[0])[0], Plain::Int(field.unit_at(m as u32).unwrap()));
    assert_eq!(list(&unit[0])[1], Plain::Int(field.columns.hp[m] as i64));
    let empty = Plain::dict(vec![]);
    assert_eq!(
        list(&first[2])[1],
        Plain::List(vec![empty.clone(), empty.clone(), empty.clone(), empty])
    );
    assert_eq!(held_of(&state, 0, 0), vec![Plain::Nil, Plain::Nil, Plain::Nil]);
}

#[test]
fn a_loose_entry_holds_exactly_the_keys_it_holds_touch_as_a_bool() {
    let (mut squads, mut field, listed) = fresh();
    let s = listed[0];
    let m = squads.members[s][0] as usize;
    let id = field.unit_at(m as u32).unwrap();
    squads.loose[s].push(id);
    field.columns.keys[m] = Key::Touch.bit() | Key::Toward.bit();
    field.columns.touch[m] = 1;

    let held = held_of(&state(&squads, &field, &listed), 0, 0);

    let want = ["\"at\"", "\"goal\"", "\"next\"", "\"touch\"", "\"toward\"", "\"unit\""];
    assert_eq!(keys(&held[0]), want);
    let pairs = list(&list(&held[0])[1]);
    assert_eq!(list(&pairs[1])[1], Plain::Nil, "no goal");
    assert_eq!(list(&pairs[3])[1], Plain::Bool(true));
    assert_eq!(list(&pairs[4])[1], Plain::Nil, "toward holds no value");
    assert_eq!(list(&pairs[5])[1], Plain::object(id));
}

#[test]
fn a_fleeing_entry_and_a_returning_chaser_are_read_with_their_keys() {
    let (mut squads, mut field, listed) = fresh();
    let s = listed[0];
    let m = squads.members[s][0] as usize;
    let id = field.unit_at(m as u32).unwrap();
    squads.fleeing[s].push(id);
    field.columns.keys[m] = Key::HeldBy.bit() | Key::HeldBy.valued();
    field.columns.held_by[m] = 7;
    let from = V2::new(1.0, 2.0);
    squads.chasers[s].push(Chaser { unit: id, foe: 3, from, leash: 4.5, returning: true });
    squads.flanks[s].push(Flank { edge: 2, foe: 3, since: 10 });

    let state = state(&squads, &field, &listed);
    let held = held_of(&state, 0, 0);

    assert_eq!(keys(&held[1]), ["\"along\"", "\"held_by\"", "\"offset\""]);
    assert_eq!(list(&list(&list(&held[1])[1])[1])[1], Plain::Int(7));
    let chaser = ["\"foe\"", "\"from\"", "\"leash\"", "\"returning\"", "\"unit\""];
    assert_eq!(keys(&held[2]), chaser);
    let flanks = &list(&list(&squad_of(&state, 0)[2])[1])[0];
    assert_eq!(keys(flanks), ["2"]);
}
