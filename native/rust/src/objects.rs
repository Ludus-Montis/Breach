//! A unit's objects (spec 39 step 8b5-iii; GDScript's SkirmishUnit), coded for the crate and
//! held in the unit store by slot (`Columns::objects`), so the snapshot carries them: how it
//! crosses terrain (its walker's fields, TerrainWalker.fields: height, swimmer, climber,
//! swims, climbs; none for a unit made without one), the command it follows and the one it
//! started under (their slots in the squad store's commands, step 8b4-iii; -1 for none), and
//! the definition it was made from (its code in the unit store's own list, the order first
//! seen; -1 for none). A definition is a Resource and stays GDScript's: the code names it
//! there (UnitStore.definition_of). Written whole when the store admits a unit and one at a
//! time after, by SkirmishUnit's own setters; nothing in the crate writes them yet.

use serde::{Deserialize, Serialize};

/// The fields that are a code: a command's slot, or a definition's code.
pub const CODES: &[&str] = &["command", "origin", "definition"];

/// One unit's objects, as codes. A slot newly given starts at SkirmishUnit's own defaults:
/// no walker, command, origin or definition.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Objects {
    pub walker: Option<[f64; 5]>,
    pub command: i64,
    pub origin: i64,
    pub definition: i64,
}

impl Default for Objects {
    fn default() -> Self {
        Objects { walker: None, command: -1, origin: -1, definition: -1 }
    }
}

impl Objects {
    /// The code field named `name` (None: no such field).
    pub fn code(&mut self, name: &str) -> Option<&mut i64> {
        match name {
            "command" => Some(&mut self.command),
            "origin" => Some(&mut self.origin),
            "definition" => Some(&mut self.definition),
            _ => None,
        }
    }

    /// Sets its walker from its fields, [height, swimmer, climber, swims, climbs]; none for
    /// any other count.
    pub fn put_walker(&mut self, fields: &[f64]) {
        self.walker = <[f64; 5]>::try_from(fields).ok();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_blank_unit_has_no_objects() {
        let objects = Objects::default();
        assert_eq!(objects.walker, None);
        assert_eq!((objects.command, objects.origin, objects.definition), (-1, -1, -1));
    }

    #[test]
    fn every_named_code_is_found_and_reads_back_what_was_set() {
        let mut objects = Objects::default();
        for (code, name) in CODES.iter().enumerate() {
            *objects.code(name).expect(name) = code as i64 + 4;
        }
        assert_eq!((objects.command, objects.origin, objects.definition), (4, 5, 6));
        assert!(objects.code("walker").is_none() && objects.code("faction_id").is_none());
    }

    #[test]
    fn a_walker_set_reads_back_its_fields_and_none_is_none() {
        let mut objects = Objects::default();
        objects.put_walker(&[1.5, 2.0, 0.0, 1.0, 0.0]);
        assert_eq!(objects.walker, Some([1.5, 2.0, 0.0, 1.0, 0.0]));
        objects.put_walker(&[]);
        assert_eq!(objects.walker, None);
    }
}
