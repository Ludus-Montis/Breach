//! The 2D feel test's field as presentation plays it (spec 39 step 10d-ii, the owner's rules in
//! #734): on a worker, read from the newest snapshot, the frame never waiting on a tick. The
//! frame owes the ticks its clock gives (`owe`, true when time was dropped: the simulation is
//! behind), gives orders (`order`, applied at the next boundary between ticks) and takes what
//! the worker has published (`take`): every tick's events, oldest first, so none are lost, and
//! the newest snapshot, which the scene draws. `caught_up` says whether that snapshot is the
//! tick the clock is drawing towards; when it isn't, units hold at the newest. The record is
//! kept here from the orders each snapshot says were taken. `run_ticks` waits, for tests and
//! headless tools.

use super::sim::{FieldSim, FieldTold, FieldView};
use crate::record::{self, Command};
use crate::worker::{Snapshot, Worker};

#[cfg(test)]
mod tests;

/// The field on a worker, as its newest snapshot shows it.
pub struct FieldPlayer {
    worker: Worker<FieldSim>,
    /// The newest snapshot's tick.
    pub tick: i64,
    /// What the scene draws: the newest snapshot's.
    pub newest: FieldView,
    /// The record of play: its header, then each command taken, a line each.
    pub record: Vec<String>,
}

impl FieldPlayer {
    /// `sim` started on a worker; until it publishes, the field as it stands.
    pub fn start(mut sim: FieldSim) -> FieldPlayer {
        let (tick, newest, record) = (sim.play.play.tick, sim.view(), sim.record.clone());
        FieldPlayer { worker: Worker::start(sim), tick, newest, record }
    }

    /// `ticks` more owed by the clock; true if time was dropped (the simulation is behind).
    pub fn owe(&self, ticks: u32) -> bool {
        self.worker.owe(ticks)
    }

    /// `given` queued, applied at the next boundary between ticks (at once if paused).
    pub fn order(&self, given: Command) {
        self.worker.order(given);
    }

    /// A replay's commands, each given when its tick comes.
    pub fn replay(&self, commands: Vec<Command>) {
        for given in commands {
            self.worker.order_at(given.tick, given);
        }
    }

    pub fn pause(&self, paused: bool) {
        self.worker.pause(paused);
    }

    /// True if the newest snapshot is the tick the field reaches once what is owed is played.
    pub fn caught_up(&self) -> bool {
        self.tick >= self.worker.reached()
    }

    /// What the worker has published since last asked, without waiting: each tick's events,
    /// oldest first; the newest snapshot kept.
    pub fn take(&mut self) -> Vec<FieldTold> {
        let shots = self.worker.ready();
        self.took(shots)
    }

    /// `count` ticks played, and every order given before applied, waiting for them; each
    /// tick's events, oldest first.
    pub fn run_ticks(&mut self, count: u32) -> Vec<FieldTold> {
        let shots = self.worker.run_ticks(count);
        self.took(shots)
    }

    /// `shots` taken in order: the orders taken recorded, the newest kept, the events.
    fn took(&mut self, shots: Vec<Snapshot<FieldSim>>) -> Vec<FieldTold> {
        let mut told = Vec::new();
        for shot in shots {
            let taken = shot.taken.iter().filter(|t| t.1);
            self.record.extend(taken.map(|t| record::line(&t.0)));
            told.extend(shot.told);
            (self.tick, self.newest) = (shot.tick, shot.view);
        }
        told
    }
}
