//! The events as data (spec 39 step 8b5-v): what a crate pass reports, as FormationEvents
//! made them in GDScript, built here and handed back flat. Each is its kind's index in
//! `KINDS`, the tick, its squad's id and faction (coded in the unit store's lexicon), then
//! one number per field its kind names, in that order. GDScript asks for the kinds once
//! (`event_kinds`) and decodes each into the same dictionary it built before (CrateEvents):
//! "type", "tick", "squad", "faction", then the fields; a field it knows as a word (an
//! order, a band, a word coded in the lexicon) it names from its number, and a heading
//! (degrees) is its float's bits. Kinds are added as their passes move here.

use crate::squads::Squads;

/// Each kind's type and its fields, in order.
pub const KINDS: &[(&str, &[&str])] = &[
    ("order_applied", &["order"]),
    ("disengaged", &[]),
    ("withdrawing", &[]),
    ("pursuing", &["of"]),
    ("morale_band", &["band", "morale"]),
    ("engaged", &["with"]),
    ("flanked", &["by", "edge"]),
    ("detoured", &["cells"]),
    ("staged", &[]),
    ("signalled", &[]),
    ("gave_up", &["then"]),
    ("arrived", &[]),
    ("returned", &[]),
    ("turning", &["facing", "heading"]),
    ("turned", &["facing", "heading"]),
    ("blocked", &[]),
];
pub const ORDER_APPLIED: usize = 0;
pub const DISENGAGED: usize = 1;
pub const WITHDRAWING: usize = 2;
pub const PURSUING: usize = 3;
pub const MORALE_BAND: usize = 4;
pub const ENGAGED: usize = 5;
pub const FLANKED: usize = 6;
pub const DETOURED: usize = 7;
pub const STAGED: usize = 8;
pub const SIGNALLED: usize = 9;
pub const GAVE_UP: usize = 10;
pub const ARRIVED: usize = 11;
pub const RETURNED: usize = 12;
pub const TURNING: usize = 13;
pub const TURNED: usize = 14;
pub const BLOCKED: usize = 15;

/// The events a pass has built and GDScript hasn't yet taken, flat, in the order built.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Log {
    flat: Vec<i64>,
}

impl Log {
    /// An event of `kind` on `tick` for the squad in slot `s`, with its kind's fields'
    /// `values`, in order.
    pub fn squad(&mut self, kind: usize, tick: i64, squads: &Squads, s: usize, values: &[i64]) {
        debug_assert_eq!(values.len(), KINDS[kind].1.len());
        self.flat.extend([kind as i64, tick, squads.id[s], i64::from(squads.faction[s])]);
        self.flat.extend_from_slice(values);
    }

    /// The events built since last taken, flat; the log is left empty.
    pub fn take(&mut self) -> Vec<i64> {
        std::mem::take(&mut self.flat)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn events_are_taken_flat_in_the_order_built_and_once() {
        let mut squads = Squads::default();
        squads.open(7);
        squads.open(9);
        squads.faction[1] = 3;
        let mut log = Log::default();
        log.squad(PURSUING, 4, &squads, 1, &[7]);
        log.squad(DISENGAGED, 4, &squads, 0, &[]);
        assert_eq!(log.take(), vec![3, 4, 9, 3, 7, 1, 4, 7, 0]);
        assert!(log.take().is_empty());
    }

    #[test]
    fn each_kind_is_named_once() {
        for (n, kind) in KINDS.iter().enumerate() {
            assert!(KINDS[n + 1..].iter().all(|other| other.0 != kind.0));
        }
        assert_eq!(KINDS[MORALE_BAND], ("morale_band", &["band", "morale"][..]));
        assert_eq!(KINDS[FLANKED], ("flanked", &["by", "edge"][..]));
    }
}
