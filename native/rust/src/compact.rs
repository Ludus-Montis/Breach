//! A squad closing up when units fall (SquadCompact, Decision 47; spec 39 step 5b) and a
//! squad taken in behind another's back rank (FormationContact.reinforce, Decisions 44 and
//! 51). Closing up: living units step up a rank while their whole footprint is clear, until
//! nothing more can move, but only front-preferring units step into the front rank. If the
//! whole front has fallen, the squad re-anchors: its foremost rank becomes rank 0 and its
//! front moves back to match, so nobody moves on the route. Only ranks change. Taking in:
//! the joining squad's living units fall in behind the leader's back rank, both lines
//! centred on the wider of the two, and step up. Moving the units between the squads' lists
//! and commands (FormationCommand.enlist) is GDScript's.

use crate::columns::Columns;
use crate::content::CELLS_PER_TILE;
use crate::entries;
use crate::march::shuffle::front_first;
#[cfg(test)]
use crate::maths::V2;
use crate::squads::Squads;

/// A living unit of a squad closing up, in the squad's order: its id, store slot, footprint
/// (width, depth) and preferred band (0: the front).
#[derive(Clone, Debug)]
pub struct Closer {
    pub id: i64,
    pub slot: usize,
    pub foot: (i32, i32),
    pub preferred: i64,
}

/// A unit in a move under way (FormationShuffle's swaps): its store slot, the place (rank,
/// column) it is heading for and its footprint (width, depth). It keeps its place until the
/// move ends, and nobody steps into where it is heading.
pub struct Moving {
    pub slot: usize,
    pub to: (i32, i32),
    pub foot: (i32, i32),
}

/// A squad taken in: the leader's squad slot, the joining squad's, and the store slots of
/// all the leader's units (the fallen too: the whole frame shifts).
pub struct Joining<'a> {
    pub leader: usize,
    pub joining: usize,
    pub leader_units: &'a [usize],
}

/// SquadCompact.compact: the squad in squad slot `squad` closes up. Returns the indices into
/// `living` of the units that stepped up, in the order each first did. Each pass looks at
/// the units front first; after the first, a unit is looked at again only if a place in one
/// of its columns changed since it was last looked at: the row ahead of it lies in its own
/// columns, so any other would find it as held as before (spec 35 step 5b).
pub fn compact(
    squads: &mut Squads,
    columns: &mut Columns,
    squad: usize,
    living: &[Closer],
    movers: &[Moving],
) -> Vec<usize> {
    let mut moved = Vec::new();
    if living.is_empty() {
        return moved;
    }
    let mut held = Held::new(columns, living, movers);
    let mut looked: Vec<Option<i64>> = vec![None; living.len()];
    let mut stepped = vec![false; living.len()];
    let mut any = true;
    while any {
        any = false;
        let places: Vec<(i32, i32)> =
            living.iter().map(|c| (columns.rank[c.slot], columns.column[c.slot])).collect();
        for n in front_first(&places) {
            let unit = &living[n];
            if movers.iter().any(|m| m.slot == unit.slot) {
                continue; // mid-move: it takes the place its move is heading for
            }
            let file = columns.column[unit.slot];
            if let Some(time) = looked[n] {
                if !held.changed_since(time, file, unit.foot.0) {
                    continue;
                }
            }
            looked[n] = Some(held.tick());
            let rank = columns.rank[unit.slot];
            if rank <= 0 || !(rank > 1 || unit.preferred == 0) {
                continue;
            }
            if held.clear_ahead(rank, file, unit.foot.0) {
                held.step_up(rank, file, unit.foot);
                columns.rank[unit.slot] = rank - 1;
                any = true;
                if !stepped[n] {
                    stepped[n] = true;
                    moved.push(n);
                }
            }
        }
    }
    if !living.iter().any(|c| columns.rank[c.slot] == 0) {
        let shift = living.iter().map(|c| columns.rank[c.slot]).fold(1 << 30, i32::min);
        for c in living {
            columns.rank[c.slot] -= shift;
        }
        let rank_depth = 1.0 / CELLS_PER_TILE as f64;
        squads.front_distance[squad] -= (squads.direction[squad] * shift) as f64 * rank_depth;
    }
    moved
}

/// FormationContact.reinforce: the joining squad's living units (`joining`, in its order)
/// move in behind the leader's back rank, both lines centred on the wider of the two; they
/// are no longer loose or fleeing, and strike no sooner than next tick. The leader widens to
/// fit, its centre staying put, and closes up over its living units (`leader`, in its order)
/// with the joining ones after them. Returns the indices into those of the units that
/// stepped up (compact).
pub fn reinforce(
    squads: &mut Squads,
    columns: &mut Columns,
    join: Joining,
    leader: &[Closer],
    joining: &[Closer],
    movers: &[Moving],
) -> Vec<usize> {
    let (from, to) = (join.leader, join.joining);
    let width = squads.width[from].max(squads.width[to]);
    let back = leader.iter().map(|c| columns.rank[c.slot] + c.foot.1).fold(0, i32::max);
    let shift = (width - squads.width[from]) / 2;
    for &slot in join.leader_units {
        columns.column[slot] += shift;
    }
    let across = (width - squads.width[to]) / 2;
    for c in joining {
        columns.rank[c.slot] += back;
        columns.column[c.slot] += across;
        columns.attack_cooldown[c.slot] = 1;
        entries::unloose(squads, columns, to, (c.id, c.slot));
        entries::unflee(squads, columns, to, (c.id, c.slot));
    }
    squads.centre_shift[from] += (width - squads.width[from]) as f64 / 2.0 - shift as f64;
    squads.width[from] = width;
    let all: Vec<Closer> = leader.iter().chain(joining).cloned().collect();
    compact(squads, columns, from, &all, movers)
}

/// The places held while a squad closes up, as a count per place on a grid over every place
/// a step-up can look at: how many living units' footprints cover it, plus how many moves
/// under way are heading there. A unit's row ahead is never one of its own places, and a
/// mover never steps up, so a place is clear to a unit exactly when its count is 0. Each
/// column of the grid keeps when a place in it last changed, on a clock `tick` advances.
struct Held {
    counts: Vec<i32>,
    changed_at: Vec<i64>,
    first_rank: i32,
    first_column: i32,
    ranks: i32,
    columns: i32,
    clock: i64,
}

impl Held {
    fn new(at: &Columns, living: &[Closer], movers: &[Moving]) -> Held {
        let (mut low_rank, mut low_column) = (0, 1 << 30); // rank 0: the lowest a step-up looks at
        let (mut high_rank, mut high_column) = (-(1 << 30), -(1 << 30));
        for c in living {
            let (rank, column) = (at.rank[c.slot], at.column[c.slot]);
            low_rank = low_rank.min(rank);
            low_column = low_column.min(column);
            high_rank = high_rank.max(rank + c.foot.1 - 1);
            high_column = high_column.max(column + c.foot.0 - 1);
        }
        let ranks = high_rank - low_rank + 1;
        let columns = high_column - low_column + 1;
        let mut held = Held {
            counts: vec![0; (ranks * columns) as usize],
            changed_at: vec![0; columns as usize],
            first_rank: low_rank,
            first_column: low_column,
            ranks,
            columns,
            clock: 0,
        };
        for c in living {
            held.cover((at.rank[c.slot], at.column[c.slot]), c.foot);
        }
        for m in movers {
            held.cover(m.to, m.foot);
        }
        held
    }

    /// The clock's next time.
    fn tick(&mut self) -> i64 {
        self.clock += 1;
        self.clock
    }

    /// True if a place in any of the `width` columns from `file` changed after `time`.
    fn changed_since(&self, time: i64, file: i32, width: i32) -> bool {
        let from = file - self.first_column;
        (from..from + width).any(|column| self.changed_at[column as usize] > time)
    }

    /// True if nothing holds the row ahead of a unit at `rank` and `file`, `width` wide.
    fn clear_ahead(&self, rank: i32, file: i32, width: i32) -> bool {
        let row = rank - 1 - self.first_rank;
        if row < 0 || row >= self.ranks {
            return true;
        }
        let from = file - self.first_column;
        (from..from + width).all(|column| self.counts[(row * self.columns + column) as usize] == 0)
    }

    /// A unit at `rank` and `file` steps up a rank: its footprint (width, depth) with it.
    fn step_up(&mut self, rank: i32, file: i32, foot: (i32, i32)) {
        let now = self.tick();
        let back = rank + foot.1 - 1 - self.first_rank;
        let ahead = rank - 1 - self.first_rank;
        let from = file - self.first_column;
        for column in from..from + foot.0 {
            self.counts[(back * self.columns + column) as usize] -= 1;
            self.counts[(ahead * self.columns + column) as usize] += 1;
            self.changed_at[column as usize] = now;
        }
    }

    fn cover(&mut self, at: (i32, i32), foot: (i32, i32)) {
        for rank in at.0 - self.first_rank..at.0 - self.first_rank + foot.1 {
            for column in at.1 - self.first_column..at.1 - self.first_column + foot.0 {
                if rank >= 0 && rank < self.ranks && column >= 0 && column < self.columns {
                    self.counts[(rank * self.columns + column) as usize] += 1;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
