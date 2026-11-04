//! The fight and its aftermath as one crate call (spec 39 step 8b5-xxxviii): the fight
//! (`fight::step`, with the surrender), the burial of the fallen (`deaths::bury`), the takers
//! sent (`taking::pass`, each squad threatened as the perception kept it), the wounds'
//! tending (`wounds::pass`) and what the downed come to (`recovery::pass`), in that order
//! over the roster's squads, as FormationSimulation made them one call at a time. Every
//! pass's events go to the log in that order; GDScript mirrors each pass's result after it.

#[cfg(test)]
mod tests;

use crate::bands::BandTuning;
use crate::deaths::bury::{self, Buried};
use crate::events::Log;
use crate::field::Field;
use crate::fight::step as fighting;
use crate::ground::Ground;
use crate::recovery::pass as recovery;
use crate::roster::Roster;
use crate::squads::Squads;
use crate::taking::pass::{self as taking, TakingTuning};
use crate::wounds::{pass as wounds, WoundsTuning};

/// Every pass's tuning: the fight's rules (its tick and seed are the tick's), the tick's
/// seconds, the takings', the wounds' (tending and recovery), the messenger's shock and
/// morale's bands.
pub struct Rules {
    pub fight: fighting::Rules,
    pub seconds: f64,
    pub taking: TakingTuning,
    pub wounds: WoundsTuning,
    pub messenger_shock: i64,
    pub bands: BandTuning,
}

/// What each pass did, as GDScript mirrors it: the units the surrender took ([index in the
/// roster, id]), the fallen buried, the bodies the tending took and what the downed came to.
#[derive(Debug, Default)]
pub struct Aftermath {
    pub taken: Vec<(usize, i64)>,
    pub buried: Vec<Buried>,
    pub tended: Vec<wounds::Landed>,
    pub recovered: Vec<recovery::Landed>,
}

/// The fight and its aftermath over `roster`'s squads on `ground`, its events in `log`.
pub fn step(
    squads: &mut Squads,
    field: &mut Field,
    roster: &Roster,
    ground: Option<&Ground>,
    rules: &Rules,
    log: &mut Log,
) -> Aftermath {
    let (tick, seed) = (rules.fight.tick, rules.fight.choose.seed);
    let interval = rules.fight.choose.interval;
    let fought = fighting::step(squads, field, roster, &rules.fight, ground, log);
    let taken = fought.taken.iter().map(|&(n, m)| (n, roster.listed[n].living[m].id)).collect();
    let listed: Vec<usize> = roster.listed.iter().map(|l| l.squad).collect();
    let buried = bury::bury(squads, field, &listed, (&rules.bands, tick), log);
    let threatened: Vec<bool> = listed.iter().map(|&s| squads.threatened[s]).collect();
    let (lists, at) = ((&listed[..], &threatened[..]), (tick, rules.seconds, seed));
    taking::step(squads, field, lists, at, &rules.taking, log);
    let shock = rules.messenger_shock;
    let t = wounds::Tuning { wounds: &rules.wounds, shock, bands: &rules.bands };
    let tended = wounds::step(squads, field, &listed, (tick, interval, seed), &t, log);
    let at = (tick, rules.seconds, seed);
    let recovered = recovery::step(squads, field, &listed, at, &rules.wounds, log);
    Aftermath { taken, buried, tended, recovered }
}
