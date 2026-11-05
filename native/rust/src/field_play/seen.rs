//! What the 2D field's scene and HUD read of the field after each tick (spec 39 step 9d-i),
//! a snapshot in the listed order: each squad's id, faction, state, front, morale and its
//! band, whether it is blocked or staged, the way a stance faces it, a pursuit's post with
//! how far it has gone of its leash (FormationPursuit.reach) and its loose units, then each
//! unit it holds: its id, slot, state, footprint, leadership and detection. Each wave's
//! places are counted built and in all, rank and file and leaders apart, and B's waiting
//! wave's staging point is where its route reaches the stage.

#[cfg(test)]
mod tests;

use super::actions::STAGING_AT;
use super::{FieldPlay, B};
use crate::bands;
use crate::content::Content;
use crate::discipline::{self, DisciplineTuning};
use crate::field::Field;
use crate::maths::V2;
use crate::play;
use crate::roster::Listed;
use crate::squads::{Squads, DEAD};

/// A unit as seen: its id, slot, state (SkirmishUnit.State's code), footprint (width,
/// depth), leadership and detection.
#[derive(Clone, Debug, PartialEq)]
pub struct SeenUnit {
    pub id: i64,
    pub slot: usize,
    pub state: i32,
    pub foot: (i64, i64),
    pub leadership: i64,
    pub detection: f64,
}

/// A pursuit under way: its post, how far the squad's front is from it, and its leash.
#[derive(Clone, Debug, PartialEq)]
pub struct Pursuing {
    pub post_at: V2,
    pub gone: f64,
    pub leash: f64,
}

/// A squad as seen.
#[derive(Clone, Debug, PartialEq)]
pub struct SeenSquad {
    pub id: i64,
    pub faction: String,
    pub state: i32,
    pub position: V2,
    pub morale: i64,
    pub band: i64,
    pub blocked: bool,
    pub staged: bool,
    pub heading: Option<f64>,
    pub pursuit: Option<Pursuing>,
    pub loose: Vec<i64>,
    pub units: Vec<SeenUnit>,
}

impl FieldPlay {
    /// The listed squads as seen, in the listed order; empty if the rules can't be read.
    pub fn seen(&self, squads: &Squads, field: &Field, content: &Content) -> Vec<SeenSquad> {
        let Some((_, rules)) = self.play.rules(content) else { return Vec::new() };
        let (bands, held) = (&rules.mustering.bands, &rules.scrum.pursue.discipline);
        let roster = play::listed(squads, field, &self.play.listed, false);
        let columns = &field.columns;
        let squad = |mut listed: Listed| {
            let s = listed.squad;
            let units = squads.members[s].iter().map(|&u| unit(field, u as usize)).collect();
            listed.living.retain(|m| columns.state[m.slot] < DEAD);
            let morale = i64::from(squads.morale[s]);
            let pursuit = squads.pursuit[s].as_ref().filter(|p| !p.returning).map(|p| Pursuing {
                post_at: p.post_at,
                gone: f64::from(squads.position[s].distance_to(p.post_at)),
                leash: leash(field, &listed, held),
            });
            SeenSquad {
                id: squads.id[s],
                faction: field.lexicon.word(squads.faction[s]).to_string(),
                state: squads.state[s],
                position: squads.position[s],
                morale,
                band: bands::band(morale, bands),
                blocked: squads.blocked[s] != 0,
                staged: squads.staging[s].is_some(),
                heading: squads.stance[s].as_ref().map(|stance| stance.heading),
                pursuit,
                loose: squads.loose[s].clone(),
                units,
            }
        };
        roster.listed.into_iter().map(squad).collect()
    }

    /// Wave `w`'s places: [rank and file built, rank and file in all, leaders built, leaders
    /// in all].
    pub fn counted(&self, w: usize, content: &Content) -> [i64; 4] {
        let wave = &self.waves[w];
        let mut counts = [0; 4];
        for (place, &filled) in wave.template.ordered().iter().zip(&wave.filled) {
            let leads = content.units.get(place.id).is_some_and(|unit| unit.leadership > 0);
            let at = if leads { 2 } else { 0 };
            counts[at] += i64::from(filled);
            counts[at + 1] += 1;
        }
        counts
    }

    /// Where B's wave waits, if it is to: its route's point at the stage.
    pub fn staging_point(&self) -> Option<V2> {
        let route = &self.routes[self.waves[B].route].0;
        self.waves[B].staging.as_ref().map(|_| route.point_at(STAGING_AT))
    }
}

/// The unit in `slot` as seen.
fn unit(field: &Field, slot: usize) -> SeenUnit {
    let sheet = &field.columns.sheet[slot];
    SeenUnit {
        id: field.unit_at(slot as u32).unwrap_or(0),
        slot,
        state: field.columns.state[slot],
        foot: (sheet.footprint_width, sheet.footprint_depth),
        leadership: sheet.leadership,
        detection: sheet.detection,
    }
}

/// FormationDiscipline.pursuit_leash: how far from its post the squad's living units chase,
/// a step out if one "pursues" and a step in if one is "cautious".
fn leash(field: &Field, living: &Listed, t: &DisciplineTuning) -> f64 {
    let has = |tactic: &str| {
        field.lexicon.find(tactic).is_some_and(|code| {
            living.living.iter().any(|m| field.columns.words[m.slot].tactics.contains(&code))
        })
    };
    let shift = i64::from(has("pursues")) - i64::from(has("cautious"));
    discipline::leash(discipline::of(&field.columns, living, t) as f64, shift, t)
}
