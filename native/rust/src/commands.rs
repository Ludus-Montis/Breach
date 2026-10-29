//! The commands the squads follow (FormationCommand, spec 30 round 3; spec 39 step 8b4-iii):
//! each one's guide route (none: the straight lane), the end it calls home, its standing
//! options (hurrying, tending its own downed, merging, pursuing, taking) and the ways round
//! its groups have learned (`route::Patch`), in the order learned. A squad names the
//! command it follows (`Squads::command`). FormationCommand is a view of one.

#[cfg(test)]
mod tests;

use serde::{Deserialize, Serialize};

use crate::maths::V2;
use crate::route::{Patch, Route};

/// A command's yes-or-no orders, as `flags` keeps them (FormationCommand names the same;
/// hurrying and merging are read by GDScript only, till the tick ports).
#[allow(dead_code)]
pub const HURRY: u8 = 1;
#[allow(dead_code)]
pub const MERGES: u8 = 2;
pub const PURSUES: u8 = 4;
pub const TAKES: u8 = 8;

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Commands {
    pub route: Vec<Option<Route>>,
    /// Tiles along its route its home end lies at.
    pub home: Vec<f64>,
    pub flags: Vec<u8>,
    /// What it does with its own downed: "" leaves them, "recover", "carry".
    pub tends: Vec<String>,
    pub patches: Vec<Vec<Patch>>,
}

impl Commands {
    /// A new command calling `home` its home, on no route, pursuing and taking (as
    /// FormationCommand.new): its slot.
    pub fn open(&mut self, home: f64) -> usize {
        self.route.push(None);
        self.home.push(home);
        self.flags.push(PURSUES | TAKES);
        self.tends.push(String::new());
        self.patches.push(Vec::new());
        self.home.len() - 1
    }

    /// Keeps the way round `detour` (its waypoints, leaving and rejoining the route) on the
    /// command in `slot`, as a patch to its route; it has a route.
    pub fn learn(&mut self, slot: usize, detour: &[V2]) {
        let route = self.route[slot].as_ref().expect("a command learns ways round its route");
        let patch = Patch::of_detour(route, detour);
        self.patches[slot].push(patch);
    }

    /// Whether the command in `slot` gives `flag`.
    pub fn gives(&self, slot: usize, flag: u8) -> bool {
        self.flags[slot] & flag != 0
    }

    /// Gives, or stops giving, `flag`.
    pub fn give(&mut self, slot: usize, flag: u8, on: bool) {
        if on {
            self.flags[slot] |= flag;
        } else {
            self.flags[slot] &= !flag;
        }
    }
}
