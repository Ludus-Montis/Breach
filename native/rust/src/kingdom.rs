//! The kingdom's response (spec 39 step 9b-iii), ported from GDScript's SuspicionSystem,
//! TaskForceDispatch and ScriptedBeatWatcher as they were
//! (specs/04-suspicion-and-response.md, specs/06-map-content-p-f-F-c.md).
//! - Suspicion is a meter from 0 to 100 (added to, clamped; less its decay each tick, never
//!   below 0), and a tier from the map's four thresholds (Wary, Alarmed, Mobilized, Full
//!   Alert; below the first, Calm). Each change of tier is told, up or down.
//! - The dispatch spawns a tier's response unit, when it has one, as the defenders' moving
//!   blocker on the lane, and holds it dispatched until it is completed or consumed.
//! - The watcher wins once when the Core is captured, and loses once when the player has no
//!   units left.
//!
//! `LaneState` (`lane/binding.rs`) holds these with the lane, as main.gd held them. Pure.

#[cfg(test)]
mod tests;

use std::collections::BTreeMap;

use crate::lane::{Blocker, Lane};

/// The side the dispatch spawns for (TaskForceDispatch's "defender").
pub const DEFENDER: &str = "defender";
/// The side whose units the watcher counts (main.gd's "player").
pub const PLAYER: &str = "player";

/// The meter's ceiling.
const METER_MAX: f64 = 100.0;

/// A suspicion tier, numbered as SuspicionSystem.Tier was.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tier {
    Calm = 0,
    Wary = 1,
    Alarmed = 2,
    Mobilized = 3,
    FullAlert = 4,
}

impl Tier {
    /// Its number.
    pub fn number(self) -> i64 {
        self as i64
    }
}

/// The suspicion meter and its tier.
#[derive(Clone, Debug, PartialEq)]
pub struct Suspicion {
    /// Wary's, Alarmed's, Mobilized's and Full Alert's thresholds, in that order.
    thresholds: [f64; 4],
    decay_per_tick: f64,
    meter: f64,
    tier: Tier,
}

impl Suspicion {
    /// Calm at 0. A threshold not given is never reached (GDScript's index was an error).
    pub fn new(thresholds: &[f64], decay_per_tick: i64) -> Suspicion {
        let mut held = [f64::INFINITY; 4];
        for (slot, threshold) in held.iter_mut().zip(thresholds) {
            *slot = *threshold;
        }
        Suspicion {
            thresholds: held,
            decay_per_tick: decay_per_tick as f64,
            meter: 0.0,
            tier: Tier::Calm,
        }
    }

    /// The meter, its fraction dropped (GDScript's `int`).
    pub fn meter(&self) -> i64 {
        self.meter as i64
    }

    pub fn tier(&self) -> Tier {
        self.tier
    }

    /// Adds `amount`, clamped to 0..100; the new tier when it changed.
    pub fn add(&mut self, amount: f64) -> Option<Tier> {
        self.meter = (self.meter + amount).clamp(0.0, METER_MAX);
        self.retier()
    }

    /// A tick's decay; the new tier when it changed.
    pub fn decay(&mut self) -> Option<Tier> {
        self.meter = (self.meter - self.decay_per_tick).max(0.0);
        self.retier()
    }

    fn retier(&mut self) -> Option<Tier> {
        let tiers = [Tier::Wary, Tier::Alarmed, Tier::Mobilized, Tier::FullAlert];
        let tier = tiers
            .iter()
            .zip(self.thresholds)
            .rev()
            .find(|(_, threshold)| self.meter >= *threshold)
            .map_or(Tier::Calm, |(tier, _)| *tier);
        if tier == self.tier {
            return None;
        }
        self.tier = tier;
        Some(tier)
    }
}

/// The task forces the tiers send.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Dispatch {
    spawn_index: i64,
    spawn_direction: i64,
    /// Each tier's response unit, by the tier's number.
    units: BTreeMap<i64, Blocker>,
    /// The movers dispatched, by id, in order.
    dispatched: Vec<i64>,
}

impl Dispatch {
    pub fn new(spawn_index: i64, spawn_direction: i64, units: BTreeMap<i64, Blocker>) -> Dispatch {
        Dispatch { spawn_index, spawn_direction, units, dispatched: Vec::new() }
    }

    /// The task forces' movers, by id, in the order dispatched.
    pub fn dispatched(&self) -> &[i64] {
        &self.dispatched
    }

    /// `tier`'s unit, as it now is.
    pub fn send(&mut self, tier: Tier, blocker: Blocker) {
        self.units.insert(tier.number(), blocker);
    }

    /// `tier` entered: its unit spawned on `lane`, its mover's id; none when it has none.
    pub fn entered(&mut self, tier: Tier, lane: &mut Lane) -> Option<i64> {
        let blocker = *self.units.get(&tier.number())?;
        let id = lane.spawn_mover(DEFENDER, blocker, self.spawn_index, self.spawn_direction);
        self.dispatched.push(id);
        Some(id)
    }

    /// Task force `id` came back (no roster yet: it is no longer dispatched).
    pub fn completed(&mut self, id: i64) {
        self.dispatched.retain(|held| *held != id);
    }

    /// Task force `id` was destroyed (no roster yet: it is no longer dispatched).
    pub fn consumed(&mut self, id: i64) {
        self.dispatched.retain(|held| *held != id);
    }

    /// After a tick, main.gd's rule: when the lane has fewer movers than before it and a
    /// task force is dispatched, the first dispatched is consumed.
    pub fn after_tick(&mut self, movers_before: usize, movers_after: usize) {
        if movers_after < movers_before && !self.dispatched.is_empty() {
            self.dispatched.remove(0);
        }
    }
}

/// This map's win and loss (Decision 16): the Core captured wins, the force lost loses.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Watcher {
    core: i64,
    won: bool,
    defeated: bool,
}

impl Watcher {
    pub fn new(core: i64) -> Watcher {
        Watcher { core, won: false, defeated: false }
    }

    /// Node `index` captured: whether that is the victory, told once.
    pub fn captured(&mut self, index: i64) -> bool {
        if index != self.core || self.won {
            return false;
        }
        self.won = true;
        true
    }

    /// The player's units counted: whether that is the defeat, told once.
    pub fn counted(&mut self, units: i64) -> bool {
        if units > 0 || self.defeated {
            return false;
        }
        self.defeated = true;
        true
    }
}
