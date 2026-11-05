//! The digest's trace in the crate (spec 37, tools/digest_trace.gd until spec 39 step
//! 9d-iii): one tick of a run as a line of JSON, the run, the tick, its events and every
//! squad and unit by their field names, nested names zipped with the state's nested values
//! (DigestState's SQUAD_FIELDS and UNIT_FIELDS). Each squad's faction comes first, beside the
//! named fields and not hashed. Points are [x, y], floats in the shortest form that reads
//! back bit for bit, and infinities and NaN as Python's json reads them.

#[cfg(test)]
mod tests;

use super::Plain;

/// A field's name, or the names of a nested list of fields.
enum Name {
    One(&'static str),
    Many(&'static [Name]),
}

use Name::{Many, One};

/// A squad's fields, as `state::state` nests them.
const SQUAD: &[Name] = &[
    Many(&[One("id"), One("state"), One("order"), One("front_distance"), One("heading")]),
    Many(&[One("position"), One("morale"), One("engaged_with"), One("stall_ticks")]),
    Many(&[
        One("fight_since"),
        Many(&[One("flank_contacts"), One("stance"), One("pursuit"), One("withdraw")]),
        One("units"),
    ]),
];

/// A unit's fields.
const UNIT: &[Name] = &[
    Many(&[One("id"), One("hp"), One("position"), One("bearing"), One("attack_cooldown")]),
    Many(&[
        One("target_id"),
        One("rank"),
        One("column"),
        Many(&[One("loose"), One("fleeing"), One("chaser")]),
    ]),
];

/// The tick's line: `{"run", "tick", "events", "squads"}`, each squad `factions`' entry for it
/// then its fields by name, its units' fields by name under "units".
pub fn line(run: &str, tick: i64, events: &[Plain], state: &Plain, factions: &[String]) -> String {
    let squads: &[Plain] = match state {
        Plain::List(squads) => squads,
        _ => &[],
    };
    let named = squads.iter().zip(factions).map(|(squad, faction)| {
        let mut out = vec![("faction".to_string(), quoted(faction))];
        named_into(SQUAD, squad, &mut out, &|name, value| match name {
            "units" => units(value),
            _ => json(value),
        });
        object(&out)
    });
    let pairs = [
        ("run".to_string(), quoted(run)),
        ("tick".to_string(), tick.to_string()),
        ("events".to_string(), array(events.iter().map(json))),
        ("squads".to_string(), array(named)),
    ];
    object(&pairs)
}

/// A squad's units, each by its fields' names.
fn units(value: &Plain) -> String {
    let Plain::List(units) = value else { return json(value) };
    array(units.iter().map(|unit| {
        let mut out = Vec::new();
        named_into(UNIT, unit, &mut out, &|_, value| json(value));
        object(&out)
    }))
}

/// Each value under its name, nested names zipped with nested values, written by `write`.
fn named_into(
    names: &[Name],
    value: &Plain,
    out: &mut Vec<(String, String)>,
    write: &dyn Fn(&str, &Plain) -> String,
) {
    let values: &[Plain] = match value {
        Plain::List(values) => values,
        _ => &[],
    };
    for (name, value) in names.iter().zip(values) {
        match name {
            One(name) => out.push((name.to_string(), write(name, value))),
            Many(names) => named_into(names, value, out, write),
        }
    }
}

/// The value as JSON text.
fn json(value: &Plain) -> String {
    match value {
        Plain::Nil => "null".to_string(),
        Plain::Bool(b) => b.to_string(),
        Plain::Int(i) => i.to_string(),
        Plain::Float(f) => float(*f),
        Plain::Str(s) => quoted(s),
        Plain::Point(p) => format!("[{},{}]", float(p.x as f64), float(p.y as f64)),
        Plain::List(values) => array(values.iter().map(json)),
    }
}

/// A float in the shortest form that reads back bit for bit; infinities and NaN by name.
fn float(f: f64) -> String {
    match f {
        f if f.is_nan() => "NaN".to_string(),
        f64::INFINITY => "Infinity".to_string(),
        f64::NEG_INFINITY => "-Infinity".to_string(),
        f => format!("{f:?}"),
    }
}

fn quoted(s: &str) -> String {
    serde_json::to_string(s).expect("a string is always JSON")
}

fn array(values: impl Iterator<Item = String>) -> String {
    format!("[{}]", values.collect::<Vec<_>>().join(","))
}

fn object(pairs: &[(String, String)]) -> String {
    let pairs: Vec<String> = pairs.iter().map(|(k, v)| format!("{}:{v}", quoted(k))).collect();
    format!("{{{}}}", pairs.join(","))
}

/// Whether `tick` is traced: from `from` to `to`, a negative `to` meaning every tick on.
pub fn traced(tick: i64, from: i64, to: i64) -> bool {
    tick >= from && (to < 0 || tick <= to)
}
