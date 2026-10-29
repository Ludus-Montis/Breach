//! The tick's opening as one crate pass (FormationSimulation's `_apply_orders`, `_engage`,
//! FormationPathing.step and `_move`; spec 39 step 8b5-xxxv): the orders queued apply
//! (`orders::pass`), each squad sees (`perception::see`), the fronts and then the edges lock
//! (`contact::pass`), the groups find their way (`pathing::pass`, on ground only), and the
//! squads move and join (`moving::pass`, `joins::step`), all over one roster, every pass's
//! events in the log in that order. Each squad's route is named by the code the crate holds
//! (`Squads::route_code`, spec 39 step 8b5-xxxvi; the same code, the same route; -1 none): a
//! pursuer that took its quarry's route holds it and its code, and a group that re-routed a
//! code of its own (`pathing::pass`).

use crate::bands::BandTuning;
use crate::contact::pass::{self as contact, Impacts};
use crate::discipline::DisciplineTuning;
use crate::events::Log;
use crate::field::Field;
use crate::joins;
use crate::locks;
use crate::moving::pass::{self as moving, Given};
use crate::orders::{pass as ordering, Coded};
use crate::pathing::pass::{self as pathing, Looks};
use crate::paths::Over;
use crate::perception;
use crate::roster::Roster;
use crate::squads::Squads;

/// Every pass's tuning: the tick and seed, the orders' (`pursuit_tempted_within`,
/// `pursuit_ragged_shock`, the discipline), morale's bands, the edges' impacts, the looks
/// and the move's (its routes left empty: this fills them in).
pub struct Rules<'a> {
    pub tick: i64,
    pub seed: i64,
    pub within: f64,
    pub ragged_shock: i64,
    pub discipline: DisciplineTuning,
    pub bands: BandTuning,
    pub impacts: Impacts,
    pub looks: Looks,
    pub moving: Given<'a>,
}

/// What GDScript mirrors after: four numbers an enemy that pursues a retreat ([quarry,
/// enemy] listed, 1 if it began now, 1 if it took its quarry's route), the slots that
/// re-routed, what each listed squad sees (listed indices) and the joins (leader, joining:
/// listed).
#[derive(Debug, Default, PartialEq)]
pub struct Opened {
    pub pursuers: Vec<[i64; 4]>,
    pub followed: Vec<usize>,
    pub seen: Vec<Vec<usize>>,
    pub joins: Vec<(usize, usize)>,
}

/// The opening over `roster`'s squads; `over` the ground and its paths (none: open ground,
/// where no group looks).
pub fn step(
    squads: &mut Squads,
    field: &mut Field,
    roster: &Roster,
    mut over: Option<&mut Over>,
    rules: &Rules,
    log: &mut Log,
) -> Opened {
    let (tick, seed) = (rules.tick, rules.seed);
    let pursuers = order(squads, field, roster, rules, log);
    let ground = over.as_ref().map(|o| o.ground);
    let seen = perception::see(squads, &field.columns, roster, ground);
    contact::engage_fronts(squads, &mut field.columns, roster, (tick, seed), log);
    let shocks = (&rules.impacts, &rules.bands);
    contact::engage_edges(squads, &mut field.columns, roster, (tick, seed), shocks, log);
    let followed = match over.as_deref_mut() {
        Some(over) => {
            let listed: Vec<usize> = roster.listed.iter().map(|l| l.squad).collect();
            pathing::step(squads, field, over, &listed, (tick, seed), &rules.looks, log)
        }
        None => Vec::new(),
    };
    let routes = routes(squads, roster);
    let given = Given { routes: &routes, ..rules.moving };
    let ground = over.as_ref().map(|o| o.ground);
    let joiners = moving::step(squads, field, roster, ground, &given, log);
    let joins = joins::step(squads, field, roster, &joiners, seed, tick, log);
    Opened { pursuers, followed, seen, joins }
}

/// The orders' pass, its routes read only when an order retreats; each pursuer that took
/// its quarry's route holds it, and its code (`took`).
fn order(
    squads: &mut Squads,
    field: &mut Field,
    roster: &Roster,
    rules: &Rules,
    log: &mut Log,
) -> Vec<[i64; 4]> {
    if squads.pending.is_empty() {
        return Vec::new();
    }
    let retreats = squads.pending.iter().any(|&(_, order)| order == locks::RETREAT);
    let given = ordering::Given {
        routes: if retreats { coded(squads, roster) } else { Vec::new() },
        within: rules.within,
        ragged_shock: rules.ragged_shock,
        discipline: &rules.discipline,
        bands: &rules.bands,
        tick: rules.tick,
        seed: rules.seed,
    };
    let applied = ordering::pass(squads, &mut field.columns, roster, given, log);
    let mut flat = Vec::new();
    for a in applied {
        for p in a.retreat.unwrap_or_default() {
            flat.push([a.listed as i64, p.listed as i64, p.began as i64, p.took as i64]);
        }
    }
    took(squads, roster, &flat);
    flat
}

/// Each pursuer in `pursuers` (as `order` hands them back) that took its quarry's route
/// holds it and its code, as `enemy.route = quarry.route` leaves GDScript's.
pub fn took(squads: &mut Squads, roster: &Roster, pursuers: &[[i64; 4]]) {
    for &[quarry, enemy, _, took] in pursuers {
        if took != 0 {
            let quarry = roster.listed[quarry as usize].squad;
            let enemy = roster.listed[enemy as usize].squad;
            squads.route[enemy] = squads.route[quarry].clone();
            squads.route_code[enemy] = squads.route_code[quarry];
        }
    }
}

/// Each listed squad's route as a retreat reads it: its code and the route the crate holds
/// (none for -1).
pub fn coded(squads: &Squads, roster: &Roster) -> Vec<Coded> {
    let held = |s: usize| squads.route[s].clone();
    let code = |s: usize| squads.route_code[s];
    let listed = roster.listed.iter().map(|l| l.squad);
    listed.map(|s| (code(s), if code(s) >= 0 { held(s) } else { None })).collect()
}

/// Each listed squad's route code as the move reads it (staging's partners: the same code,
/// the same route).
pub fn routes(squads: &Squads, roster: &Roster) -> Vec<u32> {
    let codes = squads.route_codes(roster.listed.iter().map(|l| l.squad));
    codes.into_iter().map(|code| code as u32).collect()
}

#[cfg(test)]
mod tests;
