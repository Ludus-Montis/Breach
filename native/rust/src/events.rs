//! The events as data (spec 39 step 8b5-v): what a crate pass reports, as FormationEvents
//! made them in GDScript, built here and handed back flat. Each is its kind's index in
//! `KINDS`, the tick, its squad's id and faction (coded in the unit store's lexicon), then
//! one number per field its kind names, in that order. GDScript asks for the kinds once
//! (`event_kinds`) and decodes each into the same dictionary it built before (CrateEvents):
//! "type", "tick", "squad", "faction", then the fields; a field it knows as a word (an
//! order, a band, a word coded in the lexicon) it names from its number, and a heading
//! (degrees) is its float's bits. A field named in `LISTS` is a list: how many, then each
//! (`Log::squad_listed`). Kinds are added as their passes move here.

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
    ("merged", &["into"]),
    ("reinforced", &["into"]),
    ("stepped_up", &["unit", "rank"]),
    ("faced", &["heading", "facing"]),
    ("regrouping", &[]),
    ("pursuit_ended", &[]),
    ("spat", &["unit", "target", "dmg", "damage_type", "blow"]),
    ("hit", &["target", "dmg", "flank", "blow", "target_hp"]),
    ("surrendered", &["unit"]),
    ("destroyed", &[]),
    ("died", &["unit"]),
    ("downed", &["unit"]),
    ("leader_fell", &["unit", "leadership"]),
    ("sent_to_take", &["unit"]),
    ("captured", &["unit", "by"]),
    ("sent_home", &["unit"]),
    ("finished", &["unit"]),
    ("died_of_wounds", &["unit"]),
    ("playing_dead", &["unit"]),
    ("came_to", &["unit", "alone", "joined"]),
    ("crushed", &["unit", "target", "dmg"]),
    ("fled_home", &["unit", "definition"]),
    ("scattered", &[]),
    ("rallied", &["unit", "into"]),
    ("reformed", &[]),
    ("routed", &[]),
    ("borne", &["unit", "body"]),
    ("split", &["from"]),
    ("rejoined", &["unit"]),
    ("swapped", &["unit", "passed"]),
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
pub const MERGED: usize = 16;
pub const REINFORCED: usize = 17;
pub const STEPPED_UP: usize = 18;
pub const FACED: usize = 19;
pub const REGROUPING: usize = 20;
pub const PURSUIT_ENDED: usize = 21;
pub const SPAT: usize = 22;
pub const HIT: usize = 23;
pub const SURRENDERED: usize = 24;
pub const DESTROYED: usize = 25;
pub const DIED: usize = 26;
pub const DOWNED: usize = 27;
pub const LEADER_FELL: usize = 28;
pub const SENT_TO_TAKE: usize = 29;
pub const CAPTURED: usize = 30;
pub const SENT_HOME: usize = 31;
pub const FINISHED: usize = 32;
pub const DIED_OF_WOUNDS: usize = 33;
pub const PLAYING_DEAD: usize = 34;
pub const CAME_TO: usize = 35;
pub const CRUSHED: usize = 36;
pub const FLED_HOME: usize = 37;
pub const SCATTERED: usize = 38;
pub const RALLIED: usize = 39;
pub const REFORMED: usize = 40;
pub const ROUTED: usize = 41;
pub const BORNE: usize = 42;
pub const SPLIT: usize = 43;
pub const REJOINED: usize = 44;
pub const SWAPPED: usize = 45;

/// The fields that are lists: how many, then each ("swapped"'s units passed).
pub const LISTS: &[&str] = &["passed"];

/// The kinds told of a unit, not a squad: their third number is the unit's id, and their
/// faction its own ("hit").
pub const UNIT_LED: &[usize] = &[HIT];

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

    /// An event of `kind` on `tick` for the squad in slot `s`, whose last field is a list
    /// (one of `LISTS`): the other fields' `values`, in order, then how many in `list` and
    /// each.
    pub fn squad_listed(
        &mut self,
        kind: usize,
        (tick, squads, s): (i64, &Squads, usize),
        values: &[i64],
        list: &[i64],
    ) {
        let fields = KINDS[kind].1;
        debug_assert_eq!(values.len() + 1, fields.len());
        debug_assert!(fields.last().is_some_and(|last| LISTS.contains(last)));
        self.flat.extend([kind as i64, tick, squads.id[s], i64::from(squads.faction[s])]);
        self.flat.extend_from_slice(values);
        self.flat.push(list.len() as i64);
        self.flat.extend_from_slice(list);
    }

    /// An event of `kind` (one of `UNIT_LED`) on `tick` for the unit `unit` of `faction` (a
    /// lexicon code), with its kind's fields' `values`, in order.
    pub fn unit(&mut self, kind: usize, tick: i64, unit: i64, faction: u32, values: &[i64]) {
        debug_assert!(UNIT_LED.contains(&kind));
        debug_assert_eq!(values.len(), KINDS[kind].1.len());
        self.flat.extend([kind as i64, tick, unit, i64::from(faction)]);
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
    fn a_unit_led_event_carries_its_units_id_and_faction_in_place_of_a_squads() {
        let mut log = Log::default();
        log.unit(HIT, 4, 101, 2, &[200, 7, 0, 3, 13]);
        assert_eq!(log.take(), vec![HIT as i64, 4, 101, 2, 200, 7, 0, 3, 13]);
    }

    #[test]
    fn a_listed_field_is_how_many_then_each() {
        let mut squads = Squads::default();
        squads.open(7);
        let mut log = Log::default();
        log.squad_listed(SWAPPED, (4, &squads, 0), &[101], &[102, 103]);
        assert_eq!(log.take(), vec![SWAPPED as i64, 4, 7, 0, 101, 2, 102, 103]);
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
