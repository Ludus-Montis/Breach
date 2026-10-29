//! Morale's second as one crate pass (FormationMorale.step, spec 39 step 8b5-xxvii): each
//! listed squad with anyone standing and not routing is stepped (`morale::step`) from its
//! morale (its ceiling if it has none yet, `shocks::morale`), its best leader's leadership and
//! its ceiling, all read from its living units' sheets (`bands.rs`); then each comes to what
//! the second left it, held to 0..100, a change of band "morale_band" in the log, in the
//! lists' order (`shocks::shock_living`).

#[cfg(test)]
mod tests;

use crate::bands::{self, BandTuning};
use crate::columns::Columns;
use crate::events::Log;
use crate::locks::ROUTING;
use crate::morale::{self, MoraleTuning, Stepping};
use crate::roster::Roster;
use crate::shocks;
use crate::squads::Squads;

/// The second for `roster`'s squads on `tick`.
pub fn step(
    (squads, columns): (&mut Squads, &Columns),
    roster: &Roster,
    (t, bands): (&MoraleTuning, &BandTuning),
    tick: i64,
    log: &mut Log,
) {
    let mut stepping = vec![];
    for (n, listed) in roster.listed.iter().enumerate() {
        if listed.living.is_empty() || squads.state[listed.squad] == ROUTING {
            continue;
        }
        let morale = shocks::morale(squads, columns, listed, bands);
        let sheets = listed.living.iter().map(|m| &columns.sheet[m.slot]);
        let courage: Vec<i64> = sheets.clone().map(|sheet| sheet.courage).collect();
        let leaderships: Vec<i64> = sheets.map(|sheet| sheet.leadership).collect();
        let leadership = bands::leadership(&leaderships);
        let ceiling = bands::ceiling(&courage, &leaderships, bands);
        stepping.push(Stepping { listed: n, morale, leadership, ceiling });
    }
    let after = morale::step(squads, columns, roster, &stepping, t);
    for (s, value) in stepping.iter().zip(after) {
        let listed = &roster.listed[s.listed];
        let living: Vec<usize> = listed.living.iter().map(|m| m.slot).collect();
        // A shock of what the second took (a recovery is a negative one) lands it, held.
        let took = s.morale - value;
        shocks::shock_living(
            (&mut *squads, columns),
            (listed.squad, &living),
            took,
            (bands, tick),
            log,
        );
    }
}
