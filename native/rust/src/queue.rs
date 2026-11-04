//! The command queue (spec 39 step 9c, Decision 3), ported from GDScript's CommandQueue as it
//! was (specs/01-simulation-clock-and-commands.md): a command is held from its enqueue until
//! a tick finds it filled, and is committed then, never before; an under-filled one stays,
//! checked again each tick; a cancelled one never commits. Each has its own id, counted from
//! 0 and never given twice. A tick commits those due in the order they were enqueued, each
//! taken in turn (one cancelled by an earlier commit's handler is skipped, where GDScript's
//! lookup failed).
//!
//! What a command is and how its fill is checked are the holder's (`T`): `LaneState` holds
//! each command with its fill check, a Godot callable (`lane/binding/queue.rs`). Pure.

#[cfg(test)]
mod tests;

use std::collections::BTreeMap;

/// The commands pending, by their ids.
#[derive(Clone, Debug, PartialEq)]
pub struct Queue<T> {
    pending: BTreeMap<i64, T>,
    next_id: i64,
}

impl<T> Default for Queue<T> {
    fn default() -> Queue<T> {
        Queue { pending: BTreeMap::new(), next_id: 0 }
    }
}

impl<T> Queue<T> {
    /// `command` held pending: its id.
    pub fn enqueue(&mut self, command: T) -> i64 {
        let id = self.next_id;
        self.next_id += 1;
        self.pending.insert(id, command);
        id
    }

    /// The command `id` no longer pending (none, when it isn't).
    pub fn cancel(&mut self, id: i64) {
        self.pending.remove(&id);
    }

    /// The command `id`, while it is pending.
    pub fn get(&self, id: i64) -> Option<&T> {
        self.pending.get(&id)
    }

    /// The ids pending, in the order they were enqueued.
    pub fn pending(&self) -> Vec<i64> {
        self.pending.keys().copied().collect()
    }

    /// A tick's checks: the ids `filled` finds filled, in the order they were enqueued. Every
    /// check is made before any command is taken, as GDScript's were.
    pub fn due(&self, mut filled: impl FnMut(&T) -> bool) -> Vec<i64> {
        self.pending.iter().filter(|(_, command)| filled(command)).map(|(&id, _)| id).collect()
    }

    /// The command `id` committed: taken off, when it is still pending.
    pub fn take(&mut self, id: i64) -> Option<T> {
        self.pending.remove(&id)
    }
}
