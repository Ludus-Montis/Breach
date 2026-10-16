//! The unit store's columns (spec 35, Decision 142; GDScript's UnitColumns), by slot: where
//! each unit stands (its position, where it stands loose, its place off its route), its
//! flag bits (LOOSE, FLEEING), and every other key of its loose and fleeing entries (step
//! 2: its next step, the foe and slot it seeks, that foe's place, whether it touches a
//! foe, where it walks toward, how long it has been stuck and its closest yet; how far
//! along its route it has fled, how long a friend has held it and which, how far it has
//! fanned out), with the bits saying which keys an entry holds (`keys`), and the way it
//! faces (`bearing`, step 3: degrees clockwise from north), and its place in its squad's
//! frame (`rank` and `column`, step 4a), and its hp, the ticks till its next blow and the
//! hp it may still regenerate before it rests (`hp`, `attack_cooldown`,
//! `regeneration_left`, step 5b). The field holds
//! them so its passes read them where they live. A slot newly given starts blank, each
//! column at the value a missing key reads as. Columns are found by their UnitColumns name.

use crate::maths::V2;

#[derive(Default)]
pub struct Columns {
    pub position: Vec<V2>,
    pub loose_at: Vec<V2>,
    pub flee_offset: Vec<V2>,
    pub next: Vec<V2>,
    pub foe_at: Vec<V2>,
    pub toward: Vec<V2>,
    pub flags: Vec<i32>,
    pub touch: Vec<i32>,
    pub keys: Vec<i32>,
    pub rank: Vec<i32>,
    pub column: Vec<i32>,
    pub hp: Vec<i32>,
    pub attack_cooldown: Vec<i32>,
    pub goal_foe: Vec<i64>,
    pub goal_slot: Vec<i64>,
    pub caught: Vec<i64>,
    pub held_by: Vec<i64>,
    pub stuck: Vec<f64>,
    pub closest: Vec<f64>,
    pub along: Vec<f64>,
    pub fanned: Vec<f64>,
    pub bearing: Vec<f64>,
    pub regeneration_left: Vec<f64>,
}

/// No goal (`goal_slot`).
const NO_GOAL: i64 = -1;
/// A unit not yet turned faces east (`bearing`).
const EAST: f64 = 90.0;

impl Columns {
    /// A point column by its name.
    pub fn points(&mut self, name: &str) -> Option<&mut Vec<V2>> {
        match name {
            "position" => Some(&mut self.position),
            "at" => Some(&mut self.loose_at),
            "offset" => Some(&mut self.flee_offset),
            "next" => Some(&mut self.next),
            "foe_at" => Some(&mut self.foe_at),
            "toward" => Some(&mut self.toward),
            _ => None,
        }
    }

    /// A 32-bit int column by its name.
    pub fn ints(&mut self, name: &str) -> Option<&mut Vec<i32>> {
        match name {
            "flags" => Some(&mut self.flags),
            "touch" => Some(&mut self.touch),
            "keys" => Some(&mut self.keys),
            "rank" => Some(&mut self.rank),
            "column" => Some(&mut self.column),
            "hp" => Some(&mut self.hp),
            "attack_cooldown" => Some(&mut self.attack_cooldown),
            _ => None,
        }
    }

    /// A 64-bit int column by its name.
    pub fn longs(&mut self, name: &str) -> Option<&mut Vec<i64>> {
        match name {
            "goal_foe" => Some(&mut self.goal_foe),
            "goal_slot" => Some(&mut self.goal_slot),
            "caught" => Some(&mut self.caught),
            "held_by" => Some(&mut self.held_by),
            _ => None,
        }
    }

    /// A float column by its name.
    pub fn reals(&mut self, name: &str) -> Option<&mut Vec<f64>> {
        match name {
            "stuck" => Some(&mut self.stuck),
            "closest" => Some(&mut self.closest),
            "along" => Some(&mut self.along),
            "fanned" => Some(&mut self.fanned),
            "bearing" => Some(&mut self.bearing),
            "regeneration_left" => Some(&mut self.regeneration_left),
            _ => None,
        }
    }

    /// One more slot, blank.
    pub fn grow(&mut self) {
        for column in [
            &mut self.position,
            &mut self.loose_at,
            &mut self.flee_offset,
            &mut self.next,
            &mut self.foe_at,
            &mut self.toward,
        ] {
            column.push(V2::ZERO);
        }
        for column in [
            &mut self.flags,
            &mut self.touch,
            &mut self.keys,
            &mut self.rank,
            &mut self.column,
            &mut self.hp,
            &mut self.attack_cooldown,
        ] {
            column.push(0);
        }
        for column in [&mut self.goal_foe, &mut self.caught, &mut self.held_by] {
            column.push(0);
        }
        self.goal_slot.push(NO_GOAL);
        for column in
            [&mut self.stuck, &mut self.along, &mut self.fanned, &mut self.regeneration_left]
        {
            column.push(0.0);
        }
        self.closest.push(f64::INFINITY);
        self.bearing.push(EAST);
    }

    /// Every column blank in `slot`.
    pub fn blank(&mut self, slot: usize) {
        for column in [
            &mut self.position,
            &mut self.loose_at,
            &mut self.flee_offset,
            &mut self.next,
            &mut self.foe_at,
            &mut self.toward,
        ] {
            column[slot] = V2::ZERO;
        }
        for column in [
            &mut self.flags,
            &mut self.touch,
            &mut self.keys,
            &mut self.rank,
            &mut self.column,
            &mut self.hp,
            &mut self.attack_cooldown,
        ] {
            column[slot] = 0;
        }
        for column in [&mut self.goal_foe, &mut self.caught, &mut self.held_by] {
            column[slot] = 0;
        }
        self.goal_slot[slot] = NO_GOAL;
        for column in
            [&mut self.stuck, &mut self.along, &mut self.fanned, &mut self.regeneration_left]
        {
            column[slot] = 0.0;
        }
        self.closest[slot] = f64::INFINITY;
        self.bearing[slot] = EAST;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_slot_and_a_blanked_one_read_as_a_missing_key() {
        let mut columns = Columns::default();
        columns.grow();
        assert_eq!(columns.goal_slot[0], NO_GOAL);
        assert_eq!(columns.closest[0], f64::INFINITY);
        assert_eq!(columns.bearing[0], EAST);
        columns.points("next").unwrap()[0] = V2 { x: 1.0, y: 2.0 };
        columns.ints("keys").unwrap()[0] = 5;
        columns.longs("goal_slot").unwrap()[0] = 3;
        columns.reals("closest").unwrap()[0] = 0.5;
        columns.reals("bearing").unwrap()[0] = 237.5;
        columns.ints("rank").unwrap()[0] = 3;
        columns.ints("hp").unwrap()[0] = 12;
        columns.ints("attack_cooldown").unwrap()[0] = 4;
        columns.reals("regeneration_left").unwrap()[0] = 2.5;
        columns.blank(0);
        assert_eq!(columns.next[0], V2::ZERO);
        assert_eq!(columns.keys[0], 0);
        assert_eq!(columns.goal_slot[0], NO_GOAL);
        assert_eq!(columns.closest[0], f64::INFINITY);
        assert_eq!(columns.bearing[0], EAST);
        assert_eq!(columns.rank[0], 0);
        assert_eq!(columns.hp[0], 0);
        assert_eq!(columns.attack_cooldown[0], 0);
        assert_eq!(columns.regeneration_left[0], 0.0);
    }

    #[test]
    fn every_column_is_found_by_its_unit_columns_name_and_no_other() {
        let mut columns = Columns::default();
        for name in ["position", "at", "offset", "next", "foe_at", "toward"] {
            assert!(columns.points(name).is_some(), "{name}");
        }
        for name in ["flags", "touch", "keys", "rank", "column", "hp", "attack_cooldown"] {
            assert!(columns.ints(name).is_some(), "{name}");
        }
        for name in ["goal_foe", "goal_slot", "caught", "held_by"] {
            assert!(columns.longs(name).is_some(), "{name}");
        }
        for name in ["stuck", "closest", "along", "fanned", "bearing", "regeneration_left"] {
            assert!(columns.reals(name).is_some(), "{name}");
        }
        assert!(columns.points("flags").is_none());
        assert!(columns.reals("next").is_none());
    }
}
