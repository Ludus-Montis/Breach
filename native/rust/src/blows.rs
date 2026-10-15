//! The fight's melee blows in the scrum (ScrumBlows.blows' search) and the landing's
//! counts (BlowLanding._pressed) on the field's own state, as of a sync of the scrum's
//! snapshot made for them. Every unit of a squad that strikes picks the foe it touches -
//! one in its front first, then the nearest, then by the seeded draw (ScrumBlows._pick,
//! the least by the full key, which never ties, so the grid's visiting order can't
//! matter) - and is told whether its blow would come from the foe's flank and whether it
//! has turned away from it while retreating. Cooldowns, the morale's interval and the blow
//! itself stay GDScript's (ScrumBlowsField), taken in the same order.

use std::collections::HashMap;

use crate::facing::{in_front, touching};
use crate::field::{Field, DESTROYED, ROUTING};
use crate::grid::Grid;
use crate::maths::bearing_vector;
use crate::scrum::Foes;

/// A squad's rules, as GDScript hands them over: it is fighting.
pub const FIGHTING: u8 = 1;
/// Its order is to retreat.
pub const RETREAT: u8 = 2;
/// A pick's flags: the striker retreats and has turned away from its foe.
pub const AWAY: u8 = 1;
/// The blow falls outside the foe's front.
pub const FLANK: u8 = 2;

/// BattleTuning's numbers.
pub struct Params {
    pub reach: f64,
    /// ScrumReach.in_front's least dot (cos of the front arc, less 0.000001).
    pub front: f64,
}

/// One unit's pick: flat indices of it and its foe (in the last sync), and AWAY | FLANK.
pub struct Pick {
    pub flat: i32,
    pub foe: i32,
    pub flags: u8,
}

/// ScrumBlows.blows' picks for every unit of every squad that strikes, squads in the
/// sync's order and units in their squad's: `rules` per squad (FIGHTING, RETREAT).
pub fn picks(field: &Field, rules: &[u8], params: &Params) -> Vec<Pick> {
    let mut grids: HashMap<(u32, bool), Foes> = HashMap::new();
    let mut out = Vec::new();
    for (s, &sq) in field.order.iter().enumerate() {
        let squad = &field.squads[sq as usize];
        if squad.flags & (ROUTING | DESTROYED) != 0 || squad.members.is_empty() {
            continue;
        }
        let only_retreating = rules[s] & (FIGHTING | RETREAT) == 0;
        let foes = grids
            .entry((squad.faction, only_retreating))
            .or_insert_with(|| struck(field, rules, squad.faction, only_retreating));
        if foes.bodies.is_empty() {
            continue;
        }
        for &body in &squad.members {
            if let Some(pick) = pick(field, foes, body, rules[s] & RETREAT != 0, params) {
                out.push(pick);
            }
        }
    }
    out
}

/// ScrumBlows._struck_by: the living units of the squads of other factions, not routing
/// (with `only_retreating`, only those of squads retreating), in the list's order.
fn struck(field: &Field, rules: &[u8], faction: u32, only_retreating: bool) -> Foes {
    let mut bodies = Vec::new();
    for (s, &sq) in field.order.iter().enumerate() {
        let other = &field.squads[sq as usize];
        if other.faction == faction || other.flags & (ROUTING | DESTROYED) != 0 {
            continue;
        }
        if !only_retreating || rules[s] & RETREAT != 0 {
            bodies.extend_from_slice(&other.members);
        }
    }
    let points: Vec<_> = bodies.iter().map(|&b| field.at[b as usize]).collect();
    let widest = bodies.iter().map(|&b| field.radius[b as usize]).fold(0.0, f64::max);
    Foes { grid: Grid::build(&points), bodies, widest }
}

/// One unit's foe and how its blow would fall, or None if it touches none.
fn pick(field: &Field, foes: &Foes, body: u32, retreating: bool, p: &Params) -> Option<Pick> {
    let b = body as usize;
    let (at, bearing) = (field.at[b], field.bearing[b]);
    let foe = touching(field, foes, at, field.radius[b], bearing, p.reach, p.front)? as usize;
    let there = field.at[foe];
    let mut flags = 0;
    if retreating && !in_front(bearing_vector(bearing), at, there, p.front) {
        flags |= AWAY;
    }
    if !in_front(bearing_vector(field.bearing[foe]), there, at, p.front) {
        flags |= FLANK;
    }
    Some(Pick { flat: field.flat_of[b] as i32, foe: field.flat_of[foe] as i32, flags })
}

/// BlowLanding._pressed, asked about some blows: for each [striker id, target id], the
/// striker's squad (its place in the sync's order, -1 if not on the field) and how many
/// units touching the target have it as theirs. `targets` is every unit's target id by
/// the sync's flat index.
pub fn pressed(field: &Field, targets: &[i64], asked: &[i64], reach: f64) -> Vec<i32> {
    let mut count: HashMap<u32, i32> = HashMap::new();
    for (flat, &target) in targets.iter().enumerate() {
        let Some(foe) = field.body_of(target) else { continue };
        let unit = field.flat[flat] as usize;
        let f = foe as usize;
        let gap = field.at[unit].distance_to(field.at[f]) as f64
            - field.radius[unit]
            - field.radius[f];
        if (if gap > 0.0 { gap } else { 0.0 }) <= reach {
            *count.entry(foe).or_insert(0) += 1;
        }
    }
    let mut place = vec![-1i32; field.squads.len()];
    for (s, &sq) in field.order.iter().enumerate() {
        place[sq as usize] = s as i32;
    }
    let mut out = Vec::with_capacity(asked.len());
    for pair in asked.chunks(2) {
        let squad = field.body_of(pair[0]).map_or(-1, |b| place[field.squad[b as usize] as usize]);
        let pressed = field.body_of(pair[1]).map_or(0, |b| *count.get(&b).unwrap_or(&0));
        out.push(squad);
        out.push(pressed);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::{Enrol, SyncIn};
    use crate::maths::V2;

    /// Squads 1 (faction a: unit 10 at the origin facing north) and 2 (faction b: unit
    /// 20 north of it with its back to it, and 21 nearer, to the east, facing it), and
    /// the rules with squad 1 holding and 2 fighting or retreating.
    fn field(b_retreats: bool) -> (Field, Vec<u8>) {
        let mut field = Field::default();
        let factions = ["a".to_string(), "b".to_string()];
        let points = [V2::new(0.0, 0.0), V2::new(0.0, -1.2), V2::new(1.1, 0.0)];
        for (unit, point) in [10, 20, 21].into_iter().zip(points) {
            let slot = field.hold(unit) as usize; // each loose where it stands
            field.columns.loose_at[slot] = point;
        }
        let input = SyncIn {
            squad_ids: &[1, 2],
            squad_factions: &factions,
            squad_flags: &[0, 0],
            squad_counts: &[1, 2],
            unit_ids: &[10, 20, 21],
            flags: &[1, 1, 1],
            nexts: &[],
            bases: &[],
            bearings: &[0.0, 0.0, 270.0],
        };
        let unknown = field.sync(5, &input);
        field.enrol(&Enrol {
            flat: &unknown,
            radii: &[0.5, 0.5, 0.5],
            areas: &[1.0, 1.0, 1.0],
            draws: &[3, 2, 1],
            initiatives: &[0, 0, 0],
            records: &[],
        });
        let b = if b_retreats { RETREAT } else { FIGHTING };
        (field, vec![0, b])
    }

    fn params() -> Params {
        Params { reach: 0.25, front: 0.5f64 - 0.000001 }
    }

    #[test]
    fn a_foe_in_front_before_a_nearer_one_and_its_back_is_a_flank() {
        let (field, _) = field(false);
        let picks = picks(&field, &[FIGHTING, FIGHTING], &params());
        let got: Vec<(i32, i32, u8)> = picks.iter().map(|p| (p.flat, p.foe, p.flags)).collect();
        // 10 strikes 20, in its front, not 21, nearer but at its side - on 20's back. 20
        // strikes 10 behind it, in 10's front; 21 strikes 10 on its flank.
        assert_eq!(got, vec![(0, 1, FLANK), (1, 0, 0), (2, 0, FLANK)]);
    }

    #[test]
    fn a_squad_not_fighting_strikes_only_a_retreat_and_a_retreat_only_what_it_faces() {
        let (field, rules) = field(true);
        let picks = picks(&field, &rules, &params());
        let got: Vec<(i32, i32, u8)> = picks.iter().map(|p| (p.flat, p.foe, p.flags)).collect();
        // 20 retreats, turned away from 10; 10 strikes it all the same, as it retreats.
        assert_eq!(got, vec![(0, 1, FLANK), (1, 0, AWAY), (2, 0, FLANK)]);
        assert!(super::picks(&field, &[0, 0], &params()).is_empty()); // none fights, none retreats
    }
}
