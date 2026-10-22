//! The unit store's columns (spec 35, Decision 142; GDScript's UnitColumns), by slot: where
//! each unit stands (its position, where it stands loose, its place off its route), its
//! flag bits (LOOSE, FLEEING), and every other key of its loose and fleeing entries (step
//! 2: its next step, the foe and slot it seeks, that foe's place, whether it touches a
//! foe, where it walks toward, how long it has been stuck and its closest yet; how far
//! along its route it has fled, how long a friend has held it and which, how far it has
//! fanned out), with the bits saying which keys an entry holds (`keys`), and the way it
//! faces (`bearing`, step 3: degrees clockwise from north), and its place in its squad's
//! frame (`rank` and `column`, step 4a), and its hp, the ticks till its next blow and the
//! hp it may still regenerate before it rests (`hp`, `attack_cooldown`,
//! `regeneration_left`, step 5b), and its stamina, its pace this tick, whether it spent
//! stamina this tick, seconds since it last did, seconds into its run, whether it ran last
//! tick and its condition (`stamina`, `speed`, `exerted`, `breather`, `run_build`,
//! `was_running`, `condition`, step 6c), and the downed's and recovery's: the part of a point
//! regained, the seconds regeneration is stopped, the seconds left downed, the part of a
//! point drained, and the wounds the last blow would add (`regeneration_carry`,
//! `regeneration_halt`, `wake_left`, `drain_carry`, `last_wounding`, step 7b), and the rest:
//! the unit it fights, how far along its route it stands, the last place it could stand,
//! whether it is on a climb face or in deep water, and the climb still above it (`target_id`,
//! `distance`, `foothold`, `on_face`, `climb_left`, step 7c). The field holds them so its passes read them where they live. A slot newly given starts blank, each
//! column at the value a missing key reads as. Columns are found by their UnitColumns name.

use crate::maths::V2;

#[derive(Default)]
pub struct Columns {
    pub position: Vec<V2>,
    pub loose_at: Vec<V2>,
    pub flee_offset: Vec<V2>,
    pub next: Vec<V2>,
    pub foe_at: Vec<V2>,
    pub toward: Vec<V2>,
    pub foothold: Vec<V2>,
    pub flags: Vec<i32>,
    pub touch: Vec<i32>,
    pub keys: Vec<i32>,
    pub rank: Vec<i32>,
    pub column: Vec<i32>,
    pub hp: Vec<i32>,
    pub attack_cooldown: Vec<i32>,
    pub exerted: Vec<i32>,
    pub was_running: Vec<i32>,
    pub last_wounding: Vec<i32>,
    pub on_face: Vec<i32>,
    pub goal_foe: Vec<i64>,
    pub goal_slot: Vec<i64>,
    pub caught: Vec<i64>,
    pub held_by: Vec<i64>,
    pub target_id: Vec<i64>,
    pub stuck: Vec<f64>,
    pub closest: Vec<f64>,
    pub along: Vec<f64>,
    pub fanned: Vec<f64>,
    pub bearing: Vec<f64>,
    pub regeneration_left: Vec<f64>,
    pub stamina: Vec<f64>,
    pub speed: Vec<f64>,
    pub breather: Vec<f64>,
    pub run_build: Vec<f64>,
    pub condition: Vec<f64>,
    pub regeneration_carry: Vec<f64>,
    pub regeneration_halt: Vec<f64>,
    pub wake_left: Vec<f64>,
    pub drain_carry: Vec<f64>,
    pub distance: Vec<f64>,
    pub climb_left: Vec<f64>,
}

/// No goal (`goal_slot`).
const NO_GOAL: i64 = -1;
/// A unit not yet turned faces east (`bearing`).
const EAST: f64 = 90.0;
/// A unit not yet given its own starts fresh (`stamina`) and as things stand (`condition`).
const FRESH: f64 = 100.0;
const AS_THINGS_STAND: f64 = 1.0;
/// A unit not yet downed hasn't reckoned how long it lies (`wake_left`).
const NOT_RECKONED: f64 = -1.0;

impl Columns {
    /// A point column by its name.
    pub fn points(&mut self, name: &str) -> Option<&mut Vec<V2>> {
        match name {
            "position" => Some(&mut self.position),
            "at" => Some(&mut self.loose_at),
            "offset" => Some(&mut self.flee_offset),
            "next" => Some(&mut self.next),
            "foe_at" => Some(&mut self.foe_at),
            "toward" => Some(&mut self.toward),
            "foothold" => Some(&mut self.foothold),
            _ => None,
        }
    }

    /// A 32-bit int column by its name.
    pub fn ints(&mut self, name: &str) -> Option<&mut Vec<i32>> {
        match name {
            "flags" => Some(&mut self.flags),
            "touch" => Some(&mut self.touch),
            "keys" => Some(&mut self.keys),
            "rank" => Some(&mut self.rank),
            "column" => Some(&mut self.column),
            "hp" => Some(&mut self.hp),
            "attack_cooldown" => Some(&mut self.attack_cooldown),
            "exerted" => Some(&mut self.exerted),
            "was_running" => Some(&mut self.was_running),
            "last_wounding" => Some(&mut self.last_wounding),
            "on_face" => Some(&mut self.on_face),
            _ => None,
        }
    }

    /// A 64-bit int column by its name.
    pub fn longs(&mut self, name: &str) -> Option<&mut Vec<i64>> {
        match name {
            "goal_foe" => Some(&mut self.goal_foe),
            "goal_slot" => Some(&mut self.goal_slot),
            "caught" => Some(&mut self.caught),
            "held_by" => Some(&mut self.held_by),
            "target_id" => Some(&mut self.target_id),
            _ => None,
        }
    }

    /// A float column by its name.
    pub fn reals(&mut self, name: &str) -> Option<&mut Vec<f64>> {
        match name {
            "stuck" => Some(&mut self.stuck),
            "closest" => Some(&mut self.closest),
            "along" => Some(&mut self.along),
            "fanned" => Some(&mut self.fanned),
            "bearing" => Some(&mut self.bearing),
            "regeneration_left" => Some(&mut self.regeneration_left),
            "stamina" => Some(&mut self.stamina),
            "speed" => Some(&mut self.speed),
            "breather" => Some(&mut self.breather),
            "run_build" => Some(&mut self.run_build),
            "condition" => Some(&mut self.condition),
            "regeneration_carry" => Some(&mut self.regeneration_carry),
            "regeneration_halt" => Some(&mut self.regeneration_halt),
            "wake_left" => Some(&mut self.wake_left),
            "drain_carry" => Some(&mut self.drain_carry),
            "distance" => Some(&mut self.distance),
            "climb_left" => Some(&mut self.climb_left),
            _ => None,
        }
    }

    /// One more slot, blank.
    pub fn grow(&mut self) {
        for column in [
            &mut self.position,
            &mut self.loose_at,
            &mut self.flee_offset,
            &mut self.next,
            &mut self.foe_at,
            &mut self.toward,
            &mut self.foothold,
        ] {
            column.push(V2::ZERO);
        }
        for column in [
            &mut self.flags,
            &mut self.touch,
            &mut self.keys,
            &mut self.rank,
            &mut self.column,
            &mut self.hp,
            &mut self.attack_cooldown,
            &mut self.exerted,
            &mut self.was_running,
            &mut self.last_wounding,
            &mut self.on_face,
        ] {
            column.push(0);
        }
        for column in [&mut self.goal_foe, &mut self.caught, &mut self.held_by, &mut self.target_id]
        {
            column.push(0);
        }
        self.goal_slot.push(NO_GOAL);
        for column in [
            &mut self.stuck,
            &mut self.along,
            &mut self.fanned,
            &mut self.regeneration_left,
            &mut self.speed,
            &mut self.breather,
            &mut self.run_build,
            &mut self.regeneration_carry,
            &mut self.regeneration_halt,
            &mut self.drain_carry,
            &mut self.distance,
            &mut self.climb_left,
        ] {
            column.push(0.0);
        }
        self.closest.push(f64::INFINITY);
        self.bearing.push(EAST);
        self.stamina.push(FRESH);
        self.condition.push(AS_THINGS_STAND);
        self.wake_left.push(NOT_RECKONED);
    }

    /// Every column blank in `slot`.
    pub fn blank(&mut self, slot: usize) {
        for column in [
            &mut self.position,
            &mut self.loose_at,
            &mut self.flee_offset,
            &mut self.next,
            &mut self.foe_at,
            &mut self.toward,
            &mut self.foothold,
        ] {
            column[slot] = V2::ZERO;
        }
        for column in [
            &mut self.flags,
            &mut self.touch,
            &mut self.keys,
            &mut self.rank,
            &mut self.column,
            &mut self.hp,
            &mut self.attack_cooldown,
            &mut self.exerted,
            &mut self.was_running,
            &mut self.last_wounding,
            &mut self.on_face,
        ] {
            column[slot] = 0;
        }
        for column in [&mut self.goal_foe, &mut self.caught, &mut self.held_by, &mut self.target_id]
        {
            column[slot] = 0;
        }
        self.goal_slot[slot] = NO_GOAL;
        for column in [
            &mut self.stuck,
            &mut self.along,
            &mut self.fanned,
            &mut self.regeneration_left,
            &mut self.speed,
            &mut self.breather,
            &mut self.run_build,
            &mut self.regeneration_carry,
            &mut self.regeneration_halt,
            &mut self.drain_carry,
            &mut self.distance,
            &mut self.climb_left,
        ] {
            column[slot] = 0.0;
        }
        self.closest[slot] = f64::INFINITY;
        self.bearing[slot] = EAST;
        self.stamina[slot] = FRESH;
        self.condition[slot] = AS_THINGS_STAND;
        self.wake_left[slot] = NOT_RECKONED;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_slot_and_a_blanked_one_read_as_a_missing_key() {
        let mut columns = Columns::default();
        columns.grow();
        assert_eq!(columns.goal_slot[0], NO_GOAL);
        assert_eq!(columns.closest[0], f64::INFINITY);
        assert_eq!(columns.bearing[0], EAST);
        assert_eq!(columns.stamina[0], FRESH);
        assert_eq!(columns.condition[0], AS_THINGS_STAND);
        assert_eq!(columns.wake_left[0], NOT_RECKONED);
        columns.points("next").unwrap()[0] = V2 { x: 1.0, y: 2.0 };
        columns.ints("keys").unwrap()[0] = 5;
        columns.longs("goal_slot").unwrap()[0] = 3;
        columns.reals("closest").unwrap()[0] = 0.5;
        columns.reals("bearing").unwrap()[0] = 237.5;
        columns.ints("rank").unwrap()[0] = 3;
        columns.ints("hp").unwrap()[0] = 12;
        columns.ints("attack_cooldown").unwrap()[0] = 4;
        columns.reals("regeneration_left").unwrap()[0] = 2.5;
        columns.reals("stamina").unwrap()[0] = 42.5;
        columns.reals("speed").unwrap()[0] = 1.25;
        columns.ints("exerted").unwrap()[0] = 1;
        columns.reals("condition").unwrap()[0] = 0.5;
        columns.reals("regeneration_carry").unwrap()[0] = 0.25;
        columns.reals("regeneration_halt").unwrap()[0] = 1.5;
        columns.reals("wake_left").unwrap()[0] = 4.5;
        columns.reals("drain_carry").unwrap()[0] = 0.75;
        columns.ints("last_wounding").unwrap()[0] = 2;
        columns.longs("target_id").unwrap()[0] = 7;
        columns.reals("distance").unwrap()[0] = 2.5;
        columns.points("foothold").unwrap()[0] = V2 { x: 3.5, y: 1.5 };
        columns.ints("on_face").unwrap()[0] = 1;
        columns.reals("climb_left").unwrap()[0] = 4.0;
        columns.blank(0);
        assert_eq!(columns.next[0], V2::ZERO);
        assert_eq!(columns.keys[0], 0);
        assert_eq!(columns.goal_slot[0], NO_GOAL);
        assert_eq!(columns.closest[0], f64::INFINITY);
        assert_eq!(columns.bearing[0], EAST);
        assert_eq!(columns.rank[0], 0);
        assert_eq!(columns.hp[0], 0);
        assert_eq!(columns.attack_cooldown[0], 0);
        assert_eq!(columns.regeneration_left[0], 0.0);
        assert_eq!(columns.stamina[0], FRESH);
        assert_eq!(columns.speed[0], 0.0);
        assert_eq!(columns.exerted[0], 0);
        assert_eq!(columns.condition[0], AS_THINGS_STAND);
        assert_eq!(columns.regeneration_carry[0], 0.0);
        assert_eq!(columns.regeneration_halt[0], 0.0);
        assert_eq!(columns.wake_left[0], NOT_RECKONED);
        assert_eq!(columns.drain_carry[0], 0.0);
        assert_eq!(columns.last_wounding[0], 0);
        assert_eq!(columns.target_id[0], 0);
        assert_eq!(columns.distance[0], 0.0);
        assert_eq!(columns.foothold[0], V2::ZERO);
        assert_eq!(columns.on_face[0], 0);
        assert_eq!(columns.climb_left[0], 0.0);
    }

    #[test]
    fn every_column_is_found_by_its_unit_columns_name_and_no_other() {
        let mut columns = Columns::default();
        for name in ["position", "at", "offset", "next", "foe_at", "toward", "foothold"] {
            assert!(columns.points(name).is_some(), "{name}");
        }
        for name in [
            "flags",
            "touch",
            "keys",
            "rank",
            "column",
            "hp",
            "attack_cooldown",
            "exerted",
            "was_running",
            "last_wounding",
            "on_face",
        ] {
            assert!(columns.ints(name).is_some(), "{name}");
        }
        for name in ["goal_foe", "goal_slot", "caught", "held_by", "target_id"] {
            assert!(columns.longs(name).is_some(), "{name}");
        }
        for name in [
            "stuck",
            "closest",
            "along",
            "fanned",
            "bearing",
            "regeneration_left",
            "stamina",
            "speed",
            "breather",
            "run_build",
            "condition",
            "regeneration_carry",
            "regeneration_halt",
            "wake_left",
            "drain_carry",
            "distance",
            "climb_left",
        ] {
            assert!(columns.reals(name).is_some(), "{name}");
        }
        assert!(columns.points("flags").is_none());
        assert!(columns.reals("next").is_none());
    }
}
