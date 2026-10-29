//! A unit's numbers (spec 39 step 8b5-i; GDScript's SkirmishUnit): its sheet as the
//! simulation reads it, held by the crate in the unit store by slot (`Columns::sheet`), so
//! the snapshot carries it and a crate pass reads it where it lives. Written whole when the
//! store admits a unit (UnitStore.admit) and one number at a time after, by SkirmishUnit's
//! own setters. Nothing in the crate writes it yet. Its words, lists and objects (faction,
//! tags, parts, walker, command) are 8b5-ii's.
//!
//! GDScript asks for `NAMES` once (`sheet_names`), names each number by its index there
//! (test_unit_numbers.gd) and hands it over as a float: every int here is well under 2^53,
//! and a bool is 0 or 1.

use serde::{Deserialize, Serialize};

macro_rules! sheet {
    ($($name:ident: $kind:ident = $blank:expr),* $(,)?) => {
        /// One unit's numbers. A slot newly given starts at SkirmishUnit's own defaults.
        #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
        pub struct Sheet {
            $(pub $name: $kind,)*
        }

        impl Default for Sheet {
            fn default() -> Self {
                Sheet { $($name: $blank,)* }
            }
        }

        /// Each number's name (SkirmishUnit's property), by its index.
        pub const NAMES: &[&str] = &[$(stringify!($name),)*];

        impl Sheet {
            /// The number at `index` as a float (0 past the last).
            pub fn get(&self, index: usize) -> f64 {
                let mut at = 0;
                $(
                    if at == index {
                        return sheet!(@out $kind, self.$name);
                    }
                    at += 1;
                )*
                let _ = at;
                0.0
            }

            /// Sets the number at `index` from a float (nothing past the last).
            pub fn set(&mut self, index: usize, value: f64) {
                let mut at = 0;
                $(
                    if at == index {
                        self.$name = sheet!(@in $kind, value);
                        return;
                    }
                    at += 1;
                )*
                let _ = at;
            }
        }
    };
    (@out i64, $v:expr) => { $v as f64 };
    (@out f64, $v:expr) => { $v };
    (@out bool, $v:expr) => { if $v { 1.0 } else { 0.0 } };
    (@in i64, $v:expr) => { $v as i64 };
    (@in f64, $v:expr) => { $v };
    (@in bool, $v:expr) => { $v != 0.0 };
}

sheet! {
    max_hp: i64 = 0,
    dmg: i64 = 0,
    detection: f64 = 40.0,
    courage: i64 = 60,
    leadership: i64 = 0,
    height: f64 = 1.0,
    initiative: i64 = 10,
    discipline: i64 = 30,
    turn_rate: f64 = 450.0,
    backward_pace: f64 = 0.4,
    advance_direction: i64 = 1,
    squad_id: i64 = 0,
    footprint_depth: i64 = 1,
    footprint_width: i64 = 1,
    preferred_position: i64 = 0,
    position_priority: i64 = 0,
    attack_range: i64 = 0,
    ranged_dmg: i64 = 0,
    melee_skill: i64 = 40,
    ranged_skill: i64 = 40,
    defence: i64 = 10,
    critical: f64 = 1.5,
    parries: bool = false,
    melee_floor: f64 = 1.0,
    ranged_floor: f64 = 1.0,
    armour: i64 = 0,
    ward: i64 = 0,
    max_stamina: f64 = 100.0,
    fresh_speed: f64 = 0.0,
    tiring: f64 = 1.0,
    dodging: f64 = 1.0,
    run_pace: f64 = 1.5,
    load_stage: i64 = 0,
    regeneration: f64 = 0.0,
    wounded: i64 = 0,
    wounding: i64 = 0,
    playing_dead: bool = false,
    carrying: i64 = 0,
    carried_by: i64 = 0,
    gear_weight: f64 = 0.0,
    melee_seconds: f64 = 1.0,
    ranged_seconds: f64 = 1.0,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_number_reads_back_what_was_set_at_its_own_index() {
        let mut sheet = Sheet::default();
        for index in 0..NAMES.len() {
            sheet.set(index, (index + 2) as f64);
        }
        for (index, name) in NAMES.iter().enumerate() {
            let own = (index + 2) as f64;
            let read = sheet.get(index);
            assert!(read == own || read == 1.0, "{name} read {read}");
        }
        sheet.set(NAMES.iter().position(|n| *n == "carried_by").unwrap(), 9_007_199.0);
        assert_eq!(sheet.carried_by, 9_007_199);
        sheet.set(NAMES.iter().position(|n| *n == "critical").unwrap(), 2.25);
        assert_eq!(sheet.critical, 2.25);
        sheet.set(NAMES.iter().position(|n| *n == "parries").unwrap(), 1.0);
        assert!(sheet.parries);
    }

    #[test]
    fn names_are_unique_and_past_the_last_reads_nothing() {
        let mut seen = std::collections::HashSet::new();
        assert!(NAMES.iter().all(|name| seen.insert(*name)));
        let mut sheet = Sheet::default();
        sheet.set(NAMES.len(), 5.0);
        assert_eq!(sheet, Sheet::default());
        assert_eq!(sheet.get(NAMES.len()), 0.0);
    }

    #[test]
    fn a_blank_sheet_has_the_units_own_defaults() {
        let sheet = Sheet::default();
        assert_eq!((sheet.courage, sheet.discipline, sheet.initiative), (60, 30, 10));
        assert_eq!((sheet.turn_rate, sheet.backward_pace, sheet.critical), (450.0, 0.4, 1.5));
        assert_eq!((sheet.footprint_width, sheet.advance_direction), (1, 1));
    }
}
