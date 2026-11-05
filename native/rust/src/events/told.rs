//! The log's events decoded as Godot reads them (CrateEvents.taken, spec 39 step 9d-i): each
//! its kind, tick, squad (or, for a kind told of a unit, the unit) and faction, then the
//! fields its kind names, in order. A field naming an order, a band or how a blow landed is
//! decoded to its name, a faction, a staged squad's "then" or a damage type from the lexicon,
//! a flank or "alone" to a bool, a heading from its float's bits, a "definition" to the
//! definition's id its code stands for; a "joined" of -1 (none) is left out, and a list
//! ("passed") is its numbers.

#[cfg(test)]
mod tests;

use super::{KINDS, LISTS, UNIT_LED};

/// The fields decoded from the lexicon.
const WORDS: [&str; 2] = ["then", "damage_type"];
/// The fields left out when -1 (none).
const UNSET: [&str; 1] = ["joined"];
/// SkirmishUnit.Order's names, by code.
const ORDERS: [&str; 3] = ["ADVANCE", "HOLD", "RETREAT"];
/// FormationMorale.Band's names, by code.
const BANDS: [&str; 4] = ["STEADY", "SHAKEN", "WAVERING", "ROUTING"];
/// How a blow landed, by the crate's code (BlowLanding.LANDED).
const LANDED: [&str; 5] = ["parried", "dodged", "grazed", "hit", "critical"];

/// A field's value as decoded.
#[derive(Clone, Debug, PartialEq)]
pub enum Said {
    Int(i64),
    Bool(bool),
    Real(f64),
    Word(String),
    List(Vec<i64>),
}

/// An event decoded: its kind, tick, whether it is told of a unit, the squad's (or the
/// unit's) id, its faction's name and its fields, in its kind's order.
#[derive(Clone, Debug, PartialEq)]
pub struct Told {
    pub kind: &'static str,
    pub tick: i64,
    pub of_unit: bool,
    pub who: i64,
    pub faction: String,
    pub fields: Vec<(&'static str, Said)>,
}

/// The events in the flat log `flat`, in the order built; `words` the lexicon's words by
/// their codes, `definitions` the definitions' ids by their codes.
pub fn decoded(flat: &[i64], words: &[String], definitions: &[&str]) -> Vec<Told> {
    let word = |code: i64| words.get(code as usize).cloned().unwrap_or_default();
    let mut out = Vec::new();
    let mut at = 0;
    while at + 4 <= flat.len() {
        let kind = flat[at] as usize;
        let Some(&(name, names)) = KINDS.get(kind) else { break };
        let mut told = Told {
            kind: name,
            tick: flat[at + 1],
            of_unit: UNIT_LED.contains(&kind),
            who: flat[at + 2],
            faction: word(flat[at + 3]),
            fields: Vec::with_capacity(names.len()),
        };
        at += 4;
        for &field in names {
            let Some(&value) = flat.get(at) else { break };
            if LISTS.contains(&field) {
                let end = (at + 1 + value.max(0) as usize).min(flat.len());
                told.fields.push((field, Said::List(flat[at + 1..end].to_vec())));
                at = end;
                continue;
            }
            at += 1;
            if UNSET.contains(&field) && value == -1 {
                continue;
            }
            told.fields.push((field, said(field, value, &word, definitions)));
        }
        out.push(told);
    }
    out
}

/// The field `name`'s `value` decoded.
fn said(name: &str, value: i64, word: &dyn Fn(i64) -> String, definitions: &[&str]) -> Said {
    let named = |names: &[&str]| Said::Word(names.get(value as usize).unwrap_or(&"").to_string());
    match name {
        _ if WORDS.contains(&name) => Said::Word(word(value)),
        "definition" => named(definitions),
        "order" => named(&ORDERS),
        "band" => named(&BANDS),
        "blow" => named(&LANDED),
        "flank" | "alone" => Said::Bool(value != 0),
        "heading" => Said::Real(f64::from_bits(value as u64)),
        _ => Said::Int(value),
    }
}
