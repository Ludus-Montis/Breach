//! The command queue as `LaneState` holds it (spec 39 step 9c, Decision 3): each command a
//! Variant (main.gd's wave, a dictionary) with its fill check, a callable that answers true
//! once its slots are filled. A tick commits those filled, each told as
//! `command_committed(command)` on the response's events (SimEvents), this lane lent while
//! the handlers run, so a commit's handler spawns its wave on it; the lane's own tick then
//! runs, as main.gd's handler ran the queue and then the lane.

use godot::prelude::*;

use super::LaneState;
use crate::queue::Queue;

/// A command, and the check that says its slots are filled.
pub(super) type Pending = Queue<(Variant, Callable)>;

/// Whether `check` answers true (one that answers nothing is not filled).
pub(super) fn filled(check: &Callable) -> bool {
    check.call(&[]).booleanize()
}

impl LaneState {
    /// The queue's commits, each told in turn. The checks are made on a copy while this lane
    /// is lent, so they may read it, and a command they enqueue waits for the next tick.
    pub(super) fn commit_queue(&mut self) {
        let checked = self.queue.clone();
        let due = {
            let _calls_back = self.base_mut();
            checked.due(|(_, check)| filled(check))
        };
        let Some(events) = self.response.events.clone() else { return };
        for id in due {
            if let Some((command, _)) = self.queue.take(id) {
                self.emit(&events, "command_committed", &[command]);
            }
        }
    }
}
