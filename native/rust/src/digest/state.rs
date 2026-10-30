//! DigestState.state read in the crate (spec 39 step 8c-xi): every listed squad's state and
//! its units', as the plain values tools/digest_state.gd gives, nested as its fields nest
//! (DigestState.SQUAD_FIELDS, UNIT_FIELDS). A squad's records (its flank contacts, stance,
//! pursuit and withdrawal) and a unit's loose, fleeing and chasing entries are the
//! dictionaries SquadRecords, OrderRecords and EntryKeys make of them, with exactly the keys
//! they hold; each value takes the type GDScript reads it as (an int column an int, a point a
//! point).

#[cfg(test)]
mod tests;

use super::Plain;
use crate::columns::Columns;
use crate::entries::{Key, FLEEING_KEYS, LOOSE_KEYS};
use crate::field::Field;
use crate::squads::Squads;

/// The state of the squads in `listed` (their slots, in the battle's order).
pub fn state(squads: &Squads, field: &Field, listed: &[usize]) -> Plain {
    Plain::List(listed.iter().map(|&s| squad(squads, field, s)).collect())
}

fn squad(squads: &Squads, field: &Field, s: usize) -> Plain {
    use Plain::{Float, Int, List, Point};
    let units = squads.members[s].iter().map(|&m| unit(squads, field, s, m as usize));
    let held = vec![flanks(squads, s), stance(squads, s), pursuit(squads, s), withdraw(squads, s)];
    List(vec![
        List(vec![
            Int(squads.id[s]),
            Int(squads.state[s] as i64),
            Int(squads.order[s] as i64),
            Float(squads.front_distance[s]),
            Float(squads.heading[s]),
        ]),
        List(vec![
            Point(squads.position[s]),
            Int(squads.morale[s] as i64),
            Int(squads.engaged_with[s]),
            Int(squads.stall_ticks[s] as i64),
        ]),
        List(vec![Int(squads.fight_since[s]), List(held), List(units.collect())]),
    ])
}

/// Edge -> {"foe", "since"}.
fn flanks(squads: &Squads, s: usize) -> Plain {
    let entries = squads.flanks[s].iter().map(|f| {
        let held = named(vec![("foe", Plain::Int(f.foe)), ("since", Plain::Int(f.since))]);
        (Plain::Int(f.edge as i64), held)
    });
    Plain::dict(entries.collect())
}

/// {"anchor", "heading", "foe"}, or {}.
fn stance(squads: &Squads, s: usize) -> Plain {
    let Some(held) = squads.stance[s] else { return named(vec![]) };
    named(vec![
        ("anchor", Plain::Point(held.anchor)),
        ("heading", Plain::Float(held.heading)),
        ("foe", Plain::Int(held.foe)),
    ])
}

/// {"foe", "post", "post_at", "home", "direction", "order", "returning"}, or {}.
fn pursuit(squads: &Squads, s: usize) -> Plain {
    let Some(p) = squads.pursuit[s] else { return named(vec![]) };
    named(vec![
        ("foe", Plain::Int(p.foe)),
        ("post", Plain::Float(p.post)),
        ("post_at", Plain::Point(p.post_at)),
        ("home", Plain::Float(p.home)),
        ("direction", Plain::Int(p.direction as i64)),
        ("order", Plain::Int(p.order as i64)),
        ("returning", Plain::Bool(p.returning)),
    ])
}

/// {"safe_ticks"}, or {}.
fn withdraw(squads: &Squads, s: usize) -> Plain {
    match squads.withdraw[s] {
        Some(ticks) => named(vec![("safe_ticks", Plain::Int(ticks))]),
        None => named(vec![]),
    }
}

/// The unit in store slot `m`, a member of the squad in slot `s`.
fn unit(squads: &Squads, field: &Field, s: usize, m: usize) -> Plain {
    use Plain::{Float, Int, List, Point};
    let c = &field.columns;
    let id = field.unit_at(m as u32).unwrap_or(0);
    let held =
        vec![loose(squads, c, s, m, id), fleeing(squads, c, s, m, id), chaser(squads, s, id)];
    List(vec![
        List(vec![
            Int(id),
            Int(c.hp[m] as i64),
            Point(c.position[m]),
            Float(c.bearing[m]),
            Int(c.attack_cooldown[m] as i64),
        ]),
        List(vec![Int(c.target_id[m]), Int(c.rank[m] as i64), Int(c.column[m] as i64), List(held)]),
    ])
}

/// EntryKeys.loose_entry: "unit", "goal", "next", "at" and the keys it holds ("touch" a
/// bool), or nil if the unit isn't loose.
fn loose(squads: &Squads, c: &Columns, s: usize, m: usize, id: i64) -> Plain {
    if !squads.loose[s].contains(&id) {
        return Plain::Nil;
    }
    let goal = match c.goal_slot[m] {
        sought if sought < 0 => Plain::Nil,
        sought => Plain::List(vec![Plain::Int(c.goal_foe[m]), Plain::Int(sought)]),
    };
    let mut out = vec![
        ("unit", Plain::object(id)),
        ("goal", goal),
        ("next", Plain::Point(c.next[m])),
        ("at", Plain::Point(c.loose_at[m])),
    ];
    held_keys(c, m, &LOOSE_KEYS, &mut out);
    named(out)
}

/// EntryKeys.fleeing_entry: "along", "offset" and the keys it holds, or nil if the unit
/// isn't fleeing.
fn fleeing(squads: &Squads, c: &Columns, s: usize, m: usize, id: i64) -> Plain {
    if !squads.fleeing[s].contains(&id) {
        return Plain::Nil;
    }
    let mut out =
        vec![("along", Plain::Float(c.along[m])), ("offset", Plain::Point(c.flee_offset[m]))];
    held_keys(c, m, &FLEEING_KEYS, &mut out);
    named(out)
}

/// The keys of `names` the entry holds, each by its value (nil for a `toward` or `held_by`
/// holding none).
fn held_keys(c: &Columns, m: usize, names: &[Key], out: &mut Vec<(&'static str, Plain)>) {
    let keys = c.keys[m];
    for &key in names.iter().filter(|key| keys & key.bit() != 0) {
        let none = key.valued() != 0 && keys & key.valued() == 0;
        let value = match key {
            _ if none => Plain::Nil,
            Key::FoeAt => Plain::Point(c.foe_at[m]),
            Key::Touch => Plain::Bool(c.touch[m] == 1),
            Key::Toward => Plain::Point(c.toward[m]),
            Key::Stuck => Plain::Float(c.stuck[m]),
            Key::Closest => Plain::Float(c.closest[m]),
            Key::Caught => Plain::Int(c.caught[m]),
            Key::HeldBy => Plain::Int(c.held_by[m]),
            Key::Fanned => Plain::Float(c.fanned[m]),
        };
        out.push((name_of(key), value));
    }
}

/// The key's UnitColumns name.
fn name_of(key: Key) -> &'static str {
    match key {
        Key::FoeAt => "foe_at",
        Key::Touch => "touch",
        Key::Toward => "toward",
        Key::Stuck => "stuck",
        Key::Closest => "closest",
        Key::Caught => "caught",
        Key::HeldBy => "held_by",
        Key::Fanned => "fanned",
    }
}

/// SquadRecords' chaser: {"unit", "foe", "from", "leash"} and "returning" once it turns back,
/// or nil if the unit isn't out chasing.
fn chaser(squads: &Squads, s: usize, id: i64) -> Plain {
    let Some(c) = squads.chasers[s].iter().find(|c| c.unit == id) else { return Plain::Nil };
    let mut out = vec![
        ("unit", Plain::object(id)),
        ("foe", Plain::Int(c.foe)),
        ("from", Plain::Point(c.from)),
        ("leash", Plain::Float(c.leash)),
    ];
    if c.returning {
        out.push(("returning", Plain::Bool(true)));
    }
    named(out)
}

/// A dictionary keyed by names.
fn named(entries: Vec<(&str, Plain)>) -> Plain {
    Plain::dict(entries.into_iter().map(|(key, value)| (Plain::str(key), value)).collect())
}
