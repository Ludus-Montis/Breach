//! The kingdom's response as `LaneState` holds it (spec 39 step 9b-iii): the suspicion, the
//! dispatch and the watcher (`crate::kingdom`), main.gd's wiring of them, and the
//! dictionaries GDScript's TaskForceDispatch held.
//! - A tier's response unit is its dictionary in `respond`'s `units`, and stays its truth:
//!   the task force spawned is a moving blocker with that very dictionary as its `blocker`,
//!   so each clash's hp is written into it and a later dispatch for the tier sends what it
//!   then holds, as GDScript's shared dictionary did.
//! - A task force is the mover's dictionary as its spawn gave it, held until it is completed
//!   or consumed.

use std::collections::BTreeMap;

use godot::prelude::*;

use super::{blocker_of, field, int_of, LaneState};
use crate::kingdom::{Dispatch, Suspicion, Tier, Watcher, PLAYER};

/// The response held: none until `respond`, so the meter never leaves Calm and nobody wins.
pub(super) struct Response {
    pub(super) suspicion: Suspicion,
    pub(super) dispatch: Dispatch,
    pub(super) watcher: Watcher,
    /// Each tier's unit's dictionary, by the tier's number.
    units: BTreeMap<i64, VarDictionary>,
    /// Each task force's dictionary, by its mover's id.
    task_forces: BTreeMap<i64, VarDictionary>,
    /// Where its signals are emitted (SimEvents).
    events: Option<Gd<Object>>,
}

impl Default for Response {
    fn default() -> Response {
        Response {
            suspicion: Suspicion::new(&[], 0),
            dispatch: Dispatch::default(),
            watcher: Watcher::new(-1),
            units: BTreeMap::new(),
            task_forces: BTreeMap::new(),
            events: None,
        }
    }
}

impl Response {
    /// From `respond`'s dictionary: {thresholds, decay_per_tick, core, spawn_index,
    /// spawn_direction, units {tier: {hp, dmg}}}.
    pub(super) fn of(config: &VarDictionary, events: Gd<Object>) -> Response {
        let thresholds: Vec<f64> = config
            .get("thresholds")
            .map(|listed| crate::items(&listed).iter().map(float_of).collect())
            .unwrap_or_default();
        let units: BTreeMap<i64, VarDictionary> = config
            .get("units")
            .and_then(|units| units.try_to::<VarDictionary>().ok())
            .map(|units| {
                units
                    .iter_shared()
                    .filter_map(|(tier, unit)| Some((int_of(&tier), unit.try_to().ok()?)))
                    .collect()
            })
            .unwrap_or_default();
        let stats = units.iter().map(|(&tier, unit)| (tier, blocker_of(unit))).collect();
        Response {
            suspicion: Suspicion::new(&thresholds, field(config, "decay_per_tick")),
            dispatch: Dispatch::new(
                field(config, "spawn_index"),
                field(config, "spawn_direction"),
                stats,
            ),
            watcher: Watcher::new(field(config, "core")),
            units,
            task_forces: BTreeMap::new(),
            events: Some(events),
        }
    }

    /// The task forces dispatched, in order.
    pub(super) fn task_forces(&self) -> VarArray {
        let held = self.dispatch.dispatched().iter();
        held.filter_map(|id| self.task_forces.get(id)).map(|made| made.to_variant()).collect()
    }
}

impl LaneState {
    /// A change of tier (when there was one): its task force sent, then the change told.
    pub(super) fn retiered(&mut self, tier: Option<Tier>) {
        let Some(tier) = tier else { return };
        let unit = self.response.units.get(&tier.number()).cloned();
        if let Some(unit) = &unit {
            self.response.dispatch.send(tier, blocker_of(unit));
        }
        if let Some(id) = self.response.dispatch.entered(tier, &mut self.lane) {
            if let Some(unit) = unit {
                self.blockers.insert(id, unit);
            }
            let mover = self.lane.movers().iter().find(|mover| mover.id == id).cloned();
            if let Some(mover) = mover {
                let made = self.mover_dictionary(&mover);
                self.response.task_forces.insert(id, made);
            }
        }
        if let Some(events) = self.response.events.clone() {
            self.emit(&events, "suspicion_tier_changed", &[tier.number().to_variant()]);
        }
    }

    /// Task force `task_force` (its dictionary) back, or else destroyed: no longer held.
    pub(super) fn let_go(&mut self, task_force: &VarDictionary, came_back: bool) {
        let id = field(task_force, "id");
        if came_back {
            self.response.dispatch.completed(id);
        } else {
            self.response.dispatch.consumed(id);
        }
        self.response.task_forces.remove(&id);
    }

    /// main.gd's tick: the harvests, the suspicion's decay, the lane's tick, a lost task
    /// force consumed, and the player's units counted.
    pub(super) fn ticked(&mut self) {
        self.economy.bind_mut().tick();
        let decayed = self.response.suspicion.decay();
        self.retiered(decayed);
        let before = self.lane.movers().len();
        if let Some(events) = self.response.events.clone() {
            self.advance_positions(events.clone());
            self.response.dispatch.after_tick(before, self.lane.movers().len());
            let held = self.response.dispatch.dispatched().to_vec();
            self.response.task_forces.retain(|id, _| held.contains(id));
            if self.response.watcher.counted(self.lane.units_of(PLAYER)) {
                self.emit(&events, "defeat", &[]);
            }
        }
    }
}

fn float_of(value: &Variant) -> f64 {
    match value.get_type() {
        VariantType::INT => value.to::<i64>() as f64,
        VariantType::FLOAT => value.to::<f64>(),
        _ => 0.0,
    }
}
