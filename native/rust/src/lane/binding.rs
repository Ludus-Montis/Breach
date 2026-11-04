//! `LaneState`, the lane held by the crate (spec 39 step 9b-i), as Godot reads it: main.gd,
//! the lane's view and the strategy around it. Its lists are read as dictionaries, the
//! shape GDScript's LaneSimulation held them in, each with its `id`; a moving blocker's
//! `blocker` is the dictionary it was spawned with, which stays its truth, as in GDScript
//! (whoever holds it sees each clash's hp, and what they set it to is what it fights with).
//!
//! `advance_positions(events)` is a tick: each wave arrives in turn and its events are
//! emitted on `events` (SimEvents) before the next arrives, so a handler that spawns,
//! despawns or harvests acts between arrivals, where it did.

use std::collections::BTreeMap;

use godot::prelude::*;

use super::{Blocker, Event, Lane, Mover, Outcome, Unit, Wave};
use crate::content::Node;

#[derive(GodotClass)]
#[class(base = RefCounted, init)]
struct LaneState {
    lane: Lane,
    /// Each moving blocker's `blocker` dictionary, by its id.
    blockers: BTreeMap<i64, VarDictionary>,
    base: Base<RefCounted>,
}

#[godot_api]
impl LaneState {
    /// A fresh lane over `nodes` (NodeDefs, or anything with their garrison's fields).
    #[func]
    fn start(&mut self, nodes: Variant) {
        let nodes: Vec<Node> = crate::objects(&nodes)
            .iter()
            .map(|node| Node {
                garrison: int_of(&node.get("garrison")),
                garrison_hp: int_of(&node.get("garrison_hp")),
                garrison_dmg: int_of(&node.get("garrison_dmg")),
                ..Node::default()
            })
            .collect();
        self.lane = Lane::new(&nodes);
        self.blockers.clear();
    }

    /// Node `index`: {owner ("" none), garrison_hp}.
    #[func]
    fn node_state(&self, index: i64) -> VarDictionary {
        let (owner, hp) = self.lane.node(index as usize);
        let mut state = VarDictionary::new();
        state.set("owner", &owner.to_variant());
        state.set("garrison_hp", &hp.to_variant());
        state
    }

    /// The waves on the lane: {id, owner, units [{hp, dmg}], position, direction, engaged}.
    #[func]
    fn waves(&self) -> VarArray {
        self.lane.waves().iter().map(|wave| wave_dictionary(wave).to_variant()).collect()
    }

    /// The moving blockers: {id, owner, blocker, position, direction, engaged}.
    #[func]
    fn moving_blockers(&self) -> VarArray {
        self.lane.movers().iter().map(|mover| self.mover_dictionary(mover).to_variant()).collect()
    }

    #[func]
    fn spawn_wave(
        &mut self,
        owner: GString,
        units: Variant,
        start: i64,
        direction: i64,
    ) -> VarDictionary {
        let units = crate::items(&units).iter().map(|unit| unit_of(&unit.to())).collect();
        let id = self.lane.spawn_wave(&owner.to_string(), units, start, direction);
        let wave = self.lane.waves().iter().find(|wave| wave.id == id);
        wave.map(wave_dictionary).unwrap_or_default()
    }

    /// Takes off the lane the wave `wave` names by its id.
    #[func]
    fn despawn_wave(&mut self, wave: VarDictionary) {
        self.lane.despawn_wave(wave.get("id").map_or(0, |id| int_of(&id)));
    }

    #[func]
    fn spawn_moving_blocker(
        &mut self,
        owner: GString,
        blocker: VarDictionary,
        start: i64,
        direction: i64,
    ) -> VarDictionary {
        let id = self.lane.spawn_mover(&owner.to_string(), blocker_of(&blocker), start, direction);
        self.blockers.insert(id, blocker);
        let mover = self.lane.movers().iter().find(|mover| mover.id == id);
        mover.map(|mover| self.mover_dictionary(mover)).unwrap_or_default()
    }

    /// `owner`'s waves on node `index` set to harvest (main.gd's farm).
    #[func]
    fn harvest(&mut self, index: i64, owner: GString) {
        self.lane.harvest(index, &owner.to_string());
    }

    /// `owner`'s units, harvesting and marching (PlayerRoster's total).
    #[func]
    fn units_of(&self, owner: GString) -> i64 {
        self.lane.units_of(&owner.to_string())
    }

    /// A tick: the march, then each wave's arrival, its events emitted on `events` as
    /// `combat_resolved(node, outcome)` and `node_captured(node)`.
    #[func]
    fn advance_positions(&mut self, events: Gd<Object>) {
        for id in self.lane.march() {
            self.read_blockers();
            let fired = self.lane.arrive(id);
            self.write_blockers(&fired);
            let mut events = events.clone();
            // Held while the handlers run: they may call back into this lane.
            let _calls_back = self.base_mut();
            for event in &fired {
                match event {
                    Event::Combat { node, outcome, .. } => {
                        events.emit_signal(
                            "combat_resolved",
                            &[node.to_variant(), outcome_variant(outcome)],
                        );
                    }
                    Event::Captured { node } => {
                        events.emit_signal("node_captured", &[node.to_variant()]);
                    }
                }
            }
        }
    }
}

impl LaneState {
    /// Each mover's blocker as its dictionary holds it now.
    fn read_blockers(&mut self) {
        let read: Vec<(i64, Blocker)> =
            self.blockers.iter().map(|(&id, blocker)| (id, blocker_of(blocker))).collect();
        for (id, blocker) in read {
            self.lane.set_blocker(id, blocker);
        }
    }

    /// A clash's hp into its mover's dictionary; a mover gone, its dictionary let go.
    fn write_blockers(&mut self, fired: &[Event]) {
        for event in fired {
            if let Event::Combat { outcome, mover: Some(id), .. } = event {
                if let Some(blocker) = self.blockers.get_mut(id) {
                    blocker.set("hp", &outcome.remaining_blocker_hp.to_variant());
                }
            }
        }
        let movers = self.lane.movers();
        self.blockers.retain(|id, _| movers.iter().any(|mover| mover.id == *id));
    }

    fn mover_dictionary(&self, mover: &Mover) -> VarDictionary {
        let mut made = VarDictionary::new();
        made.set("id", &mover.id.to_variant());
        made.set("owner", &mover.owner.to_variant());
        let blocker = self.blockers.get(&mover.id).cloned().unwrap_or_else(|| {
            let mut made = VarDictionary::new();
            made.set("hp", &mover.blocker.hp.to_variant());
            made.set("dmg", &mover.blocker.dmg.to_variant());
            made
        });
        made.set("blocker", &blocker.to_variant());
        made.set("position", &mover.position.to_variant());
        made.set("direction", &mover.direction.to_variant());
        made.set("engaged", &mover.engaged.to_variant());
        made
    }
}

fn wave_dictionary(wave: &Wave) -> VarDictionary {
    let mut made = VarDictionary::new();
    made.set("id", &wave.id.to_variant());
    made.set("owner", &wave.owner.to_variant());
    made.set("units", &units_variant(&wave.units));
    made.set("position", &wave.position.to_variant());
    made.set("direction", &wave.direction.to_variant());
    made.set("engaged", &wave.engaged.to_variant());
    made
}

fn outcome_variant(outcome: &Outcome) -> Variant {
    let mut made = VarDictionary::new();
    made.set("blocker_destroyed", &outcome.blocker_destroyed.to_variant());
    made.set("remaining_blocker_hp", &outcome.remaining_blocker_hp.to_variant());
    made.set("surviving_horde", &units_variant(&outcome.surviving_horde));
    made.to_variant()
}

fn units_variant(units: &[Unit]) -> Variant {
    let listed: VarArray = units
        .iter()
        .map(|unit| {
            let mut made = VarDictionary::new();
            made.set("hp", &unit.hp.to_variant());
            made.set("dmg", &unit.dmg.to_variant());
            made.to_variant()
        })
        .collect();
    listed.to_variant()
}

fn unit_of(unit: &VarDictionary) -> Unit {
    Unit { hp: field(unit, "hp"), dmg: field(unit, "dmg") }
}

fn blocker_of(blocker: &VarDictionary) -> Blocker {
    Blocker { hp: field(blocker, "hp"), dmg: field(blocker, "dmg") }
}

fn field(fields: &VarDictionary, key: &str) -> i64 {
    fields.get(key).map_or(0, |value| int_of(&value))
}

/// A whole number held as an int or a float (GDScript's ints are 64-bit).
fn int_of(value: &Variant) -> i64 {
    match value.get_type() {
        VariantType::INT => value.to::<i64>(),
        VariantType::FLOAT => value.to::<f64>() as i64,
        _ => 0,
    }
}
