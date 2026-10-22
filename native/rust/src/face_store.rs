//! The scrum's facing on the unit store (spec 35 step 4d): what each unit turns by gathered
//! on the field, and its bearing after the turn written into the store's column there, as
//! ScrumFaceField did in GDScript. Where it stands, faces and makes for are the store's;
//! its turn rate and backward pace are its record.

use crate::facing::{GOAL, TOWARD};
use crate::field::Field;
use crate::maths::V2;

/// Per unit of the facing (`members`: flat indices in the last sync), as
/// ScrumFaceField._gather read them: where it stands loose, [bearing, turn rate, backward
/// pace], its flags (GOAL: it makes for a slot; TOWARD: its walk chose a point to look
/// towards, `valued` being EntryKeys' bit for that), and its next point, the foe's place
/// there and its look (zero where its flag isn't set).
pub struct Gathered {
    pub ats: Vec<V2>,
    pub motion: Vec<f64>,
    pub flags: Vec<u8>,
    pub nexts: Vec<V2>,
    pub foe_ats: Vec<V2>,
    pub towards: Vec<V2>,
}

pub fn gather(field: &Field, members: &[i32], valued: i32) -> Gathered {
    let count = members.len();
    let mut out = Gathered {
        ats: Vec::with_capacity(count),
        motion: Vec::with_capacity(3 * count),
        flags: Vec::with_capacity(count),
        nexts: Vec::with_capacity(count),
        foe_ats: Vec::with_capacity(count),
        towards: Vec::with_capacity(count),
    };
    let c = &field.columns;
    for &flat in members {
        let b = field.flat[flat as usize] as usize;
        out.ats.push(c.loose_at[b]);
        out.motion.extend_from_slice(&[c.bearing[b], field.turn_rate[b], field.backward_pace[b]]);
        let mut flags = 0u8;
        if c.goal_slot[b] >= 0 {
            flags = GOAL;
            out.nexts.push(c.next[b]);
            out.foe_ats.push(c.foe_at[b]);
        } else {
            out.nexts.push(V2::ZERO);
            out.foe_ats.push(V2::ZERO);
        }
        if c.keys[b] & valued != 0 {
            flags |= TOWARD;
            out.towards.push(c.toward[b]);
        } else {
            out.towards.push(V2::ZERO);
        }
        out.flags.push(flags);
    }
    out
}

/// Each unit's bearing after its turn, written into the store's column.
pub fn write_back(field: &mut Field, members: &[i32], bearings: &[f64]) {
    for (m, &flat) in members.iter().enumerate() {
        let b = field.flat[flat as usize] as usize;
        field.columns.bearing[b] = bearings[m];
    }
}
