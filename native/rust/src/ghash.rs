//! Godot's `hash()` of an Array of ints and Strings, as GDScript computes it (Godot 4.7:
//! `Array::recursive_hash` - MurmurHash3 steps over each element's own hash, seeded with
//! the ARRAY type and finished with fmix32; an int's hash is `hash_one_uint64`, a String's
//! the djb2 `String::hash` over its code points, a bool's 0 or 1 itself). The battle's seeded draws (BattleRolls,
//! ScrumContest) are posmods of it. `BodyField.godot_hash` exposes it, so the suite proves
//! it against the engine's own `hash()` (tests/sim/skirmish/formation/test_body_field.gd).

const MURMUR3_SEED: u32 = 0x7F07C65;
/// `Variant::ARRAY`.
const ARRAY_TYPE: u32 = 28;

/// One element of a hashed Array.
#[derive(Clone, Copy)]
pub enum Key<'a> {
    Int(i64),
    Str(&'a str),
    Bool(bool),
}

#[inline]
fn murmur3_one_32(input: u32, seed: u32) -> u32 {
    let mut k = input.wrapping_mul(0xcc9e2d51);
    k = k.rotate_left(15);
    k = k.wrapping_mul(0x1b873593);
    let mut h = seed ^ k;
    h = h.rotate_left(13);
    h.wrapping_mul(5).wrapping_add(0xe6546b64)
}

#[inline]
fn fmix32(mut h: u32) -> u32 {
    h ^= h >> 16;
    h = h.wrapping_mul(0x85ebca6b);
    h ^= h >> 13;
    h = h.wrapping_mul(0xc2b2ae35);
    h ^= h >> 16;
    h
}

#[inline]
fn one_uint64(value: u64) -> u32 {
    let mut v = value;
    v = (!v).wrapping_add(v << 18);
    v ^= v >> 31;
    v = v.wrapping_mul(21);
    v ^= v >> 11;
    v = v.wrapping_add(v << 6);
    v ^= v >> 22;
    v as u32
}

#[inline]
fn string_hash(text: &str) -> u32 {
    let mut h: u32 = 5381;
    for c in text.chars() {
        h = (h << 5).wrapping_add(h).wrapping_add(c as u32);
    }
    h
}

/// `hash([keys...])`, as GDScript's `hash()` gives it (an int in [0, 2^32)).
pub fn hash(keys: &[Key]) -> u32 {
    let mut h = murmur3_one_32(ARRAY_TYPE, MURMUR3_SEED);
    for key in keys {
        let one = match *key {
            Key::Int(v) => one_uint64(v as u64),
            Key::Str(s) => string_hash(s),
            Key::Bool(b) => b as u32,
        };
        h = murmur3_one_32(one, h);
    }
    fmix32(h)
}

/// `BattleRolls.uniform(seed, keys)`: `float(posmod(hash([seed] + keys), 1 << 24)) / (1 << 24)`.
pub fn uniform(seed: i64, keys: &[Key]) -> f64 {
    let mut all = Vec::with_capacity(keys.len() + 1);
    all.push(Key::Int(seed));
    all.extend_from_slice(keys);
    (hash(&all) as i64 % (1 << 24)) as f64 / (1 << 24) as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_the_engine() {
        // Printed by Godot 4.7.2's own hash().
        assert_eq!(hash(&[Key::Int(1), Key::Int(2), Key::Int(3)]), 3860078832);
        let slot = [Key::Int(7), Key::Int(100), Key::Int(5), Key::Str("slot")];
        assert_eq!(hash(&slot), 906619725);
        assert_eq!(hash(&[Key::Int(-5), Key::Int(3)]), 1782146209);
        let big = [Key::Int(123456789012345), Key::Int(3), Key::Int(1)];
        assert_eq!(hash(&big), 2113128541);
        assert_eq!(hash(&[Key::Bool(true)]), 2714831339);
        assert_eq!(hash(&[Key::Bool(false)]), 1771887473);
        let blow = [Key::Int(7), Key::Int(3), Key::Int(11), Key::Str("blow"), Key::Bool(true)];
        assert_eq!(hash(&blow), 4117355511);
        let damage = [Key::Int(7), Key::Int(3), Key::Int(11), Key::Str("damage"), Key::Bool(false)];
        assert_eq!(hash(&damage), 1482015660);
    }
}
