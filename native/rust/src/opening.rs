//! The tick's opening as one crate pass (FormationSimulation's `_apply_orders`, `_engage`,
//! FormationPathing.step and `_move`; spec 39 step 8b5-xxxv): the orders queued apply
//! (`orders::pass`), each squad sees (`perception::see`), the fronts and then the edges lock
//! (`contact::pass`), the groups find their way (`pathing::pass`, on ground only), and the
//! squads move and join (`moving::pass`, `joins::step`), all over one roster, every pass's
//! events in the log in that order. The routes GDScript holds are objects still, so each
//! listed squad's route comes as a code (the same code, the same route; -1 none), and this
//! keeps the codes as GDScript's mirrors would leave them: a pursuer that took its quarry's
//! route holds it (and its code), and a group that re-routed holds a route of its own.

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

/// The opening over `roster`'s squads, each listed squad's route coded in `codes`; `over`
/// the ground and its paths (none: open ground, where no group looks).
pub fn step(
    squads: &mut Squads,
    field: &mut Field,
    roster: &Roster,
    mut over: Option<&mut Over>,
    (rules, mut codes): (&Rules, Vec<i64>),
    log: &mut Log,
) -> Opened {
    let (tick, seed) = (rules.tick, rules.seed);
    let pursuers = order(squads, field, roster, (rules, &mut codes), log);
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
    rerouted(&mut codes, roster, &followed);
    let routes: Vec<u32> = codes.iter().map(|&code| code as u32).collect();
    let given = Given { routes: &routes, ..rules.moving };
    let ground = over.as_ref().map(|o| o.ground);
    let joiners = moving::step(squads, field, roster, ground, &given, log);
    let joins = joins::step(squads, field, roster, &joiners, seed, tick, log);
    Opened { pursuers, followed, seen, joins }
}

/// The orders' pass, its routes read only when an order retreats; each pursuer that took
/// its quarry's route holds it, and its code, as `enemy.route = quarry.route` left it.
fn order(
    squads: &mut Squads,
    field: &mut Field,
    roster: &Roster,
    (rules, codes): (&Rules, &mut [i64]),
    log: &mut Log,
) -> Vec<[i64; 4]> {
    if squads.pending.is_empty() {
        return Vec::new();
    }
    let retreats = squads.pending.iter().any(|&(_, order)| order == locks::RETREAT);
    let given = ordering::Given {
        routes: if retreats { coded(squads, roster, codes) } else { Vec::new() },
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
    for &[quarry, enemy, _, took] in &flat {
        if took != 0 {
            let (quarry, enemy) = (quarry as usize, enemy as usize);
            let route = squads.route[roster.listed[quarry].squad].clone();
            squads.route[roster.listed[enemy].squad] = route;
            codes[enemy] = codes[quarry];
        }
    }
    flat
}

/// Each listed squad's route as a retreat reads it: its code and the route the crate holds
/// (none for -1).
pub fn coded(squads: &Squads, roster: &Roster, codes: &[i64]) -> Vec<Coded> {
    let held = |n: usize| squads.route[roster.listed[n].squad].clone();
    codes
        .iter()
        .enumerate()
        .map(|(n, &code)| (code, if code >= 0 { held(n) } else { None }))
        .collect()
}

/// A group that re-routed holds a route of its own (FormationPathing._follow makes a new
/// one), so each takes a code no other route has.
fn rerouted(codes: &mut [i64], roster: &Roster, followed: &[usize]) {
    let mut fresh = codes.len() as i64;
    for &slot in followed {
        if let Some(n) = roster.listed.iter().position(|l| l.squad == slot) {
            codes[n] = fresh;
            fresh += 1;
        }
    }
}

#[cfg(test)]
mod tests;
