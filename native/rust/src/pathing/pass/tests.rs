//! Pathing's pass over squads and units built here: grems in a group on a lane through the
//! pathing tests' pond (`pathing::tests::pond`), their front placed by hand.

use super::*;
use crate::maths::V2;
use crate::pathing::tests::pond;
use crate::paths::shares::Cache;
use crate::paths::PathTuning;
use crate::route::Route;
use crate::sheet::NAMES;

/// The lane: from (2, 20) to (78, 20), in cells.
fn lane() -> Route {
    Route::new(vec![V2::new(2.0, 20.0), V2::new(78.0, 20.0)], 0.0)
}

/// A group per entry of `fronts` (cells along the lane), ids from 1, on one command along
/// the lane, marching east; two units each, ids from 100, standing at the front.
fn built(fronts: &[f64]) -> (Squads, Field) {
    let (mut squads, mut field) = (Squads::default(), Field::default());
    let command = squads.commands.open(0.0);
    squads.commands.route[command] = Some(lane());
    for (s, &front) in fronts.iter().enumerate() {
        squads.open(s as i64 + 1);
        squads.direction[s] = 1;
        squads.front_distance[s] = front / CELLS;
        squads.route[s] = Some(lane());
        squads.command[s] = Some(command);
        for n in 0..2 {
            let slot = field.hold(100 * (s as i64 + 1) + n);
            field.columns.position[slot as usize] = lane().point_at(front);
            squads.members[s].push(slot as i32);
        }
    }
    (squads, field)
}

fn looks() -> (Looks, PathTuning) {
    let (_, look, climb) = pond(false);
    (Looks { relook: 4.0, look }, climb)
}

/// The pass over every squad built, on `ground`.
fn run(squads: &mut Squads, field: &Field, painted: bool, tick: i64, log: &mut Log) -> Vec<usize> {
    let (ground, _, _) = pond(painted);
    let (looks, climb) = looks();
    let mut cache = Cache::default();
    let mut over = Over { ground: &ground, cache: &mut cache, tuning: climb };
    let listed: Vec<usize> = (0..squads.id.len()).collect();
    step(squads, field, &mut over, &listed, (tick, 7), &looks, log)
}

#[test]
fn a_look_reads_each_units_walker_wits_place_and_definitions_sight_where_they_live() {
    let (squads, mut field) = built(&[20.0]);
    let wits = field.lexicon.code("wits");
    field.columns.words[0].attributes = vec![(wits, 14.0)];
    field.columns.objects[0].put_walker(&[2.0, 1.0, 3.0, 1.0, 0.0]);
    field.columns.objects[0].definition = 1;
    field.put_sight(1, [40.0, 1.0, 0.5, 0.25]);
    let height = NAMES.iter().position(|n| *n == "height").unwrap();
    field.columns.sheet[1].set(height, 1.5);

    let read = members(&field, &squads, 0);

    assert_eq!((read[0].wits, read[0].id, read[0].at), (14, 100, lane().point_at(20.0)));
    let w = read[0].walker.walker;
    assert_eq!((w.height, w.swimmer, w.climber, w.swims, w.climbs), (2.0, 1, 3, true, false));
    assert_eq!(read[0].eyes, Some([40.0, 1.0, 0.5, 0.25]));
    assert_eq!((read[1].wits, read[1].id, read[1].eyes), (AVERAGE as i64, 101, None));
    assert_eq!((read[1].walker.walker.height, read[1].walker.walker.swims), (1.5, false));
    assert_eq!((read[1].walker.stamina, read[1].walker.speed), (f64::INFINITY, 1.0));
}

#[test]
fn a_barred_route_ahead_is_kept_on_the_command_and_reported_with_its_cells() {
    let (mut squads, field) = built(&[20.0]);
    let mut log = Log::default();

    assert!(run(&mut squads, &field, true, 3, &mut log).is_empty());

    assert_eq!(squads.commands.patches[0].len(), 1);
    let taken = log.take();
    assert_eq!(taken[..4], [events::DETOURED as i64, 3, 1, 0]);
    assert!(taken[4] >= 3 && taken.len() == 5);
    assert_eq!(squads.looked_at[0], 20.0);
}

#[test]
fn every_group_of_the_command_follows_the_patched_route_on_the_next_tick() {
    let (mut squads, field) = built(&[20.0, 10.0]);
    squads.state[1] = crate::locks::HOLDING; // it doesn't look for itself
    let mut log = Log::default();
    run(&mut squads, &field, true, 1, &mut log);

    assert_eq!(run(&mut squads, &field, true, 2, &mut log), vec![0, 1]);

    let patched = detours::command_route(&squads, 1).unwrap();
    assert_eq!(squads.route[1].as_ref(), Some(&patched));
    assert!(patched.length() > lane().length());
    assert_eq!(squads.patched_count, vec![1, 1]);
    let along = squads.front_distance[1] * CELLS;
    assert_eq!(squads.position[1], patched.point_at(along));
}

#[test]
fn an_open_route_or_a_group_that_has_not_moved_on_makes_no_detour() {
    let (mut squads, field) = built(&[20.0]);
    let mut log = Log::default();
    run(&mut squads, &field, false, 1, &mut log);
    run(&mut squads, &field, true, 2, &mut log); // still where it looked
    assert!(log.take().is_empty());
    assert!(squads.commands.patches[0].is_empty());
}
