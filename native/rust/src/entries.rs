//! A squad's loose and fleeing entries (SquadEntries and EntryKeys, spec 39 step 4b), kept
//! with their places in the unit store: where a loose unit stands is its `loose_at`, a
//! fleeing unit's place off its route its `flee_offset`, and its `flags` bits say whether it
//! has each entry. The squad store lists who has one (`Squads::loose`, `Squads::fleeing`, in
//! the order each was given). Entries are given and taken away only here, so the bits always
//! match the lists. An entry's other keys are the store's columns too: a key the entry
//! doesn't hold reads as its column's blank, and the `keys` column says which it holds, and
//! for `toward` and `held_by`, whether each holds a value (the digest hashes the entries
//! with exactly the keys they hold).

use crate::columns::Columns;
use crate::maths::V2;
use crate::squads::Squads;

/// `flags`' bits: the unit has a loose entry, a fleeing one.
pub const LOOSE: i32 = 1;
pub const FLEEING: i32 = 2;
/// No goal (`goal_slot`).
const NO_GOAL: i64 = -1;

/// An entry's keys, as EntryKeys names them by their columns.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Key {
    FoeAt,
    Touch,
    Toward,
    Stuck,
    Closest,
    Caught,
    HeldBy,
    Fanned,
}

/// The keys a new loose entry doesn't hold yet, and a new fleeing one.
pub const LOOSE_KEYS: [Key; 5] = [Key::FoeAt, Key::Touch, Key::Toward, Key::Stuck, Key::Closest];
pub const FLEEING_KEYS: [Key; 3] = [Key::Caught, Key::HeldBy, Key::Fanned];

/// A key's value: by its column's kind, or none (only `toward` and `held_by` hold none).
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Value {
    None,
    Int(i64),
    Real(f64),
    Point(V2),
}

impl Key {
    /// The key by its column's UnitColumns name.
    pub fn named(name: &str) -> Option<Key> {
        Some(match name {
            "foe_at" => Key::FoeAt,
            "touch" => Key::Touch,
            "toward" => Key::Toward,
            "stuck" => Key::Stuck,
            "closest" => Key::Closest,
            "caught" => Key::Caught,
            "held_by" => Key::HeldBy,
            "fanned" => Key::Fanned,
            _ => return None,
        })
    }

    /// `keys`' bit: the entry holds the key.
    fn bit(self) -> i32 {
        match self {
            Key::FoeAt => 1,
            Key::Touch => 2,
            Key::Toward => 4,
            Key::Stuck => 16,
            Key::Closest => 32,
            Key::Caught => 64,
            Key::HeldBy => 128,
            Key::Fanned => 512,
        }
    }

    /// `keys`' bit: the key holds a value, not none (0 for the keys that always do).
    fn valued(self) -> i32 {
        match self {
            Key::Toward => 8,
            Key::HeldBy => 256,
            _ => 0,
        }
    }

    /// Writes the key's column in `slot`: `value`, or its blank for none.
    fn put(self, columns: &mut Columns, slot: usize, value: Value) {
        match (self, value) {
            (Key::FoeAt, Value::Point(p)) => columns.foe_at[slot] = p,
            (Key::FoeAt, _) => columns.foe_at[slot] = V2::ZERO,
            (Key::Toward, Value::Point(p)) => columns.toward[slot] = p,
            (Key::Toward, _) => columns.toward[slot] = V2::ZERO,
            (Key::Touch, Value::Int(v)) => columns.touch[slot] = v as i32,
            (Key::Touch, _) => columns.touch[slot] = 0,
            (Key::Caught, Value::Int(v)) => columns.caught[slot] = v,
            (Key::Caught, _) => columns.caught[slot] = 0,
            (Key::HeldBy, Value::Int(v)) => columns.held_by[slot] = v,
            (Key::HeldBy, _) => columns.held_by[slot] = 0,
            (Key::Stuck, Value::Real(v)) => columns.stuck[slot] = v,
            (Key::Stuck, _) => columns.stuck[slot] = 0.0,
            (Key::Closest, Value::Real(v)) => columns.closest[slot] = v,
            (Key::Closest, _) => columns.closest[slot] = f64::INFINITY,
            (Key::Fanned, Value::Real(v)) => columns.fanned[slot] = v,
            (Key::Fanned, _) => columns.fanned[slot] = 0.0,
        }
    }
}

/// Sets the key of the entry in `slot` (`Value::None` only for `toward` and `held_by`).
pub fn keep(columns: &mut Columns, slot: usize, key: Key, value: Value) {
    let mut keys = columns.keys[slot] | key.bit();
    if value == Value::None {
        keys &= !key.valued();
    } else {
        keys |= key.valued();
    }
    key.put(columns, slot, value);
    columns.keys[slot] = keys;
}

/// Takes the keys out of the entry in `slot`: each reads blank again.
pub fn forget(columns: &mut Columns, slot: usize, names: &[Key]) {
    let mut keys = columns.keys[slot];
    for &key in names {
        keys &= !(key.bit() | key.valued());
        key.put(columns, slot, Value::None);
    }
    columns.keys[slot] = keys;
}

/// Gives the unit (`unit_id`, in store slot `slot`) a new loose entry in the squad in
/// `squad`, standing loose at `at`, its next step there and seeking no foe.
pub fn loosen(
    squads: &mut Squads,
    columns: &mut Columns,
    squad: usize,
    unit: (i64, usize),
    at: V2,
) {
    let (unit_id, slot) = unit;
    let loose = &mut squads.loose[squad];
    if !loose.contains(&unit_id) {
        loose.push(unit_id);
    }
    columns.loose_at[slot] = at;
    columns.next[slot] = at;
    columns.goal_foe[slot] = 0;
    columns.goal_slot[slot] = NO_GOAL;
    forget(columns, slot, &LOOSE_KEYS);
    columns.flags[slot] |= LOOSE;
}

/// Takes the unit's loose entry away (nothing if it has none).
pub fn unloose(squads: &mut Squads, columns: &mut Columns, squad: usize, unit: (i64, usize)) {
    let loose = &mut squads.loose[squad];
    if let Some(at) = loose.iter().position(|&id| id == unit.0) {
        loose.remove(at);
        columns.flags[unit.1] &= !LOOSE;
    }
}

/// Takes every loose entry of the squad away. `slot_of` finds a unit's store slot by its id.
pub fn unloose_all(
    squads: &mut Squads,
    columns: &mut Columns,
    squad: usize,
    mut slot_of: impl FnMut(i64) -> Option<usize>,
) {
    for unit_id in std::mem::take(&mut squads.loose[squad]) {
        if let Some(slot) = slot_of(unit_id) {
            columns.flags[slot] &= !LOOSE;
        }
    }
}

/// Gives the unit a new fleeing entry in the squad, `along` its route and `offset` off it.
pub fn flee(
    squads: &mut Squads,
    columns: &mut Columns,
    squad: usize,
    unit: (i64, usize),
    along: f64,
    offset: V2,
) {
    let (unit_id, slot) = unit;
    let fleeing = &mut squads.fleeing[squad];
    if !fleeing.contains(&unit_id) {
        fleeing.push(unit_id);
    }
    columns.flee_offset[slot] = offset;
    columns.along[slot] = along;
    forget(columns, slot, &FLEEING_KEYS);
    columns.flags[slot] |= FLEEING;
}

/// Takes the unit's fleeing entry away (nothing if it has none).
pub fn unflee(squads: &mut Squads, columns: &mut Columns, squad: usize, unit: (i64, usize)) {
    let fleeing = &mut squads.fleeing[squad];
    if let Some(at) = fleeing.iter().position(|&id| id == unit.0) {
        fleeing.remove(at);
        columns.flags[unit.1] &= !FLEEING;
    }
}

/// Takes every fleeing entry of the squad away.
pub fn unflee_all(
    squads: &mut Squads,
    columns: &mut Columns,
    squad: usize,
    mut slot_of: impl FnMut(i64) -> Option<usize>,
) {
    for unit_id in std::mem::take(&mut squads.fleeing[squad]) {
        if let Some(slot) = slot_of(unit_id) {
            columns.flags[slot] &= !FLEEING;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store(units: usize) -> (Squads, Columns) {
        let mut squads = Squads::default();
        squads.open(1);
        let mut columns = Columns::default();
        for _ in 0..units {
            columns.grow();
        }
        (squads, columns)
    }

    #[test]
    fn a_loose_entry_stands_where_it_went_loose_seeking_nothing_and_holding_no_keys() {
        let (mut squads, mut columns) = store(2);
        keep(&mut columns, 1, Key::Stuck, Value::Real(2.0));
        keep(&mut columns, 1, Key::Toward, Value::Point(V2::new(1.0, 1.0)));
        columns.goal_slot[1] = 3;
        loosen(&mut squads, &mut columns, 0, (20, 1), V2::new(4.0, 5.0));
        assert_eq!(squads.loose[0], vec![20]);
        assert_eq!(columns.loose_at[1], V2::new(4.0, 5.0));
        assert_eq!(columns.next[1], V2::new(4.0, 5.0));
        assert_eq!(columns.goal_slot[1], -1);
        assert_eq!(columns.keys[1], 0);
        assert_eq!(columns.stuck[1], 0.0);
        assert_eq!(columns.closest[1], f64::INFINITY);
        assert_eq!(columns.toward[1], V2::ZERO);
        assert_eq!(columns.flags[1], LOOSE);
        assert_eq!(columns.flags[0], 0);
    }

    #[test]
    fn loosened_again_a_unit_keeps_its_place_in_the_order() {
        let (mut squads, mut columns) = store(3);
        for (id, slot) in [(5, 0), (6, 1), (7, 2)] {
            loosen(&mut squads, &mut columns, 0, (id, slot), V2::ZERO);
        }
        loosen(&mut squads, &mut columns, 0, (5, 0), V2::new(1.0, 0.0));
        assert_eq!(squads.loose[0], vec![5, 6, 7]);
        unloose(&mut squads, &mut columns, 0, (6, 1));
        assert_eq!(squads.loose[0], vec![5, 7]);
        assert_eq!(columns.flags[1], 0);
        unloose(&mut squads, &mut columns, 0, (6, 1)); // none: nothing
        assert_eq!(squads.loose[0], vec![5, 7]);
        unloose_all(&mut squads, &mut columns, 0, |id| Some((id - 5) as usize));
        assert!(squads.loose[0].is_empty());
        assert_eq!(columns.flags, vec![0, 0, 0]);
    }

    #[test]
    fn a_fleeing_entry_has_its_place_off_the_route_and_its_own_bit() {
        let (mut squads, mut columns) = store(1);
        loosen(&mut squads, &mut columns, 0, (9, 0), V2::ZERO);
        keep(&mut columns, 0, Key::HeldBy, Value::Int(4));
        flee(&mut squads, &mut columns, 0, (9, 0), 12.5, V2::new(0.0, 2.0));
        assert_eq!(squads.fleeing[0], vec![9]);
        assert_eq!(columns.along[0], 12.5);
        assert_eq!(columns.flee_offset[0], V2::new(0.0, 2.0));
        assert_eq!((columns.held_by[0], columns.keys[0]), (0, 0));
        assert_eq!(columns.flags[0], LOOSE | FLEEING);
        unflee_all(&mut squads, &mut columns, 0, |_| Some(0));
        assert_eq!(columns.flags[0], LOOSE);
        flee(&mut squads, &mut columns, 0, (9, 0), 1.0, V2::ZERO);
        unflee(&mut squads, &mut columns, 0, (9, 0));
        assert!(squads.fleeing[0].is_empty());
        assert_eq!(columns.flags[0], LOOSE);
    }

    #[test]
    fn a_key_held_with_no_value_is_held_but_reads_blank() {
        let (_, mut columns) = store(1);
        keep(&mut columns, 0, Key::Toward, Value::Point(V2::new(3.0, 3.0)));
        assert_eq!(columns.keys[0], 4 | 8);
        keep(&mut columns, 0, Key::Toward, Value::None);
        assert_eq!(columns.keys[0], 4);
        assert_eq!(columns.toward[0], V2::ZERO);
        keep(&mut columns, 0, Key::Closest, Value::Real(1.5));
        forget(&mut columns, 0, &[Key::Toward, Key::Closest]);
        assert_eq!((columns.keys[0], columns.closest[0]), (0, f64::INFINITY));
    }
}
