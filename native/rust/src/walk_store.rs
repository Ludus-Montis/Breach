//! The scrum's walk on the unit store (spec 35 step 4c): what each unit walks by gathered on
//! the field, and where it ends and looks written into its loose entry's columns there, as
//! ScrumWalkField did in GDScript. Each unit's bearing, rank and column are the store's;
//! its turn rate, backward pace and footprint are its record, enrolled once.

use crate::field::Field;
use crate::maths::V2;
use crate::scrum::{AIM, PRESS, TOUCH};

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
