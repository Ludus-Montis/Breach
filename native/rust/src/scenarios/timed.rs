//! A scenario timed tick by tick, for `breach_tick_phases scenario=<name>` (spec 44 step 2b):
//! the same `Replay` the capture plays, every tick from the first to the scenario's end timed
//! pass by pass into a `phases::series::Series`, so it prints as `series=1` prints. An untimed
//! replay steps beside it, and the two must end equal, as the clash's untimed twin must.

use std::path::Path;

use super::{list, named, Replay, Scenario};
use crate::content::Content;
use crate::field::Field;
use crate::laps;
use crate::phases::series::Series;
use crate::squads::Squads;
use crate::threads::Threads;

/// A scenario's ticks timed, and whether the timed replay ended apart from an untimed one.
#[derive(Debug)]
pub struct Timed {
    pub series: Series,
    pub apart: bool,
}

/// `scenario` played to its end on a fresh crate, `threaded` or not, each tick timed; None if
/// the field can't be set up.
pub fn timed(scenario: &Scenario, content: &Content, threaded: Threads) -> Option<Timed> {
    let (mut squads, mut field) = (Squads::default(), Field::default());
    let mut replay = Replay::new(scenario, (&mut squads, &mut field), content, threaded)?;
    let mut series = Series::default();
    loop {
        let tick = replay.tick();
        let ((_, ended), laps) = laps::timed(|| replay.step(&mut squads, &mut field, content));
        series.push(tick, laps);
        if ended {
            let apart = played(scenario, content, threaded)? != replay.state(&squads, &field);
            return Some(Timed { series, apart });
        }
    }
}

/// The state `scenario` ends in, played untimed; None if the field can't be set up.
fn played(scenario: &Scenario, content: &Content, threaded: Threads) -> Option<Vec<u8>> {
    let (mut squads, mut field) = (Squads::default(), Field::default());
    let mut replay = Replay::new(scenario, (&mut squads, &mut field), content, threaded)?;
    while !replay.step(&mut squads, &mut field, content).1 {}
    Some(replay.state(&squads, &field))
}

/// The scenario named `name` in `content`'s list, timed; an error naming it if it isn't listed,
/// or saying why the list or the field can't be read.
pub fn named_timed(
    content_root: &Path,
    content: &Content,
    name: &str,
    threaded: Threads,
) -> Result<(Scenario, Timed), String> {
    let list = list(content_root)?;
    let scenario = named(&list, name)?.clone();
    let timed = timed(&scenario, content, threaded)
        .ok_or_else(|| format!("scenario {name}: the field can't be set up"))?;
    Ok((scenario, timed))
}
