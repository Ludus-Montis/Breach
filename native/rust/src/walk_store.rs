//! The scrum's walk on the unit store (spec 35 step 4c): what each unit walks by gathered on
//! the field, and where it ends and looks written into its loose entry's columns there, as
//! ScrumWalkField did in GDScript. Each unit's bearing, rank and column are the store's;
//! its turn rate, backward pace and footprint are its record, enrolled once.

use crate::field::Field;
use crate::maths::V2;
use crate::scrum::{AIM, PRESS, TOUCH};
use crate::walking::{self, Params, WalkIn, WalkOut};

/// Per unit of the walk (`members`: flat indices in the last sync, `codes`: the seek's):
/// `motion` = [bearing, turn rate, backward pace] and, for one walking back to its place
/// (no slot, touching no foe), `frames` = [column, rank, footprint width, depth]; zeros
/// otherwise (walking::WalkIn).
pub fn gather(field: &Field, members: &[i32], codes: &[u8]) -> (Vec<f64>, Vec<f64>) {
    let mut motion = Vec::with_capacity(3 * members.len());
    let mut frames = vec![0.0; 4 * members.len()];
    for (m, &flat) in members.iter().enumerate() {
        let b = field.flat[flat as usize] as usize;
        motion.extend_from_slice(&[
            field.columns.bearing[b],
            field.turn_rate[b],
            field.backward_pace[b],
        ]);
        let code = codes[m];
        if code == TOUCH || code == AIM || code == PRESS {
            continue; // it has no place to walk to
        }
        frames[4 * m] = field.columns.column[b] as f64;
        frames[4 * m + 1] = field.columns.rank[b] as f64;
        frames[4 * m + 2] = field.foot_width[b] as f64;
        frames[4 * m + 3] = field.foot_depth[b] as f64;
    }
    (motion, frames)
}

/// ScrumWalkField._write_entries on the field: where each unit now stands (`ats`) and looks
/// (`looks`) written into its loose entry. One making for a slot looks at its next point;
/// one walking back to its place looks where its walk chose and its next point is where it
/// got; one touching a foe looks nowhere. `bits` = EntryKeys' [toward's key bit, its
/// valued bit].
pub fn write_back(
    field: &mut Field,
    members: &[i32],
    codes: &[u8],
    ats: &[V2],
    looks: &[V2],
    bits: [i32; 2],
) {
    let [held, valued] = bits;
    for (m, &flat) in members.iter().enumerate() {
        let b = field.flat[flat as usize] as usize;
        let code = codes[m];
        let c = &mut field.columns;
        if code == AIM || code == PRESS {
            c.toward[b] = c.next[b];
            c.keys[b] |= held | valued;
            c.loose_at[b] = ats[m];
        } else if code != TOUCH {
            c.toward[b] = looks[m];
            c.keys[b] |= held | valued;
            c.loose_at[b] = ats[m];
            c.next[b] = ats[m];
        } else {
            c.toward[b] = V2::ZERO;
            c.keys[b] = (c.keys[b] | held) & !valued;
        }
    }
}

/// What the walk is told of each squad and of the ground, beyond the seek's: per squad
/// [pace, heading, width, centre shift] and its frame's anchor, and the bodies lying.
pub struct WalkSay<'a> {
    pub squad_floats: &'a [f64],
    pub squad_anchors: &'a [V2],
    pub lying_ats: &'a [V2],
    pub lying_radii: &'a [f64],
    pub lying_areas: &'a [f64],
}

/// The walk, as a phase on the field (BodyField.walk), for the units the tick's seek
/// planned (`squads` per fighting squad [id, foes, units]; `members`, `codes`, `nexts` and
/// `speeds` per unit, as the seek gave them). Where each ends and looks is written into its
/// loose entry (`bits` as `write_back`'s), and returned.
#[allow(clippy::too_many_arguments)]
pub fn walk(
    field: &mut Field,
    squads: &[i64],
    say: &WalkSay,
    members: &[i32],
    codes: &[u8],
    nexts: &[V2],
    speeds: &[f64],
    bits: [i32; 2],
    params: &Params,
) -> WalkOut {
    let member_counts: Vec<i32> = squads.chunks(3).map(|c| c[2] as i32).collect();
    let (motion, frames) = gather(field, members, codes);
    let input = WalkIn {
        member_counts: &member_counts,
        squad_floats: say.squad_floats,
        squad_anchors: say.squad_anchors,
        members,
        codes,
        nexts,
        speeds,
        motion: &motion,
        frames: &frames,
        lying_ats: say.lying_ats,
        lying_radii: say.lying_radii,
        lying_areas: say.lying_areas,
    };
    let out = walking::walk(field, &input, params);
    write_back(field, members, codes, &out.ats, &out.towards, bits);
    out
}
