//! Blows on routers (RoutBlows, Decision 82; spec 39 step 7a): a hostile front-rank unit free
//! to fight that comes within rout_strike_reach of a router strikes it from behind, a flank
//! blow: the nearest router it reaches, of whichever routing formation, a tie going to the
//! routers' seeded draws (Decision 97), once a tick however many it reaches. It takes that
//! router as its target, its cooldown runs down, and at 0 it strikes and starts again.
//! GDScript picks the fighters (its formations free to fight, their front ranks), hands in
//! each one's cooldown, and makes the blows (their damage) from what this returns.

use crate::columns::Columns;
use crate::contest;
use crate::grid::{Grid, MARGIN};
use crate::maths::{snapped, V2};
use crate::roster::Roster;
use crate::route::Route;

/// A routing formation: its index in the roster and its route (None: none, its routers on
/// the line y = 0).
pub struct Routing {
    pub listed: usize,
    pub route: Option<Route>,
}

/// A fighter that may strike a router: its formation's index in the roster, its store slot
/// and the cooldown it starts again at once it strikes.
#[derive(Clone, Copy, Debug)]
pub struct Fighter {
    pub listed: usize,
    pub slot: usize,
    pub reset: i32,
}

/// A router: its formation's index in `routing`, its index in that formation's living, its
/// id, faction and where it stands.
struct Router {
    routing: usize,
    member: usize,
    id: i64,
    faction: u32,
    at: V2,
}

/// Where a router of a formation on `route` stands (FormationRout.where_in): its point that
/// far along its route, plus its offset from it.
pub fn where_on(route: Option<&Route>, along: f64, offset: V2) -> V2 {
    let on = match route {
        Some(route) => route.point_at(along),
        None => V2::new(along as f32, 0.0),
    };
    on.add(offset)
}

/// This tick's blows on routers, in the fighters' order: [(the fighter's index in
/// `fighters`, the router's formation's index in `routing`, its index in that one's living),
/// ...]. Sets each fighter's target and runs its cooldown down.
pub fn blows(
    columns: &mut Columns,
    roster: &Roster,
    routing: &[Routing],
    fighters: &[Fighter],
    reach: f64,
    seed: i64,
) -> Vec<(usize, usize, usize)> {
    let mut routers = Vec::new();
    for (r, formation) in routing.iter().enumerate() {
        let listed = &roster.listed[formation.listed];
        for (member, m) in listed.living.iter().enumerate() {
            let at = where_on(
                formation.route.as_ref(),
                columns.along[m.slot],
                columns.flee_offset[m.slot],
            );
            routers.push(Router { routing: r, member, id: m.id, faction: listed.faction, at });
        }
    }
    let mut out = Vec::new();
    if routers.is_empty() {
        return out;
    }
    let grid = Grid::build(&routers.iter().map(|r| r.at).collect::<Vec<_>>());
    for (f, fighter) in fighters.iter().enumerate() {
        let here = columns.position[fighter.slot];
        let faction = roster.listed[fighter.listed].faction;
        let mut best: Option<((f64, i64), usize)> = None;
        grid.near(here, reach + MARGIN, |i| {
            let router = &routers[i as usize];
            if router.faction == faction {
                return true;
            }
            let gap = here.distance_to(router.at) as f64;
            let key = (snapped(gap, 0.000001), contest::draw(seed, router.id));
            if gap <= reach && best.is_none_or(|(held, _)| key < held) {
                best = Some((key, i as usize));
            }
            true
        });
        let Some((_, i)) = best else { continue };
        let router = &routers[i];
        columns.target_id[fighter.slot] = router.id;
        columns.attack_cooldown[fighter.slot] -= 1;
        if columns.attack_cooldown[fighter.slot] <= 0 {
            columns.attack_cooldown[fighter.slot] = fighter.reset;
            out.push((f, router.routing, router.member));
        }
    }
    out
}

#[cfg(test)]
mod tests;
