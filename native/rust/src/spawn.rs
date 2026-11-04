//! A squad spawned in the crate from its units' definitions (spec 39 step 8c-iv; GDScript's
//! FormationSimulation.spawn_squad): its units' ids issued, its command opened on its route,
//! its squad opened, each unit made from its definition (`made::unit`) and admitted with its
//! sheet, words, walker and objects, placed at full hp with all its regeneration to come, at
//! full stamina and at its pace fresh (FormationUnits.enter); then the squad on its route,
//! facing along it, with its wait, its morale at its ceiling and its combat width, and its
//! units stood on their places (`march::sync`, placing). Every write is the one GDScript's
//! spawn makes, in its order where the order shows: the faction is coded before the units'
//! words, and each unit's words as it is admitted.

use crate::bands::{self, BandTuning};
use crate::content::{Content, Unit};
use crate::field::Field;
use crate::geometry::CELLS;
use crate::made;
use crate::march;
use crate::route::Route;
use crate::squads::Squads;
use crate::tick::{given, rules};

#[cfg(test)]
mod tests;

/// A unit to spawn: its definition, the code its definition goes by (UnitStore's
/// definition_code: the order definitions were first met), and its place in the squad's
/// frame, (rank, column).
pub struct Placed<'a> {
    pub unit: &'a Unit,
    pub code: i64,
    pub place: (i32, i32),
}

/// A squad to spawn (FormationSimulation.spawn_squad's arguments, and the battle's combat
/// width).
pub struct Spawn<'a> {
    pub width: i64,
    pub placed: &'a [Placed<'a>],
    pub faction: &'a str,
    pub at_player_end: bool,
    pub wait_ticks: i64,
    pub route: &'a Route,
    /// The code its route goes by, or None for a route of its own (`issue_route`).
    pub route_code: Option<i64>,
    pub combat_width: i64,
}

/// Spawns `spawn` from `content`'s definitions; returns the squad's slot.
pub fn squad(squads: &mut Squads, field: &mut Field, content: &Content, spawn: &Spawn) -> usize {
    let direction = if spawn.at_player_end { 1 } else { -1 };
    let home = if spawn.at_player_end { 0.0 } else { spawn.route.length() / CELLS };
    let first = squads.issue_units(spawn.placed.len() as i64);
    let command = squads.commands.open(home);
    squads.commands.route[command] = Some(spawn.route.clone());
    let id = squads.issue_squad();
    let s = squads.open(id) as usize;
    squads.command[s] = Some(command);
    squads.faction[s] = field.lexicon.code(spawn.faction);
    squads.direction[s] = direction;
    squads.heading[s] = if direction > 0 { 90.0 } else { 270.0 }; // SquadFrame.EAST, WEST
    squads.home_distance[s] = home;
    squads.front_distance[s] = home;
    squads.width[s] = spawn.width.max(1) as i32;
    let mut members = Vec::with_capacity(spawn.placed.len());
    for (k, placed) in spawn.placed.iter().enumerate() {
        let slot = admit(field, content, placed, (first + k as i64, id), (spawn, command));
        enter(field, placed, slot);
        members.push(slot as i32);
    }
    squads.route[s] = Some(spawn.route.clone());
    squads.route_code[s] = spawn.route_code.unwrap_or_else(|| squads.issue_route());
    squads.position[s] = spawn.route.point_at(home * CELLS);
    march::sweep::face_held(squads, s);
    squads.wait_ticks[s] = spawn.wait_ticks as i32;
    squads.morale[s] = ceiling(field, &members, content) as i32;
    squads.members[s] = members;
    squads.combat_width[s] = spawn.combat_width as i32;
    stand(squads, field, content, s);
    s
}

/// FormationMarch.sync_units([squad]): the units of the squad in slot `s` stood on their
/// places.
pub fn stand(squads: &Squads, field: &mut Field, content: &Content, s: usize) {
    let walk = given::walk(&content.tuning);
    let given = march::sync::Given {
        ground: None,
        timing: None,
        seed: 0,
        wounds_reach: content.tuning.wounds_reach,
        tuning: rules::walk_tuning_of(&walk),
    };
    march::sync::step(squads, field, &[s], &given);
}

/// The unit `ids` = (its id, its squad's) admitted (UnitStore.admit, then
/// FormationCommand.enlist): its sheet, words and walker as its definition makes it, its
/// definition's code with its sight and speed kept, its squad and its command, under which
/// it started. Returns its slot.
fn admit(
    field: &mut Field,
    content: &Content,
    placed: &Placed,
    (unit_id, squad_id): (i64, i64),
    (spawn, command): (&Spawn, usize),
) -> usize {
    let unit = placed.unit;
    let direction = if spawn.at_player_end { 1 } else { -1 };
    let slot = field.hold(unit_id) as usize;
    let made = made::unit(content, unit, spawn.faction, direction, &mut field.lexicon);
    let columns = &mut field.columns;
    columns.sheet[slot] = made.sheet;
    columns.sheet[slot].squad_id = squad_id;
    columns.words[slot] = made.words;
    let objects = &mut columns.objects[slot];
    objects.put_walker(&made.walker);
    for (name, code) in [("command", command as i64), ("origin", command as i64)] {
        *objects.code(name).expect("a unit's code") = code;
    }
    *objects.code("definition").expect("a unit's code") = placed.code;
    columns.state[slot] = 0; // MOVING (UnitState.admitted)
    let code = placed.code as usize;
    let sight = [unit.detection_range, unit.sight_ahead, unit.sight_side, unit.sight_behind];
    field.put_sight(code, sight);
    field.put_speed(code, unit.speed);
    slot
}

/// FormationUnits.enter: the unit in `slot` at its place, its full hp, all the hp it may
/// regenerate, its full stamina and its pace fresh.
fn enter(field: &mut Field, placed: &Placed, slot: usize) {
    let columns = &mut field.columns;
    let hp = placed.unit.hp;
    columns.rank[slot] = placed.place.0;
    columns.column[slot] = placed.place.1;
    columns.hp[slot] = hp as i32;
    columns.regeneration_left[slot] = placed.unit.regeneration_limit * hp as f64;
    columns.stamina[slot] = columns.sheet[slot].max_stamina;
    columns.speed[slot] = columns.sheet[slot].fresh_speed;
}

/// FormationMorale.ceiling over the units in `members`, all standing.
fn ceiling(field: &Field, members: &[i32], content: &Content) -> i64 {
    let sheets = members.iter().map(|&m| &field.columns.sheet[m as usize]);
    let (courage, leaderships): (Vec<i64>, Vec<i64>) =
        sheets.map(|sheet| (sheet.courage, sheet.leadership)).unzip();
    let tuning = BandTuning::of_slice(&given::bands(&content.tuning));
    bands::ceiling(&courage, &leaderships, &tuning)
}
