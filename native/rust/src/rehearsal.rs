//! Planned rendezvous by rehearsal (FormationRehearsal, Decision 87, spec 27 round 8; spec 39
//! step 8c-x): a wave's march timed by marching its own units down its route in a battle of
//! their own with nothing else on the field, every pace, turn, narrowing and re-form as the
//! battle will run it, until something interferes (a fight, a queue).

#[cfg(test)]
mod tests;

use crate::battles::Set;
use crate::content::Content;
use crate::events::Log;
use crate::field::Field;
use crate::geometry::CELLS;
use crate::play::Play;
use crate::route::Route;
use crate::spawn::{self, Placed, Spawn};
use crate::squads::Squads;
use crate::waves::Painted;

/// No march is rehearsed for longer (FormationRehearsal.LIMIT_TICKS).
pub const LIMIT_TICKS: i64 = 20000;

/// A march to rehearse: the units at their places (front-first, as a wave's preview lists
/// its filled ones), the squad's width, its route and how far along it its front goes
/// (cells).
pub struct March<'a> {
    pub painted: &'a [Painted],
    pub width: i64,
    pub route: &'a Route,
    pub cells: f64,
}

/// FormationRehearsal.ticks_to: the ticks `march`'s squad takes to bring its front its cells
/// along its route, in a battle of its own of `tick_seconds` ticks on the map `map` (open
/// ground if None) from `content`, unseeded and its blows unrolled as a new
/// FormationSimulation's; 0 for no units, LIMIT_TICKS if it never gets there.
pub fn ticks_to(
    march: &March,
    (tick_seconds, map, threaded): (f64, Option<&'static str>, bool),
    content: &Content,
) -> i64 {
    if march.painted.is_empty() {
        return 0;
    }
    let (mut squads, mut field) = (Squads::default(), Field::default());
    let set = Set { route_tiles: march.route.length() / CELLS, tick_seconds, combat_width: 0, map };
    let mut play = Play::open(set, (0, false, threaded), content);
    let placed: Vec<Placed> = march
        .painted
        .iter()
        .map(|p| Placed {
            unit: &content.units[p.id],
            code: play.codes.definition(p.id),
            place: p.at,
        })
        .collect();
    let spawn = Spawn {
        width: march.width,
        placed: &placed,
        faction: "player",
        at_player_end: true,
        wait_ticks: 0,
        route: march.route,
        route_code: None,
        combat_width: 0,
    };
    let s = spawn::squad(&mut squads, &mut field, content, &spawn);
    play.listed.push(s);
    let mut log = Log::default();
    for tick in 0..LIMIT_TICKS {
        if squads.front_distance[s] * CELLS >= march.cells - 0.000_001 {
            return tick;
        }
        play.step(&mut squads, &mut field, content, &mut log);
        log.take();
    }
    LIMIT_TICKS
}
