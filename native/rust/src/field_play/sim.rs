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
        let drawn = None;
        Some(FieldSim { content, squads, field, play, events, seed, captained, record, drawn })
    }

    /// One tick (FormationField.step); its events, the builders' and waves' first.
    pub fn step(&mut self) -> FieldTold {
        let play = &mut self.play;
        let (made, _) =
            play.step(&mut self.squads, &mut self.field, &self.content, &mut self.events);
        let flat = self.events.take();
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
