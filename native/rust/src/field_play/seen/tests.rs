//! The field as its scene sees it: the line and reserve with their units, a sent wave
//! counted and listed, B's staging point, and a pursuit's leash.

use super::super::tests::{content, fresh, played};
use super::*;
use crate::field_play::A;

#[test]
fn the_line_and_reserve_are_seen_with_their_units() {
    let content = content();
    let (squads, field, play) = fresh(&content);
    let seen = play.seen(&squads, &field, &content);
    assert_eq!(seen.len(), 2, "the line, then the reserve");
    let line = &seen[0];
    assert_eq!(line.faction, "the_kingdom");
    assert_eq!(line.id, squads.id[play.kingdom[0]]);
    assert_eq!(line.units.len(), squads.members[play.kingdom[0]].len());
    assert_eq!(line.morale, i64::from(squads.morale[play.kingdom[0]]));
    assert_eq!(line.position, squads.position[play.kingdom[0]]);
    assert!(!line.staged && line.pursuit.is_none());
    let unit = &line.units[0];
    let slot = squads.members[play.kingdom[0]][0] as usize;
    assert_eq!(unit.slot, slot);
    assert_eq!(Some(unit.id), field.unit_at(slot as u32));
    let sheet = &field.columns.sheet[slot];
    assert_eq!(unit.foot, (sheet.footprint_width, sheet.footprint_depth));
    assert_eq!(unit.detection, sheet.detection);
    assert_eq!(seen[1].units.len(), 6, "the reserve's six");
    assert_eq!(seen[1].band, 0, "steady");
}

#[test]
fn a_wave_is_counted_as_it_builds_and_seen_once_sent() {
    let content = content();
    let (mut squads, mut field, mut play) = fresh(&content);
    assert_eq!(play.counted(A, &content), [0, 8, 0, 0]);
    assert_eq!(play.counted(B, &content), [0, 8, 0, 1]);
    played((&mut squads, &mut field, &mut play), &content, 400);
    let [built, places, ..] = play.counted(A, &content);
    assert!(built > 0 && places == 8);
    play.send(A, 0, (&mut squads, &mut field), &content).expect("a wave");
    let seen = play.seen(&squads, &field, &content);
    assert_eq!(seen.len(), 3);
    let wave = &seen[2];
    assert_eq!(wave.faction, "player");
    assert_eq!(wave.units.len() as i64, built);
    assert_eq!(play.counted(A, &content)[0], 0, "the wave emptied");
    assert_ne!(seen[0].state, -1);
}

#[test]
fn b_s_staging_point_is_on_its_route_while_it_waits() {
    let content = content();
    let (_, _, mut play) = fresh(&content);
    assert_eq!(play.staging_point(), None);
    play.set_wait(true);
    assert_eq!(play.staging_point(), Some(V2::new(56.0, 21.0)));
    play.set_wait(false);
    assert_eq!(play.staging_point(), None);
}
