//! Bodies on the ground (GroundBodies, Decision 121, spec 28 part 6): the downed and the
//! dead lie where they fell, obstacles a walker steps over - slowed by the weight of what
//! it crosses against its own (bodies_ground_drag), blocked by one bodies_ground_block
//! times its mass. Every unit walking to its place, or loose in a fight, is slowed the same
//! way (`underfoot`). The footing is a least over the bodies lying near, so the grid's
//! visiting order can't matter.

use crate::grid::{Grid, MARGIN};
use crate::maths::V2;

/// The bodies lying on the ground, indexed: where each lies, its radius and its
/// footprint's area.
pub struct Lying<'a> {
    ats: &'a [V2],
    radii: &'a [f64],
    areas: &'a [f64],
    grid: Grid,
    widest: f64,
}

impl<'a> Lying<'a> {
    pub fn new(ats: &'a [V2], radii: &'a [f64], areas: &'a [f64]) -> Lying<'a> {
        let widest = radii.iter().copied().fold(0.0, f64::max);
        Lying { ats, radii, areas, grid: Grid::build(ats), widest }
    }

    /// GroundBodies.underfoot: the share (0 to 1) of its pace a walker of radius `own` and
    /// footprint `area` keeps stepping onto `step`; `drag` and `block` are
    /// bodies_ground_drag and bodies_ground_block.
    pub fn underfoot(&self, own: f64, area: f64, step: V2, drag: f64, block: f64) -> f64 {
        let mut worst = 1.0f64;
        let mut blocked = false;
        self.grid.near(step, own + self.widest + MARGIN, |i| {
            let i = i as usize;
            let reach = own + self.radii[i];
            if (self.ats[i].distance_to(step) as f64) >= reach {
                return true;
            }
            let weight = self.areas[i] / area;
            if weight >= block {
                blocked = true;
                return false;
            }
            let share = 1.0 / (1.0 + weight * drag);
            if share < worst {
                worst = share;
            }
            true
        });
        if blocked {
            0.0
        } else {
            worst
        }
    }
}
