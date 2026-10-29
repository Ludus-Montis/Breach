//! The orders' pass (FormationOrders.apply; spec 39 step 8b5-v): the orders queued apply
//! (`apply`), reading for a retreat whether each squad may pursue (its command's) and its
//! living units' chances to break ranks and leashes (`discipline`, from their sheets), and
//! land here: each order applied is an "order_applied" event; a retreat out of a fight is
//! "disengaged", a ragged one's shock (`shocks`, by how far short of drilled it is),
//! "withdrawing", then each enemy's "pursuing". A pursuer keeps the route it held its post
//! on here (`post_route`, step 8b5-xvi); the route it takes stays GDScript's to land, from
//! what this returns, as the routes GDScript holds are objects.

use super::{apply, Applied, Coded, Retreats};
use crate::bands::BandTuning;
use crate::columns::Columns;
use crate::commands::PURSUES;
use crate::discipline::{self, DisciplineTuning};
use crate::events::{self, Log};
use crate::locks;
use crate::roster::Roster;
use crate::shocks;
use crate::squads::Squads;

/// What the pass reads besides the queue: each listed squad's route as a retreat reads it
/// (none needed when no order retreats), `pursuit_tempted_within`, `pursuit_ragged_shock`,
/// the tunings, the tick and the battle's seed.
pub struct Given<'a> {
    pub routes: Vec<Coded>,
    pub within: f64,
    pub ragged_shock: i64,
    pub discipline: &'a DisciplineTuning,
    pub bands: &'a BandTuning,
    pub tick: i64,
    pub seed: i64,
}

/// Applies the queue, which it takes, and lands each order; its events go to `log`.
pub fn pass(
    squads: &mut Squads,
    columns: &mut Columns,
    roster: &Roster,
    given: Given,
    log: &mut Log,
) -> Vec<Applied> {
    let queued = std::mem::take(&mut squads.pending);
    let retreating = queued.iter().any(|&(_, order)| order == locks::RETREAT);
    let (pursues, temptations) =
        if retreating { read(squads, columns, roster, given.discipline) } else { (vec![], vec![]) };
    let mut retreats = Retreats {
        routes: given.routes,
        pursues: &pursues,
        temptations: &temptations,
        within: given.within,
        tick: given.tick,
        seed: given.seed,
    };
    let applied = apply(squads, columns, roster, &queued, &mut retreats);
    for a in &applied {
        let s = roster.listed[a.listed].squad;
        log.squad(events::ORDER_APPLIED, given.tick, squads, s, &[i64::from(squads.order[s])]);
        if let Some(pursuers) = &a.retreat {
            log.squad(events::DISENGAGED, given.tick, squads, s, &[]);
            let listed = &roster.listed[a.listed];
            let short = discipline::disorder(columns, listed, given.discipline);
            if short > 0.0 {
                let amount = (given.ragged_shock as f64 * short).round() as i64;
                shocks::shock((squads, &*columns), listed, amount, (given.bands, given.tick), log);
            }
            log.squad(events::WITHDRAWING, given.tick, squads, s, &[]);
            for p in pursuers {
                let enemy = roster.listed[p.listed].squad;
                log.squad(events::PURSUING, given.tick, squads, enemy, &[listed.id]);
            }
        }
    }
    applied
}

/// What a retreat reads of each listed squad: whether it may pursue, and its living units'
/// chances to break ranks and leashes, flat in the roster's order.
fn read(
    squads: &Squads,
    columns: &Columns,
    roster: &Roster,
    t: &DisciplineTuning,
) -> (Vec<bool>, Vec<(f64, f64)>) {
    let mut pursues = Vec::with_capacity(roster.listed.len());
    let mut temptations = Vec::new();
    for listed in &roster.listed {
        let command = squads.command[listed.squad];
        pursues.push(command.is_some_and(|c| squads.commands.gives(c, PURSUES)));
        let leadership = discipline::leadership(columns, listed);
        for m in &listed.living {
            let steadied = discipline::unit(columns, m.slot, leadership, t);
            let chance = (t.meets_threats - steadied) as f64 / 100.0;
            temptations.push((chance, discipline::leash(steadied as f64, 0, t)));
        }
    }
    (pursues, temptations)
}
