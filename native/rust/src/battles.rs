//! The digest's battles set up in the crate (spec 39 step 8c-vi): BattleTrials' mirrors,
//! FormationBench.clash and the field the digest's field logs play on (FormationField's
//! kingdom line and reserve), each spawned by `spawn::squad` with every write GDScript's set-up
//! makes: its definitions coded in the order first met (UnitStore.definition_code), its
//! routes coded as first used, its holds queued and, on the field, each kingdom squad's front
//! set where it holds. The field's waves are built and sent by `field_play` (8c-viii).

use crate::content::Content;
use crate::field::Field;
use crate::geometry::CELLS;
use crate::maths::V2;
use crate::orders;
use crate::route::Route;
use crate::spawn::{self, Placed, Spawn};
use crate::squads::Squads;

#[cfg(test)]
mod tests;

/// SkirmishUnit.Order.HOLD.
const HOLD: i32 = 1;
/// BattleTrials.TICK, FormationBench's and FormationFieldActions' tick.
pub const TICK_SECONDS: f64 = 0.1;

/// A battle to set up.
pub enum Battle {
    /// BattleTrials' mirror: 8 grems a side, head-on along the lane or (`flank`) onto the side
    /// of a line holding its ground; `swap` spawns its sides the other way round.
    Mirror { flank: bool, swap: bool },
    /// FormationBench.clash: `per_side` grems (the last `archers` spitters) at the player's end
    /// against as many militia, `width` wide.
    Clash { per_side: usize, width: usize, archers: usize },
    /// FormationField's kingdom line (two ranks of six militia, `captained`: a captain in the
    /// second rank's middle) holding at LINE_AT, and its reserve of six on the hill.
    Field { captained: bool },
}

/// The battle `name` with its `numbers`: "mirror_headon" and "mirror_flank" [1 if swapped],
/// "clash" [per side, width, archers], "field" [1 if captained]; a number left out is 0. None
/// for a name that isn't one.
pub fn named(name: &str, numbers: &[i64]) -> Option<Battle> {
    let n = |k: usize| numbers.get(k).copied().unwrap_or(0).max(0);
    Some(match name {
        "mirror_headon" | "mirror_flank" => {
            Battle::Mirror { flank: name == "mirror_flank", swap: n(0) != 0 }
        }
        "clash" => {
            Battle::Clash { per_side: n(0) as usize, width: n(1) as usize, archers: n(2) as usize }
        }
        "field" => Battle::Field { captained: n(0) != 0 },
        _ => return None,
    })
}

/// What the simulation is made with (FormationSimulation.new's route length, in tiles, and
/// tick; its combat width; the ground's map, if any, by `ground::map::of`'s name).
#[derive(Debug, PartialEq)]
pub struct Set {
    pub route_tiles: f64,
    pub tick_seconds: f64,
    pub combat_width: i64,
    pub map: Option<&'static str>,
}

/// FormationField's measures, in cells.
const FIELD_SIZE: f32 = 128.0;
const LINE_AT: f64 = 80.0;
const RESERVE_AT: f64 = 96.0;
const KINGDOM_WIDTH: usize = 6;
const WAVE_WIDTH: i64 = 8;

/// One squad to spawn: [width, its units' ids at their places (rank, column), its faction, at
/// the player's end], on its route (a key into the battle's routes).
struct Side {
    width: usize,
    units: Vec<(&'static str, (i32, i32))>,
    faction: &'static str,
    at_player_end: bool,
    route: usize,
}

/// The codes the battle gives: definitions in the order first met, routes as first used.
#[derive(Default)]
pub(crate) struct Codes {
    pub(crate) definitions: Vec<&'static str>,
    routes: Vec<(usize, i64)>,
}

impl Codes {
    /// The code `id` goes by, given it now if it is met for the first time.
    pub(crate) fn definition(&mut self, id: &'static str) -> i64 {
        let found = self.definitions.iter().position(|&d| d == id);
        found.unwrap_or_else(|| {
            self.definitions.push(id);
            self.definitions.len() - 1
        }) as i64
    }

    fn route(&mut self, squads: &mut Squads, key: usize) -> i64 {
        if let Some(&(_, code)) = self.routes.iter().find(|(k, _)| *k == key) {
            return code;
        }
        let code = squads.issue_route();
        self.routes.push((key, code));
        code
    }
}

/// Sets `battle` up in `squads` and `field` from `content`'s definitions; None, with nothing
/// written, if one of its definitions isn't there.
pub fn set_up(
    battle: &Battle,
    squads: &mut Squads,
    field: &mut Field,
    content: &Content,
) -> Option<Set> {
    set_up_coded(battle, squads, field, content).map(|(set, _)| set)
}

/// `set_up`, with the codes it gave, for what is spawned later in the battle.
pub(crate) fn set_up_coded(
    battle: &Battle,
    squads: &mut Squads,
    field: &mut Field,
    content: &Content,
) -> Option<(Set, Codes)> {
    let (set, routes, sides) = match battle {
        Battle::Mirror { flank, swap } => mirror(*flank, *swap),
        Battle::Clash { per_side, width, archers } => clash(*per_side, *width, *archers),
        Battle::Field { captained } => kingdom(*captained),
    };
    let known = |side: &Side| side.units.iter().all(|(id, _)| content.units.contains_key(*id));
    if !sides.iter().all(|(side, _)| known(side)) {
        return None;
    }
    let mut codes = Codes::default();
    for (side, then) in &sides {
        let placed: Vec<Placed> = side
            .units
            .iter()
            .map(|&(id, place)| Placed {
                unit: &content.units[id],
                code: codes.definition(id),
                place,
            })
            .collect();
        let route = &routes[side.route];
        let spawn = Spawn {
            width: side.width as i64,
            placed: &placed,
            faction: side.faction,
            at_player_end: side.at_player_end,
            wait_ticks: 0,
            route,
            route_code: Some(codes.route(squads, side.route)),
            combat_width: set.combat_width,
        };
        let s = spawn::squad(squads, field, content, &spawn);
        if let Some(front) = then.front {
            squads.front_distance[s] = front; // StoredSquad's front_distance, then _place
            squads.position[s] = route.point_at(front * CELLS);
        }
        if then.holds {
            orders::queue(squads, squads.id[s], HOLD);
        }
    }
    Some((set, codes))
}

/// What is done with a squad once it is spawned: its front set (tiles), and a hold ordered.
#[derive(Clone, Copy, Default)]
struct Then {
    front: Option<f64>,
    holds: bool,
}

type Plan = (Set, Vec<Route>, Vec<(Side, Then)>);

/// The simulation's own route, `tiles` long (FormationRoute.straight).
fn lane(tiles: f64) -> Route {
    Route::new(vec![V2::new(0.0, 0.0), V2::new((tiles * CELLS) as f32, 0.0)], 0.0)
}

fn open(map: Option<&'static str>, combat_width: i64) -> Set {
    Set { route_tiles: 2.0, tick_seconds: TICK_SECONDS, combat_width, map }
}

/// `count` of `id` in a row.
fn row(id: &'static str, count: usize) -> Vec<(&'static str, (i32, i32))> {
    (0..count).map(|column| (id, (0, column as i32))).collect()
}

/// BattleTrials._mirror_spawns.
fn mirror(flank: bool, swap: bool) -> Plan {
    let side = |faction, at_player_end, route| Side {
        width: 8,
        units: row("grem", 8),
        faction,
        at_player_end,
        route,
    };
    let (routes, mut sides) = if flank {
        let hold = Route::new(vec![V2::new(64.0, 32.0), V2::new(0.0, 32.0)], 0.0);
        let down = Route::new(vec![V2::new(64.5, 0.0), V2::new(64.5, 64.0)], 0.0);
        let holds = Then { holds: true, ..Then::default() };
        let sides =
            vec![(side("the_kingdom", true, 0), holds), (side("player", true, 1), Then::default())];
        (vec![hold, down], sides)
    } else {
        let sides = vec![
            (side("player", true, 0), Then::default()),
            (side("the_kingdom", false, 0), Then::default()),
        ];
        (vec![lane(2.0)], sides)
    };
    if swap {
        sides.reverse(); // the other side first: outcomes must not move (Decision 97)
    }
    (open(None, 0), routes, sides)
}

/// FormationBench.clash (its `_spawn_pair` on the simulation's own route).
fn clash(per_side: usize, width: usize, archers: usize) -> Plan {
    let width = width.max(1);
    let side = |unit: &'static str, faction, at_player_end| {
        let units = (0..per_side)
            .map(|k| {
                let shoots = unit == "grem" && k + archers >= per_side;
                let id = if shoots { "grem_spitter" } else { unit };
                (id, ((k / width) as i32, (k % width) as i32))
            })
            .collect();
        (Side { width, units, faction, at_player_end, route: 0 }, Then::default())
    };
    let sides = vec![side("grem", "player", true), side("kingdom_militia", "the_kingdom", false)];
    (open(None, 0), vec![lane(2.0)], sides)
}

/// FormationField._hold_the_line on route A, the field's 2 tiles across at y 32, 4 cells
/// either side.
fn kingdom(captained: bool) -> Plan {
    let a =
        Route::new(vec![V2::new(0.0, 32.0), V2::new(FIELD_SIZE, 32.0)], WAVE_WIDTH as f64 / 2.0);
    let mut line = Vec::new();
    for rank in 0..2_i32 {
        for column in 0..KINGDOM_WIDTH {
            let leads = captained && rank == 1 && column == KINGDOM_WIDTH / 2;
            let id = if leads { "kingdom_captain" } else { "kingdom_militia" };
            line.push((id, (rank, column as i32)));
        }
    }
    let side = |units, at: f64| {
        let then = Then { front: Some(at / CELLS), holds: true };
        (
            Side {
                width: KINGDOM_WIDTH,
                units,
                faction: "the_kingdom",
                at_player_end: false,
                route: 0,
            },
            then,
        )
    };
    let sides = vec![side(line, LINE_AT), side(row("kingdom_militia", KINGDOM_WIDTH), RESERVE_AT)];
    (open(Some("field"), WAVE_WIDTH), vec![a], sides)
}
