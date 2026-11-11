//! The 2D feel test's field as one simulation (spec 39 step 10d-i): the content it was made
//! from, shared (`Arc`), the squads, the units and the play, its events and its record of
//! play. `FieldState::step` plays it in place; `Worker` plays it on a thread (`Steps`), each
//! tick's snapshot holding what the scene draws. A snapshot's drawn columns (the units'
//! positions, bearings and ranks) are copied only when they changed: each is compared with
//! the last snapshot's copy, and an equal one is shared, not copied.

use std::sync::Arc;

use super::seen::SeenSquad;
use super::{FieldPlay, A, B};
use crate::content::Content;
use crate::events::told::{self, Told};
use crate::events::Log;
use crate::field::Field;
use crate::maths::V2;
use crate::record::{self, Command};
use crate::scenarios::{Scenario, Seen, Watch};
use crate::squads::Squads;
use crate::threads::Threads;
use crate::waves::Produced;
use crate::worker::Steps;

#[cfg(test)]
mod tests;

/// The field being played.
pub struct FieldSim {
    pub content: Arc<Content>,
    pub squads: Squads,
    pub field: Field,
    pub play: FieldPlay,
    pub events: Log,
    pub seed: i64,
    pub captained: bool,
    /// The record of play: its header, then each command taken, a line each.
    pub record: Vec<String>,
    /// The drawn columns as the last snapshot copied them.
    drawn: Option<Drawn>,
    /// A named scenario's watch, when the field plays one (`scenario`), and whether it ended.
    watch: Option<(Watch, bool)>,
}

/// What a scenario's field has shown so far, and whether the scenario has ended.
#[derive(Clone, Debug, PartialEq)]
pub struct Watched {
    pub seen: Seen,
    pub ended: bool,
}

/// The units' drawn columns, by slot, shared between snapshots while unchanged.
#[derive(Clone)]
struct Drawn {
    positions: Arc<Vec<V2>>,
    bearings: Arc<Vec<f64>>,
    ranks: Arc<Vec<i32>>,
}

/// One tick's events: what the builders and waves did, then the battle's, decoded.
pub type FieldTold = (Vec<Produced>, Vec<Told>);

/// What the scene draws of the field after a tick: the units' positions, bearings and ranks
/// by slot, the squads as seen, the kingdom's line's and reserve's ids, A's and B's places
/// (`counted`) and where B's waves wait.
pub struct FieldView {
    pub positions: Arc<Vec<V2>>,
    pub bearings: Arc<Vec<f64>>,
    pub ranks: Arc<Vec<i32>>,
    pub seen: Vec<SeenSquad>,
    pub kingdom: Vec<i64>,
    pub waves: [[i64; 4]; 2],
    pub staging: Option<V2>,
    /// A scenario's field: what it has shown and whether it ended; None for free play.
    pub watched: Option<Watched>,
}

impl FieldSim {
    /// A fresh field from `content`, seeded and captained as `at` says, its record's header
    /// stamped with `build`; None if the content lacks a unit the field needs.
    pub fn open(
        content: Arc<Content>,
        (seed, captained): (i64, bool),
        build: &str,
        threaded: Threads,
    ) -> Option<FieldSim> {
        let (mut squads, mut field) = (Squads::default(), Field::default());
        let at = (seed, true, threaded);
        let play = FieldPlay::new(captained, (&mut squads, &mut field), &content, at)?;
        let record = vec![record::header(seed, captained, build)];
        let events = Log::default();
        let (drawn, watch) = (None, None);
        Some(FieldSim {
            content,
            squads,
            field,
            play,
            events,
            seed,
            captained,
            record,
            drawn,
            watch,
        })
    }

    /// `scenario` set up from `content` to play hands-free (spec 44 step 2c): seeded and
    /// captained as its record says, its commands given on their ticks by its watch, which
    /// reads what it shows and when it ends, as the bench's `Replay` does. Its record of play is
    /// the scenario's. None if the field can't be set up.
    pub fn scenario(
        content: Arc<Content>,
        scenario: &Scenario,
        threaded: Threads,
    ) -> Option<FieldSim> {
        let at = (scenario.record.seed, scenario.record.captained);
        let mut sim = FieldSim::open(content.clone(), at, &scenario.record.build, threaded)?;
        sim.watch = Some((Watch::new(scenario, &content)?, false));
        sim.record = scenario.text.lines().map(str::to_owned).collect();
        Some(sim)
    }

    /// One tick (FormationField.step); its events, the builders' and waves' first.
    pub fn step(&mut self) -> FieldTold {
        let play = &mut self.play;
        let (made, flat) = match self.watch.as_mut() {
            Some((watch, ended)) => {
                let (made, flat, now) =
                    watch.step(play, (&mut self.squads, &mut self.field), &self.content);
                *ended |= now;
                (made, flat)
            }
            None => {
                let (made, _) =
                    play.step(&mut self.squads, &mut self.field, &self.content, &mut self.events);
                (made, self.events.take())
            }
        };
        let words = self.field.lexicon.words();
        (made, told::decoded(&flat, words, &play.play.codes.definitions))
    }

    /// `given` given to the field now; recorded, and true, if it takes it.
    pub fn given(&mut self, given: &Command) -> bool {
        let state = (&mut self.squads, &mut self.field);
        let taken = self.play.apply(given, state, &self.content);
        if taken {
            self.record.push(record::line(given));
        }
        taken
    }

    /// What the scene draws now, the drawn columns copied only where they changed.
    pub fn view(&mut self) -> FieldView {
        let columns = &self.field.columns;
        let last = self.drawn.as_ref();
        let drawn = Drawn {
            positions: marked(last.map(|d| &d.positions), &columns.position),
            bearings: marked(last.map(|d| &d.bearings), &columns.bearing),
            ranks: marked(last.map(|d| &d.ranks), &columns.rank),
        };
        self.drawn = Some(drawn.clone());
        let (play, content) = (&self.play, &self.content);
        FieldView {
            positions: drawn.positions,
            bearings: drawn.bearings,
            ranks: drawn.ranks,
            seen: play.seen(&self.squads, &self.field, content),
            kingdom: play.kingdom.iter().map(|&s| self.squads.id[s]).collect(),
            waves: [play.counted(A, content), play.counted(B, content)],
            staging: play.staging_point(),
            watched: self
                .watch
                .as_ref()
                .map(|(w, ended)| Watched { seen: w.seen.clone(), ended: *ended }),
        }
    }
}

/// `now` as a snapshot holds it: `last`'s copy if equal, else a new copy.
fn marked<T: Clone + PartialEq>(last: Option<&Arc<Vec<T>>>, now: &[T]) -> Arc<Vec<T>> {
    match last {
        Some(last) if last.as_slice() == now => last.clone(),
        _ => Arc::new(now.to_vec()),
    }
}

impl Steps for FieldSim {
    type Order = Command;
    type Told = FieldTold;
    type View = FieldView;

    fn tick(&self) -> i64 {
        self.play.play.tick
    }

    fn apply(&mut self, order: &mut Command) -> bool {
        order.tick = self.tick();
        self.given(order)
    }

    fn step(&mut self) -> FieldTold {
        FieldSim::step(self)
    }

    fn view(&mut self) -> FieldView {
        FieldSim::view(self)
    }
}
