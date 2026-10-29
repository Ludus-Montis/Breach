//! Which groups follow their command's ways round and which look ahead (FormationPathing,
//! spec 39 step 8b3). A group re-routes onto its command's route, patched (`route::Patch`),
//! whenever the command has learned a way round it doesn't carry yet, standing where it
//! stood along the new route. A marching group with a route looks ahead once its front has
//! moved on `relook` cells or more since it last looked. The look itself (`pathing`) and
//! the commands' patches are the crate's (`commands`, step 8b4-iii); GDScript calls the look
//! between these.

#[cfg(test)]
mod tests;

use crate::geometry::CELLS;
use crate::locks::MOVING;
use crate::route::{Patch, Route};
use crate::squads::Squads;

/// FormationPathing._follow's test: whether the squad in `slot` re-routes onto its
/// command's route, which carries `patches` ways round (None: the command has no route).
pub fn follows(squads: &Squads, slot: usize, patches: Option<usize>) -> bool {
    patches.is_some_and(|count| squads.patched_count[slot] != count as i32)
}

/// FormationPathing._follow: the squad in `slot` takes `route`, its command's route with
/// its `patches` ways round, standing where it stood along it; a squad bound for the far
/// end keeps it as its home.
pub fn follow(squads: &mut Squads, slot: usize, route: &Route, patches: usize) {
    squads.patched_count[slot] = patches as i32;
    squads.front_distance[slot] = route.distance_of(squads.position[slot]) / CELLS;
    if squads.home_distance[slot] > 0.0 {
        squads.home_distance[slot] = route.length() / CELLS;
    }
}

/// FormationPathing._looks: whether the squad in `slot`, marching on a route (`routed`),
/// looks ahead, its front `relook` cells or more from where it last looked; it looks from
/// there.
pub fn looks(squads: &mut Squads, slot: usize, routed: bool, relook: f64) -> bool {
    let front = squads.front_distance[slot] * CELLS;
    let looks =
        squads.state[slot] == MOVING && routed && (front - squads.looked_at[slot]).abs() >= relook;
    if looks {
        squads.looked_at[slot] = front;
    }
    looks
}

/// How many ways round the command the squad in `slot` follows has learned, if it has a
/// route (none: no route, or no command).
pub fn command_patches(squads: &Squads, slot: usize) -> Option<usize> {
    let command = squads.command[slot]?;
    let commands = &squads.commands;
    commands.route[command].as_ref().map(|_| commands.patches[command].len())
}

/// The route the command the squad in `slot` follows takes, with its ways round (none: no
/// route, or no command).
pub fn command_route(squads: &Squads, slot: usize) -> Option<Route> {
    let command = squads.command[slot]?;
    let commands = &squads.commands;
    let route = commands.route[command].as_ref()?;
    Some(Patch::patched(route, &commands.patches[command]))
}
