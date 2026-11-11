//! The crate's hash maps and sets (spec 41 step 4): std's, hashed by foldhash's fast hasher
//! rather than SipHash, and still seeded at random in each process. The random seed keeps the
//! guard std's maps gave: code that came to depend on a map's iteration order would show as
//! digests differing between runs, which a fixed-seed hasher (FxHash) would hide.
//! `clippy.toml` refuses std's own `HashMap` and `HashSet` everywhere but here.
#![allow(clippy::disallowed_types)]

pub use foldhash::{HashMapExt, HashSetExt};

/// std's `HashMap`, foldhash's random seed.
pub type HashMap<K, V> = std::collections::HashMap<K, V, foldhash::fast::RandomState>;
/// std's `HashSet`, foldhash's random seed.
pub type HashSet<T> = std::collections::HashSet<T, foldhash::fast::RandomState>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_maps_keyed_alike_iterate_in_different_orders_as_their_seeds_are_random() {
        let keyed = || (0..256i64).map(|k| (k, k)).collect::<HashMap<i64, i64>>();
        let (a, b) = (keyed(), keyed());
        assert_eq!(a, b, "the same entries");
        let order = |m: &HashMap<i64, i64>| m.keys().copied().collect::<Vec<_>>();
        assert_ne!(order(&a), order(&b), "a fixed seed would iterate them alike");
    }
}
