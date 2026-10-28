//! The crate's whole state written and read back (spec 39 step 1): the tick, the battle's
//! seed, the field (the units: the store's slots and columns, the bodies) and the squad
//! store, in bincode's compact binary through serde. Every f64 keeps its bits, infinities
//! included, and the same state always writes the same bytes: the field's maps are written
//! in key order. The digest writes them (`formation_digest.gd snapshot=`), and step 8's
//! snapshot tests read them back.

use serde::{Deserialize, Serialize};

use crate::field::Field;
use crate::squads::Squads;

/// The state as written: the tick it was taken after, and the battle's seed.
#[derive(Serialize)]
struct Out<'a> {
    tick: i64,
    seed: i64,
    field: &'a Field,
    squads: &'a Squads,
}

/// The state as read back.
#[derive(Deserialize)]
pub struct State {
    pub tick: i64,
    pub seed: i64,
    pub field: Field,
    pub squads: Squads,
}

/// The state's bytes.
pub fn write(tick: i64, seed: i64, field: &Field, squads: &Squads) -> Vec<u8> {
    bincode::serialize(&Out { tick, seed, field, squads }).expect("the state serialises")
}

/// The state written as `bytes`, or why it can't be read.
pub fn read(bytes: &[u8]) -> Result<State, String> {
    bincode::deserialize(bytes).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build;
    use crate::field_parting::part_field;
    use crate::maths::V2;
    use crate::parting::Tuning;

    /// UnitBodies.part_way's stand-in: a fixed way for each pair of draws.
    fn way(a: i64, b: i64) -> V2 {
        let turn = ((a * 31 + b * 17) % 360) as f32;
        V2 { x: turn.to_radians().cos(), y: turn.to_radians().sin() }
    }

    #[test]
    fn a_state_read_back_writes_the_same_bytes() {
        let (mut field, mut squads) = build::crowd(12, 3);
        field.columns.closest[0] = f64::INFINITY;
        squads.looked_at[1] = f64::NEG_INFINITY;
        squads.heading[0] = 0.1 + 0.2; // not a round number
        let bend = vec![V2::new(0.0, 0.0), V2::new(128.0, 0.0), V2::new(128.0, 64.0)];
        squads.route[1] = Some(crate::route::Route::new(bend, 2.0));
        squads.taking[0] = vec![(7, 2), (5, 1)];
        field.columns.sheet[2].critical = 0.1 + 0.2;
        field.columns.sheet[2].carried_by = 11;
        let fire = field.lexicon.code("fire");
        field.columns.words[2].immunities = vec![fire, field.lexicon.code("cold")];
        field.columns.words[2].traits = vec![(fire, 0.5)];
        let bytes = write(40, 3, &field, &squads);
        let state = read(&bytes).unwrap();
        assert_eq!((state.tick, state.seed), (40, 3));
        assert_eq!(state.squads, squads);
        assert_eq!(state.field.columns.sheet, field.columns.sheet);
        assert_eq!(state.field.columns.words, field.columns.words);
        assert_eq!(state.field.lexicon, field.lexicon);
        assert_eq!(write(state.tick, state.seed, &state.field, &state.squads), bytes);
    }

    #[test]
    fn a_pass_stepped_from_a_state_read_back_does_what_it_does_from_the_original() {
        let (mut field, squads) = build::crowd(12, 3);
        let mut back = read(&write(40, 3, &field, &squads)).unwrap();
        let tuning = Tuning { passes: 4, resist: 0.5, brush: 0.1 };
        let first = part_field(&mut field, &tuning, false, &mut way);
        let second = part_field(&mut back.field, &tuning, false, &mut way);
        assert!(!first.moved.is_empty(), "the crowd parts");
        assert_eq!(first.moved, second.moved);
        assert_eq!(first.points, second.points);
        assert_eq!(first.nexts, second.nexts);
        assert_eq!(write(41, 3, &field, &squads), write(41, 3, &back.field, &back.squads));
    }

    #[test]
    fn bytes_that_are_not_a_state_are_refused() {
        assert!(read(&[1, 2, 3]).is_err());
    }
}
