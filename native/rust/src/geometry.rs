//! Where squads stand relative to one another (spec 39 step 4c; SquadFrame's corners and
//! spans, SquadGeometry, SquadEdges, Decisions 74, 75 and 78): a squad's frame is the centre
//! of its front edge turned to its heading, and a unit's footprint lies in it by its place
//! (rank, column). Gaps along a squad's heading, whether two lines overlap across it, a
//! squad's four edges (front, right, rear and left, clockwise) and which units stand on one,
//! and the box its footprints cover. In GDScript's precision: points and extents in f32,
//! gaps in f64; distances between squads in tiles, everything else in cells.

use crate::columns::Columns;
use crate::maths::{bearing_vector, V2};
use crate::motion::fposmod;
use crate::rect::Rect;
use crate::roster::{Listed, Member};
use crate::squads::Squads;

/// Cells a tile spans (MapLayoutDef.CELLS_PER_TILE).
pub const CELLS: f64 = 64.0;
/// Tiles between one rank and the next: one cell (SkirmishSquad.RANK_DEPTH).
pub const RANK_DEPTH: f64 = 1.0 / CELLS;
const EPSILON: f64 = 0.000001;
/// Degrees off parallel two squads' headings may be and still share an axis.
const PARALLEL: f64 = 45.0;
/// Degrees apart two squads' headings must be for them to face each other's way.
const FACING_OFF: f64 = 135.0;

/// An extent along an axis: (least, most), as a GDScript Vector2 holds it.
pub type Extent = (f32, f32);
const NO_EXTENT: Extent = (f32::INFINITY, f32::NEG_INFINITY);

/// A squad's frame (StoredSquad.frame): the centre of its front edge, its heading, its width
/// in columns and its centre shift.
#[derive(Clone, Copy, Debug)]
pub struct Frame {
    pub anchor: V2,
    pub heading: f64,
    pub width: i32,
    pub shift: f64,
}

impl Frame {
    pub fn of(squads: &Squads, squad: usize) -> Frame {
        Frame {
            anchor: squads.position[squad],
            heading: squads.heading[squad],
            width: squads.width[squad],
            shift: squads.centre_shift[squad],
        }
    }

    /// SquadFrame.corners: the corners of a unit's footprint `foot` (width, depth) at
    /// `place` (rank, column), its columns from the squad's left, its ranks back.
    pub fn corners(&self, foot: (i64, i64), place: (i32, i32)) -> [V2; 4] {
        let ahead = bearing_vector(self.heading);
        let o = ahead.orthogonal();
        let right = V2::new(-o.x, -o.y);
        let across = place.1 as f64 - self.width as f64 / 2.0 + self.shift;
        let near = self.anchor.add(right.scale(across)).sub(ahead.scale(place.0 as f64));
        let wide = right.scale(foot.0 as f64);
        let deep = V2::new(-ahead.x, -ahead.y).scale(foot.1 as f64);
        [near, near.add(wide), near.add(wide).add(deep), near.add(deep)]
    }

    /// SquadFrame.unit_rect: the box round a unit's turned footprint.
    pub fn unit_rect(&self, foot: (i64, i64), place: (i32, i32)) -> Rect {
        let points = self.corners(foot, place);
        points.iter().fold(Rect::new(points[0], V2::ZERO), |area, &p| area.expand(p))
    }
}

/// SquadFrame.lateral_axis: the world axis a squad's columns run along at `heading`.
pub fn lateral_axis(heading: f64) -> V2 {
    bearing_vector(fposmod(heading, 180.0) + 90.0)
}

/// SquadFrame.extent: how far `points` reach along `axis`.
pub fn extent(points: &[V2], axis: V2) -> Extent {
    points.iter().fold(NO_EXTENT, |(low, high), p| widen((low, high), p.dot(axis)))
}

/// `Vector2(minf(low, along), maxf(high, along))`.
#[inline]
fn widen((low, high): Extent, along: f32) -> Extent {
    (if low < along { low } else { along }, if high > along { high } else { along })
}

/// How much two extents overlap: `minf(a.y, b.y) - maxf(a.x, b.x)`, in f64.
#[inline]
fn overlap(a: Extent, b: Extent) -> f64 {
    let most = if a.1 < b.1 { a.1 } else { b.1 } as f64;
    let least = if a.0 > b.0 { a.0 } else { b.0 } as f64;
    most - least
}

/// A listed squad's units in its frame: what its geometry reads.
pub struct Shape<'a> {
    pub frame: Frame,
    pub living: &'a [Member],
    pub columns: &'a Columns,
    pub position: V2,
    pub heading: f64,
}

impl<'a> Shape<'a> {
    pub fn of(squads: &Squads, columns: &'a Columns, listed: &'a Listed) -> Shape<'a> {
        let frame = Frame::of(squads, listed.squad);
        Shape {
            frame,
            living: &listed.living,
            columns,
            position: frame.anchor,
            heading: frame.heading,
        }
    }

    fn place(&self, unit: &Member) -> (i32, i32) {
        (self.columns.rank[unit.slot], self.columns.column[unit.slot])
    }

    /// A unit's footprint's corners in the frame.
    pub fn corners(&self, unit: &Member) -> [V2; 4] {
        self.frame.corners(unit.foot, self.place(unit))
    }

    /// SquadGeometry.lateral: the squad's extent along `axis` from its living units' spans.
    pub fn lateral(&self, axis: V2) -> Extent {
        self.living.iter().fold(NO_EXTENT, |(low, high), unit| {
            let span = extent(&self.corners(unit), axis);
            (if low < span.0 { low } else { span.0 }, if high > span.1 { high } else { span.1 })
        })
    }

    /// SquadEdges.reach: how far its living footprints reach along `way`.
    pub fn reach(&self, way: V2) -> Extent {
        let mut out = NO_EXTENT;
        for unit in self.living {
            for p in self.corners(unit) {
                out = widen(out, p.dot(way));
            }
        }
        out
    }

    /// SquadEdges.bounds: the cells its living units cover (an empty box at the origin for
    /// none).
    pub fn bounds(&self) -> Rect {
        let mut rects = self.living.iter().map(|u| self.frame.unit_rect(u.foot, self.place(u)));
        let Some(first) = rects.next() else { return Rect::default() };
        rects.fold(first, |area, rect| area.merge(rect))
    }

    /// SquadEdges.edge_units: the indices (in its living) of the units on `edge` (0 front,
    /// 1 right, 2 rear, 3 left): those with no living unit of their squad beyond them.
    pub fn edge_units(&self, edge: i32) -> Vec<usize> {
        let turned = self.heading + edge as f64 * 90.0;
        let (outward, across) = (bearing_vector(turned), lateral_axis(turned));
        let corners: Vec<[V2; 4]> = self.living.iter().map(|u| self.corners(u)).collect();
        let spans: Vec<Extent> = corners.iter().map(|c| extent(c, across)).collect();
        let reaches: Vec<f32> = corners.iter().map(|c| extent(c, outward).1).collect();
        (0..self.living.len())
            .filter(|&u| {
                !(0..self.living.len()).any(|o| {
                    o != u
                        && overlap(spans[u], spans[o]) > EPSILON
                        && reaches[o] as f64 > reaches[u] as f64 + EPSILON
                })
            })
            .collect()
    }

    /// How deep the squad is in ranks (FormationContact._back_rows).
    pub fn back_rows(&self) -> i64 {
        self.living.iter().map(|u| self.columns.rank[u.slot] as i64 + u.foot.1).max().unwrap_or(0)
    }
}

/// SquadGeometry.gap: how far `to`'s front lies ahead of `from`'s along `from`'s heading,
/// in tiles.
pub fn gap(from: &Shape, to: &Shape) -> f64 {
    to.position.sub(from.position).dot(bearing_vector(from.heading)) as f64 / CELLS
}

/// SquadGeometry.overlaps: true if the two lines overlap across `from`'s heading, the two
/// headed along one axis (within 45 degrees of parallel, either way).
pub fn overlaps(from: &Shape, to: &Shape) -> bool {
    let apart = fposmod(from.heading - to.heading, 180.0);
    if apart > PARALLEL + EPSILON && apart < 180.0 - PARALLEL - EPSILON {
        return false;
    }
    let axis = lateral_axis(from.heading);
    overlap(from.lateral(axis), to.lateral(axis)) > EPSILON
}

/// SquadGeometry.facing_off: true if the two face each other's way, their headings at least
/// FACING_OFF degrees apart.
pub fn facing_off(from: &Shape, to: &Shape) -> bool {
    let apart = (fposmod(from.heading - to.heading + 180.0, 360.0) - 180.0).abs();
    apart > FACING_OFF - EPSILON
}

/// SquadEdges.face_gap: how far `to`'s near face lies ahead of `from`'s front along `from`'s
/// heading, in tiles.
pub fn face_gap(from: &Shape, to: &Shape) -> f64 {
    let ahead = bearing_vector(from.heading);
    let near = to.reach(ahead).0 as f64;
    (near - from.position.dot(ahead) as f64) / CELLS
}

/// SquadEdges.overlap_across: true if `from`'s line overlaps `to`'s footprints across
/// `from`'s heading.
pub fn overlap_across(from: &Shape, to: &Shape) -> bool {
    let axis = lateral_axis(from.heading);
    overlap(from.lateral(axis), to.reach(axis)) > EPSILON
}

/// SquadEdges.edge_hit: the edge of a squad heading `victim` an attacker heading `attacker`
/// (degrees) meets with its front (on the line between two, the one clockwise of it).
pub fn edge_hit(victim: f64, attacker: f64) -> i32 {
    let from = fposmod(attacker + 180.0 - victim, 360.0);
    ((from / 90.0).round() as i64).rem_euclid(4) as i32
}

#[cfg(test)]
#[path = "geometry/tests.rs"]
mod tests;
