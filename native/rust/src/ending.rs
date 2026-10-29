//! The tick's end as one crate call (spec 39 step 8b5-xl): stamina (`stamina::pass`), the
//! bodies' parting (`parting::pass`), the shuffle (`swaps::pass`) and the march's sync
//! (`march::sync`), in that order over the squads listed, as FormationSimulation made them
//! one call at a time. None of GDScript's mirrors between them fed the next: stamina's
//! hands back only the units made by hand that took their speed as fresh, the parting's its
//! counts, and the sync nothing. The shuffle's events go to the log.

#[cfg(test)]
pub(crate) mod tests;

use std::time::Instant;

use crate::events::Log;
use crate::field::Field;
use crate::ground::Ground;
use crate::march::sync::{self, Given};
use crate::march::walk::WalkTuning;
use crate::parting::{self, Tuning as PartingTuning};
use crate::squads::Squads;
use crate::stamina::{self, StaminaTuning};
use crate::swaps;

/// Every pass's tuning: the tick's seconds, the battle's seed, stamina's, the parting's
/// (and whether it runs threaded), the shuffle's [tick seconds, cells a second at speed 1,
/// scrum_crowding], and the march's: its [seconds, cells a second at speed 1], its walk and
/// wounds_reach.
pub struct Rules {
    pub tick: i64,
    pub seconds: f64,
    pub seed: i64,
    pub stamina: StaminaTuning,
    pub parting: PartingTuning,
    pub threaded: bool,
    pub shuffle: [f64; 3],
    pub timing: [f64; 2],
    pub walk: WalkTuning,
    pub wounds_reach: f64,
}

/// What GDScript reads back: the units made by hand that took their speed as fresh (their
/// ids), the parting's microseconds, the passes that moved something and the pairs pushed.
#[derive(Debug, Default, PartialEq)]
pub struct Ended {
    pub made: Vec<i64>,
    pub parting_usec: i64,
    pub passes: i64,
    pub pairs: i64,
}

/// The tick's end over the squads in squad slots `listed`, in the lists' order, on
/// `ground`; the shuffle's events in `log`.
pub fn step(
    squads: &mut Squads,
    field: &mut Field,
    listed: &[usize],
    ground: Option<&Ground>,
    rules: &Rules,
    log: &mut Log,
) -> Ended {
    let made = stamina::pass::step(squads, field, listed, rules.seconds, &rules.stamina);
    let began = Instant::now();
    let parts = (&rules.parting, rules.threaded);
    let parted = parting::pass::step(squads, field, listed, rules.seed, parts);
    let parting_usec = began.elapsed().as_micros() as i64;
    swaps::pass::step(squads, field, listed, (rules.shuffle, rules.tick), log);
    let given = Given {
        ground,
        timing: Some(rules.timing),
        seed: rules.seed,
        wounds_reach: rules.wounds_reach,
        tuning: rules.walk,
    };
    sync::step(squads, field, listed, &given);
    Ended { made, parting_usec, passes: parted.passes, pairs: parted.pairs }
}
