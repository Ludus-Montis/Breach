//! The lane (Decision 4: abstracted lane capture; spec 39 step 9b-i), ported from GDScript's
//! LaneSimulation, CombatResolver and PlayerRoster as they were.
//! - A lane is its nodes in order, each with an owner ("" at the start) and its garrison's
//!   pooled hp. Waves (a side's units) and moving blockers (a side's one pooled blocker)
//!   march one node a tick in their direction unless engaged.
//! - A wave that meets another side's moving blocker on its node clashes with it; otherwise a
//!   wave on a node it doesn't own fights the garrison, or takes the node when it has none
//!   left. A fight is `resolve`: the horde strikes first; a blocker it doesn't destroy
//!   strikes back at the weakest unit first, the overkill spilling to the next.
//! - Both sides of an undecided fight are held in place; a wave with no units left is gone.
//! - A side's units are those marching in its waves and those it has set to harvest.
//!
//! A tick is `march` and then `arrive` for each wave it listed, in that order, so that
//! whoever reads the events between arrivals (Godot's handlers) acts where GDScript's did. A
//! wave despawned during the tick still arrives, as GDScript's iterated copy held it. Pure.

#[cfg(test)]
mod tests;

mod binding;

use std::collections::BTreeMap;

use crate::content::Node;

/// One of a horde's units.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Unit {
    pub hp: i64,
    pub dmg: i64,
}

/// A pooled blocker: a garrison or a moving blocker.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Blocker {
    pub hp: i64,
    pub dmg: i64,
}

/// A fight's outcome (CombatResolver.resolve's dictionary).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outcome {
    pub blocker_destroyed: bool,
    pub remaining_blocker_hp: i64,
    pub surviving_horde: Vec<Unit>,
}

/// A side's wave of units.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Wave {
    pub id: i64,
    pub owner: String,
    pub units: Vec<Unit>,
    pub position: i64,
    pub direction: i64,
    pub engaged: bool,
}

/// A side's moving blocker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mover {
    pub id: i64,
    pub owner: String,
    pub blocker: Blocker,
    pub position: i64,
    pub direction: i64,
    pub engaged: bool,
}

/// What an arrival tells (SimEvents' `combat_resolved` and `node_captured`), in order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// A fight on `node`; `mover` the moving blocker it was with, if any.
    Combat {
        node: i64,
        outcome: Outcome,
        mover: Option<i64>,
    },
    Captured {
        node: i64,
    },
}

/// The horde against the blocker: the horde strikes first, with every unit's damage; a
/// blocker left standing strikes back at the weakest unit first, spilling its overkill.
/// Units of equal hp keep their order (GDScript's sort, an insertion sort up to 16 units,
/// did too; a wave holds at most 3).
pub fn resolve(horde: &[Unit], blocker: Blocker) -> Outcome {
    let horde_dmg: i64 = horde.iter().map(|unit| unit.dmg).sum();
    let remaining = blocker.hp - horde_dmg;
    if remaining <= 0 {
        return Outcome {
            blocker_destroyed: true,
            remaining_blocker_hp: 0,
            surviving_horde: horde.to_vec(),
        };
    }
    Outcome {
        blocker_destroyed: false,
        remaining_blocker_hp: remaining,
        surviving_horde: weakest_first(horde, blocker.dmg),
    }
}

fn weakest_first(horde: &[Unit], mut pool: i64) -> Vec<Unit> {
    let mut sorted = horde.to_vec();
    sorted.sort_by_key(|unit| unit.hp);
    // The weakest is always first: a unit that dies leaves the next weakest there.
    while pool > 0 && !sorted.is_empty() {
        let hp = sorted[0].hp - pool;
        if hp <= 0 {
            pool = -hp;
            sorted.remove(0);
        } else {
            sorted[0].hp = hp;
            pool = 0;
        }
    }
    sorted
}

/// Where a listed wave is: still on the lane, or despawned during this tick.
#[derive(Clone, Copy)]
enum At {
    Live(usize),
    Gone(usize),
}

/// One lane's state.
#[derive(Clone, Debug, Default)]
pub struct Lane {
    garrisons: Vec<Blocker>,
    manned: Vec<bool>,
    owners: Vec<String>,
    waves: Vec<Wave>,
    departed: Vec<Wave>,
    movers: Vec<Mover>,
    harvesting: BTreeMap<String, i64>,
    next_id: i64,
}

impl Lane {
    /// A lane over `nodes`, in order, none owned.
    pub fn new(nodes: &[Node]) -> Lane {
        Lane {
            garrisons: nodes
                .iter()
                .map(|node| Blocker { hp: node.garrison_hp, dmg: node.garrison_dmg })
                .collect(),
            manned: nodes.iter().map(|node| node.garrison > 0).collect(),
            owners: vec![String::new(); nodes.len()],
            ..Lane::default()
        }
    }

    /// Node `index`'s owner ("" none) and its garrison's hp.
    pub fn node(&self, index: usize) -> (&str, i64) {
        (&self.owners[index], self.garrisons[index].hp)
    }

    pub fn waves(&self) -> &[Wave] {
        &self.waves
    }

    pub fn movers(&self) -> &[Mover] {
        &self.movers
    }

    pub fn spawn_wave(&mut self, owner: &str, units: Vec<Unit>, start: i64, direction: i64) -> i64 {
        let id = self.made_id();
        let owner = owner.to_string();
        self.waves.push(Wave { id, owner, units, position: start, direction, engaged: false });
        id
    }

    /// Takes wave `id` off the lane; one listed for this tick still arrives.
    pub fn despawn_wave(&mut self, id: i64) {
        if let Some(index) = self.waves.iter().position(|wave| wave.id == id) {
            self.departed.push(self.waves.remove(index));
        }
    }

    pub fn spawn_mover(
        &mut self,
        owner: &str,
        blocker: Blocker,
        start: i64,
        direction: i64,
    ) -> i64 {
        let id = self.made_id();
        let owner = owner.to_string();
        self.movers.push(Mover { id, owner, blocker, position: start, direction, engaged: false });
        id
    }

    /// Mover `id`'s blocker, set from outside (Godot's dictionary is its truth).
    pub fn set_blocker(&mut self, id: i64, blocker: Blocker) {
        if let Some(mover) = self.movers.iter_mut().find(|mover| mover.id == id) {
            mover.blocker = blocker;
        }
    }

    /// `owner`'s waves on node `index` set to harvest: their units counted as `owner`'s, the
    /// waves despawned.
    pub fn harvest(&mut self, index: i64, owner: &str) {
        let at: Vec<i64> = self
            .waves
            .iter()
            .filter(|wave| wave.position == index && wave.owner == owner)
            .map(|wave| wave.id)
            .collect();
        for id in at {
            let count = self.waves.iter().find(|wave| wave.id == id).map_or(0, |w| w.units.len());
            *self.harvesting.entry(owner.to_string()).or_insert(0) += count as i64;
            self.despawn_wave(id);
        }
    }

    /// `owner`'s units: those harvesting and those marching.
    pub fn units_of(&self, owner: &str) -> i64 {
        let marching: usize =
            self.waves.iter().filter(|wave| wave.owner == owner).map(|w| w.units.len()).sum();
        self.harvesting.get(owner).copied().unwrap_or(0) + marching as i64
    }

    /// Every wave and mover not engaged steps along its direction; the waves on the lane,
    /// in order, are those to `arrive` this tick.
    pub fn march(&mut self) -> Vec<i64> {
        self.departed.clear();
        for wave in self.waves.iter_mut().filter(|wave| !wave.engaged) {
            wave.position += wave.direction;
        }
        for mover in self.movers.iter_mut().filter(|mover| !mover.engaged) {
            mover.position += mover.direction;
        }
        self.waves.iter().map(|wave| wave.id).collect()
    }

    /// Wave `id` where it stands: a clash with another side's mover there, else a fight
    /// for, or the taking of, a node it doesn't own.
    pub fn arrive(&mut self, id: i64) -> Vec<Event> {
        let mut events = Vec::new();
        let Some(at) = self.find(id) else { return events };
        let (index, owner) = {
            let wave = self.wave(at);
            (wave.position, wave.owner.clone())
        };
        if let Some(mover) =
            self.movers.iter().position(|mover| mover.owner != owner && mover.position == index)
        {
            self.clash(at, mover, index, &mut events);
            return events;
        }
        if index < 0 || index as usize >= self.owners.len() || self.owners[index as usize] == owner
        {
            return events;
        }
        self.fight_for(at, index as usize, &mut events);
        events
    }

    fn clash(&mut self, at: At, mover: usize, index: i64, events: &mut Vec<Event>) {
        let blocker = self.movers[mover].blocker;
        let outcome = resolve(&self.wave(at).units, blocker);
        let mover_id = self.movers[mover].id;
        events.push(Event::Combat { node: index, outcome: outcome.clone(), mover: Some(mover_id) });
        self.movers[mover].blocker.hp = outcome.remaining_blocker_hp;
        let held = !outcome.blocker_destroyed;
        if held {
            self.movers[mover].engaged = true;
        } else {
            self.movers.remove(mover);
        }
        let wave = self.wave(at);
        wave.units = outcome.surviving_horde;
        wave.engaged = held;
        self.drop_if_empty(at);
    }

    fn fight_for(&mut self, at: At, index: usize, events: &mut Vec<Event>) {
        if !self.manned[index] || self.garrisons[index].hp <= 0 {
            self.owners[index] = self.wave(at).owner.clone();
            events.push(Event::Captured { node: index as i64 });
            return;
        }
        let garrison = self.garrisons[index];
        let outcome = resolve(&self.wave(at).units, garrison);
        events.push(Event::Combat { node: index as i64, outcome: outcome.clone(), mover: None });
        self.garrisons[index].hp = outcome.remaining_blocker_hp;
        let held = !outcome.blocker_destroyed;
        let wave = self.wave(at);
        wave.units = outcome.surviving_horde;
        wave.engaged = held;
        if !held {
            self.owners[index] = self.wave(at).owner.clone();
            events.push(Event::Captured { node: index as i64 });
        }
        self.drop_if_empty(at);
    }

    /// A wave left with no units is gone (one already despawned stays so).
    fn drop_if_empty(&mut self, at: At) {
        if let At::Live(index) = at {
            if self.waves[index].units.is_empty() {
                self.waves.remove(index);
            }
        }
    }

    fn find(&self, id: i64) -> Option<At> {
        if let Some(index) = self.waves.iter().position(|wave| wave.id == id) {
            return Some(At::Live(index));
        }
        self.departed.iter().rposition(|wave| wave.id == id).map(At::Gone)
    }

    fn wave(&mut self, at: At) -> &mut Wave {
        match at {
            At::Live(index) => &mut self.waves[index],
            At::Gone(index) => &mut self.departed[index],
        }
    }

    fn made_id(&mut self) -> i64 {
        self.next_id += 1;
        self.next_id
    }
}
