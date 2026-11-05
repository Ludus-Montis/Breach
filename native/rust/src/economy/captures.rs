//! CaptureResolution (specs/03): what each captured node does for the economy.
//! - A resource node harvests from its capture (Decision 2's auto-extraction): each tick adds
//!   its decayed food yield, counting ticks from the capture. Ravaged, it gives its lump sum
//!   once and harvests no more.
//! - A fort awaits a choice: fortify spends its wood and stone (nothing when either can't be
//!   afforded, and it still awaits), dismantle gives its salvage. Either ends the wait.
//! - A node captured again starts afresh. Other kinds of node hold nothing.
//!
//! Fortify's defensive effect and the ground-must-be-held reversion stay deferred (specs/03's
//! Notes): only its cost is here.

use std::collections::BTreeMap;

use super::{decayed_yield, Economy};
use crate::content::{Node, NodeType};

/// A capture choice the player issues (`issue_choice`'s word).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    Ravage,
    Fortify,
    Dismantle,
}

impl Choice {
    /// "ravage", "fortify" or "dismantle"; any other word is none.
    pub fn named(word: &str) -> Option<Self> {
        match word {
            "ravage" => Some(Self::Ravage),
            "fortify" => Some(Self::Fortify),
            "dismantle" => Some(Self::Dismantle),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Harvest {
    node: Node,
    ravaged: bool,
    ticks_since_capture: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Fort {
    Awaiting,
    Fortified,
    Dismantled,
}

/// The captured nodes, by their index on the lane.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Captures {
    harvests: BTreeMap<i64, Harvest>,
    forts: BTreeMap<i64, (Node, Fort)>,
}

impl Captures {
    pub fn captured(&mut self, index: i64, node: &Node) {
        match node.node_type {
            NodeType::Resource => {
                let harvest =
                    Harvest { node: node.clone(), ravaged: false, ticks_since_capture: 0 };
                self.harvests.insert(index, harvest);
            }
            NodeType::Fort => {
                self.forts.insert(index, (node.clone(), Fort::Awaiting));
            }
            _ => {}
        }
    }

    pub fn is_awaiting_choice(&self, index: i64) -> bool {
        self.forts.get(&index).is_some_and(|(_, fort)| *fort == Fort::Awaiting)
    }

    /// The player's `choice` for node `index`; nothing when the node holds no such choice.
    pub fn choose(&mut self, index: i64, choice: Choice, economy: &mut Economy) {
        match choice {
            Choice::Ravage => self.ravage(index, economy),
            Choice::Fortify => self.fortify(index, economy),
            Choice::Dismantle => self.dismantle(index, economy),
        }
    }

    /// A tick: each harvesting node adds its yield.
    pub fn tick(&mut self, economy: &mut Economy) {
        for harvest in self.harvests.values_mut().filter(|harvest| !harvest.ravaged) {
            harvest.ticks_since_capture += 1;
            let node = &harvest.node;
            let amount = decayed_yield(
                node.yield_food_per_tick,
                harvest.ticks_since_capture,
                node.decay_interval_ticks,
                node.decay_floor_food,
            );
            economy.add("food", amount);
        }
    }

    fn ravage(&mut self, index: i64, economy: &mut Economy) {
        if let Some(harvest) = self.harvests.get_mut(&index).filter(|harvest| !harvest.ravaged) {
            economy.add("food", harvest.node.ravage_yield_food);
            harvest.ravaged = true;
        }
    }

    fn fortify(&mut self, index: i64, economy: &mut Economy) {
        let Some((node, fort)) = self.awaiting(index) else {
            return;
        };
        let (wood, stone) = (node.fortify_wood_cost, node.fortify_stone_cost);
        if economy.balance("wood") < wood || economy.balance("stone") < stone {
            return;
        }
        economy.spend("wood", wood);
        economy.spend("stone", stone);
        *fort = Fort::Fortified;
    }

    fn dismantle(&mut self, index: i64, economy: &mut Economy) {
        let Some((node, fort)) = self.awaiting(index) else {
            return;
        };
        economy.add("wood", node.dismantle_wood_yield);
        economy.add("stone", node.dismantle_stone_yield);
        *fort = Fort::Dismantled;
    }

    /// The fort on node `index`, when it awaits a choice.
    fn awaiting(&mut self, index: i64) -> Option<(&Node, &mut Fort)> {
        let (node, fort) = self.forts.get_mut(&index)?;
        (*fort == Fort::Awaiting).then_some((&*node, fort))
    }
}
