//! `EconomyState`, the economy held by the crate (spec 39 step 9b-ii), as Godot reads it:
//! main.gd, the resource bar, the wave form and the capture choice. EconomySystem's and
//! CaptureResolution's methods in one object, as main.gd held the two together. The lane's
//! binding (`LaneState`) holds one and gives it each capture on the lane (step 9b-iii).

use godot::prelude::*;

use super::{decayed_yield, Captures, Choice, Economy};
use crate::content::{Node, NodeType};

#[derive(GodotClass)]
#[class(base = RefCounted, init)]
pub(crate) struct EconomyState {
    economy: Economy,
    captures: Captures,
}

#[godot_api]
impl EconomyState {
    #[func]
    fn balance(&self, resource: GString) -> i64 {
        self.economy.balance(&resource.to_string())
    }

    #[func]
    fn add(&mut self, resource: GString, amount: i64) {
        self.economy.add(&resource.to_string(), amount);
    }

    /// Spends `amount` of `resource` when there is that much; whether it did.
    #[func]
    fn spend(&mut self, resource: GString, amount: i64) -> bool {
        self.economy.spend(&resource.to_string(), amount)
    }

    #[func]
    fn decayed_yield(&self, base_yield: i64, ticks_elapsed: i64, interval: i64, floor: i64) -> i64 {
        decayed_yield(base_yield, ticks_elapsed, interval, floor)
    }

    /// Node `index` captured: `node` its NodeDef.
    #[func]
    fn on_node_captured(&mut self, index: i64, node: Gd<Object>) {
        self.captures.captured(index, &node_of(&node));
    }

    #[func]
    fn is_awaiting_choice(&self, index: i64) -> bool {
        self.captures.is_awaiting_choice(index)
    }

    /// "ravage", "fortify" or "dismantle" for node `index`; another word does nothing.
    #[func]
    fn issue_choice(&mut self, index: i64, choice: GString) {
        if let Some(choice) = Choice::named(&choice.to_string()) {
            self.captures.choose(index, choice, &mut self.economy);
        }
    }

    /// A tick: each harvesting node adds its yield.
    #[func]
    fn on_tick_advanced(&mut self, _tick_number: i64) {
        self.tick();
    }
}

impl EconomyState {
    /// Node `index` captured, `node` its fields.
    pub(crate) fn captured(&mut self, index: i64, node: &Node) {
        self.captures.captured(index, node);
    }

    /// A tick's harvests.
    pub(crate) fn tick(&mut self) {
        self.captures.tick(&mut self.economy);
    }
}

/// The capture's fields of a NodeDef (its `node_type` NodeDef.NodeType's number).
pub(crate) fn node_of(node: &Gd<Object>) -> Node {
    let field = |name: &str| node.get(name).try_to::<i64>().unwrap_or(0);
    let node_type = match field("node_type") {
        1 => NodeType::Resource,
        2 => NodeType::Fort,
        3 => NodeType::Neutral,
        4 => NodeType::Waypoint,
        _ => NodeType::Origin,
    };
    Node {
        node_type,
        yield_food_per_tick: field("yield_food_per_tick"),
        decay_interval_ticks: field("decay_interval_ticks"),
        decay_floor_food: field("decay_floor_food"),
        ravage_yield_food: field("ravage_yield_food"),
        dismantle_wood_yield: field("dismantle_wood_yield"),
        dismantle_stone_yield: field("dismantle_stone_yield"),
        fortify_wood_cost: field("fortify_wood_cost"),
        fortify_stone_cost: field("fortify_stone_cost"),
        ..Node::default()
    }
}
