//! Closing ranks after a fight (SquadRanks, spec 27 round 6; spec 39 step 4b): the dead
//! leave holes in a squad's places, so when its fight ends its living units are laid out
//! again as a solid block - front band first, then by their old rank and column - in rows
//! as wide as the squad, the last row centred.

use crate::columns::Columns;

/// A living unit as closing ranks reads it: its store slot, preferred band (0 the front)
/// and footprint (width, depth); its place is the store's.
pub struct Ranked {
    pub slot: usize,
    pub preferred: i64,
    pub foot: (i32, i32),
}

/// Lays the squad's living units (`units`, in its list's order) out again in rows `width`
/// columns wide, writing their new places to the store. A place holds one unit, so no two
/// sort alike; if two ever did, they would keep their list's order.
pub fn close(columns: &mut Columns, units: &[Ranked], width: i32) {
    let mut order: Vec<&Ranked> = units.iter().collect();
    order.sort_by_key(|u| (u.preferred, columns.rank[u.slot], columns.column[u.slot]));
    let mut rank = 0;
    for row in rows(&order, width) {
        let used: i32 = row.iter().map(|u| u.foot.0).sum();
        let depth = row.iter().map(|u| u.foot.1).fold(1, i32::max);
        let mut column = (width - used) / 2;
        for unit in row {
            columns.rank[unit.slot] = rank;
            columns.column[unit.slot] = column;
            column += unit.foot.0;
        }
        rank += depth;
    }
}

/// The units split into rows no wider than `width` columns.
fn rows<'a>(order: &[&'a Ranked], width: i32) -> Vec<Vec<&'a Ranked>> {
    let mut rows: Vec<Vec<&Ranked>> = vec![Vec::new()];
    let mut used = 0;
    for &unit in order {
        if used + unit.foot.0 > width && !rows.last().unwrap().is_empty() {
            rows.push(Vec::new());
            used = 0;
        }
        rows.last_mut().unwrap().push(unit);
        used += unit.foot.0;
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store(places: &[(i32, i32)]) -> Columns {
        let mut columns = Columns::default();
        for &(rank, column) in places {
            columns.grow();
            let slot = columns.rank.len() - 1;
            columns.rank[slot] = rank;
            columns.column[slot] = column;
        }
        columns
    }

    fn grem(slot: usize, preferred: i64) -> Ranked {
        Ranked { slot, preferred, foot: (1, 1) }
    }

    #[test]
    fn the_living_close_up_front_band_first_then_by_old_place_the_last_row_centred() {
        // a 3-wide squad's survivors: holes at (0, 1) and (1, 0)
        let mut columns = store(&[(1, 2), (0, 0), (2, 1), (0, 2), (1, 1)]);
        let units = [grem(0, 0), grem(1, 0), grem(2, 1), grem(3, 0), grem(4, 0)];
        close(&mut columns, &units, 3);
        let places: Vec<(i32, i32)> =
            (0..5).map(|s| (columns.rank[s], columns.column[s])).collect();
        // band 0 by old place: (0,0) (0,2) (1,1) (1,2); then band 1
        assert_eq!(places, vec![(1, 0), (0, 0), (1, 1), (0, 1), (0, 2)]);
    }

    #[test]
    fn a_wide_unit_starts_a_row_and_a_deep_one_makes_it_deeper() {
        let mut columns = store(&[(0, 0), (0, 1), (0, 3)]);
        let units = [grem(0, 0), Ranked { slot: 1, preferred: 0, foot: (2, 2) }, grem(2, 0)];
        close(&mut columns, &units, 3);
        let places: Vec<(i32, i32)> =
            (0..3).map(|s| (columns.rank[s], columns.column[s])).collect();
        assert_eq!(places, vec![(0, 0), (0, 1), (2, 1)]);
    }

    #[test]
    fn the_layout_is_the_same_whatever_the_list_order() {
        let mut one = store(&[(1, 2), (0, 0), (2, 1)]);
        let mut other = store(&[(1, 2), (0, 0), (2, 1)]);
        close(&mut one, &[grem(0, 0), grem(1, 0), grem(2, 1)], 2);
        close(&mut other, &[grem(2, 1), grem(1, 0), grem(0, 0)], 2);
        assert_eq!((one.rank, one.column), (other.rank, other.column));
    }
}
