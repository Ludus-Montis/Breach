//! The bodies' state, owned natively and kept across ticks (Decision 129): who is on the
//! field and what never changes about them (unit id, radius, footprint area, initiative,
//! the seeded draw, faction, squad), the bodies in draw order, and where each stands as of
//! the last sync. The roster changes incrementally: a sync names each squad's living units,
//! and only units new to the field are enrolled (GDScript hands over their draw and
//! sizes once), units gone are dropped, and units that changed squad are moved. The
//! hot passes - body parting, the scrum's slot search - run on it, and so can the next
//! ports (the spatial grid, pathfinding).
//!
//! A body is also a slot of the battle's unit store (spec 35, Decision 142): a unit the
//! store holds keeps its slot while it is off the field (dead, downed, gone home), and
//! is enrolled again when a sync names it. Free slots are reused lowest first. A slot is
//! storage only: no pass loops in slot order or breaks a tie by it. The store's columns
//! (`Columns`: where each unit stands, its flag bits, and its loose and fleeing entries'
//! keys) live here by slot, so a sync reads them where they are instead of GDScript
//! sending them.

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};

use crate::columns::Columns;
use crate::maths::V2;

/// A body is loose (in its squad's `loose`).
pub const LOOSE: u8 = 1;
/// A body is fleeing (in its squad's `fleeing`): it stands at its base plus an offset.
pub const FLEEING: u8 = 2;
/// A sync of the scrum's snapshot: each body loose or not, and its bearing (BodyFieldSync.scrum).
pub const SCRUM: u8 = 1;
/// A sync of the bodies as UnitBodies sees them: loose or fleeing, a loose one's next point
/// (BodyFieldSync.bodies).
pub const BODIES: u8 = 2;
/// A squad is routing.
pub const ROUTING: u8 = 1;
/// A squad is destroyed.
pub const DESTROYED: u8 = 2;

/// A squad as last synced.
#[derive(Default, Clone)]
pub struct Squad {
    pub id: i64,
    pub faction: u32,
    pub flags: u8,
    /// Its living units' bodies, in its list's order.
    pub members: Vec<u32>,
}

/// One sync's input, flat over the squads in list order (see `Field::sync`).
pub struct SyncIn<'a> {
    pub squad_ids: &'a [i64],
    pub squad_factions: &'a [String],
    pub squad_flags: &'a [u8],
    pub squad_counts: &'a [i32],
    pub unit_ids: &'a [i64],
    /// Which snapshot (SCRUM, BODIES; 0: where each stands only). How each body stands
    /// (LOOSE, FLEEING), its next point and its bearing are read from the unit store's
    /// columns (spec 35 step 4e): the flag bits are set exactly while its own squad holds
    /// the entry.
    pub mode: u8,
    /// A fleeing body's base on its route (its squad's route is GDScript's); empty:
    /// unchanged.
    pub bases: &'a [V2],
}

/// What a newly seen body needs once: GDScript's values, so no rule is duplicated here.
pub struct Enrol<'a> {
    pub flat: &'a [i32],
    pub radii: &'a [f64],
    pub areas: &'a [f64],
    pub draws: &'a [i64],
    pub initiatives: &'a [i64],
    /// Its record (spec 35 step 4b), 3 per unit: preferred position, footprint width and
    /// depth. Empty: unchanged.
    pub records: &'a [i64],
    /// Its paces (spec 35 step 4c), 2 per unit: turn rate (degrees a second) and backward
    /// pace. Empty: unchanged.
    pub paces: &'a [f64],
}

#[derive(Default)]
pub struct Field {
    seed: i64,
    stamp: u32,
    // Per body (a slot, reused once its unit leaves the field).
    pub unit_id: Vec<i64>,
    pub squad: Vec<u32>,
    pub faction: Vec<u32>,
    pub radius: Vec<f64>,
    pub area: Vec<f64>,
    pub draw: Vec<i64>,
    pub initiative: Vec<i64>,
    /// The unit's record, which never changes (spec 35 steps 4b and 4c): its preferred band
    /// (UnitDef.Position, 0 the front), its footprint's width and depth in cells, its turn
    /// rate and its backward pace.
    pub preferred: Vec<i64>,
    pub foot_width: Vec<i64>,
    pub foot_depth: Vec<i64>,
    pub turn_rate: Vec<f64>,
    pub backward_pace: Vec<f64>,
    seen: Vec<u32>,
    enrolled: Vec<bool>,
    // Where each body stands and how, as of the last sync (or a pass that moved it).
    pub at: Vec<V2>,
    pub offset: Vec<V2>,
    pub base: Vec<V2>,
    pub next: Vec<V2>,
    pub flags: Vec<u8>,
    pub bearing: Vec<f64>,
    /// The flat index each body had in the last sync (how results go back to GDScript).
    pub flat_of: Vec<u32>,
    /// The body at each flat index of the last sync.
    pub flat: Vec<u32>,
    pub squads: Vec<Squad>,
    /// The squads in the last sync's order.
    pub order: Vec<u32>,
    /// The enrolled bodies on the field, by draw (UnitBodies._drawn's order).
    pub drawn: Vec<u32>,
    /// The bodies on the field (named by the last sync).
    by_unit: HashMap<i64, u32>,
    by_squad: HashMap<i64, u32>,
    factions: Vec<String>,
    /// The unit store's slots: units held whether or not they are on the field.
    held: HashMap<i64, u32>,
    holding: Vec<bool>,
    free: BinaryHeap<Reverse<u32>>,
    /// The unit store's columns, by slot.
    pub columns: Columns,
}

impl Field {
    /// Bodies on the field.
    pub fn count(&self) -> usize {
        self.by_unit.len()
    }

    /// The body of the unit with this id, if the last sync named it.
    pub fn body_of(&self, unit_id: i64) -> Option<u32> {
        self.by_unit.get(&unit_id).copied()
    }

    /// Units the store holds.
    pub fn held_count(&self) -> usize {
        self.held.len()
    }

    /// The field's index of the squad with this id, if a sync named it.
    pub fn squad_index(&self, squad_id: i64) -> Option<u32> {
        self.by_squad.get(&squad_id).copied()
    }

    /// The store's slot of a unit, if it holds it.
    pub fn slot_of(&self, unit_id: i64) -> Option<u32> {
        self.held.get(&unit_id).copied()
    }

    /// The unit the store holds in a slot, if any.
    pub fn unit_at(&self, slot: u32) -> Option<i64> {
        let s = slot as usize;
        (s < self.holding.len() && self.holding[s]).then(|| self.unit_id[s])
    }

    /// The units the store holds, by slot.
    pub fn held_units(&self) -> Vec<i64> {
        (0..self.holding.len()).filter(|&s| self.holding[s]).map(|s| self.unit_id[s]).collect()
    }

    /// The store takes a unit: its slot, the lowest free one if it has none yet (its body,
    /// if a sync named it first).
    pub fn hold(&mut self, unit_id: i64) -> u32 {
        if let Some(&slot) = self.held.get(&unit_id) {
            return slot;
        }
        let slot = match self.by_unit.get(&unit_id) {
            Some(&body) => body,
            None => self.take_slot(unit_id),
        };
        self.holding[slot as usize] = true;
        self.held.insert(unit_id, slot);
        self.columns.blank(slot as usize);
        slot
    }

    /// The store lets a unit go: it leaves the field too, and its slot is free.
    pub fn release(&mut self, unit_id: i64) {
        let Some(slot) = self.held.remove(&unit_id) else { return };
        self.holding[slot as usize] = false;
        if self.by_unit.remove(&unit_id).is_some() {
            self.enrolled[slot as usize] = false;
            self.drawn.retain(|&b| b != slot);
        }
        self.free.push(Reverse(slot));
    }

    /// Brings the field to the squads as they stand. Returns the flat indices of units new
    /// to the field, to be enrolled (`enrol`) before any pass runs.
    pub fn sync(&mut self, seed: i64, input: &SyncIn) -> Vec<i32> {
        if seed != self.seed {
            self.seed = seed;
            self.forget_bodies(); // enrolled again, with this seed's draws
        }
        self.stamp = self.stamp.wrapping_add(1);
        self.order.clear();
        self.flat.clear();
        let mut unknown = Vec::new();
        let mut flat = 0usize;
        for (s, &squad_id) in input.squad_ids.iter().enumerate() {
            let faction = self.faction_index(&input.squad_factions[s]);
            let sq = self.squad_slot(squad_id);
            self.order.push(sq);
            let mut members = std::mem::take(&mut self.squads[sq as usize].members);
            members.clear();
            for _ in 0..input.squad_counts[s] {
                let body = self.body_for(input.unit_ids[flat], &mut unknown, flat);
                self.place(body, flat, sq, faction, input);
                members.push(body);
                flat += 1;
            }
            let squad = &mut self.squads[sq as usize];
            squad.members = members;
            squad.faction = faction;
            squad.flags = input.squad_flags[s];
        }
        self.drop_unseen();
        unknown
    }

    /// Enrols the bodies `sync` reported new: their sizes, initiative and draw.
    pub fn enrol(&mut self, input: &Enrol) {
        for (k, &flat) in input.flat.iter().enumerate() {
            let body = self.flat[flat as usize];
            let b = body as usize;
            self.radius[b] = input.radii[k];
            self.area[b] = input.areas[k];
            self.draw[b] = input.draws[k];
            self.initiative[b] = input.initiatives[k];
            if !input.records.is_empty() {
                self.preferred[b] = input.records[3 * k];
                self.foot_width[b] = input.records[3 * k + 1];
                self.foot_depth[b] = input.records[3 * k + 2];
            }
            if !input.paces.is_empty() {
                self.turn_rate[b] = input.paces[2 * k];
                self.backward_pace[b] = input.paces[2 * k + 1];
            }
            if !self.enrolled[b] {
                self.enrolled[b] = true;
                let draw = self.draw[b];
                let at = self.drawn.partition_point(|&o| self.draw[o as usize] <= draw);
                self.drawn.insert(at, body);
            }
        }
    }

    fn faction_index(&mut self, name: &str) -> u32 {
        match self.factions.iter().position(|f| f == name) {
            Some(i) => i as u32,
            None => {
                self.factions.push(name.to_string());
                (self.factions.len() - 1) as u32
            }
        }
    }

    fn squad_slot(&mut self, id: i64) -> u32 {
        if let Some(&sq) = self.by_squad.get(&id) {
            return sq;
        }
        self.squads.push(Squad { id, ..Squad::default() });
        let sq = (self.squads.len() - 1) as u32;
        self.by_squad.insert(id, sq);
        sq
    }

    /// The body of a unit, a new slot for one not on the field yet (noted in `unknown`).
    fn body_for(&mut self, unit_id: i64, unknown: &mut Vec<i32>, flat: usize) -> u32 {
        if let Some(&body) = self.by_unit.get(&unit_id) {
            return body;
        }
        unknown.push(flat as i32);
        let body = match self.held.get(&unit_id) {
            Some(&slot) => slot,
            None => self.take_slot(unit_id),
        };
        self.enrolled[body as usize] = false;
        self.by_unit.insert(unit_id, body);
        body
    }

    /// The lowest free slot, or a new one, given to a unit.
    fn take_slot(&mut self, unit_id: i64) -> u32 {
        let slot = match self.free.pop() {
            Some(Reverse(s)) => s,
            None => {
                self.grow();
                (self.unit_id.len() - 1) as u32
            }
        };
        self.unit_id[slot as usize] = unit_id;
        slot
    }

    /// Takes every body off the field; the store keeps its slots.
    fn forget_bodies(&mut self) {
        let bodies: Vec<u32> = self.by_unit.values().copied().collect();
        self.by_unit.clear();
        self.drawn.clear();
        for body in bodies {
            self.enrolled[body as usize] = false;
            if !self.holding[body as usize] {
                self.free.push(Reverse(body));
            }
        }
    }

    fn grow(&mut self) {
        self.unit_id.push(0);
        self.squad.push(0);
        self.faction.push(0);
        self.radius.push(0.0);
        self.area.push(0.0);
        self.draw.push(0);
        self.initiative.push(0);
        self.preferred.push(0);
        self.foot_width.push(1);
        self.foot_depth.push(1);
        self.turn_rate.push(0.0);
        self.backward_pace.push(0.0);
        self.seen.push(0);
        self.enrolled.push(false);
        self.holding.push(false);
        self.at.push(V2::ZERO);
        self.offset.push(V2::ZERO);
        self.base.push(V2::ZERO);
        self.next.push(V2::ZERO);
        self.flags.push(0);
        self.bearing.push(0.0);
        self.flat_of.push(0);
        self.columns.grow();
    }

    /// Writes a synced unit's state into its body.
    fn place(&mut self, body: u32, flat: usize, sq: u32, faction: u32, input: &SyncIn) {
        let b = body as usize;
        self.seen[b] = self.stamp;
        self.squad[b] = sq;
        self.faction[b] = faction;
        self.flat_of[b] = flat as u32;
        self.flat.push(body);
        let mut flags = self.columns.flags[b] as u8 & (LOOSE | FLEEING);
        if input.mode == SCRUM {
            flags &= LOOSE; // a router: where it stands
            self.bearing[b] = self.columns.bearing[b];
        } else if input.mode == BODIES {
            let walking = flags & LOOSE != 0 && flags & FLEEING == 0;
            self.next[b] = if walking { self.columns.next[b] } else { V2::ZERO };
        }
        self.flags[b] = flags;
        if !input.bases.is_empty() {
            self.base[b] = input.bases[flat];
        }
        if flags & FLEEING != 0 {
            self.offset[b] = self.columns.flee_offset[b];
            self.at[b] = self.base[b].add(self.offset[b]);
        } else if flags & LOOSE != 0 {
            self.at[b] = self.columns.loose_at[b];
        } else {
            self.at[b] = self.columns.position[b];
        }
    }

    /// Drops the bodies no squad named this sync: the dead, the taken, the gone.
    fn drop_unseen(&mut self) {
        if self.flat.len() == self.by_unit.len() {
            return; // every body on the field was named (bodies are named once each)
        }
        let stamp = self.stamp;
        let gone: Vec<u32> = self
            .by_unit
            .values()
            .copied()
            .filter(|&b| self.seen[b as usize] != stamp)
            .collect();
        for body in gone {
            self.by_unit.remove(&self.unit_id[body as usize]);
            self.enrolled[body as usize] = false;
            if !self.holding[body as usize] {
                self.free.push(Reverse(body)); // a held unit keeps its slot off the field
            }
        }
        let (seen, enrolled) = (&self.seen, &self.enrolled);
        self.drawn.retain(|&b| seen[b as usize] == stamp && enrolled[b as usize]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sync(field: &mut Field, units: &[i64]) -> Vec<i32> {
        sync_seeing(field, units, &vec![0; units.len()], &[])
    }

    /// Syncs the bodies, each held unit's flag bits set to `flags` in the store first.
    fn sync_seeing(field: &mut Field, units: &[i64], flags: &[u8], bases: &[V2]) -> Vec<i32> {
        for (k, &unit) in units.iter().enumerate() {
            if let Some(slot) = field.slot_of(unit) {
                field.columns.flags[slot as usize] = flags[k] as i32;
            }
        }
        let input = SyncIn {
            squad_ids: &[1],
            squad_factions: &["player".to_string()],
            squad_flags: &[0],
            squad_counts: &[units.len() as i32],
            unit_ids: units,
            mode: BODIES,
            bases,
        };
        field.sync(5, &input)
    }

    fn v(x: f32, y: f32) -> V2 {
        V2 { x, y }
    }

    #[test]
    fn a_sync_reads_where_each_unit_stands_from_the_columns() {
        let mut field = Field::default();
        let slots = [field.hold(1), field.hold(2), field.hold(3)].map(|s| s as usize);
        field.columns.position[slots[0]] = v(1.0, 1.0); // in its frame
        field.columns.position[slots[1]] = v(9.0, 9.0);
        field.columns.loose_at[slots[1]] = v(2.0, 2.0); // loose: where it stands loose
        field.columns.flee_offset[slots[2]] = v(0.5, 0.0); // fleeing: off its base
        let bases = [V2::ZERO, V2::ZERO, v(10.0, 0.0)];
        sync_seeing(&mut field, &[1, 2, 3], &[0, LOOSE, LOOSE | FLEEING], &bases);
        assert_eq!(slots.map(|b| field.at[b]), [v(1.0, 1.0), v(2.0, 2.0), v(10.5, 0.0)]);
        sync_seeing(&mut field, &[1, 2, 3], &[0, LOOSE, LOOSE], &[]); // the scrum's
        assert_eq!(field.at[slots[2]], V2::ZERO); // where it stands loose: never set
    }

    #[test]
    fn a_slot_given_again_starts_blank() {
        let mut field = Field::default();
        let slot = field.hold(1) as usize;
        field.columns.position[slot] = v(3.0, 4.0);
        field.columns.flags[slot] = FLEEING as i32;
        field.release(1);
        assert_eq!(field.hold(2) as usize, slot);
        assert_eq!(field.columns.position[slot], V2::ZERO);
        assert_eq!(field.columns.flags[slot], 0);
    }

    #[test]
    fn held_units_keep_their_slots_off_the_field_and_free_slots_go_lowest_first() {
        let mut field = Field::default();
        assert_eq!([field.hold(10), field.hold(11), field.hold(12)], [0, 1, 2]);
        assert_eq!(sync(&mut field, &[10, 11, 12]), vec![0, 1, 2]);
        assert_eq!(field.body_of(11), Some(1)); // the body is the slot
        sync(&mut field, &[10, 12]); // 11 died: off the field, still held
        assert_eq!(field.count(), 2);
        assert_eq!(field.slot_of(11), Some(1));
        assert_eq!(field.unit_at(1), Some(11));
        field.release(12);
        field.release(10);
        assert_eq!(field.unit_at(0), None);
        assert_eq!(field.hold(13), 0); // the lowest free slot
        assert_eq!(field.hold(14), 2);
        assert_eq!(field.held_units(), vec![13, 11, 14]);
    }

    #[test]
    fn a_unit_the_field_names_again_is_enrolled_again_in_its_held_slot() {
        let mut field = Field::default();
        field.hold(7);
        sync(&mut field, &[7]);
        sync(&mut field, &[]);
        assert_eq!(sync(&mut field, &[7]), vec![0]); // come to: enrolled again
        assert_eq!(field.body_of(7), Some(0));
    }
}
