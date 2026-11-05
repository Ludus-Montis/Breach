//! What the 1D feel test's scene draws of a lane after each tick (spec 39 step 9d-ii): each
//! listed squad, in the listed order, with its id, faction, state, order and direction, then
//! each living unit: its id, how far along the lane it stands, the middle of its span across
//! the lane (SquadFrame.span on the squad's lateral axis) with how far a swap has moved it
//! across (FormationShuffle.offset), its footprint, hp, whether it shoots and its target.

use super::Lane;
use crate::geometry::{extent, lateral_axis, Frame};
use crate::swaps;

/// A unit as seen.
#[derive(Clone, Debug, PartialEq)]
pub struct SeenUnit {
    pub id: i64,
    pub distance: f64,
    pub lateral: f64,
    /// (width, depth).
    pub foot: (i64, i64),
    pub hp: i64,
    pub max_hp: i64,
    pub ranged: bool,
    pub target: i64,
}

/// A squad as seen.
#[derive(Clone, Debug, PartialEq)]
pub struct SeenSquad {
    pub id: i64,
    pub faction: String,
    pub state: i32,
    pub order: i32,
    pub direction: i32,
    pub units: Vec<SeenUnit>,
}

impl Lane {
    /// The listed squads as seen, in the listed order.
    pub fn seen(&self) -> Vec<SeenSquad> {
        let (squads, columns) = (&self.squads, &self.field.columns);
        let squad = |s: usize| {
            let frame = Frame::of(squads, s);
            let axis = lateral_axis(frame.heading);
            let moving = swaps::offsets(squads, s);
            let direction = squads.direction[s];
            let unit = |u: i32| {
                let u = u as usize;
                let sheet = &columns.sheet[u];
                let id = self.field.unit_at(u as u32).unwrap_or(0);
                let foot = (sheet.footprint_width, sheet.footprint_depth);
                let place = (columns.rank[u], columns.column[u]);
                let span = extent(&frame.corners(foot, place), axis);
                let across = moving.iter().find(|m| m.0 == id).map_or(0.0, |m| f64::from(m.2));
                SeenUnit {
                    id,
                    distance: columns.distance[u],
                    lateral: f64::from(span.0 + span.1) * 0.5 + across * f64::from(direction),
                    foot,
                    hp: i64::from(columns.hp[u]),
                    max_hp: sheet.max_hp,
                    ranged: sheet.attack_range > 0,
                    target: columns.target_id[u],
                }
            };
            SeenSquad {
                id: squads.id[s],
                faction: self.field.lexicon.word(squads.faction[s]).to_string(),
                state: squads.state[s],
                order: squads.order[s],
                direction,
                units: squads.living(s, &columns.state).into_iter().map(unit).collect(),
            }
        };
        self.play.listed.iter().map(|&s| squad(s)).collect()
    }
}
