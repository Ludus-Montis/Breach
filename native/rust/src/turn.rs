//! Turning as a re-form (ScrumTurn and SquadTurn.reverse_ranks, Decisions 74 and 92, spec 27
//! round 6): a squad that must face another way where it stands - going back the way it
//! faces, or re-forming after a withdrawal or a pursuit - takes its new heading at once, an
//! about-face reversing its ranks, and its units walk from where they stand to their new
//! places at its re-form pace. Every unit in its frame is loosened where it stands; the
//! re-form brings it back (FormationScrum). Landed (`landed`), an about-face cancels the
//! moves under way (its swaps) and the turn's events go to the log.

use crate::columns::Columns;
use crate::entries;
use crate::events::{self, Log};
use crate::geometry::{CELLS, RANK_DEPTH};
use crate::maths::{is_equal_approx, V2};
use crate::motion::{bearing_to, fposmod};
use crate::roster::Listed;
use crate::route::Route;
use crate::squads::Squads;

/// begin's answer: the squad kept its heading.
pub const KEPT: i64 = 0;
/// It turned where it stands.
pub const TURNED: i64 = 1;
/// It turned about first, its ranks reversed.
pub const ABOUT_FACED: i64 = 2;

/// ScrumTurn.begin: re-faces `listed`'s squad where it stands before it moves `travel` (+1
/// or -1) along its route: an about-face first if it must go back the way it faces, then
/// its frame turned to its route's heading there (`route`; none: a straight lane, heading
/// kept). Unless it kept its heading, its units in its frame are loosened where they stand.
pub fn begin(
    squads: &mut Squads,
    columns: &mut Columns,
    listed: &Listed,
    travel: i32,
    route: Option<&Route>,
) -> i64 {
    let s = listed.squad;
    let before = squads.heading[s];
    let about = travel != squads.direction[s];
    if about {
        let depth = reverse_ranks(squads, columns, listed);
        squads.front_distance[s] -= (squads.direction[s] * depth) as f64 * RANK_DEPTH;
        squads.position[s] = place(route, squads.front_distance[s] * CELLS);
        squads.direction[s] = -squads.direction[s];
        squads.heading[s] = fposmod(squads.heading[s] + 180.0, 360.0);
    }
    if let Some(route) = route {
        let cells = squads.front_distance[s] * CELLS;
        let way = route.heading_at(cells).scale(squads.direction[s] as f64);
        squads.heading[s] = bearing_to(V2::ZERO, way, squads.heading[s]);
    }
    if is_equal_approx(squads.heading[s], before) && travel == squads.direction[s] {
        return KEPT;
    }
    for unit in &listed.living {
        if !squads.loose[s].contains(&unit.id) {
            let stood = columns.position[unit.slot];
            entries::loosen(squads, columns, s, (unit.id, unit.slot), stood);
        }
    }
    if about {
        ABOUT_FACED
    } else {
        TURNED
    }
}

/// ScrumTurn.begin landed: `begin` with the route the squad holds; an about-face cancels
/// its swaps under way, and a turn logs "turning" and "turned" (its facing, its heading's
/// bits) on `tick`. True if it turned: its units then re-form before it moves.
pub fn landed(
    squads: &mut Squads,
    columns: &mut Columns,
    listed: &Listed,
    travel: i32,
    tick: i64,
    log: &mut Log,
) -> bool {
    let s = listed.squad;
    let route = squads.route[s].clone();
    let turned = begin(squads, columns, listed, travel, route.as_ref());
    if turned == KEPT {
        return false;
    }
    if turned == ABOUT_FACED {
        squads.swaps[s].clear();
    }
    let faced = [facing_of(squads.heading[s]), squads.heading[s].to_bits() as i64];
    log.squad(events::TURNING, tick, squads, s, &faced);
    log.squad(events::TURNED, tick, squads, s, &faced);
    true
}

/// SkirmishSquad.facing: the four-way facing nearest a heading (an exact diagonal rounds up).
pub fn facing_of(heading: f64) -> i64 {
    ((heading / 90.0).round() as i64).rem_euclid(4)
}

/// StoredSquad's place: the point `cells` along its route, or along a straight lane on x.
pub(crate) fn place(route: Option<&Route>, cells: f64) -> V2 {
    route.map_or(V2::new(cells as f32, 0.0), |route| route.point_at(cells))
}

/// SquadTurn.reverse_ranks: turns the squad's places half round - ranks counted from the
/// old back, columns from the old right, the centre shift flipped with them - and sets it
/// re-forming. Returns its depth in ranks, how far its front moves back.
pub fn reverse_ranks(squads: &mut Squads, columns: &mut Columns, listed: &Listed) -> i32 {
    let s = listed.squad;
    let mut depth = 0;
    for unit in &listed.living {
        depth = depth.max(columns.rank[unit.slot] + unit.foot.1 as i32);
    }
    let width = squads.width[s];
    for unit in &listed.living {
        let (rank, column) = (columns.rank[unit.slot], columns.column[unit.slot]);
        columns.rank[unit.slot] = depth - rank - unit.foot.1 as i32;
        columns.column[unit.slot] = width - column - unit.foot.0 as i32;
    }
    squads.centre_shift[s] = -squads.centre_shift[s];
    squads.reforming[s] = 1;
    depth
}

#[cfg(test)]
mod tests;
