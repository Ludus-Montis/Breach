//! The digest's events in the crate (spec 39 step 8c-xii): each tick's events as
//! DigestState.plain gives them. The crate's log is decoded as CrateEvents decodes it, into
//! the dictionaries FormationEvents made: "type", "tick", "squad" (or "unit", for a kind told
//! of a unit) and "faction", then its kind's fields. An order, a band or how a blow landed is
//! named; a faction, a staged squad's "then" or a damage type is its word in the lexicon; a
//! flank or "alone" is a bool; a heading is its float's bits; a "joined" of -1 is left out; a
//! list is its values; a "definition" is the UnitDef Resource the unit store names, which
//! DigestState.plain writes as ["obj", "Resource"] (a UnitDef has no id). On the field, the
//! builders' and waves' events (DomainProduction, FormationProduction) come first.

#[cfg(test)]
mod tests;

use super::Plain;
use crate::events::{KINDS, LISTS, UNIT_LED};
use crate::waves::Produced;
use crate::words::Lexicon;

/// SkirmishUnit.Order's names, by the crate's code.
pub const ORDERS: [&str; 3] = ["ADVANCE", "HOLD", "RETREAT"];
/// FormationMorale.Band's names, by the crate's code.
pub const BANDS: [&str; 4] = ["STEADY", "SHAKEN", "WAVERING", "ROUTING"];
/// BlowLanding.LANDED: how a blow landed, by the crate's code (`landing.rs`).
pub const LANDED: [&str; 5] = ["parried", "dodged", "grazed", "hit", "critical"];
/// The fields that are words in the lexicon (CrateEvents.WORDS).
const WORDS: &[&str] = &["then", "damage_type"];
/// The fields left out when -1, none (CrateEvents.UNSET).
const UNSET: &[&str] = &["joined"];
/// FormationField's waves, by their keys; the builders' and the waves' faction.
pub const LANES: [&str; 2] = ["A", "B"];
const PLAYER: &str = "player";
/// DomainProduction.RESERVE: the lane of a unit built into the reserve.
const RESERVE: &str = "";

/// The events in `flat` (as the log hands them over, in the order built), each as
/// DigestState.plain gives the dictionary CrateEvents decodes it into; words from `lexicon`.
pub fn logged(flat: &[i64], lexicon: &Lexicon) -> Vec<Plain> {
    let mut out = Vec::new();
    let mut at = 0;
    while at < flat.len() {
        let kind = flat[at] as usize;
        let (name, fields) = KINDS[kind];
        let led = if UNIT_LED.contains(&kind) { "unit" } else { "squad" };
        let mut entries = vec![
            (Plain::str("type"), Plain::str(name)),
            (Plain::str("tick"), Plain::Int(flat[at + 1])),
            (Plain::str(led), Plain::Int(flat[at + 2])),
            (Plain::str("faction"), Plain::str(lexicon.word(flat[at + 3] as u32))),
        ];
        at += 4;
        for &field in fields {
            let value = flat[at];
            if LISTS.contains(&field) {
                let list = &flat[at + 1..at + 1 + value as usize];
                entries.push((
                    Plain::str(field),
                    Plain::List(list.iter().map(|&v| Plain::Int(v)).collect()),
                ));
                at += value as usize;
            } else if !(UNSET.contains(&field) && value == -1) {
                entries.push((Plain::str(field), decoded(field, value, lexicon)));
            }
            at += 1;
        }
        out.push(Plain::dict(entries));
    }
    out
}

/// A field's value as CrateEvents decodes it.
fn decoded(field: &str, value: i64, lexicon: &Lexicon) -> Plain {
    if WORDS.contains(&field) {
        return Plain::str(lexicon.word(value as u32));
    }
    match field {
        "order" => Plain::str(ORDERS[value as usize]),
        "band" => Plain::str(BANDS[value as usize]),
        "blow" => Plain::str(LANDED[value as usize]),
        "flank" | "alone" => Plain::Bool(value != 0),
        "heading" => Plain::Float(f64::from_bits(value as u64)),
        "definition" => Plain::List(vec![Plain::str("obj"), Plain::str("Resource")]),
        _ => Plain::Int(value),
    }
}

/// What the field's builders and waves did, each as DigestState.plain gives the event
/// DomainProduction or FormationProduction makes of it.
pub fn produced(made: &[Produced]) -> Vec<Plain> {
    made.iter()
        .map(|p| {
            let (name, tick, field, value) = match *p {
                Produced::FromReserve { tick, lane } => {
                    ("from_reserve", tick, "lane", lane_of(Some(lane)))
                }
                Produced::Built { tick, lane } => ("built", tick, "lane", lane_of(Some(lane))),
                Produced::ToReserve { tick } => ("to_reserve", tick, "lane", lane_of(None)),
                Produced::WaveFull { tick, built } => {
                    ("wave_full", tick, "built", Plain::Int(built))
                }
                Produced::Departed { tick, units } => {
                    ("departed", tick, "units", Plain::Int(units))
                }
            };
            Plain::dict(vec![
                (Plain::str("type"), Plain::str(name)),
                (Plain::str("tick"), Plain::Int(tick)),
                (Plain::str("faction"), Plain::str(PLAYER)),
                (Plain::str(field), value),
            ])
        })
        .collect()
}

/// A lane's key, or the reserve's.
fn lane_of(lane: Option<usize>) -> Plain {
    Plain::str(lane.map_or(RESERVE, |lane| LANES[lane]))
}
