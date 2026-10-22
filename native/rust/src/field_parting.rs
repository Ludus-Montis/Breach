//! Body parting (UnitBodies) on the field's own state: the bodies gathered in draw order
//! natively - no GDScript gather, no draws hashed and sorted each tick - parted by the
//! kernel (`parting`), and the field left where they now stand. Only the bodies that
//! moved go back to GDScript, by the last sync's flat indices.

use crate::field::{Field, DESTROYED, FLEEING, LOOSE};
use crate::maths::V2;
use crate::parting::{part, Bodies, Tuning, MOVED};

/// What a pass over the field did: the bodies it moved (flat indices) and where each now
/// stands (a fleeing one: its offset; a loose one: its point and next), those it made
/// loose in the order the GDScript path makes them, and its stats.
#[derive(Default)]
pub struct FieldOutcome {
    pub moved: Vec<i32>,
    pub points: Vec<V2>,
    pub nexts: Vec<V2>,
    pub loosened: Vec<i32>,
    pub passes: i64,
    pub pairs: i64,
}

/// Parts the field's bodies (all but a destroyed squad's), in draw order. `way(draw,
/// other draw)` is UnitBodies.part_way: the seeded way two bodies on each other part.
pub fn part_field(
    field: &mut Field,
    tuning: &Tuning,
    threaded: bool,
    way: &mut dyn FnMut(i64, i64) -> V2,
) -> FieldOutcome {
    let order: Vec<usize> = field
        .drawn
        .iter()
        .map(|&b| b as usize)
        .filter(|&b| field.squads[field.squad[b] as usize].flags & DESTROYED == 0)
        .collect();
    let bases: Vec<V2> = order.iter().map(|&b| field.base[b]).collect();
    let radii: Vec<f64> = order.iter().map(|&b| field.radius[b]).collect();
    let areas: Vec<f64> = order.iter().map(|&b| field.area[b]).collect();
    let squads: Vec<i32> = order.iter().map(|&b| field.squad[b] as i32).collect();
    let factions: Vec<i32> = order.iter().map(|&b| field.faction[b] as i32).collect();
    let draws: Vec<i64> = order.iter().map(|&b| field.draw[b]).collect();
    let mut bodies = Bodies {
        points: order.iter().map(|&b| standing(field, b)).collect(),
        nexts: order.iter().map(|&b| field.next[b]).collect(),
        flags: order.iter().map(|&b| field.flags[b]).collect(),
        bases: &bases,
        radii: &radii,
        areas: &areas,
        squads: &squads,
        factions: &factions,
    };
    let mut ask = |i: usize, j: usize| way(draws[i], draws[j]);
    let outcome = part(&mut bodies, tuning, threaded, &mut ask);
    let mut out = FieldOutcome {
        passes: outcome.passes,
        pairs: outcome.pairs,
        loosened: outcome
            .loosened
            .iter()
            .map(|&i| field.flat_of[order[i as usize]] as i32)
            .collect(),
        ..Default::default()
    };
    for (index, &b) in order.iter().enumerate() {
        if bodies.flags[index] & MOVED == 0 {
            continue;
        }
        field.flags[b] = bodies.flags[index] & !MOVED;
        field.next[b] = bodies.nexts[index];
        if field.flags[b] & FLEEING != 0 {
            field.offset[b] = bodies.points[index];
            field.at[b] = field.base[b].add(field.offset[b]);
        } else {
            field.at[b] = bodies.points[index];
        }
        out.moved.push(field.flat_of[b] as i32);
        out.points.push(bodies.points[index]);
        out.nexts.push(bodies.nexts[index]);
    }
    out
}

/// The kernel's point for a body: a fleeing one's offset, else where it stands.
fn standing(field: &Field, b: usize) -> V2 {
    if field.flags[b] & FLEEING != 0 {
        field.offset[b]
    } else {
        field.at[b]
    }
}

/// Writes the bodies a pass moved (`moved`: flat indices, with their `points` and `nexts`)
/// into the unit store's columns, by how each stands now (spec 35, Decision 144): a fleeing
/// one's offset off its route, a loose one's point and next step, else its position. Called
/// after GDScript has given the bodies the pass made loose their entries, as
/// BodyParting._write_back wrote them.
pub fn commit(field: &mut Field, moved: &[i32], points: &[V2], nexts: &[V2]) {
    for (k, &flat) in moved.iter().enumerate() {
        let b = field.flat[flat as usize] as usize;
        let flags = field.columns.flags[b] as u8;
        let c = &mut field.columns;
        if flags & FLEEING != 0 {
            c.flee_offset[b] = points[k];
        } else if flags & LOOSE == 0 {
            c.position[b] = points[k];
        } else {
            c.loose_at[b] = points[k];
            c.next[b] = nexts[k];
        }
    }
}
