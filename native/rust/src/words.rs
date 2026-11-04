//! A unit's words and lists (spec 39 step 8b5-ii; GDScript's SkirmishUnit): its faction and
//! damage type, its tags, tactics, weaknesses, resistances, immunities and what stops its
//! regeneration, its attributes and traits with their levels, and its weapons' parts, held
//! by the crate in the unit store by slot (`Columns::words`), so the snapshot carries them.
//! Each word is coded once for the field (`Lexicon`, in the order first seen; "" is 0), so
//! a crate pass compares codes, not strings. Written whole when the store admits a unit and
//! one field at a time after, by SkirmishUnit's own setters; nothing in the crate writes
//! them yet. Its numbers are 8b5-i's (`sheet`); its objects (walker, command, origin,
//! definition) a later part's.
//!
//! Each field is named as SkirmishUnit's property, by its shape: a word, a list of words,
//! words with levels, or parts ([damage, type, magical], UnitArms.parts).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// The field's words, each coded by the order it was first seen; "" is 0.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Lexicon {
    words: Vec<String>,
    codes: BTreeMap<String, u32>,
}

impl Default for Lexicon {
    fn default() -> Self {
        Lexicon { words: vec![String::new()], codes: BTreeMap::from([(String::new(), 0)]) }
    }
}

impl Lexicon {
    /// The code of `word`, given it if it is new.
    pub fn code(&mut self, word: &str) -> u32 {
        if let Some(&code) = self.codes.get(word) {
            return code;
        }
        let code = self.words.len() as u32;
        self.words.push(word.to_string());
        self.codes.insert(word.to_string(), code);
        code
    }

    /// The code of `word`, if it has been given one.
    pub fn find(&self, word: &str) -> Option<u32> {
        self.codes.get(word).copied()
    }

    /// The level `said` (a unit's traits or attributes) gives `word`, if it names it.
    pub fn level(&self, said: &[(u32, f64)], word: &str) -> Option<f64> {
        let code = self.find(word)?;
        said.iter().find(|&&(named, _)| named == code).map(|&(_, level)| level)
    }

    /// Every word, by its code.
    pub fn words(&self) -> &[String] {
        &self.words
    }

    /// The word coded `code` ("" for none given).
    pub fn word(&self, code: u32) -> &str {
        self.words.get(code as usize).map_or("", String::as_str)
    }
}

/// One of a weapon's parts: its damage, its damage type's code and whether it is magical.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Part {
    pub damage: f64,
    pub kind: u32,
    pub magical: bool,
}

/// The fields that are one word.
pub const WORDS: &[&str] = &["faction_id", "damage_type"];
/// The fields that are a list of words.
pub const LISTS: &[&str] =
    &["tags", "tactics", "weaknesses", "resistances", "immunities", "regeneration_stops"];
/// The fields that are words with levels, in the order given.
pub const LEVELS: &[&str] = &["attributes", "traits"];
/// The fields that are weapons' parts.
pub const PARTS: &[&str] = &["melee_parts", "ranged_parts"];

/// One unit's words and lists, as codes. A slot newly given starts at SkirmishUnit's own
/// defaults: no words, every list empty.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Words {
    pub faction_id: u32,
    pub damage_type: u32,
    pub tags: Vec<u32>,
    pub tactics: Vec<u32>,
    pub weaknesses: Vec<u32>,
    pub resistances: Vec<u32>,
    pub immunities: Vec<u32>,
    pub regeneration_stops: Vec<u32>,
    pub attributes: Vec<(u32, f64)>,
    pub traits: Vec<(u32, f64)>,
    pub melee_parts: Vec<Part>,
    pub ranged_parts: Vec<Part>,
}

impl Words {
    /// The word field named `name` (None: no such field).
    pub fn word(&mut self, name: &str) -> Option<&mut u32> {
        match name {
            "faction_id" => Some(&mut self.faction_id),
            "damage_type" => Some(&mut self.damage_type),
            _ => None,
        }
    }

    /// The list field named `name`.
    pub fn list(&mut self, name: &str) -> Option<&mut Vec<u32>> {
        match name {
            "tags" => Some(&mut self.tags),
            "tactics" => Some(&mut self.tactics),
            "weaknesses" => Some(&mut self.weaknesses),
            "resistances" => Some(&mut self.resistances),
            "immunities" => Some(&mut self.immunities),
            "regeneration_stops" => Some(&mut self.regeneration_stops),
            _ => None,
        }
    }

    /// The levels field named `name`.
    pub fn levels(&mut self, name: &str) -> Option<&mut Vec<(u32, f64)>> {
        match name {
            "attributes" => Some(&mut self.attributes),
            "traits" => Some(&mut self.traits),
            _ => None,
        }
    }

    /// The parts field named `name`.
    pub fn parts(&mut self, name: &str) -> Option<&mut Vec<Part>> {
        match name {
            "melee_parts" => Some(&mut self.melee_parts),
            "ranged_parts" => Some(&mut self.ranged_parts),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_are_coded_in_the_order_first_seen_and_read_back() {
        let mut lexicon = Lexicon::default();
        assert_eq!(lexicon.code(""), 0);
        assert_eq!(lexicon.code("player"), 1);
        assert_eq!(lexicon.code("fire"), 2);
        assert_eq!(lexicon.code("player"), 1);
        assert_eq!((lexicon.find("fire"), lexicon.find("wits")), (Some(2), None));
        assert_eq!((lexicon.word(2), lexicon.word(0), lexicon.word(9)), ("fire", "", ""));
    }

    #[test]
    fn every_named_field_is_found_by_its_shape_and_no_other() {
        let mut words = Words::default();
        for name in WORDS {
            assert!(words.word(name).is_some() && words.list(name).is_none(), "{name}");
        }
        for name in LISTS {
            assert!(words.list(name).is_some() && words.levels(name).is_none(), "{name}");
        }
        for name in LEVELS {
            assert!(words.levels(name).is_some() && words.parts(name).is_none(), "{name}");
        }
        for name in PARTS {
            assert!(words.parts(name).is_some() && words.word(name).is_none(), "{name}");
        }
        assert!(words.word("courage").is_none());
    }

    #[test]
    fn a_field_set_reads_back_what_was_set() {
        let mut words = Words::default();
        *words.word("damage_type").unwrap() = 3;
        words.list("immunities").unwrap().extend([4, 2]);
        words.levels("traits").unwrap().push((5, 1.5));
        words.parts("ranged_parts").unwrap().push(Part { damage: 2.5, kind: 3, magical: true });
        assert_eq!((words.damage_type, words.faction_id), (3, 0));
        assert_eq!(words.immunities, vec![4, 2]);
        assert_eq!(words.traits, vec![(5, 1.5)]);
        assert_eq!(words.ranged_parts[0], Part { damage: 2.5, kind: 3, magical: true });
        assert!(words.melee_parts.is_empty());
    }
}
