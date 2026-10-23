//! ScrumContest (spec 39 step 4a): who gets a contested cell in the scrum (Decision 88), and
//! the seeded draws that settle any tie left once what things are and where they stand are
//! equal (Decision 97). A seeker's key is [arrival time, roll + initiative, initiative,
//! speed, its draw], compared item by item: the earliest arrival first, then the higher
//! roll plus initiative, the higher initiative, the faster unit, and last a seeded draw
//! wide enough that it never ties in practice. The roll is the unit's seeded variance,
//! drawn once per contest from the fight's seed, the tick and the unit, so a replay
//! repeats. Every draw is a posmod of Godot's own `hash()` (`ghash`), bit for bit the
//! GDScript's.

use std::cmp::Ordering;

use crate::ghash::{self, Key};

/// A unit's draw: seeded by the fight and the unit, 62 bits wide, so two units' draws
/// (almost never) tie; the unit's id salts the draw but never orders it (Decision 97).
pub fn draw(seed: i64, id: i64) -> i64 {
    wide(&[Key::Int(seed), Key::Int(id)])
}

/// A squad's draw, as `draw` is a unit's: for ties between whole formations.
pub fn squad_draw(seed: i64, id: i64) -> i64 {
    wide(&[Key::Int(seed), Key::Str("squad"), Key::Int(id)])
}

/// `posmod(hash(keys + [0]), 1 << 31) * (1 << 31) + posmod(hash(keys + [1]), 1 << 31)`.
fn wide(keys: &[Key]) -> i64 {
    let half = |end: i64| {
        let mut all = keys.to_vec();
        all.push(Key::Int(end));
        ghash::hash(&all) as i64 % (1 << 31)
    };
    half(0) * (1 << 31) + half(1)
}

/// The unit's roll in a contest this tick: `posmod(hash([seed, tick, id]), die + 1)`, or 0
/// when the die is 0 (stats alone decide).
pub fn roll(seed: i64, tick: i64, id: i64, die: i64) -> i64 {
    if die > 0 {
        ghash::hash(&[Key::Int(seed), Key::Int(tick), Key::Int(id)]) as i64 % (die + 1)
    } else {
        0
    }
}

/// A seeker's key in a contest for a cell (ScrumContest.key).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Contest {
    /// Seconds till it would arrive.
    pub arrival: f64,
    /// Its roll plus its initiative.
    pub rolled: i64,
    pub initiative: i64,
    /// Its pace this tick.
    pub speed: f64,
    pub draw: i64,
}

impl Contest {
    /// The unit's key for a cell it would reach in `arrival` seconds at its `speed`.
    pub fn of(arrival: f64, speed: f64, initiative: i64, rolled: i64, draw: i64) -> Contest {
        Contest { arrival, rolled: rolled + initiative, initiative, speed, draw }
    }
}

/// ScrumContest.before as an order: the earlier arrival, then the higher roll plus
/// initiative, the higher initiative, the faster, the lower draw (which never ties, so the
/// order is total).
pub fn order(a: &Contest, b: &Contest) -> Ordering {
    let floats = |x: f64, y: f64| x.partial_cmp(&y).unwrap_or(Ordering::Equal);
    floats(a.arrival, b.arrival)
        .then((-a.rolled).cmp(&-b.rolled))
        .then((-a.initiative).cmp(&-b.initiative))
        .then(floats(-a.speed, -b.speed))
        .then(a.draw.cmp(&b.draw))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draws_are_godot_s() {
        // Printed by Godot 4.7.2 from the GDScript ScrumContest before it became a view.
        assert_eq!(draw(1, 7), 4_025_626_903_716_927_711);
        assert_eq!(draw(4242, 7), 2_733_695_541_191_639_934);
        assert_eq!(draw(-3, 1_000_123), 4_589_792_188_597_651_037);
        assert_eq!(squad_draw(1, 7), 561_716_261_158_975_843);
        assert_eq!(roll(9, 40, 7, 6), 2);
    }

    #[test]
    fn a_draw_is_62_bits_and_a_squad_s_is_salted_apart_from_a_unit_s() {
        for id in 1..200 {
            let d = draw(5, id);
            assert!((0..1i64 << 62).contains(&d));
            assert_ne!(d, squad_draw(5, id));
        }
        assert_ne!(draw(5, 1), draw(6, 1)); // the battle's seed draws anew
    }

    #[test]
    fn a_roll_stays_on_the_die_and_no_die_rolls_nothing() {
        for id in 1..500 {
            assert!((0..=6).contains(&roll(3, 12, id, 6)));
            assert_eq!(roll(3, 12, id, 0), 0);
        }
        let rolls: Vec<i64> = (1..500).map(|id| roll(3, 12, id, 6)).collect();
        assert!((0..=6).all(|face| rolls.contains(&face))); // every face comes up
    }

    #[test]
    fn the_earliest_arrival_wins_then_roll_and_initiative_then_initiative_then_pace() {
        let base = Contest::of(1.0, 2.0, 1, 2, 50);
        let later = Contest::of(1.5, 9.0, 9, 9, 0);
        assert_eq!(order(&base, &later), Ordering::Less);
        let rolled_higher = Contest::of(1.0, 2.0, 0, 4, 99); // 4 + 0 beats 2 + 1
        assert_eq!(order(&rolled_higher, &base), Ordering::Less);
        let keener = Contest::of(1.0, 2.0, 3, 0, 99); // 0 + 3 ties 2 + 1; initiative 3 wins
        assert_eq!(order(&keener, &base), Ordering::Less);
        let faster = Contest::of(1.0, 2.5, 1, 2, 99);
        assert_eq!(order(&faster, &base), Ordering::Less);
        let drawn = Contest::of(1.0, 2.0, 1, 2, 49);
        assert_eq!(order(&drawn, &base), Ordering::Less);
        assert_eq!(order(&base, &base), Ordering::Equal);
    }

    #[test]
    fn a_contest_sorts_the_same_whatever_the_list_order() {
        let mut keys: Vec<Contest> =
            (0..40).map(|k| Contest::of((k % 4) as f64, 1.0, k % 3, k % 2, draw(2, k))).collect();
        let mut reversed: Vec<Contest> = keys.iter().rev().copied().collect();
        keys.sort_by(order);
        reversed.sort_by(order);
        assert_eq!(keys, reversed);
    }
}
