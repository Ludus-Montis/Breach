//! The settling rule (spec 42): a contest settled in parallel, whatever the order its
//! proposals were made in. Each contender proposes a point with its key in the contest; a
//! proposal wins if no proposal ranking before it lies within its reach. Conflicts are found
//! through a grid, so each check is local; the key decides, never the list's order or an id
//! (Decision 97), and the check runs on the pool or not with the same result (Decision 159).
//! A contest gives its key, its reach and, for those who lose every round, its fallback.

use rayon::prelude::*;

use crate::grid::{Grid, MARGIN};
use crate::maths::V2;

/// What one contender wants: a point, its key, and how near another's point conflicts.
pub struct Proposal<K> {
    pub key: K,
    pub at: V2,
    /// Another proposal closer than this to `at` conflicts with this one.
    pub reach: f64,
}

/// Whether each proposal wins: no proposal within its reach ranks before it (`before(a, b)`:
/// `a` ranks before `b`, a total order).
pub fn winners<K: Sync>(
    proposals: &[Proposal<K>],
    before: impl Fn(&K, &K) -> bool + Sync,
    threaded: bool,
) -> Vec<bool> {
    let points: Vec<V2> = proposals.iter().map(|p| p.at).collect();
    let grid = Grid::build(&points);
    let wins = |(i, p): (usize, &Proposal<K>)| {
        let mut won = true;
        grid.near(p.at, p.reach + MARGIN, |j| {
            let q = &proposals[j as usize];
            let close = (q.at.distance_to(p.at) as f64) < p.reach;
            won = j as usize == i || !close || !before(&q.key, &p.key);
            won
        });
        won
    };
    if threaded {
        proposals.par_iter().enumerate().map(wins).collect()
    } else {
        proposals.iter().enumerate().map(wins).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(key: i64, x: f32, reach: f64) -> Proposal<i64> {
        Proposal { key, at: V2::new(x, 0.0), reach }
    }

    fn settle(proposals: &[Proposal<i64>], threaded: bool) -> Vec<bool> {
        winners(proposals, |a, b| a < b, threaded)
    }

    #[test]
    fn of_two_conflicting_proposals_the_one_ranking_first_wins() {
        let proposals = [at(7, 0.0, 1.0), at(3, 0.5, 1.0)];
        assert_eq!(settle(&proposals, false), vec![false, true]);
        let reversed = [at(3, 0.5, 1.0), at(7, 0.0, 1.0)];
        assert_eq!(settle(&reversed, false), vec![true, false]); // the key, not the list
    }

    #[test]
    fn proposals_out_of_each_other_s_reach_all_win() {
        let proposals = [at(7, 0.0, 1.0), at(3, 1.0, 1.0), at(5, 9.0, 1.0)];
        assert_eq!(settle(&proposals, false), vec![true; 3]);
    }

    #[test]
    fn a_proposal_loses_to_one_ranking_first_within_its_own_reach() {
        // the narrow one ranks first: the wide one, within whose reach it lies, loses
        let proposals = [at(1, 0.0, 0.5), at(2, 1.5, 2.0)];
        assert_eq!(settle(&proposals, false), vec![true, false]);
        // the wide one ranks first, outside the narrow one's reach: both win
        let proposals = [at(2, 0.0, 0.5), at(1, 1.5, 2.0)];
        assert_eq!(settle(&proposals, false), vec![true, true]);
    }

    #[test]
    fn a_proposal_beaten_by_one_that_itself_lost_still_loses() {
        let proposals = [at(1, 0.0, 1.0), at(2, 0.8, 1.0), at(3, 1.6, 1.0)];
        assert_eq!(settle(&proposals, false), vec![true, false, false]);
    }

    #[test]
    fn the_pool_settles_as_one_thread_does() {
        let proposals: Vec<Proposal<i64>> =
            (0..500).map(|n| at((n * 7919) % 1009, (n % 97) as f32 * 0.6, 1.0)).collect();
        assert_eq!(settle(&proposals, true), settle(&proposals, false));
    }
}
