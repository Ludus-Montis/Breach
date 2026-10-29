//! The squad store (spec 39 step 1; GDScript's SquadColumns): each squad's state that a
//! pass reads or writes, as columns by the squad's slot, the layout of the unit store's
//! (`Columns`, Decision 146). A squad takes a slot as it is made and keeps it for the
//! battle: squads are few, and one that has left the lists may still be read (an event, the
//! presentation). A slot is storage only: no pass loops in slot order or breaks a tie by it.
//! Its members are its units' unit-store slots, in its list's order. What it holds, by
//! SkirmishSquad's names: which way it points along its route (`direction`), its width, its
//! order and state, whether it has fought, the route patches it carries, its stalled ticks,
//! its manoeuvre, its morale, its rally ticks, whether ground halted it, its painted width
//! and narrowing ticks, its wait, whether it is re-forming, the lane's combat width; the
//! squad it fights, the ticks its fight and its taking began and it gave up taking; how far
//! it set out and how far its front is along its route, its heading, where it last looked
//! ahead, its painted shift and its centre shift; and where its front stands. A slot newly
//! given starts blank, each column at SkirmishSquad's starting value.
//!
//! Its records (spec 39 step 4b), which SkirmishSquad's dictionaries of the same names are
//! views over: its loose and fleeing units (their ids, in the order each entry was given;
//! the entries' keys are the unit store's columns, `entries`), the line it re-faced to
//! (`stance`), the foes on its other edges (`flanks`, in the order each began) and its units
//! out chasing (`chasers`, in the order each set out). Since spec 39 step 7c, also its
//! pursuit as a whole (`pursuit`) and its withdrawal (`withdraw`, the ticks it has felt
//! safe). Since step 8b4-i, also the route each follows (`route`, none: a straight lane), as
//! SkirmishSquad.route sets it. Since step 8b4-iii, also the command each follows
//! (`command`) and the commands themselves (`commands`); since 8b4-iv, its hold-until
//! order (`staging`); since 8b4-v, its swaps under way (`swaps`); since 8b4-vi, its units out
//! taking the downed (`taking`); since 8b4-vii, the orders given for the next tick (`pending`)
//! and the ids issued (`squads_issued`, `units_issued`). Since 8b5-iv, its faction (`faction`,
//! coded in the unit store's lexicon, so it compares with a unit's) and the units that joined
//! it as reinforcements (`joined`, their ids); its living and downed units are read from its
//! members and the unit store's STATE column (`living`, `downed`). Since 8b5-xvi, the route
//! a pursuer held its post on (`post_route`), which it takes up again back there.

use serde::{Deserialize, Serialize};

use crate::commands::Commands;
use crate::maths::V2;
use crate::route::Route;

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Squads {
    /// The squad's id in each slot.
    pub id: Vec<i64>,
    pub direction: Vec<i32>,
    pub width: Vec<i32>,
    pub order: Vec<i32>,
    pub state: Vec<i32>,
    pub fought: Vec<i32>,
    pub patched_count: Vec<i32>,
    pub stall_ticks: Vec<i32>,
    pub manoeuvre: Vec<i32>,
    pub morale: Vec<i32>,
    pub rally_ticks: Vec<i32>,
    pub blocked: Vec<i32>,
    pub painted_width: Vec<i32>,
    pub narrow_ticks: Vec<i32>,
    pub wait_ticks: Vec<i32>,
    pub reforming: Vec<i32>,
    pub combat_width: Vec<i32>,
    pub engaged_with: Vec<i64>,
    pub fight_since: Vec<i64>,
    pub taking_since: Vec<i64>,
    pub took_until: Vec<i64>,
    pub home_distance: Vec<f64>,
    pub front_distance: Vec<f64>,
    pub heading: Vec<f64>,
    pub looked_at: Vec<f64>,
    pub painted_shift: Vec<f64>,
    pub centre_shift: Vec<f64>,
    pub position: Vec<V2>,
    /// Its units' unit-store slots, in its list's order.
    pub members: Vec<Vec<i32>>,
    /// Its loose units' ids, in the order each went loose (SquadEntries).
    pub loose: Vec<Vec<i64>>,
    /// Its fleeing units' ids, in the order each began to flee (SquadEntries).
    pub fleeing: Vec<Vec<i64>>,
    /// The line it re-faced to (ScrumStance), if it holds one.
    pub stance: Vec<Option<Stance>>,
    /// Hostile squads fighting it on its other edges (Decision 78), in the order each began.
    pub flanks: Vec<Vec<Flank>>,
    /// Its units out chasing on their own (ScrumPursuit, Decision 109), in the order each set
    /// out.
    pub chasers: Vec<Vec<Chaser>>,
    /// Its pursuit as a whole under way (FormationPursuit, Decision 95), if any.
    pub pursuit: Vec<Option<Pursuit>>,
    /// Its withdrawal under way (FormationWithdraw, Decision 99): the ticks it has felt safe.
    pub withdraw: Vec<Option<i64>>,
    /// The route each follows (none: a straight lane along x).
    pub route: Vec<Option<Route>>,
    /// The route each pursuing as a whole held its post on (none: a straight lane, or none
    /// held), set as its pursuit began.
    pub post_route: Vec<Option<Route>>,
    /// The command each follows (a slot in `commands`; none till it is given one).
    pub command: Vec<Option<usize>>,
    /// Orders given for the next tick (FormationSimulation.order): [squad id, order], in
    /// the order first given (`orders::queue`).
    pub pending: Vec<(i64, i32)>,
    /// How many squad ids and unit ids the battle has issued (spawning: `issue_squad`,
    /// `issue_units`; spec 39 step 8b4-vii).
    pub squads_issued: i64,
    pub units_issued: i64,
    /// Its units out taking the downed (FormationTaking): [taker's id, its body's store
    /// slot] each, in the order they set out.
    pub taking: Vec<Vec<(i64, usize)>>,
    /// Its faction, as the unit store's lexicon codes a unit's (`words::Lexicon`; 0: "").
    pub faction: Vec<u32>,
    /// The units that joined it as reinforcements (FormationContact): their ids, in the
    /// order they joined. Only they spread beyond its painted columns.
    pub joined: Vec<Vec<i64>>,
    /// The swaps under way of each re-forming (FormationShuffle), in the order they began.
    pub swaps: Vec<Vec<crate::swaps::Held>>,
    /// The hold-until order each holds (FormationStaging; none: none).
    pub staging: Vec<Option<crate::staging::Stage>>,
    /// The commands the squads follow (spec 39 step 8b4-iii).
    pub commands: Commands,
}

/// A pursuit as a whole: the squad it pursues, the post it left (tiles along its own route,
/// and the point), how it held it (its home, direction and order), and whether it has given
/// up and marches back.
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct Pursuit {
    pub foe: i64,
    pub post: f64,
    pub post_at: V2,
    pub home: f64,
    pub direction: i32,
    pub order: i32,
    pub returning: bool,
}

/// A re-formed line (ScrumStance): its frame's anchor and heading, and the squad it faces.
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct Stance {
    pub anchor: V2,
    pub heading: f64,
    pub foe: i64,
}

/// A foe on one of a squad's edges (SquadEdges' edge): who, and the tick its contact began.
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct Flank {
    pub edge: i32,
    pub foe: i64,
    pub since: i64,
}

/// A unit out chasing on its own: its id, the squad it chases, where it set out from, its
/// leash, and whether it has given up and is on its way back.
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct Chaser {
    pub unit: i64,
    pub foe: i64,
    pub from: V2,
    pub leash: f64,
    pub returning: bool,
}

/// A squad not yet turned faces east (`heading`).
const EAST: f64 = 90.0;
/// No tick yet (`fight_since`, `taking_since`, `took_until`) and no morale read yet.
const NONE: i64 = -1;
/// It marches on the player's order (`manoeuvre`: FormationManoeuvre.Kind's 4).
const ORDERED: i32 = 4;
/// SkirmishUnit.State's DEAD, DOWNED and CARRIED: a state before DEAD is standing.
pub(crate) const DEAD: i32 = 4;
pub(crate) const DOWNED: i32 = 5;
pub(crate) const CARRIED: i32 = 8;

/// Every column's SquadColumns name, by its index there: the ints, the longs (from
/// `LONGS`), the reals (from `REALS`) and the points (from `POINTS`).
pub const NAMES: [&str; 27] = [
    "direction",
    "width",
    "order",
    "state",
    "fought",
    "patched_count",
    "stall_ticks",
    "manoeuvre",
    "morale",
    "rally_ticks",
    "blocked",
    "painted_width",
    "narrow_ticks",
    "wait_ticks",
    "reforming",
    "combat_width",
    "engaged_with",
    "fight_since",
    "taking_since",
    "took_until",
    "home_distance",
    "front_distance",
    "heading",
    "looked_at",
    "painted_shift",
    "centre_shift",
    "position",
];
pub const LONGS: usize = 16;
pub const REALS: usize = 20;
pub const POINTS: usize = 26;

impl Squads {
    /// An int or long column's value in a slot, by the column's index (`NAMES`).
    pub fn int(&mut self, slot: usize, column: usize) -> i64 {
        if column < LONGS {
            self.ints(NAMES[column]).unwrap()[slot] as i64
        } else {
            self.longs(NAMES[column]).unwrap()[slot]
        }
    }

    /// Sets an int or long column's value in a slot.
    pub fn set_int(&mut self, slot: usize, column: usize, value: i64) {
        if column < LONGS {
            self.ints(NAMES[column]).unwrap()[slot] = value as i32;
        } else {
            self.longs(NAMES[column]).unwrap()[slot] = value;
        }
    }

    /// A real column's value in a slot.
    pub fn real(&mut self, slot: usize, column: usize) -> f64 {
        debug_assert!((REALS..POINTS).contains(&column));
        self.reals(NAMES[column]).unwrap()[slot]
    }

    /// Sets a real column's value in a slot.
    pub fn set_real(&mut self, slot: usize, column: usize, value: f64) {
        debug_assert!((REALS..POINTS).contains(&column));
        self.reals(NAMES[column]).unwrap()[slot] = value;
    }

    /// The point column's value in a slot.
    pub fn point(&mut self, slot: usize, column: usize) -> V2 {
        debug_assert_eq!(column, POINTS);
        self.points(NAMES[column]).unwrap()[slot]
    }

    /// Sets the point column's value in a slot.
    pub fn set_point(&mut self, slot: usize, column: usize, value: V2) {
        debug_assert_eq!(column, POINTS);
        self.points(NAMES[column]).unwrap()[slot] = value;
    }

    /// A new squad's slot, every column blank there.
    pub fn open(&mut self, squad_id: i64) -> u32 {
        let slot = self.id.len() as u32;
        self.id.push(squad_id);
        for column in [
            &mut self.direction,
            &mut self.width,
            &mut self.order,
            &mut self.state,
            &mut self.fought,
            &mut self.patched_count,
            &mut self.stall_ticks,
            &mut self.rally_ticks,
            &mut self.blocked,
            &mut self.painted_width,
            &mut self.narrow_ticks,
            &mut self.wait_ticks,
            &mut self.reforming,
            &mut self.combat_width,
        ] {
            column.push(0);
        }
        self.manoeuvre.push(ORDERED);
        self.morale.push(NONE as i32);
        self.engaged_with.push(0);
        for column in [&mut self.fight_since, &mut self.taking_since, &mut self.took_until] {
            column.push(NONE);
        }
        for column in [
            &mut self.home_distance,
            &mut self.front_distance,
            &mut self.painted_shift,
            &mut self.centre_shift,
        ] {
            column.push(0.0);
        }
        self.heading.push(EAST);
        self.looked_at.push(f64::NEG_INFINITY);
        self.position.push(V2::ZERO);
        self.members.push(Vec::new());
        self.loose.push(Vec::new());
        self.fleeing.push(Vec::new());
        self.stance.push(None);
        self.flanks.push(Vec::new());
        self.chasers.push(Vec::new());
        self.pursuit.push(None);
        self.withdraw.push(None);
        self.route.push(None);
        self.post_route.push(None);
        self.command.push(None);
        self.staging.push(None);
        self.swaps.push(Vec::new());
        self.taking.push(Vec::new());
        self.faction.push(0);
        self.joined.push(Vec::new());
        slot
    }

    /// The squad in `slot`'s standing units, their unit-store slots in its members' order:
    /// those whose state (`states`, the unit store's STATE column) is before DEAD
    /// (UnitState.living).
    pub fn living(&self, slot: usize, states: &[i32]) -> Vec<i32> {
        let members = self.members[slot].iter();
        members.copied().filter(|&unit| states[unit as usize] < DEAD).collect()
    }

    /// The squad in `slot`'s downed units, DOWNED or CARRIED, as `living` reads them
    /// (UnitState.downed).
    pub fn downed(&self, slot: usize, states: &[i32]) -> Vec<i32> {
        let members = self.members[slot].iter();
        members.copied().filter(|&unit| matches!(states[unit as usize], DOWNED | CARRIED)).collect()
    }

    /// The next squad id the battle issues (ids from 1, in the order issued).
    pub fn issue_squad(&mut self) -> i64 {
        self.squads_issued += 1;
        self.squads_issued
    }

    /// The first of `count` unit ids the battle issues together (ids from 1, in order).
    pub fn issue_units(&mut self, count: i64) -> i64 {
        let first = self.units_issued + 1;
        self.units_issued += count;
        first
    }

    /// Sets the foe on `edge` of the squad in `slot`: an edge it already had keeps its
    /// place in the order, as a dictionary's key does.
    pub fn flank(&mut self, slot: usize, edge: i32, foe: i64, since: i64) {
        let flanks = &mut self.flanks[slot];
        match flanks.iter_mut().find(|f| f.edge == edge) {
            Some(held) => *held = Flank { edge, foe, since },
            None => flanks.push(Flank { edge, foe, since }),
        }
    }

    /// True if the squad in `slot` has `foe` on one of its edges.
    pub fn flanked_by(&self, slot: usize, foe: i64) -> bool {
        self.flanks[slot].iter().any(|f| f.foe == foe)
    }

    /// Sets the chase of the unit `chaser.unit` in the squad in `slot`: one already out
    /// keeps its place in the order.
    pub fn chase(&mut self, slot: usize, chaser: Chaser) {
        let chasers = &mut self.chasers[slot];
        match chasers.iter_mut().find(|c| c.unit == chaser.unit) {
            Some(held) => *held = chaser,
            None => chasers.push(chaser),
        }
    }

    /// True if the unit is out chasing for the squad in `slot`.
    pub fn chasing(&self, slot: usize, unit_id: i64) -> bool {
        self.chasers[slot].iter().any(|c| c.unit == unit_id)
    }

    /// Squads with a slot.
    pub fn count(&self) -> usize {
        self.id.len()
    }

    /// An int column by its SquadColumns name.
    pub fn ints(&mut self, name: &str) -> Option<&mut Vec<i32>> {
        match name {
            "direction" => Some(&mut self.direction),
            "width" => Some(&mut self.width),
            "order" => Some(&mut self.order),
            "state" => Some(&mut self.state),
            "fought" => Some(&mut self.fought),
            "patched_count" => Some(&mut self.patched_count),
            "stall_ticks" => Some(&mut self.stall_ticks),
            "manoeuvre" => Some(&mut self.manoeuvre),
            "morale" => Some(&mut self.morale),
            "rally_ticks" => Some(&mut self.rally_ticks),
            "blocked" => Some(&mut self.blocked),
            "painted_width" => Some(&mut self.painted_width),
            "narrow_ticks" => Some(&mut self.narrow_ticks),
            "wait_ticks" => Some(&mut self.wait_ticks),
            "reforming" => Some(&mut self.reforming),
            "combat_width" => Some(&mut self.combat_width),
            _ => None,
        }
    }

    /// A long column by its SquadColumns name.
    pub fn longs(&mut self, name: &str) -> Option<&mut Vec<i64>> {
        match name {
            "engaged_with" => Some(&mut self.engaged_with),
            "fight_since" => Some(&mut self.fight_since),
            "taking_since" => Some(&mut self.taking_since),
            "took_until" => Some(&mut self.took_until),
            _ => None,
        }
    }

    /// A real column by its SquadColumns name.
    pub fn reals(&mut self, name: &str) -> Option<&mut Vec<f64>> {
        match name {
            "home_distance" => Some(&mut self.home_distance),
            "front_distance" => Some(&mut self.front_distance),
            "heading" => Some(&mut self.heading),
            "looked_at" => Some(&mut self.looked_at),
            "painted_shift" => Some(&mut self.painted_shift),
            "centre_shift" => Some(&mut self.centre_shift),
            _ => None,
        }
    }

    /// A point column by its SquadColumns name.
    pub fn points(&mut self, name: &str) -> Option<&mut Vec<V2>> {
        match name {
            "position" => Some(&mut self.position),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn its_living_and_downed_are_its_members_by_state_in_its_order() {
        let mut squads = Squads::default();
        squads.open(1);
        squads.open(2);
        // states by unit slot: MOVING, DEAD, DOWNED, FIGHTING, CARRIED, TAKEN, ARRIVED
        let states = [0, DEAD, DOWNED, 2, CARRIED, 6, 3];
        squads.members[1] = vec![6, 4, 0, 1, 2, 5, 3];
        assert_eq!(squads.living(1, &states), vec![6, 0, 3]);
        assert_eq!(squads.downed(1, &states), vec![4, 2]);
        assert!(squads.living(0, &states).is_empty(), "a squad with no members");
    }

    #[test]
    fn ids_are_issued_from_1_in_order_squads_and_units_apart() {
        let mut squads = Squads::default();
        assert_eq!(squads.issue_squad(), 1);
        assert_eq!(squads.issue_units(3), 1);
        assert_eq!(squads.issue_squad(), 2);
        assert_eq!(squads.issue_units(2), 4);
        assert_eq!(squads.issue_units(0), 6);
        assert_eq!(squads.issue_units(1), 6);
    }

    #[test]
    fn a_new_squad_takes_the_next_slot_and_starts_as_skirmish_squad_does() {
        let mut squads = Squads::default();
        assert_eq!(squads.open(7), 0);
        assert_eq!(squads.open(3), 1);
        assert_eq!(squads.count(), 2);
        assert_eq!(squads.id, vec![7, 3]);
        assert_eq!(squads.heading[1], 90.0);
        assert_eq!(squads.order[1], 0); // ADVANCE
        assert_eq!(squads.state[1], 0); // MOVING
        assert_eq!(squads.fight_since[1], -1);
        assert_eq!(squads.taking_since[1], -1);
        assert_eq!(squads.took_until[1], -1);
        assert_eq!(squads.looked_at[1], f64::NEG_INFINITY);
        assert_eq!(squads.manoeuvre[1], 4);
        assert_eq!(squads.morale[1], -1);
        assert_eq!(squads.position[1], V2::ZERO);
        assert!(squads.members[1].is_empty());
        assert_eq!(squads.faction[1], 0); // ""
        assert!(squads.joined[1].is_empty());
    }

    #[test]
    fn every_column_is_found_by_its_squad_columns_name_and_no_other() {
        let mut squads = Squads::default();
        squads.open(1);
        let ints = [
            "direction",
            "width",
            "order",
            "state",
            "fought",
            "patched_count",
            "stall_ticks",
            "manoeuvre",
            "morale",
            "rally_ticks",
            "blocked",
            "painted_width",
            "narrow_ticks",
            "wait_ticks",
            "reforming",
            "combat_width",
        ];
        let longs = ["engaged_with", "fight_since", "taking_since", "took_until"];
        let reals = [
            "home_distance",
            "front_distance",
            "heading",
            "looked_at",
            "painted_shift",
            "centre_shift",
        ];
        for (n, name) in ints.iter().enumerate() {
            squads.ints(name).unwrap()[0] = n as i32 + 10;
            assert!(squads.longs(name).is_none() && squads.reals(name).is_none());
        }
        for (n, name) in longs.iter().enumerate() {
            squads.longs(name).unwrap()[0] = n as i64 + 100;
            assert!(squads.ints(name).is_none() && squads.points(name).is_none());
        }
        for (n, name) in reals.iter().enumerate() {
            squads.reals(name).unwrap()[0] = n as f64 + 0.5;
            assert!(squads.ints(name).is_none() && squads.longs(name).is_none());
        }
        squads.points("position").unwrap()[0] = V2 { x: 1.0, y: 2.0 };
        assert!(squads.ints("position").is_none() && squads.ints("members").is_none());
        // each name reached its own column
        assert_eq!(squads.direction[0], 10);
        assert_eq!(squads.combat_width[0], 25);
        assert_eq!(squads.took_until[0], 103);
        assert_eq!(squads.centre_shift[0], 5.5);
        assert_eq!(squads.position[0], V2 { x: 1.0, y: 2.0 });
    }
    #[test]
    fn a_column_by_index_is_the_column_of_that_name() {
        let mut squads = Squads::default();
        squads.open(1);
        for (c, name) in NAMES.iter().enumerate() {
            match c {
                c if c < LONGS => {
                    squads.set_int(0, c, c as i64 + 1);
                    assert_eq!(squads.ints(name).unwrap()[0], c as i32 + 1);
                    assert_eq!(squads.int(0, c), c as i64 + 1);
                }
                c if c < REALS => {
                    squads.set_int(0, c, 1 << 40);
                    assert_eq!(squads.longs(name).unwrap()[0], 1 << 40);
                    assert_eq!(squads.int(0, c), 1 << 40);
                }
                c if c < POINTS => {
                    squads.set_real(0, c, c as f64 + 0.25);
                    assert_eq!(squads.reals(name).unwrap()[0], c as f64 + 0.25);
                    assert_eq!(squads.real(0, c), c as f64 + 0.25);
                }
                c => {
                    squads.set_point(0, c, V2 { x: 3.0, y: 4.0 });
                    assert_eq!(squads.points(name).unwrap()[0], V2 { x: 3.0, y: 4.0 });
                    assert_eq!(squads.point(0, c), V2 { x: 3.0, y: 4.0 });
                }
            }
        }
    }
}
