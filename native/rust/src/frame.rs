//! A squad's frame (SquadFrame.basis and place_in, ScrumStance.frame; spec 39 step 4b): the
//! centre of a unit's place, its columns from the squad's left (centred on the front centre,
//! moved by the centre shift) and its ranks back from the front, in GDScript's precision.

use crate::maths::{bearing_vector, V2};
use crate::squads::Squads;

/// SquadFrame.basis: the frame worked out once for many units of one squad.
#[derive(Clone, Copy, Debug)]
pub struct Frame {
    anchor: V2,
    ahead: V2,
    orthogonal: V2,
    half_width: f64,
    shift: f64,
}

impl Frame {
    /// The frame at `anchor` turned to `heading`, `width` columns wide, shifted `shift`.
    pub fn new(anchor: V2, heading: f64, width: i32, shift: f64) -> Frame {
        let ahead = bearing_vector(heading);
        Frame {
            anchor,
            ahead,
            orthogonal: ahead.orthogonal(),
            half_width: width as f64 / 2.0,
            shift,
        }
    }

    /// The frame a squad's units keep to (ScrumStance.frame): its re-faced line's if it
    /// holds one, else its own.
    pub fn kept(squads: &Squads, squad: usize) -> Frame {
        let width = squads.width[squad];
        match squads.stance[squad] {
            Some(stance) => Frame::new(stance.anchor, stance.heading, width, 0.0),
            None => Frame::new(
                squads.position[squad],
                squads.heading[squad],
                width,
                squads.centre_shift[squad],
            ),
        }
    }

    /// SquadFrame.place_in: the centre of the place (rank, column) of a unit `foot` (width,
    /// depth) cells.
    pub fn place(&self, place: (i32, i32), foot: (i64, i64)) -> V2 {
        let across = place.1 as f64 - self.half_width + self.shift + foot.0 as f64 / 2.0;
        let back = place.0 as f64 + foot.1 as f64 / 2.0;
        self.anchor.add(self.orthogonal.scale(-across)).sub(self.ahead.scale(back))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_place_lies_its_columns_across_and_its_ranks_back_from_the_front() {
        let east = Frame::new(V2::new(10.0, 0.0), 90.0, 4, 0.0);
        // facing east, the left is north (negative y): column 0 of 4 is 1.5 cells north
        assert_eq!(east.place((0, 0), (1, 1)), V2::new(9.5, -1.5));
        assert_eq!(east.place((2, 3), (1, 1)), V2::new(7.5, 1.5));
        let shifted = Frame::new(V2::new(10.0, 0.0), 90.0, 4, 1.0);
        assert_eq!(shifted.place((0, 0), (2, 2)), V2::new(9.0, 0.0));
    }
}
