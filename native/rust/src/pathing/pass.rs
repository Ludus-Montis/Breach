//! Pathing's pass (FormationPathing.step; spec 39 step 8b5-vii): groups follow their
//! commands' patched routes (`detours::follows`), then each marching group that has moved
//! on looks ahead (`detours::looks`, `look_ahead`) and keeps any way round it finds on its
//! command, reporting it as its "detoured" event, with the way's cells. A look reads each
//! living unit where it lives (`members`): its walker from its objects (a grounded one of
//! its height for a unit made without one), its wits from its attributes (AVERAGE unlisted),
//! its place from the store's columns and its definition's sight from the field's.

use super::{look_ahead, LookTuning, Member};
use crate::detours;
use crate::events::{self, Log};
use crate::field::Field;
use crate::geometry::CELLS;
use crate::ground::Walker;
use crate::paths::{Over, PathWalker};
use crate::squads::Squads;
use crate::wounds::AVERAGE;

/// The tuning the pass reads: BattleTuning's path_relook, then the look's.
pub struct Looks {
    pub relook: f64,
    pub look: LookTuning,
}

/// FormationPathing.step on `tick` over the squads in `listed` (their slots, in the lists'
/// order): every group re-routes onto its command's patched route where it has changed,
/// then every marching group that has moved on looks ahead; each way round kept goes to
/// `log` as its "detoured" event. Returns the slots that re-routed, in the lists' order.
pub fn step(
    squads: &mut Squads,
    field: &Field,
    over: &mut Over,
    listed: &[usize],
    (tick, seed): (i64, i64),
    looks: &Looks,
    log: &mut Log,
) -> Vec<usize> {
    let followed: Vec<usize> = listed.iter().copied().filter(|&s| follow(squads, s)).collect();
    for &s in listed {
        let routed = squads.route[s].is_some();
        if detours::looks(squads, s, routed, looks.relook) {
            look(squads, (field, &mut *over), s, (tick, seed), &looks.look, log);
        }
    }
    followed
}

/// The squad in `slot`'s living units as a look reads them, in its members' order.
pub fn members(field: &Field, squads: &Squads, slot: usize) -> Vec<Member> {
    let columns = &field.columns;
    let wits = field.lexicon.find("wits");
    let living = squads.living(slot, &columns.state);
    living
        .iter()
        .map(|&unit| {
            let u = unit as usize;
            let sheet = &columns.sheet[u];
            let objects = &columns.objects[u];
            let own = |code| columns.words[u].attributes.iter().find(|(c, _)| Some(*c) == code);
            Member {
                walker: PathWalker::of(walker(objects.walker, sheet.height)),
                wits: own(wits).map_or(AVERAGE, |&(_, level)| level) as i64,
                id: field.unit_at(unit as u32).unwrap_or(0),
                at: columns.position[u],
                eyes: field.sight(objects.definition),
            }
        })
        .collect()
}

/// A walker's fields (TerrainWalker.fields), or a grounded one `height` cells tall for none.
fn walker(fields: Option<[f64; 5]>, height: f64) -> Walker {
    let [height, swimmer, climber, swims, climbs] = fields.unwrap_or([height, 0.0, 0.0, 0.0, 0.0]);
    Walker {
        height,
        swimmer: swimmer as i64,
        climber: climber as i64,
        swims: swims != 0.0,
        climbs: climbs != 0.0,
    }
}

/// FormationPathing._follow: whether the squad in `slot` re-routes onto its command's route
/// with its ways round; if so it takes it, standing where it stood along it.
fn follow(squads: &mut Squads, slot: usize) -> bool {
    let patches = detours::command_patches(squads, slot);
    if !detours::follows(squads, slot, patches) {
        return false;
    }
    let route = detours::command_route(squads, slot).expect("a command's route");
    detours::follow(squads, slot, &route, patches.expect("a command's route"));
    squads.position[slot] = route.point_at(squads.front_distance[slot] * CELLS);
    squads.route[slot] = Some(route);
    true
}

/// FormationPathing._look_ahead: the squad in `s` looks along its route ahead and keeps a
/// way round on its command where one is better.
fn look(
    squads: &mut Squads,
    (field, over): (&Field, &mut Over),
    s: usize,
    (tick, seed): (i64, i64),
    tuning: &LookTuning,
    log: &mut Log,
) {
    let (Some(route), Some(command)) = (squads.route[s].as_ref(), squads.command[s]) else {
        return;
    };
    let along = (squads.front_distance[s] * CELLS, f64::from(squads.direction[s]));
    let members = members(field, squads, s);
    let Some(way) = look_ahead(over, route, along, &members, seed, tuning) else {
        return;
    };
    squads.commands.learn(command, &way);
    log.squad(events::DETOURED, tick, squads, s, &[way.len() as i64]);
}

#[cfg(test)]
mod tests;
